# Step 4 - Assess

Correlate RiskEvents across time to surface patterns: repeated rules, entity pairs involved in
multiple events, and rising or falling risk trends.

```
eds assess run --input <FILE> --out <FILE> [--history <FILE>...] [--window-sec <N>]
```

| Flag | Description |
|------|-------------|
| `--input` | Input RiskEvent JSONL file (current window) |
| `--out` | Output Assessment JSONL file |
| `--history` | Additional RiskEvent JSONL files to merge with input (repeatable) |
| `--window-sec` | Restrict analysis to events within this many seconds of the newest event |

## Assessment schema

```json
{"eds_schema":"eds.assessment","version":"0.1"}
{
  "timestamp_ms": 9000,
  "event_count": 7,
  "trend": "Rising",
  "repeated_rules": [
    {"rule_id": "PROXIMITY_ALERT", "count": 4, "severity": "HIGH"},
    {"rule_id": "TTC_ALERT",       "count": 2, "severity": "HIGH"}
  ],
  "correlated_entities": [
    {"entity_ids": ["FL-01", "W-03"], "event_count": 6}
  ]
}
```

| Field | Description |
|-------|-------------|
| `repeated_rules` | Rules that fired more than once in the window, sorted by count descending |
| `correlated_entities` | Entity sets involved in more than one event, sorted by event_count descending |
| `trend` | `Stable`, `Rising`, or `Falling` -- see algorithm below |
| `event_count` | Total events analysed after window filtering |

## Trend algorithm

Events in the window are split into two halves by timestamp. The event rate (events per
millisecond) of each half is compared:

- Rate ratio > 1.2 -- **Rising**
- Rate ratio < 0.8 -- **Falling**
- Otherwise -- **Stable**

Fewer than 4 events always produces `Stable`.

## Using history files

To trend across multiple replay sessions or log files:

```bash
eds assess run \
  --input /tmp/events_today.jsonl \
  --history /tmp/events_yesterday.jsonl \
  --window-sec 3600 \
  --out /tmp/assessment.jsonl
```

All files are merged and sorted by timestamp before analysis. `--window-sec` is applied
after merging.
