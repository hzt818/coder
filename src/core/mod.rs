//! Core runtime modules
//!
//! Contains shared runtime components: pricing, compaction, checkpoint,
//! audit logging, capacity control, hook dispatcher, and LSP hooks.

pub mod pricing;
pub mod compaction;
pub mod checkpoint;
pub mod audit;
pub mod capacity;
pub mod snapshot;
pub mod task_manager;
pub mod automation;
pub mod hooks;
#[cfg(feature = "lsp")]
pub mod lsp_hooks;
