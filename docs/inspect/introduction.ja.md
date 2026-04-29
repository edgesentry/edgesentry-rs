# EdgeSentry-Inspect

インフラ点検向けリアルタイム・デジタルツイン監査プラットフォーム。

- **リポジトリ:** [github.com/edgesentry/edgesentry-rs](https://github.com/edgesentry/edgesentry-rs)
- **ドキュメント:** [edgesentry.github.io/edgesentry-rs/inspect/introduction/](https://edgesentry.github.io/edgesentry-rs/inspect/introduction/)

## 仕組み

EdgeSentry-Inspect は、3D 点群データと BIM 設計データを現場エッジで照合し、施工誤差や構造変化をリアルタイムで検出します。クラウドへの往復なしに、現場で完結します。

```
3D センサ（LiDAR/ToF）
    │  点群データ
    ▼
trilink-core::project          ← 3D → 2D 深度マップ / 高さマップ
    │  深度マップ（画像）
    ▼
ビジョン AI 推論               ← ローカル GPU で異常検出
    │  バウンディングボックス + クラス
    ▼
trilink-core::unproject        ← 2D 検出 → 3D ワールド座標
    │  ワールド座標系の異常位置
    ▼
Scan-vs-BIM エンジン           ← IFC 設計データと照合
    │  偏差ヒートマップ + レポート
    ▼
現場表示（タブレット / AR）    ← 点検員が現場でズレを確認
    │
    ▼  （生点群ではなくレポートのみアップロード）
クラウド監査ストア             ← 改ざん防止証跡 + デジタルツイン更新
```

## エッジファーストの理由

現場 PC がスキャンから偏差レポートまでの全処理を担います。アップロードされるのはレポート（JSON + PNG ヒートマップ）のみです。これにより、安定したクラウド接続がない環境でも 30 分以内の現場点検が実現できます。

## 依存ライブラリ

- [`trilink-core`](https://github.com/edgesentry/trilink-core) — 点群投影と空間融合（Rust）
- [`edgesentry-rs`](https://github.com/edgesentry/edgesentry-rs) — 数学的に検証可能な監査レコード（高保証が必要な用途向けオプション）

## ライセンス

MIT OR Apache-2.0
