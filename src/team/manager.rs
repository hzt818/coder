//! Team manager - coordinates multiple teammates

use std::collections::HashMap;
use uuid::Uuid;

use super::communication::{TeammateMessage, MessageContent};
use super::task::{TaskAssignment, TaskId};
use super::teammate::{Teammate, TeammateRole, TeammateStatus};

/// Status summary for a teammate
#[derive(Debug, Clone)]
pub struct TeammateStatusSummary {
    pub id: String,
    pub name: String,
    pub role: TeammateRole,
    pub status: TeammateStatus,
    pub current_task: Option<String>,
}

/// Manager for coordinating a team of AI agents.
///
/// Handles teammate creation, task assignment, status monitoring,
/// and message routing between teammates.
pub struct TeamManager {
    /// Map of teammate ID -> Teammate handle
    teammates: HashMap<String, Teammate>,
    /// Map of task ID -> TaskAssignment
    tasks: HashMap<TaskId, TaskAssignment>,
    /// Active event loop join handles (teammate ID -> JoinHandle)
    handles: HashMap<String, tokio::task::JoinHandle<()>>,
}

impl TeamManager {
    /// Create a new empty team manager
    pub fn new() -> Self {
        Self {
            teammates: HashMap::new(),
            tasks: HashMap::new(),
            handles: HashMap::new(),
        }
    }

    /// Register a new teammate.
    ///
    /// Returns the teammate's ID and the receiver side of an mpsc channel.
    /// The caller should spawn an event loop to process messages from the
    /// receiver. Use [`spawn_teammate_loop`] for a default implementation:
    ///
    /// ```ignore
    /// let (id, rx) = manager.register_teammate(name, role, agent_type);
    /// let handle = spawn_teammate_loop(rx);
    /// manager.set_handle(id, handle);
    /// ```
    pub fn register_teammate(
        &mut self,
        name: String,
        role: TeammateRole,
        agent_type: crate::agent::AgentType,
    ) -> (String, tokio::sync::mpsc::Receiver<TeammateMessage>) {
        let id = Uuid::new_v4().to_string();
        let (teammate, rx) =
            Teammate::create_with_channel(id.clone(), name, role, agent_type, 64);
        self.teammates.insert(id.clone(), teammate);
        (id, rx)
    }

    /// Add an already-created teammate to the manager.
    pub fn add_teammate(&mut self, teammate: Teammate) {
        self.teammates.insert(teammate.id().to_string(), teammate);
    }

    /// Get a teammate by ID
    pub fn get(&self, id: &str) -> Option<&Teammate> {
        self.teammates.get(id)
    }

    /// Assign a task to a teammate.
    ///
    /// Sends the task assignment message through the teammate's channel.
    /// Returns the task ID on success.
    pub async fn assign_task(
        &mut self,
        teammate_id: &str,
        description: String,
        context: Vec<crate::ai::Message>,
    ) -> anyhow::Result<TaskId> {
        let teammate = self
            .teammates
            .get(teammate_id)
            .ok_or_else(|| anyhow::anyhow!("Teammate '{}' not found", teammate_id))?;

        let task_id = Uuid::new_v4().to_string();
        let task = TaskAssignment::with_context(&task_id, &description, teammate_id, context);
        let msg = TeammateMessage::task_assignment("manager", teammate_id, task.clone());

        teammate.send(msg).await?;

        self.tasks.insert(task_id.clone(), task);
        Ok(task_id)
    }

    /// Update a task's status in the internal registry.
    pub fn update_task(&mut self, task_id: &str, task: TaskAssignment) {
        self.tasks.insert(task_id.to_string(), task);
    }

    /// Get the status summary of all teammates.
    pub async fn status(&self) -> Vec<TeammateStatusSummary> {
        let mut summaries = Vec::new();
        for teammate in self.teammates.values() {
            let status = teammate.status().lock().await.clone();
            let current_task = teammate.current_task_id().lock().await.clone();
            summaries.push(TeammateStatusSummary {
                id: teammate.id().to_string(),
                name: teammate.name().to_string(),
                role: teammate.role().clone(),
                status,
                current_task,
            });
        }
        summaries
    }

    /// Get a task by ID
    pub fn get_task(&self, task_id: &str) -> Option<&TaskAssignment> {
        self.tasks.get(task_id)
    }

    /// Get all tasks
    pub fn tasks(&self) -> &HashMap<TaskId, TaskAssignment> {
        &self.tasks
    }

    /// Remove a teammate by ID. Sends a shutdown message if the channel is open.
    pub async fn remove_teammate(&mut self, id: &str) -> anyhow::Result<()> {
        if let Some(teammate) = self.teammates.remove(id) {
            let msg = TeammateMessage::shutdown("manager", id);
            let _ = teammate.send(msg).await;
        }
        if let Some(handle) = self.handles.remove(id) {
            handle.abort();
        }
        Ok(())
    }

    /// Number of registered teammates
    pub fn teammate_count(&self) -> usize {
        self.teammates.len()
    }

