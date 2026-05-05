//! OpenCode OAuth flow
//!
//! Provides OAuth 2.0 authentication for OpenCode services.

use serde::{Deserialize, Serialize};

/// OpenCode OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeOAuth {
    pub client_id: String,
    pub auth_url: String,
    pub token_url: String,
}

impl Default for OpenCodeOAuth {
    fn default() -> Self {
        Self {
            client_id: "coder-desktop".to_string(),
            auth_url: "https://opencode.ai/oauth/authorize".to_string(),
            token_url: "https://opencode.ai/oauth/token".to_string(),
        }
    }
}

/// Result of an OAuth flow
#[derive(Debug, Clone)]
pub enum OAuthResult {
    Success(String), // api_key
    Cancelled,
    Error(String),
}

/// Run the OAuth flow (placeholder - actual flow opens browser)
pub async fn run_oauth_flow() -> OAuthResult {
    OAuthResult::Error("OAuth flow not yet implemented for desktop. Set OPENCODE_API_KEY env var.".to_string())
}
