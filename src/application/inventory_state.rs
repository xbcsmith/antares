// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Inventory state for the application layer
//!
//! This module defines `InventoryState`, which tracks which character panels are
//! open, which panel is focused, and which inventory slot is selected while the
//! player is in the inventory screen.
//!
//! The design mirrors `MenuState` from `src/application/menu.rs`: a
//! `Box<GameMode>` stores the mode that was active before the inventory opened
//! so it can be restored when the player closes the screen.
//!
//! # Examples
//!
//! ```
//! use antares::application::GameMode;
//! use antares::application::inventory_state::{InventoryState, InventoryViewMode};
//!
//! let state = InventoryState::new(GameMode::Exploration);
//! assert_eq!(state.focused_index, 0);
//! assert_eq!(state.open_panels, vec![0usize]);
//! assert_eq!(state.selected_slot, None);
//! assert_eq!(state.view_mode, InventoryViewMode::Multi);
//! assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
//! ```

use crate::application::GameMode;
use crate::domain::character::PARTY_MAX_SIZE;
use serde::{Deserialize, Serialize};

/// Controls whether the inventory screen shows all open panels in a grid
/// or focuses on a single character panel at full width.
///
/// `Multi` is the default view that displays all open character panels
/// simultaneously in a grid layout.  `Single` zooms into one panel,
/// giving it the full available width for detailed item management.
///
/// # Examples
///
/// ```
/// use antares::application::inventory_state::InventoryViewMode;
///
/// let mode = InventoryViewMode::default();
/// assert_eq!(mode, InventoryViewMode::Multi);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum InventoryViewMode {
    /// All `open_panels` are shown simultaneously in a grid.
    #[default]
    Multi,
    /// Only the `focused_index` panel is shown at full width.
    Single,
}

/// State for the inventory management screen.
///
/// Tracks which character panels are open, which panel currently has keyboard
/// focus, and which inventory slot (if any) is highlighted within the focused
/// panel.  The `previous_mode` field allows the game to resume whatever was
/// happening before the player opened the inventory.
///
/// `previous_mode` is boxed to break the recursive size dependency between
/// `InventoryState` and `GameMode::Inventory(InventoryState)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryState {
    /// The `GameMode` active before the inventory was opened.
    ///
    /// Restored via [`get_resume_mode`] when the player closes the inventory.
    ///
    /// [`get_resume_mode`]: InventoryState::get_resume_mode
    pub previous_mode: Box<GameMode>,

    /// Zero-based index of the character panel that currently has focus.
    ///
    /// Valid range: `0..party_size`. Wraps on [`tab_next`] / [`tab_prev`].
    ///
    /// [`tab_next`]: InventoryState::tab_next
    /// [`tab_prev`]: InventoryState::tab_prev
    pub focused_index: usize,

    /// Party-member indices whose panels are currently visible.
    ///
    /// Starts as `vec![0]` (only the first character's panel is open).
    /// Additional indices are pushed by [`tab_next`] / [`tab_prev`] up to
    /// [`PARTY_MAX_SIZE`].
    ///
    /// [`tab_next`]: InventoryState::tab_next
    /// [`tab_prev`]: InventoryState::tab_prev
    pub open_panels: Vec<usize>,

    /// Currently highlighted slot index within the focused panel.
    ///
    /// `None` means no slot is selected. Updated by [`select_next_slot`] and
    /// [`select_prev_slot`].
    ///
    /// [`select_next_slot`]: InventoryState::select_next_slot
    /// [`select_prev_slot`]: InventoryState::select_prev_slot
    pub selected_slot: Option<usize>,

    /// Whether the inventory screen is displaying all open panels (`Multi`) or
    /// zooming into a single panel (`Single`).
    ///
    /// Deserialises to [`InventoryViewMode::Multi`] when absent from a save file
    /// (via `#[serde(default)]`).
    #[serde(default)]
    pub view_mode: InventoryViewMode,
}

