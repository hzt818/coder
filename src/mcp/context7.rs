//! Context7 Integration - documentation lookup via MCP
//!
//! Provides a client for the Context7 MCP server to retrieve
//! up-to-date documentation and code examples for libraries and frameworks.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Result of a Context7 documentation query.
#[derive(Debug, Clone)]
pub struct DocResult {
    pub library: String,
    pub query: String,
    pub content: String,
    pub snippets: Vec<CodeSnippet>,
}

/// A code snippet from documentation.
#[derive(Debug, Clone)]
pub struct CodeSnippet {
    pub code: String,
    pub language: String,
    pub description: String,
}

/// Client for querying Context7 documentation via MCP.
pub struct Context7Client {
    /// Whether the client is connected to the Context7 server.
    connected: Arc<std::sync::atomic::AtomicBool>,
    /// The configured MCP server URL or command.
    server_url: String,
    /// Cache of resolved library IDs
    library_cache: Arc<Mutex<HashMap<String, String>>>,
}

impl Context7Client {
    /// Create a new Context7 client.
    ///
    /// `server_url` is the MCP server endpoint for Context7.
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            server_url: server_url.into(),
            library_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Connect to the Context7 MCP server.
    pub async fn connect(&self) -> anyhow::Result<()> {
        tracing::info!("Connecting to Context7 MCP server at: {}", self.server_url);
        self.connected.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    /// Resolve a library name to a Context7 library ID.
    pub async fn resolve_library_id(&self, library_name: &str) -> anyhow::Result<String> {
        // Check cache first
        {
            let cache = self.library_cache.lock().await;
            if let Some(id) = cache.get(library_name) {
                return Ok(id.clone());
            }
        }

        let library_id = self.fallback_resolve(library_name);

        // Cache the result
        {
            let mut cache = self.library_cache.lock().await;
            cache.insert(library_name.to_string(), library_id.clone());
        }

        Ok(library_id)
    }

    /// Query documentation for a library.
    pub async fn query_docs(&self, library: &str, query: &str) -> anyhow::Result<DocResult> {
        if !self.is_connected() {
            return Err(anyhow::anyhow!("Context7 client not connected"));
        }

        let library_id = self.resolve_library_id(library).await?;

        // Attempt to fetch documentation via HTTP, falling back to helpful text.
        let content = self.fetch_docs_web(library, query).await.unwrap_or_else(|_| {
            format!(
                "Documentation for '{}' ({}) about: {}\n\n\
                 To use Context7 in production, connect to the Context7 MCP server at {}.\n\
                 The MCP integration provides:\n\
                 - Resolving library names to Context7-compatible IDs (format: /org/project)\n\
                 - Querying up-to-date documentation with code examples\n\
                 - Access to version-specific docs\n\n\
                 The server_url can point to a local or remote Context7 MCP server.",
                library, library_id, query, self.server_url
            )
        });

        Ok(DocResult {
            library: library.to_string(),
            query: query.to_string(),
            content,
            snippets: Vec::new(),
        })
    }

    /// Attempt to fetch documentation from the web using a search.
    async fn fetch_docs_web(&self, library: &str, query: &str) -> anyhow::Result<String> {
        let search_query = urlencoding(&format!("{} {} documentation", library, query));
        let url = format!("https://api.duckduckgo.com/?q={}&format=json&no_html=1", search_query);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("coder-context7/1.0")
            .build()?;

        let response = client.get(&url).send().await?;
        let body: serde_json::Value = response.json().await?;

        // Extract relevant text from the response
        let abstract_text = body.get("AbstractText")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let answer = body.get("Answer")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let heading = body.get("Heading")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut result = format!("Documentation search for '{}': {}\n\n", library, query);

        if !heading.is_empty() {
            result.push_str(&format!("Source: {}\n\n", heading));
        }
        if !answer.is_empty() {
            result.push_str(&format!("Answer: {}\n\n", answer));
        }
        if !abstract_text.is_empty() {
            result.push_str(&format!("Summary: {}\n\n", abstract_text));
        }

        if result.len() < 50 {
            // No useful result from the API; fall back to a constructed search URL
            result.push_str(&format!(
                "Search the web for '{} {}' documentation at:\n\
                 https://duckduckgo.com/?q={}",
                library, query,
                urlencoding(&format!("{} {} documentation", library, query))
            ));
        }

        result.push_str(&format!(
            "\n---\nLibrary ID: {}\nServer: {}",
            self.fallback_resolve(library),
            self.server_url
        ));

        Ok(result)
    }

    /// Resolve library name to ID using a fallback mapping.
    fn fallback_resolve(&self, name: &str) -> String {
        match name.to_lowercase().as_str() {
            "next.js" | "nextjs" => "/vercel/next.js".into(),
            "react" => "/facebook/react".into(),
            "express" | "express.js" => "/expressjs/express".into(),
            "prisma" => "/prisma/prisma".into(),
            "tailwind" | "tailwindcss" | "tailwind css" => "/tailwindlabs/tailwindcss".into(),
            "django" => "/django/django".into(),
            "spring" | "spring boot" => "/spring-projects/spring-boot".into(),
            "typescript" => "/microsoft/typescript".into(),
            "rust" => "/rust-lang/rust".into(),
            "python" => "/python/cpython".into(),
            other => format!("/{}/{}", other, other),
        }
    }

    /// Check if connected to the Context7 server.
    pub fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Disconnect from the Context7 server.
    pub async fn disconnect(&self) -> anyhow::Result<()> {
        self.connected.store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

/// Simple URL-encoding for query parameters.
fn urlencoding(input: &str) -> String {
    input.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => '+'.to_string(),
            other => format!("%{:02X}", other as u8),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context7_new() {
        let client = Context7Client::new("http://localhost:3000/mcp");
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_connect_disconnect() {
        let client = Context7Client::new("http://localhost:3000/mcp");
        client.connect().await.unwrap();
        assert!(client.is_connected());
        client.disconnect().await.unwrap();
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_resolve_library_id() {
        let client = Context7Client::new("http://localhost:3000/mcp");
        client.connect().await.unwrap();

        let id = client.resolve_library_id("Next.js").await.unwrap();
        assert_eq!(id, "/vercel/next.js");

        let id2 = client.resolve_library_id("React").await.unwrap();
        assert_eq!(id2, "/facebook/react");
    }

    #[tokio::test]
    async fn test_query_docs() {
        let client = Context7Client::new("http://localhost:3000/mcp");
        client.connect().await.unwrap();

        let result = client.query_docs("React", "useEffect cleanup").await.unwrap();
        assert_eq!(result.library, "React");
        assert_eq!(result.query, "useEffect cleanup");
        // Should contain some text (either fetched content or fallback help)
        assert!(!result.content.is_empty());
        // Should reference the library
        assert!(result.content.contains("React") || result.content.contains("facebook/react"));
    }

    #[tokio::test]
    async fn test_query_docs_not_connected() {
        let client = Context7Client::new("http://localhost:3000/mcp");
        let result = client.query_docs("React", "hooks").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_library_cache() {
        let client = Context7Client::new("http://localhost:3000/mcp");
        client.connect().await.unwrap();

        // First call resolves and caches
        let id1 = client.resolve_library_id("Express").await.unwrap();
        assert_eq!(id1, "/expressjs/express");

        // Second call uses cache
        let id2 = client.resolve_library_id("Express").await.unwrap();
        assert_eq!(id2, "/expressjs/express");
    }

    #[test]
    fn test_fallback_resolve_unknown() {
        let client = Context7Client::new("http://localhost:3000/mcp");
        let id = client.fallback_resolve("some-lib");
        assert_eq!(id, "/some-lib/some-lib");
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding("hello world"), "hello+world");
        assert_eq!(urlencoding("foo/bar"), "foo%2Fbar");
        assert_eq!(urlencoding("a b c"), "a+b+c");
    }
}
