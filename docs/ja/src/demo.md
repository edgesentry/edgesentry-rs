# インタラクティブローカルデモ

注意：ライブラリのみのサンプルとは異なり、このデモには PostgreSQL と MinIO が**必要**です。

## 3 ロールモデル

EdgeSentry-RS は 3 つの明確なロールを中心に設計されています。デモ出力を正しく読むためには、各ステップがどのロールに属するかを理解することが重要です。

| ロール | 責務 | このデモでの担当 |
|------|---------------|-------------|
|**エッジデバイス**| Ed25519 秘密鍵で点検レコードに署名し、クラウドへ送出する | `examples/edge_device.rs` |
|**エッジゲートウェイ**| 署名済みレコードをデバイスから HTTPS/MQTT 経由でクラウドへ転送する。コンテンツは検証しない | `examples/edge_gateway.rs` — HTTP トランスポートはスコープ外。ディスク上のファイルがトランスポートをシミュレートする |
|**クラウドバックエンド**| `NetworkPolicy`（ CLS-06 ）を強制し、`IntegrityPolicyGate`（ルートアイデンティティ→署名→シーケンス→ハッシュチェーン）を実行し、承認されたレコードを永続化する | `examples/cloud_backend.rs`（`--features s3,postgres`付き） |

## このデモの内容

スクリプトは Docker サービスを起動し、 3 つのロールサンプルを順番に実行します。

| ステップ | ロール | 内容 |
|------|------|-------------|
| 1 〜 3 | インフラ | Docker Compose で PostgreSQL + MinIO を起動。ヘルスチェックを待機 |
| 4 | エッジデバイス | `edge_device` — 3 件のレコードに署名し`/tmp/eds_*.json`に書き込む |
| 5 | エッジゲートウェイ | `edge_gateway` — デバイスの出力を読み込み、変更せずに`/tmp/eds_fwd_*.json`へ転送する |
| 6 | クラウドバックエンド | `cloud_backend` — `NetworkPolicy`チェック → `IngestService` → PostgreSQL + MinIO 。改ざん拒否も表示 |
| 7 | クラウドバックエンド | PostgreSQL から永続化された監査レコードと操作ログを照会する |
| 8 | インフラ | Docker サービスを停止する |

前提条件：

- Docker / Docker Compose
- Rust ツールチェーン（`cargo`）

エンドツーエンドデモを実行：

```bash
bash scripts/local_demo.sh
```

スクリプトは各ステップの後に一時停止し、 Enter キー（または`OK`）を押すまで次のステップへ進みません。
フロー終了時にシャットダウンステップ（`docker compose -f docker-compose.local.yml down`）が実行されます。

## ロールサンプルの個別実行

各サンプルは Docker なしでスタンドアロンで実行することも可能です（クラウドバックエンドにインメモリストレージを使用）。

```bash
# ステップ1：エッジデバイスがレコードに署名する
cargo run -p edgesentry-rs --example edge_device

# ステップ2：エッジゲートウェイがレコードを転送する
cargo run -p edgesentry-rs --example edge_gateway

# ステップ3a：クラウドバックエンド（インメモリ — Docker不要）
cargo run -p edgesentry-rs --example cloud_backend

# ステップ3b：クラウドバックエンド（PostgreSQL + MinIO — Dockerが必要）
cargo run -p edgesentry-rs --features s3,postgres --example cloud_backend
```

各サンプルは前のサンプルの出力ファイルを`/tmp/`から読み込みます。順番に実行してください。

## 手動での確認

ステップ 6 の後に PostgreSQL に接続：

```bash
docker exec -it edgesentry-rs-postgres psql -U trace -d trace_audit
```

`psql`内で：

```sql
SELECT id, device_id, sequence, object_ref, ingested_at FROM audit_records ORDER BY sequence;
SELECT id, decision, device_id, sequence, message, created_at FROM operation_logs ORDER BY id;
```

MinIO のエンドポイント：

- API: `http://localhost:9000`
- コンソール: `http://localhost:9001`
- デフォルト認証情報: `minioadmin / minioadmin`
- セットアップコンテナが作成するバケット: `bucket`

ローカルバックエンドの手動停止（スクリプトを途中で中断した場合のみ）：

```bash
docker compose -f docker-compose.local.yml down
```
