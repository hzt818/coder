//! Checklist tool - structured progress tracking for multi-step work
//!
//! Provides checklist_write/add/update/list tools that allow the AI
//! to track progress on complex multi-step tasks with granular status.

use super::*;
use async_trait::async_trait;
use std::sync::Mutex;

/// A single checklist item
#[derive(Debug, Clone)]
struct ChecklistItem {
    id: usize,
    description: String,
    status: ChecklistStatus,
    details: Option<String>,
}

/// Status of a checklist item
#[derive(Debug, Clone, PartialEq)]
enum ChecklistStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
    Blocked,
}

impl ChecklistStatus {
    fn as_str(&self) -> &'static str {
        match self {
            ChecklistStatus::Pending => "pending",
            ChecklistStatus::InProgress => "in_progress",
            ChecklistStatus::Completed => "completed",
            ChecklistStatus::Skipped => "skipped",
            ChecklistStatus::Blocked => "blocked",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "in_progress" => ChecklistStatus::InProgress,
            "completed" => ChecklistStatus::Completed,
            "skipped" => ChecklistStatus::Skipped,
            "blocked" => ChecklistStatus::Blocked,
            _ => ChecklistStatus::Pending,
        }
    }

    fn marker(&self) -> &'static str {
        match self {
            ChecklistStatus::Pending => "[ ]",
            ChecklistStatus::InProgress => "[-]",
            ChecklistStatus::Completed => "[x]",
            ChecklistStatus::Skipped => "[~]",
            ChecklistStatus::Blocked => "[!]",
        }
    }
}

/// Global checklist state (per-process, in-memory for now)
static CHECKLIST: Mutex<ChecklistState> = Mutex::new(ChecklistState::new());
/// Maximum checklist items to retain to prevent unbounded growth.
const MAX_CHECKLIST_ITEMS: usize = 1000;

struct ChecklistState {
    items: Vec<ChecklistItem>,
    goal: Option<String>,
    next_id: usize,
}

impl ChecklistState {
    const fn new() -> Self {
        Self {
            items: Vec::new(),
            goal: None,
            next_id: 1,
        }
    }
}

pub struct ChecklistTool;

#[async_trait]
impl Tool for ChecklistTool {
    fn name(&self) -> &str {
        "checklist"
    }

