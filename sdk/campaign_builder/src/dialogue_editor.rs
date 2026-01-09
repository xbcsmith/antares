// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Dialogue Tree Editor for Campaign Builder
//!
//! This module provides a visual dialogue tree editor for creating and managing
//! NPC conversations with branching paths, conditions, and dialogue actions.
//!
//! The `show()` method provides full UI rendering following the standard editor pattern.
//! Uses shared UI components for consistent layout.
//!
//! # Features
//!
//! - Dialogue tree list view with search and filtering
//! - Node list view (flat list-based, not graph)
//! - Node editor with text, speaker, conditions, and actions
//! - Choice editor for player responses
//! - Condition and action configuration
//! - Dialogue tree validation and preview
//! - Import/export RON support

use crate::ui_helpers::{
    autocomplete_item_selector, autocomplete_quest_selector, ActionButtons, EditorToolbar,
    ItemAction, ToolbarAction, TwoColumnLayout,
};
use antares::domain::dialogue::{
    DialogueAction, DialogueChoice, DialogueCondition, DialogueId, DialogueNode, DialogueTree,
    NodeId,
};
use antares::domain::items::types::Item;
use antares::domain::quest::{Quest, QuestId};
use antares::domain::types::ItemId;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Editor state for dialogue tree editing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueEditorState {
    /// All dialogue trees being edited
    pub dialogues: Vec<DialogueTree>,

    /// Currently selected dialogue tree index
    pub selected_dialogue: Option<usize>,

    /// Currently selected node ID within selected tree
    pub selected_node: Option<NodeId>,

    /// Currently selected choice index within selected node
    pub selected_choice: Option<usize>,

    /// Dialogue editor mode
    pub mode: DialogueEditorMode,

    /// Edit buffer for dialogue tree form fields
    pub dialogue_buffer: DialogueEditBuffer,

    /// Edit buffer for node form fields
    pub node_buffer: NodeEditBuffer,

    /// Edit buffer for choice form fields
    pub choice_buffer: ChoiceEditBuffer,

    /// Edit buffer for condition form fields
    pub condition_buffer: ConditionEditBuffer,

    /// Edit buffer for action form fields
    pub action_buffer: ActionEditBuffer,

    /// Dialogue search/filter string
    pub search_filter: String,

    /// Unsaved changes flag
    pub has_unsaved_changes: bool,

    /// Validation errors for current dialogue
    pub validation_errors: Vec<String>,

    /// Available dialogue IDs (for cross-references)
    pub available_dialogue_ids: Vec<DialogueId>,

    /// Available quests (for conditions/actions/associated quest)
    pub quests: Vec<Quest>,

    /// Available items (for conditions/actions)
    pub items: Vec<Item>,

    /// Whether to show dialogue preview in list view
    pub show_preview: bool,

    /// Whether to show import dialog
    pub show_import_dialog: bool,

    /// Import buffer for RON text
    pub import_buffer: String,

    /// Whether we're currently editing a node (vs adding a new one)
    pub editing_node: bool,

    /// Node search filter for "Find Node by ID"
    pub node_search_filter: String,

    /// Unreachable nodes in current dialogue (cached from last validation)
    pub unreachable_nodes: std::collections::HashSet<NodeId>,

    /// Validation errors for current dialogue (including broken targets)
    pub dialogue_validation_errors: Vec<String>,

    /// Track navigation path through dialogue tree
    pub navigation_path: Vec<NodeId>,

    /// Target node for jump-to navigation
    pub jump_to_node: Option<NodeId>,
}

/// Dialogue editor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogueEditorMode {
    /// Viewing list of dialogues
    List,
    /// Creating new dialogue
    Creating,
    /// Editing existing dialogue
    Editing,
}

/// Buffer for dialogue tree form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueEditBuffer {
    pub id: String,
    pub name: String,
    pub speaker_name: String,
    pub repeatable: bool,
    pub associated_quest: String,
}

impl Default for DialogueEditBuffer {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            speaker_name: String::new(),
            repeatable: false,
            associated_quest: String::new(),
        }
    }
}

/// Buffer for node form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEditBuffer {
    pub id: String,
    pub text: String,
    pub speaker_override: String,
    pub is_terminal: bool,
}

impl Default for NodeEditBuffer {
    fn default() -> Self {
        Self {
            id: String::new(),
            text: String::new(),
            speaker_override: String::new(),
            is_terminal: false,
        }
    }
}

/// Buffer for choice form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceEditBuffer {
    pub text: String,
    pub target_node: String,
    pub ends_dialogue: bool,
}

impl Default for ChoiceEditBuffer {
    fn default() -> Self {
        Self {
            text: String::new(),
            target_node: String::new(),
            ends_dialogue: false,
        }
    }
}

/// Buffer for condition form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionEditBuffer {
    pub condition_type: ConditionType,
    pub quest_id: String,
    pub stage_number: String,
    pub item_id: String,
    pub item_quantity: String,
    pub gold_amount: String,
    pub min_level: String,
    pub flag_name: String,
    pub flag_value: bool,
    pub faction_name: String,
    pub reputation_threshold: String,
}

/// Condition type selector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionType {
    HasQuest,
    CompletedQuest,
    QuestStage,
    HasItem,
    HasGold,
    MinLevel,
    FlagSet,
    ReputationThreshold,
}

impl ConditionType {
    pub fn as_str(&self) -> &str {
        match self {
            ConditionType::HasQuest => "Has Quest",
            ConditionType::CompletedQuest => "Completed Quest",
            ConditionType::QuestStage => "Quest Stage",
            ConditionType::HasItem => "Has Item",
            ConditionType::HasGold => "Has Gold",
            ConditionType::MinLevel => "Minimum Level",
            ConditionType::FlagSet => "Flag Set",
            ConditionType::ReputationThreshold => "Reputation Threshold",
        }
    }
}

impl Default for ConditionEditBuffer {
    fn default() -> Self {
        Self {
            condition_type: ConditionType::HasQuest,
            quest_id: String::new(),
            stage_number: "1".to_string(),
            item_id: String::new(),
            item_quantity: "1".to_string(),
            gold_amount: "0".to_string(),
            min_level: "1".to_string(),
            flag_name: String::new(),
            flag_value: false,
            faction_name: String::new(),
            reputation_threshold: "0".to_string(),
        }
    }
}

/// Buffer for action form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionEditBuffer {
    pub action_type: ActionType,
    pub quest_id: String,
    pub stage_number: String,
    pub item_id: String,
    pub item_quantity: String,
    pub gold_amount: String,
    pub unlock_quest_id: String,
    pub flag_name: String,
    pub flag_value: bool,
    pub faction_name: String,
    pub reputation_change: String,
    pub event_name: String,
    pub experience_amount: String,
}

/// Action type selector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    StartQuest,
    CompleteQuestStage,
    GiveItems,
    TakeItems,
    GiveGold,
    TakeGold,
    SetFlag,
    ChangeReputation,
    TriggerEvent,
    GrantExperience,
}

impl ActionType {
    pub fn as_str(&self) -> &str {
        match self {
            ActionType::StartQuest => "Start Quest",
            ActionType::CompleteQuestStage => "Complete Quest Stage",
            ActionType::GiveItems => "Give Items",
            ActionType::TakeItems => "Take Items",
            ActionType::GiveGold => "Give Gold",
            ActionType::TakeGold => "Take Gold",
            ActionType::SetFlag => "Set Flag",
            ActionType::ChangeReputation => "Change Reputation",
            ActionType::TriggerEvent => "Trigger Event",
            ActionType::GrantExperience => "Grant Experience",
        }
    }
}

impl Default for ActionEditBuffer {
    fn default() -> Self {
        Self {
            action_type: ActionType::StartQuest,
            quest_id: String::new(),
            stage_number: "1".to_string(),
            item_id: String::new(),
            item_quantity: "1".to_string(),
            gold_amount: "0".to_string(),
            unlock_quest_id: String::new(),
            flag_name: String::new(),
            flag_value: false,
            faction_name: String::new(),
            reputation_change: "0".to_string(),
            event_name: String::new(),
            experience_amount: "100".to_string(),
        }
    }
}

impl Default for DialogueEditorState {
    fn default() -> Self {
        Self {
            dialogues: Vec::new(),
            selected_dialogue: None,
            selected_node: None,
            selected_choice: None,
            mode: DialogueEditorMode::List,
            dialogue_buffer: DialogueEditBuffer::default(),
            node_buffer: NodeEditBuffer::default(),
            choice_buffer: ChoiceEditBuffer::default(),
            condition_buffer: ConditionEditBuffer::default(),
            action_buffer: ActionEditBuffer::default(),
            search_filter: String::new(),
            has_unsaved_changes: false,
            validation_errors: Vec::new(),
            available_dialogue_ids: Vec::new(),
            quests: Vec::new(),
            items: Vec::new(),
            show_preview: false,
            show_import_dialog: false,
            import_buffer: String::new(),
            editing_node: false,
            node_search_filter: String::new(),
            unreachable_nodes: std::collections::HashSet::new(),
            dialogue_validation_errors: Vec::new(),
            navigation_path: Vec::new(),
            jump_to_node: None,
        }
    }
}

impl DialogueEditorState {
    /// Create a new dialogue editor state
    pub fn new() -> Self {
        Self::default()
    }

    /// Edit an existing node
    pub fn edit_node(&mut self, dialogue_idx: usize, node_id: NodeId) -> Result<(), String> {
        if dialogue_idx >= self.dialogues.len() {
            return Err("Invalid dialogue index".to_string());
        }

        let dialogue = &self.dialogues[dialogue_idx];
        if let Some(node) = dialogue.nodes.get(&node_id) {
            self.node_buffer = NodeEditBuffer {
                id: node_id.to_string(),
                text: node.text.clone(),
                speaker_override: node.speaker_override.clone().unwrap_or_default(),
                is_terminal: node.is_terminal,
            };
            self.selected_node = Some(node_id);
            self.editing_node = true;
            Ok(())
        } else {
            Err("Node not found".to_string())
        }
    }

    /// Save edited node
    pub fn save_node(&mut self, dialogue_idx: usize, node_id: NodeId) -> Result<(), String> {
        if dialogue_idx >= self.dialogues.len() {
            return Err("Invalid dialogue index".to_string());
        }

        let node_id_parsed = self
            .node_buffer
            .id
            .parse::<NodeId>()
            .map_err(|_| "Invalid node ID".to_string())?;

        let dialogue = &mut self.dialogues[dialogue_idx];
        if let Some(node) = dialogue.nodes.get_mut(&node_id) {
            node.text = self.node_buffer.text.clone();
            node.speaker_override = if self.node_buffer.speaker_override.is_empty() {
                None
            } else {
                Some(self.node_buffer.speaker_override.clone())
            };
            node.is_terminal = self.node_buffer.is_terminal;

            self.has_unsaved_changes = true;
            self.selected_node = None;
            self.editing_node = false;
            self.node_buffer = NodeEditBuffer::default();
            Ok(())
        } else {
            Err("Node not found".to_string())
        }
    }

    /// Delete a node from dialogue
    pub fn delete_node(&mut self, dialogue_idx: usize, node_id: NodeId) -> Result<(), String> {
        if dialogue_idx >= self.dialogues.len() {
            return Err("Invalid dialogue index".to_string());
        }

        let dialogue = &mut self.dialogues[dialogue_idx];

        // Don't allow deleting root node
        if node_id == dialogue.root_node {
            return Err("Cannot delete root node".to_string());
        }

        dialogue.nodes.remove(&node_id);
        self.has_unsaved_changes = true;

        if self.selected_node == Some(node_id) {
            self.selected_node = None;
        }

        Ok(())
    }

