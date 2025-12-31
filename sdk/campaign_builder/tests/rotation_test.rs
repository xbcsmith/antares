// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Phase 5: Rotation Support Tests
//!
//! Tests for rotation_y field in TileVisualMetadata and Campaign Builder integration.

use antares::domain::types::Position;
use antares::domain::world::{Map, TerrainType, Tile, TileVisualMetadata, WallType};
use campaign_builder::map_editor::{MapEditorState, VisualMetadataEditor, VisualPreset};

// ===== Domain Model Tests =====

#[test]
fn test_rotation_y_field_default() {
    let metadata = TileVisualMetadata::default();
    assert_eq!(metadata.rotation_y, None);
}

#[test]
fn test_rotation_y_effective_default() {
    let metadata = TileVisualMetadata::default();
    assert_eq!(metadata.effective_rotation_y(), 0.0);
}

#[test]
fn test_rotation_y_custom_value() {
    let metadata = TileVisualMetadata {
        rotation_y: Some(45.0),
        ..Default::default()
    };
    assert_eq!(metadata.effective_rotation_y(), 45.0);
}

#[test]
fn test_rotation_y_radians_conversion() {
    let m0 = TileVisualMetadata {
        rotation_y: Some(0.0),
        ..Default::default()
    };
    assert!((m0.rotation_y_radians() - 0.0).abs() < 0.001);

    let m90 = TileVisualMetadata {
        rotation_y: Some(90.0),
        ..Default::default()
    };
    assert!((m90.rotation_y_radians() - std::f32::consts::FRAC_PI_2).abs() < 0.001);

    let m180 = TileVisualMetadata {
        rotation_y: Some(180.0),
        ..Default::default()
    };
    assert!((m180.rotation_y_radians() - std::f32::consts::PI).abs() < 0.001);

    let m360 = TileVisualMetadata {
        rotation_y: Some(360.0),
        ..Default::default()
    };
    assert!((m360.rotation_y_radians() - std::f32::consts::TAU).abs() < 0.001);
}

#[test]
fn test_rotation_y_negative_values() {
    let metadata = TileVisualMetadata {
        rotation_y: Some(-45.0),
        ..Default::default()
    };
    assert_eq!(metadata.effective_rotation_y(), -45.0);
    assert!((metadata.rotation_y_radians() - (-45.0_f32).to_radians()).abs() < 0.001);
}

#[test]
fn test_rotation_y_large_values() {
    let metadata = TileVisualMetadata {
        rotation_y: Some(720.0),
        ..Default::default()
    }; // Two full rotations
    assert_eq!(metadata.effective_rotation_y(), 720.0);
}

#[test]
fn test_tile_with_rotation() {
    let mut tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal);
    tile.visual.rotation_y = Some(45.0);

    assert_eq!(tile.visual.rotation_y, Some(45.0));
    assert_eq!(tile.visual.effective_rotation_y(), 45.0);
}

// ===== Serialization Tests =====

#[test]
fn test_rotation_y_serialization() {
    let metadata = TileVisualMetadata {
        rotation_y: Some(45.0),
        ..Default::default()
    };

    let serialized = ron::to_string(&metadata).unwrap();
    assert!(serialized.contains("rotation_y"));
    assert!(serialized.contains("45.0"));

    let deserialized: TileVisualMetadata = ron::from_str(&serialized).unwrap();
    assert_eq!(deserialized.rotation_y, Some(45.0));
}

#[test]
fn test_rotation_y_backward_compatibility() {
    // Old RON data without rotation_y field should deserialize with None
    let ron_data = r#"(
        height: Some(2.5),
        width_x: Some(1.0),
        width_z: Some(1.0),
        color_tint: None,
        scale: None,
        y_offset: None,
    )"#;

    let metadata: TileVisualMetadata = ron::from_str(ron_data).unwrap();
    assert_eq!(metadata.height, Some(2.5));
    assert_eq!(metadata.rotation_y, None);
    assert_eq!(metadata.effective_rotation_y(), 0.0);
}

// ===== Preset Tests =====

#[test]
fn test_preset_rotated45() {
    let preset = VisualPreset::Rotated45;
    let metadata = preset.to_metadata();

    assert_eq!(preset.name(), "Rotated 45°");
    assert_eq!(metadata.rotation_y, Some(45.0));
    assert_eq!(metadata.height, None);
    assert_eq!(metadata.width_x, None);
    assert_eq!(metadata.width_z, None);
}

