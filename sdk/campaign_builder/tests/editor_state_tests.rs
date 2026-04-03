// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Editor state management, defaults, filter, compliance checker,
//! and creature template tests.

use antares::domain::character::{AttributePair, AttributePair16, Stats};
use antares::domain::combat::database::MonsterDefinition;
use antares::domain::combat::monster::{LootTable, MonsterCondition, MonsterResistances};
use antares::domain::combat::types::{Attack, AttackType};
use antares::domain::combat::SpecialEffect;
use antares::domain::conditions::ConditionDefinition;
use antares::domain::dialogue::DialogueTree;
use antares::domain::items::types::{ItemType, WeaponClassification, WeaponData};
use antares::domain::items::ArmorClassification;
use antares::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};
use antares::domain::quest::{Quest, QuestStage};
use antares::domain::types::DiceRoll;
use antares::domain::world::Map;
use campaign_builder::*;

#[test]
fn test_campaign_metadata_default() {
    let campaign = CampaignMetadata::default();
    assert_eq!(campaign.id, "");
    assert_eq!(campaign.name, "");
    assert_eq!(campaign.version, "1.0.0");
    assert_eq!(campaign.engine_version, "0.1.0");
    assert_eq!(campaign.starting_map, "starter_town");
    assert_eq!(campaign.starting_position, (10, 10));
    assert_eq!(campaign.max_party_size, 6);
    assert_eq!(campaign.max_roster_size, 20);
    assert_eq!(campaign.difficulty, Difficulty::Normal);
    assert!(!campaign.permadeath);
    assert!(!campaign.allow_multiclassing);
    assert_eq!(campaign.starting_level, 1);
    assert_eq!(campaign.max_level, 20);
}

#[test]
fn test_difficulty_as_str() {
    assert_eq!(Difficulty::Easy.as_str(), "Easy");
    assert_eq!(Difficulty::Normal.as_str(), "Normal");
    assert_eq!(Difficulty::Hard.as_str(), "Hard");
    assert_eq!(Difficulty::Brutal.as_str(), "Brutal");
}

#[test]
fn test_difficulty_default() {
    let diff: Difficulty = Default::default();
    assert_eq!(diff, Difficulty::Normal);
}

#[test]
fn test_default_starting_innkeeper() {
    let metadata = CampaignMetadata::default();
    assert_eq!(
        metadata.starting_innkeeper,
        "tutorial_innkeeper_town".to_string()
    );
}

#[test]
fn test_unsaved_changes_tracking() {
    let mut app = CampaignBuilderApp::default();
    assert!(!app.unsaved_changes);

    // Simulate a change
    app.campaign.name = "Changed".to_string();
    app.unsaved_changes = true;
    assert!(app.unsaved_changes);
}

#[test]
fn test_editor_tab_names() {
    assert_eq!(EditorTab::Metadata.name(), "Metadata");
    assert_eq!(EditorTab::Items.name(), "Items");
    assert_eq!(EditorTab::Spells.name(), "Spells");
    assert_eq!(EditorTab::Monsters.name(), "Monsters");
    assert_eq!(EditorTab::Creatures.name(), "Creatures");
    assert_eq!(EditorTab::Maps.name(), "Maps");
    assert_eq!(EditorTab::Quests.name(), "Quests");
    assert_eq!(EditorTab::Dialogues.name(), "Dialogues");
    assert_eq!(EditorTab::Assets.name(), "Assets");
    assert_eq!(EditorTab::Validation.name(), "Validation");
}

#[test]
fn test_severity_icons() {
    assert_eq!(validation::ValidationSeverity::Error.icon(), "❌");
    assert_eq!(validation::ValidationSeverity::Warning.icon(), "⚠️");
    assert_eq!(validation::ValidationSeverity::Passed.icon(), "✅");
    assert_eq!(validation::ValidationSeverity::Info.icon(), "ℹ️");
}

#[test]
fn test_validation_result_creation() {
    let error =
        validation::ValidationResult::error(validation::ValidationCategory::Metadata, "Test error");
    assert!(error.is_error());
    assert_eq!(error.message, "Test error");
    assert_eq!(error.category, validation::ValidationCategory::Metadata);
}

#[test]
fn test_editor_mode_transitions() {
    let mut app = CampaignBuilderApp::default();
    assert_eq!(
        app.editor_registry.items_editor_state.mode,
        items_editor::ItemsEditorMode::List
    );

    // Simulate adding an item
    app.editor_registry.items_editor_state.mode = items_editor::ItemsEditorMode::Add;
    assert_eq!(
        app.editor_registry.items_editor_state.mode,
        items_editor::ItemsEditorMode::Add
    );

    // Simulate editing an item
    app.editor_registry.items_editor_state.mode = items_editor::ItemsEditorMode::Edit;
    assert_eq!(
        app.editor_registry.items_editor_state.mode,
        items_editor::ItemsEditorMode::Edit
    );

    // Return to list
    app.editor_registry.items_editor_state.mode = items_editor::ItemsEditorMode::List;
    assert_eq!(
        app.editor_registry.items_editor_state.mode,
        items_editor::ItemsEditorMode::List
    );
}

#[test]
fn test_default_item_creation() {
    let item = CampaignBuilderApp::default_item();
    assert_eq!(item.id, 0);
    assert_eq!(item.name, "");
    assert!(matches!(item.item_type, ItemType::Weapon(_)));
    assert_eq!(item.base_cost, 0);
    assert_eq!(item.sell_cost, 0);
    assert!(!item.is_cursed);
}

#[test]
fn test_default_spell_creation() {
    let spell = CampaignBuilderApp::default_spell();
    assert_eq!(spell.id, 0);
    assert_eq!(spell.name, "");
    assert_eq!(spell.school, SpellSchool::Cleric);
    assert_eq!(spell.level, 1);
    assert_eq!(spell.sp_cost, 1);
    assert_eq!(spell.gem_cost, 0);
}

#[test]
fn test_default_monster_creation() {
    let monster = CampaignBuilderApp::default_monster();
    assert_eq!(monster.id, 0);
    assert_eq!(monster.name, "");
    assert_eq!(monster.hp.base, 10);
    assert_eq!(monster.ac.base, 10);
    assert!(!monster.is_undead);
    assert!(!monster.can_regenerate);
    assert!(monster.can_advance);
    assert_eq!(monster.magic_resistance, 0);
}

#[test]
fn test_items_data_structure_initialization() {
    let app = CampaignBuilderApp::default();
    assert_eq!(app.campaign_data.items.len(), 0);
    assert_eq!(app.editor_registry.items_editor_state.search_query, "");
    assert_eq!(app.editor_registry.items_editor_state.selected_item, None);
    assert_eq!(
        app.editor_registry.items_editor_state.mode,
        items_editor::ItemsEditorMode::List
    );
}

#[test]
fn test_spells_data_structure_initialization() {
    let app = CampaignBuilderApp::default();
    assert_eq!(app.campaign_data.spells.len(), 0);
    assert_eq!(app.editor_registry.spells_editor_state.search_query, "");
    assert_eq!(app.editor_registry.spells_editor_state.selected_spell, None);
    assert_eq!(
        app.editor_registry.spells_editor_state.mode,
        spells_editor::SpellsEditorMode::List
    );
}

#[test]
fn test_monsters_data_structure_initialization() {
    let app = CampaignBuilderApp::default();
    assert_eq!(app.campaign_data.monsters.len(), 0);
    assert_eq!(app.editor_registry.monsters_editor_state.search_query, "");
    assert_eq!(
        app.editor_registry.monsters_editor_state.selected_monster,
        None
    );
    assert_eq!(
        app.editor_registry.monsters_editor_state.mode,
        monsters_editor::MonstersEditorMode::List
    );
}

#[test]
fn test_item_type_detection() {
    let item = CampaignBuilderApp::default_item();
    assert!(item.is_weapon());
    assert!(!item.is_armor());
    assert!(!item.is_accessory());
    assert!(!item.is_consumable());
    assert!(!item.is_ammo());
}

#[test]
fn test_quest_objective_editor_initialization() {
    let app = CampaignBuilderApp::default();
    assert_eq!(app.campaign_data.quests.len(), 0);
    assert_eq!(app.editor_registry.quest_editor_state.selected_quest, None);
    assert_eq!(app.editor_registry.quest_editor_state.selected_stage, None);
    assert_eq!(
        app.editor_registry.quest_editor_state.selected_objective,
        None
    );
    assert_eq!(
        app.editor_registry.quest_editor_state.mode,
        quest_editor::QuestEditorMode::List
    );
}

#[test]
fn test_quest_stage_editing_flow() {
    let mut app = CampaignBuilderApp::default();

    // Create a quest with a stage
    let quest = Quest {
        id: 1,
        name: "Test Quest".to_string(),
        description: "Test Description".to_string(),
        is_main_quest: false,
        repeatable: false,
        min_level: None,
        max_level: None,
        required_quests: Vec::new(),
        stages: vec![QuestStage {
            stage_number: 1,
            name: "Stage 1".to_string(),
            description: "Stage 1 description".to_string(),
            require_all_objectives: true,
            objectives: Vec::new(),
        }],
        rewards: Vec::new(),
        quest_giver_npc: None,
        quest_giver_map: None,
        quest_giver_position: None,
    };

    app.campaign_data.quests.push(quest);
    app.editor_registry.quest_editor_state.selected_quest = Some(0);

    // Test edit stage
    let result = app
        .editor_registry
        .quest_editor_state
        .edit_stage(&app.campaign_data.quests, 0, 0);
    assert!(result.is_ok());
    assert_eq!(
        app.editor_registry.quest_editor_state.selected_stage,
        Some(0)
    );
    assert_eq!(
        app.editor_registry.quest_editor_state.stage_buffer.name,
        "Stage 1"
    );
    assert_eq!(
        app.editor_registry
            .quest_editor_state
            .stage_buffer
            .description,
        "Stage 1 description"
    );

    // Test save stage
    app.editor_registry.quest_editor_state.stage_buffer.name = "Updated Stage".to_string();
    let result =
        app.editor_registry
            .quest_editor_state
            .save_stage(&mut app.campaign_data.quests, 0, 0);
    assert!(result.is_ok());
    assert_eq!(app.campaign_data.quests[0].stages[0].name, "Updated Stage");
    assert_eq!(app.editor_registry.quest_editor_state.selected_stage, None);
    assert!(app.editor_registry.quest_editor_state.has_unsaved_changes);
}

#[test]
fn test_quest_objective_editing_flow() {
    let mut app = CampaignBuilderApp::default();

    // Create a quest with a stage and objective
    let quest = Quest {
        id: 1,
        name: "Test Quest".to_string(),
        description: "Test Description".to_string(),
        is_main_quest: false,
        repeatable: false,
        min_level: None,
        max_level: None,
        required_quests: Vec::new(),
        stages: vec![QuestStage {
            stage_number: 1,
            name: "Stage 1".to_string(),
            description: "Stage 1 description".to_string(),
            require_all_objectives: true,
            objectives: vec![antares::domain::quest::QuestObjective::KillMonsters {
                monster_id: 100,
                quantity: 5,
            }],
        }],
        rewards: Vec::new(),
        quest_giver_npc: None,
        quest_giver_map: None,
        quest_giver_position: None,
    };

    app.campaign_data.quests.push(quest);
    app.editor_registry.quest_editor_state.selected_quest = Some(0);

    // Test edit objective
    let result =
        app.editor_registry
            .quest_editor_state
            .edit_objective(&app.campaign_data.quests, 0, 0, 0);
    assert!(result.is_ok());
    assert_eq!(
        app.editor_registry.quest_editor_state.selected_objective,
        Some(0)
    );
    assert_eq!(
        app.editor_registry
            .quest_editor_state
            .objective_buffer
            .objective_type,
        quest_editor::ObjectiveType::KillMonsters
    );
    assert_eq!(
        app.editor_registry
            .quest_editor_state
            .objective_buffer
            .monster_id,
        "100"
    );
    assert_eq!(
        app.editor_registry
            .quest_editor_state
            .objective_buffer
            .quantity,
        "5"
    );

    // Test save objective
    app.editor_registry
        .quest_editor_state
        .objective_buffer
        .quantity = "10".to_string();
    let result = app.editor_registry.quest_editor_state.save_objective(
        &mut app.campaign_data.quests,
        0,
        0,
        0,
    );
    assert!(result.is_ok());

    if let antares::domain::quest::QuestObjective::KillMonsters {
        monster_id: _,
        quantity,
    } = &app.campaign_data.quests[0].stages[0].objectives[0]
    {
        assert_eq!(*quantity, 10);
    } else {
        panic!("Expected KillMonsters objective");
    }

    assert_eq!(
        app.editor_registry.quest_editor_state.selected_objective,
        None
    );
    assert!(app.editor_registry.quest_editor_state.has_unsaved_changes);
}

#[test]
fn test_quest_stage_deletion() {
    let mut app = CampaignBuilderApp::default();

    // Create a quest with two stages
    let quest = Quest {
        id: 1,
        name: "Test Quest".to_string(),
        description: "Test Description".to_string(),
        is_main_quest: false,
        repeatable: false,
        min_level: None,
        max_level: None,
        required_quests: Vec::new(),
        stages: vec![
            QuestStage {
                stage_number: 1,
                name: "Stage 1".to_string(),
                description: "Stage 1 description".to_string(),
                require_all_objectives: true,
                objectives: Vec::new(),
            },
            QuestStage {
                stage_number: 2,
                name: "Stage 2".to_string(),
                description: "Stage 2 description".to_string(),
                require_all_objectives: true,
                objectives: Vec::new(),
            },
        ],
        rewards: Vec::new(),
        quest_giver_npc: None,
        quest_giver_map: None,
        quest_giver_position: None,
    };

    app.campaign_data.quests.push(quest);
    app.editor_registry.quest_editor_state.selected_quest = Some(0);

    // Delete first stage
    assert_eq!(app.campaign_data.quests[0].stages.len(), 2);
    let result =
        app.editor_registry
            .quest_editor_state
            .delete_stage(&mut app.campaign_data.quests, 0, 0);
    assert!(result.is_ok());
    assert_eq!(app.campaign_data.quests[0].stages.len(), 1);
    assert_eq!(app.campaign_data.quests[0].stages[0].name, "Stage 2");
    assert!(app.editor_registry.quest_editor_state.has_unsaved_changes);
}

