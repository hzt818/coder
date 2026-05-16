//! Local sandbox backend
//!
//! Executes commands directly on the host system via `tokio::process::Command`.

use async_trait::async_trait;
use std::time::Duration;
use tokio::process::Command;

use super::{SandboxBackend, SandboxResult};

/// Sandbox backend that runs commands directly on the local machine
#[derive(Debug, Clone)]
pub struct LocalSandbox;

#[async_trait]
impl SandboxBackend for LocalSandbox {
    fn name(&self) -> &str {
        "local"
    }

    async fn execute(&self, command: &str, workdir: &str, timeout_secs: u64) -> SandboxResult {
        let shell = if cfg!(target_os = "windows") {
            "cmd.exe"
        } else {
            "sh"
        };
        let shell_arg = if cfg!(target_os = "windows") {
            "/C"
        } else {
            "-c"
        };

        let output = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            Command::new(shell)
                .arg(shell_arg)
                .arg(command)
                .current_dir(workdir)
                .output(),
        )
        .await;

        match output {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);
                SandboxResult {
                    stdout,
                    stderr,
                    exit_code,
                    timed_out: false,
                }
            }
            Ok(Err(e)) => SandboxResult {
                stdout: String::new(),
                stderr: format!("Failed to execute command: {}", e),
                exit_code: -1,
                timed_out: false,
            },
            Err(_) => SandboxResult {
                stdout: String::new(),
                stderr: format!("Command timed out after {}s", timeout_secs),
                exit_code: -1,
                timed_out: true,
            },
        }
    }
}

impl LocalSandbox {
    /// Create a new LocalSandbox
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalSandbox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_sandbox_echo() {
        let sandbox = LocalSandbox::new();
        let result = sandbox.execute("echo hello", ".", 30).await;
        assert_eq!(result.exit_code, 0);
        assert!(
            result.stdout.contains("hello"),
            "stdout: {:?}",
            result.stdout
        );
        assert!(!result.timed_out);
    }

    #[tokio::test]
    async fn test_local_sandbox_failing_command() {
        let sandbox = LocalSandbox::new();
        let result = sandbox.execute("exit 42", ".", 30).await;
        assert_eq!(result.exit_code, 42);
        assert!(!result.timed_out);
    }

    #[tokio::test]
    async fn test_local_sandbox_timeout() {
        let sandbox = LocalSandbox::new();
        // Use a subprocess that hangs for longer than the timeout.
        // On Windows, `ping -n 10 127.0.0.1 >nul` blocks ~9 seconds;
        // on Unix, `sleep 10` blocks 10 seconds.
        let cmd = if cfg!(target_os = "windows") {
            "ping -n 10 127.0.0.1 >nul"
        } else {
            "sleep 10"
        };
        let result = sandbox.execute(cmd, ".", 1).await;
        assert!(result.timed_out, "expected timeout, got exit_code={}", result.exit_code);
        assert!(result.stderr.contains("timed out"));
    }
}
