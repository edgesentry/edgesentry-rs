# EdgeSentry-RS — ツールキットロードマップ

EdgeSentry-RS は、センサーから封印（seal）まで7ステップのパイプラインを提供する Rust ワークスペースです。
実世界の計測データを取り込み、ルールと照合し、逸脱を平易な言語で説明し、改ざん不能な記録を生成する——
このパターンに当てはまるあらゆるドメインで利用できます。

本ドキュメントはツールキット全体の実装状況と今後の計画を追跡します。

---

## 設計原則

**1. モノリスではなくパイプラインステージ**

各ステップは単一の責務を持つ独立したクレートです。ステージ間のデータ受け渡しは JSONL ファイルの
バケツリレーで行い、プロセス内の共有状態は不要です。アプリケーションはどのステージをどこで
実行するかを自由に選択できます。

**2. 決定論的なコア、非決定論的な周辺**

ステップ 1–3（Ingest, Compute, Evaluate）は決定論的であり、エッジでのリアルタイム実行に適しています。
ステップ 4–7（Assess, Explain, Document, Seal）はレイテンシ、外部サービス、非同期スケジューリングを
伴う可能性があり、そのコンテキスト向けに設計されています。

**3. エッジは事実を封印する。クラウドは解釈する。**

改ざん不能な計測記録は、規制解釈が適用される前の発生時点で封印する必要があります。
規制ナレッジベース（ルール定義、規制文書）はクラウドで管理・更新されます。
エッジデバイスに必要なのは、オペレーターへの警告発報と生計測値の封印に使う閾値のみです。
これにより、規制の更新はクラウド側で一度行えば全現場に即時反映され、フィールドデバイスへの
再デプロイは不要になります。詳細は [Edge / Cloud パイプライン分割](../pipeline/edge-cloud-split.ja.md) を参照してください。

**4. 同じバイナリ、2つの実行コンテキスト**

`eds` バイナリはエッジとクラウドで同一です。実行コンテキスト（エッジ vs クラウド）は、
どのサブコマンドを呼ぶか・どのプロファイルファイルが存在するかによって決まり、
条件付きコンパイルや別バイナリは不要です。

**5. オープンコア**

エンジン（物理演算、ルール評価、LLM チェーン構造、監査フォーマット）は Apache 2.0 / MIT です。
規制プロファイル（ルールデータセット、ナレッジベース、管轄固有パラメータ）は別ライセンスです。
エンジンは検査可能であるから信頼でき、プロファイルはドメイン専門知識を要するから価値があります。

---

## 7ステップパイプライン — 現在の実装状況

| ステップ | クレート | CLI | 状態 |
|---|---|---|---|
| 1a — 構造化データ取り込み | `edgesentry-ingest` | `eds ingest replay` / `eds ingest stream` | ✅ 完了 |
| 1b — 非構造化データ解析 | `edgesentry-parse` | `eds parse maritime` / `eds parse image` | ✅ CSV 完了 · 📋 ONNX ビジョン スタブ |
| 2 — 演算 | `edgesentry-compute` | `eds compute run` | ✅ 完了 |
| 3 — 評価 | `edgesentry-evaluate` | `eds evaluate run` | ✅ 完了 |
| 4 — 認識 | `edgesentry-assess` | `eds assess run` | ✅ 完了 |
| 5 — 説明 | `edgesentry-explain` | `eds explain run` | ✅ 完了 |
| 6a — 安全レポート | `edgesentry-report` | `eds report generate` | ✅ Markdown + PDF 完了 |
| 6b — 書類コンプライアンス | `edgesentry-document` | `eds document fill / check / gen` | ✅ 完了 |
| 7 — 封印 | `edgesentry-audit` | `eds audit sign / verify` | ✅ 完了 |

**サポートクレート：**

