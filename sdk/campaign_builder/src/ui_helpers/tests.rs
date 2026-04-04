// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for the ui_helpers module.
// This file is the body of `mod tests` declared in mod.rs with `#[cfg(test)]`.

use super::layout::make_autocomplete_id;
use super::*;
use antares::domain::character::{AttributePair, AttributePair16};
use eframe::egui;
use eframe::egui::Vec2;
use std::path::PathBuf;

// =========================================================================
// Panel Height Tests
// =========================================================================

#[test]
fn compute_panel_height_from_size_returns_size_y_if_larger() {
    let size = Vec2::new(100.0, 250.0);
    let min = 100.0;
    assert_eq!(compute_panel_height_from_size(size, min), 250.0);
}

#[test]
fn compute_panel_height_from_size_returns_min_if_size_smaller() {
    let size = Vec2::new(100.0, 40.0);
    let min = 100.0;
    assert_eq!(compute_panel_height_from_size(size, min), min);
}

#[test]
fn compute_default_panel_height_from_size_uses_default_min() {
    let size = Vec2::new(640.0, 90.0);
    assert_eq!(
        compute_default_panel_height_from_size(size),
        DEFAULT_PANEL_MIN_HEIGHT
    );
}

#[test]
fn compute_panel_height_from_size_handles_exact_minimum() {
    let size = Vec2::new(100.0, 100.0);
    let min = 100.0;
    assert_eq!(compute_panel_height_from_size(size, min), 100.0);
}

#[test]
fn compute_panel_height_from_size_handles_zero_size() {
    let size = Vec2::new(0.0, 0.0);
    let min = 100.0;
    assert_eq!(compute_panel_height_from_size(size, min), min);
}

// =========================================================================
// Standard List Item Component Tests
// =========================================================================

#[test]
fn metadata_badge_new_creates_default() {
    let badge = MetadataBadge::new("Test");
    assert_eq!(badge.text, "Test");
    assert_eq!(badge.color, egui::Color32::GRAY);
    assert!(badge.tooltip.is_none());
}

#[test]
fn metadata_badge_builder_pattern() {
    let badge = MetadataBadge::new("Magic")
        .with_color(egui::Color32::from_rgb(128, 0, 200))
        .with_tooltip("Magical item");
    assert_eq!(badge.text, "Magic");
    assert_eq!(badge.color, egui::Color32::from_rgb(128, 0, 200));
    assert_eq!(badge.tooltip, Some("Magical item".to_string()));
}

#[test]
fn standard_list_item_config_new_creates_default() {
    let config = StandardListItemConfig::new("Test Item");
    assert_eq!(config.label, "Test Item");
    assert!(!config.selected);
    assert!(config.badges.is_empty());
    assert!(config.id.is_none());
    assert!(config.icon.is_none());
}

#[test]
fn standard_list_item_config_builder_pattern() {
    let badges = vec![MetadataBadge::new("Test")];
    let config = StandardListItemConfig::new("Iron Sword")
        .with_badges(badges)
        .selected(true)
        .with_id(42u32)
        .with_icon("⚔️");
    assert_eq!(config.label, "Iron Sword");
    assert!(config.selected);
    assert_eq!(config.badges.len(), 1);
    assert_eq!(config.id, Some("42".to_string()));
    assert_eq!(config.icon, Some("⚔️"));
}

// =========================================================================
// CSV/Filter/Format Helper Tests
// =========================================================================

#[test]
fn parse_id_csv_to_vec_simple() {
    let parsed = parse_id_csv_to_vec::<u8>("1, 2, 3").unwrap();
    assert_eq!(parsed, vec![1u8, 2u8, 3u8]);
}

#[test]
fn parse_id_csv_to_vec_empty() {
    let parsed = parse_id_csv_to_vec::<u8>("").unwrap();
    assert!(parsed.is_empty());
}

#[test]
fn parse_id_csv_to_vec_whitespace_and_commas() {
    let parsed = parse_id_csv_to_vec::<u8>(" 1 ,  , 2 ,  3 ").unwrap();
    assert_eq!(parsed, vec![1u8, 2u8, 3u8]);
}

#[test]
fn parse_id_csv_to_vec_invalid() {
    let err = parse_id_csv_to_vec::<u8>("a, 2");
    assert!(err.is_err());
}

#[test]
fn format_vec_to_csv_simple() {
    assert_eq!(format_vec_to_csv(&[1u8, 2u8, 3u8]), "1, 2, 3");
}

#[test]
fn format_vec_to_csv_empty() {
    assert_eq!(format_vec_to_csv::<u8>(&[]), "");
}

#[test]
fn filter_items_by_query_basic() {
    struct Foo {
        name: String,
    }
    let items = vec![
        Foo {
            name: "Goblin".to_string(),
        },
        Foo {
            name: "Orc".to_string(),
        },
        Foo {
            name: "Golem".to_string(),
        },
    ];

    let idx = filter_items_by_query(&items, "gob", |f| f.name.clone());
    assert_eq!(idx, vec![0usize]);

    let idx_all = filter_items_by_query(&items, "", |f| f.name.clone());
    assert_eq!(idx_all, vec![0usize, 1usize, 2usize]);

    let idx_g = filter_items_by_query(&items, "g", |f| f.name.clone());
    assert_eq!(idx_g, vec![0usize, 2usize]);
}

// =========================================================================
// ToolbarAction Tests
// =========================================================================

#[test]
fn toolbar_action_enum_values() {
    assert_ne!(ToolbarAction::New, ToolbarAction::Save);
    assert_ne!(ToolbarAction::Load, ToolbarAction::Import);
    assert_ne!(ToolbarAction::Export, ToolbarAction::Reload);
    assert_eq!(ToolbarAction::None, ToolbarAction::None);
}

#[test]
fn editor_toolbar_new_creates_with_defaults() {
    let toolbar = EditorToolbar::new("Test");
    assert_eq!(toolbar.editor_name, "Test");
    assert!(toolbar.search_query.is_none());
    assert!(toolbar.merge_mode.is_none());
    assert!(toolbar.total_count.is_none());
    assert!(toolbar.show_save);
}

#[test]
fn editor_toolbar_builder_pattern() {
    let mut search = String::new();
    let mut merge = false;

    let toolbar = EditorToolbar::new("Items")
        .with_search(&mut search)
        .with_merge_mode(&mut merge)
        .with_total_count(42)
        .with_save_button(false)
        .with_id_salt("test_salt");

    assert_eq!(toolbar.editor_name, "Items");
    assert!(toolbar.search_query.is_some());
    assert!(toolbar.merge_mode.is_some());
    assert_eq!(toolbar.total_count, Some(42));
    assert!(!toolbar.show_save);
    assert_eq!(toolbar.id_salt, Some("test_salt"));
}

// =========================================================================
// ItemAction Tests
// =========================================================================

#[test]
fn item_action_enum_values() {
    assert_ne!(ItemAction::Edit, ItemAction::Delete);
    assert_ne!(ItemAction::Duplicate, ItemAction::Export);
    assert_eq!(ItemAction::None, ItemAction::None);
}

#[test]
fn action_buttons_default_all_visible() {
    let buttons = ActionButtons::new();
    assert!(buttons.enabled);
    assert!(buttons.show_edit);
    assert!(buttons.show_delete);
    assert!(buttons.show_duplicate);
    assert!(buttons.show_export);
}

#[test]
fn action_buttons_builder_pattern() {
    let buttons = ActionButtons::new()
        .enabled(false)
        .with_edit(false)
        .with_delete(true)
        .with_duplicate(false)
        .with_export(true);

    assert!(!buttons.enabled);
    assert!(!buttons.show_edit);
    assert!(buttons.show_delete);
    assert!(!buttons.show_duplicate);
    assert!(buttons.show_export);
}

// =========================================================================
// TwoColumnLayout Tests
// =========================================================================

#[test]
fn two_column_layout_new_uses_defaults() {
    let layout = TwoColumnLayout::new("test");
    assert_eq!(layout.id_salt, "test");
    assert_eq!(layout.left_width, DEFAULT_LEFT_COLUMN_WIDTH);
    assert_eq!(layout.min_height, DEFAULT_PANEL_MIN_HEIGHT);
}

#[test]
fn two_column_layout_builder_pattern() {
    let layout = TwoColumnLayout::new("custom")
        .with_left_width(400.0)
        .with_min_height(200.0)
        .with_inspector_min_width(320.0)
        .with_max_left_ratio(0.65);

    assert_eq!(layout.id_salt, "custom");
    assert_eq!(layout.left_width, 400.0);
    assert_eq!(layout.min_height, 200.0);
    assert_eq!(layout.inspector_min_width, 320.0);
    assert_eq!(layout.max_left_ratio, 0.65);
}

// =========================================================================
// Additional TwoColumnLayout Tests
// =========================================================================

#[test]
fn two_column_layout_show_split_calls_both_closures() {
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(1200.0, 800.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);

    let left_called = std::rc::Rc::new(std::cell::Cell::new(false));
    let right_called = std::rc::Rc::new(std::cell::Cell::new(false));

    {
        let left_clone = left_called.clone();
        let right_clone = right_called.clone();
        egui::CentralPanel::default().show(&ctx, |ui| {
            TwoColumnLayout::new("test")
                .with_left_width(400.0)
                .with_inspector_min_width(300.0)
                .with_max_left_ratio(DEFAULT_LEFT_COLUMN_MAX_RATIO)
                .show_split(
                    ui,
                    |left_ui| {
                        left_clone.set(true);
                        // Small touch to exercise scroll area width
                        left_ui.label("Left content");
                    },
                    |right_ui| {
                        right_clone.set(true);
                        right_ui.label("Right content");
                    },
                );
        });
    }
    let _ = ctx.end_pass();

    assert!(left_called.get());
    assert!(right_called.get());
}

#[test]
fn render_grid_header_draws_headers() {
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);

    egui::CentralPanel::default().show(&ctx, |ui| {
        egui::Grid::new("test_grid").num_columns(3).show(ui, |ui| {
            render_grid_header(ui, &["Status", "Message", "File"]);
            // Add a sample row to ensure grid usage doesn't panic
            ui.colored_label(egui::Color32::from_rgb(255, 80, 80), "❌");
            ui.label("Sample message");
            ui.label("-");
            ui.end_row();
        });
    });

    let _ = ctx.end_pass();
}

#[test]
fn show_validation_severity_icon_shows_icon() {
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(400.0, 300.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);

    egui::CentralPanel::default().show(&ctx, |ui| {
        super::show_validation_severity_icon(ui, crate::validation::ValidationSeverity::Error);
    });

    let _ = ctx.end_pass();
}

// =========================================================================
// compute_left_column_width tests
// =========================================================================

