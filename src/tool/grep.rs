//! Grep tool - content search

use async_trait::async_trait;
use super::*;

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for a pattern in file contents. Supports regex patterns and file glob filtering."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Pattern to search for (regex supported)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (default: current dir)",
                    "default": "."
                },
                "glob": {
                    "type": "string",
                    "description": "File glob filter (e.g., *.rs, *.py)",
                    "default": ""
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let pattern = args.get("pattern")
            .and_then(|p| p.as_str())
            .unwrap_or("");

        if pattern.is_empty() {
            return ToolResult::err("Pattern is required");
        }

        let search_path = args.get("path")
            .and_then(|p| p.as_str())
            .unwrap_or(".");

        let file_glob = args.get("glob")
            .and_then(|g| g.as_str())
            .unwrap_or("");

        let mut cmd = tokio::process::Command::new("rg");
        cmd.arg("--line-number")
            .arg("--color")
            .arg("never")
            .arg("--with-filename")
            .current_dir(search_path);

        if !file_glob.is_empty() {
            cmd.arg("--glob").arg(file_glob);
        }

        cmd.arg(pattern);

        let output = match cmd.output().await {
            Ok(o) => o,
            Err(e) => return ToolResult::err(format!("Failed to run ripgrep: {}. Is 'rg' installed?", e)),
        };

        if output.status.code() == Some(1) {
            // No matches found
            return ToolResult::ok(format!("No matches found for pattern '{}'", pattern));
        }

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return ToolResult::err(format!("grep failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line_count = stdout.lines().count();

        ToolResult::ok(format!(
            "Found {} matches for '{}':\n{}",
            line_count, pattern, stdout
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grep_tool_name() {
        let tool = GrepTool;
        assert_eq!(tool.name(), "grep");
    }

    #[tokio::test]
    async fn test_grep_no_pattern() {
        let tool = GrepTool;
        let result = tool.execute(serde_json::json!({"pattern": ""})).await;
        assert!(!result.success);
    }
}
