// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Quest editor helper module
//!
//! This module provides tools for quest editing workflows including:
//! - Content browsing (items, monsters, NPCs, maps)
//! - Quest validation (cross-references, stage flow)
//! - Smart ID suggestions
//! - Quest template generation
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sdk_and_campaign_architecture.md` Phase 5 for specifications.
//!
//! # Examples
//!
//! ```no_run
//! use antares::sdk::quest_editor::*;
//! use antares::sdk::database::ContentDatabase;
//! use antares::domain::quest::{Quest, QuestStage, QuestObjective};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load content database
//! let db = ContentDatabase::load_core("data")?;
//!
//! // Create a quest
//! let mut quest = Quest::new(1, "Dragon Hunt", "Slay the dragon");
//!
//! // Add a stage with validated monster reference
//! let mut stage = QuestStage::new(1, "Kill the Dragon");
//! stage.add_objective(QuestObjective::KillMonsters {
//!     monster_id: 99,
//!     quantity: 1,
//! });
//!
//! quest.add_stage(stage);
//!
//! // Validate quest
//! let errors = validate_quest(&quest, &db);
//! if errors.is_empty() {
//!     println!("Quest is valid!");
//! }
//! # Ok(())
//! # }
//! ```

use crate::domain::quest::{Quest, QuestId, QuestObjective, QuestReward};
use crate::domain::types::{ItemId, MapId, MonsterId};
use crate::sdk::database::ContentDatabase;

// ===== Content Browsing =====

/// Browse all available items in the database
///
/// Returns a list of (ItemId, name) tuples for all items.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::quest_editor::browse_items;
/// use antares::sdk::database::ContentDatabase;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let db = ContentDatabase::load_core("data")?;
/// let items = browse_items(&db);
/// for (id, name) in items {
///     println!("Item {}: {}", id, name);
/// }
/// # Ok(())
/// # }
/// ```
pub fn browse_items(db: &ContentDatabase) -> Vec<(ItemId, String)> {
    db.items
        .all_items()
        .iter()
        .map(|item| (item.id, item.name.clone()))
        .collect()
}

/// Browse all available monsters in the database
///
/// Returns a list of (MonsterId, name) tuples for all monsters.
pub fn browse_monsters(db: &ContentDatabase) -> Vec<(MonsterId, String)> {
    db.monsters
        .all_monsters()
        .iter()
        .map(|id| {
            let name = db
                .monsters
                .get_monster(*id)
                .map(|m| m.name.clone())
                .unwrap_or_else(|| format!("Monster {}", id));
            (*id, name)
        })
        .collect()
}

/// Browse all available maps in the database
///
/// Returns a list of (MapId, name) tuples for all maps.
pub fn browse_maps(db: &ContentDatabase) -> Vec<(MapId, String)> {
    db.maps
        .all_maps()
        .iter()
        .map(|id| {
            let name = db
                .maps
                .get_map(*id)
                .map(|m| format!("Map {} ({}x{})", m.id, m.width, m.height))
                .unwrap_or_else(|| format!("Map {}", id));
            (*id, name)
        })
        .collect()
}

/// Browse all available quests in the database
///
/// Returns a list of (QuestId, name) tuples for all quests.
pub fn browse_quests(db: &ContentDatabase) -> Vec<(QuestId, String)> {
    db.quests
        .all_quests()
        .iter()
        .map(|id| {
            let name = db
                .quests
                .get_quest(*id)
                .map(|q| q.name.clone())
                .unwrap_or_else(|| format!("Quest {}", id));
            (*id, name)
        })
        .collect()
}

// ===== ID Validation =====

/// Check if an item ID is valid
pub fn is_valid_item_id(db: &ContentDatabase, item_id: &ItemId) -> bool {
    db.items.has_item(item_id)
}

/// Check if a monster ID is valid
pub fn is_valid_monster_id(db: &ContentDatabase, monster_id: &MonsterId) -> bool {
    db.monsters.has_monster(monster_id)
}

/// Check if a map ID is valid
pub fn is_valid_map_id(db: &ContentDatabase, map_id: &MapId) -> bool {
    db.maps.has_map(map_id)
}

/// Check if a quest ID is valid
pub fn is_valid_quest_id(db: &ContentDatabase, quest_id: &QuestId) -> bool {
    db.quests.has_quest(quest_id)
}

