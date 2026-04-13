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
#[derive(serde::Serialize)]
pub struct DuplicateGroupMember {
    pub id: String,
    pub file_name: String,
    pub current_path: Option<String>,
    pub volume_relative_path: String,
    pub source_id: String,
    pub source_name: String,
    pub source_kind: String,
    pub size_bytes: i64,
    pub preferred_copy: bool,
    pub is_intentional_backup: bool,
}

#[derive(serde::Serialize)]
pub struct DuplicateGroup {
    pub group_id: String,
    pub confidence: String,
    pub file_name: String,
    pub size_bytes: i64,
    pub members: Vec<DuplicateGroupMember>,
    pub recommended_id: Option<String>,
}

#[derive(serde::Serialize)]
pub struct DuplicateGroupsResult {
    pub confirmed: Vec<DuplicateGroup>,
    pub probable: Vec<DuplicateGroup>,
    pub total_recoverable_bytes: i64,
}

// Helper to determine the recommended copy
fn recommend_best_copy(members: &[DuplicateGroupMember]) -> Option<String> {
    if members.is_empty() { return None; }
    
    // 1. Explicit user pin overrides all
    if let Some(pinned) = members.iter().find(|m| m.preferred_copy) {
        return Some(pinned.id.clone());
    }

    // Default ranking heuristic: lowest score wins.
    let mut best: Option<&DuplicateGroupMember> = None;
    let mut min_score = i32::MAX;

    for m in members {
        let mut score = 0;
        // Penalities
        if m.source_kind == "removable" {
            score += 100;
        }
        let depth = m.volume_relative_path.matches('\\').count() as i32 + 
                    m.volume_relative_path.matches('/').count() as i32;
        score += depth * 10;
        
        let len_score = (m.volume_relative_path.len() / 5) as i32;
        score += len_score;

        if score < min_score {
            min_score = score;
            best = Some(m);
        }
    }

    best.map(|m| m.id.clone())
}

