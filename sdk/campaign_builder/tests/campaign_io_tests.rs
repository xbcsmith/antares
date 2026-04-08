// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign I/O tests – load/save/validate methods, merchant dialogue,
//! NPC validation, ID uniqueness checks.

use antares::domain::character::{Alignment, Sex, FOOD_MAX};
use antares::domain::character_definition::CharacterDefinition;
use antares::domain::classes::ClassDefinition;
use antares::domain::conditions::ConditionDefinition;
use antares::domain::dialogue::{DialogueAction, DialogueTree};
use antares::domain::items::types::{ItemType, WeaponClassification, WeaponData};
use antares::domain::proficiency::ProficiencyDefinition;

use antares::domain::races::RaceDefinition;
use antares::domain::types::DiceRoll;
use antares::domain::world::Map;
use campaign_builder::items_editor::ItemsEditorState;
use campaign_builder::monsters_editor::MonstersEditorState;
use campaign_builder::*;

#[test]
fn test_handle_maps_open_npc_request_success() {
    let mut app = CampaignBuilderApp::default();

    // Add a sample NPC definition and ensure it's in the NPC list
    let npc = antares::domain::world::npc::NpcDefinition {
        id: "npc_1".to_string(),
        name: "Test NPC".to_string(),
        description: String::new(),
        portrait_id: String::new(),
        sprite: None,
        dialogue_id: None,
        creature_id: None,
        quest_ids: Vec::new(),
        faction: None,
        is_merchant: false,
        is_innkeeper: false,
        is_priest: false,
        stock_template: None,
        service_catalog: None,
        economy: None,
    };
    app.editor_registry.npc_editor_state.npcs.push(npc);

    // Simulate the maps editor requesting to open the NPC editor for this NPC
    app.editor_registry.maps_editor_state.requested_open_npc = Some("npc_1".to_string());

    // Execute the handler
    app.handle_maps_open_npc_request();

    // Verify the app switched to the NPCs tab and started editing the expected NPC
    assert_eq!(app.ui_state.active_tab, EditorTab::NPCs);
    assert_eq!(
        app.editor_registry.npc_editor_state.mode,
        campaign_builder::npc_editor::NpcEditorMode::Edit
    );
    assert_eq!(app.editor_registry.npc_editor_state.selected_npc, Some(0));
    assert!(app.ui_state.status_message.contains("Opening NPC editor"));
    assert!(app
        .editor_registry
        .maps_editor_state
        .requested_open_npc
        .is_none());
}

#[test]
fn test_handle_maps_open_npc_request_not_found() {
    let mut app = CampaignBuilderApp::default();

    // Request a non-existent NPC
    app.editor_registry.maps_editor_state.requested_open_npc = Some("missing_npc".to_string());

    // Execute the handler
    app.handle_maps_open_npc_request();

    // Should not switch tabs and should clear the request with an informative message
    assert_eq!(app.ui_state.active_tab, EditorTab::Metadata);
    assert!(app
        .editor_registry
        .maps_editor_state
        .requested_open_npc
        .is_none());
    assert!(app.ui_state.status_message.contains("missing_npc"));
}

#[test]
fn test_handle_validation_open_npc_request_success() {
    let mut app = CampaignBuilderApp::default();
    app.ui_state.active_tab = EditorTab::Validation;
    app.editor_registry
        .npc_editor_state
        .npcs
        .push(antares::domain::world::npc::NpcDefinition {
            id: "merchant_tom".to_string(),
            name: "Tom".to_string(),
            description: String::new(),
            portrait_id: "tom".to_string(),
            dialogue_id: None,
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
        });
    app.editor_registry.npc_editor_state.requested_open_npc = Some("merchant_tom".to_string());

    app.handle_validation_open_npc_request();

    assert_eq!(app.ui_state.active_tab, EditorTab::NPCs);
    assert_eq!(app.editor_registry.npc_editor_state.selected_npc, Some(0));
    assert_eq!(
        app.editor_registry.npc_editor_state.mode,
        npc_editor::NpcEditorMode::Edit
    );
    assert!(app.ui_state.status_message.contains("merchant_tom"));
}

#[test]
fn test_handle_validation_open_npc_request_not_found() {
    let mut app = CampaignBuilderApp::default();
    app.editor_registry.npc_editor_state.requested_open_npc = Some("missing_merchant".to_string());

    app.handle_validation_open_npc_request();

    assert_eq!(app.ui_state.active_tab, EditorTab::Metadata);
    assert!(app.ui_state.status_message.contains("missing_merchant"));
}

#[test]
fn test_validate_merchant_dialogue_rules_reports_missing_dialogue() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.npcs_file = "data/npcs.ron".to_string();

    app.editor_registry
        .npc_editor_state
        .npcs
        .push(antares::domain::world::npc::NpcDefinition {
            id: "merchant_no_dialogue".to_string(),
            name: "No Dialogue".to_string(),
            description: String::new(),
            portrait_id: "merchant".to_string(),
            dialogue_id: None,
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
        });

    let results = app.validate_merchant_dialogue_rules();

    assert!(results.iter().any(|result| {
        result.message.contains("merchant_no_dialogue")
            && result.message.contains("has no dialogue assigned")
            && result.severity == validation::ValidationSeverity::Error
    }));
}

#[test]
fn test_validate_merchant_dialogue_rules_reports_missing_dialogue_tree() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.npcs_file = "data/npcs.ron".to_string();

    app.editor_registry
        .npc_editor_state
        .npcs
        .push(antares::domain::world::npc::NpcDefinition {
            id: "merchant_missing_tree".to_string(),
            name: "Missing Tree".to_string(),
            description: String::new(),
            portrait_id: "merchant".to_string(),
            dialogue_id: Some(77),
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
        });

    let results = app.validate_merchant_dialogue_rules();

    assert!(results.iter().any(|result| {
        result.message.contains("merchant_missing_tree")
            && result.message.contains("references missing dialogue 77")
            && result.severity == validation::ValidationSeverity::Error
    }));
}

#[test]
fn test_validate_merchant_dialogue_rules_reports_wrong_open_merchant_target() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.dialogue_file = "data/dialogue.ron".to_string();

    let mut dialogue = DialogueTree::new(5, "Wrong Merchant".to_string(), 1);
    let mut root = antares::domain::dialogue::DialogueNode::new(1, "Welcome.");
    let mut choice = antares::domain::dialogue::DialogueChoice::new("Shop", Some(2));
    choice.add_action(DialogueAction::OpenMerchant {
        npc_id: "other_merchant".to_string(),
    });
    root.add_choice(choice);
    dialogue.add_node(root);
    dialogue.add_node(antares::domain::dialogue::DialogueNode::new(2, "Shop node"));
    app.campaign_data.dialogues.push(dialogue);

    app.editor_registry
        .npc_editor_state
        .npcs
        .push(antares::domain::world::npc::NpcDefinition {
            id: "merchant_wrong_target".to_string(),
            name: "Wrong Target".to_string(),
            description: String::new(),
            portrait_id: "merchant".to_string(),
            dialogue_id: Some(5),
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
        });

    let results = app.validate_merchant_dialogue_rules();

    assert!(results.iter().any(|result| {
        result.message.contains("merchant_wrong_target")
            && result.message.contains("wrong merchant target")
            && result.severity == validation::ValidationSeverity::Error
    }));
}

