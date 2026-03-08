# edgesentry-rs

A tamper-evident audit log system built in Rust for IoT devices to cloud services.

- **Repository:** [github.com/yohei1126/edgesentry-rs](https://github.com/yohei1126/edgesentry-rs)
- **Documentation:** [yohei1126.github.io/edgesentry-rs](https://yohei1126.github.io/edgesentry-rs/)

## Vision

**"Establishing a Transparent, Software-Defined Foundation of Trust for Public Infrastructure."**

EdgeSentry-RS is an **experimental**, commercially viable open-source reference implementation. Following the governance model of successful "in-process" systems like **DuckDB**, we separate the core intellectual property of the project from commercial interests to ensure its long-term neutrality and availability as a public good.

Our goal is to serve as the **Common Trust Layer** for vendors in public infrastructure, maritime (MPA), and smart buildings (BCA), helping them meet the highest regulatory standards — including Singapore's **CLS Level 3/4**, **iM8**, and Japan's **Unified Government Standards**.

### Three Pillars of Trust

Modeled after the "Simple, Portable, Fast" philosophy, EdgeSentry-RS implements three pillars of trust in Rust, designed for high-performance embedding:

1. **Identity** — Ed25519 digital signatures to guarantee the authenticity of both devices and data. Built with C/C++ FFI at its heart, allowing legacy industrial systems and robotics platforms to adopt secure identity without a full rewrite.

2. **Integrity** — BLAKE3 hash chains to ensure data immutability. Provides a verifiable cryptographic record that can be validated locally or in the cloud, ensuring forensic readiness even in offline scenarios.

3. **Resilience** — Intelligent data summarization for narrow-bandwidth environments (e.g., VDES/Coastal Link), ensuring critical security signals are prioritized over limited links.

### Governance

We believe the infrastructure of trust should not be owned by a single private entity.

- **Open for All:** A vendor-agnostic reference implementation that lowers the barrier for companies to achieve regulatory compliance.
- **Cross-Industry Learning:** Engineers collaborate across corporate boundaries to master the complexities of global IoT security standards.
- **Sustainable Growth:** The core remains a community-driven reference implementation; commercial services (advanced analytics, automated compliance reporting) are built on top of this stable foundation.

> **Project Status:** EdgeSentry-RS is currently in an **experimental research phase**. We are actively seeking collaboration with hardware partners and regulatory experts to refine this architecture into a world-class standard for secure infrastructure.

## Package

`edgesentry-rs` (`eds` binary, `edgesentry_rs` lib): A single Rust crate that includes all audit record types, hashing, signature verification, chain verification, device-side signed record generation, ingestion-time verification, deduplication, sequence validation, persistence workflow, and the CLI.

## Documentation

- [Introduction](docs/src/introduction.md) — motivation and package overview
- [Roadmap](docs/src/roadmap.md) — phased plan: Singapore → Japan → Europe
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
