//! HTTP API server module
//!
//! Provides a RESTful HTTP API for managing sessions, tools, and real-time
//! communication via WebSocket. This module is only available with the
//! `server` feature enabled.
//!
//! # Routes
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | GET | `/api/sessions` | List all sessions |
//! | POST | `/api/sessions` | Create a new session |
//! | GET | `/api/sessions/:id` | Get session details |
//! | POST | `/api/sessions/:id/chat` | Send a message (SSE streaming) |
//! | GET | `/api/tools` | List available tools |
//! | POST | `/api/tools/:name/exec` | Execute a tool |
//! | WS | `/api/ws` | WebSocket real-time communication |
//! | GET | `/api/health` | Health check |

pub mod handler_session;
pub mod handler_tools;
pub mod router;
pub mod ws;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};

/// Shared application state for HTTP handlers.
///
/// Construct this at server startup and wrap in an `Arc` before passing
/// to `create_router`.
pub struct AppState {
    /// Session persistence manager (shared via tokio mutex for async safety).
    pub session_manager: tokio::sync::Mutex<crate::session::manager::SessionManager>,
    /// Tool registry holding all registered tool implementations.
    pub tool_registry: std::sync::Arc<crate::tool::ToolRegistry>,
    /// AI provider for chat completion and streaming.
    pub provider: tokio::sync::Mutex<Box<dyn crate::ai::Provider>>,
}

impl AppState {
    /// Create a new `AppState` from its required parts.
    pub fn new(
        session_manager: crate::session::manager::SessionManager,
        tool_registry: std::sync::Arc<crate::tool::ToolRegistry>,
        provider: Box<dyn crate::ai::Provider>,
    ) -> Self {
        Self {
            session_manager: tokio::sync::Mutex::new(session_manager),
            tool_registry,
            provider: tokio::sync::Mutex::new(provider),
        }
    }
}

/// Unified error type for API handlers.
pub enum AppError {
    /// The requested resource was not found.
    NotFound(String),
    /// The request was malformed.
    BadRequest(String),
    /// An internal server error occurred.
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

/// Start the API server on the given address.
///
/// This is a convenience function that creates the router and binds it.
pub async fn serve(
    addr: &std::net::SocketAddr,
    state: std::sync::Arc<AppState>,
) -> anyhow::Result<()> {
    let app = router::create_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("API server listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
