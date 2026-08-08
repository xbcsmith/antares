// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared constants and types for inventory UI systems.
//!
//! This module extracts layout constants and navigation types that are common
//! across the three inventory overlay systems:
//!
//! - [`super::inventory_ui`] — character inventory management
//! - [`super::merchant_inventory_ui`] — merchant buy/sell interface
//! - [`super::container_inventory_ui`] — container take/stash interface
//!
//! Centralising these definitions eliminates duplication and ensures visual
//! consistency across all inventory-related screens.

use bevy_egui::egui;

// ===== Layout constants =====

/// Height of the character-name / panel header bar inside each panel.
pub(crate) const PANEL_HEADER_H: f32 = 36.0;

/// Height of the action-button strip below the grid when a slot is selected.
pub(crate) const PANEL_ACTION_H: f32 = 48.0;

/// Number of slot columns in the inventory grid inside each character panel.
///
/// With `Inventory::MAX_ITEMS = 64` and `SLOT_COLS = 8` the grid is 8×8.
pub(crate) const SLOT_COLS: usize = 8;

// ===== Colour constants =====

/// Faint grid-line colour.
pub(crate) const GRID_LINE_COLOR: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(60, 60, 60, 255);

/// Panel body background colour.
pub(crate) const PANEL_BG_COLOR: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(18, 18, 18, 255);

/// Header background colour.
pub(crate) const HEADER_BG_COLOR: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(35, 35, 35, 255);

/// Slot / row selection highlight colour.
pub(crate) const SELECT_HIGHLIGHT_COLOR: egui::Color32 = egui::Color32::YELLOW;

/// Focused panel border colour.
pub(crate) const FOCUSED_BORDER_COLOR: egui::Color32 = egui::Color32::YELLOW;

/// Unfocused panel border colour.
pub(crate) const UNFOCUSED_BORDER_COLOR: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(80, 80, 80, 255);

/// Action button highlight colour when keyboard focus is on it.
pub(crate) const ACTION_FOCUSED_COLOR: egui::Color32 = egui::Color32::YELLOW;

// ===== Navigation Phase =====

/// The two phases of keyboard inventory navigation.
///
/// The player starts in [`SlotNavigation`](NavigationPhase::SlotNavigation).
/// Pressing Enter while a slot with an item is highlighted advances to
/// [`ActionNavigation`](NavigationPhase::ActionNavigation). Pressing Enter
/// executes the focused action and returns to `SlotNavigation` at slot 0.
/// Pressing Esc cancels and returns to `SlotNavigation` at the previously
/// highlighted slot.
///
/// This enum is shared by all three inventory overlay systems so that each
/// nav-state struct uses the same phase type.
///
/// # Examples
///
/// ```
/// use antares::game::systems::inventory_ui_common::NavigationPhase;
///
/// let phase = NavigationPhase::default();
/// assert!(matches!(phase, NavigationPhase::SlotNavigation));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NavigationPhase {
    /// Arrows navigate the slot grid; Enter enters action mode.
    #[default]
    SlotNavigation,
    /// Left/Right arrows cycle action buttons; Enter executes; Esc cancels.
    ActionNavigation,
}

// ===== Shared hint text =====

/// Keyboard hint shown during [`NavigationPhase::SlotNavigation`] on every
/// split inventory overlay (merchant and container).
///
/// Both the merchant and container screens display exactly this string while
/// the player is navigating slots, so it is centralised here to guarantee the
/// two stay in sync. Each screen keeps its own distinct `ActionNavigation`
/// hint because the available actions differ (Sell/Buy vs. Take/Stash).
pub(crate) const SLOT_NAV_HINT: &str =
    "Tab: switch panel   1-6: change character   ←→↑↓: navigate   Enter: select   Esc: close";

// ===== Shared layout scaffolding =====

/// Render the active-character selector strip shared by the split inventory
/// overlays (merchant and container).
///
/// Draws a `Character:` label followed by one `[n] Name` button per party
/// member, highlighting the active character in yellow. The buttons are
/// informational only — character switching is driven by number keys in each
/// screen's input system, so the button click responses are intentionally
/// discarded.
///
/// # Arguments
///
/// * `ui` - The egui UI to draw into.
/// * `party` - The active party whose members become buttons.
/// * `active_char_idx` - Index of the currently active character (highlighted).
/// * `id_prefix` - Widget-id salt prefix. Each button uses
///   `format!("{id_prefix}_{i}")` so callers must pass a stable, unique prefix
///   (`"merch_char_btn"` for the merchant screen, `"cont_char_btn"` for the
///   container screen) to preserve egui widget identity and keyboard focus.
pub(crate) fn render_character_strip(
    ui: &mut egui::Ui,
    party: &crate::domain::character::Party,
    active_char_idx: usize,
    id_prefix: &str,
) {
    let party_len = party.members.len();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Character:").strong());
        for i in 0..party_len {
            ui.push_id(format!("{}_{}", id_prefix, i), |ui| {
                let member = &party.members[i];
                let is_active = i == active_char_idx;
                let label = egui::RichText::new(format!("[{}] {}", i + 1, member.name))
                    .color(if is_active {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::LIGHT_GRAY
                    })
                    .small();
                // Mouse clicks on character buttons are informational only;
                // switching is handled via number keys in the input system, so
                // the button's click Response is intentionally discarded.
                #[allow(clippy::let_underscore_must_use)]
                let _ = ui.button(label);
            });
        }
    });
}

/// Run the two inventory panels side-by-side with 8 px spacing, passing each
/// closure the computed half-width so panels size themselves consistently.
///
/// This reproduces the split-panel scaffold shared by the merchant and
/// container overlays: it reads `ui.available_size()` **before** entering the
/// horizontal layout, derives `half_w = (available.x - 8.0) / 2.0`, sets the
/// inter-panel spacing to 8 px, then invokes `left` and then `right` in that
/// order.
///
/// The panel height is intentionally **not** passed: callers that need it read
/// `ui.available_size().y` immediately before calling `split_panel` (the same
/// UI state this function samples) and capture it in each closure.
///
/// # Arguments
///
/// * `ui` - The egui UI to lay the two panels out in.
/// * `left` - Closure rendering the left panel; receives the child UI and
///   `half_w`.
/// * `right` - Closure rendering the right panel; receives the child UI and
///   `half_w`.
pub(crate) fn split_panel(
    ui: &mut egui::Ui,
    left: impl FnOnce(&mut egui::Ui, f32),
    right: impl FnOnce(&mut egui::Ui, f32),
) {
    let available = ui.available_size();
    let half_w = (available.x - 8.0) / 2.0;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        left(ui, half_w);
        right(ui, half_w);
    });
}
