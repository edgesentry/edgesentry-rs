# デモパイプライン

完全オフラインのデモと、実際の IFC モデルと 3D メッシュオーバーレイを使ったデモの 2 つのパスがあります。

---

## パス 1 — 完全オフライン（ダウンロード不要・Python 不要）

外部依存なしで、AI 欠陥検出を含むパイプライン全体をエンドツーエンドで確認する最速の方法です。

```bash
# 1. 合成ウォールフィクスチャを生成（651 点の 3 m × 2 m 壁 + 中心部に 20 mm の欠陥）
eds inspect generate-fixtures --dir ./demo

# 2. 偏差計算 + AI 検出（モックモード、外部サーバー不要）を実行
cd demo
eds inspect scan --config config.toml
```

生成された `config.toml` は `inference.mode = "mock"` を使用しており、中心部の欠陥に対応する組み込みの検出結果を返します。外部 AI サーバーは不要です。

期待される出力：
```
compliant_pct    : 92.5%
max_deviation_mm : 20.000 mm
mean_deviation_mm: 2.680 mm
AI detections    :        1  ⚠  see orange spheres in viewer
```

`./demo/output/` に以下のファイルが生成されます：

| ファイル | 内容 |
|----------|------|
| `report.json` | 偏差統計 + 検出座標 |
| `heatmap.png` | 2D カラーマップ — 緑（適合）→ 赤（欠陥） |
| `points.json` | ビューアー用の点ごとの 3D 位置・偏差値・検出球 |

`./demo/output/` を Inspect App ビューアーで開くと、カラー点群と欠陥中心のオレンジ色の検出球を確認できます。

---

## パス 2 — 実際の IFC ファイルと 3D メッシュオーバーレイ

実際の buildingSMART サンプル IFC を使い、IFC 参照ジオメトリを青いワイヤーフレームとしてビューアーのスキャン点群に重ねて表示します。

### 前提条件

- `uv` — `brew install uv`（Python と `ifcopenshell` を自動管理）

### ステップ 1 — サンプル IFC をダウンロード

```bash
eds inspect download-samples --dir ./ifc-samples
```

buildingSMART から `Building-Architecture.ifc`（約 220 KB、IFC 4 PCERT サンプル）をダウンロードします。すでに存在する場合はスキップされます。

### ステップ 2 — IFC メッシュを抽出

```bash
eds inspect extract-mesh \
    --ifc ./ifc-samples/Building-Architecture.ifc \
    --out ./ifc-samples/reference.json
```

初回実行時に `uv` が Python をダウンロードし、`ifcopenshell` を自動インストールします（`~/.cache/uv/` にキャッシュ）。以降の実行は高速です。

出力：`reference.json` — ワールド座標の頂点と三角形面。

### ステップ 3 — デモ用スキャンを生成

```bash
eds inspect generate-fixtures --dir ./demo
```

PLY スキャンと設定済みの `config.toml` が生成されます。実際のスキャンを使う場合は `wall_slab_scan.ply` を差し替えてください。

### ステップ 4 — config.toml に `mesh_path` を追加

```bash
echo 'mesh_path = "../ifc-samples/reference.json"' >> ./demo/config.toml
```

### ステップ 5 — パイプラインを実行

```bash
cd demo
eds inspect scan --config config.toml
```

`reference.json` が `./demo/output/reference.json` として `points.json` と並んでコピーされます。

### ステップ 6 — Inspect App で表示

`./demo/output/` を Inspect App ビューアーで開きます。IFC 参照メッシュが半透明の青いワイヤーフレームとしてカラー点群の上に重なって表示されます。サイドバーの **Reference mesh** トグルで表示・非表示を切り替えられます。

---

## テクニカルスタックまとめ

| コンポーネント | 実装 | コマンド |
|---------------|------|---------|
| 合成フィクスチャ | Rust（組み込み） | `eds inspect generate-fixtures` |
| IFC サンプルダウンロード | Rust + ureq | `eds inspect download-samples` |
| IFC メッシュ抽出 | Python / IfcOpenShell（`uv run` 経由） | `eds inspect extract-mesh` |
| 偏差エンジン | Rust / `deviation.rs` | `eds inspect scan` |
| 3D ↔ 2D 投影 | Rust / trilink-core | `scan` 内で自動実行 |
| AI 欠陥検出（デモ） | Rust（組み込みモック） | `inference.mode = "mock"` |
| AI 欠陥検出（本番） | サードパーティ HTTP サーバー | `inference.mode = "http"` |
| ヒートマップ・レポート | Rust / `heatmap.rs`、`report.rs` | `scan` 内で自動実行 |
| 3D ビューアー | Three.js（Inspect App） | 出力フォルダをアプリで開く |
