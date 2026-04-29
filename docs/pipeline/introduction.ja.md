# はじめに

EdgeSentry は、センサーから封印までのコンプライアンスパイプラインを構築するための Rust ツールキットです。現実のデータを取得し、規制と照合し、逸脱を説明し、改ざん検知可能な記録を生成する必要があるあらゆるドメインに、同じ7ステップのパターンが適用されます。

## 7つのステップ

| ステップ | 役割 | CLI | クレート |
|------|------|-----|-------|
| ステップ 1 - 取込 | 構造化センサーデータの取得または非構造化ドキュメントの解析 | `eds ingest` / `eds parse` | `edgesentry-ingest` / `edgesentry-parse` |
| ステップ 2 - 計算 | 生の計測値に対する物理・ジオメトリ演算の適用 | `eds compute` | `edgesentry-compute` |
| ステップ 3 - 評価 | 規制または設計仕様に対する計測値の比較 | `eds evaluate` | `edgesentry-evaluate` |
| ステップ 4 - 分析 | 時系列での評価結果の相関分析によるパターン発見 | `eds assess` | `edgesentry-assess` |
| ステップ 5 - 説明 | 分析結果を根拠に基づいた平易なテキストに変換 | `eds explain` | `edgesentry-explain` |
| ステップ 6 - 文書化 | 結果をレポートまたは公式ドキュメントに整形 | `eds report` / `eds document` | `edgesentry-report` / `edgesentry-document` |
| ステップ 7 - 封印 | 各レコードへの署名と改ざん検知のためのチェーン化 | `eds audit` | `edgesentry-audit` |

## 設計原則

**パイプラインの各ステージは独立したプロセスです。** 各 `eds` コマンドはファイルから JSONL を読み込み、ファイルに JSONL を書き出します。ステージ間での共有インメモリ状態はありません。これにより、各ステージを独立してテストでき、パイプライン全体をどの時点からでも再現できます。

**評価（evaluate）vs 分析（assess）。** 評価はファクトチェック ── この単一の計測値がルールに違反しているか？ 分析はインサイト ── 多くの評価を横断してどんなパターンが浮かび上がるか？ 軸は単一イベントか複数イベントかではなく、事実か解釈かです。

**エンジンはドメイン非依存です。** 同じ7つのクレートが、倉庫の安全監視、海事ドキュメントコンプライアンス、3Dポイントクラウドの偏差分析を処理します。ドメインはプロファイルとテンプレートに存在し、エンジンには存在しません。

## ステージ間データ形式

各ステージはヘッダ付き JSONL ファイルを書き出します ── 1行目はスキーマヘッダ、以降の行がレコードです。

```json
{"eds_schema":"eds.entity-frame","version":"0.1"}
{"timestamp_ms":1000,"entity_id":"FL-01","entity_type":"forklift","x":25.0,"y":8.0,"vx":-1.0,"vy":0.0}
```

ヘッダはレコードが読み込まれる前に `JsonlReader` によって検証され、スキーマの不一致を早期に検出します。

## 提供スコープ（フェーズ 1〜3）

| フェーズ | PR | 追加クレート |
|-------|----|--------------|
| 1 | [#270](https://github.com/edgesentry/edgesentry-rs/pull/270) | edgesentry-ingest, edgesentry-compute, edgesentry-evaluate, edgesentry-profile |
| 2 | [#288](https://github.com/edgesentry/edgesentry-rs/pull/288) | edgesentry-store, edgesentry-assess, edgesentry-explain + UDP ingest |
| 3 | [#289](https://github.com/edgesentry/edgesentry-rs/pull/289) | edgesentry-report, edgesentry-parse, edgesentry-document |
