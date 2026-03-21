# Contributing

Full contribution guidelines are in [docs/src/contributing.md](docs/src/contributing.md) (English) and [docs/ja/src/contributing.md](docs/ja/src/contributing.md) (日本語).

## Quick start

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check licenses
```

## Reporting a vulnerability

Please see [SECURITY.md](SECURITY.md). Do not open a public issue for security vulnerabilities.
