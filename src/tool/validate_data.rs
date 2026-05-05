//! Validate Data tool — validate JSON or TOML content.
//!
//! Accepts inline content or a file path. Auto-detects format from extension.

use async_trait::async_trait;
use super::*;
use std::path::Path;

pub struct ValidateDataTool;

#[async_trait]
impl Tool for ValidateDataTool {
    fn name(&self) -> &str { "validate_data" }
    fn description(&self) -> &str {
        "Validate JSON or TOML content from inline input or a workspace file."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "content": { "type": "string", "description": "Inline content to validate" },
                "path": { "type": "string", "description": "Path to a file to validate" },
                "format": { "type": "string", "enum": ["auto", "json", "toml"], "default": "auto" }
            }, "additionalProperties": false
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let content = args.get("content").and_then(|c| c.as_str()).unwrap_or("");
        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
        let format = args.get("format").and_then(|f| f.as_str()).unwrap_or("auto");

        let (data, source) = if !content.is_empty() {
            (content.to_string(), "inline".to_string())
        } else if !path.is_empty() {
            match std::fs::read_to_string(Path::new(path)) {
                Ok(s) => (s, format!("file '{}'", path)),
                Err(e) => return ToolResult::err(format!("Failed to read '{}': {}", path, e)),
            }
        } else {
            return ToolResult::err("Either 'content' or 'path' is required");
        };

        let format = if format == "auto" {
            if path.ends_with(".json") { "json" }
            else if path.ends_with(".toml") { "toml" }
            else { "json" } // default
        } else { format };

        match format {
            "json" => {
                match serde_json::from_str::<serde_json::Value>(&data) {
                    Ok(val) => {
                        let keys = match &val {
                            serde_json::Value::Object(m) => format!("{} top-level keys", m.len()),
                            serde_json::Value::Array(a) => format!("{} items", a.len()),
                            _ => "single value".to_string(),
                        };
                        ToolResult::ok(format!("✅ Valid JSON from {}\nType: {}\nSize: {} bytes", source, keys, data.len()))
                    }
                    Err(e) => {
                        let (line, col) = extract_json_error_pos(&e);
                        ToolResult::err(format!("❌ Invalid JSON ({}:{}): {}\nFrom: {}", line, col, e, source))
                    }
                }
            }
            "toml" => {
                match toml::from_str::<toml::Value>(&data) {
                    Ok(val) => {
                        let keys = match &val {
                            toml::Value::Table(t) => format!("{} top-level keys", t.len()),
                            _ => "single value".to_string(),
                        };
                        ToolResult::ok(format!("✅ Valid TOML from {}\nType: {}\nSize: {} bytes", source, keys, data.len()))
                    }
                    Err(e) => ToolResult::err(format!("❌ Invalid TOML: {}\nFrom: {}", e, source)),
                }
            }
            _ => ToolResult::err(format!("Unsupported format: '{}'", format)),
        }
    }
    fn requires_permission(&self) -> bool { false }
}

fn extract_json_error_pos(e: &serde_json::Error) -> (usize, usize) {
    let line = e.line();
    let col = e.column();
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_name() { assert_eq!(ValidateDataTool.name(), "validate_data"); }

    #[tokio::test] async fn test_valid_json() {
        let r = ValidateDataTool.execute(serde_json::json!({"content": "{\"key\": \"value\"}"})).await;
        assert!(r.success, "{}", r.error.as_deref().unwrap_or(""));
        assert!(r.output.contains("Valid JSON"));
    }

    #[tokio::test] async fn test_invalid_json() {
        let r = ValidateDataTool.execute(serde_json::json!({"content": "{invalid}"})).await;
        assert!(!r.success);
    }

    #[tokio::test] async fn test_valid_toml() {
        let r = ValidateDataTool.execute(serde_json::json!({"content": "key = \"value\"", "format": "toml"})).await;
        assert!(r.success);
    }

    #[tokio::test] async fn test_no_input() {
        let r = ValidateDataTool.execute(serde_json::json!({})).await;
        assert!(!r.success);
    }

    #[test] fn test_json_error_pos() {
        let e = serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
        let (line, col) = extract_json_error_pos(&e);
        assert!(line >= 1);
    }
}
