// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Phase 6: Campaign Builder Creatures Editor Integration Tests
//!
//! These tests verify the creatures editor integration with the tutorial campaign,
//! including file I/O, validation, and RON format handling.
//!
//! Tests cover:
//! - Loading tutorial campaign creatures.ron
//! - Validation of creature references
//! - RON file parsing and serialization
//! - Creature ID range validation
//! - Duplicate detection
//! - Category-based queries

use antares::domain::types::CreatureId;
use antares::domain::visual::CreatureReference;
use std::path::PathBuf;

// Import from campaign_builder SDK
// Note: These would normally be available via the SDK crate
// For now we'll test the integration indirectly through the domain layer

#[test]
fn test_tutorial_creatures_file_exists() {
    let creatures_file = PathBuf::from("campaigns/tutorial/data/creatures.ron");
    assert!(
        creatures_file.exists(),
        "Tutorial campaign creatures.ron should exist"
    );
}

#[test]
fn test_tutorial_creatures_ron_parses() {
    // Arrange: Read the tutorial creatures.ron file
    let creatures_file = PathBuf::from("campaigns/tutorial/data/creatures.ron");
    let contents =
        std::fs::read_to_string(&creatures_file).expect("Should read tutorial creatures.ron");

    // Act: Parse as RON
    let creatures_result: Result<Vec<CreatureReference>, _> = ron::from_str(&contents);

    // Assert: Parsing succeeds
    assert!(
        creatures_result.is_ok(),
        "Tutorial creatures.ron should parse as Vec<CreatureReference>: {:?}",
        creatures_result.err()
    );

    let creatures = creatures_result.unwrap();
    println!(
        "✓ Parsed {} creatures from tutorial campaign",
        creatures.len()
    );
}

#[test]
fn test_tutorial_creatures_count() {
    // Arrange: Load creatures from tutorial campaign
    let creatures_file = PathBuf::from("campaigns/tutorial/data/creatures.ron");
    let contents =
        std::fs::read_to_string(&creatures_file).expect("Should read tutorial creatures.ron");
    let creatures: Vec<CreatureReference> =
        ron::from_str(&contents).expect("Should parse creatures.ron");

    // Assert: Expected count (32 creatures)
    assert_eq!(
        creatures.len(),
        32,
        "Tutorial campaign should have 32 creatures"
    );

    println!("✓ Tutorial campaign has {} creatures", creatures.len());
}

#[test]
fn test_tutorial_creatures_have_valid_ids() {
    // Arrange: Load creatures
    let creatures_file = PathBuf::from("campaigns/tutorial/data/creatures.ron");
    let contents =
        std::fs::read_to_string(&creatures_file).expect("Should read tutorial creatures.ron");
    let creatures: Vec<CreatureReference> =
        ron::from_str(&contents).expect("Should parse creatures.ron");

    // Assert: All IDs are non-zero
    for creature in &creatures {
        assert!(
            creature.id > 0,
            "Creature '{}' has invalid ID: {}",
            creature.name,
            creature.id
        );
    }

    println!("✓ All {} creatures have valid IDs", creatures.len());
}

#[test]
fn test_tutorial_creatures_no_duplicate_ids() {
    // Arrange: Load creatures
    let creatures_file = PathBuf::from("campaigns/tutorial/data/creatures.ron");
    let contents =
        std::fs::read_to_string(&creatures_file).expect("Should read tutorial creatures.ron");
    let creatures: Vec<CreatureReference> =
        ron::from_str(&contents).expect("Should parse creatures.ron");

    // Act: Check for duplicates
    use std::collections::HashSet;
    let mut seen_ids = HashSet::new();
    let mut duplicates = Vec::new();

    for creature in &creatures {
        if !seen_ids.insert(creature.id) {
            duplicates.push(creature.id);
        }
    }

    // Assert: No duplicates
    assert!(
        duplicates.is_empty(),
        "Found duplicate creature IDs: {:?}",
        duplicates
    );

    println!("✓ No duplicate IDs found in {} creatures", creatures.len());
}

#[test]
fn test_tutorial_creatures_have_names() {
    // Arrange: Load creatures
    let creatures_file = PathBuf::from("campaigns/tutorial/data/creatures.ron");
    let contents =
        std::fs::read_to_string(&creatures_file).expect("Should read tutorial creatures.ron");
    let creatures: Vec<CreatureReference> =
        ron::from_str(&contents).expect("Should parse creatures.ron");

    // Assert: All creatures have non-empty names
    for creature in &creatures {
        assert!(
            !creature.name.is_empty(),
            "Creature {} has empty name",
            creature.id
        );
    }

    println!("✓ All {} creatures have names", creatures.len());
}

