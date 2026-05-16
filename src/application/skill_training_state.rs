// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Skill training screen state.
//!
//! Tracks which party members are eligible for skill training, which skills
//! are offered by the current NPC, the player's current selections, and any
//! status message to display in the UI.
//!
//! `SkillTrainingState` is created when the player interacts with an NPC that
//! has `is_skill_trainer == true` and is destroyed when the player closes the
//! training screen.
//!
//! # Examples
//!
//! ```
//! use antares::application::skill_training_state::SkillTrainingState;
//!
//! let state = SkillTrainingState::new(
//!     "master_willow",
//!     vec![0, 2],
//!     vec!["perception".to_string(), "disarm_traps".to_string()],
//! );
//!
//! assert_eq!(state.npc_id, "master_willow");
//! assert_eq!(state.eligible_member_indices, vec![0, 2]);
//! assert_eq!(state.available_skill_ids.len(), 2);
//! assert!(state.selected_member_index.is_none());
//! assert!(state.selected_skill_index.is_none());
//! assert!(state.status_message.is_none());
//! ```

use crate::domain::skills::SkillId;
use serde::{Deserialize, Serialize};

/// State for the skill training screen.
///
/// Tracks the NPC offering training, which party members are eligible,
/// which skills are available, the player's current selections, and
/// any status or error message to display in the UI.
///
/// # Examples
///
/// ```
/// use antares::application::skill_training_state::SkillTrainingState;
///
/// let state = SkillTrainingState::new(
///     "master_willow",
///     vec![0, 1],
///     vec!["perception".to_string()],
/// );
///
/// assert_eq!(state.npc_id, "master_willow");
/// assert!(state.selected_member_index.is_none());
/// assert!(state.selected_skill_index.is_none());
/// assert!(state.status_message.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillTrainingState {
    /// The NPC ID of the skill trainer conducting the session
    /// (e.g., `"master_willow"`, `"perception_sage"`).
    pub npc_id: String,

    /// Indices into `party.members` for members eligible for skill training.
    ///
    /// A party member is eligible when they are alive. Dead members are excluded.
    pub eligible_member_indices: Vec<usize>,

    /// Skill IDs offered by this NPC (`is_skill_trainer == true` NPC's
    /// `trainable_skill_ids`).
    ///
    /// Empty when the NPC offers no skills (misconfigured NPC).
    pub available_skill_ids: Vec<SkillId>,

    /// Currently selected party-member index in `eligible_member_indices`.
    ///
    /// `None` when no party member is selected yet.
    pub selected_member_index: Option<usize>,

    /// Currently selected skill index in `available_skill_ids`.
    ///
    /// `None` when no skill is selected yet.
    pub selected_skill_index: Option<usize>,

    /// Last status or error message to display in the UI.
    ///
    /// `None` when idle; set after a training attempt (success or failure).
    pub status_message: Option<String>,
}

impl SkillTrainingState {
    /// Creates a new `SkillTrainingState` for the given NPC.
    ///
    /// Starts with no selections and no status message.
    ///
    /// # Arguments
    ///
    /// * `npc_id` - The ID of the skill trainer NPC (e.g., `"master_willow"`).
    /// * `eligible_member_indices` - Indices into `party.members` for alive members.
    /// * `available_skill_ids` - The `SkillId`s this NPC can train.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::skill_training_state::SkillTrainingState;
    ///
    /// let state = SkillTrainingState::new(
    ///     "master_willow",
    ///     vec![0, 1, 2],
    ///     vec!["perception".to_string(), "disarm_traps".to_string()],
    /// );
    ///
    /// assert_eq!(state.npc_id, "master_willow");
    /// assert_eq!(state.eligible_member_indices, vec![0, 1, 2]);
    /// assert_eq!(state.available_skill_ids.len(), 2);
    /// assert!(state.selected_member_index.is_none());
    /// assert!(state.selected_skill_index.is_none());
    /// assert!(state.status_message.is_none());
    /// ```
    pub fn new(
        npc_id: impl Into<String>,
        eligible_member_indices: Vec<usize>,
        available_skill_ids: Vec<SkillId>,
    ) -> Self {
        Self {
            npc_id: npc_id.into(),
            eligible_member_indices,
            available_skill_ids,
            selected_member_index: None,
            selected_skill_index: None,
            status_message: None,
        }
    }

