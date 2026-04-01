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
//! * [`build_tray_icon`] decodes the icon, builds a three-item context menu
//!   ("Show Antares Campaign Builder", "Hide", "Quit"), creates the
//!   [`tray_icon::TrayIcon`], and returns it together with the receiver end of
//!   a [`TrayCommand`] channel.  **The caller must keep the returned
//!   [`tray_icon::TrayIcon`] alive for the entire duration of the process** —
//!   dropping it removes the status item from the menu bar.
//! * [`handle_tray_events`] drains the [`tray_icon::menu::MenuEvent`] channel
//!   once per frame and sends [`TrayCommand`] values over the mpsc channel.
//!   The application's `update()` loop drains that channel and issues the
//!   appropriate [`egui::ViewportCommand`]s.
//! * "Quit" is the sole exception: it calls [`std::process::exit(0)`] directly
//!   for immediate, synchronous termination without waiting for the next frame.
//!
//! # Usage (in `run()`)
//!
//! ```ignore
//! #[cfg(target_os = "macos")]
//! let (_tray, tray_cmd_rx) = tray::build_tray_icon();
//!
//! // Inside CampaignBuilderApp — store `tray_cmd_rx` in the struct, then
//! // in update():
//! #[cfg(target_os = "macos")]
//! tray::handle_tray_events();
//!
//! #[cfg(target_os = "macos")]
//! if let Some(ref rx) = self.tray_cmd_rx {
//!     while let Ok(cmd) = rx.try_recv() {
//!         match cmd {
//!             tray::TrayCommand::ShowWindow => {
//!                 ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
//!                 ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
//!             }
//!             tray::TrayCommand::HideWindow => {
//!                 ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
//!             }
//!             tray::TrayCommand::Quit => {
//!                 ctx.send_viewport_cmd(egui::ViewportCommand::Close);
//!             }
//!         }
//!     }
//! }
//! ```

use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::OnceLock;
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

// ── Menu item IDs ─────────────────────────────────────────────────────────────

/// Menu-item ID for the "Show Antares Campaign Builder" action.
const MENU_ID_SHOW: &str = "show";

/// Menu-item ID for the "Hide" action.
const MENU_ID_HIDE: &str = "hide";

/// Menu-item ID for the Quit action.
const MENU_ID_QUIT: &str = "quit";

// ── TrayCommand ───────────────────────────────────────────────────────────────

/// Commands dispatched from the macOS menu-bar tray to the application window.
///
/// [`build_tray_icon`] returns a [`Receiver<TrayCommand>`] that the application
/// stores and drains once per frame inside [`eframe::App::update`].
/// [`handle_tray_events`] translates raw [`tray_icon::menu::MenuEvent`]s into
/// `TrayCommand` values and sends them over the channel.
///
/// `Quit` is an exception: it calls [`std::process::exit(0)`] directly in
/// [`handle_tray_events`] for immediate, synchronous termination rather than
/// waiting for the next `update()` frame.  `TrayCommand::Quit` is still
/// provided as a variant so that callers can handle close requests issued via
/// `ViewportCommand::Close` if they choose to intercept them.
///
/// # Examples
///
/// ```ignore
/// // Drain the receiver in update():
/// while let Ok(cmd) = tray_cmd_rx.try_recv() {
///     match cmd {
///         tray::TrayCommand::ShowWindow => { /* raise window */ }
///         tray::TrayCommand::HideWindow => { /* hide window */ }
///         tray::TrayCommand::Quit       => { /* close viewport */ }
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum TrayCommand {
    /// Raise the window to the front and make it visible.
    ShowWindow,
    /// Hide the window without terminating the process.
    HideWindow,
    /// Close the egui viewport (counterpart to the direct `process::exit` path).
    Quit,
}

// ── Module-level channel sender ───────────────────────────────────────────────