#[test]
fn test_tutorial_creatures_have_filepaths() {
    // Arrange: Load creatures
    let creatures_file = PathBuf::from("campaigns/tutorial/data/creatures.ron");
    let contents =
        std::fs::read_to_string(&creatures_file).expect("Should read tutorial creatures.ron");
    let creatures: Vec<CreatureReference> =
        ron::from_str(&contents).expect("Should parse creatures.ron");

    // Assert: All creatures have non-empty filepaths
    for creature in &creatures {
        assert!(
            !creature.filepath.is_empty(),
            "Creature '{}' has empty filepath",
            creature.name
        );
    }

    println!("✓ All {} creatures have filepaths", creatures.len());
}

#[test]
fn test_tutorial_creature_files_exist() {
    // Arrange: Load creatures
    let creatures_file = PathBuf::from("campaigns/tutorial/data/creatures.ron");
    let contents =
        std::fs::read_to_string(&creatures_file).expect("Should read tutorial creatures.ron");
    let creatures: Vec<CreatureReference> =
        ron::from_str(&contents).expect("Should parse creatures.ron");

    // Assert: All referenced files exist
    let campaign_root = PathBuf::from("campaigns/tutorial");
    let mut missing_files = Vec::new();

    for creature in &creatures {
        let full_path = campaign_root.join(&creature.filepath);
        if !full_path.exists() {
            missing_files.push((
                creature.id,
                creature.name.clone(),
                creature.filepath.clone(),
            ));
        }
    }

    assert!(
        missing_files.is_empty(),
        "Missing creature files: {:?}",
        missing_files
    );

    println!("✓ All {} creature files exist", creatures.len());
}

#[test]
fn test_tutorial_creatures_id_ranges() {
    // Arrange: Load creatures
    let creatures_file = PathBuf::from("campaigns/tutorial/data/creatures.ron");
    let contents =
        std::fs::read_to_string(&creatures_file).expect("Should read tutorial creatures.ron");
    let creatures: Vec<CreatureReference> =
        ron::from_str(&contents).expect("Should parse creatures.ron");

    // Act: Categorize creatures by ID range
    let mut monsters = Vec::new();
    let mut npcs = Vec::new();
    let mut templates = Vec::new();
    let mut variants = Vec::new();
    let mut custom = Vec::new();

    for creature in &creatures {
        match creature.id {
            1..=50 => monsters.push(creature),
            51..=100 => npcs.push(creature),
            101..=150 => templates.push(creature),
            151..=200 => variants.push(creature),
            _ => custom.push(creature),
        }
    }

    println!("Creature distribution:");
    println!("  Monsters (1-50): {}", monsters.len());
    println!("  NPCs (51-100): {}", npcs.len());
    println!("  Templates (101-150): {}", templates.len());
    println!("  Variants (151-200): {}", variants.len());
    println!("  Custom (201+): {}", custom.len());

    // Assert: Expected distribution for tutorial campaign
    assert_eq!(monsters.len(), 13, "Should have 13 monster creatures");
    assert_eq!(npcs.len(), 13, "Should have 13 NPC creatures");
    assert_eq!(templates.len(), 3, "Should have 3 template creatures");
    assert_eq!(variants.len(), 3, "Should have 3 variant creatures");
    assert_eq!(custom.len(), 0, "Should have 0 custom creatures");

    println!("✓ Creature ID ranges match expected distribution");
}

#[test]
fn test_tutorial_creatures_ron_roundtrip() {
    // Arrange: Load creatures
    let creatures_file = PathBuf::from("campaigns/tutorial/data/creatures.ron");
    let contents =
        std::fs::read_to_string(&creatures_file).expect("Should read tutorial creatures.ron");
    let creatures: Vec<CreatureReference> =
        ron::from_str(&contents).expect("Should parse creatures.ron");

    // Act: Serialize back to RON
    let ron_config = ron::ser::PrettyConfig::new()
        .struct_names(false)
        .separate_tuple_members(true)
        .enumerate_arrays(false)
        .new_line("\n".to_string());

    let serialized =
        ron::ser::to_string_pretty(&creatures, ron_config).expect("Should serialize to RON");

    // Act: Parse the serialized version
    let roundtrip_result: Result<Vec<CreatureReference>, _> = ron::from_str(&serialized);

    // Assert: Roundtrip succeeds
    assert!(
        roundtrip_result.is_ok(),
        "RON roundtrip should succeed: {:?}",
        roundtrip_result.err()
    );

    let roundtrip_creatures = roundtrip_result.unwrap();
    assert_eq!(
        roundtrip_creatures.len(),
        creatures.len(),
        "Roundtrip should preserve creature count"
    );

    println!(
        "✓ RON roundtrip successful for {} creatures",
        creatures.len()
    );
}

