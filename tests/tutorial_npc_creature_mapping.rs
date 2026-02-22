// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for Phase 3: NPC Procedural Mesh Integration
//!
//! This test suite validates that:
//! 1. All tutorial NPCs have valid creature_id references
//! 2. All referenced creature IDs exist in the creatures database
//! 3. NPC definitions parse correctly with creature_id field
//! 4. Backward compatibility is maintained for NPCs without creature_id

use antares::domain::types::CreatureId;
use antares::domain::world::npc::NpcDefinition;
use antares::sdk::campaign_loader::Campaign;
use std::collections::HashSet;

#[test]
fn test_tutorial_npc_creature_mapping_complete() {
    // Load the tutorial campaign
    let campaign = Campaign::load("data/test_campaign").expect("Failed to load tutorial campaign");
    let content = campaign
        .load_content()
        .expect("Failed to load campaign content");

    // Expected NPC-to-Creature mappings based on Phase 3 implementation
    let expected_mappings: Vec<(&str, CreatureId)> = vec![
        ("tutorial_elder_village", 51),           // VillageElder
        ("tutorial_innkeeper_town", 52),          // Innkeeper
        ("tutorial_merchant_town", 53),           // Merchant
        ("tutorial_priestess_town", 55),          // HighPriestess
        ("tutorial_wizard_arcturus", 56),         // WizardArcturus
        ("tutorial_wizard_arcturus_brother", 58), // OldGareth
        ("tutorial_ranger_lost", 57),             // Ranger
        ("tutorial_elder_village2", 51),          // VillageElder (reused)
        ("tutorial_innkeeper_town2", 52),         // Innkeeper (reused)
        ("tutorial_merchant_town2", 53),          // Merchant (reused)
        ("tutorial_priest_town2", 54),            // HighPriest
        ("tutorial_goblin_dying", 151),           // DyingGoblin
    ];

    // Verify each NPC has the correct creature_id
    for (npc_id, expected_creature_id) in &expected_mappings {
        let npc = content
            .npcs
            .get_npc(npc_id)
            .unwrap_or_else(|| panic!("NPC '{}' not found in campaign", npc_id));

        assert!(
            npc.creature_id.is_some(),
            "NPC '{}' should have a creature_id",
            npc_id
        );

        assert_eq!(
            npc.creature_id.unwrap(),
            *expected_creature_id,
            "NPC '{}' has incorrect creature_id. Expected {}, got {}",
            npc_id,
            expected_creature_id,
            npc.creature_id.unwrap()
        );
    }

    println!(
        "✓ All {} tutorial NPCs have correct creature_id mappings",
        expected_mappings.len()
    );
}

#[test]
fn test_all_tutorial_npcs_have_creature_visuals() {
    // Load the tutorial campaign
    let campaign = Campaign::load("data/test_campaign").expect("Failed to load tutorial campaign");
    let content = campaign
        .load_content()
        .expect("Failed to load campaign content");

    let all_npc_ids = content.npcs.all_npcs();

    // Every tutorial NPC should have a creature_id for 3D rendering
    let npcs_without_visuals: Vec<_> = all_npc_ids
        .iter()
        .filter(|npc_id| {
            if let Some(npc) = content.npcs.get_npc(npc_id) {
                npc.creature_id.is_none()
            } else {
                false
            }
        })
        .cloned()
        .collect();

    assert!(
        npcs_without_visuals.is_empty(),
        "The following NPCs are missing creature_id: {:?}",
        npcs_without_visuals
    );

    println!(
        "✓ All {} tutorial NPCs have creature visuals",
        all_npc_ids.len()
    );
}

#[test]
fn test_no_broken_npc_creature_references() {
    // Load the tutorial campaign
    let campaign = Campaign::load("data/test_campaign").expect("Failed to load tutorial campaign");
    let content = campaign
        .load_content()
        .expect("Failed to load campaign content");

    let all_npc_ids = content.npcs.all_npcs();

    // Collect all creature IDs from the creature database
    let all_creature_ids: HashSet<CreatureId> =
        content.creatures.all_creatures().map(|c| c.id).collect();

    // Verify all NPC creature_id references are valid
    let mut broken_references = Vec::new();

    for npc_id in &all_npc_ids {
        if let Some(npc) = content.npcs.get_npc(npc_id) {
            if let Some(creature_id) = npc.creature_id {
                if !all_creature_ids.contains(&creature_id) {
                    broken_references.push((npc.id.clone(), creature_id));
                }
            }
        }
    }

    assert!(
        broken_references.is_empty(),
        "The following NPCs reference non-existent creatures: {:?}",
        broken_references
    );

    println!("✓ All NPC creature references are valid");
}

#[test]
fn test_creature_database_has_expected_npc_creatures() {
    // Load the tutorial campaign
    let campaign = Campaign::load("data/test_campaign").expect("Failed to load tutorial campaign");
    let content = campaign
        .load_content()
        .expect("Failed to load campaign content");

    // NPC-specific creatures that should exist
    let expected_npc_creatures: Vec<(CreatureId, &str)> = vec![
        (51, "VillageElder"),
        (52, "Innkeeper"),
        (53, "Merchant"),
        (54, "HighPriest"),
        (55, "HighPriestess"),
        (56, "WizardArcturus"),
        (57, "Ranger"),
        (58, "OldGareth"),
        (151, "DyingGoblin"), // Special NPC (goblin variant)
    ];

    for (creature_id, expected_name) in expected_npc_creatures {
        let creature = content
            .creatures
            .get_creature(creature_id)
            .unwrap_or_else(|| {
                panic!(
                    "Creature {} ('{}') not found in creatures database",
                    creature_id, expected_name
                )
            });

        assert_eq!(
            creature.name, expected_name,
            "Creature {} has wrong name. Expected '{}', got '{}'",
            creature_id, expected_name, creature.name
        );
    }

    println!("✓ All expected NPC creatures exist in database");
}

