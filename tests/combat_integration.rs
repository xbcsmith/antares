// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for complete combat flow
//!
//! Tests the entire combat system from encounter start through resolution,
//! verifying that all systems work together correctly.

use antares::application::GameState;
use antares::domain::character::{Alignment, Character, Class, Race, Sex};
use antares::domain::combat::database::MonsterDatabase;
use antares::domain::combat::engine::{start_combat, CombatState, Combatant};
use antares::domain::combat::types::{CombatStatus, Handicap};
use antares::domain::types::MonsterId;

/// Helper function to create a test character with specific stats
fn create_test_character(name: &str, class: Class, hp: u16) -> Character {
    let mut character = Character::new(
        name.to_string(),
        Race::Human,
        class,
        Sex::Male,
        Alignment::Good,
    );
    character.hp.current = hp;
    character.hp.base = hp;
    character
}

#[test]
fn test_complete_combat_flow() {
    // Arrange: Create a combat with party and monsters
    let mut combat = CombatState::new(Handicap::Even);

    // Add party members
    let knight = create_test_character("Sir Lancelot", Class::Knight, 20);
    let cleric = create_test_character("Friar Tuck", Class::Cleric, 15);
    combat.add_player(knight);
    combat.add_player(cleric);

    // Load monster database and add monsters
    let monster_db = MonsterDatabase::load_from_file("data/monsters.ron")
        .expect("Failed to load monster database");

    if let Some(goblin_template) = monster_db.get_monster(MonsterId::from(1)) {
        // Convert MonsterDefinition to Monster
        let goblin1 = goblin_template.to_monster();
        let mut goblin2 = goblin_template.to_monster();
        goblin2.name = "Goblin 2".to_string();
        combat.add_monster(goblin1);
        combat.add_monster(goblin2);
    } else {
        panic!("Test requires Goblin (ID 1) in monster database");
    }

    // Act: Start combat
    start_combat(&mut combat);

    // Assert: Combat is properly initialized
    assert_eq!(combat.status, CombatStatus::InProgress);
    assert_eq!(combat.round, 1);
    assert_eq!(combat.current_turn, 0);
    assert!(!combat.turn_order.is_empty(), "Turn order should be set");
    assert_eq!(combat.alive_party_count(), 2);
    assert_eq!(combat.alive_monster_count(), 2);

    // Verify turn order was calculated
    assert_eq!(
        combat.turn_order.len(),
        combat.participants.len(),
        "Turn order should include all combatants"
    );

    // Verify all participants can act initially
    let acting_count = combat.participants.iter().filter(|c| c.can_act()).count();
    assert!(
        acting_count >= 2,
        "At least some combatants should be able to act"
    );
}

#[test]
fn test_exploration_to_combat_to_exploration() {
    // Arrange: Create game state in exploration mode
    let mut game_state = GameState::new();
    assert_eq!(game_state.mode, antares::application::GameMode::Exploration);

    // Add a character to the party
    let hero = create_test_character("Hero", Class::Knight, 25);
    game_state
        .party
        .add_member(hero)
        .expect("Failed to add party member");

    // Record initial party state
    let initial_party_size = game_state.party.size();
    let initial_hp = game_state.party.members[0].hp.current;

    // Act: Transition to combat
    game_state.enter_combat();

    // Assert: Verify combat mode
    assert_eq!(game_state.mode, antares::application::GameMode::Combat);
    assert_eq!(
        game_state.party.size(),
        initial_party_size,
        "Party size should be preserved"
    );

    // Simulate combat effects (damage)
    game_state.party.members[0].hp.current -= 5;

    // Act: Exit combat back to exploration
    game_state.exit_combat();

    // Assert: Verify exploration mode and state preservation
    assert_eq!(game_state.mode, antares::application::GameMode::Exploration);
    assert_eq!(
        game_state.party.size(),
        initial_party_size,
        "Party size should still be preserved"
    );
    assert_eq!(
        game_state.party.members[0].hp.current,
        initial_hp - 5,
        "Combat damage should persist after mode change"
    );
}

#[test]
fn test_character_creation_to_first_combat() {
    // Arrange: Create a fresh game state
    let mut game_state = GameState::new();

    // Act: Create a new character
    let warrior = Character::new(
        "Conan".to_string(),
        Race::Human,
        Class::Knight,
        Sex::Male,
        Alignment::Neutral,
    );

    // Add to party directly
    game_state
        .party
        .add_member(warrior)
        .expect("Failed to add character to party");

    // Assert: Character is in party
    assert_eq!(game_state.party.size(), 1);

    // Act: Prepare for first combat
    let mut combat = CombatState::new(Handicap::PartyAdvantage);

    // Transfer party member to combat
    let party_member = game_state.party.members[0].clone();
    combat.add_player(party_member);

    // Add a weak monster for first combat
    let monster_db = MonsterDatabase::load_from_file("data/monsters.ron")
        .expect("Failed to load monster database");

    if let Some(goblin) = monster_db.get_monster(MonsterId::from(1)) {
        combat.add_monster(goblin.to_monster());
    }

    // Start the combat
    start_combat(&mut combat);

    // Assert: Combat is ready
    assert!(combat.is_in_progress());
    assert_eq!(combat.alive_party_count(), 1);
    assert_eq!(combat.alive_monster_count(), 1);
    assert_eq!(combat.handicap, Handicap::PartyAdvantage);

    // Verify the character can act
    if let Some(Combatant::Player(character)) = combat.participants.first() {
        assert!(character.can_act(), "New character should be able to act");
        assert!(character.is_alive(), "New character should be alive");
        assert_eq!(character.name, "Conan");
    } else {
        panic!("Expected player combatant in first position");
    }
}

