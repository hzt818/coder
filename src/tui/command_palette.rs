//! Command palette - searchable list of commands/actions
//!
//! Provides a Ctrl+K keyboard shortcut to show a searchable list of
//! built-in commands and actions. Users can filter by typing a query,
//! navigate with arrow keys, and press Enter to execute the selected item.

use crate::tui::help::has_keybinding;

/// A single selectable item in the command palette
#[derive(Debug, Clone)]
pub struct CommandItem {
    /// Display name of the command
    pub name: &'static str,
    /// Short description of what the command does
    pub description: &'static str,
    /// The action string returned on execution (e.g., "/help")
    pub action: &'static str,
}

/// All built-in commands shown in the palette
const BUILTIN_COMMANDS: &[CommandItem] = &[
    CommandItem {
        name: "Help",
        description: "Show help",
        action: "/help",
    },
    CommandItem {
        name: "Tools",
        description: "List available tools",
        action: "/tools",
    },
    CommandItem {
        name: "Mode",
        description: "Switch interaction mode",
        action: "/mode",
    },
    CommandItem {
        name: "Model",
        description: "Show model info",
        action: "/model",
    },
    CommandItem {
        name: "Clear",
        description: "Clear conversation",
        action: "/clear",
    },
    CommandItem {
        name: "Compact",
        description: "Compact context",
        action: "/compact",
    },
    CommandItem {
        name: "Cost",
        description: "Show cost report",
        action: "/cost",
    },
    CommandItem {
        name: "Checkpoint",
        description: "Show checkpoint status",
        action: "/checkpoint",
    },
];

/// A searchable command palette overlay
#[derive(Debug, Clone)]
pub struct CommandPalette {
    /// Whether the palette is currently visible
    pub visible: bool,
    /// The current filter query typed by the user
    pub query: String,
    /// All available command items
    pub items: Vec<CommandItem>,
    /// Index of the currently selected item in the filtered list
    pub selected: usize,
}

impl CommandPalette {
    /// Create a new command palette with built-in commands
    pub fn new() -> Self {
        Self {
            visible: false,
            query: String::new(),
            items: BUILTIN_COMMANDS.to_vec(),
            selected: 0,
        }
    }

