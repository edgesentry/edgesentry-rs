# Contributing

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

Install Rust toolchain first:

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

Run edgesentry-rs with S3-compatible backend feature enabled:

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
