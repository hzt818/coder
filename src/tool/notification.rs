//! Push notification tool — sends OS-level desktop notifications.
//!
//! Supports: terminal bell, Windows Toast, macOS notification center.
//! Ported from cc's PushNotificationTool pattern.

use super::*;
use async_trait::async_trait;
use std::io::Write;
use std::process::Command;

pub struct PushNotificationTool;

#[async_trait]
impl Tool for PushNotificationTool {
    fn name(&self) -> &str {
        "push_notification"
    }
    fn description(&self) -> &str {
        "Send an OS-level notification. Use to notify the user when a long-running task completes."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "title": { "type": "string", "description": "Notification title" },
                "message": { "type": "string", "description": "Notification body (max 200 chars)" }
            }, "required": ["message"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let title = args
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or("Coder");
        let message = args.get("message").and_then(|m| m.as_str()).unwrap_or("");
        if message.is_empty() {
            return ToolResult::err("Message is required");
        }

        let sent = if cfg!(target_os = "macos") {
            let _ = Command::new("osascript")
                .args([
                    "-e",
                    &format!(
                        "display notification \"{}\" with title \"{}\"",
                        message.replace('"', "\\\""),
                        title.replace('"', "\\\"")
                    ),
                ])
                .output();
            true
        } else if cfg!(target_os = "windows") {
            // PowerShell toast notification
            let _ = Command::new("powershell")
                .args([
                    "-Command",
                    &format!(
                        "New-BurntToastNotification -Text \"{}\", \"{}\"",
                        title, message
                    ),
                ])
                .output();
            true
        } else {
            false
        };

        // Fallback: terminal bell
        if !sent {
            print!("\x07");
            let _ = std::io::stdout().flush();
        }

        ToolResult::ok(format!("Notification sent: [{}] {}", title, message))
    }
    fn requires_permission(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_name() {
        assert_eq!(PushNotificationTool.name(), "push_notification");
    }
    #[tokio::test]
    async fn test_empty_message() {
        assert!(
            !PushNotificationTool
                .execute(serde_json::json!({}))
                .await
                .success
        );
    }
}
