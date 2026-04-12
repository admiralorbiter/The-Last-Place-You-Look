# Milestones

## Phase model

The 10 epics are grouped into 5 phases. Each phase ends with the app meeting a meaningful threshold — something that can be tested end to end before the next phase begins.

No calendar dates are tracked here. Phases are ordered by dependency and value delivery.

---

## Phase 0: Walking Skeleton

**Epics: 1 (Foundation and App Shell), 2 (Storage Source Registration)**

### What this phase delivers
The app launches. A user can register mounted local drives. The command/event bridge, database, logging, and dev workflow all function correctly.

### Done when
- [x] Tauri v2 + React/TypeScript app launches on Windows
- [x] SQLite initializes and migrates via rusqlite_migration
- [x] At least one Tauri command round-trips from frontend to Rust backend and back
- [x] Structured error responses cross the command bridge
- [x] A user can add and remove storage sources
- [x] Registered sources show correct mount status on startup
- [x] Volume GUID is used as stable source identity
- [x] Per-source quarantine path is defined and stored
- [x] Offline/online source state is visible in UI

### Dependency note
All subsequent phases depend on Phase 0. Do not begin Phase 1 until source registration and the data model are stable. The schema established here must support the file instance model from Epic 3.

---

## Phase 1: Core Catalog

**Epics: 3 (Staged Scan and Catalog Creation), 4 (Unified Library and Search)**

### What this phase delivers
Scanning works. A user can start a scan on registered sources and browse the resulting catalog in a unified library.

### Done when
- Stage 1 inventory produces visible library records before hash enrichment finishes
- Scan progress is visible per source and per stage
- Library search works across cataloged items
- Filter by type, source, and basic attributes works
- Offline items remain in library with clear status
- Rescan detects new, changed, and removed files without destroying existing catalog state
- Temporarily offline sources do not trigger catalog data deletion

### Dependency note
Epics 3 and 4 can be developed in parallel once the file instance schema (from Epic 3) is stable enough to query from the library view.

---

## Phase 2: Core Intelligence

**Epics: 5 (Previews and Item Detail), 6 (Exact Duplicates and Recommendation Logic)**

### What this phase delivers
The core product value is complete. A user can identify duplicates, see an explained recommendation, and view item details and previews.

### Done when
- Supported file types show thumbnails and a basic preview panel (images, video thumbnails, PDF first page)
- Preview failures degrade gracefully — no crashes or blank screens
- Item detail page shows source, state, and relationships
- Exact duplicate grouping works across storage sources
- Duplicate group review view exists
- Likely best copy recommendation is visible with a ranked explanation
- Fixed internal ranking signals are implemented (not user-tunable in MVP)
- User can pin a preferred copy; pin persists across rescans
- Intentional backup can be marked and is distinct from accidental duplicate in UI

### Dependency note
Epic 6 requires hashing from Epic 3. Epic 5 requires file instance records from Epic 3. Both can begin once Phase 1 (Epic 3) is stable.

---

## Phase 3: Full Feature Surface

**Epics: 7 (Maps and Relationship Views), 8 (Organization Actions), 9 (Protection Rules and Safety Layer)**

### What this phase delivers
The full MVP feature set is delivered. Users can visualize relationships, organize files with guided actions, and understand protection state through explicit rules.

### Done when
- Item/group relationship map exists and is readable at scoped sizes
- Storage map shows where copies live across sources
- Collections, tags, and grouping work
- Guided move and merge actions work with required confirmation
- Actions are auditable — the user knows what happened
- Protection rules can be edited by the user
- Protection states update when rules or source availability changes
- Only-copy and unprotected warnings are prominent
- Quarantine is per-drive and reversible via restore

### Dependency note
Epics 7 and 8 can run in parallel. Epic 9 depends on source registration (Phase 0) and organization actions (Epic 8). This is the widest phase in terms of feature surface.

---

## Phase 4: MVP Hardening

**Epic: 10 (Polish, Quality, and MVP Hardening)**

### What this phase delivers
The product is trustworthy enough for real archives. Tests cover core flows. Performance is acceptable on target dataset sizes. Upgrade, restart, and rescan flows are stable and verified.

### Done when
- All items in `05-quality/release-checklist.md` pass
- Fixture-based tests cover core duplicate, scan, protection, and quarantine flows
- Performance is acceptable at the target dataset size (see architecture-overview.md)
- All Phase 0–3 acceptance criteria are formally re-verified

---

## Epic dependency map

```
1 → 2 → 3 ──→ 4
         └──→ 5
         └──→ 6 ──→ 7
                    8 → 9 → 10
```

Epic 1 must precede everything. Epics within the same phase can overlap once their shared interfaces (schema, commands) are stable.
