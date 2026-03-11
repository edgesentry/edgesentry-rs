# Contributing

## Consistency Check

After every change — whether to code, tests, scripts, or docs — check that all three layers stay in sync:

1. **Code → Docs**: If you add, remove, or rename a module, function, CLI command, or behaviour, update all docs that reference it (`concepts.md`, `architecture.md`, `cli.md`, `quickstart.md`, `demo.md`, `traceability.md`).
2. **Docs → Code**: If a doc describes a feature or command, verify it exists and works as described. Stale examples and wrong test target names cause CI failures.
3. **Scripts → Code**: If you rename a test file or cargo feature, update every script and workflow that references it (e.g. `scripts/integration_test.sh`, `.github/workflows/`).
4. **Traceability**: If you implement or change a compliance control, update the status in `docs/src/traceability.md` (✅ / ⚠️ / 🔲).

A quick grep before opening a PR:

```bash
# Find docs that mention a symbol you changed
grep -r "<old-name>" docs/ scripts/ .github/
```

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

Run S3 integration tests against a live MinIO instance (requires the env vars below to be set):

```bash
TEST_S3_ENDPOINT=http://localhost:9000 \
TEST_S3_ACCESS_KEY=minioadmin \
TEST_S3_SECRET_KEY=minioadmin \
TEST_S3_BUCKET=bucket \
cargo test -p edgesentry-rs --features s3 --test integration -- --nocapture
```

Tests skip automatically when any of the four `TEST_S3_*` variables are unset.

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

---

## Avoiding Conflicts with Main

Conflicts occur when a feature branch diverges from main while main receives other merged PRs that touch the same files. The highest-conflict files in this repo are `scripts/local_demo.sh`, `docs/src/demo.md`, and `.github/copilot-instructions.md`.

**Before starting work**

```bash
git fetch origin
git checkout main && git pull origin main
git checkout -b <your-branch>
```

**Keep your branch up to date** — rebase onto main regularly, especially before opening a PR:

```bash
git fetch origin
git rebase origin/main
```

**Resolving a conflict during rebase**

1. Identify conflicted files: `git diff --name-only --diff-filter=U`
2. For each file, decide which side to keep:
   - **Take your version:** `git checkout --theirs <file>`
   - **Take main's version:** `git checkout --ours <file>`
   - **Merge manually:** edit the file to remove `<<<<<<<` / `=======` / `>>>>>>>` markers
3. Stage the resolved file: `git add <file>`
4. Continue: `GIT_EDITOR=true git rebase --continue`
5. If a conflict recurs on the next commit, repeat from step 1.

**After resolving, force-push the rebased branch:**

```bash
git push --force-with-lease origin <your-branch>
```

**Files most likely to conflict — coordinate before editing these:**

| File | Why it conflicts often |
|------|----------------------|
| `scripts/local_demo.sh` | Multiple PRs add steps or restructure the demo flow |
| `docs/src/demo.md` | Mirrors demo script changes |
| `.github/copilot-instructions.md` | Structure section updated whenever new modules or examples are added |
| `crates/edgesentry-rs/examples/lift_inspection_flow.rs` | Touched by both quickstart improvements and role-boundary work |