#[test]
fn test_quest_objective_deletion() {
    let mut app = CampaignBuilderApp::default();

    // Create a quest with a stage and multiple objectives
    let quest = Quest {
        id: 1,
        name: "Test Quest".to_string(),
        description: "Test Description".to_string(),
        is_main_quest: false,
        repeatable: false,
        min_level: None,
        max_level: None,
        required_quests: Vec::new(),
        stages: vec![QuestStage {
            stage_number: 1,
            name: "Stage 1".to_string(),
            description: "Stage 1 description".to_string(),
            require_all_objectives: true,
            objectives: vec![
                antares::domain::quest::QuestObjective::KillMonsters {
                    monster_id: 100,
                    quantity: 5,
                },
                antares::domain::quest::QuestObjective::CollectItems {
                    item_id: 200,
                    quantity: 3,
                },
            ],
        }],
        rewards: Vec::new(),
        quest_giver_npc: None,
        quest_giver_map: None,
        quest_giver_position: None,
    };

    app.campaign_data.quests.push(quest);
    app.editor_registry.quest_editor_state.selected_quest = Some(0);

    // Delete first objective
    assert_eq!(app.campaign_data.quests[0].stages[0].objectives.len(), 2);
    let result = app.editor_registry.quest_editor_state.delete_objective(
        &mut app.campaign_data.quests,
        0,
        0,
        0,
    );
    assert!(result.is_ok());
    assert_eq!(app.campaign_data.quests[0].stages[0].objectives.len(), 1);

    // Verify remaining objective is CollectItems
    if let antares::domain::quest::QuestObjective::CollectItems { item_id, quantity } =
        &app.campaign_data.quests[0].stages[0].objectives[0]
    {
        assert_eq!(*item_id, 200);
        assert_eq!(*quantity, 3);
    } else {
        panic!("Expected CollectItems objective");
    }

    assert!(app.editor_registry.quest_editor_state.has_unsaved_changes);
}

#[test]
fn test_quest_objective_type_conversion() {
    let mut app = CampaignBuilderApp::default();

    // Create a quest with a KillMonsters objective
    let quest = Quest {
        id: 1,
        name: "Test Quest".to_string(),
        description: "Test Description".to_string(),
        is_main_quest: false,
        repeatable: false,
        min_level: None,
        max_level: None,
        required_quests: Vec::new(),
        stages: vec![QuestStage {
            stage_number: 1,
            name: "Stage 1".to_string(),
            description: "Stage 1 description".to_string(),
            require_all_objectives: true,
            objectives: vec![antares::domain::quest::QuestObjective::KillMonsters {
                monster_id: 100,
                quantity: 5,
            }],
        }],
        rewards: Vec::new(),
        quest_giver_npc: None,
        quest_giver_map: None,
        quest_giver_position: None,
    };

    app.campaign_data.quests.push(quest);

    // Edit objective and change type to CollectItems
    let result =
        app.editor_registry
            .quest_editor_state
            .edit_objective(&app.campaign_data.quests, 0, 0, 0);
    assert!(result.is_ok());

    app.editor_registry
        .quest_editor_state
        .objective_buffer
        .objective_type = quest_editor::ObjectiveType::CollectItems;
    app.editor_registry
        .quest_editor_state
        .objective_buffer
        .item_id = "250".to_string();
    app.editor_registry
        .quest_editor_state
        .objective_buffer
        .quantity = "7".to_string();

    let result = app.editor_registry.quest_editor_state.save_objective(
        &mut app.campaign_data.quests,
        0,
        0,
        0,
    );
    assert!(result.is_ok());

    // Verify objective type changed
    if let antares::domain::quest::QuestObjective::CollectItems { item_id, quantity } =
        &app.campaign_data.quests[0].stages[0].objectives[0]
    {
        assert_eq!(*item_id, 250);
        assert_eq!(*quantity, 7);
    } else {
        panic!("Expected CollectItems objective");
    }
}

#[test]
fn test_quest_editor_invalid_indices() {
    let mut app = CampaignBuilderApp::default();

    // Test with no quests
    let result = app
        .editor_registry
        .quest_editor_state
        .edit_stage(&app.campaign_data.quests, 0, 0);
    assert!(result.is_err());

    let result =
        app.editor_registry
            .quest_editor_state
            .edit_objective(&app.campaign_data.quests, 0, 0, 0);
    assert!(result.is_err());

    let result =
        app.editor_registry
            .quest_editor_state
            .delete_stage(&mut app.campaign_data.quests, 0, 0);
    assert!(result.is_err());

    let result = app.editor_registry.quest_editor_state.delete_objective(
        &mut app.campaign_data.quests,
        0,
        0,
        0,
    );
    assert!(result.is_err());
}

#[test]
fn test_item_type_is_quest_item() {
    let item = CampaignBuilderApp::default_item();
    assert!(!item.is_quest_item());
}

#[test]
fn test_spell_school_types() {
    let mut spell = CampaignBuilderApp::default_spell();
    spell.school = SpellSchool::Cleric;
    assert_eq!(spell.school, SpellSchool::Cleric);

    spell.school = SpellSchool::Sorcerer;
    assert_eq!(spell.school, SpellSchool::Sorcerer);
}

#[test]
fn test_monster_flags() {
    let mut monster = CampaignBuilderApp::default_monster();
    assert!(!monster.is_undead);
    assert!(!monster.can_regenerate);
    assert!(monster.can_advance);

    monster.is_undead = true;
    monster.can_regenerate = true;
    monster.can_advance = false;

    assert!(monster.is_undead);
    assert!(monster.can_regenerate);
    assert!(!monster.can_advance);
}

#[test]
fn test_loot_table_initialization() {
    let monster = CampaignBuilderApp::default_monster();
    assert_eq!(monster.loot.gold_min, 0);
    assert_eq!(monster.loot.gold_max, 0);
    assert_eq!(monster.loot.gems_min, 0);
    assert_eq!(monster.loot.gems_max, 0);
    assert_eq!(monster.loot.experience, 0);
}

#[test]
fn test_editor_tab_equality() {
    assert_eq!(EditorTab::Items, EditorTab::Items);
    assert_ne!(EditorTab::Items, EditorTab::Spells);
    assert_ne!(EditorTab::Spells, EditorTab::Monsters);
}

#[test]
fn test_editor_mode_equality() {
    assert_eq!(EditorMode::List, EditorMode::List);
    assert_eq!(EditorMode::Add, EditorMode::Add);
    assert_eq!(EditorMode::Edit, EditorMode::Edit);
    assert_ne!(EditorMode::List, EditorMode::Add);
    assert_ne!(EditorMode::Add, EditorMode::Edit);
}

#[test]
fn test_items_id_generation() {
    let mut app = CampaignBuilderApp::default();

    // Add first item
    let mut item1 = CampaignBuilderApp::default_item();
    item1.id = 1;
    item1.name = "Item 1".to_string();
    app.campaign_data.items.push(item1);

    // Next ID should be 2
    let next_id = app
        .campaign_data
        .items
        .iter()
        .map(|i| i.id)
        .max()
        .unwrap_or(0)
        + 1;
    assert_eq!(next_id, 2);

    // Add second item
    let mut item2 = CampaignBuilderApp::default_item();
    item2.id = next_id;
    item2.name = "Item 2".to_string();
    app.campaign_data.items.push(item2);

    assert_eq!(app.campaign_data.items.len(), 2);
}

#[test]
fn test_spells_level_range() {
    let spell = CampaignBuilderApp::default_spell();
    assert!(spell.level >= 1 && spell.level <= 7);
}

#[test]
fn test_monster_stats_initialization() {
    let monster = CampaignBuilderApp::default_monster();
    assert_eq!(monster.stats.might.base, 10);
    assert_eq!(monster.stats.intellect.base, 10);
    assert_eq!(monster.stats.personality.base, 10);
    assert_eq!(monster.stats.endurance.base, 10);
    assert_eq!(monster.stats.speed.base, 10);
    assert_eq!(monster.stats.accuracy.base, 10);
    assert_eq!(monster.stats.luck.base, 10);
}

#[test]
fn test_attack_initialization() {
    let monster = CampaignBuilderApp::default_monster();
    assert_eq!(monster.attacks.len(), 1);
    assert_eq!(monster.attacks[0].damage.count, 1);
    assert_eq!(monster.attacks[0].damage.sides, 4);
    assert!(matches!(
        monster.attacks[0].attack_type,
        AttackType::Physical
    ));
}

#[test]
fn test_magic_resistance_range() {
    let monster = CampaignBuilderApp::default_monster();
    assert!(monster.magic_resistance <= 100);
}

#[test]
fn test_apply_condition_edits_insert() {
    use campaign_builder::conditions_editor::apply_condition_edits;

    let mut conditions: Vec<ConditionDefinition> = Vec::new();

    let new_cond = ConditionDefinition {
        id: "insert_test".to_string(),
        name: "Insert Test".to_string(),
        description: "Insert via helper".to_string(),
        effects: vec![],
        default_duration: antares::domain::conditions::ConditionDuration::Rounds(1),
        icon_id: Some("icon_insert".to_string()),
    };

    // Insert should succeed
    assert!(apply_condition_edits(&mut conditions, None, &new_cond).is_ok());
    assert_eq!(conditions.len(), 1);
    assert_eq!(conditions[0].id, "insert_test");
}

#[test]
fn test_apply_condition_edits_update_success() {
    use campaign_builder::conditions_editor::apply_condition_edits;

    let mut conditions: Vec<ConditionDefinition> = Vec::new();

    let c1 = ConditionDefinition {
        id: "c1".to_string(),
        name: "C1".to_string(),
        description: "".to_string(),
        effects: vec![],
        default_duration: antares::domain::conditions::ConditionDuration::Rounds(2),
        icon_id: None,
    };
    let c2 = ConditionDefinition {
        id: "c2".to_string(),
        name: "C2".to_string(),
        description: "".to_string(),
        effects: vec![],
        default_duration: antares::domain::conditions::ConditionDuration::Minutes(3),
        icon_id: None,
    };

    conditions.push(c1.clone());
    conditions.push(c2.clone());

    // Update c1 to have id 'c3'
    let mut edited = c1.clone();
    edited.id = "c3".to_string();
    edited.name = "C3 changed".to_string();
    edited.icon_id = Some("icon_c3".to_string());
    match apply_condition_edits(&mut conditions, Some("c1"), &edited) {
        Ok(()) => {
            assert!(conditions.iter().any(|c| c.id == "c3"));
            assert!(!conditions.iter().any(|c| c.id == "c1"));
            // confirm name/icon changed
            let found = conditions.iter().find(|c| c.id == "c3").unwrap();
            assert_eq!(found.name, "C3 changed");
            assert_eq!(found.icon_id.as_ref().unwrap(), "icon_c3");
        }
        Err(e) => panic!("apply_condition_edits failed: {}", e),
    }
}

#[test]
fn test_apply_condition_edits_update_duplicate_error() {
    use campaign_builder::conditions_editor::apply_condition_edits;

    let mut conditions: Vec<ConditionDefinition> = Vec::new();

    let c1 = ConditionDefinition {
        id: "c1".to_string(),
        name: "C1".to_string(),
        description: "".to_string(),
        effects: vec![],
        default_duration: antares::domain::conditions::ConditionDuration::Rounds(2),
        icon_id: None,
    };
    let c2 = ConditionDefinition {
        id: "c2".to_string(),
        name: "C2".to_string(),
        description: "".to_string(),
        effects: vec![],
        default_duration: antares::domain::conditions::ConditionDuration::Minutes(3),
        icon_id: None,
    };

    conditions.push(c1.clone());
    conditions.push(c2.clone());

    // Attempt to update c1 to id 'c2' (duplicate) -> should fail
    let mut edited = c1.clone();
    edited.id = "c2".to_string();
    let res = apply_condition_edits(&mut conditions, Some("c1"), &edited);
    assert!(res.is_err());
}

#[test]
fn test_apply_condition_edits_insert_duplicate_error() {
    use campaign_builder::conditions_editor::apply_condition_edits;

    let mut conditions: Vec<ConditionDefinition> = Vec::new();

    let c1 = ConditionDefinition {
        id: "dup".to_string(),
        name: "Dup".to_string(),
        description: "".to_string(),
        effects: vec![],
        default_duration: antares::domain::conditions::ConditionDuration::Rounds(2),
        icon_id: None,
    };

    conditions.push(c1.clone());

    // Attempt to insert a new condition with id 'dup' -> should fail
    let new_dup = ConditionDefinition {
        id: "dup".to_string(),
        name: "Dup New".to_string(),
        description: "".to_string(),
        effects: vec![],
        default_duration: antares::domain::conditions::ConditionDuration::Minutes(1),
        icon_id: None,
    };
    let res = apply_condition_edits(&mut conditions, None, &new_dup);
    assert!(res.is_err());
}

// Effect Editing Helper Tests
#[test]
fn test_condition_effect_helpers_success_flow() {
    use campaign_builder::conditions_editor::{
        add_effect_to_condition, delete_effect_from_condition, duplicate_effect_in_condition,
        move_effect_in_condition, update_effect_in_condition,
    };

    use antares::domain::conditions::{ConditionDefinition, ConditionDuration, ConditionEffect};

    let mut condition = ConditionDefinition {
        id: "c_effects".to_string(),
        name: "Effect Test".to_string(),
        description: "".to_string(),
        effects: vec![],
        default_duration: ConditionDuration::Rounds(1),
        icon_id: None,
    };

    // Add a status effect 'sleep'
    let sleep_eff = ConditionEffect::StatusEffect("sleep".to_string());
    add_effect_to_condition(&mut condition, sleep_eff.clone());
    assert_eq!(condition.effects.len(), 1);
    assert_eq!(condition.effects[0], sleep_eff);

    // Add an AttributeModifier effect
    let buff = ConditionEffect::AttributeModifier {
        attribute: "might".to_string(),
        value: 5,
    };
    add_effect_to_condition(&mut condition, buff.clone());
    assert_eq!(condition.effects.len(), 2);
    assert_eq!(condition.effects[1], buff);

    // Duplicate the sleep effect (index 0), now effects [sleep, sleep, buff]
    duplicate_effect_in_condition(&mut condition, 0).unwrap();
    assert_eq!(condition.effects.len(), 3);
    assert_eq!(condition.effects[1], sleep_eff);

    // Move the buff up (index 2 -> index 1)
    move_effect_in_condition(&mut condition, 2, -1).unwrap();
    // After moving, index 1 should be the AttributeModifier and index 2 is the duplicate sleep
    if let ConditionEffect::AttributeModifier { attribute, value } = &condition.effects[1] {
        assert_eq!(attribute, "might");
        assert_eq!(*value, 5);
    } else {
        panic!("Expected AttributeModifier at index 1 after move");
    }

    // Update the moved buff to a different value
    let buff2 = ConditionEffect::AttributeModifier {
        attribute: "might".to_string(),
        value: 10,
    };
    update_effect_in_condition(&mut condition, 1, buff2.clone()).unwrap();
    assert_eq!(condition.effects[1], buff2);

    // Delete effect at index 0 (original sleep)
    delete_effect_from_condition(&mut condition, 0).unwrap();
    assert_eq!(condition.effects.len(), 2);
}

