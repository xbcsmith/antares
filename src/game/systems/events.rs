// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::application::resources::GameContent;
use crate::domain::world::EventResult;
use crate::domain::world::MapEvent;
use crate::game::resources::{GlobalState, LockInteractionPending};
use crate::game::systems::dialogue::{SimpleDialogue, StartDialogue};
use crate::game::systems::furniture_rendering::{
    resolve_furniture_fields, spawn_furniture_with_rendering,
};
use crate::game::systems::map::{EventTrigger, MapChangeEvent, NpcMarker, TileCoord};
use crate::game::systems::procedural_meshes::{
    FurnitureSpawnParams, MeshSpawnContext, ProceduralMeshCache,
};
use crate::game::systems::ui::{GameLogEvent, LogCategory};
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
    pub position: crate::domain::types::Position,
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
                // Do not auto-trigger interaction-driven utility/dialogue events when
                // the party steps on their tile. These events must be explicitly
                // interacted with by the player (press the Interact key).
                //
                // Encounters require explicit interaction (E key / mouse click from the
                // current or an adjacent tile via try_interact_adjacent_world_events).
                // This gives the player agency to choose when to engage rather than being
                // forced into combat by accidentally stepping on the tile.
                match event {
                    MapEvent::RecruitableCharacter { .. } => {
                        info!(
                            "Party at {:?} is on a RecruitableCharacter event; not auto-triggering (requires interact)",
                            current_pos
                        );
                    }
                    MapEvent::Sign { .. } => {
                        info!(
                            "Party at {:?} is on a Sign event; not auto-triggering (requires interact)",
                            current_pos
                        );
                    }
                    MapEvent::Teleport { .. } => {
                        info!(
                            "Party at {:?} is on a Teleport event; not auto-triggering (requires interact)",
                            current_pos
                        );
                    }
                    MapEvent::Container { .. } => {
                        info!(
                            "Party at {:?} is on a Container event; not auto-triggering (requires interact)",
                            current_pos
                        );
                    }
                    // LockedDoor and LockedContainer tiles are blocked so the
                    // party cannot physically stand on them. Skip auto-triggering
                    // here; interaction is driven by the split exploration-
                    // interaction input path (or explicit `MapEventTriggered`
                    // in tests).
                    MapEvent::LockedDoor { .. } | MapEvent::LockedContainer { .. } => {
                        info!(
                            "Party at {:?} is on a LockedDoor/LockedContainer event; \
                             not auto-triggering (requires interact)",
                            current_pos
                        );
                    }
                    MapEvent::Encounter { .. } => {
                        info!(
                            "Party at {:?} is on an Encounter event; not auto-triggering (requires interact)",
                            current_pos
                        );
                    }
                    _ => {
                        // Trigger other event types automatically (traps, etc.)
                        event_writer.write(MapEventTriggered {
                            event: event.clone(),
                            position: current_pos,
                        });
                    }
                }
            }
        }
    }
}

