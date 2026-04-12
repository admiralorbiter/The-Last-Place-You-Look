use tauri::{State, command};
use std::sync::{Arc, Mutex};
use rusqlite::{Connection, Row};
use crate::errors::AppError;
use crate::domain::library::{LibraryItem, LibraryQuery, LibraryPage, LibraryStats};

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


