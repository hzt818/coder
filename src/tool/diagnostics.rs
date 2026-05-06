//! Workspace diagnostics tool — lightweight environment probing.
//!
//! Gathers workspace info, git status, toolchain versions, and sandbox
//! availability without failing when optional commands are missing.

use super::*;
use async_trait::async_trait;
use std::env;
use std::process::Command;

pub struct DiagnosticsTool;

#[async_trait]
impl Tool for DiagnosticsTool {
    fn name(&self) -> &str {
        "diagnostics"
    }
    fn description(&self) -> &str {
        "Report workspace info, git status, Rust toolchain versions, and environment details."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {}, "additionalProperties": false
        })
    }

    async fn execute(&self, _args: serde_json::Value) -> ToolResult {
        let mut result = String::new();
        result.push_str("── Workspace Diagnostics ──\n\n");

        // Working directory
        result.push_str(&format!(
            "  Working dir: {}\n",
            env::current_dir()
                .map(|d| d.display().to_string())
                .unwrap_or_else(|_| "<error>".to_string())
        ));

        // Git probe
        let git_repo = probe_cmd(&["git", "rev-parse", "--is-inside-work-tree"]);
        result.push_str(&format!(
            "  Git repo: {}\n",
            if git_repo { "yes" } else { "no" }
        ));
        if git_repo {
            let branch = probe_cmd_output(&["git", "rev-parse", "--abbrev-ref", "HEAD"]);
            result.push_str(&format!(
                "  Git branch: {}\n",
                branch.unwrap_or_else(|| "<unknown>".to_string())
            ));
            let status = probe_cmd_output(&["git", "status", "--short"]);
            result.push_str(&format!(
                "  Git changes: {}\n",
                status.as_deref().unwrap_or("<unknown>")
            ));
        }

        // Rust toolchain
        result.push_str(&format!(
            "  Rust: {}\n",
            probe_cmd_output(&["rustc", "--version"]).unwrap_or_else(|| "<not found>".to_string())
        ));
        result.push_str(&format!(
            "  Cargo: {}\n",
            probe_cmd_output(&["cargo", "--version"]).unwrap_or_else(|| "<not found>".to_string())
        ));

        // OS info
        result.push_str(&format!("  OS: {}\n", std::env::consts::OS));
        result.push_str(&format!("  Arch: {}\n", std::env::consts::ARCH));

        // Environment (safe vars only)
        result.push_str("\n  Environment:\n");
        for var in ["SHELL", "TERM", "HOME", "USER", "LANG", "PATH"] {
            if let Ok(val) = env::var(var) {
                let display = if var == "PATH" {
                    format!("{} entries", val.split(':').count())
                } else {
                    val
                };
                result.push_str(&format!("    {}={}\n", var, display));
            }
        }

        ToolResult::ok(result)
    }

    fn requires_permission(&self) -> bool {
        false
    }
}

/// Run a command and return whether it succeeded.
fn probe_cmd(args: &[&str]) -> bool {
    Command::new(args[0])
        .args(&args[1..])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run a command and capture stdout on success.
fn probe_cmd_output(args: &[&str]) -> Option<String> {
    Command::new(args[0])
        .args(&args[1..])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        let tool = DiagnosticsTool;
        assert_eq!(tool.name(), "diagnostics");
    }

    #[test]
    fn test_tool_schema() {
        let tool = DiagnosticsTool;
        assert!(tool.schema().get("properties").is_some());
    }

    #[tokio::test]
    async fn test_execute_succeeds() {
        let tool = DiagnosticsTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.success);
        assert!(result.output.contains("Workspace Diagnostics"));
    }
}