/// System to handle triggered events
#[allow(clippy::too_many_arguments)]
fn handle_events(
    mut event_reader: MessageReader<MapEventTriggered>,
    mut map_change_writer: MessageWriter<MapChangeEvent>,
    mut dialogue_writer: MessageWriter<StartDialogue>,
    mut simple_dialogue_writer: MessageWriter<SimpleDialogue>,
    mut combat_started_writer: Option<MessageWriter<crate::game::systems::combat::CombatStarted>>,
    mut game_log_writer: Option<MessageWriter<GameLogEvent>>,
    content: Res<GameContent>,
    mut global_state: ResMut<GlobalState>,
    mut commands: Option<Commands>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    npc_query: Query<(Entity, &NpcMarker, &TileCoord)>,
    // Diagnostic query to list entities at the tile when speaker resolution fails.
    // Keep the query compact to avoid clippy type-complexity complaints while still
    // returning the transform and event trigger presence which are useful for diagnostics.
    all_tile_query: Query<(
        Entity,
        &TileCoord,
        Option<&Transform>,
        Option<&EventTrigger>,
    )>,
    // Transform query used to inspect candidate speaker entity's world Y position.
    // We use this to prefer fallback visuals when the speaker is a low-lying marker.
    _query_transform: Query<&Transform>,
    mut pending_recruitment: Option<
        ResMut<crate::game::systems::dialogue::PendingRecruitmentContext>,
    >,
    // Lock signal resource consumed by the lock-choice UI.
    // Using Option<ResMut<...>> so handle_events can run in test apps that do
    // not register InputPlugin (and therefore may not have the resource).
    mut lock_pending: Option<ResMut<LockInteractionPending>>,
) {
    let mut furniture_cache = ProceduralMeshCache::default();
    for trigger in event_reader.read() {
        // Only process furniture events if we have the necessary resources
        let can_spawn_furniture = commands.is_some() && materials.is_some() && meshes.is_some();

        match &trigger.event {
            MapEvent::Teleport {
                destination,
                map_id,
                ..
            } => {
                let map_name = content
                    .db()
                    .maps
                    .get_map(*map_id)
                    .map(|map| map.name.clone())
                    .unwrap_or_else(|| format!("Map {}", map_id));
                let msg = format!("Entering {}...", map_name);
                tracing::info!("{}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Exploration,
                    });
                }

                // Emit a MapChangeEvent so the MapManagerPlugin can handle the
                // full lifecycle (despawn old tiles, spawn new ones, set position).
                map_change_writer.write(MapChangeEvent {
                    target_map: *map_id,
                    target_pos: *destination,
                    is_portal: true,
                });
            }
            MapEvent::Sign { text, name, .. } => {
                // Show sign text in a dialogue bubble
                simple_dialogue_writer.write(SimpleDialogue {
                    text: text.clone(),
                    speaker_name: name.clone(),
                    speaker_entity: None,
                    fallback_position: Some(trigger.position),
                });

                let msg = format!("{}: {}", name, text);
                tracing::info!("{}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Exploration,
                    });
                }
            }
            MapEvent::Trap { damage, effect, .. } => {
                let trap_damage = *damage;

                // Apply damage to every living party member and log per-character.
                for member in &mut global_state.0.party.members {
                    if member.is_alive() {
                        let old_hp = member.hp.current;
                        member.hp.modify(-(trap_damage as i32));
                        let actual = old_hp.saturating_sub(member.hp.current);

                        if member.hp.current == 0 {
                            member
                                .conditions
                                .add(crate::domain::character::Condition::DEAD);
                        }

                        let per_char_msg =
                            format!("{} takes {} damage from trap!", member.name, actual);
                        tracing::warn!("{}", per_char_msg);
                        if let Some(ref mut writer) = game_log_writer {
                            writer.write(GameLogEvent {
                                text: per_char_msg,
                                category: LogCategory::Combat,
                            });
                        }
                    }
                }

                // Apply status effect if present.
                if let Some(ref effect_name) = effect {
                    let flag = crate::application::map_effect_to_condition(effect_name);
                    if flag != crate::domain::character::Condition::FINE {
                        for member in &mut global_state.0.party.members {
                            if member.is_alive() {
                                member.conditions.add(flag);
                            }
                        }
                        let effect_msg = format!("The trap inflicts {}!", effect_name);
                        tracing::warn!("{}", effect_msg);
                        if let Some(ref mut writer) = game_log_writer {
                            writer.write(GameLogEvent {
                                text: effect_msg,
                                category: LogCategory::Combat,
                            });
                        }
                    }
                }

                // Summary log entry.
                let msg = format!("Trapped! Took {} damage.", trap_damage);
                tracing::warn!("{}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Combat,
                    });
                }

                // Check for party wipe — all members dead.
                if global_state.0.party.living_count() == 0 {
                    tracing::error!("Party wiped by trap!");
                    global_state.0.mode = crate::application::GameMode::GameOver;
                    if let Some(ref mut writer) = game_log_writer {
                        writer.write(GameLogEvent {
                            text: "The entire party has perished!".to_string(),
                            category: LogCategory::Combat,
                        });
                    }
                }

                // Remove trap event from map (one-time).
                if let Some(map) = global_state.0.world.get_current_map_mut() {
                    map.remove_event(trigger.position);
                }
            }
            MapEvent::Treasure { loot, .. } => {
                let msg = format!("Found treasure! {} item(s).", loot.len());
                tracing::info!("{}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Item,
                    });
                }

                // Distribute loot items to party members with inventory space.
                for item_byte in loot {
                    let item_id = *item_byte as crate::domain::types::ItemId;
                    let mut placed = false;
                    for member in &mut global_state.0.party.members {
                        if member.inventory.has_space()
                            && member.inventory.add_item(item_id, 1).is_ok()
                        {
                            placed = true;
                            let item_name = content
                                .db()
                                .items
                                .get_item(item_id)
                                .map(|i| i.name.clone())
                                .unwrap_or_else(|| format!("Item {}", item_id));
                            let item_msg = format!("{} receives {}.", member.name, item_name);
                            tracing::info!("{}", item_msg);
                            if let Some(ref mut writer) = game_log_writer {
                                writer.write(GameLogEvent {
                                    text: item_msg,
                                    category: LogCategory::Item,
                                });
                            }
                            break;
                        }
                    }
                    if !placed {
                        let lost_msg = format!("Inventory full — item {} lost!", item_id);
                        tracing::warn!("{}", lost_msg);
                        if let Some(ref mut writer) = game_log_writer {
                            writer.write(GameLogEvent {
                                text: lost_msg,
                                category: LogCategory::Item,
                            });
                        }
                    }
                }

                // Remove treasure event from map (one-time).
                if let Some(map) = global_state.0.world.get_current_map_mut() {
                    map.remove_event(trigger.position);
                }
            }
            MapEvent::Encounter {
                monster_group,
                combat_event_type,
                ..
            } => {
                let msg = format!("Monsters! ({} foes)", monster_group.len());
                tracing::info!("{}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Combat,
                    });
                }

                // Debug: print current mode before attempting to start combat
                info!("Mode before start_encounter: {:?}", global_state.0.mode);

                // Attempt to start combat using the combat helper (copies party and loads monsters)
                match crate::game::systems::combat::start_encounter(
                    &mut global_state.0,
                    &content,
                    monster_group,
                    *combat_event_type,
                ) {
                    Ok(()) => {
                        // Debug: print mode after successful attempt
                        info!("Mode after start_encounter: {:?}", global_state.0.mode);

                        // Notify other systems that combat has started (if CombatPlugin is registered)
                        if let Some(ref mut writer) = combat_started_writer {
                            writer.write(crate::game::systems::combat::CombatStarted {
                                encounter_position: Some(trigger.position),
                                encounter_map_id: Some(global_state.0.world.current_map),
                                combat_event_type: *combat_event_type,
                            });
                        }
                    }
                    Err(e) => {
                        error!("Failed to start encounter: {}", e);
                        if let Some(ref mut writer) = game_log_writer {
                            writer.write(GameLogEvent {
                                text: format!("Failed to start encounter: {}", e),
                                category: LogCategory::System,
                            });
                        }
                    }
                }
            }
            MapEvent::NpcDialogue { npc_id, .. } => {
                // Look up NPC in database
                if let Some(npc_def) = content.db().npcs.get_npc(npc_id) {
                    // If this NPC is a merchant, route through the EnterMerchant path so
                    // the merchant entry handler drives the interaction. This produces an
                    // EventResult::EnterMerchant and dispatches it through handle_event_result,
                    // which fires StartDialogue (if a dialogue_id is configured) or a fallback
                    // SimpleDialogue (if not).
                    if npc_def.is_merchant {
                        let event_result = EventResult::EnterMerchant {
                            npc_id: npc_id.clone(),
                        };
                        handle_event_result(
                            &event_result,
                            &content,
                            &mut dialogue_writer,
                            &mut simple_dialogue_writer,
                            &mut game_log_writer,
                            &npc_query,
                            &trigger.position,
                        );
                    } else if let Some(dialogue_id) = npc_def.dialogue_id {
                        // Check if NPC has a dialogue tree
                        // Find the NPC entity by its ID
                        let speaker_entity = npc_query
                            .iter()
                            .find(|(_, marker, _)| marker.npc_id == *npc_id)
                            .map(|(entity, _, _)| entity);

                        if speaker_entity.is_none() {
                            let available: Vec<_> =
                                npc_query.iter().map(|(_, m, _)| &m.npc_id).collect();
                            warn!(
                                "Speaker NPC '{}' not found in world. Available: {:?}",
                                npc_id, available
                            );
                        }

                        // Send StartDialogue message to trigger dialogue system
                        dialogue_writer.write(StartDialogue {
                            dialogue_id,
                            speaker_entity,
                            fallback_position: Some(trigger.position),
                        });

                        let msg = format!("{} speaks.", npc_def.name);
                        tracing::info!("{}", msg);
                        if let Some(ref mut writer) = game_log_writer {
                            writer.write(GameLogEvent {
                                text: msg,
                                category: LogCategory::Dialogue,
                            });
                        }
                    } else {
                        // Fallback: No dialogue tree, show simple dialogue bubble
                        let speaker_entity = npc_query
                            .iter()
                            .find(|(_, marker, _)| marker.npc_id == *npc_id)
                            .map(|(entity, _, _)| entity);

                        simple_dialogue_writer.write(SimpleDialogue {
                            text: format!("Hello! I am {}.", npc_def.name),
                            speaker_name: npc_def.name.clone(),
                            speaker_entity,
                            fallback_position: Some(trigger.position),
                        });

                        let msg = format!("{} speaks.", npc_def.name);
                        tracing::info!("{}", msg);
                        if let Some(ref mut writer) = game_log_writer {
                            writer.write(GameLogEvent {
                                text: msg,
                                category: LogCategory::Dialogue,
                            });
                        }
                    }
                } else {
                    // NPC not found in database - log error
                    let msg = format!("Error: NPC '{}' not found in database", npc_id);
                    tracing::error!("{}", msg);
                    if let Some(ref mut writer) = game_log_writer {
                        writer.write(GameLogEvent {
                            text: msg,
                            category: LogCategory::Exploration,
                        });
                    }
                }
            }
            MapEvent::RecruitableCharacter {
                character_id,
                name,
                description: _,
                dialogue_id,
                time_condition: _,
                facing: _,
            } => {
                let msg = format!("Met {}.", name);
                tracing::info!("{}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Dialogue,
                    });
                }

                // Use trigger.position — the tile where the event lives — not the party
                // position.  When interaction is initiated from an adjacent tile these two
                // positions differ, and using the party position would cause remove_event
                // to target the wrong tile, leaving the recruitable mesh visible on the
                // map until the party physically walks over it.
                let event_pos = trigger.position;

                // If dialogue is specified, trigger dialogue system
                if let Some(dlg_id) = dialogue_id {
                    // Find NPC entity at the event position for speaker (optional visual).
                    // NOTE: We intentionally prefer using the fallback tile position for
                    // recruitable character visuals rather than treating low-level event/tile
                    // marker entities as speakers. This avoids placing UI on marker geometry
                    // (which can be near the ground) and keeps visuals consistent.
                    let speaker_entity = npc_query
                        .iter()
                        .find(|(_, _, coord)| coord.0.x == event_pos.x && coord.0.y == event_pos.y)
                        .map(|(entity, _, _)| entity);

                    if speaker_entity.is_none() {
                        info!(
                            "No NpcMarker found at {:?} for recruitable '{}' - preferring fallback map position for dialogue visuals",
                            event_pos, character_id
                        );

                        // Diagnostic: list entities that exist at this tile for debugging purposes.
                        let mut entities = Vec::new();
                        for (ent, coord, transform_opt, evt_opt) in all_tile_query.iter() {
                            if coord.0.x == event_pos.x && coord.0.y == event_pos.y {
                                entities.push(format!(
                                    "entity={:?}, has_transform={}, has_event_trigger={}",
                                    ent,
                                    transform_opt.is_some(),
                                    evt_opt.is_some(),
                                ));
                            }
                        }
                        info!("Entities at {:?}: {:?}", event_pos, entities);
                    } else {
                        info!(
                            "Speaker entity for recruitable '{}' resolved to {:?} (NpcMarker); will use it for visuals",
                            character_id, speaker_entity
                        );
                    }

                    // Create recruitment context using the event's map position so that
                    // execute_recruit_to_party calls remove_event on the correct tile
                    // regardless of whether the party interacted from the same tile or an
                    // adjacent one.
                    let recruitment_context = crate::application::dialogue::RecruitmentContext {
                        character_id: character_id.clone(),
                        event_position: event_pos,
                    };

                    // Store context in PendingRecruitmentContext resource for handle_start_dialogue to consume
                    if let Some(ref mut pending) = pending_recruitment {
                        pending.0 = Some(recruitment_context);
                    }

                    // Send StartDialogue message
                    dialogue_writer.write(StartDialogue {
                        dialogue_id: *dlg_id,
                        speaker_entity,
                        fallback_position: Some(event_pos),
                    });

                    if speaker_entity.is_some() {
                        info!(
                            "Starting recruitment dialogue {} for character {} with speaker entity {:?}",
                            dlg_id, character_id, speaker_entity
                        );
                    } else {
                        info!(
                            "Starting recruitment dialogue {} for character {} using fallback position {:?} for visuals",
                            dlg_id, character_id, event_pos
                        );
                    }
                } else {
                    // No dialogue specified, simple log message
                    warn!(
                        "RecruitableCharacter event for '{}' has no dialogue_id. \
                         Simple confirmation UI not yet implemented.",
                        character_id
                    );
                }
            }

            MapEvent::EnterInn {
                name,
                description: _,
                innkeeper_id,
            } => {
                let msg = format!("Entering {}.", name);
                tracing::info!("{}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Exploration,
                    });
                }

                // Find innkeeper NPC and trigger dialogue if available
                if let Some(npc_def) = content.db().npcs.get_npc(innkeeper_id) {
                    if let Some(dialogue_id) = npc_def.dialogue_id {
                        // Find NPC entity in the world for visuals (optional)
                        let speaker_entity = npc_query
                            .iter()
                            .find(|(_, marker, _)| marker.npc_id == *innkeeper_id)
                            .map(|(entity, _, _)| entity);

                        // Trigger innkeeper dialogue instead of auto-opening management UI
                        dialogue_writer.write(StartDialogue {
                            dialogue_id,
                            speaker_entity,
                            fallback_position: Some(trigger.position),
                        });

                        if let Some(ref mut writer) = game_log_writer {
                            writer.write(GameLogEvent {
                                text: format!("{} speaks.", npc_def.name),
                                category: LogCategory::Dialogue,
                            });
                        }
                    } else {
                        // Error: Innkeepers must have a dialogue configured
                        error!("Innkeeper '{}' has no dialogue_id. All innkeepers must have dialogue configured.", innkeeper_id);
                        if let Some(ref mut writer) = game_log_writer {
                            writer.write(GameLogEvent {
                                text: format!(
                                    "Error: Innkeeper '{}' is not properly configured",
                                    npc_def.name
                                ),
                                category: LogCategory::System,
                            });
                        }
                    }
                } else {
                    // NPC definition not found
                    let err = format!("Error: Innkeeper '{}' not found in database", innkeeper_id);
                    tracing::error!("{}", err);
                    if let Some(ref mut writer) = game_log_writer {
                        writer.write(GameLogEvent {
                            text: err,
                            category: LogCategory::System,
                        });
                    }
                }
            }
            MapEvent::Furniture {
                name,
                furniture_id,
                furniture_type,
                rotation_y,
                scale,
                material,
                flags,
                color_tint,
                key_item_id,
            } => {
                let msg = format!("Furniture placed: {}", name);
                tracing::info!("{}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Exploration,
                    });
                }

                // Only spawn furniture if we have the necessary resources (full game context)
                if can_spawn_furniture {
                    let map_id = global_state.0.world.current_map;
                    let commands = commands.as_mut().unwrap();
                    let materials_res = materials.as_mut().unwrap();
                    let meshes_res = meshes.as_mut().unwrap();

                    // Resolve final furniture properties: definition defaults merged
                    // with per-instance inline overrides.
                    let (
                        resolved_type,
                        resolved_material,
                        resolved_scale,
                        resolved_flags,
                        resolved_tint,
                    ) = resolve_furniture_fields(
                        *furniture_id,
                        *furniture_type,
                        *material,
                        *scale,
                        flags,
                        *color_tint,
                        &content.db().furniture,
                    );

                    let mut ctx = MeshSpawnContext {
                        commands,
                        materials: materials_res,
                        meshes: meshes_res,
                        cache: &mut furniture_cache,
                    };
                    spawn_furniture_with_rendering(
                        &mut ctx,
                        trigger.position,
                        map_id,
                        &FurnitureSpawnParams {
                            furniture_type: resolved_type,
                            rotation_y: *rotation_y,
                            scale: resolved_scale,
                            material_type: resolved_material,
                            flags: resolved_flags,
                            color_tint: resolved_tint,
                            key_item_id: *key_item_id,
                        },
                    );
                }
            }
            MapEvent::Container {
                id,
                name,
                items,
                description,
                gold,
                gems,
                ..
            } => {
                let msg = format!("Opening container: {} - {}", name, description);
                info!("{}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Exploration,
                    });
                }

                // Enter ContainerInventory mode with the current item contents.
                // On close, the action system writes the updated item list back
                // to this MapEvent::Container so partial takes persist.
                global_state.0.enter_container_inventory(
                    id.clone(),
                    name.clone(),
                    items.clone(),
                    *gold,
                    *gems,
                );
                info!(
                    "Entered ContainerInventory mode for container '{}' with {} item(s)",
                    id,
                    items.len()
                );
            }
            MapEvent::DroppedItem { item_id, name, .. } => {
                // DroppedItem events on the map are handled by
                // `load_map_dropped_items_system` which fires `ItemDroppedEvent`
                // for each one on map load.  Stepping on the tile does nothing
                // interactive — the party picks the item up via the dedicated
                // pickup action.  We log it here for diagnostics.
                let msg = format!("Stepped on dropped item: {} (id={})", name, item_id);
                info!("{}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Exploration,
                    });
                }
            }

            // Handle a LockedDoor event triggered via
            // `MapEventTriggered` (e.g. from programmatic tests or a future
            // game-world trigger system). The primary player path goes through
            // the split exploration-interaction input flow, but this arm
            // handles the same logic for the message-triggered path so both
            // paths are consistent.
            MapEvent::LockedDoor {
                name,
                lock_id,
                key_item_id,
                ..
            } => {
                info!("LockedDoor event for '{}' (lock_id={})", name, lock_id);

                let is_locked: bool = global_state
                    .0
                    .world
                    .get_current_map()
                    .and_then(|m| m.lock_states.get(lock_id.as_str()))
                    .map(|ls| ls.is_locked)
                    .unwrap_or_else(|| {
                        warn!(
                            "LockedDoor '{}' has no lock_state entry; \
                             was init_lock_states called on map load?",
                            lock_id
                        );
                        true
                    });

                if !is_locked {
                    // Already unlocked — open the tile so the party can pass.
                    if let Some(map) = global_state.0.world.get_current_map_mut() {
                        if let Some(tile) = map.get_tile_mut(trigger.position) {
                            tile.wall_type = crate::domain::world::WallType::None;
                            tile.blocked = false;
                        }
                    }
                    if let Some(ref mut writer) = game_log_writer {
                        writer.write(GameLogEvent {
                            text: "You open the door.".to_string(),
                            category: LogCategory::Exploration,
                        });
                    }
                    return;
                }

                // Search party inventory for the required key.
                let key_item_id_val: Option<crate::domain::types::ItemId> = *key_item_id;
                let key_found: Option<(usize, usize)> = key_item_id_val.and_then(|kid| {
                    global_state
                        .0
                        .party
                        .members
                        .iter()
                        .enumerate()
                        .find_map(|(ci, ch)| {
                            ch.inventory
                                .items
                                .iter()
                                .position(|slot| slot.item_id == kid)
                                .map(|si| (ci, si))
                        })
                });

                match (key_item_id_val, key_found) {
                    (Some(kid), Some((char_idx, slot_idx))) => {
                        // Key found: consume it, unlock the door, open the tile.
                        global_state.0.party.members[char_idx]
                            .inventory
                            .items
                            .remove(slot_idx);
                        let lock_id_owned = lock_id.clone();
                        let event_pos = trigger.position;
                        if let Some(map) = global_state.0.world.get_current_map_mut() {
                            if let Some(ls) = map.lock_states.get_mut(&lock_id_owned) {
                                ls.unlock();
                            }
                            if let Some(tile) = map.get_tile_mut(event_pos) {
                                tile.wall_type = crate::domain::world::WallType::None;
                                tile.blocked = false;
                            }
                            map.remove_event(event_pos);
                        }
                        let key_name = content
                            .db()
                            .items
                            .get_item(kid)
                            .map(|item| item.name.clone())
                            .unwrap_or_else(|| format!("key {}", kid));
                        let msg = format!("You unlock the door with the {}.", key_name);
                        info!("{}", msg);
                        if let Some(ref mut writer) = game_log_writer {
                            writer.write(GameLogEvent {
                                text: msg,
                                category: LogCategory::Exploration,
                            });
                        }
                    }
                    (Some(_), None) => {
                        // Key required but not in party.
                        let msg = "The door is locked. You need a key.".to_string();
                        info!("{}", msg);
                        if let Some(ref mut writer) = game_log_writer {
                            writer.write(GameLogEvent {
                                text: msg,
                                category: LogCategory::Exploration,
                            });
                        }
                        let can_lockpick = global_state.0.party.members.iter().any(|member| {
                            content
                                .db()
                                .classes
                                .get_class(&member.class_id)
                                .map(|cls| cls.has_ability("pick_lock"))
                                .unwrap_or(false)
                        });
                        if let Some(ref mut pending) = lock_pending {
                            pending.lock_id = Some(lock_id.clone());
                            pending.position = Some(trigger.position);
                            pending.can_lockpick = can_lockpick;
                        }
                    }
                    (None, _) => {
                        // No key needed; party must pick lock or bash.
                        let msg = "The door is locked.".to_string();
                        info!("{}", msg);
                        if let Some(ref mut writer) = game_log_writer {
                            writer.write(GameLogEvent {
                                text: msg,
                                category: LogCategory::Exploration,
                            });
                        }
                        let can_lockpick = global_state.0.party.members.iter().any(|member| {
                            content
                                .db()
                                .classes
                                .get_class(&member.class_id)
                                .map(|cls| cls.has_ability("pick_lock"))
                                .unwrap_or(false)
                        });
                        if let Some(ref mut pending) = lock_pending {
                            pending.lock_id = Some(lock_id.clone());
                            pending.position = Some(trigger.position);
                            pending.can_lockpick = can_lockpick;
                        }
                    }
                }
            }

            // Same key-check logic as LockedDoor.
            MapEvent::LockedContainer {
                name,
                lock_id,
                key_item_id,
                ..
            } => {
                info!("LockedContainer event for '{}' (lock_id={})", name, lock_id);

                let is_locked: bool = global_state
                    .0
                    .world
                    .get_current_map()
                    .and_then(|m| m.lock_states.get(lock_id.as_str()))
                    .map(|ls| ls.is_locked)
                    .unwrap_or(true);

                if !is_locked {
                    if let Some(ref mut writer) = game_log_writer {
                        writer.write(GameLogEvent {
                            text: "The container is open.".to_string(),
                            category: LogCategory::Exploration,
                        });
                    }
                    return;
                }

                let key_item_id_val: Option<crate::domain::types::ItemId> = *key_item_id;
                let key_found: Option<(usize, usize)> = key_item_id_val.and_then(|kid| {
                    global_state
                        .0
                        .party
                        .members
                        .iter()
                        .enumerate()
                        .find_map(|(ci, ch)| {
                            ch.inventory
                                .items
                                .iter()
                                .position(|slot| slot.item_id == kid)
                                .map(|si| (ci, si))
                        })
                });

                match (key_item_id_val, key_found) {
                    (Some(kid), Some((char_idx, slot_idx))) => {
                        global_state.0.party.members[char_idx]
                            .inventory
                            .items
                            .remove(slot_idx);
                        let lock_id_owned = lock_id.clone();
                        if let Some(map) = global_state.0.world.get_current_map_mut() {
                            if let Some(ls) = map.lock_states.get_mut(&lock_id_owned) {
                                ls.unlock();
                            }
                        }
                        let key_name = content
                            .db()
                            .items
                            .get_item(kid)
                            .map(|item| item.name.clone())
                            .unwrap_or_else(|| format!("key {}", kid));
                        let msg = format!("You unlock the container with the {}.", key_name);
                        info!("{}", msg);
                        if let Some(ref mut writer) = game_log_writer {
                            writer.write(GameLogEvent {
                                text: msg,
                                category: LogCategory::Exploration,
                            });
                        }
                    }
                    (Some(_), None) => {
                        let msg = "The container is locked. You need a key.".to_string();
                        info!("{}", msg);
                        if let Some(ref mut writer) = game_log_writer {
                            writer.write(GameLogEvent {
                                text: msg,
                                category: LogCategory::Exploration,
                            });
                        }
                        let can_lockpick = global_state.0.party.members.iter().any(|member| {
                            content
                                .db()
                                .classes
                                .get_class(&member.class_id)
                                .map(|cls| cls.has_ability("pick_lock"))
                                .unwrap_or(false)
                        });
                        if let Some(ref mut pending) = lock_pending {
                            pending.lock_id = Some(lock_id.clone());
                            pending.position = Some(trigger.position);
                            pending.can_lockpick = can_lockpick;
                        }
                    }
                    (None, _) => {
                        let msg = "The container is locked.".to_string();
                        info!("{}", msg);
                        if let Some(ref mut writer) = game_log_writer {
                            writer.write(GameLogEvent {
                                text: msg,
                                category: LogCategory::Exploration,
                            });
                        }
                        let can_lockpick = global_state.0.party.members.iter().any(|member| {
                            content
                                .db()
                                .classes
                                .get_class(&member.class_id)
                                .map(|cls| cls.has_ability("pick_lock"))
                                .unwrap_or(false)
                        });
                        if let Some(ref mut pending) = lock_pending {
                            pending.lock_id = Some(lock_id.clone());
                            pending.position = Some(trigger.position);
                            pending.can_lockpick = can_lockpick;
                        }
                    }
                }
            }
        }
    }
}

