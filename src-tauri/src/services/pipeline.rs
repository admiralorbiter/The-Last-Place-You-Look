use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::fs::File;
use std::io::{BufReader, Read};
use tauri::{AppHandle, Emitter, Manager};
use jwalk::WalkDir;
use uuid::Uuid;
use chrono::Utc;
use rusqlite::Connection;
use rayon::prelude::*;

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

/// Start Stage 2 hashing on demand, independent of Stage 1.
/// Safe to call even if Stage 2 is already partially complete — it resumes
/// from where it left off (only hashes files with blake3_hash IS NULL).
pub async fn start_hashing(app: AppHandle, source_id: String) -> Result<(), AppError> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| AppError::IoError(e.to_string()))?;
    let db_path = app_data_dir.join("tlpyl.db");

    // Look up source_kind so we can throttle concurrency for removable drives
    let (source_kind, total_used): (String, u64) = {
        let conn = Connection::open(&db_path).map_err(|e| AppError::DatabaseError(e.to_string()))?;
        conn.query_row(
            "SELECT source_kind, currently_mounted FROM storage_sources WHERE id = ?",
            [&source_id],
            |row| {
                let kind: String = row.get(0)?;
                let mounted: i32 = row.get(1)?;
                Ok((kind, mounted as u64))
            }
        ).map_err(|e| AppError::DatabaseError(e.to_string()))?;
        // Get used bytes separately
        let kind: String = conn.query_row(
            "SELECT source_kind FROM storage_sources WHERE id = ?",
            [&source_id],
            |row| row.get(0)
        ).unwrap_or_else(|_| "removable".into());
        let mount_path: Option<String> = conn.query_row(
            "SELECT current_mount_path FROM storage_sources WHERE id = ?",
            [&source_id],
            |row| row.get(0)
        ).unwrap_or(None);
        let used = mount_path.as_deref().map(get_drive_used_space).unwrap_or(0);
        (kind, used)
    };

    let app_clone = app.clone();
    let source_id_clone = source_id.clone();
    let db_path_clone = db_path.clone();

    tokio::task::spawn_blocking(move || {
        stage_2_worker(app_clone, db_path_clone, source_id_clone, total_used, &source_kind);
    });

    Ok(())
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

    // ── Pre-load known files for fast change detection ────────────────────────
    // On a rescan, the vast majority of files are unchanged. We load the existing
    // (relative_path, size_bytes, modified_at) fingerprints into a HashSet so the
    // walker can check in O(1) and skip unchanged files entirely — they never touch
    // the channel or DB. Only new/modified/deleted files cause any DB work.
    // Key: volume_relative_path  Value: (size_bytes, modified_at)
    let known_files: HashMap<String, (i64, String)> = {
        match Connection::open(&db_path) {
            Ok(conn) => {
                let mut stmt = conn.prepare(
                    "SELECT volume_relative_path, size_bytes, modified_at 
                     FROM file_instances 
                     WHERE source_id = ? AND deleted_at IS NULL"
                ).unwrap_or_else(|_| return conn.prepare("SELECT 1").unwrap());
                let mut map = HashMap::new();
                if let Ok(rows) = stmt.query_map([&source_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                }) {
                    for row in rows.flatten() {
                        map.insert(row.0, (row.1, row.2));
                    }
                }
                map
            }
            Err(_) => HashMap::new(),
        }
    };
    let is_rescan = !known_files.is_empty();

    // ── DB writer thread ──────────────────────────────────────────────────────
    // Runs independently of the walker. Drains the channel and writes to SQLite
    // in large batches without ever blocking the directory traversal.
    let (tx, rx) = mpsc::sync_channel::<FileRecord>(50_000);

    let db_path_writer = db_path.clone();
    let source_id_writer = source_id.clone();
    let job_id_writer = job_id.clone();
    let app_writer = app.clone();
    let is_rescan_flag = is_rescan; // capture for writer closure

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
             PRAGMA synchronous = OFF;
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

        // Perform Mark and Sweep only on FIRST scans.
        // On rescan, we use the set-difference approach below (after the walker).
        // Safe check: started_at is used as a sentinel; if known_files was empty, is_rescan=false.
        if !is_rescan_flag {
            let sweep_now = Utc::now().to_rfc3339();
            let _ = conn.execute(
                "UPDATE file_instances SET deleted_at = ?1 WHERE source_id = ?2 AND (stage_1_at < ?3 OR stage_1_at IS NULL) AND deleted_at IS NULL",
                rusqlite::params![sweep_now, source_id_writer, started_at],
            );
        }

        // Final job update
        let completed_at = Utc::now().to_rfc3339();
        let _ = conn.execute(
            "UPDATE scan_jobs SET status = 'completed', completed_at = ?1, files_found = ?2, files_inserted = ?3 WHERE id = ?4",
            rusqlite::params![completed_at, total_found, total_inserted, job_id_writer],
        );
    });

    // ── Walker thread (this thread) ──────────────────────────────────────────
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

    let now_str = Utc::now().to_rfc3339();
    let mut visited_paths: HashSet<String> = if is_rescan {
        HashSet::with_capacity(known_files.len())
    } else {
        HashSet::new()
    };

    let _total_known = known_files.len() as i32;
    let mut walker_visited_count = 0i32;
    let mut walker_bytes_visited = 0u64;
    let mut last_progress_at = std::time::Instant::now();

    for entry_res in walker {
        if let Ok(entry) = entry_res {
            if entry.file_type().is_file() {
                let abs_path = entry.path();
                
                let relative = match abs_path.strip_prefix(root_path) {
                    Ok(rel) => rel.to_string_lossy().to_string(),
                    Err(_) => continue,
                };

                // ── Fast-path: skip files we've already seen and haven't changed ──
                if is_rescan {
                    visited_paths.insert(relative.clone());
                    walker_visited_count += 1;

                    // Accumulate bytes for % calculation (use known size to avoid re-reading metadata)
                    if let Some((known_size, _)) = known_files.get(&relative) {
                        walker_bytes_visited += (*known_size).unsigned_abs() as u64;
                    }

                    // Emit progress every ~1 second (rescan heartbeat)
                    if last_progress_at.elapsed().as_millis() >= 1000 {
                        last_progress_at = std::time::Instant::now();
                        let prog = ScanProgress {
                            source_id: source_id.clone(),
                            status: "running".into(),
                            stage: 1,
                            files_found: walker_visited_count,
                            files_inserted: *inserted_counter.lock().unwrap(),
                            bytes_found: walker_bytes_visited,
                            total_used_bytes,
                        };
                        let mut scans = pipeline.active_scans.lock().unwrap();
                        scans.insert(source_id.clone(), prog.clone());
                        drop(scans);
                        let _ = app.emit("pipeline://progress", prog);
                    }

                    if let Some((known_size, known_modified)) = known_files.get(&relative) {
                        let metadata = match entry.metadata() {
                            Ok(m) => m,
                            Err(_) => continue,
                        };
                        let size_bytes = metadata.len() as i64;
                        let modified_at = metadata.modified()
                            .map(|t| chrono::DateTime::<Utc>::from(t).to_rfc3339())
                            .unwrap_or_default();

                        if *known_size == size_bytes && *known_modified == modified_at {
                            continue; // Unchanged — skip entirely
                        }
                    }
                }

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

                if tx.send(record).is_err() {
                    break;
                }
            }
        }
    }

    // Drop sender so the writer thread knows traversal is done
    drop(tx);

    // Wait for writer to finish flushing
    let _ = writer_handle.join();

    // ── Mark-and-Sweep: soft-delete files no longer on disk ───────────────────
    // On a rescan with the fast-path skip, unchanged files never touch stage_1_at.
    // We compute deletions by set-difference: known_files - visited_paths.
    if is_rescan {
        // Files that were in DB but we never saw on this walk → deleted from disk
        let deleted_paths: Vec<&String> = known_files
            .keys()
            .filter(|p| !visited_paths.contains(*p))
            .collect();

        if !deleted_paths.is_empty() {
            if let Ok(sweep_conn) = Connection::open(&db_path) {
                let _ = sweep_conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = OFF;");
                let sweep_now = Utc::now().to_rfc3339();
                for chunk in deleted_paths.chunks(500) {
                    let placeholders: Vec<String> = (1..=chunk.len()).map(|i| format!("?{}", i + 1)).collect();
                    let sql = format!(
                        "UPDATE file_instances SET deleted_at = ?1 WHERE source_id = '{}' AND volume_relative_path IN ({})",
                        source_id, placeholders.join(",")
                    );
                    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(sweep_now.clone())];
                    for p in chunk {
                        params.push(Box::new((*p).clone()));
                    }
                    let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();
                    let _ = sweep_conn.execute(&sql, rusqlite::params_from_iter(refs.iter().copied()));
                }
            }
        }
    }
    // (First scans: the writer thread already handled sweep via stage_1_at timestamp)

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

    // Trigger Stage 2 (Hashing)
    let db_path_s2 = db_path.clone();
    let source_id_s2 = source_id.clone();
    let app_s2 = app.clone();
    let total_used_s2 = total_used_bytes;
    
    tokio::task::spawn_blocking(move || {
        stage_2_worker(app_s2, db_path_s2, source_id_s2, total_used_s2, &source_kind);
    });
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

