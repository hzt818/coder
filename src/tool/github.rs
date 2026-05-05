//! GitHub tools - issue/PR/comment operations
//!
//! Provides read-only context tools and guarded write operations
//! for GitHub Issues and Pull Requests, backed by the `gh` CLI.

use async_trait::async_trait;
use super::*;

pub struct GitHubTool;

#[async_trait]
impl Tool for GitHubTool {
    fn name(&self) -> &str {
        "github"
    }

    fn description(&self) -> &str {
        concat!(
            "GitHub operations: read issue/PR context, post comments, ",
            "and close issues. Uses the 'gh' CLI for authentication. ",
            "Write operations require user approval."
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": [
                        "issue_context", "pr_context",
                        "comment", "close_issue",
                        "list_issues", "list_prs"
                    ],
                    "description": "GitHub operation"
                },
                "repo": {
                    "type": "string",
                    "description": "Repository in 'owner/repo' format (default: auto-detect)"
                },
                "number": {
                    "type": "integer",
                    "description": "Issue or PR number"
                },
                "body": {
                    "type": "string",
                    "description": "Comment body or issue close message"
                },
                "state": {
                    "type": "string",
                    "description": "Filter by state (open/closed/all)",
                    "default": "open"
                }
            },
            "required": ["operation"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let operation = args
            .get("operation")
            .and_then(|o| o.as_str())
            .unwrap_or("");

        let repo = args
            .get("repo")
            .and_then(|r| r.as_str())
            .unwrap_or("")
            .to_string();

        if operation.is_empty() {
            return ToolResult::err("Operation is required (issue_context, pr_context, comment, close_issue, list_issues, list_prs)");
        }

        match operation {
            "issue_context" => {
                let number = args.get("number").and_then(|n| n.as_i64()).unwrap_or(0);
                if number == 0 {
                    return ToolResult::err("Issue number is required");
                }
                github_issue_context(&repo, number)
            }
            "pr_context" => {
                let number = args.get("number").and_then(|n| n.as_i64()).unwrap_or(0);
                if number == 0 {
                    return ToolResult::err("PR number is required");
                }
                github_pr_context(&repo, number)
            }
            "comment" => {
                let number = args.get("number").and_then(|n| n.as_i64()).unwrap_or(0);
                let body = args.get("body").and_then(|b| b.as_str()).unwrap_or("");
                if number == 0 || body.is_empty() {
                    return ToolResult::err("Both 'number' and 'body' are required for comment");
                }
                github_comment(&repo, number, body)
            }
            "close_issue" => {
                let number = args.get("number").and_then(|n| n.as_i64()).unwrap_or(0);
                let body = args.get("body").and_then(|b| b.as_str()).unwrap_or("Closed.");
                if number == 0 {
                    return ToolResult::err("Issue number is required");
                }
                github_close_issue(&repo, number, body)
            }
            "list_issues" => {
                let state = args.get("state").and_then(|s| s.as_str()).unwrap_or("open");
                github_list_issues(&repo, state)
            }
            "list_prs" => {
                let state = args.get("state").and_then(|s| s.as_str()).unwrap_or("open");
                github_list_prs(&repo, state)
            }
            _ => ToolResult::err(format!(
                "Unknown operation: '{}'. Valid: issue_context, pr_context, comment, close_issue, list_issues, list_prs",
                operation
            )),
        }
    }
}

