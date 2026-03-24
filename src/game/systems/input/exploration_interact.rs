// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Exploration interaction helpers.
//!
//! This module owns the exploration `Interact` flow that was previously embedded
//! directly in the monolithic input system. The helpers here preserve the
//! existing interaction order while isolating furniture doors, locked map
//! events, adjacent NPC / recruitable routing, and adjacent world-event checks
//! behind a single entry point.
//!
//! The intended top-level entry point is [`handle_exploration_interact`], which
//! returns `true` when the interaction was handled and the calling system should
//! consume the frame.

use crate::application::dialogue::RecruitmentContext;
use crate::application::resources::GameContent;
use crate::application::GameState;
use crate::domain::types::{ItemId, Position};
use crate::domain::world::{MapEvent, WallType};
use crate::game::components::furniture::DoorState;
use crate::game::components::FurnitureEntity;
use crate::game::resources::LockInteractionPending;
use crate::game::systems::dialogue::PendingRecruitmentContext;
use crate::game::systems::events::MapEventTriggered;
use crate::game::systems::input::get_adjacent_positions;
use crate::game::systems::map::{NpcMarker, TileCoord};
use crate::game::systems::ui::GameLog;
use bevy::prelude::*;

/// Handles exploration interaction for the current frame.
///
/// This function preserves the existing interaction ordering:
///
/// 1. furniture doors
/// 2. tile-based locked doors
/// 3. tile-based locked containers
/// 4. plain tile-based door fallback
/// 5. adjacent NPC dialogue
/// 6. current-tile encounter fallback
/// 7. current-tile container fallback
/// 8. adjacent interaction-driven events
///
/// # Arguments
///
/// * `game_state` - Mutable game state
/// * `map_event_messages` - Message writer for map events triggered by interaction
/// * `recruitment_context` - Pending recruitment context updated for recruitables
/// * `npc_query` - Query used to locate NPC entities by tile
/// * `game_content` - Optional content database for key names and class abilities
/// * `door_entity_query` - Query for furniture door entities
/// * `game_log` - Player-visible game log
/// * `lock_pending` - Pending lock-interaction UI resource
///
/// # Returns
///
/// Returns `true` when the interaction was handled and the caller should consume
/// the frame. Returns `false` when nothing interactable was found.
#[allow(clippy::too_many_arguments)]
pub fn handle_exploration_interact(
    game_state: &mut GameState,
    map_event_messages: &mut MessageWriter<MapEventTriggered>,
    recruitment_context: &mut PendingRecruitmentContext,
    npc_query: &Query<(Entity, &NpcMarker, &TileCoord)>,
    game_content: Option<&GameContent>,
    door_entity_query: &mut Query<(
        &mut FurnitureEntity,
        &mut DoorState,
        &mut Transform,
        &TileCoord,
    )>,
    game_log: &mut GameLog,
    lock_pending: &mut LockInteractionPending,
) -> bool {
    let party_position = game_state.world.party_position;
    let adjacent_tiles = get_adjacent_positions(party_position);
    let target = game_state.world.position_ahead();

    if try_interact_furniture_door(game_state, target, door_entity_query, game_log) {
        return true;
    }

    if try_interact_locked_door_event(game_state, target, game_content, game_log, lock_pending) {
        return true;
    }

    if try_interact_locked_container_event(
        game_state,
        target,
        map_event_messages,
        game_content,
        game_log,
        lock_pending,
    ) {
        return true;
    }

    if try_interact_plain_tile_door(game_state, target) {
        return true;
    }

    if try_interact_npc_or_recruitable(
        game_state,
        party_position,
        adjacent_tiles,
        map_event_messages,
        recruitment_context,
        npc_query,
    ) {
        return true;
    }

    if try_interact_adjacent_world_events(
        game_state,
        party_position,
        adjacent_tiles,
        map_event_messages,
    ) {
        return true;
    }

    info!("No interactable object nearby");
    false
}