| クレート | CLI | 役割 | 状態 |
|---|---|---|---|
| `edgesentry-profile` | `eds profile validate / list` | ルールプロファイルのロードと検証 | ✅ 完了 |
| `edgesentry-store` | — | デーモンモード用インメモリイベントストア | ✅ 完了 |
| `edgesentry-scenario` | `eds scenario generate / simulate` | 合成 CSV・UDP フィクスチャ生成 | ✅ 完了 |
| `edgesentry-image-utils` | — | 共通画像処理（ONNX / OpenCV、feature flag 制御） | 📋 スタブ |
| `edgesentry-bridge` | — | 組み込みデプロイ向け C FFI ブリッジ | ✅ 完了 |

---

## 実装済みプロファイル

| プロファイル | 場所 | ルール数 | KB エントリ数 |
|---|---|---|---|
| `demo` | `crates/edgesentry-profile/fixtures/demo/` | 3（近接・除外ゾーン・TTC） | 3 |
| `sg-maritime-security` | `crates/edgesentry-profile/fixtures/sg-maritime-security/` | 2（制限ゾーン接近・AIS ギャップ） | 2 |
| `sg-port-compliance` | `crates/edgesentry-profile/fixtures/sg-port-compliance/` | 4（BWM・検疫・危険物・乗組員書類） | 4 |

---

## 近期作業 — 2026年6月デッドライン

エンドツーエンドのパイプライン全体をデモするために必要な項目です。

### P0 — 提出前に必ずやる

