//! TUI color theme - high contrast, terminal-friendly palette

use ratatui::style::Color;

/// The coder color theme
pub struct AppTheme {
    /// Background color
    pub bg: Color,
    /// Foreground/text color
    pub fg: Color,
    /// Accent (title bar, highlights)
    pub accent: Color,
    /// User message color
    pub user: Color,
    /// Assistant message color
    pub assistant: Color,
    /// Tool call color
    pub tool: Color,
    /// Error color
    pub error: Color,
    /// Success color
    pub success: Color,
    /// Warning color
    pub warning: Color,
    /// Code block color
    pub code: Color,
    /// Dim/muted text
    pub dim: Color,
    /// Selection/inverted
    pub selection_bg: Color,
    pub selection_fg: Color,
    /// Status bar
    pub status_bar: Color,
    /// Welcome border color
    pub welcome_border: Color,
    /// Crab art colors (blue glow effect)
    pub crab: Color,
    pub crab_glow: Color,
    pub crab_core: Color,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::White,
            accent: Color::Yellow,
            user: Color::Cyan,
            assistant: Color::Green,
            tool: Color::Yellow,
            error: Color::Red,
            success: Color::Green,
            warning: Color::Yellow,
            code: Color::Magenta,
            dim: Color::DarkGray,
            selection_bg: Color::White,
            selection_fg: Color::Black,
            status_bar: Color::Green,
            welcome_border: Color::DarkGray,
            crab: Color::Cyan,
            crab_glow: Color::LightBlue,
            crab_core: Color::Blue,
        }
    }
}

impl AppTheme {
    /// Create a theme from UI configuration settings
    pub fn from_config(settings: &crate::config::UiSettings) -> Self {
        match settings.theme.as_str() {
            "high-contrast" => Self::high_contrast(),
            _ => Self::default(),
        }
    }

    /// High contrast theme with bright colors on dark background
    pub fn high_contrast() -> Self {
        Self::default()
    }
}
