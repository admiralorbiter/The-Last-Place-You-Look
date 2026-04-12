use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    pub id: String,
    pub source_id: String,
    pub source_name: String,
    pub currently_mounted: bool,
    pub file_name: String,
    pub volume_relative_path: String,
    pub extension: Option<String>,
    pub size_bytes: u64,
    pub modified_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileDetail {
    pub id: String,
    pub asset_id: Option<String>,
    pub source_id: String,
    pub source_name: String,
    pub currently_mounted: bool,
    pub file_name: String,
    pub current_path: Option<String>,
    pub volume_relative_path: String,
    pub extension: Option<String>,
    pub size_bytes: u64,
    pub modified_at: String,
    pub created_at_fs: Option<String>,
    pub stage_2_at: Option<String>,
    pub blake3_hash: Option<String>,
    pub quarantine_status: String,
    pub thumbnail_at: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LibraryQuery {
    pub search_term: Option<String>,
    pub source_ids: Vec<String>,
    pub extensions: Vec<String>,
    pub status_filter: Option<String>, // "online", "all"
    pub sort_by: SortBy,
    pub sort_dir: SortDir,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub enum SortBy {
    #[default]
    ModifiedAt,
    SizeBytes,
    FileName,
    Extension,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub enum SortDir {
    Asc,
    #[default]
    Desc,
}

impl SortBy {
    pub fn to_sql(&self) -> &'static str {
        match self {
            SortBy::ModifiedAt => "f.modified_at",
            SortBy::SizeBytes => "f.size_bytes",
            SortBy::FileName => "f.file_name",
            SortBy::Extension => "f.extension",
        }
    }
}

impl SortDir {
    pub fn to_sql(&self) -> &'static str {
        match self {
            SortDir::Asc => "ASC",
            SortDir::Desc => "DESC",
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPage {
    pub items: Vec<LibraryItem>,
    pub total_count: u32,
    pub page: u32,
    pub page_size: u32,
    pub extension_facets: Vec<(String, u32)>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    pub total_files: u64,
    pub total_size_bytes: u64,
    pub sources_count: u32,
}
