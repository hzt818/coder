//! Message passing between teammates
//!
//! Defines the message types used for inter-teammate communication
//! over tokio mpsc channels.

use serde::{Deserialize, Serialize};

use super::task::TaskAssignment;

/// Content of a message exchanged between teammates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    /// Plain text message
    Text(String),
    /// Assignment of a new task
    TaskAssignment(TaskAssignment),
    /// Request the teammate's current status
    StatusRequest,
    /// Response to a status request
    StatusResponse(String),
    /// Graceful shutdown signal
    Shutdown,
}

/// A message sent between teammates or from the manager to a teammate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeammateMessage {
    /// Who sent this message
    pub from: String,
    /// Who should receive this message
    pub to: String,
    /// The message content
    pub content: MessageContent,
    /// ISO 8601 timestamp of when the message was created
    pub timestamp: String,
}

impl TeammateMessage {
    /// Create a new text message
    pub fn text(from: impl Into<String>, to: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            content: MessageContent::Text(text.into()),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a new task assignment message
    pub fn task_assignment(
        from: impl Into<String>,
        to: impl Into<String>,
        task: TaskAssignment,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            content: MessageContent::TaskAssignment(task),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a new status request message
    pub fn status_request(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            content: MessageContent::StatusRequest,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a new shutdown message
    pub fn shutdown(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            content: MessageContent::Shutdown,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// A channel for routing messages between teammates.
///
/// Wraps a tokio mpsc sender to provide ergonomic message passing.
#[derive(Debug, Clone)]
pub struct MessageChannel {
    /// Name of the channel (usually the recipient teammate name)
    name: String,
    sender: tokio::sync::mpsc::Sender<TeammateMessage>,
}

impl MessageChannel {
    /// Create a new message channel
    pub fn new(
        name: impl Into<String>,
        sender: tokio::sync::mpsc::Sender<TeammateMessage>,
    ) -> Self {
        Self {
            name: name.into(),
            sender,
        }
    }

    /// Get the channel name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Send a message through this channel
    pub async fn send(&self, msg: TeammateMessage) -> anyhow::Result<()> {
        self.sender
            .send(msg)
            .await
            .map_err(|e| anyhow::anyhow!("Channel '{}' send failed: {}", self.name, e))
    }

    /// Check if the channel is closed
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

/// Broadcast messages to multiple channels at once
pub struct MessageBus {
    channels: Vec<MessageChannel>,
}

impl MessageBus {
    /// Create a new empty message bus
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
        }
    }

    /// Add a channel to the bus
    pub fn add_channel(mut self, channel: MessageChannel) -> Self {
        self.channels.push(channel);
        self
    }

    /// Broadcast a message to all registered channels
    pub async fn broadcast(&self, msg: TeammateMessage) -> Vec<anyhow::Result<()>> {
        let mut results = Vec::new();
        for channel in &self.channels {
            results.push(channel.send(msg.clone()).await);
        }
        results
    }

    /// Number of registered channels
    pub fn len(&self) -> usize {
        self.channels.len()
    }

    /// Check if the bus has no channels
    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_teammate_message_text() {
        let msg = TeammateMessage::text("alice", "bob", "hello");
        assert_eq!(msg.from, "alice");
        assert_eq!(msg.to, "bob");
        assert!(matches!(msg.content, MessageContent::Text(_)));
    }

    #[tokio::test]
    async fn test_message_channel() {
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        let channel = MessageChannel::new("bob", tx);
        assert_eq!(channel.name(), "bob");
        assert!(!channel.is_closed());
    }

    #[tokio::test]
    async fn test_message_bus() {
        let (tx1, _rx1) = tokio::sync::mpsc::channel(16);
        let (tx2, _rx2) = tokio::sync::mpsc::channel(16);

        let bus = MessageBus::new()
            .add_channel(MessageChannel::new("alice", tx1))
            .add_channel(MessageChannel::new("bob", tx2));

        assert_eq!(bus.len(), 2);
    }

    #[test]
    fn test_message_content_variants() {
        assert!(matches!(
            MessageContent::StatusRequest,
            MessageContent::StatusRequest
        ));
        assert!(matches!(MessageContent::Shutdown, MessageContent::Shutdown));
        assert!(matches!(
            MessageContent::Text("hello".to_string()),
            MessageContent::Text(_)
        ));
    }
}