#[test]
fn test_update_effect_out_of_range() {
    use antares::domain::conditions::{ConditionDefinition, ConditionDuration, ConditionEffect};
    use campaign_builder::conditions_editor::update_effect_in_condition;

    let mut condition = ConditionDefinition {
        id: "c_effects2".to_string(),
        name: "Effect Test 2".to_string(),
        description: "".to_string(),
        effects: vec![],
        default_duration: ConditionDuration::Rounds(1),
        icon_id: None,
    };

    let res = update_effect_in_condition(
        &mut condition,
        0,
        ConditionEffect::StatusEffect("x".to_string()),
    );
    assert!(res.is_err());
}

#[test]
fn test_spells_referencing_condition_and_removal() {
    use antares::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};
    use antares::domain::types::DiceRoll;
    use campaign_builder::conditions_editor::{
        remove_condition_references_from_spells, spells_referencing_condition,
    };

    // Create a small set of spells, some referencing 'bless', others not.
    let mut spells: Vec<Spell> = Vec::new();

    spells.push(Spell {
        id: 0x0101,
        name: "Bless".to_string(),
        school: SpellSchool::Cleric,
        level: 1,
        sp_cost: 1,
        gem_cost: 0,
        context: SpellContext::Anytime,
        target: SpellTarget::SingleCharacter,
        description: "A simple buff".to_string(),
        damage: None,
        duration: 0,
        saving_throw: false,
        resurrect_hp: None,
        applied_conditions: vec!["bless".to_string()],
    });

    spells.push(Spell {
        id: 0x0201,
        name: "Fireball".to_string(),
        school: SpellSchool::Sorcerer,
        level: 3,
        sp_cost: 5,
        gem_cost: 0,
        context: SpellContext::CombatOnly,
        target: SpellTarget::MonsterGroup,
        description: "A large blast of fire".to_string(),
        damage: Some(DiceRoll::new(3, 6, 0)),
        duration: 0,
        saving_throw: false,
        resurrect_hp: None,
        applied_conditions: vec!["burn".to_string()],
    });

    // No match for a nonexistent condition
    let used = spells_referencing_condition(&spells, "nonexistent");
    assert!(used.is_empty());

    // A single spell references 'bless'
    let used = spells_referencing_condition(&spells, "bless");
    assert_eq!(used.len(), 1);
    assert_eq!(used[0], "Bless");

    // Remove references from spells (should remove from Bless)
    let removed = remove_condition_references_from_spells(&mut spells, "bless");
    assert_eq!(removed, 1);
    assert!(spells[0].applied_conditions.is_empty());
}

#[test]
fn test_apply_condition_edits_validation_and_effect_types() {
    use antares::domain::conditions::{ConditionDefinition, ConditionDuration, ConditionEffect};
    use antares::domain::types::DiceRoll;
    use campaign_builder::conditions_editor::apply_condition_edits;

    let mut conditions: Vec<ConditionDefinition> = Vec::new();

    // Invalid DOT - zero dice count should fail validation.
    let invalid_dot = ConditionDefinition {
        id: "invalid_dot".to_string(),
        name: "Invalid DOT".to_string(),
        description: "".to_string(),
        effects: vec![ConditionEffect::DamageOverTime {
            damage: DiceRoll::new(0, 6, 0),
            element: "fire".to_string(),
        }],
        default_duration: ConditionDuration::Rounds(1),
        icon_id: None,
    };

    assert!(apply_condition_edits(&mut conditions, None, &invalid_dot).is_err());

    // Invalid attribute modifier value (too large).
    let invalid_attr = ConditionDefinition {
        id: "invalid_attr".to_string(),
        name: "Invalid Attribute".to_string(),
        description: "".to_string(),
        effects: vec![ConditionEffect::AttributeModifier {
            attribute: "might".to_string(),
            value: 9999,
        }],
        default_duration: ConditionDuration::Rounds(1),
        icon_id: None,
    };

    assert!(apply_condition_edits(&mut conditions, None, &invalid_attr).is_err());

    // Valid round-trip: each effect type present and valid should be accepted.
    let valid_all = ConditionDefinition {
        id: "valid_all".to_string(),
        name: "Valid All".to_string(),
        description: "".to_string(),
        effects: vec![
            ConditionEffect::StatusEffect("sleep".to_string()),
            ConditionEffect::AttributeModifier {
                attribute: "might".to_string(),
                value: 5,
            },
            ConditionEffect::DamageOverTime {
                damage: DiceRoll::new(1, 6, 0),
                element: "poison".to_string(),
            },
            ConditionEffect::HealOverTime {
                amount: DiceRoll::new(1, 4, 0),
            },
        ],
        default_duration: ConditionDuration::Rounds(1),
        icon_id: None,
    };

    assert!(apply_condition_edits(&mut conditions, None, &valid_all).is_ok());
    assert_eq!(conditions.len(), 1);
}

#[test]
fn test_validate_effect_edit_buffer() {
    use antares::domain::types::DiceRoll;
    use campaign_builder::conditions_editor::{validate_effect_edit_buffer, EffectEditBuffer};

    // Attribute modifier - empty attribute fails
    let mut buf = EffectEditBuffer {
        effect_type: Some("AttributeModifier".to_string()),
        attribute: "".to_string(),
        ..Default::default()
    };
    assert!(validate_effect_edit_buffer(&buf).is_err());

    // Attribute present but value out of allowed range fails
    buf.attribute = "might".to_string();
    buf.attribute_value = 300; // out of range
    assert!(validate_effect_edit_buffer(&buf).is_err());

    // Status effect - empty tag fails
    let buf2 = EffectEditBuffer {
        effect_type: Some("StatusEffect".to_string()),
        status_tag: "".to_string(),
        ..Default::default()
    };
    assert!(validate_effect_edit_buffer(&buf2).is_err());

    // DOT validation - invalid dice count should fail
    let buf3 = EffectEditBuffer {
        effect_type: Some("DamageOverTime".to_string()),
        dice: DiceRoll::new(0, 6, 0),
        element: "fire".to_string(),
        ..Default::default()
    };
    assert!(validate_effect_edit_buffer(&buf3).is_err());

    // HOT validation - invalid dice sides should fail
    let buf4 = EffectEditBuffer {
        effect_type: Some("HealOverTime".to_string()),
        dice: DiceRoll::new(1, 1, 0), // invalid sides
        ..Default::default()
    };
    assert!(validate_effect_edit_buffer(&buf4).is_err());
}

#[test]
fn test_next_available_item_id_empty() {
    let app = CampaignBuilderApp::default();
    assert_eq!(app.next_available_item_id(), 1);
}

#[test]
fn test_next_available_item_id_with_items() {
    let mut app = CampaignBuilderApp::default();

    let mut item1 = CampaignBuilderApp::default_item();
    item1.id = 1;
    app.campaign_data.items.push(item1);

    let mut item2 = CampaignBuilderApp::default_item();
    item2.id = 5;
    app.campaign_data.items.push(item2);

    let mut item3 = CampaignBuilderApp::default_item();
    item3.id = 3;
    app.campaign_data.items.push(item3);

    // Should return max + 1 = 6
    assert_eq!(app.next_available_item_id(), 6);
}

#[test]
fn test_next_available_spell_id() {
    let mut app = CampaignBuilderApp::default();

    let mut spell1 = CampaignBuilderApp::default_spell();
    spell1.id = 100;
    app.campaign_data.spells.push(spell1);

    let mut spell2 = CampaignBuilderApp::default_spell();
    spell2.id = 150;
    app.campaign_data.spells.push(spell2);

    assert_eq!(app.next_available_spell_id(), 151);
}

#[test]
fn test_next_available_monster_id() {
    let mut app = CampaignBuilderApp::default();

    let mut monster1 = CampaignBuilderApp::default_monster();
    monster1.id = 10;
    app.campaign_data.monsters.push(monster1);

    assert_eq!(app.next_available_monster_id(), 11);
}

#[test]
fn test_next_available_map_id() {
    let mut app = CampaignBuilderApp::default();

    let map1 = Map::new(5, "Map 1".to_string(), "Desc 1".to_string(), 20, 20);
    app.campaign_data.maps.push(map1);

    let map2 = Map::new(8, "Map 2".to_string(), "Desc 2".to_string(), 30, 30);
    app.campaign_data.maps.push(map2);

    assert_eq!(app.next_available_map_id(), 9);
}

#[test]
fn test_id_generation_with_gaps() {
    let mut app = CampaignBuilderApp::default();

    // Add items with IDs: 1, 2, 5 (gap at 3, 4)
    let mut item1 = CampaignBuilderApp::default_item();
    item1.id = 1;
    app.campaign_data.items.push(item1);

    let mut item2 = CampaignBuilderApp::default_item();
    item2.id = 2;
    app.campaign_data.items.push(item2);

    let mut item3 = CampaignBuilderApp::default_item();
    item3.id = 5;
    app.campaign_data.items.push(item3);

    // Should return 6 (max + 1), not fill gap
    assert_eq!(app.next_available_item_id(), 6);
}

#[test]
fn test_saturating_add_prevents_overflow() {
    let mut app = CampaignBuilderApp::default();

    // Add item with max ID for ItemId (u8)
    let mut item = CampaignBuilderApp::default_item();
    item.id = 255; // u8::MAX
    app.campaign_data.items.push(item);

    // Should saturate at 255, not overflow
    assert_eq!(app.next_available_item_id(), 255);
}

// ===== Items Editor Enhancement Tests =====

#[test]
fn test_item_type_filter_weapon() {
    let _app = CampaignBuilderApp::default();

    let mut weapon = CampaignBuilderApp::default_item();
    weapon.item_type = ItemType::Weapon(WeaponData {
        damage: DiceRoll::new(1, 8, 0),
        bonus: 1,
        hands_required: 1,
        classification: WeaponClassification::MartialMelee,
    });

    let filter = ItemTypeFilter::Weapon;
    assert!(filter.matches(&weapon));

    // Should not match other types
    let mut armor = CampaignBuilderApp::default_item();
    armor.item_type = ItemType::Armor(antares::domain::items::types::ArmorData {
        ac_bonus: 5,
        weight: 20,
        classification: ArmorClassification::Medium,
    });
    assert!(!filter.matches(&armor));
}

