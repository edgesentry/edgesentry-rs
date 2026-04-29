# アーキテクチャ

## デバイス側とクラウド側

このシステムは、フィールドデバイス（例：エレベーター点検デバイス）が点検エビデンスをクラウドサービスに送信する、公共インフラ向け IoT デプロイを想定しています。

### デバイス側（リソース制約のあるエッジ）

デバイス側の責務は`edgesentry_rs::build_signed_record`および関連関数によって実装されます。

- 点検イベントのペイロードを生成する（ドアチェック・振動チェック・非常ブレーキチェック）
- `payload_hash`を計算する（ BLAKE3 ）
- Ed25519 秘密鍵でハッシュに署名する
- レコードがチェーンを形成するように、各イベントを前のレコードハッシュ（`prev_record_hash`）に連結する
- エッジ側のコストを抑えるために、コンパクトな監査メタデータとオブジェクト参照（`object_ref`）のみを送信する

### クラウド側（検証とトラスト強制）

クラウド側の責務は`edgesentry_rs::ingest`および関連モジュールによって実装されます。

- 承認済み IP アドレスおよび CIDR レンジへの受信接続をゲートする（`NetworkPolicy::check`）— デフォルト拒否
- デバイスが既知であることを検証する（`device_id` -> 公開鍵）
- 受信レコードごとに署名の有効性を検証する
- シーケンスの単調増加を強制し、重複を拒否する
- ハッシュチェーンの連続性を強制する（`prev_record_hash`は前のレコードハッシュと一致しなければならない）
- 改ざん・リプレイ・並べ替えされたデータを永続化前に拒否する

### 共有トラストロジック

すべてのハッシュと検証ルールは同じ`edgesentry-rs`クレート内に置かれ、エッジとクラウドの両方で使用する際のロジックが同一であることを保証します。

## リソース制約デバイスの設計

デバイス側の設計は意図的に軽量にされており、 Cortex-M クラスの環境への適用が可能です。

- **小さな暗号フットプリント：** レコードは固定サイズのハッシュ（`[u8; 32]`）と署名（`[u8; 64]`）を保存する
- **最小限の計算パス：** ハッシュと署名のみ。デバイス上に重いサーバー側検証ロジックは不要
- **コンパクトなワイヤフォーマットへの対応：** レコード構造は決定的でシリアライズ可能（コア部分で`serde` + `postcard`をサポート）
- **重い処理をクラウドにオフロード：** 重複検知・シーケンスポリシーチェック・フルチェーン検証はクラウドの責務
- **設計による改ざん検知：** 1 バイトの変更で署名チェックまたはチェーンの連続性が壊れる

## 具体的な設計フロー

1. デバイスがイベントペイロード`D`を作成する。
2. デバイスが`H = hash(D)`を計算し、`H`に署名して署名`S`を得る。
3. デバイスが`AuditRecord { device_id, sequence, timestamp_ms, payload_hash=H, signature=S, prev_record_hash, object_ref }`を送出する。
4. クラウドが登録済み公開鍵で署名を検証する。
5. クラウドがシーケンスと前ハッシュのリンクを検証する。
6. いずれかのチェックが失敗した場合、インジェストは拒否される。すべてのチェックが通過した場合、レコードは受け入れられる。

要約すると、エッジはファクトに署名し、クラウドが連続性と真正性を強制します。

## 公証メタデータスキーマ

AI 推論結果を法的に有効な証拠として扱うには（BCA/CONQUAS 検査レポート、MPA 船舶証明書、国土交通省の近接目視検査同等性証明）、暗号的完全性に加えて、5 種類のプロベナンス（来歴）メタデータを監査レコードのペイロードに含める必要があります。これが公証コネクタの目標スキーマです。

