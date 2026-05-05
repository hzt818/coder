//! Vim modal editing support for the message composer.
//!
//! Provides a `VimState` that tracks Insert/Normal mode and translates
//! normal-mode key presses into editor actions (movement, deletion, etc.).
//!
//! # Examples (run with `cargo test --doc`)
//!
//! ```ignore
//! use crate::tui::vim::{VimMode, VimState, Action};
//!
//! let mut state = VimState::new();
//! assert!(state.is_insert());
//!
//! state.toggle();
//! assert_eq!(state.mode, VimMode::Normal);
//! ```

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Vim editing mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VimMode {
    /// Insert mode — keys insert characters into the buffer.
    Insert,
    /// Normal mode — keys trigger navigation / editing commands.
    Normal,
}

/// An action to be performed by the TUI event loop in response to a
/// normal-mode key press.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// No-op (e.g., unbound key).
    NoAction,
    /// Move cursor left  (`h` or Left arrow).
    MoveLeft,
    /// Move cursor right (`l` or Right arrow).
    MoveRight,
    /// Move up / history back (`k` or Up arrow).
    MoveUp,
    /// Move down / history forward (`j` or Down arrow).
    MoveDown,
    /// Move to beginning of the line (`0` or Home).
    MoveHome,
    /// Move to end of the line (`$` or End).
    MoveEnd,
    /// Delete the character under the cursor (`x`).
    DeleteChar,
    /// Delete the character before the cursor (`X`).
    Backspace,
    /// Delete from cursor to end of line (`D`).
    DeleteToEnd,
    /// Delete the entire input line (`dd`).
    DeleteLine,
    /// Insert a single character into the buffer.
    InsertChar(char),
    /// Enter insert mode at the current cursor position (`i`, `a`, `I`, `A`, `o`, `O`).
    EnterInsertMode,
    /// Leave input mode entirely (`Esc` when already in normal mode).
    EnterNormalMode,
    /// Submit / send the current message (`Enter`).
    Submit,
}

/// Tracks the state needed for Vim modal editing inside the message composer.
#[derive(Debug, Clone)]
pub struct VimState {
    /// Current mode (Insert or Normal).
    pub mode: VimMode,
    /// Accumulated keys for multi-key commands (e.g., `dd`, `dw`).
    pending: String,
}

impl VimState {
    /// Create a new `VimState` starting in Insert mode.
    pub fn new() -> Self {
        Self {
            mode: VimMode::Insert,
            pending: String::new(),
        }
    }

    /// Toggle between Insert and Normal mode.
    ///
    /// Clears any partially-entered multi-key sequence.
    pub fn toggle(&mut self) {
        self.mode = match self.mode {
            VimMode::Insert => VimMode::Normal,
            VimMode::Normal => VimMode::Insert,
        };
        self.pending.clear();
    }

    /// Returns `true` when the editor is in Insert mode.
    pub fn is_insert(&self) -> bool {
        self.mode == VimMode::Insert
    }

    /// Switch to Insert mode and clear any pending multi-key sequence.
    pub fn enter_insert_mode(&mut self) {
        self.mode = VimMode::Insert;
        self.pending.clear();
    }

    /// Switch to Normal mode and clear any pending multi-key sequence.
    pub fn enter_normal_mode(&mut self) {
        self.mode = VimMode::Normal;
        self.pending.clear();
    }

    /// Process a key event while in Normal mode.
    ///
    /// Returns a `Vec<Action>` — most single-key commands produce one action,
    /// but some composite commands (e.g., `I` → MoveHome + EnterInsertMode)
    /// return multiple.
    ///
    /// Returns an empty `Vec` for unhandled keys or for Ctrl/Alt-prefixed keys
    /// that should fall through to the default handler.
    pub fn handle_normal_key(&mut self, key: KeyEvent) -> Vec<Action> {
        // Pass Ctrl/Alt combinations through to the default handler.
        if key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
            return Vec::new();
        }

        // ── Multi-key sequence resolving ──
        if !self.pending.is_empty() {
            return self.resolve_pending(key);
        }

