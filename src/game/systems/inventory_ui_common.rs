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
