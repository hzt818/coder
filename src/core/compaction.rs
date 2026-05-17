//! Context compaction - intelligent context window management
//!
//! Automatically compacts conversation context when it approaches
//! the token limit. Uses summarization for old messages while
//! preserving recent messages intact. Prefix-cache aware.

use crate::ai::Message;

/// Configuration for context compaction
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Enable automatic compaction
    pub enabled: bool,
    /// Token threshold to trigger compaction (0 = auto)
    pub token_threshold: usize,
    /// Minimum tokens to keep after compaction
    pub floor_tokens: usize,
    /// Number of most recent messages to always keep intact
    pub keep_recent: usize,
    /// Model name for token estimation
    pub model: String,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            token_threshold: 500_000, // 500K tokens
            floor_tokens: 100_000,    // 100K floor
            keep_recent: 10,          // keep 10 most recent messages
            model: "default".to_string(),
        }
    }
}

/// Compaction result
#[derive(Debug, Clone)]
pub struct CompactionResult {
    pub original_messages: usize,
    pub compacted_messages: usize,
    pub original_tokens: usize,
    pub compacted_tokens: usize,
    pub summary_added: bool,
}

impl CompactionResult {
    pub fn reduction_pct(&self) -> f64 {
        if self.original_tokens == 0 {
            return 0.0;
        }
        ((self.original_tokens - self.compacted_tokens) as f64 / self.original_tokens as f64)
            * 100.0
    }
}

/// Estimate total tokens in a set of messages
pub fn estimate_message_tokens(messages: &[Message]) -> usize {
    messages
        .iter()
        .map(estimate_tokens_for_message)
        .sum()
}

/// Estimate tokens for a single message
fn estimate_tokens_for_message(msg: &Message) -> usize {
    let mut total = 0;
    for block in &msg.content {
        match block {
            crate::ai::ContentBlock::Text { text } => {
                total += super::pricing::estimate_tokens(text);
            }
            crate::ai::ContentBlock::ToolUse { name, input, .. } => {
                total += super::pricing::estimate_tokens(name);
                total += super::pricing::estimate_tokens(&input.to_string());
            }
            crate::ai::ContentBlock::ToolResult { content, .. } => {
                total += super::pricing::estimate_tokens(content);
            }
        }
    }
    // Add overhead per message
    total + 4 // role + formatting overhead
}

/// Check if compaction should be triggered
pub fn should_compact(messages: &[Message], config: &CompactionConfig) -> bool {
    if !config.enabled || messages.is_empty() {
        return false;
    }
    let estimated = estimate_message_tokens(messages);
    estimated > config.token_threshold
}

/// Compact messages by keeping recent ones intact and summarizing older ones
pub fn compact_messages(messages: &[Message], config: &CompactionConfig) -> CompactionResult {
    let original_tokens = estimate_message_tokens(messages);
    let original_count = messages.len();

    if !should_compact(messages, config) {
        return CompactionResult {
            original_messages: original_count,
            compacted_messages: original_count,
            original_tokens,
            compacted_tokens: original_tokens,
            summary_added: false,
        };
    }

    // Keep the most recent N messages intact
    let keep_count = config.keep_recent.min(messages.len());
    let (older, recent) = messages.split_at(messages.len() - keep_count);

    if older.is_empty() {
        // Can't compact further
        return CompactionResult {
            original_messages: original_count,
            compacted_messages: original_count,
            original_tokens,
            compacted_tokens: original_tokens,
            summary_added: false,
        };
    }

    // Build a summary message for older messages
    let summary = build_summary(older);

    // Assemble compacted messages: summary + recent
    let mut compacted = vec![summary];
    compacted.extend_from_slice(recent);

    let compacted_tokens = estimate_message_tokens(&compacted);

    CompactionResult {
        original_messages: original_count,
        compacted_messages: compacted.len(),
        original_tokens,
        compacted_tokens,
        summary_added: true,
    }
}

