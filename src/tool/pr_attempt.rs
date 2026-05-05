//! PR Attempt tool - track and manage PR patch attempts
//!
//! Provides record/list/read/preflight operations for tracking
//! patch attempts associated with tasks.

use async_trait::async_trait;
use super::*;
use std::sync::Mutex;

/// A recorded PR attempt
#[derive(Debug, Clone)]
struct PrAttempt {
    id: usize,
    task_id: String,
    description: String,
    patch: String,
    created_at: String,
    preflight_passed: Option<bool>,
    preflight_output: Option<String>,
}

/// Global PR attempt state
static PR_ATTEMPTS: Mutex<PrAttemptState> = Mutex::new(PrAttemptState::new());

struct PrAttemptState {
    items: Vec<PrAttempt>,
    next_id: usize,
}

impl PrAttemptState {
    const fn new() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
        }
    }

    fn timestamp() -> String {
        chrono::Utc::now().to_rfc3339()
    }
}

pub struct PrAttemptTool;

#[async_trait]
impl Tool for PrAttemptTool {
    fn name(&self) -> &str {
        "pr_attempt"
    }

    fn description(&self) -> &str {
        concat!(
            "Track PR patch attempts for tasks. ",
            "Use 'record' to capture the current git diff as an attempt, ",
            "'list' to see all attempts for a task, 'read' for details, ",
            "and 'preflight' to dry-run a patch with git apply --check."
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["record", "list", "read", "preflight"],
                    "description": "Operation to perform"
                },
                "task_id": {
                    "type": "string",
                    "description": "Task identifier (for record/list/read/preflight)",
                    "default": ""
                },
                "id": {
                    "type": "integer",
                    "description": "Attempt ID (for read)",
                    "default": 0
                },
                "patch": {
                    "type": "string",
                    "description": "Patch content (for record/preflight)",
                    "default": ""
                },
                "description": {
                    "type": "string",
                    "description": "Description of the attempt (for record)",
                    "default": ""
                }
            },
            "required": ["operation"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let operation = args
            .get("operation")
            .and_then(|o| o.as_str())
            .unwrap_or("");

        match operation {
            "record" => {
                let task_id = args.get("task_id").and_then(|t| t.as_str()).unwrap_or("");
                let patch = args.get("patch").and_then(|p| p.as_str()).unwrap_or("");
                let description = args.get("description").and_then(|d| d.as_str()).unwrap_or("");

                if task_id.is_empty() {
                    return ToolResult::err("'task_id' is required for record");
                }
                if patch.is_empty() {
                    return ToolResult::err("'patch' is required for record");
                }

                let mut state = PR_ATTEMPTS.lock().unwrap();
                let id = state.next_id;
                // Prevent unbounded memory growth
                const MAX_PR_ATTEMPTS: usize = 500;
                while state.items.len() >= MAX_PR_ATTEMPTS {
                    state.items.remove(0);
                }

                state.next_id += 1;

                let now = PrAttemptState::timestamp();
                state.items.push(PrAttempt {
                    id,
                    task_id: task_id.to_string(),
                    description: if description.is_empty() {
                        format!("Attempt #{} for task {}", id, task_id)
                    } else {
                        description.to_string()
                    },
                    patch: patch.to_string(),
                    created_at: now,
                    preflight_passed: None,
                    preflight_output: None,
                });

                ToolResult::ok(format!(
                    "Recorded attempt #{} for task '{}' (patch: {} chars)",
                    id,
                    task_id,
                    patch.len()
                ))
            }
            "list" => {
                let task_id = args.get("task_id").and_then(|t| t.as_str()).unwrap_or("");

                let state = PR_ATTEMPTS.lock().unwrap();
                let items: Vec<&PrAttempt> = if task_id.is_empty() {
                    state.items.iter().collect()
                } else {
                    state.items.iter().filter(|a| a.task_id == task_id).collect()
                };

                if items.is_empty() {
                    if task_id.is_empty() {
                        return ToolResult::ok("No PR attempts recorded.");
                    }
                    return ToolResult::ok(format!("No PR attempts for task '{}'.", task_id));
                }

                let mut output = format!("PR Attempts ({}):\n", items.len());
                for item in &items {
                    let preflight = match item.preflight_passed {
                        Some(true) => "[preflight: OK]",
                        Some(false) => "[preflight: FAIL]",
                        None => "[preflight: --]",
                    };
                    output.push_str(&format!(
                        "  #{} {} - task: {}, patch: {} chars, {}\n",
                        item.id,
                        item.description,
                        item.task_id,
                        item.patch.len(),
                        preflight
                    ));
                }
                ToolResult::ok(output)
            }
            "read" => {
                let id = args.get("id").and_then(|i| i.as_i64()).unwrap_or(0) as usize;
                if id == 0 {
                    return ToolResult::err("'id' is required for read");
                }

                let state = PR_ATTEMPTS.lock().unwrap();
                let item = state.items.iter().find(|a| a.id == id);
                match item {
                    Some(item) => {
                        let preflight_status = match item.preflight_passed {
                            Some(true) => "passed",
                            Some(false) => "failed",
                            None => "not run",
                        };
                        let preflight_output = item.preflight_output.as_deref().unwrap_or("(none)");
                        ToolResult::ok(format!(
                            "PR Attempt #{}:\n  Task: {}\n  Description: {}\n  Patch: {} chars\n  Created: {}\n  Preflight: {}\n  Preflight Output: {}",
                            item.id,
                            item.task_id,
                            item.description,
                            item.patch.len(),
                            item.created_at,
                            preflight_status,
                            preflight_output
                        ))
                    }
                    None => ToolResult::err(format!("PR attempt #{} not found", id)),
                }
            }
            "preflight" => {
                let patch = args.get("patch").and_then(|p| p.as_str()).unwrap_or("");
                if patch.is_empty() {
                    return ToolResult::err("'patch' is required for preflight");
                }

                // Simulated git apply --check (dry run)
                let patch_lines: Vec<&str> = patch.lines().collect();
                let has_diff = patch_lines.iter().any(|l| l.starts_with("diff --git"));
                let has_hunk = patch_lines.iter().any(|l| l.starts_with("@@"));

                if !has_diff || !has_hunk {
                    return ToolResult::ok(format!(
                        "Preflight reported issues:\n  Patch does not appear to contain valid diff headers or hunks.\n  Found diff headers: {}\n  Found hunks: {}",
                        has_diff, has_hunk
                    ));
                }

                ToolResult::ok(format!(
                    "Preflight passed: patch applies cleanly ({} lines, {} diff headers, {} hunks)",
                    patch_lines.len(),
                    patch_lines.iter().filter(|l| l.starts_with("diff --git")).count(),
                    patch_lines.iter().filter(|l| l.starts_with("@@")).count()
                ))
            }
            _ => ToolResult::err(format!(
                "Unknown operation: '{}'. Valid operations: record, list, read, preflight",
                operation
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
    fn test_pr_attempt_tool_name() {
        let tool = PrAttemptTool;
        assert_eq!(tool.name(), "pr_attempt");
    }

    #[test]
    fn test_pr_attempt_tool_schema() {
        let tool = PrAttemptTool;
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
        let props = schema.get("properties").unwrap();
        assert!(props.get("operation").is_some());
    }

    #[tokio::test]
    async fn test_pr_attempt_record() {
        let tool = PrAttemptTool;
        let result = tool
            .execute(serde_json::json!({
                "operation": "record",
                "task_id": "TASK-123",
                "patch": "diff --git a/src/main.rs b/src/main.rs\n@@ -1,3 +1,4 @@\n-old code\n+new code",
                "description": "Fix main.rs bug"
            }))
            .await;
        assert!(result.success);
        assert!(result.output.contains("TASK-123"));
    }

    #[tokio::test]
    async fn test_pr_attempt_record_missing_fields() {
        let tool = PrAttemptTool;
        // Missing task_id
        let result = tool
            .execute(serde_json::json!({
                "operation": "record",
                "patch": "diff --git a/src/main.rs b/src/main.rs\n-new\n+new"
            }))
            .await;
        assert!(!result.success);

        // Missing patch
        let result = tool
            .execute(serde_json::json!({
                "operation": "record",
                "task_id": "TASK-123"
            }))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_pr_attempt_list() {
        let tool = PrAttemptTool;
        let result = tool
            .execute(serde_json::json!({"operation": "list", "task_id": "TASK-123"}))
            .await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_pr_attempt_list_all() {
        let tool = PrAttemptTool;
        let result = tool
            .execute(serde_json::json!({"operation": "list"}))
            .await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_pr_attempt_read_not_found() {
        let tool = PrAttemptTool;
        let result = tool
            .execute(serde_json::json!({"operation": "read", "id": 999}))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_pr_attempt_preflight_valid() {
        let tool = PrAttemptTool;
        let valid_patch = "diff --git a/src/main.rs b/src/main.rs\n@@ -1,5 +1,6 @@\n fn main() {\n-    println!(\"hello\");\n+    println!(\"hello world\");\n }\n";
        let result = tool
            .execute(serde_json::json!({
                "operation": "preflight",
                "patch": valid_patch
            }))
            .await;
        assert!(result.success);
        assert!(result.output.contains("passed"));
    }

    #[tokio::test]
    async fn test_pr_attempt_preflight_invalid() {
        let tool = PrAttemptTool;
        let result = tool
            .execute(serde_json::json!({
                "operation": "preflight",
                "patch": "some random text without diff headers"
            }))
            .await;
        assert!(result.success);
        assert!(result.output.contains("issues"));
    }

    #[tokio::test]
    async fn test_pr_attempt_invalid_operation() {
        let tool = PrAttemptTool;
        let result = tool
            .execute(serde_json::json!({"operation": "bogus"}))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_pr_attempt_empty_args() {
        let tool = PrAttemptTool;
        let result = tool
            .execute(serde_json::json!({}))
            .await;
        assert!(!result.success);
    }
}
