// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Dialogue state for the application layer
//!
//! This module implements `DialogueState`, a compact structure used by the
//! application to track the currently active dialogue tree, the current node
//! within that tree, and a small history of visited nodes.
//!
//! The state is intentionally lightweight so it can be stored directly in
//! `GameState::mode` (as `GameMode::Dialogue(DialogueState)`) or installed as a
//! Bevy resource where needed.
//!
//! # Examples
//!
//! ```no_run
//! use antares::application::dialogue::DialogueState;
//! use antares::domain::dialogue::{DialogueId, NodeId};
//!
//! // Start a dialogue tree (ID 1) at root node (ID 1)
//! let state = DialogueState::start(1 as DialogueId, 1 as NodeId);
//! assert!(state.is_active());
//! assert_eq!(state.current_node_id, 1);
//! ```

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use crate::domain::dialogue::{DialogueId, NodeId};

/// Context information for character recruitment dialogues
///
/// Stores metadata needed to complete recruitment after dialogue concludes.
/// This includes the character being recruited and the map event position
/// for cleanup after successful recruitment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecruitmentContext {
    /// ID of the character definition being recruited
    pub character_id: String,

    /// Position of the recruitment event on the map (for removal after recruitment)
    pub event_position: crate::domain::types::Position,
}

/// Tracks the currently active dialogue tree and progress through it.
///
/// This structure is small and serializable so it can be persisted inside the
/// main `GameState` or used as a Bevy `Resource` for systems that need access
/// to dialogue state.
///
/// # Fields
///
/// * `active_tree_id` - Optional active dialogue tree identifier.
/// * `current_node_id` - Node ID currently being displayed/processed.
/// * `dialogue_history` - Ordered list of node IDs visited (root .. current).
/// * `current_text` - Current node's full text content for visual systems.
/// * `current_speaker` - Current node's speaker name.
/// * `current_choices` - Current node's available choices.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Resource)]
pub struct DialogueState {
    /// The active dialogue tree ID (None if no dialogue is active)
    pub active_tree_id: Option<DialogueId>,

    /// The currently-active node within the tree
    pub current_node_id: NodeId,

    /// History of visited nodes (first = root, last = current)
    pub dialogue_history: Vec<NodeId>,

    /// Current node's full text (for visual systems)
    pub current_text: String,

    /// Current node's speaker name
    pub current_speaker: String,

    /// Current node's available choices
    pub current_choices: Vec<String>,

    /// Speaker entity that initiated this dialogue (typically an NPC)
    pub speaker_entity: Option<bevy::prelude::Entity>,

    /// NPC ID of the speaker (if the speaker corresponds to an NPC in the content DB)
    pub speaker_npc_id: Option<String>,

    /// Context for recruitment dialogues (None if not a recruitment interaction)
    pub recruitment_context: Option<RecruitmentContext>,

    /// Fallback map position for visual placement if speaker_entity is missing
    pub fallback_position: Option<crate::domain::types::Position>,
}

impl DialogueState {
    /// Create a new inactive `DialogueState`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::dialogue::DialogueState;
    ///
    /// let state = DialogueState::new();
    /// assert!(!state.is_active());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a dialogue tree at `root_node`.
    ///
    /// This returns a fresh `DialogueState` with `active_tree_id` set and the
    /// root node recorded as the current node and as the first history entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::dialogue::DialogueState;
    /// use antares::domain::dialogue::{DialogueId, NodeId};
    ///
    /// let state = DialogueState::start(5 as DialogueId, 1 as NodeId, None, None);
    /// assert_eq!(state.active_tree_id, Some(5));
    /// assert_eq!(state.current_node_id, 1);
    /// assert_eq!(state.dialogue_history, vec![1]);
    /// ```
    pub fn start(
        tree_id: DialogueId,
        root_node: NodeId,
        fallback_pos: Option<crate::domain::types::Position>,
        speaker_npc_id: Option<String>,
    ) -> Self {
        Self {
            active_tree_id: Some(tree_id),
            current_node_id: root_node,
            dialogue_history: vec![root_node],
            current_text: String::new(),
            current_speaker: String::new(),
            current_choices: Vec::new(),
            speaker_entity: None,
            speaker_npc_id,
            recruitment_context: None,
            fallback_position: fallback_pos,
        }
    }

