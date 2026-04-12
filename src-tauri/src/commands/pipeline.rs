use tauri::{AppHandle, State, command};
use crate::errors::AppError;
use crate::domain::scan_job::{ScanJob, ScanProgress};
use crate::services::pipeline::{start_scan as service_start_scan, start_hashing as service_start_hashing, PipelineManager};

#[command]
pub async fn start_scan(app: AppHandle, source_id: String) -> Result<ScanJob, AppError> {
    service_start_scan(app, source_id).await
}

#[command]
pub async fn start_hashing(app: AppHandle, source_id: String) -> Result<(), AppError> {
    service_start_hashing(app, source_id).await
}

#[command]
pub async fn get_scan_status(pipeline: State<'_, PipelineManager>) -> Result<Vec<ScanProgress>, AppError> {
    let scans = pipeline.active_scans.lock().unwrap();
    let values: Vec<ScanProgress> = scans.values().cloned().collect();
    Ok(values)
}

#[command]
pub async fn cancel_scan(_app: AppHandle, _source_id: String) -> Result<(), AppError> {
    // For MVP phase 1, cancellation is a stub. We will implement token-based passing in the future if needed.
    // Or we handle stage-by-stage soft stop.
    Err(AppError::InvalidInput("Cancellation not yet implemented".into()))
}