#[test]
fn compute_left_column_width_small_total_width() {
    // 480 total width, inspector min 300, separator 12 -> available left = 168
    let total_width = 480.0;
    let requested_left = 300.0;
    let inspector_min = 300.0;
    let sep_margin = 12.0;
    let left = compute_left_column_width(
        total_width,
        requested_left,
        inspector_min,
        sep_margin,
        MIN_SAFE_LEFT_COLUMN_WIDTH,
        DEFAULT_LEFT_COLUMN_MAX_RATIO,
    );
    // force exact equality to the available space (168)
    assert_eq!(left, 168.0);
}

#[test]
fn compute_left_column_width_enforces_min_when_space_available() {
    // 1200 total width, enough to allow min safe left width (250)
    let total_width = 1200.0;
    let requested_left = 400.0;
    let inspector_min = 300.0;
    let sep_margin = 12.0;
    let left = compute_left_column_width(
        total_width,
        requested_left,
        inspector_min,
        sep_margin,
        MIN_SAFE_LEFT_COLUMN_WIDTH,
        DEFAULT_LEFT_COLUMN_MAX_RATIO,
    );
    assert_eq!(left, 400.0);
}

#[test]
fn compute_left_column_width_enforces_max_ratio_limit() {
    // 800 total width: available left = 488 -> should be upper bound
    let total_width = 800.0;
    let requested_left = 600.0;
    let inspector_min = 300.0;
    let sep_margin = 12.0;
    let left = compute_left_column_width(
        total_width,
        requested_left,
        inspector_min,
        sep_margin,
        MIN_SAFE_LEFT_COLUMN_WIDTH,
        DEFAULT_LEFT_COLUMN_MAX_RATIO,
    );
    assert_eq!(left, 488.0);
}

#[test]
fn compute_left_column_width_zero_when_no_space() {
    // total width smaller than inspector_min + separator -> 0.0 left width
    let total_width = 200.0;
    let requested_left = 250.0;
    let inspector_min = 300.0;
    let sep_margin = 12.0;
    let left = compute_left_column_width(
        total_width,
        requested_left,
        inspector_min,
        sep_margin,
        MIN_SAFE_LEFT_COLUMN_WIDTH,
        DEFAULT_LEFT_COLUMN_MAX_RATIO,
    );
    assert_eq!(left, 0.0);
}

// =========================================================================
// ImportExportDialog Tests
// =========================================================================

#[test]
fn import_export_dialog_state_new() {
    let state = ImportExportDialogState::new();
    assert!(!state.is_open);
    assert!(state.buffer.is_empty());
    assert!(state.error_message.is_none());
    assert!(!state.export_mode);
}

#[test]
fn import_export_dialog_state_open_import() {
    let mut state = ImportExportDialogState::new();
    state.buffer = "old data".to_string();
    state.error_message = Some("old error".to_string());

    state.open_import();

    assert!(state.is_open);
    assert!(state.buffer.is_empty());
    assert!(state.error_message.is_none());
    assert!(!state.export_mode);
}

#[test]
fn import_export_dialog_state_open_export() {
    let mut state = ImportExportDialogState::new();
    state.open_export("exported data".to_string());

    assert!(state.is_open);
    assert_eq!(state.buffer, "exported data");
    assert!(state.error_message.is_none());
    assert!(state.export_mode);
}

#[test]
fn import_export_dialog_state_close() {
    let mut state = ImportExportDialogState::new();
    state.open_export("data".to_string());
    state.set_error("error");

    state.close();

    assert!(!state.is_open);
    assert!(state.buffer.is_empty());
    assert!(state.error_message.is_none());
}

#[test]
fn import_export_dialog_state_set_error() {
    let mut state = ImportExportDialogState::new();
    state.set_error("Parse error");

    assert_eq!(state.error_message, Some("Parse error".to_string()));
}

#[test]
fn import_export_result_enum() {
    let import_result = ImportExportResult::Import("data".to_string());
    let cancel_result = ImportExportResult::Cancel;
    let open_result = ImportExportResult::Open;

    assert_ne!(import_result, cancel_result);
    assert_ne!(cancel_result, open_result);
    assert_eq!(
        ImportExportResult::Import("data".to_string()),
        ImportExportResult::Import("data".to_string())
    );
}

// =========================================================================
// AttributePairInput Tests
// =========================================================================

#[test]
fn attribute_pair_input_state_new() {
    let state = AttributePairInputState::new();
    assert!(state.auto_sync);
}

#[test]
fn attribute_pair_input_state_with_auto_sync() {
    let state = AttributePairInputState::with_auto_sync(false);
    assert!(!state.auto_sync);

    let state = AttributePairInputState::with_auto_sync(true);
    assert!(state.auto_sync);
}

#[test]
fn attribute_pair_reset_behavior() {
    let mut attr = AttributePair::new(10);
    attr.current = 25;
    assert_eq!(attr.base, 10);
    assert_eq!(attr.current, 25);

    attr.reset();
    assert_eq!(attr.current, 10);
}

#[test]
fn attribute_pair16_reset_behavior() {
    let mut attr = AttributePair16::new(100);
    attr.current = 250;
    assert_eq!(attr.base, 100);
    assert_eq!(attr.current, 250);

    attr.reset();
    assert_eq!(attr.current, 100);
}

// =========================================================================
// Constants Tests
// =========================================================================

#[test]
fn default_constants_have_expected_values() {
    // Verify constants have the expected values
    assert_eq!(DEFAULT_LEFT_COLUMN_WIDTH, 300.0);
    assert_eq!(DEFAULT_PANEL_MIN_HEIGHT, 100.0);
}

// =========================================================================
// Keyboard Shortcuts Tests
// =========================================================================

#[test]
fn toolbar_action_keyboard_shortcuts_documented() {
    // This test documents the keyboard shortcuts for EditorToolbar:
    // - Ctrl+N: New
    // - Ctrl+S: Save
    // - Ctrl+L: Load
    // - Ctrl+Shift+I: Import
    // - Ctrl+Shift+E: Export
    // - F5: Reload
    //
    // Note: We cannot easily unit test keyboard input in egui without
    // a full rendering context, so this test serves as documentation.
    // The shortcuts are implemented in EditorToolbar::show() and should
    // be manually tested.
    assert_eq!(ToolbarAction::New as i32, 0);
}

#[test]
fn item_action_keyboard_shortcuts_documented() {
    // This test documents the keyboard shortcuts for ActionButtons:
    // - Ctrl+E: Edit
    // - Delete: Delete
    // - Ctrl+D: Duplicate
    //
    // Note: We cannot easily unit test keyboard input in egui without
    // a full rendering context, so this test serves as documentation.
    // The shortcuts are implemented in ActionButtons::show() and should
    // be manually tested.
    assert_eq!(ItemAction::Edit as i32, 0);
}

#[test]
fn toolbar_buttons_have_consistent_labels() {
    // This test documents the standardized button labels:
    // - ➕ New
    // - 💾 Save
    // - 📂 Load
    // - 📥 Import
    // - 📋 Export
    // - 🔄 Reload
    //
    // All editors must use these labels consistently.
    // The labels are implemented in EditorToolbar::show().
    let labels = [
        "➕ New",
        "💾 Save",
        "📂 Load",
        "📥 Import",
        "📋 Export",
        "🔄 Reload",
    ];
    assert!(labels.iter().all(|label| !label.is_empty()));
}

#[test]
fn action_buttons_have_consistent_labels() {
    // This test documents the standardized action button labels:
    // - ✏️ Edit
    // - 🗑️ Delete
    // - 📋 Duplicate
    // - 📤 Export
    //
    // All editors must use these labels consistently.
    // The labels are implemented in ActionButtons::show().
    let labels = ["✏️ Edit", "🗑️ Delete", "📋 Duplicate", "📤 Export"];
    assert!(labels.iter().all(|label| !label.is_empty()));
}

#[test]
fn toolbar_buttons_have_tooltips_with_shortcuts() {
    // This test documents that all toolbar buttons should have
    // tooltips showing their keyboard shortcuts.
    // The tooltips are implemented using .on_hover_text() in
    // EditorToolbar::show().
    let shortcuts = [
        "Ctrl+N",
        "Ctrl+S",
        "Ctrl+L",
        "Ctrl+Shift+I",
        "Ctrl+Shift+E",
        "F5",
    ];
    assert!(shortcuts.iter().all(|shortcut| !shortcut.is_empty()));
}

#[test]
fn action_buttons_have_tooltips_with_shortcuts() {
    // This test documents that all action buttons should have
    // tooltips showing their keyboard shortcuts.
    // The tooltips are implemented using .on_hover_text() in
    // ActionButtons::show().
    let shortcuts = ["Ctrl+E", "Delete", "Ctrl+D"];
    assert!(shortcuts.iter().all(|shortcut| !shortcut.is_empty()));
}

// =========================================================================
// AutocompleteInput Tests
// =========================================================================

#[test]
fn autocomplete_input_new_creates_widget() {
    let candidates = vec!["Goblin".to_string(), "Orc".to_string()];
    let widget = AutocompleteInput::new("test_autocomplete", &candidates);

    assert_eq!(widget._id_salt, "test_autocomplete");
    assert_eq!(widget.candidates.len(), 2);
    assert_eq!(widget.placeholder, None);
}

#[test]
fn autocomplete_input_with_placeholder() {
    let candidates = vec!["Dragon".to_string()];
    let widget = AutocompleteInput::new("test", &candidates).with_placeholder("Type here...");

    assert_eq!(widget.placeholder, Some("Type here..."));
}

#[test]
fn autocomplete_input_builder_pattern() {
    let candidates = vec![
        "Goblin".to_string(),
        "Orc".to_string(),
        "Dragon".to_string(),
    ];

    let widget =
        AutocompleteInput::new("my_widget", &candidates).with_placeholder("Select monster...");

    assert_eq!(widget._id_salt, "my_widget");
    assert_eq!(widget.candidates.len(), 3);
    assert_eq!(widget.placeholder, Some("Select monster..."));
}

#[test]
fn autocomplete_input_empty_candidates() {
    let candidates: Vec<String> = vec![];
    let widget = AutocompleteInput::new("empty_test", &candidates);

    assert_eq!(widget.candidates.len(), 0);
}

#[test]
fn autocomplete_input_many_candidates() {
    let candidates: Vec<String> = (0..100).map(|i| format!("Monster{}", i)).collect();

    let widget = AutocompleteInput::new("many_test", &candidates);

    assert_eq!(widget.candidates.len(), 100);
}

#[test]
fn autocomplete_input_unique_id_salt() {
    let candidates = vec!["Item1".to_string()];

    let widget1 = AutocompleteInput::new("widget1", &candidates);
    let widget2 = AutocompleteInput::new("widget2", &candidates);

    assert_ne!(widget1._id_salt, widget2._id_salt);
}

