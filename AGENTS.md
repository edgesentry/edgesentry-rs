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

- Architecture decisions and crate contracts: `docs/crates/<crate>/overview.md`
- Roadmaps (valuable — do not delete): `docs/roadmap/index.md`, `docs/roadmap/audit.md`, `docs/roadmap/inspect.md`
- Security artifacts: `docs/security/`
- CV adapter contract: `docs/pipeline/cv-adapter-spec.md`
- Edge/cloud split design: `docs/pipeline/edge-cloud-split.md`

## Agent Skills

Skills live in `.agents/skills/`. Install with:

```bash
npx skills add edgesentry/edgesentry-rs
```

| Skill | Use when |
|---|---|
| `/eds-dev-workflow` | Before every commit |
| `/eds-add-profile-rule` | Adding a new detection rule |
| `/eds-new-crate` | Scaffolding a new crate |
| `/eds-verify-audit-chain` | Verifying a sealed audit chain |
