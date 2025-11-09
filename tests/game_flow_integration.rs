// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for game flow and state transitions
//!
//! Tests complete game loops including exploration, resource management,
//! party management, and state transitions between different game modes.

use antares::application::{GameMode, GameState};
use antares::domain::character::{
    Alignment, AttributePair, Character, Class, Condition, Race, Sex,
};
use antares::domain::types::GameTime;

/// Helper function to create a test character
fn create_character(name: &str, class: Class) -> Character {
    Character::new(
        name.to_string(),
        Race::Human,
        class,
        Sex::Male,
        Alignment::Good,
    )
}

#[test]
fn test_game_initialization() {
    // Act: Create a new game
    let game_state = GameState::new();

    // Assert: Verify initial state
    assert_eq!(game_state.mode, GameMode::Exploration);
    assert!(game_state.party.is_empty());
    assert!(game_state.roster.characters.is_empty());
    assert_eq!(game_state.time.day, 1);
    assert_eq!(game_state.time.hour, 6);
    assert_eq!(game_state.time.minute, 0);
}

#[test]
fn test_party_formation() {
    // Arrange: Create game state and characters
    let mut game_state = GameState::new();

    let knight = create_character("Knight", Class::Knight);
    let cleric = create_character("Cleric", Class::Cleric);
    let sorcerer = create_character("Sorcerer", Class::Sorcerer);

    // Act: Add characters directly to party
    game_state
        .party
        .add_member(knight)
        .expect("Failed to add knight to party");
    game_state
        .party
        .add_member(cleric)
        .expect("Failed to add cleric to party");
    game_state
        .party
        .add_member(sorcerer)
        .expect("Failed to add sorcerer to party");

    // Assert: Party formed correctly
    assert_eq!(game_state.party.size(), 3);
    assert!(!game_state.party.is_empty());
}

#[test]
fn test_game_mode_transitions() {
    // Arrange: Create game with party
    let mut game_state = GameState::new();
    let hero = create_character("Hero", Class::Paladin);
    game_state
        .party
        .add_member(hero)
        .expect("Failed to add party member");

    // Test: Exploration -> Combat -> Exploration
    assert_eq!(game_state.mode, GameMode::Exploration);

    game_state.enter_combat();
    assert_eq!(game_state.mode, GameMode::Combat);
    assert_eq!(game_state.party.size(), 1); // Party preserved

    game_state.exit_combat();
    assert_eq!(game_state.mode, GameMode::Exploration);
    assert_eq!(game_state.party.size(), 1); // Party still preserved

    // Test: Exploration -> Menu -> Exploration
    game_state.enter_menu();
    assert_eq!(game_state.mode, GameMode::Menu);

    game_state.return_to_exploration();
    assert_eq!(game_state.mode, GameMode::Exploration);

    // Test: Exploration -> Dialogue -> Exploration
    game_state.enter_dialogue();
    assert_eq!(game_state.mode, GameMode::Dialogue);

    game_state.return_to_exploration();
    assert_eq!(game_state.mode, GameMode::Exploration);
}

#[test]
fn test_party_resource_sharing() {
    // Arrange: Create game state and party
    let mut game_state = GameState::new();
    let knight = create_character("Knight", Class::Knight);
    game_state
        .party
        .add_member(knight)
        .expect("Failed to add member");

    // Act: Modify shared resources
    game_state.party.gold += 100;
    game_state.party.gems += 10;
    game_state.party.food += 50;

    // Assert: Resources are shared at party level
    assert_eq!(game_state.party.gold, 100);
    assert_eq!(game_state.party.gems, 10);
    assert_eq!(game_state.party.food, 50);
}

#[test]
fn test_time_progression() {
    // Arrange: Create game state
    let mut game_state = GameState::new();
    let initial_day = game_state.time.day;
    let initial_hour = game_state.time.hour;

    // Act: Advance time by 1 hour
    game_state.time.advance_minutes(60);

    // Assert: Time progressed correctly
    assert_eq!(game_state.time.day, initial_day);
    assert_eq!(game_state.time.hour, initial_hour + 1);

    // Act: Advance time across day boundary (18 hours)
    game_state.time.advance_minutes(18 * 60);

    // Assert: Day incremented
    assert!(game_state.time.day > initial_day);
}

#[test]
fn test_character_state_persistence_across_modes() {
    // Arrange: Create game with damaged character
    let mut game_state = GameState::new();
    let mut warrior = create_character("Warrior", Class::Knight);
    warrior.hp.base = 30;
    warrior.hp.current = 30;

    game_state
        .party
        .add_member(warrior)
        .expect("Failed to add warrior");

    // Act: Damage character in exploration
    game_state.party.members[0].hp.current = 15;

    // Transition to combat
    game_state.enter_combat();
    assert_eq!(game_state.party.members[0].hp.current, 15);

    // Take more damage in combat
    game_state.party.members[0].hp.current = 5;

    // Return to exploration
    game_state.exit_combat();

    // Assert: All damage persisted
    assert_eq!(game_state.party.members[0].hp.current, 5);
    assert_eq!(game_state.party.members[0].hp.base, 30);
}

