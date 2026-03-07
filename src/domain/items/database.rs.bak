// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Item database - Loading and managing item definitions from RON files
//!
//! This module provides functionality to load item definitions from RON data files
//! and query them at runtime.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 7.1-7.2 for data file specifications.

use crate::domain::items::types::Item;
use crate::domain::proficiency::ProficiencyDatabase;
use crate::domain::types::ItemId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur when loading item data
#[derive(Error, Debug)]
pub enum ItemDatabaseError {
    #[error("Failed to read item data file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse RON data: {0}")]
    ParseError(#[from] ron::error::SpannedError),

    #[error("Item ID {0} not found in database")]
    ItemNotFound(ItemId),

    #[error("Duplicate item ID {0} detected")]
    DuplicateId(ItemId),

    #[error("Item ID {0} references unknown proficiency: {1}")]
    InvalidProficiency(ItemId, String),
}

// ===== Item Database =====

/// Item database - stores all item definitions
///
/// # Examples
///
/// ```no_run
/// use antares::domain::items::ItemDatabase;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load items from RON file
/// let db = ItemDatabase::load_from_file("data/items.ron")?;
///
/// // Query item by ID
/// if let Some(item) = db.get_item(1) {
///     println!("Found item: {}", item.name);
/// }
///
/// // Get all weapons
/// let weapons = db.get_weapons();
/// println!("Total weapons: {}", weapons.len());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDatabase {
    /// All items indexed by ID
    items: HashMap<ItemId, Item>,
}