#[test]
fn test_validate_merchant_dialogue_rules_reports_stale_sdk_content_for_non_merchant() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.dialogue_file = "data/dialogue.ron".to_string();

    let dialogue = DialogueTree::standard_merchant_template(9, "merchant_stale", "Stale Merchant");
    app.campaign_data.dialogues.push(dialogue);

    app.editor_registry
        .npc_editor_state
        .npcs
        .push(antares::domain::world::npc::NpcDefinition {
            id: "merchant_stale".to_string(),
            name: "Stale Merchant".to_string(),
            description: String::new(),
            portrait_id: "merchant".to_string(),
            dialogue_id: Some(9),
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
        });

    let results = app.validate_merchant_dialogue_rules();

    assert!(results.iter().any(|result| {
        result.message.contains("merchant_stale")
            && result.message.contains("SDK-managed merchant content")
            && result.severity == validation::ValidationSeverity::Warning
    }));
}

#[test]
fn test_repair_merchant_dialogue_validation_issues_creates_missing_dialogue() {
    let mut app = CampaignBuilderApp::default();

    app.editor_registry
        .npc_editor_state
        .npcs
        .push(antares::domain::world::npc::NpcDefinition {
            id: "merchant_repair_create".to_string(),
            name: "Create Merchant".to_string(),
            description: String::new(),
            portrait_id: "merchant".to_string(),
            dialogue_id: None,
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
        });

    let result = app.repair_merchant_dialogue_validation_issues();

    assert!(result.message.contains("Applied merchant dialogue repairs"));
    assert_eq!(app.campaign_data.dialogues.len(), 1);
    assert_eq!(
        app.editor_registry.npc_editor_state.npcs[0].dialogue_id,
        Some(1)
    );
    assert!(app.campaign_data.dialogues[0].contains_open_merchant_for_npc("merchant_repair_create"));
}

#[test]
fn test_repair_merchant_dialogue_validation_issues_rebinds_wrong_target() {
    let mut app = CampaignBuilderApp::default();

    let mut dialogue = DialogueTree::new(13, "Wrong Target".to_string(), 1);
    let mut root = antares::domain::dialogue::DialogueNode::new(1, "Welcome.");
    let mut choice = antares::domain::dialogue::DialogueChoice::new("Shop", Some(2));
    choice.add_action(DialogueAction::OpenMerchant {
        npc_id: "other_target".to_string(),
    });
    root.add_choice(choice);
    dialogue.add_node(root);
    dialogue.add_node(antares::domain::dialogue::DialogueNode::new(2, "Shop node"));
    app.campaign_data.dialogues.push(dialogue);

    app.editor_registry
        .npc_editor_state
        .npcs
        .push(antares::domain::world::npc::NpcDefinition {
            id: "merchant_rebind".to_string(),
            name: "Rebind Merchant".to_string(),
            description: String::new(),
            portrait_id: "merchant".to_string(),
            dialogue_id: Some(13),
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
        });

    let result = app.repair_merchant_dialogue_validation_issues();

    assert!(result.message.contains("Applied merchant dialogue repairs"));
    let repaired = app
        .campaign_data
        .dialogues
        .iter()
        .find(|dialogue| dialogue.id == 13)
        .expect("repaired dialogue should exist");
    assert!(repaired.contains_open_merchant_for_npc("merchant_rebind"));
    assert!(!repaired.nodes.values().any(|node| {
        node.actions.iter().any(|action| {
            matches!(
                action,
                DialogueAction::OpenMerchant { npc_id } if npc_id == "other_target"
            )
        }) || node.choices.iter().any(|choice| {
            choice.actions.iter().any(|action| {
                matches!(
                    action,
                    DialogueAction::OpenMerchant { npc_id } if npc_id == "other_target"
                )
            })
        })
    }));
}

#[test]
fn test_repair_merchant_dialogue_validation_issues_removes_stale_non_merchant_content() {
    let mut app = CampaignBuilderApp::default();

    let dialogue =
        DialogueTree::standard_merchant_template(17, "merchant_stale_cleanup", "Cleanup");
    app.campaign_data.dialogues.push(dialogue);

    app.editor_registry
        .npc_editor_state
        .npcs
        .push(antares::domain::world::npc::NpcDefinition {
            id: "merchant_stale_cleanup".to_string(),
            name: "Cleanup".to_string(),
            description: String::new(),
            portrait_id: "merchant".to_string(),
            dialogue_id: Some(17),
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
        });

    let result = app.repair_merchant_dialogue_validation_issues();

    assert!(result.message.contains("Applied merchant dialogue repairs"));
    let repaired = app
        .campaign_data
        .dialogues
        .iter()
        .find(|dialogue| dialogue.id == 17)
        .expect("cleaned dialogue should exist");
    assert!(!repaired.has_sdk_managed_merchant_content());
    assert!(!repaired.contains_open_merchant_for_npc("merchant_stale_cleanup"));
    assert!(repaired.get_node(repaired.root_node).is_some());
}

#[test]
fn test_do_new_campaign_clears_loaded_data() {
    let mut app = CampaignBuilderApp::default();

    // Populate editors and domain data to simulate an open campaign
    app.campaign_data
        .items
        .push(ItemsEditorState::default_item());
    app.editor_registry.items_editor_state.search_query = "sword".to_string();

    app.campaign_data
        .spells
        .push(CampaignBuilderApp::default_spell());
    app.editor_registry.spells_editor_state.search_query = "fire".to_string();

    app.campaign_data
        .monsters
        .push(CampaignBuilderApp::default_monster());
    app.editor_registry.monsters_editor_state.edit_buffer.name = "Orc".to_string();

    // Add a simple condition definition
    use antares::domain::conditions::{ConditionDefinition, ConditionDuration};
    app.campaign_data.conditions.push(ConditionDefinition {
        id: "cond_1".to_string(),
        name: "Test Condition".to_string(),
        description: String::new(),
        effects: Vec::new(),
        default_duration: ConditionDuration::Permanent,
        icon_id: None,
    });

    // Add a map and a quest
    use antares::domain::world::Map;
    app.campaign_data.maps.push(Map::new(
        1,
        "test_map".to_string(),
        "desc".to_string(),
        10,
        10,
    ));
    app.campaign_data
        .quests
        .push(antares::domain::quest::Quest::new(1, "Test Quest", "desc"));

    // Add a dialogue tree
    use antares::domain::dialogue::DialogueTree;
    let dialogue = DialogueTree::new(1, "Test Dialogue", 1);
    app.campaign_data.dialogues.push(dialogue);
    app.editor_registry.dialogue_editor_state.selected_dialogue = Some(0);

    // Add an NPC
    app.editor_registry.npc_editor_state.npcs.push(
        antares::domain::world::npc::NpcDefinition::new("npc_1", "NPC 1", "portrait_1"),
    );

    // Sanity checks: ensure data present before invoking the method under test
    assert!(!app.campaign_data.items.is_empty());
    assert!(!app.campaign_data.spells.is_empty());
    assert!(!app.campaign_data.monsters.is_empty());
    assert!(!app.campaign_data.maps.is_empty());
    assert!(!app.campaign_data.quests.is_empty());
    assert!(!app.campaign_data.dialogues.is_empty());
    assert!(!app.editor_registry.npc_editor_state.npcs.is_empty());

    // Call the method under test
    app.do_new_campaign();

    // Assert everything cleared and editors reset
    assert!(app.campaign_data.items.is_empty());
    assert!(app.campaign_data.spells.is_empty());
    assert!(app.campaign_data.monsters.is_empty());
    assert!(app.campaign_data.maps.is_empty());
    assert!(app.campaign_data.quests.is_empty());
    assert!(app.campaign_data.dialogues.is_empty());
    assert!(app.editor_registry.npc_editor_state.npcs.is_empty());

    // Editor states reset
    assert!(app
        .editor_registry
        .items_editor_state
        .search_query
        .is_empty());
    assert!(app
        .editor_registry
        .spells_editor_state
        .search_query
        .is_empty());
    assert_eq!(
        app.editor_registry.monsters_editor_state.edit_buffer.name,
        MonstersEditorState::new().edit_buffer.name,
        "Monster editor buffer should be reset to default"
    );
    assert!(app.asset_manager.is_none());
    assert!(!app.unsaved_changes);
}

