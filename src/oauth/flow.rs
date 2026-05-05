//! OAuth 2.0 authorization code flow implementation

use super::{
    OAuthError, OAuthProviderConfig, OAuthResult, TokenResponse,
};

/// Manages the OAuth 2.0 authorization code flow
#[derive(Debug, Clone)]
pub struct OAuthFlow {
    /// Provider configuration
    config: OAuthProviderConfig,
    /// Currently stored token
    current_token: Option<TokenResponse>,
    /// HTTP client for token requests
    http_client: reqwest::Client,
}

impl OAuthFlow {
    /// Create a new OAuthFlow from provider configuration
    pub fn new(config: OAuthProviderConfig) -> Self {
        Self {
            config,
            current_token: None,
            http_client: reqwest::Client::new(),
        }
    }

    /// Generate the authorization URL for the browser redirect
    ///
    /// The user should visit this URL to authorize the application.
    pub fn authorization_url(&self, state: &str) -> OAuthResult<String> {
        let mut url = url::Url::parse(&self.config.auth_url)
            .map_err(|e| OAuthError::InvalidConfig(format!("Invalid auth URL: {e}")))?;

        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", &self.config.redirect_uri)
            .append_pair("state", state)
            .append_pair(
                "scope",
                &self.config.scopes.join(" "),
            );

        Ok(url.to_string())
    }

    /// Exchange the authorization code for a token
    pub async fn exchange_code(&mut self, code: &str) -> OAuthResult<TokenResponse> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.config.redirect_uri),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
        ];

        let response = self
            .http_client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| OAuthError::TokenRequestFailed(format!("Request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenRequestFailed(format!(
                "Token endpoint returned {status}: {body}"
            )));
        }

        let token: TokenResponse = response
            .json()
            .await
            .map_err(|e| OAuthError::TokenRequestFailed(format!("Failed to parse token: {e}")))?;

        self.current_token = Some(token.clone());
        Ok(token)
    }

    /// Refresh the access token using a refresh token
    pub async fn refresh_access_token(&mut self, refresh_token: &str) -> OAuthResult<TokenResponse> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
        ];

        let response = self
            .http_client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| OAuthError::TokenRequestFailed(format!("Refresh request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(OAuthError::TokenRequestFailed(format!(
                "Token refresh returned {status}"
            )));
        }

        let token: TokenResponse = response
            .json()
            .await
            .map_err(|e| OAuthError::TokenRequestFailed(format!("Failed to parse token: {e}")))?;

        self.current_token = Some(token.clone());
        Ok(token)
    }

    /// Get the current access token, if available
    pub fn current_token(&self) -> Option<&TokenResponse> {
        self.current_token.as_ref()
    }

    /// Check if the current token is expired (approximate check)
    pub fn is_token_expired(&self) -> bool {
        self.current_token
            .as_ref()
            .map(|t| t.expires_in < 60)
            .unwrap_or(true)
    }

    /// Clear the stored token (force re-authentication)
    pub fn clear_token(&mut self) {
        self.current_token = None;
    }

    /// Get a reference to the provider configuration
    pub fn config(&self) -> &OAuthProviderConfig {
        &self.config
    }

    /// Create a bearer authorization header value from the current token
    pub fn authorization_header(&self) -> Option<String> {
        self.current_token
            .as_ref()
            .map(|t| format!("{} {}", t.token_type, t.access_token))
    }
}

/// Well-known OAuth provider configurations
pub mod providers {
    use super::OAuthProviderConfig;

    /// GitHub OAuth app configuration
    pub fn github(client_id: &str, client_secret: &str) -> OAuthProviderConfig {
        OAuthProviderConfig {
            name: "github".to_string(),
            auth_url: "https://github.com/login/oauth/authorize".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: "http://localhost:3000/oauth/callback".to_string(),
            scopes: vec!["repo".to_string(), "user".to_string()],
        }
    }

    /// Google OAuth 2.0 configuration
    pub fn google(client_id: &str, client_secret: &str) -> OAuthProviderConfig {
        OAuthProviderConfig {
            name: "google".to_string(),
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: "http://localhost:3000/oauth/callback".to_string(),
            scopes: vec![
                "https://www.googleapis.com/auth/userinfo.email".to_string(),
                "https://www.googleapis.com/auth/userinfo.profile".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> OAuthProviderConfig {
        OAuthProviderConfig {
            name: "test".to_string(),
            auth_url: "https://auth.example.com/authorize".to_string(),
            token_url: "https://auth.example.com/token".to_string(),
            client_id: "client123".to_string(),
            client_secret: "secret456".to_string(),
            redirect_uri: "http://localhost/callback".to_string(),
            scopes: vec!["read".to_string()],
        }
    }

    #[test]
    fn test_authorization_url() {
        let flow = OAuthFlow::new(test_config());
        let url = flow.authorization_url("state123").unwrap();
        assert!(url.contains("client_id=client123"));
        assert!(url.contains("state=state123"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope=read"));
    }

    #[test]
    fn test_token_expired_empty() {
        let flow = OAuthFlow::new(test_config());
        assert!(flow.is_token_expired());
    }

    #[test]
    fn test_authorization_header_none() {
        let flow = OAuthFlow::new(test_config());
        assert!(flow.authorization_header().is_none());
    }

    #[test]
    fn test_clear_token() {
        let mut flow = OAuthFlow::new(test_config());
        flow.current_token = Some(TokenResponse {
            access_token: "abc".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: None,
            scope: None,
        });
        assert!(flow.current_token().is_some());
        flow.clear_token();
        assert!(flow.current_token().is_none());
    }

    #[test]
    fn test_providers_github() {
        let config = providers::github("gh_client", "gh_secret");
        assert_eq!(config.name, "github");
        assert!(config.scopes.contains(&"repo".to_string()));
    }

    #[test]
    fn test_providers_google() {
        let config = providers::google("g_client", "g_secret");
        assert_eq!(config.name, "google");
        assert_eq!(config.scopes.len(), 2);
    }

    #[test]
    fn test_config_access() {
        let flow = OAuthFlow::new(test_config());
        assert_eq!(flow.config().client_id, "client123");
    }
}
