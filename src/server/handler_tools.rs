//! Tools API handlers
//!
//! Provides endpoints for listing available tools and executing them
//! by name.  All tools are registered in [`crate::tool::ToolRegistry`]
//! which is shared via [`super::AppState`].

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::tool::ToolResult;

use super::AppError;
use super::AppState;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Public tool info returned by the list endpoint.
#[derive(Debug, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,
}

/// Request body for executing a tool.
#[derive(Debug, Deserialize)]
pub struct ExecuteRequest {
    /// JSON arguments to pass to the tool.
    pub args: serde_json::Value,
}

/// Response from executing a tool.
#[derive(Debug, Serialize)]
pub struct ExecuteResponse {
    pub success: bool,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl From<ToolResult> for ExecuteResponse {
    fn from(r: ToolResult) -> Self {
        Self {
            success: r.success,
            output: r.output,
            error: r.error,
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /api/tools` -- list all registered tools with their names,
/// descriptions, and input schemas.
pub async fn list_tools(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<ToolInfo>> {
    let defs = state.tool_registry.tool_defs();
    let tools: Vec<ToolInfo> = defs
        .into_iter()
        .map(|d| ToolInfo {
            name: d.name,
            description: d.description,
            schema: d.input_schema,
        })
        .collect();
    Json(tools)
}

/// `POST /api/tools/{name}/exec` -- execute a tool by name.
///
/// The request body should contain the tool arguments as a JSON object.
pub async fn execute_tool(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<ExecuteRequest>,
) -> Result<Json<ExecuteResponse>, AppError> {
    let result = state.tool_registry.execute(&name, body.args).await;
    Ok(Json(ExecuteResponse::from(result)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_tools_handler() {
        let state = Arc::new(AppState::new(
            crate::session::manager::SessionManager::new(),
            Arc::new(crate::tool::ToolRegistry::default()),
            Box::new(crate::ai::openai::OpenAIProvider::new(
                "test".into(),
                "https://api.openai.com/v1".into(),
                "gpt-4o".into(),
            )),
        ));

        let result = list_tools(State(state)).await;
        assert!(!result.0.is_empty(), "list_tools should return tools");
    }
}
