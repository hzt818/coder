//! Feishu/Lark adapter — run Coder conversations via Feishu.
//!
//! Connects to Feishu Open API, relays messages to the agent loop,
//! and sends responses back.

use super::telegram::ImAdapter;
use async_trait::async_trait;

/// Configuration for the Feishu adapter.
#[derive(Debug, Clone)]
pub struct FeishuConfig {
    pub app_id: String,
    pub app_secret: String,
    pub max_message_length: usize,
}

impl Default for FeishuConfig {
    fn default() -> Self {
        Self {
            app_id: String::new(),
            app_secret: String::new(),
            max_message_length: 4096,
        }
    }
}

/// Feishu adapter for receiving and sending messages.
pub struct FeishuAdapter {
    config: FeishuConfig,
}

impl FeishuAdapter {
    pub fn new(config: FeishuConfig) -> Self {
        Self { config }
    }

    async fn get_access_token(&self) -> anyhow::Result<String> {
        let client = reqwest::Client::new();
        let resp = client
            .post("https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal")
            .json(&serde_json::json!({
                "app_id": self.config.app_id,
                "app_secret": self.config.app_secret,
            }))
            .send()
            .await?;
        let data: serde_json::Value = resp.json().await?;
        data["tenant_access_token"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Failed to get Feishu access token"))
    }
}

#[async_trait]
impl ImAdapter for FeishuAdapter {
    async fn start(&self) -> anyhow::Result<()> {
        if self.config.app_id.is_empty() || self.config.app_secret.is_empty() {
            anyhow::bail!("Feishu app_id and app_secret required");
        }
        tracing::info!("Feishu adapter started");
        Ok(())
    }

    async fn send_message(&self, chat_id: i64, text: &str) -> anyhow::Result<()> {
        let token = self.get_access_token().await?;
        let client = reqwest::Client::new();
        let truncated = &text[..text.len().min(self.config.max_message_length)];
        client
            .post("https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=chat_id")
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({
                "receive_id": chat_id.to_string(),
                "msg_type": "text",
                "content": serde_json::json!({"text": truncated}).to_string(),
            }))
            .send()
            .await?;
        Ok(())
    }

    async fn stop(&self) {
        tracing::info!("Feishu adapter stopped");
    }
}
