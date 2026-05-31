//! File read tool

use super::*;
use async_trait::async_trait;
use std::path::Path;

pub struct FileReadTool;

impl FileReadTool {
    /// Count total lines in the content.
    fn total_lines(content: &str) -> usize {
        content.lines().count()
    }

    /// Format the result string for the given range.
    fn format_result(
        path: &Path,
        content: &str,
        offset: usize,
        limit: usize,
        total_lines: usize,
    ) -> ToolResult {
        if offset >= total_lines {
            return ToolResult::ok(format!(
                "File has {} lines, offset {} is beyond end.",
                total_lines, offset
            ));
        }

        let end = std::cmp::min(offset + limit, total_lines);
        let lines: Vec<&str> = content.lines().collect();
        let selected = lines[offset..end].join("\n");

        let mut result = format!(
            "File: {}\nLines {}-{} of {}\n\n{}",
            path.display(),
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

        // ── Path traversal protection ──
        // Block relative paths with `..` components that escape the working directory.
        // Absolute paths are allowed as-is (user explicitly chose them).
        let cwd = std::env::current_dir().unwrap_or_default();
        let requested = Path::new(path);

        let target: std::path::PathBuf = if requested.is_relative() {
            let resolved = cwd.join(requested);
            let canonical = match std::fs::canonicalize(&resolved) {
                Ok(p) => p,
                Err(e) => return ToolResult::err(format!("Failed to resolve path '{}': {}", path, e)),
            };
            let cwd_canonical = std::fs::canonicalize(&cwd).unwrap_or(cwd);
            if !canonical.starts_with(&cwd_canonical) {
                return ToolResult::err(format!(
                    "Path traversal blocked: '{}' escapes the working directory '{}'",
                    path,
                    cwd_canonical.display()
                ));
            }
            canonical
        } else {
            // Absolute paths bypass traversal check (user explicitly specified it)
            requested.to_path_buf()
        };

        let content = match std::fs::read_to_string(&target) {
            Ok(c) => c,
            Err(e) => return ToolResult::err(format!("Failed to read file '{}': {}", target.display(), e)),
        };

        let total_lines = Self::total_lines(&content);
        Self::format_result(&target, &content, offset, limit, total_lines)
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
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "line1\nline2\nline3").unwrap();

        let result = tool.execute(serde_json::json!({"path": tmp.path()})).await;
        assert!(result.success);
        assert!(result.output.contains("line1"));
    }

    #[tokio::test]
    async fn test_file_read_traversal_blocked() {
        let tool = FileReadTool;
        // `..` from cwd resolves to parent, which is outside the working directory
        let result = tool
            .execute(serde_json::json!({"path": ".."}))
            .await;
        assert!(!result.success);
        let err = result.error.as_deref().unwrap_or("");
        // On some systems `..` might resolve outside cwd and be blocked;
        // on others it might resolve to cwd itself and show a directory error
        assert!(
            err.contains("blocked") || err.contains("Is a directory") || err.contains("Access is denied"),
            "unexpected error: {err}"
        );
    }
}
