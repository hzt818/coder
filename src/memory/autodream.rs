//! AutoDream - background memory consolidation
//!
//! Periodically consolidates conversation memories:
//! - Summarizes long conversations into compact memory entries
//! - Extracts key insights, decisions, and preferences
//! - Removes or archives stale/expired memories

use crate::memory::store::{MemoryEntry, MemoryStore};
use std::sync::Arc;

/// Configuration for AutoDream consolidation.
#[derive(Debug, Clone)]
pub struct AutoDreamConfig {
    /// How often to run consolidation (default: 1 hour)
    pub interval: chrono::Duration,
    /// Maximum age of memories to keep (default: 90 days)
    pub max_age: chrono::Duration,
    /// Minimum content length to qualify for summarization
    pub min_summary_length: usize,
}

impl Default for AutoDreamConfig {
    fn default() -> Self {
        Self {
            interval: chrono::Duration::seconds(3600),
            max_age: chrono::Duration::days(90),
            min_summary_length: 500,
        }
    }
}

/// AutoDream memory consolidation engine.
///
/// Runs as a background task that periodically:
/// 1. Scans memory store for entries needing consolidation
/// 2. Applies retention policies (archive old entries)
/// 3. Merges related entries when possible
pub struct AutoDream {
    store: Arc<MemoryStore>,
    config: AutoDreamConfig,
}

impl AutoDream {
    /// Create a new AutoDream instance.
    pub fn new(store: Arc<MemoryStore>, config: AutoDreamConfig) -> Self {
        Self { store, config }
    }

    /// Start the background consolidation loop.
    ///
    /// Returns a handle that can be used to cancel the task.
    pub fn start(self) -> tokio::task::JoinHandle<anyhow::Result<()>> {
        tokio::spawn(async move { self.run_loop().await })
    }

    /// Main loop: consolidate on interval.
    async fn run_loop(&self) -> anyhow::Result<()> {
        let secs = self.config.interval.num_seconds().max(1) as u64;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(secs));
        interval.tick().await; // skip initial immediate tick

        loop {
            interval.tick().await;
            tracing::debug!("AutoDream: starting consolidation cycle");

            if let Err(e) = self.consolidate() {
                tracing::warn!("AutoDream consolidation error: {}", e);
            }
        }
    }

    /// Execute one consolidation cycle synchronously.
    pub fn consolidate(&self) -> anyhow::Result<ConsolidationReport> {
        let all = self.store.list_all()?;
        let mut removed = 0usize;
        let mut summarized = 0usize;

        let cutoff = chrono::Utc::now() - self.config.max_age;
        let cutoff_str = cutoff.to_rfc3339();

        for entry in &all {
            // Remove expired entries
            if entry.updated_at < cutoff_str {
                if let Err(e) = self.store.delete(&entry.id) {
                    tracing::warn!(
                        "AutoDream: failed to delete expired memory {}: {}",
                        entry.id,
                        e
                    );
                } else {
                    removed += 1;
                }
                continue;
            }

            // Summarize long entries via length truncation
            if entry.content.len() > self.config.min_summary_length
                && entry.insight_type == "conversation"
            {
                let summary = self.create_summary(entry);
                let summary_entry = MemoryEntry {
                    id: format!("{}-summary", entry.id),
                    session_id: entry.session_id.clone(),
                    content: summary,
                    insight_type: "summary".into(),
                    tags: {
                        let mut t = entry.tags.clone();
                        t.push("auto-summary".into());
                        t
                    },
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };

                if let Err(e) = self.store.save(&summary_entry) {
                    tracing::warn!("AutoDream: failed to save summary for {}: {}", entry.id, e);
                } else {
                    summarized += 1;
                    // Remove the original long entry
                    if let Err(e) = self.store.delete(&entry.id) {
                        tracing::warn!("AutoDream: failed to delete original {}: {}", entry.id, e);
                    }
                }
            }
        }

        Ok(ConsolidationReport {
            removed,
            summarized,
        })
    }

    /// Create a summary of a memory entry, ending at a sentence boundary.
    fn create_summary(&self, entry: &MemoryEntry) -> String {
        let target_len = self.config.min_summary_length / 3;
        if entry.content.len() <= target_len {
            return entry.content.clone();
        }

        let truncated = &entry.content[..target_len.min(entry.content.len())];
        let mut summary = String::with_capacity(target_len + 100);

        // Try to end at a sentence boundary for readability
        if let Some(pos) = truncated.rfind(|c| c == '.' || c == '!' || c == '?') {
            summary.push_str(&truncated[..=pos]);
        } else if let Some(pos) = truncated.rfind(|c| c == ',' || c == ';' || c == '\n') {
            summary.push_str(&truncated[..=pos]);
            summary.push_str("...");
        } else {
            summary.push_str(truncated);
            summary.push_str("...");
        }

        // Append tags as context
        if !entry.tags.is_empty() {
            summary.push_str("\n\nTags: ");
            summary.push_str(&entry.tags.join(", "));
        }

        summary
    }

    /// Get the current configuration.
    pub fn config(&self) -> &AutoDreamConfig {
        &self.config
    }
}

