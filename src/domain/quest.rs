// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Quest system domain model
//!
//! This module defines the core quest structures including quests, stages,
//! objectives, and rewards. Quests are story-driven tasks that players can
//! accept and complete for rewards.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sdk_and_campaign_architecture.md` Phase 5 for quest specifications.

use crate::domain::types::{ItemId, MapId, MonsterId, Position};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Quest identifier
pub type QuestId = u16;

/// Quest definition
///
/// Represents a complete quest with multiple stages, objectives, and rewards.
/// Quests can have prerequisites, level requirements, and can be marked as
/// repeatable or one-time only.
///
/// # Examples
///
/// ```
/// use antares::domain::quest::{Quest, QuestStage, QuestObjective, QuestReward};
///
/// let mut quest = Quest::new(
///     1,
///     "The Missing Sword",
///     "Recover the legendary sword stolen by goblins",
/// );
///
/// quest.min_level = Some(5);
/// quest.required_quests = vec![]; // No prerequisites
///
/// // Add a stage
/// let mut stage = QuestStage::new(1, "Find the Goblin Camp");
/// stage.objectives.push(QuestObjective::ReachLocation {
///     map_id: 10,
///     position: antares::domain::types::Position::new(15, 20),
///     radius: 2,
/// });
///
/// quest.stages.push(stage);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    /// Unique quest identifier
    pub id: QuestId,

    /// Quest name displayed to the player
    pub name: String,

    /// Quest description and story
    pub description: String,

    /// Ordered list of quest stages
    pub stages: Vec<QuestStage>,

    /// Rewards given upon quest completion
    pub rewards: Vec<QuestReward>,

    /// Minimum character level required to accept quest
    pub min_level: Option<u8>,

    /// Maximum character level allowed to accept quest
    pub max_level: Option<u8>,

    /// Quest IDs that must be completed before this quest is available
    pub required_quests: Vec<QuestId>,

    /// Whether the quest can be repeated after completion
    pub repeatable: bool,

    /// Whether the quest is part of the main storyline
    pub is_main_quest: bool,

    /// Quest giver NPC ID (optional)
    pub quest_giver_npc: Option<u16>,

    /// Map ID where quest is given
    pub quest_giver_map: Option<MapId>,

    /// Position of quest giver
    pub quest_giver_position: Option<Position>,
}

impl Quest {
    /// Creates a new quest with the given ID, name, and description
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::quest::Quest;
    ///
    /// let quest = Quest::new(1, "Test Quest", "A simple test quest");
    /// assert_eq!(quest.id, 1);
    /// assert_eq!(quest.name, "Test Quest");
    /// assert!(!quest.repeatable);
    /// ```
    pub fn new(id: QuestId, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            description: description.into(),
            stages: Vec::new(),
            rewards: Vec::new(),
            min_level: None,
            max_level: None,
            required_quests: Vec::new(),
            repeatable: false,
            is_main_quest: false,
            quest_giver_npc: None,
            quest_giver_map: None,
            quest_giver_position: None,
        }
    }

    /// Adds a stage to the quest
    pub fn add_stage(&mut self, stage: QuestStage) {
        self.stages.push(stage);
    }

    /// Adds a reward to the quest
    pub fn add_reward(&mut self, reward: QuestReward) {
        self.rewards.push(reward);
    }

    /// Returns the number of stages in the quest
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Returns true if the quest has any stages
    pub fn has_stages(&self) -> bool {
        !self.stages.is_empty()
    }

    /// Returns true if the quest can be accepted by a character of the given level
    pub fn is_available_for_level(&self, level: u8) -> bool {
        if let Some(min) = self.min_level {
            if level < min {
                return false;
            }
        }
        if let Some(max) = self.max_level {
            if level > max {
                return false;
            }
        }
        true
    }
}

