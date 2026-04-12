# Epic 1: Foundation and App Shell — Story Breakdown

Epic goal: Create the Rust + Tauri v2 + React/TypeScript application skeleton, persistence foundation, and local development workflow.

**Depends on:** nothing — this is the starting point.

---

## Story 1.1: Bootstrap Tauri v2 project

### What
Create the root Tauri v2 project with a React/TypeScript frontend from scratch.

### Tasks
- Run `cargo create-tauri-app` (or `npm create tauri-app@latest`) and select React/TypeScript
- Confirm Tauri v2 is selected — verify `@tauri-apps/api` v2 in `package.json` and Tauri v2 in `Cargo.toml`
- Verify the app launches on Windows with `npm run tauri dev`
- Commit the initial scaffold as a clean baseline

### Done when
- `npm run tauri dev` launches the app window without errors
- The default React frontend renders
- The Rust backend compiles cleanly

---

## Story 1.2: Establish Rust module structure

### What
Organize `src-tauri/src/` into a module layout that matches the architectural subsystems. Do not implement anything yet — just establish the boundaries.

### Suggested layout
```
src-tauri/src/
  commands/        # Tauri command handlers (thin layer, delegates to services)
  domain/          # Core domain rules: ranking, protection evaluation, relationship rules
  persistence/     # SQLite access, queries, migrations
  services/        # Orchestration: scan pipeline, quarantine, organization actions
  errors.rs        # AppError type
  lib.rs
  main.rs
```

### Done when
- All modules compile with empty `mod.rs` or stub files
- Each module has a one-line comment stating its ownership boundary

---

## Story 1.3: SQLite initialization with rusqlite_migration

### What
Initialize the SQLite database with `rusqlite_migration` on app startup. Establish the migration baseline.

### Tasks
- Add `rusqlite` and `rusqlite_migration` to `Cargo.toml`
- Create `persistence/db.rs` with an `init_db(app_data_dir: &Path) -> Result<Connection, AppError>` function
- Define `M001` as the initial empty migration baseline (subsequent epics add to it)
- Call `init_db` from the Tauri app setup in `lib.rs`
- Verify the `.db` file is created in the Tauri app data directory, not a hard-coded path

### Done when
- App startup creates or reopens the SQLite database without error
- The migration baseline applies cleanly on first run and is a no-op on subsequent runs
- DB file lives under the platform app data directory (Windows: `%APPDATA%\{app-name}\`)

---

## Story 1.4: AppError type and command error handling

### What
Define a structured `AppError` type that all Tauri commands return and that can be serialized to the frontend.

### Tasks
- Define `AppError` enum in `errors.rs` with variants covering common failure categories:
  - `DatabaseError(String)`
  - `IoError(String)`
  - `NotFound(String)`
  - `InvalidInput(String)`
  - `PlatformError(String)` (for Windows API failures)
- Implement `serde::Serialize` on `AppError` so it crosses the Tauri command bridge as structured JSON
- All command handlers must return `Result<T, AppError>` — never a raw string error

### Done when
- A command handler that intentionally fails returns a structured JSON error object to the frontend
- The frontend can read the error variant and message, not just a generic error string

---

## Story 1.5: Command/event bridge skeleton

### What
Prove the Tauri command and event patterns work end to end.

### Tasks
- Implement a `get_app_info` command that returns a struct `{ version: String, db_status: String }`
- Implement a backend-emitted event `app://ready` sent after startup initialization completes
- Frontend calls `get_app_info` via `invoke` and renders the response somewhere visible
- Frontend subscribes to `app://ready` via Tauri's event listener

### Done when
- Frontend successfully calls `get_app_info` and displays structured data from Rust
- Frontend receives `app://ready` after startup without error
- A command error returns a structured `AppError` JSON object, not a panic

---

## Story 1.6: Logging baseline

### What
Set up structured logging visible during development.

### Tasks
- Add `tracing` and `tracing-subscriber` to `Cargo.toml`
- Initialize subscriber in `main.rs` respecting `RUST_LOG` env var
- Log at `info` level for key startup events: DB init, Tauri setup complete, app ready
- Log at `debug` level for command invocations (command name + inputs summary, not full payloads)

### Done when
- Running `RUST_LOG=debug npm run tauri dev` shows log output in the terminal with level and module path
- Running without `RUST_LOG` set shows only `warn` and above (no debug noise in default mode)

---

## Story 1.7: Zustand store skeleton (frontend)

### What
Install Zustand and establish the store pattern. Wire one Tauri event into a store slice.

### Tasks
- Install `zustand` via npm
- Create `src/stores/appStore.ts` with a minimal slice, e.g.:
  ```ts
  interface AppStore {
    appReady: boolean;
    setAppReady: (ready: boolean) => void;
  }
  ```
- Subscribe to `app://ready` in the app root and call `setAppReady(true)`
- Render `appReady` state visibly somewhere (e.g., a status badge in the dev layout)

### Done when
- A Zustand store exists with at least one working slice
- A Tauri event updates the store
- The updated value renders in the UI

---

## Story 1.8: Dev workflow documentation

### What
Document how to run, build, and debug the app locally for your own future reference and AI collaboration sessions.

### Tasks
- Create or update `README.md` at the project root with:
  - Prerequisites: Rust toolchain, Node.js version, Tauri CLI (`cargo install tauri-cli`)
  - Dev command: `npm run tauri dev`
  - How to set `RUST_LOG` for debug output
  - Where the SQLite database file lives on disk
  - How to wipe the database for a clean test (delete the file)

### Done when
- README describes the complete setup from a fresh machine without implied knowledge

---

## Epic 1 completion criteria (from epics.md)
- [ ] app launches reliably
- [ ] command/event bridge works
- [ ] SQLite database initializes and migrates
- [ ] logging and error surfaces exist
