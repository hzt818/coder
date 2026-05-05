//! Command system - user-defined slash commands
//!
//! Supports custom commands with `$1`, `$2`, `$ARGUMENTS` template
//! substitution. Commands are loaded from `~/.coder/commands/` and
//! `.coder/commands/`.

pub mod user_commands;

pub use user_commands::{UserCommand, UserCommandManager, format_user_commands};