/// Quest stage
///
/// Represents a single stage within a quest. Each stage has a name, description,
/// and a list of objectives that must be completed before advancing to the next stage.
///
/// # Examples
///
/// ```
/// use antares::domain::quest::{QuestStage, QuestObjective};
///
/// let mut stage = QuestStage::new(1, "Gather Materials");
/// stage.objectives.push(QuestObjective::CollectItems {
///     item_id: 42,
///     quantity: 5,
/// });
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestStage {
    /// Stage number (1-based)
    pub stage_number: u8,

    /// Stage name
    pub name: String,

    /// Stage description
    pub description: String,

    /// Objectives that must be completed in this stage
    pub objectives: Vec<QuestObjective>,

    /// Whether all objectives must be completed (true) or just one (false)
    pub require_all_objectives: bool,
}

impl QuestStage {
    /// Creates a new quest stage
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::quest::QuestStage;
    ///
    /// let stage = QuestStage::new(1, "Find the Cave");
    /// assert_eq!(stage.stage_number, 1);
    /// assert_eq!(stage.name, "Find the Cave");
    /// assert!(stage.require_all_objectives);
    /// ```
    pub fn new(stage_number: u8, name: impl Into<String>) -> Self {
        Self {
            stage_number,
            name: name.into(),
            description: String::new(),
            objectives: Vec::new(),
            require_all_objectives: true,
        }
    }

    /// Adds an objective to this stage
    pub fn add_objective(&mut self, objective: QuestObjective) {
        self.objectives.push(objective);
    }

    /// Returns the number of objectives in this stage
    pub fn objective_count(&self) -> usize {
        self.objectives.len()
    }
}

/// Quest objective
///
/// Represents a specific task that must be completed as part of a quest stage.
/// Objectives can be of various types: killing monsters, collecting items,
/// reaching locations, talking to NPCs, or custom flag-based objectives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuestObjective {
    /// Kill a specific number of a monster type
    KillMonsters {
        /// Monster type ID
        monster_id: MonsterId,
        /// Number of monsters to kill
        quantity: u16,
    },

    /// Collect a specific number of items
    CollectItems {
        /// Item type ID
        item_id: ItemId,
        /// Number of items to collect
        quantity: u16,
    },

    /// Reach a specific location on a map
    ReachLocation {
        /// Map ID
        map_id: MapId,
        /// Target position
        position: Position,
        /// Radius around position that counts as "reached"
        radius: u8,
    },

    /// Talk to a specific NPC
    TalkToNpc {
        /// NPC ID
        npc_id: u16,
        /// Map where NPC is located
        map_id: MapId,
    },

    /// Deliver an item to an NPC
    DeliverItem {
        /// Item to deliver
        item_id: ItemId,
        /// NPC to deliver to
        npc_id: u16,
        /// Quantity to deliver
        quantity: u16,
    },

    /// Escort an NPC to a location
    EscortNpc {
        /// NPC to escort
        npc_id: u16,
        /// Destination map
        map_id: MapId,
        /// Destination position
        position: Position,
    },

    /// Custom objective based on game flags
    CustomFlag {
        /// Flag name to check
        flag_name: String,
        /// Required flag value
        required_value: bool,
    },
}

impl QuestObjective {
    /// Returns a human-readable description of the objective
    pub fn description(&self) -> String {
        match self {
            QuestObjective::KillMonsters {
                monster_id,
                quantity,
            } => {
                format!("Kill {} of monster type {}", quantity, monster_id)
            }
            QuestObjective::CollectItems { item_id, quantity } => {
                format!("Collect {} of item {}", quantity, item_id)
            }
            QuestObjective::ReachLocation {
                map_id,
                position,
                radius,
            } => {
                format!(
                    "Reach ({}, {}) on map {} (radius: {})",
                    position.x, position.y, map_id, radius
                )
            }
            QuestObjective::TalkToNpc { npc_id, map_id } => {
                format!("Talk to NPC {} on map {}", npc_id, map_id)
            }
            QuestObjective::DeliverItem {
                item_id,
                npc_id,
                quantity,
            } => {
                format!("Deliver {} of item {} to NPC {}", quantity, item_id, npc_id)
            }
            QuestObjective::EscortNpc {
                npc_id,
                map_id,
                position,
            } => {
                format!(
                    "Escort NPC {} to ({}, {}) on map {}",
                    npc_id, position.x, position.y, map_id
                )
            }
            QuestObjective::CustomFlag {
                flag_name,
                required_value,
            } => {
                format!("Set flag '{}' to {}", flag_name, required_value)
            }
        }
    }
}

