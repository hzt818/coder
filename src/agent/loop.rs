//! Agent loop - the core ReAct conversation loop

use crate::ai::*;
use crate::tool::*;
use super::context::Context;
use super::types::AgentType;

#[cfg(feature = "permission")]
use crate::permission::{Action, PermissionEvaluator};

/// The main agent responsible for managing conversations
pub struct Agent {
    provider: Box<dyn Provider>,
    tools: ToolRegistry,
    context: Context,
    agent_type: AgentType,
    session: crate::session::Session,
    #[cfg(feature = "permission")]
    permission_evaluator: Option<PermissionEvaluator>,
}

impl Agent {
    /// Create a new agent
    pub fn new(provider: Box<dyn Provider>, tools: ToolRegistry) -> Self {
        let agent_type = AgentType::Coding;
        let mut context = Context::new(128_000);
        context.set_system_prompt(agent_type.system_prompt().to_string());

        Self {
            provider,
            tools,
            context,
            agent_type,
            session: crate::session::Session::new(),
            #[cfg(feature = "permission")]
            permission_evaluator: None,
        }
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
    fn check_tool_permission(&self, tool_name: &str) -> Option<String> {
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

    /// Simple one-shot query (for --print mode)
    pub async fn run_simple(&self, query: &str) -> anyhow::Result<String> {
        let messages = vec![Message::user(query)];
        let tool_defs = self.tools.tool_defs();
        let config = GenerateConfig::default();

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

        // Main ReAct loop (max 10 turns to prevent infinite loops)
        for _turn in 0..10 {
            let messages = self.context.build_request();
            let tool_defs = self.tools.tool_defs();
            let config = GenerateConfig::default();

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
                                let _ = tx.send(AgentEvent::TextChunk(chunk)).await;
                            }
                            StreamEvent::ToolCallStart(tc) => {
                                has_tool_call = true;
                                tool_calls.push(tc.clone());
                                let _ = tx
                                    .send(AgentEvent::ToolCallStart {
                                        id: tc.id,
                                        name: tc.name,
                                    })
                                    .await;
                            }
                            StreamEvent::Done {
                                stop_reason,
                                usage,
                            } => {
                                final_stop_reason = stop_reason;
                                final_usage = usage;
                            }
                            StreamEvent::Error(e) => {
                                let _ = tx.send(AgentEvent::Error(e)).await;
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
                        let _ = tx
                            .send(AgentEvent::ToolResult {
                                tool_name: tc.name.clone(),
                                result,
                            })
                            .await;
                    }

                    // Send Done event
                    let _ = tx
                        .send(AgentEvent::Done {
                            stop_reason: final_stop_reason,
                            usage: final_usage,
                        })
                        .await;

                    if !has_tool_call {
                        break; // No tool calls, conversation turn complete
                    }
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
    ThinkingStart {
        provider: String,
        model: String,
    },
    /// Text content chunk
    TextChunk(String),
    /// Tool call started
    ToolCallStart {
        id: String,
        name: String,
    },
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
}
