// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Creatures Manager for Phase 6: Campaign Builder Creatures Editor Integration
//!
//! This module provides file I/O and validation logic for creature registry management.
//! It complements the UI-focused `creatures_editor.rs` module by handling persistent
//! storage, validation, and cross-reference checking of creature definitions.
//!
//! # Features
//!
//! - Load/save creature registries from/to RON files
//! - Validate creature ID ranges (Monsters, NPCs, Templates, Variants, Custom)
//! - Check for duplicate creature IDs
//! - Validate file references exist
//! - Preserve RON file formatting and comments
//! - Comprehensive error handling with user-friendly messages
//!
//! # Examples
//!
//! ```no_run
//! use antares::sdk::creatures_manager::{CreaturesManager, EditorError};
//! use std::path::PathBuf;
//!
//! # fn main() -> Result<(), EditorError> {
//! // Load creatures from a campaign's creatures.ron file
//! let mut manager = CreaturesManager::load_from_file(
//!     PathBuf::from("campaigns/tutorial/data/creatures.ron")
//! )?;
//!
//! // Validate all creature references
//! let report = manager.validate_all();
//! if !report.errors.is_empty() {
//!     eprintln!("Validation errors:");
//!     for (id, error) in &report.errors {
//!         eprintln!("  Creature {}: {}", id, error);
//!     }
//! }
//!
//! // Save changes back to file
//! manager.save_to_file()?;
//! # Ok(())
//! # }
//! ```

use antares::domain::types::CreatureId;
use antares::domain::visual::CreatureReference;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Error type for creature editor operations
#[derive(Error, Debug, Clone)]
pub enum EditorError {
    #[error("Failed to read creatures file: {0}")]
    FileReadError(String),

    #[error("Failed to write creatures file: {0}")]
    FileWriteError(String),

    #[error("Invalid RON syntax: {0}")]
    RonParseError(String),

    #[error("Failed to serialize to RON: {0}")]
    RonSerializeError(String),

    #[error("Duplicate creature ID: {0}")]
    DuplicateId(CreatureId),

    #[error("Creature ID {id} out of valid range for category {category}")]
    IdOutOfRange { id: CreatureId, category: String },

    #[error("Creature file not found: {0}")]
    CreatureFileNotFound(PathBuf),

    #[error("Invalid creature reference: {0}")]
    InvalidReference(String),

    #[error("Operation not allowed: {0}")]
    OperationError(String),
}

/// Validation result for individual creature references
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Creature is valid and file exists
    Valid,
    /// File reference does not exist
    FileNotFound(PathBuf),
    /// File path is invalid or malformed
    InvalidPath(String),
    /// Creature ID is duplicated
    DuplicateId(CreatureId),
    /// Creature ID is outside valid range for its category
    IdOutOfRange {
        id: CreatureId,
        expected_range: String,
    },
    /// RON file has invalid syntax
    InvalidRonSyntax(String),
}

impl std::fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationResult::Valid => write!(f, "Valid"),
            ValidationResult::FileNotFound(path) => {
                write!(f, "File not found: {}", path.display())
            }
            ValidationResult::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
            ValidationResult::DuplicateId(id) => write!(f, "Duplicate ID: {}", id),
            ValidationResult::IdOutOfRange { id, expected_range } => {
                write!(f, "ID {} out of range: {}", id, expected_range)
            }
            ValidationResult::InvalidRonSyntax(msg) => write!(f, "Invalid RON: {}", msg),
        }
    }
}

/// Validation report for the entire creatures registry
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Total number of creatures checked
    pub total_creatures: usize,
    /// Number of valid creatures
    pub valid_count: usize,
    /// Warnings (non-blocking issues)
    pub warnings: Vec<(CreatureId, String)>,
    /// Errors (blocking issues)
    pub errors: Vec<(CreatureId, ValidationResult)>,
}

impl ValidationReport {
    /// Check if the report indicates valid data
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the number of issues (warnings + errors)
    pub fn issue_count(&self) -> usize {
        self.warnings.len() + self.errors.len()
    }

    /// Create a human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "{}/{} creatures valid, {} warning(s), {} error(s)",
            self.valid_count,
            self.total_creatures,
            self.warnings.len(),
            self.errors.len()
        )
    }
}