/// Quest reward
///
/// Represents rewards given upon quest completion. Multiple reward types
/// can be combined (e.g., experience + gold + items).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuestReward {
    /// Experience points
    Experience(u32),

    /// Gold coins
    Gold(u32),

    /// Items (item ID and quantity)
    Items(Vec<(ItemId, u16)>),

    /// Unlock a new quest
    UnlockQuest(QuestId),

    /// Set a game flag
    SetFlag {
        /// Flag name
        flag_name: String,
        /// Flag value
        value: bool,
    },

    /// Reputation change with a faction
    Reputation {
        /// Faction name
        faction: String,
        /// Reputation change (can be negative)
        change: i16,
    },
}

/// Quest progress tracking
///
/// Tracks the player's progress through a quest, including which stage
/// they're on and progress toward each objective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestProgress {
    /// Quest ID being tracked
    pub quest_id: QuestId,

    /// Current stage number (1-based)
    pub current_stage: u8,

    /// Progress for each objective in the current stage
    /// Map: objective index -> progress value
    pub objective_progress: HashMap<usize, u32>,

    /// Whether the quest is completed
    pub completed: bool,

    /// Whether the quest has been turned in for rewards
    pub turned_in: bool,
}

impl QuestProgress {
    /// Creates new quest progress tracking
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::quest::QuestProgress;
    ///
    /// let progress = QuestProgress::new(1);
    /// assert_eq!(progress.quest_id, 1);
    /// assert_eq!(progress.current_stage, 1);
    /// assert!(!progress.completed);
    /// ```
    pub fn new(quest_id: QuestId) -> Self {
        Self {
            quest_id,
            current_stage: 1,
            objective_progress: HashMap::new(),
            completed: false,
            turned_in: false,
        }
    }

    /// Updates progress for a specific objective
    pub fn update_objective(&mut self, objective_index: usize, progress: u32) {
        self.objective_progress.insert(objective_index, progress);
    }

    /// Gets progress for a specific objective
    pub fn get_objective_progress(&self, objective_index: usize) -> u32 {
        *self.objective_progress.get(&objective_index).unwrap_or(&0)
    }

    /// Advances to the next stage
    pub fn advance_stage(&mut self) {
        self.current_stage += 1;
        self.objective_progress.clear();
    }

    /// Marks the quest as completed
    pub fn complete(&mut self) {
        self.completed = true;
    }

    /// Marks the quest as turned in for rewards
    pub fn turn_in(&mut self) {
        self.turned_in = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quest_creation() {
        let quest = Quest::new(1, "Test Quest", "A test quest description");
        assert_eq!(quest.id, 1);
        assert_eq!(quest.name, "Test Quest");
        assert_eq!(quest.description, "A test quest description");
        assert_eq!(quest.stages.len(), 0);
        assert!(!quest.repeatable);
    }

    #[test]
    fn test_quest_add_stage() {
        let mut quest = Quest::new(1, "Test", "Description");
        let stage = QuestStage::new(1, "Stage 1");

        quest.add_stage(stage);
        assert_eq!(quest.stage_count(), 1);
        assert!(quest.has_stages());
    }

    #[test]
    fn test_quest_add_reward() {
        let mut quest = Quest::new(1, "Test", "Description");
        quest.add_reward(QuestReward::Gold(100));
        quest.add_reward(QuestReward::Experience(500));

        assert_eq!(quest.rewards.len(), 2);
    }

    #[test]
    fn test_quest_level_requirements() {
        let mut quest = Quest::new(1, "Test", "Description");
        quest.min_level = Some(5);
        quest.max_level = Some(10);

        assert!(!quest.is_available_for_level(3)); // Too low
        assert!(quest.is_available_for_level(5)); // Min level
        assert!(quest.is_available_for_level(7)); // Within range
        assert!(quest.is_available_for_level(10)); // Max level
        assert!(!quest.is_available_for_level(12)); // Too high
    }

    #[test]
    fn test_quest_stage_creation() {
        let stage = QuestStage::new(1, "Find the Cave");
        assert_eq!(stage.stage_number, 1);
        assert_eq!(stage.name, "Find the Cave");
        assert_eq!(stage.objectives.len(), 0);
        assert!(stage.require_all_objectives);
    }

    #[test]
    fn test_quest_stage_add_objective() {
        let mut stage = QuestStage::new(1, "Stage");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 5,
            quantity: 10,
        });

