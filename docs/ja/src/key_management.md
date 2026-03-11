# 鍵管理

このページでは、 EdgeSentry-RS が使用する Ed25519 デバイス鍵のライフサイクル全体を説明します。
鍵の生成・安全な保存・公開鍵の登録・ローテーションが対象です。

関連標準： Singapore CLS-04 / ETSI EN 303 645 §5.4 / JC-STAR STAR-1 R1.2 。

---

## 1. 鍵の生成

`eds` CLI を使って新しい Ed25519 キーペアを生成：

```bash
eds keygen
```

出力例：

```json
{
  "private_key_hex": "ddca9848801c658d62a010c4d306d6430a0cdc2c383add1628859258e3acfb93",
  "public_key_hex": "4bb158f302c0ad9261c0acfa95e17144ae7249eb0973bbfaeae4501165887a77"
}
```

ファイルに保存：

```bash
eds keygen --out device-lift-01.key.json
```

各デバイスは **一意の** キーペアを持たなければなりません。デバイスをまたいで鍵を再利用しないでください。

---

## 2. 既存の秘密鍵から公開鍵を導出する

`private_key_hex`をすでに持っており、対応する公開鍵を確認したい場合：

```bash
eds inspect-key --private-key-hex <64-hex-char-private-key>
```

例：

```bash
eds inspect-key \
  --private-key-hex 0101010101010101010101010101010101010101010101010101010101010101
```

出力：

```json
{
  "private_key_hex": "0101010101010101010101010101010101010101010101010101010101010101",
  "public_key_hex": "8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c"
}
```

---

## 3. 秘密鍵の安全な保存

秘密鍵はデバイス上で秘密にしておかなければなりません。推奨される実践：

