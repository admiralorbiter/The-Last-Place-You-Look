use tauri::{State, command};
use std::sync::{Arc, Mutex};
use std::fs;
use tauri::Manager;
use rusqlite::{Connection, Row};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use crate::errors::AppError;
use crate::domain::library::{LibraryItem, LibraryQuery, LibraryPage, LibraryStats, FileDetail};
use crate::services::thumbnails::extract_thumbnail;

#[command]
pub async fn list_library(db: State<'_, Arc<Mutex<Connection>>>, query: LibraryQuery) -> Result<LibraryPage, AppError> {
    search_library(db, query).await
}

fn row_to_item(row: &Row) -> Result<LibraryItem, rusqlite::Error> {
    Ok(LibraryItem {
        id: row.get(0)?,
        source_id: row.get(1)?,
        source_name: row.get(2)?,
        currently_mounted: row.get(3)?,
        file_name: row.get(4)?,
        volume_relative_path: row.get(5)?,
        extension: row.get(6)?,
        size_bytes: row.get::<_, i64>(7)? as u64,
        modified_at: row.get(8)?,
        deleted_at: row.get(9)?,
    })
}

#[command]
pub async fn search_library(db: State<'_, Arc<Mutex<Connection>>>, query: LibraryQuery) -> Result<LibraryPage, AppError> {
    let conn = db.lock().unwrap();

    let mut sql = String::new();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    
    // FTS5 JOIN
    let has_search = query.search_term.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
    
    if has_search {
        sql.push_str("FROM file_search fs JOIN file_instances f ON fs.rowid = f.rowid ");
    } else {
        sql.push_str("FROM file_instances f ");
    }
    sql.push_str("JOIN storage_sources s ON f.source_id = s.id ");
    
    let mut wheres = Vec::new();
    
    wheres.push("f.deleted_at IS NULL".to_string());
    
    if has_search {
        // FTS5 exact match or prefix match
        // Note: we append '*' to the search term to make it prefix match
        let term = format!("{}*", query.search_term.clone().unwrap().trim().replace("\"", ""));
        wheres.push("file_search MATCH ?".to_string());
        params_vec.push(Box::new(term));
    }
    
    if !query.source_ids.is_empty() {
        let placeholders = vec!["?"; query.source_ids.len()].join(", ");
        wheres.push(format!("f.source_id IN ({})", placeholders));
        for sid in &query.source_ids {
            params_vec.push(Box::new(sid.clone()));
        }
    }
    
    if let Some(status) = query.status_filter.as_ref() {
        if status == "online" {
            wheres.push("s.currently_mounted = 1".to_string());
        }
    }
    
    let mut base_wheres = sql.clone();
    if !wheres.is_empty() {
        base_wheres.push_str("WHERE ");
        base_wheres.push_str(&wheres.join(" AND "));
        base_wheres.push_str(" ");
    }
    
    // 1. Calculate Facets (ignoring current extension filter)
    let facet_sql = format!("SELECT f.extension, COUNT(*) {} GROUP BY f.extension ORDER BY COUNT(*) DESC LIMIT 8", base_wheres);
    
    let mut facet_params: Vec<&dyn rusqlite::ToSql> = Vec::new();
    for p in &params_vec { facet_params.push(p.as_ref()); }
    
    let mut extension_facets = Vec::new();
    if let Ok(mut stmt) = conn.prepare(&facet_sql) {
        if let Ok(mut rows) = stmt.query(rusqlite::params_from_iter(facet_params.iter())) {
            while let Ok(Some(row)) = rows.next() {
                let ext: Option<String> = row.get(0).unwrap_or(None);
                let count: u32 = row.get::<_, i64>(1).unwrap_or(0) as u32;
                if let Some(ext) = ext {
                    extension_facets.push((ext, count));
                }
            }
        }
    }
    
    // 2. Now apply the extension filter to the wheres
    if !query.extensions.is_empty() {
        let placeholders = vec!["?"; query.extensions.len()].join(", ");
        wheres.push(format!("LOWER(f.extension) IN ({})", placeholders));
        for ext in &query.extensions {
            params_vec.push(Box::new(ext.to_lowercase()));
        }
    }

    
    let mut final_sql = sql.clone();
    if !wheres.is_empty() {
        final_sql.push_str("WHERE ");
        final_sql.push_str(&wheres.join(" AND "));
        final_sql.push_str(" ");
    }
    
    let count_sql = format!("SELECT COUNT(*) {}", final_sql);
    
    // Borrow params for count query
    let mut count_params: Vec<&dyn rusqlite::ToSql> = Vec::new();
    for p in &params_vec {
        count_params.push(p.as_ref());
    }
    
    let total_count: u32 = conn.query_row(&count_sql, rusqlite::params_from_iter(count_params.iter()), |row| row.get(0)).unwrap_or(0);
    
    // Full items query
    let items_sql = format!(
        "SELECT f.id, f.source_id, s.display_name, s.currently_mounted, f.file_name, f.volume_relative_path, f.extension, f.size_bytes, f.modified_at, f.deleted_at {} ORDER BY {} {} LIMIT ? OFFSET ?",
        final_sql, query.sort_by.to_sql(), query.sort_dir.to_sql()
    );
    
    let limit: i64 = query.page_size as i64;
    let offset: i64 = ((query.page.max(1) - 1) * query.page_size) as i64;
    
    params_vec.push(Box::new(limit));
    params_vec.push(Box::new(offset));
    
    let mut items_params: Vec<&dyn rusqlite::ToSql> = Vec::new();
    for p in &params_vec {
        items_params.push(p.as_ref());
    }
    
    let mut stmt = conn.prepare(&items_sql).map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let rows = stmt.query_map(rusqlite::params_from_iter(items_params.iter()), row_to_item).map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    let mut items = Vec::new();
    for row in rows {
        if let Ok(item) = row {
            items.push(item);
        }
    }
    
    Ok(LibraryPage {
        items,
        total_count,
        page: query.page,
        page_size: query.page_size,
        extension_facets,
    })
}

