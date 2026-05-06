//! Agent loop - the core ReAct conversation loop

use super::auto_reasoning;
use super::context::Context;
use super::types::{AgentType, InteractionMode, ReasoningEffort};
use crate::ai::*;
use crate::tool::*;

#[cfg(feature = "permission")]
use crate::permission::{Action, PermissionEvaluator};

/// The main agent responsible for managing conversations
pub struct Agent {
    provider: Box<dyn Provider>,
    tools: ToolRegistry,
    context: Context,
    agent_type: AgentType,
    session: crate::session::Session,
    /// Current interaction mode
    mode: InteractionMode,
    /// Current reasoning effort
    reasoning_effort: ReasoningEffort,
    /// Maximum tool-calling rounds in the ReAct loop before forced stop.
    /// Override via CODER_MAX_TOOL_ROUNDS env var.
    max_tool_rounds: usize,
    #[cfg(feature = "permission")]
    permission_evaluator: Option<PermissionEvaluator>,
}

impl Agent {
    /// Create a new agent
    pub fn new(provider: Box<dyn Provider>, tools: ToolRegistry) -> Self {
        let agent_type = AgentType::Coding;
        let mut context = Context::new(128_000);
        context.set_system_prompt(agent_type.system_prompt().to_string());

        let max_tool_rounds = std::env::var("CODER_MAX_TOOL_ROUNDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);

        Self {
            provider,
            tools,
            context,
            agent_type,
            session: crate::session::Session::new(),
            mode: InteractionMode::default(),
            reasoning_effort: ReasoningEffort::default(),
            max_tool_rounds,
            #[cfg(feature = "permission")]
            permission_evaluator: None,
        }
    }

    /// Get the current interaction mode
    pub fn mode(&self) -> InteractionMode {
        self.mode
    }

    /// Cycle to the next interaction mode
    pub fn cycle_mode(&mut self) -> InteractionMode {
        self.mode = self.mode.cycle();
        self.mode
    }

    /// Get the current reasoning effort
    pub fn reasoning_effort(&self) -> ReasoningEffort {
        self.reasoning_effort
    }

    /// Cycle to the next reasoning effort level
    pub fn cycle_reasoning_effort(&mut self) -> ReasoningEffort {
        self.reasoning_effort = self.reasoning_effort.cycle();
        self.reasoning_effort
    }

    /// Set reasoning effort explicitly
    pub fn set_reasoning_effort(&mut self, effort: ReasoningEffort) {
        self.reasoning_effort = effort;
    }

    /// Get a reference to the tool registry
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    /// Get agent type
    pub fn agent_type(&self) -> &AgentType {
        &self.agent_type
    }

    /// Get mutable reference to context
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Get session
    pub fn session(&self) -> &crate::session::Session {
        &self.session
    }

    /// Get mutable session
    pub fn session_mut(&mut self) -> &mut crate::session::Session {
        &mut self.session
    }

    /// Set the permission evaluator for this agent
    #[cfg(feature = "permission")]
    pub fn with_permission_evaluator(mut self, evaluator: PermissionEvaluator) -> Self {
        self.permission_evaluator = Some(evaluator);
        self
    }

    /// Check whether a tool is permitted to execute.
    ///
    /// Returns `None` if the tool is allowed to proceed, or `Some(error_message)`
    /// if execution should be blocked with the given error.
    /// Respects the current InteractionMode:
    /// - Plan: only read-only tools allowed; shell/patch blocked
    /// - YOLO: all tools auto-approved
    /// - Agent: uses standard permission checks
    fn check_tool_permission(&self, tool_name: &str) -> Option<String> {
        // YOLO mode: auto-approve everything
        if self.mode.auto_approve_all() {
            return None;
        }

        // Plan mode: only allow read-only tools
        if self.mode.is_read_only() {
            // Block shell execution
            if tool_name == "bash" || tool_name == "exec_shell" {
                return Some(format!(
                    "Shell execution is not allowed in Plan mode. \
                     Switch to Agent or YOLO mode to run '{}'.",
                    tool_name
                ));
            }

            // Block file write/modify operations
            if tool_name == "file_write" || tool_name == "file_edit" || tool_name == "apply_patch" {
                return Some(format!(
                    "'{}' is not allowed in Plan mode. \
                     Use read-only tools like file_read, grep, list_dir.",
                    tool_name
                ));
            }
        }

        // For tools that explicitly require permission, check with evaluator
        let tool = match self.tools.get(tool_name) {
            Some(t) => t,
            None => return None,
        };

        if !tool.requires_permission() {
            return None;
        }

        #[cfg(not(feature = "permission"))]
        {
            tracing::info!(
                "Tool '{}' requires permission (permission feature not enabled), allowing",
                tool_name
            );
            return None;
        }

        #[cfg(feature = "permission")]
        self.check_permission_evaluator(tool_name)
    }

    /// Evaluate permission using the configured PermissionEvaluator.
    #[cfg(feature = "permission")]
    fn check_permission_evaluator(&self, tool_name: &str) -> Option<String> {
        let evaluator = self.permission_evaluator.as_ref()?;
        let action = Action::new(tool_name);

        if evaluator.is_allowed(&action) {
            None
        } else if evaluator.requires_confirmation(&action) {
            // Instead of silently proceeding, ask the user for confirmation
            Some(format!(
                "Permission needed: tool '{}' requires your confirmation.\nUse `/{0} <args>` to execute this command when you are ready.",
                tool_name
            ))
        } else {
            Some(format!(
                "Permission denied: tool '{}' is not allowed by policy",
                tool_name
            ))
        }
    }

    /// Build a GenerateConfig with current reasoning effort settings
    fn build_config(&self, prompt: &str) -> GenerateConfig {
        let mut config = GenerateConfig::default();

        // Resolve reasoning effort
        match self.reasoning_effort {
            ReasoningEffort::Auto => {
                let selected = auto_reasoning::select_effort(prompt);
                if let Some(api_value) = selected.api_value() {
                    config.reasoning_effort = Some(api_value.to_string());
                }
                if selected.is_thinking_enabled() {
                    config.thinking_budget = Some(2048);
                }
            }
            _ => {
                if let Some(api_value) = self.reasoning_effort.api_value() {
                    config.reasoning_effort = Some(api_value.to_string());
                }
                if self.reasoning_effort.is_thinking_enabled() {
                    config.thinking_budget = Some(2048);
                }
            }
        }

        config
    }

    /// Simple one-shot query (for --print mode)
    pub async fn run_simple(&self, query: &str) -> anyhow::Result<String> {
        let messages = vec![Message::user(query)];
        let tool_defs = self.tools.tool_defs();
        let config = self.build_config(query);

        let response = self.provider.chat(&messages, &tool_defs, &config).await?;
        Ok(response.text())
    }

    /// Run interactive session (for headless mode)
    pub async fn run_interactive(&self) -> anyhow::Result<()> {
        println!("🦀 Coder interactive mode (headless)");
        println!("Type your messages. Ctrl+C to exit.\n");

        let mut input = String::new();
        loop {
            print!("> ");
            use std::io::Write;
            std::io::stdout().flush()?;
            input.clear();
            if std::io::stdin().read_line(&mut input)? == 0 {
                break;
            }
            let input = input.trim();
            if input.is_empty() {
                continue;
            }
            if input == "/exit" || input == "/quit" {
                break;
            }

            println!();
            let response = self.run_simple(input).await?;
            println!("{}", response);
            println!();
        }
        Ok(())
    }

    /// Run the ReAct loop with streaming
    /// Returns a receiver that yields AgentEvent items for the TUI
    pub async fn run_stream(
        &mut self,
        user_input: &str,
    ) -> tokio::sync::mpsc::Receiver<AgentEvent> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);

