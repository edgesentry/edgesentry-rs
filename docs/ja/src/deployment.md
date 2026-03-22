# 本番デプロイガイド

このガイドでは、ローカル Docker Compose デモから `eds serve`（HTTP/TLS）および `eds serve-mqtt` の本番グレードデプロイへの移行について説明します。ローカルのクイックスタートは[インタラクティブデモ](demo.md)を、オブザーバビリティ・アラート・バックアップ/リストア手順については[運用ランブック](operations.md)を参照してください。

---

## 前提条件

| コンポーネント | 最小バージョン | 備考 |
|-----------|----------------|-------|
| edgesentry-rs バイナリ | 現行 `main` | HTTPS には `--features transport-http,transport-tls` でビルド；MQTT には `transport-mqtt` を追加 |
| PostgreSQL | 14 | 監査台帳および操作ログ |
| S3 互換ストア | — | AWS S3、MinIO ≥ RELEASE.2023、または Cloudflare R2 |
| （任意）MQTT ブローカー | Mosquitto ≥ 2.0 | `eds serve-mqtt` に必要な場合のみ |

---

## 1 — TLS 証明書管理

### 1.1 Let's Encrypt によるプロビジョニング（推奨）

```bash
# certbot をインストール
apt install certbot

# イングレストエンドポイント用の証明書を発行
certbot certonly --standalone \
  -d ingest.example.com \
  --agree-tos --non-interactive \
  -m ops@example.com

# 証明書の出力先：
#   /etc/letsencrypt/live/ingest.example.com/fullchain.pem  （証明書 + チェーン）
#   /etc/letsencrypt/live/ingest.example.com/privkey.pem    （秘密鍵）
```

### 1.2 TLS を有効にした `eds serve-tls` の起動

```bash
eds serve-tls \
  --addr 0.0.0.0:8443 \
  --tls-cert /etc/letsencrypt/live/ingest.example.com/fullchain.pem \
  --tls-key  /etc/letsencrypt/live/ingest.example.com/privkey.pem \
  --allowed-sources 10.0.0.0/8 \
  --device lift-01=<PUBLIC_KEY_HEX>
```

`eds serve-tls` は rustls 経由で TLS 1.2 最小・TLS 1.3 優先を適用します。追加設定は不要です。

### 1.3 証明書のローテーション（ゼロダウンタイム）

`eds serve-tls` は起動時にのみ証明書ファイルを読み込みます。ダウンタイムなしのローテーション手順：

```bash
# 1. 証明書を更新
certbot renew --quiet

# 2. 実行中プロセスに SIGTERM を送信（systemd が再起動を処理）
systemctl reload edgesentry
# — または systemd を使わない場合 —
kill -TERM $(pidof eds)
# プロセスが正常終了し、スーパーバイザー/systemd が再起動して新しい証明書を読み込む
```

cron/systemd タイマーを追加して更新を自動化：

```ini
# /etc/systemd/system/certbot.timer
[Timer]
OnCalendar=weekly
Persistent=true

[Install]
WantedBy=timers.target
```

```bash
systemctl enable --now certbot.timer
```

### 1.4 自己署名証明書（内部/エアギャップ環境）

```bash
# 10 年の自己署名証明書を生成
openssl req -x509 -newkey rsa:4096 -sha256 -days 3650 \
  -nodes -keyout server.key -out server.crt \
  -subj "/CN=ingest.internal" \
  -addext "subjectAltName=IP:10.0.1.5,DNS:ingest.internal"
```

`server.crt` を信頼 CA としてすべてのエッジデバイスに配布してください。

---

## 2 — PostgreSQL: スキーマ・インデックス・接続数設定

### 2.1 スキーママイグレーション

