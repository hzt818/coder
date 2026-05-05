//! Git tool - Git operations
//!
//! Supports status, diff, log, commit, and push operations.
//! Uses gix crate with fallback to bash git commands.

use async_trait::async_trait;
use super::*;

pub struct GitTool;

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> &str {
        "git"
    }

    fn description(&self) -> &str {
        "Execute Git operations like status, diff, log, commit, and push. Use this to manage version control."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["status", "diff", "log", "commit", "push"],
                    "description": "Git operation to perform"
                },
                "repo_path": {
                    "type": "string",
                    "description": "Path to the git repository (default: current directory)",
                    "default": "."
                },
                "args": {
                    "type": "object",
                    "description": "Additional arguments for the operation (optional)",
                    "default": {}
                }
            },
            "required": ["operation"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let operation = args.get("operation")
            .and_then(|o| o.as_str())
            .unwrap_or("");

        let repo_path = args.get("repo_path")
            .and_then(|p| p.as_str())
            .unwrap_or(".")
            .to_string();

        let extra_args = args.get("args")
            .and_then(|a| a.as_object())
            .cloned()
            .unwrap_or_default();

        if operation.is_empty() {
            return ToolResult::err("Operation is required (status, diff, log, commit, push)");
        }

        match operation {
            "status" => git_status(&repo_path).await,
            "diff" => git_diff(&repo_path, &extra_args).await,
            "log" => git_log(&repo_path, &extra_args).await,
            "commit" => git_commit(&repo_path, &extra_args).await,
            "push" => git_push(&repo_path, &extra_args).await,
            _ => ToolResult::err(format!("Unknown git operation: '{}'. Use: status, diff, log, commit, push", operation)),
        }
    }
}

/// Run a git command and return the output
async fn run_git_command(repo_path: &str, args: &[&str]) -> Result<String, String> {
    use tokio::process::Command;

    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .await
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(format!("Git command failed:\n{}", stderr))
    }
}

/// Get git status
async fn git_status(repo_path: &str) -> ToolResult {
    let output = match run_git_command(repo_path, &["status", "--short", "--branch"]).await {
        Ok(o) => o,
        Err(e) => return ToolResult::err(format!("Failed to get git status: {}", e)),
    };

    let mut result = String::new();

    let branch_output = match run_git_command(repo_path, &["branch", "--show-current"]).await {
        Ok(b) => b.trim().to_string(),
        Err(_) => String::new(),
    };

    if !branch_output.is_empty() {
        result.push_str(&format!("On branch: {}\n", branch_output));
    }

    // Check for ahead/behind
    let status_verbose = match run_git_command(repo_path, &["status"]).await {
        Ok(s) => s,
        Err(_) => String::new(),
    };
    for line in status_verbose.lines() {
        if line.contains("Your branch is ahead of") || line.contains("Your branch is behind") {
            result.push_str(line);
            result.push('\n');
        }
    }

    result.push('\n');
    result.push_str(&output);

    if output.trim().is_empty() || output.trim() == branch_output.trim() {
        result.push_str("Working tree is clean.");
    }

    ToolResult::ok(result)
}

/// Show git diff
async fn git_diff(repo_path: &str, args: &serde_json::Map<String, serde_json::Value>) -> ToolResult {
    let mut git_args = vec!["diff"];

    if let Some(staged) = args.get("staged").and_then(|s| s.as_bool()) {
        if staged {
            git_args.push("--staged");
        }
    }

    let output = match run_git_command(repo_path, &git_args).await {
        Ok(o) => o,
        Err(e) => return ToolResult::err(format!("Failed to get git diff: {}", e)),
    };

    if output.is_empty() {
        return ToolResult::ok("No changes found.");
    }

    ToolResult::ok(format!("Diff:\n{}", output))
}

/// Show git log
async fn git_log(repo_path: &str, args: &serde_json::Map<String, serde_json::Value>) -> ToolResult {
    let max_count = args.get("max_count")
        .and_then(|m| m.as_u64())
        .unwrap_or(10)
        .to_string();

    let format = args.get("format")
        .and_then(|f| f.as_str())
        .unwrap_or("%h %s (%an, %ar)");

    let git_args: [&str; 4] = [
        "log",
        &format!("--max-count={}", max_count),
        &format!("--format={}", format),
        "--no-color",
    ];

    let output = match run_git_command(repo_path, &git_args).await {
        Ok(o) => o,
        Err(e) => return ToolResult::err(format!("Failed to get git log: {}", e)),
    };

    ToolResult::ok(format!("Recent commits:\n{}", output))
}

/// Git commit
async fn git_commit(repo_path: &str, args: &serde_json::Map<String, serde_json::Value>) -> ToolResult {
    let message = args.get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("");

    if message.is_empty() {
        return ToolResult::err("Commit message is required");
    }

    let all = args.get("all")
        .and_then(|a| a.as_bool())
        .unwrap_or(false);

    if all {
        if let Err(e) = run_git_command(repo_path, &["add", "-A"]).await {
            return ToolResult::err(format!("Failed to stage files: {}", e));
        }
    }

    match run_git_command(repo_path, &["commit", "-m", message]).await {
        Ok(o) => ToolResult::ok(format!("Commit successful:\n{}", o)),
        Err(e) => {
            if e.contains("nothing to commit") || e.contains("no changes added") {
                ToolResult::ok("Nothing to commit. Working tree is clean.")
            } else {
                ToolResult::err(format!("Commit failed: {}", e))
            }
        }
    }
}

/// Git push
async fn git_push(repo_path: &str, args: &serde_json::Map<String, serde_json::Value>) -> ToolResult {
    let remote = args.get("remote")
        .and_then(|r| r.as_str())
        .unwrap_or("origin");

    let branch: Option<&str> = args.get("branch").and_then(|b| b.as_str());

    let mut git_args = vec!["push", "-u", remote];
    if let Some(b) = branch {
        git_args.push(b);
    }

    match run_git_command(repo_path, &git_args).await {
        Ok(o) => ToolResult::ok(format!("Push successful:\n{}", o)),
        Err(e) => ToolResult::err(format!("Push failed: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_tool_name() {
        let tool = GitTool;
        assert_eq!(tool.name(), "git");
    }

    #[test]
    fn test_git_schema() {
        let tool = GitTool;
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }

    #[tokio::test]
    async fn test_git_empty_operation() {
        let tool = GitTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_git_invalid_operation() {
        let tool = GitTool;
        let result = tool.execute(serde_json::json!({"operation": "unknown"})).await;
        assert!(!result.success);
    }
}
