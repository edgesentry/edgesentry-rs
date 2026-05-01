# STRIDE 脅威モデル

本文書は、SS 711:2025 **防御の厳格性（Rigour in Defence）** および IMDA IoT サイバーセキュリティガイドの脅威モデリングチェックリストに基づき、シンガポール CLS Level 3 評価向けに作成した正式な脅威モデリング成果物です。API・通信チャネル・ストレージのすべての攻撃面を対象とします。

**手法:** STRIDE（Microsoft）
**スコープ:** `edgesentry-rs` ライブラリおよび `edgesentry-bridge` FFI クレート — デバイス側署名、クラウド側インジェスト、HTTP トランスポート、オペレーションログ、監査台帳
**評価者参照:** SS 711:2025 §4.2 防御の厳格性; IMDA IoT サイバーセキュリティガイド §3 脅威モデリングチェックリスト

---

## システム概要

```
┌─────────────────────────────────────────────────────────────────┐
│  フィールドデバイス（エッジ）                                         │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  build_signed_record()                                     │ │
│  │  payload → BLAKE3 ハッシュ → Ed25519 署名 → AuditRecord   │ │
│  └────────────────────────────────────────────────────────────┘ │
└────────────────────────────┬────────────────────────────────────┘
                             │ POST /api/v1/ingest（HTTPS 上の JSON）
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│  クラウドインジェスト層                                              │
│  ┌────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │ NetworkPolicy  │  │ IntegrityPolicy │  │ AsyncIngest     │  │
│  │ IP/CIDR ゲート │→ │ Gate            │→ │ Service         │  │
│  │ （拒否デフォ）  │  │ （署名＋チェーン │  │ （ハッシュチェーン│  │
│  └────────────────┘  │  検証）         │  │  ＋シーケンス）  │  │
│                      └─────────────────┘  └────────┬────────┘  │
│                                                     │           │
│            ┌────────────────────────────────────────┤           │
│            ▼                          ▼             ▼           │
│  ┌──────────────────┐  ┌─────────────────────┐  ┌──────────┐   │
│  │  Raw Data Store  │  │  Audit Ledger       │  │ Op. Log  │   │
│  │  (S3 / メモリ)   │  │  (Postgres / メモリ) │  │          │   │
│  └──────────────────┘  └─────────────────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## STRIDE 脅威分析

### S — なりすまし（Spoofing）— デバイス ID

**脅威:** 攻撃者が `device_id` フィールドを偽装するか、侵害された鍵で署名したレコードをリプレイすることで、正規のフィールドデバイスになりすます。

**攻撃面:** `POST /api/v1/ingest` — `AuditRecord.device_id` および `AuditRecord.signature` フィールド。

| サブ脅威 | 説明 |
|---------|------|
| S-1 | 有効な `device_id` を持つが未登録の Ed25519 鍵でレコードを送信する |
| S-2 | 正当に署名された過去のレコードをリプレイする |
| S-3 | 署名鍵と一致しない偽造 `device_id` でレコードを送信する |

**緩和策:**

| ID | 緩和策 | コード位置 |
|----|--------|-----------|
| M-S-1 | デバイスの公開鍵はクラウド側で事前登録される。登録済み鍵で検証できない署名は `IngestError::UnknownDevice` として拒否される | `ingest/policy.rs` `IntegrityPolicyGate::enforce()` |
| M-S-2 | 単調増加するシーケンス番号と `prev_record_hash` チェーン継続性を強制；リプレイされたレコードは重複シーケンスとして検出される | `ingest/verify.rs` `check_sequence()` |
| M-S-3 | Ed25519 署名はペイロードハッシュを秘密鍵に結びつける；偽造 `device_id` は署名検証で失敗する | `identity.rs` `verify_payload_signature()` |

**残留リスク:** デバイスの秘密鍵が物理的に抽出された場合、有効な署名でレコードを偽造できる。ハードウェアバックアップ鍵ストレージ（TPM/SE）はデバイス層の管理策であり、本ライブラリのスコープ外。[ロードマップ](../audit/roadmap.md)に記載。

---

### T — 改ざん（Tampering）— 監査レコード

**脅威:** 攻撃者が転送中または保存中の監査レコードあるいは生ペイロードを改ざんする。

**攻撃面:** ワイヤフォーマット（JSON ボディ）、Raw データストア（S3 オブジェクト）、監査台帳（DBレコード）。

| サブ脅威 | 説明 |
|---------|------|
| T-1 | HTTP リクエストボディの `raw_payload_hex` を改ざんする |
| T-2 | 異なるペイロードに合わせて `AuditRecord.payload_hash` を改ざんする |
| T-3 | 受理後に保存 S3 オブジェクトのバイトを反転させる |
| T-4 | チェーンを断ち切るまたはリダイレクトするために `prev_record_hash` を改ざんする |

**緩和策:**

| ID | 緩和策 | コード位置 |
|----|--------|-----------|
| M-T-1 | インジェスト毎にクラウドが `BLAKE3(raw_payload)` を再計算し `record.payload_hash` と比較；不一致は `PayloadHashMismatch` として拒否される | `ingest/storage.rs` `IngestService::ingest()` |
| M-T-2 | `payload_hash` は Ed25519 署名で保護される；ハッシュが変更されると署名検証が失敗する | `identity.rs` `verify_payload_signature()` |
| M-T-3 | 保存後の改ざんは、台帳のハッシュとオブジェクト内容を再検証することで検出可能；[運用ランブック](../audit/operations.md)に記載の運用的管理策 | — |
| M-T-4 | `prev_record_hash` は直前の受理済みレコードの `hash()` と照合される；継続性が断たれると以後のすべてのレコードが拒否される | `ingest/verify.rs` `check_chain_link()` |

**残留リスク:** 受理後の保存オブジェクト改ざんはストレージ層の問題。S3 Object Lock（WORM）やDB行レベルチェックサムをデプロイ層で有効化することで排除できる。

---

### R — 否認（Repudiation）— オペレーションログ

**脅威:** デバイスまたはオペレーターが特定のインジェストイベントの発生を否定する、またはレコードが送信されなかった・拒否されたと主張する。

**攻撃面:** インジェスト中に書き込まれる `OperationLog` エントリ；監査台帳への追記操作。

| サブ脅威 | 説明 |
|---------|------|
| R-1 | デバイスがレコードを送信していないと主張する |
| R-2 | オペレーターがレコードが受理された（または拒否された）事実を否定する |
| R-3 | オペレーションログエントリを事後に削除または改ざんする |

**緩和策:**

| ID | 緩和策 | コード位置 |
|----|--------|-----------|
| M-R-1 | 受理・拒否を問わず、すべてのインジェスト試行に対して `device_id`・`sequence`・`decision`・`message` を含む `OperationLogEntry` が書き込まれる | `ingest/storage.rs` `log_acceptance()` / `log_rejection()` |
| M-R-2 | `IngestDecision::Accepted` / `Rejected` は決定と同時に操作ログに永続化される；レコードの署名済みハッシュが送信の暗号学的証明となる | `ingest/storage.rs` `OperationLogEntry` |
| M-R-3 | 追記専用のオペレーションログ（Postgres は `INSERT` のみ；ログ行への `DELETE`/`UPDATE` なし）により事後改ざんを防止する | `ingest/storage.rs` `PostgresOperationLog`；DB ユーザー権限で強制 |

**残留リスク:** ライブラリはログデータを提供する；特権インサイダーによる削除からそのデータを守るには、DB 層の管理策（ロール分離、DB 層での監査ログ）が必要。

---

### I — 情報漏えい（Information Disclosure）— ペイロードストレージ

**脅威:** 機密性の高い検査ペイロードデータが不正な第三者に露出する。

**攻撃面:** HTTP リクエストボディ（`raw_payload_hex`）、Raw データストア（S3）、監査台帳、オペレーションログ。

| サブ脅威 | 説明 |
|---------|------|
| I-1 | HTTP 通信チャネルへの盗聴 |
| I-2 | S3 オブジェクトまたは Postgres 行への不正読み取りアクセス |
| I-3 | エラーメッセージやログにペイロードバイトが現れる |

**緩和策:**

| ID | 緩和策 | コード位置 |
|----|--------|-----------|
| M-I-1 | HTTP トランスポートは TLS ターミネーション（ロードバランサー / Nginx / Cloudflare）の後ろで動作するよう設計されている；生ペイロードは JSON ボディ内で hex エンコードされ HTTPS で転送される必要がある | `transport/http.rs` — TLS はデプロイ層の管理策；[運用ランブック](../audit/operations.md)に記載 |
| M-I-2 | 生ペイロードは呼び出し元が指定したキーで `object_ref` により保存される；アクセス制御はストレージ層（S3 バケットポリシー、Postgres GRANT）で強制；ライブラリは非認証の呼び出し元に読み取り API を公開しない | `ingest/storage.rs` `RawDataStore::put()` |
| M-I-3 | エラーメッセージには `device_id` と `sequence` が含まれるが生ペイロードバイトは含まれない；`tracing` スパンはペイロードバイト長のみを記録する | `ingest/storage.rs` `#[instrument(skip(raw_payload))]` |

