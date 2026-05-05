//! AI Provider configuration

use serde::{Deserialize, Serialize};

/// Configuration for an AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderConfig {
    /// Provider type: "openai", "anthropic", "google", "custom"
    pub provider_type: String,
    /// API key (supports ${ENV_VAR} interpolation)
    pub api_key: Option<String>,
    /// Base URL for the API
    pub base_url: Option<String>,
    /// Model name
    pub model: Option<String>,
    /// API version (Anthropic, etc.)
    pub api_version: Option<String>,
    /// Max tokens for generation
    pub max_tokens: Option<u64>,
    /// Temperature
    pub temperature: Option<f64>,
    /// Top-p sampling
    pub top_p: Option<f64>,
    /// Custom request template (for custom provider type)
    pub request_template: Option<String>,
    /// Custom response parser (for custom provider type)
    pub response_parser: Option<String>,
    /// Additional headers
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider_type: "openai".to_string(),
            api_key: None,
            base_url: None,
            model: None,
            api_version: None,
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: Some(0.9),
            request_template: None,
            response_parser: None,
            extra: std::collections::HashMap::new(),
        }
    }
}
