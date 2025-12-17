// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Quest tracking system (application layer)
//!
//! This module implements a lightweight quest tracking system that listens for
//! world events (monster kills, item collections, location reached) and
//! updates quest progress accordingly. When a quest completes its rewards are
//! applied to the `GameState` (gold, items, experience).
//!
//! The implementation is intentionally conservative: it keeps progress using
//! the `domain::quest::QuestProgress` structure and exposes a small API that
//! can be invoked directly from tests or from an ECS system wrapper.
//!
//! # Design
//!
//! - `QuestSystem` is a Bevy resource that stores progress for active quests.
//! - `QuestProgressEvent` is the message type consumed by the ECS system.
//! - `update` is the Bevy system entry-point that reads events and dispatches
//!   them into `QuestSystem::process_event`.
//!
//! # Testing
//!
//! Unit tests exercise progress updates, completion and reward application.

use bevy::prelude::*;
use std::collections::HashMap;

use crate::application::resources::GameContent;

use crate::domain::quest::{
    Quest as DomainQuest, QuestId as DomainQuestId, QuestObjective, QuestProgress, QuestReward,
};
use crate::domain::types::{ItemId, MapId, MonsterId, Position};
use crate::game::resources::GlobalState;
use crate::sdk::database::ContentDatabase;

/// Events that contribute to quest progress.
///
/// These are emitted by gameplay systems (combat, map events, item pickup logic).
#[derive(Message, Clone, Debug)]
pub enum QuestProgressEvent {
    /// Some number of monsters of `monster_id` were killed.
    MonsterKilled { monster_id: MonsterId, count: u16 },

    /// The player collected an item (e.g., picked up from ground).
    ItemCollected { item_id: ItemId, count: u16 },

    /// The player or party reached a location on a map.
    LocationReached { map_id: MapId, position: Position },
}

/// Tracks progress for active quests.
///
/// Stores a mapping from `DomainQuestId` -> `QuestProgress` (domain progress
/// struct). This resource is intended to be inserted into Bevy's ECS world
/// and updated by the `update` system when `QuestProgressEvent`s are emitted.
#[derive(Resource, Debug, Default)]
pub struct QuestSystem {
    /// Active quest progress entries keyed by QuestId
    pub progress: HashMap<DomainQuestId, QuestProgress>,
}

impl QuestSystem {
    /// Creates a new, empty `QuestSystem`.
    pub fn new() -> Self {
        Self {
            progress: HashMap::new(),
        }
    }

    /// Starts tracking a quest with the given `quest_id`.
    ///
    /// This:
    /// 1. Validates the quest exists in `content_db`.
    /// 2. Adds a textual representation to `game_state.quests` (so UI shows it).
    /// 3. Inserts a fresh `QuestProgress` to this `QuestSystem`.
    ///
    /// Returns `Err(String)` if the quest doesn't exist.
    pub fn start_quest(
        &mut self,
        quest_id: DomainQuestId,
        game_state: &mut crate::application::GameState,
        content_db: &ContentDatabase,
    ) -> Result<(), String> {
        // Lookup quest
        let quest = content_db
            .quests
            .get_quest(quest_id)
            .ok_or_else(|| format!("Quest {} not found in content database", quest_id))?
            .clone();

        // Build an application-level Quest entry for quest log
        let mut app_quest = crate::application::Quest::new(
            quest_id.to_string(),
            quest.name.clone(),
            quest.description.clone(),
        );

        // Populate readable objectives
        for stage in &quest.stages {
            for obj in &stage.objectives {
                app_quest.add_objective(obj.description());
            }
        }

        // Add to game state's active quest list (UI / persistence)
        game_state.quests.add_quest(app_quest);

        // Add progress tracker
        self.progress
            .entry(quest_id)
            .or_insert_with(|| QuestProgress::new(quest_id));

        Ok(())
    }