**残留リスク:** S3 オブジェクトと Postgres 行の保存時暗号化はデプロイ層の管理策（S3 SSE-KMS、Postgres `pgcrypto` または TDE）。インジェスト HTTP エンドポイントの TLS 1.3 は[ロードマップ](../audit/roadmap.md)（issue #73）で対応予定。

---

### D — サービス拒否（Denial of Service）— ネットワークポリシー

**脅威:** 攻撃者がインジェストエンドポイントを大量のリクエストで溢れさせ、正規デバイスのレコード送信を妨害する。

**攻撃面:** `POST /api/v1/ingest` HTTP エンドポイント；`NetworkPolicy` チェック；`AsyncIngestService` tokio タスクプール。

| サブ脅威 | 説明 |
|---------|------|
| D-1 | 信頼できない IP からの大量リクエストがハンドラーを圧倒する |
| D-2 | 大きな `raw_payload_hex` 値がメモリを枯渇させる |
| D-3 | 不正な JSON ボディが解析時間を消費する |

**緩和策:**

| ID | 緩和策 | コード位置 |
|----|--------|-----------|
| M-D-1 | `NetworkPolicy` 拒否デフォルト：明示的に許可リストに登録されていない限り、すべての IP と CIDR 範囲をブロック；未承認の送信元 IP は暗号処理が実行される前に `403 Forbidden` を受け取る | `ingest/network_policy.rs` `NetworkPolicy::check()`；`transport/http.rs` ハンドラー |
| M-D-2 | Axum のデフォルトリクエストボディサイズ制限（2 MB）がペイロードサイズを上限化する | `transport/http.rs` — axum デフォルトボディ制限 |
| M-D-3 | JSON デシリアライズエラーは即座に `400 Bad Request` を返す；後段の処理は実行されない | `transport/http.rs` — axum `Json` エクストラクター |

