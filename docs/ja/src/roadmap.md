# ロードマップ

EdgeSentry-RS は段階的なアプローチを採用しています。まずシンガポールのコンプライアンスベースライン（ CLS レベル 2 → レベル 3 、 SS 711:2025 ）を確立し、次に GCLI 相互承認を通じて日本（ JC-STAR ・サイバートラストマーク）へ展開し、最終的に EU ・英国・重要インフラ市場へのグローバルコンバージェンスを目指します。これは DuckDB モデルを踏襲したアプローチです。すなわち、ロックインではなくエコシステムへの採用を通じてデファクトスタンダードとなる、埋め込み可能な OSS コアを構築します。

## なぜシンガポールを最初に選ぶのか

シンガポールの CLS は欧州の **ETSI EN 303 645** 標準を直接的な基礎としています。日本の JC-STAR も同様に ETSI EN 303 645 を技術的な根拠として参照しています。つまり、 3 つの規制体制は共通の基盤を持っています。

| 標準 | 地域 | 根拠 |
|----------|--------|----------|
| ETSI EN 303 645 | 欧州 (CRA) | オリジナル |
| CLS レベル 2/3/4 | シンガポール | ETSI EN 303 645 |
| JC-STAR | 日本 | ETSI EN 303 645 |

シンガポール CLS への対応を最初に実装することで、技術的な作業の大部分が日本の JC-STAR および欧州の CRA 要件を直接満たすことになります。シンガポールは単なる地域ターゲットではなく、グローバルなコンプライアンス対応への最短経路です。

## GCLI と日本シンガポール間の直接 MoC

日本は 2025 年に **Global Cyber Labelling Initiative (GCLI)** に署名し、シンガポール・英国・フィンランド・ドイツ・韓国など 11 カ国に加わりました。 GCLI は各国の IoT セキュリティラベル間で相互承認を確立する多国間の枠組みであり、シンガポール CLS 認証取得製品は再認証なしで日本の JC-STAR 準拠として認められます。これが「シンガポール優先」戦略を日本市場参入パスとして機能させる構造的な仕組みです。

2026 年 3 月、日本とシンガポールはこれをさらに強化する形で、 METI/IPA （日本）と CSA （シンガポール）の間で **JC-STAR と CLS の直接相互承認に関する協力覚書（ MoC ）** に署名しました。この MoC は **2026 年 6 月 1 日に発効** します。この枠組みのもとでは、有効な JC-STAR ラベルはそのまま CLS として認められます。 JC-STAR の認証データを CLS 要件に再マッピングする必要はありません。日本はシンガポール CLS との二国間相互承認を達成した 5 番目の国となりました（フィンランド・ドイツ・韓国・英国に次ぐ）。

> **未解決の問題：** JC-STAR の各レベル（ STAR-1 〜 STAR-4 ）と CLS スターレベル（ 1 〜 4 ）の公式な等価対応表は、 CSA/METI によってまだ公表されていません。どの JC-STAR レベルが特定の CLS 目標レベルを満たすかを判断するために、 CSA の CLS ページおよび METI/IPA の JC-STAR ページを継続的に確認してください。

シンガポール CLS とフィンランド・ドイツ・韓国の間には、さらに二国間 MRA （相互承認協定）も存在します。ドイツや韓国の IoT 認証を取得済みの日本のお客様にとっては、これらの MRA が CLS 取得の近道となります。

## SS 711:2025 設計原則

シンガポールの国内 IoT 標準 SS 711:2025 （ TR 64:2018 に替わるもので、 CLS レベル 3 評価の基盤となる）は、 4 つのセキュリティ設計原則を定義しています。 EdgeSentry-RS はこれらを中心に設計されています。

| 原則 | 要件 | 実装 |
|-----------|-------------|----------------|
| セキュア・バイ・デフォルト | 一意のデバイス同一性、署名付き OTA | `identity.rs`（ Ed25519 ）、`update.rs`（署名付きアップデート検証） |
| 防御の厳格性 | STRIDE の脅威モデリング、改ざん検知 | `integrity.rs`（ BLAKE3 ハッシュチェーン）、 STRIDE 脅威モデルアーティファクト |
| アカウンタビリティ | 監査証跡、操作ログ | `ingest/`（ AuditLedger 、 OperationLog 、 IntegrityPolicyGate ） |
| 回復力 | デフォルト拒否のネットワーキング、レート制限 | `ingest/network_policy.rs`（ IP/CIDR アローリスト） |

## 実装マッピング

