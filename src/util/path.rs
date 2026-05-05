//! Path utility functions

/// Get the coder config directory (~/.coder)
pub fn coder_dir() -> std::path::PathBuf {
    // Try HOME first (Unix), then USERPROFILE (Windows), then APPDATA
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .or_else(|_| std::env::var("APPDATA"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            // Last resort fallback
            std::path::PathBuf::from(".")
        })
        .join(".coder")
}

/// Get the sessions directory
pub fn sessions_dir() -> std::path::PathBuf {
    coder_dir().join("sessions")
}

/// Get the memory directory
pub fn memory_dir() -> std::path::PathBuf {
    coder_dir().join("memory")
}

/// Ensure a directory exists
pub fn ensure_dir(path: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coder_dir() {
        let dir = coder_dir();
        assert!(dir.ends_with(".coder"));
    }

    #[test]
    fn test_sessions_dir() {
        let dir = sessions_dir();
        assert!(dir.ends_with("sessions"));
    }
}
