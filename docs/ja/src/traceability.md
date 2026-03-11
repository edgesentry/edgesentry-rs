# コンプライアンス・トレーサビリティマトリックス

このページは、シンガポール CLS / iM8 の各条項および対応する ETSI EN 303 645 の規定を、それを満たすソースコードにマッピングします。各行には Japan JC-STAR の相互参照と SS 711:2025 設計原則のアライメントが含まれています。

凡例：

- ✅ 実装済み
- ⚠️ 部分的
- 🔲 計画中
- ➖ スコープ外

## SS 711:2025 設計原則カバレッジ

シンガポールの国内 IoT 標準 SS 711:2025 は 4 つの原則を定義しています。完全なモジュールマッピングについては[ロードマップ](roadmap.md)を参照してください。

| 原則 | SS 711:2025 要件 | ステータス |
|-----------|------------------------|--------|
| セキュア・バイ・デフォルト | 一意のデバイス同一性、署名付き OTA アップデート | ✅ `identity.rs`、`update.rs` |
| 防御の厳格性 | STRIDE の脅威モデル、改ざん検知 | ⚠️ ハッシュチェーン ✅ — STRIDE アーティファクト 🔲 [#93](https://github.com/edgesentry/edgesentry-rs/issues/93) |
| アカウンタビリティ | 監査証跡、操作ログ、 RBAC 設計 | ✅ `ingest/`（ AuditLedger 、 OperationLog ） |
| 回復力 | デフォルト拒否のネットワーキング、 DoS 対策 | ✅ `ingest/network_policy.rs` |

---

---

## CLS レベル 3 / ETSI EN 303 645 — コア要件

### CLS-01 / §5.1 — 汎用デフォルトパスワードの禁止

| 項目 | 詳細 |
|------|--------|
| JC-STAR | STAR-1 R3.1 |
| 要件 | デバイスは汎用デフォルト認証情報を使用してはならない |
| ステータス | ➖ スコープ外 — このプロジェクトはソフトウェア監査レコードを実装するものであり、デバイスの認証情報管理ではない |

---

### CLS-02 / §5.2 — 脆弱性報告の管理手段の実装

| 項目 | 詳細 |
|------|--------|
| JC-STAR | STAR-1 R4.1 |
| 要件 | SLA が定められた、公開・実行可能な脆弱性報告チャネル |
| ステータス | ⚠️ 部分的 |
| ギャップ | 正式な開示プロセスが未定義。[#58](https://github.com/edgesentry/edgesentry-rs/issues/58)を参照 |

---

### CLS-03 / §5.3 — ソフトウェアの最新状態維持

| 項目 | 詳細 |
|------|--------|
| JC-STAR | STAR-2 R2.2 |
| 要件 | ソフトウェアアップデートパッケージはインストール前に署名・検証されなければならない |
| ステータス | ✅ 実装済み |
| 実装 | `UpdateVerifier::verify`が BLAKE3 ペイロードハッシュと Ed25519 パブリッシャー署名をチェックしてからインストールを許可。失敗したチェックは`UpdateVerificationLog`に`UpdateVerifyDecision::Rejected`として記録される（[`src/update.rs`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/update.rs)） |
| テスト | `tests/unit/update_tests.rs` — 承認パス・改ざんペイロード・無効な署名・不明なパブリッシャー・マルチパブリッシャー分離をカバー |

---

### CLS-04 / §5.4 — 機密セキュリティパラメータの安全な保存

| 項目 | 詳細 |
|------|--------|
| JC-STAR | STAR-1 R1.2 |
| 要件 | 秘密鍵は安全に保存されなければならない。鍵登録プロセスが存在しなければならない |
| ステータス | ✅ 実装済み |
| 実装 | 公開鍵レジストリ：`IntegrityPolicyGate::register_device`（[`src/ingest/policy.rs:20`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/ingest/policy.rs#L20)） |
| 実装 | 鍵生成 CLI ：`eds keygen`（[`src/lib.rs — generate_keypair`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/lib.rs)） |
| 実装 | 鍵検査 CLI ：`eds inspect-key`（[`src/lib.rs — inspect_key`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/lib.rs)） |
| 実装 | プロビジョニングとローテーションのガイダンス：[鍵管理](key_management.md) |
| 注意 | HSM バックの鍵保存（ CLS レベル 4 ）は[#54](https://github.com/edgesentry/edgesentry-rs/issues/54)で計画中 |

---

### CLS-05 / §5.5 — 安全な通信

| 項目 | 詳細 |
|------|--------|
| JC-STAR | STAR-1 R1.1 |
| 要件 | データは真正性の保証を持って送信されなければならない |
| ステータス | ⚠️ 部分的 |
| 実装 | すべての`AuditRecord`は BLAKE3 ペイロードハッシュに対する Ed25519 署名を持つ — `build_signed_record`（[`src/agent.rs`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/agent.rs)）、`sign_payload_hash`（[`src/identity.rs:12`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/identity.rs#L12)） |
| ギャップ | トランスポート層の暗号化（ TLS ）はスコープ外 — レコードレベルの署名は真正性を提供するがチャネルの機密性は提供しない。[#73](https://github.com/edgesentry/edgesentry-rs/issues/73)で追跡中 |

---

### CLS-06 / §5.6 — 露出する攻撃面の最小化

| 項目 | 詳細 |
|------|--------|
| JC-STAR | STAR-1 R3.2 |
| 要件 | 必要なインターフェースとサービスのみを公開すべき |
| ステータス | ⚠️ 部分的 |
| 実装 | `NetworkPolicy`がデフォルト拒否の IP/CIDR アローリスト強制を提供 — 呼び出し元は`IngestService`を呼び出す前に`NetworkPolicy::check(source_ip)`を通じて各インジェストリクエストをゲートする（[`src/ingest/network_policy.rs`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/ingest/network_policy.rs)） |
| ギャップ | ライブラリはネットワークサービスを直接公開しない。トランスポート層のコントロール（ VPN ・ファイアウォールルール）はデプロイ担当者の責任 |

---

### CLS-07 / §5.7 — ソフトウェア完全性の保証

| 項目 | 詳細 |
|------|--------|
| JC-STAR | STAR-1 R1.3 |
| 要件 | デバイスはソフトウェアとデータの完全性を検証しなければならない |
| ステータス | ✅ 実装済み |
| 実装 — ペイロードハッシュ | 生ペイロードに対する BLAKE3 ハッシュ：`compute_payload_hash`（[`src/integrity.rs:12`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/integrity.rs#L12)） |
| 実装 — ハッシュチェーン | `prev_record_hash`が各レコードを前のレコードにリンク。挿入/削除は`verify_chain`で検知（[`src/integrity.rs:35`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/integrity.rs#L35)） |
| テスト | `tampered_lift_demo_chain_is_detected`（[`src/lib.rs:338`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/lib.rs#L338)） |

---

### CLS-08 / §5.8 — 個人データの安全性の確保

| 項目 | 詳細 |
|------|--------|
| JC-STAR | STAR-2 R4.1 |
| 要件 | 送信または保存される個人データは保護されなければならない |
| ステータス | ➖ スコープ外 — 現在の実装では監査レコードに個人データを含まない |

---

### CLS-09 / §5.9 — 障害に対するシステムの回復力

| 項目 | 詳細 |
|------|--------|
| JC-STAR | STAR-2 R3.2 |
| 要件 | デバイスは運用状態を維持し、優雅に回復すべき |
| ステータス | ➖ スコープ外（部分的なパスは計画中） |
| 注意 | 完全な HA はデプロイ担当者の責任だが、ライブラリは接続断失中に署名済みレコードを蓄積し、リンク回復時にチェーン順序で再送するオフラインバッファ/ストア&フォワードモジュールを提供できる。[#74](https://github.com/edgesentry/edgesentry-rs/issues/74)で追跡中 |

---

### CLS-10 / §5.10 — システムテレメトリデータの検査

| 項目 | 詳細 |
|------|--------|
| JC-STAR | STAR-2 R3.1 |
| 要件 | セキュリティ関連イベントはログに記録され、リプレイ/並べ替え攻撃が検知されなければならない |
| ステータス | ✅ 実装済み |
| 実装 — シーケンス | デバイスごとの厳密な単調増加`sequence`。重複・順序違いのレコードは`IngestState::verify_and_accept`で拒否（[`src/ingest/verify.rs:45`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/ingest/verify.rs#L45)） |
| 実装 — 監査証跡 | 承認/拒否の決定は`IngestService`と`AuditLedger`経由で永続化（[`src/ingest/storage.rs`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/ingest/storage.rs)） |

---

### CLS-11 / §5.11 — ユーザーデータの削除を容易にする

| 項目 | 詳細 |
|------|--------|
| JC-STAR | — |
| 要件 | ユーザーは個人データを削除できるべき |
| ステータス | ➖ スコープ外 |

---

## CLS レベル 4 — 追加要件

### CLS レベル 4 — ハードウェアセキュリティモジュール（ HSM ）

| 項目 | 詳細 |
|------|--------|
| JC-STAR | STAR-2 R1.4 |
| 要件 | 秘密鍵は HSM 内に保存・使用されなければならない |
| ステータス | 🔲 計画中 |
| ギャップ | HSM バックの鍵保存はフェーズ 3 （ IEC 62443-4-2 / CII/OT ）で計画中。[#54](https://github.com/edgesentry/edgesentry-rs/issues/54)および[#98](https://github.com/edgesentry/edgesentry-rs/issues/98)を参照 |

---

## JC-STAR 追加要件

### STAR-1 R2.1 — リプレイ・並べ替え防止

| 項目 | 詳細 |
|------|--------|
| CLS | CLS-10 |
| 要件 | リプレイ攻撃は検知・拒否されなければならない |
| ステータス | ✅ 実装済み |
| 実装 | `IngestState`の`seen` HashSet が重複した`(device_id, sequence)`ペアを拒否（[`src/ingest/verify.rs:56`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/ingest/verify.rs#L56)） |

---

## カバレッジサマリー

| レベル | 総条項数 | ✅ 実装済み | ⚠️ 部分的 | 🔲 計画中 | ➖ スコープ外 |
|-------|-------------|--------------|-----------|-----------|----------------|
| CLS レベル 3 | 11 | 3 | 4 | 0 | 4 |
| CLS レベル 4 | 1 | 0 | 0 | 1 | 0 |
| JC-STAR 追加 | 1 | 1 | 0 | 0 | 0 |

>**注意：** 「スコープ外」の条項は、デバイスレベルの懸念事項（パスワード・ネットワークインターフェース・個人データ）をカバーしており、監査レコードライブラリではなくデプロイ担当者の責任となるものです。
