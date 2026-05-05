//! Bash tool - executes shell commands

use async_trait::async_trait;
use super::*;

/// Tool for executing shell commands
pub struct BashTool {
    pub default_timeout: u64,
    pub max_output_bytes: usize,
    pub sandbox_backend: Option<std::sync::Arc<dyn crate::sandbox::SandboxBackend>>,
}

impl Default for BashTool {
    fn default() -> Self {
        Self {
            default_timeout: 300,
            max_output_bytes: 1_048_576,
            sandbox_backend: None,
        }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute shell commands. Use this to run code, install packages, or interact with the system."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 300)",
                    "default": 300
                },
                "workdir": {
                    "type": "string",
                    "description": "Working directory (default: current dir)",
                    "default": "."
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let command = args.get("command")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        if command.is_empty() {
            return ToolResult::err("Command is required");
        }

        let timeout = args.get("timeout")
            .and_then(|t| t.as_u64())
            .unwrap_or(self.default_timeout);

        let workdir = args.get("workdir")
            .and_then(|d| d.as_str())
            .unwrap_or(".")
            .to_string();

        // Route through sandbox backend if configured
        if let Some(ref sandbox) = self.sandbox_backend {
            let result = sandbox.execute(&command, &workdir, timeout).await;
            let mut output = format!("$ {}\n\n{}", command, result.stdout);
            if !result.stderr.is_empty() {
                output.push_str(&format!("\nSTDERR:\n{}", result.stderr));
            }
            if result.timed_out {
                output.push_str(&format!("\nCommand timed out after {}s", timeout));
            }
            if result.exit_code != 0 {
                output.push_str(&format!("\nExit code: {}", result.exit_code));
            }
            return ToolResult::ok(output);
        }

        match execute_command(&command, &workdir, timeout, self.max_output_bytes).await {
            Ok(output) => ToolResult::ok(output),
            Err(e) => ToolResult::err(format!("Command failed: {}", e)),
        }
    }

    fn requires_permission(&self) -> bool {
        true
    }
}

/// Execute a shell command and return the output
async fn execute_command(
    command: &str,
    workdir: &str,
    timeout_secs: u64,
    max_output_bytes: usize,
) -> Result<String, String> {
    use tokio::process::Command;
    use std::time::Duration;

    let shell = if cfg!(target_os = "windows") {
        "cmd.exe"
    } else {
        "sh"
    };
    let shell_arg = if cfg!(target_os = "windows") { "/C" } else { "-c" };

    let output = tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        Command::new(shell)
            .arg(shell_arg)
            .arg(command)
            .current_dir(workdir)
            .output(),
    )
    .await
    .map_err(|_| format!("Command timed out after {}s", timeout_secs))?
    .map_err(|e| format!("Failed to execute command: {}", e))?;

    let mut result = String::new();
    let mut truncated = false;

    // Process stdout
    if !output.stdout.is_empty() {
        let chunk = String::from_utf8_lossy(&output.stdout);
        if result.len() + chunk.len() > max_output_bytes {
            let remaining = max_output_bytes.saturating_sub(result.len());
            let safe_end = chunk.floor_char_boundary(remaining);
            result.push_str(&chunk[..safe_end]);
            truncated = true;
        } else {
            result.push_str(&chunk);
        }
    }

    // Process stderr with label (always include, even if stdout was truncated)
    if !output.stderr.is_empty() {
        let needs_sep = !result.is_empty();
        result.push_str("STDERR:\n");
        let chunk = String::from_utf8_lossy(&output.stderr);
        if result.len() + chunk.len() > max_output_bytes {
            let remaining = max_output_bytes.saturating_sub(result.len());
            let safe_end = chunk.floor_char_boundary(remaining);
            if needs_sep { result.push('\n'); }
            result.push_str(&chunk[..safe_end]);
            truncated = true;
        } else {
            result.push_str(&chunk);
        }
    }

    if truncated {
        result.push_str("\n\n--- Output truncated (exceeded 1MB limit) ---");
    }

    if !output.status.success() {
        let exit_code = output.status.code().unwrap_or(-1);
        result.push_str(&format!("\nExit code: {}", exit_code));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_tool_name() {
        let tool = BashTool::default();
        assert_eq!(tool.name(), "bash");
    }

    #[test]
    fn test_bash_schema() {
        let tool = BashTool::default();
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }

    #[tokio::test]
    async fn test_bash_execute_empty() {
        let tool = BashTool::default();
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.success);
    }
}
