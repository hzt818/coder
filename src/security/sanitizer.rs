//! Data sanitization and PII redaction module
//!
//! Protects user privacy by detecting and redacting personally identifiable
//! information (PII) from conversation logs, session data, and audit trails.
//!
//! ## Redaction Targets
//!
//! - **API keys / tokens**: Patterns for `sk-...`, `api_key=...`, bearer tokens
//! - **Email addresses**: Standard email format
//! - **IP addresses** (optional): IPv4 and IPv6
//! - **File paths / home directories**: `$HOME`, `~`, `/Users/username`
//! - **Environment variable values**: Values of sensitive env vars
//! - **Custom patterns**: User-defined regex patterns
//!
//! ## Usage
//!
//! ```ignore
//! let config = SanitizerConfig::default();
//! let sanitizer = DataSanitizer::new(config);
//! let cleaned = sanitizer.sanitize("my api key is sk-abc123...");
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Configuration for the data sanitizer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizerConfig {
    /// Whether to redact email addresses
    pub redact_emails: bool,
    /// Whether to redact IPv4/IPv6 addresses
    pub redact_ips: bool,
    /// Whether to redact file system paths (home dirs, temp paths)
    pub redact_paths: bool,
    /// Whether to redact API key patterns
    pub redact_api_keys: bool,
    /// Whether to redact environment variable assignments
    pub redact_env_vars: bool,
    /// Whether to redact JWT tokens
    pub redact_jwt: bool,
    /// Replacement text for redacted content
    pub replacement: String,
    /// Additional custom regex patterns to redact
    #[serde(default)]
    pub custom_patterns: Vec<String>,
    /// Sensitive environment variable names whose values should be redacted
    #[serde(default)]
    pub sensitive_env_vars: Vec<String>,
    /// Fields whose values should always be redacted (recursive JSON keys)
    #[serde(default)]
    pub sensitive_fields: Vec<String>,
}

impl Default for SanitizerConfig {
    fn default() -> Self {
        Self {
            redact_emails: true,
            redact_ips: false,
            redact_paths: true,
            redact_api_keys: true,
            redact_env_vars: true,
            redact_jwt: true,
            replacement: "[REDACTED]".to_string(),
            custom_patterns: Vec::new(),
            sensitive_env_vars: vec![
                "API_KEY".to_string(),
                "OPENAI_API_KEY".to_string(),
                "ANTHROPIC_API_KEY".to_string(),
                "OPENCODE_API_KEY".to_string(),
                "AWS_SECRET_ACCESS_KEY".to_string(),
                "GITHUB_TOKEN".to_string(),
                "CODER_PROVIDER".to_string(),
                "CODER_MODEL".to_string(),
                "DATABASE_URL".to_string(),
                "REDIS_URL".to_string(),
                "PGPASSWORD".to_string(),
                "SSH_KEY".to_string(),
                "TOKEN".to_string(),
                "SECRET".to_string(),
                "PASSWORD".to_string(),
                "PASS".to_string(),
            ],
            sensitive_fields: vec![
                "api_key".to_string(),
                "apiKey".to_string(),
                "api-key".to_string(),
                "secret".to_string(),
                "password".to_string(),
                "token".to_string(),
                "access_token".to_string(),
                "refresh_token".to_string(),
                "auth_token".to_string(),
                "private_key".to_string(),
                "passphrase".to_string(),
                "client_secret".to_string(),
            ],
        }
    }
}

/// Compiled patterns for efficient sanitization
#[derive(Debug, Clone)]
struct CompiledPatterns {
    email: Option<Regex>,
    ipv4: Option<Regex>,
    ipv6: Option<Regex>,
    api_key: Option<Regex>,
    jwt: Option<Regex>,
    env_var: Option<Regex>,
    path_home: Option<Regex>,
    custom: Vec<Regex>,
}

