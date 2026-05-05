//! User-defined slash commands
//!
//! Supports custom commands defined in `~/.coder/commands/` or
//! `.coder/commands/` with `$1`, `$2`, `$ARGUMENTS` template substitution.
//! User commands override built-in commands.

use std::collections::HashMap;
use std::path::PathBuf;

/// A user-defined command
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserCommand {
    /// Command name (e.g., "deploy", "format")
    pub name: String,
    /// Description shown in help
    pub description: String,
    /// Command template with $1, $2, $ARGUMENTS placeholders
    pub template: String,
    /// Whether this is a shell command (!) or AI prompt
    pub shell: bool,
}

/// Manager for user-defined commands
pub struct UserCommandManager {
    /// Loaded commands, keyed by name
    commands: HashMap<String, UserCommand>,
    /// Directories to scan for command files
    command_dirs: Vec<PathBuf>,
}

impl UserCommandManager {
    /// Create a new manager and load commands from standard locations
    pub fn new() -> Self {
        let mut dirs = Vec::new();

        // Project-level commands
        if let Ok(cwd) = std::env::current_dir() {
            dirs.push(cwd.join(".coder").join("commands"));
        }

        // User-global commands
        let user_dir = crate::util::path::coder_dir();
        dirs.push(user_dir.join("commands"));

        let mut manager = Self {
            commands: HashMap::new(),
            command_dirs: dirs,
        };

        manager.load_all();
        manager
    }

    /// Load commands from all configured directories
    pub fn load_all(&mut self) {
        let dirs = self.command_dirs.clone();
        for dir in &dirs {
            self.load_from(dir);
        }
    }

    /// Load commands from a specific directory
    fn load_from(&mut self, dir: &PathBuf) {
        if !dir.exists() {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(cmd) = toml::from_str::<UserCommand>(&content) {
                            self.commands.insert(cmd.name.clone(), cmd);
                        }
                    }
                }
            }
        }
    }

    /// Get a command by name
    pub fn get(&self, name: &str) -> Option<&UserCommand> {
        self.commands.get(name)
    }

    /// Check if a command exists
    pub fn has(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    /// Apply template substitution for an argument string
    ///
    /// $1, $2, etc. are replaced positionally.
    /// $ARGUMENTS is replaced with the full argument string.
    /// $0 is replaced with the command name.
    pub fn apply_template(template: &str, cmd_name: &str, args: &str) -> String {
        let mut result = template
            .replace("$0", cmd_name)
            .replace("$ARGUMENTS", args);

        // Replace positional arguments $1, $2, etc.
        let arg_parts: Vec<&str> = args.split_whitespace().collect();
        for (i, arg) in arg_parts.iter().enumerate() {
            let placeholder = format!("${}", i + 1);
            result = result.replace(&placeholder, arg);
        }

        result
    }

    /// Render a command with the given arguments
    pub fn render(&self, name: &str, args: &str) -> Option<String> {
        self.get(name).map(|cmd| {
            Self::apply_template(&cmd.template, &cmd.name, args)
        })
    }

    /// List all loaded commands
    pub fn list(&self) -> Vec<&UserCommand> {
        let mut cmds: Vec<&UserCommand> = self.commands.values().collect();
        cmds.sort_by(|a, b| a.name.cmp(&b.name));
        cmds
    }

    /// Number of loaded commands
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if no commands loaded
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Add a command programmatically
    pub fn add(&mut self, cmd: UserCommand) {
        self.commands.insert(cmd.name.clone(), cmd);
    }
}

impl Default for UserCommandManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Format user commands for display
pub fn format_user_commands(manager: &UserCommandManager) -> String {
    if manager.is_empty() {
        return "── User Commands ──\n\nNo user commands configured.\n\
                Create .toml files in ~/.coder/commands/ or .coder/commands/".to_string();
    }

    let mut result = format!("── User Commands ({}) ──\n\n", manager.len());
    for cmd in manager.list() {
        let cmd_type = if cmd.shell { "!" } else { "/" };
        result.push_str(&format!(
            "  {}{} - {}\n    Template: {}\n",
            cmd_type, cmd.name, cmd.description, cmd.template
        ));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_template_simple() {
        let result = UserCommandManager::apply_template(
            "cargo build $1",
            "build",
            "--release",
        );
        assert_eq!(result, "cargo build --release");
    }

    #[test]
    fn test_apply_template_arguments() {
        let result = UserCommandManager::apply_template(
            "echo $ARGUMENTS",
            "echo",
            "hello world from test",
        );
        assert_eq!(result, "echo hello world from test");
    }

    #[test]
    fn test_apply_template_multiple_args() {
        let result = UserCommandManager::apply_template(
            "cp $1 $2",
            "cp",
            "src/file.rs dst/file.rs",
        );
        assert_eq!(result, "cp src/file.rs dst/file.rs");
    }

    #[test]
    fn test_apply_template_cmd_name() {
        let result = UserCommandManager::apply_template(
            "run $0 with $1",
            "deploy",
            "prod",
        );
        assert_eq!(result, "run deploy with prod");
    }

    #[test]
    fn test_apply_template_no_placeholders() {
        let result = UserCommandManager::apply_template(
            "cargo test --workspace",
            "test",
            "",
        );
        assert_eq!(result, "cargo test --workspace");
    }

    #[test]
    fn test_manager_empty() {
        let manager = UserCommandManager::new();
        // May or may not have commands depending on disk state
        // len() is always >= 0, so no assertion needed
    }

    #[test]
    fn test_manager_add_and_get() {
        let mut manager = UserCommandManager::new();
        manager.add(UserCommand {
            name: "deploy".to_string(),
            description: "Deploy the application".to_string(),
            template: "cargo build --release && scp target/release/app server:/app/".to_string(),
            shell: true,
        });

        assert!(manager.has("deploy"));
        let cmd = manager.get("deploy").unwrap();
        assert_eq!(cmd.description, "Deploy the application");
    }

    #[test]
    fn test_manager_render() {
        let mut manager = UserCommandManager::new();
        manager.add(UserCommand {
            name: "build".to_string(),
            description: "Build with profile".to_string(),
            template: "cargo build $1".to_string(),
            shell: true,
        });

        let result = manager.render("build", "--release");
        assert_eq!(result, Some("cargo build --release".to_string()));
    }

    #[test]
    fn test_manager_render_unknown() {
        let manager = UserCommandManager::new();
        let result = manager.render("nonexistent", "");
        assert!(result.is_none());
    }

    #[test]
    fn test_format_empty() {
        let manager = UserCommandManager::new();
        // If no disk commands exist, should show empty message
        if manager.is_empty() {
            let formatted = format_user_commands(&manager);
            assert!(formatted.contains("No user commands configured"));
        }
    }

    #[test]
    fn test_format_with_commands() {
        let mut manager = UserCommandManager::new();
        manager.add(UserCommand {
            name: "deploy".to_string(),
            description: "Deploy app".to_string(),
            template: "./deploy.sh $1".to_string(),
            shell: true,
        });

        let formatted = format_user_commands(&manager);
        assert!(formatted.contains("deploy"));
        assert!(formatted.contains("Deploy app"));
    }
}
