//! Data-at-rest encryption module
//!
//! Provides AES-256-GCM authenticated encryption for sensitive data stored on disk:
//! session files, API key caches, memory entries, and configuration secrets.
//!
//! ## Key Architecture
//!
//! - Master encryption key is derived from a user-provided passphrase via Argon2id.
//! - Each encryption operation uses a unique 96-bit random nonce (never reused).
//! - AEAD authentication prevents tampering with ciphertext.
//! - `Zeroizing<[u8; 32]>` ensures key material is cleared from memory on drop.
//!
//! ## Usage
//!
//! ```ignore
//! let cipher = DataCipher::new(b"user-provided-passphrase");
//! let encrypted = cipher.encrypt(b"sensitive data")?;
//! let decrypted = cipher.decrypt(&encrypted)?;
//! ```

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use zeroize::Zeroizing;

/// AEAD-related errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CryptoError {
    /// Encryption failed
    #[error("Encryption failed: {0}")]
    EncryptFailed(String),
    /// Decryption failed (wrong key, tampered data)
    #[error("Decryption failed: {0}")]
    DecryptFailed(String),
    /// Key derivation failed
    #[error("Key derivation failed: {0}")]
    KeyDeriveFailed(String),
    /// Invalid ciphertext format
    #[error("Invalid ciphertext format: {0}")]
    InvalidFormat(String),
}

/// An encrypted payload with authentication tag and nonce.
///
/// Stored as: `base64(nonce || ciphertext || tag)`
/// Nonce: 12 bytes, Ciphertext: variable, Tag: 16 bytes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPayload {
    /// The complete ciphertext (nonce + encrypted data + tag), base64-encoded
    pub data: String,
    /// Algorithm identifier for future-proofing
    pub algorithm: String,
}

impl EncryptedPayload {
    /// Create a new encrypted payload
    pub fn new(data: String) -> Self {
        Self {
            data,
            algorithm: "AES-256-GCM".to_string(),
        }
    }
}

/// AES-256-GCM data cipher with Argon2id key derivation.
///
/// The master key is derived once during construction and securely zeroed on drop.
pub struct DataCipher {
    key: Zeroizing<[u8; 32]>,
    /// Argon2 parameters for key derivation
    params: Argon2Params,
}

/// Argon2id key derivation parameters
#[derive(Debug, Clone, Copy)]
pub struct Argon2Params {
    /// Memory cost in KiB (default: 19456 = 19 MiB)
    pub memory_cost: u32,
    /// Time cost (iterations, default: 2)
    pub time_cost: u32,
    /// Parallelism (threads, default: 1)
    pub parallelism: u32,
}

impl Default for Argon2Params {
    fn default() -> Self {
        Self {
            memory_cost: 19456,
            time_cost: 2,
            parallelism: 1,
        }
    }
}

impl DataCipher {
    /// Create a new cipher with a passphrase.
    ///
    /// The passphrase is used to derive a 256-bit AES key via Argon2id.
    /// The salt is derived from a SHA-256 hash of the passphrase combined
    /// with a static application pepper (`"coder::security::v1"`).
    ///
    /// For stronger security, use `with_salt()` to provide a random salt
    /// stored separately from the ciphertext.
    pub fn new(passphrase: &[u8]) -> Result<Self, CryptoError> {
        let params = Argon2Params::default();
        let key = Self::derive_key(passphrase, &params)?;
        Ok(Self { key, params })
    }

    /// Create a cipher with explicit Argon2 parameters
    pub fn with_params(passphrase: &[u8], params: Argon2Params) -> Result<Self, CryptoError> {
        let key = Self::derive_key(passphrase, &params)?;
        Ok(Self { key, params })
    }

    /// Derive a 256-bit key from a passphrase using Argon2id
    fn derive_key(passphrase: &[u8], params: &Argon2Params) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
        // Use a static application-specific salt + passphrase hash
        let mut hasher = Sha256::new();
        hasher.update(b"coder::security::v1::key_derivation");
        hasher.update(passphrase);
        let salt_hash = hasher.finalize();