#[test]
fn test_item_type_filter_all_types() {
    use antares::domain::items::types::*;

    let weapon_item = Item {
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
    };

    let armor_item = Item {
        id: 2,
        name: "Chainmail".to_string(),
        item_type: ItemType::Armor(ArmorData {
            ac_bonus: 5,
            weight: 30,
            classification: ArmorClassification::Medium,
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
        tags: vec![],
        mesh_descriptor_override: None,
        mesh_id: None,
    };

    assert!(ItemTypeFilter::Weapon.matches(&weapon_item));
    assert!(!ItemTypeFilter::Weapon.matches(&armor_item));
    assert!(ItemTypeFilter::Armor.matches(&armor_item));
    assert!(!ItemTypeFilter::Armor.matches(&weapon_item));
}

#[test]
fn test_items_filter_magical() {
    let mut app = CampaignBuilderApp::default();
    app.editor_registry.items_editor_state.filter_magical = Some(true);

    let mut magical_item = CampaignBuilderApp::default_item();
    magical_item.id = 1;
    magical_item.name = "Magic Sword".to_string();
    magical_item.max_charges = 10;
    app.campaign_data.items.push(magical_item.clone());

    let mut mundane_item = CampaignBuilderApp::default_item();
    mundane_item.id = 2;
    mundane_item.name = "Normal Sword".to_string();
    mundane_item.max_charges = 0;
    app.campaign_data.items.push(mundane_item);

    // Magical filter should only match magical items
    assert!(magical_item.is_magical());
}

#[test]
fn test_items_filter_cursed() {
    let mut cursed_item = CampaignBuilderApp::default_item();
    cursed_item.is_cursed = true;

    let normal_item = CampaignBuilderApp::default_item();

    assert!(cursed_item.is_cursed);
    assert!(!normal_item.is_cursed);
}

#[test]
fn test_items_filter_quest() {
    use antares::domain::items::types::QuestData;

    let mut quest_item = CampaignBuilderApp::default_item();
    quest_item.item_type = ItemType::Quest(QuestData {
        quest_id: "main_quest".to_string(),
        is_key_item: true,
    });

    let normal_item = CampaignBuilderApp::default_item();

    assert!(quest_item.is_quest_item());
    assert!(!normal_item.is_quest_item());
}

#[test]
fn test_item_proficiency_and_alignment_restrictions() {
    // Local imports for clarity and isolation in test function
    use antares::domain::items::types::{AlignmentRestriction, WeaponClassification};
    use antares::domain::proficiency::ProficiencyDatabase;

    let mut item = CampaignBuilderApp::default_item();

    // Default item should derive the correct proficiency for a simple weapon
    assert_eq!(
        item.required_proficiency(),
        Some(ProficiencyDatabase::proficiency_for_weapon(
            WeaponClassification::Simple
        ))
    );

    // Default alignment restriction should be None
    assert_eq!(item.alignment_restriction, None);

    // Update alignment restriction to GoodOnly and confirm it is stored correctly
    item.alignment_restriction = Some(AlignmentRestriction::GoodOnly);
    assert_eq!(
        item.alignment_restriction,
        Some(AlignmentRestriction::GoodOnly)
    );
    // Ensure EvilOnly is not set
    assert_ne!(
        item.alignment_restriction,
        Some(AlignmentRestriction::EvilOnly)
    );
}

#[test]
fn test_item_type_specific_editors() {
    use antares::domain::items::types::*;

    // Test weapon type
    let weapon = Item {
        id: 1,
        name: "Longsword".to_string(),
        item_type: ItemType::Weapon(WeaponData {
            damage: DiceRoll::new(1, 8, 0),
            bonus: 0,
            hands_required: 1,
            classification: WeaponClassification::MartialMelee,
        }),
        base_cost: 15,
        sell_cost: 7,
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
    };
    assert!(weapon.is_weapon());

    // Test armor type
    let armor = Item {
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
        tags: vec!["heavy_armor".to_string()],
        mesh_descriptor_override: None,
        mesh_id: None,
    };
    assert!(armor.is_armor());

    // Test consumable type
    let potion = Item {
        id: 3,
        name: "Healing Potion".to_string(),
        item_type: ItemType::Consumable(ConsumableData {
            effect: ConsumableEffect::HealHp(20),
            is_combat_usable: true,
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
        tags: vec![],
        mesh_descriptor_override: None,
        mesh_id: None,
    };
    assert!(potion.is_consumable());
}

#[test]
fn test_combined_filters() {
    use antares::domain::items::types::*;

    let mut app = CampaignBuilderApp::default();

    // Add various items
    let magical_weapon = Item {
        id: 1,
        name: "Magic Sword".to_string(),
        item_type: ItemType::Weapon(WeaponData {
            damage: DiceRoll::new(1, 8, 0),
            bonus: 1,
            hands_required: 1,
            classification: WeaponClassification::MartialMelee,
        }),
        base_cost: 100,
        sell_cost: 50,
        alignment_restriction: None,
        constant_bonus: Some(Bonus {
            attribute: BonusAttribute::Might,
            value: 2,
        }),
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 5,
        is_cursed: false,
        icon_path: None,
        tags: vec![],
        mesh_descriptor_override: None,
        mesh_id: None,
    };

    let cursed_armor = Item {
        id: 2,
        name: "Cursed Mail".to_string(),
        item_type: ItemType::Armor(ArmorData {
            ac_bonus: 5,
            weight: 30,
            classification: ArmorClassification::Medium,
        }),
        base_cost: 50,
        sell_cost: 0,
        alignment_restriction: None,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: true,
        icon_path: None,
        tags: vec![],
        mesh_descriptor_override: None,
        mesh_id: None,
    };

    app.campaign_data.items.push(magical_weapon.clone());
    app.campaign_data.items.push(cursed_armor.clone());

    // Test magical + weapon filters
    assert!(magical_weapon.is_magical());
    assert!(magical_weapon.is_weapon());
    assert!(!cursed_armor.is_magical());
    assert!(cursed_armor.is_cursed);
}

#[test]
fn test_items_editor_default_required_proficiency() {
    // Ensure the items editor edit buffer starts with a default weapon and correct derived proficiency
    use antares::domain::items::types::WeaponClassification;
    use antares::domain::proficiency::ProficiencyDatabase;

    let app = CampaignBuilderApp::default();

    // Default edit_buffer is a simple weapon; required_proficiency should match simple_weapon id
    let required_prof = app
        .editor_registry
        .items_editor_state
        .edit_buffer
        .required_proficiency()
        .expect("Default edit buffer should have a derived proficiency");

    assert_eq!(
        required_prof,
        ProficiencyDatabase::proficiency_for_weapon(WeaponClassification::Simple)
    );

    // Default alignment restriction should be None
    assert_eq!(
        app.editor_registry
            .items_editor_state
            .edit_buffer
            .alignment_restriction,
        None
    );
}

#[test]
fn test_items_editor_classification_changes_required_proficiency() {
    use antares::domain::items::types::{ItemType, WeaponClassification, WeaponData};
    use antares::domain::proficiency::ProficiencyDatabase;
    use antares::domain::types::DiceRoll;

    let mut app = CampaignBuilderApp::default();

    // Change the edit buffer weapon classification to MartialMelee and verify derived proficiency
    app.editor_registry.items_editor_state.edit_buffer.item_type = ItemType::Weapon(WeaponData {
        damage: DiceRoll::new(2, 6, 0),
        bonus: 1,
        hands_required: 1,
        classification: WeaponClassification::MartialMelee,
    });

    let required_prof = app
        .editor_registry
        .items_editor_state
        .edit_buffer
        .required_proficiency()
        .expect("Martial melee weapon should have a derived proficiency");

    assert_eq!(
        required_prof,
        ProficiencyDatabase::proficiency_for_weapon(WeaponClassification::MartialMelee)
    );
}

#[test]
fn test_item_preview_displays_all_info() {
    use antares::domain::items::types::*;

    let item = Item {
        id: 10,
        name: "Flaming Sword".to_string(),
        item_type: ItemType::Weapon(WeaponData {
            damage: DiceRoll::new(1, 8, 2),
            bonus: 3,
            hands_required: 1,
            classification: WeaponClassification::MartialMelee,
        }),
        base_cost: 500,
        sell_cost: 250,
        alignment_restriction: Some(AlignmentRestriction::GoodOnly),
        constant_bonus: Some(Bonus {
            attribute: BonusAttribute::Might,
            value: 2,
        }),
        temporary_bonus: None,
        spell_effect: Some(10),
        max_charges: 20,
        is_cursed: false,
        icon_path: None,
        tags: vec![],
        mesh_descriptor_override: None,
        mesh_id: None,
    };

    // Verify item has all expected properties
    assert_eq!(item.id, 10);
    assert!(item.is_weapon());
    assert!(item.is_magical());
    assert!(!item.is_cursed);
    assert!(item.constant_bonus.is_some());
    assert!(item.spell_effect.is_some());
    assert_eq!(item.max_charges, 20);
}

// ===== Spell Editor Enhancement Tests =====

#[test]
fn test_spell_school_filter_cleric() {
    let mut app = CampaignBuilderApp::default();

    // Add test spells
    app.campaign_data.spells.push(Spell::new(
        1,
        "Heal",
        SpellSchool::Cleric,
        1,
        3,
        0,
        SpellContext::Anytime,
        SpellTarget::SingleCharacter,
        "Heals wounds",
        None,
        0,
        false,
    ));
    app.campaign_data.spells.push(Spell::new(
        2,
        "Fireball",
        SpellSchool::Sorcerer,
        3,
        5,
        0,
        SpellContext::CombatOnly,
        SpellTarget::MonsterGroup,
        "Fire damage",
        None,
        0,
        false,
    ));
    app.campaign_data.spells.push(Spell::new(
        3,
        "Bless",
        SpellSchool::Cleric,
        2,
        2,
        0,
        SpellContext::Anytime,
        SpellTarget::AllCharacters,
        "Party buff",
        None,
        0,
        false,
    ));

    // Apply Cleric filter
    app.editor_registry.spells_editor_state.filter_school = Some(SpellSchool::Cleric);

    let filtered: Vec<_> = app
        .campaign_data
        .spells
        .iter()
        .filter(|s| {
            app.editor_registry
                .spells_editor_state
                .filter_school
                .is_none_or(|f| s.school == f)
        })
        .collect();

    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|s| s.school == SpellSchool::Cleric));
}

#[test]
fn test_spell_level_filter() {
    let mut app = CampaignBuilderApp::default();

    app.campaign_data.spells.push(Spell::new(
        1,
        "Heal",
        SpellSchool::Cleric,
        1,
        3,
        0,
        SpellContext::Anytime,
        SpellTarget::SingleCharacter,
        "Level 1",
        None,
        0,
        false,
    ));
    app.campaign_data.spells.push(Spell::new(
        2,
        "Fireball",
        SpellSchool::Sorcerer,
        3,
        5,
        0,
        SpellContext::CombatOnly,
        SpellTarget::MonsterGroup,
        "Level 3",
        None,
        0,
        false,
    ));
    app.campaign_data.spells.push(Spell::new(
        3,
        "Lightning",
        SpellSchool::Sorcerer,
        3,
        6,
        0,
        SpellContext::CombatOnly,
        SpellTarget::SingleMonster,
        "Level 3",
        None,
        0,
        false,
    ));

    // Filter level 3 spells
    app.editor_registry.spells_editor_state.filter_level = Some(3);

    let filtered: Vec<_> = app
        .campaign_data
        .spells
        .iter()
        .filter(|s| {
            app.editor_registry
                .spells_editor_state
                .filter_level
                .is_none_or(|f| s.level == f)
        })
        .collect();

    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|s| s.level == 3));
}

#[test]
fn test_spell_combined_filters() {
    let mut app = CampaignBuilderApp::default();

    app.campaign_data.spells.push(Spell::new(
        1,
        "Heal",
        SpellSchool::Cleric,
        1,
        3,
        0,
        SpellContext::Anytime,
        SpellTarget::SingleCharacter,
        "Cleric L1",
        None,
        0,
        false,
    ));
    app.campaign_data.spells.push(Spell::new(
        2,
        "Cure Disease",
        SpellSchool::Cleric,
        3,
        5,
        0,
        SpellContext::Anytime,
        SpellTarget::SingleCharacter,
        "Cleric L3",
        None,
        0,
        false,
    ));
    app.campaign_data.spells.push(Spell::new(
        3,
        "Fireball",
        SpellSchool::Sorcerer,
        3,
        5,
        0,
        SpellContext::CombatOnly,
        SpellTarget::MonsterGroup,
        "Sorcerer L3",
        None,
        0,
        false,
    ));

    // Filter: Cleric + Level 3
    app.editor_registry.spells_editor_state.filter_school = Some(SpellSchool::Cleric);
    app.editor_registry.spells_editor_state.filter_level = Some(3);

    let filtered: Vec<_> = app
        .campaign_data
        .spells
        .iter()
        .filter(|s| {
            app.editor_registry
                .spells_editor_state
                .filter_school
                .is_none_or(|f| s.school == f)
                && app
                    .editor_registry
                    .spells_editor_state
                    .filter_level
                    .is_none_or(|f| s.level == f)
        })
        .collect();

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "Cure Disease");
    assert_eq!(filtered[0].school, SpellSchool::Cleric);
    assert_eq!(filtered[0].level, 3);
}

#[test]
fn test_spell_context_target_editing() {
    let mut app = CampaignBuilderApp::default();
    app.editor_registry.spells_editor_state.edit_buffer = Spell::new(
        1,
        "Test",
        SpellSchool::Cleric,
        1,
        1,
        0,
        SpellContext::Anytime,
        SpellTarget::Self_,
        "Test",
        None,
        0,
        false,
    );

    // Change context
    app.editor_registry.spells_editor_state.edit_buffer.context = SpellContext::CombatOnly;
    assert_eq!(
        app.editor_registry.spells_editor_state.edit_buffer.context,
        SpellContext::CombatOnly
    );

    // Change target
    app.editor_registry.spells_editor_state.edit_buffer.target = SpellTarget::AllCharacters;
    assert_eq!(
        app.editor_registry.spells_editor_state.edit_buffer.target,
        SpellTarget::AllCharacters
    );
}

// ===== Monster Editor Enhancement Tests =====

#[test]
fn test_monster_attacks_editor() {
    let mut app = CampaignBuilderApp::default();
    app.editor_registry.monsters_editor_state.edit_buffer = CampaignBuilderApp::default_monster();

    // Initial attacks
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .attacks
            .len(),
        1
    );

    // Add attack
    app.editor_registry
        .monsters_editor_state
        .edit_buffer
        .attacks
        .push(Attack {
            damage: DiceRoll::new(2, 8, 3),
            attack_type: AttackType::Fire,
            special_effect: Some(SpecialEffect::Poison),
            is_ranged: false,
        });

    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .attacks
            .len(),
        2
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .attacks[1]
            .damage
            .count,
        2
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .attacks[1]
            .damage
            .sides,
        8
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .attacks[1]
            .damage
            .bonus,
        3
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .attacks[1]
            .attack_type,
        AttackType::Fire
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .attacks[1]
            .special_effect,
        Some(SpecialEffect::Poison)
    );
}

#[test]
fn test_monster_attack_types() {
    let mut attack = Attack {
        damage: DiceRoll::new(1, 6, 0),
        attack_type: AttackType::Physical,
        special_effect: None,
        is_ranged: false,
    };

    // Test all attack types
    attack.attack_type = AttackType::Fire;
    assert_eq!(attack.attack_type, AttackType::Fire);

    attack.attack_type = AttackType::Cold;
    assert_eq!(attack.attack_type, AttackType::Cold);

    attack.attack_type = AttackType::Electricity;
    assert_eq!(attack.attack_type, AttackType::Electricity);

    attack.attack_type = AttackType::Acid;
    assert_eq!(attack.attack_type, AttackType::Acid);

    attack.attack_type = AttackType::Poison;
    assert_eq!(attack.attack_type, AttackType::Poison);

    attack.attack_type = AttackType::Energy;
    assert_eq!(attack.attack_type, AttackType::Energy);
}

#[test]
fn test_monster_special_effects() {
    let mut attack = Attack {
        damage: DiceRoll::new(1, 6, 0),
        attack_type: AttackType::Physical,
        special_effect: None,
        is_ranged: false,
    };

    // Test all special effects
    let effects = vec![
        SpecialEffect::Poison,
        SpecialEffect::Disease,
        SpecialEffect::Paralysis,
        SpecialEffect::Sleep,
        SpecialEffect::Drain,
        SpecialEffect::Stone,
        SpecialEffect::Death,
    ];

    for effect in effects {
        attack.special_effect = Some(effect);
        assert_eq!(attack.special_effect, Some(effect));
    }

    attack.special_effect = None;
    assert!(attack.special_effect.is_none());
}

#[test]
fn test_monster_loot_editor() {
    let mut app = CampaignBuilderApp::default();
    app.editor_registry.monsters_editor_state.edit_buffer = CampaignBuilderApp::default_monster();

    // Modify loot table
    app.editor_registry
        .monsters_editor_state
        .edit_buffer
        .loot
        .gold_min = 10;
    app.editor_registry
        .monsters_editor_state
        .edit_buffer
        .loot
        .gold_max = 50;
    app.editor_registry
        .monsters_editor_state
        .edit_buffer
        .loot
        .gems_min = 0;
    app.editor_registry
        .monsters_editor_state
        .edit_buffer
        .loot
        .gems_max = 2;
    app.editor_registry
        .monsters_editor_state
        .edit_buffer
        .loot
        .experience = 150;

    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .loot
            .gold_min,
        10
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .loot
            .gold_max,
        50
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .loot
            .gems_min,
        0
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .loot
            .gems_max,
        2
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .loot
            .experience,
        150
    );
}

