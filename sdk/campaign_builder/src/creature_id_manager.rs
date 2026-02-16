// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Creature ID Management
//!
//! This module provides tools for managing creature IDs within a registry,
//! including validation, conflict detection, and automatic ID assignment
//! according to category ranges.
//!
//! # Category Ranges
//!
//! - Monsters: 1-50
//! - NPCs: 51-100
//! - Templates: 101-150
//! - Variants: 151-200
//! - Custom: 201+
//!
//! # Examples
//!
//! ```
//! use antares::sdk::creature_id_manager::{CreatureIdManager, CreatureCategory};
//! use antares::domain::visual::CreatureReference;
//! use antares::domain::types::CreatureId;
//!
//! let mut manager = CreatureIdManager::new();
//! let mut references = vec![
//!     CreatureReference {
//!         id: 1,
//!         name: "Goblin".to_string(),
//!         filepath: "creatures/goblin.ron".to_string(),
//!     },
//!     CreatureReference {
//!         id: 2,
//!         name: "Orc".to_string(),
//!         filepath: "creatures/orc.ron".to_string(),
//!     },
//! ];
//!
//! manager.update_from_registry(&references);
//!
//! // Suggest next available monster ID
//! let next_id = manager.suggest_next_id(CreatureCategory::Monsters);
//! assert_eq!(next_id, 3);
//!
//! // Validate an ID
//! assert!(manager.validate_id(3, CreatureCategory::Monsters).is_ok());
//! assert!(manager.validate_id(1, CreatureCategory::Monsters).is_err()); // Already used
//! ```

use antares::domain::types::CreatureId;
use antares::domain::visual::CreatureReference;
use std::collections::HashSet;
use thiserror::Error;

/// Category for creature ID ranges
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    ///
    /// # Returns
    ///
    /// A range of valid creature IDs for this category
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creature_id_manager::CreatureCategory;
    ///
    /// let range = CreatureCategory::Monsters.id_range();
    /// assert_eq!(range, 1..51);
    /// assert!(range.contains(&1));
    /// assert!(range.contains(&50));
    /// assert!(!range.contains(&51));
    /// ```
    pub fn id_range(&self) -> std::ops::Range<CreatureId> {
        match self {
            CreatureCategory::Monsters => 1..51,
            CreatureCategory::Npcs => 51..101,
            CreatureCategory::Templates => 101..151,
            CreatureCategory::Variants => 151..201,
            CreatureCategory::Custom => 201..u32::MAX,
        }
    }

    /// Get display name for this category
    ///
    /// # Returns
    ///
    /// A human-readable name for the category
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creature_id_manager::CreatureCategory;
    ///
    /// assert_eq!(CreatureCategory::Monsters.display_name(), "Monsters");
    /// assert_eq!(CreatureCategory::Npcs.display_name(), "NPCs");
    /// ```
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
    ///
    /// # Arguments
    ///
    /// * `id` - The creature ID to categorize
    ///
    /// # Returns
    ///
    /// The category that the ID belongs to
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creature_id_manager::CreatureCategory;
    ///
    /// assert_eq!(CreatureCategory::from_id(1), CreatureCategory::Monsters);
    /// assert_eq!(CreatureCategory::from_id(50), CreatureCategory::Monsters);
    /// assert_eq!(CreatureCategory::from_id(51), CreatureCategory::Npcs);
    /// assert_eq!(CreatureCategory::from_id(250), CreatureCategory::Custom);
    /// ```
    pub fn from_id(id: CreatureId) -> Self {
        match id {
            1..=50 => CreatureCategory::Monsters,
            51..=100 => CreatureCategory::Npcs,
            101..=150 => CreatureCategory::Templates,
            151..=200 => CreatureCategory::Variants,
            _ => CreatureCategory::Custom,
        }
    }

    /// Get color for UI display
    ///
    /// # Returns
    ///
    /// RGB color array for the category badge
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creature_id_manager::CreatureCategory;
    ///
    /// let color = CreatureCategory::Monsters.color();
    /// assert_eq!(color.len(), 3);
    /// ```
    pub fn color(&self) -> [f32; 3] {
        match self {
            CreatureCategory::Monsters => [0.8, 0.2, 0.2],  // Red
            CreatureCategory::Npcs => [0.2, 0.6, 0.8],      // Blue
            CreatureCategory::Templates => [0.6, 0.4, 0.8], // Purple
            CreatureCategory::Variants => [0.2, 0.8, 0.4],  // Green
            CreatureCategory::Custom => [0.8, 0.6, 0.2],    // Orange
        }
    }
}

