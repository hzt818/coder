//! Agent type definitions and interaction modes

use serde::{Deserialize, Serialize};

/// Types of agents with different specializations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum AgentType {
    /// Full tool access, general coding (default)
    #[default]
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

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Interaction mode for the agent loop
///
/// Controls approval behavior and tool availability:
/// - Plan: read-only tools only, no shell/patch execution
/// - Agent: full tool access with approval gates (default)
/// - YOLO: auto-approve all tools, no guardrails
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum InteractionMode {
    Plan,
    #[default]
    Agent,
    Yolo,
}

impl InteractionMode {
    /// Cycle to the next mode: Plan → Agent → Yolo → Plan
    pub fn cycle(&self) -> Self {
        match self {
            InteractionMode::Plan => InteractionMode::Agent,
            InteractionMode::Agent => InteractionMode::Yolo,
            InteractionMode::Yolo => InteractionMode::Plan,
        }
    }

    /// Whether shell commands are auto-approved in this mode
    pub fn auto_approve_shell(&self) -> bool {
        matches!(self, InteractionMode::Yolo)
    }

    /// Whether all tools are auto-approved (no approval gate)
    pub fn auto_approve_all(&self) -> bool {
        matches!(self, InteractionMode::Yolo)
    }

    /// Whether shell execution is allowed at all
    pub fn allow_shell(&self) -> bool {
        !matches!(self, InteractionMode::Plan)
    }

    /// Whether file write/modify operations are allowed
    pub fn allow_file_write(&self) -> bool {
        !matches!(self, InteractionMode::Plan)
    }

    /// Whether the mode enforces read-only operation
    pub fn is_read_only(&self) -> bool {
        matches!(self, InteractionMode::Plan)
    }

    /// Display name
    pub fn display_name(&self) -> &'static str {
        match self {
            InteractionMode::Plan => "Plan",
            InteractionMode::Agent => "Agent",
            InteractionMode::Yolo => "YOLO",
        }
    }

    /// Emoji indicator
    pub fn indicator(&self) -> &'static str {
        match self {
            InteractionMode::Plan => "🔍",
            InteractionMode::Agent => "🤖",
            InteractionMode::Yolo => "⚡",
        }
    }
}

impl std::fmt::Display for InteractionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Reasoning effort levels for LLM inference
///
/// Controls how much thinking/reasoning the model does before responding:
/// - Off: no extended thinking, direct response
/// - Low: minimal reasoning (good for simple lookups)
/// - High: standard reasoning (default)
/// - Max: maximum reasoning depth
/// - Auto: automatically chosen based on prompt content
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum ReasoningEffort {
    Off,
    Low,
    #[default]
    High,
    Max,
    Auto,
}

impl ReasoningEffort {
    /// Cycle to the next effort level: Off → Low → High → Max → Auto → Off
    pub fn cycle(&self) -> Self {
        match self {
            ReasoningEffort::Off => ReasoningEffort::Low,
            ReasoningEffort::Low => ReasoningEffort::High,
            ReasoningEffort::High => ReasoningEffort::Max,
            ReasoningEffort::Max => ReasoningEffort::Auto,
            ReasoningEffort::Auto => ReasoningEffort::Off,
        }
    }

    /// Display name
    pub fn display_name(&self) -> &'static str {
        match self {
            ReasoningEffort::Off => "off",
            ReasoningEffort::Low => "low",
            ReasoningEffort::High => "high",
            ReasoningEffort::Max => "max",
            ReasoningEffort::Auto => "auto",
        }
    }

    /// Whether extended thinking is enabled
    pub fn is_thinking_enabled(&self) -> bool {
        !matches!(self, ReasoningEffort::Off)
    }

    /// Convert to API string parameter if applicable
    pub fn api_value(&self) -> Option<&'static str> {
        match self {
            ReasoningEffort::Low => Some("low"),
            ReasoningEffort::High => Some("high"),
            ReasoningEffort::Max => Some("max"),
            ReasoningEffort::Off | ReasoningEffort::Auto => None,
        }
    }
}

impl std::fmt::Display for ReasoningEffort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod mode_tests {
    use super::*;

    #[test]
    fn test_interaction_mode_cycle() {
        assert_eq!(InteractionMode::Plan.cycle(), InteractionMode::Agent);
        assert_eq!(InteractionMode::Agent.cycle(), InteractionMode::Yolo);
        assert_eq!(InteractionMode::Yolo.cycle(), InteractionMode::Plan);
    }

    #[test]
    fn test_interaction_mode_default() {
        assert_eq!(InteractionMode::default(), InteractionMode::Agent);
    }

    #[test]
    fn test_interaction_mode_read_only() {
        assert!(InteractionMode::Plan.is_read_only());
        assert!(!InteractionMode::Agent.is_read_only());
        assert!(!InteractionMode::Yolo.is_read_only());
    }

    #[test]
    fn test_interaction_mode_auto_approve() {
        assert!(!InteractionMode::Plan.auto_approve_all());
        assert!(!InteractionMode::Agent.auto_approve_all());
        assert!(InteractionMode::Yolo.auto_approve_all());
    }

    #[test]
    fn test_interaction_mode_allow_shell() {
        assert!(!InteractionMode::Plan.allow_shell());
        assert!(InteractionMode::Agent.allow_shell());
        assert!(InteractionMode::Yolo.allow_shell());
    }

    #[test]
    fn test_reasoning_effort_cycle() {
        assert_eq!(ReasoningEffort::Off.cycle(), ReasoningEffort::Low);
        assert_eq!(ReasoningEffort::High.cycle(), ReasoningEffort::Max);
        assert_eq!(ReasoningEffort::Max.cycle(), ReasoningEffort::Auto);
        assert_eq!(ReasoningEffort::Auto.cycle(), ReasoningEffort::Off);
    }

    #[test]
    fn test_reasoning_effort_default() {
        assert_eq!(ReasoningEffort::default(), ReasoningEffort::High);
    }

    #[test]
    fn test_reasoning_effort_thinking() {
        assert!(!ReasoningEffort::Off.is_thinking_enabled());
        assert!(ReasoningEffort::High.is_thinking_enabled());
    }

    #[test]
    fn test_reasoning_effort_api_value() {
        assert_eq!(ReasoningEffort::Low.api_value(), Some("low"));
        assert_eq!(ReasoningEffort::Off.api_value(), None);
        assert_eq!(ReasoningEffort::Auto.api_value(), None);
    }
}
