// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Spell book management screen state.
//!
//! Tracks which party member's spell book is currently open and which spell
//! (if any) is highlighted in the list.  The screen is read-only: the player
//! can browse known spells, view descriptions, and inspect learnable scrolls
//! in the character's inventory, but no spell is cast from here.
//!
//! `SpellBookState` is stored inside
//! [`crate::application::GameMode::SpellBook`].  It uses a `Box<GameMode>` for
//! the previous mode (same pattern as
//! [`crate::application::spell_casting_state::SpellCastingState`] and
//! [`crate::application::inventory_state::InventoryState`]) to break the
//! recursive type dependency.
//!
//! # Lifetime
//!
//! The state is created when the player opens the Spell Book (default key `B`)
//! and destroyed when the player presses Esc (returning to whatever mode was
//! active) or presses `C` (which transitions directly to the spell-casting
//! flow for the currently browsed character).
//!
//! # Examples
//!
//! ```
//! use antares::application::spell_book_state::SpellBookState;
//! use antares::application::GameMode;
//!
//! let state = SpellBookState::new(0, GameMode::Exploration);
//! assert_eq!(state.character_index, 0);
//! assert!(state.selected_spell_id.is_none());
//! assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
//! ```

use crate::application::GameMode;
use crate::domain::types::SpellId;
use serde::{Deserialize, Serialize};

/// In-game spell book management screen state.
///
/// Stored inside [`GameMode::SpellBook`].  The `previous_mode` field is boxed
/// to break the recursive size dependency between `SpellBookState` and
/// `GameMode::SpellBook(SpellBookState)`.
///
/// # Examples
///
/// ```
/// use antares::application::spell_book_state::SpellBookState;
/// use antares::application::GameMode;
///
/// // Open the spell book on the second party member.
/// let mut state = SpellBookState::new(1, GameMode::Exploration);
/// assert_eq!(state.character_index, 1);
///
/// // Highlight a spell.
/// state.selected_spell_id = Some(0x0101);
/// assert_eq!(state.selected_spell_id, Some(0x0101));
///
/// // Restore the previous mode on close.
/// assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpellBookState {
    /// Index into `party.members` of the character whose spell book is open.
    pub character_index: usize,

    /// Spell currently highlighted in the list.
    ///
    /// `None` until the player moves the cursor over a spell entry.
    pub selected_spell_id: Option<SpellId>,

    /// Row cursor inside the spell list (0-based).
    ///
    /// Used for keyboard navigation (Up/Down arrows).
    pub selected_row: usize,

    /// The `GameMode` that was active before the Spell Book was opened.
    ///
    /// Restored when the player presses Esc.  Boxed to break the recursive
    /// size dependency.
    pub previous_mode: Box<GameMode>,
}

impl SpellBookState {
    /// Creates a new `SpellBookState` for the given party member index.
    ///
    /// # Arguments
    ///
    /// * `character_index` — index into `party.members` whose book to open.
    /// * `previous_mode`   — game mode to restore when the screen closes.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::spell_book_state::SpellBookState;
    /// use antares::application::GameMode;
    ///
    /// let state = SpellBookState::new(2, GameMode::Exploration);
    /// assert_eq!(state.character_index, 2);
    /// assert!(state.selected_spell_id.is_none());
    /// assert_eq!(state.selected_row, 0);
    /// assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
    /// ```
    pub fn new(character_index: usize, previous_mode: GameMode) -> Self {
        Self {
            character_index,
            selected_spell_id: None,
            selected_row: 0,
            previous_mode: Box::new(previous_mode),
        }
    }

    /// Returns the `GameMode` to restore when the Spell Book screen closes.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::spell_book_state::SpellBookState;
    /// use antares::application::GameMode;
    ///
    /// let state = SpellBookState::new(0, GameMode::Exploration);
    /// assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
    /// ```
    pub fn get_resume_mode(&self) -> GameMode {
        (*self.previous_mode).clone()
    }

