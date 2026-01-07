// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::application::resources::GameContent;
use crate::domain::world::MapEvent;
use crate::game::resources::GlobalState;
use crate::game::systems::dialogue::StartDialogue;
use crate::game::systems::map::MapChangeEvent;
use bevy::prelude::*;

pub struct EventPlugin;

impl Plugin for EventPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MapEventTriggered>()
            .add_systems(Update, (check_for_events, handle_events));
    }
}

/// Event triggered when the party steps on a tile with an event
#[derive(Message)]
pub struct MapEventTriggered {
    pub event: MapEvent,
}

/// System to check if the party is standing on an event
fn check_for_events(
    global_state: Res<GlobalState>,
    mut event_writer: MessageWriter<MapEventTriggered>,
    mut last_position: Local<Option<crate::domain::types::Position>>,
) {
    let game_state = &global_state.0;
    let current_pos = game_state.world.party_position;

    // Only check if position changed
    if *last_position != Some(current_pos) {
        *last_position = Some(current_pos);

        if let Some(map) = game_state.world.get_current_map() {
            if let Some(event) = map.get_event(current_pos) {
                // Trigger the event
                event_writer.write(MapEventTriggered {
                    event: event.clone(),
                });
            }
        }
    }
}

/// System to handle triggered events
fn handle_events(
    mut event_reader: MessageReader<MapEventTriggered>,
    mut map_change_writer: MessageWriter<MapChangeEvent>,
    mut dialogue_writer: MessageWriter<StartDialogue>,
    content: Res<GameContent>,
    mut game_log: Option<ResMut<crate::game::systems::ui::GameLog>>,
    mut global_state: ResMut<GlobalState>,
) {
    for trigger in event_reader.read() {
        match &trigger.event {
            MapEvent::Teleport {
                destination,
                map_id,
                ..
            } => {
                let msg = format!("Teleporting to Map {} at {:?}", map_id, destination);
                println!("{}", msg);
                if let Some(ref mut log) = game_log {
                    log.add(msg);
                }

                // Emit a MapChangeEvent so the MapManagerPlugin can handle the
                // full lifecycle (despawn old tiles, spawn new ones, set position).
                map_change_writer.write(MapChangeEvent {
                    target_map: *map_id,
                    target_pos: *destination,
                });
            }
            MapEvent::Sign { text, .. } => {
                let msg = format!("Sign reads: {}", text);
                println!("{}", msg);
                if let Some(ref mut log) = game_log {
                    log.add(msg);
                }
            }
            MapEvent::Trap { damage, effect, .. } => {
                let msg = format!("IT'S A TRAP! Took {} damage. Effect: {:?}", damage, effect);
                println!("{}", msg);
                if let Some(ref mut log) = game_log {
                    log.add(msg);
                }
                // TODO: Apply damage to party
            }
            MapEvent::Treasure { loot, .. } => {
                let msg = format!("Found treasure! Loot IDs: {:?}", loot);
                println!("{}", msg);
                if let Some(ref mut log) = game_log {
                    log.add(msg);
                }
                // TODO: Add to inventory
            }
            MapEvent::Encounter { monster_group, .. } => {
                let msg = format!("Monsters attack! Group IDs: {:?}", monster_group);
                println!("{}", msg);
                if let Some(ref mut log) = game_log {
                    log.add(msg);
                }
                // TODO: Start combat
            }
            MapEvent::NpcDialogue { npc_id, .. } => {
                // Look up NPC in database
                if let Some(npc_def) = content.db().npcs.get_npc(npc_id) {
                    // Check if NPC has a dialogue tree
                    if let Some(dialogue_id) = npc_def.dialogue_id {
                        // Send StartDialogue message to trigger dialogue system
                        dialogue_writer.write(StartDialogue { dialogue_id });

                        let msg = format!("{} wants to talk.", npc_def.name);
                        println!("{}", msg);
                        if let Some(ref mut log) = game_log {
                            log.add(msg);
                        }
                    } else {
                        // Fallback: No dialogue tree, log to game log
                        let msg =
                            format!("{}: Hello, traveler! (No dialogue available)", npc_def.name);
                        println!("{}", msg);
                        if let Some(ref mut log) = game_log {
                            log.add(msg);
                        }
                    }
                } else {
                    // NPC not found in database - log error
                    let msg = format!("Error: NPC '{}' not found in database", npc_id);
                    println!("{}", msg);
                    if let Some(ref mut log) = game_log {
                        log.add(msg);
                    }
                }
            }
            MapEvent::RecruitableCharacter {
                character_id,
                name,
                description,
                ..
            } => {
                let msg = format!("{} - {}", name, description);
                println!("{}", msg);
                if let Some(ref mut log) = game_log {
                    log.add(msg);
                }
                // Recruitment dialog will be triggered by a separate system
                // that listens for recruitment events and shows the UI
                // For now, just log that the character was encountered
                let _ = character_id; // TODO: Trigger recruitment dialog
            }
            MapEvent::EnterInn {
                name,
                description,
                innkeeper_id,
            } => {
                let msg = format!("{} - {}", name, description);
                println!("{}", msg);
                if let Some(ref mut log) = game_log {
                    log.add(msg);
                }

                // Transition GameMode to InnManagement
                use crate::application::{GameMode, InnManagementState};
                global_state.0.mode = GameMode::InnManagement(InnManagementState {
                    current_inn_id: innkeeper_id.clone(),
                    selected_party_slot: None,
                    selected_roster_slot: None,
                });

                let inn_msg = format!("Entering inn (ID: {})", innkeeper_id);
                println!("{}", inn_msg);
                if let Some(ref mut log) = game_log {
                    log.add(inn_msg);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameState;
    use crate::domain::types::Position;
    use crate::domain::world::{Map, MapEvent};

    #[test]
    fn test_event_triggered_when_party_moves_to_event_position() {
        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        map.add_event(
            event_pos,
            MapEvent::Sign {
                name: "Test".to_string(),
                description: "Test sign".to_string(),
                text: "You found it!".to_string(),
            },
        );

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(
            crate::sdk::database::ContentDatabase::new(),
        ));

        // Act
        app.update();

        // Assert
        let events = app.world().resource::<Messages<MapEventTriggered>>();
        let mut reader = events.get_cursor();
        let triggered_events: Vec<_> = reader.read(events).collect();
        assert!(
            !triggered_events.is_empty(),
            "Expected at least one event to be triggered"
        );
    }

    #[test]
    fn test_no_event_triggered_when_no_event_at_position() {
        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_plugins(EventPlugin);

        let map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        // No events added to map

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(Position::new(5, 5));

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(
            crate::sdk::database::ContentDatabase::new(),
        ));

        // Act
        app.update();

        // Assert
        let events = app.world().resource::<Messages<MapEventTriggered>>();
        let mut reader = events.get_cursor();
        let triggered_events: Vec<_> = reader.read(events).collect();
        assert!(
            triggered_events.is_empty(),
            "Expected no events to be triggered"
        );
    }

    #[test]
    fn test_event_only_triggers_once_per_position() {
        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        map.add_event(
            event_pos,
            MapEvent::Sign {
                name: "Test".to_string(),
                description: "Test sign".to_string(),
                text: "You found it!".to_string(),
            },
        );

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(
            crate::sdk::database::ContentDatabase::new(),
        ));

        // Act - update multiple times at same position
        app.update();
        let events = app.world().resource::<Messages<MapEventTriggered>>();
        let mut reader = events.get_cursor();
        let first_update_count = reader.read(events).count();

        app.update();
        let events = app.world().resource::<Messages<MapEventTriggered>>();
        let second_update_count = reader.read(events).count();

        // Assert - only first update should trigger event
        assert_eq!(
            first_update_count, 1,
            "Expected exactly one event on first update"
        );
        assert_eq!(
            second_update_count, 0,
            "Expected no events on second update (same position)"
        );
    }

    #[test]
    fn test_npc_dialogue_event_triggers_dialogue_when_npc_has_dialogue_id() {
        use crate::domain::world::NpcDefinition;

        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        map.add_event(
            event_pos,
            MapEvent::NpcDialogue {
                name: "Elder".to_string(),
                description: "Village Elder".to_string(),
                npc_id: "test_elder".to_string(),
            },
        );

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);

        // Add NPC to database with dialogue_id
        let mut db = crate::sdk::database::ContentDatabase::new();
        let npc = NpcDefinition {
            id: "test_elder".to_string(),
            name: "Village Elder".to_string(),
            description: "Wise elder".to_string(),
            portrait_id: "portraits/elder.png".to_string(),
            dialogue_id: Some(1u16),
            quest_ids: vec![],
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        };
        db.npcs.add_npc(npc).unwrap();

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));

        // Act
        app.update(); // First update: check_for_events writes MapEventTriggered
        app.update(); // Second update: handle_events processes MapEventTriggered

        // Assert - StartDialogue message should be sent
        let dialogue_messages = app.world().resource::<Messages<StartDialogue>>();
        let mut reader = dialogue_messages.get_cursor();
        let messages: Vec<_> = reader.read(dialogue_messages).collect();
        assert_eq!(messages.len(), 1, "Expected StartDialogue message");
        assert_eq!(messages[0].dialogue_id, 1u16);
    }

    #[test]
    fn test_npc_dialogue_event_logs_when_npc_has_no_dialogue_id() {
        use crate::domain::world::NpcDefinition;
        use crate::game::systems::ui::GameLog;

        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        map.add_event(
            event_pos,
            MapEvent::NpcDialogue {
                name: "Merchant".to_string(),
                description: "Town Merchant".to_string(),
                npc_id: "test_merchant".to_string(),
            },
        );

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);

        // Add NPC to database without dialogue_id
        let mut db = crate::sdk::database::ContentDatabase::new();
        let npc = NpcDefinition {
            id: "test_merchant".to_string(),
            name: "Town Merchant".to_string(),
            description: "Sells goods".to_string(),
            portrait_id: "portraits/merchant.png".to_string(),
            dialogue_id: None,
            quest_ids: vec![],
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
        };
        db.npcs.add_npc(npc).unwrap();

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));
        app.insert_resource(GameLog::new());

        // Act
        app.update(); // First update: check_for_events writes MapEventTriggered
        app.update(); // Second update: handle_events processes MapEventTriggered

        // Assert - GameLog should contain fallback message
        let game_log = app.world().resource::<GameLog>();
        let entries = game_log.entries();
        assert!(
            entries
                .iter()
                .any(|e| e.contains("Town Merchant") && e.contains("No dialogue available")),
            "Expected fallback message in game log. Actual entries: {:?}",
            entries
        );
    }

    #[test]
    fn test_npc_dialogue_event_logs_error_when_npc_not_found() {
        use crate::game::systems::ui::GameLog;

        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        map.add_event(
            event_pos,
            MapEvent::NpcDialogue {
                name: "Unknown".to_string(),
                description: "Unknown NPC".to_string(),
                npc_id: "nonexistent_npc".to_string(),
            },
        );

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);

        let db = crate::sdk::database::ContentDatabase::new();

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));
        app.insert_resource(GameLog::new());

        // Act
        app.update(); // First update: check_for_events writes MapEventTriggered
        app.update(); // Second update: handle_events processes MapEventTriggered

        // Assert - GameLog should contain error message
        let game_log = app.world().resource::<GameLog>();
        let entries = game_log.entries();
        assert!(
            entries
                .iter()
                .any(|e| e.contains("Error") && e.contains("nonexistent_npc")),
            "Expected error message in game log. Actual entries: {:?}",
            entries
        );
    }

    #[test]
    fn test_enter_inn_event_transitions_to_inn_management_mode() {
        use crate::application::GameMode;
        use crate::game::systems::ui::GameLog;

        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        map.add_event(
            event_pos,
            MapEvent::EnterInn {
                name: "Cozy Inn Entrance".to_string(),
                description: "A welcoming inn".to_string(),
                innkeeper_id: "cozy_inn".to_string(),
            },
        );

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);
        // Start in Exploration mode
        game_state.mode = GameMode::Exploration;

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(
            crate::sdk::database::ContentDatabase::new(),
        ));
        app.insert_resource(GameLog::new());

        // Act
        app.update(); // First update: check_for_events writes MapEventTriggered
        app.update(); // Second update: handle_events processes MapEventTriggered and transitions mode

        // Assert - GameMode should be InnManagement with correct inn_id
        let global_state = app.world().resource::<GlobalState>();
        match &global_state.0.mode {
            GameMode::InnManagement(state) => {
                assert_eq!(
                    state.current_inn_id,
                    "cozy_inn".to_string(),
                    "Expected innkeeper id to be 'cozy_inn'"
                );
                assert_eq!(
                    state.selected_party_slot, None,
                    "Expected no selected party slot initially"
                );
                assert_eq!(
                    state.selected_roster_slot, None,
                    "Expected no selected roster slot initially"
                );
            }
            other_mode => panic!("Expected GameMode::InnManagement, but got {:?}", other_mode),
        }

        // Assert - GameLog should contain inn entry message
        let game_log = app.world().resource::<GameLog>();
        let entries = game_log.entries();
        assert!(
            entries
                .iter()
                .any(|e| e.contains("Entering inn (ID: cozy_inn)")),
            "Expected inn entry message in game log. Actual entries: {:?}",
            entries
        );
    }

    #[test]
    fn test_enter_inn_event_with_different_inn_ids() {
        use crate::application::GameMode;

        // Test that different inn_ids are correctly set in InnManagementState
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        let event_pos = Position::new(10, 10);
        map.add_event(
            event_pos,
            MapEvent::EnterInn {
                name: "Dragon's Rest Inn".to_string(),
                description: "An upscale inn".to_string(),
                innkeeper_id: "tutorial_innkeeper_town2".to_string(),
            },
        );

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);
        game_state.mode = GameMode::Exploration;

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(
            crate::sdk::database::ContentDatabase::new(),
        ));

        // Act
        app.update();
        app.update();

        // Assert
        let global_state = app.world().resource::<GlobalState>();
        match &global_state.0.mode {
            GameMode::InnManagement(state) => {
                assert_eq!(
                    state.current_inn_id,
                    "tutorial_innkeeper_town2".to_string(),
                    "Expected innkeeper id to be 'tutorial_innkeeper_town2'"
                );
            }
            other_mode => panic!("Expected GameMode::InnManagement, but got {:?}", other_mode),
        }
    }
}
