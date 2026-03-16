// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
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
use crate::domain::types::Position;
use crate::domain::world::{MapEvent, WallType};
use crate::game::components::dialogue::NpcDialogue;
use crate::game::components::furniture::DoorState;
use crate::game::components::FurnitureEntity;
use crate::game::resources::GlobalState;
use crate::game::systems::dialogue::{PendingRecruitmentContext, StartDialogue};
use crate::game::systems::events::MapEventTriggered;
use crate::game::systems::map::{DoorOpenedEvent, NpcMarker, TileCoord};
#[cfg(test)]
use crate::game::systems::rest::InitiateRestEvent;
use crate::sdk::game_config::ControlsConfig;
use bevy::prelude::*;
use std::collections::HashMap;

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

/// Game actions that can be triggered by input
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameAction {
    /// Move forward in current facing direction
    MoveForward,

    /// Move backward (opposite of facing direction)
    MoveBack,

    /// Turn left (rotate counterclockwise)
    TurnLeft,

    /// Turn right (rotate clockwise)
    TurnRight,

    /// Interact with objects (open doors, talk to NPCs)
    Interact,

    /// Open menu
    Menu,

    /// Open or close the inventory screen
    Inventory,

    /// Begin a party rest sequence
    Rest,
}

/// Key mapping structure for efficient input lookups
///
/// Maps `KeyCode` to `GameAction` for fast input processing.
#[derive(Debug, Clone)]
pub struct KeyMap {
    /// Map from KeyCode to GameAction
    bindings: HashMap<KeyCode, GameAction>,
}

impl KeyMap {
    /// Create a KeyMap from ControlsConfig
    ///
    /// Translates string key names to Bevy KeyCode and builds the lookup map.
    ///
    /// # Arguments
    ///
    /// * `config` - Controls configuration with key binding strings
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::input::KeyMap;
    /// use antares::sdk::game_config::ControlsConfig;
    ///
    /// let config = ControlsConfig::default();
    /// let key_map = KeyMap::from_controls_config(&config);
    /// ```
    pub fn from_controls_config(config: &ControlsConfig) -> Self {
        let mut bindings = HashMap::new();

        // Map move_forward keys
        for key_str in &config.move_forward {
            if let Some(key_code) = parse_key_code(key_str) {
                bindings.insert(key_code, GameAction::MoveForward);
            } else {
                warn!("Invalid key code in move_forward: {}", key_str);
            }
        }

        // Map move_back keys
        for key_str in &config.move_back {
            if let Some(key_code) = parse_key_code(key_str) {
                bindings.insert(key_code, GameAction::MoveBack);
            } else {
                warn!("Invalid key code in move_back: {}", key_str);
            }
        }

        // Map turn_left keys
        for key_str in &config.turn_left {
            if let Some(key_code) = parse_key_code(key_str) {
                bindings.insert(key_code, GameAction::TurnLeft);
            } else {
                warn!("Invalid key code in turn_left: {}", key_str);
            }
        }

        // Map turn_right keys
        for key_str in &config.turn_right {
            if let Some(key_code) = parse_key_code(key_str) {
                bindings.insert(key_code, GameAction::TurnRight);
            } else {
                warn!("Invalid key code in turn_right: {}", key_str);
            }
        }

        // Map interact keys
        for key_str in &config.interact {
            if let Some(key_code) = parse_key_code(key_str) {
                bindings.insert(key_code, GameAction::Interact);
            } else {
                warn!("Invalid key code in interact: {}", key_str);
            }
        }

        // Map menu keys
        for key_str in &config.menu {
            if let Some(key_code) = parse_key_code(key_str) {
                bindings.insert(key_code, GameAction::Menu);
            } else {
                warn!("Invalid key code in menu: {}", key_str);
            }
        }

        // Map inventory keys
        for key_str in &config.inventory {
            if let Some(key_code) = parse_key_code(key_str) {
                bindings.insert(key_code, GameAction::Inventory);
            } else {
                warn!("Invalid key code in inventory: {}", key_str);
            }
        }

        // Map rest keys
        for key_str in &config.rest {
            if let Some(key_code) = parse_key_code(key_str) {
                bindings.insert(key_code, GameAction::Rest);
            } else {
                warn!("Invalid key code in rest: {}", key_str);
            }
        }

        Self { bindings }
    }

    /// Get the action bound to a specific key code
    ///
    /// # Arguments
    ///
    /// * `key_code` - The key code to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(GameAction)` if the key is bound, `None` otherwise
    pub fn get_action(&self, key_code: KeyCode) -> Option<GameAction> {
        self.bindings.get(&key_code).copied()
    }

