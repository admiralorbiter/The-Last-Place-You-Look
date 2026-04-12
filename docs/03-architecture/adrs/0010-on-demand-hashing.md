# ADR 0010: On-demand hashing as default; bulk hashing as opt-in

## Status
Accepted

## Context
BLAKE3 hashing is required for confirmed exact duplicate detection. Early design assumed hashing would run automatically in the background after every scan, completing before the user noticed.

In practice, a 6.5 TB external drive with hundreds of thousands of files would take hours to fully hash on spinning magnetic media. Running this as a background default would:
- Saturate I/O and degrade scan performance
- Heat the drive and increase wear
- Deliver no user-visible value for the majority of the time spent
- Block the drive from being safely unmounted

## Decision
**Hashing is on-demand only by default.**

1. **Auto on open**: when a user opens a file in the detail panel and the file is online and not yet hashed, hashing is triggered automatically. The hash is persisted to `file_instances.blake3_hash` immediately and is available for all future duplicate queries.
2. **Bulk hashing**: available as an explicit opt-in action per source. Not triggered automatically by scans.
3. **The gap is filled by probable detection**: files that have not been hashed can still appear as probable duplicates if they share the same `file_name` and `size_bytes` (≥ 512 KB). This provides immediate value without any disk I/O beyond what Stage 1 already performed.

## Consequences
- Duplicate grouping will always have two confidence tiers (Confirmed / Probable), not a single unified view
- Users who want full confirmed coverage must either open files or trigger bulk hashing manually
- The Duplicate Review page must make the two tiers clearly distinct and avoid implying false completeness
- `file_instances.blake3_hash` is the source of truth; once set, it is never recomputed unless the file's `modified_at` changes on a rescan
- The analysis queries for the Duplicate Review page must not hold the shared DB mutex — they use a dedicated read-only connection in `spawn_blocking` to keep the app responsive

## Related
- ADR 0008: Hybrid preferred copy model
- ADR 0009: Path continuity inference
- `duplicate-and-relationship-engine.md`: Two-tier detection model