CLS / ETSI EN 303 645 / JC-STAR 要件とソースコードの詳細な条項別マッピングは、[コンプライアンス・トレーサビリティマトリックス](traceability.md)を参照してください。

---

## フェーズ 1 ：シンガポールゲートウェイ（現在〜 6 ヶ月）

**目標：** CLS レベル 2 → レベル 3 、 SS 711:2025 、 iM8

IMDA の評価担当者が求める SDL エビデンスアーティファクト（脅威モデル・ SBOM ・バイナリ分析）を備え、シンガポール CLS レベル 2 のサイバーハイジーン要件を満たしつつレベル 3 へ進展するソフトウェアリファレンス実装を提供します。

### マイルストーン 1.1 ：同一性＆完全性コア ✅ 実装済み

- `edgesentry_rs::identity` — Ed25519 デバイス署名の実装
- `edgesentry_rs::integrity` — BLAKE3 ハッシュチェーンによる改ざん検知プロトコル
- `edgesentry_rs::ingest::NetworkPolicy` — デフォルト拒否の IP/CIDR アローリスト（ CLS-06 ）

### マイルストーン 1.2 ： C/C++ブリッジ ✅ 実装済み

- `edgesentry-bridge` — Ed25519 署名・署名検証・ハッシュチェーン検証を C/C++プロジェクトに公開する C 互換 FFI レイヤー
- **目的：** 最小限の変更で既存の日本製ハードウェア（ゲートウェイ・センサー）にシンガポール水準のセキュリティを組み込む
- 使い方・リンク方法・メモリ安全規約については[C/C++ FFI ブリッジ](ffi_bridge.md)を参照してください

### マイルストーン 1.3 ：コンプライアンスマッピング v1.0 ✅ 実装済み

- シンガポール CLS/iM8 条項とソースコードのトレーサビリティマトリックス：[コンプライアンス・トレーサビリティマトリックス](traceability.md)

### マイルストーン 1.4 ： SBOM ＋ベンダー開示チェックリスト ✅ 実装済み

IMDA の IoT サイバーセキュリティガイドは、 CLS レベル 3 評価エビデンスとしてベンダー開示チェックリストを必須としています。 5 つの必須カテゴリは、暗号化サポート・識別と認証・データ保護・ネットワーク保護・ライフサイクルサポート（ SBOM ）です。

