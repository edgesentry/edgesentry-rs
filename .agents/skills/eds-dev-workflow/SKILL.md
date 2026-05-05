---
name: eds-dev-workflow
description: Build, test, and lint edgesentry-rs before committing. Use before every commit or when verifying the workspace is clean.
license: Apache-2.0
compatibility: Requires Rust stable toolchain, cargo-deny
metadata:
  repo: edgesentry-rs
---

Run in order. All must pass before committing.

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check licenses
```

`-D warnings` treats every clippy warning as an error — fix all of them.

If `cargo deny` fails, check `deny.toml` for the affected license or advisory ID.
