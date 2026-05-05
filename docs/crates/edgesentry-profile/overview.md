# edgesentry-profile

Profile loader and validator. A profile is a directory:

```
<profile-name>/
  rules.json      # rule definitions
  params.toml     # edge-deployable threshold values
  kb/             # plain-language descriptions per rule ID
```

## rules.json schema

```json
{
  "rules": [
    {
      "id": "RULE_NAME",
      "condition": "distance < threshold",
      "params": { "threshold": 5.0 },
      "regulation": "Exact citation §X",
      "severity": "HIGH"
    }
  ]
}
```

## Constraints
- `id`: SCREAMING_SNAKE_CASE, unique within profile
- `regulation`: appears verbatim in AuditRecords
- KB file must match `id` (case-sensitive)

## Built-in profiles
- `fixtures/demo/` — forklift approach (PROXIMITY_ALERT, TTC_ALERT, EXCLUSION_ZONE_BREACH)
- `fixtures/sg-port-safety/` — MPA Port Safety Circulars + MOM WSH
- `fixtures/sg-maritime-security/` — AIS gap + restricted zone (SOLAS V/19, IPA §18)
- `fixtures/sg-port-compliance/` — BWM D-2, FAL compliance
