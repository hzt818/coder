use async_trait::async_trait;

#[async_trait]
pub trait Tool: Send + Sync {
    async fn name(&self) -> &'static str;
    async fn run(&self, input: &str) -> anyhow::Result<String>;
}

pub struct EchoTool;
#[async_trait]
impl Tool for EchoTool {
    async fn name(&self) -> &'static str { "echo" }
    async fn run(&self, input: &str) -> anyhow::Result<String> { Ok(input.to_string()) }
}