    /// Start a simple dialogue without a tree ID.
    pub fn start_simple(
        text: String,
        speaker_name: String,
        speaker_entity: Option<bevy::prelude::Entity>,
        fallback_pos: Option<crate::domain::types::Position>,
    ) -> Self {
        Self {
            active_tree_id: None,
            current_node_id: 0,
            dialogue_history: Vec::new(),
            current_text: text,
            current_speaker: speaker_name,
            current_choices: vec!["Goodbye".to_string()],
            speaker_entity,
            speaker_npc_id: None,
            recruitment_context: None,
            fallback_position: fallback_pos,
        }
    }

    /// Advance the current dialogue to `next_node`, recording the visit in the
    /// dialogue history.
    ///
    /// Calling `advance_to` does not perform any condition checks - it only
    /// updates state. Validations (conditions, scripted actions) belong to the
    /// dialogue runtime systems that operate on this state.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::dialogue::DialogueState;
    /// use antares::domain::dialogue::NodeId;
    ///
    /// let mut state = DialogueState::start(1, 1);
    /// state.advance_to(2);
    /// assert_eq!(state.current_node_id, 2);
    /// assert_eq!(state.dialogue_history, vec![1, 2]);
    /// ```
    pub fn advance_to(&mut self, next_node: NodeId) {
        self.current_node_id = next_node;
        self.dialogue_history.push(next_node);
    }

    /// Updates dialogue state with new node information
    ///
    /// Called when dialogue advances to a new node to update visual state.
    ///
    /// # Arguments
    ///
    /// * `text` - The new node's text content
    /// * `speaker` - The speaker's name for this node
    /// * `choices` - Available player choices (empty for terminal nodes)
    /// * `speaker_entity` - Optional entity that initiated this dialogue (typically an NPC)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::dialogue::DialogueState;
    /// use bevy::prelude::Entity;
    ///
    /// let mut state = DialogueState::new();
    /// state.update_node(
    ///     "Hello, adventurer!".to_string(),
    ///     "Village Elder".to_string(),
    ///     vec!["Greetings".to_string(), "Farewell".to_string()],
    ///     None,
    /// );
    ///
    /// assert_eq!(state.current_text, "Hello, adventurer!");
    /// assert_eq!(state.current_speaker, "Village Elder");
    /// assert_eq!(state.current_choices.len(), 2);
    /// ```
    pub fn update_node(
        &mut self,
        text: String,
        speaker: String,
        choices: Vec<String>,
        speaker_entity: Option<bevy::prelude::Entity>,
    ) {
        self.current_text = text;
        self.current_speaker = speaker;
        self.current_choices = choices;
        self.speaker_entity = speaker_entity;
    }

    /// Ends the current dialogue and clears the active state/history.
    ///
    /// After calling `end` the `DialogueState` becomes inactive.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::dialogue::DialogueState;
    ///
    /// let mut state = DialogueState::start(1, 1);
    /// state.end();
    /// assert!(!state.is_active());
    /// assert!(state.dialogue_history.is_empty());
    /// ```
    pub fn end(&mut self) {
        self.active_tree_id = None;
        self.current_node_id = 0;
        self.dialogue_history.clear();
        self.current_text.clear();
        self.current_speaker.clear();
        self.current_choices.clear();
        self.recruitment_context = None;
        self.speaker_npc_id = None;
    }