#[tauri::command]
pub async fn list_duplicate_groups(
    db_path: State<'_, Arc<std::path::PathBuf>>,
) -> Result<DuplicateGroupsResult, AppError> {
    let path = (**db_path).clone();

    // Run entirely in a blocking thread with its own dedicated read-only connection.
    // This keeps the shared Arc<Mutex<Connection>> free for all other commands while
    // the heavy GROUP BY scans run.
    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ).map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Read-optimised pragmas for this temp connection
        let _ = conn.execute_batch(
            "PRAGMA query_only = ON;
             PRAGMA cache_size = -32000;
             PRAGMA temp_store = MEMORY;"
        );

        // ── Confirmed groups ─────────────────────────────────────────────
        // Uses idx_dupes_confirmed (blake3_hash, file_name) partial index
        let confirmed_rows: Vec<(String, String, i64, String, String, Option<String>, String, String, String, i64, i32, i32)> = {
            let mut stmt = conn.prepare(
                "WITH dup_hashes AS (
                    SELECT blake3_hash
                    FROM file_instances
                    WHERE deleted_at IS NULL
                      AND blake3_hash IS NOT NULL
                      AND blake3_hash != ''
                      AND NOT EXISTS (
                          SELECT 1 FROM excluded_paths ep
                          WHERE
                            -- folder prefix: scoped to source
                            (ep.pattern_type = 'folder'
                             AND ep.source_id = file_instances.source_id
                             AND file_instances.volume_relative_path LIKE ep.volume_path_prefix || '%')
                            OR
                            -- exact filename: global or scoped
                            (ep.pattern_type = 'file_name'
                             AND file_instances.file_name = ep.volume_path_prefix
                             AND (ep.source_id IS NULL OR ep.source_id = file_instances.source_id))
                            OR
                            -- extension: global or scoped
                            (ep.pattern_type = 'extension'
                             AND file_instances.file_name LIKE '%' || ep.volume_path_prefix
                             AND (ep.source_id IS NULL OR ep.source_id = file_instances.source_id))
                      )
                    GROUP BY blake3_hash
                    HAVING COUNT(*) > 1
                )
                SELECT f.blake3_hash, f.file_name, f.size_bytes,
                       f.id, s.display_name, f.current_path,
                       f.volume_relative_path, s.source_kind, f.source_id,
                       f.size_bytes, f.preferred_copy, f.is_intentional_backup
                FROM file_instances f
                JOIN storage_sources s ON s.id = f.source_id
                JOIN dup_hashes d      ON d.blake3_hash = f.blake3_hash
                WHERE f.deleted_at IS NULL
                  AND NOT EXISTS (
                      SELECT 1 FROM excluded_paths ep
                      WHERE
                        (ep.pattern_type = 'folder'
                         AND ep.source_id = f.source_id
                         AND f.volume_relative_path LIKE ep.volume_path_prefix || '%')
                        OR
                        (ep.pattern_type = 'file_name'
                         AND f.file_name = ep.volume_path_prefix
                         AND (ep.source_id IS NULL OR ep.source_id = f.source_id))
                        OR
                        (ep.pattern_type = 'extension'
                         AND f.file_name LIKE '%' || ep.volume_path_prefix
                         AND (ep.source_id IS NULL OR ep.source_id = f.source_id))
                  )
                ORDER BY f.blake3_hash, f.file_name"
            ).map_err(|e| AppError::DatabaseError(e.to_string()))?;
            let x: Vec<_> = stmt.query_map([], |row| {
                Ok((
                    row.get(0)?, row.get(1)?, row.get(2)?,
                    row.get(3)?, row.get(4)?, row.get(5)?,
                    row.get(6)?, row.get(7)?, row.get(8)?,
                    row.get(9)?, row.get(10)?, row.get(11)?,
                ))
            }).map_err(|e| AppError::DatabaseError(e.to_string()))?.flatten().collect();
            x
        };

        let mut confirmed_map: std::collections::HashMap<String, DuplicateGroup> = Default::default();
        let mut total_recoverable_bytes = 0i64;

        for (hash, f_name, size, id, src_name, cur_path, vol_path, src_kind, src_id, sz, pref, backup) in confirmed_rows {
            let group = confirmed_map.entry(hash.clone()).or_insert_with(|| DuplicateGroup {
                group_id: format!("conf_{}", hash),
                confidence: "confirmed".into(),
                file_name: f_name,
                size_bytes: size,
                members: vec![],
                recommended_id: None,
            });
            group.members.push(DuplicateGroupMember {
                id,
                file_name: group.file_name.clone(),
                current_path: cur_path,
                volume_relative_path: vol_path,
                source_id: src_id,
                source_name: src_name,
                source_kind: src_kind,
                size_bytes: sz,
                preferred_copy: pref > 0,
                is_intentional_backup: backup > 0,
            });
        }

        let mut confirmed: Vec<DuplicateGroup> = confirmed_map.into_values().collect();
        for g in &mut confirmed {
            if g.members.len() > 1 {
                total_recoverable_bytes += g.size_bytes * (g.members.len() as i64 - 1);
            }
            g.recommended_id = recommend_best_copy(&g.members);
        }
        confirmed.retain(|g| g.members.len() > 1);

        // ── Probable groups ──────────────────────────────────────────────
        // Uses idx_dupes_probable (file_name, size_bytes, id) partial index
        let probable_rows: Vec<(String, i64, String, String, Option<String>, String, String, String, i64, i32, i32)> = {
            let mut stmt = conn.prepare(
                "WITH dup_keys AS (
                    -- 512KB floor: skip icons, configs, thumbnails — only meaningful files
                    SELECT file_name, size_bytes
                    FROM file_instances
                    WHERE deleted_at IS NULL
                      AND (blake3_hash IS NULL OR blake3_hash = '')
                      AND size_bytes >= 524288
                      AND NOT EXISTS (
                          SELECT 1 FROM excluded_paths ep
                          WHERE
                            (ep.pattern_type = 'folder'
                             AND ep.source_id = file_instances.source_id
                             AND file_instances.volume_relative_path LIKE ep.volume_path_prefix || '%')
                            OR
                            (ep.pattern_type = 'file_name'
                             AND file_instances.file_name = ep.volume_path_prefix
                             AND (ep.source_id IS NULL OR ep.source_id = file_instances.source_id))
                            OR
                            (ep.pattern_type = 'extension'
                             AND file_instances.file_name LIKE '%' || ep.volume_path_prefix
                             AND (ep.source_id IS NULL OR ep.source_id = file_instances.source_id))
                      )
                    GROUP BY file_name, size_bytes
                    HAVING COUNT(*) > 1
                )
                SELECT f.file_name, f.size_bytes,
                       f.id, s.display_name, f.current_path,
                       f.volume_relative_path, s.source_kind, f.source_id,
                       f.size_bytes, f.preferred_copy, f.is_intentional_backup
                FROM file_instances f
                JOIN storage_sources s ON s.id = f.source_id
                JOIN dup_keys dk ON dk.file_name = f.file_name
                                 AND dk.size_bytes = f.size_bytes
                WHERE f.deleted_at IS NULL
                  AND (f.blake3_hash IS NULL OR f.blake3_hash = '')
                  AND f.size_bytes >= 524288
                  AND NOT EXISTS (
                      SELECT 1 FROM excluded_paths ep
                      WHERE
                        (ep.pattern_type = 'folder'
                         AND ep.source_id = f.source_id
                         AND f.volume_relative_path LIKE ep.volume_path_prefix || '%')
                        OR
                        (ep.pattern_type = 'file_name'
                         AND f.file_name = ep.volume_path_prefix
                         AND (ep.source_id IS NULL OR ep.source_id = f.source_id))
                        OR
                        (ep.pattern_type = 'extension'
                         AND f.file_name LIKE '%' || ep.volume_path_prefix
                         AND (ep.source_id IS NULL OR ep.source_id = f.source_id))
                  )
                ORDER BY f.size_bytes DESC, f.file_name
                LIMIT 2000"
            ).map_err(|e| AppError::DatabaseError(e.to_string()))?;
            let x: Vec<_> = stmt.query_map([], |row| {
                Ok((
                    row.get(0)?, row.get(1)?,
                    row.get(2)?, row.get(3)?, row.get(4)?,
                    row.get(5)?, row.get(6)?, row.get(7)?,
                    row.get(8)?, row.get(9)?, row.get(10)?,
                ))
            }).map_err(|e| AppError::DatabaseError(e.to_string()))?.flatten().collect();
            x
        };

        let mut probable_map: std::collections::HashMap<(String, i64), DuplicateGroup> = Default::default();
        for (f_name, size, id, src_name, cur_path, vol_path, src_kind, src_id, sz, pref, backup) in probable_rows {
            let key = (f_name.clone(), size);
            let group = probable_map.entry(key).or_insert_with(|| DuplicateGroup {
                group_id: format!("prob_{}_{}", f_name, size),
                confidence: "probable".into(),
                file_name: f_name.clone(),
                size_bytes: size,
                members: vec![],
                recommended_id: None,
            });
            group.members.push(DuplicateGroupMember {
                id,
                file_name: f_name,
                current_path: cur_path,
                volume_relative_path: vol_path,
                source_id: src_id,
                source_name: src_name,
                source_kind: src_kind,
                size_bytes: sz,
                preferred_copy: pref > 0,
                is_intentional_backup: backup > 0,
            });
        }

        let mut probable: Vec<DuplicateGroup> = probable_map.into_values().collect();
        for g in &mut probable {
            g.recommended_id = recommend_best_copy(&g.members);
        }
        probable.retain(|g| g.members.len() > 1);

        Ok(DuplicateGroupsResult { confirmed, probable, total_recoverable_bytes })
    })
    .await
    .map_err(|_| AppError::IoError("Duplicate analysis task panicked".into()))?
}


