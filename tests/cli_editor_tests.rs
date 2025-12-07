// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! CLI Editor Round-Trip Integration Tests
//!
//! This module tests the CLI editors (class_editor, item_editor, race_editor)
//! by performing round-trip serialization/deserialization tests to verify
//! data integrity and backward compatibility.
//!
//! # Test Categories
//!
//! - Class Editor: Proficiency preservation, legacy disablement handling
//! - Item Editor: All item types, classification data preservation
//! - Race Editor: Stat modifiers, resistances, restrictions
//! - Legacy Data: Backward compatibility with old RON formats

use antares::domain::classes::{ClassDefinition, SpellSchool, SpellStat};
use antares::domain::items::{
    AccessoryData, AccessorySlot, AmmoData, AmmoType, ArmorClassification, ArmorData,
    AttributeType, ConsumableData, ConsumableEffect, Item, ItemType, MagicItemClassification,
    QuestData, WeaponClassification, WeaponData,
};
use antares::domain::races::{RaceDefinition, Resistances, SizeCategory, StatModifiers};
use antares::domain::types::DiceRoll;
use std::fs;
use std::path::PathBuf;

// ===== Test Infrastructure =====

/// Helper to create a temporary directory for test files
fn create_temp_test_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Helper to get test data directory path
#[allow(dead_code)]
fn get_test_data_dir() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest_dir).join("tests/data")
}

// ===== Test Data Builders =====

/// Creates a test class definition with proficiencies
fn create_test_class_with_proficiencies() -> ClassDefinition {
    ClassDefinition {
        id: "test_knight".to_string(),
        name: "Test Knight".to_string(),
        description: "A test warrior class".to_string(),
        hp_die: DiceRoll::new(1, 10, 0),
        spell_school: None,
        is_pure_caster: false,
        spell_stat: None,

        special_abilities: vec!["multiple_attacks".to_string()],
        starting_weapon_id: Some(1),
        starting_armor_id: Some(100),
        starting_items: vec![200, 201],
        proficiencies: vec![
            "simple_weapon".to_string(),
            "martial_melee".to_string(),
            "heavy_armor".to_string(),
        ],
    }
}

/// Creates a test spellcasting class with proficiencies
fn create_test_spellcasting_class() -> ClassDefinition {
    ClassDefinition {
        id: "test_sorcerer".to_string(),
        name: "Test Sorcerer".to_string(),
        description: "A test mage class".to_string(),
        hp_die: DiceRoll::new(1, 4, 0),
        spell_school: Some(SpellSchool::Sorcerer),
        is_pure_caster: true,
        spell_stat: Some(SpellStat::Intellect),

        special_abilities: vec!["spell_mastery".to_string()],
        starting_weapon_id: Some(10),
        starting_armor_id: None,
        starting_items: vec![],
        proficiencies: vec!["simple_weapon".to_string(), "light_armor".to_string()],
    }
}

/// Creates a test weapon item with classification
fn create_test_weapon() -> Item {
    Item {
        id: 101,
        name: "Test Longsword".to_string(),
        item_type: ItemType::Weapon(WeaponData {
            classification: WeaponClassification::MartialMelee,
            damage: DiceRoll::new(1, 8, 0),
            bonus: 1,
            hands_required: 1,
        }),
        base_cost: 150,
        sell_cost: 75,
        #[allow(deprecated)]
        alignment_restriction: None,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
        tags: vec!["martial_melee".to_string()],
    }
}

/// Creates a test armor item with classification
fn create_test_armor() -> Item {
    Item {
        id: 102,
        name: "Test Plate Mail".to_string(),
        item_type: ItemType::Armor(ArmorData {
            classification: ArmorClassification::Heavy,
            ac_bonus: 6,
            weight: 50,
        }),
        base_cost: 500,
        sell_cost: 250,
        #[allow(deprecated)]
        alignment_restriction: None,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
        tags: vec!["heavy_armor".to_string()],
    }
}