/// Error type for ID management operations
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    #[error("ID {id} is already in use")]
    DuplicateId { id: CreatureId },

    #[error("ID {id} is outside the valid range {range} for category {category}")]
    OutOfRange {
        id: CreatureId,
        category: String,
        range: String,
    },

    #[error("ID {id} conflicts with category {expected_category}, but is in range for {actual_category}")]
    CategoryMismatch {
        id: CreatureId,
        expected_category: String,
        actual_category: String,
    },

    #[error("No available IDs in category {0}")]
    NoAvailableIds(String),
}

/// Represents an ID conflict detected in the registry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdConflict {
    /// The conflicting creature ID
    pub id: CreatureId,
    /// Names of creatures with this ID
    pub creature_names: Vec<String>,
    /// The category this ID belongs to
    pub category: CreatureCategory,
}

/// Represents a suggested ID change for auto-reassignment
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdChange {
    /// Original creature ID
    pub old_id: CreatureId,
    /// Suggested new ID
    pub new_id: CreatureId,
    /// Creature name
    pub creature_name: String,
    /// Reason for the change
    pub reason: String,
}

/// Manager for creature ID validation and assignment
///
/// Provides tools for managing creature IDs within a registry,
/// including validation, conflict detection, and automatic ID assignment.
///
/// # Examples
///
/// ```
/// use antares::sdk::creature_id_manager::{CreatureIdManager, CreatureCategory};
/// use antares::domain::visual::CreatureReference;
///
/// let mut manager = CreatureIdManager::new();
/// let references = vec![
///     CreatureReference {
///         id: 1,
///         name: "Goblin".to_string(),
///         filepath: "creatures/goblin.ron".to_string(),
///     },
/// ];
///
/// manager.update_from_registry(&references);
/// let next_id = manager.suggest_next_id(CreatureCategory::Monsters);
/// assert_eq!(next_id, 2);
/// ```
#[derive(Debug, Clone)]
pub struct CreatureIdManager {
    /// Set of currently used IDs
    used_ids: HashSet<CreatureId>,
    /// Map of IDs to creature names (for conflict reporting)
    id_to_names: std::collections::HashMap<CreatureId, Vec<String>>,
}

