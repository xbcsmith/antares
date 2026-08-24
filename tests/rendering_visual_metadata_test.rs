// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for tile visual metadata rendering
//!
//! These tests verify that the rendering system correctly uses per-tile visual metadata
//! for mesh dimensions, Y-positioning, color tinting, and mesh caching.

use antares::domain::types::Position;
use antares::domain::world::terrain::{
    builtin_terrain_db, TERRAIN_FOREST, TERRAIN_GRASS, TERRAIN_GROUND, TERRAIN_MOUNTAIN,
};
use antares::domain::world::{Map, Tile, WallType};

/// Returns the default mesh height for a `TerrainId`.
///
/// Matches the built-in terrain heights: Mountain → 3.0, Forest → 2.2, all others → 0.0.
fn terrain_height(terrain: antares::domain::types::TerrainId) -> f32 {
    match terrain {
        TERRAIN_MOUNTAIN => 3.0,
        TERRAIN_FOREST => 2.2,
        _ => 0.0,
    }
}

#[test]
fn test_default_wall_height_unchanged() {
    // Verify that a wall with no custom visual metadata uses default height of 2.5
    let db = builtin_terrain_db();
    let tile = Tile::new(0, 0, TERRAIN_GROUND, WallType::Normal, &db);

    let (_, height, _) = tile
        .visual
        .mesh_dimensions(tile.wall_type, terrain_height(tile.terrain));
    assert_eq!(height, 2.5, "Default wall height should be 2.5 units");

    let y_pos = tile
        .visual
        .mesh_y_position(tile.wall_type, terrain_height(tile.terrain));
    assert_eq!(
        y_pos, 1.25,
        "Default wall Y-position should be 1.25 (height/2)"
    );
}

#[test]
fn test_custom_wall_height_applied() {
    // Verify that custom wall height overrides the default
    let db = builtin_terrain_db();
    let tile = Tile::new(0, 0, TERRAIN_GROUND, WallType::Normal, &db).with_height(1.5);

    let (_, height, _) = tile
        .visual
        .mesh_dimensions(tile.wall_type, terrain_height(tile.terrain));
    assert_eq!(height, 1.5, "Custom wall height should be 1.5 units");

    let y_pos = tile
        .visual
        .mesh_y_position(tile.wall_type, terrain_height(tile.terrain));
    assert_eq!(y_pos, 0.75, "Custom wall Y-position should be 0.75 (1.5/2)");
}

#[test]
fn test_custom_mountain_height_applied() {
    // Verify that custom mountain height overrides the default of 3.0
    let db = builtin_terrain_db();
    let tile = Tile::new(0, 0, TERRAIN_MOUNTAIN, WallType::None, &db).with_height(5.0);

    let (_, height, _) = tile
        .visual
        .mesh_dimensions(tile.wall_type, terrain_height(tile.terrain));
    assert_eq!(height, 5.0, "Custom mountain height should be 5.0 units");

    let y_pos = tile
        .visual
        .mesh_y_position(tile.wall_type, terrain_height(tile.terrain));
    assert_eq!(
        y_pos, 2.5,
        "Custom mountain Y-position should be 2.5 (5.0/2)"
    );
}

#[test]
fn test_default_mountain_height() {
    // Verify default mountain height is 3.0
    let db = builtin_terrain_db();
    let tile = Tile::new(0, 0, TERRAIN_MOUNTAIN, WallType::None, &db);

    let (_, height, _) = tile
        .visual
        .mesh_dimensions(tile.wall_type, terrain_height(tile.terrain));
    assert_eq!(height, 3.0, "Default mountain height should be 3.0 units");

    let y_pos = tile
        .visual
        .mesh_y_position(tile.wall_type, terrain_height(tile.terrain));
    assert_eq!(
        y_pos, 1.5,
        "Default mountain Y-position should be 1.5 (3.0/2)"
    );
}

