# ADR 0008: Hybrid preferred copy — computed default with user pin

## Status
Accepted

## Decision
The preferred copy for a duplicate group is **computed from ranking signals by default**. A user can explicitly **pin** a specific copy as preferred, which is persisted as a `preferred_copy` relationship edge and overrides the computed result until the user changes it.

## Rationale
Three options were considered:

1. **Computed only** — derived fresh from signals on every query. Simple to implement but frustrates users who have already made a deliberate choice. Rescans can silently change the recommendation.
2. **Persisted only** — requires the user to explicitly act on every group before getting any value. High friction; defeats the purpose of having ranking signals at all.
3. **Hybrid (chosen)** — the system provides a computed recommendation for every group from the start. Users who agree can ignore it. Users who disagree can pin their own choice, which persists and survives rescans.

This matches the product's design principle of "explain uncertainty clearly" — the distinction between a system-computed suggestion and a user-pinned decision is meaningful and should be visible in the UI.

## Ranking signals (MVP, fixed weights)
Applied in order of descending weight:
1. Not quarantined
2. Lives on a preferred or protection-rule-satisfying source
3. Richer metadata completeness
4. Not in a path that resembles temp/export/trash folders (heuristic)
5. Newer meaningful timestamp where relevant
6. Collection/tag membership (user has engaged with this file)

Weights are internal implementation details — not exposed to users in MVP.

## Consequences
- The `preferred_copy` relationship edge type must be treated as a user override: do not clear it on rescan
- The UI must clearly distinguish "System suggested" from "Pinned by you"
- If a pinned copy is quarantined or its source goes offline, surface a warning prompting the user to re-evaluate
- Post-MVP: user-selectable ranking profiles ("prefer newest", "prefer most organized source") would supplement this model
