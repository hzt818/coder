//! Agent type definitions

use serde::{Deserialize, Serialize};

/// Types of agents with different specializations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentType {
    /// Full tool access, general coding (default)
    Coding,
    /// Web search and research focused
    Research,
    /// Systematic debugging with specialized tools
    Debug,
    /// Planning and analysis mode (read-only tools)
    Plan,
    /// Code review specialist
    Review,
}

impl AgentType {
    /// Get the system prompt for this agent type
    pub fn system_prompt(&self) -> &'static str {
        match self {
            AgentType::Coding => {
                "You are Coder, an AI-powered development tool. \
                 You help users write, debug, and understand code. \
                 You have access to tools that let you execute commands, read/write files, and more. \
                 Be concise, accurate, and helpful. 🦀"
            }
            AgentType::Research => {
                "You are Coder in research mode. Your primary goal is to gather information \
                 using web search, documentation lookup, and code search. \
                 Do not modify files unless explicitly asked."
            }
            AgentType::Debug => {
                "You are Coder in debug mode. Systematically identify and fix issues. \
                 Use bash, grep, file_read, and LSP tools to understand the problem. \
                 Formulate and test hypotheses methodically."
            }
            AgentType::Plan => {
                "You are Coder in planning mode. Analyze requirements, consider trade-offs, \
                 and create detailed implementation plans. Do not make changes until the plan is approved."
            }
            AgentType::Review => {
                "You are Coder in review mode. Carefully examine code for bugs, \
                 security issues, and quality problems. Provide actionable feedback."
            }
        }
    }

    /// Default display name
    pub fn display_name(&self) -> &'static str {
        match self {
            AgentType::Coding => "coding",
            AgentType::Research => "research",
            AgentType::Debug => "debug",
            AgentType::Plan => "plan",
            AgentType::Review => "review",
        }
    }
}

impl Default for AgentType {
    fn default() -> Self {
        Self::Coding
    }
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
