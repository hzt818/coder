//! Security configuration and policy management
//!
//! Defines the security configuration schema that can be serialized
//! from the project's `config.toml` under the `[security]` section.
//!
//! ## Example config.toml
//!
//! ```toml
//! [security]
//! encryption_enabled = true
//! encryption_key_source = "prompt"  # "prompt", "env", "file"
//! sanitize_logs = true
//! sanitize_sessions = true
//! redact_ips = false
//!
//! [security.sanitizer]
//! redact_emails = true
//! redact_paths = true
//! custom_patterns = ["CUS\\d{6}"]
//! ```

use crate::security::sanitizer::SanitizerConfig;
use serde::{Deserialize, Serialize};

/// Security module configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Master switch — enable all security features
    pub enabled: bool,
    /// Enable AES-256-GCM data encryption at rest
    pub encryption_enabled: bool,
    /// How to obtain the encryption passphrase
    pub encryption_key_source: EncryptionKeySource,
    /// Path to encrypted key file (if key_source = "file")
    pub encryption_key_path: Option<String>,
    /// Enable PII redaction in logs
    pub sanitize_logs: bool,
    /// Enable PII redaction in saved sessions
    pub sanitize_sessions: bool,
    /// Enable PII redaction in audit events
    pub sanitize_audit: bool,
    /// Enable encrypted keyring for API keys
    pub keyring_enabled: bool,
    /// Path to the keyring file (default: ~/.coder/keyring.enc)
    pub keyring_path: Option<String>,
    /// Sanitizer configuration
    pub sanitizer: SanitizerConfig,
    /// Session encryption — encrypt individual session files
    pub encrypt_sessions: bool,
    /// Memory encryption — encrypt memory store files
    pub encrypt_memory: bool,
    /// Config file permission check (Unix: warn if permissions > 0600)
    pub check_config_permissions: bool,
    /// Whether to redact IP addresses (off by default)
    pub redact_ips: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            encryption_enabled: false,
            encryption_key_source: EncryptionKeySource::Prompt,
            encryption_key_path: None,
            sanitize_logs: true,
            sanitize_sessions: true,
            sanitize_audit: true,
            keyring_enabled: false,
            keyring_path: None,
            sanitizer: SanitizerConfig::default(),
            encrypt_sessions: false,
            encrypt_memory: false,
            check_config_permissions: true,
            redact_ips: false,
        }
    }
}

/// Source of the encryption passphrase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionKeySource {
    /// Prompt the user at startup (interactive)
    Prompt,
    /// Read from an environment variable
    Env(String),
    /// Read from a file
    File(String),
    /// Use a pre-generated machine-local key file
    MachineKey,
}

impl Default for EncryptionKeySource {
    fn default() -> Self {
        Self::Prompt
    }
}

/// Security audit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Whether audit logging is enabled
    pub enabled: bool,
    /// Whether to redact sensitive data in audit events
    pub sanitize_events: bool,
    /// Maximum audit log size in bytes before rotation (0 = unlimited)
    pub max_log_size: u64,
    /// Log file path (default: ~/.coder/audit.log)
    pub log_path: Option<String>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sanitize_events: true,
            max_log_size: 10 * 1024 * 1024, // 10 MiB
            log_path: None,
        }
    }
}

/// Runtime security policy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Whether to warn about world-readable config files
    pub warn_loose_permissions: bool,
    /// Minimum config file permission (Unix, octal)
    pub min_config_permission: u32,
    /// Whether to enforce that API keys aren't logged
    pub enforce_no_key_in_logs: bool,
    /// Whether to block sending file contents containing patterns
    /// (e.g., "PRIVATE KEY") to AI providers
    pub block_sensitive_content: bool,
    /// Whether to require encryption for session files
    pub require_encrypted_sessions: bool,
    /// Allowed providers (empty = all allowed)
    pub allowed_providers: Vec<String>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            warn_loose_permissions: true,
            min_config_permission: 0o600,
            enforce_no_key_in_logs: true,
            block_sensitive_content: true,
            require_encrypted_sessions: false,
            allowed_providers: Vec::new(),
        }
    }
}

/// Build a default security profile for the application
pub fn default_security_config() -> SecurityConfig {
    SecurityConfig {
        enabled: true,
        encryption_enabled: true,
        encryption_key_source: EncryptionKeySource::Prompt,
        sanitize_logs: true,
        sanitize_sessions: true,
        sanitize_audit: true,
        keyring_enabled: true,
        ..SecurityConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(!config.enabled);
        assert!(!config.encryption_enabled);
        assert!(config.sanitize_logs);
        assert!(config.sanitize_sessions);
    }

    #[test]
    fn test_audit_config_default() {
        let config = AuditConfig::default();
        assert!(config.enabled);
        assert!(config.sanitize_events);
        assert_eq!(config.max_log_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_security_policy_default() {
        let policy = SecurityPolicy::default();
        assert!(policy.warn_loose_permissions);
        assert_eq!(policy.min_config_permission, 0o600);
        assert!(policy.enforce_no_key_in_logs);
    }

    #[test]
    fn test_encryption_key_source_default() {
        let source = EncryptionKeySource::default();
        assert!(matches!(source, EncryptionKeySource::Prompt));
    }

    #[test]
    fn test_default_security_profile() {
        let config = default_security_config();
        assert!(config.enabled);
        assert!(config.encryption_enabled);
        assert!(config.keyring_enabled);
    }

    #[test]
    fn test_security_config_serialization() {
        let config = SecurityConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SecurityConfig = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.enabled);
        assert!(deserialized.sanitize_logs);
    }

    #[test]
    fn test_security_policy_allowed_providers() {
        let mut policy = SecurityPolicy::default();
        assert!(policy.allowed_providers.is_empty());
        policy.allowed_providers = vec!["openai".to_string(), "anthropic".to_string()];
        assert_eq!(policy.allowed_providers.len(), 2);
    }

    #[test]
    fn test_encryption_key_source_env() {
        let source = EncryptionKeySource::Env("CODER_ENCRYPTION_KEY".to_string());
        if let EncryptionKeySource::Env(var) = &source {
            assert_eq!(var, "CODER_ENCRYPTION_KEY");
        } else {
            panic!("Expected Env variant");
        }
    }

    #[test]
    fn test_encryption_key_source_file() {
        let source = EncryptionKeySource::File("/path/to/key".to_string());
        if let EncryptionKeySource::File(path) = &source {
            assert_eq!(path, "/path/to/key");
        } else {
            panic!("Expected File variant");
        }
    }
}