/// Tries to interact with a furniture door directly ahead of the party.
///
/// Returns `true` when a furniture door occupied the target tile and the
/// interaction was consumed, regardless of whether the door opened
/// successfully.
pub fn try_interact_furniture_door(
    game_state: &mut GameState,
    target: Position,
    door_entity_query: &mut Query<(
        &mut FurnitureEntity,
        &mut DoorState,
        &mut Transform,
        &TileCoord,
    )>,
    game_log: &mut GameLog,
) -> bool {
    for (mut furniture_entity, mut door_state, mut door_transform, tile_coord) in
        door_entity_query.iter_mut()
    {
        if tile_coord.0 != target {
            continue;
        }

        if door_state.is_locked {
            let can_unlock = door_state.key_item_id.is_some_and(|key_id| {
                game_state.party.members.iter().any(|member| {
                    member
                        .inventory
                        .items
                        .iter()
                        .any(|slot| slot.item_id == key_id)
                })
            });

            if can_unlock {
                door_state.is_locked = false;
                door_state.is_open = true;
                furniture_entity.blocking = false;
                door_transform.rotation =
                    Quat::from_rotation_y(door_state.base_rotation_y + std::f32::consts::FRAC_PI_2);
                if let Some(map) = game_state.world.get_current_map_mut() {
                    if let Some(tile) = map.get_tile_mut(tile_coord.0) {
                        tile.blocked = false;
                    }
                }
                info!("Unlocked and opened furniture door at {:?}", target);
            } else {
                let msg = "The door is locked.".to_string();
                info!("{}", msg);
                game_log.add(msg);
            }
        } else {
            door_state.is_open = !door_state.is_open;
            furniture_entity.blocking = !door_state.is_open;

            let angle = if door_state.is_open {
                door_state.base_rotation_y + std::f32::consts::FRAC_PI_2
            } else {
                door_state.base_rotation_y
            };
            door_transform.rotation = Quat::from_rotation_y(angle);

            if let Some(map) = game_state.world.get_current_map_mut() {
                if let Some(tile) = map.get_tile_mut(tile_coord.0) {
                    tile.blocked = !door_state.is_open;
                }
            }

            info!(
                "{} furniture door at {:?}",
                if door_state.is_open {
                    "Opened"
                } else {
                    "Closed"
                },
                target
            );
        }

        return true;
    }

    false
}

/// Tries to interact with a tile-based locked door event directly ahead.
///
/// Returns `true` when a locked-door event was found and the interaction was
/// consumed.
pub fn try_interact_locked_door_event(
    game_state: &mut GameState,
    target: Position,
    game_content: Option<&GameContent>,
    game_log: &mut GameLog,
    lock_pending: &mut LockInteractionPending,
) -> bool {
    let locked_door_info: Option<(String, Option<ItemId>)> = game_state
        .world
        .get_current_map()
        .and_then(|m| m.get_event(target))
        .and_then(|e| {
            if let MapEvent::LockedDoor {
                lock_id,
                key_item_id,
                ..
            } = e
            {
                Some((lock_id.clone(), *key_item_id))
            } else {
                None
            }
        });

    let Some((lock_id, key_item_id)) = locked_door_info else {
        return false;
    };

    let is_locked: bool = game_state
        .world
        .get_current_map()
        .and_then(|m| m.lock_states.get(&lock_id))
        .map(|ls| ls.is_locked)
        .unwrap_or_else(|| {
            warn!(
                "LockedDoor lock_id '{}' has no lock_state entry; was init_lock_states() called on map load?",
                lock_id
            );
            true
        });

    if !is_locked {
        if let Some(map) = game_state.world.get_current_map_mut() {
            if let Some(tile) = map.get_tile_mut(target) {
                tile.wall_type = WallType::None;
                tile.blocked = false;
            }
        }
        info!("Previously unlocked door at {:?} opened", target);
        return true;
    }

    let key_found: Option<(usize, usize)> = key_item_id.and_then(|kid| {
        game_state
            .party
            .members
            .iter()
            .enumerate()
            .find_map(|(char_idx, ch)| {
                ch.inventory
                    .items
                    .iter()
                    .position(|slot| slot.item_id == kid)
                    .map(|slot_idx| (char_idx, slot_idx))
            })
    });

    match (key_item_id, key_found) {
        (Some(kid), Some((char_idx, slot_idx))) => {
            game_state.party.members[char_idx]
                .inventory
                .items
                .remove(slot_idx);

            if let Some(map) = game_state.world.get_current_map_mut() {
                if let Some(ls) = map.lock_states.get_mut(&lock_id) {
                    ls.unlock();
                }
                if let Some(tile) = map.get_tile_mut(target) {
                    tile.wall_type = WallType::None;
                    tile.blocked = false;
                }
                map.remove_event(target);
            }

            let key_name = game_content
                .and_then(|gc| gc.db().items.get_item(kid))
                .map(|item| item.name.clone())
                .unwrap_or_else(|| format!("key {}", kid));
            let msg = format!("You unlock the door with the {}.", key_name);
            info!("{}", msg);
            game_log.add(msg);
        }
        (Some(_), None) => {
            let msg = "The door is locked. You need a key.".to_string();
            info!("{}", msg);
            game_log.add(msg);
            populate_lock_pending(game_state, game_content, lock_pending, lock_id, target);
        }
        (None, _) => {
            let msg = "The door is locked.".to_string();
            info!("{}", msg);
            game_log.add(msg);
            populate_lock_pending(game_state, game_content, lock_pending, lock_id, target);
        }
    }

    true
}

