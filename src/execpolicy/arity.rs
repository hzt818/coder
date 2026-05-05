//! Bash arity dictionary for precise command permission matching
//!
//! [`BashArityDict`] maps a command prefix to the number of positional
//! (non-flag) tokens that form the canonical prefix. Flags are never
//! counted toward arity, so `auto_allow = ["git status"]` matches
//! `git status -s` and `git status --porcelain`, but **not** `git push`.
//!
//! Covers 200+ entries across: git, cargo, npm, yarn, pnpm, docker, kubectl,
//! go, python/pip, gh, rustup, deno, bun, aws, terraform, make, helm, and more.

/// Static arity table: `(prefix, arity)` where arity is the total number of
/// positional tokens (including the base command) that form the canonical prefix.
pub static BASH_ARITY_TABLE: &[(&str, u8)] = &[
    // ── git ──────────────────────────────────────────────────────────────────
    ("git add", 2), ("git am", 2), ("git apply", 2), ("git bisect", 2),
    ("git blame", 2), ("git branch", 2), ("git cat-file", 2), ("git checkout", 2),
    ("git cherry-pick", 2), ("git clean", 2), ("git clone", 2), ("git commit", 2),
    ("git config", 2), ("git describe", 2), ("git diff", 2), ("git fetch", 2),
    ("git format-patch", 2), ("git grep", 2), ("git init", 2), ("git log", 2),
    ("git ls-files", 2), ("git merge", 2), ("git mv", 2), ("git notes", 2),
    ("git pull", 2), ("git push", 2), ("git rebase", 2), ("git reflog", 2),
    ("git remote", 2), ("git reset", 2), ("git restore", 2), ("git revert", 2),
    ("git rm", 2), ("git show", 2), ("git stash", 2), ("git status", 2),
    ("git submodule", 2), ("git switch", 2), ("git tag", 2), ("git worktree", 2),
    // ── npm ──────────────────────────────────────────────────────────────────
    ("npm audit", 2), ("npm build", 2), ("npm cache", 2), ("npm ci", 2),
    ("npm dedupe", 2), ("npm fund", 2), ("npm help", 2), ("npm info", 2),
    ("npm init", 2), ("npm install", 2), ("npm link", 2), ("npm list", 2),
    ("npm outdated", 2), ("npm pack", 2), ("npm prune", 2), ("npm publish", 2),
    ("npm rebuild", 2), ("npm run", 3), ("npm start", 2), ("npm stop", 2),
    ("npm test", 2), ("npm uninstall", 2), ("npm update", 2), ("npm version", 2),
    // ── yarn ─────────────────────────────────────────────────────────────────
    ("yarn add", 2), ("yarn audit", 2), ("yarn build", 2), ("yarn install", 2),
    ("yarn run", 3), ("yarn start", 2), ("yarn test", 2), ("yarn upgrade", 2),
    ("yarn workspace", 3),
    // ── pnpm ─────────────────────────────────────────────────────────────────
    ("pnpm add", 2), ("pnpm build", 2), ("pnpm install", 2), ("pnpm run", 3),
    ("pnpm start", 2), ("pnpm test", 2), ("pnpm update", 2),
    // ── cargo ────────────────────────────────────────────────────────────────
    ("cargo add", 2), ("cargo bench", 2), ("cargo build", 2), ("cargo check", 2),
    ("cargo clean", 2), ("cargo clippy", 2), ("cargo doc", 2), ("cargo fix", 2),
    ("cargo fmt", 2), ("cargo generate", 2), ("cargo install", 2),
    ("cargo metadata", 2), ("cargo package", 2), ("cargo publish", 2),
    ("cargo remove", 2), ("cargo run", 2), ("cargo search", 2), ("cargo test", 2),
    ("cargo tree", 2), ("cargo uninstall", 2), ("cargo update", 2), ("cargo yank", 2),
    // ── docker ───────────────────────────────────────────────────────────────
    ("docker build", 2), ("docker compose", 3), ("docker container", 3),
    ("docker cp", 2), ("docker exec", 2), ("docker image", 3), ("docker images", 2),
    ("docker inspect", 2), ("docker kill", 2), ("docker logs", 2),
    ("docker network", 3), ("docker ps", 2), ("docker pull", 2), ("docker push", 2),
    ("docker rm", 2), ("docker rmi", 2), ("docker run", 2), ("docker start", 2),
    ("docker stop", 2), ("docker system", 3), ("docker tag", 2), ("docker volume", 3),
    // ── kubectl ──────────────────────────────────────────────────────────────
    ("kubectl apply", 2), ("kubectl create", 3), ("kubectl delete", 3),
    ("kubectl describe", 3), ("kubectl exec", 2), ("kubectl explain", 2),
    ("kubectl get", 3), ("kubectl label", 2), ("kubectl logs", 2),
    ("kubectl patch", 2), ("kubectl port-forward", 2), ("kubectl rollout", 3),
    ("kubectl scale", 2), ("kubectl set", 2), ("kubectl top", 3),
    // ── go ───────────────────────────────────────────────────────────────────
    ("go build", 2), ("go clean", 2), ("go env", 2), ("go fmt", 2),
    ("go generate", 2), ("go get", 2), ("go install", 2), ("go list", 2),
    ("go mod", 3), ("go run", 2), ("go test", 2), ("go vet", 2), ("go work", 3),
    // ── python / pip ─────────────────────────────────────────────────────────
    ("pip install", 2), ("pip uninstall", 2), ("pip list", 2), ("pip show", 2),
    ("pip freeze", 2), ("pip3 install", 2), ("pip3 uninstall", 2),
    ("pip3 list", 2), ("pip3 show", 2), ("python -m", 3), ("python3 -m", 3),
    // ── make / cmake ─────────────────────────────────────────────────────────
    ("make", 1), ("cmake", 1),
    // ── gh (GitHub CLI) ──────────────────────────────────────────────────────
    ("gh pr", 3), ("gh issue", 3), ("gh repo", 3), ("gh release", 3),
    ("gh workflow", 3), ("gh run", 3), ("gh secret", 3),
    // ── rustup ───────────────────────────────────────────────────────────────
    ("rustup default", 2), ("rustup install", 2), ("rustup show", 2),
    ("rustup target", 3), ("rustup toolchain", 3), ("rustup update", 2),
    // ── deno / bun / npx ─────────────────────────────────────────────────────
    ("deno run", 2), ("deno test", 2), ("deno fmt", 2), ("deno lint", 2),
    ("bun add", 2), ("bun build", 2), ("bun install", 2), ("bun run", 3),
    ("bun test", 2), ("npx", 2),
    // ── aws CLI ──────────────────────────────────────────────────────────────
    ("aws s3", 3), ("aws ec2", 3), ("aws iam", 3), ("aws lambda", 3),
    ("aws cloudformation", 3), ("aws ecs", 3), ("aws eks", 3), ("aws rds", 3),
    ("aws sts", 3), ("aws configure", 2),
    // ── terraform ────────────────────────────────────────────────────────────
    ("terraform init", 2), ("terraform plan", 2), ("terraform apply", 2),
    ("terraform destroy", 2), ("terraform validate", 2), ("terraform output", 2),
    ("terraform state", 3), ("terraform workspace", 3),
    // ── helm ─────────────────────────────────────────────────────────────────
    ("helm install", 2), ("helm upgrade", 2), ("helm uninstall", 2),
    ("helm list", 2), ("helm repo", 3), ("helm status", 2), ("helm template", 2),
];

