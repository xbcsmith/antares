// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Map Authoring Integration Tests: Visual Metadata
//!
//! Tests RON serialization/deserialization, backward compatibility,
//! and example map loading.

use antares::domain::world::{Map, TerrainType, Tile, TileVisualMetadata, WallType};

#[test]
fn test_ron_round_trip_with_visual() {
    // Create a tile with visual metadata
    let mut tile = Tile::new(5, 10, TerrainType::Ground, WallType::Normal);
    tile.visual = TileVisualMetadata {
        height: Some(1.5),
        width_x: None,
        width_z: Some(0.2),
        color_tint: Some((0.8, 0.6, 0.4)),
        scale: None,
        y_offset: None,
        rotation_y: None,
        sprite: None,
        sprite_layers: vec![],
        sprite_rule: None,
        grass_density: None,
        tree_type: None,
        rock_variant: None,
        water_flow_direction: None,
        foliage_density: None,
        snow_coverage: None,
    };

    // Serialize to RON
    let ron_str = ron::to_string(&tile).expect("Failed to serialize tile");

    // Deserialize from RON
    let deserialized: Tile = ron::from_str(&ron_str).expect("Failed to deserialize tile");

    // Verify all fields match
    assert_eq!(deserialized.x, 5);
    assert_eq!(deserialized.y, 10);
    assert_eq!(deserialized.terrain, TerrainType::Ground);
    assert_eq!(deserialized.wall_type, WallType::Normal);
    assert_eq!(deserialized.visual.height, Some(1.5));
    assert_eq!(deserialized.visual.width_x, None);
    assert_eq!(deserialized.visual.width_z, Some(0.2));
    assert_eq!(deserialized.visual.color_tint, Some((0.8, 0.6, 0.4)));
    assert_eq!(deserialized.visual.scale, None);
    assert_eq!(deserialized.visual.y_offset, None);
}

#[test]
fn test_ron_backward_compat_without_visual() {
    // Old-style RON without visual field
    let old_ron = r#"(
        terrain: Ground,
        wall_type: Normal,
        blocked: false,
        is_special: false,
        is_dark: false,
        visited: false,
        x: 3,
        y: 7,
        event_trigger: None,
    )"#;

    // Should deserialize successfully with default visual metadata
    let tile: Tile = ron::from_str(old_ron).expect("Failed to deserialize old format");

    assert_eq!(tile.x, 3);
    assert_eq!(tile.y, 7);
    assert_eq!(tile.terrain, TerrainType::Ground);
    assert_eq!(tile.wall_type, WallType::Normal);

    // Visual metadata should be default (all None)
    assert_eq!(tile.visual.height, None);
    assert_eq!(tile.visual.width_x, None);
    assert_eq!(tile.visual.width_z, None);
    assert_eq!(tile.visual.color_tint, None);
    assert_eq!(tile.visual.scale, None);
    assert_eq!(tile.visual.y_offset, None);
}

#[test]
fn test_ron_partial_visual_metadata() {
    // RON with some visual fields set, others None
    let partial_ron = r#"(
        terrain: Mountain,
        wall_type: None,
        blocked: true,
        is_special: false,
        is_dark: false,
        visited: false,
        x: 12,
        y: 8,
        event_trigger: None,
        visual: (
            height: Some(4.0),
            width_x: None,
            width_z: None,
            color_tint: Some((0.5, 0.45, 0.4)),
            scale: None,
            y_offset: None,
        ),
    )"#;

    let tile: Tile = ron::from_str(partial_ron).expect("Failed to deserialize partial visual");

    assert_eq!(tile.terrain, TerrainType::Mountain);
    assert_eq!(tile.visual.height, Some(4.0));
    assert_eq!(tile.visual.width_x, None);
    assert_eq!(tile.visual.width_z, None);
    assert_eq!(tile.visual.color_tint, Some((0.5, 0.45, 0.4)));
    assert_eq!(tile.visual.scale, None);
    assert_eq!(tile.visual.y_offset, None);
}

