// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Container inventory state for the application layer
//!
//! This module defines `ContainerInventoryState`, which tracks all UI state
//! for the split-screen container interaction interface.  The interface is
//! entered by pressing `E` while the party is facing a container tile event
//! (chest, crate, hole in the wall, etc.).
//!
//! ## Layout
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  [Character Name]    ←→  CONTAINER    ←→  [Container Name]  │
//! ├────────────────────────┬────────────────────────────────────┤
//! │  Character Inventory   │       Container Contents           │
//! │  (LEFT PANEL)          │       (RIGHT PANEL)                │
//! │                        │                                    │
//! │  [slot grid]           │  [slot list / grid]                │
//! │                        │                                    │
//! │  [ Stash ]             │  [ Take ]  [ Take All ]            │
//! └────────────────────────┴────────────────────────────────────┘
//! ```
//!
//! ## Controls
//!
//! | Key           | Effect                                                            |
//! |---------------|-------------------------------------------------------------------|
//! | `Tab`         | Toggle focus between Character panel (left) and Container panel (right) |
//! | `1`–`6`       | Switch the active character whose inventory is shown on the left  |
//! | `←→↑↓`        | Navigate the slot grid inside the focused panel                   |
//! | `Enter`       | Enter action mode for the highlighted slot                        |
//! | `Esc`         | Close container inventory; return to previous mode                |
//!
//! ## Action buttons
//!
//! | Panel     | Action   | Effect                                                           |
//! |-----------|----------|------------------------------------------------------------------|
//! | Container | Take     | Move highlighted container item into the active character's inventory |
//! | Container | Take All | Move all container items into the active character's inventory   |
//! | Character | Stash    | Move highlighted character item into the container               |

use crate::application::GameMode;
use crate::domain::character::InventorySlot;
use serde::{Deserialize, Serialize};

/// Which panel has keyboard focus in the container inventory screen.
///
/// `Left` is the character panel; `Right` is the container panel.
///
/// # Examples
///
/// ```
/// use antares::application::container_inventory_state::ContainerFocus;
///
/// let focus = ContainerFocus::default();
/// assert!(matches!(focus, ContainerFocus::Left));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ContainerFocus {
    /// The character (left) panel has keyboard focus.
    #[default]
    Left,
    /// The container (right) panel has keyboard focus.
    Right,
}

/// State for the container interaction inventory screen.
///
/// Entered from exploration mode when the player presses `E` while facing a
/// container tile event (chest, hole in the wall, crate, etc.).  Stores the
/// container's current item contents as a `Vec<InventorySlot>` (mutable
/// runtime state) alongside the UI focus / selection information.
///
/// ## Slot indices
///
/// * `character_selected_slot` — highlighted slot in the **character** inventory
///   grid (left panel). `None` means no slot is highlighted.
/// * `container_selected_slot` — highlighted slot in the **container** item list
///   (right panel). `None` means no slot is highlighted.
///
/// ## Container identity
///
/// The container is identified by `container_event_id` (the `EventId` string
/// of the map event that triggered the interaction).  This ID is used by the
/// action system to persist changes back to the map event data (so the
/// container is empty after the player takes all items).
///
/// # Examples
///
/// ```
/// use antares::application::GameMode;
/// use antares::application::container_inventory_state::ContainerInventoryState;
/// use antares::domain::character::InventorySlot;
///
/// let items = vec![
///     InventorySlot { item_id: 1, charges: 0 },
///     InventorySlot { item_id: 5, charges: 3 },
/// ];
///
/// let state = ContainerInventoryState::new(
///     "chest_001".to_string(),
///     "Wooden Chest".to_string(),
///     items,
///     0,
///     GameMode::Exploration,
/// );
///
/// assert_eq!(state.container_event_id, "chest_001");
/// assert_eq!(state.container_name, "Wooden Chest");
/// assert_eq!(state.items.len(), 2);
/// assert_eq!(state.active_character_index, 0);
/// assert!(state.character_selected_slot.is_none());
/// assert!(state.container_selected_slot.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerInventoryState {
    /// The event ID of the map event that represents this container.
    ///
    /// Used to write the updated item list back to the map event after
    /// the player takes items or stashes items into the container.
    pub container_event_id: String,

    /// Display name of the container (shown in the right-panel header).
    pub container_name: String,

    /// Current runtime contents of the container.
    ///
    /// Items are removed from this list when the player takes them.
    /// Items are added to this list when the player stashes them.
    pub items: Vec<InventorySlot>,

    /// Index into `Party::members` of the character whose inventory is
    /// displayed in the left panel.
    ///
    /// Changed by pressing number keys `1`–`6` (maps to 0–5).
    /// Clamped to `0..party.members.len()` at render time.
    pub active_character_index: usize,

    /// Which panel currently has keyboard focus.
    pub focus: ContainerFocus,

    /// Highlighted slot index in the character inventory grid (left panel).
    ///
    /// `None` = no slot highlighted.
    pub character_selected_slot: Option<usize>,

    /// Highlighted slot index in the container item list (right panel).
    ///
    /// `None` = no slot highlighted.
    pub container_selected_slot: Option<usize>,

    /// The `GameMode` that was active when this screen was opened.
    ///
    /// Restored when the player closes the container inventory screen.
    ///
    /// Boxed to break the recursive `GameMode → ContainerInventoryState →
    /// GameMode` size cycle.
    pub previous_mode: Box<GameMode>,
}

