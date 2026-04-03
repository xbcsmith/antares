// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! RON serialization round-trip tests for all major game data types.

use super::*;
use antares::domain::character::{AttributePair, AttributePair16, Stats};
use antares::domain::combat::monster::{LootTable, MonsterCondition, MonsterResistances};
use antares::domain::combat::types::{Attack, AttackType};
use antares::domain::combat::SpecialEffect;
use antares::domain::magic::types::{SpellContext, SpellSchool, SpellTarget};
use antares::domain::types::DiceRoll;

#[test]
fn test_ron_serialization() {
    let campaign = CampaignMetadata {
        id: "test_campaign".to_string(),
        name: "Test Campaign".to_string(),
        version: "1.0.0".to_string(),
        author: "Test Author".to_string(),
        description: "A test campaign".to_string(),
        engine_version: "0.1.0".to_string(),
        starting_map: "test_map".to_string(),
        starting_position: (5, 5),
        starting_direction: "North".to_string(),
        starting_gold: 200,
        starting_food: 20,
        starting_innkeeper: "tutorial_innkeeper_town".to_string(),
        max_party_size: 6,
        max_roster_size: 20,
        difficulty: Difficulty::Hard,
        permadeath: true,
        allow_multiclassing: true,
        starting_level: 2,
        max_level: 15,
        items_file: "data/items.ron".to_string(),
        spells_file: "data/spells.ron".to_string(),
        monsters_file: "data/monsters.ron".to_string(),
        classes_file: "data/classes.ron".to_string(),
        races_file: "data/races.ron".to_string(),
        characters_file: "data/characters.ron".to_string(),
        creatures_file: "data/creatures.ron".to_string(),
        maps_dir: "data/maps/".to_string(),
        quests_file: "data/quests.ron".to_string(),
        dialogue_file: "data/dialogue.ron".to_string(),
        conditions_file: "data/conditions.ron".to_string(),
        npcs_file: "data/npcs.ron".to_string(),
        proficiencies_file: "data/proficiencies.ron".to_string(),
        stock_templates_file: "data/npc_stock_templates.ron".to_string(),
        furniture_file: "data/furniture.ron".to_string(),
        starting_time: default_starting_time(),
    };

    let ron_config = ron::ser::PrettyConfig::new()
        .struct_names(true)
        .enumerate_arrays(false)
        .depth_limit(4);

    let serialized = ron::ser::to_string_pretty(&campaign, ron_config);
    assert!(serialized.is_ok());

    let ron_string = serialized.unwrap();
    assert!(ron_string.contains("test_campaign"));
    assert!(ron_string.contains("Test Campaign"));

    // Test deserialization
    let deserialized: Result<CampaignMetadata, _> = ron::from_str(&ron_string);
    assert!(deserialized.is_ok());

    let loaded = deserialized.unwrap();
    assert_eq!(loaded.id, campaign.id);
    assert_eq!(loaded.name, campaign.name);
    assert_eq!(loaded.difficulty, campaign.difficulty);
    assert_eq!(loaded.permadeath, campaign.permadeath);
}

#[test]
fn test_campaign_backwards_compatibility_missing_proficiencies_file() {
    // Test that old campaign files without proficiencies_file can still load
    // This ensures older campaigns still work
    let old_campaign_ron = r#"CampaignMetadata(
id: "legacy_campaign",
name: "Legacy Campaign",
version: "1.0.0",
author: "Test",
description: "Test",
engine_version: "0.1.0",
starting_map: "test_map",
starting_position: (5, 5),
starting_direction: "North",
starting_gold: 100,
starting_food: 10,
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
characters_file: "data/characters.ron",
maps_dir: "data/maps/",
quests_file: "data/quests.ron",
dialogue_file: "data/dialogues.ron",
conditions_file: "data/conditions.ron",
npcs_file: "data/npcs.ron",
)"#;

    // Deserialize should succeed and use default proficiencies_file
    let result: Result<CampaignMetadata, _> = ron::from_str(old_campaign_ron);
    assert!(
        result.is_ok(),
        "Failed to deserialize legacy campaign: {:?}",
        result.err()
    );

    let campaign = result.unwrap();
    assert_eq!(campaign.id, "legacy_campaign");
    assert_eq!(campaign.proficiencies_file, "data/proficiencies.ron");
}