- CycloneDX JSON SBOM をすべてのクレートに対して生成し、各 GitHub Release と共に公開
- 5 つの全カテゴリのベンダー開示チェックリスト回答を文書化
- チェックリスト回答を既存実装にトレーサビリティマトリックスでマッピング
- [SBOM とベンダー開示](sbom.md)および [#92](https://github.com/edgesentry/edgesentry-rs/issues/92)を参照してください

### マイルストーン 1.5 ：トランスポート層・非同期インジェスト・オフラインバッファ ✅ 実装済み

- `async-ingest` フィーチャー：`AsyncIngestService<R,L,O>`（`Arc`による安全なマルチタスク共有） — [#115](https://github.com/edgesentry/edgesentry-rs/issues/115) クローズ
- `transport-http` フィーチャー：axum ベースの`POST /api/v1/ingest`エンドポイント；暗号検証前に`NetworkPolicy`でソース IP をゲート；`eds serve` CLI — [#116](https://github.com/edgesentry/edgesentry-rs/issues/116) クローズ
- `transport-tls` フィーチャー：rustls TLS 1.2/1.3 による`serve_tls()`；`eds serve-tls --tls-cert / --tls-key` CLI；CLS-05 HTTP チャネル機密性を満足 — [#176](https://github.com/edgesentry/edgesentry-rs/issues/176) クローズ
- `transport-mqtt-tls` フィーチャー：CA 証明書パスを持つ `MqttTlsConfig`、`rumqttc::TlsConfiguration::Rustls` 経由の rustls `ClientConfig`；`eds serve-mqtt --tls-ca-cert` CLI；CLS-05 MQTT チャネル機密性を満足 — [#180](https://github.com/edgesentry/edgesentry-rs/issues/180) クローズ
- `transport-mqtt` フィーチャー：`serve_mqtt()`が設定可能なトピックをサブスクライブし、`AsyncIngestService`にルーティング；`<topic>/response`に承認/拒否を公開；`eds serve-mqtt` CLI — [#146](https://github.com/edgesentry/edgesentry-rs/issues/146) クローズ
- `buffer` モジュール：プラグ可能な`BufferStore`トレイトを持つ`OfflineBuffer<S>`；`InMemoryBufferStore`デフォルト；`buffer-sqlite`フィーチャー配下の`SqliteBufferStore`；CLS-09 回復力を満足 — [#74](https://github.com/edgesentry/edgesentry-rs/issues/74) クローズ

### マイルストーン 1.6 ： STRIDE 脅威モデル＋バイナリ分析エビデンス ✅ 実装済み

CLS レベル 3 の評価担当者はコードだけでなく、記録された設計アーティファクトを要求します。 SS 711:2025 はすべての攻撃面（ API ・通信・ストレージ）の STRIDE ベース脅威モデリングを必要とします。

- STRIDE の脅威モデル（対象：なりすまし（デバイス同一性）・改ざん（監査レコード）・否認（操作ログ）・情報漏洩（ペイロードストレージ）・サービス拒否（ネットワークポリシー）・特権昇格（インジェストゲート））— [`docs/ja/src/threat_model.md`](threat_model.md) を参照
- 出荷クレートに既知の CVE がないことを確認するバイナリ分析レポート（`cargo-audit`・`cargo-deny`）
- 脅威モデルの緩和策をトレーサビリティマトリックスエントリにリンク— [`docs/src/traceability.md`](../../../docs/src/traceability.md)（「防御の厳格性」を ✅ に更新済み）
- 日本語訳： `docs/ja/src/threat_model.md`
- クローズ済み： [#93](https://github.com/edgesentry/edgesentry-rs/issues/93) （PR [#143](https://github.com/edgesentry/edgesentry-rs/pull/143)）

---

## フェーズ 2 ： GCLI を通じた日本対応（ 6 〜 12 ヶ月）

**目標：** CLS レベル 4 、 JC-STAR STAR-1/2 、サイバートラストマーク / ISO 27001

### マイルストーン 2.0 ：相互承認フレームワーク（ GCLI ＋ 日本シンガポール MoC ） 🔲 計画中

重複認証なしに日本市場へ参入するための、補完的な 2 つのメカニズムがあります。

1. **GCLI** — 「シンガポール優先」戦略全体を支える多国間フレームワーク（ 10 カ国以上）。
2. **日本シンガポール直接 MoC** （ 2026 年 3 月署名、 **2026 年 6 月 1 日発効** ）— JC-STAR と CLS の二国間相互承認。有効な JC-STAR ラベルはそのまま CLS として認められるため、認証データの再マッピングは不要。

本マイルストーンの成果物：

- GCLI ルートと直接 MoC ルートの両方をカバーする日本のお客様向けコンプライアンス経路ガイド
- JC-STAR ラベル検証・証明モジュール（`edgesentry_rs::compliance::jcstar`） — [#121](https://github.com/edgesentry/edgesentry-rs/issues/121)を参照してください
- CLS ↔ JC-STAR レベル等価対応表（ CSA/METI による公表待ち； CSA および METI/IPA のページを継続的に確認してください）
- フィンランド・ドイツ・韓国の IoT 認証取得済みのお客様向け MRA ファストトラックガイダンス
- [#94](https://github.com/edgesentry/edgesentry-rs/issues/94)を参照してください

### マイルストーン 2.1 ： JC-STAR STAR-1/2 アライメント 🔲 計画中

- 日本の IoT 製品セキュリティ適合性評価基準に基づくセルフチェックリストと実装ガイダンス
- [#82](https://github.com/edgesentry/edgesentry-rs/issues/82)を参照してください

### マイルストーン 2.2 ：エッジインテリジェンス 🔲 計画中

- `edgesentry-summary` — 帯域幅が制約された回線上の高性能な日本製センサー向けのデータ要約ロジック。[#83](https://github.com/edgesentry/edgesentry-rs/issues/83)を参照してください
- `edgesentry-detector` — 結果に署名済み監査エビデンスを添付したローカル異常検知。[#84](https://github.com/edgesentry/edgesentry-rs/issues/84)を参照してください

### マイルストーン 2.3 ：クロスボーダー教育プログラム 🔲 計画中

- 日本企業がシンガポールの公共インフラプロジェクトに入札するための合同技術白書
- [#85](https://github.com/edgesentry/edgesentry-rs/issues/85)を参照してください

### マイルストーン 2.4 ：サイバートラストマーク / ISO 27001 組織トラック 🔲 計画中

シンガポールのサイバートラストマークは、 2026 〜 27 年から CII （重要情報インフラ）事業者に対して義務化されます。これは製品レベルの CLS に対する組織レベルの対応物です。シンガポールの B2B および Government のお客様は、ベンダーにこのトラックへの対応をますます求めるようになるでしょう。

- EdgeSentry-RS の実装エビデンスをサイバートラストマーク評価カテゴリにマッピング
- ISO 27001 コントロールアライメント文書
- [#95](https://github.com/edgesentry/edgesentry-rs/issues/95)を参照してください

### マイルストーン 2.5 ： CLS(MD) — 医療機器バリアント 🔲 計画中

シンガポールは 2024 年 10 月に医療機器向け CLS （ CLS(MD)）を開始しました。医療 IoT を目標市場とする場合、特定のバリアント要件が適用されます。

- 現在の実装に対する CLS(MD)ギャップ分析
- 医療機器固有要件の特定
- [#96](https://github.com/edgesentry/edgesentry-rs/issues/96)を参照してください

---

## フェーズ 3 ：グローバルコンバージェンス — 「ヨーロッパへの地平線」（ 12 〜 24 ヶ月）

**目標：** EU CRA 、英国 PSTI 法、 IEC 62443-4-2 （ CII/OT ）、 CCoP 2.0

### マイルストーン 3.1 ： EU CRA コンプライアンス調査 🔲 計画中

- 欧州市場へのパスポートとして **ETSI EN 303 645** へのフルマッピング
- シンガポール CLS の基盤は最小限の追加作業で CRA 要件の大部分をカバー

### マイルストーン 3.2 ：英国 PSTI 法アライメント 🔲 計画中

英国の製品セキュリティ・通信インフラ（ PSTI ）法は ETSI EN 303 645 と整合しており、 2026 年 1 月に発効しました。 CLS 準拠を前提とすれば、追加実装はほぼ不要です。

- CLS レベル 3 と英国 PSTI 要件のギャップ分析
- PSTI コンプライアンス声明文書
- [#97](https://github.com/edgesentry/edgesentry-rs/issues/97)を参照してください

### マイルストーン 3.3 ： IEC 62443-4-2 + ハードウェア RoT 🔲 計画中

IEC 62443-4-2 は、 CII （重要情報インフラ）および OT 市場向けのコンポーネントレベル要件を規定します。ハードウェア Root of Trust （ TPM/HSM ）・ RBAC ・特権アクセス管理（ PAM ）が必要であり、 ETSI EN 303 645 とは異なる要件です。

- IEC 62443-4-2 コンポーネント要件マッピング
- ハードウェアバックの鍵保存（ CLS レベル 4 ）のための`edgesentry-bridge`による HSM 統合
- デプロイ担当者向け RBAC/PAM 設計ガイダンス
- [#54](https://github.com/edgesentry/edgesentry-rs/issues/54)および[#98](https://github.com/edgesentry/edgesentry-rs/issues/98)を参照してください

### マイルストーン 3.4 ： CCoP 2.0 / MTCS ティア 3 🔲 計画中

シンガポールのサイバーセキュリティ実践規範 2.0 （ CCoP 2.0 ）は、 CII セクター向けの運用コンプライアンス要件です。プラットフォームに政府契約を対象としたクラウドまたは SaaS コンポーネントがある場合、 MTCS ティア 3 が適用されます。

- CCoP 2.0 運用要件マッピング
- クラウドデプロイシナリオにおける MTCS ティア 3 適用性評価
- [#99](https://github.com/edgesentry/edgesentry-rs/issues/99)を参照してください

### マイルストーン 3.5 ：形式的検証とハードニング 🔲 計画中

- CLS レベル 4 で求められるサードパーティバイナリ分析に耐えるための、高度なメモリ安全性と脆弱性ハードニング

### マイルストーン 3.6 ： AI ロボティクス向けリファレンスアーキテクチャ 🔲 計画中

- 自律移動ロボット（ AMR ）および点検ドローンにおける改ざん検知可能な意思決定監査のリファレンス設計

---

## 持続可能なエコシステム戦略

DuckDB モデルに倣い、プラットフォームではなくライブラリを通じて普及する、軽量な埋め込み可能コアを目指します。

1. **「インプロセス」セキュリティ** — DuckDB が Python や Java プロセスに組み込まれるように、 OS やハードウェアを問わず既存の C++アプリケーション内にライブラリとして組み込む。

2. **オープンコンプライアンス** — 「セキュリティを達成する方法」の知識を OSS 化し、単一ベンダーがコンプライアンス経路を支配しないようにする。標準を公共のインフラとして位置づける。

3. **コラボレーティブラーニング** — 次世代の IoT セキュリティエンジニアを育成するための、企業を超えた共同学習環境として Rust コードベースを提供する。