impl ContainerInventoryState {
    /// Create a new `ContainerInventoryState`.
    ///
    /// # Arguments
    ///
    /// * `container_event_id`     – The map event ID of the container.
    /// * `container_name`         – Display name shown in the right-panel header.
    /// * `items`                  – Current item list inside the container.
    /// * `active_character_index` – Party index of the character to show first.
    /// * `previous_mode`          – The mode to resume when the screen is closed.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::container_inventory_state::ContainerInventoryState;
    ///
    /// let state = ContainerInventoryState::new(
    ///     "barrel_42".to_string(),
    ///     "Old Barrel".to_string(),
    ///     vec![],
    ///     0,
    ///     GameMode::Exploration,
    /// );
    ///
    /// assert_eq!(state.container_event_id, "barrel_42");
    /// assert!(state.items.is_empty());
    /// ```
    pub fn new(
        container_event_id: String,
        container_name: String,
        items: Vec<InventorySlot>,
        active_character_index: usize,
        previous_mode: GameMode,
    ) -> Self {
        Self {
            container_event_id,
            container_name,
            items,
            active_character_index,
            focus: ContainerFocus::default(),
            character_selected_slot: None,
            container_selected_slot: None,
            previous_mode: Box::new(previous_mode),
        }
    }

    /// Return the mode that should be restored when the container screen closes.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::container_inventory_state::ContainerInventoryState;
    ///
    /// let state = ContainerInventoryState::new(
    ///     "chest".to_string(),
    ///     "Chest".to_string(),
    ///     vec![],
    ///     0,
    ///     GameMode::Exploration,
    /// );
    ///
    /// assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
    /// ```
    pub fn get_resume_mode(&self) -> GameMode {
        *self.previous_mode.clone()
    }

    /// Toggle panel focus between Left (character) and Right (container).
    ///
    /// Clears the selected slot on the panel losing focus so the cursor
    /// does not ghost across panels.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::container_inventory_state::{ContainerFocus, ContainerInventoryState};
    ///
    /// let mut state = ContainerInventoryState::new(
    ///     "chest".to_string(), "Chest".to_string(), vec![], 0, GameMode::Exploration,
    /// );
    ///
    /// assert!(matches!(state.focus, ContainerFocus::Left));
    /// state.toggle_focus();
    /// assert!(matches!(state.focus, ContainerFocus::Right));
    /// state.toggle_focus();
    /// assert!(matches!(state.focus, ContainerFocus::Left));
    /// ```
    pub fn toggle_focus(&mut self) {
        match self.focus {
            ContainerFocus::Left => {
                self.focus = ContainerFocus::Right;
                // Clear character slot selection when moving to container panel
                self.character_selected_slot = None;
            }
            ContainerFocus::Right => {
                self.focus = ContainerFocus::Left;
                // Clear container slot selection when moving to character panel
                self.container_selected_slot = None;
            }
        }
    }