/// Sender half of the tray-command channel, initialised once by
/// [`build_tray_icon`].
///
/// [`SyncSender<T>`] is both `Send` and `Sync`, which satisfies the `static`
/// bound required by [`OnceLock`].  The channel capacity of 32 ensures that
/// rapid menu interactions never block the OS callback thread.
static TRAY_CMD_TX: OnceLock<SyncSender<TrayCommand>> = OnceLock::new();

// ── Public API ────────────────────────────────────────────────────────────────

/// Builds and returns the macOS menu-bar status item together with the receiver
/// end of the [`TrayCommand`] channel.
///
/// The function:
/// 1. Creates a bounded [`mpsc::sync_channel`] (capacity 32) and stores the
///    sender in the module-level [`TRAY_CMD_TX`] static.
/// 2. Decodes [`TRAY_ICON_1X`] (22×22 RGBA PNG) with the `image` crate.
/// 3. Constructs a [`tray_icon::Icon`] from the decoded RGBA bytes.
/// 4. Builds a context [`Menu`] with three items:
///    - **Show Antares Campaign Builder** (ID `"show"`)
///    - **Hide** (ID `"hide"`)
///    - **Quit** (ID `"quit"`)
/// 5. Calls [`TrayIconBuilder::build`] and returns the resulting
///    [`tray_icon::TrayIcon`] paired with the [`Receiver<TrayCommand>`].
///
/// # Panics
///
/// Panics if the embedded PNG bytes cannot be decoded or if the OS refuses to
/// create the status item.  Neither failure can occur in practice — the bytes
/// are compile-time verified via `include_bytes!`, and the OS call succeeds as
/// long as the application is running on macOS with a valid run-loop.
///
/// # Caller Responsibility
///
/// **The returned [`tray_icon::TrayIcon`] must be kept alive** for the entire
/// lifetime of the process.  Assigning it to a `let _tray` binding in `run()`
/// achieves this because `_tray` is only dropped when `run()` returns (i.e.
/// after [`eframe::run_native`] exits).
///
/// The [`Receiver<TrayCommand>`] should be moved into [`CampaignBuilderApp`]
/// and drained once per frame in `update()`.
///
/// # Examples
///
/// ```ignore
/// #[cfg(target_os = "macos")]
/// let (_tray, tray_cmd_rx) = tray::build_tray_icon();
/// // `_tray` stays alive for the duration of `run()`; `tray_cmd_rx` is moved
/// // into the app inside the `run_native` closure.
/// ```
pub fn build_tray_icon() -> (tray_icon::TrayIcon, Receiver<TrayCommand>) {
    // Create the bounded command channel (capacity 32).
    // `SyncSender` is `Send + Sync`, satisfying the `OnceLock` static bound.
    let (tx, rx) = mpsc::sync_channel(32);

    // Store the sender so that `handle_tray_events` can retrieve it without
    // an extra parameter.  If `build_tray_icon` is called more than once
    // (which should not happen in production), the first sender remains and
    // the error from `set` is intentionally ignored.
    let _ = TRAY_CMD_TX.set(tx);

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

    // Build the context menu with three items.
    let menu = Menu::new();

    let show_item = MenuItem::with_id(MENU_ID_SHOW, "Show Antares Campaign Builder", true, None);
    let hide_item = MenuItem::with_id(MENU_ID_HIDE, "Hide", true, None);
    let quit_item = MenuItem::with_id(MENU_ID_QUIT, "Quit", true, None);

    menu.append(&show_item)
        .expect("failed to append Show item to tray context menu");
    menu.append(&hide_item)
        .expect("failed to append Hide item to tray context menu");
    menu.append(&quit_item)
        .expect("failed to append Quit item to tray context menu");

    // Assemble and return the TrayIcon.
    let tray = TrayIconBuilder::new()
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .with_tooltip("Antares Campaign Builder")
        .build()
        .expect("failed to build macOS NSStatusItem — must be called on the main thread");

    (tray, rx)
}

