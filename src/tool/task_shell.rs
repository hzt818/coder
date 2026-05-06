//! Background shell task management
//!
//! Provides tools for starting, monitoring, interacting with, and
//! cancelling persistent background shell processes. Ported from
//! DeepSeek-TUI's BackgroundShell pattern.

use super::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::{BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Status of a shell process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellStatus {
    Running,
    Completed,
    Failed,
    Killed,
    TimedOut,
}

/// A tracked background shell job.
#[derive(Debug)]
struct BackgroundJob {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout_buf: Arc<Mutex<Vec<u8>>>,
    stderr_buf: Arc<Mutex<Vec<u8>>>,
    status: ShellStatus,
    exit_code: Option<i32>,
}

impl BackgroundJob {
    fn try_update_status(&mut self) {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.exit_code = status.code();
                    self.status = if status.success() {
                        ShellStatus::Completed
                    } else {
                        ShellStatus::Failed
                    };
                    // Read remaining output
                    // ... (handled by reader threads)
                }
                Ok(None) => {} // still running
                Err(_) => self.status = ShellStatus::Failed,
            }
        }
    }
}

/// Manages all background shell jobs.
pub struct BackgroundShellManager {
    jobs: RwLock<HashMap<String, BackgroundJob>>,
    next_id: std::sync::atomic::AtomicU64,
}

