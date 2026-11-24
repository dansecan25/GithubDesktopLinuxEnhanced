//! Saved window geometry.
//!
//! The port this app replaces kept saving whatever size the window reported, including the
//! shrunken sizes some compositors report while a window is minimized, being restored, or scaled.
//! Over many launches the window got smaller until it couldn't be opened. Here a size is only
//! recorded while the window is visible, normal (not maximized or minimized) and focused, and every
//! value is checked against the minimum again before it is used.

use serde::{Deserialize, Serialize};

use crate::constants::layout::{
    DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH, MAX_WINDOW_DIMENSION, MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH,
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowGeometry {
    pub width: f32,
    pub height: f32,
    pub maximized: bool,
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self { width: DEFAULT_WINDOW_WIDTH, height: DEFAULT_WINDOW_HEIGHT, maximized: false }
    }
}

impl WindowGeometry {
    /// Returns a geometry that is safe to open a window with, falling back to the default size
    /// for any dimension that is missing, corrupted, below the minimum or larger than the screen.
    pub fn sanitized(self, monitor: Option<(f32, f32)>) -> Self {
        let (max_width, max_height) = monitor.unwrap_or((MAX_WINDOW_DIMENSION, MAX_WINDOW_DIMENSION));
        Self {
            width: valid_dimension(self.width, MIN_WINDOW_WIDTH, max_width, DEFAULT_WINDOW_WIDTH),
            height: valid_dimension(self.height, MIN_WINDOW_HEIGHT, max_height, DEFAULT_WINDOW_HEIGHT),
            maximized: self.maximized,
        }
    }
}

fn valid_dimension(value: f32, min: f32, max: f32, default: f32) -> f32 {
    if value.is_finite() && value >= min && value <= max.max(min) {
        value
    } else {
        default.clamp(min, max.max(min))
    }
}

/// What the window reported this frame. All fields are optional because platforms differ.
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowObservation {
    pub inner_size: Option<(f32, f32)>,
    pub maximized: Option<bool>,
    pub minimized: Option<bool>,
    pub fullscreen: Option<bool>,
    pub focused: Option<bool>,
}

/// Updates `saved` from one observation, ignoring any size reported while the window isn't in a
/// normal, visible, focused state.
pub fn record(saved: &mut WindowGeometry, observed: WindowObservation) {
    if observed.minimized == Some(true) || observed.fullscreen == Some(true) {
        return;
    }
    if let Some(maximized) = observed.maximized {
        saved.maximized = maximized;
        if maximized {
            return;
        }
    }
    if observed.focused == Some(false) {
        return;
    }
    let Some((width, height)) = observed.inner_size else { return };
    if width >= MIN_WINDOW_WIDTH && height >= MIN_WINDOW_HEIGHT && width.is_finite() && height.is_finite() {
        saved.width = width;
        saved.height = height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiny_or_corrupt_sizes_fall_back_to_default() {
        let geometry = WindowGeometry { width: 12.0, height: f32::NAN, maximized: false }.sanitized(None);
        assert_eq!(geometry.width, DEFAULT_WINDOW_WIDTH);
        assert_eq!(geometry.height, DEFAULT_WINDOW_HEIGHT);
    }

    #[test]
    fn sizes_larger_than_monitor_are_rejected() {
        let geometry = WindowGeometry { width: 5000.0, height: 700.0, maximized: false }
            .sanitized(Some((1920.0, 1080.0)));
        assert_eq!(geometry.width, DEFAULT_WINDOW_WIDTH);
        assert_eq!(geometry.height, 700.0);
    }

    #[test]
    fn default_fits_small_monitors() {
        let geometry =
            WindowGeometry { width: 0.0, height: 0.0, maximized: false }.sanitized(Some((800.0, 600.0)));
        assert_eq!((geometry.width, geometry.height), (800.0, 600.0));
    }

    #[test]
    fn minimized_and_maximized_sizes_are_not_recorded() {
        let mut saved = WindowGeometry::default();
        let shrunk = Some((100.0, 100.0));
        record(
            &mut saved,
            WindowObservation { inner_size: shrunk, minimized: Some(true), ..Default::default() },
        );
        record(&mut saved, WindowObservation { inner_size: shrunk, ..Default::default() });
        assert_eq!(saved, WindowGeometry::default());

        let huge = Some((3000.0, 2000.0));
        record(
            &mut saved,
            WindowObservation { inner_size: huge, maximized: Some(true), ..Default::default() },
        );
        assert!(saved.maximized);
        assert_eq!(saved.width, DEFAULT_WINDOW_WIDTH);

        let normal = Some((900.0, 700.0));
        record(
            &mut saved,
            WindowObservation { inner_size: normal, maximized: Some(false), ..Default::default() },
        );
        assert_eq!((saved.width, saved.height, saved.maximized), (900.0, 700.0, false));
    }
}
