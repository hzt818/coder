//! Configuration system for coder
//!
//! Configuration priority (highest to lowest):
//! 1. CLI arguments (--model, --provider)
//! 2. Environment variables (CODER_*)
//! 3. Project config (./coder.toml)
//! 4. User config (~/.coder/config.toml)
//! 5. Default values

pub mod provider_config;
pub mod settings;
pub mod theme;

pub use provider_config::ProviderConfig;
pub use settings::Settings;
pub use settings::UiSettings;
