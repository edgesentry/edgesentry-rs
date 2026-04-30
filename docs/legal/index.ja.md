# 監査記録の法的証拠能力 — edgesentry-rs

- **更新日:** 2026-04-30  
- **スコープ:** 海事コンプライアンス、労働安全衛生、保険クレーム — シンガポール優先、国際的に適用

法廷で採用されない監査記録に商業的価値はありません。このドキュメントは法的証拠能力に必要な7つの要件を定義し、それぞれをedgesentry-rsの現在の実装にマッピングし、ギャップと対策を明示します。

---

## 適用される法的フレームワーク

| フレームワーク | 関連性 |
|---|---|
| シンガポール [**証拠法** (Cap. 97), s.116A](https://sso.agc.gov.sg/Act/97) | システムが「正常稼働」していた場合、コンピュータ生成記録は証拠採用可能 |
| シンガポール [**電子取引法** (Cap. 88), s.8](https://sso.agc.gov.sg/act/eta2010) | 完全性を証明できる場合、電子記録は信頼できるものとして認められる |
| シンガポール [**労働安全衛生法**](https://sso.agc.gov.sg/Act/WSHA2006) | インシデント記録；MOM（人材省）執行 |
| [**IMO FAL条約**](https://www.imo.org/en/about/conventions/pages/convention-on-facilitation-of-international-maritime-traffic-(fal).aspx) 附属書 | 完全性が証明できる場合、電子文書は紙と同等の効力 |
| [**BWM条約**](https://www.imo.org/en/about/conventions/pages/international-convention-for-the-control-and-management-of-ships'-ballast-water-and-sediments-(bwm).aspx) 規則B-2 | バラスト水記録簿 — 最低5年保存 |
| [**MLC 2006**](https://www.ilo.org/international-labour-standards/maritime-labour-convention-2006) 基準A5.2.1 | 寄港国管制（PSC）検査記録 |
| [**海上保険法 1906**](https://www.legislation.gov.uk/ukpga/Edw7/6/41/contents)（英国、シンガポールで適用） | 最大善意原則 — 事前損害記録の重要性 |
| [**UNCITRAL電子商取引モデル法**](https://uncitral.un.org/en/texts/ecommerce/modellaw/electronic_commerce) | 国際フレームワーク；機能的同等性原則 |

---

## 要件1 — 完全性：作成後に記録が改竄されていないこと

**証明すべきこと：** 各記録の内容が作成時から変更されていないことを証明できる。

**edgesentry-rs実装：**
- `payload_hash` = ペイロードバイト列のBLAKE3ハッシュ
- `payload_hash` に対するEd25519署名
- `prev_record_hash` が各記録を前の記録に連結
- `eds audit verify-chain` が全ハッシュを再計算し全署名を検証

**状態：✓ 充足**

---

## 要件2 — 帰属：誰が記録を作成したか

**証明すべきこと：** 記録を作成したデバイスとオペレーターを特定できる。

**edgesentry-rs実装：**
- 全 `AuditRecord` に `device_id` フィールド
- Ed25519署名鍵はデバイスごと；秘密鍵の保有者のみが有効な署名を生成可能
- デバイスオンボーディング時に公開鍵を `IntegrityPolicyGate` に登録

**状態：✓ 充足（Phase 1）**

**ギャップ：** 秘密鍵の管理。オペレーターが鍵を管理する場合、虚偽の記録に署名可能。

**対策 — Phase 1：** オンボーディング時の鍵登録 — 顧客の公開鍵とタイムスタンプをedgesentry側で記録。
鍵Kで日付D以降に署名された記録はサイトSに帰属。

**対策 — Phase 2：** エッジデバイス上のHSMまたはTPM — 秘密鍵は物理的に抽出不可能。
[#54](https://github.com/edgesentry/edgesentry-rs/issues/54) で追跡中。

---

## 要件3 — 信頼できるタイムスタンプ：いつ記録が作成されたか

**証明すべきこと：** オペレーターから独立したソースからの作成時刻。
デバイスのローカル時計はオペレーターが操作可能なため、法廷では信頼されない。

**edgesentry-rs実装：**
- `timestamp_ms` = 署名時の `SystemTime::now()`
- これはデバイスのローカル時計 — オペレーター管理

**状態：✗ ギャップ — 最大の弱点**

**対策A — Phase 1（即座）：**
作成時にCloudflare R2へアップロード。R2はデバイスではなくCloudflareのサーバーが設定する
不変の `x-amz-date` ヘッダを保持。Cloudflareは中立な第三者。
論拠：「デバイスが記録に署名し、Cloudflareが独立してアップロード時刻をタイムスタンプした；両者の時刻はN秒以内に一致する」

**対策B — Phase 2：**
[RFC 3161](https://www.rfc-editor.org/rfc/rfc3161.html) タイムスタンプ局（TSA）。
ローカル署名後、記録ハッシュをTSA（DigiCert、GlobalSign等）に送信。
TSAが署名付きタイムスタンプトークンを返し、記録と一緒に保存。
TSAタイムスタンプはほとんどの法域で法的に認められており、
[eIDAS](https://eur-lex.europa.eu/eli/reg/2014/910/oj/eng) PAdES-LTの長期検証標準。

---

## 要件4 — 完全性：記録が削除または欠落していないこと

**証明すべきこと：** 記録に欠落がない；オペレーターが不都合な記録を削除できない。

**edgesentry-rs実装：**
- `sequence` フィールドが単調増加（1, 2, 3 …）
- 記録Nの `prev_record_hash` は記録N-1のハッシュと等しい
- 記録Nを削除すると記録N+1でチェーンが壊れる

**状態：✓ 充足**

**注意：** 完全性は検証者がシーケンス1からの完全なチェーンを持っている場合のみ成立。
部分エクスポートには、既知のチェーン末尾との接続を確認できるよう
`prev_record_hash` を含むアンカーレコードが必要。

---

## 要件5 — 否認不可：署名者が署名を否定できないこと

**証明すべきこと：** 記録を作成した当事者がそれを否定できない。

**edgesentry-rs実装：**
- Ed25519は非対称 — 特定の公開鍵に対して有効な署名を生成できるのは秘密鍵保有者のみ
- オンボーディング時に公開鍵をedgesentry側に登録（要件2参照）

**状態：✓ 充足（条件付き）**

**ギャップ：** 鍵の共有。複数のデバイスが1つの秘密鍵を共有すると、
否認不可は「デバイスD」ではなく「サイトSの誰か」まで弱まる。

**対策：** オンボーディング時にデバイスごとに1鍵ペアを強制。
`device_id → public_key_hex` として鍵レジストリに保存。

---

## 要件6 — システム完全性：ソフトウェアが改変されていないこと

**証明すべきこと（証拠法s.116A）：** システムが「正常稼働」していた。
各記録を生成したソフトウェアバージョンが特定可能でなければならない。

**edgesentry-rs実装：**
- `MeasurementRecord` の `profile_version` — アクティブなルールセットを識別
- `device_id` がデバイスを識別

**不足：** `AuditRecord` に `software_version` フィールドがない — 現在ビルドハッシュなし。

**状態：⚠ 部分的**

**対策：** `AuditRecord` に `software_version: String` を追加 —
コンパイル時に `env!("CARGO_PKG_VERSION")` + ビルドメタデータ経由でGit SHAまたはリリースタグを埋め込む。
[ロードマップ](../roadmap/index.md) で追跡中。

---

## 要件7 — 保存と検索可能性

**証明すべきこと：** 必要なときに記録にアクセスできる。
BWM条約は5年保存を要求；MOM/WSH検査はインシデントから数年後に行われる場合がある。

**edgesentry-rs実装：**
- R2 Object Lock（コンプライアンスモード） — 保存期間中は誰も削除・上書き不可
- JSON形式とオープンなアルゴリズム（BLAKE3、Ed25519） — 独自ソフトウェア不要

**状態：✓ 充足（R2 Object Lock設定後）**

**ギャップ：** フォーマットの長期性。アルゴリズムIDが記録スキーマに埋め込まれていない。

**対策：** `AuditRecord` に `"hash_alg": "blake3"` および `"sig_alg": "ed25519"` を追加。
独立した検証者が再実装できるよう `eds audit verify-chain` をオープンソース（Apache 2.0）で公開。

---

## サマリーマトリクス

| 要件 | 状態 | ギャップ | Phase 1 対策 | Phase 2 対策 |
|---|---|---|---|---|
| 1. 完全性 | ✓ 充足 | — | — | — |
| 2. 帰属 | ✓ Phase 1 | 鍵管理 | オンボーディング時の鍵登録 | HSM / TPM ([#54](https://github.com/edgesentry/edgesentry-rs/issues/54)) |
| 3. 信頼できるタイムスタンプ | ✗ ギャップ | ローカル時計 | R2アップロード時刻（Cloudflareアンカー） | RFC 3161 TSAトークン |
| 4. 完全性（欠落なし） | ✓ 充足 | 部分エクスポート | エクスポートにアンカーレコード | — |
| 5. 否認不可 | ✓ 条件付き | 鍵の共有 | オンボーディング時にデバイスごと1鍵 | — |
| 6. システム完全性 | ⚠ 部分的 | `software_version` なし | `AuditRecord` にGit SHA追加 | — |
| 7. 保存・検索 | ✓ 充足（R2） | アルゴリズムID | `hash_alg`/`sig_alg` フィールド追加 | — |

---

## 紙ベースの海事記録との比較

| 特性 | 紙の航海日誌 | edgesentry-rs Phase 1 |
|---|---|---|
| 改竄検知 | なし（インクは変更可能） | BLAKE3ハッシュチェーン |
| 帰属 | 手書き署名 | Ed25519 + `device_id` |
| タイムスタンプ | 担当者の手書き | ローカル時計（紙と同等） |
| 完全性 | ページ除去可能 | チェーン破損で検出可能 |
| 保存 | 物理保管（火災・浸水リスク） | R2 Object Lock（地理冗長） |
| 検索可能性 | 手作業検索 | `device_id`、タイムスタンプ範囲でクエリ可能 |

現在の実装は、完全性・完全性（欠落なし）・保存においてすでに紙を上回っています。
タイムスタンプは Phase 1 では紙と同等（オペレーター管理の時計）ですが、
Phase 2 では RFC 3161 TSA により紙を上回ります。

---

## IoTセキュリティ規格との関係

法的証拠能力とIoTセキュリティ認証は重複しますが別物です：

- **IoTセキュリティ規格**（CLS、ETSI EN 303 645、JC-STAR）はシステムの*構築方法*を定義。See [`docs/security/`](../security/index.ja.md)
- **法的証拠能力**は*裁判所や規制当局が証拠として受け入れるもの*を定義。このドキュメントがカバーする領域。

CLS Level 3認証を受けたシステムは法的に認められる監査記録の強固な基盤になりますが、
認証だけでは証拠能力を保証しません。
