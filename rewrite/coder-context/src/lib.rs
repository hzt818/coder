use tokio::sync::RwLock;
use async_trait::async_trait;
use rusqlite::{params, Connection, Result};
use tokio::task;
use std::sync::{Arc, Mutex};

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

pub type SharedContext = Arc<dyn ContextStore>;

/// SQLite-backed implementation
pub struct SqliteContext {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteContext {
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY AUTOINCREMENT, message TEXT NOT NULL)",
            [],
        )?;
        Ok(SqliteContext { conn: Arc::new(Mutex::new(conn)) })
    }

    pub async fn push(&self, msg: String) -> Result<()> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            conn.execute("INSERT INTO messages (message) VALUES (?1)", params![msg])?;
            Ok(())
        })
        .await
        .unwrap()
    }

    pub async fn recent(&self, n: usize) -> Result<Vec<String>> {
        let conn = Arc::clone(&self.conn);
        task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT message FROM messages ORDER BY id DESC LIMIT ?1")?;
            let rows = stmt
                .query_map(params![n as i64], |row| Ok(row.get::<_, String>(0)?))?
                .collect::<Result<Vec<String>, rusqlite::Error>>()?;
            let mut v = rows;
            v.reverse();
            Ok(v)
        })
        .await
        .unwrap()
    }
}

#[async_trait]
impl ContextStore for SqliteContext {
    async fn push(&self, msg: String) {
        let _ = self.push(msg).await;
    }

    async fn recent(&self, n: usize) -> Vec<String> {
        match self.recent(n).await {
            Ok(v) => v,
            Err(_) => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sqlite_roundtrip() {
        let ctx = SqliteContext::new_in_memory().unwrap();
        ctx.push("x".to_string()).await.unwrap();
        ctx.push("y".to_string()).await.unwrap();
        let r = ctx.recent(10).await.unwrap();
        assert_eq!(r, vec!["x".to_string(), "y".to_string()]);
    }
}
