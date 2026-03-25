# Inspect — ロードマップ

## リリーストラック

| トラック | スコープ | 対象者 |
|---|---|---|
| **OSS** | trilink-core（3D/2D投影・偏差エンジン）、edgesentry-audit、edgesentry-inspect（CLI） | 開発者・研究者 |
| **商用 — App** | Inspect App（Tauri/GUI）、3D ヒートマップ、現場写真、BIM 連携 UI | 現場監督・検査員 |
| **商用 — Reporting** | CONQUAS・国土交通省準拠の自動レポート生成 | 行政機関・監査機関 |
| **商用 — Partnership** | パートナー企業向け高度な推論・センサー統合プラグイン | パートナー企業 |

**[OSS]** と記載されたマイルストーンはオープンソースとして公開します。**[商用]** と記載されたマイルストーンは OSS 層の上に構築されるクローズドソース製品です。

## エコシステム戦略

DuckDB モデルに倣い、アルゴリズム・ツール・仕様をできる限りオープンにすることで、ロックインではなくエコシステムへの採用を通じてデファクトスタンダードを目指す。

**OSS コアを最大開放する理由**

偏差計算エンジン・投影アルゴリズム・CLI を完全オープンにすることで、研究者・現場エンジニア・規制当局・パートナー企業が独立して検証・統合・拡張できる。アルゴリズムの透明性そのものが信頼の根拠となり、「商用ツールに依存しない建設検査の公共インフラ」としての地位を確立する。

**商用層は OSS 基盤の持続性を支える**

現場運用に必要な「即時の書類作成」「規制準拠レポート」「パートナーセンサー統合」は、OSS CLI だけでは実現しにくい領域。Inspect App・準拠レポートエンジン・パートナープラグインがここをカバーし、OSS 開発を継続するための資金基盤となる。商用製品は OSS を置き換えるのではなく、OSS の上に構築された付加価値層として位置づける。

**規制当局との共創**

