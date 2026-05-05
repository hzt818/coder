//! Web search tool - search the web for information
//!
//! Uses reqwest to query web search engines and return results.

use async_trait::async_trait;
use super::*;

pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for up-to-date information, documentation, news, and more. Returns search result snippets."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 5)",
                    "default": 5
                },
                "engine": {
                    "type": "string",
                    "enum": ["auto", "duckduckgo", "brave"],
                    "description": "Search engine to use (auto = try multiple engines)",
                    "default": "auto"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let query = args.get("query")
            .and_then(|q| q.as_str())
            .unwrap_or("");

        if query.is_empty() {
            return ToolResult::err("Query is required");
        }

        let max_results = args.get("max_results")
            .and_then(|m| m.as_u64())
            .unwrap_or(5) as usize;

        let engine = args.get("engine")
            .and_then(|e| e.as_str())
            .unwrap_or("auto");

        match search_web_multi(query, max_results, engine).await {
            Ok(results) => ToolResult::ok(results),
            Err(e) => ToolResult::err(format!("Search failed: {}", e)),
        }
    }
}

/// Search the web using the best available engine
async fn search_web_multi(query: &str, max_results: usize, engine: &str) -> Result<String, String> {
    match engine {
        "brave" => search_brave(query, max_results).await,
        _ => {
            // Try DuckDuckGo first, fall back to HTML scrape
            let ddg = search_duckduckgo(query, max_results).await;
            if ddg.is_ok() { ddg }
            else { search_duckduckgo_html(query, max_results).await }
        }
    }
}

/// Search using DuckDuckGo instant answer API (no API key needed)
async fn search_duckduckgo(query: &str, max_results: usize) -> Result<String, String> {
    let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    let url = format!("https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        encoded);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Coder/1.0 (Rust AI Development Tool)")
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("Search request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Search API returned HTTP {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse search results: {}", e))?;

    let mut result = String::new();
    result.push_str(&format!("Search results for: {}\n\n", query));

    // Extract abstract text
    if let Some(abstract_text) = body.get("AbstractText").and_then(|a| a.as_str()) {
        if !abstract_text.is_empty() {
            result.push_str(&format!("Summary: {}\n\n", abstract_text));
        }
    }

    // Extract results from RelatedTopics
    let mut count = 0;
    if let Some(topics) = body.get("RelatedTopics").and_then(|t| t.as_array()) {
        for topic in topics {
            if count >= max_results {
                break;
            }

            let text = topic.get("Text").and_then(|t| t.as_str()).unwrap_or("");
            let first_url = topic.get("FirstURL").and_then(|u| u.as_str()).unwrap_or("");

            if !text.is_empty() {
                count += 1;
                if !first_url.is_empty() {
                    result.push_str(&format!("{}. {} - {}\n", count, text, first_url));
                } else {
                    result.push_str(&format!("{}. {}\n", count, text));
                }
            }
        }
    }

    if count == 0 {
        result.push_str("(No detailed results found. Try a more specific query.)");
    }

    Ok(result)
}

/// Fallback: scrape DuckDuckGo HTML search results
async fn search_duckduckgo_html(query: &str, max_results: usize) -> Result<String, String> {
    let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    let url = format!("https://html.duckduckgo.com/html/?q={}", encoded);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; CoderBot/1.0)")
        .build()
        .map_err(|e| format!("Client error: {}", e))?;

    let response = client.get(&url).send().await
        .map_err(|e| format!("Request failed: {}", e))?;
    let body = response.text().await
        .map_err(|e| format!("Body read failed: {}", e))?;

    // Extract result links using simple HTML parsing
    let mut result = format!("Search results for: {}\n\n", query);
    let mut count = 0;

    for line in body.lines() {
        if count >= max_results { break; }
        let trimmed = line.trim();
        if let Some(url_start) = trimmed.find("uddg=") {
            let url_end = trimmed[url_start + 5..].find('"').unwrap_or(0);
            let found_url = &trimmed[url_start + 5..url_start + 5 + url_end];
            // Find the result title
            let title = trimmed.split('>').nth(1).unwrap_or("").split('<').next().unwrap_or("");
            if !title.is_empty() {
                count += 1;
                result.push_str(&format!("{}. {} - {}\n", count, html_unescape(title), found_url));
            }
        }
    }

    if count == 0 { result.push_str("(No results found.)"); }
    Ok(result)
}

/// Search using Brave Search API (requires BRAVE_API_KEY env var)
async fn search_brave(query: &str, max_results: usize) -> Result<String, String> {
    let api_key = std::env::var("BRAVE_API_KEY")
        .map_err(|_| "BRAVE_API_KEY not set".to_string())?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;

    let url = format!("https://api.search.brave.com/res/v1/web/search?q={}&count={}",
        url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>(), max_results);

    let response = client.get(&url)
        .header("Accept", "application/json")
        .header("Accept-Encoding", "gzip")
        .header("X-Subscription-Token", &api_key)
        .send()
        .await
        .map_err(|e| format!("Brave request failed: {}", e))?;

    let data: serde_json::Value = response.json().await
        .map_err(|e| format!("Parse failed: {}", e))?;

    let mut result = format!("Search results for: {}\n\n", query);
    if let Some(web) = data.get("web").and_then(|w| w.get("results")).and_then(|r| r.as_array()) {
        for (i, item) in web.iter().take(max_results).enumerate() {
            let title = item.get("title").and_then(|t| t.as_str()).unwrap_or("");
            let url = item.get("url").and_then(|u| u.as_str()).unwrap_or("");
            let desc = item.get("description").and_then(|d| d.as_str()).unwrap_or("");
            result.push_str(&format!("{}. {} - {}\n   {}\n\n", i + 1, title, url, desc));
        }
    }
    Ok(result)
}

/// Simple HTML entity unescape
fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
        .replace("&quot;", "\"").replace("&#39;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_search_tool_name() {
        let tool = WebSearchTool;
        assert_eq!(tool.name(), "web_search");
    }

    #[test]
    fn test_web_search_schema() {
        let tool = WebSearchTool;
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }

    #[tokio::test]
    async fn test_web_search_empty_query() {
        let tool = WebSearchTool;
        let result = tool.execute(serde_json::json!({"query": ""})).await;
        assert!(!result.success);
    }
}
