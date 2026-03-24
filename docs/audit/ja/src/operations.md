# 運用ランブック

このページでは、EdgeSentry-RS の本番環境デプロイにおけるオブザーバビリティの設定、アラート閾値、バックアップ・リストア手順を説明します。

---

## オブザーバビリティ

### `tracing` を使った構造化ログ

EdgeSentry-RS は [`tracing`](https://docs.rs/tracing) ファサードを使用しています。サブスクライバーはバンドルされていません。デプロイ側がアプリケーション起動時に任意のバックエンドを接続します。サブスクライバーが登録されていない場合、ライブラリのオーバーヘッドはゼロです。

**本番環境向け推奨サブスクライバー（JSON を stdout に出力し、Loki / CloudWatch に取り込む）：**

```toml
# ホストアプリケーションの Cargo.toml
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

```rust
use tracing_subscriber::{fmt, EnvFilter};

fn main() {
    fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env()) // RUST_LOG=edgesentry_rs=info
        .init();
    // ...
}
```

本番環境では `RUST_LOG=edgesentry_rs=info`、インシデント調査時は `edgesentry_rs=debug` を設定してください。

### ライブラリが出力する構造化ログイベント

すべてのイベントにはモジュールパスが `target` として含まれます。主要なイベントの一覧：

| レベル | ターゲット | イベント | 主要フィールド |
|--------|-----------|---------|---------------|
| `DEBUG` | `edgesentry_rs::agent` | `signing record` | `device_id`, `sequence`, `payload_bytes` |
| `DEBUG` | `edgesentry_rs::ingest::storage` | `ingest started` | `device_id`, `sequence`, `object_ref`, `payload_bytes` |
| `WARN`  | `edgesentry_rs::ingest::storage` | `payload hash mismatch — record rejected` | `device_id`, `sequence` |
| `WARN`  | `edgesentry_rs::ingest::storage` | `integrity policy rejected record` | `device_id`, `sequence`, `reason` |
| `ERROR` | `edgesentry_rs::ingest::storage` | `raw data store write failed` | `device_id`, `sequence`, `error` |
| `ERROR` | `edgesentry_rs::ingest::storage` | `audit ledger append failed` | `device_id`, `sequence`, `error` |
| `ERROR` | `edgesentry_rs::ingest::storage` | `operation log write failed` | `device_id`, `sequence`, `error` |
| `INFO`  | `edgesentry_rs::ingest::storage` | `record accepted` | `device_id`, `sequence`, `object_ref` |
| `DEBUG` | `edgesentry_rs::ingest::verify` | `signature verification failed` | `device_id`, `sequence` |
| `DEBUG` | `edgesentry_rs::ingest::verify` | `duplicate record rejected` | `device_id`, `sequence` |
| `DEBUG` | `edgesentry_rs::ingest::verify` | `sequence out of order` | `device_id`, `expected`, `actual` |
| `DEBUG` | `edgesentry_rs::ingest::verify` | `prev_record_hash mismatch — chain broken` | `device_id`, `sequence` |
| `DEBUG` | `edgesentry_rs::ingest::verify` | `record verified and accepted` | `device_id`, `sequence` |

### 推奨 Prometheus メトリクス（ログから導出）

ログ→メトリクスパイプライン（Promtail + Loki、または Vector など）を使い、構造化ログイベントからカウンターを導出します：

| メトリクス | 導出方法 | アラート閾値 |
|-----------|---------|------------|
| `edgesentry_ingest_accepted_total` | `INFO "record accepted"` イベントをカウント | — |
| `edgesentry_ingest_rejected_total{reason}` | `WARN` 拒否イベントを `reason` フィールドでラベル付けしてカウント | 10件/分を継続 → P1 アラート |
| `edgesentry_ingest_error_total{component}` | `ERROR` ストレージ障害イベントをコンポーネント別にカウント | 1件でも発生 → P0 アラート |
| `edgesentry_chain_break_total` | `DEBUG "prev_record_hash mismatch"` イベントをカウント | 1件でも発生 → P0 アラート |
| `edgesentry_signature_fail_total` | `DEBUG "signature verification failed"` イベントをカウント | 5件/分を継続 → P1 アラート |

### OpenTelemetry（トレーシングスパン）

`IngestService::ingest` メソッドは `tracing` スパンを出力します。OTLP エクスポーターに接続することで分散トレーシングが利用できます：

```toml
opentelemetry = "0.26"
opentelemetry-otlp = { version = "0.26", features = ["grpc-tonic"] }
tracing-opentelemetry = "0.27"
```

---

## アラート定義

| アラート | 条件 | 重要度 | 対応 |
|---------|------|--------|------|
| `IngestStorageError` | `ERROR` レベルのストレージ障害が発生 | P0 | DB/S3 接続を確認し、ディスクと認証情報を検証する |
| `ChainBreak` | `prev_record_hash mismatch` イベントが発生 | P0 | 改ざんまたはリプレイを調査し、再起動前にログを保全する |
| `HighRejectionRate` | 拒否率が 5 分間 10件/分を超過 | P1 | デバイスファームウェアを確認し、署名鍵のローテーション設定を調査する |
| `SignatureFailureSurge` | 署名失敗が 5 分間 5件/分を超過 | P1 | 鍵の漏洩またはアクティブなスプーフィングの可能性を調査する |
| `AuditLedgerLag` | Postgres `operation_logs` 挿入レイテンシの p99 が 2 秒超 | P1 | DBのクエリプランと autovacuum の競合を確認する |

---

## 復旧目標

| 目標 | ターゲット | 根拠 |
|------|-----------|------|
| RTO（復旧時間目標） | 30 分以内 | pg_basebackup + WAL リプレイによるリストア時間 |
| RPO（復旧時点目標） | 5 分以内 | 5 分間隔の継続的 WAL アーカイブ |

---

## バックアップ手順

### PostgreSQL — 監査台帳・操作ログ

**前提条件：** WAL アーカイブが有効化されていること（`archive_mode = on`、`archive_command` で S3 等に転送）。

#### 1. ベースバックアップの取得

```bash
pg_basebackup \
  --host=<DB_HOST> \
  --username=<DB_USER> \
  --pgdata=/backup/pg_base_$(date +%Y%m%d_%H%M%S) \
  --format=tar \
  --gzip \
  --wal-method=stream \
  --checkpoint=fast \
  --progress