        match key.code {
            // ── Movement ──
            KeyCode::Char('h') | KeyCode::Left => vec![Action::MoveLeft],
            KeyCode::Char('j') | KeyCode::Down => vec![Action::MoveDown],
            KeyCode::Char('k') | KeyCode::Up => vec![Action::MoveUp],
            KeyCode::Char('l') | KeyCode::Right => vec![Action::MoveRight],
            KeyCode::Char('0') | KeyCode::Home => vec![Action::MoveHome],
            KeyCode::Char('$') | KeyCode::End => vec![Action::MoveEnd],

            // ── Insertion commands ──
            // i  → enter insert mode at current position
            KeyCode::Char('i') => vec![Action::EnterInsertMode],
            // I  → go to start of line, then enter insert
            KeyCode::Char('I') => vec![Action::MoveHome, Action::EnterInsertMode],
            // a  → advance one char, then enter insert
            KeyCode::Char('a') => vec![Action::MoveRight, Action::EnterInsertMode],
            // A  → go to end of line, then enter insert
            KeyCode::Char('A') => vec![Action::MoveEnd, Action::EnterInsertMode],
            // o  → open line below (append newline at end, then insert)
            KeyCode::Char('o') => {
                vec![Action::MoveEnd, Action::InsertChar('\n'), Action::EnterInsertMode]
            }
            // O  → open line above (insert newline at start, then insert)
            KeyCode::Char('O') => {
                vec![
                    Action::MoveHome,
                    Action::InsertChar('\n'),
                    Action::MoveLeft,
                    Action::EnterInsertMode,
                ]
            }

            // ── Deletion ──
            // x  → delete character under cursor
            KeyCode::Char('x') => vec![Action::DeleteChar],
            // X  → delete character before cursor (backspace)
            KeyCode::Char('X') => vec![Action::Backspace],
            // D  → delete from cursor to end of line
            KeyCode::Char('D') => vec![Action::DeleteToEnd],
            // d  → start multi-key delete sequence (dd, dw, etc.)
            KeyCode::Char('d') => {
                self.pending.push('d');
                Vec::new() // wait for second key
            }

            // ── Submit ──
            KeyCode::Enter => vec![Action::Submit],

            // ── Normal mode no-op ──
            // Esc in normal mode does nothing by itself; the outer handler
            // may choose to switch to AppMode::Normal instead.
            KeyCode::Esc => vec![Action::EnterNormalMode],

            // ── Everything else is unhandled ──
            _ => Vec::new(),
        }
    }

    /// Resolve a multi-key command once the second key arrives.
    fn resolve_pending(&mut self, key: KeyEvent) -> Vec<Action> {
        let sequence = self.pending.clone();
        self.pending.clear();

        match sequence.as_str() {
            // dd  → delete entire line
            "d" => match key.code {
                KeyCode::Char('d') => vec![Action::DeleteLine],
                _ => Vec::new(),
            },
            _ => Vec::new(),
        }
    }
}

impl Default for VimState {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Initial state ─────────────────────────────────────────────

    #[test]
    fn test_new_insert_mode() {
        let state = VimState::new();
        assert_eq!(state.mode, VimMode::Insert);
        assert!(state.is_insert());
    }

    #[test]
    fn test_default_is_insert() {
        let state = VimState::default();
        assert_eq!(state.mode, VimMode::Insert);
    }

    // ── Mode switching ────────────────────────────────────────────

    #[test]
    fn test_toggle_switches_mode() {
        let mut state = VimState::new();

        // Insert → Normal
        state.toggle();
        assert_eq!(state.mode, VimMode::Normal);
        assert!(!state.is_insert());

        // Normal → Insert
        state.toggle();
        assert_eq!(state.mode, VimMode::Insert);
        assert!(state.is_insert());
    }

    #[test]
    fn test_enter_insert_mode() {
        let mut state = VimState::new();
        state.enter_normal_mode();
        assert_eq!(state.mode, VimMode::Normal);

        state.enter_insert_mode();
        assert_eq!(state.mode, VimMode::Insert);
        assert!(state.is_insert());
    }

