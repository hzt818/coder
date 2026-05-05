//! Axum router setup
//!
//! Configures all HTTP routes for the API server.

use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;

use super::AppState;

/// Create the API router with all routes and shared state.
///
/// # Routes
///
/// * `GET /api/sessions` -- list sessions
/// * `POST /api/sessions` -- create session
/// * `GET /api/sessions/:id` -- get session
/// * `POST /api/sessions/:id/chat` -- streaming chat (SSE)
/// * `GET /api/tools` -- list tools
/// * `POST /api/tools/:name/exec` -- execute tool
/// * `GET /api/ws` -- WebSocket upgrade
/// * `GET /api/health` -- health check
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Session routes
        .route(
            "/api/sessions",
            get(super::handler_session::list_sessions)
                .post(super::handler_session::create_session),
        )
        .route(
            "/api/sessions/{id}",
            get(super::handler_session::get_session),
        )
        .route(
            "/api/sessions/{id}/chat",
            post(super::handler_session::chat_stream),
        )
        // Tool routes
        .route(
            "/api/tools",
            get(super::handler_tools::list_tools),
        )
        .route(
            "/api/tools/{name}/exec",
            post(super::handler_tools::execute_tool),
        )
        // WebSocket
        .route("/api/ws", get(super::ws::ws_handler))
        // Health check
        .route("/api/health", get(health_check))
        .with_state(state)
}

/// Health check endpoint -- returns `OK` when the server is alive.
async fn health_check() -> &'static str {
    "OK"
}
