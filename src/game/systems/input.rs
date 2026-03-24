// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Input System Module
//!
//! Provides config-driven input handling with customizable key bindings and movement cooldown.
//!
//! # Overview
//!
//! The input system translates keyboard input into game actions using configurable key mappings
//! from `ControlsConfig`. This allows campaigns to customize controls and provides a foundation
//! for player-remappable keys.
//!
//! # Features
//!
//! - **Config-Driven Key Mapping**: All key bindings come from `ControlsConfig`
//! - **Multiple Keys Per Action**: Each action can be triggered by multiple keys
//! - **Configurable Cooldown**: Movement cooldown prevents accidental double-moves
//! - **Door Interaction**: Space/E to open doors (configurable via interact keys)
//! - **Classic Movement**: Arrow keys and WASD (default, fully customizable)
//!
//! # Examples
//!
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::systems::input::InputPlugin;
//! use antares::sdk::game_config::ControlsConfig;
//!
//! # fn setup() {
//! let config = ControlsConfig::default();
//! let mut app = App::new();
//! app.add_plugins(InputPlugin::new(config));
//! # }
//! ```

use crate::application::dialogue::RecruitmentContext;

use crate::domain::world::{MapEvent, WallType};

use crate::game::components::furniture::DoorState;
use crate::game::components::FurnitureEntity;
use crate::game::resources::{GlobalState, LockInteractionPending};
use crate::game::systems::dialogue::PendingRecruitmentContext;
use crate::game::systems::events::MapEventTriggered;
use crate::game::systems::map::{NpcMarker, TileCoord};
#[cfg(test)]
use crate::game::systems::rest::InitiateRestEvent;
use crate::game::systems::ui::GameLog;
use crate::sdk::game_config::ControlsConfig;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

mod frame_input;
mod global_toggles;
mod helpers;
mod keymap;
mod menu_toggle;

pub use frame_input::{decode_frame_input, FrameInputIntent};
pub use global_toggles::handle_global_mode_toggles;
pub use helpers::get_adjacent_positions;
pub use keymap::{parse_key_code, GameAction, KeyMap};
pub use menu_toggle::toggle_menu_state;

/// Input plugin with config-driven key mappings
///
/// Manages input handling with customizable key bindings and movement cooldown.
pub struct InputPlugin {
    /// Controls configuration for key mappings and cooldown
    config: ControlsConfig,
}

impl InputPlugin {
    /// Create a new input plugin with the given controls configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Controls configuration defining key bindings and cooldown
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::input::InputPlugin;
    /// use antares::sdk::game_config::ControlsConfig;
    ///
    /// let config = ControlsConfig::default();
    /// let plugin = InputPlugin::new(config);
    /// ```
    pub fn new(config: ControlsConfig) -> Self {
        Self { config }
    }
}

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        // Insert input config as a resource
        let key_map = KeyMap::from_controls_config(&self.config);
        app.insert_resource(InputConfigResource {
            controls: self.config.clone(),
            key_map,
        });

        // Phase 2 (locks): register LockInteractionPending so handle_input can
        // signal Phase 3's UI when the player interacts with a locked door.
        app.init_resource::<LockInteractionPending>();

        app.add_systems(Update, handle_input);
    }
}

/// Bevy resource containing input configuration and key mappings
///
/// This resource makes the controls configuration available to input systems.
#[derive(Resource)]
pub struct InputConfigResource {
    /// Controls configuration
    pub controls: ControlsConfig,

    /// Compiled key map for efficient input checking
    pub key_map: KeyMap,
}

