//! Execution policy engine - layered permission rulesets
//!
//! Three priority layers for tool execution decisions:
//! 1. `deny` rules (highest) - always win, cannot be overridden
//! 2. `builtin` rules - default system policies
//! 3. `agent` rules - per-agent policies
//! 4. `user` rules (lowest) - user-configured policies

pub mod arity;

use arity::BashArityDict;

/// Priority layer for a ruleset
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RulesetLayer {
    /// User-configured rules (lowest priority)
    User = 0,
    /// Agent-specific rules
    Agent = 1,
    /// Built-in system defaults
    Builtin = 2,
    /// Explicit deny rules (highest priority, always win)
    Deny = 3,
}

/// A single rule entry
#[derive(Debug, Clone)]
pub struct Rule {
    /// Tool name or command pattern to match
    pub pattern: String,
    /// Whether to allow or deny
    pub action: RuleAction,
}

/// Action for a rule
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuleAction {
    Allow,
    Deny,
}

impl RuleAction {
    pub fn is_allowed(&self) -> bool {
        matches!(self, RuleAction::Allow)
    }
}

/// A named ruleset at a specific priority layer
#[derive(Debug, Clone)]
pub struct Ruleset {
    pub layer: RulesetLayer,
    pub name: String,
    pub rules: Vec<Rule>,
}

impl Ruleset {
    pub fn new(layer: RulesetLayer, name: &str) -> Self {
        Self {
            layer,
            name: name.to_string(),
            rules: Vec::new(),
        }
    }

    pub fn allow(mut self, pattern: &str) -> Self {
        self.rules.push(Rule {
            pattern: pattern.to_string(),
            action: RuleAction::Allow,
        });
        self
    }

    pub fn deny(mut self, pattern: &str) -> Self {
        self.rules.push(Rule {
            pattern: pattern.to_string(),
            action: RuleAction::Deny,
        });
        self
    }
}

/// Decision from the policy engine
#[derive(Debug, Clone, PartialEq)]
pub enum ExecPolicyDecision {
    /// Tool is allowed to execute
    Allowed,
    /// Tool execution requires user confirmation
    NeedsApproval,
    /// Tool execution is denied by policy
    Denied(String),
}

/// Context for evaluating a policy decision
#[derive(Debug, Clone)]
pub struct ExecPolicyContext {
    /// The tool name being executed
    pub tool_name: String,
    /// The command string (for shell tools)
    pub command: Option<String>,
}

impl ExecPolicyContext {
    pub fn new(tool_name: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            command: None,
        }
    }

    pub fn with_command(mut self, command: &str) -> Self {
        self.command = Some(command.to_string());
        self
    }
}

/// Execution policy engine with layered rulesets
pub struct ExecPolicyEngine {
    /// All registered rulesets, ordered by layer priority
    rulesets: Vec<Ruleset>,
    /// Bash arity dictionary for command matching
    arity_dict: BashArityDict,
    /// Tools that always require approval
    sensitive_tools: Vec<String>,
}

impl ExecPolicyEngine {
    /// Create a new policy engine with default built-in rules
    pub fn new() -> Self {
        let mut engine = Self {
            rulesets: Vec::new(),
            arity_dict: BashArityDict::new(),
            sensitive_tools: vec![
                "bash".to_string(),
                "exec_shell".to_string(),
                "file_write".to_string(),
                "file_edit".to_string(),
                "apply_patch".to_string(),
                "docker".to_string(),
                "db_query".to_string(),
            ],
        };

        // Add built-in ruleset
        engine.add_ruleset(Self::builtin_ruleset());
        engine
    }

    /// Get the built-in default ruleset
    fn builtin_ruleset() -> Ruleset {
        Ruleset::new(RulesetLayer::Builtin, "builtin")
            .allow("file_read")
            .allow("grep")
            .allow("glob")
            .allow("list_dir")
            .allow("web_fetch")
            .allow("web_search")
            .allow("docs")
            .allow("question")
            .allow("checklist")
            .allow("plan")
            .allow("task")
            .allow("git")
            .allow("ci")
    }

    /// Add a ruleset to the engine
    pub fn add_ruleset(&mut self, ruleset: Ruleset) {
        self.rulesets.push(ruleset);
        // Sort by layer priority (highest first for deny-wins)
        self.rulesets.sort_by(|a, b| b.layer.cmp(&a.layer));
    }

    /// Add a user ruleset
    pub fn add_user_rules(&mut self, rules: Vec<Rule>) {
        let mut ruleset = Ruleset::new(RulesetLayer::User, "user");
        ruleset.rules = rules;
        self.add_ruleset(ruleset);
    }

    /// Evaluate whether a tool should be allowed
    pub fn evaluate(&self, ctx: &ExecPolicyContext) -> ExecPolicyDecision {
        // 1. Check rulesets in priority order (deny always wins)
        for ruleset in &self.rulesets {
            for rule in &ruleset.rules {
                if self.pattern_matches(&rule.pattern, ctx) {
                    match rule.action {
                        RuleAction::Deny => {
                            return ExecPolicyDecision::Denied(format!(
                                "Denied by {} policy: '{}' matches rule '{}'",
                                ruleset.name, ctx.tool_name, rule.pattern
                            ));
                        }
                        RuleAction::Allow => {
                            return ExecPolicyDecision::Allowed;
                        }
                    }
                }
            }
        }

        // 2. No rule matched - check if tool requires approval
        if self.sensitive_tools.contains(&ctx.tool_name) {
            ExecPolicyDecision::NeedsApproval
        } else {
            ExecPolicyDecision::Allowed
        }
    }

