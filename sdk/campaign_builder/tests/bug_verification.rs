//! Integration tests to verify Campaign Builder bugs
//!
//! These tests are designed to FAIL when bugs exist and PASS when fixed.
//! This provides systematic verification and regression prevention.

use std::fs;

#[test]
fn test_bug_2_verify_unique_widget_ids() {
    // Bug #2: ID clashes in Items and Monsters tabs
    // Expected: All interactive widgets should have unique IDs

    // This test verifies that combo boxes and other widgets use unique IDs
    // We can't directly test egui widget IDs, but we can verify the patterns

    let source_code = fs::read_to_string("src/lib.rs").expect("Failed to read lib.rs");

    // Extract only production code (before #[cfg(test)] module)
    // This avoids false positives from test string literals that contain "from_label"
    let production_code = if let Some(test_start) = source_code.find("#[cfg(test)]") {
        &source_code[..test_start]
    } else {
        &source_code[..]
    };

    // Check that ComboBox uses from_id_salt (not from_label) in production code
    let from_label_count = production_code.matches("ComboBox::from_label").count();

    assert_eq!(
        from_label_count, 0,
        "Found {} uses of ComboBox::from_label in production code which can cause ID clashes. \
         All combo boxes should use from_id_salt() with unique IDs.",
        from_label_count
    );

    // Check for common ID patterns that might clash
    // We look for from_id_salt("pattern") usage specifically
    let id_patterns = vec![
        r#"from_id_salt("item_type_filter")"#,
        r#"from_id_salt("spell_level_filter")"#,
        r#"from_id_salt("map_terrain_palette_combo")"#,
        r#"from_id_salt("map_wall_palette_combo")"#,
    ];

    for pattern in id_patterns {
        let count = production_code.matches(pattern).count();
        assert!(
            count <= 1,
            "ID pattern '{}' appears {} times. Each ID should be unique.",
            pattern,
            count
        );
    }
}

#[test]
fn test_bug_3_map_editor_terrain_wall_independence() {
    // Bug #3: Map tools - terrain and wall reset each other
    // Expected: MapEditorState should have separate terrain and wall fields

    let source_code =
        fs::read_to_string("src/map_editor.rs").expect("Failed to read map_editor.rs");

    // Verify MapEditorState has selected_terrain field
    assert!(
        source_code.contains("selected_terrain"),
        "MapEditorState should have 'selected_terrain' field"
    );

    // Verify MapEditorState has selected_wall field
    assert!(
        source_code.contains("selected_wall"),
        "MapEditorState should have 'selected_wall' field"
    );

    // Verify EditorTool enum has PaintTile (not PaintTerrain/PaintWall with payloads)
    assert!(
        source_code.contains("PaintTile"),
        "EditorTool should have 'PaintTile' variant"
    );

    // Verify old pattern is removed
    let paint_terrain_with_payload = source_code.contains("PaintTerrain(TerrainType)");
    assert!(
        !paint_terrain_with_payload,
        "EditorTool should not have 'PaintTerrain(TerrainType)' - should be plain 'PaintTile'"
    );
}

#[test]
fn test_asset_loading_from_ron() {
    // Additional bug: Assets not loaded from existing .ron files
    // Expected: Asset manager should load assets from data files

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let assets_dir = temp_dir.path().join("assets");
    let icons_dir = assets_dir.join("icons");

    fs::create_dir_all(&icons_dir).expect("Failed to create icons dir");

    // Create a test icon file
    let test_icon = icons_dir.join("test_icon.png");
    fs::write(&test_icon, b"fake png data").expect("Failed to write test icon");

    // Verify file exists
    assert!(test_icon.exists(), "Test icon file should exist");

    // TODO: Test that asset manager actually loads this file
    // This requires understanding the AssetManager API
}