// ===== Smart ID Suggestions =====

/// Suggest item IDs based on partial name match
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::quest_editor::suggest_item_ids;
/// use antares::sdk::database::ContentDatabase;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let db = ContentDatabase::load_core("data")?;
/// let suggestions = suggest_item_ids(&db, "sword");
/// for (id, name) in suggestions {
///     println!("Found: {} (ID: {})", name, id);
/// }
/// # Ok(())
/// # }
/// ```
pub fn suggest_item_ids(db: &ContentDatabase, partial_name: &str) -> Vec<(ItemId, String)> {
    let search = partial_name.to_lowercase();
    browse_items(db)
        .into_iter()
        .filter(|(_, name)| name.to_lowercase().contains(&search))
        .collect()
}

/// Suggest monster IDs based on partial name match
pub fn suggest_monster_ids(db: &ContentDatabase, partial_name: &str) -> Vec<(MonsterId, String)> {
    let search = partial_name.to_lowercase();
    browse_monsters(db)
        .into_iter()
        .filter(|(_, name)| name.to_lowercase().contains(&search))
        .collect()
}

/// Suggest map IDs based on partial name match
pub fn suggest_map_ids(db: &ContentDatabase, partial_name: &str) -> Vec<(MapId, String)> {
    let search = partial_name.to_lowercase();
    browse_maps(db)
        .into_iter()
        .filter(|(_, name)| name.to_lowercase().contains(&search))
        .collect()
}

/// Suggest quest IDs based on partial name match
pub fn suggest_quest_ids(db: &ContentDatabase, partial_name: &str) -> Vec<(QuestId, String)> {
    let search = partial_name.to_lowercase();
    browse_quests(db)
        .into_iter()
        .filter(|(_, name)| name.to_lowercase().contains(&search))
        .collect()
}

// ===== Quest Validation =====

/// Validation error for quests
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestValidationError {
    /// Quest has no stages
    NoStages,

    /// Quest stage has no objectives
    StageHasNoObjectives { stage_number: u8 },

    /// Referenced monster ID doesn't exist
    InvalidMonsterId { monster_id: MonsterId },

    /// Referenced item ID doesn't exist
    InvalidItemId { item_id: ItemId },

    /// Referenced map ID doesn't exist
    InvalidMapId { map_id: MapId },

    /// Referenced quest ID doesn't exist (for prerequisites or rewards)
    InvalidQuestId { quest_id: QuestId },

    /// Invalid level requirements (min > max)
    InvalidLevelRequirements { min: u8, max: u8 },

    /// Circular quest dependency
    CircularDependency { quest_id: QuestId },

    /// Stage numbers are not sequential
    NonSequentialStages,

    /// Duplicate stage numbers
    DuplicateStageNumber { stage_number: u8 },
}

impl std::fmt::Display for QuestValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuestValidationError::NoStages => write!(f, "Quest has no stages"),
            QuestValidationError::StageHasNoObjectives { stage_number } => {
                write!(f, "Stage {} has no objectives", stage_number)
            }
            QuestValidationError::InvalidMonsterId { monster_id } => {
                write!(f, "Invalid monster ID: {}", monster_id)
            }
            QuestValidationError::InvalidItemId { item_id } => {
                write!(f, "Invalid item ID: {}", item_id)
            }
            QuestValidationError::InvalidMapId { map_id } => {
                write!(f, "Invalid map ID: {}", map_id)
            }
            QuestValidationError::InvalidQuestId { quest_id } => {
                write!(f, "Invalid quest ID: {}", quest_id)
            }
            QuestValidationError::InvalidLevelRequirements { min, max } => {
                write!(f, "Invalid level requirements: min {} > max {}", min, max)
            }
            QuestValidationError::CircularDependency { quest_id } => {
                write!(f, "Circular quest dependency detected: {}", quest_id)
            }
            QuestValidationError::NonSequentialStages => {
                write!(f, "Quest stages are not sequential")
            }
            QuestValidationError::DuplicateStageNumber { stage_number } => {
                write!(f, "Duplicate stage number: {}", stage_number)
            }
        }
    }
}

impl std::error::Error for QuestValidationError {}

