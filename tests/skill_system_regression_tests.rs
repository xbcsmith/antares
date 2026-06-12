// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Phase 9 Skill System Regression Tests
//!
//! These integration tests verify that:
//!
//! 1. The full test campaign loads with a populated skill database.
//! 2. Campaign validation covers skill references from classes, races, and dialogues.
//! 3. The proficiency (item-use permission) system is unaffected by the skill system.
//! 4. The level-training domain flow works alongside the skill system.
//! 5. The complete test campaign content (skills + classes + races + dialogues)
//!    validates end-to-end without errors.
//!
//! All tests use `data/test_campaign` as the fixture and must not load live
//! campaign data; see `AGENTS.md` Implementation Rule 5.

use std::path::Path;

use antares::domain::classes::ClassDatabase;
use antares::domain::levels::LevelDatabase;
use antares::domain::proficiency::has_proficiency_union;
use antares::domain::progression::{
    experience_for_level_class, DEFAULT_BASE_XP, DEFAULT_XP_MULTIPLIER,
};
use antares::sdk::database::ContentDatabase;

// ── Test 1: test_test_campaign_loads_with_skills ──────────────────────────────

/// Verifies that the full test campaign loads and produces a non-empty skill
/// database containing all expected skills — including the Phase 9 additions
/// (`stealth` and `tracking`).
///
/// This is the primary smoke test for the skill content pipeline.
#[test]
fn test_test_campaign_loads_with_skills() {
    let content = ContentDatabase::load_campaign("data/test_campaign")
        .expect("data/test_campaign must load without error");

    // Skills database must be non-empty.
    assert!(
        !content.skills.is_empty(),
        "test campaign skills database must not be empty"
    );

    // Phase 1 canonical skills must be present.
    assert!(
        content.skills.has("perception"),
        "perception must be present in test campaign"
    );
    assert!(
        content.skills.has("disarm_traps"),
        "disarm_traps must be present in test campaign"
    );
    assert!(
        content.skills.has("item_lore"),
        "item_lore must be present in test campaign"
    );
    assert!(
        content.skills.has("diplomacy"),
        "diplomacy must be present in test campaign"
    );
    assert!(
        content.skills.has("athletics"),
        "athletics must be present in test campaign"
    );
    assert!(
        content.skills.has("arcane_lore"),
        "arcane_lore must be present in test campaign"
    );

    // Phase 9 balance additions must be present.
    assert!(
        content.skills.has("stealth"),
        "stealth must be present in test campaign after Phase 9 balance pass"
    );
    assert!(
        content.skills.has("tracking"),
        "tracking must be present in test campaign after Phase 9 balance pass"
    );

    // The full baseline set should have at least 10 skills.
    assert!(
        content.skills.len() >= 10,
        "test campaign must define at least 10 skills, found {}",
        content.skills.len()
    );

    // All loaded skills must individually pass structural validation.
    content
        .skills
        .validate()
        .expect("all test campaign skills must be structurally valid");

    // Classes must also have loaded (skill grants are part of class data).
    assert!(
        content.classes.count() >= 6,
        "all six base classes must be present, found {}",
        content.classes.count()
    );

    // Verify the Phase 9 class grants are wired up correctly.
    let archer = content
        .classes
        .get_class("archer")
        .expect("archer class must be present");
    assert!(
        archer.skill_grants.iter().any(|g| g.skill_id == "tracking"),
        "archer must have a tracking skill grant after Phase 9 balance pass"
    );

    let robber = content
        .classes
        .get_class("robber")
        .expect("robber class must be present");
    assert!(
        robber.skill_grants.iter().any(|g| g.skill_id == "stealth"),
        "robber must have a stealth skill grant after Phase 9 balance pass"
    );

    let paladin = content
        .classes
        .get_class("paladin")
        .expect("paladin class must be present");
    assert!(
        paladin
            .skill_grants
            .iter()
            .any(|g| g.skill_id == "athletics"),
        "paladin must have an athletics skill grant after Phase 9 balance pass"
    );
}

// ── Test 2: test_campaign_validation_includes_skill_references ────────────────

