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
        M::up("
            -- M005: 5 critical indexes for sorting and filtering
            CREATE INDEX idx_file_instances_modified ON file_instances(modified_at DESC);
            CREATE INDEX idx_file_instances_size     ON file_instances(size_bytes DESC);
            CREATE INDEX idx_file_instances_name     ON file_instances(file_name COLLATE NOCASE);
            CREATE INDEX idx_file_instances_ext      ON file_instances(extension);
            
            -- Partial index for only live files
            CREATE INDEX idx_file_instances_alive    ON file_instances(source_id, deleted_at) WHERE deleted_at IS NULL;
            
            -- Standard autonomous FTS5 table
            CREATE VIRTUAL TABLE file_search USING fts5(
                file_name,
                volume_relative_path,
                content='file_instances',
                content_rowid='rowid'
            );
            
            -- Triggers to keep FTS5 in sync with file_instances automatically
            CREATE TRIGGER fts_sync_insert AFTER INSERT ON file_instances BEGIN
              INSERT INTO file_search(rowid, file_name, volume_relative_path)
              VALUES (new.rowid, new.file_name, new.volume_relative_path);
            END;

            CREATE TRIGGER fts_sync_delete AFTER DELETE ON file_instances BEGIN
              DELETE FROM file_search WHERE rowid = old.rowid;
            END;

            CREATE TRIGGER fts_sync_update AFTER UPDATE ON file_instances BEGIN
              INSERT INTO file_search(file_search, rowid, file_name, volume_relative_path)
              VALUES ('delete', old.rowid, old.file_name, old.volume_relative_path);
              INSERT INTO file_search(rowid, file_name, volume_relative_path)
              VALUES (new.rowid, new.file_name, new.volume_relative_path);
            END;
            
            -- Populate search table for existing records
            INSERT INTO file_search (rowid, file_name, volume_relative_path)
                SELECT rowid, file_name, volume_relative_path FROM file_instances;
        "),
        M::up("
            -- M006: thumbnail tracking
            ALTER TABLE file_instances ADD COLUMN thumbnail_at TEXT;
        "),
        M::up("
            -- M007: Epic 6 exact duplicates schema
            ALTER TABLE file_instances ADD COLUMN preferred_copy INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE file_instances ADD COLUMN duplicate_note TEXT;
        "),
        M::up("
            -- M008: Composite indexes for duplicate detection queries
            -- Confirmed: covers GROUP BY blake3_hash for hashed, live files
            CREATE INDEX IF NOT EXISTS idx_dupes_confirmed
                ON file_instances(blake3_hash, file_name)
                WHERE deleted_at IS NULL
                  AND blake3_hash IS NOT NULL
                  AND blake3_hash != '';

            -- Probable: covers GROUP BY (file_name, size_bytes) for unhashed, live files
            CREATE INDEX IF NOT EXISTS idx_dupes_probable
                ON file_instances(file_name, size_bytes, id)
                WHERE deleted_at IS NULL
                  AND (blake3_hash IS NULL OR blake3_hash = '');
        "),
        M::up("
            -- M009: Folder exclusions and intentional backup flag
            CREATE TABLE excluded_paths (
                id                   TEXT PRIMARY KEY,
                source_id            TEXT REFERENCES storage_sources(id),
                volume_path_prefix   TEXT NOT NULL,
                label                TEXT,
                created_at           TEXT NOT NULL
            );
            CREATE INDEX idx_excluded_paths_source ON excluded_paths(source_id);

            ALTER TABLE file_instances ADD COLUMN is_intentional_backup INTEGER NOT NULL DEFAULT 0;
        "),
        M::up("
            -- M010: Extend excluded_paths with pattern_type for name/extension filtering
            -- pattern_type: 'folder'     -> match volume_path_prefix against volume_relative_path
            --               'file_name'  -> match volume_path_prefix against file_name (exact)
            --               'extension'  -> match volume_path_prefix against file extension (LIKE)
            ALTER TABLE excluded_paths ADD COLUMN pattern_type TEXT NOT NULL DEFAULT 'folder';
        "),
    ]);

    migrations.to_latest(&mut conn)
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(conn)
}
