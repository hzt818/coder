//! Skill system - reusable capabilities
//!
//! Skills are named, reusable capabilities that can be executed
//! with structured input and produce structured output. Builtin
//! skills include brainstorming, code review, planning, and debugging.

pub mod builtin;
pub mod loader;
pub mod registry;

pub use builtin::Skill;
pub use loader::SkillLoader;
pub use registry::SkillRegistry;

/// Result of executing a skill
pub type SkillResult = anyhow::Result<serde_json::Value>;
