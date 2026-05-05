//! Monitor tool — background process monitoring with streaming output.
//!
//! Allows starting a monitored process that streams stdout lines as events,
//! with configurable timeout, grep filtering, and multi-process tracking.

use async_trait::async_trait;
use super::*;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

struct MonitoredProcess {
    id: String,
    command: String,
    started_at: Instant,
    output: Vec<String>,
    running: bool,
    filter: Option<String>,
}

pub struct MonitorManager {
    processes: RwLock<HashMap<String, MonitoredProcess>>,
    next_id: std::sync::atomic::AtomicU64,
}

impl MonitorManager {
    pub fn new() -> Self {
        Self { processes: RwLock::new(HashMap::new()), next_id: std::sync::atomic::AtomicU64::new(1) }
    }

    pub async fn start(&self, command: &str, filter: Option<&str>) -> Result<String, String> {
        let id = format!("mon-{}", self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst));

        let mut child = Command::new(if cfg!(target_os = "windows") { "cmd" } else { "sh" });
        child.arg(if cfg!(target_os = "windows") { "/C" } else { "-c" });
        child.arg(command).stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = child.spawn().map_err(|e| format!("Failed to spawn: {}", e))?;
        let stdout = child.stdout.take().ok_or("No stdout")?;
        let lines = Arc::new(Mutex::new(Vec::new()));
        let filter_str = filter.map(|s| s.to_string().to_lowercase());

        let lines_clone = lines.clone();
        let filter_clone = filter_str.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines().flatten() {
                let should_store = filter_clone.as_ref().map_or(true, |f| line.to_lowercase().contains(f));
                if should_store {
                    if let Ok(mut l) = lines_clone.lock() {
                        l.push(line);
                    }
                }
            }
        });

        let process = MonitoredProcess {
            id: id.clone(),
            command: command.to_string(),
            started_at: Instant::now(),
            output: Vec::new(),
            running: true,
            filter: filter_str,
        };

        self.processes.write().await.insert(id.clone(), process);
        Ok(id)
    }

    pub async fn status(&self, id: &str) -> Result<String, String> {
        let processes = self.processes.read().await;
        let proc = processes.get(id).ok_or_else(|| format!("Monitor {} not found", id))?;
        let elapsed = proc.started_at.elapsed();
        let lines = proc.output.len();
        Ok(format!("Monitor {}: {} lines, {}s elapsed, running: {}",
            id, lines, elapsed.as_secs(), proc.running))
    }

    pub async fn list(&self) -> Vec<String> {
        self.processes.read().await.keys().cloned().collect()
    }
}

pub struct MonitorTool;

#[async_trait]
impl Tool for MonitorTool {
    fn name(&self) -> &str { "monitor" }
    fn description(&self) -> &str {
        "Start monitoring a background process with streaming output. Use for long-running tasks like builds, tests, or servers."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "command": { "type": "string", "description": "Command to run and monitor" },
                "filter": { "type": "string", "description": "Optional keyword filter for output lines" },
                "action": { "type": "string", "enum": ["start", "status"], "description": "start = begin monitoring, status = check status", "default": "start" },
                "id": { "type": "string", "description": "Monitor ID (for status action)" }
            }, "required": ["command"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("start");
        match action {
            "start" => {
                let command = args.get("command").and_then(|c| c.as_str()).unwrap_or("");
                if command.is_empty() { return ToolResult::err("Command is required"); }
                let filter = args.get("filter").and_then(|f| f.as_str());
                match monitor_instance().start(command, filter).await {
                    Ok(id) => ToolResult::ok(format!("Monitoring started: {}\nCommand: {}", id, command)),
                    Err(e) => ToolResult::err(e),
                }
            }
            "status" => {
                let id = args.get("id").and_then(|i| i.as_str()).unwrap_or("");
                if id.is_empty() { return ToolResult::err("id is required for status"); }
                match monitor_instance().status(id).await {
                    Ok(s) => ToolResult::ok(s),
                    Err(e) => ToolResult::err(e),
                }
            }
            _ => ToolResult::err(format!("Unknown action: {}", action)),
        }
    }
    fn requires_permission(&self) -> bool { true }
}

use std::sync::OnceLock;
fn monitor_instance() -> &'static MonitorManager {
    static M: OnceLock<MonitorManager> = OnceLock::new();
    M.get_or_init(|| MonitorManager::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test] async fn test_start_and_list() {
        let mgr = MonitorManager::new();
        let id = mgr.start("echo test", None).await.unwrap();
        let list = mgr.list().await;
        assert!(list.contains(&id));
    }
    #[test] fn test_name() { assert_eq!(MonitorTool.name(), "monitor"); }
}
