# ステップ 1 - 取込

現実のデータを取得し、パイプラインの残りの部分が理解できるスキーマに正規化します。
2つのサブコマンドが構造化された継続データ（センサーストリーム）をカバーし、1つが構造化ドキュメントデータ（海事航海記録）をカバーします。

## eds ingest replay

CSV ファイルからエンティティ位置を再生します。テスト、CI、およびオフラインデモに使用されます。

```
eds ingest replay --source <FILE> [--profile <DIR>] --out <FILE>
```

| フラグ | 説明 |
|------|-------------|
| `--source` | 入力 CSV ファイル |
| `--profile` | プロファイルディレクトリ（将来の使用のために予約済み） |
| `--out` | 出力 EntityFrame JSONL ファイル |

**CSV 形式**（ヘッダ必須）：

```
timestamp_ms,entity_id,entity_type,x,y,vx,vy
0,FL-01,forklift,25.0,8.0,-1.0,0.0
0,W-03,pedestrian,15.0,8.0,0.0,0.0
1000,FL-01,forklift,24.0,8.0,-1.0,0.0
```

**出力スキーマ**（`eds.entity-frame`）：CSV の各行に1レコード。

```json
{"eds_schema":"eds.entity-frame","version":"0.1"}
{"timestamp_ms":0,"entity_id":"FL-01","entity_type":"forklift","x":25.0,"y":8.0,"vx":-1.0,"vy":0.0}
```

## eds ingest stream

ライブ UDP ソースからエンティティ位置をストリームします。リアルタイムデプロイメントに使用されます。

```
eds ingest stream --source <udp://HOST:PORT> --profile <DIR> --out <FILE>
```

| フラグ | 説明 |
|------|-------------|
| `--source` | UDP アドレス（例：`udp://127.0.0.1:9000`） |
| `--profile` | プロファイルディレクトリ |
| `--out` | 出力 EntityFrame JSONL ファイル |

UDP から JSON エンコードされたエンティティパケットを読み込み、プロセスが中断されるまで出力 JSONL ファイルに書き込みます。ライブ監視ループで `eds evaluate run` にパイプするために設計されています。

## eds parse maritime

CSV から構造化された海事航海データを `DocumentEntity` JSONL に解析します。
ドキュメントコンプライアンスパイプラインの取込ステップとして使用されます。

```
eds parse maritime --source <FILE> --out <FILE>
```

| フラグ | 説明 |
|------|-------------|
| `--source` | 入力海事航海 CSV ファイル |
| `--out` | 出力 DocumentEntity JSONL ファイル |

**CSV 形式**（ヘッダ必須）：

```
voyage_id,vessel_name,vessel_imo,flag_state,port_of_arrival,arrival_date,
cargo_description,cargo_hs_code,crew_count,gross_tonnage,
bwm_certificate_expiry,dangerous_goods,quarantine_status
```

空のセルは出力では `null` になります。ブール型フィールドは `true`/`false`/`1`/`0` を受け付けます。

**出力スキーマ**（`eds.document-entity`）：

```json
{"eds_schema":"eds.document-entity","version":"0.1"}
{"voyage_id":"V001","vessel_name":"MV Horizon","vessel_imo":"IMO9876543",
 "flag_state":"SGP","port_of_arrival":"SGSIN","arrival_date":"2026-06-15",
 "cargo_description":"General industrial machinery","cargo_hs_code":"8428",
 "crew_count":23,"gross_tonnage":45000.0,"bwm_certificate_expiry":"2027-03-01",
 "dangerous_goods":false,"quarantine_status":"CLEAR","crew_nationalities":null}
```

## eds parse document / form

構造化された JSON ドキュメントまたはフォームを `EntityFrame` JSONL に解析し、`eds evaluate run` で直接使用できます。

```
eds parse document --source <FILE> --out <FILE>
eds parse form     --source <FILE> --out <FILE>
```

`document` と `form` は同等です ── どちらも `entities` 配列を持つ JSON オブジェクトを受け付けます。

**入力形式**（`crates/edgesentry-parse/fixtures/sample_document.json`）：

```json
{
  "site": "Demo Warehouse A",
  "recorded_at": "2026-04-30T09:00:00Z",
  "entities": [
    {"id": "FL-01", "type": "Forklift",    "x": 10.0, "y": 8.0, "vx": -1.0, "vy": 0.0, "timestamp_ms": 0},
    {"id": "W-03",  "type": "Person", "x": 5.0,  "y": 8.0, "vx": 0.0,  "vy": 0.0, "timestamp_ms": 0}
  ]
}
```

**出力スキーマ**（`eds.entity-frame`） ── `eds ingest replay` と同じで、`eds evaluate run` に直接フィードされます。

## eds parse image

スタブ ── コンパイル時に `onnx` フィーチャーフラグを有効にする必要があります。

```
eds parse image --source <FILE> --out <FILE>
```

空の `eds.entity-frame` JSONL を書き込み、警告を表示します。`onnx` フィーチャーが有効になっている場合、完全な ONNX ベースのオブジェクト検出が `edgesentry-image-utils` に実装されます。
