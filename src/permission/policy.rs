//! Permission policy definitions

use std::collections::HashMap;

use super::{Action, PermissionResult};

/// The level of permission granted or required
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionLevel {
    /// Action is always allowed
    Allow,
    /// Action is always denied
    Deny,
    /// User must be asked for confirmation
    Ask,
}

impl PermissionLevel {
    /// Check if this level allows an action
    pub fn is_allowed(&self) -> bool {
        matches!(self, PermissionLevel::Allow)
    }

    /// Check if this level denies an action
    pub fn is_denied(&self) -> bool {
        matches!(self, PermissionLevel::Deny)
    }

    /// Check if this level requires user confirmation
    pub fn requires_ask(&self) -> bool {
        matches!(self, PermissionLevel::Ask)
    }
}

/// A single permission policy rule
#[derive(Debug, Clone)]
pub struct Policy {
    /// Name of this policy
    pub name: String,
    /// Action pattern this policy applies to (supports glob)
    pub action_pattern: String,
    /// Permission level
    pub level: PermissionLevel,
    /// Optional reason for this policy
    pub reason: Option<String>,
    /// Whether this policy is active
    pub enabled: bool,
}

impl Policy {
    /// Create a new policy
    pub fn new(name: &str, action_pattern: &str, level: PermissionLevel) -> Self {
        Self {
            name: name.to_string(),
            action_pattern: action_pattern.to_string(),
            level,
            reason: None,
            enabled: true,
        }
    }

    /// Set a reason for this policy
    pub fn with_reason(mut self, reason: &str) -> Self {
        self.reason = Some(reason.to_string());
        self
    }

    /// Check if this policy matches a given action name
    pub fn matches(&self, action: &Action) -> bool {
        if !self.enabled {
            return false;
        }
        // Simple glob-style matching: support wildcard at end
        if self.action_pattern.ends_with('*') {
            let prefix = &self.action_pattern[..self.action_pattern.len() - 1];
            action.name.starts_with(prefix)
        } else {
            self.action_pattern == action.name
        }
    }
}

/// A collection of permission policies
#[derive(Debug, Clone)]
pub struct PolicySet {
    /// List of policies in evaluation order
    policies: Vec<Policy>,
    /// Default level when no policy matches
    default_level: PermissionLevel,
}

impl Default for PolicySet {
    fn default() -> Self {
        Self {
            policies: Vec::new(),
            default_level: PermissionLevel::Deny,
        }
    }
}

impl PolicySet {
    /// Create a new empty PolicySet
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a PolicySet with a custom default level
    pub fn with_default(default: PermissionLevel) -> Self {
        Self {
            policies: Vec::new(),
            default_level: default,
        }
    }

    /// Add a policy to the set (policies are evaluated in order, first match wins)
    pub fn add(&mut self, policy: Policy) {
        self.policies.push(policy);
    }

    /// Remove all policies with the given name
    pub fn remove(&mut self, name: &str) {
        self.policies.retain(|p| p.name != name);
    }

    /// Evaluate an action against the policy set
    ///
    /// Returns the first matching policy's level, or the default level if no policy matches.
    pub fn evaluate(&self, action: &Action) -> PermissionResult<PermissionLevel> {
        for policy in &self.policies {
            if policy.matches(action) {
                return Ok(policy.level);
            }
        }
        Ok(self.default_level)
    }

    /// Get all policies
    pub fn all(&self) -> &[Policy] {
        &self.policies
    }

    /// Get the number of policies
    pub fn len(&self) -> usize {
        self.policies.len()
    }

    /// Check if the policy set is empty
    pub fn is_empty(&self) -> bool {
        self.policies.is_empty()
    }

    /// Group policies by their level
    pub fn group_by_level(&self) -> HashMap<PermissionLevel, Vec<&Policy>> {
        let mut grouped: HashMap<PermissionLevel, Vec<&Policy>> = HashMap::new();
        for policy in &self.policies {
            grouped.entry(policy.level).or_default().push(policy);
        }
        grouped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_level_is_allowed() {
        assert!(PermissionLevel::Allow.is_allowed());
        assert!(!PermissionLevel::Deny.is_allowed());
        assert!(!PermissionLevel::Ask.is_allowed());
    }

    #[test]
    fn test_permission_level_is_denied() {
        assert!(PermissionLevel::Deny.is_denied());
        assert!(!PermissionLevel::Allow.is_denied());
    }

    #[test]
    fn test_permission_level_requires_ask() {
        assert!(PermissionLevel::Ask.requires_ask());
        assert!(!PermissionLevel::Allow.requires_ask());
    }

    #[test]
    fn test_policy_matches_exact() {
        let policy = Policy::new("allow-bash", "bash", PermissionLevel::Allow);
        let action = Action::new("bash");
        assert!(policy.matches(&action));
    }

    #[test]
    fn test_policy_matches_wildcard() {
        let policy = Policy::new("allow-file", "file_*", PermissionLevel::Allow);
        let action = Action::new("file_write");
        assert!(policy.matches(&action));
    }

    #[test]
    fn test_policy_disabled() {
        let mut policy = Policy::new("deny-bash", "bash", PermissionLevel::Deny);
        policy.enabled = false;
        let action = Action::new("bash");
        assert!(!policy.matches(&action));
    }

    #[test]
    fn test_policy_no_match() {
        let policy = Policy::new("allow-bash", "bash", PermissionLevel::Allow);
        let action = Action::new("file_read");
        assert!(!policy.matches(&action));
    }

    #[test]
    fn test_policy_set_default() {
        let set = PolicySet::new();
        let action = Action::new("unknown");
        assert_eq!(set.evaluate(&action).unwrap(), PermissionLevel::Deny);
    }

    #[test]
    fn test_policy_set_with_custom_default() {
        let set = PolicySet::with_default(PermissionLevel::Deny);
        let action = Action::new("unknown");
        assert_eq!(set.evaluate(&action).unwrap(), PermissionLevel::Deny);
    }

    #[test]
    fn test_policy_set_first_match_wins() {
        let mut set = PolicySet::new();
        set.add(Policy::new("allow-all", "*", PermissionLevel::Allow));
        set.add(Policy::new("deny-bash", "bash", PermissionLevel::Deny));
        let action = Action::new("bash");
        assert_eq!(set.evaluate(&action).unwrap(), PermissionLevel::Allow);
    }

    #[test]
    fn test_policy_set_remove() {
        let mut set = PolicySet::new();
        set.add(Policy::new("deny-bash", "bash", PermissionLevel::Deny));
        set.remove("deny-bash");
        assert!(set.is_empty());
    }

    #[test]
    fn test_policy_with_reason() {
        let policy = Policy::new("deny-bash", "bash", PermissionLevel::Deny)
            .with_reason("Shell access is restricted");
        assert_eq!(
            policy.reason,
            Some("Shell access is restricted".to_string())
        );
    }

    #[test]
    fn test_group_by_level() {
        let mut set = PolicySet::new();
        set.add(Policy::new("p1", "bash", PermissionLevel::Allow));
        set.add(Policy::new("p2", "file_*", PermissionLevel::Allow));
        set.add(Policy::new("p3", "rm", PermissionLevel::Deny));
        let grouped = set.group_by_level();
        assert_eq!(grouped[&PermissionLevel::Allow].len(), 2);
        assert_eq!(grouped[&PermissionLevel::Deny].len(), 1);
    }
}
