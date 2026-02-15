// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign Builder GUI Integration Tests
//!
//! Tests for the visual metadata preset system and bulk editing functionality
//! added to the Campaign Builder map editor in Phase 4.

use antares::domain::types::Position;
use antares::domain::world::{Map, TileVisualMetadata};
use campaign_builder::map_editor::{MapEditorState, VisualPreset};

// ===== Preset System Tests =====

#[test]
fn test_all_presets_defined() {
    // Verify all presets are accessible
    let presets = VisualPreset::all();
    assert_eq!(presets.len(), 32, "Expected 32 presets");

    // Verify each has a name
    for preset in presets {
        assert!(!preset.name().is_empty(), "Preset must have a name");
    }
}

#[test]
fn test_preset_default() {
    let preset = VisualPreset::Default;
    let metadata = preset.to_metadata();

    assert_eq!(preset.name(), "Default (None)");
    assert_eq!(metadata.height, None);
    assert_eq!(metadata.width_x, None);
    assert_eq!(metadata.width_z, None);
    assert_eq!(metadata.color_tint, None);
    assert_eq!(metadata.scale, None);
    assert_eq!(metadata.y_offset, None);
}

#[test]
fn test_preset_short_wall() {
    let preset = VisualPreset::ShortWall;
    let metadata = preset.to_metadata();

    assert_eq!(preset.name(), "Short Wall");
    assert_eq!(metadata.height, Some(1.5));
    assert_eq!(metadata.width_x, None);
    assert_eq!(metadata.width_z, None);
    assert_eq!(metadata.color_tint, None);
    assert_eq!(metadata.scale, None);
    assert_eq!(metadata.y_offset, None);
}

#[test]
fn test_preset_tall_wall() {
    let preset = VisualPreset::TallWall;
    let metadata = preset.to_metadata();

    assert_eq!(preset.name(), "Tall Wall");
    assert_eq!(metadata.height, Some(3.5));
    assert_eq!(metadata.width_x, None);
    assert_eq!(metadata.width_z, None);
}

#[test]
fn test_preset_thin_wall() {
    let preset = VisualPreset::ThinWall;
    let metadata = preset.to_metadata();

    assert_eq!(preset.name(), "Thin Wall");
    assert_eq!(metadata.height, None);
    assert_eq!(metadata.width_x, None);
    assert_eq!(metadata.width_z, Some(0.2));
    assert_eq!(metadata.color_tint, None);
}

#[test]
fn test_preset_small_tree() {
    let preset = VisualPreset::SmallTree;
    let metadata = preset.to_metadata();

    assert_eq!(preset.name(), "Small Tree");
    assert_eq!(metadata.height, Some(2.0));
    assert_eq!(metadata.scale, Some(0.5));

    // Verify green tint
    let (r, g, b) = metadata
        .color_tint
        .expect("Small tree should have color tint");
    assert_eq!(r, 0.6);
    assert_eq!(g, 0.9);
    assert_eq!(b, 0.6);
}

#[test]
fn test_preset_large_tree() {
    let preset = VisualPreset::LargeTree;
    let metadata = preset.to_metadata();

    assert_eq!(preset.name(), "Large Tree");
    assert_eq!(metadata.height, Some(4.0));
    assert_eq!(metadata.scale, Some(1.5));

    // Verify green tint
    let (r, g, b) = metadata
        .color_tint
        .expect("Large tree should have color tint");
    assert_eq!(r, 0.5);
    assert_eq!(g, 0.8);
    assert_eq!(b, 0.5);
}

#[test]
fn test_preset_low_mountain() {
    let preset = VisualPreset::LowMountain;
    let metadata = preset.to_metadata();

    assert_eq!(preset.name(), "Low Mountain");
    assert_eq!(metadata.height, Some(2.0));

    // Verify gray tint
    let (r, g, b) = metadata
        .color_tint
        .expect("Low mountain should have color tint");
    assert_eq!(r, 0.7);
    assert_eq!(g, 0.7);
    assert_eq!(b, 0.7);
}

