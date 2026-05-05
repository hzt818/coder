//! Help system - categorized help for slash commands
//!
//! Provides a centralized help system for all slash commands,
//! organized by category with search and formatting capabilities.
//!
//! # Theme compatibility
//!
//! Output from this module is plain text designed to be rendered by the
//! ratatui layer which applies colors from [`crate::tui::theme::AppTheme`].
//! The [`HelpRenderer`] struct bridges between help data and the theme system.

use crate::tui::theme::AppTheme;

/// Width of the separator line in characters
const SEPARATOR_WIDTH: usize = 50;

/// Categories of commands
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HelpCategory {
    /// Show all categories
    All,
    /// Informational commands (help, tools, model, context)
    Info,
    /// Git workflow commands (status, diff, commit, pr)
    Git,
    /// Search commands (search, web_search, fetch)
    Search,
    /// Action commands (clear, compact, review, plan, test, etc.)
    Action,
    /// Configuration commands (config, init, memory, quit)
    Config,
}

impl HelpCategory {
    /// Get the display label for this category
    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Info => "Info",
            Self::Git => "Git",
            Self::Search => "Search",
            Self::Action => "Action",
            Self::Config => "Config",
        }
    }

    /// Iterate over all non-All categories in display order
    pub fn all() -> impl Iterator<Item = HelpCategory> {
        [
            Self::Info,
            Self::Git,
            Self::Search,
            Self::Action,
            Self::Config,
        ]
        .into_iter()
    }
}

/// A single command's help entry
#[derive(Debug, Clone)]
pub struct HelpEntry {
    /// Primary command name (without leading slash)
    pub name: &'static str,
    /// Alternative names (shorter aliases)
    pub aliases: &'static [&'static str],
    /// One-line description of what the command does
    pub description: &'static str,
    /// Usage example showing arguments
    pub usage: &'static str,
    /// Which category this command belongs to
    pub category: HelpCategory,
}

// ── Static command registry ──

/// All registered help entries
const ALL_COMMANDS: &[HelpEntry] = &[
    // ── Info ──
    HelpEntry {
        name: "help",
        aliases: &["h"],
        description: "Show categorized help",
        usage: "/help [topic]",
        category: HelpCategory::Info,
    },
    HelpEntry {
        name: "tools",
        aliases: &["t"],
        description: "List all tools",
        usage: "/tools",
        category: HelpCategory::Info,
    },
    HelpEntry {
        name: "model",
        aliases: &["m"],
        description: "Show/switch AI model",
        usage: "/model [model_name]",
        category: HelpCategory::Info,
    },
    HelpEntry {
        name: "context",
        aliases: &["ctx"],
        description: "Show context usage",
        usage: "/context",
        category: HelpCategory::Info,
    },
    // ── Git ──
    HelpEntry {
        name: "status",
        aliases: &["st"],
        description: "Show git status",
        usage: "/status",
        category: HelpCategory::Git,
    },
    HelpEntry {
        name: "diff",
        aliases: &[],
        description: "Show git diff",
        usage: "/diff [path]",
        category: HelpCategory::Git,
    },
    HelpEntry {
        name: "commit",
        aliases: &[],
        description: "Create git commit",
        usage: "/commit <message>",
        category: HelpCategory::Git,
    },
    HelpEntry {
        name: "pr",
        aliases: &[],
        description: "Create pull request",
        usage: "/pr",
        category: HelpCategory::Git,
    },
    // ── Search ──
    HelpEntry {
        name: "search",
        aliases: &["s"],
        description: "Search codebase with ripgrep",
        usage: "/search <pattern> [path]",
        category: HelpCategory::Search,
    },
    HelpEntry {
        name: "web_search",
        aliases: &["ws"],
        description: "Search the web",
        usage: "/web_search <query>",
        category: HelpCategory::Search,
    },
    HelpEntry {
        name: "fetch",
        aliases: &[],
        description: "Fetch a web page",
        usage: "/fetch <url>",
        category: HelpCategory::Search,
    },
    // ── Action ──
    HelpEntry {
        name: "clear",
        aliases: &["c"],
        description: "Clear conversation",
        usage: "/clear",
        category: HelpCategory::Action,
    },
    HelpEntry {
        name: "compact",
        aliases: &[],
        description: "Compact conversation context",
        usage: "/compact",
        category: HelpCategory::Action,
    },
    HelpEntry {
        name: "summarize",
        aliases: &[],
        description: "Summarize conversation",
        usage: "/summarize",
        category: HelpCategory::Action,
    },
    HelpEntry {
        name: "review",
        aliases: &["r"],
        description: "Review code changes",
        usage: "/review",
        category: HelpCategory::Action,
    },
    HelpEntry {
        name: "plan",
        aliases: &[],
        description: "Create implementation plan",
        usage: "/plan <description>",
        category: HelpCategory::Action,
    },
    HelpEntry {
        name: "test",
        aliases: &[],
        description: "Add tests",
        usage: "/test [path]",
        category: HelpCategory::Action,
    },
    HelpEntry {
        name: "lint",
        aliases: &[],
        description: "Check code quality",
        usage: "/lint [path]",
        category: HelpCategory::Action,
    },
    HelpEntry {
        name: "fix",
        aliases: &[],
        description: "Fix issues",
        usage: "/fix [issue]",
        category: HelpCategory::Action,
    },
    HelpEntry {
        name: "explain",
        aliases: &[],
        description: "Explain code",
        usage: "/explain <path>",
        category: HelpCategory::Action,
    },
    HelpEntry {
        name: "doc",
        aliases: &[],
        description: "Add documentation",
        usage: "/doc [path]",
        category: HelpCategory::Action,
    },
    // ── Config ──
    HelpEntry {
        name: "config",
        aliases: &[],
        description: "View/set configuration",
        usage: "/config [key] [value]",
        category: HelpCategory::Config,
    },
    HelpEntry {
        name: "init",
        aliases: &[],
        description: "Initialize coder config",
        usage: "/init",
        category: HelpCategory::Config,
    },
    HelpEntry {
        name: "memory",
        aliases: &[],
        description: "Show memory/sessions",
        usage: "/memory",
        category: HelpCategory::Config,
    },
    HelpEntry {
        name: "quit",
        aliases: &["q", "exit"],
        description: "Exit coder",
        usage: "/quit",
        category: HelpCategory::Config,
    },
];

