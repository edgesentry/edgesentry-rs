# ステップ 5 - 説明

ローカル LLM を使用して RiskEvent の平易な言葉による説明を生成し、プロファイル内の規制 KB スニペットに基づいて根拠付けします。

```
eds explain run --input <FILE> --n <N> --out <FILE>
              [--pick severity|time|random]
              [--llm-url <URL>]
              [--model <NAME>]
              [--profile <DIR>]
```

| フラグ | デフォルト | 説明 |
|------|---------|-------------|
| `--input` | | 入力 RiskEvent JSONL ファイル |
| `--n` | 5 | 説明するイベントの数 |
| `--pick` | `severity` | イベント選択戦略 |
| `--llm-url` | `http://localhost:8080` | OpenAI 互換 LLM サーバーのベース URL |
| `--model` | 自動検出 | モデル名；省略すると `/v1/models` が返す最初のモデルを使用 |
| `--profile` | | 根拠付けのための `kb/<RULE_ID>.txt` スニペットを含むプロファイルディレクトリ |
| `--out` | | 出力 Explanation JSONL ファイル |

## イベント選択戦略

| 戦略 | 動作 |
|---|---|
| `severity` | 最も重大度の高い N 個のイベントを選択（CRITICAL、HIGH、MEDIUM、LOW の順） |
| `time` | タイムスタンプで最新の N 個のイベントを選択 |
| `random` | ランダムに N 個のイベントを選択 |

## LLM サーバーセットアップ

OpenAI `/v1/chat/completions` API を実装する任意のサーバーが動作します：

- **llama.cpp**: `llama-server --model mistral-7b.gguf --port 8080`
- **Ollama**（OpenAI 互換モード）: `OLLAMA_HOST=0.0.0.0 ollama serve`

## KB 根拠付け

`--profile` が指定された場合、`eds explain run` は説明される各イベントについて
`<profile>/kb/<RULE_ID>.txt` のテキストファイルを読み込み、権威ある規制参照として
LLM プロンプトに含めます。

生成後、説明が検証されます：LLM が KB スニペットに存在しないセクション参照
（例：`§3.1`）を引用した場合、`grounded` は `false` に設定されます。

## Explanation スキーマ

```json
{"eds_schema":"eds.explanation","version":"0.1"}
{
  "rule_id": "TTC_ALERT",
  "text": "Forklift FL-01 was on a collision course with worker W-03 with less than 3 seconds to impact, in violation of Site Safety Procedure §3.2.",
  "grounded": true,
  "model": "mistral-7b-instruct"
}
```

`grounded: false` の説明は、公式レポートに含める前に人間によるレビューのためにフラグを立てる必要があります。