/// Tries to interact with a tile-based locked container event directly ahead.
///
/// Returns `true` when a locked-container event was found and the interaction
/// was consumed.
pub fn try_interact_locked_container_event(
    game_state: &mut GameState,
    target: Position,
    map_event_messages: &mut MessageWriter<MapEventTriggered>,
    game_content: Option<&GameContent>,
    game_log: &mut GameLog,
    lock_pending: &mut LockInteractionPending,
) -> bool {
    let locked_container_info: Option<(String, String, Option<ItemId>)> = game_state
        .world
        .get_current_map()
        .and_then(|m| m.get_event(target))
        .and_then(|e| {
            if let MapEvent::LockedContainer {
                lock_id,
                name,
                key_item_id,
                ..
            } = e
            {
                Some((lock_id.clone(), name.clone(), *key_item_id))
            } else {
                None
            }
        });

    let Some((lock_id, container_name, key_item_id)) = locked_container_info else {
        return false;
    };

    let is_locked: bool = game_state
        .world
        .get_current_map()
        .and_then(|m| m.lock_states.get(&lock_id))
        .map(|ls| ls.is_locked)
        .unwrap_or_else(|| {
            warn!(
                "LockedContainer lock_id '{}' has no lock_state entry; was init_lock_states() called on map load?",
                lock_id
            );
            true
        });

    if !is_locked {
        let id = lock_id.clone();
        let name = container_name.clone();
        if let Some(map) = game_state.world.get_current_map_mut() {
            map.add_event(
                target,
                MapEvent::Container {
                    id: id.clone(),
                    name: name.clone(),
                    description: String::new(),
                    items: vec![],
                },
            );
        }
        map_event_messages.write(MapEventTriggered {
            event: MapEvent::Container {
                id,
                name,
                description: String::new(),
                items: vec![],
            },
            position: target,
        });
        info!("Opening previously unlocked container at {:?}", target);
        return true;
    }

    let key_found: Option<(usize, usize)> = key_item_id.and_then(|kid| {
        game_state
            .party
            .members
            .iter()
            .enumerate()
            .find_map(|(char_idx, ch)| {
                ch.inventory
                    .items
                    .iter()
                    .position(|slot| slot.item_id == kid)
                    .map(|slot_idx| (char_idx, slot_idx))
            })
    });

    match (key_item_id, key_found) {
        (Some(kid), Some((char_idx, slot_idx))) => {
            game_state.party.members[char_idx]
                .inventory
                .items
                .remove(slot_idx);

            let id = lock_id.clone();
            let name = container_name.clone();

            if let Some(map) = game_state.world.get_current_map_mut() {
                if let Some(ls) = map.lock_states.get_mut(&lock_id) {
                    ls.unlock();
                }
                map.add_event(
                    target,
                    MapEvent::Container {
                        id: id.clone(),
                        name: name.clone(),
                        description: String::new(),
                        items: vec![],
                    },
                );
            }

            map_event_messages.write(MapEventTriggered {
                event: MapEvent::Container {
                    id,
                    name: name.clone(),
                    description: String::new(),
                    items: vec![],
                },
                position: target,
            });

            let key_name = game_content
                .and_then(|gc| gc.db().items.get_item(kid))
                .map(|item| item.name.clone())
                .unwrap_or_else(|| format!("key {}", kid));
            let msg = format!("You unlock the {} with the {}.", container_name, key_name);
            info!("{}", msg);
            game_log.add(msg);
        }
        (Some(_), None) => {
            let msg = "The container is locked. You need a key.".to_string();
            info!("{}", msg);
            game_log.add(msg);
            populate_lock_pending(game_state, game_content, lock_pending, lock_id, target);
        }
        (None, _) => {
            let msg = "The container is locked.".to_string();
            info!("{}", msg);
            game_log.add(msg);
            populate_lock_pending(game_state, game_content, lock_pending, lock_id, target);
        }
    }

    true
}