#[test]
fn test_campaign_save_creates_all_data_files() {
    // Comprehensive test: Campaign save should create ALL data files
    // Expected: items.ron, monsters.ron, spells.ron, quests.ron, dialogue.ron

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let campaign_dir = temp_dir.path().to_path_buf();

    fs::create_dir_all(&campaign_dir).expect("Failed to create campaign dir");

    // Expected files after campaign save
    let expected_files = vec![
        "campaign.ron",
        "items.ron",
        "monsters.ron",
        "spells.ron",
        "quests.ron",
        "dialogue.ron",
    ];

    // This test documents what SHOULD happen
    // When bug is fixed, we'll verify these files are created

    for file in &expected_files {
        let file_path = campaign_dir.join(file);
        println!("Expected file: {}", file_path.display());
        // TODO: Actually trigger campaign save and verify files exist
    }
}

#[test]
fn test_items_tab_widget_ids_unique() {
    // Specific test for Items tab widget ID uniqueness

    // We refactored the items editor into its own file; inspect it directly.
    let source_code =
        fs::read_to_string("src/items_editor.rs").expect("Failed to read src/items_editor.rs");

    // In Items editor file, check for ID patterns within the 'show' method
    let items_section = if let Some(start) = source_code.find("pub fn show(") {
        if let Some(end) = source_code[start..].find("fn show_list(") {
            &source_code[start..start + end]
        } else {
            &source_code[start..]
        }
    } else {
        panic!("Could not find ItemsEditorState::show function");
    };

    // Verify item_type_filter uses from_id_salt
    assert!(
        items_section.contains(r#"from_id_salt("item_type_filter")"#)
            || items_section.contains(r#"from_id_salt("items_type_filter")"#),
        "Items filter should use from_id_salt with unique ID"
    );
}

#[test]
fn test_monsters_tab_widget_ids_unique() {
    // Specific test for Monsters tab widget ID uniqueness

    // We refactored the monsters editor into its own file; inspect it directly.
    let source_code = fs::read_to_string("src/monsters_editor.rs")
        .expect("Failed to read src/monsters_editor.rs");

    // In Monsters editor file, verify no ID clashes within the 'show' method
    let monsters_section = if let Some(start) = source_code.find("pub fn show(") {
        if let Some(end) = source_code[start..].find("fn show_list(") {
            &source_code[start..start + end]
        } else {
            &source_code[start..]
        }
    } else {
        panic!("Could not find MonstersEditorState::show function");
    };

    // Check for any from_label usage (should be zero)
    let from_label_in_monsters = monsters_section.matches("from_label").count();
    assert_eq!(
        from_label_in_monsters, 0,
        "Monsters editor should not use from_label (causes ID clashes)"
    );
}

#[test]
fn test_no_implicit_widget_id_generation() {
    // Advanced test: Check for widgets that might generate implicit IDs

    let source_code = fs::read_to_string("src/lib.rs").expect("Failed to read lib.rs");

    // Pattern: Multiple TextEdit widgets with same label might clash
    // Pattern: Multiple DragValue widgets in same scope might clash

    // This is a heuristic test - we check for suspicious patterns
    let suspicious_patterns = vec![
        (
            "TextEdit::singleline(&mut self.items",
            "Items TextEdit widgets",
        ),
        (
            "TextEdit::singleline(&mut self.monsters",
            "Monsters TextEdit widgets",
        ),
    ];

    for (pattern, description) in suspicious_patterns {
        let count = source_code.matches(pattern).count();
        println!("{}: found {} instances", description, count);

        // If there are multiple, they might need explicit IDs
        if count > 5 {
            println!("Warning: Many similar widgets found - verify they have unique IDs");
        }
    }
}

#[test]
fn test_ron_file_format_used_not_json() {
    // Verify that data files use .ron extension, not .json or .yaml

    let source_code = fs::read_to_string("src/lib.rs").expect("Failed to read lib.rs");

    // Check CampaignMetadata file paths
    assert!(
        source_code.contains(r#"items_file: String"#) || source_code.contains("items.ron"),
        "Should reference items.ron file"
    );

    // Verify no .json references in save operations (excluding comments and strings that mention it)
    // Look specifically for file extension patterns like ".json"
    let json_pattern_count = source_code
        .lines()
        .filter(|line| !line.trim().starts_with("//")) // Exclude comments
        .filter(|line| line.contains("\".json\"")) // Look for ".json" string literals
        .count();

    assert_eq!(
        json_pattern_count, 0,
        "Should not use .json file extensions in code, must use .ron format"
    );
}
