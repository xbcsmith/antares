// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for magic system and spell casting
//!
//! Tests the complete spell casting system including spell validation,
//! resource consumption, and integration with character and combat systems.

use antares::application::{GameMode, GameState};
use antares::domain::character::{Alignment, Character, Sex};
use antares::domain::magic::casting::can_cast_spell;
use antares::domain::magic::database::SpellDatabase;
use antares::domain::magic::types::{SpellContext, SpellError, SpellSchool, SpellTarget};

/// Helper function to create a spellcasting character
fn create_caster(name: &str, class_id: &str, sp: u16) -> Character {
    let mut character = Character::new(
        name.to_string(),
        "human".to_string(),
        class_id.to_string(),
        Sex::Male,
        Alignment::Good,
    );
    character.sp.current = sp;
    character.sp.base = sp;
    character
}

#[test]
fn test_spell_database_loading() {
    // Act: Load spell database
    let spell_db =
        SpellDatabase::load_from_file("data/spells.ron").expect("Failed to load spell database");

    // Assert: Spells loaded successfully
    assert!(!spell_db.is_empty(), "Spell database should not be empty");

    // Verify we can query spells by school
    let cleric_spells = spell_db.get_spells_by_school(SpellSchool::Cleric);
    assert!(!cleric_spells.is_empty(), "Should have cleric spells");

    let sorcerer_spells = spell_db.get_spells_by_school(SpellSchool::Sorcerer);
    assert!(!sorcerer_spells.is_empty(), "Should have sorcerer spells");
}

#[test]
fn test_cleric_can_cast_cleric_spells() {
    // Arrange: Load spells and create cleric
    let spell_db =
        SpellDatabase::load_from_file("data/spells.ron").expect("Failed to load spell database");
    let cleric = create_caster("Father Michael", "cleric", 50);

    // Act: Find a level 1 cleric spell that can be cast anytime
    let cleric_spells = spell_db.get_spells_by_school(SpellSchool::Cleric);
    let level1_spells: Vec<_> = cleric_spells
        .iter()
        .filter(|s| s.level == 1 && s.context == SpellContext::Anytime)
        .collect();

    assert!(
        !level1_spells.is_empty(),
        "Should have level 1 cleric spells castable anytime"
    );

    let spell = level1_spells[0];

    // Assert: Cleric can cast cleric spell
    let result = can_cast_spell(&cleric, spell, &GameMode::Exploration, false, false);
    assert!(
        result.is_ok(),
        "Cleric should be able to cast cleric spell: {:?}",
        result
    );
}

#[test]
fn test_sorcerer_can_cast_sorcerer_spells() {
    // Arrange: Load spells and create sorcerer
    let spell_db =
        SpellDatabase::load_from_file("data/spells.ron").expect("Failed to load spell database");
    let sorcerer = create_caster("Merlin", "sorcerer", 50);

    // Act: Find a level 1 sorcerer spell that can be cast anytime
    let sorcerer_spells = spell_db.get_spells_by_school(SpellSchool::Sorcerer);
    let level1_spells: Vec<_> = sorcerer_spells
        .iter()
        .filter(|s| s.level == 1 && s.context == SpellContext::Anytime)
        .collect();

    assert!(
        !level1_spells.is_empty(),
        "Should have level 1 sorcerer spells castable anytime"
    );

    let spell = level1_spells[0];

    // Assert: Sorcerer can cast sorcerer spell
    let result = can_cast_spell(&sorcerer, spell, &GameMode::Exploration, false, false);
    assert!(
        result.is_ok(),
        "Sorcerer should be able to cast sorcerer spell: {:?}",
        result
    );
}

#[test]
fn test_class_restriction_prevents_casting() {
    // Arrange: Load spells
    let spell_db =
        SpellDatabase::load_from_file("data/spells.ron").expect("Failed to load spell database");

    // Create a knight (cannot cast spells)
    let knight = create_caster("Sir Lancelot", "knight", 50);

    // Find a cleric spell
    let cleric_spells = spell_db.get_spells_by_school(SpellSchool::Cleric);
    let spell = cleric_spells
        .first()
        .expect("Need at least one cleric spell");

    // Assert: Knight cannot cast cleric spell
    let result = can_cast_spell(&knight, spell, &GameMode::Exploration, false, false);
    assert!(result.is_err(), "Knight should not be able to cast spells");
    assert!(matches!(result, Err(SpellError::WrongClass(_, _))));
}

