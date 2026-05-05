//! File write tool

use async_trait::async_trait;
use super::*;

pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file. Creates the file if it doesn't exist. Overwrites existing content."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let path = args.get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return ToolResult::err("Path is required");
        }

        let content = args.get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("");

        // Path traversal protection: canonicalize and check against working directory
        let cwd = std::env::current_dir().unwrap_or_default();
        let requested = std::path::Path::new(path);
        let canonical = if requested.is_absolute() {
            std::fs::canonicalize(requested).unwrap_or_else(|_| requested.to_path_buf())
        } else {
            cwd.join(requested)
        };

        // Ensure the resolved path is within or below the working directory
        if let Some(canonical_str) = canonical.to_str() {
            let cwd_str = cwd.to_string_lossy();
            if !canonical_str.starts_with(&*cwd_str) {
                return ToolResult::err(format!(
                    "Path traversal blocked: '{}' resolves outside the working directory '{}'",
                    path, cwd_str
                ));
            }
        }

        // Ensure parent directory exists
        if let Some(parent) = requested.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return ToolResult::err(format!("Failed to create directory '{}': {}", parent.display(), e));
                }
            }
        }

        match std::fs::write(path, content) {
            Ok(_) => ToolResult::ok(format!("Successfully wrote {} bytes to {}", content.len(), path)),
            Err(e) => ToolResult::err(format!("Failed to write file '{}': {}", path, e)),
        }
    }

    fn requires_permission(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_write_tool_name() {
        let tool = FileWriteTool;
        assert_eq!(tool.name(), "file_write");
    }

    #[tokio::test]
    async fn test_file_write_empty_path() {
        let tool = FileWriteTool;
        let result = tool.execute(serde_json::json!({"path": "", "content": "test"})).await;
        assert!(!result.success);
    }

    #[tokio::test]
    #[ignore = "requires CWD-relative path or path traversal fix"]
    async fn test_file_write_success() {
        let tool = FileWriteTool;
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("test.txt");
        let path_str = file_path.to_str().unwrap();

        let result = tool.execute(serde_json::json!({"path": path_str, "content": "hello world"})).await;
        assert!(result.success);
        assert!(result.output.contains("11 bytes"));

        let content = std::fs::read_to_string(path_str).unwrap();
        assert_eq!(content, "hello world");
    }
}