    /// Number of tasks (all statuses)
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Check if all tasks are in a terminal state
    pub fn all_tasks_complete(&self) -> bool {
        self.tasks.values().all(|t| t.status.is_terminal())
    }

    /// Get teammate IDs by role
    pub fn find_by_role(&self, role: &TeammateRole) -> Vec<&Teammate> {
        self.teammates
            .values()
            .filter(|t| t.role() == role)
            .collect()
    }

    /// Store a join handle for a teammate's event loop.
    pub fn set_handle(&mut self, teammate_id: String, handle: tokio::task::JoinHandle<()>) {
        self.handles.insert(teammate_id, handle);
    }
}

impl Default for TeamManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn a teammate's event loop in a background task.
///
/// Reads messages from the receiver and processes them:
/// - `TaskAssignment`: logs the task and begins processing
/// - `Shutdown`: breaks the loop and terminates the task
/// - `Text`, `StatusRequest`, `StatusResponse`: logged at debug level
///
/// Use this with the receiver returned by [`TeamManager::register_teammate`].
pub fn spawn_teammate_loop(
    mut rx: tokio::sync::mpsc::Receiver<TeammateMessage>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::debug!("Teammate event loop started");
        while let Some(msg) = rx.recv().await {
            match msg.content {
                MessageContent::TaskAssignment(ref task) => {
                    tracing::info!(
                        "Processing task '{}' from {}: {}",
                        task.id,
                        msg.from,
                        task.description
                    );
                }
                MessageContent::Shutdown => {
                    tracing::debug!("Teammate received shutdown signal");
                    break;
                }
                MessageContent::Text(ref text) => {
                    tracing::debug!("Text message from {}: {}", msg.from, text);
                }
                MessageContent::StatusRequest => {
                    tracing::debug!("Status request from {}", msg.from);
                }
                MessageContent::StatusResponse(ref status) => {
                    tracing::debug!("Status response from {}: {}", msg.from, status);
                }
            }
        }
        tracing::debug!("Teammate event loop ended");
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentType;

    #[tokio::test]
    async fn test_team_manager_create() {
        let manager = TeamManager::new();
        assert_eq!(manager.teammate_count(), 0);
        assert_eq!(manager.task_count(), 0);
    }

    #[tokio::test]
    async fn test_register_teammate() {
        let mut manager = TeamManager::new();
        let (_id, _rx) = manager.register_teammate(
            "TestBot".to_string(),
            TeammateRole::Worker,
            AgentType::Coding,
        );
        assert_eq!(manager.teammate_count(), 1);
    }

    #[tokio::test]
    async fn test_get_teammate() {
        let mut manager = TeamManager::new();
        let (id, _rx) = manager.register_teammate(
            "TestBot".to_string(),
            TeammateRole::Worker,
            AgentType::Coding,
        );
        let teammate = manager.get(&id);
        assert!(teammate.is_some());
        assert_eq!(teammate.unwrap().name(), "TestBot");
    }

    #[tokio::test]
    async fn test_assign_task() {
        let mut manager = TeamManager::new();
        let (id, mut rx) = manager.register_teammate(
            "WorkerBot".to_string(),
            TeammateRole::Worker,
            AgentType::Coding,
        );

        let task_id = manager
            .assign_task(&id, "Write unit tests".to_string(), Vec::new())
            .await
            .unwrap();

        assert!(manager.get_task(&task_id).is_some());
        assert_eq!(manager.task_count(), 1);

        // Verify the message was sent through the channel
        let msg = rx.recv().await.unwrap();
        assert!(matches!(msg.content, MessageContent::TaskAssignment(_)));
    }

    #[tokio::test]
    async fn test_team_status() {
        let mut manager = TeamManager::new();
        manager.register_teammate("A".to_string(), TeammateRole::Worker, AgentType::Coding);
        manager.register_teammate("B".to_string(), TeammateRole::Reviewer, AgentType::Review);

        let statuses = manager.status().await;
        assert_eq!(statuses.len(), 2);

        let names: Vec<&str> = statuses.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"A"));
        assert!(names.contains(&"B"));
    }

    #[tokio::test]
    async fn test_find_by_role() {
        let mut manager = TeamManager::new();
        manager.register_teammate("A".to_string(), TeammateRole::Worker, AgentType::Coding);
        manager.register_teammate("B".to_string(), TeammateRole::Lead, AgentType::Coding);
        manager.register_teammate("C".to_string(), TeammateRole::Worker, AgentType::Coding);

        let workers = manager.find_by_role(&TeammateRole::Worker);
        assert_eq!(workers.len(), 2);

        let leads = manager.find_by_role(&TeammateRole::Lead);
        assert_eq!(leads.len(), 1);
    }

    #[tokio::test]
    async fn test_remove_teammate() {
        let mut manager = TeamManager::new();
        let (id, _rx) = manager.register_teammate(
            "TestBot".to_string(),
            TeammateRole::Worker,
            AgentType::Coding,
        );
        assert_eq!(manager.teammate_count(), 1);

        manager.remove_teammate(&id).await.unwrap();
        assert_eq!(manager.teammate_count(), 0);
    }
}
