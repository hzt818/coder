//! Skill system - reusable capabilities
//!
//! Skills are named, reusable capabilities that can be executed
//! with structured input and produce structured output. Builtin
//! skills include brainstorming, code review, planning, and debugging.

pub mod registry;
pub mod loader;
pub mod builtin;

pub use registry::SkillRegistry;
pub use loader::SkillLoader;
pub use builtin::Skill;

/// Result of executing a skill
pub type SkillResult = anyhow::Result<serde_json::Value>;