/// Run a gh CLI command and return stdout
fn run_gh(args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("gh")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run 'gh' CLI: {}. Is GitHub CLI installed?", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh command failed: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Detect the current GitHub repo from git remote
fn detect_repo() -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(|e| format!("Failed to detect git remote: {}", e))?;

    if !output.status.success() {
        return Err("Not a git repository or no remote 'origin'".to_string());
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Parse GitHub URL formats
    if url.contains("github.com") {
        let parts: Vec<&str> = url.split("github.com").collect();
        if parts.len() > 1 {
            let repo = parts[1]
                .trim_start_matches(':')
                .trim_start_matches('/')
                .trim_end_matches(".git")
                .to_string();
            if !repo.is_empty() {
                return Ok(repo);
            }
        }
    }

    Err(format!("Could not parse GitHub repo from remote: {}", url))
}

/// Resolve repo: use provided or auto-detect
fn resolve_repo(repo: &str) -> Result<String, String> {
    if repo.is_empty() {
        detect_repo()
    } else {
        Ok(repo.to_string())
    }
}

/// Get issue context
fn github_issue_context(repo: &str, number: i64) -> ToolResult {
    let repo = match resolve_repo(repo) {
        Ok(r) => r,
        Err(e) => return ToolResult::err(e),
    };

    let output = match run_gh(&["issue", "view", &number.to_string(), "--repo", &repo]) {
        Ok(o) => o,
        Err(e) => return ToolResult::err(format!("Failed to get issue: {}", e)),
    };

    let mut result = format!("Issue #{} in {}\n\n{}", number, repo, output);

    // Try to get comments
    if let Ok(comments) = run_gh(&[
        "issue",
        "view",
        &number.to_string(),
        "--repo",
        &repo,
        "--comments",
    ]) {
        result.push_str("\n── Comments ──\n");
        result.push_str(&comments);
    }

    ToolResult::ok(result)
}

/// Get PR context
fn github_pr_context(repo: &str, number: i64) -> ToolResult {
    let repo = match resolve_repo(repo) {
        Ok(r) => r,
        Err(e) => return ToolResult::err(e),
    };

    let output = match run_gh(&["pr", "view", &number.to_string(), "--repo", &repo]) {
        Ok(o) => o,
        Err(e) => return ToolResult::err(format!("Failed to get PR: {}", e)),
    };

    let mut result = format!("PR #{} in {}\n\n{}", number, repo, output);

    // Try to get the diff
    if let Ok(diff) = run_gh(&["pr", "diff", &number.to_string(), "--repo", &repo]) {
        result.push_str("\n── Diff (truncated) ──\n");
        let lines: Vec<&str> = diff.lines().collect();
        let max_lines = 200;
        for line in lines.iter().take(max_lines) {
            result.push_str(line);
            result.push('\n');
        }
        if lines.len() > max_lines {
            result.push_str(&format!("... ({} more lines)", lines.len() - max_lines));
        }
    }

    ToolResult::ok(result)
}

/// Post a comment on an issue or PR
fn github_comment(repo: &str, number: i64, body: &str) -> ToolResult {
    let repo = match resolve_repo(repo) {
        Ok(r) => r,
        Err(e) => return ToolResult::err(e),
    };

    match run_gh(&[
        "issue",
        "comment",
        &number.to_string(),
        "--repo",
        &repo,
        "--body",
        body,
    ]) {
        Ok(_) => ToolResult::ok(format!("Comment posted on {} #{}", repo, number)),
        Err(e) => ToolResult::err(format!("Failed to post comment: {}", e)),
    }
}

/// Close an issue
fn github_close_issue(repo: &str, number: i64, comment: &str) -> ToolResult {
    let repo = match resolve_repo(repo) {
        Ok(r) => r,
        Err(e) => return ToolResult::err(e),
    };

    // Add closing comment first
    let _ = run_gh(&[
        "issue",
        "comment",
        &number.to_string(),
        "--repo",
        &repo,
        "--body",
        comment,
    ]);

    match run_gh(&["issue", "close", &number.to_string(), "--repo", &repo]) {
        Ok(_) => ToolResult::ok(format!("Issue #{} closed in {}", number, repo)),
        Err(e) => ToolResult::err(format!("Failed to close issue: {}", e)),
    }
}

/// List open issues
fn github_list_issues(repo: &str, state: &str) -> ToolResult {
    let repo = match resolve_repo(repo) {
        Ok(r) => r,
        Err(e) => return ToolResult::err(e),
    };

    let output = match run_gh(&[
        "issue",
        "list",
        "--repo",
        &repo,
        "--state",
        state,
        "--limit",
        "20",
    ]) {
        Ok(o) => o,
        Err(e) => return ToolResult::err(format!("Failed to list issues: {}", e)),
    };

    ToolResult::ok(format!("Issues in {} (state: {}):\n\n{}", repo, state, output))
}

/// List open PRs
fn github_list_prs(repo: &str, state: &str) -> ToolResult {
    let repo = match resolve_repo(repo) {
        Ok(r) => r,
        Err(e) => return ToolResult::err(e),
    };

    let output = match run_gh(&[
        "pr",
        "list",
        "--repo",
        &repo,
        "--state",
        state,
        "--limit",
        "20",
    ]) {
        Ok(o) => o,
        Err(e) => return ToolResult::err(format!("Failed to list PRs: {}", e)),
    };

    ToolResult::ok(format!("PRs in {} (state: {}):\n\n{}", repo, state, output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_tool_name() {
        let tool = GitHubTool;
        assert_eq!(tool.name(), "github");
    }

    #[test]
    fn test_github_tool_schema() {
        let tool = GitHubTool;
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }

    #[tokio::test]
    async fn test_github_empty_operation() {
        let tool = GitHubTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_github_invalid_operation() {
        let tool = GitHubTool;
        let result = tool
            .execute(serde_json::json!({"operation": "bogus"}))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_github_issue_no_number() {
        let tool = GitHubTool;
        let result = tool
            .execute(serde_json::json!({"operation": "issue_context"}))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_github_pr_no_number() {
        let tool = GitHubTool;
        let result = tool
            .execute(serde_json::json!({"operation": "pr_context"}))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_github_comment_no_body() {
        let tool = GitHubTool;
        let result = tool
            .execute(serde_json::json!({"operation": "comment", "number": 1}))
            .await;
        assert!(!result.success);
    }

    #[test]
    fn test_resolve_repo_provided() {
        let result = resolve_repo("owner/test-repo");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "owner/test-repo");
    }

    #[test]
    fn test_resolve_repo_empty() {
        // Without git remote, this should fail gracefully
        let result = resolve_repo("");
        assert!(result.is_err());
    }
}
