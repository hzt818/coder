//! Automation tool - managed cron scheduling for automations
//!
//! Provides create/list/read/update/pause/resume/delete/run operations
//! for scheduling and managing automated prompts on a cron schedule.

use super::*;
use crate::core::automation::{AutomationManager, AutomationStatus};
use async_trait::async_trait;
use std::sync::Mutex;

static AUTO_MGR: Mutex<Option<AutomationManager>> = Mutex::new(None);

fn get_manager() -> std::sync::MutexGuard<'static, Option<AutomationManager>> {
    let mut guard = AUTO_MGR.lock().unwrap();
    if guard.is_none() {
        *guard = Some(AutomationManager::new());
    }
    guard
}

pub struct AutomationTool;

#[async_trait]
impl Tool for AutomationTool {
    fn name(&self) -> &str {
        "automation"
    }

    fn description(&self) -> &str {
        concat!(
            "Manage scheduled automations on a cron schedule. ",
            "Use 'create' to register a new automation with a name, cron schedule, and prompt. ",
            "'list' shows all automations, 'read' shows details, ",
            "'update' changes configuration, 'pause'/'resume' control lifecycle, ",
            "'delete' removes an automation, and 'run' executes immediately."
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["create", "list", "read", "update", "pause", "resume", "delete", "run"],
                    "description": "Operation to perform"
                },
                "name": {
                    "type": "string",
                    "description": "Automation name (for create/update)",
                    "default": ""
                },
                "schedule": {
                    "type": "string",
                    "description": "Cron expression for scheduling (for create/update)",
                    "default": ""
                },
                "prompt": {
                    "type": "string",
                    "description": "Prompt text to execute (for create/update)",
                    "default": ""
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory for execution",
                    "default": ""
                },
                "id": {
                    "type": "string",
                    "description": "Automation ID or name (for read/update/pause/resume/delete/run)",
                    "default": ""
                },
                "status": {
                    "type": "string",
                    "enum": ["active", "paused"],
                    "description": "Automation status (for update)",
                    "default": "active"
                }
            },
            "required": ["operation"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let op = args.get("operation").and_then(|o| o.as_str()).unwrap_or("");

        let mut guard = get_manager();
        let mgr = guard.as_mut().unwrap();

        match op {
            "create" => {
                let name = args.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let schedule = args.get("schedule").and_then(|s| s.as_str()).unwrap_or("");
                let prompt = args.get("prompt").and_then(|p| p.as_str()).unwrap_or("");

                if name.is_empty() {
                    return ToolResult::err("'name' is required for create");
                }
                if schedule.is_empty() {
                    return ToolResult::err("'schedule' is required for create");
                }
                if prompt.is_empty() {
                    return ToolResult::err("'prompt' is required for create");
                }

                let auto = mgr.create(name, schedule, prompt);

                ToolResult::ok(format!(
                    "Created automation: {} (id: {}) schedule: {}",
                    auto.name, auto.id, auto.schedule
                ))
            }
            "list" => {
                ToolResult::ok(mgr.format_list())
            }
            "read" => {
                let id = args.get("id").and_then(|i| i.as_str()).unwrap_or("");
                if id.is_empty() {
                    return ToolResult::err("'id' is required for read");
                }
                match mgr.get(id) {
                    Some(a) => {
                        let last = a.last_run.as_deref().unwrap_or("never");
                        ToolResult::ok(format!(
                            "Automation: {} (id: {})\n  Schedule: {}\n  Prompt: {}\n  Status: {}\n  CWD: {}\n  Created: {}\n  Last Run: {}",
                            a.name, a.id, a.schedule, a.prompt, a.status, a.cwd, a.created_at, last
                        ))
                    }
                    None => ToolResult::err(format!("Automation '{}' not found", id)),
                }
            }
            "update" => {
                let id = args.get("id").and_then(|i| i.as_str()).unwrap_or("");
                if id.is_empty() {
                    return ToolResult::err("'id' is required for update");
                }

                let name = args.get("name").and_then(|n| n.as_str()).filter(|n| !n.is_empty());
                let schedule = args.get("schedule").and_then(|s| s.as_str()).filter(|s| !s.is_empty());
                let prompt = args.get("prompt").and_then(|p| p.as_str()).filter(|p| !p.is_empty());
                let cwd = args.get("cwd").and_then(|c| c.as_str()).filter(|c| !c.is_empty());
                let status = args.get("status").and_then(|s| s.as_str()).filter(|s| !s.is_empty());

                if name.or(schedule).or(prompt).or(cwd).or(status).is_none() {
                    return ToolResult::err("No fields to update. Provide at least one of: name, schedule, prompt, cwd, status");
                }

                if mgr.update(id, name, schedule, prompt, cwd, status) {
                    ToolResult::ok(format!("Automation '{}' updated", id))
                } else {
                    ToolResult::err(format!("Automation '{}' not found", id))
                }
            }
            "pause" => {
                let id = args.get("id").and_then(|i| i.as_str()).unwrap_or("");
                if id.is_empty() {
                    return ToolResult::err("'id' is required for pause");
                }
                if mgr.set_status(id, AutomationStatus::Paused) {
                    ToolResult::ok(format!("Automation '{}' paused", id))
                } else {
                    ToolResult::err(format!("Automation '{}' not found", id))
                }
            }
            "resume" => {
                let id = args.get("id").and_then(|i| i.as_str()).unwrap_or("");
                if id.is_empty() {
                    return ToolResult::err("'id' is required for resume");
                }
                if mgr.set_status(id, AutomationStatus::Active) {
                    ToolResult::ok(format!("Automation '{}' resumed", id))
                } else {
                    ToolResult::err(format!("Automation '{}' not found", id))
                }
            }
            "delete" => {
                let id = args.get("id").and_then(|i| i.as_str()).unwrap_or("");
                if id.is_empty() {
                    return ToolResult::err("'id' is required for delete");
                }
                if mgr.delete(id) {
                    ToolResult::ok(format!("Automation '{}' deleted", id))
                } else {
                    ToolResult::err(format!("Automation '{}' not found", id))
                }
            }
            "run" => {
                let id = args.get("id").and_then(|i| i.as_str()).unwrap_or("");
                if id.is_empty() {
                    return ToolResult::err("'id' is required for run");
                }
                match mgr.run_now(id) {
                    Some(output) => ToolResult::ok(output),
                    None => ToolResult::err(format!("Automation '{}' not found", id)),
                }
            }
            _ => ToolResult::err(format!(
                "Unknown operation: '{}'. Valid operations: create, list, read, update, pause, resume, delete, run",
                op
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
    fn test_automation_tool_name() {
        let tool = AutomationTool;
        assert_eq!(tool.name(), "automation");
    }

    #[test]
    fn test_automation_tool_schema() {
        let tool = AutomationTool;
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
        let props = schema.get("properties").unwrap();
        assert!(props.get("operation").is_some());
    }

    #[tokio::test]
    async fn test_automation_create() {
        let tool = AutomationTool;
        let result = tool
            .execute(serde_json::json!({
                "operation": "create",
                "name": "daily-report",
                "schedule": "0 9 * * *",
                "prompt": "Generate daily report"
            }))
            .await;
        assert!(result.success);
        assert!(result.output.contains("daily-report"));
        assert!(result.output.contains("0 9 * * *"));
    }

    #[tokio::test]
    async fn test_automation_create_missing_fields() {
        let tool = AutomationTool;
        // Missing name
        let result = tool
            .execute(serde_json::json!({
                "operation": "create",
                "schedule": "0 9 * * *",
                "prompt": "test"
            }))
            .await;
        assert!(!result.success);

        // Missing schedule
        let result = tool
            .execute(serde_json::json!({
                "operation": "create",
                "name": "test",
                "prompt": "test"
            }))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_automation_list() {
        let tool = AutomationTool;
        let result = tool.execute(serde_json::json!({"operation": "list"})).await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_automation_read() {
        let tool = AutomationTool;
        // First create
        let _ = tool
            .execute(serde_json::json!({
                "operation": "create", "name": "read-test",
                "schedule": "0 9 * * *", "prompt": "test"
            }))
            .await;
        let result = tool
            .execute(serde_json::json!({"operation": "read", "id": "read-test"}))
            .await;
        assert!(result.success);
        assert!(result.output.contains("read-test"));
    }

    #[tokio::test]
    async fn test_automation_read_not_found() {
        let tool = AutomationTool;
        let result = tool
            .execute(serde_json::json!({"operation": "read", "id": "nonexistent"}))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_automation_update() {
        let tool = AutomationTool;
        // First create
        let _ = tool
            .execute(serde_json::json!({
                "operation": "create", "name": "update-test",
                "schedule": "0 9 * * *", "prompt": "test"
            }))
            .await;
        let result = tool
            .execute(serde_json::json!({
                "operation": "update",
                "id": "update-test",
                "schedule": "0 10 * * *"
            }))
            .await;
        assert!(result.success);
        assert!(result.output.contains("updated"));
    }

    #[tokio::test]
    async fn test_automation_pause_resume_delete() {
        let tool = AutomationTool;
        // Create first
        let _ = tool
            .execute(serde_json::json!({
                "operation": "create", "name": "prd-test",
                "schedule": "0 9 * * *", "prompt": "test"
            }))
            .await;

        // Pause
        let result = tool
            .execute(serde_json::json!({"operation": "pause", "id": "prd-test"}))
            .await;
        assert!(result.success);
        assert!(result.output.contains("paused"));

        // Resume
        let result = tool
            .execute(serde_json::json!({"operation": "resume", "id": "prd-test"}))
            .await;
        assert!(result.success);
        assert!(result.output.contains("resumed"));

        // Delete
        let result = tool
            .execute(serde_json::json!({"operation": "delete", "id": "prd-test"}))
            .await;
        assert!(result.success);
        assert!(result.output.contains("deleted"));
    }

    #[tokio::test]
    async fn test_automation_run() {
        let tool = AutomationTool;
        // Create before running
        let _ = tool
            .execute(serde_json::json!({
                "operation": "create",
                "name": "run-test",
                "schedule": "0 0 * * *",
                "prompt": "Execute now"
            }))
            .await;

        let result = tool
            .execute(serde_json::json!({"operation": "run", "id": "run-test"}))
            .await;
        assert!(result.success);
        assert!(result.output.contains("Execute now"));
    }

    #[tokio::test]
    async fn test_automation_invalid_operation() {
        let tool = AutomationTool;
        let result = tool
            .execute(serde_json::json!({"operation": "bogus"}))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_automation_empty_args() {
        let tool = AutomationTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.success);
    }
}
