# Contributing

## Issue Labels

Every issue should carry one **type** label, one **priority** label, and one or more **category** labels.

### Type labels

| Label | When to use |
|-------|-------------|
| `bug` | Something is broken or behaves incorrectly |
| `enhancement` | New feature or improvement to existing behavior |
| `documentation` | Docs-only change — no production code affected |

### Priority labels

| Label | Meaning | Examples |
|-------|---------|---------|
| `priority:P0` | Must-have — directly required to satisfy a target standard (CLS, JC-STAR, CRA). Work is blocked until resolved. | Broken signature verification, missing hash-chain link, failing integrity gate |
| `priority:P1` | Good-to-have — strengthens compliance posture or developer experience but is not a hard blocker for standard conformance. | Key rotation tooling, CI hardening, traceability matrix, FFI bridge |
| `priority:P2` | Best-effort — stretch goals, nice-to-haves, or anything that requires dedicated hardware. Pursue if capacity allows. | HSM integration, education white papers, reference architectures |

When in doubt, ask: *"Does the standard explicitly require this?"* If yes → P0. Otherwise, if it helps but is not mandated → P1. For stretch goals, nice additions, or hardware-dependent work → P2.

### Category labels

| Label | When to use |
|-------|-------------|
| `core` | Core security controls — signing, hashing, integrity gate, ingest pipeline |
| `compliance-governance` | Compliance evidence, traceability matrices, disclosure processes |
| `devsecops` | CI/CD pipelines, supply-chain security, static analysis, audit tooling |
| `platform-operations` | Infrastructure, deployment, operational readiness |
| `hardware-needed` | Requires physical hardware or hardware-backed infrastructure (always pair with `priority:P2`) |

---

## Pull Request Conventions

When creating a pull request, always assign it to the user who authored the branch:

```bash
gh pr create --assignee "@me" --title "..." --body "..."
```

## Mandatory: Run Tests After Every Code Change

After **every** code change, run:

```bash
cargo test --workspace
```

Do not consider a change complete until all tests pass.

## Unit Tests

### Prerequisites (macOS)

Install the Rust tool chain first:

```bash
brew install rustup-init
rustup-init -y
source "$HOME/.cargo/env"
rustup default stable
```

Install `cargo-deny` (required for OSS license checks):

```bash
cargo install cargo-deny
source "$HOME/.cargo/env"
cargo deny --version
```

### Running Tests

Run all unit tests:

```bash
cargo test --workspace
```

Run tests for a specific crate:

```bash
cargo test -p edgesentry-rs
```

Run the `edgesentry-rs` crate with the S3-compatible backend feature enabled:

```bash
cargo test -p edgesentry-rs --features s3
```

Run unit tests + OSS license checks in one command:

```bash
./scripts/run_unit_and_license_check.sh
```

## Static Analysis and OSS License Check

Use the following checks before release.

### 1) Static analysis (`clippy`)

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### 2) Dependency security advisory check (`cargo-audit`)

Install once:

```bash
cargo install cargo-audit
```

Run:

```bash
cargo audit
```

### 3) Commercial-use OSS license check (`cargo-deny`)

Install once:

```bash
cargo install cargo-deny
```

Run license check (policy in `deny.toml`):

```bash
cargo deny check licenses
```

Optional full dependency policy check:

```bash
cargo deny check advisories bans licenses sources
```

If this check fails, inspect violating crates and update dependencies or the policy only after legal/security review.
