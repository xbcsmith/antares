// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Proficiency system for item usage restrictions
//!
//! This module implements the proficiency system that determines which items
//! a character can use based on their class and race. Proficiencies are
//! granted by classes and races, and items require specific proficiencies
//! based on their classification.
//!
//! # Design Principles
//!
//! 1. **No proficiency inheritance** - Each proficiency is independent;
//!    `martial_melee` does NOT imply `simple_weapon`
//! 2. **UNION logic** - A character can use an item if EITHER their class
//!    OR their race grants the required proficiency
//! 3. **Classification-based** - Items derive their proficiency requirement
//!    from their classification (weapon/armor/magic item type)
//!
//! # Architecture Reference
//!
//! See `docs/explanation/proficiency_migration_plan.md` for the complete design.
//!
//! # Examples
//!
//! ```
//! use antares::domain::proficiency::{ProficiencyDatabase, ProficiencyCategory};
//! use antares::domain::items::WeaponClassification;
//!
//! // Get proficiency ID from weapon classification
//! let prof_id = ProficiencyDatabase::proficiency_for_weapon(WeaponClassification::MartialMelee);
//! assert_eq!(prof_id, "martial_melee");
//!
//! // Check if a proficiency list contains the required proficiency
//! let class_profs = vec!["simple_weapon".to_string(), "martial_melee".to_string()];
//! assert!(class_profs.contains(&prof_id));
//! ```

use crate::domain::items::{ArmorClassification, MagicItemClassification, WeaponClassification};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur when working with proficiency definitions
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ProficiencyError {
    /// Proficiency not found in database
    #[error("Proficiency not found: {0}")]
    ProficiencyNotFound(String),

    /// Failed to load proficiency database from file
    #[error("Failed to load proficiency database from file: {0}")]
    LoadError(String),

    /// Failed to parse proficiency data
    #[error("Failed to parse proficiency data: {0}")]
    ParseError(String),

    /// Validation error in proficiency data
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Duplicate proficiency ID detected
    #[error("Duplicate proficiency ID: {0}")]
    DuplicateId(String),
}

// ===== Type Aliases =====

/// Unique identifier for a proficiency
///
/// Maps to classification enum variants (e.g., "simple_weapon", "light_armor").
/// Using String allows for extensibility and data-driven definitions.
pub type ProficiencyId = String;

// ===== Enums =====

/// Category for grouping proficiencies in UI
///
/// Proficiencies are organized into categories for display purposes
/// and for filtering in editors.
///
/// # Examples
///
/// ```
/// use antares::domain::proficiency::ProficiencyCategory;
///
/// let category = ProficiencyCategory::Weapon;
/// assert_ne!(category, ProficiencyCategory::Armor);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ProficiencyCategory {
    /// Weapon proficiencies (swords, bows, etc.)
    #[default]
    Weapon,
    /// Armor proficiencies (light, medium, heavy)
    Armor,
    /// Shield proficiency
    Shield,
    /// Magic item proficiencies (arcane, divine)
    MagicItem,
}

// ===== Structures =====

/// A proficiency definition loaded from data files
///
/// Proficiencies define what types of items a character can use.
/// They are granted by classes and races, and required by items
/// based on their classification.
///
/// # Examples
///
/// ```
/// use antares::domain::proficiency::{ProficiencyDefinition, ProficiencyCategory};
///
/// let prof = ProficiencyDefinition {
///     id: "martial_melee".to_string(),
///     name: "Martial Melee Weapons".to_string(),
///     category: ProficiencyCategory::Weapon,
///     description: "Swords, axes, maces".to_string(),
/// };
///
/// assert_eq!(prof.id, "martial_melee");
/// assert_eq!(prof.category, ProficiencyCategory::Weapon);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProficiencyDefinition {
    /// Unique identifier (e.g., "martial_melee")
    pub id: ProficiencyId,

    /// Display name (e.g., "Martial Melee Weapons")
    pub name: String,

    /// Category for UI grouping
    pub category: ProficiencyCategory,

    /// Description for tooltips
    #[serde(default)]
    pub description: String,
}