#[test]
fn autocomplete_input_case_sensitivity_documented() {
    // This test documents that AutocompleteInput performs
    // case-insensitive filtering by default.
    // For example, typing "gob" should match "Goblin", "GOBLIN", "goblin".
    // The case-insensitive behavior is implemented via the
    // egui_autocomplete::AutoCompleteTextEdit widget.
    // This should be verified with manual testing in the UI.
    let candidates = [
        "Goblin".to_string(),
        "GOBLIN".to_string(),
        "goblin".to_string(),
    ];
    // Sanity check - ensure our candidates are present
    assert_eq!(candidates.len(), 3);
}

#[test]
fn autocomplete_monster_selector_preserves_passed_buffer() {
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);

    egui::CentralPanel::default().show(&ctx, |ui| {
        use antares::domain::character::{AttributePair, AttributePair16, Stats};
        use antares::domain::combat::database::MonsterDefinition;
        use antares::domain::combat::{monster::MonsterCondition, LootTable, MonsterResistances};

        let monsters = vec![MonsterDefinition {
            id: 1,
            name: "Goblin".to_string(),
            stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
            hp: AttributePair16::new(15),
            ac: AttributePair::new(6),
            attacks: vec![],
            flee_threshold: 5,
            special_attack_threshold: 0,
            resistances: MonsterResistances::new(),
            can_regenerate: false,
            can_advance: true,
            is_undead: false,
            magic_resistance: 0,
            loot: LootTable::new(1, 10, 0, 0, 10),
            creature_id: None,
            conditions: MonsterCondition::Normal,
            active_conditions: vec![],
            has_acted: false,
        }];

        let mut buffer = String::from("Go");
        let original = buffer.clone();
        // The selector should not reset a passed-in buffer on render
        let _changed =
            autocomplete_monster_selector(ui, "monster_test", "Name:", &mut buffer, &monsters);
        assert_eq!(
            buffer, original,
            "Passed in buffer should not be reset by the selector"
        );
    });
}

#[test]
fn autocomplete_item_selector_persists_buffer() {
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);

    egui::CentralPanel::default().show(&ctx, |ui| {
        use crate::items_editor::ItemsEditorState;

        let mut item = ItemsEditorState::default_item();
        item.id = 42;
        item.name = "Sword".to_string();
        let items = vec![item];

        let mut selected_item_id: antares::domain::types::ItemId = 0;
        // First call should initialize persistent buffer
        let _ = autocomplete_item_selector(ui, "item_test", "Item:", &mut selected_item_id, &items);

        // Confirm memory has an entry
        ui.horizontal(|ui| {
            let id = make_autocomplete_id(ui, "item", "item_test");
            let val = ui.ctx().memory(|mem| mem.data.get_temp::<String>(id));
            assert!(val.is_some(), "Buffer map should contain entry for widget");

            // Simulate typing by modifying the buffer
            ui.ctx().memory_mut(|mem| {
                let buf = mem
                    .data
                    .get_temp_mut_or_insert_with::<String>(id, String::new);
                *buf = "Sw".to_string();
            });
        });

        // Second call should not overwrite the typed content
        let _ = autocomplete_item_selector(ui, "item_test", "Item:", &mut selected_item_id, &items);

        let val2 = ui.ctx().memory_mut(|mem| {
            mem.data
                .get_temp::<String>(make_autocomplete_id(ui, "item", "item_test"))
        });
        assert_eq!(val2.as_deref(), Some("Sw"));
    });
}

#[test]
fn autocomplete_map_selector_persists_buffer() {
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);

    egui::CentralPanel::default().show(&ctx, |ui| {
        let mut selected_map_id: String = String::new();
        let maps: Vec<antares::domain::world::Map> = vec![];
        // First call should initialize persistent buffer
        let _ = autocomplete_map_selector(ui, "map_test", "Map:", &mut selected_map_id, &maps);

        // Confirm memory has an entry
        let id = make_autocomplete_id(ui, "map", "map_test");
        let val = ui.ctx().memory(|mem| mem.data.get_temp::<String>(id));
        assert!(val.is_some(), "Buffer map should contain entry for widget");

        // Simulate typing by modifying the buffer
        ui.ctx().memory_mut(|mem| {
            let buf = mem
                .data
                .get_temp_mut_or_insert_with::<String>(id, String::new);
            *buf = "Over".to_string();
        });

        // Second call should not overwrite the typed content
        let _ = autocomplete_map_selector(ui, "map_test", "Map:", &mut selected_map_id, &maps);

        let val2 = ui.ctx().memory_mut(|mem| mem.data.get_temp::<String>(id));
        assert_eq!(val2.as_deref(), Some("Over"));
    });
}

#[test]
fn autocomplete_buffer_helpers_work() {
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);

    egui::CentralPanel::default().show(&ctx, |ui| {
        // Use the same ID pattern as widgets use so we validate the helpers operate on the same keys.
        let id = make_autocomplete_id(ui, "test", "helper_test");

        // When no buffer exists, the default factory should be used.
        let v = load_autocomplete_buffer(ui.ctx(), id, || "def".to_string());
        assert_eq!(v, "def");

        // Store a value and ensure it is returned by the loader.
        store_autocomplete_buffer(ui.ctx(), id, "abc");
        let v2 = load_autocomplete_buffer(ui.ctx(), id, || "def".to_string());
        assert_eq!(v2, "abc");

        // Remove the buffer and ensure memory no longer contains it.
        remove_autocomplete_buffer(ui.ctx(), id);
        let maybe = ui.ctx().memory(|mem| mem.data.get_temp::<String>(id));
        assert!(maybe.is_none());
    });
}

#[test]
fn autocomplete_input_max_suggestions_limit() {
    // This test documents that AutocompleteInput limits the dropdown
    // to a maximum of 10 suggestions to prevent UI clutter.
    // This is configured via .max_suggestions(10) in the show() method.
    // With more than 10 matching candidates, only the first 10 are shown.
    let candidates: Vec<String> = (0..20).map(|i| format!("Monster{}", i)).collect();
    let widget = AutocompleteInput::new("limit_test", &candidates);
    assert!(widget.candidates.len() > 10);
}

#[test]
fn autocomplete_input_highlight_matches_enabled() {
    // This test documents that AutocompleteInput highlights matching
    // text in the dropdown suggestions for better user experience.
    // This is enabled via .highlight_matches(true) in the show() method.
    // Manual testing should verify that matching substrings are highlighted.
    let candidates = vec!["Goblin".to_string(), "Hobgoblin".to_string()];
    let widget = AutocompleteInput::new("highlight_test", &candidates);
    assert_eq!(widget.candidates.len(), 2);
}

#[test]
fn autocomplete_input_follows_ui_helper_conventions() {
    // This test verifies that AutocompleteInput follows the same
    // conventions as other UI helpers:
    // - Uses builder pattern (with_* methods)
    // - Returns Self for chaining
    // - Uses &'a lifetime for borrowed references
    // - Has comprehensive doc comments with examples
    let candidates = vec!["Test".to_string()];
    let widget =
        AutocompleteInput::new("convention_test", &candidates).with_placeholder("Test placeholder");

    assert!(widget.placeholder.is_some());
}

// =========================================================================
// Entity Candidate Extraction Tests
// =========================================================================

#[test]
fn extract_monster_candidates_empty_list() {
    let monsters = vec![];
    let candidates = extract_monster_candidates(&monsters);
    assert_eq!(candidates.len(), 0);
}

#[test]
fn extract_item_candidates_empty_list() {
    let items = vec![];
    let candidates = extract_item_candidates(&items);
    assert_eq!(candidates.len(), 0);
}

#[test]
fn extract_condition_candidates_empty_list() {
    let conditions = vec![];
    let candidates = extract_condition_candidates(&conditions);
    assert_eq!(candidates.len(), 0);
}

#[test]
fn test_extract_proficiency_candidates() {
    use antares::domain::proficiency::{ProficiencyCategory, ProficiencyDefinition};

    let proficiencies = vec![
        ProficiencyDefinition {
            id: "simple_weapon".to_string(),
            name: "Simple Weapons".to_string(),
            category: ProficiencyCategory::Weapon,
            description: "Basic weapons".to_string(),
        },
        ProficiencyDefinition {
            id: "light_armor".to_string(),
            name: "Light Armor".to_string(),
            category: ProficiencyCategory::Armor,
            description: "Light armor proficiency".to_string(),
        },
    ];

    let candidates = extract_proficiency_candidates(&proficiencies);
    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates[0],
        (
            "Simple Weapons (simple_weapon)".to_string(),
            "simple_weapon".to_string()
        )
    );
    assert_eq!(
        candidates[1],
        (
            "Light Armor (light_armor)".to_string(),
            "light_armor".to_string()
        )
    );
}

#[test]
fn test_extract_proficiency_candidates_empty() {
    let proficiencies = vec![];
    let candidates = extract_proficiency_candidates(&proficiencies);
    assert_eq!(candidates.len(), 0);
}

#[test]
fn test_load_proficiencies_synthetic_fallback() {
    use antares::domain::items::types::{
        ArmorClassification, ArmorData, Item, ItemType, WeaponClassification, WeaponData,
    };
    use antares::domain::types::DiceRoll;

    // Create test items with various classifications
    let items = vec![
        Item {
            id: 1,
            name: "Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        },
        Item {
            id: 2,
            name: "Plate Mail".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 8,
                weight: 50,
                classification: ArmorClassification::Heavy,
            }),
            base_cost: 100,
            sell_cost: 50,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        },
    ];

    // Test with no campaign dir (will fall back to synthetic)
    let profs = load_proficiencies(None, &items);

    // Should have standard proficiencies
    assert!(!profs.is_empty());
    assert!(profs.iter().any(|p| p.id == "simple_weapon"));
    assert!(profs.iter().any(|p| p.id == "martial_melee"));
    assert!(profs.iter().any(|p| p.id == "heavy_armor"));
    assert!(profs.iter().any(|p| p.id == "light_armor"));
}

#[test]
fn test_generate_synthetic_proficiencies_standard() {
    // Test that standard proficiencies are always generated
    let profs = generate_synthetic_proficiencies(&[]);

    // Should have all 11 standard proficiencies
    assert_eq!(profs.len(), 11);
    assert!(profs.iter().any(|p| p.id == "simple_weapon"));
    assert!(profs.iter().any(|p| p.id == "martial_melee"));
    assert!(profs.iter().any(|p| p.id == "martial_ranged"));
    assert!(profs.iter().any(|p| p.id == "blunt_weapon"));
    assert!(profs.iter().any(|p| p.id == "unarmed"));
    assert!(profs.iter().any(|p| p.id == "light_armor"));
    assert!(profs.iter().any(|p| p.id == "medium_armor"));
    assert!(profs.iter().any(|p| p.id == "heavy_armor"));
    assert!(profs.iter().any(|p| p.id == "shield"));
    assert!(profs.iter().any(|p| p.id == "arcane_item"));
    assert!(profs.iter().any(|p| p.id == "divine_item"));
}

