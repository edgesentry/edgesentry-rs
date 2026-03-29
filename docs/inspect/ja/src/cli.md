# CLI リファレンス

`eds inspect` は M4 フィールド PC パイプラインを実行します。IFC 参照 + PLY スキャン → 偏差計算 → オプションの AI 推論 → ヒートマップ + レポート。

---

## インストール

### エンドユーザー向け — ビルド済みバイナリ

最新リリースを [GitHub Releases ページ](https://github.com/edgesentry/edgesentry-rs/releases) からダウンロードしてください。

| プラットフォーム | ファイル |
|----------------|---------|
| Linux (x86-64) | `eds-{version}-x86_64-unknown-linux-gnu.tar.gz` |
| macOS (Apple Silicon) | `eds-{version}-aarch64-apple-darwin.tar.gz` |
| Windows (x86-64) | `eds-{version}-x86_64-pc-windows-msvc.zip` |

展開して `eds` バイナリを `PATH` に追加してください：

```bash
# Linux / macOS
tar -xzf eds-{version}-{target}.tar.gz
sudo mv eds /usr/local/bin/
eds --help
```

```powershell
# Windows（PowerShell）
Expand-Archive eds-{version}-x86_64-pc-windows-msvc.zip
# eds.exe を PATH が通ったディレクトリに移動してください
eds --help
```

### 開発者向け — ソースからインストール

[Rust](https://rustup.rs)（stable ツールチェーン）が必要です。

```bash
cargo install --git https://github.com/edgesentry/edgesentry-rs --locked --bin eds
```

---

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

生成したデータに対してパイプラインを実行するには：

```bash
cd demo-data && eds inspect scan --config config.toml
```

---

## `eds inspect scan`

TOML 設定ファイルからフルスキャンパイプラインを実行：

```bash
eds inspect scan --config config.toml
```

| フラグ | 説明 |
|--------|------|
| `-c`, `--config` | TOML 設定ファイルのパス（必須） |

### 設定ファイルの形式

```toml
ifc_path  = "path/to/design.ifc"
scan_path = "path/to/scan.ply"

[camera]
fx = 525.0
fy = 525.0
cx = 319.5
cy = 239.5
width  = 640
height = 480

[inference]
mode = "off"          # "off" または "http"
# endpoint = "http://localhost:8000/infer"   # mode = "http" の場合に必須

[output]
dir = "out"
```

注釈付きのサンプルは [`config.example.toml`](../../../../crates/edgesentry-inspect/config.example.toml) を参照してください。

---

## 出力ファイル

| ファイル | 説明 |
|----------|------|
| `out/report.json` | `compliant_pct`・`max_deviation_mm`・`mean_deviation_mm`、オプションで `detections` |
| `out/heatmap.png` | 点ごとの偏差ヒートマップ（青 = 適合、赤 = 閾値超過） |

---

## 推論モード

**`mode = "off"`** — 偏差計算とヒートマップのみ。AI 呼び出しなし。

**`mode = "http"`** — 深度マップが PNG として `endpoint` に POST されます。サーバーはバウンディングボックスの JSON 配列を返す必要があります：

```json
[{"x": 10, "y": 20, "w": 50, "h": 60}, ...]
```

検出された領域は `trilink-core::unproject` によってワールド座標に逆投影され、`report.json` に含まれます。

---

## オプションフィーチャーでのビルド

`eds inspect scan` コマンドには追加のフィーチャーフラグは不要です。トランスポートフィーチャー（`transport-http`、`transport-tls` など）は `eds audit serve*` コマンドにのみ適用されます。

```bash
# デフォルトビルド — inspect scan はそのまま動作します
cargo build -p eds

# 監査 HTTP トランスポートも含める場合
cargo build -p eds --features transport-http
```
