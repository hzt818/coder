//! LSP integration - Language Server Protocol client
//!
//! Provides an LSP client that can connect to language servers via stdio,
//! enabling code completion, hover information, go-to-definition, and diagnostics.

pub mod client;
pub mod handler;

pub use client::{LspClient, LspServerConfig};
pub use handler::LspHandler;

/// A single diagnostic item from an LSP server.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Diagnostic {
    pub severity: i64,
    pub message: String,
    #[serde(rename = "range")]
    pub range: DiagnosticRange,
    pub source: Option<String>,
}

impl Diagnostic {
    /// Extract the line number (0-based) from the diagnostic range.
    pub fn line(&self) -> usize {
        self.range.start.line as usize
    }

    /// Extract the column number (0-based) from the diagnostic range.
    pub fn column(&self) -> usize {
        self.range.start.character as usize
    }
}

/// Range within a document, as defined by the LSP protocol.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DiagnosticRange {
    pub start: DiagnosticPosition,
    pub end: DiagnosticPosition,
}

/// A position in a document (0-based line/character).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DiagnosticPosition {
    pub line: i64,
    pub character: i64,
}
