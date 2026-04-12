# Open Questions

## Resolved

These questions were open at project start and have been decided. Recorded here for traceability.

| Question | Decision | Where recorded |
|---|---|---|
| How broad should initial file preview support be? | Thumbnails + basic preview panel. Images: full-size inline. Video: thumbnail only. PDF: first page image. Others: metadata only. No full in-app media player in MVP. | Epic 5 stories |
| Should project clustering ship in MVP or be lightly scaffolded only? | Scaffold only. `project_member_of` edge type exists in the domain model but no UI surface is built in MVP. Collections cover the user need in MVP. | domain-model.md |
| What exact ranking weights should likely-best-copy use? | Fixed internal weights, not user-tunable in MVP. The ranking formula is an implementation detail. User-tunable profiles are a post-MVP feature. | ADR 0008 |
| How much path continuity should be inferred automatically for moved/renamed files? | Hash-based best-effort. If a file disappears and a file with an identical content hash appears at a new path on the same source during rescan, continuity is inferred. Name-only or size-only inference is not used. | ADR 0009 |
| Should preferred copy be persisted directly or derived from signals each time? | Hybrid. Computed by default from ranking signals. User can pin a preferred copy, which persists as a `preferred_copy` relationship edge and survives rescans. | ADR 0008 |

---

## Still open — resolve before indicated epic

| Question | Resolve before | Current lean |
|---|---|---|
| What is the minimum target dataset size the scan pipeline must handle acceptably? (e.g., 50k files? 200k files?) | **Resolved during Epic 3.** Standard serial traversal handles 2M+ files on large mechanical HDDs in 7–15 minutes. Fast Scan (MFT) deferred as optional post-MVP extension. Target: 200k files in standard mode, unlimited in Fast Scan mode. | Epic 3, Story 3.5 |
| What is the exact UI pattern for user-pinning a preferred copy? Button in group review? Right-click context menu? | Epic 6 | Inline "Pin as preferred" button in the duplicate group review member list. |
| Which file types get a preview panel (beyond just thumbnail) in the MVP preview scope? | Epic 5 | Images: inline panel. Video: thumbnail only. PDF: first page rendered as image. All others: metadata panel only. |

---

## Deferred beyond MVP

- NAS / network source support
- Near-duplicate media similarity (perceptual hashing for images/video)
- Semantic search / LLM-assisted discovery
- Automated project reconstruction
- Cloud storage integrations
- User-tunable ranking weights for preferred copy
- Preset ranking profiles ("prefer newest," "prefer most organized source")
- Mobile companion app
- **Fast Scan via NTFS MFT** — optional elevated scan mode for NTFS volumes, ~10–20× faster than standard traversal on mechanical drives. Requires UAC Administrator prompt. See Story 3.5 in epic-03-stories.md for full technical design.
