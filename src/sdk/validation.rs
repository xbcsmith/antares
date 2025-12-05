// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Cross-reference validation and balance checking
//!
//! This module provides validation capabilities for game content, including
//! cross-reference validation (ensuring referenced IDs exist), connectivity
//! checks (ensuring maps are reachable), and balance warnings.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sdk_implementation_plan.md` Phase 3.3 for specifications.
//!
//! # Examples
//!
//! ```no_run
//! use antares::sdk::database::ContentDatabase;
//! use antares::sdk::validation::Validator;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let db = ContentDatabase::load_campaign("campaigns/my_campaign")?;
//! let validator = Validator::new(&db);
//!
//! let errors = validator.validate_all()?;
//!
//! for error in &errors {
//!     eprintln!("{}", error);
//! }
//!
//! println!("Found {} validation issues", errors.len());
//! # Ok(())
//! # }
//! ```

use crate::domain::classes::ClassId;
use crate::domain::races::RaceId;
use crate::domain::types::{ItemId, MapId, MonsterId, SpellId};
use crate::sdk::database::ContentDatabase;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

// ===== Error Types =====

/// Severity level for validation issues
///
/// # Examples
///
/// ```
/// use antares::sdk::validation::Severity;
///
/// let error = Severity::Error;
/// let warning = Severity::Warning;
/// let info = Severity::Info;
///
/// assert!(error > warning);
/// assert!(warning > info);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Informational message
    Info,
    /// Warning - content may work but has issues
    Warning,
    /// Error - content is invalid and will cause problems
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => write!(f, "INFO"),
            Severity::Warning => write!(f, "WARNING"),
            Severity::Error => write!(f, "ERROR"),
        }
    }
}

/// Validation errors and warnings
///
/// Each variant represents a specific type of validation issue that can
/// occur in game content.
///
/// # Examples
///
/// ```
/// use antares::sdk::validation::{ValidationError, Severity};
///
/// let error = ValidationError::MissingClass {
///     context: "Character creation".to_string(),
///     class_id: "invalid_class".to_string(),
/// };
///
/// assert!(error.to_string().contains("invalid_class"));
/// assert_eq!(error.severity(), Severity::Error);
/// ```
#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationError {
    /// Referenced class does not exist
    #[error("Missing class '{class_id}' referenced in {context}")]
    MissingClass { context: String, class_id: ClassId },

    /// Referenced race does not exist
    #[error("Missing race '{race_id}' referenced in {context}")]
    MissingRace { context: String, race_id: RaceId },

    /// Referenced item does not exist
    #[error("Missing item ID {item_id} referenced in {context}")]
    MissingItem { context: String, item_id: ItemId },

    /// Referenced monster does not exist
    #[error("Missing monster ID {monster_id} on map {map}")]
    MissingMonster { map: MapId, monster_id: MonsterId },

    /// Referenced spell does not exist
    #[error("Missing spell ID {spell_id:#06x} referenced in {context}")]
    MissingSpell { context: String, spell_id: SpellId },

    /// Map is not connected to the world graph
    #[error("Disconnected map ID {map_id} - unreachable from starting location")]
    DisconnectedMap { map_id: MapId },

    /// Duplicate ID detected
    #[error("Duplicate {entity_type} ID: {id}")]
    DuplicateId { entity_type: String, id: String },

    /// Balance warning
    #[error("Balance issue ({severity}): {message}")]
    BalanceWarning { severity: Severity, message: String },
}

impl ValidationError {
    /// Returns the severity of this validation error
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::validation::{ValidationError, Severity};
    ///
    /// let error = ValidationError::MissingClass {
    ///     context: "test".to_string(),
    ///     class_id: "invalid".to_string(),
    /// };
    ///
    /// assert_eq!(error.severity(), Severity::Error);
    /// ```
    pub fn severity(&self) -> Severity {
        match self {
            ValidationError::MissingClass { .. }
            | ValidationError::MissingRace { .. }
            | ValidationError::MissingItem { .. }
            | ValidationError::MissingMonster { .. }
            | ValidationError::MissingSpell { .. }
            | ValidationError::DuplicateId { .. } => Severity::Error,

            ValidationError::DisconnectedMap { .. } => Severity::Warning,

            ValidationError::BalanceWarning { severity, .. } => *severity,
        }
    }

