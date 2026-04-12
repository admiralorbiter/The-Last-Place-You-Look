use tauri::command;
use std::process::Command;
use crate::errors::AppError;

#[command]
pub async fn reveal_in_explorer(path: String) -> Result<(), AppError> {
    // Only works on Windows natively
    let result = Command::new("explorer")
        .arg("/select,")
        .arg(&path)
        .spawn();
        
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::PlatformError(format!("Failed to open explorer: {}", e)))
    }
}
