use serde::Serialize;
use crate::errors::AppError;

#[derive(Serialize)]
pub struct AppInfo {
    pub version: String,
    pub db_status: String,
}

#[tauri::command]
pub async fn get_app_info() -> Result<AppInfo, AppError> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        db_status: "ok".to_string(),
    })
}
