//! Plan tool — AI-powered structured planning.
//!
//! Analyzes a goal and constraints using the AI provider (when available)
//! to generate a step-by-step implementation plan. Falls back to a
//! template when no provider is configured.

use super::*;
use async_trait::async_trait;

pub struct PlanTool;

#[async_trait]
impl Tool for PlanTool {
    fn name(&self) -> &str {
        "plan"
    }

    fn description(&self) -> &str {
        "Create a structured plan before implementing complex features. Analyzes requirements and breaks them into actionable steps using AI."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "goal": { "type": "string", "description": "What you want to accomplish" },
                "constraints": { "type": "string", "description": "Any constraints or requirements", "default": "" }
            },
            "required": ["goal"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let goal = args.get("goal").and_then(|g| g.as_str()).unwrap_or("");
        if goal.is_empty() {
            return ToolResult::err("Goal is required");
        }
        let constraints = args
            .get("constraints")
            .and_then(|c| c.as_str())
            .unwrap_or("");

        // Try AI-powered planning first
        if let Some(plan) = ai_generate_plan(goal, constraints).await {
            return ToolResult::ok(plan);
        }

        // Fallback to template plan
        let mut plan = format!("📋 Plan: {}\n\n", goal);
        if !constraints.is_empty() {
            plan.push_str(&format!("Constraints: {}\n\n", constraints));
        }
        plan.push_str("Steps:\n");
        plan.push_str("  1. Analysis — Understand the requirements and existing code\n");
        plan.push_str("  2. Design — Plan the approach, data flow, and interfaces\n");
        plan.push_str("  3. Implementation — Build incrementally with tests\n");
        plan.push_str("  4. Testing — Verify correctness and edge cases\n");
        plan.push_str("  5. Review — Quality check and documentation\n\n");
        plan.push_str("To use AI-powered planning, set OPENAI_API_KEY or CODER_API_KEY.\n");
        ToolResult::ok(plan)
    }

    fn requires_permission(&self) -> bool {
        false
    }
}

/// Generate a plan using the configured AI provider.
async fn ai_generate_plan(goal: &str, constraints: &str) -> Option<String> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("CODER_API_KEY"))
        .ok()?;

    let base_url = std::env::var("OPENAI_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let model = std::env::var("CODER_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    let system_msg = "You are a senior software architect. Given a goal and constraints, produce a structured, actionable implementation plan. Output in this format:\n\nGoal: <restated goal>\n\n## Analysis\n<key considerations>\n\n## Implementation Steps\n1. <step>\n   Details: <what to do>\n   Files: <which files to modify>\n2. ...\n\n## Testing Strategy\n<how to verify>\n\n## Risks\n<potential issues>\n\nKeep steps concrete and specific to the codebase. Include file paths and function names where relevant.";

    let user_msg = if constraints.is_empty() {
        format!("Goal: {}", goal)
    } else {
        format!("Goal: {}\n\nConstraints: {}", goal, constraints)
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .ok()?;

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_msg},
            {"role": "user", "content": user_msg}
        ],
        "max_tokens": 2048,
        "temperature": 0.4,
    });

    let resp = client
        .post(format!("{}/chat/completions", base_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let json: serde_json::Value = resp.json().await.ok()?;
    let text = json["choices"][0]["message"]["content"].as_str()?;

    Some(format!("📋 AI-Generated Plan\n\n{}", text.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_tool_name() {
        assert_eq!(PlanTool.name(), "plan");
    }
    #[test]
    fn test_plan_schema() {
        let schema = PlanTool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }
    #[tokio::test]
    async fn test_plan_empty_goal() {
        assert!(
            !PlanTool
                .execute(serde_json::json!({"goal": ""}))
                .await
                .success
        );
    }
    #[tokio::test]
    async fn test_plan_fallback() {
        let r = PlanTool
            .execute(serde_json::json!({"goal": "Build a web server"}))
            .await;
        assert!(r.success);
        assert!(r.output.contains("Plan:"));
        assert!(r.output.contains("Steps:"));
    }
}
