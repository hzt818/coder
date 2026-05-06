//! Custom provider - user-defined request/response templates
//!
//! ⚠️ EXPERIMENTAL: request_template and response_parser parsing is not yet
//! implemented. Falls back to OpenAI-compatible HTTP call when no custom
//! template is provided.

use super::provider::{Provider, StreamHandler};
use super::*;
use async_trait::async_trait;

/// Custom provider with user-defined templates.
///
/// ⚠️ EXPERIMENTAL — request_template/response_parser are parsed but not
/// yet applied. When both are `None`, the provider acts as a generic
/// HTTP passthrough (sends messages as JSON, expects a text response stream).
#[derive(Debug)]
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
        messages: &[Message],
        _tools: &[ToolDef],
        _config: &GenerateConfig,
    ) -> anyhow::Result<StreamHandler> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);

        // When no custom template is set, try a simple JSON POST
        if self.request_template.is_none() && self.response_parser.is_none() {
            let client = crate::ai::build_http_client();
            let body = serde_json::json!({
                "model": self.model,
                "messages": crate::ai::types::messages_to_openai(messages),
                "stream": true,
            });
            let response = client
                .post(&self.base_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        let _ = tx
                            .send(StreamEvent::Error(format!(
                                "Custom provider HTTP error ({}): {}",
                                status, body
                            )))
                            .await;
                        return Ok(rx);
                    }
                    tokio::spawn(async move {
                        if let Err(e) =
                            super::openai::parse_sse_stream_public(resp, tx.clone()).await
                        {
                            tracing::error!("Custom provider SSE parse error: {}", e);
                            let _ = tx.send(StreamEvent::Error(e.to_string())).await;
                        }
                    });
                    Ok(rx)
                }
                Err(e) => {
                    let _ = tx
                        .send(StreamEvent::Error(format!(
                            "Custom provider request failed: {}",
                            e
                        )))
                        .await;
                    Ok(rx)
                }
            }
        } else {
            // Experimental: templates defined but not yet interpreted
            tracing::warn!(
                "Custom provider: request_template ({:?}) and response_parser ({:?}) are stored but not yet applied. \
                 Falling back to direct HTTP POST.",
                self.request_template.as_deref(),
                self.response_parser.as_deref()
            );
            let _ = tx
                .send(StreamEvent::Error(
                    "Custom provider: request_template parsing not yet implemented".to_string(),
                ))
                .await;
            Ok(rx)
        }
    }
}
