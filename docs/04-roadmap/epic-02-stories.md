# Epic 2: Storage Source Registration — Story Breakdown

Epic goal: Register mounted local storage sources and preserve identity across restarts and remounts.

**Depends on:** Epic 1 — database, command bridge, AppError, and module structure must be complete.

---

## Story 2.1: Windows Volume GUID resolution

### What
Implement the Rust function that resolves a stable Windows Volume GUID from a given path. This is the foundation of all storage source identity.

### Tasks
- Add the `windows` crate to `Cargo.toml` with required features:
  `windows = { version = "...", features = ["Win32_Storage_FileSystem"] }`
- Implement `resolve_volume_guid(path: &Path) -> Result<String, AppError>` using `GetVolumeNameForVolumeMountPoint`
- Return value format: `\\?\Volume{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}\`
- Handle error cases: path does not exist, volume not mounted, unsupported filesystem type
- Write a unit test that resolves the GUID for `C:\` (always available on Windows dev machine)

### Done when
- Function returns the Volume GUID for any mounted path on Windows
- Function returns a structured `AppError::PlatformError` (not a panic) for invalid paths
- Unit test passes

---

## Story 2.2: StorageSource schema and migration M002

### What
Define the `storage_sources` table as migration M002.

### Schema
```sql
CREATE TABLE storage_sources (
    id                    TEXT PRIMARY KEY,   -- UUID v4
    display_name          TEXT NOT NULL,
    source_kind           TEXT NOT NULL,      -- 'internal' | 'removable'
    stable_volume_identity TEXT NOT NULL UNIQUE,  -- Windows Volume GUID
    current_mount_path    TEXT,               -- NULL if offline
    currently_mounted     INTEGER NOT NULL DEFAULT 0,
    quarantine_root       TEXT,
    created_at            TEXT NOT NULL,      -- ISO 8601
    removed_at            TEXT                -- soft delete; NULL if active
);
```

### Done when
- M002 applies cleanly on top of M001 baseline
- App startup runs both migrations without error on a fresh database
- App startup is a no-op for both migrations on a pre-existing database

---

## Story 2.3: Add storage source command

### What
Implement the `add_storage_source` Tauri command.

### Rust signature
```rust
#[tauri::command]
async fn add_storage_source(
    path: String,
    display_name: String,
    source_kind: String,
) -> Result<StorageSource, AppError>
```

### Behavior
1. Resolve Volume GUID from `path` using Story 2.1
2. Check if a non-removed source with the same GUID already exists — return `AppError::InvalidInput` if so
3. Derive default quarantine root: `{path}\.tlpyl-quarantine`
4. Generate a UUID for `id`
5. Insert into `storage_sources` with `currently_mounted = true`, `removed_at = NULL`
6. Return a `StorageSource` struct matching the DB record

### Done when
- Frontend can register a new source and receive the source record back
- Attempting to register the same drive twice returns a clear error
- Record persists across app restarts

---

## Story 2.4: Remove storage source command (soft delete)

### What
Implement the `remove_storage_source` Tauri command. Removal sets `removed_at` — it does **not** delete catalog data.

### Decision
Catalog knowledge (file instances, relationships) is preserved after source removal. File instances for a removed source remain in the database and are accessible in the library with a "source removed" status. This prevents data loss from accidental source removal.

### Rust signature
```rust
#[tauri::command]
async fn remove_storage_source(source_id: String) -> Result<(), AppError>
```

### Behavior
1. Find the source by `id` — return `AppError::NotFound` if absent or already removed
2. Set `removed_at = now()` on the record
3. Do not delete any `file_instances` or relationship data

### Done when
- Source is soft-deleted
- Source no longer appears in active source list queries
- File instances for the source remain in the database

---

## Story 2.5: List storage sources command

### What
Implement the `list_storage_sources` command that returns all active (non-removed) sources.

### Rust signature
```rust
#[tauri::command]
async fn list_storage_sources() -> Result<Vec<StorageSource>, AppError>
```

### Returns
Only sources where `removed_at IS NULL`, ordered by `created_at`. Each record includes: `id`, `display_name`, `source_kind`, `currently_mounted`, `current_mount_path`, `quarantine_root`.

### Done when
- Frontend receives source list on app startup via a Zustand store slice
- Removed sources do not appear in the list

---

## Story 2.6: Mount status detection on startup

### What
On app startup, check which registered sources are currently mounted and update the database accordingly.

### Behavior
1. Load all active storage sources from the database
2. For each source, attempt to find a currently-mounted volume matching `stable_volume_identity`
3. If found: update `currently_mounted = true` and set `current_mount_path` to the current mount point
4. If not found: update `currently_mounted = false`, set `current_mount_path = NULL`
5. Emit a `sources://status_updated` event to the frontend after reconciliation completes

### Note on implementation
The Volume GUID-to-path resolution requires enumerating mounted volumes. Use `FindFirstVolume` / `FindNextVolume` to enumerate GUIDs, then `GetVolumePathNamesForVolumeName` to find the mount path.

### Done when
- App startup correctly marks all mounted sources as online and all unmounted sources as offline
- Source status updates are reflected in the Zustand store via the emitted event
- Unplugging a drive before restarting correctly shows that source as offline

---

## Story 2.7: Per-source quarantine path validation

### What
When adding a source, validate that the quarantine root path is reachable and writable. Surface a warning if not, but do not block registration.

### Behavior
- After insertion, attempt to create the quarantine directory (or verify it exists and is writable)
- If creation fails or the directory appears unwritable, emit a warning (not an error) in the `add_storage_source` response
- Consider a response shape: `{ source: StorageSource, warnings: Vec<String> }`

### Done when
- Adding a source to a healthy mounted drive shows no warning
- Adding a source where the quarantine path is not writable surfaces a visible warning message in the UI
- Warning does not prevent the source registration from completing

---

## Story 2.8: Sources UI

### What
Build the Sources view in the React/TypeScript frontend.

### UI elements
- Source list: display name, source kind badge (Internal / Removable), mount status indicator (Online / Offline), quarantine root path
- "Add source" button → folder/path picker dialog → display name input → confirm
- "Remove source" action per source → confirmation dialog showing what catalog data will be preserved
- Offline sources styled distinctly (muted, offline indicator) but still visible in the list

### Zustand slice
Add a `sources` slice to the store that:
- Is populated by `list_storage_sources` on startup
- Updates when `sources://status_updated` is received

### Done when
- User can add a source via UI and see it appear immediately in the list
- User can remove a source with a confirmation step
- Mounted and unmounted sources are visually distinguishable
- Zustand store drives the list — no ad-hoc fetching from components

---

## Epic 2 completion criteria (from epics.md)
- [x] user can add/remove storage sources
- [x] sources show mount status
- [x] removable sources are recognized as offline/online
- [x] per-source quarantine path is defined