#[test]
fn test_monster_stats_editor() {
    let mut app = CampaignBuilderApp::default();
    app.editor_registry.monsters_editor_state.edit_buffer = CampaignBuilderApp::default_monster();

    // Modify all stats
    app.editor_registry
        .monsters_editor_state
        .edit_buffer
        .stats
        .might
        .base = 20;
    app.editor_registry
        .monsters_editor_state
        .edit_buffer
        .stats
        .intellect
        .base = 5;
    app.editor_registry
        .monsters_editor_state
        .edit_buffer
        .stats
        .personality
        .base = 8;
    app.editor_registry
        .monsters_editor_state
        .edit_buffer
        .stats
        .endurance
        .base = 18;
    app.editor_registry
        .monsters_editor_state
        .edit_buffer
        .stats
        .speed
        .base = 12;
    app.editor_registry
        .monsters_editor_state
        .edit_buffer
        .stats
        .accuracy
        .base = 15;
    app.editor_registry
        .monsters_editor_state
        .edit_buffer
        .stats
        .luck
        .base = 6;

    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .stats
            .might
            .base,
        20
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .stats
            .intellect
            .base,
        5
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .stats
            .personality
            .base,
        8
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .stats
            .endurance
            .base,
        18
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .stats
            .speed
            .base,
        12
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .stats
            .accuracy
            .base,
        15
    );
    assert_eq!(
        app.editor_registry
            .monsters_editor_state
            .edit_buffer
            .stats
            .luck
            .base,
        6
    );
}

#[test]
fn test_monster_xp_calculation_basic() {
    let app = CampaignBuilderApp::default();
    let monster = MonsterDefinition {
        id: 1,
        name: "Test Monster".to_string(),
        stats: Stats::new(10, 10, 10, 10, 10, 10, 10),
        hp: AttributePair16::new(20),
        ac: AttributePair::new(10),
        attacks: vec![Attack {
            damage: DiceRoll::new(1, 6, 0),
            attack_type: AttackType::Physical,
            special_effect: None,
            is_ranged: false,
        }],
        flee_threshold: 0,
        special_attack_threshold: 0,
        resistances: MonsterResistances::new(),
        can_regenerate: false,
        can_advance: true,
        is_undead: false,
        magic_resistance: 0,
        loot: LootTable {
            gold_min: 0,
            gold_max: 0,
            gems_min: 0,
            gems_max: 0,
            items: Vec::new(),
            experience: 0,
        },
        creature_id: None,
        conditions: MonsterCondition::Normal,
        active_conditions: vec![],
        has_acted: false,
    };

    let xp = app
        .editor_registry
        .monsters_editor_state
        .calculate_monster_xp(&monster);

    // Base: 20 HP * 10 = 200
    // + 1 attack * 20 = 20
    // + avg damage (1d6 = 3.5) * 5 = ~17
    // Total should be ~237
    assert!(xp >= 200);
    assert!(xp < 300);
}

#[test]
fn test_monster_xp_calculation_with_abilities() {
    let app = CampaignBuilderApp::default();
    let monster = MonsterDefinition {
        id: 1,
        name: "Powerful Monster".to_string(),
        stats: Stats::new(20, 10, 10, 20, 15, 15, 10),
        hp: AttributePair16::new(50),
        ac: AttributePair::new(5),
        attacks: vec![
            Attack {
                damage: DiceRoll::new(2, 8, 5),
                attack_type: AttackType::Physical,
                special_effect: None,
                is_ranged: false,
            },
            Attack {
                damage: DiceRoll::new(1, 6, 2),
                attack_type: AttackType::Fire,
                special_effect: Some(SpecialEffect::Paralysis),
                is_ranged: false,
            },
        ],
        flee_threshold: 5,
        special_attack_threshold: 30,
        resistances: MonsterResistances::new(),
        can_regenerate: true,
        can_advance: true,
        is_undead: false,
        magic_resistance: 15,
        loot: LootTable {
            gold_min: 0,
            gold_max: 0,
            gems_min: 0,
            gems_max: 0,
            items: Vec::new(),
            experience: 0,
        },
        creature_id: None,
        conditions: MonsterCondition::Normal,
        active_conditions: vec![],
        has_acted: false,
    };

    let xp = app
        .editor_registry
        .monsters_editor_state
        .calculate_monster_xp(&monster);

    // Should have significant XP due to:
    // - High HP (50 * 10 = 500)
    // - Low AC bonus ((10 - 5) * 50 = 250)
    // - 2 attacks (2 * 20 = 40)
    // - High damage
    // - Special effect (+50)
    // - Regenerate (+100)
    // - Undead (+50)
    // - Magic resistance (50 * 2 = 100)
    assert!(xp >= 1000);
}

#[test]
fn test_monster_preview_fields() {
    let _app = CampaignBuilderApp::default();
    let monster = MonsterDefinition {
        id: 1,
        name: "Goblin".to_string(),
        stats: Stats::new(12, 8, 6, 10, 14, 10, 5),
        hp: AttributePair16::new(15),
        ac: AttributePair::new(12),
        attacks: vec![Attack {
            damage: DiceRoll::new(1, 4, 1),
            attack_type: AttackType::Physical,
            special_effect: None,
            is_ranged: false,
        }],
        flee_threshold: 2,
        special_attack_threshold: 10,
        resistances: MonsterResistances::new(),
        can_regenerate: false,
        can_advance: true,
        is_undead: false,
        magic_resistance: 0,
        loot: LootTable {
            gold_min: 0,
            gold_max: 0,
            gems_min: 0,
            gems_max: 0,
            items: Vec::new(),
            experience: 25,
        },
        creature_id: None,
        conditions: MonsterCondition::Normal,
        active_conditions: vec![],
        has_acted: false,
    };

    // Verify all preview fields exist
    assert_eq!(monster.name, "Goblin");
    assert_eq!(monster.hp.base, 15);
    assert_eq!(monster.ac.base, 12);
    assert_eq!(monster.attacks.len(), 1);
    assert!(!monster.is_undead);
    assert!(!monster.can_regenerate);
    assert!(monster.can_advance);
    assert_eq!(monster.loot.experience, 25);
}

#[test]
fn test_spell_all_contexts() {
    // Test all spell contexts are available
    let contexts = vec![
        SpellContext::Anytime,
        SpellContext::CombatOnly,
        SpellContext::NonCombatOnly,
        SpellContext::OutdoorOnly,
        SpellContext::IndoorOnly,
        SpellContext::OutdoorCombat,
    ];

    for context in contexts {
        let spell = Spell::new(
            1,
            "Test",
            SpellSchool::Cleric,
            1,
            1,
            0,
            context,
            SpellTarget::Self_,
            "Test",
            None,
            0,
            false,
        );
        assert_eq!(spell.context, context);
    }
}

#[test]
fn test_spell_all_targets() {
    // Test all spell targets are available
    let targets = vec![
        SpellTarget::Self_,
        SpellTarget::SingleCharacter,
        SpellTarget::AllCharacters,
        SpellTarget::SingleMonster,
        SpellTarget::MonsterGroup,
        SpellTarget::AllMonsters,
        SpellTarget::SpecificMonsters,
    ];

    for target in targets {
        let spell = Spell::new(
            1,
            "Test",
            SpellSchool::Cleric,
            1,
            1,
            0,
            SpellContext::Anytime,
            target,
            "Test",
            None,
            0,
            false,
        );
        assert_eq!(spell.target, target);
    }
}

// ============================================================
// Quest Editor Integration Tests
// ============================================================

#[test]
fn test_quest_editor_state_initialization() {
    let app = CampaignBuilderApp::default();
    assert_eq!(app.campaign_data.quests.len(), 0);
    assert_eq!(app.editor_registry.quest_editor_state.selected_quest, None);
    assert_eq!(
        app.editor_registry.quest_editor_state.mode,
        quest_editor::QuestEditorMode::List
    );
    assert!(app.editor_registry._quests_show_preview);
    assert!(app
        .editor_registry
        .quest_editor_state
        .search_filter
        .is_empty());
}

#[test]
fn test_quest_list_operations() {
    let mut app = CampaignBuilderApp::default();

    let quest1 = Quest::new(1, "Quest 1", "First quest");
    let quest2 = Quest::new(2, "Quest 2", "Second quest");
    app.campaign_data.quests.push(quest1);
    app.campaign_data.quests.push(quest2);

    assert_eq!(app.campaign_data.quests.len(), 2);
    assert_eq!(app.campaign_data.quests[0].name, "Quest 1");
    assert_eq!(app.campaign_data.quests[1].name, "Quest 2");
}

#[test]
fn test_quest_search_filter() {
    let mut app = CampaignBuilderApp::default();

    let quest1 = Quest::new(1, "Dragon Slayer", "Kill the dragon");
    let quest2 = Quest::new(2, "Fetch Water", "Get water from the well");
    let quest3 = Quest::new(3, "Dragon Rider", "Tame a dragon");

    app.campaign_data.quests.push(quest1);
    app.campaign_data.quests.push(quest2);
    app.campaign_data.quests.push(quest3);

    // Filter by "dragon"
    app.editor_registry.quest_editor_state.search_filter = "dragon".to_string();
    let filtered = app
        .editor_registry
        .quest_editor_state
        .filtered_quests(&app.campaign_data.quests);

    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().any(|(_, q)| q.id == 1));
    assert!(filtered.iter().any(|(_, q)| q.id == 3));
}

#[test]
fn test_next_available_quest_id() {
    let mut app = CampaignBuilderApp::default();

    assert_eq!(app.next_available_quest_id(), 1);

    app.campaign_data
        .quests
        .push(Quest::new(5, "Quest 5", "Description"));
    assert_eq!(app.next_available_quest_id(), 6);

    app.campaign_data
        .quests
        .push(Quest::new(10, "Quest 10", "Description"));
    assert_eq!(app.next_available_quest_id(), 11);
}

#[test]
fn test_quest_with_stages() {
    let mut quest = Quest::new(1, "Multi-Stage Quest", "A quest with multiple stages");
    let stage1 = antares::domain::quest::QuestStage::new(1, "Stage 1");
    let stage2 = antares::domain::quest::QuestStage::new(2, "Stage 2");

    quest.add_stage(stage1);
    quest.add_stage(stage2);

    assert_eq!(quest.stages.len(), 2);
    assert_eq!(quest.stages[0].stage_number, 1);
    assert_eq!(quest.stages[1].stage_number, 2);
}

#[test]
fn test_quest_with_rewards() {
    use antares::domain::quest::QuestReward;

    let mut quest = Quest::new(1, "Rewarding Quest", "A quest with rewards");
    quest.add_reward(QuestReward::Experience(1000));
    quest.add_reward(QuestReward::Gold(500));

    assert_eq!(quest.rewards.len(), 2);
    assert!(matches!(quest.rewards[0], QuestReward::Experience(1000)));
    assert!(matches!(quest.rewards[1], QuestReward::Gold(500)));
}

#[test]
fn test_quest_level_requirements() {
    let mut quest = Quest::new(1, "Level Quest", "Quest with level requirements");
    quest.min_level = Some(5);
    quest.max_level = Some(15);

    assert_eq!(quest.min_level, Some(5));
    assert_eq!(quest.max_level, Some(15));
    assert!(quest.is_available_for_level(10));
    assert!(!quest.is_available_for_level(3));
    assert!(!quest.is_available_for_level(20));
}

#[test]
fn test_quest_preview_toggle() {
    let mut app = CampaignBuilderApp::default();

    // Default is true
    assert!(app.editor_registry._quests_show_preview);

    app.editor_registry._quests_show_preview = false;
    assert!(!app.editor_registry._quests_show_preview);

    app.editor_registry._quests_show_preview = true;
    assert!(app.editor_registry._quests_show_preview);
}

#[test]
fn test_quest_with_giver_location() {
    let mut quest = Quest::new(1, "NPC Quest", "Quest from an NPC");
    quest.quest_giver_npc = Some("100".to_string());
    quest.quest_giver_map = Some(5);
    quest.quest_giver_position = Some(antares::domain::types::Position::new(10, 20));

    assert_eq!(quest.quest_giver_npc, Some("100".to_string()));
    assert_eq!(quest.quest_giver_map, Some(5));
    assert!(quest.quest_giver_position.is_some());
}

#[test]
fn test_quest_repeatable_flag() {
    let mut quest1 = Quest::new(1, "One-Time Quest", "Can only do once");
    quest1.repeatable = false;

    let mut quest2 = Quest::new(2, "Daily Quest", "Can repeat daily");
    quest2.repeatable = true;

    assert!(!quest1.repeatable);
    assert!(quest2.repeatable);
}

#[test]
fn test_quest_main_quest_flag() {
    let mut quest1 = Quest::new(1, "Main Story", "Part of main storyline");
    quest1.is_main_quest = true;

    let mut quest2 = Quest::new(2, "Side Story", "Optional side quest");
    quest2.is_main_quest = false;

    assert!(quest1.is_main_quest);
    assert!(!quest2.is_main_quest);
}

#[test]
fn test_quest_import_buffer() {
    let app = CampaignBuilderApp::default();
    assert!(app.editor_registry._quests_import_buffer.is_empty());
    assert!(!app.editor_registry._quests_show_import_dialog);
}

#[test]
fn test_quest_editor_mode_transitions() {
    let mut app = CampaignBuilderApp::default();

    // Start in list mode
    assert_eq!(
        app.editor_registry.quest_editor_state.mode,
        quest_editor::QuestEditorMode::List
    );

    // Transition to creating
    let next_id = app.next_available_quest_id();
    app.editor_registry
        .quest_editor_state
        .start_new_quest(&mut app.campaign_data.quests, next_id.to_string());
    assert_eq!(
        app.editor_registry.quest_editor_state.mode,
        quest_editor::QuestEditorMode::Creating
    );

    // Cancel back to list
    app.editor_registry
        .quest_editor_state
        .cancel_edit(&mut app.campaign_data.quests);
    assert_eq!(
        app.editor_registry.quest_editor_state.mode,
        quest_editor::QuestEditorMode::List
    );
}

// ============================================================
// Dialogue Editor Integration Tests
// ============================================================

#[test]
fn test_dialogue_editor_state_initialization() {
    let app = CampaignBuilderApp::default();

    assert!(app.campaign_data.dialogues.is_empty());
    assert!(app
        .editor_registry
        .dialogue_editor_state
        .dialogues
        .is_empty());
    assert_eq!(
        app.editor_registry.dialogue_editor_state.mode,
        dialogue_editor::DialogueEditorMode::List
    );
    // Search filter and import state are managed by DialogueEditorState
    assert!(app
        .editor_registry
        .dialogue_editor_state
        .search_filter
        .is_empty());
    assert!(!app.editor_registry.dialogue_editor_state.show_preview);
    assert!(app
        .editor_registry
        .dialogue_editor_state
        .import_buffer
        .is_empty());
    assert!(!app.editor_registry.dialogue_editor_state.show_import_dialog);
}