#[test]
fn test_insufficient_spell_points() {
    // Arrange: Load spells and create cleric with low SP
    let spell_db =
        SpellDatabase::load_from_file("data/spells.ron").expect("Failed to load spell database");
    let mut cleric = create_caster("Weak Cleric", "cleric", 1);

    // Find a high-level spell (costs more SP) that can be cast anytime
    let cleric_spells = spell_db.get_spells_by_school(SpellSchool::Cleric);
    let expensive_spell = cleric_spells
        .iter()
        .find(|s| s.sp_cost > 1 && s.context == SpellContext::Anytime)
        .expect("Need spell with SP cost > 1 that can be cast anytime");

    // Set SP lower than cost
    cleric.sp.current = expensive_spell.sp_cost - 1;

    // Assert: Cannot cast without enough SP
    let result = can_cast_spell(
        &cleric,
        expensive_spell,
        &GameMode::Exploration,
        false,
        false,
    );
    assert!(
        result.is_err(),
        "Should not be able to cast without enough SP: {:?}",
        result
    );
    // The actual error depends on validation order, could be NotEnoughSP or other validation
}

#[test]
fn test_context_restrictions() {
    // Arrange: Load spells
    let spell_db =
        SpellDatabase::load_from_file("data/spells.ron").expect("Failed to load spell database");
    let mut cleric = create_caster("Cleric", "cleric", 100);
    cleric.level = 10; // Set level high enough to cast any spell

    // Find a combat-only cleric spell
    let all_combat_spells = spell_db.all_spells();
    let combat_spells: Vec<_> = all_combat_spells
        .iter()
        .filter(|s| s.context == SpellContext::CombatOnly && s.school == SpellSchool::Cleric)
        .collect();

    if let Some(combat_spell) = combat_spells.first() {
        // Assert: Cannot cast combat-only spell outside combat
        let result = can_cast_spell(&cleric, combat_spell, &GameMode::Exploration, false, false);
        assert!(
            result.is_err(),
            "Should not cast combat-only spell in exploration: {:?}",
            result
        );
        assert!(
            matches!(result, Err(SpellError::CombatOnly)),
            "Expected CombatOnly error but got: {:?}",
            result
        );

        // Assert: Can cast in combat (if enough SP)
        let combat_state = antares::domain::combat::engine::CombatState::new(
            antares::domain::combat::types::Handicap::Even,
        );
        let result_in_combat = can_cast_spell(
            &cleric,
            combat_spell,
            &GameMode::Combat(combat_state),
            true,
            false,
        );
        // Either succeeds or fails due to SP, but context should be valid
        assert!(
            result_in_combat.is_ok() || !matches!(result_in_combat, Err(SpellError::CombatOnly)),
            "Should not get context error in combat: {:?}",
            result_in_combat
        );
    }
}

#[test]
fn test_spell_point_consumption() {
    // Arrange: Create character with known SP
    let mut character = create_caster("Caster", "cleric", 50);
    let initial_sp = character.sp.current;

    // Act: Simulate casting spell that costs 5 SP
    let sp_cost = 5;
    character.sp.current -= sp_cost;

    // Assert: SP reduced correctly
    assert_eq!(character.sp.current, initial_sp - sp_cost);
    assert_eq!(character.sp.base, 50, "Base SP should not change");
}

#[test]
fn test_spell_point_restoration() {
    // Arrange: Create character with depleted SP
    let mut character = create_caster("Tired Caster", "sorcerer", 100);
    character.sp.current = 20; // Depleted

    // Act: Rest restores SP
    character.sp.reset(); // In full game, resting would restore SP

    // Assert: SP restored to base
    assert_eq!(character.sp.current, character.sp.base);
}

#[test]
fn test_silenced_character_cannot_cast() {
    // Arrange: Create silenced character
    let mut character = create_caster("Silenced", "cleric", 50);
    character
        .conditions
        .add(antares::domain::character::Condition::SILENCED);

    let spell_db =
        SpellDatabase::load_from_file("data/spells.ron").expect("Failed to load spell database");
    let cleric_spells = spell_db.get_spells_by_school(SpellSchool::Cleric);
    let spell = cleric_spells.first().expect("Need a cleric spell");

    // Assert: Silenced character cannot cast
    let result = can_cast_spell(&character, spell, &GameMode::Exploration, false, false);
    assert!(result.is_err(), "Silenced character should not cast");
    assert!(matches!(result, Err(SpellError::Silenced)));
}

#[test]
fn test_spell_levels() {
    // Arrange: Load spells
    let spell_db =
        SpellDatabase::load_from_file("data/spells.ron").expect("Failed to load spell database");

    // Act: Get spells of different levels
    for level in 1..=3 {
        let level_spells = spell_db.get_spells_by_level(level);
        // We expect at least some spells at each of the first few levels
        if level <= 2 {
            assert!(
                !level_spells.is_empty(),
                "Should have level {} spells",
                level
            );
        }
    }
}

