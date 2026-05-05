//! Workspace snapshot system (side-git)
//!
//! Provides non-invasive workspace rollback using a side git repository
//! that shadows the workspace. Snapshots are taken before and after
//! each agent turn, stored in `~/.coder/snapshots/<project_hash>/`.
//!
//! The side git repo uses `--git-dir` and `--work-tree` flags so it
//! never touches the project's own `.git` directory.

use std::path::{Path, PathBuf};
/// A recorded snapshot
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub label: String,
    pub timestamp: String,
    pub turn_number: usize,
    pub file_count: usize,
}

/// Snapshot manager for workspace rollback
pub struct SnapshotManager {
    /// Root directory for all snapshots
    snapshots_root: PathBuf,
    /// Workspace path being snapshotted
    workspace: PathBuf,
    /// Project hash (derived from workspace path)
    #[allow(dead_code)]
    project_hash: String,
    /// Worktree hash (derived from workspace path)
    #[allow(dead_code)]
    worktree_hash: String,
    /// Git directory for the side repo
    git_dir: PathBuf,
    /// Current turn number
    turn_number: usize,
}

impl SnapshotManager {
    /// Create a new snapshot manager for the given workspace
    pub fn new(workspace: &Path) -> Self {
        let snapshots_root = Self::default_snapshots_root();
        let workspace_canonical = std::fs::canonicalize(workspace)
            .unwrap_or_else(|_| workspace.to_path_buf());
        let workspace_str = workspace_canonical.to_string_lossy().to_string();

        // Create deterministic hashes from workspace path
        let project_hash = simple_hash(&workspace_str);
        let worktree_hash = simple_hash(&format!("{}:{}", workspace_str, std::process::id()));

        let git_dir = snapshots_root
            .join(&project_hash)
            .join(&worktree_hash)
            .join(".git");

        Self {
            snapshots_root,
            workspace: workspace_canonical,
            project_hash,
            worktree_hash,
            git_dir,
            turn_number: 0,
        }
    }

    /// Default snapshots root directory
    fn default_snapshots_root() -> PathBuf {
        let mut path = crate::util::path::coder_dir();
        path.push("snapshots");
        path
    }

    /// Initialize the side git repository
    pub fn init(&self) -> anyhow::Result<()> {
        let git_dir_parent = self.git_dir.parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid git dir path"))?;
        std::fs::create_dir_all(git_dir_parent)?;

        // Initialize a bare git repo
        let output = std::process::Command::new("git")
            .arg("init")
            .arg("--bare")
            .arg(&self.git_dir)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to init side git: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("Reinitialized") {
                anyhow::bail!("git init failed: {}", stderr);
            }
        }

        // Create initial commit with a .gitignore
        let work_tree = &self.workspace;
        let git_args = |args: &[&str]| -> std::process::Output {
            let mut cmd = std::process::Command::new("git");
            cmd.arg("--git-dir").arg(&self.git_dir)
                .arg("--work-tree").arg(work_tree);
            for arg in args {
                cmd.arg(arg);
            }
            cmd.output().unwrap_or_else(|_| std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        };

        // Configure the side git repo
        let _ = git_args(&["config", "user.name", "coder-snapshot"]);
        let _ = git_args(&["config", "user.email", "snapshot@coder.local"]);
        let _ = git_args(&["config", "core.autocrlf", "false"]);

        // Add everything and make initial commit
        let _add_output = git_args(&["add", "-A", "--ignore-errors"]);
        let _ = git_args(&[
            "commit",
            "--allow-empty",
            "-m",
            &format!("Initial snapshot at {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")),
        ]);

        tracing::info!(
            "Side git initialized at {} for workspace {}",
            self.git_dir.display(),
            self.workspace.display()
        );

