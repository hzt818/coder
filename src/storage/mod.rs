//! Storage layer - database abstraction and persistence
//!
//! Provides a unified Database trait with SQLite backend for
//! session persistence, memory storage, configuration, and telemetry.

pub mod db;
pub mod migrate;
pub mod sqlite;

pub use db::{Database, Event, Memory};
pub use sqlite::SqliteDb;
