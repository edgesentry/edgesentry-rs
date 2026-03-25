# CLI リファレンス

`eds inspect` は M4 フィールド PC パイプラインを実行します。IFC 参照 + PLY スキャン → 偏差計算 → オプションの AI 推論 → ヒートマップ + レポート。

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
