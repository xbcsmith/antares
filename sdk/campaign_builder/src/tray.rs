// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! macOS menu-bar status item (NSStatusItem) for the Campaign Builder.
//!
//! This module is compiled **only on macOS** (`#![cfg(target_os = "macos")]`).
//! On every other platform the module compiles to nothing, so non-macOS builds
//! remain warning-free.
//!
//! # Design
//!
//! * Two 22×22 / 44×44 PNG assets are embedded at compile time via
//!   [`include_bytes!`].  The 22×22 image is used to construct the
//!   [`tray_icon::Icon`]; the 44×44 image is available for Retina displays if
//!   the crate adds HiDPI support in a future release.
//! * [`build_tray_icon`] decodes the icon, builds a one-item context menu
//!   ("Quit"), constructs the [`tray_icon::TrayIcon`], and returns it to the
//!   caller.  **The caller must keep the returned value alive for the entire
//!   duration of the process** — dropping it removes the status item from the
//!   menu bar.
//! * [`handle_tray_events`] drains the [`tray_icon::menu::MenuEvent`] channel
//!   once per frame.  Dispatching "quit" calls [`std::process::exit(0)`].
//!
//! # Usage (in `run()`)
//!
//! ```ignore
//! #[cfg(target_os = "macos")]
//! let _tray = tray::build_tray_icon();
//!
//! // Inside CampaignBuilderApp::update():
//! #[cfg(target_os = "macos")]
//! tray::handle_tray_events();
//! ```

#![cfg(target_os = "macos")]

use tray_icon::{
    menu::{Menu, MenuItem},
    Icon, TrayIconBuilder,
};

// ── Embedded assets ──────────────────────────────────────────────────────────

/// 22×22 PNG used as the standard-resolution menu-bar icon (1× displays).
///
/// Produced by `scripts/generate_icons.sh` from `assets/icons/antares_tray.png`
/// and committed to `sdk/campaign_builder/assets/icons/tray_icon_1x.png`.
const TRAY_ICON_1X: &[u8] = include_bytes!("../assets/icons/tray_icon_1x.png");

/// 44×44 PNG reserved for Retina (2×) menu-bar displays.
///
/// Available for future HiDPI support.  Embedded here so that it is
/// compile-time verified regardless of whether it is currently used.
const TRAY_ICON_2X: &[u8] = include_bytes!("../assets/icons/tray_icon_2x.png");

// ── Menu item IDs ─────────────────────────────────────────────────────────────

/// Menu-item ID for the Quit action.
const MENU_ID_QUIT: &str = "quit";

// ── Public API ────────────────────────────────────────────────────────────────

/// Builds and returns the macOS menu-bar status item (NSStatusItem).
///
/// The function:
/// 1. Decodes [`TRAY_ICON_1X`] (22×22 RGBA PNG) with the `image` crate.
/// 2. Constructs a [`tray_icon::Icon`] from the decoded RGBA bytes.
/// 3. Builds a context [`Menu`] containing one item: **Quit** (ID `"quit"`).
/// 4. Calls [`TrayIconBuilder::build`] and returns the resulting
///    [`tray_icon::TrayIcon`].
///
/// # Panics
///
/// Panics if the embedded PNG bytes cannot be decoded or if the OS refuses
/// to create the status item.  Neither failure can occur in practice — the
/// bytes are compile-time verified via `include_bytes!`, and the OS call
/// succeeds as long as the application is running on macOS.
///
/// # Caller Responsibility
///
/// **The returned [`tray_icon::TrayIcon`] must be kept alive** for the entire
/// lifetime of the process.  Assigning it to a `let _tray` binding in `run()`
/// achieves this because `_tray` is only dropped when `run()` returns (i.e.
/// after [`eframe::run_native`] exits).
///
/// # Examples
///
/// ```ignore
/// #[cfg(target_os = "macos")]
/// let _tray = tray::build_tray_icon();
/// ```
pub fn build_tray_icon() -> tray_icon::TrayIcon {
    // Decode the 22×22 PNG to RGBA8.
    let img = image::load_from_memory(TRAY_ICON_1X)
        .expect("failed to decode tray_icon_1x.png — embedded bytes must be valid PNG")
        .into_rgba8();
    let width = img.width();
    let height = img.height();
    let rgba = img.into_raw();

    // Construct the platform icon from raw RGBA bytes.
    let icon = Icon::from_rgba(rgba, width, height)
        .expect("failed to construct tray_icon::Icon — RGBA dimensions must be consistent");

    // Build the context menu: one "Quit" item.
    let menu = Menu::new();
    let quit_item = MenuItem::with_id(MENU_ID_QUIT, "Quit", true, None);
    menu.append(&quit_item)
        .expect("failed to append Quit item to tray context menu");

    // Assemble and return the TrayIcon.
    TrayIconBuilder::new()
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .with_tooltip("Antares Campaign Builder")
        .build()
        .expect("failed to build macOS NSStatusItem — must be called on the main thread")
}

