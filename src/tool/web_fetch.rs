//! Web fetch tool - fetches web page content

use async_trait::async_trait;
use super::*;

pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch and read the content of a web page."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to fetch"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let url = args.get("url")
            .and_then(|u| u.as_str())
            .unwrap_or("");

        if url.is_empty() {
            return ToolResult::err("URL is required");
        }

        match fetch_url(url).await {
            Ok(content) => ToolResult::ok(content),
            Err(e) => ToolResult::err(format!("Failed to fetch URL: {}", e)),
        }
    }
}

async fn fetch_url(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Coder/1.0")
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client.get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    // Truncate if too large
    const MAX_SIZE: usize = 500_000;
    let mut result = String::new();
    result.push_str(&format!("URL: {}\n", url));
    result.push_str(&format!("Content-Type: {}\n", content_type));
    result.push_str(&format!("Size: {} bytes\n", body.len()));
    result.push('\n');

    if body.len() > MAX_SIZE {
        result.push_str(&body[..MAX_SIZE]);
        result.push_str(&format!("\n... (truncated, {} bytes total)", body.len()));
    } else {
        result.push_str(&body);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_fetch_tool_name() {
        let tool = WebFetchTool;
        assert_eq!(tool.name(), "web_fetch");
    }

    #[tokio::test]
    async fn test_web_fetch_no_url() {
        let tool = WebFetchTool;
        let result = tool.execute(serde_json::json!({"url": ""})).await;
        assert!(!result.success);
    }
}