impl ItemDatabase {
    /// Create an empty item database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::ItemDatabase;
    ///
    /// let db = ItemDatabase::new();
    /// assert_eq!(db.len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    /// Load item database from a RON file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file containing item definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(ItemDatabase)` on success
    ///
    /// # Errors
    ///
    /// Returns `ItemDatabaseError::ReadError` if file cannot be read
    /// Returns `ItemDatabaseError::ParseError` if RON parsing fails
    /// Returns `ItemDatabaseError::DuplicateId` if duplicate item IDs found
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::items::ItemDatabase;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = ItemDatabase::load_from_file("data/items.ron")?;
    /// println!("Loaded {} items", db.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ItemDatabaseError> {
        let contents = std::fs::read_to_string(path)?;
        Self::load_from_string(&contents)
    }

    /// Load item database from a RON string
    ///
    /// # Arguments
    ///
    /// * `ron_data` - RON-formatted string containing item definitions
    ///
    /// # Returns
    ///
    /// Returns `Ok(ItemDatabase)` on success
    ///
    /// # Errors
    ///
    /// Returns `ItemDatabaseError::ParseError` if RON parsing fails
    /// Returns `ItemDatabaseError::DuplicateId` if duplicate item IDs found
    pub fn load_from_string(ron_data: &str) -> Result<Self, ItemDatabaseError> {
        let items: Vec<Item> = ron::from_str(ron_data)?;
        let mut db = Self::new();

        for item in items {
            if db.items.contains_key(&item.id) {
                return Err(ItemDatabaseError::DuplicateId(item.id));
            }
            db.items.insert(item.id, item);
        }

        Ok(db)
    }

    /// Add an item to the database
    ///
    /// # Arguments
    ///
    /// * `item` - Item to add
    ///
    /// # Errors
    ///
    /// Returns `ItemDatabaseError::DuplicateId` if item ID already exists
    pub fn add_item(&mut self, item: Item) -> Result<(), ItemDatabaseError> {
        if self.items.contains_key(&item.id) {
            return Err(ItemDatabaseError::DuplicateId(item.id));
        }
        self.items.insert(item.id, item);
        Ok(())
    }

    /// Get an item by ID
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::{ItemDatabase, Item, ItemType, WeaponData, WeaponClassification};
    /// use antares::domain::types::DiceRoll;
    ///
    /// let mut db = ItemDatabase::new();
    /// let club = Item {
    ///     id: 1,
    ///     name: "Club".to_string(),
    ///     item_type: ItemType::Weapon(WeaponData {
    ///         damage: DiceRoll::new(1, 3, 0),
    ///         bonus: 0,
    ///         hands_required: 1,
    ///         classification: WeaponClassification::Simple,
    ///     }),
    ///     base_cost: 1,
    ///     sell_cost: 0,
    ///     alignment_restriction: None,
    ///     constant_bonus: None,
    ///     temporary_bonus: None,
    ///     spell_effect: None,
    ///     max_charges: 0,
    ///     is_cursed: false,
    ///     icon_path: None,
    ///     tags: vec![],
    /// };
    /// db.add_item(club).unwrap();
    ///
    /// assert!(db.get_item(1).is_some());
    /// assert!(db.get_item(99).is_none());
    /// ```
    pub fn get_item(&self, id: ItemId) -> Option<&Item> {
        self.items.get(&id)
    }

    /// Get all items in the database
    pub fn all_items(&self) -> Vec<&Item> {
        self.items.values().collect()
    }

    /// Get all weapons
    pub fn get_weapons(&self) -> Vec<&Item> {
        self.items
            .values()
            .filter(|item| item.is_weapon())
            .collect()
    }

    /// Get all armor
    pub fn get_armor(&self) -> Vec<&Item> {
        self.items.values().filter(|item| item.is_armor()).collect()
    }

    /// Get all accessories
    pub fn get_accessories(&self) -> Vec<&Item> {
        self.items
            .values()
            .filter(|item| item.is_accessory())
            .collect()
    }

    /// Get all consumables
    pub fn get_consumables(&self) -> Vec<&Item> {
        self.items
            .values()
            .filter(|item| item.is_consumable())
            .collect()
    }

    /// Get all quest items
    pub fn get_quest_items(&self) -> Vec<&Item> {
        self.items
            .values()
            .filter(|item| item.is_quest_item())
            .collect()
    }

    /// Get all magical items
    pub fn get_magical_items(&self) -> Vec<&Item> {
        self.items
            .values()
            .filter(|item| item.is_magical())
            .collect()
    }

    /// Get number of items in database
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if database is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Check if an item exists in the database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::{ItemDatabase, Item, ItemType, WeaponData, WeaponClassification};
    /// use antares::domain::types::DiceRoll;
    ///
    /// let mut db = ItemDatabase::new();
    /// let club = Item {
    ///     id: 1,
    ///     name: "Club".to_string(),
    ///     item_type: ItemType::Weapon(WeaponData {
    ///         damage: DiceRoll::new(1, 3, 0),
    ///         bonus: 0,
    ///         hands_required: 1,
    ///         classification: WeaponClassification::Simple,
    ///     }),
    ///     base_cost: 1,
    ///     sell_cost: 0,
    ///     alignment_restriction: None,
    ///     constant_bonus: None,
    ///     temporary_bonus: None,
    ///     spell_effect: None,
    ///     max_charges: 0,
    ///     is_cursed: false,
    ///     icon_path: None,
    ///     tags: vec![],
    /// };
    /// db.add_item(club.clone()).unwrap();
    ///
    /// assert_eq!(db.all_items().len(), 1);
    /// assert!(db.has_item(&1));
    /// assert!(!db.has_item(&99));
    /// ```
    pub fn has_item(&self, id: &ItemId) -> bool {
        self.items.contains_key(id)
    }

    /// Validate that each item's required proficiency (derived from classification) exists
    /// in the given `ProficiencyDatabase`.
    ///
    /// # Arguments
    ///
    /// * `prof_db` - Reference to the loaded `ProficiencyDatabase` for cross-reference validation
    ///
    /// # Errors
    ///
    /// Returns `ItemDatabaseError::InvalidProficiency` if any item references a proficiency that
    /// does not exist in `prof_db`.
    pub fn validate_with_proficiency_db(
        &self,
        prof_db: &ProficiencyDatabase,
    ) -> Result<(), ItemDatabaseError> {
        for (id, item) in &self.items {
            if let Some(prof) = item.required_proficiency() {
                if !prof_db.has(&prof) {
                    return Err(ItemDatabaseError::InvalidProficiency(*id, prof.clone()));
                }
            }
        }
        Ok(())
    }
}

