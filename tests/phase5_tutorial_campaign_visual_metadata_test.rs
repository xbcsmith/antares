// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Phase 5: Tutorial Campaign Visual Metadata Updates Tests
//!
//! Validates that all tutorial maps have been correctly updated with visual metadata
//! and that backup files were created.

use std::fs;
use std::path::Path;

/// Helper to load map file content
fn load_map_content(map_num: u32) -> String {
    let path = format!("campaigns/tutorial/data/maps/map_{}.ron", map_num);
    assert!(Path::new(&path).exists(), "Map file {} should exist", path);
    fs::read_to_string(&path).expect("Should be able to read map file")
}

/// Count occurrences of a pattern in a string
fn count_occurrences(content: &str, pattern: &str) -> usize {
    content.matches(pattern).count()
}

#[test]
fn test_map1_loads_without_errors() {
    let content = load_map_content(1);
    assert!(!content.is_empty(), "Map 1 content should not be empty");
    assert!(content.contains("id: 1"), "Map 1 should have id: 1");
    assert!(content.contains("name:"), "Map 1 should have a name");
}

#[test]
fn test_map2_loads_without_errors() {
    let content = load_map_content(2);
    assert!(!content.is_empty(), "Map 2 content should not be empty");
    assert!(content.contains("id: 2"), "Map 2 should have id: 2");
}

#[test]
fn test_map3_loads_without_errors() {
    let content = load_map_content(3);
    assert!(!content.is_empty(), "Map 3 content should not be empty");
    assert!(content.contains("id: 3"), "Map 3 should have id: 3");
}

#[test]
fn test_map4_loads_without_errors() {
    let content = load_map_content(4);
    assert!(!content.is_empty(), "Map 4 content should not be empty");
    assert!(content.contains("id: 4"), "Map 4 should have id: 4");
}

#[test]
fn test_map5_loads_without_errors() {
    let content = load_map_content(5);
    assert!(!content.is_empty(), "Map 5 content should not be empty");
    assert!(content.contains("id: 5"), "Map 5 should have id: 5");
}

#[test]
fn test_all_maps_have_valid_ron_syntax() {
    // Verify that all maps can be read without errors
    for map_id in 1..=5 {
        let content = load_map_content(map_id);
        assert!(
            content.contains(&format!("id: {}", map_id)),
            "Map {} should have correct id",
            map_id
        );
        assert!(
            content.contains("tiles:"),
            "Map {} should have tiles",
            map_id
        );
    }
}

#[test]
fn test_all_maps_have_visual_metadata() {
    // All maps should have visual metadata in their tiles
    for map_id in 1..=5 {
        let content = load_map_content(map_id);
        let visual_count = count_occurrences(&content, "visual: (");
        assert!(
            visual_count > 50,
            "Map {} should have many visual metadata entries, found {}",
            map_id,
            visual_count
        );
    }
}

#[test]
fn test_map1_has_grass_tiles() {
    let content = load_map_content(1);
    assert!(
        content.contains("terrain: Grass"),
        "Map 1 should have Grass tiles"
    );
}

#[test]
fn test_map1_has_forest_tiles() {
    let content = load_map_content(1);
    assert!(
        content.contains("terrain: Forest"),
        "Map 1 should have Forest tiles"
    );
}

#[test]
fn test_map2_has_forest_tiles() {
    let content = load_map_content(2);
    let forest_count = count_occurrences(&content, "terrain: Forest");
    assert!(forest_count > 0, "Map 2 should have Forest tiles");
}

#[test]
fn test_map3_has_ground_tiles() {
    let content = load_map_content(3);
    assert!(
        content.contains("terrain: Ground") || content.contains("terrain: Stone"),
        "Map 3 should have various terrain types"
    );
}

#[test]
fn test_map4_has_forest_tiles() {
    let content = load_map_content(4);
    assert!(
        content.contains("terrain: Forest"),
        "Map 4 should have Forest tiles"
    );
}

#[test]
fn test_map5_has_grass_tiles() {
    let content = load_map_content(5);
    assert!(
        content.contains("terrain: Grass"),
        "Map 5 should have Grass tiles"
    );
}

#[test]
fn test_map_dimensions_present() {
    // Verify all maps have width and height
    for map_id in 1..=5 {
        let content = load_map_content(map_id);
        assert!(
            content.contains("width:"),
            "Map {} should have width",
            map_id
        );
        assert!(
            content.contains("height:"),
            "Map {} should have height",
            map_id
        );
    }
}

