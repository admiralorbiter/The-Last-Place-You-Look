use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use std::path::Path;
use crate::errors::AppError;

pub fn init_db(app_data_dir: &Path) -> Result<Connection, AppError> {
    std::fs::create_dir_all(app_data_dir)?;
    let db_path = app_data_dir.join("tlpyl.db");
    let mut conn = Connection::open(&db_path)?;

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
    ]);

    migrations.to_latest(&mut conn)
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(conn)
}
