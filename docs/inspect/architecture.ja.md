# EdgeSentry-Inspect — アーキテクチャ

## エッジ・クラウド分担

```
┌──────────────────────────────────────────────────────────┐
│  現場 PC（エッジ）                                       │
│                                                          │
│  3D センサ（LiDAR / ToF）                               │
│      │  点群データ（PointCloud）                        │
│      ▼                                                   │
│  trilink-core::project_to_depth_map                      │
│  trilink-core::project_to_height_map                     │
│      │  DepthMap  HeightMap                              │
│      ▼                                                   │
│  AI 推論（組み込みモデルまたは HTTP エンドポイント）    │
│      │  Vec<Detection>（BBox2D + クラス + 信頼度）      │
│      ▼                                                   │
│  trilink-core::unproject                                 │
│      │  ワールド座標系の Point3D（検出ごと）            │
│      ▼                                                   │
│  edgesentry-inspect::ifc      — IFC ジオメトリ          │
│  edgesentry-inspect::deviation — 偏差計算（mm）          │
│  edgesentry-inspect::heatmap   — ヒートマップ PNG        │
│  edgesentry-inspect::report    — JSON レポート           │
│      │                                                   │
│      ├── タブレット / AR ヘッドセットに即時表示         │
└──────┬───────────────────────────────────────────────────┘
       │  レポート JSON + ヒートマップ PNG（生点群ではない）
       ▼
┌──────────────────────────────────────────────────────────┐
│  クラウド（監査ストア / デジタルツイン）                │
│                                                          │
│  edgesentry-inspect::sync                                │
│      │  S3 互換アップロード（Object Lock WORM）         │
│      │  構造変化フラグ → メッセージキュー               │
│      ▼                                                   │
│  監査レポートストア — 改ざん防止の証跡                  │
│  デジタルツイン更新 — 実測 IFC デルタ                   │
│  中央ダッシュボード — フリート全体の偏差傾向            │
└──────────────────────────────────────────────────────────┘
```

### 現場 PC で処理するもの

| ステップ | エッジで処理する理由 |
|---|---|
| 3D → 2D 投影 | 点群はギガバイト単位。判定前のアップロードは不要 |
| AI 推論 | サブ秒のレイテンシ要件。ローカル GPU で処理。オフライン動作 |
| 2D → 3D 逆投影 | 現場での AR フィードバックに必要 |
| IFC 読み込み + 偏差計算 | 点検員が現場を離れる前に偏差を確認しなければならない |
| ヒートマップ + レポート生成 | レポートがアップロード成果物。現場で準備完了している必要がある |

### クラウドに送るもの

| データ | クラウドで保管する理由 |
|---|---|
| 偏差レポート（JSON） | 改ざん防止の監査証跡。規制アーカイブ |
| ヒートマップ（PNG） | レポートに添付する人間可読な証拠 |
| 構造変化フラグ | 中央監視への即時アラート（UC-2） |
| 実測 IFC デルタ | デジタルツイン資産モデルへの永続的な更新 |

---

## コンポーネント設計

### edgesentry-inspect::ifc

- 入力: IFC ファイルパス（`.ifc`）
- 出力: `Vec<Point3D>` — 壁・スラブ・柱のジオメトリからサンプリングした設計参照点群
- 実装: `pyo3` 経由の Python FFI（`ifcopenshell`）またはネイティブ Rust IFC リーダー
- 参照点群は点検セッション 1 回につき 1 度だけ読み込み、メモリにキャッシュ

### edgesentry-inspect::deviation

- 入力: スキャン `Vec<Point3D>`（`trilink-core::unproject` 経由）+ 設計 `Vec<Point3D>`（`ifc` 経由）
- 出力: スキャン点ごとの偏差値 `f32`（メートル単位）
- アルゴリズム: k-d ツリー最近傍探索（`kiddo` クレート）。O(n log m)
- しきい値: 設定可能（建設デフォルト 10 mm、海事船体 5 mm）

