# Inspect — ロードマップ

## リリーストラック

| トラック | スコープ | 対象者 |
|---|---|---|
| **OSS** | trilink-core、edgesentry-audit、edgesentry-inspect（CLI） | 開発者・研究者 |
| **商用** | Inspect App（GUI）、改ざん防止監査コネクタ | 現場監督・行政機関・監査機関 |

**[OSS]** と記載されたマイルストーンはオープンソースとして公開します。**[商用]** と記載されたマイルストーンは OSS 層の上に構築されるクローズドソース製品です。

---

## 基盤（trilink-core リポジトリ）

以下は Inspect の全マイルストーンの前提条件です。
[`trilink-core`](https://github.com/edgesentry/trilink-core) リポジトリで追跡・実装されます。

| イシュー | 成果物 | 状態 |
|---|---|---|
| [#30](https://github.com/edgesentry/trilink-core/issues/30) | `PointCloud`・`DepthMap`・`HeightMap` 型定義 | 完了 |
| [#31](https://github.com/edgesentry/trilink-core/issues/31) | `project_to_depth_map`（3D → 深度マップ） | 完了 |
| [#32](https://github.com/edgesentry/trilink-core/issues/32) | `project_to_height_map`（3D → 高さマップ） | 完了 |
| [#33](https://github.com/edgesentry/trilink-core/issues/33) | `docs/math.md` 順投影セクションの追記 | 完了 |
| [#34](https://github.com/edgesentry/trilink-core/issues/34) | 投影 → 逆投影ラウンドトリップテスト | 完了 |

基盤の全イシューがマージ済みです。M2 の実装を開始できます。

---

## M2 — IFC ローダーと偏差エンジン \[OSS\]

**目標:** スキャン点群と IFC 設計ファイルから、点ごとのミリ単位偏差を計算できること。

**成果物:**

- `Cargo.toml` — ワークスペースルート。メンバー: `crates/edgesentry-inspect`
- `src/ifc.rs` — IFC ジオメトリを `Vec<Point3D>` として読み込む（設計参照点群）
- `src/deviation.rs` — k-d ツリー最近傍偏差計算。設定可能なしきい値
- `src/report.rs` — JSON レポートシリアライゼーション（スキーマは [architecture.md](architecture.md) を参照）
- 統合テスト: サンプル IFC フィクスチャを読み込み → 既知のスキャン点群に対して偏差を計算 → `compliant_pct`・`max_deviation_mm`・`mean_deviation_mm` を検証

---

## M3 — ヒートマップ生成 \[OSS\]

**目標:** 点ごとの偏差を色にマッピングし、深度マップ投影を使って 2D に位置付けた PNG ヒートマップを生成できること。

**成果物:**

- `src/heatmap.rs` — 偏差 → RGB 色（緑 ≤ しきい値、黄 2 倍、赤 4 倍以上）→ `image` クレートで PNG 出力
- `trilink-core::project_to_depth_map` を再利用して各色点を 2D に配置
- 統合テスト: 既知の偏差値 → 出力 PNG の期待ピクセル位置・色を検証

---

## M4 — 現場 PC パイプライン（CLI） \[OSS\]

**目標:** 点群から偏差レポートまでの現場エンドツーエンドパイプラインを単一の CLI コマンドで実行できること。

**成果物:**

- `src/main.rs` — CLI: `edgesentry-inspect scan --config config.toml`
- 接続: 点群インジェスト（`trilink-core::FrameSource`）→ `project_to_depth_map` → AI 推論クライアント → `unproject` → 偏差計算 → ヒートマップ → レポート
- 設定: IFC ファイルパス、`inference.mode`（`builtin` | `http`）、推論エンドポイント URL（`http` 時）、偏差しきい値、出力ディレクトリ
- エンドツーエンドテスト: `MockSource` + モック推論サーバーを使用。レポートが生成され全フィールドが正しく、ヒートマップ PNG が書き出されることを検証

---

## M4.5 — 可視化プロトタイプ \[商用\] *(M5/M6 と並行)*

**目標:** 現場デモ向けのインタラクティブな 3D ヒートマップビューア。CLI パイプラインと並行して動作し、M5・M6 への依存はない。

**成果物:**

- Tauri デスクトップシェル（Windows / macOS / Linux）上に Three.js WebGL レンダラーを組み込む
- M4 が生成した JSON レポートと PNG ヒートマップを読み込み、偏差点群を色付きで描画
- Rust コードの変更は不要 — M4 の出力ファイルをそのまま利用

> **このタイミングで実施する理由:** CLI の出力は現場監督や検査員にとって直感的ではありません。視覚的なデモは PoC 承認を加速します。このマイルストーンは並行開発であり、M5・M6 をブロックしません。

---

## M5 — クラウド同期 \[OSS\]

**目標:** 偏差レポートとヒートマップを S3 互換ストアにアップロードし、構造変化フラグを発報できること。

**成果物:**

- `src/sync.rs` — S3 互換アップロード（標準 PUT）。しきい値の 2 倍を超える異常検出時に SQS または MQTT へ構造変化フラグを発報
- 統合テスト: モック S3 + モック SQS → しきい値超過の異常に対してレポートがアップロードされフラグが発報されること、しきい値未満の場合はフラグが発報されないことを検証

> **商用拡張:** Object Lock（WORM）の適用と API を介した公式証跡の発行は OSS リリースのスコープ外です。これらは商用の改ざん防止監査コネクタとして提供されます。

---

## M6 — 組み込み推論モデル \[OSS\]

**目標:** 軽量 ONNX 欠陥検出モデルを Inspect に同梱し、外部サーバーなしで `inference.mode = "builtin"` がすぐに使えること。

**成果物:**

- `src/inference/mod.rs` — `InferenceBackend` トレイト。`inference.mode` に基づいて組み込みまたは HTTP に振り分け
- `src/inference/builtin.rs` — ONNX Runtime ランナー（`ort` クレート）。バンドル済みモデルの重みを読み込む
- `src/inference/http.rs` — M4 の HTTP クライアントを同モジュールに抽出（組み込みとの対称性）
- `models/detect.onnx` — `surface_void`・`misalignment`・`rebar_exposure` をカバーする初期モデル
- 統合テスト: サンプル深度マップで組み込みモデルを実行 → 検出結果が空でなく、クラスラベルが正しいことを検証

---

## デモパイプライン

**目標:** オープンデータセットを使って Inspect CLI のエンドツーエンドデモを自己完結した形で実行できること。本番ハードウェアや本番データは不要。

**前提条件:** M2、M3、M4（CLI がビルド済みで PATH が通っていること）。

**手順:**

1. 公開 IFC ファイル（buildingSMART BIMNet ギャラリー）と屋内 LiDAR スキャン（S3DIS データセット）をダウンロードする。
2. IfcOpenShell で IFC 表面をサンプリングし、参照点群を生成する。
3. Open3D で 15 mm の意図的な変形を加え、既知の欠陥を持つシミュレーションスキャンを作成する。
4. `edgesentry-inspect scan --config config.toml` を実行する。CLI は IFC を読み込み、偏差を計算し、深度マップに投影して HTTP 推論サーバーを呼び出し、検出結果を逆投影してヒートマップを生成し、JSON レポートを出力する。
5. `report.json`（`compliant_pct`・`max_deviation_mm`・`mean_deviation_mm`）と PNG ヒートマップを確認し、シミュレーションした欠陥が検出・定量化されていることを検証する。

詳細な手順は [デモパイプライン](demo.md) を参照してください。

---

## 監査層 — ISO 19650 対応 \[計画中\]

現在の監査層は検査イベントのハッシュチェーンを記録します。計画中の拡張では、各レコードを ISO 19650 の**情報コンテナ**として再定義し、BIM ステータス遷移（WIP → Shared → Published）の管理と準拠ペイロードスキーマを追加します。これにより第三者 BIM ツールとの相互運用性が生まれ、建設検査トレーサビリティのデファクトスタンダードを目指します。

この拡張は上記の Inspect マイルストーンとは別に追跡されます。

---

## 依存グラフ

```
trilink-core #30, #31, #32, #33, #34（基盤 — 完了）
    └── M2（IFC ローダー + 偏差エンジン）          [OSS]
         └── M3（ヒートマップ生成）                 [OSS]
              └── M4（現場 PC パイプライン CLI）     [OSS]
                   ├── M4.5（可視化プロトタイプ）    [商用、並行]
                   ├── M5（クラウド同期）            [OSS]
                   ├── M6（組み込み推論モデル）      [OSS]
                   └── デモパイプライン（オープンデータセット + CLI）
```
