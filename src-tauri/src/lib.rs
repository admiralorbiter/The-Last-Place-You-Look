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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
            let conn = persistence::db::init_db(&app_data_dir).expect("failed to init db");
            
            let arc_conn = Arc::new(Mutex::new(conn));
            app.manage(arc_conn.clone());
            
            services::sources::reconcile_mount_status(&arc_conn.lock().unwrap(), app.handle())
                .expect("failed to reconcile mounts on startup");
            
            app.emit("app://ready", ()).ok();
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
            commands::sources::add_storage_source,
            commands::sources::remove_storage_source,
            commands::sources::list_storage_sources,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