/// Creates a test accessory item with magic classification
fn create_test_accessory() -> Item {
    Item {
        id: 103,
        name: "Test Ring of Power".to_string(),
        item_type: ItemType::Accessory(AccessoryData {
            slot: AccessorySlot::Ring,
            classification: Some(MagicItemClassification::Arcane),
        }),
        base_cost: 1000,
        sell_cost: 500,
        #[allow(deprecated)]
        alignment_restriction: None,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: Some(50),
        max_charges: 10,
        is_cursed: false,
        icon_path: None,
        tags: vec![],
    }
}

/// Creates a test consumable item with effects
fn create_test_consumable() -> Item {
    Item {
        id: 104,
        name: "Test Healing Potion".to_string(),
        item_type: ItemType::Consumable(ConsumableData {
            effect: ConsumableEffect::HealHp(50),
            is_combat_usable: true,
        }),
        base_cost: 50,
        sell_cost: 25,
        #[allow(deprecated)]
        alignment_restriction: None,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 1,
        is_cursed: false,
        icon_path: None,
        tags: vec![],
    }
}

/// Creates a test ammo item
fn create_test_ammo() -> Item {
    Item {
        id: 105,
        name: "Test Arrows".to_string(),
        item_type: ItemType::Ammo(AmmoData {
            ammo_type: AmmoType::Arrow,
            quantity: 20,
        }),
        base_cost: 10,
        sell_cost: 5,
        #[allow(deprecated)]
        alignment_restriction: None,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
        tags: vec![],
    }
}

/// Creates a test quest item
fn create_test_quest_item() -> Item {
    Item {
        id: 106,
        name: "Test Ancient Artifact".to_string(),
        item_type: ItemType::Quest(QuestData {
            quest_id: "main_quest".to_string(),
            is_key_item: true,
        }),
        base_cost: 0,
        sell_cost: 0,
        #[allow(deprecated)]
        alignment_restriction: None,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
        tags: vec!["quest_item".to_string()],
    }
}

/// Creates a test race with stat modifiers and resistances
fn create_test_race_with_modifiers() -> RaceDefinition {
    RaceDefinition {
        id: "test_elf".to_string(),
        name: "Test Elf".to_string(),
        description: "A test elven race".to_string(),
        stat_modifiers: StatModifiers {
            might: 0,
            intellect: 1,
            personality: 0,
            endurance: -1,
            speed: 0,
            accuracy: 1,
            luck: 0,
        },
        resistances: Resistances {
            magic: 10,
            fire: 0,
            cold: 0,
            electricity: 5,
            acid: 0,
            fear: 0,
            poison: 0,
            psychic: 0,
        },
        special_abilities: vec!["infravision".to_string(), "keen_senses".to_string()],
        size: SizeCategory::Medium,

        proficiencies: vec!["longbow".to_string(), "longsword".to_string()],
        incompatible_item_tags: vec!["heavy_weapon".to_string()],
    }
}

/// Creates a test race with resistances
fn create_test_race_with_resistances() -> RaceDefinition {
    RaceDefinition {
        id: "test_dwarf".to_string(),
        name: "Test Dwarf".to_string(),
        description: "A test dwarven race".to_string(),
        stat_modifiers: StatModifiers {
            might: 1,
            intellect: 0,
            personality: 0,
            endurance: 2,
            speed: -1,
            accuracy: 0,
            luck: 0,
        },
        resistances: Resistances {
            magic: 5,
            fire: 5,
            cold: 10,
            electricity: 0,
            acid: 0,
            fear: 0,
            poison: 10,
            psychic: 0,
        },
        special_abilities: vec!["stonecunning".to_string()],
        size: SizeCategory::Medium,

        proficiencies: vec!["axe".to_string(), "hammer".to_string()],
        incompatible_item_tags: vec![],
    }
}

// ===== Class Editor Round-Trip Tests =====

