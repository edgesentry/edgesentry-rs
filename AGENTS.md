# AGENTS Runbook

All procedures are documented in [docs/src/](docs/src/) and apply equally to humans and AI agents.

## Quick Reference

### Understanding the system
- **[Roadmap](docs/src/roadmap.md)** — phased compliance plan (Singapore → Japan → Europe), implementation mapping to ETSI EN 303 645 / CLS / JC-STAR
- **[Concepts](docs/src/concepts.md)** — tamper-evident design, AuditRecord fields, hash chain, sequence policy, ingest-time verification, storage model
- **[Architecture](docs/src/architecture.md)** — device side vs cloud side responsibilities, resource-constrained design, concrete sign-and-ingest flow

### Running examples and demos
- **[Library Usage](docs/src/quickstart.md)** — run `cargo run -p edgesentry-rs --example lift_inspection_flow`; S3/MinIO backend switching
- **[Interactive Demo](docs/src/demo.md)** — run `bash scripts/local_demo.sh`; requires Docker (PostgreSQL + MinIO)

### Using the CLI
- **[CLI Reference](docs/src/cli.md)** — `sign-record`, `verify-record`, `verify-chain` commands with examples; lift inspection end-to-end scenario; tampering detection walkthrough

### Development workflow
- **[Contributing](docs/src/contributing.md)** — macOS prerequisites, run `cargo test --workspace` after every change, static analysis (`clippy`, `cargo-audit`, `cargo-deny`), PR conventions (`gh pr create --assignee "@me"`)

## Avoiding conflicts with main

Conflicts occur when a feature branch diverges from main while main receives other merged PRs that touch the same files. The highest-conflict files in this repo are `scripts/local_demo.sh`, `docs/src/demo.md`, and `.github/copilot-instructions.md`.

**Before starting work**

```bash
git fetch origin
git checkout main && git pull
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

### Release
- **[Build and Release](docs/src/release.md)** — build artifacts, publish to crates.io, GitHub Actions CI/release pipeline, automatic version increment (Conventional Commits)
