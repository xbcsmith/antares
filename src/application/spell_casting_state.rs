// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Spell casting state for exploration mode.
//!
//! Tracks the multi-step UI flow when a player casts a spell outside of combat:
//!
//! 1. **`SelectCaster`** — player selects which party member will cast.
//! 2. **`SelectSpell`**  — player selects a spell from the caster's book.
//! 3. **`SelectTarget`** — for `SingleCharacter` spells: player selects a target
//!    party member.  Skipped for `Self_` and `AllCharacters` spells.
//! 4. **`ShowResult`**   — the result message is displayed until dismissed.
//!
//! `SpellCastingState` is stored inside [`crate::application::GameMode::SpellCasting`].
//! It uses a `Box<GameMode>` for the previous mode (same pattern as
//! [`crate::application::inventory_state::InventoryState`]) to break the
//! recursive type dependency.
//!
//! # Examples
//!
//! ```
//! use antares::application::spell_casting_state::{SpellCastingState, SpellCastingStep};
//! use antares::application::GameMode;
//!
//! let state = SpellCastingState::new(GameMode::Exploration, 0);
//! assert_eq!(state.step, SpellCastingStep::SelectSpell);
//! assert_eq!(state.caster_index, 0);
//! assert!(state.selected_spell_id.is_none());
//! assert!(state.target_index.is_none());
//! assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
//! ```

use crate::application::GameMode;
use crate::domain::types::SpellId;
use serde::{Deserialize, Serialize};

/// Steps in the exploration spell-casting flow.
///
/// The UI advances through these steps in order (skipping `SelectTarget` when
/// the spell doesn't need a specific party member target).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellCastingStep {
    /// Player is selecting which party member should cast the spell.
    ///
    /// This step is used when the spell menu is opened without a pre-selected
    /// caster (e.g., the C key opens a full-party selector first).
    SelectCaster,
    /// Player is browsing and selecting a spell from the caster's spell book.
    SelectSpell,
    /// Player is selecting a target party member for a `SingleCharacter` spell.
    SelectTarget,
    /// The cast has completed and the result message is being shown.
    ShowResult,
}

/// Multi-step spell-casting UI flow state for exploration mode.
///
/// Stored inside [`GameMode::SpellCasting`].  The `previous_mode` field is
/// boxed to break the recursive size dependency between `SpellCastingState`
/// and `GameMode::SpellCasting(SpellCastingState)`.
///
/// # Examples
///
/// ```
/// use antares::application::spell_casting_state::{SpellCastingState, SpellCastingStep};
/// use antares::application::GameMode;
///
/// // Open the spell menu with party member 0 pre-selected as caster.
/// let mut state = SpellCastingState::new(GameMode::Exploration, 0);
/// assert_eq!(state.step, SpellCastingStep::SelectSpell);
///
/// // Select a spell.
/// state.select_spell(0x0101);
/// assert_eq!(state.selected_spell_id, Some(0x0101));
///
/// // Move to target selection.
/// state.step = SpellCastingStep::SelectTarget;
/// state.select_target(1);
/// assert_eq!(state.target_index, Some(1));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpellCastingState {
    /// Which UI step is currently active.
    pub step: SpellCastingStep,

    /// Index of the party member who is casting.
    ///
    /// Set during construction or during `SelectCaster`.
    pub caster_index: usize,

    /// The spell chosen by the player (set when leaving `SelectSpell`).
    pub selected_spell_id: Option<SpellId>,

    /// Party member index of the spell target (set when leaving `SelectTarget`).
    ///
    /// `None` for `Self_` and `AllCharacters` spells.
    pub target_index: Option<usize>,

    /// Selected row in the current step's list (used for keyboard navigation).
    pub selected_row: usize,

    /// Result message to display in `ShowResult` step.
    pub feedback_message: Option<String>,

    /// The `GameMode` that was active before the spell menu was opened.
    ///
    /// Restored when the player closes or completes the cast flow.
    /// Boxed to break the recursive size dependency.
    pub previous_mode: Box<GameMode>,
}

