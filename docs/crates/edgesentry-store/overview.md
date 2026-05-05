# edgesentry-store

Trait-abstracted event store.

## Backends
- In-memory (current, v1)
- Future: SQLite, DuckDB

## Interface
`RiskEvent` → store; query by time range, rule ID, entity ID.
