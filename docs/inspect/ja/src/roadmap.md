# EdgeSentry-Inspect — ロードマップ

## 基盤（trilink-core リポジトリ）

以下は EdgeSentry-Inspect の全マイルストーンの前提条件です。
[`trilink-core`](https://github.com/edgesentry/trilink-core) リポジトリで追跡・実装されます。

| イシュー | 成果物 | 状態 |
|---|---|---|
| [#30](https://github.com/edgesentry/trilink-core/issues/30) | `PointCloud`・`DepthMap`・`HeightMap` 型定義 | Todo |
| [#31](https://github.com/edgesentry/trilink-core/issues/31) | `project_to_depth_map`（3D → 深度マップ） | Todo |
| [#32](https://github.com/edgesentry/trilink-core/issues/32) | `project_to_height_map`（3D → 高さマップ） | Todo |
| [#33](https://github.com/edgesentry/trilink-core/issues/33) | `docs/math.md` 順投影セクションの追記 | Todo |
| [#34](https://github.com/edgesentry/trilink-core/issues/34) | 投影 → 逆投影ラウンドトリップテスト | Todo |

`#30`、`#31`、`#32`、`#34` がマージされるまで M2 を開始しないこと。

---

## M2 — IFC ローダーと偏差エンジン

**目標:** スキャン点群と IFC 設計ファイルから、点ごとのミリ単位偏差を計算できること。

**成果物:**

- `Cargo.toml` — ワークスペースルート。メンバー: `crates/edgesentry-inspect`
- `src/ifc.rs` — IFC ジオメトリを `Vec<Point3D>` として読み込む（設計参照点群）
- `src/deviation.rs` — k-d ツリー最近傍偏差計算。設定可能なしきい値
- `src/report.rs` — JSON レポートシリアライゼーション（スキーマは [architecture.md](architecture.md) を参照）
- 統合テスト: サンプル IFC フィクスチャを読み込み → 既知のスキャン点群に対して偏差を計算 → `compliant_pct`・`max_deviation_mm`・`mean_deviation_mm` を検証

---

## M3 — ヒートマップ生成

**目標:** 点ごとの偏差を色にマッピングし、深度マップ投影を使って 2D に位置付けた PNG ヒートマップを生成できること。

**成果物:**

- `src/heatmap.rs` — 偏差 → RGB 色（緑 ≤ しきい値、黄 2 倍、赤 4 倍以上）→ `image` クレートで PNG 出力
- `trilink-core::project_to_depth_map` を再利用して各色点を 2D に配置
- 統合テスト: 既知の偏差値 → 出力 PNG の期待ピクセル位置・色を検証

---

## M4 — 現場 PC パイプライン（CLI）

**目標:** 点群から偏差レポートまでの現場エンドツーエンドパイプラインを単一の CLI コマンドで実行できること。

**成果物:**

- `src/main.rs` — CLI: `edgesentry-inspect scan --config config.toml`
- 接続: 点群インジェスト（`trilink-core::FrameSource`）→ `project_to_depth_map` → AI 推論クライアント → `unproject` → 偏差計算 → ヒートマップ → レポート
- 設定: IFC ファイルパス、`inference.mode`（`builtin` | `http`）、推論エンドポイント URL（`http` 時）、偏差しきい値、出力ディレクトリ
- エンドツーエンドテスト: `MockSource` + モック推論サーバーを使用。レポートが生成され全フィールドが正しく、ヒートマップ PNG が書き出されることを検証

---

## M5 — クラウド同期

**目標:** 偏差レポートとヒートマップを改ざん防止の監査ストアにアップロードし、構造変化フラグを発報できること。

**成果物:**

- `src/sync.rs` — S3 互換アップロード（Object Lock WORM）。しきい値の 2 倍を超える異常検出時に SQS または MQTT へ構造変化フラグを発報
- 統合テスト: モック S3 + モック SQS → しきい値超過の異常に対してレポートがアップロードされフラグが発報されること、しきい値未満の場合はフラグが発報されないことを検証

---

## M6 — 組み込み推論モデル

**目標:** 軽量 ONNX 欠陥検出モデルを EdgeSentry-Inspect に同梱し、外部サーバーなしで `inference.mode = "builtin"` がすぐに使えること。

**成果物:**

- `src/inference/mod.rs` — `InferenceBackend` トレイト。`inference.mode` に基づいて組み込みまたは HTTP に振り分け
- `src/inference/builtin.rs` — ONNX Runtime ランナー（`ort` クレート）。バンドル済みモデルの重みを読み込む
- `src/inference/http.rs` — M4 の HTTP クライアントを同モジュールに抽出（組み込みとの対称性）
- `models/detect.onnx` — `surface_void`・`misalignment`・`rebar_exposure` をカバーする初期モデル
- 統合テスト: サンプル深度マップで組み込みモデルを実行 → 検出結果が空でなく、クラスラベルが正しいことを検証

---

## 依存グラフ

```
trilink-core #30, #31, #32, #34（基盤 — 最初に完了させること）
    └── M2（IFC ローダー + 偏差エンジン）
         └── M3（ヒートマップ生成）
              └── M4（現場 PC パイプライン CLI）
                   ├── M5（クラウド同期）
                   └── M6（組み込み推論モデル）
```
