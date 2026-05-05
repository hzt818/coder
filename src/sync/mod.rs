//! Sync module - data synchronization capabilities
//!
//! Provides cloud-based synchronization for sessions, configuration,
//! and other data across multiple devices.

pub mod cloud;

pub use cloud::CloudSync;

/// Result type for sync operations
pub type SyncResult<T> = std::result::Result<T, SyncError>;

/// Errors that can occur during sync operations
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    /// Sync conflict detected
    #[error("Sync conflict: {0}")]
    Conflict(String),
    /// Data serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
    /// Remote storage error
    #[error("Remote storage error: {0}")]
    RemoteStorage(String),
    /// Sync not configured
    #[error("Sync not configured: {0}")]
    NotConfigured(String),
}

/// Sync direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SyncDirection {
    /// Upload local data to remote
    Upload,
    /// Download remote data to local
    Download,
    /// Bidirectional sync (merge)
    Bidirectional,
}

/// Sync status of an item
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SyncStatus {
    /// Data is in sync
    Synced,
    /// Local changes pending upload
    LocalPending,
    /// Remote changes pending download
    RemotePending,
    /// Conflict detected
    Conflict,
}

/// A syncable data item
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncItem {
    /// Unique identifier
    pub id: String,
    /// Last modified timestamp (Unix epoch seconds)
    pub modified_at: u64,
    /// Current sync status
    pub status: SyncStatus,
    /// Data payload
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_error_display() {
        let err = SyncError::ConnectionFailed("timeout".to_string());
        assert_eq!(err.to_string(), "Connection failed: timeout");
    }

    #[test]
    fn test_sync_direction_display() {
        assert_eq!(format!("{:?}", SyncDirection::Upload), "Upload");
        assert_eq!(format!("{:?}", SyncDirection::Bidirectional), "Bidirectional");
    }

    #[test]
    fn test_sync_status_display() {
        assert_eq!(format!("{:?}", SyncStatus::Synced), "Synced");
        assert_eq!(format!("{:?}", SyncStatus::Conflict), "Conflict");
    }
}
