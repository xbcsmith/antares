// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::domain::world::MapEvent;
use crate::game::resources::GlobalState;
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
    mut global_state: ResMut<GlobalState>,
    mut game_log: ResMut<crate::game::systems::ui::GameLog>,
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
                game_log.add(msg);

                let game_state = &mut global_state.0;

                // TODO: Load the new map if it's different
                // For now, just update position and map ID
                game_state.world.set_current_map(*map_id);
                game_state.world.set_party_position(*destination);

                // Note: Real implementation needs to handle map loading/unloading
            }
            MapEvent::Sign { text, .. } => {
                let msg = format!("Sign reads: {}", text);
                println!("{}", msg);
                game_log.add(msg);
            }
            MapEvent::Trap { damage, effect, .. } => {
                let msg = format!("IT'S A TRAP! Took {} damage. Effect: {:?}", damage, effect);
                println!("{}", msg);
                game_log.add(msg);
                // TODO: Apply damage to party
            }
            MapEvent::Treasure { loot, .. } => {
                let msg = format!("Found treasure! Loot IDs: {:?}", loot);
                println!("{}", msg);
                game_log.add(msg);
                // TODO: Add to inventory
            }
            MapEvent::Encounter { monster_group, .. } => {
                let msg = format!("Monsters attack! Group IDs: {:?}", monster_group);
                println!("{}", msg);
                game_log.add(msg);
                // TODO: Start combat
            }
            MapEvent::NpcDialogue { npc_id, .. } => {
                let msg = format!("NPC {} wants to talk.", npc_id);
                println!("{}", msg);
                game_log.add(msg);
                // TODO: Start dialogue
            }
        }
    }
}
