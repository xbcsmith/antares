// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Quest Designer for Campaign Builder
//!
//! This module provides a visual quest designer with support for creating,
//! editing, and managing quests with multiple stages, objectives, and rewards.
//!
//! # Features
//!
//! - Quest list view with search and filtering
//! - Multi-stage quest editor with ordered stages
//! - Objective builder with multiple objective types
//! - Reward configuration
//! - Prerequisite chain management
//! - Quest validation and preview

use antares::domain::quest::{Quest, QuestId, QuestObjective, QuestStage};
use antares::domain::types::{ItemId, MapId, MonsterId, Position};
use serde::{Deserialize, Serialize};

/// Editor state for quest designer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestEditorState {
    /// All quests being edited
    pub quests: Vec<Quest>,

    /// Currently selected quest index
    pub selected_quest: Option<usize>,

    /// Currently selected stage index within selected quest
    pub selected_stage: Option<usize>,

    /// Currently selected objective index within selected stage
    pub selected_objective: Option<usize>,

    /// Quest editor mode
    pub mode: QuestEditorMode,

    /// Edit buffer for quest form fields
    pub quest_buffer: QuestEditBuffer,

    /// Edit buffer for stage form fields
    pub stage_buffer: StageEditBuffer,

    /// Edit buffer for objective form fields
    pub objective_buffer: ObjectiveEditBuffer,

    /// Edit buffer for reward form fields
    pub reward_buffer: RewardEditBuffer,

    /// Quest search/filter string
    pub search_filter: String,

    /// Unsaved changes flag
    pub has_unsaved_changes: bool,

    /// Validation errors for current quest
    pub validation_errors: Vec<String>,
}

/// Quest editor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestEditorMode {
    /// Viewing list of quests
    List,
    /// Creating new quest
    Creating,
    /// Editing existing quest
    Editing,
}

/// Buffer for quest form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestEditBuffer {
    pub id: String,
    pub name: String,
    pub description: String,
    pub min_level: String,
    pub max_level: String,
    pub repeatable: bool,
    pub is_main_quest: bool,
    pub quest_giver_npc: String,
    pub quest_giver_map: String,
    pub quest_giver_x: String,
    pub quest_giver_y: String,
}

impl Default for QuestEditBuffer {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            min_level: "1".to_string(),
            max_level: "30".to_string(),
            repeatable: false,
            is_main_quest: false,
            quest_giver_npc: String::new(),
            quest_giver_map: String::new(),
            quest_giver_x: String::new(),
            quest_giver_y: String::new(),
        }
    }
}

/// Buffer for stage form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageEditBuffer {
    pub number: String,
    pub name: String,
    pub description: String,
    pub require_all: bool,
}

impl Default for StageEditBuffer {
    fn default() -> Self {
        Self {
            number: "1".to_string(),
            name: String::new(),
            description: String::new(),
            require_all: false,
        }
    }
}

/// Buffer for objective form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveEditBuffer {
    pub objective_type: ObjectiveType,
    pub monster_id: String,
    pub item_id: String,
    pub quantity: String,
    pub map_id: String,
    pub location_x: String,
    pub location_y: String,
    pub location_radius: String,
    pub npc_id: String,
    pub flag_name: String,
    pub flag_value: bool,
}

/// Objective type selector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectiveType {
    KillMonsters,
    CollectItems,
    ReachLocation,
    TalkToNpc,
    DeliverItem,
    EscortNpc,
    CustomFlag,
}

impl ObjectiveType {
    pub fn as_str(&self) -> &str {
        match self {
            ObjectiveType::KillMonsters => "Kill Monsters",
            ObjectiveType::CollectItems => "Collect Items",
            ObjectiveType::ReachLocation => "Reach Location",
            ObjectiveType::TalkToNpc => "Talk to NPC",
            ObjectiveType::DeliverItem => "Deliver Item",
            ObjectiveType::EscortNpc => "Escort NPC",
            ObjectiveType::CustomFlag => "Custom Flag",
        }
    }
}