/// Tries to interact with adjacent NPCs or recruitable characters.
///
/// Returns `true` when an NPC or recruitable interaction was found and routed.
pub fn try_interact_npc_or_recruitable(
    game_state: &mut GameState,
    party_position: Position,
    adjacent_tiles: [Position; 8],
    map_event_messages: &mut MessageWriter<MapEventTriggered>,
    recruitment_context: &mut PendingRecruitmentContext,
    npc_query: &Query<(Entity, &NpcMarker, &TileCoord)>,
) -> bool {
    let Some(map) = game_state.world.get_current_map() else {
        info!("No interactable object nearby");
        return true;
    };

    if let Some(npc) = map
        .npc_placements
        .iter()
        .find(|npc| adjacent_tiles.contains(&npc.position))
    {
        info!(
            "Interacting with NPC '{}' at {:?}",
            npc.npc_id, npc.position
        );
        map_event_messages.write(MapEventTriggered {
            event: MapEvent::NpcDialogue {
                name: npc.npc_id.clone(),
                description: String::new(),
                npc_id: npc.npc_id.clone(),
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
            },
            position: npc.position,
        });
        return true;
    }

    if let Some(MapEvent::RecruitableCharacter {
        name, character_id, ..
    }) = map.get_event(party_position)
    {
        info!(
            "Interacting with recruitable character '{}' (ID: {}) at current position {:?}",
            name, character_id, party_position
        );
        recruitment_context.0 = Some(RecruitmentContext {
            character_id: character_id.clone(),
            event_position: party_position,
        });

        map_event_messages.write(MapEventTriggered {
            event: MapEvent::NpcDialogue {
                name: name.clone(),
                description: String::new(),
                npc_id: character_id.clone(),
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
            },
            position: party_position,
        });
        return true;
    }

    for position in adjacent_tiles {
        if let Some(MapEvent::RecruitableCharacter {
            name, character_id, ..
        }) = map.get_event(position)
        {
            info!(
                "Interacting with recruitable character '{}' (ID: {}) at {:?}",
                name, character_id, position
            );

            let _speaker_entity = npc_query
                .iter()
                .find(|(_, _, tile_coord)| tile_coord.0 == position)
                .map(|(entity, _, _)| entity);

            recruitment_context.0 = Some(RecruitmentContext {
                character_id: character_id.clone(),
                event_position: position,
            });

            map_event_messages.write(MapEventTriggered {
                event: MapEvent::NpcDialogue {
                    name: name.clone(),
                    description: String::new(),
                    npc_id: character_id.clone(),
                    time_condition: None,
                    facing: None,
                    proximity_facing: false,
                    rotation_speed: None,
                },
                position,
            });
            return true;
        }
    }

    false
}

/// Tries to interact with adjacent or current-tile world events.
///
/// Returns `true` when an interaction-driven world event was found and routed.
pub fn try_interact_adjacent_world_events(
    game_state: &mut GameState,
    party_position: Position,
    adjacent_tiles: [Position; 8],
    map_event_messages: &mut MessageWriter<MapEventTriggered>,
) -> bool {
    let Some(map) = game_state.world.get_current_map() else {
        info!("No interactable object nearby");
        return true;
    };

    if let Some(event) = map.get_event(party_position) {
        if let MapEvent::Encounter { .. } = event {
            info!(
                "Interacting with encounter at current position {:?}",
                party_position
            );
            map_event_messages.write(MapEventTriggered {
                event: event.clone(),
                position: party_position,
            });
            return true;
        }
    }

    if let Some(event) = map.get_event(party_position) {
        if let MapEvent::Container { id, name, .. } = event {
            info!(
                "Interacting with container '{}' ({}) at current position {:?}",
                id, name, party_position
            );
            map_event_messages.write(MapEventTriggered {
                event: event.clone(),
                position: party_position,
            });
            return true;
        }
    }

    for position in adjacent_tiles {
        if let Some(event) = map.get_event(position) {
            match event {
                MapEvent::Sign { .. }
                | MapEvent::Teleport { .. }
                | MapEvent::Encounter { .. }
                | MapEvent::Container { .. } => {
                    info!("Interacting with event at {:?}", position);
                    map_event_messages.write(MapEventTriggered {
                        event: event.clone(),
                        position,
                    });
                    return true;
                }
                _ => continue,
            }
        }
    }

    false
}

/// Tries to open a plain tile-based door directly ahead of the party.
///
/// Returns `true` when a plain `WallType::Door` tile was found and opened.
pub fn try_interact_plain_tile_door(game_state: &mut GameState, target: Position) -> bool {
    let has_plain_door = game_state
        .world
        .get_current_map()
        .and_then(|m| m.get_tile(target))
        .map(|t| t.wall_type == WallType::Door)
        .unwrap_or(false);

    if has_plain_door {
        if let Some(map) = game_state.world.get_current_map_mut() {
            if let Some(tile) = map.get_tile_mut(target) {
                tile.wall_type = WallType::None;
                tile.blocked = false;
            }
        }
        info!("Opened door at {:?}", target);
        return true;
    }

    false
}

/// Populates the pending lock-interaction resource using the current party and
/// optional game content.
fn populate_lock_pending(
    game_state: &GameState,
    game_content: Option<&GameContent>,
    lock_pending: &mut LockInteractionPending,
    lock_id: String,
    position: Position,
) {
    let can_lockpick = game_content
        .map(|gc| {
            game_state.party.members.iter().any(|member| {
                gc.db()
                    .classes
                    .get_class(&member.class_id)
                    .map(|cls| cls.has_ability("pick_lock"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    lock_pending.lock_id = Some(lock_id);
    lock_pending.position = Some(position);
    lock_pending.can_lockpick = can_lockpick;
}
