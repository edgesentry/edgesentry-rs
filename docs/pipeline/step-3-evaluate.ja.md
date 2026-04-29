# ステップ 3 - 評価

プロファイル内のルールに対して計測値を比較します。各ルール違反は `RiskEvent` を生成します。

```
eds evaluate run --input <FILE> --profile <DIR> --out <FILE>
```

| フラグ | 説明 |
|------|-------------|
| `--input` | 入力 EntityFrame JSONL ファイル |
| `--profile` | `rules.json` を含むプロファイルディレクトリ |
| `--out` | 出力 RiskEvent JSONL ファイル |

## RiskEvent スキーマ

```json
{"eds_schema":"eds.risk-event","version":"0.1"}
{
  "rule_id": "PROXIMITY_ALERT",
  "severity": "HIGH",
  "regulation": "Site Safety Procedure §3.1",
  "entity_ids": ["FL-01", "W-03"],
  "measured_value": 3.0,
  "threshold": 5.0,
  "timestamp_ms": 6000
}
```

| フィールド | 型 | 説明 |
|-------|------|-------------|
| `rule_id` | 文字列 | `rules.json` のルールに一致する識別子 |
| `severity` | 列挙型 | `LOW`、`MEDIUM`、`HIGH`、または `CRITICAL` |
| `regulation` | 文字列 | プロファイルからの正確な規制条項 |
| `entity_ids` | 文字列[] | 関与するエンティティ（近接/TTC は2つ、ゾーンは1つ） |
| `measured_value` | 浮動小数点 | 閾値を超えた物理計測値 |
| `threshold` | 浮動小数点 | ルールからの閾値 |
| `timestamp_ms` | 整数 | フレームのタイムスタンプ |

## ルール条件タイプ

`rules.json` では3種類の条件タイプがサポートされています：

| 条件の構文 | 発火条件 |
|---|---|
| `distance < N` | 任意の2つのエンティティ間のユークリッド距離が N メートルを下回る |
| `ttc < N` | 接近中の任意の2つのエンティティ間の衝突時間が N 秒を下回る |
| `zone_member` | 任意のエンティティの位置が `zone` フィールドで定義された多角形の内側に入る |

完全な `rules.json` 形式については、[プロファイル作成](profile-authoring.ja.md)を参照してください。

## プロファイル管理

パイプラインを実行せずにプロファイルを検証・検査する：

```bash
# rules.json が有効で KB ファイルが存在することを確認
eds profile validate --profile crates/edgesentry-profile/fixtures/demo

# プロファイルで定義されたルール ID を一覧表示
eds profile list --profile crates/edgesentry-profile/fixtures/demo
```
