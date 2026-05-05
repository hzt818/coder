//! TUI interface - terminal user interface built with ratatui
//!
//! Layout:
//! ┌─────────────────────────────────────────────┐
//! │  🦀 coder v0.1.0          model: claude     │  Title bar
//! ├─────────────────────────────────────────────┤
//! │                                             │
//! │  ┌─ Message ────────────────────────────┐   │
//! │  │ user text...                         │   │  Chat panel
//! │  └──────────────────────────────────────┘   │
//! │  ┌─ Tool ───────────────────────────────┐   │
//! │  │ tool output...                       │   │
//! │  └──────────────────────────────────────┘   │
//! │                                             │
//! ├─────────────────────────────────────────────┤
//! │ > input text                      [hint]   │  Input
//! ├─────────────────────────────────────────────┤
//! │ 🦀 tools:8 | session:active | tokens:1.2k  │  Status bar
//! └─────────────────────────────────────────────┘

pub mod app;
pub mod ui;
pub mod chat_panel;
pub mod input;
pub mod status_bar;
pub mod detail_popup;
pub mod mention_popup;
pub mod help;
pub mod theme;
pub mod syntax;
pub mod dialog_provider_setup;

pub use app::App;

use anyhow::Result;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::stdout;

/// Initialize the terminal for TUI mode
pub fn init_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide,
        crossterm::event::EnableMouseCapture,
    )?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal after TUI mode
pub fn restore_terminal() -> Result<()> {
    crossterm::execute!(
        stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show,
        crossterm::event::DisableMouseCapture,
    )?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}
