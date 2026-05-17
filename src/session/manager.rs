//! Session manager - handles session persistence

use super::{Session, SessionSummary};
use crate::util::path::{ensure_dir, sessions_dir};

/// Manages session persistence to disk
pub struct SessionManager {
    sessions_dir: std::path::PathBuf,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        let dir = sessions_dir();
        let _ = ensure_dir(&dir);
        Self { sessions_dir: dir }
    }

    /// Save a session to disk
    pub fn save(&self, session: &Session) -> anyhow::Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", session.id));
        let content = serde_json::to_string_pretty(session)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Start a periodic auto-save task for a session.
    ///
    /// Spawns a tokio task that saves `session` every `interval_secs` seconds.
    /// Returns a `oneshot::Sender` that can be used to stop the task.
    pub fn start_auto_save(
        &self,
        session: std::sync::Arc<std::sync::Mutex<crate::session::Session>>,
        interval_secs: u64,
    ) -> tokio::sync::oneshot::Sender<()> {
        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
        let save_dir = self.sessions_dir.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
            // Skip the immediate first tick (avoid saving before any message)
            interval.tick().await;
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let session = match session.lock() {
                            Ok(s) => s.clone(),
                            Err(_) => continue,
                        };
                        let path = save_dir.join(format!("{}.json", session.id));
                        if let Ok(content) = serde_json::to_string_pretty(&session) {
                            let _ = std::fs::write(&path, content);
                        }
                    }
                    _ = &mut stop_rx => break,
                }
            }
        });

        stop_tx
    }

    /// Load a session from disk
    pub fn load(&self, id: &str) -> anyhow::Result<Option<Session>> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        let session: Session = serde_json::from_str(&content)?;
        Ok(Some(session))
    }

    /// List all saved sessions
    pub fn list(&self) -> anyhow::Result<Vec<SessionSummary>> {
        let mut sessions = Vec::new();

        if !self.sessions_dir.exists() {
            return Ok(sessions);
        }

        for entry in std::fs::read_dir(&self.sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }

            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(session) = serde_json::from_str::<Session>(&content) {
                    let msg_count = session.message_count();
                    sessions.push(SessionSummary {
                        id: session.id,
                        title: session.title,
                        created_at: session.created_at,
                        updated_at: session.updated_at,
                        message_count: msg_count,
                    });
                }
            }
        }

        // Sort by updated_at, newest first
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(sessions)
    }

    /// Delete a session
    pub fn delete(&self, id: &str) -> anyhow::Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_manager_new() {
        let manager = SessionManager::new();
        assert!(manager.sessions_dir.ends_with("sessions"));
    }

    #[test]
    fn test_session_save_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = SessionManager {
            sessions_dir: tmp.path().to_path_buf(),
        };

        let session = Session::new();
        manager.save(&session).unwrap();

        let loaded = manager.load(&session.id).unwrap().unwrap();
        assert_eq!(loaded.id, session.id);
    }

    #[test]
    fn test_session_list() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = SessionManager {
            sessions_dir: tmp.path().to_path_buf(),
        };

        let session = Session::new();
        manager.save(&session).unwrap();

        let list = manager.list().unwrap();
        assert!(!list.is_empty());
        assert_eq!(list[0].id, session.id);
    }
}