**残留リスク:** ソース IP ごと・デバイスごとのレート制限はライブラリ層では未実装；本番デプロイではリバースプロキシまたは API ゲートウェイ層で追加すべき。issue #73（TLS、P2）が計画中のフォローアップマイルストーン。

---

### E — 特権昇格（Elevation of Privilege）— インジェストゲート

**脅威:** 攻撃者がインジェスト検証ゲートを回避し、任意のレコードを台帳または Raw データストアに書き込む。

**攻撃面:** `IntegrityPolicyGate`、`ingest_handler`、サービス登録 API（`register_device`）。

| サブ脅威 | 説明 |
|---------|------|
| E-1 | 攻撃者が未登録デバイスのレコードで `ingest` を呼び出し成功させる |
| E-2 | 攻撃者が制御していないデバイスの有効なシーケンス/チェーンを持つレコードを送信する |
| E-3 | 攻撃者が `register_device` を直接呼び出すことで悪意のあるデバイスを登録する |

**緩和策:**

| ID | 緩和策 | コード位置 |
|----|--------|-----------|
| M-E-1 | `IntegrityPolicyGate::enforce()` はストレージ書き込みの前に無条件で呼び出される；未知のデバイスは `IngestError::UnknownDevice` で失敗する | `ingest/policy.rs` |
| M-E-2 | 署名検証は `device_id` に対して登録済みの公開鍵を使用する；デバイスの秘密鍵なしに有効なチェーンは偽造できない | `identity.rs` `verify_payload_signature()` |
| M-E-3 | `register_device` は起動時にアプリケーション層のみが呼び出す特権操作；HTTP インジェストハンドラーはデバイス登録をネットワーク経由で公開しない | `transport/http.rs` — 登録エンドポイントなし；`ingest/storage.rs` `AsyncIngestService::register_device()` |

**残留リスク:** `register_device` を呼び出すアプリケーション層が侵害された場合、任意のデバイスを登録できる。これは運用セキュリティの管理策：登録は強力な認証を持つ別の特権 API の背後に置くべき。

---

## バイナリ解析エビデンス

### `cargo audit` — アドバイザリデータベーススキャン