    /// Clears the current selections and status message.
    ///
    /// Resets `selected_member_index`, `selected_skill_index`, and
    /// `status_message` to `None`. Does not change `npc_id`,
    /// `eligible_member_indices`, or `available_skill_ids`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::skill_training_state::SkillTrainingState;
    ///
    /// let mut state = SkillTrainingState::new(
    ///     "trainer",
    ///     vec![0],
    ///     vec!["perception".to_string()],
    /// );
    /// state.selected_member_index = Some(0);
    /// state.selected_skill_index = Some(0);
    /// state.status_message = Some("Training complete!".to_string());
    ///
    /// state.clear();
    ///
    /// assert!(state.selected_member_index.is_none());
    /// assert!(state.selected_skill_index.is_none());
    /// assert!(state.status_message.is_none());
    /// ```
    pub fn clear(&mut self) {
        self.selected_member_index = None;
        self.selected_skill_index = None;
        self.status_message = None;
    }

    /// Sets `selected_member_index` to `Some(index)`.
    ///
    /// The `index` refers to a position within `eligible_member_indices`,
    /// not directly into `party.members`.
    ///
    /// # Arguments
    ///
    /// * `index` - Position in `eligible_member_indices` to select.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::skill_training_state::SkillTrainingState;
    ///
    /// let mut state = SkillTrainingState::new(
    ///     "trainer",
    ///     vec![0, 2, 4],
    ///     vec!["perception".to_string()],
    /// );
    ///
    /// state.select_member(1); // selects eligible_member_indices[1] = party index 2
    /// assert_eq!(state.selected_member_index, Some(1));
    /// ```
    pub fn select_member(&mut self, index: usize) {
        self.selected_member_index = Some(index);
    }

