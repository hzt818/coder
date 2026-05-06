//! Provider trait - the core interface for AI providers

use super::types::*;
use async_trait::async_trait;

/// Result type for stream handlers
pub type StreamHandler = tokio::sync::mpsc::Receiver<StreamEvent>;

/// AI Provider trait
///
/// All providers (OpenAI, Anthropic, Google, Custom) must implement this trait.
#[async_trait]
pub trait Provider: Send + Sync + std::fmt::Debug {
    /// Provider display name
    fn name(&self) -> &str;

    /// Current model name
    fn model(&self) -> &str;

    /// Whether this provider supports extended thinking
    fn supports_thinking(&self) -> bool {
        false
    }

    /// Stream a chat completion with tool support.
    /// Returns a receiver that yields StreamEvent items.
    async fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        config: &GenerateConfig,
    ) -> anyhow::Result<StreamHandler>;

    /// Non-streaming chat completion.
    /// Returns the full assistant message.
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        config: &GenerateConfig,
    ) -> anyhow::Result<Message> {
        let mut stream = self.chat_stream(messages, tools, config).await?;
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        while let Some(event) = stream.recv().await {
            match event {
                StreamEvent::TextChunk(chunk) => content.push_str(&chunk),
                StreamEvent::ToolCallStart(tc) => tool_calls.push(tc),
                StreamEvent::Done { .. } => break,
                StreamEvent::Error(e) => anyhow::bail!("Provider error: {}", e),
                _ => {}
            }
        }

        let mut msg = Message::assistant(content);
        if !tool_calls.is_empty() {
            for tc in tool_calls {
                msg.content.push(ContentBlock::ToolUse {
                    id: tc.id,
                    name: tc.name,
                    input: tc.arguments,
                });
            }
        }
        Ok(msg)
    }
}