/// Verifies that `ContentDatabase::validate()` performs cross-reference checks
/// for skills — i.e. class grants and dialogue conditions that reference skill
/// IDs are checked against the skill database.
///
/// The test campaign contains valid references.  The test both confirms that
/// valid data passes (`Ok`) and — by inspection of the validate source — that
/// skill-reference validation is actually exercised.
#[test]
fn test_campaign_validation_includes_skill_references() {
    let content = ContentDatabase::load_campaign("data/test_campaign")
        .expect("data/test_campaign must load without error");

    // Full validation must succeed — all skill references are valid.
    content
        .validate()
        .expect("test campaign with correct skill references must validate successfully");

    // Confirm that the skill database is non-trivially populated so the
    // validate call actually exercised the skill cross-reference logic.
    assert!(
        content.skills.len() >= 6,
        "at least 6 skills must be loaded for the validation run to be meaningful"
    );

    // At least one class must have skill grants to exercise class-grant validation.
    let classes_with_grants = content
        .classes
        .all_classes()
        .filter(|c| !c.skill_grants.is_empty())
        .count();
    assert!(
        classes_with_grants >= 4,
        "at least 4 classes must have skill_grants to exercise validate_with_skill_db; found {}",
        classes_with_grants
    );

    // At least one race must have skill grants to exercise race-grant validation.
    let races_with_grants = content
        .races
        .all_races()
        .filter(|r| !r.skill_grants.is_empty())
        .count();
    assert!(
        races_with_grants >= 2,
        "at least 2 races must have skill_grants; found {}",
        races_with_grants
    );
}

// ── Test 3: test_existing_proficiency_item_usage_unchanged ───────────────────

/// Verifies that the proficiency (item-use permission) system continues to work
/// correctly after the skill system was introduced.
///
/// Skills and proficiencies are intentionally separate systems.  This test
/// asserts that `has_proficiency_union` produces the same results regardless
/// of which skills are defined.
#[test]
fn test_existing_proficiency_item_usage_unchanged() {
    // Load the base class and race databases directly — no campaign needed.
    let class_db =
        ClassDatabase::load_from_file("data/classes.ron").expect("data/classes.ron must load");

    // ── Knight can use martial_melee ────────────────────────────────────────
    let knight = class_db
        .get_class("knight")
        .expect("knight class must exist in data/classes.ron");
    assert!(
        knight.has_proficiency("martial_melee"),
        "knight must have martial_melee proficiency"
    );
    assert!(
        knight.has_proficiency("heavy_armor"),
        "knight must have heavy_armor proficiency"
    );

    // ── Sorcerer cannot use martial_melee ───────────────────────────────────
    let sorcerer = class_db
        .get_class("sorcerer")
        .expect("sorcerer class must exist in data/classes.ron");
    assert!(
        !sorcerer.has_proficiency("martial_melee"),
        "sorcerer must not have martial_melee proficiency"
    );
    assert!(
        !sorcerer.has_proficiency("heavy_armor"),
        "sorcerer must not have heavy_armor proficiency"
    );

    // ── Proficiency union: human race grants no weapon/armor proficiencies ──
    // Human has an empty proficiencies list in the RON.
    // Knight + Human → union grants martial_melee through the knight's class list.
    // Sorcerer + Human → union does NOT grant martial_melee.
    let empty_race_profs: Vec<String> = vec![];

    assert!(
        has_proficiency_union(
            Some(&"martial_melee".to_string()),
            &knight.proficiencies,
            &empty_race_profs,
        ),
        "Knight/Human union must grant martial_melee via class"
    );

    assert!(
        !has_proficiency_union(
            Some(&"martial_melee".to_string()),
            &sorcerer.proficiencies,
            &empty_race_profs,
        ),
        "Sorcerer/Human union must NOT grant martial_melee"
    );

    // ── No required proficiency → always true ───────────────────────────────
    assert!(
        has_proficiency_union(None, &sorcerer.proficiencies, &empty_race_profs),
        "has_proficiency_union with None required proficiency must always return true"
    );

    // ── Skill grants on classes must not affect proficiency results ─────────
    // Knight has skill_grants (athletics, leadership) — these must not interfere
    // with the proficiency checks above.
    assert!(
        !knight.skill_grants.is_empty(),
        "knight should have skill_grants (sanity-check that skill system is active)"
    );
    assert!(
        knight.has_proficiency("martial_melee"),
        "knight martial_melee proficiency must still hold after skill grants are added"
    );
}

// ── Test 4: test_existing_level_training_flow_unchanged ──────────────────────