#[test]
fn test_generate_category_status_checks_empty_races_shows_info() {
    // Default app has no races loaded
    let app = CampaignBuilderApp::default();
    let results = app.generate_category_status_checks();

    let race_info = results
        .iter()
        .find(|r| r.category == validation::ValidationCategory::Races);

    assert!(
        race_info.is_some(),
        "Races category missing from validation results"
    );
    let result = race_info.unwrap();
    assert_eq!(
        result.severity,
        validation::ValidationSeverity::Info,
        "Expected Info severity for empty races"
    );
    assert!(
        result
            .message
            .contains("No races loaded - add races or load from file"),
        "Unexpected message: {}",
        result.message
    );
}

#[test]
fn test_generate_category_status_checks_loaded_races_shows_passed() {
    // Create a CampaignBuilderApp and add a single race definition
    let mut app = CampaignBuilderApp::default();
    let race = RaceDefinition::new("test".to_string(), "Test".to_string(), "A test".to_string());
    app.editor_registry.races_editor_state.races.push(race);

    let results = app.generate_category_status_checks();
    let race_result = results
        .iter()
        .find(|r| r.category == validation::ValidationCategory::Races);

    assert!(
        race_result.is_some(),
        "Races category missing from validation results"
    );
    let result = race_result.unwrap();
    assert_eq!(
        result.severity,
        validation::ValidationSeverity::Passed,
        "Expected Passed severity when races are loaded"
    );
    assert!(
        result.message.contains(&format!(
            "{} races validated",
            app.editor_registry.races_editor_state.races.len()
        )),
        "Unexpected message: {}",
        result.message
    );
}

#[test]
fn test_load_races_from_campaign_populates_races_editor_state() {
    use std::fs;

    use std::time::{SystemTime, UNIX_EPOCH};

    // Build a temporary campaign directory under the system temp dir
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis();
    let tmp_base = std::env::temp_dir();
    let tmpdir = tmp_base.join(format!("antares_test_races_{}", unique));
    let data_dir = tmpdir.join("data");

    // Ensure directory exists
    fs::create_dir_all(&data_dir).expect("Failed to create temp data dir");

    // RON content containing two race definitions (human, elf)
    let races_ron = r#"
[
(
    id: "human",
    name: "Human",
),
(
    id: "elf",
    name: "Elf",
),
]
"#;

    // Write races.ron to the data directory
    let races_path = data_dir.join("races.ron");
    fs::write(&races_path, races_ron).expect("Failed to write races.ron");

    // Setup the CampaignBuilderApp, and point it at our temporary campaign
    let mut app = CampaignBuilderApp::default();
    app.campaign_dir = Some(tmpdir.clone());
    // Use the default "data/races.ron" path; set explicitly to be safe
    app.campaign.races_file = "data/races.ron".to_string();

    // Load races into the editor state (this should populate races_editor_state.races)
    app.load_races_from_campaign();

    // Validate we loaded exactly 2 races and they have the expected IDs
    assert_eq!(app.editor_registry.races_editor_state.races.len(), 2);
    assert_eq!(app.editor_registry.races_editor_state.races[0].id, "human");
    assert_eq!(app.editor_registry.races_editor_state.races[1].id, "elf");

    // Cleanup any temporary files/directories
    let _ = fs::remove_dir_all(tmpdir);
}

#[test]
fn test_validation_empty_id() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "".to_string();
    app.validate_campaign();

    let has_id_error = app
        .validation_state
        .validation_errors
        .iter()
        .any(|e| e.message.contains("Campaign ID is required"));
    assert!(has_id_error);
}

#[test]
fn test_validation_filter_all_shows_passed() {
    let mut app = CampaignBuilderApp::default();

    // Add a passed check to the validation_errors and ensure All shows it
    app.validation_state
        .validation_errors
        .push(validation::ValidationResult::passed(
            validation::ValidationCategory::Items,
            format!("{} items validated", 1),
        ));
    app.validation_state.validation_filter = ValidationFilter::All;

    let grouped = app.grouped_filtered_validation_results();
    let has_passed = grouped.iter().any(|(_cat, results)| {
        results
            .iter()
            .any(|r| r.severity == validation::ValidationSeverity::Passed)
    });
    assert!(
        has_passed,
        "All filter should include passed checks by default"
    );
}

#[test]
fn test_validation_invalid_id_characters() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "invalid-id-with-dashes".to_string();
    app.validate_campaign();

    let has_id_error = app.validation_state.validation_errors.iter().any(|e| {
        e.message
            .contains("alphanumeric characters and underscores")
    });
    assert!(has_id_error);
}

#[test]
fn test_validation_valid_id() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "valid_campaign_123".to_string();
    app.campaign.name = "Valid Campaign".to_string();
    app.campaign.author = "Test Author".to_string();
    app.campaign.starting_map = "test_map".to_string();
    app.validate_campaign();

    let has_id_error = app
        .validation_state
        .validation_errors
        .iter()
        .filter(|e| e.is_error())
        .any(|e| e.message.contains("Campaign ID"));
    assert!(!has_id_error);
}

#[test]
fn test_validation_version_format() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.version = "invalid".to_string();
    app.validate_campaign();

    let has_version_error = app
        .validation_state
        .validation_errors
        .iter()
        .any(|e| e.message.contains("semantic versioning"));
    assert!(has_version_error);
}

#[test]
fn test_validation_roster_size_less_than_party() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();
    app.campaign.name = "Test".to_string();
    app.campaign.starting_map = "test".to_string();
    app.campaign.max_party_size = 10;
    app.campaign.max_roster_size = 5;
    app.validate_campaign();

    let has_roster_error = app
        .validation_state
        .validation_errors
        .iter()
        .any(|e| e.message.contains("roster size must be >= max party size"));
    assert!(has_roster_error);
}

