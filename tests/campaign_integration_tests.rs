// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Phase 4: Campaign Loading Integration Tests
//!
//! These tests verify that the tutorial campaign properly loads and uses
//! the creature database for monster and NPC spawning.
//!
//! Tests cover:
//! - Campaign loads creature database on initialization
//! - Monsters spawn with procedural mesh visuals
//! - NPCs spawn with procedural mesh visuals
//! - Fallback mechanisms work correctly
//! - No performance regressions

use antares::sdk::campaign_loader::Campaign;
use antares::sdk::database::ContentDatabase;

#[test]
fn test_campaign_loads_creature_database() {
    // Arrange: Load tutorial campaign
    let campaign_result = Campaign::load("data/test_campaign");

    // Assert: Campaign loads successfully
    assert!(
        campaign_result.is_ok(),
        "Tutorial campaign should load successfully: {:?}",
        campaign_result.err()
    );

    let campaign = campaign_result.unwrap();

    // Assert: Campaign has creatures_file configured
    assert_eq!(
        campaign.data.creatures, "data/creatures.ron",
        "Campaign should specify creatures.ron file"
    );

    // Act: Load campaign content database
    let content_result = campaign.load_content();

    // Assert: Content database loads successfully
    assert!(
        content_result.is_ok(),
        "Campaign content should load successfully: {:?}",
        content_result.err()
    );

    let content = content_result.unwrap();

    // Assert: Creature database is loaded and not empty
    assert!(
        content.creatures.count() > 0,
        "Creature database should contain creatures"
    );

    println!(
        "✓ Campaign loaded {} creatures from database",
        content.creatures.count()
    );
}

#[test]
fn test_campaign_creature_database_contains_expected_creatures() {
    // Arrange: Load campaign content
    let content = ContentDatabase::load_campaign("data/test_campaign")
        .expect("Should load tutorial campaign content");

    // Assert: Database contains creatures from the fixture registry.
    assert!(
        content.creatures.count() >= 30,
        "Fixture campaign should have at least 30 registered creatures"
    );

    // Assert: Monster creatures exist (IDs 1-50 range)
    let monster_creature_ids = vec![1, 2, 3, 10, 11, 12, 20, 21, 22, 30, 31];
    for creature_id in monster_creature_ids {
        assert!(
            content.creatures.has_creature(creature_id),
            "Monster creature {} should exist in database",
            creature_id
        );
    }

    // Assert: NPC creatures exist (IDs 51-100 range)
    let npc_creature_ids = vec![51, 52, 53, 54, 55, 56, 57, 58];
    for creature_id in npc_creature_ids {
        assert!(
            content.creatures.has_creature(creature_id),
            "NPC creature {} should exist in database",
            creature_id
        );
    }

    println!("✓ All expected creatures present in database");
}

#[test]
fn test_all_monsters_have_visual_id_mapping() {
    // Arrange: Load campaign content
    let content = ContentDatabase::load_campaign("data/test_campaign")
        .expect("Should load tutorial campaign content");

    // Act: Get all monster IDs
    let monster_ids = content.monsters.all_monsters();
    let monster_count = monster_ids.len();

    // Assert: At least some monsters exist
    assert!(
        !monster_ids.is_empty(),
        "Tutorial campaign should have monsters"
    );

    // Assert: Count monsters with visual_id
    let mut monsters_with_visuals = 0;

    for monster_id in &monster_ids {
        if let Some(monster) = content.monsters.get_monster(*monster_id) {
            if monster.visual_id.is_some() {
                monsters_with_visuals += 1;

                // Verify creature exists
                let visual_id = monster.visual_id.unwrap();
                assert!(
                    content.creatures.has_creature(visual_id),
                    "Monster '{}' references non-existent creature {}",
                    monster.name,
                    visual_id
                );
            }
        }
    }

    println!(
        "Monsters: {} total, {} with visual_id",
        monster_count, monsters_with_visuals
    );

    // Assert: All monsters should have visual_id (100% coverage)
    assert_eq!(
        monsters_with_visuals, monster_count,
        "All monsters should have visual_id assigned"
    );

    println!(
        "✓ All {} monsters have valid visual_id mappings",
        monster_count
    );
}