/// Usage examples shown in detailed command help
const COMMAND_EXAMPLES: &[(&str, &[&str])] = &[
    ("help", &["/help", "/help git", "/help /commit"]),
    ("tools", &["/tools"]),
    ("model", &["/model", "/model claude-sonnet-4-6"]),
    ("context", &["/context"]),
    ("status", &["/status"]),
    ("diff", &["/diff", "/diff src/main.rs"]),
    ("commit", &["/commit Add login feature"]),
    ("pr", &["/pr"]),
    ("search", &["/search fn main", "/search TODO src/"]),
    ("web_search", &["/web_search Rust async patterns"]),
    ("fetch", &["/fetch https://example.com"]),
    ("clear", &["/clear"]),
    ("compact", &["/compact"]),
    ("summarize", &["/summarize"]),
    ("review", &["/review"]),
    ("plan", &["/plan Add authentication"]),
    ("test", &["/test", "/test src/auth.rs"]),
    ("lint", &["/lint", "/lint src/"]),
    ("fix", &["/fix unused variables"]),
    ("explain", &["/explain src/main.rs"]),
    ("doc", &["/doc", "/doc src/lib.rs"]),
    ("config", &["/config", "/config theme dark"]),
    ("init", &["/init"]),
    ("memory", &["/memory"]),
    ("quit", &["/quit"]),
];

// ── Public API ──

/// Get all help entries as a vector of clones from the static registry.
pub fn all_entries() -> Vec<HelpEntry> {
    ALL_COMMANDS.to_vec()
}

/// Get detailed help for a specific command.
///
/// Returns `None` if the command is not found.
/// Accepts command names with or without a leading `/`.
pub fn get_help(cmd: &str) -> Option<String> {
    let entry = find_entry(cmd)?;

    let header_dashes = "─".repeat(SEPARATOR_WIDTH.saturating_sub(entry.name.len() + 8));

    let aliases = if entry.aliases.is_empty() {
        "(none)".to_string()
    } else {
        entry.aliases.join(", ")
    };

    let examples = COMMAND_EXAMPLES
        .iter()
        .find(|(name, _)| *name == entry.name)
        .map(|(_, ex)| *ex)
        .unwrap_or(&[]);

    let mut result = format!(
        "── /{} ─{}──\n\
         Aliases:      {}\n\
         Category:     {}\n\
         Description:  {}\n\
         Usage:        {}\n",
        entry.name,
        header_dashes,
        aliases,
        entry.category.label(),
        entry.description,
        entry.usage,
    );

    if !examples.is_empty() {
        result.push_str("\n  Examples:\n");
        for ex in examples {
            result.push_str(&format!("    {}\n", ex));
        }
    }

    Some(result)
}

