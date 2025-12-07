// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for Proficiency UNION Logic
//!
//! These tests verify that the proficiency system correctly implements UNION logic:
//! - A character can use an item if EITHER their class OR race grants the required proficiency
//! - Race incompatible tags override proficiency grants
//! - Both conditions must be satisfied for item usage
//!
//! ## Test Coverage
//!
//! - Class grants proficiency (race does not)
//! - Race grants proficiency (class does not)
//! - Neither class nor race grants proficiency
//! - Race incompatible tags block usage despite proficiency
//! - Proficiency does NOT override race tag restrictions

use antares::domain::classes::ClassDatabase;
use antares::domain::items::{ArmorClassification, WeaponClassification};
use antares::domain::proficiency::{has_proficiency_union, ProficiencyDatabase, ProficiencyId};
use antares::domain::races::RaceDatabase;
use std::path::PathBuf;

// ===== Helper Functions =====

/// Load tutorial campaign databases for testing
fn load_tutorial_databases() -> (ClassDatabase, RaceDatabase, ProficiencyDatabase) {
    let campaign_root = PathBuf::from("campaigns/tutorial");

    let class_db = ClassDatabase::load_from_file(campaign_root.join("data/classes.ron"))
        .expect("Failed to load classes.ron");

    let race_db = RaceDatabase::load_from_file(campaign_root.join("data/races.ron"))
        .expect("Failed to load races.ron");

    let proficiency_db =
        ProficiencyDatabase::load_from_file(PathBuf::from("data/proficiencies.ron"))
            .expect("Failed to load proficiencies.ron");

    (class_db, race_db, proficiency_db)
}

/// Check if a character (class + race) can use an item
///
/// This simulates the full proficiency check:
/// 1. Check proficiency requirement (UNION: class OR race)
/// 2. Check item tags against race incompatible tags
/// 3. Both must pass
fn can_character_use_item(
    required_proficiency: Option<&ProficiencyId>,
    class_proficiencies: &[ProficiencyId],
    race_proficiencies: &[ProficiencyId],
    item_tags: &[String],
    race_incompatible_tags: &[String],
) -> bool {
    // First check: proficiency requirement (UNION: class OR race)
    let has_prof = has_proficiency_union(
        required_proficiency,
        class_proficiencies,
        race_proficiencies,
    );

    // Second check: item tags must be compatible with race
    let tags_ok = !item_tags
        .iter()
        .any(|tag| race_incompatible_tags.contains(tag));

    // Both must pass
    has_prof && tags_ok
}

/// Get proficiency requirement from weapon classification
fn get_weapon_proficiency(classification: WeaponClassification) -> Option<ProficiencyId> {
    match classification {
        WeaponClassification::Simple => Some("simple_weapon".to_string()),
        WeaponClassification::MartialMelee => Some("martial_melee".to_string()),
        WeaponClassification::MartialRanged => Some("martial_ranged".to_string()),
        WeaponClassification::Blunt => Some("blunt_weapon".to_string()),
        WeaponClassification::Unarmed => None, // Unarmed attacks require no proficiency
    }
}

/// Get proficiency requirement from armor classification
fn get_armor_proficiency(classification: ArmorClassification) -> Option<ProficiencyId> {
    match classification {
        ArmorClassification::Light => Some("light_armor".to_string()),
        ArmorClassification::Medium => Some("medium_armor".to_string()),
        ArmorClassification::Heavy => Some("heavy_armor".to_string()),
        ArmorClassification::Shield => Some("shield".to_string()),
    }
}

// ===== Phase 5 Task 5.1: Class Grants Proficiency =====

#[test]
fn test_proficiency_union_class_grants() {
    // Scenario: Human Knight CAN use Longsword
    // Knight class has martial_melee proficiency
    // Human race has no proficiencies
    // Longsword requires martial_melee proficiency
    //
    // Expected: SUCCESS - class grants proficiency

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let knight = class_db
        .get_class("knight")
        .expect("Knight class not found");
    let human = race_db.get_race("human").expect("Human race not found");

    let longsword_proficiency = get_weapon_proficiency(WeaponClassification::MartialMelee);
    let longsword_tags: Vec<String> = vec![];

    let can_use = can_character_use_item(
        longsword_proficiency.as_ref(),
        &knight.proficiencies,
        &human.proficiencies,
        &longsword_tags,
        &human.incompatible_item_tags,
    );

    assert!(
        can_use,
        "Human Knight should be able to use Longsword (class grants martial_melee)"
    );
}