/// Handle keyboard input and translate to game actions.
///
/// This system processes keyboard input using the configured key mappings,
/// applies movement cooldown, and updates game state accordingly.
///
/// Phase 6: exploration mouse-to-interact currently uses the documented
/// fallback centre-screen click heuristic from the mouse-input plan rather than
/// full Bevy mesh picking. A left mouse click inside the centre third of the
/// primary window is treated as the same interaction target as the keyboard
/// `Interact` action: the tile directly ahead of the party plus the existing
/// adjacent-tile and NPC interaction checks. This keeps keyboard and mouse
/// exploration interaction on the same canonical logic path with no duplicated
/// routing behavior.
/// TODO: Upgrade this fallback to full world picking once clickable entity
/// picking is wired for exploration meshes and billboards.
#[allow(clippy::too_many_arguments)]
fn handle_input(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    input_config: Res<InputConfigResource>,
    mut global_state: ResMut<GlobalState>,
    mut map_event_messages: MessageWriter<MapEventTriggered>,
    mut recruitment_context: ResMut<PendingRecruitmentContext>,

    time: Res<Time>,
    mut last_move_time: Local<f32>,
    victory_roots: Query<Entity, With<crate::game::systems::combat::VictorySummaryRoot>>,
    npc_query: Query<(Entity, &NpcMarker, &TileCoord)>,

    primary_window: Query<&Window, With<PrimaryWindow>>,
    game_content: Option<Res<crate::application::resources::GameContent>>,
    // Query for furniture door entities — provides open/locked state and
    // allows toggling the door transform without a full map respawn.
    mut door_entity_query: Query<(
        &mut FurnitureEntity,
        &mut DoorState,
        &mut Transform,
        &TileCoord,
    )>,
    // Game log for player-visible feedback messages (e.g. "Door is locked.")
    mut game_log: ResMut<GameLog>,
    // Phase 2 (locks): resource that signals Phase 3's UI when the player
    // interacts with a locked door or container.
    mut lock_pending: ResMut<LockInteractionPending>,
) {
    let current_time = time.elapsed_secs();
    let cooldown = input_config.controls.movement_cooldown;
    let frame_input = decode_frame_input(
        &input_config.key_map,
        &keyboard_input,
        &mouse_buttons,
        primary_window.single().ok(),
    );

    if handle_global_mode_toggles(&mut global_state.0, frame_input, game_content.as_deref()) {
        return;
    }

    // Throttle movement input using cooldown. Only block when an actual movement
    // action is being attempted.
    let is_movement_attempt = frame_input.is_movement_attempt();

    if is_movement_attempt && (current_time - *last_move_time < cooldown) {
        // Movement attempted but still within cooldown window - ignore movement input.
        return;
    }

    // ALLOW input processing in Dialogue mode to enable "Move to Cancel"
    // But block Interaction actions (doors, etc.) if in Dialogue.
    // BLOCK all movement/interaction input when in Menu mode (menu system handles its own input)

    let game_state = &mut global_state.0;

    // Menu toggle handled above before movement cooldown checks.

    // Block all movement/interaction input when in Menu, Inventory, or Automap mode.
    // Each mode's own system handles its own input processing.
    if matches!(
        game_state.mode,
        crate::application::GameMode::Menu(_)
            | crate::application::GameMode::Inventory(_)
            | crate::application::GameMode::Automap
    ) {
        return;
    }

    // Block all movement/interaction input when in Combat mode.
    // Combat action input is handled exclusively by combat_input_system.
    if matches!(game_state.mode, crate::application::GameMode::Combat(_)) {
        return;
    }

    // Block all movement/interaction input when resting or in the rest menu.
    // The rest orchestration system drives the rest sequence; the player
    // cannot walk away mid-rest.
    if matches!(
        game_state.mode,
        crate::application::GameMode::Resting(_) | crate::application::GameMode::RestMenu
    ) {
        return;
    }

    let world = &mut game_state.world;
    let mut moved = false;

    // Interact - check for doors, NPCs, signs, teleports
    //
    // NOTE: We intentionally use "pressed" (not "just pressed") for keyboard
    // interaction so behavior stays consistent with the existing movement model
    // and door behavior, and so headless tests can exercise interaction without
    // depending on per-frame edge detection.
    //
    // Phase 6 fallback mouse path: a left click inside the centre third of the
    // primary window is treated as an `Interact` action on the tile directly
    // ahead of the party. This reuses the exact same logic below rather than
    // duplicating event-routing behavior.
    // Only allow Interaction if NOT in Dialogue mode
    if !matches!(game_state.mode, crate::application::GameMode::Dialogue(_))
        && frame_input.is_interact_attempt()
    {
        let party_position = world.party_position;
        let adjacent_tiles = get_adjacent_positions(party_position);

        // Door interaction - check if there's a door in front of the party
        let target = world.position_ahead();

        // ── Phase 3: Furniture door interaction ───────────────────────────
        // Check for a furniture-based door entity at the tile ahead.
        // Using a local flag to avoid holding query borrows across the `return`.
        {
            let mut furniture_door_handled = false;
            for (mut furniture_entity, mut door_state, mut door_transform, tile_coord) in
                door_entity_query.iter_mut()
            {
                if tile_coord.0 != target {
                    continue;
                }
                furniture_door_handled = true;

                if door_state.is_locked {
                    // Check if any party member carries the required key.
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
                        // Unlock and open in one action.
                        door_state.is_locked = false;
                        door_state.is_open = true;
                        furniture_entity.blocking = false;
                        door_transform.rotation = Quat::from_rotation_y(
                            door_state.base_rotation_y + std::f32::consts::FRAC_PI_2,
                        );
                        if let Some(map) = world.get_current_map_mut() {
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
                    // Toggle open ↔ closed.
                    door_state.is_open = !door_state.is_open;
                    furniture_entity.blocking = !door_state.is_open;

                    let angle = if door_state.is_open {
                        door_state.base_rotation_y + std::f32::consts::FRAC_PI_2
                    } else {
                        door_state.base_rotation_y
                    };
                    door_transform.rotation = Quat::from_rotation_y(angle);

                    // Sync tile blocked state so movement checks are accurate.
                    if let Some(map) = world.get_current_map_mut() {
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
                break;
            }

            if furniture_door_handled {
                return; // Door handled; don't fall through to other checks
            }
        }

        // ── Phase 2: Tile-based LockedDoor (MapEvent) interaction ────────────
        //
        // Check whether the tile directly ahead carries a `MapEvent::LockedDoor`.
        // This is separate from the furniture-entity door system above: it uses
        // the map's event table and `lock_states` HashMap rather than Bevy ECS
        // components.
        //
        // Evaluation order (mirrors `try_unlock` semantics):
        //   1. If the map event at `target` is `LockedDoor` and the lock is
        //      already open, open the tile and return early.
        //   2. If locked and the party carries the required key: consume it,
        //      unlock the tile, log success, return early.
        //   3. If locked but no matching key in the party: log the locked
        //      message, populate `LockInteractionPending` for Phase 3's UI,
        //      and return early.
        {
            // Step A: read locked-door event info (immutable borrow of world).
            let locked_door_info: Option<(String, Option<crate::domain::types::ItemId>)> = world
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

            if let Some((lock_id, key_item_id)) = locked_door_info {
                // Step B: read lock status (immutable).
                let is_locked: bool = world
                    .get_current_map()
                    .and_then(|m| m.lock_states.get(&lock_id))
                    .map(|ls| ls.is_locked)
                    .unwrap_or_else(|| {
                        // lock_states entry missing → init_lock_states was not
                        // called yet for this map. Treat as locked and warn.
                        warn!(
                            "LockedDoor lock_id '{}' has no lock_state entry; \
                             was init_lock_states() called on map load?",
                            lock_id
                        );
                        true
                    });

                if !is_locked {
                    // Already unlocked from a previous interaction. Open the
                    // tile (WallType::Door → None) so the party can walk through.
                    if let Some(map) = world.get_current_map_mut() {
                        if let Some(tile) = map.get_tile_mut(target) {
                            tile.wall_type = WallType::None;
                            tile.blocked = false;
                        }
                    }
                    info!("Previously unlocked door at {:?} opened", target);
                    return;
                }

                // Step C: search party inventory for the required key (immutable).
                // `game_state.party` and `world` (= `game_state.world`) are
                // disjoint fields of `GameState`, so the borrow checker accepts
                // both mutable borrows simultaneously.
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
                        // ── Key found: consume it, unlock, open the door ──────
                        game_state.party.members[char_idx]
                            .inventory
                            .items
                            .remove(slot_idx);

                        if let Some(map) = world.get_current_map_mut() {
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
                            .as_deref()
                            .and_then(|gc| gc.db().items.get_item(kid))
                            .map(|item| item.name.clone())
                            .unwrap_or_else(|| format!("key {}", kid));
                        let msg = format!("You unlock the door with the {}.", key_name);
                        info!("{}", msg);
                        game_log.add(msg);
                        return;
                    }
                    (Some(_), None) => {
                        // ── Key required but not in party ─────────────────────
                        let msg = "The door is locked. You need a key.".to_string();
                        info!("{}", msg);
                        game_log.add(msg);
                        let can_lockpick = game_content
                            .as_deref()
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
                        lock_pending.position = Some(target);
                        lock_pending.can_lockpick = can_lockpick;
                        return;
                    }
                    (None, _) => {
                        // ── No key required: must pick lock or bash ───────────
                        let msg = "The door is locked.".to_string();
                        info!("{}", msg);
                        game_log.add(msg);
                        let can_lockpick = game_content
                            .as_deref()
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
                        lock_pending.position = Some(target);
                        lock_pending.can_lockpick = can_lockpick;
                        return;
                    }
                }
            }
        }
        // ── End Phase 2 locked-door block ─────────────────────────────────────

        // ── Phase 3: Tile-based LockedContainer (MapEvent) interaction ─────────
        //
        // When the tile directly ahead carries a `MapEvent::LockedContainer`,
        // follow the same key-check logic as the LockedDoor path above.
        //
        // On success (key found or already unlocked), fire a
        // `MapEventTriggered` for the replacement `Container` event so that
        // `handle_events` enters `ContainerInventory` mode — this avoids any
        // borrow-checker conflict with `world = &mut game_state.world`.
        {
            // Step A: read locked-container event info (immutable).
            let locked_container_info: Option<(
                String,
                String,
                Option<crate::domain::types::ItemId>,
            )> = world
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

            if let Some((lock_id, container_name, key_item_id)) = locked_container_info {
                // Step B: check lock status (immutable).
                let is_locked: bool = world
                    .get_current_map()
                    .and_then(|m| m.lock_states.get(&lock_id))
                    .map(|ls| ls.is_locked)
                    .unwrap_or_else(|| {
                        warn!(
                            "LockedContainer lock_id '{}' has no lock_state entry; \
                             was init_lock_states() called on map load?",
                            lock_id
                        );
                        true
                    });

                if !is_locked {
                    // Already unlocked — replace with Container event and fire
                    // MapEventTriggered so handle_events enters container mode.
                    let id = lock_id.clone();
                    let name = container_name.clone();
                    if let Some(map) = world.get_current_map_mut() {
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
                    return;
                }

                // Step C: search party inventory for the required key (immutable).
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
                        // ── Key found: consume it, unlock, open the container ──
                        game_state.party.members[char_idx]
                            .inventory
                            .items
                            .remove(slot_idx);

                        let id = lock_id.clone();
                        let name = container_name.clone();

                        // Mutate world — last use of `world` in this branch.
                        if let Some(map) = world.get_current_map_mut() {
                            if let Some(ls) = map.lock_states.get_mut(&lock_id) {
                                ls.unlock();
                            }
                            // Replace LockedContainer with open Container event.
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

                        // Fire MapEventTriggered so handle_events enters container mode.
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
                            .as_deref()
                            .and_then(|gc| gc.db().items.get_item(kid))
                            .map(|item| item.name.clone())
                            .unwrap_or_else(|| format!("key {}", kid));
                        let msg =
                            format!("You unlock the {} with the {}.", container_name, key_name);
                        info!("{}", msg);
                        game_log.add(msg);
                        return;
                    }
                    (Some(_), None) => {
                        // ── Key required but not in party ─────────────────────
                        let msg = "The container is locked. You need a key.".to_string();
                        info!("{}", msg);
                        game_log.add(msg);
                        let can_lockpick = game_content
                            .as_deref()
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
                        lock_pending.position = Some(target);
                        lock_pending.can_lockpick = can_lockpick;
                        return;
                    }
                    (None, _) => {
                        // ── No key required: must pick lock or bash ───────────
                        let msg = "The container is locked.".to_string();
                        info!("{}", msg);
                        game_log.add(msg);
                        let can_lockpick = game_content
                            .as_deref()
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
                        lock_pending.position = Some(target);
                        lock_pending.can_lockpick = can_lockpick;
                        return;
                    }
                }
            }
        }
        // ── End Phase 3 locked-container block ────────────────────────────────

        // ── Tile-based WallType::Door fallback ────────────────────────────────
        //
        // If the tile directly ahead is a plain WallType::Door tile with no
        // associated LockedDoor map event (handled above), open it immediately
        // by resetting the wall type and clearing the blocked flag. This is the
        // "existing behaviour" referenced in the Phase 2 spec for non-locked
        // tile-based doors.
        {
            let has_plain_door = world
                .get_current_map()
                .and_then(|m| m.get_tile(target))
                .map(|t| t.wall_type == WallType::Door)
                .unwrap_or(false);

            if has_plain_door {
                if let Some(map) = world.get_current_map_mut() {
                    if let Some(tile) = map.get_tile_mut(target) {
                        tile.wall_type = WallType::None;
                        tile.blocked = false;
                    }
                }
                info!("Opened door at {:?}", target);
                return;
            }
        }
        // ── End tile-based door fallback ──────────────────────────────────────

        // Snapshot current map state for adjacency checks (no mutation needed)
        let Some(map) = world.get_current_map() else {
            info!("No interactable object nearby");
            return;
        };

        // Check for NPC in any adjacent tile
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
            return;
        }

        // Support explicit encounter interaction at current tile as a fallback.
        // This helps recover from legacy maps/positions where the party may already
        // stand on an encounter tile.
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
                return;
            }
        }

        // Check for a Container event at the current tile first (party may
        // already be standing on the container tile).
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
                return;
            }
        }

        // Check for interaction-driven map events in any adjacent tile.
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
                        return;
                    }
                    MapEvent::RecruitableCharacter {
                        name,
                        character_id,
                        dialogue_id: _dialogue_id,
                        ..
                    } => {
                        info!(
                            "Interacting with recruitable character '{}' (ID: {}) at {:?}",
                            name, character_id, position
                        );
                        // Find the NPC entity at this position
                        let _speaker_entity = npc_query
                            .iter()
                            .find(|(_, _, tile_coord)| tile_coord.0 == position)
                            .map(|(entity, _, _)| entity);

                        // Set recruitment context so the dialogue system knows who to recruit
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
                        return;
                    }
                    _ => continue,
                }
            }
        }

        // No interactable found
        info!("No interactable object nearby");
    }
    // Move forward
    else if frame_input.move_forward {
        let target = game_state.world.position_ahead();
        let facing = game_state.world.party_facing;

        // Phase 3: deny movement into a locked furniture door, surfacing
        // MovementError::DoorLocked semantics at the input layer.
        let locked_door_ahead = door_entity_query
            .iter()
            .any(|(_, ds, _, tc)| tc.0 == target && ds.is_locked && !ds.is_open);

        if locked_door_ahead {
            let msg = "The door is locked.".to_string();
            info!("{}", msg);
            game_log.add(msg);
        } else if let Some(content) = game_content.as_deref() {
            match game_state.move_party_and_handle_events(facing, content.db()) {
                Ok(()) => moved = true,
                Err(crate::application::MoveHandleError::Movement(
                    crate::domain::world::MovementError::DoorLocked(_, _),
                )) => {
                    let msg = "The door is locked.".to_string();
                    info!("{}", msg);
                    game_log.add(msg);
                }
                Err(crate::application::MoveHandleError::Movement(
                    crate::domain::world::MovementError::Blocked(_, _),
                )) => {}
                Err(crate::application::MoveHandleError::Movement(
                    crate::domain::world::MovementError::OutOfBounds(_, _),
                )) => {}
                Err(err) => {
                    warn!("move forward failed: {}", err);
                }
            }
        } else if let Some(map) = game_state.world.get_current_map() {
            if !map.is_blocked(target) {
                game_state.world.set_party_position(target);
                moved = true;
            }
        }
    }
    // Move backward
    else if frame_input.move_back {
        // Route backward movement through the same world movement path so
        // fog-of-war reveal and tile events stay in sync with position updates.
        let back_facing = game_state.world.party_facing.turn_left().turn_left();

        if let Some(content) = game_content.as_deref() {
            match game_state.move_party_and_handle_events(back_facing, content.db()) {
                Ok(()) => moved = true,
                Err(crate::application::MoveHandleError::Movement(
                    crate::domain::world::MovementError::DoorLocked(_, _),
                )) => {
                    let msg = "The door is locked.".to_string();
                    info!("{}", msg);
                    game_log.add(msg);
                }
                Err(crate::application::MoveHandleError::Movement(
                    crate::domain::world::MovementError::Blocked(_, _),
                )) => {}
                Err(crate::application::MoveHandleError::Movement(
                    crate::domain::world::MovementError::OutOfBounds(_, _),
                )) => {}
                Err(err) => {
                    warn!("move backward failed: {}", err);
                }
            }
        } else {
            let target = back_facing.forward(game_state.world.party_position);
            if let Some(map) = game_state.world.get_current_map() {
                if !map.is_blocked(target) {
                    game_state.world.set_party_position(target);
                    moved = true;
                }
            }
        }
    }
    // Turn left
    else if frame_input.turn_left {
        game_state.world.turn_left();
        if matches!(game_state.mode, crate::application::GameMode::Exploration) {
            let party_position = game_state.world.party_position;
            crate::domain::world::mark_visible_area(
                &mut game_state.world,
                party_position,
                crate::domain::world::VISIBILITY_RADIUS,
            );
        }
        moved = true;
    }
    // Turn right
    else if frame_input.turn_right {
        game_state.world.turn_right();
        if matches!(game_state.mode, crate::application::GameMode::Exploration) {
            let party_position = game_state.world.party_position;
            crate::domain::world::mark_visible_area(
                &mut game_state.world,
                party_position,
                crate::domain::world::VISIBILITY_RADIUS,
            );
        }
        moved = true;
    }

    if moved {
        *last_move_time = current_time;

        // If we moved while in Dialogue mode, cancel the dialogue
        if matches!(game_state.mode, crate::application::GameMode::Dialogue(_)) {
            info!("Movement detected during dialogue - cancelling dialogue");
            // Switch back to exploration mode
            game_state.mode = crate::application::GameMode::Exploration;
        }

        // Dismiss post-combat victory overlay once normal movement controls are
        // used again in exploration flow.
        if moved {
            for entity in victory_roots.iter() {
                commands.entity(entity).despawn();
            }
        }

        // TODO: Check for events at new position (Phase 4)
    }
}

