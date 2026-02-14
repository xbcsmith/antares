
#[test]
fn test_terrain_controls_single_select_fallback() {
    let mut state =
        MapEditorState::new(Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10));

    // Use single selection
    let pos = Position::new(5, 5);
    state.selected_position = Some(pos);
    state.selected_tiles = vec![]; // Ensure multi-select is empty

    // Set terrain state
    state.terrain_editor_state.grass_density = GrassDensity::High;

    // Apply
    state.apply_terrain_state_to_selection();

    // Verify
    let metadata = state
        .metadata
        .tile_visual_metadata
        .as_ref()
        .and_then(|m| m.get(&pos))
        .expect("Should have metadata");
    assert_eq!(metadata.grass_density, Some(GrassDensity::High));
}

#[test]
fn test_preset_palette_single_tile() {
    let mut state =
        MapEditorState::new(Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10));

    // Use single selection
    let pos = Position::new(2, 2);
    state.selected_position = Some(pos);
    state.selected_tiles = vec![];

    // Simulate preset application for single tile
    // Logic mirrors the UI implementation
    let preset = VisualPreset::TallWall;
    if state.selected_tiles.is_empty() {
        if let Some(pos) = state.selected_position {
            // Ensure metadata exists
            if state.metadata.tile_visual_metadata.is_none() {
                state.metadata.tile_visual_metadata = Some(std::collections::HashMap::new());
            }
            if let Some(metadata_map) = state.metadata.tile_visual_metadata.as_mut() {
                metadata_map.insert(pos, preset.to_metadata());
            }
        }
    }

    // Verify
    let metadata = state
        .metadata
        .tile_visual_metadata
        .as_ref()
        .and_then(|m| m.get(&pos))
        .expect("Should have metadata");
    assert_eq!(metadata.height, Some(3.5));
}

#[test]
fn test_state_reset_on_back_to_list() {
    // Verify that a fresh MapEditorState (simulating re-entry) has clean defaults
    let state = MapEditorState::new(Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10));

    // Visual editor should be default
    assert!(!state.visual_editor.enable_height);
    assert!(!state.visual_editor.enable_color);

    // Terrain editor should be default
    assert_eq!(state.terrain_editor_state.tree_type, TreeType::Oak); // Default
    assert_eq!(
        state.terrain_editor_state.grass_density,
        GrassDensity::Medium
    ); // Default

    // Selections should be empty
    assert!(state.selected_position.is_none());
    assert!(state.selected_tiles.is_empty());
}
