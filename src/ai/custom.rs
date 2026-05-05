//! Custom provider - user-defined request/response templates

use async_trait::async_trait;
use super::*;
use super::provider::{Provider, StreamHandler};

/// Custom provider with user-defined templates
#[derive(Debug)]
#[allow(dead_code)]
pub struct CustomProvider {
    api_key: String,
    base_url: String,
    model: String,
    request_template: Option<String>,
    response_parser: Option<String>,
}

impl CustomProvider {
    pub fn new(
        api_key: String,
        base_url: String,
        model: String,
        request_template: Option<String>,
        response_parser: Option<String>,
    ) -> Self {
        Self {
            api_key,
            base_url,
            model,
            request_template,
            response_parser,
        }
    }
}

#[async_trait]
impl Provider for CustomProvider {
    fn name(&self) -> &str {
        "Custom"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn chat_stream(
        &self,
        _messages: &[Message],
        _tools: &[ToolDef],
        _config: &GenerateConfig,
    ) -> anyhow::Result<StreamHandler> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);

        // Stub: return an error message
        let _ = tx.send(StreamEvent::Error(
            "Custom provider: request_template parsing not yet implemented".to_string(),
        )).await;

        Ok(rx)
    }
}
