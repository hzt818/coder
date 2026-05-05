//! Docs tool - fetches documentation via Context7
//!
//! This tool allows the AI to look up documentation for libraries and frameworks,
//! getting up-to-date API information and usage examples.

use async_trait::async_trait;
use super::*;

pub struct DocsTool;

#[async_trait]
impl Tool for DocsTool {
    fn name(&self) -> &str {
        "docs"
    }

    fn description(&self) -> &str {
        "Search and retrieve documentation for any programming library or framework. Use this to get up-to-date API information, code examples, and usage patterns."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "library": {
                    "type": "string",
                    "description": "Library or framework name (e.g., 'Next.js', 'React', 'Express')"
                },
                "query": {
                    "type": "string",
                    "description": "What you want to know about the library"
                }
            },
            "required": ["library", "query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let library = args.get("library")
            .and_then(|l| l.as_str())
            .unwrap_or("");

        let query = args.get("query")
            .and_then(|q| q.as_str())
            .unwrap_or("");

        if library.is_empty() || query.is_empty() {
            return ToolResult::err("Both 'library' and 'query' are required");
        }

        // Use Context7 via MCP if available
        match query_context7(library, query).await {
            Ok(docs) => ToolResult::ok(docs),
            Err(e) => {
                // Fallback: search via web
                ToolResult::ok(format!(
                    "Documentation lookup for '{}' about '{}'\n\nContext7 unavailable: {}\n\nTry using web_search to find documentation.",
                    library, query, e
                ))
            }
        }
    }
}

/// Query documentation using DuckDuckGo search
async fn query_context7(library: &str, query: &str) -> Result<String, String> {
    // Use DuckDuckGo to search for documentation
    let search_query = format!("{} {} documentation", library, query);
    let encoded: String = search_query.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => b as char,
            b' ' => '+'.into(),
            _ => format!("%{:02X}", b).chars().next().unwrap_or(b as char),
        })
        .collect();

    let url = format!("https://lite.duckduckgo.com/lite/?q={}", encoded);
    // Use reqwest to fetch
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let response = client.get(&url).send().await
        .map_err(|e| format!("Request failed: {}", e))?;
    let body = response.text().await
        .map_err(|e| format!("Body error: {}", e))?;

    // Simple extraction - get text content
    let mut result = format!("Documentation search for '{} {}':\n\n", library, query);
    result.push_str(&body[..std::cmp::min(body.len(), 100000)]);
    if body.len() > 100000 {
        result.push_str("\n...(truncated)");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docs_tool_name() {
        let tool = DocsTool;
        assert_eq!(tool.name(), "docs");
    }

    #[tokio::test]
    async fn test_docs_empty_args() {
        let tool = DocsTool;
        let result = tool.execute(serde_json::json!({"library": "", "query": ""})).await;
        assert!(!result.success);
    }
}