    /// Returns true if this is an error-level validation issue
    pub fn is_error(&self) -> bool {
        self.severity() == Severity::Error
    }

    /// Returns true if this is a warning-level validation issue
    pub fn is_warning(&self) -> bool {
        self.severity() == Severity::Warning
    }

    /// Returns true if this is an info-level validation issue
    pub fn is_info(&self) -> bool {
        self.severity() == Severity::Info
    }
}

// ===== Validator =====

/// Content validator for cross-reference and balance checking
///
/// The validator performs comprehensive validation on a ContentDatabase,
/// checking for missing references, disconnected content, and balance issues.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::database::ContentDatabase;
/// use antares::sdk::validation::Validator;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let db = ContentDatabase::load_campaign("campaigns/my_campaign")?;
/// let validator = Validator::new(&db);
///
/// let errors = validator.validate_all()?;
///
/// let error_count = errors.iter().filter(|e| e.is_error()).count();
/// let warning_count = errors.iter().filter(|e| e.is_warning()).count();
///
/// println!("Found {} errors, {} warnings", error_count, warning_count);
/// # Ok(())
/// # }
/// ```
pub struct Validator<'a> {
    db: &'a ContentDatabase,
}

impl<'a> Validator<'a> {
    /// Creates a new validator for the given content database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::database::ContentDatabase;
    /// use antares::sdk::validation::Validator;
    ///
    /// let db = ContentDatabase::new();
    /// let validator = Validator::new(&db);
    /// ```
    pub fn new(db: &'a ContentDatabase) -> Self {
        Self { db }
    }

