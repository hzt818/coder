//! Plan skill - creates implementation plans

use async_trait::async_trait;
use serde_json::json;

use super::{Skill, SkillOutput};

/// Skill for creating structured implementation plans
pub struct PlanSkill;

#[async_trait]
impl Skill for PlanSkill {
    fn name(&self) -> &str {
        "plan"
    }

    fn description(&self) -> &str {
        "Create a structured implementation plan with steps, dependencies, and estimates"
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "goal": {
                    "type": "string",
                    "description": "The goal or feature to plan"
                },
                "context": {
                    "type": "string",
                    "description": "Background context and constraints"
                },
                "detail_level": {
                    "type": "string",
                    "description": "Level of detail: 'high', 'medium', 'detailed'",
                    "enum": ["high", "medium", "detailed"]
                }
            },
            "required": ["goal"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let goal = input
            .get("goal")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Plan skill requires a 'goal' field"))?;

        let context = input
            .get("context")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let detail_level = input
            .get("detail_level")
            .and_then(|v| v.as_str())
            .unwrap_or("medium");

        let mut plan = String::new();
        plan.push_str(&format!("# Implementation Plan: {}\n\n", goal));

        if !context.is_empty() {
            plan.push_str(&format!("> **Context:** {}\n\n", context));
        }

        plan.push_str(&format!("**Detail level:** {} | **Goal:** {}\n\n---\n\n", detail_level, goal));

        // Phase 1: Analysis & Requirements
        plan.push_str("## Phase 1: Analysis & Requirements\n\n");
        plan.push_str("- [ ] Define clear acceptance criteria\n");
        plan.push_str("- [ ] Identify dependencies and risks\n");
        plan.push_str("- [ ] Determine success metrics\n");
        if detail_level == "detailed" {
            plan.push_str("- [ ] Stakeholder review and sign-off\n");
            plan.push_str("- [ ] Document edge cases and error scenarios\n");
            plan.push_str("- [ ] Define performance requirements\n");
        }
        plan.push_str("\n");

        // Phase 2: Design
        plan.push_str("## Phase 2: Design\n\n");
        plan.push_str("- [ ] Architecture and component design\n");
        plan.push_str("- [ ] Interface/API definitions\n");
        plan.push_str("- [ ] Data model design\n");
        plan.push_str("- [ ] Design review\n");
        if detail_level == "detailed" {
            plan.push_str("- [ ] Security review of the design\n");
            plan.push_str("- [ ] Create technical specification document\n");
        }
        plan.push_str("\n");

        // Phase 3: Implementation
        plan.push_str("## Phase 3: Implementation\n\n");
        plan.push_str("- [ ] Set up development environment\n");
        plan.push_str("- [ ] Implement core functionality (TDD approach)\n");
        plan.push_str("- [ ] Write unit tests\n");
        plan.push_str("- [ ] Implement error handling and edge cases\n");
        if detail_level != "high" {
            plan.push_str("- [ ] Integration tests\n");
        }
        if detail_level == "detailed" {
            plan.push_str("- [ ] Performance benchmarks\n");
            plan.push_str("- [ ] Documentation of implementation details\n");
        }
        plan.push_str("\n");

        // Phase 4: Review & Quality
        plan.push_str("## Phase 4: Review & Quality\n\n");
        plan.push_str("- [ ] Code review\n");
        plan.push_str("- [ ] Static analysis / linting\n");
        plan.push_str("- [ ] Security audit\n");
        plan.push_str("- [ ] Documentation update\n");
        plan.push_str("\n");

        // Phase 5: Deployment
        plan.push_str("## Phase 5: Deployment\n\n");
        plan.push_str("- [ ] Build and package\n");
        plan.push_str("- [ ] Staging deployment and validation\n");
        plan.push_str("- [ ] Production deployment\n");
        plan.push_str("- [ ] Monitoring and rollback plan\n");
        plan.push_str("\n");

        // Dependencies table
        plan.push_str("## Dependencies\n\n");
        plan.push_str("| Dependency | Type | Status |\n");
        plan.push_str("|------------|------|--------|\n");
        plan.push_str("| Core language/framework | Runtime | Available |\n");
        plan.push_str("| Testing framework | Build | Available |\n");
        plan.push_str("| CI/CD pipeline | Infrastructure | Needed |\n");
        plan.push_str("| Documentation | Process | Needed |\n");
        plan.push_str("\n---\n\n");
        plan.push_str("*Plan generated by coder plan skill. Adjust phases and tasks based on your specific needs.*\n");

        let output = SkillOutput::ok_with_data(
            plan,
            json!({
                "goal": goal,
                "has_context": !context.is_empty(),
                "detail_level": detail_level,
            }),
        );

        Ok(output.to_json())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plan_execute() {
        let skill = PlanSkill;
        let result = skill
            .execute(json!({ "goal": "Add authentication" }))
            .await
            .unwrap();
        assert_eq!(result["success"], true);
        assert!(result["output"].as_str().unwrap().contains("authentication"));
    }

    #[tokio::test]
    async fn test_plan_missing_goal() {
        let skill = PlanSkill;
        let result = skill.execute(json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_plan_with_context() {
        let skill = PlanSkill;
        let result = skill
            .execute(json!({
                "goal": "Refactor CLI",
                "context": "Must be backwards compatible",
                "detail_level": "detailed"
            }))
            .await
            .unwrap();
        assert!(result["data"]["has_context"] == true);
        assert_eq!(result["data"]["detail_level"], "detailed");
    }
}