/// Validate a quest against the content database
///
/// Checks:
/// - Quest has at least one stage
/// - All stages have at least one objective
/// - All referenced IDs (monsters, items, maps, quests) exist
/// - Level requirements are valid
/// - Stage numbers are sequential
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::quest_editor::validate_quest;
/// use antares::sdk::database::ContentDatabase;
/// use antares::domain::quest::{Quest, QuestStage, QuestObjective};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let db = ContentDatabase::load_core("data")?;
/// let mut quest = Quest::new(1, "Test", "Test quest");
///
/// let mut stage = QuestStage::new(1, "Stage 1");
/// stage.add_objective(QuestObjective::KillMonsters {
///     monster_id: 1,
///     quantity: 5,
/// });
/// quest.add_stage(stage);
///
/// let errors = validate_quest(&quest, &db);
/// if !errors.is_empty() {
///     for error in errors {
///         println!("Error: {}", error);
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub fn validate_quest(quest: &Quest, db: &ContentDatabase) -> Vec<QuestValidationError> {
    let mut errors = Vec::new();

    // Check quest has stages
    if quest.stages.is_empty() {
        errors.push(QuestValidationError::NoStages);
        return errors; // Early return if no stages
    }

    // Check level requirements
    if let (Some(min), Some(max)) = (quest.min_level, quest.max_level) {
        if min > max {
            errors.push(QuestValidationError::InvalidLevelRequirements { min, max });
        }
    }

    // Check stage numbers are sequential and unique
    let mut stage_numbers: Vec<u8> = quest.stages.iter().map(|s| s.stage_number).collect();
    stage_numbers.sort_unstable();

    // Check for duplicates
    for i in 1..stage_numbers.len() {
        if stage_numbers[i] == stage_numbers[i - 1] {
            errors.push(QuestValidationError::DuplicateStageNumber {
                stage_number: stage_numbers[i],
            });
        }
    }

    // Check if sequential (1, 2, 3, ...)
    for (idx, &stage_num) in stage_numbers.iter().enumerate() {
        if stage_num != (idx + 1) as u8 {
            errors.push(QuestValidationError::NonSequentialStages);
            break;
        }
    }

    // Validate each stage
    for stage in &quest.stages {
        // Check stage has objectives
        if stage.objectives.is_empty() {
            errors.push(QuestValidationError::StageHasNoObjectives {
                stage_number: stage.stage_number,
            });
        }

        // Validate objective references
        for objective in &stage.objectives {
            match objective {
                QuestObjective::KillMonsters { monster_id, .. } => {
                    if !is_valid_monster_id(db, monster_id) {
                        errors.push(QuestValidationError::InvalidMonsterId {
                            monster_id: *monster_id,
                        });
                    }
                }
                QuestObjective::CollectItems { item_id, .. } => {
                    if !is_valid_item_id(db, item_id) {
                        errors.push(QuestValidationError::InvalidItemId { item_id: *item_id });
                    }
                }
                QuestObjective::ReachLocation { map_id, .. } => {
                    if !is_valid_map_id(db, map_id) {
                        errors.push(QuestValidationError::InvalidMapId { map_id: *map_id });
                    }
                }
                QuestObjective::TalkToNpc { map_id, .. } => {
                    if !is_valid_map_id(db, map_id) {
                        errors.push(QuestValidationError::InvalidMapId { map_id: *map_id });
                    }
                }
                QuestObjective::DeliverItem { item_id, .. } => {
                    if !is_valid_item_id(db, item_id) {
                        errors.push(QuestValidationError::InvalidItemId { item_id: *item_id });
                    }
                }
                QuestObjective::EscortNpc { map_id, .. } => {
                    if !is_valid_map_id(db, map_id) {
                        errors.push(QuestValidationError::InvalidMapId { map_id: *map_id });
                    }
                }
                QuestObjective::CustomFlag { .. } => {
                    // No validation needed for custom flags
                }
            }
        }
    }

    // Validate quest prerequisites
    for &prereq_id in &quest.required_quests {
        if prereq_id == quest.id {
            errors.push(QuestValidationError::CircularDependency {
                quest_id: prereq_id,
            });
        } else if !is_valid_quest_id(db, &prereq_id) {
            errors.push(QuestValidationError::InvalidQuestId {
                quest_id: prereq_id,
            });
        }
    }

    // Validate rewards
    for reward in &quest.rewards {
        match reward {
            QuestReward::Items(items) => {
                for (item_id, _) in items {
                    if !is_valid_item_id(db, item_id) {
                        errors.push(QuestValidationError::InvalidItemId { item_id: *item_id });
                    }
                }
            }
            QuestReward::UnlockQuest(quest_id) => {
                if *quest_id == quest.id {
                    errors.push(QuestValidationError::CircularDependency {
                        quest_id: *quest_id,
                    });
                }
                // Note: Don't validate if quest exists, as it might be added later
            }
            _ => {
                // Other rewards don't need validation
            }
        }
    }

    errors
}

