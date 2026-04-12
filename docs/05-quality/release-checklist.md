# MVP Release Checklist

Run this checklist before any MVP release candidate. All items must pass or have a documented exception.

---

## Database and persistence

- [ ] All migrations apply cleanly on a **fresh database** (delete `.db` file and restart)
- [ ] All migrations apply cleanly on a **pre-existing database** from a prior build (simulate upgrade)
- [ ] App restarts without data loss — all catalog state survives a clean shutdown/restart cycle
- [ ] SQLite database file is in the correct Tauri app data directory, not a hard-coded path

---

## Storage sources

- [ ] A removable drive is recognized by Volume GUID after being unplugged and reattached (drive letter may differ)
- [ ] An offline source remains visible in the source list with an "Offline" indicator
- [ ] Reattaching a known source restores online status without creating a duplicate registration
- [ ] Removing a source (soft delete) preserves file instance catalog data for that source

---

## Scan pipeline

- [ ] Stage 1 inventory produces visible library results before hash enrichment finishes
- [ ] Scan progress is visible per source and updates in real time
- [ ] Rescan detects newly added files
- [ ] Rescan detects removed files
- [ ] Rescan updates changed files (size or modified timestamp changed)
- [ ] Rescan does **not** delete catalog data for files on an offline (temporarily unavailable) source
- [ ] An interrupted scan (force-quit mid-scan) recovers on next startup without corruption or data loss

---

## Duplicate logic

- [ ] Exact duplicate fixture (same content, two sources) produces a correctly grouped duplicate group
- [ ] Same file name, different content → **not** grouped as duplicates
- [ ] Same content, different file name → **correctly** grouped as exact duplicates
- [ ] Intentional backup can be marked via UI and is visually distinct from accidental duplicate
- [ ] Likely best copy recommendation is visible with a ranked explanation for every duplicate group
- [ ] System-suggested vs user-pinned preferred copy is visually distinguishable in the group review
- [ ] A user pin on preferred copy survives a rescan without being cleared

---

## Protection and safety

- [ ] All protection state types render correctly: Only Copy, Multiple Copies, Protected by Rule, Not Protected by Rule, Unknown
- [ ] "Only Copy" warning is prominent and not buried
- [ ] App **never** displays "backed up" or "protected" without a matching user-defined rule
- [ ] Protection state updates correctly when a source goes offline (an item that was "Protected by Rule" may become "Only Copy" if its backup is on the offline source)

---

## Quarantine and reversibility

- [ ] No hard-delete code path exists in MVP — audit all action handlers to confirm
- [ ] Quarantine action sends a file to the correct per-drive quarantine folder (`{source_root}\.tlpyl-quarantine`)
- [ ] Quarantine of an offline source's file surfaces a clear error — does not silently fail or corrupt state
- [ ] Restore from quarantine returns the file to its original path (or prompts if path is gone)
- [ ] All move, merge, and quarantine actions require explicit user confirmation before executing

---

## UI and views

- [ ] Home dashboard loads without errors
- [ ] Library view loads, search works, filters work
- [ ] Item detail page loads for at least: a photo, a video, a document
- [ ] Duplicate Group Review loads for at least: a 2-member group, a 3-member group
- [ ] Sources view loads and shows correct mount status
- [ ] Quarantine view loads and lists quarantined files
- [ ] Settings view loads; protection rules can be added and edited
- [ ] Relationship map renders for a duplicate group without visual breakage
- [ ] Storage map renders and shows which sources a group's copies live on
- [ ] Scan progress is visible in the UI during an active scan
- [ ] Offline items are browsable with a visible offline indicator and limited-action messaging

---

## Manual exploratory pass

- [ ] Files with spaces, unicode characters, or long path names scan without error
- [ ] Files above 1 GB scan and hash without UI freeze
- [ ] A scan interrupted by drive removal (pull the USB drive) completes gracefully or recovers on reattachment
- [ ] After Windows reassigns a drive letter, source is still recognized and scanned correctly
- [ ] The UI distinction between "duplicate" and "intentional backup" is not confusing to a first-time user (walk through the group review flow)
