//! Computer use module - desktop control capabilities
//!
//! Provides screen capture, mouse control, and keyboard input
//! functionality for automated desktop interaction.

pub mod keyboard;
pub mod mouse;
pub mod screenshot;

pub use keyboard::KeyboardController;
pub use mouse::MouseController;
pub use screenshot::Screenshotter;

/// Result type for computer operations
pub type ComputerResult<T> = std::result::Result<T, ComputerError>;

/// Errors that can occur during computer operations
#[derive(Debug, thiserror::Error)]
pub enum ComputerError {
    /// Screen capture failed
    #[error("Screenshot failed: {0}")]
    Screenshot(String),
    /// Mouse control failed
    #[error("Mouse control failed: {0}")]
    Mouse(String),
    /// Keyboard input failed
    #[error("Keyboard input failed: {0}")]
    Keyboard(String),
    /// Display not found
    #[error("Display not found: {0}")]
    DisplayNotFound(String),
    /// Initialization failed
    #[error("Computer module initialization failed: {0}")]
    Init(String),
}

/// Coordinate on screen
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
}

impl Point {
    /// Create a new point
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Screen resolution
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolution {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

impl Resolution {
    /// Create a new resolution
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_creation() {
        let point = Point::new(100, 200);
        assert_eq!(point.x, 100);
        assert_eq!(point.y, 200);
    }

    #[test]
    fn test_resolution_creation() {
        let res = Resolution::new(1920, 1080);
        assert_eq!(res.width, 1920);
        assert_eq!(res.height, 1080);
    }

    #[test]
    fn test_computer_error_display() {
        let err = ComputerError::Screenshot("test error".to_string());
        assert_eq!(err.to_string(), "Screenshot failed: test error");
    }
}
