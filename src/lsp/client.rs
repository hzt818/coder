//! LSP client - connects to language servers via stdio JSON-RPC
//!
//! Manages the lifecycle of LSP server processes and provides
//! methods for sending requests and handling notifications.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{Mutex, RwLock};

/// Configuration for connecting to an LSP server.
#[derive(Debug, Clone)]
pub struct LspServerConfig {
    /// The language server command (e.g., "rust-analyzer", "typescript-language-server")
    pub command: String,
    /// Arguments to pass to the server
    pub args: Vec<String>,
    /// Language ID (e.g., "rust", "typescript", "python")
    pub language_id: String,
    /// Root URI for the workspace
    pub root_uri: Option<String>,
}

/// Represents the capabilities of a connected LSP server.
#[derive(Debug, Clone, Default)]
pub struct ServerCapabilities {
    pub supports_completion: bool,
    pub supports_hover: bool,
    pub supports_definition: bool,
    pub supports_references: bool,
    pub supports_diagnostics: bool,
    pub supports_formatting: bool,
}

/// A client connected to an LSP server via stdio.
pub struct LspClient {
    process: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    reader: Arc<Mutex<Option<BufReader<ChildStdout>>>>,
    capabilities: Arc<RwLock<ServerCapabilities>>,
    server_config: LspServerConfig,
    next_id: Arc<AtomicU64>,
    /// Whether the client is currently initialized
    initialized: Arc<AtomicBool>,
}

impl LspClient {
    /// Create a new LSP client configuration (not yet connected).
    pub fn new(config: LspServerConfig) -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            stdin: Arc::new(Mutex::new(None)),
            reader: Arc::new(Mutex::new(None)),
            capabilities: Arc::new(RwLock::new(ServerCapabilities::default())),
            server_config: config,
            next_id: Arc::new(AtomicU64::new(1)),
            initialized: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start the LSP server and initialize the connection.
    pub async fn start(&self) -> anyhow::Result<()> {
        let mut child = Command::new(&self.server_config.command)
            .args(&self.server_config.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to start LSP server '{}': {}", self.server_config.command, e))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdin for LSP server"))?;

        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdout for LSP server"))?;

        let mut process_lock = self.process.lock().await;
        *process_lock = Some(child);

        let mut stdin_lock = self.stdin.lock().await;
        *stdin_lock = Some(stdin);

        let mut reader_lock = self.reader.lock().await;
        *reader_lock = Some(BufReader::new(stdout));

        // Send initialize request
        self.send_initialize().await?;
        self.initialized.store(true, Ordering::SeqCst);

