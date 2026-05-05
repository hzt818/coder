//! Docs tool - fetches documentation via web search
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

        match fetch_docs_web(library, query).await {
            Ok(docs) => ToolResult::ok(docs),
            Err(e) => {
                ToolResult::ok(format!(
                    "Documentation lookup for '{}' about '{}'\n\nSearch failed: {}\n\nTry using web_search to find documentation.",
                    library, query, e
                ))
            }
        }
    }
}

/// Fetch documentation by searching DuckDuckGo and extracting text content.
async fn fetch_docs_web(library: &str, query: &str) -> Result<String, String> {
    let search_query = format!("{} {} documentation", library, query);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;

    // Try DuckDuckGo HTML first
    let url = format!("https://html.duckduckgo.com/html/?q={}", urlencode(&search_query));
    let response = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (compatible; CoderDocs/1.0)")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let body = response.text().await
        .map_err(|e| format!("Body error: {}", e))?;

    // Strip HTML tags and extract meaningful text content
    let text = strip_html_tags(&body);
    let lines: Vec<&str> = text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    // Find relevant content (skip navigation/boilerplate)
    let start_idx = lines.iter()
        .position(|l| l.to_lowercase().contains(&library.to_lowercase()))
        .unwrap_or(0);

    let relevant: Vec<&str> = lines.iter()
        .skip(start_idx)
        .take(200) // limit output
        .map(|l| *l)
        .collect();

    let mut result = format!("Documentation search for '{} {}':\n\n", library, query);
    result.push_str(&relevant.join("\n"));

    let max_len = 50_000;
    if result.len() > max_len {
        let safe_end = result.floor_char_boundary(max_len);
        result.truncate(safe_end);
        result.push_str("\n...(truncated)");
    }

    Ok(result)
}

/// Simple HTML tag stripper that preserves text content.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_entity = false;
    let mut entity_buf = String::new();

    for ch in html.chars() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
            }
            continue;
        }
        if ch == '<' {
            in_tag = true;
            continue;
        }
        if ch == '&' {
            in_entity = true;
            entity_buf.clear();
            continue;
        }
        if in_entity {
            if ch == ';' {
                let decoded = match entity_buf.as_str() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "nbsp" => " ",
                    "apos" => "'",
                    _ => "",
                };
                result.push_str(decoded);
                in_entity = false;
            } else if entity_buf.len() < 16 {
                entity_buf.push(ch);
            } else {
                result.push('&');
                result.push_str(&entity_buf);
                result.push(ch);
                in_entity = false;
            }
            continue;
        }
        result.push(ch);
    }

    // Collapse multiple whitespace into single space
    let mut cleaned = String::with_capacity(result.len());
    let mut prev_space = false;
    for ch in result.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                cleaned.push(' ');
                prev_space = true;
            }
        } else {
            cleaned.push(ch);
            prev_space = false;
        }
    }

    cleaned
}

fn urlencode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
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

    #[test]
    fn test_strip_html_simple() {
        let html = "<p>Hello <b>World</b></p>";
        assert_eq!(strip_html_tags(html).trim(), "Hello World");
    }

    #[test]
    fn test_strip_html_entities() {
        let html = "<p>Rust &amp; C++ are &lt;fast&gt;</p>";
        assert_eq!(strip_html_tags(html).trim(), "Rust & C++ are <fast>");
    }

    #[test]
    fn test_strip_html_no_tags() {
        let text = "Plain text content";
        assert_eq!(strip_html_tags(text).trim(), "Plain text content");
    }

    #[test]
    fn test_strip_html_empty() {
        assert_eq!(strip_html_tags("").trim(), "");
    }

    #[test]
    fn test_strip_html_nested() {
        let html = "<div><ul><li>Item 1</li><li>Item 2</li></ul></div>";
        // Without whitespace between tags, items get concatenated
        let stripped = strip_html_tags(html);
        let result = stripped.trim();
        assert!(result.contains("Item 1"), "Expected 'Item 1' in result, got: '{}'", result);
        assert!(result.contains("Item 2"), "Expected 'Item 2' in result, got: '{}'", result);
    }
}
