//! WebSocket handler for real-time communication
//!
//! Clients connect to `ws://host/api/ws` and send/receive JSON messages.
//! This provides a bidirectional channel suitable for streaming events
//! (chat completions, tool execution results, etc.).

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use futures::SinkExt;
use futures::StreamExt;
use serde_json::Value;

use super::AppState;

/// WebSocket upgrade handler.
///
/// Accepts an upgrade request and spawns a handler task for the
/// connection.  The handler reads JSON messages from the client and
/// streams back events.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Incoming message from the WebSocket client.
#[derive(Debug, serde::Deserialize)]
struct WsIncoming {
    /// Message type: `"chat"`, `"ping"`, `"tool_exec"`.
    #[serde(rename = "type")]
    msg_type: String,
    /// Optional session identifier.
    #[serde(default)]
    session_id: String,
    /// Message payload.
    #[serde(default)]
    payload: serde_json::Value,
}

/// Outgoing event sent to the WebSocket client.
#[derive(Debug, serde::Serialize)]
struct WsOutgoing {
    /// Event type: `"text"`, `"done"`, `"error"`, `"pong"`, `"tool_result"`.
    #[serde(rename = "type")]
    event_type: String,
    /// Event data.
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

impl WsOutgoing {
    fn new(event_type: impl Into<String>, data: Option<serde_json::Value>) -> Self {
        Self {
            event_type: event_type.into(),
            data,
        }
    }
}

/// Process a single WebSocket connection.
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    // Use split for concurrent read/write.
    let (mut sender, mut receiver) = socket.split();

    // Clone what we need so we don't hold the state lock across awaits.
    let tools = state.tool_registry.clone();

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let response = handle_message(&text, &tools).await;
                        let json = serde_json::to_string(&response)
                            .unwrap_or_else(|_| r#"{"type":"error","data":"serialization failed"}"#.to_string());
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        let _ = sender.send(Message::Pong(data)).await;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Parse and handle an incoming WebSocket text message.
async fn handle_message(text: &str, tools: &crate::tool::ToolRegistry) -> WsOutgoing {
    let incoming: WsIncoming = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            return WsOutgoing::new(
                "error",
                Some(serde_json::json!({"message": format!("invalid message: {}", e)})),
            );
        }
    };

    match incoming.msg_type.as_str() {
        "ping" => WsOutgoing::new("pong", Some(incoming.payload)),
        "tool_exec" => {
            let tool_name = match incoming.payload.get("name").and_then(|v| v.as_str()) {
                Some(n) => n,
                None => {
                    return WsOutgoing::new(
                        "error",
                        Some(serde_json::json!({"message": "missing 'name' in payload"})),
                    );
                }
            };
            let args = incoming.payload.get("args").cloned().unwrap_or(Value::Null);
            let result = tools.execute(tool_name, args).await;
            WsOutgoing::new(
                "tool_result",
                Some(serde_json::json!({
                    "name": tool_name,
                    "success": result.success,
                    "output": result.output,
                    "error": result.error,
                })),
            )
        }
        other => WsOutgoing::new(
            "error",
            Some(serde_json::json!({"message": format!("unknown message type: {}", other)})),
        ),
    }
}
