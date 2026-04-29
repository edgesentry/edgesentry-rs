# Profile Authoring

A profile is a directory that tells the pipeline which rules to enforce and provides the
regulatory text used to ground LLM explanations.

## Directory layout

```
profiles/my-profile/
  rules.json        -- rule definitions (required)
  kb/
    RULE_ID.txt     -- one KB snippet per rule (required for eds explain run --profile)
```

## rules.json format

A JSON array of rule objects. Three condition types are supported:

```json
[
  {
    "rule_id": "PROXIMITY_ALERT",
    "condition": "distance < 5.0",
    "severity": "HIGH",
    "regulation": "Site Safety Procedure §3.1"
  },
  {
    "rule_id": "TTC_ALERT",
    "condition": "ttc < 3.0",
    "severity": "HIGH",
    "regulation": "Site Safety Procedure §3.2"
  },
  {
    "rule_id": "EXCLUSION_ZONE_BREACH",
    "condition": "zone_member",
    "severity": "CRITICAL",
    "regulation": "Site Safety Procedure §4.1",
    "zone": [[0,0],[10,0],[10,10],[0,10]]
  }
]
```

| Field | Required | Description |
|-------|----------|-------------|
| `rule_id` | yes | Unique identifier; must match the KB filename if grounding is used |
| `condition` | yes | `distance < N`, `ttc < N`, or `zone_member` |
| `severity` | yes | `LOW`, `MEDIUM`, `HIGH`, or `CRITICAL` |
| `regulation` | yes | Exact regulation clause cited in the RiskEvent |
| `zone` | for `zone_member` only | Polygon vertices as `[x, y]` pairs (metres, local coordinate system) |

## KB snippets

For each rule, create a plain-text file at `kb/<RULE_ID>.txt` containing the verbatim
regulatory text. The LLM uses this as the authoritative reference when generating explanations.
Grounding checks that the LLM cites a section reference (e.g. `§3.1`) present in the snippet.

Example (`kb/TTC_ALERT.txt`):

```
Site Safety Procedure §3.2 -- Time-to-Collision Emergency Stop

When the projected time-to-collision (TTC) between a powered industrial truck and any
person or stationary obstacle drops below 3 seconds, the operator must initiate an
emergency stop immediately.

TTC is computed as: TTC = current_distance / closing_speed

...
```

## Document compliance rules format

For the document compliance pipeline (`eds document check`), `rules.json` uses a different schema --
it operates on document fields rather than physics measurements:

```json
[
  {
    "rule_id": "BWM_D2_EXPIRED",
    "field": "bwm_certificate_expiry",
    "check": "not_expired",
    "severity": "HIGH",
    "regulation": "Ballast Water Management Convention (BWM) D-2 Standard"
  },
  {
    "rule_id": "DG_RESTRICTION",
    "field": "dangerous_goods",
    "check": "not_true",
    "severity": "HIGH",
    "regulation": "IMDG Code -- Dangerous Goods require prior MPA approval"
  }
]
```

| Check type | Fires when |
|---|---|
| `not_expired` | Field value is a date (YYYY-MM-DD) that is before the current demo date |
| `not_null` | Field is absent, empty, or flagged with confidence 0.0 |
| `not_true` | Boolean field value is `"true"` |

## Validation

```bash
eds profile validate --profile profiles/my-profile
eds profile list     --profile profiles/my-profile
```

`validate` checks that `rules.json` parses correctly and all condition strings are valid.
`list` prints the rule IDs defined in the profile.

## Bundled profiles

| Profile path | Domain | Rules |
|---|---|---|
| `crates/edgesentry-profile/fixtures/demo` | Warehouse safety | PROXIMITY_ALERT, TTC_ALERT, EXCLUSION_ZONE_BREACH |
| `crates/edgesentry-profile/fixtures/sg-port-compliance` | Singapore port compliance | BWM_D2_EXPIRED, QUARANTINE_PRENOTIFICATION, DG_RESTRICTION, CREW_DOC_VALIDITY |
| `crates/edgesentry-profile/fixtures/sg-maritime-security` | Maritime security | RESTRICTED_ZONE_APPROACH, AIS_TRACK_GAP |