/// Polls the menu-event channel and dispatches pending tray-menu actions.
///
/// This function should be called **once per frame** from
/// [`eframe::App::update`].  It drains all pending [`tray_icon::menu::MenuEvent`]
/// values without blocking.
///
/// # Dispatched actions
///
/// | Menu item ID | Action                        |
/// |--------------|-------------------------------|
/// | `"quit"`     | [`std::process::exit(0)`]     |
///
/// All other IDs are silently ignored so that future menu items can be added
/// without modifying this function.
///
/// # Examples
///
/// ```ignore
/// // Inside CampaignBuilderApp::update():
/// #[cfg(target_os = "macos")]
/// tray::handle_tray_events();
/// ```
pub fn handle_tray_events() {
    while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
        if event.id == MENU_ID_QUIT {
            std::process::exit(0);
        }
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────
//
// These tests exercise only embedded-asset properties (PNG magic bytes and
// decoded pixel dimensions).  No NSApp / NSStatusItem is touched, so the
// tests are safe to run in any environment where the crate compiles.

#[cfg(test)]
mod tests {
    use super::*;

    /// The first 8 bytes of every valid PNG file (ISO/IEC 15948:2003 §5.2).
    const PNG_MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    // ── PNG magic ─────────────────────────────────────────────────────────────

    /// The 1× asset must begin with the PNG magic number.
    ///
    /// This confirms that `assets/icons/tray_icon_1x.png` is a valid PNG file
    /// and has not been accidentally replaced or truncated.
    #[test]
    fn test_tray_icon_1x_png_magic() {
        assert!(
            TRAY_ICON_1X.len() >= PNG_MAGIC.len(),
            "TRAY_ICON_1X must be at least 8 bytes long"
        );
        assert_eq!(
            &TRAY_ICON_1X[..PNG_MAGIC.len()],
            &PNG_MAGIC,
            "TRAY_ICON_1X must start with the PNG magic number"
        );
    }

    /// The 2× asset must begin with the PNG magic number.
    ///
    /// This confirms that `assets/icons/tray_icon_2x.png` is a valid PNG file
    /// and has not been accidentally replaced or truncated.
    #[test]
    fn test_tray_icon_2x_png_magic() {
        assert!(
            TRAY_ICON_2X.len() >= PNG_MAGIC.len(),
            "TRAY_ICON_2X must be at least 8 bytes long"
        );
        assert_eq!(
            &TRAY_ICON_2X[..PNG_MAGIC.len()],
            &PNG_MAGIC,
            "TRAY_ICON_2X must start with the PNG magic number"
        );
    }

    // ── Decoded dimensions ────────────────────────────────────────────────────

    /// The 1× asset must decode to exactly 22×22 pixels.
    ///
    /// macOS uses 22×22 for standard-resolution menu-bar status items.
    #[test]
    fn test_tray_icon_1x_dimensions() {
        let img = image::load_from_memory(TRAY_ICON_1X)
            .expect("TRAY_ICON_1X must decode successfully")
            .into_rgba8();
        assert_eq!(img.width(), 22, "1× tray icon width must be 22 px");
        assert_eq!(img.height(), 22, "1× tray icon height must be 22 px");
    }

    /// The 2× asset must decode to exactly 44×44 pixels.
    ///
    /// macOS uses 44×44 for Retina (2×) menu-bar status items.
    #[test]
    fn test_tray_icon_2x_dimensions() {
        let img = image::load_from_memory(TRAY_ICON_2X)
            .expect("TRAY_ICON_2X must decode successfully")
            .into_rgba8();
        assert_eq!(img.width(), 44, "2× tray icon width must be 44 px");
        assert_eq!(img.height(), 44, "2× tray icon height must be 44 px");
    }

    // ── RGBA buffer length ────────────────────────────────────────────────────

    /// The RGBA byte buffer for the 1× asset must be exactly 22 × 22 × 4 bytes.
    ///
    /// Verifies that the `image` crate produces a fully-populated RGBA8 buffer
    /// with no padding or truncation.
    #[test]
    fn test_tray_icon_1x_rgba_length() {
        let img = image::load_from_memory(TRAY_ICON_1X)
            .expect("TRAY_ICON_1X must decode successfully")
            .into_rgba8();
        let expected: usize = 22 * 22 * 4;
        assert_eq!(
            img.into_raw().len(),
            expected,
            "1× RGBA buffer must be 22 × 22 × 4 = {expected} bytes"
        );
    }
}