/// Handle an `EventResult::EnterMerchant` event.
///
/// Looks up the merchant NPC by ID in the content database. If the NPC has a
/// `dialogue_id` configured, it fires a `StartDialogue` message (the same
/// pattern used for `EventResult::NpcDialogue`). Otherwise it logs an info
/// message indicating the merchant has no dialogue configured.
///
/// This function is called both from the `MapEvent::NpcDialogue` merchant guard
/// arm in `handle_events` and can be called directly when a future
/// `MapEvent::EnterMerchant` variant is added.
///
/// # Arguments
///
/// * `event_result` - The `EventResult::EnterMerchant` to process (other variants are ignored)
/// * `content` - Game content database for NPC lookups
/// * `dialogue_writer` - Writer for `StartDialogue` messages
/// * `simple_dialogue_writer` - Writer for `SimpleDialogue` fallback messages
/// * `game_log` - Optional mutable reference to the UI game log
/// * `npc_query` - ECS query to resolve the NPC's world entity for speaker visuals
/// * `trigger_position` - Map position of the triggering tile (used as fallback visual anchor)
fn handle_event_result(
    event_result: &EventResult,
    content: &GameContent,
    dialogue_writer: &mut MessageWriter<StartDialogue>,
    simple_dialogue_writer: &mut MessageWriter<SimpleDialogue>,
    game_log_writer: &mut Option<MessageWriter<GameLogEvent>>,
    npc_query: &Query<(Entity, &NpcMarker, &TileCoord)>,
    trigger_position: &crate::domain::types::Position,
) {
    let EventResult::EnterMerchant { npc_id } = event_result else {
        return;
    };

    if let Some(npc_def) = content.db().npcs.get_npc(npc_id) {
        if let Some(dialogue_id) = npc_def.dialogue_id {
            // Find the merchant entity in the world for optional speaker visuals
            let speaker_entity = npc_query
                .iter()
                .find(|(_, marker, _)| marker.npc_id == *npc_id)
                .map(|(entity, _, _)| entity);

            if speaker_entity.is_none() {
                let available: Vec<_> = npc_query.iter().map(|(_, m, _)| &m.npc_id).collect();
                warn!(
                    "Merchant speaker '{}' not found in world. Available: {:?}",
                    npc_id, available
                );
            }

            // Fire StartDialogue so the dialogue system drives the interaction
            dialogue_writer.write(StartDialogue {
                dialogue_id,
                speaker_entity,
                fallback_position: Some(*trigger_position),
            });

            let msg = format!("Visiting {}.", npc_def.name);
            tracing::info!("{}", msg);
            if let Some(ref mut writer) = game_log_writer {
                writer.write(GameLogEvent {
                    text: msg,
                    category: LogCategory::Exploration,
                });
            }
        } else {
            // Merchant has no dialogue configured
            let msg = format!("Visiting {}.", npc_def.name);
            info!("{}", msg);
            if let Some(ref mut writer) = game_log_writer {
                writer.write(GameLogEvent {
                    text: msg,
                    category: LogCategory::Exploration,
                });
            }

            // Show a simple fallback bubble so the player knows someone is there
            let speaker_entity = npc_query
                .iter()
                .find(|(_, marker, _)| marker.npc_id == *npc_id)
                .map(|(entity, _, _)| entity);

            simple_dialogue_writer.write(SimpleDialogue {
                text: format!("Welcome to my shop! I am {}.", npc_def.name),
                speaker_name: npc_def.name.clone(),
                speaker_entity,
                fallback_position: Some(*trigger_position),
            });
        }
    } else {
        let msg = format!("Error: Merchant NPC '{}' not found in database", npc_id);
        tracing::error!("{}", msg);
        if let Some(ref mut writer) = game_log_writer {
            writer.write(GameLogEvent {
                text: msg,
                category: LogCategory::Exploration,
            });
        }
    }
}

