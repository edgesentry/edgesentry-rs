# edgesentry-audit

Tamper-evident immutable audit chain.

## Input → Output
Any payload (JSONL, document hash) → `AuditRecord` (BLAKE3 hash + Ed25519 signature)

## Design
- Each record hashes its payload and the hash of the previous record (chain linkage)
- `verify-chain` detects any modification or insertion
- Offline buffer: records accumulate locally, sync when connectivity resumes

## CLI
See `/eds-verify-audit-chain` skill or `eds audit --help`.

## Compliance targets
CLS Level 3 (Singapore SS 711:2025), JC-STAR (Japan), ETSI EN 303 645 (EU CRA).
See `roadmap.md`.