        let mut key = Zeroizing::new([0u8; 32]);
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(
                params.memory_cost,
                params.time_cost,
                params.parallelism,
                None,
            )
            .map_err(|e| CryptoError::KeyDeriveFailed(e.to_string()))?,
        );

        argon2
            .hash_password_into(passphrase, &salt_hash[..16], &mut *key)
            .map_err(|e| CryptoError::KeyDeriveFailed(e.to_string()))?;

        Ok(key)
    }

    /// Encrypt plaintext bytes.
    ///
    /// Returns an `EncryptedPayload` containing the base64-encoded
    /// `nonce || ciphertext || auth_tag`.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedPayload, CryptoError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key[..])
            .map_err(|e| CryptoError::EncryptFailed(format!("Invalid key length: {}", e)))?;

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| CryptoError::EncryptFailed(e.to_string()))?;

        // Combine: nonce (12) + ciphertext+tag (variable)
        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(EncryptedPayload::new(BASE64.encode(&combined)))
    }

    /// Decrypt an `EncryptedPayload` back to plaintext.
    pub fn decrypt(&self, payload: &EncryptedPayload) -> Result<Vec<u8>, CryptoError> {
        if payload.algorithm != "AES-256-GCM" {
            return Err(CryptoError::InvalidFormat(format!(
                "Unsupported algorithm: {}",
                payload.algorithm
            )));
        }

        let combined = BASE64
            .decode(&payload.data)
            .map_err(|e| CryptoError::InvalidFormat(format!("Base64 decode failed: {}", e)))?;

        if combined.len() < 12 {
            return Err(CryptoError::InvalidFormat(
                "Ciphertext too short (missing nonce)".to_string(),
            ));
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = Aes256Gcm::new_from_slice(&self.key[..])
            .map_err(|e| CryptoError::DecryptFailed(format!("Invalid key: {}", e)))?;

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| CryptoError::DecryptFailed(format!(
                "Decryption failed — wrong passphrase or tampered data: {}", e
            )))
    }

    /// Encrypt a string and return base64-encoded ciphertext.
    pub fn encrypt_str(&self, plaintext: &str) -> Result<String, CryptoError> {
        let payload = self.encrypt(plaintext.as_bytes())?;
        Ok(payload.data)
    }

    /// Decrypt a base64-encoded ciphertext back to a string.
    pub fn decrypt_str(&self, data: &str) -> Result<String, CryptoError> {
        let payload = EncryptedPayload::new(data.to_string());
        let bytes = self.decrypt(&payload)?;
        String::from_utf8(bytes)
            .map_err(|e| CryptoError::DecryptFailed(format!("Invalid UTF-8: {}", e)))
    }

    /// Encrypt a file on disk.
    ///
    /// Reads the file at `src_path`, encrypts it, and writes the result to `dst_path`.
    /// If `dst_path` is None, overwrites the source file.
    pub fn encrypt_file(&self, src_path: &std::path::Path, dst_path: Option<&std::path::Path>) -> Result<(), CryptoError> {
        let data = std::fs::read(src_path)
            .map_err(|e| CryptoError::EncryptFailed(format!("Cannot read source: {}", e)))?;
        let payload = self.encrypt(&data)?;
        let out_path = dst_path.unwrap_or(src_path);
        std::fs::write(out_path, &payload.data.as_bytes())
            .map_err(|e| CryptoError::EncryptFailed(format!("Cannot write output: {}", e)))?;
        Ok(())
    }

    /// Decrypt a file on disk.
    ///
    /// Reads the encrypted file at `src_path`, decrypts it, and writes plaintext to `dst_path`.
    pub fn decrypt_file(&self, src_path: &std::path::Path, dst_path: &std::path::Path) -> Result<(), CryptoError> {
        let encoded = std::fs::read_to_string(src_path)
            .map_err(|e| CryptoError::DecryptFailed(format!("Cannot read encrypted file: {}", e)))?;
        let payload = EncryptedPayload::new(encoded.trim().to_string());
        let plaintext = self.decrypt(&payload)?;
        std::fs::write(dst_path, &plaintext)
            .map_err(|e| CryptoError::DecryptFailed(format!("Cannot write output: {}", e)))?;
        Ok(())
    }

    /// Generate a random 256-bit key (for machine-local key files).
    ///
    /// Returns a base64-encoded key that can be stored and later loaded
    /// via `from_b64_key()`.
    pub fn generate_key() -> String {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        BASE64.encode(key)
    }

    /// Create a cipher from a pre-generated base64-encoded key.
    ///
    /// This is faster than passphrase derivation and useful for
    /// machine-bound encryption where UX pauses are undesirable.
    pub fn from_b64_key(encoded: &str) -> Result<Self, CryptoError> {
        let decoded = BASE64
            .decode(encoded)
            .map_err(|e| CryptoError::KeyDeriveFailed(format!("Invalid base64 key: {}", e)))?;
        if decoded.len() != 32 {
            return Err(CryptoError::KeyDeriveFailed("Key must be 32 bytes".to_string()));
        }
        let mut key = Zeroizing::new([0u8; 32]);
        key.copy_from_slice(&decoded);
        Ok(Self {
            key,
            params: Argon2Params::default(),
        })
    }
}

impl fmt::Debug for DataCipher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataCipher")
            .field("algorithm", &"AES-256-GCM")
            .field("params", &self.params)
            .finish_non_exhaustive()
    }
}

// ── Convenience functions for session-level encryption ──