    /// Switch the active character by party index.
    ///
    /// `index` is clamped to `0..party_size`.  If the index changes, the
    /// character selected slot is cleared because the new character's
    /// inventory may be entirely different.
    ///
    /// # Arguments
    ///
    /// * `index`      – Desired party index (0-based).
    /// * `party_size` – Current number of party members (for bounds clamping).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::container_inventory_state::ContainerInventoryState;
    ///
    /// let mut state = ContainerInventoryState::new(
    ///     "chest".to_string(), "Chest".to_string(), vec![], 0, GameMode::Exploration,
    /// );
    ///
    /// state.switch_character(3, 6);
    /// assert_eq!(state.active_character_index, 3);
    /// assert!(state.character_selected_slot.is_none());
    /// ```
    pub fn switch_character(&mut self, index: usize, party_size: usize) {
        if party_size == 0 {
            return;
        }
        let clamped = index.min(party_size - 1);
        if clamped != self.active_character_index {
            self.active_character_index = clamped;
            self.character_selected_slot = None;
        }
    }

    /// Returns `true` if the left (character) panel currently has focus.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::container_inventory_state::ContainerInventoryState;
    ///
    /// let state = ContainerInventoryState::new(
    ///     "chest".to_string(), "Chest".to_string(), vec![], 0, GameMode::Exploration,
    /// );
    /// assert!(state.character_has_focus());
    /// ```
    pub fn character_has_focus(&self) -> bool {
        self.focus == ContainerFocus::Left
    }

    /// Returns `true` if the right (container) panel currently has focus.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::container_inventory_state::{ContainerFocus, ContainerInventoryState};
    ///
    /// let mut state = ContainerInventoryState::new(
    ///     "chest".to_string(), "Chest".to_string(), vec![], 0, GameMode::Exploration,
    /// );
    /// state.focus = ContainerFocus::Right;
    /// assert!(state.container_has_focus());
    /// ```
    pub fn container_has_focus(&self) -> bool {
        self.focus == ContainerFocus::Right
    }

    /// Returns the number of items currently in the container.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::container_inventory_state::ContainerInventoryState;
    /// use antares::domain::character::InventorySlot;
    ///
    /// let items = vec![InventorySlot { item_id: 1, charges: 0 }];
    /// let state = ContainerInventoryState::new(
    ///     "chest".to_string(), "Chest".to_string(), items, 0, GameMode::Exploration,
    /// );
    /// assert_eq!(state.item_count(), 1);
    /// ```
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the container has no items.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::container_inventory_state::ContainerInventoryState;
    ///
    /// let state = ContainerInventoryState::new(
    ///     "chest".to_string(), "Chest".to_string(), vec![], 0, GameMode::Exploration,
    /// );
    /// assert!(state.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Remove and return the item at `slot_index` from the container.
    ///
    /// Returns `None` if the index is out of bounds.  If the removal
    /// leaves the container empty and `container_selected_slot` pointed
    /// to the removed slot, the selection is cleared.
    ///
    /// # Arguments
    ///
    /// * `slot_index` – Zero-based index into `self.items`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::container_inventory_state::ContainerInventoryState;
    /// use antares::domain::character::InventorySlot;
    ///
    /// let items = vec![
    ///     InventorySlot { item_id: 10, charges: 0 },
    ///     InventorySlot { item_id: 20, charges: 0 },
    /// ];
    /// let mut state = ContainerInventoryState::new(
    ///     "chest".to_string(), "Chest".to_string(), items, 0, GameMode::Exploration,
    /// );
    ///
    /// let taken = state.take_item(0);
    /// assert!(taken.is_some());
    /// assert_eq!(taken.unwrap().item_id, 10);
    /// assert_eq!(state.items.len(), 1);
    /// ```
    pub fn take_item(&mut self, slot_index: usize) -> Option<InventorySlot> {
        if slot_index >= self.items.len() {
            return None;
        }
        let item = self.items.remove(slot_index);
        // Clamp or clear the container selection after removal
        if let Some(sel) = self.container_selected_slot {
            if self.items.is_empty() {
                self.container_selected_slot = None;
            } else if sel >= self.items.len() {
                self.container_selected_slot = Some(self.items.len() - 1);
            }
        }
        Some(item)
    }

