//! Sub-agent role taxonomy
//!
//! Defines 7 agent roles with distinct system prompts and permission profiles.
//! Each role has a different stance toward the work and different
//! tool access levels.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Sub-agent role types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SubAgentRole {
    /// Flexible; do whatever the parent says
    General,
    /// Read-only; map the relevant code fast
    Explore,
    /// Analyse and produce a strategy
    Plan,
    /// Read-and-grade with severity scores
    Review,
    /// Land a specific change with minimal edits
    Implementer,
    /// Run tests / validation, report outcome
    Verifier,
    /// Explicit narrow tool allowlist
    Custom,
}

impl SubAgentRole {
    /// Get the role's system prompt
    pub fn system_prompt(&self) -> &'static str {
        match self {
            SubAgentRole::General => {
                "You are a general-purpose coding agent. \
                 Complete the task you've been given thoroughly and accurately. \
                 You have full tool access. Be concise and focused."
            }
            SubAgentRole::Explore => {
                "You are an explorer agent. Your job is to read-only explore the codebase. \
                 You can read files, search code, look up documentation, and run read-only shell commands. \
                 Do NOT modify any files. Focus on gathering evidence and understanding the code. \
                 Report your findings in detail."
            }
            SubAgentRole::Plan => {
                "You are a planning agent. Your job is to analyze requirements and create \
                 a detailed implementation plan. You can read files and search the code. \
                 Do NOT modify files. Produce a structured plan with clear steps. \
                 Use update_plan and checklist_write to track your planning."
            }
            SubAgentRole::Review => {
                "You are a code review agent. Carefully examine code for bugs, \
                 security issues, correctness, and quality problems. \
                 You have read-only access. Do not modify files. \
                 Provide actionable feedback with severity scores: \
                 CRITICAL, HIGH, MEDIUM, LOW, or INFO for each finding."
            }
            SubAgentRole::Implementer => {
                "You are an implementer agent. Your job is to land specific, \
                 well-defined changes with minimal side effects. \
                 Stay tightly scoped - implement exactly what was requested, \
                 no drive-by refactoring. Write tests first where appropriate. \
                 Run a quick verification before handing back."
            }
            SubAgentRole::Verifier => {
                "You are a verifier agent. Run tests and validation, then report outcomes. \
                 Do NOT fix failures - capture the failing assertion, stack trace, \
                 and any error output. List fix candidates under RISKS. \
                 You can run shell commands for testing only."
            }
            SubAgentRole::Custom => {
                "You are a custom agent with restricted tool access. \
                 Only use the tools that have been explicitly provided to you. \
                 Stay focused on your specific assignment."
            }
        }
    }

    /// Whether this role can write/modify files
    pub fn can_write_files(&self) -> bool {
        matches!(self, SubAgentRole::General | SubAgentRole::Implementer)
    }

    /// Whether this role can execute shell commands
    pub fn can_run_shell(&self) -> bool {
        matches!(
            self,
            SubAgentRole::General
                | SubAgentRole::Explore
                | SubAgentRole::Implementer
                | SubAgentRole::Verifier
        )
    }

    /// Whether this role is read-only
    pub fn is_read_only(&self) -> bool {
        matches!(self, SubAgentRole::Explore | SubAgentRole::Plan | SubAgentRole::Review)
    }

    /// Aliases for matching from model input
    pub fn aliases(&self) -> &[&'static str] {
        match self {
            SubAgentRole::General => &["general", "worker", "default", "general-purpose"],
            SubAgentRole::Explore => &["explore", "explorer", "exploration"],
            SubAgentRole::Plan => &["plan", "planning", "awaiter"],
            SubAgentRole::Review => &["review", "reviewer", "code-review"],
            SubAgentRole::Implementer => &["implementer", "implement", "implementation", "builder"],
            SubAgentRole::Verifier => &["verifier", "verify", "verification", "validator", "tester"],
            SubAgentRole::Custom => &["custom"],
        }
    }

    /// Display name
    pub fn display_name(&self) -> &'static str {
        match self {
            SubAgentRole::General => "general",
            SubAgentRole::Explore => "explore",
            SubAgentRole::Plan => "plan",
            SubAgentRole::Review => "review",
            SubAgentRole::Implementer => "implementer",
            SubAgentRole::Verifier => "verifier",
            SubAgentRole::Custom => "custom",
        }
    }
}

impl fmt::Display for SubAgentRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Parse a sub-agent role from a string (case-insensitive, supports aliases)
pub fn parse_role(s: &str) -> Option<SubAgentRole> {
    let lower = s.to_lowercase();
    for role in &ALL_ROLES {
        for alias in role.aliases() {
            if lower == *alias {
                return Some(*role);
            }
        }
    }
    // Try matching display name directly
    match lower.as_str() {
        "general" | "coding" => Some(SubAgentRole::General),
        "explore" | "explorer" => Some(SubAgentRole::Explore),
        "plan" => Some(SubAgentRole::Plan),
        "review" | "code_review" => Some(SubAgentRole::Review),
        "implementer" | "implement" => Some(SubAgentRole::Implementer),
        "verifier" | "verify" | "tester" => Some(SubAgentRole::Verifier),
        "custom" => Some(SubAgentRole::Custom),
        _ => None,
    }
}

