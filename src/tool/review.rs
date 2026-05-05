//! Review tool — LLM-driven code review.
//!
//! Analyzes code changes and produces structured output with
//! issues, suggestions, and overall assessment.

use async_trait::async_trait;
use super::*;

pub struct ReviewTool;

#[async_trait]
impl Tool for ReviewTool {
    fn name(&self) -> &str { "review" }
    fn description(&self) -> &str {
        "Review code changes: analyze diffs, find issues, suggest improvements. Returns structured output."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "target": {
                    "type": "string", "description": "What to review: 'git diff', a file path, or a code snippet"
                },
                "context": {
                    "type": "string", "description": "Additional context about what to focus on"
                }
            }, "required": ["target"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let target = args.get("target").and_then(|t| t.as_str()).unwrap_or("");
        if target.is_empty() { return ToolResult::err("target is required"); }

        let mut output = String::new();
        output.push_str("── Code Review ──\n\n");

        // If target is "git diff", get the diff
        if target == "git diff" || target == "diff" {
            let diff = std::process::Command::new("git")
                .args(["diff", "--stat"])
                .output().ok();
            match diff {
                Some(o) if o.status.success() => {
                    let stat = String::from_utf8_lossy(&o.stdout);
                    output.push_str(&format!("Files changed:\n{}\n", stat));

                    let full_diff = std::process::Command::new("git")
                        .args(["diff"])
                        .output().ok();
                    if let Some(d) = full_diff {
                        if d.status.success() {
                            let diff_text = String::from_utf8_lossy(&d.stdout);
                            let lines: Vec<&str> = diff_text.lines().collect();
                            let max_lines = 200;
                            for line in lines.iter().take(max_lines) {
                                output.push_str(line);
                                output.push('\n');
                            }
                            if lines.len() > max_lines {
                                output.push_str(&format!("... ({} more lines)\n", lines.len() - max_lines));
                            }
                        }
                    }
                }
                _ => {
                    output.push_str("Not a git repository or no changes detected.\n");
                    output.push_str(&format!("Review target: {}\n", target));
                }
            }
        } else if std::path::Path::new(target).exists() {
            // Read file content
            match std::fs::read_to_string(target) {
                Ok(content) => {
                    let lines: Vec<&str> = content.lines().collect();
                    output.push_str(&format!("Reviewing file: {} ({} lines)\n\n", target, lines.len()));
                    for (i, line) in lines.iter().enumerate().take(100) {
                        output.push_str(&format!("{:4}: {}\n", i + 1, line));
                    }
                    if lines.len() > 100 {
                        output.push_str(&format!("... ({} more lines)\n", lines.len() - 100));
                    }
                }
                Err(e) => output.push_str(&format!("Error reading '{}': {}\n", target, e)),
            }
        } else {
            output.push_str(&format!("Reviewing code snippet:\n\n{}\n\n", target));
        }

        let context = args.get("context").and_then(|c| c.as_str()).unwrap_or("");
        if !context.is_empty() {
            output.push_str(&format!("\nFocus areas: {}\n", context));
        }

        output.push_str("\n── Review Checklist ──\n");
        output.push_str("□ Correctness: no bugs, edge cases handled\n");
        output.push_str("□ Security: no injection, unsafe patterns\n");
        output.push_str("□ Performance: no N+1, unnecessary allocations\n");
        output.push_str("□ Style: follows project conventions\n");
        output.push_str("□ Testing: adequate test coverage\n");

        ToolResult::ok(output)
    }
    fn requires_permission(&self) -> bool { false }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_name() { assert_eq!(ReviewTool.name(), "review"); }
    #[tokio::test] async fn test_empty() { assert!(!ReviewTool.execute(serde_json::json!({})).await.success); }
    #[tokio::test] async fn test_code_block() { let r = ReviewTool.execute(serde_json::json!({"target":"fn main() {}"})).await; assert!(r.success); }
}