    /// Toggle the palette visibility and reset state when opening
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.query.clear();
            self.selected = 0;
        }
    }

    /// Filter items by substring match against name or description.
    ///
    /// Returns items whose name or description contains the query string
    /// (case-insensitive). An empty query returns all items.
    pub fn filter(&self) -> Vec<&CommandItem> {
        if self.query.is_empty() {
            return self.items.iter().collect();
        }
        let q = self.query.to_lowercase();
        self.items
            .iter()
            .filter(|item| {
                item.name.to_lowercase().contains(&q)
                    || item.description.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Move selection to the next item in the filtered list (wraps around)
    pub fn select_next(&mut self) {
        let filtered = self.filter();
        if filtered.is_empty() {
            self.selected = 0;
            return;
        }
        self.selected = (self.selected + 1) % filtered.len();
    }

    /// Move selection to the previous item in the filtered list (wraps around)
    pub fn select_prev(&mut self) {
        let filtered = self.filter();
        if filtered.is_empty() {
            self.selected = 0;
            return;
        }
        if self.selected == 0 {
            self.selected = filtered.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    /// Execute the currently selected item and return its action string.
    ///
    /// Returns `None` if the filtered list is empty or the selection
    /// index is out of bounds.
    pub fn execute(&self) -> Option<String> {
        let filtered = self.filter();
        filtered.get(self.selected).map(|item| item.action.to_string())
    }

    /// Check whether a given keybinding (e.g., "Ctrl+K") is covered
    /// by the help system.
    pub fn is_keybinding_covered(&self, keybinding: &str) -> bool {
        has_keybinding(keybinding)
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_creation() {
        let palette = CommandPalette::new();
        assert!(!palette.visible);
        assert!(palette.query.is_empty());
        assert_eq!(palette.items.len(), 8);
        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn test_toggle() {
        let mut palette = CommandPalette::new();

        // Initially hidden
        assert!(!palette.visible);

        // Toggle to visible
        palette.toggle();
        assert!(palette.visible);
        assert!(palette.query.is_empty());
        assert_eq!(palette.selected, 0);

        // Set query and select an item, then toggle off
        palette.query = "help".to_string();
        palette.selected = 2;
        palette.toggle();
        assert!(!palette.visible);

        // Toggle back on — state should be reset
        palette.toggle();
        assert!(palette.visible);
        assert!(palette.query.is_empty());
        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn test_empty_query_shows_all_items() {
        let palette = CommandPalette::new();
        let filtered = palette.filter();
        assert_eq!(filtered.len(), 8);
        // Verify all 8 built-in commands are present
        let names: Vec<&str> = filtered.iter().map(|item| item.name).collect();
        assert!(names.contains(&"Help"));
        assert!(names.contains(&"Tools"));
        assert!(names.contains(&"Mode"));
        assert!(names.contains(&"Model"));
        assert!(names.contains(&"Clear"));
        assert!(names.contains(&"Compact"));
        assert!(names.contains(&"Cost"));
        assert!(names.contains(&"Checkpoint"));
    }

    #[test]
    fn test_filtering_by_name() {
        let palette = CommandPalette {
            query: "help".to_string(),
            ..CommandPalette::new()
        };
        let filtered = palette.filter();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Help");
        assert_eq!(filtered[0].action, "/help");
    }

    #[test]
    fn test_filtering_by_description() {
        let palette = CommandPalette {
            query: "cost".to_string(),
            ..CommandPalette::new()
        };
        let filtered = palette.filter();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Cost");
    }

    #[test]
    fn test_filtering_case_insensitive() {
        let palette = CommandPalette {
            query: "CLEAR".to_string(),
            ..CommandPalette::new()
        };
        let filtered = palette.filter();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Clear");
    }

    #[test]
    fn test_filtering_partial_match() {
        let palette = CommandPalette {
            query: "co".to_string(),
            ..CommandPalette::new()
        };
        let filtered = palette.filter();
        // Should match "Cost" (name), "Compact" (name), and "Clear" (description: "Clear conversation")
        let names: Vec<&str> = filtered.iter().map(|item| item.name).collect();
        assert!(names.contains(&"Cost"), "Cost should match 'co'");
        assert!(names.contains(&"Compact"), "Compact should match 'co'");
        assert!(names.contains(&"Clear"), "Clear describes 'Clear conversation' which has 'co'");
        assert_eq!(filtered.len(), 3, "Cost, Compact, and Clear match 'co'");
    }

    #[test]
    fn test_filtering_no_matches() {
        let palette = CommandPalette {
            query: "zzzzz".to_string(),
            ..CommandPalette::new()
        };
        let filtered = palette.filter();
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_select_next() {
        let mut palette = CommandPalette::new();
        assert_eq!(palette.selected, 0);

        palette.select_next();
        assert_eq!(palette.selected, 1);
    }

    #[test]
    fn test_select_next_wraps_around() {
        let mut palette = CommandPalette::new();
        // Navigate to the last item
        for _ in 0..palette.items.len() - 1 {
            palette.select_next();
        }
        assert_eq!(palette.selected, palette.items.len() - 1);

        // Next should wrap to 0
        palette.select_next();
        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn test_select_prev() {
        let mut palette = CommandPalette::new();
        palette.selected = 3;

        palette.select_prev();
        assert_eq!(palette.selected, 2);
    }

    #[test]
    fn test_select_prev_wraps_around() {
        let mut palette = CommandPalette::new();
        palette.selected = 0;

        palette.select_prev();
        assert_eq!(palette.selected, palette.items.len() - 1);
    }

    #[test]
    fn test_select_next_with_empty_filtered() {
        let mut palette = CommandPalette {
            query: "zzzzz".to_string(),
            selected: 0,
            ..CommandPalette::new()
        };
        // Should not panic and reset to 0
        palette.select_next();
        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn test_select_prev_with_empty_filtered() {
        let mut palette = CommandPalette {
            query: "zzzzz".to_string(),
            selected: 0,
            ..CommandPalette::new()
        };
        // Should not panic and reset to 0
        palette.select_prev();
        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn test_execute_returns_correct_action() {
        let palette = CommandPalette {
            query: String::new(),
            ..CommandPalette::new()
        };
        // Select "Help" (index 0)
        assert_eq!(palette.execute(), Some("/help".to_string()));

        // Navigate to "Clear" (index 4)
        let test_cases = &[
            (1, "/tools"),
            (2, "/mode"),
            (3, "/model"),
            (4, "/clear"),
            (5, "/compact"),
            (6, "/cost"),
            (7, "/checkpoint"),
        ];
        for (idx, expected_action) in test_cases {
            assert_eq!(
                CommandPalette {
                    selected: *idx,
                    ..palette.clone()
                }
                .execute(),
                Some(expected_action.to_string()),
                "Expected action for index {}",
                idx
            );
        }
    }

    #[test]
    fn test_execute_with_empty_filtered_list() {
        let palette = CommandPalette {
            query: "zzzzz".to_string(),
            selected: 0,
            ..CommandPalette::new()
        };
        assert_eq!(palette.execute(), None);
    }

    #[test]
    fn test_execute_with_out_of_bounds_selection() {
        let palette = CommandPalette {
            query: String::new(),
            selected: 999,
            ..CommandPalette::new()
        };
        assert_eq!(palette.execute(), None);
    }

    #[test]
    fn test_all_command_items_have_unique_actions() {
        let palette = CommandPalette::new();
        let mut actions: Vec<&str> = palette.items.iter().map(|item| item.action).collect();
        actions.sort();
        actions.dedup();
        // If dedup didn't remove anything, all actions are unique
        assert_eq!(actions.len(), palette.items.len());
    }

    #[test]
    fn test_is_keybinding_covered() {
        let palette = CommandPalette::new();
        // This should call through to help::has_keybinding
        // Currently has_keybinding always returns true
        assert!(palette.is_keybinding_covered("Ctrl+K"));
    }
}
