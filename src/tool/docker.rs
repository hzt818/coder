//! Docker tool - Docker container management via bash commands
//!
//! Supports ps, logs, exec, and compose operations.
//! Uses shell commands (docker CLI) instead of bollard API.

use async_trait::async_trait;
use tokio::process::Command;
use super::*;

pub struct DockerTool;

#[async_trait]
impl Tool for DockerTool {
    fn name(&self) -> &str {
        "docker"
    }

    fn description(&self) -> &str {
        "Manage Docker containers, images, and compose stacks. Supports ps, logs, exec, and compose operations."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["ps", "logs", "exec", "compose"],
                    "description": "Docker operation to perform"
                },
                "args": {
                    "type": "object",
                    "description": "Additional arguments for the operation",
                    "default": {}
                }
            },
            "required": ["operation"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let operation = args.get("operation")
            .and_then(|o| o.as_str())
            .unwrap_or("");

        let extra_args = args.get("args")
            .and_then(|a| a.as_object())
            .cloned()
            .unwrap_or_default();

        if operation.is_empty() {
            return ToolResult::err("Operation is required (ps, logs, exec, compose)");
        }

        match operation {
            "ps" => docker_ps(&extra_args).await,
            "logs" => docker_logs(&extra_args).await,
            "exec" => docker_exec(&extra_args).await,
            "compose" => docker_compose(&extra_args).await,
            _ => ToolResult::err(format!("Unknown docker operation: '{}'. Use: ps, logs, exec, compose", operation)),
        }
    }

    fn requires_permission(&self) -> bool {
        true
    }
}

async fn run_docker_cmd(args: &[&str]) -> Result<String, String> {
    let output = Command::new("docker")
        .args(args)
        .output()
        .await
        .map_err(|e| format!("Failed to run docker: {}. Is Docker installed?", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

/// List Docker containers (ps)
async fn docker_ps(args: &serde_json::Map<String, serde_json::Value>) -> ToolResult {
    let all = args.get("all").and_then(|a| a.as_bool()).unwrap_or(false);

    let mut cmd_args = vec!["ps"];
    if all {
        cmd_args.push("--all");
    }
    cmd_args.extend_from_slice(&["--format", "table {{.ID}}\t{{.Image}}\t{{.Status}}\t{{.Names}}"]);

    match run_docker_cmd(&cmd_args).await {
        Ok(output) => ToolResult::ok(output),
        Err(e) => ToolResult::err(format!("Docker ps failed: {}", e)),
    }
}

/// Get container logs
async fn docker_logs(args: &serde_json::Map<String, serde_json::Value>) -> ToolResult {
    let container = args.get("container")
        .and_then(|c| c.as_str())
        .unwrap_or("");

    if container.is_empty() {
        return ToolResult::err("Container name or ID is required");
    }

    let tail = args.get("tail")
        .and_then(|t| t.as_u64())
        .unwrap_or(50);

    match run_docker_cmd(&["logs", "--tail", &tail.to_string(), container]).await {
        Ok(output) => ToolResult::ok(format!("Logs for '{}' (last {} lines):\n\n{}", container, tail, output)),
        Err(e) => ToolResult::err(format!("Docker logs failed: {}", e)),
    }
}

/// Execute a command in a container
async fn docker_exec(args: &serde_json::Map<String, serde_json::Value>) -> ToolResult {
    let container = args.get("container")
        .and_then(|c| c.as_str())
        .unwrap_or("");

    if container.is_empty() {
        return ToolResult::err("Container name or ID is required");
    }

    let cmd = args.get("cmd")
        .and_then(|c| c.as_str())
        .unwrap_or("");

    if cmd.is_empty() {
        return ToolResult::err("Command to execute is required");
    }

    let output = match Command::new("docker")
        .arg("exec")
        .arg(container)
        .arg("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => return ToolResult::err(format!("Failed to execute docker exec: {}", e)),
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !result.is_empty() { result.push('\n'); }
        result.push_str(&format!("STDERR:\n{}", stderr));
    }

    ToolResult::ok(format!("Exec output from '{}':\n\n{}", container, result))
}

/// Docker compose operations (via docker CLI)
async fn docker_compose(args: &serde_json::Map<String, serde_json::Value>) -> ToolResult {
    let subcommand = args.get("subcommand")
        .and_then(|s| s.as_str())
        .unwrap_or("up");

    let project_dir = args.get("project_dir")
        .and_then(|p| p.as_str())
        .unwrap_or(".")
        .to_string();

    let mut cmd = Command::new("docker");
    cmd.arg("compose").current_dir(&project_dir);

    if let Some(f) = args.get("file").and_then(|f| f.as_str()) {
        cmd.arg("-f").arg(f);
    }

    match subcommand {
        "up" => { cmd.arg("up").arg("-d"); }
        "down" => { cmd.arg("down"); }
        "ps" => { cmd.arg("ps"); }
        "logs" => { cmd.arg("logs"); }
        "pull" => { cmd.arg("pull"); }
        "build" => { cmd.arg("build"); }
        "restart" => { cmd.arg("restart"); }
        other => { cmd.arg(other); }
    }

    let output = match cmd.output().await {
        Ok(o) => o,
        Err(e) => return ToolResult::err(format!("Failed to run docker compose: {}", e)),
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        ToolResult::ok(stdout)
    } else {
        ToolResult::err(if stderr.is_empty() { stdout } else { stderr })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_tool_name() {
        let tool = DockerTool;
        assert_eq!(tool.name(), "docker");
    }

    #[test]
    fn test_docker_schema() {
        let tool = DockerTool;
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }

    #[tokio::test]
    async fn test_docker_empty_operation() {
        let tool = DockerTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_docker_invalid_operation() {
        let tool = DockerTool;
        let result = tool.execute(serde_json::json!({"operation": "invalid"})).await;
        assert!(!result.success);
    }
}
