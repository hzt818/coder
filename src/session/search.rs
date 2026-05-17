//! Session search — fuzzy search across conversation history.
//!
//! Provides `thinkback` functionality to search past sessions
//! by content keywords using basic fuzzy matching.

/// Search session content for a query string.
pub fn search_sessions(query: &str, max_results: usize) -> Vec<SessionSearchResult> {
    let session_dir = crate::util::path::coder_dir().join("sessions");
    if !session_dir.exists() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();
    let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
    let mut results = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&session_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let content_lower = content.to_lowercase();
                    let match_count = query_terms
                        .iter()
                        .filter(|t| content_lower.contains(*t))
                        .count();

                    if match_count > 0 {
                        // Extract session title and message count from JSON
                        let title = extract_title(&content).unwrap_or_else(|| {
                            path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string()
                        });
                        let msg_count = content.matches("\"role\"").count();
                        let preview = extract_preview(&content, query);

                        results.push(SessionSearchResult {
                            path: path.display().to_string(),
                            title,
                            message_count: msg_count,
                            match_count,
                            preview,
                        });
                    }
                }
            }
        }
    }

    // Sort by match relevance
    results.sort_by_key(|r| std::cmp::Reverse(r.match_count));
    results.truncate(max_results);
    results
}

/// A session search result.
#[derive(Debug, Clone)]
pub struct SessionSearchResult {
    pub path: String,
    pub title: String,
    pub message_count: usize,
    pub match_count: usize,
    pub preview: String,
}

/// Format search results for display.
pub fn format_search_results(results: &[SessionSearchResult]) -> String {
    if results.is_empty() {
        return "── Thinkback ──\n\nNo matching sessions found.".to_string();
    }
    let mut output = format!("── Thinkback ({}) ──\n\n", results.len());
    for (i, r) in results.iter().enumerate() {
        output.push_str(&format!(
            "{}. {} ({} msgs, {} matches)\n   Preview: {}\n\n",
            i + 1,
            r.title,
            r.message_count,
            r.match_count,
            r.preview
        ));
    }
    output
}

fn extract_title(json: &str) -> Option<String> {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json) {
        val.get("title")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                val.get("id")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
            })
    } else {
        None
    }
}

fn extract_preview(json: &str, query: &str) -> String {
    let query_lower = query.to_lowercase();
    // Find the first message containing the query terms
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json) {
        if let Some(messages) = val.get("messages").and_then(|m| m.as_array()) {
            for msg in messages {
                if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                    if content.to_lowercase().contains(&query_lower) {
                        return content.chars().take(300).collect();
                    }
                }
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title_from_json() {
        let json = r#"{"title": "Test Session", "messages": []}"#;
        assert_eq!(extract_title(json), Some("Test Session".to_string()));
    }

    #[test]
    fn test_extract_title_missing() {
        let json = r#"{"id": "s1"}"#;
        assert_eq!(extract_title(json), Some("s1".to_string()));
    }

    #[test]
    fn test_extract_preview() {
        let json = r#"{"messages": [{"role": "user", "content": "Hello world test"}]}"#;
        let preview = extract_preview(json, "world");
        assert!(preview.contains("world"));
    }

    #[test]
    fn test_format_empty() {
        let result = format_search_results(&[]);
        assert!(result.contains("No matching sessions"));
    }

    #[test]
    fn test_format_results() {
        let results = vec![SessionSearchResult {
            path: "/tmp/s1.json".into(),
            title: "Test".into(),
            message_count: 5,
            match_count: 3,
            preview: "Hello world".into(),
        }];
        let formatted = format_search_results(&results);
        assert!(formatted.contains("Test"));
        assert!(formatted.contains("5 msgs"));
    }
}