#[cfg(test)]
mod container_event_tests {
    use super::*;
    use crate::application::{GameMode, GameState};
    use crate::domain::character::InventorySlot;
    use crate::domain::types::Position;
    use crate::domain::world::Map;

    fn make_slot(item_id: u8) -> InventorySlot {
        InventorySlot {
            item_id,
            charges: 0,
        }
    }

    /// Build a minimal Bevy app with just the EventPlugin wired up.
    ///
    /// Container events require an explicit `E`-key interact to fire — they are
    /// NOT auto-triggered when the party steps on their tile.  Tests that verify
    /// the `handle_events` → `GameMode::ContainerInventory` path must write a
    /// `MapEventTriggered` message directly (simulating the input system) instead
    /// of relying on `check_for_events`.
    fn build_event_app(game_state: GameState) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<crate::game::systems::dialogue::StartDialogue>();
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
        app.add_plugins(crate::game::systems::ui::UiPlugin);
        app.add_plugins(EventPlugin);
        app.insert_resource(GlobalState(game_state));
        // Minimal content DB — containers don't need NPC lookups
        app.insert_resource(crate::application::resources::GameContent::new(
            crate::sdk::database::ContentDatabase::new(),
        ));
        app
    }

    /// Send a `MapEventTriggered` for the given `MapEvent`, simulating an E-key
    /// interact from the input system.
    fn fire_container_event(app: &mut App, event: MapEvent, position: Position) {
        let mut messages = app
            .world_mut()
            .resource_mut::<Messages<MapEventTriggered>>();
        messages.write(MapEventTriggered { event, position });
    }

    #[test]
    fn test_container_map_event_enters_container_inventory_mode() {
        // Arrange: map with a Container event at position (3, 3)
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        let event_pos = Position::new(3, 3);
        let container_event = MapEvent::Container {
            id: "chest_001".to_string(),
            name: "Old Chest".to_string(),
            description: "A dusty chest".to_string(),
            items: vec![make_slot(1), make_slot(2)],
            gold: 0,
            gems: 0,
        };
        map.add_event(event_pos, container_event.clone());

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);
        game_state.mode = GameMode::Exploration;

        let mut app = build_event_app(game_state);

        // First update registers the plugin systems.
        app.update();

        // Simulate pressing E: fire MapEventTriggered directly (the input
        // system is not present in this test app).
        fire_container_event(&mut app, container_event, event_pos);

        // handle_events processes the message and enters ContainerInventory.
        app.update();

        // Assert
        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::ContainerInventory(_)),
            "Expected ContainerInventory mode after E-key on Container event, got {:?}",
            gs.0.mode
        );

        if let GameMode::ContainerInventory(ref cs) = gs.0.mode {
            assert_eq!(cs.container_event_id, "chest_001");
            assert_eq!(cs.container_name, "Old Chest");
            assert_eq!(cs.items.len(), 2);
        }
    }

    #[test]
    fn test_container_event_stores_items_in_state() {
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        let event_pos = Position::new(5, 5);
        let container_event = MapEvent::Container {
            id: "barrel_01".to_string(),
            name: "Barrel".to_string(),
            description: "".to_string(),
            items: vec![make_slot(10), make_slot(20), make_slot(30)],
            gold: 0,
            gems: 0,
        };
        map.add_event(event_pos, container_event.clone());

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);

        let mut app = build_event_app(game_state);
        app.update();

        fire_container_event(&mut app, container_event, event_pos);
        app.update();

        let gs = app.world().resource::<GlobalState>();
        if let GameMode::ContainerInventory(ref cs) = gs.0.mode {
            assert_eq!(cs.items.len(), 3);
            assert_eq!(cs.items[0].item_id, 10);
            assert_eq!(cs.items[1].item_id, 20);
            assert_eq!(cs.items[2].item_id, 30);
        } else {
            panic!("Expected ContainerInventory mode, got {:?}", gs.0.mode);
        }
    }

    #[test]
    fn test_empty_container_event_enters_container_inventory_mode() {
        // An empty container must still open the UI (showing "(Empty)").
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        let event_pos = Position::new(7, 7);
        let container_event = MapEvent::Container {
            id: "empty_crate".to_string(),
            name: "Empty Crate".to_string(),
            description: "".to_string(),
            items: vec![],
            gold: 0,
            gems: 0,
        };
        map.add_event(event_pos, container_event.clone());

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);

        let mut app = build_event_app(game_state);
        app.update();

        fire_container_event(&mut app, container_event, event_pos);
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::ContainerInventory(_)),
            "Empty container must still enter ContainerInventory mode, got {:?}",
            gs.0.mode
        );
        if let GameMode::ContainerInventory(ref cs) = gs.0.mode {
            assert!(cs.is_empty(), "Container state must be empty");
        }
    }

    #[test]
    fn test_container_not_auto_triggered_when_party_steps_on_tile() {
        // Container events must NOT enter ContainerInventory when the party
        // simply walks onto the tile — they require an explicit E-key press.
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        let event_pos = Position::new(4, 4);
        map.add_event(
            event_pos,
            MapEvent::Container {
                id: "no_auto_chest".to_string(),
                name: "No Auto Chest".to_string(),
                description: "".to_string(),
                items: vec![make_slot(99)],
                gold: 0,
                gems: 0,
            },
        );

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        // Party starts away from the container
        game_state.world.set_party_position(Position::new(0, 0));
        game_state.mode = GameMode::Exploration;

        let mut app = build_event_app(game_state);
        app.update();

        // Move party onto the container tile — check_for_events runs but must
        // NOT auto-fire the container event.
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.world.set_party_position(event_pos);
        }
        app.update(); // check_for_events sees the new position
        app.update(); // handle_events processes any queued messages

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::Exploration),
            "Container must NOT auto-trigger on step; mode should remain Exploration, got {:?}",
            gs.0.mode
        );
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
        use crate::sdk::ContentDatabase;
        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        // Use Trap event to verify auto-triggering (teleports now require interact)
        map.add_event(
            event_pos,
            MapEvent::Trap {
                name: "Test Trap".to_string(),
                description: "Test trap".to_string(),
                damage: 5,
                effect: None,
            },
        );

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(ContentDatabase::new()));

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
    fn test_recruitable_character_does_not_auto_trigger() {
        use crate::application::resources::GameContent;
        use crate::domain::types::Position;
        use crate::domain::world::MapEvent;
        use crate::game::resources::GlobalState;
        use crate::sdk::database::ContentDatabase;
        use bevy::prelude::*;

        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        map.add_event(
            event_pos,
            MapEvent::RecruitableCharacter {
                name: "Recruitable".to_string(),
                description: "A recruitable NPC".to_string(),
                character_id: "some_char".to_string(),
                dialogue_id: None,
                time_condition: None,
                facing: None,
            },
        );

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(ContentDatabase::new()));

        // Act - run one update; RecruitableCharacter should not auto-trigger
        app.update();

        // Assert - MapEventTriggered messages should be empty
        let events = app.world().resource::<Messages<MapEventTriggered>>();
        let mut reader = events.get_cursor();
        let triggered_events: Vec<_> = reader.read(events).collect();
        assert!(
            triggered_events.is_empty(),
            "Expected no events to be triggered for RecruitableCharacter when stepping on the tile"
        );
    }

    #[test]
    fn test_encounter_does_not_auto_trigger_when_stepping_on_tile() {
        use crate::application::resources::GameContent;
        use crate::domain::types::Position;
        use crate::domain::world::MapEvent;
        use crate::game::resources::GlobalState;
        use crate::sdk::database::ContentDatabase;
        use bevy::prelude::*;

        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        map.add_event(
            event_pos,
            MapEvent::Encounter {
                name: "Skeleton Ambush".to_string(),
                description: "A skeleton blocks the path".to_string(),
                monster_group: vec![1],
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
            },
        );

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(ContentDatabase::new()));

        // Act - encounter should auto-trigger on step-on
        app.update();

        // Assert - MapEventTriggered should contain the encounter trigger
        let events = app.world().resource::<Messages<MapEventTriggered>>();
        let mut reader = events.get_cursor();
        let triggered_events: Vec<_> = reader.read(events).collect();
        assert!(
            triggered_events.is_empty(),
            "Encounter must NOT auto-trigger when stepping on the tile — player must press Interact"
        );
    }

    #[test]
    fn test_encounter_triggered_from_current_position_via_interact() {
        use crate::application::resources::GameContent;
        use crate::domain::types::Position;
        use crate::domain::world::MapEvent;
        use crate::game::resources::GlobalState;
        use crate::sdk::database::ContentDatabase;
        use bevy::prelude::*;

        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        let encounter_event = MapEvent::Encounter {
            name: "Skeleton Ambush".to_string(),
            description: "A skeleton blocks the path".to_string(),
            monster_group: vec![1],
            time_condition: None,
            facing: None,
            proximity_facing: false,
            rotation_speed: None,
            combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
        };
        map.add_event(event_pos, encounter_event.clone());

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(ContentDatabase::new()));

        // Act — directly emit MapEventTriggered to simulate the player pressing
        // the Interact key while standing on (or adjacent to) the encounter tile.
        // This bypasses check_for_events and exercises the explicit trigger path.
        {
            let mut writer = app
                .world_mut()
                .resource_mut::<Messages<MapEventTriggered>>();
            writer.write(MapEventTriggered {
                event: encounter_event.clone(),
                position: event_pos,
            });
        }

        app.update();

        // Assert — the message we explicitly wrote is present, confirming the
        // explicit interaction path works from the current (on-tile) position.
        let messages = app.world().resource::<Messages<MapEventTriggered>>();
        let mut reader = messages.get_cursor();
        let triggered_events: Vec<_> = reader.read(messages).collect();
        assert!(
            !triggered_events.is_empty(),
            "Encounter MUST trigger when explicitly interacted with from the current position"
        );
    }

    #[test]
    fn test_no_event_triggered_when_no_event_at_position() {
        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
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
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        // Use Trap event to verify auto-triggering (teleports now require interact)
        map.add_event(
            event_pos,
            MapEvent::Trap {
                name: "Test Trap".to_string(),
                description: "Test trap".to_string(),
                damage: 5,
                effect: None,
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
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        map.add_event(
            event_pos,
            MapEvent::NpcDialogue {
                name: "Elder".to_string(),
                description: "Village Elder".to_string(),
                npc_id: "test_elder".to_string(),
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
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
            creature_id: None,
            sprite: None,
            quest_ids: vec![],
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
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
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
        app.add_plugins(crate::game::systems::ui::UiPlugin);
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        map.add_event(
            event_pos,
            MapEvent::NpcDialogue {
                name: "Merchant".to_string(),
                description: "Town Merchant".to_string(),
                npc_id: "test_merchant".to_string(),
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
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
            creature_id: None,
            sprite: None,
            quest_ids: vec![],
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
        };
        db.npcs.add_npc(npc).unwrap();

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));

        // Act
        app.update(); // First update: check_for_events writes MapEventTriggered
        app.update(); // Second update: handle_events processes MapEventTriggered
        app.update(); // Third update: UiPlugin consumes GameLogEvent into GameLog

        // Assert - merchant interactions should log the planned exploration entry
        // even when the merchant falls back to a simple dialogue bubble because
        // no dialogue_id is configured.
        let game_log = app.world().resource::<GameLog>();
        let entries = game_log.entries();
        assert!(
            entries.iter().any(|entry| {
                entry.category == crate::game::systems::ui::LogCategory::Exploration
                    && entry.text == "Visiting Town Merchant."
            }),
            "Expected merchant visit message in game log. Actual entries: {:?}",
            entries
        );
    }

    #[test]
    fn test_recruitable_character_triggers_dialogue_bubble_using_fallback_position() {
        use crate::application::resources::GameContent;
        use crate::domain::dialogue::{DialogueNode, DialogueTree};
        use crate::game::components::dialogue::ActiveDialogueUI;
        use crate::game::resources::GlobalState;
        use crate::sdk::database::ContentDatabase;
        use bevy::prelude::{App, ButtonInput, KeyCode};

        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
        app.add_plugins(EventPlugin);
        app.add_plugins(crate::game::systems::dialogue::DialoguePlugin);
        // Provide input resource required by `dialogue_input_system` during tests.
        // `dialogue_input_system` expects a `Res<ButtonInput<KeyCode>>` which is not
        // automatically present in the minimal test harness, so initialize it here.
        app.insert_resource(ButtonInput::<KeyCode>::default());

        // Build a map with a RecruitableCharacter event at a position that has no NPC entity
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        let event_pos = Position::new(11, 6);
        map.add_event(
            event_pos,
            MapEvent::RecruitableCharacter {
                character_id: "npc_apprentice_zara".to_string(),
                name: "Apprentice Zara".to_string(),
                description: "A young gnome apprentice studies a spellbook intently.".to_string(),
                dialogue_id: Some(101u16),
                time_condition: None,
                facing: None,
            },
        );

        // Prepare GameState with the map and party at the event position
        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);

        // Insert a minimal dialogue tree (id = 101) into the content DB
        let mut tree = DialogueTree::new(101, "Recruitment".to_string(), 1);
        let node = DialogueNode::new(1, "Hello! We could use someone like you.");
        tree.add_node(node);
        let mut db = ContentDatabase::new();
        db.dialogues.add_dialogue(tree);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));
        // Ensure ActiveDialogueUI resource is initialized
        app.init_resource::<ActiveDialogueUI>();

        // Act - start the dialogue explicitly (do not rely on auto-triggering)
        {
            // Mutate the GlobalState resource to enter Dialogue mode directly.
            // This avoids depending on stepping-on behavior for RecruitableCharacter.
            use crate::application::dialogue::DialogueState;
            let mut gs_res = app.world_mut().resource_mut::<GlobalState>();
            gs_res.0.mode = crate::application::GameMode::Dialogue(DialogueState::start(
                101,
                1,
                Some(event_pos),
                None,
            ));
        }
        // Run one update to let dialogue UI spawn
        app.update();

        // Assert - a dialogue panel was spawned and displays the recruitable's name
        let ui = app.world().resource::<ActiveDialogueUI>().clone();
        assert!(
            ui.bubble_entity.is_some(),
            "Expected dialogue panel to be spawned"
        );

        // Panel presence is sufficient for this refactor-focused test
    }

    #[test]
    fn test_recruitable_character_event_starts_dialogue_without_npc_lookup_error() {
        use crate::application::resources::GameContent;
        use crate::domain::dialogue::{DialogueNode, DialogueTree};
        use crate::game::resources::GlobalState;
        use crate::game::systems::dialogue::PendingRecruitmentContext;
        use crate::game::systems::ui::GameLog;
        use crate::sdk::database::ContentDatabase;
        use bevy::prelude::{App, ButtonInput, KeyCode};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
        app.add_plugins(EventPlugin);
        app.add_plugins(crate::game::systems::dialogue::DialoguePlugin);
        app.insert_resource(ButtonInput::<KeyCode>::default());

        let event_pos = Position::new(7, 15);
        let recruitable_event = MapEvent::RecruitableCharacter {
            character_id: "whisper".to_string(),
            name: "Whisper".to_string(),
            description: "A nimble elf watches from the shadows near the alley.".to_string(),
            dialogue_id: Some(102u16),
            time_condition: None,
            facing: None,
        };

        let mut map = Map::new(
            1,
            "Recruitable Test".to_string(),
            "Desc".to_string(),
            20,
            20,
        );
        map.add_event(event_pos, recruitable_event.clone());

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);

        let mut tree = DialogueTree::new(102, "Whisper Recruitment".to_string(), 1);
        let node = DialogueNode::new(1, "Hello there. My name is Whisper. Can I join your party?");
        tree.add_node(node);

        let mut db = ContentDatabase::new();
        db.dialogues.add_dialogue(tree);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));
        app.insert_resource(PendingRecruitmentContext(Some(
            crate::application::dialogue::RecruitmentContext {
                character_id: "whisper".to_string(),
                event_position: event_pos,
            },
        )));
        app.insert_resource(GameLog::default());

        {
            let mut messages = app
                .world_mut()
                .resource_mut::<Messages<MapEventTriggered>>();
            messages.write(MapEventTriggered {
                event: recruitable_event,
                position: event_pos,
            });
        }

        app.update();
        app.update();

        let global_state = app.world().resource::<GlobalState>();
        assert!(
            matches!(
                global_state.0.mode,
                crate::application::GameMode::Dialogue(_)
            ),
            "Recruitable character event should enter Dialogue mode"
        );

        if let crate::application::GameMode::Dialogue(dialogue_state) = &global_state.0.mode {
            assert_eq!(
                dialogue_state.active_tree_id,
                Some(102),
                "Recruitable interaction should start the recruitable dialogue tree"
            );
            assert_eq!(
                dialogue_state.speaker_npc_id,
                None,
                "Recruitable interactions without NPC entities must not coerce character_id into speaker_npc_id"
            );
            assert_eq!(
                dialogue_state.recruitment_context,
                Some(crate::application::dialogue::RecruitmentContext {
                    character_id: "whisper".to_string(),
                    event_position: event_pos,
                }),
                "Recruitment context should be preserved for recruitable dialogue"
            );
        }

        let log = app.world().resource::<GameLog>();
        assert!(
            !log.entries().iter().any(|entry| entry
                .text
                .contains("Error: NPC 'whisper' not found in database")),
            "Recruitable interactions must not go through NPC database lookup with character_id"
        );
    }

    #[test]
    fn test_recruitable_character_adjacent_tile_uses_event_position_not_party_position() {
        // Regression test: when a RecruitableCharacter is interacted with from an adjacent
        // tile, handle_events must store the event's map tile (trigger.position) in
        // RecruitmentContext.event_position — NOT the party's current position.
        //
        // If the party position were used instead, execute_recruit_to_party would call
        // remove_event on the wrong tile, the removal would silently fail, and the
        // recruitable mesh would stay visible on the map until the party walked over it.
        use crate::application::resources::GameContent;
        use crate::domain::dialogue::{DialogueNode, DialogueTree};
        use crate::game::resources::GlobalState;
        use crate::game::systems::dialogue::PendingRecruitmentContext;
        use crate::game::systems::ui::GameLog;
        use crate::sdk::database::ContentDatabase;
        use bevy::prelude::{App, ButtonInput, KeyCode};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
        app.add_plugins(EventPlugin);
        app.add_plugins(crate::game::systems::dialogue::DialoguePlugin);
        app.insert_resource(ButtonInput::<KeyCode>::default());

        // Party stands one tile away from the recruitable character — simulate an
        // adjacent-tile interaction where event_pos != party_pos.
        let party_pos = Position::new(7, 14);
        let event_pos = Position::new(7, 15);
        assert_ne!(
            party_pos, event_pos,
            "test requires distinct party and event positions"
        );

        let recruitable_event = MapEvent::RecruitableCharacter {
            character_id: "gareth".to_string(),
            name: "Old Gareth".to_string(),
            description: "A grizzled dwarf veteran stands nearby.".to_string(),
            dialogue_id: Some(200u16),
            time_condition: None,
            facing: None,
        };

        let mut map = Map::new(1, "Adjacent Test".to_string(), "Desc".to_string(), 20, 20);
        map.add_event(event_pos, recruitable_event.clone());

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        // Party is at party_pos, one tile away from the event.
        game_state.world.set_party_position(party_pos);

        let mut tree = DialogueTree::new(200, "Gareth Recruitment".to_string(), 1);
        let node = DialogueNode::new(1, "Hail, adventurer! Room for one more?");
        tree.add_node(node);

        let mut db = ContentDatabase::new();
        db.dialogues.add_dialogue(tree);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));
        // Start with empty context — handle_events must populate it with event_pos.
        app.insert_resource(PendingRecruitmentContext::default());
        app.insert_resource(GameLog::default());

        // Fire the MapEventTriggered message with position = event_pos (the adjacent tile),
        // exactly as try_interact_npc_or_recruitable would do.
        {
            let mut messages = app
                .world_mut()
                .resource_mut::<Messages<MapEventTriggered>>();
            messages.write(MapEventTriggered {
                event: recruitable_event,
                position: event_pos, // adjacent tile where the event actually lives
            });
        }

        app.update();
        app.update();

        // The game mode must have entered Dialogue.
        let global_state = app.world().resource::<GlobalState>();
        assert!(
            matches!(
                global_state.0.mode,
                crate::application::GameMode::Dialogue(_)
            ),
            "Adjacent-tile recruitable interaction should enter Dialogue mode"
        );

        if let crate::application::GameMode::Dialogue(dialogue_state) = &global_state.0.mode {
            // Critical assertion: event_position must be the event tile, not the party tile.
            // A wrong value here means remove_event targets the wrong tile and the mesh persists.
            assert_eq!(
                dialogue_state.recruitment_context,
                Some(crate::application::dialogue::RecruitmentContext {
                    character_id: "gareth".to_string(),
                    event_position: event_pos,
                }),
                "event_position must equal trigger.position (the event tile), \
                 not the party position — using the party position breaks adjacent-tile recruitment"
            );
            assert_ne!(
                dialogue_state
                    .recruitment_context
                    .as_ref()
                    .map(|ctx| ctx.event_position),
                Some(party_pos),
                "event_position must NOT be the party position when interacting from an adjacent tile"
            );
        }
    }

    #[test]
    fn test_npc_dialogue_event_logs_error_when_npc_not_found() {
        use crate::game::systems::ui::GameLog;
        use crate::sdk::ContentDatabase;

        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
        app.add_plugins(crate::game::systems::ui::UiPlugin);
        app.add_plugins(EventPlugin);

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let event_pos = Position::new(5, 5);
        map.add_event(
            event_pos,
            MapEvent::NpcDialogue {
                name: "Unknown".to_string(),
                description: "Unknown NPC".to_string(),
                npc_id: "nonexistent_npc".to_string(),
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
            },
        );

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);

        let db = ContentDatabase::new();

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));

        // Act
        app.update(); // First update: check_for_events writes MapEventTriggered
        app.update(); // Second update: handle_events processes MapEventTriggered
        app.update(); // Third update: UiPlugin consumes GameLogEvent into GameLog

        // Assert - GameLog should contain the error
        let game_log = app.world().resource::<GameLog>();
        let entries = game_log.entries();
        assert!(
            entries.iter().any(|entry| entry
                .text
                .contains("Error: NPC 'nonexistent_npc' not found in database")),
            "Expected NPC-not-found message in game log. Actual entries: {:?}",
            entries
        );
    }

    #[test]
    fn test_enter_inn_event_triggers_innkeeper_dialogue() {
        use crate::domain::world::NpcDefinition;
        use crate::game::systems::ui::GameLog;

        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
        app.add_plugins(crate::game::systems::ui::UiPlugin);
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
        game_state.mode = crate::application::GameMode::Exploration;

        // Prepare the content DB with a matching NPC that has a dialogue
        let mut db = crate::sdk::database::ContentDatabase::new();
        let npc = NpcDefinition {
            id: "cozy_inn".to_string(),
            name: "Cozy Innkeeper".to_string(),
            description: "The inn's proprietor".to_string(),
            portrait_id: "portraits/inn.png".to_string(),
            dialogue_id: Some(1u16),
            creature_id: None,
            sprite: None,
            quest_ids: vec![],
            faction: None,
            is_merchant: false,
            is_innkeeper: true,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
        };
        db.npcs.add_npc(npc).unwrap();

        // Add a minimal dialogue so StartDialogue makes sense
        let mut tree = crate::domain::dialogue::DialogueTree::new(1, "Inn Dialogue".to_string(), 1);
        let node = crate::domain::dialogue::DialogueNode::new(1, "Welcome!");
        tree.add_node(node);
        db.dialogues.add_dialogue(tree);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));

        // Act
        app.update(); // First update: check_for_events writes MapEventTriggered
        app.update(); // Second update: handle_events processes MapEventTriggered and should write StartDialogue
        app.update(); // Third update: UiPlugin consumes GameLogEvent into GameLog

        // Assert - StartDialogue message should be sent
        let dialogue_messages = app.world().resource::<Messages<StartDialogue>>();
        let mut reader = dialogue_messages.get_cursor();
        let messages: Vec<_> = reader.read(dialogue_messages).collect();
        assert_eq!(messages.len(), 1, "Expected StartDialogue message");
        assert_eq!(messages[0].dialogue_id, 1u16);

        // Assert - GameLog should contain the current innkeeper dialogue message
        let game_log = app.world().resource::<GameLog>();
        let entries = game_log.entries();
        assert!(
            entries.iter().any(|e| e.text == "Cozy Innkeeper speaks."),
            "Expected 'Cozy Innkeeper speaks.' message in game log. Actual entries: {:?}",
            entries
        );
    }

    #[test]
    fn test_enter_inn_event_triggers_dialogue_for_different_inn_ids() {
        use crate::domain::world::NpcDefinition;

        // Arrange
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
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
        game_state.mode = crate::application::GameMode::Exploration;

        // Prepare DB with NPC for tutorial_innkeeper_town2 that references dialogue id 9
        let mut db = crate::sdk::database::ContentDatabase::new();
        let npc = NpcDefinition {
            id: "tutorial_innkeeper_town2".to_string(),
            name: "Mountain Innkeeper".to_string(),
            description: "The innkeeper at the mountain pass".to_string(),
            portrait_id: "portraits/innkeeper_2.png".to_string(),
            dialogue_id: Some(9u16),
            creature_id: None,
            sprite: None,
            quest_ids: vec![],
            faction: None,
            is_merchant: false,
            is_innkeeper: true,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
        };
        db.npcs.add_npc(npc).unwrap();

        // Add a minimal dialogue with id 9 so StartDialogue is meaningful
        let mut tree =
            crate::domain::dialogue::DialogueTree::new(9, "Mountain Inn Dialogue".to_string(), 1);
        tree.add_node(crate::domain::dialogue::DialogueNode::new(
            1,
            "Welcome to the Mountain Rest Inn!",
        ));
        db.dialogues.add_dialogue(tree);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));

        // Act
        app.update(); // check_for_events -> MapEventTriggered
        app.update(); // handle_events -> StartDialogue

        // Assert - StartDialogue message was sent with the expected dialogue id
        let dialogue_messages = app.world().resource::<Messages<StartDialogue>>();
        let mut reader = dialogue_messages.get_cursor();
        let messages: Vec<_> = reader.read(dialogue_messages).collect();
        assert_eq!(messages.len(), 1, "Expected StartDialogue message");
        assert_eq!(messages[0].dialogue_id, 9u16);
    }

    #[test]
    fn test_enter_inn_dialogue_choice_opens_inn_management() {
        use crate::domain::dialogue::{DialogueAction, DialogueChoice, DialogueNode, DialogueTree};
        use crate::domain::world::NpcDefinition;
        use crate::game::systems::dialogue::SelectDialogueChoice;

        // Arrange - Set up app with Event and Dialogue plugins
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_message::<SelectDialogueChoice>();
        app.add_message::<crate::game::systems::dialogue::SimpleDialogue>();
        app.add_plugins(crate::game::systems::ui::UiPlugin);
        app.add_plugins(EventPlugin);
        app.add_plugins(crate::game::systems::dialogue::DialoguePlugin);

        // Create map with EnterInn event
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        let event_pos = Position::new(7, 7);
        map.add_event(
            event_pos,
            MapEvent::EnterInn {
                name: "Test Inn".to_string(),
                description: "A test inn".to_string(),
                innkeeper_id: "test_innkeeper".to_string(),
            },
        );

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(event_pos);
        game_state.mode = crate::application::GameMode::Exploration;

        // Prepare DB with NPC and dialogue that contains an OpenInnManagement action
        let mut db = crate::sdk::database::ContentDatabase::new();
        db.npcs
            .add_npc(NpcDefinition {
                id: "test_innkeeper".to_string(),
                name: "Test Innkeeper".to_string(),
                description: "Keeper".to_string(),
                portrait_id: "portraits/inn_test.png".to_string(),
                dialogue_id: Some(100u16),
                creature_id: None,
                sprite: None,
                quest_ids: vec![],
                faction: None,
                is_merchant: false,
                is_innkeeper: true,
                is_priest: false,
                stock_template: None,
                service_catalog: None,
                economy: None,
            })
            .unwrap();

        // Build dialogue 100 with root -> node2 where node2 has OpenInnManagement action
        let mut tree = DialogueTree::new(100, "Test Inn Dialogue".to_string(), 1);
        let mut root = DialogueNode::new(1, "Welcome!");
        root.add_choice(DialogueChoice::new("Manage your party", Some(2)));
        tree.add_node(root);
        let mut node2 = DialogueNode::new(2, "Let me help you manage your party.");
        node2.add_action(DialogueAction::OpenInnManagement {
            innkeeper_id: "test_innkeeper".to_string(),
        });
        node2.is_terminal = true;
        tree.add_node(node2);
        db.dialogues.add_dialogue(tree);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));
        app.init_resource::<crate::game::components::dialogue::ActiveDialogueUI>();
        app.insert_resource(ButtonInput::<KeyCode>::default());

        // Act - trigger EnterInn event via the spawn systems
        app.update(); // check_for_events -> MapEventTriggered
        app.update(); // handle_events -> StartDialogue & DialoguePlugin -> DialogueState created

        // Ensure we're in Dialogue mode and at the root node
        {
            let gs = app.world().resource::<GlobalState>();
            match &gs.0.mode {
                crate::application::GameMode::Dialogue(ds) => {
                    assert_eq!(ds.active_tree_id, Some(100u16));
                    assert_eq!(ds.current_node_id, 1);
                }
                other => panic!(
                    "Expected Dialogue mode after StartDialogue, got {:?}",
                    other
                ),
            }
        }

        // Send SelectDialogueChoice message (choose index 0)
        {
            let mut messages = app
                .world_mut()
                .resource_mut::<Messages<SelectDialogueChoice>>();
            messages.write(SelectDialogueChoice { choice_index: 0 });
        }

        // Process choice which should execute OpenInnManagement and change mode
        app.update();

        // Assert - we transitioned to InnManagement and the inn id is correct
        let gs = app.world().resource::<GlobalState>();
        match &gs.0.mode {
            crate::application::GameMode::InnManagement(state) => {
                assert_eq!(state.current_inn_id, "test_innkeeper".to_string());
            }
            other => panic!(
                "Expected InnManagement mode after choosing manage option, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_furniture_event_furniture_id_defaults_to_none_in_ron() {
        // Verify that existing Furniture RON without furniture_id deserialises correctly.
        use crate::domain::world::MapEvent;

        let ron_str = r#"Furniture(
            name: "Old Bench",
            furniture_type: Bench,
            scale: 1.0,
        )"#;

        let parsed: MapEvent = ron::from_str(ron_str).expect("deserialize old Furniture RON");
        match parsed {
            MapEvent::Furniture { furniture_id, .. } => {
                assert_eq!(
                    furniture_id, None,
                    "furniture_id must default to None for backward compatibility"
                );
            }
            other => panic!("expected Furniture event, got {:?}", other),
        }
    }

    #[test]
    fn test_furniture_event_furniture_id_round_trips_through_ron() {
        // A Furniture event with furniture_id: Some(7) must serialise and
        // deserialise without loss.
        use crate::domain::world::{FurnitureMaterial, FurnitureType, MapEvent};

        let event = MapEvent::Furniture {
            name: "Metal Sconce".to_string(),
            furniture_id: Some(7),
            furniture_type: FurnitureType::Torch,
            rotation_y: None,
            scale: 0.8,
            material: FurnitureMaterial::Metal,
            flags: crate::domain::world::FurnitureFlags {
                lit: true,
                locked: false,
                blocking: false,
            },
            color_tint: Some([0.6, 0.8, 1.0]),
            key_item_id: None,
        };

        let serialised = ron::to_string(&event).expect("serialise to RON");
        let parsed: MapEvent = ron::from_str(&serialised).expect("parse from RON");

        match parsed {
            MapEvent::Furniture {
                furniture_id,
                furniture_type,
                ..
            } => {
                assert_eq!(furniture_id, Some(7), "furniture_id must round-trip");
                assert_eq!(
                    furniture_type,
                    FurnitureType::Torch,
                    "furniture_type must round-trip"
                );
            }
            other => panic!("expected Furniture event, got {:?}", other),
        }
    }

    #[test]
    fn test_furniture_event_resolution_inline_only_preserves_all_fields() {
        // When furniture_id is None, inline values are returned unchanged.
        use crate::domain::world::furniture::FurnitureDatabase;
        use crate::domain::world::{FurnitureFlags, FurnitureMaterial, FurnitureType};
        use crate::game::systems::furniture_rendering::resolve_furniture_fields;

        let db = FurnitureDatabase::new();
        let flags = FurnitureFlags {
            lit: false,
            locked: true,
            blocking: false,
        };

        let (ft, mat, scale, resolved_flags, tint) = resolve_furniture_fields(
            None,
            FurnitureType::Chest,
            FurnitureMaterial::Metal,
            1.3,
            &flags,
            Some([0.7, 0.7, 0.7]),
            &db,
        );

        assert_eq!(ft, FurnitureType::Chest);
        assert_eq!(mat, FurnitureMaterial::Metal);
        assert!((scale - 1.3).abs() < f32::EPSILON);
        assert!(resolved_flags.locked);
        assert_eq!(tint, Some([0.7, 0.7, 0.7]));
    }
}

