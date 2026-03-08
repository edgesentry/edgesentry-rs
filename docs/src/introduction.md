# edgesentry-rs

A tamper-evident audit log system built in Rust for IoT devices to cloud services.

- **Repository:** [github.com/yohei1126/edgesentry-rs](https://github.com/yohei1126/edgesentry-rs)
- **Documentation:** [yohei1126.github.io/edgesentry-rs](https://yohei1126.github.io/edgesentry-rs/)

## Vision

**"Establishing a Transparent, Software-Defined Foundation of Trust for Public Infrastructure."**

EdgeSentry-RS is an **experimental**, commercially viable open-source reference implementation. Following the governance model of successful "in-process" systems like **DuckDB**, we separate the core intellectual property of the project from commercial interests to ensure its long-term neutrality and availability as a public good.

Our goal is to serve as the **Common Trust Layer** for vendors in public infrastructure, maritime (MPA), and smart buildings (BCA), helping them meet the highest regulatory standards — including Singapore's **CLS Level 3/4**, **iM8**, and Japan's **Unified Government Standards**.

See the [Roadmap](roadmap.md) for the phased plan.

## Motivation

In recent years, labor shortages have become a serious challenge in infrastructure operations. Labor-intensive industries such as construction are increasingly adopting IoT devices for remote inspections.

At the same time, if device spoofing, device takeover, or inspection data tampering occurs, trust in the entire system is fundamentally undermined. This makes continuous verification of both device authenticity and data integrity essential.

For public-infrastructure IoT deployments, Singapore's Cybersecurity Labelling Scheme (CLS) may require Level 3 or Level 4, which introduces hardware-level security requirements. Because those hardware-dependent setups are often difficult to evaluate quickly in an early evaluation phase, this repository focuses on sample code for tamper prevention and tamper-evident audit records.

## Package

`edgesentry-rs` (`eds` binary, `edgesentry_rs` lib): A single Rust crate that includes all audit record types, hashing, signature verification, chain verification, device-side signed record generation, ingestion-time verification, deduplication, sequence validation, persistence workflow, and the CLI.

## License

This project is licensed under either of:

- [Apache License, Version 2.0](https://github.com/yohei1126/edgesentry-rs/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/yohei1126/edgesentry-rs/blob/main/LICENSE-MIT)

at your option.