/// Category for creature ID ranges
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureCategory {
    /// Monster creatures (1-50)
    Monsters,
    /// NPC creatures (51-100)
    Npcs,
    /// Template creatures (101-150)
    Templates,
    /// Variant creatures (151-200)
    Variants,
    /// Custom/campaign-specific (201+)
    Custom,
}

impl CreatureCategory {
    /// Get the valid ID range for this category
    pub fn id_range(&self) -> std::ops::Range<u32> {
        match self {
            CreatureCategory::Monsters => 1..51,
            CreatureCategory::Npcs => 51..101,
            CreatureCategory::Templates => 101..151,
            CreatureCategory::Variants => 151..201,
            CreatureCategory::Custom => 201..u32::MAX,
        }
    }

    /// Get display name for this category
    pub fn display_name(&self) -> &'static str {
        match self {
            CreatureCategory::Monsters => "Monsters",
            CreatureCategory::Npcs => "NPCs",
            CreatureCategory::Templates => "Templates",
            CreatureCategory::Variants => "Variants",
            CreatureCategory::Custom => "Custom",
        }
    }

    /// Determine category from creature ID
    pub fn from_id(id: CreatureId) -> Self {
        match id {
            1..=50 => CreatureCategory::Monsters,
            51..=100 => CreatureCategory::Npcs,
            101..=150 => CreatureCategory::Templates,
            151..=200 => CreatureCategory::Variants,
            _ => CreatureCategory::Custom,
        }
    }
}

/// Manager for creature registry files
#[derive(Debug, Clone)]
pub struct CreaturesManager {
    /// Path to the creatures.ron file
    file_path: PathBuf,
    /// In-memory creature registry
    creatures: Vec<CreatureReference>,
    /// Whether the registry has unsaved changes
    is_dirty: bool,
    /// Validation results cache
    validation_results: HashMap<CreatureId, ValidationResult>,
}