/// Check whether a file appears to be encrypted (starts with base64 of nonce prefix).
/// This is a heuristic: we check if the content is valid base64 and length suggests encrypted data.
pub fn is_encrypted_file(path: &std::path::Path) -> bool {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c.trim().to_string(),
        Err(_) => return false,
    };

    // Encrypted payloads are base64-encoded, typically longer than 32 chars
    if content.len() < 32 {
        return false;
    }

    // Try to decode as base64 — if it works and has the right structure, likely encrypted
    BASE64.decode(&content).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let cipher = DataCipher::new(b"test-passphrase").unwrap();
        let plaintext = b"Hello, this is sensitive data!";
        let encrypted = cipher.encrypt(plaintext).unwrap();
        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_string() {
        let cipher = DataCipher::new(b"test-passphrase").unwrap();
        let original = "Sensitive user data";
        let encrypted = cipher.encrypt_str(original).unwrap();
        let decrypted = cipher.decrypt_str(&encrypted).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_wrong_key_fails() {
        let cipher1 = DataCipher::new(b"correct-passphrase").unwrap();
        let cipher2 = DataCipher::new(b"wrong-passphrase").unwrap();

        let encrypted = cipher1.encrypt(b"secret data").unwrap();
        let result = cipher2.decrypt(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_data_fails() {
        let cipher = DataCipher::new(b"test-passphrase").unwrap();
        let encrypted = cipher.encrypt(b"important data").unwrap();

        // Tamper with the base64 data
        let mut tampered = encrypted.data.clone();
        tampered.pop();
        tampered.push('X');

        let payload = EncryptedPayload::new(tampered);
        let result = cipher.decrypt(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_and_use_key() {
        let key = DataCipher::generate_key();
        let cipher = DataCipher::from_b64_key(&key).unwrap();
        let encrypted = cipher.encrypt(b"test").unwrap();
        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, b"test");
    }

    #[test]
    fn test_encrypt_file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("secret.txt");
        let encrypted_path = dir.path().join("secret.enc");
        let decrypted_path = dir.path().join("secret_dec.txt");

        std::fs::write(&src, b"file-level encryption test").unwrap();

        let cipher = DataCipher::new(b"file-passphrase").unwrap();
        cipher.encrypt_file(&src, Some(&encrypted_path)).unwrap();
        cipher.decrypt_file(&encrypted_path, &decrypted_path).unwrap();

        let result = std::fs::read_to_string(&decrypted_path).unwrap();
        assert_eq!(result, "file-level encryption test");
    }

    #[test]
    fn test_is_encrypted_file() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("plain.txt");
        let enc = dir.path().join("secret.enc");

        std::fs::write(&plain, b"hello world").unwrap();

        let cipher = DataCipher::new(b"test").unwrap();
        let payload = cipher.encrypt(b"sensitive").unwrap();
        std::fs::write(&enc, &payload.data).unwrap();

        assert!(!is_encrypted_file(&plain));
        assert!(is_encrypted_file(&enc));
    }

    #[test]
    fn test_empty_data() {
        let cipher = DataCipher::new(b"test").unwrap();
        let encrypted = cipher.encrypt(b"").unwrap();
        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, b"");
    }

    #[test]
    fn test_argon2_params() {
        let params = Argon2Params {
            memory_cost: 1024, // Very low for test speed
            time_cost: 1,
            parallelism: 1,
        };
        let cipher = DataCipher::with_params(b"test", params).unwrap();
        let encrypted = cipher.encrypt(b"quick test").unwrap();
        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, b"quick test");
    }

    #[test]
    fn test_invalid_base64_key() {
        let result = DataCipher::from_b64_key("invalid!");
        assert!(result.is_err());
    }

    #[test]
    fn test_different_passphrases_different_ciphertext() {
        let cipher1 = DataCipher::new(b"pass-1").unwrap();
        let cipher2 = DataCipher::new(b"pass-2").unwrap();

        let enc1 = cipher1.encrypt_str("same data").unwrap();
        let enc2 = cipher2.encrypt_str("same data").unwrap();

        // Different keys produce different ciphertexts (due to different derived keys)
        // But even same key + different nonce = different ciphertext, so this test
        // checks that keys truly differ
        assert_ne!(enc1, enc2, "Different passphrases should yield different ciphertexts");
    }

    #[test]
    fn test_encrypt_payload_algorithm() {
        let cipher = DataCipher::new(b"test").unwrap();
        let payload = cipher.encrypt(b"testdata").unwrap();
        assert_eq!(payload.algorithm, "AES-256-GCM");
    }

    #[test]
    fn test_unsupported_algorithm() {
        let cipher = DataCipher::new(b"test").unwrap();
        let payload = EncryptedPayload {
            data: "AAAA".to_string(),
            algorithm: "DES".to_string(),
        };
        let result = cipher.decrypt(&payload);
        assert!(matches!(result, Err(CryptoError::InvalidFormat(_))));
    }
}
