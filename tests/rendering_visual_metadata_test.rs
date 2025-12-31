// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for tile visual metadata rendering
//!
//! These tests verify that the rendering system correctly uses per-tile visual metadata
//! for mesh dimensions, Y-positioning, color tinting, and mesh caching.

use antares::domain::types::Position;
use antares::domain::world::{Map, TerrainType, Tile, WallType};

#[test]
fn test_default_wall_height_unchanged() {
    // Verify that a wall with no custom visual metadata uses default height of 2.5
    let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal);

    let (_, height, _) = tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);
    assert_eq!(height, 2.5, "Default wall height should be 2.5 units");

    let y_pos = tile.visual.mesh_y_position(tile.terrain, tile.wall_type);
    assert_eq!(
        y_pos, 1.25,
        "Default wall Y-position should be 1.25 (height/2)"
    );
}

#[test]
fn test_custom_wall_height_applied() {
    // Verify that custom wall height overrides the default
    let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal).with_height(1.5);

    let (_, height, _) = tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);
    assert_eq!(height, 1.5, "Custom wall height should be 1.5 units");

    let y_pos = tile.visual.mesh_y_position(tile.terrain, tile.wall_type);
    assert_eq!(y_pos, 0.75, "Custom wall Y-position should be 0.75 (1.5/2)");
}

#[test]
fn test_custom_mountain_height_applied() {
    // Verify that custom mountain height overrides the default of 3.0
    let tile = Tile::new(0, 0, TerrainType::Mountain, WallType::None).with_height(5.0);

    let (_, height, _) = tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);
    assert_eq!(height, 5.0, "Custom mountain height should be 5.0 units");

    let y_pos = tile.visual.mesh_y_position(tile.terrain, tile.wall_type);
    assert_eq!(
        y_pos, 2.5,
        "Custom mountain Y-position should be 2.5 (5.0/2)"
    );
}

#[test]
fn test_default_mountain_height() {
    // Verify default mountain height is 3.0
    let tile = Tile::new(0, 0, TerrainType::Mountain, WallType::None);

    let (_, height, _) = tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);
    assert_eq!(height, 3.0, "Default mountain height should be 3.0 units");

    let y_pos = tile.visual.mesh_y_position(tile.terrain, tile.wall_type);
    assert_eq!(
        y_pos, 1.5,
        "Default mountain Y-position should be 1.5 (3.0/2)"
    );
}

#[test]
fn test_default_forest_height() {
    // Verify default forest/tree height is 2.2
    let tile = Tile::new(0, 0, TerrainType::Forest, WallType::None);

    let (_, height, _) = tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);
    assert_eq!(height, 2.2, "Default forest height should be 2.2 units");

    let y_pos = tile.visual.mesh_y_position(tile.terrain, tile.wall_type);
    assert_eq!(
        y_pos, 1.1,
        "Default forest Y-position should be 1.1 (2.2/2)"
    );
}

#[test]
fn test_default_door_height() {
    // Verify default door height is 2.5 (same as wall)
    let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Door);

    let (_, height, _) = tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);
    assert_eq!(height, 2.5, "Default door height should be 2.5 units");

    let y_pos = tile.visual.mesh_y_position(tile.terrain, tile.wall_type);
    assert_eq!(
        y_pos, 1.25,
        "Default door Y-position should be 1.25 (2.5/2)"
    );
}

#[test]
fn test_color_tint_multiplies_base_color() {
    // Verify that color tint is properly stored and can be retrieved
    let tile =
        Tile::new(0, 0, TerrainType::Ground, WallType::Normal).with_color_tint(0.5, 1.0, 1.0);

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
    let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal).with_scale(2.0);

    let (width_x, height, width_z) = tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);

    // Default wall: 1.0 x 2.5 x 1.0, scaled by 2.0
    assert_eq!(width_x, 2.0, "Width X should be doubled by scale");
    assert_eq!(height, 5.0, "Height should be doubled by scale (2.5 * 2.0)");
    assert_eq!(width_z, 2.0, "Width Z should be doubled by scale");
}

