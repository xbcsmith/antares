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
//! See `docs/explanation/sdk_implementation_plan.md` for specifications.
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
use crate::sdk::creature_validation::validate_creature_topology;
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
#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    /// Innkeeper definition does not include a dialogue id
    ///
    /// All NPCs that act as innkeepers MUST have a `dialogue_id` configured
    /// so they can present interactive options (such as party management).
    #[error("Innkeeper '{innkeeper_id}' must have dialogue_id configured")]
    InnkeeperMissingDialogue {
        /// The innkeeper ID missing a dialogue_id
        innkeeper_id: String,
    },

    /// Creature name is empty
    #[error("Creature ID {creature_id} has empty name")]
    CreatureEmptyName { creature_id: u32 },

    /// Creature has invalid scale
    #[error("Creature '{name}' (ID {creature_id}) has invalid scale: {scale}")]
    CreatureInvalidScale {
        creature_id: u32,
        name: String,
        scale: f32,
    },

    /// Creature has no meshes
    #[error("Creature '{name}' (ID {creature_id}) has no meshes")]
    CreatureNoMeshes { creature_id: u32, name: String },

    /// Creature mesh topology error
    #[error("Creature '{name}' (ID {creature_id}) mesh {mesh_index}: {error}")]
    CreatureMeshTopology {
        creature_id: u32,
        name: String,
        mesh_index: usize,
        error: String,
    },

    /// Creature has duplicate mesh names
    #[error("Creature '{name}' (ID {creature_id}) has duplicate mesh names")]
    CreatureDuplicateMeshNames { creature_id: u32, name: String },

    /// Item referenced in a merchant stock template does not exist in the item database
    #[error("NPC '{context}' stock template references missing item ID {item_id}")]
    MissingStockTemplateItem {
        /// The NPC ID whose stock template contains the bad reference
        context: String,
        /// The item ID that could not be found in the item database
        item_id: crate::domain::types::ItemId,
    },

    /// A service catalog entry uses an unrecognised service ID
    ///
    /// Emitted as a `Warning` (not `Error`) to allow custom service IDs for
    /// future extensibility.
    #[error("NPC '{context}' service catalog contains unknown service ID '{service_id}'")]
    InvalidServiceId {
        /// The NPC ID whose service catalog contains the unrecognised entry
        context: String,
        /// The unrecognised service ID string
        service_id: String,
    },

    /// An item's auto-generated mesh descriptor produced an invalid
    /// [`CreatureDefinition`] when validated by [`CreatureDefinition::validate`].
    ///
    /// This indicates a data inconsistency that would prevent the item from
    /// being rendered as a dropped mesh in the game world.
    #[error("Item ID {item_id} has invalid mesh descriptor: {message}")]
    ItemMeshDescriptorInvalid {
        /// The item ID whose mesh descriptor failed validation
        item_id: crate::domain::types::ItemId,
        /// Human-readable description of why the descriptor is invalid
        message: String,
    },

    /// Item placed in `equipment.helmet` is not classified as `ArmorClassification::Helmet`
    ///
    /// Emitted when a character definition's `starting_equipment.helmet` references
    /// an item whose `ArmorData.classification` is not `Helmet`.  This catches
    /// copy-paste errors such as putting a pair of boots in the helmet slot.
    #[error(
        "Character '{character_id}' equipment.helmet references item {item_id} \
         which is not classified as Helmet (got {actual_classification})"
    )]
    HelmetSlotTypeMismatch {
        /// The character definition ID whose equipment was validated
        character_id: String,
        /// The item ID that was found in the helmet slot
        item_id: crate::domain::types::ItemId,
        /// String representation of the item's actual ArmorClassification
        actual_classification: String,
    },

    /// Item placed in `equipment.boots` is not classified as `ArmorClassification::Boots`
    ///
    /// Emitted when a character definition's `starting_equipment.boots` references
    /// an item whose `ArmorData.classification` is not `Boots`.  This catches
    /// copy-paste errors such as putting a helmet in the boots slot.
    #[error(
        "Character '{character_id}' equipment.boots references item {item_id} \
         which is not classified as Boots (got {actual_classification})"
    )]
    BootsSlotTypeMismatch {
        /// The character definition ID whose equipment was validated
        character_id: String,
        /// The item ID that was found in the boots slot
        item_id: crate::domain::types::ItemId,
        /// String representation of the item's actual ArmorClassification
        actual_classification: String,
    },

    /// A locked object's `key_item_id` references an item that is not a key item
    /// (i.e. `ItemType::Quest(QuestData { is_key_item: true })`).
    ///
    /// Emitted when the item exists but `is_key_item` is `false`.
    #[error(
        "Map {map_id} locked object '{lock_id}' key_item_id {item_id} \
         is not a key item (is_key_item must be true)"
    )]
    LockedObjectKeyNotKeyItem {
        /// Map where the locked event is located
        map_id: crate::domain::types::MapId,
        /// Lock identifier from the event
        lock_id: String,
        /// The item ID that was found but is not a key item
        item_id: crate::domain::types::ItemId,
    },

    /// Two or more locked objects on the same map share the same `lock_id`.
    ///
    /// Each locked door / container on a map must have a unique `lock_id` because
    /// `Map::lock_states` uses `lock_id` as the HashMap key.
    #[error("Map {map_id} has duplicate lock_id '{lock_id}'")]
    DuplicateLockId {
        /// Map where the duplicate was found
        map_id: crate::domain::types::MapId,
        /// The duplicated lock identifier
        lock_id: String,
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
            | ValidationError::InvalidStartingInnkeeper { .. }
            | ValidationError::InnkeeperMissingDialogue { .. }
            | ValidationError::CreatureEmptyName { .. }
            | ValidationError::CreatureInvalidScale { .. }
            | ValidationError::CreatureNoMeshes { .. }
            | ValidationError::CreatureMeshTopology { .. }
            | ValidationError::CreatureDuplicateMeshNames { .. }
            | ValidationError::MissingStockTemplateItem { .. }
            | ValidationError::ItemMeshDescriptorInvalid { .. }
            | ValidationError::HelmetSlotTypeMismatch { .. }
            | ValidationError::BootsSlotTypeMismatch { .. }
            | ValidationError::LockedObjectKeyNotKeyItem { .. }
            | ValidationError::DuplicateLockId { .. } => Severity::Error,

            ValidationError::DisconnectedMap { .. } | ValidationError::InvalidServiceId { .. } => {
                Severity::Warning
            }

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

/* test moved to existing tests module (avoids duplicate `mod tests` definitions) */

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

        // Validate innkeepers have dialogue configured
        errors.extend(self.validate_innkeepers());

        // Validate creatures
        errors.extend(self.validate_creatures());

        // Validate merchant stock template references
        errors.extend(self.validate_merchant_stock());

        // Validate service catalog entries
        errors.extend(self.validate_service_catalogs());

        // Validate item mesh descriptors
        errors.extend(self.validate_item_mesh_descriptors());

        Ok(errors)
    }

    /// Validates all cross-references in the content
    ///
    /// Checks that all ID references (class IDs, race IDs, item IDs, etc.)
    /// point to existing content.
    fn validate_references(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Validate monster loot references: each loot item_id must exist in the item database
        for monster_id in self.db.monsters.all_monsters() {
            if let Some(monster) = self.db.monsters.get_monster(monster_id) {
                for &(_probability, item_id) in &monster.loot.items {
                    if !self.db.items.has_item(&(item_id as ItemId)) {
                        errors.push(ValidationError::MissingItem {
                            context: format!(
                                "Monster '{}' (id={}) loot table",
                                monster.name, monster_id
                            ),
                            item_id: item_id as ItemId,
                        });
                    }
                }
            }
        }

        // Validate spell references: applied_conditions must exist in the condition database
        for spell_id in self.db.spells.all_spells() {
            if let Some(spell) = self.db.spells.get_spell(spell_id) {
                for condition_id in &spell.applied_conditions {
                    if !self.db.conditions.has_condition(condition_id) {
                        errors.push(ValidationError::BalanceWarning {
                            severity: Severity::Warning,
                            message: format!(
                                "Spell '{}' (id={}) references unknown condition '{}'",
                                spell.name, spell_id, condition_id
                            ),
                        });
                    }
                }
            }
        }

        // Validate map cross-references using the existing validate_map method
        for map_id in self.db.maps.all_maps() {
            if let Some(map) = self.db.maps.get_map(map_id) {
                match self.validate_map(map) {
                    Ok(map_errors) => errors.extend(map_errors),
                    Err(e) => {
                        errors.push(ValidationError::BalanceWarning {
                            severity: Severity::Error,
                            message: format!("Failed to validate map {}: {}", map_id, e),
                        });
                    }
                }
            }
        }

        // Validate character definition cross-references
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

            // Validate helmet slot: item must have ArmorClassification::Helmet
            if let Some(helmet_id) = character.starting_equipment.helmet {
                if let Some(item) = self.db.items.get_item(helmet_id) {
                    use crate::domain::items::types::{ArmorClassification, ItemType};
                    match &item.item_type {
                        ItemType::Armor(data)
                            if data.classification != ArmorClassification::Helmet =>
                        {
                            errors.push(ValidationError::HelmetSlotTypeMismatch {
                                character_id: character.id.clone(),
                                item_id: helmet_id,
                                actual_classification: format!("{:?}", data.classification),
                            });
                        }
                        ItemType::Armor(_) => {} // correct classification — ok
                        _ => {
                            errors.push(ValidationError::HelmetSlotTypeMismatch {
                                character_id: character.id.clone(),
                                item_id: helmet_id,
                                actual_classification: format!("{:?}", item.item_type),
                            });
                        }
                    }
                }
            }

            // Validate boots slot: item must have ArmorClassification::Boots
            if let Some(boots_id) = character.starting_equipment.boots {
                if let Some(item) = self.db.items.get_item(boots_id) {
                    use crate::domain::items::types::{ArmorClassification, ItemType};
                    match &item.item_type {
                        ItemType::Armor(data)
                            if data.classification != ArmorClassification::Boots =>
                        {
                            errors.push(ValidationError::BootsSlotTypeMismatch {
                                character_id: character.id.clone(),
                                item_id: boots_id,
                                actual_classification: format!("{:?}", data.classification),
                            });
                        }
                        ItemType::Armor(_) => {} // correct classification — ok
                        _ => {
                            errors.push(ValidationError::BootsSlotTypeMismatch {
                                character_id: character.id.clone(),
                                item_id: boots_id,
                                actual_classification: format!("{:?}", item.item_type),
                            });
                        }
                    }
                }
            }
        }

        errors
    }

    /// Validates that all NPCs marked as `is_innkeeper` have a dialogue configured.
    ///
    /// This is an error-level validation because innkeeper NPCs are expected to
    /// present dialogue options (for example, to open the party management UI).
    fn validate_innkeepers(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for npc_id in self.db.npcs.all_npcs() {
            if let Some(npc) = self.db.npcs.get_npc(&npc_id) {
                if npc.is_innkeeper && npc.dialogue_id.is_none() {
                    errors.push(ValidationError::InnkeeperMissingDialogue {
                        innkeeper_id: npc_id.clone(),
                    });
                }
            }
        }

        errors
    }

    /// Validates merchant stock templates referenced by NPC definitions.
    ///
    /// For every NPC marked `is_merchant == true`:
    /// - If `stock_template` is `Some(id)`, verifies the template exists in
    ///   `db.npc_stock_templates`.
    /// - For each entry in the found template, verifies the `item_id` exists in
    ///   `db.items`.
    ///
    /// # Returns
    ///
    /// A `Vec<ValidationError>` containing any errors found.  An empty vector
    /// means all merchant stock references are valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::database::ContentDatabase;
    /// use antares::sdk::validation::Validator;
    ///
    /// let db = ContentDatabase::new();
    /// let validator = Validator::new(&db);
    /// let errors = validator.validate_merchant_stock();
    /// assert!(errors.is_empty());
    /// ```
    pub fn validate_merchant_stock(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for npc_id in self.db.npcs.all_npcs() {
            let npc = match self.db.npcs.get_npc(&npc_id) {
                Some(n) => n,
                None => continue,
            };

            if !npc.is_merchant {
                continue;
            }

            let template_id = match &npc.stock_template {
                Some(id) => id,
                None => continue,
            };

            // Check that the referenced template exists
            let template = match self.db.npc_stock_templates.get(template_id) {
                Some(t) => t,
                None => {
                    errors.push(ValidationError::MissingItem {
                        context: format!("NPC '{}' stock_template '{}'", npc_id, template_id),
                        item_id: 0,
                    });
                    continue;
                }
            };

            // Validate every item in the template exists
            for entry in &template.entries {
                if !self.db.items.has_item(&entry.item_id) {
                    errors.push(ValidationError::MissingStockTemplateItem {
                        context: npc_id.clone(),
                        item_id: entry.item_id,
                    });
                }
            }
        }

        errors
    }

    /// Known built-in service IDs recognised by the game engine.
    ///
    /// Service IDs outside this list are emitted as `Warning`-level
    /// `InvalidServiceId` errors to allow custom service IDs for future
    /// extensibility without breaking existing campaigns.
    const KNOWN_SERVICE_IDS: &'static [&'static str] = &[
        "heal_all",
        "heal",
        "restore_sp",
        "cure_poison",
        "cure_disease",
        "cure_all",
        "resurrect",
        "rest",
    ];

    /// Validates service catalog entries for all NPCs that have one.
    ///
    /// For every NPC whose `service_catalog` is `Some`, each `ServiceEntry`
    /// is checked against [`Validator::KNOWN_SERVICE_IDS`].  Entries with
    /// unknown service IDs are emitted as `Warning`-severity
    /// [`ValidationError::InvalidServiceId`] to allow forward-compatible
    /// custom service IDs.
    ///
    /// # Returns
    ///
    /// A `Vec<ValidationError>` containing any warnings found.  An empty
    /// vector means all service IDs are known built-in IDs.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::database::ContentDatabase;
    /// use antares::sdk::validation::Validator;
    ///
    /// let db = ContentDatabase::new();
    /// let validator = Validator::new(&db);
    /// let warnings = validator.validate_service_catalogs();
    /// assert!(warnings.is_empty());
    /// ```
    pub fn validate_service_catalogs(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for npc_id in self.db.npcs.all_npcs() {
            let npc = match self.db.npcs.get_npc(&npc_id) {
                Some(n) => n,
                None => continue,
            };

            let catalog = match &npc.service_catalog {
                Some(c) => c,
                None => continue,
            };

            for entry in &catalog.services {
                if !Self::KNOWN_SERVICE_IDS.contains(&entry.service_id.as_str()) {
                    errors.push(ValidationError::InvalidServiceId {
                        context: npc_id.clone(),
                        service_id: entry.service_id.clone(),
                    });
                }
            }
        }

        errors
    }

    /// Validates creature definitions
    ///
    /// Checks that:
    /// - Creature name is not empty
    /// - Creature scale is positive
    /// - Creature has at least one mesh
    /// - Mesh topology is valid (no degenerate triangles, consistent winding, etc.)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::database::ContentDatabase;
    /// use antares::sdk::validation::Validator;
    ///
    /// let db = ContentDatabase::new();
    /// let validator = Validator::new(&db);
    /// let errors = validator.validate_all();
    /// ```
    /// Validates that every item in the database produces a well-formed
    /// procedural mesh descriptor.
    ///
    /// Calls [`ItemDatabase::validate_mesh_descriptors`] and converts any
    /// resulting error into a [`ValidationError::ItemMeshDescriptorInvalid`].
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::database::ContentDatabase;
    /// use antares::sdk::validation::Validator;
    ///
    /// let db = ContentDatabase::new();
    /// let validator = Validator::new(&db);
    /// let errors = validator.validate_item_mesh_descriptors();
    /// // Empty database has no items to fail validation
    /// assert!(errors.is_empty());
    /// ```
    pub fn validate_item_mesh_descriptors(&self) -> Vec<ValidationError> {
        use crate::domain::items::database::ItemDatabaseError;

        match self.db.items.validate_mesh_descriptors() {
            Ok(()) => Vec::new(),
            Err(ItemDatabaseError::InvalidMeshDescriptor { item_id, message }) => {
                vec![ValidationError::ItemMeshDescriptorInvalid { item_id, message }]
            }
            Err(_) => Vec::new(),
        }
    }

    fn validate_creatures(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for creature in self.db.creatures.all_creatures() {
            // Check name not empty
            if creature.name.trim().is_empty() {
                errors.push(ValidationError::CreatureEmptyName {
                    creature_id: creature.id,
                });
            }

            // Check scale is positive
            if creature.scale <= 0.0 {
                errors.push(ValidationError::CreatureInvalidScale {
                    creature_id: creature.id,
                    name: creature.name.clone(),
                    scale: creature.scale,
                });
            }

            // Check has at least one mesh
            if creature.meshes.is_empty() {
                errors.push(ValidationError::CreatureNoMeshes {
                    creature_id: creature.id,
                    name: creature.name.clone(),
                });
            }

            // Validate mesh topology
            if let Err(topology_error) = validate_creature_topology(creature) {
                errors.push(ValidationError::CreatureMeshTopology {
                    creature_id: creature.id,
                    name: creature.name.clone(),
                    mesh_index: 0, // Error doesn't specify which mesh, use 0
                    error: topology_error.to_string(),
                });
            }
        }

        errors
    }

    /* tests moved to top-level scope */

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
    ///     starting_time: antares::domain::types::GameTime::new(1, 8, 0),
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
        let mut errors = Vec::new();

        let all_maps = self.db.maps.all_maps();
        if all_maps.is_empty() {
            return errors;
        }

        // Build adjacency list: map_id -> set of destination map_ids reachable via Teleport
        let mut adjacency: std::collections::HashMap<MapId, std::collections::HashSet<MapId>> =
            std::collections::HashMap::new();

        for &map_id in &all_maps {
            adjacency.entry(map_id).or_default();

            if let Some(map) = self.db.maps.get_map(map_id) {
                for event in map.events.values() {
                    if let crate::domain::world::MapEvent::Teleport { map_id: dest, .. } = event {
                        adjacency.entry(map_id).or_default().insert(*dest);
                    }
                }
            }
        }

        // BFS from the smallest map ID (assumed starting map)
        let start_map = *all_maps.iter().min().unwrap_or(&0);
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        visited.insert(start_map);
        queue.push_back(start_map);

        while let Some(current) = queue.pop_front() {
            if let Some(neighbors) = adjacency.get(&current) {
                for &neighbor in neighbors {
                    if visited.insert(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        // Report unreachable maps
        for &map_id in &all_maps {
            if !visited.contains(&map_id) {
                errors.push(ValidationError::DisconnectedMap { map_id });
            }
        }

        // Report maps with no teleport exits (dead ends)
        for &map_id in &all_maps {
            if let Some(exits) = adjacency.get(&map_id) {
                if exits.is_empty() {
                    errors.push(ValidationError::BalanceWarning {
                        severity: Severity::Warning,
                        message: format!("Map {} has no teleport exits (dead end)", map_id),
                    });
                }
            }
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
                crate::domain::world::MapEvent::Furniture { .. } => {
                    // Furniture events are always valid - they spawn procedurally
                }
                crate::domain::world::MapEvent::Container { id, items, .. } => {
                    // Validate that the container id is non-empty
                    if id.trim().is_empty() {
                        errors.push(ValidationError::BalanceWarning {
                            severity: Severity::Error,
                            message: format!(
                                "Map {} has Container event with empty id at ({}, {})",
                                map.id, pos.x, pos.y
                            ),
                        });
                    }
                    // Validate each item id referenced in the container
                    for slot in items {
                        if !self.db.items.has_item(&(slot.item_id as ItemId)) {
                            errors.push(ValidationError::MissingItem {
                                context: format!(
                                    "Map {} container '{}' at ({}, {})",
                                    map.id, id, pos.x, pos.y
                                ),
                                item_id: slot.item_id as ItemId,
                            });
                        }
                    }
                }
                crate::domain::world::MapEvent::DroppedItem { item_id, .. } => {
                    // Validate that the item exists in the database
                    if !self.db.items.has_item(item_id) {
                        errors.push(ValidationError::MissingItem {
                            context: format!(
                                "Map {} DroppedItem event at ({}, {})",
                                map.id, pos.x, pos.y
                            ),
                            item_id: *item_id,
                        });
                    }
                }
                crate::domain::world::MapEvent::LockedDoor {
                    lock_id,
                    key_item_id,
                    ..
                } => {
                    // Validate that the lock_id is non-empty
                    if lock_id.trim().is_empty() {
                        errors.push(ValidationError::BalanceWarning {
                            severity: Severity::Error,
                            message: format!(
                                "Map {} has LockedDoor event with empty lock_id at ({}, {})",
                                map.id, pos.x, pos.y
                            ),
                        });
                    }
                    // Validate key item exists if one is specified
                    if let Some(kid) = key_item_id {
                        if !self.db.items.has_item(kid) {
                            errors.push(ValidationError::MissingItem {
                                context: format!(
                                    "Map {} LockedDoor '{}' key_item_id at ({}, {})",
                                    map.id, lock_id, pos.x, pos.y
                                ),
                                item_id: *kid,
                            });
                        }
                    }
                    // Check the item exists AND is a key item
                    if let Some(kid) = key_item_id {
                        if let Some(item) = self.db.items.get_item(*kid) {
                            let is_key = matches!(
                                &item.item_type,
                                crate::domain::items::ItemType::Quest(q) if q.is_key_item
                            );
                            if !is_key {
                                errors.push(ValidationError::LockedObjectKeyNotKeyItem {
                                    map_id: map.id,
                                    lock_id: lock_id.clone(),
                                    item_id: *kid,
                                });
                            }
                        }
                        // Note: MissingItem is already emitted above when item not found
                    }
                }
                crate::domain::world::MapEvent::LockedContainer {
                    lock_id,
                    key_item_id,
                    ..
                } => {
                    // Validate that the lock_id is non-empty
                    if lock_id.trim().is_empty() {
                        errors.push(ValidationError::BalanceWarning {
                            severity: Severity::Error,
                            message: format!(
                                "Map {} has LockedContainer event with empty lock_id at ({}, {})",
                                map.id, pos.x, pos.y
                            ),
                        });
                    }
                    // Validate key item exists if one is specified
                    if let Some(kid) = key_item_id {
                        if !self.db.items.has_item(kid) {
                            errors.push(ValidationError::MissingItem {
                                context: format!(
                                    "Map {} LockedContainer '{}' key_item_id at ({}, {})",
                                    map.id, lock_id, pos.x, pos.y
                                ),
                                item_id: *kid,
                            });
                        }
                    }
                    // Check the item exists AND is a key item
                    if let Some(kid) = key_item_id {
                        if let Some(item) = self.db.items.get_item(*kid) {
                            let is_key = matches!(
                                &item.item_type,
                                crate::domain::items::ItemType::Quest(q) if q.is_key_item
                            );
                            if !is_key {
                                errors.push(ValidationError::LockedObjectKeyNotKeyItem {
                                    map_id: map.id,
                                    lock_id: lock_id.clone(),
                                    item_id: *kid,
                                });
                            }
                        }
                        // Note: MissingItem is already emitted above when item not found
                    }
                }
            }
        }

        // Check for duplicate lock_ids across all LockedDoor and LockedContainer events
        {
            let mut seen_lock_ids: std::collections::HashSet<&str> =
                std::collections::HashSet::new();
            let mut duplicate_ids: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for event in map.events.values() {
                let lock_id = match event {
                    crate::domain::world::MapEvent::LockedDoor { lock_id, .. } => {
                        Some(lock_id.as_str())
                    }
                    crate::domain::world::MapEvent::LockedContainer { lock_id, .. } => {
                        Some(lock_id.as_str())
                    }
                    _ => None,
                };
                if let Some(id) = lock_id {
                    if !seen_lock_ids.insert(id) {
                        duplicate_ids.insert(id.to_string());
                    }
                }
            }
            for dup_id in duplicate_ids {
                errors.push(ValidationError::DuplicateLockId {
                    map_id: map.id,
                    lock_id: dup_id,
                });
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
    }

    #[test]
    fn test_innkeeper_missing_dialogue_validation() {
        // Arrange: build a content DB with an innkeeper who has no dialogue_id
        let mut db = ContentDatabase::new();
        let npc = crate::domain::world::npc::NpcDefinition::innkeeper(
            "missing_dialogue",
            "Missing Dialogue Inn",
            "portrait",
        );
        db.npcs.add_npc(npc).unwrap();

        // Act: validate
        let validator = Validator::new(&db);
        let errors = validator.validate_all().expect("validation failed");

        // Assert: we found the InnkeeperMissingDialogue error for our NPC
        assert!(errors.iter().any(|e| matches!(
            e,
            ValidationError::InnkeeperMissingDialogue { innkeeper_id } if innkeeper_id == "missing_dialogue"
        )));
    }

    #[test]
    fn test_balance_warning_severity() {
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
            level_up_mode: crate::domain::campaign::LevelUpMode::Auto,
            base_xp: 1000,
            xp_multiplier: 1.5,
            starting_time: crate::domain::types::GameTime::new(1, 8, 0),
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
            level_up_mode: crate::domain::campaign::LevelUpMode::Auto,
            base_xp: 1000,
            xp_multiplier: 1.5,
            starting_time: crate::domain::types::GameTime::new(1, 8, 0),
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
            level_up_mode: crate::domain::campaign::LevelUpMode::Auto,
            base_xp: 1000,
            xp_multiplier: 1.5,
            starting_time: crate::domain::types::GameTime::new(1, 8, 0),
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

    // ========================================================================
    // Integration Testing - Edge Cases for Innkeeper Validation
    // ========================================================================

    #[test]
    fn test_innkeeper_with_dialogue_is_valid() {
        // Arrange: Innkeeper with dialogue_id should pass validation
        let mut db = ContentDatabase::new();

        let mut innkeeper = crate::domain::world::npc::NpcDefinition::innkeeper(
            "good_innkeeper",
            "Good Innkeeper",
            "portrait",
        );
        innkeeper.dialogue_id = Some(999); // Has dialogue
        db.npcs.add_npc(innkeeper).unwrap();

        // Act
        let validator = Validator::new(&db);
        let errors = validator.validate_all().expect("validation failed");

        // Assert: No InnkeeperMissingDialogue error
        assert!(!errors
            .iter()
            .any(|e| matches!(e, ValidationError::InnkeeperMissingDialogue { .. })));
    }

    #[test]
    fn test_multiple_innkeepers_missing_dialogue() {
        // Arrange: Multiple innkeepers without dialogue
        let mut db = ContentDatabase::new();

        for i in 1..=3 {
            let npc = crate::domain::world::npc::NpcDefinition::innkeeper(
                format!("inn_{}", i),
                format!("Inn {}", i),
                "portrait",
            );
            db.npcs.add_npc(npc).unwrap();
        }

        // Act
        let validator = Validator::new(&db);
        let errors = validator.validate_all().expect("validation failed");

        // Assert: Should have 3 errors for missing dialogues
        let missing_dialogue_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e, ValidationError::InnkeeperMissingDialogue { .. }))
            .collect();

        assert_eq!(missing_dialogue_errors.len(), 3);
    }

    #[test]
    fn test_innkeeper_missing_dialogue_error_severity() {
        // Verify InnkeeperMissingDialogue is Error severity
        let error = ValidationError::InnkeeperMissingDialogue {
            innkeeper_id: "test_inn".to_string(),
        };

        assert_eq!(error.severity(), Severity::Error);
        assert!(error.is_error());
        assert!(!error.is_warning());
        assert!(!error.is_info());
    }

    #[test]
    fn test_innkeeper_missing_dialogue_display() {
        // Verify error message format
        let error = ValidationError::InnkeeperMissingDialogue {
            innkeeper_id: "broken_inn".to_string(),
        };

        let message = error.to_string();
        assert!(message.contains("broken_inn"));
        assert!(message.contains("dialogue_id"));
    }

    #[test]
    fn test_non_innkeeper_without_dialogue_is_ok() {
        // Arrange: Regular NPC without dialogue should not trigger innkeeper validation
        let mut db = ContentDatabase::new();

        let regular_npc = crate::domain::world::npc::NpcDefinition::new(
            "merchant".to_string(),
            "Town Merchant".to_string(),
            "portrait.png".to_string(),
        );
        db.npcs.add_npc(regular_npc).unwrap();

        // Act
        let validator = Validator::new(&db);
        let errors = validator.validate_all().expect("validation failed");

        // Assert: No InnkeeperMissingDialogue error for non-innkeeper
        assert!(!errors.iter().any(|e| matches!(
            e,
            ValidationError::InnkeeperMissingDialogue { innkeeper_id } if innkeeper_id == "merchant"
        )));
    }

    #[test]
    fn test_innkeeper_edge_case_empty_id() {
        // Arrange: Innkeeper with empty string ID
        let mut db = ContentDatabase::new();

        let innkeeper =
            crate::domain::world::npc::NpcDefinition::innkeeper("", "No ID Inn", "portrait");
        db.npcs.add_npc(innkeeper).unwrap();

        // Act
        let validator = Validator::new(&db);
        let errors = validator.validate_all().expect("validation failed");

        // Assert: Should still validate (empty ID is valid string)
        assert!(errors.iter().any(|e| matches!(
            e,
            ValidationError::InnkeeperMissingDialogue { innkeeper_id } if innkeeper_id.is_empty()
        )));
    }

    #[test]
    fn test_innkeeper_edge_case_special_characters_in_id() {
        // Arrange: Innkeeper with special characters in ID
        let mut db = ContentDatabase::new();

        let special_ids = vec!["inn-town-1", "inn_underscore", "inn.dot", "Inn:Colon"];

        for id in &special_ids {
            let innkeeper =
                crate::domain::world::npc::NpcDefinition::innkeeper(*id, "Special Inn", "portrait");
            db.npcs.add_npc(innkeeper).unwrap();
        }

        // Act
        let validator = Validator::new(&db);
        let errors = validator.validate_all().expect("validation failed");

        // Assert: Each should have validation error
        for id in special_ids {
            assert!(errors.iter().any(|e| matches!(
                e,
                ValidationError::InnkeeperMissingDialogue { innkeeper_id } if innkeeper_id == id
            )));
        }
    }

    #[test]
    fn test_validate_innkeepers_performance_large_database() {
        // Test performance with many NPCs (mix of innkeepers and regular)
        let mut db = ContentDatabase::new();

        // Add 50 regular NPCs
        for i in 0..50 {
            let npc = crate::domain::world::npc::NpcDefinition::new(
                format!("npc_{}", i),
                format!("NPC {}", i),
                "portrait.png".to_string(),
            );
            db.npcs.add_npc(npc).unwrap();
        }

        // Add 10 innkeepers without dialogue
        for i in 0..10 {
            let innkeeper = crate::domain::world::npc::NpcDefinition::innkeeper(
                format!("innkeeper_{}", i),
                format!("Innkeeper {}", i),
                "portrait",
            );
            db.npcs.add_npc(innkeeper).unwrap();
        }

        // Act
        let validator = Validator::new(&db);
        let errors = validator.validate_all().expect("validation failed");

        // Assert: Should find exactly 10 innkeeper errors
        let innkeeper_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e, ValidationError::InnkeeperMissingDialogue { .. }))
            .collect();

        assert_eq!(innkeeper_errors.len(), 10);
    }

    #[test]
    fn test_innkeeper_validation_isolated() {
        // Test validate_innkeepers() method directly
        let mut db = ContentDatabase::new();

        let innkeeper = crate::domain::world::npc::NpcDefinition::innkeeper(
            "isolated_test",
            "Test Inn",
            "portrait",
        );
        db.npcs.add_npc(innkeeper).unwrap();

        let validator = Validator::new(&db);
        let errors = validator.validate_innkeepers();

        // Assert: Direct call should also catch the error
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::InnkeeperMissingDialogue { ref innkeeper_id } if innkeeper_id == "isolated_test"
        ));
    }

    // ===== Merchant Stock and Service Catalog Validation Tests =====

    #[test]
    fn test_validate_merchant_stock_valid() {
        // Arrange: merchant NPC with valid stock template referencing valid items
        use crate::domain::items::ItemDatabase;
        use crate::domain::world::npc_runtime::{MerchantStockTemplate, TemplateStockEntry};

        let mut db = ContentDatabase::new();

        // Build a minimal item database with item 1
        let item_ron = r#"[
            (
                id: 1,
                name: "Club",
                item_type: Weapon((
                    damage: (count: 1, sides: 3, bonus: 0),
                    bonus: 0,
                    hands_required: 1,
                    classification: Simple,
                )),
                base_cost: 1,
                sell_cost: 0,
                alignment_restriction: None,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
                icon_path: None,
                tags: [],
            ),
        ]"#;
        db.items = ItemDatabase::load_from_string(item_ron).expect("Failed to load test items");

        // Add a stock template referencing item 1
        db.npc_stock_templates.add(MerchantStockTemplate {
            id: "test_template".to_string(),
            entries: vec![TemplateStockEntry {
                item_id: 1,
                quantity: 3,
                override_price: None,
            }],
            magic_item_pool: vec![],
            magic_slot_count: 0,
            magic_refresh_days: 7,
            description: String::new(),
        });

        // Add a merchant NPC referencing the template
        let mut merchant = crate::domain::world::npc::NpcDefinition::merchant(
            "valid_merchant",
            "Valid Merchant",
            "portrait",
        );
        merchant.stock_template = Some("test_template".to_string());
        db.npcs.add_npc(merchant).unwrap();

        // Act
        let validator = Validator::new(&db);
        let errors = validator.validate_merchant_stock();

        // Assert: no errors for valid configuration
        assert!(
            errors.is_empty(),
            "Expected no errors but got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_merchant_stock_missing_template() {
        // Arrange: merchant NPC referencing a non-existent template
        let mut db = ContentDatabase::new();

        let mut merchant = crate::domain::world::npc::NpcDefinition::merchant(
            "bad_merchant",
            "Bad Merchant",
            "portrait",
        );
        merchant.stock_template = Some("nonexistent_template".to_string());
        db.npcs.add_npc(merchant).unwrap();

        // Act
        let validator = Validator::new(&db);
        let errors = validator.validate_merchant_stock();

        // Assert: at least one error for missing template
        assert!(
            !errors.is_empty(),
            "Expected at least one error for missing template"
        );
    }

    #[test]
    fn test_validate_merchant_stock_invalid_item() {
        // Arrange: stock template references item_id 999 which is not in ItemDatabase
        use crate::domain::world::npc_runtime::{MerchantStockTemplate, TemplateStockEntry};

        let mut db = ContentDatabase::new();

        // Template referencing an item that does not exist
        db.npc_stock_templates.add(MerchantStockTemplate {
            id: "bad_items_template".to_string(),
            entries: vec![TemplateStockEntry {
                item_id: 200,
                quantity: 2,
                override_price: None,
            }],
            magic_item_pool: vec![],
            magic_slot_count: 0,
            magic_refresh_days: 7,
            description: String::new(),
        });

        let mut merchant = crate::domain::world::npc::NpcDefinition::merchant(
            "item_merchant",
            "Item Merchant",
            "portrait",
        );
        merchant.stock_template = Some("bad_items_template".to_string());
        db.npcs.add_npc(merchant).unwrap();

        // Act
        let validator = Validator::new(&db);
        let errors = validator.validate_merchant_stock();

        // Assert: should have a MissingStockTemplateItem error
        let has_missing_stock_item = errors.iter().any(|e| {
            matches!(
                e,
                ValidationError::MissingStockTemplateItem {
                    context,
                    item_id: 200,
                } if context == "item_merchant"
            )
        });
        assert!(
            has_missing_stock_item,
            "Expected MissingStockTemplateItem error but got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_service_catalogs_known_ids() {
        // Arrange: NPC with service catalog using only known service IDs
        use crate::domain::inventory::{ServiceCatalog, ServiceEntry};

        let mut db = ContentDatabase::new();

        let mut priest = crate::domain::world::npc::NpcDefinition::priest(
            "good_priest",
            "Good Priest",
            "portrait",
        );

        let mut catalog = ServiceCatalog::new();
        catalog
            .services
            .push(ServiceEntry::new("heal_all", 50, "Heal all party members"));
        catalog
            .services
            .push(ServiceEntry::new("cure_poison", 25, "Cure poison"));
        catalog.services.push(ServiceEntry::new(
            "resurrect",
            200,
            "Resurrect dead character",
        ));
        catalog
            .services
            .push(ServiceEntry::new("rest", 10, "Rest the party"));
        priest.service_catalog = Some(catalog);
        db.npcs.add_npc(priest).unwrap();

        // Act
        let validator = Validator::new(&db);
        let warnings = validator.validate_service_catalogs();

        // Assert: no warnings for all-known service IDs
        assert!(
            warnings.is_empty(),
            "Expected no warnings but got: {:?}",
            warnings
        );
    }

    #[test]
    fn test_validate_service_catalogs_unknown_id() {
        // Arrange: NPC with service catalog containing an unknown service ID
        use crate::domain::inventory::{ServiceCatalog, ServiceEntry};

        let mut db = ContentDatabase::new();

        let mut priest = crate::domain::world::npc::NpcDefinition::priest(
            "custom_priest",
            "Custom Priest",
            "portrait",
        );

        let mut catalog = ServiceCatalog::new();
        catalog
            .services
            .push(ServiceEntry::new("heal_all", 50, "Heal all party members"));
        // Unknown custom service ID
        catalog.services.push(ServiceEntry::new(
            "transmute_gold",
            500,
            "Transmute items to gold",
        ));
        priest.service_catalog = Some(catalog);
        db.npcs.add_npc(priest).unwrap();

        // Act
        let validator = Validator::new(&db);
        let warnings = validator.validate_service_catalogs();

        // Assert: exactly one warning with Warning severity (not Error)
        assert_eq!(
            warnings.len(),
            1,
            "Expected exactly one warning but got: {:?}",
            warnings
        );
        assert_eq!(
            warnings[0].severity(),
            Severity::Warning,
            "Unknown service ID should be Warning severity, not Error"
        );
        assert!(
            matches!(
                &warnings[0],
                ValidationError::InvalidServiceId {
                    context,
                    service_id,
                } if context == "custom_priest" && service_id == "transmute_gold"
            ),
            "Expected InvalidServiceId for 'transmute_gold' but got: {:?}",
            warnings[0]
        );
    }

    // ===== Helmet / Boots Slot Integrity Tests =====

    #[test]
    fn test_sdk_validation_helmet_in_wrong_slot_fails() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::{CharacterDefinition, StartingEquipment};
        use crate::domain::classes::ClassDefinition;
        use crate::domain::items::types::{ArmorClassification, ArmorData, Item, ItemType};
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

        // Add a Boots-classified item to the item database
        let boots_item = Item {
            id: 26,
            name: "Leather Boots".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 1,
                weight: 2,
                classification: ArmorClassification::Boots,
            }),
            base_cost: 20,
            sell_cost: 10,
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
        db.items.add_item(boots_item).unwrap();

        // Create a character that places the Boots item in the helmet slot — wrong!
        let mut knight = CharacterDefinition::new(
            "test_knight".to_string(),
            "Sir Test".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        knight.starting_equipment = StartingEquipment {
            helmet: Some(26), // Boots item in helmet slot — should fail
            ..StartingEquipment::new()
        };
        db.characters.add_character(knight).unwrap();

        let validator = Validator::new(&db);
        let errors = validator.validate_all().unwrap();

        // Should have exactly one HelmetSlotTypeMismatch error
        let mismatch_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e, ValidationError::HelmetSlotTypeMismatch { .. }))
            .collect();

        assert_eq!(
            mismatch_errors.len(),
            1,
            "Expected one HelmetSlotTypeMismatch error but got: {:?}",
            errors
        );

        // Verify the error message contains the expected details
        let err_str = mismatch_errors[0].to_string();
        assert!(
            err_str.contains("test_knight"),
            "Error should mention character ID 'test_knight', got: {}",
            err_str
        );
        assert!(
            err_str.contains("Boots"),
            "Error should mention actual classification 'Boots', got: {}",
            err_str
        );
        assert_eq!(
            mismatch_errors[0].severity(),
            Severity::Error,
            "HelmetSlotTypeMismatch should be Error severity"
        );
    }

    #[test]
    fn test_sdk_validation_boots_in_wrong_slot_fails() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::{CharacterDefinition, StartingEquipment};
        use crate::domain::classes::ClassDefinition;
        use crate::domain::items::types::{ArmorClassification, ArmorData, Item, ItemType};
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

        // Add a Helmet-classified item to the item database
        let helmet_item = Item {
            id: 25,
            name: "Iron Helmet".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 1,
                weight: 5,
                classification: ArmorClassification::Helmet,
            }),
            base_cost: 40,
            sell_cost: 20,
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
        db.items.add_item(helmet_item).unwrap();

        // Create a character that places the Helmet item in the boots slot — wrong!
        let mut knight = CharacterDefinition::new(
            "test_knight2".to_string(),
            "Sir Test2".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        knight.starting_equipment = StartingEquipment {
            boots: Some(25), // Helmet item in boots slot — should fail
            ..StartingEquipment::new()
        };
        db.characters.add_character(knight).unwrap();

        let validator = Validator::new(&db);
        let errors = validator.validate_all().unwrap();

        // Should have exactly one BootsSlotTypeMismatch error
        let mismatch_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e, ValidationError::BootsSlotTypeMismatch { .. }))
            .collect();

        assert_eq!(
            mismatch_errors.len(),
            1,
            "Expected one BootsSlotTypeMismatch error but got: {:?}",
            errors
        );

        // Verify the error message contains the expected details
        let err_str = mismatch_errors[0].to_string();
        assert!(
            err_str.contains("test_knight2"),
            "Error should mention character ID 'test_knight2', got: {}",
            err_str
        );
        assert!(
            err_str.contains("Helmet"),
            "Error should mention actual classification 'Helmet', got: {}",
            err_str
        );
        assert_eq!(
            mismatch_errors[0].severity(),
            Severity::Error,
            "BootsSlotTypeMismatch should be Error severity"
        );
    }

    #[test]
    fn test_sdk_validation_correct_helmet_passes() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::{CharacterDefinition, StartingEquipment};
        use crate::domain::classes::ClassDefinition;
        use crate::domain::items::types::{ArmorClassification, ArmorData, Item, ItemType};
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

        // Add a correctly-classified Helmet item
        let helmet_item = Item {
            id: 25,
            name: "Iron Helmet".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 1,
                weight: 5,
                classification: ArmorClassification::Helmet,
            }),
            base_cost: 40,
            sell_cost: 20,
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
        db.items.add_item(helmet_item).unwrap();

        // Character correctly places a Helmet item in the helmet slot
        let mut knight = CharacterDefinition::new(
            "test_knight3".to_string(),
            "Sir Test3".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        knight.starting_equipment = StartingEquipment {
            helmet: Some(25), // Correctly-classified Helmet item
            ..StartingEquipment::new()
        };
        db.characters.add_character(knight).unwrap();

        let validator = Validator::new(&db);
        let errors = validator.validate_all().unwrap();

        // Should have NO HelmetSlotTypeMismatch errors
        let mismatch_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e, ValidationError::HelmetSlotTypeMismatch { .. }))
            .collect();

        assert!(
            mismatch_errors.is_empty(),
            "Expected no HelmetSlotTypeMismatch errors but got: {:?}",
            mismatch_errors
        );
    }

    #[test]
    fn test_locked_door_with_valid_key_item_passes_validation() {
        use crate::domain::items::{Item, ItemType, QuestData};
        use crate::domain::world::{Map, MapEvent};

        let mut db = ContentDatabase::new();

        // Add a key item with is_key_item: true
        let key_item = Item {
            id: 200,
            name: "Dungeon Gate Key".to_string(),
            item_type: ItemType::Quest(QuestData {
                quest_id: "dungeon_key".to_string(),
                is_key_item: true,
            }),
            base_cost: 0,
            sell_cost: 0,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 1,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        db.items.add_item(key_item).unwrap();

        // Add a map with LockedDoor referencing this key item
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let pos = crate::domain::types::Position::new(5, 5);
        map.add_event(
            pos,
            MapEvent::LockedDoor {
                name: "Gate".to_string(),
                lock_id: "gate_01".to_string(),
                key_item_id: Some(200),
                initial_trap_chance: 0,
            },
        );

        let validator = Validator::new(&db);
        let errors = validator.validate_map(&map).unwrap();

        // Should have no LockedObjectKeyNotKeyItem or DuplicateLockId errors
        assert!(
            !errors
                .iter()
                .any(|e| matches!(e, ValidationError::LockedObjectKeyNotKeyItem { .. })),
            "Expected no LockedObjectKeyNotKeyItem error but got: {:?}",
            errors
        );
        assert!(
            !errors
                .iter()
                .any(|e| matches!(e, ValidationError::DuplicateLockId { .. })),
            "Expected no DuplicateLockId error but got: {:?}",
            errors
        );
    }

    #[test]
    fn test_locked_door_with_invalid_key_item_id_fails_validation() {
        // key_item_id references a non-existent item — should emit MissingItem
        use crate::domain::world::{Map, MapEvent};

        let db = ContentDatabase::new(); // empty — item 999 does not exist

        let mut map = Map::new(2, "Test2".to_string(), "Desc".to_string(), 10, 10);
        let pos = crate::domain::types::Position::new(3, 3);
        map.add_event(
            pos,
            MapEvent::LockedDoor {
                name: "Vault Door".to_string(),
                lock_id: "vault_01".to_string(),
                key_item_id: Some(254),
                initial_trap_chance: 0,
            },
        );

        let validator = Validator::new(&db);
        let errors = validator.validate_map(&map).unwrap();

        // Should emit MissingItem for the non-existent item
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::MissingItem { item_id: 254, .. })),
            "Expected MissingItem error for item_id 254 but got: {:?}",
            errors
        );
        // Should NOT emit LockedObjectKeyNotKeyItem (item was not found at all)
        assert!(
            !errors
                .iter()
                .any(|e| matches!(e, ValidationError::LockedObjectKeyNotKeyItem { .. })),
            "Should not emit LockedObjectKeyNotKeyItem when item is missing entirely, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_locked_door_with_non_key_item_fails_validation() {
        // key_item_id references an item that exists but has is_key_item: false
        use crate::domain::items::{Item, ItemType, QuestData};
        use crate::domain::world::{Map, MapEvent};

        let mut db = ContentDatabase::new();

        // Add a quest item that is NOT a key item
        let non_key_item = Item {
            id: 201,
            name: "Old Letter".to_string(),
            item_type: ItemType::Quest(QuestData {
                quest_id: "side_quest".to_string(),
                is_key_item: false, // NOT a key item
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
            mesh_id: None,
        };
        db.items.add_item(non_key_item).unwrap();

        let mut map = Map::new(3, "Test3".to_string(), "Desc".to_string(), 10, 10);
        let pos = crate::domain::types::Position::new(4, 4);
        map.add_event(
            pos,
            MapEvent::LockedDoor {
                name: "Secret Door".to_string(),
                lock_id: "secret_01".to_string(),
                key_item_id: Some(201),
                initial_trap_chance: 0,
            },
        );

        let validator = Validator::new(&db);
        let errors = validator.validate_map(&map).unwrap();

        // Should emit LockedObjectKeyNotKeyItem
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::LockedObjectKeyNotKeyItem { .. })),
            "Expected LockedObjectKeyNotKeyItem error but got: {:?}",
            errors
        );
        // Verify the error carries the correct details
        assert!(
            errors.iter().any(|e| matches!(
                e,
                ValidationError::LockedObjectKeyNotKeyItem { map_id, lock_id, item_id }
                if *map_id == 3 && lock_id == "secret_01" && *item_id == 201
            )),
            "Expected LockedObjectKeyNotKeyItem with map_id=3, lock_id='secret_01', item_id=201, got: {:?}",
            errors
        );
        // Verify severity
        let key_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e, ValidationError::LockedObjectKeyNotKeyItem { .. }))
            .collect();
        assert_eq!(
            key_errors[0].severity(),
            Severity::Error,
            "LockedObjectKeyNotKeyItem should be Error severity"
        );
    }

    #[test]
    fn test_locked_door_with_duplicate_lock_id_fails_validation() {
        // Two LockedDoor events on the same map share the same lock_id
        use crate::domain::world::{Map, MapEvent};

        let db = ContentDatabase::new();

        let mut map = Map::new(4, "Test4".to_string(), "Desc".to_string(), 10, 10);

        // Add two LockedDoor events with the same lock_id at different positions
        map.add_event(
            crate::domain::types::Position::new(1, 1),
            MapEvent::LockedDoor {
                name: "Door A".to_string(),
                lock_id: "shared_lock".to_string(),
                key_item_id: None,
                initial_trap_chance: 0,
            },
        );
        map.add_event(
            crate::domain::types::Position::new(2, 2),
            MapEvent::LockedDoor {
                name: "Door B".to_string(),
                lock_id: "shared_lock".to_string(), // duplicate!
                key_item_id: None,
                initial_trap_chance: 0,
            },
        );

        let validator = Validator::new(&db);
        let errors = validator.validate_map(&map).unwrap();

        // Should emit a DuplicateLockId error
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::DuplicateLockId { .. })),
            "Expected DuplicateLockId error but got: {:?}",
            errors
        );
        // Verify the error carries the correct details
        assert!(
            errors.iter().any(|e| matches!(
                e,
                ValidationError::DuplicateLockId { map_id, lock_id }
                if *map_id == 4 && lock_id == "shared_lock"
            )),
            "Expected DuplicateLockId with map_id=4, lock_id='shared_lock', got: {:?}",
            errors
        );
        // Verify severity
        let dup_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e, ValidationError::DuplicateLockId { .. }))
            .collect();
        assert_eq!(
            dup_errors[0].severity(),
            Severity::Error,
            "DuplicateLockId should be Error severity"
        );
    }

    #[test]
    fn test_validate_connectivity_empty_database() {
        let db = ContentDatabase::new();
        let validator = Validator::new(&db);
        let errors = validator.validate_all().unwrap();
        // No maps → no connectivity errors
        let disconnected = errors
            .iter()
            .filter(|e| matches!(e, ValidationError::DisconnectedMap { .. }))
            .count();
        assert_eq!(disconnected, 0);
    }

    #[test]
    fn test_validate_references_with_empty_database() {
        let db = ContentDatabase::new();
        let validator = Validator::new(&db);
        let errors = validator.validate_all().unwrap();
        // Empty database should have no reference errors
        let missing_items = errors
            .iter()
            .filter(|e| matches!(e, ValidationError::MissingItem { .. }))
            .count();
        assert_eq!(missing_items, 0);
    }
}