impl Default for ObjectiveEditBuffer {
    fn default() -> Self {
        Self {
            objective_type: ObjectiveType::KillMonsters,
            monster_id: String::new(),
            item_id: String::new(),
            quantity: "1".to_string(),
            map_id: String::new(),
            location_x: "0".to_string(),
            location_y: "0".to_string(),
            location_radius: "1".to_string(),
            npc_id: String::new(),
            flag_name: String::new(),
            flag_value: false,
        }
    }
}

/// Buffer for reward form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardEditBuffer {
    pub reward_type: RewardType,
    pub experience: String,
    pub gold: String,
    pub item_id: String,
    pub unlock_quest_id: String,
    pub flag_name: String,
    pub flag_value: bool,
    pub faction_name: String,
    pub reputation_change: String,
}

/// Reward type selector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RewardType {
    Experience,
    Gold,
    Items,
    UnlockQuest,
    SetFlag,
    Reputation,
}

impl RewardType {
    pub fn as_str(&self) -> &str {
        match self {
            RewardType::Experience => "Experience",
            RewardType::Gold => "Gold",
            RewardType::Items => "Items",
            RewardType::UnlockQuest => "Unlock Quest",
            RewardType::SetFlag => "Set Flag",
            RewardType::Reputation => "Reputation",
        }
    }
}

impl Default for RewardEditBuffer {
    fn default() -> Self {
        Self {
            reward_type: RewardType::Gold,
            experience: "100".to_string(),
            gold: "50".to_string(),
            item_id: String::new(),
            unlock_quest_id: String::new(),
            flag_name: String::new(),
            flag_value: false,
            faction_name: String::new(),
            reputation_change: "10".to_string(),
        }
    }
}

impl Default for QuestEditorState {
    fn default() -> Self {
        Self {
            quests: Vec::new(),
            selected_quest: None,
            selected_stage: None,
            selected_objective: None,
            mode: QuestEditorMode::List,
            quest_buffer: QuestEditBuffer::default(),
            stage_buffer: StageEditBuffer::default(),
            objective_buffer: ObjectiveEditBuffer::default(),
            reward_buffer: RewardEditBuffer::default(),
            search_filter: String::new(),
            has_unsaved_changes: false,
            validation_errors: Vec::new(),
        }
    }
}

impl QuestEditorState {
    /// Create a new quest editor state
    pub fn new() -> Self {
        Self::default()
    }

    /// Load quests from data
    pub fn load_quests(&mut self, quests: Vec<Quest>) {
        self.quests = quests;
        self.has_unsaved_changes = false;
        self.validation_errors.clear();
    }

    /// Get filtered quests based on search
    pub fn filtered_quests(&self) -> Vec<(usize, &Quest)> {
        self.quests
            .iter()
            .enumerate()
            .filter(|(_, quest)| {
                self.search_filter.is_empty()
                    || quest
                        .name
                        .to_lowercase()
                        .contains(&self.search_filter.to_lowercase())
                    || quest.id.to_string().contains(&self.search_filter)
            })
            .collect()
    }

    /// Start creating a new quest
    pub fn start_new_quest(&mut self) {
        self.mode = QuestEditorMode::Creating;
        self.quest_buffer = QuestEditBuffer::default();
        self.selected_stage = None;
        self.selected_objective = None;
        self.validation_errors.clear();
    }

