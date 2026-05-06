//! RLM (Recursive Language Model) tool — parallel LLM sub-queries.
//!
//! Accepts a main prompt and optional sub-tasks, fans out to the configured
//! AI provider, executes each sub-task as an independent LLM call, and
//! aggregates the results.

use super::*;
use async_trait::async_trait;
use futures::future::join_all;

pub struct RlmTool;

#[async_trait]
impl Tool for RlmTool {
    fn name(&self) -> &str {
        "rlm"
    }
    fn description(&self) -> &str {
        concat!(
            "Recursive Language Model - fan out parallel LLM sub-queries. ",
            "Accepts a prompt and optional sub-tasks, runs them in parallel (1-16), ",
            "and returns aggregated results. Use for batch analysis, code review."
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "prompt": { "type": "string", "description": "The main prompt describing the analysis task" },
                "sub_tasks": { "type": "array", "items": {"type": "string"}, "description": "Optional list of sub-tasks to parallelize" },
                "max_parallel": { "type": "integer", "description": "Max parallel sub-queries (1-16)", "default": 4 },
                "model": { "type": "string", "description": "Model for sub-queries", "default": "auto" }
            }, "required": ["prompt"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let prompt = args.get("prompt").and_then(|p| p.as_str()).unwrap_or("");
        if prompt.is_empty() {
            return ToolResult::err("Prompt is required");
        }

        let max_parallel = args
            .get("max_parallel")
            .and_then(|m| m.as_i64())
            .unwrap_or(4);
        if max_parallel < 1 || max_parallel > 16 {
            return ToolResult::err("max_parallel must be between 1 and 16");
        }

        let sub_tasks: Vec<String> = args
            .get("sub_tasks")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let tasks = if sub_tasks.is_empty() {
            vec![
                "Analyze".to_string(),
                "Identify issues".to_string(),
                "Suggest improvements".to_string(),
            ]
        } else {
            sub_tasks
        };

        let api_key = std::env::var("OPENAI_API_KEY")
            .or_else(|_| std::env::var("CODER_API_KEY"))
            .unwrap_or_default();
        let base_url = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let model = args
            .get("model")
            .and_then(|m| m.as_str())
            .filter(|m| *m != "auto")
            .map(|m| m.to_string())
            .unwrap_or_else(|| {
                std::env::var("CODER_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string())
            });

        if api_key.is_empty() {
            let mut result = format!("🔁 RLM Analysis\nPrompt: {}\n\n", prompt);
            result.push_str(&format!("Tasks ({}):\n", tasks.len()));
            for (i, task) in tasks.iter().enumerate() {
                result.push_str(&format!("  {}. {}\n", i + 1, task));
            }
            result.push_str("\nRLM requires an AI provider API key.\n");
            result.push_str("Set OPENAI_API_KEY or CODER_API_KEY environment variable, or configure a provider in config.toml.\n");
            return ToolResult::ok(result);
        }

        let actual_parallel = tasks.len().min(max_parallel as usize);
        let mut result = format!("🔁 RLM Analysis\nPrompt: {}\n", prompt);
        result.push_str(&format!("Model: {}\n", model));
        result.push_str(&format!(
            "Tasks: {} (parallel: {})\n\n",
            tasks.len(),
            actual_parallel
        ));

        let mut all_results: Vec<(usize, String, String)> = Vec::new();

        // Clone strings before the loop so they can be moved into async closures
        let api_key_shared = api_key;
        let base_url_shared = base_url;

        for chunk in tasks.chunks(actual_parallel) {
            let mut handles = Vec::new();
            for (i, task) in chunk.iter().enumerate() {
                let task_prompt = format!(
                    "{}\n\nSub-task: {}\n\nProvide a concise analysis for this specific sub-task.",
                    prompt, task
                );
                let ak = api_key_shared.clone();
                let bu = base_url_shared.clone();
                let md = model.clone();
                let tn = task.clone();

                handles.push(async move {
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(60))
                        .build();
                    let body = serde_json::json!({
                        "model": md,
                        "messages": [{"role": "user", "content": task_prompt}],
                        "max_tokens": 1024,
                        "temperature": 0.3,
                    });

                    match &client {
                        Ok(c) => {
                            let resp = c
                                .post(&format!("{}/chat/completions", bu))
                                .header("Authorization", format!("Bearer {}", ak))
                                .header("Content-Type", "application/json")
                                .json(&body)
                                .send()
                                .await;
                            match resp {
                                Ok(r) if r.status().is_success() => {
                                    match r.json::<serde_json::Value>().await {
                                        Ok(json) => {
                                            let text = json["choices"][0]["message"]["content"]
                                                .as_str()
                                                .unwrap_or("(no response)")
                                                .to_string();
                                            (i, tn, text)
                                        }
                                        Err(e) => (i, tn, format!("Parse error: {}", e)),
                                    }
                                }
                                Ok(r) => {
                                    let status = r.status();
                                    let body = r.text().await.unwrap_or_default();
                                    (
                                        i,
                                        tn,
                                        format!(
                                            "API error ({}): {}",
                                            status,
                                            body.lines().next().unwrap_or(&body)
                                        ),
                                    )
                                }
                                Err(e) => (i, tn, format!("Request failed: {}", e)),
                            }
                        }
                        Err(e) => (i, tn, format!("Client error: {}", e)),
                    }
                });
            }

            let results = join_all(handles).await;
            all_results.extend(results);
        }

        all_results.sort_by_key(|r| r.0);
        for (idx, task_name, text) in &all_results {
            result.push_str(&format!(
                "── Result {}: {} ──\n{}\n\n",
                idx + 1,
                task_name,
                text
            ));
        }

        ToolResult::ok(result)
    }

    fn requires_permission(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_name() {
        assert_eq!(RlmTool.name(), "rlm");
    }
    #[test]
    fn test_schema() {
        assert!(RlmTool.schema().get("properties").is_some());
    }
    #[tokio::test]
    async fn test_empty_prompt() {
        assert!(
            !RlmTool
                .execute(serde_json::json!({"prompt":""}))
                .await
                .success
        );
    }
    #[tokio::test]
    async fn test_max_parallel_low() {
        assert!(
            !RlmTool
                .execute(serde_json::json!({"prompt":"test","max_parallel":0}))
                .await
                .success
        );
    }
    #[tokio::test]
    async fn test_max_parallel_high() {
        assert!(
            !RlmTool
                .execute(serde_json::json!({"prompt":"test","max_parallel":17}))
                .await
                .success
        );
    }
    #[tokio::test]
    async fn test_valid_execution() {
        let r = RlmTool
            .execute(serde_json::json!({"prompt":"analyze","max_parallel":2}))
            .await;
        assert!(r.success);
        assert!(r.output.contains("RLM Analysis"));
    }
}
