//! Cron scheduling tools — create, update, list, and delete recurring tasks.
//!
//! Uses the existing AutomationManager from `core::automation` for persistence,
//! and exposes 4 tools: `cron_create`, `cron_update`, `cron_delete`, `cron_list`.

use super::*;
use crate::core::automation;
use async_trait::async_trait;

pub struct CronCreate;
pub struct CronUpdate;
pub struct CronDelete;
pub struct CronList;

#[async_trait]
impl Tool for CronCreate {
    fn name(&self) -> &str {
        "cron_create"
    }
    fn description(&self) -> &str {
        "Create a recurring scheduled automation. The schedule uses cron format: 'minute hour day-of-month month day-of-week'."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "name": { "type": "string", "description": "Unique name for this automation" },
                "schedule": { "type": "string", "description": "Cron expression (e.g., '0 9 * * *' for daily 9am)" },
                "prompt": { "type": "string", "description": "Prompt to execute on schedule" }
            }, "required": ["name", "schedule", "prompt"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let name = args.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let schedule = args.get("schedule").and_then(|s| s.as_str()).unwrap_or("");
        let prompt = args.get("prompt").and_then(|p| p.as_str()).unwrap_or("");
        if name.is_empty() || schedule.is_empty() || prompt.is_empty() {
            return ToolResult::err("name, schedule, and prompt are required");
        }
        match automation::with_automation(|mgr| mgr.create(name, schedule, prompt)) {
            Some(auto) => ToolResult::ok(format!(
                "Created automation '{}' (id: {}) schedule: {}\n{}",
                auto.name, auto.id, auto.schedule, auto.prompt
            )),
            None => ToolResult::err("Automation manager not initialized"),
        }
    }
    fn requires_permission(&self) -> bool {
        true
    }
}

#[async_trait]
impl Tool for CronUpdate {
    fn name(&self) -> &str {
        "cron_update"
    }
    fn description(&self) -> &str {
        "Update an existing scheduled automation."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "id": { "type": "string", "description": "Automation ID or name" },
                "schedule": { "type": "string", "description": "New cron expression" },
                "prompt": { "type": "string", "description": "New prompt" },
                "status": { "type": "string", "description": "New status: active, paused", "default": "active" }
            }, "required": ["id"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let id = args.get("id").and_then(|i| i.as_str()).unwrap_or("");
        if id.is_empty() {
            return ToolResult::err("id is required");
        }
        let schedule = args.get("schedule").and_then(|s| s.as_str());
        let prompt = args.get("prompt").and_then(|p| p.as_str());
        let status = args.get("status").and_then(|s| s.as_str());
        match automation::with_automation(|mgr| {
            mgr.update(id, None, schedule, prompt, None, status)
        }) {
            Some(true) => ToolResult::ok(format!("Updated automation '{}'", id)),
            Some(false) => ToolResult::err(format!("Automation '{}' not found", id)),
            None => ToolResult::err("Automation manager not initialized"),
        }
    }
    fn requires_permission(&self) -> bool {
        true
    }
}

#[async_trait]
impl Tool for CronDelete {
    fn name(&self) -> &str {
        "cron_delete"
    }
    fn description(&self) -> &str {
        "Delete a scheduled automation."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "id": { "type": "string", "description": "Automation ID or name" }
            }, "required": ["id"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let id = args.get("id").and_then(|i| i.as_str()).unwrap_or("");
        if id.is_empty() {
            return ToolResult::err("id is required");
        }
        match automation::with_automation(|mgr| mgr.delete(id)) {
            Some(true) => ToolResult::ok(format!("Deleted automation '{}'", id)),
            Some(false) => ToolResult::err(format!("Automation '{}' not found", id)),
            None => ToolResult::err("Automation manager not initialized"),
        }
    }
    fn requires_permission(&self) -> bool {
        true
    }
}

#[async_trait]
impl Tool for CronList {
    fn name(&self) -> &str {
        "cron_list"
    }
    fn description(&self) -> &str {
        "List all scheduled automations."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {}, "additionalProperties": false
        })
    }
    async fn execute(&self, _args: serde_json::Value) -> ToolResult {
        let output = automation::with_automation(|mgr| mgr.format_list()).unwrap_or_else(|| {
            "── Automations ──\n\nAutomation manager not initialized.".to_string()
        });
        ToolResult::ok(output)
    }
    fn requires_permission(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::automation::init_automation_manager;

    fn ensure_init() {
        init_automation_manager();
    }

    #[tokio::test]
    async fn test_cron_create() {
        ensure_init();
        let tool = CronCreate;
        let r = tool
            .execute(serde_json::json!({
                "name": "test-job", "schedule": "0 9 * * *", "prompt": "Run daily report"
            }))
            .await;
        assert!(r.success, "{}", r.error.as_deref().unwrap_or(""));
        assert!(r.output.contains("test-job"));
    }

    #[tokio::test]
    async fn test_cron_list() {
        ensure_init();
        let tool = CronList;
        let r = tool.execute(serde_json::json!({})).await;
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_cron_delete() {
        ensure_init();
        let create = CronCreate;
        create
            .execute(serde_json::json!({
                "name": "del-me", "schedule": "* * * * *", "prompt": "test"
            }))
            .await;

        let del = CronDelete;
        let r = del.execute(serde_json::json!({"id": "del-me"})).await;
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_cron_create_empty() {
        let tool = CronCreate;
        assert!(!tool.execute(serde_json::json!({})).await.success);
    }
}
