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
        // M002 and beyond added in Stage 4
    ]);

    migrations.to_latest(&mut conn)
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(conn)
}