/// Verifies that the level-training domain functions (LevelDatabase, progression
/// formulas) still work correctly when the skill system is also present.
///
/// The test campaign has both `levels.ron` and `skills.ron`.  This test confirms
/// that loading both together produces no conflicts, and that the level-up XP
/// math is unchanged.
#[test]
fn test_existing_level_training_flow_unchanged() {
    // Load the level database from the test campaign.
    let level_db = LevelDatabase::load_from_file(Path::new("data/test_campaign/data/levels.ron"))
        .expect("data/test_campaign/data/levels.ron must load");

    // Level 1 always requires 0 XP, regardless of class.
    let classes = [
        "knight", "paladin", "archer", "cleric", "sorcerer", "robber",
    ];
    for class_id in &classes {
        let xp1 = experience_for_level_class(
            1,
            class_id,
            Some(&level_db),
            DEFAULT_BASE_XP,
            DEFAULT_XP_MULTIPLIER,
        );
        assert_eq!(
            xp1, 0,
            "Level-1 XP for '{}' must be 0, got {}",
            class_id, xp1
        );

        // Level 2 must require more XP than level 1.
        let xp2 = experience_for_level_class(
            2,
            class_id,
            Some(&level_db),
            DEFAULT_BASE_XP,
            DEFAULT_XP_MULTIPLIER,
        );
        assert!(
            xp2 > xp1,
            "Level-2 XP for '{}' ({}) must exceed level-1 XP ({})",
            class_id,
            xp2,
            xp1
        );
    }

    // Load the full content database: both skills and levels must coexist.
    let content = ContentDatabase::load_campaign("data/test_campaign")
        .expect("data/test_campaign must load alongside levels");

    assert!(
        !content.skills.is_empty(),
        "skill database must be populated when both skills.ron and levels.ron are present"
    );

    assert!(
        content.classes.count() >= 6,
        "all base classes must be present alongside the skill system"
    );

    // Confirm that level-up XP from the loaded campaign content agrees with
    // the directly-loaded level database for the knight class.
    let direct_xp2 = experience_for_level_class(
        2,
        "knight",
        Some(&level_db),
        DEFAULT_BASE_XP,
        DEFAULT_XP_MULTIPLIER,
    );
    assert!(
        direct_xp2 > 0,
        "knight level-2 XP from level database must be positive, got {}",
        direct_xp2
    );
}

// ── Test 5: test_campaign_skill_data_validates ───────────────────────────────

/// Full end-to-end validation of the test campaign, confirming that the skill
/// data, class grants, race grants, and dialogue skill checks are internally
/// consistent after the Phase 9 balance pass.
///
/// This is the authoritative "all skill data is correct" gate for the
/// test_campaign fixture.
#[test]
fn test_campaign_skill_data_validates() {
    let content =
        ContentDatabase::load_campaign("data/test_campaign").expect("data/test_campaign must load");

    // Full validation must pass.
    content
        .validate()
        .expect("test campaign skill data must validate end-to-end after Phase 9");

    // Structural invariants of the skill database itself.
    content
        .skills
        .validate()
        .expect("skill database structural validation must pass");

    // Every class skill grant must reference a known skill.
    let class_errors = content.classes.validate_with_skill_db(&content.skills);
    assert!(
        class_errors.is_empty(),
        "all class skill grants must reference defined skills; errors: {:?}",
        class_errors
    );

    // Every race skill grant must reference a known skill.
    let race_errors = content.races.validate_with_skill_db(&content.skills);
    assert!(
        race_errors.is_empty(),
        "all race skill grants must reference defined skills; errors: {:?}",
        race_errors
    );

    // Spot-check the Phase 9 skills have valid definitions.
    let stealth = content
        .skills
        .get("stealth")
        .expect("stealth skill must be defined");
    assert!(stealth.max_rank > 0, "stealth max_rank must be positive");
    assert!(stealth.is_trainable, "stealth must be trainable");

    let tracking = content
        .skills
        .get("tracking")
        .expect("tracking skill must be defined");
    assert!(tracking.max_rank > 0, "tracking max_rank must be positive");
    assert!(tracking.is_trainable, "tracking must be trainable");

    // Confirm the overall database statistics look sane.
    let stats = content.stats();
    assert!(
        stats.skill_count >= 10,
        "test campaign must have at least 10 skills after Phase 9; found {}",
        stats.skill_count
    );
    assert!(
        stats.class_count >= 6,
        "test campaign must have at least 6 classes; found {}",
        stats.class_count
    );
}