| 環境 | 推奨ストレージ |
|-------------|---------------------|
| 開発 / CI | 環境変数（`DEVICE_PRIVATE_KEY_HEX`）— バージョン管理にコミットしないこと |
| 本番（ソフトウェア） | 暗号化されたシークレットストア（例： HashiCorp Vault 、 AWS Secrets Manager 、 Azure Key Vault ） |
| 本番（ハードウェア） | ハードウェアセキュリティモジュール（ HSM ）またはトラステッド実行環境（ TEE ）— 計画中の HSM パスについては[#54](https://github.com/edgesentry/edgesentry-rs/issues/54)を参照 |

ファイルベースのストレージ（開発環境のみ）：

```bash
chmod 600 device-lift-01.key.json
```

`private_key_hex`をログ・ HTTP レスポンス・エラーメッセージに含めないでください。

---

## 4. 公開鍵の登録（クラウド側）

キーペアを生成した後、レコードがインジェストされる前にデバイスの公開鍵を`IntegrityPolicyGate`に登録します：

```rust
use edgesentry_rs::{IntegrityPolicyGate, parse_fixed_hex};
use ed25519_dalek::VerifyingKey;

let public_key_bytes = parse_fixed_hex::<32>(&public_key_hex)?;
let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)?;

let mut gate = IntegrityPolicyGate::new();
gate.register_device("lift-01", verifying_key);
```

`register_device`に渡す`device_id`文字列は、そのデバイスが署名するすべての`AuditRecord`の`device_id`フィールドと完全に一致しなければなりません。

未知の`device_id`からのレコードは`IngestError::UnknownDevice`で拒否されます。

---

## 5. 鍵のローテーション

以下の場合にデバイス鍵をローテーションしてください。

- 秘密鍵が漏洩した可能性がある
- デバイスが廃棄されて再プロビジョニングされる
- セキュリティポリシーで定期的なローテーションが求められている

**ローテーション手順：**

1. 新しいデバイス設定用に新しいキーペアを生成：
   ```bash
   eds keygen --out device-lift-01-v2.key.json
   ```

2. 新しい公開鍵を古い鍵と並べて登録します（ゲートは`device_id`あたり複数の鍵をまだサポートしていません。移行ウィンドウ中は`lift-01-v2`のような新しい`device_id`で登録してください）。

3. デバイスを更新して、新しい秘密鍵と新しい`device_id`で新しいレコードに署名するようにします。

4. 古い鍵で署名されたすべての処理中のレコードがインジェストされて検証されたら、ポリシーゲートから古いデバイス登録を削除します。

5. すべてのストレージから古い秘密鍵を安全に削除または失効させます。

> **注意：** 同じ`device_id`で古い鍵と新しい鍵を同時に許可するマルチキー・パー・デバイスのサポートは[#57](https://github.com/edgesentry/edgesentry-rs/issues/57)で追跡されています。

---

## 6. ソフトウェアアップデートのパブリッシャー鍵

ソフトウェアアップデートの検証には、デバイス署名鍵とは別の Ed25519 鍵セットを使用します。 **パブリッシャー鍵**はファームウェアまたはソフトウェアパッケージに署名するエンティティに属し、**デバイス署名鍵** は監査レコードに署名する個々のデバイスに属します。これらのロールを混在させないでください。

### 6.1 鍵の生成と保存

デバイスキーペアと同じ方法でパブリッシャーキーペアを生成：

```bash
eds keygen --out publisher-acme-firmware.key.json
```

**秘密鍵** は高セキュリティのオフライン環境（ HSM ・エアギャップワークステーション、または厳格なアクセス制御付きのシークレットマネージャー）に保管しなければなりません。リリースアーティファクトへの署名のためにビルド時にのみ使用し、デバイス自体には置きません。

**公開鍵** は製造時にデバイスのファームウェアイメージに埋め込まれ、実行時に`UpdateVerifier`にロードされます：

```rust
use edgesentry_rs::update::UpdateVerifier;
use ed25519_dalek::VerifyingKey;

let public_key_bytes: [u8; 32] = /* bytes baked into firmware */;
let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)?;

let mut verifier = UpdateVerifier::new();
verifier.register_publisher("acme-firmware", verifying_key);
```

### 6.2 パブリッシャー ID と鍵は 1 対 1 で

各鍵を異なる`publisher_id`の下に登録してください。脅威モデルで明示的に要求されない限り、同一の鍵を複数の ID で登録したり、同一の ID で複数の鍵を登録したりすることは避けてください。

```rust
// 正しい：パブリッシャーごとに1つの鍵
verifier.register_publisher("acme-firmware", firmware_key);
verifier.register_publisher("acme-config",   config_key);

// 避けること：パブリッシャー間で鍵を共有すると、
// あるパッケージタイプの署名が別のタイプで受け入れられる可能性がある
verifier.register_publisher("acme-firmware", shared_key); // ⚠
verifier.register_publisher("acme-config",   shared_key); // ⚠
```

### 6.3 鍵混同攻撃

**鍵混同攻撃** は、あるパッケージタイプ向けに生成された署名が別のパッケージタイプの有効な署名として提出されるときに発生します。`UpdateVerifier`は以下の理由でこれを防ぎます：

1. 呼び出し元は`verify()`に明示的な`publisher_id`を渡す。
2. 検証器はその正確な ID の下に登録された鍵を検索する。
3. `acme-config`の鍵による署名は`acme-firmware`の鍵で検証できない。

これは各パブリッシャーが一意の鍵を持つ場合にのみ成立します。§6.2 で説明したように鍵がパブリッシャー間で共有される場合、この分離は破れます。

### 6.4 パブリッシャー鍵のローテーション

秘密鍵が漏洩した可能性がある場合、またはセキュリティポリシーで定期的なローテーションが求められる場合にパブリッシャー鍵をローテーションしてください。

1. オフラインで新しいキーペアを生成する。
2. 新しい秘密鍵で次のファームウェアリリースに署名する。
3. 新しい公開鍵を埋め込んで新しい鍵で`register_publisher`を呼び出すファームウェアアップデートを配布する。移行ウィンドウ中は古い鍵と新しい鍵の両方を含め、どちらのファームウェアバージョンのデバイスもアップデートを検証できるようにする。
4. すべてのデバイスが新しいファームウェアに移行したら、古い鍵の登録を削除する。
5. 古い秘密鍵を安全に破棄する。

### 6.5 FFI （ C/C++デバイス）

C/C++ FFI ブリッジ経由で統合するデバイスでは、パブリッシャー鍵の検証が`eds_verify_update`として公開される予定です（[#80](https://github.com/edgesentry/edgesentry-rs/issues/80)で追跡中）。この関数が利用可能になるまで、 C/C++デバイスはシンラッパー経由で Rust を呼び出すか、アプリケーション層でパブリッシャー検証を処理しなければなりません。

`eds_verify_update`に渡す公開鍵バイト列は、上記と同じ 32 バイトの Ed25519 公開鍵です。製造時にデバイスへプロビジョニングし、読み取り専用フラッシュ領域またはセキュアエレメントに保存してください。

---

## 7. HSM パス（ CLS レベル 4 ）

CLS レベル 4 および高保証デプロイメントでは、秘密鍵は抽出可能なバイト配列として存在すべきではありません。代わりに、署名操作は HSM または TEE 内部で実行し、秘密鍵材料がセキュア境界から外に出ないようにする必要があります。

計画中の`edgesentry-bridge` C/C++ FFI レイヤー（#53 ）と HSM 統合（#54 ）では、生の鍵バイトをアプリケーションコードに公開することなく、 Ed25519 の`sign`操作を HSM バックのプロバイダーに委譲する署名インターフェースを提供します。
