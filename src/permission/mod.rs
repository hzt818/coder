//! Permission system for tool access control
//!
//! Defines permission policies (allow, deny, ask) and an evaluator
//! that checks whether a given action is permitted.

pub mod evaluator;
pub mod policy;

pub use evaluator::PermissionEvaluator;
pub use policy::{PermissionLevel, Policy, PolicySet};

/// Result type for permission operations
pub type PermissionResult<T> = std::result::Result<T, PermissionError>;

/// Errors that can occur during permission evaluation
#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    /// No matching policy found
    #[error("No policy found for action: {0}")]
    NoPolicyFound(String),
    /// Policy evaluation failed
    #[error("Policy evaluation failed: {0}")]
    EvaluationFailed(String),
    /// Action requires user confirmation
    #[error("Action requires confirmation: {0}")]
    RequiresConfirmation(String),
}

/// A resource or action that requires permission
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Action {
    /// Action name (e.g., "file_write", "bash")
    pub name: String,
    /// Optional resource path or identifier
    pub resource: Option<String>,
}

impl Action {
    /// Create a new action
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            resource: None,
        }
    }

    /// Create a new action with a specific resource
    pub fn with_resource(name: &str, resource: &str) -> Self {
        Self {
            name: name.to_string(),
            resource: Some(resource.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_creation() {
        let action = Action::new("bash");
        assert_eq!(action.name, "bash");
        assert!(action.resource.is_none());
    }

    #[test]
    fn test_action_with_resource() {
        let action = Action::with_resource("file_write", "/tmp/test.txt");
        assert_eq!(action.name, "file_write");
        assert_eq!(action.resource, Some("/tmp/test.txt".to_string()));
    }

    #[test]
    fn test_permission_error_display() {
        let err = PermissionError::NoPolicyFound("bash".to_string());
        assert_eq!(err.to_string(), "No policy found for action: bash");
    }
}
