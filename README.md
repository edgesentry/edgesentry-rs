# edgesentry-rs

**"Paving the Way for a New Global Standard: Mathematically Provable Integrity at the Edge."**

An early-stage learning project building tamper-evident audit log infrastructure in Rust for IoT devices to cloud services. The license is commercially compatible (MIT/Apache 2.0); the implementation is actively in development and not yet production-ready.

- **Repository:** [github.com/yohei1126/edgesentry-rs](https://github.com/yohei1126/edgesentry-rs)
- **Documentation:** [yohei1126.github.io/edgesentry-rs](https://yohei1126.github.io/edgesentry-rs/)

## Getting Started

| Goal | Where to start |
|------|---------------|
| Understand the concepts | [Concepts](docs/src/concepts.md) |
| Run a quick in-memory example (no Docker needed) | [Library Usage](docs/src/quickstart.md) |
| Run the full end-to-end demo (PostgreSQL + MinIO) | [Interactive Demo](docs/src/demo.md) |
| Use the CLI | [CLI Reference](docs/src/cli.md) |
| Contribute or run tests | [Contributing](docs/src/contributing.md) |

## Package

`edgesentry-rs` is the crate and binary name (`eds`). The Rust library is imported as `edgesentry_rs` (underscores). It includes all audit record types, hashing, signature verification, chain verification, ingestion-time verification, deduplication, sequence validation, persistence workflow, and the CLI.

## Documentation

- [Introduction](docs/src/introduction.md) — vision, motivation, three pillars of trust
- [Roadmap](docs/src/roadmap.md) — phased plan: Singapore → Japan → Europe, compliance mapping
- [Concepts](docs/src/concepts.md) — tamper-evident design, AuditRecord, hash chain
- [Architecture](docs/src/architecture.md) — device side vs cloud side, design flow
- [Library Usage](docs/src/quickstart.md) — in-memory example, S3/MinIO switching
- [Interactive Demo](docs/src/demo.md) — end-to-end demo with PostgreSQL + MinIO
- [CLI Reference](docs/src/cli.md) — CLI commands and lift inspection scenario
- [Contributing](docs/src/contributing.md) — prerequisites, tests, static analysis, PR conventions
- [Build and Release](docs/src/release.md) — release pipeline and version automation

## License

This project is licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT license](LICENSE-MIT)

at your option.
