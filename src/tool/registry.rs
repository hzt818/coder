//! Tool registry - manages tool registration and lookup

use super::*;
use crate::ai::ToolDef;
use std::collections::HashMap;
use std::sync::Arc;

use super::{docs, plan, task};

/// Registry for all available tools
pub struct ToolRegistry {
    tools: HashMap<String, SharedTool>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: SharedTool) -> &mut Self {
        self.tools.insert(tool.name().to_string(), tool);
        self
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&SharedTool> {
        self.tools.get(name)
    }

    /// Get all tool definitions for the AI
    pub fn tool_defs(&self) -> Vec<ToolDef> {
        self.tools
            .values()
            .map(|tool| ToolDef {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                input_schema: tool.schema(),
            })
            .collect()
    }

    /// Execute a tool by name with JSON arguments
    pub async fn execute(&self, name: &str, args: serde_json::Value) -> ToolResult {
        match self.tools.get(name) {
            Some(tool) => tool.execute(args).await,
            None => ToolResult::err(format!("Tool '{}' not found", name)),
        }
    }

    /// List all registered tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Number of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    /// Create a default registry with all core tools registered
    fn default() -> Self {
        let mut reg = Self::new();

        // Core tools
        reg.register(Arc::new(bash::BashTool::default()));
        reg.register(Arc::new(file_read::FileReadTool));
        reg.register(Arc::new(file_write::FileWriteTool));
        reg.register(Arc::new(file_edit::FileEditTool));
        reg.register(Arc::new(glob::GlobTool));
        reg.register(Arc::new(grep::GrepTool));
        reg.register(Arc::new(question::QuestionTool));
        reg.register(Arc::new(web_fetch::WebFetchTool));
        reg.register(Arc::new(web_search::WebSearchTool));

        // Conditional tools (feature-gated)
        #[cfg(feature = "tools-git")]
        reg.register(Arc::new(git::GitTool));
        #[cfg(feature = "tools-docker")]
        reg.register(Arc::new(docker::DockerTool));
        reg.register(Arc::new(ci::CiTool));
        #[cfg(feature = "tools-db")]
        reg.register(Arc::new(db_query::DbQueryTool));
        #[cfg(feature = "tools-oauth")]
        reg.register(Arc::new(oauth::OAuthTool));
        #[cfg(feature = "tools-git")]
        reg.register(Arc::new(worktree::WorktreeTool));

        // Phase 1+ tools
        reg.register(Arc::new(docs::DocsTool));
        reg.register(Arc::new(plan::PlanTool));
        reg.register(Arc::new(task::TaskTool));
        reg.register(Arc::new(apply_patch::ApplyPatchTool));
        reg.register(Arc::new(fim_edit::FimEditTool));
        reg.register(Arc::new(list_dir::ListDirTool));
        reg.register(Arc::new(checklist::ChecklistTool));
        reg.register(Arc::new(snapshot_tool::SnapshotTool));
        reg.register(Arc::new(github::GitHubTool));
        reg.register(Arc::new(rlm::RlmTool));
        reg.register(Arc::new(task_gate::TaskGateTool));
        reg.register(Arc::new(automation_tool::AutomationTool));
        reg.register(Arc::new(pr_attempt::PrAttemptTool));

        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_default() {
        let reg = ToolRegistry::default();
        assert!(reg.len() >= 13);
        assert!(reg.get("bash").is_some());
        assert!(reg.get("file_read").is_some());
        assert!(reg.get("apply_patch").is_some());
        assert!(reg.get("list_dir").is_some());
        assert!(reg.get("checklist").is_some());
        assert!(reg.get("snapshot").is_some());
    }

    #[test]
    fn test_tool_defs() {
        let reg = ToolRegistry::default();
        let defs = reg.tool_defs();
        assert!(!defs.is_empty());

        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"bash"));
    }
}