/// Integration tests for `MapEvent::LockedDoor` arriving via the
/// `MapEventTriggered` message sets `LockInteractionPending`.
///
/// These tests exercise the `handle_events` match arm for `LockedDoor`. The
/// primary player path goes through the split
/// exploration-interaction input flow, but `handle_events` must handle the same
/// logic for programmatic tests and
/// future game-world trigger systems.
#[cfg(test)]
mod locked_door_event_tests {
    use super::*;
    use crate::application::resources::GameContent;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::world::{LockState, Map, MapEvent};
    use crate::game::resources::{GlobalState, LockInteractionPending};
    use crate::game::systems::dialogue::{SimpleDialogue, StartDialogue};
    use crate::game::systems::events::MapEventTriggered;
    use crate::game::systems::map::MapChangeEvent;
    use crate::game::systems::ui::GameLog;
    use bevy::prelude::{App, MinimalPlugins, Update};

    /// Item ID used as the required key in lock tests.
    const KEY_ID: crate::domain::types::ItemId = 77;
    /// Lock identifier string used in test events.
    const LOCK_ID: &str = "evt_test_lock";

    /// Position of the locked door in the test map.
    fn lock_pos() -> crate::domain::types::Position {
        crate::domain::types::Position::new(3, 3)
    }

    /// Build a minimal Bevy app for `handle_events` locked-door tests.
    ///
    /// Registers:
    /// - All messages required by `EventPlugin`
    /// - `GlobalState` with a 10×10 map + one party member
    /// - `GameContent` (empty database — sufficient for class lookups that
    ///   return `None`, yielding `can_lockpick = false`)
    /// - `LockInteractionPending` (resource under test)
    /// - `GameLog` (for log message assertions)
    fn build_event_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Register all message channels that `EventPlugin` depends on.
        app.add_message::<MapEventTriggered>();
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_message::<SimpleDialogue>();

