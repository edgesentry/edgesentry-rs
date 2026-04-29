# Build and Release

## Build Release Artifacts

```bash
cargo build --workspace --release
```

Build a specific crate only:

```bash
cargo build -p edgesentry-rs --release
```

## Publish to crates.io

1) Validate quality gates first:

```bash
./scripts/run_unit_and_license_check.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

2) Login once:

```bash
cargo login <CRATES_IO_TOKEN>
```

3) Dry-run publish:

```bash
cargo publish --dry-run -p edgesentry-rs
```

4) Publish:

```bash
cargo publish -p edgesentry-rs
```

## GitHub Actions Release Automation (macOS / Windows / Linux)

This repository includes `.github/workflows/release.yml`.

- Trigger: push a tag like `v0.1.0`
- Quality gate: build, unit tests, license check, clippy
- Publish `edgesentry-rs` to crates.io
- Build `eds` binaries for Linux, macOS (x64 + arm64), and Windows
- Upload packaged binaries to GitHub Release assets

Note: `.github/workflows/ci.yml` runs `cargo publish --dry-run` for `edgesentry-rs`.

Required GitHub secret:

- `CRATES_IO_TOKEN`: crates.io API token used by `cargo publish`

## Automatic Version Increment After Merge

This repository also includes `.github/workflows/auto-version-tag.yml`.

- Trigger: when `CI` succeeds on `main`
- Action: update `workspace.package.version` in `Cargo.toml` and create/push a `vX.Y.Z` tag
- Then: `release.yml` is triggered by that tag and performs the full release pipeline

Version bump rules (Conventional Commits):

- `fix:` -> patch bump (`x.y.z` -> `x.y.(z+1)`)
- `feat:` -> minor bump (`x.y.z` -> `x.(y+1).0`)
- `!` or `BREAKING CHANGE` -> major bump (`x.y.z` -> `(x+1).0.0`)
