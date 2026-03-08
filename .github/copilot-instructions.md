# Copilot Instructions for edgesentry-rs

## Project Overview

`edgesentry-rs` is a tamper-evident audit log proof-of-concept written in Rust for IoT-to-cloud deployments. It provides cryptographic signing, hash chaining, and ingestion-time verification for inspection records from edge devices (for example, lift inspection sensors).

For core concept definitions see [docs/src/concepts.md](../docs/src/concepts.md).
For all executable procedures (build, test, CLI usage, demo scripts) see [AGENTS.md](../AGENTS.md).

## Repository Structure

```
Cargo.toml                          # Workspace root (single member)
crates/
  edgesentry-rs/
    Cargo.toml                      # Crate manifest (lib + eds binary)
    src/
      lib.rs                        # Public library surface
      main.rs                       # eds CLI entry point
      record.rs                     # AuditRecord type and hashing
      chain.rs                      # Hash-chain verification
      crypto.rs                     # Ed25519 sign/verify helpers (BLAKE3 hash)
      agent.rs                      # Device-side signed-record builder
      ingest/                       # Cloud-side ingestion and trust enforcement
        mod.rs                      # Module exports
        policy.rs                   # IntegrityPolicyGate — P0 gate (route, sig, seq, hash)
        verify.rs                   # IngestState — per-device sequence/hash state machine
        storage.rs                  # IngestService + in-memory and S3 store impls
    examples/
      lift_inspection_flow.rs       # End-to-end library example (no external deps)
    tests/                          # Integration tests
scripts/
  run_unit_and_license_check.sh     # Unit tests + cargo-deny license check
  local_demo.sh                     # Interactive local demo (requires Docker)
deny.toml                           # cargo-deny license policy
docker-compose.local.yml            # PostgreSQL + MinIO for local demo
```

## Architecture

The codebase is split into **device side** and **cloud side** concerns within a single crate:

- **Device side** (`agent` module): builds `AuditRecord` values, computes `payload_hash` (BLAKE3), signs with Ed25519, and links records into a hash chain via `prev_record_hash`.
- **Cloud side** (`ingest` module): verifies device identity, validates signatures, enforces sequence monotonicity, checks hash-chain continuity, and rejects tampered or replayed data before persistence.
- **Shared** (`record`, `chain`, `crypto` modules): types and algorithms used by both sides.

The `AuditRecord` fields are:

| Field | Purpose |
|---|---|
| `device_id` | Source device identity |
| `sequence` | Monotonically increasing counter per device |
| `timestamp_ms` | Event timestamp |
| `payload_hash` | BLAKE3 hash of raw payload data (`[u8; 32]`) |
| `signature` | Ed25519 signature over `payload_hash` (`[u8; 64]`) |
| `prev_record_hash` | Hash of the previous record (zero-hash for first record) |
| `object_ref` | Reference to raw payload storage (e.g. `s3://...`) |

## Coding Conventions

- **Language / edition:** Rust 2021, stable toolchain.
- **Workspace layout:** single workspace member `crates/edgesentry-rs`. Workspace-level `[workspace.package]` centralises `version`, `edition`, `license`, and `repository`; each crate uses `*.workspace = true` to inherit them.
- **Internal dependencies:** declared in `[workspace.dependencies]` with both `version` and `path`; referenced in member crates via `dep.workspace = true`.
- **Error handling:** use `thiserror` for library error types. Avoid `unwrap`/`expect` in library code; use them only in examples or tests.
- **Serialisation:** `serde` with `derive` feature for all record types; `postcard` for compact binary wire format; `serde_json` for CLI file I/O.
- **Hashing:** BLAKE3 (`blake3` crate) for all content hashing.
- **Signatures:** Ed25519 via `ed25519-dalek`.
- **CLI:** `clap` with the `derive` feature.
- **Optional S3 feature:** AWS S3 / MinIO support is behind the `s3` cargo feature. Add AWS SDK dependencies as optional under that feature.
- **Comments:** add comments only when they match the style of surrounding code or explain a non-obvious design choice.
- **Clippy:** all code must pass `cargo clippy --workspace --all-targets --all-features -- -D warnings` without warnings.

## Build and Test Commands

Run all unit tests:

```bash
cargo test --workspace
```

Run tests with the S3 feature enabled:

```bash
cargo test -p edgesentry-rs --features s3
```

Run unit tests and OSS license check together:

```bash
./scripts/run_unit_and_license_check.sh
```

Static analysis:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Build release artifacts:

```bash
cargo build --workspace --release
```

## Quality Gates

Before opening a PR or releasing, all of the following must pass:

1. `cargo test --workspace` — all unit tests green
2. `cargo test -p edgesentry-rs --features s3` — S3-feature tests green
3. `cargo clippy --workspace --all-targets --all-features -- -D warnings` — zero warnings
4. `cargo deny check licenses` — OSS license policy satisfied

## Commit Message Convention

This repository uses [Conventional Commits](https://www.conventionalcommits.org/) for automatic version bumping:

- `fix:` → patch bump
- `feat:` → minor bump
- `feat!:` or `BREAKING CHANGE:` in footer → major bump

## CI / Release

- **CI** (`.github/workflows/ci.yml`): runs on every push/PR — build, test, license check, clippy, and `cargo publish --dry-run`.
- **Auto-version-tag** (`.github/workflows/auto-version-tag.yml`): after CI passes on `main`, bumps `workspace.package.version` in `Cargo.toml` and pushes a `vX.Y.Z` tag.
- **Release** (`.github/workflows/release.yml`): triggered by a `vX.Y.Z` tag — publishes `edgesentry-rs` to crates.io and builds Linux / macOS / Windows binaries as GitHub Release assets.

Required secret: `CRATES_IO_TOKEN`.
