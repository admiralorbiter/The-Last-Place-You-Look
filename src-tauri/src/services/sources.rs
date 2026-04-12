use rusqlite::{params, Connection};
use tauri::{AppHandle, Emitter};
use crate::errors::AppError;

pub fn reconcile_mount_status(
    conn: &Connection,
    app_handle: &AppHandle,
) -> Result<(), AppError> {
    #[cfg(windows)]
    {
        use windows::Win32::Storage::FileSystem::{FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, GetVolumePathNamesForVolumeNameW};
        use std::collections::HashMap;

        // 1. Get all known volumes
        let mut mounted_volumes = HashMap::new();
        let mut buf = vec![0u16; 50];
        
        unsafe {
            let handle = match FindFirstVolumeW(&mut buf) {
                Ok(h) => h,
                Err(_) => return Ok(()), // If it fails, assume no volumes or skip
            };

            loop {
                let guid = String::from_utf16_lossy(&buf).trim_end_matches('\0').to_string();
                
                // Try to get mount paths for this volume
                let mut path_buf = vec![0u16; 256];
                let mut return_len = 0;
                let has_paths = GetVolumePathNamesForVolumeNameW(
                    windows::core::PCWSTR(buf.as_ptr()),
                    Some(&mut path_buf),
                    &mut return_len
                ).is_ok();
                
                if has_paths {
                    let path = String::from_utf16_lossy(&path_buf).trim_end_matches('\0').to_string();
                    if !path.is_empty() {
                        mounted_volumes.insert(guid, path);
                    }
                }

                buf.fill(0);
                if FindNextVolumeW(handle, &mut buf).is_err() {
                    break;
                }
            }
            
            let _ = FindVolumeClose(handle);
        }

        // 2. Reconcile with DB
        let mut stmt = conn.prepare("SELECT id, stable_volume_identity FROM storage_sources WHERE removed_at IS NULL")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (id, guid) = row?;
            
            if let Some(mount_path) = mounted_volumes.get(&guid) {
                // Determine if we need to update
                conn.execute(
                    "UPDATE storage_sources SET currently_mounted = 1, current_mount_path = ?1 WHERE id = ?2",
                    params![mount_path, id]
                )?;
            } else {
                conn.execute(
                    "UPDATE storage_sources SET currently_mounted = 0, current_mount_path = NULL WHERE id = ?1",
                    params![id]
                )?;
            }
        }
    }
    
    // 3. Emit event to frontend
    app_handle.emit("sources://status_updated", ()).ok();

    Ok(())
}
