# Contributing

Full contribution guidelines:

- **Audit** — [docs/audit/en/src/contributing.md](docs/audit/en/src/contributing.md) (English) · [docs/audit/ja/src/contributing.md](docs/audit/ja/src/contributing.md) (日本語)
- **Inspect** — [docs/inspect/en/src/contributing.md](docs/inspect/en/src/contributing.md) (English) · [docs/inspect/ja/src/contributing.md](docs/inspect/ja/src/contributing.md) (日本語)

## Quick start

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check licenses
```

## Opening issues

Every new issue must:

1. **Carry proper labels** — one type label (`bug`, `enhancement`, `documentation`), one priority label, and one or more category labels (see the per-project contributing guides for the full label reference)
2. **Be added to the relevant [edgesentry project board](https://github.com/orgs/edgesentry/projects)** with a priority set

```bash
# Add to the relevant project board after creating the issue
gh project item-add <project-number> --owner edgesentry --url <issue-url>
```

## Issue priorities

| Label | Meaning |
|-------|---------|
| `priority:P0` | Must have — blocks a release or core functionality |
| `priority:P1` | Nice to have — high value, scheduled for near-term |
| `priority:P2` | Good to have — valuable but deferrable |
| `priority:P3` | Low priority — improvements with no urgency |

## Reporting a vulnerability

Please see [SECURITY.md](SECURITY.md). Do not open a public issue for security vulnerabilities.
