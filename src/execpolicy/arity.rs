//! Bash arity dictionary for precise command permission matching
//!
//! Allows `auto_allow = ["git status"]` to match `git status -s`
//! but NOT `git push`. Built-in arity tables cover common CLI tools:
//! git, cargo, npm, yarn, pnpm, docker, kubectl, aws, make, and more.

use std::collections::HashMap;

/// Bash arity dictionary for matching command prefixes
pub struct BashArityDict {
    /// Known commands and their subcommand depth
    /// e.g., "git status" has arity 2 (git + status)
    known_commands: HashMap<String, usize>,
}

impl BashArityDict {
    /// Create a new dictionary with built-in command table
    pub fn new() -> Self {
        Self {
            known_commands: build_arity_table(),
        }
    }

    /// Check if an allow rule matches a command considering arity
    ///
    /// For example, rule "git status" (2 words) should match "git status -s"
    /// but NOT "git push" because the first 2 words don't match.
    ///
    /// For unknown commands (not in the arity table), falls back to
    /// simple prefix matching.
    pub fn allow_rule_matches(&self, rule_prefix: &str, command: &str) -> bool {
        let rule_words: Vec<&str> = rule_prefix.split_whitespace().collect();
        if rule_words.is_empty() {
            return true;
        }

        let cmd_words: Vec<&str> = command.split_whitespace().collect();
        if cmd_words.is_empty() {
            return false;
        }

        // Determine the arity (significant words) for this rule
        // If the rule command is in our dictionary, use its arity
        let rule_base = rule_words[0];
        let arity = self.known_commands.get(rule_base).copied().unwrap_or(1);

        // For commands in the dictionary, check up to the arity depth
        // For commands not in the dictionary, simple prefix match
        if self.known_commands.contains_key(rule_base) {
            // Match the first `arity` words
            let max_words = arity.min(rule_words.len()).min(cmd_words.len());
            for i in 0..max_words {
                if rule_words[i] != cmd_words[i] {
                    return false;
                }
            }
            true
        } else {
            // Legacy flat prefix match for unknown commands
            command.starts_with(rule_prefix)
        }
    }

    /// Get the arity (subcommand depth) for a base command
    pub fn arity(&self, command: &str) -> usize {
        self.known_commands.get(command).copied().unwrap_or(1)
    }

    /// Check if a command is known to the dictionary
    pub fn is_known(&self, command: &str) -> bool {
        self.known_commands.contains_key(command)
    }
}

