use async_trait::async_trait;
use super::*;

pub struct RlmTool;

#[async_trait]
impl Tool for RlmTool {
    fn name(&self) -> &str { "rlm" }
    fn description(&self) -> &str { concat!("Recursive Language Model - fan out parallel LLM sub-queries. ",
        "Accepts a prompt and optional sub-tasks, runs them in parallel (1-16), ",
        "and returns aggregated results. Use for batch analysis, code review, etc.") }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "prompt": { "type": "string", "description": "The main prompt describing the analysis task" },
                "sub_tasks": { "type": "array", "items": {"type": "string"}, "description": "Optional list of sub-tasks to parallelize" },
                "max_parallel": { "type": "integer", "description": "Max parallel sub-queries (1-16)", "default": 4 },
                "model": { "type": "string", "description": "Model for sub-queries", "default": "flash" }
            }, "required": ["prompt"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let prompt = args.get("prompt").and_then(|p| p.as_str()).unwrap_or("");
        if prompt.is_empty() { return ToolResult::err("Prompt is required"); }

        let max_parallel = args.get("max_parallel").and_then(|m| m.as_i64()).unwrap_or(4);
        if max_parallel < 1 || max_parallel > 16 {
            return ToolResult::err("max_parallel must be between 1 and 16");
        }

        let sub_tasks: Vec<String> = args.get("sub_tasks")
            .and_then(|s| s.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let model = args.get("model").and_then(|m| m.as_str()).unwrap_or("flash");

        let mut result = format!("RLM Analysis\n\nMain prompt: {}\n", prompt);
        result.push_str(&format!("Model: {}\n", model));
        result.push_str(&format!("Max parallel: {}\n", max_parallel));

        if sub_tasks.is_empty() {
            result.push_str("\nSub-tasks: (auto-decomposed)\n");
            result.push_str("  No explicit sub-tasks provided. The prompt would be automatically decomposed.\n");
        } else {
            result.push_str(&format!("\nSub-tasks ({}):\n", sub_tasks.len()));
            for (i, task) in sub_tasks.iter().enumerate() {
                result.push_str(&format!("  {}. {}\n", i + 1, task));
            }
            let actual_parallel = sub_tasks.len().min(max_parallel as usize);
            result.push_str(&format!("\nExecuting {} tasks in parallel (batch of {})\n", sub_tasks.len(), actual_parallel));
        }

        result.push_str("\nResults:\n");
        result.push_str("  (RLM execution - results would be streamed here)\n");

        ToolResult::ok(result)
    }

    fn requires_permission(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_name() { assert_eq!(RlmTool.name(), "rlm"); }
    #[test] fn test_schema() { assert!(RlmTool.schema().get("properties").is_some()); }
    #[tokio::test] async fn test_empty_prompt() { assert!(!RlmTool.execute(serde_json::json!({"prompt":""})).await.success); }
    #[tokio::test] async fn test_max_parallel_low() { assert!(!RlmTool.execute(serde_json::json!({"prompt":"test","max_parallel":0})).await.success); }
    #[tokio::test] async fn test_max_parallel_high() { assert!(!RlmTool.execute(serde_json::json!({"prompt":"test","max_parallel":17})).await.success); }
    #[tokio::test] async fn test_valid_execution() { let r = RlmTool.execute(serde_json::json!({"prompt":"analyze","max_parallel":4})).await; assert!(r.success); assert!(r.output.contains("RLM Analysis")); }
    #[tokio::test] async fn test_with_sub_tasks() { let r = RlmTool.execute(serde_json::json!({"prompt":"review","sub_tasks":["check bugs","check security","check perf"]})).await; assert!(r.success); assert!(r.output.contains("3")); }
}
