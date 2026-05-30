/// coder-core: core runtime primitives (minimal)

use crate::agent::Agent;

pub mod agent {
    use async_trait::async_trait;
    use crate::provider::Provider;
    use crate::tool::Tool;

    #[async_trait]
    pub trait Agent: Send + Sync {
        async fn run(&self, input: &str) -> anyhow::Result<String>;
    }

    pub struct SimpleAgent<P: Provider, T: Tool> {
        pub provider: P,
        pub tool: T,
    }

    #[async_trait]
    impl<P: Provider, T: Tool> Agent for SimpleAgent<P, T>
    where
        P: Provider + Send + Sync,
        T: Tool + Send + Sync,
    {
        async fn run(&self, input: &str) -> anyhow::Result<String> {
            let completion = self.provider.complete(input).await?;
            let tool_out = self.tool.run(&completion).await?;
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
        let agent = SimpleAgent { provider: MockProv, tool: EchoTool };
        // Agent trait is in scope, so run() is available
        let out = Agent::run(&agent, "hello").await.unwrap();
        assert_eq!(out, "tool:prov:hello");
    }
}

pub async fn start() -> &'static str {
    // placeholder for agent loop
    "coder-core started"
}