#[test]
fn test_tutorial_creatures_specific_ids() {
    // Arrange: Load creatures
    let creatures_file = PathBuf::from("campaigns/tutorial/data/creatures.ron");
    let contents =
        std::fs::read_to_string(&creatures_file).expect("Should read tutorial creatures.ron");
    let creatures: Vec<CreatureReference> =
        ron::from_str(&contents).expect("Should parse creatures.ron");

    // Create lookup map
    use std::collections::HashMap;
    let creature_map: HashMap<CreatureId, &CreatureReference> =
        creatures.iter().map(|c| (c.id, c)).collect();

    // Assert: Specific creatures exist
    let expected_ids = vec![
        (1, "Goblin"),
        (2, "Kobold"),
        (3, "GiantRat"),
        (10, "Orc"),
        (11, "Skeleton"),
        (30, "Dragon"),
        (31, "Lich"),
        (51, "VillageElder"),
        (52, "Innkeeper"),
        (53, "Merchant"),
    ];

    for (id, expected_name) in expected_ids {
        assert!(
            creature_map.contains_key(&id),
            "Should have creature with ID {}",
            id
        );

        let creature = creature_map[&id];
        assert_eq!(
            creature.name, expected_name,
            "Creature {} should have name '{}'",
            id, expected_name
        );
    }

    println!("✓ All expected creatures found with correct names");
}

#[test]
fn test_tutorial_creatures_filepath_format() {
    // Arrange: Load creatures
    let creatures_file = PathBuf::from("campaigns/tutorial/data/creatures.ron");
    let contents =
        std::fs::read_to_string(&creatures_file).expect("Should read tutorial creatures.ron");
    let creatures: Vec<CreatureReference> =
        ron::from_str(&contents).expect("Should parse creatures.ron");

    // Assert: All filepaths follow expected format
    for creature in &creatures {
        assert!(
            creature.filepath.starts_with("assets/creatures/"),
            "Creature '{}' filepath should start with 'assets/creatures/', got: {}",
            creature.name,
            creature.filepath
        );

        assert!(
            creature.filepath.ends_with(".ron"),
            "Creature '{}' filepath should end with '.ron', got: {}",
            creature.name,
            creature.filepath
        );
    }

    println!(
        "✓ All {} creature filepaths follow correct format",
        creatures.len()
    );
}

#[test]
fn test_tutorial_creatures_sorted_by_id() {
    // Arrange: Load creatures
    let creatures_file = PathBuf::from("campaigns/tutorial/data/creatures.ron");
    let contents =
        std::fs::read_to_string(&creatures_file).expect("Should read tutorial creatures.ron");
    let creatures: Vec<CreatureReference> =
        ron::from_str(&contents).expect("Should parse creatures.ron");

    // Check if sorted by ID
    let mut is_sorted = true;
    for i in 1..creatures.len() {
        if creatures[i].id < creatures[i - 1].id {
            is_sorted = false;
            break;
        }
    }

    if is_sorted {
        println!("✓ Creatures are sorted by ID");
    } else {
        println!("ℹ Creatures are not sorted by ID (not required but recommended)");
    }
}

#[test]
fn test_creature_reference_serialization() {
    // Arrange: Create a test creature reference
    let creature = CreatureReference {
        id: 42,
        name: "TestCreature".to_string(),
        filepath: "assets/creatures/test_creature.ron".to_string(),
    };

    // Act: Serialize to RON
    let ron_config = ron::ser::PrettyConfig::new()
        .struct_names(false)
        .separate_tuple_members(true)
        .enumerate_arrays(false)
        .new_line("\n".to_string());

    let serialized = ron::ser::to_string_pretty(&creature, ron_config)
        .expect("Should serialize creature reference");

    // Assert: Contains expected fields
    assert!(serialized.contains("42"), "Should contain ID");
    assert!(serialized.contains("TestCreature"), "Should contain name");
    assert!(
        serialized.contains("assets/creatures/test_creature.ron"),
        "Should contain filepath"
    );

    // Act: Deserialize back
    let deserialized: CreatureReference =
        ron::from_str(&serialized).expect("Should deserialize creature reference");

    // Assert: Fields match
    assert_eq!(deserialized.id, creature.id);
    assert_eq!(deserialized.name, creature.name);
    assert_eq!(deserialized.filepath, creature.filepath);

    println!("✓ CreatureReference serialization roundtrip successful");
}

#[test]
fn test_tutorial_creatures_editor_compatibility() {
    // This test verifies that the creatures.ron format is compatible
    // with what the creatures editor expects

    // Arrange: Load creatures
    let creatures_file = PathBuf::from("campaigns/tutorial/data/creatures.ron");
    let contents =
        std::fs::read_to_string(&creatures_file).expect("Should read tutorial creatures.ron");
    let creatures: Vec<CreatureReference> =
        ron::from_str(&contents).expect("Should parse creatures.ron");

    // Assert: Structure is editor-compatible
    for creature in &creatures {
        // ID must be valid
        assert!(creature.id > 0, "Creature ID must be > 0");

        // Name must be non-empty
        assert!(!creature.name.is_empty(), "Creature name must be non-empty");

        // Filepath must be non-empty and valid format
        assert!(!creature.filepath.is_empty(), "Filepath must be non-empty");
        assert!(
            creature.filepath.contains('/') || creature.filepath.contains('\\'),
            "Filepath should be a path: {}",
            creature.filepath
        );
    }

    println!("✓ All {} creatures are editor-compatible", creatures.len());
}
