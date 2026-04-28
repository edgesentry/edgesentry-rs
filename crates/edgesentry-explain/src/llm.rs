use serde::{Deserialize, Serialize};

/// Thin client for any OpenAI-compatible local LLM server.
///
/// Works with llama-server (llama.cpp) at http://localhost:8080 (default)
/// and with Ollama at http://localhost:11434 when its OpenAI-compat layer is enabled.
pub struct LlmClient {
    base_url: String,
    /// If None, the model ID is discovered from /v1/models on first use.
    model: Option<String>,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: [ChatMessage<'a>; 1],
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelObject>,
}

#[derive(Deserialize)]
struct ModelObject {
    id: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Deserialize)]
struct AssistantMessage {
    content: String,
}

impl LlmClient {
    /// Use an explicit model name (e.g. when the user passes `--model`).
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self { base_url: base_url.into(), model: Some(model.into()) }
    }

    /// Let the server advertise which model is loaded via GET /v1/models.
    pub fn new_autodiscover(base_url: impl Into<String>) -> Self {
        Self { base_url: base_url.into(), model: None }
    }

    /// Default: llama-server on localhost:8080, auto-discover model.
    pub fn default_local() -> Self {
        Self::new_autodiscover("http://localhost:8080")
    }

    /// Query /v1/models and return the first available model ID.
    fn discover_model(&self) -> Result<String, String> {
        let url = format!("{}/v1/models", self.base_url);
        let resp: ModelsResponse = ureq::get(&url)
            .call()
            .map_err(|e| format!("GET /v1/models failed: {e}"))?
            .into_json()
            .map_err(|e| format!("/v1/models parse error: {e}"))?;
        resp.data
            .into_iter()
            .next()
            .map(|m| m.id)
            .ok_or_else(|| "server returned no models".to_string())
    }

    /// Send a prompt; returns the assistant reply text.
    pub fn generate(&self, prompt: &str) -> Result<String, String> {
        let model = match &self.model {
            Some(m) => m.clone(),
            None => self.discover_model()?,
        };

        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = ChatRequest {
            model: &model,
            messages: [ChatMessage { role: "user", content: prompt }],
            stream: false,
        };
        let resp: ChatResponse = ureq::post(&url)
            .send_json(&body)
            .map_err(|e| match e {
                ureq::Error::Status(code, response) => {
                    let body = response.into_string().unwrap_or_default();
                    format!("LLM request failed (HTTP {code}): {body}")
                }
                other => format!("LLM request failed: {other}"),
            })?
            .into_json()
            .map_err(|e| format!("LLM response parse error: {e}"))?;
        resp.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| "LLM returned empty choices".to_string())
    }
}

/// Backward-compatible alias.
pub type OllamaClient = LlmClient;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_constructs_with_explicit_model() {
        let c = LlmClient::new("http://192.168.1.10:8080", "mistral");
        assert_eq!(c.base_url, "http://192.168.1.10:8080");
        assert_eq!(c.model.as_deref(), Some("mistral"));
    }

    #[test]
    fn default_local_uses_localhost_8080() {
        let c = LlmClient::default_local();
        assert!(c.base_url.contains("localhost:8080"));
        assert!(c.model.is_none());
    }

    #[test]
    fn autodiscover_sets_no_model() {
        let c = LlmClient::new_autodiscover("http://localhost:8080");
        assert!(c.model.is_none());
    }
}
