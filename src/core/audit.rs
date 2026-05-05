//! Audit logging - structured, append-only audit trail
//!
//! Records security-relevant events to `~/.coder/audit.log` in JSONL format.
//! Events include credential access, tool approvals/denials, privilege
//! escalation, and configuration changes.

use std::path::PathBuf;
use std::sync::Mutex;

static AUDIT_LOGGER: Mutex<Option<AuditLogger>> = Mutex::new(None);

/// Types of audit events
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AuditEventType {
    /// Tool execution started
    ToolExecution,
    /// Tool approval was granted
    ApprovalGranted,
    /// Tool approval was denied
    ApprovalDenied,
    /// API credential was accessed
    CredentialAccess,
    /// Configuration was changed
    ConfigChange,
    /// Session was created/resumed
    SessionEvent,
    /// Mode was changed
    ModeChange,
    /// Error occurred
    Error,
}

impl AuditEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditEventType::ToolExecution => "tool_execution",
            AuditEventType::ApprovalGranted => "approval_granted",
            AuditEventType::ApprovalDenied => "approval_denied",
            AuditEventType::CredentialAccess => "credential_access",
            AuditEventType::ConfigChange => "config_change",
            AuditEventType::SessionEvent => "session_event",
            AuditEventType::ModeChange => "mode_change",
            AuditEventType::Error => "error",
        }
    }
}

/// A single audit log entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEvent {
    pub timestamp: String,
    pub event_type: String,
    pub tool_name: Option<String>,
    pub session_id: Option<String>,
    pub details: String,
}

/// Structured audit logger writing to a JSONL file
pub struct AuditLogger {
    log_path: PathBuf,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(path: Option<PathBuf>) -> Self {
        let log_path = path.unwrap_or_else(default_log_path);
        // Ensure directory exists
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self { log_path }
    }

    /// Initialize the global audit logger
    pub fn init(path: Option<PathBuf>) {
        let logger = Self::new(path);
        let mut guard = AUDIT_LOGGER.lock().unwrap();
        *guard = Some(logger);
    }

    /// Record an audit event
    pub fn record(&self, event: &AuditEvent) -> anyhow::Result<()> {
        let line = serde_json::to_string(event)?;
        // Use std::fs::OpenOptions for append-mode writing
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    /// Get the log file path
    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }
}

/// Default audit log path: ~/.coder/audit.log
fn default_log_path() -> PathBuf {
    let mut path = crate::util::path::coder_dir();
    path.push("audit.log");
    path
}

/// Record an audit event using the global logger
pub fn record_event(
    event_type: AuditEventType,
    tool_name: Option<&str>,
    session_id: Option<&str>,
    details: impl Into<String>,
) {
    let guard = AUDIT_LOGGER.lock().unwrap();
    if let Some(logger) = guard.as_ref() {
        let event = AuditEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type: event_type.as_str().to_string(),
            tool_name: tool_name.map(String::from),
            session_id: session_id.map(String::from),
            details: details.into(),
        };
        if let Err(e) = logger.record(&event) {
            tracing::warn!("Failed to write audit log: {}", e);
        }
    } else {
        // Logger not initialized - log at debug level
        tracing::debug!(
            "Audit event (no logger): {:?} - {}",
            event_type,
            details.into()
        );
    }
}

/// Convenience functions for common audit events
pub mod events {
    use super::*;

    /// Record a tool execution
    pub fn tool_execution(tool_name: &str, session_id: Option<&str>) {
        record_event(
            AuditEventType::ToolExecution,
            Some(tool_name),
            session_id,
            format!("Tool '{}' executed", tool_name),
        );
    }

    /// Record an approval grant
    pub fn approval_granted(tool_name: &str, session_id: Option<&str>) {
        record_event(
            AuditEventType::ApprovalGranted,
            Some(tool_name),
            session_id,
            format!("Tool '{}' approved", tool_name),
        );
    }

    /// Record an approval denial
    pub fn approval_denied(tool_name: &str, reason: &str, session_id: Option<&str>) {
        record_event(
            AuditEventType::ApprovalDenied,
            Some(tool_name),
            session_id,
            format!("Tool '{}' denied: {}", tool_name, reason),
        );
    }

    /// Record credential access
    pub fn credential_access(provider: &str, session_id: Option<&str>) {
        record_event(
            AuditEventType::CredentialAccess,
            None,
            session_id,
            format!("Credential accessed for provider '{}'", provider),
        );
    }

    /// Record a mode change
    pub fn mode_change(old_mode: &str, new_mode: &str) {
        record_event(
            AuditEventType::ModeChange,
            None,
            None,
            format!("Mode changed: {} → {}", old_mode, new_mode),
        );
    }
}

/// Format the audit log for display
pub fn format_audit_log() -> String {
    let path = default_log_path();
    if !path.exists() {
        return "── Audit Log ──\n\nNo audit log found.".to_string();
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => return format!("Error reading audit log: {}", e),
    };

    let mut result = format!("── Audit Log ({}) ──\n\n", path.display());

    let lines: Vec<&str> = content.lines().collect();
    let show = lines.len().min(50); // Show last 50 entries
    let start = lines.len() - show;

    for line in &lines[start..] {
        if let Ok(event) = serde_json::from_str::<AuditEvent>(line) {
            result.push_str(&format!(
                "[{}] {} {} {}\n",
                &event.timestamp[..19].replace('T', " "),
                event.event_type,
                event.tool_name.as_deref().unwrap_or(""),
                event.details,
            ));
        }
    }

    if lines.len() > 50 {
        result.push_str(&format!("\n... {} total entries (showing last 50)", lines.len()));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_serialization() {
        let event = AuditEvent {
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            event_type: "tool_execution".to_string(),
            tool_name: Some("bash".to_string()),
            session_id: Some("test-session".to_string()),
            details: "Executed bash command".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("tool_execution"));
        assert!(json.contains("bash"));
    }

    #[test]
    fn test_audit_event_type_as_str() {
        assert_eq!(AuditEventType::ToolExecution.as_str(), "tool_execution");
        assert_eq!(AuditEventType::ApprovalGranted.as_str(), "approval_granted");
        assert_eq!(AuditEventType::Error.as_str(), "error");
    }

    #[test]
    fn test_logger_creation() {
        let tmp = tempfile::tempdir().unwrap();
        let log_path = tmp.path().join("audit.log");
        let logger = AuditLogger::new(Some(log_path.clone()));

        let event = AuditEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type: "test".to_string(),
            tool_name: None,
            session_id: None,
            details: "Test event".to_string(),
        };

        assert!(logger.record(&event).is_ok());
        assert!(log_path.exists());

        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Test event"));
    }

    #[test]
    fn test_format_empty_log() {
        let _tmp = tempfile::tempdir().unwrap();
        let result = format_audit_log();
        // Should not panic
        assert!(result.contains("Audit Log") || result.contains("No audit log"));
    }

    #[test]
    fn test_events_module() {
        // These should not panic
        events::tool_execution("test_tool", None);
        events::approval_granted("test_tool", None);
        events::approval_denied("test_tool", "no reason", None);
    }
}