#[test]
fn test_example_map_loads() {
    // Load the visual metadata examples map
    let map_path = "data/maps/visual_metadata_examples.ron";
    let ron_data =
        std::fs::read_to_string(map_path).expect("Failed to read visual_metadata_examples.ron");

    let map: Map = ron::from_str(&ron_data).expect("Failed to deserialize example map");

    // Verify basic map properties
    assert_eq!(map.id, 99);
    assert_eq!(map.width, 25);
    assert_eq!(map.height, 10);
    assert_eq!(map.name, "Visual Metadata Examples");

    // Find and verify castle wall (Section 1)
    let castle_wall = map
        .tiles
        .iter()
        .find(|t| t.x == 0 && t.y == 2)
        .expect("Castle wall tile not found");
    assert_eq!(castle_wall.visual.height, Some(3.0));
    assert_eq!(castle_wall.visual.color_tint, Some((0.6, 0.6, 0.7)));

    // Find and verify garden wall (Section 1)
    let garden_wall = map
        .tiles
        .iter()
        .find(|t| t.x == 0 && t.y == 5)
        .expect("Garden wall tile not found");
    assert_eq!(garden_wall.visual.height, Some(1.0));
    assert_eq!(garden_wall.visual.width_z, Some(0.3));
    assert_eq!(garden_wall.visual.color_tint, Some((0.7, 0.5, 0.3)));

    // Find and verify mountain variations (Section 2)
    let small_hill = map
        .tiles
        .iter()
        .find(|t| t.x == 5 && t.y == 4)
        .expect("Small hill not found");
    assert_eq!(small_hill.terrain, TerrainType::Mountain);
    assert_eq!(small_hill.visual.height, Some(2.0));

    let tall_mountain = map
        .tiles
        .iter()
        .find(|t| t.x == 7 && t.y == 4)
        .expect("Tall mountain not found");
    assert_eq!(tall_mountain.visual.height, Some(4.0));

    // Find and verify color-tinted walls (Section 3)
    let sandstone = map
        .tiles
        .iter()
        .find(|t| t.x == 10 && t.y == 3)
        .expect("Sandstone wall not found");
    assert_eq!(sandstone.visual.color_tint, Some((0.9, 0.7, 0.4)));

    let marble = map
        .tiles
        .iter()
        .find(|t| t.x == 12 && t.y == 3)
        .expect("Marble wall not found");
    assert_eq!(marble.visual.color_tint, Some((0.95, 0.95, 0.98)));

    // Find and verify scaled trees (Section 4)
    let small_tree = map
        .tiles
        .iter()
        .find(|t| t.x == 15 && t.y == 4)
        .expect("Small tree not found");
    assert_eq!(small_tree.terrain, TerrainType::Forest);
    assert_eq!(small_tree.visual.scale, Some(0.5));

    let large_tree = map
        .tiles
        .iter()
        .find(|t| t.x == 17 && t.y == 4)
        .expect("Large tree not found");
    assert_eq!(large_tree.visual.scale, Some(2.0));

    // Find and verify y_offset variations (Section 5)
    let sunken_pit = map
        .tiles
        .iter()
        .find(|t| t.x == 20 && t.y == 4)
        .expect("Sunken pit not found");
    assert_eq!(sunken_pit.visual.y_offset, Some(-0.5));

    let raised_platform = map
        .tiles
        .iter()
        .find(|t| t.x == 22 && t.y == 4)
        .expect("Raised platform not found");
    assert_eq!(raised_platform.visual.y_offset, Some(0.5));
}

#[test]
fn test_map_round_trip_preserves_visual() {
    // Create a map with custom visual metadata
    let mut map = Map::new(
        1,
        "Test Map".to_string(),
        "Test description".to_string(),
        5,
        5,
    );

    // Modify a tile with visual metadata (position 2,2 which is index y*width+x = 2*5+2 = 12)
    let tile_index = 2 * 5 + 2; // y * width + x
    map.tiles[tile_index].visual = TileVisualMetadata {
        sprite_layers: vec![],
        sprite_rule: None,
        height: Some(2.8),
        width_x: Some(0.9),
        width_z: Some(0.8),
        color_tint: Some((0.7, 0.6, 0.5)),
        scale: Some(1.2),
        y_offset: Some(0.3),
        rotation_y: None,
        sprite: None,
        grass_density: None,
        tree_type: None,
        rock_variant: None,
        water_flow_direction: None,
        foliage_density: None,
        snow_coverage: None,
    };

    // Serialize to RON
    let ron_str =
        ron::ser::to_string_pretty(&map, Default::default()).expect("Failed to serialize map");

    // Deserialize back
    let deserialized: Map = ron::from_str(&ron_str).expect("Failed to deserialize map");

    // Find the tile with visual metadata
    let restored_tile = deserialized
        .tiles
        .iter()
        .find(|t| t.x == 2 && t.y == 2)
        .expect("Tile not found after round trip");

    // Verify all visual metadata preserved
    assert_eq!(restored_tile.visual.height, Some(2.8));
    assert_eq!(restored_tile.visual.width_x, Some(0.9));
    assert_eq!(restored_tile.visual.width_z, Some(0.8));
    assert_eq!(restored_tile.visual.color_tint, Some((0.7, 0.6, 0.5)));
    assert_eq!(restored_tile.visual.scale, Some(1.2));
    assert_eq!(restored_tile.visual.y_offset, Some(0.3));
}

