# ステップ 7 - 封印

BLAKE3 + Ed25519 を使用して各レコードに署名し、前のレコードにチェーン化します。改ざんされたレコードはチェーンを壊し、`eds audit verify-chain` が即座に検出します。

`edgesentry-audit` クレートが実装です。鍵管理、デプロイメント、脅威モデルの詳細については専用の本を参照してください。このページでは、フェーズ 1-3 パイプラインデモで使用される CLI コマンドを説明します。

## デモ監査チェーン

デモ目的の事前構築されたリフト点検チェーンを生成します：

```
eds audit demo-lift-inspection
  --device-id <ID>
  --private-key-hex <HEX>
  --out-file <FILE>
  [--start-timestamp-ms <MS>]
  [--object-prefix <PREFIX>]
  [--payloads-file <FILE>]
```

| フラグ | デフォルト | 説明 |
|------|---------|-------------|
| `--device-id` | `lift-01` | 各レコードに埋め込まれるデバイス識別子 |
| `--private-key-hex` | `0101...01` | 64 文字の16進数で表した Ed25519 秘密鍵 |
| `--out-file` | `lift_inspection_records.json` | AuditRecord の JSON 配列の出力ファイル |
| `--start-timestamp-ms` | `1700000000000` | 最初のレコードのタイムスタンプ |
| `--payloads-file` | | オプション：生のペイロードを16進文字列として書き込む（demo-ingest 用） |

デモ鍵（`0101...01` 32バイト繰り返し）はローカルテスト専用です。本番デプロイメントの前に `eds audit keygen` で実際の鍵ペアを生成してください。

## チェーンの検証

```
eds audit verify-chain --records-file <FILE>
```

AuditRecord の JSON 配列を読み込み、各 BLAKE3 ハッシュを再計算し、各 Ed25519 署名を検証し、レコード N の `prev_hash` がレコード N-1 のハッシュと一致することを確認します。成功時は 0 で終了し、失敗時は特定のエラーメッセージとともに 1 で終了します。

## 鍵管理

```bash
# 新しい Ed25519 鍵ペアを生成
eds audit keygen [--out <FILE>]

# 既存の秘密鍵から公開鍵を導出
eds audit inspect-key --private-key-hex <HEX> [--out <FILE>]
```

本番環境では秘密鍵をシークレットマネージャーまたはハードウェアセキュリティモジュールに保存してください。公開鍵は自由に配布できます ── 検証にのみ使用されます。

## AuditRecord の構造

各レコードには以下が含まれます：

```json
{
  "sequence": 1,
  "device_id": "demo-edge-01",
  "timestamp_ms": 1700000001000,
  "payload_hash": "<BLAKE3 hex of the payload bytes>",
  "prev_hash": "<BLAKE3 hex of the previous record>",
  "signature": "<Ed25519 signature hex over hash of all above fields>"
}
```

チェーンは最初のレコードの `prev_hash` がすべてゼロから始まります。任意のフィールドへの挿入、削除、または変更は、そのレコード以降のすべての署名を無効にします。