        tracing::info!("LSP server '{}' initialized", self.server_config.command);
        Ok(())
    }

    /// Send the initialize request to the LSP server.
    async fn send_initialize(&self) -> anyhow::Result<()> {
        let params = serde_json::json!({
            "processId": std::process::id(),
            "clientInfo": {
                "name": "coder",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true
                        }
                    },
                    "hover": {
                        "contentFormat": ["markdown", "plaintext"]
                    }
                }
            },
            "rootUri": self.server_config.root_uri,
            "workspaceFolders": null
        });

        let result: serde_json::Value = self.send_request("initialize", params).await?;

        // Parse capabilities from response
        if let Some(caps) = result.get("capabilities") {
            let mut capabilities = self.capabilities.write().await;
            capabilities.supports_completion = caps.get("completionProvider").is_some();
            capabilities.supports_hover = caps.get("hoverProvider").is_some();
            capabilities.supports_definition = caps.get("definitionProvider").is_some();
            capabilities.supports_references = caps.get("referencesProvider").is_some();
            capabilities.supports_diagnostics = caps.get("textDocumentSync").is_some();
            capabilities.supports_formatting = caps.get("documentFormattingProvider").is_some();
        }

        // Send initialized notification
        self.send_notification("initialized", serde_json::json!({})).await?;

        Ok(())
    }

    /// Send a JSON-RPC request and wait for the response.
    pub async fn send_request(&self, method: &str, params: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        self.write_message(&request).await?;
        self.read_response(id).await
    }

    /// Send a JSON-RPC notification (no response expected).
    pub async fn send_notification(&self, method: &str, params: serde_json::Value) -> anyhow::Result<()> {
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
            .ok_or_else(|| anyhow::anyhow!("LSP server not connected"))?;

        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(content.as_bytes()).await?;
        stdin.flush().await?;

        Ok(())
    }

    /// Read a JSON-RPC response with the given ID.
    async fn read_response(&self, _expected_id: u64) -> anyhow::Result<serde_json::Value> {
        let mut reader_lock = self.reader.lock().await;
        let reader = reader_lock.as_mut()
            .ok_or_else(|| anyhow::anyhow!("LSP server not connected"))?;

        let mut line = String::new();
        let mut content_length: Option<usize> = None;

        // Read headers
        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await
                .map_err(|e| anyhow::anyhow!("Failed to read LSP response: {}", e))?;
            if bytes_read == 0 {
                anyhow::bail!("LSP server closed connection");
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
            .ok_or_else(|| anyhow::anyhow!("Missing Content-Length header in LSP response"))?;

        // Read the JSON body
        let mut body = vec![0u8; len];
        reader.read_exact(&mut body).await
            .map_err(|e| anyhow::anyhow!("Failed to read LSP response body: {}", e))?;

        let response: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|e| anyhow::anyhow!("Failed to parse LSP response: {}", e))?;

        // Check for error response
        if let Some(error) = response.get("error") {
            let msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
            let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            anyhow::bail!("LSP error ({}): {}", code, msg);
        }

        // Return the result field
        response.get("result")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("LSP response missing 'result' field"))
    }

    /// Request code completion at a given position.
    pub async fn get_completion(
        &self,
        uri: &str,
        line: u64,
        character: u64,
    ) -> anyhow::Result<serde_json::Value> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });
        self.send_request("textDocument/completion", params).await
    }

    /// Request hover information at a given position.
    pub async fn get_hover(
        &self,
        uri: &str,
        line: u64,
        character: u64,
    ) -> anyhow::Result<serde_json::Value> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });
        self.send_request("textDocument/hover", params).await
    }

    /// Request go-to-definition at a given position.
    pub async fn get_definition(
        &self,
        uri: &str,
        line: u64,
        character: u64,
    ) -> anyhow::Result<serde_json::Value> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });
        self.send_request("textDocument/definition", params).await
    }

    /// Get current server capabilities.
    pub async fn capabilities(&self) -> ServerCapabilities {
        self.capabilities.read().await.clone()
    }

    /// Check if the client is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    /// Shutdown the LSP server gracefully.
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        if !self.is_initialized() {
            return Ok(());
        }

        // Send shutdown request (ignore result)
        let _ = self.send_request("shutdown", serde_json::json!({})).await;
        let _ = self.send_notification("exit", serde_json::json!({})).await;

        self.initialized.store(false, Ordering::SeqCst);

        let mut process_lock = self.process.lock().await;
        if let Some(mut child) = process_lock.take() {
            let _ = child.wait().await;
        }

        Ok(())
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        // Best-effort cleanup in non-async context
        self.initialized.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_client_new() {
        let config = LspServerConfig {
            command: "rust-analyzer".into(),
            args: vec![],
            language_id: "rust".into(),
            root_uri: None,
        };
        let client = LspClient::new(config);
        assert!(!client.is_initialized());
    }

    #[test]
    fn test_server_config_defaults() {
        let config = LspServerConfig {
            command: "typescript-language-server".into(),
            args: vec!["--stdio".into()],
            language_id: "typescript".into(),
            root_uri: Some("file:///project".into()),
        };
        assert_eq!(config.language_id, "typescript");
        assert_eq!(config.command, "typescript-language-server");
    }
}
