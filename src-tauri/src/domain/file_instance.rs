use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInstance {
    pub id: String,
    pub asset_id: Option<String>,
    pub source_id: String,
    pub stable_location_id: String,
    pub volume_relative_path: String,
    pub current_path: Option<String>,
    pub file_name: String,
    pub extension: Option<String>,
    pub size_bytes: i64,
    pub modified_at: String,
    pub created_at_fs: Option<String>,

    pub stage_1_at: Option<String>,
    pub stage_2_at: Option<String>,
    pub stage_3_at: Option<String>,
    pub blake3_hash: Option<String>,

    pub deleted_at: Option<String>,
    pub quarantine_status: String,
}
