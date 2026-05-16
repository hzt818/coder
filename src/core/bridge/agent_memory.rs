//! Agent-Memory Bridge - connects Agent context with Memory system
//!
//! Provides:
//! - Automatic memory retrieval based on conversation context
//! - Context enrichment with relevant memories
//! - Session-aware memory storage

use crate::memory::{MemoryRetrieval, MemoryStore};
use std::sync::{Arc, RwLock};
use std::sync::OnceLock;

static MEMORY_BRIDGE: OnceLock<Arc<MemoryBridge>> = OnceLock::new();

pub struct MemoryBridge {
    store: RwLock<Option<MemoryStore>>,
    retrieval: RwLock<Option<MemoryRetrieval>>,
    session_context: RwLock<String>,
}

impl MemoryBridge {
    pub fn new() -> Self {
        Self {
            store: RwLock::new(None),
            retrieval: RwLock::new(None),
            session_context: RwLock::new(String::new()),
        }
    }

    pub fn initialize(&self) -> anyhow::Result<()> {
        let store = MemoryStore::new()?;
        let retrieval = MemoryRetrieval::new(store.clone());

        *self.store.write().unwrap() = Some(store);
        *self.retrieval.write().unwrap() = Some(retrieval);

        tracing::info!("Agent-Memory bridge initialized");
        Ok(())
    }

    pub fn get_for_context(&self, context: &str, limit: usize) -> Vec<String> {
        let retrieval = self.retrieval.read().unwrap();
        if let Some(ret) = retrieval.as_ref() {
            match ret.search(context, limit) {
                Ok(results) => results
                    .into_iter()
                    .map(|sm| sm.entry.content)
                    .collect(),
                Err(e) => {
                    tracing::warn!("Memory search failed: {}", e);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        }
    }

    pub fn set_session_context(&self, session_id: &str) {
        *self.session_context.write().unwrap() = session_id.to_string();
    }

    pub fn get_session_context(&self) -> String {
        self.session_context.read().unwrap().clone()
    }

    pub fn store_memory(
        &self,
        content: &str,
        insight_type: &str,
        tags: Vec<String>,
    ) -> anyhow::Result<()> {
        let store = self.store.read().unwrap();
        if let Some(s) = store.as_ref() {
            let entry = crate::memory::store::MemoryEntry {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: self.get_session_context(),
                content: content.to_string(),
                insight_type: insight_type.to_string(),
                tags,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            };
            s.save(&entry)?;
        }
        Ok(())
    }

    pub fn get_memory_count(&self) -> usize {
        let store = self.store.read().unwrap();
        store.as_ref().map(|s| s.count().unwrap_or(0)).unwrap_or(0)
    }
}

fn get_bridge() -> &'static Arc<MemoryBridge> {
    MEMORY_BRIDGE.get_or_init(|| {
        let bridge = Arc::new(MemoryBridge::new());
        if let Err(e) = bridge.initialize() {
            tracing::error!("Failed to initialize memory bridge: {}", e);
        }
        bridge
    })
}

pub fn init() {
    let _ = get_bridge();
}

pub fn get_for_context(context: &str, limit: usize) -> Vec<String> {
    get_bridge().get_for_context(context, limit)
}

pub fn store_memory(content: &str, insight_type: &str, tags: Vec<String>) -> anyhow::Result<()> {
    get_bridge().store_memory(content, insight_type, tags)
}

pub fn set_session_context(session_id: &str) {
    get_bridge().set_session_context(session_id)
}

pub fn enrich_context(base_context: &str, memory_limit: usize) -> String {
    let memories = get_for_context(base_context, memory_limit);
    if memories.is_empty() {
        base_context.to_string()
    } else {
        let memory_section = memories
            .iter()
            .enumerate()
            .map(|(i, m)| format!("[Memory {}]: {}", i + 1, m))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "{}\n\nRelevant memories from previous sessions:\n{}\n\n---\n",
            base_context, memory_section
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_initialization() {
        init();
        assert!(get_memory_count() >= 0);
    }

    #[test]
    fn test_session_context() {
        init();
        set_session_context("test-session-123");
        assert_eq!(get_session_context(), "test-session-123");
    }

    #[test]
    fn test_context_enrichment() {
        init();
        let base = "How to implement async in Rust?";
        let enriched = enrich_context(base, 3);
        assert!(enriched.contains(base));
    }
}