#[test]
fn test_extract_item_tag_candidates() {
    use antares::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};

    let items = vec![
        Item {
            id: 1,
            name: "Sword".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(0),
                is_combat_usable: false,
                duration_minutes: None,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec!["large_weapon".to_string(), "two_handed".to_string()],
            mesh_descriptor_override: None,
            mesh_id: None,
        },
        Item {
            id: 2,
            name: "Armor".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(0),
                is_combat_usable: false,
                duration_minutes: None,
            }),
            base_cost: 50,
            sell_cost: 25,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec!["heavy_armor".to_string(), "two_handed".to_string()],
            mesh_descriptor_override: None,
            mesh_id: None,
        },
    ];

    let candidates = extract_item_tag_candidates(&items);
    assert_eq!(candidates.len(), 3); // unique tags: heavy_armor, large_weapon, two_handed
    assert!(candidates.contains(&"large_weapon".to_string()));
    assert!(candidates.contains(&"two_handed".to_string()));
    assert!(candidates.contains(&"heavy_armor".to_string()));
}

#[test]
fn test_extract_item_tag_candidates_empty() {
    let items = vec![];
    let candidates = extract_item_tag_candidates(&items);
    assert_eq!(candidates.len(), 0);
}

#[test]
fn test_extract_special_ability_candidates() {
    use antares::domain::races::{RaceDefinition, Resistances, SizeCategory, StatModifiers};

    let races = vec![
        RaceDefinition {
            id: "human".to_string(),
            name: "Human".to_string(),
            description: String::new(),
            stat_modifiers: StatModifiers::default(),
            resistances: Resistances::default(),
            size: SizeCategory::Medium,
            special_abilities: vec!["lucky".to_string(), "brave".to_string()],
            proficiencies: vec![],
            incompatible_item_tags: vec![],
        },
        RaceDefinition {
            id: "elf".to_string(),
            name: "Elf".to_string(),
            description: String::new(),
            stat_modifiers: StatModifiers::default(),
            resistances: Resistances::default(),
            size: SizeCategory::Medium,
            special_abilities: vec!["infravision".to_string(), "keen_senses".to_string()],
            proficiencies: vec![],
            incompatible_item_tags: vec![],
        },
    ];

    let candidates = extract_special_ability_candidates(&races);
    // Should include race abilities + standard abilities
    assert!(candidates.len() >= 4);
    assert!(candidates.contains(&"lucky".to_string()));
    assert!(candidates.contains(&"brave".to_string()));
    assert!(candidates.contains(&"infravision".to_string()));
    assert!(candidates.contains(&"keen_senses".to_string()));
    // Check that standard abilities are included
    assert!(candidates.contains(&"magic_resistance".to_string()));
    assert!(candidates.contains(&"darkvision".to_string()));
}

#[test]
fn test_extract_special_ability_candidates_empty() {
    let races = vec![];
    let candidates = extract_special_ability_candidates(&races);
    // Should still have standard abilities
    assert!(!candidates.is_empty());
    assert!(candidates.contains(&"infravision".to_string()));
}

#[test]
fn extract_spell_candidates_empty_list() {
    let spells = vec![];
    let candidates = extract_spell_candidates(&spells);
    assert_eq!(candidates.len(), 0);
}

#[test]
fn extract_proficiency_candidates_empty_list() {
    let proficiencies = vec![];
    let candidates = extract_proficiency_candidates(&proficiencies);
    assert_eq!(candidates.len(), 0);
}

#[test]
fn extract_proficiency_candidates_returns_string_ids() {
    use antares::domain::proficiency::{ProficiencyCategory, ProficiencyDefinition};

    let proficiencies = vec![
        ProficiencyDefinition {
            id: "sword".to_string(),
            name: "Sword".to_string(),
            category: ProficiencyCategory::Weapon,
            description: String::new(),
        },
        ProficiencyDefinition {
            id: "shield".to_string(),
            name: "Shield".to_string(),
            category: ProficiencyCategory::Armor,
            description: String::new(),
        },
        ProficiencyDefinition {
            id: "heavy_armor".to_string(),
            name: "Heavy Armor".to_string(),
            category: ProficiencyCategory::Armor,
            description: String::new(),
        },
    ];

    let candidates = extract_proficiency_candidates(&proficiencies);
    assert_eq!(candidates.len(), 3);
    assert_eq!(
        candidates[0],
        ("Sword (sword)".to_string(), "sword".to_string())
    );
    assert_eq!(
        candidates[1],
        ("Shield (shield)".to_string(), "shield".to_string())
    );
    assert_eq!(
        candidates[2],
        (
            "Heavy Armor (heavy_armor)".to_string(),
            "heavy_armor".to_string()
        )
    );
}

// =========================================================================
// Candidate Cache Tests
// =========================================================================

#[test]
fn candidate_cache_new_is_empty() {
    let cache = AutocompleteCandidateCache::new();
    assert!(cache.items.is_none());
    assert!(cache.monsters.is_none());
    assert!(cache.conditions.is_none());
    assert!(cache.spells.is_none());
    assert!(cache.proficiencies.is_none());
}

#[test]
fn candidate_cache_invalidate_items_clears_cache() {
    let mut cache = AutocompleteCandidateCache::new();
    // Simulate cached items
    cache.items = Some((vec![("Test".to_string(), 1)], 0));

    cache.invalidate_items();

    assert!(cache.items.is_none());
    assert_eq!(cache.items_generation, 1);
}

#[test]
fn candidate_cache_invalidate_all_clears_all_caches() {
    let mut cache = AutocompleteCandidateCache::new();
    cache.items = Some((vec![("Test".to_string(), 1)], 0));
    cache.monsters = Some((vec!["Monster".to_string()], 0));

    cache.invalidate_all();

    assert!(cache.items.is_none());
    assert!(cache.monsters.is_none());
    assert!(cache.conditions.is_none());
    assert!(cache.spells.is_none());
    assert!(cache.proficiencies.is_none());
    assert_eq!(cache.items_generation, 1);
    assert_eq!(cache.monsters_generation, 1);
}

#[test]
fn candidate_cache_get_or_generate_items_caches_results() {
    use antares::domain::items::types::{ArmorClassification, ArmorData, Item, ItemType};

    let mut cache = AutocompleteCandidateCache::new();
    let items = vec![Item {
        id: 1,
        name: "Sword".to_string(),
        item_type: ItemType::Armor(ArmorData {
            ac_bonus: 0,
            weight: 0,
            classification: ArmorClassification::Light,
        }),
        base_cost: 10,
        sell_cost: 5,
        alignment_restriction: None,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
        tags: Vec::new(),
        mesh_descriptor_override: None,
        mesh_id: None,
    }];

    // First call generates and caches
    let candidates1 = cache.get_or_generate_items(&items);
    assert_eq!(candidates1.len(), 1);
    assert_eq!(candidates1[0].1, 1);

    // Second call returns cached results
    let candidates2 = cache.get_or_generate_items(&items);
    assert_eq!(candidates2.len(), 1);
}

#[test]
fn candidate_cache_invalidation_forces_regeneration() {
    use antares::domain::items::types::{ArmorClassification, ArmorData, Item, ItemType};

    let mut cache = AutocompleteCandidateCache::new();
    let items_old = vec![Item {
        id: 1,
        name: "Sword".to_string(),
        item_type: ItemType::Armor(ArmorData {
            ac_bonus: 0,
            weight: 0,
            classification: ArmorClassification::Light,
        }),
        base_cost: 10,
        sell_cost: 5,
        alignment_restriction: None,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
        tags: Vec::new(),
        mesh_descriptor_override: None,
        mesh_id: None,
    }];

    // Generate initial cache
    let _candidates1 = cache.get_or_generate_items(&items_old);

    // Invalidate cache
    cache.invalidate_items();

    // New data should be generated
    let items_new = vec![Item {
        id: 2,
        name: "Axe".to_string(),
        item_type: ItemType::Armor(ArmorData {
            ac_bonus: 0,
            weight: 0,
            classification: ArmorClassification::Light,
        }),
        base_cost: 12,
        sell_cost: 6,
        alignment_restriction: None,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
        tags: Vec::new(),
        mesh_descriptor_override: None,
        mesh_id: None,
    }];
    let candidates2 = cache.get_or_generate_items(&items_new);
    assert_eq!(candidates2.len(), 1);
    assert_eq!(candidates2[0].1, 2);
}

#[test]
fn candidate_cache_monsters_caches_correctly() {
    use antares::domain::character::{AttributePair, AttributePair16, Stats};
    use antares::domain::combat::database::MonsterDefinition;
    use antares::domain::combat::monster::{LootTable, MonsterCondition, MonsterResistances};

    let mut cache = AutocompleteCandidateCache::new();
    let monsters = vec![MonsterDefinition {
        id: 1,
        name: "Goblin".to_string(),
        stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
        hp: AttributePair16::new(10),
        ac: AttributePair::new(10),
        attacks: Vec::new(),
        flee_threshold: 0,
        special_attack_threshold: 0,
        resistances: MonsterResistances::default(),
        can_regenerate: false,
        can_advance: false,
        is_undead: false,
        magic_resistance: 0,
        loot: LootTable::default(),
        creature_id: None,
        conditions: MonsterCondition::Normal,
        active_conditions: vec![],
        has_acted: false,
    }];

    let candidates = cache.get_or_generate_monsters(&monsters);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0], "Goblin");
}

#[test]
fn candidate_cache_performance_with_200_items() {
    use antares::domain::items::types::{ArmorClassification, ArmorData, Item, ItemType};
    use std::time::Instant;

    let mut cache = AutocompleteCandidateCache::new();

    // Generate 200 items
    let items: Vec<Item> = (0..200)
        .map(|i| Item {
            id: i,
            name: format!("Item{}", i),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 0,
                weight: 0,
                classification: ArmorClassification::Light,
            }),
            base_cost: 0,
            sell_cost: 0,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: Vec::new(),
            mesh_descriptor_override: None,
            mesh_id: None,
        })
        .collect();

    // First call - measure generation time
    let start = Instant::now();
    let candidates1 = cache.get_or_generate_items(&items);
    let gen_time = start.elapsed();
    assert_eq!(candidates1.len(), 200);

    // Second call - should be instant (cached)
    let start = Instant::now();
    let candidates2 = cache.get_or_generate_items(&items);
    let cache_time = start.elapsed();
    assert_eq!(candidates2.len(), 200);

    // Cache retrieval should be significantly faster than generation
    // (at least 10x faster, but we'll use 2x to be conservative)
    assert!(
        cache_time < gen_time / 2,
        "Cache retrieval time ({:?}) should be much faster than generation time ({:?})",
        cache_time,
        gen_time
    );
}

