# ステップ 6 - 文書化

パイプラインの結果を人間が読める形式に変換します：安全監視パイプライン用の Markdown 安全レポート、またはドキュメントコンプライアンスパイプライン用の入力済みポート入港 HTML ドキュメント。

---

## 安全監視レポート

### eds report generate

```
eds report generate --events <FILE> --assessment <FILE> --out <FILE>
                    [--site-name <NAME>] [--period <STR>] [--chain-valid]
                    [--format md|pdf]
```

| フラグ | デフォルト | 説明 |
|------|---------|-------------|
| `--events` | | RiskEvent JSONL ファイル（`eds evaluate run` から） |
| `--assessment` | | Assessment JSONL ファイル（`eds assess run` から） |
| `--site-name` | | レポートヘッダに含まれるオプションのサイト名 |
| `--period` | | オプションのレポート期間文字列（例：`"April 2026"`） |
| `--chain-valid` | | 設定すると、レポートに「Chain integrity: PASS」行を追加 |
| `--format` | `md` | 出力形式：`md`（Markdown）または `pdf`（printpdf による A4 PDF） |
| `--out` | | 出力ファイルパス |

レポートには以下が含まれます：

- サマリーテーブル：重大度別のイベント数
- ルール別リスクイベントテーブル：ルール、件数、重大度、正確な規制引用
- エンティティ相関セクション（エンティティペアが複数のイベントに現れた場合）
- トレンド分析セクション：Stable / Rising / Falling と簡単な解釈

**PDF 出力**：`--format pdf` が指定された場合、出力はバイナリ A4 PDF（Helvetica フォント、printpdf 0.7）です。ファイルには `.pdf` 拡張子が必要です。任意の PDF ビューアで開くか、直接印刷できます。

### eds report validate

```
eds report validate --events <FILE> --assessment <FILE>
```

両方のファイルが空でなく解析可能であれば 0 で終了します。そうでなければ、エラーメッセージとともに非ゼロで終了します。レポート生成前の事前チェックとして使用します。

---

## ドキュメントコンプライアンス - 入力とレンダリング

### eds document fill

DocumentEntity フィールドをドキュメントテンプレートにマッピングし、欠損または低信頼度のフィールドにフラグを立てます。

```
eds document fill --input <FILE> --template <NAME> --out <FILE>
                  [--llm-url <URL>] [--confidence-threshold <FLOAT>]
```

| フラグ | デフォルト | 説明 |
|------|---------|-------------|
| `--input` | | DocumentEntity JSONL ファイル（`eds parse maritime` から） |
| `--template` | | テンプレート名：`fal-form-1`、`fal-form-5`、または `sg-port-entry` |
| `--llm-url` | | AI 導出フィールド用の LLM サーバー URL（オプション） |
| `--confidence-threshold` | 0.5 | この信頼度を下回るフィールドにフラグを立てる |
| `--out` | | 出力 FilledDocument JSONL ファイル |

出力スキーマ（`eds.filled-document`）：

```json
{"voyage_id":"V001","template":"fal-form-1","review_required":false,
 "fields":{
   "VESSEL_NAME":{"value":"MV Horizon","confidence":0.95,"source":"Direct","flagged":false},
   "CREW_COUNT":  {"value":"","confidence":0.0,"source":"Direct","flagged":true}
 }}
```

いずれかのフィールドにフラグが立つと `review_required: true` になります。フラグが立ったすべてのフィールドが人間のレビュアーによって解決されるまでエクスポートはブロックされます。

### eds document check

入力済みドキュメントフィールドをコンプライアンスルールセットと照合します。

```
eds document check --input <FILE> --profile <DIR> --out <FILE>
```

`<profile>/rules.json`（ドキュメントコンプライアンス形式）を読み込み、失敗した各ルールについて `ComplianceAlert` を出力します。

出力スキーマ（`eds.compliance-alert`）：

```json
{"rule_id":"BWM_D2_EXPIRED","severity":"HIGH","field":"bwm_certificate_expiry",
 "message":"Rule 'BWM_D2_EXPIRED' failed check 'not_expired' on field 'bwm_certificate_expiry'",
 "regulation":"Ballast Water Management Convention (BWM) D-2 Standard",
 "voyage_id":"V002"}
```

`HIGH` 重大度のアラートはドキュメントのエクスポートをブロックします。

### eds document gen

組み込みテンプレートを使用して入力済みドキュメントを HTML にレンダリングします。

```
eds document gen --input <FILE> --template <NAME> --out <FILE>
```

| テンプレート名 | フォーム |
|---|---|
| `fal-form-1` | FAL Form 1 - 一般申告書（IMO） |
| `fal-form-5` | FAL Form 5 - 乗組員名簿（IMO） |
| `sg-port-entry` | シンガポール MPA Port+ パッケージ |

テンプレートは `eds` バイナリに埋め込まれています。各 `{{FIELD_NAME}}` プレースホルダーは FilledDocument の対応するフィールド値で置き換えられます。出力はブラウザの印刷ダイアログから PDF に印刷できる自己完結型の HTML ファイルです。
