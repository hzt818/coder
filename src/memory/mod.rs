//! Memory system - cross-session memory persistence and retrieval
//!
//! Manages conversation memories stored as JSON files in ~/.coder/memory/,
//! with keyword-based retrieval and background consolidation via AutoDream.

pub mod store;
pub mod retrieve;
pub mod autodream;

pub use store::MemoryStore;
pub use retrieve::MemoryRetrieval;
pub use autodream::AutoDream;