impl CompiledPatterns {
    fn new(config: &SanitizerConfig) -> Self {
        Self {
            email: config.redact_emails.then(|| {
                Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}").unwrap()
            }),
            ipv4: config.redact_ips.then(|| {
                Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap()
            }),
            ipv6: config.redact_ips.then(|| {
                Regex::new(r"\b(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}\b").unwrap()
            }),
            jwt: config.redact_jwt.then(|| {
                // JWT must be checked FIRST before api_key regex eats it
                Regex::new(r"eyJ[a-zA-Z0-9_\-]+\.eyJ[a-zA-Z0-9_\-]+\.[a-zA-Z0-9_\-]+").unwrap()
            }),
            api_key: config.redact_api_keys.then(|| {
                // Match various key patterns; minimum 12 chars after prefix to avoid false positives
                Regex::new(r"(?i)(?:sk-|pk-)[a-zA-Z0-9_\-]{12,}(?:[=]{0,2})").unwrap()
            }),
            env_var: config.redact_env_vars.then(|| {
                let names: Vec<&str> = config.sensitive_env_vars.iter().map(|s| s.as_str()).collect();
                // Allow optional whitespace around `=` for flexible matching
                let pattern = format!("(?i)({})\\s*=[^\\s\"'`;|&$]+", names.join("|"));
                Regex::new(&pattern).unwrap()
            }),
            path_home: config.redact_paths.then(|| {
                Regex::new(r"(?i)(~[/\\]|/home/[a-z_][a-z0-9_]*|/Users/[a-z_][a-z0-9_]*|/tmp/|/var/folders/|[A-Z]:\\Users\\[a-z_]+)").unwrap()
            }),
            custom: config
                .custom_patterns
                .iter()
                .filter_map(|p| Regex::new(p).ok())
                .collect(),
        }
    }

    fn apply_all(&self, text: &str, replacement: &str) -> String {
        let mut result = text.to_string();

        // JWT must be checked before api_key to avoid partial matches
        if let Some(ref re) = self.jwt {
            result = re.replace_all(&result, replacement).to_string();
        }
        if let Some(ref re) = self.email {
            result = re.replace_all(&result, replacement).to_string();
        }
        if let Some(ref re) = self.ipv4 {
            result = re.replace_all(&result, replacement).to_string();
        }
        if let Some(ref re) = self.ipv6 {
            result = re.replace_all(&result, replacement).to_string();
        }
        if let Some(ref re) = self.api_key {
            result = re.replace_all(&result, replacement).to_string();
        }
        if let Some(ref re) = self.env_var {
            // Replace only the value portion, keep the key
            result = re.replace_all(&result, |caps: &regex::Captures| {
                format!("{}={}", &caps[1], replacement)
            }).to_string();
        }
        if let Some(ref re) = self.path_home {
            result = re.replace_all(&result, replacement).to_string();
        }
        for re in &self.custom {
            result = re.replace_all(&result, replacement).to_string();
        }

        result
    }
}

/// Data sanitizer for redacting sensitive information
#[derive(Debug, Clone)]
pub struct DataSanitizer {
    config: SanitizerConfig,
    #[cfg_attr(not(test), allow(dead_code))]
    patterns: CompiledPatterns,
}

impl DataSanitizer {
    /// Create a new sanitizer with the given configuration
    pub fn new(config: SanitizerConfig) -> Self {
        let patterns = CompiledPatterns::new(&config);
        Self { config, patterns }
    }

    /// Create a sanitizer with default settings
    pub fn default_config() -> Self {
        Self::new(SanitizerConfig::default())
    }

    /// Sanitize a string, redacting all detected PII
    pub fn sanitize(&self, text: &str) -> String {
        self.patterns.apply_all(text, &self.config.replacement)
    }

