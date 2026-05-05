//! Message history helper

use crate::ai::Message;

/// Helper for managing message history within a session
pub struct History {
    messages: Vec<Message>,
    max_messages: usize,
}

impl History {
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_messages,
        }
    }

    pub fn add(&mut self, msg: Message) {
        self.messages.push(msg);
        if self.messages.len() > self.max_messages {
            self.messages.remove(0);
        }
    }

    pub fn all(&self) -> &[Message] {
        &self.messages
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new(1000)
    }
}
