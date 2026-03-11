# C/C++ FFI ブリッジ

`edgesentry-bridge`は、 Ed25519 署名と BLAKE3 ハッシュチェーン検証を安定した C ABI として公開する独立した Rust クレートです。 C および C++のファームウェアやゲートウェイは、全面的な書き直しなしに Rust ライブラリと同じセキュリティロジックを呼び出せます。

---

## ライブラリのビルド

```bash
cargo build -p edgesentry-bridge --release
```

生成されるファイル：

| プラットフォーム | ファイル |
|----------|------|
| macOS | `target/release/libedgesentry_bridge.dylib`および`.a` |
| Linux | `target/release/libedgesentry_bridge.so`および`.a` |

ヘッダー`crates/edgesentry-bridge/include/edgesentry_bridge.h`は`build.rs`が`cbindgen`を使って自動的に再生成します。

---

## C/C++からのリンク

**macOS:**

```bash
cc -o my_app main.c \
   -I path/to/edgesentry-bridge/include \
   -L path/to/target/release \
   -ledgesentry_bridge \
   -framework Security -framework CoreFoundation
```

**Linux:**

```bash
cc -o my_app main.c \
   -I path/to/edgesentry-bridge/include \
   -L path/to/target/release \
   -ledgesentry_bridge \
   -lpthread -ldl
```

既製の`Makefile`が`crates/edgesentry-bridge/examples/c_integration/`に用意されています。

---

## API リファレンス

### エラーコード

| 定数 | 値 | 意味 |
|----------|-------|---------|
| `EDS_OK` | `0` | 成功 |
| `EDS_ERR_NULL_PTR` | `-1` | 必須ポインタが NULL だった |
| `EDS_ERR_INVALID_UTF8` | `-2` | 文字列引数が有効な UTF-8 でない |
| `EDS_ERR_INVALID_KEY` | `-3` | 鍵またはハッシュバッファが無効 |
| `EDS_ERR_STRING_TOO_LONG` | `-4` | 文字列が固定バッファサイズを超えている |
| `EDS_ERR_CHAIN_INVALID` | `-5` | ハッシュチェーン検証に失敗 |
| `EDS_ERR_PANIC` | `-6` | 予期しない内部エラー |

### レコード構造体

```c
typedef struct {
    uint64_t sequence;           /* monotonic record index (starts at 1) */
    uint64_t timestamp_ms;       /* Unix epoch in milliseconds           */
    uint8_t  payload_hash[32];   /* BLAKE3 hash of the raw payload        */
    uint8_t  signature[64];      /* Ed25519 signature over payload_hash   */
    uint8_t  prev_record_hash[32]; /* hash of preceding record (zero for first) */
    uint8_t  device_id[256];     /* null-terminated device identifier     */
    uint8_t  object_ref[512];    /* null-terminated storage reference     */
} EdsAuditRecord;
```

`EdsAuditRecord`は**呼び出し元が確保**します。 Rust は`malloc`を呼び出したりヒープポインタを返したりしません。そのため`_free`関数は不要です。

### 関数

```c
/* Generate an Ed25519 keypair via OS CSPRNG.
   private_key_out and public_key_out must each point to 32 bytes. */
int32_t eds_keygen(uint8_t *private_key_out, uint8_t *public_key_out);

/* Hash payload with BLAKE3, sign with Ed25519, fill *out.
   Pass NULL for prev_record_hash to use the zero hash (first record). */
int32_t eds_sign_record(const char    *device_id,
                        uint64_t       sequence,
                        uint64_t       timestamp_ms,
                        const uint8_t *payload,
                        size_t         payload_len,
                        const uint8_t *prev_record_hash,
                        const char    *object_ref,
                        const uint8_t *private_key,
                        EdsAuditRecord *out);

/* Compute the per-record hash (used as prev_record_hash for the next record).
   hash_out must point to 32 bytes. */
int32_t eds_record_hash(const EdsAuditRecord *record, uint8_t *hash_out);

/* Verify Ed25519 signature. Returns 1 valid, 0 invalid, negative on error. */
int32_t eds_verify_record(const EdsAuditRecord *record,
                          const uint8_t *public_key);

/* Verify the entire hash chain. Returns EDS_OK or EDS_ERR_CHAIN_INVALID. */
int32_t eds_verify_chain(const EdsAuditRecord *records, size_t count);
```

---

## 最小限の C サンプル

```c
#include "edgesentry_bridge.h"
#include <string.h>
#include <assert.h>

int main(void) {
    uint8_t priv_key[32], pub_key[32];
    assert(eds_keygen(priv_key, pub_key) == EDS_OK);

    const char *payload = "check=door,status=ok";
    EdsAuditRecord rec;
    memset(&rec, 0, sizeof(rec));

    assert(eds_sign_record("lift-01", 1, 1700000000000ULL,
                           (const uint8_t *)payload, strlen(payload),
                           NULL,              /* zero hash — first record */
                           "lift-01/1.bin",
                           priv_key, &rec) == EDS_OK);

    assert(eds_verify_record(&rec, pub_key) == 1);
    return 0;
}
```

完全なサンプルは`crates/edgesentry-bridge/examples/c_integration/main.c`を参照してください。

---

## メモリ安全規約

| 規約 | 詳細 |
|------|--------|
| ヒープ確保なし | `EdsAuditRecord`は呼び出し元が確保。 Rust は`malloc`を呼び出さない |
| NULL チェック | すべてのポインタ引数はチェック済み。失敗時は`EDS_ERR_NULL_PTR`を返す |
| 固定長文字列 | `device_id`は最大 255 文字、`object_ref`は最大 511 文字。超過入力は`EDS_ERR_STRING_TOO_LONG`を返す |
| パニック安全性 | すべての FFI 関数を`std::panic::catch_unwind`でラップ。 Rust パニックは C 境界を越えてアンワインドする代わりに`EDS_ERR_PANIC`を返す |
| 鍵サイズ | `private_key`と`public_key`はちょうど 32 バイトを指していなければならない。ハッシュバッファは 32 バイト、署名バッファは 64 バイト |

---

## HSM パス

CLS レベル 4 では、秘密鍵は抽出可能なバイト配列として存在すべきではありません。計画中の HSM 統合（[#54](https://github.com/edgesentry/edgesentry-rs/issues/54)）では、鍵バイトを呼び出し元に公開することなく`eds_sign_record`操作を HSM バックのプロバイダーに委譲します。
