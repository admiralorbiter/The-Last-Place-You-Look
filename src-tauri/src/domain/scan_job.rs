use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanJob {
    pub id: String,
    pub source_id: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: String,
    pub stage: i32,
    pub files_found: i32,
    pub files_inserted: i32,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub source_id: String,
    pub status: String,
    pub stage: i32,
    pub files_found: i32,
    pub files_inserted: i32,
    pub bytes_found: u64,
    pub total_used_bytes: u64,
}
