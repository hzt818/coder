//! Bridge module - connects core modules for seamless functionality
//!
//! This module provides integration bridges between:
//! - Agent and Memory (context-aware memory retrieval)
//! - Session and Storage (persistent state)
//! - Tools and Skills (unified capability system)
//! - Team and Subagent (coordinated execution)
//!
//! These bridges ensure high continuity between features.

#[cfg(feature = "memory")]
pub mod agent_memory;
pub mod session_storage;
pub mod tool_skill;
pub mod team_subagent;

/// Bridge configuration for feature continuity
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub enable_memory_integration: bool,
    pub enable_session_persistence: bool,
    pub enable_tool_skill_bridge: bool,
    pub enable_team_coordination: bool,
    pub auto_compact_threshold: f64,
    pub memory_search_limit: usize,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            enable_memory_integration: true,
            enable_session_persistence: true,
            enable_tool_skill_bridge: true,
            enable_team_coordination: true,
            auto_compact_threshold: 0.8,
            memory_search_limit: 5,
        }
    }
}

/// Initialize all bridges based on configuration
pub fn init_bridges(config: &BridgeConfig) {
    if config.enable_memory_integration {
        tracing::info!("Initializing Agent-Memory bridge");
        #[cfg(feature = "memory")]
        agent_memory::init();
    }

    if config.enable_session_persistence {
        tracing::info!("Initializing Session-Storage bridge");
        session_storage::init();
    }

    if config.enable_tool_skill_bridge {
        tracing::info!("Initializing Tool-Skill bridge");
        tool_skill::init();
    }

    if config.enable_team_coordination {
        tracing::info!("Initializing Team-Subagent bridge");
        team_subagent::init();
    }

    tracing::info!("All bridges initialized with config: {:?}", config);
}