    /// Validates all content and returns a list of validation errors
    ///
    /// This performs comprehensive validation including:
    /// - Cross-reference validation (checking all ID references)
    /// - Connectivity validation (ensuring maps are reachable)
    /// - Balance checking (identifying potential balance issues)
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<ValidationError>)` with all validation issues found.
    /// An empty vector means the content is valid.
    ///
    /// # Errors
    ///
    /// Returns an error only if validation itself fails (e.g., I/O errors).
    /// Content validation issues are returned as ValidationError items in the Vec.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::database::ContentDatabase;
    /// use antares::sdk::validation::Validator;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = ContentDatabase::new();
    /// let validator = Validator::new(&db);
    ///
    /// let errors = validator.validate_all()?;
    /// // Empty database generates warnings about missing content
    /// println!("Validation found {} issues", errors.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn validate_all(&self) -> Result<Vec<ValidationError>, Box<dyn std::error::Error>> {
        let mut errors = Vec::new();

        // Validate cross-references
        errors.extend(self.validate_references());

        // Validate map connectivity
        errors.extend(self.validate_connectivity());

        // Check balance issues
        errors.extend(self.check_balance());

        Ok(errors)
    }

    /// Validates all cross-references in the content
    ///
    /// Checks that all ID references (class IDs, race IDs, item IDs, etc.)
    /// point to existing content.
    fn validate_references(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Validate item references (e.g., items that reference classes)
        // This is a placeholder - actual validation depends on item structure
        for item in self.db.items.all_items() {
            // Check if item references valid classes via disablement flags
            // This would require inspecting the item's disablement field
            // and verifying against class database
            // Placeholder for now
            let _ = item;
        }

        // Validate spell references
        // Placeholder - would check if spells reference valid classes/items

        // Validate monster references
        // Placeholder - would check if monsters reference valid items (loot)

        // Validate map references
        // Placeholder - would check if maps reference valid monsters/items/events

        // Validate character definition references
        errors.extend(self.validate_character_references());

        errors
    }

    /// Validates character definition cross-references
    ///
    /// Checks that all character definitions reference:
    /// - Valid race IDs
    /// - Valid class IDs
    /// - Valid item IDs for starting items
    /// - Valid item IDs for starting equipment
    fn validate_character_references(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for character in self.db.characters.all_characters() {
            let context = format!("character '{}'", character.id);

            // Check race reference
            if !self.db.races.has_race(&character.race_id) {
                errors.push(ValidationError::MissingRace {
                    context: context.clone(),
                    race_id: character.race_id.clone(),
                });
            }

            // Check class reference
            if self.db.classes.get_class(&character.class_id).is_none() {
                errors.push(ValidationError::MissingClass {
                    context: context.clone(),
                    class_id: character.class_id.clone(),
                });
            }

            // Check starting items references
            for item_id in &character.starting_items {
                if !self.db.items.has_item(item_id) {
                    errors.push(ValidationError::MissingItem {
                        context: format!("{} starting items", context),
                        item_id: *item_id,
                    });
                }
            }

            // Check starting equipment references
            for item_id in character.starting_equipment.all_item_ids() {
                if !self.db.items.has_item(&item_id) {
                    errors.push(ValidationError::MissingItem {
                        context: format!("{} starting equipment", context),
                        item_id,
                    });
                }
            }
        }

        errors
    }

    /// Validates that all maps are connected to the world graph
    ///
    /// Ensures that every map is reachable from the starting location
    /// through a chain of map transitions.
    fn validate_connectivity(&self) -> Vec<ValidationError> {
        let errors = Vec::new();

        // Placeholder implementation
        // This would perform a graph traversal starting from the initial map
        // to identify disconnected map islands

        // For now, just check if we have maps
        let map_count = self.db.maps.count();
        if map_count > 0 {
            // Would perform actual connectivity check here
        }

        errors
    }

    /// Checks for balance issues in the content
    ///
    /// Identifies potential balance problems such as:
    /// - Items that are too powerful for their availability
    /// - Monsters that are too strong/weak for their location
    /// - Inconsistent progression curves
    fn check_balance(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check for empty content databases
        let stats = self.db.stats();

        if stats.class_count == 0 {
            errors.push(ValidationError::BalanceWarning {
                severity: Severity::Warning,
                message: "No classes defined in database".to_string(),
            });
        }

        if stats.item_count == 0 {
            errors.push(ValidationError::BalanceWarning {
                severity: Severity::Info,
                message: "No items defined in database".to_string(),
            });
        }

        if stats.monster_count == 0 {
            errors.push(ValidationError::BalanceWarning {
                severity: Severity::Info,
                message: "No monsters defined in database".to_string(),
            });
        }

        if stats.map_count == 0 {
            errors.push(ValidationError::BalanceWarning {
                severity: Severity::Warning,
                message: "No maps defined in database".to_string(),
            });
        }

        // Additional balance checks would go here
        // - Check class HP progression
        // - Check item power curves
        // - Check monster difficulty scaling
        // - Check spell balance

        errors
    }

    /// Validates a single map for errors and warnings
    ///
    /// Performs comprehensive validation on a map including:
    /// - Event cross-references (monster IDs, item IDs, map IDs)
    /// - NPC validation
    /// - Map connectivity (teleport destinations)
    /// - Balance checks (monster encounters, treasure)
    ///
    /// # Arguments
    ///
    /// * `map` - The map to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<ValidationError>)` with all validation issues found.
    /// An empty vector means the map is valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::database::ContentDatabase;
    /// use antares::sdk::validation::Validator;
    /// use antares::domain::world::Map;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = ContentDatabase::new();
    /// let validator = Validator::new(&db);
    /// let map = Map::new(0, "Test Map".to_string(), "Description".to_string(), 10, 10);
    ///
    /// let errors = validator.validate_map(&map)?;
    /// println!("Map has {} validation issues", errors.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn validate_map(
        &self,
        map: &crate::domain::world::Map,
    ) -> Result<Vec<ValidationError>, Box<dyn std::error::Error>> {
        let mut errors = Vec::new();

        // Validate map events
        for (pos, event) in &map.events {
            match event {
                crate::domain::world::MapEvent::Encounter { monster_group, .. } => {
                    // Validate monster IDs
                    for &monster_id in monster_group {
                        if !self.db.monsters.has_monster(&(monster_id as MonsterId)) {
                            errors.push(ValidationError::MissingMonster {
                                map: map.id,
                                monster_id: monster_id as MonsterId,
                            });
                        }
                    }
                }
                crate::domain::world::MapEvent::Treasure { loot, .. } => {
                    // Validate item IDs
                    for &item_id in loot {
                        if !self.db.items.has_item(&(item_id as ItemId)) {
                            errors.push(ValidationError::MissingItem {
                                context: format!(
                                    "Map {} treasure at ({}, {})",
                                    map.id, pos.x, pos.y
                                ),
                                item_id: item_id as ItemId,
                            });
                        }
                    }
                }
                crate::domain::world::MapEvent::Teleport {
                    map_id,
                    destination,
                    ..
                } => {
                    // Validate destination map exists
                    if !self.db.maps.has_map(map_id) {
                        errors.push(ValidationError::BalanceWarning {
                            severity: Severity::Error,
                            message: format!(
                                "Map {} has teleport to non-existent map {} at ({}, {})",
                                map.id, map_id, pos.x, pos.y
                            ),
                        });
                    }

                    // Check if destination is valid (we can't fully validate without loading the target map)
                    let _ = destination; // Used in error message if needed
                }
                crate::domain::world::MapEvent::Trap { damage, .. } => {
                    // Balance check for trap damage
                    if *damage > 100 {
                        errors.push(ValidationError::BalanceWarning {
                            severity: Severity::Warning,
                            message: format!(
                                "Map {} has high-damage trap ({} damage) at ({}, {})",
                                map.id, damage, pos.x, pos.y
                            ),
                        });
                    }
                }
                crate::domain::world::MapEvent::Sign { .. } => {
                    // Signs are always valid
                }
                crate::domain::world::MapEvent::NpcDialogue { npc_id, .. } => {
                    // Validate NPC exists on this map
                    if !map.npcs.iter().any(|npc| npc.id == *npc_id) {
                        errors.push(ValidationError::BalanceWarning {
                            severity: Severity::Error,
                            message: format!(
                                "Map {} has NPC dialogue event for non-existent NPC {} at ({}, {})",
                                map.id, npc_id, pos.x, pos.y
                            ),
                        });
                    }
                }
            }
        }

        // Validate NPCs
        for npc in &map.npcs {
            // Check if NPC position is valid
            if !map.is_valid_position(npc.position) {
                errors.push(ValidationError::BalanceWarning {
                    severity: Severity::Error,
                    message: format!(
                        "Map {} has NPC '{}' (ID {}) at invalid position ({}, {})",
                        map.id, npc.name, npc.id, npc.position.x, npc.position.y
                    ),
                });
            }
        }

        // Check for duplicate NPC IDs
        let mut seen_npc_ids = std::collections::HashSet::new();
        for npc in &map.npcs {
            if !seen_npc_ids.insert(npc.id) {
                errors.push(ValidationError::DuplicateId {
                    entity_type: "NPC".to_string(),
                    id: npc.id.to_string(),
                });
            }
        }

        // Balance checks
        if map.events.len() > 1000 {
            errors.push(ValidationError::BalanceWarning {
                severity: Severity::Warning,
                message: format!(
                    "Map {} has {} events, which may cause performance issues",
                    map.id,
                    map.events.len()
                ),
            });
        }

        if map.npcs.len() > 100 {
            errors.push(ValidationError::BalanceWarning {
                severity: Severity::Warning,
                message: format!(
                    "Map {} has {} NPCs, which may cause performance issues",
                    map.id,
                    map.npcs.len()
                ),
            });
        }

        Ok(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdk::database::ContentDatabase;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
        assert!(Severity::Error > Severity::Info);
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Error.to_string(), "ERROR");
        assert_eq!(Severity::Warning.to_string(), "WARNING");
        assert_eq!(Severity::Info.to_string(), "INFO");
    }

    #[test]
    fn test_validation_error_severity() {
        let error = ValidationError::MissingClass {
            context: "test".to_string(),
            class_id: "invalid".to_string(),
        };
        assert_eq!(error.severity(), Severity::Error);
        assert!(error.is_error());
        assert!(!error.is_warning());
        assert!(!error.is_info());

        let warning = ValidationError::DisconnectedMap { map_id: 1 };
        assert_eq!(warning.severity(), Severity::Warning);
        assert!(!warning.is_error());
        assert!(warning.is_warning());
        assert!(!warning.is_info());

        let info = ValidationError::BalanceWarning {
            severity: Severity::Info,
            message: "test".to_string(),
        };
        assert_eq!(info.severity(), Severity::Info);
        assert!(!info.is_error());
        assert!(!info.is_warning());
        assert!(info.is_info());
    }

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError::MissingClass {
            context: "Character creation".to_string(),
            class_id: "knight".to_string(),
        };
        let display = error.to_string();
        assert!(display.contains("knight"));
        assert!(display.contains("Character creation"));

        let error = ValidationError::MissingItem {
            context: "Loot table".to_string(),
            item_id: 42,
        };
        let display = error.to_string();
        assert!(display.contains("42"));
        assert!(display.contains("Loot table"));
    }

    #[test]
    fn test_validator_new() {
        let db = ContentDatabase::new();
        let _validator = Validator::new(&db);
    }

    #[test]
    fn test_validator_empty_database() {
        let db = ContentDatabase::new();
        let validator = Validator::new(&db);

        let errors = validator.validate_all().unwrap();

        // Empty database should generate warnings about missing content
        assert!(!errors.is_empty());

        // Should have warnings about empty databases
        let has_class_warning = errors.iter().any(|e| {
            matches!(
                e,
                ValidationError::BalanceWarning { message, .. }
                if message.contains("No classes")
            )
        });
        assert!(has_class_warning);
    }

    #[test]
    fn test_validator_filters_by_severity() {
        let db = ContentDatabase::new();
        let validator = Validator::new(&db);
        let errors = validator.validate_all().unwrap();

        let error_count = errors.iter().filter(|e| e.is_error()).count();
        let warning_count = errors.iter().filter(|e| e.is_warning()).count();
        let info_count = errors.iter().filter(|e| e.is_info()).count();

        // Empty database should have warnings/info, no errors
        assert_eq!(error_count, 0);
        assert!(warning_count > 0 || info_count > 0);
    }

    #[test]
    fn test_validation_error_missing_spell_format() {
        let error = ValidationError::MissingSpell {
            context: "Class spell list".to_string(),
            spell_id: 0x0101,
        };
        let display = error.to_string();
        // Should display spell ID in hex format
        assert!(display.contains("0x0101") || display.contains("101"));
    }

    #[test]
    fn test_validation_error_clone() {
        let error = ValidationError::MissingClass {
            context: "test".to_string(),
            class_id: "knight".to_string(),
        };
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_validator_character_references_valid() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::CharacterDefinition;
        use crate::domain::classes::ClassDefinition;
        use crate::domain::races::RaceDefinition;

        let mut db = ContentDatabase::new();

        // Add valid race and class
        let human_race = RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "A versatile race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        let knight_class = ClassDefinition::new("knight".to_string(), "Knight".to_string());
        db.classes.add_class(knight_class).unwrap();

        // Add a character with valid references
        let knight = CharacterDefinition::new(
            "test_knight".to_string(),
            "Sir Test".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        db.characters.add_character(knight).unwrap();

        let validator = Validator::new(&db);
        let errors = validator.validate_all().unwrap();

        // Should have no MissingClass or MissingRace errors for our character
        let character_errors: Vec<_> = errors
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    ValidationError::MissingClass { context, .. }
                    | ValidationError::MissingRace { context, .. }
                    if context.contains("test_knight")
                )
            })
            .collect();
        assert!(
            character_errors.is_empty(),
            "Should have no character reference errors"
        );
    }

    #[test]
    fn test_validator_character_invalid_race() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::CharacterDefinition;
        use crate::domain::classes::ClassDefinition;

        let mut db = ContentDatabase::new();

        // Add class but NOT race
        let knight_class = ClassDefinition::new("knight".to_string(), "Knight".to_string());
        db.classes.add_class(knight_class).unwrap();

        // Add a character with invalid race reference
        let knight = CharacterDefinition::new(
            "test_knight".to_string(),
            "Sir Test".to_string(),
            "nonexistent_race".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        db.characters.add_character(knight).unwrap();

        let validator = Validator::new(&db);
        let errors = validator.validate_all().unwrap();

        // Should have a MissingRace error
        let has_race_error = errors.iter().any(|e| {
            matches!(
                e,
                ValidationError::MissingRace { race_id, .. }
                if race_id == "nonexistent_race"
            )
        });
        assert!(has_race_error, "Should detect missing race reference");
    }

    #[test]
    fn test_validator_character_invalid_class() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::CharacterDefinition;
        use crate::domain::races::RaceDefinition;

        let mut db = ContentDatabase::new();

        // Add race but NOT class
        let human_race = RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "A versatile race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        // Add a character with invalid class reference
        let knight = CharacterDefinition::new(
            "test_knight".to_string(),
            "Sir Test".to_string(),
            "human".to_string(),
            "nonexistent_class".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        db.characters.add_character(knight).unwrap();

        let validator = Validator::new(&db);
        let errors = validator.validate_all().unwrap();

        // Should have a MissingClass error
        let has_class_error = errors.iter().any(|e| {
            matches!(
                e,
                ValidationError::MissingClass { class_id, .. }
                if class_id == "nonexistent_class"
            )
        });
        assert!(has_class_error, "Should detect missing class reference");
    }

    #[test]
    fn test_validator_character_invalid_starting_items() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::CharacterDefinition;
        use crate::domain::classes::ClassDefinition;
        use crate::domain::races::RaceDefinition;

        let mut db = ContentDatabase::new();

        // Add valid race and class
        let human_race = RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "A versatile race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        let knight_class = ClassDefinition::new("knight".to_string(), "Knight".to_string());
        db.classes.add_class(knight_class).unwrap();

        // Add a character with invalid starting items
        let mut knight = CharacterDefinition::new(
            "test_knight".to_string(),
            "Sir Test".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        knight.starting_items = vec![200, 201]; // Non-existent item IDs
        db.characters.add_character(knight).unwrap();

        let validator = Validator::new(&db);
        let errors = validator.validate_all().unwrap();

        // Should have MissingItem errors
        let item_errors: Vec<_> = errors
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    ValidationError::MissingItem { context, .. }
                    if context.contains("test_knight")
                )
            })
            .collect();
        assert_eq!(
            item_errors.len(),
            2,
            "Should detect both missing item references"
        );
    }

    #[test]
    fn test_validator_character_invalid_starting_equipment() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::{CharacterDefinition, StartingEquipment};
        use crate::domain::classes::ClassDefinition;
        use crate::domain::races::RaceDefinition;

        let mut db = ContentDatabase::new();

        // Add valid race and class
        let human_race = RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "A versatile race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        let knight_class = ClassDefinition::new("knight".to_string(), "Knight".to_string());
        db.classes.add_class(knight_class).unwrap();

        // Add a character with invalid starting equipment
        let mut knight = CharacterDefinition::new(
            "test_knight".to_string(),
            "Sir Test".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        knight.starting_equipment = StartingEquipment {
            weapon: Some(250), // Non-existent item ID
            armor: Some(251),  // Non-existent item ID
            ..StartingEquipment::new()
        };
        db.characters.add_character(knight).unwrap();

        let validator = Validator::new(&db);
        let errors = validator.validate_all().unwrap();

        // Should have MissingItem errors for equipment
        let equipment_errors: Vec<_> = errors
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    ValidationError::MissingItem { context, .. }
                    if context.contains("equipment")
                )
            })
            .collect();
        assert_eq!(
            equipment_errors.len(),
            2,
            "Should detect both missing equipment item references"
        );
    }
}