impl CreatureIdManager {
    /// Create a new empty ID manager
    ///
    /// # Returns
    ///
    /// A new `CreatureIdManager` instance with no registered IDs
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creature_id_manager::CreatureIdManager;
    ///
    /// let manager = CreatureIdManager::new();
    /// assert_eq!(manager.used_id_count(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            used_ids: HashSet::new(),
            id_to_names: std::collections::HashMap::new(),
        }
    }

    /// Update the manager's state from a creature registry
    ///
    /// # Arguments
    ///
    /// * `registry` - Slice of creature references to register
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creature_id_manager::CreatureIdManager;
    /// use antares::domain::visual::CreatureReference;
    ///
    /// let mut manager = CreatureIdManager::new();
    /// let refs = vec![
    ///     CreatureReference {
    ///         id: 1,
    ///         name: "Goblin".to_string(),
    ///         filepath: "creatures/goblin.ron".to_string(),
    ///     },
    /// ];
    ///
    /// manager.update_from_registry(&refs);
    /// assert!(manager.is_id_used(1));
    /// assert!(!manager.is_id_used(2));
    /// ```
    pub fn update_from_registry(&mut self, registry: &[CreatureReference]) {
        self.used_ids.clear();
        self.id_to_names.clear();

        for reference in registry {
            self.used_ids.insert(reference.id);
            self.id_to_names
                .entry(reference.id)
                .or_default()
                .push(reference.name.clone());
        }
    }

    /// Suggest the next available ID for a given category
    ///
    /// # Arguments
    ///
    /// * `category` - The category to suggest an ID for
    ///
    /// # Returns
    ///
    /// The next available ID in the category's range, or the first ID if all are taken
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creature_id_manager::{CreatureIdManager, CreatureCategory};
    /// use antares::domain::visual::CreatureReference;
    ///
    /// let mut manager = CreatureIdManager::new();
    /// let refs = vec![
    ///     CreatureReference { id: 1, name: "Goblin".to_string(), filepath: "".to_string() },
    ///     CreatureReference { id: 2, name: "Orc".to_string(), filepath: "".to_string() },
    /// ];
    /// manager.update_from_registry(&refs);
    ///
    /// assert_eq!(manager.suggest_next_id(CreatureCategory::Monsters), 3);
    /// assert_eq!(manager.suggest_next_id(CreatureCategory::Npcs), 51);
    /// ```
    pub fn suggest_next_id(&self, category: CreatureCategory) -> CreatureId {
        let range = category.id_range();

        // Find first unused ID in the range
        for id in range.clone() {
            if !self.used_ids.contains(&id) {
                return id;
            }
        }

        // If all IDs are used, return the first ID in the range
        range.start
    }

    /// Validate an ID for a given category
    ///
    /// # Arguments
    ///
    /// * `id` - The ID to validate
    /// * `category` - The expected category for the ID
    ///
    /// # Returns
    ///
    /// `Ok(())` if the ID is valid, or an error describing the issue
    ///
    /// # Errors
    ///
    /// Returns `IdError::DuplicateId` if the ID is already in use
    /// Returns `IdError::OutOfRange` if the ID is outside the category's range
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creature_id_manager::{CreatureIdManager, CreatureCategory};
    /// use antares::domain::visual::CreatureReference;
    ///
    /// let mut manager = CreatureIdManager::new();
    /// let refs = vec![
    ///     CreatureReference { id: 1, name: "Goblin".to_string(), filepath: "".to_string() },
    /// ];
    /// manager.update_from_registry(&refs);
    ///
    /// assert!(manager.validate_id(2, CreatureCategory::Monsters).is_ok());
    /// assert!(manager.validate_id(1, CreatureCategory::Monsters).is_err()); // Duplicate
    /// assert!(manager.validate_id(51, CreatureCategory::Monsters).is_err()); // Out of range
    /// ```
    pub fn validate_id(&self, id: CreatureId, category: CreatureCategory) -> Result<(), IdError> {
        // Check if ID is already used
        if self.used_ids.contains(&id) {
            return Err(IdError::DuplicateId { id });
        }

        // Check if ID is in the correct range
        let range = category.id_range();
        if !range.contains(&id) {
            return Err(IdError::OutOfRange {
                id,
                category: category.display_name().to_string(),
                range: format!("{}-{}", range.start, range.end - 1),
            });
        }

        Ok(())
    }

    /// Check for ID conflicts in the current registry
    ///
    /// # Returns
    ///
    /// Vector of detected conflicts where multiple creatures share the same ID
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creature_id_manager::CreatureIdManager;
    /// use antares::domain::visual::CreatureReference;
    ///
    /// let mut manager = CreatureIdManager::new();
    /// let refs = vec![
    ///     CreatureReference { id: 1, name: "Goblin".to_string(), filepath: "".to_string() },
    ///     CreatureReference { id: 1, name: "Orc".to_string(), filepath: "".to_string() },
    /// ];
    /// manager.update_from_registry(&refs);
    ///
    /// let conflicts = manager.check_conflicts();
    /// assert_eq!(conflicts.len(), 1);
    /// assert_eq!(conflicts[0].id, 1);
    /// assert_eq!(conflicts[0].creature_names.len(), 2);
    /// ```
    pub fn check_conflicts(&self) -> Vec<IdConflict> {
        let mut conflicts = Vec::new();

        for (id, names) in &self.id_to_names {
            if names.len() > 1 {
                conflicts.push(IdConflict {
                    id: *id,
                    creature_names: names.clone(),
                    category: CreatureCategory::from_id(*id),
                });
            }
        }

        conflicts.sort_by_key(|c| c.id);
        conflicts
    }

    /// Suggest automatic ID reassignments to resolve conflicts and fix category mismatches
    ///
    /// # Arguments
    ///
    /// * `registry` - The creature registry to analyze
    /// * `category` - Optional category to filter reassignments (None = all categories)
    ///
    /// # Returns
    ///
    /// Vector of suggested ID changes with reasons
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creature_id_manager::CreatureIdManager;
    /// use antares::domain::visual::CreatureReference;
    ///
    /// let mut manager = CreatureIdManager::new();
    /// let refs = vec![
    ///     CreatureReference { id: 51, name: "Goblin".to_string(), filepath: "".to_string() },
    /// ];
    /// manager.update_from_registry(&refs);
    ///
    /// // Goblin ID 51 is in NPC range but should be in Monster range
    /// let changes = manager.auto_reassign_ids(&refs, None);
    /// // Note: This won't suggest changes for single items, only conflicts
    /// ```
    pub fn auto_reassign_ids(
        &self,
        registry: &[CreatureReference],
        category: Option<CreatureCategory>,
    ) -> Vec<IdChange> {
        let mut changes = Vec::new();
        let mut temp_used_ids = self.used_ids.clone();
        let mut seen_ids: HashSet<CreatureId> = HashSet::new();

        for reference in registry {
            // Skip if filtering by category and this doesn't match
            if let Some(cat) = category {
                if CreatureCategory::from_id(reference.id) != cat {
                    continue;
                }
            }

            // Check if this ID has been seen before (is a duplicate)
            let is_duplicate = seen_ids.contains(&reference.id);

            if is_duplicate {
                // Find next available ID in the correct category
                let correct_category = CreatureCategory::from_id(reference.id);
                if let Some(new_id) =
                    self.find_next_available_in_category(correct_category, &temp_used_ids)
                {
                    changes.push(IdChange {
                        old_id: reference.id,
                        new_id,
                        creature_name: reference.name.clone(),
                        reason: format!(
                            "Resolving duplicate ID {} in category {}",
                            reference.id,
                            correct_category.display_name()
                        ),
                    });
                    temp_used_ids.insert(new_id);
                }
            } else {
                // Mark this ID as seen
                seen_ids.insert(reference.id);
            }
        }

        changes
    }

    /// Find gaps (unused IDs) in a category's range
    ///
    /// # Arguments
    ///
    /// * `category` - The category to search for gaps
    ///
    /// # Returns
    ///
    /// Vector of unused IDs in the category's range
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creature_id_manager::{CreatureIdManager, CreatureCategory};
    /// use antares::domain::visual::CreatureReference;
    ///
    /// let mut manager = CreatureIdManager::new();
    /// let refs = vec![
    ///     CreatureReference { id: 1, name: "Goblin".to_string(), filepath: "".to_string() },
    ///     CreatureReference { id: 3, name: "Orc".to_string(), filepath: "".to_string() },
    /// ];
    /// manager.update_from_registry(&refs);
    ///
    /// let gaps = manager.find_gaps(CreatureCategory::Monsters);
    /// assert!(gaps.contains(&2));
    /// assert_eq!(gaps.len(), 48); // 50 total - 2 used = 48 gaps
    /// ```
    pub fn find_gaps(&self, category: CreatureCategory) -> Vec<CreatureId> {
        let range = category.id_range();
        let mut gaps = Vec::new();

        for id in range {
            if !self.used_ids.contains(&id) {
                gaps.push(id);
            }
        }

        gaps
    }

    /// Check if an ID is currently in use
    ///
    /// # Arguments
    ///
    /// * `id` - The ID to check
    ///
    /// # Returns
    ///
    /// `true` if the ID is in use, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creature_id_manager::CreatureIdManager;
    /// use antares::domain::visual::CreatureReference;
    ///
    /// let mut manager = CreatureIdManager::new();
    /// let refs = vec![
    ///     CreatureReference { id: 1, name: "Goblin".to_string(), filepath: "".to_string() },
    /// ];
    /// manager.update_from_registry(&refs);
    ///
    /// assert!(manager.is_id_used(1));
    /// assert!(!manager.is_id_used(2));
    /// ```
    pub fn is_id_used(&self, id: CreatureId) -> bool {
        self.used_ids.contains(&id)
    }

    /// Get the count of used IDs
    ///
    /// # Returns
    ///
    /// The number of IDs currently in use
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creature_id_manager::CreatureIdManager;
    /// use antares::domain::visual::CreatureReference;
    ///
    /// let mut manager = CreatureIdManager::new();
    /// assert_eq!(manager.used_id_count(), 0);
    ///
    /// let refs = vec![
    ///     CreatureReference { id: 1, name: "Goblin".to_string(), filepath: "".to_string() },
    /// ];
    /// manager.update_from_registry(&refs);
    /// assert_eq!(manager.used_id_count(), 1);
    /// ```
    pub fn used_id_count(&self) -> usize {
        self.used_ids.len()
    }

    /// Get statistics for a specific category
    ///
    /// # Arguments
    ///
    /// * `category` - The category to get statistics for
    ///
    /// # Returns
    ///
    /// A tuple of (used_count, total_capacity, first_gap_id)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::creature_id_manager::{CreatureIdManager, CreatureCategory};
    /// use antares::domain::visual::CreatureReference;
    ///
    /// let mut manager = CreatureIdManager::new();
    /// let refs = vec![
    ///     CreatureReference { id: 1, name: "Goblin".to_string(), filepath: "".to_string() },
    /// ];
    /// manager.update_from_registry(&refs);
    ///
    /// let (used, total, first_gap) = manager.category_stats(CreatureCategory::Monsters);
    /// assert_eq!(used, 1);
    /// assert_eq!(total, 50);
    /// assert_eq!(first_gap, Some(2));
    /// ```
    pub fn category_stats(&self, category: CreatureCategory) -> (usize, usize, Option<CreatureId>) {
        let range = category.id_range();
        let total = if category == CreatureCategory::Custom {
            1000 // Reasonable upper bound for display
        } else {
            (range.end - range.start) as usize
        };

        let used = range
            .clone()
            .filter(|id| self.used_ids.contains(id))
            .count();

        let first_gap = range.clone().find(|id| !self.used_ids.contains(id));

        (used, total, first_gap)
    }

    /// Helper function to find next available ID in a category
    fn find_next_available_in_category(
        &self,
        category: CreatureCategory,
        used_ids: &HashSet<CreatureId>,
    ) -> Option<CreatureId> {
        category.id_range().find(|id| !used_ids.contains(id))
    }
}

