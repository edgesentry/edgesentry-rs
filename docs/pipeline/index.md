# Pipeline Architecture

Seven-stage sensor-to-seal pipeline. Each stage is a separate crate piping JSONL to the next.

```
[Edge]                         [Cloud]
  Step 1  eds ingest replay    → eds.entity-frame
  Step 2  eds compute run      → eds.measurement-frame
  Step 3  eds evaluate run     → eds.risk-event          ← Edge/Cloud boundary
  ─────────────────────────────────────────────────────
  Step 4  eds assess
  Step 5  eds explain
  Step 6  eds document gen
  Step 7  eds audit sign-document → AuditRecord (BLAKE3 + Ed25519)
```

Steps 1–3 are deterministic and suitable for real-time edge execution.
Steps 4–7 may involve latency, external services, or async scheduling.

## Documents

| Document | Covers |
|---|---|
| [tier-architecture.md](tier-architecture.md) | Why Edge/Cloud are separated; which crates run where; design principles |
| [tier-implementation.md](tier-implementation.md) | Concrete Rust types, CLI commands, profile split (`params.toml` edge-only) |
| [ingest-cv-adapter.md](ingest-cv-adapter.md) | CV adapter contract — Step 0/1 input: camera frames → `eds.entity-frame` JSONL |
