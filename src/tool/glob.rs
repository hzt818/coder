//! Glob tool - file pattern matching

use super::*;
use async_trait::async_trait;

pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Search for files matching a glob pattern. Supports **, *, and ? wildcards."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match (e.g., **/*.rs, src/**/*.ts)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (default: current dir)",
                    "default": "."
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let pattern = args.get("pattern").and_then(|p| p.as_str()).unwrap_or("");

        if pattern.is_empty() {
            return ToolResult::err("Pattern is required");
        }

        let search_path = args.get("path").and_then(|p| p.as_str()).unwrap_or(".");

        let glob_pattern = if search_path == "." {
            pattern.to_string()
        } else {
            format!("{}/{}", search_path.trim_end_matches('/'), pattern)
        };

        let mut results: Vec<String> = Vec::new();

        match ::glob::glob(&glob_pattern) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    results.push(entry.display().to_string());
                }
            }
            Err(e) => return ToolResult::err(format!("Invalid glob pattern: {}", e)),
        }

        results.sort();

        if results.is_empty() {
            return ToolResult::ok(format!("No matches found for pattern '{}'", pattern));
        }

        let output = format!(
            "Found {} matches for '{}':\n{}",
            results.len(),
            pattern,
            results.join("\n")
        );

        ToolResult::ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_tool_name() {
        let tool = GlobTool;
        assert_eq!(tool.name(), "glob");
    }

    #[tokio::test]
    async fn test_glob_no_pattern() {
        let tool = GlobTool;
        let result = tool.execute(serde_json::json!({"pattern": ""})).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_glob_pattern() {
        let tool = GlobTool;
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("test.rs"), "").unwrap();
        let path_str = tmp.path().to_str().unwrap();

        let result = tool
            .execute(serde_json::json!({
                "pattern": "**/*.rs",
                "path": path_str
            }))
            .await;
        assert!(result.success);
        assert!(result.output.contains("test.rs"));
    }
}
