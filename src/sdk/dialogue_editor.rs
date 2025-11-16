// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Dialogue editor helper module
//!
//! This module provides tools for dialogue editing workflows including:
//! - Dialogue tree validation (node references, structure)
//! - Content browsing (quests, items)
//! - Smart ID suggestions
//! - Dialogue tree analysis
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sdk_and_campaign_architecture.md` Phase 5 for specifications.
//!
//! # Examples
//!
//! ```no_run
//! use antares::sdk::dialogue_editor::*;
//! use antares::sdk::database::ContentDatabase;
//! use antares::domain::dialogue::{DialogueTree, DialogueNode, DialogueChoice};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load content database
//! let db = ContentDatabase::load_core("data")?;
//!
//! // Create a dialogue tree
//! let mut dialogue = DialogueTree::new(1, "Merchant Conversation", 1);
//! dialogue.add_node(DialogueNode::new(1, "Hello, traveler!"));
//!
//! // Validate dialogue
//! let errors = validate_dialogue(&dialogue, &db);
//! if errors.is_empty() {
//!     println!("Dialogue is valid!");
//! }
//! # Ok(())
//! # }
//! ```

use crate::domain::dialogue::{
    DialogueAction, DialogueCondition, DialogueId, DialogueTree, NodeId,
};
use crate::domain::quest::QuestId;
use crate::domain::types::ItemId;
use crate::sdk::database::ContentDatabase;
use std::collections::{HashSet, VecDeque};

// ===== Content Browsing =====

/// Browse all available dialogues in the database
///
/// Returns a list of (DialogueId, name) tuples for all dialogues.
pub fn browse_dialogues(db: &ContentDatabase) -> Vec<(DialogueId, String)> {
    db.dialogues
        .all_dialogues()
        .iter()
        .map(|id| {
            let name = db
                .dialogues
                .get_dialogue(*id)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| format!("Dialogue {}", id));
            (*id, name)
        })
        .collect()
}

/// Browse all available quests in the database
///
/// Returns a list of (QuestId, name) tuples for all quests.
pub fn browse_quests(db: &ContentDatabase) -> Vec<(QuestId, String)> {
    db.quests
        .all_quests()
        .iter()
        .map(|id| {
            let name = db
                .quests
                .get_quest(*id)
                .map(|q| q.name.clone())
                .unwrap_or_else(|| format!("Quest {}", id));
            (*id, name)
        })
        .collect()
}

/// Browse all available items in the database
///
/// Returns a list of (ItemId, name) tuples for all items.
pub fn browse_items(db: &ContentDatabase) -> Vec<(ItemId, String)> {
    db.items
        .all_items()
        .iter()
        .map(|item| (item.id, item.name.clone()))
        .collect()
}

// ===== ID Validation =====

/// Check if a dialogue ID is valid
pub fn is_valid_dialogue_id(db: &ContentDatabase, dialogue_id: &DialogueId) -> bool {
    db.dialogues.has_dialogue(dialogue_id)
}

/// Check if a quest ID is valid
pub fn is_valid_quest_id(db: &ContentDatabase, quest_id: &QuestId) -> bool {
    db.quests.has_quest(quest_id)
}

/// Check if an item ID is valid
pub fn is_valid_item_id(db: &ContentDatabase, item_id: &ItemId) -> bool {
    db.items.has_item(item_id)
}

// ===== Smart ID Suggestions =====

/// Suggest dialogue IDs based on partial name match
pub fn suggest_dialogue_ids(db: &ContentDatabase, partial_name: &str) -> Vec<(DialogueId, String)> {
    let search = partial_name.to_lowercase();
    browse_dialogues(db)
        .into_iter()
        .filter(|(_, name)| name.to_lowercase().contains(&search))
        .collect()
}

/// Suggest quest IDs based on partial name match
pub fn suggest_quest_ids(db: &ContentDatabase, partial_name: &str) -> Vec<(QuestId, String)> {
    let search = partial_name.to_lowercase();
    browse_quests(db)
        .into_iter()
        .filter(|(_, name)| name.to_lowercase().contains(&search))
        .collect()
}

