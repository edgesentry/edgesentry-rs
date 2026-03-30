# CLI リファレンス

`eds inspect` はフィールド PC パイプラインを実行します。IFC 参照 + PLY スキャン → 偏差計算 → オプションの AI 推論 → ヒートマップ + レポート + オプションの 3D メッシュオーバーレイ。

---

## インストール

### エンドユーザー向け — Homebrew（macOS / Linux）

```bash
brew install edgesentry/tap/eds
```

`uv` は Homebrew の依存関係として自動インストールされます。Python を別途インストールする必要はありません。

### エンドユーザー向け — ビルド済みバイナリ

最新リリースを [GitHub Releases ページ](https://github.com/edgesentry/edgesentry-rs/releases) からダウンロードしてください。

| プラットフォーム | ファイル |
|----------------|---------|
| Linux (x86-64) | `eds-{version}-x86_64-unknown-linux-gnu.tar.gz` |
| macOS (Apple Silicon) | `eds-{version}-aarch64-apple-darwin.tar.gz` |
| Windows (x86-64) | `eds-{version}-x86_64-pc-windows-msvc.zip` |

```bash
# Linux / macOS
tar -xzf eds-{version}-{target}.tar.gz
sudo mv eds /usr/local/bin/
eds --help
```

### 開発者向け — ソースからインストール

[Rust](https://rustup.rs)（stable ツールチェーン）が必要です。

```bash
cargo install --git https://github.com/edgesentry/edgesentry-rs --locked --bin eds
```

---

## `eds inspect generate-fixtures`

オフラインのデモデータを生成します（外部依存なし）：

```bash
eds inspect generate-fixtures --dir ./demo-data
```

| フラグ | 説明 |
|--------|------|
| `-d`, `--dir` | 出力ディレクトリ（存在しない場合は作成、デフォルト: `demo-data`） |

`<dir>` に以下の 3 ファイルを生成します：

| ファイル | 内容 |
|----------|------|
| `wall_slab.ifc` | 651 個の `IFCCARTESIANPOINT` — 3 m × 2 m フラット壁の参照モデル |
| `wall_slab_scan.ply` | 同じグリッドで中心部に 20 mm の外向き膨らみ（49 点が非適合） |
| `config.toml` | `eds inspect scan` 用に事前設定済み |

生成後、パイプラインを実行するには：

```bash
cd demo-data && eds inspect scan --config config.toml
```

---

## `eds inspect download-samples`

buildingSMART サンプル IFC ファイルをダウンロードします：

```bash
eds inspect download-samples --dir ./ifc-samples
```

| フラグ | 説明 |
|--------|------|
| `-d`, `--dir` | 出力ディレクトリ（存在しない場合は作成、デフォルト: `ifc-samples`） |

buildingSMART Sample-Test-Files リポジトリから `Building-Architecture.ifc`（約 220 KB、IFC 4、PCERT サンプルシーン）をダウンロードします。ファイルがすでに存在する場合はスキップされます。

---

## `eds inspect extract-mesh`

IFC ファイルから三角形メッシュジオメトリを抽出します：

```bash
eds inspect extract-mesh \
    --ifc ./ifc-samples/Building-Architecture.ifc \
    --out ./ifc-samples/reference.json
```

| フラグ | 説明 |
|--------|------|
| `--ifc` | 入力 IFC ファイル |
| `--out` | 出力 `reference.json` のパス |

**前提条件：** PATH 上に `uv`（`brew install uv`）。Python のインストールは不要 — `uv` が初回実行時に Python と `ifcopenshell` を自動で管理します（以降の実行はキャッシュを使用）。

IfcOpenShell 抽出スクリプトは `eds` バイナリに埋め込まれています。初回呼び出し時に `uv` が Python をダウンロードし、`ifcopenshell` をローカルキャッシュ（`~/.cache/uv/`）にインストールします。

### 出力形式（`reference.json`）

```json
{
  "vertices": [[x, y, z], ...],
  "faces":    [[i, j, k], ...]
}
```

座標はメートル単位（ワールド座標系）です。`config.toml` の `mesh_path` にこのファイルのパスを指定すると、スキャン出力にメッシュが含まれます。

---

## `eds inspect scan`

TOML 設定ファイルからフルスキャンパイプラインを実行：

```bash
eds inspect scan --config config.toml
```

| フラグ | 説明 |
|--------|------|
| `-c`, `--config` | TOML 設定ファイルのパス（デフォルト: `config.toml`） |

### 設定ファイルの形式

```toml
ifc_path  = "path/to/design.ifc"
scan_path = "path/to/scan.ply"

# オプション: ビューアーで IFC 参照を青いワイヤーフレームとして表示する場合に設定
# mesh_path = "path/to/reference.json"

[camera]
fx = 525.0
fy = 525.0
cx = 319.5
cy = 239.5
width  = 640
height = 480

[inference]
mode = "off"          # "off"、"mock"、"onnx"、または "http"
# model_path = "model.onnx"                 # mode = "onnx" の場合に必須
# endpoint = "http://localhost:8000/infer"   # mode = "http" の場合に必須

[output]
dir          = "out"
threshold_mm = 10.0
```

注釈付きのサンプルは [`config.example.toml`](../../../../crates/edgesentry-inspect/config.example.toml) を参照してください。

---

## 出力ファイル

| ファイル | 説明 |
|----------|------|
| `out/report.json` | `compliant_pct`・`max_deviation_mm`・`mean_deviation_mm`、オプションで `detections` |
| `out/heatmap.png` | 点ごとの偏差ヒートマップ（緑 = 適合、赤 = 閾値超過） |
| `out/points.json` | ビューアー用の点ごとの 3D 位置と偏差値 |
| `out/reference.json` | `mesh_path` が設定されている場合にコピー — ビューアーのワイヤーフレーム用 IFC メッシュ |

---

## 推論モード

**`mode = "off"`** — 偏差計算とヒートマップのみ。AI 呼び出しなし。

**`mode = "mock"`** — 合成ウォールフィクスチャ用の組み込み検出結果を返します。外部サーバー不要。本番モデルなしで AI 検出パイプライン全体（深度マップ → ビューアーのオレンジ球）をデモするために使用します。

**`mode = "onnx"`** — ローカルの `.onnx` モデルファイルを読み込み、[`tract`](https://github.com/sonos/tract)（純 Rust、C 依存なし）でプロセス内推論を実行します。`model_path` にモデルファイルを指定してください。エッジ / フィールド PC デプロイに最適 — ネットワーク不要。合成フィクスチャ用プロトタイプモデルの生成：

```bash
uv run scripts/generate_prototype_model.py --out model.onnx
```

**`mode = "http"`** — 深度マップが PNG としてサードパーティの推論サーバー（例：YOLOv8）の `endpoint` に POST されます。サーバーはバウンディングボックスの JSON 配列を返す必要があります：

```json
[{"u0": 10, "v0": 20, "u1": 60, "v1": 80}, ...]
```

検出された領域は `trilink-core::unproject` によってワールド座標に逆投影され、`report.json` に含まれます。

---

## オプションフィーチャーでのビルド

`eds inspect` コマンドには追加のフィーチャーフラグは不要です。トランスポートフィーチャー（`transport-http`、`transport-tls` など）は `eds audit serve*` コマンドにのみ適用されます。

```bash
# デフォルトビルド — すべての inspect コマンドはそのまま動作します
cargo build -p eds

# 監査 HTTP トランスポートも含める場合
cargo build -p eds --features transport-http
```