impl SpellCastingState {
    /// Creates a new `SpellCastingState` with the given previous mode and
    /// pre-selected caster.
    ///
    /// Starts at [`SpellCastingStep::SelectSpell`] because the caller already
    /// knows which party member should cast (typically triggered by pressing C
    /// with the HUD character card in focus, or via a party-level shortcut that
    /// picks the first available caster).
    ///
    /// Use [`SpellCastingState::new_with_caster_select`] when the player should
    /// choose the caster first.
    ///
    /// # Arguments
    ///
    /// * `previous_mode` — game mode to restore on cancel or completion.
    /// * `caster_index`  — index into `party.members` for the casting character.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::spell_casting_state::{SpellCastingState, SpellCastingStep};
    /// use antares::application::GameMode;
    ///
    /// let state = SpellCastingState::new(GameMode::Exploration, 2);
    /// assert_eq!(state.caster_index, 2);
    /// assert_eq!(state.step, SpellCastingStep::SelectSpell);
    /// ```
    pub fn new(previous_mode: GameMode, caster_index: usize) -> Self {
        Self {
            step: SpellCastingStep::SelectSpell,
            caster_index,
            selected_spell_id: None,
            target_index: None,
            selected_row: 0,
            feedback_message: None,
            previous_mode: Box::new(previous_mode),
        }
    }

    /// Creates a `SpellCastingState` that starts at the caster-selection step.
    ///
    /// Used when the player opens the spell menu without a pre-selected caster
    /// (e.g., pressing C in the middle of the screen without a focused card).
    ///
    /// # Arguments
    ///
    /// * `previous_mode` — game mode to restore on cancel or completion.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::spell_casting_state::{SpellCastingState, SpellCastingStep};
    /// use antares::application::GameMode;
    ///
    /// let state = SpellCastingState::new_with_caster_select(GameMode::Exploration);
    /// assert_eq!(state.step, SpellCastingStep::SelectCaster);
    /// assert_eq!(state.caster_index, 0);
    /// ```
    pub fn new_with_caster_select(previous_mode: GameMode) -> Self {
        Self {
            step: SpellCastingStep::SelectCaster,
            caster_index: 0,
            selected_spell_id: None,
            target_index: None,
            selected_row: 0,
            feedback_message: None,
            previous_mode: Box::new(previous_mode),
        }
    }

    /// Returns the `GameMode` to restore when the spell menu closes.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::spell_casting_state::SpellCastingState;
    /// use antares::application::GameMode;
    ///
    /// let state = SpellCastingState::new(GameMode::Exploration, 0);
    /// assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
    /// ```
    pub fn get_resume_mode(&self) -> GameMode {
        (*self.previous_mode).clone()
    }

    /// Records the selected spell ID and advances to the next step.
    ///
    /// If the spell requires a `SingleCharacter` target the next step is
    /// `SelectTarget`; otherwise the caller should immediately execute the cast
    /// and advance to `ShowResult`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::spell_casting_state::SpellCastingState;
    /// use antares::application::GameMode;
    ///
    /// let mut state = SpellCastingState::new(GameMode::Exploration, 0);
    /// state.select_spell(0x0101);
    /// assert_eq!(state.selected_spell_id, Some(0x0101));
    /// ```
    pub fn select_spell(&mut self, spell_id: SpellId) {
        self.selected_spell_id = Some(spell_id);
        self.selected_row = 0;
    }

    /// Records the selected target party-member index.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::spell_casting_state::SpellCastingState;
    /// use antares::application::GameMode;
    ///
    /// let mut state = SpellCastingState::new(GameMode::Exploration, 0);
    /// state.select_target(2);
    /// assert_eq!(state.target_index, Some(2));
    /// ```
    pub fn select_target(&mut self, party_index: usize) {
        self.target_index = Some(party_index);
    }

    /// Sets the feedback message and advances the step to `ShowResult`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::spell_casting_state::{SpellCastingState, SpellCastingStep};
    /// use antares::application::GameMode;
    ///
    /// let mut state = SpellCastingState::new(GameMode::Exploration, 0);
    /// state.show_result("First Aid restores 7 HP.".to_string());
    /// assert_eq!(state.step, SpellCastingStep::ShowResult);
    /// assert_eq!(
    ///     state.feedback_message.as_deref(),
    ///     Some("First Aid restores 7 HP."),
    /// );
    /// ```
    pub fn show_result(&mut self, message: String) {
        self.feedback_message = Some(message);
        self.step = SpellCastingStep::ShowResult;
    }