fn stage_2_worker(app: AppHandle, db_path: PathBuf, source_id: String, total_used_bytes: u64, _source_kind: &str) {
    let source_kind = _source_kind; // re-bind so threadpool code below can use it as `source_kind`
    let pipeline = app.state::<PipelineManager>();

    // Load the list of files needing hashes — smallest first for fastest initial progress
    let files_to_hash: Vec<(String, String, i64)> = {
        let conn = match Connection::open(&db_path) {
            Ok(c) => c,
            Err(_) => return,
        };
        let _ = conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -64000;
             PRAGMA temp_store = MEMORY;",
        );
        let mut stmt = match conn.prepare(
            "SELECT id, current_path, size_bytes FROM file_instances 
             WHERE source_id = ? AND blake3_hash IS NULL AND deleted_at IS NULL AND current_path IS NOT NULL
             ORDER BY size_bytes ASC"
        ) {
            Ok(s) => s,
            Err(_) => return,
        };
        let mut result = Vec::new();
        if let Ok(rows) = stmt.query_map([&source_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
        }) {
            for row in rows.flatten() {
                result.push(row);
            }
        }
        result
    };

    let total_files = files_to_hash.len() as i32;
    if total_files == 0 {
        let final_progress = ScanProgress {
            source_id: source_id.clone(),
            status: "completed".into(),
            stage: 2,
            files_found: 0,
            files_inserted: 0,
            bytes_found: 0,
            total_used_bytes,
        };
        if let Ok(mut scans) = pipeline.active_scans.lock() {
            scans.insert(source_id.clone(), final_progress.clone());
        }
        let _ = app.emit("pipeline://progress", final_progress);
        return;
    }

    // ── Parallel hashing with rayon ─────────────────────────────────────────
    // CRITICAL: Spinning USB drives perform WORSE with more threads — random
    // seeks cost more than the threading gain. Cap at 2 for removable.
    // Internal SSDs benefit from full parallelism.
    let num_threads = if source_kind == "removable" { 2 } else { num_cpus::get().min(8) };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());

    let hashed_counter = Arc::new(Mutex::new(0i32));
    let (result_tx, result_rx) = mpsc::channel::<(String, String)>(); // (id, hash)

    let source_id_par = source_id.clone();
    let counter_par = Arc::clone(&hashed_counter);
    let app_par = app.clone();
    let source_id_progress = source_id.clone();

    // Spawn the parallel hash work in a separate thread so we can drain results
    // concurrently on the main stage-2 thread below.
    let pipeline_state = Arc::clone(&pipeline.active_scans);
    let result_tx_thread = result_tx.clone();
    let hash_thread = thread::spawn(move || {
        pool.install(|| {
            files_to_hash.into_par_iter().for_each(|(id, path_str, _size)| {
                let hash_result = hash_file(&path_str);
                if let Ok(hex) = hash_result {
                    let _ = result_tx_thread.send((id, hex));
                }

                let count = {
                    let mut c = counter_par.lock().unwrap();
                    *c += 1;
                    *c
                };

                if count % 500 == 0 || count == total_files {
                    let prog = ScanProgress {
                        source_id: source_id_par.clone(),
                        status: if count == total_files { "completed".into() } else { "running".into() },
                        stage: 2,
                        files_found: total_files,
                        files_inserted: count,
                        bytes_found: 0,
                        total_used_bytes,
                    };
                    if let Ok(mut scans) = pipeline_state.lock() {
                        scans.insert(source_id_par.clone(), prog.clone());
                    }
                    let _ = app_par.emit("pipeline://progress", prog);
                }
            });
        });
        drop(result_tx_thread);
    });

    // We no longer need our own clone of the sender; drop it so result_rx
    // terminates correctly when the hash_thread finishes.
    drop(result_tx);

    // ── Batched DB writes ────────────────────────────────────────────────────
    // We drain the result channel and write hashes in large batches so SQLite
    // isn't hit with a separate transaction for every single file.
    const HASH_BATCH: usize = 1_000;
    let mut conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let _ = conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA cache_size = -64000;
         PRAGMA temp_store = MEMORY;",
    );

    let now_str = Utc::now().to_rfc3339();
    let mut pending: Vec<(String, String)> = Vec::with_capacity(HASH_BATCH);

    let flush = |conn: &mut Connection, batch: &[(String, String)], ts: &str| {
        if batch.is_empty() { return; }
        let tx = conn.transaction();
        if let Ok(tx) = tx {
            for (id, hash) in batch {
                let _ = tx.execute(
                    "UPDATE file_instances SET blake3_hash = ?1, stage_2_at = ?2 WHERE id = ?3",
                    rusqlite::params![hash, ts, id],
                );
            }
            let _ = tx.commit();
        }
    };

    for result in result_rx {
        pending.push(result);
        if pending.len() >= HASH_BATCH {
            flush(&mut conn, &pending, &now_str);
            pending.clear();
        }
    }
    flush(&mut conn, &pending, &now_str);

    let _ = hash_thread.join();

    // Emit a definitive "completed" event in case the last batch didn't land on a 500 boundary
    let final_count = *hashed_counter.lock().unwrap();
    let final_prog = ScanProgress {
        source_id: source_id_progress.clone(),
        status: "completed".into(),
        stage: 2,
        files_found: total_files,
        files_inserted: final_count,
        bytes_found: 0,
        total_used_bytes,
    };
    if let Ok(mut scans) = pipeline.active_scans.lock() {
        scans.insert(source_id_progress.clone(), final_prog.clone());
    }
    let _ = app.emit("pipeline://progress", final_prog);
}

/// Hash a single file using BLAKE3. Uses mmap for large files (>= 64 MB)
/// and a buffered reader for small files to keep memory pressure low.
fn hash_file(path: &str) -> Result<String, std::io::Error> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len();

    // For large files, let the OS memory-map the file — fastest possible path
    if file_size >= 64 * 1024 * 1024 {
        // Safety: the file is read-only and we don't modify it during hashing
        let mmap = unsafe { memmap2::Mmap::map(&file) };
        if let Ok(mmap) = mmap {
            let hash = blake3::hash(&mmap);
            return Ok(hash.to_hex().to_string());
        }
        // Fall through to buffered reader on mmap failure
    }

    // Buffered read with an 8 MB buffer
    let mut reader = BufReader::with_capacity(8 * 1024 * 1024, file);
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 8 * 1024 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// Public wrapper so the on-demand hash command in library.rs can call the same hash logic.
pub fn hash_file_public(path: &str) -> Result<String, String> {
    hash_file(path).map_err(|e| e.to_string())
}