#[test]
fn test_default_forest_height() {
    // Verify default forest/tree height is 2.2
    let db = builtin_terrain_db();
    let tile = Tile::new(0, 0, TERRAIN_FOREST, WallType::None, &db);

    let (_, height, _) = tile
        .visual
        .mesh_dimensions(tile.wall_type, terrain_height(tile.terrain));
    assert_eq!(height, 2.2, "Default forest height should be 2.2 units");

    let y_pos = tile
        .visual
        .mesh_y_position(tile.wall_type, terrain_height(tile.terrain));
    assert_eq!(
        y_pos, 1.1,
        "Default forest Y-position should be 1.1 (2.2/2)"
    );
}

#[test]
fn test_default_door_height() {
    // Verify default door height is 2.5 (same as wall)
    let db = builtin_terrain_db();
    let tile = Tile::new(0, 0, TERRAIN_GROUND, WallType::Door, &db);

    let (_, height, _) = tile
        .visual
        .mesh_dimensions(tile.wall_type, terrain_height(tile.terrain));
    assert_eq!(height, 2.5, "Default door height should be 2.5 units");

    let y_pos = tile
        .visual
        .mesh_y_position(tile.wall_type, terrain_height(tile.terrain));
    assert_eq!(
        y_pos, 1.25,
        "Default door Y-position should be 1.25 (2.5/2)"
    );
}

#[test]
fn test_color_tint_multiplies_base_color() {
    // Verify that color tint is properly stored and can be retrieved
    let db = builtin_terrain_db();
    let tile =
        Tile::new(0, 0, TERRAIN_GROUND, WallType::Normal, &db).with_color_tint(0.5, 1.0, 1.0);

    assert_eq!(
        tile.visual.color_tint,
        Some((0.5, 1.0, 1.0)),
        "Color tint should be stored correctly"
    );

    // Verify the tint values are in valid range
    if let Some((r, g, b)) = tile.visual.color_tint {
        assert!(
            (0.0..=1.0).contains(&r),
            "Red tint should be in 0.0-1.0 range"
        );
        assert!(
            (0.0..=1.0).contains(&g),
            "Green tint should be in 0.0-1.0 range"
        );
        assert!(
            (0.0..=1.0).contains(&b),
            "Blue tint should be in 0.0-1.0 range"
        );
    }
}

#[test]
fn test_scale_multiplies_dimensions() {
    // Verify that scale multiplier affects all dimensions uniformly
    let db = builtin_terrain_db();
    let tile = Tile::new(0, 0, TERRAIN_GROUND, WallType::Normal, &db).with_scale(2.0);

    let (width_x, height, width_z) = tile
        .visual
        .mesh_dimensions(tile.wall_type, terrain_height(tile.terrain));

    // Default wall: 1.0 x 2.5 x 1.0, scaled by 2.0
    assert_eq!(width_x, 2.0, "Width X should be doubled by scale");
    assert_eq!(height, 5.0, "Height should be doubled by scale (2.5 * 2.0)");
    assert_eq!(width_z, 2.0, "Width Z should be doubled by scale");
}

#[test]
fn test_scale_affects_y_position() {
    // Verify that scale affects Y-position calculation (height/2)
    let db = builtin_terrain_db();
    let tile = Tile::new(0, 0, TERRAIN_GROUND, WallType::Normal, &db).with_scale(2.0);

    let y_pos = tile
        .visual
        .mesh_y_position(tile.wall_type, terrain_height(tile.terrain));

    // Default wall Y-pos is 1.25 (height 2.5 / 2), scaled by 2.0 = 2.5
    assert_eq!(y_pos, 2.5, "Y-position should account for scaled height");
}

#[test]
fn test_y_offset_shifts_position() {
    // Verify that y_offset raises or lowers the mesh
    let db = builtin_terrain_db();
    let tile_raised = Tile::new(0, 0, TERRAIN_GROUND, WallType::Normal, &db);
    let mut tile_raised_visual = tile_raised.visual.clone();
    tile_raised_visual.y_offset = Some(0.5);

    // Ground / Normal wall → terrain_height = 0.0, but WallType::Normal overrides to 2.5
    let y_pos = tile_raised_visual.mesh_y_position(WallType::Normal, 0.0);

    // Default Y-pos 1.25 + offset 0.5 = 1.75
    assert_eq!(y_pos, 1.75, "Y-position should be raised by offset");

    // Test negative offset (sunken)
    let mut tile_sunken_visual = tile_raised.visual.clone();
    tile_sunken_visual.y_offset = Some(-0.3);

    let y_pos_sunken = tile_sunken_visual.mesh_y_position(WallType::Normal, 0.0);

    // Default Y-pos 1.25 + offset -0.3 = 0.95
    assert_eq!(
        y_pos_sunken, 0.95,
        "Y-position should be lowered by negative offset"
    );
}