#[cfg(test)]
mod dialogue_inventory_tests {
    use super::*;
    use crate::application::resources::GameContent;
    use crate::domain::world::npc::NpcDefinition;
    use crate::sdk::database::ContentDatabase;
    use bevy::prelude::{App, ButtonInput, KeyCode, Time, Update};

    /// Helper: build a minimal Bevy app for I-key-in-dialogue tests.
    ///
    /// Inserts a `GameContent` resource populated with the given `ContentDatabase`
    /// so the `handle_input` system can resolve NPC definitions.
    fn build_dialogue_input_app(
        db: ContentDatabase,
        initial_mode: crate::application::GameMode,
    ) -> App {
        let mut app = App::new();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        let cfg = crate::sdk::game_config::ControlsConfig::default();
        let km = KeyMap::from_controls_config(&cfg);
        app.insert_resource(InputConfigResource {
            controls: cfg,
            key_map: km,
        });
        let mut gs = crate::application::GameState::new();
        gs.mode = initial_mode;
        app.insert_resource(GlobalState(gs));
        app.insert_resource(ButtonInput::<MouseButton>::default());
        app.insert_resource::<Time>(Time::default());
        app.insert_resource(PendingRecruitmentContext::default());
        app.insert_resource(GameContent::new(db));
        app.init_resource::<LockInteractionPending>();
        app.init_resource::<crate::game::systems::ui::GameLog>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<InitiateRestEvent>();
        app.add_systems(Update, handle_input);
        app
    }

    /// Build a `ContentDatabase` with a single merchant NPC ("merchant_tom").
    fn merchant_db() -> ContentDatabase {
        let mut db = ContentDatabase::new();
        let merchant = NpcDefinition::merchant("merchant_tom", "Tom the Merchant", "tom.png");
        db.npcs.add_npc(merchant).unwrap();
        db
    }

    /// Build a `ContentDatabase` with a single non-merchant NPC ("elder_bob").
    fn non_merchant_db() -> ContentDatabase {
        let mut db = ContentDatabase::new();
        let elder = NpcDefinition::new("elder_bob", "Elder Bob", "bob.png");
        db.npcs.add_npc(elder).unwrap();
        db
    }

    /// Build a `DialogueState` with the given `speaker_npc_id`.
    fn dialogue_state_for(npc_id: &str) -> crate::application::dialogue::DialogueState {
        crate::application::dialogue::DialogueState::start(1, 1, None, Some(npc_id.to_string()))
    }

