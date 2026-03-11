# AGENTS Runbook

All procedures are documented in [docs/src/](docs/src/) and apply equally to humans and AI agents.

## Quick Reference

### Understanding the system
- **[Roadmap](docs/src/roadmap.md)** — phased compliance plan (Singapore → Japan → Europe), implementation mapping to ETSI EN 303 645 / CLS / JC-STAR
- **[Concepts](docs/src/concepts.md)** — tamper-evident design, AuditRecord fields, hash chain, sequence policy, ingest-time verification, storage model
- **[Architecture](docs/src/architecture.md)** — device side vs cloud side responsibilities, resource-constrained design, concrete sign-and-ingest flow

### Running examples and demos
- **[Library Usage](docs/src/quickstart.md)** — run `cargo run -p edgesentry-rs --example lift_inspection_flow`; S3/MinIO backend switching
- **[Interactive Demo](docs/src/demo.md)** — run `bash scripts/local_demo.sh`; requires Docker (PostgreSQL + MinIO)

### Using the CLI
- **[CLI Reference](docs/src/cli.md)** — `sign-record`, `verify-record`, `verify-chain` commands with examples; lift inspection end-to-end scenario; tampering detection walkthrough

### Development workflow
- **[Contributing](docs/src/contributing.md)** — macOS prerequisites, run `cargo test --workspace` after every change, static analysis (`clippy`, `cargo-audit`, `cargo-deny`), PR conventions, avoiding conflicts with main

### Release
- **[Build and Release](docs/src/release.md)** — build artifacts, publish to crates.io, GitHub Actions CI/release pipeline, automatic version increment (Conventional Commits)