    /// Processes a single quest progress event.
    ///
    /// This method is the core logic that maps world events to updates on
    /// `QuestProgress`. It will:
    /// - Update objective progress counts
    /// - Advance quest stages when objectives are satisfied
    /// - Complete quests and grant rewards when the final stage finishes
    pub fn process_event(
        &mut self,
        ev: &QuestProgressEvent,
        game_state: &mut crate::application::GameState,
        content_db: &ContentDatabase,
    ) {
        // For each active quest
        let quest_ids: Vec<DomainQuestId> = self.progress.keys().copied().collect();

        for qid in quest_ids {
            // Borrow progress mutably
            if let Some(progress) = self.progress.get_mut(&qid) {
                // Lookup domain quest
                if let Some(domain_quest) = content_db.quests.get_quest(qid) {
                    // Current stage (1-based in QuestProgress)
                    let stage_idx = (progress.current_stage as usize).saturating_sub(1);

                    if stage_idx >= domain_quest.stages.len() {
                        // Invalid stage (shouldn't happen), mark complete defensively
                        progress.complete();
                        continue;
                    }

                    let stage = &domain_quest.stages[stage_idx];

                    // Evaluate each objective
                    for (obj_idx, obj) in stage.objectives.iter().enumerate() {
                        let already_done = progress.get_objective_progress(obj_idx)
                            >= Self::objective_goal_count(obj);

                        if already_done {
                            // Skip already satisfied objectives
                            continue;
                        }

                        match (obj, ev) {
                            (
                                QuestObjective::KillMonsters {
                                    monster_id,
                                    quantity,
                                },
                                QuestProgressEvent::MonsterKilled {
                                    monster_id: m,
                                    count,
                                },
                            ) => {
                                if monster_id == m {
                                    // Add counts and clamp at required quantity
                                    let new_count = progress
                                        .get_objective_progress(obj_idx)
                                        .saturating_add(*count as u32);
                                    let capped = new_count.min(*quantity as u32);
                                    progress.update_objective(obj_idx, capped);
                                }
                            }
                            (
                                QuestObjective::CollectItems { item_id, quantity },
                                QuestProgressEvent::ItemCollected {
                                    item_id: iid,
                                    count,
                                },
                            ) => {
                                if item_id == iid {
                                    let new_count = progress
                                        .get_objective_progress(obj_idx)
                                        .saturating_add(*count as u32);
                                    let capped = new_count.min(*quantity as u32);
                                    progress.update_objective(obj_idx, capped);
                                }
                            }
                            (
                                QuestObjective::ReachLocation {
                                    map_id,
                                    position,
                                    radius,
                                },
                                QuestProgressEvent::LocationReached {
                                    map_id: mid,
                                    position: pos,
                                },
                            ) => {
                                if map_id == mid {
                                    let dx = pos.x - position.x;
                                    let dy = pos.y - position.y;
                                    let dist_sq = dx * dx + dy * dy;
                                    let radius_i32 = *radius as i32;
                                    if dist_sq.abs() <= radius_i32 * radius_i32 {
                                        // Mark objective as completed (use 1 to indicate completion)
                                        progress.update_objective(obj_idx, 1);
                                    }
                                }
                            }
                            // For other objective types (TalkToNpc, DeliverItem, EscortNpc, CustomFlag)
                            // we leave them unhandled here - they can be supported later.
                            _ => {}
                        }
                    }

                    // Check if stage objectives are satisfied
                    let stage_completed = if stage.require_all_objectives {
                        (0..stage.objectives.len()).all(|i| {
                            progress.get_objective_progress(i)
                                >= Self::objective_goal_count(&stage.objectives[i])
                        })
                    } else {
                        (0..stage.objectives.len()).any(|i| {
                            progress.get_objective_progress(i)
                                >= Self::objective_goal_count(&stage.objectives[i])
                        })
                    };

                    if stage_completed {
                        // Advance to next stage or complete quest
                        if (progress.current_stage as usize) < domain_quest.stages.len() {
                            progress.advance_stage();
                        } else {
                            // Complete quest
                            progress.complete();

                            // Apply rewards
                            self.apply_rewards(domain_quest, game_state);

                            // Move quest from active -> completed in game_state.quests
                            let qid_str = qid.to_string();
                            game_state.quests.complete_quest(&qid_str);
                        }
                    }
                }
            }
        }
    }

