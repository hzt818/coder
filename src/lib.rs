//! Coder - AI-powered development tool
//!
//! Integrates features from Claude Code (CC) and OpenCode (OP).
//! 🦀

pub mod adapters;
pub mod agent;
pub mod ai;
pub mod commands;
pub mod config;
pub mod core;
pub mod execpolicy;
pub mod i18n;
pub mod sandbox;
pub mod session;
pub mod tool;
pub mod tui;
pub mod util;

// Phase 1
#[cfg(feature = "lsp")]
pub mod lsp;
#[cfg(feature = "mcp")]
pub mod mcp;
#[cfg(feature = "memory")]
pub mod memory;
#[cfg(feature = "skill")]
pub mod skill;
#[cfg(feature = "storage")]
pub mod storage;
#[cfg(feature = "subagent")]
pub mod subagent;
#[cfg(feature = "team")]
pub mod team;

// Phase 2
#[cfg(feature = "analytics")]
pub mod analytics;
#[cfg(feature = "computer")]
pub mod computer;
#[cfg(feature = "oauth")]
pub mod oauth;
#[cfg(feature = "permission")]
pub mod permission;
#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "voice")]
pub mod voice;
#[cfg(feature = "worktree")]
pub mod worktree;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Global shutdown flag — set by OS signal handlers (SIGTERM, SIGINT) to
/// request a clean shutdown. The TUI loop polls this flag rather than letting
/// the signal handler call process::exit() directly (which could race with
/// terminal rendering).
pub static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Notifier for graceful shutdown — signal handlers notify this;
/// the TUI event loop awaits it to break out of rendering.
pub fn shutdown_notifier() -> Arc<tokio::sync::Notify> {
    static NOTIFIER: std::sync::OnceLock<Arc<tokio::sync::Notify>> = std::sync::OnceLock::new();
    NOTIFIER
        .get_or_init(|| Arc::new(tokio::sync::Notify::new()))
        .clone()
}
