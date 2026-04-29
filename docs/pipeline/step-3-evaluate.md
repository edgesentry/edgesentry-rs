# Step 3 - Evaluate

Compare measurements against the rules in a profile. Each rule breach produces a `RiskEvent`.

```
eds evaluate run --input <FILE> --profile <DIR> --out <FILE>
```

| Flag | Description |
|------|-------------|
| `--input` | Input EntityFrame JSONL file |
| `--profile` | Profile directory containing `rules.json` |
| `--out` | Output RiskEvent JSONL file |

## RiskEvent schema

```json
{"eds_schema":"eds.risk-event","version":"0.1"}
{
  "rule_id": "PROXIMITY_ALERT",
  "severity": "HIGH",
  "regulation": "Site Safety Procedure §3.1",
  "entity_ids": ["FL-01", "W-03"],
  "measured_value": 3.0,
  "threshold": 5.0,
  "timestamp_ms": 6000
}
```

| Field | Type | Description |
|-------|------|-------------|
| `rule_id` | string | Identifier matching the rule in `rules.json` |
| `severity` | enum | `LOW`, `MEDIUM`, `HIGH`, or `CRITICAL` |
| `regulation` | string | Exact regulation clause from the profile |
| `entity_ids` | string[] | Entities involved (two for proximity/TTC, one for zone) |
| `measured_value` | float | The physics measurement that breached the threshold |
| `threshold` | float | The threshold value from the rule |
| `timestamp_ms` | integer | Frame timestamp |

## Rule condition types

Three condition types are supported in `rules.json`:

| Condition syntax | Fires when |
|---|---|
| `distance < N` | Euclidean distance between any two entities drops below N metres |
| `ttc < N` | Time-to-collision between any two approaching entities drops below N seconds |
| `zone_member` | Any entity's position falls inside the polygon defined by the `zone` field |

See [Profile Authoring](profile-authoring.md) for the full `rules.json` format.

## Profile management

Validate and inspect a profile without running the pipeline:

```bash
# Check that rules.json is valid and KB files are present
eds profile validate --profile crates/edgesentry-profile/fixtures/demo

# List rule IDs defined in the profile
eds profile list --profile crates/edgesentry-profile/fixtures/demo
```