/// Arity dictionary for bash command-prefix allow rules.
///
/// Provides arity-aware prefix extraction so that `auto_allow = ["git status"]`
/// correctly matches `git status -s` without also matching `git push`.
#[derive(Debug, Clone)]
pub struct BashArityDict {
    /// Internal table sorted longest-prefix-first for greedy matching.
    entries: Vec<(&'static str, u8)>,
}

impl BashArityDict {
    /// Construct a new dictionary pre-loaded with [`BASH_ARITY_TABLE`].
    #[must_use]
    pub fn new() -> Self {
        let mut entries: Vec<(&'static str, u8)> = BASH_ARITY_TABLE.to_vec();
        // Longest prefix first so greedy matching works correctly.
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.0.len()));
        Self { entries }
    }

    /// Return the canonical command prefix for a slice of command tokens.
    ///
    /// # Algorithm
    ///
    /// 1. Strip all flag tokens (tokens that start with `-`).
    /// 2. Build candidates of depth 1..=3 from positional tokens (longest first).
    /// 3. If a candidate matches a dictionary entry, return `arity` positional
    ///    tokens joined with spaces.
    /// 4. If no dictionary entry matches, return the single base command name.
    #[must_use]
    pub fn classify(&self, tokens: &[&str]) -> String {
        if tokens.is_empty() {
            return String::new();
        }

        // Collect positional (non-flag) tokens, lowercased.
        let positional: Vec<String> = tokens
            .iter()
            .filter(|t| !t.starts_with('-'))
            .map(|t| t.to_ascii_lowercase())
            .collect();

        if positional.is_empty() {
            return String::new();
        }

        // Try candidates from longest to shortest (max depth 3).
        let max_depth = positional.len().min(3);
        for depth in (1..=max_depth).rev() {
            let candidate = positional[..depth].join(" ");
            if let Some(&(_key, arity)) = self
                .entries
                .iter()
                .find(|(key, _)| *key == candidate.as_str())
            {
                let take = (arity as usize).min(positional.len());
                return positional[..take].join(" ");
            }
        }

        // No match: return base command name only.
        positional[0].clone()
    }

