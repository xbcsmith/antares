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
use crate::domain::visual::creature_database::CreatureDatabase;
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

    #[error("Item ID {item_id} produced invalid mesh descriptor: {message}")]
    InvalidMeshDescriptor { item_id: ItemId, message: String },

    #[error("Item ID {item_id} references unknown item mesh creature ID {creature_id}")]
    UnknownMeshOverride {
        /// Item whose `mesh_descriptor_override` references an unknown ID
        item_id: ItemId,
        /// The creature ID that was not found in the `ItemMeshDatabase`
        creature_id: u32,
    },
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
    ///     mesh_descriptor_override: None,
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
    ///     mesh_descriptor_override: None,
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

    /// Validates that every item in the database produces a well-formed
    /// [`ItemMeshDescriptor`] and a valid [`CreatureDefinition`].
    ///
    /// Calls [`ItemMeshDescriptor::from_item`] for every loaded item and then
    /// calls [`CreatureDefinition::validate`] on the resulting definition.
    /// This catches configuration errors early — before the game engine tries
    /// to spawn the item mesh at runtime.
    ///
    /// # Errors
    ///
    /// Returns `ItemDatabaseError::InvalidMeshDescriptor` for the first item
    /// whose auto-generated [`CreatureDefinition`] fails validation.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::{ItemDatabase, Item, ItemType, WeaponData, WeaponClassification};
    /// use antares::domain::types::DiceRoll;
    ///
    /// let mut db = ItemDatabase::new();
    /// let sword = Item {
    ///     id: 1,
    ///     name: "Short Sword".to_string(),
    ///     item_type: ItemType::Weapon(WeaponData {
    ///         damage: DiceRoll::new(1, 6, 0),
    ///         bonus: 0,
    ///         hands_required: 1,
    ///         classification: WeaponClassification::MartialMelee,
    ///     }),
    ///     base_cost: 10,
    ///     sell_cost: 5,
    ///     alignment_restriction: None,
    ///     constant_bonus: None,
    ///     temporary_bonus: None,
    ///     spell_effect: None,
    ///     max_charges: 0,
    ///     is_cursed: false,
    ///     icon_path: None,
    ///     tags: vec![],
    ///     mesh_descriptor_override: None,
    /// };
    /// db.add_item(sword).unwrap();
    /// assert!(db.validate_mesh_descriptors().is_ok());
    /// ```
    pub fn validate_mesh_descriptors(&self) -> Result<(), ItemDatabaseError> {
        use crate::domain::visual::item_mesh::ItemMeshDescriptor;

        for (id, item) in &self.items {
            let descriptor = ItemMeshDescriptor::from_item(item);
            let creature_def = descriptor.to_creature_definition();
            creature_def.validate().map_err(|message| {
                ItemDatabaseError::InvalidMeshDescriptor {
                    item_id: *id,
                    message,
                }
            })?;
        }
        Ok(())
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

    /// Validate that every item whose `mesh_descriptor_override` carries an explicit
    /// creature ID exists in the supplied `ItemMeshDatabase`.
    ///
    /// Currently the domain `ItemMeshDescriptorOverride` does not store a creature ID
    /// directly — that link lives at the campaign layer.  This method is therefore a
    /// forward-compatibility hook: it validates the override *if* an explicit
    /// `creature_id` field is ever added to `ItemMeshDescriptorOverride`.  For now it
    /// simply walks all items and confirms that any item whose `mesh_descriptor_override`
    /// is `Some` can still produce a valid `CreatureDefinition` via
    /// `ItemMeshDescriptor::from_item`, ensuring the override does not break mesh
    /// generation.  Full registry cross-linking is performed separately by
    /// `CampaignLoader`.
    ///
    /// # Arguments
    ///
    /// * `registry` - The `ItemMeshDatabase` (thin wrapper around `CreatureDatabase`)
    ///   loaded for this campaign.
    ///
    /// # Errors
    ///
    /// Returns `ItemDatabaseError::InvalidMeshDescriptor` if the descriptor produced
    /// for an override item fails `CreatureDefinition::validate`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::database::{ItemDatabase, ItemMeshDatabase};
    ///
    /// let item_db = ItemDatabase::new();
    /// let mesh_db = ItemMeshDatabase::new();
    /// assert!(item_db.link_mesh_overrides(&mesh_db).is_ok());
    /// ```
    pub fn link_mesh_overrides(
        &self,
        _registry: &ItemMeshDatabase,
    ) -> Result<(), ItemDatabaseError> {
        use crate::domain::visual::item_mesh::ItemMeshDescriptor;

        for (id, item) in &self.items {
            // Only validate items that carry an explicit override
            if item.mesh_descriptor_override.is_some() {
                let descriptor = ItemMeshDescriptor::from_item(item);
                let creature_def = descriptor.to_creature_definition();
                creature_def.validate().map_err(|message| {
                    ItemDatabaseError::InvalidMeshDescriptor {
                        item_id: *id,
                        message,
                    }
                })?;
            }
        }
        Ok(())
    }
}

