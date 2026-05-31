use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[async_trait]
pub trait Tool: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    async fn run(&self, input: &str) -> Result<String>;
}

pub struct EchoTool;
#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &'static str { "echo" }
    async fn run(&self, input: &str) -> Result<String> { Ok(input.to_string()) }
}

pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: RwLock::new(HashMap::new()) }
    }

    pub async fn register(&self, tool: Arc<dyn Tool>) {
        let mut w = self.tools.write().await;
        w.insert(tool.name().to_string(), tool);
    }

    pub async fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        let r = self.tools.read().await;
        r.get(name).cloned()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self { Self::new() }
}
