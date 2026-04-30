# IoTセキュリティ規格 — edgesentry-rs

EdgeSentry-RSは以下のIoTセキュリティ認証規格を満たすよう設計されています。
このフォルダはワークスペース全体をカバーするコンプライアンス証拠・脅威モデル・SBOMアーティファクトの
単一ソースです。

---

## 対象規格

| 規格 | 発行機関 | スコープ | 対象レベル |
|---|---|---|---|
| [SS 711:2025](https://www.singaporestandardseshop.sg/) | シンガポール標準評議会 | 国家IoTサイバーセキュリティ標準；4つの設計原則 | 全原則 |
| [CLS Level 3 / Level 4](https://www.csa.gov.sg/our-programmes/certification-and-labelling-schemes/cybersecurity-labelling-scheme) | CSAシンガポール | IoT製品のサイバーセキュリティラベリング | Level 3（現在）、Level 4（計画） |
| [ETSI EN 303 645](https://www.etsi.org/deliver/etsi_en/303600_303699/303645/02.01.01_60/en_303645v020101p.pdf) | ETSI | 欧州IoTサイバーセキュリティベースライン；13条項 | 全13条項マッピング済 |
| [iM8](https://www.imda.gov.sg/regulations-and-licensing-listing/IMDA-Standards-Collection/iM8) | IMDAシンガポール | IoTサイバーセキュリティガイド；ベンダー開示チェックリスト | 全チェックリスト |
| [JC-STAR](https://www.soumu.go.jp/main_sosiki/cybersecurity/jc-star/) | 総務省 | 日本のIoTセキュリティ適合性評価制度（STAR-1 / STAR-2） | STAR-1 + STAR-2 マッピング済 |
| [IMO MSC.428(98)](https://www.imo.org/en/OurWork/Security/Pages/Cyber-security.aspx) | IMO | 安全管理システムにおける海事サイバーリスク管理 | 参照のみ |

---

## ドキュメントマップ

| ドキュメント | 内容 |
|---|---|
| [STRIDE脅威モデル](threat-model.ja.md) | 攻撃面の完全分析：なりすまし・改竄・否認・情報漏洩・DoS・権限昇格をソースコードにマッピング |
| [CLS / ETSI / JC-STAR 準拠トレーサビリティマトリクス](cls-traceability.ja.md) | CLS Level 3/4、ETSI EN 303 645、iM8、JC-STARの各条項と実装の対応表 |
| [SBOMとベンダー開示チェックリスト](sbom.ja.md) | ソフトウェア部品表（CycloneDX形式）、サプライチェーン監視、IMDAベンダー開示チェックリスト回答 |

---

## SS 711:2025 — 4つの設計原則

| 原則 | 要件 | 状態 | 実装 |
|---|---|---|---|
| Secure by Default（デフォルト安全） | デバイス固有ID、署名付きOTAアップデート | ✅ | `identity.rs`、`update.rs` |
| Rigour in Defence（防御の厳密性） | STRIDE脅威モデル、改竄検知 | ✅ | ハッシュチェーン（`integrity.rs`）＋[脅威モデル](threat-model.ja.md) |
| Accountability（説明責任） | 監査証跡、オペレーションログ | ✅ | `ingest/`（AuditLedger、OperationLog） |
| Resiliency（回復力） | デフォルト拒否ネットワーク、DoS保護 | ✅ | `ingest/network_policy.rs` |

---

## カバレッジサマリー

| レベル | 総条項数 | ✅ 実装済 | ⚠️ 部分的 | 🔲 計画中 | ➖ スコープ外 |
|---|---|---|---|---|---|
| CLS Level 3 | 11 | 6 | 2 | 0 | 3 |
| CLS Level 4 | 1 | 0 | 0 | 1 | 0 |
| JC-STAR追加要件 | 1 | 1 | 0 | 0 | 0 |

条項別の詳細：[CLS / ETSI / JC-STAR 準拠トレーサビリティマトリクス](cls-traceability.ja.md)

---

## 他のドキュメントフォルダとの関係

- **`docs/audit/`** — auditクレート内部：`AuditRecord`設計、鍵管理、CLI、デプロイ
- **`docs/legal/`** — 法的に認められる監査記録の要件（7要件分析、信頼できるタイムスタンプ、RFC 3161 TSAロードマップ）
- **`docs/pipeline/`** — 7ステップパイプラインとエッジ/クラウド分離設計