#[test]
fn test_dialogue_list_operations() {
    let mut app = CampaignBuilderApp::default();

    // Add dialogues
    let dialogue1 = DialogueTree::new(1, "Merchant Greeting", 1);
    let dialogue2 = DialogueTree::new(2, "Guard Warning", 1);

    app.campaign_data.dialogues.push(dialogue1);
    app.campaign_data.dialogues.push(dialogue2);
    app.editor_registry
        .dialogue_editor_state
        .load_dialogues(app.campaign_data.dialogues.clone());

    assert_eq!(app.editor_registry.dialogue_editor_state.dialogues.len(), 2);
    assert_eq!(
        app.editor_registry.dialogue_editor_state.dialogues[0].name,
        "Merchant Greeting"
    );
    assert_eq!(
        app.editor_registry.dialogue_editor_state.dialogues[1].name,
        "Guard Warning"
    );
}

#[test]
fn test_dialogue_search_filter() {
    let mut app = CampaignBuilderApp::default();

    let dialogue1 = DialogueTree::new(1, "Merchant Greeting", 1);
    let dialogue2 = DialogueTree::new(2, "Guard Warning", 1);

    app.campaign_data.dialogues.push(dialogue1);
    app.campaign_data.dialogues.push(dialogue2);
    app.editor_registry
        .dialogue_editor_state
        .load_dialogues(app.campaign_data.dialogues.clone());

    // No filter - all dialogues
    let filtered = app
        .editor_registry
        .dialogue_editor_state
        .filtered_dialogues();
    assert_eq!(filtered.len(), 2);

    // Filter by name
    app.editor_registry.dialogue_editor_state.search_filter = "merchant".to_string();
    let filtered = app
        .editor_registry
        .dialogue_editor_state
        .filtered_dialogues();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].1.name, "Merchant Greeting");

    // Filter by ID
    app.editor_registry.dialogue_editor_state.search_filter = "2".to_string();
    let filtered = app
        .editor_registry
        .dialogue_editor_state
        .filtered_dialogues();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].1.id, 2);
}

#[test]
fn test_dialogue_start_new() {
    let mut app = CampaignBuilderApp::default();

    app.editor_registry
        .dialogue_editor_state
        .start_new_dialogue();

    assert_eq!(
        app.editor_registry.dialogue_editor_state.mode,
        dialogue_editor::DialogueEditorMode::Creating
    );
    assert!(app
        .editor_registry
        .dialogue_editor_state
        .dialogue_buffer
        .id
        .is_empty());
    assert!(app
        .editor_registry
        .dialogue_editor_state
        .dialogue_buffer
        .name
        .is_empty());
    assert!(app
        .editor_registry
        .dialogue_editor_state
        .validation_errors
        .is_empty());
}

#[test]
fn test_dialogue_edit_existing() {
    let mut app = CampaignBuilderApp::default();

    let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
    dialogue.speaker_name = Some("Merchant".to_string());
    dialogue.repeatable = true;

    app.campaign_data.dialogues.push(dialogue);
    app.editor_registry
        .dialogue_editor_state
        .load_dialogues(app.campaign_data.dialogues.clone());

    app.editor_registry
        .dialogue_editor_state
        .start_edit_dialogue(0);

    assert_eq!(
        app.editor_registry.dialogue_editor_state.mode,
        dialogue_editor::DialogueEditorMode::Editing
    );
    assert_eq!(
        app.editor_registry.dialogue_editor_state.dialogue_buffer.id,
        "1"
    );
    assert_eq!(
        app.editor_registry
            .dialogue_editor_state
            .dialogue_buffer
            .name,
        "Test Dialogue"
    );
    assert_eq!(
        app.editor_registry
            .dialogue_editor_state
            .dialogue_buffer
            .speaker_name,
        "Merchant"
    );
    assert!(
        app.editor_registry
            .dialogue_editor_state
            .dialogue_buffer
            .repeatable
    );
}

#[test]
fn test_dialogue_save_new() {
    let mut app = CampaignBuilderApp::default();

    app.editor_registry
        .dialogue_editor_state
        .start_new_dialogue();
    app.editor_registry.dialogue_editor_state.dialogue_buffer.id = "10".to_string();
    app.editor_registry
        .dialogue_editor_state
        .dialogue_buffer
        .name = "New Dialogue".to_string();
    app.editor_registry
        .dialogue_editor_state
        .dialogue_buffer
        .speaker_name = "NPC".to_string();

    let result = app.editor_registry.dialogue_editor_state.save_dialogue();

    assert!(result.is_ok());
    assert_eq!(app.editor_registry.dialogue_editor_state.dialogues.len(), 1);
    assert_eq!(
        app.editor_registry.dialogue_editor_state.dialogues[0].id,
        10
    );
    assert_eq!(
        app.editor_registry.dialogue_editor_state.dialogues[0].name,
        "New Dialogue"
    );
    assert_eq!(
        app.editor_registry.dialogue_editor_state.dialogues[0].speaker_name,
        Some("NPC".to_string())
    );
    assert_eq!(
        app.editor_registry.dialogue_editor_state.mode,
        dialogue_editor::DialogueEditorMode::List
    );
}

#[test]
fn test_dialogue_delete() {
    let mut app = CampaignBuilderApp::default();

    let dialogue1 = DialogueTree::new(1, "Dialogue 1", 1);
    let dialogue2 = DialogueTree::new(2, "Dialogue 2", 1);

    app.campaign_data.dialogues.push(dialogue1);
    app.campaign_data.dialogues.push(dialogue2);
    app.editor_registry
        .dialogue_editor_state
        .load_dialogues(app.campaign_data.dialogues.clone());

    assert_eq!(app.editor_registry.dialogue_editor_state.dialogues.len(), 2);

    app.editor_registry.dialogue_editor_state.delete_dialogue(0);

    assert_eq!(app.editor_registry.dialogue_editor_state.dialogues.len(), 1);
    assert_eq!(app.editor_registry.dialogue_editor_state.dialogues[0].id, 2);
}

#[test]
fn test_dialogue_cancel_edit() {
    let mut app = CampaignBuilderApp::default();

    app.editor_registry
        .dialogue_editor_state
        .start_new_dialogue();
    app.editor_registry
        .dialogue_editor_state
        .dialogue_buffer
        .name = "Test".to_string();

    app.editor_registry.dialogue_editor_state.cancel_edit();

    assert_eq!(
        app.editor_registry.dialogue_editor_state.mode,
        dialogue_editor::DialogueEditorMode::List
    );
    assert!(app
        .editor_registry
        .dialogue_editor_state
        .dialogue_buffer
        .name
        .is_empty());
    assert!(app
        .editor_registry
        .dialogue_editor_state
        .selected_dialogue
        .is_none());
}

#[test]
fn test_dialogue_add_node() {
    let mut app = CampaignBuilderApp::default();

    let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
    let root_node = antares::domain::dialogue::DialogueNode::new(1, "Root node text");
    dialogue.add_node(root_node);

    app.campaign_data.dialogues.push(dialogue);
    app.editor_registry
        .dialogue_editor_state
        .load_dialogues(app.campaign_data.dialogues.clone());
    app.editor_registry
        .dialogue_editor_state
        .start_edit_dialogue(0);

    // Add new node
    app.editor_registry.dialogue_editor_state.node_buffer.id = "2".to_string();
    app.editor_registry.dialogue_editor_state.node_buffer.text = "New node text".to_string();
    app.editor_registry
        .dialogue_editor_state
        .node_buffer
        .is_terminal = true;

    let result = app.editor_registry.dialogue_editor_state.add_node();

    assert!(result.is_ok());
    assert_eq!(
        app.editor_registry.dialogue_editor_state.dialogues[0]
            .nodes
            .len(),
        2
    );
    assert!(app.editor_registry.dialogue_editor_state.dialogues[0]
        .nodes
        .contains_key(&2));
}

#[test]
fn test_dialogue_add_choice() {
    let mut app = CampaignBuilderApp::default();

    let mut dialogue = DialogueTree::new(1, "Test Dialogue", 1);
    let root_node = antares::domain::dialogue::DialogueNode::new(1, "Root");
    let target_node = antares::domain::dialogue::DialogueNode::new(2, "Target");
    dialogue.add_node(root_node);
    dialogue.add_node(target_node);

    app.campaign_data.dialogues.push(dialogue);
    app.editor_registry
        .dialogue_editor_state
        .load_dialogues(app.campaign_data.dialogues.clone());
    app.editor_registry
        .dialogue_editor_state
        .start_edit_dialogue(0);

    // Select node 1 to add choice to
    app.editor_registry.dialogue_editor_state.selected_node = Some(1);
    app.editor_registry.dialogue_editor_state.choice_buffer.text = "Go to node 2".to_string();
    app.editor_registry
        .dialogue_editor_state
        .choice_buffer
        .target_node = "2".to_string();

    let result = app.editor_registry.dialogue_editor_state.add_choice();

    assert!(result.is_ok());
    let node = app.editor_registry.dialogue_editor_state.dialogues[0]
        .nodes
        .get(&1)
        .unwrap();
    assert_eq!(node.choices.len(), 1);
    assert_eq!(node.choices[0].text, "Go to node 2");
}

#[test]
fn test_dialogue_next_available_id() {
    let mut app = CampaignBuilderApp::default();

    // Empty list - should return 1
    assert_eq!(
        app.editor_registry
            .dialogue_editor_state
            .next_available_dialogue_id(),
        1
    );

    // With dialogues - should return max + 1
    app.campaign_data
        .dialogues
        .push(DialogueTree::new(1, "D1", 1));
    app.campaign_data
        .dialogues
        .push(DialogueTree::new(5, "D2", 1));
    app.campaign_data
        .dialogues
        .push(DialogueTree::new(3, "D3", 1));
    app.editor_registry
        .dialogue_editor_state
        .load_dialogues(app.campaign_data.dialogues.clone());

    assert_eq!(
        app.editor_registry
            .dialogue_editor_state
            .next_available_dialogue_id(),
        6
    );
}

#[test]
fn test_dialogue_editor_mode_transitions() {
    let mut app = CampaignBuilderApp::default();

    // Start in list mode
    assert_eq!(
        app.editor_registry.dialogue_editor_state.mode,
        dialogue_editor::DialogueEditorMode::List
    );

    // Transition to creating
    app.editor_registry
        .dialogue_editor_state
        .start_new_dialogue();
    assert_eq!(
        app.editor_registry.dialogue_editor_state.mode,
        dialogue_editor::DialogueEditorMode::Creating
    );

    // Cancel back to list
    app.editor_registry.dialogue_editor_state.cancel_edit();
    assert_eq!(
        app.editor_registry.dialogue_editor_state.mode,
        dialogue_editor::DialogueEditorMode::List
    );
}

#[test]
fn test_dialogue_node_terminal_flag() {
    let mut app = CampaignBuilderApp::default();

    let mut dialogue = DialogueTree::new(1, "Test", 1);
    let mut node = antares::domain::dialogue::DialogueNode::new(1, "Terminal node");
    node.is_terminal = true;
    dialogue.add_node(node);

    app.campaign_data.dialogues.push(dialogue);
    app.editor_registry
        .dialogue_editor_state
        .load_dialogues(app.campaign_data.dialogues.clone());

    let node = app.editor_registry.dialogue_editor_state.dialogues[0]
        .nodes
        .get(&1)
        .unwrap();
    assert!(node.is_terminal);
}

#[test]
fn test_dialogue_speaker_override() {
    let mut app = CampaignBuilderApp::default();

    let mut dialogue = DialogueTree::new(1, "Test", 1);
    dialogue.speaker_name = Some("Default Speaker".to_string());

    let mut node = antares::domain::dialogue::DialogueNode::new(1, "Override test");
    node.speaker_override = Some("Special Speaker".to_string());
    dialogue.add_node(node);

    app.campaign_data.dialogues.push(dialogue);
    app.editor_registry
        .dialogue_editor_state
        .load_dialogues(app.campaign_data.dialogues.clone());

    let node = app.editor_registry.dialogue_editor_state.dialogues[0]
        .nodes
        .get(&1)
        .unwrap();
    assert_eq!(node.speaker_override, Some("Special Speaker".to_string()));
}

#[test]
fn test_dialogue_associated_quest() {
    let mut app = CampaignBuilderApp::default();

    app.editor_registry
        .dialogue_editor_state
        .start_new_dialogue();
    app.editor_registry.dialogue_editor_state.dialogue_buffer.id = "10".to_string();
    app.editor_registry
        .dialogue_editor_state
        .dialogue_buffer
        .name = "Quest Dialogue".to_string();
    app.editor_registry
        .dialogue_editor_state
        .dialogue_buffer
        .associated_quest = "5".to_string();

    let result = app.editor_registry.dialogue_editor_state.save_dialogue();

    assert!(result.is_ok());
    assert_eq!(
        app.editor_registry.dialogue_editor_state.dialogues[0].associated_quest,
        Some(5)
    );
}

#[test]
fn test_dialogue_repeatable_flag() {
    let mut dialogue1 = DialogueTree::new(1, "One-time", 1);
    dialogue1.repeatable = false;

    let mut dialogue2 = DialogueTree::new(2, "Repeatable", 1);
    dialogue2.repeatable = true;

    assert!(!dialogue1.repeatable);
    assert!(dialogue2.repeatable);
}

#[test]
fn test_dialogue_import_buffer() {
    let app = CampaignBuilderApp::default();
    // Import buffer and dialog state are managed by DialogueEditorState
    assert!(app
        .editor_registry
        .dialogue_editor_state
        .import_buffer
        .is_empty());
    assert!(!app.editor_registry.dialogue_editor_state.show_import_dialog);
}

#[test]
fn test_dialogue_validation_errors_cleared() {
    let mut app = CampaignBuilderApp::default();

    app.editor_registry
        .dialogue_editor_state
        .validation_errors
        .push("Test error".to_string());
    assert!(!app
        .editor_registry
        .dialogue_editor_state
        .validation_errors
        .is_empty());

    app.editor_registry
        .dialogue_editor_state
        .start_new_dialogue();
    assert!(app
        .editor_registry
        .dialogue_editor_state
        .validation_errors
        .is_empty());
}

// ===== Conditions Editor QoL Tests =====

