//! Tool-Skill Bridge - connects Tools with Skills for unified capabilities
//!
//! Provides:
//! - Tool-based skill invocation
//! - Skill result to tool format conversion
//! - Unified capability registry

use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::OnceLock;

static TOOL_SKILL_BRIDGE: OnceLock<RwLock<ToolSkillBridgeState>> = OnceLock::new();

pub struct ToolSkillBridgeState {
    skill_tool_map: HashMap<String, String>,
    enabled_skills: Vec<String>,
}

impl ToolSkillBridgeState {
    pub fn new() -> Self {
        let mut skill_tool_map = HashMap::new();

        skill_tool_map.insert("brainstorm".to_string(), "skill_brainstorm".to_string());
        skill_tool_map.insert("code-review".to_string(), "skill_code_review".to_string());
        skill_tool_map.insert("plan".to_string(), "skill_plan".to_string());
        skill_tool_map.insert("debug".to_string(), "skill_debug".to_string());
        skill_tool_map.insert("tdd".to_string(), "skill_tdd".to_string());

        let enabled_skills = vec![
            "brainstorm".to_string(),
            "code-review".to_string(),
            "plan".to_string(),
            "debug".to_string(),
        ];

        Self {
            skill_tool_map,
            enabled_skills,
        }
    }
}

fn get_state() -> &'static RwLock<ToolSkillBridgeState> {
    TOOL_SKILL_BRIDGE.get_or_init(|| RwLock::new(ToolSkillBridgeState::new()))
}

pub fn init() {
    let _ = get_state();
    tracing::info!("Tool-Skill bridge initialized");
}

pub fn get_tool_for_skill(skill_name: &str) -> Option<String> {
    if let Ok(state) = get_state().read() {
        return state.skill_tool_map.get(skill_name).cloned();
    }
    None
}

pub fn is_skill_enabled(skill_name: &str) -> bool {
    if let Ok(state) = get_state().read() {
        return state.enabled_skills.contains(&skill_name.to_string());
    }
    false
}

pub fn enable_skill(skill_name: &str) {
    if let Ok(mut state) = get_state().write() {
        if !state.enabled_skills.contains(&skill_name.to_string()) {
            state.enabled_skills.push(skill_name.to_string());
            tracing::info!("Skill '{}' enabled", skill_name);
        }
    }
}

pub fn disable_skill(skill_name: &str) {
    if let Ok(mut state) = get_state().write() {
        state.enabled_skills.retain(|n| n != skill_name);
        tracing::info!("Skill '{}' disabled", skill_name);
    }
}

pub fn list_enabled_skills() -> Vec<String> {
    if let Ok(state) = get_state().read() {
        return state.enabled_skills.clone();
    }
    Vec::new()
}

#[cfg(feature = "skill")]
pub async fn invoke_skill_async(
    skill_name: &str,
    input: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    use crate::skill::builtin::Skill;
    if !is_skill_enabled(skill_name) {
        anyhow::bail!("Skill '{}' is not enabled", skill_name);
    }

    match skill_name {
        "brainstorm" => {
            let skill = crate::skill::builtin::brainstorm::BrainstormSkill;
            skill.execute(input).await
        }
        "code-review" => {
            let skill = crate::skill::builtin::code_review::CodeReviewSkill;
            skill.execute(input).await
        }
        "plan" => {
            let skill = crate::skill::builtin::plan::PlanSkill;
            skill.execute(input).await
        }
        "debug" => {
            let skill = crate::skill::builtin::debug::DebugSkill;
            skill.execute(input).await
        }
        other => anyhow::bail!("Unknown skill: {}", other),
    }
}

#[cfg(not(feature = "skill"))]
pub async fn invoke_skill_async(
    _skill_name: &str,
    _input: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    anyhow::bail!("Skill feature not enabled")
}

#[cfg(feature = "skill")]
pub fn invoke_skill_blocking(
    skill_name: &str,
    input: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    if !is_skill_enabled(skill_name) {
        anyhow::bail!("Skill '{}' is not enabled", skill_name);
    }

    let rt = tokio::runtime::Handle::current();
    rt.block_on(async { invoke_skill_async(skill_name, input).await })
}

#[cfg(not(feature = "skill"))]
pub fn invoke_skill_blocking(
    _skill_name: &str,
    _input: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    anyhow::bail!("Skill feature not enabled")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        init();
        assert!(!list_enabled_skills().is_empty());
    }

    #[test]
    fn test_get_tool_for_skill() {
        init();
        assert_eq!(get_tool_for_skill("brainstorm"), Some("skill_brainstorm".to_string()));
        assert_eq!(get_tool_for_skill("unknown"), None);
    }

    #[test]
    fn test_skill_enabled() {
        init();
        assert!(is_skill_enabled("brainstorm"));
        assert!(!is_skill_enabled("nonexistent"));
    }

    #[test]
    fn test_enable_disable() {
        init();
        disable_skill("brainstorm");
        assert!(!is_skill_enabled("brainstorm"));
        enable_skill("brainstorm");
        assert!(is_skill_enabled("brainstorm"));
    }
}
