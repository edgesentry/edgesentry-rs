---
name: eds-release
description: Publish a new edgesentry-rs release to crates.io. Use when cutting a release after all tests pass.
license: Apache-2.0
compatibility: Requires cargo-release, crates.io token configured
metadata:
  repo: edgesentry-rs
---

## Pre-flight

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check licenses
```

## Dry-run publish (verify everything before touching crates.io)

```bash
cargo publish --dry-run -p edgesentry-types
cargo publish --dry-run -p edgesentry-audit
# repeat for each crate in dependency order
```

## Publish in dependency order

```bash
cargo publish -p edgesentry-types
cargo publish -p edgesentry-ingest
cargo publish -p edgesentry-compute
cargo publish -p edgesentry-profile
cargo publish -p edgesentry-evaluate
cargo publish -p edgesentry-assess
cargo publish -p edgesentry-explain
cargo publish -p edgesentry-report
cargo publish -p edgesentry-scenario
cargo publish -p edgesentry-store
cargo publish -p edgesentry-audit
cargo publish -p edgesentry-parse
cargo publish -p edgesentry-document
cargo publish -p edgesentry-wasm
cargo publish -p eds
```

## Version bump

Versions follow Conventional Commits:
- `fix:` → patch
- `feat:` → minor
- `feat!:` or `BREAKING CHANGE:` → major

GitHub Actions (`release.yml`) automates tagging and crates.io publish on push to `main`.

See [references/release.md](references/release.md) for GitHub Actions workflow details and manual override procedures.
