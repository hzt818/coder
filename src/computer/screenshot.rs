//! Screen capture functionality using the `screenshots` crate

use image::RgbaImage;

use super::{ComputerError, ComputerResult, Resolution};

/// Captures screenshots of the primary or specified display
#[derive(Debug)]
pub struct Screenshotter {
    /// Capture scale factor (1.0 = native resolution)
    scale: f64,
}

impl Default for Screenshotter {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

impl Screenshotter {
    /// Create a new Screenshotter with default scale
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new Screenshotter with a custom scale factor
    pub fn with_scale(scale: f64) -> Self {
        Self { scale }
    }

    /// Capture the entire primary display
    ///
    /// Returns an RGBA image of the screen contents.
    pub fn capture_primary_display(&self) -> ComputerResult<RgbaImage> {
        let screens = screenshots::Screen::all();
        let primary = screens
            .first()
            .ok_or_else(|| ComputerError::DisplayNotFound("No displays found".to_string()))?;

        self.capture_screen(primary)
    }

    /// Capture a specific display by its ID
    pub fn capture_display(&self, display_id: u32) -> ComputerResult<RgbaImage> {
        let screens = screenshots::Screen::all();
        let screen = screens
            .into_iter()
            .find(|s| s.id == display_id)
            .ok_or_else(|| ComputerError::Screenshot(format!("Display {display_id} not found")))?;

        self.capture_screen(&screen)
    }

    /// Internal helper: capture a screen reference into an RgbaImage
    fn capture_screen(&self, screen: &screenshots::Screen) -> ComputerResult<RgbaImage> {
        let image_data = screen
            .capture()
            .ok_or_else(|| ComputerError::Screenshot("Capture returned no data".to_string()))?;

        let rgba = RgbaImage::from_raw(
            image_data.width(),
            image_data.height(),
            image_data.buffer().clone(),
        )
        .ok_or_else(|| ComputerError::Screenshot("Failed to create image buffer".to_string()))?;

        if (self.scale - 1.0).abs() > f64::EPSILON {
            let new_width = (rgba.width() as f64 * self.scale) as u32;
            let new_height = (rgba.height() as f64 * self.scale) as u32;
            let scaled = image::imageops::resize(
                &rgba,
                new_width,
                new_height,
                image::imageops::FilterType::Lanczos3,
            );
            Ok(scaled)
        } else {
            Ok(rgba)
        }
    }

    /// Get the resolution of the primary display
    pub fn primary_display_resolution(&self) -> ComputerResult<Resolution> {
        let screens = screenshots::Screen::all();
        let primary = screens
            .first()
            .ok_or_else(|| ComputerError::DisplayNotFound("No displays found".to_string()))?;

        Ok(Resolution::new(primary.width, primary.height))
    }

    /// List all available display IDs and their resolutions
    pub fn list_displays(&self) -> ComputerResult<Vec<(u32, Resolution)>> {
        let screens = screenshots::Screen::all();

        Ok(screens
            .iter()
            .map(|s| (s.id, Resolution::new(s.width, s.height)))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screenshotter_new() {
        let shotter = Screenshotter::new();
        assert!((shotter.scale - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_screenshotter_with_scale() {
        let shotter = Screenshotter::with_scale(2.0);
        assert!((shotter.scale - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_screenshotter_default() {
        let shotter = Screenshotter::default();
        assert!((shotter.scale - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_capture_display_invalid_id() {
        let shotter = Screenshotter::new();
        let result = shotter.capture_display(9999);
        assert!(result.is_err());
    }
}
