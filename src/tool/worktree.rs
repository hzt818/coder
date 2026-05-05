//! Git worktree tool - manage git worktrees
//!
//! Supports creating, removing, and listing git worktrees.
//! Uses gix crate with fallback to bash git commands.

use async_trait::async_trait;
use super::*;

pub struct WorktreeTool;

#[async_trait]
impl Tool for WorktreeTool {
    fn name(&self) -> &str {
        "worktree"
    }

    fn description(&self) -> &str {
        "Manage Git worktrees. Create, remove, or list worktrees for working on multiple branches simultaneously."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "remove", "list", "prune"],
                    "description": "Worktree action to perform"
                },
                "path": {
                    "type": "string",
                    "description": "Path where the worktree should be created (for create) or path of the worktree to remove (for remove)",
                    "default": ""
                },
                "branch": {
                    "type": "string",
                    "description": "Branch name for the new worktree (default: derived from path)",
                    "default": ""
                },
                "commit": {
                    "type": "string",
                    "description": "Commit-ish to check out in the new worktree (default: HEAD)",
                    "default": ""
                },
                "repo_path": {
                    "type": "string",
                    "description": "Path to the main repository (default: current directory)",
                    "default": "."
                },
                "force": {
                    "type": "boolean",
                    "description": "Force the operation (for remove)",
                    "default": false
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let action = args.get("action")
            .and_then(|a| a.as_str())
            .unwrap_or("");

        if action.is_empty() {
            return ToolResult::err("Action is required (create, remove, list, prune)");
        }

        let repo_path = args.get("repo_path")
            .and_then(|p| p.as_str())
            .unwrap_or(".")
            .to_string();

        match action {
            "list" => worktree_list(&repo_path).await,
            "create" => {
                let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string();
                let branch = args.get("branch").and_then(|b| b.as_str()).unwrap_or("").to_string();
                let commit = args.get("commit").and_then(|c| c.as_str()).unwrap_or("").to_string();
                worktree_create(&repo_path, &path, &branch, &commit).await
            }
            "remove" => {
                let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string();
                let force = args.get("force").and_then(|f| f.as_bool()).unwrap_or(false);
                worktree_remove(&repo_path, &path, force).await
            }
            "prune" => worktree_prune(&repo_path).await,
            _ => ToolResult::err(format!("Unknown worktree action: '{}'. Use: create, remove, list, prune", action)),
        }
    }

    fn requires_permission(&self) -> bool {
        true
    }
}

/// Run a git command in the specified repo
async fn run_git(repo_path: &str, args: &[&str]) -> Result<String, String> {
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
        Ok(stdout.trim().to_string())
    } else {
        Err(stderr.trim().to_string())
    }
}

/// List all worktrees in a repository
async fn worktree_list(repo_path: &str) -> ToolResult {
    match run_git(repo_path, &["worktree", "list"]).await {
        Ok(output) => {
            if output.is_empty() {
                ToolResult::ok("No worktrees found.")
            } else {
                let mut result = format!("Worktrees in '{}':\n\n", repo_path);
                result.push_str(&format!("{:<50} {:<30} {}\n", "PATH", "BRANCH", "COMMIT"));
                result.push_str(&"-".repeat(100));
                result.push('\n');

                for line in output.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let wt_path = parts[0];
                        let branch = parts[1].trim_start_matches('(').trim_end_matches(')');
                        let commit = parts.get(2).unwrap_or(&"");
                        result.push_str(&format!("{:<50} {:<30} {}\n", wt_path, branch, commit));
                    } else {
                        result.push_str(line);
                        result.push('\n');
                    }
                }

                ToolResult::ok(result)
            }
        }
        Err(e) => ToolResult::err(format!("Failed to list worktrees: {}", e)),
    }
}

/// Create a new worktree
async fn worktree_create(repo_path: &str, path: &str, branch: &str, commit: &str) -> ToolResult {
    if path.is_empty() {
        return ToolResult::err("Path is required for creating a worktree");
    }

    let mut args = vec!["worktree", "add"];

    // Determine branch name from path if not specified
    if !branch.is_empty() {
        args.push("-b");
        args.push(branch);
    }

    args.push(path);

    if !commit.is_empty() {
        args.push(commit);
    }

    match run_git(repo_path, &args).await {
        Ok(output) => {
            let branch_info = if !branch.is_empty() {
                format!(" with branch '{}'", branch)
            } else if !commit.is_empty() {
                format!(" at commit '{}'", &commit[..commit.len().min(12)])
            } else {
                String::new()
            };
            ToolResult::ok(format!("Worktree created at '{}'{}\n\n{}", path, branch_info, output))
        }
        Err(e) => ToolResult::err(format!("Failed to create worktree: {}", e)),
    }
}

/// Remove a worktree
async fn worktree_remove(repo_path: &str, path: &str, force: bool) -> ToolResult {
    if path.is_empty() {
        return ToolResult::err("Path is required for removing a worktree");
    }

    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(path);

    match run_git(repo_path, &args).await {
        Ok(output) => ToolResult::ok(format!("Worktree removed from '{}'\n{}", path, output)),
        Err(e) => ToolResult::err(format!("Failed to remove worktree: {}", e)),
    }
}

/// Prune stale worktree references
async fn worktree_prune(repo_path: &str) -> ToolResult {
    match run_git(repo_path, &["worktree", "prune"]).await {
        Ok(output) => {
            if output.is_empty() {
                ToolResult::ok("Worktree references pruned.")
            } else {
                ToolResult::ok(format!("Worktree prune result:\n{}", output))
            }
        }
        Err(e) => ToolResult::err(format!("Failed to prune worktrees: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worktree_tool_name() {
        let tool = WorktreeTool;
        assert_eq!(tool.name(), "worktree");
    }

    #[test]
    fn test_worktree_schema() {
        let tool = WorktreeTool;
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }

    #[tokio::test]
    async fn test_worktree_empty_action() {
        let tool = WorktreeTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_worktree_invalid_action() {
        let tool = WorktreeTool;
        let result = tool.execute(serde_json::json!({"action": "bogus"})).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_worktree_list_not_repo() {
        let tool = WorktreeTool;
        // Run in a temp dir that's not a git repo
        let tmp = tempfile::tempdir().unwrap();
        let path_str = tmp.path().to_str().unwrap();

        let result = tool.execute(serde_json::json!({
            "action": "list",
            "repo_path": path_str
        })).await;
        assert!(!result.success);
    }
}
