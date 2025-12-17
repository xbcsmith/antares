// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Equipment validation - Enforce class/race equipment restrictions
//!
//! This module provides validation functions to check whether a character
//! can equip items based on class proficiencies, race restrictions, and
//! alignment requirements.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4 for core data structures.
//! See `docs/explanation/engine_sdk_support_plan.md` Phase 6 for implementation details.

use crate::domain::character::{Character, Equipment};
use crate::domain::classes::ClassDatabase;
use crate::domain::items::ItemDatabase;
use crate::domain::proficiency::has_proficiency_union;
use crate::domain::races::RaceDatabase;
use crate::domain::types::ItemId;
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur during equipment validation
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum EquipError {
    #[error("Item not found in database: {0}")]
    ItemNotFound(ItemId),

    #[error("Character class cannot use this item (missing required proficiency)")]
    ClassRestriction,

    #[error("Character race cannot use this item (incompatible tag)")]
    RaceRestriction,

    #[error("No equipment slot available for this item type")]
    NoSlotAvailable,

    #[error("Character's alignment cannot use this item")]
    AlignmentRestriction,

    #[error("Invalid race definition: {0}")]
    InvalidRace(String),

    #[error("Invalid class definition: {0}")]
    InvalidClass(String),
}

// ===== Equipment Validation =====

/// Check if a character can equip an item
///
/// This function validates all restrictions:
/// - Class proficiency requirements
/// - Race incompatibilities (via item tags)
/// - Alignment restrictions
/// - Equipment slot availability
///
/// # Arguments
///
/// * `character` - The character attempting to equip the item
/// * `item_id` - The item ID to check
/// * `items` - Item database for looking up item definitions
/// * `classes` - Class database for proficiency checks
/// * `races` - Race database for race restriction checks
///
/// # Returns
///
/// Returns `Ok(true)` if the character can equip the item
///
/// # Errors
///
/// Returns various `EquipError` variants if validation fails:
/// - `ItemNotFound` - Item ID not in database
/// - `ClassRestriction` - Character lacks required proficiency
/// - `RaceRestriction` - Item has incompatible tag for character's race
/// - `AlignmentRestriction` - Item has alignment restriction character doesn't meet
/// - `NoSlotAvailable` - No appropriate equipment slot for item type
/// - `InvalidRace` - Character's race not found in database
/// - `InvalidClass` - Character's class not found in database
///
/// # Examples
///
/// ```
/// use antares::domain::character::Character;
/// use antares::domain::items::can_equip_item;
/// use antares::domain::classes::ClassDatabase;
/// use antares::domain::races::RaceDatabase;
/// use antares::domain::items::ItemDatabase;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let character = Character::new("Hero", "human".to_string(), "knight".to_string());
/// let items = ItemDatabase::new();
/// let classes = ClassDatabase::new();
/// let races = RaceDatabase::new();
///
/// let item_id = 1;
/// let result = can_equip_item(&character, item_id, &items, &classes, &races);
/// # Ok(())
/// # }
/// ```
pub fn can_equip_item(
    character: &Character,
    item_id: ItemId,
    items: &ItemDatabase,
    classes: &ClassDatabase,
    races: &RaceDatabase,
) -> Result<bool, EquipError> {
    // Look up item definition
    let item = items
        .get_item(item_id)
        .ok_or(EquipError::ItemNotFound(item_id))?;

    // Check alignment restriction
    if !item.can_use_alignment(character.alignment) {
        return Err(EquipError::AlignmentRestriction);
    }

    // Look up class definition
    let class_def = classes
        .get_class(&character.class_id)
        .ok_or_else(|| EquipError::InvalidClass(character.class_id.clone()))?;

    // Look up race definition
    let race_def = races
        .get_race(&character.race_id)
        .ok_or_else(|| EquipError::InvalidRace(character.race_id.clone()))?;

    // Check proficiency requirement (class OR race must have it)
    let required_prof = item.required_proficiency();
    if !has_proficiency_union(
        required_prof.as_ref(),
        &class_def.proficiencies,
        &race_def.proficiencies,
    ) {
        return Err(EquipError::ClassRestriction);
    }

    // Check race incompatibilities (item tags)
    if !race_def.can_use_item(&item.tags) {
        return Err(EquipError::RaceRestriction);
    }

    // Check equipment slot availability
    if !has_slot_for_item(&character.equipment, item_id, item) {
        return Err(EquipError::NoSlotAvailable);
    }

    Ok(true)
}

