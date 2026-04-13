// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Character sheet screen state.
//!
//! Tracks which party member is focused, which view layout is active, and
//! which mode to restore when the player closes the screen.
//!
//! `CharacterSheetState` is stored inside
//! [`crate::application::GameMode::CharacterSheet`].  It uses a
//! `Box<GameMode>` for the previous mode — the same pattern as
//! [`crate::application::inventory_state::InventoryState`] and
//! [`crate::application::spell_book_state::SpellBookState`] — to break the
//! recursive size dependency.
//!
//! # Lifetime
//!
//! The state is created when the player presses the character sheet key
//! (default `P`) in Exploration mode and destroyed when the player presses
//! Esc (returning to the mode that was active before).
//!
//! # Examples
//!
//! ```
//! use antares::application::character_sheet_state::{CharacterSheetState, CharacterSheetView};
//! use antares::application::GameMode;
//!
//! let state = CharacterSheetState::new(GameMode::Exploration);
//! assert_eq!(state.focused_index, 0);
//! assert_eq!(state.view, CharacterSheetView::Single);
//! assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
//! ```

use crate::application::GameMode;
use serde::{Deserialize, Serialize};

// ── View mode ─────────────────────────────────────────────────────────────────

/// Display mode for the character sheet screen.
///
/// The screen supports two layouts that the player can toggle between using
/// the `O` key:
///
/// * [`Single`](CharacterSheetView::Single) — detailed stats panel for one
///   party member at a time, navigated with Tab / Shift-Tab or arrow keys.
/// * [`PartyOverview`](CharacterSheetView::PartyOverview) — compact summary
///   cards for every party member side-by-side in a horizontal scroll area.
///
/// # Examples
///
/// ```
/// use antares::application::character_sheet_state::CharacterSheetView;
///
/// let view = CharacterSheetView::default();
/// assert_eq!(view, CharacterSheetView::Single);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum CharacterSheetView {
    /// Detailed single-character panel.
    #[default]
    Single,
    /// Compact overview cards for every party member.
    PartyOverview,
}

// ── State ─────────────────────────────────────────────────────────────────────

/// State for the character sheet read-only viewer screen.
///
/// Tracks which party member's sheet is displayed, which view layout is
/// active, and the mode to restore when the screen is closed.
///
/// `previous_mode` is boxed to break the recursive size dependency between
/// `CharacterSheetState` and `GameMode::CharacterSheet(CharacterSheetState)`.
///
/// # Examples
///
/// ```
/// use antares::application::character_sheet_state::{CharacterSheetState, CharacterSheetView};
/// use antares::application::GameMode;
///
/// let state = CharacterSheetState::new(GameMode::Exploration);
/// assert_eq!(state.focused_index, 0);
/// assert_eq!(state.view, CharacterSheetView::Single);
/// assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSheetState {
    /// The `GameMode` active before the character sheet was opened.
    ///
    /// Restored via [`get_resume_mode`] when the player closes the screen.
    ///
    /// [`get_resume_mode`]: CharacterSheetState::get_resume_mode
    pub previous_mode: Box<GameMode>,

    /// Zero-based index of the party member whose sheet is currently shown.
    ///
    /// Valid range: `0..party_size`.  Wraps on [`focus_next`] / [`focus_prev`].
    ///
    /// [`focus_next`]: CharacterSheetState::focus_next
    /// [`focus_prev`]: CharacterSheetState::focus_prev
    pub focused_index: usize,

    /// Currently active display layout.
    ///
    /// Toggled between [`Single`](CharacterSheetView::Single) and
    /// [`PartyOverview`](CharacterSheetView::PartyOverview) by
    /// [`toggle_view`](CharacterSheetState::toggle_view).
    pub view: CharacterSheetView,
}

impl CharacterSheetState {
    /// Create a new `CharacterSheetState`, storing the mode that was active
    /// before opening the character sheet.
    ///
    /// The new state has `focused_index = 0` and
    /// `view = CharacterSheetView::Single`.
    ///
    /// # Arguments
    ///
    /// * `previous_mode` – The `GameMode` to restore when the screen closes.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::character_sheet_state::{CharacterSheetState, CharacterSheetView};
    /// use antares::application::GameMode;
    ///
    /// let state = CharacterSheetState::new(GameMode::Exploration);
    /// assert_eq!(state.focused_index, 0);
    /// assert_eq!(state.view, CharacterSheetView::Single);
    /// ```
    pub fn new(previous_mode: GameMode) -> Self {
        Self {
            previous_mode: Box::new(previous_mode),
            focused_index: 0,
            view: CharacterSheetView::Single,
        }
    }

    /// Return the `GameMode` to restore when closing the character sheet.
    ///
    /// Clones the stored `previous_mode` to match the pattern used by
    /// `InventoryState::get_resume_mode` and `SpellBookState::get_resume_mode`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::character_sheet_state::CharacterSheetState;
    /// use antares::application::GameMode;
    ///
    /// let state = CharacterSheetState::new(GameMode::Exploration);
    /// assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
    /// ```
    pub fn get_resume_mode(&self) -> GameMode {
        (*self.previous_mode).clone()
    }

    /// Advance focus to the next party member, wrapping around.
    ///
    /// This is a no-op when `party_size == 0`.
    ///
    /// # Arguments
    ///
    /// * `party_size` – Number of members currently in the party (`1..=PARTY_MAX_SIZE`).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::character_sheet_state::CharacterSheetState;
    /// use antares::application::GameMode;
    ///
    /// let mut state = CharacterSheetState::new(GameMode::Exploration);
    /// state.focus_next(3);
    /// assert_eq!(state.focused_index, 1);
    ///
    /// state.focused_index = 2;
    /// state.focus_next(3);
    /// assert_eq!(state.focused_index, 0); // wraps
    /// ```
    pub fn focus_next(&mut self, party_size: usize) {
        if party_size == 0 {
            return;
        }
        self.focused_index = (self.focused_index + 1) % party_size;
    }