```

#### 2. バックアップの検証

```bash
pg_restore --list /backup/pg_base_<timestamp>/base.tar.gz | head -20
```

#### 3. WAL の継続アーカイブ

`postgresql.conf` の `archive_command` が WAL セグメントを耐久性のあるストレージ（例：S3）に転送していることを確認します：

```
archive_command = 'aws s3 cp %p s3://<BUCKET>/wal/%f'
```

#### 4. 保持ポリシー

| バックアップ種別 | 保持期間 |
|----------------|---------|
| ベースバックアップ | 30 日 |
| WAL アーカイブ | 30 日 |
| 論理ダンプ（`pg_dump`） | 7 日（週次） |

---

### S3 / MinIO — ペイロード生データストア

バケットで**バージョニング**と**クロスリージョンレプリケーション**を有効化します：

```bash
# バージョニングの有効化
aws s3api put-bucket-versioning \
  --bucket <BUCKET> \
  --versioning-configuration Status=Enabled

# レプリケーションの有効化（宛先バケットと IAM ロールを別途設定した上で）
aws s3api put-bucket-replication \
  --bucket <BUCKET> \
  --replication-configuration file://replication.json
```

最低限のレプリケーション先：別リージョン 1 か所。CLS Level 3 のエビデンス完全性を確保するため、オブジェクトロックまたはバージョニングを有効化し、ペイロードが上書き削除されないようにします。

---

## リストア手順

### PostgreSQL — ポイントインタイムリカバリ（PITR）

```bash
# 1. Postgres サービスを停止
systemctl stop postgresql

# 2. ベースバックアップを展開
tar -xzf /backup/pg_base_<timestamp>/base.tar.gz -C /var/lib/postgresql/data/

# 3. リカバリ設定を作成
cat > /var/lib/postgresql/data/recovery.conf <<EOF
restore_command = 'aws s3 cp s3://<BUCKET>/wal/%f %p'
recovery_target_time = '<TARGET_TIMESTAMP>'
recovery_target_action = 'promote'
EOF

# 4. Postgres を起動 — 指定時刻まで WAL をリプレイする
systemctl start postgresql

# 5. 確認：デバイスごとの最終受理シーケンスを確認
psql -U <DB_USER> -d <DB_NAME> \
  -c "SELECT device_id, MAX(sequence) FROM audit_records GROUP BY device_id;"
```

#### 復旧確認チェックリスト

- [ ] デバイスごとの最終シーケンスがインシデント前のスナップショットと一致する
- [ ] ハッシュチェーンの連続性を確認：`eds verify-chain <exported-records.json>`
- [ ] 操作ログにリカバリ対象時刻前後のギャップがないことを確認する
- [ ] 確認完了後、アラート抑制を解除する

### S3 / MinIO — オブジェクトのリストア

```bash
# 特定バージョンのオブジェクトをリストア
aws s3api get-object \
  --bucket <BUCKET> \
  --key <OBJECT_KEY> \
  --version-id <VERSION_ID> \
  <OUTPUT_FILE>
```

---

## 障害訓練スケジュール

以下の訓練を四半期ごとに実施し、ランブックの正確性を検証します：

| 訓練 | 手順 | 合格基準 |
|------|------|---------|
| DB フェイルオーバー | プライマリ Postgres を停止してレプリカを昇格 | 30 分以内にインジェストが再開 |
| DB リストア | ステージング環境で 1 時間前への PITR を実施 | 30 分以内にチェーン連続性を確認 |
| S3 オブジェクト復旧 | 削除したテストオブジェクトをリストア | バイト単位で元のオブジェクトと一致 |
| アラート発火 | テストハーネスで不正な署名を注入 | 2 分以内に P1 アラートが発火 |