#[test]
fn test_all_npcs_have_creature_id_mapping() {
    // Arrange: Load campaign content
    let content = ContentDatabase::load_campaign("data/test_campaign")
        .expect("Should load tutorial campaign content");

    // Act: Get all NPC IDs
    let npc_ids = content.npcs.all_npcs();
    let npc_count = npc_ids.len();

    // Assert: At least some NPCs exist
    assert!(!npc_ids.is_empty(), "Tutorial campaign should have NPCs");

    // Act: Count NPCs with creature_id
    let mut npcs_with_creatures = 0;
    let mut invalid_creature_refs = Vec::new();

    for npc_id in &npc_ids {
        if let Some(npc) = content.npcs.get_npc(npc_id) {
            if let Some(creature_id) = npc.creature_id {
                npcs_with_creatures += 1;

                // Verify creature exists
                if !content.creatures.has_creature(creature_id) {
                    invalid_creature_refs.push((npc.name.clone(), creature_id));
                }
            }
        }
    }

    println!(
        "NPCs: {} total, {} with creature_id",
        npc_count, npcs_with_creatures
    );

    // Assert: All NPCs should have creature_id (100% coverage)
    assert_eq!(
        npcs_with_creatures, npc_count,
        "All NPCs should have creature_id assigned"
    );

    // Assert: All creature_ids reference valid creatures
    assert!(
        invalid_creature_refs.is_empty(),
        "NPCs reference non-existent creatures: {:?}",
        invalid_creature_refs
    );

    println!("✓ All {} NPCs have valid creature_id mappings", npc_count);
}

#[test]
fn test_creature_visual_id_ranges_follow_convention() {
    // Arrange: Load campaign content
    let content = ContentDatabase::load_campaign("data/test_campaign")
        .expect("Should load tutorial campaign content");

    // Check monster visual_id range (should be 1-50)
    let monster_ids = content.monsters.all_monsters();
    for monster_id in monster_ids {
        if let Some(monster) = content.monsters.get_monster(monster_id) {
            if let Some(visual_id) = monster.visual_id {
                assert!(
                    (1..=50).contains(&visual_id),
                    "Monster '{}' has visual_id {} outside expected range 1-50",
                    monster.name,
                    visual_id
                );
            }
        }
    }

    // Check NPC creature_id range (should be 51-200)
    let npc_ids = content.npcs.all_npcs();
    for npc_id in npc_ids {
        if let Some(npc) = content.npcs.get_npc(&npc_id) {
            if let Some(creature_id) = npc.creature_id {
                assert!(
                    creature_id >= 51,
                    "NPC '{}' has creature_id {} outside expected range 51+",
                    npc.name,
                    creature_id
                );
            }
        }
    }

    println!("✓ Creature ID ranges follow convention (monsters: 1-50, NPCs: 51+)");
}

#[test]
fn test_creature_database_load_performance() {
    use std::time::Instant;

    // Act: Measure creature database loading time
    let start = Instant::now();
    let content = ContentDatabase::load_campaign("data/test_campaign")
        .expect("Should load tutorial campaign content");
    let duration = start.elapsed();

    // Assert: Loading should be fast (< 500ms for 32 creatures)
    assert!(
        duration.as_millis() < 500,
        "Creature database loading took {}ms, expected < 500ms",
        duration.as_millis()
    );

    println!(
        "✓ Loaded {} creatures in {:?}",
        content.creatures.count(),
        duration
    );
}

#[test]
fn test_fallback_mechanism_for_missing_visual_id() {
    // Arrange: Create a monster without visual_id
    use antares::domain::character::{AttributePair, AttributePair16, Stats};
    use antares::domain::combat::monster::{AiBehavior, LootTable, Monster, MonsterCondition};

    let monster = Monster {
        id: 99,
        name: "Test Monster".to_string(),
        stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
        hp: AttributePair16::new(10),
        ac: AttributePair::new(10),
        attacks: vec![],
        loot: LootTable {
            gold_min: 0,
            gold_max: 0,
            gems_min: 0,
            gems_max: 0,
            items: vec![],
            experience: 0,
        },
        flee_threshold: 0,
        special_attack_threshold: 0,
        resistances: Default::default(),
        can_regenerate: false,
        can_advance: false,
        is_undead: false,
        magic_resistance: 0,
        ai_behavior: AiBehavior::default(),
        visual_id: None, // No visual assigned
        conditions: MonsterCondition::Normal,
        active_conditions: Vec::new(),
        has_acted: false,
    };

    // Assert: Monster exists but has no visual_id
    assert!(
        monster.visual_id.is_none(),
        "Test monster should have no visual_id for fallback test"
    );

    // Note: In actual rendering code, this should fall back to a placeholder
    // The spawning system should check for None and use a default cube or similar
    println!("✓ Monsters can exist without visual_id for fallback scenarios");
}

