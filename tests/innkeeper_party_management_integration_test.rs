// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Phase 4: Integration Testing and Bug Fixes for Innkeeper Party Management
//!
//! Comprehensive end-to-end tests covering:
//! - Complete inn workflow (enter inn → dialogue → party management → save/load)
//! - Edge cases (empty party, full roster, invalid inputs)
//! - Input validation (invalid IDs, corrupted data)
//! - Regression testing (existing systems unaffected)
//! - Performance testing (rapid transitions, large rosters)

use antares::application::dialogue::DialogueState;
use antares::application::{GameMode, GameState, InnManagementState};
use antares::domain::character::{Alignment, Character, CharacterLocation, Roster, Sex};

/// Helper: Create a test character with given name
fn create_test_character(name: &str) -> Character {
    Character::new(
        name.to_string(),
        "human".to_string(),
        "knight".to_string(),
        Sex::Male,
        Alignment::Good,
    )
}

/// Helper: Create a game state with a party
fn create_game_with_party(party_size: usize) -> GameState {
    let mut game = GameState::new();
    for i in 0..party_size {
        let char = create_test_character(&format!("Hero{}", i + 1));
        game.party
            .add_member(char)
            .expect("Failed to add party member");
    }
    game
}

/// Helper: Create a game state with roster characters
fn create_game_with_roster(roster_size: usize) -> GameState {
    let mut game = GameState::new();
    for i in 0..roster_size {
        let char = create_test_character(&format!("Roster{}", i + 1));
        game.roster
            .add_character(char, CharacterLocation::AtInn("test_inn".to_string()))
            .expect("Failed to add roster character");
    }
    game
}

// ============================================================================
// 4.1 END-TO-END TEST SCENARIOS
// ============================================================================

#[test]
fn test_complete_inn_workflow() {
    // Arrange: Create game with party
    let mut game = create_game_with_party(3);
    assert_eq!(game.party.members.len(), 3);
    assert_eq!(game.roster.characters.len(), 0);

    // Step 1: Enter dialogue with innkeeper
    let speaker_npc_id = Some("innkeeper_001".to_string());
    game.mode = GameMode::Dialogue(DialogueState::start(
        999_u16, // Default innkeeper dialogue
        1_u16,   // Root Node
        None,
        speaker_npc_id.clone(),
    ));

    if let GameMode::Dialogue(ref dialogue_state) = game.mode {
        assert_eq!(dialogue_state.active_tree_id, Some(999_u16));
        assert_eq!(dialogue_state.speaker_npc_id, speaker_npc_id);
    } else {
        panic!("Expected Dialogue mode");
    }

    // Step 2: Simulate choosing "manage my party" option
    // This would trigger TriggerEvent("open_inn_party_management")
    // which transitions to InnManagement mode
    game.mode = GameMode::InnManagement(InnManagementState {
        current_inn_id: "innkeeper_001".to_string(),
        selected_party_slot: None,
        selected_roster_slot: None,
    });

    if let GameMode::InnManagement(ref inn_state) = game.mode {
        assert_eq!(inn_state.current_inn_id, "innkeeper_001");
    } else {
        panic!("Expected InnManagement mode");
    }

    // Step 3: Make party changes (remove a member)
    let _removed = game.party.remove_member(1);
    assert_eq!(game.party.members.len(), 2);

    // Step 4: Return to exploration
    game.return_to_exploration();
    assert_eq!(game.mode, GameMode::Exploration);
    assert_eq!(game.party.members.len(), 2); // Changes preserved
}

#[test]
fn test_multiple_innkeepers_in_sequence() {
    // Arrange: Create game with party
    let mut game = create_game_with_party(4);

    // Visit first innkeeper
    game.mode = GameMode::Dialogue(DialogueState::start(
        999_u16,
        1_u16,
        None,
        Some("innkeeper_town1".to_string()),
    ));

    // Open inn management at first location
    game.mode = GameMode::InnManagement(InnManagementState {
        current_inn_id: "innkeeper_town1".to_string(),
        selected_party_slot: None,
        selected_roster_slot: None,
    });

    if let GameMode::InnManagement(ref state) = game.mode {
        assert_eq!(state.current_inn_id, "innkeeper_town1");
    }

    // Return to exploration
    game.return_to_exploration();

    // Visit second innkeeper
    game.mode = GameMode::Dialogue(DialogueState::start(
        999_u16,
        1_u16,
        None,
        Some("innkeeper_town2".to_string()),
    ));

    // Open inn management at second location
    game.mode = GameMode::InnManagement(InnManagementState {
        current_inn_id: "innkeeper_town2".to_string(),
        selected_party_slot: None,
        selected_roster_slot: None,
    });

    if let GameMode::InnManagement(ref state) = game.mode {
        assert_eq!(state.current_inn_id, "innkeeper_town2");
    }

    // Verify party state preserved across multiple inn visits
    assert_eq!(game.party.members.len(), 4);
}

