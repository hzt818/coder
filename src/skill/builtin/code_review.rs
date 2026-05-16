//! Code Review skill - analyzes code for issues and improvements

use async_trait::async_trait;
use serde_json::json;

use super::{Skill, SkillOutput};

/// Skill for reviewing code quality, security, and correctness
#[derive(Default)]
pub struct CodeReviewSkill;

impl CodeReviewSkill {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Skill for CodeReviewSkill {
    fn name(&self) -> &str {
        "code_review"
    }

    fn description(&self) -> &str {
        "Review code for bugs, security issues, performance problems, and quality improvements"
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "The source code to review"
                },
                "language": {
                    "type": "string",
                    "description": "Programming language of the code"
                },
                "focus": {
                    "type": "string",
                    "description": "Review focus: 'security', 'performance', 'quality', or 'all'",
                    "enum": ["security", "performance", "quality", "all"]
                },
                "file_path": {
                    "type": "string",
                    "description": "Optional file path for context"
                }
            },
            "required": ["code"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let code = input
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Code review skill requires a 'code' field"))?;

        let language = input
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let focus = input.get("focus").and_then(|v| v.as_str()).unwrap_or("all");

        let _file_path = input.get("file_path").and_then(|v| v.as_str());

        let line_count = code.lines().count();

        // Get git diff context (best-effort)
        let git_diff = get_git_diff();

        let mut review = String::new();
        review.push_str(&format!("# Code Review\n\n"));
        review.push_str(&format!("- **Language:** {}\n", language));
        review.push_str(&format!("- **Focus:** {}\n", focus));
        review.push_str(&format!("- **Lines reviewed:** {}\n\n", line_count));

        // Include git context if available
        if let Some(ref diff) = git_diff {
            if !diff.is_empty() {
                review.push_str("---\n\n**Working tree changes:**\n");
                review.push_str("```\n");
                review.push_str(diff);
                review.push_str("```\n\n");
            }
        }

        // Quality analysis
        if focus == "all" || focus == "quality" {
            review.push_str("## Quality Analysis\n\n");

            let long_lines = code.lines().filter(|l| l.len() > 120).count();
            if long_lines > 0 {
                review.push_str(&format!(
                    "- **{} line(s)** exceed 120 characters (consider wrapping)\n",
                    long_lines
                ));
            } else {
                review.push_str("- Lines are within reasonable length limits\n");
            }

            let todo_count = code
                .lines()
                .filter(|l| l.contains("TODO") || l.contains("FIXME") || l.contains("HACK"))
                .count();
            if todo_count > 0 {
                review.push_str(&format!(
                    "- **{} TODO/FIXME/HACK comment(s)** found (address before merging)\n",
                    todo_count
                ));
            }

            if line_count > 400 {
                review.push_str(
                    "- **File is long (>400 lines)**; consider splitting into smaller modules\n",
                );
            } else if line_count > 200 {
                review.push_str("- File is moderately long (>200 lines); monitor as it grows\n");
            }

            let blank_lines = code.lines().filter(|l| l.trim().is_empty()).count();
            if line_count > 0 {
                let ratio = blank_lines as f64 / line_count as f64;
                if ratio > 0.3 {
                    review.push_str(&format!(
                        "- **High whitespace ratio ({:.0}%)**; consider reducing vertical spacing\n",
                        ratio * 100.0
                    ));
                }
            }

            review.push_str("\n");
        }

        // Security analysis
        if focus == "all" || focus == "security" {
            review.push_str("## Security Analysis\n\n");

            if code.contains("unsafe") {
                review.push_str(
                    "- **`unsafe` blocks detected** - verify safety invariants are correctly maintained\n",
                );
            }
            if code.contains("unwrap(") {
                review.push_str(
                    "- **`unwrap()` calls detected** - prefer proper error handling with `?` or pattern matching\n",
                );
            }
            if code.contains("expect(") {
                review.push_str(
                    "- **`expect()` calls detected** - ensure panic messages are meaningful or handle errors gracefully\n",
                );
            }
            if code.contains("password") || code.contains("secret") || code.contains("api_key") {
                review.push_str(
                    "- **References to secrets/credentials found** - verify nothing is hardcoded\n",
                );
            }
            if code.contains("eval(") || code.contains("exec(") {
                review.push_str(
                    "- **Dynamic code execution detected** - verify input is properly sanitized\n",
                );
            }

            let none_reviewed = [
                "unsafe", "unwrap(", "expect(", "password", "secret", "eval(", "exec(",
            ]
            .iter()
            .all(|pat| !code.contains(pat));
            if none_reviewed {
                review.push_str(
                    "- No obvious security anti-patterns detected in the provided code\n",
                );
            }

            review.push_str("\n");
        }

        // Performance analysis
        if focus == "all" || focus == "performance" {
            review.push_str("## Performance Analysis\n\n");

            if code.contains("clone(") {
                review.push_str(
                    "- **`.clone()` calls detected** - verify clones are necessary; consider borrowing instead\n",
                );
            }
            if code.contains("to_string(") || code.contains("to_owned(") {
                review.push_str(
                    "- **Allocation calls detected** - consider if owned values are needed or references suffice\n",
                );
            }
            if code.contains("for ") && (code.contains("Vec::new") || code.contains("vec![")) {
                review.push_str(
                    "- **Loops over Vec collections** - consider pre-allocating capacity with `Vec::with_capacity`\n",
                );
            }
            if code.contains("Rc<") || code.contains("RefCell<") {
                review.push_str(
                    "- **Reference counting types detected** - verify `Rc`/`RefCell` is appropriate; prefer `Arc` for threading\n",
                );
            }

            review.push_str("\n");
        }

        // Recommendations
        review.push_str("## Recommendations\n\n");
        review.push_str("1. Run `cargo fmt` to ensure consistent formatting\n");
        review.push_str("2. Run `cargo clippy` for additional lint checks\n");
        review.push_str("3. Ensure adequate test coverage for the logic\n");
        review.push_str("4. Review for any hardcoded configuration values\n\n");

        let output = SkillOutput::ok_with_data(
            review,
            json!({
                "language": language,
                "focus": focus,
                "lines_reviewed": line_count,
            }),
        );

        Ok(output.to_json())
    }
}

/// Run `git diff --stat` to get an overview of working tree changes.
/// Returns None if git is unavailable or there are no changes.
fn get_git_diff() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["diff", "--stat"])
        .output()
        .ok()?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if stdout.is_empty() {
            None
        } else {
            Some(stdout)
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_code_review_execute() {
        let skill = CodeReviewSkill;
        let result = skill
            .execute(json!({
                "code": "fn main() { println!(\"hello\"); }",
                "language": "rust",
                "focus": "quality"
            }))
            .await
            .unwrap();
        assert_eq!(result["success"], true);
        assert!(result["output"].as_str().unwrap().contains("Code Review"));
    }

    #[tokio::test]
    async fn test_code_review_missing_code() {
        let skill = CodeReviewSkill;
        let result = skill.execute(json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_code_review_defaults() {
        let skill = CodeReviewSkill;
        let result = skill.execute(json!({ "code": "x = 1" })).await.unwrap();
        assert_eq!(result["data"]["focus"], "all");
        assert_eq!(result["data"]["language"], "unknown");
    }
}
