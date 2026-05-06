//! Theme and color configuration for the TUI

use serde::{Deserialize, Serialize};

/// Color theme definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
}

/// Theme color palette
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    pub primary: String,
    pub secondary: String,
    pub success: String,
    pub error: String,
    pub warning: String,
    pub info: String,
    pub user_message: String,
    pub assistant_message: String,
    pub tool_call: String,
    pub code_block: String,
    pub selection: String,
    pub background: String,
    pub foreground: String,
    pub dim_text: String,
    pub status_bar: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self::coder_dark()
    }
}

impl Theme {
    /// Default dark theme with high-contrast colors
    pub fn coder_dark() -> Self {
        Self {
            name: "coder-dark".to_string(),
            colors: ThemeColors {
                primary: "#00FFAA".into(),   // Bright green
                secondary: "#00AAFF".into(), // Bright cyan
                success: "#00FF00".into(),
                error: "#FF3333".into(),
                warning: "#FFAA00".into(),
                info: "#8888FF".into(),
                user_message: "#00DDFF".into(),      // Cyan for user
                assistant_message: "#00FF88".into(), // Green for AI
                tool_call: "#FFDD00".into(),         // Yellow for tools
                code_block: "#FF66FF".into(),        // Magenta for code
                selection: "#FFFFFF".into(),         // White
                background: "#1A1A2E".into(),
                foreground: "#E0E0E0".into(),
                dim_text: "#666666".into(),
                status_bar: "#00FFAA".into(),
            },
        }
    }
}
