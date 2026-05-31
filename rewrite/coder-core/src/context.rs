use tokio::sync::RwLock;
use async_trait::async_trait;

/// Simple in-memory context store for messages.
#[derive(Default)]
pub struct Context {
    messages: RwLock<Vec<String>>,
}

impl Context {
    pub fn new() -> Self {
        Self { messages: RwLock::new(Vec::new()) }
    }

    pub async fn push(&self, msg: String) {
        let mut w = self.messages.write().await;
        w.push(msg);
    }

    pub async fn recent(&self, n: usize) -> Vec<String> {
        let r = self.messages.read().await;
        let len = r.len();
        let start = if n > len { 0 } else { len.saturating_sub(n) };
        r[start..].to_vec()
    }
}

#[async_trait]
pub trait ContextStore: Send + Sync {
    async fn push(&self, msg: String);
    async fn recent(&self, n: usize) -> Vec<String>;
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
