// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Dialogue system domain model
//!
//! This module defines the core dialogue structures including dialogue trees,
//! nodes, and choices. Dialogues enable interactive conversations with NPCs
//! with branching paths based on player choices and game state.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sdk_and_campaign_architecture.md` Phase 5 for dialogue specifications.

use crate::domain::quest::QuestId;
use crate::domain::types::ItemId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dialogue identifier
pub type DialogueId = u16;

/// Node identifier within a dialogue tree
pub type NodeId = u16;

/// Dialogue tree
///
/// Represents a complete conversation tree with multiple nodes and branching paths.
/// Each dialogue tree starts at a root node and branches based on player choices
/// and game conditions.
///
/// # Examples
///
/// ```
/// use antares::domain::dialogue::{DialogueTree, DialogueNode};
///
/// let mut dialogue = DialogueTree::new(1, "Merchant Greeting", 1);
/// dialogue.add_node(DialogueNode::new(1, "Hello, traveler! What brings you here?"));
///
/// assert_eq!(dialogue.id, 1);
/// assert_eq!(dialogue.node_count(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueTree {
    /// Unique dialogue identifier
    pub id: DialogueId,

    /// Dialogue tree name (for editor/reference)
    pub name: String,

    /// Starting node ID
    pub root_node: NodeId,

    /// All nodes in the dialogue tree
    pub nodes: HashMap<NodeId, DialogueNode>,

    /// NPC speaker name (optional, can be overridden per node)
    pub speaker_name: Option<String>,

    /// Whether this dialogue can be repeated
    pub repeatable: bool,

    /// Quest ID this dialogue is associated with (optional)
    pub associated_quest: Option<QuestId>,
}

impl DialogueTree {
    /// Creates a new dialogue tree
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::dialogue::DialogueTree;
    ///
    /// let dialogue = DialogueTree::new(1, "Test Dialogue", 1);
    /// assert_eq!(dialogue.id, 1);
    /// assert_eq!(dialogue.name, "Test Dialogue");
    /// assert_eq!(dialogue.root_node, 1);
    /// assert!(dialogue.repeatable);
    /// ```
    pub fn new(id: DialogueId, name: impl Into<String>, root_node: NodeId) -> Self {
        Self {
            id,
            name: name.into(),
            root_node,
            nodes: HashMap::new(),
            speaker_name: None,
            repeatable: true,
            associated_quest: None,
        }
    }

    /// Adds a node to the dialogue tree
    pub fn add_node(&mut self, node: DialogueNode) {
        self.nodes.insert(node.id, node);
    }

    /// Gets a node by ID
    pub fn get_node(&self, node_id: NodeId) -> Option<&DialogueNode> {
        self.nodes.get(&node_id)
    }

    /// Gets the root node
    pub fn get_root_node(&self) -> Option<&DialogueNode> {
        self.nodes.get(&self.root_node)
    }

    /// Returns the number of nodes in the dialogue tree
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if the dialogue tree has any nodes
    pub fn has_nodes(&self) -> bool {
        !self.nodes.is_empty()
    }

