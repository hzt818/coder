//! In-memory context storage

use async_trait::async_trait;
use tokio::sync::RwLock;

/// Pluggable message context storage trait
#[async_trait]
pub trait ContextStore: Send + Sync {
    /// Append a message to the context
    async fn push(&self, msg: String);
    /// Get the most recent `n` messages
    async fn recent(&self, n: usize) -> Vec<String>;
}

/// In-memory context store backed by a `RwLock<Vec<String>>`.
#[derive(Default)]
pub struct Context {
    messages: RwLock<Vec<String>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            messages: RwLock::new(Vec::new()),
        }
    }
}

#[async_trait]
impl ContextStore for Context {
    async fn push(&self, msg: String) {
        let mut w = self.messages.write().await;
        w.push(msg);
    }

    async fn recent(&self, n: usize) -> Vec<String> {
        let r = self.messages.read().await;
        let len = r.len();
        let start = if n > len { 0 } else { len.saturating_sub(n) };
        r[start..].to_vec()
    }
}

pub type SharedContext = std::sync::Arc<dyn ContextStore>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn context_stores_messages() {
        let ctx = Context::new();
        ctx.push("first".to_string()).await;
        ctx.push("second".to_string()).await;

        let recent1 = ctx.recent(1).await;
        assert_eq!(recent1, vec!["second".to_string()]);

        let recent2 = ctx.recent(5).await;
        assert_eq!(recent2, vec!["first".to_string(), "second".to_string()]);
    }
}