#[test]
fn test_preset_rotated90() {
    let preset = VisualPreset::Rotated90;
    let metadata = preset.to_metadata();

    assert_eq!(preset.name(), "Rotated 90°");
    assert_eq!(metadata.rotation_y, Some(90.0));
    assert_eq!(metadata.height, None);
}

#[test]
fn test_preset_diagonal_wall() {
    let preset = VisualPreset::DiagonalWall;
    let metadata = preset.to_metadata();

    assert_eq!(preset.name(), "Diagonal Wall");
    assert_eq!(metadata.rotation_y, Some(45.0));
    assert_eq!(metadata.width_z, Some(0.2));
    assert_eq!(metadata.height, None);
}

#[test]
fn test_preset_all_includes_rotation_presets() {
    let all_presets = VisualPreset::all();

    assert!(all_presets.contains(&VisualPreset::Rotated45));
    assert!(all_presets.contains(&VisualPreset::Rotated90));
    assert!(all_presets.contains(&VisualPreset::DiagonalWall));
}

// ===== Editor State Tests =====

#[test]
fn test_visual_editor_default_rotation() {
    let editor = VisualMetadataEditor::default();

    assert_eq!(editor.enable_rotation_y, false);
    assert_eq!(editor.temp_rotation_y, 0.0);
}

#[test]
fn test_visual_editor_load_rotation_from_tile() {
    let mut tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal);
    tile.visual.rotation_y = Some(45.0);

    let mut editor = VisualMetadataEditor::default();
    editor.load_from_tile(&tile);

    assert_eq!(editor.enable_rotation_y, true);
    assert_eq!(editor.temp_rotation_y, 45.0);
}

#[test]
fn test_visual_editor_load_no_rotation_from_tile() {
    let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal);

    let mut editor = VisualMetadataEditor::default();
    editor.load_from_tile(&tile);

    assert_eq!(editor.enable_rotation_y, false);
    assert_eq!(editor.temp_rotation_y, 0.0);
}

#[test]
fn test_visual_editor_to_metadata_with_rotation() {
    let editor = VisualMetadataEditor {
        enable_rotation_y: true,
        temp_rotation_y: 90.0,
        ..Default::default()
    };

    let metadata = editor.to_metadata();
    assert_eq!(metadata.rotation_y, Some(90.0));
}

#[test]
fn test_visual_editor_to_metadata_without_rotation() {
    let editor = VisualMetadataEditor {
        enable_rotation_y: false,
        temp_rotation_y: 90.0,
        ..Default::default()
    }; // Should be ignored

    let metadata = editor.to_metadata();
    assert_eq!(metadata.rotation_y, None);
}

// ===== Integration Tests =====

#[test]
fn test_map_editor_apply_rotation_to_tile() {
    let map = Map::new(1, "Test Map".to_string(), "".to_string(), 10, 10);
    let mut editor_state = MapEditorState::new(map);

    let pos = Position::new(5, 5);
    editor_state.visual_editor.enable_rotation_y = true;
    editor_state.visual_editor.temp_rotation_y = 45.0;

    let metadata = editor_state.visual_editor.to_metadata();
    editor_state.apply_visual_metadata(pos, &metadata);

    let map = &mut editor_state.map;
    if let Some(tile) = map.get_tile(pos) {
        assert_eq!(tile.visual.rotation_y, Some(45.0));
    } else {
        panic!("Tile not found at {:?}", pos);
    }
}

#[test]
fn test_map_editor_apply_rotation_preset() {
    let map = Map::new(1, "Test Map".to_string(), "".to_string(), 10, 10);
    let mut editor_state = MapEditorState::new(map);

    let pos = Position::new(3, 3);
    let preset = VisualPreset::Rotated90;
    let metadata = preset.to_metadata();

    editor_state.apply_visual_metadata(pos, &metadata);

    let map = &mut editor_state.map;
    if let Some(tile) = map.get_tile(pos) {
        assert_eq!(tile.visual.rotation_y, Some(90.0));
    } else {
        panic!("Tile not found at {:?}", pos);
    }
}

