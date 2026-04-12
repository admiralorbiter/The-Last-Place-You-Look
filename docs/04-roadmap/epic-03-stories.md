# Epic 3: Staged Scan and Catalog Creation — Story Breakdown

Epic goal: Build fast inventory-first scanning with background enrichment.

**Depends on:** Epic 2 — Requires registered sources and stable volume bindings to save files against.

---

## Story 3.1: File Instance Schema & Migrations (M003, M004) ✅ DONE

### What
Establish the core `file_instances` and `scan_jobs` tables that form the backbone of the catalog.

### Implemented schema (actual)
```sql
CREATE TABLE file_instances (
    id                    TEXT PRIMARY KEY,          -- UUID v4
    asset_id              TEXT,                      -- NULL until Stage 6 assigns asset grouping
    source_id             TEXT NOT NULL REFERENCES storage_sources(id),
    stable_location_id    TEXT NOT NULL,             -- preserved on move inference (ADR 0009)
    volume_relative_path  TEXT NOT NULL,             -- drive-letter-stable path
    current_path          TEXT,
    file_name             TEXT NOT NULL,
    extension             TEXT,
    size_bytes            INTEGER NOT NULL,
    modified_at           TEXT NOT NULL,             -- ISO 8601
    created_at_fs         TEXT,
    stage_1_at            TEXT,
    stage_2_at            TEXT,
    stage_3_at            TEXT,
    blake3_hash           TEXT,
    deleted_at            TEXT,
    quarantine_status     TEXT NOT NULL DEFAULT 'none',
    UNIQUE(source_id, volume_relative_path)
);

CREATE TABLE scan_jobs (
    id             TEXT PRIMARY KEY,
    source_id      TEXT NOT NULL REFERENCES storage_sources(id),
    started_at     TEXT NOT NULL,
    completed_at   TEXT,
    status         TEXT NOT NULL DEFAULT 'running',
    stage          INTEGER NOT NULL DEFAULT 1,
    files_found    INTEGER NOT NULL DEFAULT 0,
    files_inserted INTEGER NOT NULL DEFAULT 0,
    error_message  TEXT
);
```

### Done when
- [x] M003 and M004 migrations defined in `persistence/db.rs`
- [x] Database migrates safely on startup
- [x] SQLite performance pragmas applied (WAL, synchronous=NORMAL, cache=64MB, temp_store=MEMORY)

---

## Story 3.2: The Pipeline Orchestrator & PipelineManager State ✅ DONE

### What
An async Tauri-managed state object that tracks active scan jobs and bridges the Tokio runtime to the jwalk worker thread.

### Implemented
- `services/pipeline.rs` — contains `PipelineManager` (managed state), `start_scan`, `stage_1_worker`
- `commands/pipeline.rs` — exposes `start_scan`, `get_scan_status`, `cancel_scan` to frontend
- `PipelineManager` registered in `lib.rs` via `.manage()`
- A `sync_channel(50_000)` decouples the walker thread from the SQLite writer thread (producer/consumer)

### Done when
- [x] Frontend can command backend to start scanning a specific source
- [x] Scan runs fully in background without blocking the UI
- [x] Progress emitted via `pipeline://progress` events

---

## Story 3.3: Stage 1 Traversal (Inventory) ✅ DONE

### What
High-performance filesystem walk that populates `file_instances` from a registered source.

### Implemented
- `jwalk` for directory traversal; mode selected by `source_kind`:
  - **Removable/external drives**: `Serial` parallelism (avoids HDD head thrashing)
  - **Internal drives**: `RayonDefaultPool` (multi-threaded, SSD-optimized)
- System directories skipped automatically: `.tlpyl-quarantine`, `System Volume Information`, `$RECYCLE.BIN`
- Records sent from walker thread → DB writer thread via `mpsc::sync_channel`
- DB writer uses chunked multi-row `INSERT OR IGNORE` (85 rows × 11 columns = 935 params, under SQLite limit)
- Batch size: 10,000 records per transaction
- Progress events emitted every 10,000 files; DB `scan_jobs` updated every 25,000 files

### Performance (observed, H:\ — 6.54 TB USB external HDD)
- Serial mode: ~150–300k files/minute sustained
- Full drive (~2M files): approximately 7–15 minutes
- SQLite is **not** the bottleneck — mechanical disk I/O is the ceiling in standard OS traversal

### Done when
- [x] Scanning a large drive populates `file_instances` without freezing UI
- [x] `INSERT OR IGNORE` ensures idempotency — rescans do not clobber Stage 3 hashes
- [x] `volume_relative_path` stored (not absolute), stable across drive letter changes

---

## Story 3.4: Scan UI & Real-Time Progress ✅ DONE

