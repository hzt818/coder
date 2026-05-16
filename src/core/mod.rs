//! Core runtime modules
//!
//! Contains shared runtime components: pricing, compaction, checkpoint,
//! audit logging, capacity control, hook dispatcher, and LSP hooks.

pub mod audit;
pub mod automation;
pub mod bridge;
pub mod capacity;
pub mod checkpoint;
pub mod compaction;
pub mod features;
pub mod hooks;
#[cfg(feature = "lsp")]
pub mod lsp_hooks;
pub mod pricing;
pub mod snapshot;
pub mod task_manager;
