use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use std::path::Path;
use crate::errors::AppError;

pub fn init_db(app_data_dir: &Path) -> Result<Connection, AppError> {
    std::fs::create_dir_all(app_data_dir)?;
    let db_path = app_data_dir.join("tlpyl.db");
    let mut conn = Connection::open(&db_path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA cache_size = -64000;
         PRAGMA temp_store = MEMORY;",
    )?;

    let migrations = Migrations::new(vec![
        M::up("-- M001: baseline (empty)"),
        M::up("
            CREATE TABLE storage_sources (
                id                     TEXT PRIMARY KEY,
                display_name           TEXT NOT NULL,
                source_kind            TEXT NOT NULL,
                stable_volume_identity TEXT NOT NULL UNIQUE,
                current_mount_path     TEXT,
                currently_mounted      INTEGER NOT NULL DEFAULT 0,
                quarantine_root        TEXT,
                created_at             TEXT NOT NULL,
                removed_at             TEXT
            );
        "),
        M::up("
            CREATE TABLE file_instances (
                id                    TEXT PRIMARY KEY,
                asset_id              TEXT,
                source_id             TEXT NOT NULL REFERENCES storage_sources(id),
                stable_location_id    TEXT NOT NULL,
                volume_relative_path  TEXT NOT NULL,
                current_path          TEXT,
                file_name             TEXT NOT NULL,
                extension             TEXT,
                size_bytes            INTEGER NOT NULL,
                modified_at           TEXT NOT NULL,
                created_at_fs         TEXT,
                
                stage_1_at            TEXT,
                stage_2_at            TEXT,
                stage_3_at            TEXT,
                blake3_hash           TEXT,
                
                deleted_at            TEXT,
                quarantine_status     TEXT NOT NULL DEFAULT 'none',
                
                UNIQUE(source_id, volume_relative_path)
            );
            CREATE INDEX idx_file_instances_source ON file_instances(source_id);
            CREATE INDEX idx_file_instances_hash ON file_instances(blake3_hash) WHERE blake3_hash IS NOT NULL;
        "),
        M::up("
            CREATE TABLE scan_jobs (
                id            TEXT PRIMARY KEY,
                source_id     TEXT NOT NULL REFERENCES storage_sources(id),
                started_at    TEXT NOT NULL,
                completed_at  TEXT,
                status        TEXT NOT NULL DEFAULT 'running',
                stage         INTEGER NOT NULL DEFAULT 1,
                files_found   INTEGER NOT NULL DEFAULT 0,
                files_inserted INTEGER NOT NULL DEFAULT 0,
                error_message TEXT
            );
        "),
    ]);

    migrations.to_latest(&mut conn)
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(conn)
}