#[test]
fn test_preset_high_mountain() {
    let preset = VisualPreset::HighMountain;
    let metadata = preset.to_metadata();

    assert_eq!(preset.name(), "High Mountain");
    assert_eq!(metadata.height, Some(5.0));

    // Verify darker gray tint
    let (r, g, b) = metadata
        .color_tint
        .expect("High mountain should have color tint");
    assert_eq!(r, 0.6);
    assert_eq!(g, 0.6);
    assert_eq!(b, 0.6);
}

#[test]
fn test_preset_sunken() {
    let preset = VisualPreset::Sunken;
    let metadata = preset.to_metadata();

    assert_eq!(preset.name(), "Sunken");
    assert_eq!(metadata.height, None);
    assert_eq!(metadata.y_offset, Some(-0.5));
    assert_eq!(metadata.color_tint, None);
}

#[test]
fn test_preset_raised() {
    let preset = VisualPreset::Raised;
    let metadata = preset.to_metadata();

    assert_eq!(preset.name(), "Raised");
    assert_eq!(metadata.height, None);
    assert_eq!(metadata.y_offset, Some(0.5));
    assert_eq!(metadata.color_tint, None);
}

// ===== Multi-Select Mode Tests =====

#[test]
fn test_multi_select_mode_initialization() {
    let map = Map::new(1, "Test Map".to_string(), "Test".to_string(), 10, 10);
    let editor = MapEditorState::new(map);

    assert!(!editor.multi_select_mode);
    assert_eq!(editor.selected_tiles.len(), 0);
}

#[test]
fn test_toggle_multi_select_mode() {
    let map = Map::new(1, "Test Map".to_string(), "Test".to_string(), 10, 10);
    let mut editor = MapEditorState::new(map);

    // Initially off
    assert!(!editor.multi_select_mode);

    // Toggle on
    editor.toggle_multi_select_mode();
    assert!(editor.multi_select_mode);

    // Toggle off
    editor.toggle_multi_select_mode();
    assert!(!editor.multi_select_mode);
}

#[test]
fn test_toggle_multi_select_clears_selection() {
    let map = Map::new(1, "Test Map".to_string(), "Test".to_string(), 10, 10);
    let mut editor = MapEditorState::new(map);

    // Add some selections
    editor.multi_select_mode = true;
    editor.toggle_tile_selection(Position::new(0, 0));
    editor.toggle_tile_selection(Position::new(1, 1));
    assert_eq!(editor.selected_tiles.len(), 2);

    // Toggling off should clear
    editor.toggle_multi_select_mode();
    assert_eq!(editor.selected_tiles.len(), 0);
}

#[test]
fn test_toggle_tile_selection() {
    let map = Map::new(1, "Test Map".to_string(), "Test".to_string(), 10, 10);
    let mut editor = MapEditorState::new(map);

    let pos1 = Position::new(0, 0);
    let pos2 = Position::new(1, 1);

    // Add first tile
    editor.toggle_tile_selection(pos1);
    assert_eq!(editor.selected_tiles.len(), 1);
    assert!(editor.is_tile_selected(pos1));

    // Add second tile
    editor.toggle_tile_selection(pos2);
    assert_eq!(editor.selected_tiles.len(), 2);
    assert!(editor.is_tile_selected(pos2));

    // Remove first tile
    editor.toggle_tile_selection(pos1);
    assert_eq!(editor.selected_tiles.len(), 1);
    assert!(!editor.is_tile_selected(pos1));
    assert!(editor.is_tile_selected(pos2));

    // Remove second tile
    editor.toggle_tile_selection(pos2);
    assert_eq!(editor.selected_tiles.len(), 0);
}

#[test]
fn test_clear_tile_selection() {
    let map = Map::new(1, "Test Map".to_string(), "Test".to_string(), 10, 10);
    let mut editor = MapEditorState::new(map);

    // Add multiple tiles
    editor.toggle_tile_selection(Position::new(0, 0));
    editor.toggle_tile_selection(Position::new(1, 1));
    editor.toggle_tile_selection(Position::new(2, 2));
    assert_eq!(editor.selected_tiles.len(), 3);

    // Clear all
    editor.clear_tile_selection();
    assert_eq!(editor.selected_tiles.len(), 0);
}

#[test]
fn test_is_tile_selected() {
    let map = Map::new(1, "Test Map".to_string(), "Test".to_string(), 10, 10);
    let mut editor = MapEditorState::new(map);

    let pos = Position::new(5, 5);
    assert!(!editor.is_tile_selected(pos));

    editor.toggle_tile_selection(pos);
    assert!(editor.is_tile_selected(pos));
}

