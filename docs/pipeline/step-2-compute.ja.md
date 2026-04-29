# ステップ 2 - 計算

生のエンティティ計測値に物理・ジオメトリ演算を適用します。

```
eds compute run --input <FILE> --out <FILE>
```

| フラグ | 説明 |
|------|-------------|
| `--input` | 入力 EntityFrame JSONL ファイル（`eds ingest replay` または `eds ingest stream` から） |
| `--out` | 出力 Measurement JSONL ファイル |

## 計算内容

すべてのフレーム内のすべてのエンティティペアについて：

| 関数 | 出力 | 計算式 |
|---|---|---|
| `euclidean_distance` | メートル | `sqrt((x2-x1)^2 + (y2-y1)^2)` |
| `relative_velocity` | m/s | エンティティ間の線に沿った速度の成分 |
| `time_to_collision` | 秒 | `distance / closing_speed`（エンティティが接近している場合のみ） |
| `braking_distance` | メートル | エンティティクラス固有のルックアップテーブル |
| `zone_membership` | bool | プロファイルのゾーン定義を使用した多角形内の点判定 |

TTC は `closing_speed > 0`（エンティティが接近中）の場合のみ計算されます。正の TTC は現在の軌跡で衝突が発生することを意味し、負または無限の TTC はエンティティが離れていることを意味します。

## パイプライン使用に関する注記

`eds evaluate run` は元の `EntityFrame` JSONL を読み込み、内部で物理演算を適用します。
`eds compute run` は検査とデバッグのために提供されており ── ルール評価の前に生の計測値を確認できます。本番パイプラインでは、両方のコマンドを実行することも、中間計測値が不要な場合は `eds evaluate run` のみを実行することもできます。