/// Check if equipment has an available slot for an item
///
/// This is a helper function that determines if the character has an appropriate
/// slot for the item type. Note: This checks if a slot EXISTS for the item type,
/// not necessarily if it's empty (items can be swapped).
///
/// # Arguments
///
/// * `equipment` - The character's current equipment
/// * `item_id` - The item ID to check
/// * `item` - The item definition
///
/// # Returns
///
/// `true` if an appropriate equipment slot exists for this item type
fn has_slot_for_item(
    _equipment: &Equipment,
    _item_id: ItemId,
    item: &crate::domain::items::types::Item,
) -> bool {
    use crate::domain::items::types::{AccessorySlot, ItemType};

    match &item.item_type {
        ItemType::Weapon(_) => {
            // Weapon slot always exists
            true
        }
        ItemType::Armor(_) => {
            // Armor slot always exists
            true
        }
        ItemType::Accessory(data) => {
            // Check if appropriate accessory slot exists
            match data.slot {
                AccessorySlot::Ring => {
                    // Both accessory slots can hold rings
                    true
                }
                AccessorySlot::Amulet | AccessorySlot::Belt | AccessorySlot::Cloak => {
                    // Other accessories use accessory slots
                    true
                }
            }
        }
        ItemType::Consumable(_) | ItemType::Ammo(_) | ItemType::Quest(_) => {
            // These items are not equipped, they go in inventory
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::classes::{ClassDefinition, SpellSchool, SpellStat};
    use crate::domain::items::types::{
        AccessoryData, AccessorySlot, AlignmentRestriction, ArmorClassification, ArmorData, Item,
        ItemType, MagicItemClassification, WeaponClassification, WeaponData,
    };
    use crate::domain::races::{RaceDefinition, SizeCategory};
    use crate::domain::types::DiceRoll;

    // Helper function to create test knight class
    fn create_test_knight_class() -> ClassDefinition {
        ClassDefinition {
            id: "knight".to_string(),
            name: "Knight".to_string(),
            description: "A brave warrior".to_string(),
            hp_die: DiceRoll::new(1, 10, 0),
            spell_school: None,
            is_pure_caster: false,
            spell_stat: None,
            special_abilities: vec!["multiple_attacks".to_string()],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec![
                "simple_weapon".to_string(),
                "martial_melee".to_string(),
                "heavy_armor".to_string(),
            ],
        }
    }

    // Helper function to create test sorcerer class
    fn create_test_sorcerer_class() -> ClassDefinition {
        ClassDefinition {
            id: "sorcerer".to_string(),
            name: "Sorcerer".to_string(),
            description: "A master of arcane magic".to_string(),
            hp_die: DiceRoll::new(1, 4, 0),
            spell_school: Some(SpellSchool::Sorcerer),
            is_pure_caster: true,
            spell_stat: Some(SpellStat::Intellect),
            special_abilities: vec!["spell_mastery".to_string()],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec![
                "simple_weapon".to_string(),
                "light_armor".to_string(),
                "arcane_item".to_string(),
            ],
        }
    }

    // Helper function to create test human race
    fn create_test_human_race() -> RaceDefinition {
        RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Versatile and adaptable".to_string(),
        )
    }

    // Helper function to create test elf race
    fn create_test_elf_race() -> RaceDefinition {
        RaceDefinition {
            id: "elf".to_string(),
            name: "Elf".to_string(),
            description: "Graceful and magical".to_string(),
            stat_modifiers: crate::domain::races::StatModifiers::default(),
            resistances: crate::domain::races::Resistances::default(),
            special_abilities: vec!["infravision".to_string()],
            size: SizeCategory::Medium,
            proficiencies: vec![],
            incompatible_item_tags: vec!["heavy_armor".to_string()],
        }
    }

    // Helper function to create test gnome race
    fn create_test_gnome_race() -> RaceDefinition {
        RaceDefinition {
            id: "gnome".to_string(),
            name: "Gnome".to_string(),
            description: "Small but clever".to_string(),
            stat_modifiers: crate::domain::races::StatModifiers::default(),
            resistances: crate::domain::races::Resistances::default(),
            special_abilities: vec![],
            size: SizeCategory::Small,
            proficiencies: vec![],
            incompatible_item_tags: vec!["large_weapon".to_string(), "heavy_armor".to_string()],
        }
    }

    // Helper function to create a simple sword
    fn create_test_sword() -> Item {
        Item {
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
        }
    }

    // Helper function to create plate armor
    fn create_test_plate_armor() -> Item {
        Item {
            id: 2,
            name: "Plate Mail".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 6,
                weight: 45,
                classification: ArmorClassification::Heavy,
            }),
            base_cost: 400,
            sell_cost: 200,
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

    #[test]
    fn test_knight_can_equip_sword() {
        // Arrange
        let character = Character::new(
            "Sir Lancelot".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        let mut items = ItemDatabase::new();
        let sword = create_test_sword();
        items.add_item(sword).unwrap();

        let mut classes = ClassDatabase::new();
        let knight = create_test_knight_class();
        classes.add_class(knight).unwrap();

        let mut races = RaceDatabase::new();
        let human = create_test_human_race();
        races.add_race(human).unwrap();

        // Act
        let result = can_equip_item(&character, 1, &items, &classes, &races);

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_sorcerer_cannot_equip_plate_armor() {
        // Arrange
        let character = Character::new(
            "Gandalf".to_string(),
            "human".to_string(),
            "sorcerer".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        let mut items = ItemDatabase::new();
        let plate = create_test_plate_armor();
        items.add_item(plate).unwrap();

        let mut classes = ClassDatabase::new();
        let sorcerer = create_test_sorcerer_class();
        classes.add_class(sorcerer).unwrap();

        let mut races = RaceDatabase::new();
        let human = create_test_human_race();
        races.add_race(human).unwrap();

        // Act
        let result = can_equip_item(&character, 2, &items, &classes, &races);

        // Assert
        assert!(result.is_err());
        assert!(matches!(result, Err(EquipError::ClassRestriction)));
    }

    #[test]
    fn test_elf_cannot_equip_heavy_armor() {
        // Arrange
        let character = Character::new(
            "Legolas".to_string(),
            "elf".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        let mut items = ItemDatabase::new();
        let plate = create_test_plate_armor();
        items.add_item(plate).unwrap();

        let mut classes = ClassDatabase::new();
        let knight = create_test_knight_class();
        classes.add_class(knight).unwrap();

        let mut races = RaceDatabase::new();
        let elf = create_test_elf_race();
        races.add_race(elf).unwrap();

        // Act
        let result = can_equip_item(&character, 2, &items, &classes, &races);

        // Assert
        assert!(result.is_err());
        assert!(matches!(result, Err(EquipError::RaceRestriction)));
    }

    #[test]
    fn test_equip_with_full_slots_error() {
        // Arrange - Create a consumable item (cannot be equipped)
        let character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        let mut items = ItemDatabase::new();
        let potion = Item {
            id: 99,
            name: "Health Potion".to_string(),
            item_type: ItemType::Consumable(crate::domain::items::types::ConsumableData {
                effect: crate::domain::items::types::ConsumableEffect::HealHp(10),
                is_combat_usable: true,
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
        items.add_item(potion).unwrap();

        let mut classes = ClassDatabase::new();
        let knight = create_test_knight_class();
        classes.add_class(knight).unwrap();

        let mut races = RaceDatabase::new();
        let human = create_test_human_race();
        races.add_race(human).unwrap();

        // Act - Try to equip a consumable (should fail - no slot available)
        let result = can_equip_item(&character, 99, &items, &classes, &races);

        // Assert
        assert!(result.is_err());
        assert!(matches!(result, Err(EquipError::NoSlotAvailable)));
    }

    #[test]
    fn test_equip_invalid_item_id_error() {
        // Arrange
        let character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        let items = ItemDatabase::new(); // Empty database

        let mut classes = ClassDatabase::new();
        let knight = create_test_knight_class();
        classes.add_class(knight).unwrap();

        let mut races = RaceDatabase::new();
        let human = create_test_human_race();
        races.add_race(human).unwrap();

        // Act
        let result = can_equip_item(&character, 255, &items, &classes, &races);

        // Assert
        assert!(result.is_err());
        assert!(matches!(result, Err(EquipError::ItemNotFound(255))));
    }

    #[test]
    fn test_good_character_cannot_equip_evil_item() {
        // Arrange
        let character = Character {
            name: "Paladin".to_string(),
            race_id: "human".to_string(),
            class_id: "knight".to_string(),
            sex: Sex::Male,
            alignment: Alignment::Good,
            alignment_initial: Alignment::Good,
            level: 1,
            experience: 0,
            age: 20,
            age_days: 0,
            stats: crate::domain::character::Stats::new(10, 10, 10, 10, 10, 10, 10),
            hp: crate::domain::character::AttributePair16::new(10),
            sp: crate::domain::character::AttributePair16::new(0),
            ac: crate::domain::character::AttributePair::new(10),
            spell_level: crate::domain::character::AttributePair::new(0),
            inventory: crate::domain::character::Inventory::new(),
            equipment: crate::domain::character::Equipment::new(),
            spells: crate::domain::character::SpellBook::new(),
            conditions: crate::domain::character::Condition::new(),
            active_conditions: vec![],
            resistances: crate::domain::character::Resistances::default(),
            quest_flags: crate::domain::character::QuestFlags::new(),
            portrait_id: 0,
            worthiness: 0,
            gold: 0,
            gems: 0,
            food: 0,
        };

        let mut items = ItemDatabase::new();
        let evil_sword = Item {
            id: 3,
            name: "Cursed Blade".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 10, 0),
                bonus: 2,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 500,
            sell_cost: 250,
            alignment_restriction: Some(AlignmentRestriction::EvilOnly),
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: true,
            icon_path: None,
            tags: vec![],
        };
        items.add_item(evil_sword).unwrap();

        let mut classes = ClassDatabase::new();
        let knight = create_test_knight_class();
        classes.add_class(knight).unwrap();

        let mut races = RaceDatabase::new();
        let human = create_test_human_race();
        races.add_race(human).unwrap();

        // Act
        let result = can_equip_item(&character, 3, &items, &classes, &races);

        // Assert
        assert!(result.is_err());
        assert!(matches!(result, Err(EquipError::AlignmentRestriction)));
    }

    #[test]
    fn test_gnome_cannot_equip_large_weapon() {
        // Arrange
        let character = Character::new(
            "Gimli".to_string(),
            "gnome".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        let mut items = ItemDatabase::new();
        let large_sword = Item {
            id: 4,
            name: "Greatsword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(2, 6, 0),
                bonus: 0,
                hands_required: 2,
                classification: WeaponClassification::MartialMelee,
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
            tags: vec!["large_weapon".to_string(), "two_handed".to_string()],
        };
        items.add_item(large_sword).unwrap();

        let mut classes = ClassDatabase::new();
        let knight = create_test_knight_class();
        classes.add_class(knight).unwrap();

        let mut races = RaceDatabase::new();
        let gnome = create_test_gnome_race();
        races.add_race(gnome).unwrap();

        // Act
        let result = can_equip_item(&character, 4, &items, &classes, &races);

        // Assert
        assert!(result.is_err());
        assert!(matches!(result, Err(EquipError::RaceRestriction)));
    }

    #[test]
    fn test_accessory_can_be_equipped() {
        // Arrange
        let character = Character::new(
            "Wizard".to_string(),
            "human".to_string(),
            "sorcerer".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        let mut items = ItemDatabase::new();
        let ring = Item {
            id: 5,
            name: "Ring of Protection".to_string(),
            item_type: ItemType::Accessory(AccessoryData {
                slot: AccessorySlot::Ring,
                classification: Some(MagicItemClassification::Arcane),
            }),
            base_cost: 200,
            sell_cost: 100,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
        };
        items.add_item(ring).unwrap();

        let mut classes = ClassDatabase::new();
        let sorcerer = create_test_sorcerer_class();
        classes.add_class(sorcerer).unwrap();

        let mut races = RaceDatabase::new();
        let human = create_test_human_race();
        races.add_race(human).unwrap();

        // Act
        let result = can_equip_item(&character, 5, &items, &classes, &races);

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_invalid_race_returns_error() {
        // Arrange
        let character = Character::new(
            "Hero".to_string(),
            "unknown_race".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        let mut items = ItemDatabase::new();
        let sword = create_test_sword();
        items.add_item(sword).unwrap();

        let mut classes = ClassDatabase::new();
        let knight = create_test_knight_class();
        classes.add_class(knight).unwrap();

        let races = RaceDatabase::new(); // Empty - no races

        // Act
        let result = can_equip_item(&character, 1, &items, &classes, &races);

        // Assert
        assert!(result.is_err());
        assert!(matches!(result, Err(EquipError::InvalidRace(_))));
    }

    #[test]
    fn test_invalid_class_returns_error() {
        // Arrange
        let character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "unknown_class".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        let mut items = ItemDatabase::new();
        let sword = create_test_sword();
        items.add_item(sword).unwrap();

        let classes = ClassDatabase::new(); // Empty - no classes

        let mut races = RaceDatabase::new();
        let human = create_test_human_race();
        races.add_race(human).unwrap();

        // Act
        let result = can_equip_item(&character, 1, &items, &classes, &races);

        // Assert
        assert!(result.is_err());
        assert!(matches!(result, Err(EquipError::InvalidClass(_))));
    }
}