| カテゴリ | フィールド | 目的 |
|---|---|---|
| **センサー** | `sensor_id`、`calibration_ts`、`firmware_version`、`sampling_rate` | 計測機器がキャプチャ時に校正済みかつ仕様範囲内で動作していたことを証明する |
| **AI モデル** | `model_uuid`、`model_arch`、`weight_sha256`、`prompt_version` | 同一入力から同一推論出力を第三者が再現できることを担保する（AI Verify アウトカム 3.1 / 3.5） |
| **計算環境** | `device_type`、`os_version`、`dependency_hashes`、`hw_temp_c` | ランタイムの完全な再現性。ハードウェア温度は推論タイミングに影響するサーマルスロットリングを検出する |
| **コンテキスト** | `ntp_ts`、`gps_lat_lon`（または屋内測位）、`input_data_hash` | レコードを特定の物理的場所と時刻に紐付ける。`input_data_hash` はペイロードの差し替えを防ぐ |
| **推論プロセス** | `confidence_score`、`preprocessing_algo`、`guardrail_actions` | ヒューマン・イン・ザ・ループのトリアージを支援する（AI Verify アウトカム 4.5）。信頼度が低いレコードは手動レビューに回すことができる |

これらのフィールドはドメイン固有の検出データと共に `payload` オブジェクトに格納されます。`AuditRecord` の `payload_hash` はペイロード全体を対象とするため、メタデータフィールドを 1 つでも変更すると署名が無効になります。

**ALCOA+ との対応：** この 5 カテゴリは規制当局への提出に求められる ALCOA+ データ整合性フレームワークに直接対応します。帰属性（センサー・モデル識別情報）、判読性（構造化 JSON）、同時性（`ntp_ts`）、原本性（`input_data_hash`）、正確性（`weight_sha256`、`calibration_ts`）、加えて完全性・一貫性・耐久性・可用性（WORM ストレージコネクタが担保）。

## インジェストサービス：同期・非同期パス

`edgesentry-rs` はフィーチャーフラグで選択できるクラウド側インジェスト用オーケストレーションサービスを 2 種類提供します。

| 型 | フィーチャーフラグ | スレッドモデル | 用途 |
|------|-------------|-------------|--------------|
| `IngestService` | *（常に利用可能）* | ブロッキング / 同期 | 組み込み・CLI ツール・組み込みランタイム |
| `AsyncIngestService` | `async-ingest` | `async/await`（tokio）| HTTP サーバー・非同期パイプライン |

### 同期パス（`IngestService`）

同期サービスはデフォルトであり、追加フィーチャーは不要です。S3 書き込み（`s3` フィーチャーが有効な場合）は組み込みの `tokio::runtime::Runtime` 内で `block_on` して実行されます。シングルスレッドツールや組み込み環境に適しています。

```rust
let mut svc = IngestService::new(policy, raw_store, ledger, op_log);
svc.register_device("lift-01", verifying_key);
svc.ingest(record, payload, None)?;
```

### 非同期パス（`AsyncIngestService`）

`features = ["async-ingest"]` で有効化します。すべてのストレージ呼び出しが `.await` を使用するため、呼び出しスレッドがブロックされず、高並行パイプラインを実現します。ポリシーゲートは `tokio::sync::Mutex` でラップされているため、`Arc` 経由でタスク間で共有できます。

```rust
let svc = Arc::new(AsyncIngestService::new(policy, raw_store, ledger, op_log));
svc.register_device("lift-01", verifying_key).await;
svc.ingest(record, payload, None).await?;
```

`s3` と `async-ingest` が両方有効な場合、`S3CompatibleRawDataStore` は AWS SDK フューチャーを直接呼び出して `AsyncRawDataStore` を実装します。組み込みランタイムは不要です。

### フィーチャーフラグ一覧

| フラグ | 追加されるもの |
|------|-------------|
| `async-ingest` | `AsyncRawDataStore`・`AsyncAuditLedger`・`AsyncOperationLogStore` トレイト；`AsyncIngestService`；インメモリ非同期ストア；`tokio`（sync + macros）|
| `s3` | `S3CompatibleRawDataStore`（同期）；`async-ingest` と組み合わせると `AsyncRawDataStore` も実装 |
| `postgres` | `PostgresAuditLedger`・`PostgresOperationLog`（同期）|
| `transport-http` | `transport::http::serve()` — axum ベースの `POST /api/v1/ingest` サーバー；`eds serve` CLI サブコマンド |
| `transport-mqtt` | `transport::mqtt::serve_mqtt()` — 非同期 rumqttc イベントループ；トピックをサブスクライブし、レコードを `AsyncIngestService` にルーティングし、承認/拒否レスポンスを発行 |

## トランスポート層

