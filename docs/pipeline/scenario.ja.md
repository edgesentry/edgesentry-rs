# シナリオ - 合成データ生成

物理センサーなしでパイプラインをテストするために、合成エンティティ CSV フィクスチャを生成し、UDP 経由でストリームします。

## eds scenario generate

N フレームにわたる合成エンティティ位置を含む CSV ファイルを生成します。

```
eds scenario generate --out <FILE>
                      [--entities N] [--frames N] [--seed N]
                      [--scenario-type entity]
```

| フラグ | デフォルト | 説明 |
|------|---------|-------------|
| `--entities` | 2 | シミュレートするエンティティの数 |
| `--frames` | 10 | 時間フレーム数 |
| `--seed` | 0 | 再現可能な出力のための整数シード（LCG RNG ── 外部依存なし） |
| `--scenario-type` | `entity` | シナリオタイプ（現在は `entity` のみサポート） |
| `--out` | | 出力 CSV ファイルパス |

**出力形式** ── `eds ingest replay` の入力と同じヘッダ：

```
timestamp_ms,entity_id,entity_type,x,y,vx,vy
0,E-0,Forklift,12.3,7.6,-0.8,0.2
0,E-1,Person,4.1,9.3,0.0,0.0
100,E-0,Forklift,12.2,7.6,-0.8,0.2
...
```

エンティティタイプは交互になります：偶数インデックスのエンティティは `Forklift`、奇数インデックスは `Person` です。
各エンティティは `[0, 20]` メートル内のランダムな開始位置と固定速度を持ちます。
フレーム間隔は 100 ms（デフォルトで 10 fps）です。

**バンドルされたフィクスチャ**：`crates/edgesentry-scenario/fixtures/simple_crossing.csv` ── 10フレームにわたって正面から接近する2つの Forklift エンティティ。

## eds scenario simulate

シナリオ CSV を読み込み、実行中の `eds ingest stream` プロセスにエンティティフレームを UDP 経由でストリームします。

```
eds scenario simulate --source <FILE> --target <udp://HOST:PORT>
                      [--fps N]
```

| フラグ | デフォルト | 説明 |
|------|---------|-------------|
| `--source` | | 入力 CSV ファイル（`eds scenario generate` で生成または手書き） |
| `--target` | | UDP ターゲットアドレス（例：`udp://127.0.0.1:9000`） |
| `--fps` | 10 | 1秒あたりのフレーム数 ── フレーム送信間のスリープを制御 |

各フレームは JSON オブジェクトを含む単一の UDP データグラムとして送信されます：

```json
{"entities": [
  {"id": "E-0", "class": "Forklift", "x": 12.3, "y": 7.6, "vx": -0.8, "vy": 0.2, "timestamp_ms": 0},
  {"id": "E-1", "class": "Person",   "x": 4.1,  "y": 9.3, "vx": 0.0,  "vy": 0.0, "timestamp_ms": 0}
]}
```

これは `eds ingest stream` が期待する `UnityPacket` 形式と一致します。

## エンドツーエンドの例

```bash
# ターミナル 1 ── 取込ストリームリスナーを開始
eds ingest stream \
  --source udp://127.0.0.1:9000 \
  --profile crates/edgesentry-profile/fixtures/demo \
  --out /tmp/live.jsonl

# ターミナル 2 ── シナリオを生成してストリーム
eds scenario generate --frames 20 --entities 3 --out /tmp/scenario.csv
eds scenario simulate --source /tmp/scenario.csv --target udp://127.0.0.1:9000 --fps 10

# キャプチャされたフレームを評価
eds evaluate run \
  --input /tmp/live.jsonl \
  --profile crates/edgesentry-profile/fixtures/demo \
  --out /tmp/events.jsonl
```
