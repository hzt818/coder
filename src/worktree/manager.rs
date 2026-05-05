//! Git worktree management

use std::path::{Path, PathBuf};
use std::process::Command;

use super::{WorktreeError, WorktreeInfo, WorktreeResult, WorktreeStatus};

/// Manages Git worktrees for isolated development environments
#[derive(Debug)]
pub struct WorktreeManager {
    /// Path to the main repository
    repo_path: PathBuf,
    /// Base directory where worktrees are created
    worktree_base: PathBuf,
}

impl WorktreeManager {
    /// Create a new WorktreeManager for the given repository
    ///
    /// `worktree_base` is the directory where worktree directories will be created.
    pub fn new(repo_path: &Path, worktree_base: &Path) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
            worktree_base: worktree_base.to_path_buf(),
        }
    }

    /// Create a new worktree with the given branch name
    ///
    /// If `branch` does not exist yet, it will be created from the current HEAD.
    pub fn create(&self, name: &str, branch: &str) -> WorktreeResult<WorktreeInfo> {
        self.validate_name(name)?;
        let worktree_path = self.worktree_base.join(name);

        if worktree_path.exists() {
            return Err(WorktreeError::AlreadyExists(format!(
                "Worktree directory already exists: {}",
                worktree_path.display()
            )));
        }

        let output = Command::new("git")
            .arg("-C")
            .arg(&self.repo_path)
            .arg("worktree")
            .arg("add")
            .arg(&worktree_path)
            .arg(branch)
            .output()
            .map_err(|e| WorktreeError::Git(format!("Failed to execute git: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already checked out") {
                return Err(WorktreeError::BranchConflict(branch.to_string()));
            }
            return Err(WorktreeError::Git(format!(
                "Failed to create worktree: {stderr}"
            )));
        }

        self.get_info(name)
    }

    /// Remove a worktree
    ///
    /// This both deletes the worktree directory and prunes the git worktree record.
    pub fn remove(&self, name: &str) -> WorktreeResult<()> {
        let worktree_path = self.worktree_base.join(name);

        if !worktree_path.exists() {
            return Err(WorktreeError::NotFound(format!(
                "Worktree not found: {}",
                worktree_path.display()
            )));
        }

        let output = Command::new("git")
            .arg("-C")
            .arg(&self.repo_path)
            .arg("worktree")
            .arg("remove")
            .arg(&worktree_path)
            .output()
            .map_err(|e| WorktreeError::Git(format!("Failed to execute git: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorktreeError::Git(format!(
                "Failed to remove worktree: {stderr}"
            )));
        }

        Ok(())
    }

    /// List all worktrees in the repository
    pub fn list(&self) -> WorktreeResult<Vec<WorktreeInfo>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.repo_path)
            .arg("worktree")
            .arg("list")
            .arg("--porcelain")
            .output()
            .map_err(|e| WorktreeError::Git(format!("Failed to list worktrees: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorktreeError::Git(format!(
                "Failed to list worktrees: {stderr}"
            )));
        }

        self.parse_worktree_list(&output.stdout)
    }

    /// Get information about a specific worktree
    pub fn get_info(&self, name: &str) -> WorktreeResult<WorktreeInfo> {
        let worktrees = self.list()?;
        let worktree_path = self.worktree_base.join(name);
        let canonical = std::fs::canonicalize(&worktree_path)
            .unwrap_or(worktree_path)
            .to_string_lossy()
            .to_string();

        worktrees
            .into_iter()
            .find(|wt| {
                let wt_canonical = Path::new(&wt.path)
                    .canonicalize()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or(wt.path.clone());
                wt_canonical == canonical || wt.path == name || wt.branch == name
            })
            .ok_or_else(|| WorktreeError::NotFound(format!("Worktree '{name}' not found")))
    }

    /// Prune stale worktree records
    pub fn prune(&self) -> WorktreeResult<()> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.repo_path)
            .arg("worktree")
            .arg("prune")
            .output()
            .map_err(|e| WorktreeError::Git(format!("Failed to prune worktrees: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorktreeError::Git(format!(
                "Failed to prune worktrees: {stderr}"
            )));
        }

        Ok(())
    }

    /// Validate a worktree name
    fn validate_name(&self, name: &str) -> WorktreeResult<()> {
        if name.is_empty() {
            return Err(WorktreeError::InvalidName("Worktree name cannot be empty".to_string()));
        }
        if name.contains('/') || name.contains('\\') {
            return Err(WorktreeError::InvalidName(
                "Worktree name cannot contain path separators".to_string(),
            ));
        }
        if name.starts_with('.') {
            return Err(WorktreeError::InvalidName(
                "Worktree name cannot start with a dot".to_string(),
            ));
        }
        Ok(())
    }

    /// Parse the output of `git worktree list --porcelain`
    fn parse_worktree_list(&self, stdout: &[u8]) -> WorktreeResult<Vec<WorktreeInfo>> {
        let output = String::from_utf8_lossy(stdout);
        let mut worktrees = Vec::new();
        let mut current: Option<WorktreeInfo> = None;

        for line in output.lines() {
            if line.starts_with("worktree ") {
                if let Some(wt) = current.take() {
                    worktrees.push(wt);
                }
                let path = line.strip_prefix("worktree ").unwrap_or("").to_string();
                current = Some(WorktreeInfo {
                    path,
                    branch: String::new(),
                    head: String::new(),
                    status: WorktreeStatus::Clean,
                });
            } else if let Some(ref mut wt) = current {
                if line.starts_with("HEAD ") {
                    wt.head = line.strip_prefix("HEAD ").unwrap_or("").to_string();
                } else if line.starts_with("branch ") {
                    let ref_str = line.strip_prefix("branch ").unwrap_or("");
                    // Extract branch name from refs/heads/<branch>
                    wt.branch = ref_str
                        .strip_prefix("refs/heads/")
                        .unwrap_or(ref_str)
                        .to_string();
                } else if line.starts_with("bare") {
                    wt.status = WorktreeStatus::Bare;
                } else if line.starts_with("detached") {
                    // Detached HEAD, branch name is empty
                }
            }
        }

        if let Some(wt) = current.take() {
            worktrees.push(wt);
        }

        Ok(worktrees)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_repo() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().join("main-repo");
        fs::create_dir_all(&repo_path).unwrap();

        // Initialize git repo
        Command::new("git")
            .arg("init")
            .arg(&repo_path)
            .output()
            .unwrap();

        // Configure git user
        Command::new("git")
            .arg("-C")
            .arg(&repo_path)
            .args(["config", "user.email", "test@test.com"])
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(&repo_path)
            .args(["config", "user.name", "Test"])
            .output()
            .unwrap();

        // Create an initial commit
        let readme = repo_path.join("README.md");
        fs::write(&readme, "# Test").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(&repo_path)
            .args(["add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(&repo_path)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        (dir, repo_path)
    }

    #[test]
    fn test_worktree_manager_new() {
        let repo = Path::new("/tmp/repo");
        let base = Path::new("/tmp/worktrees");
        let manager = WorktreeManager::new(repo, base);
        assert_eq!(manager.repo_path, repo);
        assert_eq!(manager.worktree_base, base);
    }

    #[test]
    fn test_validate_name_empty() {
        let manager = WorktreeManager::new(Path::new("."), Path::new("."));
        assert!(manager.validate_name("").is_err());
    }

    #[test]
    fn test_validate_name_with_slash() {
        let manager = WorktreeManager::new(Path::new("."), Path::new("."));
        assert!(manager.validate_name("foo/bar").is_err());
    }

    #[test]
    fn test_validate_name_with_dot_prefix() {
        let manager = WorktreeManager::new(Path::new("."), Path::new("."));
        assert!(manager.validate_name(".hidden").is_err());
    }

    #[test]
    fn test_validate_name_valid() {
        let manager = WorktreeManager::new(Path::new("."), Path::new("."));
        assert!(manager.validate_name("feature-branch").is_ok());
    }

    #[test]
    fn test_parse_worktree_list() {
        let (_dir, repo_path) = setup_test_repo();
        let worktree_base = _dir.path().join("worktrees");
        let manager = WorktreeManager::new(&repo_path, &worktree_base);

        let porcelain_output = format!(
            "worktree {}\nHEAD {}\nbranch refs/heads/main\n\n",
            repo_path.display(),
            "abc123def456"
        );

        let worktrees = manager.parse_worktree_list(porcelain_output.as_bytes()).unwrap();
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].branch, "main");
        assert_eq!(worktrees[0].status, WorktreeStatus::Clean);
    }

    #[test]
    #[ignore = "requires git installed and correct default branch name"]
    fn test_create_worktree() {
        let (_dir, repo_path) = setup_test_repo();
        let worktree_base = _dir.path().join("worktrees");
        fs::create_dir_all(&worktree_base).unwrap();

        let manager = WorktreeManager::new(&repo_path, &worktree_base);
        let result = manager.create("feature-test", "main");
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.branch, "main");
    }
}