#[test]
fn test_conditions_save_load_roundtrip() {
    let mut app = CampaignBuilderApp::default();

    // Create a unique temporary directory under the system temp dir
    let tmp_dir = std::env::temp_dir().join(format!(
        "antares_test_conditions_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp_dir).unwrap();

    app.campaign_dir = Some(tmp_dir);
    app.campaign.conditions_file = "conditions_test.ron".to_string();

    let c1 = ConditionDefinition {
        id: "c1".to_string(),
        name: "Condition 1".to_string(),
        description: "".to_string(),
        effects: vec![],
        default_duration: antares::domain::conditions::ConditionDuration::Rounds(2),
        icon_id: None,
    };
    let c2 = ConditionDefinition {
        id: "c2".to_string(),
        name: "Condition 2".to_string(),
        description: "".to_string(),
        effects: vec![],
        default_duration: antares::domain::conditions::ConditionDuration::Rounds(4),
        icon_id: None,
    };

    app.campaign_data.conditions.push(c1);
    app.campaign_data.conditions.push(c2);

    assert!(app.save_conditions().is_ok());

    app.campaign_data.conditions.clear();
    app.load_conditions();

    assert_eq!(app.campaign_data.conditions.len(), 2);
    assert_eq!(app.campaign_data.conditions[0].id, "c1");
    assert_eq!(app.campaign_data.conditions[1].id, "c2");
}

#[test]
fn test_item_import_export_roundtrip() {
    use antares::domain::items::types::*;

    let original_item = Item {
        id: 42,
        name: "Test Sword".to_string(),
        item_type: ItemType::Weapon(WeaponData {
            damage: DiceRoll::new(2, 6, 1),
            bonus: 2,
            hands_required: 1,
            classification: WeaponClassification::MartialMelee,
        }),
        base_cost: 100,
        sell_cost: 50,
        alignment_restriction: None,
        constant_bonus: Some(Bonus {
            attribute: BonusAttribute::Might,
            value: 3,
        }),
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
        tags: vec![],
        mesh_descriptor_override: None,
        mesh_id: None,
    };

    // Export to RON
    let ron_string = ron::ser::to_string_pretty(&original_item, ron::ser::PrettyConfig::default())
        .expect("Failed to serialize item");

    // Import from RON
    let imported_item: Item = ron::from_str(&ron_string).expect("Failed to deserialize item");

    // Verify roundtrip
    assert_eq!(original_item.id, imported_item.id);
    assert_eq!(original_item.name, imported_item.name);
    assert_eq!(original_item.base_cost, imported_item.base_cost);
    assert_eq!(original_item.is_cursed, imported_item.is_cursed);

    // Verify weapon data
    if let (ItemType::Weapon(orig_data), ItemType::Weapon(import_data)) =
        (&original_item.item_type, &imported_item.item_type)
    {
        assert_eq!(orig_data.damage, import_data.damage);
        assert_eq!(orig_data.bonus, import_data.bonus);
        assert_eq!(orig_data.hands_required, import_data.hands_required);
    } else {
        panic!("Item type mismatch after roundtrip");
    }
}

#[test]
fn test_spell_import_export_roundtrip() {
    let original = Spell::new(
        42,
        "Test Spell",
        SpellSchool::Sorcerer,
        5,
        10,
        2,
        SpellContext::CombatOnly,
        SpellTarget::AllMonsters,
        "Test description",
        None,
        0,
        false,
    );

    // Export to RON
    let ron_data = ron::to_string(&original).unwrap();

    // Import from RON
    let imported: Spell = ron::from_str(&ron_data).unwrap();

    assert_eq!(imported.id, original.id);
    assert_eq!(imported.name, original.name);
    assert_eq!(imported.school, original.school);
    assert_eq!(imported.level, original.level);
    assert_eq!(imported.sp_cost, original.sp_cost);
    assert_eq!(imported.gem_cost, original.gem_cost);
    assert_eq!(imported.context, original.context);
    assert_eq!(imported.target, original.target);
}

#[test]
fn test_monster_import_export_roundtrip() {
    let original = MonsterDefinition {
        id: 42,
        name: "Test Monster".to_string(),
        stats: Stats::new(15, 12, 10, 14, 13, 11, 8),
        hp: AttributePair16::new(30),
        ac: AttributePair::new(8),
        attacks: vec![Attack {
            damage: DiceRoll::new(2, 6, 2),
            attack_type: AttackType::Fire,
            special_effect: Some(SpecialEffect::Paralysis),
            is_ranged: false,
        }],
        flee_threshold: 10,
        special_attack_threshold: 25,
        resistances: MonsterResistances::new(),
        can_regenerate: false,
        can_advance: true,
        is_undead: false,
        magic_resistance: 10,
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

    // Export to RON
    let ron_data = ron::to_string(&original).unwrap();

    // Import from RON
    let imported: MonsterDefinition = ron::from_str(&ron_data).unwrap();

    assert_eq!(imported.id, original.id);
    assert_eq!(imported.name, original.name);
    assert_eq!(imported.hp, original.hp);
    assert_eq!(imported.ac, original.ac);
    assert_eq!(imported.attacks.len(), original.attacks.len());
    assert_eq!(imported.can_regenerate, original.can_regenerate);
    assert_eq!(imported.can_advance, original.can_advance);
    assert_eq!(imported.is_undead, original.is_undead);
    assert_eq!(imported.magic_resistance, original.magic_resistance);
    assert_eq!(imported.loot.experience, original.loot.experience);
}

#[test]
fn test_quest_import_export_roundtrip() {
    use antares::domain::quest::QuestReward;

    let original = Quest::new(42, "Export Quest", "Quest for export testing");
    let mut quest = original.clone();
    quest.min_level = Some(10);
    quest.repeatable = true;
    quest.is_main_quest = true;
    quest.add_reward(QuestReward::Experience(2000));
    quest.add_reward(QuestReward::Gold(1000));

    // Export to RON
    let exported = ron::ser::to_string_pretty(&quest, Default::default());
    assert!(exported.is_ok());

    let ron_string = exported.unwrap();
    assert!(ron_string.contains("Export Quest"));
    assert!(ron_string.contains("repeatable: true"));

    // Import from RON
    let imported: Result<Quest, _> = ron::from_str(&ron_string);
    assert!(imported.is_ok());

    let quest_imported = imported.unwrap();
    assert_eq!(quest_imported.id, quest.id);
    assert_eq!(quest_imported.name, quest.name);
    assert_eq!(quest_imported.repeatable, quest.repeatable);
    assert_eq!(quest_imported.is_main_quest, quest.is_main_quest);
    assert_eq!(quest_imported.min_level, quest.min_level);
    assert_eq!(quest_imported.rewards.len(), 2);
}

#[test]
fn test_dialogue_import_export_roundtrip() {
    let mut app = CampaignBuilderApp::default();

    let mut dialogue = DialogueTree::new(42, "Test Export", 1);
    dialogue.speaker_name = Some("Merchant".to_string());
    dialogue.repeatable = false;

    let root_node = antares::domain::dialogue::DialogueNode::new(1, "Hello!");
    dialogue.add_node(root_node);

    app.campaign_data.dialogues.push(dialogue);

    // Export to RON
    let ron_str =
        ron::ser::to_string_pretty(&app.campaign_data.dialogues, Default::default()).unwrap();

    // Import back
    let imported: Vec<DialogueTree> = ron::from_str(&ron_str).unwrap();

    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].id, 42);
    assert_eq!(imported[0].name, "Test Export");
    assert_eq!(imported[0].speaker_name, Some("Merchant".to_string()));
    assert!(!imported[0].repeatable);
    assert_eq!(imported[0].nodes.len(), 1);
}
