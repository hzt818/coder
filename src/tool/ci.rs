//! CI/CD tool - continuous integration and deployment integration
//!
//! Supports checking CI status, listing workflows, and triggering workflow runs.

use async_trait::async_trait;
use super::*;

pub struct CiTool;

#[async_trait]
impl Tool for CiTool {
    fn name(&self) -> &str {
        "ci"
    }

    fn description(&self) -> &str {
        "Manage CI/CD workflows. Check CI status, list workflows, and trigger workflow runs for GitHub Actions."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["check", "workflows", "trigger", "status"],
                    "description": "CI operation to perform"
                },
                "repo": {
                    "type": "string",
                    "description": "Repository in owner/name format (e.g., 'user/repo')",
                    "default": ""
                },
                "workflow": {
                    "type": "string",
                    "description": "Workflow name or filename (e.g., 'ci.yml')",
                    "default": ""
                },
                "branch": {
                    "type": "string",
                    "description": "Branch name for triggers",
                    "default": ""
                },
                "run_id": {
                    "type": "string",
                    "description": "Workflow run ID for status checks",
                    "default": ""
                }
            },
            "required": ["operation"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let operation = args.get("operation")
            .and_then(|o| o.as_str())
            .unwrap_or("");

        let repo = args.get("repo")
            .and_then(|r| r.as_str())
            .unwrap_or("")
            .to_string();

        let workflow = args.get("workflow")
            .and_then(|w| w.as_str())
            .unwrap_or("")
            .to_string();

        let branch = args.get("branch")
            .and_then(|b| b.as_str())
            .unwrap_or("")
            .to_string();

        let run_id = args.get("run_id")
            .and_then(|r| r.as_str())
            .unwrap_or("")
            .to_string();

        if operation.is_empty() {
            return ToolResult::err("Operation is required (check, workflows, trigger, status)");
        }

        let result = match operation {
            "check" => ci_check(&repo, &branch).await,
            "workflows" => ci_list_workflows(&repo).await,
            "trigger" => ci_trigger(&repo, &workflow, &branch).await,
            "status" => ci_status(&repo, &run_id).await,
            _ => return ToolResult::err(format!("Unknown CI operation: '{}'. Use: check, workflows, trigger, status", operation)),
        };

        match result {
            Ok(output) => ToolResult::ok(output),
            Err(e) => ToolResult::err(e),
        }
    }

    fn requires_permission(&self) -> bool {
        true
    }
}

/// Get the GitHub token from environment
fn get_github_token() -> Result<String, String> {
    std::env::var("GITHUB_TOKEN")
        .or_else(|_| std::env::var("GH_TOKEN"))
        .map_err(|_| "GitHub token not found. Set GITHUB_TOKEN or GH_TOKEN environment variable.".to_string())
}

/// Create a GitHub API client
fn create_github_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Coder/1.0")
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))
}

/// Send an authenticated GET request to the GitHub API
async fn github_get(url: &str, token: &str) -> Result<serde_json::Value, String> {
    let client = create_github_client()?;

    let response = client.get(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("GitHub API request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned HTTP {}", response.status()));
    }

    response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Check the latest CI run status for a repo/branch
async fn ci_check(repo: &str, branch: &str) -> Result<String, String> {
    let repo = if repo.is_empty() {
        detect_local_repo().await?
    } else {
        repo.to_string()
    };

    let token = get_github_token()?;

    let mut url = format!("https://api.github.com/repos/{}/actions/runs", repo);
    if !branch.is_empty() {
        url.push_str(&format!("?branch={}", branch));
    }
    url.push_str("&per_page=5");

    let body = github_get(&url, &token).await?;

    let mut result = format!("CI status for {}:\n\n", repo);

    if let Some(runs) = body.get("workflow_runs").and_then(|r| r.as_array()) {
        if runs.is_empty() {
            result.push_str("No workflow runs found.");
        } else {
            for run in runs {
                let name = run.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                let status = run.get("status").and_then(|s| s.as_str()).unwrap_or("?");
                let conclusion = run.get("conclusion").and_then(|c| c.as_str()).unwrap_or("?");
                let branch_name = run.get("head_branch").and_then(|b| b.as_str()).unwrap_or("?");
                let html_url = run.get("html_url").and_then(|u| u.as_str()).unwrap_or("");

                let status_icon = match conclusion {
                    "success" => "PASS",
                    "failure" | "cancelled" => "FAIL",
                    _ => "PEND",
                };

                result.push_str(&format!("  [{}] {} (branch: {})\n", status_icon, name, branch_name));
                result.push_str(&format!("       Status: {} - {}\n", status, conclusion));
                if !html_url.is_empty() {
                    result.push_str(&format!("       URL: {}\n", html_url));
                }
                result.push('\n');
            }
        }
    } else {
        result.push_str("No workflow runs data available.");
    }

    Ok(result)
}