    /// Pressing `I` while in `GameMode::Dialogue` with a merchant NPC must
    /// transition the game mode to `GameMode::MerchantInventory`.
    #[test]
    fn test_handle_input_i_in_dialogue_with_merchant_opens_merchant_inventory() {
        let db = merchant_db();
        let initial_mode =
            crate::application::GameMode::Dialogue(dialogue_state_for("merchant_tom"));
        let mut app = build_dialogue_input_app(db, initial_mode);

        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::KeyI);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(
                gs.0.mode,
                crate::application::GameMode::MerchantInventory(_)
            ),
            "pressing I in Dialogue with a merchant must open MerchantInventory, got {:?}",
            gs.0.mode
        );
    }

    /// Pressing `I` while in `GameMode::Dialogue` with a non-merchant NPC must
    /// leave the mode unchanged (still `Dialogue`).
    #[test]
    fn test_handle_input_i_in_dialogue_with_non_merchant_does_not_open_inventory() {
        let db = non_merchant_db();
        let initial_mode = crate::application::GameMode::Dialogue(dialogue_state_for("elder_bob"));
        let mut app = build_dialogue_input_app(db, initial_mode);

        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::KeyI);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, crate::application::GameMode::Dialogue(_)),
            "pressing I in Dialogue with a non-merchant must not change mode, got {:?}",
            gs.0.mode
        );
    }

    /// Pressing `I` while in `GameMode::Dialogue` with `npc_id: None` must
    /// do nothing — mode stays `Dialogue`.
    #[test]
    fn test_handle_input_i_in_dialogue_with_no_npc_id_does_nothing() {
        let db = ContentDatabase::new();
        // DialogueState with speaker_npc_id = None
        let dialogue_state = crate::application::dialogue::DialogueState::start(1, 1, None, None);
        let initial_mode = crate::application::GameMode::Dialogue(dialogue_state);
        let mut app = build_dialogue_input_app(db, initial_mode);

        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::KeyI);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, crate::application::GameMode::Dialogue(_)),
            "pressing I in Dialogue with no npc_id must not change mode, got {:?}",
            gs.0.mode
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controls_config_default_cooldown() {
        let config = ControlsConfig::default();
        assert_eq!(config.movement_cooldown, 0.2);
    }

    #[test]
    fn test_controls_config_validation_valid() {
        let config = ControlsConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_controls_config_validation_negative_cooldown() {
        let config = ControlsConfig {
            move_forward: vec!["W".to_string()],
            move_back: vec!["S".to_string()],
            turn_left: vec!["A".to_string()],
            turn_right: vec!["D".to_string()],
            interact: vec!["Space".to_string()],
            menu: vec!["Escape".to_string()],
            inventory: vec!["I".to_string()],
            rest: vec!["R".to_string()],
            automap: vec!["M".to_string()],
            movement_cooldown: -0.1,
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-negative"));
    }

    #[test]
    fn test_input_plugin_creation() {
        let config = ControlsConfig::default();
        let plugin = InputPlugin::new(config.clone());
        assert_eq!(plugin.config, config);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use bevy::prelude::{App, ButtonInput, KeyCode, Time, Update};

    /// Helper: build a minimal Bevy `App` wired up with all resources and
    /// message channels that `handle_input` requires.
    fn build_input_app() -> App {
        let mut app = App::new();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        let cfg = ControlsConfig::default();
        let km = KeyMap::from_controls_config(&cfg);
        app.insert_resource(InputConfigResource {
            controls: cfg,
            key_map: km,
        });
        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.insert_resource(ButtonInput::<MouseButton>::default());
        app.insert_resource::<Time>(Time::default());
        app.insert_resource(PendingRecruitmentContext::default());
        app.init_resource::<LockInteractionPending>();
        app.init_resource::<crate::game::systems::ui::GameLog>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<InitiateRestEvent>();
        app.add_systems(Update, handle_input);
        app
    }

    /// Integration-style test: simulate pressing ESC via `ButtonInput` and ensure the
    /// input system toggles the in-game menu open and closed.
    #[test]
    fn test_escape_opens_and_closes_menu_via_button_input() {
        // Build a minimal app and register the input system under test.
        let mut app = App::new();

        // Insert required resources: keyboard/mouse input, config, global state, and time.
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());

        let cfg = ControlsConfig::default();
        let km = KeyMap::from_controls_config(&cfg);
        app.insert_resource(InputConfigResource {
            controls: cfg,
            key_map: km,
        });

        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.insert_resource::<Time>(Time::default());
        app.insert_resource(PendingRecruitmentContext::default());
        app.init_resource::<LockInteractionPending>();
        app.init_resource::<crate::game::systems::ui::GameLog>();

        // Register message channels the input system depends on.
        app.add_message::<MapEventTriggered>();
        app.add_message::<InitiateRestEvent>();

        // Add the handle_input system (the system under test)
        app.add_systems(Update, handle_input);

        // Press Escape - should open the menu
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::Escape);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(matches!(gs.0.mode, crate::application::GameMode::Menu(_)));

        // Press Escape again - should close the menu and resume previous mode
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::Escape);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(matches!(
            gs.0.mode,
            crate::application::GameMode::Exploration
        ));
    }

    #[test]
    fn test_escape_opens_after_movement() {
        use bevy::prelude::*;

        let mut app = App::new();

        // Basic resources the input system expects
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());
        let cfg = ControlsConfig::default();
        app.insert_resource(InputConfigResource {
            controls: cfg.clone(),
            key_map: KeyMap::from_controls_config(&cfg),
        });
        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.insert_resource::<Time>(Time::default());
        app.insert_resource(PendingRecruitmentContext::default());
        app.init_resource::<LockInteractionPending>();
        app.init_resource::<crate::game::systems::ui::GameLog>();

        // Register messages used by input system
        app.add_message::<MapEventTriggered>();
        app.add_message::<InitiateRestEvent>();

        // Add just the input system (we want to simulate input frames)
        app.add_systems(Update, handle_input);

        // Frame 1: press MoveForward
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::ArrowUp);
        }
        app.update();

        // Frame 2: release MoveForward and press Escape
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.release(KeyCode::ArrowUp);
            btn.press(KeyCode::Escape);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(matches!(gs.0.mode, crate::application::GameMode::Menu(_)));
    }

    #[test]
    fn test_escape_opens_when_move_and_menu_pressed_simultaneously() {
        use bevy::prelude::*;

        let mut app = App::new();

        // Basic resources the input system expects
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());
        let cfg = ControlsConfig::default();
        app.insert_resource(InputConfigResource {
            controls: cfg.clone(),
            key_map: KeyMap::from_controls_config(&cfg),
        });
        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.insert_resource::<Time>(Time::default());
        app.insert_resource(PendingRecruitmentContext::default());
        app.init_resource::<LockInteractionPending>();
        app.init_resource::<crate::game::systems::ui::GameLog>();

        // Register messages used by input system
        app.add_message::<MapEventTriggered>();
        app.add_message::<InitiateRestEvent>();

        // Add the input system so frames process input
        app.add_systems(Update, handle_input);

        // Single frame: press MoveForward and Menu at the same time
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::ArrowUp);
            btn.press(KeyCode::Escape);
        }
        app.update();
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(matches!(gs.0.mode, crate::application::GameMode::Menu(_)));
    }

    /// Pressing `KeyCode::KeyI` in `GameMode::Exploration` must transition the
    /// mode to `GameMode::Inventory(_)`.
    #[test]
    fn test_handle_input_i_opens_inventory() {
        let mut app = build_input_app();

        // Press "I" – should open inventory
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::KeyI);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, crate::application::GameMode::Inventory(_)),
            "pressing I in Exploration must open the inventory"
        );
    }

    #[test]
    fn test_world_click_npc_triggers_dialogue() {
        let mut app = build_input_app();

        let mut map =
            crate::domain::world::Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let party_pos = crate::domain::types::Position::new(5, 5);
        let npc_pos = crate::domain::types::Position::new(5, 4);
        map.npc_placements.push(crate::domain::world::NpcPlacement {
            npc_id: "test_npc".to_string(),
            position: npc_pos,
            facing: None,
            dialogue_override: None,
        });

        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.world.add_map(map);
            gs.0.world.set_current_map(1);
            gs.0.world.set_party_position(party_pos);
            gs.0.world.party_facing = crate::domain::types::Direction::North;
            gs.0.mode = crate::application::GameMode::Exploration;
        }

        let mut window = Window::default();
        window.resolution.set_physical_resolution(900, 600);
        window.set_cursor_position(Some(Vec2::new(450.0, 300.0)));
        app.world_mut().spawn((window, PrimaryWindow));

        let mut mouse = ButtonInput::<MouseButton>::default();
        mouse.press(MouseButton::Left);
        app.insert_resource(mouse);

        app.update();

        let events = app.world().resource::<Messages<MapEventTriggered>>();
        let mut reader = events.get_cursor();
        let triggered_events: Vec<_> = reader.read(events).collect();

        assert_eq!(
            triggered_events.len(),
            1,
            "Expected exactly one interaction event"
        );
        match &triggered_events[0].event {
            MapEvent::NpcDialogue { npc_id, .. } => {
                assert_eq!(npc_id, "test_npc");
            }
            other => panic!("Expected NpcDialogue event, got {:?}", other),
        }
        assert_eq!(triggered_events[0].position, npc_pos);
    }

    #[test]
    fn test_world_click_blocked_outside_exploration_mode() {
        let mut app = build_input_app();

        let mut map =
            crate::domain::world::Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let party_pos = crate::domain::types::Position::new(5, 5);
        let npc_pos = crate::domain::types::Position::new(5, 4);
        map.npc_placements.push(crate::domain::world::NpcPlacement {
            npc_id: "test_npc".to_string(),
            position: npc_pos,
            facing: None,
            dialogue_override: None,
        });

        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.world.add_map(map);
            gs.0.world.set_current_map(1);
            gs.0.world.set_party_position(party_pos);
            gs.0.world.party_facing = crate::domain::types::Direction::North;
            gs.0.mode = crate::application::GameMode::Menu(
                crate::application::menu::MenuState::new(crate::application::GameMode::Exploration),
            );
        }

        let mut window = Window::default();
        window.resolution.set_physical_resolution(900, 600);
        window.set_cursor_position(Some(Vec2::new(450.0, 300.0)));
        app.world_mut().spawn((window, PrimaryWindow));

        let mut mouse = ButtonInput::<MouseButton>::default();
        mouse.press(MouseButton::Left);
        app.insert_resource(mouse);

        app.update();

        let events = app.world().resource::<Messages<MapEventTriggered>>();
        let mut reader = events.get_cursor();
        let triggered_events: Vec<_> = reader.read(events).collect();

        assert!(
            triggered_events.is_empty(),
            "Mouse world click must not trigger outside Exploration mode"
        );
    }

    /// Pressing `KeyCode::KeyM` in `GameMode::Exploration` must open the
    /// full-screen automap overlay.
    #[test]
    fn test_gamemode_automap_toggle() {
        let mut app = build_input_app();

        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::KeyM);
        }
        app.update();

        {
            let gs = app.world().resource::<GlobalState>();
            assert!(
                matches!(gs.0.mode, crate::application::GameMode::Automap),
                "pressing M in Exploration must open Automap"
            );
        }

        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.release(KeyCode::KeyM);
            btn.press(KeyCode::KeyM);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, crate::application::GameMode::Exploration),
            "pressing M again in Automap must return to Exploration"
        );
    }

    /// Pressing `Escape` while in `GameMode::Automap` must close the overlay
    /// and return to exploration instead of opening the menu.
    #[test]
    fn test_gamemode_automap_escape_closes() {
        let mut app = build_input_app();

        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.mode = crate::application::GameMode::Automap;
        }

        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::Escape);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, crate::application::GameMode::Exploration),
            "pressing Escape in Automap must return to Exploration"
        );
    }

    /// Pressing `KeyCode::KeyI` while already in `GameMode::Inventory` must
    /// restore the previous mode (toggle off).
    #[test]
    fn test_handle_input_i_closes_inventory() {
        let mut app = build_input_app();

        // Frame 1: open inventory
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::KeyI);
        }
        app.update();

        {
            let gs = app.world().resource::<GlobalState>();
            assert!(
                matches!(gs.0.mode, crate::application::GameMode::Inventory(_)),
                "mode must be Inventory after first I press"
            );
        }

        // Frame 2: release and press again to close
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.release(KeyCode::KeyI);
            btn.press(KeyCode::KeyI);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, crate::application::GameMode::Exploration),
            "pressing I again must close the inventory and restore Exploration mode"
        );
    }

    /// Pressing `KeyCode::KeyI` while in `GameMode::Menu` must NOT open the
    /// inventory — the I key is ignored when the menu is active.
    ///
    /// This test manually sets the game mode to `Menu` without using the
    /// keyboard so that no stale `just_pressed` state leaks between frames.
    #[test]
    fn test_handle_input_i_ignored_in_menu_mode() {
        let mut app = build_input_app();

        // Place the game state directly into Menu mode without pressing ESC,
        // so there is no stale just_pressed(Escape) entry that would re-toggle
        // the menu when update() runs.
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.enter_menu();
        }

        // Verify we are in Menu mode before pressing I.
        {
            let gs = app.world().resource::<GlobalState>();
            assert!(
                matches!(gs.0.mode, crate::application::GameMode::Menu(_)),
                "mode must be Menu before pressing I"
            );
        }

        // Press I — must stay in Menu because inventory is blocked while in menu.
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::KeyI);
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, crate::application::GameMode::Menu(_)),
            "pressing I while in Menu must not switch to Inventory"
        );
    }

    /// Pressing `R` in `GameMode::Exploration` must open the rest-duration
    /// menu (`GameMode::RestMenu`).  No `InitiateRestEvent` is fired at this
    /// point — that happens when the player selects a duration from the menu.
    #[test]
    fn test_handle_input_r_in_exploration_fires_initiate_rest_event() {
        let mut app = build_input_app();

        // Confirm we start in Exploration mode.
        {
            let gs = app.world().resource::<GlobalState>();
            assert!(
                matches!(gs.0.mode, crate::application::GameMode::Exploration),
                "must start in Exploration mode"
            );
        }

        // Press R — should open the rest menu.
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::KeyR);
        }
        app.update();

        // Mode must have transitioned to RestMenu.
        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, crate::application::GameMode::RestMenu),
            "R in Exploration must open RestMenu; got {:?}",
            gs.0.mode
        );

        // No InitiateRestEvent must have been fired yet — the player still
        // needs to pick a duration from the menu.
        let events = app.world().resource::<Messages<InitiateRestEvent>>();
        let mut reader = events.get_cursor();
        let fired: Vec<&InitiateRestEvent> = reader.read(events).collect();
        assert!(
            fired.is_empty(),
            "R must not fire InitiateRestEvent before duration is chosen; got {:?}",
            fired
        );
    }

    /// Pressing `R` while in `GameMode::Menu` must NOT open the rest menu
    /// and must NOT fire `InitiateRestEvent`.
    #[test]
    fn test_handle_input_r_ignored_in_menu_mode() {
        let mut app = build_input_app();

        // Put game state into Menu mode directly.
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.enter_menu();
        }

        // Press R.
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::KeyR);
        }
        app.update();

        // Mode must still be Menu — R is ignored outside Exploration.
        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, crate::application::GameMode::Menu(_)),
            "R in Menu mode must not change mode; got {:?}",
            gs.0.mode
        );

        // No InitiateRestEvent must have been sent.
        let events = app.world().resource::<Messages<InitiateRestEvent>>();
        let mut reader = events.get_cursor();
        let fired: Vec<&InitiateRestEvent> = reader.read(events).collect();
        assert!(
            fired.is_empty(),
            "R in Menu mode must not fire InitiateRestEvent; got {:?}",
            fired
        );
    }

    /// Pressing `R` while in `GameMode::Inventory` must NOT open the rest menu
    /// and must NOT fire `InitiateRestEvent`.
    #[test]
    fn test_handle_input_r_ignored_in_inventory_mode() {
        let mut app = build_input_app();

        // Open inventory first (without pressing I, to avoid stale key state).
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.enter_inventory();
        }

        // Press R.
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::KeyR);
        }
        app.update();

        // Mode must still be Inventory — R is ignored outside Exploration.
        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, crate::application::GameMode::Inventory(_)),
            "R in Inventory mode must not change mode; got {:?}",
            gs.0.mode
        );

        // No InitiateRestEvent must have been sent.
        let events = app.world().resource::<Messages<InitiateRestEvent>>();
        let mut reader = events.get_cursor();
        let fired: Vec<&InitiateRestEvent> = reader.read(events).collect();
        assert!(
            fired.is_empty(),
            "R in Inventory mode must not fire InitiateRestEvent; got {:?}",
            fired
        );
    }

    /// Pressing `R` while in `GameMode::Combat` must NOT open the rest menu
    /// and must NOT fire `InitiateRestEvent`.
    #[test]
    fn test_handle_input_r_ignored_in_combat_mode() {
        let mut app = build_input_app();

        // Enter combat mode directly.
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            let hero = crate::domain::character::Character::new(
                "Rest Guard Hero".to_string(),
                "human".to_string(),
                "knight".to_string(),
                crate::domain::character::Sex::Male,
                crate::domain::character::Alignment::Good,
            );
            gs.0.party.add_member(hero).unwrap();
            gs.0.enter_combat();
        }

        // Press R.
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::KeyR);
        }
        app.update();

        // Mode must still be Combat — R is ignored outside Exploration.
        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, crate::application::GameMode::Combat(_)),
            "R in Combat mode must not change mode; got {:?}",
            gs.0.mode
        );

        // No InitiateRestEvent must have been sent.
        let events = app.world().resource::<Messages<InitiateRestEvent>>();
        let mut reader = events.get_cursor();
        let fired: Vec<&InitiateRestEvent> = reader.read(events).collect();
        assert!(
            fired.is_empty(),
            "R in Combat mode must not fire InitiateRestEvent; got {:?}",
            fired
        );
    }

    /// Pressing `R` in Exploration opens RestMenu.  Pressing `R` again while
    /// in RestMenu must be ignored (R only acts in Exploration mode).
    #[test]
    fn test_handle_input_r_in_exploration_two_frames_two_events() {
        let mut app = build_input_app();

        // Frame 1: press R in Exploration — opens RestMenu.
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.press(KeyCode::KeyR);
        }
        app.update();

        {
            let gs = app.world().resource::<GlobalState>();
            assert!(
                matches!(gs.0.mode, crate::application::GameMode::RestMenu),
                "R in Exploration must open RestMenu on frame 1; got {:?}",
                gs.0.mode
            );
        }

        // Frame 2: release then press R again — now in RestMenu, must be ignored.
        {
            let mut btn = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            btn.release(KeyCode::KeyR);
            btn.press(KeyCode::KeyR);
        }
        app.update();

        // Still in RestMenu — R has no effect while the menu is open.
        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, crate::application::GameMode::RestMenu),
            "R in RestMenu must be ignored; mode should stay RestMenu; got {:?}",
            gs.0.mode
        );

        // No InitiateRestEvent should have fired at any point.
        let events = app.world().resource::<Messages<InitiateRestEvent>>();
        let mut reader = events.get_cursor();
        let fired: Vec<&InitiateRestEvent> = reader.read(events).collect();
        assert!(
            fired.is_empty(),
            "R must not fire InitiateRestEvent before duration is chosen; got {:?}",
            fired
        );
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;

    /// Test that adjacent positions are correctly identified for interaction purposes.
    /// This test verifies that the helper function identifies all 8 surrounding tiles.
    #[test]
    fn test_npc_interaction_adjacent_positions() {
        // Arrange
        let center = crate::domain::types::Position::new(5, 5);
        let adjacent = get_adjacent_positions(center);

        assert!(adjacent.contains(&crate::domain::types::Position::new(5, 4))); // North
        assert!(adjacent.contains(&crate::domain::types::Position::new(6, 4))); // NorthEast
        assert!(adjacent.contains(&crate::domain::types::Position::new(6, 5))); // East
        assert!(adjacent.contains(&crate::domain::types::Position::new(6, 6))); // SouthEast
        assert!(adjacent.contains(&crate::domain::types::Position::new(5, 6))); // South
        assert!(adjacent.contains(&crate::domain::types::Position::new(4, 6))); // SouthWest
        assert!(adjacent.contains(&crate::domain::types::Position::new(4, 5))); // West
        assert!(adjacent.contains(&crate::domain::types::Position::new(4, 4))); // NorthWest
    }

    /// Test that sign interaction detects signs in adjacent positions.
    /// Validates that map events are properly stored and retrievable.
    #[test]
    fn test_sign_interaction_event_storage() {
        // Arrange
        let mut map =
            crate::domain::world::Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);

        let sign_pos = crate::domain::types::Position::new(5, 4);
        map.add_event(
            sign_pos,
            MapEvent::Sign {
                name: "TestSign".to_string(),
                description: "This is a test sign".to_string(),
                text: "You found it!".to_string(),
                time_condition: None,
                facing: None,
            },
        );

        // Act
        let event = map.get_event(sign_pos);

        // Assert
        assert!(event.is_some());
        assert!(matches!(event, Some(MapEvent::Sign { .. })));
    }

    /// Test that teleport events are properly stored and retrievable.
    /// Validates event data persistence in the map.
    #[test]
    fn test_teleport_interaction_event_storage() {
        // Arrange
        let mut map =
            crate::domain::world::Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);

        let teleport_pos = crate::domain::types::Position::new(5, 4);
        map.add_event(
            teleport_pos,
            MapEvent::Teleport {
                name: "TestPortal".to_string(),
                description: "Portal to destination".to_string(),
                destination: crate::domain::types::Position::new(2, 2),
                map_id: 1,
            },
        );

        // Act
        let event = map.get_event(teleport_pos);

        // Assert
        assert!(event.is_some());
        assert!(matches!(event, Some(MapEvent::Teleport { .. })));
    }

    /// Test that NPC placements are properly stored and retrievable.
    /// Validates the NPC data structure and storage mechanisms.
    #[test]
    fn test_npc_interaction_placement_storage() {
        // Arrange
        let mut map =
            crate::domain::world::Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);

        let npc_pos = crate::domain::types::Position::new(5, 4);
        map.npc_placements
            .push(crate::domain::world::NpcPlacement::new("test_npc", npc_pos));

        // Act
        let npc = map
            .npc_placements
            .iter()
            .find(|npc| npc.position == npc_pos);

        // Assert
        assert!(npc.is_some());
        assert_eq!(npc.unwrap().npc_id, "test_npc");
        assert_eq!(npc.unwrap().position, npc_pos);
    }

    /// Test that recruitable character events are properly stored and retrievable.
    /// Validates that map events for recruitables are correctly managed.
    #[test]
    fn test_recruitable_character_event_storage() {
        // Arrange
        let mut map =
            crate::domain::world::Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);

        let recruit_pos = crate::domain::types::Position::new(5, 4);
        map.add_event(
            recruit_pos,
            MapEvent::RecruitableCharacter {
                name: "TestRecruit".to_string(),
                description: "A recruitable character".to_string(),
                character_id: "hero_01".to_string(),
                dialogue_id: None,
                time_condition: None,
                facing: None,
            },
        );

        // Act
        let event = map.get_event(recruit_pos);

        // Assert
        assert!(event.is_some());
        assert!(matches!(event, Some(MapEvent::RecruitableCharacter { .. })));
        if let Some(MapEvent::RecruitableCharacter {
            character_id, name, ..
        }) = event
        {
            assert_eq!(character_id, "hero_01");
            assert_eq!(name, "TestRecruit");
        }
    }

    /// Test that encounter events are properly stored and retrievable.
    /// Validates that encounter interaction can resolve map event data.
    #[test]
    fn test_encounter_event_storage() {
        // Arrange
        let mut map =
            crate::domain::world::Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);

        let encounter_pos = crate::domain::types::Position::new(5, 4);
        map.add_event(
            encounter_pos,
            MapEvent::Encounter {
                name: "Skeleton".to_string(),
                description: "A rattling skeleton".to_string(),
                monster_group: vec![1],
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
            },
        );

        // Act
        let event = map.get_event(encounter_pos);

        // Assert
        assert!(event.is_some());
        assert!(matches!(event, Some(MapEvent::Encounter { .. })));
    }
}

