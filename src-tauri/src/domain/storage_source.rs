use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSource {
    pub id: String,
    pub display_name: String,
    pub source_kind: String,          // "internal" | "removable"
    pub stable_volume_identity: String,
    pub current_mount_path: Option<String>,
    pub currently_mounted: bool,
    pub quarantine_root: Option<String>,
    pub created_at: String,
}