    /// Edit an existing choice
    pub fn edit_choice(
        &mut self,
        dialogue_idx: usize,
        node_id: NodeId,
        choice_idx: usize,
    ) -> Result<(), String> {
        if dialogue_idx >= self.dialogues.len() {
            return Err("Invalid dialogue index".to_string());
        }

        let dialogue = &self.dialogues[dialogue_idx];
        if let Some(node) = dialogue.nodes.get(&node_id) {
            if choice_idx >= node.choices.len() {
                return Err("Invalid choice index".to_string());
            }

            let choice = &node.choices[choice_idx];
            self.choice_buffer = ChoiceEditBuffer {
                text: choice.text.clone(),
                target_node: choice
                    .target_node
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
                ends_dialogue: choice.ends_dialogue,
            };
            self.selected_choice = Some(choice_idx);
            Ok(())
        } else {
            Err("Node not found".to_string())
        }
    }

    /// Save edited choice
    pub fn save_choice(
        &mut self,
        dialogue_idx: usize,
        node_id: NodeId,
        choice_idx: usize,
    ) -> Result<(), String> {
        if dialogue_idx >= self.dialogues.len() {
            return Err("Invalid dialogue index".to_string());
        }

        let dialogue = &mut self.dialogues[dialogue_idx];
        if let Some(node) = dialogue.nodes.get_mut(&node_id) {
            if choice_idx >= node.choices.len() {
                return Err("Invalid choice index".to_string());
            }

            let target_node = if self.choice_buffer.target_node.is_empty() {
                None
            } else {
                Some(
                    self.choice_buffer
                        .target_node
                        .parse::<NodeId>()
                        .map_err(|_| "Invalid target node ID".to_string())?,
                )
            };

            let choice = &mut node.choices[choice_idx];
            choice.text = self.choice_buffer.text.clone();
            choice.target_node = target_node;
            choice.ends_dialogue = self.choice_buffer.ends_dialogue;

            self.has_unsaved_changes = true;
            self.selected_choice = None;
            Ok(())
        } else {
            Err("Node not found".to_string())
        }
    }

    /// Delete a choice from a node
    pub fn delete_choice(
        &mut self,
        dialogue_idx: usize,
        node_id: NodeId,
        choice_idx: usize,
    ) -> Result<(), String> {
        if dialogue_idx >= self.dialogues.len() {
            return Err("Invalid dialogue index".to_string());
        }

        let dialogue = &mut self.dialogues[dialogue_idx];
        if let Some(node) = dialogue.nodes.get_mut(&node_id) {
            if choice_idx >= node.choices.len() {
                return Err("Invalid choice index".to_string());
            }

            node.choices.remove(choice_idx);
            self.has_unsaved_changes = true;

            if self.selected_choice == Some(choice_idx) {
                self.selected_choice = None;
            }

            Ok(())
        } else {
            Err("Node not found".to_string())
        }
    }

    /// Find unreachable nodes in all dialogues
    pub fn find_unreachable_nodes(&self) -> Vec<(DialogueId, Vec<NodeId>)> {
        let mut unreachable = Vec::new();

        for dialogue in &self.dialogues {
            let mut reachable = std::collections::HashSet::new();
            let mut to_visit = vec![dialogue.root_node];

            // BFS to find all reachable nodes
            while let Some(node_id) = to_visit.pop() {
                if reachable.contains(&node_id) {
                    continue;
                }
                reachable.insert(node_id);

                if let Some(node) = dialogue.nodes.get(&node_id) {
                    for choice in &node.choices {
                        if let Some(target) = choice.target_node {
                            if !reachable.contains(&target) {
                                to_visit.push(target);
                            }
                        }
                    }
                }
            }

            // Find nodes that exist but are not reachable
            let mut unreachable_nodes: Vec<NodeId> = dialogue
                .nodes
                .keys()
                .filter(|id| !reachable.contains(id))
                .copied()
                .collect();

            if !unreachable_nodes.is_empty() {
                unreachable_nodes.sort();
                unreachable.push((dialogue.id, unreachable_nodes));
            }
        }

        unreachable
    }

    /// Get unreachable nodes for current dialogue with cached results
    pub fn get_unreachable_nodes_for_dialogue(
        &mut self,
        dialogue_idx: usize,
    ) -> std::collections::HashSet<NodeId> {
        if dialogue_idx >= self.dialogues.len() {
            return std::collections::HashSet::new();
        }

        let dialogue = &self.dialogues[dialogue_idx];
        let mut reachable = std::collections::HashSet::new();
        let mut to_visit = vec![dialogue.root_node];

        // BFS to find all reachable nodes
        while let Some(node_id) = to_visit.pop() {
            if reachable.contains(&node_id) {
                continue;
            }
            reachable.insert(node_id);

            if let Some(node) = dialogue.nodes.get(&node_id) {
                for choice in &node.choices {
                    if let Some(target) = choice.target_node {
                        if !reachable.contains(&target) {
                            to_visit.push(target);
                        }
                    }
                }
            }
        }

        // Return nodes that exist but are not reachable
        dialogue
            .nodes
            .keys()
            .filter(|id| !reachable.contains(id))
            .copied()
            .collect()
    }

