//! Context management for agent conversations

use crate::ai::Message;

/// Manages the conversation context window
#[derive(Debug, Clone)]
pub struct Context {
    /// Full message history
    messages: Vec<Message>,
    /// Maximum tokens before compaction
    max_tokens: u64,
    /// System prompt
    system_prompt: Option<String>,
}

impl Context {
    /// Create a new empty context
    pub fn new(max_tokens: u64) -> Self {
        Self {
            messages: Vec::new(),
            max_tokens,
            system_prompt: None,
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

    /// Compact old messages into a summary (placeholder for future implementation)
    pub fn compact(&mut self) {
        if self.messages.len() > 10 {
            // Keep last 10 messages, summarize the rest
            // TODO: Implement actual summarization
            let _ = &self.messages[..self.messages.len() - 10];
            let keep = self.messages.split_off(self.messages.len() - 10);
            self.messages = keep;
        }
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