    /// Start editing the selected quest
    pub fn start_edit_quest(&mut self, index: usize) {
        if index < self.quests.len() {
            self.selected_quest = Some(index);
            let quest = &self.quests[index];

            self.quest_buffer = QuestEditBuffer {
                id: quest.id.to_string(),
                name: quest.name.clone(),
                description: quest.description.clone(),
                min_level: quest.min_level.map_or(String::new(), |l| l.to_string()),
                max_level: quest.max_level.map_or(String::new(), |l| l.to_string()),
                repeatable: quest.repeatable,
                is_main_quest: quest.is_main_quest,
                quest_giver_npc: quest
                    .quest_giver_npc
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
                quest_giver_map: quest
                    .quest_giver_map
                    .map_or(String::new(), |m| m.to_string()),
                quest_giver_x: quest
                    .quest_giver_position
                    .as_ref()
                    .map_or(String::new(), |p| p.x.to_string()),
                quest_giver_y: quest
                    .quest_giver_position
                    .as_ref()
                    .map_or(String::new(), |p| p.y.to_string()),
            };

            self.mode = QuestEditorMode::Editing;
            self.selected_stage = None;
            self.selected_objective = None;
            self.validation_errors.clear();
        }
    }

    /// Save current quest being edited
    pub fn save_quest(&mut self) -> Result<(), String> {
        let id = self
            .quest_buffer
            .id
            .parse::<QuestId>()
            .map_err(|_| "Invalid quest ID".to_string())?;

        let min_level = if self.quest_buffer.min_level.is_empty() {
            None
        } else {
            Some(
                self.quest_buffer
                    .min_level
                    .parse::<u8>()
                    .map_err(|_| "Invalid min level".to_string())?,
            )
        };

        let max_level = if self.quest_buffer.max_level.is_empty() {
            None
        } else {
            Some(
                self.quest_buffer
                    .max_level
                    .parse::<u8>()
                    .map_err(|_| "Invalid max level".to_string())?,
            )
        };

        let quest_giver_position = if !self.quest_buffer.quest_giver_x.is_empty()
            && !self.quest_buffer.quest_giver_y.is_empty()
        {
            let x = self
                .quest_buffer
                .quest_giver_x
                .parse::<u32>()
                .map_err(|_| "Invalid quest giver X coordinate".to_string())?;
            let y = self
                .quest_buffer
                .quest_giver_y
                .parse::<u32>()
                .map_err(|_| "Invalid quest giver Y coordinate".to_string())?;
            Some(Position::new(x as i32, y as i32))
        } else {
            None
        };

        let mut quest = if let Some(idx) = self.selected_quest {
            self.quests[idx].clone()
        } else {
            Quest::new(id, &self.quest_buffer.name, &self.quest_buffer.description)
        };

        quest.name = self.quest_buffer.name.clone();
        quest.description = self.quest_buffer.description.clone();
        quest.min_level = min_level;
        quest.max_level = max_level;
        quest.repeatable = self.quest_buffer.repeatable;
        quest.is_main_quest = self.quest_buffer.is_main_quest;
        quest.quest_giver_npc = if self.quest_buffer.quest_giver_npc.is_empty() {
            None
        } else {
            self.quest_buffer.quest_giver_npc.parse::<u16>().ok()
        };
        quest.quest_giver_map = if self.quest_buffer.quest_giver_map.is_empty() {
            None
        } else {
            self.quest_buffer.quest_giver_map.parse::<MapId>().ok()
        };
        quest.quest_giver_position = quest_giver_position;

        if let Some(idx) = self.selected_quest {
            self.quests[idx] = quest;
        } else {
            self.quests.push(quest);
        }

        self.has_unsaved_changes = true;
        self.mode = QuestEditorMode::List;
        self.selected_quest = None;
        Ok(())
    }

    /// Cancel current edit
    pub fn cancel_edit(&mut self) {
        self.mode = QuestEditorMode::List;
        self.selected_quest = None;
        self.selected_stage = None;
        self.selected_objective = None;
        self.quest_buffer = QuestEditBuffer::default();
        self.validation_errors.clear();
    }

    /// Delete quest at index
    pub fn delete_quest(&mut self, index: usize) {
        if index < self.quests.len() {
            self.quests.remove(index);
            self.has_unsaved_changes = true;
        }
    }

