//! MCP (Model Context Protocol) support
//!
//! Provides:
//! - MCP client for connecting to external MCP servers and discovering tools
//! - MCP server for exposing coder's tools to other MCP clients
//! - Context7 integration for documentation lookup

pub mod client;
pub mod context7;
pub mod server;

pub use client::McpClient;
pub use context7::Context7Client;
pub use server::McpServer;
