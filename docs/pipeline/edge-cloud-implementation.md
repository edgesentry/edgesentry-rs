# Edge / Cloud Pipeline Split — Rust Types and CLI Design

- **Date:** 2026-04-30
- **Principle:** The edge seals facts. The cloud interprets them.
- **Constraint:** Same `edgesentry-rs` library used at both edge and cloud. No separate codebase.
- **Overview:** [edge-cloud-split.md](edge-cloud-split.md)

---

## What changes and what stays

| Layer | Current state | After split |
|---|---|---|
| `eds compute run` | Produces `Measurement` JSONL (distance, TTC, zone) | **Unchanged** |
| `eds evaluate run` | Takes `Measurement` + full `rules.json` → `RiskEvent` (includes `rule_id`, `regulation`, `severity`) | Split: edge produces `MeasurementRecord`; cloud produces `EvaluatedRecord` |
| `eds explain run` | Takes `RiskEvent` → LLM explanation | Takes `EvaluatedRecord` → unchanged logic |
| `eds report generate` | Takes `RiskEvent` + `Assessment` | Takes `EvaluatedRecord` + `Assessment` → unchanged logic |
| `eds audit sign` | Signs any JSONL payload | **Unchanged** — used in both tiers |

---

## New types in `edgesentry-evaluate`

### `MeasurementRecord` — edge output

Produced by the edge when a threshold is breached. Contains only physical facts.
No rule name, no regulation text, no severity.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementRecord {
    pub breach_type: BreachType,      // which physics quantity breached
    pub measured_value: f32,          // actual measurement (metres, seconds, or 1.0 for zone)
    pub threshold: f32,               // the limit that was crossed
    pub entity_ids: Vec<String>,      // which entities were involved
    pub timestamp_ms: u64,            // event time — must be set at the edge
    pub profile_version: String,      // e.g. "sg-port-safety@2.1.0" — required for UC1 + UC5
    pub site_id: Option<String>,      // site UUID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreachType {
    Distance,   // Condition::DistanceLt
    Ttc,        // Condition::TtcLt
    Zone,       // Condition::ZoneMember
}
```

`breach_type` + `threshold` together uniquely identify a rule in `rules.json`,
allowing the cloud evaluator to look up `rule_id`, `regulation`, and `severity` without
any coordination between edge and cloud at runtime.

### `EvaluatedRecord` — cloud output

Produced by the cloud after receiving a `MeasurementRecord`. Adds the regulatory
interpretation. References the sealed `MeasurementRecord` by its BLAKE3 hash.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatedRecord {
    pub measurement_record_hash: [u8; 32],  // BLAKE3 of the sealed MeasurementRecord
    pub rule_id: String,                     // e.g. "PROXIMITY_ALERT"
    pub severity: Severity,                  // HIGH / CRITICAL / etc.
    pub regulation: String,                  // e.g. "MPA Port Safety Circular 2024-07 §3.1"
    pub site_id: Option<String>,
    pub timestamp_ms: u64,                   // copied from MeasurementRecord
}
```

`RiskEvent` (current type) is **kept as-is** for backward compatibility and for the
June submission demo, where the full pipeline runs at the edge. `EvaluatedRecord` is
the cloud-tier equivalent introduced in Phase 2.

---

## Profile split

The edge only needs threshold values. The cloud needs the full regulatory mapping.

### Edge profile (`params.toml` + `manifest.toml` only)

```toml
# crates/edgesentry-profile/fixtures/sg-port-safety/params.toml
[thresholds]
distance_m   = 5.0    # PROXIMITY_ALERT threshold
ttc_s        = 3.0    # TTC_ALERT threshold

[zones.exclusion_zone_a]
vertices = [[0, 0], [10, 0], [10, 10], [0, 10]]
```

The edge loads `params.toml` only. It knows "fire when distance < 5.0" but not
"this is rule PROXIMITY_ALERT citing MPA §3.1". The regulation lookup happens in the cloud.

### Cloud profile (full — `rules.json` + `kb/` + `params.toml`)

Unchanged. The cloud loads the full profile including `rules.json` and the regulatory
KB for LLM explanation.

---

## CLI design — edge tier

```
eds compute run   --input frames.jsonl                    --out measurements.jsonl
eds measure run   --input measurements.jsonl              \
                  --params sg-port-safety/params.toml     \
                  --profile-version sg-port-safety@2.1.0  \
                  --out breaches.jsonl
eds audit sign    --input breaches.jsonl                  \
                  --key $EDGE_KEY                         \
                  --state chain.state                     \
                  --out sealed_breaches.jsonl
```

`eds measure run` is a new command in the `eds measure` category (new subcommand group).
It replaces `eds evaluate run` at the edge. It:
- Reads `Measurement` JSONL from `eds compute`
- Compares against threshold values from `params.toml`
- Emits `MeasurementRecord` JSONL when a threshold is breached
- Does NOT load `rules.json` or the regulatory KB

The sealed `breaches.jsonl` (= `MeasurementRecord` + `AuditRecord`) is uploaded to R2.

### Operator alert