#[test]
fn test_effect_type_filter_matches() {
    use antares::domain::conditions::{ConditionDefinition, ConditionDuration, ConditionEffect};
    use antares::domain::types::DiceRoll;
    use campaign_builder::conditions_editor::EffectTypeFilter;

    // Condition with AttributeModifier
    let attr_cond = ConditionDefinition {
        id: "attr".to_string(),
        name: "Attribute Buff".to_string(),
        description: "".to_string(),
        effects: vec![ConditionEffect::AttributeModifier {
            attribute: "might".to_string(),
            value: 5,
        }],
        default_duration: ConditionDuration::Rounds(3),
        icon_id: None,
    };

    // Condition with DOT
    let dot_cond = ConditionDefinition {
        id: "dot".to_string(),
        name: "Burning".to_string(),
        description: "".to_string(),
        effects: vec![ConditionEffect::DamageOverTime {
            damage: DiceRoll::new(1, 6, 0),
            element: "fire".to_string(),
        }],
        default_duration: ConditionDuration::Rounds(3),
        icon_id: None,
    };

    // Condition with HOT
    let hot_cond = ConditionDefinition {
        id: "hot".to_string(),
        name: "Regeneration".to_string(),
        description: "".to_string(),
        effects: vec![ConditionEffect::HealOverTime {
            amount: DiceRoll::new(1, 4, 1),
        }],
        default_duration: ConditionDuration::Rounds(5),
        icon_id: None,
    };

    // Empty condition
    let empty_cond = ConditionDefinition {
        id: "empty".to_string(),
        name: "Empty".to_string(),
        description: "".to_string(),
        effects: vec![],
        default_duration: ConditionDuration::Instant,
        icon_id: None,
    };

    // Test All filter matches everything
    assert!(EffectTypeFilter::All.matches(&attr_cond));
    assert!(EffectTypeFilter::All.matches(&dot_cond));
    assert!(EffectTypeFilter::All.matches(&hot_cond));
    assert!(EffectTypeFilter::All.matches(&empty_cond));

    // Test specific filters
    assert!(EffectTypeFilter::AttributeModifier.matches(&attr_cond));
    assert!(!EffectTypeFilter::AttributeModifier.matches(&dot_cond));

    assert!(EffectTypeFilter::DamageOverTime.matches(&dot_cond));
    assert!(!EffectTypeFilter::DamageOverTime.matches(&hot_cond));

    assert!(EffectTypeFilter::HealOverTime.matches(&hot_cond));
    assert!(!EffectTypeFilter::HealOverTime.matches(&attr_cond));

    // Empty condition doesn't match specific filters
    assert!(!EffectTypeFilter::AttributeModifier.matches(&empty_cond));
    assert!(!EffectTypeFilter::DamageOverTime.matches(&empty_cond));
}

#[test]
fn test_condition_sort_order_as_str() {
    use campaign_builder::conditions_editor::ConditionSortOrder;

    assert_eq!(ConditionSortOrder::NameAsc.as_str(), "Name (A-Z)");
    assert_eq!(ConditionSortOrder::NameDesc.as_str(), "Name (Z-A)");
    assert_eq!(ConditionSortOrder::IdAsc.as_str(), "ID (A-Z)");
    assert_eq!(ConditionSortOrder::IdDesc.as_str(), "ID (Z-A)");
    assert_eq!(ConditionSortOrder::EffectCount.as_str(), "Effect Count");
}

#[test]
fn test_condition_statistics_computation() {
    use antares::domain::conditions::{ConditionDefinition, ConditionDuration, ConditionEffect};
    use antares::domain::types::DiceRoll;
    use campaign_builder::conditions_editor::compute_condition_statistics;

    let conditions = vec![
        ConditionDefinition {
            id: "c1".to_string(),
            name: "Buff".to_string(),
            description: "".to_string(),
            effects: vec![ConditionEffect::AttributeModifier {
                attribute: "might".to_string(),
                value: 5,
            }],
            default_duration: ConditionDuration::Rounds(3),
            icon_id: None,
        },
        ConditionDefinition {
            id: "c2".to_string(),
            name: "Burning".to_string(),
            description: "".to_string(),
            effects: vec![ConditionEffect::DamageOverTime {
                damage: DiceRoll::new(1, 6, 0),
                element: "fire".to_string(),
            }],
            default_duration: ConditionDuration::Rounds(3),
            icon_id: None,
        },
        ConditionDefinition {
            id: "c3".to_string(),
            name: "Multi".to_string(),
            description: "".to_string(),
            effects: vec![
                ConditionEffect::StatusEffect("poisoned".to_string()),
                ConditionEffect::DamageOverTime {
                    damage: DiceRoll::new(1, 4, 0),
                    element: "poison".to_string(),
                },
            ],
            default_duration: ConditionDuration::Rounds(5),
            icon_id: None,
        },
        ConditionDefinition {
            id: "c4".to_string(),
            name: "Empty".to_string(),
            description: "".to_string(),
            effects: vec![],
            default_duration: ConditionDuration::Instant,
            icon_id: None,
        },
    ];

    let stats = compute_condition_statistics(&conditions);

    assert_eq!(stats.total, 4);
    assert_eq!(stats.attribute_count, 1);
    assert_eq!(stats.dot_count, 2); // c2 has 1, c3 has 1
    assert_eq!(stats.status_count, 1);
    assert_eq!(stats.hot_count, 0);
    assert_eq!(stats.empty_count, 1);
    assert_eq!(stats.multi_effect_count, 1); // only c3 has multiple effects
}

#[test]
fn test_conditions_editor_navigation_request() {
    let mut state = conditions_editor::ConditionsEditorState::new();

    // Initially no navigation request
    assert!(state.navigate_to_spell.is_none());

    // Set a navigation request
    state.navigate_to_spell = Some("Fireball".to_string());
    assert_eq!(state.navigate_to_spell, Some("Fireball".to_string()));

    // Take clears the request
    let nav = state.navigate_to_spell.take();
    assert_eq!(nav, Some("Fireball".to_string()));
    assert!(state.navigate_to_spell.is_none());
}

#[test]
fn test_conditions_editor_state_qol_defaults() {
    use campaign_builder::conditions_editor::{ConditionSortOrder, EffectTypeFilter};

    let state = conditions_editor::ConditionsEditorState::new();

    // Verify new QoL fields have correct defaults
    assert_eq!(state.filter_effect_type, EffectTypeFilter::All);
    assert_eq!(state.sort_order, ConditionSortOrder::NameAsc);
    assert!(!state.show_statistics);
    assert!(state.navigate_to_spell.is_none());
}

#[test]
fn test_effect_type_filter_all_variants() {
    use campaign_builder::conditions_editor::EffectTypeFilter;

    let all = EffectTypeFilter::all();
    assert_eq!(all.len(), 5);
    assert_eq!(all[0], EffectTypeFilter::All);
    assert_eq!(all[1], EffectTypeFilter::AttributeModifier);
    assert_eq!(all[2], EffectTypeFilter::StatusEffect);
    assert_eq!(all[3], EffectTypeFilter::DamageOverTime);
    assert_eq!(all[4], EffectTypeFilter::HealOverTime);
}

#[test]
fn test_effect_type_filter_as_str() {
    use campaign_builder::conditions_editor::EffectTypeFilter;

    assert_eq!(EffectTypeFilter::All.as_str(), "All");
    assert_eq!(EffectTypeFilter::AttributeModifier.as_str(), "Attribute");
    assert_eq!(EffectTypeFilter::StatusEffect.as_str(), "Status");
    assert_eq!(EffectTypeFilter::DamageOverTime.as_str(), "DOT");
    assert_eq!(EffectTypeFilter::HealOverTime.as_str(), "HOT");
}

// =========================================================================
// Testing Infrastructure
// Pattern Matching and Compliance Tests
// =========================================================================

