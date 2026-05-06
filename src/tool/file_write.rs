//! File write tool

use super::*;
use async_trait::async_trait;

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
        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");

        if path.is_empty() {
            return ToolResult::err("Path is required");
        }

        let content = args.get("content").and_then(|c| c.as_str()).unwrap_or("");

        // ── Path traversal protection ──
        // Resolve the path safely: canonicalize an existing ancestor, then
        // append any non-existent tail components. Always use the resolved
        // path for the actual write.
        let cwd = std::env::current_dir().unwrap_or_default();
        let requested = std::path::Path::new(path);
        let resolved = if requested.is_absolute() {
            // Walk up from the requested path until we find an existing ancestor.
            let mut ancestor = Some(requested);
            let mut tails: Vec<&std::path::Path> = Vec::new();

            loop {
                match ancestor {
                    Some(p) if p.exists() => {
                        let base = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
                        let mut result = base;
                        for comp in tails.iter().rev() {
                            result.push(comp);
                        }
                        break result;
                    }
                    Some(p) => {
                        tails.push(p.file_name().map(std::path::Path::new).unwrap_or(p));
                        ancestor = p.parent();
                    }
                    None => {
                        return ToolResult::err(format!("Cannot resolve path '{}'", path));
                    }
                }
            }
        } else {
            cwd.join(requested)
        };

        // Verify the resolved path is within the working directory
        if let Some(resolved_str) = resolved.to_str() {
            let cwd_str = cwd.to_string_lossy();
            if !resolved_str.starts_with(&*cwd_str) {
                return ToolResult::err(format!(
                    "Path traversal blocked: '{}' resolves outside the working directory '{}'",
                    path, cwd_str
                ));
            }
        }

        // Ensure parent directory exists (create if needed)
        if let Some(parent) = resolved.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return ToolResult::err(format!(
                        "Failed to create directory '{}': {}",
                        parent.display(),
                        e
                    ));
                }
            }
        }

        // Write to the resolved canonical path
        match std::fs::write(&resolved, content) {
            Ok(_) => ToolResult::ok(format!(
                "Successfully wrote {} bytes to {}",
                content.len(),
                resolved.display()
            )),
            Err(e) => ToolResult::err(format!(
                "Failed to write file '{}': {}",
                resolved.display(),
                e
            )),
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
        let result = tool
            .execute(serde_json::json!({"path": "", "content": "test"}))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_file_write_success() {
        let tool = FileWriteTool;
        // Use a relative path so it stays within the cwd and bypasses
        // the platform-specific canonicalization differences (e.g. \\?\ prefix
        // on Windows, symlinks on macOS).
        let test_path = "_test_tmp_write_output.txt";

        let result = tool
            .execute(serde_json::json!({"path": test_path, "content": "hello world"}))
            .await;

        assert!(result.success, "write should succeed: {:?}", result.error);
        assert!(result.output.contains("11 bytes"));

        // Verify the file was written correctly, then clean up.
        if let Ok(content) = std::fs::read_to_string(test_path) {
            assert_eq!(content, "hello world");
        }
        let _ = std::fs::remove_file(test_path);
    }
}
