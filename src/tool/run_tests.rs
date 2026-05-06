//! Test runner tool — runs `cargo test` in the workspace.
//!
//! Intentionally auto-approved to encourage frequent test loops.

use super::*;
use async_trait::async_trait;
use std::process::Command;

pub struct RunTestsTool;

#[async_trait]
impl Tool for RunTestsTool {
    fn name(&self) -> &str {
        "run_tests"
    }
    fn description(&self) -> &str {
        "Run `cargo test` in the workspace root with optional extra arguments."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "args": { "type": "string", "description": "Extra args for cargo test" },
                "all_features": { "type": "boolean", "description": "Include --all-features", "default": false }
            }, "additionalProperties": false
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let all_features = args
            .get("all_features")
            .and_then(|a| a.as_bool())
            .unwrap_or(false);
        let extra_args = args.get("args").and_then(|a| a.as_str()).unwrap_or("");

        let mut cmd = Command::new("cargo");
        cmd.arg("test");
        if all_features {
            cmd.arg("--all-features");
        }
        if !extra_args.is_empty() {
            for token in extra_args.split_whitespace() {
                cmd.arg(token);
            }
        }

        let start = std::time::Instant::now();
        let output = match cmd.output() {
            Ok(o) => o,
            Err(e) => return ToolResult::err(format!("Failed to run cargo test: {}", e)),
        };
        let elapsed = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let success = output.status.success();

        let mut result = format!("── Test Results ({:.1?}) ──\n\n", elapsed);
        result.push_str(&format!(
            "Status: {}\n",
            if success { "PASSED" } else { "FAILED" }
        ));

        // Extract test summary from stdout
        let summary_line = stdout
            .lines()
            .rev()
            .find(|l| l.contains("test result:") || l.contains("test ") && l.contains("passed"));
        if let Some(summary) = summary_line {
            result.push_str(&format!("Summary: {}\n\n", summary));
        }

        // Show failures prominently
        let failures: Vec<&str> = stdout
            .lines()
            .filter(|l| l.contains("FAILED") || l.contains("panicked at"))
            .collect();
        if !failures.is_empty() {
            result.push_str(&format!("Failures ({}):\n", failures.len()));
            for f in failures.iter().take(10) {
                result.push_str(&format!("  {}\n", f));
            }
        }

        // Show output
        result.push_str(&format!("\nstdout ({} bytes):\n", stdout.len()));
        let max_chars = 20_000;
        if stdout.len() > max_chars {
            result.push_str(&stdout[..max_chars]);
            result.push_str(&format!(
                "\n... (truncated, {} more bytes)",
                stdout.len() - max_chars
            ));
        } else {
            result.push_str(&stdout);
        }

        if !stderr.is_empty() {
            result.push_str(&format!(
                "\nstderr ({} bytes):\n{}",
                stderr.len(),
                &stderr[..stderr.len().min(5000)]
            ));
        }

        ToolResult::ok(result)
    }

    fn requires_permission(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(RunTestsTool.name(), "run_tests");
    }

    #[test]
    fn test_tool_schema() {
        assert!(RunTestsTool.schema().get("properties").is_some());
    }
}