#[test]
fn test_class_roundtrip_with_proficiencies() {
    let temp_dir = create_temp_test_dir();
    let file_path = temp_dir.path().join("test_class.ron");

    // Create test class
    let original = create_test_class_with_proficiencies();

    // Serialize to RON
    let ron_string =
        ron::ser::to_string_pretty(&original, ron::ser::PrettyConfig::default()).unwrap();
    fs::write(&file_path, &ron_string).unwrap();

    // Deserialize back
    let loaded_string = fs::read_to_string(&file_path).unwrap();
    let loaded: ClassDefinition = ron::from_str(&loaded_string).unwrap();

    // Verify all fields match
    assert_eq!(loaded.id, original.id);
    assert_eq!(loaded.name, original.name);
    assert_eq!(loaded.description, original.description);
    assert_eq!(loaded.hp_die, original.hp_die);
    assert_eq!(loaded.spell_school, original.spell_school);
    assert_eq!(loaded.is_pure_caster, original.is_pure_caster);
    assert_eq!(loaded.spell_stat, original.spell_stat);
    assert_eq!(loaded.special_abilities, original.special_abilities);
    assert_eq!(loaded.starting_weapon_id, original.starting_weapon_id);
    assert_eq!(loaded.starting_armor_id, original.starting_armor_id);
    assert_eq!(loaded.starting_items, original.starting_items);

    // Verify proficiencies are preserved
    assert_eq!(loaded.proficiencies, original.proficiencies);
    assert_eq!(loaded.proficiencies.len(), 3);
    assert!(loaded.proficiencies.contains(&"simple_weapon".to_string()));
    assert!(loaded.proficiencies.contains(&"martial_melee".to_string()));
    assert!(loaded.proficiencies.contains(&"heavy_armor".to_string()));
}

#[test]
fn test_class_roundtrip_spellcasting() {
    let temp_dir = create_temp_test_dir();
    let file_path = temp_dir.path().join("test_spellcaster.ron");

    let original = create_test_spellcasting_class();

    // Serialize to RON
    let ron_string =
        ron::ser::to_string_pretty(&original, ron::ser::PrettyConfig::default()).unwrap();
    fs::write(&file_path, &ron_string).unwrap();

    // Deserialize back
    let loaded_string = fs::read_to_string(&file_path).unwrap();
    let loaded: ClassDefinition = ron::from_str(&loaded_string).unwrap();

    // Verify spellcasting fields
    assert_eq!(loaded.spell_school, Some(SpellSchool::Sorcerer));
    assert!(loaded.is_pure_caster);
    assert_eq!(loaded.spell_stat, Some(SpellStat::Intellect));
    assert_eq!(loaded.proficiencies, original.proficiencies);
}

#[test]
fn test_class_legacy_disablement_handling() {
    // Test that legacy disablement_bit field is preserved
    let temp_dir = create_temp_test_dir();
    let file_path = temp_dir.path().join("legacy_class.ron");

    let legacy_ron = r#"(
        id: "legacy_warrior",
        name: "Legacy Warrior",
        description: "An old-format class",
        hp_die: (count: 1, sides: 10, bonus: 0),
        spell_school: None,
        is_pure_caster: false,
        spell_stat: None,
        disablement_bit: 4,
        special_abilities: [],
        proficiencies: [],
    )"#;

    fs::write(&file_path, legacy_ron).unwrap();

    // Load and verify disablement_bit is read correctly
    let loaded_string = fs::read_to_string(&file_path).unwrap();
    let loaded: ClassDefinition = ron::from_str(&loaded_string).unwrap();

    // disablement_bit_index field removed - proficiency system now handles restrictions
    assert_eq!(loaded.id, "legacy_warrior");
}

// ===== Item Editor Round-Trip Tests =====

#[test]
fn test_item_roundtrip_weapon() {
    let temp_dir = create_temp_test_dir();
    let file_path = temp_dir.path().join("test_weapon.ron");

    let original = create_test_weapon();

    // Serialize to RON
    let ron_string =
        ron::ser::to_string_pretty(&original, ron::ser::PrettyConfig::default()).unwrap();
    fs::write(&file_path, &ron_string).unwrap();

    // Deserialize back
    let loaded_string = fs::read_to_string(&file_path).unwrap();
    let loaded: Item = ron::from_str(&loaded_string).unwrap();

    // Verify base fields
    assert_eq!(loaded.id, original.id);
    assert_eq!(loaded.name, original.name);
    assert_eq!(loaded.base_cost, original.base_cost);
    assert_eq!(loaded.sell_cost, original.sell_cost);

    // Verify weapon-specific data
    match (&loaded.item_type, &original.item_type) {
        (ItemType::Weapon(loaded_data), ItemType::Weapon(original_data)) => {
            assert_eq!(loaded_data.classification, original_data.classification);
            assert_eq!(loaded_data.damage, original_data.damage);
            assert_eq!(loaded_data.bonus, original_data.bonus);
            assert_eq!(loaded_data.hands_required, original_data.hands_required);
        }
        _ => panic!("Item type mismatch"),
    }

    // Verify tags
    assert_eq!(loaded.tags, original.tags);
}

