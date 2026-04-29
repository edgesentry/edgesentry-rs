# Quickstart - Safety Monitoring

End-to-end walkthrough of the safety monitoring pipeline using the bundled forklift approach fixture.
The full automated version of this walkthrough is `scripts/local_demo.sh`.

## Prerequisites

```bash
cargo build -p eds
```

For Step 5 (Explain), an OpenAI-compatible LLM server must be running on `http://localhost:8080`.
All other steps work offline.

## Fixture

The demo uses a 10-frame scenario: forklift FL-01 approaches stationary worker W-03, and
forklift FL-02 enters an exclusion zone. The CSV is at:

```
crates/edgesentry-ingest/fixtures/forklift_approach.csv
```

Profile and knowledge base:

```
crates/edgesentry-profile/fixtures/demo/
  rules.json          -- PROXIMITY_ALERT, TTC_ALERT, EXCLUSION_ZONE_BREACH
  kb/
    PROXIMITY_ALERT.txt
    TTC_ALERT.txt
    EXCLUSION_ZONE_BREACH.txt
```

## Step 1 - Ingest

```bash
eds ingest replay \
  --source crates/edgesentry-ingest/fixtures/forklift_approach.csv \
  --profile crates/edgesentry-profile/fixtures/demo \
  --out /tmp/frames.jsonl
```

Expected output:

```
ingest replay: wrote 10 frame(s) to /tmp/frames.jsonl
```

`frames.jsonl` schema: `eds.entity-frame`. Each record is one timestamp snapshot of all entities.

## Step 2 - Compute

```bash
eds compute run --input /tmp/frames.jsonl --out /tmp/measurements.jsonl
```

Outputs pairwise distances, relative velocities, TTC values, and zone memberships for every
entity pair in every frame.

## Step 3 - Evaluate

```bash
eds evaluate run \
  --input /tmp/frames.jsonl \
  --profile crates/edgesentry-profile/fixtures/demo \
  --out /tmp/events.jsonl
```

Expected output:

```
evaluate run: 7 event(s) from 10 frame(s) written to /tmp/events.jsonl
```

The 7 events are: PROXIMITY_ALERT x4, TTC_ALERT x2, EXCLUSION_ZONE_BREACH x1.

Sample `events.jsonl` record:

```json
{"rule_id":"PROXIMITY_ALERT","severity":"HIGH","regulation":"Site Safety Procedure §3.1",
 "entity_ids":["FL-01","W-03"],"measured_value":3.0,"threshold":5.0,"timestamp_ms":6000}
```

## Step 4 - Assess

```bash
eds assess run --input /tmp/events.jsonl --out /tmp/assessment.jsonl
```

Expected output:

```
assess run: 7 event(s) analysed, 2 repeated rule(s), 1 correlated entity pair(s), trend=Rising
```

`assessment.jsonl` contains repeated rules, correlated entity pairs, and risk trend
(Stable / Rising / Falling).

## Step 5 - Explain (optional)

Requires a running llama-server or Ollama (OpenAI-compatible) on port 8080.

```bash
eds explain run \
  --input /tmp/events.jsonl \
  --n 2 \
  --pick severity \
  --profile crates/edgesentry-profile/fixtures/demo \
  --llm-url http://localhost:8080 \
  --out /tmp/explanations.jsonl
```

`--pick severity` selects the two highest-severity events. Each explanation is checked against
the KB snippet for the rule; `grounded: true` means the LLM cited a section reference present
in the KB.

## Step 6 - Report

```bash
eds report generate \
  --events /tmp/events.jsonl \
  --assessment /tmp/assessment.jsonl \
  --site-name "Demo Warehouse A" \
  --period "April 2026" \
  --out /tmp/report.md
```

`report.md` is a Markdown file with:
- Summary table (events by severity)
- Risk Events by Rule table (rule, count, severity, regulation citation)
- Entity Correlations section
- Trend Analysis section

## Step 7 - Seal

```bash
eds audit demo-lift-inspection \
  --device-id demo-edge-01 \
  --private-key-hex 0101010101010101010101010101010101010101010101010101010101010101 \
  --out-file /tmp/chain.json

eds audit verify-chain --records-file /tmp/chain.json
```

Expected verify output:

```
Chain verification passed: N records, all hashes and signatures valid.
```