impl ProficiencyDefinition {
    /// Creates a new proficiency definition
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the proficiency
    /// * `name` - Display name for the proficiency
    /// * `category` - Category for grouping
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::{ProficiencyDefinition, ProficiencyCategory};
    ///
    /// let prof = ProficiencyDefinition::new(
    ///     "simple_weapon".to_string(),
    ///     "Simple Weapons".to_string(),
    ///     ProficiencyCategory::Weapon,
    /// );
    /// assert_eq!(prof.id, "simple_weapon");
    /// ```
    pub fn new(id: ProficiencyId, name: String, category: ProficiencyCategory) -> Self {
        Self {
            id,
            name,
            category,
            description: String::new(),
        }
    }

    /// Creates a proficiency definition with a description
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the proficiency
    /// * `name` - Display name for the proficiency
    /// * `category` - Category for grouping
    /// * `description` - Description for tooltips
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::{ProficiencyDefinition, ProficiencyCategory};
    ///
    /// let prof = ProficiencyDefinition::with_description(
    ///     "martial_melee".to_string(),
    ///     "Martial Melee Weapons".to_string(),
    ///     ProficiencyCategory::Weapon,
    ///     "Swords, axes, maces".to_string(),
    /// );
    /// assert!(!prof.description.is_empty());
    /// ```
    pub fn with_description(
        id: ProficiencyId,
        name: String,
        category: ProficiencyCategory,
        description: String,
    ) -> Self {
        Self {
            id,
            name,
            category,
            description,
        }
    }
}

/// Database of all proficiency definitions
///
/// The proficiency database loads proficiency definitions from RON files
/// and provides lookup and validation functions.
///
/// # Examples
///
/// ```no_run
/// use antares::domain::proficiency::ProficiencyDatabase;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let db = ProficiencyDatabase::load_from_file("data/proficiencies.ron")?;
///
/// if let Some(prof) = db.get("martial_melee") {
///     println!("Found: {}", prof.name);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct ProficiencyDatabase {
    proficiencies: HashMap<ProficiencyId, ProficiencyDefinition>,
}