#[test]
fn test_validation_starting_map_missing() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();
    app.campaign.name = "Test".to_string();

    // Add a loaded map (name "Starter Town") so the validator checks against loaded maps
    let map = Map::new(1, "Starter Town".to_string(), "Desc".to_string(), 10, 10);
    app.campaign_data.maps.push(map);

    app.campaign.starting_map = "does_not_exist".to_string();
    app.validate_campaign();

    let has_map_error = app.validation_state.validation_errors.iter().any(|e| {
        e.category == validation::ValidationCategory::Configuration
            && e.is_error()
            && e.message.contains("Starting map")
    });
    assert!(has_map_error);
}

#[test]
fn test_validation_starting_innkeeper_missing() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();
    app.campaign.name = "Test".to_string();
    app.campaign.starting_map = "test_map".to_string();

    // No NPCs loaded - starting_innkeeper should be flagged as missing
    app.campaign.starting_innkeeper = "does_not_exist".to_string();
    app.validate_campaign();

    let has_inn_error = app.validation_state.validation_errors.iter().any(|e| {
        e.category == validation::ValidationCategory::Configuration
            && e.is_error()
            && e.message.contains("Starting innkeeper")
    });
    assert!(has_inn_error);
}

#[test]
fn test_validation_starting_innkeeper_not_innkeeper() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();
    app.campaign.name = "Test".to_string();
    app.campaign.starting_map = "test_map".to_string();

    // Add an NPC that exists but is NOT an innkeeper
    app.editor_registry.npc_editor_state.npcs.push(
        antares::domain::world::npc::NpcDefinition::new(
            "npc_not_inn".to_string(),
            "NPC Not Inn".to_string(),
            "portrait.png".to_string(),
        ),
    );

    app.campaign.starting_innkeeper = "npc_not_inn".to_string();
    app.validate_campaign();

    let has_inn_error = app.validation_state.validation_errors.iter().any(|e| {
        e.category == validation::ValidationCategory::Configuration
            && e.is_error()
            && e.message.contains("is not marked as is_innkeeper")
    });
    assert!(has_inn_error);
}

#[test]
fn test_metadata_editor_validate_triggers_validation_and_switches_tab() {
    let mut app = CampaignBuilderApp::default();

    // Ensure ID/Name are present to avoid unrelated Metadata errors and create a
    // configuration error by leaving starting_map empty.
    app.campaign.id = "test".to_string();
    app.campaign.name = "Test".to_string();
    app.campaign.starting_map = "".to_string();

    // Simulate 'Validate' button click in the Campaign metadata editor by setting
    // the request flag that the editor sets when the UI Validate button is clicked.
    app.editor_registry.campaign_editor_state.validate_requested = true;

    // Behavior in the main app after the editor returns:
    // If a validate request was issued, consume it, run validation, and switch to the Validation tab.
    if app
        .editor_registry
        .campaign_editor_state
        .consume_validate_request()
    {
        app.validate_campaign();
        app.ui_state.active_tab = EditorTab::Validation;
    }

    // After validation, the active tab should be the Validation tab and at least
    // one Configuration category error should be present.
    assert_eq!(app.ui_state.active_tab, EditorTab::Validation);

    let has_config_error = app
        .validation_state
        .validation_errors
        .iter()
        .any(|e| e.category == validation::ValidationCategory::Configuration && e.is_error());

    assert!(has_config_error);
}

#[test]
fn test_validation_starting_level_invalid() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();
    app.campaign.name = "Test".to_string();
    app.campaign.starting_map = "test".to_string();
    app.campaign.starting_level = 0;
    app.validate_campaign();

    let has_level_error = app.validation_state.validation_errors.iter().any(|e| {
        e.message
            .contains("Starting level must be between 1 and max level")
    });
    assert!(has_level_error);
}

#[test]
fn test_validation_starting_map_exists_by_name() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();
    app.campaign.name = "Test".to_string();

    // Add a loaded map (name "Starter Town") so the validator checks against loaded maps
    let map = Map::new(1, "Starter Town".to_string(), "Desc".to_string(), 10, 10);
    app.campaign_data.maps.push(map);

    // Use the normalized map key (underscores -> spaces) that should match the map name
    app.campaign.starting_map = "starter_town".to_string();
    app.validate_campaign();

    // There should NOT be a configuration error related to starting map
    let has_map_error = app.validation_state.validation_errors.iter().any(|e| {
        e.category == validation::ValidationCategory::Configuration
            && e.is_error()
            && e.message.contains("Starting map")
    });
    assert!(!has_map_error);
}

#[test]
fn test_validation_configuration_category_grouping() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();
    app.campaign.name = "Test".to_string();

    // Force a configuration error (empty starting_map)
    app.campaign.starting_map = "".to_string();
    app.validate_campaign();

    let grouped = validation::group_results_by_category(&app.validation_state.validation_errors);
    let has_config_group = grouped
        .iter()
        .any(|(category, _results)| *category == validation::ValidationCategory::Configuration);
    assert!(has_config_group);
}

#[test]
fn test_validation_filter_errors_only() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();
    app.campaign.name = "Test".to_string();

    // Add an error and a warning in the same category
    app.validation_state.validation_errors.clear();
    app.validation_state
        .validation_errors
        .push(validation::ValidationResult::error(
            validation::ValidationCategory::Items,
            "Item ID duplicate",
        ));
    app.validation_state
        .validation_errors
        .push(validation::ValidationResult::warning(
            validation::ValidationCategory::Items,
            "Item name recommended",
        ));

    // Enable Errors Only filter
    app.validation_state.validation_filter = ValidationFilter::ErrorsOnly;
    let _grouped = validation::group_results_by_category(&app.validation_state.validation_errors);

    // Use the grouped & filtered results (matching what the UI shows)
    let grouped2 = app.grouped_filtered_validation_results();
    let visible: usize = grouped2
        .iter()
        .map(|(_category, results)| results.len())
        .sum();

    assert_eq!(visible, 1, "Filter should show only the error result");
}

#[test]
fn test_validation_filter_warnings_only() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();
    app.campaign.name = "Test".to_string();

    // Add an error and a warning in the same category
    app.validation_state.validation_errors.clear();
    app.validation_state
        .validation_errors
        .push(validation::ValidationResult::error(
            validation::ValidationCategory::Items,
            "Item ID duplicate",
        ));
    app.validation_state
        .validation_errors
        .push(validation::ValidationResult::warning(
            validation::ValidationCategory::Items,
            "Item name recommended",
        ));

    // Enable Warnings Only filter
    app.validation_state.validation_filter = ValidationFilter::WarningsOnly;

    let grouped2 = app.grouped_filtered_validation_results();
    let visible: usize = grouped2
        .iter()
        .map(|(_category, results)| results.len())
        .sum();

    assert_eq!(visible, 1, "Filter should show only the warning result");
}

