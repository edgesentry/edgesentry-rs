# AGENTS

Rust library and CLI for IoT security primitives. No business use cases are implemented here — those live in application repositories (clarus, documaris, arktrace).

## Crate map

| Crate | I/O contract |
|---|---|
| `edgesentry-types` | Shared types — no I/O |
| `edgesentry-ingest` | CSV / AIS / PLY → `EntityFrame` JSONL (`eds.entity-frame`) |
| `edgesentry-compute` | `EntityFrame` JSONL → `MeasurementFrame` JSONL (`eds.measurement-frame`) |
| `edgesentry-profile` | `rules.json` + `kb/` → loaded `Profile` |
| `edgesentry-evaluate` | `MeasurementFrame` + `Profile` → `RiskEvent` JSONL |
| `edgesentry-assess` | `RiskEvent` stream → trend / correlation output |
| `edgesentry-explain` | `RiskEvent` + KB → plain-language explanation |
| `edgesentry-report` | pipeline output → Markdown report |
| `edgesentry-scenario` | config → CSV / UDP synthetic fixture |
| `edgesentry-store` | `RiskEvent` → trait-abstracted store (in-memory, future backends) |
| `edgesentry-audit` | any payload → BLAKE3-hashed, Ed25519-signed `AuditRecord` |
| `edgesentry-inspect` | point cloud → deviation report |
| `edgesentry-parse` | maritime CSV/Parquet → `DocumentEntity` JSONL |
| `edgesentry-document` | `DocumentEntity` → filled FAL form + compliance alerts |
| `edgesentry-wasm` | document pipeline → WASM bindings for browser |
| `edgesentry-bridge` | `edgesentry-audit` → C/C++ FFI |
| `edgesentry-image-utils` | frame → ONNX / OpenCV utilities |
| `eds` | CLI — composes all stages via subcommands |

## Build and test

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check licenses
```

All tests must pass and clippy must be clean before every commit.

## Coding conventions

- Rust 2021, stable toolchain
- `thiserror` for errors — no `unwrap`/`expect` in library code
- `serde` for serialisation — `f32` for geometry (consistent with trilink-core)
- No company names in source, comments, or docs

## Commit convention

Conventional Commits:
- `fix:` → patch
- `feat:` → minor
- `feat!:` or `BREAKING CHANGE:` → major

## Docs

- Crate I/O contracts and constraints: `docs/crates.md`
- Roadmaps (valuable — do not delete): `docs/roadmap/core-pipeline.md`, `docs/roadmap/feature-inspect.md`, `docs/roadmap/strategy-compliance.md`
- Security artifacts (valuable — do not delete): `docs/security/` — threat-model, compliance-matrix, sbom-lifecycle, key-management
- CV adapter contract: `docs/pipeline/ingest-cv-adapter.md`
- Edge/cloud split design: `docs/pipeline/tier-architecture.md`

## Agent Skills

Skills live in `.agents/skills/`. Install with:

```bash
npx skills add edgesentry/edgesentry-rs
```

| Skill | Trigger |
|---|---|
| `/eds-dev-workflow` | Before every commit; when CI fails on `cargo clippy` or `cargo deny` |
| `/eds-add-profile-rule` | Adding a regulation-backed detection rule; when a `RiskEvent` is missing for a known regulation |
| `/eds-new-crate` | Adding a new pipeline stage or utility crate to the workspace |
| `/eds-verify-audit-chain` | After `eds audit sign-document`; when investigating a tamper allegation; before submitting to an assessor |
| `/eds-deploy` | Setting up a new server; when TLS, PostgreSQL, S3, or systemd configuration is needed |
| `/eds-ops` | When a health check fails; investigating chain lag; running scheduled backup or restore |
| `/eds-release` | After all tests pass and a version tag is ready; when crates.io publish fails mid-run |