#[test]
fn test_stat_modification_and_reset() {
    // Arrange: Create character with base stats
    let mut character = create_character("TestChar", Class::Knight);
    let original_might = character.stats.might.base;

    // Act: Apply temporary buff
    character.stats.might.current += 5;

    // Assert: Current modified, base unchanged
    assert_eq!(character.stats.might.current, original_might + 5);
    assert_eq!(character.stats.might.base, original_might);

    // Act: Reset stat
    character.stats.might.reset();

    // Assert: Current restored to base
    assert_eq!(character.stats.might.current, original_might);
}

#[test]
fn test_party_member_conditions() {
    // Arrange: Create party
    let mut game_state = GameState::new();
    let hero = create_character("Hero", Class::Cleric);
    game_state
        .party
        .add_member(hero)
        .expect("Failed to add hero");

    // Act: Apply condition (PARALYZED prevents acting)
    game_state.party.members[0]
        .conditions
        .add(Condition::PARALYZED);

    // Assert: Character has condition
    assert!(game_state.party.members[0]
        .conditions
        .has(Condition::PARALYZED));
    assert!(!game_state.party.members[0].can_act());

    // Act: Clear condition
    game_state.party.members[0].conditions.clear();

    // Assert: Character recovered
    assert!(!game_state.party.members[0]
        .conditions
        .has(Condition::PARALYZED));
    assert!(game_state.party.members[0].can_act());
}

#[test]
fn test_multiple_characters_in_roster_and_party() {
    // Arrange: Create game state
    let mut game_state = GameState::new();

    // Create 6 characters for party
    let party_members = vec![
        create_character("Knight", Class::Knight),
        create_character("Paladin", Class::Paladin),
        create_character("Archer", Class::Archer),
        create_character("Cleric", Class::Cleric),
        create_character("Sorcerer", Class::Sorcerer),
        create_character("Robber", Class::Robber),
    ];

    // Act: Form full party (max 6)
    for member in party_members {
        game_state
            .party
            .add_member(member)
            .expect("Failed to add to party");
    }

    // Assert: Full party formed
    assert_eq!(game_state.party.size(), 6);
    assert!(game_state.party.is_full());

    // Try to add one more (should fail - party full)
    let bench_char = create_character("Reserve Knight", Class::Knight);
    let result = game_state.party.add_member(bench_char);
    assert!(
        result.is_err(),
        "Should not be able to add 7th member to party"
    );
}

#[test]
fn test_exploration_loop_simulation() {
    // Arrange: Create game ready for exploration
    let mut game_state = GameState::new();
    let adventurer = create_character("Adventurer", Class::Archer);
    game_state
        .party
        .add_member(adventurer)
        .expect("Failed to add adventurer");

    game_state.party.food = 100;
    game_state.party.gold = 50;

    // Simulate exploration actions
    // 1. Move (costs time)
    game_state.time.advance_minutes(10);

    // 2. Check status
    assert_eq!(game_state.mode, GameMode::Exploration);
    assert!(game_state.party.members[0].is_alive());

    // 3. Consume food (every 24 hours in real game)
    game_state.party.food -= 1;
    assert_eq!(game_state.party.food, 99);

    // 4. Enter menu to check inventory
    game_state.enter_menu();
    assert_eq!(game_state.mode, GameMode::Menu);

    // 5. Return to exploration
    game_state.return_to_exploration();
    assert_eq!(game_state.mode, GameMode::Exploration);

    // Assert: Party state maintained throughout
    assert_eq!(game_state.party.size(), 1);
    assert_eq!(game_state.party.gold, 50);
}

#[test]
fn test_attribute_pair_system() {
    // Test the AttributePair pattern used throughout the game
    let mut attr = AttributePair::new(15);

    assert_eq!(attr.base, 15);
    assert_eq!(attr.current, 15);

    // Apply buff
    attr.modify(5);
    assert_eq!(attr.current, 20);
    assert_eq!(attr.base, 15); // Base unchanged

    // Apply debuff
    attr.modify(-3);
    assert_eq!(attr.current, 17);

    // Reset
    attr.reset();
    assert_eq!(attr.current, 15);
    assert_eq!(attr.base, 15);
}

#[test]
fn test_game_time_system() {
    let mut time = GameTime::new(1, 6, 0);

    // Start at day 1, 6:00 AM
    assert_eq!(time.day, 1);
    assert_eq!(time.hour, 6);
    assert_eq!(time.minute, 0);

    // Advance 30 minutes
    time.advance_minutes(30);
    assert_eq!(time.hour, 6);
    assert_eq!(time.minute, 30);

    // Advance to next hour
    time.advance_minutes(30);
    assert_eq!(time.hour, 7);
    assert_eq!(time.minute, 0);

    // Advance multiple hours
    time.advance_minutes(300); // 5 hours
    assert_eq!(time.hour, 12);

    // Advance past midnight
    time.advance_minutes(12 * 60); // 12 hours to midnight and beyond
    assert_eq!(time.day, 2);
}
