//! Per-domain network policy for outbound network calls (SSRF protection).
//!
//! Three pieces:
//! 1. [`Decision`] — Allow / Deny / Prompt.
//! 2. [`NetworkPolicy`] — list of allow/deny hostnames + default decision,
//!    with **deny-wins precedence**.
//! 3. [`NetworkAuditor`] — appends one line per outbound call to the audit log.
//!
//! # Host-matching rules
//!
//! * **Exact match** — `api.example.com` matches only `api.example.com` (case-insensitive).
//! * **Subdomain match** — an entry starting with `.` (e.g. `.example.com`) matches
//!   any subdomain (`api.example.com`) but **not** the apex `example.com`.
//! * Deny always wins over allow.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// What the policy decided about an outbound network call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
    Prompt,
}

impl Decision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "Allow",
            Self::Deny => "Deny",
            Self::Prompt => "Prompt",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "allow" => Self::Allow,
            "deny" | "block" => Self::Deny,
            _ => Self::Prompt,
        }
    }
}

/// Wire-format for TOML/JSON serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionToml {
    Allow,
    Deny,
    Prompt,
}

impl From<DecisionToml> for Decision {
    fn from(v: DecisionToml) -> Self {
        match v {
            DecisionToml::Allow => Self::Allow,
            DecisionToml::Deny => Self::Deny,
            DecisionToml::Prompt => Self::Prompt,
        }
    }
}

impl From<Decision> for DecisionToml {
    fn from(v: Decision) -> Self {
        match v {
            Decision::Allow => Self::Allow,
            Decision::Deny => Self::Deny,
            Decision::Prompt => Self::Prompt,
        }
    }
}

/// Per-domain allow/deny list with default fallback. Deny-wins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    #[serde(default = "default_decision")]
    pub default: DecisionToml,
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default = "default_audit")]
    pub audit: bool,
}

fn default_decision() -> DecisionToml {
    DecisionToml::Prompt
}
fn default_audit() -> bool {
    true
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            default: DecisionToml::Prompt,
            allow: Vec::new(),
            deny: Vec::new(),
            audit: true,
        }
    }
}

impl NetworkPolicy {
    /// Decide for a single outbound call to `host`. Deny-wins.
    pub fn decide(&self, host: &str) -> Decision {
        let normalized = normalize_host(host);
        if normalized.is_empty() {
            return self.default.into();
        }
        if self.deny.iter().any(|e| host_matches(e, &normalized)) {
            return Decision::Deny;
        }
        if self.allow.iter().any(|e| host_matches(e, &normalized)) {
            return Decision::Allow;
        }
        self.default.into()
    }

    pub fn add_allow(&mut self, host: &str) {
        let normalized = normalize_host(host);
        if normalized.is_empty() {
            return;
        }
        if !self.allow.iter().any(|e| normalize_host(e) == normalized) {
            self.allow.push(normalized);
        }
    }

    pub fn audit_enabled(&self) -> bool {
        self.audit
    }
}

fn normalize_host(host: &str) -> String {
    let trimmed = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if let Some(rest) = trimmed.strip_prefix("*.") {
        format!(".{rest}")
    } else {
        trimmed
    }
}

fn host_matches(entry: &str, normalized_host: &str) -> bool {
    let entry_norm = normalize_host(entry);
    if let Some(suffix) = entry_norm.strip_prefix('.') {
        if suffix.is_empty() {
            return false;
        }
        normalized_host.ends_with(&format!(".{suffix}"))
    } else {
        entry_norm == normalized_host
    }
}

/// Best-effort writer for the network audit log.
#[derive(Debug, Clone)]
pub struct NetworkAuditor {
    path: PathBuf,
    enabled: bool,
}

impl NetworkAuditor {
    pub fn new(path: PathBuf, enabled: bool) -> Self {
        Self { path, enabled }
    }

    pub fn default_path(enabled: bool) -> Option<Self> {
        let home = dirs::home_dir()?;
        Some(Self::new(home.join(".coder").join("audit.log"), enabled))
    }

    pub fn record(&self, host: &str, tool: &str, decision_label: &str) {
        if !self.enabled {
            return;
        }
        if let Err(err) = self.try_record(host, tool, decision_label) {
            eprintln!("network audit write failed: {err}");
        }
    }

