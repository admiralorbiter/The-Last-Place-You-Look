# Scan and Index Pipeline

## Chosen approach
Use a **staged scan**.

The application should become useful quickly with shallow inventory, then deepen in the background.

## Stages

### Stage 1: Source inventory
- enumerate files from registered mounted storage sources
- capture path, name, extension, size, basic timestamps, source identity
- create or update file instance records
- surface results in library as soon as possible

### Stage 2: Enrichment scheduling
- queue items for deeper work
- prioritize visible, recently changed, or high-interest items first where helpful

### Stage 3: Hashing
- compute content hashes for duplicate detection
- support resumable progress
- avoid blocking the UI thread

### Stage 4: Metadata extraction
- extract media/doc metadata where supported
- record structured metadata for filtering and detail views

### Stage 5: Preview generation
- generate thumbnails/previews as needed
- cache results and track failures

### Stage 6: Relationship and protection evaluation
- build/update duplicate groups
- recompute likely best copy
- evaluate protection rules
- emit updates to frontend

## Rescan behavior
Rescans should:
- detect new files
- detect removed files
- update changed files
- preserve history where path continuity can be inferred
- handle temporarily unavailable sources without deleting catalog knowledge immediately

## Mount/unmount behavior
Because MVP is optimized for removable drives:
- a source may be absent and still remain in the catalog
- missing sources should be clearly marked as offline/unmounted
- offline file instances remain searchable/browsable with limited actions

## Job system requirements
- cancellable or resumable where practical
- progress emitted by source and by stage
- bounded concurrency for hashing and metadata extraction
- separation between fast inventory I/O and heavier enrichment work

## Acceptance criteria
- user sees first library results before full enrichment completes
- scan progress is visible
- scans survive restarts or fail gracefully
- unplugging a source does not destroy known catalog state
- reattaching a known source reconnects to prior source identity where possible