/// Format help for a category as a formatted string.
///
/// If `HelpCategory::All` is passed, delegates to [`format_all`].
pub fn format_category(category: HelpCategory) -> String {
    match category {
        HelpCategory::All => format_all(),
        cat => {
            let label = cat.label();
            let dashes = "─".repeat(SEPARATOR_WIDTH.saturating_sub(label.len() + 6));
            let mut result = format!("── {} ─{}──\n", label, dashes);

            let entries: Vec<_> = ALL_COMMANDS.iter().filter(|e| e.category == cat).collect();
            if entries.is_empty() {
                result.push_str("  No commands in this category.\n");
            } else {
                for entry in &entries {
                    result.push_str(&format_entry_line(entry));
                }
            }

            result
        }
    }
}

/// Format full help with all categories.
pub fn format_all() -> String {
    let main_dashes = "─".repeat(SEPARATOR_WIDTH.saturating_sub(8));
    let mut result = format!("── Help ─{}──\n\n", main_dashes);

    for cat in HelpCategory::all() {
        let label = cat.label();
        let dashes = "─".repeat(SEPARATOR_WIDTH.saturating_sub(label.len() + 6));
        result.push_str(&format!("── {} ─{}──\n", label, dashes));

        for entry in ALL_COMMANDS.iter().filter(|e| e.category == cat) {
            result.push_str(&format_entry_line(entry));
        }

        result.push('\n');
    }

    result
}

/// Search help for a keyword.
///
/// Matches against command names, aliases, and descriptions (case-insensitive).
pub fn search(query: &str) -> String {
    let q = query.to_lowercase();
    let matches: Vec<_> = ALL_COMMANDS
        .iter()
        .filter(|e| {
            e.name.contains(&q)
                || e.aliases.iter().any(|a| a.contains(&q))
                || e.description.to_lowercase().contains(&q)
        })
        .collect();

    if matches.is_empty() {
        return format!("No help entries matching '{}'.\n", query);
    }

    let query_dashes = SEPARATOR_WIDTH.saturating_sub(query.len() + 24);
    let mut result = format!(
        "── Help Results for \"{}\" ─{}──\n",
        query,
        "─".repeat(query_dashes),
    );

    for entry in &matches {
        result.push_str(&format_entry_line(entry));
    }

    result
}

// ── Theme-aware rendering ──

/// Help text renderer that bridges help data with [`AppTheme`].
///
/// Colors are applied by the ratatui rendering layer rather than through
/// ANSI escape codes embedded in strings. This struct ensures compatibility
/// between the help data and the theme system.
pub struct HelpRenderer<'a> {
    /// The theme to use for rendering
    theme: &'a AppTheme,
}

impl<'a> HelpRenderer<'a> {
    /// Create a new renderer with the given theme
    pub fn new(theme: &'a AppTheme) -> Self {
        Self { theme }
    }

    /// Render full categorized help text
    pub fn render_all(&self) -> String {
        format_all()
    }

    /// Render help for a single category
    pub fn render_category(&self, category: HelpCategory) -> String {
        format_category(category)
    }

    /// Render detailed help for a specific command
    pub fn render_command(&self, cmd: &str) -> Option<String> {
        get_help(cmd)
    }

    /// Get the theme reference
    pub fn theme(&self) -> &'a AppTheme {
        self.theme
    }
}

// ── Internal helpers ──

/// Find a help entry by command name (with or without leading /)
fn find_entry(cmd: &str) -> Option<&'static HelpEntry> {
    let cmd = cmd.trim_start_matches('/');
    ALL_COMMANDS
        .iter()
        .find(|e| e.name == cmd || e.aliases.contains(&cmd))
}

/// Check whether a given keybinding is covered by the help system.
///
/// Currently always returns `true`; this function exists so that
/// [`CommandPalette`](crate::tui::command_palette::CommandPalette)
/// can query whether a keybinding (e.g. `"Ctrl+K"`) has corresponding
/// help content. Future implementations may perform a lookup against
/// a keybinding registry.
pub fn has_keybinding(_keybinding: &str) -> bool {
    true
}

/// Format a single entry as a one-line summary
fn format_entry_line(entry: &HelpEntry) -> String {
    let cmd = if entry.aliases.is_empty() {
        format!("/{}", entry.name)
    } else {
        format!("/{} ({})", entry.name, entry.aliases.join(", "))
    };
    format!("  {:<24}{}\n", cmd, entry.description)
}