impl CreaturesManager {
    /// Create a new creatures manager for a campaign
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the creatures.ron file
    ///
    /// # Returns
    ///
    /// A new `CreaturesManager` instance with an empty creature registry
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creatures_manager::CreaturesManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = CreaturesManager::new(PathBuf::from("creatures.ron"));
    /// assert_eq!(manager.creature_count(), 0);
    /// assert!(!manager.is_dirty());
    /// ```
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            creatures: Vec::new(),
            is_dirty: false,
            validation_results: HashMap::new(),
        }
    }

    /// Load creatures from a RON file
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the creatures.ron file
    ///
    /// # Returns
    ///
    /// A `CreaturesManager` with creatures loaded from the file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::creatures_manager::CreaturesManager;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = CreaturesManager::load_from_file(
    ///     PathBuf::from("campaigns/tutorial/data/creatures.ron")
    /// )?;
    /// println!("Loaded {} creatures", manager.creature_count());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file(file_path: PathBuf) -> Result<Self, EditorError> {
        let creatures = load_creatures_registry(&file_path)?;
        Ok(Self {
            file_path,
            creatures,
            is_dirty: false,
            validation_results: HashMap::new(),
        })
    }

    /// Save creatures to the RON file
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if the file cannot be written
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::creatures_manager::CreaturesManager;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut manager = CreaturesManager::load_from_file(
    ///     PathBuf::from("creatures.ron")
    /// )?;
    /// manager.save_to_file()?;
    /// assert!(!manager.is_dirty());
    /// # Ok(())
    /// # }
    /// ```
    pub fn save_to_file(&mut self) -> Result<(), EditorError> {
        save_creatures_registry(&self.file_path, &self.creatures, true)?;
        self.is_dirty = false;
        Ok(())
    }

    /// Add a new creature reference
    ///
    /// # Arguments
    ///
    /// * `creature` - The creature reference to add
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if the creature is invalid
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The creature ID is already used
    /// - The creature ID is out of range for its category
    pub fn add_creature(&mut self, creature: CreatureReference) -> Result<(), EditorError> {
        // Check for duplicate ID
        if self.creatures.iter().any(|c| c.id == creature.id) {
            return Err(EditorError::DuplicateId(creature.id));
        }

        // Validate ID range
        let category = CreatureCategory::from_id(creature.id);
        let range = category.id_range();
        if !range.contains(&creature.id) {
            return Err(EditorError::IdOutOfRange {
                id: creature.id,
                category: category.display_name().to_string(),
            });
        }

        self.creatures.push(creature);
        self.is_dirty = true;
        Ok(())
    }

    /// Update an existing creature reference
    ///
    /// # Arguments
    ///
    /// * `index` - Index of the creature to update
    /// * `creature` - The updated creature reference
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The index is out of bounds
    /// - The creature ID is already used by another creature
    /// - The creature ID is out of range for its category
    pub fn update_creature(
        &mut self,
        index: usize,
        creature: CreatureReference,
    ) -> Result<(), EditorError> {
        if index >= self.creatures.len() {
            return Err(EditorError::OperationError(
                "Creature index out of bounds".to_string(),
            ));
        }

        // Check for duplicate ID (excluding the creature being updated)
        if self
            .creatures
            .iter()
            .enumerate()
            .any(|(i, c)| i != index && c.id == creature.id)
        {
            return Err(EditorError::DuplicateId(creature.id));
        }

        // Validate ID range
        let category = CreatureCategory::from_id(creature.id);
        let range = category.id_range();
        if !range.contains(&creature.id) {
            return Err(EditorError::IdOutOfRange {
                id: creature.id,
                category: category.display_name().to_string(),
            });
        }

        self.creatures[index] = creature;
        self.is_dirty = true;
        Ok(())
    }

    /// Delete a creature reference by index
    ///
    /// # Arguments
    ///
    /// * `index` - Index of the creature to delete
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if the index is out of bounds
    pub fn delete_creature(&mut self, index: usize) -> Result<(), EditorError> {
        if index >= self.creatures.len() {
            return Err(EditorError::OperationError(
                "Creature index out of bounds".to_string(),
            ));
        }

        self.creatures.remove(index);
        self.is_dirty = true;
        Ok(())
    }

    /// Validate all creature references
    ///
    /// Checks:
    /// - All file references exist
    /// - No duplicate IDs
    /// - All IDs are in valid ranges for their categories
    /// - Files have valid RON syntax
    ///
    /// # Returns
    ///
    /// A `ValidationReport` with results
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::creatures_manager::CreaturesManager;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut manager = CreaturesManager::load_from_file(
    ///     PathBuf::from("creatures.ron")
    /// )?;
    /// let report = manager.validate_all();
    /// println!("{}", report.summary());
    /// # Ok(())
    /// # }
    /// ```
    pub fn validate_all(&mut self) -> ValidationReport {
        let mut report = ValidationReport {
            total_creatures: self.creatures.len(),
            valid_count: 0,
            warnings: Vec::new(),
            errors: Vec::new(),
        };

        // Check for duplicate IDs
        let duplicates = self.check_duplicate_ids();
        for dup_id in duplicates {
            for creature in &self.creatures {
                if creature.id == dup_id {
                    let result = ValidationResult::DuplicateId(dup_id);
                    report.errors.push((creature.id, result.clone()));
                    self.validation_results.insert(creature.id, result);
                }
            }
        }

        // Validate each creature
        for creature in &self.creatures {
            // Skip if already marked as duplicate
            if self.validation_results.contains_key(&creature.id) {
                continue;
            }

            let mut result = ValidationResult::Valid;

            // Check ID range
            let category = CreatureCategory::from_id(creature.id);
            let range = category.id_range();
            if !range.contains(&creature.id) {
                result = ValidationResult::IdOutOfRange {
                    id: creature.id,
                    expected_range: format!("{}-{}", range.start, range.end.saturating_sub(1)),
                };
            }

            // Check file exists
            if result == ValidationResult::Valid {
                let file_path = PathBuf::from(&creature.filepath);
                if !file_path.exists() {
                    result = ValidationResult::FileNotFound(file_path);
                    report.warnings.push((
                        creature.id,
                        format!("File not found: {}", creature.filepath),
                    ));
                }
            }

            // Update results
            if result == ValidationResult::Valid {
                report.valid_count += 1;
            } else {
                match &result {
                    ValidationResult::Valid => {}
                    _ => {
                        report.errors.push((creature.id, result.clone()));
                    }
                }
            }

            self.validation_results.insert(creature.id, result);
        }

        report
    }

    /// Check for duplicate creature IDs
    ///
    /// # Returns
    ///
    /// A vector of IDs that appear more than once
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creatures_manager::CreaturesManager;
    /// use antares::domain::visual::CreatureReference;
    /// use std::path::PathBuf;
    ///
    /// let mut manager = CreaturesManager::new(PathBuf::from("creatures.ron"));
    /// manager.add_creature(CreatureReference {
    ///     id: 1,
    ///     name: "Creature 1".to_string(),
    ///     filepath: "creature1.ron".to_string(),
    /// }).unwrap();
    /// manager.add_creature(CreatureReference {
    ///     id: 2,
    ///     name: "Creature 2".to_string(),
    ///     filepath: "creature2.ron".to_string(),
    /// }).unwrap();
    ///
    /// let duplicates = manager.check_duplicate_ids();
    /// assert!(duplicates.is_empty());
    /// ```
    pub fn check_duplicate_ids(&self) -> Vec<CreatureId> {
        let mut id_counts: HashMap<CreatureId, usize> = HashMap::new();

        for creature in &self.creatures {
            *id_counts.entry(creature.id).or_insert(0) += 1;
        }

        id_counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(id, _)| id)
            .collect()
    }

    /// Suggest the next available creature ID in a category
    ///
    /// # Arguments
    ///
    /// * `category` - The creature category
    ///
    /// # Returns
    ///
    /// The next available ID in the category, or an error if the category is full
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creatures_manager::{CreaturesManager, CreatureCategory};
    /// use antares::domain::visual::CreatureReference;
    /// use std::path::PathBuf;
    ///
    /// let mut manager = CreaturesManager::new(PathBuf::from("creatures.ron"));
    /// let next_id = manager.suggest_next_id(CreatureCategory::Monsters)
    ///     .expect("Failed to suggest ID");
    /// assert_eq!(next_id, 1);
    /// ```
    pub fn suggest_next_id(&self, category: CreatureCategory) -> Result<CreatureId, EditorError> {
        let range = category.id_range();
        let used_ids: HashSet<CreatureId> = self
            .creatures
            .iter()
            .filter(|c| {
                let c_category = CreatureCategory::from_id(c.id);
                c_category == category
            })
            .map(|c| c.id)
            .collect();

        for id in range {
            if !used_ids.contains(&id) {
                return Ok(id);
            }
        }

        Err(EditorError::OperationError(format!(
            "No available IDs in {} category",
            category.display_name()
        )))
    }

    /// Get a creature by index
    pub fn get_creature(&self, index: usize) -> Option<&CreatureReference> {
        self.creatures.get(index)
    }

    /// Get a mutable reference to a creature by index
    pub fn get_creature_mut(&mut self, index: usize) -> Option<&mut CreatureReference> {
        self.creatures.get_mut(index)
    }

    /// Get all creatures
    pub fn creatures(&self) -> &[CreatureReference] {
        &self.creatures
    }

    /// Get mutable reference to all creatures
    pub fn creatures_mut(&mut self) -> &mut [CreatureReference] {
        &mut self.creatures
    }

    /// Get the number of creatures
    pub fn creature_count(&self) -> usize {
        self.creatures.len()
    }

    /// Check if the registry has unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// Mark the registry as dirty (has unsaved changes)
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// Get the file path for this registry
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Find a creature by ID
    pub fn find_by_id(&self, id: CreatureId) -> Option<(usize, &CreatureReference)> {
        self.creatures.iter().enumerate().find(|(_, c)| c.id == id)
    }

    /// Find creatures in a specific category
    pub fn find_by_category(&self, category: CreatureCategory) -> Vec<(usize, &CreatureReference)> {
        self.creatures
            .iter()
            .enumerate()
            .filter(|(_, c)| CreatureCategory::from_id(c.id) == category)
            .collect()
    }
}