/// Suggest item IDs based on partial name match
pub fn suggest_item_ids(db: &ContentDatabase, partial_name: &str) -> Vec<(ItemId, String)> {
    let search = partial_name.to_lowercase();
    browse_items(db)
        .into_iter()
        .filter(|(_, name)| name.to_lowercase().contains(&search))
        .collect()
}

// ===== Dialogue Validation =====

/// Validation error for dialogues
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogueValidationError {
    /// Dialogue has no nodes
    NoNodes,

    /// Root node doesn't exist
    RootNodeMissing { root_node: NodeId },

    /// Choice references non-existent node
    InvalidChoiceTarget {
        source_node: NodeId,
        target_node: NodeId,
    },

    /// Node has no choices and is not marked as terminal
    NonTerminalNodeWithoutChoices { node_id: NodeId },

    /// Referenced quest ID doesn't exist
    InvalidQuestId { quest_id: QuestId },

    /// Referenced item ID doesn't exist
    InvalidItemId { item_id: ItemId },

    /// Orphaned node (unreachable from root)
    OrphanedNode { node_id: NodeId },

    /// Circular path detected (infinite loop possible)
    CircularPath { node_id: NodeId },

    /// Node marked as terminal but has choices
    TerminalNodeWithChoices { node_id: NodeId },

    /// Empty node text
    EmptyNodeText { node_id: NodeId },

    /// Choice has no text
    EmptyChoiceText {
        node_id: NodeId,
        choice_index: usize,
    },
}

impl std::fmt::Display for DialogueValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialogueValidationError::NoNodes => write!(f, "Dialogue has no nodes"),
            DialogueValidationError::RootNodeMissing { root_node } => {
                write!(f, "Root node {} doesn't exist", root_node)
            }
            DialogueValidationError::InvalidChoiceTarget {
                source_node,
                target_node,
            } => {
                write!(
                    f,
                    "Node {} has choice targeting non-existent node {}",
                    source_node, target_node
                )
            }
            DialogueValidationError::NonTerminalNodeWithoutChoices { node_id } => {
                write!(
                    f,
                    "Node {} has no choices and is not marked as terminal",
                    node_id
                )
            }
            DialogueValidationError::InvalidQuestId { quest_id } => {
                write!(f, "Invalid quest ID: {}", quest_id)
            }
            DialogueValidationError::InvalidItemId { item_id } => {
                write!(f, "Invalid item ID: {}", item_id)
            }
            DialogueValidationError::OrphanedNode { node_id } => {
                write!(f, "Node {} is orphaned (unreachable from root)", node_id)
            }
            DialogueValidationError::CircularPath { node_id } => {
                write!(f, "Circular path detected at node {}", node_id)
            }
            DialogueValidationError::TerminalNodeWithChoices { node_id } => {
                write!(f, "Node {} is marked as terminal but has choices", node_id)
            }
            DialogueValidationError::EmptyNodeText { node_id } => {
                write!(f, "Node {} has empty text", node_id)
            }
            DialogueValidationError::EmptyChoiceText {
                node_id,
                choice_index,
            } => {
                write!(f, "Node {} choice {} has empty text", node_id, choice_index)
            }
        }
    }
}

impl std::error::Error for DialogueValidationError {}

