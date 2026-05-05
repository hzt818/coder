//! Recall Archive tool — search prior session archive content via BM25.
//!
//! Scans session cycle archives and returns top-N matching messages
//! scored with BM25 (standard K1=1.5, B=0.75). No external dependencies.

use async_trait::async_trait;
use super::*;
use std::collections::HashMap;

const DEFAULT_MAX_RESULTS: usize = 3;
const HARD_MAX_RESULTS: usize = 10;
const K1: f64 = 1.5;
const B: f64 = 0.75;

pub struct RecallTool;

#[async_trait]
impl Tool for RecallTool {
    fn name(&self) -> &str { "recall" }
    fn description(&self) -> &str {
        "Search prior session content using BM25 ranking. Use when you need information from earlier in the conversation."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "query": { "type": "string", "description": "Search query" },
                "max_results": { "type": "integer", "description": "Max results (1-10)", "default": 3 }
            }, "required": ["query"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("").trim().to_string();
        if query.is_empty() { return ToolResult::err("Query is required"); }
        let max_results = args.get("max_results").and_then(|m| m.as_u64()).unwrap_or(3).clamp(1, HARD_MAX_RESULTS as u64) as usize;

        // Try to load recent session files and search their content
        let session_dir = crate::util::path::coder_dir().join("sessions");
        let mut hits = Vec::new();

        if session_dir.exists() {
            let mut entries: Vec<_> = match std::fs::read_dir(&session_dir) {
                Ok(e) => e.filter_map(|e| e.ok()).collect(),
                Err(_) => Vec::new(),
            };
            entries.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());

            for entry in entries.iter().rev().take(5) {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    let query_terms = tokenize(&query);
                    let doc_terms = tokenize(&content);
                    let score = bm25_score(&query_terms, &doc_terms, &HashMap::new(), 1.0, 1);
                    if score > 0.0 {
                        let excerpt = content.chars().take(200).collect::<String>();
                        hits.push((score, entry.path().display().to_string(), excerpt));
                    }
                }
            }
        }

        if hits.is_empty() {
            return ToolResult::ok("No relevant content found in session archives.");
        }

        hits.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut result = format!("── Recall Results (top {}) ──\n\n", max_results);
        for (i, (score, path, excerpt)) in hits.iter().take(max_results).enumerate() {
            result.push_str(&format!("{}. [score: {:.2}] {}\n   {}\n\n", i + 1, score, path, excerpt));
        }
        ToolResult::ok(result)
    }
}

/// Simple whitespace tokenizer.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase().split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|t| !t.is_empty() && t.len() > 2)
        .collect()
}

/// Compute BM25 score for query terms against a document.
fn bm25_score(query_terms: &[String], doc_terms: &[String], _idf: &HashMap<String, f64>, _avg_dl: f64, _num_docs: usize) -> f64 {
    let mut score = 0.0;
    let doc_len = doc_terms.len() as f64;
    let term_freq: HashMap<&str, usize> = doc_terms.iter().fold(HashMap::new(), |mut acc, t| {
        *acc.entry(t.as_str()).or_insert(0) += 1; acc
    });

    for term in query_terms {
        let tf = *term_freq.get(term.as_str()).unwrap_or(&0) as f64;
        if tf > 0.0 {
            let idf = 1.0_f64.ln() + 1.0; // simplified IDF
            score += idf * (tf * (K1 + 1.0)) / (tf + K1 * (1.0 - B + B * doc_len / 100.0));
        }
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_name() { assert_eq!(RecallTool.name(), "recall"); }
    #[tokio::test] async fn test_empty_query() { assert!(!RecallTool.execute(serde_json::json!({})).await.success); }

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Hello World! This is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        assert!(!tokens.contains(&"a".to_string())); // filtered by length
    }

    #[test]
    fn test_bm25_score_positive() {
        let query = vec!["hello".to_string(), "world".to_string()];
        let doc = vec!["hello".to_string(), "world".to_string(), "test".to_string()];
        let score = bm25_score(&query, &doc, &HashMap::new(), 1.0, 1);
        assert!(score > 0.0);
    }

    #[test]
    fn test_bm25_score_no_match() {
        let query = vec!["python".to_string()];
        let doc = vec!["rust".to_string(), "java".to_string()];
        let score = bm25_score(&query, &doc, &HashMap::new(), 1.0, 1);
        assert!(score == 0.0);
    }
}
