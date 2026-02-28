// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Merchant inventory state for the application layer
//!
//! This module defines `MerchantInventoryState`, which tracks all UI state
//! for the split-screen merchant buy/sell interface.  The interface is entered
//! by pressing `I` while in `GameMode::Dialogue` with an NPC whose
//! `is_merchant` flag is `true`.
//!
//! ## Layout
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  [Character Name]    ←→  MERCHANT TRADE  ←→  [Merchant Name] │
//! ├────────────────────────┬────────────────────────────────────┤
//! │  Character Inventory   │        Merchant Stock              │
//! │  (LEFT PANEL)          │       (RIGHT PANEL)                │
//! │                        │                                    │
//! │  [slot grid]           │  [slot grid / stock list]          │
//! │                        │                                    │
//! │  [ Sell ]              │  [ Buy ]                           │
//! └────────────────────────┴────────────────────────────────────┘
//! ```
//!
//! ## Controls
//!
//! | Key           | Effect                                                      |
//! |---------------|-------------------------------------------------------------|
//! | `Tab`         | Toggle focus between Character panel (left) and NPC panel (right) |
//! | `1`–`6`       | Switch the active character whose inventory is shown on the left |
//! | `←→↑↓`        | Navigate the slot grid inside the focused panel             |
//! | `Enter`       | Enter action mode for the highlighted slot                  |
//! | `Esc`         | Close merchant inventory; return to `GameMode::Dialogue`     |
//!
//! ## Action buttons
//!
//! | Panel     | Action  | Effect                                                        |
//! |-----------|---------|---------------------------------------------------------------|
//! | Character | Sell    | Remove highlighted item from character; add gold to party     |
//! | Merchant  | Buy     | Remove highlighted stock entry; deduct gold; add item to lead char |

use crate::application::GameMode;
use crate::domain::world::npc::NpcId;
use serde::{Deserialize, Serialize};

/// Which panel has keyboard focus in the merchant inventory screen.
///
/// `Left` is the character panel; `Right` is the NPC/merchant panel.
///
/// # Examples
///
/// ```
/// use antares::application::merchant_inventory_state::MerchantFocus;
///
/// let focus = MerchantFocus::default();
/// assert!(matches!(focus, MerchantFocus::Left));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MerchantFocus {
    /// The character (left) panel has keyboard focus.
    #[default]
    Left,
    /// The merchant / NPC (right) panel has keyboard focus.
    Right,
}

/// State for the merchant buy/sell inventory screen.
///
/// Entered from `GameMode::Dialogue` when the player presses `I` while
/// interacting with a merchant NPC.  Stores enough information to render the
/// split-screen UI and to restore the previous dialogue state when the player
/// closes the screen.
///
/// ## Slot indices
///
/// * `character_selected_slot` — the highlighted slot in the **character**
///   inventory grid (left panel).  `None` means no slot is highlighted.
/// * `merchant_selected_slot` — the highlighted row in the **merchant** stock
///   list (right panel).  This is an index into the `MerchantStock::entries`
///   vec stored in `NpcRuntimeState`.  `None` means no row is highlighted.
///
/// # Examples
///
/// ```
/// use antares::application::GameMode;
/// use antares::application::merchant_inventory_state::MerchantInventoryState;
///
/// let state = MerchantInventoryState::new(
///     "merchant_bob".to_string(),
///     "Bob's Goods".to_string(),
///     0,
///     GameMode::Exploration,
/// );
///
/// assert_eq!(state.npc_id, "merchant_bob");
/// assert_eq!(state.npc_name, "Bob's Goods");
/// assert_eq!(state.active_character_index, 0);
/// assert!(state.character_selected_slot.is_none());
/// assert!(state.merchant_selected_slot.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantInventoryState {
    /// The NPC identifier of the merchant being traded with.
    ///
    /// Used to look up `NpcRuntimeState` (and thus `MerchantStock`) in
    /// `GameState::npc_runtime`.
    pub npc_id: NpcId,

    /// Display name of the merchant (shown in the right-panel header).
    pub npc_name: String,

    /// Index into `Party::members` of the character whose inventory is
    /// displayed in the left panel.
    ///
    /// Changed by pressing number keys `1`–`6` (maps to 0–5).
    /// Clamped to `0..party.members.len()` at render time.
    pub active_character_index: usize,

    /// Which panel currently has keyboard focus.
    pub focus: MerchantFocus,

    /// Highlighted slot index in the character inventory grid (left panel).
    ///
    /// `None` = no slot highlighted.
    pub character_selected_slot: Option<usize>,

    /// Highlighted stock entry index in the merchant stock list (right panel).
    ///
    /// `None` = no row highlighted.
    pub merchant_selected_slot: Option<usize>,

    /// The `GameMode` that was active when this screen was opened.
    ///
    /// Restored when the player closes the merchant inventory screen.
    ///
    /// Boxed to break the recursive `GameMode → MerchantInventoryState →
    /// GameMode` size cycle.
    pub previous_mode: Box<GameMode>,
}