// ===== Bulk Edit Tests =====

#[test]
fn test_apply_visual_metadata_single_tile() {
    let map = Map::new(1, "Test Map".to_string(), "Test".to_string(), 10, 10);
    let mut editor = MapEditorState::new(map);

    let pos = Position::new(0, 0);
    editor.selected_position = Some(pos);

    let metadata = TileVisualMetadata {
        height: Some(2.5),
        color_tint: Some((1.0, 0.5, 0.5)),
        ..Default::default()
    };

    editor.apply_visual_metadata(pos, &metadata.clone());

    let tile = editor.map.get_tile(pos).expect("Tile should exist");
    assert_eq!(tile.visual.height, Some(2.5));
    assert_eq!(tile.visual.color_tint, Some((1.0, 0.5, 0.5)));
    assert!(editor.has_changes);
}

#[test]
fn test_apply_visual_metadata_to_selection_empty() {
    let map = Map::new(1, "Test Map".to_string(), "Test".to_string(), 10, 10);
    let mut editor = MapEditorState::new(map);

    let pos = Position::new(0, 0);
    editor.selected_position = Some(pos);

    let metadata = TileVisualMetadata {
        height: Some(3.0),
        ..Default::default()
    };

    // No multi-selection, should apply to current position
    editor.apply_visual_metadata_to_selection(&metadata);

    let tile = editor.map.get_tile(pos).expect("Tile should exist");
    assert_eq!(tile.visual.height, Some(3.0));
}

#[test]
fn test_apply_visual_metadata_to_multiple_tiles() {
    let map = Map::new(1, "Test Map".to_string(), "Test".to_string(), 10, 10);
    let mut editor = MapEditorState::new(map);

    let pos1 = Position::new(0, 0);
    let pos2 = Position::new(1, 1);
    let pos3 = Position::new(2, 2);

    // Select multiple tiles
    editor.toggle_tile_selection(pos1);
    editor.toggle_tile_selection(pos2);
    editor.toggle_tile_selection(pos3);

    let metadata = TileVisualMetadata {
        height: Some(4.0),
        scale: Some(1.5),
        color_tint: Some((0.8, 0.8, 0.8)),
        ..Default::default()
    };

    editor.apply_visual_metadata_to_selection(&metadata);

    // Verify all tiles have the metadata
    for pos in &[pos1, pos2, pos3] {
        let tile = editor.map.get_tile(*pos).expect("Tile should exist");
        assert_eq!(tile.visual.height, Some(4.0));
        assert_eq!(tile.visual.scale, Some(1.5));
        assert_eq!(tile.visual.color_tint, Some((0.8, 0.8, 0.8)));
    }
}

#[test]
fn test_bulk_edit_workflow() {
    let map = Map::new(1, "Test Map".to_string(), "Test".to_string(), 10, 10);
    let mut editor = MapEditorState::new(map);

    // Workflow: Create a wall section with uniform height
    let wall_positions = vec![
        Position::new(0, 0),
        Position::new(0, 1),
        Position::new(0, 2),
        Position::new(0, 3),
        Position::new(0, 4),
    ];

    // Enable multi-select mode
    editor.toggle_multi_select_mode();
    assert!(editor.multi_select_mode);

    // Select wall tiles
    for pos in &wall_positions {
        editor.toggle_tile_selection(*pos);
    }
    assert_eq!(editor.selected_tiles.len(), 5);

    // Apply tall wall preset
    let preset_metadata = VisualPreset::TallWall.to_metadata();
    editor.apply_visual_metadata_to_selection(&preset_metadata);

    // Verify all wall tiles have the same height
    for pos in &wall_positions {
        let tile = editor.map.get_tile(*pos).expect("Tile should exist");
        assert_eq!(tile.visual.height, Some(3.5));
    }

    // Disable multi-select mode
    editor.toggle_multi_select_mode();
    assert!(!editor.multi_select_mode);
    assert_eq!(editor.selected_tiles.len(), 0);
}