// ===== ItemMeshDatabase =====

/// Thin wrapper around [`CreatureDatabase`] dedicated to item-mesh assets.
///
/// Item mesh RON files share the same [`CreatureDefinition`] format as creature
/// meshes.  A separate wrapper type prevents accidental mixing of creature IDs
/// (1–8999) with item mesh IDs (9000+) and provides a named type that the
/// campaign loader and SDK can pass around without confusion.
///
/// # Examples
///
/// ```
/// use antares::domain::items::database::ItemMeshDatabase;
///
/// let db = ItemMeshDatabase::new();
/// assert!(db.is_empty());
/// assert_eq!(db.count(), 0);
/// ```
#[derive(Debug, Clone, Default)]
pub struct ItemMeshDatabase {
    inner: CreatureDatabase,
}

impl ItemMeshDatabase {
    /// Creates a new, empty `ItemMeshDatabase`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::database::ItemMeshDatabase;
    ///
    /// let db = ItemMeshDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            inner: CreatureDatabase::new(),
        }
    }

    /// Creates an `ItemMeshDatabase` by loading from a registry RON file.
    ///
    /// The registry file is a `Vec<CreatureReference>` (same format as
    /// `data/creatures.ron`) where each entry points at a per-item mesh file.
    ///
    /// # Arguments
    ///
    /// * `registry_path` - Path to the `item_mesh_registry.ron` file.
    /// * `campaign_root` - Root of the campaign directory; asset file paths in
    ///   the registry are resolved relative to this root.
    ///
    /// # Errors
    ///
    /// Returns `CreatureDatabaseError` (wrapped) if the registry or any asset
    /// file cannot be read or parsed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::items::database::ItemMeshDatabase;
    /// use std::path::Path;
    ///
    /// let db = ItemMeshDatabase::load_from_registry(
    ///     Path::new("data/test_campaign/data/item_mesh_registry.ron"),
    ///     Path::new("data/test_campaign"),
    /// ).unwrap();
    /// assert!(!db.is_empty());
    /// ```
    pub fn load_from_registry(
        registry_path: &std::path::Path,
        campaign_root: &std::path::Path,
    ) -> Result<Self, crate::domain::visual::creature_database::CreatureDatabaseError> {
        let inner = CreatureDatabase::load_from_registry(registry_path, campaign_root)?;
        Ok(Self { inner })
    }

    /// Returns a reference to the underlying [`CreatureDatabase`].
    ///
    /// Useful for querying mesh definitions directly via the creature API.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::database::ItemMeshDatabase;
    ///
    /// let db = ItemMeshDatabase::new();
    /// assert!(db.as_creature_database().is_empty());
    /// ```
    pub fn as_creature_database(&self) -> &CreatureDatabase {
        &self.inner
    }

    /// Returns `true` if the database has no entries.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::database::ItemMeshDatabase;
    ///
    /// let db = ItemMeshDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of item mesh entries.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::database::ItemMeshDatabase;
    ///
    /// let db = ItemMeshDatabase::new();
    /// assert_eq!(db.count(), 0);
    /// ```
    pub fn count(&self) -> usize {
        self.inner.count()
    }

    /// Returns `true` if a mesh entry with the given ID exists.
    ///
    /// # Arguments
    ///
    /// * `id` - Creature ID to look up (should be in the 9000+ range).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::database::ItemMeshDatabase;
    ///
    /// let db = ItemMeshDatabase::new();
    /// assert!(!db.has_mesh(9001));
    /// ```
    pub fn has_mesh(&self, id: u32) -> bool {
        self.inner.has_creature(id)
    }

    /// Validates all mesh entries in the database.
    ///
    /// # Errors
    ///
    /// Returns `CreatureDatabaseError::ValidationError` if any mesh is malformed.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::database::ItemMeshDatabase;
    ///
    /// let db = ItemMeshDatabase::new();
    /// assert!(db.validate().is_ok());
    /// ```
    pub fn validate(
        &self,
    ) -> Result<(), crate::domain::visual::creature_database::CreatureDatabaseError> {
        self.inner.validate()
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
            mesh_descriptor_override: None,
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
            mesh_descriptor_override: None,
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
            mesh_descriptor_override: None,
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

    /// Loads `data/items.ron` and asserts that every item in it produces a
    /// valid procedural mesh descriptor.  This is the canonical regression
    /// test that catches any future item additions that break the mesh
    /// generation pipeline.
    #[test]
    fn test_validate_mesh_descriptors_all_base_items() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = std::path::PathBuf::from(manifest_dir).join("data/items.ron");

        // If the file does not exist (e.g., stripped CI image) skip gracefully.
        if !path.exists() {
            eprintln!("SKIP: data/items.ron not found at {:?}", path);
            return;
        }

        let db = ItemDatabase::load_from_file(&path)
            .unwrap_or_else(|e| panic!("Failed to load data/items.ron: {}", e));

        assert!(
            !db.is_empty(),
            "data/items.ron loaded but contained no items"
        );

        db.validate_mesh_descriptors().unwrap_or_else(|e| {
            panic!("validate_mesh_descriptors failed for data/items.ron: {}", e)
        });
    }

    /// Verifies that validate_mesh_descriptors returns Ok for an empty database.
    #[test]
    fn test_validate_mesh_descriptors_empty_db() {
        let db = ItemDatabase::new();
        assert!(db.validate_mesh_descriptors().is_ok());
    }

    /// Verifies that validate_mesh_descriptors returns Ok for a single
    /// hand-crafted item of every ItemType variant.
    #[test]
    fn test_validate_mesh_descriptors_all_item_types() {
        use crate::domain::items::types::{
            AccessoryData, AccessorySlot, AmmoData, AmmoType, ArmorClassification, ArmorData,
            ConsumableData, ConsumableEffect, QuestData,
        };

        let mut db = ItemDatabase::new();

        // Weapon
        db.add_item(Item {
            id: 1,
            name: "Test Sword".to_string(),
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
        })
        .unwrap();

        // Armor
        db.add_item(Item {
            id: 2,
            name: "Leather Armor".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 2,
                weight: 15,
                classification: ArmorClassification::Light,
            }),
            base_cost: 5,
            sell_cost: 2,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
        })
        .unwrap();

        // Accessory
        db.add_item(Item {
            id: 3,
            name: "Plain Ring".to_string(),
            item_type: ItemType::Accessory(AccessoryData {
                slot: AccessorySlot::Ring,
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
            mesh_descriptor_override: None,
        })
        .unwrap();

        // Consumable
        db.add_item(Item {
            id: 4,
            name: "Healing Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(20),
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
            mesh_descriptor_override: None,
        })
        .unwrap();

        // Ammo
        db.add_item(Item {
            id: 5,
            name: "Arrows".to_string(),
            item_type: ItemType::Ammo(AmmoData {
                ammo_type: AmmoType::Arrow,
                quantity: 20,
            }),
            base_cost: 2,
            sell_cost: 1,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
        })
        .unwrap();

        // Quest
        db.add_item(Item {
            id: 6,
            name: "Ancient Key".to_string(),
            item_type: ItemType::Quest(QuestData {
                quest_id: "main_quest".to_string(),
                is_key_item: true,
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
            tags: vec![],
            mesh_descriptor_override: None,
        })
        .unwrap();

        assert!(
            db.validate_mesh_descriptors().is_ok(),
            "All base item types must produce valid mesh descriptors"
        );
    }

    // ── ItemMeshDatabase unit tests ────────────────────────────────────────

    /// `ItemMeshDatabase::new()` starts empty.
    #[test]
    fn test_item_mesh_database_new_is_empty() {
        let db = ItemMeshDatabase::new();
        assert!(db.is_empty());
        assert_eq!(db.count(), 0);
    }

    /// `ItemMeshDatabase::default()` is equivalent to `new()`.
    #[test]
    fn test_item_mesh_database_default_is_empty() {
        let db = ItemMeshDatabase::default();
        assert!(db.is_empty());
    }

    /// `has_mesh` returns `false` for an id that has never been inserted.
    #[test]
    fn test_item_mesh_database_has_mesh_absent() {
        let db = ItemMeshDatabase::new();
        assert!(!db.has_mesh(9001));
        assert!(!db.has_mesh(0));
    }

    /// `validate()` on an empty database succeeds.
    #[test]
    fn test_item_mesh_database_validate_empty() {
        let db = ItemMeshDatabase::new();
        assert!(db.validate().is_ok());
    }

    /// `as_creature_database()` returns the inner database.
    #[test]
    fn test_item_mesh_database_as_creature_database() {
        let db = ItemMeshDatabase::new();
        assert!(db.as_creature_database().is_empty());
    }

    /// `load_from_registry` with a non-existent path returns an error.
    #[test]
    fn test_item_mesh_database_load_from_registry_missing_file() {
        let result = ItemMeshDatabase::load_from_registry(
            std::path::Path::new("nonexistent/item_mesh_registry.ron"),
            std::path::Path::new("nonexistent"),
        );
        assert!(result.is_err(), "Expected error for missing registry file");
    }

    /// `load_from_registry` correctly loads the test-campaign fixture with
    /// at least two entries (sword id=9001 and potion id=9201).
    #[test]
    fn test_item_mesh_database_load_from_registry_test_campaign() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let campaign_root = std::path::PathBuf::from(manifest_dir).join("data/test_campaign");
        let registry_path = campaign_root.join("data/item_mesh_registry.ron");

        if !registry_path.exists() {
            eprintln!("SKIP: data/test_campaign/data/item_mesh_registry.ron not found");
            return;
        }

        let db = ItemMeshDatabase::load_from_registry(&registry_path, &campaign_root)
            .expect("load_from_registry should succeed for test_campaign");

        assert!(
            db.count() >= 2,
            "Expected ≥ 2 item mesh entries, got {}",
            db.count()
        );
        assert!(db.has_mesh(9001), "Expected sword mesh (id 9001)");
        assert!(db.has_mesh(9201), "Expected potion mesh (id 9201)");
        assert!(!db.is_empty());
    }

    /// `validate()` on the loaded test-campaign registry succeeds.
    #[test]
    fn test_item_mesh_database_validate_test_campaign() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let campaign_root = std::path::PathBuf::from(manifest_dir).join("data/test_campaign");
        let registry_path = campaign_root.join("data/item_mesh_registry.ron");

        if !registry_path.exists() {
            eprintln!("SKIP: data/test_campaign/data/item_mesh_registry.ron not found");
            return;
        }

        let db = ItemMeshDatabase::load_from_registry(&registry_path, &campaign_root)
            .expect("load_from_registry should succeed");

        assert!(
            db.validate().is_ok(),
            "ItemMeshDatabase loaded from test_campaign must validate without errors"
        );
    }

    // ── link_mesh_overrides unit tests ─────────────────────────────────────

    /// `link_mesh_overrides` succeeds on an empty `ItemDatabase`.
    #[test]
    fn test_link_mesh_overrides_empty_item_db() {
        let item_db = ItemDatabase::new();
        let mesh_db = ItemMeshDatabase::new();
        assert!(item_db.link_mesh_overrides(&mesh_db).is_ok());
    }

    /// Items without `mesh_descriptor_override` are skipped by
    /// `link_mesh_overrides`.
    #[test]
    fn test_link_mesh_overrides_no_override_items_skipped() {
        let mut item_db = ItemDatabase::new();
        item_db
            .add_item(create_test_item(1, "Plain Sword"))
            .unwrap();

        let mesh_db = ItemMeshDatabase::new();
        assert!(
            item_db.link_mesh_overrides(&mesh_db).is_ok(),
            "Items without overrides must not cause link_mesh_overrides to fail"
        );
    }

    /// An item with a valid `mesh_descriptor_override` passes validation.
    #[test]
    fn test_link_mesh_overrides_valid_override_passes() {
        use crate::domain::visual::item_mesh::ItemMeshDescriptorOverride;

        let mut item_db = ItemDatabase::new();
        item_db
            .add_item(Item {
                id: 100,
                name: "Fancy Sword".to_string(),
                item_type: ItemType::Weapon(WeaponData {
                    damage: DiceRoll::new(1, 8, 0),
                    bonus: 0,
                    hands_required: 1,
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
                tags: vec![],
                mesh_descriptor_override: Some(ItemMeshDescriptorOverride {
                    primary_color: Some([0.8, 0.2, 0.2, 1.0]),
                    accent_color: None,
                    scale: Some(0.4),
                    emissive: None,
                }),
            })
            .unwrap();

        let mesh_db = ItemMeshDatabase::new();
        assert!(
            item_db.link_mesh_overrides(&mesh_db).is_ok(),
            "Valid override must pass link_mesh_overrides"
        );
    }
}
