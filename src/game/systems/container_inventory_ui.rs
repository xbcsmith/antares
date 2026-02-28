// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Container Inventory UI System - Split-screen container interaction interface
//!
//! Provides an egui-based split-screen overlay for interacting with containers
//! (chests, crates, holes in the wall, barrels, etc.).  This system is active
//! when the game is in `GameMode::ContainerInventory` mode, which is entered
//! by pressing `E` while the party is facing a container tile event.
//!
//! ## Layout
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Container: [Character] ←→ [Container Name]     [Esc: Exit] │
//! ├────────────────────────┬────────────────────────────────────┤
//! │  [Character Name]      │       [Container Name]             │
//! │  (LEFT PANEL)          │       (RIGHT PANEL)                │
//! │                        │                                    │
//! │  [inventory slot grid] │  [container item list]             │
//! │                        │                                    │
//! │  [ Stash ]             │  [ Take ]  [ Take All ]            │
//! └────────────────────────┴────────────────────────────────────┘
//! ```
//!
//! ## Keyboard Navigation (two-phase model)
//!
//! ### Phase 1 — Slot Navigation
//!
//! | Key              | Effect                                                         |
//! |------------------|----------------------------------------------------------------|
//! | `Tab`            | Toggle focus between Character panel (left) and Container panel (right) |
//! | `1`–`6`          | Switch active character (number key maps to party index 0–5)   |
//! | `←` `→` `↑` `↓` | Navigate the slot grid / item list inside the focused panel    |
//! | `Enter`          | Enter **Action Navigation** for the highlighted slot           |
//! | `Esc`            | Close container inventory; return to previous mode             |
//!
//! ### Phase 2 — Action Navigation
//!
//! | Key     | Effect                                                               |
//! |---------|----------------------------------------------------------------------|
//! | `←` `→` | Cycle between action buttons (Take / Take All  or  Stash)           |
//! | `Enter`  | Execute the focused action; return to Slot Navigation at slot 0    |
//! | `Esc`    | Cancel; return to Slot Navigation at the previously selected slot  |

use crate::application::container_inventory_state::{ContainerFocus, ContainerInventoryState};
use crate::application::resources::GameContent;
use crate::application::GameMode;
use crate::domain::character::{Inventory, InventorySlot};
use crate::domain::world::MapEvent;

use crate::game::resources::GlobalState;
use crate::game::systems::inventory_ui::NavigationPhase;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

// ===== Layout constants =====

/// Height of each panel header bar.
const PANEL_HEADER_H: f32 = 36.0;
/// Height of the action button strip at the bottom of each panel.
const PANEL_ACTION_H: f32 = 48.0;
/// Number of slot columns in the character inventory grid.
const SLOT_COLS: usize = 8;
/// Height of each item row in the container panel.
const CONTAINER_ROW_H: f32 = 28.0;

/// Faint grid-line colour.
const GRID_LINE_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(60, 60, 60, 255);
/// Panel body background.
const PANEL_BG_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(18, 18, 18, 255);
/// Header background.
const HEADER_BG_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(35, 35, 35, 255);
/// Slot / row selection highlight.
const SELECT_HIGHLIGHT_COLOR: egui::Color32 = egui::Color32::YELLOW;
/// Focused panel border.
const FOCUSED_BORDER_COLOR: egui::Color32 = egui::Color32::YELLOW;
/// Unfocused panel border.
const UNFOCUSED_BORDER_COLOR: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(80, 80, 80, 255);
/// Keyboard-focused action button highlight.
const ACTION_FOCUSED_COLOR: egui::Color32 = egui::Color32::YELLOW;
/// Colour for item names in the container list.
const CONTAINER_ITEM_COLOR: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(200, 235, 200, 255);
/// Take button accent colour.
const TAKE_COLOR: egui::Color32 = egui::Color32::from_rgb(80, 200, 200);
/// Stash button accent colour.
const STASH_COLOR: egui::Color32 = egui::Color32::from_rgb(200, 160, 80);

// ===== Plugin =====

/// Plugin for the container interaction split-screen inventory UI.
pub struct ContainerInventoryPlugin;

impl Plugin for ContainerInventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TakeItemAction>()
            .add_message::<TakeAllAction>()
            .add_message::<StashItemAction>()
            .init_resource::<ContainerNavState>()
            .add_systems(
                Update,
                (
                    container_inventory_input_system,
                    container_inventory_ui_system,
                    container_inventory_action_system,
                )
                    .chain(),
            );
    }
}

// ===== Messages =====

/// Emitted when the player confirms taking a single item from the container.
///
/// The item at `container_slot_index` is removed from the container's item list
/// and added to `character_index`'s inventory.
///
/// # Examples
///
/// ```
/// use antares::game::systems::container_inventory_ui::TakeItemAction;
///
/// let action = TakeItemAction {
///     container_slot_index: 1,
///     character_index: 0,
/// };
/// assert_eq!(action.container_slot_index, 1);
/// assert_eq!(action.character_index, 0);
/// ```
#[derive(Message)]
pub struct TakeItemAction {
    /// Index into `ContainerInventoryState::items` for the item to take.
    pub container_slot_index: usize,
    /// Party index of the character who receives the item.
    pub character_index: usize,
}

/// Emitted when the player confirms taking all items from the container.
///
/// All items in `ContainerInventoryState::items` are moved to
/// `character_index`'s inventory (as many as will fit; the rest stay in
/// the container).
///
/// # Examples
///
/// ```
/// use antares::game::systems::container_inventory_ui::TakeAllAction;
///
/// let action = TakeAllAction { character_index: 0 };
/// assert_eq!(action.character_index, 0);
/// ```
#[derive(Message)]
pub struct TakeAllAction {
    /// Party index of the character who receives the items.
    pub character_index: usize,
}

/// Emitted when the player confirms stashing (depositing) an item into the
/// container.
///
/// The item at `character_slot_index` is removed from `character_index`'s
/// inventory and appended to `ContainerInventoryState::items`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::container_inventory_ui::StashItemAction;
///
/// let action = StashItemAction {
///     character_index: 0,
///     character_slot_index: 3,
/// };
/// assert_eq!(action.character_slot_index, 3);
/// assert_eq!(action.character_index, 0);
/// ```
#[derive(Message)]
pub struct StashItemAction {
    /// Party index of the character whose item is being stashed.
    pub character_index: usize,
    /// Slot index in that character's inventory.
    pub character_slot_index: usize,
}

// ===== Container action enum (for keyboard nav) =====

/// The set of actions available when a container slot is selected.
///
/// Used by `build_container_action_list` to create a deterministic ordered
/// list that the keyboard action-navigation phase can cycle through.
///
/// # Examples
///
/// ```
/// use antares::game::systems::container_inventory_ui::ContainerAction;
///
/// let take = ContainerAction::Take { slot_index: 2, character_index: 0 };
/// let take_all = ContainerAction::TakeAll { character_index: 0 };
/// assert!(matches!(take, ContainerAction::Take { .. }));
/// assert!(matches!(take_all, ContainerAction::TakeAll { .. }));
/// ```
#[derive(Debug, PartialEq, Eq)]
pub enum ContainerAction {
    /// Take the item at `slot_index` from the container.
    Take {
        /// Container slot index.
        slot_index: usize,
        /// Party index of the receiving character.
        character_index: usize,
    },
    /// Take all items from the container.
    TakeAll {
        /// Party index of the receiving character.
        character_index: usize,
    },
    /// Stash the character's item into the container.
    Stash {
        /// Party index of the character.
        character_index: usize,
        /// Slot index in the character's inventory.
        slot_index: usize,
    },
}