    /// Validate dialogue tree and collect inline errors
    ///
    /// Returns validation errors including:
    /// - Missing root node
    /// - Non-existent choice targets
    /// - Unreachable nodes
    pub fn validate_dialogue_tree(&mut self, dialogue_idx: usize) -> Vec<String> {
        let mut errors = Vec::new();

        if dialogue_idx >= self.dialogues.len() {
            return errors;
        }

        let dialogue = &self.dialogues[dialogue_idx];

        // Check root node exists
        if !dialogue.nodes.contains_key(&dialogue.root_node) {
            errors.push(format!("Root node {} does not exist", dialogue.root_node));
            return errors;
        }

        // Check all choice targets exist
        for (node_id, node) in &dialogue.nodes {
            for (idx, choice) in node.choices.iter().enumerate() {
                if let Some(target) = choice.target_node {
                    if !dialogue.nodes.contains_key(&target) {
                        errors.push(format!(
                            "Node {} choice {} references non-existent node {}",
                            node_id, idx, target
                        ));
                    }
                }
            }
        }

        // Get unreachable nodes
        let unreachable = self.get_unreachable_nodes_for_dialogue(dialogue_idx);
        if !unreachable.is_empty() {
            let mut unreachable_list: Vec<_> = unreachable.iter().copied().collect();
            unreachable_list.sort();
            errors.push(format!(
                "Unreachable nodes (not connected from root): {}",
                unreachable_list
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        self.unreachable_nodes = unreachable;
        self.dialogue_validation_errors = errors.clone();
        errors
    }

    /// Get reachability stats for a dialogue
    pub fn get_reachability_stats(&mut self, dialogue_idx: usize) -> (usize, usize, usize) {
        if dialogue_idx >= self.dialogues.len() {
            return (0, 0, 0);
        }

        let dialogue = &self.dialogues[dialogue_idx];
        let total_nodes = dialogue.node_count();
        let unreachable = self.get_unreachable_nodes_for_dialogue(dialogue_idx);
        let unreachable_count = unreachable.len();
        let reachable_count = total_nodes.saturating_sub(unreachable_count);

        (total_nodes, reachable_count, unreachable_count)
    }

    /// Get preview text for a node (first 50 chars)
    pub fn get_node_preview(&self, dialogue_idx: usize, node_id: NodeId) -> String {
        if dialogue_idx >= self.dialogues.len() {
            return String::new();
        }

        let dialogue = &self.dialogues[dialogue_idx];
        if let Some(node) = dialogue.get_node(node_id) {
            let preview = node.text.chars().take(50).collect::<String>();
            if node.text.len() > 50 {
                format!("{}…", preview)
            } else {
                preview
            }
        } else {
            String::new()
        }
    }

    /// Check if a choice target is valid
    pub fn is_choice_target_valid(&self, dialogue_idx: usize, target: NodeId) -> bool {
        if dialogue_idx >= self.dialogues.len() {
            return false;
        }

        self.dialogues[dialogue_idx].nodes.contains_key(&target)
    }

    /// Find nodes matching search filter
    pub fn search_nodes(&self, dialogue_idx: usize, search: &str) -> Vec<NodeId> {
        if dialogue_idx >= self.dialogues.len() || search.trim().is_empty() {
            return Vec::new();
        }

        let search_lower = search.to_lowercase();
        let dialogue = &self.dialogues[dialogue_idx];

        dialogue
            .nodes
            .iter()
            .filter(|(_, node)| {
                node.id.to_string().contains(&search_lower)
                    || node.text.to_lowercase().contains(&search_lower)
            })
            .map(|(_, node)| node.id)
            .collect()
    }

    /// Load dialogues from data
    pub fn load_dialogues(&mut self, dialogues: Vec<DialogueTree>) {
        self.dialogues = dialogues;
        self.has_unsaved_changes = false;
        self.validation_errors.clear();
    }

    /// Get filtered dialogues based on search
    pub fn filtered_dialogues(&self) -> Vec<(usize, &DialogueTree)> {
        self.dialogues
            .iter()
            .enumerate()
            .filter(|(_, dialogue)| {
                self.search_filter.is_empty()
                    || dialogue
                        .name
                        .to_lowercase()
                        .contains(&self.search_filter.to_lowercase())
                    || dialogue.id.to_string().contains(&self.search_filter)
            })
            .collect()
    }

    /// Start creating a new dialogue
    pub fn start_new_dialogue(&mut self) {
        self.mode = DialogueEditorMode::Creating;
        self.dialogue_buffer = DialogueEditBuffer::default();
        self.selected_node = None;
        self.selected_choice = None;
        self.validation_errors.clear();
    }

    /// Start editing the selected dialogue
    pub fn start_edit_dialogue(&mut self, index: usize) {
        if index < self.dialogues.len() {
            self.selected_dialogue = Some(index);
            let dialogue = &self.dialogues[index];

            self.dialogue_buffer = DialogueEditBuffer {
                id: dialogue.id.to_string(),
                name: dialogue.name.clone(),
                speaker_name: dialogue.speaker_name.clone().unwrap_or_default(),
                repeatable: dialogue.repeatable,
                associated_quest: dialogue
                    .associated_quest
                    .map_or(String::new(), |q| q.to_string()),
            };

            self.mode = DialogueEditorMode::Editing;
            self.selected_node = Some(dialogue.root_node);
            self.selected_choice = None;
            self.validation_errors.clear();
        }
    }

    /// Save current dialogue being edited
    pub fn save_dialogue(&mut self) -> Result<(), String> {
        let id = self
            .dialogue_buffer
            .id
            .parse::<DialogueId>()
            .map_err(|_| "Invalid dialogue ID".to_string())?;

        let associated_quest = if self.dialogue_buffer.associated_quest.is_empty() {
            None
        } else {
            Some(
                self.dialogue_buffer
                    .associated_quest
                    .parse::<QuestId>()
                    .map_err(|_| "Invalid quest ID".to_string())?,
            )
        };

        let mut dialogue = if let Some(idx) = self.selected_dialogue {
            self.dialogues[idx].clone()
        } else {
            DialogueTree::new(id, &self.dialogue_buffer.name, 1)
        };

        dialogue.name = self.dialogue_buffer.name.clone();
        dialogue.speaker_name = if self.dialogue_buffer.speaker_name.is_empty() {
            None
        } else {
            Some(self.dialogue_buffer.speaker_name.clone())
        };
        dialogue.repeatable = self.dialogue_buffer.repeatable;
        dialogue.associated_quest = associated_quest;

        if let Some(idx) = self.selected_dialogue {
            self.dialogues[idx] = dialogue;
        } else {
            self.dialogues.push(dialogue);
        }

        self.has_unsaved_changes = true;
        self.mode = DialogueEditorMode::List;
        self.selected_dialogue = None;
        Ok(())
    }

    /// Cancel current edit
    pub fn cancel_edit(&mut self) {
        self.mode = DialogueEditorMode::List;
        self.selected_dialogue = None;
        self.selected_node = None;
        self.selected_choice = None;
        self.dialogue_buffer = DialogueEditBuffer::default();
        self.validation_errors.clear();
    }

    /// Delete dialogue at index
    pub fn delete_dialogue(&mut self, index: usize) {
        if index < self.dialogues.len() {
            self.dialogues.remove(index);
            self.has_unsaved_changes = true;
        }
    }

    /// Add a new node to current dialogue
    pub fn add_node(&mut self) -> Result<NodeId, String> {
        if self.selected_dialogue.is_none() {
            return Err("No dialogue selected".to_string());
        }

        if self.node_buffer.text.trim().is_empty() {
            return Err("Node text cannot be empty".to_string());
        }

        let node_id = self
            .next_available_node_id()
            .ok_or_else(|| "No dialogue selected".to_string())?;

        let mut node = DialogueNode::new(node_id, &self.node_buffer.text);

        if !self.node_buffer.speaker_override.is_empty() {
            node.speaker_override = Some(self.node_buffer.speaker_override.clone());
        }
        node.is_terminal = self.node_buffer.is_terminal;

        if let Some(idx) = self.selected_dialogue {
            self.dialogues[idx].add_node(node);
            self.has_unsaved_changes = true;
            self.node_buffer = NodeEditBuffer::default();
            Ok(node_id)
        } else {
            Err("No dialogue selected".to_string())
        }
    }

    /// Add a choice to current node
    pub fn add_choice(&mut self) -> Result<(), String> {
        if let Some(dialogue_idx) = self.selected_dialogue {
            if dialogue_idx >= self.dialogues.len() {
                return Err("Invalid dialogue index".to_string());
            }

            if let Some(node_id) = self.selected_node {
                let target_node = self
                    .choice_buffer
                    .target_node
                    .parse::<NodeId>()
                    .map_err(|_| "Invalid target node ID".to_string())?;

                let mut choice = DialogueChoice::new(&self.choice_buffer.text, Some(target_node));
                choice.ends_dialogue = self.choice_buffer.ends_dialogue;

                // Verify target node exists
                if !self.dialogues[dialogue_idx]
                    .nodes
                    .contains_key(&target_node)
                {
                    return Err("Target node does not exist".to_string());
                }

                // Add choice to node (accessing nodes directly since no mutable getter)
                if let Some(node) = self.dialogues[dialogue_idx].nodes.get_mut(&node_id) {
                    node.add_choice(choice);
                    self.has_unsaved_changes = true;
                    self.choice_buffer = ChoiceEditBuffer::default();
                    self.selected_choice = None;
                    return Ok(());
                }

                Err("Node not found".to_string())
            } else {
                Err("No node selected".to_string())
            }
        } else {
            Err("No dialogue selected".to_string())
        }
    }

    /// Build a condition from current params
    pub fn build_condition_from_buffer(&self) -> Result<DialogueCondition, String> {
        Ok(match self.condition_buffer.condition_type {
            ConditionType::HasQuest => {
                let quest_id = self
                    .condition_buffer
                    .quest_id
                    .parse::<QuestId>()
                    .map_err(|_| "Invalid quest ID".to_string())?;
                DialogueCondition::HasQuest { quest_id }
            }
            ConditionType::CompletedQuest => {
                let quest_id = self
                    .condition_buffer
                    .quest_id
                    .parse::<QuestId>()
                    .map_err(|_| "Invalid quest ID".to_string())?;
                DialogueCondition::CompletedQuest { quest_id }
            }
            ConditionType::QuestStage => {
                let quest_id = self
                    .condition_buffer
                    .quest_id
                    .parse::<QuestId>()
                    .map_err(|_| "Invalid quest ID".to_string())?;
                let stage_number = self
                    .condition_buffer
                    .stage_number
                    .parse::<u8>()
                    .map_err(|_| "Invalid stage number".to_string())?;
                DialogueCondition::QuestStage {
                    quest_id,
                    stage_number,
                }
            }
            ConditionType::HasItem => {
                let item_id = self
                    .condition_buffer
                    .item_id
                    .parse::<ItemId>()
                    .map_err(|_| "Invalid item ID".to_string())?;
                let quantity = self
                    .condition_buffer
                    .item_quantity
                    .parse::<u16>()
                    .map_err(|_| "Invalid quantity".to_string())?;
                DialogueCondition::HasItem { item_id, quantity }
            }
            ConditionType::HasGold => {
                let amount = self
                    .condition_buffer
                    .gold_amount
                    .parse::<u32>()
                    .map_err(|_| "Invalid gold amount".to_string())?;
                DialogueCondition::HasGold { amount }
            }
            ConditionType::MinLevel => {
                let level = self
                    .condition_buffer
                    .min_level
                    .parse::<u8>()
                    .map_err(|_| "Invalid level".to_string())?;
                DialogueCondition::MinLevel { level }
            }
            ConditionType::FlagSet => DialogueCondition::FlagSet {
                flag_name: self.condition_buffer.flag_name.clone(),
                value: self.condition_buffer.flag_value,
            },
            ConditionType::ReputationThreshold => DialogueCondition::ReputationThreshold {
                faction: self.condition_buffer.faction_name.clone(),
                threshold: self
                    .condition_buffer
                    .reputation_threshold
                    .parse::<i16>()
                    .map_err(|_| "Invalid reputation threshold".to_string())?,
            },
        })
    }

    /// Build an action from current params
    pub fn build_action_from_buffer(&self) -> Result<DialogueAction, String> {
        Ok(match self.action_buffer.action_type {
            ActionType::StartQuest => {
                let quest_id = self
                    .action_buffer
                    .quest_id
                    .parse::<QuestId>()
                    .map_err(|_| "Invalid quest ID".to_string())?;
                DialogueAction::StartQuest { quest_id }
            }
            ActionType::CompleteQuestStage => {
                let quest_id = self
                    .action_buffer
                    .quest_id
                    .parse::<QuestId>()
                    .map_err(|_| "Invalid quest ID".to_string())?;
                let stage_number = self
                    .action_buffer
                    .stage_number
                    .parse::<u8>()
                    .map_err(|_| "Invalid stage number".to_string())?;
                DialogueAction::CompleteQuestStage {
                    quest_id,
                    stage_number,
                }
            }
            ActionType::GiveItems => {
                let item_id = self
                    .action_buffer
                    .item_id
                    .parse::<ItemId>()
                    .map_err(|_| "Invalid item ID".to_string())?;
                DialogueAction::GiveItems {
                    items: vec![(item_id, 1)],
                }
            }
            ActionType::TakeItems => {
                let item_id = self
                    .action_buffer
                    .item_id
                    .parse::<ItemId>()
                    .map_err(|_| "Invalid item ID".to_string())?;
                DialogueAction::TakeItems {
                    items: vec![(item_id, 1)],
                }
            }
            ActionType::GiveGold => {
                let amount = self
                    .action_buffer
                    .gold_amount
                    .parse::<u32>()
                    .map_err(|_| "Invalid gold amount".to_string())?;
                DialogueAction::GiveGold { amount }
            }
            ActionType::TakeGold => {
                let amount = self
                    .action_buffer
                    .gold_amount
                    .parse::<u32>()
                    .map_err(|_| "Invalid gold amount".to_string())?;
                DialogueAction::TakeGold { amount }
            }
            ActionType::SetFlag => DialogueAction::SetFlag {
                flag_name: self.action_buffer.flag_name.clone(),
                value: self.action_buffer.flag_value,
            },
            ActionType::ChangeReputation => DialogueAction::ChangeReputation {
                faction: self.action_buffer.faction_name.clone(),
                change: self
                    .action_buffer
                    .reputation_change
                    .parse::<i16>()
                    .map_err(|_| "Invalid reputation change".to_string())?,
            },
            ActionType::TriggerEvent => DialogueAction::TriggerEvent {
                event_name: self.action_buffer.event_name.clone(),
            },
            ActionType::GrantExperience => DialogueAction::GrantExperience {
                amount: self
                    .action_buffer
                    .experience_amount
                    .parse::<u32>()
                    .map_err(|_| "Invalid experience amount".to_string())?,
            },
        })
    }

    /// Validate current dialogue
    pub fn validate_current_dialogue(&mut self) {
        self.validation_errors.clear();

        if let Some(idx) = self.selected_dialogue {
            if idx < self.dialogues.len() {
                let dialogue = &self.dialogues[idx];

                if dialogue.name.is_empty() {
                    self.validation_errors
                        .push("Dialogue name cannot be empty".to_string());
                }

                if !dialogue.has_nodes() {
                    self.validation_errors
                        .push("Dialogue must have at least one node".to_string());
                }

                // Validate root node exists
                if dialogue.get_node(dialogue.root_node).is_none() {
                    self.validation_errors
                        .push(format!("Root node {} does not exist", dialogue.root_node));
                }

                // Validate all choice targets exist
                for node in dialogue.nodes.values() {
                    for choice in &node.choices {
                        if let Some(target) = choice.target_node {
                            if dialogue.get_node(target).is_none() {
                                self.validation_errors.push(format!(
                                    "Choice in node {} targets non-existent node {}",
                                    node.id, target
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get preview text for current dialogue
    pub fn get_dialogue_preview(&self, index: usize) -> String {
        if index >= self.dialogues.len() {
            return String::new();
        }

        let dialogue = &self.dialogues[index];
        let mut preview = format!(
            "Dialogue: {}\n\nID: {}\nSpeaker: {}\n\n",
            dialogue.name,
            dialogue.id,
            dialogue
                .speaker_name
                .clone()
                .unwrap_or_else(|| "[No speaker]".to_string())
        );

        if let Some(quest_id) = dialogue.associated_quest {
            preview.push_str(&format!("Associated Quest: {}\n\n", quest_id));
        }

        preview.push_str(&format!("Nodes: {}\n", dialogue.node_count()));

        if let Some(root) = dialogue.get_node(dialogue.root_node) {
            preview.push_str(&format!("\nRoot Node Text:\n{}\n", root.text));
        }

        preview
    }

    /// Generate next available dialogue ID
    pub fn next_available_dialogue_id(&self) -> u16 {
        self.dialogues
            .iter()
            .map(|d| d.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    /// Finds the next available node ID for the currently selected dialogue.
    ///
    /// Returns the maximum node ID in the selected dialogue plus 1, or 1 if no nodes exist.
    /// Returns `None` if no dialogue is currently selected.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut editor = DialogueEditorState::new();
    /// editor.start_new_dialogue();
    /// assert_eq!(editor.next_available_node_id(), Some(1));
    /// ```
    pub fn next_available_node_id(&self) -> Option<NodeId> {
        if let Some(idx) = self.selected_dialogue {
            let dialogue = &self.dialogues[idx];
            let max_id = dialogue.nodes.keys().max().copied().unwrap_or(0);
            Some(max_id.saturating_add(1))
        } else {
            None
        }
    }

    /// Load dialogues from a file path
    pub fn load_from_file(&mut self, path: &std::path::Path) -> Result<(), String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
        let dialogues: Vec<DialogueTree> =
            ron::from_str(&content).map_err(|e| format!("Failed to parse dialogues: {}", e))?;
        self.dialogues = dialogues;
        self.has_unsaved_changes = false;
        Ok(())
    }

    /// Save dialogues to a file path
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        let content = ron::ser::to_string_pretty(&self.dialogues, Default::default())
            .map_err(|e| format!("Failed to serialize dialogues: {}", e))?;
        std::fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(())
    }

    /// Main UI rendering method following standard editor signature
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `dialogues` - Mutable reference to the dialogues data (synced with app state)
    /// * `quests` - Slice of available quests for autocomplete
    /// * `items` - Slice of available items for autocomplete
    /// * `campaign_dir` - Optional campaign directory path
    /// * `dialogue_file` - Filename for dialogue data
    /// * `unsaved_changes` - Mutable flag for tracking unsaved changes
    /// * `status_message` - Mutable string for status messages
    /// * `file_load_merge_mode` - Whether to merge or replace when loading files
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        dialogues: &mut Vec<DialogueTree>,
        quests: &[Quest],
        items: &[Item],
        campaign_dir: Option<&PathBuf>,
        dialogue_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        file_load_merge_mode: &mut bool,
    ) {
        // Sync dialogues from parameter to internal state
        self.dialogues = dialogues.clone();
        self.quests = quests.to_vec();
        self.items = items.to_vec();

        ui.heading("💬 Dialogues Editor");
        ui.add_space(5.0);

        // Use shared EditorToolbar component
        let toolbar_action = EditorToolbar::new("Dialogues")
            .with_search(&mut self.search_filter)
            .with_merge_mode(file_load_merge_mode)
            .with_total_count(self.dialogues.len())
            .with_id_salt("dialogues_toolbar")
            .show(ui);

        // Handle toolbar actions
        match toolbar_action {
            ToolbarAction::New => {
                self.start_new_dialogue();
            }
            ToolbarAction::Save => {
                // Save to campaign directory
                if let Some(dir) = campaign_dir {
                    let path = dir.join(dialogue_file);
                    match self.save_to_file(&path) {
                        Ok(()) => {
                            *dialogues = self.dialogues.clone();
                            *status_message = format!("Saved {} dialogues", self.dialogues.len());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to save dialogues: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Load => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("RON", &["ron"])
                    .pick_file()
                {
                    match self.load_from_file(&path) {
                        Ok(()) => {
                            *dialogues = self.dialogues.clone();
                            *status_message = format!("Loaded {} dialogues", self.dialogues.len());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to load dialogues: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Import => {
                self.show_import_dialog = true;
                self.import_buffer.clear();
            }
            ToolbarAction::Export => {
                match ron::ser::to_string_pretty(&self.dialogues, Default::default()) {
                    Ok(ron_str) => {
                        ui.ctx().copy_text(ron_str.clone());
                        *status_message = "Dialogues exported to clipboard".to_string();
                    }
                    Err(e) => {
                        *status_message = format!("Export failed: {:?}", e);
                    }
                }
            }
            ToolbarAction::Reload => {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(dialogue_file);
                    if path.exists() {
                        match self.load_from_file(&path) {
                            Ok(()) => {
                                *dialogues = self.dialogues.clone();
                                *status_message =
                                    format!("Reloaded {} dialogues", self.dialogues.len());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to reload dialogues: {}", e);
                            }
                        }
                    } else {
                        *status_message = "Dialogues file does not exist".to_string();
                    }
                }
            }
            ToolbarAction::None => {}
        }

        // Additional options row
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_preview, "Show Preview");
        });

        ui.separator();

        // Import dialog
        self.show_import_dialog_window(ui, dialogues, status_message);

        // Main content area
        match self.mode {
            DialogueEditorMode::List => {
                self.show_dialogue_list(ui, dialogues, unsaved_changes, status_message);
            }
            DialogueEditorMode::Creating | DialogueEditorMode::Editing => {
                self.show_dialogue_form(ui, dialogues, status_message);
            }
        }
    }

    /// Show the import dialog window
    fn show_import_dialog_window(
        &mut self,
        ui: &mut egui::Ui,
        dialogues: &mut Vec<DialogueTree>,
        status_message: &mut String,
    ) {
        let mut import_complete = false;
        let mut import_result: Option<Result<DialogueTree, String>> = None;
        let mut cancel_import = false;

        if self.show_import_dialog {
            let mut show_dialog = self.show_import_dialog;
            egui::Window::new("Import Dialogue RON")
                .open(&mut show_dialog)
                .show(ui.ctx(), |ui| {
                    ui.label("Paste RON dialogue data:");
                    ui.add(
                        egui::TextEdit::multiline(&mut self.import_buffer)
                            .desired_width(500.0)
                            .desired_rows(15),
                    );

                    ui.horizontal(|ui| {
                        if ui.button("Import").clicked() {
                            match ron::from_str::<DialogueTree>(&self.import_buffer) {
                                Ok(dialogue) => {
                                    import_result = Some(Ok(dialogue));
                                    import_complete = true;
                                }
                                Err(e) => {
                                    import_result = Some(Err(format!("{:?}", e)));
                                    import_complete = true;
                                }
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            cancel_import = true;
                        }
                    });
                });
            self.show_import_dialog = show_dialog;
        }

        // Process import result outside window closure
        if cancel_import {
            self.show_import_dialog = false;
        }
        if import_complete {
            if let Some(result) = import_result {
                match result {
                    Ok(mut dialogue) => {
                        let new_id = self.next_available_dialogue_id();
                        dialogue.id = new_id;
                        self.dialogues.push(dialogue);
                        *dialogues = self.dialogues.clone();
                        *status_message = format!("Imported dialogue {}", new_id);
                        self.show_import_dialog = false;
                    }
                    Err(e) => {
                        *status_message = format!("Import failed: {}", e);
                    }
                }
            }
        }
    }

    /// Show dialogue list view
    fn show_dialogue_list(
        &mut self,
        ui: &mut egui::Ui,
        dialogues: &mut Vec<DialogueTree>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        // Clone data for closures
        let search_filter = self.search_filter.clone();
        let dialogues_snapshot: Vec<(usize, DialogueTree)> = self
            .dialogues
            .iter()
            .enumerate()
            .filter(|(_, d)| {
                search_filter.is_empty()
                    || d.name
                        .to_lowercase()
                        .contains(&search_filter.to_lowercase())
                    || d.id.to_string().contains(&search_filter)
            })
            .map(|(i, d)| (i, d.clone()))
            .collect();

        let selected_dialogue_idx = self.selected_dialogue;
        let show_preview = self.show_preview;

        // Track actions
        let mut new_selection: Option<usize> = None;
        let mut action_requested: Option<ItemAction> = None;

        TwoColumnLayout::new("dialogues").show_split(
            ui,
            |left_ui| {
                // Left panel: Dialogues list
                left_ui.heading("Dialogues");
                left_ui.separator();

                for (idx, dialogue) in &dialogues_snapshot {
                    let is_selected = selected_dialogue_idx == Some(*idx);

                    left_ui.group(|group_ui| {
                        group_ui.set_min_width(group_ui.available_width());

                        // Main dialogue name
                        let label = format!("{} ({})", dialogue.name, dialogue.nodes.len());
                        if group_ui.selectable_label(is_selected, &label).clicked() {
                            new_selection = Some(*idx);
                        }

                        // Show text excerpt from first node if available
                        if let Some((_, first_node)) = dialogue.nodes.iter().next() {
                            let excerpt: String = first_node.text.chars().take(60).collect();
                            let display_text = if first_node.text.len() > 60 {
                                format!("\"{}...\"", excerpt)
                            } else {
                                format!("\"{}\"", excerpt)
                            };
                            group_ui.label(egui::RichText::new(display_text).small().weak());
                        }

                        // Show speaker and quest info if available
                        group_ui.horizontal(|inner_ui| {
                            if let Some(speaker) = &dialogue.speaker_name {
                                inner_ui.label(
                                    egui::RichText::new(format!("👤 {}", speaker))
                                        .small()
                                        .color(egui::Color32::LIGHT_BLUE),
                                );
                            }
                            if let Some(quest_id) = dialogue.associated_quest {
                                inner_ui.label(
                                    egui::RichText::new(format!("📜 Quest #{}", quest_id))
                                        .small()
                                        .color(egui::Color32::GOLD),
                                );
                            }
                            if dialogue.repeatable {
                                inner_ui.label(
                                    egui::RichText::new("🔄")
                                        .small()
                                        .color(egui::Color32::GREEN),
                                );
                            }
                        });
                    });
                    left_ui.add_space(4.0);
                }
            },
            |right_ui| {
                // Right panel: Detail view
                if let Some(idx) = selected_dialogue_idx {
                    if let Some((_, dialogue)) = dialogues_snapshot.iter().find(|(i, _)| *i == idx)
                    {
                        right_ui.heading(&dialogue.name);
                        right_ui.separator();

                        // Action buttons using shared component
                        let action = ActionButtons::new().enabled(true).show(right_ui);
                        if action != ItemAction::None {
                            action_requested = Some(action);
                        }

                        right_ui.separator();

                        // Dialogue details
                        egui::Grid::new("dialogue_detail_grid")
                            .num_columns(2)
                            .spacing([10.0, 5.0])
                            .show(right_ui, |ui| {
                                ui.label("ID:");
                                ui.label(format!("{}", dialogue.id));
                                ui.end_row();

                                ui.label("Root Node:");
                                ui.label(format!("{}", dialogue.root_node));
                                ui.end_row();

                                ui.label("Nodes:");
                                ui.label(dialogue.nodes.len().to_string());
                                ui.end_row();

                                ui.label("Repeatable:");
                                ui.label(if dialogue.repeatable { "Yes" } else { "No" });
                                ui.end_row();

                                if let Some(speaker) = &dialogue.speaker_name {
                                    ui.label("Speaker:");
                                    ui.label(speaker);
                                    ui.end_row();
                                }

                                if let Some(quest_id) = dialogue.associated_quest {
                                    ui.label("Associated Quest:");
                                    ui.label(quest_id.to_string());
                                    ui.end_row();
                                }
                            });

                        // Node preview with better formatting
                        if show_preview && !dialogue.nodes.is_empty() {
                            right_ui.separator();
                            right_ui.heading("Dialogue Flow Preview");
                            right_ui.add_space(5.0);

                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .id_salt("dialogue_preview_scroll")
                                .show(right_ui, |ui| {
                                    for (node_id, node) in dialogue.nodes.iter().take(5) {
                                        ui.group(|ui| {
                                            ui.label(
                                                egui::RichText::new(format!("Node: {}", node_id))
                                                    .strong(),
                                            );

                                            // Show speaker override if present
                                            if let Some(speaker) = &node.speaker_override {
                                                ui.label(
                                                    egui::RichText::new(format!("👤 {}", speaker))
                                                        .small()
                                                        .color(egui::Color32::LIGHT_BLUE),
                                                );
                                            }

                                            // Show node text with wrapping
                                            let excerpt: String =
                                                node.text.chars().take(120).collect();
                                            let display_text = if node.text.len() > 120 {
                                                format!("\"{}...\"", excerpt)
                                            } else {
                                                format!("\"{}\"", node.text)
                                            };
                                            ui.label(display_text);

                                            // Show choice count
                                            if !node.choices.is_empty() {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "→ {} choices",
                                                        node.choices.len()
                                                    ))
                                                    .small()
                                                    .weak(),
                                                );
                                            }

                                            // Terminal node indicator
                                            if node.is_terminal {
                                                ui.label(
                                                    egui::RichText::new("🏁 Terminal")
                                                        .small()
                                                        .color(egui::Color32::from_rgb(
                                                            255, 100, 100,
                                                        )),
                                                );
                                            }
                                        });
                                        ui.add_space(4.0);
                                    }

                                    if dialogue.nodes.len() > 5 {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "... and {} more nodes",
                                                dialogue.nodes.len() - 5
                                            ))
                                            .weak(),
                                        );
                                    }
                                });
                        } else if !dialogue.nodes.is_empty() {
                            right_ui.separator();
                            right_ui.label(
                                egui::RichText::new("💡 Tip: Enable preview to see dialogue flow")
                                    .small()
                                    .weak(),
                            );
                        }
                    } else {
                        right_ui.label("Select a dialogue to view details");
                    }
                } else {
                    right_ui.label("Select a dialogue to view details");
                }
            },
        );

        // Apply selection change
        if let Some(idx) = new_selection {
            self.selected_dialogue = Some(idx);
        }

        // Handle action button clicks
        if let Some(action) = action_requested {
            match action {
                ItemAction::Edit => {
                    if let Some(idx) = self.selected_dialogue {
                        self.start_edit_dialogue(idx);
                    }
                }
                ItemAction::Delete => {
                    if let Some(idx) = self.selected_dialogue {
                        self.delete_dialogue(idx);
                        *dialogues = self.dialogues.clone();
                        *unsaved_changes = true;
                    }
                }
                ItemAction::Duplicate => {
                    if let Some(idx) = self.selected_dialogue {
                        if let Some(dialogue) = self.dialogues.get(idx) {
                            let mut new_dialogue = dialogue.clone();
                            new_dialogue.id = self.next_available_dialogue_id();
                            new_dialogue.name = format!("{} (Copy)", new_dialogue.name);
                            self.dialogues.push(new_dialogue);
                            *dialogues = self.dialogues.clone();
                            *unsaved_changes = true;
                            *status_message = "Dialogue duplicated".to_string();
                        }
                    }
                }
                ItemAction::Export => {
                    if let Some(idx) = self.selected_dialogue {
                        if let Some(dialogue) = self.dialogues.get(idx) {
                            match ron::ser::to_string_pretty(dialogue, Default::default()) {
                                Ok(contents) => {
                                    ui.ctx().copy_text(contents);
                                    *status_message = "Copied dialogue to clipboard".to_string();
                                }
                                Err(e) => {
                                    *status_message =
                                        format!("Failed to serialize dialogue: {}", e);
                                }
                            }
                        }
                    }
                }
                ItemAction::None => {}
            }
        }
    }

    /// Show dialogue form editor
    fn show_dialogue_form(
        &mut self,
        ui: &mut egui::Ui,
        dialogues: &mut Vec<DialogueTree>,
        status_message: &mut String,
    ) {
        ui.horizontal(|ui| {
            if ui.button("⬅ Back to List").clicked() {
                self.cancel_edit();
            }

            if ui.button("💾 Save Dialogue").clicked() {
                match self.save_dialogue() {
                    Ok(()) => {
                        *dialogues = self.dialogues.clone();
                        *status_message = "Dialogue saved".to_string();
                    }
                    Err(e) => {
                        *status_message = format!("Save failed: {}", e);
                    }
                }
            }
        });

        ui.separator();

        // Dialogue form
        egui::Grid::new("dialogue_form_grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Dialogue ID:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.dialogue_buffer.id).desired_width(200.0),
                );
                ui.end_row();

                ui.label("Name:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.dialogue_buffer.name).desired_width(300.0),
                );
                ui.end_row();

                ui.label("Speaker Name:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.dialogue_buffer.speaker_name)
                        .desired_width(200.0),
                );
                ui.end_row();

                ui.label("Repeatable:");
                ui.checkbox(&mut self.dialogue_buffer.repeatable, "");
                ui.end_row();

                ui.label("Associated Quest:");
                autocomplete_quest_selector(
                    ui,
                    "dialogue_associated_quest",
                    "",
                    &mut self.dialogue_buffer.associated_quest,
                    &self.quests,
                );
                ui.end_row();
            });

        ui.separator();

        // Node tree editor
        if let Some(dialogue_idx) = self.selected_dialogue {
            if dialogue_idx < self.dialogues.len() {
                ui.heading("Dialogue Nodes");
                self.show_dialogue_nodes_editor(ui, dialogue_idx, status_message);
            }
        } else {
            ui.label("Save dialogue to add nodes");
        }
    }

    /// Show dialogue node tree editor with Phase 3 enhancements:
    /// - Visual node hierarchy with indented choices
    /// - Node navigation helpers (search, jump-to, show-root)
    /// - Inline validation feedback
    /// - Unreachable node highlighting
    /// - Reachability statistics
    fn show_dialogue_nodes_editor(
        &mut self,
        ui: &mut egui::Ui,
        dialogue_idx: usize,
        status_message: &mut String,
    ) {
        // Add node form - only show when not editing a node
        if !self.editing_node {
            let dialogue_name = self.dialogues[dialogue_idx].name.clone();

            ui.group(|ui| {
                ui.label(format!("➕ Adding node to: \"{}\"", dialogue_name));

                if let Some(next_id) = self.next_available_node_id() {
                    ui.label(format!("Next Node ID: {}", next_id));
                }

                ui.separator();

                ui.label("Node Text:");
                ui.text_edit_multiline(&mut self.node_buffer.text);

                ui.label("Speaker Override (optional):");
                ui.add(
                    egui::TextEdit::singleline(&mut self.node_buffer.speaker_override)
                        .hint_text("Leave empty to use dialogue speaker"),
                );

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.node_buffer.is_terminal, "Terminal Node");

                    if ui.button("✓ Add Node").clicked() {
                        match self.add_node() {
                            Ok(node_id) => {
                                *status_message =
                                    format!("✓ Node {} created successfully", node_id);
                            }
                            Err(e) => {
                                *status_message = format!("✗ Failed to add node: {}", e);
                            }
                        }
                    }
                });
            });

            ui.separator();
        }

        // Phase 3: Dialogue Header with Reachability Stats
        let dialogue = self.dialogues[dialogue_idx].clone();
        let (total_nodes, reachable_nodes, unreachable_count) =
            self.get_reachability_stats(dialogue_idx);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "📊 Nodes: {} total | {} reachable",
                    total_nodes, reachable_nodes
                ))
                .strong(),
            );
            if unreachable_count > 0 {
                ui.label(
                    egui::RichText::new(format!("⚠️ {} unreachable", unreachable_count))
                        .color(egui::Color32::from_rgb(255, 165, 0)),
                );
            }
        });

