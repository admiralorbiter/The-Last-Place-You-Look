# ADR 0003: Per-drive quarantine instead of central quarantine

## Status
Accepted

## Decision
Quarantine storage is **per storage source** (one quarantine folder per registered drive), not centralized in a single location.

## Rationale
The product is optimized for removable external drives with large media archives. A centralized quarantine would require cross-device file copies, which are:

- **Slow** — large media files (multi-GB video) copied across USB would be unacceptably slow for a "safe removal" action
- **Expensive** — requires free space on a separate drive the user may not have configured
- **Confusing** — quarantined files would be on a different drive from where they came from

Per-drive quarantine keeps "removed" files local to the source drive:
- The quarantine folder (`.tlpyl-quarantine`) lives at the root of each registered source
- If the drive is unplugged, quarantined files travel with it and remain accessible on reattachment
- Restore actions always work on the originating drive

## Consequences
- The UI must present quarantine as one coherent feature even though backing storage is per-drive
- The UI must make clear which drive a quarantined file lives on
- Restore operations require the originating drive to be mounted — this must be surfaced clearly
- Cross-source quarantine (e.g., "quarantine from Drive A onto Drive B") is out of scope for MVP