### edgesentry-inspect::heatmap

- 入力: スキャン点群 + 点ごとの偏差値
- 出力: PNG 画像 — 偏差を色でマッピング（緑 ≤ しきい値、黄 2 倍、赤 4 倍以上）
- `trilink-core::project_to_depth_map` を再利用して各色点を 2D に配置

### edgesentry-inspect::report

JSON スキーマ:

```json
{
  "compliant_pct": 94.2,
  "max_deviation_mm": 23.1,
  "mean_deviation_mm": 3.8,
  "point_count": 142850,
  "threshold_mm": 10.0
}
```

AI 検出位置はレポートと並んで `points.json` に書き込まれます:

```json
{
  "scan_points": [
    { "x": 12.3, "y": 4.1, "z": 2.05, "deviation_mm": 23.1 }
  ],
  "detections": [
    { "x": 12.3, "y": 4.1, "z": 2.05 }
  ]
}
```

### edgesentry-inspect::sync

- 偏差レポート JSON とヒートマップ PNG を S3 互換監査ストアにアップロード（Object Lock WORM）
- 設定しきい値の 2 倍を超える異常が検出された場合、構造変化フラグをメッセージキュー（SQS または MQTT）に発報
- `edgesentry-rs` の S3 互換インタフェースを再利用

---

## AI 推論モード

EdgeSentry-Inspect は `config.toml` の `inference.mode` で選択できる 2 つの推論バックエンドをサポートします。どちらも同じ `Vec<Detection>` を出力し、以降のパイプラインで使用されます。

### 組み込みモデル（`inference.mode = "builtin"`）

EdgeSentry-Inspect にバンドルされた軽量の欠陥検出モデルです。ONNX Runtime を使ってプロセス内で実行されるため、外部サーバーもネットワークアクセスも不要です。

- 入力: `trilink-core` が生成する `DepthMap` + `HeightMap` 画像
- 出力: `Vec<Detection>` — バウンディングボックス・クラスラベル・信頼度スコア
- 初期クラス: `surface_void`（表面空洞）・`misalignment`（位置ずれ）・`rebar_exposure`（鉄筋露出）
- ハードウェア: 標準的な現場 PC CPU で動作。基本的な用途では専用 GPU 不要

外部サーバーが利用できないオフライン専用デプロイや、すぐに使い始めたい場合に `builtin` を使用してください。

### 外部 HTTP エンドポイント（`inference.mode = "http"`）

推論クライアントが `inference.base_url` に深度マップと高さマップを POST し、検出リストを受け取ります。エンドポイントは以下のいずれかです。

- 現場 PC またはロボット上でローカルに動作するベンダーのモデルサーバー（同一ホスト。インターネット不要）
- 専門的なクラウド推論 API（シナリオ 1 / 接続ありのデプロイのみ）

このモードがベンダー連携の統合ポイントです。ベンダーは自社モデルでサーバー側を実装し、EdgeSentry-Inspect は固定スキーマでそれを呼び出します。オペレーターは設定で `inference.base_url` を指定するだけでコード変更は不要です。

**インタフェース仕様:**

```
POST /detect
Content-Type: multipart/form-data
  depth_map: <PNG バイト列>
  height_map: <PNG バイト列>

200 OK
[{"x":120,"y":45,"w":30,"h":20,"class":"surface_void","confidence":0.87}, ...]
```

| モード | 使用場面 |
|---|---|
| `builtin` | ベンダーモデルなし。オフライン専用。すぐに使い始めたい場合 |
| `http` — ローカルベンダーサーバー | 同一デバイス上のパートナーモデル。インターネット不要 |
| `http` — クラウド API | シナリオ 1（接続あり）。ベンダーがモデルをリモートでホスト |

---

## オプション：数学的に検証可能な監査レコード