impl InventoryState {
    /// Create a new `InventoryState`, storing the mode that was active before
    /// opening the inventory.
    ///
    /// The new state has `focused_index = 0`, `open_panels = vec![0]`,
    /// `selected_slot = None`, and `view_mode = InventoryViewMode::Multi`.
    ///
    /// # Arguments
    ///
    /// * `previous_mode` – The `GameMode` to restore when the inventory closes.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::inventory_state::{InventoryState, InventoryViewMode};
    ///
    /// let state = InventoryState::new(GameMode::Exploration);
    /// assert_eq!(state.focused_index, 0);
    /// assert_eq!(state.open_panels, vec![0usize]);
    /// assert_eq!(state.selected_slot, None);
    /// assert_eq!(state.view_mode, InventoryViewMode::Multi);
    /// ```
    pub fn new(previous_mode: GameMode) -> Self {
        Self {
            previous_mode: Box::new(previous_mode),
            focused_index: 0,
            open_panels: vec![0],
            selected_slot: None,
            view_mode: InventoryViewMode::Multi,
        }
    }

    /// Return the `GameMode` to restore when closing the inventory.
    ///
    /// Clones the stored `previous_mode` to match the `MenuState::get_resume_mode`
    /// pattern.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::inventory_state::InventoryState;
    ///
    /// let state = InventoryState::new(GameMode::Exploration);
    /// assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
    /// ```
    pub fn get_resume_mode(&self) -> GameMode {
        (*self.previous_mode).clone()
    }

    /// Switch to `Single` view, focusing on the specified character index.
    ///
    /// Sets `view_mode` to [`InventoryViewMode::Single`], updates `focused_index`
    /// to `character_index`, and clears `selected_slot`.  The current
    /// `open_panels` list is preserved so the multi-view state is intact when
    /// the player returns via [`enter_multi_view`] or a Tab press.
    ///
    /// [`enter_multi_view`]: InventoryState::enter_multi_view
    ///
    /// # Arguments
    ///
    /// * `character_index` – Zero-based party index of the character to focus on.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::inventory_state::{InventoryState, InventoryViewMode};
    ///
    /// let mut state = InventoryState::new(GameMode::Exploration);
    /// state.open_panels = vec![0, 1, 2];
    /// state.enter_single_view(2);
    ///
    /// assert_eq!(state.view_mode, InventoryViewMode::Single);
    /// assert_eq!(state.focused_index, 2);
    /// assert_eq!(state.selected_slot, None);
    /// assert_eq!(state.open_panels, vec![0, 1, 2]); // preserved
    /// ```
    pub fn enter_single_view(&mut self, character_index: usize) {
        self.view_mode = InventoryViewMode::Single;
        self.focused_index = character_index;
        self.selected_slot = None;
    }

    /// Switch to `Multi` view, rebuilding `open_panels` from `party_size`.
    ///
    /// Sets `view_mode` to [`InventoryViewMode::Multi`] and replaces
    /// `open_panels` with `(0..party_size).collect()`.  If the resulting
    /// `open_panels` is empty (i.e. `party_size == 0`), index `0` is pushed as
    /// a fallback so there is always at least one visible panel.  `focused_index`
    /// is left unchanged.
    ///
    /// # Arguments
    ///
    /// * `party_size` – Number of members currently in the party.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::inventory_state::{InventoryState, InventoryViewMode};
    ///
    /// let mut state = InventoryState::new(GameMode::Exploration);
    /// state.focused_index = 1;
    /// state.enter_multi_view(3);
    ///
    /// assert_eq!(state.view_mode, InventoryViewMode::Multi);
    /// assert_eq!(state.open_panels, vec![0, 1, 2]);
    /// assert_eq!(state.focused_index, 1); // unchanged
    /// ```
    pub fn enter_multi_view(&mut self, party_size: usize) {
        self.view_mode = InventoryViewMode::Multi;
        self.open_panels = (0..party_size).collect();
        if self.open_panels.is_empty() {
            self.open_panels.push(0);
        }
    }

    /// Advance focus to the next character panel, wrapping around.
    ///
    /// If the state is currently in [`InventoryViewMode::Single`], calling
    /// `tab_next` first transitions back to Multi view (via [`enter_multi_view`])
    /// before advancing `focused_index`.
    ///
    /// If the newly focused index is not yet in `open_panels` **and** fewer than
    /// `PARTY_MAX_SIZE` panels are already open, the index is appended to
    /// `open_panels`.
    ///
    /// This is a no-op when `party_size == 0`.
    ///
    /// [`enter_multi_view`]: InventoryState::enter_multi_view
    ///
    /// # Arguments
    ///
    /// * `party_size` – Number of members currently in the party (`1..=PARTY_MAX_SIZE`).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::inventory_state::InventoryState;
    ///
    /// let mut state = InventoryState::new(GameMode::Exploration);
    /// state.tab_next(3);
    /// assert_eq!(state.focused_index, 1);
    /// assert!(state.open_panels.contains(&1));
    /// ```
    pub fn tab_next(&mut self, party_size: usize) {
        // If in Single view, pressing Tab expands back to Multi first, then cycles.
        if self.view_mode == InventoryViewMode::Single {
            self.enter_multi_view(party_size);
        }
        if party_size == 0 {
            return;
        }
        self.focused_index = (self.focused_index + 1) % party_size;
        if !self.open_panels.contains(&self.focused_index)
            && self.open_panels.len() < PARTY_MAX_SIZE
        {
            self.open_panels.push(self.focused_index);
        }
    }

