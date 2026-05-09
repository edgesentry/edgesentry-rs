# EdgeSentry-RS

Reusable Rust crates and CLI for sensor-to-seal compliance pipelines.

- **[Architecture & Pipeline](pipeline/index.md)** — seven-stage pipeline, edge/cloud split, CV adapter contract
- **[Zero-Knowledge Proofs](zkp.md)** — what ZKPs are, how `edgesentry-zkp` works, mock vs SP1, regulatory fit
- **[Security & Compliance](security/index.md)** — CLS Level 3/4, ETSI EN 303 645, JC-STAR evidence package
- **[Roadmaps](roadmap/index.md)** — pipeline progress, inspect feature, SG/JP/EU compliance strategy
- **[API Reference](https://edgesentry.github.io/edgesentry-rs/api/)** — rustdoc for all 18 crates

## Quick start

```bash
cargo build --workspace
cargo test --workspace
```

## Agent Skills

```bash
npx skills add edgesentry/edgesentry-rs
```