    /// Check if any of the keys for an action are currently pressed
    ///
    /// # Arguments
    ///
    /// * `action` - The game action to check
    /// * `keyboard_input` - Bevy keyboard input state
    ///
    /// # Returns
    ///
    /// Returns `true` if any key bound to the action is pressed
    pub fn is_action_pressed(
        &self,
        action: GameAction,
        keyboard_input: &ButtonInput<KeyCode>,
    ) -> bool {
        self.bindings.iter().any(|(key_code, bound_action)| {
            *bound_action == action && keyboard_input.pressed(*key_code)
        })
    }

    /// Check if any of the keys for an action were just pressed this frame
    ///
    /// # Arguments
    ///
    /// * `action` - The game action to check
    /// * `keyboard_input` - Bevy keyboard input state
    ///
    /// # Returns
    ///
    /// Returns `true` if any key bound to the action was just pressed
    pub fn is_action_just_pressed(
        &self,
        action: GameAction,
        keyboard_input: &ButtonInput<KeyCode>,
    ) -> bool {
        self.bindings.iter().any(|(key_code, bound_action)| {
            *bound_action == action && keyboard_input.just_pressed(*key_code)
        })
    }
}

/// Parse a key code string into Bevy's KeyCode enum
///
/// Supports common key names and aliases for compatibility.
///
/// # Arguments
///
/// * `key_str` - String representation of the key (e.g., "W", "ArrowUp", "Space")
///
/// # Returns
///
/// Returns `Some(KeyCode)` if the string is recognized, `None` otherwise
///
/// # Examples
///
/// ```
/// use antares::game::systems::input::parse_key_code;
/// use bevy::prelude::KeyCode;
///
/// assert_eq!(parse_key_code("W"), Some(KeyCode::KeyW));
/// assert_eq!(parse_key_code("ArrowUp"), Some(KeyCode::ArrowUp));
/// assert_eq!(parse_key_code("Space"), Some(KeyCode::Space));
/// assert_eq!(parse_key_code("Invalid"), None);
/// ```
pub fn parse_key_code(key_str: &str) -> Option<KeyCode> {
    match key_str {
        // Letter keys
        "A" => Some(KeyCode::KeyA),
        "B" => Some(KeyCode::KeyB),
        "C" => Some(KeyCode::KeyC),
        "D" => Some(KeyCode::KeyD),
        "E" => Some(KeyCode::KeyE),
        "F" => Some(KeyCode::KeyF),
        "G" => Some(KeyCode::KeyG),
        "H" => Some(KeyCode::KeyH),
        "I" => Some(KeyCode::KeyI),
        "J" => Some(KeyCode::KeyJ),
        "K" => Some(KeyCode::KeyK),
        "L" => Some(KeyCode::KeyL),
        "M" => Some(KeyCode::KeyM),
        "N" => Some(KeyCode::KeyN),
        "O" => Some(KeyCode::KeyO),
        "P" => Some(KeyCode::KeyP),
        "Q" => Some(KeyCode::KeyQ),
        "R" => Some(KeyCode::KeyR),
        "S" => Some(KeyCode::KeyS),
        "T" => Some(KeyCode::KeyT),
        "U" => Some(KeyCode::KeyU),
        "V" => Some(KeyCode::KeyV),
        "W" => Some(KeyCode::KeyW),
        "X" => Some(KeyCode::KeyX),
        "Y" => Some(KeyCode::KeyY),
        "Z" => Some(KeyCode::KeyZ),

        // Arrow keys
        "ArrowUp" | "Up" => Some(KeyCode::ArrowUp),
        "ArrowDown" | "Down" => Some(KeyCode::ArrowDown),
        "ArrowLeft" | "Left" => Some(KeyCode::ArrowLeft),
        "ArrowRight" | "Right" => Some(KeyCode::ArrowRight),

        // Special keys
        "Space" | "Spacebar" => Some(KeyCode::Space),
        "Enter" | "Return" => Some(KeyCode::Enter),
        "Escape" | "Esc" => Some(KeyCode::Escape),
        "Tab" => Some(KeyCode::Tab),
        "Backspace" => Some(KeyCode::Backspace),

        // Number keys
        "0" | "Digit0" => Some(KeyCode::Digit0),
        "1" | "Digit1" => Some(KeyCode::Digit1),
        "2" | "Digit2" => Some(KeyCode::Digit2),
        "3" | "Digit3" => Some(KeyCode::Digit3),
        "4" | "Digit4" => Some(KeyCode::Digit4),
        "5" | "Digit5" => Some(KeyCode::Digit5),
        "6" | "Digit6" => Some(KeyCode::Digit6),
        "7" | "Digit7" => Some(KeyCode::Digit7),
        "8" | "Digit8" => Some(KeyCode::Digit8),
        "9" | "Digit9" => Some(KeyCode::Digit9),

        // Function keys
        "F1" => Some(KeyCode::F1),
        "F2" => Some(KeyCode::F2),
        "F3" => Some(KeyCode::F3),
        "F4" => Some(KeyCode::F4),
        "F5" => Some(KeyCode::F5),
        "F6" => Some(KeyCode::F6),
        "F7" => Some(KeyCode::F7),
        "F8" => Some(KeyCode::F8),
        "F9" => Some(KeyCode::F9),
        "F10" => Some(KeyCode::F10),
        "F11" => Some(KeyCode::F11),
        "F12" => Some(KeyCode::F12),

        // Modifier keys
        "Shift" | "ShiftLeft" => Some(KeyCode::ShiftLeft),
        "ShiftRight" => Some(KeyCode::ShiftRight),
        "Control" | "Ctrl" | "ControlLeft" => Some(KeyCode::ControlLeft),
        "ControlRight" | "CtrlRight" => Some(KeyCode::ControlRight),
        "Alt" | "AltLeft" => Some(KeyCode::AltLeft),
        "AltRight" => Some(KeyCode::AltRight),

        _ => {
            // Try lowercase version
            let lowercase = key_str.to_lowercase();
            if lowercase != key_str {
                return parse_key_code(&lowercase);
            }
            None
        }
    }
}

