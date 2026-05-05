//! OpenAI-compatible provider
//!
//! Supports: OpenAI, DeepSeek, Ollama, MiniMax, Groq, and any OpenAI-compatible API.

use async_trait::async_trait;
use futures::StreamExt;
use super::*;
use super::provider::{Provider, StreamHandler};

/// OpenAI-compatible provider
#[derive(Debug)]
pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        let base_url = if base_url.is_empty() {
            "https://api.openai.com/v1".to_string()
        } else {
            base_url.trim_end_matches('/').to_string()
        };

        Self {
            api_key,
            base_url,
            model,
        }
    }

    /// Build the request body for the chat completions API
    fn build_request(&self, messages: &[Message], tools: &[ToolDef], config: &GenerateConfig) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages_to_openai(messages),
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
            "stream": true,
        });

        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools.iter().map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    }
                })
            }).collect::<Vec<_>>());
        }

        body
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &str {
        "OpenAI Compatible"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        config: &GenerateConfig,
    ) -> anyhow::Result<StreamHandler> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);

        let client = reqwest::Client::new();
        let request_body = self.build_request(messages, tools, config);

        let request = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body);

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI API error ({}): {}", status, body);
        }

        tokio::spawn(async move {
            if let Err(e) = parse_sse_stream_public(response, tx.clone()).await {
                tracing::error!("SSE parse error: {}", e);
                let _ = tx.send(StreamEvent::Error(e.to_string())).await;
            }
        });

        Ok(rx)
    }
}

/// Parse an SSE stream from the OpenAI API chat completions response.
///
/// Each SSE event contains one `data:` line with a JSON chunk or `[DONE]`.
/// Events are separated by `\n\n` (or `\r\n\r\n`).
pub async fn parse_sse_stream_public(
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
                    Err(_) => break, // wait for more data
                };

                // Handle both \r\n\r\n and \n\n event separators
                if let Some(pos) = s.find("\r\n\r\n") {
                    (s[..pos].to_string(), pos + 4)
                } else if let Some(pos) = s.find("\n\n") {
                    (s[..pos].to_string(), pos + 2)
                } else {
                    break; // need more data
                }
            };

            buf.drain(..event_len);

            // Process each line in the event body
            for line in event_content.lines() {
                let line = line.trim();
                if let Some(data) = line.strip_prefix("data: ") {
                    let data = data.trim();
                    if data == "[DONE]" {
                        let _ = tx
                            .send(StreamEvent::Done {
                                stop_reason: "stop".to_string(),
                                usage: None,
                            })
                            .await;
                        return Ok(());
                    }

                    match serde_json::from_str::<serde_json::Value>(data) {
                        Ok(json) => process_sse_data(json, &tx).await,
                        Err(e) => tracing::warn!("Failed to parse SSE JSON: {} - {}", e, data),
                    }
                }
            }
        }
    }

    // Stream ended without receiving [DONE]; send a graceful Done.
    let _ = tx
        .send(StreamEvent::Done {
            stop_reason: "stop".to_string(),
            usage: None,
        })
        .await;

    Ok(())
}

/// Process a single SSE data event (a JSON delta chunk from the API).
pub async fn process_sse_data(
    json: serde_json::Value,
    tx: &tokio::sync::mpsc::Sender<StreamEvent>,
) {
    let choices = match json.get("choices").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return,
    };

    let first = match choices.first() {
        Some(c) => c,
        None => return,
    };

    // If finish_reason is present, emit Done with optional usage info
    if let Some(reason) = first.get("finish_reason").and_then(|r| r.as_str()) {
        if !reason.is_empty() {
            let usage = json
                .get("usage")
                .and_then(|u| serde_json::from_value::<Usage>(u.clone()).ok());
            let _ = tx
                .send(StreamEvent::Done {
                    stop_reason: reason.to_string(),
                    usage,
                })
                .await;
            return;
        }
    }

    let delta = match first.get("delta") {
        Some(d) => d,
        None => return,
    };

    // Extract text content delta
    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
        if !content.is_empty() {
            let _ = tx.send(StreamEvent::TextChunk(content.to_string())).await;
        }
    }

    // Extract tool call deltas
    if let Some(tool_calls) = delta.get("tool_calls").and_then(|t| t.as_array()) {
        for tc in tool_calls {
            let index = tc.get("index").and_then(|i| i.as_i64()).unwrap_or(0);
            let id = tc
                .get("id")
                .and_then(|i| i.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("call_{}", index));

            let name = tc
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();

            let arguments = tc
                .get("function")
                .and_then(|f| f.get("arguments"))
                .and_then(|a| a.as_str())
                .unwrap_or("{}");

            // Try to parse arguments as JSON; fall back to raw string
            let args_value: serde_json::Value =
                serde_json::from_str(arguments).unwrap_or_else(|_| {
                    serde_json::Value::String(arguments.to_string())
                });

            let _ = tx
                .send(StreamEvent::ToolCallStart(ToolCall {
                    id,
                    name,
                    arguments: args_value,
                }))
                .await;
        }
    }
}

/// Convert internal messages to OpenAI format
fn messages_to_openai(messages: &[Message]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|msg| {
            let role = msg.role.to_string();
            let mut json_msg = serde_json::json!({
                "role": role,
                "content": msg.text(),
            });

            if let Some(tcid) = &msg.tool_call_id {
                json_msg["tool_call_id"] = serde_json::json!(tcid);
            }

            json_msg
        })
        .collect()
}