    /// Sets `selected_skill_index` to `Some(index)`.
    ///
    /// The `index` refers to a position within `available_skill_ids`.
    ///
    /// # Arguments
    ///
    /// * `index` - Position in `available_skill_ids` to select.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::skill_training_state::SkillTrainingState;
    ///
    /// let mut state = SkillTrainingState::new(
    ///     "trainer",
    ///     vec![0],
    ///     vec!["perception".to_string(), "disarm_traps".to_string()],
    /// );
    ///
    /// state.select_skill(1); // selects "disarm_traps"
    /// assert_eq!(state.selected_skill_index, Some(1));
    /// ```
    pub fn select_skill(&mut self, index: usize) {
        self.selected_skill_index = Some(index);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── SkillTrainingState::new ──────────────────────────────────────────────

    #[test]
    fn test_skill_training_state_new_stores_npc_id() {
        let state =
            SkillTrainingState::new("master_willow", vec![0, 1], vec!["perception".to_string()]);
        assert_eq!(state.npc_id, "master_willow");
    }

    #[test]
    fn test_skill_training_state_new_stores_eligible_indices() {
        let state =
            SkillTrainingState::new("trainer", vec![0, 2, 4], vec!["perception".to_string()]);
        assert_eq!(state.eligible_member_indices, vec![0, 2, 4]);
    }

    #[test]
    fn test_skill_training_state_new_stores_available_skills() {
        let state = SkillTrainingState::new(
            "trainer",
            vec![0],
            vec!["perception".to_string(), "disarm_traps".to_string()],
        );
        assert_eq!(
            state.available_skill_ids,
            vec!["perception".to_string(), "disarm_traps".to_string()]
        );
    }

    #[test]
    fn test_skill_training_state_new_has_no_selections() {
        let state = SkillTrainingState::new("trainer", vec![0], vec!["perception".to_string()]);
        assert!(state.selected_member_index.is_none());
        assert!(state.selected_skill_index.is_none());
        assert!(state.status_message.is_none());
    }

    #[test]
    fn test_skill_training_state_new_accepts_string_slice() {
        let state = SkillTrainingState::new("trainer_bob", vec![], vec!["diplomacy".to_string()]);
        assert_eq!(state.npc_id, "trainer_bob");
    }

    // ── clear ────────────────────────────────────────────────────────────────

    #[test]
    fn test_clear_resets_member_selection() {
        let mut state = SkillTrainingState::new("t", vec![0], vec!["perception".to_string()]);
        state.selected_member_index = Some(0);
        state.clear();
        assert!(state.selected_member_index.is_none());
    }

    #[test]
    fn test_clear_resets_skill_selection() {
        let mut state = SkillTrainingState::new("t", vec![0], vec!["perception".to_string()]);
        state.selected_skill_index = Some(0);
        state.clear();
        assert!(state.selected_skill_index.is_none());
    }

    #[test]
    fn test_clear_resets_status_message() {
        let mut state = SkillTrainingState::new("t", vec![0], vec!["perception".to_string()]);
        state.status_message = Some("Training complete!".to_string());
        state.clear();
        assert!(state.status_message.is_none());
    }

    #[test]
    fn test_clear_preserves_npc_id_and_lists() {
        let mut state = SkillTrainingState::new(
            "sage_of_perception",
            vec![0, 1],
            vec!["perception".to_string()],
        );
        state.selected_member_index = Some(1);
        state.selected_skill_index = Some(0);
        state.status_message = Some("msg".to_string());

        state.clear();

        assert_eq!(state.npc_id, "sage_of_perception");
        assert_eq!(state.eligible_member_indices, vec![0, 1]);
        assert_eq!(state.available_skill_ids, vec!["perception".to_string()]);
    }

    // ── select_member ────────────────────────────────────────────────────────

    #[test]
    fn test_select_member_sets_index() {
        let mut state = SkillTrainingState::new("t", vec![0, 2, 4], vec!["p".to_string()]);
        state.select_member(1);
        assert_eq!(state.selected_member_index, Some(1));
    }

    #[test]
    fn test_select_member_overwrites_previous_selection() {
        let mut state = SkillTrainingState::new("t", vec![0, 1, 2], vec!["p".to_string()]);
        state.select_member(0);
        state.select_member(2);
        assert_eq!(state.selected_member_index, Some(2));
    }

    // ── select_skill ─────────────────────────────────────────────────────────

    #[test]
    fn test_select_skill_sets_index() {
        let mut state = SkillTrainingState::new(
            "t",
            vec![0],
            vec!["perception".to_string(), "disarm_traps".to_string()],
        );
        state.select_skill(1);
        assert_eq!(state.selected_skill_index, Some(1));
    }

    #[test]
    fn test_select_skill_overwrites_previous_selection() {
        let mut state = SkillTrainingState::new(
            "t",
            vec![0],
            vec!["perception".to_string(), "disarm_traps".to_string()],
        );
        state.select_skill(0);
        state.select_skill(1);
        assert_eq!(state.selected_skill_index, Some(1));
    }

    // ── Default state assertions ─────────────────────────────────────────────

    #[test]
    fn test_skill_training_state_all_defaults() {
        let state = SkillTrainingState::new("npc", vec![], vec![]);
        assert_eq!(state.npc_id, "npc");
        assert!(state.eligible_member_indices.is_empty());
        assert!(state.available_skill_ids.is_empty());
        assert!(state.selected_member_index.is_none());
        assert!(state.selected_skill_index.is_none());
        assert!(state.status_message.is_none());
    }

    // ── Serde round-trip ─────────────────────────────────────────────────────

    #[test]
    fn test_skill_training_state_serde_roundtrip() {
        let mut state = SkillTrainingState::new(
            "perception_sage",
            vec![0, 2],
            vec!["perception".to_string(), "item_lore".to_string()],
        );
        state.selected_member_index = Some(0);
        state.selected_skill_index = Some(1);
        state.status_message = Some("Training complete!".to_string());

        let json = serde_json::to_string(&state).unwrap();
        let restored: SkillTrainingState = serde_json::from_str(&json).unwrap();

        assert_eq!(state, restored);
    }
}
