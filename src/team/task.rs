//! Task assignment types for team collaboration

use serde::{Deserialize, Serialize};

/// A unique identifier for a task
pub type TaskId = String;

/// Status of a task assignment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    /// Task has been created but not yet started
    Pending,
    /// Task is currently being worked on
    InProgress,
    /// Task completed successfully
    Completed,
    /// Task failed with an error message
    Failed(String),
}

impl TaskStatus {
    /// Check if the task is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Failed(_))
    }

    /// Human-readable label
    pub fn label(&self) -> &str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed(_) => "failed",
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A task assigned to a teammate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    /// Unique task identifier
    pub id: TaskId,
    /// Human-readable description of the task
    pub description: String,
    /// ID of the teammate assigned to this task
    pub assigned_to: String,
    /// Conversation context for the task
    pub context: Vec<crate::ai::Message>,
    /// Current status of the task
    pub status: TaskStatus,
    /// Result output (populated on completion)
    pub result: Option<String>,
    /// ISO 8601 creation timestamp
    pub created_at: String,
    /// ISO 8601 completion timestamp
    pub completed_at: Option<String>,
}

impl TaskAssignment {
    /// Create a new task assignment
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        assigned_to: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            assigned_to: assigned_to.into(),
            context: Vec::new(),
            status: TaskStatus::Pending,
            result: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            completed_at: None,
        }
    }

    /// Create a new task with initial context
    pub fn with_context(
        id: impl Into<String>,
        description: impl Into<String>,
        assigned_to: impl Into<String>,
        context: Vec<crate::ai::Message>,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            assigned_to: assigned_to.into(),
            context,
            status: TaskStatus::Pending,
            result: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            completed_at: None,
        }
    }

    /// Mark this task as in progress (returns a new copy)
    pub fn mark_in_progress(&self) -> Self {
        Self {
            status: TaskStatus::InProgress,
            ..self.clone()
        }
    }

    /// Mark this task as completed with a result (returns a new copy)
    pub fn mark_completed(&self, result: impl Into<String>) -> Self {
        Self {
            status: TaskStatus::Completed,
            result: Some(result.into()),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            ..self.clone()
        }
    }

    /// Mark this task as failed with an error (returns a new copy)
    pub fn mark_failed(&self, error: impl Into<String>) -> Self {
        Self {
            status: TaskStatus::Failed(error.into()),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = TaskAssignment::new("task-1", "Write tests", "alice");
        assert_eq!(task.id, "task-1");
        assert_eq!(task.description, "Write tests");
        assert_eq!(task.assigned_to, "alice");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.result.is_none());
    }

    #[test]
    fn test_task_status_transitions() {
        let task = TaskAssignment::new("task-1", "Do something", "bob");
        assert!(!task.status.is_terminal());

        let in_progress = task.mark_in_progress();
        assert_eq!(in_progress.status, TaskStatus::InProgress);

        let completed = in_progress.mark_completed("done");
        assert_eq!(completed.status, TaskStatus::Completed);
        assert!(completed.status.is_terminal());
        assert_eq!(completed.result.unwrap(), "done");
        assert!(completed.completed_at.is_some());
    }

    #[test]
    fn test_task_failure() {
        let task = TaskAssignment::new("task-1", "Do risky thing", "charlie");
        let failed = task.mark_failed("timeout");
        assert_eq!(failed.status, TaskStatus::Failed("timeout".to_string()));
        assert!(failed.status.is_terminal());
    }

    #[test]
    fn test_task_status_labels() {
        assert_eq!(TaskStatus::Pending.label(), "pending");
        assert_eq!(TaskStatus::InProgress.label(), "in_progress");
        assert_eq!(TaskStatus::Completed.label(), "completed");
        assert_eq!(TaskStatus::Failed("err".to_string()).label(), "failed");
    }

    #[test]
    fn test_task_with_context() {
        let context = vec![crate::ai::Message::user("context info")];
        let task = TaskAssignment::with_context("task-2", "Review PR", "reviewer", context.clone());
        assert_eq!(task.context.len(), 1);
        assert_eq!(task.context[0].text(), "context info");
    }
}
