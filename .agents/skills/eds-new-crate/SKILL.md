---
name: eds-new-crate
description: Scaffold a new crate in the edgesentry-rs Cargo workspace. Use when adding a new pipeline stage or utility crate.
license: Apache-2.0
compatibility: Requires Rust stable toolchain
metadata:
  repo: edgesentry-rs
---

## Steps

**1. Create the crate**

```bash
cargo new --lib crates/<crate-name>
```

**2. Register in the workspace**

Add to the root `Cargo.toml`:

```toml
[workspace]
members = [
  # ... existing ...
  "crates/<crate-name>",
]
```

**3. Set package metadata**

In `crates/<crate-name>/Cargo.toml`:

```toml
[package]
name = "edgesentry-<name>"
version = "0.1.0"
edition = "2021"
description = "One-line description (no trailing period)"
license = "MIT OR Apache-2.0"
```

**4. Verify**

```bash
cargo build --workspace
```

**5. Add an overview doc**

Create `docs/crates/<crate-name>/overview.md` with:
- What it does (not how)
- I/O contract: input type → output type
- Non-obvious dependencies

## Naming convention

- Crate name: `edgesentry-<noun>` (kebab-case)
- Rust module: `edgesentry_<noun>` (snake_case)
