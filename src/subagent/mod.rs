//! Subagent system - lightweight agent spawning for focused tasks
//!
//! Subagents are short-lived agents with isolated context, spawned
//! to accomplish a specific task and report results back to a supervisor.

pub mod roles;
pub mod spawn;
pub mod supervisor;

pub use roles::{parse_role, SubAgentRole, ALL_ROLES};
pub use spawn::{spawn_subagent, SpawnConfig, SubagentHandle};
pub use supervisor::{SubagentResult, Supervisor};