    #[test]
    fn test_enter_normal_mode() {
        let mut state = VimState::new();
        assert_eq!(state.mode, VimMode::Insert);

        state.enter_normal_mode();
        assert_eq!(state.mode, VimMode::Normal);
        assert!(!state.is_insert());
    }

    // ── Toggle clears pending state ──────────────────────────────

    #[test]
    fn test_toggle_clears_pending() {
        let mut state = VimState::new();
        state.enter_normal_mode();

        // Start a multi-key sequence
        let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        let actions = state.handle_normal_key(key);
        assert!(actions.is_empty()); // waiting for second key
        assert_eq!(state.pending, "d");

        // Toggle should clear pending
        state.toggle();
        assert_eq!(state.pending, "");

        // Back in normal mode — try again, pending should be empty
        state.toggle();
        assert_eq!(state.pending, "");
    }

    // ── Movement keys ─────────────────────────────────────────────

    #[test]
    fn test_movement_keys() {
        let mut state = VimState::new();
        state.enter_normal_mode();

        let checks = [
            (KeyCode::Char('h'), Action::MoveLeft),
            (KeyCode::Char('j'), Action::MoveDown),
            (KeyCode::Char('k'), Action::MoveUp),
            (KeyCode::Char('l'), Action::MoveRight),
            (KeyCode::Char('0'), Action::MoveHome),
            (KeyCode::Char('$'), Action::MoveEnd),
        ];

        for (code, expected) in &checks {
            let ev = KeyEvent::new(*code, KeyModifiers::NONE);
            let actions = state.handle_normal_key(ev);
            assert_eq!(actions.len(), 1, "key {:?} should produce 1 action", code);
            assert_eq!(actions[0], *expected, "key {:?}", code);
        }
    }

    // ── Insertion commands ────────────────────────────────────────

    #[test]
    fn test_insert_commands_single() {
        let mut state = VimState::new();
        state.enter_normal_mode();

        // i → EnterInsertMode
        let ev = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
        assert_eq!(
            state.handle_normal_key(ev),
            vec![Action::EnterInsertMode],
        );
    }