    fn try_record(&self, host: &str, tool: &str, decision_label: &str) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(
            file,
            "{} network {} {} {}",
            chrono::Utc::now().to_rfc3339(),
            sanitize_field(host),
            sanitize_field(tool),
            decision_label
        )
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn sanitize_field(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect()
}

/// In-process session cache for "approve once" decisions.
#[derive(Debug, Default, Clone)]
pub struct NetworkSessionCache {
    inner: Arc<Mutex<NetworkSessionCacheInner>>,
}

#[derive(Debug, Default)]
struct NetworkSessionCacheInner {
    approved: HashSet<String>,
    denied: HashSet<String>,
}

impl NetworkSessionCache {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn is_approved(&self, host: &str) -> bool {
        self.inner
            .lock()
            .map(|g| g.approved.contains(&normalize_host(host)))
            .unwrap_or(false)
    }
    pub fn is_denied(&self, host: &str) -> bool {
        self.inner
            .lock()
            .map(|g| g.denied.contains(&normalize_host(host)))
            .unwrap_or(false)
    }
    pub fn approve(&self, host: &str) {
        let n = normalize_host(host);
        if let Ok(mut g) = self.inner.lock() {
            g.denied.remove(&n);
            g.approved.insert(n);
        }
    }
    pub fn deny(&self, host: &str) {
        let n = normalize_host(host);
        if let Ok(mut g) = self.inner.lock() {
            g.approved.remove(&n);
            g.denied.insert(n);
        }
    }
}

/// Error returned when a call is blocked by policy.
#[derive(Debug, Clone, Error)]
#[error("network call to '{0}' blocked by network policy")]
pub struct NetworkDenied(pub String);

impl NetworkDenied {
    pub fn host(&self) -> &str {
        &self.0
    }
}

/// Bundles policy + session cache + auditor for one-stop network decision.
#[derive(Debug, Clone)]
pub struct NetworkPolicyDecider {
    policy: NetworkPolicy,
    cache: NetworkSessionCache,
    auditor: Option<NetworkAuditor>,
}

impl NetworkPolicyDecider {
    pub fn new(policy: NetworkPolicy, auditor: Option<NetworkAuditor>) -> Self {
        Self {
            policy,
            cache: NetworkSessionCache::new(),
            auditor,
        }
    }

    pub fn with_default_audit(policy: NetworkPolicy) -> Self {
        let auditor = if policy.audit_enabled() {
            NetworkAuditor::default_path(true)
        } else {
            None
        };
        Self::new(policy, auditor)
    }

    pub fn policy(&self) -> &NetworkPolicy {
        &self.policy
    }
    pub fn cache(&self) -> &NetworkSessionCache {
        &self.cache
    }

    pub fn evaluate(&self, host: &str, tool: &str) -> Decision {
        let normalized = normalize_host(host);
        if normalized.is_empty() {
            return self.policy.default.into();
        }
        if self.cache.is_denied(&normalized) {
            self.audit_record(&normalized, tool, "Deny");
            return Decision::Deny;
        }
        if self.cache.is_approved(&normalized) {
            self.audit_record(&normalized, tool, "Allow");
            return Decision::Allow;
        }
        let d = self.policy.decide(&normalized);
        match d {
            Decision::Allow => self.audit_record(&normalized, tool, "Allow"),
            Decision::Deny => self.audit_record(&normalized, tool, "Deny"),
            Decision::Prompt => {}
        }
        d
    }

    pub fn approve_session(&self, host: &str, tool: &str) {
        self.cache.approve(host);
        self.audit_record(host, tool, "Prompt-Approved");
    }
    pub fn deny_session(&self, host: &str, tool: &str) {
        self.cache.deny(host);
        self.audit_record(host, tool, "Prompt-Denied");
    }

    pub fn approve_persistent(&mut self, host: &str, tool: &str) -> &NetworkPolicy {
        self.policy.add_allow(host);
        self.cache.approve(host);
        self.audit_record(host, tool, "Prompt-Approved");
        &self.policy
    }

