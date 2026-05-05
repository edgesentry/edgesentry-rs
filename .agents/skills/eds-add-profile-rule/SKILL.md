---
name: eds-add-profile-rule
description: Add a new rule to an edgesentry-rs safety profile. Use when implementing a new regulation-backed detection rule (PROXIMITY_ALERT, ZONE_ENTRY, etc.).
license: Apache-2.0
compatibility: Requires Rust stable toolchain
metadata:
  repo: edgesentry-rs
---

## Steps

**1. Add the rule to `rules.json`**

Open `crates/edgesentry-profile/fixtures/<profile-name>/rules.json` and add:

```json
{
  "id": "RULE_NAME",
  "condition": "distance < threshold",
  "params": { "threshold": 5.0 },
  "regulation": "Exact regulation citation §X",
  "severity": "HIGH"
}
```

**2. Add a KB file**

Create `crates/edgesentry-profile/fixtures/<profile-name>/kb/RULE_NAME.md` with the plain-language description used by `edgesentry-explain`.

**3. Verify the profile loads**

```bash
cargo test -p edgesentry-profile
```

**4. Add an integration test**

In `crates/eds/tests/cli_integration.rs`, add a test that:
- runs `eds ingest replay` + `eds evaluate run` against a fixture
- asserts the new `RiskEvent` fires at the expected frame
- asserts `evidence_quality` is `CERTIFIED`

## Constraints

- `id` must be SCREAMING_SNAKE_CASE and unique within the profile
- `regulation` appears verbatim in AuditRecords — use the exact clause text
- KB filename must match `id` exactly (case-sensitive)
