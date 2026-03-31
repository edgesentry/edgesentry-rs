# Contributing

Full contribution guidelines:

- **Audit** — [docs/audit/en/src/contributing.md](docs/audit/en/src/contributing.md) (English) · [docs/audit/ja/src/contributing.md](docs/audit/ja/src/contributing.md) (日本語)
- **Inspect** — [docs/inspect/en/src/contributing.md](docs/inspect/en/src/contributing.md) (English) · [docs/inspect/ja/src/contributing.md](docs/inspect/ja/src/contributing.md) (日本語)

## Crate placement principle

Every new feature must be placed in the correct layer. The rule is:

| Layer | Crate | What belongs there |
|---|---|---|
| **Pure math / geometry** | [`trilink-core`](https://github.com/edgesentry/trilink-core) | Pinhole projection/unprojection, pose buffer, math types. No I/O, no file formats, no external services. |
| **OSS inspection pipeline** | `edgesentry-inspect` (this workspace) | Sensor ingress (`FrameSource`, `SensorFrame`, mocks), IFC/PLY parsing, scan pipeline, deviation engine, OSS CLI. |
| **Commercial application** | [`edgesentry-app`](https://github.com/edgesentry/edgesentry-app) | SQLite egress, BIM server client, PDF compliance reports (CONQUAS/MLIT), Tauri desktop UI. |

**If you are unsure**, ask these questions:
- Could a non-construction domain (e.g. autonomous vehicles) reuse this? → `trilink-core`
- Is it inspection-specific but domain-agnostic (open-source)? → `edgesentry-inspect`
- Does it require a commercial license, SQLite, or proprietary API? → `edgesentry-app`

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