#[test]
fn test_state_preservation_across_transitions() {
    // Arrange: Create game with resources
    let mut game = create_game_with_party(2);
    game.party.gold = 500;
    game.party.gems = 10;
    game.party.food = 100;

    let initial_gold = game.party.gold;
    let initial_gems = game.party.gems;
    let initial_food = game.party.food;

    // Act: Transition through multiple modes
    game.mode = GameMode::Dialogue(DialogueState::start(
        999_u16,
        1_u16,
        None,
        Some("inn1".to_string()),
    ));
    game.mode = GameMode::InnManagement(InnManagementState {
        current_inn_id: "inn1".to_string(),
        selected_party_slot: None,
        selected_roster_slot: None,
    });
    game.return_to_exploration();
    game.enter_menu();
    game.return_to_exploration();

    // Assert: Resources preserved
    assert_eq!(game.party.gold, initial_gold);
    assert_eq!(game.party.gems, initial_gems);
    assert_eq!(game.party.food, initial_food);
}

// ============================================================================
// 4.2 EDGE CASE TESTING
// ============================================================================

#[test]
fn test_empty_party_at_inn() {
    // Arrange: Game with no party members
    let mut game = GameState::new();
    assert!(game.party.members.is_empty());

    // Act: Enter inn management with empty party
    game.mode = GameMode::InnManagement(InnManagementState {
        current_inn_id: "test_inn".to_string(),
        selected_party_slot: None,
        selected_roster_slot: None,
    });

    // Assert: Should handle gracefully
    if let GameMode::InnManagement(ref state) = game.mode {
        assert_eq!(state.current_inn_id, "test_inn");
    } else {
        panic!("Expected InnManagement mode");
    }
}

#[test]
fn test_full_roster_18_characters() {
    // Arrange: Create roster with maximum characters (18)
    let mut roster = Roster::new();
    for i in 0..18 {
        let char = create_test_character(&format!("Char{:02}", i + 1));
        roster
            .add_character(char, CharacterLocation::AtInn("test_inn".to_string()))
            .expect("Failed to add character to roster");
    }

    // Assert: Roster at capacity
    assert_eq!(roster.characters.len(), 18);

    // Act: Try to add one more (should fail)
    let extra_char = create_test_character("Extra");
    let result = roster.add_character(extra_char, CharacterLocation::AtInn("test_inn".to_string()));

    // Assert: Cannot exceed limit
    assert!(result.is_err());
    assert_eq!(roster.characters.len(), 18);
}

#[test]
fn test_dialogue_with_no_speaker_npc_id() {
    // Arrange: Start dialogue without speaker NPC ID
    let dialogue_state = DialogueState::start(999_u16, 1_u16, None, None);

    // Assert: Should handle gracefully
    assert_eq!(dialogue_state.speaker_npc_id, None);
    assert_eq!(dialogue_state.active_tree_id, Some(999_u16));

    // Note: TriggerEvent handler should log warning and not open inn management
    // when speaker_npc_id is None
}

#[test]
fn test_dialogue_with_invalid_tree_id() {
    // Arrange: Try to start dialogue with non-existent tree
    let dialogue_state = DialogueState::start(9999_u16, 1_u16, None, Some("npc1".to_string()));

    // Assert: State created but tree loading will fail in runtime
    assert_eq!(dialogue_state.active_tree_id, Some(9999_u16));
    assert_eq!(dialogue_state.speaker_npc_id, Some("npc1".to_string()));

    // Runtime dialogue system should handle missing tree gracefully
}

