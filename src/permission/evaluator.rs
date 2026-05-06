//! Permission evaluation engine

use super::{policy::PolicySet, Action, PermissionError, PermissionLevel, PermissionResult};

/// Evaluates whether actions are permitted based on policies
#[derive(Debug, Clone)]
pub struct PermissionEvaluator {
    /// The policy set used for evaluation
    policies: PolicySet,
    /// Whether to log evaluation results
    verbose: bool,
}

impl Default for PermissionEvaluator {
    fn default() -> Self {
        Self {
            policies: PolicySet::new(),
            verbose: false,
        }
    }
}

impl PermissionEvaluator {
    /// Create a new PermissionEvaluator
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a PermissionEvaluator with a given policy set
    pub fn with_policies(policies: PolicySet) -> Self {
        Self {
            policies,
            verbose: false,
        }
    }

    /// Enable or disable verbose logging
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Evaluate whether an action is permitted
    ///
    /// Returns `Ok(())` if the action is allowed,
    /// or an error explaining why it was denied or needs confirmation.
    pub fn evaluate(&self, action: &Action) -> PermissionResult<()> {
        let level = self.policies.evaluate(action)?;

        if self.verbose {
            tracing::debug!(
                action = %action.name,
                resource = ?action.resource,
                level = ?level,
                "Permission evaluation"
            );
        }

        match level {
            PermissionLevel::Allow => Ok(()),
            PermissionLevel::Deny => Err(PermissionError::EvaluationFailed(format!(
                "Action '{}' is denied by policy",
                action.name
            ))),
            PermissionLevel::Ask => Err(PermissionError::RequiresConfirmation(format!(
                "Action '{}' requires user confirmation",
                action.name
            ))),
        }
    }

    /// Check if an action is allowed (returns bool, no error)
    pub fn is_allowed(&self, action: &Action) -> bool {
        self.evaluate(action).is_ok()
    }

    /// Check if an action requires user confirmation
    pub fn requires_confirmation(&self, action: &Action) -> bool {
        matches!(self.policies.evaluate(action), Ok(PermissionLevel::Ask))
    }

    /// Get a reference to the underlying policy set
    pub fn policies(&self) -> &PolicySet {
        &self.policies
    }

    /// Get a mutable reference to the underlying policy set
    pub fn policies_mut(&mut self) -> &mut PolicySet {
        &mut self.policies
    }

    /// Add a list of default policies (allow read-only, ask for others)
    pub fn add_default_policies(&mut self) {
        use super::Policy;

        let defaults = vec![
            Policy::new("allow-read", "file_read", PermissionLevel::Allow)
                .with_reason("Reading files is safe"),
            Policy::new("allow-grep", "grep", PermissionLevel::Allow)
                .with_reason("Content search is read-only"),
            Policy::new("allow-glob", "glob", PermissionLevel::Allow)
                .with_reason("File search is read-only"),
            Policy::new("allow-web-fetch", "web_fetch", PermissionLevel::Allow)
                .with_reason("Web fetching is read-only"),
            Policy::new("ask-bash", "bash", PermissionLevel::Ask)
                .with_reason("Shell commands may modify the system"),
            Policy::new("ask-write", "file_write", PermissionLevel::Ask)
                .with_reason("Writing files modifies the project"),
            Policy::new("ask-edit", "file_edit", PermissionLevel::Ask)
                .with_reason("Editing files modifies the project"),
        ];

        for policy in defaults {
            self.policies.add(policy);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Policy;
    use super::*;

    #[test]
    fn test_evaluator_default_allows_nothing() {
        let evaluator = PermissionEvaluator::new();
        let action = Action::new("bash");
        assert!(!evaluator.is_allowed(&action));
    }

    #[test]
    fn test_evaluator_with_policies() {
        let mut policy_set = PolicySet::new();
        policy_set.add(Policy::new("allow", "bash", PermissionLevel::Allow));
        let evaluator = PermissionEvaluator::with_policies(policy_set);
        let action = Action::new("bash");
        assert!(evaluator.is_allowed(&action));
    }

    #[test]
    fn test_evaluator_requires_confirmation() {
        let mut policy_set = PolicySet::new();
        policy_set.add(Policy::new("ask", "bash", PermissionLevel::Ask));
        let evaluator = PermissionEvaluator::with_policies(policy_set);
        let action = Action::new("bash");
        assert!(evaluator.requires_confirmation(&action));
    }

    #[test]
    fn test_evaluator_denied() {
        let mut policy_set = PolicySet::new();
        policy_set.add(Policy::new("deny", "bash", PermissionLevel::Deny));
        let evaluator = PermissionEvaluator::with_policies(policy_set);
        let action = Action::new("bash");
        assert!(!evaluator.is_allowed(&action));
        assert!(evaluator.evaluate(&action).is_err());
    }

    #[test]
    fn test_default_policies() {
        let mut evaluator = PermissionEvaluator::new();
        evaluator.add_default_policies();
        assert!(evaluator.is_allowed(&Action::new("file_read")));
        assert!(evaluator.requires_confirmation(&Action::new("bash")));
    }

    #[test]
    fn test_verbose_evaluator() {
        let evaluator = PermissionEvaluator::new().with_verbose(true);
        let action = Action::new("test");
        // Verbose just logs, doesn't change outcome
        assert!(!evaluator.is_allowed(&action));
    }
}
