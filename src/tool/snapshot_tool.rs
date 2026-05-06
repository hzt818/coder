//! Snapshot and restore tools for workspace rollback
//!
//! Provides `/restore` and `revert_turn` functionality through
//! the side-git snapshot system. Allows rolling back workspace
//! changes without affecting the project's own `.git`.

use super::*;
use crate::core::snapshot::SnapshotManager;
use async_trait::async_trait;
use std::path::Path;

pub struct SnapshotTool;

#[async_trait]
impl Tool for SnapshotTool {
    fn name(&self) -> &str {
        "snapshot"
    }

    fn description(&self) -> &str {
        concat!(
            "Manage workspace snapshots and rollback. ",
            "Use 'list' to view snapshots, 'restore' to revert to a snapshot, ",
            "or 'diff' to see changes since a snapshot. ",
            "This uses side-git and does not affect the project's own git history."
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "restore", "diff"],
                    "description": "Snapshot action to perform"
                },
                "snapshot_id": {
                    "type": "string",
                    "description": "Snapshot ID (for restore/diff)",
                    "default": ""
                },
                "workspace": {
                    "type": "string",
                    "description": "Workspace path (default: current directory)",
                    "default": "."
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("");

        let workspace = args
            .get("workspace")
            .and_then(|w| w.as_str())
            .unwrap_or(".");

        let ws_path = Path::new(workspace);
        let manager = SnapshotManager::new(ws_path);

        match action {
            "list" => match manager.list_snapshots() {
                Ok(snapshots) => {
                    let output = crate::core::snapshot::format_snapshot_list(&snapshots);
                    ToolResult::ok(output)
                }
                Err(e) => ToolResult::err(format!("Failed to list snapshots: {}", e)),
            },
            "restore" => {
                let snapshot_id = args
                    .get("snapshot_id")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");

                if snapshot_id.is_empty() {
                    return ToolResult::err("snapshot_id is required for 'restore' action");
                }

                match manager.restore(snapshot_id) {
                    Ok(_) => ToolResult::ok(format!(
                        "Workspace restored to snapshot '{}'. Files have been reverted.",
                        snapshot_id
                    )),
                    Err(e) => ToolResult::err(format!("Failed to restore snapshot: {}", e)),
                }
            }
            "diff" => {
                let snapshot_id = args
                    .get("snapshot_id")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");

                if snapshot_id.is_empty() {
                    return ToolResult::err("snapshot_id is required for 'diff' action");
                }

                match manager.diff_snapshot(snapshot_id) {
                    Ok(diff) => ToolResult::ok(format!(
                        "Changes since snapshot '{}':\n{}",
                        snapshot_id, diff
                    )),
                    Err(e) => ToolResult::err(format!("Failed to diff snapshot: {}", e)),
                }
            }
            _ => ToolResult::err(format!(
                "Unknown action: '{}'. Valid actions: list, restore, diff",
                action
            )),
        }
    }

    fn requires_permission(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_tool_name() {
        let tool = SnapshotTool;
        assert_eq!(tool.name(), "snapshot");
    }

    #[test]
    fn test_snapshot_tool_schema() {
        let tool = SnapshotTool;
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }

    #[tokio::test]
    async fn test_snapshot_tool_empty_action() {
        let tool = SnapshotTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_snapshot_tool_list() {
        let tool = SnapshotTool;
        let result = tool.execute(serde_json::json!({"action": "list"})).await;
        // Should succeed even with no snapshots
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_snapshot_tool_restore_no_id() {
        let tool = SnapshotTool;
        let result = tool.execute(serde_json::json!({"action": "restore"})).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_snapshot_tool_diff_no_id() {
        let tool = SnapshotTool;
        let result = tool.execute(serde_json::json!({"action": "diff"})).await;
        assert!(!result.success);
    }
}