#[test]
fn test_proficiency_union_class_grants_heavy_armor() {
    // Scenario: Dwarf Knight CAN use Plate Mail
    // Knight class has heavy_armor proficiency
    // Dwarf race has no armor proficiencies
    // Plate Mail requires heavy_armor proficiency and has heavy_armor tag
    //
    // Expected: SUCCESS - class grants proficiency, dwarf has no size restrictions

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let knight = class_db
        .get_class("knight")
        .expect("Knight class not found");
    let dwarf = race_db.get_race("dwarf").expect("Dwarf race not found");

    let plate_proficiency = get_armor_proficiency(ArmorClassification::Heavy);
    let plate_tags = vec!["heavy_armor".to_string()];

    let can_use = can_character_use_item(
        plate_proficiency.as_ref(),
        &knight.proficiencies,
        &dwarf.proficiencies,
        &plate_tags,
        &dwarf.incompatible_item_tags,
    );

    assert!(
        can_use,
        "Dwarf Knight should be able to use Plate Mail (class grants heavy_armor)"
    );
}

// ===== Phase 5 Task 5.2: Race Grants Proficiency =====

#[test]
fn test_proficiency_union_race_grants() {
    // Scenario: Elf Sorcerer CAN use Long Bow
    // Sorcerer class does NOT have martial_ranged proficiency
    // Elf race HAS martial_ranged proficiency (racial bow mastery)
    // Long Bow requires martial_ranged proficiency
    //
    // Expected: SUCCESS - race grants proficiency (UNION logic)

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let sorcerer = class_db
        .get_class("sorcerer")
        .expect("Sorcerer class not found");
    let elf = race_db.get_race("elf").expect("Elf race not found");

    let longbow_proficiency = get_weapon_proficiency(WeaponClassification::MartialRanged);
    let longbow_tags = vec!["large_weapon".to_string(), "two_handed".to_string()];

    let can_use = can_character_use_item(
        longbow_proficiency.as_ref(),
        &sorcerer.proficiencies,
        &elf.proficiencies,
        &longbow_tags,
        &elf.incompatible_item_tags,
    );

    assert!(
        can_use,
        "Elf Sorcerer should be able to use Long Bow (race grants martial_ranged proficiency)"
    );
}

#[test]
fn test_proficiency_union_race_grants_longsword() {
    // Scenario: Elf Archer CAN use Longsword
    // Archer class does NOT have longsword proficiency explicitly
    // Elf race HAS longsword proficiency (racial blade mastery)
    //
    // Expected: SUCCESS - race grants proficiency

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let archer = class_db
        .get_class("archer")
        .expect("Archer class not found");
    let elf = race_db.get_race("elf").expect("Elf race not found");

    let longsword_proficiency = Some("longsword".to_string());
    let longsword_tags: Vec<String> = vec![];

    let can_use = can_character_use_item(
        longsword_proficiency.as_ref(),
        &archer.proficiencies,
        &elf.proficiencies,
        &longsword_tags,
        &elf.incompatible_item_tags,
    );

    assert!(
        can_use,
        "Elf Archer should be able to use Longsword (race grants longsword proficiency)"
    );
}

// ===== Phase 5 Task 5.3: Neither Class Nor Race Grants Proficiency =====

#[test]
fn test_proficiency_union_neither_grants() {
    // Scenario: Human Sorcerer CANNOT use Longsword
    // Sorcerer class does NOT have martial_melee proficiency
    // Human race has NO proficiencies
    // Longsword requires martial_melee proficiency
    //
    // Expected: FAILURE - neither class nor race grants proficiency

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let sorcerer = class_db
        .get_class("sorcerer")
        .expect("Sorcerer class not found");
    let human = race_db.get_race("human").expect("Human race not found");

    let longsword_proficiency = get_weapon_proficiency(WeaponClassification::MartialMelee);
    let longsword_tags: Vec<String> = vec![];

    let can_use = can_character_use_item(
        longsword_proficiency.as_ref(),
        &sorcerer.proficiencies,
        &human.proficiencies,
        &longsword_tags,
        &human.incompatible_item_tags,
    );

    assert!(
        !can_use,
        "Human Sorcerer should NOT be able to use Longsword (neither class nor race grants martial_melee)"
    );
}