    /// Add a new stage to current quest
    pub fn add_stage(&mut self) -> Result<(), String> {
        if self.selected_quest.is_none() {
            return Err("No quest selected".to_string());
        }

        let stage_num = self
            .stage_buffer
            .number
            .parse::<u8>()
            .map_err(|_| "Invalid stage number".to_string())?;

        let stage = QuestStage::new(stage_num, &self.stage_buffer.name);

        if let Some(idx) = self.selected_quest {
            self.quests[idx].add_stage(stage);
            self.has_unsaved_changes = true;
            self.stage_buffer = StageEditBuffer::default();
            self.selected_stage = None;
            Ok(())
        } else {
            Err("No quest selected".to_string())
        }
    }

    /// Add objective to current stage
    pub fn add_objective(&mut self) -> Result<(), String> {
        if let Some(quest_idx) = self.selected_quest {
            if quest_idx >= self.quests.len() {
                return Err("Invalid quest index".to_string());
            }

            let objective = match self.objective_buffer.objective_type {
                ObjectiveType::KillMonsters => {
                    let monster_id = self
                        .objective_buffer
                        .monster_id
                        .parse::<MonsterId>()
                        .map_err(|_| "Invalid monster ID".to_string())?;
                    let quantity = self
                        .objective_buffer
                        .quantity
                        .parse::<u32>()
                        .map_err(|_| "Invalid quantity".to_string())?;
                    QuestObjective::KillMonsters {
                        monster_id,
                        quantity: quantity as u16,
                    }
                }
                ObjectiveType::CollectItems => {
                    let item_id = self
                        .objective_buffer
                        .item_id
                        .parse::<ItemId>()
                        .map_err(|_| "Invalid item ID".to_string())?;
                    let quantity = self
                        .objective_buffer
                        .quantity
                        .parse::<u32>()
                        .map_err(|_| "Invalid quantity".to_string())?;
                    QuestObjective::CollectItems {
                        item_id,
                        quantity: quantity as u16,
                    }
                }
                ObjectiveType::ReachLocation => {
                    let map_id = self
                        .objective_buffer
                        .map_id
                        .parse::<MapId>()
                        .map_err(|_| "Invalid map ID".to_string())?;
                    let x = self
                        .objective_buffer
                        .location_x
                        .parse::<u32>()
                        .map_err(|_| "Invalid X coordinate".to_string())?;
                    let y = self
                        .objective_buffer
                        .location_y
                        .parse::<u32>()
                        .map_err(|_| "Invalid Y coordinate".to_string())?;
                    let radius = self
                        .objective_buffer
                        .location_radius
                        .parse::<u32>()
                        .map_err(|_| "Invalid radius".to_string())?;
                    QuestObjective::ReachLocation {
                        map_id,
                        position: Position::new(x as i32, y as i32),
                        radius: radius as u8,
                    }
                }
                ObjectiveType::TalkToNpc => {
                    let map_id = self
                        .objective_buffer
                        .map_id
                        .parse::<MapId>()
                        .map_err(|_| "Invalid map ID".to_string())?;
                    QuestObjective::TalkToNpc {
                        npc_id: self.objective_buffer.npc_id.parse::<u16>().unwrap_or(0),
                        map_id,
                    }
                }
                ObjectiveType::DeliverItem => {
                    let item_id = self
                        .objective_buffer
                        .item_id
                        .parse::<ItemId>()
                        .map_err(|_| "Invalid item ID".to_string())?;
                    let quantity = self
                        .objective_buffer
                        .quantity
                        .parse::<u32>()
                        .map_err(|_| "Invalid quantity".to_string())?;
                    QuestObjective::DeliverItem {
                        item_id,
                        npc_id: self.objective_buffer.npc_id.parse::<u16>().unwrap_or(0),
                        quantity: quantity as u16,
                    }
                }
                ObjectiveType::EscortNpc => {
                    let map_id = self
                        .objective_buffer
                        .map_id
                        .parse::<MapId>()
                        .map_err(|_| "Invalid map ID".to_string())?;
                    let x = self
                        .objective_buffer
                        .location_x
                        .parse::<u32>()
                        .map_err(|_| "Invalid X coordinate".to_string())?;
                    let y = self
                        .objective_buffer
                        .location_y
                        .parse::<u32>()
                        .map_err(|_| "Invalid Y coordinate".to_string())?;
                    QuestObjective::EscortNpc {
                        npc_id: self.objective_buffer.npc_id.parse::<u16>().unwrap_or(0),
                        map_id,
                        position: Position::new(x as i32, y as i32),
                    }
                }
                ObjectiveType::CustomFlag => QuestObjective::CustomFlag {
                    flag_name: self.objective_buffer.flag_name.clone(),
                    required_value: self.objective_buffer.flag_value,
                },
            };

            if let Some(stage_idx) = self.selected_stage {
                if stage_idx < self.quests[quest_idx].stages.len() {
                    self.quests[quest_idx].stages[stage_idx].add_objective(objective);
                    self.has_unsaved_changes = true;
                    self.objective_buffer = ObjectiveEditBuffer::default();
                    self.selected_objective = None;
                    return Ok(());
                }
            }

            Err("No stage selected".to_string())
        } else {
            Err("No quest selected".to_string())
        }
    }