/// List available workflows in a repo
async fn ci_list_workflows(repo: &str) -> Result<String, String> {
    let repo = if repo.is_empty() {
        detect_local_repo().await?
    } else {
        repo.to_string()
    };

    let token = get_github_token()?;
    let url = format!("https://api.github.com/repos/{}/actions/workflows", repo);
    let body = github_get(&url, &token).await?;

    let mut result = format!("Workflows for {}:\n\n", repo);

    if let Some(workflows) = body.get("workflows").and_then(|w| w.as_array()) {
        if workflows.is_empty() {
            result.push_str("No workflows found.");
        } else {
            for wf in workflows {
                let name = wf.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                let state = wf.get("state").and_then(|s| s.as_str()).unwrap_or("?");
                let path = wf.get("path").and_then(|p| p.as_str()).unwrap_or("");

                result.push_str(&format!("  {} ({})\n", name, path));
                result.push_str(&format!("       State: {}\n", state));
            }
        }
    }

    Ok(result)
}

/// Trigger a workflow run
async fn ci_trigger(repo: &str, workflow: &str, branch: &str) -> Result<String, String> {
    let repo = if repo.is_empty() {
        detect_local_repo().await?
    } else {
        repo.to_string()
    };

    if workflow.is_empty() {
        return Err("Workflow name is required".to_string());
    }

    let branch = if branch.is_empty() { "main" } else { branch };
    let token = get_github_token()?;
    let client = create_github_client()?;

    let url = format!("https://api.github.com/repos/{}/actions/workflows/{}/dispatches", repo, workflow);

    let payload = serde_json::json!({
        "ref": branch
    });

    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to trigger workflow: {}", e))?;

    if response.status().is_success() || response.status().as_u16() == 204 {
        Ok(format!("Successfully triggered workflow '{}' on branch '{}'", workflow, branch))
    } else {
        Err(format!("Failed to trigger workflow: HTTP {}", response.status()))
    }
}

/// Check status of a specific workflow run
async fn ci_status(repo: &str, run_id: &str) -> Result<String, String> {
    if repo.is_empty() || run_id.is_empty() {
        return Err("Both 'repo' and 'run_id' are required".to_string());
    }

    let token = get_github_token()?;
    let url = format!("https://api.github.com/repos/{}/actions/runs/{}", repo, run_id);
    let body = github_get(&url, &token).await?;

    let name = body.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
    let status = body.get("status").and_then(|s| s.as_str()).unwrap_or("?");
    let conclusion = body.get("conclusion").and_then(|c| c.as_str()).unwrap_or("?");
    let branch = body.get("head_branch").and_then(|b| b.as_str()).unwrap_or("?");
    let html_url = body.get("html_url").and_then(|u| u.as_str()).unwrap_or("");

    let mut result = format!("Run #{}: {}\n", run_id, name);
    result.push_str(&format!("  Branch: {}\n", branch));
    result.push_str(&format!("  Status: {}\n", status));
    result.push_str(&format!("  Conclusion: {}\n", conclusion));
    if !html_url.is_empty() {
        result.push_str(&format!("  URL: {}\n", html_url));
    }

    Ok(result)
}

/// Attempt to detect the current GitHub repo from local git
async fn detect_local_repo() -> Result<String, String> {
    use tokio::process::Command;

    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .await
        .map_err(|e| format!("Failed to detect git remote: {}", e))?;

    if !output.status.success() {
        return Err("Not a git repository or no remote 'origin'".to_string());
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Parse GitHub URL formats: git@github.com:user/repo.git or https://github.com/user/repo.git
    let repo = if url.contains("github.com") {
        let parts: Vec<&str> = url.split("github.com").collect();
        if parts.len() > 1 {
            parts[1]
                .trim_start_matches(':')
                .trim_start_matches('/')
                .trim_end_matches(".git")
                .to_string()
        } else {
            return Err(format!("Could not parse remote URL: {}", url));
        }
    } else {
        return Err(format!("Not a GitHub remote: {}", url));
    };

    if repo.is_empty() {
        Err("Could not detect repository from remote".to_string())
    } else {
        Ok(repo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ci_tool_name() {
        let tool = CiTool;
        assert_eq!(tool.name(), "ci");
    }

    #[test]
    fn test_ci_schema() {
        let tool = CiTool;
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }

    #[tokio::test]
    async fn test_ci_empty_operation() {
        let tool = CiTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_ci_invalid_operation() {
        let tool = CiTool;
        let result = tool.execute(serde_json::json!({"operation": "bogus"})).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_ci_check_no_token() {
        let tool = CiTool;
        let result = tool.execute(serde_json::json!({"operation": "check", "repo": "user/repo"})).await;
        // Without token, should fail gracefully
        assert!(!result.success);
    }
}
