//! Grep tool - content search
//!
//! Tries ripgrep (`rg`) first, then falls back to `grep` (Unix) or
//! `findstr` (Windows) if ripgrep is not installed.

use super::*;
use async_trait::async_trait;

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for a pattern in file contents. Supports regex patterns and file glob filtering."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Pattern to search for (regex supported)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (default: current dir)",
                    "default": "."
                },
                "glob": {
                    "type": "string",
                    "description": "File glob filter (e.g., *.rs, *.py)",
                    "default": ""
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let pattern = args.get("pattern").and_then(|p| p.as_str()).unwrap_or("");

        if pattern.is_empty() {
            return ToolResult::err("Pattern is required");
        }

        let search_path = args.get("path").and_then(|p| p.as_str()).unwrap_or(".");

        let file_glob = args.get("glob").and_then(|g| g.as_str()).unwrap_or("");

        // Try ripgrep first, fall back to grep/findstr
        let result = try_search_rg(pattern, search_path, file_glob)
            .await
            .or_else(|_| try_search_fallback(pattern, search_path, file_glob));

        match result {
            Ok(output) => {
                let line_count = output.lines().count();
                ToolResult::ok(format!(
                    "Found {} matches for '{}':\n{}",
                    line_count, pattern, output
                ))
            }
            Err(e) => ToolResult::err(format!("Search failed: {}", e)),
        }
    }
}

/// Try searching with ripgrep (`rg`)
async fn try_search_rg(pattern: &str, path: &str, glob: &str) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new("rg");
    cmd.arg("--line-number")
        .arg("--color")
        .arg("never")
        .arg("--with-filename")
        .current_dir(path);

    if !glob.is_empty() {
        cmd.arg("--glob").arg(glob);
    }
    cmd.arg(pattern);

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to run ripgrep: {}", e))?;

    if output.status.code() == Some(1) {
        // No matches — not an error, just empty
        return Err("No matches".to_string());
    }
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ripgrep failed: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Fallback: use basic `grep` (Unix) or `findstr` (Windows)
fn try_search_fallback(pattern: &str, path: &str, glob: &str) -> Result<String, String> {
    // Check which command is available
    let has_grep = which("grep");
    let has_findstr = cfg!(target_os = "windows") && which("findstr");

    if has_grep {
        let mut cmd = std::process::Command::new("grep");
        cmd.arg("-rn").arg("--color=never").arg(pattern).arg(path);
        if !glob.is_empty() {
            cmd.arg(format!("--include={}", glob));
        }
        let output = cmd
            .output()
            .map_err(|e| format!("Failed to run grep: {}", e))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
        if output.status.code() == Some(1) {
            return Err("No matches".to_string());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("grep failed: {}", stderr));
    }

    if has_findstr {
        let mut cmd = std::process::Command::new("findstr");
        cmd.arg("/S").arg("/N").arg(pattern);
        if !glob.is_empty() {
            cmd.arg(format!("*.{}", glob.trim_start_matches('*')));
        } else {
            cmd.arg("*.*");
        }
        cmd.current_dir(path);
        let output = cmd
            .output()
            .map_err(|e| format!("Failed to run findstr: {}", e))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Err("No search tool available. Install ripgrep (rg), grep (Unix), or use a different search method.".to_string())
}

/// Quick check if a binary is available on PATH
fn which(name: &str) -> bool {
    std::process::Command::new(if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    })
    .arg(name)
    .output()
    .map(|o| o.status.success())
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grep_tool_name() {
        let tool = GrepTool;
        assert_eq!(tool.name(), "grep");
    }

    #[tokio::test]
    async fn test_grep_no_pattern() {
        let tool = GrepTool;
        let result = tool.execute(serde_json::json!({"pattern": ""})).await;
        assert!(!result.success);
    }
}