    /// Move focus to the previous character panel, wrapping around.
    ///
    /// If the state is currently in [`InventoryViewMode::Single`], calling
    /// `tab_prev` first transitions back to Multi view (via [`enter_multi_view`])
    /// before decrementing `focused_index`.
    ///
    /// If the newly focused index is not yet in `open_panels` **and** fewer than
    /// `PARTY_MAX_SIZE` panels are already open, the index is appended to
    /// `open_panels`.
    ///
    /// This is a no-op when `party_size == 0`.
    ///
    /// [`enter_multi_view`]: InventoryState::enter_multi_view
    ///
    /// # Arguments
    ///
    /// * `party_size` – Number of members currently in the party (`1..=PARTY_MAX_SIZE`).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::inventory_state::InventoryState;
    ///
    /// let mut state = InventoryState::new(GameMode::Exploration);
    /// state.tab_prev(3); // wraps from 0 → 2
    /// assert_eq!(state.focused_index, 2);
    /// assert!(state.open_panels.contains(&2));
    /// ```
    pub fn tab_prev(&mut self, party_size: usize) {
        // If in Single view, pressing Tab expands back to Multi first, then cycles.
        if self.view_mode == InventoryViewMode::Single {
            self.enter_multi_view(party_size);
        }
        if party_size == 0 {
            return;
        }
        if self.focused_index == 0 {
            self.focused_index = party_size - 1;
        } else {
            self.focused_index -= 1;
        }
        if !self.open_panels.contains(&self.focused_index)
            && self.open_panels.len() < PARTY_MAX_SIZE
        {
            self.open_panels.push(self.focused_index);
        }
    }

    /// Remove the focused panel from `open_panels`.
    ///
    /// If this would leave `open_panels` empty, index `0` is re-added so there
    /// is always at least one visible panel.  The calling system should check
    /// whether the only remaining panel is `0` and transition back to
    /// `previous_mode` when appropriate.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::inventory_state::InventoryState;
    ///
    /// let mut state = InventoryState::new(GameMode::Exploration);
    /// // Panel 0 is the only open panel; closing it re-adds 0 to keep state valid.
    /// state.close_focused_panel();
    /// assert!(!state.open_panels.is_empty());
    /// ```
    pub fn close_focused_panel(&mut self) {
        self.open_panels.retain(|&idx| idx != self.focused_index);
        if self.open_panels.is_empty() {
            self.open_panels.push(0);
        }
    }

    /// Advance the selected slot index forward by one, wrapping around.
    ///
    /// If `selected_slot` is currently `None`, selection starts at index `1`
    /// (i.e. `(0 + 1) % slot_count`).
    ///
    /// This is a no-op when `slot_count == 0`.
    ///
    /// # Arguments
    ///
    /// * `slot_count` – Total number of inventory slots in the focused panel.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::inventory_state::InventoryState;
    ///
    /// let mut state = InventoryState::new(GameMode::Exploration);
    /// state.select_next_slot(6);
    /// assert_eq!(state.selected_slot, Some(1));
    /// ```
    pub fn select_next_slot(&mut self, slot_count: usize) {
        if slot_count == 0 {
            return;
        }
        let current = self.selected_slot.unwrap_or(0);
        self.selected_slot = Some((current + 1) % slot_count);
    }