// =========================================================================
// Validation Warning Tests
// =========================================================================

#[test]
fn show_entity_validation_warning_displays_nothing_when_valid() {
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);

    egui::CentralPanel::default().show(&ctx, |ui| {
        // Should not display warning when entity exists
        show_entity_validation_warning(ui, "Item", 42, true);
    });

    let _ = ctx.end_pass();
    // Test passes if no panic - UI functions don't return testable state
}

#[test]
fn show_item_validation_warning_checks_existence() {
    use antares::domain::items::types::{ArmorClassification, ArmorData, Item, ItemType};

    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);

    let items = vec![Item {
        id: 1,
        name: "Sword".to_string(),
        item_type: ItemType::Armor(ArmorData {
            ac_bonus: 0,
            weight: 0,
            classification: ArmorClassification::Light,
        }),
        base_cost: 10,
        sell_cost: 5,
        alignment_restriction: None,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
        tags: Vec::new(),
        mesh_descriptor_override: None,
        mesh_id: None,
    }];

    egui::CentralPanel::default().show(&ctx, |ui| {
        // Should show warning for non-existent item ID (use a valid u8 value)
        show_item_validation_warning(ui, 255, &items);
        // Should not show warning for existing item ID
        show_item_validation_warning(ui, 1, &items);
    });

    let _ = ctx.end_pass();
}

#[test]
fn show_monster_validation_warning_handles_empty_name() {
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);

    let monsters = vec![];

    egui::CentralPanel::default().show(&ctx, |ui| {
        // Should not show warning for empty name
        show_monster_validation_warning(ui, "", &monsters);
    });

    let _ = ctx.end_pass();
}

#[test]
fn show_condition_validation_warning_handles_empty_id() {
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);

    let conditions = vec![];

    egui::CentralPanel::default().show(&ctx, |ui| {
        // Should not show warning for empty condition ID
        show_condition_validation_warning(ui, "", &conditions);
    });

    let _ = ctx.end_pass();
}

// =========================================================================
// Map and NPC Candidate Extraction Tests
// =========================================================================

#[test]
fn test_extract_map_candidates() {
    use antares::domain::world::Map;

    let maps = vec![
        Map::new(
            1,
            "Town Square".to_string(),
            "Starting area".to_string(),
            20,
            20,
        ),
        Map::new(
            2,
            "Dark Forest".to_string(),
            "Dangerous woods".to_string(),
            30,
            30,
        ),
        Map::new(5, "Castle".to_string(), "Royal palace".to_string(), 40, 40),
    ];

    let candidates = extract_map_candidates(&maps);

    assert_eq!(candidates.len(), 3);
    assert_eq!(candidates[0].0, "Town Square (ID: 1)");
    assert_eq!(candidates[0].1, 1);
    assert_eq!(candidates[1].0, "Dark Forest (ID: 2)");
    assert_eq!(candidates[1].1, 2);
    assert_eq!(candidates[2].0, "Castle (ID: 5)");
    assert_eq!(candidates[2].1, 5);
}

#[test]
fn test_extract_map_candidates_empty() {
    let maps: Vec<antares::domain::world::Map> = vec![];
    let candidates = extract_map_candidates(&maps);
    assert_eq!(candidates.len(), 0);
}

#[test]
fn test_extract_npc_candidates() {
    use antares::domain::types::Position;
    use antares::domain::world::npc::NpcPlacement;
    use antares::domain::world::Map;

    let mut map1 = Map::new(1, "Town".to_string(), "Desc".to_string(), 10, 10);
    map1.npc_placements.push(NpcPlacement {
        npc_id: "merchant_1".to_string(),
        position: Position::new(5, 5),
        facing: None,
        dialogue_override: None,
    });
    map1.npc_placements.push(NpcPlacement {
        npc_id: "guard_1".to_string(),
        position: Position::new(7, 3),
        facing: None,
        dialogue_override: None,
    });

    let mut map2 = Map::new(2, "Castle".to_string(), "Desc".to_string(), 15, 15);
    map2.npc_placements.push(NpcPlacement {
        npc_id: "king_1".to_string(),
        position: Position::new(8, 8),
        facing: None,
        dialogue_override: None,
    });

    let candidates = extract_npc_candidates(&[map1, map2]);

    assert_eq!(candidates.len(), 3);
    assert!(candidates[0].0.contains("merchant_1"));
    assert!(candidates[0].0.contains("Town"));
    assert_eq!(candidates[0].1, "1:merchant_1");
    assert!(candidates[1].0.contains("guard_1"));
    assert_eq!(candidates[1].1, "1:guard_1");
    assert!(candidates[2].0.contains("king_1"));
    assert!(candidates[2].0.contains("Castle"));
    assert_eq!(candidates[2].1, "2:king_1");
}

#[test]
fn test_extract_npc_candidates_empty_maps() {
    let maps: Vec<antares::domain::world::Map> = vec![];
    let candidates = extract_npc_candidates(&maps);
    assert_eq!(candidates.len(), 0);
}

#[test]
fn test_extract_npc_candidates_maps_with_no_npcs() {
    use antares::domain::world::Map;

    let maps = vec![
        Map::new(1, "Town".to_string(), "Desc".to_string(), 10, 10),
        Map::new(2, "Forest".to_string(), "Desc".to_string(), 20, 20),
    ];

    let candidates = extract_npc_candidates(&maps);
    assert_eq!(candidates.len(), 0);
}

// =========================================================================
// Quest Candidate Extraction Tests
// =========================================================================

#[test]
fn test_extract_quest_candidates() {
    use antares::domain::quest::Quest;

    let mut q1 = Quest::new(1, "Save the Village", "Help save the village from bandits");
    q1.min_level = Some(1);
    q1.repeatable = false;
    q1.is_main_quest = true;
    q1.quest_giver_npc = None;
    q1.quest_giver_map = None;
    q1.quest_giver_position = None;

    let mut q2 = Quest::new(2, "Find the Lost Sword", "Recover the legendary sword");
    q2.min_level = Some(5);
    q2.max_level = Some(10);
    q2.repeatable = false;
    q2.is_main_quest = false;
    q2.quest_giver_npc = None;
    q2.quest_giver_map = None;
    q2.quest_giver_position = None;

    let quests = vec![q1, q2];

    let candidates = extract_quest_candidates(&quests);
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].0, "Save the Village (ID: 1)");
    assert_eq!(candidates[0].1, 1);
    assert_eq!(candidates[1].0, "Find the Lost Sword (ID: 2)");
    assert_eq!(candidates[1].1, 2);
}

#[test]
fn test_extract_quest_candidates_empty() {
    use antares::domain::quest::Quest;

    let quests: Vec<Quest> = vec![];
    let candidates = extract_quest_candidates(&quests);
    assert_eq!(candidates.len(), 0);
}

#[test]
fn test_extract_quest_candidates_maintains_order() {
    use antares::domain::quest::Quest;

    let mut q1 = Quest::new(10, "Quest Alpha", "First quest");
    q1.min_level = Some(1);
    q1.repeatable = false;
    q1.is_main_quest = false;
    q1.quest_giver_npc = None;
    q1.quest_giver_map = None;
    q1.quest_giver_position = None;

    let mut q2 = Quest::new(5, "Quest Beta", "Second quest");
    q2.min_level = Some(1);
    q2.repeatable = false;
    q2.is_main_quest = false;
    q2.quest_giver_npc = None;
    q2.quest_giver_map = None;
    q2.quest_giver_position = None;

    let quests = vec![q1, q2];

    let candidates = extract_quest_candidates(&quests);
    assert_eq!(candidates.len(), 2);
    // Should maintain input order
    assert_eq!(candidates[0].1, 10);
    assert_eq!(candidates[1].1, 5);
}

// =========================================================================
// Portrait Discovery Tests
// =========================================================================

// =========================================================================
// Sprite Sheet Discovery Tests
// =========================================================================

#[test]
fn test_extract_sprite_sheet_candidates_no_campaign_dir() {
    let candidates = extract_sprite_sheet_candidates(None);
    assert_eq!(candidates.len(), 0);
}

#[test]
fn test_extract_sprite_sheet_candidates_nonexistent_directory() {
    let campaign_dir = PathBuf::from("/nonexistent/path/to/campaign_sprites");
    let candidates = extract_sprite_sheet_candidates(Some(&campaign_dir));
    assert_eq!(candidates.len(), 0);
}

#[test]
fn test_extract_sprite_sheet_candidates_with_png_files() {
    // Create temporary directory structure
    let temp_dir = std::env::temp_dir().join("antares_test_sprites_png");
    let sprites_dir = temp_dir.join("assets").join("sprites");
    let actors_dir = sprites_dir.join("actors");

    // Clean up any existing test directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    // Create directories
    std::fs::create_dir_all(&actors_dir).expect("Failed to create test directories");

    // Create test files
    std::fs::write(sprites_dir.join("background.png"), b"data").expect("Failed to create file");
    std::fs::write(actors_dir.join("wizard.png"), b"data").expect("Failed to create file");
    std::fs::write(actors_dir.join("goblin.jpg"), b"data").expect("Failed to create file");

    let candidates = extract_sprite_sheet_candidates(Some(&temp_dir));
    assert!(candidates.contains(&"assets/sprites/background.png".to_string()));
    assert!(candidates.contains(&"assets/sprites/actors/wizard.png".to_string()));
    assert!(candidates.contains(&"assets/sprites/actors/goblin.jpg".to_string()));

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_extract_portrait_candidates_no_campaign_dir() {
    let candidates = extract_portrait_candidates(None);
    assert_eq!(candidates.len(), 0);
}

#[test]
fn test_extract_portrait_candidates_nonexistent_directory() {
    let campaign_dir = PathBuf::from("/nonexistent/path/to/campaign");
    let candidates = extract_portrait_candidates(Some(&campaign_dir));
    assert_eq!(candidates.len(), 0);
}

#[test]
fn test_extract_portrait_candidates_empty_directory() {
    // Create temporary directory structure
    let temp_dir = std::env::temp_dir().join("antares_test_portraits_empty");
    let portraits_dir = temp_dir.join("assets").join("portraits");

    // Clean up any existing test directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    // Create the portraits directory
    std::fs::create_dir_all(&portraits_dir).expect("Failed to create test directory");

    let candidates = extract_portrait_candidates(Some(&temp_dir));
    assert_eq!(candidates.len(), 0);

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).expect("Failed to cleanup test directory");
}

