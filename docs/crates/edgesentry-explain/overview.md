# edgesentry-explain

LLM-powered plain-language explanation of `RiskEvent` records.

## Input → Output
`RiskEvent` + KB file for the rule → natural-language explanation string

## LLM backend
OpenAI-compatible endpoint (configurable via `--llm-url`). Falls back to structured
summary without LLM if endpoint is unavailable.
