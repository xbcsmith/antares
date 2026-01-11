// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Dialogue tree validation utilities
//!
//! This module provides validation functions to detect common errors in dialogue trees,
//! including:
//! - Missing root nodes
//! - Invalid node references in choices
//! - Orphaned nodes unreachable from root
//! - Circular references
//!
//! These checks are useful both at load time and for debugging dialogue authoring issues.

use crate::domain::dialogue::{DialogueTree, NodeId};
use bevy::log::info;
use std::collections::{HashSet, VecDeque};

/// Result type for dialogue validation operations
pub type ValidationResult = Result<(), String>;

/// Validates a dialogue tree for common errors
///
/// Performs the following checks:
/// - Root node exists in the tree
/// - All nodes referenced by choices exist
/// - No circular references between nodes
/// - All nodes are reachable from root (no orphaned nodes)
///
/// # Arguments
///
/// * `tree` - The dialogue tree to validate
///
/// # Returns
///
/// Returns `Ok(())` if valid, `Err(message)` with a description if invalid
///
/// # Examples
///
/// ```no_run
/// use antares::domain::dialogue::DialogueTree;
/// use antares::game::systems::dialogue_validation::validate_dialogue_tree;
///
/// let tree = DialogueTree::new(1, "Test Dialogue", 1);
/// match validate_dialogue_tree(&tree) {
///     Ok(()) => println!("Tree is valid"),
///     Err(e) => eprintln!("Validation error: {}", e),
/// }
/// ```
pub fn validate_dialogue_tree(tree: &DialogueTree) -> ValidationResult {
    // Check 1: Root node exists
    if tree.get_node(tree.root_node).is_none() {
        return Err(format!(
            "Root node {} not found in dialogue tree {}",
            tree.root_node, tree.id
        ));
    }

    // Check 2: All choice targets exist
    for (node_id, node) in &tree.nodes {
        for (choice_idx, choice) in node.choices.iter().enumerate() {
            if let Some(target_node) = choice.target_node {
                if tree.get_node(target_node).is_none() {
                    return Err(format!(
                        "Choice {} in node {} references non-existent target node {}",
                        choice_idx, node_id, target_node
                    ));
                }
            }
        }
    }

    // Check 3: Detect circular references using cycle detection
    detect_cycles(tree)?;

    // Check 4: Find orphaned nodes (unreachable from root)
    let reachable = find_reachable_nodes(tree);
    let defined: HashSet<NodeId> = tree.nodes.keys().copied().collect();
    let orphaned: Vec<NodeId> = defined.difference(&reachable).copied().collect();

    if !orphaned.is_empty() {
        // Log warning but don't fail - orphaned nodes are not critical
        info!(
            "Dialogue {} has orphaned nodes (unreachable from root): {:?}",
            tree.id, orphaned
        );
    }

    Ok(())
}

/// Detects cycles in the dialogue tree using depth-first search
///
/// Returns an error description if a cycle is found
fn detect_cycles(tree: &DialogueTree) -> ValidationResult {
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    for node_id in tree.nodes.keys() {
        if !visited.contains(node_id) && dfs_has_cycle(tree, *node_id, &mut visited, &mut rec_stack)
        {
            return Err(format!(
                "Circular reference detected in dialogue tree {} starting from node {}",
                tree.id, node_id
            ));
        }
    }

    Ok(())
}

