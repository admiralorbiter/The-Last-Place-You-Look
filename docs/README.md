# The Last Place You Look — Docs

Documentation for **The Last Place You Look**, a local-first desktop app for understanding, comparing, and safely managing files across mounted local drives.

---

## Sections

### 01-product
Core product decisions that don't change often.

| File | Contents |
|---|---|
| `vision.md` | One-sentence statement, what the product is and is not, design principles, success criteria |
| `mvp.md` | MVP scope, included/excluded features, acceptance criteria by workflow |
| `personas.md` | Primary and secondary user personas with goals and frustrations |
| `glossary.md` | Canonical definitions for domain terms used across all docs |

---

### 02-features
User-facing feature specs: what the user sees and does, one feature per file.

| File | Contents |
|---|---|
| `information-architecture.md` | All top-level UI surfaces, navigation philosophy, view modes |
| `duplicate-review.md` | Duplicate group review workflow: member list, preferred copy, actions, edge cases |

---

### 03-architecture
Technical architecture decisions and subsystem designs.

| File | Contents |
|---|---|
| `architecture-overview.md` | Stack, architectural style, major subsystems, performance targets |
| `domain-model.md` | Asset + File Instance + Relationship Edge model, entity fields, identity rules |
| `duplicate-and-relationship-engine.md` | Duplicate detection, recommendation model, ranking signals, relationship maps |
| `scan-and-index-pipeline.md` | Staged scan approach, all stages, rescan and mount/unmount behavior |
| `protection-and-safety-model.md` | Protection states, quarantine design, recommendation language rules |

#### 03-architecture/adrs
Architecture Decision Records — one decision per file, kept permanently.

| ADR | Decision |
|---|---|
| `0001-stack.md` | Rust core + Tauri + React/TypeScript |
| `0002-asset-model.md` | Asset + File Instance + Relationship Edge domain model |
| `0003-per-drive-quarantine.md` | Per-drive quarantine instead of central quarantine |
| `0004-volume-guid-identity.md` | Windows Volume GUID as stable storage source identity |
| `0005-tauri-v2.md` | Tauri v2 (not v1) |
| `0006-zustand.md` | Zustand for frontend state management |
| `0007-rusqlite-migration.md` | rusqlite_migration for embedded schema management |
| `0008-preferred-copy-model.md` | Hybrid preferred copy: computed default + user-pinnable override |
| `0009-path-continuity.md` | Hash-based best-effort path continuity for moved/renamed files |

---

### 04-roadmap
Planning and phasing. Keep these updated as decisions are made.

| File | Contents |
|---|---|
| `milestones.md` | 5-phase plan grouping all 10 epics with dependency ordering |
| `epics.md` | All 10 epics with goals and completion criteria |
| `epic-01-stories.md` | Story breakdown for Epic 1: Foundation and App Shell |
| `epic-02-stories.md` | Story breakdown for Epic 2: Storage Source Registration |
| `open-questions.md` | Resolved decisions (with rationale) and remaining open questions |

---

### 05-quality
Test strategy and release gates.

| File | Contents |
|---|---|
| `test-strategy.md` | Test layers, fixture approach, critical safety tests, exploratory priorities |
| `release-checklist.md` | Runnable checklist for any MVP release candidate |

---

## Writing convention

- Each file is standalone — no implicit cross-file dependencies
- Open questions stay in `04-roadmap/open-questions.md` — do not scatter them across files
- Architecture decisions go in ADRs — do not embed decisions in prose descriptions
- Accepted ADRs are never edited retroactively; add a superseding ADR if a decision changes
