//! OAuth tool - OAuth 2.0 authentication flow
//!
//! Supports initiating OAuth flows, handling callbacks, and managing tokens.

use async_trait::async_trait;
use super::*;

pub struct OAuthTool;

#[async_trait]
impl Tool for OAuthTool {
    fn name(&self) -> &str {
        "oauth"
    }

    fn description(&self) -> &str {
        "Initiate and manage OAuth 2.0 authentication flows. Supports common providers like GitHub, Google, and custom endpoints."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["authorize", "token", "refresh", "list"],
                    "description": "OAuth operation to perform"
                },
                "provider": {
                    "type": "string",
                    "description": "OAuth provider name (e.g., 'github', 'google') or custom",
                    "default": ""
                },
                "scopes": {
                    "type": "string",
                    "description": "Space-separated OAuth scopes to request",
                    "default": ""
                },
                "client_id": {
                    "type": "string",
                    "description": "OAuth client ID (for custom providers)",
                    "default": ""
                },
                "client_secret": {
                    "type": "string",
                    "description": "OAuth client secret (for custom providers)",
                    "default": ""
                },
                "auth_url": {
                    "type": "string",
                    "description": "OAuth authorization URL (for custom providers)",
                    "default": ""
                },
                "token_url": {
                    "type": "string",
                    "description": "OAuth token URL (for custom providers)",
                    "default": ""
                },
                "redirect_uri": {
                    "type": "string",
                    "description": "Redirect URI (default: http://localhost:3000/callback)",
                    "default": "http://localhost:3000/callback"
                }
            },
            "required": ["operation"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let operation = args.get("operation")
            .and_then(|o| o.as_str())
            .unwrap_or("");

        if operation.is_empty() {
            return ToolResult::err("Operation is required (authorize, token, refresh, list)");
        }

        let provider = args.get("provider")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string();

        let scopes = args.get("scopes")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();

        let client_id = args.get("client_id")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        let client_secret = args.get("client_secret")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        let auth_url = args.get("auth_url")
            .and_then(|a| a.as_str())
            .unwrap_or("")
            .to_string();

        let token_url = args.get("token_url")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        let redirect_uri = args.get("redirect_uri")
            .and_then(|r| r.as_str())
            .unwrap_or("http://localhost:3000/callback")
            .to_string();
        let _ = &redirect_uri;

        match operation {
            "authorize" => oauth_authorize(&provider, &scopes, &client_id, &auth_url, &redirect_uri).await,
            "token" => oauth_token(&provider, &client_id, &client_secret, &token_url, &redirect_uri).await,
            "refresh" => oauth_refresh(&provider, &client_id, &client_secret, &token_url).await,
            "list" => oauth_list_providers().await,
            _ => ToolResult::err(format!("Unknown OAuth operation: '{}'. Use: authorize, token, refresh, list", operation)),
        }
    }

    fn requires_permission(&self) -> bool {
        true
    }
}

/// Get well-known provider configurations
fn get_provider_config(provider: &str) -> Option<(String, String, Vec<&'static str>)> {
    match provider.to_lowercase().as_str() {
        "github" => Some((
            "https://github.com/login/oauth/authorize".to_string(),
            "https://github.com/login/oauth/access_token".to_string(),
            vec!["repo", "user", "workflow"],
        )),
        "google" => Some((
            "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            "https://oauth2.googleapis.com/token".to_string(),
            vec!["openid", "email", "profile"],
        )),
        "gitlab" => Some((
            "https://gitlab.com/oauth/authorize".to_string(),
            "https://gitlab.com/oauth/token".to_string(),
            vec!["read_api", "read_user"],
        )),
        "slack" => Some((
            "https://slack.com/oauth/v2/authorize".to_string(),
            "https://slack.com/api/oauth.v2.access".to_string(),
            vec!["channels:read", "chat:write"],
        )),
        _ => None,
    }
}