    /// Validate current quest
    pub fn validate_current_quest(&mut self) {
        self.validation_errors.clear();

        if let Some(idx) = self.selected_quest {
            if idx < self.quests.len() {
                let quest = &self.quests[idx];

                if quest.name.is_empty() {
                    self.validation_errors
                        .push("Quest name cannot be empty".to_string());
                }

                if quest.stages.is_empty() {
                    self.validation_errors
                        .push("Quest must have at least one stage".to_string());
                }

                for stage in &quest.stages {
                    if stage.objectives.is_empty() {
                        self.validation_errors
                            .push(format!("Stage {} has no objectives", stage.stage_number));
                    }
                }
            }
        }
    }

    /// Get preview text for current quest
    pub fn get_quest_preview(&self, index: usize) -> String {
        if index >= self.quests.len() {
            return String::new();
        }

        let quest = &self.quests[index];
        let mut preview = format!(
            "Quest: {}\n\nID: {}\n\nDescription:\n{}\n\n",
            quest.name, quest.id, quest.description
        );

        if !quest.stages.is_empty() {
            preview.push_str("Stages:\n");
            for stage in &quest.stages {
                preview.push_str(&format!("  Stage {}: {}\n", stage.stage_number, stage.name));
                for obj in &stage.objectives {
                    preview.push_str(&format!("    - {}\n", obj.description()));
                }
            }
        }

        if !quest.rewards.is_empty() {
            preview.push_str("\nRewards:\n");
            for reward in &quest.rewards {
                let desc = match reward {
                    antares::domain::quest::QuestReward::Experience(xp) => format!("{} XP", xp),
                    antares::domain::quest::QuestReward::Gold(gold) => format!("{} Gold", gold),
                    antares::domain::quest::QuestReward::Items(items) => {
                        let item_strs: Vec<String> = items
                            .iter()
                            .map(|(id, qty)| format!("{}x Item #{}", qty, id))
                            .collect();
                        item_strs.join(", ")
                    }
                    antares::domain::quest::QuestReward::UnlockQuest(qid) => {
                        format!("Unlock Quest {}", qid)
                    }
                    antares::domain::quest::QuestReward::SetFlag { flag_name, value } => {
                        format!("Set flag '{}' to {}", flag_name, value)
                    }
                    antares::domain::quest::QuestReward::Reputation { faction, change } => {
                        format!("Reputation with {}: {:+}", faction, change)
                    }
                };
                preview.push_str(&format!("  - {}\n", desc));
            }
        }

        preview
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quest_editor_state_creation() {
        let editor = QuestEditorState::new();
        assert_eq!(editor.quests.len(), 0);
        assert_eq!(editor.mode, QuestEditorMode::List);
    }

    #[test]
    fn test_start_new_quest() {
        let mut editor = QuestEditorState::new();
        editor.start_new_quest();
        assert_eq!(editor.mode, QuestEditorMode::Creating);
    }

    #[test]
    fn test_save_quest_creates_new() {
        let mut editor = QuestEditorState::new();
        editor.start_new_quest();
        editor.quest_buffer.id = "1".to_string();
        editor.quest_buffer.name = "Test Quest".to_string();
        editor.quest_buffer.description = "Test Description".to_string();

        assert!(editor.save_quest().is_ok());
        assert_eq!(editor.quests.len(), 1);
        assert_eq!(editor.quests[0].name, "Test Quest");
    }

    #[test]
    fn test_delete_quest() {
        let mut editor = QuestEditorState::new();
        editor.start_new_quest();
        editor.quest_buffer.id = "1".to_string();
        editor.quest_buffer.name = "Test Quest".to_string();
        editor.save_quest().unwrap();

        assert_eq!(editor.quests.len(), 1);
        editor.delete_quest(0);
        assert_eq!(editor.quests.len(), 0);
    }

    #[test]
    fn test_filtered_quests() {
        let mut editor = QuestEditorState::new();
        editor.start_new_quest();
        editor.quest_buffer.id = "1".to_string();
        editor.quest_buffer.name = "Dragon Slayer".to_string();
        editor.save_quest().unwrap();

        editor.start_new_quest();
        editor.quest_buffer.id = "2".to_string();
        editor.quest_buffer.name = "Treasure Hunt".to_string();
        editor.save_quest().unwrap();

        editor.search_filter = "dragon".to_string();
        let filtered = editor.filtered_quests();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.name, "Dragon Slayer");
    }

