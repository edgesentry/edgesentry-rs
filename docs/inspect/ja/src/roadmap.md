# Inspect — ロードマップ

## リリーストラック

| トラック | スコープ | 対象者 |
|---|---|---|
| **OSS**（本リポジトリ） | trilink-core（3D/2D 投影・偏差エンジン）、edgesentry-audit、edgesentry-inspect（CLI） | 開発者・研究者 |
| **商用**（クローズド商用リポジトリ） | Inspect App（Tauri/GUI）、準拠レポート、パートナーセンサープラグイン | 現場監督・検査員・規制機関 |

本ドキュメントのすべてのマイルストーンはオープンソースとして公開します。商用マイルストーンは商用コンプライアンス層で追跡します。

## エコシステム戦略

DuckDB モデルに倣い、アルゴリズム・ツール・仕様をできる限りオープンにすることで、ロックインではなくエコシステムへの採用を通じてデファクトスタンダードを目指す。

**OSS コアを最大開放する理由**

偏差計算エンジン・投影アルゴリズム・CLI を完全オープンにすることで、研究者・現場エンジニア・規制当局・パートナー企業が独立して検証・統合・拡張できる。アルゴリズムの透明性そのものが信頼の根拠となり、「商用ツールに依存しない建設検査の公共インフラ」としての地位を確立する。

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

## M5 — クラウド同期 \[OSS\]

**目標:** 偏差レポートとヒートマップを S3 互換ストアにアップロードし、構造変化フラグを発報できること。

**成果物:**

- `src/sync.rs` — S3 互換アップロード（標準 PUT）。しきい値の 2 倍を超える異常検出時に SQS または MQTT へ構造変化フラグを発報
- 統合テスト: モック S3 + モック SQS → しきい値超過の異常に対してレポートがアップロードされフラグが発報されること、しきい値未満の場合はフラグが発報されないことを検証

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

## 既知の制約

現在の設計に内在する制約事項です。制約の詳細は [`trilink-core/docs/limitations.md`](https://github.com/edgesentry/trilink-core/blob/main/docs/limitations.md) に記載されています。

| # | 制約 | 影響マイルストーン | 回避策 |
|---|---|---|---|
| L1 | **単一視点遮蔽** — Z バッファ投影はキャプチャ姿勢から見えない表面を破棄する | M3, M4 | 投影前に複数姿勢を統合する。深度マップの NaN 割合を監視する |
| L2 | **高さマップは突出物のみ** — 最大 Z 集計は陥没（剥落・断面欠損）を検出できない | M3 | 陥没検出には偏差エンジン（M2）を使用する。高さマップは補助的な位置付けとする |
| L3 | **曲面の逆投影バイアス** — unproject は平面を仮定する。円柱・アーチでの相対誤差は約 11.7%（平面の約 2.5% と比較） | M4, M6 | 高曲率領域の検出にフラグを立てる。拡大した許容誤差を適用する |
| L4 | **ローカルフレーム外の f32 精度** — 座標はローカル接平面フレームで表現する必要がある。UTM/WGS-84 入力は約 12 mm 刻みに暗黙的に劣化する | 基盤 | `Point3D` 構築前にサイト原点を引く。詳細は `trilink-core/docs/math.md` を参照 |
| L5 | **深度のみの推論** — 組み込み ONNX モデルは深度マップのみ使用。RGB チャンネルなし。F1 は約 76%（RGB-D 融合の約 87% と比較） | M6 | `InferenceBackend` の RGB-D 拡張を計画中。`FusionPacket.image_jpeg` は既に利用可能 |
| L6 | **フォールバック深度による測位劣化** — センサー値なしの場合 `fallback_depth_m = 2.0 m` を使用。位置誤差 ∝ `\|true_depth − 2.0\|` | M4 | 常にレンジセンサーと共登録する。フォールバック検出は位置アノテーションとしてのみ扱う |
| L7 | **姿勢バッファのデッドゾーン** — キャプチャから 200 ms 超過、または 33 秒のバッファウィンドウ超過で到着した推論結果はサイレントに破棄される | 基盤 | `world_pos = None` の発生率を監視する。許容誤差超過と破棄を区別してログに記録する |
| L8 | **近接目視検査同等性は未達成** — MLIT/CONQUAS の同等性テストなし。OSS 層での IFC 4.3 メタデータ書き戻しも未実装 | — | 商用コンプライアンス層で対応 |

### RGB-D 融合（M6 拡張）

組み込み推論モデル（M6）は、深度マップに加えてオプションの RGB チャンネルを受け付けられるよう拡張予定です。`FusionPacket` はすでに `image_jpeg` を保持しています。主な変更は推論モジュールとモデルの再トレーニングです。

| 入力 | F1 |
|---|---|
| 2D RGB のみ | 67.6% |
| 3D 深度のみ | 76.0% |
| RGB-D 融合 | **86.7%** |

このアイテムは M6 の一部として追跡されます。`InferenceBackend` トレイトのシグネチャは変更しません。RGB テンソルは追加のオプションチャンネルとして渡されます。

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
                   ├── M5（クラウド同期）            [OSS]
                   ├── M6（組み込み推論モデル）      [OSS]
                   └── デモパイプライン（オープンデータセット + CLI）

商用マイルストーン（M4.5、M7、M8）および 2D/1D フェーズ 2 は商用コンプライアンス層で追跡します。
```

開発優先順位は 3D デモ → 2D（MPA/JTC 向け YOLO11/SAM 2）→ 1D（NEA/PUB 向け PatchTST/iTransformer）の順です。
