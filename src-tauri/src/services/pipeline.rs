use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use tauri::{AppHandle, Emitter, Manager};
use jwalk::WalkDir;
use uuid::Uuid;
use chrono::Utc;
use rusqlite::Connection;

use crate::errors::AppError;
use crate::domain::scan_job::{ScanJob, ScanProgress};

// Row type sent from walker thread to DB writer thread
type FileRecord = (String, String, String, String, String, String, Option<String>, i64, String, Option<String>, String);

const BATCH_SIZE: usize = 10_000;
const PROGRESS_EVERY: i32 = 10_000;
const INSERT_ROWS_PER_STMT: usize = 85; // 85 * 11 params = 935, under SQLite's 999 limit

pub struct PipelineManager {
    pub active_scans: Arc<Mutex<HashMap<String, ScanProgress>>>,
}

impl PipelineManager {
    pub fn new() -> Self {
        Self {
            active_scans: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

pub async fn start_scan(app: AppHandle, source_id: String) -> Result<ScanJob, AppError> {
    let pipeline = app.state::<PipelineManager>();

    {
        let scans = pipeline.active_scans.lock().unwrap();
        if let Some(existing) = scans.get(&source_id) {
            if existing.status == "running" {
                return Err(AppError::InvalidInput("Scan already running for this source".into()));
            }
        }
    }

    let app_data_dir = app.path().app_data_dir().map_err(|e| AppError::IoError(e.to_string()))?;
    let db_path = app_data_dir.join("tlpyl.db");

    let (mount_path, source_kind): (String, String) = {
        let conn = Connection::open(&db_path).map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT current_mount_path, currently_mounted, source_kind FROM storage_sources WHERE id = ?")
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let result = stmt.query_row([&source_id], |row| {
            let mounted: i32 = row.get(1)?;
            let path: Option<String> = row.get(0)?;
            let kind: String = row.get(2)?;
            Ok((mounted, path, kind))
        }).map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if result.0 == 0 || result.1.is_none() {
            return Err(AppError::InvalidInput("Source is offline and cannot be scanned".into()));
        }
        (result.1.unwrap(), result.2)
    };

    let job_id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();

    let job = ScanJob {
        id: job_id.clone(),
        source_id: source_id.clone(),
        started_at: started_at.clone(),
        completed_at: None,
        status: "running".into(),
        stage: 1,
        files_found: 0,
        files_inserted: 0,
        error_message: None,
    };

    let total_used = get_drive_used_space(&mount_path);

    let progress = ScanProgress {
        source_id: source_id.clone(),
        status: "running".into(),
        stage: 1,
        files_found: 0,
        files_inserted: 0,
        bytes_found: 0,
        total_used_bytes: total_used,
    };

    {
        let conn = Connection::open(&db_path).map_err(|e| AppError::DatabaseError(e.to_string()))?;
        conn.execute(
            "INSERT INTO scan_jobs (id, source_id, started_at, status, stage, files_found, files_inserted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![job.id, job.source_id, job.started_at, job.status, job.stage, job.files_found, job.files_inserted],
        ).map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    {
        let mut scans = pipeline.active_scans.lock().unwrap();
        scans.insert(source_id.clone(), progress.clone());
    }

    let _ = app.emit("pipeline://progress", progress.clone());

    let source_id_clone = source_id.clone();
    let app_clone = app.clone();
    let db_path_clone = db_path.clone();
    
    let started_at_clone = started_at.clone();
    tokio::task::spawn_blocking(move || {
        stage_1_worker(app_clone, db_path_clone, job_id, source_id_clone, mount_path, total_used, source_kind, started_at_clone);
    });

    Ok(job)
}

fn get_drive_used_space(mount_path: &str) -> u64 {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let mut free_bytes_available: u64 = 0;
    let mut total_number_of_bytes: u64 = 0;
    let mut total_number_of_free_bytes: u64 = 0;

    let hstring = HSTRING::from(mount_path);
    let pcwstr = PCWSTR(hstring.as_ptr());
    
    unsafe {
        let _ = GetDiskFreeSpaceExW(
            pcwstr,
            Some(&mut free_bytes_available),
            Some(&mut total_number_of_bytes),
            Some(&mut total_number_of_free_bytes),
        );
    }
    
    total_number_of_bytes.saturating_sub(total_number_of_free_bytes)
}

fn stage_1_worker(app: AppHandle, db_path: PathBuf, job_id: String, source_id: String, mount_path: String, total_used_bytes: u64, source_kind: String, started_at: String) {
    let pipeline = app.state::<PipelineManager>();

    // ── DB writer thread ──────────────────────────────────────────────────────
    // Runs independently of the walker. Drains the channel and writes to SQLite
    // in large batches without ever blocking the directory traversal.
    let (tx, rx) = mpsc::sync_channel::<FileRecord>(50_000);

    let db_path_writer = db_path.clone();
    let source_id_writer = source_id.clone();
    let job_id_writer = job_id.clone();
    let app_writer = app.clone();

    // Shared progress counters written by DB thread, read by progress emitter
    let found_counter = Arc::new(Mutex::new(0i32));
    let inserted_counter = Arc::new(Mutex::new(0i32));
    let bytes_counter = Arc::new(Mutex::new(0u64));

    let found_w = Arc::clone(&found_counter);
    let inserted_w = Arc::clone(&inserted_counter);
    let bytes_w = Arc::clone(&bytes_counter);
    let pipeline_state = Arc::clone(&pipeline.active_scans);

    let writer_handle = thread::spawn(move || {
        let mut conn = match Connection::open(&db_path_writer) {
            Ok(c) => c,
            Err(_) => return,
        };

        // Apply SQLite performance pragmas on this connection too
        let _ = conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -64000;
             PRAGMA temp_store = MEMORY;",
        );

        let mut batch: Vec<FileRecord> = Vec::with_capacity(BATCH_SIZE);
        let mut total_found = 0i32;
        let mut total_inserted = 0i32;
        let mut total_bytes = 0u64;

        for record in rx {
            total_found += 1;
            total_bytes += record.7.unsigned_abs() as u64;
            batch.push(record);

            if batch.len() >= BATCH_SIZE {
                if let Ok(ins) = insert_batch(&mut conn, &batch) {
                    total_inserted += ins;
                }
                batch.clear();

                // Update shared counters
                *found_w.lock().unwrap() = total_found;
                *inserted_w.lock().unwrap() = total_inserted;
                *bytes_w.lock().unwrap() = total_bytes;

                // Emit progress every PROGRESS_EVERY files
                if total_found % PROGRESS_EVERY == 0 {
                    let prog = ScanProgress {
                        source_id: source_id_writer.clone(),
                        status: "running".into(),
                        stage: 1,
                        files_found: total_found,
                        files_inserted: total_inserted,
                        bytes_found: total_bytes,
                        total_used_bytes,
                    };
                    {
                        let mut scans = pipeline_state.lock().unwrap();
                        scans.insert(source_id_writer.clone(), prog.clone());
                    }
                    let _ = app_writer.emit("pipeline://progress", prog);

                    // Persist to DB infrequently (every 25k files)
                    if total_found % 25_000 == 0 {
                        let _ = conn.execute(
                            "UPDATE scan_jobs SET files_found = ?1, files_inserted = ?2 WHERE id = ?3",
                            rusqlite::params![total_found, total_inserted, job_id_writer],
                        );
                    }
                }
            }
        }

        // Flush remaining
        if !batch.is_empty() {
            if let Ok(ins) = insert_batch(&mut conn, &batch) {
                total_inserted += ins;
            }
        }

        *found_w.lock().unwrap() = total_found;
        *inserted_w.lock().unwrap() = total_inserted;
        *bytes_w.lock().unwrap() = total_bytes;

        // Perform Mark and Sweep: Soft-delete anything we didn't touch this scan
        let sweep_now = Utc::now().to_rfc3339();
        let _ = conn.execute(
            "UPDATE file_instances SET deleted_at = ?1 WHERE source_id = ?2 AND (stage_1_at < ?3 OR stage_1_at IS NULL) AND deleted_at IS NULL",
            rusqlite::params![sweep_now, source_id_writer, started_at],
        );

        // Final job update
        let completed_at = Utc::now().to_rfc3339();
        let _ = conn.execute(
            "UPDATE scan_jobs SET status = 'completed', completed_at = ?1, files_found = ?2, files_inserted = ?3 WHERE id = ?4",
            rusqlite::params![completed_at, total_found, total_inserted, job_id_writer],
        );
    });

    // ── Walker thread (this thread) ───────────────────────────────────────────
    let root_path = Path::new(&mount_path);
    
    let parallelism = if source_kind == "removable" {
        jwalk::Parallelism::Serial
    } else {
        jwalk::Parallelism::RayonDefaultPool { busy_timeout: std::time::Duration::from_secs(1).into() }
    };
    
    let walker = WalkDir::new(root_path)
        .parallelism(parallelism)
        .skip_hidden(false)
        .process_read_dir(|_, _, _, dir_entry_results| {
            dir_entry_results.retain(|dir_entry_result| {
                if let Ok(entry) = dir_entry_result {
                    if let Some(name) = entry.file_name().to_str() {
                        if name == ".tlpyl-quarantine" || name == "System Volume Information" || name == "$RECYCLE.BIN" {
                            return false;
                        }
                    }
                }
                true
            });
        });

    let now_str = Utc::now().to_rfc3339(); // reuse same timestamp for all stage_1_at in this scan

    for entry_res in walker {
        if let Ok(entry) = entry_res {
            if entry.file_type().is_file() {
                let abs_path = entry.path();
                
                let relative = match abs_path.strip_prefix(root_path) {
                    Ok(rel) => rel.to_string_lossy().to_string(),
                    Err(_) => continue,
                };

                let file_name = entry.file_name().to_string_lossy().to_string();
                let extension = abs_path.extension().map(|e| e.to_string_lossy().to_string());
                let current_path_str = abs_path.to_string_lossy().to_string();
                
                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                
                let size_bytes = metadata.len() as i64;
                
                let modified_at = metadata.modified()
                    .map(|sys_time| chrono::DateTime::<Utc>::from(sys_time).to_rfc3339())
                    .unwrap_or_else(|_| now_str.clone());
                
                let created_at_fs = metadata.created()
                    .ok()
                    .map(|sys_time| chrono::DateTime::<Utc>::from(sys_time).to_rfc3339());

                let id = Uuid::new_v4().to_string();

                let record: FileRecord = (
                    id.clone(),
                    source_id.clone(),
                    id, // stable_location_id
                    relative,
                    current_path_str,
                    file_name,
                    extension,
                    size_bytes,
                    modified_at,
                    created_at_fs,
                    now_str.clone(),
                );

                // If the writer is backed up (channel full), this will block briefly —
                // that's intentional backpressure so we don't endlessly buffer RAM.
                if tx.send(record).is_err() {
                    break; // writer died, abort
                }
            }
        }
    }

    // Drop sender so the writer thread knows traversal is done
    drop(tx);

    // Wait for writer to finish flushing
    let _ = writer_handle.join();

    // Emit final completed event
    let final_found = *found_counter.lock().unwrap();
    let final_inserted = *inserted_counter.lock().unwrap();
    let final_bytes = *bytes_counter.lock().unwrap();

    let final_progress = ScanProgress {
        source_id: source_id.clone(),
        status: "completed".into(),
        stage: 1,
        files_found: final_found,
        files_inserted: final_inserted,
        bytes_found: final_bytes,
        total_used_bytes,
    };

    {
        let mut scans = pipeline.active_scans.lock().unwrap();
        scans.insert(source_id.clone(), final_progress.clone());
    }

    let _ = app.emit("pipeline://progress", final_progress);
}

fn insert_batch(conn: &mut Connection, batch: &[FileRecord]) -> Result<i32, rusqlite::Error> {
    let tx = conn.transaction()?;
    let mut inserted = 0;
    
    // Chunk into groups that stay under SQLite's 999-parameter limit
    for chunk in batch.chunks(INSERT_ROWS_PER_STMT) {
        let row_placeholders: Vec<String> = chunk.iter().enumerate().map(|(i, _)| {
            let b = i * 11;
            format!("(?{},?{},?{},?{},?{},?{},?{},?{},?{},?{},?{})",
                b+1, b+2, b+3, b+4, b+5, b+6, b+7, b+8, b+9, b+10, b+11)
        }).collect();
        
        let sql = format!(
            "INSERT INTO file_instances \
             (id, source_id, stable_location_id, volume_relative_path, current_path, \
              file_name, extension, size_bytes, modified_at, created_at_fs, stage_1_at) \
             VALUES {} \
             ON CONFLICT(source_id, volume_relative_path) DO UPDATE SET \
                current_path = excluded.current_path, \
                stage_1_at = excluded.stage_1_at, \
                deleted_at = NULL, \
                blake3_hash = CASE WHEN file_instances.size_bytes != excluded.size_bytes OR file_instances.modified_at != excluded.modified_at THEN NULL ELSE file_instances.blake3_hash END, \
                stage_2_at = CASE WHEN file_instances.size_bytes != excluded.size_bytes OR file_instances.modified_at != excluded.modified_at THEN NULL ELSE file_instances.stage_2_at END, \
                stage_3_at = CASE WHEN file_instances.size_bytes != excluded.size_bytes OR file_instances.modified_at != excluded.modified_at THEN NULL ELSE file_instances.stage_3_at END, \
                size_bytes = excluded.size_bytes, \
                modified_at = excluded.modified_at, \
                created_at_fs = excluded.created_at_fs",
            row_placeholders.join(",")
        );
        
        let params: Vec<&dyn rusqlite::ToSql> = chunk.iter().flat_map(|r| {
            let v: Vec<&dyn rusqlite::ToSql> = vec![
                &r.0, &r.1, &r.2, &r.3, &r.4,
                &r.5, &r.6, &r.7, &r.8, &r.9, &r.10
            ];
            v
        }).collect();
        
        inserted += tx.execute(&sql, rusqlite::params_from_iter(params.iter().copied()))? as i32;
    }
    
    tx.commit()?;
    Ok(inserted)
}
