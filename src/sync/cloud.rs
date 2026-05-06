//! Cloud sync mechanism for remote data synchronization

use std::collections::HashMap;

use super::{SyncError, SyncItem, SyncResult, SyncStatus};

/// Cloud synchronization client
///
/// Manages syncing data to and from a remote cloud storage backend.
#[derive(Debug, Clone)]
pub struct CloudSync {
    /// Remote endpoint URL
    endpoint: Option<String>,
    /// Authentication token
    auth_token: Option<String>,
    /// Local data store (id -> SyncItem)
    local_store: HashMap<String, SyncItem>,
    /// Whether sync is enabled
    enabled: bool,
}

impl Default for CloudSync {
    fn default() -> Self {
        Self {
            endpoint: None,
            auth_token: None,
            local_store: HashMap::new(),
            enabled: false,
        }
    }
}

impl CloudSync {
    /// Create a new CloudSync instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the sync endpoint
    pub fn with_endpoint(mut self, endpoint: &str) -> Self {
        self.endpoint = Some(endpoint.to_string());
        self
    }

    /// Set the authentication token
    pub fn with_auth(mut self, token: &str) -> Self {
        self.auth_token = Some(token.to_string());
        self
    }

    /// Enable or disable sync
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if sync is configured and enabled
    pub fn is_ready(&self) -> bool {
        self.enabled && self.endpoint.is_some()
    }

    /// Add or update a local item
    pub fn add_item(&mut self, id: &str, data: Vec<u8>) {
        let item = SyncItem {
            id: id.to_string(),
            modified_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            status: SyncStatus::LocalPending,
            data,
        };
        self.local_store.insert(id.to_string(), item);
    }

    /// Get a local item by ID
    pub fn get_item(&self, id: &str) -> Option<&SyncItem> {
        self.local_store.get(id)
    }

    /// Remove a local item
    pub fn remove_item(&mut self, id: &str) {
        self.local_store.remove(id);
    }

    /// List all local items
    pub fn list_items(&self) -> Vec<&SyncItem> {
        self.local_store.values().collect()
    }

    /// Push local changes to the remote endpoint
    pub async fn push(&self) -> SyncResult<u64> {
        if !self.is_ready() {
            return Err(SyncError::NotConfigured(
                "Cloud sync is not configured or enabled".to_string(),
            ));
        }

        let pending: Vec<&SyncItem> = self
            .local_store
            .values()
            .filter(|item| item.status == SyncStatus::LocalPending)
            .collect();

        if pending.is_empty() {
            return Ok(0);
        }

        let endpoint = self
            .endpoint
            .as_ref()
            .ok_or_else(|| SyncError::NotConfigured("No endpoint configured".to_string()))?;

        let payload = serde_json::to_value(&pending)
            .map_err(|e| SyncError::Serialization(format!("Failed to serialize: {e}")))?;

        let client = reqwest::Client::new();
        let mut request = client
            .post(format!("{}/sync/push", endpoint))
            .json(&payload);

        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| SyncError::ConnectionFailed(format!("Push failed: {e}")))?;

        if !response.status().is_success() {
            return Err(SyncError::RemoteStorage(format!(
                "Push returned status: {}",
                response.status()
            )));
        }

        Ok(pending.len() as u64)
    }

    /// Pull remote changes to the local store
    pub async fn pull(&self) -> SyncResult<Vec<SyncItem>> {
        if !self.is_ready() {
            return Err(SyncError::NotConfigured(
                "Cloud sync is not configured or enabled".to_string(),
            ));
        }

        let endpoint = self
            .endpoint
            .as_ref()
            .ok_or_else(|| SyncError::NotConfigured("No endpoint configured".to_string()))?;

        let client = reqwest::Client::new();
        let mut request = client.get(format!("{}/sync/pull", endpoint));

        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| SyncError::ConnectionFailed(format!("Pull failed: {e}")))?;

        if !response.status().is_success() {
            return Err(SyncError::RemoteStorage(format!(
                "Pull returned status: {}",
                response.status()
            )));
        }

        let items: Vec<SyncItem> = response
            .json()
            .await
            .map_err(|e| SyncError::Serialization(format!("Failed to parse response: {e}")))?;

        Ok(items)
    }

    /// Perform a full bidirectional sync (push local, then pull remote)
    pub async fn sync(&mut self) -> SyncResult<SyncSummary> {
        let pushed = self.push().await?;
        let remote_items = self.pull().await?;

        let mut pulled = 0u64;
        for item in remote_items {
            if self
                .local_store
                .get(&item.id)
                .map(|existing| item.modified_at > existing.modified_at)
                .unwrap_or(true)
            {
                let mut synced_item = SyncItem {
                    status: SyncStatus::Synced,
                    ..item
                };
                synced_item.status = SyncStatus::Synced;
                self.local_store.insert(synced_item.id.clone(), synced_item);
                pulled += 1;
            }
        }

        // Mark all local items as synced
        for item in self.local_store.values_mut() {
            item.status = SyncStatus::Synced;
        }

        Ok(SyncSummary { pushed, pulled })
    }

    /// Get the endpoint URL
    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }
}

/// Summary of a sync operation
#[derive(Debug, Clone, Default)]
pub struct SyncSummary {
    /// Number of items pushed
    pub pushed: u64,
    /// Number of items pulled
    pub pulled: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_sync_new() {
        let sync = CloudSync::new();
        assert!(!sync.is_ready());
        assert!(sync.endpoint.is_none());
    }

    #[test]
    fn test_cloud_sync_with_endpoint() {
        let sync = CloudSync::new()
            .with_endpoint("https://sync.example.com")
            .with_auth("token123");
        assert!(!sync.is_ready()); // not enabled yet
        assert_eq!(sync.endpoint(), Some("https://sync.example.com"));
    }

    #[test]
    fn test_cloud_sync_is_ready() {
        let mut sync = CloudSync::new().with_endpoint("https://sync.example.com");
        sync.set_enabled(true);
        assert!(sync.is_ready());
    }

    #[test]
    fn test_add_and_get_item() {
        let mut sync = CloudSync::new();
        sync.add_item("session-1", b"test data".to_vec());
        let item = sync.get_item("session-1");
        assert!(item.is_some());
        assert_eq!(item.unwrap().id, "session-1");
        assert_eq!(item.unwrap().status, SyncStatus::LocalPending);
    }

    #[test]
    fn test_remove_item() {
        let mut sync = CloudSync::new();
        sync.add_item("session-1", b"test data".to_vec());
        sync.remove_item("session-1");
        assert!(sync.get_item("session-1").is_none());
    }

    #[test]
    fn test_list_items() {
        let mut sync = CloudSync::new();
        sync.add_item("a", vec![1]);
        sync.add_item("b", vec![2]);
        assert_eq!(sync.list_items().len(), 2);
    }

    #[test]
    fn test_push_not_configured() {
        let sync = CloudSync::new();
        let result = futures::executor::block_on(sync.push());
        assert!(result.is_err());
    }

    #[test]
    fn test_pull_not_configured() {
        let sync = CloudSync::new();
        let result = futures::executor::block_on(sync.pull());
        assert!(result.is_err());
    }
}
