# MVP Scope

## MVP thesis
The first version of **The Last Place You Look** should make mounted local archives understandable and manageable without asking users to trust destructive automation.

## MVP promise
A user can connect mounted local drives, build a unified catalog, identify duplicates and copy relationships, understand protection state through explicit rules, and reorganize safely using guided moves, merges, and per-drive quarantine.

## Included in MVP
- Windows-first desktop app
- Rust core with Tauri + React/TypeScript UI
- support for mounted local storage sources, optimized for removable external drives
- staged scan pipeline
- unified library view
- home dashboard
- exact duplicate detection
- asset/file instance/relationship model
- likely best copy recommendation with explanation
- storage map for where copies live
- relationship map for item or duplicate group
- virtual collections / tags / grouping
- guided move/merge actions
- per-drive quarantine
- rule-based protection awareness
- adaptive UI with progressive disclosure

## Explicitly included even though ambitious
These are part of MVP because they are central to the product value, not nice-to-haves:
- graph/map style views at focused scope
- backup/protection awareness based on rules
- organization actions, not just browsing

## Excluded from MVP
- hard delete
- cloud backup integration
- network/NAS as first-class storage sources
- giant all-files graph view
- semantic search / LLM features
- auto-resolving duplicates without user confirmation
- mobile companion apps

## Safety model
The app must not permanently delete files in MVP.
Any “removal” action sends files to **per-drive quarantine**.

## Key user outcomes
A user should be able to:
1. register drives and begin scanning quickly
2. search and browse one combined library
3. inspect a duplicate group and see a recommended best copy
4. understand why a copy is recommended
5. move or merge selected files safely
6. understand protection state from explicit rules
7. leave and return later without losing context

## Acceptance criteria
### Cataloging
- user can register multiple mounted local storage sources
- first scan produces visible results before deep enrichment finishes
- scanned files persist across restarts

### Duplicate workflow
- app can detect exact duplicates across storage sources
- duplicate group view shows all members
- app can rank likely best copy using visible criteria
- explanation includes why, what differs, and what is uncertain

### Protection workflow
- user can define protection rules
- items display a rule-derived protection state
- app never labels a file “backed up” without matching a visible rule

### Organization workflow
- user can create collections or apply grouping metadata
- user can perform guided move or merge actions
- user can quarantine instead of delete

### UI
- dashboard + library are both present
- advanced detail is progressively disclosed
- item/group map shows copies and storage locations

## Open questions kept for post-MVP refinement
- scope of project clustering in MVP
- breadth of file preview support in first release
- depth of media-specific metadata extraction
