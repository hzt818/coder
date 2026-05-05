//! OAuth module - OAuth 2.0 authentication flows
//!
//! Provides OAuth 2.0 authorization code flow for integrating
//! with external services and APIs.

pub mod flow;
#[cfg(feature = "ai-opencode")]
pub mod opencode;

pub use flow::OAuthFlow;

/// Result type for OAuth operations
pub type OAuthResult<T> = std::result::Result<T, OAuthError>;

/// Errors that can occur during OAuth operations
#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    /// OAuth flow failed
    #[error("OAuth flow failed: {0}")]
    FlowFailed(String),
    /// Token request failed
    #[error("Token request failed: {0}")]
    TokenRequestFailed(String),
    /// Authorization failed
    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),
    /// Token is expired
    #[error("Token expired: {0}")]
    TokenExpired(String),
    /// Invalid configuration
    #[error("Invalid OAuth configuration: {0}")]
    InvalidConfig(String),
    /// HTTP request failed
    #[error("HTTP error: {0}")]
    Http(String),
}

/// OAuth 2.0 token response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenResponse {
    /// Access token
    pub access_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Expiry time in seconds
    pub expires_in: u64,
    /// Refresh token (if provided)
    pub refresh_token: Option<String>,
    /// Scope of the token
    pub scope: Option<String>,
}

/// OAuth 2.0 provider configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OAuthProviderConfig {
    /// Provider name (e.g., "github", "google")
    pub name: String,
    /// Authorization endpoint URL
    pub auth_url: String,
    /// Token endpoint URL
    pub token_url: String,
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Redirect URI
    pub redirect_uri: String,
    /// Default scopes
    pub scopes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_error_display() {
        let err = OAuthError::FlowFailed("user cancelled".to_string());
        assert_eq!(err.to_string(), "OAuth flow failed: user cancelled");
    }

    #[test]
    fn test_token_response_serialization() {
        let token = TokenResponse {
            access_token: "abc123".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some("refresh123".to_string()),
            scope: Some("repo,user".to_string()),
        };
        let json = serde_json::to_string(&token).unwrap();
        let deserialized: TokenResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.access_token, "abc123");
        assert_eq!(deserialized.token_type, "Bearer");
    }
}
