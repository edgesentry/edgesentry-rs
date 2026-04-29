# CLI Reference

All `eds` subcommands delivered in Phases 1-3. Run `eds <command> --help` for the full
flag list directly from the binary.

## eds ingest

| Subcommand | Required flags | Optional flags |
|---|---|---|
| `replay` | `--source FILE` `--out FILE` | `--profile DIR` |
| `stream` | `--source udp://HOST:PORT` `--profile DIR` `--out FILE` | |

## eds parse

| Subcommand | Required flags | Optional flags |
|---|---|---|
| `maritime` | `--source FILE` `--out FILE` | |

## eds compute

| Subcommand | Required flags | Optional flags |
|---|---|---|
| `run` | `--input FILE` `--out FILE` | |

## eds evaluate

| Subcommand | Required flags | Optional flags |
|---|---|---|
| `run` | `--input FILE` `--profile DIR` `--out FILE` | |

## eds profile

| Subcommand | Required flags | Optional flags |
|---|---|---|
| `validate` | `--profile DIR` | |
| `list` | `--profile DIR` | |

## eds assess

| Subcommand | Required flags | Optional flags |
|---|---|---|
| `run` | `--input FILE` `--out FILE` | `--history FILE` (repeatable) `--window-sec N` |

## eds explain

| Subcommand | Required flags | Optional flags |
|---|---|---|
| `run` | `--input FILE` `--n N` `--out FILE` | `--pick severity\|time\|random` `--llm-url URL` `--model NAME` `--profile DIR` |

## eds report

| Subcommand | Required flags | Optional flags |
|---|---|---|
| `generate` | `--events FILE` `--assessment FILE` `--out FILE` | `--site-name STR` `--period STR` `--chain-valid` |
| `validate` | `--events FILE` `--assessment FILE` | |

## eds document

| Subcommand | Required flags | Optional flags |
|---|---|---|
| `fill` | `--input FILE` `--template NAME` `--out FILE` | `--llm-url URL` `--confidence-threshold FLOAT` |
| `check` | `--input FILE` `--profile DIR` `--out FILE` | |
| `gen` | `--input FILE` `--template NAME` `--out FILE` | |

## eds audit

| Subcommand | Required flags | Optional flags |
|---|---|---|
| `keygen` | | `--out FILE` |
| `inspect-key` | `--private-key-hex HEX` | `--out FILE` |
| `sign-record` | `--device-id ID` `--sequence N` `--timestamp-ms N` `--payload STR` `--object-ref STR` `--private-key-hex HEX` | `--prev-hash-hex HEX` `--out FILE` |
| `verify-record` | `--record-file FILE` `--public-key-hex HEX` | |
| `verify-chain` | `--records-file FILE` | |
| `demo-lift-inspection` | | `--device-id ID` `--private-key-hex HEX` `--start-timestamp-ms N` `--object-prefix STR` `--out-file FILE` `--payloads-file FILE` |
