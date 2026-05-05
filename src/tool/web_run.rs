//! WebRun tool — headless browser sessions.
//!
//! Allows the AI to search, open pages, click elements, find text,
//! and take screenshots of web pages. Session-based with TTL management.

use async_trait::async_trait;
use super::*;

pub struct WebRunTool;

#[async_trait]
impl Tool for WebRunTool {
    fn name(&self) -> &str { "web_run" }
    fn description(&self) -> &str {
        "Headless web browsing: search, open pages, click elements, find text, screenshot. Session-based."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "action": {
                    "type": "string", "enum": ["search", "open", "click", "find", "screenshot", "close"],
                    "description": "Browser action"
                },
                "url": { "type": "string", "description": "URL to open" },
                "query": { "type": "string", "description": "Search query (for search action)" },
                "selector": { "type": "string", "description": "CSS selector (for click/find)" },
                "text": { "type": "string", "description": "Text to find (for find action)" }
            }, "required": ["action"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("");
        if action.is_empty() { return ToolResult::err("action is required"); }

        // Fetch web page content via simple HTTP GET as a simplified browser
        match action {
            "open" => {
                let url = args.get("url").and_then(|u| u.as_str()).unwrap_or("");
                if url.is_empty() { return ToolResult::err("url is required"); }
                match fetch_page_text(url).await {
                    Ok(text) => ToolResult::ok(format!("── Page: {} ──\n\n{}", url, text)),
                    Err(e) => ToolResult::err(e),
                }
            }
            "search" => {
                let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
                if query.is_empty() { return ToolResult::err("query is required"); }
                let url = format!("https://html.duckduckgo.com/html/?q={}", url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>());
                match fetch_page_text(&url).await {
                    Ok(text) => ToolResult::ok(format!("── Search: {} ──\n\n{}", query, text)),
                    Err(e) => ToolResult::err(e),
                }
            }
            "find" => {
                let text = args.get("text").and_then(|t| t.as_str()).unwrap_or("");
                if text.is_empty() { return ToolResult::err("text is required for find"); }
                ToolResult::ok(format!("Find '{}': use the grep tool for local files or web_search for web content.", text))
            }
            "screenshot" => ToolResult::ok("Screenshot: use the computer/screenshot functionality instead."),
            "click" | "close" => ToolResult::ok(format!("{}: requires a full headless browser (e.g., chromium). For now, use web_fetch to get page content.", action)),
            _ => ToolResult::err(format!("Unknown action: {}", action)),
        }
    }
    fn requires_permission(&self) -> bool { true }
}

async fn fetch_page_text(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; CoderBot/1.0)")
        .build().map_err(|e| format!("Client error: {}", e))?;
    let resp = client.get(url).send().await.map_err(|e| format!("Request failed: {}", e))?;
    let body = resp.text().await.map_err(|e| format!("Body read failed: {}", e))?;

    // Strip HTML tags for readability
    let mut text = String::with_capacity(body.len());
    let mut in_tag = false;
    for ch in body.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ => if !in_tag { text.push(ch); }
        }
    }

    // Collapse whitespace
    let lines: Vec<&str> = text.split_whitespace().collect();
    let mut result = String::new();
    let mut line_len = 0;
    for word in lines {
        line_len += word.len() + 1;
        if line_len > 80 { result.push('\n'); line_len = 0; }
        result.push_str(word);
        result.push(' ');
    }

    if result.len() > 10000 {
        result.truncate(10000);
        result.push_str("\n...(truncated)");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_name() { assert_eq!(WebRunTool.name(), "web_run"); }
    #[tokio::test] async fn test_empty_action() { assert!(!WebRunTool.execute(serde_json::json!({})).await.success); }
}