#[test]
fn test_validation_focus_asset_click_sets_state() {
    use std::path::PathBuf;
    let mut app = CampaignBuilderApp::default();

    // Create a temporary campaign directory and write a dummy asset file
    use std::time::{SystemTime, UNIX_EPOCH};
    let tmp_base = std::env::temp_dir();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis();
    let tmpdir = tmp_base.join(format!("antares_test_assets_{}", unique));
    std::fs::create_dir_all(tmpdir.join("data")).expect("Failed to create temp data dir");

    let item_file = tmpdir.join("data").join("items.ron");
    std::fs::write(&item_file, "[]").expect("Failed to write dummy items.ron");

    let mut manager = asset_manager::AssetManager::new(tmpdir.clone());
    manager.scan_directory().expect("scan_directory failed");
    app.asset_manager = Some(manager);

    let asset_path = PathBuf::from("data/items.ron");

    // Simulate clicking the file path (invoking the focus method directly)
    app.focus_asset(asset_path.clone());

    assert!(app.ui_state.show_asset_manager);
    assert_eq!(
        app.validation_state.validation_focus_asset,
        Some(asset_path.clone())
    );
}

#[test]
fn test_validation_reset_filters_clears_state() {
    use std::path::PathBuf;
    let mut app = CampaignBuilderApp::default();
    app.validation_state.validation_filter = ValidationFilter::ErrorsOnly;
    app.validation_state.validation_focus_asset = Some(PathBuf::from("data/items.ron"));

    // Call the reset helper and verify all state is reverted
    app.reset_validation_filters();

    assert_eq!(
        app.validation_state.validation_filter,
        ValidationFilter::All
    );
    assert_eq!(app.validation_state.validation_focus_asset, None);
}

#[test]
fn test_validation_starting_food_invalid() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();
    app.campaign.name = "Test".to_string();
    app.campaign.starting_map = "test".to_string();
    app.campaign.starting_food = (FOOD_MAX as u32) + 10;
    app.validate_campaign();

    let has_food_warning = app.validation_state.validation_errors.iter().any(|e| {
        e.category == validation::ValidationCategory::Configuration
            && e.is_warning()
            && e.message.contains("Starting food")
    });
    assert!(has_food_warning);
}

#[test]
fn test_validation_file_paths_empty() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();
    app.campaign.name = "Test".to_string();
    app.campaign.starting_map = "test".to_string();
    app.campaign.items_file = "".to_string();
    app.validate_campaign();

    let has_path_error = app
        .validation_state
        .validation_errors
        .iter()
        .any(|e| e.message.contains("Items file path is required"));
    assert!(has_path_error);
}

#[test]
fn test_validation_file_paths_wrong_extension() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();
    app.campaign.name = "Test".to_string();
    app.campaign.starting_map = "test".to_string();
    app.campaign.items_file = "data/items.json".to_string();
    app.validate_campaign();

    let has_extension_warning = app
        .validation_state
        .validation_errors
        .iter()
        .any(|e| e.message.contains("should use .ron extension"));
    assert!(has_extension_warning);
}

#[test]
fn test_validation_all_pass() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test_campaign".to_string();
    app.campaign.name = "Test Campaign".to_string();
    app.campaign.author = "Test Author".to_string();
    app.campaign.version = "1.0.0".to_string();
    app.campaign.engine_version = "0.1.0".to_string();
    app.campaign.starting_map = "test_map".to_string();
    app.validate_campaign();

    let error_count = app
        .validation_state
        .validation_errors
        .iter()
        .filter(|e| e.is_error())
        .count();
    assert_eq!(error_count, 0);
}

#[test]
fn test_validate_character_ids_duplicate() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();

    // Add two characters with the same ID
    let char1 = CharacterDefinition::new(
        "char_1".to_string(),
        "Hero".to_string(),
        "race_1".to_string(),
        "class_1".to_string(),
        Sex::Male,
        Alignment::Neutral,
    );
    let char2 = CharacterDefinition::new(
        "char_1".to_string(), // Duplicate ID
        "Another Hero".to_string(),
        "race_1".to_string(),
        "class_1".to_string(),
        Sex::Male,
        Alignment::Neutral,
    );

    app.editor_registry
        .characters_editor_state
        .characters
        .push(char1);
    app.editor_registry
        .characters_editor_state
        .characters
        .push(char2);

    let results = app.validate_character_ids();
    let has_duplicate_error = results
        .iter()
        .any(|r| r.is_error() && r.message.contains("Duplicate character ID"));
    assert!(has_duplicate_error);
}

#[test]
fn test_validate_character_ids_empty_id() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();

    let char = CharacterDefinition::new(
        "".to_string(), // Empty ID
        "Hero".to_string(),
        "race_1".to_string(),
        "class_1".to_string(),
        Sex::Male,
        Alignment::Neutral,
    );

    app.editor_registry
        .characters_editor_state
        .characters
        .push(char);

    let results = app.validate_character_ids();
    let has_empty_id_error = results
        .iter()
        .any(|r| r.is_error() && r.message.contains("empty ID"));
    assert!(has_empty_id_error);
}

#[test]
fn test_validate_character_ids_empty_name_warning() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();

    let char = CharacterDefinition::new(
        "char_1".to_string(),
        "".to_string(), // Empty name
        "race_1".to_string(),
        "class_1".to_string(),
        Sex::Male,
        Alignment::Neutral,
    );

    app.editor_registry
        .characters_editor_state
        .characters
        .push(char);

    let results = app.validate_character_ids();
    let has_name_warning = results.iter().any(|r| {
        r.severity == validation::ValidationSeverity::Warning && r.message.contains("empty name")
    });
    assert!(has_name_warning);
}

#[test]
fn test_validate_character_ids_invalid_class_reference() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();

    let char = CharacterDefinition::new(
        "char_1".to_string(),
        "Hero".to_string(),
        "race_1".to_string(),
        "nonexistent_class".to_string(),
        Sex::Male,
        Alignment::Neutral,
    );

    app.editor_registry
        .characters_editor_state
        .characters
        .push(char);

    let results = app.validate_character_ids();
    let has_class_error = results
        .iter()
        .any(|r| r.is_error() && r.message.contains("non-existent class"));
    assert!(has_class_error);
}

#[test]
fn test_validate_character_ids_invalid_race_reference() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();

    let char = CharacterDefinition::new(
        "char_1".to_string(),
        "Hero".to_string(),
        "nonexistent_race".to_string(),
        "class_1".to_string(),
        Sex::Male,
        Alignment::Neutral,
    );

    app.editor_registry
        .characters_editor_state
        .characters
        .push(char);

    let results = app.validate_character_ids();
    let has_race_error = results
        .iter()
        .any(|r| r.is_error() && r.message.contains("non-existent race"));
    assert!(has_race_error);
}

#[test]
fn test_validate_character_ids_valid() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();

    // Add a class and race that the character can reference
    app.editor_registry
        .classes_editor_state
        .classes
        .push(ClassDefinition::new(
            "class_1".to_string(),
            "Knight".to_string(),
        ));

    app.editor_registry
        .races_editor_state
        .races
        .push(RaceDefinition::new(
            "race_1".to_string(),
            "Human".to_string(),
            "A balanced race".to_string(),
        ));

    let char = CharacterDefinition::new(
        "char_1".to_string(),
        "Hero".to_string(),
        "race_1".to_string(),
        "class_1".to_string(),
        Sex::Male,
        Alignment::Neutral,
    );

    app.editor_registry
        .characters_editor_state
        .characters
        .push(char);

    let results = app.validate_character_ids();
    let has_pass = results
        .iter()
        .any(|r| r.severity == validation::ValidationSeverity::Passed);
    assert!(has_pass);
}

