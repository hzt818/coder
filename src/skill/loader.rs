//! Skill loader - discovers and loads skills from configuration
//!
//! Supports multi-path discovery from project directories,
//! user directories, and community registry.

use std::path::{Path, PathBuf};
use super::registry::SkillRegistry;

/// Configuration for loading a skill
#[derive(Debug, Clone)]
pub struct SkillConfig {
    /// Name of the skill
    pub name: String,
    /// Optional path to a skill definition file
    pub path: Option<std::path::PathBuf>,
    /// Whether to enable this skill
    pub enabled: bool,
}

/// Loader for discovering and configuring skills.
///
/// Currently supports builtin skills and can be extended to load
/// custom skills from configuration files.
pub struct SkillLoader {
    /// Configuration for known skills
    configs: Vec<SkillConfig>,
    /// List of skill names to load
    enabled_skills: Vec<String>,
}

impl SkillLoader {
    /// Create a new skill loader with default configuration
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
            enabled_skills: vec![
                "brainstorm".to_string(),
                "code_review".to_string(),
                "plan".to_string(),
                "debug".to_string(),
            ],
        }
    }

    /// Load enabled skills into a registry.
    ///
    /// Returns a SkillRegistry with all enabled skills registered.
    pub fn load(&self) -> SkillRegistry {
        let mut registry = SkillRegistry::new();

        for skill_name in &self.enabled_skills {
            match skill_name.as_str() {
                "brainstorm" => {
                    registry.register(super::builtin::brainstorm::BrainstormSkill);
                }
                "code_review" => {
                    registry.register(super::builtin::code_review::CodeReviewSkill);
                }
                "plan" => {
                    registry.register(super::builtin::plan::PlanSkill);
                }
                "debug" => {
                    registry.register(super::builtin::debug::DebugSkill);
                }
                _ => {
                    // Unknown skills are skipped during loading
                }
            }
        }

        registry
    }

    /// Enable a specific skill by name
    pub fn enable(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        if !self.enabled_skills.contains(&name) {
            self.enabled_skills.push(name);
        }
        self
    }

    /// Disable a specific skill by name
    pub fn disable(mut self, name: &str) -> Self {
        self.enabled_skills.retain(|s| s != name);
        self
    }

    /// Get the list of enabled skill names
    pub fn enabled(&self) -> &[String] {
        &self.enabled_skills
    }

    /// Discover skills from all standard locations
    pub fn discover_all(workspace: Option<&Path>) -> Vec<String> {
        let user_dir = crate::util::path::coder_dir();
        let paths = SkillRegistry::discovery_paths(workspace, &user_dir);

        let mut all_skills = Vec::new();
        for path in &paths {
            let found = SkillRegistry::discover_from(path);
            for skill in found {
                if !all_skills.contains(&skill) {
                    all_skills.push(skill);
                }
            }
        }

        all_skills
    }

    /// Add a custom skill configuration
    pub fn with_config(mut self, config: SkillConfig) -> Self {
        self.configs.push(config);
        self
    }
}

impl Default for SkillLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_default() {
        let loader = SkillLoader::new();
        assert_eq!(loader.enabled().len(), 4);
    }

    #[test]
    fn test_loader_enable_disable() {
        let loader = SkillLoader::new().disable("debug").enable("custom");
        assert_eq!(loader.enabled().len(), 4); // debug removed, custom added
        assert!(!loader.enabled().contains(&"debug".to_string()));
        assert!(loader.enabled().contains(&"custom".to_string()));
    }

    #[test]
    fn test_loader_load() {
        let loader = SkillLoader::new();
        let registry = loader.load();
        assert!(registry.get("brainstorm").is_some());
        assert!(registry.get("code_review").is_some());
        assert!(registry.get("plan").is_some());
        assert!(registry.get("debug").is_some());
    }

    #[test]
    fn test_loader_load_partial() {
        let loader = SkillLoader::new().disable("debug");
        let registry = loader.load();
        assert!(registry.get("debug").is_none());
        assert!(registry.get("plan").is_some());
        assert_eq!(registry.len(), 3);
    }
}