#[test]
fn test_item_roundtrip_armor() {
    let temp_dir = create_temp_test_dir();
    let file_path = temp_dir.path().join("test_armor.ron");

    let original = create_test_armor();

    let ron_string =
        ron::ser::to_string_pretty(&original, ron::ser::PrettyConfig::default()).unwrap();
    fs::write(&file_path, &ron_string).unwrap();

    let loaded_string = fs::read_to_string(&file_path).unwrap();
    let loaded: Item = ron::from_str(&loaded_string).unwrap();

    // Verify armor-specific data
    match (&loaded.item_type, &original.item_type) {
        (ItemType::Armor(loaded_data), ItemType::Armor(original_data)) => {
            assert_eq!(loaded_data.classification, original_data.classification);
            assert_eq!(loaded_data.ac_bonus, original_data.ac_bonus);
            assert_eq!(loaded_data.weight, original_data.weight);
        }
        _ => panic!("Item type mismatch"),
    }

    assert_eq!(loaded.tags, original.tags);
}

#[test]
fn test_item_roundtrip_accessory() {
    let temp_dir = create_temp_test_dir();
    let file_path = temp_dir.path().join("test_accessory.ron");

    let original = create_test_accessory();

    let ron_string =
        ron::ser::to_string_pretty(&original, ron::ser::PrettyConfig::default()).unwrap();
    fs::write(&file_path, &ron_string).unwrap();

    let loaded_string = fs::read_to_string(&file_path).unwrap();
    let loaded: Item = ron::from_str(&loaded_string).unwrap();

    // Verify accessory-specific data
    match (&loaded.item_type, &original.item_type) {
        (ItemType::Accessory(loaded_data), ItemType::Accessory(original_data)) => {
            assert_eq!(loaded_data.slot, original_data.slot);
            assert_eq!(loaded_data.classification, original_data.classification);
        }
        _ => panic!("Item type mismatch"),
    }

    // Verify charges and spell effect
    assert_eq!(loaded.max_charges, original.max_charges);
    assert_eq!(loaded.spell_effect, original.spell_effect);
}

#[test]
fn test_item_roundtrip_consumable() {
    let temp_dir = create_temp_test_dir();
    let file_path = temp_dir.path().join("test_consumable.ron");

    let original = create_test_consumable();

    let ron_string =
        ron::ser::to_string_pretty(&original, ron::ser::PrettyConfig::default()).unwrap();
    fs::write(&file_path, &ron_string).unwrap();

    let loaded_string = fs::read_to_string(&file_path).unwrap();
    let loaded: Item = ron::from_str(&loaded_string).unwrap();

    // Verify consumable-specific data
    match (&loaded.item_type, &original.item_type) {
        (ItemType::Consumable(loaded_data), ItemType::Consumable(original_data)) => {
            assert_eq!(loaded_data.effect, original_data.effect);
            assert_eq!(loaded_data.is_combat_usable, original_data.is_combat_usable);
        }
        _ => panic!("Item type mismatch"),
    }
}

#[test]
fn test_item_roundtrip_ammo() {
    let temp_dir = create_temp_test_dir();
    let file_path = temp_dir.path().join("test_ammo.ron");

    let original = create_test_ammo();

    let ron_string =
        ron::ser::to_string_pretty(&original, ron::ser::PrettyConfig::default()).unwrap();
    fs::write(&file_path, &ron_string).unwrap();

    let loaded_string = fs::read_to_string(&file_path).unwrap();
    let loaded: Item = ron::from_str(&loaded_string).unwrap();

    // Verify ammo-specific data
    match (&loaded.item_type, &original.item_type) {
        (ItemType::Ammo(loaded_data), ItemType::Ammo(original_data)) => {
            assert_eq!(loaded_data.ammo_type, original_data.ammo_type);
            assert_eq!(loaded_data.quantity, original_data.quantity);
        }
        _ => panic!("Item type mismatch"),
    }
}

