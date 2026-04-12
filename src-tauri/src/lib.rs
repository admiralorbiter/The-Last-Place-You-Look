pub mod commands;
pub mod domain;
pub mod persistence;
pub mod services;
pub mod errors;

use tauri::Manager;
use std::sync::{Arc, Mutex};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
            let conn = persistence::db::init_db(&app_data_dir).expect("failed to init db");
            app.manage(Arc::new(Mutex::new(conn)));
            
            app.emit("app://ready", ()).ok();
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::app::get_app_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

