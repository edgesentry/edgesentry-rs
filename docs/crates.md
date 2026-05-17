# Crates

# eds — Unified CLI

Composes all pipeline stages as subcommands. Same binary at edge and cloud — context is determined by which subcommands are invoked and which profile files are present.

## Inter-stage JSONL schemas

| Schema | Producer | Consumer |
|---|---|---|
| `eds.entity-frame` | `eds ingest replay` | `eds compute run` |
| `eds.measurement-frame` | `eds compute run` | `eds evaluate run` |
| `eds.risk-event` | `eds evaluate run` | `eds assess`, `eds explain`, `eds audit` |
| `eds.document-entity` | `eds parse maritime` | `eds document fill` |

# edgesentry-types

Shared types used across all edgesentry crates. No I/O — depended on by every other crate.

# edgesentry-ingest

Produces `eds.entity-frame` JSONL from structured input sources.

Infers `SensorReading` from `EntityClass` (e.g. Vessel/AisGap → AIS) when the source does not provide it explicitly.

# edgesentry-compute

`eds.entity-frame` JSONL → `eds.measurement-frame` JSONL

Runs physics computations (distances, TTC, zone membership, entity confidence) over each frame. No external I/O.

# edgesentry-profile

Loads and validates a profile directory:

```
<profile-name>/
  rules.json      # rule definitions
  params.toml     # edge-deployable threshold values (no regulation text)
  kb/             # one file per rule ID — used by edgesentry-explain
```

`regulation` in `rules.json` appears verbatim in `AuditRecord`s. Use the exact clause text.

Built-in profiles: `fixtures/demo/`, `fixtures/sg-port-safety/`, `fixtures/sg-maritime-security/`, `fixtures/sg-port-compliance/`

# edgesentry-evaluate

`eds.measurement-frame` JSONL + profile → `eds.risk-event` JSONL

`evidence_quality` in each `RiskEvent` is derived from the entity's computed confidence score. Simulation entities always produce `NOT_APPLICABLE`.

# edgesentry-assess

`eds.risk-event` JSONL → trend and correlation output

Detects rising frequency and escalating severity patterns across a `RiskEvent` stream.

# edgesentry-explain

`eds.risk-event` + KB file → plain-language explanation string

Calls an OpenAI-compatible endpoint (`--llm-url`). Falls back to a structured summary if the endpoint is unavailable.

# edgesentry-report

Pipeline output → Markdown safety report file

# edgesentry-scenario

Generates synthetic CSV fixtures and UDP entity streams for development and testing.

# edgesentry-store

Trait-abstracted event store for `RiskEvent` records. Current backend: in-memory. Future: SQLite, DuckDB.

# edgesentry-audit

Any payload → BLAKE3-hashed, Ed25519-signed `AuditRecord` appended to an immutable chain.

Each record hashes its payload and the previous record's hash. `eds audit verify-chain` detects any modification or insertion. Supports offline store-and-forward for intermittent connectivity.

Compliance targets: CLS Level 3 (SS 711:2025), JC-STAR, ETSI EN 303 645. See `docs/roadmap/strategy-compliance.md`.

## eds audit export-aims

**Input:** AuditRecord JSON array (`--chain`) · optional `RiskEvent` JSONL (`--events`) · optional profile directory (`--profile-dir`)

**Output:** JSON evidence bundle (`--out`) · optional Markdown summary (`--md`)

Maps the chain to ISO/IEC 42001 Annex A.4 controls:

| Control | What is populated |
|---|---|
| A.4.2 Resource documentation | Record count, device IDs, timestamp range, chain validity, `object_ref` types |
| A.4.3 Data resources | Unique `object_ref`s, rule IDs triggered, regulations referenced (from `--events`) |
| A.4.4 Tooling resources | `eds` version, `edgesentry-evaluate` crate, rule count and IDs (from `--profile-dir`) |
| A.4.5 System and computing resources | Phase 2 placeholder — see issue [#399](https://github.com/edgesentry/edgesentry-rs/issues/399) |
| A.4.6 Human resources | `document:` object refs counted as HITL-reviewed records |

All output includes a disclaimer: *control-aligned evidence for a customer's AIMS audit — not an ISO/IEC 42001 certificate.*

# edgesentry-zkp

Generic ZKP infrastructure — `ZkProgram` trait, `ZkProof` envelope, `ZkFramework` enum.

Implementing crates call `prover.prove(private_inputs)` → `ZkProof { framework, program_id, proof_bytes, public_values }` where `public_values` is base64-encoded JSON (the public attestation). `verify()` checks program_id and framework.

SP1 SDK is intentionally NOT a dependency of this crate — it belongs in the implementing crate (e.g. clarus/edge). This avoids licence conflicts (LGPL/MPL transitive deps from SP1) and keeps the trait crate Apache 2.0 / MIT clean.

Current implementations: `GreenMarkProgram` (BCA Green Mark EUI/COP/LPD attestation, in clarus) · `OtIntegrityProgram` (OT software integrity allowlist check, in clarus).

# edgesentry-inspect

Point cloud (LiDAR/ToF) → deviation report against a reference geometry.

Depends on `trilink-core` for 3D↔2D projection and unprojection. Do not reimplement those primitives here — they live in `edgesentry/trilink-core`.

See `docs/roadmap/feature-inspect.md`.

# edgesentry-parse

Maritime CSV/Parquet → `eds.document-entity` JSONL

`parquet-support` feature (default on) pulls C bindings via `snap`. Disable for WASM builds: `--no-default-features`.

# edgesentry-document

`eds.document-entity` JSONL → filled form JSONL → compliance alerts JSONL → HTML

Three `eds document` steps: `fill` (AI field completion), `check` (compliance rules), `gen` (HTML render).

`llm` feature (default on) pulls C/ASM bindings. Disable for WASM: `--no-default-features`.

# edgesentry-wasm

WebAssembly bindings for the document pipeline (`edgesentry-parse` → `edgesentry-document` → `edgesentry-audit`).

Build: `wasm-pack build --target web --no-default-features` — both `parquet-support` and `llm` must be disabled (C/ASM deps incompatible with wasm-bindgen).

Consumer: [documaris](https://documaris.pages.dev)

# edgesentry-bridge

C/C++ FFI bridge for `edgesentry-audit`. Header generated via `cbindgen`.

# edgesentry-image-utils

Shared image-processing utilities behind feature flags (`onnx`, `opencv`). No functionality without at least one enabled.