コマンドと出力（アドバイザリデータベースコミット：最新）:

```
cargo audit
```

**結果:** 検出されたアドバイザリはすべて `deny.toml` で事前承認済み（下表参照）:

| アドバイザリ | クレート | バージョン | ステータス | 理由 |
|------------|--------|----------|---------|------|
| RUSTSEC-2026-0049 | `rustls-webpki` | 0.101.7 | 無視（[#125](https://github.com/edgesentry/edgesentry-rs/issues/125)） | `aws-smithy-http-client` のレガシー `hyper-rustls 0.24` → `rustls 0.21` チェーンに固定されている；0.101.x パッチは存在しない。`0.103.x` インスタンスは 0.103.10 に更新済み。 |
| RUSTSEC-2026-0049 | `rustls-webpki` | 0.102.8 | 無視（[#166](https://github.com/edgesentry/edgesentry-rs/issues/166)） | `rumqttc 0.25` → `rustls 0.22` チェーンに固定されている；rustls 0.23+ を採用した rumqttc のリリースが必要。コードベース内に CRL 失効 API 呼び出しは存在しない。 |

その他のスキャン済みクレート依存関係: **既知の CVE なし**

再現手順:

```bash
cargo install cargo-audit --locked
cargo audit
```

### `cargo deny check` — ポリシー強制

コマンド:

```bash
cargo deny check
```

**結果:** `advisories ok, bans ok, licenses ok, sources ok`

`deny.toml` ポリシーの強制内容:
- アドバイザリ: 文書化された理由を持つ明示的な無視エントリを除き、すべての脆弱性をデフォルトで拒否
- バン: 複数クレートバージョンを警告；ワイルドカード依存を警告
- ライセンス: MIT・Apache-2.0・BSD-2-Clause・BSD-3-Clause・Unicode-3.0・CC0-1.0・Zlib のみ許可；例外 1 件: `cbindgen`（MPL-2.0、ビルド専用ヘッダー生成ツール — コピーレフトは生成物やソースコードに及ばない）
- ソース: `crates.io` および信頼済み git ソースのみ

再現手順:

```bash
cargo install cargo-deny --locked
cargo deny check
```

---

## 脅威と緩和策のトレーサビリティ要約

| STRIDE カテゴリ | 脅威 ID | 緩和策 ID | ソースファイル | ステータス |
|----------------|--------|---------|-------------|----------|
| なりすまし | S-1 | M-S-1 | `ingest/policy.rs` | ✅ |
| なりすまし | S-2 | M-S-2 | `ingest/verify.rs` | ✅ |
| なりすまし | S-3 | M-S-3 | `identity.rs` | ✅ |
| 改ざん | T-1 | M-T-1 | `ingest/storage.rs` | ✅ |
| 改ざん | T-2 | M-T-2 | `identity.rs` | ✅ |
| 改ざん | T-3 | M-T-3 | 運用的管理策 | ⚠️ デプロイ |
| 改ざん | T-4 | M-T-4 | `ingest/verify.rs` | ✅ |
| 否認 | R-1 | M-R-1 | `ingest/storage.rs` | ✅ |
| 否認 | R-2 | M-R-2 | `ingest/storage.rs` | ✅ |
| 否認 | R-3 | M-R-3 | DB 権限層 | ⚠️ デプロイ |
| 情報漏えい | I-1 | M-I-1 | デプロイ（TLS） | ⚠️ [#73](https://github.com/edgesentry/edgesentry-rs/issues/73) |
| 情報漏えい | I-2 | M-I-2 | ストレージアクセス制御 | ⚠️ デプロイ |
| 情報漏えい | I-3 | M-I-3 | `ingest/storage.rs` | ✅ |
| サービス拒否 | D-1 | M-D-1 | `ingest/network_policy.rs`、`transport/http.rs` | ✅ |
| サービス拒否 | D-2 | M-D-2 | `transport/http.rs`（axum ボディ制限） | ✅ |
| サービス拒否 | D-3 | M-D-3 | `transport/http.rs` | ✅ |
| 特権昇格 | E-1 | M-E-1 | `ingest/policy.rs` | ✅ |
| 特権昇格 | E-2 | M-E-2 | `identity.rs` | ✅ |
| 特権昇格 | E-3 | M-E-3 | `transport/http.rs` | ✅ |

**凡例:** ✅ ライブラリコードに実装済み — ⚠️ デプロイ層の管理策（ライブラリスコープ外）
