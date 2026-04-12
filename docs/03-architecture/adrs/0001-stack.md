# ADR 0001: Rust core with Tauri and React/TypeScript

## Status
Accepted

## Decision
Use Rust for the core engine and Tauri + React/TypeScript for the desktop application shell and frontend.

## Rationale
The product needs a systems-heavy local engine plus a rich modern UI. Rust fits scanning, hashing, filesystem work, and safety-sensitive logic. React/TypeScript fits dashboard, library, and map-heavy UI work.

## Consequences
- clear backend/frontend boundary is required
- command/event contracts must stay clean
- frontend can iterate faster without pushing systems logic into UI code
