# エッジ / クラウド パイプライン分割

7ステップのパイプラインは2つの実行層に分割できます。
ステップ 1–2 はエッジで実行する必要があります。ステップ 5–7 はクラウドでの実行に適しています。
ステップ 3（Evaluate）が分割の境界点です。

---

## 原則

**エッジは事実を封印する。クラウドは解釈する。**

改ざん不能な計測記録を取得することと、どの規制に違反するかを判定することは、
同じ時間・同じ場所で行う必要のない独立した操作です。

| 層 | タイミング | 実行内容 |
|---|---|---|
| **エッジ**（同期） | 閾値超過の瞬間 | 物理演算、閾値判定、オペレーターへの警告、生計測値の封印とアップロード |
| **クラウド**（非同期） | アップロード後 | 規制ルールの特定、severity の付与、LLM 説明文の生成、コンプライアンスレポートの生成 |

規制ナレッジベース（ルール定義、規制文書、更新された通達）をエッジデバイスに
デプロイする必要はありません。クラウドで管理・更新されます。
規制の更新はクラウド側で一度行えば全現場に反映され、フィールドデバイスへの
再デプロイは不要です。

---

## エッジ層

```
eds ingest stream   # または eds ingest replay
      │ EntityFrame JSONL
      ▼
eds compute run
      │ Measurement JSONL
      ▼
eds measure run     # 軽量閾値チェック — rules.json 不要
      │ MeasurementRecord JSONL
      │   { breach_type, measured_value, threshold,
      │     entity_ids, timestamp_ms, profile_version, site_id }
      ▼
eds audit sign      # BLAKE3 + Ed25519 で封印
      │ 封印済み MeasurementRecord JSONL
      ▼
eds r2 push --immutable   # R2 Object Lock バケットへアップロード
```

**オペレーターへの警告** は封印前に `eds measure run` の中でリアルタイムに発報されます（1秒以内）。

**エッジプロファイルに必要なもの：** `params.toml`（閾値、ゾーン形状）と
`manifest.toml`（バージョン、管轄）のみ。`rules.json` と `kb/` は不要。

---

## クラウド層

```
eds r2 pull         # 封印済み MeasurementRecord をダウンロード
      │ 封印済み MeasurementRecord JSONL
      ▼
eds evaluate run --mode cloud   # MeasurementRecord + フルプロファイル → EvaluatedRecord
      │ EvaluatedRecord JSONL
      │   { measurement_record_hash, rule_id, severity, regulation,
      │     site_id, timestamp_ms }
      ▼
eds audit sign      # EvaluatedRecord を封印 — 同じチェーンに追記
      │ 封印済み EvaluatedRecord JSONL
      ▼
eds explain run     # LLM 平易な説明文生成（非同期）
      ▼
eds report generate # コンプライアンスレポート PDF
```

**クラウドプロファイルに必要なもの：** フルプロファイル — `params.toml` + `rules.json` + `kb/` + `manifest.toml`。

---

## 2層チェーン

`MeasurementRecord` と `EvaluatedRecord` はどちらも `edgesentry-audit` で封印され、
R2 不変バケット内の同じ BLAKE3 + Ed25519 ハッシュチェーンに追記されます。

```
エッジ:   MeasurementRecord  ──封印──▶  R2 (Object Lock)
                                             │
クラウド: EvaluatedRecord    ──封印──▶  R2 (同じチェーン)
          (measurement_record_hash を参照)
```

チェーンを照会する検証者は、あるイベントの両レコードを受け取り、以下を確認できます：
- 物理的計測値（`MeasurementRecord` より）— エッジで封印
- 規制上の判定（`EvaluatedRecord` より）— 非同期で適用
- 両者の紐付け（`measurement_record_hash` 経由）— 暗号学的に検証可能

どちらのレコードもアップロード後に改ざんすることはできません。

---

## プロファイル分割

| ファイル | エッジデバイス | クラウド |
|---|---|---|
| `params.toml` — 閾値、ゾーン形状 | ✅ | ✅ |
| `manifest.toml` — バージョン、管轄 | ✅ | ✅ |
| `rules.json` — rule_id、条件、規制、severity | ❌ | ✅ |
| `kb/` — LLM 用規制ナレッジベース | ❌ | ✅ |

---

## 同一バイナリ

`eds` バイナリはエッジとクラウドで同一です。
実行コンテキストはどのサブコマンドを呼ぶか・どのプロファイルファイルが存在するかで決まり、
ビルドフラグや別バイナリは不要です。

---

## 新しい型

`MeasurementRecord` と `EvaluatedRecord` は `edgesentry-evaluate` に定義されます。
既存の `RiskEvent` 型は後方互換性のために維持され、全ステップをエッジで実行する
単一層パイプラインでも引き続き使用できます。

完全な型定義とビルド順序は [edge-cloud-implementation.md](edge-cloud-implementation.md) を参照してください。
