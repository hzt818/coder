//! Subagent spawning - creates isolated agent instances for focused tasks

use std::sync::Arc;
use uuid::Uuid;

use crate::ai::{GenerateConfig, Message, Provider};
use crate::agent::context::Context;
use crate::tool::ToolRegistry;

/// Configuration for spawning a subagent
#[derive(Debug, Clone)]
pub struct SpawnConfig {
    /// Maximum tokens for generation
    pub max_tokens: u64,
    /// Temperature for generation
    pub temperature: f64,
    /// Context window size in tokens
    pub context_window: u64,
    /// System prompt override (uses agent type default if None)
    pub system_prompt: Option<String>,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            max_tokens: 2048,
            temperature: 0.7,
            context_window: 32_000,
            system_prompt: None,
        }
    }
}

/// Handle to a running subagent
///
/// Used to await the subagent's result or cancel it.
pub struct SubagentHandle {
    /// Unique identifier for this subagent
    pub id: String,
    /// Join handle for the spawned task
    handle: tokio::task::JoinHandle<anyhow::Result<String>>,
}

impl SubagentHandle {
    /// Wait for the subagent to complete and return its result
    pub async fn join(self) -> anyhow::Result<String> {
        self.handle
            .await
            .map_err(|e| anyhow::anyhow!("Subagent task failed: {}", e))?
    }

    /// Abort the subagent task
    pub fn abort(self) {
        self.handle.abort();
    }
}

/// Spawn a subagent to process a given task with isolated context.
///
/// The subagent creates a fresh context, loads the provided tools,
/// and runs a single chat completion against the AI provider.
///
/// # Arguments
/// * `provider` - The AI provider to use
/// * `tools` - Available tools for the subagent
/// * `messages` - The conversation messages (context) for this subagent
/// * `config` - Spawn configuration
///
/// # Returns
/// A `SubagentHandle` that can be awaited for the result.
pub fn spawn_subagent(
    provider: Arc<dyn Provider>,
    tools: Arc<ToolRegistry>,
    messages: Vec<Message>,
    config: SpawnConfig,
) -> SubagentHandle {
    let id = Uuid::new_v4().to_string();

    let handle = tokio::spawn(async move {
        let generate_config = GenerateConfig {
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            ..GenerateConfig::default()
        };

        // Build context with system prompt
        let mut context = Context::new(config.context_window);
        if let Some(prompt) = &config.system_prompt {
            context.set_system_prompt(prompt.clone());
        }
        for msg in &messages {
            context.add_message(msg.clone());
        }

        let request_messages = context.build_request();
        let tool_defs = tools.tool_defs();

        // If there are tools, use the provider's chat method with them
        let response = if tool_defs.is_empty() {
            provider
                .chat(&request_messages, &[], &generate_config)
                .await?
        } else {
            provider
                .chat(&request_messages, &tool_defs, &generate_config)
                .await?
        };

        Ok(response.text())
    });

    SubagentHandle { id, handle }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Role;

    #[test]
    fn test_spawn_config_default() {
        let config = SpawnConfig::default();
        assert_eq!(config.max_tokens, 2048);
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.context_window, 32_000);
    }

    #[test]
    fn test_spawn_config_custom() {
        let config = SpawnConfig {
            max_tokens: 4096,
            temperature: 0.5,
            context_window: 64_000,
            system_prompt: Some("Be concise.".to_string()),
        };
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.system_prompt.unwrap(), "Be concise.");
    }

    #[test]
    fn test_subagent_handle_creation() {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async {
            tx.send(()).unwrap();
            Ok::<_, anyhow::Error>("done".to_string())
        });
        // Just verify we can create a handle struct (not actually joining here
        // since the test runtime may not support it in all configurations)
        let _subagent_handle = SubagentHandle {
            id: "test-1".to_string(),
            handle,
        };
    }
}