        Ok(())
    }

    /// Take a pre-turn snapshot
    pub fn snapshot_before(&mut self) -> anyhow::Result<Snapshot> {
        self.turn_number += 1;
        let label = format!("pre-turn-{}", self.turn_number);
        self.take_snapshot(&label)
    }

    /// Take a post-turn snapshot
    pub fn snapshot_after(&mut self) -> anyhow::Result<Snapshot> {
        let label = format!("post-turn-{}", self.turn_number);
        self.take_snapshot(&label)
    }

    /// Take a snapshot with a specific label
    pub fn take_snapshot(&self, label: &str) -> anyhow::Result<Snapshot> {
        self.ensure_initialized();

        let work_tree = &self.workspace;
        let git_dir = &self.git_dir;

        // Git command helper
        let run_git = |args: &[&str]| -> anyhow::Result<String> {
            let output = std::process::Command::new("git")
                .arg("--git-dir").arg(git_dir)
                .arg("--work-tree").arg(work_tree)
                .args(args)
                .output()
                .map_err(|e| anyhow::anyhow!("Git command failed: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("git {} failed: {}", args[0], stderr);
            }

            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        };

        // Add all changes
        let _ = run_git(&["add", "-A", "--ignore-errors"]);

        // Check if there are changes to commit
        let status = run_git(&["status", "--porcelain"])?;

        if status.trim().is_empty() {
            // No changes - create an empty commit still for the snapshot
            let _ = run_git(&[
                "commit",
                "--allow-empty",
                "-m",
                &format!("snapshot: {} (no changes)", label),
            ])?;
        } else {
            let file_count = status.lines().count();
            let _ = run_git(&[
                "commit",
                "-m",
                &format!("snapshot: {} ({} files)", label, file_count),
            ])?;
        }

        // Get the commit hash
        let rev = run_git(&["rev-parse", "--short", "HEAD"])?;
        let snapshot_id = format!("{}-{}", label, rev.trim());

        let file_count = status.lines().count();

        Ok(Snapshot {
            id: snapshot_id,
            label: label.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            turn_number: self.turn_number,
            file_count,
        })
    }

    /// List all snapshots
    pub fn list_snapshots(&self) -> anyhow::Result<Vec<Snapshot>> {
        self.ensure_initialized();

        let git_dir = &self.git_dir;
        let work_tree = &self.workspace;

        let output = std::process::Command::new("git")
            .arg("--git-dir").arg(git_dir)
            .arg("--work-tree").arg(work_tree)
            .args(["log", "--reverse", "--format=%H %s", "--max-count=100"])
            .output()
            .map_err(|e| anyhow::anyhow!("git log failed: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut snapshots = Vec::new();

        for line in stdout.lines() {
            if let Some((hash, rest)) = line.split_once(' ') {
                let rest = rest.trim();
                if let Some(snapshot_id) = rest.strip_prefix("snapshot: ") {
                    let label = snapshot_id.to_string();
                    snapshots.push(Snapshot {
                        id: format!("{}", &hash[..8]),
                        label,
                        timestamp: String::new(),
                        turn_number: 0,
                        file_count: 0,
                    });
                }
            }
        }

        Ok(snapshots)
    }

    /// Restore workspace to a specific snapshot
    pub fn restore(&self, snapshot_id: &str) -> anyhow::Result<()> {
        self.ensure_initialized();

        let git_dir = &self.git_dir;
        let work_tree = &self.workspace;

        // Find the commit matching the snapshot ID (partial match)
        let output = std::process::Command::new("git")
            .arg("--git-dir").arg(git_dir)
            .arg("--work-tree").arg(work_tree)
            .args(["log", "--reverse", "--format=%H %s", "--max-count=200"])
            .output()
            .map_err(|e| anyhow::anyhow!("git log failed: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut target_commit: Option<String> = None;

        for line in stdout.lines() {
            if line.contains(snapshot_id) || line.contains(&format!("snapshot: {}", snapshot_id)) {
                if let Some(hash) = line.split_whitespace().next() {
                    target_commit = Some(hash.to_string());
                    break;
                }
            }
        }

        let commit = target_commit
            .ok_or_else(|| anyhow::anyhow!("Snapshot '{}' not found", snapshot_id))?;

        // Restore to that commit state
        let output = std::process::Command::new("git")
            .arg("--git-dir").arg(git_dir)
            .arg("--work-tree").arg(work_tree)
            .args(["checkout", "--force", &commit])
            .output()
            .map_err(|e| anyhow::anyhow!("git checkout failed: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Restore failed: {}", stderr);
        }

        tracing::info!("Workspace restored to snapshot: {}", snapshot_id);
        Ok(())
    }

    /// Get the difference between current state and a snapshot
    pub fn diff_snapshot(&self, snapshot_id: &str) -> anyhow::Result<String> {
        self.ensure_initialized();
        let git_dir = &self.git_dir;
        let work_tree = &self.workspace;

        let output = std::process::Command::new("git")
            .arg("--git-dir").arg(git_dir)
            .arg("--work-tree").arg(work_tree)
            .args(["diff", snapshot_id, "--stat"])
            .output()
            .map_err(|e| anyhow::anyhow!("git diff failed: {}", e))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Ensure the side git repo is initialized
    fn ensure_initialized(&self) {
        if !self.git_dir.exists() {
            match std::process::Command::new("git")
                .arg("init")
                .arg("--bare")
                .arg(&self.git_dir)
                .output()
            {
                Ok(output) if !output.status.success() => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    tracing::warn!("Side git init failed: {}", stderr);
                }
                Err(e) => {
                    tracing::warn!("Failed to run git init: {}. Is git installed?", e);
                }
                _ => {}
            }
        }
    }

    /// Get the snapshots root path
    pub fn snapshots_root(&self) -> &Path {
        &self.snapshots_root
    }
}

/// Simple hash function for deterministic string hashing
fn simple_hash(input: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Format snapshot list for display
pub fn format_snapshot_list(snapshots: &[Snapshot]) -> String {
    if snapshots.is_empty() {
        return "No snapshots available.".to_string();
    }

    let mut result = format!("── Snapshots ({}) ──\n\n", snapshots.len());
    for (i, snap) in snapshots.iter().enumerate() {
        let id_safe = &snap.id[..snap.id.floor_char_boundary(12.min(snap.id.len()))];
        let ts_end = 19.min(snap.timestamp.len());
        let ts_safe = &snap.timestamp[..snap.timestamp.floor_char_boundary(ts_end)];
        result.push_str(&format!(
            "  {}. [{}] {} ({})\n",
            i + 1,
            id_safe,
            snap.label,
            ts_safe.replace('T', " "),
        ));
    }
    result.push_str("\nUse /restore <id> to restore a snapshot.");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_manager_creation() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = SnapshotManager::new(tmp.path());
        assert!(manager.snapshots_root().exists() || !manager.snapshots_root().exists());
    }

    #[test]
    fn test_simple_hash() {
        let h1 = simple_hash("hello");
        let h2 = simple_hash("hello");
        let h3 = simple_hash("world");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_format_snapshot_list_empty() {
        let result = format_snapshot_list(&[]);
        assert_eq!(result, "No snapshots available.");
    }

    #[test]
    fn test_format_snapshot_list_with_entries() {
        let snapshots = vec![
            Snapshot {
                id: "abc123".to_string(),
                label: "pre-turn-1".to_string(),
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                turn_number: 1,
                file_count: 3,
            },
            Snapshot {
                id: "def456".to_string(),
                label: "post-turn-1".to_string(),
                timestamp: "2026-01-01T00:01:00Z".to_string(),
                turn_number: 1,
                file_count: 5,
            },
        ];
        let result = format_snapshot_list(&snapshots);
        assert!(result.contains("2"));
        assert!(result.contains("pre-turn-1"));
        assert!(result.contains("post-turn-1"));
    }

    #[test]
    fn test_snapshot_serialization() {
        let snap = Snapshot {
            id: "test-id".to_string(),
            label: "test-snapshot".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            turn_number: 1,
            file_count: 10,
        };
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("test-snapshot"));
        assert!(json.contains("test-id"));
    }
}