#[test]
fn test_custom_dimensions_override_defaults() {
    // Verify that custom width_x and width_z override defaults
    let db = builtin_terrain_db();
    let tile =
        Tile::new(0, 0, TERRAIN_GROUND, WallType::Normal, &db).with_dimensions(0.8, 1.5, 0.8);

    let (width_x, height, width_z) = tile
        .visual
        .mesh_dimensions(tile.wall_type, terrain_height(tile.terrain));

    assert_eq!(width_x, 0.8, "Width X should be custom value");
    assert_eq!(height, 1.5, "Height should be custom value");
    assert_eq!(width_z, 0.8, "Width Z should be custom value");
}

#[test]
fn test_default_dimensions_are_full_tile() {
    // Verify that default dimensions fill a full tile (1.0 x 1.0)
    let db = builtin_terrain_db();
    let tile = Tile::new(0, 0, TERRAIN_GROUND, WallType::Normal, &db);

    assert_eq!(
        tile.visual.effective_width_x(),
        1.0,
        "Default width_x should be 1.0 (full tile)"
    );
    assert_eq!(
        tile.visual.effective_width_z(),
        1.0,
        "Default width_z should be 1.0 (full tile)"
    );
}

#[test]
fn test_flat_terrain_has_no_height() {
    // Verify that flat terrain types have 0.0 height when no wall
    let db = builtin_terrain_db();
    let ground_tile = Tile::new(0, 0, TERRAIN_GROUND, WallType::None, &db);
    let grass_tile = Tile::new(0, 0, TERRAIN_GRASS, WallType::None, &db);

    let (_, ground_height, _) = ground_tile
        .visual
        .mesh_dimensions(ground_tile.wall_type, terrain_height(ground_tile.terrain));
    let (_, grass_height, _) = grass_tile
        .visual
        .mesh_dimensions(grass_tile.wall_type, terrain_height(grass_tile.terrain));

    assert_eq!(
        ground_height, 0.0,
        "Ground terrain with no wall should have 0.0 height"
    );
    assert_eq!(
        grass_height, 0.0,
        "Grass terrain with no wall should have 0.0 height"
    );
}

#[test]
fn test_builder_methods_are_chainable() {
    // Verify that builder methods can be chained
    let db = builtin_terrain_db();
    let tile = Tile::new(0, 0, TERRAIN_GROUND, WallType::Normal, &db)
        .with_height(2.0)
        .with_scale(1.5)
        .with_color_tint(0.8, 0.9, 1.0);

    assert_eq!(tile.visual.height, Some(2.0));
    assert_eq!(tile.visual.scale, Some(1.5));
    assert_eq!(tile.visual.color_tint, Some((0.8, 0.9, 1.0)));
}

#[test]
fn test_combined_scale_and_custom_height() {
    // Verify that scale and custom height work together correctly
    let db = builtin_terrain_db();
    let tile = Tile::new(0, 0, TERRAIN_GROUND, WallType::Normal, &db)
        .with_height(2.0)
        .with_scale(1.5);

    let (_, height, _) = tile
        .visual
        .mesh_dimensions(tile.wall_type, terrain_height(tile.terrain));

    // Custom height 2.0 * scale 1.5 = 3.0
    assert_eq!(
        height, 3.0,
        "Height should be custom value multiplied by scale"
    );
}

#[test]
fn test_torch_default_height() {
    // Verify that torches use the same default height as walls (2.5)
    let db = builtin_terrain_db();
    let tile = Tile::new(0, 0, TERRAIN_GROUND, WallType::Torch, &db);

    let (_, height, _) = tile
        .visual
        .mesh_dimensions(tile.wall_type, terrain_height(tile.terrain));
    assert_eq!(
        height, 2.5,
        "Default torch height should be 2.5 units (same as wall)"
    );
}