/// Validate a dialogue tree against the content database
///
/// Checks:
/// - Dialogue has at least one node
/// - Root node exists
/// - All choice targets exist
/// - No orphaned nodes
/// - Terminal nodes are properly marked
/// - Referenced IDs (quests, items) exist
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::dialogue_editor::validate_dialogue;
/// use antares::sdk::database::ContentDatabase;
/// use antares::domain::dialogue::{DialogueTree, DialogueNode, DialogueChoice};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let db = ContentDatabase::load_core("data")?;
/// let mut dialogue = DialogueTree::new(1, "Test", 1);
/// dialogue.add_node(DialogueNode::new(1, "Hello!"));
///
/// let errors = validate_dialogue(&dialogue, &db);
/// for error in errors {
///     println!("Error: {}", error);
/// }
/// # Ok(())
/// # }
/// ```
pub fn validate_dialogue(
    dialogue: &DialogueTree,
    db: &ContentDatabase,
) -> Vec<DialogueValidationError> {
    let mut errors = Vec::new();

    // Check dialogue has nodes
    if dialogue.nodes.is_empty() {
        errors.push(DialogueValidationError::NoNodes);
        return errors; // Early return
    }

    // Check root node exists
    if !dialogue.nodes.contains_key(&dialogue.root_node) {
        errors.push(DialogueValidationError::RootNodeMissing {
            root_node: dialogue.root_node,
        });
        return errors; // Can't continue validation without root
    }

    // Validate each node
    for (node_id, node) in &dialogue.nodes {
        // Check for empty text
        if node.text.trim().is_empty() {
            errors.push(DialogueValidationError::EmptyNodeText { node_id: *node_id });
        }

        // Check terminal nodes
        if node.is_terminal && !node.choices.is_empty() {
            errors.push(DialogueValidationError::TerminalNodeWithChoices { node_id: *node_id });
        }

        // Check non-terminal nodes have choices
        if !node.is_terminal && node.choices.is_empty() {
            errors
                .push(DialogueValidationError::NonTerminalNodeWithoutChoices { node_id: *node_id });
        }

        // Validate choices
        for (choice_idx, choice) in node.choices.iter().enumerate() {
            // Check for empty choice text
            if choice.text.trim().is_empty() {
                errors.push(DialogueValidationError::EmptyChoiceText {
                    node_id: *node_id,
                    choice_index: choice_idx,
                });
            }

            // Check choice target exists
            if let Some(target) = choice.target_node {
                if !dialogue.nodes.contains_key(&target) {
                    errors.push(DialogueValidationError::InvalidChoiceTarget {
                        source_node: *node_id,
                        target_node: target,
                    });
                }
            }

            // Validate choice conditions
            validate_conditions(&choice.conditions, db, &mut errors);

            // Validate choice actions
            validate_actions(&choice.actions, db, &mut errors);
        }

        // Validate node conditions
        validate_conditions(&node.conditions, db, &mut errors);

        // Validate node actions
        validate_actions(&node.actions, db, &mut errors);
    }

    // Check for orphaned nodes
    let reachable = get_reachable_nodes(dialogue);
    for node_id in dialogue.nodes.keys() {
        if !reachable.contains(node_id) {
            errors.push(DialogueValidationError::OrphanedNode { node_id: *node_id });
        }
    }

    // Check for circular paths (warning, not necessarily an error)
    if has_circular_path(dialogue) {
        // Circular paths are allowed (e.g., "Tell me more" loops back)
        // But we could add this as a warning in a future enhancement
    }

    errors
}

/// Helper function to validate conditions
fn validate_conditions(
    conditions: &[DialogueCondition],
    db: &ContentDatabase,
    errors: &mut Vec<DialogueValidationError>,
) {
    for condition in conditions {
        match condition {
            DialogueCondition::HasQuest { quest_id }
            | DialogueCondition::CompletedQuest { quest_id }
            | DialogueCondition::QuestStage { quest_id, .. } => {
                if !is_valid_quest_id(db, quest_id) {
                    errors.push(DialogueValidationError::InvalidQuestId {
                        quest_id: *quest_id,
                    });
                }
            }
            DialogueCondition::HasItem { item_id, .. } => {
                if !is_valid_item_id(db, item_id) {
                    errors.push(DialogueValidationError::InvalidItemId { item_id: *item_id });
                }
            }
            DialogueCondition::And(conds) | DialogueCondition::Or(conds) => {
                validate_conditions(conds, db, errors);
            }
            DialogueCondition::Not(cond) => {
                validate_conditions(&[*cond.clone()], db, errors);
            }
            _ => {
                // Other conditions don't need external validation
            }
        }
    }
}

/// Helper function to validate actions
fn validate_actions(
    actions: &[DialogueAction],
    db: &ContentDatabase,
    errors: &mut Vec<DialogueValidationError>,
) {
    for action in actions {
        match action {
            DialogueAction::StartQuest { quest_id }
            | DialogueAction::CompleteQuestStage { quest_id, .. } => {
                if !is_valid_quest_id(db, quest_id) {
                    errors.push(DialogueValidationError::InvalidQuestId {
                        quest_id: *quest_id,
                    });
                }
            }
            DialogueAction::GiveItems { items } | DialogueAction::TakeItems { items } => {
                for (item_id, _) in items {
                    if !is_valid_item_id(db, item_id) {
                        errors.push(DialogueValidationError::InvalidItemId { item_id: *item_id });
                    }
                }
            }
            _ => {
                // Other actions don't need external validation
            }
        }
    }
}

