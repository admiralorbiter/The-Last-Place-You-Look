# ADR 0007: rusqlite_migration for embedded schema management

## Status
Accepted

## Decision
Use the **`rusqlite_migration`** crate for SQLite schema versioning and migration management.

## Rationale
The product needs reliable, embedded schema migrations that:
- Run automatically at app startup without external tooling
- Track migration state in the database itself
- Work cleanly with the `rusqlite` connection model already required for the domain

`rusqlite_migration` is purpose-built for exactly this embedded use case. It integrates directly with `rusqlite::Connection`, supports Tokio async, and tracks migration state via an internal version table. No server-side plumbing or CLI tooling is needed.

Alternatives considered:
- **`refinery`** — designed for server-side workflows with file-based migration directories; more ceremony than needed here
- **Manual SQL migration management** — would require writing version tracking, conflict detection, and rollback logic that `rusqlite_migration` provides out of the box

## Consequences
- Migrations are defined as numbered Rust constants (inline SQL strings) in the `persistence/` module
- All schema changes require a new numbered migration — editing the live DB schema manually is not a valid workflow
- Migration state is tracked by `rusqlite_migration` in a `__migrations` table managed by the crate
- Migrations run at app startup via `init_db()` before any other database access