impl MerchantInventoryState {
    /// Create a new `MerchantInventoryState`.
    ///
    /// # Arguments
    ///
    /// * `npc_id`               – ID of the merchant NPC.
    /// * `npc_name`             – Display name of the merchant.
    /// * `active_character_index` – Party index of the character to show first
    ///   (typically the party leader, index `0`).
    /// * `previous_mode`        – The mode to resume when the screen is closed.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::merchant_inventory_state::MerchantInventoryState;
    ///
    /// let state = MerchantInventoryState::new(
    ///     "smith_jenny".to_string(),
    ///     "Jenny the Blacksmith".to_string(),
    ///     0,
    ///     GameMode::Exploration,
    /// );
    ///
    /// assert_eq!(state.npc_id, "smith_jenny");
    /// assert_eq!(state.npc_name, "Jenny the Blacksmith");
    /// assert_eq!(state.active_character_index, 0);
    /// ```
    pub fn new(
        npc_id: NpcId,
        npc_name: String,
        active_character_index: usize,
        previous_mode: GameMode,
    ) -> Self {
        Self {
            npc_id,
            npc_name,
            active_character_index,
            focus: MerchantFocus::default(),
            character_selected_slot: None,
            merchant_selected_slot: None,
            previous_mode: Box::new(previous_mode),
        }
    }

    /// Return the mode that should be restored when the merchant screen closes.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::merchant_inventory_state::MerchantInventoryState;
    /// use antares::application::dialogue::DialogueState;
    ///
    /// let dialogue_mode = GameMode::Dialogue(DialogueState::new());
    /// let state = MerchantInventoryState::new(
    ///     "npc_001".to_string(),
    ///     "NPC".to_string(),
    ///     0,
    ///     dialogue_mode.clone(),
    /// );
    ///
    /// assert_eq!(state.get_resume_mode(), dialogue_mode);
    /// ```
    pub fn get_resume_mode(&self) -> GameMode {
        *self.previous_mode.clone()
    }

    /// Toggle panel focus between Left (character) and Right (merchant).
    ///
    /// Clears the selected slot on the panel losing focus so the cursor
    /// does not ghost across panels.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::merchant_inventory_state::{MerchantFocus, MerchantInventoryState};
    ///
    /// let mut state = MerchantInventoryState::new(
    ///     "npc".to_string(), "NPC".to_string(), 0, GameMode::Exploration,
    /// );
    ///
    /// assert!(matches!(state.focus, MerchantFocus::Left));
    /// state.toggle_focus();
    /// assert!(matches!(state.focus, MerchantFocus::Right));
    /// state.toggle_focus();
    /// assert!(matches!(state.focus, MerchantFocus::Left));
    /// ```
    pub fn toggle_focus(&mut self) {
        match self.focus {
            MerchantFocus::Left => {
                self.focus = MerchantFocus::Right;
                // Clear character slot selection when moving to the merchant panel
                self.character_selected_slot = None;
            }
            MerchantFocus::Right => {
                self.focus = MerchantFocus::Left;
                // Clear merchant slot selection when moving to the character panel
                self.merchant_selected_slot = None;
            }
        }
    }

    /// Switch the active character by party index.
    ///
    /// `index` is clamped to `0..party_size`.  If the index changes the
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
    /// use antares::application::merchant_inventory_state::MerchantInventoryState;
    ///
    /// let mut state = MerchantInventoryState::new(
    ///     "npc".to_string(), "NPC".to_string(), 0, GameMode::Exploration,
    /// );
    ///
    /// state.switch_character(2, 4);
    /// assert_eq!(state.active_character_index, 2);
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
    /// use antares::application::merchant_inventory_state::MerchantInventoryState;
    ///
    /// let state = MerchantInventoryState::new(
    ///     "npc".to_string(), "NPC".to_string(), 0, GameMode::Exploration,
    /// );
    /// assert!(state.character_has_focus());
    /// ```
    pub fn character_has_focus(&self) -> bool {
        self.focus == MerchantFocus::Left
    }

