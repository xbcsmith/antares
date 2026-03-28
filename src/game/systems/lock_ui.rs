// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Lock interaction UI — Pick Lock / Bash prompt.
//!
//! When [`LockInteractionPending`] is set by Phase 2's input or events system,
//! this module renders a small centred egui window prompting the player to
//! choose "Pick Lock" (Robber only) or "Bash" and to select an acting character.
//!
//! `lock_action_system` reads the resulting [`LockActionChosen`] message,
//! dispatches to the domain functions `try_lockpick` / `try_bash`, applies
//! the outcome to the world (opening the door or entering container inventory),
//! and applies trap damage when a trap fires.
//!
//! # Architecture Reference
//!
//! See Phase 3 of
//! `docs/explanation/locked_objects_and_keys_implementation_plan.md`.

use crate::application::resources::GameContent;
use crate::domain::character::Character;
use crate::domain::classes::ClassDatabase;
use crate::domain::types::Position;
use crate::domain::world::lock::{try_bash, try_lockpick, LockState, UnlockOutcome};
use crate::domain::world::MapEvent;
use crate::game::resources::{GlobalState, LockInteractionPending};
use crate::game::systems::ui::GameLog;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

// ─────────────────────────────────────────────────────────────────────────────
// Plugin
// ─────────────────────────────────────────────────────────────────────────────

/// Plugin that registers the lock prompt UI and lock action handler systems.
///
/// Add to the app in `src/bin/antares.rs`:
///
/// ```ignore
/// app.add_plugins(antares::game::systems::lock_ui::LockUiPlugin);
/// ```
pub struct LockUiPlugin;

impl Plugin for LockUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<LockActionChosen>()
            .init_resource::<LockNavState>()
            .add_systems(Update, (lock_prompt_ui_system, lock_action_system).chain());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

/// Emitted when the player selects an action from the lock UI prompt.
///
/// `lock_action_system` reads this message each frame and dispatches to the
/// appropriate domain function (`try_lockpick` or `try_bash`).
///
/// # Examples
///
/// ```
/// use antares::game::systems::lock_ui::{LockAction, LockActionChosen};
/// use antares::domain::types::Position;
///
/// let msg = LockActionChosen {
///     lock_id: "gate_01".to_string(),
///     position: Position::new(3, 5),
///     action: LockAction::Bash,
///     party_index: 0,
/// };
/// assert_eq!(msg.lock_id, "gate_01");
/// assert_eq!(msg.party_index, 0);
/// assert_eq!(msg.action, LockAction::Bash);
/// ```
#[derive(Message, Debug, Clone)]
pub struct LockActionChosen {
    /// Lock identifier matching `MapEvent::LockedDoor::lock_id` or
    /// `MapEvent::LockedContainer::lock_id`.
    pub lock_id: String,
    /// Tile position of the locked object on the current map.
    pub position: Position,
    /// The action the player has chosen.
    pub action: LockAction,
    /// Index of the acting character in the active party (0-based).
    pub party_index: usize,
}

/// The player's chosen action for a lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockAction {
    /// Attempt to pick the lock (Robber class only).
    Lockpick,
    /// Attempt to bash the lock open (any character, no class restriction).
    Bash,
}

