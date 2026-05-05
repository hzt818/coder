//! RemoteTrigger tool — manage remote agent triggers.
//!
//! Allows creating, listing, updating, running, and deleting remote
//! triggers via API. Ported from cc's RemoteTriggerTool pattern.

use async_trait::async_trait;
use super::*;

pub struct RemoteTriggerTool;

#[async_trait]
impl Tool for RemoteTriggerTool {
    fn name(&self) -> &str { "remote_trigger" }
    fn description(&self) -> &str {
        "Manage remote agent triggers: create, list, get, update, run. Triggers execute a prompt remotely on a schedule."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "action": {
                    "type": "string", "enum": ["create", "list", "get", "update", "run", "delete"],
                    "description": "Trigger action"
                },
                "trigger_id": { "type": "string", "description": "Trigger ID (for get/update/run)" },
                "prompt": { "type": "string", "description": "Prompt to execute (for create/update)" },
                "schedule": { "type": "string", "description": "Cron schedule (for create)" }
            }, "required": ["action"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("");
        match action {
            "list" => {
                let triggers = crate::core::automation::with_automation(|mgr| mgr.format_list())
                    .unwrap_or_else(|| "No automations.".to_string());
                ToolResult::ok(format!("── Remote Triggers ──\n\n{}", triggers))
            }
            "create" => {
                let prompt = args.get("prompt").and_then(|p| p.as_str()).unwrap_or("");
                let schedule = args.get("schedule").and_then(|s| s.as_str()).unwrap_or("");
                if prompt.is_empty() || schedule.is_empty() {
                    return ToolResult::err("prompt and schedule are required");
                }
                let name = format!("trigger-{}", chrono::Utc::now().timestamp());
                match crate::core::automation::with_automation(|mgr| mgr.create(&name, schedule, prompt)) {
                    Some(auto) => ToolResult::ok(format!("Created trigger: {} (id: {})", auto.name, auto.id)),
                    None => ToolResult::err("Automation manager not initialized"),
                }
            }
            "run" => {
                let id = args.get("trigger_id").and_then(|t| t.as_str()).unwrap_or("");
                if id.is_empty() { return ToolResult::err("trigger_id is required"); }
                match crate::core::automation::with_automation(|mgr| mgr.run_now(id)).flatten() {
                    Some(msg) => ToolResult::ok(format!("Trigger executed: {}", msg)),
                    None => ToolResult::err(format!("Trigger '{}' not found or manager not initialized", id)),
                }
            }
            "delete" => {
                let id = args.get("trigger_id").and_then(|t| t.as_str()).unwrap_or("");
                if id.is_empty() { return ToolResult::err("trigger_id is required"); }
                match crate::core::automation::with_automation(|mgr| mgr.delete(id)) {
                    Some(true) => ToolResult::ok(format!("Deleted trigger '{}'", id)),
                    _ => ToolResult::err(format!("Trigger '{}' not found", id)),
                }
            }
            _ => ToolResult::err(format!("Unknown action: {}", action)),
        }
    }
    fn requires_permission(&self) -> bool { true }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_name() { assert_eq!(RemoteTriggerTool.name(), "remote_trigger"); }
    #[tokio::test] async fn test_empty_action() { assert!(!RemoteTriggerTool.execute(serde_json::json!({})).await.success); }
}
