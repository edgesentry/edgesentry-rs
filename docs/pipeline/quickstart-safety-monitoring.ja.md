# クイックスタート - 安全監視

付属のフォークリフト接近フィクスチャを使用した安全監視パイプラインのエンドツーエンドウォークスルーです。
このウォークスルーの完全な自動化バージョンは `scripts/run_local_demo.sh` です。

## 前提条件

```bash
cargo build -p eds
```

ステップ 5（説明）では、`http://localhost:8080` で OpenAI 互換の LLM サーバーが実行されている必要があります。
その他のすべてのステップはオフラインで動作します。

## フィクスチャ

デモは10フレームのシナリオを使用します：フォークリフト FL-01 が静止した作業者 W-03 に接近し、
フォークリフト FL-02 が排除ゾーンに進入します。CSV ファイルの場所：

```
crates/edgesentry-ingest/fixtures/forklift_approach.csv
```

プロファイルとナレッジベース：

```
crates/edgesentry-profile/fixtures/demo/
  rules.json          -- PROXIMITY_ALERT, TTC_ALERT, EXCLUSION_ZONE_BREACH
  kb/
    PROXIMITY_ALERT.txt
    TTC_ALERT.txt
    EXCLUSION_ZONE_BREACH.txt
```

## ステップ 1 - 取込

```bash
eds ingest replay \
  --source crates/edgesentry-ingest/fixtures/forklift_approach.csv \
  --profile crates/edgesentry-profile/fixtures/demo \
  --out /tmp/frames.jsonl
```

期待される出力：

```
ingest replay: wrote 10 frame(s) to /tmp/frames.jsonl
```

`frames.jsonl` スキーマ：`eds.entity-frame`。各レコードはすべてのエンティティの1タイムスタンプスナップショットです。

## ステップ 2 - 計算

```bash
eds compute run --input /tmp/frames.jsonl --out /tmp/measurements.jsonl
```

すべてのフレーム内のすべてのエンティティペアについて、ペアワイズ距離、相対速度、TTC 値、およびゾーンメンバーシップを出力します。

## ステップ 3 - 評価

```bash
eds evaluate run \
  --input /tmp/frames.jsonl \
  --profile crates/edgesentry-profile/fixtures/demo \
  --out /tmp/events.jsonl
```

期待される出力：

```
evaluate run: 7 event(s) from 10 frame(s) written to /tmp/events.jsonl
```

7つのイベントは：PROXIMITY_ALERT x4、TTC_ALERT x2、EXCLUSION_ZONE_BREACH x1 です。

`events.jsonl` レコードのサンプル：

```json
{"rule_id":"PROXIMITY_ALERT","severity":"HIGH","regulation":"Site Safety Procedure §3.1",
 "entity_ids":["FL-01","W-03"],"measured_value":3.0,"threshold":5.0,"timestamp_ms":6000}
```

## ステップ 4 - 分析

```bash
eds assess run --input /tmp/events.jsonl --out /tmp/assessment.jsonl
```

期待される出力：

```
assess run: 7 event(s) analysed, 2 repeated rule(s), 1 correlated entity pair(s), trend=Rising
```

`assessment.jsonl` には繰り返しルール、相関エンティティペア、およびリスクトレンド（Stable / Rising / Falling）が含まれます。

## ステップ 5 - 説明（オプション）

ポート 8080 で実行中の llama-server または Ollama（OpenAI 互換）が必要です。

```bash
eds explain run \
  --input /tmp/events.jsonl \
  --n 2 \
  --pick severity \
  --profile crates/edgesentry-profile/fixtures/demo \
  --llm-url http://localhost:8080 \
  --out /tmp/explanations.jsonl
```

`--pick severity` は最も重大度の高い2つのイベントを選択します。各説明はそのルールの KB スニペットと照合されます。`grounded: true` は LLM が KB 内に存在するセクション参照を引用したことを意味します。

## ステップ 6 - レポート

```bash
eds report generate \
  --events /tmp/events.jsonl \
  --assessment /tmp/assessment.jsonl \
  --site-name "Demo Warehouse A" \
  --period "April 2026" \
  --out /tmp/report.md
```

`report.md` は以下を含む Markdown ファイルです：
- サマリーテーブル（重大度別イベント数）
- ルール別リスクイベントテーブル（ルール、件数、重大度、規制引用）
- エンティティ相関セクション
- トレンド分析セクション

## ステップ 7 - 封印

```bash
eds audit demo-lift-inspection \
  --device-id demo-edge-01 \
  --private-key-hex 0101010101010101010101010101010101010101010101010101010101010101 \
  --out-file /tmp/chain.json

eds audit verify-chain --records-file /tmp/chain.json
```

期待される検証出力：

```
Chain verification passed: N records, all hashes and signatures valid.
```
