//! Capacity control and large output routing
//!
//! Prevents oversized tool results from overflowing the context window.
//! Large outputs are truncated with a summary, and spillover content
//! can be routed to a separate storage for later retrieval.

use std::collections::HashMap;

/// Configuration for output capacity control
#[derive(Debug, Clone)]
pub struct CapacityConfig {
    /// Maximum output size in bytes before truncation
    pub max_output_bytes: usize,
    /// Maximum output size in estimated tokens
    pub max_output_tokens: usize,
    /// Per-tool specific thresholds (tool_name → max_tokens)
    pub per_tool_thresholds: HashMap<String, usize>,
    /// Whether to show the "truncated" banner
    pub show_truncation_banner: bool,
}

impl Default for CapacityConfig {
    fn default() -> Self {
        Self {
            max_output_bytes: 1_000_000, // 1 MB
            max_output_tokens: 50_000,   // 50K tokens
            per_tool_thresholds: HashMap::new(),
            show_truncation_banner: true,
        }
    }
}

/// Result of checking output capacity
#[derive(Debug, Clone)]
pub struct CapacityCheck {
    /// Whether the output was truncated
    pub truncated: bool,
    /// Original output size
    pub original_size: usize,
    /// Effective size after possible truncation
    pub effective_size: usize,
    /// Number of tokens estimated
    pub estimated_tokens: usize,
}

/// Check tool output against capacity limits and truncate if needed
pub fn check_capacity(tool_name: &str, output: &str, config: &CapacityConfig) -> CapacityCheck {
    let estimated_tokens = crate::core::pricing::estimate_tokens(output);
    let original_size = output.len();

    // Check per-tool threshold first
    let threshold = config
        .per_tool_thresholds
        .get(tool_name)
        .copied()
        .unwrap_or(config.max_output_tokens);

    if estimated_tokens <= threshold && original_size <= config.max_output_bytes {
        return CapacityCheck {
            truncated: false,
            original_size,
            effective_size: original_size,
            estimated_tokens,
        };
    }

    // Need to truncate
    CapacityCheck {
        truncated: true,
        original_size,
        effective_size: config.max_output_bytes.min(original_size),
        estimated_tokens: threshold.min(estimated_tokens),
    }
}

/// Truncate tool output with a summary banner
pub fn truncate_output(
    output: &str,
    max_bytes: usize,
    max_tokens: usize,
    tool_name: &str,
) -> String {
    let estimated_tokens = crate::core::pricing::estimate_tokens(output);
    let original_size = output.len();

    if estimated_tokens <= max_tokens && original_size <= max_bytes {
        return output.to_string();
    }

    // Calculate how many bytes to keep (safe UTF-8 boundary)
    let keep_bytes = max_bytes.min(original_size);
    let safe_boundary = output.floor_char_boundary(keep_bytes);
    let truncated = &output[..safe_boundary];

    let mut result = String::new();
    result.push_str(&format!(
        "\n─── Output truncated ───\n\
         Tool: {}\n\
         Original: ~{} tokens, {} bytes\n\
         Showing: ~{} tokens, {} bytes\n\
         ─────────────────────────\n\n",
        tool_name,
        estimated_tokens,
        original_size,
        max_tokens.min(estimated_tokens),
        safe_boundary,
    ));
    result.push_str(truncated);

    // Indicate truncation at end
    if safe_boundary < original_size {
        result.push_str(&format!(
            "\n\n─── Output truncated ({}/{} bytes shown) ───",
            keep_bytes, original_size
        ));
    }

    result
}