#[test]
fn test_proficiency_union_neither_grants_heavy_armor() {
    // Scenario: Human Sorcerer CANNOT use Plate Mail
    // Sorcerer class does NOT have heavy_armor proficiency
    // Human race has NO armor proficiencies
    // Plate Mail requires heavy_armor proficiency
    //
    // Expected: FAILURE - neither class nor race grants proficiency

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let sorcerer = class_db
        .get_class("sorcerer")
        .expect("Sorcerer class not found");
    let human = race_db.get_race("human").expect("Human race not found");

    let plate_proficiency = get_armor_proficiency(ArmorClassification::Heavy);
    let plate_tags = vec!["heavy_armor".to_string()];

    let can_use = can_character_use_item(
        plate_proficiency.as_ref(),
        &sorcerer.proficiencies,
        &human.proficiencies,
        &plate_tags,
        &human.incompatible_item_tags,
    );

    assert!(
        !can_use,
        "Human Sorcerer should NOT be able to use Plate Mail (neither class nor race grants heavy_armor)"
    );
}

// ===== Phase 5 Task 5.4: Race Incompatible Tags Block Usage =====

#[test]
fn test_race_incompatible_tags() {
    // Scenario: Gnome Archer CANNOT use Long Bow
    // Archer class HAS martial_ranged proficiency (satisfies proficiency requirement)
    // Gnome race is Small and has incompatible_item_tags: ["large_weapon"]
    // Long Bow has tags: ["large_weapon", "two_handed"]
    //
    // Expected: FAILURE - race incompatible tag blocks usage despite proficiency

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let archer = class_db
        .get_class("archer")
        .expect("Archer class not found");
    let gnome = race_db.get_race("gnome").expect("Gnome race not found");

    let longbow_proficiency = get_weapon_proficiency(WeaponClassification::MartialRanged);
    let longbow_tags = vec!["large_weapon".to_string(), "two_handed".to_string()];

    let can_use = can_character_use_item(
        longbow_proficiency.as_ref(),
        &archer.proficiencies,
        &gnome.proficiencies,
        &longbow_tags,
        &gnome.incompatible_item_tags,
    );

    assert!(
        !can_use,
        "Gnome Archer should NOT be able to use Long Bow (large_weapon tag incompatible with Small size)"
    );
}

#[test]
fn test_race_incompatible_tags_heavy_armor() {
    // Scenario: Gnome Knight CANNOT use Plate Mail
    // Knight class HAS heavy_armor proficiency (satisfies proficiency requirement)
    // Gnome race is Small and has incompatible_item_tags: ["heavy_armor"]
    // Plate Mail has tags: ["heavy_armor"]
    //
    // Expected: FAILURE - race incompatible tag blocks usage despite proficiency

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let knight = class_db
        .get_class("knight")
        .expect("Knight class not found");
    let gnome = race_db.get_race("gnome").expect("Gnome race not found");

    let plate_proficiency = get_armor_proficiency(ArmorClassification::Heavy);
    let plate_tags = vec!["heavy_armor".to_string()];

    let can_use = can_character_use_item(
        plate_proficiency.as_ref(),
        &knight.proficiencies,
        &gnome.proficiencies,
        &plate_tags,
        &gnome.incompatible_item_tags,
    );

    assert!(
        !can_use,
        "Gnome Knight should NOT be able to use Plate Mail (heavy_armor tag incompatible with Small size)"
    );
}

// ===== Phase 5 Task 5.5: Proficiency Does NOT Override Race Tags =====