#[test]
fn test_validate_proficiency_ids_duplicate() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();

    // Add two proficiencies with the same ID
    app.campaign_data.proficiencies.push(ProficiencyDefinition {
        id: "prof_1".to_string(),
        name: "Longsword".to_string(),
        category: antares::domain::proficiency::ProficiencyCategory::Weapon,
        description: String::new(),
    });

    app.campaign_data.proficiencies.push(ProficiencyDefinition {
        id: "prof_1".to_string(), // Duplicate ID
        name: "Shortsword".to_string(),
        category: antares::domain::proficiency::ProficiencyCategory::Weapon,
        description: String::new(),
    });

    let results = app.validate_proficiency_ids();
    let has_duplicate_error = results
        .iter()
        .any(|r| r.is_error() && r.message.contains("Duplicate proficiency ID"));
    assert!(has_duplicate_error);
}

#[test]
fn test_validate_proficiency_ids_empty_id() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();

    app.campaign_data.proficiencies.push(ProficiencyDefinition {
        id: "".to_string(), // Empty ID
        name: "Longsword".to_string(),
        category: antares::domain::proficiency::ProficiencyCategory::Weapon,
        description: String::new(),
    });

    let results = app.validate_proficiency_ids();
    let has_empty_id_error = results
        .iter()
        .any(|r| r.is_error() && r.message.contains("empty ID"));
    assert!(has_empty_id_error);
}

#[test]
fn test_validate_proficiency_ids_empty_name_warning() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();

    app.campaign_data.proficiencies.push(ProficiencyDefinition {
        id: "prof_1".to_string(),
        name: "".to_string(), // Empty name
        category: antares::domain::proficiency::ProficiencyCategory::Weapon,
        description: String::new(),
    });

    let results = app.validate_proficiency_ids();
    let has_name_warning = results.iter().any(|r| {
        r.severity == validation::ValidationSeverity::Warning && r.message.contains("empty name")
    });
    assert!(has_name_warning);
}

#[test]
fn test_validate_proficiency_ids_referenced_by_class() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();

    // Add a proficiency
    app.campaign_data.proficiencies.push(ProficiencyDefinition {
        id: "prof_1".to_string(),
        name: "Longsword".to_string(),
        category: antares::domain::proficiency::ProficiencyCategory::Weapon,
        description: String::new(),
    });

    // Add a class that references the proficiency
    let mut class = ClassDefinition::new("class_1".to_string(), "Knight".to_string());
    class.proficiencies = vec!["prof_1".to_string()];
    app.editor_registry.classes_editor_state.classes.push(class);

    let results = app.validate_proficiency_ids();
    let has_pass = results
        .iter()
        .any(|r| r.severity == validation::ValidationSeverity::Passed);
    assert!(has_pass);
}

#[test]
fn test_validate_proficiency_ids_class_references_nonexistent() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();

    // Add a class that references a non-existent proficiency
    let mut class = ClassDefinition::new("class_1".to_string(), "Knight".to_string());
    class.proficiencies = vec!["nonexistent_prof".to_string()];
    app.editor_registry.classes_editor_state.classes.push(class);

    let results = app.validate_proficiency_ids();
    let has_error = results
        .iter()
        .any(|r| r.is_error() && r.message.contains("references non-existent proficiency"));
    assert!(has_error);
}

#[test]
fn test_validate_proficiency_ids_race_references_nonexistent() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();

    // Add a race that references a non-existent proficiency
    let mut race = RaceDefinition::new("race_1".to_string(), "Human".to_string(), String::new());
    race.proficiencies = vec!["nonexistent_prof".to_string()];
    app.editor_registry.races_editor_state.races.push(race);

    let results = app.validate_proficiency_ids();
    let has_error = results
        .iter()
        .any(|r| r.is_error() && r.message.contains("references non-existent proficiency"));
    assert!(has_error);
}

#[test]
fn test_validate_proficiency_ids_item_requires_nonexistent() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();

    // Add an item that requires a non-existent proficiency
    let mut item = ItemsEditorState::default_item();
    item.item_type = ItemType::Weapon(WeaponData {
        damage: DiceRoll::new(1, 6, 0),
        bonus: 0,
        hands_required: 1,
        classification: WeaponClassification::MartialMelee,
    });
    // "martial_melee" is not in app.campaign_data.proficiencies by default
    app.campaign_data.items.push(item);

    let results = app.validate_proficiency_ids();
    let has_error = results
        .iter()
        .any(|r| r.is_error() && r.message.contains("requires non-existent proficiency"));
    assert!(has_error);
}

#[test]
fn test_validate_proficiency_ids_unreferenced_info() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test".to_string();

    // Add a proficiency that is not referenced by anything
    app.campaign_data.proficiencies.push(ProficiencyDefinition {
        id: "unused_prof".to_string(),
        name: "Unused".to_string(),
        category: antares::domain::proficiency::ProficiencyCategory::Weapon,
        description: String::new(),
    });

    let results = app.validate_proficiency_ids();
    let has_info = results.iter().any(|r| {
        r.severity == validation::ValidationSeverity::Info && r.message.contains("not used")
    });
    assert!(has_info);
}

#[test]
fn test_validation_all_shows_passed_and_errors_only_filters_out_passed() {
    // Ensure 'All' filter shows Passed results by default and 'Errors Only' filters them out.
    let mut app = CampaignBuilderApp::default();

    // Add a single 'Passed' validation result for the Items category.
    app.validation_state
        .validation_errors
        .push(validation::ValidationResult::passed(
            validation::ValidationCategory::Items,
            "All items validated",
        ));

    // Default filter is All; verify passed checks are included.
    app.validation_state.validation_filter = ValidationFilter::All;
    let grouped_all = app.grouped_filtered_validation_results();
    let has_passed_in_all = grouped_all.iter().any(|(_cat, results)| {
        results
            .iter()
            .any(|r| r.severity == validation::ValidationSeverity::Passed)
    });
    assert!(
        has_passed_in_all,
        "All filter should include passed checks by default"
    );

    // Switch to ErrorsOnly filter and verify passed checks are excluded.
    app.validation_state.validation_filter = ValidationFilter::ErrorsOnly;
    let grouped_errors = app.grouped_filtered_validation_results();
    let has_passed_in_errors = grouped_errors.iter().any(|(_cat, results)| {
        results
            .iter()
            .any(|r| r.severity == validation::ValidationSeverity::Passed)
    });
    assert!(
        !has_passed_in_errors,
        "ErrorsOnly filter should exclude passed checks"
    );
}

#[test]
fn test_save_campaign_no_path() {
    let mut app = CampaignBuilderApp::default();
    let result = app.save_campaign();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), CampaignError::NoPath));
}

