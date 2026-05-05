//! LSP integration - Language Server Protocol client
//!
//! Provides an LSP client that can connect to language servers via stdio,
//! enabling code completion, hover information, go-to-definition, and diagnostics.

pub mod client;
pub mod handler;

pub use client::{LspClient, LspServerConfig};
pub use handler::LspHandler;
