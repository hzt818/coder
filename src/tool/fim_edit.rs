//! FIM (fill-in-the-middle) edit tool
//!
//! Provides surgical code edits by splitting a file at a given position
//! and generating the middle section. Works without an API call by using
//! context-aware heuristics.

use async_trait::async_trait;
use super::*;

pub struct FimEditTool;

#[async_trait]
impl Tool for FimEditTool {
    fn name(&self) -> &str {
        "fim_edit"
    }

    fn description(&self) -> &str {
        concat!(
            "Fill-in-the-middle edit. Specify file path, cursor position, ",
            "and optional instructions. Analyzes context to generate code."
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "line": {
                    "type": "integer",
                    "description": "Line number where edit should be inserted (0-indexed)"
                },
                "instructions": {
                    "type": "string",
                    "description": "Optional instructions for what to generate",
                    "default": ""
                }
            },
            "required": ["path", "line"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
        if path.is_empty() {
            return ToolResult::err("Path is required");
        }

        let line = args.get("line").and_then(|l| l.as_i64()).unwrap_or(-1);
        if line < 0 {
            return ToolResult::err("Line must be a non-negative integer");
        }
        let line_idx = line as usize;

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return ToolResult::err(format!("Failed to read '{}': {}", path, e)),
        };

        let lines: Vec<&str> = content.lines().collect();
        if line_idx > lines.len() {
            return ToolResult::err(format!("Line {} beyond file length {}", line_idx, lines.len()));
        }

        let prefix = if line_idx > 0 { lines[..line_idx].join("\n") } else { String::new() };
        let suffix = if line_idx < lines.len() { lines[line_idx..].join("\n") } else { String::new() };

        let instructions = args.get("instructions").and_then(|i| i.as_str()).unwrap_or("");

        // Generate the middle section using heuristic-based FIM
        let generated = fim_simple(&prefix, &suffix, "", instructions);

        if generated.is_empty() {
            return ToolResult::err("Failed to generate meaningful content");
        }

        // Assemble the final content: prefix + generated + suffix
        let new_content = if prefix.is_empty() {
            format!("{}\n{}", generated, suffix)
        } else if suffix.is_empty() {
            format!("{}\n{}", prefix, generated)
        } else {
            format!("{}\n{}\n{}", prefix, generated, suffix)
        };

        // Write the modified content back to the file
        match std::fs::write(path, &new_content) {
            Ok(_) => ToolResult::ok(format!(
                "FIM edit applied at {}:{}\nGenerated: {}",
                path,
                line_idx + 1,
                generated,
            )),
            Err(e) => ToolResult::err(format!("Failed to write '{}': {}", path, e)),
        }
    }

    fn requires_permission(&self) -> bool {
        true
    }
}

/// Forward fill-in-the-middle that generates code from context without an API call.
///
/// Uses heuristics and naming conventions to produce reasonable completions
/// for common code patterns such as function bodies, variable declarations,
/// and return statements.
pub fn fim_simple(prefix: &str, suffix: &str, middle: &str, instructions: &str) -> String {
    if !instructions.is_empty() {
        let placeholder = if middle.is_empty() { "unimplemented!()" } else { middle };
        return format!("// TODO: {}\n{}", instructions, placeholder);
    }

    if !middle.is_empty() {
        return middle.to_string();
    }

    // Try function body completion (prefix ends with `{`, suffix starts with `}`)
    let trimmed_prefix = prefix.trim_end();
    let trimmed_suffix = suffix.trim_start();
    if trimmed_prefix.ends_with('{') && trimmed_suffix.starts_with('}') {
        if let Some(sig_line) = trimmed_prefix.lines().last() {
            if sig_line.contains("->") {
                return String::from("unimplemented!()");
            }
        }
        return String::from("// TODO: implement");
    }

    // Try variable declaration completion (`let x = ` or `let x: Type = `)
    if trimmed_prefix.ends_with('=') {
        if let Some(after_colon) = prefix.trim().splitn(2, ':').nth(1) {
            let type_str = after_colon.trim_end_matches('=').trim();
            if !type_str.is_empty() && !type_str.contains(' ') {
                let default = match type_str {
                    "String" | "&str" => "String::new()",
                    "bool" => "false",
                    "i32" | "i64" | "u32" | "u64" | "isize" | "usize" => "0",
                    "f32" | "f64" => "0.0",
                    "Vec" | "Vec<T>" => "Vec::new()",
                    "Option" | "Option<T>" => "None",
                    _ => "Default::default()",
                };
                return String::from(default);
            }
        }
        return String::from("Default::default()");
    }

    // Try return statement completion
    if trimmed_prefix.ends_with("return") || trimmed_prefix.ends_with("return ") {
        return String::from("Default::default();");
    }

    "unimplemented!()".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fim_tool_name() {
        let tool = FimEditTool;
        assert_eq!(tool.name(), "fim_edit");
    }

    #[test]
    fn test_fim_schema() {
        let tool = FimEditTool;
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        let props = schema.get("properties").unwrap();
        assert!(props.get("path").is_some());
        assert!(props.get("line").is_some());
        assert!(props.get("instructions").is_some());

        let required = schema.get("required").unwrap().as_array().unwrap();
        let names: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(names.contains(&"path"));
        assert!(names.contains(&"line"));
    }

    #[tokio::test]
    async fn test_fim_empty_path() {
        let tool = FimEditTool;
        assert!(!tool.execute(serde_json::json!({"line": 0})).await.success);
    }

    #[tokio::test]
    async fn test_fim_negative_line() {
        let tool = FimEditTool;
        assert!(!tool.execute(serde_json::json!({"path": "f", "line": -1})).await.success);
    }

    #[tokio::test]
    async fn test_fim_valid() {
        let tool = FimEditTool;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "a\nb\nc\n").unwrap();
        let r = tool.execute(serde_json::json!({"path": tmp.path(), "line": 1})).await;
        assert!(r.success);
    }

    #[test]
    fn test_fim_simple_function_with_return() {
        let result = fim_simple("fn add(a: i32, b: i32) -> i32 {", "}", "", "");
        assert!(result.contains("unimplemented"));
    }

    #[test]
    fn test_fim_simple_void_function() {
        let result = fim_simple("fn greet() {", "}", "", "");
        assert!(result.contains("TODO"));
    }

    #[test]
    fn test_fim_simple_with_instructions() {
        let result = fim_simple("fn calc() -> i32 {", "}", "", "return 42");
        assert!(result.contains("TODO"));
        assert!(result.contains("return 42"));
    }

    #[test]
    fn test_fim_simple_preserves_middle() {
        let result = fim_simple("fn foo() {", "}", "let x = 1;", "");
        assert_eq!(result, "let x = 1;");
    }

    #[test]
    fn test_fim_simple_variable() {
        let result = fim_simple("let name: String = ", ";", "", "");
        assert_eq!(result, "String::new()");
    }

    #[test]
    fn test_fim_simple_default_variable() {
        let result = fim_simple("let x = ", ";", "", "");
        assert_eq!(result, "Default::default()");
    }

    #[test]
    fn test_fim_simple_return() {
        let result = fim_simple("    return", ";", "", "");
        assert!(result.contains("Default::default"));
    }

    #[test]
    fn test_fim_simple_fallback() {
        let result = fim_simple("fn foo(", "}", "", "");
        assert!(result.contains("unimplemented"));
    }
}
