# Introduction

EdgeSentry is a Rust toolkit for building sensor-to-seal compliance pipelines. Any domain that needs to capture real-world data, check it against regulations, explain deviations, and produce a tamper-evident record fits the same seven-step pattern.

## The seven steps

| Step | Role | CLI | Crate |
|------|------|-----|-------|
| Step 1 - Ingest | Capture structured sensor data or parse unstructured documents | `eds ingest` / `eds parse` | `edgesentry-ingest` / `edgesentry-parse` |
| Step 2 - Compute | Apply physics and geometry operations to raw measurements | `eds compute` | `edgesentry-compute` |
| Step 3 - Evaluate | Compare measurements against regulations or design specs | `eds evaluate` | `edgesentry-evaluate` |
| Step 4 - Assess | Correlate evaluation results across time to find patterns | `eds assess` | `edgesentry-assess` |
| Step 5 - Explain | Translate assessments into grounded plain-language text | `eds explain` | `edgesentry-explain` |
| Step 6 - Document | Format results into a report or official document | `eds report` / `eds document` | `edgesentry-report` / `edgesentry-document` |
| Step 7 - Seal | Sign each record; chain records for tamper detection | `eds audit` | `edgesentry-audit` |

## Design principles

**Pipeline stages are independent processes.** Each `eds` command reads JSONL from a file and writes JSONL to a file. No shared in-memory state between stages. This makes every stage independently testable and the entire pipeline reproducible from any point.

**evaluate vs assess.** Evaluate is fact-checking — does this single measurement breach a rule? Assess is insight — what pattern emerges across many evaluations? Single vs multiple events is not the axis; fact vs interpretation is.

**The engine is domain-agnostic.** The same seven crates handle warehouse safety monitoring, maritime document compliance, and 3D point-cloud deviation analysis. The domain lives in profiles and templates, not the engine.

## Inter-stage data format

Every stage writes a headed JSONL file — line 1 is a schema header, subsequent lines are records:

```json
{"eds_schema":"eds.entity-frame","version":"0.1"}
{"timestamp_ms":1000,"entity_id":"FL-01","entity_type":"forklift","x":25.0,"y":8.0,"vx":-1.0,"vy":0.0}
```

The header is validated by `JsonlReader` before any records are read, catching schema mismatches early.

## Delivered scope (Phases 1–3)

| Phase | PR | Crates added |
|-------|----|--------------|
| 1 | [#270](https://github.com/edgesentry/edgesentry-rs/pull/270) | edgesentry-ingest, edgesentry-compute, edgesentry-evaluate, edgesentry-profile |
| 2 | [#288](https://github.com/edgesentry/edgesentry-rs/pull/288) | edgesentry-store, edgesentry-assess, edgesentry-explain + UDP ingest |
| 3 | [#289](https://github.com/edgesentry/edgesentry-rs/pull/289) | edgesentry-report, edgesentry-parse, edgesentry-document |
