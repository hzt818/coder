//! Task tool - task management

use super::*;
use async_trait::async_trait;
use std::sync::Mutex;

struct TaskItem {
    id: usize,
    description: String,
    status: String, // "pending", "in_progress", "completed"
}

static TASKS: Mutex<Vec<TaskItem>> = Mutex::new(Vec::new());
/// Maximum tasks to keep in memory to prevent unbounded growth.
const MAX_TASKS: usize = 500;

pub struct TaskTool;

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &str {
        "task"
    }

    fn description(&self) -> &str {
        "Manage tasks and track progress on multi-step work."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "update", "list", "complete"],
                    "description": "Action to perform"
                },
                "id": {
                    "type": "integer",
                    "description": "Task ID (for update/complete)"
                },
                "description": {
                    "type": "string",
                    "description": "Task description (for create/update)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("");

        match action {
            "create" => {
                let desc = args
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("Task");
                if desc.is_empty() {
                    return ToolResult::err("Description is required for create");
                }
                let mut tasks = TASKS.lock().unwrap();
                let id = tasks.len() + 1;
                tasks.push(TaskItem {
                    id,
                    description: desc.to_string(),
                    status: "pending".to_string(),
                });
                // Prevent unbounded memory growth by evicting oldest tasks
                if tasks.len() > MAX_TASKS {
                    tasks.remove(0);
                }
                ToolResult::ok(format!("Task #{} created: {}", id, desc))
            }
            "list" => {
                let tasks = TASKS.lock().unwrap();
                if tasks.is_empty() {
                    return ToolResult::ok("No tasks.");
                }
                let mut output = String::from("Tasks:\n");
                for task in tasks.iter() {
                    let marker = match task.status.as_str() {
                        "completed" => "[x]",
                        "in_progress" => "[-]",
                        _ => "[ ]",
                    };
                    output.push_str(&format!(
                        "  {} #{} - {}\n",
                        marker, task.id, task.description
                    ));
                }
                ToolResult::ok(output)
            }
            "complete" => {
                let id = args.get("id").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                let mut tasks = TASKS.lock().unwrap();
                if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                    task.status = "completed".to_string();
                    ToolResult::ok(format!(
                        "Task #{} marked as complete: {}",
                        id, task.description
                    ))
                } else {
                    ToolResult::err(format!("Task #{} not found", id))
                }
            }
            "update" => {
                let id = args.get("id").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                let desc = args
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                if desc.is_empty() {
                    return ToolResult::err("Description is required for update");
                }
                let mut tasks = TASKS.lock().unwrap();
                if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                    task.description = desc.to_string();
                    ToolResult::ok(format!("Task #{} updated: {}", id, desc))
                } else {
                    ToolResult::err(format!("Task #{} not found", id))
                }
            }
            _ => ToolResult::err(format!(
                "Unknown action: '{}'. Valid actions: create, list, complete, update",
                action
            )),
        }
    }
}
