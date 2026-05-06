//! Builtin skills included with coder
//!
//! Each builtin skill implements the Skill trait and provides
//! a reusable capability that can be invoked by name.

pub mod brainstorm;
pub mod code_review;
pub mod debug;
pub mod plan;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// A reusable capability that can be invoked with structured input.
///
/// Skills are similar to tools but are higher-level compositions
/// that may use multiple internal steps or AI calls.
#[async_trait]
pub trait Skill: Send + Sync {
    /// Name of the skill (used to identify it)
    fn name(&self) -> &str;

    /// Human-readable description of what this skill does
    fn description(&self) -> &str;

    /// JSON Schema for the skill's input parameters
    fn input_schema(&self) -> serde_json::Value;

    /// Execute the skill with the given input
    async fn execute(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value>;
}

/// Standard result structure for skill execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    /// Whether the skill executed successfully
    pub success: bool,
    /// Primary output text
    pub output: String,
    /// Optional structured data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl SkillOutput {
    /// Create a successful skill output
    pub fn ok(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            data: None,
            error: None,
        }
    }

    /// Create a successful skill output with structured data
    pub fn ok_with_data(output: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            output: output.into(),
            data: Some(data),
            error: None,
        }
    }

    /// Create a failed skill output
    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            data: None,
            error: Some(error.into()),
        }
    }

    /// Convert to a JSON value
    pub fn to_json(self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_output_ok() {
        let out = SkillOutput::ok("done");
        assert!(out.success);
        assert_eq!(out.output, "done");
        assert!(out.error.is_none());
    }

    #[test]
    fn test_skill_output_err() {
        let out = SkillOutput::err("something went wrong");
        assert!(!out.success);
        assert!(out.error.is_some());
    }

    #[test]
    fn test_skill_output_to_json() {
        let out = SkillOutput::ok("result");
        let json = out.to_json();
        assert_eq!(json["output"], "result");
        assert_eq!(json["success"], true);
    }
}
