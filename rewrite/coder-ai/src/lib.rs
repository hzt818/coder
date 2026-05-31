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
        OpenAIProvider::new_with_client(Client::new(), api_key, base_url, model)
    }

    /// Create a provider with an injected reqwest::Client (useful for tests)
    pub fn new_with_client(client: Client, api_key: Option<String>, base_url: Option<&str>, model: Option<&str>) -> Self {
        OpenAIProvider {
            client,
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
            Some(k) if !k.is_empty() => Some(k.clone()),
            _ => None,
        };

        if api_key.is_none() {
            return Ok(format!("openai-mock: {}", prompt));
        }
        let api_key = api_key.unwrap();

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


#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[tokio::test]
    async fn openai_provider_mock_when_no_key() {
        let p = OpenAIProvider::new(None, None, None);
        let out = p.complete("hello").await.unwrap();
        assert_eq!(out, "openai-mock: hello");
    }

    #[tokio::test]
    async fn openai_provider_calls_api() {
        // start a mock server
        let server = MockServer::start();

        // Expected response body matching ChatResponse structure
        let body = r#"{"choices":[{"message":{"content":"hello-response"}}]}"#;

        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/chat/completions");
            then.status(200)
                .header("content-type", "application/json")
                .body(body);
        });

        let client = reqwest::Client::new();
        let base = server.url("/");
        let p = OpenAIProvider::new_with_client(client, Some("testkey".to_string()), Some(&base), Some("gpt-test"));
        let out = p.complete("input prompt").await.unwrap();
        assert_eq!(out, "hello-response");

        mock.assert();
    }
}
