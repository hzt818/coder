use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String>;
}

pub struct MockProvider;
#[async_trait]
impl Provider for MockProvider {
    async fn complete(&self, prompt: &str) -> Result<String> {
        Ok(format!("mock: {}", prompt))
    }
}

pub struct OpenAIProvider {
    client: Client,
    api_key: Option<String>,
    base_url: String,
    model: String,
}

impl OpenAIProvider {
    pub fn new(api_key: Option<String>, base_url: Option<&str>, model: Option<&str>) -> Self {
        OpenAIProvider {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or("https://api.openai.com/v1").to_string(),
            model: model.unwrap_or("gpt-3.5-turbo").to_string(),
        }
    }
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    max_tokens: Option<i64>,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[async_trait]
impl Provider for OpenAIProvider {
    async fn complete(&self, prompt: &str) -> Result<String> {
        // If no API key configured, return deterministic mock response for tests
        let api_key = match &self.api_key {
            Some(k) if !k.is_empty() => k,
            _ => return Ok(format!("openai-mock: {}", prompt)),
        };

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = ChatRequest {
            model: &self.model,
            messages: vec![ChatMessage { role: "user", content: prompt.to_string() }],
            max_tokens: Some(256),
        };

        let resp = self.client.post(&url)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("request failed: {}", e))?;

        let resp = resp.error_for_status().map_err(|e| anyhow!("bad status: {}", e))?;
        let jr: ChatResponse = resp.json().await.map_err(|e| anyhow!("invalid json: {}", e))?;
        if jr.choices.is_empty() {
            return Err(anyhow!("no choices in response"));
        }
        Ok(jr.choices[0].message.content.clone())
    }
}
