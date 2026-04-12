# Epics

## Epic 1: Foundation and App Shell
### Goal
Create the Rust + Tauri + React/TypeScript application skeleton, persistence foundation, and local development workflow.

### Completion criteria
- [x] app launches reliably
- [x] command/event bridge works
- [x] SQLite database initializes and migrates
- [x] logging and error surfaces exist

---

## Epic 2: Storage Source Registration
### Goal
Register mounted local storage sources and preserve identity across restarts and remounts.

### Completion criteria
- [x] user can add/remove storage sources
- [x] sources show mount status
- [x] removable sources are recognized as offline/online
- [x] per-source quarantine path is defined

---

## Epic 3: Staged Scan and Catalog Creation
### Goal
Build fast inventory-first scanning with background enrichment.

### Completion criteria
- [x] stage 1 inventory produces visible library records quickly
- [x] progress is visible by source (live file count, bytes, animated % bar)
- [x] rescans detect changed and removed files (UPSERT change detection, Mark and Sweep soft-deletion)
- [x] source disappearance does not destroy catalog knowledge (soft-delete field `deleted_at` ensures preservation)

### Optional extension (post-MVP)
- Fast Scan via NTFS MFT read (requires UAC elevation; ~10–20× faster on mechanical HDDs). See Story 3.5 in epic-03-stories.md.

---

## Epic 4: Unified Library and Search
### Goal
Provide one browse/search surface across all registered sources.

### Completion criteria
- [x] search works across cataloged items
- [x] filters and sorts are usable
- [x] offline items remain visible with clear status

---

## Epic 5: Previews and Item Detail
### Goal
Deliver rich enough detail and preview to avoid constant context switching.

### Completion criteria
- [x] supported file types show preview or metadata summary
- [x] item detail page explains source, state, and relationships
- [x] preview failures degrade gracefully

---

## Epic 6: Exact Duplicates and Recommendation Logic
### Goal
Detect exact duplicates and recommend likely best copies with explanation.

### Completion criteria
- [x] exact duplicate grouping works
- [x] duplicate review view exists
- [x] recommendation shows why and what differs

---

## Epic 7: Maps and Relationship Views
### Goal
Add focused relationship and storage maps.

### Completion criteria
- item/group relationship map exists
- storage map shows where copies live
- map remains readable at scoped sizes

---

## Epic 8: Organization Actions
### Goal
Support virtual organization and guided real actions.

### Completion criteria
- collections/tags/grouping work
- guided move/merge actions work
- actions are auditable and confirmed

---

## Epic 9: Protection Rules and Safety Layer
### Goal
Surface rule-based protection awareness and safe action flows.

### Completion criteria
- protection rules can be edited
- item states update from rules
- only-copy and unprotected warnings are visible
- quarantine and restore work

---

## Epic 10: Polish, Quality, and MVP Hardening
### Goal
Make the product trustworthy enough for real archives.

### Completion criteria
- fixture-based tests cover core flows
- performance is acceptable on target datasets
- upgrade/restart/rescan flows are stable
- MVP release checklist passes