#[test]
fn test_preset_application_workflow() {
    let map = Map::new(1, "Test Map".to_string(), "Test".to_string(), 10, 10);
    let mut editor = MapEditorState::new(map);

    let pos = Position::new(5, 5);

    // Apply each preset and verify it's applied correctly
    for preset in VisualPreset::all() {
        let metadata = preset.to_metadata();
        editor.apply_visual_metadata(pos, &metadata.clone());

        let tile = editor.map.get_tile(pos).expect("Tile should exist");
        assert_eq!(tile.visual, metadata, "Preset {} failed", preset.name());
    }
}

#[test]
fn test_mixed_editing_workflow() {
    let map = Map::new(1, "Test Map".to_string(), "Test".to_string(), 10, 10);
    let mut editor = MapEditorState::new(map);

    // Step 1: Apply preset to single tile
    let pos1 = Position::new(0, 0);
    editor.apply_visual_metadata(pos1, &VisualPreset::SmallTree.to_metadata());

    let tile1 = editor.map.get_tile(pos1).expect("Tile should exist");
    assert_eq!(tile1.visual.height, Some(2.0));
    assert_eq!(tile1.visual.scale, Some(0.5));

    // Step 2: Custom edit to multiple tiles
    editor.toggle_tile_selection(Position::new(1, 1));
    editor.toggle_tile_selection(Position::new(2, 2));

    let custom_metadata = TileVisualMetadata {
        height: Some(6.0),
        color_tint: Some((0.9, 0.9, 0.95)),
        ..Default::default()
    };

    editor.apply_visual_metadata_to_selection(&custom_metadata);

    let tile2 = editor
        .map
        .get_tile(Position::new(1, 1))
        .expect("Tile should exist");
    assert_eq!(tile2.visual.height, Some(6.0));
    assert_eq!(tile2.visual.color_tint, Some((0.9, 0.9, 0.95)));
}

// ===== Integration Tests =====

#[test]
fn test_editor_state_initialization_with_phase4_fields() {
    let map = Map::new(1, "Test Map".to_string(), "Test".to_string(), 10, 10);
    let editor = MapEditorState::new(map);

    // Verify Phase 4 fields are initialized correctly
    assert_eq!(editor.selected_tiles.len(), 0);
    assert!(!editor.multi_select_mode);
    assert!(!editor.visual_editor.enable_height);
    assert!(!editor.visual_editor.enable_width_x);
    assert!(!editor.visual_editor.enable_width_z);
    assert!(!editor.visual_editor.enable_color_tint);
    assert!(!editor.visual_editor.enable_scale);
    assert!(!editor.visual_editor.enable_y_offset);
}

#[test]
fn test_has_changes_flag_on_visual_edit() {
    let map = Map::new(1, "Test Map".to_string(), "Test".to_string(), 10, 10);
    let mut editor = MapEditorState::new(map);

    assert!(!editor.has_changes);

    let metadata = TileVisualMetadata {
        height: Some(2.0),
        ..Default::default()
    };

    editor.apply_visual_metadata(Position::new(0, 0), &metadata);
    assert!(editor.has_changes);
}

#[test]
fn test_preset_names_are_unique() {
    let presets = VisualPreset::all();
    let mut names = std::collections::HashSet::new();

    for preset in presets {
        let name = preset.name();
        assert!(names.insert(name), "Duplicate preset name found: {}", name);
    }
}

#[test]
fn test_all_presets_produce_valid_metadata() {
    for preset in VisualPreset::all() {
        let metadata = preset.to_metadata();

        // Verify field constraints
        if let Some(height) = metadata.height {
            assert!(height >= 0.0, "Height must be non-negative");
        }
        if let Some(width_x) = metadata.width_x {
            assert!(width_x > 0.0, "Width X must be positive");
        }
        if let Some(width_z) = metadata.width_z {
            assert!(width_z > 0.0, "Width Z must be positive");
        }
        if let Some(scale) = metadata.scale {
            assert!(scale > 0.0, "Scale must be positive");
        }
        if let Some((r, g, b)) = metadata.color_tint {
            assert!((0.0..=1.0).contains(&r), "R must be in 0.0-1.0 range");
            assert!((0.0..=1.0).contains(&g), "G must be in 0.0-1.0 range");
            assert!((0.0..=1.0).contains(&b), "B must be in 0.0-1.0 range");
        }
    }
}