    /// Returns `true` if the right (merchant) panel currently has focus.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::GameMode;
    /// use antares::application::merchant_inventory_state::{MerchantFocus, MerchantInventoryState};
    ///
    /// let mut state = MerchantInventoryState::new(
    ///     "npc".to_string(), "NPC".to_string(), 0, GameMode::Exploration,
    /// );
    /// state.focus = MerchantFocus::Right;
    /// assert!(state.merchant_has_focus());
    /// ```
    pub fn merchant_has_focus(&self) -> bool {
        self.focus == MerchantFocus::Right
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameMode;

    fn make_state() -> MerchantInventoryState {
        MerchantInventoryState::new(
            "merchant_tom".to_string(),
            "Tom's Trading Post".to_string(),
            0,
            GameMode::Exploration,
        )
    }

    #[test]
    fn test_merchant_inventory_state_new_defaults() {
        let state = make_state();
        assert_eq!(state.npc_id, "merchant_tom");
        assert_eq!(state.npc_name, "Tom's Trading Post");
        assert_eq!(state.active_character_index, 0);
        assert!(matches!(state.focus, MerchantFocus::Left));
        assert!(state.character_selected_slot.is_none());
        assert!(state.merchant_selected_slot.is_none());
    }

    #[test]
    fn test_merchant_inventory_state_get_resume_mode() {
        let state = make_state();
        assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
    }

    #[test]
    fn test_merchant_inventory_state_get_resume_mode_dialogue() {
        use crate::application::dialogue::DialogueState;

        let dialogue = GameMode::Dialogue(DialogueState::new());
        let state =
            MerchantInventoryState::new("npc".to_string(), "NPC".to_string(), 0, dialogue.clone());
        assert_eq!(state.get_resume_mode(), dialogue);
    }

    #[test]
    fn test_toggle_focus_left_to_right() {
        let mut state = make_state();
        assert!(state.character_has_focus());
        state.toggle_focus();
        assert!(state.merchant_has_focus());
    }

    #[test]
    fn test_toggle_focus_right_to_left() {
        let mut state = make_state();
        state.toggle_focus();
        assert!(state.merchant_has_focus());
        state.toggle_focus();
        assert!(state.character_has_focus());
    }

    #[test]
    fn test_toggle_focus_clears_character_slot_when_moving_right() {
        let mut state = make_state();
        state.character_selected_slot = Some(3);
        state.toggle_focus();
        assert!(
            state.character_selected_slot.is_none(),
            "character slot should be cleared when focus moves to merchant"
        );
    }

    #[test]
    fn test_toggle_focus_clears_merchant_slot_when_moving_left() {
        let mut state = make_state();
        state.focus = MerchantFocus::Right;
        state.merchant_selected_slot = Some(2);
        state.toggle_focus();
        assert!(
            state.merchant_selected_slot.is_none(),
            "merchant slot should be cleared when focus moves to character"
        );
    }

    #[test]
    fn test_switch_character_changes_index() {
        let mut state = make_state();
        state.switch_character(2, 4);
        assert_eq!(state.active_character_index, 2);
    }

    #[test]
    fn test_switch_character_clears_selected_slot() {
        let mut state = make_state();
        state.character_selected_slot = Some(5);
        state.switch_character(1, 3);
        assert!(state.character_selected_slot.is_none());
    }

    #[test]
    fn test_switch_character_same_index_does_not_clear_slot() {
        let mut state = make_state();
        state.character_selected_slot = Some(5);
        // Switching to the same index should not clear the slot
        state.switch_character(0, 4);
        assert_eq!(
            state.character_selected_slot,
            Some(5),
            "slot should NOT be cleared when switching to the same character"
        );
    }

    #[test]
    fn test_switch_character_clamps_to_party_size() {
        let mut state = make_state();
        state.switch_character(10, 3); // only 3 members → max index is 2
        assert_eq!(state.active_character_index, 2);
    }

    #[test]
    fn test_switch_character_noop_on_empty_party() {
        let mut state = make_state();
        state.active_character_index = 1;
        state.switch_character(0, 0); // party_size == 0 → noop
        assert_eq!(state.active_character_index, 1);
    }

    #[test]
    fn test_character_has_focus_default() {
        let state = make_state();
        assert!(state.character_has_focus());
        assert!(!state.merchant_has_focus());
    }

    #[test]
    fn test_merchant_has_focus_after_toggle() {
        let mut state = make_state();
        state.toggle_focus();
        assert!(!state.character_has_focus());
        assert!(state.merchant_has_focus());
    }

    #[test]
    fn test_merchant_focus_default_is_left() {
        let focus = MerchantFocus::default();
        assert!(matches!(focus, MerchantFocus::Left));
    }

    #[test]
    fn test_merchant_focus_equality() {
        assert_eq!(MerchantFocus::Left, MerchantFocus::Left);
        assert_eq!(MerchantFocus::Right, MerchantFocus::Right);
        assert_ne!(MerchantFocus::Left, MerchantFocus::Right);
    }
}