        // Phase 3: Navigation Controls
        ui.horizontal(|ui| {
            ui.label("Find Node:");
            let response = ui.text_edit_singleline(&mut self.node_search_filter);

            if !self.node_search_filter.is_empty() {
                let matches = self.search_nodes(dialogue_idx, &self.node_search_filter);
                if !matches.is_empty() {
                    if ui.button(format!("→ Go to Node {}", matches[0])).clicked() {
                        self.jump_to_node = Some(matches[0]);
                    }
                    if matches.len() > 1 {
                        ui.label(format!("({} matches)", matches.len()));
                    }
                } else {
                    ui.label("No matches");
                }
            }

            if ui.button("🏠 Root").clicked() {
                self.jump_to_node = Some(dialogue.root_node);
            }

            if ui.button("✓ Validate").clicked() {
                let errors = self.validate_dialogue_tree(dialogue_idx);
                if errors.is_empty() {
                    *status_message = "✓ Dialogue tree is valid".to_string();
                } else {
                    *status_message = format!("✗ {} validation errors found", errors.len());
                }
            }
        });

        // Phase 3: Show Validation Errors
        if !self.dialogue_validation_errors.is_empty() {
            ui.separator();
            ui.colored_label(
                egui::Color32::from_rgb(255, 100, 100),
                "⚠️ Validation Errors:",
            );
            for error in &self.dialogue_validation_errors {
                ui.label(egui::RichText::new(error).color(egui::Color32::from_rgb(255, 100, 100)));
            }
            ui.separator();
        }

