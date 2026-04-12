# ADR 0002: Asset + File Instance + Relationship Edge domain model

## Status
Accepted

## Decision
Model the domain using three core entities: **Asset**, **File Instance**, and **Relationship Edge**. Do not use a file-only model.

## Rationale
A purely file-centric model conflates "what a file is" (logical identity) with "where it lives" (physical location). This breaks down the moment the product needs to reason about:

- **Duplicates** — the same logical content at multiple paths is one Asset, many File Instances
- **Storage maps** — showing where an Asset's copies live requires the Files→Sources relationship
- **Intentional backups** — a backup copy and an accidental duplicate both point to the same content, but their relationship intent is different and must be preserved in the relationship layer, not collapsed
- **Preferred copy** — selecting "the best one" requires grouping instances under a shared identity

A file-only model would require re-deriving these groupings on every query, and would make the UI inconsistent about what "this item" refers to.

## Consequences
- Asset records must be created and maintained alongside File Instance records
- Relationship edges need a typed, extensible schema
- Queries spanning assets and instances require joins — this is acceptable for a local embedded SQLite database
- The UI consistently distinguishes between "the thing" (Asset) and "where it is" (File Instance)
- Preferred copy and intentional backup are relationship edges, not fields on File Instance