/// T1-8: Verify that `handle_input` silently ignores all movement input when
/// `GameMode::Combat` is active.  The party position must remain unchanged after
/// pressing the forward-movement key.
#[cfg(test)]
mod inventory_guard_tests {
    use super::*;
    use bevy::prelude::{App, ButtonInput, KeyCode, Update};

    /// Movement keys must not alter the party position while
    /// `GameMode::Inventory` is active.
    #[test]
    fn test_movement_blocked_in_inventory_mode() {
        let mut app = App::new();

        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());

        let cfg = ControlsConfig {
            movement_cooldown: 0.0,
            ..ControlsConfig::default()
        };
        let key_map = KeyMap::from_controls_config(&cfg);
        app.insert_resource(InputConfigResource {
            controls: cfg,
            key_map,
        });

        // Build a game state and place it in Inventory mode.
        let mut gs = crate::application::GameState::new();
        gs.enter_inventory();
        let original_position = gs.world.party_position;

        app.insert_resource(GlobalState(gs));
        app.insert_resource::<bevy::time::Time>(bevy::time::Time::default());
        app.insert_resource(PendingRecruitmentContext::default());
        app.init_resource::<crate::game::systems::ui::GameLog>();
        app.init_resource::<LockInteractionPending>();

        // Register message channels that handle_input depends on.
        app.add_message::<MapEventTriggered>();
        app.add_message::<InitiateRestEvent>();

        // Register the system under test.
        app.add_systems(Update, handle_input);

        // Press MoveForward (W key per default config).
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(KeyCode::KeyW);
        }
        app.update();

        let gs_after = app.world().resource::<GlobalState>();
        assert_eq!(
            gs_after.0.world.party_position, original_position,
            "Party must not move while GameMode::Inventory is active"
        );
    }

    /// Turn-left input must not alter party facing while inventory is open.
    #[test]
    fn test_turn_blocked_in_inventory_mode() {
        let mut app = App::new();

        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());

        let cfg = ControlsConfig {
            movement_cooldown: 0.0,
            ..ControlsConfig::default()
        };
        let key_map = KeyMap::from_controls_config(&cfg);
        app.insert_resource(InputConfigResource {
            controls: cfg,
            key_map,
        });

        let mut gs = crate::application::GameState::new();
        let original_facing = gs.world.party_facing;
        gs.enter_inventory();

        app.insert_resource(GlobalState(gs));
        app.insert_resource::<bevy::time::Time>(bevy::time::Time::default());
        app.insert_resource(PendingRecruitmentContext::default());
        app.init_resource::<crate::game::systems::ui::GameLog>();
        app.init_resource::<LockInteractionPending>();

        app.add_message::<MapEventTriggered>();
        app.add_message::<InitiateRestEvent>();

        app.add_systems(Update, handle_input);

        // Press TurnLeft (A key per default config).
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(KeyCode::KeyA);
        }
        app.update();

        let gs_after = app.world().resource::<GlobalState>();
        assert_eq!(
            gs_after.0.world.party_facing, original_facing,
            "Party facing must not change while GameMode::Inventory is active"
        );
    }
}

#[cfg(test)]
mod combat_guard_tests {
    use super::*;
    use bevy::prelude::{App, ButtonInput, KeyCode, Update};

