# ADR 0009: Hash-based best-effort path continuity for moved/renamed files

## Status
Accepted

## Decision
When a file's path disappears from a source during rescan and a file with an **identical content hash** appears at a new path on the **same source**, the system infers the file was moved or renamed and preserves catalog continuity for that file instance.

Name-only or size-only heuristics are not used for continuity inference.

## Rationale
Three options were considered:

1. **No continuity** — a disappeared path is always treated as a deleted file instance; any new file at a new path is always a new record. Simple, but destructive: any time the user reorganizes their archive, all catalog history (relationships, collection membership, user metadata) is lost. This is incompatible with the product's goal of "come back later and still understand what happened."

2. **Hash-based best-effort (chosen)** — safe and correct. Content identity is the most reliable available signal. If two instances have identical content, treating a path change as a move is almost always right and preserves meaningful catalog history.

3. **Name + size + timestamp heuristics** — produces false positives. Many common files (e.g., `export.mp4`, `final.pdf`) share names across completely unrelated sources and could be incorrectly linked.

## Scope and constraints
- Continuity inference applies only **within a single storage source** (same source, new path). Cross-source path changes are not inferred as moves — they produce new file instances, which is correct.
- Inference runs during the rescan pipeline **after hashing completes** (Stage 3+).
- Files that disappear before being hashed cannot benefit from continuity inference. Their history is lost.
- Edge case: a file is deleted and a genuinely different but hash-identical file appears at a new path. The system will incorrectly infer continuity. This is accepted as an edge case — content identity is the best available signal and the user can inspect and correct.

## Consequences
- Rescan must compare new file instance hashes against the last known hash for paths that no longer exist on that source
- A heuristic match triggers an update to `current_path` on the existing file instance record rather than creating a new record and orphaning the old one
- This behavior should be noted in scan progress output so users understand what happened