#[test]
fn test_map_with_mixed_visual_metadata() {
    // Integration test: create a small map with mixed visual metadata
    let mut map = Map::new(
        0,
        "Test Map".to_string(),
        "Test Description".to_string(),
        3,
        3,
    );

    // Standard wall
    if let Some(tile) = map.get_tile_mut(Position::new(0, 0)) {
        tile.wall_type = WallType::Normal;
        tile.blocked = true;
    }

    // Tall custom wall
    if let Some(tile) = map.get_tile_mut(Position::new(1, 0)) {
        tile.wall_type = WallType::Normal;
        tile.blocked = true;
        tile.visual.height = Some(4.0);
    }

    // Scaled mountain
    if let Some(tile) = map.get_tile_mut(Position::new(2, 0)) {
        tile.terrain = TERRAIN_MOUNTAIN;
        tile.blocked = true;
        tile.visual.scale = Some(1.5);
    }

    // Tinted door
    if let Some(tile) = map.get_tile_mut(Position::new(0, 1)) {
        tile.wall_type = WallType::Door;
        tile.visual.color_tint = Some((1.0, 0.5, 0.5));
    }

    // Verify each tile has correct metadata
    let wall = map.get_tile(Position::new(0, 0)).unwrap();
    assert_eq!(
        wall.visual
            .mesh_dimensions(wall.wall_type, terrain_height(wall.terrain))
            .1,
        2.5
    );

    let tall = map.get_tile(Position::new(1, 0)).unwrap();
    assert_eq!(
        tall.visual
            .mesh_dimensions(tall.wall_type, terrain_height(tall.terrain))
            .1,
        4.0
    );

    let mountain = map.get_tile(Position::new(2, 0)).unwrap();
    assert_eq!(
        mountain
            .visual
            .mesh_dimensions(mountain.wall_type, terrain_height(mountain.terrain))
            .1,
        4.5 // 3.0 * 1.5
    );

    let door = map.get_tile(Position::new(0, 1)).unwrap();
    assert_eq!(door.visual.color_tint, Some((1.0, 0.5, 0.5)));
}

#[test]
fn test_visual_metadata_serialization_roundtrip() {
    // Verify that visual metadata survives serialization/deserialization
    use antares::domain::world::TileVisualMetadata;

    let metadata = TileVisualMetadata {
        height: Some(3.5),
        width_x: Some(0.9),
        width_z: Some(0.9),
        color_tint: Some((0.8, 0.9, 1.0)),
        scale: Some(1.2),
        y_offset: Some(0.3),
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
        grass_blade_config: None,
    };

    // Serialize to RON
    let ron_string = ron::to_string(&metadata).expect("Failed to serialize to RON");

    // Deserialize back
    let deserialized: TileVisualMetadata =
        ron::from_str(&ron_string).expect("Failed to deserialize from RON");

    // Verify all fields match
    assert_eq!(deserialized.height, Some(3.5));
    assert_eq!(deserialized.width_x, Some(0.9));
    assert_eq!(deserialized.width_z, Some(0.9));
    assert_eq!(deserialized.color_tint, Some((0.8, 0.9, 1.0)));
    assert_eq!(deserialized.scale, Some(1.2));
    assert_eq!(deserialized.y_offset, Some(0.3));
}

#[test]
fn test_backward_compatibility_default_visual() {
    // Verify that tiles without visual metadata use defaults.
    // terrain is now a TerrainId (u32), so RON uses the numeric ID.
    use antares::domain::types::TerrainId;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct OldTile {
        terrain: TerrainId,
        wall_type: WallType,
        blocked: bool,
        is_special: bool,
        is_dark: bool,
        visited: bool,
        x: i32,
        y: i32,
        // No visual field
    }

    let old_tile = OldTile {
        terrain: TERRAIN_GROUND,
        wall_type: WallType::Normal,
        blocked: true,
        is_special: false,
        is_dark: false,
        visited: false,
        x: 5,
        y: 10,
    };

    // Serialize old format
    let ron_string = ron::to_string(&old_tile).expect("Failed to serialize old tile");

    // Deserialize as new Tile (should use default visual)
    let new_tile: Tile =
        ron::from_str(&ron_string).expect("Failed to deserialize to new Tile format");

    // Verify visual metadata uses defaults
    assert_eq!(new_tile.visual.height, None);
    // Normal wall → effective_height ignores terrain_height, returns 2.5
    assert_eq!(
        new_tile.visual.effective_height(new_tile.wall_type, 0.0),
        2.5
    );
}

