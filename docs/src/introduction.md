# edgesentry-rs

**"Paving the Way for a New Global Standard: Mathematically Provable Integrity at the Edge."**

- **Repository:** [github.com/yohei1126/edgesentry-rs](https://github.com/yohei1126/edgesentry-rs)
- **Documentation:** [yohei1126.github.io/edgesentry-rs](https://yohei1126.github.io/edgesentry-rs/)

## Vision

EdgeSentry-RS is an **early-stage learning project** — we are building this to deepen our understanding of IoT security techniques hands-on. The license is commercially compatible (MIT/Apache 2.0), but the implementation is just getting started and is not yet production-ready. Following the governance model of successful "in-process" systems like **DuckDB**, we keep the core intellectual property open and vendor-neutral so it can grow into a public good over time.

Our goal is to serve as the **Common Trust Layer** for vendors in public infrastructure, maritime (MPA), and smart buildings (BCA), helping them meet the highest regulatory standards — including Singapore's **CLS Level 3/4**, **iM8**, and Japan's **Unified Government Standards**.

See the [Roadmap](roadmap.md) for the phased compliance plan.

## Three Pillars of Trust

Modeled after the "Simple, Portable, Fast" philosophy, EdgeSentry-RS implements three pillars of trust in Rust, designed for high-performance embedding:

1. **Identity** — Ed25519 digital signatures to guarantee the authenticity of both devices and data. Built with C/C++ FFI at its heart, allowing legacy industrial systems and robotics platforms to adopt secure identity without a full rewrite.

2. **Integrity** — BLAKE3 hash chains to ensure data immutability. Provides a verifiable cryptographic record that can be validated locally or in the cloud, ensuring forensic readiness even in offline scenarios.

3. **Resilience** *(planned)* — Intelligent data summarization for narrow-bandwidth environments, ensuring critical security signals are prioritized over limited links. See [Phase 2 in the Roadmap](roadmap.md).

## Governance

We believe the infrastructure of trust should not be owned by a single private entity.

- **Open for All:** A vendor-agnostic reference implementation that lowers the barrier for companies to achieve regulatory compliance.
- **Cross-Industry Learning:** Engineers collaborate across corporate boundaries to master the complexities of global IoT security standards.
- **Sustainable Growth:** The core remains a community-driven reference implementation; commercial services (advanced analytics, automated compliance reporting) are built on top of this stable foundation.

## Motivation

In recent years, labor shortages have become a serious challenge in infrastructure operations. Labor-intensive industries such as construction are increasingly adopting IoT devices for remote inspections.

At the same time, if device spoofing, device takeover, or inspection data tampering occurs, trust in the entire system is fundamentally undermined. This makes continuous verification of both device authenticity and data integrity essential.

For public-infrastructure IoT deployments, Singapore's Cybersecurity Labelling Scheme (CLS) may require Level 3 or Level 4, which introduces hardware-level security requirements. Because those hardware-dependent setups are often difficult to evaluate quickly in an early evaluation phase, this repository focuses on sample code for tamper prevention and tamper-evident audit records.

## Package

`edgesentry-rs` is the crate and binary name (`eds`). The Rust library is imported as `edgesentry_rs` (underscores). It includes all audit record types, hashing, signature verification, chain verification, ingestion-time verification, deduplication, sequence validation, persistence workflow, and the CLI.

## License

This project is licensed under either of:

- [Apache License, Version 2.0](https://github.com/yohei1126/edgesentry-rs/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/yohei1126/edgesentry-rs/blob/main/LICENSE-MIT)

at your option.
