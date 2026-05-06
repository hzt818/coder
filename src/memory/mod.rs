//! Memory system - cross-session memory persistence and retrieval
//!
//! Manages conversation memories stored as JSON files in ~/.coder/memory/,
//! with keyword-based retrieval and background consolidation via AutoDream.

pub mod autodream;
pub mod retrieve;
pub mod store;

pub use autodream::AutoDream;
pub use retrieve::MemoryRetrieval;
pub use store::MemoryStore;
