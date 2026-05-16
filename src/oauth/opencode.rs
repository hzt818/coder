//! OpenCode OAuth flow
//!
//! Provides OAuth 2.0 authentication for OpenCode services.
//! Implements a complete OAuth 2.0 authorization code flow with:
//! - Local HTTP server for callback
//! - Automatic browser opening
//! - Token exchange and storage

use crate::oauth::{OAuthError, TokenResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tokio::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeOAuth {
    pub client_id: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

impl Default for OpenCodeOAuth {
    fn default() -> Self {
        Self {
            client_id: "coder-desktop".to_string(),
            auth_url: "https://opencode.ai/oauth/authorize".to_string(),
            token_url: "https://opencode.ai/oauth/token".to_string(),
            redirect_uri: "http://127.0.0.1:3001/oauth/callback".to_string(),
            scopes: vec!["api".to_string()],
        }
    }
}

#[derive(Debug, Clone)]
pub enum OAuthFlowState {
    WaitingForAuth,
    AuthCompleted(String),
    AuthFailed(String),
    TokenReady(String),
}

#[derive(Debug, Clone)]
pub enum OAuthResultEnum {
    Success(String),
    Cancelled,
    Error(String),
}

struct OAuthFlowHandler {
    config: OpenCodeOAuth,
    #[allow(dead_code)]
    http_client: reqwest::Client,
}

impl OAuthFlowHandler {
    fn new(config: OpenCodeOAuth) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    fn authorization_url(&self, state: &str) -> Result<String, OAuthError> {
        let mut url = url::Url::parse(&self.config.auth_url)
            .map_err(|e| OAuthError::InvalidConfig(format!("Invalid auth URL: {e}")))?;

        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", &self.config.redirect_uri)
            .append_pair("state", state)
            .append_pair("scope", &self.config.scopes.join(" "));

        Ok(url.to_string())
    }

    async fn exchange_code(&self, code: &str) -> Result<TokenResponse, OAuthError> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.config.redirect_uri),
            ("client_id", &self.config.client_id),
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

        response
            .json()
            .await
            .map_err(|e| OAuthError::TokenRequestFailed(format!("Failed to parse token: {e}")))
    }
}

async fn start_callback_server(
    tx: oneshot::Sender<OAuthFlowState>,
) -> Result<(), OAuthError> {
    use axum::{
        extract::{Query, State},
        response::{Html, IntoResponse, Response},
        routing::get,
        Router,
    };
    use std::net::SocketAddr;

    #[derive(Deserialize)]
    struct CallbackParams {
        code: Option<String>,
        state: Option<String>,
        error: Option<String>,
        error_description: Option<String>,
    }

    type SenderState = Arc<Mutex<Option<oneshot::Sender<OAuthFlowState>>>>;

    async fn callback_handler(
        Query(params): Query<CallbackParams>,
        State(state): State<SenderState>,
    ) -> Response {
        let mut tx_guard = state.lock().await;
        if let Some(tx) = tx_guard.take() {
            if let Some(error) = params.error {
                let msg = params.error_description.unwrap_or_else(|| error.clone());
                let _ = tx.send(OAuthFlowState::AuthFailed(msg));
                return Html("<html><body><h2 style=\"color:red;\">Authentication Failed</h2><p>Please close this window and try again.</p></body></html>").into_response();
            }

            if let Some(code) = params.code {
                let _ = tx.send(OAuthFlowState::AuthCompleted(code.clone()));
                return Html("<html><body><h2 style=\"color:green;\">Authentication Successful!</h2><p>You can close this window and return to Coder.</p><script>setTimeout(() => window.close(), 2000);</script></body></html>").into_response();
            }
        }

        Html("<html><body><h2>Invalid callback</h2></body></html>").into_response()
    }

    let state: SenderState = Arc::new(Mutex::new(Some(tx)));
    let app = Router::new()
        .route("/oauth/callback", get(callback_handler))
        .with_state(state);

    let addr: SocketAddr = "127.0.0.1:3001".parse().unwrap();

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| OAuthError::Http(format!("Failed to bind callback server: {e}")))?;

    tracing::info!("OAuth callback server listening on {}", addr);

    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    Ok(())
}

