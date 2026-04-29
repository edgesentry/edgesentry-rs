# プロファイル作成

プロファイルは、パイプラインに適用するルールを指示し、LLM の説明を根拠付けるために使用される規制テキストを提供するディレクトリです。

## ディレクトリ構造

```
profiles/my-profile/
  rules.json        -- ルール定義（必須）
  kb/
    RULE_ID.txt     -- ルールごとに1つの KB スニペット（eds explain run --profile に必須）
```

## rules.json 形式

ルールオブジェクトの JSON 配列です。3種類の条件タイプがサポートされています：

```json
[
  {
    "rule_id": "PROXIMITY_ALERT",
    "condition": "distance < 5.0",
    "severity": "HIGH",
    "regulation": "Site Safety Procedure §3.1"
  },
  {
    "rule_id": "TTC_ALERT",
    "condition": "ttc < 3.0",
    "severity": "HIGH",
    "regulation": "Site Safety Procedure §3.2"
  },
  {
    "rule_id": "EXCLUSION_ZONE_BREACH",
    "condition": "zone_member",
    "severity": "CRITICAL",
    "regulation": "Site Safety Procedure §4.1",
    "zone": [[0,0],[10,0],[10,10],[0,10]]
  }
]
```

| フィールド | 必須 | 説明 |
|-------|----------|-------------|
| `rule_id` | はい | 一意の識別子；根拠付けを使用する場合は KB ファイル名と一致する必要がある |
| `condition` | はい | `distance < N`、`ttc < N`、または `zone_member` |
| `severity` | はい | `LOW`、`MEDIUM`、`HIGH`、または `CRITICAL` |
| `regulation` | はい | RiskEvent に引用される正確な規制条項 |
| `zone` | `zone_member` のみ | `[x, y]` ペアとしての多角形の頂点（メートル、ローカル座標系） |

## KB スニペット

各ルールについて、`kb/<RULE_ID>.txt` に逐語的な規制テキストを含むプレーンテキストファイルを作成してください。LLM は説明を生成する際にこれを権威ある参照として使用します。根拠付けは LLM がスニペット内に存在するセクション参照（例：`§3.1`）を引用していることを確認します。

例（`kb/TTC_ALERT.txt`）：

```
Site Safety Procedure §3.2 -- Time-to-Collision Emergency Stop

When the projected time-to-collision (TTC) between a powered industrial truck and any
person or stationary obstacle drops below 3 seconds, the operator must initiate an
emergency stop immediately.

TTC is computed as: TTC = current_distance / closing_speed

...
```

## ドキュメントコンプライアンスのルール形式

ドキュメントコンプライアンスパイプライン（`eds document check`）では、`rules.json` は異なるスキーマを使用します ──
物理計測値ではなくドキュメントフィールドで動作します：

```json
[
  {
    "rule_id": "BWM_D2_EXPIRED",
    "field": "bwm_certificate_expiry",
    "check": "not_expired",
    "severity": "HIGH",
    "regulation": "Ballast Water Management Convention (BWM) D-2 Standard"
  },
  {
    "rule_id": "DG_RESTRICTION",
    "field": "dangerous_goods",
    "check": "not_true",
    "severity": "HIGH",
    "regulation": "IMDG Code -- Dangerous Goods require prior MPA approval"
  }
]
```

| チェックタイプ | 発火条件 |
|---|---|
| `not_expired` | フィールド値が現在のデモ日付より前の日付（YYYY-MM-DD）である |
| `not_null` | フィールドが存在しない、空、または信頼度 0.0 でフラグが立っている |
| `not_true` | ブール型フィールドの値が `"true"` である |

## 検証

```bash
eds profile validate --profile profiles/my-profile
eds profile list     --profile profiles/my-profile
```

`validate` は `rules.json` が正しく解析され、すべての条件文字列が有効であることを確認します。
`list` はプロファイルで定義されたルール ID を表示します。

## バンドルされたプロファイル

| プロファイルパス | ドメイン | ルール |
|---|---|---|
| `crates/edgesentry-profile/fixtures/demo` | 倉庫安全 | PROXIMITY_ALERT, TTC_ALERT, EXCLUSION_ZONE_BREACH |
| `crates/edgesentry-profile/fixtures/sg-port-compliance` | シンガポール港コンプライアンス | BWM_D2_EXPIRED, QUARANTINE_PRENOTIFICATION, DG_RESTRICTION, CREW_DOC_VALIDITY |
| `crates/edgesentry-profile/fixtures/sg-maritime-security` | 海事セキュリティ | RESTRICTED_ZONE_APPROACH, AIS_TRACK_GAP |
