//! Security module — data encryption, privacy protection, and credential management
//!
//! This module provides comprehensive security features for protecting user data:
//!
//! ## Sub-modules
//!
//! - [`encryption`] — AES-256-GCM data-at-rest encryption with Argon2id key derivation
//! - [`sanitizer`] — PII redaction and sensitive data sanitization for logs and sessions
//! - [`keychain`] — Secure in-memory API key and credential management
//! - [`config`] — Security configuration schema (serialized from `config.toml`)
//!
//! ## Quick Start
//!
//! ```ignore
//! use coder::security::{SecurityManager, SecurityConfig};
//!
//! let config = SecurityConfig::default();
//! let mut manager = SecurityManager::init(config)?;
//!
//! // Protect sensitive data
//! let encrypted = manager.cipher().encrypt_str("my secret")?;
//! let decrypted = manager.cipher().decrypt_str(&encrypted)?;
//!
//! // Sanitize PII from logs
//! let clean = manager.sanitize("user@example.com");
//!
//! // Store API keys securely
//! manager.store_api_key("openai", "sk-...")?;
//! let key = manager.api_key("openai")?;
//! ```

pub mod config;
pub mod encryption;
pub mod keychain;
pub mod sanitizer;

use config::{SecurityConfig, SecurityPolicy};
use encryption::{CryptoError, DataCipher};
use keychain::{Keychain, KeychainError};
use sanitizer::DataSanitizer;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

/// Global security manager instance (initialized once)
static SECURITY_MANAGER: OnceLock<Arc<SecurityManager>> = OnceLock::new();

/// Main entry point for all security operations.
///
/// The `SecurityManager` is initialized once at application startup and
/// provides access to encryption, sanitization, and key management services.
pub struct SecurityManager {
    /// Security configuration
    config: SecurityConfig,
    /// Data cipher for encryption/decryption
    cipher: Option<DataCipher>,
    /// Data sanitizer for PII redaction
    sanitizer: DataSanitizer,
    /// Secure keychain for API keys
    keychain: Keychain,
    /// Security policy
    policy: SecurityPolicy,
    /// Whether the manager is fully initialized
    initialized: bool,
}

/// Errors from SecurityManager operations
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    /// Cryptography error
    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),
    /// Keychain error
    #[error("Keychain error: {0}")]
    Keychain(#[from] KeychainError),
    /// Not initialized
    #[error("Security manager not initialized")]
    NotInitialized,
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
    /// Path traversal detected
    #[error("Path traversal detected: {0}")]
    PathTraversal(String),
}

