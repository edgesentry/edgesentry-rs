# EdgeSentry

EdgeSentry は、センサーから封印までのコンプライアンスパイプラインを構築するための再利用可能な Rust クレート群と統合 CLI（`eds`）のコレクションです。

## 7ステップパイプライン

現実のデータを取得し、規制と照合し、逸脱を説明し、改ざん検知可能な記録を生成する必要があるあらゆるドメインに、同じパターンが適用されます。

| ステップ | 役割 | クレート | CLI |
|------|------|-------|-----|
| 1 - 取込 | センサーデータの取得またはドキュメントの解析 | `edgesentry-ingest` / `edgesentry-parse` | `eds ingest` / `eds parse` |
| 2 - 計算 | 物理・ジオメトリ演算の適用 | `edgesentry-compute` | `eds compute` |
| 3 - 評価 | ルールに対する計測値の比較 | `edgesentry-evaluate` | `eds evaluate` |
| 4 - 分析 | 評価結果からパターンを発見 | `edgesentry-assess` | `eds assess` |
| 5 - 説明 | 根拠に基づいた平易なテキストの生成 | `edgesentry-explain` | `eds explain` |
| 6 - 文書化 | 結果をレポートまたはドキュメントに整形 | `edgesentry-report` / `edgesentry-document` | `eds report` / `eds document` |
| 7 - 封印 | 改ざん検知のためのレコードの署名とチェーン化 | `edgesentry-audit` | `eds audit` |

## クイックリンク

- [パイプラインドキュメント](introduction.ja.md)
- [クイックスタート - 安全監視](quickstart-safety-monitoring.ja.md)
- [クイックスタート - ドキュメントコンプライアンス](quickstart-document-compliance.ja.md)
- [CLIリファレンス](cli-reference.ja.md)