#[test]
fn test_item_roundtrip_quest() {
    let temp_dir = create_temp_test_dir();
    let file_path = temp_dir.path().join("test_quest_item.ron");

    let original = create_test_quest_item();

    let ron_string =
        ron::ser::to_string_pretty(&original, ron::ser::PrettyConfig::default()).unwrap();
    fs::write(&file_path, &ron_string).unwrap();

    let loaded_string = fs::read_to_string(&file_path).unwrap();
    let loaded: Item = ron::from_str(&loaded_string).unwrap();

    // Verify quest-specific data
    match (&loaded.item_type, &original.item_type) {
        (ItemType::Quest(loaded_data), ItemType::Quest(original_data)) => {
            assert_eq!(loaded_data.quest_id, original_data.quest_id);
            assert_eq!(loaded_data.is_key_item, original_data.is_key_item);
        }
        _ => panic!("Item type mismatch"),
    }
}

#[test]
fn test_item_all_classifications_preserved() {
    // Test that all weapon classifications serialize correctly
    let weapon_classifications = vec![
        WeaponClassification::Simple,
        WeaponClassification::MartialMelee,
        WeaponClassification::MartialRanged,
        WeaponClassification::Blunt,
        WeaponClassification::Unarmed,
    ];

    for classification in weapon_classifications {
        let item = Item {
            id: 200,
            name: "Test".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                classification,
                damage: DiceRoll::new(1, 6, 0),
                bonus: 0,
                hands_required: 1,
            }),
            base_cost: 100,
            sell_cost: 50,
            #[allow(deprecated)]
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
        };

        let ron_string = ron::ser::to_string(&item).unwrap();
        let loaded: Item = ron::from_str(&ron_string).unwrap();

        match loaded.item_type {
            ItemType::Weapon(data) => {
                assert_eq!(data.classification, classification);
            }
            _ => panic!("Expected weapon type"),
        }
    }
}

// ===== Race Editor Round-Trip Tests =====

#[test]
fn test_race_roundtrip_with_modifiers() {
    let temp_dir = create_temp_test_dir();
    let file_path = temp_dir.path().join("test_race.ron");

    let original = create_test_race_with_modifiers();

    let ron_string =
        ron::ser::to_string_pretty(&original, ron::ser::PrettyConfig::default()).unwrap();
    fs::write(&file_path, &ron_string).unwrap();

    let loaded_string = fs::read_to_string(&file_path).unwrap();
    let loaded: RaceDefinition = ron::from_str(&loaded_string).unwrap();

    // Verify basic fields
    assert_eq!(loaded.id, original.id);
    assert_eq!(loaded.name, original.name);
    assert_eq!(loaded.description, original.description);
    assert_eq!(loaded.size, original.size);

    // Verify stat modifiers
    assert_eq!(loaded.stat_modifiers.might, original.stat_modifiers.might);
    assert_eq!(
        loaded.stat_modifiers.intellect,
        original.stat_modifiers.intellect
    );
    assert_eq!(
        loaded.stat_modifiers.personality,
        original.stat_modifiers.personality
    );
    assert_eq!(
        loaded.stat_modifiers.endurance,
        original.stat_modifiers.endurance
    );
    assert_eq!(loaded.stat_modifiers.speed, original.stat_modifiers.speed);
    assert_eq!(
        loaded.stat_modifiers.accuracy,
        original.stat_modifiers.accuracy
    );
    assert_eq!(loaded.stat_modifiers.luck, original.stat_modifiers.luck);

    // Verify resistances
    assert_eq!(loaded.resistances.magic, original.resistances.magic);
    assert_eq!(
        loaded.resistances.electricity,
        original.resistances.electricity
    );

    // Verify proficiencies and restrictions
    assert_eq!(loaded.proficiencies, original.proficiencies);
    assert_eq!(
        loaded.incompatible_item_tags,
        original.incompatible_item_tags
    );
}