#[command]
pub async fn get_library_stats(db: State<'_, Arc<Mutex<Connection>>>) -> Result<LibraryStats, AppError> {
    let conn = db.lock().unwrap();
    
    let total_files: u64 = conn.query_row("SELECT COUNT(*) FROM file_instances WHERE deleted_at IS NULL", [], |row| row.get::<_, i64>(0)).unwrap_or(0) as u64;
    let total_size_bytes: u64 = conn.query_row("SELECT COALESCE(SUM(size_bytes), 0) FROM file_instances WHERE deleted_at IS NULL", [], |row| row.get::<_, i64>(0)).unwrap_or(0) as u64;
    let sources_count: u32 = conn.query_row("SELECT COUNT(DISTINCT source_id) FROM file_instances WHERE deleted_at IS NULL", [], |row| row.get(0)).unwrap_or(0);
    
    Ok(LibraryStats {
        total_files,
        total_size_bytes,
        sources_count,
    })
}

#[command]
pub async fn get_file_detail(id: String, db: State<'_, Arc<Mutex<Connection>>>) -> Result<FileDetail, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT f.id, f.asset_id, f.source_id, s.display_name, s.currently_mounted,
                f.file_name, f.current_path, f.volume_relative_path, f.extension, f.size_bytes,
                f.modified_at, f.created_at_fs, f.stage_2_at, f.blake3_hash, f.quarantine_status,
                f.thumbnail_at
         FROM file_instances f
         JOIN storage_sources s ON f.source_id = s.id
         WHERE f.id = ?"
    ).map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let detail = stmt.query_row([id], |row| {
        Ok(FileDetail {
            id: row.get(0)?,
            asset_id: row.get(1)?,
            source_id: row.get(2)?,
            source_name: row.get(3)?,
            currently_mounted: row.get(4)?,
            file_name: row.get(5)?,
            current_path: row.get(6)?,
            volume_relative_path: row.get(7)?,
            extension: row.get(8)?,
            size_bytes: row.get::<_, i64>(9)? as u64,
            modified_at: row.get(10)?,
            created_at_fs: row.get(11)?,
            stage_2_at: row.get(12)?,
            blake3_hash: row.get(13)?,
            quarantine_status: row.get(14)?,
            thumbnail_at: row.get(15)?,
        })
    }).map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(detail)
}

