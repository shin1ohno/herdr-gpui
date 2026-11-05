//! Keep native corner controls beside the header, without moving the body.
use super::{LEADING, MARGIN};
use gpui::Window;

pub(super) fn leading(window: &Window) -> f32 {
    // The public UIKit adapter already samples the current physical corner.
    // Reuse it so resize frames and tests see the same platform-owned region.
    LEADING.max(f32::from(crate::window::viewport::window_controls(window).left) + MARGIN)
}