    #[test]
    fn test_movement_blocked_in_combat_mode() {
        let mut app = App::new();

        // Minimal resources required by handle_input.
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());

        let cfg = ControlsConfig::default();
        let key_map = KeyMap::from_controls_config(&cfg);
        app.insert_resource(InputConfigResource {
            controls: cfg,
            key_map,
        });

        // Build a game state in Combat mode.
        let mut gs = crate::application::GameState::new();
        let hero = crate::domain::character::Character::new(
            "Guard Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            crate::domain::character::Sex::Male,
            crate::domain::character::Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        // enter_combat sets GameMode::Combat so the guard in handle_input fires.
        gs.enter_combat();
        let original_position = gs.world.party_position;

        app.insert_resource(GlobalState(gs));
        app.insert_resource::<bevy::time::Time>(bevy::time::Time::default());
        app.insert_resource(PendingRecruitmentContext::default());
        app.init_resource::<crate::game::systems::ui::GameLog>();
        app.init_resource::<LockInteractionPending>();

        // Register message channels that handle_input depends on.
        app.add_message::<MapEventTriggered>();
        app.add_message::<InitiateRestEvent>();

        app.add_systems(Update, handle_input);

        // Press MoveForward (W key per default config).
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(KeyCode::KeyW);
        }
        app.update();

        let gs_after = app.world().resource::<GlobalState>();
        assert_eq!(
            gs_after.0.world.party_position, original_position,
            "Party must not move while GameMode::Combat is active"
        );
    }

    #[test]
    fn test_victory_overlay_dismissed_after_party_moves() {
        let mut app = App::new();

        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());

        let cfg = ControlsConfig {
            movement_cooldown: 0.0,
            ..ControlsConfig::default()
        };
        let turn_left_key =
            parse_key_code(&cfg.turn_left[0]).expect("invalid default turn_left key");
        let key_map = KeyMap::from_controls_config(&cfg);
        app.insert_resource(InputConfigResource {
            controls: cfg,
            key_map,
        });

        // Build an exploration game state.
        let gs = crate::application::GameState::new();
        let original_facing = gs.world.party_facing;

        app.insert_resource(GlobalState(gs));
        app.insert_resource::<bevy::time::Time>(bevy::time::Time::default());
        app.insert_resource(PendingRecruitmentContext::default());
        app.init_resource::<crate::game::systems::ui::GameLog>();
        app.init_resource::<LockInteractionPending>();

        app.add_message::<MapEventTriggered>();
        app.add_message::<InitiateRestEvent>();

        // Spawn a victory overlay marker to verify cleanup behavior.
        app.world_mut()
            .spawn(crate::game::systems::combat::VictorySummaryRoot);

        app.add_systems(Update, handle_input);

        // Turn left (movement control) to trigger post-combat overlay dismissal.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(turn_left_key);
        }
        app.update();

        let gs_after = app.world().resource::<GlobalState>();
        assert_ne!(
            gs_after.0.world.party_facing, original_facing,
            "Party facing should change after turn input"
        );

        let mut overlay_query = app
            .world_mut()
            .query_filtered::<Entity, With<crate::game::systems::combat::VictorySummaryRoot>>();
        assert_eq!(
            overlay_query.iter(app.world()).count(),
            0,
            "Victory overlay must be dismissed after movement"
        );
    }
}

#[cfg(test)]
mod door_interaction_tests {
    use super::*;
    use crate::domain::world::Map;
    use crate::game::components::furniture::{DoorState, FurnitureEntity};
    use crate::game::systems::map::TileCoord;
    use bevy::prelude::{App, ButtonInput, Entity, KeyCode, Transform, Update};

    /// Helper: build a minimal app wired for furniture-door interaction tests.
    ///
    /// World: 10×10 map, party at (5, 5).  Party facing defaults to North,
    /// so `world.position_ahead()` → (5, 4).
    fn build_door_test_app() -> App {
        let mut app = App::new();

        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());

        // Zero cooldown so input fires on the first update.
        let cfg = ControlsConfig {
            movement_cooldown: 0.0,
            ..ControlsConfig::default()
        };
        let key_map = KeyMap::from_controls_config(&cfg);
        app.insert_resource(InputConfigResource {
            controls: cfg,
            key_map,
        });

        let mut gs = crate::application::GameState::new();
        let map = Map::new(1, "DoorTestMap".to_string(), "Test".to_string(), 10, 10);
        gs.world.add_map(map);
        gs.world.set_current_map(1);
        gs.world
            .set_party_position(crate::domain::types::Position::new(5, 5));
        // Default party_facing is North → position_ahead() == (5, 4).

        app.insert_resource(GlobalState(gs));
        app.insert_resource::<bevy::time::Time>(bevy::time::Time::default());
        app.insert_resource(PendingRecruitmentContext::default());
        app.init_resource::<crate::game::systems::ui::GameLog>();
        app.init_resource::<LockInteractionPending>();

        app.add_message::<MapEventTriggered>();
        app.add_message::<InitiateRestEvent>();

        app.add_systems(Update, handle_input);
        app
    }

    /// Spawn a furniture door entity at `position` with the given locked state.
    /// Returns the spawned entity ID.
    fn spawn_door_entity(
        app: &mut App,
        position: crate::domain::types::Position,
        is_locked: bool,
    ) -> Entity {
        app.world_mut()
            .spawn((
                FurnitureEntity::new(crate::domain::world::FurnitureType::Door, !is_locked),
                DoorState::new(is_locked, 0.0),
                Transform::default(),
                TileCoord(position),
            ))
            .id()
    }

    /// Resolve the interact `KeyCode` from the default `ControlsConfig`.
    fn interact_key() -> KeyCode {
        let cfg = ControlsConfig::default();
        parse_key_code(&cfg.interact[0]).expect("default interact key must be parseable")
    }

    /// Resolve the move-forward `KeyCode` from the default `ControlsConfig`.
    fn move_forward_key() -> KeyCode {
        let cfg = ControlsConfig::default();
        parse_key_code(&cfg.move_forward[0]).expect("default move_forward key must be parseable")
    }

    // ── Phase 3 Unit-style tests (pure DoorState logic) ───────────────────

    /// `DoorState::new(false, 0.0)` produces a closed, unlocked door with no key.
    #[test]
    fn test_door_state_component_default_values() {
        let door = DoorState::default();
        assert!(!door.is_open);
        assert!(!door.is_locked);
        assert!(door.key_item_id.is_none());
        assert_eq!(door.base_rotation_y, 0.0);
    }

    /// Open angle is base + π/2; closed angle restores base.
    #[test]
    fn test_door_state_rotation_angles() {
        let base = std::f32::consts::PI;
        let door = DoorState::new(false, base);

        let open_angle = door.base_rotation_y + std::f32::consts::FRAC_PI_2;
        let closed_angle = door.base_rotation_y;
        assert!((open_angle - (base + std::f32::consts::FRAC_PI_2)).abs() < 1e-6);
        assert!((closed_angle - base).abs() < 1e-6);
    }

    // ── Phase 3 Integration tests (Bevy headless App) ─────────────────────

    /// Pressing interact on an unlocked furniture door opens it.
    #[test]
    fn test_furniture_door_opens_on_interact() {
        let mut app = build_door_test_app();

        // Door directly north of the party (= position_ahead when facing North).
        let door_pos = crate::domain::types::Position::new(5, 4);
        let door_entity = spawn_door_entity(&mut app, door_pos, false);

        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(interact_key());
        }
        app.update();

        let door_state = app
            .world()
            .entity(door_entity)
            .get::<DoorState>()
            .expect("DoorState must be on the door entity");
        assert!(
            door_state.is_open,
            "Furniture door must be open after interact"
        );
        assert!(!door_state.is_locked, "Unlocked door must remain unlocked");
    }

    /// Pressing interact a second time on an open furniture door closes it.
    #[test]
    fn test_furniture_door_closes_on_second_interact() {
        let mut app = build_door_test_app();
        let door_pos = crate::domain::types::Position::new(5, 4);
        let door_entity = spawn_door_entity(&mut app, door_pos, false);
        let key = interact_key();

        // First interact → open.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(key);
        }
        app.update();
        {
            let ds = app.world().entity(door_entity).get::<DoorState>().unwrap();
            assert!(ds.is_open, "Door should be open after first interact");
        }

        // Second interact → close.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(key);
        }
        app.update();

        let ds = app.world().entity(door_entity).get::<DoorState>().unwrap();
        assert!(!ds.is_open, "Door must be closed after second interact");
    }

    /// Interacting with a locked door (no key) leaves it closed and locked.
    #[test]
    fn test_locked_furniture_door_stays_closed_without_key() {
        let mut app = build_door_test_app();
        let door_pos = crate::domain::types::Position::new(5, 4);
        let door_entity = spawn_door_entity(&mut app, door_pos, true); // locked

        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(interact_key());
        }
        app.update();

        let ds = app.world().entity(door_entity).get::<DoorState>().unwrap();
        assert!(
            !ds.is_open,
            "Locked door must stay closed when party has no key"
        );
        assert!(ds.is_locked, "Door must remain locked");
    }

    /// Locked door opens when a party member holds the matching key item.
    #[test]
    fn test_locked_door_opens_with_correct_key_in_inventory() {
        const KEY_ITEM: crate::domain::types::ItemId = 77;
        let mut app = build_door_test_app();

        // Give the party a hero carrying key item 77.
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            let mut hero = crate::domain::character::Character::new(
                "Keyholder".to_string(),
                "human".to_string(),
                "knight".to_string(),
                crate::domain::character::Sex::Male,
                crate::domain::character::Alignment::Good,
            );
            hero.inventory.add_item(KEY_ITEM, 1).unwrap();
            gs.0.party.add_member(hero).unwrap();
        }

        // Locked door that requires KEY_ITEM.
        let door_pos = crate::domain::types::Position::new(5, 4);
        let mut door_state = DoorState::new(true, 0.0);
        door_state.key_item_id = Some(KEY_ITEM);
        let door_entity = app
            .world_mut()
            .spawn((
                FurnitureEntity::new(crate::domain::world::FurnitureType::Door, true),
                door_state,
                Transform::default(),
                TileCoord(door_pos),
            ))
            .id();

        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(interact_key());
        }
        app.update();

        let ds = app.world().entity(door_entity).get::<DoorState>().unwrap();
        assert!(
            ds.is_open,
            "Locked door must open when party carries the key"
        );
        assert!(!ds.is_locked, "Door must be unlocked after key is used");
    }

    /// Opening a furniture door unblocks the tile in the world data.
    #[test]
    fn test_furniture_door_open_unblocks_tile() {
        let mut app = build_door_test_app();
        let door_pos = crate::domain::types::Position::new(5, 4);

        // Pre-block the tile to simulate an initially-closed door.
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            if let Some(map) = gs.0.world.get_current_map_mut() {
                if let Some(tile) = map.get_tile_mut(door_pos) {
                    tile.blocked = true;
                }
            }
        }

        spawn_door_entity(&mut app, door_pos, false);

        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(interact_key());
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        let tile =
            gs.0.world
                .get_current_map()
                .unwrap()
                .get_tile(door_pos)
                .unwrap();
        assert!(
            !tile.blocked,
            "Opening a furniture door must unblock the tile"
        );
    }

    /// Closing an open furniture door re-blocks the tile in the world data.
    #[test]
    fn test_furniture_door_close_reblocks_tile() {
        let mut app = build_door_test_app();
        let door_pos = crate::domain::types::Position::new(5, 4);

        // Spawn a door that starts open.
        let mut open_state = DoorState::new(false, 0.0);
        open_state.is_open = true;
        let door_entity = app
            .world_mut()
            .spawn((
                FurnitureEntity::new(crate::domain::world::FurnitureType::Door, false),
                open_state,
                Transform::default(),
                TileCoord(door_pos),
            ))
            .id();

        // Ensure tile is unblocked (matching the open state).
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            if let Some(map) = gs.0.world.get_current_map_mut() {
                if let Some(tile) = map.get_tile_mut(door_pos) {
                    tile.blocked = false;
                }
            }
        }

        // Interact → close the door.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(interact_key());
        }
        app.update();

        {
            let ds = app.world().entity(door_entity).get::<DoorState>().unwrap();
            assert!(
                !ds.is_open,
                "Door must be closed after interacting with open door"
            );
        }

        let gs = app.world().resource::<GlobalState>();
        let tile =
            gs.0.world
                .get_current_map()
                .unwrap()
                .get_tile(door_pos)
                .unwrap();
        assert!(
            tile.blocked,
            "Closing a furniture door must re-block the tile"
        );
    }

    /// A furniture door that is NOT in front of the party is unaffected by interact.
    #[test]
    fn test_door_not_opened_when_not_directly_ahead() {
        let mut app = build_door_test_app();

        // Door to the east — party faces North so this is off to the side.
        let door_pos = crate::domain::types::Position::new(6, 5);
        let door_entity = spawn_door_entity(&mut app, door_pos, false);

        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(interact_key());
        }
        app.update();

        let ds = app.world().entity(door_entity).get::<DoorState>().unwrap();
        assert!(
            !ds.is_open,
            "Door to the side must not be opened by a forward-facing interact"
        );
    }

    /// Moving forward into a locked (closed) furniture door is blocked at the
    /// input layer, surfacing `MovementError::DoorLocked` semantics.
    #[test]
    fn test_locked_furniture_door_blocks_forward_movement() {
        let mut app = build_door_test_app();

        let door_pos = crate::domain::types::Position::new(5, 4);
        // Spawn a locked, closed door with no key — permanently blocks movement.
        app.world_mut().spawn((
            FurnitureEntity::new(crate::domain::world::FurnitureType::Door, true),
            DoorState::new(true, 0.0),
            Transform::default(),
            TileCoord(door_pos),
        ));

        let original_position = {
            let gs = app.world().resource::<GlobalState>();
            gs.0.world.party_position
        };

        // Press move-forward.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(move_forward_key());
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.world.party_position, original_position,
            "Party must not move through a locked furniture door"
        );
    }

    /// An open (unlocked) furniture door does NOT block forward movement.
    #[test]
    fn test_open_furniture_door_allows_forward_movement() {
        let mut app = build_door_test_app();

        let door_pos = crate::domain::types::Position::new(5, 4);
        let mut open_state = DoorState::new(false, 0.0);
        open_state.is_open = true;
        app.world_mut().spawn((
            FurnitureEntity::new(crate::domain::world::FurnitureType::Door, false),
            open_state,
            Transform::default(),
            TileCoord(door_pos),
        ));

        // Ensure tile is unblocked (open door).
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            if let Some(map) = gs.0.world.get_current_map_mut() {
                if let Some(tile) = map.get_tile_mut(door_pos) {
                    tile.blocked = false;
                }
            }
        }

        // Press move-forward — should succeed.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(move_forward_key());
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.world.party_position, door_pos,
            "Party must be able to move through an open furniture door"
        );
    }
}