    #[test]
    fn test_add_stage() {
        let mut editor = QuestEditorState::new();
        editor.start_new_quest();
        editor.quest_buffer.id = "1".to_string();
        editor.quest_buffer.name = "Test Quest".to_string();
        editor.save_quest().unwrap();

        editor.selected_quest = Some(0);
        editor.stage_buffer.number = "1".to_string();
        editor.stage_buffer.name = "Find the Artifact".to_string();
        assert!(editor.add_stage().is_ok());

        assert_eq!(editor.quests[0].stages.len(), 1);
    }

    #[test]
    fn test_validation_empty_quest() {
        let mut editor = QuestEditorState::new();
        editor.start_new_quest();
        editor.quest_buffer.id = "1".to_string();
        editor.quest_buffer.name = "Test".to_string();
        editor.save_quest().unwrap();

        editor.selected_quest = Some(0);
        editor.validate_current_quest();
        assert!(!editor.validation_errors.is_empty());
    }

    #[test]
    fn test_quest_buffer_levels() {
        let mut buffer = QuestEditBuffer {
            min_level: "5".to_string(),
            max_level: "15".to_string(),
            ..Default::default()
        };

        assert_eq!(buffer.min_level.parse::<u8>().unwrap(), 5);
        assert_eq!(buffer.max_level.parse::<u8>().unwrap(), 15);
    }

    #[test]
    fn test_objective_type_display() {
        assert_eq!(ObjectiveType::KillMonsters.as_str(), "Kill Monsters");
        assert_eq!(ObjectiveType::CollectItems.as_str(), "Collect Items");
        assert_eq!(ObjectiveType::ReachLocation.as_str(), "Reach Location");
    }

    #[test]
    fn test_reward_type_display() {
        assert_eq!(RewardType::Experience.as_str(), "Experience");
        assert_eq!(RewardType::Gold.as_str(), "Gold");
        assert_eq!(RewardType::Items.as_str(), "Items");
    }
}
