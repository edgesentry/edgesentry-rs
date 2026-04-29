# EdgeSentry

EdgeSentry is a collection of reusable Rust crates and a unified CLI (`eds`) for building
sensor-to-seal compliance pipelines.

## Seven-step pipeline

Any domain that needs to capture real-world data, check it against regulations, explain
deviations, and produce a tamper-evident record fits the same pattern:

| Step | Role | Crate | CLI |
|------|------|-------|-----|
| 1 - Ingest | Capture sensor data or parse documents | `edgesentry-ingest` / `edgesentry-parse` | `eds ingest` / `eds parse` |
| 2 - Compute | Apply physics and geometry operations | `edgesentry-compute` | `eds compute` |
| 3 - Evaluate | Compare measurements against rules | `edgesentry-evaluate` | `eds evaluate` |
| 4 - Assess | Find patterns across evaluation results | `edgesentry-assess` | `eds assess` |
| 5 - Explain | Generate grounded plain-language text | `edgesentry-explain` | `eds explain` |
| 6 - Document | Format results into reports or documents | `edgesentry-report` / `edgesentry-document` | `eds report` / `eds document` |
| 7 - Seal | Sign and chain records for tamper detection | `edgesentry-audit` | `eds audit` |

## Quick links

- [Pipeline documentation](introduction.md)
- [Quickstart - Safety Monitoring](quickstart-safety-monitoring.md)
- [Quickstart - Document Compliance](quickstart-document-compliance.md)
- [CLI Reference](cli-reference.md)