impl ProficiencyDatabase {
    /// Creates a new empty proficiency database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::ProficiencyDatabase;
    ///
    /// let db = ProficiencyDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            proficiencies: HashMap::new(),
        }
    }

    /// Loads proficiency definitions from a RON file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file containing proficiency definitions
    ///
    /// # Returns
    ///
    /// Returns a `ProficiencyDatabase` on success
    ///
    /// # Errors
    ///
    /// Returns `ProficiencyError::LoadError` if the file cannot be read
    /// Returns `ProficiencyError::ParseError` if the RON syntax is invalid
    /// Returns `ProficiencyError::DuplicateId` if duplicate IDs are found
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::proficiency::ProficiencyDatabase;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = ProficiencyDatabase::load_from_file("data/proficiencies.ron")?;
    /// assert!(!db.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ProficiencyError> {
        let contents = std::fs::read_to_string(path.as_ref())
            .map_err(|e| ProficiencyError::LoadError(e.to_string()))?;

        Self::load_from_string(&contents)
    }

    /// Loads proficiency definitions from a RON string
    ///
    /// # Arguments
    ///
    /// * `contents` - RON-formatted string containing proficiency definitions
    ///
    /// # Returns
    ///
    /// Returns a `ProficiencyDatabase` on success
    ///
    /// # Errors
    ///
    /// Returns `ProficiencyError::ParseError` if the RON syntax is invalid
    /// Returns `ProficiencyError::DuplicateId` if duplicate IDs are found
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::ProficiencyDatabase;
    ///
    /// let ron = r#"[
    ///     (
    ///         id: "simple_weapon",
    ///         name: "Simple Weapons",
    ///         category: Weapon,
    ///         description: "Basic weapons",
    ///     ),
    /// ]"#;
    ///
    /// let db = ProficiencyDatabase::load_from_string(ron).unwrap();
    /// assert_eq!(db.len(), 1);
    /// ```
    pub fn load_from_string(contents: &str) -> Result<Self, ProficiencyError> {
        let definitions: Vec<ProficiencyDefinition> =
            ron::from_str(contents).map_err(|e| ProficiencyError::ParseError(e.to_string()))?;

        let mut db = Self::new();

        for def in definitions {
            if db.proficiencies.contains_key(&def.id) {
                return Err(ProficiencyError::DuplicateId(def.id));
            }
            db.proficiencies.insert(def.id.clone(), def);
        }

        Ok(db)
    }

    /// Gets a proficiency definition by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The proficiency ID to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(&ProficiencyDefinition)` if found, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::{ProficiencyDatabase, ProficiencyDefinition, ProficiencyCategory};
    ///
    /// let mut db = ProficiencyDatabase::new();
    /// db.add(ProficiencyDefinition::new(
    ///     "test".to_string(),
    ///     "Test".to_string(),
    ///     ProficiencyCategory::Weapon,
    /// )).unwrap();
    ///
    /// assert!(db.get("test").is_some());
    /// assert!(db.get("nonexistent").is_none());
    /// ```
    pub fn get(&self, id: &str) -> Option<&ProficiencyDefinition> {
        self.proficiencies.get(id)
    }

    /// Returns all proficiency definitions
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::ProficiencyDatabase;
    ///
    /// let db = ProficiencyDatabase::new();
    /// assert!(db.all().is_empty());
    /// ```
    pub fn all(&self) -> Vec<&ProficiencyDefinition> {
        self.proficiencies.values().collect()
    }

    /// Returns all proficiency IDs
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::{ProficiencyDatabase, ProficiencyDefinition, ProficiencyCategory};
    ///
    /// let mut db = ProficiencyDatabase::new();
    /// db.add(ProficiencyDefinition::new(
    ///     "test".to_string(),
    ///     "Test".to_string(),
    ///     ProficiencyCategory::Weapon,
    /// )).unwrap();
    ///
    /// let ids: Vec<_> = db.all_ids().collect();
    /// assert!(ids.contains(&&"test".to_string()));
    /// ```
    pub fn all_ids(&self) -> impl Iterator<Item = &ProficiencyId> {
        self.proficiencies.keys()
    }

    /// Returns proficiencies filtered by category
    ///
    /// # Arguments
    ///
    /// * `category` - The category to filter by
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::{ProficiencyDatabase, ProficiencyDefinition, ProficiencyCategory};
    ///
    /// let mut db = ProficiencyDatabase::new();
    /// db.add(ProficiencyDefinition::new(
    ///     "sword".to_string(),
    ///     "Sword".to_string(),
    ///     ProficiencyCategory::Weapon,
    /// )).unwrap();
    /// db.add(ProficiencyDefinition::new(
    ///     "leather".to_string(),
    ///     "Leather".to_string(),
    ///     ProficiencyCategory::Armor,
    /// )).unwrap();
    ///
    /// let weapons = db.by_category(ProficiencyCategory::Weapon);
    /// assert_eq!(weapons.len(), 1);
    /// ```
    pub fn by_category(&self, category: ProficiencyCategory) -> Vec<&ProficiencyDefinition> {
        self.proficiencies
            .values()
            .filter(|p| p.category == category)
            .collect()
    }

    /// Validates the proficiency database
    ///
    /// Checks for:
    /// - Empty IDs
    /// - Empty names
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if validation passes
    ///
    /// # Errors
    ///
    /// Returns `ProficiencyError::ValidationError` if any proficiency is invalid
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::{ProficiencyDatabase, ProficiencyDefinition, ProficiencyCategory};
    ///
    /// let mut db = ProficiencyDatabase::new();
    /// db.add(ProficiencyDefinition::new(
    ///     "test".to_string(),
    ///     "Test Prof".to_string(),
    ///     ProficiencyCategory::Weapon,
    /// )).unwrap();
    ///
    /// assert!(db.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), ProficiencyError> {
        for (id, prof) in &self.proficiencies {
            if id.is_empty() {
                return Err(ProficiencyError::ValidationError(
                    "Proficiency ID cannot be empty".to_string(),
                ));
            }

            if prof.name.is_empty() {
                return Err(ProficiencyError::ValidationError(format!(
                    "Proficiency '{}' has empty name",
                    id
                )));
            }

            if id != &prof.id {
                return Err(ProficiencyError::ValidationError(format!(
                    "Proficiency key '{}' does not match id '{}'",
                    id, prof.id
                )));
            }
        }

        Ok(())
    }

    /// Adds a proficiency definition to the database
    ///
    /// # Arguments
    ///
    /// * `definition` - The proficiency definition to add
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the proficiency was added successfully
    ///
    /// # Errors
    ///
    /// Returns `ProficiencyError::DuplicateId` if a proficiency with the same ID exists
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::{ProficiencyDatabase, ProficiencyDefinition, ProficiencyCategory};
    ///
    /// let mut db = ProficiencyDatabase::new();
    /// let result = db.add(ProficiencyDefinition::new(
    ///     "test".to_string(),
    ///     "Test".to_string(),
    ///     ProficiencyCategory::Weapon,
    /// ));
    /// assert!(result.is_ok());
    /// ```
    pub fn add(&mut self, definition: ProficiencyDefinition) -> Result<(), ProficiencyError> {
        if self.proficiencies.contains_key(&definition.id) {
            return Err(ProficiencyError::DuplicateId(definition.id));
        }
        self.proficiencies.insert(definition.id.clone(), definition);
        Ok(())
    }

    /// Removes a proficiency from the database
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the proficiency to remove
    ///
    /// # Returns
    ///
    /// Returns `Some(ProficiencyDefinition)` if found and removed, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::{ProficiencyDatabase, ProficiencyDefinition, ProficiencyCategory};
    ///
    /// let mut db = ProficiencyDatabase::new();
    /// db.add(ProficiencyDefinition::new(
    ///     "test".to_string(),
    ///     "Test".to_string(),
    ///     ProficiencyCategory::Weapon,
    /// )).unwrap();
    ///
    /// let removed = db.remove("test");
    /// assert!(removed.is_some());
    /// assert!(db.is_empty());
    /// ```
    pub fn remove(&mut self, id: &str) -> Option<ProficiencyDefinition> {
        self.proficiencies.remove(id)
    }

    /// Returns the number of proficiencies in the database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::ProficiencyDatabase;
    ///
    /// let db = ProficiencyDatabase::new();
    /// assert_eq!(db.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.proficiencies.len()
    }

    /// Returns true if the database is empty
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::ProficiencyDatabase;
    ///
    /// let db = ProficiencyDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.proficiencies.is_empty()
    }

    /// Checks if a proficiency exists in the database
    ///
    /// # Arguments
    ///
    /// * `id` - The proficiency ID to check
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::{ProficiencyDatabase, ProficiencyDefinition, ProficiencyCategory};
    ///
    /// let mut db = ProficiencyDatabase::new();
    /// db.add(ProficiencyDefinition::new(
    ///     "test".to_string(),
    ///     "Test".to_string(),
    ///     ProficiencyCategory::Weapon,
    /// )).unwrap();
    ///
    /// assert!(db.has("test"));
    /// assert!(!db.has("nonexistent"));
    /// ```
    pub fn has(&self, id: &str) -> bool {
        self.proficiencies.contains_key(id)
    }

    // ===== Classification to Proficiency Mapping =====

    /// Get proficiency ID from weapon classification
    ///
    /// Maps a weapon's classification to the required proficiency ID.
    ///
    /// # Arguments
    ///
    /// * `classification` - The weapon classification
    ///
    /// # Returns
    ///
    /// The proficiency ID required to use weapons of this classification
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::ProficiencyDatabase;
    /// use antares::domain::items::WeaponClassification;
    ///
    /// let prof = ProficiencyDatabase::proficiency_for_weapon(WeaponClassification::Simple);
    /// assert_eq!(prof, "simple_weapon");
    ///
    /// let prof = ProficiencyDatabase::proficiency_for_weapon(WeaponClassification::MartialMelee);
    /// assert_eq!(prof, "martial_melee");
    /// ```
    pub fn proficiency_for_weapon(classification: WeaponClassification) -> ProficiencyId {
        match classification {
            WeaponClassification::Simple => "simple_weapon".to_string(),
            WeaponClassification::MartialMelee => "martial_melee".to_string(),
            WeaponClassification::MartialRanged => "martial_ranged".to_string(),
            WeaponClassification::Blunt => "blunt_weapon".to_string(),
            WeaponClassification::Unarmed => "unarmed".to_string(),
        }
    }

    /// Get proficiency ID from armor classification
    ///
    /// Maps an armor's classification to the required proficiency ID.
    ///
    /// # Arguments
    ///
    /// * `classification` - The armor classification
    ///
    /// # Returns
    ///
    /// The proficiency ID required to use armor of this classification
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::ProficiencyDatabase;
    /// use antares::domain::items::ArmorClassification;
    ///
    /// let prof = ProficiencyDatabase::proficiency_for_armor(ArmorClassification::Light);
    /// assert_eq!(prof, "light_armor");
    ///
    /// let prof = ProficiencyDatabase::proficiency_for_armor(ArmorClassification::Shield);
    /// assert_eq!(prof, "shield");
    /// ```
    pub fn proficiency_for_armor(classification: ArmorClassification) -> ProficiencyId {
        match classification {
            ArmorClassification::Light => "light_armor".to_string(),
            ArmorClassification::Medium => "medium_armor".to_string(),
            ArmorClassification::Heavy => "heavy_armor".to_string(),
            ArmorClassification::Shield => "shield".to_string(),
        }
    }

    /// Get proficiency ID from magic item classification
    ///
    /// Maps a magic item's classification to the required proficiency ID.
    /// Universal items return None as they require no proficiency.
    ///
    /// # Arguments
    ///
    /// * `classification` - The magic item classification
    ///
    /// # Returns
    ///
    /// `Some(ProficiencyId)` for arcane/divine items, `None` for universal items
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::proficiency::ProficiencyDatabase;
    /// use antares::domain::items::MagicItemClassification;
    ///
    /// let prof = ProficiencyDatabase::proficiency_for_magic_item(MagicItemClassification::Arcane);
    /// assert_eq!(prof, Some("arcane_item".to_string()));
    ///
    /// let prof = ProficiencyDatabase::proficiency_for_magic_item(MagicItemClassification::Universal);
    /// assert_eq!(prof, None);
    /// ```
    pub fn proficiency_for_magic_item(
        classification: MagicItemClassification,
    ) -> Option<ProficiencyId> {
        match classification {
            MagicItemClassification::Arcane => Some("arcane_item".to_string()),
            MagicItemClassification::Divine => Some("divine_item".to_string()),
            MagicItemClassification::Universal => None,
        }
    }
}