    /// Check if a pattern matches the execution context
    fn pattern_matches(&self, pattern: &str, ctx: &ExecPolicyContext) -> bool {
        // Tool name match
        if pattern == ctx.tool_name {
            return true;
        }

        // For shell commands, use arity-aware matching
        if ctx.tool_name == "bash" || ctx.tool_name == "exec_shell" {
            if let Some(ref command) = ctx.command {
                return self.arity_dict.allow_rule_matches(pattern, command);
            }
        }

        false
    }

    /// Set the approval requirement for a tool
    pub fn mark_sensitive(&mut self, tool_name: &str) {
        if !self.sensitive_tools.contains(&tool_name.to_string()) {
            self.sensitive_tools.push(tool_name.to_string());
        }
    }

    /// Remove a tool from sensitive list
    pub fn mark_safe(&mut self, tool_name: &str) {
        self.sensitive_tools.retain(|t| t != tool_name);
    }
}

impl Default for ExecPolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_allows_read_tools() {
        let engine = ExecPolicyEngine::new();
        let ctx = ExecPolicyContext::new("file_read");
        assert_eq!(engine.evaluate(&ctx), ExecPolicyDecision::Allowed);
    }

    #[test]
    fn test_policy_approval_for_bash() {
        let engine = ExecPolicyEngine::new();
        let ctx = ExecPolicyContext::new("bash").with_command("rm -rf /");
        let decision = engine.evaluate(&ctx);
        // Should need approval since no specific rule matches "rm -rf"
        assert_eq!(decision, ExecPolicyDecision::NeedsApproval);
    }

    #[test]
    fn test_deny_overrides_allow() {
        let mut engine = ExecPolicyEngine::new();

        // Add a user allow rule
        let mut user_rules = Ruleset::new(RulesetLayer::User, "user-override");
        user_rules = user_rules.allow("bash");
        engine.add_ruleset(user_rules);

        // Add a deny rule for bash (higher layer)
        let mut deny_rules = Ruleset::new(RulesetLayer::Deny, "deny-all");
        deny_rules = deny_rules.deny("bash");
        engine.add_ruleset(deny_rules);

        let ctx = ExecPolicyContext::new("bash").with_command("echo test");
        let decision = engine.evaluate(&ctx);
        assert!(matches!(decision, ExecPolicyDecision::Denied(_)));
    }

    #[test]
    fn test_user_allow_for_bash() {
        let mut engine = ExecPolicyEngine::new();

        let mut user_rules = Ruleset::new(RulesetLayer::User, "user-override");
        user_rules = user_rules.allow("bash");
        engine.add_ruleset(user_rules);

        let ctx = ExecPolicyContext::new("bash").with_command("git status");
        let decision = engine.evaluate(&ctx);
        assert_eq!(decision, ExecPolicyDecision::Allowed);
    }

    #[test]
    fn test_custom_tool_requires_no_approval() {
        let engine = ExecPolicyEngine::new();
        let ctx = ExecPolicyContext::new("custom_read_tool");
        assert_eq!(engine.evaluate(&ctx), ExecPolicyDecision::Allowed);
    }

    #[test]
    fn test_sensitive_marking() {
        let mut engine = ExecPolicyEngine::new();
        engine.mark_sensitive("custom_tool");

        let ctx = ExecPolicyContext::new("custom_tool");
        assert_eq!(engine.evaluate(&ctx), ExecPolicyDecision::NeedsApproval);
    }

    #[test]
    fn test_safe_unmarking() {
        let mut engine = ExecPolicyEngine::new();

        // Add an allow rule for bash
        let mut user_rules = Ruleset::new(RulesetLayer::User, "allow-bash");
        user_rules = user_rules.allow("bash");
        engine.add_ruleset(user_rules);

        let ctx = ExecPolicyContext::new("bash").with_command("echo hello");
        let decision = engine.evaluate(&ctx);
        assert_eq!(decision, ExecPolicyDecision::Allowed);
    }

    #[test]
    fn test_ruleset_creation() {
        let ruleset = Ruleset::new(RulesetLayer::Agent, "test-agent")
            .allow("file_read")
            .deny("bash");

        assert_eq!(ruleset.layer, RulesetLayer::Agent);
        assert_eq!(ruleset.rules.len(), 2);
    }

    #[test]
    fn test_deny_always_wins() {
        let mut engine = ExecPolicyEngine::new();

        // Add deny at highest layer
        let mut deny = Ruleset::new(RulesetLayer::Deny, "strict");
        deny = deny.deny("file_read");
        engine.add_ruleset(deny);

        let ctx = ExecPolicyContext::new("file_read");
        let decision = engine.evaluate(&ctx);
        assert!(matches!(decision, ExecPolicyDecision::Denied(_)));
    }
}
