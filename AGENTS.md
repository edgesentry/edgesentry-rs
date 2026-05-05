# AGENTS Runbook

This repository is a Cargo workspace containing two crates:

- **edgesentry-audit** — cryptographic audit trail (Ed25519 + BLAKE3) for IoT devices and infrastructure
- **edgesentry-inspect** — edge-first 3D scan vs. reference deviation detection for construction and maritime inspection

All procedures below apply equally to humans and AI agents.

## Guidelines

**After every change, verify consistency across code, tests, and docs.** See [Contributing — Consistency Check](docs/audit/en/src/contributing.md#consistency-check) for the checklist.

**English and Japanese documentation must be updated together.** Every change to `docs/*/en/src/*.md` requires a corresponding update to `docs/*/ja/src/*.md`, and vice versa. Never update one language without updating the other.

## Build and test

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check licenses
```

All tests must pass before any commit. No clippy warnings allowed.

## Quick Reference — edgesentry-audit

### Understanding the system
- **[Roadmap](docs/audit/en/src/roadmap.md)** — phased compliance plan (Singapore → Japan → Europe), implementation mapping to ETSI EN 303 645 / CLS / JC-STAR
- **[Concepts](docs/audit/en/src/concepts.md)** — tamper-evident design, AuditRecord fields, hash chain, sequence policy, ingest-time verification, storage model
- **[Architecture](docs/audit/en/src/architecture.md)** — device side vs cloud side responsibilities, resource-constrained design, concrete sign-and-ingest flow

### Running examples and demos
- **[Library Usage](docs/audit/en/src/quickstart.md)** — run `cargo run -p edgesentry-audit --example lift_inspection_flow`; S3/MinIO backend switching
- **[Interactive Demo](docs/audit/en/src/demo.md)** — run `bash scripts/run_local_demo.sh`; requires Docker (PostgreSQL + MinIO)

### Using the CLI
- **[CLI Reference](docs/audit/en/src/cli.md)** — `sign-record`, `verify-record`, `verify-chain` commands with examples; lift inspection end-to-end scenario; tampering detection walkthrough

### Development workflow
- **[Contributing](CONTRIBUTING.md)** — issue priorities, quick-start commands, links to full per-project guides
- **[Audit Contributing](docs/audit/en/src/contributing.md)** — macOS prerequisites, run `cargo test --workspace` after every change, static analysis (`clippy`, `cargo-audit`, `cargo-deny`), PR conventions, avoiding conflicts with main
- **[Inspect Contributing](docs/inspect/en/src/contributing.md)** — crate layout, inspect-specific issue labels, running inspect unit and CLI integration tests

### Release
- **[Build and Release](docs/audit/en/src/release.md)** — build artifacts, publish to crates.io, GitHub Actions CI/release pipeline, automatic version increment (Conventional Commits)

## Quick Reference — edgesentry-inspect

### Understanding the system
- **[Requirements](docs/inspect/en/src/requirements.md)** — before designing any feature; defines the 30-min inspection constraint and KPIs
- **[Architecture](docs/inspect/en/src/architecture.md)** — before writing any code; edge-cloud split, component interfaces, AI inference modes
- **[Roadmap](docs/inspect/en/src/roadmap.md)** — before picking up a task; milestone sequence and trilink-core dependencies
- **[Scenarios](docs/inspect/en/src/scenarios.md)** — step-by-step flows, construction and maritime case studies

### Key external dependency: trilink-core

`edgesentry-inspect` depends on `trilink-core` for:

| Symbol | Purpose |
|---|---|
| `PointCloud` | Input type for a single LiDAR/ToF sweep |
| `DepthMap` | Output of `project_to_depth_map` — fed to AI inference |
| `HeightMap` | Output of `project_to_height_map` — floor-level anomaly view |
| `project_to_depth_map` | 3D point cloud → 2D depth map (forward projection) |
| `project_to_height_map` | 3D point cloud → top-down height map |
| `unproject` | 2D detection + depth → 3D world coordinate |
| `PoseBuffer` | Pose lookup by timestamp |
| `Transform4x4`, `CameraIntrinsics`, `Point3D` | Shared geometry types |

These are implemented in the `trilink-core` repo ([issues #30–#34](https://github.com/edgesentry/trilink-core/issues)). Do not reimplement them here.

## Quick Reference — Maritime document pipeline (PIER71 / documaris)

### Understanding the system
- **[WASM build guide](docs/pipeline/wasm-build.md)** — how to compile edgesentry-wasm, feature flag rationale, API reference, integration with documaris
- **[PIER71 demo runbook](docs/pipeline/pier71-demo-runbook.md)** — TC1/TC2/TC3 test cases, manual run steps, expected outputs
- **[Document compliance quickstart](docs/pipeline/quickstart-document-compliance.md)** — end-to-end CLI walkthrough

### Key facts
- Maritime crates: `edgesentry-parse` → `edgesentry-document` → `edgesentry-audit` → `edgesentry-wasm`
- WASM build requires `--no-default-features` (disables `parquet-support` and `llm` features — both pull C/ASM deps incompatible with wasm-bindgen)
- Web app consumer: [documaris](https://documaris.pages.dev) (repo: `edgesentry/documaris`)
- Demo script: `bash demo/document-pipeline.sh`

## Repository structure

```
edgesentry-rs/
  crates/
    edgesentry-audit/    — cryptographic audit trail (Ed25519, BLAKE3, offline buffer)
    edgesentry-bridge/   — C/C++ FFI bridge for edgesentry-audit
    edgesentry-inspect/  — scan-vs-reference engine (implementation begins at M2)
    edgesentry-parse/    — maritime CSV ingestion → ParsedDocument
    edgesentry-document/ — FAL form filling, compliance rules, HTML render
    edgesentry-wasm/     — wasm-bindgen bindings for browser use
  demo/
    document-pipeline.sh — FAL Form 1 end-to-end demo (TC1/TC2/TC3)
  docs/
    audit/               — audit documentation (English + Japanese)
    inspect/             — inspect documentation (English + Japanese)
    pipeline/            — maritime document pipeline docs
      wasm-build.md      — WASM compilation, feature flags, API reference
      pier71-demo-runbook.md — PIER71 test cases and demo steps
  scripts/
    run_local_demo.sh    — end-to-end audit demo (Docker)
    preview_docs.sh      — build and serve all docs locally at localhost:8080/edgesentry-rs/
```

## Coding conventions

- Rust 2021, stable toolchain
- `thiserror` for errors; no `unwrap`/`expect` in library code
- `serde` for serialisation; `f32` for geometry (consistent with trilink-core types)
- All code must pass `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- No company names in source code, comments, or docs — use generic terms

## Commit convention

Conventional Commits:

- `fix:` → patch bump
- `feat:` → minor bump
- `feat!:` or `BREAKING CHANGE:` → major bump
