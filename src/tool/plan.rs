//! Plan tool - structured planning mode

use async_trait::async_trait;
use super::*;

pub struct PlanTool;

#[async_trait]
impl Tool for PlanTool {
    fn name(&self) -> &str {
        "plan"
    }

    fn description(&self) -> &str {
        "Create a structured plan before implementing complex features. Analyzes requirements and breaks them into steps."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "goal": {
                    "type": "string",
                    "description": "What you want to accomplish"
                },
                "constraints": {
                    "type": "string",
                    "description": "Any constraints or requirements",
                    "default": ""
                }
            },
            "required": ["goal"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let goal = args.get("goal").and_then(|g| g.as_str()).unwrap_or("");
        if goal.is_empty() {
            return ToolResult::err("Goal is required");
        }
        let constraints = args.get("constraints").and_then(|c| c.as_str()).unwrap_or("");

        let mut plan = format!("📋 Plan: {}\n", goal);
        if !constraints.is_empty() {
            plan.push_str(&format!("Constraints: {}\n", constraints));
        }
        plan.push_str("\nSteps:\n  1. Analysis\n  2. Design\n  3. Implementation\n  4. Testing\n  5. Review\n\n");
        plan.push_str("Use /test to run tests, /lint to check code quality.");
        ToolResult::ok(plan)
    }
}
