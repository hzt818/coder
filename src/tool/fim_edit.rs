//! FIM (fill-in-the-middle) edit tool
//!
//! Provides surgical code edits by splitting a file at a given position
//! and generating the middle section using an AI provider's chat completion
//! with a FIM-specific prompt. Falls back to heuristic completion when
//! no AI provider is configured.

use async_trait::async_trait;
use super::*;

pub struct FimEditTool;

#[async_trait]
impl Tool for FimEditTool {
    fn name(&self) -> &str { "fim_edit" }

    fn description(&self) -> &str {
        concat!(
            "Fill-in-the-middle edit. Specify file path, cursor position, ",
            "and optional instructions. Uses AI to generate context-aware code. ",
            "Falls back to heuristics when no API key is available."
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to edit" },
                "line": { "type": "integer", "description": "Line number where edit should be inserted (0-indexed)" },
                "instructions": { "type": "string", "description": "Optional instructions for what to generate", "default": "" }
            },
            "required": ["path", "line"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
        if path.is_empty() { return ToolResult::err("Path is required"); }

        let line = args.get("line").and_then(|l| l.as_i64()).unwrap_or(-1);
        if line < 0 { return ToolResult::err("Line must be a non-negative integer"); }
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

        // Try AI-based FIM first, fall back to heuristic
        let generated = match ai_fim_completion(&prefix, &suffix, instructions).await {
            Some(code) => code,
            None => fim_simple(&prefix, &suffix, "", instructions),
        };

        if generated.is_empty() {
            return ToolResult::err("Failed to generate meaningful content");
        }

        // Assemble: prefix + generated + suffix
        let new_content = if prefix.is_empty() {
            format!("{}\n{}", generated, suffix)
        } else if suffix.is_empty() {
            format!("{}\n{}", prefix, generated)
        } else {
            format!("{}\n{}\n{}", prefix, generated, suffix)
        };

        match std::fs::write(path, &new_content) {
            Ok(_) => ToolResult::ok(format!(
                "FIM edit applied at {}:{}\n--- Generated ---\n{}\n---",
                path, line_idx + 1, generated,
            )),
            Err(e) => ToolResult::err(format!("Failed to write '{}': {}", path, e)),
        }
    }

    fn requires_permission(&self) -> bool { true }
}

/// Attempt AI-powered FIM completion using the configured API key.
/// Returns `None` if no API key is available or the call fails.
async fn ai_fim_completion(prefix: &str, suffix: &str, instructions: &str) -> Option<String> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("CODER_API_KEY"))
        .ok()?;

    let base_url = std::env::var("OPENAI_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let model = std::env::var("CODER_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    let system_msg = "You are a code completion engine. Given the code BEFORE and AFTER the cursor position, generate the code that should go BETWEEN them. Only output the generated code, no explanations, no markdown fences.";
    let user_msg = if instructions.is_empty() {
        format!(
            "BEFORE (what comes before the cursor):\n```\n{}\n```\n\nAFTER (what comes after the cursor):\n```\n{}\n```\n\nGenerate the code that should be inserted at the cursor position.",
            prefix, suffix
        )
    } else {
        format!(
            "BEFORE:\n```\n{}\n```\n\nAFTER:\n```\n{}\n```\n\nInstructions: {}\n\nGenerate the code that should be inserted at the cursor position.",
            prefix, suffix, instructions
        )
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build().ok()?;

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_msg},
            {"role": "user", "content": user_msg}
        ],
        "max_tokens": 2048,
        "temperature": 0.2,
    });

    let resp = client
        .post(&format!("{}/chat/completions", base_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await.ok()?;

    if !resp.status().is_success() { return None; }

    let json: serde_json::Value = resp.json().await.ok()?;
    let text = json["choices"][0]["message"]["content"].as_str()?;
    let text = text.trim();

    // Clean up markdown fences if the AI added them
    let text = text.strip_prefix("```").and_then(|t| {
        t.split_once('\n').map(|(_, rest)| rest)
    }).unwrap_or(text);
    let text = text.strip_suffix("```").unwrap_or(text);
    let text = text.strip_prefix("```rust").and_then(|t| {
        t.split_once('\n').map(|(_, rest)| rest)
    }).unwrap_or(text);

    Some(text.trim().to_string())
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

    if trimmed_prefix.ends_with("return") || trimmed_prefix.ends_with("return ") {
        return String::from("Default::default();");
    }

    "unimplemented!()".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_fim_tool_name() { assert_eq!(FimEditTool.name(), "fim_edit"); }

    #[test] fn test_fim_schema() {
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

    #[tokio::test] async fn test_fim_empty_path() {
        assert!(!FimEditTool.execute(serde_json::json!({"line": 0})).await.success);
    }

    #[tokio::test] async fn test_fim_negative_line() {
        assert!(!FimEditTool.execute(serde_json::json!({"path": "f", "line": -1})).await.success);
    }

    #[tokio::test] async fn test_fim_valid() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "a\nb\nc\n").unwrap();
        let r = FimEditTool.execute(serde_json::json!({"path": tmp.path(), "line": 1})).await;
        assert!(r.success);
    }

    #[test] fn test_fim_simple_function_with_return() {
        let result = fim_simple("fn add(a: i32, b: i32) -> i32 {", "}", "", "");
        assert!(result.contains("unimplemented"));
    }
    #[test] fn test_fim_simple_void_function() {
        let result = fim_simple("fn greet() {", "}", "", "");
        assert!(result.contains("TODO"));
    }
    #[test] fn test_fim_simple_with_instructions() {
        let result = fim_simple("fn calc() -> i32 {", "}", "", "return 42");
        assert!(result.contains("TODO") && result.contains("return 42"));
    }
    #[test] fn test_fim_simple_variable() {
        let result = fim_simple("let name: String = ", ";", "", "");
        assert_eq!(result, "String::new()");
    }
    #[test] fn test_fim_simple_fallback() {
        let result = fim_simple("fn foo(", "}", "", "");
        assert!(result.contains("unimplemented"));
    }
}