/// Get quest dependency chain
///
/// Returns all quests that must be completed before the given quest,
/// in order (immediate prerequisites first, then their prerequisites, etc.).
///
/// Returns an error if a circular dependency is detected.
pub fn get_quest_dependencies(
    quest_id: QuestId,
    db: &ContentDatabase,
) -> Result<Vec<QuestId>, String> {
    let mut dependencies = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut to_process = vec![quest_id];

    while let Some(current_id) = to_process.pop() {
        if visited.contains(&current_id) {
            return Err(format!(
                "Circular dependency detected at quest {}",
                current_id
            ));
        }
        visited.insert(current_id);

        if let Some(quest) = db.quests.get_quest(current_id) {
            for &prereq_id in &quest.required_quests {
                if !dependencies.contains(&prereq_id) {
                    dependencies.push(prereq_id);
                    to_process.push(prereq_id);
                }
            }
        }
    }

    Ok(dependencies)
}

/// Generate quest summary for display
///
/// Returns a formatted string with quest details.
pub fn generate_quest_summary(quest: &Quest) -> String {
    let mut summary = String::new();

    summary.push_str(&format!("Quest {}: {}\n", quest.id, quest.name));
    summary.push_str(&format!("Description: {}\n", quest.description));

    if quest.is_main_quest {
        summary.push_str("Type: Main Quest\n");
    } else {
        summary.push_str("Type: Side Quest\n");
    }

    if let Some(min) = quest.min_level {
        summary.push_str(&format!("Min Level: {}\n", min));
    }
    if let Some(max) = quest.max_level {
        summary.push_str(&format!("Max Level: {}\n", max));
    }

    summary.push_str(&format!("Stages: {}\n", quest.stages.len()));
    summary.push_str(&format!("Rewards: {}\n", quest.rewards.len()));

    if quest.repeatable {
        summary.push_str("Repeatable: Yes\n");
    }

    if !quest.required_quests.is_empty() {
        summary.push_str(&format!("Prerequisites: {:?}\n", quest.required_quests));
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::quest::{QuestObjective, QuestReward, QuestStage};

    #[test]
    fn test_browse_items_empty_db() {
        let db = ContentDatabase::new();
        let items = browse_items(&db);
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_browse_monsters_empty_db() {
        let db = ContentDatabase::new();
        let monsters = browse_monsters(&db);
        assert_eq!(monsters.len(), 0);
    }

    #[test]
    fn test_validate_quest_no_stages() {
        let db = ContentDatabase::new();
        let quest = Quest::new(1, "Test", "Test quest");

        let errors = validate_quest(&quest, &db);
        assert!(!errors.is_empty());
        assert!(matches!(errors[0], QuestValidationError::NoStages));
    }

    #[test]
    fn test_validate_quest_stage_no_objectives() {
        let db = ContentDatabase::new();
        let mut quest = Quest::new(1, "Test", "Test quest");
        let stage = QuestStage::new(1, "Empty stage");
        quest.add_stage(stage);

        let errors = validate_quest(&quest, &db);
        assert!(errors.iter().any(|e| matches!(
            e,
            QuestValidationError::StageHasNoObjectives { stage_number: 1 }
        )));
    }

    #[test]
    fn test_validate_quest_invalid_level_requirements() {
        let db = ContentDatabase::new();
        let mut quest = Quest::new(1, "Test", "Test quest");
        quest.min_level = Some(10);
        quest.max_level = Some(5); // Invalid: min > max

        let mut stage = QuestStage::new(1, "Stage");
        stage.add_objective(QuestObjective::CustomFlag {
            flag_name: "test".to_string(),
            required_value: true,
        });
        quest.add_stage(stage);

        let errors = validate_quest(&quest, &db);
        assert!(errors.iter().any(|e| matches!(
            e,
            QuestValidationError::InvalidLevelRequirements { min: 10, max: 5 }
        )));
    }

    #[test]
    fn test_validate_quest_non_sequential_stages() {
        let db = ContentDatabase::new();
        let mut quest = Quest::new(1, "Test", "Test quest");

        let mut stage1 = QuestStage::new(1, "Stage 1");
        stage1.add_objective(QuestObjective::CustomFlag {
            flag_name: "test".to_string(),
            required_value: true,
        });

        let mut stage3 = QuestStage::new(3, "Stage 3"); // Skipped stage 2
        stage3.add_objective(QuestObjective::CustomFlag {
            flag_name: "test2".to_string(),
            required_value: true,
        });

        quest.add_stage(stage1);
        quest.add_stage(stage3);

        let errors = validate_quest(&quest, &db);
        assert!(errors
            .iter()
            .any(|e| matches!(e, QuestValidationError::NonSequentialStages)));
    }

    #[test]
    fn test_validate_quest_duplicate_stage_numbers() {
        let db = ContentDatabase::new();
        let mut quest = Quest::new(1, "Test", "Test quest");

        let mut stage1a = QuestStage::new(1, "Stage 1a");
        stage1a.add_objective(QuestObjective::CustomFlag {
            flag_name: "test".to_string(),
            required_value: true,
        });

        let mut stage1b = QuestStage::new(1, "Stage 1b");
        stage1b.add_objective(QuestObjective::CustomFlag {
            flag_name: "test2".to_string(),
            required_value: true,
        });

        quest.add_stage(stage1a);
        quest.add_stage(stage1b);

        let errors = validate_quest(&quest, &db);
        assert!(errors.iter().any(|e| matches!(
            e,
            QuestValidationError::DuplicateStageNumber { stage_number: 1 }
        )));
    }

    #[test]
    fn test_validate_quest_circular_dependency() {
        let db = ContentDatabase::new();
        let mut quest = Quest::new(1, "Test", "Test quest");
        quest.required_quests.push(1); // Self-reference

        let mut stage = QuestStage::new(1, "Stage");
        stage.add_objective(QuestObjective::CustomFlag {
            flag_name: "test".to_string(),
            required_value: true,
        });
        quest.add_stage(stage);

        let errors = validate_quest(&quest, &db);
        assert!(errors
            .iter()
            .any(|e| matches!(e, QuestValidationError::CircularDependency { quest_id: 1 })));
    }

    #[test]
    fn test_validate_valid_quest() {
        let db = ContentDatabase::new();
        let mut quest = Quest::new(1, "Test", "Test quest");

        let mut stage = QuestStage::new(1, "Stage 1");
        stage.add_objective(QuestObjective::CustomFlag {
            flag_name: "test".to_string(),
            required_value: true,
        });
        quest.add_stage(stage);

        let errors = validate_quest(&quest, &db);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_suggest_item_ids() {
        let db = ContentDatabase::new();
        // Empty database returns no suggestions
        let suggestions = suggest_item_ids(&db, "sword");
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_is_valid_monster_id() {
        let db = ContentDatabase::new();
        assert!(!is_valid_monster_id(&db, &99));
    }

    #[test]
    fn test_generate_quest_summary() {
        let mut quest = Quest::new(1, "Epic Quest", "Save the world");
        quest.min_level = Some(10);
        quest.is_main_quest = true;

        let mut stage = QuestStage::new(1, "Stage 1");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 5,
            quantity: 10,
        });
        quest.add_stage(stage);
        quest.add_reward(QuestReward::Gold(100));

        let summary = generate_quest_summary(&quest);
        assert!(summary.contains("Epic Quest"));
        assert!(summary.contains("Main Quest"));
        assert!(summary.contains("Min Level: 10"));
    }

    #[test]
    fn test_get_quest_dependencies_empty() {
        let db = ContentDatabase::new();
        let deps = get_quest_dependencies(1, &db);
        assert!(deps.is_ok());
        assert_eq!(deps.unwrap().len(), 0);
    }
}