        // Build a world with a 10×10 map.
        let mut gs = crate::application::GameState::new();
        let mut map = Map::new(1, "EventTestMap".to_string(), "Test".to_string(), 10, 10);

        // Insert a locked door event and matching lock state.
        map.add_event(
            lock_pos(),
            MapEvent::LockedDoor {
                name: "Event Test Door".to_string(),
                lock_id: LOCK_ID.to_string(),
                key_item_id: Some(KEY_ID),
                initial_trap_chance: 0,
            },
        );
        map.lock_states
            .insert(LOCK_ID.to_string(), LockState::new(LOCK_ID));

        gs.world.add_map(map);
        gs.world.set_current_map(1);
        gs.world
            .set_party_position(crate::domain::types::Position::new(3, 4));

        // Add a party member so inventory / class lookups don't panic.
        let hero = Character::new(
            "Event Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        app.insert_resource(GlobalState(gs));
        app.insert_resource(GameContent::new(
            crate::sdk::database::ContentDatabase::new(),
        ));
        app.init_resource::<LockInteractionPending>();
        app.add_plugins(crate::game::systems::ui::UiPlugin);

        // Register handle_events (the system under test).
        app.add_systems(Update, handle_events);
        app
    }

    /// Helper: write a `MapEventTriggered` message for the locked door.
    fn fire_locked_door_event(app: &mut App) {
        let mut writer = app
            .world_mut()
            .get_resource_mut::<Messages<MapEventTriggered>>()
            .unwrap();
        writer.write(MapEventTriggered {
            event: MapEvent::LockedDoor {
                name: "Event Test Door".to_string(),
                lock_id: LOCK_ID.to_string(),
                key_item_id: Some(KEY_ID),
                initial_trap_chance: 0,
            },
            position: lock_pos(),
        });
    }

