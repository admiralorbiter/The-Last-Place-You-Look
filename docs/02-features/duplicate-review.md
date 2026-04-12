# Feature: Duplicate Group Review

## Purpose

Give users a structured, safe, and explained experience for resolving a duplicate group. The user must be able to understand what's happening, why the system recommends a copy, and take action without fear of making a mistake that can't be undone.

---

## Entry points

- Home dashboard: "X duplicate groups need review" summary card
- Library view: filter by duplicate state → select a group
- Item Detail page: "View duplicate group" link when item is part of a group

---

## Group review view

### Header
- File type icon + group summary: "3 exact copies detected"
- Content confirmation: "Identical content confirmed by hash"
- Member count and whether any members are offline

### Member list

Each member is a card showing:
- Full path and storage source name
- File size (all should match for exact duplicates — flag any discrepancy)
- Timestamps: modified date, created date if available
- Metadata completeness indicator (e.g., has EXIF data, has media info)
- Mount status: Online badge or Offline badge
- Collection / tag membership
- Rule-derived protection state
- "Intentional Backup" badge if marked
- "Pinned as preferred" badge if user-pinned

### Preferred copy highlight

One member is highlighted as recommended:
- Labeled "Likely best copy" (system computed) or "Your pinned copy" (user-pinned)
- Reason list: e.g., "On your primary source · Has complete metadata · Not in a temp folder"
- "Pin as preferred" button on each non-preferred member
- If user clicks "Pin as preferred" on a different member: confirm, update persisted edge, re-render

### Per-member comparison panel

A comparison showing key attributes across all members in a table or side-by-side layout:
- Timestamp (all should match for exact duplicates)
- Location quality indicator (organized folder vs temp/export/trash-like path)
- Source and its protection status
- Collection membership
- Intentional backup status

---

## Available actions per member

| Action | What it does | Requires confirmation |
|---|---|---|
| Mark as intentional backup | Adds `intentional_backup` edge; changes duplicate language in UI | No |
| Pin as preferred copy | Persists `preferred_copy` edge; overrides computed suggestion | No |
| Move | Guided move to another registered source | Yes |
| Merge | Move all other members to this member's location and quarantine the rest | Yes |
| Quarantine | Send this file to its source's per-drive quarantine | Yes |

### Confirmation step content
All move, merge, and quarantine actions show a confirmation modal containing:
- What will happen in plain language
- Which file(s) will move and where they will go
- What is reversible (quarantine = reversible; merge = partially reversible)
- "Cancel" and "Confirm" buttons — no keyboard shortcut to bypass confirmation

---

## Edge cases to handle in implementation

| Case | Expected behavior |
|---|---|
| All members on the same source | Note this prominently — no cross-source protection regardless of copy count |
| One or more members offline | Disable action buttons for offline members; show "Drive offline — reconnect to act" |
| User-pinned preferred copy is offline | Show warning banner: "Your pinned copy is on an offline drive" |
| User-pinned preferred copy is quarantined | Show warning banner and prompt to re-evaluate or remove the pin |
| Group has only 2 members, one is quarantined | Note that only one active copy remains — "Only Copy" warning may now apply |
| Group has more than ~10 members | Paginate or virtualize the member list — do not crash the UI |

---

## What this view is not

- **Not an automatic resolver** — the user always confirms before anything happens to files
- **Not a bulk processor** — designed for reviewing one group at a time with full context
- **Not a file manager** — no arbitrary filesystem navigation from within this view
- **Not a permanent history log** — past actions are in the Quarantine view, not inline here
