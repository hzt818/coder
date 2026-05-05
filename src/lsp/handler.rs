//! LSP handler - processes LSP requests and manages multiple servers
//!
//! Provides a high-level API for interacting with LSP servers,
//! managing multiple server connections per language.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::client::{LspClient, LspServerConfig, ServerCapabilities};

/// High-level handler for LSP operations across multiple languages.
pub struct LspHandler {
    servers: Arc<RwLock<HashMap<String, Arc<LspClient>>>>,
    /// Default root URI for workspace
    root_uri: Option<String>,
}

impl LspHandler {
    /// Create a new LSP handler.
    pub fn new(root_uri: Option<String>) -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            root_uri,
        }
    }

    /// Start an LSP server for the given language.
    ///
    /// If a server for this language is already running, it is returned instead.
    pub async fn start_server(&self, language: &str, command: &str, args: Vec<String>) -> anyhow::Result<Arc<LspClient>> {
        let servers = self.servers.read().await;
        if let Some(existing) = servers.get(language) {
            return Ok(Arc::clone(existing));
        }
        drop(servers);

        let config = LspServerConfig {
            command: command.to_string(),
            args,
            language_id: language.to_string(),
            root_uri: self.root_uri.clone(),
        };

        let client = Arc::new(LspClient::new(config));
        client.start().await?;

        let mut servers = self.servers.write().await;
        servers.insert(language.to_string(), Arc::clone(&client));
        Ok(client)
    }

    /// Get the LSP client for a given language, if connected.
    pub async fn get_client(&self, language: &str) -> Option<Arc<LspClient>> {
        let servers = self.servers.read().await;
        servers.get(language).map(Arc::clone)
    }

    /// Get code completions for a file at a given position.
    pub async fn complete(
        &self,
        language: &str,
        uri: &str,
        line: u64,
        character: u64,
    ) -> anyhow::Result<serde_json::Value> {
        let client = self.get_client(language).await
            .ok_or_else(|| anyhow::anyhow!("No LSP server running for language: {}", language))?;
        client.get_completion(uri, line, character).await
    }

    /// Get hover information for a symbol at a position.
    pub async fn hover(
        &self,
        language: &str,
        uri: &str,
        line: u64,
        character: u64,
    ) -> anyhow::Result<serde_json::Value> {
        let client = self.get_client(language).await
            .ok_or_else(|| anyhow::anyhow!("No LSP server running for language: {}", language))?;
        client.get_hover(uri, line, character).await
    }

    /// Get the definition location for a symbol at a position.
    pub async fn goto_definition(
        &self,
        language: &str,
        uri: &str,
        line: u64,
        character: u64,
    ) -> anyhow::Result<serde_json::Value> {
        let client = self.get_client(language).await
            .ok_or_else(|| anyhow::anyhow!("No LSP server running for language: {}", language))?;
        client.get_definition(uri, line, character).await
    }

    /// Get capabilities for a language server.
    pub async fn capabilities(&self, language: &str) -> anyhow::Result<ServerCapabilities> {
        let client = self.get_client(language).await
            .ok_or_else(|| anyhow::anyhow!("No LSP server running for language: {}", language))?;
        Ok(client.capabilities().await)
    }

    /// Shutdown a specific language server.
    pub async fn shutdown_server(&self, language: &str) -> anyhow::Result<()> {
        let mut servers = self.servers.write().await;
        if let Some(client) = servers.remove(language) {
            client.shutdown().await?;
        }
        Ok(())
    }

    /// Shutdown all running LSP servers.
    pub async fn shutdown_all(&self) -> anyhow::Result<()> {
        let mut servers = self.servers.write().await;
        for (_, client) in servers.drain() {
            if let Err(e) = client.shutdown().await {
                tracing::warn!("Error shutting down LSP server: {}", e);
            }
        }
        Ok(())
    }

    /// List all connected languages.
    pub async fn connected_languages(&self) -> Vec<String> {
        let servers = self.servers.read().await;
        servers.keys().cloned().collect()
    }
}

impl Drop for LspHandler {
    fn drop(&mut self) {
        // Servers will be dropped and shutdown in their Drop impls
    }
}

/// Built-in language-to-server mappings.
pub fn default_lsp_configs() -> HashMap<&'static str, (&'static str, Vec<&'static str>)> {
    let mut map = HashMap::new();
    map.insert("rust", ("rust-analyzer", vec![]));
    map.insert("typescript", ("typescript-language-server", vec!["--stdio"]));
    map.insert("javascript", ("typescript-language-server", vec!["--stdio"]));
    map.insert("python", ("pyright-langserver", vec!["--stdio"]));
    map.insert("go", ("gopls", vec![]));
    map.insert("json", ("vscode-json-languageserver", vec!["--stdio"]));
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handler_new() {
        let handler = LspHandler::new(None);
        assert!(handler.connected_languages().await.is_empty());
    }

    #[test]
    fn test_default_configs() {
        let configs = default_lsp_configs();
        assert!(configs.contains_key("rust"));
        assert!(configs.contains_key("typescript"));
        assert!(configs.contains_key("python"));
        assert_eq!(configs.get("rust").unwrap().0, "rust-analyzer");
    }

    #[tokio::test]
    async fn test_get_client_nonexistent() {
        let handler = LspHandler::new(None);
        let client = handler.get_client("rust").await;
        assert!(client.is_none());
    }

    #[tokio::test]
    async fn test_shutdown_all_empty() {
        let handler = LspHandler::new(None);
        // Should not error when no servers are running
        handler.shutdown_all().await.unwrap();
    }
}
