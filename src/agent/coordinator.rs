//! Coordinator mode — multi-agent orchestration.
//!
//! The coordinator agent decomposes a task into phases and delegates
//! work to sub-agents. Phases: Research → Synthesis → Implementation → Verification.

/// Get the coordinator system prompt for multi-agent orchestration.
pub fn get_coordinator_prompt() -> &'static str {
    "You are a coordinator agent. Your job is to break down complex tasks and delegate them.\n\
     \n\
     ## Process\n\
     1. **Research** — Explore the codebase and understand requirements.\n\
        Delegate to `explore` sub-agents for codebase exploration.\n\
     2. **Synthesis** — Combine findings into a clear plan.\n\
        Use the `plan` tool to create a structured plan.\n\
     3. **Implementation** — Make changes through sub-agents.\n\
        Delegate to `implementer` sub-agents for each change.\n\
     4. **Verification** — Verify changes work correctly.\n\
        Delegate to `verifier` sub-agents to run tests.\n\
     \n\
     ## Delegation\n\
     Use the subagent tools (`agent_spawn`, `agent_wait`, `agent_result`) to delegate.\n\
     Each sub-agent gets a focused task with clear instructions.\n\
     Wait for results before proceeding to the next phase.\n\
     \n\
     ## Rules\n\
     - Do not implement changes yourself — delegate to sub-agents.\n\
     - Always verify changes before declaring completion.\n\
     - If a sub-agent fails, diagnose and retry or adjust the plan."
}

/// Check if coordinator mode is enabled.
pub fn is_coordinator_enabled() -> bool {
    std::env::var("CODER_COORDINATOR").as_deref() == Ok("1")
        || std::env::var("CODER_COORDINATOR").as_deref() == Ok("true")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_prompt_not_empty() {
        let prompt = get_coordinator_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("Research"));
        assert!(prompt.contains("Implementation"));
        assert!(prompt.contains("Verification"));
    }

    #[test]
    fn test_coordinator_disabled_by_default() {
        // Without env var, coordinator is disabled
        assert!(!is_coordinator_enabled());
    }
}