    /// Returns the numeric goal count for an objective (used for comparisons).
    fn objective_goal_count(obj: &QuestObjective) -> u32 {
        match obj {
            QuestObjective::KillMonsters { quantity, .. } => *quantity as u32,
            QuestObjective::CollectItems { quantity, .. } => *quantity as u32,
            // ReachLocation is boolean (1 = reached)
            QuestObjective::ReachLocation { .. } => 1,
            QuestObjective::TalkToNpc { .. } => 1,
            QuestObjective::DeliverItem { quantity, .. } => *quantity as u32,
            QuestObjective::EscortNpc { .. } => 1,
            QuestObjective::CustomFlag { .. } => 1,
        }
    }

    /// Grants quest rewards to the `GameState`.
    ///
    /// This function applies common reward types:
    /// - `Experience` → added to first living party member's experience
    /// - `Gold` → added to party gold
    /// - `Items` → added to the first party member's inventory (single slot with charges)
    /// - `SetFlag`/`Reputation`/`UnlockQuest` are handled conservatively
    fn apply_rewards(&self, quest: &DomainQuest, game_state: &mut crate::application::GameState) {
        for reward in &quest.rewards {
            match reward {
                QuestReward::Experience(amount) => {
                    if let Some(ch) = game_state.party.members.get_mut(0) {
                        ch.experience = ch.experience.saturating_add(*amount as u64);
                    }
                }
                QuestReward::Gold(amount) => {
                    game_state.party.gold = game_state.party.gold.saturating_add(*amount);
                }
                QuestReward::Items(items) => {
                    if let Some(ch) = game_state.party.members.get_mut(0) {
                        for (item_id, qty) in items {
                            // Use single slot with charges = qty (domain Inventory supports charges)
                            let _ = ch.inventory.add_item(*item_id, *qty as u8);
                        }
                    }
                }
                QuestReward::UnlockQuest(_qid) => {
                    // Unlocking quests requires campaign logic; for now it's a no-op.
                    // TODO: integrate with quest availability system.
                }
                QuestReward::SetFlag { flag_name, value } => {
                    // Simple global flag handling is not implemented in GameState yet.
                    // We log a message for visibility (tests don't rely on flags).
                    println!("Quest reward sets flag '{}' = {}", flag_name, value);
                }
                QuestReward::Reputation { faction, change } => {
                    println!(
                        "Quest reward changes reputation with {} by {}",
                        faction, change
                    );
                }
            }
        }
    }
}