#[tauri::command]
pub async fn set_preferred_copy(
    db: State<'_, Arc<Mutex<Connection>>>,
    file_id: String,
    group_member_ids: Vec<String>, 
) -> Result<(), AppError> {
    let mut conn = db.lock().unwrap();
    let tx = conn.transaction()?;
    for id in &group_member_ids {
        if id == &file_id {
            tx.execute("UPDATE file_instances SET preferred_copy = 1 WHERE id = ?", rusqlite::params![id])?;
        } else {
            tx.execute("UPDATE file_instances SET preferred_copy = 0 WHERE id = ?", rusqlite::params![id])?;
        }
    }
    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub async fn set_duplicate_note(
    db: State<'_, Arc<Mutex<Connection>>>,
    file_id: String,
    note: String,
) -> Result<(), AppError> {
    let conn = db.lock().unwrap();
    conn.execute(
        "UPDATE file_instances SET duplicate_note = ? WHERE id = ?",
        rusqlite::params![note, file_id],
    )?;
    Ok(())
}

#[tauri::command]
pub async fn verify_probable_group(
    db: State<'_, Arc<Mutex<Connection>>>,
    file_ids: Vec<String>,
) -> Result<bool, AppError> {
    let mut paths_to_hash = Vec::new();
    {
        let conn = db.lock().unwrap();
        for id in &file_ids {
            let p: Option<String> = conn.query_row(
                "SELECT current_path FROM file_instances WHERE id = ?", 
                rusqlite::params![id], 
                |r| r.get(0)
            ).unwrap_or(None);
            
            if let Some(path) = p {
                paths_to_hash.push((id.clone(), path));
            } else {
                return Err(AppError::InvalidInput("Cannot verify: one or more files in this group are offline.".into()));
            }
        }
    }

    let mut hashes = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();

    for (id, path) in paths_to_hash {
        let hash = tokio::task::spawn_blocking(move || {
            crate::services::pipeline::hash_file_public(&path)
        })
        .await
        .map_err(|_| AppError::IoError("Hash task panicked".into()))?
        .map_err(AppError::IoError)?;

        hashes.push((id, hash));
    }

    {
        let mut conn = db.lock().unwrap();
        let tx = conn.transaction()?;
        for (id, hash) in &hashes {
            tx.execute(
                "UPDATE file_instances SET blake3_hash = ?1, stage_2_at = ?2 WHERE id = ?3",
                rusqlite::params![hash, now, id],
            )?;
        }
        tx.commit()?;
    }

    if hashes.is_empty() { return Ok(false); }
    let first_hash = &hashes[0].1;
    let all_match = hashes.iter().all(|(_, h)| h == first_hash);

    Ok(all_match)
}

// ── Folder exclusion commands ────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExcludedPath {
    pub id: String,
    pub source_id: Option<String>,
    pub source_name: Option<String>,
    pub volume_path_prefix: String,
    pub pattern_type: String,
    pub label: Option<String>,
    pub created_at: String,
}