#[command]
pub async fn get_thumbnail(id: String, app: tauri::AppHandle, db: State<'_, Arc<Mutex<Connection>>>) -> Result<String, AppError> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| AppError::IoError(e.to_string()))?;
    let thumbnails_dir = app_data_dir.join("thumbnails");
    
    if !thumbnails_dir.exists() {
        fs::create_dir_all(&thumbnails_dir).map_err(|e| AppError::IoError(e.to_string()))?;
    }
    
    let thumb_path = thumbnails_dir.join(format!("{}.png", id));
    
    if thumb_path.exists() {
        let bytes = fs::read(&thumb_path).map_err(|e| AppError::IoError(e.to_string()))?;
        return Ok(format!("data:image/png;base64,{}", BASE64.encode(&bytes)));
    }
    
    let current_path: Option<String> = {
        let conn = db.lock().unwrap();
        conn.query_row(
            "SELECT current_path FROM file_instances WHERE id = ?",
            [&id],
            |row| row.get(0)
        ).unwrap_or(None)
    };
    
    let file_path_str = match current_path {
        Some(p) => p,
        None => return Err(AppError::InvalidInput("File is offline or has no known path".into())),
    };
    
    let file_path_buf = std::path::PathBuf::from(file_path_str);
    
    let bytes = tokio::task::spawn_blocking(move || {
        extract_thumbnail(file_path_buf.as_path(), 256)
    }).await.map_err(|_e| AppError::IoError("Task panicked".into()))?
      .map_err(|e| AppError::IoError(e))?;
    
    fs::write(&thumb_path, &bytes).map_err(|e| AppError::IoError(e.to_string()))?;
    
    let now_str = chrono::Utc::now().to_rfc3339();
    {
        let conn = db.lock().unwrap();
        let _ = conn.execute(
            "UPDATE file_instances SET thumbnail_at = ? WHERE id = ?",
            rusqlite::params![now_str, id]
        );
    }
    
    Ok(format!("data:image/png;base64,{}", BASE64.encode(&bytes)))
}


/// Hash a single file on demand, persist the result, and return the hex hash.
/// If already hashed, returns the cached value immediately.
#[command]
pub async fn hash_single_file(
    db: State<'_, Arc<Mutex<Connection>>>,
    id: String,
) -> Result<String, AppError> {
    // Cache hit?
    let existing: Option<String> = {
        let conn = db.lock().unwrap();
        conn.query_row(
            "SELECT blake3_hash FROM file_instances WHERE id = ?",
            rusqlite::params![id],
            |row| row.get(0),
        ).unwrap_or(None)
    };
    if let Some(h) = existing {
        if !h.is_empty() { return Ok(h); }
    }

    // Get path
    let path: String = {
        let conn = db.lock().unwrap();
        conn.query_row(
            "SELECT current_path FROM file_instances WHERE id = ?",
            rusqlite::params![id],
            |row| row.get::<_, Option<String>>(0),
        ).map_err(|e| AppError::NotFound(e.to_string()))?
         .ok_or_else(|| AppError::InvalidInput("File is offline or has no path".into()))?
    };

    // Hash (blocking)
    let id_clone = id.clone();
    let hash = tokio::task::spawn_blocking(move || {
        crate::services::pipeline::hash_file_public(&path)
    })
    .await
    .map_err(|_| AppError::IoError("Hash task panicked".into()))?
    .map_err(AppError::IoError)?;

    // Persist
    {
        let conn = db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let _ = conn.execute(
            "UPDATE file_instances SET blake3_hash = ?1, stage_2_at = ?2 WHERE id = ?3",
            rusqlite::params![hash, now, id_clone],
        );
    }

    Ok(hash)
}

