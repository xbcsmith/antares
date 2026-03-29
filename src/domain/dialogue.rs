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
//! See `docs/explanation/sdk_and_campaign_architecture.md` for dialogue specifications.

use crate::domain::quest::QuestId;
use crate::domain::types::ItemId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
/// Merchant dialogue policy is encoded through [`DialogueTree::contains_open_merchant_for_npc`]
/// and the SDK-owned metadata stored in [`DialogueSdkMetadata`]. A merchant-capable
/// dialogue must explicitly contain a [`DialogueAction::OpenMerchant`] action for the
/// merchant NPC rather than relying on implicit runtime shortcuts.
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
    pub nodes: BTreeMap<NodeId, DialogueNode>,

    /// NPC speaker name (optional, can be overridden per node)
    pub speaker_name: Option<String>,

    /// Whether this dialogue can be repeated
    pub repeatable: bool,

    /// Quest ID this dialogue is associated with (optional)
    pub associated_quest: Option<QuestId>,

    /// SDK-owned metadata used to track generated or injected dialogue content.
    ///
    /// This metadata lets the Campaign Builder distinguish SDK-managed merchant
    /// dialogue content from author-created dialogue content so that merchant
    /// augmentation and removal can be performed non-destructively.
    #[serde(default)]
    pub sdk_metadata: DialogueSdkMetadata,
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
            nodes: BTreeMap::new(),
            speaker_name: None,
            repeatable: true,
            associated_quest: None,
            sdk_metadata: DialogueSdkMetadata::default(),
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

    /// Returns `true` when any node or choice in this tree contains an
    /// explicit [`DialogueAction::OpenMerchant`] action for `npc_id`.
    ///
    /// This is the machine-checkable contract for merchant-capable dialogue.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::dialogue::{DialogueAction, DialogueChoice, DialogueNode, DialogueTree};
    ///
    /// let mut tree = DialogueTree::new(7, "Merchant".to_string(), 1);
    /// let mut root = DialogueNode::new(1, "Welcome.");
    /// let mut choice = DialogueChoice::new("Show me your wares.", Some(2));
    /// choice.add_action(DialogueAction::OpenMerchant {
    ///     npc_id: "merchant_tom".to_string(),
    /// });
    /// root.add_choice(choice);
    /// tree.add_node(root);
    ///
    /// assert!(tree.contains_open_merchant_for_npc("merchant_tom"));
    /// assert!(!tree.contains_open_merchant_for_npc("innkeeper_anna"));
    /// ```
    pub fn contains_open_merchant_for_npc(&self, npc_id: &str) -> bool {
        self.nodes.values().any(|node| {
            node.actions
                .iter()
                .any(|action| action.opens_merchant_for_npc(npc_id))
                || node.choices.iter().any(|choice| {
                    choice
                        .actions
                        .iter()
                        .any(|action| action.opens_merchant_for_npc(npc_id))
                })
        })
    }

    /// Returns `true` when this tree contains SDK-managed merchant content.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::dialogue::{DialogueSdkManagedContent, DialogueTree};
    ///
    /// let mut tree = DialogueTree::new(1, "Merchant".to_string(), 1);
    /// assert!(!tree.has_sdk_managed_merchant_content());
    ///
    /// tree.sdk_metadata
    ///     .managed_content
    ///     .insert(DialogueSdkManagedContent::MerchantTemplateTree);
    ///
    /// assert!(tree.has_sdk_managed_merchant_content());
    /// ```
    pub fn has_sdk_managed_merchant_content(&self) -> bool {
        self.sdk_metadata.has_merchant_content()
            || self
                .nodes
                .values()
                .any(DialogueNode::has_sdk_managed_merchant_content)
    }

    /// Returns the next available node ID for adding SDK-managed nodes to this tree.
    ///
    /// Returns `1` when the tree has no nodes.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::dialogue::{DialogueNode, DialogueTree};
    ///
    /// let mut tree = DialogueTree::new(1, "Merchant".to_string(), 1);
    /// assert_eq!(tree.next_available_node_id(), 1);
    ///
    /// tree.add_node(DialogueNode::new(1, "Hello"));
    /// tree.add_node(DialogueNode::new(4, "Shop"));
    ///
    /// assert_eq!(tree.next_available_node_id(), 5);
    /// ```
    pub fn next_available_node_id(&self) -> NodeId {
        self.nodes
            .keys()
            .copied()
            .max()
            .map(|max_id| max_id.saturating_add(1))
            .unwrap_or(1)
    }

    /// Creates a built-in standard merchant dialogue template tree for `npc_id`.
    ///
    /// The generated template:
    ///
    /// - marks the tree as SDK-managed merchant content
    /// - creates a root greeting node
    /// - adds a root choice `"Show me your wares."`
    /// - routes that choice to a generated merchant action node
    /// - includes a goodbye choice that ends the dialogue
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::dialogue::DialogueTree;
    ///
    /// let tree = DialogueTree::standard_merchant_template(
    ///     10,
    ///     "merchant_tom",
    ///     "Tom the Merchant",
    /// );
    ///
    /// assert!(tree.contains_open_merchant_for_npc("merchant_tom"));
    /// assert!(tree.has_sdk_managed_merchant_content());
    /// ```
    pub fn standard_merchant_template(id: DialogueId, npc_id: &str, npc_name: &str) -> Self {
        let root_node_id: NodeId = 1;
        let merchant_node_id: NodeId = 2;
        let goodbye_text = "Farewell.";

        let mut tree =
            DialogueTree::new(id, format!("{} Merchant Dialogue", npc_name), root_node_id);
        tree.speaker_name = Some(npc_name.to_string());
        tree.repeatable = true;
        tree.sdk_metadata
            .managed_content
            .insert(DialogueSdkManagedContent::MerchantTemplateTree);

        let mut root = DialogueNode::new(
            root_node_id,
            format!("Welcome. Take a look at what {} has for sale.", npc_name),
        );
        root.add_choice(DialogueChoice::sdk_managed_merchant_choice(
            merchant_node_id,
        ));
        root.add_choice(DialogueChoice::new(goodbye_text, None));

        let mut merchant_node = DialogueNode::new(
            merchant_node_id,
            format!("Of course. Here is what {} is offering.", npc_name),
        );
        merchant_node
            .sdk_metadata
            .managed_content
            .insert(DialogueSdkManagedContent::MerchantOpenNode);
        merchant_node.add_action(DialogueAction::OpenMerchant {
            npc_id: npc_id.to_string(),
        });
        merchant_node.is_terminal = true;

        tree.add_node(root);
        tree.add_node(merchant_node);
        tree
    }

    /// Inserts the standard merchant branch into the root node when the tree
    /// does not already contain an explicit merchant-opening path for `npc_id`.
    ///
    /// Returns `true` when the tree was modified and `false` when the tree
    /// already satisfied the merchant dialogue contract.
    ///
    /// The inserted branch is SDK-managed and can be removed later without
    /// affecting unrelated dialogue content.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::dialogue::{DialogueNode, DialogueTree};
    ///
    /// let mut tree = DialogueTree::new(20, "Custom Dialogue".to_string(), 1);
    /// tree.add_node(DialogueNode::new(1, "Hello there."));
    ///
    /// let changed = tree.ensure_standard_merchant_branch("merchant_tom", "Tom the Merchant");
    ///
    /// assert!(changed);
    /// assert!(tree.contains_open_merchant_for_npc("merchant_tom"));
    /// ```
    pub fn ensure_standard_merchant_branch(&mut self, npc_id: &str, npc_name: &str) -> bool {
        if self.contains_open_merchant_for_npc(npc_id) {
            return false;
        }

        let merchant_node_id = self.next_available_node_id();
        let root_node_id = self.root_node;

        let Some(root) = self.nodes.get_mut(&root_node_id) else {
            return false;
        };

        let merchant_choice = DialogueChoice::sdk_managed_merchant_choice(merchant_node_id);
        root.choices.push(merchant_choice);

        let mut merchant_node = DialogueNode::new(
            merchant_node_id,
            format!("Of course. Here is what {} is offering.", npc_name),
        );
        merchant_node
            .sdk_metadata
            .managed_content
            .insert(DialogueSdkManagedContent::MerchantOpenNode);
        merchant_node.add_action(DialogueAction::OpenMerchant {
            npc_id: npc_id.to_string(),
        });
        merchant_node.is_terminal = true;

        self.nodes.insert(merchant_node_id, merchant_node);
        self.sdk_metadata
            .managed_content
            .insert(DialogueSdkManagedContent::MerchantBranchInsertion);

        true
    }

    /// Removes SDK-managed merchant dialogue content from this tree.
    ///
    /// Returns `true` when any merchant content was removed and `false` when
    /// the tree was unchanged.
    ///
    /// Removal is intentionally conservative:
    ///
    /// - SDK-managed merchant nodes are removed entirely
    /// - root or custom nodes are retained
    /// - only SDK-managed merchant choices are removed from retained nodes
    /// - all non-merchant custom content remains intact
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::dialogue::DialogueTree;
    ///
    /// let mut tree = DialogueTree::standard_merchant_template(
    ///     30,
    ///     "merchant_tom",
    ///     "Tom the Merchant",
    /// );
    ///
    /// assert!(tree.remove_sdk_managed_merchant_content());
    /// assert!(!tree.contains_open_merchant_for_npc("merchant_tom"));
    /// ```
    pub fn remove_sdk_managed_merchant_content(&mut self) -> bool {
        let removable_nodes: Vec<NodeId> = self
            .nodes
            .iter()
            .filter_map(|(node_id, node)| {
                node.sdk_metadata
                    .managed_content
                    .contains(&DialogueSdkManagedContent::MerchantOpenNode)
                    .then_some(*node_id)
            })
            .collect();

        let mut changed = false;

        for node_id in removable_nodes {
            if self.nodes.remove(&node_id).is_some() {
                changed = true;
            }
        }

        for node in self.nodes.values_mut() {
            let original_choice_count = node.choices.len();
            node.choices
                .retain(|choice| !choice.is_sdk_managed_merchant_choice());
            if node.choices.len() != original_choice_count {
                changed = true;
            }

            if node
                .sdk_metadata
                .managed_content
                .remove(&DialogueSdkManagedContent::MerchantOpenNode)
            {
                changed = true;
            }
        }

        if self
            .sdk_metadata
            .managed_content
            .remove(&DialogueSdkManagedContent::MerchantTemplateTree)
        {
            changed = true;
        }

        if self
            .sdk_metadata
            .managed_content
            .remove(&DialogueSdkManagedContent::MerchantBranchInsertion)
        {
            changed = true;
        }

        changed
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

    /// SDK-owned metadata used to identify machine-managed node content.
    #[serde(default)]
    pub sdk_metadata: DialogueSdkMetadata,
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
            sdk_metadata: DialogueSdkMetadata::default(),
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

    /// Returns `true` when this node contains SDK-managed merchant content.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::dialogue::{DialogueNode, DialogueSdkManagedContent};
    ///
    /// let mut node = DialogueNode::new(1, "Shop");
    /// assert!(!node.has_sdk_managed_merchant_content());
    ///
    /// node.sdk_metadata
    ///     .managed_content
    ///     .insert(DialogueSdkManagedContent::MerchantOpenNode);
    ///
    /// assert!(node.has_sdk_managed_merchant_content());
    /// ```
    pub fn has_sdk_managed_merchant_content(&self) -> bool {
        self.sdk_metadata.has_merchant_content()
            || self
                .choices
                .iter()
                .any(DialogueChoice::is_sdk_managed_merchant_choice)
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

    /// SDK-owned metadata used to identify machine-managed choice content.
    #[serde(default)]
    pub sdk_metadata: DialogueSdkMetadata,
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
            sdk_metadata: DialogueSdkMetadata::default(),
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

    /// Creates the standard SDK-managed merchant choice that opens the merchant branch.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::dialogue::{DialogueChoice, DialogueSdkManagedContent};
    ///
    /// let choice = DialogueChoice::sdk_managed_merchant_choice(2);
    ///
    /// assert_eq!(choice.target_node, Some(2));
    /// assert!(choice
    ///     .sdk_metadata
    ///     .managed_content
    ///     .contains(&DialogueSdkManagedContent::MerchantChoice));
    /// ```
    pub fn sdk_managed_merchant_choice(target_node: NodeId) -> Self {
        let mut choice = Self::new("Show me your wares.", Some(target_node));
        choice
            .sdk_metadata
            .managed_content
            .insert(DialogueSdkManagedContent::MerchantChoice);
        choice
    }

    /// Returns `true` when this choice is SDK-managed merchant content.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::dialogue::DialogueChoice;
    ///
    /// let choice = DialogueChoice::sdk_managed_merchant_choice(3);
    /// assert!(choice.is_sdk_managed_merchant_choice());
    /// ```
    pub fn is_sdk_managed_merchant_choice(&self) -> bool {
        self.sdk_metadata
            .managed_content
            .contains(&DialogueSdkManagedContent::MerchantChoice)
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

/// SDK-owned metadata used to track generated or inserted dialogue content.
///
/// Merchant dialogue generation and augmentation must be reversible without
/// deleting unrelated authored dialogue content. This metadata marks which
/// dialogue structures are managed by the SDK so later phases can remove or
/// repair only the machine-managed merchant portions.
///
/// # Examples
///
/// ```
/// use antares::domain::dialogue::{DialogueSdkManagedContent, DialogueSdkMetadata};
///
/// let mut metadata = DialogueSdkMetadata::default();
/// metadata
///     .managed_content
///     .insert(DialogueSdkManagedContent::MerchantTemplateTree);
///
/// assert!(metadata.has_merchant_content());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueSdkMetadata {
    /// Set of SDK-managed content markers attached to a dialogue structure.
    #[serde(default)]
    pub managed_content: BTreeSet<DialogueSdkManagedContent>,
}
impl DialogueSdkMetadata {
    /// Returns `true` when any merchant-related SDK marker is present.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::dialogue::{DialogueSdkManagedContent, DialogueSdkMetadata};
    ///
    /// let mut metadata = DialogueSdkMetadata::default();
    /// assert!(!metadata.has_merchant_content());
    ///
    /// metadata
    ///     .managed_content
    ///     .insert(DialogueSdkManagedContent::MerchantChoice);
    ///
    /// assert!(metadata.has_merchant_content());
    /// ```
    pub fn has_merchant_content(&self) -> bool {
        self.managed_content
            .iter()
            .any(|marker| marker.is_merchant_marker())
    }
}

/// Marker describing which SDK-managed content is attached to a dialogue structure.
///
/// # Examples
///
/// ```
/// use antares::domain::dialogue::DialogueSdkManagedContent;
///
/// assert!(DialogueSdkManagedContent::MerchantChoice.is_merchant_marker());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DialogueSdkManagedContent {
    /// Entire dialogue tree was generated from the built-in merchant template.
    MerchantTemplateTree,
    /// Existing dialogue tree was augmented with an SDK-managed merchant branch.
    MerchantBranchInsertion,
    /// Choice was inserted by the SDK to route into a merchant branch.
    MerchantChoice,
    /// Node was inserted by the SDK to execute `OpenMerchant`.
    MerchantOpenNode,
}

impl DialogueSdkManagedContent {
    /// Returns `true` for merchant-related SDK markers.
    pub fn is_merchant_marker(&self) -> bool {
        matches!(
            *self,
            DialogueSdkManagedContent::MerchantTemplateTree
                | DialogueSdkManagedContent::MerchantBranchInsertion
                | DialogueSdkManagedContent::MerchantChoice
                | DialogueSdkManagedContent::MerchantOpenNode
        )
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

    /// Open the inn party management interface
    OpenInnManagement { innkeeper_id: String },

    /// Buy an item from a merchant NPC
    ///
    /// Triggers a purchase transaction using the domain transaction layer.
    /// If `target_character_id` is `None`, the item is given to the first
    /// available party member.
    BuyItem {
        /// The item to purchase
        item_id: crate::domain::types::ItemId,
        /// Character to receive the item (None = first available party member)
        target_character_id: Option<crate::domain::types::CharacterId>,
    },

    /// Sell an item to a merchant NPC
    ///
    /// Triggers a sell transaction using the domain transaction layer.
    /// If `source_character_id` is `None`, the first party member who has the
    /// item in their inventory will be used as the source.
    SellItem {
        /// The item to sell
        item_id: crate::domain::types::ItemId,
        /// Character selling the item (None = search all party members)
        source_character_id: Option<crate::domain::types::CharacterId>,
    },

    /// Open the merchant shop UI for an NPC
    ///
    /// Transitions the game into the shop interaction mode for the specified
    /// merchant NPC.
    ///
    /// Runtime contract:
    /// - executing this action must open the merchant inventory for `npc_id`
    ///
    /// Authoring contract:
    /// - merchant-capable dialogue must explicitly contain this action for the
    ///   merchant NPC
    /// - the `I` key during dialogue remains only a runtime convenience shortcut,
    ///   not the content-authoring standard
    OpenMerchant {
        /// NpcId of the merchant whose shop should be opened
        npc_id: String,
    },

    /// Consume a service from a priest or innkeeper NPC
    ///
    /// Triggers a service transaction using the domain transaction layer. If
    /// `target_character_ids` is empty, the service is applied to the whole party.
    ConsumeService {
        /// Identifier of the service to consume (e.g. "heal_all", "resurrect")
        service_id: String,
        /// Characters to apply the service to (empty = apply to whole party)
        target_character_ids: Vec<crate::domain::types::CharacterId>,
    },
}

impl DialogueAction {
    /// Returns `true` when this action explicitly opens the merchant UI for `npc_id`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::dialogue::DialogueAction;
    ///
    /// let action = DialogueAction::OpenMerchant {
    ///     npc_id: "merchant_tom".to_string(),
    /// };
    ///
    /// assert!(action.opens_merchant_for_npc("merchant_tom"));
    /// assert!(!action.opens_merchant_for_npc("merchant_sue"));
    /// ```
    pub fn opens_merchant_for_npc(&self, npc_id: &str) -> bool {
        matches!(
            self,
            DialogueAction::OpenMerchant { npc_id: action_npc_id } if action_npc_id == npc_id
        )
    }

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
            DialogueAction::OpenInnManagement { innkeeper_id } => {
                format!("Open party management at inn (keeper: {})", innkeeper_id)
            }
            DialogueAction::BuyItem {
                item_id,
                target_character_id,
            } => match target_character_id {
                Some(cid) => format!("Buy item {} for character {}", item_id, cid),
                None => format!("Buy item {} for first available character", item_id),
            },
            DialogueAction::SellItem {
                item_id,
                source_character_id,
            } => match source_character_id {
                Some(cid) => format!("Sell item {} from character {}", item_id, cid),
                None => format!("Sell item {} from first character with it", item_id),
            },
            DialogueAction::OpenMerchant { npc_id } => {
                format!("Open merchant shop for '{}'", npc_id)
            }
            DialogueAction::ConsumeService {
                service_id,
                target_character_ids,
            } => {
                if target_character_ids.is_empty() {
                    format!("Consume service '{}' for whole party", service_id)
                } else {
                    format!(
                        "Consume service '{}' for {} character(s)",
                        service_id,
                        target_character_ids.len()
                    )
                }
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
        assert!(dialogue.sdk_metadata.managed_content.is_empty());
    }

    #[test]
    fn test_dialogue_choice_creation_sets_empty_sdk_metadata() {
        let choice = DialogueChoice::new("Tell me more", Some(5));
        assert!(choice.sdk_metadata.managed_content.is_empty());
    }

    #[test]
    fn test_standard_merchant_template_contains_open_merchant_and_metadata() {
        let tree = DialogueTree::standard_merchant_template(10, "merchant_tom", "Tom the Merchant");

        assert!(tree.contains_open_merchant_for_npc("merchant_tom"));
        assert!(tree.has_sdk_managed_merchant_content());
        assert!(tree
            .sdk_metadata
            .managed_content
            .contains(&DialogueSdkManagedContent::MerchantTemplateTree));
        assert_eq!(tree.root_node, 1);
        assert_eq!(tree.node_count(), 2);

        let root = tree.get_node(1).expect("root node should exist");
        assert_eq!(root.choice_count(), 2);
        assert!(root
            .choices
            .iter()
            .any(DialogueChoice::is_sdk_managed_merchant_choice));

        let merchant_node = tree.get_node(2).expect("merchant node should exist");
        assert!(merchant_node
            .sdk_metadata
            .managed_content
            .contains(&DialogueSdkManagedContent::MerchantOpenNode));
    }

    #[test]
    fn test_ensure_standard_merchant_branch_inserts_one_branch_for_existing_dialogue() {
        let mut tree = DialogueTree::new(20, "Custom".to_string(), 1);
        tree.add_node(DialogueNode::new(1, "Hello there."));

        let changed = tree.ensure_standard_merchant_branch("merchant_tom", "Tom the Merchant");

        assert!(changed);
        assert!(tree.contains_open_merchant_for_npc("merchant_tom"));
        assert!(tree.has_sdk_managed_merchant_content());
        assert!(tree
            .sdk_metadata
            .managed_content
            .contains(&DialogueSdkManagedContent::MerchantBranchInsertion));

        let root = tree.get_node(1).expect("root node should exist");
        assert_eq!(root.choice_count(), 1);
        assert!(root.choices[0].is_sdk_managed_merchant_choice());

        let merchant_node_id = root.choices[0]
            .target_node
            .expect("merchant choice should target merchant node");
        let merchant_node = tree
            .get_node(merchant_node_id)
            .expect("merchant node should exist");
        assert!(merchant_node
            .actions
            .iter()
            .any(|action| action.opens_merchant_for_npc("merchant_tom")));
    }

    #[test]
    fn test_ensure_standard_merchant_branch_is_noop_when_open_merchant_exists() {
        let mut tree = DialogueTree::new(21, "Custom".to_string(), 1);
        let mut root = DialogueNode::new(1, "Hello there.");
        let mut choice = DialogueChoice::new("Trade", Some(2));
        choice.add_action(DialogueAction::OpenMerchant {
            npc_id: "merchant_tom".to_string(),
        });
        root.add_choice(choice);
        tree.add_node(root);

        let changed = tree.ensure_standard_merchant_branch("merchant_tom", "Tom the Merchant");

        assert!(!changed);
        assert_eq!(tree.node_count(), 1);
        let root = tree.get_node(1).expect("root node should exist");
        assert_eq!(root.choice_count(), 1);
    }

    #[test]
    fn test_remove_sdk_managed_merchant_content_preserves_custom_nodes_and_choices() {
        let mut tree = DialogueTree::new(30, "Custom".to_string(), 1);
        let mut root = DialogueNode::new(1, "Hello there.");
        root.add_choice(DialogueChoice::new("Tell me more.", Some(3)));
        tree.add_node(root);
        tree.add_node(DialogueNode::new(3, "Here is more information."));

        let changed = tree.ensure_standard_merchant_branch("merchant_tom", "Tom the Merchant");
        assert!(changed);

        let removed = tree.remove_sdk_managed_merchant_content();

        assert!(removed);
        assert!(!tree.contains_open_merchant_for_npc("merchant_tom"));
        assert!(!tree.has_sdk_managed_merchant_content());

        let root = tree.get_node(1).expect("root node should still exist");
        assert_eq!(root.choice_count(), 1);
        assert_eq!(root.choices[0].text, "Tell me more.");
        assert!(tree.get_node(3).is_some(), "custom node should remain");
    }

    #[test]
    fn test_remove_sdk_managed_merchant_content_is_idempotent() {
        let mut tree =
            DialogueTree::standard_merchant_template(40, "merchant_tom", "Tom the Merchant");

        assert!(tree.remove_sdk_managed_merchant_content());
        assert!(!tree.remove_sdk_managed_merchant_content());
    }

    #[test]
    fn test_ensure_standard_merchant_branch_is_idempotent() {
        let mut tree = DialogueTree::new(41, "Custom".to_string(), 1);
        tree.add_node(DialogueNode::new(1, "Hello there."));

        assert!(tree.ensure_standard_merchant_branch("merchant_tom", "Tom the Merchant"));
        assert!(!tree.ensure_standard_merchant_branch("merchant_tom", "Tom the Merchant"));

        let root = tree.get_node(1).expect("root node should exist");
        assert_eq!(root.choice_count(), 1);
    }

    #[test]
    fn test_dialogue_action_opens_merchant_for_npc_matches_exact_id() {
        let action = DialogueAction::OpenMerchant {
            npc_id: "merchant_tom".to_string(),
        };

        assert!(action.opens_merchant_for_npc("merchant_tom"));
        assert!(!action.opens_merchant_for_npc("merchant_sue"));
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
    fn test_dialogue_action_open_inn_management_description() {
        let action = DialogueAction::OpenInnManagement {
            innkeeper_id: "innkeeper_town_01".to_string(),
        };
        assert_eq!(
            action.description(),
            "Open party management at inn (keeper: innkeeper_town_01)"
        );
    }

    #[test]
    fn test_dialogue_action_description_buy_item_no_target() {
        let action = DialogueAction::BuyItem {
            item_id: 1,
            target_character_id: None,
        };
        let desc = action.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("1"));
    }

    #[test]
    fn test_dialogue_action_description_buy_item_with_target() {
        let action = DialogueAction::BuyItem {
            item_id: 42,
            target_character_id: Some(2),
        };
        let desc = action.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("42"));
        assert!(desc.contains("2"));
    }

    #[test]
    fn test_dialogue_action_description_sell_item_no_source() {
        let action = DialogueAction::SellItem {
            item_id: 7,
            source_character_id: None,
        };
        let desc = action.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("7"));
    }

    #[test]
    fn test_dialogue_action_description_sell_item_with_source() {
        let action = DialogueAction::SellItem {
            item_id: 99,
            source_character_id: Some(0),
        };
        let desc = action.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("99"));
    }

    #[test]
    fn test_dialogue_action_description_open_merchant() {
        let action = DialogueAction::OpenMerchant {
            npc_id: "merchant_tom".to_string(),
        };
        let desc = action.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("merchant_tom"));
    }

    #[test]
    fn test_dialogue_action_description_consume_service_whole_party() {
        let action = DialogueAction::ConsumeService {
            service_id: "heal_all".to_string(),
            target_character_ids: vec![],
        };
        let desc = action.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("heal_all"));
        assert!(desc.contains("party"));
    }

    #[test]
    fn test_dialogue_action_description_consume_service_targeted() {
        let action = DialogueAction::ConsumeService {
            service_id: "resurrect".to_string(),
            target_character_ids: vec![0, 1],
        };
        let desc = action.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("resurrect"));
        assert!(desc.contains("2"));
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
