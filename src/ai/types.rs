//! Shared AI types for messages, tool calls, and streaming

use serde::{Deserialize, Serialize};

/// Role of a message participant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::Text { text: content.into() }],
            name: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentBlock::Text { text: content.into() }],
            name: None,
            tool_call_id: None,
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: vec![ContentBlock::Text { text: content.into() }],
            name: None,
            tool_call_id: None,
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        let tcid: String = tool_call_id.into();
        Self {
            role: Role::Tool,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: tcid.clone(),
                content: content.into(),
            }],
            name: None,
            tool_call_id: Some(tcid),
        }
    }

    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Convert internal messages to OpenAI-compatible API format.
/// Preserves tool_calls on assistant messages and tool_call_id on tool results.
pub(crate) fn messages_to_openai(messages: &[Message]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|msg| {
            let role = msg.role.to_string();

            match msg.role {
                Role::Assistant => {
                    let text = msg.text();
                    let tool_calls: Vec<serde_json::Value> = msg.content.iter()
                        .filter_map(|block| match block {
                            ContentBlock::ToolUse { id, name, input } => {
                                Some(serde_json::json!({
                                    "id": id,
                                    "type": "function",
                                    "function": {
                                        "name": name,
                                        "arguments": input.to_string(),
                                    }
                                }))
                            }
                            _ => None,
                        })
                        .collect();

                    let mut json_msg = serde_json::json!({
                        "role": role,
                        "content": if text.is_empty() && !tool_calls.is_empty() { serde_json::Value::Null } else { serde_json::json!(text) },
                    });

                    if !tool_calls.is_empty() {
                        json_msg["tool_calls"] = serde_json::json!(tool_calls);
                    }

                    json_msg
                }
                Role::Tool => {
                    let tool_content: String = msg.content.iter()
                        .filter_map(|block| match block {
                            ContentBlock::ToolResult { content, .. } => Some(content.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    let mut json_msg = serde_json::json!({
                        "role": "tool",
                        "content": tool_content,
                    });

                    if let Some(tcid) = &msg.tool_call_id {
                        json_msg["tool_call_id"] = serde_json::json!(tcid);
                    }

                    json_msg
                }
                _ => {
                    serde_json::json!({
                        "role": role,
                        "content": msg.text(),
                    })
                }
            }
        })
        .collect()
}

/// Content block types (text, tool_use, tool_result)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

/// Tool definition sent to the AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// A tool call requested by the AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Token usage statistics with cache breakdown
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    /// Tokens served from cache hit (prefix caching)
    #[serde(default)]
    pub cache_hit_tokens: u64,
    /// Tokens from cache miss (actual input)
    #[serde(default)]
    pub cache_miss_tokens: u64,
}

impl Usage {
    /// Create usage from input/output token counts
    pub fn new(input: u64, output: u64) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            total_tokens: input + output,
            cache_hit_tokens: 0,
            cache_miss_tokens: input,
        }
    }

    /// Set cache breakdown from API response headers
    pub fn with_cache(mut self, hit: u64, miss: u64) -> Self {
        self.cache_hit_tokens = hit;
        self.cache_miss_tokens = miss;
        self
    }

    /// Calculate cost estimate for this usage against a model
    pub fn cost_estimate(&self, model: &str) -> crate::core::pricing::CostEstimate {
        crate::core::pricing::calculate_cost(
            model,
            self.input_tokens,
            self.output_tokens,
            self.cache_hit_tokens,
            self.cache_miss_tokens,
        )
    }
}

/// Event from streaming response
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Text content chunk
    TextChunk(String),
    /// AI requesting a tool call
    ToolCallStart(ToolCall),
    /// Tool call result to send back
    ToolCallResult {
        id: String,
        name: String,
        result: String,
    },
    /// Streaming is done
    Done {
        stop_reason: String,
        usage: Option<Usage>,
    },
    /// An error occurred
    Error(String),
}

/// Configuration for text generation
#[derive(Debug, Clone)]
pub struct GenerateConfig {
    pub max_tokens: u64,
    pub temperature: f64,
    pub top_p: f64,
    pub thinking_budget: Option<u64>,
    pub reasoning_effort: Option<String>,
}

impl Default for GenerateConfig {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            temperature: 0.7,
            top_p: 0.9,
            thinking_budget: None,
            reasoning_effort: None,
        }
    }
}

/// Convert role to API-specific string
impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::System => write!(f, "system"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

impl std::str::FromStr for Role {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Role::User),
            "assistant" => Ok(Role::Assistant),
            "system" => Ok(Role::System),
            "tool" => Ok(Role::Tool),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_user() {
        let msg = Message::user("hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.text(), "hello");
    }

    #[test]
    fn test_role_display() {
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Assistant.to_string(), "assistant");
    }

    #[test]
    fn test_tool_def_serialize() {
        let def = ToolDef {
            name: "bash".to_string(),
            description: "Execute shell commands".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string"}
                }
            }),
        };
        let json = serde_json::to_string(&def).unwrap();
        assert!(json.contains("bash"));
    }
}