#[test]
fn test_npc_definition_parses_with_creature_id() {
    // Test parsing a single NPC definition with creature_id
    let ron_str = r#"
(
    id: "test_elder",
    name: "Village Elder",
    description: "Wise elder",
    portrait_id: "elder.png",
    dialogue_id: Some(1),
    creature_id: Some(51),
    sprite: None,
    quest_ids: [1, 2],
    faction: Some("Village"),
    is_merchant: false,
    is_innkeeper: false,
)
"#;

    let npc: NpcDefinition = ron::from_str(ron_str).expect("Failed to parse NPC with creature_id");

    assert_eq!(npc.id, "test_elder");
    assert_eq!(npc.creature_id, Some(51));
    assert_eq!(npc.name, "Village Elder");

    println!("✓ NPC definition parses correctly with creature_id");
}

#[test]
fn test_npc_definition_backward_compatible_without_creature_id() {
    // Test that old NPC definitions without creature_id still parse
    let ron_str = r#"
(
    id: "old_npc",
    name: "Old NPC",
    description: "Legacy NPC",
    portrait_id: "old.png",
    dialogue_id: None,
    sprite: None,
    quest_ids: [],
    faction: None,
    is_merchant: false,
    is_innkeeper: false,
)
"#;

    let npc: NpcDefinition = ron::from_str(ron_str).expect("Failed to parse old NPC format");

    assert_eq!(npc.id, "old_npc");
    assert_eq!(npc.creature_id, None); // Should default to None
    assert_eq!(npc.name, "Old NPC");

    println!("✓ Backward compatibility maintained for NPCs without creature_id");
}

#[test]
fn test_npc_creature_id_counts() {
    // Load the tutorial campaign
    let campaign = Campaign::load("data/test_campaign").expect("Failed to load tutorial campaign");
    let content = campaign
        .load_content()
        .expect("Failed to load campaign content");

    let all_npc_ids = content.npcs.all_npcs();

    let npcs_with_creatures = all_npc_ids
        .iter()
        .filter(|npc_id| {
            if let Some(npc) = content.npcs.get_npc(npc_id) {
                npc.creature_id.is_some()
            } else {
                false
            }
        })
        .count();

    let total_npcs = all_npc_ids.len();

    println!(
        "✓ NPC creature mapping: {}/{} NPCs have creature visuals ({}%)",
        npcs_with_creatures,
        total_npcs,
        (npcs_with_creatures * 100) / total_npcs.max(1)
    );

    // In the tutorial campaign, all NPCs should have creature visuals
    assert_eq!(
        npcs_with_creatures, total_npcs,
        "All tutorial NPCs should have creature_id mappings"
    );
}

#[test]
fn test_npc_creature_reuse() {
    // Load the tutorial campaign
    let campaign = Campaign::load("data/test_campaign").expect("Failed to load tutorial campaign");
    let content = campaign
        .load_content()
        .expect("Failed to load campaign content");

    let all_npc_ids = content.npcs.all_npcs();

    // Count how many times each creature is used
    let mut creature_usage: std::collections::HashMap<CreatureId, Vec<String>> =
        std::collections::HashMap::new();

    for npc_id in &all_npc_ids {
        if let Some(npc) = content.npcs.get_npc(npc_id) {
            if let Some(creature_id) = npc.creature_id {
                creature_usage
                    .entry(creature_id)
                    .or_default()
                    .push(npc.id.clone());
            }
        }
    }

    // Verify that some creatures are reused (e.g., VillageElder, Innkeeper, Merchant)
    let reused_creatures: Vec<_> = creature_usage
        .iter()
        .filter(|(_, npcs)| npcs.len() > 1)
        .collect();

    assert!(
        !reused_creatures.is_empty(),
        "Expected some creatures to be reused by multiple NPCs"
    );

    println!("✓ Creature reuse detected:");
    for (creature_id, npcs) in reused_creatures {
        println!("  Creature {}: used by {} NPCs", creature_id, npcs.len());
    }
}

#[test]
fn test_npc_hybrid_sprite_and_creature_support() {
    // Test that an NPC can have both sprite and creature_id (for flexibility)
    let ron_str = r#"
(
    id: "hybrid_npc",
    name: "Hybrid NPC",
    description: "Has both sprite and creature",
    portrait_id: "hybrid.png",
    dialogue_id: None,
    creature_id: Some(51),
    sprite: Some((
        sheet_path: "npcs.png",
        sprite_index: 5,
        animation: None,
        material_properties: None,
    )),
    quest_ids: [],
    faction: None,
    is_merchant: false,
    is_innkeeper: false,
)
"#;

    let npc: NpcDefinition = ron::from_str(ron_str).expect("Failed to parse hybrid NPC");

    assert_eq!(npc.creature_id, Some(51));
    assert!(npc.sprite.is_some());
    assert_eq!(npc.sprite.as_ref().unwrap().sprite_index, 5);

    println!("✓ NPCs can have both creature_id and sprite (hybrid support)");
}
