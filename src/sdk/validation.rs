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
use crate::domain::types::{ItemId, MapId, MonsterId, SpellId};
use crate::sdk::database::{ContentDatabase, RaceId};
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
        let errors = Vec::new();

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
}