    /// Moves the character-tab cursor to the next party member, wrapping at
    /// `party_size`.
    ///
    /// Resets `selected_row` and `selected_spell_id` so the spell list starts
    /// fresh for the new character.
    ///
    /// Does nothing if `party_size` is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::spell_book_state::SpellBookState;
    /// use antares::application::GameMode;
    ///
    /// let mut state = SpellBookState::new(0, GameMode::Exploration);
    /// state.next_character(3);
    /// assert_eq!(state.character_index, 1);
    ///
    /// state.character_index = 2;
    /// state.next_character(3);
    /// assert_eq!(state.character_index, 0); // wraps
    /// ```
    pub fn next_character(&mut self, party_size: usize) {
        if party_size == 0 {
            return;
        }
        self.character_index = (self.character_index + 1) % party_size;
        self.selected_row = 0;
        self.selected_spell_id = None;
    }

    /// Moves the character-tab cursor to the previous party member, wrapping
    /// at `party_size`.
    ///
    /// Resets `selected_row` and `selected_spell_id` so the spell list starts
    /// fresh for the new character.
    ///
    /// Does nothing if `party_size` is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::spell_book_state::SpellBookState;
    /// use antares::application::GameMode;
    ///
    /// let mut state = SpellBookState::new(0, GameMode::Exploration);
    /// state.prev_character(3);
    /// assert_eq!(state.character_index, 2); // wraps to end
    ///
    /// state.prev_character(3);
    /// assert_eq!(state.character_index, 1);
    /// ```
    pub fn prev_character(&mut self, party_size: usize) {
        if party_size == 0 {
            return;
        }
        if self.character_index == 0 {
            self.character_index = party_size - 1;
        } else {
            self.character_index -= 1;
        }
        self.selected_row = 0;
        self.selected_spell_id = None;
    }

    /// Moves the spell-row cursor up by one, wrapping at the list top.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::spell_book_state::SpellBookState;
    /// use antares::application::GameMode;
    ///
    /// let mut state = SpellBookState::new(0, GameMode::Exploration);
    /// state.selected_row = 2;
    /// state.cursor_up(5);
    /// assert_eq!(state.selected_row, 1);
    ///
    /// state.selected_row = 0;
    /// state.cursor_up(5);
    /// assert_eq!(state.selected_row, 4); // wraps
    /// ```
    pub fn cursor_up(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        if self.selected_row == 0 {
            self.selected_row = item_count - 1;
        } else {
            self.selected_row -= 1;
        }
    }

    /// Moves the spell-row cursor down by one, wrapping at the list bottom.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::spell_book_state::SpellBookState;
    /// use antares::application::GameMode;
    ///
    /// let mut state = SpellBookState::new(0, GameMode::Exploration);
    /// state.cursor_down(3);
    /// assert_eq!(state.selected_row, 1);
    ///
    /// state.selected_row = 2;
    /// state.cursor_down(3);
    /// assert_eq!(state.selected_row, 0); // wraps
    /// ```
    pub fn cursor_down(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        self.selected_row = (self.selected_row + 1) % item_count;
    }
}