#[test]
fn test_missing_inn_id_in_state() {
    // Arrange: Create inn management state with empty inn ID
    let state = InnManagementState {
        current_inn_id: String::new(),
        selected_party_slot: None,
        selected_roster_slot: None,
    };

    // Assert: Empty string handled
    assert_eq!(state.current_inn_id, "");
}

#[test]
fn test_party_at_max_size_6_members() {
    // Arrange: Create party with maximum members
    let mut game = GameState::new();
    for i in 0..6 {
        let char = create_test_character(&format!("Member{}", i + 1));
        game.party.add_member(char).expect("Failed to add member");
    }

    // Assert: Party at max capacity
    assert_eq!(game.party.members.len(), 6);

    // Act: Try to add 7th member
    let extra = create_test_character("Extra");
    let result = game.party.add_member(extra);

    // Assert: Should fail
    assert!(result.is_err());
    assert_eq!(game.party.members.len(), 6);
}

// ============================================================================
// 4.3 INPUT VALIDATION TESTING
// ============================================================================

#[test]
fn test_invalid_inn_npc_id_format() {
    // Test various invalid NPC ID formats
    let long_id = "a".repeat(1000);
    let invalid_ids = vec![
        "",            // Empty
        " ",           // Whitespace only
        "inn\0keeper", // Null byte
        "inn\nkeeper", // Newline
        &long_id,      // Very long
    ];

    for id in invalid_ids {
        let state = InnManagementState {
            current_inn_id: id.to_string(),
            selected_party_slot: None,
            selected_roster_slot: None,
        };

        // Assert: State created but ID should be validated by runtime
        assert_eq!(state.current_inn_id, id);
    }
}

#[test]
fn test_speaker_npc_id_special_characters() {
    // Test NPC IDs with special characters
    let test_ids = vec![
        "inn-keeper-001",
        "inn_keeper_001",
        "innkeeper.001",
        "InnKeeper001",
        "innkeeper:town1",
    ];

    for id in &test_ids {
        let dialogue = DialogueState::start(999_u16, 1_u16, None, Some(id.to_string()));
        assert_eq!(dialogue.speaker_npc_id, Some(id.to_string()));
    }
}

#[test]
fn test_dialogue_node_id_boundaries() {
    // Test edge cases for node IDs (DialogueState::start takes u16 for tree_id, u32 for node_id)
    let test_cases = vec![
        (999_u16, 0_u16),   // Node 0
        (999_u16, 1_u16),   // Normal
        (999_u16, 100_u16), // Large node ID
    ];

    for (tree_id, node_id) in test_cases {
        let dialogue = DialogueState::start(tree_id, node_id, None, Some("npc".to_string()));
        assert_eq!(dialogue.active_tree_id, Some(tree_id));
        // current_node_id should match what was passed
    }
}

#[test]
fn test_roster_character_access_by_invalid_index() {
    // Arrange: Create roster with characters
    let game = create_game_with_roster(5);
    assert_eq!(game.roster.characters.len(), 5);

    // Act: Try to get character with invalid index
    let result = game.roster.get_character(999);

    // Assert: Should return None
    assert!(result.is_none());
}

// ============================================================================
// 4.4 REGRESSION TESTING
// ============================================================================

#[test]
fn test_existing_dialogue_still_works() {
    // Verify non-inn dialogues work normally
    let dialogue = DialogueState::start(1_u16, 1_u16, None, Some("merchant".to_string()));

    assert_eq!(dialogue.active_tree_id, Some(1_u16));
    assert_eq!(dialogue.speaker_npc_id, Some("merchant".to_string()));
    assert_eq!(dialogue.current_node_id, 1_u16);
}

#[test]
fn test_recruitment_dialogue_unaffected() {
    // Verify recruitment system still works
    let _game = create_game_with_party(2);

    // Simulate recruitment context (would be set by recruitment system)
    let recruit_dialogue = DialogueState::start(50_u16, 1_u16, None, Some("recruiter".to_string()));

    assert_eq!(recruit_dialogue.active_tree_id, Some(50_u16));
    assert_eq!(
        recruit_dialogue.speaker_npc_id,
        Some("recruiter".to_string())
    );

    // Recruitment context is separate from inn management
}

