// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared UI helper utilities for the campaign builder.
//!
//! This module contains small, pure helper functions and constants used by the
//! editor UI components (items, spells, monsters, etc.) to compute panel heights
//! and standard column widths. These helpers centralize layout constants and
//! logic so multiple editors behave consistently when windows are resized.

use eframe::egui;

/// Default width (in points) of the left-hand column used by editors
/// (commonly the list panel).
pub const DEFAULT_LEFT_COLUMN_WIDTH: f32 = 300.0;

/// Default minimum height (in points) for editor list & preview panels.
///
/// This value prevents panels from collapsing to 0 or very short heights when
/// the computed available region is very small.
pub const DEFAULT_PANEL_MIN_HEIGHT: f32 = 100.0;

/// Compute the effective panel height from a Vec2 size and a minimum height.
///
/// This is a pure function that can be tested easily. It returns either the
/// `size.y` or the provided `min_height` if `size.y` is smaller.
///
/// # Examples
///
/// ```
/// use eframe::egui::Vec2;
/// use antares::sdk::campaign_builder::ui_helpers::compute_panel_height_from_size;
///
/// let size = Vec2::new(400.0, 240.0);
/// assert_eq!(compute_panel_height_from_size(size, 100.0), 240.0);
///
/// let tiny_size = Vec2::new(200.0, 20.0);
/// assert_eq!(compute_panel_height_from_size(tiny_size, 100.0), 100.0);
/// ```
pub fn compute_panel_height_from_size(size: egui::Vec2, min_height: f32) -> f32 {
    size.y.max(min_height)
}

/// Compute effective panel height from an `egui::Ui` instance.
///
/// This returns the vertical size in the given `Ui` (via `available_size_before_wrap`) or the
/// provided `min_height` if the region is smaller. This function should be used
/// by editor UIs to decide the `max_height` used for `ScrollArea` and the
/// `min_height` used for columns.
///
/// # Example
///
/// ```no_run
/// use eframe::egui;
/// use antares::sdk::campaign_builder::ui_helpers::compute_panel_height;
///
/// // In editor code where `ui: &mut egui::Ui` exists:
/// // let panel_height = compute_panel_height(ui, DEFAULT_PANEL_MIN_HEIGHT);
/// ```
pub fn compute_panel_height(ui: &mut egui::Ui, min_height: f32) -> f32 {
    compute_panel_height_from_size(ui.available_size_before_wrap(), min_height)
}

/// Convenience function that uses the module's default minimum height.
///
/// Call this from editor panels when you want to use the standard configured
/// default.
pub fn compute_default_panel_height(ui: &mut egui::Ui) -> f32 {
    compute_panel_height(ui, DEFAULT_PANEL_MIN_HEIGHT)
}

/// Convenience function that uses `DEFAULT_PANEL_MIN_HEIGHT` for a given size.
pub fn compute_default_panel_height_from_size(size: egui::Vec2) -> f32 {
    compute_panel_height_from_size(size, DEFAULT_PANEL_MIN_HEIGHT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Vec2;

    #[test]
    fn compute_panel_height_from_size_returns_size_y_if_larger() {
        let size = Vec2::new(100.0, 250.0);
        let min = 100.0;
        assert_eq!(compute_panel_height_from_size(size, min), 250.0);
    }

    #[test]
    fn compute_panel_height_from_size_returns_min_if_size_smaller() {
        let size = Vec2::new(100.0, 40.0);
        let min = 100.0;
        assert_eq!(compute_panel_height_from_size(size, min), min);
    }

    #[test]
    fn compute_default_panel_height_from_size_uses_default_min() {
        let size = Vec2::new(640.0, 90.0);
        assert_eq!(
            compute_default_panel_height_from_size(size),
            DEFAULT_PANEL_MIN_HEIGHT
        );
    }
}
