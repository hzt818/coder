//! Skill registry - stores and manages available skills

use std::collections::BTreeMap;

use super::builtin::Skill;

/// Registry of available skills, stored in a BTreeMap for deterministic ordering.
pub struct SkillRegistry {
    skills: BTreeMap<String, Box<dyn Skill>>,
}

impl SkillRegistry {
    /// Create an empty skill registry
    pub fn new() -> Self {
        Self {
            skills: BTreeMap::new(),
        }
    }

    /// Create a registry pre-populated with all builtin skills
    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        reg.register(super::builtin::brainstorm::BrainstormSkill);
        reg.register(super::builtin::code_review::CodeReviewSkill);
        reg.register(super::builtin::plan::PlanSkill);
        reg.register(super::builtin::debug::DebugSkill);
        reg
    }

    /// Register a skill
    pub fn register(&mut self, skill: impl Skill + 'static) -> &mut Self {
        self.skills.insert(skill.name().to_string(), Box::new(skill));
        self
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<&Box<dyn Skill>> {
        self.skills.get(name)
    }

    /// Execute a skill by name with the given input
    pub async fn execute(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        match self.skills.get(name) {
            Some(skill) => skill.execute(input).await,
            None => anyhow::bail!("Skill '{}' not found in registry", name),
        }
    }

    /// List all registered skill names
    pub fn skill_names(&self) -> Vec<String> {
        self.skills.keys().cloned().collect()
    }

    /// Number of registered skills
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Get all skills as an iterator of (name, &Skill) pairs
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Box<dyn Skill>)> {
        self.skills.iter().map(|(k, v)| (k.as_str(), v))
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_empty() {
        let reg = SkillRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[tokio::test]
    async fn test_registry_with_builtins() {
        let reg = SkillRegistry::with_builtins();
        assert_eq!(reg.len(), 4);
        let names = reg.skill_names();
        assert!(names.contains(&"brainstorm".to_string()));
        assert!(names.contains(&"code_review".to_string()));
        assert!(names.contains(&"plan".to_string()));
        assert!(names.contains(&"debug".to_string()));
    }

    #[tokio::test]
    async fn test_execute_unknown_skill() {
        let reg = SkillRegistry::new();
        let result = reg
            .execute("nonexistent", serde_json::json!({}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