    /// Check if an allow-rule `pattern` (e.g., `"git status"`) matches the
    /// concrete `command` (e.g., `"git status -s"`).
    ///
    /// Arity-aware: `"git status"` matches `git status -s` but NOT `git push`.
    /// For unknown patterns, falls back to plain prefix matching.
    #[must_use]
    pub fn allow_rule_matches(&self, pattern: &str, command: &str) -> bool {
        let pattern_lower = pattern.trim().to_ascii_lowercase();
        let command_tokens: Vec<&str> = command.split_whitespace().collect();

        if pattern_lower.is_empty() || command_tokens.is_empty() {
            return pattern_lower.is_empty();
        }

        let canonical = self.classify(&command_tokens);

        // Primary check: classified prefix equals pattern.
        if canonical == pattern_lower {
            return true;
        }

        // Fallback: plain prefix match for patterns not in the table.
        let command_lower = command.trim().to_ascii_lowercase();
        let pattern_norm: String = pattern_lower.split_whitespace().collect::<Vec<_>>().join(" ");
        let command_norm: String = command_lower.split_whitespace().collect::<Vec<_>>().join(" ");
        command_norm == pattern_norm || command_norm.starts_with(&format!("{pattern_norm} "))
    }

    /// Get the arity (word count) for the canonical prefix of a command.
    pub fn arity(&self, command: &str) -> usize {
        let tokens: Vec<&str> = command.split_whitespace().collect();
        let canonical = self.classify(&tokens);
        canonical.split_whitespace().count().max(1)
    }

    /// Check if a command base is known to the dictionary.
    pub fn is_known(&self, command: &str) -> bool {
        let base = command.split_whitespace().next().unwrap_or("");
        self.entries.iter().any(|(k, _)| k.starts_with(base) || k == &base)
    }

