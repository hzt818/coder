//! Mouse control functionality using the `enigo` crate

use enigo::{
    Coordinate, Direction, Enigo, Mouse as EnigoMouse, Settings,
};

use super::{ComputerError, ComputerResult, Point};

/// Controls mouse position, clicks, and scrolling
#[derive(Debug)]
pub struct MouseController {
    /// Inner enigo instance for platform-level control
    enigo: Enigo,
}

impl Default for MouseController {
    fn default() -> Self {
        Self {
            enigo: Enigo::new(&Settings::default())
                .expect("Failed to initialize enigo mouse control"),
        }
    }
}

impl MouseController {
    /// Create a new MouseController
    ///
    /// # Panics
    /// Panics if the platform's mouse control cannot be initialized.
    pub fn new() -> Self {
        Self::default()
    }

    /// Move the mouse cursor to absolute screen coordinates
    pub fn move_to(&mut self, point: Point) -> ComputerResult<()> {
        self.enigo
            .move_mouse(point.x, point.y, Coordinate::Abs)
            .map_err(|e| ComputerError::Mouse(format!("Failed to move mouse: {e}")))?;
        Ok(())
    }

    /// Move the mouse cursor relative to its current position
    pub fn move_relative(&mut self, dx: i32, dy: i32) -> ComputerResult<()> {
        self.enigo
            .move_mouse(dx, dy, Coordinate::Rel)
            .map_err(|e| ComputerError::Mouse(format!("Failed to move mouse relative: {e}")))?;
        Ok(())
    }

    /// Click the left mouse button at the current position
    pub fn click_left(&mut self) -> ComputerResult<()> {
        self.enigo
            .button(enigo::Button::Left, Direction::Click)
            .map_err(|e| ComputerError::Mouse(format!("Failed to click left: {e}")))?;
        Ok(())
    }

    /// Click the right mouse button at the current position
    pub fn click_right(&mut self) -> ComputerResult<()> {
        self.enigo
            .button(enigo::Button::Right, Direction::Click)
            .map_err(|e| ComputerError::Mouse(format!("Failed to click right: {e}")))?;
        Ok(())
    }

    /// Click the middle mouse button at the current position
    pub fn click_middle(&mut self) -> ComputerResult<()> {
        self.enigo
            .button(enigo::Button::Middle, Direction::Click)
            .map_err(|e| ComputerError::Mouse(format!("Failed to click middle: {e}")))?;
        Ok(())
    }

    /// Press and hold the left mouse button
    pub fn press_left(&mut self) -> ComputerResult<()> {
        self.enigo
            .button(enigo::Button::Left, Direction::Press)
            .map_err(|e| ComputerError::Mouse(format!("Failed to press left: {e}")))?;
        Ok(())
    }

    /// Release the left mouse button
    pub fn release_left(&mut self) -> ComputerResult<()> {
        self.enigo
            .button(enigo::Button::Left, Direction::Release)
            .map_err(|e| ComputerError::Mouse(format!("Failed to release left: {e}")))?;
        Ok(())
    }

    /// Perform a double-click with the left mouse button
    pub fn double_click(&mut self) -> ComputerResult<()> {
        self.click_left()?;
        self.click_left()?;
        Ok(())
    }

    /// Scroll the mouse wheel vertically
    ///
    /// Positive `clicks` scrolls up, negative scrolls down.
    pub fn scroll_vertical(&mut self, clicks: i32) -> ComputerResult<()> {
        self.enigo
            .scroll(clicks, enigo::Axis::Vertical)
            .map_err(|e| ComputerError::Mouse(format!("Failed to scroll vertical: {e}")))?;
        Ok(())
    }

    /// Scroll the mouse wheel horizontally
    ///
    /// Positive `clicks` scrolls right, negative scrolls left.
    pub fn scroll_horizontal(&mut self, clicks: i32) -> ComputerResult<()> {
        self.enigo
            .scroll(clicks, enigo::Axis::Horizontal)
            .map_err(|e| ComputerError::Mouse(format!("Failed to scroll horizontal: {e}")))?;
        Ok(())
    }

    /// Get the current cursor position
    pub fn position(&mut self) -> ComputerResult<Point> {
        let location = self
            .enigo
            .location()
            .map_err(|e| ComputerError::Mouse(format!("Failed to get cursor position: {e}")))?;
        Ok(Point::new(location.0, location.1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_controller_new() {
        let controller = MouseController::new();
        // Enigo is created, just verify no panic
        let _ = controller;
    }

    #[test]
    fn test_point_move_to() {
        let point = Point::new(500, 500);
        assert_eq!(point.x, 500);
        assert_eq!(point.y, 500);
    }
}