    fn description(&self) -> &str {
        concat!(
            "Manage a structured checklist for complex multi-step tasks. ",
            "Use 'write' to set a goal and create items, 'add' to append items, ",
            "'update' to change status, and 'list' to view current progress. ",
            "This helps track progress on complex work."
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["write", "add", "update", "list"],
                    "description": "Action to perform"
                },
                "goal": {
                    "type": "string",
                    "description": "Overall goal description (for 'write' action)"
                },
                "items": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "List of checklist items (for 'write' and 'add' actions)"
                },
                "id": {
                    "type": "integer",
                    "description": "Item ID (for 'update' action)"
                },
                "status": {
                    "type": "string",
                    "enum": ["pending", "in_progress", "completed", "skipped", "blocked"],
                    "description": "New status (for 'update' action)"
                },
                "details": {
                    "type": "string",
                    "description": "Optional details or notes for the update",
                    "default": ""
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("");

        match action {
            "write" => {
                let goal = args.get("goal").and_then(|g| g.as_str()).unwrap_or("");

                let items: Vec<String> = args
                    .get("items")
                    .and_then(|i| i.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                if goal.is_empty() && items.is_empty() {
                    return ToolResult::err("Either 'goal' or 'items' is required for 'write'");
                }

                let mut state = CHECKLIST.lock().unwrap();
                state.goal = if goal.is_empty() {
                    None
                } else {
                    Some(goal.to_string())
                };

                let start_id = state.next_id;
                for item_desc in &items {
                    let current_id = state.next_id;
                    state.items.push(ChecklistItem {
                        id: current_id,
                        description: item_desc.clone(),
                        status: ChecklistStatus::Pending,
                        details: None,
                    });
                    state.next_id = current_id + 1;
                }

                // Prevent unbounded memory growth
                while state.items.len() > MAX_CHECKLIST_ITEMS {
                    state.items.remove(0);
                }

                let mut output = String::new();
                if let Some(g) = &state.goal {
                    output.push_str(&format!("Goal: {}\n\n", g));
                }
                output.push_str(&format!("Created {} checklist item(s):\n", items.len()));
                for (i, item) in items.iter().enumerate() {
                    output.push_str(&format!("  #{} - {}\n", start_id + i, item));
                }

                ToolResult::ok(output)
            }
            "add" => {
                let items: Vec<String> = args
                    .get("items")
                    .and_then(|i| i.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                if items.is_empty() {
                    return ToolResult::err("'items' array is required for 'add'");
                }

                let mut state = CHECKLIST.lock().unwrap();
                let start_id = state.next_id;
                for item_desc in &items {
                    let current_id = state.next_id;
                    state.items.push(ChecklistItem {
                        id: current_id,
                        description: item_desc.clone(),
                        status: ChecklistStatus::Pending,
                        details: None,
                    });
                    state.next_id = current_id + 1;
                }

                // Prevent unbounded memory growth
                while state.items.len() > MAX_CHECKLIST_ITEMS {
                    state.items.remove(0);
                }

                let mut output = String::new();
                output.push_str(&format!("Added {} item(s):\n", items.len()));
                for (i, item) in items.iter().enumerate() {
                    output.push_str(&format!("  #{} - {}\n", start_id + i, item));
                }

                ToolResult::ok(output)
            }
            "update" => {
                let id = args.get("id").and_then(|i| i.as_i64()).unwrap_or(0) as usize;
                let status_str = args.get("status").and_then(|s| s.as_str()).unwrap_or("");

                if id == 0 {
                    return ToolResult::err("'id' is required for 'update'");
                }
                if status_str.is_empty() {
                    return ToolResult::err("'status' is required for 'update'");
                }

                let details = args.get("details").and_then(|d| d.as_str()).unwrap_or("");

                let mut state = CHECKLIST.lock().unwrap();
                let item = state.items.iter_mut().find(|item| item.id == id);

                match item {
                    Some(item) => {
                        let new_status = ChecklistStatus::from_str(status_str);
                        let old_status = item.status.as_str().to_string();
                        let new_status_str = new_status.as_str().to_string();
                        item.status = new_status;
                        if !details.is_empty() {
                            item.details = Some(details.to_string());
                        }
                        ToolResult::ok(format!(
                            "Item #{} ({}) status: {} → {}{}",
                            id,
                            item.description,
                            old_status,
                            new_status_str,
                            if details.is_empty() {
                                String::new()
                            } else {
                                format!("\nDetails: {}", details)
                            }
                        ))
                    }
                    None => ToolResult::err(format!("Item #{} not found", id)),
                }
            }
            "list" => {
                let state = CHECKLIST.lock().unwrap();

                let mut output = String::new();

                if let Some(goal) = &state.goal {
                    output.push_str(&format!("Goal: {}\n\n", goal));
                }

                if state.items.is_empty() {
                    output.push_str("(no checklist items)");
                    return ToolResult::ok(output);
                }

                let completed = state
                    .items
                    .iter()
                    .filter(|i| i.status == ChecklistStatus::Completed)
                    .count();
                let total = state.items.len();

                output.push_str(&format!(
                    "Progress: {}/{} items completed\n\n",
                    completed, total
                ));

                for item in &state.items {
                    let marker = item.status.marker();
                    output.push_str(&format!(
                        "  {} #{} - {}\n",
                        marker, item.id, item.description
                    ));
                    if let Some(details) = &item.details {
                        output.push_str(&format!("       Details: {}\n", details));
                    }
                }

                ToolResult::ok(output)
            }
            _ => ToolResult::err(format!(
                "Unknown action: '{}'. Valid actions: write, add, update, list",
                action
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checklist_tool_name() {
        let tool = ChecklistTool;
        assert_eq!(tool.name(), "checklist");
    }

    #[test]
    fn test_checklist_schema() {
        let tool = ChecklistTool;
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }

    #[tokio::test]
    async fn test_checklist_write() {
        let tool = ChecklistTool;
        let result = tool
            .execute(serde_json::json!({
                "action": "write",
                "goal": "Test goal",
                "items": ["Step 1", "Step 2"]
            }))
            .await;
        assert!(result.success);
        assert!(result.output.contains("Step 1"));
        assert!(result.output.contains("Step 2"));
    }

    #[tokio::test]
    async fn test_checklist_list() {
        let tool = ChecklistTool;
        let result = tool.execute(serde_json::json!({"action": "list"})).await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_checklist_update() {
        let tool = ChecklistTool;
        // First write some items
        let _ = tool
            .execute(serde_json::json!({
                "action": "write",
                "goal": "Test",
                "items": ["Item A"]
            }))
            .await;

        // Update item #1
        let result = tool
            .execute(serde_json::json!({
                "action": "update",
                "id": 1,
                "status": "completed"
            }))
            .await;
        assert!(result.success);
        assert!(result.output.contains("completed"));
    }

    #[tokio::test]
    async fn test_checklist_update_not_found() {
        let tool = ChecklistTool;
        let result = tool
            .execute(serde_json::json!({
                "action": "update",
                "id": 999,
                "status": "completed"
            }))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_checklist_invalid_action() {
        let tool = ChecklistTool;
        let result = tool.execute(serde_json::json!({"action": "bogus"})).await;
        assert!(!result.success);
    }
}