    /// Drain and return all items from the container.
    ///
    /// After this call `self.items` is empty and `container_selected_slot`
    /// is cleared.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::container_inventory_state::ContainerInventoryState;
    /// use antares::domain::character::InventorySlot;
    ///
    /// let items = vec![
    ///     InventorySlot { item_id: 1, charges: 0 },
    ///     InventorySlot { item_id: 2, charges: 0 },
    /// ];
    /// let mut state = ContainerInventoryState::new(
    ///     "chest".to_string(), "Chest".to_string(), items, 0, GameMode::Exploration,
    /// );
    ///
    /// let all = state.take_all();
    /// assert_eq!(all.len(), 2);
    /// assert!(state.items.is_empty());
    /// assert!(state.container_selected_slot.is_none());
    /// ```
    pub fn take_all(&mut self) -> Vec<InventorySlot> {
        self.container_selected_slot = None;
        std::mem::take(&mut self.items)
    }

    /// Add an item to the container (used by the Stash action).
    ///
    /// # Arguments
    ///
    /// * `slot` – The `InventorySlot` to push into `self.items`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::container_inventory_state::ContainerInventoryState;
    /// use antares::domain::character::InventorySlot;
    ///
    /// let mut state = ContainerInventoryState::new(
    ///     "chest".to_string(), "Chest".to_string(), vec![], 0, GameMode::Exploration,
    /// );
    ///
    /// state.stash_item(InventorySlot { item_id: 7, charges: 0 });
    /// assert_eq!(state.items.len(), 1);
    /// assert_eq!(state.items[0].item_id, 7);
    /// ```
    pub fn stash_item(&mut self, slot: InventorySlot) {
        self.items.push(slot);
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameMode;
    use crate::domain::character::InventorySlot;
    use crate::domain::types::ItemId;

    fn make_slot(item_id: ItemId) -> InventorySlot {
        InventorySlot {
            item_id,
            charges: 0,
        }
    }

    fn make_state() -> ContainerInventoryState {
        ContainerInventoryState::new(
            "chest_001".to_string(),
            "Old Chest".to_string(),
            vec![make_slot(1), make_slot(2), make_slot(3)],
            0,
            GameMode::Exploration,
        )
    }

    #[test]
    fn test_container_inventory_state_new_defaults() {
        let state = make_state();
        assert_eq!(state.container_event_id, "chest_001");
        assert_eq!(state.container_name, "Old Chest");
        assert_eq!(state.items.len(), 3);
        assert_eq!(state.active_character_index, 0);
        assert!(matches!(state.focus, ContainerFocus::Left));
        assert!(state.character_selected_slot.is_none());
        assert!(state.container_selected_slot.is_none());
    }

    #[test]
    fn test_container_inventory_state_get_resume_mode() {
        let state = make_state();
        assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
    }

    #[test]
    fn test_toggle_focus_left_to_right() {
        let mut state = make_state();
        assert!(state.character_has_focus());
        state.toggle_focus();
        assert!(state.container_has_focus());
    }

    #[test]
    fn test_toggle_focus_right_to_left() {
        let mut state = make_state();
        state.toggle_focus();
        state.toggle_focus();
        assert!(state.character_has_focus());
    }

    #[test]
    fn test_toggle_focus_clears_character_slot() {
        let mut state = make_state();
        state.character_selected_slot = Some(2);
        state.toggle_focus(); // Left → Right
        assert!(
            state.character_selected_slot.is_none(),
            "character slot should clear when moving to container panel"
        );
    }

    #[test]
    fn test_toggle_focus_clears_container_slot() {
        let mut state = make_state();
        state.focus = ContainerFocus::Right;
        state.container_selected_slot = Some(1);
        state.toggle_focus(); // Right → Left
        assert!(
            state.container_selected_slot.is_none(),
            "container slot should clear when moving to character panel"
        );
    }

    #[test]
    fn test_switch_character_changes_index() {
        let mut state = make_state();
        state.switch_character(2, 4);
        assert_eq!(state.active_character_index, 2);
    }

    #[test]
    fn test_switch_character_clears_slot() {
        let mut state = make_state();
        state.character_selected_slot = Some(4);
        state.switch_character(1, 3);
        assert!(state.character_selected_slot.is_none());
    }

    #[test]
    fn test_switch_character_same_index_preserves_slot() {
        let mut state = make_state();
        state.character_selected_slot = Some(4);
        state.switch_character(0, 4); // same index
        assert_eq!(state.character_selected_slot, Some(4));
    }

    #[test]
    fn test_switch_character_clamps_to_party_size() {
        let mut state = make_state();
        state.switch_character(99, 3);
        assert_eq!(state.active_character_index, 2);
    }

    #[test]
    fn test_switch_character_noop_on_empty_party() {
        let mut state = make_state();
        state.active_character_index = 1;
        state.switch_character(0, 0);
        assert_eq!(state.active_character_index, 1);
    }

    #[test]
    fn test_item_count() {
        let state = make_state();
        assert_eq!(state.item_count(), 3);
    }

    #[test]
    fn test_is_empty_false() {
        let state = make_state();
        assert!(!state.is_empty());
    }

    #[test]
    fn test_is_empty_true() {
        let state = ContainerInventoryState::new(
            "c".to_string(),
            "C".to_string(),
            vec![],
            0,
            GameMode::Exploration,
        );
        assert!(state.is_empty());
    }

    #[test]
    fn test_take_item_removes_correct_item() {
        let mut state = make_state();
        let taken = state.take_item(1);
        assert!(taken.is_some());
        assert_eq!(taken.unwrap().item_id, 2);
        assert_eq!(state.items.len(), 2);
    }

    #[test]
    fn test_take_item_out_of_bounds_returns_none() {
        let mut state = make_state();
        let taken = state.take_item(99);
        assert!(taken.is_none());
        assert_eq!(state.items.len(), 3);
    }

    #[test]
    fn test_take_item_clamps_container_selection_after_removal() {
        let mut state = make_state(); // 3 items: indices 0, 1, 2
        state.container_selected_slot = Some(2); // last item selected
        state.take_item(0); // remove first → 2 items remain, old index 2 is now OOB
        assert_eq!(state.container_selected_slot, Some(1));
    }

    #[test]
    fn test_take_item_clears_selection_when_last_item_removed() {
        let mut state = ContainerInventoryState::new(
            "c".to_string(),
            "C".to_string(),
            vec![InventorySlot {
                item_id: 42,
                charges: 0,
            }],
            0,
            GameMode::Exploration,
        );
        state.container_selected_slot = Some(0);
        state.take_item(0);
        assert!(state.container_selected_slot.is_none());
    }

    #[test]
    fn test_take_all_drains_items() {
        let mut state = make_state();
        let all = state.take_all();
        assert_eq!(all.len(), 3);
        assert!(state.items.is_empty());
        assert!(state.container_selected_slot.is_none());
    }

    #[test]
    fn test_take_all_on_empty_container() {
        let mut state = ContainerInventoryState::new(
            "c".to_string(),
            "C".to_string(),
            vec![],
            0,
            GameMode::Exploration,
        );
        let all = state.take_all();
        assert!(all.is_empty());
    }

    #[test]
    fn test_stash_item_adds_to_container() {
        let mut state = make_state();
        let initial_len = state.items.len();
        state.stash_item(InventorySlot {
            item_id: 99,
            charges: 0,
        });
        assert_eq!(state.items.len(), initial_len + 1);
        assert_eq!(state.items.last().unwrap().item_id, 99);
    }

    #[test]
    fn test_container_focus_default_is_left() {
        let focus = ContainerFocus::default();
        assert!(matches!(focus, ContainerFocus::Left));
    }

    #[test]
    fn test_container_focus_equality() {
        assert_eq!(ContainerFocus::Left, ContainerFocus::Left);
        assert_eq!(ContainerFocus::Right, ContainerFocus::Right);
        assert_ne!(ContainerFocus::Left, ContainerFocus::Right);
    }
}