点検レポートに対して**第三者が独立して改ざんを検証できる**必要がある場合（規制提出書類、法的拘束力のある構造認証など）、偏差レポートを [`edgesentry-rs`](https://github.com/edgesentry/edgesentry-rs) を使って署名・ハッシュチェーンに組み込むことができます。

| 機能 | EdgeSentry-Inspect への適用 |
|---|---|
| Ed25519 ペイロード署名 | 現場 PC がハードウェアセキュア素子の鍵で各偏差レポートに署名。特定のセンサデバイスからの発行を証明 |
| BLAKE3 ハッシュチェーン | 各レポートが `prev_record_hash` を持ち、連鎖を形成。レポートの欠落や並び替えが検出可能 |
| シーケンス単調性 | レポートのシーケンス番号は厳密な単調増加。リプレイと削除を暗号的に検出可能 |
| `IngestService::ingest()` | クラウド側ゲートがアップロード時に署名とハッシュチェーンを再検証。改ざんや順序外レポートを拒否 |

このレイヤーは**オプション**です。標準的な建設点検では `edgesentry-inspect::sync` の S3 Object Lock WORM ストアで十分です。海事船体認証や法的効力を持つ構造サインオフなど高保証が求められる用途では、`edgesentry-rs` の `AuditRecord` でレポートをラップすることで、EdgeSentry-Inspect インフラとは独立して検証可能な暗号監査証跡が得られます。

---

## 計測精度の要因

UC-1（建設）の目標精度は 10 mm、UC-2（海事）は 5 mm です。現場での計測精度を決定する主要因と対策を以下に示します。

| 要因 | 影響 | 対策 |
|---|---|---|
| 3D センサの精度 | 主要な影響因子 | 必要な距離における目標精度に適合したセンサを使用 |
| SLAM 姿勢精度 | 偏差計算に伝播 | 定期的なループクロージャ。特徴のない空間ではフィデューシャルマーカー |
| IFC アライメント誤差 | 偏差マップ全体をシフトさせる | IFC-to-SLAM 登録に 3 点以上の既知制御点を使用。残差 2 mm 未満を確認。オペレーターによらず一貫した結果を得るには、点検前に IFC 既知座標にフィデューシャルマーカー（ArUco / AprilTag）を設置する。SLAM システムが自動検出し、登録から手動の判断を排除する。 |
| 投影ラウンドトリップ誤差 | `trilink-core` ラウンドトリップテスト（#34）で 1 mm 未満を検証 | 算術誤差は有意な影響因子ではない |
| k-d ツリー解像度 | 最近傍探索精度 | 設計点群を 2 mm ピッチ以下でサンプリング（検出しきい値より細かく） |

---

## 技術サマリー

| コンポーネント | 言語 | 主要クレート |
|---|---|---|
| `edgesentry-inspect`（偏差エンジン） | Rust | `trilink-core`、`kiddo`、`image`、`pyo3` |
| `edgesentry-inspect`（CLI） | Rust | `clap`、`tokio`、`reqwest`、`serde_json` |
| `edgesentry-inspect`（クラウド同期） | Rust | `edgesentry-rs` S3 互換インタフェース |
| IFC ジオメトリ | Python（`pyo3` 経由） | `ifcopenshell` |
| AI 推論 — 組み込み | Rust + ONNX Runtime | バンドル済み軽量欠陥検出モデル（`ort` クレート） |
| AI 推論 — 外部 | HTTP（`reqwest`） | ベンダーエンドポイント: POST 画像 → `Vec<BBox2D>`。ローカルまたはクラウド |
| クラウド監査ストア | AWS | S3 + Object Lock（WORM）、SQS |

---

## PoC 用オープンデータセット

| 分野 | データセット | 用途 |
|---|---|---|
| 建設 | BIMNet（公開 IFC モデル） | Scan-vs-BIM 用の設計参照ジオメトリ |
| 建設 | ETH3D / S3DIS 点群 | 偏差テスト用サンプルスキャン |
| 海事 | MBES 海底調査データ | 船体スキャン点群 |
| 汎用 | NYU Depth V2 | 投影処理の正確性検証用深度マップ |
