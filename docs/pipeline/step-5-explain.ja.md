# Step 5 - Explain

Generate plain-language explanations for RiskEvents using a local LLM, grounded against
the regulatory KB snippets in the profile.

```
eds explain run --input <FILE> --n <N> --out <FILE>
              [--pick severity|time|random]
              [--llm-url <URL>]
              [--model <NAME>]
              [--profile <DIR>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--input` | | Input RiskEvent JSONL file |
| `--n` | 5 | Number of events to explain |
| `--pick` | `severity` | Event selection strategy |
| `--llm-url` | `http://localhost:8080` | OpenAI-compatible LLM server base URL |
| `--model` | auto-discovered | Model name; omit to use the first model returned by `/v1/models` |
| `--profile` | | Profile directory with `kb/<RULE_ID>.txt` snippets for grounding |
| `--out` | | Output Explanation JSONL file |

## Event selection strategies

| Strategy | Behaviour |
|---|---|
| `severity` | Selects the N highest-severity events (CRITICAL before HIGH before MEDIUM before LOW) |
| `time` | Selects the N most recent events by timestamp |
| `random` | Selects N events at random |

## LLM server setup

Any server that implements the OpenAI `/v1/chat/completions` API works:

- **llama.cpp**: `llama-server --model mistral-7b.gguf --port 8080`
- **Ollama** (with OpenAI compat mode): `OLLAMA_HOST=0.0.0.0 ollama serve`

## KB grounding

When `--profile` is provided, `eds explain run` loads the text file at
`<profile>/kb/<RULE_ID>.txt` for each event being explained and includes it in the
LLM prompt as the authoritative regulatory reference.

After generation, the explanation is checked: if the LLM cited a section reference
(e.g. `§3.1`) that is not present in the KB snippet, `grounded` is set to `false`.

## Explanation schema

```json
{"eds_schema":"eds.explanation","version":"0.1"}
{
  "rule_id": "TTC_ALERT",
  "text": "Forklift FL-01 was on a collision course with worker W-03 with less than 3 seconds to impact, in violation of Site Safety Procedure §3.2.",
  "grounded": true,
  "model": "mistral-7b-instruct"
}
```

A `grounded: false` explanation should be flagged for human review before inclusion in
an official report.