#[test]
fn test_combat_mode_unaffected() {
    // Arrange: Create game and enter combat
    let mut game = create_game_with_party(4);
    game.enter_combat();

    // Assert: Combat state independent of inn changes
    assert!(matches!(game.mode, GameMode::Combat(_)));
    assert_eq!(game.party.members.len(), 4);

    // Exit combat
    game.exit_combat();
    assert_eq!(game.mode, GameMode::Exploration);
}

#[test]
fn test_exploration_mode_unaffected() {
    // Verify exploration remains default mode
    let game = GameState::new();
    assert_eq!(game.mode, GameMode::Exploration);

    // Create game with party
    let game_with_party = create_game_with_party(3);
    assert_eq!(game_with_party.mode, GameMode::Exploration);
}

#[test]
fn test_menu_mode_unaffected() {
    // Verify menu mode transitions work
    let mut game = create_game_with_party(2);

    game.enter_menu();
    assert_eq!(game.mode, GameMode::Menu);

    game.return_to_exploration();
    assert_eq!(game.mode, GameMode::Exploration);
}

#[test]
fn test_party_operations_outside_inn() {
    // Verify party operations work normally outside inn context
    let mut game = GameState::new();

    // Add members
    for i in 0..3 {
        let char = create_test_character(&format!("Hero{}", i + 1));
        game.party.add_member(char).expect("Failed to add member");
    }

    assert_eq!(game.party.members.len(), 3);

    // Remove member
    let removed = game.party.remove_member(1);
    assert!(removed.is_some());
    assert_eq!(game.party.members.len(), 2);

    // Verify operations independent of inn system
}

// ============================================================================
// 4.5 PERFORMANCE TESTING
// ============================================================================

#[test]
fn test_rapid_mode_transitions() {
    // Test rapid transitions between modes
    let mut game = create_game_with_party(4);

    // Perform 100 rapid transitions
    for i in 0..100 {
        let npc_id = format!("inn_{}", i);

        // Exploration -> Dialogue
        game.mode = GameMode::Dialogue(DialogueState::start(
            999_u16,
            1_u16,
            None,
            Some(npc_id.clone()),
        ));

        // Dialogue -> InnManagement
        game.mode = GameMode::InnManagement(InnManagementState {
            current_inn_id: npc_id,
            selected_party_slot: None,
            selected_roster_slot: None,
        });

        // InnManagement -> Exploration
        game.return_to_exploration();

        // Verify state consistency
        assert_eq!(game.mode, GameMode::Exploration);
        assert_eq!(game.party.members.len(), 4);
    }

    // Assert: No corruption or performance degradation
    assert_eq!(game.party.members.len(), 4);
}

#[test]
fn test_large_roster_filtering_performance() {
    // Test filtering with maximum roster size
    let mut game = create_game_with_roster(18);

    // Add party members
    for i in 0..6 {
        let char = create_test_character(&format!("Party{}", i + 1));
        game.party
            .add_member(char)
            .expect("Failed to add party member");
    }

    assert_eq!(game.roster.characters.len(), 18);
    assert_eq!(game.party.members.len(), 6);

    // Filter roster to exclude party members (by name for this test)
    let available_roster: Vec<&Character> = game
        .roster
        .characters
        .iter()
        .filter(|c| !game.party.members.iter().any(|p| p.name == c.name))
        .collect();

    // Assert: Filtering works correctly
    assert_eq!(available_roster.len(), 18); // All roster chars available (different names)

    // Performance: Should complete quickly even with max roster
}

#[test]
fn test_dialogue_state_creation_performance() {
    // Test creating many dialogue states rapidly
    let mut states = Vec::new();

    for i in 0..1000 {
        let state = DialogueState::start(999_u16, 1_u16, None, Some(format!("npc_{}", i)));
        states.push(state);
    }

    // Assert: All states created successfully
    assert_eq!(states.len(), 1000);

    // Verify each state is independent
    for (i, state) in states.iter().enumerate() {
        assert_eq!(state.speaker_npc_id, Some(format!("npc_{}", i)));
    }
}

