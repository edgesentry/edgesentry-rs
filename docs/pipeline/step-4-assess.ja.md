# ステップ 4 - 分析

RiskEvent を時系列で相関分析し、パターンを浮かび上がらせます：繰り返しルール、複数のイベントに関与するエンティティペア、および上昇または下降のリスクトレンド。

```
eds assess run --input <FILE> --out <FILE> [--history <FILE>...] [--window-sec <N>]
```

| フラグ | 説明 |
|------|-------------|
| `--input` | 入力 RiskEvent JSONL ファイル（現在のウィンドウ） |
| `--out` | 出力 Assessment JSONL ファイル |
| `--history` | 入力とマージする追加の RiskEvent JSONL ファイル（繰り返し可能） |
| `--window-sec` | 最新イベントからこの秒数以内のイベントに分析を制限 |

## Assessment スキーマ

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

| フィールド | 説明 |
|-------|-------------|
| `repeated_rules` | ウィンドウ内で複数回発火したルール、件数の降順でソート |
| `correlated_entities` | 複数のイベントに関与したエンティティセット、event_count の降順でソート |
| `trend` | `Stable`、`Rising`、または `Falling` ── 以下のアルゴリズムを参照 |
| `event_count` | ウィンドウフィルタリング後に分析されたイベントの合計数 |

## トレンドアルゴリズム

ウィンドウ内のイベントはタイムスタンプで前半と後半に分割されます。各半分のイベントレート（ミリ秒あたりのイベント数）が比較されます：

- レート比 > 1.2 -- **Rising**
- レート比 < 0.8 -- **Falling**
- それ以外 -- **Stable**

4イベント未満の場合は常に `Stable` になります。

## 履歴ファイルの使用

複数の再生セッションまたはログファイルにわたってトレンドを追跡するには：

```bash
eds assess run \
  --input /tmp/events_today.jsonl \
  --history /tmp/events_yesterday.jsonl \
  --window-sec 3600 \
  --out /tmp/assessment.jsonl
```

すべてのファイルは分析前にタイムスタンプでマージおよびソートされます。`--window-sec` はマージ後に適用されます。