    /// Move the selected slot index backward by one, wrapping around.
    ///
    /// If `selected_slot` is currently `None`, selection starts at index
    /// `slot_count - 1` (i.e. `0 - 1` with wrap).
    ///
    /// This is a no-op when `slot_count == 0`.
    ///
    /// # Arguments
    ///
    /// * `slot_count` – Total number of inventory slots in the focused panel.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::inventory_state::InventoryState;
    ///
    /// let mut state = InventoryState::new(GameMode::Exploration);
    /// state.select_prev_slot(6); // wraps: (0 - 1 + 6) % 6 = 5
    /// assert_eq!(state.selected_slot, Some(5));
    /// ```
    pub fn select_prev_slot(&mut self, slot_count: usize) {
        if slot_count == 0 {
            return;
        }
        let current = self.selected_slot.unwrap_or(0);
        self.selected_slot = Some(if current == 0 {
            slot_count - 1
        } else {
            current - 1
        });
    }
}

impl Default for InventoryState {
    /// Create a default `InventoryState` with `GameMode::Exploration` as the
    /// previous mode.
    fn default() -> Self {
        Self::new(GameMode::Exploration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameMode;

    // -------------------------------------------------------------------------
    // Construction / basic accessors
    // -------------------------------------------------------------------------

    /// `InventoryState::new` must initialise with focused_index 0, open_panels
    /// containing only index 0, and no slot selected.
    #[test]
    fn test_inventory_state_new() {
        let state = InventoryState::new(GameMode::Exploration);
        assert_eq!(state.focused_index, 0, "focused_index must start at 0");
        assert_eq!(
            state.open_panels,
            vec![0usize],
            "open_panels must start as [0]"
        );
        assert_eq!(
            state.selected_slot, None,
            "selected_slot must start as None"
        );
    }

    /// `get_resume_mode` must return a clone of the stored `previous_mode`.
    #[test]
    fn test_inventory_state_get_resume_mode_returns_previous_mode() {
        let state = InventoryState::new(GameMode::Exploration);
        assert!(
            matches!(state.get_resume_mode(), GameMode::Exploration),
            "get_resume_mode must return GameMode::Exploration"
        );
    }

    // -------------------------------------------------------------------------
    // tab_next
    // -------------------------------------------------------------------------

    /// After calling `tab_next` twice on a 3-member party starting at index 0,
    /// `open_panels` must be `[0, 1, 2]` and `focused_index` must be `2`.
    #[test]
    fn test_inventory_state_tab_next_opens_panels() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.tab_next(3);
        state.tab_next(3);
        assert_eq!(
            state.focused_index, 2,
            "focused_index must be 2 after two tab_next calls"
        );
        assert_eq!(
            state.open_panels,
            vec![0, 1, 2],
            "open_panels must be [0, 1, 2] after opening panels 1 and 2"
        );
    }

    /// `tab_next` must wrap `focused_index` back to `0` when called at the last
    /// index of a 2-member party.
    #[test]
    fn test_inventory_state_tab_next_wraps() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.focused_index = 1;
        state.tab_next(2);
        assert_eq!(
            state.focused_index, 0,
            "focused_index must wrap from 1 to 0 for a 2-member party"
        );
    }

