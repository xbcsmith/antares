// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Application icon helpers for the Campaign Builder window and Dock entry.
//!
//! The Antares icon PNG is embedded at compile time via [`include_bytes!`],
//! decoded once at startup with the [`image`] crate, and returned as an
//! [`egui::IconData`] suitable for `ViewportBuilder::with_icon()`.
//!
//! # Design
//!
//! * The asset path is relative to this source file, so the SDK crate is
//!   self-contained — it does not depend on the workspace root layout.
//! * Decoding failure is treated defensively: `None` is returned and a warning
//!   is printed to `stderr` so the application still launches with a generic
//!   icon.  In practice the failure path is unreachable because `include_bytes!`
//!   verifies the file exists at compile time.

use eframe::egui;
use std::sync::Arc;

/// Raw PNG bytes for the application icon, embedded at compile time.
///
/// The file is embedded relative to this source file so no runtime file-system
/// access is required.
const ICON_PNG: &[u8] = include_bytes!("../assets/antares_tray.png");

/// Decodes the embedded application icon and returns it as [`egui::IconData`].
///
/// The PNG is decoded to RGBA8 pixels using the [`image`] crate.  On success
/// an [`Arc`]-wrapped [`egui::IconData`] is returned so it can be passed
/// directly to `ViewportBuilder::with_icon()`.
///
/// Returns `None` and emits a warning to `stderr` if the embedded bytes cannot
/// be decoded.  This cannot happen in practice — `include_bytes!` guarantees
/// the file is present at compile time — but the defensive fallback ensures the
/// application always starts even under unexpected conditions.
///
/// # Examples
///
/// ```
/// use campaign_builder::icon::app_icon_data;
///
/// let icon = app_icon_data();
/// assert!(icon.is_some(), "embedded icon must always decode successfully");
/// ```
pub fn app_icon_data() -> Option<Arc<egui::IconData>> {
    match image::load_from_memory(ICON_PNG) {
        Ok(img) => {
            let width = img.width();
            let height = img.height();
            let rgba = img.into_rgba8().into_raw();
            Some(Arc::new(egui::IconData {
                rgba,
                width,
                height,
            }))
        }
        Err(e) => {
            eprintln!("[WARN] Failed to decode application icon: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PNG magic number: the first 8 bytes of every valid PNG file.
    ///
    /// Defined by the PNG specification (ISO/IEC 15948:2003, section 5.2).
    const PNG_MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    /// `app_icon_data()` must return `Some` for the embedded asset.
    #[test]
    fn test_app_icon_data_returns_some() {
        assert!(
            app_icon_data().is_some(),
            "embedded icon must decode to Some"
        );
    }

    /// Both decoded dimensions must be greater than zero.
    #[test]
    fn test_app_icon_data_dimensions_non_zero() {
        let icon = app_icon_data().expect("icon must be Some");
        assert!(icon.width > 0, "decoded icon width must be > 0");
        assert!(icon.height > 0, "decoded icon height must be > 0");
    }

    /// The RGBA byte buffer must be exactly `width * height * 4` bytes long.
    #[test]
    fn test_app_icon_data_rgba_length_matches_dimensions() {
        let icon = app_icon_data().expect("icon must be Some");
        let expected = (icon.width * icon.height * 4) as usize;
        assert_eq!(
            icon.rgba.len(),
            expected,
            "RGBA byte length must equal width × height × 4 (got {} bytes, expected {})",
            icon.rgba.len(),
            expected,
        );
    }

    /// The raw embedded bytes must begin with the PNG magic number.
    ///
    /// This confirms that the asset copied into `assets/antares_tray.png` is a
    /// valid PNG file and has not been accidentally replaced or truncated.
    #[test]
    fn test_app_icon_data_is_valid_png() {
        assert!(
            ICON_PNG.len() >= PNG_MAGIC.len(),
            "embedded ICON_PNG must be at least 8 bytes long"
        );
        assert_eq!(
            &ICON_PNG[..PNG_MAGIC.len()],
            &PNG_MAGIC,
            "embedded ICON_PNG must start with the PNG magic number"
        );
    }
}