/// Integration tests for Phase 2: tile-based LockedDoor map-event interaction.
///
/// These tests exercise the E-key path added in Phase 2 that checks whether the
/// tile directly ahead has an associated `MapEvent::LockedDoor` and handles:
///
/// - Regular (non-locked) `WallType::Door` tiles open on E with no event.
/// - Locked door + correct key in party → door opens, key consumed.
/// - Locked door + no key in party → `LockInteractionPending` set.
/// - Locked door + wrong key in party → door stays locked, pending set.
#[cfg(test)]
mod locked_container_map_event_tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::world::{LockState, Map, MapEvent};
    use bevy::prelude::{App, ButtonInput, KeyCode, Update};

    const CONTAINER_KEY_ID: crate::domain::types::ItemId = 55;
    const CONTAINER_LOCK_ID: &str = "test_container_lock";

    fn container_pos() -> crate::domain::types::Position {
        crate::domain::types::Position::new(5, 4)
    }

    fn build_locked_container_app() -> App {
        let mut app = App::new();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());

        let cfg = ControlsConfig {
            movement_cooldown: 0.0,
            ..ControlsConfig::default()
        };
        let key_map = KeyMap::from_controls_config(&cfg);
        app.insert_resource(InputConfigResource {
            controls: cfg,
            key_map,
        });

        let mut gs = crate::application::GameState::new();
        let map = Map::new(
            1,
            "ContainerTestMap".to_string(),
            "Test".to_string(),
            10,
            10,
        );
        gs.world.add_map(map);
        gs.world.set_current_map(1);
        gs.world
            .set_party_position(crate::domain::types::Position::new(5, 5));

        let hero = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        app.insert_resource(GlobalState(gs));
        app.insert_resource::<bevy::time::Time>(bevy::time::Time::default());
        app.insert_resource(PendingRecruitmentContext::default());
        app.init_resource::<LockInteractionPending>();
        app.init_resource::<crate::game::systems::ui::GameLog>();

        app.add_message::<MapEventTriggered>();
        app.add_message::<InitiateRestEvent>();

        app.add_systems(Update, handle_input);
        app
    }

    fn add_locked_container_event(
        app: &mut App,
        key_item_id: Option<crate::domain::types::ItemId>,
    ) {
        let pos = container_pos();
        let mut gs = app.world_mut().resource_mut::<GlobalState>();
        if let Some(map) = gs.0.world.get_current_map_mut() {
            map.add_event(
                pos,
                MapEvent::LockedContainer {
                    name: "Iron Chest".to_string(),
                    lock_id: CONTAINER_LOCK_ID.to_string(),
                    key_item_id,
                    items: vec![],
                    initial_trap_chance: 0,
                },
            );
            map.lock_states.insert(
                CONTAINER_LOCK_ID.to_string(),
                LockState::new(CONTAINER_LOCK_ID),
            );
        }
    }

    fn give_key_to_party(app: &mut App, key_id: crate::domain::types::ItemId) {
        let mut gs = app.world_mut().resource_mut::<GlobalState>();
        gs.0.party.members[0]
            .inventory
            .add_item(key_id, 1)
            .expect("inventory must not be full for test key");
    }

    fn interact_key() -> KeyCode {
        KeyCode::KeyE
    }

    fn press_key(app: &mut App, key: KeyCode) {
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.press(key);
    }

    /// Pressing E on a `LockedContainer` with no key required sets
    /// `LockInteractionPending` with the container's lock_id.
    #[test]
    fn test_e_key_on_locked_container_without_key_sets_pending() {
        let mut app = build_locked_container_app();
        add_locked_container_event(&mut app, None);
        press_key(&mut app, interact_key());
        app.update();

        let pending = app.world().resource::<LockInteractionPending>();
        assert_eq!(
            pending.lock_id,
            Some(CONTAINER_LOCK_ID.to_string()),
            "LockInteractionPending.lock_id must be set when container has no key"
        );
        assert_eq!(
            pending.position,
            Some(container_pos()),
            "LockInteractionPending.position must point to the container tile"
        );

        let log = app.world().resource::<crate::game::systems::ui::GameLog>();
        assert!(
            log.messages.iter().any(|m| m.contains("locked")),
            "game log must contain a 'locked' message; got: {:?}",
            log.messages
        );
    }

    /// Pressing E on a `LockedContainer` with the correct key fires a
    /// `MapEventTriggered` Container event so handle_events can enter container
    /// inventory mode.
    #[test]
    fn test_e_key_on_locked_container_with_correct_key_fires_container_event() {
        let mut app = build_locked_container_app();
        add_locked_container_event(&mut app, Some(CONTAINER_KEY_ID));
        give_key_to_party(&mut app, CONTAINER_KEY_ID);
        press_key(&mut app, interact_key());
        app.update();

        // Key must be consumed.
        let gs = app.world().resource::<GlobalState>();
        assert!(
            !gs.0.party.members[0]
                .inventory
                .items
                .iter()
                .any(|s| s.item_id == CONTAINER_KEY_ID),
            "Key must be consumed after unlocking the container"
        );

        // Lock state must be unlocked.
        let lock_state =
            gs.0.world
                .get_current_map()
                .unwrap()
                .lock_states
                .get(CONTAINER_LOCK_ID)
                .unwrap();
        assert!(
            !lock_state.is_locked,
            "LockState must be unlocked after key success"
        );

        // LockInteractionPending must NOT be set.
        let pending = app.world().resource::<LockInteractionPending>();
        assert!(
            pending.lock_id.is_none(),
            "LockInteractionPending must be empty after key success"
        );
    }
}

