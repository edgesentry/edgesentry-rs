# EdgeSentry-RS — Toolkit Roadmap

EdgeSentry-RS is a Rust workspace providing a seven-step sensor-to-seal pipeline. Any domain that needs to capture real-world measurements, check them against rules, explain deviations in plain language, and produce a tamper-evident record fits the same pattern.

This document tracks the implementation status of the full toolkit and the planned
work ahead.

---

## Design principles

**1. Pipeline stages over monoliths**

Each step is a separate crate with a narrow responsibility. Stages are composed
by piping JSONL files between CLI commands — no shared in-process state required.
Applications choose which stages to use and where to run them.

**2. Deterministic core, non-deterministic periphery**

Steps 1–3 (Ingest, Compute, Evaluate) are deterministic and suitable for real-time
edge execution. Steps 4–7 (Assess, Explain, Document, Seal) may involve latency,
external services, or asynchronous scheduling and are designed for that context.

**3. Edge seals facts. Cloud interprets them.**

A tamper-evident measurement must be sealed at the moment it occurs, before any
regulatory interpretation is applied. The regulatory knowledge base (rule definitions,
regulation texts) lives in the cloud and is updated there. Edge devices need only
the threshold values required to fire operator alerts and seal raw measurements.
This means a regulation update takes effect once in the cloud — no field deployment
required. See [Edge / Cloud Pipeline Split](../pipeline/tier-architecture.md).

**4. Same binary, two execution contexts**

The `eds` binary is identical at the edge and in the cloud. The execution context
is determined by which subcommands are invoked and which profile files are present —
not by conditional compilation or separate builds.

**5. Open core**

The engine (physics, rule evaluation, LLM chain structure, audit format) is
Apache 2.0 / MIT. Regulatory profiles (rule datasets, knowledge bases, jurisdiction-
specific parameters) are separately licensed. The engine is trustworthy because it
is inspectable; the profiles are valuable because they require domain expertise.

---

## Seven-step pipeline — current status

