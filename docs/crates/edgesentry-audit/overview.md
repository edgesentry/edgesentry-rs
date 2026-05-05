# edgesentry-audit

Any payload → BLAKE3-hashed, Ed25519-signed `AuditRecord` appended to an immutable chain.

Each record hashes its payload and the previous record's hash. `eds audit verify-chain` detects any modification or insertion. Supports offline store-and-forward for intermittent connectivity.

Compliance targets: CLS Level 3 (SS 711:2025), JC-STAR, ETSI EN 303 645. See `docs/roadmap/security-compliance.md`.
