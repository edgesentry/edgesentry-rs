# デモパイプライン

このページでは、オープンデータセットと Inspect CLI を使って、自己完結した概念実証（PoC）デモを構築する手順を説明します。本番データが用意できる前の技術評価・現場デモでの利用を想定しています。

---

## オープンデータセット

| アセット | ソース | 備考 |
|---|---|---|
| IFC 設計モデル | [buildingSMART BIMNet ギャラリー](https://awards.buildingsmart.org/gallery/) | BIM 受賞作品として公開された IFC ファイル |
| 3D 点群 | [S3DIS（Stanford Large-Area Indoor Spaces）](https://www.open3d.org/docs/latest/python_api/open3d.ml.tf.datasets.S3DIS.html) | 実建物の屋内 LiDAR スキャン。構造検査シナリオに適している |

> IFC のダウンロード URL は使用前に必ず確認してください。buildingSMART ギャラリーが一次ソースです。サードパーティのミラーは改変済みのファイルを配信している場合があります。

---

## パイプライン手順

### ステップ 1 — IFC から設計点群を生成

[IfcOpenShell](https://ifcopenshell.org/) を使って IFC の表面ジオメトリをサンプリングし、参照点群（「正解」となる設計データ）を生成します。各 `IfcProduct` 要素を三角形分割し、頂点座標を `(N, 3)` の配列として収集します。

### ステップ 2 — 損傷スキャンのシミュレーション

[Open3D](https://www.open3d.org/) を使って設計点群のコピーに意図的な変形を加え、既知の欠陥を持つ「実測データ」を作成します。デモの例では、特定領域を 15 mm 押し込んで表面の凹みを再現し、結果を PLY ファイルとして保存します。

### ステップ 3 — 偏差計算（M2）

`eds inspect scan` CLI コマンドを実行し、IFC 設計ファイルとシミュレーションスキャンの PLY ファイルを指定します。CLI は `src/ifc.rs` で設計参照点群を読み込み、`src/deviation.rs` で点ごとの最近傍偏差を計算して、`compliant_pct`・`max_deviation_mm`・`mean_deviation_mm` を含む JSON レポートを出力します。

このステップでは `src/ifc.rs` と `src/deviation.rs`（M2）を使います。

### ステップ 4 — 3D → 2D 投影（trilink-core）

`trilink-core::project_to_depth_map` がスキャン点群を深度マップ画像に変換し、AI 推論の入力とします。`config.toml` に設定されたカメラ内部パラメータを使って CLI が自動的に処理するため、手動操作は不要です。

このステップでは `trilink-core::project_to_depth_map`（基盤 #31）を使います。

### ステップ 5 — AI による欠陥検出

HTTP 推論パス（`inference.mode = "http"`）経由で深度マップに対して検出モデルを実行します。デモでは YOLOv8 を外部推論サーバーとして使用できます。CLI は深度マップ画像を設定済みの HTTP エンドポイントに送信し、バウンディングボックスの検出結果を受け取ります（M4）。

### ステップ 6 — 2D → 3D 逆投影

検出された 2D バウンディングボックスは `trilink-core::unproject` によってワールド座標に逆投影され、3D モデル上に重ねて表示されるとともに偏差レポートに含まれます（M4）。

---

## デモにおける偏差エンジンの位置づけ

偏差エンジン（M2）はデモの定量的な核心です。「異常があるか？」だけでなく、**「IFC 設計に対して実際の構造物が何ミリずれているか？」** に答えます。汎用的な欠陥検出器との差別化ポイントであるため、ステップ 3 を必ずデモで明示してください。

---

## テクニカルスタックまとめ

| コンポーネント | 言語・ライブラリ | ロードマップのマイルストーン |
|---|---|---|
| IFC 表面サンプリング | Python / IfcOpenShell | デモ準備（M2 以前） |
| 損傷シミュレーション | Python / Open3D | デモ専用 |
| IFC 偏差エンジン | Rust CLI / `src/ifc.rs`、`src/deviation.rs` | M2 |
| 3D ↔ 2D 投影 | Rust / trilink-core | 基盤 #31〜#32 |
| AI 欠陥検出 | 外部 HTTP サーバー（例: YOLOv8） | M4 `inference.mode = "http"` |
| レポート・ヒートマップ | Rust CLI / `src/report.rs`、`src/heatmap.rs` | M2〜M3 |
