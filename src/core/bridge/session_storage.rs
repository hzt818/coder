//! Session-Storage Bridge - connects Session with persistent Storage
//!
//! Provides:
//! - Automatic session persistence
//! - Session metadata indexing
//! - Cross-session state recovery

use crate::session::Session;
use std::sync::RwLock;
use std::time::Instant;
use std::sync::OnceLock;

static SESSION_STORAGE_BRIDGE: OnceLock<RwLock<SessionStorageBridgeState>> = OnceLock::new();

pub struct SessionStorageBridgeState {
    auto_save_enabled: bool,
    auto_save_interval_secs: u64,
    last_save: Instant,
}

impl SessionStorageBridgeState {
    pub fn new() -> Self {
        Self {
            auto_save_enabled: true,
            auto_save_interval_secs: 60,
            last_save: Instant::now(),
        }
    }
}

fn get_state() -> &'static RwLock<SessionStorageBridgeState> {
    SESSION_STORAGE_BRIDGE.get_or_init(|| RwLock::new(SessionStorageBridgeState::new()))
}

pub fn init() {
    let _ = get_state();
    tracing::info!("Session-Storage bridge initialized");
}

pub fn enable_auto_save(enabled: bool) {
    if let Ok(mut state) = get_state().write() {
        state.auto_save_enabled = enabled;
        tracing::info!("Auto-save {}", if enabled { "enabled" } else { "disabled" });
    }
}

pub fn set_auto_save_interval(secs: u64) {
    if let Ok(mut state) = get_state().write() {
        state.auto_save_interval_secs = secs;
        tracing::info!("Auto-save interval set to {} seconds", secs);
    }
}

pub fn should_auto_save() -> bool {
    if let Ok(state) = get_state().read() {
        if !state.auto_save_enabled {
            return false;
        }
        let elapsed = state.last_save.elapsed().as_secs();
        return elapsed >= state.auto_save_interval_secs;
    }
    false
}

pub fn mark_saved() {
    if let Ok(mut state) = get_state().write() {
        state.last_save = Instant::now();
    }
}

pub fn save_session(session: &Session) -> anyhow::Result<()> {
    let manager = crate::session::manager::SessionManager::new();
    manager.save(session)?;
    mark_saved();
    tracing::debug!("Session {} saved", session.id);
    Ok(())
}

pub fn load_session(id: &str) -> anyhow::Result<Option<Session>> {
    let manager = crate::session::manager::SessionManager::new();
    manager.load(id)
}

pub fn list_sessions() -> anyhow::Result<Vec<crate::session::SessionSummary>> {
    let manager = crate::session::manager::SessionManager::new();
    manager.list()
}

pub fn delete_session(id: &str) -> anyhow::Result<()> {
    let manager = crate::session::manager::SessionManager::new();
    manager.delete(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        init();
        // Auto-save interval defaults to 60s; set to 0 for immediate trigger
        set_auto_save_interval(0);
        assert!(should_auto_save());
    }

    #[test]
    fn test_auto_save_toggle() {
        init();
        set_auto_save_interval(0);
        enable_auto_save(false);
        assert!(!should_auto_save());
        enable_auto_save(true);
        assert!(should_auto_save());
    }

    #[test]
    fn test_auto_save_interval() {
        init();
        set_auto_save_interval(120);
        assert!(!should_auto_save());
    }
}
