// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Quest plugin wiring
//!
//! This small module provides an ECS plugin that wires together the
//! application-level `QuestSystem` and the `QuestProgressEvent` message so the
//! quest update system runs automatically within the Bevy schedule.
//!
//! The heavy lifting (progress bookkeeping and reward application) lives in
//! `crate::application::quests`. This plugin simply inserts the resource,
//! registers the message type, and schedules the update system.

use bevy::prelude::*;

/// Plugin that registers quest systems and events.
///
/// Usage:
/// ```no_run
/// use antares::game::systems::quest::QuestPlugin;
/// app.add_plugins(QuestPlugin);
/// ```
pub struct QuestPlugin;

impl Plugin for QuestPlugin {
    fn build(&self, app: &mut App) {
        // Ensure the application-level QuestSystem resource exists and that
        // the QuestProgressEvent message and update system are registered.
        app.insert_resource(crate::application::quests::QuestSystem::new())
            .add_message::<crate::application::quests::QuestProgressEvent>()
            .add_systems(Update, crate::application::quests::update);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::resources::GameContent;
    use crate::domain::quest::{Quest, QuestObjective, QuestReward, QuestStage};
    use crate::game::resources::GlobalState;
    use crate::sdk::database::ContentDatabase;
    use bevy::prelude::*;

    /// Sanity check: plugin registers the QuestSystem resource and the message writer.
    #[test]
    fn test_plugin_registers_resources_and_messages() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(QuestPlugin);

        // QuestSystem resource inserted
        assert!(app
            .world
            .get_resource::<crate::application::quests::QuestSystem>()
            .is_some());

        // MessageWriter resource for QuestProgressEvent should be present
        assert!(app
            .world
            .get_resource::<MessageWriter<crate::application::quests::QuestProgressEvent>>()
            .is_some());
    }

    /// Integration smoke test: a MonsterKilled event processed by the plugin's
    /// wired update system should modify quest progress and (optionally) apply rewards.
    #[test]
    fn test_plugin_processes_quest_event() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(QuestPlugin);

        // Create content DB with a simple quest (kill 1 of monster 5)
        let mut db = ContentDatabase::new();
        let mut q = Quest::new(1, "Simple Kill", "Kill the goblin leader");
        let mut stage = QuestStage::new(1, "Eliminate the leader");
        stage.add_objective(QuestObjective::KillMonsters {
            monster_id: 5,
            quantity: 1,
        });
        q.add_stage(stage);
        q.add_reward(QuestReward::Gold(100));
        db.quests.add_quest(q);

        // Insert required resources: content DB and GlobalState
        app.insert_resource(GameContent::new(db));
        app.insert_resource(GlobalState(crate::application::GameState::new()));

        // Start tracking the quest
        {
            let mut qs = app
                .world
                .resource_mut::<crate::application::quests::QuestSystem>();
            let mut gs = app.world.resource_mut::<GlobalState>();
            let content = app.world.resource::<GameContent>();
            qs.start_quest(1, &mut gs.0, content.db())
                .expect("start quest");
        }

        // Emit a MonsterKilled event
        {
            let mut writer = app
                .world
                .resource_mut::<MessageWriter<crate::application::quests::QuestProgressEvent>>();
            writer.write(
                crate::application::quests::QuestProgressEvent::MonsterKilled {
                    monster_id: 5,
                    count: 1,
                },
            );
        }

        // Run schedule once so the update system processes the event
        app.update();

        // Ensure quest marked completed and reward applied
        let gs = app.world.resource::<GlobalState>();
        assert_eq!(gs.0.party.gold, 100);
    }
}