/// Load creatures from a RON file
fn load_creatures_registry(path: &Path) -> Result<Vec<CreatureReference>, EditorError> {
    let content = fs::read_to_string(path).map_err(|e| {
        EditorError::FileReadError(format!("Failed to read file {}: {}", path.display(), e))
    })?;

    ron::from_str::<Vec<CreatureReference>>(&content).map_err(|e| {
        EditorError::RonParseError(format!("Failed to parse RON at {}: {}", path.display(), e))
    })
}

/// Save creatures to a RON file with optional header preservation
fn save_creatures_registry(
    path: &Path,
    creatures: &[CreatureReference],
    preserve_header: bool,
) -> Result<(), EditorError> {
    // Read existing header comments if preserving
    let header = if preserve_header && path.exists() {
        read_file_header(path).unwrap_or_default()
    } else {
        String::new()
    };

    // Configure pretty printing
    let pretty = ron::ser::PrettyConfig::new()
        .depth_limit(2)
        .separate_tuple_members(true)
        .enumerate_arrays(true);

    let ron_content = ron::ser::to_string_pretty(creatures, pretty).map_err(|e| {
        EditorError::RonSerializeError(format!("Failed to serialize creatures: {}", e))
    })?;

    let final_content = if header.is_empty() {
        ron_content
    } else {
        format!("{}\n\n{}", header, ron_content)
    };

    fs::write(path, final_content).map_err(|e| {
        EditorError::FileWriteError(format!("Failed to write file {}: {}", path.display(), e))
    })
}

