//! Keyboard input functionality using the `enigo` crate

use enigo::{Direction, Enigo, Keyboard as EnigoKeyboard, Settings};

use super::{ComputerError, ComputerResult};

/// Controls keyboard input
#[derive(Debug)]
pub struct KeyboardController {
    /// Inner enigo instance for platform-level control
    enigo: Enigo,
}

impl Default for KeyboardController {
    fn default() -> Self {
        Self {
            enigo: Enigo::new(&Settings::default())
                .expect("Failed to initialize enigo keyboard control"),
        }
    }
}

impl KeyboardController {
    /// Create a new KeyboardController
    ///
    /// # Panics
    /// Panics if the platform's keyboard control cannot be initialized.
    pub fn new() -> Self {
        Self::default()
    }

    /// Type a string of text using the keyboard
    pub fn type_text(&mut self, text: &str) -> ComputerResult<()> {
        self.enigo
            .text(text)
            .map_err(|e| ComputerError::Keyboard(format!("Failed to type text: {e}")))?;
        Ok(())
    }

    /// Press a single key
    pub fn press_key(&mut self, key: enigo::Key) -> ComputerResult<()> {
        self.enigo
            .key(key, Direction::Press)
            .map_err(|e| ComputerError::Keyboard(format!("Failed to press key: {e}")))?;
        Ok(())
    }

    /// Release a single key
    pub fn release_key(&mut self, key: enigo::Key) -> ComputerResult<()> {
        self.enigo
            .key(key, Direction::Release)
            .map_err(|e| ComputerError::Keyboard(format!("Failed to release key: {e}")))?;
        Ok(())
    }

    /// Tap a single key (press and release)
    pub fn tap_key(&mut self, key: enigo::Key) -> ComputerResult<()> {
        self.enigo
            .key(key, Direction::Click)
            .map_err(|e| ComputerError::Keyboard(format!("Failed to tap key: {e}")))?;
        Ok(())
    }

    /// Hold a modifier key and tap another key (e.g., Ctrl+C)
    pub fn key_combo(&mut self, modifier: enigo::Key, key: enigo::Key) -> ComputerResult<()> {
        self.enigo
            .key(modifier, Direction::Press)
            .map_err(|e| ComputerError::Keyboard(format!("Failed to press modifier: {e}")))?;
        self.enigo
            .key(key, Direction::Click)
            .map_err(|e| ComputerError::Keyboard(format!("Failed to tap key in combo: {e}")))?;
        self.enigo
            .key(modifier, Direction::Release)
            .map_err(|e| ComputerError::Keyboard(format!("Failed to release modifier: {e}")))?;
        Ok(())
    }

    /// Type text followed by Enter
    pub fn type_line(&mut self, text: &str) -> ComputerResult<()> {
        self.type_text(text)?;
        self.tap_key(enigo::Key::Return)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_controller_new() {
        let controller = KeyboardController::new();
        let _ = controller;
    }

    #[test]
    fn test_keyboard_controller_default() {
        let controller = KeyboardController::default();
        let _ = controller;
    }
}