#[test]
fn test_proficiency_overrides_race_tag() {
    // Scenario: Gnome Archer with BOTH class and race proficiency CANNOT use Long Bow
    // Archer class HAS martial_ranged proficiency
    // Gnome race also has crossbow proficiency (different weapon)
    // Long Bow requires martial_ranged proficiency
    // Long Bow has large_weapon tag, Gnome has incompatible large_weapon tag
    //
    // Expected: FAILURE - proficiency does NOT override race tag restrictions
    //
    // This test verifies the two-step validation:
    // 1. Proficiency check passes (class grants martial_ranged)
    // 2. Tag check fails (gnome incompatible with large_weapon)
    // Result: CANNOT USE (both checks must pass)

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let archer = class_db
        .get_class("archer")
        .expect("Archer class not found");
    let gnome = race_db.get_race("gnome").expect("Gnome race not found");

    let longbow_proficiency = get_weapon_proficiency(WeaponClassification::MartialRanged);
    let longbow_tags = vec!["large_weapon".to_string(), "two_handed".to_string()];

    // Verify proficiency check passes (UNION logic works)
    let has_prof = has_proficiency_union(
        longbow_proficiency.as_ref(),
        &archer.proficiencies,
        &gnome.proficiencies,
    );
    assert!(
        has_prof,
        "Gnome Archer DOES have martial_ranged proficiency (from class)"
    );

    // Verify tag check fails (race restriction applies)
    let tags_ok = !longbow_tags
        .iter()
        .any(|tag| gnome.incompatible_item_tags.contains(tag));
    assert!(
        !tags_ok,
        "Long Bow tags are NOT compatible with Gnome (large_weapon restriction)"
    );

    // Final result: cannot use (proficiency does not override tags)
    let can_use = can_character_use_item(
        longbow_proficiency.as_ref(),
        &archer.proficiencies,
        &gnome.proficiencies,
        &longbow_tags,
        &gnome.incompatible_item_tags,
    );

    assert!(
        !can_use,
        "Gnome Archer should NOT be able to use Long Bow (proficiency does NOT override race tag restrictions)"
    );
}

#[test]
fn test_proficiency_overrides_race_tag_heavy_armor() {
    // Scenario: Gnome Paladin with heavy_armor proficiency CANNOT use Plate Mail
    // Paladin class HAS heavy_armor proficiency
    // Gnome race is Small with incompatible_item_tags: ["heavy_armor"]
    // Plate Mail requires heavy_armor proficiency and has heavy_armor tag
    //
    // Expected: FAILURE - proficiency does NOT override race tag restrictions

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let paladin = class_db
        .get_class("paladin")
        .expect("Paladin class not found");
    let gnome = race_db.get_race("gnome").expect("Gnome race not found");

    let plate_proficiency = get_armor_proficiency(ArmorClassification::Heavy);
    let plate_tags = vec!["heavy_armor".to_string()];

    // Verify proficiency check passes
    let has_prof = has_proficiency_union(
        plate_proficiency.as_ref(),
        &paladin.proficiencies,
        &gnome.proficiencies,
    );
    assert!(
        has_prof,
        "Gnome Paladin DOES have heavy_armor proficiency (from class)"
    );

    // Verify tag check fails
    let tags_ok = !plate_tags
        .iter()
        .any(|tag| gnome.incompatible_item_tags.contains(tag));
    assert!(
        !tags_ok,
        "Plate Mail tags are NOT compatible with Gnome (heavy_armor restriction)"
    );

    // Final result: cannot use
    let can_use = can_character_use_item(
        plate_proficiency.as_ref(),
        &paladin.proficiencies,
        &gnome.proficiencies,
        &plate_tags,
        &gnome.incompatible_item_tags,
    );

    assert!(
        !can_use,
        "Gnome Paladin should NOT be able to use Plate Mail (proficiency does NOT override race tag restrictions)"
    );
}

// ===== Additional Edge Cases =====

#[test]
fn test_gnome_archer_can_use_shortbow() {
    // Scenario: Gnome Archer CAN use Short Bow
    // Archer class HAS martial_ranged proficiency
    // Short Bow requires martial_ranged proficiency
    // Short Bow has NO large_weapon tag (just two_handed)
    // Gnome incompatible_item_tags: ["large_weapon", "heavy_armor"]
    //
    // Expected: SUCCESS - proficiency satisfied, no incompatible tags

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let archer = class_db
        .get_class("archer")
        .expect("Archer class not found");
    let gnome = race_db.get_race("gnome").expect("Gnome race not found");

    let shortbow_proficiency = get_weapon_proficiency(WeaponClassification::MartialRanged);
    let shortbow_tags = vec!["two_handed".to_string()]; // No large_weapon tag

    let can_use = can_character_use_item(
        shortbow_proficiency.as_ref(),
        &archer.proficiencies,
        &gnome.proficiencies,
        &shortbow_tags,
        &gnome.incompatible_item_tags,
    );

    assert!(
        can_use,
        "Gnome Archer should be able to use Short Bow (no large_weapon tag)"
    );
}

