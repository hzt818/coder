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

        match search_web(query, max_results).await {
            Ok(results) => ToolResult::ok(results),
            Err(e) => ToolResult::err(format!("Search failed: {}", e)),
        }
    }
}

/// Search the web using a public search engine API
async fn search_web(query: &str, max_results: usize) -> Result<String, String> {
    // Use DuckDuckGo's instant answer API (no API key required)
    let url = format!("https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        urlencoding(query));

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

/// URL-encode a string for use in search queries
fn urlencoding(input: &str) -> String {
    let mut result = String::with_capacity(input.len() * 3);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => {
                result.push('+');
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
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

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding("hello world"), "hello+world");
        assert_eq!(urlencoding("rust/lang"), "rust%2Flang");
        assert_eq!(urlencoding("a+b"), "a%2Bb");
    }
}