スキーマは [`db/init/001_schema.sql`](https://github.com/edgesentry/edgesentry-rs/blob/main/db/init/001_schema.sql) にあります。本番データベースに適用してください：

```bash
psql "$DATABASE_URL" -f db/init/001_schema.sql
```

スキーマは冪等（`CREATE TABLE IF NOT EXISTS`）であり、再実行しても安全です。

### 2.2 推奨インデックス

ベーススキーマには `UNIQUE (device_id, sequence)` 制約が含まれており、これが B-tree インデックスを兼ねてデータベースレベルでリプレイ攻撃を拒否します。一般的なクエリパターン向けに以下のインデックスを追加してください：

```sql
-- デバイスごとの最新レコード検索（チェーンヘッドクエリ）
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_audit_device_seq
    ON audit_records (device_id, sequence DESC);

-- コンプライアンスレポート用の時間範囲クエリ
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_audit_ingested_at
    ON audit_records (ingested_at);

-- 決定種別によるオペレーションログフィルタリング
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_oplog_decision_device
    ON operation_logs (decision, device_id, created_at DESC);
```

`CONCURRENTLY` を指定することで、本番環境でテーブルをロックせずにインデックスを作成できます。

### 2.3 接続プールのサイジング

`PostgresAuditLedger` と `PostgresOperationLog` はそれぞれ `postgres` クレート経由で 1 つの同期接続を開きます。マルチノードデプロイ（§5 参照）では各 `eds` プロセスが 2 つの接続を保持します。`postgresql.conf` の `max_connections` を以下のように設定してください：

```
max_connections = 2 × <eds インスタンス数> + 10   # psql・監視用のヘッドルーム
```

高速なイングレストレート（500 レコード/秒超）の場合、カスタム `AsyncAuditLedger` 実装として `sqlx` + `PgPool` を使用した非同期接続プールに置き換えてください。

### 2.4 長期保持のためのパーティショニング

テーブルが 1 億行を超えると予想される場合は、`ingested_at` で `audit_records` をパーティション分割してください：

```sql
-- 範囲パーティションテーブルに変換（データ蓄積前に一度実行）
CREATE TABLE audit_records_new (LIKE audit_records INCLUDING ALL)
    PARTITION BY RANGE (ingested_at);

CREATE TABLE audit_records_2026_q1
    PARTITION OF audit_records_new
    FOR VALUES FROM ('2026-01-01') TO ('2026-04-01');

-- アタッチ、スワップ、削除
ALTER TABLE audit_records RENAME TO audit_records_old;
ALTER TABLE audit_records_new RENAME TO audit_records;
DROP TABLE audit_records_old;
```

---

## 3 — オブジェクトストレージ: バケットポリシーとライフサイクルルール

### 3.1 AWS S3 — バケットポリシー（最小権限）

イングレストサービス専用の IAM ロールを書き込み専用アクセスで作成してください：

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "IngestWriteOnly",
      "Effect": "Allow",
      "Action": ["s3:PutObject"],
      "Resource": "arn:aws:s3:::edgesentry-audit/*"
    },
    {
      "Sid": "ListBucket",
      "Effect": "Allow",
      "Action": ["s3:ListBucket"],
      "Resource": "arn:aws:s3:::edgesentry-audit"
    }
  ]
}
```

コンプライアンス監査者には別の読み取り専用ロールを付与してください。

### 3.2 ライフサイクルルール（保持期間 + コスト管理）

```json
{
  "Rules": [
    {
      "Id": "TransitionToIA",
      "Status": "Enabled",
      "Filter": { "Prefix": "" },
      "Transitions": [
        { "Days": 90,  "StorageClass": "STANDARD_IA" },
        { "Days": 365, "StorageClass": "GLACIER_IR" }
      ]
    },
    {
      "Id": "ExpireOldObjects",
      "Status": "Enabled",
      "Filter": { "Prefix": "" },
      "Expiration": { "Days": 2555 }
    }
  ]
}
```

CLI で適用：

```bash
aws s3api put-bucket-lifecycle-configuration \
  --bucket edgesentry-audit \
  --lifecycle-configuration file://lifecycle.json
```

### 3.3 MinIO（オンプレミス）

```bash
# オブジェクトロック付きでバケットを作成（コンプライアンス向け不変性）
mc mb --with-lock minio/edgesentry-audit

# ライフサイクル設定: 90 日後に低コストティアへ移行
mc ilm import minio/edgesentry-audit <<EOF
{
  "Rules": [{
    "ID": "expire-3-years",
    "Status": "Enabled",
    "Expiration": { "Days": 1095 }
  }]
}
EOF

