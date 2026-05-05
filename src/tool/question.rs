//! Question tool - interactive user questioning

use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use super::*;

/// Global flag to check if there's a pending question
static PENDING_QUESTION: AtomicBool = AtomicBool::new(false);

static LAST_ANSWER: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Set the answer from the TUI or CLI
pub fn set_answer(answer: String) {
    if let Ok(mut guard) = LAST_ANSWER.lock() {
        *guard = Some(answer);
    }
    PENDING_QUESTION.store(true, Ordering::SeqCst);
}

pub struct QuestionTool;

#[async_trait]
impl Tool for QuestionTool {
    fn name(&self) -> &str {
        "question"
    }

    fn description(&self) -> &str {
        "Ask the user a question when you need their input, confirmation, or a choice between options."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The question to ask the user"
                },
                "options": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Multiple choice options (optional)",
                    "default": []
                }
            },
            "required": ["question"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let question = args.get("question")
            .and_then(|q| q.as_str())
            .unwrap_or("");

        if question.is_empty() {
            return ToolResult::err("Question is required");
        }

        let options: Vec<String> = args.get("options")
            .and_then(|o| o.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        // Format the question for display
        let mut display = format!("\n❓ {}\n", question);
        if !options.is_empty() {
            display.push_str("\nOptions:\n");
            for (i, opt) in options.iter().enumerate() {
                display.push_str(&format!("  {}. {}\n", i + 1, opt));
            }
        }
        display.push_str("\n> ");

        // Wait for answer from TUI or stdin
        // Reset the pending flag
        PENDING_QUESTION.store(false, Ordering::SeqCst);

        // In TUI mode, the TUI will set the answer via set_answer()
        // Fall back to stdin for headless/print modes
        // Guard against infinite loop: max 300 retries = ~60 seconds
        let mut retries = 0u32;
        const MAX_RETRIES: u32 = 300;
        let answer = loop {
            if retries >= MAX_RETRIES {
                break "TIMEOUT: No user input received".to_string();
            }
            retries += 1;

            if let Ok(mut guard) = LAST_ANSWER.lock() {
                if let Some(ans) = guard.take() {
                    break ans;
                }
            }

            // Try reading from stdin as fallback
            let mut input = String::new();
            match std::io::stdin().read_line(&mut input) {
                Ok(0) => {
                    // EOF (stdin closed) — break with empty to avoid busy-loop
                    break String::new();
                }
                Ok(_) => {
                    let trimmed = input.trim().to_string();
                    if !trimmed.is_empty() {
                        break trimmed;
                    }
                }
                Err(_) => {
                    // stdin error — keep waiting
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        };

        ToolResult::ok(format!("User response: {}", answer))
    }
}