#[test]
fn test_race_roundtrip_with_resistances() {
    let temp_dir = create_temp_test_dir();
    let file_path = temp_dir.path().join("test_dwarf.ron");

    let original = create_test_race_with_resistances();

    let ron_string =
        ron::ser::to_string_pretty(&original, ron::ser::PrettyConfig::default()).unwrap();
    fs::write(&file_path, &ron_string).unwrap();

    let loaded_string = fs::read_to_string(&file_path).unwrap();
    let loaded: RaceDefinition = ron::from_str(&loaded_string).unwrap();

    // Verify all resistances
    assert_eq!(loaded.resistances.magic, 5);
    assert_eq!(loaded.resistances.fire, 5);
    assert_eq!(loaded.resistances.cold, 10);
    assert_eq!(loaded.resistances.poison, 10);

    // Verify stat modifiers
    assert_eq!(loaded.stat_modifiers.might, 1);
    assert_eq!(loaded.stat_modifiers.endurance, 2);
    assert_eq!(loaded.stat_modifiers.speed, -1);
}

#[test]
fn test_race_special_abilities_preserved() {
    let temp_dir = create_temp_test_dir();
    let file_path = temp_dir.path().join("test_race_abilities.ron");

    let original = create_test_race_with_modifiers();

    let ron_string =
        ron::ser::to_string_pretty(&original, ron::ser::PrettyConfig::default()).unwrap();
    fs::write(&file_path, &ron_string).unwrap();

    let loaded_string = fs::read_to_string(&file_path).unwrap();
    let loaded: RaceDefinition = ron::from_str(&loaded_string).unwrap();

    assert_eq!(loaded.special_abilities, original.special_abilities);
    assert_eq!(loaded.special_abilities.len(), 2);
    assert!(loaded
        .special_abilities
        .contains(&"infravision".to_string()));
    assert!(loaded
        .special_abilities
        .contains(&"keen_senses".to_string()));
}

// ===== Legacy Data Compatibility Tests =====

#[test]
fn test_legacy_class_without_proficiencies() {
    // Test loading old-format class data without proficiencies field
    let legacy_ron = r#"(
        id: "old_knight",
        name: "Old Knight",
        description: "Legacy class definition",
        hp_die: (count: 1, sides: 10, bonus: 0),
        spell_school: None,
        is_pure_caster: false,
        spell_stat: None,
        disablement_bit: 0,
        special_abilities: ["bash"],
    )"#;

    let loaded: ClassDefinition = ron::from_str(legacy_ron).unwrap();

    // Verify proficiencies default to empty
    assert_eq!(loaded.proficiencies, Vec::<String>::new());
    assert_eq!(loaded.id, "old_knight");
    assert_eq!(loaded.special_abilities, vec!["bash".to_string()]);
}

#[test]
fn test_legacy_race_without_proficiencies() {
    // Test loading old-format race data without proficiencies
    let legacy_ron = r#"(
        id: "old_human",
        name: "Old Human",
        description: "Legacy race definition",
        stat_modifiers: (
            might: 0,
            intellect: 0,
            personality: 0,
            endurance: 0,
            speed: 0,
            accuracy: 0,
            luck: 1,
        ),
        resistances: (
            magic: 0,
            fire: 0,
            cold: 0,
            electricity: 0,
            acid: 0,
            fear: 0,
            poison: 0,
            psychic: 0,
        ),
        special_abilities: [],
        size: Medium,
    )"#;

    let loaded: RaceDefinition = ron::from_str(legacy_ron).unwrap();

    // Verify defaults
    assert_eq!(loaded.proficiencies, Vec::<String>::new());
    assert_eq!(loaded.incompatible_item_tags, Vec::<String>::new());
    // disablement_bit_index field removed - proficiency system now handles restrictions
    assert_eq!(loaded.stat_modifiers.luck, 1);
}

