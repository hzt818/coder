//! AI Provider system
//!
//! Supports multiple AI providers:
//! - OpenAI compatible (OpenAI, DeepSeek, Ollama, MiniMax, Groq)
//! - Anthropic (Claude)
//! - Google (Gemini)
//! - Custom (user-defined request/response templates)

pub mod provider;
pub mod types;
pub mod openai;
pub mod anthropic;
pub mod google;
pub mod custom;
#[cfg(feature = "ai-opencode")]
pub mod opencode;

pub use provider::Provider;
pub use types::*;

/// Create a provider instance from configuration.
pub fn create_provider(
    name: &str,
    config: crate::config::ProviderConfig,
    model_override: Option<String>,
) -> anyhow::Result<Box<dyn Provider>> {
    let model = model_override.or(config.model.clone()).unwrap_or_default();

    match config.provider_type.as_str() {
        "openai" => Ok(Box::new(openai::OpenAIProvider::new(
            config.api_key.unwrap_or_default(),
            config.base_url.unwrap_or_default(),
            model,
        ))),
        "anthropic" => Ok(Box::new(anthropic::AnthropicProvider::new(
            config.api_key.unwrap_or_default(),
            config.base_url.unwrap_or_default(),
            model,
            config.api_version,
        ))),
        "google" => Ok(Box::new(google::GoogleProvider::new(
            config.api_key.unwrap_or_default(),
            model,
        ))),
        "custom" => Ok(Box::new(custom::CustomProvider::new(
            config.api_key.unwrap_or_default(),
            config.base_url.unwrap_or_default(),
            model,
            config.request_template,
            config.response_parser,
        ))),
        "opencode" => {
            #[cfg(feature = "ai-opencode")]
            {
                Ok(Box::new(opencode::OpenCodeProvider::new(
                    config.api_key,
                    config.base_url,
                    model,
                )))
            }
            #[cfg(not(feature = "ai-opencode"))]
            anyhow::bail!("OpenCode provider requires 'ai-opencode' feature")
        }
        other => anyhow::bail!("Unknown provider type: {}", other),
    }
}
