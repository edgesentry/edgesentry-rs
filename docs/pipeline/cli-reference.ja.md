# CLI リファレンス

フェーズ 1-3 で提供されたすべての `eds` サブコマンドです。バイナリから直接フラグの全一覧を確認するには `eds <command> --help` を実行してください。

## eds ingest

| サブコマンド | 必須フラグ | オプションフラグ |
|---|---|---|
| `replay` | `--source FILE` `--out FILE` | `--profile DIR` |
| `stream` | `--source udp://HOST:PORT` `--profile DIR` `--out FILE` | |

## eds parse

| サブコマンド | 必須フラグ | オプションフラグ |
|---|---|---|
| `maritime` | `--source FILE` `--out FILE` | |
| `document` | `--source FILE` `--out FILE` | |
| `form` | `--source FILE` `--out FILE` | |
| `image` | `--source FILE` `--out FILE` | （スタブ ── `onnx` フィーチャーが必要） |

## eds scenario

| サブコマンド | 必須フラグ | オプションフラグ |
|---|---|---|
| `generate` | `--out FILE` | `--entities N` `--frames N` `--seed N` `--scenario-type entity` |
| `simulate` | `--source FILE` `--target udp://HOST:PORT` | `--fps N` |

## eds compute

| サブコマンド | 必須フラグ | オプションフラグ |
|---|---|---|
| `run` | `--input FILE` `--out FILE` | |

## eds evaluate

| サブコマンド | 必須フラグ | オプションフラグ |
|---|---|---|
| `run` | `--input FILE` `--profile DIR` `--out FILE` | |

## eds profile

| サブコマンド | 必須フラグ | オプションフラグ |
|---|---|---|
| `validate` | `--profile DIR` | |
| `list` | `--profile DIR` | |

## eds assess

| サブコマンド | 必須フラグ | オプションフラグ |
|---|---|---|
| `run` | `--input FILE` `--out FILE` | `--history FILE`（繰り返し可能）`--window-sec N` |

## eds explain

| サブコマンド | 必須フラグ | オプションフラグ |
|---|---|---|
| `run` | `--input FILE` `--n N` `--out FILE` | `--pick severity\|time\|random` `--llm-url URL` `--model NAME` `--profile DIR` |

## eds report

| サブコマンド | 必須フラグ | オプションフラグ |
|---|---|---|
| `generate` | `--events FILE` `--assessment FILE` `--out FILE` | `--site-name STR` `--period STR` `--chain-valid` `--format md\|pdf` |
| `validate` | `--events FILE` `--assessment FILE` | |

## eds document

| サブコマンド | 必須フラグ | オプションフラグ |
|---|---|---|
| `fill` | `--input FILE` `--template NAME` `--out FILE` | `--llm-url URL` `--confidence-threshold FLOAT` |
| `check` | `--input FILE` `--profile DIR` `--out FILE` | |
| `gen` | `--input FILE` `--template NAME` `--out FILE` | |

## eds audit

| サブコマンド | 必須フラグ | オプションフラグ |
|---|---|---|
| `keygen` | | `--out FILE` |
| `inspect-key` | `--private-key-hex HEX` | `--out FILE` |
| `sign-record` | `--device-id ID` `--sequence N` `--timestamp-ms N` `--payload STR` `--object-ref STR` `--private-key-hex HEX` | `--prev-hash-hex HEX` `--out FILE` |
| `verify-record` | `--record-file FILE` `--public-key-hex HEX` | |
| `verify-chain` | `--records-file FILE` | |
| `demo-lift-inspection` | | `--device-id ID` `--private-key-hex HEX` `--start-timestamp-ms N` `--object-prefix STR` `--out-file FILE` `--payloads-file FILE` |
