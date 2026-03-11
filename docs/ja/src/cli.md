# CLI リファレンス

## デバイスプロビジョニング

新しいデバイス用に Ed25519 キーペアを生成：

```bash
cargo run -p edgesentry-rs -- keygen
```

ファイルに直接保存：

```bash
cargo run -p edgesentry-rs -- keygen --out device-lift-01.key.json
```

既存の秘密鍵から公開鍵を導出：

```bash
cargo run -p edgesentry-rs -- inspect-key \
  --private-key-hex 0101010101010101010101010101010101010101010101010101010101010101
```

完全なプロビジョニングおよびローテーションのワークフローについては[鍵管理](key_management.md)を参照してください。

---

## CLI の使い方

ビルドしてヘルプを表示：

```bash
cargo run -p edgesentry-rs -- --help
```

署名済みレコードを作成して`record1.json`に保存：

```bash
cargo run -p edgesentry-rs -- sign-record \
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
cargo run -p edgesentry-rs -- verify-record \
  --record-file record1.json \
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c
```

JSON 配列ファイルからチェーン全体を検証：

```bash
cargo run -p edgesentry-rs -- verify-chain --records-file records.json
```

## エレベーター点検シナリオ（ CLI エンドツーエンド）

このシナリオは 3 つのチェックによるリモートエレベーター点検をシミュレートします。

1. ドア開閉サイクルチェック
2. 振動チェック
3. 非常ブレーキ応答チェック

### 1) 1 回の点検セッション用の署名済みチェーン全体を生成する

```bash
cargo run -p edgesentry-rs -- demo-lift-inspection \
  --device-id lift-01 \
  --out-file lift_inspection_records.json
```

期待される出力：

```text
DEMO_CREATED:lift_inspection_records.json
CHAIN_VALID
```

### 2) ファイルからチェーンのインテグリティを検証する

```bash
cargo run -p edgesentry-rs -- verify-chain --records-file lift_inspection_records.json
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
cargo run -p edgesentry-rs -- verify-chain --records-file lift_inspection_records.json
```

期待される結果：コマンドが非ゼロコードで終了し、`chain verification failed: invalid previous hash ...`のようなエラーを出力します。

### 3) 1 件の署名済み点検イベントを作成して検証する

1 件の署名済みイベントを生成：

```bash
cargo run -p edgesentry-rs -- sign-record \
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
cargo run -p edgesentry-rs -- verify-record \
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
cargo run -p edgesentry-rs -- verify-record \
  --record-file lift_single_record.json \
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c
```

期待される出力：

```text
INVALID
```

---

## インジェストデモ（ PostgreSQL + MinIO ）

`s3`および`postgres`の Cargo フィーチャーと、実行中の PostgreSQL + MinIO インスタンスが必要です（`docker compose -f docker-compose.local.yml up -d`を使用）。

### 1) ペイロードファイル付きのチェーンを生成する

```bash
cargo run -p edgesentry-rs --features s3,postgres -- demo-lift-inspection \
  --device-id lift-01 \
  --out-file lift_inspection_records.json \
  --payloads-file lift_inspection_payloads.json
```

### 2) IngestService 経由でレコードをインジェストする

```bash
cargo run -p edgesentry-rs --features s3,postgres -- demo-ingest \
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
