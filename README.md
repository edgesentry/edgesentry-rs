# edgesentry-rs

A tamper-evident audit log system built in Rust for IoT devices to cloud services.

- **Repository:** [github.com/yohei1126/edgesentry-rs](https://github.com/yohei1126/edgesentry-rs)
- **Documentation:** [yohei1126.github.io/edgesentry-rs](https://yohei1126.github.io/edgesentry-rs/)

`edgesentry-rs` (`eds` binary, `edgesentry_rs` lib): A single Rust crate that includes all audit record types, hashing, signature verification, chain verification, device-side signed record generation, ingestion-time verification, deduplication, sequence validation, persistence workflow, and the CLI.

## Documentation

- [Introduction](docs/src/introduction.md) — motivation and package overview
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
