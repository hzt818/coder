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

// Phase 1+ tools
#[cfg(feature = "tools-git")]
pub mod git;
pub mod web_fetch;
pub mod web_search;
pub mod docs;
pub mod task;
pub mod plan;

// Phase 2 tools
#[cfg(feature = "tools-docker")]
pub mod docker;
pub mod ci;
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
}

impl ToolResult {
    pub fn ok(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
            metadata: None,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
            metadata: None,
        }
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