        // Clone dialogue data to avoid borrow conflicts
        let unreachable = self.unreachable_nodes.clone();
        let mut select_node_for_choice: Option<NodeId> = None;
        let mut edit_node_id: Option<NodeId> = None;
        let mut delete_node_id: Option<NodeId> = None;

        // Phase 3: Display nodes with hierarchy and enhanced navigation
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                for (node_id, node) in &dialogue.nodes {
                    // Phase 3: Highlight unreachable nodes
                    let is_unreachable = unreachable.contains(node_id);
                    let bg_color = if is_unreachable {
                        egui::Color32::from_rgba_unmultiplied(255, 165, 0, 20)
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    ui.group(|ui| {
                        ui.visuals_mut().override_text_color = if is_unreachable {
                            Some(egui::Color32::from_rgb(255, 140, 0))
                        } else {
                            None
                        };

                        ui.horizontal(|ui| {
                            // Node label with unreachable warning
                            let node_label = if is_unreachable {
                                format!("⚠️ Node {}", node_id)
                            } else {
                                format!("Node {}", node_id)
                            };

                            ui.strong(node_label);
                            if *node_id == dialogue.root_node {
                                ui.label("(ROOT)");
                            }
                            if node.is_terminal {
                                ui.label("(TERMINAL)");
                            }

                            // Add Edit/Delete buttons for each node
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    // Don't show delete button for root node
                                    let action = if *node_id == dialogue.root_node {
                                        ActionButtons::new()
                                            .with_edit(true)
                                            .with_delete(false)
                                            .with_duplicate(false)
                                            .with_export(false)
                                            .show(ui)
                                    } else {
                                        ActionButtons::new()
                                            .with_edit(true)
                                            .with_delete(true)
                                            .with_duplicate(false)
                                            .with_export(false)
                                            .show(ui)
                                    };

                                    match action {
                                        ItemAction::Edit => {
                                            edit_node_id = Some(*node_id);
                                        }
                                        ItemAction::Delete => {
                                            delete_node_id = Some(*node_id);
                                        }
                                        _ => {}
                                    }
                                },
                            );
                        });

                        ui.label(&node.text);

                        if let Some(speaker) = &node.speaker_override {
                            ui.label(format!("Speaker: {}", speaker));
                        }

                        // Phase 3: Show choices with visual hierarchy and navigation
                        if !node.choices.is_empty() {
                            ui.separator();
                            ui.label("Choices:");
                            for (choice_idx, choice) in node.choices.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.label(format!("  {}. {}", choice_idx + 1, choice.text));

                                    if let Some(target) = choice.target_node {
                                        // Phase 3: Check if target is valid and show error if not
                                        if !self.is_choice_target_valid(dialogue_idx, target) {
                                            ui.label(
                                                egui::RichText::new(format!("❌ Node {}", target))
                                                    .color(egui::Color32::from_rgb(255, 0, 0)),
                                            );
                                        } else {
                                            // Phase 3: Show target with preview and jump button
                                            let preview =
                                                self.get_node_preview(dialogue_idx, target);
                                            ui.label(format!("→ Node {}: {}", target, preview));

                                            if ui.button("→").clicked() {
                                                self.jump_to_node = Some(target);
                                            }
                                        }
                                    }

                                    if choice.ends_dialogue {
                                        ui.label("(Ends)");
                                    }
                                });
                            }
                        }

                        // Show conditions
                        if !node.conditions.is_empty() {
                            ui.separator();
                            ui.label(format!("Conditions: {}", node.conditions.len()));
                        }

                        // Show actions
                        if !node.actions.is_empty() {
                            ui.separator();
                            ui.label(format!("Actions: {}", node.actions.len()));
                        }

                        // Add choice to this node
                        if ui.button("➕ Add Choice").clicked() {
                            select_node_for_choice = Some(*node_id);
                        }
                    });
                }
            });

        // Process node actions outside scroll area
        if let Some(node_id) = edit_node_id {
            match self.edit_node(dialogue_idx, node_id) {
                Ok(()) => {
                    *status_message = format!("Editing Node {}", node_id);
                }
                Err(e) => {
                    *status_message = format!("Failed to edit node: {}", e);
                }
            }
        }

        if let Some(node_id) = delete_node_id {
            match self.delete_node(dialogue_idx, node_id) {
                Ok(()) => {
                    *status_message = format!("Node {} deleted", node_id);
                }
                Err(e) => {
                    *status_message = format!("Failed to delete node: {}", e);
                }
            }
        }

        if let Some(node_id) = select_node_for_choice {
            self.selected_node = Some(node_id);
            self.editing_node = false;
        }

        // Clear jump-to after processing
        self.jump_to_node = None;

        // Node editor panel - show when editing a node
        self.show_node_editor_panel(ui, dialogue_idx, status_message);

        // Choice editor panel - only show when not editing a node
        if !self.editing_node {
            self.show_choice_editor_panel(ui, status_message);
        }
    }

    fn show_node_editor_panel(
        &mut self,
        ui: &mut egui::Ui,
        dialogue_idx: usize,
        status_message: &mut String,
    ) {
        let mut save_node_clicked = false;
        let mut cancel_node_clicked = false;

        if self.editing_node {
            if let Some(selected_node_id) = self.selected_node {
                ui.separator();
                ui.heading(format!("Edit Node {}", selected_node_id));

                ui.horizontal(|ui| {
                    ui.label("Node Text:");
                });
                ui.add(
                    egui::TextEdit::multiline(&mut self.node_buffer.text)
                        .desired_width(ui.available_width())
                        .desired_rows(3),
                );

                ui.horizontal(|ui| {
                    ui.label("Speaker Override:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.node_buffer.speaker_override)
                            .hint_text("Leave empty to use dialogue speaker")
                            .desired_width(250.0),
                    );
                });

                ui.checkbox(&mut self.node_buffer.is_terminal, "Terminal Node");

                ui.horizontal(|ui| {
                    if ui.button("✓ Save").clicked() {
                        save_node_clicked = true;
                    }
                    if ui.button("✗ Cancel").clicked() {
                        cancel_node_clicked = true;
                    }
                });
            }
        }

        // Process node actions outside the panel
        if save_node_clicked {
            if let Some(node_id) = self.selected_node {
                match self.save_node(dialogue_idx, node_id) {
                    Ok(()) => {
                        *status_message = format!("Node {} saved", node_id);
                    }
                    Err(e) => {
                        *status_message = format!("Failed to save node: {}", e);
                    }
                }
            }
        }

        if cancel_node_clicked {
            self.selected_node = None;
            self.editing_node = false;
            self.node_buffer = NodeEditBuffer::default();
            *status_message = "Node editing cancelled".to_string();
        }
    }

    /// Show the choice editor panel
    fn show_choice_editor_panel(&mut self, ui: &mut egui::Ui, status_message: &mut String) {
        let mut add_choice_clicked = false;
        let mut cancel_choice_clicked = false;

        if let Some(selected_node_id) = self.selected_node {
            ui.separator();
            ui.heading(format!("Add Choice to Node {}", selected_node_id));

            ui.horizontal(|ui| {
                ui.label("Choice Text:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.choice_buffer.text).desired_width(250.0),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Target Node ID:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.choice_buffer.target_node)
                        .desired_width(80.0),
                );
                ui.checkbox(&mut self.choice_buffer.ends_dialogue, "Ends Dialogue");
            });

            ui.horizontal(|ui| {
                if ui.button("✓ Add Choice").clicked() {
                    add_choice_clicked = true;
                }
                if ui.button("✗ Cancel").clicked() {
                    cancel_choice_clicked = true;
                }
            });
        }

        // Process choice actions outside choice panel
        if add_choice_clicked {
            match self.add_choice() {
                Ok(()) => {
                    *status_message = "Choice added".to_string();
                }
                Err(e) => {
                    *status_message = format!("Failed to add choice: {}", e);
                }
            }
        }
        if cancel_choice_clicked {
            self.selected_node = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialogue_editor_state_creation() {
        let editor = DialogueEditorState::new();
        assert_eq!(editor.dialogues.len(), 0);
        assert_eq!(editor.mode, DialogueEditorMode::List);
        assert!(!editor.show_preview);
        assert!(!editor.show_import_dialog);
        assert!(editor.import_buffer.is_empty());
    }

    #[test]
    fn test_next_available_dialogue_id() {
        let mut editor = DialogueEditorState::new();

        // Empty list
        assert_eq!(editor.next_available_dialogue_id(), 1);

        // Add dialogues
        editor.start_new_dialogue();
        editor.dialogue_buffer.id = "1".to_string();
        editor.dialogue_buffer.name = "Dialogue 1".to_string();
        editor.save_dialogue().unwrap();

        editor.start_new_dialogue();
        editor.dialogue_buffer.id = "5".to_string();
        editor.dialogue_buffer.name = "Dialogue 5".to_string();
        editor.save_dialogue().unwrap();

        assert_eq!(editor.next_available_dialogue_id(), 6);
    }

    #[test]
    fn test_start_new_dialogue() {
        let mut editor = DialogueEditorState::new();
        editor.start_new_dialogue();
        assert_eq!(editor.mode, DialogueEditorMode::Creating);
    }

    #[test]
    fn test_save_dialogue_creates_new() {
        let mut editor = DialogueEditorState::new();
        editor.start_new_dialogue();
        editor.dialogue_buffer.id = "1".to_string();
        editor.dialogue_buffer.name = "Test Dialogue".to_string();
        editor.dialogue_buffer.speaker_name = "Merchant".to_string();

        assert!(editor.save_dialogue().is_ok());
        assert_eq!(editor.dialogues.len(), 1);
        assert_eq!(editor.dialogues[0].name, "Test Dialogue");
    }

    #[test]
    fn test_delete_dialogue() {
        let mut editor = DialogueEditorState::new();
        editor.start_new_dialogue();
        editor.dialogue_buffer.id = "1".to_string();
        editor.dialogue_buffer.name = "Test Dialogue".to_string();
        editor.save_dialogue().unwrap();

        assert_eq!(editor.dialogues.len(), 1);
        editor.delete_dialogue(0);
        assert_eq!(editor.dialogues.len(), 0);
    }

    #[test]
    fn test_filtered_dialogues() {
        let mut editor = DialogueEditorState::new();
        editor.start_new_dialogue();
        editor.dialogue_buffer.id = "1".to_string();
        editor.dialogue_buffer.name = "Merchant Greeting".to_string();
        editor.save_dialogue().unwrap();

        editor.start_new_dialogue();
        editor.dialogue_buffer.id = "2".to_string();
        editor.dialogue_buffer.name = "Guard Warning".to_string();
        editor.save_dialogue().unwrap();

        editor.search_filter = "merchant".to_string();
        let filtered = editor.filtered_dialogues();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.name, "Merchant Greeting");
    }

    #[test]
    fn test_condition_type_display() {
        assert_eq!(ConditionType::HasQuest.as_str(), "Has Quest");
        assert_eq!(ConditionType::CompletedQuest.as_str(), "Completed Quest");
        assert_eq!(ConditionType::HasItem.as_str(), "Has Item");
    }

    #[test]
    fn test_action_type_display() {
        assert_eq!(ActionType::StartQuest.as_str(), "Start Quest");
        assert_eq!(ActionType::GiveGold.as_str(), "Give Gold");
        assert_eq!(ActionType::SetFlag.as_str(), "Set Flag");
    }

    #[test]
    fn test_build_has_quest_condition() {
        let mut editor = DialogueEditorState::new();
        editor.condition_buffer.condition_type = ConditionType::HasQuest;
        editor.condition_buffer.quest_id = "5".to_string();

        let condition = editor.build_condition_from_buffer();
        assert!(condition.is_ok());
    }

    #[test]
    fn test_build_start_quest_action() {
        let mut editor = DialogueEditorState::new();
        editor.action_buffer.action_type = ActionType::StartQuest;
        editor.action_buffer.quest_id = "3".to_string();

        let action = editor.build_action_from_buffer();
        assert!(action.is_ok());
    }

    #[test]
    fn test_validation_empty_dialogue() {
        let mut editor = DialogueEditorState::new();
        editor.start_new_dialogue();
        editor.dialogue_buffer.id = "1".to_string();
        editor.dialogue_buffer.name = "Test".to_string();
        editor.save_dialogue().unwrap();

        editor.selected_dialogue = Some(0);
        editor.validate_current_dialogue();
        assert!(!editor.validation_errors.is_empty());
    }

    #[test]
    fn test_condition_buffer_defaults() {
        let buffer = ConditionEditBuffer::default();
        assert_eq!(buffer.condition_type, ConditionType::HasQuest);
        assert_eq!(buffer.stage_number, "1");
    }

    #[test]
    fn test_action_buffer_defaults() {
        let buffer = ActionEditBuffer::default();
        assert_eq!(buffer.action_type, ActionType::StartQuest);
        assert_eq!(buffer.experience_amount, "100");
    }

    #[test]
    fn test_dialogue_preview() {
        let mut editor = DialogueEditorState::new();
        editor.start_new_dialogue();
        editor.dialogue_buffer.id = "1".to_string();
        editor.dialogue_buffer.name = "Test Dialogue".to_string();
        editor.dialogue_buffer.speaker_name = "NPC".to_string();
        editor.save_dialogue().unwrap();

        let preview = editor.get_dialogue_preview(0);
        assert!(preview.contains("Test Dialogue"));
        assert!(preview.contains("NPC"));
    }

    #[test]
    fn test_edit_node() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        let mut node = DialogueNode::new(1, "Hello, traveler!");
        node.is_terminal = false;
        dialogue.add_node(node);
        editor.dialogues.push(dialogue);

        // Edit the node
        assert!(editor.edit_node(0, 1).is_ok());
        assert_eq!(editor.node_buffer.id, "1");
        assert_eq!(editor.node_buffer.text, "Hello, traveler!");
        assert_eq!(editor.selected_node, Some(1));
    }

    #[test]
    fn test_delete_node() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        dialogue.add_node(DialogueNode::new(1, "Root node"));
        dialogue.add_node(DialogueNode::new(2, "Another node"));
        editor.dialogues.push(dialogue);

        assert_eq!(editor.dialogues[0].nodes.len(), 2);

        // Cannot delete root node
        assert!(editor.delete_node(0, 1).is_err());

        // Can delete non-root node
        assert!(editor.delete_node(0, 2).is_ok());
        assert_eq!(editor.dialogues[0].nodes.len(), 1);
    }

    #[test]
    fn test_edit_choice() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        let mut node = DialogueNode::new(1, "Hello!");
        node.add_choice(DialogueChoice::new("Goodbye", Some(2)));
        dialogue.add_node(node);
        editor.dialogues.push(dialogue);

        // Edit the choice
        assert!(editor.edit_choice(0, 1, 0).is_ok());
        assert_eq!(editor.choice_buffer.text, "Goodbye");
        assert_eq!(editor.choice_buffer.target_node, "2");
        assert_eq!(editor.selected_choice, Some(0));
    }

    #[test]
    fn test_delete_choice() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        let mut node = DialogueNode::new(1, "Hello!");
        node.add_choice(DialogueChoice::new("Option 1", Some(2)));
        node.add_choice(DialogueChoice::new("Option 2", Some(3)));
        dialogue.add_node(node);
        editor.dialogues.push(dialogue);

        assert_eq!(editor.dialogues[0].nodes.get(&1).unwrap().choices.len(), 2);

        // Delete first choice
        assert!(editor.delete_choice(0, 1, 0).is_ok());
        assert_eq!(editor.dialogues[0].nodes.get(&1).unwrap().choices.len(), 1);
        assert_eq!(
            editor.dialogues[0].nodes.get(&1).unwrap().choices[0].text,
            "Option 2"
        );
    }

    #[test]
    fn test_find_unreachable_nodes() {
        let mut editor = DialogueEditorState::new();

        // Create dialogue with reachable nodes
        let mut dialogue1 = DialogueTree::new(1, "Reachable Dialogue", 1);
        let mut node1 = DialogueNode::new(1, "Root");
        node1.add_choice(DialogueChoice::new("Go to 2", Some(2)));
        dialogue1.add_node(node1);
        dialogue1.add_node(DialogueNode::new(2, "Node 2"));
        editor.dialogues.push(dialogue1);

        // Create dialogue with unreachable node
        let mut dialogue2 = DialogueTree::new(2, "Unreachable Dialogue", 1);
        dialogue2.add_node(DialogueNode::new(1, "Root"));
        dialogue2.add_node(DialogueNode::new(2, "Unreachable node"));
        dialogue2.add_node(DialogueNode::new(3, "Another unreachable"));
        editor.dialogues.push(dialogue2);

        let unreachable = editor.find_unreachable_nodes();
        assert_eq!(unreachable.len(), 1);
        assert_eq!(unreachable[0].0, 2);
        assert_eq!(unreachable[0].1.len(), 2);
        assert!(unreachable[0].1.contains(&2));
        assert!(unreachable[0].1.contains(&3));
    }

    #[test]
    fn test_save_edited_node() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        dialogue.add_node(DialogueNode::new(1, "Original text"));
        editor.dialogues.push(dialogue);

        // Edit and save the node
        editor.edit_node(0, 1).unwrap();
        editor.node_buffer.text = "Modified text".to_string();
        editor.node_buffer.speaker_override = "NewSpeaker".to_string();
        editor.node_buffer.is_terminal = true;

        assert!(editor.save_node(0, 1).is_ok());

        // Verify changes
        let node = editor.dialogues[0].nodes.get(&1).unwrap();
        assert_eq!(node.text, "Modified text");
        assert_eq!(node.speaker_override, Some("NewSpeaker".to_string()));
        assert!(node.is_terminal);
    }

    #[test]
    fn test_save_edited_choice() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        let mut node = DialogueNode::new(1, "Hello!");
        node.add_choice(DialogueChoice::new("Original", Some(2)));
        dialogue.add_node(node);
        editor.dialogues.push(dialogue);

        // Edit and save the choice
        editor.edit_choice(0, 1, 0).unwrap();
        editor.choice_buffer.text = "Modified choice".to_string();
        editor.choice_buffer.target_node = "5".to_string();
        editor.choice_buffer.ends_dialogue = true;

        assert!(editor.save_choice(0, 1, 0).is_ok());

        // Verify changes
        let choice = &editor.dialogues[0].nodes.get(&1).unwrap().choices[0];
        assert_eq!(choice.text, "Modified choice");
        assert_eq!(choice.target_node, Some(5));
        assert!(choice.ends_dialogue);
    }

    #[test]
    fn test_dialogue_editor_loads_quests_and_items() {
        use antares::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
        use antares::domain::quest::Quest;

        let mut editor = DialogueEditorState::new();

        // Create a quest using the up-to-date Quest API
        let mut quest = Quest::new(1, "Save the Village", "Help save the village");
        quest.min_level = Some(1);
        quest.repeatable = false;
        quest.is_main_quest = true;

        let items = vec![Item {
            id: 42,
            name: "Healing Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(50),
                is_combat_usable: true,
            }),
            base_cost: 50,
            sell_cost: 25,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec!["healing".to_string()],
        }];

        editor.quests = vec![quest.clone()];
        editor.items = items.clone();

        assert_eq!(editor.quests.len(), 1);
        assert_eq!(editor.items.len(), 1);
        assert_eq!(editor.quests[0].name, "Save the Village");
        assert_eq!(editor.items[0].name, "Healing Potion");
    }

    #[test]
    fn test_dialogue_buffer_with_quest_reference() {
        let mut editor = DialogueEditorState::new();

        let mut quest = Quest::new(5, "Find the Artifact", "Locate the ancient artifact");
        quest.min_level = Some(3);
        quest.repeatable = false;
        quest.is_main_quest = false;
        quest.quest_giver_npc = None;
        quest.quest_giver_map = None;
        quest.quest_giver_position = None;

        editor.quests = vec![quest];
        editor.dialogue_buffer.associated_quest = "5".to_string();

        // Verify buffer can store quest ID as string
        assert_eq!(editor.dialogue_buffer.associated_quest, "5");

        // Verify we can find the quest by ID
        let quest_id: QuestId = editor.dialogue_buffer.associated_quest.parse().unwrap();
        let found_quest = editor.quests.iter().find(|q| q.id == quest_id);
        assert!(found_quest.is_some());
        assert_eq!(found_quest.unwrap().name, "Find the Artifact");
    }

    #[test]
    fn test_condition_buffer_with_quest_and_item_references() {
        let mut editor = DialogueEditorState::new();

        editor.condition_buffer.quest_id = "10".to_string();
        editor.condition_buffer.item_id = "25".to_string();

        // Verify buffers store IDs as strings
        assert_eq!(editor.condition_buffer.quest_id, "10");
        assert_eq!(editor.condition_buffer.item_id, "25");

        // Verify parsing works
        let quest_id: Result<QuestId, _> = editor.condition_buffer.quest_id.parse();
        let item_id: Result<ItemId, _> = editor.condition_buffer.item_id.parse();
        assert!(quest_id.is_ok());
        assert!(item_id.is_ok());
        assert_eq!(quest_id.unwrap(), 10);
        assert_eq!(item_id.unwrap(), 25);
    }

    #[test]
    fn test_action_buffer_with_quest_and_item_references() {
        let mut editor = DialogueEditorState::new();

        editor.action_buffer.quest_id = "3".to_string();
        editor.action_buffer.unlock_quest_id = "7".to_string();
        editor.action_buffer.item_id = "15".to_string();

        // Verify buffers store IDs as strings
        assert_eq!(editor.action_buffer.quest_id, "3");
        assert_eq!(editor.action_buffer.unlock_quest_id, "7");
        assert_eq!(editor.action_buffer.item_id, "15");

        // Verify parsing works
        let quest_id: Result<QuestId, _> = editor.action_buffer.quest_id.parse();
        let unlock_quest_id: Result<QuestId, _> = editor.action_buffer.unlock_quest_id.parse();
        let item_id: Result<ItemId, _> = editor.action_buffer.item_id.parse();
        assert!(quest_id.is_ok());
        assert!(unlock_quest_id.is_ok());
        assert!(item_id.is_ok());
        assert_eq!(quest_id.unwrap(), 3);
        assert_eq!(unlock_quest_id.unwrap(), 7);
        assert_eq!(item_id.unwrap(), 15);
    }

    #[test]
    fn test_next_available_node_id() {
        let mut editor = DialogueEditorState::new();

        // No dialogue selected - should return None
        assert_eq!(editor.next_available_node_id(), None);

        // Create a new dialogue
        editor.start_new_dialogue();
        editor.dialogue_buffer.id = "1".to_string();
        editor.dialogue_buffer.name = "Test Dialogue".to_string();
        editor.dialogue_buffer.speaker_name = "NPC".to_string();
        editor.save_dialogue().unwrap();
        editor.selected_dialogue = Some(0);

        // First available node ID should be 1 (no nodes yet)
        assert_eq!(editor.next_available_node_id(), Some(1));

        // Add a node with ID 1
        editor.node_buffer.text = "Hello!".to_string();
        editor.add_node().unwrap();

        // Next available should be 2
        assert_eq!(editor.next_available_node_id(), Some(2));

        // Add another node
        editor.node_buffer.text = "Goodbye!".to_string();
        editor.add_node().unwrap();

        // Next available should be 3
        assert_eq!(editor.next_available_node_id(), Some(3));
    }

    #[test]
    fn test_next_available_node_id_no_dialogue_selected() {
        let editor = DialogueEditorState::new();
        assert_eq!(editor.next_available_node_id(), None);
    }

    #[test]
    fn test_add_node_with_auto_generated_id() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        dialogue.add_node(DialogueNode::new(1, "Root node"));
        editor.dialogues.push(dialogue);
        editor.selected_dialogue = Some(0);

        // Set node text
        editor.node_buffer.text = "New node text".to_string();

        // Add node - should auto-generate ID as 2
        let result = editor.add_node();
        assert!(result.is_ok());
        let node_id = result.unwrap();
        assert_eq!(node_id, 2);

        // Verify node was actually added with correct ID
        let node = editor.dialogues[0].nodes.get(&2);
        assert!(node.is_some());
        assert_eq!(node.unwrap().text, "New node text");
    }

    #[test]
    fn test_add_node_empty_text_validation() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        dialogue.add_node(DialogueNode::new(1, "Root node"));
        editor.dialogues.push(dialogue);
        editor.selected_dialogue = Some(0);

        // Try to add node with empty text
        editor.node_buffer.text = "".to_string();
        let result = editor.add_node();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));

        // Try with only whitespace
        editor.node_buffer.text = "   ".to_string();
        let result = editor.add_node();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[test]
    fn test_add_node_with_speaker_override() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        dialogue.add_node(DialogueNode::new(1, "Root node"));
        editor.dialogues.push(dialogue);
        editor.selected_dialogue = Some(0);

        // Add node with speaker override
        editor.node_buffer.text = "Custom speaker text".to_string();
        editor.node_buffer.speaker_override = "Custom Speaker".to_string();

        let result = editor.add_node();
        assert!(result.is_ok());
        let node_id = result.unwrap();

        // Verify node has speaker override set
        let node = editor.dialogues[0].nodes.get(&node_id);
        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(node.text, "Custom speaker text");
        assert_eq!(node.speaker_override, Some("Custom Speaker".to_string()));
    }

    #[test]
    fn test_add_node_terminal_flag() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        dialogue.add_node(DialogueNode::new(1, "Root node"));
        editor.dialogues.push(dialogue);
        editor.selected_dialogue = Some(0);

        // Add node marked as terminal
        editor.node_buffer.text = "Farewell!".to_string();
        editor.node_buffer.is_terminal = true;

        let result = editor.add_node();
        assert!(result.is_ok());
        let node_id = result.unwrap();

        // Verify terminal flag is set
        let node = editor.dialogues[0].nodes.get(&node_id);
        assert!(node.is_some());
        assert!(node.unwrap().is_terminal);
    }

    #[test]
    fn test_add_node_clears_buffer_on_success() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        dialogue.add_node(DialogueNode::new(1, "Root node"));
        editor.dialogues.push(dialogue);
        editor.selected_dialogue = Some(0);

        // Fill buffer
        editor.node_buffer.text = "Some text".to_string();
        editor.node_buffer.speaker_override = "Speaker".to_string();
        editor.node_buffer.is_terminal = true;

        // Add node
        assert!(editor.add_node().is_ok());

        // Verify buffer was cleared
        assert_eq!(editor.node_buffer.text, "");
        assert_eq!(editor.node_buffer.speaker_override, "");
        assert!(!editor.node_buffer.is_terminal);
    }

    #[test]
    fn test_add_node_no_dialogue_selected() {
        let mut editor = DialogueEditorState::new();
        editor.node_buffer.text = "Some text".to_string();

        // Try to add node without selecting dialogue
        let result = editor.add_node();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No dialogue selected"));
    }

    // ========== Phase 3 Tests: Node Navigation and Validation ==========

    #[test]
    fn test_get_unreachable_nodes_for_dialogue() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        // Create nodes: 1 -> 2, 3 is orphaned
        dialogue.add_node(DialogueNode::new(1, "Root node"));
        let mut node2 = DialogueNode::new(2, "Node 2");
        let node3 = DialogueNode::new(3, "Orphaned node");
        node2.add_choice(DialogueChoice::new("Go to 2", Some(2)));
        dialogue.add_node(node2);
        dialogue.add_node(node3);

        editor.dialogues.push(dialogue);

        // Node 3 should be unreachable
        let unreachable = editor.get_unreachable_nodes_for_dialogue(0);
        assert_eq!(unreachable.len(), 1);
        assert!(unreachable.contains(&3));
    }

    #[test]
    fn test_get_unreachable_nodes_all_reachable() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        // Create nodes: 1 -> 2 -> 3 (all reachable)
        dialogue.add_node(DialogueNode::new(1, "Root node"));
        let mut node2 = DialogueNode::new(2, "Node 2");
        let node3 = DialogueNode::new(3, "Node 3");
        node2.add_choice(DialogueChoice::new("Go to 3", Some(3)));
        dialogue.add_node(node2);
        dialogue.add_node(node3);

        let mut node1 = dialogue.nodes.remove(&1).unwrap();
        node1.add_choice(DialogueChoice::new("Go to 2", Some(2)));
        dialogue.nodes.insert(1, node1);

        editor.dialogues.push(dialogue);

        // All nodes should be reachable
        let unreachable = editor.get_unreachable_nodes_for_dialogue(0);
        assert_eq!(unreachable.len(), 0);
    }

    #[test]
    fn test_validate_dialogue_tree_valid() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        dialogue.add_node(DialogueNode::new(1, "Root node"));
        let mut node2 = DialogueNode::new(2, "Node 2");
        node2.add_choice(DialogueChoice::new("Back", Some(1)));
        dialogue.add_node(node2);

        let mut node1 = dialogue.nodes.remove(&1).unwrap();
        node1.add_choice(DialogueChoice::new("Forward", Some(2)));
        dialogue.nodes.insert(1, node1);

        editor.dialogues.push(dialogue);

        // Validation should pass
        let errors = editor.validate_dialogue_tree(0);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_dialogue_tree_missing_root() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        // Don't add root node 1, only add node 2

        dialogue.add_node(DialogueNode::new(2, "Node 2"));
        editor.dialogues.push(dialogue);

        // Validation should fail
        let errors = editor.validate_dialogue_tree(0);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("Root node"));
    }

    #[test]
    fn test_validate_dialogue_tree_broken_target() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        dialogue.add_node(DialogueNode::new(1, "Root node"));
        let mut node2 = DialogueNode::new(2, "Node 2");
        node2.add_choice(DialogueChoice::new("Broken", Some(99))); // Target doesn't exist
        dialogue.add_node(node2);

        let mut node1 = dialogue.nodes.remove(&1).unwrap();
        node1.add_choice(DialogueChoice::new("Forward", Some(2)));
        dialogue.nodes.insert(1, node1);

        editor.dialogues.push(dialogue);

        // Validation should fail
        let errors = editor.validate_dialogue_tree(0);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("non-existent")));
    }

    #[test]
    fn test_validate_dialogue_tree_unreachable_nodes() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        dialogue.add_node(DialogueNode::new(1, "Root"));
        dialogue.add_node(DialogueNode::new(2, "Unreachable"));
        editor.dialogues.push(dialogue);

        // Validation should report unreachable nodes
        let errors = editor.validate_dialogue_tree(0);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("Unreachable")));
    }

    #[test]
    fn test_get_reachability_stats() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        dialogue.add_node(DialogueNode::new(1, "Root"));
        dialogue.add_node(DialogueNode::new(2, "Reachable"));
        dialogue.add_node(DialogueNode::new(3, "Unreachable"));

        let mut node1 = dialogue.nodes.remove(&1).unwrap();
        node1.add_choice(DialogueChoice::new("Go to 2", Some(2)));
        dialogue.nodes.insert(1, node1);

        editor.dialogues.push(dialogue);

        let (total, reachable, unreachable) = editor.get_reachability_stats(0);
        assert_eq!(total, 3);
        assert_eq!(reachable, 2);
        assert_eq!(unreachable, 1);
    }

    #[test]
    fn test_get_node_preview() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        dialogue.add_node(DialogueNode::new(
            1,
            "This is a very long node text that should be truncated",
        ));
        editor.dialogues.push(dialogue);

        let preview = editor.get_node_preview(0, 1);
        assert_eq!(preview.len(), 51); // 50 chars + "…"
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn test_get_node_preview_short() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        dialogue.add_node(DialogueNode::new(1, "Short text"));
        editor.dialogues.push(dialogue);

        let preview = editor.get_node_preview(0, 1);
        assert_eq!(preview, "Short text");
        assert!(!preview.ends_with('…'));
    }

    #[test]
    fn test_is_choice_target_valid() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        dialogue.add_node(DialogueNode::new(1, "Root"));
        dialogue.add_node(DialogueNode::new(2, "Valid target"));
        editor.dialogues.push(dialogue);

        assert!(editor.is_choice_target_valid(0, 2));
        assert!(!editor.is_choice_target_valid(0, 99));
    }

    #[test]
    fn test_search_nodes_by_id() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        dialogue.add_node(DialogueNode::new(1, "Root"));
        dialogue.add_node(DialogueNode::new(5, "Node 5"));
        dialogue.add_node(DialogueNode::new(10, "Node 10"));
        editor.dialogues.push(dialogue);

        let results = editor.search_nodes(0, "5");
        assert_eq!(results.len(), 1);
        assert!(results.contains(&5));
    }

    #[test]
    fn test_search_nodes_by_text() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        dialogue.add_node(DialogueNode::new(1, "Hello merchant"));
        dialogue.add_node(DialogueNode::new(2, "Guard says hello"));
        editor.dialogues.push(dialogue);

        let results = editor.search_nodes(0, "merchant");
        assert_eq!(results.len(), 1);
        assert!(results.contains(&1));
    }

    #[test]
    fn test_search_nodes_case_insensitive() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        dialogue.add_node(DialogueNode::new(1, "IMPORTANT MESSAGE"));
        editor.dialogues.push(dialogue);

        let results = editor.search_nodes(0, "important");
        assert_eq!(results.len(), 1);
        assert!(results.contains(&1));
    }

    #[test]
    fn test_search_nodes_empty_filter() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        dialogue.add_node(DialogueNode::new(1, "Node 1"));
        dialogue.add_node(DialogueNode::new(2, "Node 2"));
        editor.dialogues.push(dialogue);

        let results = editor.search_nodes(0, "");
        assert_eq!(results.len(), 0);

        let results = editor.search_nodes(0, "   ");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_nodes_multiple_matches() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        dialogue.add_node(DialogueNode::new(1, "The key is important"));
        dialogue.add_node(DialogueNode::new(2, "This is important too"));
        dialogue.add_node(DialogueNode::new(3, "Not relevant"));
        editor.dialogues.push(dialogue);

        let results = editor.search_nodes(0, "important");
        assert_eq!(results.len(), 2);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
    }

    #[test]
    fn test_validation_errors_caching() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        dialogue.add_node(DialogueNode::new(1, "Root"));
        dialogue.add_node(DialogueNode::new(2, "Orphaned"));
        editor.dialogues.push(dialogue);

        // First validation
        let errors = editor.validate_dialogue_tree(0);
        assert!(!errors.is_empty());

        // Check that errors are cached
        assert!(!editor.dialogue_validation_errors.is_empty());
        assert_eq!(editor.dialogue_validation_errors, errors);
    }

    #[test]
    fn test_unreachable_nodes_caching() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);

        dialogue.add_node(DialogueNode::new(1, "Root"));
        dialogue.add_node(DialogueNode::new(2, "Unreachable"));
        editor.dialogues.push(dialogue);

        // Validate to populate cache
        let _ = editor.validate_dialogue_tree(0);

        // Check that unreachable nodes are cached
        assert!(editor.unreachable_nodes.contains(&2));
        assert!(!editor.unreachable_nodes.contains(&1));
    }

    #[test]
    fn test_complex_dialogue_tree_reachability() {
        let mut editor = DialogueEditorState::new();
        let mut dialogue = DialogueTree::new(1, "Complex Dialogue", 1);

        // Build tree:
        //     1 (root)
        //    / \
        //   2   3
        //  / \
        // 4   5
        // 6 is orphaned

        dialogue.add_node(DialogueNode::new(1, "Start"));
        dialogue.add_node(DialogueNode::new(2, "Path A"));
        dialogue.add_node(DialogueNode::new(3, "Path B"));
        dialogue.add_node(DialogueNode::new(4, "Path A-1"));
        dialogue.add_node(DialogueNode::new(5, "Path A-2"));
        dialogue.add_node(DialogueNode::new(6, "Orphaned"));

        // Add choices to create connections
        let mut node1 = dialogue.nodes.remove(&1).unwrap();
        node1.add_choice(DialogueChoice::new("Path A", Some(2)));
        node1.add_choice(DialogueChoice::new("Path B", Some(3)));
        dialogue.nodes.insert(1, node1);

        let mut node2 = dialogue.nodes.remove(&2).unwrap();
        node2.add_choice(DialogueChoice::new("Option 1", Some(4)));
        node2.add_choice(DialogueChoice::new("Option 2", Some(5)));
        dialogue.nodes.insert(2, node2);

        editor.dialogues.push(dialogue);

        let unreachable = editor.get_unreachable_nodes_for_dialogue(0);
        assert_eq!(unreachable.len(), 1);
        assert!(unreachable.contains(&6));

        let (total, reachable, unreachable_count) = editor.get_reachability_stats(0);
        assert_eq!(total, 6);
        assert_eq!(reachable, 5);
        assert_eq!(unreachable_count, 1);
    }
}
