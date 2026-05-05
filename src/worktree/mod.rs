//! Worktree module - Git worktree management
//!
//! Provides functionality for creating, listing, and removing
//! Git worktrees for isolated development environments.

pub mod manager;

pub use manager::WorktreeManager;

/// Result type for worktree operations
pub type WorktreeResult<T> = std::result::Result<T, WorktreeError>;

/// Errors that can occur during worktree operations
#[derive(Debug, thiserror::Error)]
pub enum WorktreeError {
    /// Git operation failed
    #[error("Git worktree operation failed: {0}")]
    Git(String),
    /// Worktree not found
    #[error("Worktree not found: {0}")]
    NotFound(String),
    /// Worktree already exists
    #[error("Worktree already exists: {0}")]
    AlreadyExists(String),
    /// Invalid worktree name
    #[error("Invalid worktree name: {0}")]
    InvalidName(String),
    /// Branch conflict
    #[error("Branch already checked out: {0}")]
    BranchConflict(String),
    /// Repository error
    #[error("Repository error: {0}")]
    Repository(String),
}

/// Status of a worktree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeStatus {
    /// Worktree is clean (no uncommitted changes)
    Clean,
    /// Worktree has uncommitted changes
    Dirty,
    /// Worktree is bare
    Bare,
}

/// Information about a git worktree
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// Path to the worktree
    pub path: String,
    /// Branch name checked out in this worktree
    pub branch: String,
    /// Current HEAD commit hash
    pub head: String,
    /// Worktree status
    pub status: WorktreeStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worktree_error_display() {
        let err = WorktreeError::NotFound("/tmp/missing".to_string());
        assert_eq!(err.to_string(), "Worktree not found: /tmp/missing");
    }

    #[test]
    fn test_worktree_status_display() {
        assert_eq!(format!("{:?}", WorktreeStatus::Clean), "Clean");
        assert_eq!(format!("{:?}", WorktreeStatus::Dirty), "Dirty");
    }
}
