//! Anthropic Claude API provider

use async_trait::async_trait;
use super::*;
use super::provider::{Provider, StreamHandler};

/// Anthropic Claude provider
#[derive(Debug)]
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    model: String,
    api_version: Option<String>,
}

impl AnthropicProvider {
    pub fn new(
        api_key: String,
        base_url: String,
        model: String,
        api_version: Option<String>,
    ) -> Self {
        let base_url = if base_url.is_empty() {
            "https://api.anthropic.com/v1".to_string()
        } else {
            base_url.trim_end_matches('/').to_string()
        };

        Self {
            api_key,
            base_url,
            model,
            api_version,
        }
    }

    fn build_request(&self, messages: &[Message], tools: &[ToolDef], config: &GenerateConfig) -> serde_json::Value {
        let system_content = messages
            .iter()
            .filter(|m| m.role == crate::ai::Role::System)
            .map(|m| m.text())
            .collect::<Vec<_>>()
            .join("\n");

        let non_system: Vec<&Message> = messages
            .iter()
            .filter(|m| m.role != crate::ai::Role::System)
            .collect();

        let mut body = serde_json::json!({
            "model": self.model,
            "max_tokens": config.max_tokens,
            "messages": messages_to_anthropic(&non_system),
            "stream": true,
        });

        if !system_content.is_empty() {
            body["system"] = serde_json::json!(system_content);
        }

        // Add thinking configuration if budget is specified
        if let Some(budget) = config.thinking_budget {
            body["thinking"] = serde_json::json!({
                "type": "enabled",
                "budget_tokens": budget
            });
        }

        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools);
        }

        body
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "Anthropic Claude"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn supports_thinking(&self) -> bool {
        self.model.contains("claude-sonnet")
            || self.model.contains("claude-opus")
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

        let mut request = client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", self.api_version.as_deref().unwrap_or("2023-06-01"))
            .header("Content-Type", "application/json")
            .json(&request_body);

        // Add beta features header for extended thinking
        if config.thinking_budget.is_some() {
            request = request.header("anthropic-beta", "thinking-2025-01-01");
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic API error ({}): {}", status, body);
        }

        let byte_stream = response.bytes_stream();

        tokio::spawn(async move {
            use futures::StreamExt;
            use std::collections::HashMap;

            struct PendingToolCall {
                id: String,
                name: String,
                arguments_json: String,
            }

            let mut stream = byte_stream;
            let mut buf: Vec<u8> = Vec::new();
            let mut current_event = String::new();
            let mut current_data = String::new();
            let mut pending_tool_calls: HashMap<usize, PendingToolCall> = HashMap::new();
            let mut final_stop_reason: Option<String> = None;
            let mut final_usage: Option<Usage> = None;

            while let Some(chunk_result) = stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx.send(StreamEvent::Error(e.to_string())).await;
                        return;
                    }
                };

                buf.extend_from_slice(&chunk);

                // Process all complete lines from the buffer
                loop {
                    let newline_pos = match buf.iter().position(|&b| b == b'\n') {
                        Some(p) => p,
                        None => break,
                    };

                    let mut line_bytes: Vec<u8> = buf.drain(..newline_pos).collect();
                    buf.remove(0); // Remove the '\n'
                    if line_bytes.last() == Some(&b'\r') {
                        line_bytes.pop();
                    }

                    let line_str = String::from_utf8_lossy(&line_bytes);

                    if line_str.is_empty() {
                        // Empty line = end of an SSE event; process accumulated data
                        if !current_data.is_empty() {
                            let data = std::mem::take(&mut current_data);
                            let _ = std::mem::take(&mut current_event);

                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
                                let msg_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

                                match msg_type {
                                    "message_start" => {
                                        if let Some(msg) = json.get("message") {
                                            if let Some(u) = msg.get("usage") {
                                                let inp = u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                                                let out = u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                                                final_usage = Some(Usage {
                                                    input_tokens: inp,
                                                    output_tokens: out,
                                                    total_tokens: inp + out,
                                                    cache_hit_tokens: u.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                                                    cache_miss_tokens: inp,
                                                });
                                            }
                                        }
                                    }
                                    "content_block_start" => {
                                        let index = json.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                                        if let Some(block) = json.get("content_block") {
                                            let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                            match block_type {
                                                "tool_use" => {
                                                    let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                    let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                    pending_tool_calls.insert(index, PendingToolCall { id, name, arguments_json: String::new() });
                                                }
                                                "text" => {
                                                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                                        if !text.is_empty() {
                                                            let _ = tx.send(StreamEvent::TextChunk(text.to_string())).await;
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    "content_block_delta" => {
                                        let index = json.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                                        if let Some(delta) = json.get("delta") {
                                            let delta_type = delta.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                            match delta_type {
                                                "text_delta" => {
                                                    if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                                                        if !text.is_empty() {
                                                            let _ = tx.send(StreamEvent::TextChunk(text.to_string())).await;
                                                        }
                                                    }
                                                }
                                                "input_json_delta" => {
                                                    if let Some(partial) = delta.get("partial_json").and_then(|v| v.as_str()) {
                                                        if let Some(pending) = pending_tool_calls.get_mut(&index) {
                                                            pending.arguments_json.push_str(partial);
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    "content_block_stop" => {
                                        let index = json.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                                        if let Some(pending) = pending_tool_calls.remove(&index) {
                                            let arguments: serde_json::Value = serde_json::from_str(&pending.arguments_json)
                                                .unwrap_or(serde_json::json!({}));
                                            let _ = tx.send(StreamEvent::ToolCallStart(ToolCall {
                                                id: pending.id,
                                                name: pending.name,
                                                arguments,
                                            })).await;
                                        }
                                    }
                                    "message_delta" => {
                                        if let Some(delta) = json.get("delta") {
                                            if let Some(reason) = delta.get("stop_reason").and_then(|v| v.as_str()) {
                                                final_stop_reason = Some(reason.to_string());
                                            }
                                        }
                                        if let Some(u) = json.get("usage") {
                                            let input = final_usage.as_ref().map(|u| u.input_tokens).unwrap_or(0);
                                            let out = u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                                            final_usage = Some(Usage {
                                                input_tokens: input,
                                                output_tokens: out,
                                                total_tokens: input + out,
                                                cache_hit_tokens: 0,
                                                cache_miss_tokens: input,
                                            });
                                        }
                                    }
                                    "message_stop" | "ping" => {}
                                    _ => {}
                                }
                            }
                        }
                    } else if let Some(val) = line_str.strip_prefix("event: ") {
                        current_event = val.to_string();
                    } else if let Some(val) = line_str.strip_prefix("data: ") {
                        current_data = val.to_string();
                    }
                }
            }

            let _ = tx.send(StreamEvent::Done {
                stop_reason: final_stop_reason.unwrap_or_else(|| "end_turn".to_string()),
                usage: final_usage,
            }).await;
        });

        Ok(rx)
    }
}

/// Convert internal messages to Anthropic format
fn messages_to_anthropic(messages: &[&Message]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|msg| {
            let role = match msg.role {
                crate::ai::Role::Assistant => "assistant",
                crate::ai::Role::Tool => "user", // tool_result goes as user in Anthropic
                _ => "user",
            };

            let json_msg = serde_json::json!({
                "role": role,
                "content": msg.text(),
            });

            json_msg
        })
        .collect()
}