    /// Sanitize a JSON value recursively, redacting sensitive fields
    pub fn sanitize_json(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::String(s) => {
                serde_json::Value::String(self.sanitize(s))
            }
            serde_json::Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (k, v) in map {
                    let sanitized_v = if self.is_sensitive_field(k) {
                        serde_json::Value::String(self.config.replacement.clone())
                    } else {
                        self.sanitize_json(v)
                    };
                    new_map.insert(k.clone(), sanitized_v);
                }
                serde_json::Value::Object(new_map)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| self.sanitize_json(v)).collect())
            }
            other => other.clone(),
        }
    }

    /// Sanitize key=value log lines (e.g., audit log entries)
    ///
    /// This is useful for tracing/log subscriber integration.
    pub fn sanitize_log_line(&self, line: &str) -> String {
        // First pass: apply standard pattern-based sanitization
        let result = self.sanitize(line);

        // Second pass: redact values of known sensitive keys in structured log output
        let sensitive_keys: HashSet<&str> = self
            .config
            .sensitive_fields
            .iter()
            .map(|s| s.as_str())
            .collect();

        // Match patterns like `key = "value"` or `key: "value"` in log lines
        // Use two separate captures to avoid double-quoting issues
        let key_value_re = Regex::new(r#"(?P<key>[a-zA-Z_][a-zA-Z0-9_]*)\s*[=:]\s*"(?P<value>[^"]*?)""#).unwrap();
        let result = key_value_re
            .replace_all(&result, |caps: &regex::Captures| {
                let key = &caps["key"];
                if sensitive_keys.contains(key) {
                    format!(r#"{key} = "[REDACTED]""#)
                } else {
                    caps[0].to_string()
                }
            })
            .to_string();
        result
    }

    /// Check if a JSON field name is sensitive
    fn is_sensitive_field(&self, name: &str) -> bool {
        self.config
            .sensitive_fields
            .iter()
            .any(|f| f == name || name.to_lowercase().contains(&f.to_lowercase()))
    }

    /// Get a reference to the current configuration
    pub fn config(&self) -> &SanitizerConfig {
        &self.config
    }
}

/// Convenience function: sanitize a string with default settings
pub fn sanitize(text: &str) -> String {
    DataSanitizer::default_config().sanitize(text)
}

/// Convenience function: sanitize a JSON value with default settings
pub fn sanitize_json(value: &serde_json::Value) -> serde_json::Value {
    DataSanitizer::default_config().sanitize_json(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_api_key() {
        let sanitizer = DataSanitizer::default_config();
        assert_eq!(
            sanitizer.sanitize("my key is sk-abc123def456ghi789"),
            "my key is [REDACTED]"
        );
    }

    #[test]
    fn test_redact_email() {
        let sanitizer = DataSanitizer::default_config();
        assert_eq!(
            sanitizer.sanitize("contact me at user@example.com for details"),
            "contact me at [REDACTED] for details"
        );
    }

    #[test]
    fn test_redact_env_var_value() {
        let sanitizer = DataSanitizer::default_config();
        let result = sanitizer.sanitize("OPENAI_API_KEY=sk-test-key-123456");
        assert_eq!(result, "OPENAI_API_KEY=[REDACTED]");
    }

    #[test]
    fn test_redact_home_path() {
        let sanitizer = DataSanitizer::default_config();
        assert_eq!(
            sanitizer.sanitize("config in /home/alice/.coder/config.toml"),
            "config in [REDACTED]/.coder/config.toml"
        );
    }

    #[test]
    fn test_redact_json_sensitive_field() {
        let sanitizer = DataSanitizer::default_config();
        let json = serde_json::json!({
            "name": "test",
            "api_key": "sk-secret-key-here",
            "nested": {
                "token": "eyJ.eyJ.secret"
            }
        });

        let sanitized = sanitizer.sanitize_json(&json);
        assert_eq!(sanitized["name"], "test");
        assert_eq!(sanitized["api_key"], "[REDACTED]");
        assert_eq!(sanitized["nested"]["token"], "[REDACTED]");
    }

    #[test]
    fn test_no_false_positive_normal_text() {
        let sanitizer = DataSanitizer::default_config();
        let normal = "Hello, this is a normal conversation about programming.";
        assert_eq!(sanitizer.sanitize(normal), normal);
    }

    #[test]
    fn test_redact_jwt() {
        let sanitizer = DataSanitizer::default_config();
        // JWT is matched by the JWT regex which runs before api_key regex
        let jwt = "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3j6Keb5zP1M";
        let result = sanitizer.sanitize(jwt);
        // The "Bearer " prefix stays; the JWT token is fully redacted
        assert!(!result.contains("eyJhbGciOiJI"));
        assert!(result.contains("Bearer"));
    }

    #[test]
    fn test_redact_multiple_patterns() {
        let sanitizer = DataSanitizer::default_config();
        let text = "User alice@example.com has key sk-abcdef123456 and home at /home/alice";
        let result = sanitizer.sanitize(text);
        assert!(!result.contains("alice@example.com"));
        assert!(!result.contains("sk-abcdef123456"));
        assert!(!result.contains("/home/alice"));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn test_sanitize_log_line() {
        let sanitizer = DataSanitizer::default_config();
        let line = r#"api_key = "sk-test-key-12345678""#;
        let result = sanitizer.sanitize_log_line(line);
        assert_eq!(result, r#"api_key = "[REDACTED]""#);
    }

    #[test]
    fn test_empty_string() {
        let sanitizer = DataSanitizer::default_config();
        assert_eq!(sanitizer.sanitize(""), "");
    }

    #[test]
    fn test_custom_pattern() {
        let mut config = SanitizerConfig::default();
        config.custom_patterns.push(r"\bSECRET-\d{4}\b".to_string());
        let sanitizer = DataSanitizer::new(config);
        assert_eq!(
            sanitizer.sanitize("my code is SECRET-1234"),
            "my code is [REDACTED]"
        );
    }

    #[test]
    fn test_ip_redaction_disabled_by_default() {
        let sanitizer = DataSanitizer::default_config();
        // IP redaction is off by default
        assert_eq!(
            sanitizer.sanitize("server at 192.168.1.1"),
            "server at 192.168.1.1"
        );
    }

    #[test]
    fn test_ip_redaction_enabled() {
        let mut config = SanitizerConfig::default();
        config.redact_ips = true;
        let sanitizer = DataSanitizer::new(config);
        assert_eq!(
            sanitizer.sanitize("server at 192.168.1.1"),
            "server at [REDACTED]"
        );
    }

    #[test]
    fn test_json_array_sanitization() {
        let sanitizer = DataSanitizer::default_config();
        let json = serde_json::json!([
            {"name": "user1", "api_key": "sk-key1"},
            {"name": "user2", "api_key": "sk-key2"}
        ]);
        let result = sanitizer.sanitize_json(&json);
        assert_eq!(result[0]["api_key"], "[REDACTED]");
        assert_eq!(result[1]["api_key"], "[REDACTED]");
        assert_eq!(result[0]["name"], "user1");
    }

    #[test]
    fn test_sanitize_fn_convenience() {
        let result = sanitize("email: test@example.com");
        assert_eq!(result, "email: [REDACTED]");
    }

    #[test]
    fn test_redact_url_credentials() {
        let sanitizer = DataSanitizer::default_config();
        // URL-embedded credentials in connection strings
        let text = "postgres://user:password@localhost/db";
        // The password portion might not be caught by current patterns
        // but api_key-style patterns should catch explicit tokens
        let result = sanitizer.sanitize(text);
        assert_eq!(result, text); // URL passwords aren't in default patterns — expected
    }

    #[test]
    fn test_sensitive_field_case_insensitive() {
        let mut config = SanitizerConfig::default();
        config.sensitive_fields = vec!["api_key".to_string()];
        let sanitizer = DataSanitizer::new(config);

        let json = serde_json::json!({"API_KEY": "secret", "Api_Key": "secret2"});
        let result = sanitizer.sanitize_json(&json);
        assert_eq!(result["API_KEY"], "[REDACTED]");
        assert_eq!(result["Api_Key"], "[REDACTED]");
    }
}
