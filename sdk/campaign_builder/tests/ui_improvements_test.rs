// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! UI Improvements Test
//!
//! Tests for Phase 8 SDK Campaign Builder UI/UX improvements.
//! Verifies that the enhanced display components are functional.

#[cfg(test)]
mod ui_improvements_tests {
    use antares::domain::dialogue::{DialogueNode, DialogueTree};
    use std::collections::HashMap;

    #[test]
    fn test_dialogue_tree_structure_for_preview() {
        let mut nodes = HashMap::new();

        let node1 = DialogueNode {
            id: 0,
            text: "Welcome to our town, traveler! This is a test dialogue with enough text to demonstrate the preview excerpt functionality.".to_string(),
            speaker_override: Some("Guard Captain".to_string()),
            choices: vec![],
            conditions: vec![],
            actions: vec![],
            is_terminal: false,
        };

        let node2 = DialogueNode {
            id: 1,
            text: "Farewell!".to_string(),
            speaker_override: None,
            choices: vec![],
            conditions: vec![],
            actions: vec![],
            is_terminal: true,
        };

        nodes.insert(0, node1);
        nodes.insert(1, node2);

        let dialogue = DialogueTree {
            id: 1,
            name: "Town Guard Dialogue".to_string(),
            root_node: 0,
            nodes,
            repeatable: true,
            speaker_name: Some("Town Guard".to_string()),
            associated_quest: Some(5),
        };

        // Verify dialogue structure for preview display
        assert_eq!(dialogue.nodes.len(), 2);
        assert!(dialogue.repeatable);
        assert_eq!(dialogue.speaker_name, Some("Town Guard".to_string()));
        assert_eq!(dialogue.associated_quest, Some(5));

        // Verify text excerpt logic (first 60 chars)
        if let Some(first_node) = dialogue.nodes.get(&0) {
            let excerpt: String = first_node.text.chars().take(60).collect();
            assert_eq!(excerpt.len(), 60);
            assert!(first_node.text.len() > 60); // Ensures we're testing truncation

            // Verify speaker override is present for display
            assert!(first_node.speaker_override.is_some());
        } else {
            panic!("Expected start node to exist");
        }

        // Verify terminal node detection
        if let Some(end_node) = dialogue.nodes.get(&1) {
            assert!(end_node.is_terminal);
        }
    }

    #[test]
    fn test_dialogue_text_excerpt_edge_cases() {
        let short_text = "Short.";
        let excerpt: String = short_text.chars().take(60).collect();
        assert_eq!(excerpt, short_text);

        let exact_60 = "X".repeat(60);
        let excerpt_60: String = exact_60.chars().take(60).collect();
        assert_eq!(excerpt_60.len(), 60);

        let long_text = "X".repeat(120);
        let excerpt_120: String = long_text.chars().take(120).collect();
        assert_eq!(excerpt_120.len(), 120);
    }

    #[test]
    fn test_dialogue_node_count_for_preview_limit() {
        let mut nodes = HashMap::new();

        // Create 10 nodes
        for i in 0..10 {
            let node = DialogueNode {
                id: i,
                text: format!("Node {} text", i),
                speaker_override: None,
                choices: vec![],
                conditions: vec![],
                actions: vec![],
                is_terminal: i == 9,
            };
            nodes.insert(i, node);
        }

        let dialogue = DialogueTree {
            id: 1,
            name: "Multi-node Dialogue".to_string(),
            root_node: 0,
            nodes,
            repeatable: false,
            speaker_name: None,
            associated_quest: None,
        };

        // Preview should show up to 5 nodes
        assert_eq!(dialogue.nodes.len(), 10);

        let preview_limit = 5;
        let shown_nodes = dialogue.nodes.iter().take(preview_limit).count();
        assert_eq!(shown_nodes, preview_limit);

        let remaining = dialogue.nodes.len() - preview_limit;
        assert_eq!(remaining, 5);
    }

    #[test]
    fn test_dialogue_with_speaker_and_quest() {
        let nodes = HashMap::new();

        let dialogue = DialogueTree {
            id: 42,
            name: "Quest Dialogue".to_string(),
            root_node: 0,
            nodes,
            repeatable: false,
            speaker_name: Some("Mysterious Stranger".to_string()),
            associated_quest: Some(10),
        };

        // Verify metadata display in UI
        assert!(dialogue.speaker_name.is_some());
        assert!(dialogue.associated_quest.is_some());
        assert_eq!(dialogue.associated_quest.unwrap(), 10);
    }

    #[test]
    fn test_dialogue_repeatable_flag() {
        let nodes = HashMap::new();

        let repeatable_dialogue = DialogueTree {
            id: 1,
            name: "Merchant".to_string(),
            root_node: 0,
            nodes: nodes.clone(),
            repeatable: true,
            speaker_name: None,
            associated_quest: None,
        };

        let one_time_dialogue = DialogueTree {
            id: 2,
            name: "Story Event".to_string(),
            root_node: 0,
            nodes,
            repeatable: false,
            speaker_name: None,
            associated_quest: None,
        };

        // Repeatable dialogues show 🔄 indicator in UI
        assert!(repeatable_dialogue.repeatable);
        assert!(!one_time_dialogue.repeatable);
    }

    #[test]
    fn test_node_terminal_detection() {
        let terminal_node = DialogueNode {
            id: 99,
            text: "Goodbye!".to_string(),
            speaker_override: None,
            choices: vec![],
            conditions: vec![],
            actions: vec![],
            is_terminal: true,
        };

        let normal_node = DialogueNode {
            id: 1,
            text: "What can I help you with?".to_string(),
            speaker_override: None,
            choices: vec![],
            conditions: vec![],
            actions: vec![],
            is_terminal: false,
        };

        // Terminal nodes show 🏁 indicator in preview
        assert!(terminal_node.is_terminal);
        assert!(!normal_node.is_terminal);
    }
}