impl Default for ItemDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::proficiency::{
        ProficiencyCategory, ProficiencyDatabase, ProficiencyDefinition,
    };
    use crate::domain::types::DiceRoll;

    #[test]
    fn test_item_validate_with_proficiency_db_rejects_unknown_proficiency() {
        let mut db = ItemDatabase::new();

        let sword = Item {
            id: 250,
            name: "Test Sword".to_string(),
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
        };

        db.add_item(sword).unwrap();

        // Proficiency DB missing "martial_melee", validation must fail
        let prof_db = ProficiencyDatabase::new();
        let res = db.validate_with_proficiency_db(&prof_db);
        assert!(matches!(
            res,
            Err(ItemDatabaseError::InvalidProficiency(_, _))
        ));
    }

    #[test]
    fn test_item_validate_with_proficiency_db_accepts_known_proficiency() {
        let mut db = ItemDatabase::new();

        let sword = Item {
            id: 251,
            name: "Test Sword 2".to_string(),
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
        };

        db.add_item(sword).unwrap();

        // Proficiency DB contains "martial_melee"
        let mut prof_db = ProficiencyDatabase::new();
        let prof_def = ProficiencyDefinition::new(
            "martial_melee".to_string(),
            "Martial Melee".to_string(),
            ProficiencyCategory::Weapon,
        );
        prof_db.add(prof_def).unwrap();

        let res_ok = db.validate_with_proficiency_db(&prof_db);
        assert!(res_ok.is_ok());
    }
    use crate::domain::items::{ItemType, WeaponClassification, WeaponData};

    fn create_test_item(id: ItemId, name: &str) -> Item {
        Item {
            id,
            name: name.to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 6, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::Simple,
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
        }
    }

    #[test]
    fn test_new_database_is_empty() {
        let db = ItemDatabase::new();
        assert_eq!(db.len(), 0);
        assert!(db.is_empty());
    }

    #[test]
    fn test_add_and_retrieve_item() {
        let mut db = ItemDatabase::new();
        let item = create_test_item(1, "Test Sword");

        db.add_item(item.clone()).unwrap();

        assert_eq!(db.len(), 1);
        assert!(!db.is_empty());

        let retrieved = db.get_item(1).unwrap();
        assert_eq!(retrieved.name, "Test Sword");
    }

    #[test]
    fn test_duplicate_id_error() {
        let mut db = ItemDatabase::new();
        let item1 = create_test_item(1, "First");
        let item2 = create_test_item(1, "Second");

        db.add_item(item1).unwrap();
        let result = db.add_item(item2);

        assert!(result.is_err());
        assert!(matches!(result, Err(ItemDatabaseError::DuplicateId(1))));
    }

    #[test]
    fn test_get_nonexistent_item() {
        let db = ItemDatabase::new();
        assert!(db.get_item(99).is_none());
    }

    #[test]
    fn test_load_from_ron_string() {
        let ron_data = r#"
[
    (
        id: 1,
        name: "Club",
        item_type: Weapon((
            damage: (count: 1, sides: 3, bonus: 0),
            bonus: 0,
            hands_required: 1,
        )),
        base_cost: 1,
        sell_cost: 0,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    ),
    (
        id: 2,
        name: "Sword",
        item_type: Weapon((
            damage: (count: 1, sides: 8, bonus: 0),
            bonus: 0,
            hands_required: 1,
        )),
        base_cost: 10,
        sell_cost: 5,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
    ),
]
"#;

        let db = ItemDatabase::load_from_string(ron_data).unwrap();
        assert_eq!(db.len(), 2);
        assert!(db.get_item(1).is_some());
        assert!(db.get_item(2).is_some());
    }

    #[test]
    fn test_filter_weapons() {
        let mut db = ItemDatabase::new();
        db.add_item(create_test_item(1, "Sword")).unwrap();
        db.add_item(create_test_item(2, "Axe")).unwrap();

        let weapons = db.get_weapons();
        assert_eq!(weapons.len(), 2);
    }

    #[test]
    fn test_all_items() {
        let mut db = ItemDatabase::new();
        db.add_item(create_test_item(1, "Item1")).unwrap();
        db.add_item(create_test_item(2, "Item2")).unwrap();
        db.add_item(create_test_item(3, "Item3")).unwrap();

        let all = db.all_items();
        assert_eq!(all.len(), 3);
    }
}
