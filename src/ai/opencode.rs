//! OpenCode Zen API provider
//!
//! Provides access to OpenCode's free AI models via their Zen API proxy.
//! Supports two modes:
//! - Anonymous (no API key): IP-based rate limiting, limited model set
//! - Authenticated (with API key): workspace-based access, full model set

use async_trait::async_trait;
use serde::Deserialize;
use super::*;
use super::provider::{Provider, StreamHandler};

/// Response from GET /zen/v1/models
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
    #[allow(dead_code)]
    #[serde(default)]
    object: String,
}

/// OpenCode Zen API provider
#[derive(Debug)]
pub struct OpenCodeProvider {
    /// API key (None = anonymous/free tier mode)
    pub api_key: Option<String>,
    /// Base URL for Zen API (default: https://opencode.ai/zen/v1)
    pub base_url: String,
    /// Current model name
    model: String,
    /// Available models fetched from /zen/v1/models
    available_models: Vec<String>,
}

impl OpenCodeProvider {
    pub fn new(api_key: Option<String>, base_url: Option<String>, model: String) -> Self {
        let base_url = base_url
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| "https://opencode.ai/zen/v1".to_string())
            .trim_end_matches('/')
            .to_string();
        Self { api_key, base_url, model, available_models: Vec::new() }
    }

    pub async fn fetch_models(&self) -> anyhow::Result<Vec<String>> {
        let client = reqwest::Client::new();
        let mut req = client.get(format!("{}/models", self.base_url));
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("OpenCode models API error ({}): {}", status, body);
        }
        let data: ModelsResponse = resp.json().await?;
        Ok(data.data.into_iter().map(|m| m.id).collect())
    }

    pub fn set_available_models(&mut self, models: Vec<String>) {
        self.available_models = models;
    }

    pub fn available_models(&self) -> &[String] {
        &self.available_models
    }

    pub fn is_anonymous(&self) -> bool {
        self.api_key.is_none()
    }

    fn build_request(&self, messages: &[Message], tools: &[ToolDef], config: &GenerateConfig) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages_to_openai(messages),
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
            "stream": true,
        });
        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools.iter().map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    }
                })
            }).collect::<Vec<_>>());
        }
        body
    }
}

#[async_trait]
impl Provider for OpenCodeProvider {
    fn name(&self) -> &str { "OpenCode" }
    fn model(&self) -> &str { &self.model }

    async fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        config: &GenerateConfig,
    ) -> anyhow::Result<StreamHandler> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        let client = reqwest::Client::new();
        let request_body = self.build_request(messages, tools, config);
        let mut request = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body);
        if let Some(ref key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }
        let response = request.send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            let msg = match status {
                429 => format!("OpenCode API rate limit exceeded. {}", if self.is_anonymous() {
                    "Try again later or get a free API key at https://opencode.ai/zen"
                } else { "Try again later." }),
                401 | 403 => "Invalid or expired API key. Check your key at https://opencode.ai/zen".to_string(),
                404 => format!("Model '{}' not available via OpenCode. Run /model to see available models.", self.model),
                _ => format!("OpenCode API error ({}): {}", status, body),
            };
            anyhow::bail!("{}", msg);
        }
        tokio::spawn(async move {
            if let Err(e) = super::openai::parse_sse_stream_public(response, tx.clone()).await {
                tracing::error!("OpenCode SSE parse error: {}", e);
                let _ = tx.send(StreamEvent::Error(e.to_string())).await;
            }
        });
        Ok(rx)
    }
}

fn messages_to_openai(messages: &[Message]) -> Vec<serde_json::Value> {
    messages.iter().map(|msg| {
        let role = msg.role.to_string();
        let mut json_msg = serde_json::json!({ "role": role, "content": msg.text() });
        if let Some(tcid) = &msg.tool_call_id {
            json_msg["tool_call_id"] = serde_json::json!(tcid);
        }
        json_msg
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_default_url() {
        let p = OpenCodeProvider::new(None, None, "claude-sonnet-4-6".to_string());
        assert_eq!(p.base_url, "https://opencode.ai/zen/v1");
        assert!(p.is_anonymous());
        assert_eq!(p.model(), "claude-sonnet-4-6");
    }

    #[test]
    fn test_new_with_custom_url() {
        let p = OpenCodeProvider::new(Some("opk_test".to_string()), Some("https://custom.zen.url/v1".to_string()), "gpt-4o".to_string());
        assert_eq!(p.base_url, "https://custom.zen.url/v1");
        assert!(!p.is_anonymous());
    }

    #[test]
    fn test_new_trims_trailing_slash() {
        let p = OpenCodeProvider::new(None, Some("https://opencode.ai/zen/v1/".to_string()), "m".to_string());
        assert_eq!(p.base_url, "https://opencode.ai/zen/v1");
    }

    #[test]
    fn test_build_request_basic() {
        let p = OpenCodeProvider::new(None, None, "claude-sonnet-4-6".to_string());
        let msgs = vec![Message::user("hello")];
        let tools = vec![];
        let config = GenerateConfig::default();
        let body = p.build_request(&msgs, &tools, &config);
        assert_eq!(body["model"], "claude-sonnet-4-6");
    }

    #[test]
    fn test_build_request_with_tools() {
        let p = OpenCodeProvider::new(None, None, "m".to_string());
        let msgs = vec![Message::user("list files")];
        let tools = vec![ToolDef {
            name: "bash".to_string(), description: "Run shell".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        }];
        let config = GenerateConfig::default();
        let body = p.build_request(&msgs, &tools, &config);
        assert!(body["tools"].is_array());
    }

    #[test]
    fn test_available_models() {
        let mut p = OpenCodeProvider::new(None, None, "m".to_string());
        assert!(p.available_models().is_empty());
        p.set_available_models(vec!["m1".to_string(), "m2".to_string()]);
        assert_eq!(p.available_models().len(), 2);
    }
}
