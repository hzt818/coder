use async_trait::async_trait;

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