// ── Stock-template wipe-prevention regression test ────────────────────────
//
// Root cause: `do_save_campaign` was calling `save_to_file` unconditionally,
// so the default empty `Vec<MerchantStockTemplate>` (present before any load)
// was written to disk as `[]`, wiping whatever the file contained.
//
// Fix: the save is now guarded by `loaded_from_file || has_unsaved_changes`.
// This test confirms the guard holds: a campaign save with templates that
// were *never loaded* must not touch the on-disk file.
#[test]
fn test_do_save_campaign_does_not_overwrite_stock_templates_when_not_loaded() {
    use tempfile::tempdir;

    let dir = tempdir().expect("tempdir");
    let campaign_path = dir.path().join("campaign.ron");

    // --- Write a minimal valid campaign.ron ---
    let campaign_ron = r#"CampaignConfig(
id: "wipe_test",
name: "Wipe Test",
version: "0.1.0",
author: "Tester",
description: "",
engine_version: "0.1.0",
starting_map: "map_1",
starting_position: (0, 0),
starting_direction: "North",
starting_gold: 100,
starting_food: 10,
starting_innkeeper: "",
max_party_size: 6,
max_roster_size: 20,
difficulty: Normal,
permadeath: false,
allow_multiclassing: false,
starting_level: 1,
max_level: 20,
items_file: "data/items.ron",
spells_file: "data/spells.ron",
monsters_file: "data/monsters.ron",
classes_file: "data/classes.ron",
races_file: "data/races.ron",
npcs_file: "data/npcs.ron",
maps_file: "data/maps.ron",
characters_file: "data/characters.ron",
conditions_file: "data/conditions.ron",
quests_file: "data/quests.ron",
dialogue_file: "data/dialogues.ron",
proficiencies_file: "data/proficiencies.ron",
creatures_file: "data/creatures.ron",
stock_templates_file: "data/npc_stock_templates.ron",
starting_time: GameTime(year: 1, month: 1, day: 1, hour: 8, minute: 0),
)"#;
    std::fs::write(&campaign_path, campaign_ron).expect("write campaign.ron");

    // --- Write a non-empty stock-templates file ---
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let templates_path = data_dir.join("npc_stock_templates.ron");
    let original_content = r#"[MerchantStockTemplate(id:"sentinel_template",entries:[],magic_item_pool:[],magic_slot_count:0,magic_refresh_days:7)]"#;
    std::fs::write(&templates_path, original_content).expect("write templates");

    // --- Build app state that mirrors "opened campaign, never visited
    //     Stock Templates tab, stock_templates_editor_state.loaded_from_file = false" ---
    let mut app = CampaignBuilderApp::default();
    app.campaign_path = Some(campaign_path.clone());
    app.campaign_dir = Some(dir.path().to_path_buf());
    // stock_templates_editor_state starts with loaded_from_file = false
    // and templates = Vec::new() — the exact pre-bug state.
    assert!(
        !app.editor_registry
            .stock_templates_editor_state
            .loaded_from_file,
        "loaded_from_file must be false before any load"
    );
    assert!(
        app.editor_registry
            .stock_templates_editor_state
            .templates
            .is_empty(),
        "templates must be empty before any load"
    );

    // --- Trigger the campaign save ---
    // We use do_save_campaign directly because save_campaign checks for a
    // valid campaign_path (already set above).  The RON parse for the
    // minimal campaign.ron above may fail due to missing fields; we only
    // care that the stock-templates file is untouched.
    let _ = app.do_save_campaign();

    // --- Assert the stock-templates file was NOT overwritten ---
    let on_disk = std::fs::read_to_string(&templates_path).expect("read back");
    assert_eq!(
        on_disk, original_content,
        "do_save_campaign must not overwrite npc_stock_templates.ron \
         when loaded_from_file = false and has_unsaved_changes = false"
    );
}

// ===== ID Validation and Generation Tests =====

#[test]
fn test_item_id_uniqueness_validation() {
    let mut app = CampaignBuilderApp::default();

    // Add items with duplicate IDs
    let mut item1 = CampaignBuilderApp::default_item();
    item1.id = 1;
    item1.name = "Item 1".to_string();
    app.campaign_data.items.push(item1);

    let mut item2 = CampaignBuilderApp::default_item();
    item2.id = 1; // Duplicate ID
    item2.name = "Item 2".to_string();
    app.campaign_data.items.push(item2);

    let mut item3 = CampaignBuilderApp::default_item();
    item3.id = 2;
    item3.name = "Item 3".to_string();
    app.campaign_data.items.push(item3);

    // Validate
    let errors = app.validate_item_ids();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].is_error());
    assert_eq!(errors[0].category, validation::ValidationCategory::Items);
    assert!(errors[0].message.contains("Duplicate item ID: 1"));
}

#[test]
fn test_spell_id_uniqueness_validation() {
    let mut app = CampaignBuilderApp::default();

    // Add spells with duplicate IDs
    let mut spell1 = CampaignBuilderApp::default_spell();
    spell1.id = 100;
    spell1.name = "Spell 1".to_string();
    app.campaign_data.spells.push(spell1);

    let mut spell2 = CampaignBuilderApp::default_spell();
    spell2.id = 100; // Duplicate ID
    spell2.name = "Spell 2".to_string();
    app.campaign_data.spells.push(spell2);

    // Validate
    let errors = app.validate_spell_ids();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].is_error());
    assert_eq!(errors[0].category, validation::ValidationCategory::Spells);
    assert!(errors[0].message.contains("Duplicate spell ID: 100"));
}

#[test]
fn test_monster_id_uniqueness_validation() {
    let mut app = CampaignBuilderApp::default();

    // Add monsters with duplicate IDs
    let mut monster1 = CampaignBuilderApp::default_monster();
    monster1.id = 5;
    monster1.name = "Monster 1".to_string();
    app.campaign_data.monsters.push(monster1);

    let mut monster2 = CampaignBuilderApp::default_monster();
    monster2.id = 5; // Duplicate ID
    monster2.name = "Monster 2".to_string();
    app.campaign_data.monsters.push(monster2);

    // Validate
    let errors = app.validate_monster_ids();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].is_error());
    assert_eq!(errors[0].category, validation::ValidationCategory::Monsters);
    assert!(errors[0].message.contains("Duplicate monster ID: 5"));
}

#[test]
fn test_map_id_uniqueness_validation() {
    let mut app = CampaignBuilderApp::default();

    // Add maps with duplicate IDs
    let map1 = Map::new(10, "Map 1".to_string(), "Desc 1".to_string(), 20, 20);
    app.campaign_data.maps.push(map1);

    let map2 = Map::new(10, "Map 2".to_string(), "Desc 2".to_string(), 30, 30); // Duplicate ID
    app.campaign_data.maps.push(map2);

    // Validate
    let errors = app.validate_map_ids();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].is_error());
    assert_eq!(errors[0].category, validation::ValidationCategory::Maps);
    assert!(errors[0].message.contains("Duplicate map ID: 10"));
}

#[test]
fn test_condition_id_uniqueness_validation() {
    let mut app = CampaignBuilderApp::default();

    let cond1 = ConditionDefinition {
        id: "dup_test".to_string(),
        name: "Duplicate 1".to_string(),
        description: "".to_string(),
        effects: vec![],
        default_duration: antares::domain::conditions::ConditionDuration::Rounds(3),
        icon_id: None,
    };

    let mut cond2 = cond1.clone();
    cond2.name = "Duplicate 2".to_string();
    app.campaign_data.conditions.push(cond1);
    app.campaign_data.conditions.push(cond2);

    // Validate
    let errors = app.validate_condition_ids();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].is_error());
    assert_eq!(
        errors[0].category,
        validation::ValidationCategory::Conditions
    );
    assert!(errors[0]
        .message
        .contains("Duplicate condition ID: dup_test"));
}

