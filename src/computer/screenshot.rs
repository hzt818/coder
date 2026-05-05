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
        let screens = screenshots::Screen::all()
            .map_err(|e| ComputerError::Screenshot(format!("Failed to list screens: {e}")))?;

        let primary = screens
            .first()
            .ok_or_else(|| ComputerError::DisplayNotFound("No displays found".to_string()))?;

        self.capture_display(*primary.id())
    }

    /// Capture a specific display by its ID
    pub fn capture_display(&self, display_id: u32) -> ComputerResult<RgbaImage> {
        let screen = screenshots::Screen::from_id(display_id)
            .map_err(|e| ComputerError::Screenshot(format!("Failed to get display {display_id}: {e}")))?;

        let image_data = screen
            .capture()
            .map_err(|e| ComputerError::Screenshot(format!("Capture failed: {e}")))?;

        let rgba = RgbaImage::from_raw(
            image_data.width(),
            image_data.height(),
            image_data.bytes().to_vec(),
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
        let screens = screenshots::Screen::all()
            .map_err(|e| ComputerError::Screenshot(format!("Failed to list screens: {e}")))?;

        let primary = screens
            .first()
            .ok_or_else(|| ComputerError::DisplayNotFound("No displays found".to_string()))?;

        Ok(Resolution::new(primary.display_info().width, primary.display_info().height))
    }

    /// List all available display IDs and their resolutions
    pub fn list_displays(&self) -> ComputerResult<Vec<(u32, Resolution)>> {
        let screens = screenshots::Screen::all()
            .map_err(|e| ComputerError::Screenshot(format!("Failed to list screens: {e}")))?;

        Ok(screens
            .iter()
            .map(|s| {
                let id = *s.id();
                let info = s.display_info();
                (id, Resolution::new(info.width, info.height))
            })
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
