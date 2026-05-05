//! Session API handlers
//!
//! Provides CRUD operations for sessions and SSE streaming chat.
//! Each handler is an Axum extractor function injected with shared
//! application state.

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Json};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use crate::ai::{GenerateConfig, Message, StreamEvent};
use crate::session::{Session, SessionSummary};

use super::AppError;
use super::AppState;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Request body for creating a new session.
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    /// Optional title for the session.
    pub title: Option<String>,
}

/// Request body for sending a chat message.
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    /// The user message text.
    pub message: String,
}

/// Full session data returned by the API.
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
    pub messages: Vec<Message>,
}

impl From<Session> for SessionResponse {
    fn from(s: Session) -> Self {
        let count = s.message_count();
        Self {
            id: s.id,
            title: s.title,
            created_at: s.created_at,
            updated_at: s.updated_at,
            message_count: count,
            messages: s.messages,
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /api/sessions` -- list all saved sessions.
pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SessionSummary>>, AppError> {
    let manager = state.session_manager.lock().await;
    let sessions = manager.list()?;
    Ok(Json(sessions))
}

/// `POST /api/sessions` -- create a new session.
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    body: Option<Json<CreateSessionRequest>>,
) -> Result<Json<SessionResponse>, AppError> {
    let mut session = Session::new();
    if let Some(Json(ref req)) = body {
        if let Some(ref title) = req.title {
            session.title = title.clone();
        }
    }

    {
        let manager = state.session_manager.lock().await;
        manager.save(&session)?;
    }

    Ok(Json(SessionResponse::from(session)))
}

/// `GET /api/sessions/{id}` -- get a session by ID.
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SessionResponse>, AppError> {
    let manager = state.session_manager.lock().await;
    let session = manager
        .load(&id)?
        .ok_or_else(|| AppError::NotFound(format!("Session '{}' not found", id)))?;
    Ok(Json(SessionResponse::from(session)))
}

/// `POST /api/sessions/{id}/chat` -- send a message and stream the response.
///
/// Returns a Server-Sent Events (SSE) stream.  Event types:
///
/// | Event  | Data                          |
/// |--------|-------------------------------|
/// | `text` | A chunk of response text      |
/// | `done` | JSON `{stop_reason, usage}`   |
/// | `error`| Error message string          |
pub async fn chat_stream(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ChatRequest>,
) -> Result<Response, AppError> {
    // Load the session.
    let session = {
        let manager = state.session_manager.lock().await;
        manager
            .load(&id)?
            .ok_or_else(|| AppError::NotFound(format!("Session '{}' not found", id)))?
    };

    // Build messages with user input appended.
    let user_message = Message::user(&body.message);
    let mut messages = session.messages.clone();
    messages.push(user_message.clone());

    // Persist the user message to the session.
    {
        let manager = state.session_manager.lock().await;
        let mut updated = session;
        updated.add_message(user_message);
        manager.save(&updated)?;
    }

    // Grab the provider lock, create the stream, then drop the lock.
    let stream = {
        let provider = state.provider.lock().await;
        let tool_defs = state.tool_registry.tool_defs();
        let config = GenerateConfig::default();
        provider
            .chat_stream(&messages, &tool_defs, &config)
            .await?
    };
    // Provider lock is released here -- streaming happens without holding it.

    let event_stream = ReceiverStream::new(stream).map(|event| {
        match event {
            StreamEvent::TextChunk(text) => {
                Ok::<_, Infallible>(SseEvent::default().event("text").data(text))
            }
            StreamEvent::Done { stop_reason, usage } => {
                let payload = serde_json::json!({
                    "stop_reason": stop_reason,
                    "usage": usage.map(|u| {
                        serde_json::json!({
                            "input_tokens": u.input_tokens,
                            "output_tokens": u.output_tokens,
                            "total_tokens": u.total_tokens,
                        })
                    }),
                });
                Ok(SseEvent::default()
                    .event("done")
                    .data(payload.to_string()))
            }
            StreamEvent::Error(e) => {
                Ok(SseEvent::default().event("error").data(e))
            }
            // Tool call events are collected internally by the provider --
            // we skip them in the raw stream for the simple chat API.
            _ => Ok(SseEvent::default().event("skip").data("")),
        }
    });

    let sse = Sse::new(event_stream)
        .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(15)));

    Ok(sse.into_response())
}
