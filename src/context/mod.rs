//! Context storage abstraction for agent conversations
//!
//! Provides pluggable backends for storing message history:
//! - In-memory `Context` (default)
//! - SQLite-backed `SqliteContext` (requires `storage` feature)

pub mod store;

#[cfg(feature = "storage")]
pub mod sqlite;

pub use store::{Context, ContextStore};
#[cfg(feature = "storage")]
pub use sqlite::SqliteContext;
