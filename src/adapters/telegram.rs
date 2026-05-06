//! Telegram adapter — run Coder conversations via Telegram.
//!
//! Connects to Telegram Bot API, relays messages to the agent loop,
//! and sends responses back. Operates as an independent sidecar.

use async_trait::async_trait;

/// Configuration for the Telegram adapter.
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub allowed_chat_ids: Vec<i64>,
    pub max_message_length: usize,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            allowed_chat_ids: Vec::new(),
            max_message_length: 4096,
        }
    }
}

/// Represents a message received from Telegram.
#[derive(Debug, Clone)]
pub struct TelegramMessage {
    pub chat_id: i64,
    pub user_id: i64,
    pub username: Option<String>,
    pub text: String,
    pub message_id: i64,
}

/// Adapter trait for IM platforms.
#[async_trait]
pub trait ImAdapter: Send + Sync {
    /// Start listening for messages.
    async fn start(&self) -> anyhow::Result<()>;
    /// Send a message to a chat.
    async fn send_message(&self, chat_id: i64, text: &str) -> anyhow::Result<()>;
    /// Stop the adapter.
    async fn stop(&self);
}

pub struct TelegramAdapter {
    config: TelegramConfig,
}

impl TelegramAdapter {
    pub fn new(config: TelegramConfig) -> Self {
        Self { config }
    }

    /// Get updates from Telegram Bot API.
    #[allow(dead_code)]
    async fn get_updates(&self, offset: &mut i64) -> anyhow::Result<Vec<TelegramMessage>> {
        let client = reqwest::Client::new();
        let url = format!(
            "https://api.telegram.org/bot{}/getUpdates",
            self.config.bot_token
        );
        let resp = client
            .post(&url)
            .json(&serde_json::json!({
                "offset": offset,
                "timeout": 30,
                "allowed_updates": ["message"]
            }))
            .send()
            .await?;
        let data: serde_json::Value = resp.json().await?;
        let mut messages = Vec::new();

        if let Some(results) = data["result"].as_array() {
            for update in results {
                if let Some(msg) = update["message"].as_object() {
                    let chat_id = msg["chat"]["id"].as_i64().unwrap_or(0);
                    let user_id = msg["from"]["id"].as_i64().unwrap_or(0);
                    let text = msg["text"].as_str().unwrap_or("").to_string();
                    let message_id = msg["message_id"].as_i64().unwrap_or(0);

                    if !self.config.allowed_chat_ids.is_empty()
                        && !self.config.allowed_chat_ids.contains(&chat_id)
                    {
                        continue;
                    }

                    messages.push(TelegramMessage {
                        chat_id,
                        user_id,
                        username: msg["from"]["username"].as_str().map(String::from),
                        text,
                        message_id,
                    });
                }
                if let Some(update_id) = update["update_id"].as_i64() {
                    *offset = update_id + 1;
                }
            }
        }
        Ok(messages)
    }
}

#[async_trait]
impl ImAdapter for TelegramAdapter {
    async fn start(&self) -> anyhow::Result<()> {
        if self.config.bot_token.is_empty() {
            anyhow::bail!("Telegram bot token not configured");
        }
        tracing::info!("Telegram adapter started");
        Ok(())
    }

    async fn send_message(&self, chat_id: i64, text: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.config.bot_token
        );
        let truncated = &text[..text.len().min(self.config.max_message_length)];
        client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": truncated,
                "parse_mode": "Markdown"
            }))
            .send()
            .await?;
        Ok(())
    }

    async fn stop(&self) {
        tracing::info!("Telegram adapter stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telegram_config_default() {
        let config = TelegramConfig::default();
        assert!(config.bot_token.is_empty());
        assert!(config.allowed_chat_ids.is_empty());
        assert_eq!(config.max_message_length, 4096);
    }

    #[test]
    fn test_adapter_start_without_token() {
        let config = TelegramConfig::default();
        let adapter = TelegramAdapter::new(config);
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(adapter.start());
        assert!(result.is_err());
    }
}