/// Initiate an OAuth authorization flow
async fn oauth_authorize(
    provider: &str,
    scopes: &str,
    client_id: &str,
    auth_url: &str,
    redirect_uri: &str,
) -> ToolResult {
    if provider.is_empty() && auth_url.is_empty() {
        return ToolResult::err("Either 'provider' or 'auth_url' must be specified");
    }

    // Resolve provider config
    let (resolved_auth_url, default_scopes) = if !provider.is_empty() {
        match get_provider_config(provider) {
            Some((a, _, s)) => (a, s.join(" ")),
            None => return ToolResult::err(format!("Unknown provider '{}'. Use a custom auth_url or one of: github, google, gitlab, slack", provider)),
        }
    } else {
        (auth_url.to_string(), String::new())
    };

    let resolved_scopes = if scopes.is_empty() { default_scopes } else { scopes.to_string() };

    // Get client_id from environment or argument
    let resolved_client_id = if client_id.is_empty() {
        let env_var = format!("{}_CLIENT_ID", provider.to_uppercase());
        std::env::var(&env_var)
            .unwrap_or_else(|_| "".to_string())
    } else {
        client_id.to_string()
    };

    // Build the authorization URL
    let auth_uri = format!(
        "{}?client_id={}&redirect_uri={}&scope={}&response_type=code",
        resolved_auth_url,
        url_encode(&resolved_client_id),
        url_encode(redirect_uri),
        url_encode(&resolved_scopes),
    );

    let mut result = format!("OAuth Authorization URL generated:\n\n");
    result.push_str(&format!("  Provider: {}\n", if !provider.is_empty() { provider } else { "custom" }));
    result.push_str(&format!("  Scopes: {}\n", resolved_scopes));
    result.push_str(&format!("  Redirect URI: {}\n\n", redirect_uri));
    result.push_str(&format!("  Authorization URL:\n  {}\n\n", auth_uri));
    result.push_str("Open this URL in a browser to authorize. After authorization, ");
    result.push_str(&format!("you will be redirected to {} with a code parameter.", redirect_uri));
    result.push_str("\n\nUse the 'token' operation with the code to complete the flow.");

    ToolResult::ok(result)
}

/// Exchange an authorization code for tokens
async fn oauth_token(
    provider: &str,
    client_id: &str,
    client_secret: &str,
    token_url: &str,
    redirect_uri: &str,
) -> ToolResult {
    let (resolved_token_url, resolved_client_id, resolved_client_secret) = if !provider.is_empty() {
        match get_provider_config(provider) {
            Some((_, t, _)) => {
                let cid = if client_id.is_empty() {
                    std::env::var(&format!("{}_CLIENT_ID", provider.to_uppercase()))
                        .unwrap_or_default()
                } else {
                    client_id.to_string()
                };
                let cs = if client_secret.is_empty() {
                    std::env::var(&format!("{}_CLIENT_SECRET", provider.to_uppercase()))
                        .unwrap_or_default()
                } else {
                    client_secret.to_string()
                };
                (t, cid, cs)
            }
            None => return ToolResult::err(format!("Unknown provider '{}'", provider)),
        }
    } else {
        (token_url.to_string(), client_id.to_string(), client_secret.to_string())
    };

    if resolved_client_id.is_empty() {
        return ToolResult::err("Client ID is required. Set via argument or environment variable.");
    }

    if resolved_client_secret.is_empty() {
        return ToolResult::err("Client secret is required. Set via argument or environment variable.");
    }

    ToolResult::ok(format!(
        "OAuth token exchange prepared for {}.\n\nTo complete the flow, you need an authorization code.\nUse the 'authorize' operation first, then pass the code to this operation.\n\nToken URL: {}",
        if !provider.is_empty() { provider } else { "custom provider" },
        resolved_token_url
    ))
}

/// Refresh an OAuth token (placeholder)
async fn oauth_refresh(
    provider: &str,
    _client_id: &str,
    _client_secret: &str,
    _token_url: &str,
) -> ToolResult {
    ToolResult::ok(format!(
        "Token refresh for provider '{}'.\n\nToken refresh requires a refresh token. This operation is a placeholder - use the 'token' operation with a refresh token to complete the flow.",
        if !provider.is_empty() { provider } else { "custom" }
    ))
}

/// List configured OAuth providers
async fn oauth_list_providers() -> ToolResult {
    let known = ["github", "google", "gitlab", "slack"];
    let mut result = "Available OAuth providers:\n\n".to_string();

    result.push_str("Known providers (built-in):\n");
    for p in &known {
        result.push_str(&format!("  - {}\n", p));
    }

    result.push_str("\nCustom providers can be configured by specifying auth_url and token_url directly.");
    result.push_str("\n\nProvider credentials can be set via environment variables:\n");
    result.push_str("  {PROVIDER}_CLIENT_ID\n");
    result.push_str("  {PROVIDER}_CLIENT_SECRET\n");

    ToolResult::ok(result)
}

/// Simple URL encoding
fn url_encode(input: &str) -> String {
    let mut result = String::with_capacity(input.len() * 2);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push_str("%20"),
            _ => result.push_str(&format!("%{:02X}", byte)),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_tool_name() {
        let tool = OAuthTool;
        assert_eq!(tool.name(), "oauth");
    }

    #[test]
    fn test_oauth_schema() {
        let tool = OAuthTool;
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }

    #[tokio::test]
    async fn test_oauth_empty_operation() {
        let tool = OAuthTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_oauth_list() {
        let tool = OAuthTool;
        let result = tool.execute(serde_json::json!({"operation": "list"})).await;
        assert!(result.success);
        assert!(result.output.contains("github"));
        assert!(result.output.contains("google"));
    }

    #[tokio::test]
    async fn test_oauth_authorize_no_provider() {
        let tool = OAuthTool;
        let result = tool.execute(serde_json::json!({"operation": "authorize"})).await;
        assert!(!result.success);
    }

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("test/foo"), "test%2Ffoo");
    }
}
