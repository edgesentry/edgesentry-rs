# Contributing to EdgeSentry Inspect

## Consistency Check

After every change — whether to code, tests, scripts, or docs — check that all three layers stay in sync:

1. **Code → Docs**: If you add, remove, or rename a module, function, CLI command, or behavior, update all docs that reference it (`architecture.md`, `cli.md`, `demo.md`, `roadmap.md`).
2. **Docs → Code**: If a doc describes a feature or command, verify it exists and works as described. Stale examples and wrong cargo feature names cause CI failures.
3. **Scripts → Code**: If you rename a test file or cargo feature, update every script and workflow that references it (e.g. `.github/workflows/ci.yml`).

A quick grep before opening a PR:

```bash
# Find docs that mention a symbol you changed
grep -r "<old-name>" docs/ scripts/ .github/
```

---

## Crate layout

| Crate | Purpose |
|-------|---------|
| `edgesentry-inspect` | IFC loader, deviation engine, heatmap renderer, JSON report |
| `eds` | Unified CLI binary — `eds inspect scan` entry point |
| `trilink-core` | Point cloud projection / unprojection (upstream dependency) |

---

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
| `priority:P0` | Must have — blocks a release or core pipeline functionality | Broken IFC loader, deviation engine panic, CLI crash on valid input |
| `priority:P1` | Nice to have — high value, scheduled for near-term | Built-in inference model, demo walkthrough, visualisation prototype |
| `priority:P2` | Good to have — valuable but deferrable | Compliance report generation, partner sensor plugins |
| `priority:P3` | Low priority — improvements with no urgency | CI optimisations, minor DX improvements |

When in doubt, ask: *"Does this block a user from running `eds inspect scan` end-to-end?"* If yes → P0. If it materially improves the experience → P1. If it is a milestone feature that can ship later → P2.

### Category labels

| Label | When to use |
|-------|-------------|
| `core` | Deviation engine, IFC geometry, heatmap, report serialisation |
| `compliance-governance` | CONQUAS / MLIT report generation, ISO 19650 integration |
| `devsecops` | CI/CD pipelines, static analysis, release automation |
| `platform-operations` | Field PC deployment, cloud sync, infrastructure |
| `hardware-needed` | Requires physical LiDAR / ToF sensor hardware (always pair with `priority:P2`) |

---

## Pull Request Conventions

Always assign the PR to its author:

```bash
gh pr create --assignee "@me" --title "..." --body "..."
```

---

## Mandatory: Run Tests After Every Code Change

After **every** code change, run:

```bash
cargo test --workspace
```

Do not consider a change complete until all tests pass.

---

## Running Tests

### Prerequisites (macOS)

```bash
brew install rustup-init
rustup-init -y
source "$HOME/.cargo/env"
rustup default stable
```

### Unit tests

```bash
# All crates
cargo test --workspace

# Inspect crate only
cargo test -p edgesentry-inspect
```

### Integration tests (CLI end-to-end)

```bash
cargo test -p eds --features transport-http,transport-tls --test cli_integration
```

---

## Static Analysis and License Check

Run before opening a PR:

```bash
# Lint
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Security advisories
cargo audit

# OSS license policy
cargo deny check licenses
```

---

## Avoiding Conflicts with Main

**Before starting work:**

```bash
git fetch origin
git checkout main && git pull origin main
git checkout -b <your-branch>
```

**Keep your branch up to date** — rebase onto main before opening a PR:

```bash
git fetch origin
git rebase origin/main
```

**Files most likely to conflict — coordinate before editing these:**

| File | Why it conflicts often |
|------|------------------------|
| `docs/inspect/en/src/demo.md` | Multiple PRs extend the demo walkthrough |
| `docs/inspect/en/src/cli.md` | Updated whenever CLI flags or subcommands change |
| `docs/inspect/en/src/roadmap.md` | Milestone status updated as work completes |
| `.github/workflows/ci.yml` | Touched by both feature and CI improvement PRs |
