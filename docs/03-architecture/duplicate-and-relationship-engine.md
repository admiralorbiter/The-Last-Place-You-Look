# Duplicate and Relationship Engine

## Purpose
This subsystem turns raw files into understandable relationships.

## MVP scope
MVP focuses on:
- exact duplicate detection
- likely best copy recommendation
- intentional backup distinction
- focused relationship maps

## Exact duplicate definition
Two file instances are **confirmed exact duplicates** when they share an identical BLAKE3 content hash. This is the ground-truth definition.

Because bulk hashing is not the default (hashing is triggered on demand when a file is viewed), the engine also maintains a **probable duplicate** tier for files that share the same `file_name` and `size_bytes`. Probable matches are a reliable heuristic and are immediately available from Stage 1 scan data without any disk reads.

## Two-tier detection model

| Tier | Signal | How acquired | Confidence |
|---|---|---|---|
| **Confirmed** | Identical BLAKE3 hash | Hash computed on demand when file is opened in detail panel (auto) or via Verify button (manual) | Exact |
| **Probable** | Same `file_name` + `size_bytes` (≥ 512 KB) | Available immediately after Stage 1 scan, no hashing required | Very high; false positives rare above 512 KB |

The 512 KB floor on probable detection filters out common system-file noise (icons, thumbnails, configs) that are numerous but never meaningful duplicates worth reviewing.

Probable groups can be promoted to Confirmed at any time via the **Verify Hash** action, which hashes all members on demand and updates the database.

## Not exact duplicates
These should not be treated as exact duplicates:
- transcodes
- exports
- resized images
- same name different contents
- same project but different versions

## Recommendation model
The app should recommend a likely best copy, but never auto-resolve without user confirmation.

The recommendation UI must show:
- likely best copy
- why it was chosen
- what differs among members
- what action is available
- what the system is not sure about

## Initial ranking signals
Candidate signals for likely best copy:
- not quarantined
- lives in a preferred or protected source
- richer metadata completeness
- newer meaningful timestamp when relevant
- more user markings / collection membership
- better location semantics (for example not in temp/export/trash-like folders)

## Intentional backup handling
A user should be able to mark copies or folders as intentional mirrors/backups.
That state must affect:
- recommendation language
- cleanup suggestions
- protection evaluation

## Relationship maps in MVP
### Duplicate group map
Shows:
- selected group
- its file instances
- source locations
- preferred copy if any
- intentional backup status

### Item relationship map
Shows:
- asset/file instance
- duplicates
- derivative edges where known
- storage location links

### Project cluster view
Optional in MVP if schedule allows. Should remain focused, not global.

## Acceptance criteria
- [x] Probable duplicate groups (name + size ≥ 512 KB) are immediately visible after Stage 1 scan
- [x] Confirmed duplicate groups (BLAKE3 hash) are visible for any files that have been hashed on demand
- [x] Verify Hash action promotes a probable group to confirmed or dismisses it if files differ
- [x] Recommendation is computed per group from fixed ranking signals (source type, path depth, path length)
- [x] User can pin a preferred copy; pin persists as `preferred_copy = 1` on `file_instances` and survives rescans
- [ ] Intentional backups can be distinguished from accidental duplicates
- [ ] Group review supports user confirmation before destructive action
- [ ] Relationship map is readable and scoped

## Deferred work
- near-duplicate images
- semantic similarity
- automated project reconstruction
- transcoding lineage inference beyond basic rules
