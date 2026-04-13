pub mod commands;
pub mod domain;
pub mod persistence;
pub mod services;
pub mod errors;

use tauri::{Manager, Emitter};
use std::sync::{Arc, Mutex};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(services::pipeline::PipelineManager::new())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
            let conn = persistence::db::init_db(&app_data_dir).expect("failed to init db");
            let db_path = app_data_dir.join("tlpyl.db");
            
            let arc_conn = Arc::new(Mutex::new(conn));
            app.manage(arc_conn.clone());
            // Store path so heavy read-only commands can open their own connection
            app.manage(Arc::new(db_path));
            
            // Reconcile mount status — non-fatal if it fails (e.g. on first run or schema issues)
            match services::sources::reconcile_mount_status(&arc_conn.lock().unwrap(), app.handle()) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("[WARN] reconcile_mount_status failed on startup: {:?}", e);
                }
            }
            
            app.emit("app://ready", ()).ok();
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
            commands::sources::add_storage_source,
            commands::sources::remove_storage_source,
            commands::sources::list_storage_sources,
            commands::pipeline::start_scan,
            commands::pipeline::start_hashing,
            commands::pipeline::get_scan_status,
            commands::pipeline::cancel_scan,
            commands::library::list_library,
            commands::library::search_library,
            commands::library::get_library_stats,
            commands::library::get_file_detail,
            commands::library::get_thumbnail,
            commands::library::hash_single_file,
            commands::library::find_duplicates,
            commands::library::list_duplicate_groups,
            commands::library::set_preferred_copy,
            commands::library::set_duplicate_note,
            commands::library::verify_probable_group,
            commands::library::add_excluded_path,
            commands::library::remove_excluded_path,
            commands::library::list_excluded_paths,
            commands::library::set_intentional_backup,
            commands::os::reveal_in_explorer
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

