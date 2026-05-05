//! Sandbox backend system
//!
//! Provides a pluggable `SandboxBackend` trait for executing commands
//! in isolated environments. Supports local and remote sandboxes.

pub mod local;
pub mod remote;

pub use local::LocalSandbox;
pub use remote::RemoteSandbox;

use async_trait::async_trait;

/// Result of executing a command in a sandbox
#[derive(Debug, Clone)]
pub struct SandboxResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

/// Pluggable sandbox backend for command execution
#[async_trait]
pub trait SandboxBackend: Send + Sync {
    /// Execute a command within the sandbox
    async fn execute(&self, command: &str, workdir: &str, timeout_secs: u64) -> SandboxResult;

    /// Human-readable name of this sandbox backend
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_result_creation() {
        let result = SandboxResult {
            stdout: "hello".into(),
            stderr: String::new(),
            exit_code: 0,
            timed_out: false,
        };
        assert_eq!(result.stdout, "hello");
        assert!(!result.timed_out);
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn test_sandbox_result_with_error() {
        let result = SandboxResult {
            stdout: String::new(),
            stderr: "permission denied".into(),
            exit_code: 1,
            timed_out: false,
        };
        assert_eq!(result.exit_code, 1);
        assert!(result.stderr.contains("denied"));
    }

    #[test]
    fn test_sandbox_result_timed_out() {
        let result = SandboxResult {
            stdout: String::new(),
            stderr: "timeout".into(),
            exit_code: -1,
            timed_out: true,
        };
        assert!(result.timed_out);
        assert_eq!(result.exit_code, -1);
    }
}
