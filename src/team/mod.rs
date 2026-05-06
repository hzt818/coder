//! Team management module
//!
//! Coordinates multiple agents working together on tasks.
//! Teammates communicate via message passing over tokio channels.

pub mod communication;
pub mod manager;
pub mod task;
pub mod teammate;

pub use communication::{MessageContent, TeammateMessage};
pub use manager::TeamManager;
pub use task::{TaskAssignment, TaskId, TaskStatus};
pub use teammate::{Teammate, TeammateRole, TeammateStatus};