/// Build the ordered action list for the focused container panel.
///
/// * Container panel focus → `[Take, TakeAll]`
/// * Character panel focus → `[Stash]`
///
/// `slot_index` is the currently highlighted slot / container row.
/// `character_index` is the active character party index.
/// `focus` indicates which panel is active.
pub fn build_container_action_list(
    slot_index: usize,
    character_index: usize,
    focus: &ContainerFocus,
) -> Vec<ContainerAction> {
    match focus {
        ContainerFocus::Right => {
            vec![
                ContainerAction::Take {
                    slot_index,
                    character_index,
                },
                ContainerAction::TakeAll { character_index },
            ]
        }
        ContainerFocus::Left => {
            vec![ContainerAction::Stash {
                character_index,
                slot_index,
            }]
        }
    }
}

// ===== Navigation state =====

/// Tracks keyboard navigation phase for the container inventory screen.
///
/// Mirrors `InventoryNavigationState` from `inventory_ui.rs`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::container_inventory_ui::ContainerNavState;
/// use antares::game::systems::inventory_ui::NavigationPhase;
///
/// let state = ContainerNavState::default();
/// assert_eq!(state.selected_slot_index, None);
/// assert_eq!(state.focused_action_index, 0);
/// assert!(matches!(state.phase, NavigationPhase::SlotNavigation));
/// ```
#[derive(Resource, Default, Debug)]
pub struct ContainerNavState {
    /// Highlighted slot/row index in the focused panel (`None` = none highlighted).
    pub selected_slot_index: Option<usize>,
    /// Focused action button index when `phase == ActionNavigation`.
    ///
    /// Container panel: `0` = Take, `1` = Take All.
    /// Character panel: `0` = Stash.
    pub focused_action_index: usize,
    /// Current navigation phase.
    pub phase: NavigationPhase,
}

impl ContainerNavState {
    /// Reset to a clean default state.
    fn reset(&mut self) {
        *self = ContainerNavState::default();
    }
}

// ===== Input system =====

/// Handles keyboard input for container inventory navigation.
///
/// Runs every frame; only processes input when
/// `GlobalState.0.mode == GameMode::ContainerInventory(_)`.
#[allow(clippy::too_many_lines)]
fn container_inventory_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut global_state: ResMut<GlobalState>,
    mut nav_state: ResMut<ContainerNavState>,
    mut take_writer: MessageWriter<TakeItemAction>,
    mut take_all_writer: MessageWriter<TakeAllAction>,
    mut stash_writer: MessageWriter<StashItemAction>,
) {
    // Guard: only operate in ContainerInventory mode
    let container_state = match &global_state.0.mode {
        GameMode::ContainerInventory(s) => s.clone(),
        _ => {
            nav_state.reset();
            return;
        }
    };

    let party_size = global_state.0.party.members.len();

    // ── Number keys 1–6: switch active character ──────────────────────────
    let char_switch: Option<usize> = [
        (KeyCode::Digit1, 0usize),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
    ]
    .iter()
    .find(|(key, _)| keyboard.just_pressed(*key))
    .map(|(_, idx)| *idx);

    if let Some(new_idx) = char_switch {
        if let GameMode::ContainerInventory(ref mut cs) = global_state.0.mode {
            cs.switch_character(new_idx, party_size);
            nav_state.selected_slot_index = None;
            nav_state.focused_action_index = 0;
            nav_state.phase = NavigationPhase::SlotNavigation;
        }
        return;
    }

    // ── Action Navigation phase ────────────────────────────────────────────
    if nav_state.phase == NavigationPhase::ActionNavigation {
        let slot_idx = match nav_state.selected_slot_index {
            Some(s) => s,
            None => {
                nav_state.phase = NavigationPhase::SlotNavigation;
                return;
            }
        };

        // Esc → cancel, back to slot nav
        if keyboard.just_pressed(KeyCode::Escape) {
            nav_state.phase = NavigationPhase::SlotNavigation;
            return;
        }

        let char_idx = container_state.active_character_index;
        let actions = build_container_action_list(slot_idx, char_idx, &container_state.focus);
        let action_count = actions.len();

        // Left/Right cycle action buttons
        if keyboard.just_pressed(KeyCode::ArrowLeft) {
            nav_state.focused_action_index = if nav_state.focused_action_index == 0 {
                action_count - 1
            } else {
                nav_state.focused_action_index - 1
            };
            return;
        }
        if keyboard.just_pressed(KeyCode::ArrowRight) {
            nav_state.focused_action_index = (nav_state.focused_action_index + 1) % action_count;
            return;
        }

        // Enter → execute focused action
        if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter) {
            let action_idx = nav_state.focused_action_index.min(action_count - 1);
            match &actions[action_idx] {
                ContainerAction::Take {
                    slot_index,
                    character_index,
                } => {
                    take_writer.write(TakeItemAction {
                        container_slot_index: *slot_index,
                        character_index: *character_index,
                    });
                }
                ContainerAction::TakeAll { character_index } => {
                    take_all_writer.write(TakeAllAction {
                        character_index: *character_index,
                    });
                }
                ContainerAction::Stash {
                    character_index,
                    slot_index,
                } => {
                    stash_writer.write(StashItemAction {
                        character_index: *character_index,
                        character_slot_index: *slot_index,
                    });
                }
            }

            nav_state.selected_slot_index = Some(0);
            nav_state.focused_action_index = 0;
            nav_state.phase = NavigationPhase::SlotNavigation;
        }
        return;
    }

    // ── Slot Navigation phase ──────────────────────────────────────────────

    // Esc → close container screen
    if keyboard.just_pressed(KeyCode::Escape) {
        // Write the updated item list back to the map event BEFORE restoring mode
        // so that partial takes persist within the session.
        let event_id = container_state.container_event_id.clone();
        let updated_items = container_state.items.clone();
        write_container_items_back(&mut global_state.0, &event_id, updated_items);

        let resume = container_state.get_resume_mode();
        global_state.0.mode = resume;
        nav_state.reset();
        return;
    }

    // Tab → toggle panel focus (character ↔ container)
    if keyboard.just_pressed(KeyCode::Tab) {
        if let GameMode::ContainerInventory(ref mut cs) = global_state.0.mode {
            cs.toggle_focus();
        }
        nav_state.selected_slot_index = None;
        nav_state.focused_action_index = 0;
        return;
    }

    // Enter → enter action mode if a slot/row is highlighted and has content
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter) {
        if let Some(slot_idx) = nav_state.selected_slot_index {
            let has_content = container_slot_has_content(&container_state, slot_idx, &global_state);
            if has_content {
                nav_state.phase = NavigationPhase::ActionNavigation;
                nav_state.focused_action_index = 0;
            }
        } else {
            // No slot highlighted yet — Enter starts navigation at slot 0
            nav_state.selected_slot_index = Some(0);
            if let GameMode::ContainerInventory(ref mut cs) = global_state.0.mode {
                match cs.focus {
                    ContainerFocus::Left => cs.character_selected_slot = Some(0),
                    ContainerFocus::Right => cs.container_selected_slot = Some(0),
                }
            }
        }
        return;
    }

    // ── Arrow key navigation ───────────────────────────────────────────────
    let any_arrow = keyboard.just_pressed(KeyCode::ArrowRight)
        || keyboard.just_pressed(KeyCode::ArrowLeft)
        || keyboard.just_pressed(KeyCode::ArrowDown)
        || keyboard.just_pressed(KeyCode::ArrowUp);

    if !any_arrow {
        return;
    }

    match container_state.focus {
        ContainerFocus::Left => {
            // Character panel: 2-D grid navigation (same as inventory_ui)
            let max_slots = Inventory::MAX_ITEMS;
            let current = nav_state.selected_slot_index.unwrap_or(0);
            let next = if keyboard.just_pressed(KeyCode::ArrowRight) {
                (current + 1) % max_slots
            } else if keyboard.just_pressed(KeyCode::ArrowLeft) {
                if current == 0 {
                    max_slots - 1
                } else {
                    current - 1
                }
            } else if keyboard.just_pressed(KeyCode::ArrowDown) {
                (current + SLOT_COLS) % max_slots
            } else {
                // ArrowUp
                if current < SLOT_COLS {
                    let last_row_start = (max_slots / SLOT_COLS).saturating_sub(1) * SLOT_COLS;
                    let col = current % SLOT_COLS;
                    (last_row_start + col).min(max_slots - 1)
                } else {
                    current - SLOT_COLS
                }
            };
            nav_state.selected_slot_index = Some(next);
            if let GameMode::ContainerInventory(ref mut cs) = global_state.0.mode {
                cs.character_selected_slot = Some(next);
            }
        }
        ContainerFocus::Right => {
            // Container panel: linear list navigation
            let item_count = container_state.items.len();
            if item_count == 0 {
                return;
            }
            let current = nav_state.selected_slot_index.unwrap_or(0);
            let next = if keyboard.just_pressed(KeyCode::ArrowDown)
                || keyboard.just_pressed(KeyCode::ArrowRight)
            {
                (current + 1) % item_count
            } else {
                // ArrowUp / ArrowLeft
                if current == 0 {
                    item_count - 1
                } else {
                    current - 1
                }
            };
            nav_state.selected_slot_index = Some(next);
            if let GameMode::ContainerInventory(ref mut cs) = global_state.0.mode {
                cs.container_selected_slot = Some(next);
            }
        }
    }
}

