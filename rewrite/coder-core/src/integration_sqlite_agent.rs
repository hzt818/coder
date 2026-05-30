#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use crate::context_sqlite::SqliteContext;
    use crate::SharedContext;
    use crate::agent::SimpleAgent;
    use crate::provider::Provider;
    use crate::tool::Tool;
    use async_trait::async_trait;

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
    async fn agent_with_sqlite_context_records_messages() {
        let sqlite = SqliteContext::new_in_memory().unwrap();
        let ctx: SharedContext = Arc::new(sqlite);

        let agent = SimpleAgent { provider: MockProv, tool: EchoTool, context: ctx.clone() };

        let out = agent.run("hello").await.unwrap();
        assert_eq!(out, "tool:prov:hello");

        // verify context captured the interaction
        let recent = ctx.recent(10).await;
        assert!(recent.len() >= 3);
        assert_eq!(recent[0], "input: hello");
        assert!(recent.iter().any(|s| s.starts_with("provider: prov:")));
        assert!(recent.iter().any(|s| s.starts_with("tool:")));
    }
}
