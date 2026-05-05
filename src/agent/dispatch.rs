//! Agent dispatch - routes to the appropriate agent type

use super::types::AgentType;

/// Route a request to the appropriate agent handler based on agent type
pub fn route_to_agent(agent_type: &AgentType) -> &'static str {
    match agent_type {
        AgentType::Coding => "coding_agent",
        AgentType::Research => "research_agent",
        AgentType::Debug => "debug_agent",
        AgentType::Plan => "plan_agent",
        AgentType::Review => "review_agent",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_to_agent() {
        assert_eq!(route_to_agent(&AgentType::Coding), "coding_agent");
        assert_eq!(route_to_agent(&AgentType::Research), "research_agent");
    }
}
