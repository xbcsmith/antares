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
    /// Returns `Err(ValidationError::NotFound)` if the quest doesn't exist.
    pub fn start_quest(
        &mut self,
        quest_id: DomainQuestId,
        game_state: &mut crate::application::GameState,
        content_db: &ContentDatabase,
    ) -> Result<(), crate::domain::validation::ValidationError> {
        // Lookup quest
        let quest = content_db
            .quests
            .get_quest(quest_id)
            .ok_or_else(|| {
                crate::domain::validation::ValidationError::NotFound(format!(
                    "Quest {} not found in content database",
                    quest_id
                ))
            })?
            .clone();

        // Build an application-level Quest entry for quest log
        let mut app_quest = crate::application::Quest::new(
            quest_id.to_string(),
            quest.name.clone(),
            quest.description.clone(),
        );

        // Populate readable objectives, preserving map/location metadata where
        // the domain objective provides it so automap / mini-map POIs can be
        // reconstructed from the persisted application quest log.
        for stage in &quest.stages {
            for obj in &stage.objectives {
                let (map_id, position) = match obj {
                    QuestObjective::ReachLocation {
                        map_id, position, ..
                    } => (Some(*map_id), Some(*position)),
                    QuestObjective::EscortNpc {
                        map_id, position, ..
                    } => (Some(*map_id), Some(*position)),
                    _ => (None, None),
                };

                app_quest.add_objective_with_location(obj.description(), map_id, position);
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
                            self.apply_rewards(domain_quest, game_state, content_db);

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
    /// - `LearnSpell` → taught to the first eligible party member via `learn_spell`
    /// - `SetFlag`/`Reputation`/`UnlockQuest` are handled conservatively
    fn apply_rewards(
        &self,
        quest: &DomainQuest,
        game_state: &mut crate::application::GameState,
        content_db: &ContentDatabase,
    ) {
        for reward in &quest.rewards {
            match reward {
                QuestReward::Experience(amount) => {
                    // Scale by the campaign experience_rate so that a doubled XP
                    // campaign awards 2× quest XP just as it awards 2× combat XP.
                    let rate = game_state.campaign_config.experience_rate;
                    let scaled = (*amount as f64 * rate as f64).round() as u64;
                    if let Some(ch) = game_state.party.members.get_mut(0) {
                        ch.experience = ch.experience.saturating_add(scaled);
                    }
                }
                QuestReward::Gold(amount) => {
                    game_state.party.gold = game_state.party.gold.saturating_add(*amount);
                }
                QuestReward::Items(items) => {
                    if let Some(ch) = game_state.party.members.get_mut(0) {
                        for (item_id, qty) in items {
                            // Use single slot with charges = qty (domain Inventory supports charges)
                            if let Err(e) = ch.inventory.add_item(*item_id, *qty as u8) {
                                tracing::warn!(
                                    "Quest reward: failed to add item {:?} (qty {}): {}",
                                    item_id,
                                    qty,
                                    e
                                );
                            }
                        }
                    }
                }
                QuestReward::UnlockQuest(qid) => {
                    game_state.quests.unlock_quest(*qid);
                    tracing::info!(
                        "Quest reward unlocked quest {} for future availability",
                        qid
                    );
                }
                QuestReward::SetFlag { flag_name, value } => {
                    // Simple global flag handling is not implemented in GameState yet.
                    // We log a message for visibility (tests don't rely on flags).
                    tracing::warn!(
                        "Quest reward sets flag '{}' = {} (not yet persisted)",
                        flag_name,
                        value
                    );
                }
                QuestReward::Reputation { faction, change } => {
                    tracing::warn!(
                        "Quest reward changes reputation with {} by {} (not yet implemented)",
                        faction,
                        change
                    );
                }
                QuestReward::LearnSpell { spell_id } => {
                    // Try to teach the spell to the first eligible party member
                    let spell_db = &content_db.spells;
                    let class_db = &content_db.classes;
                    let mut taught = false;
                    for character in &mut game_state.party.members {
                        match crate::domain::magic::learning::learn_spell(
                            character, *spell_id, spell_db, class_db,
                        ) {
                            Ok(()) => {
                                let spell_name = spell_db
                                    .get_spell(*spell_id)
                                    .map(|s| s.name.clone())
                                    .unwrap_or_else(|| spell_id.to_string());
                                tracing::info!(
                                    "Quest reward: {} learned spell '{}'",
                                    character.name,
                                    spell_name
                                );
                                taught = true;
                                break;
                            }
                            Err(crate::domain::magic::learning::SpellLearnError::AlreadyKnown(
                                _,
                            )) => {
                                // Already knows this spell; try next member
                                continue;
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Quest reward: cannot teach spell {} to {}: {}",
                                    spell_id,
                                    character.name,
                                    e
                                );
                                // Keep trying other party members for class/level mismatches
                                continue;
                            }
                        }
                    }
                    if !taught {
                        tracing::warn!(
                            "Quest reward: spell {} could not be taught to any party member",
                            spell_id
                        );
                    }
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
/// use bevy::prelude::{App, Update};
/// use antares::application::quests::QuestProgressEvent;
///
/// let mut app = App::new();
/// app.add_message::<QuestProgressEvent>()
///    .add_systems(Update, antares::application::quests::update);
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

    #[test]
    fn test_unlock_quest_reward_makes_quest_available() {
        // Quest 1: kill 1 monster, reward = UnlockQuest(2)
        let mut db = ContentDatabase::new();
        let mut q = Quest::new(1, "First Quest", "Start here");
        let mut stage = QuestStage::new(1, "Do task");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 1,
            quantity: 1,
        });
        q.add_stage(stage);
        q.add_reward(QuestReward::UnlockQuest(2));
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
        qs.start_quest(1, &mut gs, &db).expect("start_quest");

        // Quest 2 should not yet be available
        assert!(!gs.quests.is_quest_available(2));

        // Complete quest 1
        qs.process_event(
            &QuestProgressEvent::MonsterKilled {
                monster_id: 1,
                count: 1,
            },
            &mut gs,
            &db,
        );

        // Quest 1 should be completed
        assert!(gs.quests.completed_quests.contains(&"1".to_string()));

        // Quest 2 should now be available via UnlockQuest reward
        assert!(gs.quests.is_quest_available(2));
    }

    #[test]
    fn test_unlock_quest_reward_multiple_unlocks() {
        let mut db = ContentDatabase::new();
        let mut q = Quest::new(10, "Multi Unlock", "Unlocks two quests");
        let mut stage = QuestStage::new(1, "Finish");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 3,
            quantity: 1,
        });
        q.add_stage(stage);
        q.add_reward(QuestReward::UnlockQuest(20));
        q.add_reward(QuestReward::UnlockQuest(30));
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
        qs.start_quest(10, &mut gs, &db).expect("start_quest");

        qs.process_event(
            &QuestProgressEvent::MonsterKilled {
                monster_id: 3,
                count: 1,
            },
            &mut gs,
            &db,
        );

        assert!(gs.quests.is_quest_available(20));
        assert!(gs.quests.is_quest_available(30));
        assert!(!gs.quests.is_quest_available(40));
    }

    // ===== QuestReward::LearnSpell tests =====

    fn build_db_with_spells() -> ContentDatabase {
        let mut db = ContentDatabase::new();
        let classes = crate::domain::classes::ClassDatabase::load_from_file("data/classes.ron")
            .expect("data/classes.ron must exist");
        db.classes = classes;
        use crate::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};
        db.spells
            .add_spell(Spell::new(
                0x0101,
                "Cure Wounds",
                SpellSchool::Cleric,
                1,
                2,
                0,
                SpellContext::Anytime,
                SpellTarget::SingleCharacter,
                "Heals 8 HP",
                None,
                0,
                false,
            ))
            .unwrap();
        db.spells
            .add_spell(Spell::new(
                0x0501,
                "Magic Arrow",
                SpellSchool::Sorcerer,
                1,
                2,
                0,
                SpellContext::CombatOnly,
                SpellTarget::SingleMonster,
                "Deals magic damage",
                None,
                0,
                false,
            ))
            .unwrap();
        db
    }

    fn make_learn_spell_quest(quest_id: u16, spell_id: u16) -> Quest {
        let mut q = Quest::new(quest_id, "Spell Quest", "Learn a spell as reward");
        let mut stage = QuestStage::new(1, "Complete task");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 1,
            quantity: 1,
        });
        q.add_stage(stage);
        q.add_reward(QuestReward::LearnSpell { spell_id });
        q
    }

    #[test]
    fn test_quest_reward_learn_spell_teaches_cleric_spell() {
        let mut db = build_db_with_spells();
        db.quests.add_quest(make_learn_spell_quest(40, 0x0101));

        let mut gs = crate::application::GameState::new();
        let cleric = Character::new(
            "Aria".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        gs.party.add_member(cleric).unwrap();

        let mut qs = QuestSystem::new();
        qs.start_quest(40, &mut gs, &db)
            .expect("start_quest failed");
        qs.process_event(
            &QuestProgressEvent::MonsterKilled {
                monster_id: 1,
                count: 1,
            },
            &mut gs,
            &db,
        );

        // Cleric's spellbook should contain the learned spell at level index 0
        assert!(gs.party.members[0].spells.cleric_spells[0].contains(&0x0101));
    }

    #[test]
    fn test_quest_reward_learn_spell_skips_wrong_class_tries_next_member() {
        let mut db = build_db_with_spells();
        db.quests.add_quest(make_learn_spell_quest(41, 0x0101));

        let mut gs = crate::application::GameState::new();
        // First member is a knight (cannot learn cleric spells)
        let knight = Character::new(
            "Tank".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Second member is a cleric (can learn cleric spells)
        let cleric = Character::new(
            "Healer".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        gs.party.add_member(knight).unwrap();
        gs.party.add_member(cleric).unwrap();

        let mut qs = QuestSystem::new();
        qs.start_quest(41, &mut gs, &db)
            .expect("start_quest failed");
        qs.process_event(
            &QuestProgressEvent::MonsterKilled {
                monster_id: 1,
                count: 1,
            },
            &mut gs,
            &db,
        );

        // Knight's spellbook must remain empty
        assert!(gs.party.members[0].spells.cleric_spells[0].is_empty());
        // Cleric should have the spell
        assert!(gs.party.members[1].spells.cleric_spells[0].contains(&0x0101));
    }

    #[test]
    fn test_quest_reward_learn_spell_no_eligible_member_is_silent() {
        let mut db = build_db_with_spells();
        // Quest rewards a cleric spell, but party is all knights
        db.quests.add_quest(make_learn_spell_quest(42, 0x0101));

        let mut gs = crate::application::GameState::new();
        let knight = Character::new(
            "Sir Iron".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(knight).unwrap();

        let mut qs = QuestSystem::new();
        qs.start_quest(42, &mut gs, &db)
            .expect("start_quest failed");
        // Should not panic even when no eligible member exists
        qs.process_event(
            &QuestProgressEvent::MonsterKilled {
                monster_id: 1,
                count: 1,
            },
            &mut gs,
            &db,
        );

        // No spells should have been added
        assert!(gs.party.members[0].spells.cleric_spells[0].is_empty());
        assert!(gs.party.members[0].spells.sorcerer_spells[0].is_empty());
    }

    #[test]
    fn test_quest_reward_learn_spell_already_known_skips_gracefully() {
        let mut db = build_db_with_spells();
        db.quests.add_quest(make_learn_spell_quest(43, 0x0101));

        let mut gs = crate::application::GameState::new();
        let mut cleric = Character::new(
            "OldSage".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        // Pre-fill the spell so it's already known
        cleric.spells.cleric_spells[0].push(0x0101);
        gs.party.add_member(cleric).unwrap();

        let mut qs = QuestSystem::new();
        qs.start_quest(43, &mut gs, &db)
            .expect("start_quest failed");
        qs.process_event(
            &QuestProgressEvent::MonsterKilled {
                monster_id: 1,
                count: 1,
            },
            &mut gs,
            &db,
        );

        // Spell should still appear exactly once — no duplicate
        assert_eq!(
            gs.party.members[0].spells.cleric_spells[0]
                .iter()
                .filter(|&&id| id == 0x0101)
                .count(),
            1
        );
    }

    #[test]
    fn test_quest_reward_learn_sorcerer_spell_for_sorcerer() {
        let mut db = build_db_with_spells();
        db.quests.add_quest(make_learn_spell_quest(44, 0x0501));

        let mut gs = crate::application::GameState::new();
        let sorc = Character::new(
            "Mage".to_string(),
            "human".to_string(),
            "sorcerer".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        gs.party.add_member(sorc).unwrap();

        let mut qs = QuestSystem::new();
        qs.start_quest(44, &mut gs, &db)
            .expect("start_quest failed");
        qs.process_event(
            &QuestProgressEvent::MonsterKilled {
                monster_id: 1,
                count: 1,
            },
            &mut gs,
            &db,
        );

        assert!(gs.party.members[0].spells.sorcerer_spells[0].contains(&0x0501));
    }

    // ===== Phase 2: experience_rate scaling tests =====

    #[test]
    fn test_quest_experience_reward_scaled_by_experience_rate() {
        // Quest: kill 1 monster, reward 100 XP
        let mut db = ContentDatabase::new();
        let mut q = Quest::new(100, "XP Quest", "Awards experience");
        let mut stage = QuestStage::new(1, "Kill something");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 5,
            quantity: 1,
        });
        q.add_stage(stage);
        q.add_reward(QuestReward::Experience(100));
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

        // Double XP rate
        gs.campaign_config.experience_rate = 2.0;

        let mut qs = QuestSystem::new();
        qs.start_quest(100, &mut gs, &db)
            .expect("start_quest failed");
        qs.process_event(
            &QuestProgressEvent::MonsterKilled {
                monster_id: 5,
                count: 1,
            },
            &mut gs,
            &db,
        );

        // 100 XP * 2.0 rate = 200
        assert_eq!(
            gs.party.members[0].experience, 200,
            "experience_rate = 2.0 must double quest XP (100 → 200)"
        );
    }

    #[test]
    fn test_quest_experience_reward_halved_by_experience_rate() {
        let mut db = ContentDatabase::new();
        let mut q = Quest::new(101, "Half XP Quest", "Awards experience at half rate");
        let mut stage = QuestStage::new(1, "Kill something");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 6,
            quantity: 1,
        });
        q.add_stage(stage);
        q.add_reward(QuestReward::Experience(100));
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

        // Half XP rate
        gs.campaign_config.experience_rate = 0.5;

        let mut qs = QuestSystem::new();
        qs.start_quest(101, &mut gs, &db)
            .expect("start_quest failed");
        qs.process_event(
            &QuestProgressEvent::MonsterKilled {
                monster_id: 6,
                count: 1,
            },
            &mut gs,
            &db,
        );

        // 100 XP * 0.5 rate = 50
        assert_eq!(
            gs.party.members[0].experience, 50,
            "experience_rate = 0.5 must halve quest XP (100 → 50)"
        );
    }

    #[test]
    fn test_quest_experience_reward_default_rate_unchanged() {
        // Default experience_rate is 1.0 — quest XP should pass through unchanged.
        let mut db = ContentDatabase::new();
        let mut q = Quest::new(102, "Normal XP Quest", "Awards standard XP");
        let mut stage = QuestStage::new(1, "Do task");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 7,
            quantity: 1,
        });
        q.add_stage(stage);
        q.add_reward(QuestReward::Experience(75));
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

        // Default rate = 1.0 (no explicit assignment needed)
        assert_eq!(gs.campaign_config.experience_rate, 1.0);

        let mut qs = QuestSystem::new();
        qs.start_quest(102, &mut gs, &db)
            .expect("start_quest failed");
        qs.process_event(
            &QuestProgressEvent::MonsterKilled {
                monster_id: 7,
                count: 1,
            },
            &mut gs,
            &db,
        );

        assert_eq!(
            gs.party.members[0].experience, 75,
            "default experience_rate = 1.0 must leave quest XP unchanged"
        );
    }
}