| Issue | 成果物 | 理由 |
|---|---|---|
| [#299](https://github.com/edgesentry/edgesentry-rs/issues/299) | AIS NMEA 0183 入力アダプタ — `eds ingest stream --source ais://` | 海事セキュリティシナリオ（Tier 2） |
| [#300](https://github.com/edgesentry/edgesentry-rs/issues/300) | `eds audit sign-document` / `verify-document` — `DocumentAuditPayload` + PDF ハッシュ埋め込み | 書類監査チェーン（TC4 デモ） |
| [#19](https://github.com/edgesentry/edgesentry-rs/issues/19) | ブラウザデモ UI — `eds serve` 分割画面（レポート生成・検証パネル） | 提出デモ動画 |

### P1 — 提出前にやった方が良い

| Issue | 成果物 | 理由 |
|---|---|---|
| [#302](https://github.com/edgesentry/edgesentry-rs/issues/302) | `sg-maritime-security` デモ用 合成 AIS CSV フィクスチャ | #299 完成前の代替手段 |
| [#303](https://github.com/edgesentry/edgesentry-rs/issues/303) | ARM64 クロスコンパイル CI（`aarch64-unknown-linux-gnu`） | エッジデプロイの主張を裏付ける |
| [#18](https://github.com/edgesentry/edgesentry-rs/issues/18) | LLM ランタイム判断ドキュメント（Ollama vs llama.cpp vs MLX） | 提出書類の技術セクション |
| [#301](https://github.com/edgesentry/edgesentry-rs/issues/301) | `eds parse maritime` が MVP で CSV を使うことを確認・Parquet は Phase 2 | スコープ明確化 |

---

## 法的証拠能力 — `AuditRecord` 強化

要件分析の全体：[docs/legal/index.ja.md](../legal/index.ja.md)。
最大のギャップは**信頼できるタイムスタンプ**と**`software_version` フィールドの欠如**。

### 2026年6月提出前（P1）

| 成果物 | 対応する要件 | 詳細 |
|---|---|---|
| `AuditRecord` に `software_version: String` を追加 | 要件6 — システム完全性 | コンパイル時に `env!("CARGO_PKG_VERSION")` + ビルドメタデータ経由でGit SHAを埋め込む；証拠法s.116A「正常稼働」を満たす |
| `AuditRecord` に `hash_alg: String`・`sig_alg: String` を追加 | 要件7 — 保存とフォーマット長期性 | アルゴリズムIDを記録に固定；10年以上後の独立検証を可能にする |
| 鍵登録プロセスのドキュメント化 | 要件2 — 帰属 | 公開鍵 → 顧客 → edgesentry オンボーディング；タイムスタンプ付きで保存 |
| R2アップロード時刻を信頼できるアンカーとして文書化 | 要件3 — 信頼できるタイムスタンプ（Phase 1） | Cloudflare `x-amz-date` はオペレーター独立；「インシデント前に封印された」論拠を確立 |

### Phase 2 — 提出後PoC（2026年11月）

| 成果物 | 対応する要件 | 詳細 |
|---|---|---|
| RFC 3161 TSA統合 | 要件3 — 信頼できるタイムスタンプ（Phase 2） | 署名時に記録ハッシュをTSA（DigiCert/GlobalSign）に送信；トークンを `AuditRecord` と一緒に保存 |
| HSM / TPM 鍵ストレージ | 要件2 — 帰属（Phase 2） | 秘密鍵が物理的に抽出不可；CLS Level 4 を満たす；[#54](https://github.com/edgesentry/edgesentry-rs/issues/54) で追跡 |
| 部分チェーンエクスポート形式 | 要件4 — 完全性 | アンカーレコード + ルートへの接続証明付きで時間範囲エクスポート |

### 本番 / 保険パートナーシップ前

| 成果物 | 詳細 |
|---|---|
| 外部法律意見書 | シンガポール海事法律事務所による証拠法s.116A適合性レビュー |
| P&I / H&M 保険会社パイロット | 1社の保険会社との実際の証拠要件確認 |

---

## 中期作業 — エッジ / クラウド分割

**アーキテクチャ：** エッジは生の `MeasurementRecord` を封印し、クラウドが `EvaluatedRecord` に評価する。
設計詳細は [edge-cloud-implementation.md](../pipeline/edge-cloud-implementation.md) を参照。

### `edgesentry-evaluate` に追加する新型

```
MeasurementRecord   ← エッジ出力
  breach_type: BreachType    (Distance | Ttc | Zone)
  measured_value: f32
  threshold: f32
  entity_ids: Vec<String>
  timestamp_ms: u64
  profile_version: String    ← 必須：イベント時点の有効閾値を証明する
  site_id: Option<String>

EvaluatedRecord     ← クラウド出力
  measurement_record_hash: [u8; 32]
  rule_id: String
  severity: Severity
  regulation: String
  site_id: Option<String>
  timestamp_ms: u64
```

`RiskEvent`（現行型）は後方互換性のため維持します。移行は追加的に行います。

### 新 CLI コマンド

```bash
# エッジ層
eds measure run  --input measurements.jsonl \
                 --params profile/params.toml \
                 --profile-version sg-port-safety@2.1.0 \
                 --out breaches.jsonl

# クラウド層
eds evaluate run --input breaches.jsonl \
                 --profile full-profile/ \
                 --mode cloud \
                 --out evaluated.jsonl

# R2 転送
eds r2 push  --input FILE --bucket BUCKET --prefix PREFIX [--immutable]
eds r2 pull  --bucket BUCKET --prefix PREFIX --out FILE
```

### プロファイル分割

| プロファイル構成要素 | エッジデバイス | クラウド |
|---|---|---|
| `params.toml` — 閾値、ゾーン形状 | ✅ 必要 | ✅ 必要 |
| `rules.json` — rule_id・条件・規制・severity | ❌ デプロイしない | ✅ 必要 |
| `kb/` — LLM 用規制 KB | ❌ デプロイしない | ✅ 必要 |
| `manifest.toml` — バージョン・管轄 | ✅ 必要 | ✅ 必要 |

### ビルド順序（提出後）

| 順序 | 成果物 |
|---|---|
| 1 | `BreachType` enum + `MeasurementRecord` 型 in `edgesentry-evaluate` |
| 2 | `EvaluatedRecord` 型 in `edgesentry-evaluate` |
| 3 | `evaluate_edge()` — 閾値チェックのみ、ルール検索なし |
| 4 | `evaluate_cloud()` — `MeasurementRecord` → `EvaluatedRecord` |
| 5 | `eds measure run` CLI |
| 6 | `eds evaluate run --mode cloud` |
| 7 | `eds r2 push / pull / list` |
| 8 | `edgesentry-profile` でエッジ用 `params.toml` を `rules.json` から分離 |
| 9 | R2 Object Lock（`--immutable` フラグ） |

---

## 長期作業 — 本番ハードニングと入力拡張

### ビジョン / カメラ入力

| Issue | 成果物 |
|---|---|
| [#305](https://github.com/edgesentry/edgesentry-rs/issues/305) | `edgesentry-image-utils` ONNX 物体検知 — USB/RTSP カメラ → `EntityStream` |
| [#304](https://github.com/edgesentry/edgesentry-rs/issues/304) | RTSP ストリームアダプタ — ライブ IP カメラ入力 |

### マルチソース fan-in

| Issue | 成果物 |
|---|---|
| [#307](https://github.com/edgesentry/edgesentry-rs/issues/307) | RTSP + AIS の同時入力を 1 つのルールエンジンへ；`Entity` / `RiskEvent` / `AuditRecord` に `sensor_id` フィールドを追加 |

### ハートビートと運用記録

| Issue | 成果物 |
|---|---|
| [#290](https://github.com/edgesentry/edgesentry-rs/issues/290) | 5 分毎のハートビート `AuditRecord` 発行 — ゾーンサマリ、センサー状態、パイプラインレイテンシ |
| [#291](https://github.com/edgesentry/edgesentry-rs/issues/291) | `eds report monthly` — 日付範囲フィルタ付き月次安全レポート |

### デプロイと運用

| Issue | 成果物 |
|---|---|
| [#303](https://github.com/edgesentry/edgesentry-rs/issues/303) | ARM64 CI ジョブ（`aarch64-unknown-linux-gnu`） |
| [#306](https://github.com/edgesentry/edgesentry-rs/issues/306) | RPi e2e スモークテスト — `deploy/smoke-test.sh` |
| [#10](https://github.com/edgesentry/edgesentry-rs/issues/10) | 本番 `sg-port-safety` プロファイル — ルール拡張、`params.toml`、`manifest.toml` |
| [#28](https://github.com/edgesentry/edgesentry-rs/issues/28) | OIDC トラステッドパブリッシングによるクレート公開 |
| [#30](https://github.com/edgesentry/edgesentry-rs/issues/30) | リリース品質ゲートをロックドビルドのみに絞り込み |

---

## JSONL スキーマバージョニング

`eds` が生成する JSONL ファイルの先頭行は、スキーマ名とバージョンを宣言するヘッダーレコードです。
これがパイプラインステージ間のコントラクトです。

```json
{"eds_schema": "EntityFrame",   "version": "1.0"}
{"eds_schema": "Measurement",   "version": "1.0"}
{"eds_schema": "RiskEvent",     "version": "1.0"}
{"eds_schema": "Assessment",    "version": "1.0"}
{"eds_schema": "Explanation",   "version": "1.0"}
{"eds_schema": "AuditRecord",   "version": "1.0"}
```

`MAJOR` バージョンの増加は破壊的変更です。`MINOR` の増加は追加のみです。
ヘッダーなしのファイルはバージョン不明として扱われ警告が出ます。

---

## 参照

- `docs/pipeline/` — ステップ別パイプラインドキュメント
- `docs/audit/roadmap.md` — 監査クレートのコンプライアンスロードマップ（CLS / JC-STAR）
- `docs/inspect/roadmap.md` — インスペクトクレートのロードマップ（3D 偏差、IFC）
- `docs/pipeline/edge-cloud-implementation.md` — エッジ / クラウド分割：Rust 型定義・CLI 設計・プロファイル分割・ビルド順序
- `_inputs/mvp.md` — 2026年6月提出スコープとデモフロー
- `_inputs/migration_roadmap.md` — Phase 1–3 クレート移行の経緯
