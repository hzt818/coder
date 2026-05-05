//! Teammate definition for multi-agent collaboration

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::communication::TeammateMessage;

/// Role of a teammate within a team
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TeammateRole {
    /// Team lead, responsible for coordination
    Lead,
    /// General worker agent
    Worker,
    /// Code reviewer
    Reviewer,
    /// Research specialist
    Researcher,
}

impl TeammateRole {
    /// Human-readable label for the role
    pub fn label(&self) -> &'static str {
        match self {
            TeammateRole::Lead => "lead",
            TeammateRole::Worker => "worker",
            TeammateRole::Reviewer => "reviewer",
            TeammateRole::Researcher => "researcher",
        }
    }
}

impl std::fmt::Display for TeammateRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Status of a teammate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TeammateStatus {
    /// Ready to accept work
    Idle,
    /// Currently processing a task
    Busy,
    /// Encountered an error
    Error(String),
}

impl std::fmt::Display for TeammateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeammateStatus::Idle => write!(f, "idle"),
            TeammateStatus::Busy => write!(f, "busy"),
            TeammateStatus::Error(msg) => write!(f, "error: {}", msg),
        }
    }
}

/// A teammate that can participate in collaborative tasks.
///
/// Each teammate has a role, agent type, and a communication channel
/// for receiving messages from the team manager or other teammates.
pub struct Teammate {
    id: String,
    name: String,
    role: TeammateRole,
    agent_type: crate::agent::AgentType,
    status: Arc<Mutex<TeammateStatus>>,
    current_task_id: Arc<Mutex<Option<String>>>,
    /// Channel for sending messages to this teammate's event loop
    sender: tokio::sync::mpsc::Sender<TeammateMessage>,
}

impl Teammate {
    /// Create a new teammate with the given parameters.
    pub fn new(
        id: String,
        name: String,
        role: TeammateRole,
        agent_type: crate::agent::AgentType,
        sender: tokio::sync::mpsc::Sender<TeammateMessage>,
    ) -> Self {
        Self {
            id,
            name,
            role,
            agent_type,
            status: Arc::new(Mutex::new(TeammateStatus::Idle)),
            current_task_id: Arc::new(Mutex::new(None)),
            sender,
        }
    }

    /// Get the teammate's unique identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the teammate's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the teammate's role.
    pub fn role(&self) -> &TeammateRole {
        &self.role
    }

    /// Get the teammate's agent type.
    pub fn agent_type(&self) -> &crate::agent::AgentType {
        &self.agent_type
    }

    /// Get a reference to the shared status.
    pub fn status(&self) -> &Arc<Mutex<TeammateStatus>> {
        &self.status
    }

    /// Get a reference to the shared current task id.
    pub fn current_task_id(&self) -> &Arc<Mutex<Option<String>>> {
        &self.current_task_id
    }

    /// Get the channel sender for this teammate.
    pub fn sender(&self) -> &tokio::sync::mpsc::Sender<TeammateMessage> {
        &self.sender
    }

    /// Send a message to this teammate. Returns an error if the channel is closed.
    pub async fn send(&self, msg: TeammateMessage) -> anyhow::Result<()> {
        self.sender
            .send(msg)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send message to teammate '{}': {}", self.name, e))
    }

    /// Create a new teammate without spawning a listener.
    /// Returns the teammate handle and the receiver side for the event loop.
    pub fn create_with_channel(
        id: String,
        name: String,
        role: TeammateRole,
        agent_type: crate::agent::AgentType,
        buffer: usize,
    ) -> (Self, tokio::sync::mpsc::Receiver<TeammateMessage>) {
        let (tx, rx) = tokio::sync::mpsc::channel(buffer);
        let teammate = Self::new(id, name, role, agent_type, tx);
        (teammate, rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentType;

    fn create_test_teammate() -> (Teammate, tokio::sync::mpsc::Receiver<TeammateMessage>) {
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        let teammate = Teammate::new(
            "test-1".to_string(),
            "TestBot".to_string(),
            TeammateRole::Worker,
            AgentType::Coding,
            tx,
        );
        (teammate, rx)
    }

    #[test]
    fn test_teammate_creation() {
        let (teammate, _rx) = create_test_teammate();
        assert_eq!(teammate.id(), "test-1");
        assert_eq!(teammate.name(), "TestBot");
        assert_eq!(teammate.role(), &TeammateRole::Worker);
        assert_eq!(teammate.agent_type(), &AgentType::Coding);
    }

    #[test]
    fn test_teammate_role_display() {
        assert_eq!(TeammateRole::Lead.to_string(), "lead");
        assert_eq!(TeammateRole::Worker.to_string(), "worker");
        assert_eq!(TeammateRole::Reviewer.to_string(), "reviewer");
        assert_eq!(TeammateRole::Researcher.to_string(), "researcher");
    }

    #[test]
    fn test_teammate_status_display() {
        assert_eq!(TeammateStatus::Idle.to_string(), "idle");
        assert_eq!(TeammateStatus::Busy.to_string(), "busy");
        assert!(TeammateStatus::Error("oops".to_string()).to_string().contains("error"));
    }

    #[tokio::test]
    async fn test_teammate_send_message() {
        let (teammate, mut rx) = create_test_teammate();
        let msg = TeammateMessage {
            from: "manager".to_string(),
            to: "test-1".to_string(),
            content: crate::team::MessageContent::Text("hello".to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        teammate.send(msg.clone()).await.unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.from, "manager");
        assert_eq!(received.to, "test-1");
    }
}