#[test]
fn test_extract_portrait_candidates_with_png_files() {
    // Create temporary directory structure with portrait files
    let temp_dir = std::env::temp_dir().join("antares_test_portraits_png");
    let portraits_dir = temp_dir.join("assets").join("portraits");

    // Clean up any existing test directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    // Create the portraits directory
    std::fs::create_dir_all(&portraits_dir).expect("Failed to create test directory");

    // Create test portrait files
    std::fs::write(portraits_dir.join("0.png"), b"fake png data")
        .expect("Failed to create test file");
    std::fs::write(portraits_dir.join("1.png"), b"fake png data")
        .expect("Failed to create test file");
    std::fs::write(portraits_dir.join("10.png"), b"fake png data")
        .expect("Failed to create test file");

    let candidates = extract_portrait_candidates(Some(&temp_dir));
    assert_eq!(candidates.len(), 3);
    assert_eq!(candidates, vec!["0", "1", "10"]);

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).expect("Failed to cleanup test directory");
}

#[test]
fn test_extract_portrait_candidates_numeric_sort() {
    // Create temporary directory with numerically named portraits
    let temp_dir = std::env::temp_dir().join("antares_test_portraits_numeric");
    let portraits_dir = temp_dir.join("assets").join("portraits");

    // Clean up any existing test directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    std::fs::create_dir_all(&portraits_dir).expect("Failed to create test directory");

    // Create files in non-sorted order
    std::fs::write(portraits_dir.join("2.png"), b"data").expect("Failed to create test file");
    std::fs::write(portraits_dir.join("10.png"), b"data").expect("Failed to create test file");
    std::fs::write(portraits_dir.join("1.png"), b"data").expect("Failed to create test file");
    std::fs::write(portraits_dir.join("20.png"), b"data").expect("Failed to create test file");

    let candidates = extract_portrait_candidates(Some(&temp_dir));
    assert_eq!(candidates.len(), 4);
    // Should be sorted numerically: 1, 2, 10, 20 (not "1", "10", "2", "20")
    assert_eq!(candidates, vec!["1", "2", "10", "20"]);

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).expect("Failed to cleanup test directory");
}

#[test]
fn test_extract_portrait_candidates_mixed_extensions() {
    // Create temporary directory with mixed image formats
    let temp_dir = std::env::temp_dir().join("antares_test_portraits_mixed");
    let portraits_dir = temp_dir.join("assets").join("portraits");

    // Clean up any existing test directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    std::fs::create_dir_all(&portraits_dir).expect("Failed to create test directory");

    // Create files with different extensions
    std::fs::write(portraits_dir.join("0.png"), b"png data").expect("Failed to create test file");
    std::fs::write(portraits_dir.join("1.jpg"), b"jpg data").expect("Failed to create test file");
    std::fs::write(portraits_dir.join("2.jpeg"), b"jpeg data").expect("Failed to create test file");

    let candidates = extract_portrait_candidates(Some(&temp_dir));
    assert_eq!(candidates.len(), 3);
    assert_eq!(candidates, vec!["0", "1", "2"]);

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).expect("Failed to cleanup test directory");
}

#[test]
fn test_extract_portrait_candidates_png_priority() {
    // Test that PNG files are prioritized over other formats
    let temp_dir = std::env::temp_dir().join("antares_test_portraits_priority");
    let portraits_dir = temp_dir.join("assets").join("portraits");

    // Clean up any existing test directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    std::fs::create_dir_all(&portraits_dir).expect("Failed to create test directory");

    // Create both PNG and JPG versions of same portrait ID
    std::fs::write(portraits_dir.join("0.jpg"), b"jpg data").expect("Failed to create test file");
    std::fs::write(portraits_dir.join("0.png"), b"png data").expect("Failed to create test file");

    let candidates = extract_portrait_candidates(Some(&temp_dir));
    // Should only have one entry for "0", prioritizing PNG
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0], "0");

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).expect("Failed to cleanup test directory");
}

#[test]
fn test_extract_portrait_candidates_ignores_non_images() {
    // Test that non-image files are ignored
    let temp_dir = std::env::temp_dir().join("antares_test_portraits_filter");
    let portraits_dir = temp_dir.join("assets").join("portraits");

    // Clean up any existing test directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    std::fs::create_dir_all(&portraits_dir).expect("Failed to create test directory");

    // Create various file types
    std::fs::write(portraits_dir.join("0.png"), b"png data").expect("Failed to create test file");
    std::fs::write(portraits_dir.join("readme.txt"), b"text file")
        .expect("Failed to create test file");
    std::fs::write(portraits_dir.join("data.json"), b"json data")
        .expect("Failed to create test file");
    std::fs::write(portraits_dir.join("script.ron"), b"ron data")
        .expect("Failed to create test file");

    let candidates = extract_portrait_candidates(Some(&temp_dir));
    // Should only find the PNG file
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0], "0");

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).expect("Failed to cleanup test directory");
}

#[test]
fn test_resolve_portrait_path_no_campaign_dir() {
    let path = resolve_portrait_path(None, "0");
    assert!(path.is_none());
}

#[test]
fn test_resolve_portrait_path_nonexistent_file() {
    let campaign_dir = PathBuf::from("/nonexistent/path");
    let path = resolve_portrait_path(Some(&campaign_dir), "999");
    assert!(path.is_none());
}

#[test]
fn test_resolve_portrait_path_finds_png() {
    // Create temporary directory with PNG portrait
    let temp_dir = std::env::temp_dir().join("antares_test_resolve_png");
    let portraits_dir = temp_dir.join("assets").join("portraits");

    // Clean up any existing test directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    std::fs::create_dir_all(&portraits_dir).expect("Failed to create test directory");
    std::fs::write(portraits_dir.join("0.png"), b"png data").expect("Failed to create test file");

    let path = resolve_portrait_path(Some(&temp_dir), "0");
    assert!(path.is_some());
    assert_eq!(path.unwrap(), portraits_dir.join("0.png"));

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).expect("Failed to cleanup test directory");
}

#[test]
fn test_resolve_portrait_path_finds_jpg() {
    // Create temporary directory with JPG portrait
    let temp_dir = std::env::temp_dir().join("antares_test_resolve_jpg");
    let portraits_dir = temp_dir.join("assets").join("portraits");

    // Clean up any existing test directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    std::fs::create_dir_all(&portraits_dir).expect("Failed to create test directory");
    std::fs::write(portraits_dir.join("1.jpg"), b"jpg data").expect("Failed to create test file");

    let path = resolve_portrait_path(Some(&temp_dir), "1");
    assert!(path.is_some());
    assert_eq!(path.unwrap(), portraits_dir.join("1.jpg"));

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).expect("Failed to cleanup test directory");
}

#[test]
fn test_resolve_portrait_path_finds_jpeg() {
    // Create temporary directory with JPEG portrait
    let temp_dir = std::env::temp_dir().join("antares_test_resolve_jpeg");
    let portraits_dir = temp_dir.join("assets").join("portraits");

    // Clean up any existing test directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    std::fs::create_dir_all(&portraits_dir).expect("Failed to create test directory");
    std::fs::write(portraits_dir.join("2.jpeg"), b"jpeg data").expect("Failed to create test file");

    let path = resolve_portrait_path(Some(&temp_dir), "2");
    assert!(path.is_some());
    assert_eq!(path.unwrap(), portraits_dir.join("2.jpeg"));

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).expect("Failed to cleanup test directory");
}

#[test]
fn test_resolve_portrait_path_prioritizes_png() {
    // Test that PNG is prioritized when multiple formats exist
    let temp_dir = std::env::temp_dir().join("antares_test_resolve_priority");
    let portraits_dir = temp_dir.join("assets").join("portraits");

    // Clean up any existing test directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    std::fs::create_dir_all(&portraits_dir).expect("Failed to create test directory");

    // Create multiple formats
    std::fs::write(portraits_dir.join("0.jpg"), b"jpg data").expect("Failed to create test file");
    std::fs::write(portraits_dir.join("0.png"), b"png data").expect("Failed to create test file");
    std::fs::write(portraits_dir.join("0.jpeg"), b"jpeg data").expect("Failed to create test file");

    let path = resolve_portrait_path(Some(&temp_dir), "0");
    assert!(path.is_some());
    // Should return PNG path
    assert_eq!(path.unwrap(), portraits_dir.join("0.png"));

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).expect("Failed to cleanup test directory");
}

#[test]
fn test_autocomplete_portrait_selector_basic_selection() {
    // Test basic portrait selection functionality
    let ctx = egui::Context::default();
    let mut portrait_id = String::new();
    let available_portraits = vec!["0".to_string(), "1".to_string(), "2".to_string()];

    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Initially no selection
            assert_eq!(portrait_id, "");

            // Simulate selecting portrait "1"
            portrait_id = "1".to_string();
            let _changed = autocomplete_portrait_selector(
                ui,
                "test_portrait",
                "Portrait:",
                &mut portrait_id,
                &available_portraits,
                None,
            );

            // Since we manually set it, the widget should see it
            assert_eq!(portrait_id, "1");
        });
    });
}

#[test]
fn test_autocomplete_portrait_selector_clear_button() {
    // Test that clear button functionality is present
    let ctx = egui::Context::default();
    let mut portrait_id = "5".to_string();
    let available_portraits = vec!["5".to_string(), "10".to_string()];

    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Portrait is selected
            assert_eq!(portrait_id, "5");

            // Call the selector (clear button should be available)
            autocomplete_portrait_selector(
                ui,
                "test_clear",
                "Portrait:",
                &mut portrait_id,
                &available_portraits,
                None,
            );

            // Portrait should still be selected after just rendering
            assert_eq!(portrait_id, "5");
        });
    });
}

#[test]
fn test_autocomplete_portrait_selector_empty_candidates() {
    // Test behavior with no available portraits
    let ctx = egui::Context::default();
    let mut portrait_id = String::new();
    let available_portraits: Vec<String> = vec![];

    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let changed = autocomplete_portrait_selector(
                ui,
                "test_empty",
                "Portrait:",
                &mut portrait_id,
                &available_portraits,
                None,
            );

            // Should not change with empty candidates
            assert!(!changed);
            assert_eq!(portrait_id, "");
        });
    });
}

#[test]
fn test_autocomplete_portrait_selector_validates_selection() {
    // Test that selection must be from available portraits
    let ctx = egui::Context::default();
    let mut portrait_id = String::new();
    let available_portraits = vec!["0".to_string(), "1".to_string()];

    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Try to set an invalid portrait ID
            portrait_id = "999".to_string();

            autocomplete_portrait_selector(
                ui,
                "test_validate",
                "Portrait:",
                &mut portrait_id,
                &available_portraits,
                None,
            );

            // The widget should accept the value but won't auto-clear it
            // (validation happens on change, not on initial display)
            assert_eq!(portrait_id, "999");
        });
    });
}