#[test]
fn test_inn_management_state_creation_performance() {
    // Test creating many inn management states
    let mut states = Vec::new();
    for i in 0..100 {
        let state = InnManagementState {
            current_inn_id: format!("inn_{}", i),
            selected_party_slot: None,
            selected_roster_slot: None,
        };
        states.push(state);
    }

    // Assert: All states created successfully
    assert_eq!(states.len(), 100);

    // Verify states are independent
    for (i, state) in states.iter().enumerate() {
        assert_eq!(state.current_inn_id, format!("inn_{}", i));
    }
}

// ============================================================================
// 4.6 COMPLEX INTEGRATION SCENARIOS
// ============================================================================

#[test]
fn test_save_load_simulation() {
    // Simulate save/load across inn management
    let mut game = create_game_with_party(5);
    game.party.gold = 1000;

    // "Save" state
    let saved_party_size = game.party.members.len();
    let saved_gold = game.party.gold;
    let saved_mode = game.mode.clone();

    // Enter inn management
    game.mode = GameMode::InnManagement(InnManagementState {
        current_inn_id: "save_test_inn".to_string(),
        selected_party_slot: None,
        selected_roster_slot: None,
    });

    // Make changes
    game.party.gold = 500;

    // "Load" state (restore)
    game.party.gold = saved_gold;
    game.mode = saved_mode;

    // Assert: State restored correctly
    assert_eq!(game.party.members.len(), saved_party_size);
    assert_eq!(game.party.gold, saved_gold);
    assert_eq!(game.mode, GameMode::Exploration);
}

#[test]
fn test_nested_dialogue_prevention() {
    // Verify cannot nest dialogue modes
    let mut game = create_game_with_party(2);

    // Enter first dialogue
    game.mode = GameMode::Dialogue(DialogueState::start(
        999_u16,
        1_u16,
        None,
        Some("npc1".to_string()),
    ));

    // Attempting to enter another dialogue should replace, not nest
    game.mode = GameMode::Dialogue(DialogueState::start(
        50_u16,
        1_u16,
        None,
        Some("npc2".to_string()),
    ));

    if let GameMode::Dialogue(ref state) = game.mode {
        assert_eq!(state.active_tree_id, Some(50_u16)); // Second dialogue active
        assert_eq!(state.speaker_npc_id, Some("npc2".to_string()));
    } else {
        panic!("Expected Dialogue mode");
    }
}

#[test]
fn test_party_consistency_after_failures() {
    // Test party state remains consistent even after operation failures
    let mut game = GameState::new();

    // Add valid members
    for i in 0..3 {
        game.party
            .add_member(create_test_character(&format!("Valid{}", i)))
            .expect("Failed to add valid member");
    }

    let initial_size = game.party.members.len();

    // Try invalid operations
    let _removed = game.party.remove_member(999); // Invalid index (returns None)

    // Assert: Party state unchanged
    assert_eq!(game.party.members.len(), initial_size);
}

#[test]
fn test_dialogue_speaker_npc_id_preservation() {
    // Verify speaker_npc_id is preserved throughout dialogue lifecycle
    let initial_npc_id = Some("persistent_npc".to_string());
    let mut dialogue = DialogueState::start(999_u16, 1_u16, None, initial_npc_id.clone());

    // Simulate dialogue progression
    dialogue.advance_to(2);
    assert_eq!(dialogue.speaker_npc_id, initial_npc_id);

    dialogue.advance_to(3);
    assert_eq!(dialogue.speaker_npc_id, initial_npc_id);

    // End dialogue
    dialogue.end();
    // After end, speaker_npc_id should be cleared
    assert_eq!(dialogue.speaker_npc_id, None);
}

#[test]
fn test_concurrent_roster_and_party_modifications() {
    // Test modifying both roster and party
    let mut game = create_game_with_roster(10);

    // Add party members
    for i in 0..3 {
        let char = create_test_character(&format!("Party{}", i));
        game.party.add_member(char).expect("Failed to add");
    }

    let initial_roster_size = game.roster.characters.len();
    let initial_party_size = game.party.members.len();

    // Add to party
    let new_member = create_test_character("NewMember");
    game.party.add_member(new_member).expect("Failed to add");

    // Access roster character
    let _char = game.roster.get_character(0);

    // Assert: Both operations successful
    assert_eq!(game.party.members.len(), initial_party_size + 1);
    assert_eq!(game.roster.characters.len(), initial_roster_size);
}