#[tauri::command]
pub async fn add_excluded_path(
    db: State<'_, Arc<Mutex<Connection>>>,
    source_id: Option<String>,   // None = applies globally (all sources)
    volume_path_prefix: String,
    pattern_type: String,         // 'folder' | 'file_name' | 'extension'
    label: Option<String>,
) -> Result<String, AppError> {
    let conn = db.lock().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    // For folder type: strip trailing separators so LIKE prefix% works uniformly
    let value = if pattern_type == "folder" {
        volume_path_prefix.trim_end_matches(['/', '\\']).to_string()
    } else {
        volume_path_prefix
    };
    let kind = match pattern_type.as_str() {
        "file_name" | "extension" => pattern_type,
        _ => "folder".to_string(),
    };
    conn.execute(
        "INSERT INTO excluded_paths (id, source_id, volume_path_prefix, pattern_type, label, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, source_id, value, kind, label, now],
    )?;
    Ok(id)
}

#[tauri::command]
pub async fn remove_excluded_path(
    db: State<'_, Arc<Mutex<Connection>>>,
    id: String,
) -> Result<(), AppError> {
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM excluded_paths WHERE id = ?", rusqlite::params![id])?;
    Ok(())
}

#[tauri::command]
pub async fn list_excluded_paths(
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Vec<ExcludedPath>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT ep.id, ep.source_id, s.display_name, ep.volume_path_prefix, ep.pattern_type, ep.label, ep.created_at
         FROM excluded_paths ep
         LEFT JOIN storage_sources s ON s.id = ep.source_id
         ORDER BY ep.created_at DESC"
    )?;
    let rows: Vec<ExcludedPath> = stmt.query_map([], |row| {
        Ok(ExcludedPath {
            id: row.get(0)?,
            source_id: row.get(1)?,
            source_name: row.get(2)?,
            volume_path_prefix: row.get(3)?,
            pattern_type: row.get(4)?,
            label: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?.flatten().collect();
    Ok(rows)
}

// ── Intentional backup command ───────────────────────────────────────────────

#[tauri::command]
pub async fn set_intentional_backup(
    db: State<'_, Arc<Mutex<Connection>>>,
    file_id: String,
    is_backup: bool,
) -> Result<(), AppError> {
    let conn = db.lock().unwrap();
    conn.execute(
        "UPDATE file_instances SET is_intentional_backup = ?1 WHERE id = ?2",
        rusqlite::params![is_backup as i32, file_id],
    )?;
    Ok(())
}

// ── Folder Cluster Analysis ──────────────────────────────────────────────────

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher, DefaultHasher};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FolderCluster {
    pub id: String,
    pub folder_a: FolderSide,
    pub folder_b: FolderSide,
    pub similarity: f64,
    pub jaccard: f64,
    pub common_files: usize,
    pub common_bytes: u64,
    pub child_cluster_count: usize,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct FolderSide {
    pub source_id: String,
    pub source_name: String,
    pub folder_path: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub only_here: Vec<DiffFile>,
    pub only_here_total: usize,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DiffFile {
    pub file_name: String,
    pub size_bytes: u64,
    pub volume_relative_path: String,
}

#[derive(Clone, Default)]
struct FolderProfile {
    source_id: String,
    source_name: String,
    folder_path: String,
    files: Vec<DiffFile>,
    total_bytes: u64,
    file_count: usize,
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0; b.len() + 1]; a.len() + 1];
    for i in 0..=a.len() { d[i][0] = i; }
    for j in 0..=b.len() { d[0][j] = j; }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
        }
    }
    d[a.len()][b.len()]
}