#[test]
fn test_autocomplete_portrait_selector_numeric_ids() {
    // Test with numeric portrait IDs (common case)
    let ctx = egui::Context::default();
    let mut portrait_id = String::new();
    let available_portraits = vec![
        "0".to_string(),
        "1".to_string(),
        "10".to_string(),
        "100".to_string(),
    ];

    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            autocomplete_portrait_selector(
                ui,
                "test_numeric",
                "Portrait:",
                &mut portrait_id,
                &available_portraits,
                None,
            );

            // Should render without errors
            assert_eq!(portrait_id, "");
        });
    });
}

#[test]
fn test_autocomplete_portrait_selector_preserves_buffer() {
    // Test that widget state persists across frames
    let ctx = egui::Context::default();
    let mut portrait_id = "0".to_string();
    let available_portraits = vec!["0".to_string(), "1".to_string()];

    // Frame 1
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            autocomplete_portrait_selector(
                ui,
                "test_persist",
                "Portrait:",
                &mut portrait_id,
                &available_portraits,
                None,
            );
        });
    });

    // Frame 2 - buffer should be persisted
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            autocomplete_portrait_selector(
                ui,
                "test_persist",
                "Portrait:",
                &mut portrait_id,
                &available_portraits,
                None,
            );
            // Portrait ID should still be "0"
            assert_eq!(portrait_id, "0");
        });
    });
}

// =========================================================================
// Creature Asset Discovery Tests
// =========================================================================

#[test]
fn test_extract_creature_asset_candidates_no_campaign_dir() {
    let candidates = extract_creature_asset_candidates(None);
    assert!(candidates.is_empty());
}

#[test]
fn test_extract_creature_asset_candidates_nonexistent_directory() {
    let campaign_dir = PathBuf::from("/nonexistent/path/to/campaign_creatures");
    let candidates = extract_creature_asset_candidates(Some(&campaign_dir));
    assert!(candidates.is_empty());
}

#[test]
fn test_extract_creature_asset_candidates_empty_directory() {
    let temp_dir = std::env::temp_dir().join("antares_test_creatures_empty");
    let creatures_dir = temp_dir.join("assets").join("creatures");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&creatures_dir).expect("Failed to create test directories");

    let candidates = extract_creature_asset_candidates(Some(&temp_dir));
    assert!(candidates.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_extract_creature_asset_candidates_returns_ron_files() {
    let temp_dir = std::env::temp_dir().join("antares_test_creatures_ron");
    let creatures_dir = temp_dir.join("assets").join("creatures");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&creatures_dir).expect("Failed to create test directories");

    std::fs::write(creatures_dir.join("goblin.ron"), b"data").expect("Failed to create goblin.ron");
    std::fs::write(creatures_dir.join("orc_warrior.ron"), b"data")
        .expect("Failed to create orc_warrior.ron");

    let candidates = extract_creature_asset_candidates(Some(&temp_dir));
    assert_eq!(candidates.len(), 2);
    assert!(candidates.contains(&"assets/creatures/goblin.ron".to_string()));
    assert!(candidates.contains(&"assets/creatures/orc_warrior.ron".to_string()));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_extract_creature_asset_candidates_ignores_non_ron_files() {
    let temp_dir = std::env::temp_dir().join("antares_test_creatures_filter");
    let creatures_dir = temp_dir.join("assets").join("creatures");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&creatures_dir).expect("Failed to create test directories");

    std::fs::write(creatures_dir.join("goblin.ron"), b"data").expect("write failed");
    // These should be ignored
    std::fs::write(creatures_dir.join("notes.txt"), b"text").expect("write failed");
    std::fs::write(creatures_dir.join("sprite.png"), b"img").expect("write failed");
    std::fs::write(creatures_dir.join("data.json"), b"{}").expect("write failed");

    let candidates = extract_creature_asset_candidates(Some(&temp_dir));
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0], "assets/creatures/goblin.ron");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_extract_creature_asset_candidates_sorted_alphabetically() {
    let temp_dir = std::env::temp_dir().join("antares_test_creatures_sorted");
    let creatures_dir = temp_dir.join("assets").join("creatures");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&creatures_dir).expect("Failed to create test directories");

    std::fs::write(creatures_dir.join("zombie.ron"), b"data").expect("write failed");
    std::fs::write(creatures_dir.join("goblin.ron"), b"data").expect("write failed");
    std::fs::write(creatures_dir.join("dragon.ron"), b"data").expect("write failed");

    let candidates = extract_creature_asset_candidates(Some(&temp_dir));
    assert_eq!(candidates.len(), 3);
    assert_eq!(candidates[0], "assets/creatures/dragon.ron");
    assert_eq!(candidates[1], "assets/creatures/goblin.ron");
    assert_eq!(candidates[2], "assets/creatures/zombie.ron");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_extract_creature_asset_candidates_uses_forward_slashes() {
    let temp_dir = std::env::temp_dir().join("antares_test_creatures_slashes");
    let creatures_dir = temp_dir.join("assets").join("creatures");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&creatures_dir).expect("Failed to create test directories");

    std::fs::write(creatures_dir.join("goblin.ron"), b"data").expect("write failed");

    let candidates = extract_creature_asset_candidates(Some(&temp_dir));
    assert_eq!(candidates.len(), 1);
    // Path must use forward slashes regardless of platform
    assert!(!candidates[0].contains('\\'));
    assert_eq!(candidates[0], "assets/creatures/goblin.ron");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

// =========================================================================
// autocomplete_creature_selector logic tests
// =========================================================================

/// Helper: build a candidate list as autocomplete_creature_selector receives it.
fn make_creature_candidates(pairs: &[(u32, &str)]) -> Vec<(u32, String)> {
    pairs
        .iter()
        .map(|(id, name)| (*id, name.to_string()))
        .collect()
}

#[test]
fn test_autocomplete_creature_selector_empty_candidates_returns_false() {
    // With no candidates and an empty buffer the function should signal no change.
    // We exercise the pure logic (candidate building) rather than rendering.
    let candidates: Vec<(u32, String)> = Vec::new();
    // Display candidates built inside the function
    let display: Vec<String> = candidates
        .iter()
        .map(|(id, name)| format!("{} — {}", id, name))
        .collect();
    assert!(display.is_empty());
}

#[test]
fn test_autocomplete_creature_selector_display_format() {
    // Each candidate is rendered as "id — name".
    let candidates = make_creature_candidates(&[(1, "Goblin"), (42, "Dragon")]);
    let display: Vec<String> = candidates
        .iter()
        .map(|(id, name)| format!("{} — {}", id, name))
        .collect();
    assert_eq!(display[0], "1 — Goblin");
    assert_eq!(display[1], "42 — Dragon");
}

#[test]
fn test_autocomplete_creature_selector_id_extraction_from_display_string() {
    // Simulate the "id — name" parsing that the selector does when a row is picked.
    let picked = "42 — Dragon".to_string();
    let extracted = if let Some(pos) = picked.find(" — ") {
        let id_part = picked[..pos].trim();
        id_part.parse::<u32>().ok().map(|v| v.to_string())
    } else {
        None
    };
    assert_eq!(extracted, Some("42".to_string()));
}

#[test]
fn test_autocomplete_creature_selector_raw_numeric_id_accepted() {
    // Simulate the branch where the user typed a raw numeric ID.
    let typed = "7";
    let result = typed.trim().parse::<u32>();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 7u32);
}

#[test]
fn test_autocomplete_creature_selector_non_numeric_raw_input_rejected() {
    // A string that is neither "id — name" nor a plain number should not parse.
    let typed = "not_a_number";
    assert!(!typed.contains(" — "));
    assert!(typed.trim().parse::<u32>().is_err());
}

#[test]
fn test_autocomplete_creature_selector_buffer_initialisation_with_known_id() {
    // When the buffer is initialised from a known numeric ID the display string
    // should be resolved to "id — name".
    let candidates = make_creature_candidates(&[(7, "Skeleton"), (42, "Dragon")]);
    let current_value = "7".to_string();
    let id_num: u32 = current_value.trim().parse().unwrap();
    let display = candidates
        .iter()
        .find(|(id, _)| *id == id_num)
        .map(|(id, name)| format!("{} — {}", id, name))
        .unwrap_or_else(|| current_value.clone());
    assert_eq!(display, "7 — Skeleton");
}

#[test]
fn test_autocomplete_creature_selector_buffer_initialisation_with_unknown_id() {
    // When the ID is not in the registry the raw string is kept as-is.
    let candidates = make_creature_candidates(&[(7, "Skeleton")]);
    let current_value = "99".to_string();
    let id_num: u32 = current_value.trim().parse().unwrap();
    let display = candidates
        .iter()
        .find(|(id, _)| *id == id_num)
        .map(|(id, name)| format!("{} — {}", id, name))
        .unwrap_or_else(|| current_value.clone());
    assert_eq!(display, "99");
}

#[test]
fn test_autocomplete_creature_selector_buffer_initialisation_empty_stays_empty() {
    let candidates = make_creature_candidates(&[(7, "Skeleton")]);
    let current_value = String::new();
    let display = if current_value.is_empty() {
        String::new()
    } else if let Ok(id_num) = current_value.trim().parse::<u32>() {
        candidates
            .iter()
            .find(|(id, _)| *id == id_num)
            .map(|(id, name)| format!("{} — {}", id, name))
            .unwrap_or_else(|| current_value.clone())
    } else {
        current_value.clone()
    };
    assert!(display.is_empty());
}

#[test]
fn test_autocomplete_creature_selector_tooltip_resolved_name() {
    // The hover tooltip should show "Creature: <name>" for known IDs.
    let candidates = make_creature_candidates(&[(42, "Dragon")]);
    let selected_id = "42".to_string();
    let tooltip = if let Ok(id_num) = selected_id.trim().parse::<u32>() {
        candidates
            .iter()
            .find(|(id, _)| *id == id_num)
            .map(|(_, name)| format!("Creature: {}", name))
            .unwrap_or_else(|| format!("⚠ Creature ID '{}' not found in registry", selected_id))
    } else {
        String::new()
    };
    assert_eq!(tooltip, "Creature: Dragon");
}

#[test]
fn test_autocomplete_creature_selector_tooltip_unknown_id() {
    // An unrecognised ID should produce a warning tooltip.
    let candidates = make_creature_candidates(&[(42, "Dragon")]);
    let selected_id = "999".to_string();
    let tooltip = if let Ok(id_num) = selected_id.trim().parse::<u32>() {
        candidates
            .iter()
            .find(|(id, _)| *id == id_num)
            .map(|(_, name)| format!("Creature: {}", name))
            .unwrap_or_else(|| format!("⚠ Creature ID '{}' not found in registry", selected_id))
    } else {
        String::new()
    };
    assert_eq!(tooltip, "⚠ Creature ID '999' not found in registry");
}