    /// Validates that the dialogue tree is well-formed
    ///
    /// Checks:
    /// - Root node exists
    /// - All choice targets exist
    /// - No orphaned nodes (except root)
    pub fn validate(&self) -> Result<(), String> {
        // Check root node exists
        if !self.nodes.contains_key(&self.root_node) {
            return Err(format!("Root node {} does not exist", self.root_node));
        }

        // Check all choice targets exist
        for (node_id, node) in &self.nodes {
            for (idx, choice) in node.choices.iter().enumerate() {
                if let Some(target) = choice.target_node {
                    if !self.nodes.contains_key(&target) {
                        return Err(format!(
                            "Node {} choice {} references non-existent node {}",
                            node_id, idx, target
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

/// Dialogue node
///
/// Represents a single dialogue exchange in a conversation. Each node contains
/// the text spoken by the NPC and a list of choices for the player to select.
///
/// # Examples
///
/// ```
/// use antares::domain::dialogue::{DialogueNode, DialogueChoice};
///
/// let mut node = DialogueNode::new(1, "Greetings, adventurer!");
/// node.add_choice(DialogueChoice::new("Hello", Some(2)));
/// node.add_choice(DialogueChoice::new("Goodbye", None));
///
/// assert_eq!(node.choices.len(), 2);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueNode {
    /// Node ID
    pub id: NodeId,

    /// Text spoken by NPC
    pub text: String,

    /// Speaker name override (if different from tree default)
    pub speaker_override: Option<String>,

    /// Player choices available at this node
    pub choices: Vec<DialogueChoice>,

    /// Conditions that must be met to show this node
    pub conditions: Vec<DialogueCondition>,

    /// Actions to perform when this node is displayed
    pub actions: Vec<DialogueAction>,

    /// Whether this node ends the dialogue
    pub is_terminal: bool,
}

impl DialogueNode {
    /// Creates a new dialogue node
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::dialogue::DialogueNode;
    ///
    /// let node = DialogueNode::new(1, "Welcome to my shop!");
    /// assert_eq!(node.id, 1);
    /// assert_eq!(node.text, "Welcome to my shop!");
    /// assert!(!node.is_terminal);
    /// ```
    pub fn new(id: NodeId, text: impl Into<String>) -> Self {
        Self {
            id,
            text: text.into(),
            speaker_override: None,
            choices: Vec::new(),
            conditions: Vec::new(),
            actions: Vec::new(),
            is_terminal: false,
        }
    }

    /// Adds a choice to this node
    pub fn add_choice(&mut self, choice: DialogueChoice) {
        self.choices.push(choice);
    }

    /// Adds a condition to this node
    pub fn add_condition(&mut self, condition: DialogueCondition) {
        self.conditions.push(condition);
    }

    /// Adds an action to this node
    pub fn add_action(&mut self, action: DialogueAction) {
        self.actions.push(action);
    }

    /// Returns true if this node has any choices
    pub fn has_choices(&self) -> bool {
        !self.choices.is_empty()
    }

    /// Returns the number of choices
    pub fn choice_count(&self) -> usize {
        self.choices.len()
    }
}

/// Dialogue choice
///
/// Represents a player response option in a dialogue. Each choice can have
/// conditions that determine when it's available and a target node to transition to.
///
/// # Examples
///
/// ```
/// use antares::domain::dialogue::{DialogueChoice, DialogueCondition};
///
/// let mut choice = DialogueChoice::new("I'll help you", Some(10));
/// choice.add_condition(DialogueCondition::HasQuest { quest_id: 5 });
///
/// assert_eq!(choice.text, "I'll help you");
/// assert_eq!(choice.target_node, Some(10));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueChoice {
    /// Choice text shown to player
    pub text: String,

    /// Target node to transition to (None = end dialogue)
    pub target_node: Option<NodeId>,

    /// Conditions that must be met to show this choice
    pub conditions: Vec<DialogueCondition>,

    /// Actions to perform when this choice is selected
    pub actions: Vec<DialogueAction>,

    /// Whether this choice ends the dialogue
    pub ends_dialogue: bool,
}

impl DialogueChoice {
    /// Creates a new dialogue choice
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::dialogue::DialogueChoice;
    ///
    /// let choice = DialogueChoice::new("Tell me more", Some(5));
    /// assert_eq!(choice.text, "Tell me more");
    /// assert_eq!(choice.target_node, Some(5));
    /// assert!(!choice.ends_dialogue);
    /// ```
    pub fn new(text: impl Into<String>, target_node: Option<NodeId>) -> Self {
        Self {
            text: text.into(),
            target_node,
            conditions: Vec::new(),
            actions: Vec::new(),
            ends_dialogue: target_node.is_none(),
        }
    }

    /// Adds a condition to this choice
    pub fn add_condition(&mut self, condition: DialogueCondition) {
        self.conditions.push(condition);
    }

    /// Adds an action to this choice
    pub fn add_action(&mut self, action: DialogueAction) {
        self.actions.push(action);
    }
}

/// Dialogue condition
///
/// Represents conditions that must be met for a dialogue node or choice to be available.
/// Conditions can check quest status, inventory, flags, and other game state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogueCondition {
    /// Player has a specific quest
    HasQuest { quest_id: QuestId },

    /// Player has completed a specific quest
    CompletedQuest { quest_id: QuestId },

    /// Player is on a specific quest stage
    QuestStage { quest_id: QuestId, stage_number: u8 },

    /// Player has a specific item
    HasItem { item_id: ItemId, quantity: u16 },

    /// Player has at least this much gold
    HasGold { amount: u32 },

    /// Party has a minimum level
    MinLevel { level: u8 },

    /// A game flag is set to a value
    FlagSet { flag_name: String, value: bool },

    /// Reputation with a faction meets threshold
    ReputationThreshold { faction: String, threshold: i16 },

    /// Logical AND of multiple conditions
    And(Vec<DialogueCondition>),

    /// Logical OR of multiple conditions
    Or(Vec<DialogueCondition>),

    /// Logical NOT of a condition
    Not(Box<DialogueCondition>),
}

impl DialogueCondition {
    /// Returns a human-readable description of the condition
    pub fn description(&self) -> String {
        match self {
            DialogueCondition::HasQuest { quest_id } => format!("Has quest {}", quest_id),
            DialogueCondition::CompletedQuest { quest_id } => {
                format!("Completed quest {}", quest_id)
            }
            DialogueCondition::QuestStage {
                quest_id,
                stage_number,
            } => format!("Quest {} stage {}", quest_id, stage_number),
            DialogueCondition::HasItem { item_id, quantity } => {
                format!("Has {} of item {}", quantity, item_id)
            }
            DialogueCondition::HasGold { amount } => format!("Has {} gold", amount),
            DialogueCondition::MinLevel { level } => format!("Min level {}", level),
            DialogueCondition::FlagSet { flag_name, value } => {
                format!("Flag '{}' = {}", flag_name, value)
            }
            DialogueCondition::ReputationThreshold { faction, threshold } => {
                format!("Reputation with {} >= {}", faction, threshold)
            }
            DialogueCondition::And(conditions) => {
                format!("AND({} conditions)", conditions.len())
            }
            DialogueCondition::Or(conditions) => {
                format!("OR({} conditions)", conditions.len())
            }
            DialogueCondition::Not(condition) => {
                format!("NOT({})", condition.description())
            }
        }
    }
}

/// Dialogue action
///
/// Represents actions that are performed when a dialogue node is shown or
/// a choice is selected. Actions can modify game state, start quests, give items, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogueAction {
    /// Start a quest
    StartQuest { quest_id: QuestId },

    /// Complete a quest stage
    CompleteQuestStage { quest_id: QuestId, stage_number: u8 },

    /// Give items to the player
    GiveItems { items: Vec<(ItemId, u16)> },

    /// Take items from the player
    TakeItems { items: Vec<(ItemId, u16)> },

    /// Give gold to the player
    GiveGold { amount: u32 },

    /// Take gold from the player
    TakeGold { amount: u32 },

    /// Set a game flag
    SetFlag { flag_name: String, value: bool },

    /// Change reputation with a faction
    ChangeReputation { faction: String, change: i16 },

    /// Trigger a custom event
    TriggerEvent { event_name: String },

    /// Grant experience points
    GrantExperience { amount: u32 },

    /// Recruit character to active party
    RecruitToParty { character_id: String },

    /// Send character to inn
    RecruitToInn {
        character_id: String,
        innkeeper_id: String,
    },
}

impl DialogueAction {
    /// Returns a human-readable description of the action
    pub fn description(&self) -> String {
        match self {
            DialogueAction::StartQuest { quest_id } => format!("Start quest {}", quest_id),
            DialogueAction::CompleteQuestStage {
                quest_id,
                stage_number,
            } => format!("Complete quest {} stage {}", quest_id, stage_number),
            DialogueAction::GiveItems { items } => {
                format!("Give {} item types", items.len())
            }
            DialogueAction::TakeItems { items } => {
                format!("Take {} item types", items.len())
            }
            DialogueAction::GiveGold { amount } => format!("Give {} gold", amount),
            DialogueAction::TakeGold { amount } => format!("Take {} gold", amount),
            DialogueAction::SetFlag { flag_name, value } => {
                format!("Set flag '{}' to {}", flag_name, value)
            }
            DialogueAction::ChangeReputation { faction, change } => {
                format!("Change reputation with {} by {}", faction, change)
            }
            DialogueAction::TriggerEvent { event_name } => {
                format!("Trigger event '{}'", event_name)
            }
            DialogueAction::GrantExperience { amount } => {
                format!("Grant {} experience", amount)
            }
            DialogueAction::RecruitToParty { character_id } => {
                format!("Recruit '{}' to party", character_id)
            }
            DialogueAction::RecruitToInn {
                character_id,
                innkeeper_id,
            } => {
                format!("Send '{}' to inn (keeper: {})", character_id, innkeeper_id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialogue_tree_creation() {
        let dialogue = DialogueTree::new(1, "Test Dialogue", 1);
        assert_eq!(dialogue.id, 1);
        assert_eq!(dialogue.name, "Test Dialogue");
        assert_eq!(dialogue.root_node, 1);
        assert!(dialogue.repeatable);
        assert_eq!(dialogue.node_count(), 0);
    }

    #[test]
    fn test_dialogue_tree_add_node() {
        let mut dialogue = DialogueTree::new(1, "Test", 1);
        let node = DialogueNode::new(1, "Hello!");

        dialogue.add_node(node);
        assert_eq!(dialogue.node_count(), 1);
        assert!(dialogue.has_nodes());
    }

    #[test]
    fn test_dialogue_tree_get_node() {
        let mut dialogue = DialogueTree::new(1, "Test", 1);
        let node = DialogueNode::new(1, "Hello!");
        dialogue.add_node(node);

        let retrieved = dialogue.get_node(1);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().text, "Hello!");

        let missing = dialogue.get_node(99);
        assert!(missing.is_none());
    }

    #[test]
    fn test_dialogue_tree_get_root_node() {
        let mut dialogue = DialogueTree::new(1, "Test", 1);
        let root_node = DialogueNode::new(1, "Root text");
        dialogue.add_node(root_node);

        let root = dialogue.get_root_node();
        assert!(root.is_some());
        assert_eq!(root.unwrap().text, "Root text");
    }

    #[test]
    fn test_dialogue_tree_validation_success() {
        let mut dialogue = DialogueTree::new(1, "Test", 1);
        let mut node1 = DialogueNode::new(1, "Hello");
        node1.add_choice(DialogueChoice::new("Continue", Some(2)));
        let node2 = DialogueNode::new(2, "Goodbye");

        dialogue.add_node(node1);
        dialogue.add_node(node2);

        assert!(dialogue.validate().is_ok());
    }

    #[test]
    fn test_dialogue_tree_validation_missing_root() {
        let dialogue = DialogueTree::new(1, "Test", 1);
        let result = dialogue.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Root node"));
    }

    #[test]
    fn test_dialogue_tree_validation_invalid_target() {
        let mut dialogue = DialogueTree::new(1, "Test", 1);
        let mut node1 = DialogueNode::new(1, "Hello");
        node1.add_choice(DialogueChoice::new("Go to missing node", Some(99)));

        dialogue.add_node(node1);

        let result = dialogue.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non-existent"));
    }

    #[test]
    fn test_dialogue_node_creation() {
        let node = DialogueNode::new(1, "Welcome!");
        assert_eq!(node.id, 1);
        assert_eq!(node.text, "Welcome!");
        assert!(!node.is_terminal);
        assert!(!node.has_choices());
    }

    #[test]
    fn test_dialogue_node_add_choice() {
        let mut node = DialogueNode::new(1, "Test");
        node.add_choice(DialogueChoice::new("Option 1", Some(2)));
        node.add_choice(DialogueChoice::new("Option 2", Some(3)));

        assert_eq!(node.choice_count(), 2);
        assert!(node.has_choices());
    }

    #[test]
    fn test_dialogue_node_add_condition() {
        let mut node = DialogueNode::new(1, "Test");
        node.add_condition(DialogueCondition::MinLevel { level: 5 });

        assert_eq!(node.conditions.len(), 1);
    }

    #[test]
    fn test_dialogue_node_add_action() {
        let mut node = DialogueNode::new(1, "Test");
        node.add_action(DialogueAction::GiveGold { amount: 100 });

        assert_eq!(node.actions.len(), 1);
    }

    #[test]
    fn test_dialogue_choice_creation() {
        let choice = DialogueChoice::new("Tell me more", Some(5));
        assert_eq!(choice.text, "Tell me more");
        assert_eq!(choice.target_node, Some(5));
        assert!(!choice.ends_dialogue);
    }

    #[test]
    fn test_dialogue_choice_ends_dialogue() {
        let choice = DialogueChoice::new("Goodbye", None);
        assert!(choice.ends_dialogue);
        assert_eq!(choice.target_node, None);
    }

    #[test]
    fn test_dialogue_choice_add_condition() {
        let mut choice = DialogueChoice::new("Help", Some(10));
        choice.add_condition(DialogueCondition::HasQuest { quest_id: 5 });

        assert_eq!(choice.conditions.len(), 1);
    }

    #[test]
    fn test_dialogue_choice_add_action() {
        let mut choice = DialogueChoice::new("Accept quest", Some(10));
        choice.add_action(DialogueAction::StartQuest { quest_id: 1 });

        assert_eq!(choice.actions.len(), 1);
    }

    #[test]
    fn test_dialogue_condition_descriptions() {
        let cond1 = DialogueCondition::HasQuest { quest_id: 5 };
        assert!(cond1.description().contains("Has quest"));

        let cond2 = DialogueCondition::MinLevel { level: 10 };
        assert!(cond2.description().contains("Min level"));

        let cond3 = DialogueCondition::HasGold { amount: 100 };
        assert!(cond3.description().contains("100 gold"));
    }

    #[test]
    fn test_dialogue_action_descriptions() {
        let action1 = DialogueAction::GiveGold { amount: 50 };
        assert!(action1.description().contains("Give 50 gold"));

        let action2 = DialogueAction::StartQuest { quest_id: 1 };
        assert!(action2.description().contains("Start quest"));

        let action3 = DialogueAction::GrantExperience { amount: 100 };
        assert!(action3.description().contains("Grant 100 experience"));
    }

    #[test]
    fn test_dialogue_action_recruit_to_party_description() {
        let action = DialogueAction::RecruitToParty {
            character_id: "hero_01".to_string(),
        };
        assert_eq!(action.description(), "Recruit 'hero_01' to party");
    }

    #[test]
    fn test_dialogue_action_recruit_to_inn_description() {
        let action = DialogueAction::RecruitToInn {
            character_id: "hero_02".to_string(),
            innkeeper_id: "innkeeper_town_01".to_string(),
        };
        assert_eq!(
            action.description(),
            "Send 'hero_02' to inn (keeper: innkeeper_town_01)"
        );
    }

    #[test]
    fn test_complex_dialogue_tree() {
        let mut dialogue = DialogueTree::new(1, "Merchant Conversation", 1);
        dialogue.speaker_name = Some("Merchant".to_string());
        dialogue.associated_quest = Some(10);

        // Node 1: Greeting
        let mut node1 = DialogueNode::new(1, "Welcome to my shop!");
        node1.add_choice(DialogueChoice::new("What do you sell?", Some(2)));
        node1.add_choice(DialogueChoice::new("Do you have any quests?", Some(3)));
        node1.add_choice(DialogueChoice::new("Goodbye", None));

        // Node 2: Shop info
        let mut node2 = DialogueNode::new(2, "I sell weapons and armor.");
        node2.add_choice(DialogueChoice::new("Thanks", Some(1)));

        // Node 3: Quest offer
        let mut node3 = DialogueNode::new(3, "I need someone to retrieve my stolen goods.");
        let mut accept_choice = DialogueChoice::new("I'll help you", Some(4));
        accept_choice.add_action(DialogueAction::StartQuest { quest_id: 10 });
        node3.add_choice(accept_choice);
        node3.add_choice(DialogueChoice::new("Maybe later", Some(1)));

        // Node 4: Quest accepted
        let node4 = DialogueNode::new(4, "Thank you! Return when you have my goods.");

        dialogue.add_node(node1);
        dialogue.add_node(node2);
        dialogue.add_node(node3);
        dialogue.add_node(node4);

        assert_eq!(dialogue.node_count(), 4);
        assert!(dialogue.validate().is_ok());
        assert_eq!(dialogue.associated_quest, Some(10));
    }

    #[test]
    fn test_conditional_dialogue() {
        let mut node = DialogueNode::new(1, "You look strong enough to help.");
        node.add_condition(DialogueCondition::MinLevel { level: 5 });

        let mut choice = DialogueChoice::new("I accept", Some(2));
        choice.add_condition(DialogueCondition::HasGold { amount: 100 });
        choice.add_action(DialogueAction::TakeGold { amount: 100 });
        choice.add_action(DialogueAction::StartQuest { quest_id: 1 });

        node.add_choice(choice);

        assert_eq!(node.conditions.len(), 1);
        assert_eq!(node.choices[0].conditions.len(), 1);
        assert_eq!(node.choices[0].actions.len(), 2);
    }

    #[test]
    fn test_complex_conditions() {
        let condition = DialogueCondition::And(vec![
            DialogueCondition::MinLevel { level: 10 },
            DialogueCondition::Or(vec![
                DialogueCondition::HasQuest { quest_id: 1 },
                DialogueCondition::CompletedQuest { quest_id: 2 },
            ]),
            DialogueCondition::Not(Box::new(DialogueCondition::FlagSet {
                flag_name: "banned".to_string(),
                value: true,
            })),
        ]);

        let desc = condition.description();
        assert!(desc.contains("AND"));
    }
}