    /// Moves the cursor up in the current list (with wrapping).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::spell_casting_state::SpellCastingState;
    /// use antares::application::GameMode;
    ///
    /// let mut state = SpellCastingState::new(GameMode::Exploration, 0);
    /// state.selected_row = 2;
    /// state.cursor_up(5);
    /// assert_eq!(state.selected_row, 1);
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

    /// Moves the cursor down in the current list (with wrapping).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::spell_casting_state::SpellCastingState;
    /// use antares::application::GameMode;
    ///
    /// let mut state = SpellCastingState::new(GameMode::Exploration, 0);
    /// state.cursor_down(3);
    /// assert_eq!(state.selected_row, 1);
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

impl Default for SpellCastingState {
    fn default() -> Self {
        Self::new(GameMode::Exploration, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_starts_at_select_spell() {
        let state = SpellCastingState::new(GameMode::Exploration, 1);
        assert_eq!(state.step, SpellCastingStep::SelectSpell);
        assert_eq!(state.caster_index, 1);
        assert!(state.selected_spell_id.is_none());
        assert!(state.target_index.is_none());
        assert_eq!(state.selected_row, 0);
    }

    #[test]
    fn test_new_with_caster_select_starts_at_select_caster() {
        let state = SpellCastingState::new_with_caster_select(GameMode::Exploration);
        assert_eq!(state.step, SpellCastingStep::SelectCaster);
        assert_eq!(state.caster_index, 0);
    }

    #[test]
    fn test_get_resume_mode_returns_previous() {
        let state = SpellCastingState::new(GameMode::Exploration, 0);
        assert!(matches!(state.get_resume_mode(), GameMode::Exploration));
    }

    #[test]
    fn test_select_spell_stores_id() {
        let mut state = SpellCastingState::new(GameMode::Exploration, 0);
        state.select_spell(0x0101);
        assert_eq!(state.selected_spell_id, Some(0x0101));
    }

    #[test]
    fn test_select_target_stores_index() {
        let mut state = SpellCastingState::new(GameMode::Exploration, 0);
        state.select_target(3);
        assert_eq!(state.target_index, Some(3));
    }

    #[test]
    fn test_show_result_updates_step_and_message() {
        let mut state = SpellCastingState::new(GameMode::Exploration, 0);
        state.show_result("Healed 12 HP.".to_string());
        assert_eq!(state.step, SpellCastingStep::ShowResult);
        assert_eq!(state.feedback_message.as_deref(), Some("Healed 12 HP."));
    }

    #[test]
    fn test_cursor_up_decrements() {
        let mut state = SpellCastingState::new(GameMode::Exploration, 0);
        state.selected_row = 3;
        state.cursor_up(5);
        assert_eq!(state.selected_row, 2);
    }

    #[test]
    fn test_cursor_up_wraps_at_zero() {
        let mut state = SpellCastingState::new(GameMode::Exploration, 0);
        state.selected_row = 0;
        state.cursor_up(4);
        assert_eq!(state.selected_row, 3);
    }

    #[test]
    fn test_cursor_up_noop_on_empty() {
        let mut state = SpellCastingState::new(GameMode::Exploration, 0);
        state.cursor_up(0);
        assert_eq!(state.selected_row, 0);
    }

    #[test]
    fn test_cursor_down_increments() {
        let mut state = SpellCastingState::new(GameMode::Exploration, 0);
        state.cursor_down(5);
        assert_eq!(state.selected_row, 1);
    }

    #[test]
    fn test_cursor_down_wraps() {
        let mut state = SpellCastingState::new(GameMode::Exploration, 0);
        state.selected_row = 4;
        state.cursor_down(5);
        assert_eq!(state.selected_row, 0);
    }

    #[test]
    fn test_cursor_down_noop_on_empty() {
        let mut state = SpellCastingState::new(GameMode::Exploration, 0);
        state.cursor_down(0);
        assert_eq!(state.selected_row, 0);
    }

    #[test]
    fn test_default_matches_new_exploration() {
        let d = SpellCastingState::default();
        assert_eq!(d.step, SpellCastingStep::SelectSpell);
        assert_eq!(d.caster_index, 0);
    }
}