    /// Move focus to the previous party member, wrapping around.
    ///
    /// This is a no-op when `party_size == 0`.
    ///
    /// # Arguments
    ///
    /// * `party_size` – Number of members currently in the party (`1..=PARTY_MAX_SIZE`).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::character_sheet_state::CharacterSheetState;
    /// use antares::application::GameMode;
    ///
    /// let mut state = CharacterSheetState::new(GameMode::Exploration);
    /// state.focus_prev(3); // wraps from 0 → 2
    /// assert_eq!(state.focused_index, 2);
    /// ```
    pub fn focus_prev(&mut self, party_size: usize) {
        if party_size == 0 {
            return;
        }
        if self.focused_index == 0 {
            self.focused_index = party_size - 1;
        } else {
            self.focused_index -= 1;
        }
    }

    /// Toggle between `Single` and `PartyOverview` view layouts.
    ///
    /// Flips `Single → PartyOverview` or `PartyOverview → Single` each call.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::character_sheet_state::{CharacterSheetState, CharacterSheetView};
    /// use antares::application::GameMode;
    ///
    /// let mut state = CharacterSheetState::new(GameMode::Exploration);
    /// assert_eq!(state.view, CharacterSheetView::Single);
    ///
    /// state.toggle_view();
    /// assert_eq!(state.view, CharacterSheetView::PartyOverview);
    ///
    /// state.toggle_view();
    /// assert_eq!(state.view, CharacterSheetView::Single);
    /// ```
    pub fn toggle_view(&mut self) {
        self.view = match self.view {
            CharacterSheetView::Single => CharacterSheetView::PartyOverview,
            CharacterSheetView::PartyOverview => CharacterSheetView::Single,
        };
    }
}

impl Default for CharacterSheetState {
    /// Create a default `CharacterSheetState` with `GameMode::Exploration` as
    /// the previous mode.
    fn default() -> Self {
        Self::new(GameMode::Exploration)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameMode;

    // ── CharacterSheetState::new ─────────────────────────────────────────────

    #[test]
    fn test_character_sheet_state_new() {
        let state = CharacterSheetState::new(GameMode::Exploration);
        assert_eq!(state.focused_index, 0);
        assert_eq!(state.view, CharacterSheetView::Single);
        assert!(matches!(*state.previous_mode, GameMode::Exploration));
    }

    // ── get_resume_mode ──────────────────────────────────────────────────────

    #[test]
    fn test_get_resume_mode() {
        let state = CharacterSheetState::new(GameMode::Exploration);
        assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
    }

    #[test]
    fn test_get_resume_mode_clone_is_independent() {
        let state = CharacterSheetState::new(GameMode::Automap);
        let mode1 = state.get_resume_mode();
        let mode2 = state.get_resume_mode();
        assert_eq!(mode1, mode2);
    }

    // ── focus_next ───────────────────────────────────────────────────────────

    #[test]
    fn test_focus_next_increments_index() {
        let mut state = CharacterSheetState::new(GameMode::Exploration);
        state.focus_next(4);
        assert_eq!(state.focused_index, 1);
    }

    #[test]
    fn test_focus_next_wraps() {
        let mut state = CharacterSheetState::new(GameMode::Exploration);
        state.focused_index = 2;
        state.focus_next(3);
        assert_eq!(state.focused_index, 0);
    }

    #[test]
    fn test_focus_next_noop_on_empty() {
        let mut state = CharacterSheetState::new(GameMode::Exploration);
        state.focused_index = 0;
        state.focus_next(0);
        assert_eq!(state.focused_index, 0);
    }

    // ── focus_prev ───────────────────────────────────────────────────────────

    #[test]
    fn test_focus_prev_decrements_index() {
        let mut state = CharacterSheetState::new(GameMode::Exploration);
        state.focused_index = 2;
        state.focus_prev(4);
        assert_eq!(state.focused_index, 1);
    }

    #[test]
    fn test_focus_prev_wraps() {
        let mut state = CharacterSheetState::new(GameMode::Exploration);
        state.focused_index = 0;
        state.focus_prev(3);
        assert_eq!(state.focused_index, 2);
    }

    #[test]
    fn test_focus_prev_noop_on_empty() {
        let mut state = CharacterSheetState::new(GameMode::Exploration);
        state.focused_index = 0;
        state.focus_prev(0);
        assert_eq!(state.focused_index, 0);
    }

    // ── toggle_view ──────────────────────────────────────────────────────────

    #[test]
    fn test_toggle_view() {
        let mut state = CharacterSheetState::new(GameMode::Exploration);
        assert_eq!(state.view, CharacterSheetView::Single);

        state.toggle_view();
        assert_eq!(state.view, CharacterSheetView::PartyOverview);

        state.toggle_view();
        assert_eq!(state.view, CharacterSheetView::Single);
    }

    #[test]
    fn test_toggle_view_from_party_overview_back_to_single() {
        let mut state = CharacterSheetState::new(GameMode::Exploration);
        state.view = CharacterSheetView::PartyOverview;
        state.toggle_view();
        assert_eq!(state.view, CharacterSheetView::Single);
    }

    // ── Default ──────────────────────────────────────────────────────────────

    #[test]
    fn test_default_matches_new_exploration() {
        let default_state = CharacterSheetState::default();
        let new_state = CharacterSheetState::new(GameMode::Exploration);
        assert_eq!(default_state.focused_index, new_state.focused_index);
        assert_eq!(default_state.view, new_state.view);
        assert_eq!(default_state.get_resume_mode(), new_state.get_resume_mode());
    }
}