#[test]
fn test_human_versatility_no_restrictions() {
    // Scenario: Human Archer CAN use Long Bow
    // Archer class HAS martial_ranged proficiency
    // Human race has NO incompatible_item_tags
    // Long Bow has large_weapon tag
    //
    // Expected: SUCCESS - humans have no size restrictions

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let archer = class_db
        .get_class("archer")
        .expect("Archer class not found");
    let human = race_db.get_race("human").expect("Human race not found");

    let longbow_proficiency = get_weapon_proficiency(WeaponClassification::MartialRanged);
    let longbow_tags = vec!["large_weapon".to_string(), "two_handed".to_string()];

    let can_use = can_character_use_item(
        longbow_proficiency.as_ref(),
        &archer.proficiencies,
        &human.proficiencies,
        &longbow_tags,
        &human.incompatible_item_tags,
    );

    assert!(
        can_use,
        "Human Archer should be able to use Long Bow (humans have no size restrictions)"
    );
}

#[test]
fn test_elf_versatility_no_restrictions() {
    // Scenario: Elf Knight CAN use Plate Mail
    // Knight class HAS heavy_armor proficiency
    // Elf race has NO incompatible_item_tags (medium size, no restrictions)
    // Plate Mail has heavy_armor tag
    //
    // Expected: SUCCESS - elves have no size restrictions despite being graceful

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let knight = class_db
        .get_class("knight")
        .expect("Knight class not found");
    let elf = race_db.get_race("elf").expect("Elf race not found");

    let plate_proficiency = get_armor_proficiency(ArmorClassification::Heavy);
    let plate_tags = vec!["heavy_armor".to_string()];

    let can_use = can_character_use_item(
        plate_proficiency.as_ref(),
        &knight.proficiencies,
        &elf.proficiencies,
        &plate_tags,
        &elf.incompatible_item_tags,
    );

    assert!(
        can_use,
        "Elf Knight should be able to use Plate Mail (elves have no incompatible tags)"
    );
}

#[test]
fn test_item_with_no_proficiency_requirement() {
    // Scenario: ALL characters can use items with no proficiency requirement
    // Test with Gnome Sorcerer (least proficiencies) and a basic item
    //
    // Expected: SUCCESS - no proficiency requirement means anyone can use it

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let sorcerer = class_db
        .get_class("sorcerer")
        .expect("Sorcerer class not found");
    let gnome = race_db.get_race("gnome").expect("Gnome race not found");

    let no_proficiency: Option<ProficiencyId> = None;
    let item_tags: Vec<String> = vec![];

    let can_use = can_character_use_item(
        no_proficiency.as_ref(),
        &sorcerer.proficiencies,
        &gnome.proficiencies,
        &item_tags,
        &gnome.incompatible_item_tags,
    );

    assert!(
        can_use,
        "Gnome Sorcerer should be able to use items with no proficiency requirement"
    );
}

#[test]
fn test_item_with_no_tags() {
    // Scenario: Items with no tags can be used by characters with incompatible_item_tags
    // if proficiency is satisfied
    // Test Gnome Archer with a martial_ranged weapon that has no large_weapon tag
    //
    // Expected: SUCCESS - no conflicting tags

    let (class_db, race_db, _prof_db) = load_tutorial_databases();

    let archer = class_db
        .get_class("archer")
        .expect("Archer class not found");
    let gnome = race_db.get_race("gnome").expect("Gnome race not found");

    let weapon_proficiency = get_weapon_proficiency(WeaponClassification::MartialRanged);
    let no_tags: Vec<String> = vec![];

    let can_use = can_character_use_item(
        weapon_proficiency.as_ref(),
        &archer.proficiencies,
        &gnome.proficiencies,
        &no_tags,
        &gnome.incompatible_item_tags,
    );

    assert!(
        can_use,
        "Gnome Archer should be able to use martial_ranged weapons with no large_weapon tag"
    );
}