        assert_eq!(stage.objective_count(), 1);
    }

    #[test]
    fn test_quest_objective_descriptions() {
        let obj1 = QuestObjective::KillMonsters {
            monster_id: 5,
            quantity: 10,
        };
        assert!(obj1.description().contains("Kill 10"));

        let obj2 = QuestObjective::CollectItems {
            item_id: 42,
            quantity: 5,
        };
        assert!(obj2.description().contains("Collect 5"));

        let obj3 = QuestObjective::TalkToNpc {
            npc_id: 7,
            map_id: 3,
        };
        assert!(obj3.description().contains("Talk to NPC 7"));
    }

    #[test]
    fn test_quest_progress_creation() {
        let progress = QuestProgress::new(1);
        assert_eq!(progress.quest_id, 1);
        assert_eq!(progress.current_stage, 1);
        assert!(!progress.completed);
        assert!(!progress.turned_in);
    }

    #[test]
    fn test_quest_progress_update_objective() {
        let mut progress = QuestProgress::new(1);
        progress.update_objective(0, 5);
        progress.update_objective(1, 10);

        assert_eq!(progress.get_objective_progress(0), 5);
        assert_eq!(progress.get_objective_progress(1), 10);
        assert_eq!(progress.get_objective_progress(2), 0); // Not set
    }

    #[test]
    fn test_quest_progress_advance_stage() {
        let mut progress = QuestProgress::new(1);
        progress.update_objective(0, 10);
        progress.update_objective(1, 5);

        progress.advance_stage();

        assert_eq!(progress.current_stage, 2);
        assert_eq!(progress.objective_progress.len(), 0); // Cleared
    }

    #[test]
    fn test_quest_progress_completion() {
        let mut progress = QuestProgress::new(1);

        assert!(!progress.completed);
        assert!(!progress.turned_in);

        progress.complete();
        assert!(progress.completed);
        assert!(!progress.turned_in);

        progress.turn_in();
        assert!(progress.turned_in);
    }

    #[test]
    fn test_complex_quest_with_multiple_stages() {
        let mut quest = Quest::new(1, "Epic Quest", "An epic multi-stage quest");
        quest.min_level = Some(10);
        quest.is_main_quest = true;

        // Stage 1: Gather materials
        let mut stage1 = QuestStage::new(1, "Gather Materials");
        stage1.add_objective(QuestObjective::CollectItems {
            item_id: 10,
            quantity: 5,
        });
        stage1.add_objective(QuestObjective::CollectItems {
            item_id: 11,
            quantity: 3,
        });
        quest.add_stage(stage1);

        // Stage 2: Defeat boss
        let mut stage2 = QuestStage::new(2, "Defeat the Boss");
        stage2.add_objective(QuestObjective::KillMonsters {
            monster_id: 99,
            quantity: 1,
        });
        quest.add_stage(stage2);

        // Stage 3: Return to quest giver
        let mut stage3 = QuestStage::new(3, "Return to Village");
        stage3.add_objective(QuestObjective::TalkToNpc {
            npc_id: 1,
            map_id: 1,
        });
        quest.add_stage(stage3);

        // Add rewards
        quest.add_reward(QuestReward::Experience(1000));
        quest.add_reward(QuestReward::Gold(500));
        quest.add_reward(QuestReward::Items(vec![(42, 1)]));

        assert_eq!(quest.stage_count(), 3);
        assert_eq!(quest.rewards.len(), 3);
        assert!(quest.is_main_quest);
    }
}