#[test]
fn test_map_round_trip_preserves_visual() {
    // Create a map with custom visual metadata
    use antares::domain::world::TileVisualMetadata;

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
        grass_blade_config: None,
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
    use antares::domain::world::TileVisualMetadata;
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
    let db = builtin_terrain_db();
    let tile = Tile::new(10, 15, TERRAIN_MOUNTAIN, WallType::None, &db)
        .with_height(5.0)
        .with_color_tint(0.4, 0.35, 0.3)
        .with_scale(1.5);

    assert_eq!(tile.x, 10);
    assert_eq!(tile.y, 15);
    assert_eq!(tile.terrain, TERRAIN_MOUNTAIN);
    assert_eq!(tile.visual.height, Some(5.0));
    assert_eq!(tile.visual.color_tint, Some((0.4, 0.35, 0.3)));
    assert_eq!(tile.visual.scale, Some(1.5));
}

#[test]
fn test_visual_metadata_effective_values() {
    use antares::domain::world::TileVisualMetadata;
    let mut metadata = TileVisualMetadata::default();

    // Test effective height with defaults using new (wall_type, terrain_height) signature.
    // Ground → terrain_height=0.0, Normal wall overrides to 2.5.
    assert_eq!(metadata.effective_height(WallType::Normal, 0.0), 2.5);
    // Mountain → terrain_height=3.0, None wall uses terrain_height.
    assert_eq!(metadata.effective_height(WallType::None, 3.0), 3.0);
    // Forest → terrain_height=2.2, None wall uses terrain_height.
    assert_eq!(metadata.effective_height(WallType::None, 2.2), 2.2);

    // Test with custom height
    metadata.height = Some(4.5);
    assert_eq!(metadata.effective_height(WallType::Normal, 0.0), 4.5);
    assert_eq!(metadata.effective_height(WallType::None, 3.0), 4.5);

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
    use antares::domain::world::TileVisualMetadata;
    let mut metadata = TileVisualMetadata::default();

    // Test default dimensions for normal wall (Ground → terrain_height=0.0)
    let (width_x, height, width_z) = metadata.mesh_dimensions(WallType::Normal, 0.0);
    assert_eq!(width_x, 1.0);
    assert_eq!(height, 2.5);
    assert_eq!(width_z, 1.0);

    // Test with custom dimensions and scale
    metadata.height = Some(3.0);
    metadata.width_x = Some(0.8);
    metadata.width_z = Some(0.6);
    metadata.scale = Some(1.5);

    let (width_x, height, width_z) = metadata.mesh_dimensions(WallType::Normal, 0.0);
    assert_eq!(width_x, 0.8 * 1.5);
    assert_eq!(height, 3.0 * 1.5);
    assert_eq!(width_z, 0.6 * 1.5);
}

#[test]
fn test_mesh_y_position_calculation() {
    use antares::domain::world::TileVisualMetadata;
    let mut metadata = TileVisualMetadata::default();

    // Test default Y position (half height, no offset)
    // Normal wall → height 2.5, y_pos = 2.5/2 = 1.25
    let y_pos = metadata.mesh_y_position(WallType::Normal, 0.0);
    assert_eq!(y_pos, 2.5 / 2.0); // height / 2

    // Test with custom height and offset
    metadata.height = Some(4.0);
    metadata.y_offset = Some(0.5);

    let y_pos = metadata.mesh_y_position(WallType::Normal, 0.0);
    assert_eq!(y_pos, 4.0 / 2.0 + 0.5); // (height / 2) + offset
}

#[test]
fn test_color_tint_range_validation() {
    use antares::domain::world::TileVisualMetadata;
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