#[test]
fn test_pattern_matcher_combobox_id_salt_detection() {
    use campaign_builder::test_utils::PatternMatcher;

    let matcher = PatternMatcher::combobox_id_salt();

    // Valid patterns
    assert!(matcher.matches(r#"ComboBox::from_id_salt("test_combo")"#));
    assert!(matcher.matches(r#"egui::ComboBox::from_id_salt("another_combo")"#));
    assert!(matcher.matches(r#"ComboBox::from_id_salt('single_quotes')"#));

    // Invalid patterns (should not match)
    assert!(!matcher.matches("ComboBox::new()"));
    assert!(!matcher.matches("from_id_salt without ComboBox"));
}

#[test]
fn test_pattern_matcher_combobox_from_label_detection() {
    use campaign_builder::test_utils::PatternMatcher;

    let matcher = PatternMatcher::combobox_from_label();

    // Should detect from_label usage
    assert!(matcher.matches(r#"ComboBox::from_label("bad_pattern")"#));
    assert!(matcher.matches(r#"egui::ComboBox::from_label("also_bad")"#));

    // Should not match from_id_salt
    assert!(!matcher.matches(r#"ComboBox::from_id_salt("good")"#));
}

#[test]
fn test_pattern_matcher_pub_fn_show_detection() {
    use campaign_builder::test_utils::PatternMatcher;

    let matcher = PatternMatcher::pub_fn_show();

    // Valid patterns
    assert!(matcher.matches("pub fn show(&mut self, ui: &mut Ui)"));
    assert!(matcher.matches("    pub fn show("));
    assert!(matcher.matches("pub fn show(&self)"));

    // Invalid patterns
    assert!(!matcher.matches("fn show(")); // not public
    assert!(!matcher.matches("pub fn show_items(")); // different function name
    assert!(!matcher.matches("pub fn showing(")); // partial match
}

#[test]
fn test_pattern_matcher_editor_state_struct_detection() {
    use campaign_builder::test_utils::PatternMatcher;

    let matcher = PatternMatcher::editor_state_struct();

    // Valid patterns
    assert!(matcher.matches("pub struct ItemsEditorState {"));
    assert!(matcher.matches("pub struct SpellsEditorState {"));
    assert!(matcher.matches("pub struct MonstersEditorState {"));

    // Invalid patterns
    assert!(!matcher.matches("struct ItemsEditorState {")); // not pub
    assert!(!matcher.matches("pub struct SomeOtherState {")); // not *EditorState
}

#[test]
fn test_source_file_creation_and_analysis() {
    use campaign_builder::test_utils::SourceFile;

    let content = r#"
pub struct TestEditorState {
items: Vec<String>,
}

impl TestEditorState {
pub fn new() -> Self {
    Self { items: vec![] }
}

pub fn show(&mut self, ui: &mut egui::Ui) {
    // editor content
}
}

#[cfg(test)]
mod tests {
#[test]
fn test_something() {}
}
"#;

    let file = SourceFile::new("test_editor.rs", content);

    assert_eq!(file.name, "test_editor");
    assert!(file.line_count() > 10);
    assert!(file.contains_pattern("pub struct"));
    assert!(file.contains_pattern("pub fn show"));
}

#[test]
fn test_editor_compliance_check_detects_issues() {
    use campaign_builder::test_utils::{check_editor_compliance, SourceFile};

    // Editor with from_label violation
    let bad_content = r#"
impl BadEditor {
pub fn show(&mut self, ui: &mut egui::Ui) {
    egui::ComboBox::from_label("Bad ID")
        .show_ui(ui, |ui| {});
}
}
"#;

    let file = SourceFile::new("bad_editor.rs", bad_content);
    let result = check_editor_compliance(&file);

    assert_eq!(result.combobox_from_label_count, 1);
    assert!(!result.is_compliant());
    assert!(result.issues.iter().any(|i| i.contains("from_label")));
}

#[test]
fn test_editor_compliance_check_passes_good_editor() {
    use campaign_builder::test_utils::{check_editor_compliance, SourceFile};

    let good_content = r#"
pub struct GoodEditorState {
data: Vec<String>,
}

impl GoodEditorState {
pub fn new() -> Self {
    Self { data: vec![] }
}

pub fn show(&mut self, ui: &mut egui::Ui) {
    egui::ComboBox::from_id_salt("good_combo")
        .show_ui(ui, |ui| {});
    EditorToolbar::new("Good").show(ui);
    ActionButtons::new().show(ui);
    TwoColumnLayout::new("good").show_split(ui, |left_ui| {}, |right_ui| {});
}
}

#[cfg(test)]
mod tests {
#[test]
fn test_good_editor() {}
}
"#;

    let file = SourceFile::new("good_editor.rs", good_content);
    let result = check_editor_compliance(&file);

    assert!(result.has_show_method);
    assert!(result.has_new_method);
    assert!(result.has_state_struct);
    assert!(result.has_tests);
    assert_eq!(result.combobox_from_label_count, 0);
    assert_eq!(result.combobox_id_salt_count, 1);
}

#[test]
fn test_compliance_score_calculation() {
    use campaign_builder::test_utils::EditorComplianceResult;

    // Full compliance = 100 points
    let full = EditorComplianceResult {
        editor_name: "full".to_string(),
        has_show_method: true,        // 20
        has_new_method: true,         // 10
        has_state_struct: true,       // 15
        uses_toolbar: true,           // 15
        uses_action_buttons: true,    // 10
        uses_two_column_layout: true, // 10
        has_tests: true,              // 10
        combobox_id_salt_count: 5,
        combobox_from_label_count: 0, // 10
        issues: vec![],
    };
    assert_eq!(full.compliance_score(), 100);

    // Partial compliance
    let partial = EditorComplianceResult {
        editor_name: "partial".to_string(),
        has_show_method: true,         // 20
        has_new_method: false,         // 0
        has_state_struct: false,       // 0
        uses_toolbar: true,            // 15
        uses_action_buttons: false,    // 0
        uses_two_column_layout: false, // 0
        has_tests: true,               // 10
        combobox_id_salt_count: 0,
        combobox_from_label_count: 0, // 10
        issues: vec![],
    };
    assert_eq!(partial.compliance_score(), 55);
}

#[test]
fn test_collect_combobox_id_salts() {
    use campaign_builder::test_utils::{collect_combobox_id_salts, SourceFile};

    let content = r#"
egui::ComboBox::from_id_salt("difficulty_combo")
.show_ui(ui, |ui| {});
egui::ComboBox::from_id_salt("terrain_combo")
.show_ui(ui, |ui| {});
egui::ComboBox::from_id_salt("wall_combo")
.show_ui(ui, |ui| {});
"#;

    let file = SourceFile::new("test.rs", content);
    let salts = collect_combobox_id_salts(&file);

    assert_eq!(salts.len(), 3);
    assert!(salts.contains(&"difficulty_combo".to_string()));
    assert!(salts.contains(&"terrain_combo".to_string()));
    assert!(salts.contains(&"wall_combo".to_string()));
}

#[test]
fn test_find_duplicate_combobox_ids_detects_conflicts() {
    use campaign_builder::test_utils::{find_duplicate_combobox_ids, SourceFile};

    let file1 = SourceFile::new(
        "editor1.rs",
        r#"egui::ComboBox::from_id_salt("duplicate_id")"#,
    );
    let file2 = SourceFile::new(
        "editor2.rs",
        r#"egui::ComboBox::from_id_salt("duplicate_id")"#,
    );
    let file3 = SourceFile::new("editor3.rs", r#"egui::ComboBox::from_id_salt("unique_id")"#);

    let files = vec![file1, file2, file3];
    let duplicates = find_duplicate_combobox_ids(&files);

    // Only "duplicate_id" should be reported as duplicate
    assert_eq!(duplicates.len(), 1);
    assert!(duplicates.contains_key("duplicate_id"));
    assert_eq!(duplicates["duplicate_id"].len(), 2);
    assert!(!duplicates.contains_key("unique_id"));
}

#[test]
fn test_compliance_summary_calculation() {
    use campaign_builder::test_utils::{ComplianceSummary, EditorComplianceResult};
    use std::collections::HashMap;

    let mut results = HashMap::new();

    results.insert(
        "compliant_editor".to_string(),
        EditorComplianceResult {
            editor_name: "compliant_editor".to_string(),
            has_show_method: true,
            has_new_method: true,
            has_state_struct: true,
            uses_toolbar: true,
            uses_action_buttons: true,
            uses_two_column_layout: true,
            has_tests: true,
            combobox_id_salt_count: 2,
            combobox_from_label_count: 0,
            issues: vec![],
        },
    );

    results.insert(
        "noncompliant_editor".to_string(),
        EditorComplianceResult {
            editor_name: "noncompliant_editor".to_string(),
            has_show_method: true,
            has_new_method: false,
            has_state_struct: false,
            uses_toolbar: false,
            uses_action_buttons: false,
            uses_two_column_layout: false,
            has_tests: false,
            combobox_id_salt_count: 0,
            combobox_from_label_count: 2,
            issues: vec!["Missing tests".to_string(), "Uses from_label".to_string()],
        },
    );

    let summary = ComplianceSummary::from_results(&results);

    assert_eq!(summary.total_editors, 2);
    assert_eq!(summary.compliant_editors, 1);
    assert_eq!(summary.total_issues, 2);
    assert_eq!(summary.from_label_violations, 2);
    assert!(summary.average_score > 0.0);
}

#[test]
fn test_pattern_match_line_numbers() {
    use campaign_builder::test_utils::PatternMatcher;

    let content = r#"line 1
line 2
ComboBox::from_id_salt("test")
line 4
line 5
ComboBox::from_id_salt("another")
line 7"#;

    let matcher = PatternMatcher::combobox_id_salt();
    let matches = matcher.find_matches(content);

    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].line_number, 3);
    assert_eq!(matches[1].line_number, 6);
}

#[test]
fn test_pattern_matcher_test_annotation() {
    use campaign_builder::test_utils::PatternMatcher;

    let matcher = PatternMatcher::test_annotation();
    let content = r#"
#[test]
fn test_something() {}

#[test]
fn test_another() {}
"#;

    assert_eq!(matcher.count_matches(content), 2);
}

#[test]
fn test_pattern_matcher_toolbar_usage() {
    use campaign_builder::test_utils::PatternMatcher;

    let matcher = PatternMatcher::editor_toolbar_usage();

    assert!(matcher.matches("EditorToolbar::new(\"Items\")"));
    assert!(matcher.matches("    EditorToolbar::new("));
    assert!(!matcher.matches("Toolbar::new(")); // different struct
}

#[test]
fn test_pattern_matcher_action_buttons_usage() {
    use campaign_builder::test_utils::PatternMatcher;

    let matcher = PatternMatcher::action_buttons_usage();

    assert!(matcher.matches("ActionButtons::new()"));
    assert!(matcher.matches("    ActionButtons::new("));
    assert!(!matcher.matches("Buttons::new(")); // different struct
}

#[test]
fn test_pattern_matcher_two_column_layout_usage() {
    use campaign_builder::test_utils::PatternMatcher;

    let matcher = PatternMatcher::two_column_layout_usage();

    assert!(matcher.matches("TwoColumnLayout::new(\"items\")"));
    assert!(matcher.matches("    TwoColumnLayout::new("));
    assert!(!matcher.matches("ColumnLayout::new(")); // different struct
}

#[test]
fn test_editor_compliance_result_is_compliant() {
    use campaign_builder::test_utils::EditorComplianceResult;

    // Compliant: no issues and no from_label
    let compliant = EditorComplianceResult {
        editor_name: "test".to_string(),
        has_show_method: true,
        has_new_method: true,
        has_state_struct: true,
        uses_toolbar: true,
        uses_action_buttons: true,
        uses_two_column_layout: true,
        has_tests: true,
        combobox_id_salt_count: 1,
        combobox_from_label_count: 0,
        issues: vec![],
    };
    assert!(compliant.is_compliant());

    // Not compliant: has from_label usage
    let with_from_label = EditorComplianceResult {
        editor_name: "test".to_string(),
        has_show_method: true,
        has_new_method: true,
        has_state_struct: true,
        uses_toolbar: true,
        uses_action_buttons: true,
        uses_two_column_layout: true,
        has_tests: true,
        combobox_id_salt_count: 1,
        combobox_from_label_count: 1,
        issues: vec![],
    };
    assert!(!with_from_label.is_compliant());

    // Not compliant: has issues
    let with_issues = EditorComplianceResult {
        editor_name: "test".to_string(),
        has_show_method: true,
        has_new_method: true,
        has_state_struct: true,
        uses_toolbar: true,
        uses_action_buttons: true,
        uses_two_column_layout: true,
        has_tests: true,
        combobox_id_salt_count: 1,
        combobox_from_label_count: 0,
        issues: vec!["Some issue".to_string()],
    };
    assert!(!with_issues.is_compliant());
}

#[test]
fn test_compliance_summary_to_string_format() {
    use campaign_builder::test_utils::ComplianceSummary;

    let summary = ComplianceSummary {
        total_editors: 5,
        compliant_editors: 4,
        total_issues: 2,
        from_label_violations: 1,
        average_score: 90.5,
        all_issues: vec!["issue1".to_string(), "issue2".to_string()],
    };

    let output = summary.to_string();

    assert!(output.contains("Total Editors: 5"));
    assert!(output.contains("Compliant: 4"));
    assert!(output.contains("Total Issues: 2"));
    assert!(output.contains("from_label Violations: 1"));
    assert!(output.contains("Average Score: 90.5"));
}

// =========================================================================
// Creature Template Browser Tests
// =========================================================================

#[test]
fn test_creature_template_browser_defaults_to_hidden() {
    let app = CampaignBuilderApp::default();
    assert!(
        !app.ui_state.show_creature_template_browser,
        "Creature template browser should be hidden on default construction"
    );
}

#[test]
fn test_creature_template_registry_non_empty_on_default() {
    let app = CampaignBuilderApp::default();
    assert!(
        !app.creature_template_registry.is_empty(),
        "Creature template registry should contain templates after initialization"
    );
    // The initialize_template_registry function registers all 24 known templates.
    assert!(
        app.creature_template_registry.len() >= 24,
        "Expected at least 24 registered creature templates, got {}",
        app.creature_template_registry.len()
    );
}

#[test]
fn test_creature_template_sentinel_sets_show_flag() {
    let mut app = CampaignBuilderApp::default();

    // The sentinel is returned by the creatures editor when the user clicks
    // "Browse Templates".  The update() loop checks for this sentinel and sets
    // show_creature_template_browser = true instead of writing to status_message.

    // Simulate what update() does when it receives the sentinel from show().
    let sentinel_msg = creatures_editor::OPEN_CREATURE_TEMPLATES_SENTINEL.to_string();
    if sentinel_msg == creatures_editor::OPEN_CREATURE_TEMPLATES_SENTINEL {
        app.ui_state.show_creature_template_browser = true;
    } else {
        app.ui_state.status_message = sentinel_msg;
    }

    assert!(
        app.ui_state.show_creature_template_browser,
        "show_creature_template_browser should be true after receiving the sentinel"
    );
    assert!(
        app.ui_state.status_message.is_empty(),
        "status_message should NOT be set when the sentinel is received"
    );
}

#[test]
fn test_sync_creature_id_manager_from_creatures_tracks_current_registry() {
    let mut app = CampaignBuilderApp::default();
    let template_id = app
        .creature_template_registry
        .all_templates()
        .first()
        .expect("template registry should not be empty")
        .metadata
        .id
        .clone();

    let creature_one = app
        .creature_template_registry
        .generate(&template_id, "Phase1 Sync One", 1)
        .expect("template generation should succeed");
    let creature_two = app
        .creature_template_registry
        .generate(&template_id, "Phase1 Sync Two", 2)
        .expect("template generation should succeed");

    app.campaign_data.creatures = vec![creature_one, creature_two];
    app.editor_registry
        .creatures_editor_state
        .id_manager
        .update_from_registry(&[]);

    app.sync_creature_id_manager_from_creatures();

    assert!(app
        .editor_registry
        .creatures_editor_state
        .id_manager
        .is_id_used(1));
    assert!(app
        .editor_registry
        .creatures_editor_state
        .id_manager
        .is_id_used(2));
}

#[test]
fn test_next_available_creature_id_refreshes_stale_id_manager_state() {
    let mut app = CampaignBuilderApp::default();
    let template_id = app
        .creature_template_registry
        .all_templates()
        .first()
        .expect("template registry should not be empty")
        .metadata
        .id
        .clone();

    let existing = app
        .creature_template_registry
        .generate(&template_id, "Existing Monster", 1)
        .expect("template generation should succeed");
    app.campaign_data.creatures = vec![existing];

    // Simulate stale manager state: this mirrors opening templates directly
    // from Tools before visiting the Creatures tab.
    app.editor_registry
        .creatures_editor_state
        .id_manager
        .update_from_registry(&[]);

    let next_id = app
        .next_available_creature_id_for_category(creature_id_manager::CreatureCategory::Monsters)
        .expect("ID suggestion should succeed");

    assert_eq!(
        next_id, 2,
        "next available monster ID should skip used ID 1"
    );
}

// =========================================================================
// Stock Templates Editor Tests
// =========================================================================

#[test]
fn test_validate_npc_ids_detects_unknown_stock_template() {
    use antares::domain::world::NpcDefinition;

    let mut app = CampaignBuilderApp::default();

    // Add an NPC that references a non-existent stock template
    let mut npc = NpcDefinition::new("merchant_bad", "Bad Merchant", "bad.png");
    npc.is_merchant = true;
    npc.stock_template = Some("nonexistent_template".to_string());
    app.editor_registry.npc_editor_state.npcs.push(npc);

    // stock_templates is empty — reference is unresolvable
    app.campaign_data.stock_templates = vec![];

    let errors = app.validate_npc_ids();
    assert!(
        errors.iter().any(|e| e
            .message
            .contains("unknown stock template 'nonexistent_template'")),
        "expected unknown-stock-template error, got: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_npc_ids_valid_stock_template_passes() {
    use antares::domain::world::npc_runtime::MerchantStockTemplate;
    use antares::domain::world::NpcDefinition;

    let mut app = CampaignBuilderApp::default();

    let tmpl = MerchantStockTemplate {
        id: "basic_stock".to_string(),
        entries: vec![],
        magic_item_pool: vec![],
        magic_slot_count: 0,
        magic_refresh_days: 7,
    };
    app.campaign_data.stock_templates = vec![tmpl];

    let mut npc = NpcDefinition::new("merchant_ok", "Good Merchant", "ok.png");
    npc.is_merchant = true;
    npc.stock_template = Some("basic_stock".to_string());
    app.editor_registry.npc_editor_state.npcs.push(npc);

    let errors = app.validate_npc_ids();
    // No stock-template error should be present for this NPC
    assert!(
        !errors
            .iter()
            .any(|e| e.message.contains("unknown stock template")),
        "unexpected stock-template error for valid reference: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_campaign_warns_unknown_item_in_template() {
    use antares::domain::world::npc_runtime::{MerchantStockTemplate, TemplateStockEntry};

    let mut app = CampaignBuilderApp::default();

    // Template references item_id 255 which does not exist in items list
    let tmpl = MerchantStockTemplate {
        id: "bad_items_tmpl".to_string(),
        entries: vec![TemplateStockEntry {
            item_id: 255,
            quantity: 1,
            override_price: None,
        }],
        magic_item_pool: vec![],
        magic_slot_count: 0,
        magic_refresh_days: 7,
    };
    app.campaign_data.stock_templates = vec![tmpl.clone()];
    app.editor_registry.stock_templates_editor_state.templates = vec![tmpl];
    // items is empty — item_id 255 is unknown
    app.campaign_data.items = vec![];

    let warnings = app.validate_stock_template_refs();
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("unknown item_id 255")),
        "expected unknown-item warning, got: {:?}",
        warnings.iter().map(|w| &w.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_campaign_warns_unknown_item_in_magic_pool() {
    use antares::domain::world::npc_runtime::MerchantStockTemplate;

    let mut app = CampaignBuilderApp::default();

    // Template has a magic pool entry for item_id 254 which doesn't exist
    let tmpl = MerchantStockTemplate {
        id: "bad_pool_tmpl".to_string(),
        entries: vec![],
        magic_item_pool: vec![254],
        magic_slot_count: 1,
        magic_refresh_days: 7,
    };
    app.campaign_data.stock_templates = vec![tmpl.clone()];
    app.editor_registry.stock_templates_editor_state.templates = vec![tmpl];
    app.campaign_data.items = vec![];

    let warnings = app.validate_stock_template_refs();
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("magic pool") && w.message.contains("254")),
        "expected magic-pool unknown-item warning, got: {:?}",
        warnings.iter().map(|w| &w.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_editor_tab_stock_templates_name() {
    assert_eq!(EditorTab::StockTemplates.name(), "Stock Templates");
}

#[test]
fn test_campaign_metadata_default_stock_templates_file() {
    let meta = CampaignMetadata::default();
    assert_eq!(
        meta.stock_templates_file, "data/npc_stock_templates.ron",
        "default stock_templates_file should be 'data/npc_stock_templates.ron'"
    );
}

#[test]
fn test_next_available_creature_id_returns_error_when_monster_range_is_full() {
    let mut app = CampaignBuilderApp::default();
    let template_id = app
        .creature_template_registry
        .all_templates()
        .first()
        .expect("template registry should not be empty")
        .metadata
        .id
        .clone();

    let mut full_registry = Vec::new();
    for id in 1..=999 {
        full_registry.push(
            app.creature_template_registry
                .generate(&template_id, &format!("Monster {id}"), id)
                .expect("template generation should succeed"),
        );
    }
    app.campaign_data.creatures = full_registry;

    let error = app
        .next_available_creature_id_for_category(creature_id_manager::CreatureCategory::Monsters)
        .expect_err("ID suggestion should fail when monster range is fully allocated");

    assert!(
        error.contains("No available IDs in Monsters range (1-999)"),
        "unexpected error message: {error}"
    );
}