/// DFS helper for cycle detection
fn dfs_has_cycle(
    tree: &DialogueTree,
    node_id: NodeId,
    visited: &mut HashSet<NodeId>,
    rec_stack: &mut HashSet<NodeId>,
) -> bool {
    visited.insert(node_id);
    rec_stack.insert(node_id);

    if let Some(node) = tree.get_node(node_id) {
        for choice in &node.choices {
            if let Some(target_id) = choice.target_node {
                if !visited.contains(&target_id) {
                    if dfs_has_cycle(tree, target_id, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(&target_id) {
                    return true; // Back edge found - cycle detected
                }
            }
        }
    }

    rec_stack.remove(&node_id);
    false
}

/// Finds all nodes reachable from the root using BFS
fn find_reachable_nodes(tree: &DialogueTree) -> HashSet<NodeId> {
    let mut reachable = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back(tree.root_node);
    reachable.insert(tree.root_node);

    while let Some(node_id) = queue.pop_front() {
        if let Some(node) = tree.get_node(node_id) {
            for choice in &node.choices {
                if let Some(target_id) = choice.target_node {
                    if !reachable.contains(&target_id) {
                        reachable.insert(target_id);
                        queue.push_back(target_id);
                    }
                }
            }
        }
    }

    reachable
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dialogue::{DialogueChoice, DialogueNode};

    #[test]
    fn test_validates_missing_root_node() {
        let tree = DialogueTree::new(1, "Test", 999); // Root node 999 doesn't exist
        let result = validate_dialogue_tree(&tree);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Root node 999 not found"));
    }

    #[test]
    fn test_validates_invalid_choice_target() {
        let mut tree = DialogueTree::new(1, "Test", 1);
        let mut node = DialogueNode::new(1, "Hello");
        node.choices.push(DialogueChoice {
            text: "Invalid".to_string(),
            target_node: Some(999), // Doesn't exist
            conditions: vec![],
            actions: vec![],
            ends_dialogue: false,
        });
        tree.add_node(node);

        let result = validate_dialogue_tree(&tree);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non-existent target node 999"));
    }

    #[test]
    fn test_validates_correct_tree() {
        let mut tree = DialogueTree::new(1, "Test", 1);
        tree.add_node(DialogueNode::new(1, "Hello"));
        let result = validate_dialogue_tree(&tree);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validates_tree_with_valid_choices() {
        let mut tree = DialogueTree::new(1, "Test", 1);
        tree.add_node(DialogueNode::new(1, "Hello"));

        let node2 = DialogueNode::new(2, "Goodbye");
        tree.add_node(node2);

        let mut node1 = DialogueNode::new(1, "Hello");
        node1.choices.push(DialogueChoice {
            text: "Say goodbye".to_string(),
            target_node: Some(2),
            conditions: vec![],
            actions: vec![],
            ends_dialogue: false,
        });
        tree.add_node(node1);

        let result = validate_dialogue_tree(&tree);
        assert!(result.is_ok());
    }

    #[test]
    fn test_detects_circular_references() {
        let mut tree = DialogueTree::new(1, "Test", 1);

        let mut node1 = DialogueNode::new(1, "Node 1");
        node1.choices.push(DialogueChoice {
            text: "Go to 2".to_string(),
            target_node: Some(2),
            conditions: vec![],
            actions: vec![],
            ends_dialogue: false,
        });
        tree.add_node(node1);

        let mut node2 = DialogueNode::new(2, "Node 2");
        node2.choices.push(DialogueChoice {
            text: "Go to 3".to_string(),
            target_node: Some(3),
            conditions: vec![],
            actions: vec![],
            ends_dialogue: false,
        });
        tree.add_node(node2);

        let mut node3 = DialogueNode::new(3, "Node 3");
        node3.choices.push(DialogueChoice {
            text: "Go back to 1".to_string(),
            target_node: Some(1), // Creates cycle
            conditions: vec![],
            actions: vec![],
            ends_dialogue: false,
        });
        tree.add_node(node3);

        let result = validate_dialogue_tree(&tree);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Circular reference"));
    }

    #[test]
    fn test_identifies_orphaned_nodes() {
        let mut tree = DialogueTree::new(1, "Test", 1);

        // Root node
        tree.add_node(DialogueNode::new(1, "Root"));

        // Orphaned node (not reachable from root)
        tree.add_node(DialogueNode::new(999, "Orphaned"));

        // Validation should succeed (orphaned nodes are warnings, not errors)
        let result = validate_dialogue_tree(&tree);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_choices_are_valid() {
        let mut tree = DialogueTree::new(1, "Test", 1);
        tree.add_node(DialogueNode::new(1, "Root node"));
        let result = validate_dialogue_tree(&tree);
        // Tree with root node and no choices is valid
        assert!(result.is_ok());
    }
}
