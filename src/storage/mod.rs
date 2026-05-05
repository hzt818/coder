//! Storage layer - database abstraction and persistence
//!
//! Provides a unified Database trait with SQLite backend for
//! session persistence, memory storage, configuration, and telemetry.

pub mod db;
pub mod sqlite;
pub mod migrate;

pub use db::{Database, Memory, Event};
pub use sqlite::SqliteDb;
