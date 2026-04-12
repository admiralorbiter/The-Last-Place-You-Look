# Architecture Overview

## Chosen stack
- **Core engine:** Rust
- **Desktop shell:** Tauri
- **Frontend:** React + TypeScript
- **Database:** SQLite via rusqlite
- **Async runtime:** Tokio
- **File watching:** notify
- **Search:** SQLite FTS5 initially
- **Media probing/preview helpers:** FFmpeg / ffprobe where needed

## Architectural style
Use a **hybrid architecture**:
- direct service methods for user-initiated commands
- background job queue + internal event pipeline for scanning, indexing, preview generation, and update propagation

## Major subsystems

### Frontend shell
Responsible for:
- rendering UI
- local view state
- command dispatch to backend
- subscribing to progress/state events

### Application services
Responsible for:
- command handling
- validation
- transaction boundaries
- orchestration of domain logic

### Domain layer
Responsible for:
- asset/file instance/relationship rules
- duplicate ranking
- protection evaluation
- quarantine semantics

### Persistence layer
Responsible for:
- SQLite schema
- queries and indexes
- migrations
- FTS tables

### Scan/index pipeline
Responsible for:
- source enumeration
- staged inventory
- hashing
- metadata extraction
- relationship updates

### Preview pipeline
Responsible for:
- preview scheduling
- thumbnail extraction
- cache bookkeeping

### Watcher/update pipeline
Responsible for:
- detecting mounted-source changes
- path additions/removals/modifications
- reconciling file state after scan

## Command vs event boundary
Use direct commands for:
- register source
- create collection
- apply tag
- move/merge action
- quarantine action
- protection rule CRUD

Use events/jobs for:
- scan started/updated/completed
- hash completed
- preview completed
- duplicate group updated
- source mounted/unmounted
- protection state recomputed

## Source of truth
SQLite is the source of truth for catalog state.
The filesystem remains the source of truth for actual file bytes and paths.
The app should reconcile the two rather than pretending the catalog replaces the filesystem.

## Windows-first assumptions
MVP is Windows-first. Storage source identity should not rely purely on drive letters. Prefer stable volume identity and mount resolution so removable drives can be recognized across remounts.

## Key architectural constraints
- large scans must not block UI
- blocking I/O and hashing must not starve async work
- safety-sensitive actions need clear auditability
- state must persist across restarts and temporary source disappearance

## Performance targets

These are the minimum acceptable thresholds for MVP. Establish baseline measurements before Epic 10 hardening.

| Metric | Target |
|---|---|
| Target catalog size (comfortable) | 50,000 file instances across all sources |
| Target catalog size (stretch / stress test) | 200,000 file instances |
| Time to first visible library results (Stage 1 inventory) | < 5 seconds for a mounted source with 10,000 files |
| Hash throughput | Must not visibly freeze UI; progress updates at least every 2 seconds |
| Search latency (FTS5 query) | < 200ms for a 50,000-item catalog |
| Preview thumbnail generation | Must not block library rendering; thumbnails load progressively |
| App startup to interactive | < 3 seconds on a mid-range Windows machine |
| Memory use during active scan | No unbounded growth; establish baseline and flag regressions |

These targets are not hard deadlines — they are the benchmark against which Epic 10 quality work is measured.
