# CLI リファレンス

`eds` は EdgeSentry の統合 CLI です。監査コマンドはすべて `eds audit` サブコマンド配下にあり、スキャン点検コマンドは `eds inspect` 配下にあります。

```
eds audit <command>    — 改ざん検知付き監査レコード操作
eds inspect <command>  — 3D スキャン vs. IFC 偏差・AI 検出パイプライン
```

---

## インストール

### エンドユーザー向け — ビルド済みバイナリ

最新リリースを [GitHub Releases ページ](https://github.com/edgesentry/edgesentry-rs/releases) からダウンロードしてください。

| プラットフォーム | ファイル |
|----------------|---------|
| Linux (x86-64) | `eds-{version}-x86_64-unknown-linux-gnu.tar.gz` |
| macOS (Apple Silicon) | `eds-{version}-aarch64-apple-darwin.tar.gz` |
| Windows (x86-64) | `eds-{version}-x86_64-pc-windows-msvc.zip` |

展開して `eds` バイナリを `PATH` に追加してください：

```bash
# Linux / macOS
tar -xzf eds-{version}-{target}.tar.gz
sudo mv eds /usr/local/bin/
eds --help
```

```powershell
# Windows（PowerShell）
Expand-Archive eds-{version}-x86_64-pc-windows-msvc.zip
# eds.exe を PATH が通ったディレクトリに移動してください
eds --help
```

### 開発者向け — ソースからインストール

[Rust](https://rustup.rs)（stable ツールチェーン）が必要です。

```bash
cargo install --git https://github.com/edgesentry/edgesentry-rs --locked --bin eds
```

オプションのトランスポートフィーチャーを含めてインストールする場合：

```bash
cargo install --git https://github.com/edgesentry/edgesentry-rs --locked --bin eds \
  --features transport-http,transport-tls
```

インストールの確認：

```bash
eds --version
eds --help
```

---

## デバイスプロビジョニング

新しいデバイス用に Ed25519 キーペアを生成：

```bash
eds audit keygen
```

ファイルに直接保存：

```bash
eds audit keygen --out device-lift-01.key.json
```

既存の秘密鍵から公開鍵を導出：

```bash
eds audit inspect-key \
  --private-key-hex 0101010101010101010101010101010101010101010101010101010101010101
```

完全なプロビジョニングおよびローテーションのワークフローについては[鍵管理](key_management.md)を参照してください。

---

## CLI の使い方

ヘルプを表示：

```bash
eds --help
eds audit --help
```

署名済みレコードを作成して`record1.json`に保存：

```bash
eds audit sign-record \
  --device-id lift-01 \
  --sequence 1 \
  --timestamp-ms 1700000000000 \
  --payload "door-open" \
  --object-ref "s3://bucket/lift-01/1.bin" \
  --private-key-hex 0101010101010101010101010101010101010101010101010101010101010101 \
  --out record1.json
```

1 件のレコードの署名を検証：

```bash
eds audit verify-record \
  --record-file record1.json \
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c
```

JSON 配列ファイルからチェーン全体を検証：

```bash
eds audit verify-chain --records-file records.json
```

## エレベーター点検シナリオ（ CLI エンドツーエンド）

このシナリオは 3 つのチェックによるリモートエレベーター点検をシミュレートします。

1. ドア開閉サイクルチェック
2. 振動チェック
3. 非常ブレーキ応答チェック

### 1) 1 回の点検セッション用の署名済みチェーン全体を生成する

```bash
eds audit demo-lift-inspection \
  --device-id lift-01 \
  --out-file lift_inspection_records.json
```

期待される出力：

```text
DEMO_CREATED:lift_inspection_records.json
CHAIN_VALID
```

### 2) ファイルからチェーンの完全性を検証する

```bash
eds audit verify-chain --records-file lift_inspection_records.json
```

期待される出力：

```text
CHAIN_VALID
```

### 2.1) チェーンファイルを改ざんして検知されることを確認する

最初のレコードのハッシュ値をインプレースで変更：

```bash
python3 - <<'PY'
import json

path = "lift_inspection_records.json"
with open(path, "r", encoding="utf-8") as f:
  records = json.load(f)

records[0]["payload_hash"][0] ^= 0x01

with open(path, "w", encoding="utf-8") as f:
  json.dump(records, f, indent=2)
print("tampered", path)
PY
```

再度チェーン検証を実行：

```bash
eds audit verify-chain --records-file lift_inspection_records.json
```

期待される結果：コマンドが非ゼロコードで終了し、`chain verification failed: invalid previous hash ...`のようなエラーを出力します。

### 3) 1 件の署名済み点検イベントを作成して検証する

1 件の署名済みイベントを生成：

```bash
eds audit sign-record \
  --device-id lift-01 \
  --sequence 1 \
  --timestamp-ms 1700000000000 \
  --payload "scenario=lift-inspection,check=door,status=ok" \
  --object-ref "s3://bucket/lift-01/door-check-1.bin" \
  --private-key-hex 0101010101010101010101010101010101010101010101010101010101010101 \
  --out lift_single_record.json
```

署名を検証：

```bash
eds audit verify-record \
  --record-file lift_single_record.json \
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c
```

期待される出力：

```text
VALID
```

### 3.1) 1 件のレコードの署名を改ざんして拒否されることを確認する

署名の 1 バイトを変更：

```bash
python3 - <<'PY'
import json

path = "lift_single_record.json"
with open(path, "r", encoding="utf-8") as f:
  record = json.load(f)

record["signature"][0] ^= 0x01

with open(path, "w", encoding="utf-8") as f:
  json.dump(record, f, indent=2)
print("tampered", path)
PY
```

再度署名を検証：

```bash
eds audit verify-record \
  --record-file lift_single_record.json \
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c
```

期待される出力：

```text
INVALID
```

---

## サーバーコマンド

### `eds audit serve` — HTTP インジェストサーバー

`transport-http` Cargo フィーチャーが必要です。

| フラグ | デフォルト | 説明 |
|--------|-----------|------|
| `--addr` | `0.0.0.0:8080` | バインドするソケットアドレス |
| `--allowed-sources` | `127.0.0.1` | 接続を許可する CIDR / IP のカンマ区切りリスト |
| `--device ID=PUBKEY_HEX` | _（なし）_ | デバイスを登録；複数デバイスは繰り返し指定 |

```bash
eds audit serve \
  --addr 0.0.0.0:8080 \
  --allowed-sources 10.0.0.0/8 \
  --device lift-01=<PUBLIC_KEY_HEX>
```

ポート 8080 でプレーン HTTP を提供します。TLS 終端リバースプロキシの背後で使用するか、組み込み TLS には `eds audit serve-tls` を使用してください。

---

### `eds audit serve-tls` — HTTPS インジェストサーバー（TLS 1.2/1.3）

`transport-tls` Cargo フィーチャーが必要です。

| フラグ | デフォルト | 説明 |
|--------|-----------|------|
| `--addr` | `0.0.0.0:8443` | バインドするソケットアドレス |
| `--allowed-sources` | `127.0.0.1` | 接続を許可する CIDR / IP のカンマ区切りリスト |
| `--device ID=PUBKEY_HEX` | _（なし）_ | デバイスを登録；複数デバイスは繰り返し指定 |
| `--tls-cert` | _（必須）_ | PEM 証明書チェーンのパス（リーフ証明書を先頭に） |
| `--tls-key` | _（必須）_ | PEM 秘密鍵のパス（PKCS #8 または PKCS #1 RSA） |

```bash
eds audit serve-tls \
  --addr 0.0.0.0:8443 \
  --allowed-sources 10.0.0.0/8 \
  --device lift-01=<PUBLIC_KEY_HEX> \
  --tls-cert /etc/edgesentry/server.crt \
  --tls-key  /etc/edgesentry/server.key
```

rustls の TLS 1.2/1.3 を使用します。ネットワークポリシー（IP アローリスト）は TLS ハンドシェイク前の TCP 接続受け入れ時点で適用されます。

---

### `eds audit serve-mqtt` — MQTT インジェストサブスクライバー

`transport-mqtt` Cargo フィーチャーが必要です。MQTTS には `transport-mqtt-tls` も追加してください。

| フラグ | デフォルト | 説明 |
|--------|-----------|------|
| `--broker` | `localhost` | MQTT ブローカーホスト |
| `--port` | `1883` | MQTT ブローカーポート（MQTTS には `8883` を使用） |
| `--topic` | `edgesentry/ingest` | インジェストレコードを受信するトピック |
| `--client-id` | `eds-server` | MQTT クライアント識別子 |
| `--device ID=PUBKEY_HEX` | _（なし）_ | デバイスを登録；複数デバイスは繰り返し指定 |
| `--tls-ca-cert` | _（なし）_ | MQTTS ブローカー検証用 PEM CA 証明書のパス（`transport-mqtt-tls` のみ） |

```bash
# プレーン MQTT（ポート 1883）
eds audit serve-mqtt \
  --broker broker.example.com \
  --port 1883 \
  --topic edgesentry/ingest \
  --device lift-01=<PUBLIC_KEY_HEX>

# MQTTS（ポート 8883、transport-mqtt-tls フィーチャーが必要）
eds audit serve-mqtt \
  --broker broker.example.com \
  --port 8883 \
  --tls-ca-cert /etc/edgesentry/ca.crt \
  --device lift-01=<PUBLIC_KEY_HEX>
```

レスポンスは `<topic>/response` に `status: "accepted"` または `status: "rejected"` の JSON として発行されます。

---

## インジェストデモ（ PostgreSQL + MinIO ）

`s3`および`postgres`の Cargo フィーチャーと、実行中の PostgreSQL + MinIO インスタンスが必要です（`docker compose -f docker-compose.local.yml up -d`を使用）。

### 1) ペイロードファイル付きのチェーンを生成する

```bash
eds audit demo-lift-inspection \
  --device-id lift-01 \
  --out-file lift_inspection_records.json \
  --payloads-file lift_inspection_payloads.json
```

### 2) IngestService 経由でレコードをインジェストする

```bash
eds audit demo-ingest \
  --records-file lift_inspection_records.json \
  --payloads-file lift_inspection_payloads.json \
  --device-id lift-01 \
  --pg-url postgresql://trace:trace@localhost:5433/trace_audit \
  --minio-endpoint http://localhost:9000 \
  --minio-bucket bucket \
  --minio-access-key minioadmin \
  --minio-secret-key minioadmin \
  --reset
```

`--reset`はインジェスト前に`audit_records`と`operation_logs`を truncate します。既存の実行に追記するにはこのオプションを省略してください。

同じ`IngestService`を通じた改ざんされたチェーンの拒否もデモするには`--tampered-records-file <path>`を渡してください。

PostgreSQL と MinIO を使った完全なガイド付きウォークスルーは[インタラクティブデモ](demo.md)を参照してください。
