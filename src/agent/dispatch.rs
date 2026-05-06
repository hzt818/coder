//! Agent dispatch - routes to the appropriate agent type with focused system prompts

use super::types::AgentType;

/// Route a request to the appropriate agent handler and return the system prompt.
pub fn route_to_agent(agent_type: &AgentType) -> &'static str {
    get_agent_prompt(agent_type)
}

/// Get the system prompt for a given agent type.
pub fn get_agent_prompt(agent_type: &AgentType) -> &'static str {
    match agent_type {
        AgentType::Coding => {
            "You are a coding assistant. Write correct, idiomatic code. \
             Follow the project's existing patterns. Test your changes."
        }
        AgentType::Research => {
            "You are a research assistant. Search for information thoroughly. \
             Use web_search and docs tools. Cite your sources. \
             Be objective and comprehensive."
        }
        AgentType::Debug => {
            "You are a debugging specialist. Follow a systematic approach:\n\
             1. Reproduce the issue and gather diagnostic data\n\
             2. Formulate hypotheses about root causes\n\
             3. Test each hypothesis with minimal experiments\n\
             4. Implement the fix once the root cause is confirmed\n\
             5. Verify the fix doesn't introduce regressions"
        }
        AgentType::Plan => {
            "You are a planning specialist. Before writing any code:\n\
             1. Understand the full requirements\n\
             2. Explore the codebase to understand existing patterns\n\
             3. Design the architecture (components, data flow, interfaces)\n\
             4. Identify risks and edge cases\n\
             5. Produce a step-by-step implementation plan\n\
             You may NOT execute shell commands or write files."
        }
        AgentType::Review => {
            "You are a code review specialist. Review code for:\n\
             1. Correctness: bugs, logic errors, edge cases\n\
             2. Security: injection, authentication, data handling\n\
             3. Performance: unnecessary allocations, N+1 queries\n\
             4. Style: consistency with project conventions\n\
             5. Maintainability: readability, testability\n\
             Be constructive and specific in your feedback."
        }
    }
}

/// Determine if the agent type has shell execution privileges.
pub fn agent_allows_shell(agent_type: &AgentType) -> bool {
    match agent_type {
        AgentType::Plan | AgentType::Review => false,
        AgentType::Coding | AgentType::Research | AgentType::Debug => true,
    }
}

/// Determine if the agent type can write files.
pub fn agent_allows_file_write(agent_type: &AgentType) -> bool {
    match agent_type {
        AgentType::Plan | AgentType::Research | AgentType::Review => false,
        AgentType::Coding | AgentType::Debug => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_to_agent() {
        assert_eq!(
            route_to_agent(&AgentType::Coding),
            get_agent_prompt(&AgentType::Coding)
        );
        assert_eq!(
            route_to_agent(&AgentType::Research),
            get_agent_prompt(&AgentType::Research)
        );
    }

    #[test]
    fn test_agent_types_have_prompts() {
        for t in &[
            AgentType::Coding,
            AgentType::Research,
            AgentType::Debug,
            AgentType::Plan,
            AgentType::Review,
        ] {
            let prompt = get_agent_prompt(t);
            assert!(!prompt.is_empty(), "AgentType {:?} has empty prompt", t);
        }
    }

    #[test]
    fn test_agent_permissions() {
        assert!(!agent_allows_shell(&AgentType::Plan));
        assert!(!agent_allows_shell(&AgentType::Review));
        assert!(agent_allows_shell(&AgentType::Coding));
        assert!(!agent_allows_file_write(&AgentType::Plan));
        assert!(!agent_allows_file_write(&AgentType::Review));
        assert!(agent_allows_file_write(&AgentType::Coding));
    }
}