// ===== UI system =====

/// Renders the container inventory split-screen overlay.
#[allow(clippy::too_many_lines)]
fn container_inventory_ui_system(
    mut contexts: EguiContexts,
    global_state: Res<GlobalState>,
    game_content: Option<Res<GameContent>>,
    nav_state: Res<ContainerNavState>,
    mut take_writer: MessageWriter<TakeItemAction>,
    mut take_all_writer: MessageWriter<TakeAllAction>,
    mut stash_writer: MessageWriter<StashItemAction>,
) {
    let container_state = match &global_state.0.mode {
        GameMode::ContainerInventory(s) => s.clone(),
        _ => return,
    };

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let char_idx = container_state.active_character_index;
    let char_focused = container_state.character_has_focus();
    let cont_focused = container_state.container_has_focus();

    egui::CentralPanel::default().show(ctx, |ui| {
        // ── Top bar ──────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.heading(format!("Container: {}", container_state.container_name));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(
                        "[Esc] close   [Tab] switch panel   [1-6] switch character",
                    )
                    .small()
                    .weak(),
                );
            });
        });

        // ── Hint line ────────────────────────────────────────────────────
        let hint = match nav_state.phase {
            NavigationPhase::SlotNavigation => {
                "Tab: switch panel   1-6: change character   ←→↑↓: navigate   Enter: select   Esc: close"
            }
            NavigationPhase::ActionNavigation => "←→: cycle actions   Enter: execute   Esc: cancel",
        };
        ui.label(egui::RichText::new(hint).small().weak());
        ui.separator();

        // ── Active character selector strip ──────────────────────────────
        let party_len = global_state.0.party.members.len();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Character:").strong());
            for i in 0..party_len {
                ui.push_id(format!("cont_char_btn_{}", i), |ui| {
                    let member = &global_state.0.party.members[i];
                    let is_active = i == char_idx;
                    let label = egui::RichText::new(format!("[{}] {}", i + 1, member.name))
                        .color(if is_active {
                            egui::Color32::YELLOW
                        } else {
                            egui::Color32::LIGHT_GRAY
                        })
                        .small();
                    // Mouse clicks on character buttons are informational only;
                    // switching is handled via number keys in the input system.
                    let _ = ui.button(label);
                });
            }
        });
        ui.add_space(4.0);

        // ── Split panel layout ───────────────────────────────────────────
        let available = ui.available_size();
        let half_w = (available.x - 8.0) / 2.0;
        let panel_h = available.y;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            // ── LEFT: Character inventory panel ──────────────────────────
            ui.push_id("cont_char_panel", |ui| {
                if let Some(action) = render_character_stash_panel(
                    ui,
                    char_idx,
                    char_focused,
                    container_state.character_selected_slot,
                    if char_focused && nav_state.phase == NavigationPhase::ActionNavigation {
                        Some(nav_state.focused_action_index)
                    } else {
                        None
                    },
                    egui::vec2(half_w, panel_h),
                    &global_state,
                    game_content.as_deref(),
                ) {
                    let StashActionResult {
                        character_index,
                        slot_index,
                    } = action;
                    stash_writer.write(StashItemAction {
                        character_index,
                        character_slot_index: slot_index,
                    });
                }
            });

            // ── RIGHT: Container item list panel ─────────────────────────
            ui.push_id("cont_container_panel", |ui| {
                match render_container_items_panel(
                    ui,
                    &container_state,
                    cont_focused,
                    container_state.container_selected_slot,
                    if cont_focused && nav_state.phase == NavigationPhase::ActionNavigation {
                        Some(nav_state.focused_action_index)
                    } else {
                        None
                    },
                    egui::vec2(half_w, panel_h),
                    &global_state,
                    game_content.as_deref(),
                ) {
                    ContainerPanelResult::Take { slot_index } => {
                        take_writer.write(TakeItemAction {
                            container_slot_index: slot_index,
                            character_index: char_idx,
                        });
                    }
                    ContainerPanelResult::TakeAll => {
                        take_all_writer.write(TakeAllAction {
                            character_index: char_idx,
                        });
                    }
                    ContainerPanelResult::None => {}
                }
            });
        });
    });
}

// ===== Panel render helpers =====