# 保存時のサーバーサイド暗号化
mc encrypt set sse-s3 minio/edgesentry-audit
```

---

## 4 — プロセス管理

### 4.1 systemd サービスユニット（HTTP + TLS）

```ini
# /etc/systemd/system/edgesentry.service
[Unit]
Description=EdgeSentry-RS ingest server
After=network-online.target postgresql.service
Wants=network-online.target

[Service]
Type=exec
User=edgesentry
Group=edgesentry
ExecStart=/usr/local/bin/eds serve-tls \
    --addr 0.0.0.0:8443 \
    --tls-cert /etc/edgesentry/server.crt \
    --tls-key  /etc/edgesentry/server.key \
    --allowed-sources 10.0.0.0/8 \
    --device lift-01=<PUBLIC_KEY_HEX>
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=edgesentry_rs=info

# セキュリティ強化
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/edgesentry
PrivateTmp=true
CapabilityBoundingSet=

[Install]
WantedBy=multi-user.target
```

```bash
# インストールと起動
install -m 755 target/release/eds /usr/local/bin/eds
useradd --system --no-create-home edgesentry
mkdir -p /var/log/edgesentry && chown edgesentry:edgesentry /var/log/edgesentry

systemctl daemon-reload
systemctl enable --now edgesentry
systemctl status edgesentry
```

### 4.2 systemd サービスユニット（MQTT）

```ini
# /etc/systemd/system/edgesentry-mqtt.service
[Unit]
Description=EdgeSentry-RS MQTT ingest subscriber
After=network-online.target mosquitto.service
Wants=network-online.target

[Service]
Type=exec
User=edgesentry
Group=edgesentry
ExecStart=/usr/local/bin/eds serve-mqtt \
    --broker 10.0.1.10 \
    --port 1883 \
    --topic edgesentry/ingest \
    --client-id eds-prod-1 \
    --device lift-01=<PUBLIC_KEY_HEX>
Restart=on-failure
RestartSec=10
Environment=RUST_LOG=edgesentry_rs=info

[Install]
WantedBy=multi-user.target
```

### 4.3 ヘルスチェック

`eds serve` 自体は `/health` エンドポイントを公開しません。ロードバランサーまたは監視エージェントで TCP チェックを設定してください：

```bash
# TLS ポートが接続を受け入れていることを確認
openssl s_client -connect ingest.example.com:8443 -verify_return_error </dev/null
echo $?   # 0 = 正常
```

Kubernetes では `tcpSocket` liveness probe を使用してください：

```yaml
livenessProbe:
  tcpSocket:
    port: 8443
  initialDelaySeconds: 5
  periodSeconds: 15
```

---

## 5 — 水平スケーリング

### 5.1 アーキテクチャ

```
                      ┌─────────────────┐
Edge devices  ──TLS──►│  ロードバランサー │
                      │  (nginx /       │
                      │   AWS ALB など) │
                      └────────┬────────┘
                               │  ラウンドロビン
                ┌──────────────┼──────────────┐
                ▼              ▼              ▼
         ┌────────────┐ ┌────────────┐ ┌────────────┐
         │  eds serve │ │  eds serve │ │  eds serve │
         │  ノード 1  │ │  ノード 2  │ │  ノード 3  │
         └──────┬─────┘ └──────┬─────┘ └──────┬─────┘
                └──────────────┼──────────────┘
                               │
                ┌──────────────┼──────────────┐
                ▼              ▼              ▼
         ┌─────────┐    ┌──────────┐   ┌─────────┐
         │Postgres │    │  S3 /    │   │ MinIO   │
         │（プライマリ）│    │  バケット │   │ クラスター │
         └─────────┘    └──────────┘   └─────────┘