`transport` モジュールは `AsyncIngestService` の上に構築されたネットワーク向けインジェストエンドポイントを提供します。

### HTTP（`transport-http` フィーチャー）

`features = ["transport-http"]` で有効化します。`axum 0.8` を取り込み、単一の `POST /api/v1/ingest` エンドポイントを公開します。

#### リクエスト / レスポンス

| フィールド | 型 | 説明 |
|-------|------|-------------|
| `record` | `AuditRecord`（JSON）| デバイスからの署名済み監査レコード |
| `raw_payload_hex` | `String` | 16 進数エンコードされた生ペイロードバイト |

| ステータス | 意味 |
|--------|---------|
| `202 Accepted` | レコードがすべてのチェックを通過し、保存された |
| `400 Bad Request` | `raw_payload_hex` が有効な 16 進数でない |
| `403 Forbidden` | クライアント IP が `NetworkPolicy` の許可リストにない |
| `422 Unprocessable Entity` | レコードの署名・ハッシュ・チェーン検証に失敗した |

#### 使用例

```rust
use edgesentry_rs::{
    AsyncIngestService, AsyncInMemoryRawDataStore, AsyncInMemoryAuditLedger,
    AsyncInMemoryOperationLog, IntegrityPolicyGate, NetworkPolicy,
};
use edgesentry_rs::transport::http::serve;

let mut policy = IntegrityPolicyGate::new();
policy.register_device("lift-01", verifying_key);

let mut network_policy = NetworkPolicy::new();
network_policy.allow_cidr("10.0.0.0/8").unwrap();

let service = AsyncIngestService::new(
    policy,
    AsyncInMemoryRawDataStore::default(),
    AsyncInMemoryAuditLedger::default(),
    AsyncInMemoryOperationLog::default(),
);

let addr = "0.0.0.0:8080".parse().unwrap();
serve(service, network_policy, addr).await?;
```

#### CLI

```sh
eds serve \
  --addr 0.0.0.0:8080 \
  --allowed-sources 10.0.0.0/8,127.0.0.1 \
  --device lift-01=<pubkey_hex>
```

### MQTT（`transport-mqtt` フィーチャー）

`features = ["transport-mqtt"]` で有効化します。`rumqttc` を取り込み、`serve_mqtt()` を公開します。`serve_mqtt()` は MQTT ブローカーに接続し、設定可能なインジェストトピックをサブスクライブし、受信メッセージを `AsyncIngestService` にルーティングする完全非同期イベントループです。

メッセージ形式は HTTP トランスポートと同じ JSON エンベロープです：

```json
{ "record": { "device_id": "...", "sequence": 1, ... }, "raw_payload_hex": "deadbeef..." }
```

承認/拒否の結果は `<topic>/response` に発行されます：

```json
{ "device_id": "...", "sequence": 1, "status": "accepted" }
{ "device_id": "...", "sequence": 1, "status": "rejected", "error": "..." }
```

#### 使用例

```rust
use edgesentry_rs::transport::mqtt::{MqttIngestConfig, serve_mqtt};
use edgesentry_rs::{
    AsyncIngestService, AsyncInMemoryRawDataStore, AsyncInMemoryAuditLedger,
    AsyncInMemoryOperationLog, IntegrityPolicyGate,
};

let service = AsyncIngestService::new(
    IntegrityPolicyGate::new(),
    AsyncInMemoryRawDataStore::default(),
    AsyncInMemoryAuditLedger::default(),
    AsyncInMemoryOperationLog::default(),
);

let config = MqttIngestConfig::new("mqtt.example.com", "devices/+/ingest", "edgesentry-cloud");
serve_mqtt(config, service).await?;
```

`serve_mqtt` はブローカー接続が切断されるまで実行し、`MqttServeError::EventLoop` を返します。自動再接続にはリトライループでラップしてください。

#### 主な動作

| 動作 | 詳細 |
|-----------|--------|
| 不正な JSON | メッセージはログに記録され破棄される；イベントループは継続する |
| 無効な 16 進数ペイロード | メッセージはログに記録され破棄される；イベントループは継続する |
| インジェスト拒否 | `"status": "rejected"` を含むレスポンスを `<topic>/response` に発行する |
| レスポンス発行失敗 | 警告としてログに記録される；イベントループは停止しない |