/// Get all nodes reachable from the root node
fn get_reachable_nodes(dialogue: &DialogueTree) -> HashSet<NodeId> {
    let mut reachable = HashSet::new();
    let mut to_visit = VecDeque::new();

    to_visit.push_back(dialogue.root_node);

    while let Some(node_id) = to_visit.pop_front() {
        if reachable.contains(&node_id) {
            continue;
        }
        reachable.insert(node_id);

        if let Some(node) = dialogue.nodes.get(&node_id) {
            for choice in &node.choices {
                if let Some(target) = choice.target_node {
                    if !reachable.contains(&target) {
                        to_visit.push_back(target);
                    }
                }
            }
        }
    }

    reachable
}

/// Check if the dialogue tree has any circular paths
fn has_circular_path(dialogue: &DialogueTree) -> bool {
    fn visit(
        node_id: NodeId,
        dialogue: &DialogueTree,
        visited: &mut HashSet<NodeId>,
        rec_stack: &mut HashSet<NodeId>,
    ) -> bool {
        visited.insert(node_id);
        rec_stack.insert(node_id);

        if let Some(node) = dialogue.nodes.get(&node_id) {
            for choice in &node.choices {
                if let Some(target) = choice.target_node {
                    if !visited.contains(&target) {
                        if visit(target, dialogue, visited, rec_stack) {
                            return true;
                        }
                    } else if rec_stack.contains(&target) {
                        return true;
                    }
                }
            }
        }

        rec_stack.remove(&node_id);
        false
    }

    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    visit(dialogue.root_node, dialogue, &mut visited, &mut rec_stack)
}

/// Analyze dialogue tree structure
///
/// Returns statistics about the dialogue tree.
#[derive(Debug, Clone)]
pub struct DialogueStats {
    /// Total number of nodes
    pub node_count: usize,

    /// Total number of choices across all nodes
    pub choice_count: usize,

    /// Number of terminal nodes
    pub terminal_node_count: usize,

    /// Number of orphaned nodes
    pub orphaned_node_count: usize,

    /// Maximum depth from root node
    pub max_depth: usize,

    /// Number of nodes with conditions
    pub conditional_node_count: usize,

    /// Number of nodes with actions
    pub action_node_count: usize,
}

/// Analyze a dialogue tree and return statistics
///
/// # Examples
///
/// ```
/// use antares::sdk::dialogue_editor::analyze_dialogue;
/// use antares::domain::dialogue::{DialogueTree, DialogueNode, DialogueChoice};
///
/// let mut dialogue = DialogueTree::new(1, "Test", 1);
/// dialogue.add_node(DialogueNode::new(1, "Hello!"));
///
/// let stats = analyze_dialogue(&dialogue);
/// assert_eq!(stats.node_count, 1);
/// ```
pub fn analyze_dialogue(dialogue: &DialogueTree) -> DialogueStats {
    let node_count = dialogue.nodes.len();
    let choice_count: usize = dialogue.nodes.values().map(|n| n.choices.len()).sum();
    let terminal_node_count = dialogue.nodes.values().filter(|n| n.is_terminal).count();

    let reachable = get_reachable_nodes(dialogue);
    let orphaned_node_count = node_count - reachable.len();

    let max_depth = calculate_max_depth(dialogue);

    let conditional_node_count = dialogue
        .nodes
        .values()
        .filter(|n| !n.conditions.is_empty())
        .count();

    let action_node_count = dialogue
        .nodes
        .values()
        .filter(|n| !n.actions.is_empty())
        .count();

    DialogueStats {
        node_count,
        choice_count,
        terminal_node_count,
        orphaned_node_count,
        max_depth,
        conditional_node_count,
        action_node_count,
    }
}