fn name_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() { return 1.0; }
    let a_leaf = a.split(&['/', '\\']).last().unwrap_or(a);
    let b_leaf = b.split(&['/', '\\']).last().unwrap_or(b);
    let max_len = a_leaf.chars().count().max(b_leaf.chars().count());
    if max_len == 0 { return 1.0; }
    let dist = levenshtein(a_leaf, b_leaf);
    1.0 - (dist as f64 / max_len as f64)
}

fn path_depth(path: &str) -> usize {
    path.chars().filter(|c| *c == '/' || *c == '\\').count()
}

fn is_ancestor(ancestor: &str, child: &str) -> bool {
    if ancestor.is_empty() { return true; }
    let a = ancestor.replace("\\", "/");
    let c = child.replace("\\", "/");
    c == a || c.starts_with(&format!("{}/", a))
}

#[tauri::command]
pub async fn analyze_folder_clusters(
    db: State<'_, Arc<Mutex<Connection>>>,
    min_similarity: f64,
    min_files: usize,
) -> Result<Vec<FolderCluster>, AppError> {
    println!("analyze_folder_clusters: Started with min_sim={}, min_files={}", min_similarity, min_files);
    let start_ts = std::time::Instant::now();
    
    let conn = db.lock().unwrap();
    println!("analyze_folder_clusters: Acquired DB lock. Executing query...");

    // 1. Load active files, skipping noise exclusions
    let mut stmt = conn.prepare(
        "SELECT f.file_name, f.size_bytes, f.source_id, s.display_name, f.volume_relative_path
         FROM file_instances f
         JOIN storage_sources s ON s.id = f.source_id
         WHERE f.deleted_at IS NULL
           AND NOT EXISTS (
               SELECT 1 FROM excluded_paths ep
               WHERE
                 (ep.pattern_type = 'folder' AND ep.source_id = f.source_id AND f.volume_relative_path LIKE ep.volume_path_prefix || '%')
                 OR
                 (ep.pattern_type = 'file_name' AND f.file_name = ep.volume_path_prefix AND (ep.source_id IS NULL OR ep.source_id = f.source_id))
                 OR
                 (ep.pattern_type = 'extension' AND f.file_name LIKE '%' || ep.volume_path_prefix AND (ep.source_id IS NULL OR ep.source_id = f.source_id))
           )"
    )?;

    let mut profiles: HashMap<(String, String), FolderProfile> = HashMap::new();
    let rows: Vec<(String, u64, String, String, String)> = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get::<_, i64>(1)? as u64, row.get(2)?, row.get(3)?, row.get(4)?))
    })?.flatten().collect();
    
    println!("analyze_folder_clusters: Query retrieved {} rows. Building profiles...", rows.len());

    for (file_name, size_bytes, source_id, source_name, vol_path) in rows {
        let sep = if vol_path.contains('\\') { '\\' } else { '/' };
        let folder_path = match vol_path.rsplit_once(sep) {
            Some((dir, _)) => dir.to_string(),
            None => "".to_string(),
        };

        let fkey = (source_id.clone(), folder_path.clone());
        let profile = profiles.entry(fkey).or_insert_with(|| FolderProfile {
            source_id,
            source_name,
            folder_path,
            files: Vec::new(),
            total_bytes: 0,
            file_count: 0,
        });

        profile.files.push(DiffFile { file_name, size_bytes, volume_relative_path: vol_path });
        profile.total_bytes += size_bytes;
        profile.file_count += 1;
    }

    println!("analyze_folder_clusters: Built {} folder profiles. Building inverted index...", profiles.len());

    // 2. Build inverted index
    // File key -> [Folder keys containing it]
    let mut inverted_index: HashMap<(String, u64), Vec<(String, String)>> = HashMap::new();
    for (fkey, profile) in &profiles {
        if profile.file_count < min_files { continue; }
        for file in &profile.files {
            inverted_index.entry((file.file_name.clone(), file.size_bytes)).or_default().push(fkey.clone());
        }
    }

    // Tally candidates
    let mut pair_tallies: HashMap<((String, String), (String, String)), (usize, u64)> = HashMap::new();

    for ((_fi_name, fi_size), folders) in &inverted_index {
        if folders.len() < 2 { continue; }
        
        // Defensive O(N^2) cap: If a single file name+size appears in > 100 unique folders
        // across the DB, we skip it. This prevents the candidate pair generator from hanging
        // deeply on things like `.DS_Store`, `__init__.py`, or `.git/objects`.
        if folders.len() > 100 { continue; }

        for i in 0..folders.len() {
            for j in (i+1)..folders.len() {
                let f1 = &folders[i];
                let f2 = &folders[j];
                let key = if f1 < f2 { (f1.clone(), f2.clone()) } else { (f2.clone(), f1.clone()) };
                
                let tally = pair_tallies.entry(key).or_insert((0, 0));
                tally.0 += 1;
                tally.1 += *fi_size;
            }
        }
    }
    
    println!("analyze_folder_clusters: Tallied pairs. Scoring candidates...");

    // 3. Score candidates
    let mut surviving_pairs = Vec::new();
    let mut evaluation_count = 0;
    
    for (pair_key, (common_files, common_bytes)) in pair_tallies {
        evaluation_count += 1;
        if common_files < min_files { continue; }
        let p_a = profiles.get(&pair_key.0).unwrap();
        let p_b = profiles.get(&pair_key.1).unwrap();
        
        let jaccard = common_files as f64 / (p_a.file_count + p_b.file_count - common_files) as f64;
        let byte_overlap = common_bytes as f64 / (p_a.total_bytes.max(p_b.total_bytes).max(1) as f64);
        let name_sim = name_similarity(&p_a.folder_path, &p_b.folder_path);
        
        let composite = (jaccard * 0.55) + (byte_overlap * 0.35) + (name_sim * 0.10);
        
        if composite >= min_similarity {
            surviving_pairs.push((pair_key, composite, jaccard, common_files, common_bytes));
        }
    }
    
    println!("analyze_folder_clusters: Evaluated {} valid pairs, {} survived threshold. Rolling up ancestors...", evaluation_count, surviving_pairs.len());

    // 4. Ancestor Roll-Up (Shallowest First)
    // IMPORTANT: Cap to top 3000 by composite score BEFORE O(N²) roll-up.
    // With 67k+ candidates this loop would take billions of iterations.
    surviving_pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    surviving_pairs.truncate(3000);
    println!("analyze_folder_clusters: Capped to {} pairs for roll-up.", surviving_pairs.len());
    
    // Re-sort by depth (shallowest first) for ancestor suppression
    surviving_pairs.sort_by(|a, b| {
        let d_a = path_depth(&a.0.0.1) + path_depth(&a.0.1.1);
        let d_b = path_depth(&b.0.0.1) + path_depth(&b.0.1.1);
        d_a.cmp(&d_b)
    });

    let mut accepted_pairs: Vec<( ((String, String), (String, String)), f64, f64, usize, u64, usize)> = Vec::new();

    for pair in surviving_pairs {
        let mut covered_by_idx = None;
        for (i, acc) in accepted_pairs.iter().enumerate() {
            // acc.2 is the jaccard score. If the parent is a perfect match (≥99.9%),
            // we know all files in both folders are identical. In that case, suppress
            // any candidate where EITHER side is a descendant — because the other side
            // is necessarily already implied by the parent's coverage.
            // For partial-match parents, require BOTH sides to be descendants.
            let parent_is_perfect = acc.2 >= 0.999;

            if pair.0.0.0 == acc.0.0.0 && pair.0.1.0 == acc.0.1.0 {
                let a_covered = is_ancestor(&acc.0.0.1, &pair.0.0.1);
                let b_covered = is_ancestor(&acc.0.1.1, &pair.0.1.1);
                let suppressed = if parent_is_perfect { a_covered || b_covered } else { a_covered && b_covered };
                if suppressed { covered_by_idx = Some(i); break; }
            }
            if pair.0.0.0 == acc.0.1.0 && pair.0.1.0 == acc.0.0.0 {
                let a_covered = is_ancestor(&acc.0.1.1, &pair.0.0.1);
                let b_covered = is_ancestor(&acc.0.0.1, &pair.0.1.1);
                let suppressed = if parent_is_perfect { a_covered || b_covered } else { a_covered && b_covered };
                if suppressed { covered_by_idx = Some(i); break; }
            }
        }
        
        if let Some(idx) = covered_by_idx {
            accepted_pairs[idx].5 += 1;
        } else {
            accepted_pairs.push((pair.0, pair.1, pair.2, pair.3, pair.4, 0));
        }
    }
    
    println!("analyze_folder_clusters: After roll-up, {} final clusters remain. Generating diffs...", accepted_pairs.len());

    // 5. Generate Diff Response
    let mut results = Vec::new();
    for (pair_key, sim, jaccard, common_files, common_bytes, child_count) in accepted_pairs {
        let p_a = profiles.get(&pair_key.0).unwrap();
        let p_b = profiles.get(&pair_key.1).unwrap();
        
        let set_b: HashSet<_> = p_b.files.iter().map(|f| (f.file_name.clone(), f.size_bytes)).collect();
        let mut only_in_a: Vec<_> = p_a.files.iter().filter(|f| !set_b.contains(&(f.file_name.clone(), f.size_bytes))).cloned().collect();
        
        let set_a: HashSet<_> = p_a.files.iter().map(|f| (f.file_name.clone(), f.size_bytes)).collect();
        let mut only_in_b: Vec<_> = p_b.files.iter().filter(|f| !set_a.contains(&(f.file_name.clone(), f.size_bytes))).cloned().collect();
        
        only_in_a.sort_by_key(|f| std::cmp::Reverse(f.size_bytes));
        only_in_b.sort_by_key(|f| std::cmp::Reverse(f.size_bytes));
        
        let only_here_total_a = only_in_a.len();
        let only_here_total_b = only_in_b.len();
        
        only_in_a.truncate(20);
        only_in_b.truncate(20);
        
        let side_a = FolderSide {
            source_id: p_a.source_id.clone(),
            source_name: p_a.source_name.clone(),
            folder_path: p_a.folder_path.clone(),
            file_count: p_a.file_count,
            total_bytes: p_a.total_bytes,
            only_here: only_in_a,
            only_here_total: only_here_total_a,
        };
        
        let side_b = FolderSide {
            source_id: p_b.source_id.clone(),
            source_name: p_b.source_name.clone(),
            folder_path: p_b.folder_path.clone(),
            file_count: p_b.file_count,
            total_bytes: p_b.total_bytes,
            only_here: only_in_b,
            only_here_total: only_here_total_b,
        };
        
        let mut hasher = DefaultHasher::new();
        pair_key.0.0.hash(&mut hasher);
        pair_key.0.1.hash(&mut hasher);
        pair_key.1.0.hash(&mut hasher);
        pair_key.1.1.hash(&mut hasher);
        let id = format!("cluster_{:x}", hasher.finish());
        
        results.push(FolderCluster {
            id,
            folder_a: side_a,
            folder_b: side_b,
            similarity: sim,
            jaccard,
            common_files,
            common_bytes,
            child_cluster_count: child_count,
        });
    }
    
    results.sort_by(|a, b| b.common_bytes.cmp(&a.common_bytes));
    
    let elapsed = start_ts.elapsed();
    println!("analyze_folder_clusters: Completed successfully in {:?}", elapsed);
    Ok(results)
}
