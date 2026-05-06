//! Hook dispatcher — lifecycle event broadcasting
//!
//! Provides a pluggable hook system that emits structured events to
//! configurable sinks (stdout, JSONL file, webhook). Useful for
//! observability, auditing, and external integrations.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Structured hook event with type-tag for easy filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookEvent {
    /// Agent response started streaming.
    ResponseStart { response_id: String },
    /// A chunk of the agent response.
    ResponseDelta { response_id: String, delta: String },
    /// Agent response finished.
    ResponseEnd { response_id: String },
    /// Tool call lifecycle event (call/result/error).
    ToolLifecycle {
        response_id: String,
        tool_name: String,
        phase: String, // "call" | "result" | "error"
        payload: Value,
    },
    /// Background job lifecycle.
    JobLifecycle {
        job_id: String,
        phase: String, // "queued" | "running" | "completed" | "failed" | "cancelled"
        progress: Option<u8>,
        detail: Option<String>,
    },
    /// Approval request lifecycle.
    ApprovalLifecycle {
        approval_id: String,
        phase: String, // "requested" | "approved" | "denied"
        reason: Option<String>,
    },
}

impl HookEvent {
    /// Serialize the event to a JSON Value.
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| json!({"type":"serialization_error"}))
    }
}

// ── Sinks ────────────────────────────────────────────────────────────────────

/// A sink that can consume hook events.
#[async_trait]
pub trait HookSink: Send + Sync {
    async fn emit(&self, event: &HookEvent) -> Result<()>;
}

/// Sink that prints events to stdout as JSON lines.
#[derive(Default)]
pub struct StdoutHookSink;

#[async_trait]
impl HookSink for StdoutHookSink {
    async fn emit(&self, event: &HookEvent) -> Result<()> {
        println!("{}", event.to_json());
        Ok(())
    }
}

/// Sink that appends events as JSONL to a file.
pub struct JsonlHookSink {
    path: PathBuf,
}

impl JsonlHookSink {
    /// Create a new JSONL hook sink writing to `path`.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[async_trait]
impl HookSink for JsonlHookSink {
    async fn emit(&self, event: &HookEvent) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!("failed to create hook log directory {}", parent.display())
            })?;
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await
            .with_context(|| format!("failed to open hook log {}", self.path.display()))?;

        let payload = serde_json::to_string(&event).context("failed to encode hook event")?;
        use tokio::io::AsyncWriteExt;
        file.write_all(payload.as_bytes())
            .await
            .context("failed to write hook event")?;
        file.write_all(b"\n")
            .await
            .context("failed to write hook event newline")?;
        Ok(())
    }
}

/// Sink that POSTs events as JSON to an HTTP endpoint with retry.
pub struct WebhookHookSink {
    url: String,
    client: reqwest::Client,
}

impl WebhookHookSink {
    /// Create a new webhook sink posting to `url`.
    pub fn new(url: String) -> Self {
        Self {
            url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl HookSink for WebhookHookSink {
    async fn emit(&self, event: &HookEvent) -> Result<()> {
        let mut retries = 0usize;
        loop {
            let resp = self
                .client
                .post(&self.url)
                .json(&json!({ "event": event }))
                .send()
                .await;
            match resp {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response) => {
                    if retries >= 2 {
                        anyhow::bail!("webhook returned non-success status {}", response.status());
                    }
                }
                Err(err) => {
                    if retries >= 2 {
                        return Err(err).context("webhook request failed");
                    }
                }
            }
            retries += 1;
            tokio::time::sleep(std::time::Duration::from_millis(200 * retries as u64)).await;
        }
    }
}

// ── Dispatcher ───────────────────────────────────────────────────────────────

/// Dispatches [`HookEvent`]s to all registered [`HookSink`]s.
///
/// Each sink is called concurrently (fire-and-forget). Failures from
/// individual sinks are logged but do not propagate to callers.
#[derive(Default, Clone)]
pub struct HookDispatcher {
    sinks: Vec<Arc<dyn HookSink>>,
}

impl HookDispatcher {
    /// Create a new empty dispatcher.
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    /// Register a sink. Sinks are called in registration order.
    pub fn add_sink(&mut self, sink: Arc<dyn HookSink>) {
        self.sinks.push(sink);
    }

    /// Emit an event to all registered sinks.
    ///
    /// Sink failures are logged through `tracing::warn!` but not propagated.
    pub async fn emit(&self, event: HookEvent) {
        for sink in &self.sinks {
            if let Err(e) = sink.emit(&event).await {
                tracing::warn!("Hook sink error: {:#}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = HookEvent::ResponseStart {
            response_id: "test-1".into(),
        };
        let json = event.to_json();
        assert_eq!(json["type"], "response_start");
        assert_eq!(json["response_id"], "test-1");
    }

    #[tokio::test]
    async fn test_stdout_sink_does_not_panic() {
        let sink = StdoutHookSink;
        let event = HookEvent::ResponseEnd {
            response_id: "r1".into(),
        };
        // Should not panic
        let result = sink.emit(&event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_jsonl_sink_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hooks.jsonl");
        let sink = JsonlHookSink::new(path.clone());

        let event = HookEvent::ToolLifecycle {
            response_id: "r1".into(),
            tool_name: "bash".into(),
            phase: "call".into(),
            payload: serde_json::json!({"cmd": "echo hello"}),
        };

        sink.emit(&event).await.unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("tool_lifecycle"));
        assert!(content.contains("bash"));
    }

    #[tokio::test]
    async fn test_jsonl_sink_appends_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hooks.jsonl");
        let sink = JsonlHookSink::new(path.clone());

        sink.emit(&HookEvent::ResponseStart {
            response_id: "1".into(),
        })
        .await
        .unwrap();
        sink.emit(&HookEvent::ResponseStart {
            response_id: "2".into(),
        })
        .await
        .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.lines().filter(|l| !l.is_empty()).count(), 2);
    }

    #[test]
    fn test_dispatcher_default() {
        let d = HookDispatcher::new();
        assert!(d.sinks.is_empty());
    }

    #[tokio::test]
    async fn test_dispatcher_with_sinks() {
        let mut d = HookDispatcher::new();
        d.add_sink(Arc::new(StdoutHookSink));
        d.add_sink(Arc::new(StdoutHookSink));

        // Should not panic with multiple sinks
        d.emit(HookEvent::ResponseStart {
            response_id: "multi".into(),
        })
        .await;
    }

    #[test]
    fn test_event_types_serialize_correctly() {
        let events = vec![
            HookEvent::ResponseStart {
                response_id: "a".into(),
            },
            HookEvent::ResponseDelta {
                response_id: "b".into(),
                delta: "hello".into(),
            },
            HookEvent::ResponseEnd {
                response_id: "c".into(),
            },
            HookEvent::ToolLifecycle {
                response_id: "d".into(),
                tool_name: "test".into(),
                phase: "call".into(),
                payload: serde_json::json!({}),
            },
            HookEvent::JobLifecycle {
                job_id: "j1".into(),
                phase: "running".into(),
                progress: Some(50),
                detail: None,
            },
            HookEvent::ApprovalLifecycle {
                approval_id: "a1".into(),
                phase: "requested".into(),
                reason: Some("needs approval".into()),
            },
        ];
        for event in &events {
            let json = event.to_json();
            assert!(
                json.get("type").is_some(),
                "Event {:?} missing type tag",
                event
            );
        }
    }
}
