use async_trait::async_trait;
use super::*;
use std::sync::Mutex;

static GATES: Mutex<Vec<GateRecord>> = Mutex::new(Vec::new());
/// Maximum gate records to retain in memory.
const MAX_GATES: usize = 500;

#[derive(Debug, Clone)]
struct GateRecord {
    id: usize,
    task_id: String,
    command: String,
    #[allow(dead_code)]
    exit_code: i32,
    duration_ms: u64,
    #[allow(dead_code)]
    stdout: String,
    #[allow(dead_code)]
    stderr: String,
    classification: String,
}

pub struct TaskGateTool;

#[async_trait]
impl Tool for TaskGateTool {
    fn name(&self) -> &str { "task_gate" }
    fn description(&self) -> &str { "Run verification commands and attach structured evidence to tasks." }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "operation": { "type": "string", "enum": ["run","list"], "description": "Operation" },
                "task_id": { "type": "string", "description": "Task ID" },
                "command": { "type": "string", "description": "Verification command" },
                "cwd": { "type": "string", "description": "Working directory", "default": "." }
            }, "required": ["operation"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let op = args.get("operation").and_then(|o| o.as_str()).unwrap_or("");
        match op {
            "run" => {
                let task_id = args.get("task_id").and_then(|t| t.as_str()).unwrap_or("");
                let command = args.get("command").and_then(|c| c.as_str()).unwrap_or("");
                if task_id.is_empty() || command.is_empty() {
                    return ToolResult::err("task_id and command required");
                }
                let start = std::time::Instant::now();
                let (shell, arg) = if cfg!(target_os = "windows") {
                    ("cmd.exe", "/C")
                } else {
                    ("sh", "-c")
                };
                let output = tokio::process::Command::new(shell)
                    .arg(arg).arg(command)
                    .output().await;
                let duration = start.elapsed().as_millis() as u64;
                match output {
                    Ok(o) => {
                        let code = o.status.code().unwrap_or(-1);
                        let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                        let classification = if code == 0 { "pass" } else { "fail" };
                        let mut gates = GATES.lock().unwrap();
                        let id = gates.len() + 1;
                        gates.push(GateRecord { id, task_id: task_id.into(), command: command.into(), exit_code: code, duration_ms: duration, stdout, stderr, classification: classification.into() });
                        if gates.len() > MAX_GATES {
                            gates.remove(0);
                        }
                        ToolResult::ok(format!("Gate run #{} for task '{}':\nCommand: {}\nExit code: {}\nDuration: {}ms\nClassification: {}", id, task_id, command, code, duration, classification))
                    }
                    Err(e) => ToolResult::err(format!("Failed: {}", e)),
                }
            }
            "list" => {
                let task_id = args.get("task_id").and_then(|t| t.as_str()).unwrap_or("");
                let gates = GATES.lock().unwrap();
                let results: Vec<&GateRecord> = if task_id.is_empty() { gates.iter().collect() } else { gates.iter().filter(|g| g.task_id == task_id).collect() };
                if results.is_empty() { return ToolResult::ok("No gate results."); }
                let mut out = format!("Gate results ({}):\n", results.len());
                for g in &results { out.push_str(&format!("  #{} [{}] {} - {}ms\n", g.id, g.classification, g.command, g.duration_ms)); }
                ToolResult::ok(out)
            }
            _ => ToolResult::err(format!("Unknown operation: {}", op)),
        }
    }

    fn requires_permission(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_name() { assert_eq!(TaskGateTool.name(), "task_gate"); }
    #[test] fn test_schema() { assert!(TaskGateTool.schema().get("properties").is_some()); }
    #[tokio::test] async fn test_empty_op() { assert!(!TaskGateTool.execute(serde_json::json!({})).await.success); }
    #[tokio::test] async fn test_list() { assert!(TaskGateTool.execute(serde_json::json!({"operation":"list"})).await.success); }
    #[tokio::test] async fn test_run_no_args() { assert!(!TaskGateTool.execute(serde_json::json!({"operation":"run"})).await.success); }
}
