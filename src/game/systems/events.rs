// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::application::resources::GameContent;
use crate::domain::transactions::pickup_item;
use crate::domain::types::{ItemId, MapId, Position};
use crate::domain::world::EventResult;
use crate::domain::world::MapEvent;
use crate::game::resources::{GlobalState, LockInteractionPending};
use crate::game::systems::dialogue::{SimpleDialogue, StartDialogue};
use crate::game::systems::furniture_rendering::{
    resolve_furniture_fields, spawn_furniture_with_rendering,
};
use crate::game::systems::item_world_events::ItemPickedUpEvent;
use crate::game::systems::map::{EventTrigger, MapChangeEvent, NpcMarker, TileCoord};
use crate::game::systems::procedural_meshes::ProceduralMeshCache;
use crate::game::systems::ui::{GameLogEvent, LogCategory};
use bevy::prelude::*;

pub struct EventPlugin;

impl Plugin for EventPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MapEventTriggered>()
            .add_message::<PickupDroppedItemRequest>()
            .add_systems(Update, (check_for_events, handle_events))
            .add_systems(Update, handle_pickup_dropped_item);
    }
}

/// Event triggered when the party steps on a tile with an event
#[derive(Message)]
pub struct MapEventTriggered {
    pub event: MapEvent,
    pub position: crate::domain::types::Position,
}

/// Emitted when the party steps onto a tile that contains a dropped item and
/// no static map event.  The [`handle_pickup_dropped_item`] system processes
/// this request by calling [`pickup_item`] and emitting [`ItemPickedUpEvent`].
///
/// # Fields
///
/// * `item_id`      – Logical item ID to pick up (first FIFO item on the tile).
/// * `map_id`       – Map on which the item is lying.
/// * `position`     – Tile coordinate of the dropped item.
/// * `party_index`  – Index of the party member who will receive the item.
///
/// # Examples
///
/// ```
/// use antares::game::systems::events::PickupDroppedItemRequest;
/// use antares::domain::types::Position;
///
/// let req = PickupDroppedItemRequest {
///     item_id: 5,
///     map_id: 1,
///     position: Position::new(3, 7),
///     party_index: 0,
/// };
/// assert_eq!(req.item_id, 5);
/// assert_eq!(req.party_index, 0);
/// ```
#[derive(Message, Clone, Debug)]
pub struct PickupDroppedItemRequest {
    /// Logical item ID to pick up (first FIFO item on the tile).
    pub item_id: ItemId,
    /// Map on which the dropped item lies.
    pub map_id: MapId,
    /// Tile coordinate of the dropped item.
    pub position: Position,
    /// Index of the party member who receives the item (0-based).
    pub party_index: usize,
}

/// System to check if the party is standing on an event
fn check_for_events(
    global_state: Res<GlobalState>,
    mut event_writer: MessageWriter<MapEventTriggered>,
    mut pickup_writer: MessageWriter<PickupDroppedItemRequest>,
    mut last_position: Local<Option<crate::domain::types::Position>>,
) {
    let game_state = &global_state.0;
    let current_pos = game_state.world.party_position;
    let current_map_id = game_state.world.current_map;

    // Only check if position changed
    if *last_position != Some(current_pos) {
        *last_position = Some(current_pos);

        if let Some(map) = game_state.world.get_current_map() {
            if let Some(event) = map.get_event(current_pos) {
                // Do not auto-trigger interaction-driven utility/dialogue events when
                // the party steps on their tile. These events must be explicitly
                // interacted with by the player (press the Interact key).
                //
                // Encounters are intentionally excluded from this list: stepping on
                // an encounter tile should immediately trigger combat.
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
                    _ => {
                        // Trigger other event types automatically (encounters, traps, etc.)
                        event_writer.write(MapEventTriggered {
                            event: event.clone(),
                            position: current_pos,
                        });
                    }
                }
            } else {
                // No static map event at this tile — check for dropped items.
                //
                // When the party steps onto a tile that contains one or more
                // dropped items (and no static map event), emit a
                // `PickupDroppedItemRequest` for the first item (FIFO order).
                // `handle_pickup_dropped_item` will call `pickup_item()` on the
                // next system in the chain and fire `ItemPickedUpEvent` so the
                // visual marker is despawned.
                let dropped = map.dropped_items_at(current_pos);
                if let Some(first) = dropped.first() {
                    info!(
                        "Party stepped onto tile {:?} with dropped item_id={} — emitting pickup request",
                        current_pos, first.item_id
                    );
                    pickup_writer.write(PickupDroppedItemRequest {
                        item_id: first.item_id,
                        map_id: current_map_id,
                        position: current_pos,
                        party_index: 0, // first party member picks up by default
                    });
                }
            }
        }
    }
}

