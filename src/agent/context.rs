//! Context management for agent conversations

use crate::ai::Message;
use crate::core::compaction::{self, CompactionConfig, CompactionResult};

/// Manages the conversation context window
#[derive(Debug, Clone)]
pub struct Context {
    /// Full message history
    messages: Vec<Message>,
    /// Maximum tokens before compaction
    max_tokens: u64,
    /// System prompt
    system_prompt: Option<String>,
    /// Compaction configuration
    compaction_config: CompactionConfig,
}

impl Context {
    /// Create a new empty context
    pub fn new(max_tokens: u64) -> Self {
        Self {
            messages: Vec::new(),
            max_tokens,
            system_prompt: None,
            compaction_config: CompactionConfig::default(),
        }
    }

    /// Set the system prompt
    pub fn set_system_prompt(&mut self, prompt: String) {
        self.system_prompt = Some(prompt);
    }

    /// Get the system prompt
    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    /// Add a message to the history
    pub fn add_message(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    /// Get all messages
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Get messages for the API call (system prompt + messages)
    pub fn build_request(&self) -> Vec<Message> {
        let mut result = Vec::new();

        // Add system prompt as first message if present
        if let Some(prompt) = &self.system_prompt {
            result.push(Message::system(prompt));
        }

        // Add all messages
        result.extend(self.messages.clone());

        result
    }

    /// Number of messages
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if context is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Clear message history (keep system prompt)
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Compact old messages into a summary using intelligent compaction
    pub fn compact(&mut self) -> Option<CompactionResult> {
        let result = compaction::compact_messages(&self.messages, &self.compaction_config);
        if result.summary_added {
            // Replace messages with compacted version
            let keep_count = self.compaction_config.keep_recent.min(self.messages.len());
            let split_at = self.messages.len().saturating_sub(keep_count);
            self.messages.drain(..split_at);
            // The summary was already prepended by compact_messages
            // We need to insert the summary message
            self.messages.insert(0, Message::system(format!(
                "[Prior context: {} messages compacted, ~{:.1}% reduction]",
                result.original_messages,
                result.reduction_pct()
            )));
            Some(result)
        } else {
            None
        }
    }

    /// Check if compaction should be triggered
    pub fn should_compact(&self) -> bool {
        compaction::should_compact(&self.messages, &self.compaction_config)
    }

    /// Get estimated token count
    pub fn estimated_tokens(&self) -> usize {
        compaction::estimate_message_tokens(&self.messages)
    }

    /// Set compaction configuration
    pub fn set_compaction_config(&mut self, config: CompactionConfig) {
        self.compaction_config = config;
    }

    /// Get max tokens
    pub fn max_tokens(&self) -> u64 {
        self.max_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_empty() {
        let ctx = Context::new(128_000);
        assert!(ctx.is_empty());
    }

    #[test]
    fn test_context_add_message() {
        let mut ctx = Context::new(128_000);
        ctx.add_message(Message::user("hello"));
        assert_eq!(ctx.len(), 1);
    }

    #[test]
    fn test_context_system_prompt() {
        let mut ctx = Context::new(128_000);
        ctx.set_system_prompt("You are helpful.".to_string());
        assert_eq!(ctx.system_prompt(), Some("You are helpful."));
    }
}
