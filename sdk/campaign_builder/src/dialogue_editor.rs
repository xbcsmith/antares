// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Dialogue Tree Editor for Campaign Builder
//!
//! This module provides a visual dialogue tree editor for creating and managing
//! NPC conversations with branching paths, conditions, and dialogue actions.
//!
//! # Features
//!
//! - Dialogue tree list view with search and filtering
//! - Node list view (flat list-based, not graph)
//! - Node editor with text, speaker, conditions, and actions
//! - Choice editor for player responses
//! - Condition and action configuration
//! - Dialogue tree validation and preview

use antares::domain::dialogue::{
    DialogueAction, DialogueChoice, DialogueCondition, DialogueId, DialogueNode, DialogueTree,
    NodeId,
};
use antares::domain::quest::QuestId;
use antares::domain::types::ItemId;
use serde::{Deserialize, Serialize};

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

    /// Available quest IDs (for conditions/actions)
    pub available_quest_ids: Vec<QuestId>,

    /// Available item IDs (for conditions/actions)
    pub available_item_ids: Vec<ItemId>,
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
            available_quest_ids: Vec::new(),
            available_item_ids: Vec::new(),
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
    pub fn add_node(&mut self) -> Result<(), String> {
        if self.selected_dialogue.is_none() {
            return Err("No dialogue selected".to_string());
        }

        let node_id = self
            .node_buffer
            .id
            .parse::<NodeId>()
            .map_err(|_| "Invalid node ID".to_string())?;

        let mut node = DialogueNode::new(node_id, &self.node_buffer.text);

        if !self.node_buffer.speaker_override.is_empty() {
            node.speaker_override = Some(self.node_buffer.speaker_override.clone());
        }
        node.is_terminal = self.node_buffer.is_terminal;

        if let Some(idx) = self.selected_dialogue {
            self.dialogues[idx].add_node(node);
            self.has_unsaved_changes = true;
            self.node_buffer = NodeEditBuffer::default();
            Ok(())
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialogue_editor_state_creation() {
        let editor = DialogueEditorState::new();
        assert_eq!(editor.dialogues.len(), 0);
        assert_eq!(editor.mode, DialogueEditorMode::List);
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
}
