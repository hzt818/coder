//! Git tool - Git operations
//!
//! Supports status, diff, log, commit, and push operations.
//! Uses gix crate with fallback to bash git commands.

use super::*;
use async_trait::async_trait;

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
                    "enum": ["status", "diff", "log", "commit", "push", "blame", "branch", "show"],
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
        let operation = args.get("operation").and_then(|o| o.as_str()).unwrap_or("");

        let repo_path = args
            .get("repo_path")
            .and_then(|p| p.as_str())
            .unwrap_or(".")
            .to_string();

        let extra_args = args
            .get("args")
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
            "blame" => git_blame(&repo_path, &extra_args).await,
            "branch" => git_branch(&repo_path, &extra_args).await,
            "show" => git_show(&repo_path, &extra_args).await,
            _ => ToolResult::err(format!("Unknown git operation: '{}'. Use: status, diff, log, commit, push, blame, branch, show", operation)),
        }
    }

    fn requires_permission(&self) -> bool {
        true
    }
}

/// Run a git command and return the output
async fn run_git_command(repo_path: &str, args: &[&str]) -> Result<String, String> {
    use tokio::process::Command;

    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .env("LC_ALL", "C")
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
async fn git_diff(
    repo_path: &str,
    args: &serde_json::Map<String, serde_json::Value>,
) -> ToolResult {
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
    let max_count = args
        .get("max_count")
        .and_then(|m| m.as_u64())
        .unwrap_or(10)
        .to_string();

    let format = args
        .get("format")
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
async fn git_commit(
    repo_path: &str,
    args: &serde_json::Map<String, serde_json::Value>,
) -> ToolResult {
    let message = args.get("message").and_then(|m| m.as_str()).unwrap_or("");

    if message.is_empty() {
        return ToolResult::err("Commit message is required");
    }

    let all = args.get("all").and_then(|a| a.as_bool()).unwrap_or(false);

    if all {
        // Stage tracked files with modifications (-u), avoids staging untracked files
        if let Err(e) = run_git_command(repo_path, &["add", "-u"]).await {
            return ToolResult::err(format!("Failed to stage changes: {}", e));
        }
    }

    // Check porcelain before committing to avoid fragile error-message parsing
    match run_git_command(repo_path, &["status", "--porcelain"]).await {
        Ok(porcelain) if porcelain.trim().is_empty() => {
            return ToolResult::ok("Nothing to commit. Working tree is clean.");
        }
        _ => {} // has changes — proceed
    }

    match run_git_command(repo_path, &["commit", "-m", message]).await {
        Ok(o) => ToolResult::ok(format!("Commit successful:\n{}", o)),
        Err(e) => ToolResult::err(format!("Commit failed: {}", e)),
    }
}

/// Git push
async fn git_push(
    repo_path: &str,
    args: &serde_json::Map<String, serde_json::Value>,
) -> ToolResult {
    let remote = args
        .get("remote")
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

/// Git blame
async fn git_blame(
    repo_path: &str,
    args: &serde_json::Map<String, serde_json::Value>,
) -> ToolResult {
    let file = args.get("file").and_then(|f| f.as_str()).unwrap_or("");

    if file.is_empty() {
        return ToolResult::err("'file' argument is required for blame");
    }

    let git_args = ["blame", "--line-porcelain", file];

    let output = match run_git_command(repo_path, &git_args).await {
        Ok(o) => o,
        Err(e) => return ToolResult::err(format!("Failed to get git blame: {}", e)),
    };

    // For line-porcelain output, extract summary
    let mut summary = String::new();
    summary.push_str(&format!("Blame for file: {}\n\n", file));

    // Extract key info from porcelain output
    for line in output.lines() {
        if line.starts_with("author ") {
            summary.push_str(&format!("Author: {}\n", &line[7..]));
        } else if line.starts_with("author-time ") {
            let timestamp: i64 = line[12..].parse().unwrap_or(0);
            let naive = chrono::DateTime::from_timestamp(timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            summary.push_str(&format!("Date: {}\n", naive));
        } else if line.starts_with("summary ") {
            summary.push_str(&format!("Summary: {}\n", &line[8..]));
        } else if line.starts_with('\t') {
            summary.push_str(&format!("  {}\n", line));
        }
    }

    ToolResult::ok(summary)
}

/// Git branch operations
async fn git_branch(
    repo_path: &str,
    args: &serde_json::Map<String, serde_json::Value>,
) -> ToolResult {
    let list = args.get("list").and_then(|l| l.as_bool()).unwrap_or(true); // Default to listing

    if list {
        let git_args = ["branch", "--all"];
        let output = match run_git_command(repo_path, &git_args).await {
            Ok(o) => o,
            Err(e) => return ToolResult::err(format!("Failed to list branches: {}", e)),
        };

        let mut result = String::from("Branches:\n");
        result.push_str(&output);

        // Show current branch
        let current = match run_git_command(repo_path, &["branch", "--show-current"]).await {
            Ok(b) => b.trim().to_string(),
            Err(_) => String::new(),
        };
        if !current.is_empty() {
            result.push_str(&format!("\nCurrent branch: {}", current));
        }

        ToolResult::ok(result)
    } else {
        let delete = args.get("delete").and_then(|d| d.as_str()).unwrap_or("");
        let new_branch = args.get("create").and_then(|c| c.as_str()).unwrap_or("");

        if !delete.is_empty() {
            let git_args = ["branch", "-d", delete];
            match run_git_command(repo_path, &git_args).await {
                Ok(o) => ToolResult::ok(format!("Deleted branch '{}':\n{}", delete, o)),
                Err(e) => ToolResult::err(format!("Failed to delete branch '{}': {}", delete, e)),
            }
        } else if !new_branch.is_empty() {
            let git_args = ["branch", new_branch];
            match run_git_command(repo_path, &git_args).await {
                Ok(o) => ToolResult::ok(format!("Created branch '{}':\n{}", new_branch, o)),
                Err(e) => {
                    ToolResult::err(format!("Failed to create branch '{}': {}", new_branch, e))
                }
            }
        } else {
            ToolResult::err("Specify --delete <name> or --create <name> for branch operation")
        }
    }
}

/// Git show (view commit details)
async fn git_show(
    repo_path: &str,
    args: &serde_json::Map<String, serde_json::Value>,
) -> ToolResult {
    let revision = args
        .get("revision")
        .and_then(|r| r.as_str())
        .unwrap_or("HEAD");

    let format = args
        .get("format")
        .and_then(|f| f.as_str())
        .unwrap_or("%H%n%an%n%ae%n%ad%n%s%n%b");

    let git_args = [
        "show",
        "--no-color",
        &format!("--format={}", format),
        revision,
    ];

    let output = match run_git_command(repo_path, &git_args).await {
        Ok(o) => o,
        Err(e) => return ToolResult::err(format!("Failed to show revision '{}': {}", revision, e)),
    };

    ToolResult::ok(format!("Commit {}:\n{}", revision, output))
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
    async fn test_git_blame_no_file() {
        let tool = GitTool;
        let result = tool
            .execute(serde_json::json!({"operation": "blame"}))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_git_invalid_operation() {
        let tool = GitTool;
        let result = tool
            .execute(serde_json::json!({"operation": "unknown"}))
            .await;
        assert!(!result.success);
    }
}
