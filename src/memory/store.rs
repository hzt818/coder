//! MemoryStore - persistence of memory entries as JSON files
//!
//! Each memory is stored as a single JSON file at ~/.coder/memory/{id}.json.

use serde::{Deserialize, Serialize};

/// A single memory entry persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub session_id: String,
    pub content: String,
    pub insight_type: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// File-based memory store.
///
/// Memories are stored as individual JSON files in a memory directory,
/// enabling simple inspection and manual editing.
#[derive(Clone)]
pub struct MemoryStore {
    memory_dir: std::path::PathBuf,
}

impl MemoryStore {
    /// Create a new MemoryStore, ensuring the memory directory exists.
    pub fn new() -> anyhow::Result<Self> {
        let memory_dir = crate::util::path::memory_dir();
        std::fs::create_dir_all(&memory_dir)?;
        Ok(Self { memory_dir })
    }

    /// Create a store at a specific path (for testing).
    pub fn new_at(path: std::path::PathBuf) -> Self {
        Self { memory_dir: path }
    }

    /// Save a memory entry to disk.
    pub fn save(&self, entry: &MemoryEntry) -> anyhow::Result<()> {
        let path = self.memory_dir.join(format!("{}.json", entry.id));
        let content = serde_json::to_string_pretty(entry)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Load a memory entry by ID.
    pub fn load(&self, id: &str) -> anyhow::Result<Option<MemoryEntry>> {
        let path = self.memory_dir.join(format!("{}.json", id));
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        let entry: MemoryEntry = serde_json::from_str(&content)?;
        Ok(Some(entry))
    }

    /// Delete a memory entry.
    pub fn delete(&self, id: &str) -> anyhow::Result<()> {
        let path = self.memory_dir.join(format!("{}.json", id));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// List all memory entries in the store.
    pub fn list_all(&self) -> anyhow::Result<Vec<MemoryEntry>> {
        let mut entries = Vec::new();

        if !self.memory_dir.exists() {
            return Ok(entries);
        }

        for entry in std::fs::read_dir(&self.memory_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(true, |e| e != "json") {
                continue;
            }

            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(mem) = serde_json::from_str::<MemoryEntry>(&content) {
                    entries.push(mem);
                }
            }
        }

        entries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(entries)
    }

    /// Count the number of stored entries.
    pub fn count(&self) -> anyhow::Result<usize> {
        Ok(self.list_all()?.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(id: &str) -> MemoryEntry {
        MemoryEntry {
            id: id.into(),
            session_id: "sess-1".into(),
            content: format!("Memory entry {}", id),
            insight_type: "general".into(),
            tags: vec!["test".into()],
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new_at(dir.path().to_path_buf());

        let entry = create_test_entry("test-1");
        store.save(&entry).unwrap();

        let loaded = store.load("test-1").unwrap().unwrap();
        assert_eq!(loaded.id, "test-1");
        assert_eq!(loaded.content, "Memory entry test-1");
    }

    #[test]
    fn test_list_all() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new_at(dir.path().to_path_buf());

        store.save(&create_test_entry("a")).unwrap();
        store.save(&create_test_entry("b")).unwrap();

        let entries = store.list_all().unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_delete() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new_at(dir.path().to_path_buf());

        store.save(&create_test_entry("to-delete")).unwrap();
        store.delete("to-delete").unwrap();
        assert!(store.load("to-delete").unwrap().is_none());
    }

    #[test]
    fn test_count() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new_at(dir.path().to_path_buf());

        store.save(&create_test_entry("x")).unwrap();
        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn test_load_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new_at(dir.path().to_path_buf());
        assert!(store.load("nonexistent").unwrap().is_none());
    }
}