impl Default for CreatureIdManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_reference(id: CreatureId, name: &str) -> CreatureReference {
        CreatureReference {
            id,
            name: name.to_string(),
            filepath: format!("creatures/{}.ron", name.to_lowercase()),
        }
    }

    #[test]
    fn test_category_id_range() {
        assert_eq!(CreatureCategory::Monsters.id_range(), 1..51);
        assert_eq!(CreatureCategory::Npcs.id_range(), 51..101);
        assert_eq!(CreatureCategory::Templates.id_range(), 101..151);
        assert_eq!(CreatureCategory::Variants.id_range(), 151..201);
        assert!(CreatureCategory::Custom.id_range().contains(&201));
        assert!(CreatureCategory::Custom.id_range().contains(&10000));
    }

    #[test]
    fn test_category_from_id() {
        assert_eq!(CreatureCategory::from_id(1), CreatureCategory::Monsters);
        assert_eq!(CreatureCategory::from_id(50), CreatureCategory::Monsters);
        assert_eq!(CreatureCategory::from_id(51), CreatureCategory::Npcs);
        assert_eq!(CreatureCategory::from_id(100), CreatureCategory::Npcs);
        assert_eq!(CreatureCategory::from_id(101), CreatureCategory::Templates);
        assert_eq!(CreatureCategory::from_id(150), CreatureCategory::Templates);
        assert_eq!(CreatureCategory::from_id(151), CreatureCategory::Variants);
        assert_eq!(CreatureCategory::from_id(200), CreatureCategory::Variants);
        assert_eq!(CreatureCategory::from_id(201), CreatureCategory::Custom);
        assert_eq!(CreatureCategory::from_id(9999), CreatureCategory::Custom);
    }

    #[test]
    fn test_category_display_name() {
        assert_eq!(CreatureCategory::Monsters.display_name(), "Monsters");
        assert_eq!(CreatureCategory::Npcs.display_name(), "NPCs");
        assert_eq!(CreatureCategory::Templates.display_name(), "Templates");
        assert_eq!(CreatureCategory::Variants.display_name(), "Variants");
        assert_eq!(CreatureCategory::Custom.display_name(), "Custom");
    }

    #[test]
    fn test_category_color() {
        let color = CreatureCategory::Monsters.color();
        assert_eq!(color.len(), 3);
        assert!(color[0] >= 0.0 && color[0] <= 1.0);
        assert!(color[1] >= 0.0 && color[1] <= 1.0);
        assert!(color[2] >= 0.0 && color[2] <= 1.0);
    }

    #[test]
    fn test_new_manager_empty() {
        let manager = CreatureIdManager::new();
        assert_eq!(manager.used_id_count(), 0);
        assert!(!manager.is_id_used(1));
    }

    #[test]
    fn test_update_from_registry() {
        let mut manager = CreatureIdManager::new();
        let refs = vec![
            create_test_reference(1, "Goblin"),
            create_test_reference(2, "Orc"),
            create_test_reference(51, "Villager"),
        ];

        manager.update_from_registry(&refs);

        assert_eq!(manager.used_id_count(), 3);
        assert!(manager.is_id_used(1));
        assert!(manager.is_id_used(2));
        assert!(manager.is_id_used(51));
        assert!(!manager.is_id_used(3));
    }

    #[test]
    fn test_suggest_next_id_empty() {
        let manager = CreatureIdManager::new();
        assert_eq!(manager.suggest_next_id(CreatureCategory::Monsters), 1);
        assert_eq!(manager.suggest_next_id(CreatureCategory::Npcs), 51);
        assert_eq!(manager.suggest_next_id(CreatureCategory::Templates), 101);
    }

    #[test]
    fn test_suggest_next_id_with_used() {
        let mut manager = CreatureIdManager::new();
        let refs = vec![
            create_test_reference(1, "Goblin"),
            create_test_reference(2, "Orc"),
        ];
        manager.update_from_registry(&refs);

        assert_eq!(manager.suggest_next_id(CreatureCategory::Monsters), 3);
        assert_eq!(manager.suggest_next_id(CreatureCategory::Npcs), 51);
    }

    #[test]
    fn test_suggest_next_id_with_gap() {
        let mut manager = CreatureIdManager::new();
        let refs = vec![
            create_test_reference(1, "Goblin"),
            create_test_reference(3, "Orc"),
        ];
        manager.update_from_registry(&refs);

        assert_eq!(manager.suggest_next_id(CreatureCategory::Monsters), 2);
    }

    #[test]
    fn test_validate_id_success() {
        let mut manager = CreatureIdManager::new();
        let refs = vec![create_test_reference(1, "Goblin")];
        manager.update_from_registry(&refs);

        assert!(manager.validate_id(2, CreatureCategory::Monsters).is_ok());
        assert!(manager.validate_id(51, CreatureCategory::Npcs).is_ok());
    }

    #[test]
    fn test_validate_id_duplicate() {
        let mut manager = CreatureIdManager::new();
        let refs = vec![create_test_reference(1, "Goblin")];
        manager.update_from_registry(&refs);

        let result = manager.validate_id(1, CreatureCategory::Monsters);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            IdError::DuplicateId { id: 1 }
        ));
    }

    #[test]
    fn test_validate_id_out_of_range() {
        let manager = CreatureIdManager::new();

        let result = manager.validate_id(51, CreatureCategory::Monsters);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IdError::OutOfRange { .. }));
    }

    #[test]
    fn test_check_conflicts_none() {
        let mut manager = CreatureIdManager::new();
        let refs = vec![
            create_test_reference(1, "Goblin"),
            create_test_reference(2, "Orc"),
        ];
        manager.update_from_registry(&refs);

        let conflicts = manager.check_conflicts();
        assert_eq!(conflicts.len(), 0);
    }

    #[test]
    fn test_check_conflicts_duplicate() {
        let mut manager = CreatureIdManager::new();
        let refs = vec![
            create_test_reference(1, "Goblin"),
            create_test_reference(1, "Orc"),
        ];
        manager.update_from_registry(&refs);

        let conflicts = manager.check_conflicts();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].id, 1);
        assert_eq!(conflicts[0].creature_names.len(), 2);
        assert!(conflicts[0].creature_names.contains(&"Goblin".to_string()));
        assert!(conflicts[0].creature_names.contains(&"Orc".to_string()));
    }

    #[test]
    fn test_find_gaps() {
        let mut manager = CreatureIdManager::new();
        let refs = vec![
            create_test_reference(1, "Goblin"),
            create_test_reference(3, "Orc"),
            create_test_reference(5, "Troll"),
        ];
        manager.update_from_registry(&refs);

        let gaps = manager.find_gaps(CreatureCategory::Monsters);
        assert!(gaps.contains(&2));
        assert!(gaps.contains(&4));
        assert!(!gaps.contains(&1));
        assert!(!gaps.contains(&3));
        assert!(!gaps.contains(&5));
    }

    #[test]
    fn test_category_stats() {
        let mut manager = CreatureIdManager::new();
        let refs = vec![
            create_test_reference(1, "Goblin"),
            create_test_reference(2, "Orc"),
        ];
        manager.update_from_registry(&refs);

        let (used, total, first_gap) = manager.category_stats(CreatureCategory::Monsters);
        assert_eq!(used, 2);
        assert_eq!(total, 50);
        assert_eq!(first_gap, Some(3));
    }

    #[test]
    fn test_category_stats_no_gaps() {
        let mut manager = CreatureIdManager::new();
        let refs: Vec<_> = (1..=50)
            .map(|i| create_test_reference(i, &format!("Creature{}", i)))
            .collect();
        manager.update_from_registry(&refs);

        let (used, total, first_gap) = manager.category_stats(CreatureCategory::Monsters);
        assert_eq!(used, 50);
        assert_eq!(total, 50);
        assert_eq!(first_gap, None);
    }

    #[test]
    fn test_auto_reassign_ids_duplicates() {
        let mut manager = CreatureIdManager::new();
        let refs = vec![
            create_test_reference(1, "Goblin"),
            create_test_reference(1, "Orc"),
        ];
        manager.update_from_registry(&refs);

        let changes = manager.auto_reassign_ids(&refs, None);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].old_id, 1);
        assert_eq!(changes[0].new_id, 2);
    }

    #[test]
    fn test_default_trait() {
        let manager = CreatureIdManager::default();
        assert_eq!(manager.used_id_count(), 0);
    }
}