#[test]
fn test_visual_metadata_default_values() {
    let metadata = TileVisualMetadata::default();

    assert_eq!(metadata.height, None);
    assert_eq!(metadata.width_x, None);
    assert_eq!(metadata.width_z, None);
    assert_eq!(metadata.color_tint, None);
    assert_eq!(metadata.scale, None);
    assert_eq!(metadata.y_offset, None);
}

#[test]
fn test_tile_builder_with_visual_metadata() {
    let tile = Tile::new(10, 15, TerrainType::Mountain, WallType::None)
        .with_height(5.0)
        .with_color_tint(0.4, 0.35, 0.3)
        .with_scale(1.5);

    assert_eq!(tile.x, 10);
    assert_eq!(tile.y, 15);
    assert_eq!(tile.terrain, TerrainType::Mountain);
    assert_eq!(tile.visual.height, Some(5.0));
    assert_eq!(tile.visual.color_tint, Some((0.4, 0.35, 0.3)));
    assert_eq!(tile.visual.scale, Some(1.5));
}

#[test]
fn test_visual_metadata_effective_values() {
    let mut metadata = TileVisualMetadata::default();

    // Test effective height with defaults
    assert_eq!(
        metadata.effective_height(TerrainType::Ground, WallType::Normal),
        2.5
    );
    assert_eq!(
        metadata.effective_height(TerrainType::Mountain, WallType::None),
        3.0
    );
    assert_eq!(
        metadata.effective_height(TerrainType::Forest, WallType::None),
        2.2
    );

    // Test with custom height
    metadata.height = Some(4.5);
    assert_eq!(
        metadata.effective_height(TerrainType::Ground, WallType::Normal),
        4.5
    );
    assert_eq!(
        metadata.effective_height(TerrainType::Mountain, WallType::None),
        4.5
    );

    // Test effective dimensions
    assert_eq!(metadata.effective_width_x(), 1.0);
    assert_eq!(metadata.effective_width_z(), 1.0);
    assert_eq!(metadata.effective_scale(), 1.0);
    assert_eq!(metadata.effective_y_offset(), 0.0);

    // Set custom values
    metadata.width_x = Some(0.5);
    metadata.width_z = Some(0.3);
    metadata.scale = Some(2.0);
    metadata.y_offset = Some(0.8);

    assert_eq!(metadata.effective_width_x(), 0.5);
    assert_eq!(metadata.effective_width_z(), 0.3);
    assert_eq!(metadata.effective_scale(), 2.0);
    assert_eq!(metadata.effective_y_offset(), 0.8);
}

#[test]
fn test_mesh_dimensions_calculation() {
    let mut metadata = TileVisualMetadata::default();

    // Test default dimensions for normal wall
    let (width_x, height, width_z) =
        metadata.mesh_dimensions(TerrainType::Ground, WallType::Normal);
    assert_eq!(width_x, 1.0);
    assert_eq!(height, 2.5);
    assert_eq!(width_z, 1.0);

    // Test with custom dimensions and scale
    metadata.height = Some(3.0);
    metadata.width_x = Some(0.8);
    metadata.width_z = Some(0.6);
    metadata.scale = Some(1.5);

    let (width_x, height, width_z) =
        metadata.mesh_dimensions(TerrainType::Ground, WallType::Normal);
    assert_eq!(width_x, 0.8 * 1.5);
    assert_eq!(height, 3.0 * 1.5);
    assert_eq!(width_z, 0.6 * 1.5);
}

#[test]
fn test_mesh_y_position_calculation() {
    let mut metadata = TileVisualMetadata::default();

    // Test default Y position (half height, no offset)
    let y_pos = metadata.mesh_y_position(TerrainType::Ground, WallType::Normal);
    assert_eq!(y_pos, 2.5 / 2.0); // height / 2

    // Test with custom height and offset
    metadata.height = Some(4.0);
    metadata.y_offset = Some(0.5);

    let y_pos = metadata.mesh_y_position(TerrainType::Ground, WallType::Normal);
    assert_eq!(y_pos, 4.0 / 2.0 + 0.5); // (height / 2) + offset
}

#[test]
fn test_color_tint_range_validation() {
    // Valid tints (0.0 to 1.0 range)
    let valid_tints = vec![
        (0.0, 0.0, 0.0),
        (1.0, 1.0, 1.0),
        (0.5, 0.5, 0.5),
        (0.9, 0.7, 0.4),
    ];

    for tint in valid_tints {
        let metadata = TileVisualMetadata {
            color_tint: Some(tint),
            ..Default::default()
        };

        // Serialize and deserialize
        let ron = ron::to_string(&metadata).expect("Serialization failed");
        let _deserialized: TileVisualMetadata =
            ron::from_str(&ron).expect("Deserialization failed");
    }
}