### What
Surface the pipeline state in the React frontend with live progress.

### Implemented
- `stores/pipelineStore.ts` — Zustand store subscribing to `pipeline://progress` events
- `pages/Sources.tsx` — redesigned source cards with:
  - "Start Full Scan" / "Rescan Index" button (per online source)
  - Live file count + bytes transferred
  - Animated gradient progress bar derived from `bytes_found / total_used_bytes` (disk-space-based %)
  - `GetDiskFreeSpaceExW` Win32 call to fetch total used bytes for percentage calculation
  - Completion state shows total files cataloged and total size

### Done when
- [x] Clicking Scan triggers backend correctly
- [x] Progress bar increments smoothly without UI freeze
- [x] Percentage estimate is grounded in real disk usage data

---

## Story 3.5 (Extension — Post-MVP): Fast Scan via NTFS MFT

### What
An optional "Fast Scan" mode that reads the NTFS Master File Table (MFT) directly, bypassing OS filesystem traversal entirely. This is how WizTree and Everything achieve near-instant full-drive indexing.

### Why this matters
Standard OS traversal (`ReadDirectoryChanges`, `FindFirstFile`) on a 6.54 TB mechanical HDD takes 7–15 minutes for 2M files. MFT-direct reading completes the same job in under 60 seconds on the same hardware.

### Technical approach
- The NTFS MFT lives at a fixed location on every NTFS volume (`$MFT`)
- Reading it requires opening the volume handle with `CreateFileW(\\.\H:, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, ...)` as an Administrator
- The MFT is then read in 1 MB chunks; each entry (1 KB) is parsed to extract: file reference number, parent reference, file name, size, timestamps, flags
- A second pass reconstructs the full path tree from the parent reference chain
- This produces the complete file listing in seconds with zero directory I/O

### Design constraints
- **Requires elevation**: Must open volume with `SeManageVolumePrivilege`. Implemented as a one-time UAC prompt before the scan begins. The standard scan mode remains the default.
- **NTFS only**: Falls back to standard traversal for FAT32, exFAT, ReFS volumes automatically
- **Windows only**: No Linux/macOS MFT equivalent — this is already Windows-only code
- **Read-only**: Only reads MFT entries, never writes to the volume

### UI
- "⚡ Fast Scan (requires Administrator)" button alongside the standard scan button
- UAC prompt fires before the scan begins
- Clear note that the fast scan reads NTFS volume records directly

### Done when (if scheduled)
- [ ] `detect_volume_filesystem(mount_path) -> FilesystemType` helper returns `Ntfs | Fat32 | ExFat | Refs`
- [ ] Fast scan button visible only on NTFS sources
- [ ] UAC elevation prompt fires before the scan starts
- [ ] MFT reader parses file reference, parent reference, file name, size, timestamps
- [ ] Path reconstruction from parent reference chain produces correct `volume_relative_path`
- [ ] Results feed into the same `file_instances` insert pipeline as Stage 1
- [ ] Fallback to standard traversal if elevation is denied

---

## Story 3.6: Rescan Synchronization (Change & Removal Detection) ✅ DONE

### What
Ensure that subsequent scans of an already-cataloged source accurately capture changes (edits) and removals (deletions), without falsely claiming a removed disk was completely deleted.

### Implemented
- **Change Detection (Modified files):** Uses SQLite `UPSERT` (`ON CONFLICT DO UPDATE`). When a file matches an existing `source_id` + `volume_relative_path`, it compares the incoming `size_bytes` and `modified_at`. If either changed, it updates the record and sets `blake3_hash`, `stage_2_at`, and `stage_3_at` to `NULL`, flagging it for re-hashing.
- **Deletion Detection (Removed files):** Uses the "Mark and Sweep" algorithm. In the final seconds of a successful scan, one single `UPDATE` query soft-deletes (`deleted_at = NOW()`) every file on the source whose `stage_1_at` timestamp is older than the scan's start time.
- **Disappearance Safety:** If a source is temporarily disconnected, it cannot be scanned, meaning a successful scan never runs on it while offline, preventing accidental mass false-deletions.

### Done when
- [x] Edited files are updated and stripped of stale hashes
- [x] Missing files are soft-deleted from the catalog without DB iteration bottlenecks
- [x] Reappearing files are seamlessly restored (`deleted_at = NULL`)

---

## Epic 3 completion criteria (from epics.md)

- [x] Stage 1 inventory produces visible library records quickly
- [x] Progress is visible by source
- [x] Rescans update existing catalog state (UPSERT + Mark and Sweep implemented)
- [x] Source disappearance does not destroy catalog knowledge (schema and offline protections in place)