fn open_browser(url: &str) -> Result<(), OAuthError> {
    open::that(url)
        .map_err(|e| OAuthError::AuthorizationFailed(format!("Failed to open browser: {e}")))
}

pub async fn run_oauth_flow_impl() -> OAuthResultEnum {
    let config = OpenCodeOAuth::default();
    let handler = OAuthFlowHandler::new(config.clone());

    let state = uuid::Uuid::new_v4().to_string();

    let (tx, rx) = oneshot::channel::<OAuthFlowState>();

    if let Err(e) = start_callback_server(tx).await {
        return OAuthResultEnum::Error(format!("Failed to start callback server: {e}"));
    }

    let auth_url = match handler.authorization_url(&state) {
        Ok(url) => url,
        Err(e) => return OAuthResultEnum::Error(format!("Failed to build auth URL: {e}")),
    };

    if let Err(e) = open_browser(&auth_url) {
        tracing::warn!("Failed to open browser: {e}. Please manually visit:");
        tracing::info!("{}", auth_url);
    } else {
        tracing::info!("Opened browser for OAuth authentication");
    }

    tracing::info!("Waiting for OAuth callback at {}", config.redirect_uri);
    tracing::info!("Or visit: {}", auth_url);

    let timeout_duration = Duration::from_secs(300);
    let result = tokio::time::timeout(timeout_duration, rx).await;

    match result {
        Ok(Ok(state)) => {
            match state {
                OAuthFlowState::AuthCompleted(code) => {
                    tracing::info!("Received authorization code, exchanging for token...");

                    match handler.exchange_code(&code).await {
                        Ok(token) => {
                            tracing::info!("Successfully obtained access token");
                            OAuthResultEnum::Success(token.access_token)
                        }
                        Err(e) => OAuthResultEnum::Error(format!("Token exchange failed: {e}")),
                    }
                }
                OAuthFlowState::AuthFailed(msg) => {
                    OAuthResultEnum::Error(format!("Authorization failed: {msg}"))
                }
                _ => OAuthResultEnum::Cancelled,
            }
        }
        Ok(Err(_)) => OAuthResultEnum::Cancelled,
        Err(_) => {
            OAuthResultEnum::Error("OAuth flow timed out (5 minutes). Please try again.".to_string())
        }
    }
}

pub async fn run_oauth_flow() -> OAuthResultEnum {
    if let Ok(api_key) = std::env::var("OPENCODE_API_KEY") {
        if !api_key.is_empty() {
            tracing::info!("Using OPENCODE_API_KEY from environment");
            return OAuthResultEnum::Success(api_key);
        }
    }

    tracing::info!("Starting OpenCode OAuth flow...");
    let result = run_oauth_flow_impl().await;

    match &result {
        OAuthResultEnum::Success(_) => {}
        OAuthResultEnum::Error(msg) => {
            tracing::warn!("OAuth flow failed: {}. Checking environment...", msg);
            if let Ok(api_key) = std::env::var("OPENCODE_API_KEY") {
                if !api_key.is_empty() {
                    tracing::info!("Falling back to OPENCODE_API_KEY from environment");
                    return OAuthResultEnum::Success(api_key);
                }
            }
        }
        OAuthResultEnum::Cancelled => {
            tracing::info!("OAuth flow cancelled by user");
        }
    }

    result
}

pub fn is_oauth_available() -> bool {
    true
}

pub fn get_auth_url() -> Result<String, OAuthError> {
    let config = OpenCodeOAuth::default();
    let handler = OAuthFlowHandler::new(config);
    let state = uuid::Uuid::new_v4().to_string();
    handler.authorization_url(&state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opencode_oauth_default() {
        let oauth = OpenCodeOAuth::default();
        assert_eq!(oauth.client_id, "coder-desktop");
        assert!(oauth.auth_url.contains("opencode.ai"));
        assert!(oauth.redirect_uri.contains("127.0.0.1"));
    }

    #[test]
    fn test_auth_url_generation() {
        let config = OpenCodeOAuth::default();
        let handler = OAuthFlowHandler::new(config);
        let url = handler.authorization_url("test-state").unwrap();
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=coder-desktop"));
        assert!(url.contains("state=test-state"));
    }

    #[test]
    fn test_oauth_available() {
        assert!(is_oauth_available());
    }
}