/// Toggle the in-game menu: open it if not open, or close it and return to the previous mode if open.
///
/// This helper intentionally does not consider movement cooldown so it can be called
/// from input handlers that must ensure the menu key always works.
///
/// # Arguments
///
/// * `game_state` - Mutable reference to the current `GameState`
fn toggle_menu_state(game_state: &mut crate::application::GameState) {
    use crate::application::menu::MenuState;
    match &game_state.mode {
        crate::application::GameMode::Menu(menu_state) => {
            let resume_mode = menu_state.get_resume_mode();
            info!("Closing menu, resuming to: {:?}", resume_mode);
            game_state.mode = resume_mode;
        }
        current_mode => {
            info!("Opening menu from: {:?}", current_mode);
            let menu_state = MenuState::new(current_mode.clone());
            game_state.mode = crate::application::GameMode::Menu(menu_state);
        }
    }
}

/// Handle keyboard input and translate to game actions
///
/// This system processes keyboard input using the configured key mappings,
/// applies movement cooldown, and updates game state accordingly.
#[allow(clippy::too_many_arguments)]
fn handle_input(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    input_config: Res<InputConfigResource>,
    mut global_state: ResMut<GlobalState>,
    mut door_messages: MessageWriter<DoorOpenedEvent>,
    mut map_event_messages: MessageWriter<MapEventTriggered>,
    mut dialogue_writer: MessageWriter<StartDialogue>,
    mut recruitment_context: ResMut<PendingRecruitmentContext>,

    time: Res<Time>,
    mut last_move_time: Local<f32>,
    victory_roots: Query<Entity, With<crate::game::systems::combat::VictorySummaryRoot>>,
    npc_query: Query<(Entity, &NpcMarker, &TileCoord)>,
    dialogue_query: Query<&NpcDialogue>,
    game_content: Option<Res<crate::application::resources::GameContent>>,
    // Query for furniture door entities — provides open/locked state and
    // allows toggling the door transform without a full map respawn.
    mut door_entity_query: Query<(
        &mut FurnitureEntity,
        &mut DoorState,
        &mut Transform,
        &TileCoord,
    )>,
    // Optional game log for player-visible feedback messages (e.g. "Door is locked.")
    mut game_log: Option<ResMut<crate::game::systems::ui::GameLog>>,
) {
    let current_time = time.elapsed_secs();
    let cooldown = input_config.controls.movement_cooldown;

    // Check for menu toggle (ESC key) first — it should always take priority and
    // must not be blocked by movement cooldown.
    if input_config
        .key_map
        .is_action_just_pressed(GameAction::Menu, &keyboard_input)
    {
        let game_state = &mut global_state.0;
        toggle_menu_state(game_state);
        info!("Menu toggled: new_mode = {:?}", game_state.mode);
        return; // Exit early after menu toggle
    }

    // Check for inventory toggle ("I" key) — same priority as menu toggle.
    if input_config
        .key_map
        .is_action_just_pressed(GameAction::Inventory, &keyboard_input)
    {
        let game_state = &mut global_state.0;

        // Capture the npc_id from dialogue state before any mutable borrow.
        let dialogue_npc_id: Option<String> =
            if let crate::application::GameMode::Dialogue(ref ds) = game_state.mode {
                ds.speaker_npc_id.clone()
            } else {
                None
            };

        match &game_state.mode {
            crate::application::GameMode::Inventory(inv_state) => {
                // Close inventory: restore previous mode
                let resume = inv_state.get_resume_mode();
                info!("Inventory closed: restored mode = {:?}", resume);
                game_state.mode = resume;
            }
            crate::application::GameMode::Dialogue(_) => {
                // In dialogue: only open merchant inventory when the NPC is a merchant.
                if let Some(npc_id) = dialogue_npc_id {
                    if let Some(content) = game_content.as_deref() {
                        if let Some(npc_def) = content.db().npcs.get_npc(&npc_id) {
                            if npc_def.is_merchant {
                                game_state.ensure_npc_runtime_initialized(content.db());
                                let npc_name = npc_def.name.clone();
                                info!(
                                    "I key in Dialogue: opening merchant inventory for '{}'",
                                    npc_id
                                );
                                game_state.enter_merchant_inventory(npc_id, npc_name);
                            } else {
                                // Non-merchant NPC: silently ignore the I key press.
                                info!(
                                    "I key in Dialogue: NPC '{}' is not a merchant, ignoring",
                                    npc_id
                                );
                            }
                        }
                    }
                }
                // Whether we opened the merchant inventory or not, consume the
                // key press and return so it doesn't fall through to other branches.
            }
            crate::application::GameMode::Menu(_) | crate::application::GameMode::Combat(_) => {
                // Do not open inventory from menu or combat mode
            }
            _ => {
                game_state.enter_inventory();
                info!("Inventory opened: mode = {:?}", game_state.mode);
            }
        }
        return; // Exit early after inventory toggle
    }

    // Check for rest action ("R" key) — opens the rest-duration menu only
    // during Exploration mode.  All other modes silently ignore the key press.
    if input_config
        .key_map
        .is_action_just_pressed(GameAction::Rest, &keyboard_input)
    {
        let game_state = &mut global_state.0;
        if matches!(game_state.mode, crate::application::GameMode::Exploration) {
            info!("Rest key pressed: opening rest menu");
            game_state.enter_rest_menu();
        } else {
            info!(
                "Rest key pressed but mode is {:?} — ignoring",
                game_state.mode
            );
        }
        return; // Consume the key press regardless of mode
    }

    // Throttle movement input using cooldown. Only block when an actual movement
    // action is being attempted.
    let is_movement_attempt = input_config
        .key_map
        .is_action_pressed(GameAction::MoveForward, &keyboard_input)
        || input_config
            .key_map
            .is_action_pressed(GameAction::MoveBack, &keyboard_input)
        || input_config
            .key_map
            .is_action_pressed(GameAction::TurnLeft, &keyboard_input)
        || input_config
            .key_map
            .is_action_pressed(GameAction::TurnRight, &keyboard_input);

    if is_movement_attempt && (current_time - *last_move_time < cooldown) {
        // Movement attempted but still within cooldown window - ignore movement input.
        return;
    }

    // ALLOW input processing in Dialogue mode to enable "Move to Cancel"
    // But block Interaction actions (doors, etc.) if in Dialogue.
    // BLOCK all movement/interaction input when in Menu mode (menu system handles its own input)

    let game_state = &mut global_state.0;

    // Menu toggle handled above before movement cooldown checks.

    // Block all movement/interaction input when in Menu or Inventory mode.
    // Each mode's own system handles its own input processing.
    if matches!(
        game_state.mode,
        crate::application::GameMode::Menu(_) | crate::application::GameMode::Inventory(_)
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
    // NOTE: We intentionally use "pressed" (not "just pressed") so interaction
    // behaves consistently with the existing movement model and door behavior,
    // and so headless tests can exercise interaction without depending on
    // Bevy's per-frame input edge detection.
    // Only allow Interaction if NOT in Dialogue mode
    if !matches!(game_state.mode, crate::application::GameMode::Dialogue(_))
        && input_config
            .key_map
            .is_action_pressed(GameAction::Interact, &keyboard_input)
    {
        let party_position = world.party_position;
        let adjacent_tiles = get_adjacent_positions(party_position);

        // Door interaction - check if there's a door in front of the party
        let target = world.position_ahead();

        // ── Phase 3: Furniture door interaction ───────────────────────────
        // Check for a furniture-based door entity at the tile ahead BEFORE the
        // legacy WallType::Door path so migrated doors are handled first.
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
                        if let Some(ref mut log) = game_log {
                            log.add(msg);
                        }
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

        // ── Legacy: WallType::Door tile interaction ────────────────────────
        // Kept as a fallback for un-migrated maps. Phase 4 removes this path.
        if let Some(map) = world.get_current_map_mut() {
            if let Some(tile) = map.get_tile_mut(target) {
                if tile.wall_type == WallType::Door {
                    // Open the door by changing it to None
                    tile.wall_type = WallType::None;
                    info!("Opened door at {:?}", target);
                    // Send event to trigger map visual refresh
                    door_messages.write(DoorOpenedEvent { position: target });
                    return; // Door handled; don't fall through to other checks
                }
            }
        }

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
                        dialogue_id,
                        ..
                    } => {
                        info!(
                            "Interacting with recruitable character '{}' (ID: {}) at {:?}",
                            name, character_id, position
                        );
                        // Find the NPC entity at this position
                        let speaker_entity = npc_query
                            .iter()
                            .find(|(_, _, tile_coord)| tile_coord.0 == position)
                            .map(|(entity, _, _)| entity);

                        // Use specific dialogue ID if the NPC has one, otherwise fallback to 100
                        // Use specific dialogue ID from event if available,
                        // OR fallback to NPC component,
                        // OR fallback to default 100
                        let dialogue_id = dialogue_id
                            .or_else(|| {
                                speaker_entity
                                    .and_then(|entity| dialogue_query.get(entity).ok())
                                    .map(|npc_dlg| npc_dlg.dialogue_id)
                            })
                            .unwrap_or(100);

                        // Set recruitment context so the dialogue system knows who to recruit
                        recruitment_context.0 = Some(RecruitmentContext {
                            character_id: character_id.clone(),
                            event_position: position,
                        });

                        dialogue_writer.write(StartDialogue {
                            dialogue_id,
                            speaker_entity,
                            fallback_position: Some(position),
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
    else if input_config
        .key_map
        .is_action_pressed(GameAction::MoveForward, &keyboard_input)
    {
        let target = world.position_ahead();

        // Phase 3: deny movement into a locked furniture door, surfacing
        // MovementError::DoorLocked semantics at the input layer.
        let locked_door_ahead = door_entity_query
            .iter()
            .any(|(_, ds, _, tc)| tc.0 == target && ds.is_locked && !ds.is_open);

        if locked_door_ahead {
            let msg = "The door is locked.".to_string();
            info!("{}", msg);
            if let Some(ref mut log) = game_log {
                log.add(msg);
            }
        } else if let Some(map) = world.get_current_map() {
            if !map.is_blocked(target) {
                world.set_party_position(target);
                moved = true;
            }
        }
    }
    // Move backward
    else if input_config
        .key_map
        .is_action_pressed(GameAction::MoveBack, &keyboard_input)
    {
        // Calculate position behind party
        let back_facing = world.party_facing.turn_left().turn_left();
        let target = back_facing.forward(world.party_position);

        if let Some(map) = world.get_current_map() {
            if !map.is_blocked(target) {
                world.set_party_position(target);
                moved = true;
            }
        }
    }
    // Turn left
    else if input_config
        .key_map
        .is_action_pressed(GameAction::TurnLeft, &keyboard_input)
    {
        world.turn_left();
        moved = true;
    }
    // Turn right
    else if input_config
        .key_map
        .is_action_pressed(GameAction::TurnRight, &keyboard_input)
    {
        world.turn_right();
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

/// Returns all 8 adjacent positions around a given position
///
/// Returns tiles in clockwise order starting from North:
/// N, NE, E, SE, S, SW, W, NW
///
/// # Arguments
///
/// * `position` - The center position
///
/// # Returns
///
/// Array of 8 `Position` values representing adjacent tiles
fn get_adjacent_positions(position: Position) -> [Position; 8] {
    [
        Position::new(position.x, position.y - 1),     // North
        Position::new(position.x + 1, position.y - 1), // NorthEast
        Position::new(position.x + 1, position.y),     // East
        Position::new(position.x + 1, position.y + 1), // SouthEast
        Position::new(position.x, position.y + 1),     // South
        Position::new(position.x - 1, position.y + 1), // SouthWest
        Position::new(position.x - 1, position.y),     // West
        Position::new(position.x - 1, position.y - 1), // NorthWest
    ]
}

#[cfg(test)]
mod adjacent_tile_tests {
    use super::*;

    #[test]
    fn test_adjacent_positions_count() {
        let center = Position::new(5, 5);
        let adjacent = get_adjacent_positions(center);
        assert_eq!(adjacent.len(), 8);
    }

    #[test]
    fn test_adjacent_positions_north() {
        let center = Position::new(5, 5);
        let adjacent = get_adjacent_positions(center);
        assert_eq!(adjacent[0], Position::new(5, 4)); // North
    }

    #[test]
    fn test_adjacent_positions_east() {
        let center = Position::new(5, 5);
        let adjacent = get_adjacent_positions(center);
        assert_eq!(adjacent[2], Position::new(6, 5)); // East
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
        app.insert_resource::<Time>(Time::default());
        app.insert_resource(PendingRecruitmentContext::default());
        app.insert_resource(GameContent::new(db));
        app.add_message::<DoorOpenedEvent>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<StartDialogue>();
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
    fn test_key_map_rest_action() {
        let config = ControlsConfig::default();
        let key_map = KeyMap::from_controls_config(&config);
        assert_eq!(
            key_map.get_action(KeyCode::KeyR),
            Some(GameAction::Rest),
            "KeyCode::KeyR must map to GameAction::Rest with default config"
        );
    }

    #[test]
    fn test_custom_rest_key() {
        let config = ControlsConfig {
            rest: vec!["F5".to_string()],
            ..Default::default()
        };
        let key_map = KeyMap::from_controls_config(&config);
        assert_eq!(
            key_map.get_action(KeyCode::F5),
            Some(GameAction::Rest),
            "F5 must map to GameAction::Rest when configured as rest key"
        );
        // Default R key must not be mapped when overridden
        assert_eq!(
            key_map.get_action(KeyCode::KeyR),
            None,
            "KeyR must not be mapped when rest is overridden to F5"
        );
    }

    #[test]
    fn test_key_map_inventory_action() {
        let config = ControlsConfig::default();
        let key_map = KeyMap::from_controls_config(&config);
        assert_eq!(
            key_map.get_action(KeyCode::KeyI),
            Some(GameAction::Inventory),
            "KeyCode::KeyI must map to GameAction::Inventory with default config"
        );
    }

    #[test]
    fn test_parse_key_code_letters() {
        assert_eq!(parse_key_code("W"), Some(KeyCode::KeyW));
        assert_eq!(parse_key_code("A"), Some(KeyCode::KeyA));
        assert_eq!(parse_key_code("S"), Some(KeyCode::KeyS));
        assert_eq!(parse_key_code("D"), Some(KeyCode::KeyD));
    }

    #[test]
    fn test_parse_key_code_arrows() {
        assert_eq!(parse_key_code("ArrowUp"), Some(KeyCode::ArrowUp));
        assert_eq!(parse_key_code("ArrowDown"), Some(KeyCode::ArrowDown));
        assert_eq!(parse_key_code("ArrowLeft"), Some(KeyCode::ArrowLeft));
        assert_eq!(parse_key_code("ArrowRight"), Some(KeyCode::ArrowRight));
    }

    #[test]
    fn test_parse_key_code_arrow_aliases() {
        assert_eq!(parse_key_code("Up"), Some(KeyCode::ArrowUp));
        assert_eq!(parse_key_code("Down"), Some(KeyCode::ArrowDown));
        assert_eq!(parse_key_code("Left"), Some(KeyCode::ArrowLeft));
        assert_eq!(parse_key_code("Right"), Some(KeyCode::ArrowRight));
    }

    #[test]
    fn test_parse_key_code_special() {
        assert_eq!(parse_key_code("Space"), Some(KeyCode::Space));
        assert_eq!(parse_key_code("Spacebar"), Some(KeyCode::Space));
        assert_eq!(parse_key_code("Escape"), Some(KeyCode::Escape));
        assert_eq!(parse_key_code("Esc"), Some(KeyCode::Escape));
        assert_eq!(parse_key_code("Enter"), Some(KeyCode::Enter));
    }

    #[test]
    fn test_toggle_menu_state_from_exploration_and_back() {
        // Start in Exploration mode
        let mut state = crate::application::GameState::new();
        assert!(matches!(
            state.mode,
            crate::application::GameMode::Exploration
        ));

        // Toggle to Menu
        toggle_menu_state(&mut state);
        assert!(matches!(state.mode, crate::application::GameMode::Menu(_)));

        // Toggle back to Exploration
        toggle_menu_state(&mut state);
        assert!(matches!(
            state.mode,
            crate::application::GameMode::Exploration
        ));
    }

    #[test]
    fn test_toggle_menu_state_preserves_previous_mode() {
        // Ensure the MenuState records the previous mode correctly
        let mut state = crate::application::GameState::new();
        toggle_menu_state(&mut state);

        if let crate::application::GameMode::Menu(menu_state) = &state.mode {
            assert!(matches!(
                menu_state.get_resume_mode(),
                crate::application::GameMode::Exploration
            ));
        } else {
            panic!("Expected to be in Menu mode after toggle");
        }
    }

    #[test]
    fn test_parse_key_code_invalid() {
        assert_eq!(parse_key_code("InvalidKey"), None);
        assert_eq!(parse_key_code(""), None);
    }

    #[test]
    fn test_key_map_from_default_config() {
        let config = ControlsConfig::default();
        let key_map = KeyMap::from_controls_config(&config);

        // Check that default keys are mapped correctly
        assert_eq!(
            key_map.get_action(KeyCode::KeyW),
            Some(GameAction::MoveForward)
        );
        assert_eq!(
            key_map.get_action(KeyCode::ArrowUp),
            Some(GameAction::MoveForward)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyS),
            Some(GameAction::MoveBack)
        );
        assert_eq!(
            key_map.get_action(KeyCode::ArrowDown),
            Some(GameAction::MoveBack)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyA),
            Some(GameAction::TurnLeft)
        );
        assert_eq!(
            key_map.get_action(KeyCode::ArrowLeft),
            Some(GameAction::TurnLeft)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyD),
            Some(GameAction::TurnRight)
        );
        assert_eq!(
            key_map.get_action(KeyCode::ArrowRight),
            Some(GameAction::TurnRight)
        );
        assert_eq!(
            key_map.get_action(KeyCode::Space),
            Some(GameAction::Interact)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyE),
            Some(GameAction::Interact)
        );
        assert_eq!(key_map.get_action(KeyCode::Escape), Some(GameAction::Menu));
    }

    #[test]
    fn test_key_map_custom_config() {
        let config = ControlsConfig {
            move_forward: vec!["I".to_string()],
            move_back: vec!["K".to_string()],
            turn_left: vec!["J".to_string()],
            turn_right: vec!["L".to_string()],
            interact: vec!["U".to_string()],
            menu: vec!["P".to_string()],
            inventory: vec!["F".to_string()],
            rest: vec!["G".to_string()],
            movement_cooldown: 0.1,
        };

        let key_map = KeyMap::from_controls_config(&config);

        assert_eq!(
            key_map.get_action(KeyCode::KeyI),
            Some(GameAction::MoveForward)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyK),
            Some(GameAction::MoveBack)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyJ),
            Some(GameAction::TurnLeft)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyL),
            Some(GameAction::TurnRight)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyU),
            Some(GameAction::Interact)
        );
        assert_eq!(key_map.get_action(KeyCode::KeyP), Some(GameAction::Menu));

        // Old defaults should not be mapped
        assert_eq!(key_map.get_action(KeyCode::KeyW), None);
        assert_eq!(key_map.get_action(KeyCode::Space), None);
    }

    #[test]
    fn test_key_map_multiple_keys_per_action() {
        let config = ControlsConfig {
            move_forward: vec!["W".to_string(), "ArrowUp".to_string(), "I".to_string()],
            move_back: vec!["S".to_string()],
            turn_left: vec!["A".to_string()],
            turn_right: vec!["D".to_string()],
            interact: vec!["Space".to_string()],
            menu: vec!["Escape".to_string()],
            inventory: vec!["F".to_string()],
            rest: vec!["R".to_string()],
            movement_cooldown: 0.2,
        };

        let key_map = KeyMap::from_controls_config(&config);

        // All three keys should map to MoveForward
        assert_eq!(
            key_map.get_action(KeyCode::KeyW),
            Some(GameAction::MoveForward)
        );
        assert_eq!(
            key_map.get_action(KeyCode::ArrowUp),
            Some(GameAction::MoveForward)
        );
        assert_eq!(
            key_map.get_action(KeyCode::KeyI),
            Some(GameAction::MoveForward)
        );
    }

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
        app.insert_resource::<Time>(Time::default());
        app.insert_resource(PendingRecruitmentContext::default());
        app.add_message::<DoorOpenedEvent>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<StartDialogue>();
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

        // Insert required resources: button input, config, global state, and time.
        app.insert_resource(ButtonInput::<KeyCode>::default());

        let cfg = ControlsConfig::default();
        let km = KeyMap::from_controls_config(&cfg);
        app.insert_resource(InputConfigResource {
            controls: cfg,
            key_map: km,
        });

        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.insert_resource::<Time>(Time::default());
        app.insert_resource(PendingRecruitmentContext::default());

        // Register message channels the input system depends on so MessageWriter<T>
        // parameters are initialized when running the system in tests.
        app.add_message::<DoorOpenedEvent>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<StartDialogue>();
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
        let cfg = ControlsConfig::default();
        app.insert_resource(InputConfigResource {
            controls: cfg.clone(),
            key_map: KeyMap::from_controls_config(&cfg),
        });
        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.insert_resource::<Time>(Time::default());
        app.insert_resource(PendingRecruitmentContext::default());

        // Register messages used by input system
        app.add_message::<DoorOpenedEvent>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<StartDialogue>();
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
        let cfg = ControlsConfig::default();
        app.insert_resource(InputConfigResource {
            controls: cfg.clone(),
            key_map: KeyMap::from_controls_config(&cfg),
        });
        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.insert_resource::<Time>(Time::default());
        app.insert_resource(PendingRecruitmentContext::default());

        // Register messages used by input system
        app.add_message::<DoorOpenedEvent>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<StartDialogue>();
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
        let center = Position::new(5, 5);
        let adjacent = get_adjacent_positions(center);

        // Assert - verify all 8 positions are adjacent
        assert!(adjacent.contains(&Position::new(5, 4))); // North
        assert!(adjacent.contains(&Position::new(6, 4))); // NorthEast
        assert!(adjacent.contains(&Position::new(6, 5))); // East
        assert!(adjacent.contains(&Position::new(6, 6))); // SouthEast
        assert!(adjacent.contains(&Position::new(5, 6))); // South
        assert!(adjacent.contains(&Position::new(4, 6))); // SouthWest
        assert!(adjacent.contains(&Position::new(4, 5))); // West
        assert!(adjacent.contains(&Position::new(4, 4))); // NorthWest
    }

    /// Test that sign interaction detects signs in adjacent positions.
    /// Validates that map events are properly stored and retrievable.
    #[test]
    fn test_sign_interaction_event_storage() {
        // Arrange
        let mut map =
            crate::domain::world::Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);

        let sign_pos = Position::new(5, 4);
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

        let teleport_pos = Position::new(5, 4);
        map.add_event(
            teleport_pos,
            MapEvent::Teleport {
                name: "TestPortal".to_string(),
                description: "Portal to destination".to_string(),
                destination: Position::new(2, 2),
                map_id: 1,
            },
        );

        // Act
        let event = map.get_event(teleport_pos);

        // Assert
        assert!(event.is_some());
        assert!(matches!(event, Some(MapEvent::Teleport { .. })));
    }

    /// Test that door interaction state changes correctly.
    /// Validates the door opening mechanism by checking wall type transitions.
    #[test]
    fn test_door_interaction_wall_state() {
        // Arrange
        let mut map =
            crate::domain::world::Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);

        let door_pos = Position::new(5, 4);
        if let Some(tile) = map.get_tile_mut(door_pos) {
            tile.wall_type = WallType::Door;
        }

        // Act - verify initial state
        let tile_before = map.get_tile(door_pos).expect("tile missing");
        assert_eq!(tile_before.wall_type, WallType::Door);

        // Act - open door by changing wall type
        if let Some(tile) = map.get_tile_mut(door_pos) {
            tile.wall_type = WallType::None;
        }

        // Assert - verify final state
        let tile_after = map.get_tile(door_pos).expect("tile missing");
        assert_eq!(tile_after.wall_type, WallType::None);
    }

    /// Test that NPC placements are properly stored and retrievable.
    /// Validates the NPC data structure and storage mechanisms.
    #[test]
    fn test_npc_interaction_placement_storage() {
        // Arrange
        let mut map =
            crate::domain::world::Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);

        let npc_pos = Position::new(5, 4);
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

        let recruit_pos = Position::new(5, 4);
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

        let encounter_pos = Position::new(5, 4);
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

        app.add_message::<DoorOpenedEvent>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<StartDialogue>();
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
            "Party must not move while GameMode::Inventory is active"
        );
    }

    /// Turn-left input must not alter party facing while inventory is open.
    #[test]
    fn test_turn_blocked_in_inventory_mode() {
        let mut app = App::new();

        app.insert_resource(ButtonInput::<KeyCode>::default());

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

        app.add_message::<DoorOpenedEvent>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<StartDialogue>();
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

        // Register message channels that handle_input depends on.
        app.add_message::<DoorOpenedEvent>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<StartDialogue>();
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
            "Party must not move while GameMode::Combat is active"
        );
    }

    #[test]
    fn test_victory_overlay_dismissed_after_party_moves() {
        let mut app = App::new();

        app.insert_resource(ButtonInput::<KeyCode>::default());

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

        app.add_message::<DoorOpenedEvent>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<StartDialogue>();
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
        gs.world.set_party_position(Position::new(5, 5));
        // Default party_facing is North → position_ahead() == (5, 4).

        app.insert_resource(GlobalState(gs));
        app.insert_resource::<bevy::time::Time>(bevy::time::Time::default());
        app.insert_resource(PendingRecruitmentContext::default());

        app.add_message::<DoorOpenedEvent>();
        app.add_message::<MapEventTriggered>();
        app.add_message::<StartDialogue>();
        app.add_message::<InitiateRestEvent>();

        app.add_systems(Update, handle_input);
        app
    }

    /// Spawn a furniture door entity at `position` with the given locked state.
    /// Returns the spawned entity ID.
    fn spawn_door_entity(app: &mut App, position: Position, is_locked: bool) -> Entity {
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
        let door_pos = Position::new(5, 4);
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
        let door_pos = Position::new(5, 4);
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
        let door_pos = Position::new(5, 4);
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
        let door_pos = Position::new(5, 4);
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
        let door_pos = Position::new(5, 4);

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
        let door_pos = Position::new(5, 4);

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
        let door_pos = Position::new(6, 5);
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

        let door_pos = Position::new(5, 4);
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

        let door_pos = Position::new(5, 4);
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
