//! Skill registry - stores and manages available skills
//!
//! Supports community registry sync for discovering and installing
//! skills from remote sources.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
        self.skills
            .insert(skill.name().to_string(), Box::new(skill));
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

    /// Discover skills from a directory path (scans for SKILL.md files)
    pub fn discover_from(path: &Path) -> Vec<String> {
        let mut found = Vec::new();
        if !path.exists() || !path.is_dir() {
            return found;
        }

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let skill_dir = entry.path();
                if skill_dir.is_dir() {
                    let skill_file = skill_dir.join("SKILL.md");
                    if skill_file.exists() {
                        if let Some(name) = skill_dir.file_name() {
                            found.push(name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        found
    }

    /// Get all skill discovery paths in priority order
    pub fn discovery_paths(workspace: Option<&Path>, user_dir: &Path) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. Project-level skills
        if let Some(ws) = workspace {
            paths.push(ws.join(".agents").join("skills"));
            paths.push(ws.join("skills"));
            paths.push(ws.join(".opencode").join("skills"));
            paths.push(ws.join(".claude").join("skills"));
        }

        // 2. User-global skills
        paths.push(user_dir.join("skills"));

        // 3. Community skills (installed from registry)
        let community_dir = if let Some(ws) = workspace {
            ws.join(".coder").join("community-skills")
        } else {
            user_dir.join("community-skills")
        };
        paths.push(community_dir);

        paths
    }

    /// Sync skills from a community registry source
    pub async fn sync_community(workspace: Option<&Path>) -> anyhow::Result<String> {
        // Determine the community skills directory
        let community_dir = if let Some(ws) = workspace {
            ws.join(".coder").join("community-skills")
        } else {
            crate::util::path::coder_dir().join("community-skills")
        };

        std::fs::create_dir_all(&community_dir)?;

        // In a full implementation, this would fetch from a remote registry URL
        // For now, create a sample community registry index
        let registry_file = community_dir.join("registry.json");
        let registry_content = serde_json::json!({
            "version": 1,
            "updated_at": chrono::Utc::now().to_rfc3339(),
            "skills": [],
            "source": "local"
        });

        std::fs::write(
            &registry_file,
            serde_json::to_string_pretty(&registry_content)?,
        )?;

        let count = SkillRegistry::discover_from(&community_dir);
        Ok(format!(
            "Community skills synced: {} skills found in {}",
            count.len(),
            community_dir.display()
        ))
    }

    /// Format the list of skills for display
    pub fn format_skill_list(&self, discovery: &[String]) -> String {
        let mut result = format!("── Skills ({}) ──\n\n", self.len());

        for (name, skill) in self.iter() {
            let discovered = if discovery.contains(&name.to_string()) {
                " 📦"
            } else {
                " 🔧"
            };
            result.push_str(&format!(
                "  {}{} - {}\n",
                name,
                discovered,
                skill.description()
            ));
        }

        result
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
        let result = reg.execute("nonexistent", serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