The operator alert fires inline during `eds measure run` before sealing:
```
stdout: BREACH distance 3.2m < 5.0m [FL-01, W-03] @ 14:23:07
```
or via webhook/MQTT (same `--alert-url` flag as currently supported in `eds evaluate`).

---

## CLI design — cloud tier

```
# Download sealed breaches from R2
eds r2 pull   --bucket maridb-edge --prefix measurements/sg-ms-0042/ --out sealed_breaches.jsonl

# Evaluate: MeasurementRecord + full profile → EvaluatedRecord
eds evaluate run  --input sealed_breaches.jsonl         \
                  --profile sg-port-safety/             \
                  --mode cloud                          \
                  --out evaluated.jsonl

# Sign EvaluatedRecord and append to chain
eds audit sign    --input evaluated.jsonl               \
                  --key $CLOUD_KEY                      \
                  --state cloud_chain.state             \
                  --out sealed_evaluated.jsonl

# Explain + report (unchanged)
eds explain run   --input evaluated.jsonl --n 5 --llm-url http://localhost:11434/v1 --out explanations.jsonl
eds report generate --events evaluated.jsonl --assessment assessment.jsonl --out report.pdf
```

`eds evaluate run --mode cloud` resolves `MeasurementRecord` → `EvaluatedRecord` by
matching `breach_type` + `threshold` to a rule in `rules.json`. A rule matches when:
- `BreachType::Distance` → `Condition::DistanceLt(t)` where `t == record.threshold`
- `BreachType::Ttc`      → `Condition::TtcLt(t)` where `t == record.threshold`
- `BreachType::Zone`     → `Condition::ZoneMember` (matched by entity_id in zone polygon)

If no rule matches, the cloud logs a warning and emits an `EvaluatedRecord` with
`rule_id: "UNMATCHED"`. This handles profile version drift gracefully.

---

## R2 upload/download — `eds r2`

New subcommand group for R2 object operations.

```
eds r2 push  --input FILE  --bucket BUCKET  --prefix PREFIX  [--immutable]
eds r2 pull  --bucket BUCKET  --prefix PREFIX  --out FILE
eds r2 list  --bucket BUCKET  --prefix PREFIX
```

Uses the Cloudflare R2 API (S3-compatible). Credentials from env:
`R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`.

`--immutable` sets the Object Lock header on upload (`x-amz-object-lock-mode: COMPLIANCE`).
Default: enabled for edge breach records, disabled for cloud evaluation records
(cloud records are still append-only via the hash chain, but Object Lock on the edge
records is the first line of tamper-evidence).

---

## Same library, two execution contexts

The Rust library `edgesentry-rs` is deployed identically at edge and cloud.
The execution context (edge vs. cloud) is determined by which CLI command is called:

| Context | Commands used | Profile loaded | Output type |
|---|---|---|---|
| Edge | `eds compute` → `eds measure` → `eds audit sign` → `eds r2 push` | `params.toml` only | `MeasurementRecord` |
| Cloud | `eds r2 pull` → `eds evaluate --mode cloud` → `eds audit sign` → `eds explain` → `eds report` | Full profile | `EvaluatedRecord` |

No feature flags, no conditional compilation, no separate binaries. The same `eds`
binary is installed at the edge and in the cloud runtime. The difference is which
subcommands are invoked and which profile files are available.

---

## June submission scope (unchanged)

The June demo runs the **full pipeline at the edge** using the existing commands:
```
eds ingest replay → eds compute → eds evaluate → eds assess → eds explain → eds report → eds audit sign
```

`RiskEvent` (current type) is used throughout. `MeasurementRecord` and `EvaluatedRecord`
are introduced **post-submission** as Phase 2 work.

The June demo is not broken by this plan — it uses a superset of what the edge tier
will eventually run. The architectural direction is established here; the migration
is tracked separately.

---

## Build order (Phase 2, post-submission)

| Order | Deliverable | Crate / CLI |
|---|---|---|
| 1 | `BreachType` enum + `MeasurementRecord` struct | `edgesentry-evaluate` |
| 2 | `EvaluatedRecord` struct | `edgesentry-evaluate` |
| 3 | `evaluate_edge()` function — threshold check only, no rule lookup | `edgesentry-evaluate` |
| 4 | `evaluate_cloud()` function — MeasurementRecord → EvaluatedRecord | `edgesentry-evaluate` |
| 5 | `eds measure run` CLI command | `eds` |
| 6 | `eds evaluate run --mode cloud` | `eds` (extend existing) |
| 7 | `eds r2 push / pull / list` | `eds` (new subcommand group) |
| 8 | Edge profile `params.toml` split from `rules.json` | `edgesentry-profile` |
| 9 | `--immutable` Object Lock flag in `eds r2 push` | `eds` |

---

## References

- `clarus-commercial/docs/submission/architecture-core.md` — tier diagram
- `clarus-commercial/docs/submission/audit-record-design.md` — MeasurementRecord + EvaluatedRecord field specs
- `crates/edgesentry-evaluate/src/rules.rs` — current `RiskEvent` and `evaluate()` implementation
- `_inputs/migration_roadmap.md` — Phase 1–3 crate plan (this is Phase 2+ work)