/// Keyboard navigation state for the lock prompt UI.
///
/// Initialised and registered as a Bevy resource by [`LockUiPlugin`].
///
/// # Examples
///
/// ```
/// use antares::game::systems::lock_ui::LockNavState;
///
/// let nav = LockNavState::default();
/// assert!(nav.selected_character.is_none());
/// ```
#[derive(Resource, Default, Debug)]
pub struct LockNavState {
    /// Currently selected party member index (0-based), or `None` when no
    /// character has been chosen yet.
    pub selected_character: Option<usize>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Private helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Whether the locked object at the event position is a door or a container.
#[derive(Debug, Clone)]
pub(crate) enum EventKind {
    /// A locked door — opened by clearing the tile's wall type.
    Door,
    /// A locked container — opened by entering `ContainerInventory` mode.
    /// Carries the display name and pre-loaded items of the container.
    Container {
        /// Display name shown in the inventory panel header.
        name: String,
        /// Items pre-loaded in the container (from
        /// `MapEvent::LockedContainer::items`).
        items: Vec<crate::domain::character::InventorySlot>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Systems
// ─────────────────────────────────────────────────────────────────────────────

/// Renders the lock choice prompt when [`LockInteractionPending`] is set.
///
/// Shows a centred `egui::Window` with:
/// - Character selection (keys `1`–`6` or mouse click).
/// - **Pick Lock** button — disabled when `can_lockpick` is `false`.
/// - **Bash** button — always available.
/// - **Cancel** / `Esc` — clears the pending state with no side effect.
///
/// When the player confirms an action with a character selected, emits
/// [`LockActionChosen`] and clears [`LockInteractionPending`].
fn lock_prompt_ui_system(
    mut contexts: EguiContexts,
    mut lock_pending: ResMut<LockInteractionPending>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    global_state: Res<GlobalState>,
    mut nav_state: ResMut<LockNavState>,
    mut action_writer: MessageWriter<LockActionChosen>,
) {
    // Only run when a lock interaction is pending.
    let lock_id = match lock_pending.lock_id.clone() {
        Some(id) => id,
        None => return,
    };
    let position = match lock_pending.position {
        Some(pos) => pos,
        None => return,
    };
    let can_lockpick = lock_pending.can_lockpick;

    // Handle keyboard shortcuts before rendering.
    if let Some(ref kb) = keyboard {
        // Esc cancels the prompt.
        if kb.just_pressed(KeyCode::Escape) {
            *lock_pending = LockInteractionPending::default();
            nav_state.selected_character = None;
            return;
        }

        // Digit keys 1–6 select a party member.
        let digit_keys = [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
        ];
        let member_count = global_state.0.party.members.len();
        for (idx, key) in digit_keys.iter().enumerate() {
            if kb.just_pressed(*key) && idx < member_count {
                nav_state.selected_character = Some(idx);
            }
        }
    }

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    // Collect party names for the character selector.
    let party_names: Vec<String> = global_state
        .0
        .party
        .members
        .iter()
        .map(|m| m.name.clone())
        .collect();

    let mut chosen_action: Option<(LockAction, usize)> = None;
    let mut do_cancel = false;

    egui::Window::new("Locked Object")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("The way is locked. Choose an action:");
            ui.separator();

            // Character selection.
            ui.label("Acting character:");
            for (idx, name) in party_names.iter().enumerate() {
                let selected = nav_state.selected_character == Some(idx);
                if ui
                    .selectable_label(selected, format!("[{}] {}", idx + 1, name))
                    .clicked()
                {
                    nav_state.selected_character = Some(idx);
                }
            }

            ui.separator();

            ui.horizontal(|ui| {
                // Pick Lock — disabled when no party member has the ability.
                ui.add_enabled_ui(can_lockpick, |ui| {
                    if ui.button("Pick Lock  (Robber only)").clicked() {
                        if let Some(party_index) = nav_state.selected_character {
                            chosen_action = Some((LockAction::Lockpick, party_index));
                        }
                    }
                });

                // Bash — available to any character.
                if ui.button("Bash").clicked() {
                    if let Some(party_index) = nav_state.selected_character {
                        chosen_action = Some((LockAction::Bash, party_index));
                    }
                }

                // Cancel.
                if ui.button("Cancel").clicked() {
                    do_cancel = true;
                }
            });

            if nav_state.selected_character.is_none() && !do_cancel {
                ui.separator();
                ui.colored_label(egui::Color32::YELLOW, "Select a character first.");
            }
        });

    if do_cancel {
        *lock_pending = LockInteractionPending::default();
        nav_state.selected_character = None;
        return;
    }

