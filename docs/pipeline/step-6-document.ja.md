# Step 6 - Document

Format pipeline results into human-readable outputs: a Markdown safety report for the
safety monitoring pipeline, or a filled port-entry HTML document for the document compliance pipeline.

---

## Safety monitoring report

### eds report generate

```
eds report generate --events <FILE> --assessment <FILE> --out <FILE>
                    [--site-name <NAME>] [--period <STR>] [--chain-valid]
```

| Flag | Description |
|------|-------------|
| `--events` | RiskEvent JSONL file (from `eds evaluate run`) |
| `--assessment` | Assessment JSONL file (from `eds assess run`) |
| `--site-name` | Optional site name included in the report header |
| `--period` | Optional reporting period string, e.g. `"April 2026"` |
| `--chain-valid` | If set, adds a "Chain integrity: PASS" line to the report |
| `--out` | Output Markdown file |

The report contains:

- Summary table: event count broken down by severity
- Risk Events by Rule table: rule, count, severity, exact regulation citation
- Entity Correlations section (if any entity pair appeared in multiple events)
- Trend Analysis section: Stable / Rising / Falling with a brief interpretation

### eds report validate

```
eds report validate --events <FILE> --assessment <FILE>
```

Exits 0 if both files are non-empty and parseable. Exits non-zero with an error message
otherwise. Use as a pre-flight check before generating a report.

---

## Document compliance - fill and render

### eds document fill

Map DocumentEntity fields to a document template and flag any missing or low-confidence fields.

```
eds document fill --input <FILE> --template <NAME> --out <FILE>
                  [--llm-url <URL>] [--confidence-threshold <FLOAT>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--input` | | DocumentEntity JSONL file (from `eds parse maritime`) |
| `--template` | | Template name: `fal-form-1`, `fal-form-5`, or `sg-port-entry` |
| `--llm-url` | | LLM server URL for AI-derived fields (optional) |
| `--confidence-threshold` | 0.5 | Fields below this confidence are flagged |
| `--out` | | Output FilledDocument JSONL file |

Output schema (`eds.filled-document`):

```json
{"voyage_id":"V001","template":"fal-form-1","review_required":false,
 "fields":{
   "VESSEL_NAME":{"value":"MV Horizon","confidence":0.95,"source":"Direct","flagged":false},
   "CREW_COUNT":  {"value":"","confidence":0.0,"source":"Direct","flagged":true}
 }}
```

`review_required: true` if any field is flagged. Export is blocked until all flagged fields
are resolved by a human reviewer.

### eds document check

Check filled document fields against a compliance rule set.

```
eds document check --input <FILE> --profile <DIR> --out <FILE>
```

Loads `<profile>/rules.json` (document compliance format) and emits a `ComplianceAlert` for
each rule that fails.

Output schema (`eds.compliance-alert`):

```json
{"rule_id":"BWM_D2_EXPIRED","severity":"HIGH","field":"bwm_certificate_expiry",
 "message":"Rule 'BWM_D2_EXPIRED' failed check 'not_expired' on field 'bwm_certificate_expiry'",
 "regulation":"Ballast Water Management Convention (BWM) D-2 Standard",
 "voyage_id":"V002"}
```

A `HIGH` severity alert blocks document export.

### eds document gen

Render a filled document into HTML using an embedded template.

```
eds document gen --input <FILE> --template <NAME> --out <FILE>
```

| Template name | Form |
|---|---|
| `fal-form-1` | FAL Form 1 - General Declaration (IMO) |
| `fal-form-5` | FAL Form 5 - Crew List (IMO) |
| `sg-port-entry` | Singapore MPA Port+ package |

Templates are embedded in the `eds` binary. Each `{{FIELD_NAME}}` placeholder is replaced
with the corresponding field value from the FilledDocument. The output is a self-contained
HTML file that can be printed to PDF via a browser print dialog.
