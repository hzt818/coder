//! MCP server - exposes coder's tools to other MCP clients
//!
//! Implements the MCP server side, making coder's ToolRegistry
//! available as MCP tools that external AI agents can discover and use.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::tool::ToolRegistry;

/// MCP server that exposes coder tools via stdio transport.
pub struct McpServer {
    registry: Arc<ToolRegistry>,
    running: Arc<std::sync::atomic::AtomicBool>,
    next_id: Arc<std::sync::atomic::AtomicU64>,
}

impl McpServer {
    /// Create a new MCP server with the given tool registry.
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self {
            registry,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            next_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// Start the MCP server, reading JSON-RPC requests from stdin
    /// and writing responses to stdout.
    pub async fn start(&self) -> anyhow::Result<()> {
        self.running.store(true, Ordering::SeqCst);
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        while self.running.load(Ordering::SeqCst) {
            let mut content_length: Option<usize> = None;

            // Read headers line by line
            loop {
                line.clear();
                let bytes_read = reader.read_line(&mut line).await
                    .map_err(|e| anyhow::anyhow!("Error reading stdin: {}", e))?;
                if bytes_read == 0 {
                    self.running.store(false, Ordering::SeqCst);
                    return Ok(());
                }
                let trimmed = line.trim_end_matches("\r\n").trim_end_matches('\n');
                if trimmed.is_empty() {
                    // End of headers
                    break;
                }
                if let Some(len_str) = trimmed.strip_prefix("Content-Length:") {
                    if let Ok(len) = len_str.trim().parse::<usize>() {
                        content_length = Some(len);
                    }
                }
            }

            // Read the JSON body
            if let Some(len) = content_length {
                let mut buf = vec![0u8; len];
                reader.read_exact(&mut buf).await
                    .map_err(|e| anyhow::anyhow!("Failed to read request body: {}", e))?;

                if let Ok(request) = serde_json::from_slice::<serde_json::Value>(&buf) {
                    let response = self.handle_request(&request).await;
                    self.write_response(&response).await?;
                }
            }
        }

        Ok(())
    }

    /// Write a JSON-RPC response with Content-Length header to stdout.
    async fn write_response(&self, response: &serde_json::Value) -> anyhow::Result<()> {
        let content = serde_json::to_string(response)?;
        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        let mut stdout = tokio::io::stdout();
        stdout.write_all(header.as_bytes()).await?;
        stdout.write_all(content.as_bytes()).await?;
        stdout.flush().await?;

        Ok(())
    }

    /// Handle a single JSON-RPC request and produce a response.
    async fn handle_request(&self, request: &serde_json::Value) -> serde_json::Value {
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = request.get("id");

        match method {
            "initialize" => self.handle_initialize(id),
            "shutdown" => self.handle_shutdown(id),
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tools_call(request, id).await,
            _ => self.make_error(id, -32601, format!("Method not found: {}", method)),
        }
    }

    /// Handle MCP initialize request.
    fn handle_initialize(&self, id: Option<&serde_json::Value>) -> serde_json::Value {
        let response = serde_json::json!({
            "protocolVersion": "0.1.0",
            "serverInfo": {
                "name": "coder",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": {
                "tools": {}
            }
        });
        self.make_response(id, response)
    }

    /// Handle tools/list request - return all registered tools as MCP tool definitions.
    fn handle_tools_list(&self, id: Option<&serde_json::Value>) -> serde_json::Value {
        let tool_defs = self.registry.tool_defs();

        let tools: Vec<serde_json::Value> = tool_defs
            .iter()
            .map(|def| {
                serde_json::json!({
                    "name": def.name,
                    "description": def.description,
                    "inputSchema": def.input_schema,
                })
            })
            .collect();

        self.make_response(id, serde_json::json!({ "tools": tools }))
    }

    /// Handle tools/call request - execute a tool.
    async fn handle_tools_call(&self, request: &serde_json::Value, id: Option<&serde_json::Value>) -> serde_json::Value {
        let params = request.get("params");
        let tool_name = params.and_then(|p| p.get("name")).and_then(|n| n.as_str()).unwrap_or("");
        let arguments = params.and_then(|p| p.get("arguments")).cloned().unwrap_or(serde_json::Value::Null);

        if tool_name.is_empty() {
            return self.make_error(id, -32602, "Tool name is required".to_string());
        }

        let result = self.registry.execute(tool_name, arguments).await;

        if result.success {
            self.make_response(id, serde_json::json!({
                "content": [
                    {
                        "type": "text",
                        "text": result.output
                    }
                ]
            }))
        } else {
            self.make_error(id, -32000, result.error.unwrap_or_else(|| "Tool execution failed".into()))
        }
    }

    fn handle_shutdown(&self, id: Option<&serde_json::Value>) -> serde_json::Value {
        self.running.store(false, Ordering::SeqCst);
        self.make_response(id, serde_json::json!(null))
    }

    /// Create a JSON-RPC success response.
    fn make_response(&self, id: Option<&serde_json::Value>, result: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        })
    }

    /// Create a JSON-RPC error response.
    fn make_error(&self, id: Option<&serde_json::Value>, code: i64, message: String) -> serde_json::Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        })
    }

    /// Check if the server is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Stop the server.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ToolRegistry;
    use std::sync::Arc;

    fn create_server() -> McpServer {
        let registry = Arc::new(ToolRegistry::default());
        McpServer::new(registry)
    }

    #[test]
    fn test_server_new() {
        let server = create_server();
        assert!(!server.is_running());
    }

    #[test]
    fn test_handle_initialize() {
        let server = create_server();
        let response = server.handle_initialize(Some(&serde_json::json!(1)));
        assert_eq!(response["id"], 1);
        assert!(response["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn test_handle_tools_list() {
        let server = create_server();
        let response = server.handle_tools_list(Some(&serde_json::json!(2)));
        assert_eq!(response["id"], 2);
        let tools = response["result"]["tools"].as_array().unwrap();
        assert!(!tools.is_empty());

        // Verify tool format
        let first = &tools[0];
        assert!(first.get("name").is_some());
        assert!(first.get("inputSchema").is_some());
    }

    #[test]
    fn test_handle_shutdown() {
        let server = create_server();
        let response = server.handle_shutdown(Some(&serde_json::json!(3)));
        assert_eq!(response["id"], 3);
        assert!(!server.is_running());
    }

    #[tokio::test]
    async fn test_handle_unknown_method() {
        let server = create_server();
        let response = server.handle_request(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "unknown"
        })).await;
        assert!(response.get("error").is_some());
        assert_eq!(response["error"]["code"], -32601);
    }

    #[test]
    fn test_make_error() {
        let server = create_server();
        let response = server.make_error(Some(&serde_json::json!(5)), -32700, "Parse error".into());
        assert!(response.get("error").is_some());
        assert_eq!(response["error"]["message"], "Parse error");
    }
}
