# EdgeSentry

[![License](https://img.shields.io/badge/license-Apache%202.0%20OR%20MIT-blue.svg)](LICENSE-APACHE)

Trust and verification for edge infrastructure.

- **Documentation:** [edgesentry.github.io/edgesentry-rs/en/](https://edgesentry.github.io/edgesentry-rs/en/)

This repository is a Cargo workspace containing two crates:

| Crate | Description |
|---|---|
| [edgesentry-audit](crates/edgesentry-audit/) | Ed25519 + BLAKE3 cryptographic audit trail for IoT devices and infrastructure |
| [edgesentry-inspect](crates/edgesentry-inspect/) | Edge-first 3D scan vs. reference deviation detection for construction and maritime inspection |

## edgesentry-audit

Implements three pillars of trust for IoT deployments:

1. **Identity** — Ed25519 digital signatures to guarantee the authenticity of devices and data
2. **Integrity** — BLAKE3 hash chains to ensure data immutability and forensic readiness
3. **Resilience** — Store-and-forward offline buffering for narrow-bandwidth and intermittent-connectivity environments

Designed to support Singapore's Cybersecurity Labelling Scheme (CLS) Level 3/4, iM8, and Japan's Unified Government Standards.

### Documentation — edgesentry-audit

| Document | Description |
|---|---|
| [Introduction](docs/audit/en/src/introduction.md) | Vision, three pillars of trust, motivation |
| [Roadmap](docs/audit/en/src/roadmap.md) | Phased plan: Singapore → Japan → Europe, compliance mapping |
| [Concepts](docs/audit/en/src/concepts.md) | Tamper-evident design, AuditRecord, hash chain |
| [Architecture](docs/audit/en/src/architecture.md) | Device side vs cloud side, design flow |
| [Library Usage](docs/audit/en/src/quickstart.md) | **Start here** — in-memory example, no Docker needed |
| [Interactive Demo](docs/audit/en/src/demo.md) | End-to-end demo with PostgreSQL + MinIO |
| [CLI Reference](docs/audit/en/src/cli.md) | CLI commands and lift inspection scenario |
| [Contributing](docs/audit/en/src/contributing.md) | Prerequisites, tests, static analysis, PR conventions |
| [Build and Release](docs/audit/en/src/release.md) | Release pipeline and version automation |

## edgesentry-inspect

Detects structural deviations at the field edge by fusing 3D point clouds with reference design data — no cloud round-trip required during inspection.

```
3D sensor (LiDAR/ToF)
    │  point cloud
    ▼
trilink-core::project          ← 3D → 2D depth map / height map
    │  depth map (image)
    ▼
AI inference                   ← anomaly detection (built-in model or HTTP endpoint)
    │  bounding boxes + class
    ▼
trilink-core::unproject        ← 2D detections → 3D world coords
    │  world-space anomaly points
    ▼
Scan-vs-reference engine       ← compare against reference design geometry
    │  deviation heatmap + report
    ▼
Field display (tablet / AR)    ← inspector sees deviation on site
    │
    ▼  (upload report only)
Cloud audit store              ← immutable evidence + digital twin update
```

### Target use cases

| Domain | Constraint | How EdgeSentry-Inspect addresses it |
|---|---|---|
| Construction site inspection | Full unit scan-and-verdict within 30 min | Edge-only pipeline; no upload before verdict |
| Maritime structure inspection | Intermittent connectivity; autonomous robot | Local anomaly flag; cloud sync after mission |

### Documentation — edgesentry-inspect

| Document | Description |
|---|---|
| [Background](docs/inspect/en/src/background.md) | Problem, pain points, and how it differs from existing solutions |
| [Requirements](docs/inspect/en/src/requirements.md) | Inspection constraints, KPIs, and regulatory context |
| [Scenarios](docs/inspect/en/src/scenarios.md) | Step-by-step flows, construction and maritime case studies |
| [Architecture](docs/inspect/en/src/architecture.md) | Edge-cloud split, AI inference modes, technology choices |
| [Roadmap](docs/inspect/en/src/roadmap.md) | Milestone plan; links to trilink-core issues for foundation work |

## Security

To report a vulnerability privately, use [GitHub's private vulnerability reporting](https://github.com/edgesentry/edgesentry-rs/security/advisories/new). See [SECURITY.md](SECURITY.md) for the full disclosure policy, supported versions, response SLAs, and scope.

## License

This project is licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT license](LICENSE-MIT)

at your option.
