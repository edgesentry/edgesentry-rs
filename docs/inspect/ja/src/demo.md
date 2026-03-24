# デモパイプライン

このページでは、オープンデータセットと Inspect パイプラインを使って、自己完結した概念実証（PoC）デモを構築する手順を説明します。本番データが用意できる前の技術評価・現場デモでの利用を想定しています。

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

[IfcOpenShell](https://ifcopenshell.org/) を使って IFC の表面ジオメトリをサンプリングし、参照点群（「正解」となる設計データ）を生成します。

```python
import ifcopenshell
import ifcopenshell.geom
import numpy as np

settings = ifcopenshell.geom.settings()
model = ifcopenshell.open("design.ifc")

points = []
for product in model.by_type("IfcProduct"):
    try:
        shape = ifcopenshell.geom.create_shape(settings, product)
        verts = np.array(shape.geometry.verts).reshape(-1, 3)
        points.append(verts)
    except Exception:
        pass

design_cloud = np.vstack(points)  # shape: (N, 3)
```

### ステップ 2 — 損傷スキャンのシミュレーション

[Open3D](https://www.open3d.org/) を使って設計点群のコピーに意図的な変形を加え、既知の欠陥を持つ「実測データ」を作成します。

```python
import open3d as o3d
import numpy as np

pcd = o3d.geometry.PointCloud()
pcd.points = o3d.utility.Vector3dVector(design_cloud)

# 特定領域を 15 mm 押し込む
points = np.asarray(pcd.points)
mask = (points[:, 0] > 1.0) & (points[:, 0] < 1.5)
points[mask, 2] -= 0.015  # 15 mm の凹み

pcd.points = o3d.utility.Vector3dVector(points)
o3d.io.write_point_cloud("scan.ply", pcd)
```

### ステップ 3 — 偏差計算（M2）

IFC 偏差エンジンがシミュレーションスキャンと設計点群を比較し、点ごとのミリ単位偏差を含む JSON レポートを生成します。

```
edgesentry-inspect scan \
  --ifc design.ifc \
  --scan scan.ply \
  --threshold-mm 5.0 \
  --out report.json
```

このステップでは `src/ifc.rs` と `src/deviation.rs`（M2）を使います。

### ステップ 4 — 3D → 2D 投影（trilink-core）

`trilink-core::project_to_depth_map` がスキャン点群を深度マップ画像に変換し、AI 推論の入力とします。

```
# パイプライン内部で自動的に処理されます — 手動操作は不要です。
# CLI は設定されたカメラ内部パラメータを使って project_to_depth_map を呼び出します。
```

このステップでは `trilink-core::project_to_depth_map`（基盤 #31）を使います。

### ステップ 5 — AI による欠陥検出

深度マップに対して検出モデルを実行します。デモでは YOLOv8 を HTTP 推論パス（`inference.mode = "http"`）経由で使用します。

```python
from ultralytics import YOLO
import requests

model = YOLO("yolov8n.pt")  # 欠陥検出用にファインチューニングされたモデルでも可
results = model("depth_map.png")
# 検出結果を Inspect の HTTP 推論エンドポイントに転送
```

CLI は `inference.mode = "http"` で HTTP サーバーからの検出結果を受け取るよう設計されており、Python で動く YOLOv8 は Rust コードを変更せずにそのまま接続できます（M4）。

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
| IFC 偏差エンジン | Rust / `src/ifc.rs`、`src/deviation.rs` | M2 |
| 3D ↔ 2D 投影 | Rust / trilink-core | 基盤 #31〜#32 |
| AI 欠陥検出 | Python / YOLOv8（HTTP） | M4 `inference.mode = "http"` |
| レポート・ヒートマップ | Rust / `src/report.rs`、`src/heatmap.rs` | M2〜M3 |
