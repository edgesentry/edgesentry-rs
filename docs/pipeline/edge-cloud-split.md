# Edge / Cloud Pipeline Split

The seven-step pipeline can be partitioned across two execution tiers.
Steps 1–2 must run at the edge. Steps 5–7 are best run in the cloud.
Step 3 (Evaluate) is the key split point.

---

## Principle

**The edge seals facts. The cloud interprets them.**

Capturing a tamper-evident measurement and determining which regulation it violates
are two operations that do not need to happen at the same time or place.

| Tier | When | What happens |
|---|---|---|
| **Edge** (synchronous) | At the moment of breach | Physics computed, threshold crossed, operator alerted, raw measurement sealed and uploaded |
| **Cloud** (asynchronous) | After upload | Regulatory rule looked up, severity assigned, LLM explanation generated, compliance report produced |

The regulatory knowledge base — rule definitions, regulation texts, updated circulars —
never needs to be deployed to edge devices. It lives in the cloud and is updated there.
A regulation update takes effect once in the cloud without any field deployment.

---

## Edge tier

```
eds ingest stream   # or eds ingest replay
      │ EntityFrame JSONL
      ▼
eds compute run
      │ Measurement JSONL
      ▼
eds measure run     # lightweight threshold check — no rules.json needed
      │ MeasurementRecord JSONL
      │   { breach_type, measured_value, threshold,
      │     entity_ids, timestamp_ms, profile_version, site_id }
      ▼
eds audit sign      # seal with BLAKE3 + Ed25519
      │ sealed MeasurementRecord JSONL
      ▼
eds r2 push --immutable   # upload to R2 Object Lock bucket
```

**Operator alert** fires inline during `eds measure run` before sealing — within 1 second.

**What the edge profile needs:** `params.toml` (threshold values, zone geometry)
and `manifest.toml` (version, jurisdiction). No `rules.json`. No `kb/`.

---

## Cloud tier

```
eds r2 pull         # download sealed MeasurementRecords
      │ sealed MeasurementRecord JSONL
      ▼
eds evaluate run --mode cloud   # MeasurementRecord + full profile → EvaluatedRecord
      │ EvaluatedRecord JSONL
      │   { measurement_record_hash, rule_id, severity, regulation,
      │     site_id, timestamp_ms }
      ▼
eds audit sign      # seal EvaluatedRecord — same chain
      │ sealed EvaluatedRecord JSONL
      ▼
eds explain run     # LLM plain-language explanation (async)
      ▼
eds report generate # compliance report PDF
```

**What the cloud profile needs:** full profile — `params.toml` + `rules.json` + `kb/` + `manifest.toml`.

---

## Two-tier chain

Both `MeasurementRecord` and `EvaluatedRecord` are sealed with `edgesentry-audit`
and appended to the same BLAKE3 + Ed25519 hash chain in the R2 immutable bucket.

```
Edge:   MeasurementRecord  ──seal──▶  R2 (Object Lock)
                                           │
Cloud:  EvaluatedRecord    ──seal──▶  R2 (same chain)
        (refs measurement_record_hash)
```

A verifier querying the chain receives both records for a given event and can confirm:
- The physical measurement (from `MeasurementRecord`) — sealed at the edge
- The regulatory determination (from `EvaluatedRecord`) — applied asynchronously
- The link between them (via `measurement_record_hash`) — cryptographically verified

Neither record can be altered after upload.

---

## Profile split

| File | Edge device | Cloud |
|---|---|---|
| `params.toml` — threshold values, zone geometry | ✅ | ✅ |
| `manifest.toml` — version, jurisdiction | ✅ | ✅ |
| `rules.json` — rule_id, condition, regulation, severity | ❌ | ✅ |
| `kb/` — regulatory knowledge base for LLM | ❌ | ✅ |

---

## Same binary

The `eds` binary is identical at the edge and in the cloud.
The execution context is determined by which subcommands are invoked
and which profile files are present — not by build flags or separate binaries.

---

## New types

`MeasurementRecord` and `EvaluatedRecord` are defined in `edgesentry-evaluate`.
The existing `RiskEvent` type is kept for backward compatibility and for the full
single-tier pipeline (where all steps run at the edge).

For full type definitions and build order, see `_inputs/edge_cloud_pipeline.md`.