/// ECS system wrapper that consumes `QuestProgressEvent` messages and updates the
/// `QuestSystem` resource accordingly.
///
/// # Notes
///
/// This function is designed to be registered as a normal Bevy system:
/// ```no_run
/// app.add_message::<QuestProgressEvent>()
///    .add_systems(Update, crate::application::quests::update);
/// ```
pub fn update(
    mut quest_system: ResMut<QuestSystem>,
    mut global_state: ResMut<GlobalState>,
    content: Res<GameContent>,
    mut ev_reader: MessageReader<QuestProgressEvent>,
) {
    for ev in ev_reader.read() {
        quest_system.process_event(ev, &mut global_state.0, content.db());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::quest::{Quest, QuestStage};
    use crate::sdk::database::ContentDatabase;

    #[test]
    fn test_quest_progress_updates_on_event() {
        // Build content DB with a simple quest (kill 3 of monster 7)
        let mut db = ContentDatabase::new();
        let mut q = Quest::new(1, "Goblin Hunt", "Kill goblins in the forest");
        let mut stage = QuestStage::new(1, "Slay Goblins");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 7,
            quantity: 3,
        });
        q.add_stage(stage);
        db.quests.add_quest(q);

        // Game state and quest system
        let mut gs = crate::application::GameState::new();
        // Add a party member so rewards can be applied later
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        let mut qs = QuestSystem::new();
        qs.start_quest(1, &mut gs, &db).expect("start_quest failed");

        // Fire a single monster kill event
        let ev = QuestProgressEvent::MonsterKilled {
            monster_id: 7,
            count: 1,
        };
        qs.process_event(&ev, &mut gs, &db);

        // Progress should increment for objective 0
        let p = qs.progress.get(&1).expect("progress entry missing");
        assert_eq!(p.get_objective_progress(0), 1);
    }

    #[test]
    fn test_quest_completion_grants_rewards() {
        // Quest: kill 1 goblin, reward 100 gold and item 42 x1
        let mut db = ContentDatabase::new();
        let mut q = Quest::new(2, "Quick Quest", "A short test quest");
        let mut stage = QuestStage::new(1, "Do it");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 9,
            quantity: 1,
        });
        q.add_stage(stage);
        q.add_reward(QuestReward::Gold(100));
        q.add_reward(QuestReward::Items(vec![(42 as ItemId, 1)]));
        db.quests.add_quest(q);

        let mut gs = crate::application::GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        let mut qs = QuestSystem::new();
        qs.start_quest(2, &mut gs, &db).expect("start_quest failed");

        // Kill the required monster
        let ev = QuestProgressEvent::MonsterKilled {
            monster_id: 9,
            count: 1,
        };
        qs.process_event(&ev, &mut gs, &db);

        // Quest should be marked complete in progress
        let p = qs.progress.get(&2).expect("progress entry missing");
        assert!(p.completed);

        // The quest should have been moved to completed_quests in game state
        assert!(gs.quests.completed_quests.contains(&"2".to_string()));

        // Gold reward applied
        assert_eq!(gs.party.gold, 100);

        // Item reward applied to first party member
        let got_item = gs.party.members[0]
            .inventory
            .items
            .iter()
            .any(|slot| slot.item_id == 42);
        assert!(got_item);
    }

    #[test]
    fn test_quest_multiple_objectives_tracking() {
        // Quest: kill 2 of monster 11 AND collect item 5
        let mut db = ContentDatabase::new();
        let mut q = Quest::new(3, "Compound Task", "Multi objective");
        let mut stage = QuestStage::new(1, "Multiple");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 11,
            quantity: 2,
        });
        stage.add_objective(QuestObjective::CollectItems {
            item_id: 5,
            quantity: 1,
        });
        q.add_stage(stage);
        q.add_reward(QuestReward::Gold(50));
        db.quests.add_quest(q);

        let mut gs = crate::application::GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        let mut qs = QuestSystem::new();
        qs.start_quest(3, &mut gs, &db).expect("start_quest failed");

        // Kill two monsters (send as two events)
        qs.process_event(
            &QuestProgressEvent::MonsterKilled {
                monster_id: 11,
                count: 1,
            },
            &mut gs,
            &db,
        );
        // Not complete yet
        let p = qs.progress.get(&3).unwrap();
        assert!(!p.completed);

        // Collect the item
        qs.process_event(
            &QuestProgressEvent::ItemCollected {
                item_id: 5,
                count: 1,
            },
            &mut gs,
            &db,
        );

        // Still not complete until second monster
        let p = qs.progress.get(&3).unwrap();
        assert!(!p.completed);

        // Kill the second monster
        qs.process_event(
            &QuestProgressEvent::MonsterKilled {
                monster_id: 11,
                count: 1,
            },
            &mut gs,
            &db,
        );

        let p = qs.progress.get(&3).unwrap();
        assert!(p.completed);

        // Reward applied
        assert_eq!(gs.party.gold, 50);
        assert!(gs.quests.completed_quests.contains(&"3".to_string()));
    }
}
