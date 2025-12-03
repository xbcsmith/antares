// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Quest Designer for Campaign Builder
//!
//! This module provides a visual quest designer with support for creating,
//! editing, and managing quests with multiple stages, objectives, and rewards.
//! Uses shared UI components for consistent layout.
//!
//! # Features
//!
//! - Quest list view with search and filtering
//! - Multi-stage quest editor with ordered stages
//! - Objective builder with multiple objective types
//! - Reward configuration
//! - Prerequisite chain management
//! - Quest validation and preview

use crate::ui_helpers::{ActionButtons, EditorToolbar, ItemAction, ToolbarAction, TwoColumnLayout};
use antares::domain::combat::database::MonsterDefinition;
use antares::domain::items::types::Item;
use antares::domain::quest::{Quest, QuestId, QuestObjective, QuestReward, QuestStage};
use antares::domain::types::{ItemId, MapId, MonsterId, Position};
use antares::domain::world::Map;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Editor state for quest designer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestEditorState {
    /// List of quests (synchronized with main app)
    pub quests: Vec<Quest>,

    /// Currently selected quest index
    pub selected_quest: Option<usize>,

    /// Currently selected stage index within selected quest
    pub selected_stage: Option<usize>,

    /// Currently selected objective index within selected stage
    /// Currently selected objective index within selected stage
    pub selected_objective: Option<usize>,

    /// Currently selected reward index within selected quest
    pub selected_reward: Option<usize>,

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
    pub item_quantity: String,
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
            item_quantity: "1".to_string(),
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
            selected_reward: None,
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

    /// Edit an existing reward
    pub fn edit_reward(
        &mut self,
        quests: &[Quest],
        quest_idx: usize,
        reward_idx: usize,
    ) -> Result<(), String> {
        if quest_idx >= quests.len() {
            return Err("Invalid quest index".to_string());
        }

        let quest = &quests[quest_idx];
        if reward_idx >= quest.rewards.len() {
            return Err("Invalid reward index".to_string());
        }

        let reward = &quest.rewards[reward_idx];
        self.reward_buffer = match reward {
            antares::domain::quest::QuestReward::Experience(xp) => RewardEditBuffer {
                reward_type: RewardType::Experience,
                experience: xp.to_string(),
                ..Default::default()
            },
            antares::domain::quest::QuestReward::Gold(gold) => RewardEditBuffer {
                reward_type: RewardType::Gold,
                gold: gold.to_string(),
                ..Default::default()
            },
            antares::domain::quest::QuestReward::Items(items) => {
                // For simplicity in this editor version, we only edit the first item in the list
                // A more complex editor would handle multiple items per reward entry
                if let Some((id, qty)) = items.first() {
                    RewardEditBuffer {
                        reward_type: RewardType::Items,
                        item_id: id.to_string(),
                        // We use the 'quantity' field from objective buffer for item quantity here too?
                        // No, RewardEditBuffer doesn't have quantity. We should add it or use a different field.
                        // Let's check RewardEditBuffer definition. It doesn't have quantity.
                        // We should add 'quantity' to RewardEditBuffer in a separate chunk.
                        // For now let's use 'experience' string for quantity as a temporary placeholder if needed,
                        // but correct way is to add the field.
                        // I will add the field in a separate chunk first.
                        ..Default::default()
                    }
                } else {
                    RewardEditBuffer {
                        reward_type: RewardType::Items,
                        ..Default::default()
                    }
                }
            }
            antares::domain::quest::QuestReward::UnlockQuest(qid) => RewardEditBuffer {
                reward_type: RewardType::UnlockQuest,
                unlock_quest_id: qid.to_string(),
                ..Default::default()
            },
            antares::domain::quest::QuestReward::SetFlag { flag_name, value } => RewardEditBuffer {
                reward_type: RewardType::SetFlag,
                flag_name: flag_name.clone(),
                flag_value: *value,
                ..Default::default()
            },
            antares::domain::quest::QuestReward::Reputation { faction, change } => {
                RewardEditBuffer {
                    reward_type: RewardType::Reputation,
                    faction_name: faction.clone(),
                    reputation_change: change.to_string(),
                    ..Default::default()
                }
            }
        };

        // Fix for item quantity:
        if let antares::domain::quest::QuestReward::Items(items) = reward {
            if let Some((_, qty)) = items.first() {
                self.reward_buffer.item_quantity = qty.to_string();
            }
        }

        self.selected_reward = Some(reward_idx);
        Ok(())
    }

    /// Save edited reward
    pub fn save_reward(
        &mut self,
        quests: &mut Vec<Quest>,
        quest_idx: usize,
        reward_idx: usize,
    ) -> Result<(), String> {
        if quest_idx >= quests.len() {
            return Err("Invalid quest index".to_string());
        }

        if reward_idx >= quests[quest_idx].rewards.len() {
            return Err("Invalid reward index".to_string());
        }

        let reward = match self.reward_buffer.reward_type {
            RewardType::Experience => {
                let xp = self
                    .reward_buffer
                    .experience
                    .parse::<u32>()
                    .map_err(|_| "Invalid experience amount".to_string())?;
                antares::domain::quest::QuestReward::Experience(xp)
            }
            RewardType::Gold => {
                let gold = self
                    .reward_buffer
                    .gold
                    .parse::<u32>()
                    .map_err(|_| "Invalid gold amount".to_string())?;
                antares::domain::quest::QuestReward::Gold(gold)
            }
            RewardType::Items => {
                let item_id = self
                    .reward_buffer
                    .item_id
                    .parse::<ItemId>()
                    .map_err(|_| "Invalid item ID".to_string())?;
                let quantity = self
                    .reward_buffer
                    .item_quantity
                    .parse::<u16>()
                    .map_err(|_| "Invalid quantity".to_string())?;
                antares::domain::quest::QuestReward::Items(vec![(item_id, quantity)])
            }
            RewardType::UnlockQuest => {
                let qid = self
                    .reward_buffer
                    .unlock_quest_id
                    .parse::<QuestId>()
                    .map_err(|_| "Invalid quest ID".to_string())?;
                antares::domain::quest::QuestReward::UnlockQuest(qid)
            }
            RewardType::SetFlag => antares::domain::quest::QuestReward::SetFlag {
                flag_name: self.reward_buffer.flag_name.clone(),
                value: self.reward_buffer.flag_value,
            },
            RewardType::Reputation => {
                let change = self
                    .reward_buffer
                    .reputation_change
                    .parse::<i16>()
                    .map_err(|_| "Invalid reputation change".to_string())?;
                antares::domain::quest::QuestReward::Reputation {
                    faction: self.reward_buffer.faction_name.clone(),
                    change,
                }
            }
        };

        quests[quest_idx].rewards[reward_idx] = reward;
        self.has_unsaved_changes = true;
        self.selected_reward = None;
        Ok(())
    }

    /// Delete a reward from quest
    pub fn delete_reward(
        &mut self,
        quests: &mut Vec<Quest>,
        quest_idx: usize,
        reward_idx: usize,
    ) -> Result<(), String> {
        if quest_idx >= quests.len() {
            return Err("Invalid quest index".to_string());
        }

        if reward_idx >= quests[quest_idx].rewards.len() {
            return Err("Invalid reward index".to_string());
        }

        quests[quest_idx].rewards.remove(reward_idx);
        self.has_unsaved_changes = true;

        if self.selected_reward == Some(reward_idx) {
            self.selected_reward = None;
        }

        Ok(())
    }

    /// Add a default reward to current quest
    pub fn add_default_reward(&mut self, quests: &mut Vec<Quest>) -> Result<usize, String> {
        if let Some(quest_idx) = self.selected_quest {
            if quest_idx >= quests.len() {
                return Err("Invalid quest index".to_string());
            }

            // Default reward: 100 XP
            let reward = antares::domain::quest::QuestReward::Experience(100);
            quests[quest_idx].rewards.push(reward);
            self.has_unsaved_changes = true;

            Ok(quests[quest_idx].rewards.len() - 1)
        } else {
            Err("No quest selected".to_string())
        }
    }

    /// Edit an existing stage
    pub fn edit_stage(&mut self, quest_idx: usize, stage_idx: usize) -> Result<(), String> {
        if quest_idx >= self.quests.len() {
            return Err("Invalid quest index".to_string());
        }

        let quest = &self.quests[quest_idx];
        if stage_idx >= quest.stages.len() {
            return Err("Invalid stage index".to_string());
        }

        let stage = &quest.stages[stage_idx];
        self.stage_buffer = StageEditBuffer {
            number: stage.stage_number.to_string(),
            name: stage.name.clone(),
            description: stage.description.clone(),
            require_all: stage.require_all_objectives,
        };

        self.selected_stage = Some(stage_idx);
        Ok(())
    }

    /// Save edited stage
    pub fn save_stage(&mut self, quest_idx: usize, stage_idx: usize) -> Result<(), String> {
        if quest_idx >= self.quests.len() {
            return Err("Invalid quest index".to_string());
        }

        if stage_idx >= self.quests[quest_idx].stages.len() {
            return Err("Invalid stage index".to_string());
        }

        let stage_num = self
            .stage_buffer
            .number
            .parse::<u8>()
            .map_err(|_| "Invalid stage number".to_string())?;

        let stage = &mut self.quests[quest_idx].stages[stage_idx];
        stage.stage_number = stage_num;
        stage.name = self.stage_buffer.name.clone();
        stage.description = self.stage_buffer.description.clone();
        stage.require_all_objectives = self.stage_buffer.require_all;

        self.has_unsaved_changes = true;
        self.selected_stage = None;
        self.selected_objective = None;
        self.selected_reward = None;
        Ok(())
    }

    /// Delete a stage from quest
    pub fn delete_stage(&mut self, quest_idx: usize, stage_idx: usize) -> Result<(), String> {
        if quest_idx >= self.quests.len() {
            return Err("Invalid quest index".to_string());
        }

        if stage_idx >= self.quests[quest_idx].stages.len() {
            return Err("Invalid stage index".to_string());
        }

        self.quests[quest_idx].stages.remove(stage_idx);
        self.has_unsaved_changes = true;

        if self.selected_stage == Some(stage_idx) {
            self.selected_stage = None;
        }

        Ok(())
    }

    /// Edit an existing objective
    pub fn edit_objective(
        &mut self,
        quest_idx: usize,
        stage_idx: usize,
        objective_idx: usize,
    ) -> Result<(), String> {
        if quest_idx >= self.quests.len() {
            return Err("Invalid quest index".to_string());
        }

        let quest = &self.quests[quest_idx];
        if stage_idx >= quest.stages.len() {
            return Err("Invalid stage index".to_string());
        }

        let stage = &quest.stages[stage_idx];
        if objective_idx >= stage.objectives.len() {
            return Err("Invalid objective index".to_string());
        }

        let objective = &stage.objectives[objective_idx];
        self.objective_buffer = match objective {
            QuestObjective::KillMonsters {
                monster_id,
                quantity,
            } => ObjectiveEditBuffer {
                objective_type: ObjectiveType::KillMonsters,
                monster_id: monster_id.to_string(),
                quantity: quantity.to_string(),
                ..Default::default()
            },
            QuestObjective::CollectItems { item_id, quantity } => ObjectiveEditBuffer {
                objective_type: ObjectiveType::CollectItems,
                item_id: item_id.to_string(),
                quantity: quantity.to_string(),
                ..Default::default()
            },
            QuestObjective::ReachLocation {
                map_id,
                position,
                radius,
            } => ObjectiveEditBuffer {
                objective_type: ObjectiveType::ReachLocation,
                map_id: map_id.to_string(),
                location_x: position.x.to_string(),
                location_y: position.y.to_string(),
                location_radius: radius.to_string(),
                ..Default::default()
            },
            QuestObjective::TalkToNpc { npc_id, map_id } => ObjectiveEditBuffer {
                objective_type: ObjectiveType::TalkToNpc,
                npc_id: npc_id.to_string(),
                map_id: map_id.to_string(),
                ..Default::default()
            },
            QuestObjective::DeliverItem {
                item_id,
                npc_id,
                quantity,
            } => ObjectiveEditBuffer {
                objective_type: ObjectiveType::DeliverItem,
                item_id: item_id.to_string(),
                npc_id: npc_id.to_string(),
                quantity: quantity.to_string(),
                ..Default::default()
            },
            QuestObjective::EscortNpc {
                npc_id,
                map_id,
                position,
            } => ObjectiveEditBuffer {
                objective_type: ObjectiveType::EscortNpc,
                npc_id: npc_id.to_string(),
                map_id: map_id.to_string(),
                location_x: position.x.to_string(),
                location_y: position.y.to_string(),
                ..Default::default()
            },
            QuestObjective::CustomFlag {
                flag_name,
                required_value,
            } => ObjectiveEditBuffer {
                objective_type: ObjectiveType::CustomFlag,
                flag_name: flag_name.clone(),
                flag_value: *required_value,
                ..Default::default()
            },
        };

        self.selected_objective = Some(objective_idx);
        Ok(())
    }

    /// Save edited objective
    pub fn save_objective(
        &mut self,
        quest_idx: usize,
        stage_idx: usize,
        objective_idx: usize,
    ) -> Result<(), String> {
        if quest_idx >= self.quests.len() {
            return Err("Invalid quest index".to_string());
        }

        if stage_idx >= self.quests[quest_idx].stages.len() {
            return Err("Invalid stage index".to_string());
        }

        if objective_idx >= self.quests[quest_idx].stages[stage_idx].objectives.len() {
            return Err("Invalid objective index".to_string());
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

        self.quests[quest_idx].stages[stage_idx].objectives[objective_idx] = objective;
        self.has_unsaved_changes = true;
        self.selected_objective = None;
        self.selected_reward = None;
        Ok(())
    }

    /// Delete an objective from a stage
    pub fn delete_objective(
        &mut self,
        quest_idx: usize,
        stage_idx: usize,
        objective_idx: usize,
    ) -> Result<(), String> {
        if quest_idx >= self.quests.len() {
            return Err("Invalid quest index".to_string());
        }

        if stage_idx >= self.quests[quest_idx].stages.len() {
            return Err("Invalid stage index".to_string());
        }

        if objective_idx >= self.quests[quest_idx].stages[stage_idx].objectives.len() {
            return Err("Invalid objective index".to_string());
        }

        self.quests[quest_idx].stages[stage_idx]
            .objectives
            .remove(objective_idx);
        self.has_unsaved_changes = true;

        if self.selected_objective == Some(objective_idx) {
            self.selected_objective = None;
        }

        Ok(())
    }

    /// Find orphaned objectives (stages with no objectives)
    pub fn find_orphaned_objectives(&self) -> Vec<(QuestId, u8)> {
        let mut orphaned = Vec::new();

        for quest in &self.quests {
            for stage in &quest.stages {
                if stage.objectives.is_empty() {
                    orphaned.push((quest.id, stage.stage_number));
                }
            }
        }

        orphaned
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
    pub fn start_new_quest(&mut self, next_id: String) {
        // Create a temporary quest
        let mut new_quest = Quest::new(0, "New Quest", "Description");
        // Try to parse ID if numeric, otherwise use 0 (will be updated on save)
        if let Ok(id_num) = next_id.parse::<QuestId>() {
            new_quest.id = id_num;
        }

        self.quests.push(new_quest);
        let new_idx = self.quests.len() - 1;

        self.selected_quest = Some(new_idx);
        self.mode = QuestEditorMode::Creating;

        // Initialize buffer with default values
        self.quest_buffer = QuestEditBuffer::default();
        self.quest_buffer.id = next_id;
        self.quest_buffer.name = "New Quest".to_string();
        self.quest_buffer.description = "Description".to_string();

        self.selected_stage = None;
        self.selected_stage = None;
        self.selected_objective = None;
        self.selected_reward = None;
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
            self.selected_stage = None;
            self.selected_objective = None;
            self.selected_reward = None;
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

        quest.id = id;
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
        self.selected_reward = None;
        Ok(())
    }

    /// Render the Quest Editor UI and manage editor state transitions.
    ///
    /// The Quest Editor comprises a left-hand list view, a right-hand preview, and full
    /// screen editor forms for creating/editing quests. This method renders the toolbar
    /// (New, Save, Load), synchronizes the editor's internal quest buffer with the `quests`
    /// parameter, and delegates to the modular helpers for list/preview/form rendering.
    ///
    /// # Arguments
    ///
    /// * `ui` - egui `Ui` instance to draw the editor into
    /// * `quests` - Mutable reference to the global quests vector (synchronized on exit)
    /// * `items` - Available items used for objective/reward selection
    /// * `monsters` - Monster definitions used for objective selection
    /// * `maps` - Map definitions and NPC lists used for location/objective selection
    /// * `campaign_dir` - Optional campaign directory path used for file saving/loading
    /// * `quests_file` - Filename within the campaign directory for quests
    /// * `unsaved_changes` - Mutable flag indicating application-level unsaved changes
    /// * `status_message` - Mutable status string to show operation results to the user
    /// * `file_load_merge_mode` - Toggle controlling whether imported quests replace or merge
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::campaign_builder::quest_editor::{QuestEditorState, QuestEditorMode};
    ///
    /// let mut editor = QuestEditorState::new();
    /// editor.start_new_quest("1".to_string());
    /// assert_eq!(editor.mode, QuestEditorMode::Creating);
    /// ```
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        quests: &mut Vec<Quest>,
        items: &[Item],
        monsters: &[MonsterDefinition],
        maps: &[Map],
        campaign_dir: Option<&PathBuf>,
        quests_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        file_load_merge_mode: &mut bool,
    ) {
        // Sync quests from parameter to internal state
        self.quests = quests.clone();

        ui.heading("📜 Quests Editor");
        ui.add_space(5.0);

        // Use shared EditorToolbar component
        let toolbar_action = EditorToolbar::new("Quests")
            .with_search(&mut self.search_filter)
            .with_merge_mode(file_load_merge_mode)
            .with_total_count(quests.len())
            .with_id_salt("quests_toolbar")
            .show(ui);

        // Handle toolbar actions
        match toolbar_action {
            ToolbarAction::New => {
                let next_id = quests.iter().map(|q| q.id).max().unwrap_or(0) + 1;
                self.start_new_quest(next_id.to_string());
                *unsaved_changes = true;
            }
            ToolbarAction::Save => {
                if let Some(dir) = campaign_dir {
                    let quests_path = dir.join(quests_file);
                    if let Some(parent) = quests_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }

                    match ron::ser::to_string_pretty(&quests, Default::default()) {
                        Ok(contents) => match std::fs::write(&quests_path, contents) {
                            Ok(_) => {
                                *status_message =
                                    format!("Saved quests to: {}", quests_path.display());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to save quests: {}", e);
                            }
                        },
                        Err(e) => {
                            *status_message = format!("Failed to serialize quests: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Load => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("RON", &["ron"])
                    .pick_file()
                {
                    let load_result = std::fs::read_to_string(&path).and_then(|contents| {
                        ron::from_str::<Vec<Quest>>(&contents)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    });

                    match load_result {
                        Ok(loaded_quests) => {
                            if *file_load_merge_mode {
                                for quest in loaded_quests {
                                    if let Some(existing) =
                                        quests.iter_mut().find(|q| q.id == quest.id)
                                    {
                                        *existing = quest;
                                    } else {
                                        quests.push(quest);
                                    }
                                }
                            } else {
                                *quests = loaded_quests;
                            }
                            *unsaved_changes = true;
                            *status_message = format!("Loaded quests from: {}", path.display());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to load quests: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Import => {
                // Import not yet implemented for quests
                *status_message = "Import not yet implemented for quests".to_string();
            }
            ToolbarAction::Export => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("quests.ron")
                    .add_filter("RON", &["ron"])
                    .save_file()
                {
                    match ron::ser::to_string_pretty(&quests, Default::default()) {
                        Ok(contents) => match std::fs::write(&path, contents) {
                            Ok(_) => {
                                *status_message = format!("Saved quests to: {}", path.display());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to save quests: {}", e);
                            }
                        },
                        Err(e) => {
                            *status_message = format!("Failed to serialize quests: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Reload => {
                if let Some(dir) = campaign_dir {
                    let quests_path = dir.join(quests_file);
                    if quests_path.exists() {
                        match std::fs::read_to_string(&quests_path) {
                            Ok(contents) => match ron::from_str::<Vec<Quest>>(&contents) {
                                Ok(loaded_quests) => {
                                    *quests = loaded_quests;
                                    self.quests = quests.clone();
                                    *status_message = format!("Reloaded {} quests", quests.len());
                                }
                                Err(e) => {
                                    *status_message = format!("Failed to parse quests: {}", e);
                                }
                            },
                            Err(e) => {
                                *status_message = format!("Failed to read quests file: {}", e);
                            }
                        }
                    } else {
                        *status_message = "Quests file does not exist".to_string();
                    }
                }
            }
            ToolbarAction::None => {}
        }

        ui.separator();

        // Main content - split view or form editor
        match self.mode {
            QuestEditorMode::List => {
                // Build filtered list snapshot to avoid borrow conflicts in closures
                let search_filter = self.search_filter.to_lowercase();
                let filtered_quests: Vec<(usize, Quest)> = quests
                    .iter()
                    .enumerate()
                    .filter(|(_, q)| {
                        search_filter.is_empty()
                            || q.name.to_lowercase().contains(&search_filter)
                            || q.description.to_lowercase().contains(&search_filter)
                    })
                    .map(|(idx, q)| (idx, q.clone()))
                    .collect();

                let selected_idx = self.selected_quest;
                let mut new_selection: Option<usize> = None;
                let mut action_requested: Option<ItemAction> = None;

                // Use shared TwoColumnLayout component
                TwoColumnLayout::new("quests").show_split(
                    ui,
                    |left_ui| {
                        // Left panel: Quest list
                        left_ui.heading("Quests");
                        left_ui.separator();

                        for (idx, quest) in &filtered_quests {
                            let is_selected = selected_idx == Some(*idx);
                            let label = format!(
                                "{} - {} {}",
                                quest.id,
                                quest.name,
                                if quest.is_main_quest { "⭐" } else { "" }
                            );
                            if left_ui.selectable_label(is_selected, label).clicked() {
                                new_selection = Some(*idx);
                            }
                        }

                        if filtered_quests.is_empty() {
                            left_ui.label("No quests found");
                        }
                    },
                    |right_ui| {
                        // Right panel: Detail view
                        if let Some(idx) = selected_idx {
                            if let Some((_, quest)) =
                                filtered_quests.iter().find(|(i, _)| *i == idx)
                            {
                                right_ui.heading(&quest.name);
                                right_ui.separator();

                                // Use shared ActionButtons component
                                let action = ActionButtons::new().enabled(true).show(right_ui);
                                if action != ItemAction::None {
                                    action_requested = Some(action);
                                }

                                right_ui.separator();

                                // Show quest preview
                                Self::show_quest_preview_static(right_ui, quest);
                            } else {
                                right_ui.vertical_centered(|ui| {
                                    ui.add_space(100.0);
                                    ui.label("Select a quest to view details");
                                });
                            }
                        } else {
                            right_ui.vertical_centered(|ui| {
                                ui.add_space(100.0);
                                ui.label("Select a quest to view details or create a new quest");
                            });
                        }
                    },
                );

                // Apply selection change after closures
                if let Some(idx) = new_selection {
                    self.selected_quest = Some(idx);
                }

                // Handle action button clicks after closures
                if let Some(action) = action_requested {
                    match action {
                        ItemAction::Edit => {
                            if let Some(idx) = self.selected_quest {
                                self.start_edit_quest(idx);
                            }
                        }
                        ItemAction::Delete => {
                            if let Some(idx) = self.selected_quest {
                                self.delete_quest(idx);
                                self.selected_quest = None;
                                *unsaved_changes = true;
                            }
                        }
                        ItemAction::Duplicate => {
                            if let Some(idx) = self.selected_quest {
                                if idx < quests.len() {
                                    let next_id =
                                        quests.iter().map(|q| q.id).max().unwrap_or(0) + 1;
                                    let mut new_quest = quests[idx].clone();
                                    new_quest.id = next_id;
                                    new_quest.name = format!("{} (Copy)", new_quest.name);
                                    quests.push(new_quest);
                                    *unsaved_changes = true;
                                    *status_message = "Quest duplicated".to_string();
                                }
                            }
                        }
                        ItemAction::Export => {
                            if let Some(idx) = self.selected_quest {
                                if idx < quests.len() {
                                    if let Ok(ron_str) = ron::ser::to_string_pretty(
                                        &quests[idx],
                                        ron::ser::PrettyConfig::default(),
                                    ) {
                                        ui.ctx().copy_text(ron_str);
                                        *status_message = "Quest copied to clipboard".to_string();
                                    } else {
                                        *status_message = "Failed to export quest".to_string();
                                    }
                                }
                            }
                        }
                        ItemAction::None => {}
                    }
                }
            }
            QuestEditorMode::Creating | QuestEditorMode::Editing => {
                // Full-screen quest form editor - use the external `quests` buffer to
                // prevent overlapping mutable borrows of `self.quests` within UI closures.
                self.show_quest_form(ui, quests, items, monsters, maps, unsaved_changes);
            }
        }

        // Sync quests back from internal state to parameter
        *quests = self.quests.clone();
    }

    /// Cancel current edit
    pub fn cancel_edit(&mut self) {
        if self.mode == QuestEditorMode::Creating {
            if let Some(idx) = self.selected_quest {
                if idx < self.quests.len() {
                    self.quests.remove(idx);
                }
            }
        }

        self.mode = QuestEditorMode::List;
        self.selected_quest = None;
        self.selected_stage = None;
        self.selected_stage = None;
        self.selected_objective = None;
        self.selected_reward = None;
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

    /// Add a default objective to current stage
    pub fn add_default_objective(&mut self, stage_idx: usize) -> Result<usize, String> {
        if let Some(quest_idx) = self.selected_quest {
            if quest_idx >= self.quests.len() {
                return Err("Invalid quest index".to_string());
            }

            if stage_idx < self.quests[quest_idx].stages.len() {
                // Add a default objective (Kill Monster 0, Qty 1)
                let objective = QuestObjective::KillMonsters {
                    monster_id: 0,
                    quantity: 1,
                };

                self.quests[quest_idx].stages[stage_idx].add_objective(objective);
                self.has_unsaved_changes = true;

                // Return the index of the new objective
                Ok(self.quests[quest_idx].stages[stage_idx].objectives.len() - 1)
            } else {
                Err("Invalid stage index".to_string())
            }
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

    /// Static quest preview method that doesn't require self
    fn show_quest_preview_static(ui: &mut egui::Ui, quest: &Quest) {
        ui.heading(&quest.name);
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.label("Description:");
                ui.label(&quest.description);
            });

            ui.add_space(5.0);

            ui.group(|ui| {
                ui.label("Quest Info:");
                ui.separator();
                ui.label(format!("ID: {}", quest.id));
                ui.label(format!(
                    "Type: {}",
                    if quest.is_main_quest {
                        "Main Quest ⭐"
                    } else {
                        "Side Quest"
                    }
                ));
                ui.label(format!("Repeatable: {}", quest.repeatable));

                if let Some(min) = quest.min_level {
                    ui.label(format!("Min Level: {}", min));
                }
                if let Some(max) = quest.max_level {
                    ui.label(format!("Max Level: {}", max));
                }
            });

            ui.add_space(5.0);

            ui.group(|ui| {
                ui.label(format!("Stages ({}):", quest.stages.len()));
                ui.separator();
                for stage in &quest.stages {
                    ui.collapsing(
                        format!("Stage {}: {}", stage.stage_number, stage.name),
                        |ui| {
                            ui.label(&stage.description);
                            ui.separator();
                            ui.label(format!("Objectives ({})", stage.objectives.len()));
                            for objective in &stage.objectives {
                                ui.label(format!("  • {}", objective.description()));
                            }
                        },
                    );
                }
            });

            ui.add_space(5.0);

            ui.group(|ui| {
                ui.label(format!("Rewards ({}):", quest.rewards.len()));
                ui.separator();
                for reward in &quest.rewards {
                    match reward {
                        QuestReward::Experience(xp) => ui.label(format!("  • {} XP", xp)),
                        QuestReward::Gold(gold) => ui.label(format!("  • {} Gold", gold)),
                        QuestReward::Items(items) => {
                            for (item_id, qty) in items {
                                ui.label(format!("  • {} x Item {}", qty, item_id));
                            }
                            ui.label("")
                        }
                        QuestReward::UnlockQuest(quest_id) => {
                            ui.label(format!("  • Unlock Quest {}", quest_id))
                        }
                        QuestReward::SetFlag { flag_name, value } => {
                            ui.label(format!("  • Set Flag '{}' = {}", flag_name, value))
                        }
                        QuestReward::Reputation { faction, change } => {
                            ui.label(format!("  • {} Reputation: {:+}", faction, change))
                        }
                    };
                }
            });
        });
    }

    /// Show quest form editor
    fn show_quest_form(
        &mut self,
        ui: &mut egui::Ui,
        quests: &mut Vec<Quest>,
        items: &[Item],
        monsters: &[MonsterDefinition],
        maps: &[Map],
        unsaved_changes: &mut bool,
    ) {
        let is_creating = matches!(self.mode, QuestEditorMode::Creating);

        ui.heading(if is_creating {
            "Create New Quest"
        } else {
            "Edit Quest"
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.label("Basic Information");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("ID:");
                        ui.text_edit_singleline(&mut self.quest_buffer.id);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.quest_buffer.name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Description:");
                    });
                    ui.add(
                        egui::TextEdit::multiline(&mut self.quest_buffer.description)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY),
                    );

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.quest_buffer.repeatable, "Repeatable");
                        ui.checkbox(&mut self.quest_buffer.is_main_quest, "Main Quest");
                    });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label("Level Requirements");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Min Level:");
                        ui.text_edit_singleline(&mut self.quest_buffer.min_level);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Max Level:");
                        ui.text_edit_singleline(&mut self.quest_buffer.max_level);
                    });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label("Quest Giver");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("NPC ID:");
                        ui.text_edit_singleline(&mut self.quest_buffer.quest_giver_npc);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Map ID:");
                        ui.text_edit_singleline(&mut self.quest_buffer.quest_giver_map);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Position X:");
                        ui.text_edit_singleline(&mut self.quest_buffer.quest_giver_x);
                        ui.label("Y:");
                        ui.text_edit_singleline(&mut self.quest_buffer.quest_giver_y);
                    });
                });

                ui.add_space(10.0);

                // Stages editor
                self.show_quest_stages_editor(ui, quests, items, monsters, maps, unsaved_changes);

                ui.add_space(10.0);

                // Rewards editor
                self.show_quest_rewards_editor(ui, quests, items, unsaved_changes);

                ui.add_space(10.0);

                // Validation display
                self.show_quest_validation(ui);

                ui.add_space(10.0);

                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button("✅ Save Quest").clicked() {}

                    if ui.button("❌ Cancel").clicked() {
                        self.cancel_edit();
                    }
                });
            });
    }

    /// Show quest stages editor
    fn show_quest_stages_editor(
        &mut self,
        ui: &mut egui::Ui,
        quests: &mut Vec<Quest>,
        items: &[Item],
        monsters: &[MonsterDefinition],
        maps: &[Map],
        unsaved_changes: &mut bool,
    ) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading("Quest Stages");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➕ Add Stage").clicked() {
                        let _ = self.add_stage();
                        *unsaved_changes = true;
                    }
                });
            });

            ui.separator();

            if let Some(selected_idx) = self.selected_quest {
                if selected_idx < quests.len() {
                    // Clone stages to avoid borrowing issues
                    let stages = quests[selected_idx].stages.clone();
                    let mut stage_to_delete: Option<usize> = None;
                    let mut stage_to_edit: Option<usize> = None;

                    for (stage_idx, stage) in stages.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let header = ui.collapsing(
                                format!("Stage {}: {}", stage.stage_number, stage.name),
                                |ui| {
                                    ui.label(&stage.description);
                                    ui.label(format!(
                                        "Require all objectives: {}",
                                        stage.require_all_objectives
                                    ));
                                    ui.separator();

                                    // Show objectives with edit/delete controls
                                    self.show_quest_objectives_editor(
                                        ui,
                                        selected_idx,
                                        stage_idx,
                                        &stage.objectives,
                                        quests,
                                        items,
                                        monsters,
                                        maps,
                                        unsaved_changes,
                                    );
                                },
                            );

                            // Stage action buttons
                            if ui.small_button("✏️").on_hover_text("Edit Stage").clicked() {
                                stage_to_edit = Some(stage_idx);
                            }
                            if ui
                                .small_button("🗑️")
                                .on_hover_text("Delete Stage")
                                .clicked()
                            {
                                stage_to_delete = Some(stage_idx);
                            }
                        });
                    }

                    // Handle stage deletion
                    if let Some(stage_idx) = stage_to_delete {
                        if self.delete_stage(selected_idx, stage_idx).is_ok() {
                            *unsaved_changes = true;
                        }
                    }

                    // Handle stage editing
                    if let Some(stage_idx) = stage_to_edit {
                        if self.edit_stage(selected_idx, stage_idx).is_ok() {
                            self.mode = QuestEditorMode::Editing;
                        }
                    }

                    if stages.is_empty() {
                        ui.label("No stages defined yet");
                    }
                } else {
                    ui.label("No quest selected");
                }
            } else {
                ui.label("No quest selected");
            }
        });

        // Stage editor modal
        if let Some(stage_idx) = self.selected_stage {
            egui::Window::new("Edit Stage")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Stage Number:");
                        ui.text_edit_singleline(&mut self.stage_buffer.number);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.stage_buffer.name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Description:");
                    });
                    ui.add(
                        egui::TextEdit::multiline(&mut self.stage_buffer.description)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY),
                    );

                    ui.checkbox(
                        &mut self.stage_buffer.require_all,
                        "Require all objectives to complete",
                    );

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("✅ Save").clicked() {
                            if let Some(selected_idx) = self.selected_quest {
                                if self.save_stage(selected_idx, stage_idx).is_ok() {
                                    *unsaved_changes = true;
                                }
                            }
                        }

                        if ui.button("❌ Cancel").clicked() {
                            self.selected_stage = None;
                        }
                    });
                });
        }
    }

    /// Show quest objectives editor
    fn show_quest_objectives_editor(
        &mut self,
        ui: &mut egui::Ui,
        quest_idx: usize,
        stage_idx: usize,
        objectives: &[QuestObjective],
        quests: &mut Vec<Quest>,
        items: &[Item],
        monsters: &[MonsterDefinition],
        maps: &[Map],
        unsaved_changes: &mut bool,
    ) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Objectives ({})", objectives.len()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("➕")
                        .on_hover_text("Add Objective")
                        .clicked()
                    {
                        if let Ok(new_idx) = self.add_default_objective(stage_idx) {
                            *unsaved_changes = true;
                            // Immediately start editing the new objective
                            let _ = self.edit_objective(quest_idx, stage_idx, new_idx);
                        }
                    }
                });
            });

            ui.separator();

            let mut objective_to_delete: Option<usize> = None;
            let mut objective_to_edit: Option<usize> = None;

            for (obj_idx, objective) in objectives.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("{}.", obj_idx + 1));
                    ui.label(objective.description());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("🗑️")
                            .on_hover_text("Delete Objective")
                            .clicked()
                        {
                            objective_to_delete = Some(obj_idx);
                        }
                        if ui
                            .small_button("✏️")
                            .on_hover_text("Edit Objective")
                            .clicked()
                        {
                            objective_to_edit = Some(obj_idx);
                        }
                    });
                });
            }

            // Handle objective deletion
            if let Some(obj_idx) = objective_to_delete {
                if self.delete_objective(quest_idx, stage_idx, obj_idx).is_ok() {
                    *unsaved_changes = true;
                }
            }

            // Handle objective editing
            if let Some(obj_idx) = objective_to_edit {
                if self.edit_objective(quest_idx, stage_idx, obj_idx).is_ok() {
                    // Objective editing modal will be shown below
                }
            }

            if objectives.is_empty() {
                ui.label("No objectives defined");
            }
        });

        // Objective editor modal
        if let Some(obj_idx) = self.selected_objective {
            egui::Window::new("Edit Objective")
                .collapsible(false)
                .resizable(true)
                .default_size([500.0, 400.0])
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Objective Type:");
                        egui::ComboBox::new("objective_type_selector", "")
                            .selected_text(self.objective_buffer.objective_type.as_str())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.objective_buffer.objective_type,
                                    crate::quest_editor::ObjectiveType::KillMonsters,
                                    "Kill Monsters",
                                );
                                ui.selectable_value(
                                    &mut self.objective_buffer.objective_type,
                                    crate::quest_editor::ObjectiveType::CollectItems,
                                    "Collect Items",
                                );
                                ui.selectable_value(
                                    &mut self.objective_buffer.objective_type,
                                    crate::quest_editor::ObjectiveType::ReachLocation,
                                    "Reach Location",
                                );
                                ui.selectable_value(
                                    &mut self.objective_buffer.objective_type,
                                    crate::quest_editor::ObjectiveType::TalkToNpc,
                                    "Talk To NPC",
                                );
                                ui.selectable_value(
                                    &mut self.objective_buffer.objective_type,
                                    crate::quest_editor::ObjectiveType::DeliverItem,
                                    "Deliver Item",
                                );
                                ui.selectable_value(
                                    &mut self.objective_buffer.objective_type,
                                    crate::quest_editor::ObjectiveType::EscortNpc,
                                    "Escort NPC",
                                );
                                ui.selectable_value(
                                    &mut self.objective_buffer.objective_type,
                                    crate::quest_editor::ObjectiveType::CustomFlag,
                                    "Custom Flag",
                                );
                            });
                    });

                    ui.separator();

                    // Type-specific fields
                    match self.objective_buffer.objective_type {
                        crate::quest_editor::ObjectiveType::KillMonsters => {
                            ui.horizontal(|ui| {
                                ui.label("Monster:");
                                egui::ComboBox::from_id_salt("monster_selector")
                                    .selected_text(
                                        monsters
                                            .iter()
                                            .find(|m| {
                                                m.id.to_string() == self.objective_buffer.monster_id
                                            })
                                            .map(|m| format!("{} - {}", m.id, m.name))
                                            .unwrap_or_else(|| {
                                                self.objective_buffer.monster_id.clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for monster in monsters {
                                            ui.selectable_value(
                                                &mut self.objective_buffer.monster_id,
                                                monster.id.to_string(),
                                                format!("{} - {}", monster.id, monster.name),
                                            );
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("Quantity:");
                                ui.text_edit_singleline(&mut self.objective_buffer.quantity);
                            });
                        }
                        crate::quest_editor::ObjectiveType::CollectItems => {
                            ui.horizontal(|ui| {
                                ui.label("Item:");
                                egui::ComboBox::from_id_salt("item_selector")
                                    .selected_text(
                                        items
                                            .iter()
                                            .find(|i| {
                                                i.id.to_string() == self.objective_buffer.item_id
                                            })
                                            .map(|i| format!("{} - {}", i.id, i.name))
                                            .unwrap_or_else(|| {
                                                self.objective_buffer.item_id.clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for item in items {
                                            ui.selectable_value(
                                                &mut self.objective_buffer.item_id,
                                                item.id.to_string(),
                                                format!("{} - {}", item.id, item.name),
                                            );
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("Quantity:");
                                ui.text_edit_singleline(&mut self.objective_buffer.quantity);
                            });
                        }
                        crate::quest_editor::ObjectiveType::ReachLocation => {
                            ui.horizontal(|ui| {
                                ui.label("Map:");
                                egui::ComboBox::from_id_salt("map_selector")
                                    .selected_text(
                                        maps.iter()
                                            .find(|m| {
                                                m.id.to_string() == self.objective_buffer.map_id
                                            })
                                            .map(|m| format!("{} - {}", m.id, m.name))
                                            .unwrap_or_else(|| {
                                                self.objective_buffer.map_id.clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for map in maps {
                                            ui.selectable_value(
                                                &mut self.objective_buffer.map_id,
                                                map.id.to_string(),
                                                format!("{} - {}", map.id, map.name),
                                            );
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("X:");
                                ui.text_edit_singleline(&mut self.objective_buffer.location_x);
                                ui.label("Y:");
                                ui.text_edit_singleline(&mut self.objective_buffer.location_y);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Radius:");
                                ui.text_edit_singleline(&mut self.objective_buffer.location_radius);
                            });
                        }
                        crate::quest_editor::ObjectiveType::TalkToNpc => {
                            ui.horizontal(|ui| {
                                ui.label("Map:");
                                egui::ComboBox::from_id_salt("map_selector_npc")
                                    .selected_text(
                                        maps.iter()
                                            .find(|m| {
                                                m.id.to_string() == self.objective_buffer.map_id
                                            })
                                            .map(|m| format!("{} - {}", m.id, m.name))
                                            .unwrap_or_else(|| {
                                                self.objective_buffer.map_id.clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for map in maps {
                                            ui.selectable_value(
                                                &mut self.objective_buffer.map_id,
                                                map.id.to_string(),
                                                format!("{} - {}", map.id, map.name),
                                            );
                                        }
                                    });
                            });

                            // Filter NPCs based on selected map
                            let selected_map_id =
                                self.objective_buffer.map_id.parse::<u16>().unwrap_or(0);
                            let map_npcs: Vec<_> = maps
                                .iter()
                                .find(|m| m.id == selected_map_id)
                                .map(|m| m.npcs.clone())
                                .unwrap_or_default();

                            ui.horizontal(|ui| {
                                ui.label("NPC:");
                                egui::ComboBox::from_id_salt("npc_selector")
                                    .selected_text(
                                        map_npcs
                                            .iter()
                                            .find(|n| {
                                                n.id.to_string() == self.objective_buffer.npc_id
                                            })
                                            .map(|n| format!("{} - {}", n.id, n.name))
                                            .unwrap_or_else(|| {
                                                self.objective_buffer.npc_id.clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for npc in &map_npcs {
                                            ui.selectable_value(
                                                &mut self.objective_buffer.npc_id,
                                                npc.id.to_string(),
                                                format!("{} - {}", npc.id, npc.name),
                                            );
                                        }
                                    });
                            });
                        }
                        crate::quest_editor::ObjectiveType::DeliverItem => {
                            ui.horizontal(|ui| {
                                ui.label("Item:");
                                egui::ComboBox::from_id_salt("item_selector_deliver")
                                    .selected_text(
                                        items
                                            .iter()
                                            .find(|i| {
                                                i.id.to_string() == self.objective_buffer.item_id
                                            })
                                            .map(|i| format!("{} - {}", i.id, i.name))
                                            .unwrap_or_else(|| {
                                                self.objective_buffer.item_id.clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for item in items {
                                            ui.selectable_value(
                                                &mut self.objective_buffer.item_id,
                                                item.id.to_string(),
                                                format!("{} - {}", item.id, item.name),
                                            );
                                        }
                                    });
                            });

                            ui.horizontal(|ui| {
                                ui.label("NPC ID:");
                                ui.text_edit_singleline(&mut self.objective_buffer.npc_id);
                            });

                            ui.horizontal(|ui| {
                                ui.label("Quantity:");
                                ui.text_edit_singleline(&mut self.objective_buffer.quantity);
                            });
                        }
                        crate::quest_editor::ObjectiveType::EscortNpc => {
                            ui.horizontal(|ui| {
                                ui.label("Map:");
                                egui::ComboBox::from_id_salt("map_selector_escort")
                                    .selected_text(
                                        maps.iter()
                                            .find(|m| {
                                                m.id.to_string() == self.objective_buffer.map_id
                                            })
                                            .map(|m| format!("{} - {}", m.id, m.name))
                                            .unwrap_or_else(|| {
                                                self.objective_buffer.map_id.clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for map in maps {
                                            ui.selectable_value(
                                                &mut self.objective_buffer.map_id,
                                                map.id.to_string(),
                                                format!("{} - {}", map.id, map.name),
                                            );
                                        }
                                    });
                            });

                            // Filter NPCs based on selected map
                            let selected_map_id =
                                self.objective_buffer.map_id.parse::<u16>().unwrap_or(0);
                            let map_npcs: Vec<_> = maps
                                .iter()
                                .find(|m| m.id == selected_map_id)
                                .map(|m| m.npcs.clone())
                                .unwrap_or_default();

                            ui.horizontal(|ui| {
                                ui.label("NPC:");
                                egui::ComboBox::from_id_salt("npc_selector_escort")
                                    .selected_text(
                                        map_npcs
                                            .iter()
                                            .find(|n| {
                                                n.id.to_string() == self.objective_buffer.npc_id
                                            })
                                            .map(|n| format!("{} - {}", n.id, n.name))
                                            .unwrap_or_else(|| {
                                                self.objective_buffer.npc_id.clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for npc in &map_npcs {
                                            ui.selectable_value(
                                                &mut self.objective_buffer.npc_id,
                                                npc.id.to_string(),
                                                format!("{} - {}", npc.id, npc.name),
                                            );
                                        }
                                    });
                            });

                            ui.horizontal(|ui| {
                                ui.label("Destination X:");
                                ui.text_edit_singleline(&mut self.objective_buffer.location_x);
                                ui.label("Y:");
                                ui.text_edit_singleline(&mut self.objective_buffer.location_y);
                            });
                        }
                        crate::quest_editor::ObjectiveType::CustomFlag => {
                            ui.horizontal(|ui| {
                                ui.label("Flag Name:");
                                ui.text_edit_singleline(&mut self.objective_buffer.flag_name);
                            });
                            ui.checkbox(&mut self.objective_buffer.flag_value, "Required Value");
                        }
                    }

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("✅ Save").clicked() {
                            if self.save_objective(quest_idx, stage_idx, obj_idx).is_ok() {
                                *unsaved_changes = true;
                            }
                        }

                        if ui.button("❌ Cancel").clicked() {
                            self.selected_objective = None;
                        }
                    });
                });
        }
    }

    /// Show quest rewards editor
    fn show_quest_rewards_editor(
        &mut self,
        ui: &mut egui::Ui,
        quests: &mut Vec<Quest>,
        items: &[Item],
        unsaved_changes: &mut bool,
    ) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading("Rewards");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➕ Add Reward").clicked() {
                        if let Ok(new_idx) = self.add_default_reward(quests) {
                            *unsaved_changes = true;
                            // Immediately start editing the new reward
                            let _ = self.edit_reward(
                                quests.as_slice(),
                                self.selected_quest.unwrap(),
                                new_idx,
                            );
                        }
                    }
                });
            });

            ui.separator();

            if let Some(selected_idx) = self.selected_quest {
                if selected_idx < quests.len() {
                    let rewards = quests[selected_idx].rewards.clone();
                    let mut reward_to_delete: Option<usize> = None;
                    let mut reward_to_edit: Option<usize> = None;

                    for (reward_idx, reward) in rewards.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let desc = match reward {
                                QuestReward::Experience(xp) => {
                                    format!("{} XP", xp)
                                }
                                QuestReward::Gold(gold) => {
                                    format!("{} Gold", gold)
                                }
                                QuestReward::Items(items_list) => {
                                    let item_strs: Vec<String> = items_list
                                        .iter()
                                        .map(|(id, qty)| {
                                            let name = items
                                                .iter()
                                                .find(|i| i.id == *id)
                                                .map(|i| i.name.clone())
                                                .unwrap_or_else(|| "Unknown Item".to_string());
                                            format!("{}x {} ({})", qty, name, id)
                                        })
                                        .collect();
                                    item_strs.join(", ")
                                }
                                QuestReward::UnlockQuest(qid) => {
                                    let name = quests
                                        .iter()
                                        .find(|q| q.id == *qid)
                                        .map(|q| q.name.clone())
                                        .unwrap_or_else(|| "Unknown Quest".to_string());
                                    format!("Unlock Quest: {} ({})", name, qid)
                                }
                                QuestReward::SetFlag { flag_name, value } => {
                                    format!("Set Flag '{}' to {}", flag_name, value)
                                }
                                QuestReward::Reputation { faction, change } => {
                                    format!("Reputation: {} ({:+})", faction, change)
                                }
                            };

                            ui.label(format!("{}.", reward_idx + 1));
                            ui.label(desc);

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .small_button("🗑️")
                                        .on_hover_text("Delete Reward")
                                        .clicked()
                                    {
                                        reward_to_delete = Some(reward_idx);
                                    }
                                    if ui.small_button("✏️").on_hover_text("Edit Reward").clicked()
                                    {
                                        reward_to_edit = Some(reward_idx);
                                    }
                                },
                            );
                        });
                    }

                    if let Some(reward_idx) = reward_to_delete {
                        if self.delete_reward(quests, selected_idx, reward_idx).is_ok() {
                            *unsaved_changes = true;
                        }
                    }

                    if let Some(reward_idx) = reward_to_edit {
                        if self
                            .edit_reward(quests.as_slice(), selected_idx, reward_idx)
                            .is_ok()
                        {
                            // Modal will show
                        }
                    }

                    if rewards.is_empty() {
                        ui.label("No rewards defined");
                    }
                }
            } else {
                ui.label("No quest selected");
            }
        });

        // Reward editor modal
        if let Some(reward_idx) = self.selected_reward {
            egui::Window::new("Edit Reward")
                .collapsible(false)
                .resizable(true)
                .default_size([400.0, 300.0])
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Type:");
                        egui::ComboBox::from_id_salt("reward_type_selector")
                            .selected_text(self.reward_buffer.reward_type.as_str())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.reward_buffer.reward_type,
                                    crate::quest_editor::RewardType::Experience,
                                    "Experience",
                                );
                                ui.selectable_value(
                                    &mut self.reward_buffer.reward_type,
                                    crate::quest_editor::RewardType::Gold,
                                    "Gold",
                                );
                                ui.selectable_value(
                                    &mut self.reward_buffer.reward_type,
                                    crate::quest_editor::RewardType::Items,
                                    "Items",
                                );
                                ui.selectable_value(
                                    &mut self.reward_buffer.reward_type,
                                    crate::quest_editor::RewardType::UnlockQuest,
                                    "Unlock Quest",
                                );
                                ui.selectable_value(
                                    &mut self.reward_buffer.reward_type,
                                    crate::quest_editor::RewardType::SetFlag,
                                    "Set Flag",
                                );
                                ui.selectable_value(
                                    &mut self.reward_buffer.reward_type,
                                    crate::quest_editor::RewardType::Reputation,
                                    "Reputation",
                                );
                            });
                    });

                    ui.separator();

                    match self.reward_buffer.reward_type {
                        crate::quest_editor::RewardType::Experience => {
                            ui.horizontal(|ui| {
                                ui.label("Amount:");
                                ui.text_edit_singleline(&mut self.reward_buffer.experience);
                            });
                        }
                        crate::quest_editor::RewardType::Gold => {
                            ui.horizontal(|ui| {
                                ui.label("Amount:");
                                ui.text_edit_singleline(&mut self.reward_buffer.gold);
                            });
                        }
                        crate::quest_editor::RewardType::Items => {
                            ui.horizontal(|ui| {
                                ui.label("Item:");
                                egui::ComboBox::from_id_salt("reward_item_selector")
                                    .selected_text(
                                        items
                                            .iter()
                                            .find(|i| {
                                                i.id.to_string() == self.reward_buffer.item_id
                                            })
                                            .map(|i| format!("{} - {}", i.id, i.name))
                                            .unwrap_or_else(|| self.reward_buffer.item_id.clone()),
                                    )
                                    .show_ui(ui, |ui| {
                                        for item in items {
                                            ui.selectable_value(
                                                &mut self.reward_buffer.item_id,
                                                item.id.to_string(),
                                                format!("{} - {}", item.id, item.name),
                                            );
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("Quantity:");
                                ui.text_edit_singleline(&mut self.reward_buffer.item_quantity);
                            });
                        }
                        crate::quest_editor::RewardType::UnlockQuest => {
                            ui.horizontal(|ui| {
                                ui.label("Quest:");
                                egui::ComboBox::from_id_salt("reward_quest_selector")
                                    .selected_text(
                                        quests
                                            .iter()
                                            .find(|q| {
                                                q.id.to_string()
                                                    == self.reward_buffer.unlock_quest_id
                                            })
                                            .map(|q| format!("{} - {}", q.id, q.name))
                                            .unwrap_or_else(|| {
                                                self.reward_buffer.unlock_quest_id.clone()
                                            }),
                                    )
                                    .show_ui(ui, |ui| {
                                        for quest in quests.iter() {
                                            ui.selectable_value(
                                                &mut self.reward_buffer.unlock_quest_id,
                                                quest.id.to_string(),
                                                format!("{} - {}", quest.id, quest.name),
                                            );
                                        }
                                    });
                            });
                        }
                        crate::quest_editor::RewardType::SetFlag => {
                            ui.horizontal(|ui| {
                                ui.label("Flag Name:");
                                ui.text_edit_singleline(&mut self.reward_buffer.flag_name);
                            });
                            ui.checkbox(&mut self.reward_buffer.flag_value, "Value");
                        }
                        crate::quest_editor::RewardType::Reputation => {
                            ui.horizontal(|ui| {
                                ui.label("Faction:");
                                ui.text_edit_singleline(&mut self.reward_buffer.faction_name);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Change:");
                                ui.text_edit_singleline(&mut self.reward_buffer.reputation_change);
                            });
                        }
                    }

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("✅ Save").clicked() {
                            if let Some(selected_idx) = self.selected_quest {
                                if self.save_reward(quests, selected_idx, reward_idx).is_ok() {
                                    *unsaved_changes = true;
                                }
                            }
                        }
                        if ui.button("❌ Cancel").clicked() {
                            self.selected_reward = None;
                        }
                    });
                });
        }
    }

    /// Show quest validation display
    fn show_quest_validation(&mut self, ui: &mut egui::Ui) {
        self.validate_current_quest();
        let errors = &self.validation_errors;

        if !errors.is_empty() {
            ui.group(|ui| {
                ui.label("⚠️ Validation Errors:");
                ui.separator();

                for error in errors {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 100, 100),
                        format!("• {}", error),
                    );
                }
            });
        } else if self.selected_quest.is_some() {
            ui.group(|ui| {
                ui.colored_label(egui::Color32::from_rgb(100, 255, 100), "✅ Quest is valid");
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use antares::domain::types::Position;

    #[test]
    fn test_quest_editor_state_creation() {
        let editor = QuestEditorState::new();
        assert_eq!(editor.quests.len(), 0);
        assert_eq!(editor.mode, QuestEditorMode::List);
    }

    #[test]
    fn test_start_new_quest() {
        let mut editor = QuestEditorState::new();
        editor.start_new_quest("1".to_string());
        assert_eq!(editor.mode, QuestEditorMode::Creating);
    }

    #[test]
    fn test_save_quest_creates_new() {
        let mut editor = QuestEditorState::new();
        editor.start_new_quest("1".to_string());
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
        editor.start_new_quest("1".to_string());
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
        editor.start_new_quest("1".to_string());
        editor.quest_buffer.id = "1".to_string();
        editor.quest_buffer.name = "Dragon Slayer".to_string();
        editor.save_quest().unwrap();

        editor.start_new_quest("2".to_string());
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
        editor.start_new_quest("1".to_string());
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
        editor.start_new_quest("1".to_string());
        editor.quest_buffer.id = "1".to_string();
        editor.quest_buffer.name = "Test".to_string();
        editor.save_quest().unwrap();

        editor.selected_quest = Some(0);
        editor.validate_current_quest();
        assert!(!editor.validation_errors.is_empty());
    }

    #[test]
    fn test_quest_buffer_levels() {
        let buffer = QuestEditBuffer {
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

    #[test]
    fn test_edit_stage() {
        let mut state = QuestEditorState::new();
        let mut quest = Quest::new(1, "Test Quest", "Test description");
        let mut stage = QuestStage::new(1, "Stage 1");
        stage.description = "Test stage description".to_string();
        quest.add_stage(stage);
        state.quests.push(quest);

        // Edit the stage
        assert!(state.edit_stage(0, 0).is_ok());
        assert_eq!(state.stage_buffer.number, "1");
        assert_eq!(state.stage_buffer.name, "Stage 1");
        assert_eq!(state.stage_buffer.description, "Test stage description");
        assert_eq!(state.selected_stage, Some(0));
    }

    #[test]
    fn test_delete_stage() {
        let mut state = QuestEditorState::new();
        let mut quest = Quest::new(1, "Test Quest", "Test description");
        quest.add_stage(QuestStage::new(1, "Stage 1"));
        quest.add_stage(QuestStage::new(2, "Stage 2"));
        state.quests.push(quest);

        assert_eq!(state.quests[0].stages.len(), 2);

        // Delete first stage
        assert!(state.delete_stage(0, 0).is_ok());
        assert_eq!(state.quests[0].stages.len(), 1);
        assert_eq!(state.quests[0].stages[0].stage_number, 2);
    }

    #[test]
    fn test_edit_objective() {
        use antares::domain::quest::QuestObjective;

        let mut state = QuestEditorState::new();
        let mut quest = Quest::new(1, "Test Quest", "Test description");
        let mut stage = QuestStage::new(1, "Stage 1");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 5,
            quantity: 10,
        });
        quest.add_stage(stage);
        state.quests.push(quest);

        // Edit the objective
        assert!(state.edit_objective(0, 0, 0).is_ok());
        assert_eq!(state.objective_buffer.monster_id, "5");
        assert_eq!(state.objective_buffer.quantity, "10");
        assert_eq!(state.selected_objective, Some(0));
    }

    #[test]
    fn test_delete_objective() {
        use antares::domain::quest::QuestObjective;

        let mut state = QuestEditorState::new();
        let mut quest = Quest::new(1, "Test Quest", "Test description");
        let mut stage = QuestStage::new(1, "Stage 1");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 5,
            quantity: 10,
        });
        stage.add_objective(QuestObjective::CollectItems {
            item_id: 3,
            quantity: 5,
        });
        quest.add_stage(stage);
        state.quests.push(quest);

        assert_eq!(state.quests[0].stages[0].objectives.len(), 2);

        // Delete first objective
        assert!(state.delete_objective(0, 0, 0).is_ok());
        assert_eq!(state.quests[0].stages[0].objectives.len(), 1);
    }

    #[test]
    fn test_find_orphaned_objectives() {
        let mut state = QuestEditorState::new();

        // Quest with valid stages
        let mut quest1 = Quest::new(1, "Quest 1", "Test");
        let mut stage1 = QuestStage::new(1, "Stage 1");
        stage1.add_objective(QuestObjective::KillMonsters {
            monster_id: 1,
            quantity: 5,
        });
        quest1.add_stage(stage1);
        state.quests.push(quest1);

        // Quest with orphaned stage
        let mut quest2 = Quest::new(2, "Quest 2", "Test");
        quest2.add_stage(QuestStage::new(1, "Empty Stage"));
        state.quests.push(quest2);

        let orphaned = state.find_orphaned_objectives();
        assert_eq!(orphaned.len(), 1);
        assert_eq!(orphaned[0], (2, 1));
    }

    #[test]
    fn test_save_edited_objective() {
        use antares::domain::quest::QuestObjective;

        let mut state = QuestEditorState::new();
        let mut quest = Quest::new(1, "Test Quest", "Test description");
        let mut stage = QuestStage::new(1, "Stage 1");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 5,
            quantity: 10,
        });
        quest.add_stage(stage);
        state.quests.push(quest);

        // Edit and save the objective
        state.edit_objective(0, 0, 0).unwrap();
        state.objective_buffer.monster_id = "8".to_string();
        state.objective_buffer.quantity = "15".to_string();

        assert!(state.save_objective(0, 0, 0).is_ok());

        // Verify changes
        match &state.quests[0].stages[0].objectives[0] {
            QuestObjective::KillMonsters {
                monster_id,
                quantity,
            } => {
                assert_eq!(*monster_id, 8);
                assert_eq!(*quantity, 15);
            }
            _ => panic!("Expected KillMonsters objective"),
        }
    }
}