impl SecurityManager {
    /// Initialize the security manager with the given configuration.
    ///
    /// This should be called once at application startup.
    /// On success, the global instance is available via `SecurityManager::global()`.
    pub fn init(config: SecurityConfig) -> Result<&'static Arc<Self>, SecurityError> {
        let manager = Self::new(config)?;
        let arc = Arc::new(manager);
        Ok(SECURITY_MANAGER.get_or_init(|| arc))
    }

    /// Create a new security manager (without registering as global)
    pub fn new(config: SecurityConfig) -> Result<Self, SecurityError> {
        // Initialize cipher if encryption is enabled
        let cipher = if config.encryption_enabled {
            match Self::initialize_cipher(&config) {
                Ok(c) => Some(c),
                Err(SecurityError::Config(_)) if matches!(config.encryption_key_source, config::EncryptionKeySource::Prompt) => {
                    // Prompt mode: cipher will be set later via set_passphrase()
                    None
                }
                Err(e) => return Err(e),
            }
        } else {
            None
        };

        // Initialize sanitizer
        let mut sanitizer_config = config.sanitizer.clone();
        if config.redact_ips {
            sanitizer_config.redact_ips = true;
        }
        let sanitizer = DataSanitizer::new(sanitizer_config);

        // Initialize keychain
        let mut keychain = Keychain::new();

        // Load keyring if enabled
        if config.keyring_enabled {
            if let Some(ref cipher) = cipher {
                let keyring_path = config
                    .keyring_path
                    .as_ref()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                        p.push(".coder");
                        p.push("keyring.enc");
                        p
                    });

                if keyring_path.exists() {
                    if let Err(e) = keychain.load_keyring(cipher, &keyring_path) {
                        tracing::warn!("Failed to load keyring (may be first use): {}", e);
                    }
                }
            }
        }

        Ok(Self {
            config: config.clone(),
            cipher,
            sanitizer,
            keychain,
            policy: SecurityPolicy::default(),
            initialized: true,
        })
    }

    /// Get the global security manager instance
    pub fn global() -> Result<&'static Arc<Self>, SecurityError> {
        SECURITY_MANAGER
            .get()
            .ok_or(SecurityError::NotInitialized)
    }

    /// Initialize the data cipher based on configured key source
    fn initialize_cipher(config: &SecurityConfig) -> Result<DataCipher, SecurityError> {
        match &config.encryption_key_source {
            config::EncryptionKeySource::Prompt => {
                // Will be set later via set_passphrase()
                Err(SecurityError::Config(
                    "Passphrase-based encryption requires interactive setup via set_passphrase()"
                        .to_string(),
                ))
            }
            config::EncryptionKeySource::Env(var) => {
                let passphrase = std::env::var(var).map_err(|_| {
                    SecurityError::Config(format!(
                        "Encryption key env var '{}' not set",
                        var
                    ))
                })?;
                Ok(DataCipher::new(passphrase.as_bytes())?)
            }
            config::EncryptionKeySource::File(path) => {
                let key_data = std::fs::read_to_string(path).map_err(|e| {
                    SecurityError::Config(format!("Cannot read encryption key file: {}", e))
                })?;
                let trimmed = key_data.trim().to_string();
                // Try as base64-encoded key first, then as passphrase
                if trimmed.len() == 44 && trimmed.ends_with('=') {
                    DataCipher::from_b64_key(&trimmed)
                } else {
                    DataCipher::new(trimmed.as_bytes())
                }
                .map_err(SecurityError::from)
            }
            config::EncryptionKeySource::MachineKey => {
                let key_path = get_machine_key_path();
                if key_path.exists() {
                    let key_data = std::fs::read_to_string(&key_path).map_err(|e| {
                        SecurityError::Config(format!("Cannot read machine key: {}", e))
                    })?;
                    DataCipher::from_b64_key(key_data.trim())
                        .map_err(SecurityError::from)
                } else {
                    // Generate and save a new machine key
                    let new_key = DataCipher::generate_key();
                    if let Some(parent) = key_path.parent() {
                        std::fs::create_dir_all(parent).ok();
                    }
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::write(&key_path, &new_key).ok();
                        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600)).ok();
                    }
                    #[cfg(not(unix))]
                    std::fs::write(&key_path, &new_key).ok();

                    DataCipher::from_b64_key(&new_key).map_err(SecurityError::from)
                }
            }
        }
    }

    /// Set the encryption passphrase interactively (for `Prompt` key source)
    pub fn set_passphrase(&mut self, passphrase: &str) -> Result<(), SecurityError> {
        let cipher = DataCipher::new(passphrase.as_bytes())?;
        self.cipher = Some(cipher);
        Ok(())
    }

    /// Get the data cipher (for encryption operations)
    pub fn cipher(&self) -> Result<&DataCipher, SecurityError> {
        self.cipher
            .as_ref()
            .ok_or(SecurityError::Config("Encryption not enabled".to_string()))
    }

    /// Sanitize a string (redact PII)
    pub fn sanitize(&self, text: &str) -> String {
        self.sanitizer.sanitize(text)
    }

    /// Sanitize a JSON value (redact sensitive fields + PII)
    pub fn sanitize_json(&self, value: &serde_json::Value) -> serde_json::Value {
        self.sanitizer.sanitize_json(value)
    }

    /// Sanitize a log line
    pub fn sanitize_log(&self, line: &str) -> String {
        self.sanitizer.sanitize_log_line(line)
    }

    /// Store an API key securely (in memory, optionally persisted to keyring)
    pub fn store_api_key(&mut self, name: &str, value: &str) {
        self.keychain.set_key(name, value);
    }

    /// Retrieve an API key (checks memory, then env vars)
    pub fn api_key(&self, name: &str) -> Result<String, KeychainError> {
        self.keychain.get_key(name)
    }

    /// Persist the current keychain to disk (encrypted)
    pub fn save_keyring(&self) -> Result<(), SecurityError> {
        let cipher = self.cipher()?;
        let keyring_path = self
            .config
            .keyring_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                p.push(".coder");
                p.push("keyring.enc");
                p
            });
        self.keychain.save_keyring(cipher, &keyring_path)?;
        Ok(())
    }

    /// Encrypt a string for storage
    pub fn encrypt(&self, plaintext: &str) -> Result<String, SecurityError> {
        Ok(self.cipher()?.encrypt_str(plaintext)?)
    }

    /// Decrypt a string
    pub fn decrypt(&self, ciphertext: &str) -> Result<String, SecurityError> {
        Ok(self.cipher()?.decrypt_str(ciphertext)?)
    }

    /// Check if a file path is safe (no path traversal)
    pub fn validate_path(&self, path: &std::path::Path, allowed_base: &std::path::Path) -> Result<(), SecurityError> {
        let canonical = path.canonicalize().map_err(|_| {
            SecurityError::PathTraversal(format!("Cannot resolve path: {}", path.display()))
        })?;

        let base = allowed_base.canonicalize().map_err(|_| {
            SecurityError::PathTraversal(format!("Cannot resolve base: {}", allowed_base.display()))
        })?;

        if !canonical.starts_with(&base) {
            return Err(SecurityError::PathTraversal(format!(
                "Path '{}' is outside allowed base '{}'",
                canonical.display(),
                base.display()
            )));
        }

        Ok(())
    }

    /// Get the security configuration
    pub fn config(&self) -> &SecurityConfig {
        &self.config
    }

    /// Get the security policy
    pub fn policy(&self) -> &SecurityPolicy {
        &self.policy
    }

    /// Get the underlying keychain
    pub fn keychain(&self) -> &Keychain {
        &self.keychain
    }

    /// Get mutable access to the keychain
    pub fn keychain_mut(&mut self) -> &mut Keychain {
        &mut self.keychain
    }

    /// Check if the security manager is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Check if encryption is available
    pub fn encryption_available(&self) -> bool {
        self.cipher.is_some()
    }

    /// Validate that sensitive file content is not being sent to AI providers.
    ///
    /// Returns `true` if the content is safe to send, `false` if it contains
    /// sensitive patterns like private keys.
    pub fn check_content_safe(&self, content: &str) -> bool {
        if !self.policy.block_sensitive_content {
            return true;
        }

        let sensitive_patterns = [
            "BEGIN RSA PRIVATE KEY",
            "BEGIN DSA PRIVATE KEY",
            "BEGIN EC PRIVATE KEY",
            "BEGIN OPENSSH PRIVATE KEY",
            "BEGIN PGP PRIVATE KEY",
            "ghp_",       // GitHub personal access token (classic)
            "gho_",       // GitHub OAuth access token
            "ghu_",       // GitHub user-to-server token
            "ghs_",       // GitHub server-to-server token
            "ghr_",       // GitHub refresh token
        ];

        !sensitive_patterns.iter().any(|p| content.contains(p))
    }

    /// Check config file permissions (Unix only) and warn if too open
    pub fn check_config_permissions(_config_path: &std::path::Path) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(config_path) {
                let mode = metadata.permissions().mode();
                // Warn if world-readable or group-writable
                if mode & 0o004 > 0 || mode & 0o020 > 0 {
                    tracing::warn!(
                        "Config file '{}' has loose permissions ({:o}). \
                         Consider: chmod 600 {}",
                        config_path.display(),
                        mode,
                        config_path.display()
                    );
                    return false;
                }
            }
            true
        }
        #[cfg(not(unix))]
        true // No permission model on Windows
    }
}

