//! OpenAI-compatible provider
//!
//! Supports: OpenAI, DeepSeek, Ollama, MiniMax, Groq, and any OpenAI-compatible API.

use super::provider::{Provider, StreamHandler};
use super::*;
use async_trait::async_trait;
use futures::StreamExt;

/// OpenAI-compatible provider
///
/// Supports: OpenAI, DeepSeek, Ollama, MiniMax, Groq, and any OpenAI-compatible API.
/// When no API key is configured, returns deterministic mock responses (useful for testing).
#[derive(Debug)]
pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    model: String,
    client: reqwest::Client,
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
            client: crate::ai::build_http_client(),
        }
    }

    /// Create a provider with an injected `reqwest::Client`.
    ///
    /// Useful for tests using `httpmock` or other mock HTTP servers.
    pub fn new_with_client(
        client: reqwest::Client,
        api_key: String,
        base_url: String,
        model: String,
    ) -> Self {
        let base_url = if base_url.is_empty() {
            "https://api.openai.com/v1".to_string()
        } else {
            base_url.trim_end_matches('/').to_string()
        };

        Self {
            api_key,
            base_url,
            model,
            client,
        }
    }

    /// Returns `true` when no real API key is configured (empty or all-whitespace).
    fn has_no_api_key(&self) -> bool {
        self.api_key.trim().is_empty()
    }

    /// Build the request body for the chat completions API
    fn build_request(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        config: &GenerateConfig,
    ) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": crate::ai::types::messages_to_openai(messages),
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
            "stream": true,
        });

        // Add reasoning effort if specified (for DeepSeek / OpenAI reasoning models)
        if let Some(effort) = &config.reasoning_effort {
            if !effort.is_empty() {
                // OpenAI-compatible reasoning_effort parameter
                body["reasoning_effort"] = serde_json::json!(effort);
            }
        }

        // thinking_budget is Anthropic-only; NOT sent to OpenAI-compatible APIs
        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.input_schema,
                        }
                    })
                })
                .collect::<Vec<_>>());
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

        // When no API key is configured, return a deterministic mock response.
        // This keeps the binary runnable for testing without real credentials.
        if self.has_no_api_key() {
            let prompt = messages
                .iter()
                .filter_map(|m| m.content.first())
                .filter_map(|c| match c {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ");
            let mock = format!("mocked-openai-response to: {}", &prompt[..prompt.len().min(120)]);
            tokio::spawn(async move {
                tx.send(StreamEvent::TextChunk(mock)).await.ok();
                tx.send(StreamEvent::Done {
                    stop_reason: "stop".to_string(),
                    usage: None,
                })
                .await
                .ok();
            });
            return Ok(rx);
        }

        let request_body = self.build_request(messages, tools, config);

        let request = self
            .client
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
            let args_value: serde_json::Value = serde_json::from_str(arguments)
                .unwrap_or_else(|_| serde_json::Value::String(arguments.to_string()));

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn openai_provider_mock_when_no_key() {
        let p = OpenAIProvider::new("".to_string(), "".to_string(), "gpt-test".to_string());
        assert!(p.has_no_api_key());

        let msgs = vec![Message::user("hello")];
        let mut stream = p.chat_stream(&msgs, &[], &GenerateConfig::default()).await.unwrap();

        let mut got_text = String::new();
        while let Some(event) = stream.recv().await {
            if let StreamEvent::TextChunk(t) = event {
                got_text.push_str(&t);
            }
        }
        assert!(got_text.contains("mocked-openai-response"));
    }

    #[tokio::test]
    async fn openai_provider_calls_mock_server() {
        let server = httpmock::MockServer::start();
        // SSE-style response: each line is a "data: <json>" event, terminated by [DONE]
        let sse_body = "data: {\"choices\":[{\"delta\":{\"content\":\"hello-response\"},\"index\":0}]}\n\ndata: [DONE]\n\n";

        let mock = server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/chat/completions");
            then.status(200)
                .header("content-type", "text/event-stream")
                .body(sse_body);
        });

        let client = reqwest::Client::new();
        let base = server.url("/");
        let p = OpenAIProvider::new_with_client(
            client,
            "testkey".to_string(),
            base,
            "gpt-test".to_string(),
        );

        let msgs = vec![Message::user("input prompt")];
        let mut stream = p.chat_stream(&msgs, &[], &GenerateConfig::default()).await.unwrap();

        let mut got_text = String::new();
        while let Some(event) = stream.recv().await {
            match event {
                StreamEvent::TextChunk(t) => got_text.push_str(&t),
                StreamEvent::Done { .. } => break,
                _ => {}
            }
        }
        assert!(got_text.contains("hello-response"), "got: {}", got_text);
        mock.assert();
    }
}
