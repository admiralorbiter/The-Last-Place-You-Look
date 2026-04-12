# ADR 0005: Tauri v2

## Status
Accepted

## Decision
Use **Tauri v2** (not v1) for the desktop application shell.

## Rationale
The project is starting from zero with no existing v1 codebase to preserve. Tauri v2 is the current stable release and offers:

- A cleaner Rust API surface for backend command and event registration
- A more granular capability and permission system — important for a filesystem-heavy app that needs to declare explicit access scopes
- Active development and community direction; v1 is in maintenance mode

Starting on v1 and migrating later would be a significant disruption with no benefit.

## Consequences
- Some older Tauri v1 community examples will not apply directly — check API versions when referencing external code
- The Tauri v2 capability system requires explicitly declaring file system access in capability config files
- Mobile targets are technically possible in Tauri v2 but are not used in MVP and are not a design consideration