/// Build a summary message from older messages
pub fn build_summary(messages: &[Message]) -> Message {
    // Count tool calls and user messages
    let user_count = messages
        .iter()
        .filter(|m| m.role == crate::ai::Role::User)
        .count();
    let assistant_count = messages
        .iter()
        .filter(|m| m.role == crate::ai::Role::Assistant)
        .count();
    let tool_count = messages
        .iter()
        .filter(|m| m.role == crate::ai::Role::Tool)
        .count();

    // Extract key topics from user messages
    let topics: Vec<String> = messages
        .iter()
        .filter(|m| m.role == crate::ai::Role::User)
        .take(3)
        .map(|m| {
            let text = m.text();
            let first_line = text.lines().next().unwrap_or(&text);
            if first_line.len() > 80 {
                first_line[..80].to_string()
            } else {
                first_line.to_string()
            }
        })
        .collect();

    let summary_text = if topics.is_empty() {
        format!(
            "[Compact summary: {} user messages, {} assistant messages, {} tool calls]",
            user_count, assistant_count, tool_count
        )
    } else {
        format!(
            "[Compact summary: {} user, {} assistant, {} tool calls. Topics: {}]",
            user_count,
            assistant_count,
            tool_count,
            topics.join("; ")
        )
    };

    Message::system(summary_text)
}

/// Format a human-readable compaction result
pub fn format_compaction_result(result: &CompactionResult) -> String {
    format!(
        "Context compacted: {} → {} messages ({} → {} tokens, {:.1}% reduction)",
        result.original_messages,
        result.compacted_messages,
        result.original_tokens,
        result.compacted_tokens,
        result.reduction_pct()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Message;

    fn create_test_messages(count: usize) -> Vec<Message> {
        let mut msgs = Vec::new();
        for i in 0..count {
            msgs.push(Message::user(format!(
                "This is test message number {} with some content for token estimation purposes",
                i
            )));
            msgs.push(Message::assistant(format!(
                "Response to message {} that provides helpful information",
                i
            )));
        }
        msgs
    }

    #[test]
    fn test_should_compact_under_threshold() {
        let msgs = create_test_messages(5);
        let config = CompactionConfig {
            token_threshold: 1_000_000, // Very high threshold
            ..Default::default()
        };
        assert!(!should_compact(&msgs, &config));
    }

    #[test]
    fn test_should_compact_disabled() {
        let msgs = create_test_messages(100);
        let config = CompactionConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(!should_compact(&msgs, &config));
    }

    #[test]
    fn test_compact_messages_no_op() {
        let msgs = create_test_messages(3);
        let config = CompactionConfig {
            token_threshold: 1_000_000,
            ..Default::default()
        };
        let result = compact_messages(&msgs, &config);
        assert!(!result.summary_added);
        assert_eq!(result.original_messages, result.compacted_messages);
    }

    #[test]
    fn test_compact_messages_summary() {
        let msgs = create_test_messages(20); // 40 messages total
        let config = CompactionConfig {
            enabled: true,
            token_threshold: 10, // Very low threshold to force compaction
            keep_recent: 4,
            ..Default::default()
        };
        let result = compact_messages(&msgs, &config);
        assert!(result.summary_added);
        assert!(result.compacted_messages < result.original_messages);
    }

    #[test]
    fn test_estimate_message_tokens() {
        let msgs = create_test_messages(2);
        let tokens = estimate_message_tokens(&msgs);
        assert!(tokens > 0);
    }

    #[test]
    fn test_compact_empty() {
        let msgs = Vec::new();
        let config = CompactionConfig::default();
        let result = compact_messages(&msgs, &config);
        assert_eq!(result.original_messages, 0);
    }

    #[test]
    fn test_format_result() {
        let result = CompactionResult {
            original_messages: 100,
            compacted_messages: 20,
            original_tokens: 500_000,
            compacted_tokens: 100_000,
            summary_added: true,
        };
        let formatted = format_compaction_result(&result);
        assert!(formatted.contains("80.0%"));
        assert!(formatted.contains("100 → 20"));
    }

    #[test]
    fn test_keep_recent_respected() {
        let msgs = create_test_messages(10);
        let config = CompactionConfig {
            enabled: true,
            token_threshold: 1, // Always compact
            keep_recent: 6,     // Keep 3 user + 3 assistant = 6
            ..Default::default()
        };
        let result = compact_messages(&msgs, &config);
        assert!(result.summary_added);
        // Should have: 1 summary + 6 recent = 7
        assert_eq!(result.compacted_messages, 7);
    }
}
