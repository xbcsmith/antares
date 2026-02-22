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
fn test_monster_creature_mappings_are_valid() {
    let monsters_path = "data/test_campaign/data/monsters.ron";
    let creatures_path = "data/test_campaign/data/creatures.ron";
    let data_root = Path::new("data/test_campaign");

    if !std::path::Path::new(monsters_path).exists() {
        println!("Skipping test - data/monsters.ron not found");
        return;
    }

    let monster_db =
        MonsterDatabase::load_from_file(monsters_path).expect("Failed to load monsters");
    let creature_db = if std::path::Path::new(creatures_path).exists() {
        CreatureDatabase::load_from_registry(Path::new(creatures_path), data_root)
            .expect("Failed to load creatures")
    } else {
        CreatureDatabase::new()
    };

    println!("Validating monster-to-creature mappings from global data...");

    for monster in monster_db.all_monsters() {
        assert!(
            monster.visual_id.is_some(),
            "Monster {} ({}) missing visual_id",
            monster.id,
            monster.name
        );
        let visual_id = monster.visual_id.unwrap();
        assert!(
            creature_db.has_creature(visual_id),
            "Monster {} ({}) references missing creature {}",
            monster.id,
            monster.name,
            visual_id
        );
    }
    println!("✓ All monsters reference existing creature visuals");
}

#[test]
fn test_all_monsters_have_visuals() {
    let monsters_path = "data/test_campaign/data/monsters.ron";

    if !std::path::Path::new(monsters_path).exists() {
        println!("Skipping test - data/monsters.ron not found");
        return;
    }

    let monster_db =
        MonsterDatabase::load_from_file(monsters_path).expect("Failed to load monsters");

    let total_monsters = monster_db.len();
    assert!(
        total_monsters > 0,
        "Global monsters database should contain at least one entry"
    );

    let mut monsters_without_visuals = Vec::new();
    for mon in monster_db.all_monsters() {
        if mon.visual_id.is_none() {
            monsters_without_visuals.push((mon.id, mon.name.clone()));
        }
    }

    assert!(
        monsters_without_visuals.is_empty(),
        "The following monsters are missing visual_id: {:?}",
        monsters_without_visuals
    );

    println!("✓ All {} monsters have visual_id set", total_monsters);
}

#[test]
fn test_no_broken_creature_references() {
    let monsters_path = "data/test_campaign/data/monsters.ron";
    let creatures_path = "data/test_campaign/data/creatures.ron";
    let data_root = Path::new("data/test_campaign");

    if !std::path::Path::new(monsters_path).exists() {
        println!("Skipping test - data/monsters.ron not found");
        return;
    }

    let monster_db =
        MonsterDatabase::load_from_file(monsters_path).expect("Failed to load monsters");

    let creature_db = if std::path::Path::new(creatures_path).exists() {
        CreatureDatabase::load_from_registry(Path::new(creatures_path), data_root)
            .expect("Failed to load creatures")
    } else {
        CreatureDatabase::new()
    };

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
    let creatures_path = "data/test_campaign/data/creatures.ron";
    let data_root = Path::new("data/test_campaign");

    if !std::path::Path::new(creatures_path).exists() {
        println!("Skipping test - data/creatures.ron not found");
        return;
    }

    let creature_db = CreatureDatabase::load_from_registry(Path::new(creatures_path), data_root)
        .expect("Failed to load creatures");

    assert!(
        creature_db.count() > 0,
        "Creature database should contain at least one entry"
    );

    println!("✓ Creature database has {} entries", creature_db.count());
}