    if let Some((action, party_index)) = chosen_action {
        action_writer.write(LockActionChosen {
            lock_id,
            position,
            action,
            party_index,
        });
        *lock_pending = LockInteractionPending::default();
        nav_state.selected_character = None;
    }
}

/// Reads [`LockActionChosen`] messages and performs the unlock attempt.
///
/// Dispatches to [`handle_lock_action`], writes the updated [`LockState`]
/// back to the map, and then handles every [`UnlockOutcome`] variant:
///
/// | Outcome | Behaviour |
/// |---------|-----------|
/// | `LockpickSuccess` / `BashSuccess` | Open object, log success |
/// | `LockpickFailed` / `BashFailed` | Log failure with new trap chance |
/// | `TrapTriggered` | Apply damage to every party member, log |
///
/// Always clears [`LockInteractionPending`] when done.
fn lock_action_system(
    mut reader: MessageReader<LockActionChosen>,
    mut global_state: ResMut<GlobalState>,
    game_content: Option<Res<GameContent>>,
    mut game_log: Option<ResMut<GameLog>>,
    mut lock_pending: ResMut<LockInteractionPending>,
) {
    for msg in reader.read() {
        let LockActionChosen {
            lock_id,
            position,
            action,
            party_index,
        } = msg;

        // Build class database (falls back to empty when content is absent).
        let empty_class_db = ClassDatabase::new();
        let class_db: &ClassDatabase = game_content
            .as_deref()
            .map(|gc| &gc.db().classes)
            .unwrap_or(&empty_class_db);

        // Clone what we need before any mutable borrows.
        let character: Option<Character> = global_state.0.party.members.get(*party_index).cloned();
        let Some(character) = character else {
            warn!(
                "lock_action_system: party_index {} out of range (party size {})",
                party_index,
                global_state.0.party.members.len()
            );
            continue;
        };

        let lock_state_snap = global_state
            .0
            .world
            .get_current_map()
            .and_then(|m| m.lock_states.get(lock_id.as_str()))
            .cloned();
        let Some(mut lock_state) = lock_state_snap else {
            warn!(
                "lock_action_system: no LockState for lock_id '{}' — \
                 was init_lock_states() called on map load?",
                lock_id
            );
            continue;
        };

        // Determine whether this is a door or a container.
        let event_kind = global_state
            .0
            .world
            .get_current_map()
            .and_then(|m| m.get_event(*position))
            .map(|e| match e {
                MapEvent::LockedContainer { name, items, .. } => EventKind::Container {
                    name: name.clone(),
                    items: items.clone(),
                },
                _ => EventKind::Door,
            })
            .unwrap_or(EventKind::Door);

        let char_name = character.name.clone();
        let has_pick_ability = class_db
            .get_class(&character.class_id)
            .map(|c| c.has_ability("pick_lock"))
            .unwrap_or(false);

        // Run the domain function with thread_rng.
        let outcome = handle_lock_action(
            *action,
            &mut lock_state,
            &character,
            *party_index,
            class_db,
            &mut rand::rng(),
        );

        // Write the updated LockState back to the map.
        if let Some(map) = global_state.0.world.get_current_map_mut() {
            if let Some(ls) = map.lock_states.get_mut(lock_id.as_str()) {
                *ls = lock_state;
            }
        }

        // Apply the outcome to world + party + log.
        match &outcome {
            UnlockOutcome::LockpickSuccess { .. } => {
                let msg = format!("{} picks the lock!", char_name);
                info!("{}", msg);
                if let Some(ref mut log) = game_log {
                    log.add_exploration(msg);
                }
                let extra = apply_success(lock_id, *position, &event_kind, &mut global_state.0);
                for m in extra {
                    info!("{}", m);
                    if let Some(ref mut log) = game_log {
                        log.add_system(m);
                    }
                }
            }
            UnlockOutcome::BashSuccess { .. } => {
                let msg = match &event_kind {
                    EventKind::Container { name, .. } => {
                        format!("{} smashes open the {}!", char_name, name)
                    }
                    EventKind::Door => format!("{} smashes the door open!", char_name),
                };
                info!("{}", msg);
                if let Some(ref mut log) = game_log {
                    log.add_exploration(msg);
                }
                let extra = apply_success(lock_id, *position, &event_kind, &mut global_state.0);
                for m in extra {
                    info!("{}", m);
                    if let Some(ref mut log) = game_log {
                        log.add_system(m);
                    }
                }
            }
            UnlockOutcome::LockpickFailed {
                new_trap_chance, ..
            } => {
                let msg = if !has_pick_ability {
                    "Only a skilled Robber can pick this lock.".to_string()
                } else {
                    format!(
                        "You fail to pick the lock. Trap chance: {}%.",
                        new_trap_chance
                    )
                };
                info!("{}", msg);
                if let Some(ref mut log) = game_log {
                    log.add_exploration(msg);
                }
            }
            UnlockOutcome::BashFailed {
                new_trap_chance, ..
            } => {
                let msg = format!(
                    "You fail to break open the door. Trap chance: {}%.",
                    new_trap_chance
                );
                info!("{}", msg);
                if let Some(ref mut log) = game_log {
                    log.add_exploration(msg);
                }
            }
            UnlockOutcome::TrapTriggered { damage, effect } => {
                let msg = format!("A trap fires! The party takes {} damage!", damage);
                info!("{}", msg);
                if let Some(ref mut log) = game_log {
                    log.add_exploration(msg);
                }
                // Apply damage to every living party member.
                for member in global_state.0.party.members.iter_mut() {
                    member.hp.modify(-(*damage as i32));
                }
                // Apply status-condition / teleport effects (Phase 5).
                let effect_messages = apply_trap_effects(effect.as_deref(), &mut global_state.0);
                for effect_msg in effect_messages {
                    info!("{}", effect_msg);
                    if let Some(ref mut log) = game_log {
                        log.add_system(effect_msg);
                    }
                }
            }
            _ => {
                // OpenedWithKey and Locked are handled upstream before
                // LockActionChosen is emitted; treat them as no-ops here.
                warn!(
                    "lock_action_system: unexpected outcome {:?} for lock '{}'",
                    outcome, lock_id
                );
            }
        }

        // Always clear pending state.
        *lock_pending = LockInteractionPending::default();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pure helpers (testable without a Bevy App)
// ─────────────────────────────────────────────────────────────────────────────

/// Dispatches the player's chosen lock action to the appropriate domain function.
///
/// Extracted from [`lock_action_system`] so that unit tests can inject a seeded
/// RNG and verify deterministic outcomes without a running Bevy `App`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::lock_ui::{handle_lock_action, LockAction};
/// use antares::domain::world::lock::{LockState, UnlockOutcome};
/// use antares::domain::character::{Alignment, Character, Sex};
/// use antares::domain::classes::ClassDatabase;
/// use rand::SeedableRng;
///
/// let mut lock = LockState::new("test_lock");
/// let character = Character::new(
///     "Hero".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Male, Alignment::Good,
/// );
/// let class_db = ClassDatabase::new();
/// let mut rng = rand::rngs::StdRng::seed_from_u64(0);
///
/// let outcome = handle_lock_action(
///     LockAction::Bash, &mut lock, &character, 0, &class_db, &mut rng,
/// );
/// // Knight bash always returns Success, Failed, or TrapTriggered.
/// assert!(matches!(
///     outcome,
///     UnlockOutcome::BashSuccess { .. }
///         | UnlockOutcome::BashFailed { .. }
///         | UnlockOutcome::TrapTriggered { .. }
/// ));
/// ```
pub(crate) fn handle_lock_action<R: rand::Rng>(
    action: LockAction,
    lock_state: &mut LockState,
    character: &Character,
    party_index: usize,
    class_db: &ClassDatabase,
    rng: &mut R,
) -> UnlockOutcome {
    match action {
        LockAction::Lockpick => try_lockpick(lock_state, character, party_index, class_db, rng),
        LockAction::Bash => try_bash(lock_state, character, party_index, rng),
    }
}

/// Apply a successful unlock outcome to the game world.
///
/// Returns a `Vec<String>` of game-log messages to be added by the caller.
///
/// - **Door**: clears `wall_type` → `None`, `blocked` → `false`, removes the
///   `LockedDoor` event.
/// - **Container**: replaces the `LockedContainer` event with an open
///   `Container` event (empty items), then transitions the game mode to
///   `ContainerInventory`.
///
/// The function is `pub(crate)` so that unit tests can call it directly
/// without a running Bevy `App`.
pub(crate) fn apply_success(
    lock_id: &str,
    position: Position,
    event_kind: &EventKind,
    game_state: &mut crate::application::GameState,
) -> Vec<String> {
    use crate::domain::world::WallType;
    let mut messages: Vec<String> = Vec::new();

    match event_kind {
        EventKind::Door => {
            if let Some(map) = game_state.world.get_current_map_mut() {
                if let Some(tile) = map.get_tile_mut(position) {
                    tile.wall_type = WallType::None;
                    tile.blocked = false;
                }
                map.remove_event(position);
            }
        }
        EventKind::Container {
            name: container_name,
            items: container_items,
        } => {
            let name = container_name.clone();
            let id = lock_id.to_string();
            let items = container_items.clone();

            // Step 1 — world mutations (explicit block so borrow ends before step 2).
            {
                if let Some(map) = game_state.world.get_current_map_mut() {
                    // Replace LockedContainer with an open Container seeded
                    // with the items that were locked inside, so the player
                    // can re-access it without re-triggering the prompt.
                    map.add_event(
                        position,
                        MapEvent::Container {
                            id: id.clone(),
                            name: name.clone(),
                            description: String::new(),
                            items: items.clone(),
                        },
                    );
                }
            } // borrow of game_state.world ends here

            // Step 2 — enter ContainerInventory mode with the locked items.
            game_state.enter_container_inventory(id, name.clone(), items);

            messages.push(format!("The {} is unlocked.", name));
        }
    }
    messages
}

/// Applies the status-condition or teleport side-effect produced by a fired trap.
///
/// | `effect`      | Action |
/// |---------------|--------|
/// | `"poison"`    | Applies `Condition::POISONED` to the lead party member (index 0) |
/// | `"paralysis"` | Applies `Condition::PARALYZED` to every party member |
/// | `"teleport"`  | Teleports the party to `Position::new(1, 1)` (safe map-start fallback) |
/// | `None` / other | No-op (damage-only trap or unrecognised effect) |
///
/// Returns a `Vec<String>` of game-log messages so that callers can emit
/// them through the normal logging path.
///
/// Extracted from [`lock_action_system`] as a `pub(crate)` function so that
/// unit tests can exercise condition application without a running Bevy `App`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::lock_ui::apply_trap_effects;
/// use antares::application::GameState;
/// use antares::domain::character::Condition;
///
/// let mut gs = GameState::new();
/// let msgs = apply_trap_effects(Some("poison"), &mut gs);
/// // Party is empty → no condition applied, but no panic either.
/// assert!(msgs.is_empty());
/// ```
pub(crate) fn apply_trap_effects(
    effect: Option<&str>,
    game_state: &mut crate::application::GameState,
) -> Vec<String> {
    use crate::domain::character::Condition;

    let mut messages: Vec<String> = Vec::new();
    let Some(effect_name) = effect else {
        return messages;
    };

    match effect_name {
        "poison" => {
            if let Some(lead) = game_state.party.members.first_mut() {
                lead.conditions.add(Condition::POISONED);
                messages.push(format!("{} has been poisoned by the trap!", lead.name));
            }
        }
        "paralysis" => {
            for member in game_state.party.members.iter_mut() {
                member.conditions.add(Condition::PARALYZED);
            }
            messages.push("The paralytic gas freezes your entire party!".to_string());
        }
        "teleport" => {
            // `Map` has no `starting_position` field; use (1, 1) as the safe
            // fallback per the architecture specification.
            let start = crate::domain::types::Position::new(1, 1);
            game_state.world.set_party_position(start);
            messages.push("The trap teleports your party to the start of the map!".to_string());
        }
        other => {
            messages.push(format!("Effect: {}", other));
        }
    }
    messages
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameMode;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::world::lock::{LockState, UnlockOutcome, BASH_TRAP_INCREMENT};
    use crate::domain::world::{Map, MapEvent, WallType};
    use rand::SeedableRng;

    // ── helpers ─────────────────────────────────────────────────────────────

    /// Build a plain `Character` with the given class for use in tests.
    fn make_character(class_id: &str, level: u32) -> Character {
        let mut c = Character::new(
            "TestHero".to_string(),
            "human".to_string(),
            class_id.to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.level = level;
        c
    }

    /// Build a minimal `GameState` containing a 10×10 map with the given
    /// `MapEvent` at `position`, a matching `LockState` in `lock_states`, and
    /// one party member.
    fn make_game_state_with_event(
        position: Position,
        event: MapEvent,
        lock_id: &str,
        initial_trap_chance: u8,
    ) -> crate::application::GameState {
        let mut gs = crate::application::GameState::new();
        let mut map = Map::new(1, "TestMap".to_string(), "Test".to_string(), 10, 10);
        map.add_event(position, event);
        map.lock_states
            .insert(lock_id.to_string(), LockState::new(lock_id));
        if initial_trap_chance > 0 {
            if let Some(ls) = map.lock_states.get_mut(lock_id) {
                ls.trap_chance = initial_trap_chance;
            }
        }
        // Place a door tile so the door-opening logic has something to clear.
        if let Some(tile) = map.get_tile_mut(position) {
            tile.wall_type = WallType::Door;
            tile.blocked = true;
        }
        gs.world.add_map(map);
        gs.world.set_current_map(1);
        gs.world.set_party_position(Position::new(0, 0));
        let hero = make_character("knight", 5);
        gs.party.add_member(hero).expect("party should not be full");
        gs
    }

    // ────────────────────────────────────────────────────────────────────────
    // §3.4.1 — LockActionChosen fields
    // ────────────────────────────────────────────────────────────────────────

    /// Verify that `LockActionChosen` stores all fields correctly.
    #[test]
    fn test_lock_action_chosen_fields() {
        let msg = LockActionChosen {
            lock_id: "gate_01".to_string(),
            position: Position::new(3, 5),
            action: LockAction::Bash,
            party_index: 2,
        };
        assert_eq!(msg.lock_id, "gate_01");
        assert_eq!(msg.position, Position::new(3, 5));
        assert_eq!(msg.action, LockAction::Bash);
        assert_eq!(msg.party_index, 2);
    }

    /// `LockAction` variants are distinct.
    #[test]
    fn test_lock_action_variants_distinct() {
        assert_ne!(LockAction::Lockpick, LockAction::Bash);
        assert_eq!(LockAction::Bash, LockAction::Bash);
        assert_eq!(LockAction::Lockpick, LockAction::Lockpick);
    }

    // ────────────────────────────────────────────────────────────────────────
    // §3.4.2 — Bash success opens door tile
    // ────────────────────────────────────────────────────────────────────────

    /// When bash succeeds, `apply_success` sets the door tile to
    /// `WallType::None` and `blocked = false`.
    #[test]
    fn test_lock_action_system_bash_success_opens_door() {
        let lock_id = "door_test_01";
        let position = Position::new(4, 4);

        // Use a seeded RNG and a high-level character to find a success outcome.
        let class_db = ClassDatabase::new();
        let character = make_character("knight", 20); // (25 + 60).clamp(5,80) = 80 % bash
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);

        let mut saw_success = false;
        for _ in 0..200 {
            let mut ls = LockState::new(lock_id);
            if let UnlockOutcome::BashSuccess { .. } = handle_lock_action(
                LockAction::Bash,
                &mut ls,
                &character,
                0,
                &class_db,
                &mut rng,
            ) {
                saw_success = true;
                break;
            }
        }
        assert!(
            saw_success,
            "a level-20 knight should bash-succeed within 200 tries"
        );

        // Now verify apply_success actually opens the door tile.
        let event = MapEvent::LockedDoor {
            name: "Test Door".to_string(),
            lock_id: lock_id.to_string(),
            key_item_id: None,
            initial_trap_chance: 0,
        };
        let mut gs = make_game_state_with_event(position, event, lock_id, 0);

        let messages = apply_success(lock_id, position, &EventKind::Door, &mut gs);

        // Tile must be opened.
        let tile = gs
            .world
            .get_current_map()
            .unwrap()
            .get_tile(position)
            .unwrap();
        assert_eq!(
            tile.wall_type,
            WallType::None,
            "apply_success must set WallType::None on door success"
        );
        assert!(
            !tile.blocked,
            "apply_success must clear blocked flag on door success"
        );

        // Event must be removed.
        assert!(
            gs.world
                .get_current_map()
                .unwrap()
                .get_event(position)
                .is_none(),
            "LockedDoor event must be removed after door success"
        );

        // apply_success returns no messages for doors.
        assert!(
            messages.is_empty(),
            "door success produces no extra log messages"
        );
    }

    // ────────────────────────────────────────────────────────────────────────
    // §3.4.3 — Bash failure logs message with trap chance
    // ────────────────────────────────────────────────────────────────────────

    /// When bash fails, the trap chance is incremented and the failure message
    /// format is correct.
    #[test]
    fn test_lock_action_system_bash_failure_logs_message() {
        // Level-1 knight: (25 + 3).clamp(5,80) = 28 % success → ~72 % failure.
        let class_db = ClassDatabase::new();
        let character = make_character("knight", 1);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);

        let mut saw_failure = false;
        let mut failure_trap_chance: u8 = 0;

        for _ in 0..200 {
            let mut ls = LockState::new("fail_door");
            match handle_lock_action(
                LockAction::Bash,
                &mut ls,
                &character,
                0,
                &class_db,
                &mut rng,
            ) {
                UnlockOutcome::BashFailed {
                    new_trap_chance, ..
                } => {
                    assert_eq!(
                        new_trap_chance, BASH_TRAP_INCREMENT,
                        "first bash failure must set trap_chance to BASH_TRAP_INCREMENT"
                    );
                    failure_trap_chance = new_trap_chance;
                    saw_failure = true;
                    break;
                }
                _ => continue,
            }
        }
        assert!(
            saw_failure,
            "level-1 knight should bash-fail within 200 tries"
        );

        // Verify the expected message format.
        let expected_msg = format!(
            "You fail to break open the door. Trap chance: {}%.",
            failure_trap_chance
        );
        assert!(
            expected_msg.contains("Trap chance:"),
            "failure message must contain 'Trap chance:'"
        );
        assert!(
            expected_msg.contains('%'),
            "failure message must contain a '%' sign"
        );
    }

    // ────────────────────────────────────────────────────────────────────────
    // §3.4.4 — Lockpick with no ability logs rejection
    // ────────────────────────────────────────────────────────────────────────

    /// A knight (no `pick_lock` ability) always gets `LockpickFailed` and
    /// `has_pick_ability` is `false`, producing the rejection message.
    #[test]
    fn test_lock_action_system_lockpick_no_ability_logs_rejection() {
        let class_db = ClassDatabase::new(); // empty — no abilities defined
        let character = make_character("knight", 10);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let mut lock = LockState::new("no_ability_lock");

        let outcome = handle_lock_action(
            LockAction::Lockpick,
            &mut lock,
            &character,
            0,
            &class_db,
            &mut rng,
        );

        // Must always fail for a knight with an empty class DB.
        assert!(
            matches!(outcome, UnlockOutcome::LockpickFailed { .. }),
            "knight with no class DB must always get LockpickFailed"
        );

        // has_pick_ability must be false (no class entry → unwrap_or(false)).
        let has_pick_ability = class_db
            .get_class(&character.class_id)
            .map(|c| c.has_ability("pick_lock"))
            .unwrap_or(false);
        assert!(
            !has_pick_ability,
            "has_pick_ability must be false for knight in empty class DB"
        );

        // The message for no-ability is "Only a skilled Robber…"
        let msg = if !has_pick_ability {
            "Only a skilled Robber can pick this lock.".to_string()
        } else if let UnlockOutcome::LockpickFailed {
            new_trap_chance, ..
        } = outcome
        {
            format!(
                "You fail to pick the lock. Trap chance: {}%.",
                new_trap_chance
            )
        } else {
            String::new()
        };
        assert_eq!(
            msg, "Only a skilled Robber can pick this lock.",
            "rejection message must be the Robber-only notice"
        );
    }

    // ────────────────────────────────────────────────────────────────────────
    // §3.4.5 — Trap triggered damages party
    // ────────────────────────────────────────────────────────────────────────

    /// With `trap_chance == 100` the trap always fires before any bash attempt.
    /// Party HP must decrease.
    #[test]
    fn test_lock_action_system_trap_triggered_damages_party() {
        let class_db = ClassDatabase::new();
        let character = make_character("knight", 5);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);

        let mut lock_state = LockState {
            lock_id: "trap_door".to_string(),
            is_locked: true,
            trap_chance: 100, // always fires
        };

        let outcome = handle_lock_action(
            LockAction::Bash,
            &mut lock_state,
            &character,
            0,
            &class_db,
            &mut rng,
        );

        assert!(
            matches!(outcome, UnlockOutcome::TrapTriggered { .. }),
            "trap_chance == 100 must always produce TrapTriggered; got {:?}",
            outcome
        );

        // Apply trap damage to a test party.
        if let UnlockOutcome::TrapTriggered { damage, .. } = outcome {
            let mut party = crate::domain::character::Party::new();
            let mut hero = make_character("knight", 5);
            hero.hp = crate::domain::character::AttributePair16::new(50);
            party.members.push(hero);

            let initial_hp = party.members[0].hp.current;
            for member in party.members.iter_mut() {
                member.hp.modify(-(damage as i32));
            }
            assert!(
                party.members[0].hp.current < initial_hp,
                "trap damage must reduce party HP; before={}, after={}",
                initial_hp,
                party.members[0].hp.current
            );
        }
    }

    // ────────────────────────────────────────────────────────────────────────
    // §3.4.6 — Container success enters ContainerInventory mode
    // ────────────────────────────────────────────────────────────────────────

    /// When bash/lockpick succeeds on a `LockedContainer`, `apply_success`
    /// transitions the game mode to `ContainerInventory`.
    #[test]
    fn test_lock_action_system_container_success_enters_container_mode() {
        let lock_id = "chest_01";
        let position = Position::new(3, 3);
        let container_name = "Iron Chest";

        let event = MapEvent::LockedContainer {
            name: container_name.to_string(),
            lock_id: lock_id.to_string(),
            key_item_id: None,
            items: vec![],
            initial_trap_chance: 0,
        };
        let mut gs = make_game_state_with_event(position, event, lock_id, 0);

        let messages = apply_success(
            lock_id,
            position,
            &EventKind::Container {
                name: container_name.to_string(),
                items: vec![],
            },
            &mut gs,
        );

        // Game mode must be ContainerInventory.
        assert!(
            matches!(gs.mode, GameMode::ContainerInventory(_)),
            "game mode must be ContainerInventory after container success; got {:?}",
            gs.mode
        );

        // A message about the container being unlocked must be returned.
        assert!(
            messages.iter().any(|m| m.contains("unlocked")),
            "apply_success must return an unlock message for containers; got: {:?}",
            messages
        );

        // The LockedContainer event must have been replaced with a Container event.
        let replacement_event = gs.world.get_current_map().unwrap().get_event(position);
        assert!(
            matches!(replacement_event, Some(MapEvent::Container { .. })),
            "LockedContainer must be replaced with Container after success"
        );
    }

    // ────────────────────────────────────────────────────────────────────────
    // Additional coverage
    // ────────────────────────────────────────────────────────────────────────

    /// `LockNavState` defaults to no selected character.
    #[test]
    fn test_lock_nav_state_default() {
        let nav = LockNavState::default();
        assert!(nav.selected_character.is_none());
    }

    /// `apply_success` for a door removes the `LockedDoor` event.
    #[test]
    fn test_apply_success_door_removes_event() {
        let lock_id = "rm_event_lock";
        let position = Position::new(2, 2);
        let event = MapEvent::LockedDoor {
            name: "Test".to_string(),
            lock_id: lock_id.to_string(),
            key_item_id: None,
            initial_trap_chance: 0,
        };
        let mut gs = make_game_state_with_event(position, event, lock_id, 0);

        // Confirm event exists before.
        assert!(gs
            .world
            .get_current_map()
            .unwrap()
            .get_event(position)
            .is_some());

        apply_success(lock_id, position, &EventKind::Door, &mut gs);

        // Event must be gone.
        assert!(
            gs.world
                .get_current_map()
                .unwrap()
                .get_event(position)
                .is_none(),
            "LockedDoor event must be removed after apply_success"
        );
    }

    /// `apply_success` for a container replaces the event with an open Container.
    #[test]
    fn test_apply_success_container_replaces_event() {
        let lock_id = "replace_lock";
        let position = Position::new(5, 5);
        let event = MapEvent::LockedContainer {
            name: "Old Chest".to_string(),
            lock_id: lock_id.to_string(),
            key_item_id: None,
            items: vec![],
            initial_trap_chance: 0,
        };
        let mut gs = make_game_state_with_event(position, event, lock_id, 0);

        apply_success(
            lock_id,
            position,
            &EventKind::Container {
                name: "Old Chest".to_string(),
                items: vec![],
            },
            &mut gs,
        );

        let replacement = gs.world.get_current_map().unwrap().get_event(position);

        match replacement {
            Some(MapEvent::Container { id, .. }) => {
                assert_eq!(id, lock_id, "replacement Container id must match lock_id");
            }
            other => panic!(
                "expected MapEvent::Container after container success, got {:?}",
                other
            ),
        }
    }

    /// Items stored inside a `LockedContainer` are passed through to
    /// `ContainerInventory` mode (and into the replacement `Container` event)
    /// after a successful unlock via `apply_success`.
    ///
    /// This is the regression test for the Phase 1 gap where `items` was
    /// missing from `MapEvent::LockedContainer` and `apply_success` always
    /// passed `vec![]`.
    #[test]
    fn test_apply_success_container_items_passed_through() {
        use crate::application::GameMode;
        use crate::domain::character::InventorySlot;

        let lock_id = "items_lock";
        let position = Position::new(6, 6);

        // Two items pre-loaded in the container.
        let chest_items = vec![
            InventorySlot {
                item_id: 50,
                charges: 0,
            },
            InventorySlot {
                item_id: 51,
                charges: 0,
            },
        ];

        let event = MapEvent::LockedContainer {
            name: "Treasure Chest".to_string(),
            lock_id: lock_id.to_string(),
            key_item_id: None,
            items: chest_items.clone(),
            initial_trap_chance: 0,
        };
        let mut gs = make_game_state_with_event(position, event, lock_id, 0);

        apply_success(
            lock_id,
            position,
            &EventKind::Container {
                name: "Treasure Chest".to_string(),
                items: chest_items.clone(),
            },
            &mut gs,
        );

        // 1. Game mode must be ContainerInventory.
        assert!(
            matches!(gs.mode, GameMode::ContainerInventory(_)),
            "game mode must be ContainerInventory after container unlock; got {:?}",
            gs.mode
        );

        // 2. ContainerInventory state must contain both items.
        if let GameMode::ContainerInventory(ref state) = gs.mode {
            assert_eq!(
                state.items.len(),
                2,
                "ContainerInventory must hold both locked items; got {} items",
                state.items.len()
            );
            assert_eq!(state.items[0].item_id, 50);
            assert_eq!(state.items[1].item_id, 51);
        }

        // 3. The replacement Container event on the map must also carry both items.
        let replacement = gs.world.get_current_map().unwrap().get_event(position);
        match replacement {
            Some(MapEvent::Container { items, .. }) => {
                assert_eq!(
                    items.len(),
                    2,
                    "replacement Container event must carry both items; got {}",
                    items.len()
                );
                assert_eq!(items[0].item_id, 50);
                assert_eq!(items[1].item_id, 51);
            }
            other => panic!(
                "expected MapEvent::Container with items after container success, got {:?}",
                other
            ),
        }
    }

    /// `handle_lock_action` for `Bash` on a lock with `trap_chance == 0`
    /// always returns `BashSuccess` or `BashFailed` (never `TrapTriggered`).
    #[test]
    fn test_handle_lock_action_bash_no_trap_when_zero_chance() {
        let class_db = ClassDatabase::new();
        let character = make_character("knight", 1);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        for _ in 0..50 {
            let mut ls = LockState::new("notrap");
            let outcome = handle_lock_action(
                LockAction::Bash,
                &mut ls,
                &character,
                0,
                &class_db,
                &mut rng,
            );
            assert!(
                !matches!(outcome, UnlockOutcome::TrapTriggered { .. }),
                "no trap should trigger when trap_chance == 0"
            );
        }
    }

    /// `handle_lock_action` for `Lockpick` with an empty class DB always
    /// returns `LockpickFailed` (ability check fails).
    #[test]
    fn test_handle_lock_action_lockpick_no_class_db_always_fails() {
        let class_db = ClassDatabase::new(); // empty
        let character = make_character("robber", 10);
        let mut rng = rand::rngs::StdRng::seed_from_u64(99);

        let mut lock = LockState::new("empty_class_lock");
        let outcome = handle_lock_action(
            LockAction::Lockpick,
            &mut lock,
            &character,
            0,
            &class_db,
            &mut rng,
        );

        // Empty class DB → has_ability returns false → always LockpickFailed.
        assert!(
            matches!(outcome, UnlockOutcome::LockpickFailed { .. }),
            "empty class DB must cause LockpickFailed for any class"
        );
    }

    // ────────────────────────────────────────────────────────────────────────
    // Phase 5 — Trap condition-effect tests
    // ────────────────────────────────────────────────────────────────────────

    /// A `"poison"` trap effect applies `Condition::POISONED` to the lead
    /// party member only.
    #[test]
    fn test_trap_poison_effect_applies_condition_to_lead_character() {
        use crate::domain::character::Condition;

        let lock_id = "poison_trap_lock";
        let position = Position::new(2, 2);
        let event = MapEvent::LockedDoor {
            name: "Poison Door".to_string(),
            lock_id: lock_id.to_string(),
            key_item_id: None,
            initial_trap_chance: 0,
        };
        let mut gs = make_game_state_with_event(position, event, lock_id, 0);

        // Add a second party member so we can verify only the lead is poisoned.
        let second = make_character("knight", 3);
        gs.party.add_member(second).unwrap();

        let msgs = apply_trap_effects(Some("poison"), &mut gs);

        assert!(
            gs.party.members[0].conditions.has(Condition::POISONED),
            "lead character must be poisoned after 'poison' trap effect"
        );
        assert!(
            !gs.party.members[1].conditions.has(Condition::POISONED),
            "second party member must NOT be poisoned — only the lead is affected"
        );
        assert!(
            msgs.iter().any(|m| m.contains("poisoned")),
            "apply_trap_effects must return a poison message; got: {:?}",
            msgs
        );
    }

    /// A `"paralysis"` trap effect applies `Condition::PARALYZED` to every
    /// party member.
    #[test]
    fn test_trap_paralysis_effect_applies_to_all_party_members() {
        use crate::domain::character::Condition;

        let lock_id = "paralysis_trap_lock";
        let position = Position::new(3, 3);
        let event = MapEvent::LockedDoor {
            name: "Paralysis Door".to_string(),
            lock_id: lock_id.to_string(),
            key_item_id: None,
            initial_trap_chance: 0,
        };
        let mut gs = make_game_state_with_event(position, event, lock_id, 0);

        // Add a second party member.
        let second = make_character("sorcerer", 2);
        gs.party.add_member(second).unwrap();

        let msgs = apply_trap_effects(Some("paralysis"), &mut gs);

        for (idx, member) in gs.party.members.iter().enumerate() {
            assert!(
                member.conditions.has(Condition::PARALYZED),
                "party member {} ({}) must be paralyzed after 'paralysis' trap",
                idx,
                member.name
            );
        }
        assert!(
            msgs.iter()
                .any(|m| m.contains("paralytic") || m.contains("paralyz")),
            "apply_trap_effects must return a paralysis message; got: {:?}",
            msgs
        );
    }

    /// A `"teleport"` trap effect moves the party position to `(1, 1)`.
    #[test]
    fn test_trap_teleport_effect_moves_party_to_start() {
        let lock_id = "teleport_trap_lock";
        let position = Position::new(4, 4);
        let event = MapEvent::LockedDoor {
            name: "Teleport Door".to_string(),
            lock_id: lock_id.to_string(),
            key_item_id: None,
            initial_trap_chance: 0,
        };
        let mut gs = make_game_state_with_event(position, event, lock_id, 0);

        // Place party far from (1, 1) to verify the teleport effect.
        gs.world.set_party_position(Position::new(8, 8));
        assert_eq!(
            gs.world.party_position,
            Position::new(8, 8),
            "pre-condition: party must start at (8, 8)"
        );

        let msgs = apply_trap_effects(Some("teleport"), &mut gs);

        assert_eq!(
            gs.world.party_position,
            Position::new(1, 1),
            "party must be teleported to (1, 1) by a teleport trap"
        );
        assert!(
            msgs.iter()
                .any(|m| m.contains("teleport") || m.contains("start")),
            "apply_trap_effects must return a teleport message; got: {:?}",
            msgs
        );
    }
}
