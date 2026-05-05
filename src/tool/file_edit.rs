//! File edit tool - precise string replacement

use async_trait::async_trait;
use super::*;

pub struct FileEditTool;

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing exact text matches. Use this for surgical edits instead of rewriting entire files."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "The exact text to find and replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "The new text to insert"
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let path = args.get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return ToolResult::err("Path is required");
        }

        let old_string = args.get("old_string")
            .and_then(|o| o.as_str())
            .unwrap_or("");

        let new_string = args.get("new_string")
            .and_then(|n| n.as_str())
            .unwrap_or("");

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return ToolResult::err(format!("Failed to read file '{}': {}", path, e)),
        };

        if !content.contains(old_string) {
            return ToolResult::err(format!("Could not find old_string in '{}'", path));
        }

        let new_content = content.replace(old_string, new_string);
        let count = content.matches(old_string).count();

        match std::fs::write(path, &new_content) {
            Ok(_) => ToolResult::ok(format!(
                "Successfully replaced {} occurrence(s) in {}",
                count, path
            )),
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
    fn test_file_edit_tool_name() {
        let tool = FileEditTool;
        assert_eq!(tool.name(), "file_edit");
    }

    #[tokio::test]
    async fn test_file_edit_success() {
        let tool = FileEditTool;
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("test.txt");
        std::fs::write(&file_path, "Hello, world!").unwrap();
        let path_str = file_path.to_str().unwrap();

        let result = tool.execute(serde_json::json!({
            "path": path_str,
            "old_string": "world",
            "new_string": "Rust"
        })).await;
        assert!(result.success);

        let content = std::fs::read_to_string(path_str).unwrap();
        assert_eq!(content, "Hello, Rust!");
    }

    #[tokio::test]
    async fn test_file_edit_not_found() {
        let tool = FileEditTool;
        let result = tool.execute(serde_json::json!({
            "path": "nonexistent.txt",
            "old_string": "hello",
            "new_string": "world"
        })).await;
        assert!(!result.success);
    }
}