impl BackgroundShellManager {
    pub fn new() -> Self {
        Self {
            jobs: RwLock::new(HashMap::new()),
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Generate a unique job ID.
    fn generate_id(&self) -> String {
        format!(
            "job-{}",
            self.next_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        )
    }

    /// Start a background shell command.
    pub async fn start(&self, command: &str, cwd: Option<&str>) -> Result<String, String> {
        let id = self.generate_id();

        let mut child = Command::new(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "sh"
        });
        child.arg(if cfg!(target_os = "windows") {
            "/C"
        } else {
            "-c"
        });
        child
            .arg(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = cwd {
            child.current_dir(dir);
        }

        let mut child = child
            .spawn()
            .map_err(|e| format!("Failed to spawn: {}", e))?;

        let stdin = child.stdin.take().ok_or_else(|| "No stdin".to_string())?;
        let stdout = child.stdout.take().ok_or_else(|| "No stdout".to_string())?;
        let stderr = child.stderr.take().ok_or_else(|| "No stderr".to_string())?;

        let stdout_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let stderr_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));

        // Reader threads
        let out_buf = stdout_buf.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut buf = vec![0u8; 4096];
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(mut g) = out_buf.lock() {
                            g.extend_from_slice(&buf[..n]);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let err_buf = stderr_buf.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            loop {
                let mut buf = vec![0u8; 4096];
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(mut g) = err_buf.lock() {
                            g.extend_from_slice(&buf[..n]);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let job = BackgroundJob {
            child: Some(child),
            stdin: Some(stdin),
            stdout_buf,
            stderr_buf,
            status: ShellStatus::Running,
            exit_code: None,
        };

        self.jobs.write().await.insert(id.clone(), job);
        Ok(id)
    }

    /// List all background jobs.
    pub async fn list(&self) -> Vec<String> {
        let jobs = self.jobs.read().await;
        jobs.keys().cloned().collect()
    }

    /// Wait for a job and return its output.
    pub async fn wait(&self, id: &str, timeout_secs: Option<u64>) -> Result<String, String> {
        // This is simplified - a real implementation would poll
        let deadline = timeout_secs.map(|s| Instant::now() + Duration::from_secs(s));
        loop {
            {
                let mut jobs = self.jobs.write().await;
                if let Some(job) = jobs.get_mut(id) {
                    job.try_update_status();
                    if job.status != ShellStatus::Running {
                        return Ok(format!(
                            "Job {} finished: {:?}\nstdout: {} bytes\nstderr: {} bytes",
                            id,
                            job.status,
                            job.stdout_buf.lock().unwrap().len(),
                            job.stderr_buf.lock().unwrap().len()
                        ));
                    }
                } else {
                    return Err(format!("Job not found: {}", id));
                }
            }
            if let Some(dead) = deadline {
                if Instant::now() >= dead {
                    return Err("Timeout".to_string());
                }
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    /// Send input to a running job's stdin.
    pub async fn send_input(&self, id: &str, input: &str) -> Result<(), String> {
        let mut jobs = self.jobs.write().await;
        let job = jobs
            .get_mut(id)
            .ok_or_else(|| format!("Job not found: {}", id))?;
        if job.status != ShellStatus::Running {
            return Err("Job is not running".to_string());
        }
        if let Some(ref mut stdin) = job.stdin {
            stdin
                .write_all(input.as_bytes())
                .map_err(|e| format!("Write error: {}", e))?;
            stdin.flush().map_err(|e| format!("Flush error: {}", e))?;
        }
        Ok(())
    }

    /// Cancel a running job.
    pub async fn cancel(&self, id: &str) -> Result<(), String> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(id) {
            job.status = ShellStatus::Killed;
            if let Some(ref mut child) = job.child {
                let _ = child.kill();
                let _ = child.wait();
            }
            Ok(())
        } else {
            Err(format!("Job not found: {}", id))
        }
    }
}

// ── Tool Implementations ──────────────────────────────────────────────────────

use std::sync::OnceLock;
fn bg_manager() -> &'static BackgroundShellManager {
    static BG: OnceLock<BackgroundShellManager> = OnceLock::new();
    BG.get_or_init(|| BackgroundShellManager::new())
}

pub struct TaskShellStart;

#[async_trait]
impl Tool for TaskShellStart {
    fn name(&self) -> &str {
        "task_shell_start"
    }
    fn description(&self) -> &str {
        "Start a command in the background and return a job ID"
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "command": { "type": "string", "description": "Shell command to run" },
                "cwd": { "type": "string", "description": "Working directory (optional)" }
            }, "required": ["command"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let cmd = args.get("command").and_then(|c| c.as_str()).unwrap_or("");
        if cmd.is_empty() {
            return ToolResult::err("Command is required");
        }
        let cwd = args.get("cwd").and_then(|c| c.as_str());
        match bg_manager().start(cmd, cwd).await {
            Ok(id) => ToolResult::ok(format!(
                "Started background job: {}\nCommand: {}\nUse task_shell_wait to check on it.",
                id, cmd
            )),
            Err(e) => ToolResult::err(e),
        }
    }
    fn requires_permission(&self) -> bool {
        true
    }
}

pub struct TaskShellWait;

#[async_trait]
impl Tool for TaskShellWait {
    fn name(&self) -> &str {
        "task_shell_wait"
    }
    fn description(&self) -> &str {
        "Wait for a background job to complete and get its output"
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "job_id": { "type": "string", "description": "Job ID from task_shell_start" },
                "timeout": { "type": "integer", "description": "Max wait time in seconds", "default": 30 }
            }, "required": ["job_id"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let id = args.get("job_id").and_then(|j| j.as_str()).unwrap_or("");
        if id.is_empty() {
            return ToolResult::err("job_id is required");
        }
        let timeout = args.get("timeout").and_then(|t| t.as_i64()).unwrap_or(30);
        match bg_manager().wait(id, Some(timeout as u64)).await {
            Ok(output) => ToolResult::ok(output),
            Err(e) => ToolResult::err(e),
        }
    }
    fn requires_permission(&self) -> bool {
        false
    }
}

pub struct TaskShellInteract;

#[async_trait]
impl Tool for TaskShellInteract {
    fn name(&self) -> &str {
        "task_shell_interact"
    }
    fn description(&self) -> &str {
        "Send input to a running background job's stdin"
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "job_id": { "type": "string", "description": "Job ID" },
                "input": { "type": "string", "description": "Input to send" }
            }, "required": ["job_id", "input"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let id = args.get("job_id").and_then(|j| j.as_str()).unwrap_or("");
        let input = args.get("input").and_then(|i| i.as_str()).unwrap_or("");
        if id.is_empty() {
            return ToolResult::err("job_id is required");
        }
        match bg_manager().send_input(id, input).await {
            Ok(()) => ToolResult::ok("Input sent"),
            Err(e) => ToolResult::err(e),
        }
    }
    fn requires_permission(&self) -> bool {
        true
    }
}

pub struct TaskShellCancel;

#[async_trait]
impl Tool for TaskShellCancel {
    fn name(&self) -> &str {
        "task_shell_cancel"
    }
    fn description(&self) -> &str {
        "Cancel a background job"
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "job_id": { "type": "string", "description": "Job ID to cancel" }
            }, "required": ["job_id"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let id = args.get("job_id").and_then(|j| j.as_str()).unwrap_or("");
        if id.is_empty() {
            return ToolResult::err("job_id is required");
        }
        match bg_manager().cancel(id).await {
            Ok(()) => ToolResult::ok(format!("Job {} cancelled", id)),
            Err(e) => ToolResult::err(e),
        }
    }
    fn requires_permission(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_and_list() {
        let mgr = BackgroundShellManager::new();
        let id = mgr.start("echo hello", None).await.unwrap();
        let jobs = mgr.list().await;
        assert!(jobs.contains(&id));
    }

    #[tokio::test]
    async fn test_cancel_nonexistent() {
        let mgr = BackgroundShellManager::new();
        assert!(mgr.cancel("nonexistent").await.is_err());
    }

    #[tokio::test]
    async fn test_wait_nonexistent() {
        let mgr = BackgroundShellManager::new();
        assert!(mgr.wait("nonexistent", Some(1)).await.is_err());
    }

    #[test]
    fn test_shell_status_equality() {
        assert_eq!(ShellStatus::Running, ShellStatus::Running);
        assert_ne!(ShellStatus::Running, ShellStatus::Completed);
    }
}