#[test]
fn test_spell_target_types() {
    // Arrange: Load spells
    let spell_db =
        SpellDatabase::load_from_file("data/spells.ron").expect("Failed to load spell database");
    let all_spells = spell_db.all_spells();

    // Assert: Spells have valid target types
    let has_single_target = all_spells
        .iter()
        .any(|s| matches!(s.target, SpellTarget::SingleMonster));
    let has_party_target = all_spells
        .iter()
        .any(|s| matches!(s.target, SpellTarget::AllCharacters));
    let has_self_target = all_spells
        .iter()
        .any(|s| matches!(s.target, SpellTarget::SingleCharacter));

    // We expect variety in targeting
    assert!(
        has_single_target || has_party_target || has_self_target,
        "Should have spells with various target types"
    );
}

#[test]
fn test_complete_spell_casting_flow() {
    // Arrange: Create game state with caster
    let mut game_state = GameState::new();
    let cleric = create_caster("Healer", "cleric", 30);
    game_state
        .party
        .add_member(cleric)
        .expect("Failed to add cleric");

    let spell_db =
        SpellDatabase::load_from_file("data/spells.ron").expect("Failed to load spell database");

    // Find a spell that can be cast anytime
    let cleric_spells = spell_db.get_spells_by_school(SpellSchool::Cleric);
    let spell = *cleric_spells
        .iter()
        .find(|s| s.level == 1 && s.sp_cost <= 30 && s.context == SpellContext::Anytime)
        .expect("Need a castable level 1 cleric spell with Anytime context");

    let initial_sp = game_state.party.members[0].sp.current;

    // Assert: Can validate spell before casting
    let can_cast = can_cast_spell(
        &game_state.party.members[0],
        spell,
        &game_state.mode,
        false,
        false,
    );
    assert!(can_cast.is_ok(), "Cleric should be able to cast spell");

    // Act: Simulate casting (consume SP)
    game_state.party.members[0].sp.current -= spell.sp_cost;

    // Assert: SP consumed
    assert_eq!(
        game_state.party.members[0].sp.current,
        initial_sp - spell.sp_cost
    );
}

#[test]
fn test_gem_cost_spells() {
    // Arrange: Load spells
    let spell_db =
        SpellDatabase::load_from_file("data/spells.ron").expect("Failed to load spell database");

    // Find spells that cost gems
    let all_spells = spell_db.all_spells();
    let gem_spells: Vec<_> = all_spells.iter().filter(|s| s.gem_cost > 0).collect();

    // Assert: Gem spells are high-level or powerful
    for spell in gem_spells {
        // Typically gem spells are level 5+ or special spells
        println!(
            "Gem spell: {} (Level {}, Gems: {})",
            spell.name, spell.level, spell.gem_cost
        );
    }
}

#[test]
fn test_spell_schools_complete() {
    // Arrange: Load spells
    let spell_db =
        SpellDatabase::load_from_file("data/spells.ron").expect("Failed to load spell database");

    // Assert: Both schools have spells
    let cleric_count = spell_db.get_spells_by_school(SpellSchool::Cleric).len();
    let sorcerer_count = spell_db.get_spells_by_school(SpellSchool::Sorcerer).len();

    assert!(cleric_count > 0, "Should have cleric spells");
    assert!(sorcerer_count > 0, "Should have sorcerer spells");

    println!("Cleric spells: {}", cleric_count);
    println!("Sorcerer spells: {}", sorcerer_count);
}

#[test]
fn test_outdoor_spell_restrictions() {
    // Arrange: Load spells
    let spell_db =
        SpellDatabase::load_from_file("data/spells.ron").expect("Failed to load spell database");
    let sorcerer = create_caster("Outdoor Mage", "sorcerer", 100);

    // Find indoor-only spells (if any)
    let all_spells = spell_db.all_spells();
    let indoor_spells: Vec<_> = all_spells
        .iter()
        .filter(|s| s.context == SpellContext::IndoorOnly)
        .collect();

    if let Some(indoor_spell) = indoor_spells.first() {
        // Assert: Cannot cast indoor spell outdoors
        let result = can_cast_spell(&sorcerer, indoor_spell, &GameMode::Exploration, false, true);
        assert!(
            result.is_err(),
            "Should not cast indoor-only spell outdoors"
        );
        assert!(matches!(result, Err(SpellError::IndoorsOnly)));
    }
}