// ===== Helper Functions =====

/// Checks if a character has a proficiency using UNION logic
///
/// A character has a proficiency if EITHER their class OR their race grants it.
/// This is the core UNION logic for the proficiency system.
///
/// # Arguments
///
/// * `required_proficiency` - The proficiency ID required, or None if no proficiency needed
/// * `class_proficiencies` - Proficiencies granted by the character's class
/// * `race_proficiencies` - Proficiencies granted by the character's race
///
/// # Returns
///
/// `true` if no proficiency is required, or if either class or race grants it
///
/// # Examples
///
/// ```
/// use antares::domain::proficiency::has_proficiency_union;
///
/// let class_profs = vec!["simple_weapon".to_string(), "light_armor".to_string()];
/// let race_profs = vec!["martial_ranged".to_string()];
///
/// // Class grants simple_weapon
/// assert!(has_proficiency_union(
///     Some(&"simple_weapon".to_string()),
///     &class_profs,
///     &race_profs,
/// ));
///
/// // Race grants martial_ranged
/// assert!(has_proficiency_union(
///     Some(&"martial_ranged".to_string()),
///     &class_profs,
///     &race_profs,
/// ));
///
/// // Neither grants martial_melee
/// assert!(!has_proficiency_union(
///     Some(&"martial_melee".to_string()),
///     &class_profs,
///     &race_profs,
/// ));
///
/// // No proficiency required
/// assert!(has_proficiency_union(None, &class_profs, &race_profs));
/// ```
pub fn has_proficiency_union(
    required_proficiency: Option<&ProficiencyId>,
    class_proficiencies: &[ProficiencyId],
    race_proficiencies: &[ProficiencyId],
) -> bool {
    match required_proficiency {
        None => true,
        Some(prof) => class_proficiencies.contains(prof) || race_proficiencies.contains(prof),
    }
}

