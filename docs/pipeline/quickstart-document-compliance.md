# Quickstart - Document Compliance

End-to-end walkthrough of the document compliance pipeline using the bundled voyage fixtures.
Three test cases are covered: TC1 (clean pass), TC2 (BWM certificate expired), TC3 (low confidence).

## Prerequisites

```bash
cargo build -p eds
```

No LLM server required. All steps work offline.

## Fixtures

```
crates/edgesentry-document/fixtures/
  voyage_V001_compliant.csv      -- TC1: clean vessel         (CSV — human-authored fixture)
  voyage_V002_bwm_expired.csv    -- TC2: BWM D-2 expired      (CSV — human-authored fixture)
  voyage_V003_low_confidence.csv -- TC3: missing fields        (CSV — human-authored fixture)
  # Production: maridb writes .parquet — same column schema, auto-detected by eds parse maritime

clarus-commercial/profiles/sg-port-compliance/
  rules.json                     -- BWM_D2_EXPIRED, QUARANTINE_PRENOTIFICATION,
                                 --   DG_RESTRICTION, CREW_DOC_VALIDITY
  kb/
    BWM_D2_EXPIRED.txt
    QUARANTINE_PRENOTIFICATION.txt
    DG_RESTRICTION.txt
    CREW_DOC_VALIDITY.txt
```

## TC1 - Compliant voyage

```bash
# Step 1 - Ingest (parse maritime — CSV fixture)
eds parse maritime \
  --source crates/edgesentry-document/fixtures/voyage_V001_compliant.csv \
  --out /tmp/entity.jsonl

# Step 3 - Evaluate (fill document fields)
eds document fill \
  --input /tmp/entity.jsonl \
  --template fal-form-1 \
  --out /tmp/filled.jsonl
# review_required: false -- all fields confidence 0.95

# Step 3 cont. - Check compliance rules
eds document check \
  --input /tmp/filled.jsonl \
  --profile clarus-commercial/profiles/sg-port-compliance \
  --out /tmp/alerts.jsonl
# 0 compliance alerts

# Step 6 - Document (render HTML)
eds document gen \
  --input /tmp/filled.jsonl \
  --template fal-form-1 \
  --out /tmp/fal-form-1.html
```

Open `fal-form-1.html` in a browser to review the filled FAL Form 1. Print to PDF via the
browser print dialog.

## TC2 - BWM certificate expired

```bash
eds parse maritime \
  --source crates/edgesentry-document/fixtures/voyage_V002_bwm_expired.csv \
  --out /tmp/entity_v002.jsonl

eds document fill \
  --input /tmp/entity_v002.jsonl \
  --template fal-form-1 \
  --out /tmp/filled_v002.jsonl

eds document check \
  --input /tmp/filled_v002.jsonl \
  --profile clarus-commercial/profiles/sg-port-compliance \
  --out /tmp/alerts_v002.jsonl
```

Expected alert in `alerts_v002.jsonl`:

```json
{"rule_id":"BWM_D2_EXPIRED","severity":"HIGH","field":"bwm_certificate_expiry",
 "message":"Rule 'BWM_D2_EXPIRED' failed check 'not_expired' on field 'bwm_certificate_expiry'",
 "regulation":"Ballast Water Management Convention (BWM) D-2 Standard -- MPA Port Marine Circular No. 19 of 2023",
 "voyage_id":"V002"}
```

A HIGH severity alert blocks export. The vessel cannot proceed until the BWM certificate is renewed.

## TC3 - Low confidence (missing fields)

```bash
eds parse maritime \
  --source crates/edgesentry-document/fixtures/voyage_V003_low_confidence.csv \
  --out /tmp/entity_v003.jsonl

eds document fill \
  --input /tmp/entity_v003.jsonl \
  --template fal-form-1 \
  --out /tmp/filled_v003.jsonl
```

`filled_v003.jsonl` will have `review_required: true`. The fields `CREW_COUNT` and `CARGO_HS_CODE`
are missing from the source CSV and receive `confidence: 0.0, flagged: true`. A human reviewer
must supply the correct values before the document can be submitted.

## Available templates

| Template name | Form |
|---|---|
| `fal-form-1` | FAL Form 1 - General Declaration (IMO) |
| `fal-form-5` | FAL Form 5 - Crew List (IMO) |
| `sg-port-entry` | Singapore MPA Port+ package |
