//! Supervisor - manages subagent lifecycle and collects results

use super::spawn::{SpawnConfig, SubagentHandle, spawn_subagent};
use crate::ai::{Message, Provider};
use crate::tool::ToolRegistry;
use std::sync::Arc;

/// Result from a completed subagent
#[derive(Debug, Clone)]
pub struct SubagentResult {
    /// Subagent identifier
    pub id: String,
    /// Whether execution succeeded
    pub success: bool,
    /// Output text (on success)
    pub output: Option<String>,
    /// Error message (on failure)
    pub error: Option<String>,
}

impl SubagentResult {
    fn ok(id: String, output: String) -> Self {
        Self {
            id,
            success: true,
            output: Some(output),
            error: None,
        }
    }

    fn err(id: String, error: String) -> Self {
        Self {
            id,
            success: false,
            output: None,
            error: Some(error),
        }
    }
}

/// Configuration for the supervisor
#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    /// Maximum number of concurrent subagents
    pub max_concurrent: usize,
    /// Whether to stop on first failure
    pub fail_fast: bool,
    /// Spawn configuration for each subagent
    pub spawn_config: SpawnConfig,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 4,
            fail_fast: false,
            spawn_config: SpawnConfig::default(),
        }
    }
}

/// Supervisor manages the lifecycle of multiple subagents.
///
/// It spawns subagents, collects their results, and provides
/// aggregated outcome information.
pub struct Supervisor {
    /// Active subagent handles
    handles: Vec<(String, SubagentHandle)>,
    /// Collected results from completed subagents
    results: Vec<SubagentResult>,
    /// Configuration
    config: SupervisorConfig,
}

impl Supervisor {
    /// Create a new supervisor with the given configuration
    pub fn new(config: SupervisorConfig) -> Self {
        Self {
            handles: Vec::new(),
            results: Vec::new(),
            config,
        }
    }

    /// Spawn a new subagent and register it with the supervisor.
    ///
    /// Returns the subagent ID. If the maximum concurrent limit
    /// would be exceeded, the call blocks until a slot is available.
    pub fn spawn(
        &mut self,
        provider: Arc<dyn Provider>,
        tools: Arc<ToolRegistry>,
        messages: Vec<Message>,
    ) -> String {
        // Generate the subagent
        let handle = spawn_subagent(
            provider,
            tools,
            messages,
            self.config.spawn_config.clone(),
        );
        let id = handle.id.clone();
        self.handles.push((id.clone(), handle));
        id
    }

    /// Wait for all spawned subagents to complete and collect results.
    ///
    /// Clears the internal handles after collection.
    pub async fn collect(&mut self) -> Vec<SubagentResult> {
        let mut results = Vec::new();

        // Take all handles to avoid borrow issues
        let handles = std::mem::take(&mut self.handles);

        for (id, handle) in handles {
            match handle.join().await {
                Ok(output) => {
                    let result = SubagentResult::ok(id, output);
                    if self.config.fail_fast && !result.success {
                        self.results.push(result.clone());
                        results.push(result);
                        break;
                    }
                    self.results.push(result.clone());
                    results.push(result);
                }
                Err(e) => {
                    let result = SubagentResult::err(id, e.to_string());
                    self.results.push(result.clone());
                    results.push(result);
                    if self.config.fail_fast {
                        break;
                    }
                }
            }
        }

        results
    }

    /// Get all collected results so far
    pub fn results(&self) -> &[SubagentResult] {
        &self.results
    }

    /// Get the number of successful results
    pub fn success_count(&self) -> usize {
        self.results.iter().filter(|r| r.success).count()
    }

    /// Get the number of failed results
    pub fn failure_count(&self) -> usize {
        self.results.iter().filter(|r| !r.success).count()
    }

    /// Check if all collected results are successful
    pub fn all_succeeded(&self) -> bool {
        !self.results.is_empty() && self.results.iter().all(|r| r.success)
    }

    /// Check if any results have been collected
    pub fn has_results(&self) -> bool {
        !self.results.is_empty()
    }

    /// Number of pending (not yet collected) subagents
    pub fn pending_count(&self) -> usize {
        self.handles.len()
    }

    /// Abort all pending subagents
    pub fn abort_all(&mut self) {
        for (_id, handle) in self.handles.drain(..) {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supervisor_config_default() {
        let config = SupervisorConfig::default();
        assert_eq!(config.max_concurrent, 4);
        assert!(!config.fail_fast);
    }

    #[test]
    fn test_supervisor_creation() {
        let supervisor = Supervisor::new(SupervisorConfig::default());
        assert_eq!(supervisor.pending_count(), 0);
        assert!(!supervisor.has_results());
    }

    #[test]
    fn test_subagent_result_ok() {
        let result = SubagentResult::ok("test-1".to_string(), "success".to_string());
        assert!(result.success);
        assert_eq!(result.output.unwrap(), "success");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_subagent_result_err() {
        let result = SubagentResult::err("test-1".to_string(), "failure".to_string());
        assert!(!result.success);
        assert!(result.output.is_none());
        assert_eq!(result.error.unwrap(), "failure");
    }

    #[test]
    fn test_supervisor_abort_all() {
        let mut supervisor = Supervisor::new(SupervisorConfig::default());
        // No handles to abort is a no-op
        supervisor.abort_all();
        assert_eq!(supervisor.pending_count(), 0);
    }
}