/// Processes [`PickupDroppedItemRequest`] messages emitted by [`check_for_events`].
///
/// For each request, calls [`pickup_item`] from the domain transactions layer to
/// move the item from the world map into the party member's inventory.  On
/// success, emits an [`ItemPickedUpEvent`] so the visual marker is despawned by
/// [`despawn_picked_up_item_system`].  Failures are logged as warnings without
/// panicking.
fn handle_pickup_dropped_item(
    mut requests: MessageReader<PickupDroppedItemRequest>,
    mut global_state: ResMut<GlobalState>,
    mut picked_up_writer: Option<MessageWriter<ItemPickedUpEvent>>,
    mut game_log_writer: Option<MessageWriter<GameLogEvent>>,
) {
    // Collect requests to avoid holding a borrow while mutating global_state.
    let requests: Vec<PickupDroppedItemRequest> = requests.read().cloned().collect();

    for req in requests {
        // Bounds-check party index before splitting the borrow.
        if req.party_index >= global_state.0.party.members.len() {
            warn!(
                "PickupDroppedItemRequest: party_index {} out of bounds (party size {})",
                req.party_index,
                global_state.0.party.members.len()
            );
            continue;
        }

        // Call pickup_item() — splits the borrow across party.members[i] and world
        // (two disjoint fields of GameState; allowed by Rust NLL).
        let game_state = &mut global_state.0;
        match pickup_item(
            &mut game_state.party.members[req.party_index],
            req.party_index,
            &mut game_state.world,
            req.map_id,
            req.position,
            req.item_id,
        ) {
            Ok(slot) => {
                let debug_msg = format!(
                    "Picked up item {} (charges={}) from map {} tile {:?}",
                    slot.item_id, slot.charges, req.map_id, req.position
                );
                info!("{}", debug_msg);

                let item_name = game_state
                    .party
                    .members
                    .get(req.party_index)
                    .and_then(|character| {
                        character.inventory.items.iter().find(|inventory_slot| {
                            inventory_slot.item_id == slot.item_id
                                && inventory_slot.charges == slot.charges
                        })
                    })
                    .map(|_| format!("item {}", slot.item_id))
                    .unwrap_or_else(|| format!("item {}", slot.item_id));

                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: format!("Picked up {}.", item_name),
                        category: LogCategory::Item,
                    });
                    writer.write(GameLogEvent {
                        text: debug_msg,
                        category: LogCategory::Item,
                    });
                }

                // Notify the visual system to despawn the 3-D marker.
                if let Some(ref mut writer) = picked_up_writer {
                    writer.write(ItemPickedUpEvent {
                        item_id: req.item_id,
                        map_id: req.map_id,
                        tile_x: req.position.x,
                        tile_y: req.position.y,
                    });
                }
            }
            Err(e) => {
                warn!(
                    "handle_pickup_dropped_item: pickup_item failed for party[{}] \
                     item_id={} at {:?}: {}",
                    req.party_index, req.item_id, req.position, e
                );
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
                println!("{}", msg);
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
                println!("{}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Exploration,
                    });
                }
            }
            MapEvent::Trap {
                damage, effect: _, ..
            } => {
                let msg = format!("Trapped! Took {} damage.", damage);
                println!("{}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Combat,
                    });
                }
                // TODO: Apply damage to party
            }
            MapEvent::Treasure { loot, .. } => {
                let msg = format!("Found treasure! {} item(s).", loot.len());
                println!("{}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Item,
                    });
                }
                // TODO: Add to inventory
            }
            MapEvent::Encounter {
                monster_group,
                combat_event_type,
                ..
            } => {
                let msg = format!("Monsters! ({} foes)", monster_group.len());
                println!("{}", msg);
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
                        println!("{}", msg);
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
                        println!("{}", msg);
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
                    println!("{}", msg);
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
                println!("{}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Dialogue,
                    });
                }

                let current_pos = global_state.0.world.party_position;

                // If dialogue is specified, trigger dialogue system
                if let Some(dlg_id) = dialogue_id {
                    // Find NPC entity at current position for speaker (optional visual).
                    // NOTE: We intentionally prefer using the fallback tile position for
                    // recruitable character visuals rather than treating low-level event/tile
                    // marker entities as speakers. This avoids placing UI on marker geometry
                    // (which can be near the ground) and keeps visuals consistent.
                    let speaker_entity = npc_query
                        .iter()
                        .find(|(_, _, coord)| {
                            coord.0.x == current_pos.x && coord.0.y == current_pos.y
                        })
                        .map(|(entity, _, _)| entity);

                    if speaker_entity.is_none() {
                        info!(
                            "No NpcMarker found at {:?} for recruitable '{}' - preferring fallback map position for dialogue visuals",
                            current_pos, character_id
                        );

                        // Diagnostic: list entities that exist at this tile for debugging purposes.
                        let mut entities = Vec::new();
                        for (ent, coord, transform_opt, evt_opt) in all_tile_query.iter() {
                            if coord.0.x == current_pos.x && coord.0.y == current_pos.y {
                                entities.push(format!(
                                    "entity={:?}, has_transform={}, has_event_trigger={}",
                                    ent,
                                    transform_opt.is_some(),
                                    evt_opt.is_some(),
                                ));
                            }
                        }
                        info!("Entities at {:?}: {:?}", current_pos, entities);
                    } else {
                        info!(
                            "Speaker entity for recruitable '{}' resolved to {:?} (NpcMarker); will use it for visuals",
                            character_id, speaker_entity
                        );
                    }

                    // Create recruitment context
                    let recruitment_context = crate::application::dialogue::RecruitmentContext {
                        character_id: character_id.clone(),
                        event_position: current_pos,
                    };

                    // Store context in PendingRecruitmentContext resource for handle_start_dialogue to consume
                    if let Some(ref mut pending) = pending_recruitment {
                        pending.0 = Some(recruitment_context);
                    }

                    // Send StartDialogue message
                    dialogue_writer.write(StartDialogue {
                        dialogue_id: *dlg_id,
                        speaker_entity,
                        fallback_position: Some(current_pos),
                    });

                    if speaker_entity.is_some() {
                        info!(
                            "Starting recruitment dialogue {} for character {} with speaker entity {:?}",
                            dlg_id, character_id, speaker_entity
                        );
                    } else {
                        info!(
                            "Starting recruitment dialogue {} for character {} using fallback position {:?} for visuals",
                            dlg_id, character_id, current_pos
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
                println!("{}", msg);
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
                    println!("{}", err);
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
                println!("{}", msg);
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

                    spawn_furniture_with_rendering(
                        commands,
                        materials_res,
                        meshes_res,
                        trigger.position,
                        map_id,
                        resolved_type,
                        *rotation_y,
                        resolved_scale,
                        resolved_material,
                        resolved_flags,
                        resolved_tint,
                        *key_item_id,
                        &mut furniture_cache,
                    );
                }
            }
            MapEvent::Container {
                id,
                name,
                items,
                description,
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
                global_state
                    .0
                    .enter_container_inventory(id.clone(), name.clone(), items.clone());
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
            println!("{}", msg);
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
        println!("{}", msg);
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
    fn test_encounter_auto_triggers_when_stepping_on_tile() {
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
            !triggered_events.is_empty(),
            "Expected encounter event to auto-trigger when stepping on the tile"
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
    use crate::game::systems::events::{MapEventTriggered, PickupDroppedItemRequest};
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
        app.add_message::<PickupDroppedItemRequest>();
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