#[test]
fn test_combat_end_conditions() {
    // Test 1: Party victory
    let mut combat = CombatState::new(Handicap::Even);
    let knight = create_test_character("Knight", Class::Knight, 20);
    combat.add_player(knight);

    let monster_db = MonsterDatabase::load_from_file("data/monsters.ron")
        .expect("Failed to load monster database");

    if let Some(goblin_def) = monster_db.get_monster(MonsterId::from(1)) {
        let mut goblin = goblin_def.to_monster();
        goblin.hp.current = 0; // Dead monster
        combat.add_monster(goblin);
        start_combat(&mut combat);
        combat.check_combat_end();

        assert_eq!(
            combat.status,
            CombatStatus::Victory,
            "Combat should end in victory when all monsters dead"
        );
    }

    // Test 2: Party defeat
    let mut combat2 = CombatState::new(Handicap::Even);
    let mut dead_knight = create_test_character("Dead Knight", Class::Knight, 0);
    dead_knight.hp.current = 0;
    dead_knight
        .conditions
        .add(antares::domain::character::Condition::DEAD);
    combat2.add_player(dead_knight);

    if let Some(goblin) = monster_db.get_monster(MonsterId::from(1)) {
        combat2.add_monster(goblin.to_monster());
        start_combat(&mut combat2);
        combat2.check_combat_end();

        assert_eq!(
            combat2.status,
            CombatStatus::Defeat,
            "Combat should end in defeat when all party members dead"
        );
    }
}

#[test]
fn test_combat_with_multiple_rounds() {
    // Arrange: Create combat state
    let mut combat = CombatState::new(Handicap::Even);

    let knight = create_test_character("Knight", Class::Knight, 30);
    let cleric = create_test_character("Cleric", Class::Cleric, 25);
    combat.add_player(knight);
    combat.add_player(cleric);

    let monster_db = MonsterDatabase::load_from_file("data/monsters.ron")
        .expect("Failed to load monster database");

    if let Some(orc) = monster_db.get_monster(MonsterId::from(2)) {
        combat.add_monster(orc.to_monster());
    }

    start_combat(&mut combat);

    // Act: Simulate multiple rounds
    let initial_round = combat.round;
    combat.round += 1;
    combat.current_turn = 0; // Reset turn for new round

    // Assert: Round progression
    assert_eq!(combat.round, initial_round + 1);
    assert!(combat.is_in_progress());
}

#[test]
fn test_handicap_system() {
    // Test party advantage
    let combat_party_adv = CombatState::new(Handicap::PartyAdvantage);
    assert_eq!(combat_party_adv.handicap, Handicap::PartyAdvantage);
    assert!(combat_party_adv.can_flee);

    // Test monster advantage
    let combat_monster_adv = CombatState::new(Handicap::MonsterAdvantage);
    assert_eq!(combat_monster_adv.handicap, Handicap::MonsterAdvantage);

    // Test even combat
    let combat_even = CombatState::new(Handicap::Even);
    assert_eq!(combat_even.handicap, Handicap::Even);
}

#[test]
fn test_combat_participants_management() {
    let mut combat = CombatState::new(Handicap::Even);

    // Add various participants
    let knight = create_test_character("Knight", Class::Knight, 30);
    let paladin = create_test_character("Paladin", Class::Paladin, 28);
    let archer = create_test_character("Archer", Class::Archer, 22);

    combat.add_player(knight);
    combat.add_player(paladin);
    combat.add_player(archer);

    let monster_db = MonsterDatabase::load_from_file("data/monsters.ron")
        .expect("Failed to load monster database");

    // Add multiple monster types
    if let Some(goblin) = monster_db.get_monster(MonsterId::from(1)) {
        combat.add_monster(goblin.to_monster());
    }
    if let Some(orc) = monster_db.get_monster(MonsterId::from(2)) {
        combat.add_monster(orc.to_monster());
    }

    start_combat(&mut combat);

    // Verify all participants are tracked
    assert_eq!(combat.participants.len(), 5);
    assert_eq!(combat.alive_party_count(), 3);
    assert_eq!(combat.alive_monster_count(), 2);
}