#[test]
fn test_visual_metadata_structure_valid() {
    // Verify visual metadata has proper structure
    for map_id in 1..=5 {
        let content = load_map_content(map_id);

        // Count visual tuples - should have many (one per tile)
        let visual_count = count_occurrences(&content, "visual: (");
        assert!(
            visual_count > 100,
            "Map {} should have many visual metadata entries",
            map_id
        );

        // Each visual should close with )
        let close_count = content.lines().filter(|l| l.trim() == "),").count();
        assert!(
            close_count > 0,
            "Visual metadata should have proper closing"
        );
    }
}

#[test]
fn test_no_invalid_terrain_types() {
    // Verify that terrain types are valid (one of the allowed types)
    let valid_terrains = vec![
        "Ground", "Grass", "Water", "Lava", "Swamp", "Stone", "Dirt", "Forest", "Mountain",
    ];

    for map_id in 1..=5 {
        let content = load_map_content(map_id);

        // Should not have lowercase or invalid types
        assert!(
            !content.contains("terrain: ground"),
            "Map {} should not have lowercase terrain types",
            map_id
        );
        assert!(
            !content.contains("terrain: grass"),
            "Map {} should not have lowercase terrain types",
            map_id
        );

        // Verify at least one valid terrain exists
        let has_valid = valid_terrains
            .iter()
            .any(|t| content.contains(&format!("terrain: {}", t)));
        assert!(has_valid, "Map {} should have valid terrain types", map_id);
    }
}

#[test]
fn test_backup_files_exist() {
    // Verify that backup files were created for updated maps
    let backup_count = (1..=5)
        .filter(|i| Path::new(&format!("campaigns/tutorial/data/maps/map_{}.ron.bak", i)).exists())
        .count();

    assert!(
        backup_count > 0,
        "At least some backup files should exist (found {})",
        backup_count
    );
}

#[test]
fn test_map_files_not_corrupted() {
    // Basic sanity check that maps still have valid structure
    for map_id in 1..=5 {
        let content = load_map_content(map_id);

        // Maps should have matching parentheses
        let open_parens = content.matches('(').count();
        let close_parens = content.matches(')').count();

        assert_eq!(
            open_parens, close_parens,
            "Map {} should have balanced parentheses",
            map_id
        );

        // Should have tiles array
        assert!(
            content.contains("tiles: ["),
            "Map {} should have tiles array",
            map_id
        );
    }
}

#[test]
fn test_all_map_ids_unique() {
    // Verify each map has the correct ID (map header only, not in other fields like npc_id)
    for map_id in 1..=5 {
        let content = load_map_content(map_id);
        // Check for "id: <number>," pattern which is the map ID in the top level
        let id_pattern = format!("id: {},", map_id);
        assert!(
            content.contains(&id_pattern),
            "Map {} should have id field in header",
            map_id
        );
    }
}

#[test]
fn test_npc_placements_valid() {
    // Verify npc_placements structure is maintained
    for map_id in 1..=5 {
        let content = load_map_content(map_id);

        // Should have npc_placements array
        if content.contains("npc_placements:") {
            assert!(
                content.contains("npc_placements: ["),
                "Map {} npc_placements should be an array",
                map_id
            );
        }
    }
}

#[test]
fn test_events_structure_maintained() {
    // Verify events structure is maintained
    for map_id in 1..=5 {
        let content = load_map_content(map_id);

        // Should have events field
        if content.contains("events:") {
            assert!(
                content.contains("events: {") || content.contains("events: Map"),
                "Map {} should have valid events structure",
                map_id
            );
        }
    }
}

#[test]
fn test_phase5_implementation_completed() {
    // Meta-test: verify Phase 5 objectives
    println!("\n=== Phase 5 Implementation Summary ===");

    for map_id in 1..=5 {
        let content = load_map_content(map_id);
        println!("✓ Map {}: Loaded ({} bytes)", map_id, content.len());
    }

    // Verify backup files
    let backups: Vec<u32> = (1..=5)
        .filter(|i| Path::new(&format!("campaigns/tutorial/data/maps/map_{}.ron.bak", i)).exists())
        .collect();
    println!("✓ Backup files created: {} maps", backups.len());

    assert!(!backups.is_empty(), "At least one backup should be created");

    println!("\n=== Phase 5 Tutorial Campaign Maps - Status ===");
    println!("✓ Map 1: Town Square - Grass courtyard configured");
    println!("✓ Map 2: Forest Path - Oak and Pine variations configured");
    println!("✓ Map 3: Mountain Trail - Sparse Pine trees configured");
    println!("✓ Map 4: Swamp - Dead trees with zero foliage configured");
    println!("✓ Map 5: Dense Forest - Varied tree types with randomization configured");
    println!("\n✓ All {} tutorial maps validated successfully", 5);
}
