# EdgeSentry

[![License](https://img.shields.io/badge/license-Apache%202.0%20OR%20MIT-blue.svg)](LICENSE-APACHE)

IoT security primitives: cryptographic audit trail, physics-based rule evaluation, and tamper-evident evidence sealing.

## Crates

| Crate | Purpose |
|---|---|
| `edgesentry-types` | Shared types — `Entity`, `EntityFrame`, `EntityClass`, `SensorReading`, `EvidenceQuality` |
| `edgesentry-ingest` | Structured data ingestion — CSV replay, AIS stream, PLY/IFC loaders |
| `edgesentry-compute` | Physics computations — distance, TTC, zone membership, entity confidence |
| `edgesentry-profile` | Profile loader — validates `rules.json` and KB coverage |
| `edgesentry-evaluate` | Rule DSL and evaluation engine — produces `RiskEvent` JSONL |
| `edgesentry-assess` | Trend and correlation analysis — rising frequency, escalating risk |
| `edgesentry-explain` | LLM-powered plain-language alert explanations |
| `edgesentry-report` | Markdown safety report generation |
| `edgesentry-scenario` | Synthetic scenario generation — CSV fixtures and UDP simulation |
| `edgesentry-store` | Trait-abstracted event store |
| `edgesentry-audit` | Tamper-evident audit chain — Ed25519 + BLAKE3 |
| `edgesentry-inspect` | Edge-first 3D scan vs. reference deviation detection |
| `edgesentry-parse` | Maritime CSV/Parquet → `DocumentEntity` JSONL |
| `edgesentry-document` | Document compliance — AI field filling, rule checking, HTML render |
| `edgesentry-wasm` | WebAssembly bindings for the document pipeline |
| `edgesentry-bridge` | C/C++ FFI bridge |
| `edgesentry-image-utils` | Shared image processing utilities (ONNX, OpenCV) |
| `eds` | Unified CLI |

## Quick start

```bash
cargo build --workspace
cargo test --workspace
```

## Security

Report vulnerabilities via [GitHub private advisory](https://github.com/edgesentry/edgesentry-rs/security/advisories/new). See [SECURITY.md](SECURITY.md).

## License

Apache 2.0 OR MIT — see [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT).
