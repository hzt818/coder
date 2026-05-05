//! Google Gemini provider
//!
//! Uses the Gemini streaming API:
//! - URL: `https://generativelanguage.googleapis.com/v1/models/{model}:streamGenerateContent?alt=sse&key={API_KEY}`
//! - Request: `{"contents":[{"role":"user","parts":[{"text":"hello"}]}]}`
//! - SSE: `data: {"candidates":[{"content":{"parts":[{"text":"Hello"}],"role":"model"}}]}`

use async_trait::async_trait;
use futures::StreamExt;
use super::*;
use super::provider::{Provider, StreamHandler};

/// Google Gemini provider
#[derive(Debug)]
pub struct GoogleProvider {
    api_key: String,
    model: String,
}

impl GoogleProvider {
    pub fn new(api_key: String, model: String) -> Self {
        let model = if model.is_empty() {
            "gemini-2.0-flash".to_string()
        } else {
            model
        };
        Self { api_key, model }
    }
}

#[async_trait]
impl Provider for GoogleProvider {
    fn name(&self) -> &str {
        "Google Gemini"
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

        // Convert messages to Gemini format
        // Gemini uses "user" and "model" roles (not "assistant")
        let contents: Vec<serde_json::Value> = messages
            .iter()
            .filter(|m| m.role != crate::ai::Role::System)
            .map(|m| {
                let role = match m.role {
                    crate::ai::Role::Assistant => "model",
                    _ => "user",
                };
                serde_json::json!({
                    "role": role,
                    "parts": [{"text": m.text()}]
                })
            })
            .collect();

        let body = serde_json::json!({
            "contents": contents
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1/models/{}:streamGenerateContent?alt=sse&key={}",
            self.model, self.api_key
        );

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Gemini API error ({}): {}", status, body_text);
        }

        tokio::spawn(async move {
            if let Err(e) = parse_gemini_sse(response, tx.clone()).await {
                tracing::error!("Gemini SSE parse error: {}", e);
                let _ = tx.send(StreamEvent::Error(e.to_string())).await;
            }
        });

        Ok(rx)
    }
}

/// Parse a Gemini SSE stream.
///
/// Each SSE event has a `data:` line with a JSON object.
/// Text content is at `candidates[0].content.parts[0].text`.
/// When `candidates[0].finish_reason` is present, streaming is done.
async fn parse_gemini_sse(
    response: reqwest::Response,
    tx: tokio::sync::mpsc::Sender<StreamEvent>,
) -> anyhow::Result<()> {
    let mut stream = response.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        buf.extend_from_slice(&chunk);

        // Process all complete SSE events in the buffer
        loop {
            let (event_content, event_len) = {
                let s = match std::str::from_utf8(&buf) {
                    Ok(s) => s,
                    Err(_) => break,
                };

                // Handle both \r\n\r\n and \n\n event separators
                if let Some(pos) = s.find("\r\n\r\n") {
                    (s[..pos].to_string(), pos + 4)
                } else if let Some(pos) = s.find("\n\n") {
                    (s[..pos].to_string(), pos + 2)
                } else {
                    break;
                }
            };

            buf.drain(..event_len);

            // Process each line in the event body
            for line in event_content.lines() {
                let line = line.trim();
                if let Some(data) = line.strip_prefix("data: ") {
                    let data = data.trim();
                    if data.is_empty() || data == "[DONE]" {
                        continue;
                    }

                    match serde_json::from_str::<serde_json::Value>(data) {
                        Ok(json) => process_gemini_data(json, &tx).await,
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse Gemini SSE JSON: {} - {}",
                                e,
                                data
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Process a single Gemini SSE data event.
///
/// Extracts text from `candidates[0].content.parts[*].text` and emits
/// `StreamEvent::TextChunk`. When `candidates[0].finish_reason` is present,
/// emits `StreamEvent::Done` with the finish reason and optional usage info.
async fn process_gemini_data(
    json: serde_json::Value,
    tx: &tokio::sync::mpsc::Sender<StreamEvent>,
) {
    let candidates = match json.get("candidates").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return,
    };

    let first_candidate = match candidates.first() {
        Some(c) => c,
        None => return,
    };

    // Check for finish_reason — signals end of stream
    if let Some(finish_reason) = first_candidate
        .get("finish_reason")
        .and_then(|r| r.as_str())
    {
        if !finish_reason.is_empty() {
            let usage = json.get("usageMetadata").map(|u| Usage {
                input_tokens: u
                    .get("promptTokenCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                output_tokens: u
                    .get("candidatesTokenCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                total_tokens: u
                    .get("totalTokenCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
            });

            let _ = tx
                .send(StreamEvent::Done {
                    stop_reason: finish_reason.to_string(),
                    usage,
                })
                .await;
            return;
        }
    }

    // Extract text from content parts
    let content = match first_candidate.get("content") {
        Some(c) => c,
        None => return,
    };

    let parts = match content.get("parts").and_then(|p| p.as_array()) {
        Some(p) => p,
        None => return,
    };

    for part in parts {
        if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
            if !text.is_empty() {
                let _ = tx.send(StreamEvent::TextChunk(text.to_string())).await;
            }
        }
    }
}