impl Default for BashArityDict {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the built-in arity table for common CLI tools
fn build_arity_table() -> HashMap<String, usize> {
    let mut m = HashMap::new();

    // Git: "git status", "git push", "git commit" etc. — arity 2
    m.insert("git".to_string(), 2);

    // Cargo: "cargo build", "cargo test", "cargo check" etc. — arity 2
    m.insert("cargo".to_string(), 2);

    // npm: "npm install", "npm run", "npm test" etc. — arity 2
    m.insert("npm".to_string(), 2);

    // Yarn: "yarn add", "yarn install", "yarn run" etc. — arity 2
    m.insert("yarn".to_string(), 2);

    // pnpm: "pnpm add", "pnpm install" — arity 2
    m.insert("pnpm".to_string(), 2);

    // Docker: "docker run", "docker build", "docker ps" — arity 2
    m.insert("docker".to_string(), 2);

    // Docker Compose: "docker compose up", "docker compose down" — arity 2
    // (special case: docker-compose is its own binary with arity 1)
    m.insert("docker-compose".to_string(), 1);

    // kubectl: "kubectl get", "kubectl apply" — arity 2
    m.insert("kubectl".to_string(), 2);

    // AWS CLI: "aws s3", "aws ec2" — arity 2
    m.insert("aws".to_string(), 2);

    // Make: "make build", "make test" — arity 2
    m.insert("make".to_string(), 2);

    // Rust toolchain
    m.insert("rustc".to_string(), 1);
    m.insert("rustup".to_string(), 2);

    // Python
    m.insert("pip".to_string(), 2);
    m.insert("pip3".to_string(), 2);
    m.insert("python".to_string(), 1);
    m.insert("python3".to_string(), 1);
    m.insert("poetry".to_string(), 2);

    // Node.js
    m.insert("node".to_string(), 1);
    m.insert("npx".to_string(), 1);

    // System tools
    m.insert("ls".to_string(), 1);
    m.insert("cat".to_string(), 1);
    m.insert("cd".to_string(), 1);
    m.insert("rm".to_string(), 1);
    m.insert("cp".to_string(), 1);
    m.insert("mv".to_string(), 1);
    m.insert("mkdir".to_string(), 1);
    m.insert("chmod".to_string(), 1);
    m.insert("chown".to_string(), 1);
    m.insert("echo".to_string(), 1);
    m.insert("curl".to_string(), 1);
    m.insert("wget".to_string(), 1);

    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_status_matches() {
        let dict = BashArityDict::new();
        assert!(dict.allow_rule_matches("git status", "git status"));
        assert!(dict.allow_rule_matches("git status", "git status -s"));
        assert!(dict.allow_rule_matches("git status", "git status --short"));
    }

    #[test]
    fn test_git_status_does_not_match_push() {
        let dict = BashArityDict::new();
        assert!(!dict.allow_rule_matches("git status", "git push"));
        assert!(!dict.allow_rule_matches("git status", "git push origin main"));
    }

    #[test]
    fn test_cargo_build_matches() {
        let dict = BashArityDict::new();
        assert!(dict.allow_rule_matches("cargo build", "cargo build"));
        assert!(dict.allow_rule_matches("cargo build", "cargo build --release"));
        assert!(dict.allow_rule_matches("cargo build", "cargo build --features test"));
    }

    #[test]
    fn test_cargo_build_does_not_match_test() {
        let dict = BashArityDict::new();
        assert!(!dict.allow_rule_matches("cargo build", "cargo test"));
        assert!(!dict.allow_rule_matches("cargo build", "cargo clippy"));
    }

    #[test]
    fn test_docker_run_does_not_match_build() {
        let dict = BashArityDict::new();
        assert!(dict.allow_rule_matches("docker run", "docker run -it ubuntu"));
        assert!(!dict.allow_rule_matches("docker run", "docker build ."));
    }

    #[test]
    fn test_unknown_command_fallback() {
        let dict = BashArityDict::new();
        // 'echo' is known (arity 1), so 'echo hello' matches 'echo'
        assert!(dict.allow_rule_matches("echo", "echo hello"));
    }

    #[test]
    fn test_npm_install() {
        let dict = BashArityDict::new();
        assert!(dict.allow_rule_matches("npm install", "npm install"));
        assert!(dict.allow_rule_matches("npm install", "npm install --save-dev"));
        assert!(!dict.allow_rule_matches("npm install", "npm run build"));
    }

    #[test]
    fn test_kubectl_get() {
        let dict = BashArityDict::new();
        assert!(dict.allow_rule_matches("kubectl get", "kubectl get pods"));
        assert!(!dict.allow_rule_matches("kubectl get", "kubectl apply -f deploy.yaml"));
    }

    #[test]
    fn test_aws_s3() {
        let dict = BashArityDict::new();
        assert!(dict.allow_rule_matches("aws s3", "aws s3 ls"));
        assert!(!dict.allow_rule_matches("aws s3", "aws ec2 describe-instances"));
    }

    #[test]
    fn test_empty_rule() {
        let dict = BashArityDict::new();
        assert!(dict.allow_rule_matches("", "anything"));
    }

    #[test]
    fn test_empty_command() {
        let dict = BashArityDict::new();
        assert!(!dict.allow_rule_matches("git status", ""));
    }

    #[test]
    fn test_arity_values() {
        let dict = BashArityDict::new();
        assert_eq!(dict.arity("git"), 2);
        assert_eq!(dict.arity("echo"), 1);
        assert_eq!(dict.arity("unknown_cmd"), 1); // default arity
    }

    #[test]
    fn test_is_known() {
        let dict = BashArityDict::new();
        assert!(dict.is_known("git"));
        assert!(dict.is_known("cargo"));
        assert!(dict.is_known("docker"));
        assert!(!dict.is_known("my_custom_tool"));
    }
}
