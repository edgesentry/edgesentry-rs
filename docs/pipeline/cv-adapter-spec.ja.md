# CV アダプター仕様

edgesentry-rs はエンティティ位置から処理を開始します。カメラフレームをエンティティ位置に
変換するコンポーネントを **CV adapter** と呼びます。このドキュメントでは、
CV adapter が満たすべき契約と、暫定 OSS ベースアダプター（specula）について説明します。

---

## 推奨パス：CV ベンダーとの連携

CV adapter は edgesentry のコアコンピテンシーではありません。
本番アーキテクチャの推奨は、産業環境（港湾ターミナル・倉庫）向けに認定済みの
検出モデルを持つ専門 CV ベンダーとの統合です。

ベンダー統合により以下が軽減されます：
- モデル精度リスク（検出品質の責任はベンダーが負う）
- 規制上のリスク（産業安全規格での低信頼度検出問題）
- エンジニアリング工数（学習データ収集・アノテーションパイプラインが不要）

ベンダーの adapter は以下の出力契約を実装するだけでよく、
edgesentry-rs の物理エンジンと監査チェーンはベンダー非依存です。

---

## 出力契約：EntityFrame JSONL

すべての CV adapter は `eds.entity-frame` JSONL を出力する必要があります：

```json
{"eds_schema": "eds.entity-frame", "version": "0.2.0"}
{
  "timestamp_ms": 6000,
  "entities": [
    {
      "id": "FL-01",
      "class": "Forklift",
      "x": 6.0,
      "y": 0.0,
      "vx": 3.0,
      "vy": 0.0,
      "confidence": 0.91
    },
    {
      "id": "W-03",
      "class": "Person",
      "x": 12.0,
      "y": 0.0,
      "vx": 0.0,
      "vy": 0.0,
      "confidence": 0.87
    }
  ]
}
```

**要件：**

| フィールド | 要件 |
|---|---|
| `id` | 同一の物理エンティティに対してフレーム間で安定（トラッカー出力） |
| `class` | `Forklift`, `Person`, `Vessel`, `ReachStacker` のいずれか |
| `x`, `y` | サイト基準点からの実世界メートル値（ピクセルではない） |
| `vx`, `vy` | メートル毎秒（フレーム間差分またはトラッカー提供値） |
| `confidence` | 0.0–1.0、オプションだが強く推奨 |
| `timestamp_ms` | Unix ミリ秒、単調増加 |
| 位置精度 | 運用距離での誤差 < 0.5 m（TTC が意味を持つために必要） |

---

## 暫定ソリューション：specula

**リポジトリ：** `edgesentry/specula`
**ステータス：** 暫定対応——CV ベンダーパートナーが確保できていない場合の現場 PoC 用

specula は最小限の OSS ベース CV adapter です。本番システムではありません。
ベンダー統合が未整備の状態で現場 PoC を実施せざるを得ない場合のブロッカー解消を目的とします。

### スタック

| コンポーネント | 選択 | 理由 |
|---|---|---|
| 物体検出 | YOLO v11 (Ultralytics) | Apache 2.0、ターミナル・倉庫向け事前学習済み重み |
| 多物体追跡 | ByteTrack (supervision 経由) | 遮蔽をまたいだ安定した ID 維持 |
| 座標変換 | OpenCV ホモグラフィ | 4点以上の地上真値からカメラごとのキャリブレーション |
| 出力 | UDP → `edgesentry-ingest` または JSONL ファイル | edgesentry-rs ingest インターフェースに準拠 |
| 言語 | Python 3.11+ | 最速の反復開発。本番 Rust スタックには展開しない |

### アダプター構成

```
specula/
  adapters/
    mock_replay/   # CSV フィクスチャ → EntityFrame UDP（デモ / CI 用）
    yolo_v8/       # ライブカメラまたは録画映像 → EntityFrame
  calibration/
    homography.py  # ピクセル → メートル変換
    site_config.toml
  specula/
    entity_stream.py   # EntityFrame JSONL / UDP 出力
    gap_detector.py    # エンティティ消失時に EntityGap を出力
  README.md
```

### 制限事項（PoC 時に必ず開示）

- 検出精度は産業認定規格に対して未検証
- キャリブレーションは手動（4点ホモグラフィ）。誤差は TTC 計算に伝播
- マルチカメラフュージョンなし。各カメラが独立した adapter インスタンス
- 低照度・強グレア環境では IR カメラまたは別途照明設備が必要
- システム信頼性の証拠としては不適——機能デモとしてのみ使用可

### specula と本番の差異

| 要件 | specula | 本番（ベンダー） |
|---|---|---|
| 検出精度 | 約 85〜90 %（YOLO 事前学習済み） | ベンダー認定値 |
| マルチカメラフュージョン | 手動・カメラ単位 | ベンダー提供 |
| 信頼度キャリブレーション | 生の softmax（未校正） | Platt スケーリング等 |
| エッジデバイス展開 | Python、GPU 推奨 | ベンダー SDK、CPU 動作可能な場合も |
| サポート・責任 | なし | ベンダー SLA |

---

## 統合テスト

`mock_replay` adapter は任意の edgesentry-rs CSV フィクスチャを
UDP EntityFrame ストリームとして再生します。
ライブカメラなしでパイプライン全体（specula → edgesentry → 封印 → R2）を
エンドツーエンドで検証できます。

```bash
# mock replay を起動
python specula/adapters/mock_replay/replay.py \
  --fixture ../../clarus/fixtures/forklift_approach.csv \
  --port 9000 --fps 2

# edgesentry が UDP で受信
eds ingest stream --source udp://localhost:9000 --profile profiles/demo --out /tmp/frames.jsonl
```