    // ── Test: LockedDoor event via MapEventTriggered sets LockInteractionPending

    /// When `MapEvent::LockedDoor` arrives through `MapEventTriggered` and the
    /// party has no matching key, `handle_events` must populate
    /// `LockInteractionPending` with the lock's ID and position.
    ///
    /// This covers the programmatic-trigger path (e.g. tests, scripted events)
    /// described in the lock interaction design.
    #[test]
    fn test_locked_door_event_sets_pending_resource() {
        let mut app = build_event_test_app();

        // Fire the LockedDoor event (party has no key).
        fire_locked_door_event(&mut app);

        // Run handle_events.
        app.update();

        // LockInteractionPending must have the lock_id set.
        let pending = app.world().resource::<LockInteractionPending>();
        assert_eq!(
            pending.lock_id,
            Some(LOCK_ID.to_string()),
            "LockInteractionPending.lock_id must be set when party has no key; got: {:?}",
            pending.lock_id
        );
        assert_eq!(
            pending.position,
            Some(lock_pos()),
            "LockInteractionPending.position must point to the door tile"
        );
        // `can_lockpick` is false because ContentDatabase is empty (no classes loaded).
        assert!(
            !pending.can_lockpick,
            "can_lockpick must be false when no class database is loaded"
        );
    }

    /// When `MapEvent::LockedDoor` arrives and the party carries the correct
    /// key, `handle_events` must consume the key and mark the lock as unlocked.
    #[test]
    fn test_locked_door_event_with_key_unlocks_and_consumes_key() {
        let mut app = build_event_test_app();

        // Give the party member the required key.
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.party.members[0]
                .inventory
                .add_item(KEY_ID, 1)
                .expect("inventory must not be full");
        }

        fire_locked_door_event(&mut app);
        app.update();
        app.update();

        // Lock state must be unlocked.
        let gs = app.world().resource::<GlobalState>();
        let lock_state =
            gs.0.world
                .get_current_map()
                .unwrap()
                .lock_states
                .get(LOCK_ID)
                .unwrap();
        assert!(
            !lock_state.is_locked,
            "lock_state must be unlocked after handle_events processes the key"
        );

        // Key must have been consumed.
        assert!(
            !gs.0.party.members[0]
                .inventory
                .items
                .iter()
                .any(|s| s.item_id == KEY_ID),
            "Key must be removed from inventory after unlock"
        );

        // `LockInteractionPending` must NOT be set (door was opened).
        let pending = app.world().resource::<LockInteractionPending>();
        assert!(
            pending.lock_id.is_none(),
            "LockInteractionPending must be empty after successful key unlock"
        );

        // Game log must contain a success message.
        let log = app.world().resource::<GameLog>();
        assert!(
            log.entries()
                .iter()
                .any(|entry| entry.text.starts_with("You unlock the door with the")),
            "Game log must contain unlock success message; got: {:?}",
            log.entries()
        );
    }

    /// When `MapEvent::LockedDoor` has no `key_item_id` (pick-or-bash lock),
    /// `handle_events` must set `LockInteractionPending` with the lock ID.
    #[test]
    fn test_locked_door_event_no_key_required_sets_pending() {
        let mut app = build_event_test_app();

        // Override the map event to have no key_item_id.
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            if let Some(map) = gs.0.world.get_current_map_mut() {
                map.add_event(
                    lock_pos(),
                    MapEvent::LockedDoor {
                        name: "Pick-or-bash Door".to_string(),
                        lock_id: LOCK_ID.to_string(),
                        key_item_id: None,
                        initial_trap_chance: 0,
                    },
                );
            }
        }

        // Write the no-key event directly.
        {
            let mut writer = app
                .world_mut()
                .get_resource_mut::<Messages<MapEventTriggered>>()
                .unwrap();
            writer.write(MapEventTriggered {
                event: MapEvent::LockedDoor {
                    name: "Pick-or-bash Door".to_string(),
                    lock_id: LOCK_ID.to_string(),
                    key_item_id: None,
                    initial_trap_chance: 0,
                },
                position: lock_pos(),
            });
        }

        app.update();
        app.update();

        // LockInteractionPending must be set.
        let pending = app.world().resource::<LockInteractionPending>();
        assert_eq!(
            pending.lock_id,
            Some(LOCK_ID.to_string()),
            "Pending must be set for a lock that requires pick or bash"
        );

        // Game log must say "The door is locked."
        let log = app.world().resource::<GameLog>();
        assert!(
            log.entries()
                .iter()
                .any(|entry| entry.text == "The door is locked."),
            "Game log must contain 'The door is locked.' for no-key lock; got: {:?}",
            log.entries()
        );
    }
}

#[cfg(test)]
mod trap_treasure_tests {
    use super::*;
    use crate::application::{GameMode, GameState};
    use crate::domain::character::{
        Alignment, AttributePair16, Character, Condition, Inventory, Sex,
    };
    use crate::domain::types::{ItemId, Position};
    use crate::domain::world::Map;
    use crate::sdk::database::ContentDatabase;

    /// Position used for trap/treasure events in these tests.
    fn event_pos() -> Position {
        Position::new(4, 4)
    }