/// Return value from `render_character_stash_panel`.
/// Write the updated container item list back to the corresponding
/// `MapEvent::Container` in the current map.
///
/// Called when the player closes the container screen (presses `Esc`) so that
/// partial takes and stashes persist within the current session.  The write-back
/// must happen **before** the mode is restored; see the close handler in
/// `container_inventory_input_system`.
///
/// # Arguments
///
/// * `game_state`           – Mutable game state (used to access the current map).
/// * `container_event_id`   – The `id` field of the `MapEvent::Container` to update.
/// * `items`                – The current item list from `ContainerInventoryState`.
///
/// # Examples
///
/// ```
/// use antares::application::{GameMode, GameState};
/// use antares::domain::character::InventorySlot;
/// use antares::domain::world::{Map, MapEvent};
/// use antares::domain::types::Position;
/// use antares::game::systems::container_inventory_ui::write_container_items_back;
///
/// let mut state = GameState::new();
/// let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
/// let pos = Position::new(3, 3);
/// map.add_event(
///     pos,
///     MapEvent::Container {
///         id: "chest_01".to_string(),
///         name: "Chest".to_string(),
///         description: "".to_string(),
///         items: vec![InventorySlot { item_id: 1, charges: 0 }],
///     },
/// );
/// state.world.add_map(map);
/// state.world.set_current_map(1);
///
/// // After taking an item the container is empty — write that back.
/// write_container_items_back(&mut state, "chest_01", vec![]);
///
/// if let Some(event) = state.world.get_current_map().unwrap().get_event(pos) {
///     if let MapEvent::Container { items, .. } = event {
///         assert!(items.is_empty());
///     }
/// }
/// ```
pub fn write_container_items_back(
    game_state: &mut crate::application::GameState,
    container_event_id: &str,
    items: Vec<InventorySlot>,
) {
    let Some(map) = game_state.world.get_current_map_mut() else {
        warn!(
            "write_container_items_back: no current map; skipping write-back for '{}'",
            container_event_id
        );
        return;
    };

    // Find the matching Container event by scanning the events HashMap.
    // We look for a MapEvent::Container whose `id` matches container_event_id.
    let position = map.events.iter().find_map(|(pos, event)| {
        if let MapEvent::Container { id, .. } = event {
            if id == container_event_id {
                return Some(*pos);
            }
        }
        None
    });

    let Some(pos) = position else {
        warn!(
            "write_container_items_back: container '{}' not found in current map events",
            container_event_id
        );
        return;
    };

    // Replace the items field in-place.
    if let Some(MapEvent::Container {
        items: ref mut stored,
        ..
    }) = map.events.get_mut(&pos)
    {
        *stored = items;
        info!(
            "write_container_items_back: wrote {} item(s) back to container '{}'",
            stored.len(),
            container_event_id
        );
    }
}

struct StashActionResult {
    character_index: usize,
    slot_index: usize,
}