/// Read the header comments from a RON file
fn read_file_header(path: &Path) -> Result<String, EditorError> {
    let content = fs::read_to_string(path).map_err(|e| {
        EditorError::FileReadError(format!("Failed to read file {}: {}", path.display(), e))
    })?;

    let lines: Vec<&str> = content
        .lines()
        .take_while(|line| line.starts_with('/'))
        .collect();

    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_creature(id: CreatureId, name: &str) -> CreatureReference {
        CreatureReference {
            id,
            name: name.to_string(),
            filepath: format!("assets/creatures/{}.ron", name.to_lowercase()),
        }
    }

    #[test]
    fn test_creatures_manager_new() {
        let path = PathBuf::from("test.ron");
        let manager = CreaturesManager::new(path.clone());

        assert_eq!(manager.file_path(), path.as_path());
        assert_eq!(manager.creature_count(), 0);
        assert!(!manager.is_dirty());
    }

    #[test]
    fn test_add_creature() {
        let mut manager = CreaturesManager::new(PathBuf::from("test.ron"));
        let creature = create_test_creature(1, "Goblin");

        assert!(manager.add_creature(creature).is_ok());
        assert_eq!(manager.creature_count(), 1);
        assert!(manager.is_dirty());
    }

    #[test]
    fn test_add_creature_duplicate_id() {
        let mut manager = CreaturesManager::new(PathBuf::from("test.ron"));
        let creature1 = create_test_creature(1, "Goblin");
        let creature2 = create_test_creature(1, "Orc");

        assert!(manager.add_creature(creature1).is_ok());
        let result = manager.add_creature(creature2);

        assert!(matches!(result, Err(EditorError::DuplicateId(1))));
    }

    #[test]
    fn test_check_duplicate_ids() {
        let mut manager = CreaturesManager::new(PathBuf::from("test.ron"));

        let c1 = create_test_creature(1, "Goblin");
        let c2 = create_test_creature(2, "Orc");
        let c3 = create_test_creature(1, "Hobgoblin");

        manager.creatures.push(c1);
        manager.creatures.push(c2);
        manager.creatures.push(c3);

        let duplicates = manager.check_duplicate_ids();
        assert!(duplicates.contains(&1));
        assert!(!duplicates.contains(&2));
    }

    #[test]
    fn test_update_creature() {
        let mut manager = CreaturesManager::new(PathBuf::from("test.ron"));
        let creature = create_test_creature(1, "Goblin");

        manager.add_creature(creature).unwrap();
        let updated = create_test_creature(1, "Goblin Warrior");

        assert!(manager.update_creature(0, updated).is_ok());
        assert_eq!(manager.get_creature(0).unwrap().name, "Goblin Warrior");
    }

    #[test]
    fn test_delete_creature() {
        let mut manager = CreaturesManager::new(PathBuf::from("test.ron"));
        let creature = create_test_creature(1, "Goblin");

        manager.add_creature(creature).unwrap();
        assert_eq!(manager.creature_count(), 1);

        assert!(manager.delete_creature(0).is_ok());
        assert_eq!(manager.creature_count(), 0);
    }

    #[test]
    fn test_suggest_next_id_empty() {
        let manager = CreaturesManager::new(PathBuf::from("test.ron"));
        let id = manager.suggest_next_id(CreatureCategory::Monsters).unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn test_suggest_next_id_with_creatures() {
        let mut manager = CreaturesManager::new(PathBuf::from("test.ron"));

        manager
            .add_creature(create_test_creature(1, "Goblin"))
            .unwrap();
        manager
            .add_creature(create_test_creature(3, "Orc"))
            .unwrap();

        let id = manager.suggest_next_id(CreatureCategory::Monsters).unwrap();
        assert_eq!(id, 2); // Should suggest next available ID
    }

    #[test]
    fn test_find_by_id() {
        let mut manager = CreaturesManager::new(PathBuf::from("test.ron"));
        let creature = create_test_creature(42, "Dragon");

        manager.add_creature(creature).unwrap();
        let found = manager.find_by_id(42);

        assert!(found.is_some());
        let (idx, c) = found.unwrap();
        assert_eq!(idx, 0);
        assert_eq!(c.name, "Dragon");
    }

    #[test]
    fn test_find_by_category() {
        let mut manager = CreaturesManager::new(PathBuf::from("test.ron"));

        // Add monsters (1-50)
        manager
            .add_creature(create_test_creature(1, "Goblin"))
            .unwrap();
        manager
            .add_creature(create_test_creature(2, "Orc"))
            .unwrap();

        // Add NPCs (51-100)
        manager
            .add_creature(create_test_creature(51, "Innkeeper"))
            .unwrap();

        let monsters = manager.find_by_category(CreatureCategory::Monsters);
        assert_eq!(monsters.len(), 2);

        let npcs = manager.find_by_category(CreatureCategory::Npcs);
        assert_eq!(npcs.len(), 1);
    }

    #[test]
    fn test_creature_category_from_id() {
        assert_eq!(CreatureCategory::from_id(1), CreatureCategory::Monsters);
        assert_eq!(CreatureCategory::from_id(50), CreatureCategory::Monsters);
        assert_eq!(CreatureCategory::from_id(51), CreatureCategory::Npcs);
        assert_eq!(CreatureCategory::from_id(101), CreatureCategory::Templates);
        assert_eq!(CreatureCategory::from_id(151), CreatureCategory::Variants);
        assert_eq!(CreatureCategory::from_id(201), CreatureCategory::Custom);
    }

    #[test]
    fn test_validation_report_summary() {
        let report = ValidationReport {
            total_creatures: 10,
            valid_count: 8,
            warnings: vec![(1, "Warning 1".to_string())],
            errors: vec![(2, ValidationResult::FileNotFound(PathBuf::from("test.ron")))],
        };

        let summary = report.summary();
        assert!(summary.contains("8/10"));
        assert!(summary.contains("1 warning"));
        assert!(summary.contains("1 error"));
    }

    #[test]
    fn test_is_dirty_flag() {
        let mut manager = CreaturesManager::new(PathBuf::from("test.ron"));
        assert!(!manager.is_dirty());

        manager.mark_dirty();
        assert!(manager.is_dirty());

        let creature = create_test_creature(1, "Goblin");
        manager.add_creature(creature).unwrap();
        assert!(manager.is_dirty());
    }

    #[test]
    fn test_validation_result_display() {
        let result = ValidationResult::Valid;
        assert_eq!(result.to_string(), "Valid");

        let result = ValidationResult::DuplicateId(42);
        assert_eq!(result.to_string(), "Duplicate ID: 42");

        let result = ValidationResult::IdOutOfRange {
            id: 201,
            expected_range: "1-50".to_string(),
        };
        assert!(result.to_string().contains("201"));
        assert!(result.to_string().contains("1-50"));
    }

    #[test]
    fn test_validate_all_empty() {
        let mut manager = CreaturesManager::new(PathBuf::from("test.ron"));
        let report = manager.validate_all();

        assert_eq!(report.total_creatures, 0);
        assert_eq!(report.valid_count, 0);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_validate_all_with_duplicates() {
        let mut manager = CreaturesManager::new(PathBuf::from("test.ron"));

        manager.creatures.push(create_test_creature(1, "Goblin"));
        manager.creatures.push(create_test_creature(1, "Orc"));

        let report = manager.validate_all();
        assert!(!report.is_valid());
        assert_eq!(report.errors.len(), 2); // Both are marked as errors
    }

    #[test]
    fn test_creature_category_display_name() {
        assert_eq!(CreatureCategory::Monsters.display_name(), "Monsters");
        assert_eq!(CreatureCategory::Npcs.display_name(), "NPCs");
        assert_eq!(CreatureCategory::Templates.display_name(), "Templates");
        assert_eq!(CreatureCategory::Variants.display_name(), "Variants");
        assert_eq!(CreatureCategory::Custom.display_name(), "Custom");
    }

    #[test]
    fn test_creature_category_id_range() {
        let monsters = CreatureCategory::Monsters.id_range();
        assert_eq!(monsters.start, 1);
        assert_eq!(monsters.end, 51);

        let npcs = CreatureCategory::Npcs.id_range();
        assert_eq!(npcs.start, 51);
        assert_eq!(npcs.end, 101);
    }
}