/// All roles in a static array
pub const ALL_ROLES: [SubAgentRole; 7] = [
    SubAgentRole::General,
    SubAgentRole::Explore,
    SubAgentRole::Plan,
    SubAgentRole::Review,
    SubAgentRole::Implementer,
    SubAgentRole::Verifier,
    SubAgentRole::Custom,
];

/// Output format for sub-agent results
pub const SUBAGENT_OUTPUT_FORMAT: &str = "\
When you finish your task, provide your results in this format:

SUMMARY:
    One paragraph describing what you did and what happened.

CHANGES:
    Files modified, with one-line descriptions. Use 'None.' if read-only.

EVIDENCE:
    Path:line:col citations and key findings. One bullet each.

RISKS:
    What could go wrong or what the parent should double-check.

BLOCKERS:
    What stopped you. Use 'None.' if you finished cleanly.
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_display_names() {
        assert_eq!(SubAgentRole::General.display_name(), "general");
        assert_eq!(SubAgentRole::Explore.display_name(), "explore");
        assert_eq!(SubAgentRole::Plan.display_name(), "plan");
        assert_eq!(SubAgentRole::Review.display_name(), "review");
        assert_eq!(SubAgentRole::Implementer.display_name(), "implementer");
        assert_eq!(SubAgentRole::Verifier.display_name(), "verifier");
        assert_eq!(SubAgentRole::Custom.display_name(), "custom");
    }

    #[test]
    fn test_system_prompts_not_empty() {
        for role in &ALL_ROLES {
            assert!(!role.system_prompt().is_empty(), "System prompt for {:?} should not be empty", role);
        }
    }

    #[test]
    fn test_read_only_roles() {
        assert!(SubAgentRole::Explore.is_read_only());
        assert!(SubAgentRole::Plan.is_read_only());
        assert!(SubAgentRole::Review.is_read_only());
        assert!(!SubAgentRole::General.is_read_only());
        assert!(!SubAgentRole::Implementer.is_read_only());
    }

    #[test]
    fn test_write_permissions() {
        assert!(SubAgentRole::General.can_write_files());
        assert!(SubAgentRole::Implementer.can_write_files());
        assert!(!SubAgentRole::Explore.can_write_files());
        assert!(!SubAgentRole::Review.can_write_files());
    }

    #[test]
    fn test_shell_permissions() {
        assert!(SubAgentRole::General.can_run_shell());
        assert!(SubAgentRole::Explore.can_run_shell());
        assert!(!SubAgentRole::Plan.can_run_shell());
        assert!(!SubAgentRole::Review.can_run_shell());
    }

    #[test]
    fn test_parse_role_by_name() {
        assert_eq!(parse_role("general"), Some(SubAgentRole::General));
        assert_eq!(parse_role("explore"), Some(SubAgentRole::Explore));
        assert_eq!(parse_role("plan"), Some(SubAgentRole::Plan));
        assert_eq!(parse_role("review"), Some(SubAgentRole::Review));
        assert_eq!(parse_role("implementer"), Some(SubAgentRole::Implementer));
        assert_eq!(parse_role("verifier"), Some(SubAgentRole::Verifier));
        assert_eq!(parse_role("custom"), Some(SubAgentRole::Custom));
    }

    #[test]
    fn test_parse_role_by_alias() {
        assert_eq!(parse_role("worker"), Some(SubAgentRole::General));
        assert_eq!(parse_role("explorer"), Some(SubAgentRole::Explore));
        assert_eq!(parse_role("reviewer"), Some(SubAgentRole::Review));
        assert_eq!(parse_role("tester"), Some(SubAgentRole::Verifier));
        assert_eq!(parse_role("builder"), Some(SubAgentRole::Implementer));
    }

    #[test]
    fn test_parse_role_case_insensitive() {
        assert_eq!(parse_role("GENERAL"), Some(SubAgentRole::General));
        assert_eq!(parse_role("Explore"), Some(SubAgentRole::Explore));
        assert_eq!(parse_role("PLAN"), Some(SubAgentRole::Plan));
    }

    #[test]
    fn test_parse_role_unknown() {
        assert_eq!(parse_role("unknown_role_xyz"), None);
    }

    #[test]
    fn test_aliases_contain_display_name() {
        for role in &ALL_ROLES {
            assert!(
                role.aliases().contains(&role.display_name()),
                "Aliases for {:?} should include '{}'",
                role,
                role.display_name()
            );
        }
    }

    #[test]
    fn test_output_format_not_empty() {
        assert!(!SUBAGENT_OUTPUT_FORMAT.is_empty());
        assert!(SUBAGENT_OUTPUT_FORMAT.contains("SUMMARY:"));
        assert!(SUBAGENT_OUTPUT_FORMAT.contains("EVIDENCE:"));
        assert!(SUBAGENT_OUTPUT_FORMAT.contains("BLOCKERS:"));
    }
}