        // Add user message to context
        self.context.add_message(Message::user(user_input));

        // Send thinking start
        let _ = tx
            .send(AgentEvent::ThinkingStart {
                provider: self.provider.name().to_string(),
                model: self.provider.model().to_string(),
            })
            .await;

        // Main ReAct loop (configurable max rounds to prevent infinite loops).
        // Override via CODER_MAX_TOOL_ROUNDS env var. Default: 50.
        let max_rounds = self.max_tool_rounds;
        for turn in 0..max_rounds {
            // Warn when nearing the limit
            if turn >= max_rounds.saturating_sub(3) {
                let remaining = max_rounds - turn;
                tracing::warn!(
                    "ReAct loop approaching limit: {} round(s) remaining",
                    remaining
                );
            }

            let messages = self.context.build_request();
            let tool_defs = self.tools.tool_defs();
            let config = self.build_config(user_input);

            match self
                .provider
                .chat_stream(&messages, &tool_defs, &config)
                .await
            {
                Ok(mut stream) => {
                    let mut has_tool_call = false;
                    let mut full_text = String::new();
                    let mut tool_calls: Vec<ToolCall> = Vec::new();
                    let mut final_stop_reason = String::new();
                    let mut final_usage = None;

                    while let Some(event) = stream.recv().await {
                        match event {
                            StreamEvent::TextChunk(chunk) => {
                                full_text.push_str(&chunk);
                                if tx.send(AgentEvent::TextChunk(chunk)).await.is_err() {
                                    tracing::warn!("Agent event channel closed, stopping stream");
                                    break;
                                }
                            }
                            StreamEvent::ToolCallStart(tc) => {
                                has_tool_call = true;
                                tool_calls.push(tc.clone());
                                if tx
                                    .send(AgentEvent::ToolCallStart {
                                        id: tc.id,
                                        name: tc.name,
                                    })
                                    .await
                                    .is_err()
                                {
                                    tracing::warn!("Agent event channel closed, stopping stream");
                                    break;
                                }
                            }
                            StreamEvent::Done { stop_reason, usage } => {
                                final_stop_reason = stop_reason;
                                final_usage = usage;
                            }
                            StreamEvent::Error(e) => {
                                if tx.send(AgentEvent::Error(e)).await.is_err() {
                                    tracing::warn!("Agent event channel closed after error");
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }

                    // Build and add assistant message with text + tool_use blocks
                    let mut assistant_msg = Message::assistant(&full_text);
                    for tc in &tool_calls {
                        assistant_msg.content.push(ContentBlock::ToolUse {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            input: tc.arguments.clone(),
                        });
                    }
                    self.context.add_message(assistant_msg);

                    // Execute each tool and add results to context
                    for tc in &tool_calls {
                        // Check permission before executing
                        if let Some(error_msg) = self.check_tool_permission(&tc.name) {
                            let result = ToolResult::err(error_msg);
                            self.context
                                .add_message(Message::tool_result(&tc.id, &result.output));
                            let _ = tx
                                .send(AgentEvent::ToolResult {
                                    tool_name: tc.name.clone(),
                                    result,
                                })
                                .await;
                            continue;
                        }

                        let result = self.tools.execute(&tc.name, tc.arguments.clone()).await;
                        self.context
                            .add_message(Message::tool_result(&tc.id, &result.output));
                        if tx
                            .send(AgentEvent::ToolResult {
                                tool_name: tc.name.clone(),
                                result,
                            })
                            .await
                            .is_err()
                        {
                            tracing::warn!("Agent event channel closed during tool result");
                            break;
                        }
                    }

                    if !has_tool_call {
                        // No tool calls — conversation turn is complete, send final Done
                        if tx
                            .send(AgentEvent::Done {
                                stop_reason: final_stop_reason,
                                usage: final_usage,
                            })
                            .await
                            .is_err()
                        {
                            tracing::warn!("Agent event channel closed at loop end");
                        }
                        break;
                    }
                    // has_tool_call → continue the ReAct loop with tool results fed back in
                }
                Err(e) => {
                    let _ = tx.send(AgentEvent::Error(e.to_string())).await;
                    break;
                }
            }
        }

        rx
    }
}

/// Events emitted by the agent loop for the TUI to consume
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// AI started thinking
    ThinkingStart { provider: String, model: String },
    /// Text content chunk
    TextChunk(String),
    /// Reasoning/thinking content chunk (reserved for future use; not yet emitted by any provider)
    ReasoningChunk(String),
    /// Tool call started
    ToolCallStart { id: String, name: String },
    /// Tool execution result
    ToolResult {
        tool_name: String,
        result: ToolResult,
    },
    /// Generation complete
    Done {
        stop_reason: String,
        usage: Option<Usage>,
    },
    /// Error occurred
    Error(String),
    /// Mode changed
    ModeChanged { mode: super::InteractionMode },
    /// Reasoning effort changed
    ReasoningEffortChanged { effort: super::ReasoningEffort },
    /// Cost estimate for the turn
    CostEstimate {
        estimate: crate::core::pricing::CostEstimate,
    },
    /// Context compaction event
    Compaction {
        result: crate::core::compaction::CompactionResult,
    },
}
