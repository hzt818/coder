//! Subagent system - lightweight agent spawning for focused tasks
//!
//! Subagents are short-lived agents with isolated context, spawned
//! to accomplish a specific task and report results back to a supervisor.

pub mod spawn;
pub mod supervisor;

pub use spawn::{SubagentHandle, SpawnConfig, spawn_subagent};
pub use supervisor::{Supervisor, SubagentResult};