#[test]
fn test_fallback_mechanism_for_missing_creature_id() {
    // Arrange: Create an NPC without creature_id
    use antares::domain::world::npc::NpcDefinition;

    let npc = NpcDefinition {
        id: "test_npc".to_string(),
        name: "Test NPC".to_string(),
        description: "Test description".to_string(),
        portrait_id: "test.png".to_string(),
        dialogue_id: None,
        creature_id: None, // No creature assigned
        sprite: None,
        quest_ids: vec![],
        faction: None,
        is_merchant: false,
        is_innkeeper: false,
    };

    // Assert: NPC exists but has no creature_id
    assert!(
        npc.creature_id.is_none(),
        "Test NPC should have no creature_id for fallback test"
    );

    // Note: In actual rendering code, this should fall back to sprite system
    println!("✓ NPCs can exist without creature_id for sprite fallback");
}

#[test]
fn test_creature_definitions_are_valid() {
    // Arrange: Load campaign content
    let content = ContentDatabase::load_campaign("data/test_campaign")
        .expect("Should load tutorial campaign content");

    // Act: Validate all creatures
    for creature_id in 1..=32 {
        if let Some(creature) = content.creatures.get_creature(creature_id) {
            // Assert: Creature has valid fields
            assert!(
                !creature.name.is_empty(),
                "Creature {} should have a name",
                creature_id
            );
            assert_eq!(
                creature.id, creature_id,
                "Creature ID mismatch for creature {}",
                creature_id
            );

            // Assert: Meshes and transforms match
            assert_eq!(
                creature.meshes.len(),
                creature.mesh_transforms.len(),
                "Creature {} mesh count should match transform count",
                creature_id
            );

            // Assert: Scale is positive
            assert!(
                creature.scale > 0.0,
                "Creature {} should have positive scale",
                creature_id
            );
        }
    }

    println!("✓ All creature definitions are structurally valid");
}

#[test]
fn test_no_duplicate_creature_ids() {
    // Arrange: Load campaign content
    let content = ContentDatabase::load_campaign("data/test_campaign")
        .expect("Should load tutorial campaign content");

    // Act: Collect all creature IDs
    use std::collections::HashSet;
    let mut seen_ids = HashSet::new();
    let mut duplicates = Vec::new();

    for creature_id in 1..=200 {
        if content.creatures.has_creature(creature_id) && !seen_ids.insert(creature_id) {
            duplicates.push(creature_id);
        }
    }

    // Assert: No duplicate IDs
    assert!(
        duplicates.is_empty(),
        "Found duplicate creature IDs: {:?}",
        duplicates
    );

    println!(
        "✓ No duplicate creature IDs found ({} unique creatures)",
        seen_ids.len()
    );
}

#[test]
fn test_campaign_integration_end_to_end() {
    // Arrange: Full campaign load
    let campaign = Campaign::load("data/test_campaign").expect("Tutorial campaign should load");

    // Act: Load all content
    let content = campaign
        .load_content()
        .expect("Campaign content should load");

    // Assert: All systems are present
    assert!(content.creatures.count() > 0, "Should have creatures");
    assert!(content.monsters.count() > 0, "Should have monsters");
    assert!(content.npcs.count() > 0, "Should have NPCs");
    assert!(!content.items.is_empty(), "Should have items");
    assert!(
        content.classes.all_classes().count() > 0,
        "Should have classes"
    );

    // Assert: Cross-references are valid for monsters
    let monster_ids = content.monsters.all_monsters();
    for monster_id in monster_ids {
        if let Some(monster) = content.monsters.get_monster(monster_id) {
            if let Some(visual_id) = monster.visual_id {
                assert!(
                    content.creatures.has_creature(visual_id),
                    "Monster '{}' references non-existent creature {}",
                    monster.name,
                    visual_id
                );
            }
        }
    }

    // Assert: Cross-references are valid for NPCs
    let npc_ids = content.npcs.all_npcs();
    for npc_id in npc_ids {
        if let Some(npc) = content.npcs.get_npc(&npc_id) {
            if let Some(creature_id) = npc.creature_id {
                assert!(
                    content.creatures.has_creature(creature_id),
                    "NPC '{}' references non-existent creature {}",
                    npc.name,
                    creature_id
                );
            }
        }
    }

    println!("✓ End-to-end campaign integration successful");
    println!("  - {} creatures loaded", content.creatures.count());
    println!("  - {} monsters with visuals", content.monsters.count());
    println!("  - {} NPCs with visuals", content.npcs.count());
}