#[test]
fn test_validate_campaign_includes_id_checks() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test_campaign".to_string();
    app.campaign.name = "Test Campaign".to_string();
    app.campaign.version = "1.0.0".to_string();
    app.campaign.engine_version = "0.1.0".to_string();
    app.campaign.starting_map = "start".to_string();

    // Add duplicate item IDs
    let mut item1 = CampaignBuilderApp::default_item();
    item1.id = 1;
    app.campaign_data.items.push(item1);

    let mut item2 = CampaignBuilderApp::default_item();
    item2.id = 1; // Duplicate
    app.campaign_data.items.push(item2);

    // Run validation
    app.validate_campaign();

    // Should have error for duplicate item ID
    let has_duplicate_error = app
        .validation_state
        .validation_errors
        .iter()
        .any(|e| e.message.contains("Duplicate item ID"));
    assert!(has_duplicate_error);
}

#[test]
fn test_no_duplicate_ids_validation_passes() {
    let mut app = CampaignBuilderApp::default();

    // Add items with unique IDs
    let mut item1 = CampaignBuilderApp::default_item();
    item1.id = 1;
    app.campaign_data.items.push(item1);

    let mut item2 = CampaignBuilderApp::default_item();
    item2.id = 2;
    app.campaign_data.items.push(item2);

    // Validate
    let errors = app.validate_item_ids();
    assert_eq!(errors.len(), 0);
}

// =========================================================================
// validate_character_starting_spells integration tests
// =========================================================================

#[test]
fn test_validate_campaign_character_invalid_starting_spell_produces_error() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test_campaign".to_string();
    app.campaign.name = "Test Campaign".to_string();
    app.campaign.version = "1.0.0".to_string();
    app.campaign.engine_version = "0.1.0".to_string();
    app.campaign.starting_map = "start".to_string();

    // Add a spell with ID 100 to the campaign
    let mut known_spell = CampaignBuilderApp::default_spell();
    known_spell.id = 100;
    known_spell.name = "Known Spell".to_string();
    app.campaign_data.spells.push(known_spell);

    // Add a character that references a spell ID not in the campaign (ID 9999)
    let mut char_def = CharacterDefinition::new(
        "test_char".to_string(),
        "Test Character".to_string(),
        "human".to_string(),
        "knight".to_string(),
        Sex::Male,
        Alignment::Neutral,
    );
    char_def.starting_spells = vec![9999];
    app.editor_registry
        .characters_editor_state
        .characters
        .push(char_def);

    app.validate_campaign();

    let has_spell_error = app
        .validation_state
        .validation_errors
        .iter()
        .any(|e| e.is_error() && e.message.contains("unknown spell ID"));
    assert!(
        has_spell_error,
        "validate_campaign() must surface an error for a character with an invalid starting spell ID"
    );
}

#[test]
fn test_validate_campaign_character_valid_starting_spell_no_error() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test_campaign".to_string();
    app.campaign.name = "Test Campaign".to_string();
    app.campaign.version = "1.0.0".to_string();
    app.campaign.engine_version = "0.1.0".to_string();
    app.campaign.starting_map = "start".to_string();

    // Add a spell with ID 200 to the campaign
    let mut known_spell = CampaignBuilderApp::default_spell();
    known_spell.id = 200;
    known_spell.name = "Valid Spell".to_string();
    app.campaign_data.spells.push(known_spell);

    // Add a character that references the valid spell ID
    let mut char_def = CharacterDefinition::new(
        "spell_char".to_string(),
        "Spell Character".to_string(),
        "human".to_string(),
        "knight".to_string(),
        Sex::Male,
        Alignment::Neutral,
    );
    char_def.starting_spells = vec![200];
    app.editor_registry
        .characters_editor_state
        .characters
        .push(char_def);

    app.validate_campaign();

    let has_starting_spell_error = app
        .validation_state
        .validation_errors
        .iter()
        .any(|e| e.is_error() && e.message.contains("starting_spells"));
    assert!(
        !has_starting_spell_error,
        "A character whose starting_spells all resolve should not produce a starting_spells error"
    );
}

#[test]
fn test_validate_campaign_character_empty_starting_spells_no_error() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test_campaign".to_string();
    app.campaign.name = "Test Campaign".to_string();
    app.campaign.version = "1.0.0".to_string();
    app.campaign.engine_version = "0.1.0".to_string();
    app.campaign.starting_map = "start".to_string();

    // Add a character with no starting spells
    let char_def = CharacterDefinition::new(
        "no_spell_char".to_string(),
        "No Spell Character".to_string(),
        "human".to_string(),
        "knight".to_string(),
        Sex::Male,
        Alignment::Neutral,
    );
    app.editor_registry
        .characters_editor_state
        .characters
        .push(char_def);

    app.validate_campaign();

    let has_starting_spell_error = app
        .validation_state
        .validation_errors
        .iter()
        .any(|e| e.is_error() && e.message.contains("starting_spells"));
    assert!(
        !has_starting_spell_error,
        "A character with empty starting_spells should not produce a starting_spells error"
    );
}

#[test]
fn test_validate_campaign_multiple_characters_one_invalid_starting_spell() {
    let mut app = CampaignBuilderApp::default();
    app.campaign.id = "test_campaign".to_string();
    app.campaign.name = "Test Campaign".to_string();
    app.campaign.version = "1.0.0".to_string();
    app.campaign.engine_version = "0.1.0".to_string();
    app.campaign.starting_map = "start".to_string();

    // Add a spell with ID 300
    let mut known_spell = CampaignBuilderApp::default_spell();
    known_spell.id = 300;
    known_spell.name = "Fireball".to_string();
    app.campaign_data.spells.push(known_spell);

    // Character 1: valid reference
    let mut char1 = CharacterDefinition::new(
        "char_valid".to_string(),
        "Valid Char".to_string(),
        "human".to_string(),
        "knight".to_string(),
        Sex::Male,
        Alignment::Good,
    );
    char1.starting_spells = vec![300];
    app.editor_registry
        .characters_editor_state
        .characters
        .push(char1);

    // Character 2: invalid reference (spell ID 8888 not in campaign)
    let mut char2 = CharacterDefinition::new(
        "char_invalid".to_string(),
        "Invalid Char".to_string(),
        "human".to_string(),
        "knight".to_string(),
        Sex::Female,
        Alignment::Evil,
    );
    char2.starting_spells = vec![8888];
    app.editor_registry
        .characters_editor_state
        .characters
        .push(char2);

    app.validate_campaign();

    let spell_errors: Vec<_> = app
        .validation_state
        .validation_errors
        .iter()
        .filter(|e| e.is_error() && e.message.contains("unknown spell ID"))
        .collect();

    assert_eq!(
        spell_errors.len(),
        1,
        "Only the one invalid starting spell reference should produce an error"
    );
    assert!(
        spell_errors[0].message.contains("Invalid Char"),
        "The error should name the character with the invalid spell"
    );
}
