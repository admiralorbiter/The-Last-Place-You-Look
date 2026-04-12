use std::sync::{Arc, Mutex};
use rusqlite::{params, Connection};
use tauri::{State, AppHandle, Manager};
use uuid::Uuid;
use chrono::Utc;

use crate::errors::AppError;
use crate::domain::storage_source::StorageSource;
use crate::persistence::volume::resolve_volume_guid;

#[derive(serde::Serialize)]
pub struct AddSourceResult {
    pub source: StorageSource,
    pub warnings: Vec<String>,
}

#[tauri::command]
pub async fn add_storage_source(
    state: State<'_, Arc<Mutex<Connection>>>,
    path: String,
    display_name: String,
    source_kind: String,
) -> Result<AddSourceResult, AppError> {
    let path_obj = std::path::Path::new(&path);
    let volume_guid = resolve_volume_guid(path_obj)?;

    let conn = state.inner().lock().unwrap();
    
    // Check if a non-removed source with the same GUID already exists
    let exists: bool = conn.query_row(
        "SELECT 1 FROM storage_sources WHERE stable_volume_identity = ? AND removed_at IS NULL",
        params![volume_guid],
        |_| Ok(true)
    ).unwrap_or(false);

    if exists {
        return Err(AppError::InvalidInput("A source with this volume identity is already registered.".into()));
    }

    let quarantine_root = path_obj.join(".tlpyl-quarantine");
    let quarantine_str = quarantine_root.to_string_lossy().to_string();

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let source = StorageSource {
        id: id.clone(),
        display_name,
        source_kind,
        stable_volume_identity: volume_guid,
        current_mount_path: Some(path.clone()),
        currently_mounted: true,
        quarantine_root: Some(quarantine_str.clone()),
        created_at: now.clone(),
    };

    conn.execute(
        "INSERT INTO storage_sources (id, display_name, source_kind, stable_volume_identity, current_mount_path, currently_mounted, quarantine_root, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            source.id, source.display_name, source.source_kind, source.stable_volume_identity,
            source.current_mount_path, source.currently_mounted, source.quarantine_root, source.created_at
        ]
    )?;

    // We no longer need the lock
    drop(conn);

    let mut warnings = Vec::new();
    if let Err(e) = std::fs::create_dir_all(&quarantine_root) {
        warnings.push(format!("Could not create quarantine directory: {}", e));
    }

    Ok(AddSourceResult { source, warnings })
}


#[tauri::command]
pub async fn remove_storage_source(
    state: State<'_, Arc<Mutex<Connection>>>,
    source_id: String,
) -> Result<(), AppError> {
    let conn = state.inner().lock().unwrap();
    let now = Utc::now().to_rfc3339();

    let updated = conn.execute(
        "UPDATE storage_sources SET removed_at = ?1 WHERE id = ?2 AND removed_at IS NULL",
        params![now, source_id]
    )?;

    if updated == 0 {
        return Err(AppError::NotFound("Source not found or already removed".into()));
    }

    Ok(())
}

#[tauri::command]
pub async fn list_storage_sources(
    state: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Vec<StorageSource>, AppError> {
    let conn = state.inner().lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, display_name, source_kind, stable_volume_identity, current_mount_path, currently_mounted, quarantine_root, created_at 
         FROM storage_sources 
         WHERE removed_at IS NULL 
         ORDER BY created_at ASC"
    )?;

    let iter = stmt.query_map([], |row| {
        Ok(StorageSource {
            id: row.get(0)?,
            display_name: row.get(1)?,
            source_kind: row.get(2)?,
            stable_volume_identity: row.get(3)?,
            current_mount_path: row.get(4)?,
            currently_mounted: row.get(5)?,
            quarantine_root: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;

    let mut sources = Vec::new();
    for row in iter {
        sources.push(row?);
    }
    
    Ok(sources)
}
