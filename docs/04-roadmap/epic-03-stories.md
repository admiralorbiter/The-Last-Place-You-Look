# Epic 3: Staged Scan and Catalog Creation — Story Breakdown

Epic goal: Build fast inventory-first scanning with background enrichment.

**Depends on:** Epic 2 — Requires registered sources and stable volume bindings to save files against.

---

## Story 3.1: File Instance Schema & M003 Migration

### What
Establish the core `file_instances` table that will securely hold every file discovered across all volumes.

### Schema
```sql
CREATE TABLE file_instances (
    id                    TEXT PRIMARY KEY,
    source_id             TEXT NOT NULL REFERENCES storage_sources(id),
    volume_relative_path  TEXT NOT NULL,
    size_bytes            INTEGER NOT NULL,
    modified_time         TEXT NOT NULL,  -- ISO 8601
    
    stage_1_completed     INTEGER NOT NULL DEFAULT 0,
    stage_2_completed     INTEGER NOT NULL DEFAULT 0,
    
    blake3_hash           TEXT,           -- Generated in Stage 2
    
    deleted_at            TEXT,           -- Soft delete if file goes missing
    
    UNIQUE(source_id, volume_relative_path)
);
```

### Done when
- M003 migration is defined in `persistence/db.rs`
- Database migrates safely on startup

---

## Story 3.2: The Pipeline Orchestrator & State

### What
Create the Rust backend thread that manages background scanning operations. Tauri commands should trigger scans asynchronously without blocking the UI.

### Tasks
- Create `services/pipeline.rs` 
- Set up a Tokio async worker thread that reads from an MPSC channel (for incoming scan requests)
- Maintain an `ActiveScan` state object in a Rust Mutex so the frontend can query progress
- Expose `start_scan(source_id)` and `get_scan_status()` Tauri commands

### Done when
- Frontend can command backend to start scanning a specific source
- A backend thread wakes up, executes dummy work, and updates a state map the frontend can poll

---

## Story 3.3: Stage 1 Traversal (Inventory)

### What
Implement the lightning-fast filesystem walk over a source using `jwalk`.

### Tasks
- Introduce `jwalk` and `crossbeam-channel` into `Cargo.toml`.
- When `start_scan` fires for a source, resolve the Volume GUID into the `current_mount_path`.
- Recursively walk the source mount path, skipping system directories and the `.tlpyl-quarantine` folder.
- Harvest relative paths, sizes, and modified times.
- Yield items instantly to an SQLite write-thread using channels to avoid database lock contention.

### Done when
- Scanning a large drive inserts thousands of records per second without freezing the UI.
- The `file_instances` table is populated correctly.

---

## Story 3.4: Scan UI & Progress

### What
Surface the pipeline state in the React frontend.

### Tasks
- Add a "Scan" button onto each Source card in the `Sources.tsx` view.
- Introduce a global `pipelineStore.ts` tracking active jobs.
- Show a simple progress spinner or indicator when a drive is actively scanning.

### Done when
- Clicking Scan triggers the backend correctly.
- Background scanning is visually apparent attached to the correct source card.

---

## Epic 3 completion criteria (from epics.md)
- [ ] stage 1 inventory produces visible library records quickly
- [ ] progress is visible by source
- [ ] rescans update existing catalog state
- [ ] source disappearance does not destroy catalog knowledge