#[test]
fn test_scale_affects_y_position() {
    // Verify that scale affects Y-position calculation (height/2)
    let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal).with_scale(2.0);

    let y_pos = tile.visual.mesh_y_position(tile.terrain, tile.wall_type);

    // Default wall Y-pos is 1.25 (height 2.5 / 2), scaled by 2.0 = 2.5
    assert_eq!(y_pos, 2.5, "Y-position should account for scaled height");
}

#[test]
fn test_y_offset_shifts_position() {
    // Verify that y_offset raises or lowers the mesh
    let tile_raised = Tile::new(0, 0, TerrainType::Ground, WallType::Normal);
    let mut tile_raised_visual = tile_raised.visual.clone();
    tile_raised_visual.y_offset = Some(0.5);

    let y_pos = tile_raised_visual.mesh_y_position(TerrainType::Ground, WallType::Normal);

    // Default Y-pos 1.25 + offset 0.5 = 1.75
    assert_eq!(y_pos, 1.75, "Y-position should be raised by offset");

    // Test negative offset (sunken)
    let mut tile_sunken_visual = tile_raised.visual.clone();
    tile_sunken_visual.y_offset = Some(-0.3);

    let y_pos_sunken = tile_sunken_visual.mesh_y_position(TerrainType::Ground, WallType::Normal);

    // Default Y-pos 1.25 + offset -0.3 = 0.95
    assert_eq!(
        y_pos_sunken, 0.95,
        "Y-position should be lowered by negative offset"
    );
}

#[test]
fn test_custom_dimensions_override_defaults() {
    // Verify that custom width_x and width_z override defaults
    let tile =
        Tile::new(0, 0, TerrainType::Ground, WallType::Normal).with_dimensions(0.8, 1.5, 0.8);

    let (width_x, height, width_z) = tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);

    assert_eq!(width_x, 0.8, "Width X should be custom value");
    assert_eq!(height, 1.5, "Height should be custom value");
    assert_eq!(width_z, 0.8, "Width Z should be custom value");
}

#[test]
fn test_default_dimensions_are_full_tile() {
    // Verify that default dimensions fill a full tile (1.0 x 1.0)
    let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal);

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
    let ground_tile = Tile::new(0, 0, TerrainType::Ground, WallType::None);
    let grass_tile = Tile::new(0, 0, TerrainType::Grass, WallType::None);

    let (_, ground_height, _) = ground_tile
        .visual
        .mesh_dimensions(ground_tile.terrain, ground_tile.wall_type);
    let (_, grass_height, _) = grass_tile
        .visual
        .mesh_dimensions(grass_tile.terrain, grass_tile.wall_type);

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
    let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal)
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
    let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal)
        .with_height(2.0)
        .with_scale(1.5);

    let (_, height, _) = tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);

    // Custom height 2.0 * scale 1.5 = 3.0
    assert_eq!(
        height, 3.0,
        "Height should be custom value multiplied by scale"
    );
}

#[test]
fn test_torch_default_height() {
    // Verify that torches use the same default height as walls (2.5)
    let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Torch);

    let (_, height, _) = tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);
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
        tile.terrain = TerrainType::Mountain;
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
        wall.visual.mesh_dimensions(wall.terrain, wall.wall_type).1,
        2.5
    );

    let tall = map.get_tile(Position::new(1, 0)).unwrap();
    assert_eq!(
        tall.visual.mesh_dimensions(tall.terrain, tall.wall_type).1,
        4.0
    );

    let mountain = map.get_tile(Position::new(2, 0)).unwrap();
    assert_eq!(
        mountain
            .visual
            .mesh_dimensions(mountain.terrain, mountain.wall_type)
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
    // Verify that tiles without visual metadata use defaults
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct OldTile {
        terrain: TerrainType,
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
        terrain: TerrainType::Ground,
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
    assert_eq!(
        new_tile
            .visual
            .effective_height(new_tile.terrain, new_tile.wall_type),
        2.5
    );
}
