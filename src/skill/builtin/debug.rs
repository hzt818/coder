//! Debug skill - systematic issue diagnosis

use async_trait::async_trait;
use serde_json::json;

use super::{Skill, SkillOutput};

/// Skill for systematically debugging issues
pub struct DebugSkill;

/// A diagnostic hypothesis with a suggested test approach
struct Hypothesis {
    name: String,
    test: String,
}

/// Generate diagnostic hypotheses based on the symptom description
fn generate_hypotheses(symptom: &str) -> Vec<Hypothesis> {
    let lower = symptom.to_lowercase();

    if lower.contains("crash") || lower.contains("panic") {
        vec![
            Hypothesis {
                name: "Unhandled panic or exception".into(),
                test: "Check for unwrap(), expect(), or index [] calls that could panic".into(),
            },
            Hypothesis {
                name: "Null pointer or nil reference".into(),
                test: "Verify all optional values are checked before use".into(),
            },
            Hypothesis {
                name: "Out of memory / stack overflow".into(),
                test: "Check for unbounded recursion or unbounded data structures".into(),
            },
        ]
    } else if lower.contains("slow") || lower.contains("performance") || lower.contains("hang") {
        vec![
            Hypothesis {
                name: "Inefficient algorithm or data structure".into(),
                test: "Profile the application to identify hot paths and bottlenecks".into(),
            },
            Hypothesis {
                name: "Resource contention (lock, network, I/O)".into(),
                test: "Check for lock contention, network latency, or disk bottlenecks".into(),
            },
            Hypothesis {
                name: "Unbounded growth or memory leak".into(),
                test: "Monitor memory usage over time for accumulation patterns".into(),
            },
        ]
    } else if lower.contains("error") || lower.contains("fail") {
        vec![
            Hypothesis {
                name: "Input validation failure".into(),
                test: "Check for malformed, missing, or unexpected input values".into(),
            },
            Hypothesis {
                name: "External dependency unavailable".into(),
                test: "Verify all external services, databases, and APIs are reachable".into(),
            },
            Hypothesis {
                name: "State corruption or race condition".into(),
                test: "Review shared mutable state and synchronization mechanisms".into(),
            },
        ]
    } else if lower.contains("compile") || lower.contains("build") {
        vec![
            Hypothesis {
                name: "Missing or incompatible dependency".into(),
                test: "Check dependency versions and feature flags in Cargo.toml".into(),
            },
            Hypothesis {
                name: "Type mismatch or API change".into(),
                test: "Review the error message for type/function signature mismatches".into(),
            },
            Hypothesis {
                name: "Feature flag mismatch".into(),
                test: "Ensure required features are enabled in the crate configuration".into(),
            },
        ]
    } else {
        vec![
            Hypothesis {
                name: "Configuration or environment issue".into(),
                test: "Verify configuration values, environment variables, and file paths".into(),
            },
            Hypothesis {
                name: "Logic error in business rules".into(),
                test: "Trace through the code path with the specific inputs that trigger the bug".into(),
            },
            Hypothesis {
                name: "Timing or race condition".into(),
                test: "Check for concurrent access patterns without proper synchronization".into(),
            },
        ]
    }
}

#[async_trait]
impl Skill for DebugSkill {
    fn name(&self) -> &str {
        "debug"
    }

    fn description(&self) -> &str {
        "Systematically diagnose and debug issues by analyzing symptoms and suggesting fixes"
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "symptom": {
                    "type": "string",
                    "description": "Description of the bug or issue"
                },
                "code": {
                    "type": "string",
                    "description": "Relevant source code"
                },
                "error_message": {
                    "type": "string",
                    "description": "Error message or stack trace"
                },
                "environment": {
                    "type": "string",
                    "description": "Environment details (OS, versions, etc.)"
                }
            },
            "required": ["symptom"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let symptom = input
            .get("symptom")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Debug skill requires a 'symptom' field"))?;

        let error_message = input.get("error_message").and_then(|v| v.as_str());
        let code = input.get("code").and_then(|v| v.as_str());
        let environment = input.get("environment").and_then(|v| v.as_str());

        let mut output = String::new();
        output.push_str(&format!("# Debug Analysis\n\n"));
        output.push_str(&format!("**Symptom:** {}\n\n", symptom));
        output.push_str("---\n\n");

        // Step 1: Problem clarification
        output.push_str("## Step 1: Understand the Problem\n\n");
        output.push_str(&format!("The reported symptom is: \"{}\"\n\n", symptom));
        if let Some(err) = error_message {
            output.push_str(&format!("**Error message:** `{}`\n\n", err));
        }
        if let Some(env) = environment {
            output.push_str(&format!("**Environment:** {}\n\n", env));
        }
        output.push_str("- [ ] Can you reproduce the issue consistently?\n");
        output.push_str("- [ ] When did this last work correctly?\n");
        output.push_str("- [ ] What changed since then?\n\n");

        // Step 2: Form hypotheses
        output.push_str("## Step 2: Form Hypotheses\n\n");
        let hypotheses = generate_hypotheses(symptom);
        for (i, h) in hypotheses.iter().enumerate() {
            output.push_str(&format!("{}. **{}**\n   - *Test:* {}\n", i + 1, h.name, h.test));
        }
        output.push_str("\n");

        // Step 3: Investigate
        output.push_str("## Step 3: Investigate\n\n");
        output.push_str("1. Check logs, error outputs, and monitoring dashboards\n");
        output.push_str("2. Add targeted logging or tracing around the suspicious area\n");
        output.push_str("3. Isolate the failing component (binary search approach)\n");
        output.push_str("4. Create a minimal reproduction case\n");
        if code.is_some() {
            output.push_str("5. Review the relevant source code section for logic errors\n");
        }
        if error_message.is_some() {
            output.push_str("6. Search for the error message in project issues or documentation\n");
        }
        output.push_str("\n");

        // Step 4: Fix & verify
        output.push_str("## Step 4: Implement & Verify Fix\n\n");
        output.push_str("1. Apply the most likely fix based on hypotheses\n");
        output.push_str("2. Verify the fix resolves the issue without introducing regressions\n");
        output.push_str("3. Run existing tests to check for regressions\n");
        output.push_str("4. Add a regression test for this specific bug\n");
        output.push_str("5. Document the root cause and fix for future reference\n\n");

        // Summary
        output.push_str("---\n\n");
        output.push_str("*Debug analysis provided by coder debug skill. Verify each hypothesis before applying fixes.*\n");

        let output = SkillOutput::ok_with_data(
            output,
            json!({
                "symptom": symptom,
                "has_code": code.is_some(),
                "has_error_message": error_message.is_some(),
                "has_environment": environment.is_some(),
            }),
        );

        Ok(output.to_json())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_debug_execute() {
        let skill = DebugSkill;
        let result = skill
            .execute(json!({ "symptom": "App crashes on startup" }))
            .await
            .unwrap();
        assert_eq!(result["success"], true);
        assert!(result["output"]
            .as_str()
            .unwrap()
            .contains("App crashes on startup"));
    }

    #[tokio::test]
    async fn test_debug_missing_symptom() {
        let skill = DebugSkill;
        let result = skill.execute(json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_debug_with_error() {
        let skill = DebugSkill;
        let result = skill
            .execute(json!({
                "symptom": "Null pointer exception",
                "error_message": "NPE at line 42",
                "code": "println!(x.len())",
                "environment": "Windows 11, Rust 1.70"
            }))
            .await
            .unwrap();
        assert!(result["data"]["has_error_message"] == true);
        assert!(result["data"]["has_code"] == true);
        assert!(result["data"]["has_environment"] == true);
    }
}