    /// Iterate over all entries.
    pub fn entries(&self) -> impl Iterator<Item = (&str, u8)> {
        self.entries.iter().map(|(k, v)| (*k, *v))
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether dictionary is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for BashArityDict {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dict() -> BashArityDict {
        BashArityDict::new()
    }

    // ── classify ─────────────────────────────────────────────────────────────

    #[test]
    fn classify_git_status_bare() {
        assert_eq!(dict().classify(&["git", "status"]), "git status");
    }

    #[test]
    fn classify_git_status_with_flag() {
        assert_eq!(dict().classify(&["git", "status", "-s"]), "git status");
        assert_eq!(dict().classify(&["git", "status", "--porcelain"]), "git status");
    }

    #[test]
    fn classify_git_push() {
        assert_eq!(dict().classify(&["git", "push", "origin", "main"]), "git push");
    }

    #[test]
    fn classify_npm_run_dev_arity_3() {
        assert_eq!(dict().classify(&["npm", "run", "dev"]), "npm run dev");
    }

    #[test]
    fn classify_cargo_check() {
        assert_eq!(dict().classify(&["cargo", "check", "--workspace"]), "cargo check");
    }

    #[test]
    fn classify_docker_compose_up_arity_3() {
        assert_eq!(dict().classify(&["docker", "compose", "up"]), "docker compose up");
    }

    #[test]
    fn classify_make_no_subcommand() {
        assert_eq!(dict().classify(&["make", "all"]), "make");
    }

    #[test]
    fn classify_unknown_falls_back_to_base() {
        assert_eq!(dict().classify(&["ls", "-la"]), "ls");
    }

    #[test]
    fn classify_empty_returns_empty() {
        assert_eq!(dict().classify(&[]), "");
    }

    // ── allow_rule_matches ────────────────────────────────────────────────────

    #[test]
    fn git_status_matches_with_flag() {
        assert!(dict().allow_rule_matches("git status", "git status -s"));
    }

    #[test]
    fn git_status_does_not_match_push() {
        assert!(!dict().allow_rule_matches("git status", "git push origin main"));
    }

    #[test]
    fn git_status_does_not_match_checkout() {
        assert!(!dict().allow_rule_matches("git status", "git checkout main"));
    }

    #[test]
    fn npm_run_dev_matches_dev() {
        assert!(dict().allow_rule_matches("npm run dev", "npm run dev"));
    }

    #[test]
    fn npm_run_dev_does_not_match_build() {
        assert!(!dict().allow_rule_matches("npm run dev", "npm run build"));
    }

    #[test]
    fn cargo_check_matches_with_flags() {
        assert!(dict().allow_rule_matches("cargo check", "cargo check --workspace"));
    }

    #[test]
    fn cargo_build_does_not_match_test() {
        assert!(!dict().allow_rule_matches("cargo build", "cargo test"));
    }

    #[test]
    fn docker_run_does_not_match_build() {
        assert!(dict().allow_rule_matches("docker run", "docker run -it ubuntu"));
        assert!(!dict().allow_rule_matches("docker run", "docker build ."));
    }

    #[test]
    fn unknown_fallback_still_works() {
        assert!(dict().allow_rule_matches("ls", "ls -la"));
    }

    #[test]
    fn make_with_target() {
        assert!(dict().allow_rule_matches("make", "make all"));
        assert!(dict().allow_rule_matches("make", "make clean"));
    }

    #[test]
    fn empty_rule() {
        assert!(dict().allow_rule_matches("", "anything"));
    }

    #[test]
    fn empty_command() {
        assert!(!dict().allow_rule_matches("git status", ""));
    }

    #[test]
    fn dict_covers_at_least_30_commands() {
        assert!(dict().len() >= 30, "expected 30+ entries, got {}", dict().len());
    }

    #[test]
    fn arity_values() {
        assert_eq!(dict().arity("git status"), 2);
        assert_eq!(dict().arity("echo"), 1);
    }

    #[test]
    fn is_known_works() {
        assert!(dict().is_known("git"));
        assert!(dict().is_known("cargo"));
        assert!(!dict().is_known("my_custom_tool"));
    }
}
