//! Agent engine - the core ReAct loop
//!
//! Manages the AI conversation loop:
//! 1. Receive user input
//! 2. Build request with context
//! 3. Stream AI response
//! 4. Handle tool calls
//! 5. Repeat until complete

pub mod r#loop;
pub mod context;
pub mod dispatch;
pub mod types;
pub mod auto_reasoning;
pub mod coordinator;

pub use r#loop::Agent;
pub use types::AgentType;
pub use types::InteractionMode;
pub use types::ReasoningEffort;
