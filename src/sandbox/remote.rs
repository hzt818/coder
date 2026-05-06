//! Remote sandbox backend
//!
//! Sends commands via HTTP to a remote sandbox API (e.g., Alibaba OpenSandbox).

use async_trait::async_trait;

use super::{SandboxBackend, SandboxResult};

/// Configuration for a remote sandbox backend
#[derive(Debug, Clone)]
pub struct RemoteSandboxConfig {
    pub url: String,
    pub api_key: Option<String>,
}

/// Sandbox backend that delegates command execution to a remote API
#[derive(Debug, Clone)]
pub struct RemoteSandbox {
    config: RemoteSandboxConfig,
    client: reqwest::Client,
}

impl RemoteSandbox {
    /// Create a new RemoteSandbox with the given configuration
    pub fn new(config: RemoteSandboxConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        Self { config, client }
    }

    /// Create a new RemoteSandbox with explicit URL and optional API key
    pub fn new_with_key(url: impl Into<String>, api_key: Option<String>) -> Self {
        Self::new(RemoteSandboxConfig {
            url: url.into(),
            api_key,
        })
    }
}

#[async_trait]
impl SandboxBackend for RemoteSandbox {
    fn name(&self) -> &str {
        "remote"
    }

    async fn execute(&self, command: &str, workdir: &str, timeout_secs: u64) -> SandboxResult {
        let mut req = self.client.post(&self.config.url).json(&serde_json::json!({
            "command": command,
            "workdir": workdir,
            "timeout_secs": timeout_secs,
        }));

        if let Some(ref api_key) = self.config.api_key {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req.send().await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<serde_json::Value>().await {
                    Ok(body) => SandboxResult {
                        stdout: body
                            .get("stdout")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        stderr: body
                            .get("stderr")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        exit_code: body.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(-1)
                            as i32,
                        timed_out: body
                            .get("timed_out")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    },
                    Err(e) => SandboxResult {
                        stdout: String::new(),
                        stderr: format!("Failed to parse response (status {}): {}", status, e),
                        exit_code: -1,
                        timed_out: false,
                    },
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    SandboxResult {
                        stdout: String::new(),
                        stderr: format!("Remote sandbox request timed out after {}s", timeout_secs),
                        exit_code: -1,
                        timed_out: true,
                    }
                } else {
                    SandboxResult {
                        stdout: String::new(),
                        stderr: format!("Remote sandbox request failed: {}", e),
                        exit_code: -1,
                        timed_out: false,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_sandbox_constructor() {
        let sandbox = RemoteSandbox::new_with_key(
            "https://sandbox.example.com/api/execute",
            Some("test-key".into()),
        );
        assert_eq!(sandbox.name(), "remote");
        assert_eq!(
            sandbox.config.url,
            "https://sandbox.example.com/api/execute"
        );
    }

    #[test]
    fn test_remote_sandbox_without_key() {
        let sandbox = RemoteSandbox::new_with_key("https://sandbox.example.com/api/execute", None);
        assert!(sandbox.config.api_key.is_none());
    }

    #[test]
    fn test_remote_sandbox_missing_url() {
        let sandbox = RemoteSandbox::new_with_key("", None);
        assert!(sandbox.config.url.is_empty());
    }
}
