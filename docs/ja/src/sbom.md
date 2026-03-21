# SBOM とベンダー開示チェックリスト

このページは、シンガポール CLS Level 3 審査における IMDA IoT サイバーセキュリティガイドのライフサイクルサポートエビデンス要件を満たすものです。SBOM フォーマット、生成手順、および 5 つの必須カテゴリーに関するベンダー開示チェックリストの回答を掲載しています。

---

## ソフトウェア部品表（SBOM）

### フォーマット

EdgeSentry-RS は [CycloneDX](https://cyclonedx.org/) JSON 形式（仕様バージョン 1.3）の SBOM をリリース時に `Cargo.lock` から [`cargo-cyclonedx`](https://crates.io/crates/cargo-cyclonedx) を使用して生成・公開しています。

### 公開アーティファクト

各 GitHub Release には 2 つの SBOM ファイルがリリースアセットとして含まれます。リリースページからダウンロードできます：

```
https://github.com/edgesentry/edgesentry-rs/releases/tag/v<version>
```

| ファイル | スコープ |
|---------|---------|
| `edgesentry-rs-<version>.cdx.json` | `edgesentry-rs` クレートおよびすべての推移的依存関係 |
| `edgesentry-bridge-<version>.cdx.json` | `edgesentry-bridge` C/C++ FFI クレートとその依存関係 |

例として v0.1.2 の場合：
- `https://github.com/edgesentry/edgesentry-rs/releases/download/v0.1.2/edgesentry-rs-0.1.2.cdx.json`
- `https://github.com/edgesentry/edgesentry-rs/releases/download/v0.1.2/edgesentry-bridge-0.1.2.cdx.json`

### SBOM のローカル生成手順

```bash
cargo install cargo-cyclonedx --locked
cargo cyclonedx --format json --all
# 出力: crates/edgesentry-rs/edgesentry-rs.cdx.json
#       crates/edgesentry-bridge/edgesentry-bridge.cdx.json
```

### 依存コンポーネント数の確認方法

依存関係の更新のたびに変化するため、生成後に以下で現在の数を確認してください：

```bash
cargo cyclonedx --format json --all
python3 -c "
import json
for f in ['crates/edgesentry-rs/edgesentry-rs.cdx.json',
          'crates/edgesentry-bridge/edgesentry-bridge.cdx.json']:
    bom = json.load(open(f))
    print(f\"{f}: {len(bom.get('components', []))} components\")
"
```

### 継続的なサプライチェーン監視

- **`cargo-audit`** — すべての CI ビルドと PR で実行。[RustSec Advisory Database](https://rustsec.org/) に対してすべての依存関係をチェック
- **`cargo-deny`** — すべての CI ビルドでライセンスポリシーと禁止事項を強制
- **Dependabot** — 週次の依存関係バージョン更新 PR を自動作成

---

## ベンダー開示チェックリスト

IMDA IoT サイバーセキュリティガイドは 5 つのカテゴリーにわたる回答を要求しています。以下の表は EdgeSentry-RS の各カテゴリーにおける状況を文書化したものです。

### 1. 暗号化サポート

| 項目 | 回答 |
|------|------|
| 使用アルゴリズム | Ed25519（署名）、BLAKE3（ハッシュ） |
| 鍵長 | Ed25519: 256 ビット；BLAKE3 出力: 256 ビット |
| 乱数生成 | `rand::OsRng` 経由の OS CSPRNG — カスタム RNG なし |
| 転送暗号化 | レコードレベル：ペイロードハッシュへの Ed25519 署名。トランスポート層 TLS はデプロイ側の責務（計画中：[#73](https://github.com/edgesentry/edgesentry-rs/issues/73)） |
| 鍵の保管 | パブリックキーはメモリ内レジストリ（`IntegrityPolicyGate`）；プライベートキーはデプロイ側が管理。HSM 対応は計画中：[#54](https://github.com/edgesentry/edgesentry-rs/issues/54) |
| 実装 | `crates/edgesentry-rs/src/identity.rs`、`crates/edgesentry-rs/src/integrity.rs` |

### 2. 識別と認証

| 項目 | 回答 |
|------|------|
| デバイス認証方式 | Ed25519 非対称鍵ペア：デバイスが各レコードに秘密鍵で署名し、クラウドが登録済み公開鍵で検証 |
| 認証情報の保管 | 秘密鍵はデバイス上にのみ保管；公開鍵は `IntegrityPolicyGate::register_device` でクラウド側に登録 |
| デフォルト認証情報 | なし — 各デバイスが `eds keygen` でユニークなキーペアを生成 |
| ブルートフォース対策 | 署名検証は単一の定時間演算；認証情報ベースのログイン面は存在しない |
| ルート同一性強制 | `IngestService::ingest` の `cert_identity` パラメーター — TLS クライアント証明書の同一性と `record.device_id` の不一致は即時拒否 |
| 実装 | `crates/edgesentry-rs/src/identity.rs`、`crates/edgesentry-rs/src/ingest/policy.rs` |

### 3. データ保護

| 項目 | 回答 |
|------|------|
| 転送中のデータ | すべての `AuditRecord` が BLAKE3 ペイロードハッシュへの Ed25519 署名を保持 — トランスポートに関わらずレコードレベルの真正性を保証 |
| 保存データ | 生ペイロードは `RawDataStore`（S3/MinIO）経由で保管；監査レコードは `AuditLedger`（PostgreSQL）。保存時の暗号化はデプロイ側の責務（S3 SSE、Postgres 列暗号化） |
| 個人データ | `AuditRecord` は設計上個人データフィールドを持たない — `object_ref` はストレージキーへの参照；ペイロード本体は別途保管 |
| データ最小化 | 監査メタデータ（`payload_hash`、`signature`、`prev_record_hash`）とペイロード本体を分離 — クラウドはハッシュチェーンのみ保管；生データは `object_ref` 経由で独立して保管 |
| 実装 | `crates/edgesentry-rs/src/record.rs`、`crates/edgesentry-rs/src/ingest/storage.rs` |

### 4. ネットワーク保護

| 項目 | 回答 |
|------|------|
| 不要なポート/サービス | ライブラリのみ — `edgesentry-rs` はネットワークサービスを直接開放しない。トランスポートはデプロイ側の責務 |
| デフォルト拒否ネットワークポリシー | `NetworkPolicy` が IP/CIDR アローリストを強制；`check(source_ip)` は暗号演算の前に呼び出され、リスト外のすべての送信元を拒否 |
| DoS 耐性 | `NetworkPolicy` ゲートがリスト外送信元を暗号処理前に拒否し、攻撃面を制限。完全なレート制限はデプロイ側の責務 |
| 実装 | `crates/edgesentry-rs/src/ingest/network_policy.rs` |
| CLS 参照 | CLS-06 / ETSI EN 303 645 §5.6 |

### 5. ライフサイクルサポート

| 項目 | 回答 |
|------|------|
| 脆弱性報告 | GitHub プライベート脆弱性報告を有効化。[SECURITY.md](https://github.com/edgesentry/edgesentry-rs/blob/main/SECURITY.md) 参照 — SLA：承認 3 営業日；パッチ 30 日（重大/高）、90 日（中/低） |
| SBOM の提供 | 各 GitHub Release に CycloneDX JSON を添付（上記参照） |
| 依存関係のアドバイザリスキャン | `cargo-audit` を CI ビルドおよび PR ごとに RustSec Advisory DB に対して実行 |
| サポート終了ポリシー | `edgesentry-rs` v0.x：現行バージョンをサポート。セキュリティ更新はパッチリリースで提供 |
| ソフトウェア更新の完全性 | `UpdateVerifier` が更新適用前に BLAKE3 ペイロードハッシュと Ed25519 パブリッシャー署名を確認 — [CLS-03](traceability.md) 参照 |
| 対応バージョン | [SECURITY.md](https://github.com/edgesentry/edgesentry-rs/blob/main/SECURITY.md#supported-versions) 参照 |
| CLS 参照 | CLS-02 / ETSI EN 303 645 §5.2 |

---

## トレーサビリティ

このドキュメントは[ロードマップ](roadmap.md)のマイルストーン 1.4 を満たします。条項別のコンプライアンスマッピングの詳細は[コンプライアンス・トレーサビリティマトリックス](traceability.md)を参照してください。
