//! Brainstorm skill - generates creative ideas

use async_trait::async_trait;
use serde_json::json;

use super::{Skill, SkillOutput};

/// Skill for generating creative ideas on a topic
pub struct BrainstormSkill;

#[async_trait]
impl Skill for BrainstormSkill {
    fn name(&self) -> &str {
        "brainstorm"
    }

    fn description(&self) -> &str {
        "Generate creative ideas and solutions for a given topic or problem"
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "topic": {
                    "type": "string",
                    "description": "The topic or problem to brainstorm about"
                },
                "count": {
                    "type": "integer",
                    "description": "Number of ideas to generate (default: 5)",
                    "minimum": 1,
                    "maximum": 20
                },
                "context": {
                    "type": "string",
                    "description": "Additional context or constraints"
                }
            },
            "required": ["topic"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let topic = input
            .get("topic")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Brainstorm skill requires a 'topic' field"))?;

        let count = input
            .get("count")
            .and_then(|v| v.as_i64())
            .unwrap_or(5)
            .min(20)
            .max(1) as usize;

        let context = input.get("context").and_then(|v| v.as_str()).unwrap_or("");

        let mut summary = String::new();
        summary.push_str(&format!("# Brainstorm: {}\n\n", topic));

        if !context.is_empty() {
            summary.push_str(&format!("**Context:** {}\n\n---\n\n", context));
        }

        summary.push_str("## Generated Ideas\n\n");
        for i in 1..=count {
            summary.push_str(&format!(
                "{}. **Idea {}:** Explore an aspect of '{}'\n",
                i, i, topic
            ));
            if i % 3 == 1 {
                summary.push_str("   - What are the fundamental components or first principles?\n");
                summary.push_str("   - How can existing approaches be simplified?\n");
            } else if i % 3 == 2 {
                summary.push_str("   - What adjacent problems could this solve?\n");
                summary.push_str("   - Who else has solved similar challenges?\n");
            } else {
                summary.push_str("   - What constraints or limitations should be considered?\n");
                summary.push_str("   - What resources or skills are needed?\n");
            }
            summary.push_str("\n");
        }

        summary.push_str("## Evaluation Questions\n\n");
        summary.push_str("- Which ideas have the best effort-to-impact ratio?\n");
        summary.push_str("- What key assumptions are we making?\n");
        summary.push_str("- How can we validate these ideas quickly?\n");
        summary.push_str("- Are there any quick wins that can be implemented immediately?\n");

        if count >= 8 {
            summary.push_str("- Which ideas can be combined for greater impact?\n");
        }

        summary.push_str("\n## Next Steps\n\n");
        summary.push_str("1. Select top 2-3 ideas for further exploration\n");
        summary.push_str("2. Create action items for each selected idea\n");
        summary.push_str("3. Set a timeframe for initial validation\n\n");

        let output = SkillOutput::ok_with_data(
            summary,
            json!({
                "topic": topic,
                "ideas_generated": count,
                "has_context": !context.is_empty(),
            }),
        );

        Ok(output.to_json())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_brainstorm_execute() {
        let skill = BrainstormSkill;
        let result = skill
            .execute(json!({ "topic": "Rust project ideas" }))
            .await
            .unwrap();
        assert_eq!(result["success"], true);
        assert!(result["output"]
            .as_str()
            .unwrap()
            .contains("Rust project ideas"));
    }

    #[tokio::test]
    async fn test_brainstorm_missing_topic() {
        let skill = BrainstormSkill;
        let result = skill.execute(json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_brainstorm_with_count() {
        let skill = BrainstormSkill;
        let result = skill
            .execute(json!({ "topic": "ideas", "count": 3 }))
            .await
            .unwrap();
        assert_eq!(result["success"], true);
    }

    #[test]
    fn test_brainstorm_schema() {
        let skill = BrainstormSkill;
        let schema = skill.input_schema();
        assert!(schema.get("required").is_some());
    }
}
