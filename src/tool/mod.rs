//! Tool system - all tools implement the Tool trait
//!
//! Tools are registered in a ToolRegistry and exposed to the AI
//! via JSON Schema tool definitions.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod registry;
pub mod bash;
pub mod file_read;
pub mod file_write;
pub mod file_edit;
pub mod glob;
pub mod grep;
pub mod question;

// Phase 1+ tools (always compiled)
#[cfg(feature = "tools-git")]
pub mod git;
pub mod web_fetch;
pub mod web_search;
pub mod docs;
pub mod task;
pub mod plan;
pub mod apply_patch;
pub mod fim_edit;
pub mod list_dir;
pub mod checklist;
pub mod rlm;
pub mod task_gate;
pub mod automation_tool;
pub mod pr_attempt;
pub mod snapshot_tool;
pub mod github;
pub mod ci;

// Phase 2 tools (feature-gated)
#[cfg(feature = "tools-docker")]
pub mod docker;
#[cfg(feature = "tools-db")]
pub mod db_query;
#[cfg(feature = "tools-oauth")]
pub mod oauth;
#[cfg(feature = "tools-git")]
pub mod worktree;

pub use registry::ToolRegistry;

/// Result of executing a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
    /// Whether the output was truncated by capacity control
    #[serde(default)]
    pub truncated: bool,
    /// Estimated token count of the output
    #[serde(default)]
    pub estimated_tokens: usize,
    /// Original output size before potential truncation
    #[serde(default)]
    pub original_size: usize,
}

impl ToolResult {
    pub fn ok(output: impl Into<String>) -> Self {
        let output = output.into();
        let estimated_tokens = crate::core::pricing::estimate_tokens(&output);
        let output_len = output.len();
        Self {
            success: true,
            output,
            error: None,
            metadata: None,
            truncated: false,
            estimated_tokens,
            original_size: output_len,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
            metadata: None,
            truncated: false,
            estimated_tokens: 0,
            original_size: 0,
        }
    }

    /// Check if this result is "large" for capacity routing
    pub fn is_large(&self, threshold_tokens: usize) -> bool {
        self.estimated_tokens > threshold_tokens
    }

    /// Apply capacity routing to truncate output if needed
    pub fn apply_capacity(mut self, tool_name: &str, config: &crate::core::capacity::CapacityConfig) -> Self {
        let check = crate::core::capacity::check_capacity(tool_name, &self.output, config);
        if check.truncated {
            self.original_size = self.output.len();
            self.output = crate::core::capacity::truncate_output(
                &self.output,
                config.max_output_bytes,
                config.max_output_tokens,
                tool_name,
            );
            self.truncated = true;
            self.estimated_tokens = check.estimated_tokens;
        }
        self
    }
}

/// Tool trait - all tools must implement this
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (used by AI to identify the tool)
    fn name(&self) -> &str;

    /// Description for the AI (helps it decide when to use this tool)
    fn description(&self) -> &str;

    /// JSON Schema for the tool's input parameters
    fn schema(&self) -> serde_json::Value;

    /// Execute the tool with the given arguments
    async fn execute(&self, args: serde_json::Value) -> ToolResult;

    /// Whether this tool requires user confirmation before execution
    fn requires_permission(&self) -> bool {
        false
    }
}

/// Allow `?` operator in functions returning `ToolResult` when error is a `String`.
impl From<String> for ToolResult {
    fn from(msg: String) -> Self {
        ToolResult::err(msg)
    }
}

/// Shared tool type
pub type SharedTool = Arc<dyn Tool>;
