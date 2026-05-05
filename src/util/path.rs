//! Path utility functions

/// Get the coder config directory (~/.coder)
pub fn coder_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .expect("HOME directory not found")
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