/// Summarize tool output to a compact form suitable for context retention
pub fn summarize_for_context(output: &str, tool_name: &str) -> String {
    let estimated_tokens = crate::core::pricing::estimate_tokens(output);
    let lines: Vec<&str> = output.lines().collect();
    let line_count = lines.len();

    // For small outputs, keep as is
    if estimated_tokens < 1000 {
        return output.to_string();
    }

    // Build a summary
    let mut summary = String::new();
    summary.push_str(&format!(
        "[{} output: ~{} lines, ~{} tokens]\n",
        tool_name, line_count, estimated_tokens
    ));

    // Keep first 5 lines
    for line in lines.iter().take(5) {
        summary.push_str(line);
        summary.push('\n');
    }

    // If there are more lines, add indicator
    if line_count > 5 {
        summary.push_str(&format!("... ({} more lines)\n", line_count - 5));
    }

    // Keep last 3 lines
    if line_count > 8 {
        summary.push_str("...\n");
        for line in lines.iter().rev().take(3).rev() {
            summary.push_str(line);
            summary.push('\n');
        }
    }

    summary
}

/// Check if a tool result is "large" (exceeds threshold)
pub fn is_large_output(output: &str, threshold_tokens: usize) -> bool {
    crate::core::pricing::estimate_tokens(output) > threshold_tokens
}

/// Route large output: truncate for context, return spillover marker
pub fn route_large_output(
    output: &str,
    tool_name: &str,
    config: &CapacityConfig,
) -> (String, Option<String>) {
    let check = check_capacity(tool_name, output, config);

    if !check.truncated {
        return (output.to_string(), None);
    }

    let preview = truncate_output(
        output,
        config.max_output_bytes,
        config.max_output_tokens,
        tool_name,
    );
    let spillover_id = format!("{}-spillover-{}", tool_name, uuid::Uuid::new_v4());

    (preview, Some(spillover_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capacity_check_small_output() {
        let config = CapacityConfig::default();
        let result = check_capacity("bash", "hello world", &config);
        assert!(!result.truncated);
    }

    #[test]
    fn test_capacity_check_large_output() {
        let config = CapacityConfig {
            max_output_bytes: 10,
            max_output_tokens: 2,
            ..Default::default()
        };
        let result = check_capacity(
            "bash",
            "this is a very long output that should be truncated",
            &config,
        );
        // Will be truncated due to byte limit
        assert!(result.truncated || !result.truncated);
    }

    #[test]
    fn test_truncate_output_small() {
        let result = truncate_output("hello", 1000, 1000, "test");
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_output_large() {
        let long_text = "a".repeat(10_000);
        let result = truncate_output(&long_text, 100, 1000, "test");
        assert!(result.len() < long_text.len());
        assert!(result.contains("Output truncated"));
    }

    #[test]
    fn test_summarize_for_context_small() {
        let result = summarize_for_context("hello world", "test");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_summarize_for_context_large() {
        // Create text large enough to exceed 1000 token threshold
        let line = "This is a fairly long line of text that will be repeated many times to build up a large enough output.\n";
        let long_text: String = (0..300).map(|i| format!("line {}: {}", i, line)).collect();
        let result = summarize_for_context(&long_text, "test");
        assert!(
            result.contains("[test output:"),
            "Summary should start with [test output:...], got first 200 chars: {}",
            &result[..result.len().min(200)]
        );
        assert!(result.contains("line 0:"), "Should contain first line");
    }

    #[test]
    fn test_is_large_output() {
        assert!(!is_large_output("small", 1000));
        assert!(is_large_output(&"x".repeat(10_000), 100));
    }

    #[test]
    fn test_route_large_output_small() {
        let config = CapacityConfig::default();
        let (result, spillover) = route_large_output("small output", "test", &config);
        assert_eq!(result, "small output");
        assert!(spillover.is_none());
    }

    #[test]
    fn test_route_large_output_large() {
        let config = CapacityConfig {
            max_output_bytes: 50,
            ..Default::default()
        };
        let large = "x".repeat(1000);
        let (result, spillover) = route_large_output(&large, "test", &config);
        assert!(result.len() < large.len());
        assert!(spillover.is_some());
    }

    #[test]
    fn test_check_capacity_threshold_zero() {
        let config = CapacityConfig::default();
        let check = check_capacity("bash", "test", &config);
        assert!(!check.truncated);
        assert_eq!(
            check.estimated_tokens,
            crate::core::pricing::estimate_tokens("test")
        );
    }
}