#[cfg(test)]
mod locked_door_map_event_tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::world::{LockState, Map, MapEvent, WallType};
    use bevy::prelude::{App, ButtonInput, KeyCode, Update};

    /// Item ID used as the correct door key in these tests.
    const DOOR_KEY_ID: crate::domain::types::ItemId = 99;
    /// Item ID used as a wrong key (different from the door's required key).
    const WRONG_KEY_ID: crate::domain::types::ItemId = 100;
    /// Lock identifier string used in test map events.
    const LOCK_ID: &str = "test_door_lock";

    /// Position of the locked door (position_ahead from party at (5,5) facing North).
    fn door_pos() -> crate::domain::types::Position {
        crate::domain::types::Position::new(5, 4)
    }

    /// Build a minimal Bevy app for locked-door interaction tests.
    ///
    /// World: 10×10 map, party at (5, 5) facing North.
    /// `world.position_ahead()` → (5, 4).
    ///
    /// Resources registered:
    /// - `InputConfigResource` (zero cooldown)
    /// - `GlobalState` (with map + party placeholder)
    /// - `LockInteractionPending` (Phase 2 resource)
    /// - `GameLog` (for log message assertions)
    /// - `PendingRecruitmentContext`
    /// - Messages: `MapEventTriggered`, `InitiateRestEvent`
    fn build_locked_door_app() -> App {
        let mut app = App::new();

        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());

        let cfg = ControlsConfig {
            movement_cooldown: 0.0,
            ..ControlsConfig::default()
        };
        let key_map = KeyMap::from_controls_config(&cfg);
        app.insert_resource(InputConfigResource {
            controls: cfg,
            key_map,
        });

        // Set up a world with a 10×10 map. Party starts at (5,5) facing North.
        let mut gs = crate::application::GameState::new();
        let map = Map::new(
            1,
            "LockedDoorTestMap".to_string(),
            "Test".to_string(),
            10,
            10,
        );
        gs.world.add_map(map);
        gs.world.set_current_map(1);
        gs.world
            .set_party_position(crate::domain::types::Position::new(5, 5));

        // Add a default party member so inventory operations work.
        let hero = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        app.insert_resource(GlobalState(gs));
        app.insert_resource::<bevy::time::Time>(bevy::time::Time::default());
        app.insert_resource(PendingRecruitmentContext::default());
        app.init_resource::<LockInteractionPending>();
        app.init_resource::<crate::game::systems::ui::GameLog>();

        app.add_message::<MapEventTriggered>();
        app.add_message::<InitiateRestEvent>();

        app.add_systems(Update, handle_input);
        app
    }

    /// Set the tile at `door_pos()` to `WallType::Door` with `blocked = true`.
    fn place_door_tile(app: &mut App) {
        let pos = door_pos();
        let mut gs = app.world_mut().resource_mut::<GlobalState>();
        if let Some(map) = gs.0.world.get_current_map_mut() {
            if let Some(tile) = map.get_tile_mut(pos) {
                tile.wall_type = WallType::Door;
                tile.blocked = true;
            }
        }
    }

    /// Add a `MapEvent::LockedDoor` at `door_pos()` and insert the matching
    /// `LockState` (locked) into `map.lock_states`.
    fn add_locked_door_event(app: &mut App, key_item_id: Option<crate::domain::types::ItemId>) {
        let pos = door_pos();
        let mut gs = app.world_mut().resource_mut::<GlobalState>();
        if let Some(map) = gs.0.world.get_current_map_mut() {
            map.add_event(
                pos,
                MapEvent::LockedDoor {
                    name: "Test Locked Door".to_string(),
                    lock_id: LOCK_ID.to_string(),
                    key_item_id,
                    initial_trap_chance: 0,
                },
            );
            map.lock_states
                .insert(LOCK_ID.to_string(), LockState::new(LOCK_ID));
        }
    }

    /// Give the first party member a key with the given item ID.
    fn give_key_to_party(app: &mut App, key_id: crate::domain::types::ItemId) {
        let mut gs = app.world_mut().resource_mut::<GlobalState>();
        gs.0.party.members[0]
            .inventory
            .add_item(key_id, 1)
            .expect("inventory must not be full for test key");
    }

    /// Resolve the interact `KeyCode` from the default controls config.
    fn interact_key() -> KeyCode {
        let cfg = ControlsConfig::default();
        parse_key_code(&cfg.interact[0]).expect("default interact key must be parseable")
    }

    // ── Test 1: regular WallType::Door tile (no LockedDoor event) opens immediately

    /// Pressing `E` in front of a plain `WallType::Door` tile (no `LockedDoor`
    /// map event) must set the tile to `WallType::None` and clear `blocked`.
    ///
    /// This covers the "existing behaviour" fallback added in Phase 2 for
    /// tile-based doors that are not associated with a lock.
    #[test]
    fn test_e_key_on_regular_door_opens_it() {
        let mut app = build_locked_door_app();
        let pos = door_pos();

        // Place a WallType::Door tile at target with NO LockedDoor event.
        place_door_tile(&mut app);

        // Press E (interact).
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(interact_key());
        }
        app.update();

        let gs = app.world().resource::<GlobalState>();
        let tile = gs.0.world.get_current_map().unwrap().get_tile(pos).unwrap();

        assert_eq!(
            tile.wall_type,
            WallType::None,
            "Plain WallType::Door tile must become WallType::None after E-key"
        );
        assert!(
            !tile.blocked,
            "Plain door tile must be unblocked after opening"
        );
    }

    // ── Test 2: locked door + correct key opens it

    /// Pressing `E` in front of a locked door when the party carries the
    /// required key must:
    ///   1. Set the tile to `WallType::None` and clear `blocked`.
    ///   2. Mark `lock_state.is_locked = false`.
    ///   3. Consume the key from the carrying character's inventory.
    ///   4. Log a "You unlock the door with the…" message.
    #[test]
    fn test_e_key_on_locked_door_with_correct_key_opens_it() {
        let mut app = build_locked_door_app();
        let pos = door_pos();

        place_door_tile(&mut app);
        add_locked_door_event(&mut app, Some(DOOR_KEY_ID));
        give_key_to_party(&mut app, DOOR_KEY_ID);

        // Confirm party starts with the key.
        {
            let gs = app.world().resource::<GlobalState>();
            assert!(
                gs.0.party.members[0]
                    .inventory
                    .items
                    .iter()
                    .any(|s| s.item_id == DOOR_KEY_ID),
                "Party must have the key before interacting"
            );
        }

        // Press E.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(interact_key());
        }
        app.update();

        // Tile must be open.
        let gs = app.world().resource::<GlobalState>();
        let map = gs.0.world.get_current_map().unwrap();
        let tile = map.get_tile(pos).unwrap();
        assert_eq!(
            tile.wall_type,
            WallType::None,
            "Locked door tile must become WallType::None after key unlock"
        );
        assert!(
            !tile.blocked,
            "Door tile must be unblocked after key unlock"
        );

        // Lock state must be unlocked.
        let lock_state = map.lock_states.get(LOCK_ID).unwrap();
        assert!(
            !lock_state.is_locked,
            "lock_state.is_locked must be false after key unlock"
        );

        // Key must have been consumed.
        assert!(
            !gs.0.party.members[0]
                .inventory
                .items
                .iter()
                .any(|s| s.item_id == DOOR_KEY_ID),
            "Key must be removed from inventory after unlocking"
        );

        // Game log must contain a success message.
        let log = app.world().resource::<crate::game::systems::ui::GameLog>();
        assert!(
            log.messages
                .iter()
                .any(|m| m.starts_with("You unlock the door with the")),
            "Game log must contain 'You unlock the door with the…' message; got: {:?}",
            log.messages
        );
    }

    // ── Test 3: locked door with no key → LockInteractionPending is set

    /// Pressing `E` in front of a locked door when the party has no key must:
    ///   1. Leave the tile unchanged (`WallType::Door`, `blocked = true`).
    ///   2. Set `LockInteractionPending.lock_id` to `Some(LOCK_ID)`.
    ///   3. Log "The door is locked. You need a key."
    #[test]
    fn test_e_key_on_locked_door_without_key_sets_pending() {
        let mut app = build_locked_door_app();
        let pos = door_pos();

        place_door_tile(&mut app);
        // Lock requires DOOR_KEY_ID but party has none.
        add_locked_door_event(&mut app, Some(DOOR_KEY_ID));

        // Press E.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(interact_key());
        }
        app.update();

        // Tile must remain locked.
        let gs = app.world().resource::<GlobalState>();
        let tile = gs.0.world.get_current_map().unwrap().get_tile(pos).unwrap();
        assert_eq!(
            tile.wall_type,
            WallType::Door,
            "Door tile must remain WallType::Door when party lacks the key"
        );
        assert!(tile.blocked, "Door tile must remain blocked");

        // LockInteractionPending must be populated.
        let pending = app.world().resource::<LockInteractionPending>();
        assert_eq!(
            pending.lock_id,
            Some(LOCK_ID.to_string()),
            "LockInteractionPending.lock_id must be set to the lock ID"
        );
        assert_eq!(
            pending.position,
            Some(pos),
            "LockInteractionPending.position must point to the door tile"
        );

        // Game log must contain the "need a key" message.
        let log = app.world().resource::<crate::game::systems::ui::GameLog>();
        assert!(
            log.messages.iter().any(|m| m.contains("You need a key")),
            "Game log must contain 'You need a key' message; got: {:?}",
            log.messages
        );
    }

    // ── Test 4: locked door with wrong key stays locked

    /// Pressing `E` in front of a locked door when the party carries a
    /// *different* key (wrong item ID) must leave the door locked and set
    /// `LockInteractionPending`, just as if no key were present.
    #[test]
    fn test_e_key_on_locked_door_wrong_key_stays_locked() {
        let mut app = build_locked_door_app();
        let pos = door_pos();

        place_door_tile(&mut app);
        // Lock requires DOOR_KEY_ID; party has WRONG_KEY_ID.
        add_locked_door_event(&mut app, Some(DOOR_KEY_ID));
        give_key_to_party(&mut app, WRONG_KEY_ID);

        // Press E.
        {
            let mut kb = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            kb.press(interact_key());
        }
        app.update();

        // Tile must still be WallType::Door.
        let gs = app.world().resource::<GlobalState>();
        let tile = gs.0.world.get_current_map().unwrap().get_tile(pos).unwrap();
        assert_eq!(
            tile.wall_type,
            WallType::Door,
            "Door must remain WallType::Door when party only has the wrong key"
        );
        assert!(tile.blocked, "Door tile must remain blocked with wrong key");

        // LockInteractionPending must be set (Phase 3 will offer pick-lock/bash).
        let pending = app.world().resource::<LockInteractionPending>();
        assert_eq!(
            pending.lock_id,
            Some(LOCK_ID.to_string()),
            "LockInteractionPending must be set even when party has the wrong key"
        );

        // Wrong key must still be in the party inventory (not consumed).
        assert!(
            gs.0.party.members[0]
                .inventory
                .items
                .iter()
                .any(|s| s.item_id == WRONG_KEY_ID),
            "Wrong key must not be consumed from inventory"
        );
    }
}