    #[test]
    fn test_insert_commands_composite() {
        let mut state = VimState::new();
        state.enter_normal_mode();

        // I → MoveHome + EnterInsertMode
        let ev = KeyEvent::new(KeyCode::Char('I'), KeyModifiers::NONE);
        assert_eq!(
            state.handle_normal_key(ev),
            vec![Action::MoveHome, Action::EnterInsertMode],
        );

        // Reset
        state.handle_normal_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        state.enter_normal_mode();

        // a → MoveRight + EnterInsertMode
        let ev = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(
            state.handle_normal_key(ev),
            vec![Action::MoveRight, Action::EnterInsertMode],
        );

        state.enter_normal_mode();

        // A → MoveEnd + EnterInsertMode
        let ev = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE);
        assert_eq!(
            state.handle_normal_key(ev),
            vec![Action::MoveEnd, Action::EnterInsertMode],
        );
    }

    #[test]
    fn test_open_line_below() {
        let mut state = VimState::new();
        state.enter_normal_mode();

        let ev = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE);
        assert_eq!(
            state.handle_normal_key(ev),
            vec![
                Action::MoveEnd,
                Action::InsertChar('\n'),
                Action::EnterInsertMode,
            ],
        );
    }

    #[test]
    fn test_open_line_above() {
        let mut state = VimState::new();
        state.enter_normal_mode();

        let ev = KeyEvent::new(KeyCode::Char('O'), KeyModifiers::NONE);
        assert_eq!(
            state.handle_normal_key(ev),
            vec![
                Action::MoveHome,
                Action::InsertChar('\n'),
                Action::MoveLeft,
                Action::EnterInsertMode,
            ],
        );
    }

    // ── Deletion keys ─────────────────────────────────────────────

    #[test]
    fn test_deletion_keys() {
        let mut state = VimState::new();
        state.enter_normal_mode();

        // x → DeleteChar
        let ev = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        assert_eq!(state.handle_normal_key(ev), vec![Action::DeleteChar]);

        // X → Backspace
        let ev = KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE);
        assert_eq!(state.handle_normal_key(ev), vec![Action::Backspace]);

        // D → DeleteToEnd
        let ev = KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE);
        assert_eq!(state.handle_normal_key(ev), vec![Action::DeleteToEnd]);
    }

    // ── Multi-key delete (dd) ─────────────────────────────────────

    #[test]
    fn test_dd_sequence_deletes_line() {
        let mut state = VimState::new();
        state.enter_normal_mode();

        // First 'd' — should wait
        let first = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        let actions = state.handle_normal_key(first);
        assert!(actions.is_empty());
        assert_eq!(state.pending, "d");

        // Second 'd' — should produce DeleteLine
        let second = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        let actions = state.handle_normal_key(second);
        assert_eq!(actions, vec![Action::DeleteLine]);

        // Pending should be cleared
        assert_eq!(state.pending, "");
    }

    #[test]
    fn test_incomplete_sequence_is_abandoned() {
        let mut state = VimState::new();
        state.enter_normal_mode();

        // First 'd' starts a sequence
        let first = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        state.handle_normal_key(first);
        assert_eq!(state.pending, "d");

        // Pressing an unrelated key abandons the sequence
        let unrelated = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
        let actions = state.handle_normal_key(unrelated);
        assert!(actions.is_empty());
        assert_eq!(state.pending, "");
    }

    // ── Submit ────────────────────────────────────────────────────

    #[test]
    fn test_enter_submits() {
        let mut state = VimState::new();
        state.enter_normal_mode();

        let ev = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(state.handle_normal_key(ev), vec![Action::Submit]);
    }

    // ── Esc exits to AppMode::Normal ──────────────────────────────

    #[test]
    fn test_esc_exits_input_mode() {
        let mut state = VimState::new();
        state.enter_normal_mode();

        let ev = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(
            state.handle_normal_key(ev),
            vec![Action::EnterNormalMode],
        );
    }

    // ── Ctrl keys pass through ─────────────────────────────────

    #[test]
    fn test_ctrl_keys_pass_through() {
        let mut state = VimState::new();
        state.enter_normal_mode();

        // Ctrl+C → empty (pass through)
        let ev = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(state.handle_normal_key(ev).is_empty());

        // Ctrl+O → empty (pass through)
        let ev = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL);
        assert!(state.handle_normal_key(ev).is_empty());
    }

    // ── Arrow keys aliases ─────────────────────────────────────

    #[test]
    fn test_arrow_keys() {
        let mut state = VimState::new();
        state.enter_normal_mode();

        assert_eq!(
            state.handle_normal_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
            vec![Action::MoveLeft],
        );
        assert_eq!(
            state.handle_normal_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
            vec![Action::MoveRight],
        );
        assert_eq!(
            state.handle_normal_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            vec![Action::MoveUp],
        );
        assert_eq!(
            state.handle_normal_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            vec![Action::MoveDown],
        );
        assert_eq!(
            state.handle_normal_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
            vec![Action::MoveHome],
        );
        assert_eq!(
            state.handle_normal_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
            vec![Action::MoveEnd],
        );
    }

    // ── Unhandled keys return empty ─────────────────────────────

    #[test]
    fn test_unhandled_keys_return_empty() {
        let mut state = VimState::new();
        state.enter_normal_mode();

        let unhandled = [
            KeyCode::Char('z'),
            KeyCode::Char('v'),
            KeyCode::Char('r'),
            KeyCode::Char('p'),
            KeyCode::Char('y'),
            KeyCode::F(1),
            KeyCode::Tab,
            KeyCode::Backspace, // Backspace is handled by X, not by regular key
            KeyCode::Delete,
        ];

        for code in &unhandled {
            let ev = KeyEvent::new(*code, KeyModifiers::NONE);
            assert!(
                state.handle_normal_key(ev).is_empty(),
                "key {:?} should be unhandled",
                code,
            );
        }
    }
}