/// Get the path to the machine-local encryption key
fn get_machine_key_path() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("coder");
    path.push("machine-key.enc");
    path
}

/// Initialize the global security manager with default settings.
/// Called once at startup from `main.rs`.
pub fn init_security(config: SecurityConfig) -> Result<(), SecurityError> {
    SecurityManager::init(config)?;
    tracing::info!("Security module initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_manager_new() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config).unwrap();
        assert!(manager.is_initialized());
        assert!(!manager.encryption_available());
    }

    #[test]
    fn test_sanitize_via_manager() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config).unwrap();
        let result = manager.sanitize("email: user@example.com");
        assert_eq!(result, "email: [REDACTED]");
    }

    #[test]
    fn test_encrypt_decrypt_via_manager() {
        let config = SecurityConfig {
            encryption_enabled: true,
            encryption_key_source: config::EncryptionKeySource::MachineKey,
            ..SecurityConfig::default()
        };
        let manager = SecurityManager::new(config).unwrap();
        assert!(manager.encryption_available());

        let encrypted = manager.encrypt("secret data").unwrap();
        let decrypted = manager.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, "secret data");
    }

    #[test]
    fn test_api_key_storage() {
        let config = SecurityConfig::default();
        let mut manager = SecurityManager::new(config).unwrap();
        manager.store_api_key("test-provider", "test-key-123");
        let key = manager.api_key("test-provider").unwrap();
        assert_eq!(key, "test-key-123");
    }

    #[test]
    fn test_check_content_safe() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config).unwrap();

        assert!(manager.check_content_safe("normal code content"));
        assert!(!manager.check_content_safe("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(!manager.check_content_safe("ghp_abc123xxx"));
    }

    #[test]
    fn test_validate_path() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let allowed_base = dir.path().to_path_buf();
        let safe_file = dir.path().join("safe.txt");
        std::fs::write(&safe_file, b"test").unwrap();

        // Safe path should pass
        assert!(manager.validate_path(&safe_file, &allowed_base).is_ok());

        // Non-existent path under base should fail (can't canonicalize)
        let non_existent = dir.path().join("nonexistent");
        assert!(manager.validate_path(&non_existent, &allowed_base).is_err());
    }

    #[test]
    fn test_manager_not_initialized() {
        let result = SecurityManager::global();
        assert!(result.is_err());
    }

    #[test]
    fn test_passphrase_config() {
        let config = SecurityConfig {
            encryption_enabled: true,
            encryption_key_source: config::EncryptionKeySource::Prompt,
            ..SecurityConfig::default()
        };
        let mut manager = SecurityManager::new(config).unwrap();
        // Cipher should not be available until passphrase is set
        assert!(!manager.encryption_available());

        manager.set_passphrase("my-strong-passphrase").unwrap();
        assert!(manager.encryption_available());

        let encrypted = manager.encrypt("test").unwrap();
        let decrypted = manager.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, "test");
    }

    #[test]
    fn test_keychain_integration() {
        let config = SecurityConfig::default();
        let mut manager = SecurityManager::new(config).unwrap();

        manager.store_api_key("openai", "sk-test");
        assert!(manager.api_key("openai").is_ok());

        // Access the underlying keychain
        assert_eq!(manager.keychain().len(), 1);
    }

    #[test]
    fn test_config_permission_check() {
        // Just verify the function exists and doesn't panic
        let path = std::path::Path::new("Cargo.toml");
        let _ = SecurityManager::check_config_permissions(path);
    }
}