impl Default for SpellBookState {
    fn default() -> Self {
        Self::new(0, GameMode::Exploration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SpellBookState::new ──────────────────────────────────────────────────

    #[test]
    fn test_new_sets_character_index() {
        let state = SpellBookState::new(2, GameMode::Exploration);
        assert_eq!(state.character_index, 2);
    }

    #[test]
    fn test_new_captures_previous_mode() {
        let state = SpellBookState::new(0, GameMode::Exploration);
        assert!(matches!(*state.previous_mode, GameMode::Exploration));
    }

    #[test]
    fn test_new_selected_spell_id_is_none() {
        let state = SpellBookState::new(0, GameMode::Exploration);
        assert!(state.selected_spell_id.is_none());
    }

    #[test]
    fn test_new_selected_row_is_zero() {
        let state = SpellBookState::new(0, GameMode::Exploration);
        assert_eq!(state.selected_row, 0);
    }

    // ── SpellBookState::get_resume_mode ─────────────────────────────────────

    #[test]
    fn test_get_resume_mode_returns_exploration() {
        let state = SpellBookState::new(0, GameMode::Exploration);
        assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
    }

    #[test]
    fn test_get_resume_mode_returns_automap() {
        let state = SpellBookState::new(0, GameMode::Automap);
        assert!(matches!(state.get_resume_mode(), GameMode::Automap));
    }

    #[test]
    fn test_get_resume_mode_clone_is_independent() {
        let state = SpellBookState::new(1, GameMode::Exploration);
        let mode1 = state.get_resume_mode();
        let mode2 = state.get_resume_mode();
        assert_eq!(mode1, mode2);
    }

    // ── SpellBookState::next_character ───────────────────────────────────────

    #[test]
    fn test_next_character_increments_index() {
        let mut state = SpellBookState::new(0, GameMode::Exploration);
        state.next_character(4);
        assert_eq!(state.character_index, 1);
    }

    #[test]
    fn test_next_character_wraps_at_party_size() {
        let mut state = SpellBookState::new(2, GameMode::Exploration);
        state.next_character(3);
        assert_eq!(state.character_index, 0);
    }

    #[test]
    fn test_next_character_resets_row_and_selection() {
        let mut state = SpellBookState::new(0, GameMode::Exploration);
        state.selected_row = 5;
        state.selected_spell_id = Some(0x0101);
        state.next_character(3);
        assert_eq!(state.selected_row, 0);
        assert!(state.selected_spell_id.is_none());
    }

    #[test]
    fn test_next_character_noop_on_empty_party() {
        let mut state = SpellBookState::new(0, GameMode::Exploration);
        state.next_character(0);
        assert_eq!(state.character_index, 0);
    }

    // ── SpellBookState::prev_character ───────────────────────────────────────

    #[test]
    fn test_prev_character_decrements_index() {
        let mut state = SpellBookState::new(2, GameMode::Exploration);
        state.prev_character(4);
        assert_eq!(state.character_index, 1);
    }

    #[test]
    fn test_prev_character_wraps_to_end_at_zero() {
        let mut state = SpellBookState::new(0, GameMode::Exploration);
        state.prev_character(3);
        assert_eq!(state.character_index, 2);
    }

    #[test]
    fn test_prev_character_resets_row_and_selection() {
        let mut state = SpellBookState::new(1, GameMode::Exploration);
        state.selected_row = 3;
        state.selected_spell_id = Some(0x0201);
        state.prev_character(3);
        assert_eq!(state.selected_row, 0);
        assert!(state.selected_spell_id.is_none());
    }

    #[test]
    fn test_prev_character_noop_on_empty_party() {
        let mut state = SpellBookState::new(0, GameMode::Exploration);
        state.prev_character(0);
        assert_eq!(state.character_index, 0);
    }

    // ── SpellBookState::cursor_up ────────────────────────────────────────────

    #[test]
    fn test_cursor_up_decrements_row() {
        let mut state = SpellBookState::new(0, GameMode::Exploration);
        state.selected_row = 3;
        state.cursor_up(5);
        assert_eq!(state.selected_row, 2);
    }

    #[test]
    fn test_cursor_up_wraps_at_zero() {
        let mut state = SpellBookState::new(0, GameMode::Exploration);
        state.selected_row = 0;
        state.cursor_up(4);
        assert_eq!(state.selected_row, 3);
    }

    #[test]
    fn test_cursor_up_noop_on_empty_list() {
        let mut state = SpellBookState::new(0, GameMode::Exploration);
        state.cursor_up(0);
        assert_eq!(state.selected_row, 0);
    }

    // ── SpellBookState::cursor_down ──────────────────────────────────────────

    #[test]
    fn test_cursor_down_increments_row() {
        let mut state = SpellBookState::new(0, GameMode::Exploration);
        state.cursor_down(5);
        assert_eq!(state.selected_row, 1);
    }

    #[test]
    fn test_cursor_down_wraps_at_end() {
        let mut state = SpellBookState::new(0, GameMode::Exploration);
        state.selected_row = 4;
        state.cursor_down(5);
        assert_eq!(state.selected_row, 0);
    }

    #[test]
    fn test_cursor_down_noop_on_empty_list() {
        let mut state = SpellBookState::new(0, GameMode::Exploration);
        state.cursor_down(0);
        assert_eq!(state.selected_row, 0);
    }

    // ── Default ─────────────────────────────────────────────────────────────

    #[test]
    fn test_default_matches_new_zero_exploration() {
        let d = SpellBookState::default();
        assert_eq!(d.character_index, 0);
        assert!(d.selected_spell_id.is_none());
        assert_eq!(d.selected_row, 0);
        assert!(matches!(d.get_resume_mode(), GameMode::Exploration));
    }
}