#[derive(serde::Serialize)]
pub struct DuplicateEntry {
    pub id: String,
    pub file_name: String,
    pub current_path: Option<String>,
    pub source_name: String,
    pub size_bytes: i64,
    pub confidence: String, // "confirmed" | "probable"
}

#[derive(serde::Serialize)]
pub struct DuplicateResult {
    pub confirmed: Vec<DuplicateEntry>,  // same BLAKE3 hash
    pub probable: Vec<DuplicateEntry>,   // same name + same size, not yet hashed
}

/// Two-tier duplicate detection:
///   - Confirmed: files with identical BLAKE3 hash (definitive)
///   - Probable:  files with identical (file_name, size_bytes) but no hash yet (very reliable heuristic)
#[command]
pub async fn find_duplicates(
    db: State<'_, Arc<Mutex<Connection>>>,
    id: String,
) -> Result<DuplicateResult, AppError> {
    let conn = db.lock().unwrap();

    // Get this file's info
    let (hash, file_name, size_bytes): (Option<String>, String, i64) = conn.query_row(
        "SELECT blake3_hash, file_name, size_bytes FROM file_instances WHERE id = ?",
        rusqlite::params![id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|e| AppError::NotFound(e.to_string()))?;

    // Tier 1: Confirmed (hash match — only possible if we have a hash)
    let confirmed = if let Some(h) = &hash {
        if !h.is_empty() {
            let mut stmt = conn.prepare(
                "SELECT f.id, f.file_name, f.current_path, s.display_name, f.size_bytes
                 FROM file_instances f
                 JOIN storage_sources s ON s.id = f.source_id
                 WHERE f.blake3_hash = ?1 AND f.id != ?2 AND f.deleted_at IS NULL
                 ORDER BY f.file_name ASC"
            )?;
            let res: Vec<DuplicateEntry> = stmt.query_map(rusqlite::params![h, id], |row| Ok(DuplicateEntry {
                id: row.get(0)?,
                file_name: row.get(1)?,
                current_path: row.get(2)?,
                source_name: row.get(3)?,
                size_bytes: row.get(4)?,
                confidence: "confirmed".into(),
            }))?.flatten().collect();
            res
        } else { vec![] }
    } else { vec![] };

    // Tier 2: Probable (same name + same size, excluding already-confirmed and self)
    // Exclude files that are already in confirmed results to avoid double-listing.
    let confirmed_ids: Vec<&str> = confirmed.iter().map(|e| e.id.as_str()).collect();
    let mut stmt2 = conn.prepare(
        "SELECT f.id, f.file_name, f.current_path, s.display_name, f.size_bytes
         FROM file_instances f
         JOIN storage_sources s ON s.id = f.source_id
         WHERE f.file_name = ?1 AND f.size_bytes = ?2 AND f.id != ?3 AND f.deleted_at IS NULL
         ORDER BY f.file_name ASC"
    )?;
    let probable: Vec<DuplicateEntry> = stmt2.query_map(
        rusqlite::params![file_name, size_bytes, id],
        |row| Ok(DuplicateEntry {
            id: row.get(0)?,
            file_name: row.get(1)?,
            current_path: row.get(2)?,
            source_name: row.get(3)?,
            size_bytes: row.get(4)?,
            confidence: "probable".into(),
        })
    )?.flatten()
    .filter(|e| !confirmed_ids.contains(&e.id.as_str()))
    .collect();

    Ok(DuplicateResult { confirmed, probable })
}