//! Google Gemini provider
//!
//! Uses the Gemini streaming API:
//! - URL: `https://generativelanguage.googleapis.com/v1/models/{model}:streamGenerateContent?alt=sse&key={API_KEY}`
//! - Request: `{"contents":[{"role":"user","parts":[{"text":"hello"}]}]}`
//! - SSE: `data: {"candidates":[{"content":{"parts":[{"text":"Hello"}],"role":"model"}}]}`

use super::provider::{Provider, StreamHandler};
use super::*;
use async_trait::async_trait;
use futures::StreamExt;

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
        tools: &[ToolDef],
        _config: &GenerateConfig,
    ) -> anyhow::Result<StreamHandler> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);

        // Convert messages to Gemini format
        let contents: Vec<serde_json::Value> = messages
            .iter()
            .filter(|m| m.role != crate::ai::Role::System)
            .map(|m| {
                let (role, parts) = match m.role {
                    crate::ai::Role::Assistant => ("model", assistant_parts(m)),
                    crate::ai::Role::Tool => ("function", tool_result_parts(m)),
                    _ => ("user", text_parts(m)),
                };
                serde_json::json!({ "role": role, "parts": parts })
            })
            .collect();

        let mut body = serde_json::json!({
            "contents": contents,
        });

        // Add tool definitions if provided (Gemini function calling)
        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "functionDeclarations": [{
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.input_schema,
                        }]
                    })
                })
                .collect::<Vec<_>>());
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1/models/{}:streamGenerateContent?alt=sse&key={}",
            self.model, self.api_key
        );

        let client = crate::ai::build_http_client();
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

/// Build text-only parts for user/system messages
fn text_parts(msg: &Message) -> Vec<serde_json::Value> {
    msg.content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(serde_json::json!({ "text": text })),
            _ => None,
        })
        .collect()
}

/// Build parts for assistant messages, preserving functionCall blocks
fn assistant_parts(msg: &Message) -> Vec<serde_json::Value> {
    msg.content
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => {
                serde_json::json!({ "text": text })
            }
            ContentBlock::ToolUse { name, input, .. } => {
                serde_json::json!({
                    "functionCall": {
                        "name": name,
                        "args": input,
                    }
                })
            }
            _ => serde_json::Value::Null,
        })
        .filter(|v| !v.is_null())
        .collect()
}

/// Build parts for tool result messages (functionResponse in Gemini)
fn tool_result_parts(msg: &Message) -> Vec<serde_json::Value> {
    msg.content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
            } => {
                // Gemini uses the function name, not tool_use_id; extract from adjacent ToolUse
                // Fallback: use tool_use_id as name
                Some(serde_json::json!({
                    "functionResponse": {
                        "name": tool_use_id,
                        "response": {
                            "content": content,
                        }
                    }
                }))
            }
            _ => None,
        })
        .collect()
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
                // Use lossy conversion to avoid dropping data on invalid UTF-8
                let s = String::from_utf8_lossy(&buf);
                let s: &str = &s;

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
                            tracing::warn!("Failed to parse Gemini SSE JSON: {} - {}", e, data);
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
async fn process_gemini_data(json: serde_json::Value, tx: &tokio::sync::mpsc::Sender<StreamEvent>) {
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
            let usage = json.get("usageMetadata").map(|u| {
                let inp = u
                    .get("promptTokenCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let out = u
                    .get("candidatesTokenCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                Usage {
                    input_tokens: inp,
                    output_tokens: out,
                    total_tokens: u
                        .get("totalTokenCount")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                    cache_hit_tokens: 0,
                    cache_miss_tokens: inp,
                }
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

    // Extract text and function calls from content parts
    let content = match first_candidate.get("content") {
        Some(c) => c,
        None => return,
    };

    let parts = match content.get("parts").and_then(|p| p.as_array()) {
        Some(p) => p,
        None => return,
    };

    for part in parts {
        // Text content
        if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
            if !text.is_empty() {
                let _ = tx.send(StreamEvent::TextChunk(text.to_string())).await;
            }
        }
        // Function call (tool use)
        if let Some(fc) = part.get("functionCall") {
            let name = fc.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
            let args = fc.get("args").cloned().unwrap_or(serde_json::json!({}));
            let id = format!("fc_{}", name);
            let _ = tx
                .send(StreamEvent::ToolCallStart(crate::ai::ToolCall {
                    id,
                    name: name.to_string(),
                    arguments: args,
                }))
                .await;
        }
    }
}
