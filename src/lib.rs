//! Coder - AI-powered development tool
//!
//! Integrates features from Claude Code (CC) and OpenCode (OP).
//! 🦀

pub mod ai;
pub mod agent;
pub mod config;
pub mod session;
pub mod tool;
pub mod tui;
pub mod util;
pub mod core;
pub mod execpolicy;
pub mod commands;
pub mod i18n;
pub mod sandbox;

// Phase 1
#[cfg(feature = "team")]
pub mod team;
#[cfg(feature = "skill")]
pub mod skill;
#[cfg(feature = "subagent")]
pub mod subagent;
#[cfg(feature = "memory")]
pub mod memory;
#[cfg(feature = "storage")]
pub mod storage;
#[cfg(feature = "lsp")]
pub mod lsp;
#[cfg(feature = "mcp")]
pub mod mcp;

// Phase 2
#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "permission")]
pub mod permission;
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "voice")]
pub mod voice;
#[cfg(feature = "oauth")]
pub mod oauth;
#[cfg(feature = "analytics")]
pub mod analytics;
#[cfg(feature = "computer")]
pub mod computer;
#[cfg(feature = "worktree")]
pub mod worktree;

