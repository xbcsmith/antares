// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration test for tutorial campaign monster-to-creature visual mappings
//!
//! This test validates that all monsters in the tutorial campaign have valid
//! visual_id references that point to existing creatures in the creature database.
//!
//! Part of Phase 2: Monster Visual Mapping implementation.

use antares::domain::combat::database::MonsterDatabase;
use antares::domain::visual::creature_database::CreatureDatabase;
use std::path::Path;

#[test]
fn test_tutorial_monster_creature_mapping_complete() {
    let monsters_path = "campaigns/tutorial/data/monsters.ron";
    let creatures_path = "campaigns/tutorial/data/creatures.ron";

    // Skip if files don't exist (running tests outside project root)
    if !std::path::Path::new(monsters_path).exists()
        || !std::path::Path::new(creatures_path).exists()
    {
        println!("Skipping test - tutorial campaign data files not found");
        return;
    }

    // Load both databases
    let monster_db =
        MonsterDatabase::load_from_file(monsters_path).expect("Failed to load monsters");

    // Load creatures and build database
    let creatures = CreatureDatabase::load_from_file(Path::new(creatures_path))
        .expect("Failed to load creatures");
    let mut creature_db = CreatureDatabase::new();
    for creature in creatures {
        creature_db
            .add_creature(creature)
            .expect("Failed to add creature");
    }

    // Expected monster-to-creature mappings from Phase 2
    let expected_mappings = [
        (1, 1, "Goblin", "Goblin"),
        (2, 3, "Kobold", "Kobold"),
        (3, 4, "Giant Rat", "GiantRat"),
        (10, 7, "Orc", "Orc"),
        (11, 5, "Skeleton", "Skeleton"),
        (12, 2, "Wolf", "Wolf"),
        (20, 8, "Ogre", "Ogre"),
        (21, 6, "Zombie", "Zombie"),
        (22, 9, "Fire Elemental", "FireElemental"),
        (30, 30, "Dragon", "Dragon"),
        (31, 10, "Lich", "Lich"),
    ];

    println!("Validating monster-to-creature mappings...");

    for (monster_id, expected_creature_id, monster_name, expected_creature_name) in
        expected_mappings
    {
        // Verify monster exists
        let monster = monster_db
            .get_monster(monster_id)
            .unwrap_or_else(|| panic!("Monster {} not found", monster_id));

        assert_eq!(
            monster.name, monster_name,
            "Monster {} name mismatch",
            monster_id
        );

        // Verify visual_id is set
        assert!(
            monster.visual_id.is_some(),
            "Monster {} ({}) missing visual_id",
            monster_id,
            monster.name
        );

        let visual_id = monster.visual_id.unwrap();
        assert_eq!(
            visual_id, expected_creature_id,
            "Monster {} ({}) has wrong visual_id: expected {}, got {}",
            monster_id, monster.name, expected_creature_id, visual_id
        );

        // Verify creature exists
        let creature = creature_db.get_creature(visual_id).unwrap_or_else(|| {
            panic!(
                "Creature {} not found (referenced by monster {})",
                visual_id, monster_id
            )
        });

        assert_eq!(
            creature.name, expected_creature_name,
            "Creature {} name mismatch",
            visual_id
        );

        println!(
            "✓ Monster {} ({}) -> Creature {} ({})",
            monster_id, monster.name, visual_id, creature.name
        );
    }

    println!("All monster-to-creature mappings validated successfully!");
}

#[test]
fn test_all_tutorial_monsters_have_visuals() {
    let monsters_path = "campaigns/tutorial/data/monsters.ron";

    if !std::path::Path::new(monsters_path).exists() {
        println!("Skipping test - monsters.ron not found");
        return;
    }

    let monster_db =
        MonsterDatabase::load_from_file(monsters_path).expect("Failed to load monsters");

    let total_monsters = monster_db.len();
    let mut monsters_with_visuals = 0;
    let mut monsters_without_visuals = Vec::new();

    for monster in monster_db.all_monsters() {
        if monster.visual_id.is_some() {
            monsters_with_visuals += 1;
        } else {
            monsters_without_visuals.push((monster.id, monster.name.clone()));
        }
    }

    assert_eq!(
        total_monsters, 11,
        "Expected 11 monsters in tutorial campaign, found {}",
        total_monsters
    );

    assert_eq!(
        monsters_with_visuals, 11,
        "Expected all 11 monsters to have visual_id set, only {} have it",
        monsters_with_visuals
    );

    assert!(
        monsters_without_visuals.is_empty(),
        "The following monsters are missing visual_id: {:?}",
        monsters_without_visuals
    );

    println!(
        "✓ All {} tutorial monsters have visual_id set",
        total_monsters
    );
}

#[test]
fn test_no_broken_creature_references() {
    let monsters_path = "campaigns/tutorial/data/monsters.ron";
    let creatures_path = "campaigns/tutorial/data/creatures.ron";

    if !std::path::Path::new(monsters_path).exists()
        || !std::path::Path::new(creatures_path).exists()
    {
        println!("Skipping test - campaign data files not found");
        return;
    }

    let monster_db =
        MonsterDatabase::load_from_file(monsters_path).expect("Failed to load monsters");

    // Load creatures and build database
    let creatures = CreatureDatabase::load_from_file(Path::new(creatures_path))
        .expect("Failed to load creatures");
    let mut creature_db = CreatureDatabase::new();
    for creature in creatures {
        creature_db
            .add_creature(creature)
            .expect("Failed to add creature");
    }

    let mut broken_references = Vec::new();

    for monster in monster_db.all_monsters() {
        if let Some(visual_id) = monster.visual_id {
            if creature_db.get_creature(visual_id).is_none() {
                broken_references.push((monster.id, monster.name.clone(), visual_id));
            }
        }
    }

    assert!(
        broken_references.is_empty(),
        "Found broken creature references: {:?}",
        broken_references
    );

    println!("✓ No broken creature references found");
}

#[test]
fn test_creature_database_has_expected_creatures() {
    let creatures_path = "campaigns/tutorial/data/creatures.ron";

    if !std::path::Path::new(creatures_path).exists() {
        println!("Skipping test - creatures.ron not found");
        return;
    }

    // Load creatures and build database
    let creatures = CreatureDatabase::load_from_file(Path::new(creatures_path))
        .expect("Failed to load creatures");
    let mut creature_db = CreatureDatabase::new();
    for creature in creatures {
        creature_db
            .add_creature(creature)
            .expect("Failed to add creature");
    }

    // All creature IDs that should exist based on monster mappings
    let required_creature_ids = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 30];

    for creature_id in required_creature_ids {
        assert!(
            creature_db.has_creature(creature_id),
            "Required creature {} not found in database",
            creature_id
        );
    }

    println!("✓ All required creatures exist in database");
}