/// Calculate maximum depth from root node
fn calculate_max_depth(dialogue: &DialogueTree) -> usize {
    fn dfs(
        node_id: NodeId,
        dialogue: &DialogueTree,
        visited: &mut HashSet<NodeId>,
        depth: usize,
    ) -> usize {
        if visited.contains(&node_id) {
            return depth;
        }
        visited.insert(node_id);

        let mut max_depth = depth;

        if let Some(node) = dialogue.nodes.get(&node_id) {
            for choice in &node.choices {
                if let Some(target) = choice.target_node {
                    let child_depth = dfs(target, dialogue, visited, depth + 1);
                    max_depth = max_depth.max(child_depth);
                }
            }
        }

        visited.remove(&node_id);
        max_depth
    }

    let mut visited = HashSet::new();
    dfs(dialogue.root_node, dialogue, &mut visited, 0)
}

/// Generate dialogue summary for display
pub fn generate_dialogue_summary(dialogue: &DialogueTree) -> String {
    let mut summary = String::new();

    summary.push_str(&format!("Dialogue {}: {}\n", dialogue.id, dialogue.name));

    if let Some(speaker) = &dialogue.speaker_name {
        summary.push_str(&format!("Speaker: {}\n", speaker));
    }

    summary.push_str(&format!("Root Node: {}\n", dialogue.root_node));

    if dialogue.repeatable {
        summary.push_str("Repeatable: Yes\n");
    } else {
        summary.push_str("Repeatable: No\n");
    }

    if let Some(quest_id) = dialogue.associated_quest {
        summary.push_str(&format!("Associated Quest: {}\n", quest_id));
    }

    let stats = analyze_dialogue(dialogue);
    summary.push_str(&format!("Total Nodes: {}\n", stats.node_count));
    summary.push_str(&format!("Total Choices: {}\n", stats.choice_count));
    summary.push_str(&format!("Terminal Nodes: {}\n", stats.terminal_node_count));
    summary.push_str(&format!("Max Depth: {}\n", stats.max_depth));

    if stats.orphaned_node_count > 0 {
        summary.push_str(&format!(
            "Warning: {} orphaned nodes\n",
            stats.orphaned_node_count
        ));
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dialogue::{DialogueChoice, DialogueNode};

    #[test]
    fn test_validate_dialogue_no_nodes() {
        let db = ContentDatabase::new();
        let dialogue = DialogueTree::new(1, "Test", 1);

        let errors = validate_dialogue(&dialogue, &db);
        assert!(!errors.is_empty());
        assert!(matches!(errors[0], DialogueValidationError::NoNodes));
    }

    #[test]
    fn test_validate_dialogue_missing_root() {
        let db = ContentDatabase::new();
        let mut dialogue = DialogueTree::new(1, "Test", 99); // Root node doesn't exist
        dialogue.add_node(DialogueNode::new(1, "Hello"));

        let errors = validate_dialogue(&dialogue, &db);
        assert!(errors.iter().any(|e| matches!(
            e,
            DialogueValidationError::RootNodeMissing { root_node: 99 }
        )));
    }

    #[test]
    fn test_validate_dialogue_invalid_choice_target() {
        let db = ContentDatabase::new();
        let mut dialogue = DialogueTree::new(1, "Test", 1);
        let mut node = DialogueNode::new(1, "Hello");
        node.add_choice(DialogueChoice::new("Go to missing node", Some(99)));
        dialogue.add_node(node);

        let errors = validate_dialogue(&dialogue, &db);
        assert!(errors.iter().any(|e| matches!(
            e,
            DialogueValidationError::InvalidChoiceTarget {
                source_node: 1,
                target_node: 99
            }
        )));
    }

    #[test]
    fn test_validate_dialogue_non_terminal_without_choices() {
        let db = ContentDatabase::new();
        let mut dialogue = DialogueTree::new(1, "Test", 1);
        let node = DialogueNode::new(1, "Hello"); // Not terminal, no choices
        dialogue.add_node(node);

        let errors = validate_dialogue(&dialogue, &db);
        assert!(errors.iter().any(|e| matches!(
            e,
            DialogueValidationError::NonTerminalNodeWithoutChoices { node_id: 1 }
        )));
    }

    #[test]
    fn test_validate_dialogue_terminal_with_choices() {
        let db = ContentDatabase::new();
        let mut dialogue = DialogueTree::new(1, "Test", 1);
        let mut node = DialogueNode::new(1, "Hello");
        node.is_terminal = true;
        node.add_choice(DialogueChoice::new("This shouldn't be here", None));
        dialogue.add_node(node);

        let errors = validate_dialogue(&dialogue, &db);
        assert!(errors.iter().any(|e| matches!(
            e,
            DialogueValidationError::TerminalNodeWithChoices { node_id: 1 }
        )));
    }

    #[test]
    fn test_validate_dialogue_empty_text() {
        let db = ContentDatabase::new();
        let mut dialogue = DialogueTree::new(1, "Test", 1);
        let mut node = DialogueNode::new(1, "   "); // Empty after trim
        node.is_terminal = true;
        dialogue.add_node(node);

        let errors = validate_dialogue(&dialogue, &db);
        assert!(errors
            .iter()
            .any(|e| matches!(e, DialogueValidationError::EmptyNodeText { node_id: 1 })));
    }

    #[test]
    fn test_validate_valid_dialogue() {
        let db = ContentDatabase::new();
        let mut dialogue = DialogueTree::new(1, "Test", 1);
        let mut node = DialogueNode::new(1, "Hello!");
        node.is_terminal = true;
        dialogue.add_node(node);

        let errors = validate_dialogue(&dialogue, &db);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_get_reachable_nodes() {
        let mut dialogue = DialogueTree::new(1, "Test", 1);
        let mut node1 = DialogueNode::new(1, "Node 1");
        node1.add_choice(DialogueChoice::new("Go to 2", Some(2)));
        let mut node2 = DialogueNode::new(2, "Node 2");
        node2.is_terminal = true;
        let mut node3 = DialogueNode::new(3, "Orphan");
        node3.is_terminal = true;

        dialogue.add_node(node1);
        dialogue.add_node(node2);
        dialogue.add_node(node3);

        let reachable = get_reachable_nodes(&dialogue);
        assert_eq!(reachable.len(), 2); // Only nodes 1 and 2
        assert!(reachable.contains(&1));
        assert!(reachable.contains(&2));
        assert!(!reachable.contains(&3));
    }

    #[test]
    fn test_analyze_dialogue() {
        let mut dialogue = DialogueTree::new(1, "Test", 1);
        let mut node1 = DialogueNode::new(1, "Hello");
        node1.add_choice(DialogueChoice::new("Continue", Some(2)));
        let mut node2 = DialogueNode::new(2, "Goodbye");
        node2.is_terminal = true;

        dialogue.add_node(node1);
        dialogue.add_node(node2);

        let stats = analyze_dialogue(&dialogue);
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.choice_count, 1);
        assert_eq!(stats.terminal_node_count, 1);
        assert_eq!(stats.orphaned_node_count, 0);
    }

    #[test]
    fn test_has_circular_path() {
        let mut dialogue = DialogueTree::new(1, "Test", 1);
        let mut node1 = DialogueNode::new(1, "Node 1");
        node1.add_choice(DialogueChoice::new("Go to 2", Some(2)));
        let mut node2 = DialogueNode::new(2, "Node 2");
        node2.add_choice(DialogueChoice::new("Back to 1", Some(1))); // Circle

        dialogue.add_node(node1);
        dialogue.add_node(node2);

        assert!(has_circular_path(&dialogue));
    }

    #[test]
    fn test_generate_dialogue_summary() {
        let mut dialogue = DialogueTree::new(1, "Merchant Talk", 1);
        dialogue.speaker_name = Some("Merchant".to_string());
        dialogue.associated_quest = Some(10);

        let mut node = DialogueNode::new(1, "Welcome!");
        node.is_terminal = true;
        dialogue.add_node(node);

        let summary = generate_dialogue_summary(&dialogue);
        assert!(summary.contains("Merchant Talk"));
        assert!(summary.contains("Speaker: Merchant"));
        assert!(summary.contains("Associated Quest: 10"));
    }

    #[test]
    fn test_browse_empty_database() {
        let db = ContentDatabase::new();
        assert_eq!(browse_dialogues(&db).len(), 0);
        assert_eq!(browse_quests(&db).len(), 0);
        assert_eq!(browse_items(&db).len(), 0);
    }
}