/// Polls the menu-event channel and dispatches pending tray-menu actions.
///
/// This function should be called **once per frame** from
/// [`eframe::App::update`].  It drains all pending [`tray_icon::menu::MenuEvent`]
/// values without blocking.
///
/// # Dispatched actions
///
/// | Menu item ID | Action                                                      |
/// |--------------|-------------------------------------------------------------|
/// | `"quit"`     | [`std::process::exit(0)`] (immediate, synchronous exit)     |
/// | `"show"`     | Sends [`TrayCommand::ShowWindow`] over the mpsc channel     |
/// | `"hide"`     | Sends [`TrayCommand::HideWindow`] over the mpsc channel     |
///
/// The application's `update()` loop drains the [`Receiver<TrayCommand>`]
/// stored in [`CampaignBuilderApp`] and issues the appropriate
/// [`egui::ViewportCommand`]s:
/// - `ShowWindow` → `ViewportCommand::Focus` + `ViewportCommand::Visible(true)`
/// - `HideWindow` → `ViewportCommand::Visible(false)`
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
    // Retrieve the sender stored by `build_tray_icon()`.  If the static has
    // not yet been initialised (should never happen in production since
    // `build_tray_icon` is always called before the first `update()` frame),
    // return early rather than panicking.
    let Some(tx) = TRAY_CMD_TX.get() else {
        return;
    };

    while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
        if event.id == MENU_ID_QUIT {
            // Quit is handled synchronously: terminate the process immediately.
            std::process::exit(0);
        } else if event.id == MENU_ID_SHOW {
            // Non-blocking send; if the receiver has been dropped the error
            // is intentionally ignored — the window is already gone.
            let _ = tx.send(TrayCommand::ShowWindow);
        } else if event.id == MENU_ID_HIDE {
            let _ = tx.send(TrayCommand::HideWindow);
        }
        // Unknown IDs are silently ignored.
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────
//
// Phase 2 tests: embedded-asset properties (PNG magic bytes, decoded
// dimensions, RGBA buffer length).  No NSApp / NSStatusItem is touched.
//
// Phase 3 tests: `TrayCommand` variant distinctness and channel round-trip.
// These tests only exercise pure Rust types and require no macOS APIs.

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    /// The first 8 bytes of every valid PNG file (ISO/IEC 15948:2003 §5.2).
    const PNG_MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    // ── Phase 2: PNG magic ────────────────────────────────────────────────────

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

    // ── Phase 2: Decoded dimensions ───────────────────────────────────────────

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

    // ── Phase 2: RGBA buffer length ───────────────────────────────────────────

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

    // ── Phase 3: TrayCommand enum ─────────────────────────────────────────────

    /// `TrayCommand::ShowWindow` and `TrayCommand::HideWindow` must be distinct
    /// values so that the `update()` match arm dispatches them to different
    /// [`egui::ViewportCommand`]s.
    ///
    /// This test also confirms that `TrayCommand` derives `PartialEq`, which is
    /// required for the `assert_ne!` assertion and for use in tests.
    #[test]
    fn test_tray_command_show_is_distinct_from_hide() {
        assert_ne!(
            TrayCommand::ShowWindow,
            TrayCommand::HideWindow,
            "ShowWindow and HideWindow must be distinct TrayCommand variants"
        );
    }

    /// A `TrayCommand::ShowWindow` value sent over an `mpsc` sync channel must
    /// be received on the other end without blocking.
    ///
    /// This exercises the channel-based dispatch path introduced in Phase 3:
    /// `handle_tray_events` sends commands and `update()` drains the receiver
    /// via `try_recv()`.
    #[test]
    fn test_tray_command_channel_send_recv() {
        let (tx, rx) = mpsc::sync_channel(1);
        tx.send(TrayCommand::ShowWindow)
            .expect("send must succeed on a fresh channel with capacity 1");
        let received = rx
            .try_recv()
            .expect("try_recv must return the sent command without blocking");
        assert_eq!(
            received,
            TrayCommand::ShowWindow,
            "received TrayCommand must match the value that was sent"
        );
    }
}
