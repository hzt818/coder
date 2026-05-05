//! Main settings struct for coder configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::provider_config::ProviderConfig;

/// Top-level settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// AI-related configuration
    pub ai: AiSettings,
    /// UI/TUI configuration
    pub ui: UiSettings,
    /// Tool configuration
    pub tools: ToolSettings,
    /// Session configuration
    pub session: SessionSettings,
    /// Storage configuration
    pub storage: StorageSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ai: AiSettings::default(),
            ui: UiSettings::default(),
            tools: ToolSettings::default(),
            session: SessionSettings::default(),
            storage: StorageSettings::default(),
        }
    }
}

impl Settings {
    /// Load settings from a config file.
    /// Falls back to default config if not found.
    pub fn load(path: Option<&str>) -> anyhow::Result<Self> {
        let config_path = if let Some(p) = path {
            std::path::PathBuf::from(p)
        } else {
            // Try user config first, then project config
            let user_config = crate::util::path::coder_dir().join("config.toml");
            let project_config = std::path::PathBuf::from("./coder.toml");

            if user_config.exists() {
                user_config
            } else if project_config.exists() {
                project_config
            } else {
                return Ok(Self::default());
            }
        };

        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| anyhow::anyhow!("Failed to read config {}: {}", config_path.display(), e))?;

        let mut settings: Settings = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;

        // Resolve environment variable references like ${VAR_NAME}
        settings.resolve_env_vars();

        Ok(settings)
    }

    /// Check for unknown config keys that may indicate typos
    fn check_unknown_keys(&self) {
        for (name, provider) in &self.ai.providers {
            if !provider.extra.is_empty() {
                let keys: Vec<&String> = provider.extra.keys().collect();
                tracing::warn!(
                    "Provider '{}' has unknown config keys: {:?}. These may be typos or misspellings. \
                     Supported keys: provider_type, api_key, base_url, model, api_version, \
                     max_tokens, temperature, top_p, request_template, response_parser",
                    name, keys
                );
            }
        }
    }

    /// Resolve `${VAR_NAME}` patterns in string fields using environment variables.
    fn resolve_env_vars(&mut self) {
        self.check_unknown_keys();
        for (_name, provider) in self.ai.providers.iter_mut() {
            if let Some(key) = &provider.api_key {
                provider.api_key = Some(Self::resolve_env(key));
            }
            if let Some(url) = &provider.base_url {
                provider.base_url = Some(Self::resolve_env(url));
            }
        }
    }

    fn resolve_env(value: &str) -> String {
        if value.starts_with("${") && value.ends_with('}') {
            let var_name = &value[2..value.len() - 1];
            match std::env::var(var_name) {
                Ok(v) => v,
                Err(_) => {
                    tracing::warn!(
                        "Environment variable '{}' is not set (referenced as '{}'). The API key will be empty.",
                        var_name, value
                    );
                    String::new()
                }
            }
        } else {
            value.to_string()
        }
    }
}

/// AI provider configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiSettings {
    /// Default provider name
    pub default_provider: String,
    /// Provider configurations keyed by name
    pub providers: HashMap<String, ProviderConfig>,
}

impl Default for AiSettings {
    fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                provider_type: "openai".to_string(),
                api_key: Some("${OPENAI_API_KEY}".to_string()),
                base_url: Some("https://api.openai.com/v1".to_string()),
                model: Some("gpt-4o".to_string()),
                ..Default::default()
            },
        );

        providers.insert(
            "opencode".to_string(),
            ProviderConfig {
                provider_type: "opencode".to_string(),
                api_key: None,  // None = anonymous/free tier
                base_url: Some("https://opencode.ai/zen/v1".to_string()),
                model: Some("claude-sonnet-4-6".to_string()),
                ..Default::default()
            },
        );

        Self {
            default_provider: "opencode".to_string(),
            providers,
        }
    }
}

/// UI/TUI settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    /// Color theme name
    pub theme: String,
    /// Show line numbers in code blocks
    pub show_line_numbers: bool,
    /// Enable syntax highlighting
    pub syntax_highlight: bool,
    /// Mouse support
    pub mouse_support: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: "coder-dark".to_string(),
            show_line_numbers: true,
            syntax_highlight: true,
            mouse_support: true,
        }
    }
}

/// Tool settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolSettings {
    /// Require confirmation before executing tools
    pub confirm_before_exec: bool,
    /// Default timeout in seconds for tool execution
    pub timeout_seconds: u64,
    /// Maximum output size in bytes (truncate beyond this)
    pub max_output_bytes: u64,
    /// Allowed tool names (empty = all allowed)
    pub allowed_tools: Vec<String>,
}

impl Default for ToolSettings {
    fn default() -> Self {
        Self {
            confirm_before_exec: false,
            timeout_seconds: 300,
            max_output_bytes: 1_000_000,
            allowed_tools: Vec::new(),
        }
    }
}

/// Session settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionSettings {
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
    /// Maximum messages before compaction
    pub max_messages_before_compact: usize,
    /// Maximum context tokens
    pub max_context_tokens: u64,
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            auto_save_interval: 60,
            max_messages_before_compact: 100,
            max_context_tokens: 128_000,
        }
    }
}

/// Storage settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageSettings {
    /// Database type (sqlite, postgres)
    pub db_type: String,
    /// Database URL
    pub db_url: String,
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            db_type: "sqlite".to_string(),
            db_url: "".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert_eq!(settings.ai.default_provider, "opencode");
        assert!(settings.ai.providers.contains_key("opencode"));
        assert!(settings.ai.providers.contains_key("openai"));
    }

    #[test]
    fn test_resolve_env() {
        std::env::set_var("CODER_TEST_KEY", "test-value");
        let resolved = Settings::resolve_env("${CODER_TEST_KEY}");
        assert_eq!(resolved, "test-value");
    }

    #[test]
    fn test_resolve_env_no_match() {
        let resolved = Settings::resolve_env("plain text");
        assert_eq!(resolved, "plain text");
    }
}
