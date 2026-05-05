# edgesentry-evaluate

Rule DSL and evaluation engine.

## Input → Output
`MeasurementFrame` JSONL + loaded `Profile` → `RiskEvent` JSONL (`eds.risk-event`)

## RiskEvent fields
- `rule_id` — matches `id` in `rules.json`
- `regulation` — verbatim from profile
- `severity` — `HIGH` / `MEDIUM` / `LOW`
- `evidence_quality` — `EvidenceQuality` derived from entity confidence
- `entities` — involved entity IDs
- `measured_value` — the value that triggered the rule
- `timestamp_ms`

## Evidence quality mapping
`compute_entity_confidence` → if ≥ 0.8: `CERTIFIED`, ≥ 0.5: `DEGRADED`, else: `NOT_APPLICABLE`.
Simulation entities always `NOT_APPLICABLE`.
