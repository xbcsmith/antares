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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Resource, Default)]
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
    /// let state = DialogueState::start(5 as DialogueId, 1 as NodeId);
    /// assert_eq!(state.active_tree_id, Some(5));
    /// assert_eq!(state.current_node_id, 1);
    /// assert_eq!(state.dialogue_history, vec![1]);
    /// ```
    pub fn start(tree_id: DialogueId, root_node: NodeId) -> Self {
        Self {
            active_tree_id: Some(tree_id),
            current_node_id: root_node,
            dialogue_history: vec![root_node],
            current_text: String::new(),
            current_speaker: String::new(),
            current_choices: Vec::new(),
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
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::dialogue::DialogueState;
    ///
    /// let mut state = DialogueState::new();
    /// state.update_node(
    ///     "Hello, adventurer!".to_string(),
    ///     "Village Elder".to_string(),
    ///     vec!["Greetings".to_string(), "Farewell".to_string()],
    /// );
    ///
    /// assert_eq!(state.current_text, "Hello, adventurer!");
    /// assert_eq!(state.current_speaker, "Village Elder");
    /// assert_eq!(state.current_choices.len(), 2);
    /// ```
    pub fn update_node(&mut self, text: String, speaker: String, choices: Vec<String>) {
        self.current_text = text;
        self.current_speaker = speaker;
        self.current_choices = choices;
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
    }

    #[test]
    fn test_dialogue_state_start_sets_active_tree_and_root_node() {
        let s = DialogueState::start(7 as DialogueId, 3 as NodeId);
        assert!(s.is_active());
        assert_eq!(s.active_tree_id, Some(7));
        assert_eq!(s.current_node_id, 3);
        assert_eq!(s.dialogue_history, vec![3]);
    }

    #[test]
    fn test_dialogue_state_advance_to_node_appends_history() {
        let mut s = DialogueState::start(2 as DialogueId, 10 as NodeId);
        s.advance_to(11 as NodeId);
        s.advance_to(12 as NodeId);

        assert_eq!(s.current_node_id, 12);
        assert_eq!(s.dialogue_history, vec![10, 11, 12]);
    }

    #[test]
    fn test_dialogue_state_end_resets_state() {
        let mut s = DialogueState::start(99 as DialogueId, 1 as NodeId);
        s.advance_to(2);
        s.end();

        assert!(!s.is_active());
        assert_eq!(s.active_tree_id, None);
        assert_eq!(s.current_node_id, 0);
        assert!(s.dialogue_history.is_empty());
    }

    #[test]
    fn test_dialogue_state_update_node() {
        let mut state = DialogueState::default();

        state.update_node(
            "Hello, traveler!".to_string(),
            "Village Elder".to_string(),
            vec!["Greetings".to_string(), "Farewell".to_string()],
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

        state.update_node("First".to_string(), "Speaker1".to_string(), vec![]);
        state.update_node(
            "Second".to_string(),
            "Speaker2".to_string(),
            vec!["Choice".to_string()],
        );

        assert_eq!(state.current_text, "Second");
        assert_eq!(state.current_speaker, "Speaker2");
        assert_eq!(state.current_choices.len(), 1);
    }

    #[test]
    fn test_dialogue_state_end_clears_visual_state() {
        let mut state = DialogueState::start(1 as DialogueId, 1 as NodeId);
        state.update_node(
            "Some text".to_string(),
            "Speaker".to_string(),
            vec!["Choice".to_string()],
        );

        state.end();

        assert_eq!(state.current_text, "");
        assert_eq!(state.current_speaker, "");
        assert!(state.current_choices.is_empty());
    }
}