    fn audit_record(&self, host: &str, tool: &str, label: &str) {
        if let Some(a) = self.auditor.as_ref() {
            a.record(host, tool, label);
        }
    }
}

/// Extract the host portion of a URL.
pub fn host_from_url(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url.trim()).ok()?;
    parsed.host_str().map(str::to_ascii_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn mk(default: Decision, allow: &[&str], deny: &[&str]) -> NetworkPolicy {
        NetworkPolicy {
            default: default.into(),
            allow: allow.iter().map(|s| s.to_string()).collect(),
            deny: deny.iter().map(|s| s.to_string()).collect(),
            audit: false,
        }
    }

    #[test]
    fn exact_match_in_allow() {
        let p = mk(Decision::Deny, &["api.example.com"], &[]);
        assert_eq!(p.decide("api.example.com"), Decision::Allow);
    }
    #[test]
    fn unknown_host_returns_default() {
        assert_eq!(
            mk(Decision::Deny, &["api.example.com"], &[]).decide("evil.example.com"),
            Decision::Deny
        );
    }
    #[test]
    fn deny_wins() {
        let p = mk(Decision::Prompt, &["api.example.com"], &["api.example.com"]);
        assert_eq!(p.decide("api.example.com"), Decision::Deny);
    }
    #[test]
    fn deny_wins_subdomain() {
        assert_eq!(
            mk(Decision::Allow, &["api.example.com"], &[".example.com"]).decide("api.example.com"),
            Decision::Deny
        );
    }
    #[test]
    fn subdomain_wildcard() {
        let p = mk(Decision::Deny, &[".example.com"], &[]);
        assert_eq!(p.decide("api.example.com"), Decision::Allow);
        assert_eq!(p.decide("example.com"), Decision::Deny);
    }
    #[test]
    fn case_insensitive() {
        assert_eq!(
            mk(Decision::Deny, &["API.Example.COM"], &[]).decide("api.example.com"),
            Decision::Allow
        );
    }
    #[test]
    fn trailing_dot_ignored() {
        assert_eq!(
            mk(Decision::Deny, &["api.example.com"], &[]).decide("api.example.com."),
            Decision::Allow
        );
    }
    #[test]
    fn empty_host_uses_default() {
        assert_eq!(mk(Decision::Deny, &[], &[]).decide(""), Decision::Deny);
    }
    #[test]
    fn add_allow_dedupes() {
        let mut p = mk(Decision::Deny, &[], &[]);
        p.add_allow("Example.COM");
        p.add_allow("example.com");
        assert_eq!(p.allow.len(), 1);
    }
    #[test]
    fn host_from_url_extracts() {
        assert_eq!(
            host_from_url("https://api.example.com/health"),
            Some("api.example.com".to_string())
        );
        assert_eq!(host_from_url("not a url"), None);
    }
    #[test]
    fn auditor_writes_lines() {
        let dir = tempdir().unwrap();
        let a = NetworkAuditor::new(dir.path().join("audit.log"), true);
        a.record("api.example.com", "fetch_url", "Allow");
        let body = std::fs::read_to_string(dir.path().join("audit.log")).unwrap();
        assert!(body.contains("Allow"));
    }
    #[test]
    fn session_cache_short_circuits() {
        let d = NetworkPolicyDecider::new(mk(Decision::Prompt, &[], &[]), None);
        assert_eq!(d.evaluate("api.example.com", "fetch"), Decision::Prompt);
        d.approve_session("api.example.com", "fetch");
        assert_eq!(d.evaluate("api.example.com", "fetch"), Decision::Allow);
    }
    #[test]
    fn approve_persistent_writes_back() {
        let mut d = NetworkPolicyDecider::new(mk(Decision::Prompt, &[], &[]), None);
        d.approve_persistent("api.example.com", "fetch");
        assert!(d.policy().allow.iter().any(|h| h == "api.example.com"));
    }
    #[test]
    fn decision_parse() {
        assert_eq!(Decision::parse("allow"), Decision::Allow);
        assert_eq!(Decision::parse("BLOCK"), Decision::Deny);
        assert_eq!(Decision::parse("garbage"), Decision::Prompt);
    }
    #[test]
    fn network_denied_carries_host() {
        let e = NetworkDenied("bad.com".into());
        assert_eq!(e.host(), "bad.com");
        assert!(format!("{e}").contains("bad.com"));
    }
}
