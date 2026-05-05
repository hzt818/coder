//! Team management module
//!
//! Coordinates multiple agents working together on tasks.
//! Teammates communicate via message passing over tokio channels.

pub mod manager;
pub mod teammate;
pub mod communication;
pub mod task;

pub use manager::TeamManager;
pub use teammate::{Teammate, TeammateRole, TeammateStatus};
pub use communication::{TeammateMessage, MessageContent};
pub use task::{TaskAssignment, TaskStatus, TaskId};
