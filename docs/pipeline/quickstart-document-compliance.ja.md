# クイックスタート - ドキュメントコンプライアンス

付属の航海フィクスチャを使用したドキュメントコンプライアンスパイプラインのエンドツーエンドウォークスルーです。
3つのテストケースがカバーされています：TC1（正常通過）、TC2（BWM 証明書期限切れ）、TC3（低信頼度）。

## 前提条件

```bash
cargo build -p eds
```

LLM サーバーは不要です。すべてのステップはオフラインで動作します。

## フィクスチャ

```
crates/edgesentry-document/fixtures/
  voyage_V001_compliant.csv      -- TC1: 正常な船舶
  voyage_V002_bwm_expired.csv    -- TC2: BWM D-2 証明書の有効期限切れ
  voyage_V003_low_confidence.csv -- TC3: crew_count と cargo HS コードが欠損

clarus-commercial/profiles/sg-port-compliance/
  rules.json                     -- BWM_D2_EXPIRED, QUARANTINE_PRENOTIFICATION,
                                 --   DG_RESTRICTION, CREW_DOC_VALIDITY
  kb/
    BWM_D2_EXPIRED.txt
    QUARANTINE_PRENOTIFICATION.txt
    DG_RESTRICTION.txt
    CREW_DOC_VALIDITY.txt
```

## TC1 - コンプライアント航海

```bash
# ステップ 1 - 取込（海事データの解析 — CSV フィクスチャ）
eds parse maritime \
  --source crates/edgesentry-document/fixtures/voyage_V001_compliant.csv \
  --out /tmp/entity.jsonl

# ステップ 3 - 評価（ドキュメントフィールドの入力）
eds document fill \
  --input /tmp/entity.jsonl \
  --template fal-form-1 \
  --out /tmp/filled.jsonl
# review_required: false -- すべてのフィールドの信頼度 0.95

# ステップ 3 続き - コンプライアンスルールの確認
eds document check \
  --input /tmp/filled.jsonl \
  --profile clarus-commercial/profiles/sg-port-compliance \
  --out /tmp/alerts.jsonl
# 0件のコンプライアンスアラート

# ステップ 6 - 文書化（HTML のレンダリング）
eds document gen \
  --input /tmp/filled.jsonl \
  --template fal-form-1 \
  --out /tmp/fal-form-1.html
```

ブラウザで `fal-form-1.html` を開いて、入力済みの FAL Form 1 を確認してください。ブラウザの印刷ダイアログから PDF に印刷できます。

## TC2 - BWM 証明書期限切れ

```bash
eds parse maritime \
  --source crates/edgesentry-document/fixtures/voyage_V002_bwm_expired.csv \
  --out /tmp/entity_v002.jsonl

eds document fill \
  --input /tmp/entity_v002.jsonl \
  --template fal-form-1 \
  --out /tmp/filled_v002.jsonl

eds document check \
  --input /tmp/filled_v002.jsonl \
  --profile clarus-commercial/profiles/sg-port-compliance \
  --out /tmp/alerts_v002.jsonl
```

`alerts_v002.jsonl` の期待されるアラート：

```json
{"rule_id":"BWM_D2_EXPIRED","severity":"HIGH","field":"bwm_certificate_expiry",
 "message":"Rule 'BWM_D2_EXPIRED' failed check 'not_expired' on field 'bwm_certificate_expiry'",
 "regulation":"Ballast Water Management Convention (BWM) D-2 Standard -- MPA Port Marine Circular No. 19 of 2023",
 "voyage_id":"V002"}
```

HIGH 重大度のアラートはエクスポートをブロックします。BWM 証明書が更新されるまで、船舶は進めません。

## TC3 - 低信頼度（フィールド欠損）

```bash
eds parse maritime \
  --source crates/edgesentry-document/fixtures/voyage_V003_low_confidence.csv \
  --out /tmp/entity_v003.jsonl

eds document fill \
  --input /tmp/entity_v003.jsonl \
  --template fal-form-1 \
  --out /tmp/filled_v003.jsonl
```

`filled_v003.jsonl` は `review_required: true` となります。`CREW_COUNT` および `CARGO_HS_CODE` フィールドはソース CSV から欠落しており、`confidence: 0.0, flagged: true` を受け取ります。ドキュメントを提出する前に、人間のレビュアーが正しい値を入力する必要があります。

## 利用可能なテンプレート

| テンプレート名 | フォーム |
|---|---|
| `fal-form-1` | FAL Form 1 - 一般申告書（IMO） |
| `fal-form-5` | FAL Form 5 - 乗組員名簿（IMO） |
| `sg-port-entry` | シンガポール MPA Port+ パッケージ |
