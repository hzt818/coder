/// coder-core: core runtime primitives (minimal)

use coder_context::{Context, SqliteContext, SharedContext, ContextStore};

use crate::agent::Agent;

pub mod agent {
    use async_trait::async_trait;
    use crate::provider::Provider;
    use crate::tool::Tool;
    use coder_context::SharedContext;
    use anyhow::Result;

    #[async_trait]
    pub trait Agent: Send + Sync {
        async fn run(&self, input: &str) -> Result<String>;
    }

    pub struct SimpleAgent<P: Provider + Send + Sync, T: Tool + Send + Sync> {
        pub provider: P,
        pub tool: T,
        pub context: SharedContext,
    }

    #[async_trait]
    impl<P: Provider + Send + Sync, T: Tool + Send + Sync> Agent for SimpleAgent<P, T>
    {
        async fn run(&self, input: &str) -> Result<String> {
            self.context.push(format!("input: {}", input)).await;
            let completion = self.provider.complete(input).await?;
            self.context.push(format!("provider: {}", completion)).await;
            let tool_out = self.tool.run(&completion).await?;
            self.context.push(format!("tool: {}", tool_out)).await;
            Ok(tool_out)
        }
    }
}

pub mod provider {
    use async_trait::async_trait;
    #[async_trait]
    pub trait Provider: Send + Sync {
        async fn complete(&self, prompt: &str) -> anyhow::Result<String>;
    }
}

pub mod tool {
    use async_trait::async_trait;
    #[async_trait]
    pub trait Tool: Send + Sync {
        async fn name(&self) -> &'static str;
        async fn run(&self, input: &str) -> anyhow::Result<String>;
    }
}

#[cfg(test)]
mod tests {
    use super::agent::{Agent, SimpleAgent};
    use async_trait::async_trait;
    use crate::provider::Provider;
    use crate::tool::Tool;
    use coder_context::Context;
    use coder_context::ContextStore;
    use std::sync::Arc;

    struct MockProv;
    #[async_trait]
    impl Provider for MockProv {
        async fn complete(&self, prompt: &str) -> anyhow::Result<String> {
            Ok(format!("prov:{}", prompt))
        }
    }

    struct EchoTool;
    #[async_trait]
    impl Tool for EchoTool {
        async fn name(&self) -> &'static str { "echo" }
        async fn run(&self, input: &str) -> anyhow::Result<String> { Ok(format!("tool:{}", input)) }
    }

    #[tokio::test]
    async fn simple_agent_runs() {
        let ctx: coder_context::SharedContext = Arc::new(Context::new());
        let agent = SimpleAgent { provider: MockProv, tool: EchoTool, context: ctx };
        let out = Agent::run(&agent, "hello").await.unwrap();
        assert_eq!(out, "tool:prov:hello");
    }
}

pub async fn start() -> &'static str {
    // placeholder for agent loop
    "coder-core started"
}