```

### 5.2 主要な特性

- **`IngestState` はプロセスごと。** 各 `eds serve` ノードは独自のインメモリシーケンス/ハッシュチェーン状態を保持します。PostgreSQL の `UNIQUE (device_id, sequence)` 制約がクロスノードのリプレイフェンスとなり、重複インサートは unique-violation エラーとなって `PostgresAuditLedger` がストアエラーとして通知し、イングレストが拒否・ログ記録されます。
- **スティッキーセッション不要。** シーケンス強制は DB レベルで行われ、どのノードもどのデバイスのリクエストも処理できます。
- **S3/MinIO への書き込みはステートレス。** すべてのノードが同一バケットに書き込みます。オブジェクトキーは `object_ref` から導出され、エッジデバイスが設定し、慣例として（例：`<device_id>/<sequence>.bin`）グローバルに一意です。

### 5.3 nginx TLS 終端 + アップストリームプロキシ

```nginx
upstream edgesentry_nodes {
    least_conn;
    server 10.0.1.11:8080;
    server 10.0.1.12:8080;
    server 10.0.1.13:8080;
}

server {
    listen 443 ssl;
    server_name ingest.example.com;

    ssl_certificate     /etc/letsencrypt/live/ingest.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/ingest.example.com/privkey.pem;
    ssl_protocols       TLSv1.2 TLSv1.3;
    ssl_ciphers         HIGH:!aNULL:!MD5;

    location /api/v1/ingest {
        proxy_pass         http://edgesentry_nodes;
        proxy_set_header   X-Forwarded-For $remote_addr;
        proxy_read_timeout 10s;
    }
}
```

各ノードで `eds serve`（プライベートポートでのプレーン HTTP）を実行し、nginx に TLS 終端を任せてください。`--allowed-sources` には nginx アップストリームの IP 範囲を指定します。リバースプロキシを使わずに組み込み TLS を利用する場合は `eds serve-tls` を使用してください。

> **注意：** TLS がロードバランサーで終端される場合、`eds serve` はデバイスの IP ではなく LB の IP を認識します。`--allowed-sources` を LB の内部アドレス範囲に設定し、デバイスごとのソース制御は LB 側のアローリストに委ねてください。

### 5.4 レポート用 PostgreSQL リードレプリカ

書き込みパス（イングレスト）：プライマリのみ。
読み取りパス（コンプライアンスクエリ、チェーン検証）：リードレプリカに直接アクセス。

```bash
# コンプライアンスツール用のリードレプリカ接続
psql "postgres://audit_ro:pass@pg-replica:5432/audit?sslmode=require"
```

---

## 6 — オブザーバビリティ

構造化ログとトレーシングは `tracing` ファサードで処理されます。JSON ログフォーマット、ライブラリが出力する構造化イベントフィールド、Prometheus メトリクス導出、OpenTelemetry スパン設定を含む詳細なセットアップは[運用ランブック — オブザーバビリティ](operations.md#observability)を参照してください。

### クイックスタート: stdout への JSON ログ出力（Loki / CloudWatch 向け）

```toml
# バイナリラッパーの Cargo.toml
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

```bash
# JSON ログで eds を実行
RUST_LOG=edgesentry_rs=info eds serve ... 2>&1 | \
  promtail --stdin --client.url http://loki:3100/loki/api/v1/push
```

### アラート対象の主要ログフィールド

| フィールド | 値 | アラート条件 |
|-------|-------|----------------|
| `message` | `"MQTT record rejected"` / `"record rejected"` | 5 分間の拒否率 > 1 % |
| `reason` | `"invalid signature"` | 任意の発生 — 改ざんの試みの可能性 |
| `reason` | `"unknown device"` | 継続的な発生 — 未登録デバイスの探索 |
| `message` | `"MQTT event loop error"` | 任意の発生 — ブローカー接続が切断 |

Prometheus アラートルールについては[運用ランブック — アラート定義](operations.md#alert-definitions)を参照してください。

---

## 関連ドキュメント

- [インタラクティブデモ](demo.md) — ローカル Docker Compose クイックスタート
- [鍵管理](key_management.md) — デバイス鍵のプロビジョニングとローテーション
- [運用ランブック](operations.md) — オブザーバビリティ、バックアップ、リストア、障害訓練
- [CLI リファレンス](cli.md) — `eds serve`・`eds serve-mqtt`・全サブコマンドの完全なフラグリファレンス