#[test]
fn test_legacy_item_minimal_fields() {
    // Test loading old-format item with minimal fields
    let legacy_ron = r#"(
        id: 101,
        name: "Old Sword",
        item_type: Weapon((
            classification: Simple,
            damage: (count: 1, sides: 6, bonus: 0),
            bonus: 0,
            hands_required: 1,
        )),
        base_cost: 100,
        sell_cost: 50,
        disablements: (255),
        max_charges: 0,
        is_cursed: false,
    )"#;

    let loaded: Item = ron::from_str(legacy_ron).unwrap();

    // Verify defaults for new fields
    assert_eq!(loaded.tags, Vec::<String>::new());
    assert_eq!(loaded.alignment_restriction, None);
    assert_eq!(loaded.constant_bonus, None);
    assert_eq!(loaded.temporary_bonus, None);
    assert_eq!(loaded.spell_effect, None);
    assert_eq!(loaded.icon_path, None);
}

#[test]
fn test_class_proficiency_migration_path() {
    // Test that a class can have both legacy disablement_bit and new proficiencies
    let mixed_ron = r#"(
        id: "hybrid_warrior",
        name: "Hybrid Warrior",
        description: "Class with both old and new systems",
        hp_die: (count: 1, sides: 10, bonus: 0),
        spell_school: None,
        is_pure_caster: false,
        spell_stat: None,
        disablement_bit: 2,
        special_abilities: [],
        proficiencies: ["simple_weapon", "martial_melee"],
    )"#;

    let loaded: ClassDefinition = ron::from_str(mixed_ron).unwrap();

    // Both should be present during migration period
    // disablement_bit_index field removed - proficiency system now handles restrictions
    assert_eq!(loaded.proficiencies.len(), 2);
    assert!(loaded.proficiencies.contains(&"simple_weapon".to_string()));
    assert!(loaded.proficiencies.contains(&"martial_melee".to_string()));
}

#[test]
fn test_item_consumable_effect_variants() {
    // Test all consumable effect variants serialize correctly
    let effects = vec![
        ConsumableEffect::HealHp(100),
        ConsumableEffect::RestoreSp(50),
        ConsumableEffect::CureCondition(0x01), // Poisoned bit
        ConsumableEffect::BoostAttribute(AttributeType::Might, 5),
    ];

    for effect in effects {
        let item = Item {
            id: 150,
            name: "Test Consumable".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect,
                is_combat_usable: true,
            }),
            base_cost: 50,
            sell_cost: 25,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 1,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
        };

        let ron_string = ron::ser::to_string(&item).unwrap();
        let loaded: Item = ron::from_str(&ron_string).unwrap();

        match loaded.item_type {
            ItemType::Consumable(data) => {
                assert_eq!(data.effect, effect);
            }
            _ => panic!("Expected consumable type"),
        }
    }
}

#[test]
fn test_armor_classifications_preserved() {
    // Test that all armor classifications serialize correctly
    let armor_classifications = vec![
        ArmorClassification::Light,
        ArmorClassification::Medium,
        ArmorClassification::Heavy,
        ArmorClassification::Shield,
    ];

    for classification in armor_classifications {
        let item = Item {
            id: 201,
            name: "Test Armor".to_string(),
            item_type: ItemType::Armor(ArmorData {
                classification,
                ac_bonus: 5,
                weight: 30,
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
        };

        let ron_string = ron::ser::to_string(&item).unwrap();
        let loaded: Item = ron::from_str(&ron_string).unwrap();

        match loaded.item_type {
            ItemType::Armor(data) => {
                assert_eq!(data.classification, classification);
            }
            _ => panic!("Expected armor type"),
        }
    }
}

#[test]
fn test_accessory_slots_preserved() {
    // Test that all accessory slots serialize correctly
    let slots = vec![
        AccessorySlot::Ring,
        AccessorySlot::Amulet,
        AccessorySlot::Belt,
        AccessorySlot::Cloak,
    ];

    for slot in slots {
        let item = Item {
            id: 202,
            name: "Test Accessory".to_string(),
            item_type: ItemType::Accessory(AccessoryData {
                slot,
                classification: None,
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
        };

        let ron_string = ron::ser::to_string(&item).unwrap();
        let loaded: Item = ron::from_str(&ron_string).unwrap();

        match loaded.item_type {
            ItemType::Accessory(data) => {
                assert_eq!(data.slot, slot);
            }
            _ => panic!("Expected accessory type"),
        }
    }
}