    /// Returns true if a dialogue is currently active.
    pub fn is_active(&self) -> bool {
        self.active_tree_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dialogue::{DialogueId, NodeId};

    #[test]
    fn test_dialogue_state_new_is_inactive() {
        let s = DialogueState::new();
        assert!(!s.is_active());
        assert_eq!(s.active_tree_id, None);
        assert_eq!(s.current_node_id, 0);
        assert!(s.dialogue_history.is_empty());
        assert_eq!(s.speaker_entity, None);
        assert_eq!(s.speaker_npc_id, None);
        assert_eq!(s.recruitment_context, None);
    }

    #[test]
    fn test_dialogue_state_start_sets_active_tree_and_root_node() {
        let s = DialogueState::start(7 as DialogueId, 3 as NodeId, None, None);
        assert!(s.is_active());
        assert_eq!(s.active_tree_id, Some(7));
        assert_eq!(s.current_node_id, 3);
        assert_eq!(s.dialogue_history, vec![3]);
        assert_eq!(s.speaker_npc_id, None);
    }

    #[test]
    fn test_dialogue_state_advance_to_node_appends_history() {
        let mut s = DialogueState::start(2 as DialogueId, 10 as NodeId, None, None);
        s.advance_to(11 as NodeId);
        s.advance_to(12 as NodeId);

        assert_eq!(s.current_node_id, 12);
        assert_eq!(s.dialogue_history, vec![10, 11, 12]);
        assert_eq!(s.speaker_npc_id, None);
    }

    #[test]
    fn test_dialogue_state_end_resets_state() {
        let mut s = DialogueState::start(99 as DialogueId, 1 as NodeId, None, None);
        s.advance_to(2);
        s.end();

        assert!(!s.is_active());
        assert_eq!(s.active_tree_id, None);
        assert_eq!(s.current_node_id, 0);
        assert!(s.dialogue_history.is_empty());
        assert_eq!(s.speaker_npc_id, None);
    }

    #[test]
    fn test_dialogue_state_update_node() {
        let mut state = DialogueState::default();

        state.update_node(
            "Hello, traveler!".to_string(),
            "Village Elder".to_string(),
            vec!["Greetings".to_string(), "Farewell".to_string()],
            None,
        );

        assert_eq!(state.current_text, "Hello, traveler!");
        assert_eq!(state.current_speaker, "Village Elder");
        assert_eq!(state.current_choices.len(), 2);
        assert_eq!(state.current_choices[0], "Greetings");
    }

    #[test]
    fn test_dialogue_state_initialization() {
        let state = DialogueState::default();

        assert_eq!(state.current_text, "");
        assert_eq!(state.current_speaker, "");
        assert_eq!(state.current_choices.len(), 0);
    }

    #[test]
    fn test_update_node_overwrites_previous() {
        let mut state = DialogueState::default();

        state.update_node("First".to_string(), "Speaker1".to_string(), vec![], None);
        state.update_node(
            "Second".to_string(),
            "Speaker2".to_string(),
            vec!["Choice".to_string()],
            None,
        );

        assert_eq!(state.current_text, "Second");
        assert_eq!(state.current_speaker, "Speaker2");
        assert_eq!(state.current_choices.len(), 1);
    }

    #[test]
    fn test_dialogue_state_end_clears_visual_state() {
        let mut state = DialogueState::start(1 as DialogueId, 1 as NodeId, None, None);
        state.update_node(
            "Some text".to_string(),
            "Speaker".to_string(),
            vec!["Choice".to_string()],
            None,
        );

        state.end();

        assert_eq!(state.current_text, "");
        assert_eq!(state.current_speaker, "");
        assert!(state.current_choices.is_empty());
        assert_eq!(state.speaker_entity, None);
        assert_eq!(state.speaker_npc_id, None);
        assert_eq!(state.recruitment_context, None);
    }

    #[test]
    fn test_dialogue_state_speaker_entity() {
        use bevy::prelude::Entity;

        let mut state = DialogueState::default();
        let test_entity = Entity::from_bits(42);

        state.update_node(
            "Test".to_string(),
            "NPC".to_string(),
            vec![],
            Some(test_entity),
        );

        assert_eq!(state.speaker_entity, Some(test_entity));
    }

    #[test]
    fn test_dialogue_state_recruitment_context_none_by_default() {
        let state = DialogueState::default();
        assert_eq!(state.recruitment_context, None);
    }

    #[test]
    fn test_dialogue_state_start_recruitment_context_none() {
        let state = DialogueState::start(1 as DialogueId, 1 as NodeId, None, None);
        assert_eq!(state.recruitment_context, None);
    }

    #[test]
    fn test_dialogue_state_tracks_speaker_npc_id() {
        let state = DialogueState::start(
            42 as DialogueId,
            1 as NodeId,
            None,
            Some("inn_test".to_string()),
        );
        assert_eq!(state.speaker_npc_id, Some("inn_test".to_string()));
    }
}