#[test]
fn test_map_editor_bulk_apply_rotation() {
    let map = Map::new(1, "Test Map".to_string(), "".to_string(), 10, 10);
    let mut editor_state = MapEditorState::new(map);

    // Select multiple tiles
    editor_state.toggle_multi_select_mode();
    editor_state.toggle_tile_selection(Position::new(2, 2));
    editor_state.toggle_tile_selection(Position::new(3, 3));
    editor_state.toggle_tile_selection(Position::new(4, 4));

    // Apply diagonal wall preset
    let preset = VisualPreset::DiagonalWall;
    let metadata = preset.to_metadata();
    editor_state.apply_visual_metadata_to_selection(&metadata);

    // Verify all selected tiles have the rotation
    let map = &mut editor_state.map;
    for pos in [
        Position::new(2, 2),
        Position::new(3, 3),
        Position::new(4, 4),
    ] {
        if let Some(tile) = map.get_tile(pos) {
            assert_eq!(tile.visual.rotation_y, Some(45.0));
            assert_eq!(tile.visual.width_z, Some(0.2));
        } else {
            panic!("Tile not found at {:?}", pos);
        }
    }
}

// ===== Combined Feature Tests =====

#[test]
fn test_rotation_with_other_properties() {
    let metadata = TileVisualMetadata {
        height: Some(2.5),
        rotation_y: Some(45.0),
        scale: Some(1.2),
        color_tint: Some((0.8, 0.9, 1.0)),
        ..Default::default()
    };

    assert_eq!(
        metadata.effective_height(TerrainType::Ground, WallType::Normal),
        2.5
    );
    assert_eq!(metadata.effective_rotation_y(), 45.0);
    assert_eq!(metadata.effective_scale(), 1.2);
    assert_eq!(metadata.color_tint, Some((0.8, 0.9, 1.0)));

    let (w, h, d) = metadata.mesh_dimensions(TerrainType::Ground, WallType::Normal);
    assert_eq!(w, 1.2); // 1.0 * 1.2 scale
    assert_eq!(h, 3.0); // 2.5 * 1.2 scale
    assert_eq!(d, 1.2); // 1.0 * 1.2 scale
}

#[test]
fn test_rotation_roundtrip_through_editor() {
    let mut original_tile = Tile::new(0, 0, TerrainType::Ground, WallType::Door);
    original_tile.visual.rotation_y = Some(135.0);
    original_tile.visual.height = Some(2.0);

    // Load into editor
    let mut editor = VisualMetadataEditor::default();
    editor.load_from_tile(&original_tile);

    assert_eq!(editor.enable_rotation_y, true);
    assert_eq!(editor.temp_rotation_y, 135.0);
    assert_eq!(editor.enable_height, true);
    assert_eq!(editor.temp_height, 2.0);

    // Convert back
    let metadata = editor.to_metadata();
    assert_eq!(metadata.rotation_y, Some(135.0));
    assert_eq!(metadata.height, Some(2.0));
}

// ===== Edge Case Tests =====

#[test]
fn test_rotation_zero_vs_none() {
    let with_zero = TileVisualMetadata {
        rotation_y: Some(0.0),
        ..Default::default()
    };

    let with_none = TileVisualMetadata::default();

    // Both should have effective rotation of 0
    assert_eq!(with_zero.effective_rotation_y(), 0.0);
    assert_eq!(with_none.effective_rotation_y(), 0.0);

    // But serialized form differs
    assert_ne!(with_zero.rotation_y, with_none.rotation_y);
}

#[test]
fn test_rotation_boundary_values() {
    // Test common boundary values
    for angle in [0.0, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0, 360.0] {
        let metadata = TileVisualMetadata {
            rotation_y: Some(angle),
            ..Default::default()
        };
        assert_eq!(metadata.effective_rotation_y(), angle);
        assert!((metadata.rotation_y_radians() - angle.to_radians()).abs() < 0.001);
    }
}

#[test]
fn test_rotation_with_all_fields_none() {
    let metadata = TileVisualMetadata::default();

    assert_eq!(metadata.height, None);
    assert_eq!(metadata.width_x, None);
    assert_eq!(metadata.width_z, None);
    assert_eq!(metadata.color_tint, None);
    assert_eq!(metadata.scale, None);
    assert_eq!(metadata.y_offset, None);
    assert_eq!(metadata.rotation_y, None);

    // All effective values should be defaults
    assert_eq!(
        metadata.effective_height(TerrainType::Ground, WallType::None),
        0.0
    );
    assert_eq!(metadata.effective_width_x(), 1.0);
    assert_eq!(metadata.effective_width_z(), 1.0);
    assert_eq!(metadata.effective_scale(), 1.0);
    assert_eq!(metadata.effective_y_offset(), 0.0);
    assert_eq!(metadata.effective_rotation_y(), 0.0);
}