BCA・CSA・国土交通省といった規制当局は競争相手ではなく、建設品質標準を共に育てるパートナーである。CLS / JC-STAR / CONQUAS への準拠を先行実装することは、これらの機関が定める標準を真剣に受け止めているというコミットメントの表明であり、OSS コアの品質を独立した第三者に検証してもらう機会でもある。この信頼関係が国際的なエコシステム採用を加速する。

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
| [#39](https://github.com/edgesentry/trilink-core/issues/39) | `HeightMap` の次元命名統一（`cols/rows` → `width/height`） | 完了 |
| [#40](https://github.com/edgesentry/trilink-core/issues/40) | 座標精度の設計決定（`Point3D` は `f32` を維持） | 完了 |
| [#38](https://github.com/edgesentry/trilink-core/issues/38) | `Transform4x4` / `Point3D` への glam 採用（SIMD・行列逆変換） | 完了 |

基盤の全イシューがマージ済みです。M2 の実装も完了しています。M3 の実装を開始できます。

---

## M2 — IFC ローダーと偏差エンジン \[OSS\] ✅ 実装済み

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

## M4.5 — Inspect App（可視化プロトタイプ）\[商用\] *(M5/M6 と並行)*

**目標:** 現場デモ向けのインタラクティブな 3D ヒートマップビューア。CLI パイプラインと並行して動作し、M5・M6 への依存はない。

**アーキテクチャ（Python × JS ハイブリッド / Tauri 基盤）:**

| レイヤー | 技術 | 役割 |
|---|---|---|
| Frontend | JavaScript / Three.js + Potree Core | 数百万点の点群描画、Vertex Color ヒートマップ、BIM 連携 UI |
| Backend (Sidecar) | Python / IfcOpenShell | IFC パース（幾何形状 + `GlobalId` 属性情報）→ JSON 出力 |
| Core Engine | Rust / trilink-core | 偏差計算、座標変換、監査ハッシュ署名 |

**成果物:**

- Tauri デスクトップシェル（Windows / macOS / Linux）— Python 環境を `sidecar` として同梱し、単一実行ファイルとして配布可能
- Potree Core: ブラウザ上で数百万点の点群を効率的に表示
- Python sidecar: IfcOpenShell で IFC から部材ごとの `GlobalId` と属性情報を JSON 抽出
- メタデータオーバーレイ: 部材クリック時に `GlobalId` をキーに属性情報を Tooltip/サイドバーに表示
- Vertex Color ヒートマップ: 偏差値を RGB にマッピングし Three.js メッシュに直接反映（PNG ではなくリアルタイム描画）
- 現場写真ビューア: スキャン現場の写真を 3D ビューと並列表示
- M4 が生成した JSON レポートをそのまま利用 — Rust 側のコード変更は不要

> **このタイミングで実施する理由:** CLI の出力は現場監督や検査員にとって直感的ではありません。視覚的なデモは PoC 承認を加速します。このマイルストーンは並行開発であり、M5・M6 をブロックしません。
>
> **デモの価値:** 「設計データ（BIM 属性）」と「現場の実測（偏差）」が 3D 空間で一体化している様子を、OSS スタック（Tauri + Python + Three.js）だけで実現。商用ツールへの依存なし。

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

## M7 — 準拠レポート自動生成 \[商用\]

**目標:** CONQUAS（シンガポール）および国土交通省（日本）の検査基準に準拠した PDF レポートを、M4 の偏差データから自動生成できること。

**背景:** BCA（シンガポール建設庁）・CSA・国土交通省が定める規制標準に先行して準拠することで、規制当局との信頼関係を構築し、OSS コアの品質を独立した第三者として証明してもらう。規制機関はパートナーであり、共に建設品質の標準を育てる存在として位置づける。

**成果物:**

- CONQUAS 準拠レポートテンプレート — BCA（Building and Construction Authority）提出形式
- 国土交通省準拠レポートテンプレート — 日本の建設品質管理基準に対応
- `report.json` とヒートマップ PNG から PDF を自動生成するレポートエンジン
- edgesentry-audit 連携による電子署名付き改ざん防止レポート出力

**前提条件:** M4（レポート JSON 生成）、M4.5（Inspect App）

---

## M8 — パートナーセンサー統合プラグイン \[商用\]

**目標:** パートナー企業のセンサー・推論プラットフォームと Inspect を直接統合し、高度な欠陥検出と専用センサーデータのインジェストを実現する。

**成果物:**

- `plugins/<partner>/` — パートナー AI 推論エンジンとの統合インターフェース（高精度欠陥検出）
- パートナーセンサープラットフォームからの点群データ直接インジェスト
- プラグイン API: M6 の `InferenceBackend` トレイト拡張（組み込みモデルと対称設計）
- パートナー向けプラグイン SDK ドキュメント

**前提条件:** M6（`InferenceBackend` トレイト）

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

## 監査層 — ISO 19650 対応

ISO 19650 情報コンテナスキーマの実装（BIM ステータス遷移・準拠ペイロード・第三者 BIM ツールとの相互運用性）は、edgesentry-rs クレートの責務です。

実装計画は **[edgesentry-audit ロードマップ — Milestone 2.7](../../audit/en/src/roadmap.md)** で追跡されます。

---

## 依存グラフ

```
trilink-core #30, #31, #32, #33, #34（基盤 — 完了）
    └── M2（IFC ローダー + 偏差エンジン）          [OSS]
         └── M3（ヒートマップ生成）                 [OSS]
              └── M4（現場 PC パイプライン CLI）     [OSS]
                   ├── M4.5（Inspect App — Python×JS ハイブリッド）  [商用、並行]
                   │    └── M7（準拠レポート自動生成）               [商用]
                   ├── M5（クラウド同期）            [OSS]
                   ├── M6（組み込み推論モデル）      [OSS]
                   │    └── M8（パートナーセンサー統合プラグイン）   [商用]
                   └── デモパイプライン（オープンデータセット + CLI）
```