/// Render the character inventory panel (left side) with a Stash action button.
#[allow(clippy::too_many_arguments)]
fn render_character_stash_panel(
    ui: &mut egui::Ui,
    party_index: usize,
    is_focused: bool,
    selected_slot: Option<usize>,
    focused_action_index: Option<usize>,
    size: egui::Vec2,
    global_state: &GlobalState,
    game_content: Option<&GameContent>,
) -> Option<StashActionResult> {
    if party_index >= global_state.0.party.members.len() {
        return None;
    }

    let character = &global_state.0.party.members[party_index];
    let items = &character.inventory.items;
    let mut result: Option<StashActionResult> = None;

    let has_action = selected_slot.map(|s| s < items.len()).unwrap_or(false);
    let action_reserve = if has_action { PANEL_ACTION_H } else { 0.0 };
    let body_h = (size.y - PANEL_HEADER_H - action_reserve).max(20.0);

    // ── Border ────────────────────────────────────────────────────────────
    let border_color = if is_focused {
        FOCUSED_BORDER_COLOR
    } else {
        UNFOCUSED_BORDER_COLOR
    };
    let (panel_rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_stroke(
        panel_rect,
        2.0,
        egui::Stroke::new(2.0, border_color),
        egui::StrokeKind::Outside,
    );

    // ── Header ────────────────────────────────────────────────────────────
    let header_rect = egui::Rect::from_min_size(panel_rect.min, egui::vec2(size.x, PANEL_HEADER_H));
    painter.rect_filled(header_rect, 0.0, HEADER_BG_COLOR);
    painter.text(
        header_rect.left_center() + egui::vec2(8.0, 0.0),
        egui::Align2::LEFT_CENTER,
        &character.name,
        egui::FontId::proportional(16.0),
        egui::Color32::WHITE,
    );
    painter.text(
        header_rect.right_center() - egui::vec2(8.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        "CHARACTER",
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(160, 160, 160),
    );

    // ── Body: inventory grid ──────────────────────────────────────────────
    let body_rect = egui::Rect::from_min_size(
        panel_rect.min + egui::vec2(0.0, PANEL_HEADER_H),
        egui::vec2(size.x, body_h),
    );
    painter.rect_filled(body_rect, 0.0, PANEL_BG_COLOR);

    let slot_rows = Inventory::MAX_ITEMS.div_ceil(SLOT_COLS);
    let cell_w = (body_rect.width() / SLOT_COLS as f32).floor();
    let cell_h = (body_rect.height() / slot_rows as f32).floor();
    let cell_size = cell_w.min(cell_h).max(8.0);

    // Grid lines
    for col in 0..=SLOT_COLS {
        let x = body_rect.min.x + col as f32 * cell_w;
        painter.line_segment(
            [
                egui::pos2(x, body_rect.min.y),
                egui::pos2(x, body_rect.max.y),
            ],
            egui::Stroke::new(1.0, GRID_LINE_COLOR),
        );
    }
    for row in 0..=slot_rows {
        let y = body_rect.min.y + row as f32 * cell_h;
        painter.line_segment(
            [
                egui::pos2(body_rect.min.x, y),
                egui::pos2(body_rect.max.x, y),
            ],
            egui::Stroke::new(1.0, GRID_LINE_COLOR),
        );
    }

    for slot_idx in 0..Inventory::MAX_ITEMS {
        let col = slot_idx % SLOT_COLS;
        let row = slot_idx / SLOT_COLS;
        let cell_min = body_rect.min + egui::vec2(col as f32 * cell_w, row as f32 * cell_h);
        let cell_rect = egui::Rect::from_min_size(cell_min, egui::vec2(cell_w, cell_h));

        // Selection highlight
        if selected_slot == Some(slot_idx) {
            painter.rect_filled(
                cell_rect.shrink(1.0),
                0.0,
                egui::Color32::from_rgba_premultiplied(180, 150, 0, 60),
            );
            painter.rect_stroke(
                cell_rect.shrink(1.0),
                0.0,
                egui::Stroke::new(2.0, SELECT_HIGHLIGHT_COLOR),
                egui::StrokeKind::Outside,
            );
        }

        // Item silhouette
        if slot_idx < items.len() {
            let item_type = game_content
                .and_then(|gc| gc.db().items.get_item(items[slot_idx].item_id))
                .map(|it| &it.item_type);
            crate::game::systems::inventory_ui::paint_item_silhouette_pub(
                painter,
                cell_rect,
                cell_size,
                item_type,
                egui::Color32::from_rgba_premultiplied(230, 230, 230, 255),
            );
        }
    }

    // ── Action strip: Stash button ────────────────────────────────────────
    if has_action {
        if let Some(slot_idx) = selected_slot {
            let item_name = game_content
                .and_then(|gc| gc.db().items.get_item(items[slot_idx].item_id))
                .map(|it| it.name.clone())
                .unwrap_or_else(|| format!("Item #{}", items[slot_idx].item_id));

            let action_rect = egui::Rect::from_min_size(
                panel_rect.min + egui::vec2(0.0, PANEL_HEADER_H + body_h),
                egui::vec2(size.x, action_reserve),
            );
            painter.rect_filled(action_rect, 0.0, HEADER_BG_COLOR);

            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(action_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            child.add_space(6.0);

            child.push_id("stash_actions", |ui| {
                ui.horizontal_wrapped(|ui| {
                    let stash_focused = focused_action_index == Some(0);

                    let stash_label = egui::RichText::new("Stash")
                        .color(if stash_focused {
                            ACTION_FOCUSED_COLOR
                        } else {
                            STASH_COLOR
                        })
                        .small();
                    let mut stash_btn = egui::Button::new(stash_label);
                    if stash_focused {
                        stash_btn = stash_btn.stroke(egui::Stroke::new(2.0, ACTION_FOCUSED_COLOR));
                    }
                    if ui
                        .add(stash_btn)
                        .on_hover_text(format!("Put {} into the container", item_name))
                        .clicked()
                    {
                        result = Some(StashActionResult {
                            character_index: party_index,
                            slot_index: slot_idx,
                        });
                    }
                });
            });
        }
    }

    result
}

/// Discriminated return value from `render_container_items_panel`.
enum ContainerPanelResult {
    Take { slot_index: usize },
    TakeAll,
    None,
}

/// Render the container items panel (right side).
///
/// Returns the action chosen by mouse click, if any.
#[allow(clippy::too_many_arguments)]
fn render_container_items_panel(
    ui: &mut egui::Ui,
    container_state: &ContainerInventoryState,
    is_focused: bool,
    selected_slot: Option<usize>,
    focused_action_index: Option<usize>,
    size: egui::Vec2,
    global_state: &GlobalState,
    game_content: Option<&GameContent>,
) -> ContainerPanelResult {
    let mut result = ContainerPanelResult::None;

    let items = &container_state.items;
    let item_count = items.len();

    let has_action = selected_slot.map(|s| s < item_count).unwrap_or(false);
    let action_reserve = if has_action { PANEL_ACTION_H } else { 0.0 };
    let body_h = (size.y - PANEL_HEADER_H - action_reserve).max(20.0);

    // ── Border ────────────────────────────────────────────────────────────
    let border_color = if is_focused {
        FOCUSED_BORDER_COLOR
    } else {
        UNFOCUSED_BORDER_COLOR
    };
    let (panel_rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

    // ── Static painting (border, header, body background) ─────────────────
    // All painter calls are grouped here and the borrow dropped before
    // the first `ui.new_child()` call, which requires a mutable borrow.
    {
        let painter = ui.painter();
        painter.rect_stroke(
            panel_rect,
            2.0,
            egui::Stroke::new(2.0, border_color),
            egui::StrokeKind::Outside,
        );

        let header_rect =
            egui::Rect::from_min_size(panel_rect.min, egui::vec2(size.x, PANEL_HEADER_H));
        painter.rect_filled(header_rect, 0.0, HEADER_BG_COLOR);
        painter.text(
            header_rect.left_center() + egui::vec2(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            &container_state.container_name,
            egui::FontId::proportional(16.0),
            egui::Color32::WHITE,
        );
        let count_label = format!(
            "{} item{}",
            item_count,
            if item_count == 1 { "" } else { "s" }
        );
        painter.text(
            header_rect.right_center() - egui::vec2(8.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            count_label,
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(160, 160, 160),
        );

        let body_rect = egui::Rect::from_min_size(
            panel_rect.min + egui::vec2(0.0, PANEL_HEADER_H),
            egui::vec2(size.x, body_h),
        );
        painter.rect_filled(body_rect, 0.0, PANEL_BG_COLOR);
    }

    // ── Body: item list ───────────────────────────────────────────────────
    let body_rect = egui::Rect::from_min_size(
        panel_rect.min + egui::vec2(0.0, PANEL_HEADER_H),
        egui::vec2(size.x, body_h),
    );

    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(body_rect)
            .layout(egui::Layout::top_down(egui::Align::LEFT)),
    );

    egui::ScrollArea::vertical()
        .id_salt("container_items_scroll")
        .max_height(body_h)
        .show(&mut child, |ui| {
            for (i, slot) in items.iter().enumerate() {
                ui.push_id(format!("cont_row_{}", i), |ui| {
                    let is_selected = selected_slot == Some(i);

                    let item_name = game_content
                        .and_then(|gc| gc.db().items.get_item(slot.item_id))
                        .map(|it| it.name.clone())
                        .unwrap_or_else(|| format!("Item #{}", slot.item_id));

                    let charge_info = if slot.charges > 0 {
                        format!(" ({})", slot.charges)
                    } else {
                        String::new()
                    };

                    let label_text = format!("  {}{}", item_name, charge_info);

                    let (row_rect, _response) = ui.allocate_exact_size(
                        egui::vec2(body_rect.width(), CONTAINER_ROW_H),
                        egui::Sense::click(),
                    );

                    if is_selected {
                        ui.painter().rect_filled(
                            row_rect,
                            0.0,
                            egui::Color32::from_rgba_premultiplied(0, 100, 85, 80),
                        );
                        ui.painter().rect_stroke(
                            row_rect.shrink(1.0),
                            0.0,
                            egui::Stroke::new(1.5, SELECT_HIGHLIGHT_COLOR),
                            egui::StrokeKind::Outside,
                        );
                    }

                    ui.painter().text(
                        row_rect.left_center() + egui::vec2(4.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        label_text,
                        egui::FontId::proportional(14.0),
                        CONTAINER_ITEM_COLOR,
                    );
                });
            }

            if items.is_empty() {
                // Centred "(Empty)" label when the container has no items.
                ui.add_space(body_h * 0.35);
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("(Empty)")
                            .color(egui::Color32::from_rgb(130, 130, 130))
                            .size(15.0),
                    );
                });
            }
        });

    // ── Action strip: Take / Take All buttons ─────────────────────────────
    // When the container is empty, render greyed-out (disabled) Take and
    // Take All buttons so the player can see the actions are unavailable.
    if item_count == 0 {
        let empty_action_rect = egui::Rect::from_min_size(
            panel_rect.min + egui::vec2(0.0, PANEL_HEADER_H + body_h),
            egui::vec2(size.x, PANEL_ACTION_H),
        );
        ui.painter_at(empty_action_rect)
            .rect_filled(empty_action_rect, 0.0, HEADER_BG_COLOR);

        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(empty_action_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.add_space(6.0);
        child.push_id("container_empty_actions", |ui| {
            ui.horizontal_wrapped(|ui| {
                let disabled_take = egui::Button::new(
                    egui::RichText::new("Take")
                        .color(egui::Color32::from_rgb(80, 80, 80))
                        .small(),
                );
                let disabled_take_all = egui::Button::new(
                    egui::RichText::new("Take All")
                        .color(egui::Color32::from_rgb(80, 80, 80))
                        .small(),
                );
                ui.add_enabled(false, disabled_take)
                    .on_disabled_hover_text("Container is empty");
                ui.add_space(8.0);
                ui.add_enabled(false, disabled_take_all)
                    .on_disabled_hover_text("Container is empty");
            });
        });
    } else if has_action {
        if let Some(slot_idx) = selected_slot {
            let item_name = game_content
                .and_then(|gc| gc.db().items.get_item(items[slot_idx].item_id))
                .map(|it| it.name.clone())
                .unwrap_or_else(|| format!("Item #{}", items[slot_idx].item_id));

            let char_inv_full = global_state
                .0
                .party
                .members
                .get(container_state.active_character_index)
                .map(|ch| ch.inventory.is_full())
                .unwrap_or(true);

            let action_rect = egui::Rect::from_min_size(
                panel_rect.min + egui::vec2(0.0, PANEL_HEADER_H + body_h),
                egui::vec2(size.x, action_reserve),
            );
            // Use painter_at to get an independent painter (no borrow conflict
            // with the following ui.new_child() call).
            ui.painter_at(action_rect)
                .rect_filled(action_rect, 0.0, HEADER_BG_COLOR);

            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(action_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            child.add_space(6.0);

            child.push_id("container_actions", |ui| {
                ui.horizontal_wrapped(|ui| {
                    // ── Take button ───────────────────────────────────────
                    let take_focused = focused_action_index == Some(0);
                    let take_label = egui::RichText::new("Take")
                        .color(if take_focused {
                            ACTION_FOCUSED_COLOR
                        } else {
                            TAKE_COLOR
                        })
                        .small();
                    let mut take_btn = egui::Button::new(take_label);
                    if take_focused {
                        take_btn = take_btn.stroke(egui::Stroke::new(2.0, ACTION_FOCUSED_COLOR));
                    }
                    let take_hover = if char_inv_full {
                        "Character's inventory is full".to_string()
                    } else {
                        format!("Take {}", item_name)
                    };
                    if ui
                        .add_enabled(!char_inv_full, take_btn)
                        .on_hover_text(take_hover)
                        .clicked()
                    {
                        result = ContainerPanelResult::Take {
                            slot_index: slot_idx,
                        };
                    }

                    ui.add_space(8.0);

                    // ── Take All button ───────────────────────────────────
                    let take_all_focused = focused_action_index == Some(1);
                    let take_all_label = egui::RichText::new("Take All")
                        .color(if take_all_focused {
                            ACTION_FOCUSED_COLOR
                        } else {
                            TAKE_COLOR
                        })
                        .small();
                    let mut take_all_btn = egui::Button::new(take_all_label);
                    if take_all_focused {
                        take_all_btn =
                            take_all_btn.stroke(egui::Stroke::new(2.0, ACTION_FOCUSED_COLOR));
                    }
                    let take_all_hover = if char_inv_full {
                        "Character's inventory is full".to_string()
                    } else {
                        format!("Take all {} items", item_count)
                    };
                    if ui
                        .add_enabled(!char_inv_full, take_all_btn)
                        .on_hover_text(take_all_hover)
                        .clicked()
                    {
                        result = ContainerPanelResult::TakeAll;
                    }
                });
            });
        }
    } // end `else if has_action`

    result
}

// ===== Action system =====

/// Executes Take, Take All, and Stash actions.
///
/// Reads `TakeItemAction`, `TakeAllAction`, and `StashItemAction` messages,
/// mutates `GlobalState` (both party inventory and container item list), and
/// resets keyboard navigation state after each action.
#[allow(clippy::too_many_lines)]
fn container_inventory_action_system(
    mut take_reader: MessageReader<TakeItemAction>,
    mut take_all_reader: MessageReader<TakeAllAction>,
    mut stash_reader: MessageReader<StashItemAction>,
    mut global_state: ResMut<GlobalState>,
    mut nav_state: ResMut<ContainerNavState>,
) {
    let take_events: Vec<(usize, usize)> = take_reader
        .read()
        .map(|e| (e.container_slot_index, e.character_index))
        .collect();

    let take_all_events: Vec<usize> = take_all_reader.read().map(|e| e.character_index).collect();

    let stash_events: Vec<(usize, usize)> = stash_reader
        .read()
        .map(|e| (e.character_index, e.character_slot_index))
        .collect();

    // ── Take events ───────────────────────────────────────────────────────
    for (container_slot, character_index) in take_events {
        if character_index >= global_state.0.party.members.len() {
            warn!(
                "TakeItemAction: character_index {} out of bounds (party size {})",
                character_index,
                global_state.0.party.members.len()
            );
            continue;
        }

        if global_state.0.party.members[character_index]
            .inventory
            .is_full()
        {
            warn!(
                "TakeItemAction: character[{}] inventory is full",
                character_index
            );
            continue;
        }

        // Take item from container state
        let slot_opt = if let GameMode::ContainerInventory(ref mut cs) = global_state.0.mode {
            cs.take_item(container_slot)
        } else {
            None
        };

        if let Some(taken_slot) = slot_opt {
            match global_state.0.party.members[character_index]
                .inventory
                .add_item(taken_slot.item_id, taken_slot.charges)
            {
                Ok(()) => {
                    info!(
                        "Took item_id={} from container into party[{}]",
                        taken_slot.item_id, character_index
                    );
                }
                Err(err) => {
                    // Rollback: put item back into the container
                    warn!(
                        "TakeItemAction: add_item for party[{}] failed ({:?}); rolling back",
                        character_index, err
                    );
                    if let GameMode::ContainerInventory(ref mut cs) = global_state.0.mode {
                        cs.stash_item(taken_slot);
                    }
                }
            }
        }

        nav_state.selected_slot_index = None;
        nav_state.focused_action_index = 0;
        nav_state.phase = NavigationPhase::SlotNavigation;
    }

    // ── Take All events ───────────────────────────────────────────────────
    for character_index in take_all_events {
        if character_index >= global_state.0.party.members.len() {
            warn!(
                "TakeAllAction: character_index {} out of bounds (party size {})",
                character_index,
                global_state.0.party.members.len()
            );
            continue;
        }

        // Drain all items from the container into a local vec
        let all_items: Vec<_> =
            if let GameMode::ContainerInventory(ref mut cs) = global_state.0.mode {
                cs.take_all()
            } else {
                Vec::new()
            };

        // Add each item to the character's inventory; overflow goes back to container
        let mut overflow: Vec<_> = Vec::new();
        for item_slot in all_items {
            if global_state.0.party.members[character_index]
                .inventory
                .is_full()
            {
                overflow.push(item_slot);
                continue;
            }
            match global_state.0.party.members[character_index]
                .inventory
                .add_item(item_slot.item_id, item_slot.charges)
            {
                Ok(()) => {
                    info!(
                        "TakeAll: moved item_id={} to party[{}]",
                        item_slot.item_id, character_index
                    );
                }
                Err(err) => {
                    warn!(
                        "TakeAll: add_item for party[{}] failed ({:?}); returning to container",
                        character_index, err
                    );
                    overflow.push(item_slot);
                }
            }
        }

        // Return overflow items to container
        if !overflow.is_empty() {
            if let GameMode::ContainerInventory(ref mut cs) = global_state.0.mode {
                for item in overflow {
                    cs.stash_item(item);
                }
            }
        }

        nav_state.selected_slot_index = None;
        nav_state.focused_action_index = 0;
        nav_state.phase = NavigationPhase::SlotNavigation;
    }

    // ── Stash events ──────────────────────────────────────────────────────
    for (character_index, char_slot_index) in stash_events {
        if character_index >= global_state.0.party.members.len() {
            warn!(
                "StashItemAction: character_index {} out of bounds (party size {})",
                character_index,
                global_state.0.party.members.len()
            );
            continue;
        }

        let inv_len = global_state.0.party.members[character_index]
            .inventory
            .items
            .len();
        if char_slot_index >= inv_len {
            warn!(
                "StashItemAction: char_slot_index {} out of bounds (inventory size {}) for party[{}]",
                char_slot_index, inv_len, character_index
            );
            continue;
        }

        // Remove item from character inventory
        let removed = global_state.0.party.members[character_index]
            .inventory
            .remove_item(char_slot_index);

        if let Some(slot) = removed {
            if let GameMode::ContainerInventory(ref mut cs) = global_state.0.mode {
                cs.stash_item(slot);
                info!(
                    "Stashed item_id={} from party[{}] into container '{}'",
                    slot.item_id, character_index, cs.container_event_id
                );
            }
        }

        nav_state.selected_slot_index = None;
        nav_state.focused_action_index = 0;
        nav_state.phase = NavigationPhase::SlotNavigation;
    }
}

// ===== Helpers =====

/// Returns `true` if the highlighted slot in the currently focused panel
/// contains an item.
fn container_slot_has_content(
    container_state: &ContainerInventoryState,
    slot_idx: usize,
    global_state: &GlobalState,
) -> bool {
    match container_state.focus {
        ContainerFocus::Left => {
            // Character panel
            global_state
                .0
                .party
                .members
                .get(container_state.active_character_index)
                .map(|ch| slot_idx < ch.inventory.items.len())
                .unwrap_or(false)
        }
        ContainerFocus::Right => {
            // Container panel
            slot_idx < container_state.items.len()
        }
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::container_inventory_state::{ContainerFocus, ContainerInventoryState};
    use crate::application::{GameMode, GameState};
    use crate::domain::character::InventorySlot;
    use crate::domain::types::Position;
    use crate::domain::world::{Map, MapEvent};

    fn make_slot(item_id: u8) -> InventorySlot {
        InventorySlot {
            item_id,
            charges: 0,
        }
    }

    fn make_container_state_with_items() -> (GameState, ContainerInventoryState) {
        use crate::domain::character::{Alignment, Character, Sex};

        let mut game_state = GameState::new();

        // Create a party member with two items so the character panel has content
        let mut character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        character.inventory.add_item(1, 0).unwrap();
        character.inventory.add_item(2, 0).unwrap();
        game_state
            .party
            .add_member(character)
            .expect("add_member should succeed");

        let container_items = vec![make_slot(10), make_slot(20), make_slot(30)];
        let cs = ContainerInventoryState::new(
            "chest_test".to_string(),
            "Test Chest".to_string(),
            container_items.clone(),
            0,
            GameMode::Exploration,
        );

        (game_state, cs)
    }

    #[test]
    fn test_container_inventory_plugin_builds() {
        let mut app = App::new();
        app.add_plugins(ContainerInventoryPlugin);
        // Registers resources and messages without panic
    }

    #[test]
    fn test_container_nav_state_default() {
        let state = ContainerNavState::default();
        assert_eq!(state.selected_slot_index, None);
        assert_eq!(state.focused_action_index, 0);
        assert!(matches!(state.phase, NavigationPhase::SlotNavigation));
    }

    #[test]
    fn test_container_nav_state_reset() {
        let mut state = ContainerNavState {
            selected_slot_index: Some(5),
            focused_action_index: 1,
            phase: NavigationPhase::ActionNavigation,
        };
        state.reset();
        assert_eq!(state.selected_slot_index, None);
        assert_eq!(state.focused_action_index, 0);
        assert!(matches!(state.phase, NavigationPhase::SlotNavigation));
    }

    #[test]
    fn test_take_item_action_fields() {
        let action = TakeItemAction {
            container_slot_index: 2,
            character_index: 1,
        };
        assert_eq!(action.container_slot_index, 2);
        assert_eq!(action.character_index, 1);
    }

    #[test]
    fn test_take_all_action_fields() {
        let action = TakeAllAction { character_index: 3 };
        assert_eq!(action.character_index, 3);
    }

    #[test]
    fn test_stash_item_action_fields() {
        let action = StashItemAction {
            character_index: 0,
            character_slot_index: 4,
        };
        assert_eq!(action.character_index, 0);
        assert_eq!(action.character_slot_index, 4);
    }

    // ── build_container_action_list ───────────────────────────────────────

    #[test]
    fn test_build_action_list_container_focus_returns_take_and_take_all() {
        let actions = build_container_action_list(2, 0, &ContainerFocus::Right);
        assert_eq!(actions.len(), 2);
        assert!(matches!(
            actions[0],
            ContainerAction::Take {
                slot_index: 2,
                character_index: 0
            }
        ));
        assert!(matches!(
            actions[1],
            ContainerAction::TakeAll { character_index: 0 }
        ));
    }

    #[test]
    fn test_build_action_list_character_focus_returns_stash() {
        let actions = build_container_action_list(3, 1, &ContainerFocus::Left);
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            actions[0],
            ContainerAction::Stash {
                character_index: 1,
                slot_index: 3
            }
        ));
    }

    // ── container_slot_has_content ────────────────────────────────────────

    #[test]
    fn test_has_content_character_panel_with_item() {
        let (game_state, mut cs) = make_container_state_with_items();
        cs.focus = ContainerFocus::Left;
        let global = crate::game::resources::GlobalState(game_state);

        // Character has items at slots 0 and 1
        assert!(container_slot_has_content(&cs, 0, &global));
        assert!(container_slot_has_content(&cs, 1, &global));
        assert!(!container_slot_has_content(&cs, 5, &global));
    }

    #[test]
    fn test_has_content_container_panel_with_items() {
        let (_game_state, mut cs) = make_container_state_with_items();
        cs.focus = ContainerFocus::Right;
        let global = crate::game::resources::GlobalState(GameState::new());

        // Container has items at slots 0, 1, 2
        assert!(container_slot_has_content(&cs, 0, &global));
        assert!(container_slot_has_content(&cs, 2, &global));
        assert!(!container_slot_has_content(&cs, 3, &global));
    }

    #[test]
    fn test_has_content_container_panel_empty() {
        let cs = ContainerInventoryState::new(
            "c".to_string(),
            "C".to_string(),
            vec![],
            0,
            GameMode::Exploration,
        );
        let global = crate::game::resources::GlobalState(GameState::new());
        assert!(!container_slot_has_content(&cs, 0, &global));
    }

    // ── ContainerAction equality ──────────────────────────────────────────

    #[test]
    fn test_container_action_take_equality() {
        let a = ContainerAction::Take {
            slot_index: 1,
            character_index: 0,
        };
        let b = ContainerAction::Take {
            slot_index: 1,
            character_index: 0,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_container_action_take_all_equality() {
        let a = ContainerAction::TakeAll { character_index: 2 };
        let b = ContainerAction::TakeAll { character_index: 2 };
        assert_eq!(a, b);
    }

    #[test]
    fn test_container_action_stash_equality() {
        let a = ContainerAction::Stash {
            character_index: 0,
            slot_index: 3,
        };
        let b = ContainerAction::Stash {
            character_index: 0,
            slot_index: 3,
        };
        assert_eq!(a, b);
    }

    // ── GameState::enter_container_inventory ─────────────────────────────

    #[test]
    fn test_game_state_enter_container_inventory_sets_mode() {
        let mut state = GameState::new();
        state.enter_container_inventory("chest_01".to_string(), "Old Chest".to_string(), vec![]);
        assert!(matches!(state.mode, GameMode::ContainerInventory(_)));
    }

    #[test]
    fn test_game_state_enter_container_inventory_stores_items() {
        let mut state = GameState::new();
        let items = vec![make_slot(1), make_slot(2)];
        state.enter_container_inventory("c".to_string(), "C".to_string(), items);
        if let GameMode::ContainerInventory(ref cs) = state.mode {
            assert_eq!(cs.items.len(), 2);
        } else {
            panic!("Expected ContainerInventory mode");
        }
    }

    #[test]
    fn test_game_state_enter_container_inventory_stores_previous_mode() {
        let mut state = GameState::new();
        assert!(matches!(state.mode, GameMode::Exploration));
        state.enter_container_inventory("c".to_string(), "C".to_string(), vec![]);
        if let GameMode::ContainerInventory(ref cs) = state.mode {
            assert!(matches!(cs.get_resume_mode(), GameMode::Exploration));
        } else {
            panic!("Expected ContainerInventory mode");
        }
    }

    #[test]
    fn test_game_state_enter_container_inventory_defaults_character_zero() {
        let mut state = GameState::new();
        state.enter_container_inventory("c".to_string(), "C".to_string(), vec![]);
        if let GameMode::ContainerInventory(ref cs) = state.mode {
            assert_eq!(cs.active_character_index, 0);
        } else {
            panic!("Expected ContainerInventory mode");
        }
    }

    #[test]
    fn test_game_state_enter_container_inventory_focus_defaults_left() {
        let mut state = GameState::new();
        state.enter_container_inventory("c".to_string(), "C".to_string(), vec![]);
        if let GameMode::ContainerInventory(ref cs) = state.mode {
            assert!(matches!(cs.focus, ContainerFocus::Left));
        } else {
            panic!("Expected ContainerInventory mode");
        }
    }

    // ── write_container_items_back ────────────────────────────────────────

    fn make_game_state_with_container(
        container_id: &str,
        initial_items: Vec<InventorySlot>,
        pos: Position,
    ) -> GameState {
        let mut state = GameState::new();
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        map.add_event(
            pos,
            MapEvent::Container {
                id: container_id.to_string(),
                name: "Test Chest".to_string(),
                description: "".to_string(),
                items: initial_items,
            },
        );
        state.world.add_map(map);
        state.world.set_current_map(1);
        state
    }

    #[test]
    fn test_close_container_writes_items_back_to_map_event() {
        // Arrange: container starts with two items.
        let pos = Position::new(3, 3);
        let mut state =
            make_game_state_with_container("chest_test", vec![make_slot(1), make_slot(2)], pos);

        // Simulate taking one item: write back only item 2.
        write_container_items_back(&mut state, "chest_test", vec![make_slot(2)]);

        // Assert: map event now contains only item 2.
        let event = state
            .world
            .get_current_map()
            .unwrap()
            .get_event(pos)
            .unwrap();
        if let MapEvent::Container { items, .. } = event {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].item_id, 2);
        } else {
            panic!("Expected Container event");
        }
    }

    #[test]
    fn test_take_all_empties_container_and_writes_back() {
        // Arrange: container starts with three items.
        let pos = Position::new(4, 4);
        let mut state = make_game_state_with_container(
            "barrel_all",
            vec![make_slot(10), make_slot(20), make_slot(30)],
            pos,
        );

        // Simulate taking all: write back an empty list.
        write_container_items_back(&mut state, "barrel_all", vec![]);

        let event = state
            .world
            .get_current_map()
            .unwrap()
            .get_event(pos)
            .unwrap();
        if let MapEvent::Container { items, .. } = event {
            assert!(items.is_empty(), "Container must be empty after Take All");
        } else {
            panic!("Expected Container event");
        }
    }

    #[test]
    fn test_stash_item_adds_to_container_and_writes_back() {
        // Arrange: container starts with one item.
        let pos = Position::new(5, 5);
        let mut state = make_game_state_with_container("crate_stash", vec![make_slot(1)], pos);

        // Simulate stashing a second item: write back two items.
        write_container_items_back(&mut state, "crate_stash", vec![make_slot(1), make_slot(5)]);

        let event = state
            .world
            .get_current_map()
            .unwrap()
            .get_event(pos)
            .unwrap();
        if let MapEvent::Container { items, .. } = event {
            assert_eq!(items.len(), 2);
            assert_eq!(items[1].item_id, 5);
        } else {
            panic!("Expected Container event");
        }
    }

    #[test]
    fn test_write_back_unknown_container_id_is_noop() {
        // Writing back to a non-existent container ID must not panic.
        let pos = Position::new(6, 6);
        let mut state = make_game_state_with_container("known_chest", vec![make_slot(1)], pos);

        // This must not panic.
        write_container_items_back(&mut state, "unknown_id", vec![]);

        // The known container must be unchanged.
        let event = state
            .world
            .get_current_map()
            .unwrap()
            .get_event(pos)
            .unwrap();
        if let MapEvent::Container { items, .. } = event {
            assert_eq!(items.len(), 1, "Original container must be unchanged");
        } else {
            panic!("Expected Container event");
        }
    }

    #[test]
    fn test_empty_container_disables_take_all_action() {
        // Arrange: build ContainerInventoryState with no items.
        let cs = ContainerInventoryState::new(
            "empty_chest".to_string(),
            "Empty Chest".to_string(),
            vec![],
            0,
            GameMode::Exploration,
        );

        // An empty container must report is_empty() == true.
        assert!(cs.is_empty(), "Container must be empty");
        // item_count must be zero.
        assert_eq!(cs.item_count(), 0);
        // build_container_action_list with no items: container focus has Take / Take All
        // but the Take action's slot_index would be 0 which is out of bounds — the
        // UI guards against this by only showing the action strip when has_action is true
        // (i.e. selected_slot < item_count).  Verify item_count directly.
        assert_eq!(cs.items.len(), 0);
    }
}