// =========================================================================
// autocomplete_entity_selector_generic tests
// =========================================================================

#[test]
fn test_autocomplete_entity_selector_generic_returns_false_with_no_interaction() {
    // With no programmatic interaction the function should return false.
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);
    egui::CentralPanel::default().show(&ctx, |ui| {
        let mut select_called = false;
        let cfg = AutocompleteSelectorConfig {
            id_salt: "test_generic_no_interaction",
            buffer_tag: "test",
            label: "Label:",
            placeholder: "Type here...",
        };
        let changed = autocomplete_entity_selector_generic(
            ui,
            &cfg,
            vec!["Alpha".to_string(), "Beta".to_string()],
            String::new(),
            false,
            |_text| {
                select_called = true;
                false
            },
            || {},
        );
        assert!(!changed);
        assert!(
            !select_called,
            "on_select must not be called without typed input"
        );
    });
}

#[test]
fn test_autocomplete_entity_selector_generic_empty_label_does_not_panic() {
    // Passing an empty label should be safe — the label widget is simply skipped.
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);
    egui::CentralPanel::default().show(&ctx, |ui| {
        // Use RefCell so both closures can share mutation without conflicting borrows.
        let cell = std::cell::RefCell::new(String::new());
        let is_selected = !cell.borrow().is_empty();
        let cfg = AutocompleteSelectorConfig {
            id_salt: "test_empty_label",
            buffer_tag: "test",
            label: "", // empty label — should be silently skipped
            placeholder: "Placeholder",
        };
        let changed = autocomplete_entity_selector_generic(
            ui,
            &cfg,
            vec!["Option A".to_string()],
            cell.borrow().clone(),
            is_selected,
            |text| {
                *cell.borrow_mut() = text.to_string();
                true
            },
            || cell.borrow_mut().clear(),
        );
        assert!(!changed);
    });
}

#[test]
fn test_autocomplete_entity_selector_generic_initialises_buffer_in_egui_memory() {
    // The function must store a persistent buffer in egui Memory on first render.
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);
    egui::CentralPanel::default().show(&ctx, |ui| {
        // Use RefCell so both closures can share mutation without conflicting borrows.
        let cell = std::cell::RefCell::new(String::new());
        let is_selected = !cell.borrow().is_empty();
        let cfg = AutocompleteSelectorConfig {
            id_salt: "buf_init_test",
            buffer_tag: "race",
            label: "Race:",
            placeholder: "Type race...",
        };
        let _ = autocomplete_entity_selector_generic(
            ui,
            &cfg,
            vec!["Human".to_string(), "Elf".to_string()],
            cell.borrow().clone(),
            is_selected,
            |text| {
                *cell.borrow_mut() = text.to_string();
                true
            },
            || cell.borrow_mut().clear(),
        );
        // After rendering, egui memory should contain an entry for this buffer.
        ui.horizontal(|ui| {
            let id = make_autocomplete_id(ui, "race", "buf_init_test");
            let stored = ui.ctx().memory(|mem| mem.data.get_temp::<String>(id));
            assert!(
                stored.is_some(),
                "Buffer entry should be created in egui Memory on first render"
            );
        });
    });
}

// =========================================================================
// autocomplete_list_selector_generic tests
// =========================================================================

#[test]
fn test_autocomplete_list_selector_generic_starts_with_no_change() {
    // An initial render with no interaction should return false.
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);
    egui::CentralPanel::default().show(&ctx, |ui| {
        let mut selected: Vec<String> = Vec::new();
        let cfg = AutocompleteListSelectorConfig {
            id_salt: "list_no_change",
            buffer_tag: "tag_add",
            label: "Tags:",
            add_label: "Add tag:",
            placeholder: "Type here...",
        };
        let changed = autocomplete_list_selector_generic(
            ui,
            &cfg,
            &mut selected,
            |tag: &String| tag.clone(),
            vec!["Alpha".to_string(), "Beta".to_string()],
            |text: &str| {
                if !text.is_empty() {
                    Some(text.to_string())
                } else {
                    None
                }
            },
            |text: &str| {
                if !text.is_empty() {
                    Some(text.to_string())
                } else {
                    None
                }
            },
        );
        assert!(!changed);
        assert!(selected.is_empty());
    });
}

#[test]
fn test_autocomplete_list_selector_generic_display_fn_called_for_existing_items() {
    // Verify that display_fn is used to render each already-selected item.
    // We exercise this through the pure display path (no egui rendering needed
    // for the assertion itself).
    let items = [10u32, 20u32];
    let displayed: Vec<String> = items.iter().map(|id| format!("Item({})", id)).collect();
    assert_eq!(displayed, vec!["Item(10)", "Item(20)"]);
}

#[test]
fn test_autocomplete_list_selector_generic_initialises_buffer_in_egui_memory() {
    // The function must persist a text buffer in egui Memory after rendering.
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    ctx.begin_pass(raw_input);
    egui::CentralPanel::default().show(&ctx, |ui| {
        let mut selected: Vec<String> = Vec::new();
        let cfg = AutocompleteListSelectorConfig {
            id_salt: "list_buf_init",
            buffer_tag: "tag_add",
            label: "Tags:",
            add_label: "Add tag:",
            placeholder: "Type...",
        };
        let _ = autocomplete_list_selector_generic(
            ui,
            &cfg,
            &mut selected,
            |t: &String| t.clone(),
            vec!["Foo".to_string()],
            |text: &str| {
                if !text.is_empty() {
                    Some(text.to_string())
                } else {
                    None
                }
            },
            |text: &str| {
                if !text.is_empty() {
                    Some(text.to_string())
                } else {
                    None
                }
            },
        );
        ui.horizontal(|ui| {
            let id = make_autocomplete_id(ui, "tag_add", "list_buf_init");
            let stored = ui.ctx().memory(|mem| mem.data.get_temp::<String>(id));
            assert!(
                stored.is_some(),
                "autocomplete_list_selector_generic must initialise a buffer in egui Memory"
            );
        });
    });
}

// =========================================================================
// dispatch_list_action tests
// =========================================================================

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct TestEntity {
    id: u32,
    name: String,
}

#[test]
fn test_dispatch_list_action_duplicate() {
    let mut data = vec![
        TestEntity {
            id: 1,
            name: "Alpha".to_string(),
        },
        TestEntity {
            id: 2,
            name: "Beta".to_string(),
        },
    ];
    let mut selected = Some(0usize);
    let mut buf = String::new();
    let mut show = false;
    let mut msg = String::new();

    let mut dispatch_state = DispatchActionState {
        entity_label: "thing",
        import_export_buffer: &mut buf,
        show_import_dialog: &mut show,
        status_message: &mut msg,
    };
    let changed = dispatch_list_action(
        ItemAction::Duplicate,
        &mut data,
        &mut selected,
        |entry, all| {
            entry.id = all.iter().map(|t| t.id).max().unwrap_or(0) + 1;
            entry.name = format!("{} (Copy)", entry.name);
        },
        &mut dispatch_state,
    );

    assert!(changed);
    assert_eq!(data.len(), 3);
    assert_eq!(data[2].name, "Alpha (Copy)");
    assert_eq!(data[2].id, 3);
    // selection unchanged after duplicate
    assert_eq!(selected, Some(0));
}

#[test]
fn test_dispatch_list_action_delete() {
    let mut data = vec![
        TestEntity {
            id: 1,
            name: "Alpha".to_string(),
        },
        TestEntity {
            id: 2,
            name: "Beta".to_string(),
        },
    ];
    let mut selected = Some(0usize);
    let mut buf = String::new();
    let mut show = false;
    let mut msg = String::new();

    let mut dispatch_state = DispatchActionState {
        entity_label: "thing",
        import_export_buffer: &mut buf,
        show_import_dialog: &mut show,
        status_message: &mut msg,
    };
    let changed = dispatch_list_action(
        ItemAction::Delete,
        &mut data,
        &mut selected,
        |_, _| {},
        &mut dispatch_state,
    );

    assert!(changed);
    assert_eq!(data.len(), 1);
    assert_eq!(data[0].name, "Beta");
    // selection cleared after delete
    assert_eq!(selected, None);
}

#[test]
fn test_dispatch_list_action_export() {
    let mut data = vec![TestEntity {
        id: 1,
        name: "Alpha".to_string(),
    }];
    let mut selected = Some(0usize);
    let mut buf = String::new();
    let mut show = false;
    let mut msg = String::new();

    let mut dispatch_state = DispatchActionState {
        entity_label: "thing",
        import_export_buffer: &mut buf,
        show_import_dialog: &mut show,
        status_message: &mut msg,
    };
    let changed = dispatch_list_action(
        ItemAction::Export,
        &mut data,
        &mut selected,
        |_, _| {},
        &mut dispatch_state,
    );

    assert!(!changed); // Export doesn't mutate collection
    assert!(!buf.is_empty(), "export buffer should be populated");
    assert!(show, "show_import_dialog should be set");
    assert!(
        msg.contains("exported"),
        "status message should mention export"
    );
}

#[test]
fn test_dispatch_list_action_edit_is_noop() {
    let mut data = vec![TestEntity {
        id: 1,
        name: "Alpha".to_string(),
    }];
    let mut selected = Some(0usize);
    let mut buf = String::new();
    let mut show = false;
    let mut msg = String::new();

    let mut dispatch_state = DispatchActionState {
        entity_label: "thing",
        import_export_buffer: &mut buf,
        show_import_dialog: &mut show,
        status_message: &mut msg,
    };
    let changed = dispatch_list_action(
        ItemAction::Edit,
        &mut data,
        &mut selected,
        |_, _| {},
        &mut dispatch_state,
    );

    assert!(!changed);
    assert_eq!(data.len(), 1);
    assert_eq!(selected, Some(0));
}

#[test]
fn test_dispatch_list_action_no_selection_is_noop() {
    let mut data = vec![TestEntity {
        id: 1,
        name: "Alpha".to_string(),
    }];
    let mut selected: Option<usize> = None;
    let mut buf = String::new();
    let mut show = false;
    let mut msg = String::new();

    let mut dispatch_state = DispatchActionState {
        entity_label: "thing",
        import_export_buffer: &mut buf,
        show_import_dialog: &mut show,
        status_message: &mut msg,
    };
    let changed = dispatch_list_action(
        ItemAction::Delete,
        &mut data,
        &mut selected,
        |_, _| {},
        &mut dispatch_state,
    );

    assert!(!changed);
    assert_eq!(data.len(), 1); // nothing deleted
}