| Step | Crate | CLI | Status |
|---|---|---|---|
| 1a — Structured ingest | `edgesentry-ingest` | `eds ingest replay` / `eds ingest stream` | ✅ Done |
| 1b — Unstructured parse | `edgesentry-parse` | `eds parse maritime` / `eds parse image` | ✅ CSV + Parquet done · 📋 ONNX vision stub |
| 2 — Compute | `edgesentry-compute` | `eds compute run` | ✅ Done |
| 3 — Evaluate | `edgesentry-evaluate` | `eds evaluate run` | ✅ Done |
| 4 — Assess | `edgesentry-assess` | `eds assess run` | ✅ Done |
| 5 — Explain | `edgesentry-explain` | `eds explain run` | ✅ Done |
| 6a — Safety report | `edgesentry-report` | `eds report generate` | ✅ Markdown + PDF done |
| 6b — Document compliance | `edgesentry-document` | `eds document fill / check / gen` | ✅ Done |
| 7 — Seal | `edgesentry-audit` | `eds audit sign / verify` | ✅ Done |
| 8 — Evidence export | `edgesentry-audit` | `eds audit export-aims` | ✅ Done ([#397](https://github.com/edgesentry/edgesentry-rs/issues/397)) |

**Supporting crates:**

| Crate | CLI | Purpose | Status |
|---|---|---|---|
| `edgesentry-profile` | `eds profile validate / list` | Load and validate rule profiles | ✅ Done |
| `edgesentry-store` | — | In-memory event store for daemon mode | ✅ Done |
| `edgesentry-scenario` | `eds scenario generate / simulate` | Synthetic CSV and UDP fixture generation | ✅ Done |
| `edgesentry-image-utils` | — | Shared image processing (ONNX / OpenCV, feature-gated) | 📋 Stub |
| `edgesentry-bridge` | — | C FFI bridge for embedded deployments | ✅ Done |

---

## Implemented profiles

| Profile | Location | Rules | KB entries |
|---|---|---|---|
| `demo` | `crates/edgesentry-profile/fixtures/demo/` | PROXIMITY_ALERT, EXCLUSION_ZONE_BREACH, TTC_ALERT | 3 |
| `sg-maritime-security` | `clarus-commercial/profiles/sg-maritime-security/` | RESTRICTED_ZONE_APPROACH, AIS_TRACK_GAP | 2 |
| `sg-port-compliance` | `clarus-commercial/profiles/sg-port-compliance/` | BWM_D2_EXPIRED, QUARANTINE_PRENOTIFICATION, DG_RESTRICTION, CREW_DOC_VALIDITY | 4 |

---

## Near-term work — before June 2026 deadline

These items are required to demonstrate the full end-to-end pipeline.

### P0 — Must have

| Issue | Deliverable | Why |
|---|---|---|
| [#299](https://github.com/edgesentry/edgesentry-rs/issues/299) | AIS NMEA 0183 input adapter — `eds ingest stream --source ais://` | Maritime security demo scenario (Tier 2) |
| [#300](https://github.com/edgesentry/edgesentry-rs/issues/300) | `eds audit sign-document` / `verify-document` — `DocumentAuditPayload` + PDF hash embed | Document audit trail (TC4 demo) |
| [#19](https://github.com/edgesentry/edgesentry-rs/issues/19) | Browser demo UI — `eds serve` split-screen with report generation and verification panel | Submission demo video |

### P1 — Should have

| Issue | Deliverable | Why |
|---|---|---|
| [#302](https://github.com/edgesentry/edgesentry-rs/issues/302) | Synthetic AIS `EntityStream` CSV fixture for `sg-maritime-security` demo | AIS adapter workaround before #299 ships |
| [#303](https://github.com/edgesentry/edgesentry-rs/issues/303) | ARM64 cross-compile CI — `aarch64-unknown-linux-gnu` build job | Validates edge deployment claim |
| [#18](https://github.com/edgesentry/edgesentry-rs/issues/18) | LLM runtime decision doc — Ollama vs llama.cpp vs MLX | Submission technical section |
| ~~[#301](https://github.com/edgesentry/edgesentry-rs/issues/301)~~ | ~~Confirm `eds parse maritime` uses CSV for MVP; defer Parquet~~ | ✅ Parquet implemented (#326) — `.parquet` auto-detected, same schema as CSV |

---

## Legal admissibility — `AuditRecord` hardening

Full requirements analysis: [docs/legal.md](../legal.md).
The two weakest points are the **trusted timestamp** and the missing **`software_version`** field.

### Before June 2026 submission (P1)

| Deliverable | Requirement addressed | Detail |
|---|---|---|
| Add `software_version: String` to `AuditRecord` | Requirement 6 — System integrity | Embed Git SHA at compile time via `env!("CARGO_PKG_VERSION")` + build metadata; satisfies Evidence Act s.116A "operating properly" |
| Add `hash_alg: String` and `sig_alg: String` to `AuditRecord` | Requirement 7 — Retention / format longevity | Pin algorithm identifiers in the record; enables independent verification after 10+ years |
| Document key registration process | Requirement 2 — Attribution | Public key → customer → edgesentry onboarding; stored with timestamp |
| Document R2 upload timestamp as trusted anchor | Requirement 3 — Trusted timestamp (Phase 1) | Cloudflare `x-amz-date` is operator-independent; establishes "sealed before incident" argument |

### Phase 2 — post-submission PoC (November 2026)

| Deliverable | Requirement addressed | Detail |
|---|---|---|
| RFC 3161 TSA integration | Requirement 3 — Trusted timestamp (Phase 2) | Submit record hash to TSA (DigiCert/GlobalSign) on signing; store token alongside `AuditRecord` |
| HSM / TPM key storage | Requirement 2 — Attribution (Phase 2) | Private key never extractable; satisfies CLS Level 4; tracked [#54](https://github.com/edgesentry/edgesentry-rs/issues/54) |
| Partial chain export format | Requirement 4 — Completeness | Anchor record + proof of connection to root for time-range exports |

### Before production / insurance partnership

| Deliverable | Detail |
|---|---|
| External legal opinion | Singapore maritime law firm review of Evidence Act s.116A compliance |
| P&I / H&M underwriter pilot | Confirm actual evidence requirements with one insurer |

---

## Medium-term work — edge / cloud split

**Architecture:** edge seals raw `MeasurementRecord`; cloud evaluates to `EvaluatedRecord`.
The full design is in [tier-implementation.md](../pipeline/tier-implementation.md).

### New types in `edgesentry-evaluate`

```
MeasurementRecord   ← edge output
  breach_type: BreachType    (Distance | Ttc | Zone)
  measured_value: f32
  threshold: f32
  entity_ids: Vec<String>
  timestamp_ms: u64
  profile_version: String    ← required: proves active threshold at time of event
  site_id: Option<String>

EvaluatedRecord     ← cloud output
  measurement_record_hash: [u8; 32]
  rule_id: String
  severity: Severity
  regulation: String
  site_id: Option<String>
  timestamp_ms: u64
```

`RiskEvent` (current) is kept for backward compatibility. Migration is additive.

### New CLI commands

```
# Edge tier
eds measure run  --input measurements.jsonl \
                 --params profile/params.toml \
                 --profile-version sg-port-safety@2.1.0 \
                 --out breaches.jsonl

# Cloud tier
eds evaluate run --input breaches.jsonl \
                 --profile full-profile/ \
                 --mode cloud \
                 --out evaluated.jsonl

# R2 transport
eds r2 push  --input FILE --bucket BUCKET --prefix PREFIX [--immutable]
eds r2 pull  --bucket BUCKET --prefix PREFIX --out FILE
```

### Profile split

| Profile component | Edge device | Cloud |
|---|---|---|
| `params.toml` — threshold values, zone geometry | ✅ Required | ✅ Required |
| `rules.json` — rule_id, condition, regulation, severity | ❌ Not deployed | ✅ Required |
| `kb/` — regulatory KB for LLM | ❌ Not deployed | ✅ Required |
| `manifest.toml` — version, jurisdiction | ✅ Required | ✅ Required |

### Build order (post-submission)

| Order | Deliverable |
|---|---|
| 1 | `BreachType` enum + `MeasurementRecord` struct in `edgesentry-evaluate` |
| 2 | `EvaluatedRecord` struct in `edgesentry-evaluate` |
| 3 | `evaluate_edge()` — threshold check only, no rule lookup |
| 4 | `evaluate_cloud()` — `MeasurementRecord` → `EvaluatedRecord` |
| 5 | `eds measure run` CLI |
| 6 | `eds evaluate run --mode cloud` |
| 7 | `eds r2 push / pull / list` |
| 8 | Edge profile `params.toml` split from `rules.json` in `edgesentry-profile` |
| 9 | R2 Object Lock (`--immutable`) flag |

---

## Long-term work — production hardening and expanded inputs

### Vision / camera input

| Issue | Deliverable |
|---|---|
| [#305](https://github.com/edgesentry/edgesentry-rs/issues/305) | `edgesentry-image-utils` ONNX object detection — USB/RTSP camera → `EntityStream` |
| [#304](https://github.com/edgesentry/edgesentry-rs/issues/304) | RTSP stream adapter — live IP camera input |

### Multi-source fan-in

| Issue | Deliverable |
|---|---|
| [#307](https://github.com/edgesentry/edgesentry-rs/issues/307) | Concurrent RTSP + AIS streams into one rule engine; `sensor_id` field in `Entity` / `RiskEvent` / `AuditRecord` |

### Heartbeat and operational records

| Issue | Deliverable |
|---|---|
| [#290](https://github.com/edgesentry/edgesentry-rs/issues/290) | Heartbeat `AuditRecord` emission every 5 min — zone summary, sensor status, pipeline latency |
| [#291](https://github.com/edgesentry/edgesentry-rs/issues/291) | `eds report monthly` — date-range filtered monthly safety report |

### Deployment and operations

| Issue | Deliverable |
|---|---|
| [#303](https://github.com/edgesentry/edgesentry-rs/issues/303) | ARM64 CI job (`aarch64-unknown-linux-gnu`) |
| [#306](https://github.com/edgesentry/edgesentry-rs/issues/306) | RPi e2e smoke test — `deploy/smoke-test.sh` |
| [#10](https://github.com/edgesentry/edgesentry-rs/issues/10) | Production `sg-port-safety` profile — expanded rules, `params.toml`, `manifest.toml` |
| [#28](https://github.com/edgesentry/edgesentry-rs/issues/28) | Publish crates via OIDC trusted publishing |
| [#30](https://github.com/edgesentry/edgesentry-rs/issues/30) | Slim release quality gate to locked build only |

---

## JSONL schema versioning

Every JSONL file produced by `eds` begins with a header record declaring the schema
name and version. This is the contract between pipeline stages.

```json
{"eds_schema": "EntityFrame",   "version": "1.0"}
{"eds_schema": "Measurement",   "version": "1.0"}
{"eds_schema": "RiskEvent",     "version": "1.0"}
{"eds_schema": "Assessment",    "version": "1.0"}
{"eds_schema": "Explanation",   "version": "1.0"}
{"eds_schema": "AuditRecord",   "version": "1.0"}
```

`MAJOR` version increments are breaking. `MINOR` increments are additive.
Files without a header record are treated as version-unknown and produce a warning.

---

## References

- `docs/pipeline/` — step-by-step pipeline documentation
- `docs/roadmap/strategy-compliance.md` — audit crate compliance roadmap (CLS / JC-STAR)
- `docs/roadmap/feature-inspect.md` — inspect crate roadmap (3D deviation, IFC)
- `docs/pipeline/tier-implementation.md` — edge/cloud split: Rust types, CLI design, profile split, build order
- `_inputs/mvp.md` — June 2026 submission scope and demo flows
- `_inputs/migration_roadmap.md` — Phase 1–3 crate migration history