    /// `tab_next` must be a no-op when `party_size` is 0.
    #[test]
    fn test_inventory_state_tab_next_noop_on_empty_party() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.tab_next(0);
        assert_eq!(state.focused_index, 0);
        assert_eq!(state.open_panels, vec![0usize]);
    }

    // -------------------------------------------------------------------------
    // tab_prev
    // -------------------------------------------------------------------------

    /// `tab_prev` on a 2-member party with `focused_index = 0` must wrap to `1`.
    #[test]
    fn test_inventory_state_tab_prev_wraps() {
        let mut state = InventoryState::new(GameMode::Exploration);
        assert_eq!(state.focused_index, 0);
        state.tab_prev(2);
        assert_eq!(
            state.focused_index, 1,
            "focused_index must wrap from 0 to 1 for a 2-member party"
        );
        assert!(
            state.open_panels.contains(&1),
            "panel 1 must be added to open_panels on first visit"
        );
    }

    /// `tab_prev` must be a no-op when `party_size` is 0.
    #[test]
    fn test_inventory_state_tab_prev_noop_on_empty_party() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.tab_prev(0);
        assert_eq!(state.focused_index, 0);
    }

    /// `tab_prev` from index 2 on a 3-member party must move to index 1.
    #[test]
    fn test_inventory_state_tab_prev_decrements() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.focused_index = 2;
        state.open_panels = vec![0, 1, 2];
        state.tab_prev(3);
        assert_eq!(state.focused_index, 1);
    }

    // -------------------------------------------------------------------------
    // close_focused_panel
    // -------------------------------------------------------------------------

    /// Closing a panel that has a sibling removes it from `open_panels`.
    #[test]
    fn test_inventory_state_close_focused_panel() {
        let mut state = InventoryState::new(GameMode::Exploration);
        // Open a second panel manually so we have two panels.
        state.open_panels.push(1);
        state.focused_index = 1;

        state.close_focused_panel();

        assert!(
            !state.open_panels.contains(&1),
            "panel 1 must be removed from open_panels"
        );
        assert!(
            !state.open_panels.is_empty(),
            "open_panels must not be empty after close"
        );
    }

    /// Closing the last (only) open panel must re-add panel 0 to keep state
    /// valid — the inventory is open but at least one panel must remain visible.
    #[test]
    fn test_inventory_state_close_last_panel_keeps_one() {
        let mut state = InventoryState::new(GameMode::Exploration);
        // Only panel 0 is open (default).
        state.close_focused_panel();
        assert!(
            !state.open_panels.is_empty(),
            "open_panels must not be empty after closing the last panel"
        );
        assert!(
            state.open_panels.contains(&0),
            "panel 0 must be re-added when it was the last open panel"
        );
    }

    // -------------------------------------------------------------------------
    // select_next_slot / select_prev_slot
    // -------------------------------------------------------------------------

    /// With 6 slots and no prior selection, `select_next_slot(6)` must set
    /// `selected_slot = Some(1)`.
    #[test]
    fn test_inventory_state_select_next_slot() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.select_next_slot(6);
        assert_eq!(
            state.selected_slot,
            Some(1),
            "select_next_slot from None (treated as 0) must yield Some(1)"
        );
    }

    /// `select_next_slot` must wrap from the last slot back to `0`.
    #[test]
    fn test_inventory_state_select_next_slot_wraps() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.selected_slot = Some(5);
        state.select_next_slot(6);
        assert_eq!(
            state.selected_slot,
            Some(0),
            "select_next_slot must wrap from slot 5 to slot 0 with 6 slots"
        );
    }

    /// `select_next_slot` must be a no-op when `slot_count == 0`.
    #[test]
    fn test_inventory_state_select_next_slot_noop_on_zero() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.select_next_slot(0);
        assert_eq!(state.selected_slot, None);
    }

    /// With 6 slots and no prior selection, `select_prev_slot(6)` must set
    /// `selected_slot = Some(5)` (wraps: `(0 - 1 + 6) % 6 = 5`).
    #[test]
    fn test_inventory_state_select_prev_slot() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.select_prev_slot(6);
        assert_eq!(
            state.selected_slot,
            Some(5),
            "select_prev_slot from None (treated as 0) must wrap to Some(5)"
        );
    }

    /// `select_prev_slot` must move from slot 3 to slot 2.
    #[test]
    fn test_inventory_state_select_prev_slot_decrements() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.selected_slot = Some(3);
        state.select_prev_slot(6);
        assert_eq!(state.selected_slot, Some(2));
    }

    /// `select_prev_slot` must be a no-op when `slot_count == 0`.
    #[test]
    fn test_inventory_state_select_prev_slot_noop_on_zero() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.select_prev_slot(0);
        assert_eq!(state.selected_slot, None);
    }

    // -------------------------------------------------------------------------
    // Default impl
    // -------------------------------------------------------------------------

    /// `InventoryState::default()` must produce the same state as
    /// `InventoryState::new(GameMode::Exploration)`.
    #[test]
    fn test_inventory_state_default_matches_new_exploration() {
        let default_state = InventoryState::default();
        let explicit_state = InventoryState::new(GameMode::Exploration);
        assert_eq!(default_state, explicit_state);
    }

    // -------------------------------------------------------------------------
    // InventoryViewMode
    // -------------------------------------------------------------------------

    /// `InventoryViewMode::default()` must return `InventoryViewMode::Multi`.
    #[test]
    fn test_inventory_view_mode_default_is_multi() {
        assert_eq!(
            InventoryViewMode::default(),
            InventoryViewMode::Multi,
            "default view mode must be Multi"
        );
    }

    // -------------------------------------------------------------------------
    // enter_single_view
    // -------------------------------------------------------------------------

    /// `enter_single_view(2)` on a state with `focused_index = 0` must set
    /// `view_mode = Single`, `focused_index = 2`, and `selected_slot = None`.
    #[test]
    fn test_enter_single_view_sets_mode_and_updates_index() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.focused_index = 0;
        state.selected_slot = Some(3);
        state.enter_single_view(2);
        assert_eq!(
            state.view_mode,
            InventoryViewMode::Single,
            "view_mode must be Single after enter_single_view"
        );
        assert_eq!(
            state.focused_index, 2,
            "focused_index must be updated to the supplied character_index"
        );
        assert_eq!(
            state.selected_slot, None,
            "selected_slot must be cleared by enter_single_view"
        );
    }

    /// `enter_single_view` must not modify `open_panels`.
    #[test]
    fn test_enter_single_view_preserves_open_panels() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.open_panels = vec![0, 1, 2];
        state.enter_single_view(1);
        assert_eq!(
            state.open_panels,
            vec![0, 1, 2],
            "open_panels must be unchanged after enter_single_view"
        );
    }

    // -------------------------------------------------------------------------
    // enter_multi_view
    // -------------------------------------------------------------------------

    /// `enter_multi_view(3)` must set `view_mode = Multi` and rebuild
    /// `open_panels` as `[0, 1, 2]`.
    #[test]
    fn test_enter_multi_view_sets_mode_and_restores_panels() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.enter_multi_view(3);
        assert_eq!(
            state.view_mode,
            InventoryViewMode::Multi,
            "view_mode must be Multi after enter_multi_view"
        );
        assert_eq!(
            state.open_panels,
            vec![0, 1, 2],
            "open_panels must be rebuilt as [0, 1, 2] for party_size 3"
        );
    }

    /// When `party_size == 0`, `enter_multi_view` must push index 0 as fallback
    /// so `open_panels` is never empty.
    #[test]
    fn test_enter_multi_view_empty_party_keeps_panel_zero() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.enter_multi_view(0);
        assert_eq!(
            state.open_panels,
            vec![0],
            "open_panels must fall back to [0] when party_size is 0"
        );
    }

    /// `enter_multi_view` must not change `focused_index`.
    #[test]
    fn test_enter_multi_view_preserves_focused_index() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.focused_index = 2;
        state.enter_multi_view(4);
        assert_eq!(
            state.focused_index, 2,
            "focused_index must be unchanged after enter_multi_view"
        );
    }

    // -------------------------------------------------------------------------
    // tab_next / tab_prev — Single-view interactions
    // -------------------------------------------------------------------------

    /// `tab_next` called while in Single view must first expand to Multi view
    /// and then advance `focused_index` by one.
    #[test]
    fn test_tab_next_from_single_view_enters_multi_and_advances() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.enter_single_view(0);
        assert_eq!(state.view_mode, InventoryViewMode::Single);

        state.tab_next(3);

        assert_eq!(
            state.view_mode,
            InventoryViewMode::Multi,
            "tab_next must transition to Multi view when called from Single"
        );
        assert_eq!(
            state.focused_index, 1,
            "focused_index must advance from 0 to 1 after tab_next"
        );
        assert!(
            state.open_panels.contains(&1),
            "open_panels must contain 1 after tab_next"
        );
    }

    /// `tab_prev` called while in Single view must first expand to Multi view
    /// and then decrement `focused_index` by one.
    #[test]
    fn test_tab_prev_from_single_view_enters_multi_and_retreats() {
        let mut state = InventoryState::new(GameMode::Exploration);
        state.enter_single_view(2);
        assert_eq!(state.view_mode, InventoryViewMode::Single);

        state.tab_prev(3);

        assert_eq!(
            state.view_mode,
            InventoryViewMode::Multi,
            "tab_prev must transition to Multi view when called from Single"
        );
        assert_eq!(
            state.focused_index, 1,
            "focused_index must decrement from 2 to 1 after tab_prev"
        );
    }

    // -------------------------------------------------------------------------
    // new() — view_mode default
    // -------------------------------------------------------------------------

    /// `InventoryState::new` must initialise `view_mode` as `InventoryViewMode::Multi`.
    #[test]
    fn test_inventory_state_new_view_mode_defaults_to_multi() {
        let state = InventoryState::new(GameMode::Exploration);
        assert_eq!(
            state.view_mode,
            InventoryViewMode::Multi,
            "view_mode must default to Multi on construction"
        );
    }
}
