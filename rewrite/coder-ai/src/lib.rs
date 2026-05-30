use async_trait::async_trait;
use reqwest::Client;

#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(&self, prompt: &str) -> anyhow::Result<String>;
}

pub struct MockProvider;
#[async_trait]
impl Provider for MockProvider {
    async fn complete(&self, prompt: &str) -> anyhow::Result<String> {
        Ok(format!("mock: {}", prompt))
    }
}

pub struct OpenAIProvider {
    client: Client,
    api_key: Option<String>,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: Option<String>, base_url: Option<&str>) -> Self {
        OpenAIProvider {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or("https://api.openai.com/v1").to_string(),
        }
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    async fn complete(&self, prompt: &str) -> anyhow::Result<String> {
        // Placeholder: if no API key, return a mock-like response
        if self.api_key.is_none() {
            return Ok(format!("openai-mock: {}", prompt));
        }
        // Real request implementation omitted for now; return placeholder
        Ok(format!("openai: {}", prompt))
    }
}
