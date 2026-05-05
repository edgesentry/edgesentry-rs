# Contributing

## Scope

This repository is a **Rust library and CLI for IoT security primitives**. Business use cases are implemented in application repositories (clarus, documaris, arktrace). Do not add business logic or application-specific documentation here.

## Crate placement

| Layer | Where | What belongs there |
|---|---|---|
| Pure math / geometry | [`trilink-core`](https://github.com/edgesentry/trilink-core) | Projection, pose buffer, math types — no I/O, no file formats |
| IoT security primitives | this workspace | Signing, audit chain, physics engine, rule evaluation, document pipeline |
| Application / business logic | app repos (clarus, documaris, …) | Profiles for specific regulations, business workflows, UI |

## Build and test

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check licenses
```

All must pass before committing.

## Language

English is the single source of truth for all documentation. Do not create translated (`.ja.md`, etc.) versions — they diverge silently and double the maintenance cost.

## Documentation rules

1. **README.md** — human-facing, high-level only
2. **AGENTS.md** — agent-facing, high-level only
3. **Agent Skills** — step-by-step procedures (`npx skills add edgesentry/edgesentry-rs`)
4. **`docs/`** — supplementary information that fits none of the above
5. **Do not write what `cargo doc` already shows** — type names, struct fields, method signatures belong in rustdoc only
6. **No duplication** — each fact lives in exactly one place
7. **No business use cases** — those belong in application repositories
8. **Roadmaps** → `docs/roadmap/` (do not delete)
9. **IoT security compliance** → `docs/security/`

### File naming

All files under `docs/` use `kebab-case.md`. Use role prefixes where they aid discoverability:

| Prefix | Use for |
|---|---|
| `feature-` | A specific product feature (e.g. `feature-inspect.md`) |
| `strategy-` | Market or regulatory strategy (e.g. `strategy-compliance.md`) |
| `tier-` | Architecture layer documents (e.g. `tier-architecture.md`) |
| `ingest-` | Pipeline entry-point specs (e.g. `ingest-cv-adapter.md`) |

### Skill-first policy

Before adding a step-by-step procedure to `docs/`, create a Skill instead:

1. `mkdir .agents/skills/<skill-name>`
2. Write `SKILL.md` with frontmatter (`name`, `description`)
3. Put reference material in `references/` if the procedure requires it
4. Link from AGENTS.md skills table

Only add to `docs/` if the content is **reference** (facts, schemas, thresholds), not **procedure** (how-to steps).

## Agent Skills

Skills follow the [agentskills.io](https://agentskills.io/specification) spec and live in `.agents/skills/`.

```bash
npx skills add edgesentry/edgesentry-rs
```

## Issues

Add every new issue to the relevant [project board](https://github.com/orgs/edgesentry/projects) with a priority set.

| Label | Meaning |
|---|---|
| `priority:P0` | Blocks a release or core functionality |
| `priority:P1` | High value, scheduled near-term |
| `priority:P2` | Valuable but deferrable |

## Security vulnerabilities

See [SECURITY.md](SECURITY.md). Do not open a public issue for vulnerabilities.