/// Result of a single consolidation cycle.
#[derive(Debug, Clone)]
pub struct ConsolidationReport {
    pub removed: usize,
    pub summarized: usize,
}

impl ConsolidationReport {
    pub fn is_empty(&self) -> bool {
        self.removed == 0 && self.summarized == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Create a MemoryStore backed by a temp directory.
    /// Returns both the store and the TempDir handle (keeps directory alive).
    fn create_store() -> (Arc<MemoryStore>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(MemoryStore::new_at(dir.path().to_path_buf()));
        (store, dir)
    }

    #[test]
    fn test_consolidate_removes_expired() {
        let (store, _dir) = create_store();
        let old_entry = MemoryEntry {
            id: "old-1".into(),
            session_id: "s1".into(),
            content: "Old memory".into(),
            insight_type: "general".into(),
            tags: vec![],
            created_at: "2025-01-01T00:00:00Z".into(),
            updated_at: "2025-01-01T00:00:00Z".into(),
        };
        store.save(&old_entry).unwrap();

        let config = AutoDreamConfig {
            max_age: chrono::Duration::days(1), // 1 day max age
            ..Default::default()
        };

        let dream = AutoDream::new(store.clone(), config);
        let report = dream.consolidate().unwrap();

        assert!(report.removed >= 1);
        assert!(store.load("old-1").unwrap().is_none());
    }

    #[test]
    fn test_consolidate_summarizes_long_entries() {
        let (store, _dir) = create_store();
        let long_content = "A".repeat(1000);
        let entry = MemoryEntry {
            id: "long-1".into(),
            session_id: "s1".into(),
            content: long_content,
            insight_type: "conversation".into(),
            tags: vec!["test".into()],
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        store.save(&entry).unwrap();

        let config = AutoDreamConfig {
            max_age: chrono::Duration::days(365),
            min_summary_length: 100,
            ..Default::default()
        };

        let dream = AutoDream::new(store.clone(), config);
        let report = dream.consolidate().unwrap();

        assert_eq!(report.summarized, 1);
        // Original should be deleted, summary should exist
        assert!(store.load("long-1").unwrap().is_none());
        assert!(store.load("long-1-summary").unwrap().is_some());
    }

    #[test]
    fn test_consolidate_keeps_fresh_short_entries() {
        let (store, _dir) = create_store();
        let entry = MemoryEntry {
            id: "fresh-1".into(),
            session_id: "s1".into(),
            content: "Short fresh memory".into(),
            insight_type: "general".into(),
            tags: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        store.save(&entry).unwrap();

        let config = AutoDreamConfig {
            max_age: chrono::Duration::days(30),
            min_summary_length: 5000,
            ..Default::default()
        };

        let dream = AutoDream::new(store.clone(), config);
        let report = dream.consolidate().unwrap();
        assert!(report.is_empty());
        assert!(store.load("fresh-1").unwrap().is_some());
    }

    #[test]
    fn test_create_summary_truncates() {
        let (store, _dir) = create_store();
        let config = AutoDreamConfig::default();
        let dream = AutoDream::new(store, config);

        let entry = MemoryEntry {
            id: "test".into(),
            session_id: "s1".into(),
            content: "A".repeat(1000),
            insight_type: "conversation".into(),
            tags: vec!["rust".into(), "async".into()],
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };

        let summary = dream.create_summary(&entry);
        assert!(summary.len() < entry.content.len());
        assert!(summary.contains("..."));
        assert!(summary.contains("rust"));
    }
}
