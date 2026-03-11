# ライブラリ使用例

ライブラリ API を直接使って実装されたエンドツーエンドのエレベーター点検サンプルを実行します。

前提条件：

- Rust ツールチェーン（`cargo`）
- このサンプルには PostgreSQL / MinIO は **不要** です（インメモリストアを使用）

```bash
cargo run -p edgesentry-rs --example lift_inspection_flow
```

サンプルが対象とするシナリオ：

1. `IntegrityPolicyGate`に 1 台のエレベーターデバイスの公開鍵を登録する
2. `build_signed_record`で 3 件の署名済み点検レコードを生成する
3. `IngestService`経由ですべてのレコードをインジェストする（承認パス）
4. 1 件のレコードを改ざん（`payload_hash`）して拒否されることを確認する
5. 保存された監査レコードと操作ログを出力する

デモの内容：

- `edgesentry_rs::build_signed_record`によるレコード署名
- `edgesentry_rs::ingest::IngestService`によるインジェスト検証
- 改ざん拒否（変更された`payload_hash`）
- 監査レコードと操作ログの出力

ソース：

- `crates/edgesentry-rs/examples/lift_inspection_flow.rs`

---

## 3 ロール分散デモ

エッジからクラウドへのフローをより現実的に確認するために、 3 つのサンプルを順番に実行できます。それぞれのサンプルはちょうど 1 つのロールを担います。

| サンプル | ロール | 外部依存 |
|---------|------|--------------|
| `edge_device` | レコードに署名し`/tmp/eds_*.json`に書き込む | なし |
| `edge_gateway` | レコードをルーティングするが暗号検証は行わない | なし |
| `cloud_backend` | NetworkPolicy + IngestService + ストレージ | なし（インメモリ）または PostgreSQL + MinIO （`--features s3,postgres`） |

順番に実行：

```bash
cargo run -p edgesentry-rs --example edge_device
cargo run -p edgesentry-rs --example edge_gateway
cargo run -p edgesentry-rs --example cloud_backend
```

各サンプルは前のサンプルの出力ファイルを`/tmp/`から読み込みます。実際のバックエンドを使用したフル実行（ Docker が必要 — [インタラクティブデモ](demo.md)を参照）：

```bash
cargo run -p edgesentry-rs --example edge_device
cargo run -p edgesentry-rs --example edge_gateway
cargo run -p edgesentry-rs --features s3,postgres --example cloud_backend
```

このシーケンスのデモ内容：

- `edge_device` — `build_signed_record`によるデバイス側署名。拒否デモ用の改ざんコピーも書き込む
- `edge_gateway` — ゲートウェイはレコードを受信するが署名を検証しない（ルーティング専任）
- `cloud_backend` — `NetworkPolicy::check`がすべての`IngestService::ingest`の前に実行される。承認・拒否されたレコードの両方が確認できる

ソース：

- `crates/edgesentry-rs/examples/edge_device.rs`
- `crates/edgesentry-rs/examples/edge_gateway.rs`
- `crates/edgesentry-rs/examples/cloud_backend.rs`

---

## S3 / MinIO の切り替え

`edgesentry-rs`は`s3`フィーチャーフラグで切り替え可能な S3 互換生データバックエンドをサポートしています。

- `S3Backend::AwsS3`： AWS S3 を使用（デフォルトの AWS 認証情報チェーン、またはオプションの静的キー）
- `S3Backend::Minio`： MinIO を使用（カスタムエンドポイント + 静的アクセスキー/シークレット）

インジェストレイヤーは共通の生データストレージ抽象化に対してコーディングされており、具体的な設定によってインジェストのビジネスロジックを変更せずに AWS S3 または MinIO を選択します。

`edgesentry_rs`から以下の型を使用します。

- `S3ObjectStoreConfig::for_aws_s3(...)`
- `S3ObjectStoreConfig::for_minio(...)`
- `S3CompatibleRawDataStore::new(config)`

S3 フィーチャーを有効にしてビルド・テスト：

```bash
cargo test -p edgesentry-rs --features s3
```

ライブ MinIO インスタンスに対して S3 統合テストを実行するには、環境変数を設定して専用のテストファイルを実行します。

```bash
TEST_S3_ENDPOINT=http://localhost:9000 \
TEST_S3_ACCESS_KEY=minioadmin \
TEST_S3_SECRET_KEY=minioadmin \
TEST_S3_BUCKET=bucket \
cargo test -p edgesentry-rs --features s3 --test integration -- --nocapture
```

4 つの`TEST_S3_*`変数のいずれかが未設定の場合、テストは自動的にスキップされます。