/// Checks if an item's tags are compatible with a race's restrictions
///
/// Returns true if the item has NO tags that appear in the race's
/// incompatible tags list.
///
/// # Arguments
///
/// * `item_tags` - Tags on the item
/// * `incompatible_tags` - Tags the race cannot use
///
/// # Returns
///
/// `true` if the race can use the item (no incompatible tags match)
///
/// # Examples
///
/// ```
/// use antares::domain::proficiency::is_item_compatible_with_race;
///
/// let item_tags = vec!["large_weapon".to_string(), "two_handed".to_string()];
/// let halfling_incompatible = vec!["large_weapon".to_string(), "heavy_armor".to_string()];
/// let human_incompatible: Vec<String> = vec![];
///
/// // Halfling can't use large weapons
/// assert!(!is_item_compatible_with_race(&item_tags, &halfling_incompatible));
///
/// // Human has no restrictions
/// assert!(is_item_compatible_with_race(&item_tags, &human_incompatible));
///
/// // Item with no problematic tags
/// let small_item_tags = vec!["light".to_string()];
/// assert!(is_item_compatible_with_race(&small_item_tags, &halfling_incompatible));
/// ```
pub fn is_item_compatible_with_race(item_tags: &[String], incompatible_tags: &[String]) -> bool {
    !item_tags.iter().any(|tag| incompatible_tags.contains(tag))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== ProficiencyDefinition Tests =====

    #[test]
    fn test_proficiency_definition_new() {
        let prof = ProficiencyDefinition::new(
            "test".to_string(),
            "Test Proficiency".to_string(),
            ProficiencyCategory::Weapon,
        );

        assert_eq!(prof.id, "test");
        assert_eq!(prof.name, "Test Proficiency");
        assert_eq!(prof.category, ProficiencyCategory::Weapon);
        assert!(prof.description.is_empty());
    }

    #[test]
    fn test_proficiency_definition_with_description() {
        let prof = ProficiencyDefinition::with_description(
            "test".to_string(),
            "Test".to_string(),
            ProficiencyCategory::Armor,
            "A test description".to_string(),
        );

        assert_eq!(prof.description, "A test description");
    }

    // ===== ProficiencyCategory Tests =====

    #[test]
    fn test_proficiency_category_default() {
        let category = ProficiencyCategory::default();
        assert_eq!(category, ProficiencyCategory::Weapon);
    }

    #[test]
    fn test_proficiency_category_equality() {
        assert_eq!(ProficiencyCategory::Weapon, ProficiencyCategory::Weapon);
        assert_ne!(ProficiencyCategory::Weapon, ProficiencyCategory::Armor);
    }

    // ===== ProficiencyDatabase Tests =====

    #[test]
    fn test_database_new() {
        let db = ProficiencyDatabase::new();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
    }

    #[test]
    fn test_database_add() {
        let mut db = ProficiencyDatabase::new();
        let prof = ProficiencyDefinition::new(
            "test".to_string(),
            "Test".to_string(),
            ProficiencyCategory::Weapon,
        );

        let result = db.add(prof);
        assert!(result.is_ok());
        assert_eq!(db.len(), 1);
        assert!(db.has("test"));
    }

    #[test]
    fn test_database_add_duplicate() {
        let mut db = ProficiencyDatabase::new();
        let prof1 = ProficiencyDefinition::new(
            "test".to_string(),
            "Test".to_string(),
            ProficiencyCategory::Weapon,
        );
        let prof2 = ProficiencyDefinition::new(
            "test".to_string(),
            "Test 2".to_string(),
            ProficiencyCategory::Armor,
        );

        db.add(prof1).unwrap();
        let result = db.add(prof2);

        assert!(matches!(result, Err(ProficiencyError::DuplicateId(_))));
    }

    #[test]
    fn test_database_get() {
        let mut db = ProficiencyDatabase::new();
        db.add(ProficiencyDefinition::new(
            "test".to_string(),
            "Test Prof".to_string(),
            ProficiencyCategory::Weapon,
        ))
        .unwrap();

        let prof = db.get("test");
        assert!(prof.is_some());
        assert_eq!(prof.unwrap().name, "Test Prof");

        assert!(db.get("nonexistent").is_none());
    }

    #[test]
    fn test_database_remove() {
        let mut db = ProficiencyDatabase::new();
        db.add(ProficiencyDefinition::new(
            "test".to_string(),
            "Test".to_string(),
            ProficiencyCategory::Weapon,
        ))
        .unwrap();

        let removed = db.remove("test");
        assert!(removed.is_some());
        assert!(db.is_empty());

        let removed_again = db.remove("test");
        assert!(removed_again.is_none());
    }

    #[test]
    fn test_database_all() {
        let mut db = ProficiencyDatabase::new();
        db.add(ProficiencyDefinition::new(
            "a".to_string(),
            "A".to_string(),
            ProficiencyCategory::Weapon,
        ))
        .unwrap();
        db.add(ProficiencyDefinition::new(
            "b".to_string(),
            "B".to_string(),
            ProficiencyCategory::Armor,
        ))
        .unwrap();

        let all = db.all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_database_all_ids() {
        let mut db = ProficiencyDatabase::new();
        db.add(ProficiencyDefinition::new(
            "a".to_string(),
            "A".to_string(),
            ProficiencyCategory::Weapon,
        ))
        .unwrap();
        db.add(ProficiencyDefinition::new(
            "b".to_string(),
            "B".to_string(),
            ProficiencyCategory::Armor,
        ))
        .unwrap();

        let ids: Vec<_> = db.all_ids().collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&&"a".to_string()));
        assert!(ids.contains(&&"b".to_string()));
    }

    #[test]
    fn test_database_by_category() {
        let mut db = ProficiencyDatabase::new();
        db.add(ProficiencyDefinition::new(
            "sword".to_string(),
            "Sword".to_string(),
            ProficiencyCategory::Weapon,
        ))
        .unwrap();
        db.add(ProficiencyDefinition::new(
            "bow".to_string(),
            "Bow".to_string(),
            ProficiencyCategory::Weapon,
        ))
        .unwrap();
        db.add(ProficiencyDefinition::new(
            "leather".to_string(),
            "Leather".to_string(),
            ProficiencyCategory::Armor,
        ))
        .unwrap();

        let weapons = db.by_category(ProficiencyCategory::Weapon);
        assert_eq!(weapons.len(), 2);

        let armor = db.by_category(ProficiencyCategory::Armor);
        assert_eq!(armor.len(), 1);

        let shields = db.by_category(ProficiencyCategory::Shield);
        assert!(shields.is_empty());
    }

    #[test]
    fn test_database_validate_success() {
        let mut db = ProficiencyDatabase::new();
        db.add(ProficiencyDefinition::new(
            "test".to_string(),
            "Test".to_string(),
            ProficiencyCategory::Weapon,
        ))
        .unwrap();

        assert!(db.validate().is_ok());
    }

    #[test]
    fn test_database_load_from_string() {
        let ron = r#"[
            (
                id: "simple_weapon",
                name: "Simple Weapons",
                category: Weapon,
                description: "Basic weapons",
            ),
            (
                id: "light_armor",
                name: "Light Armor",
                category: Armor,
            ),
        ]"#;

        let db = ProficiencyDatabase::load_from_string(ron).unwrap();
        assert_eq!(db.len(), 2);
        assert!(db.has("simple_weapon"));
        assert!(db.has("light_armor"));
    }

    #[test]
    fn test_database_load_from_string_duplicate() {
        let ron = r#"[
            (
                id: "test",
                name: "Test 1",
                category: Weapon,
            ),
            (
                id: "test",
                name: "Test 2",
                category: Armor,
            ),
        ]"#;

        let result = ProficiencyDatabase::load_from_string(ron);
        assert!(matches!(result, Err(ProficiencyError::DuplicateId(_))));
    }

    #[test]
    fn test_database_load_from_string_parse_error() {
        let ron = "invalid ron syntax {{{";

        let result = ProficiencyDatabase::load_from_string(ron);
        assert!(matches!(result, Err(ProficiencyError::ParseError(_))));
    }

    // ===== Classification Mapping Tests =====

    #[test]
    fn test_proficiency_for_weapon() {
        assert_eq!(
            ProficiencyDatabase::proficiency_for_weapon(WeaponClassification::Simple),
            "simple_weapon"
        );
        assert_eq!(
            ProficiencyDatabase::proficiency_for_weapon(WeaponClassification::MartialMelee),
            "martial_melee"
        );
        assert_eq!(
            ProficiencyDatabase::proficiency_for_weapon(WeaponClassification::MartialRanged),
            "martial_ranged"
        );
        assert_eq!(
            ProficiencyDatabase::proficiency_for_weapon(WeaponClassification::Blunt),
            "blunt_weapon"
        );
        assert_eq!(
            ProficiencyDatabase::proficiency_for_weapon(WeaponClassification::Unarmed),
            "unarmed"
        );
    }

    #[test]
    fn test_proficiency_for_armor() {
        assert_eq!(
            ProficiencyDatabase::proficiency_for_armor(ArmorClassification::Light),
            "light_armor"
        );
        assert_eq!(
            ProficiencyDatabase::proficiency_for_armor(ArmorClassification::Medium),
            "medium_armor"
        );
        assert_eq!(
            ProficiencyDatabase::proficiency_for_armor(ArmorClassification::Heavy),
            "heavy_armor"
        );
        assert_eq!(
            ProficiencyDatabase::proficiency_for_armor(ArmorClassification::Shield),
            "shield"
        );
    }

    #[test]
    fn test_proficiency_for_magic_item() {
        assert_eq!(
            ProficiencyDatabase::proficiency_for_magic_item(MagicItemClassification::Arcane),
            Some("arcane_item".to_string())
        );
        assert_eq!(
            ProficiencyDatabase::proficiency_for_magic_item(MagicItemClassification::Divine),
            Some("divine_item".to_string())
        );
        assert_eq!(
            ProficiencyDatabase::proficiency_for_magic_item(MagicItemClassification::Universal),
            None
        );
    }

    // ===== Helper Function Tests =====

    #[test]
    fn test_has_proficiency_union_class_grants() {
        let class_profs = vec!["simple_weapon".to_string()];
        let race_profs: Vec<String> = vec![];

        assert!(has_proficiency_union(
            Some(&"simple_weapon".to_string()),
            &class_profs,
            &race_profs,
        ));
    }

    #[test]
    fn test_has_proficiency_union_race_grants() {
        let class_profs: Vec<String> = vec![];
        let race_profs = vec!["martial_ranged".to_string()];

        assert!(has_proficiency_union(
            Some(&"martial_ranged".to_string()),
            &class_profs,
            &race_profs,
        ));
    }

    #[test]
    fn test_has_proficiency_union_both_grant() {
        let class_profs = vec!["simple_weapon".to_string()];
        let race_profs = vec!["simple_weapon".to_string()];

        assert!(has_proficiency_union(
            Some(&"simple_weapon".to_string()),
            &class_profs,
            &race_profs,
        ));
    }

    #[test]
    fn test_has_proficiency_union_neither_grants() {
        let class_profs = vec!["simple_weapon".to_string()];
        let race_profs = vec!["martial_ranged".to_string()];

        assert!(!has_proficiency_union(
            Some(&"heavy_armor".to_string()),
            &class_profs,
            &race_profs,
        ));
    }

    #[test]
    fn test_has_proficiency_union_no_requirement() {
        let class_profs: Vec<String> = vec![];
        let race_profs: Vec<String> = vec![];

        assert!(has_proficiency_union(None, &class_profs, &race_profs,));
    }

    #[test]
    fn test_is_item_compatible_no_tags() {
        let item_tags: Vec<String> = vec![];
        let incompatible_tags = vec!["large_weapon".to_string()];

        assert!(is_item_compatible_with_race(&item_tags, &incompatible_tags));
    }

    #[test]
    fn test_is_item_compatible_no_restrictions() {
        let item_tags = vec!["large_weapon".to_string()];
        let incompatible_tags: Vec<String> = vec![];

        assert!(is_item_compatible_with_race(&item_tags, &incompatible_tags));
    }

    #[test]
    fn test_is_item_compatible_incompatible() {
        let item_tags = vec!["large_weapon".to_string(), "two_handed".to_string()];
        let incompatible_tags = vec!["large_weapon".to_string()];

        assert!(!is_item_compatible_with_race(
            &item_tags,
            &incompatible_tags
        ));
    }

    #[test]
    fn test_is_item_compatible_no_overlap() {
        let item_tags = vec!["light".to_string(), "one_handed".to_string()];
        let incompatible_tags = vec!["large_weapon".to_string(), "heavy_armor".to_string()];

        assert!(is_item_compatible_with_race(&item_tags, &incompatible_tags));
    }

    // ===== Load from file test =====

    #[test]
    fn test_load_proficiencies_from_data_file() {
        // Try to load from the actual data file
        let result = ProficiencyDatabase::load_from_file("data/proficiencies.ron");
        if let Ok(db) = result {
            // Should have 11 standard proficiencies
            assert!(db.len() >= 11, "Expected at least 11 proficiencies");

            // Check weapon proficiencies
            assert!(db.has("simple_weapon"));
            assert!(db.has("martial_melee"));
            assert!(db.has("martial_ranged"));
            assert!(db.has("blunt_weapon"));
            assert!(db.has("unarmed"));

            // Check armor proficiencies
            assert!(db.has("light_armor"));
            assert!(db.has("medium_armor"));
            assert!(db.has("heavy_armor"));
            assert!(db.has("shield"));

            // Check magic item proficiencies
            assert!(db.has("arcane_item"));
            assert!(db.has("divine_item"));

            // Validate database
            assert!(db.validate().is_ok());
        }
        // If file doesn't exist yet, that's OK during Phase 1
    }
}
