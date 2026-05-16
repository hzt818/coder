//! Team-Subagent Bridge - connects Team coordination with Subagent execution
//!
//! Provides:
//! - Subagent task delegation from team
//! - Shared context propagation
//! - Result aggregation

use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::OnceLock;

static TEAM_SUBAGENT_BRIDGE: OnceLock<RwLock<TeamSubagentBridgeState>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct SubagentTask {
    pub id: String,
    pub role: String,
    pub description: String,
    pub status: TaskStatus,
    pub result: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

pub struct TeamSubagentBridgeState {
    active_tasks: HashMap<String, SubagentTask>,
    team_context: Option<String>,
}

impl TeamSubagentBridgeState {
    pub fn new() -> Self {
        Self {
            active_tasks: HashMap::new(),
            team_context: None,
        }
    }
}

fn get_state() -> &'static RwLock<TeamSubagentBridgeState> {
    TEAM_SUBAGENT_BRIDGE.get_or_init(|| RwLock::new(TeamSubagentBridgeState::new()))
}

pub fn init() {
    let _ = get_state();
    tracing::info!("Team-Subagent bridge initialized");
}

pub fn set_team_context(context: &str) {
    if let Ok(mut state) = get_state().write() {
        state.team_context = Some(context.to_string());
    }
}

pub fn get_team_context() -> Option<String> {
    if let Ok(state) = get_state().read() {
        return state.team_context.clone();
    }
    None
}

pub fn create_task(role: &str, description: &str) -> String {
    let task_id = uuid::Uuid::new_v4().to_string();

    if let Ok(mut state) = get_state().write() {
        state.active_tasks.insert(
            task_id.clone(),
            SubagentTask {
                id: task_id.clone(),
                role: role.to_string(),
                description: description.to_string(),
                status: TaskStatus::Pending,
                result: None,
            },
        );
        tracing::debug!("Created task {} for role '{}'", task_id, role);
    }

    task_id
}

pub fn start_task(task_id: &str) -> bool {
    if let Ok(mut state) = get_state().write() {
        if let Some(task) = state.active_tasks.get_mut(task_id) {
            task.status = TaskStatus::Running;
            tracing::debug!("Started task {}", task_id);
            return true;
        }
    }
    false
}

pub fn complete_task(task_id: &str, result: &str) -> bool {
    if let Ok(mut state) = get_state().write() {
        if let Some(task) = state.active_tasks.get_mut(task_id) {
            task.status = TaskStatus::Completed;
            task.result = Some(result.to_string());
            tracing::debug!("Completed task {}", task_id);
            return true;
        }
    }
    false
}

pub fn fail_task(task_id: &str, error: &str) -> bool {
    if let Ok(mut state) = get_state().write() {
        if let Some(task) = state.active_tasks.get_mut(task_id) {
            task.status = TaskStatus::Failed;
            task.result = Some(format!("Error: {}", error));
            tracing::warn!("Task {} failed: {}", task_id, error);
            return true;
        }
    }
    false
}

pub fn get_task(task_id: &str) -> Option<SubagentTask> {
    if let Ok(state) = get_state().read() {
        return state.active_tasks.get(task_id).cloned();
    }
    None
}

pub fn get_all_tasks() -> Vec<SubagentTask> {
    if let Ok(state) = get_state().read() {
        return state.active_tasks.values().cloned().collect();
    }
    Vec::new()
}

pub fn get_pending_tasks() -> Vec<SubagentTask> {
    get_all_tasks()
        .into_iter()
        .filter(|t| t.status == TaskStatus::Pending)
        .collect()
}

pub fn get_completed_tasks() -> Vec<SubagentTask> {
    get_all_tasks()
        .into_iter()
        .filter(|t| t.status == TaskStatus::Completed)
        .collect()
}

pub fn aggregate_results() -> String {
    let completed = get_completed_tasks();
    if completed.is_empty() {
        return "No completed tasks yet.".to_string();
    }

    let mut summary = String::from("## Task Results Summary\n\n");

    for task in &completed {
        summary.push_str(&format!("### Task: {} ({})\n", task.role, task.id));
        summary.push_str(&format!("{}\n\n", task.description));
        if let Some(result) = &task.result {
            summary.push_str(&format!("Result:\n{}\n\n", result));
        }
    }

    summary
}

pub fn clear_completed_tasks() {
    if let Ok(mut state) = get_state().write() {
        state.active_tasks.retain(|_, t| t.status != TaskStatus::Completed);
        tracing::debug!("Cleared completed tasks");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        init();
        // Verify bridge is operational: task creation works
        let id = create_task("test", "verify bridge init");
        let task = get_task(&id);
        assert!(task.is_some());
        assert_eq!(task.unwrap().status, TaskStatus::Pending);
    }

    #[test]
    fn test_task_lifecycle() {
        init();

        let task_id = create_task("coder", "Implement feature X");
        assert_eq!(get_task(&task_id).unwrap().status, TaskStatus::Pending);

        assert!(start_task(&task_id));
        assert_eq!(get_task(&task_id).unwrap().status, TaskStatus::Running);

        assert!(complete_task(&task_id, "Done!"));
        assert_eq!(get_task(&task_id).unwrap().status, TaskStatus::Completed);
    }

    #[test]
    fn test_team_context() {
        init();
        set_team_context("Working on Rust async");
        assert_eq!(get_team_context(), Some("Working on Rust async".to_string()));
    }

    #[test]
    fn test_aggregate_results() {
        init();
        let task_id = create_task("tester", "Run tests");
        start_task(&task_id);
        complete_task(&task_id, "All tests passed");

        let summary = aggregate_results();
        assert!(summary.contains("tester"));
        assert!(summary.contains("All tests passed"));
    }
}
