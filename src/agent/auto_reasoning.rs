//! Auto reasoning effort selection
//!
//! When `reasoning_effort = "auto"`, this module inspects the last user
//! message for keywords and resolves the appropriate tier before each
//! API request. This mirrors the behavior from DeepSeek TUI v0.8.12.

use super::types::ReasoningEffort;

/// Select the reasoning effort based on prompt content analysis.
///
/// Rules:
/// - Debug/error/fix keywords → Max reasoning
/// - Search/lookup/find keywords → Low reasoning
/// - Sub-agent tasks → Low reasoning (handled by caller)
/// - Default → High reasoning
pub fn select_effort(prompt: &str) -> ReasoningEffort {
    let lower = prompt.to_lowercase();

    // Check for debug/error/fix patterns → Max
    let max_keywords = [
        "debug",
        "error",
        "bug",
        "fix",
        "crash",
        "panic",
        "fail",
        "traceback",
        "exception",
        "wrong",
        "incorrect",
        "broken",
    ];
    if max_keywords.iter().any(|k| lower.contains(k)) {
        return ReasoningEffort::Max;
    }

    // Check for search/lookup patterns → Low
    let low_keywords = [
        "search",
        "lookup",
        "find",
        "what is",
        "explain briefly",
        "summarize",
        "list",
        "show me",
        "tell me about",
    ];
    if low_keywords.iter().any(|k| lower.contains(k)) {
        return ReasoningEffort::Low;
    }

    // Default → High
    ReasoningEffort::High
}

/// Determine if this is a sub-agent call that should use Low effort.
pub fn is_subagent_call(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    lower.contains("subagent")
        || lower.contains("sub-agent")
        || lower.starts_with("explore:")
        || lower.starts_with("review:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_debug_effort() {
        let result = select_effort("I need to debug this crash");
        assert_eq!(result, ReasoningEffort::Max);
    }

    #[test]
    fn test_select_error_effort() {
        let result = select_effort("There is an error in the code");
        assert_eq!(result, ReasoningEffort::Max);
    }

    #[test]
    fn test_select_fix_effort() {
        let result = select_effort("Fix the broken function");
        assert_eq!(result, ReasoningEffort::Max);
    }

    #[test]
    fn test_select_search_effort() {
        let result = select_effort("Search for the documentation");
        assert_eq!(result, ReasoningEffort::Low);
    }

    #[test]
    fn test_select_lookup_effort() {
        let result = select_effort("Lookup the API for that library");
        assert_eq!(result, ReasoningEffort::Low);
    }

    #[test]
    fn test_select_summarize_effort() {
        let result = select_effort("Summarize the main features");
        assert_eq!(result, ReasoningEffort::Low);
    }

    #[test]
    fn test_select_default_effort() {
        let result = select_effort("Write a function to parse JSON");
        assert_eq!(result, ReasoningEffort::High);
    }

    #[test]
    fn test_select_default_generic() {
        let result = select_effort("Can you help me with this project?");
        assert_eq!(result, ReasoningEffort::High);
    }

    #[test]
    fn test_is_subagent_call() {
        assert!(is_subagent_call("subagent: explore the codebase"));
        assert!(is_subagent_call("explore: find all call sites"));
        assert!(is_subagent_call("review: audit this PR"));
        assert!(!is_subagent_call("Write a function"));
    }

    #[test]
    fn test_case_insensitive() {
        let result = select_effort("DEBUG the system");
        assert_eq!(result, ReasoningEffort::Max);
    }
}
