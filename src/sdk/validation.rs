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

    /// Too many starting party members
    #[error("Too many starting party members: {count} characters have starts_in_party=true, but max party size is {max}")]
    TooManyStartingPartyMembers { count: usize, max: usize },

    /// Error indicating the campaign `starting_innkeeper` configuration is invalid.
    ///
    /// This occurs when the configured innkeeper ID is:
    /// - empty,
    /// - not present in the campaign's NPC database, or
    /// - present but not marked as an innkeeper (`is_innkeeper = true`).
    #[error("Invalid starting innkeeper '{innkeeper_id}': {reason}")]
    InvalidStartingInnkeeper {
        /// The innkeeper ID from the campaign configuration that failed validation.
        innkeeper_id: String,
        /// Human-readable reason describing why the configured innkeeper is invalid.
        reason: String,
    },
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
            | ValidationError::DuplicateId { .. }
            | ValidationError::InvalidStartingInnkeeper { .. } => Severity::Error,

            ValidationError::DisconnectedMap { .. } => Severity::Warning,

            ValidationError::BalanceWarning { severity, .. } => *severity,

            ValidationError::TooManyStartingPartyMembers { .. } => Severity::Error,
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
    /// - Character validation (party size limits, valid references)
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

        // Validate characters (party size, references)
        errors.extend(self.validate_characters());

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

    /// Validates character definitions for party management constraints
    ///
    /// Checks that:
    /// - No more than 6 characters have `starts_in_party: true`
    /// - All character references are valid (handled by validate_character_references)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::database::ContentDatabase;
    /// use antares::sdk::validation::Validator;
    ///
    /// let db = ContentDatabase::new();
    /// let validator = Validator::new(&db);
    ///
    /// // Run the public validation entrypoint (includes character-related checks)
    /// let errors = validator.validate_all().expect("validation failed");
    /// // Check for party size violations
    /// ```
    fn validate_characters(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Count characters with starts_in_party = true
        let starting_party_count = self
            .db
            .characters
            .premade_characters()
            .filter(|c| c.starts_in_party)
            .count();

        // Enforce max party size of 6
        const MAX_PARTY_SIZE: usize = 6;
        if starting_party_count > MAX_PARTY_SIZE {
            errors.push(ValidationError::TooManyStartingPartyMembers {
                count: starting_party_count,
                max: MAX_PARTY_SIZE,
            });
        }

        errors
    }

    /// Validates that all maps are connected to the world graph
    ///
    /// Ensures that every map is reachable from the starting location
    /// through a chain of map transitions.
    /// Validates campaign configuration values that depend on loaded content.
    ///
    /// Ensures configuration fields that reference content (NPCs, maps, etc.)
    /// are valid with respect to the provided `ContentDatabase`. Currently this
    /// performs checks for the `starting_innkeeper` value:
    ///
    /// - The `starting_innkeeper` ID is non-empty.
    /// - The referenced NPC exists in the NPC database.
    /// - The referenced NPC has `is_innkeeper == true`.
    ///
    /// # Arguments
    ///
    /// * `config` - Campaign configuration to validate
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::database::ContentDatabase;
    /// use antares::sdk::validation::Validator;
    /// use antares::sdk::campaign_loader::{CampaignConfig, Difficulty};
    /// use antares::domain::types::{Position, Direction};
    /// use antares::sdk::validation::ValidationError;
    ///
    /// let db = ContentDatabase::new();
    /// let validator = Validator::new(&db);
    /// let config = CampaignConfig {
    ///     starting_map: 1,
    ///     starting_position: Position::new(0, 0),
    ///     starting_direction: Direction::North,
    ///     starting_gold: 0,
    ///     starting_food: 0,
    ///     starting_innkeeper: "nonexistent_inn".to_string(),
    ///     max_party_size: 6,
    ///     max_roster_size: 20,
    ///     difficulty: Difficulty::Normal,
    ///     permadeath: false,
    ///     allow_multiclassing: false,
    ///     starting_level: 1,
    ///     max_level: 20,
    /// };
    ///
    /// let errors = validator.validate_campaign_config(&config);
    /// assert!(errors.iter().any(|e| matches!(e, ValidationError::InvalidStartingInnkeeper { .. })));
    /// ```
    pub fn validate_campaign_config(
        &self,
        config: &crate::sdk::campaign_loader::CampaignConfig,
    ) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Validate starting_innkeeper exists and is an innkeeper
        if config.starting_innkeeper.is_empty() {
            errors.push(ValidationError::InvalidStartingInnkeeper {
                innkeeper_id: config.starting_innkeeper.clone(),
                reason: "Starting innkeeper ID is empty".to_string(),
            });
        } else if let Some(npc) = self.db.npcs.get_npc(&config.starting_innkeeper) {
            if !npc.is_innkeeper {
                errors.push(ValidationError::InvalidStartingInnkeeper {
                    innkeeper_id: config.starting_innkeeper.clone(),
                    reason: format!(
                        "NPC '{}' exists but is not marked as is_innkeeper=true",
                        npc.name
                    ),
                });
            }
        } else {
            errors.push(ValidationError::InvalidStartingInnkeeper {
                innkeeper_id: config.starting_innkeeper.clone(),
                reason: "NPC not found in database".to_string(),
            });
        }

        errors
    }

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
                    // Validate NPC exists in database or placements
                    let npc_exists = self.db.npcs.has_npc(npc_id)
                        || map.npc_placements.iter().any(|p| &p.npc_id == npc_id);

                    if !npc_exists {
                        errors.push(ValidationError::BalanceWarning {
                            severity: Severity::Error,
                            message: format!(
                                "Map {} has NPC dialogue event for non-existent NPC '{}' at ({}, {})",
                                map.id, npc_id, pos.x, pos.y
                            ),
                        });
                    }
                }
                crate::domain::world::MapEvent::RecruitableCharacter { character_id, .. } => {
                    // Validate character exists in database
                    if self.db.characters.get_character(character_id).is_none() {
                        errors.push(ValidationError::BalanceWarning {
                            severity: Severity::Error,
                            message: format!(
                                "Map {} has recruitable character event for non-existent character '{}' at ({}, {})",
                                map.id, character_id, pos.x, pos.y
                            ),
                        });
                    }
                }
                crate::domain::world::MapEvent::EnterInn { innkeeper_id, .. } => {
                    // Validate `innkeeper_id` references a valid innkeeper NPC (non-empty and exists)
                    let innkeeper_id_trimmed = innkeeper_id.trim();
                    if innkeeper_id_trimmed.is_empty() {
                        errors.push(ValidationError::BalanceWarning {
                            severity: Severity::Error,
                            message: format!(
                                "Map {} has EnterInn event with empty innkeeper_id at ({}, {}).",
                                map.id, pos.x, pos.y
                            ),
                        });
                    } else {
                        // Check if NPC exists in the database or as a placement on this map
                        let npc_exists_in_db = self.db.npcs.has_npc(innkeeper_id_trimmed);
                        let npc_placed_on_map = map
                            .npc_placements
                            .iter()
                            .any(|p| p.npc_id == innkeeper_id_trimmed);

                        if !npc_exists_in_db && !npc_placed_on_map {
                            errors.push(ValidationError::BalanceWarning {
                                severity: Severity::Error,
                                message: format!(
                                    "Map {} has EnterInn event referencing non-existent NPC '{}' at ({}, {}).",
                                    map.id, innkeeper_id, pos.x, pos.y
                                ),
                            });
                        } else if npc_exists_in_db {
                            // If NPC is present in DB, ensure it's marked as an innkeeper
                            if let Some(npc_def) = self.db.npcs.get_npc(innkeeper_id_trimmed) {
                                if !npc_def.is_innkeeper {
                                    errors.push(ValidationError::BalanceWarning {
                                        severity: Severity::Error,
                                        message: format!(
                                            "Map {} has EnterInn event referencing NPC '{}' which is not an innkeeper at ({}, {}). Set `is_innkeeper: true` on the NPC definition.",
                                            map.id, innkeeper_id, pos.x, pos.y
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Validate NPC placements
        for placement in &map.npc_placements {
            // Check if placement position is valid
            if !map.is_valid_position(placement.position) {
                errors.push(ValidationError::BalanceWarning {
                    severity: Severity::Error,
                    message: format!(
                        "Map {} has NPC placement '{}' at invalid position ({}, {})",
                        map.id, placement.npc_id, placement.position.x, placement.position.y
                    ),
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

        if map.npc_placements.len() > 100 {
            errors.push(ValidationError::BalanceWarning {
                severity: Severity::Warning,
                message: format!(
                    "Map {} has {} NPC placements, which may cause performance issues",
                    map.id,
                    map.npc_placements.len()
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
    fn test_validate_map_enterinn_nonexistent_npc() {
        // An EnterInn referencing a non-existent NPC should produce a validation error
        let db = ContentDatabase::new();

        // Build a map with an EnterInn event referencing "nonexistent_inn"
        let mut map =
            crate::domain::world::Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let pos = crate::domain::types::Position::new(1, 1);
        map.add_event(
            pos,
            crate::domain::world::MapEvent::EnterInn {
                name: "Cozy Inn Entrance".to_string(),
                description: "A cozy entrance".to_string(),
                innkeeper_id: "nonexistent_inn".to_string(),
            },
        );

        let validator = Validator::new(&db);
        let errors = validator.validate_map(&map).expect("Validation failed");

        assert!(
            errors.iter().any(|e| matches!(e, ValidationError::BalanceWarning { severity: Severity::Error, message } if message.contains("non-existent NPC") || message.contains("referencing non-existent NPC"))),
            "Expected an error about a non-existent NPC for EnterInn, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_map_enterinn_not_innkeeper() {
        // An EnterInn referencing an NPC that exists but isn't an innkeeper should produce an error
        let mut db = ContentDatabase::new();

        // Add an NPC that is NOT an innkeeper
        let npc = crate::domain::world::npc::NpcDefinition::new(
            "not_inn".to_string(),
            "Not Inn".to_string(),
            "portrait.png".to_string(),
        );
        db.npcs.add_npc(npc).expect("Failed to add NPC");

        // Build a map with EnterInn referencing that NPC
        let mut map =
            crate::domain::world::Map::new(2, "Test2".to_string(), "Desc".to_string(), 10, 10);
        let pos = crate::domain::types::Position::new(2, 2);
        map.add_event(
            pos,
            crate::domain::world::MapEvent::EnterInn {
                name: "Fake Inn".to_string(),
                description: "This NPC isn't an innkeeper".to_string(),
                innkeeper_id: "not_inn".to_string(),
            },
        );

        let validator = Validator::new(&db);
        let errors = validator.validate_map(&map).expect("Validation failed");

        assert!(
            errors.iter().any(|e| matches!(e, ValidationError::BalanceWarning { severity: Severity::Error, message } if message.contains("not an innkeeper") || message.contains("which is not an innkeeper"))),
            "Expected an error about NPC not being an innkeeper, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_map_enterinn_valid() {
        // An EnterInn referencing a valid innkeeper NPC should produce no errors
        let mut db = ContentDatabase::new();

        // Add an NPC that IS an innkeeper
        let innkeeper = crate::domain::world::npc::NpcDefinition::innkeeper(
            "inn_good".to_string(),
            "Good Inn".to_string(),
            "portrait.png".to_string(),
        );
        db.npcs.add_npc(innkeeper).expect("Failed to add NPC");

        // Build a map with EnterInn referencing that innkeeper
        let mut map =
            crate::domain::world::Map::new(3, "Test3".to_string(), "Desc".to_string(), 10, 10);
        let pos = crate::domain::types::Position::new(3, 3);
        map.add_event(
            pos,
            crate::domain::world::MapEvent::EnterInn {
                name: "Good Inn Entrance".to_string(),
                description: "A proper inn entrance".to_string(),
                innkeeper_id: "inn_good".to_string(),
            },
        );

        let validator = Validator::new(&db);
        let errors = validator.validate_map(&map).expect("Validation failed");

        assert!(
            errors.is_empty(),
            "Expected no validation errors for a valid innkeeper reference, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_starting_innkeeper_missing() {
        // If the campaign's starting_innkeeper references an NPC that does not exist,
        // Validator::validate_campaign_config should return an InvalidStartingInnkeeper error.
        let db = ContentDatabase::new();
        let validator = Validator::new(&db);

        let config = crate::sdk::campaign_loader::CampaignConfig {
            starting_map: 1,
            starting_position: crate::domain::types::Position::new(0, 0),
            starting_direction: crate::domain::types::Direction::North,
            starting_gold: 100,
            starting_food: 50,
            starting_innkeeper: "missing_inn".to_string(),
            max_party_size: 6,
            max_roster_size: 20,
            difficulty: crate::sdk::campaign_loader::Difficulty::Normal,
            permadeath: false,
            allow_multiclassing: false,
            starting_level: 1,
            max_level: 20,
        };

        let errors = validator.validate_campaign_config(&config);
        assert!(
            errors.iter().any(|e| matches!(
                e,
                ValidationError::InvalidStartingInnkeeper { innkeeper_id, reason }
                if innkeeper_id == "missing_inn" && reason == "NPC not found in database"
            )),
            "Expected InvalidStartingInnkeeper for missing NPC, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_starting_innkeeper_not_innkeeper() {
        // If the campaign's starting_innkeeper references an NPC that exists but
        // isn't flagged as an innkeeper, the validator should return an error.
        let mut db = ContentDatabase::new();

        // Add an NPC that is NOT an innkeeper
        let npc = crate::domain::world::npc::NpcDefinition::new(
            "not_inn".to_string(),
            "Not Inn".to_string(),
            "portrait.png".to_string(),
        );
        db.npcs.add_npc(npc).expect("Failed to add NPC");

        let validator = Validator::new(&db);

        let config = crate::sdk::campaign_loader::CampaignConfig {
            starting_map: 1,
            starting_position: crate::domain::types::Position::new(0, 0),
            starting_direction: crate::domain::types::Direction::North,
            starting_gold: 100,
            starting_food: 50,
            starting_innkeeper: "not_inn".to_string(),
            max_party_size: 6,
            max_roster_size: 20,
            difficulty: crate::sdk::campaign_loader::Difficulty::Normal,
            permadeath: false,
            allow_multiclassing: false,
            starting_level: 1,
            max_level: 20,
        };

        let errors = validator.validate_campaign_config(&config);
        assert!(
            errors.iter().any(|e| matches!(
                e,
                ValidationError::InvalidStartingInnkeeper { innkeeper_id, reason }
                if innkeeper_id == "not_inn" && reason.contains("not marked as is_innkeeper")
            )),
            "Expected InvalidStartingInnkeeper for existing non-innkeeper NPC, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_starting_innkeeper_valid() {
        // A valid innkeeper reference should produce no validation errors.
        let mut db = ContentDatabase::new();

        let inn = crate::domain::world::npc::NpcDefinition::innkeeper(
            "inn_good".to_string(),
            "Good Inn".to_string(),
            "portrait.png".to_string(),
        );
        db.npcs.add_npc(inn).expect("Failed to add NPC");

        let validator = Validator::new(&db);

        let config = crate::sdk::campaign_loader::CampaignConfig {
            starting_map: 1,
            starting_position: crate::domain::types::Position::new(0, 0),
            starting_direction: crate::domain::types::Direction::North,
            starting_gold: 100,
            starting_food: 50,
            starting_innkeeper: "inn_good".to_string(),
            max_party_size: 6,
            max_roster_size: 20,
            difficulty: crate::sdk::campaign_loader::Difficulty::Normal,
            permadeath: false,
            allow_multiclassing: false,
            starting_level: 1,
            max_level: 20,
        };

        let errors = validator.validate_campaign_config(&config);
        assert!(
            errors.is_empty(),
            "Expected no validation errors for a valid starting_innkeeper, got: {:?}",
            errors
        );
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

    #[test]
    fn test_validator_party_size_limit_valid() {
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

        // Add exactly 6 characters with starts_in_party = true (at limit)
        for i in 0..6 {
            let mut char = CharacterDefinition::new(
                format!("char_{}", i),
                format!("Character {}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            char.is_premade = true;
            char.starts_in_party = true;
            db.characters.add_character(char).unwrap();
        }

        let validator = Validator::new(&db);
        let errors = validator.validate_all().unwrap();

        // Should NOT have TooManyStartingPartyMembers error
        let party_size_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e, ValidationError::TooManyStartingPartyMembers { .. }))
            .collect();
        assert_eq!(
            party_size_errors.len(),
            0,
            "Should not have party size error with exactly 6 starting members"
        );
    }

    #[test]
    fn test_validator_party_size_limit_exceeded() {
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

        // Add 7 characters with starts_in_party = true (exceeds limit)
        for i in 0..7 {
            let mut char = CharacterDefinition::new(
                format!("char_{}", i),
                format!("Character {}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            char.is_premade = true;
            char.starts_in_party = true;
            db.characters.add_character(char).unwrap();
        }

        let validator = Validator::new(&db);
        let errors = validator.validate_all().unwrap();

        // Should have TooManyStartingPartyMembers error
        let party_size_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e, ValidationError::TooManyStartingPartyMembers { .. }))
            .collect();
        assert_eq!(
            party_size_errors.len(),
            1,
            "Should detect party size violation"
        );

        // Verify error details
        if let Some(ValidationError::TooManyStartingPartyMembers { count, max }) =
            party_size_errors.first()
        {
            assert_eq!(*count, 7, "Should report 7 starting party members");
            assert_eq!(*max, 6, "Should report max of 6");
        } else {
            panic!("Expected TooManyStartingPartyMembers error");
        }
    }

    #[test]
    fn test_validator_party_size_ignores_non_starting_characters() {
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

        // Add 3 characters with starts_in_party = true
        for i in 0..3 {
            let mut char = CharacterDefinition::new(
                format!("starting_char_{}", i),
                format!("Starting Character {}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            char.is_premade = true;
            char.starts_in_party = true;
            db.characters.add_character(char).unwrap();
        }

        // Add 10 more characters with starts_in_party = false
        for i in 0..10 {
            let mut char = CharacterDefinition::new(
                format!("recruit_char_{}", i),
                format!("Recruitable Character {}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            char.is_premade = true;
            char.starts_in_party = false; // NOT starting in party
            db.characters.add_character(char).unwrap();
        }

        let validator = Validator::new(&db);
        let errors = validator.validate_all().unwrap();

        // Should NOT have TooManyStartingPartyMembers error (only 3 starting)
        let party_size_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e, ValidationError::TooManyStartingPartyMembers { .. }))
            .collect();
        assert_eq!(
            party_size_errors.len(),
            0,
            "Should only count characters with starts_in_party=true"
        );
    }

    #[test]
    fn test_validation_error_party_size_severity() {
        let error = ValidationError::TooManyStartingPartyMembers { count: 7, max: 6 };

        assert_eq!(error.severity(), Severity::Error);
        assert!(error.is_error());
        assert!(!error.is_warning());
        assert!(!error.is_info());
    }

    #[test]
    fn test_validation_error_party_size_display() {
        let error = ValidationError::TooManyStartingPartyMembers { count: 8, max: 6 };

        let message = error.to_string();
        assert!(message.contains("8"));
        assert!(message.contains("6"));
        assert!(message.contains("starts_in_party"));
    }
}
