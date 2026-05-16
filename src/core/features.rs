//! Feature gate module - manages conditional compilation features
//!
//! Provides runtime checks for feature availability and graceful degradation
//! when features are not enabled at compile time.

#[derive(Debug, Clone)]
pub struct FeatureStatus {
    pub name: String,
    pub enabled: bool,
    pub description: String,
}

pub fn get_all_features() -> Vec<FeatureStatus> {
    vec![
        FeatureStatus {
            name: "ai-openai".to_string(),
            enabled: cfg!(feature = "ai-openai"),
            description: "OpenAI API provider support".to_string(),
        },
        FeatureStatus {
            name: "ai-anthropic".to_string(),
            enabled: cfg!(feature = "ai-anthropic"),
            description: "Anthropic Claude API provider support".to_string(),
        },
        FeatureStatus {
            name: "ai-google".to_string(),
            enabled: cfg!(feature = "ai-google"),
            description: "Google Gemini API provider support".to_string(),
        },
        FeatureStatus {
            name: "ai-opencode".to_string(),
            enabled: cfg!(feature = "ai-opencode"),
            description: "OpenCode free tier support".to_string(),
        },
        FeatureStatus {
            name: "tools-git".to_string(),
            enabled: cfg!(feature = "tools-git"),
            description: "Git operations tool".to_string(),
        },
        FeatureStatus {
            name: "tools-docker".to_string(),
            enabled: cfg!(feature = "tools-docker"),
            description: "Docker management tool".to_string(),
        },
        FeatureStatus {
            name: "tools-db".to_string(),
            enabled: cfg!(feature = "tools-db"),
            description: "Database query tool".to_string(),
        },
        FeatureStatus {
            name: "tools-oauth".to_string(),
            enabled: cfg!(feature = "tools-oauth"),
            description: "OAuth tool for third-party integrations".to_string(),
        },
        FeatureStatus {
            name: "team".to_string(),
            enabled: cfg!(feature = "team"),
            description: "Multi-agent team collaboration".to_string(),
        },
        FeatureStatus {
            name: "skill".to_string(),
            enabled: cfg!(feature = "skill"),
            description: "Skill system for reusable capabilities".to_string(),
        },
        FeatureStatus {
            name: "subagent".to_string(),
            enabled: cfg!(feature = "subagent"),
            description: "Subagent spawning system".to_string(),
        },
        FeatureStatus {
            name: "memory".to_string(),
            enabled: cfg!(feature = "memory"),
            description: "Cross-session memory persistence".to_string(),
        },
        FeatureStatus {
            name: "storage".to_string(),
            enabled: cfg!(feature = "storage"),
            description: "Database storage layer".to_string(),
        },
        FeatureStatus {
            name: "server".to_string(),
            enabled: cfg!(feature = "server"),
            description: "HTTP API server".to_string(),
        },
        FeatureStatus {
            name: "mcp".to_string(),
            enabled: cfg!(feature = "mcp"),
            description: "Model Context Protocol support".to_string(),
        },
        FeatureStatus {
            name: "lsp".to_string(),
            enabled: cfg!(feature = "lsp"),
            description: "Language Server Protocol integration".to_string(),
        },
        FeatureStatus {
            name: "oauth".to_string(),
            enabled: cfg!(feature = "oauth"),
            description: "OAuth authentication flows".to_string(),
        },
        FeatureStatus {
            name: "analytics".to_string(),
            enabled: cfg!(feature = "analytics"),
            description: "Analytics and usage tracking".to_string(),
        },
        FeatureStatus {
            name: "permission".to_string(),
            enabled: cfg!(feature = "permission"),
            description: "Permission and access control".to_string(),
        },
        FeatureStatus {
            name: "computer".to_string(),
            enabled: cfg!(feature = "computer"),
            description: "Computer control (keyboard/mouse)".to_string(),
        },
        FeatureStatus {
            name: "worktree".to_string(),
            enabled: cfg!(feature = "worktree"),
            description: "Git worktree management".to_string(),
        },
        FeatureStatus {
            name: "voice".to_string(),
            enabled: cfg!(feature = "voice"),
            description: "Voice input/output support".to_string(),
        },
        FeatureStatus {
            name: "sync".to_string(),
            enabled: cfg!(feature = "sync"),
            description: "Cloud synchronization".to_string(),
        },
    ]
}

pub fn is_feature_enabled(name: &str) -> bool {
    match name {
        "ai-openai" => cfg!(feature = "ai-openai"),
        "ai-anthropic" => cfg!(feature = "ai-anthropic"),
        "ai-google" => cfg!(feature = "ai-google"),
        "ai-opencode" => cfg!(feature = "ai-opencode"),
        "tools-git" => cfg!(feature = "tools-git"),
        "tools-docker" => cfg!(feature = "tools-docker"),
        "tools-db" => cfg!(feature = "tools-db"),
        "tools-oauth" => cfg!(feature = "tools-oauth"),
        "team" => cfg!(feature = "team"),
        "skill" => cfg!(feature = "skill"),
        "subagent" => cfg!(feature = "subagent"),
        "memory" => cfg!(feature = "memory"),
        "storage" => cfg!(feature = "storage"),
        "server" => cfg!(feature = "server"),
        "mcp" => cfg!(feature = "mcp"),
        "lsp" => cfg!(feature = "lsp"),
        "oauth" => cfg!(feature = "oauth"),
        "analytics" => cfg!(feature = "analytics"),
        "permission" => cfg!(feature = "permission"),
        "computer" => cfg!(feature = "computer"),
        "worktree" => cfg!(feature = "worktree"),
        "voice" => cfg!(feature = "voice"),
        "sync" => cfg!(feature = "sync"),
        _ => false,
    }
}

pub fn get_enabled_features() -> Vec<String> {
    get_all_features()
        .into_iter()
        .filter(|f| f.enabled)
        .map(|f| f.name)
        .collect()
}

pub fn format_feature_report() -> String {
    let features = get_all_features();
    let enabled: Vec<_> = features.iter().filter(|f| f.enabled).collect();
    let disabled: Vec<_> = features.iter().filter(|f| !f.enabled).collect();

    let mut report = String::from("# Feature Report\n\n");
    report.push_str(&format!("**Enabled features:** {}\n\n", enabled.len()));

    report.push_str("## Enabled\n\n");
    for f in &enabled {
        report.push_str(&format!("- ✅ `{}` - {}\n", f.name, f.description));
    }

    report.push_str("\n## Disabled\n\n");
    for f in &disabled {
        report.push_str(&format!("- ❌ `{}` - {}\n", f.name, f.description));
    }

    report.push_str("\n---\n");
    report.push_str("*Enable features with: `cargo build --features \"feature1,feature2,...\"`*\n");

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_features() {
        let features = get_all_features();
        assert!(!features.is_empty());
    }

    #[test]
    fn test_is_feature_enabled() {
        #[cfg(feature = "ai-openai")]
        assert!(is_feature_enabled("ai-openai"));

        #[cfg(not(feature = "ai-openai"))]
        assert!(!is_feature_enabled("ai-openai"));
    }

    #[test]
    fn test_get_enabled_features() {
        let enabled = get_enabled_features();
        // Default features should always be available
        #[cfg(feature = "ai-openai")]
        assert!(enabled.contains(&"ai-openai".to_string()));

        #[cfg(feature = "ai-anthropic")]
        assert!(enabled.contains(&"ai-anthropic".to_string()));
    }

    #[test]
    fn test_feature_report() {
        let report = format_feature_report();
        assert!(report.contains("# Feature Report"));
        assert!(report.contains("Enabled features:"));
    }
}
