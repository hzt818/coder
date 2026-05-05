//! MCP client - connects to external MCP servers
//!
//! Discovers tools from MCP servers and executes them.
//! Uses JSON-RPC over stdio or TCP transport.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

/// A tool exposed by a connected MCP server.
#[derive(Debug, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Configuration for connecting to an MCP server.
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

/// Connected MCP client instance.
pub struct McpClient {
    name: String,
    command: String,
    args: Vec<String>,
    process: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    stdout: Arc<Mutex<Option<BufReader<ChildStdout>>>>,
    tools: Arc<Mutex<Vec<McpTool>>>,
    next_id: Arc<AtomicU64>,
}

impl McpClient {
    /// Create a new MCP client configuration.
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            name: config.name,
            command: config.command,
            args: config.args,
            process: Arc::new(Mutex::new(None)),
            stdin: Arc::new(Mutex::new(None)),
            stdout: Arc::new(Mutex::new(None)),
            tools: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Connect to the MCP server and initialize the session.
    pub async fn connect(&self) -> anyhow::Result<()> {
        // In production, parse and use config.env
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to start MCP server '{}': {}", self.name, e))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdin for MCP server"))?;

        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdout for MCP server"))?;

        let mut process_lock = self.process.lock().await;
        *process_lock = Some(child);

        let mut stdin_lock = self.stdin.lock().await;
        *stdin_lock = Some(stdin);

        let mut stdout_lock = self.stdout.lock().await;
        *stdout_lock = Some(BufReader::new(stdout));

        // Initialize session
        self.send_initialize().await?;

        // Discover tools
        self.discover_tools().await?;

        Ok(())
    }

    /// Send MCP initialize request.
    async fn send_initialize(&self) -> anyhow::Result<()> {
        let params = serde_json::json!({
            "protocolVersion": "0.1.0",
            "clientInfo": {
                "name": "coder",
                "version": env!("CARGO_PKG_VERSION")
            }
        });
        self.send_request("initialize", params).await?;
        self.send_notification("notifications/initialized", serde_json::json!({})).await
    }

    /// Discover available tools from the server.
    async fn discover_tools(&self) -> anyhow::Result<()> {
        let response: serde_json::Value = self.send_request("tools/list", serde_json::json!({})).await?;

        let mut tools = self.tools.lock().await;
        if let Some(tool_list) = response.get("tools").and_then(|t| t.as_array()) {
            for tool_val in tool_list {
                tools.push(McpTool {
                    name: tool_val.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                    description: tool_val.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                    input_schema: tool_val.get("inputSchema").cloned().unwrap_or(serde_json::Value::Null),
                });
            }
        }

        Ok(())
    }

    /// Execute a tool on the MCP server.
    pub async fn execute_tool(&self, name: &str, args: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let params = serde_json::json!({
            "name": name,
            "arguments": args,
        });
        self.send_request("tools/call", params).await
    }

    /// Get the list of discovered tools.
    pub async fn list_tools(&self) -> Vec<McpTool> {
        self.tools.lock().await.clone()
    }

    /// Get the server name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Send a JSON-RPC request and read the response.
    async fn send_request(&self, method: &str, params: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.write_message(&request).await?;

        // Read the response from stdout
        self.read_response(id).await
    }

    /// Read a JSON-RPC response with Content-Length header parsing.
    async fn read_response(&self, _expected_id: u64) -> anyhow::Result<serde_json::Value> {
        let mut stdout_lock = self.stdout.lock().await;
        let reader = stdout_lock.as_mut()
            .ok_or_else(|| anyhow::anyhow!("MCP server stdout not available"))?;

        let mut line = String::new();
        let mut content_length: Option<usize> = None;

        // Read headers
        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await
                .map_err(|e| anyhow::anyhow!("Failed to read MCP response: {}", e))?;
            if bytes_read == 0 {
                anyhow::bail!("MCP server closed connection");
            }
            let trimmed = line.trim_end_matches("\r\n").trim_end_matches('\n');
            if trimmed.is_empty() {
                // End of headers
                break;
            }
            if let Some(len_str) = trimmed.strip_prefix("Content-Length:") {
                content_length = Some(
                    len_str.trim().parse()
                        .map_err(|e| anyhow::anyhow!("Invalid Content-Length: {}", e))?,
                );
            }
        }

        let len = content_length
            .ok_or_else(|| anyhow::anyhow!("Missing Content-Length header in MCP response"))?;

        // Read the JSON body
        let mut body = vec![0u8; len];
        use tokio::io::AsyncReadExt;
        reader.read_exact(&mut body).await
            .map_err(|e| anyhow::anyhow!("Failed to read MCP response body: {}", e))?;

        let response: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|e| anyhow::anyhow!("Failed to parse MCP response: {}", e))?;

        // Check for error response
        if let Some(error) = response.get("error") {
            let msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
            let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            anyhow::bail!("MCP error ({}): {}", code, msg);
        }

        // Return the result field, or the full response if no result
        Ok(response.get("result").cloned().unwrap_or(response))
    }

    /// Send a JSON-RPC notification (no response expected).
    async fn send_notification(&self, method: &str, params: serde_json::Value) -> anyhow::Result<()> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.write_message(&notification).await
    }

    /// Write a JSON-RPC message with Content-Length header.
    async fn write_message(&self, message: &serde_json::Value) -> anyhow::Result<()> {
        let content = serde_json::to_string(message)?;
        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        let mut stdin = self.stdin.lock().await;
        let stdin = stdin.as_mut()
            .ok_or_else(|| anyhow::anyhow!("MCP server not connected"))?;

        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(content.as_bytes()).await?;
        stdin.flush().await?;

        Ok(())
    }

    /// Disconnect from the MCP server.
    pub async fn disconnect(&self) -> anyhow::Result<()> {
        let mut process_lock = self.process.lock().await;
        if let Some(mut child) = process_lock.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_new() {
        let config = McpServerConfig {
            name: "test".into(),
            command: "echo".into(),
            args: vec![],
            env: HashMap::new(),
        };
        let client = McpClient::new(config);
        assert_eq!(client.name(), "test");
        assert!(client.list_tools().await.is_empty());
    }
}