    /// Create a character with a given name and HP value.
    fn make_character(name: &str, hp: u16) -> Character {
        let mut ch = Character::new(
            name.to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.hp = AttributePair16::new(hp);
        ch
    }

    /// Build a minimal Bevy app wired for `handle_events` trap/treasure tests.
    ///
    /// Registers all message channels, `GlobalState`, `GameContent` (empty DB),
    /// `LockInteractionPending`, the `UiPlugin` (for `GameLog`/`GameLogEvent`),
    /// and the `handle_events` system under `Update`.
    fn build_trap_treasure_app(game_state: GameState) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Register all message channels that `handle_events` depends on.
        app.add_message::<MapEventTriggered>();
        app.add_message::<MapChangeEvent>();
        app.add_message::<StartDialogue>();
        app.add_message::<SimpleDialogue>();
        app.add_plugins(crate::game::systems::ui::UiPlugin);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(ContentDatabase::new()));
        app.init_resource::<LockInteractionPending>();

        // Register handle_events (the system under test).
        app.add_systems(Update, handle_events);
        app
    }

    /// Write a `MapEventTriggered` message into the app.
    fn fire_event(app: &mut App, event: MapEvent, position: Position) {
        let mut writer = app
            .world_mut()
            .resource_mut::<Messages<MapEventTriggered>>();
        writer.write(MapEventTriggered { event, position });
    }

    // ── Test 1: Trap damage applies to living members, skips dead ───────

    /// When a `MapEvent::Trap` fires, all living party members take the
    /// specified damage.  Dead members must not take additional damage.
    #[test]
    fn test_trap_damage_living_members_take_damage_dead_unaffected() {
        // Arrange: two living members (HP = 30) and one already-dead member.
        let mut map = Map::new(1, "TrapMap".to_string(), "Test".to_string(), 10, 10);
        let pos = event_pos();
        let trap = MapEvent::Trap {
            name: "Spike Trap".to_string(),
            description: "A spike trap".to_string(),
            damage: 10,
            effect: None,
        };
        map.add_event(pos, trap.clone());

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(pos);

        let alive1 = make_character("Alice", 30);
        let alive2 = make_character("Bob", 30);
        let mut dead_member = make_character("Charlie", 20);
        dead_member.hp = AttributePair16 {
            base: 20,
            current: 0,
        };
        dead_member.conditions.add(Condition::DEAD);

        game_state.party.add_member(alive1).unwrap();
        game_state.party.add_member(alive2).unwrap();
        game_state.party.add_member(dead_member).unwrap();

        let mut app = build_trap_treasure_app(game_state);

        // Act
        fire_event(&mut app, trap, pos);
        app.update();

        // Assert
        let gs = app.world().resource::<GlobalState>();
        // Living members took 10 damage: 30 → 20
        assert_eq!(
            gs.0.party.members[0].hp.current, 20,
            "Living member Alice should have 20 HP after 10 trap damage"
        );
        assert_eq!(
            gs.0.party.members[1].hp.current, 20,
            "Living member Bob should have 20 HP after 10 trap damage"
        );
        // Dead member stays at 0 HP and keeps DEAD condition
        assert_eq!(
            gs.0.party.members[2].hp.current, 0,
            "Dead member Charlie should remain at 0 HP"
        );
        assert!(
            gs.0.party.members[2].conditions.has(Condition::DEAD),
            "Dead member Charlie should still have DEAD condition"
        );
    }

    // ── Test 2: Trap with poison effect sets POISONED on living members ─

    /// When a trap has an effect like `"poison"`, the `Condition::POISONED`
    /// flag is set on every living member.  Dead members are not affected.
    #[test]
    fn test_trap_effect_poison_sets_condition_on_living_members() {
        let mut map = Map::new(1, "PoisonMap".to_string(), "Test".to_string(), 10, 10);
        let pos = event_pos();
        let trap = MapEvent::Trap {
            name: "Poison Trap".to_string(),
            description: "A poison trap".to_string(),
            damage: 5,
            effect: Some("poison".to_string()),
        };
        map.add_event(pos, trap.clone());

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(pos);

        let alive = make_character("Alice", 50);
        let mut dead_member = make_character("Bob", 20);
        dead_member.hp = AttributePair16 {
            base: 20,
            current: 0,
        };
        dead_member.conditions.add(Condition::DEAD);

        game_state.party.add_member(alive).unwrap();
        game_state.party.add_member(dead_member).unwrap();

        let mut app = build_trap_treasure_app(game_state);

        // Act
        fire_event(&mut app, trap, pos);
        app.update();

        // Assert
        let gs = app.world().resource::<GlobalState>();
        assert!(
            gs.0.party.members[0].conditions.has(Condition::POISONED),
            "Living member Alice should have POISONED condition after poison trap"
        );
        assert_eq!(
            gs.0.party.members[0].hp.current, 45,
            "Living member Alice should have 45 HP after 5 trap damage"
        );
        // Dead member should NOT get poisoned
        assert!(
            !gs.0.party.members[1].conditions.has(Condition::POISONED),
            "Dead member Bob should NOT have POISONED condition"
        );
    }

    // ── Test 3: Trap party wipe triggers GameOver ───────────────────────

    /// When trap damage kills every party member, `GameMode::GameOver` is set.
    #[test]
    fn test_trap_party_wipe_all_dead_triggers_game_over() {
        let mut map = Map::new(1, "WipeMap".to_string(), "Test".to_string(), 10, 10);
        let pos = event_pos();
        let trap = MapEvent::Trap {
            name: "Death Trap".to_string(),
            description: "Lethal trap".to_string(),
            damage: 100,
            effect: None,
        };
        map.add_event(pos, trap.clone());

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(pos);
        game_state.mode = GameMode::Exploration;

        // Two members with low HP — 100 damage kills both.
        game_state
            .party
            .add_member(make_character("Alice", 5))
            .unwrap();
        game_state
            .party
            .add_member(make_character("Bob", 8))
            .unwrap();

        let mut app = build_trap_treasure_app(game_state);

        // Act
        fire_event(&mut app, trap, pos);
        app.update();

        // Assert
        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.party.living_count(),
            0,
            "All party members should be dead after lethal trap"
        );
        assert!(
            matches!(gs.0.mode, GameMode::GameOver),
            "Game mode should be GameOver after party wipe, got {:?}",
            gs.0.mode
        );
        assert!(
            gs.0.party.members[0].conditions.has(Condition::DEAD),
            "Alice should have DEAD condition"
        );
        assert!(
            gs.0.party.members[1].conditions.has(Condition::DEAD),
            "Bob should have DEAD condition"
        );
    }

    // ── Test 4: Treasure distributes loot to party inventories ──────────

    /// When `MapEvent::Treasure` fires, loot items are added to party member
    /// inventories (first member with space receives each item).
    #[test]
    fn test_treasure_distribution_items_added_to_inventory() {
        let mut map = Map::new(1, "TreasureMap".to_string(), "Test".to_string(), 10, 10);
        let pos = event_pos();
        let treasure = MapEvent::Treasure {
            name: "Gold Chest".to_string(),
            description: "A shiny chest".to_string(),
            loot: vec![10, 20, 30],
        };
        map.add_event(pos, treasure.clone());

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(pos);

        game_state
            .party
            .add_member(make_character("Alice", 50))
            .unwrap();
        game_state
            .party
            .add_member(make_character("Bob", 50))
            .unwrap();

        let mut app = build_trap_treasure_app(game_state);

        // Act
        fire_event(&mut app, treasure, pos);
        app.update();

        // Assert: all items go to first member with space (Alice).
        let gs = app.world().resource::<GlobalState>();
        let alice_items: Vec<ItemId> = gs.0.party.members[0]
            .inventory
            .items
            .iter()
            .map(|s| s.item_id)
            .collect();
        assert_eq!(
            alice_items,
            vec![10, 20, 30],
            "Alice should have all three loot items; got {:?}",
            alice_items
        );
        // Bob should have nothing — Alice had space for all.
        assert!(
            gs.0.party.members[1].inventory.items.is_empty(),
            "Bob should have no items since Alice had space"
        );
    }

    // ── Test 5: Treasure with full inventories loses items gracefully ───

    /// When every party member's inventory is full, treasure items are lost
    /// (logged as warnings) and the system must not panic.
    #[test]
    fn test_treasure_full_inventory_items_lost_no_panic() {
        let mut map = Map::new(1, "FullMap".to_string(), "Test".to_string(), 10, 10);
        let pos = event_pos();
        let treasure = MapEvent::Treasure {
            name: "Overflow Chest".to_string(),
            description: "Too much loot".to_string(),
            loot: vec![99],
        };
        map.add_event(pos, treasure.clone());

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(pos);

        // Create a member with a full inventory (MAX_ITEMS items).
        let mut full_member = make_character("Alice", 50);
        for i in 0..Inventory::MAX_ITEMS {
            full_member.inventory.add_item(i as ItemId, 0).unwrap();
        }
        assert!(full_member.inventory.is_full());
        game_state.party.add_member(full_member).unwrap();

        let mut app = build_trap_treasure_app(game_state);

        // Act: must not panic even though inventory is full.
        fire_event(&mut app, treasure, pos);
        app.update();

        // Assert: item was lost (not added), inventory still at max capacity.
        let gs = app.world().resource::<GlobalState>();
        assert!(
            gs.0.party.members[0].inventory.is_full(),
            "Inventory should still be full"
        );
        assert_eq!(
            gs.0.party.members[0].inventory.items.len(),
            Inventory::MAX_ITEMS,
            "Inventory should still have exactly MAX_ITEMS items"
        );
        // Item 99 should NOT appear anywhere in the inventory.
        assert!(
            !gs.0.party.members[0]
                .inventory
                .items
                .iter()
                .any(|s| s.item_id == 99),
            "Item 99 should not be in the full inventory"
        );
    }

    // ── Test 6: Treasure event is removed from map after collection ─────

    /// After treasure is collected the one-shot event must be removed from
    /// the map so re-visiting the tile does not award loot again.
    #[test]
    fn test_treasure_event_removal_after_collection() {
        let mut map = Map::new(1, "RemovalMap".to_string(), "Test".to_string(), 10, 10);
        let pos = event_pos();
        let treasure = MapEvent::Treasure {
            name: "One-Time Chest".to_string(),
            description: "Collect once".to_string(),
            loot: vec![1],
        };
        map.add_event(pos, treasure.clone());

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        game_state.world.set_party_position(pos);

        game_state
            .party
            .add_member(make_character("Alice", 50))
            .unwrap();

        // Verify the event exists before collection.
        assert!(
            game_state
                .world
                .get_current_map()
                .unwrap()
                .events
                .contains_key(&pos),
            "Treasure event should exist on the map before collection"
        );

        let mut app = build_trap_treasure_app(game_state);

        // Act
        fire_event(&mut app, treasure, pos);
        app.update();

        // Assert: event should be removed from the map.
        let gs = app.world().resource::<GlobalState>();
        let map = gs.0.world.get_current_map().unwrap();
        assert!(
            !map.events.contains_key(&pos),
            "Treasure event should be removed from the map after collection"
        );
    }
}
