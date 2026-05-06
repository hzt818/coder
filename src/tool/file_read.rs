//! File read tool

use super::*;
use async_trait::async_trait;

pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Supports optional offset and limit for partial reads."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line offset to start reading from (0-indexed)",
                    "default": 0
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read",
                    "default": 2000
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");

        if path.is_empty() {
            return ToolResult::err("Path is required");
        }

        let offset = args.get("offset").and_then(|o| o.as_u64()).unwrap_or(0) as usize;
        let limit = args.get("limit").and_then(|l| l.as_u64()).unwrap_or(2000) as usize;

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return ToolResult::err(format!("Failed to read file '{}': {}", path, e)),
        };

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        if offset >= total_lines {
            return ToolResult::ok(format!(
                "File has {} lines, offset {} is beyond end.",
                total_lines, offset
            ));
        }

        let end = std::cmp::min(offset + limit, total_lines);
        let selected = lines[offset..end].join("\n");

        let mut result = format!(
            "File: {}\nLines {}-{} of {}\n\n{}",
            path,
            offset + 1,
            end,
            total_lines,
            selected
        );

        if end < total_lines {
            result.push_str(&format!(
                "\n\n... ({} more lines. Use offset={} to continue)",
                total_lines - end,
                end
            ));
        }

        ToolResult::ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_file_read_tool_name() {
        let tool = FileReadTool;
        assert_eq!(tool.name(), "file_read");
    }

    #[tokio::test]
    async fn test_file_read_not_found() {
        let tool = FileReadTool;
        let result = tool
            .execute(serde_json::json!({"path": "/nonexistent/file.txt"}))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_file_read_empty_path() {
        let tool = FileReadTool;
        let result = tool.execute(serde_json::json!({"path": ""})).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_file_read_success() {
        let tool = FileReadTool;
        // Create temp file
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "line1\nline2\nline3").unwrap();

        let result = tool.execute(serde_json::json!({"path": tmp.path()})).await;
        assert!(result.success);
        assert!(result.output.contains("line1"));
    }
}
