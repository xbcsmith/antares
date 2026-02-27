// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Inventory UI System - Character inventory management interface
//!
//! Provides an egui-based overlay for viewing and managing character inventory
//! when the game is in `GameMode::Inventory` mode. This system is active when
//! the player presses the configured inventory key (default: `I`).
//!
//! Follows the `InnUiPlugin` pattern from `src/game/systems/inn_ui.rs` exactly.

use crate::application::resources::GameContent;
use crate::application::GameMode;
use crate::domain::character::{Inventory, PARTY_MAX_SIZE};
use crate::game::resources::GlobalState;
use crate::game::systems::input::InputConfigResource;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

/// Plugin for inventory management UI
pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DropItemAction>()
            .add_message::<TransferItemAction>()
            .init_resource::<InventoryNavigationState>()
            .add_systems(
                Update,
                (
                    inventory_input_system,
                    inventory_ui_system,
                    inventory_action_system,
                )
                    .chain(),
            );
    }
}

// ===== Events =====

/// Event to drop an item from a character's inventory
///
/// Removing an item places it nowhere (drops it from the party entirely).
#[derive(Message)]
pub struct DropItemAction {
    /// Index of the party member (0-based) whose inventory contains the item
    pub party_index: usize,
    /// Index of the slot within that character's inventory to drop
    pub slot_index: usize,
}

/// Event to transfer an item from one character's inventory to another's
#[derive(Message)]
pub struct TransferItemAction {
    /// Party index of the character giving the item
    pub from_party_index: usize,
    /// Slot index in the source character's inventory
    pub from_slot_index: usize,
    /// Party index of the character receiving the item
    pub to_party_index: usize,
}

// ===== Navigation State =====

/// Tracks keyboard navigation state for the inventory overlay.
///
/// Mirrors the pattern of `InnNavigationState` from `inn_ui.rs`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::inventory_ui::InventoryNavigationState;
///
/// let state = InventoryNavigationState::default();
/// assert_eq!(state.selected_slot_index, None);
/// assert_eq!(state.focus_on_panel, 0);
/// ```
#[derive(Resource, Default, Debug)]
pub struct InventoryNavigationState {
    /// Index of the selected slot within the focused panel (`None` = header focused).
    pub selected_slot_index: Option<usize>,
    /// Which panel column has keyboard focus (maps to `open_panels` index, not `party_index`).
    pub focus_on_panel: usize,
}

// ===== Input System =====

/// Handles keyboard input for inventory navigation.
///
/// Runs every frame; only processes input when
/// `GlobalState.0.mode` is `GameMode::Inventory(_)`.
fn inventory_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut global_state: ResMut<GlobalState>,
    mut nav_state: ResMut<InventoryNavigationState>,
    input_config: Option<Res<InputConfigResource>>,
) {
    // Extract inventory state — bail if not in inventory mode
    let (party_size, slot_count, focused_panel_index) = {
        let inv_state = match &global_state.0.mode {
            GameMode::Inventory(s) => s,
            _ => {
                // Reset navigation state when not in inventory mode
                *nav_state = InventoryNavigationState::default();
                return;
            }
        };

        let party_size = global_state.0.party.members.len().min(PARTY_MAX_SIZE);
        let focused_party_index = inv_state.focused_index;
        let slot_count = if focused_party_index < global_state.0.party.members.len() {
            global_state.0.party.members[focused_party_index]
                .inventory
                .items
                .len()
        } else {
            0
        };
        let focused_panel_index = nav_state.focus_on_panel;
        (party_size, slot_count, focused_panel_index)
    };

    // Determine whether the configured inventory key was just pressed
    let inventory_key_pressed = if let Some(ref cfg) = input_config {
        cfg.key_map.is_action_just_pressed(
            crate::game::systems::input::GameAction::Inventory,
            &keyboard,
        )
    } else {
        // Fallback: check default key "I"
        keyboard.just_pressed(KeyCode::KeyI)
    };

    // Escape or inventory key closes the overlay
    if keyboard.just_pressed(KeyCode::Escape) || inventory_key_pressed {
        let resume_mode = match &global_state.0.mode {
            GameMode::Inventory(s) => s.get_resume_mode(),
            _ => return,
        };
        global_state.0.mode = resume_mode;
        *nav_state = InventoryNavigationState::default();
        return;
    }

    // Tab (no modifier) — cycle to the next panel
    if keyboard.just_pressed(KeyCode::Tab)
        && !keyboard.pressed(KeyCode::ShiftLeft)
        && !keyboard.pressed(KeyCode::ShiftRight)
    {
        let inv_state = match &mut global_state.0.mode {
            GameMode::Inventory(s) => s,
            _ => return,
        };
        inv_state.tab_next(party_size);
        // Clamp focus_on_panel to the new open_panels length
        let panels_len = inv_state.open_panels.len();
        if nav_state.focus_on_panel >= panels_len {
            nav_state.focus_on_panel = panels_len.saturating_sub(1);
        }
        nav_state.selected_slot_index = None;
        return;
    }

    // Shift+Tab — cycle to the previous panel
    if keyboard.just_pressed(KeyCode::Tab)
        && (keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight))
    {
        let inv_state = match &mut global_state.0.mode {
            GameMode::Inventory(s) => s,
            _ => return,
        };
        inv_state.tab_prev(party_size);
        let panels_len = inv_state.open_panels.len();
        if nav_state.focus_on_panel >= panels_len {
            nav_state.focus_on_panel = panels_len.saturating_sub(1);
        }
        nav_state.selected_slot_index = None;
        return;
    }

    // ArrowUp — select the previous slot in the focused panel
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        let inv_state = match &mut global_state.0.mode {
            GameMode::Inventory(s) => s,
            _ => return,
        };
        inv_state.select_prev_slot(slot_count);
        nav_state.selected_slot_index = inv_state.selected_slot;
        return;
    }

    // ArrowDown — select the next slot in the focused panel
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        let inv_state = match &mut global_state.0.mode {
            GameMode::Inventory(s) => s,
            _ => return,
        };
        inv_state.select_next_slot(slot_count);
        nav_state.selected_slot_index = inv_state.selected_slot;
        return;
    }

    // ArrowLeft / ArrowRight — move focus between visible panels
    if keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::ArrowRight) {
        let panels_len = match &global_state.0.mode {
            GameMode::Inventory(s) => s.open_panels.len(),
            _ => return,
        };
        if panels_len == 0 {
            return;
        }
        if keyboard.just_pressed(KeyCode::ArrowRight) {
            nav_state.focus_on_panel = (focused_panel_index + 1) % panels_len;
        } else {
            nav_state.focus_on_panel = if focused_panel_index == 0 {
                panels_len - 1
            } else {
                focused_panel_index - 1
            };
        }
        nav_state.selected_slot_index = None;
        // Sync focused_index in InventoryState to the new panel
        let new_panel_idx = nav_state.focus_on_panel;
        if let GameMode::Inventory(inv_state) = &mut global_state.0.mode {
            if new_panel_idx < inv_state.open_panels.len() {
                inv_state.focused_index = inv_state.open_panels[new_panel_idx];
            }
        }
    }
}

// ===== UI System =====

/// Renders the egui inventory overlay when in `GameMode::Inventory`.
///
/// Uses `egui::CentralPanel` as the outer container so it occupies the full
/// viewport.  The Bevy native HUD rendered in a separate pass is unaffected.
#[allow(clippy::too_many_lines)]
fn inventory_ui_system(
    mut contexts: EguiContexts,
    global_state: Res<GlobalState>,
    nav_state: Res<InventoryNavigationState>,
    game_content: Option<Res<GameContent>>,
) {
    // Only render when in Inventory mode
    let inv_state = match &global_state.0.mode {
        GameMode::Inventory(s) => s.clone(),
        _ => return,
    };

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    // Snapshot data needed for rendering before taking any mutable borrows
    let open_panels = inv_state.open_panels.clone();
    let focused_index = inv_state.focused_index;
    let selected_slot = inv_state.selected_slot;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Inventory");
        ui.label(
            egui::RichText::new("[I] or [Esc] to close")
                .italics()
                .weak(),
        );
        ui.add_space(8.0);

        // Lay out character panels side by side
        ui.horizontal(|ui| {
            for (panel_pos, &party_index) in open_panels.iter().enumerate() {
                let is_focused =
                    party_index == focused_index && panel_pos == nav_state.focus_on_panel;
                ui.push_id(format!("inv_panel_{}", party_index), |ui| {
                    render_character_panel(
                        ui,
                        party_index,
                        is_focused,
                        selected_slot,
                        &global_state,
                        game_content.as_deref(),
                    );
                });
            }
        });

        ui.add_space(8.0);
        ui.separator();

        // Footer: focused character name and selected item details
        let party = &global_state.0.party;
        if focused_index < party.members.len() {
            let character = &party.members[focused_index];
            let detail_text = match selected_slot {
                Some(slot_idx) if slot_idx < character.inventory.items.len() => {
                    let slot = &character.inventory.items[slot_idx];
                    let item_name = game_content
                        .as_deref()
                        .and_then(|gc| gc.db().items.get_item(slot.item_id))
                        .map(|item| item.name.clone())
                        .unwrap_or_else(|| format!("Item #{}", slot.item_id));
                    format!(
                        "Focus: {} | Selected: {} (slot {})",
                        character.name, item_name, slot_idx
                    )
                }
                _ => format!("Focus: {}", character.name),
            };
            ui.label(egui::RichText::new(detail_text).strong());
        } else {
            ui.label("No party members.");
        }
    });
}

/// Renders a single character's inventory panel.
///
/// # Arguments
///
/// * `ui` – The egui `Ui` to render into.
/// * `party_index` – Zero-based index into `global_state.0.party.members`.
/// * `is_focused` – Whether this panel currently has keyboard focus.
/// * `selected_slot` – Highlighted slot index within this panel (if any).
/// * `global_state` – Read-only reference to global game state.
/// * `game_content` – Optional content database for item name lookups.
#[allow(clippy::too_many_arguments)]
fn render_character_panel(
    ui: &mut egui::Ui,
    party_index: usize,
    is_focused: bool,
    selected_slot: Option<usize>,
    global_state: &GlobalState,
    game_content: Option<&GameContent>,
) {
    // Bounds-check: silently skip if party_index is out of range
    if party_index >= global_state.0.party.members.len() {
        return;
    }

    let character = &global_state.0.party.members[party_index];

    // Mandatory egui ID scope per sdk/AGENTS.md — every loop body uses push_id
    ui.push_id(party_index, |ui| {
        let border_color = if is_focused {
            egui::Color32::YELLOW
        } else {
            egui::Color32::DARK_GRAY
        };

        let frame = egui::Frame::group(ui.style())
            .stroke(egui::Stroke::new(2.0, border_color))
            .inner_margin(egui::Margin::same(8));

        frame.show(ui, |ui| {
            ui.set_min_width(150.0);

            // Character name heading
            ui.label(egui::RichText::new(&character.name).strong().size(14.0));
            ui.label(format!("Gold: {}", character.gold));
            ui.label(format!(
                "HP: {}/{} | SP: {}/{}",
                character.hp.current, character.hp.base, character.sp.current, character.sp.base
            ));
            ui.add_space(4.0);

            ui.label(
                egui::RichText::new(format!(
                    "Items ({}/{})",
                    character.inventory.items.len(),
                    Inventory::MAX_ITEMS
                ))
                .size(12.0)
                .strong(),
            );
            ui.separator();

            // Render each inventory slot (0..MAX_ITEMS)
            for slot_idx in 0..Inventory::MAX_ITEMS {
                // Mandatory per egui ID rules — unique ID for each slot widget
                ui.push_id(format!("slot_{}", slot_idx), |ui| {
                    let is_selected = selected_slot == Some(slot_idx);

                    if slot_idx < character.inventory.items.len() {
                        let slot = &character.inventory.items[slot_idx];
                        let item_label = game_content
                            .and_then(|gc| gc.db().items.get_item(slot.item_id))
                            .map(|item| item.name.clone())
                            .unwrap_or_else(|| format!("Item #{}", slot.item_id));

                        if is_selected {
                            // Highlight selected slot with yellow background
                            let rich =
                                egui::RichText::new(format!("[{}] {}", slot_idx, item_label))
                                    .color(egui::Color32::BLACK)
                                    .background_color(egui::Color32::YELLOW)
                                    .monospace();
                            ui.label(rich);
                        } else {
                            ui.label(format!("[{}] {}", slot_idx, item_label));
                        }
                    } else {
                        // Empty slot — dimmed label
                        let empty = egui::RichText::new(format!("[{}] [empty]", slot_idx))
                            .weak()
                            .italics();
                        ui.label(empty);
                    }
                });
            }
        });
    });
}

// ===== Action System =====

/// Processes `DropItemAction` and `TransferItemAction` events each frame.
///
/// This is a stub in Phase 3; full drop/transfer logic is implemented in Phase 4.
fn inventory_action_system(
    mut drop_events: MessageReader<DropItemAction>,
    mut transfer_events: MessageReader<TransferItemAction>,
    mut global_state: ResMut<GlobalState>,
) {
    // Only process actions while in inventory mode
    if !matches!(global_state.0.mode, GameMode::Inventory(_)) {
        return;
    }

    // Process drop events (Phase 4 will add item removal logic)
    for event in drop_events.read() {
        let party = &mut global_state.0.party;
        if event.party_index < party.members.len() {
            let inv = &mut party.members[event.party_index].inventory;
            if event.slot_index < inv.items.len() {
                inv.items.remove(event.slot_index);
                debug!(
                    "DropItemAction: removed slot {} from party member {}",
                    event.slot_index, event.party_index
                );
            }
        }
    }

    // Process transfer events (Phase 4 will add full transfer logic)
    for event in transfer_events.read() {
        let party = &mut global_state.0.party;
        let from_idx = event.from_party_index;
        let to_idx = event.to_party_index;
        let slot_idx = event.from_slot_index;

        if from_idx == to_idx {
            continue; // Cannot transfer to yourself
        }

        if from_idx >= party.members.len() || to_idx >= party.members.len() {
            continue;
        }

        // Check that the slot exists and the destination has space
        if slot_idx >= party.members[from_idx].inventory.items.len() {
            continue;
        }

        if party.members[to_idx].inventory.is_full() {
            debug!(
                "TransferItemAction: destination party member {} inventory is full",
                to_idx
            );
            continue;
        }

        let slot = party.members[from_idx].inventory.items.remove(slot_idx);
        party.members[to_idx].inventory.items.push(slot);
        debug!(
            "TransferItemAction: transferred slot {} from member {} to member {}",
            slot_idx, from_idx, to_idx
        );
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameState;

    // ------------------------------------------------------------------
    // 3.4.1  Plugin smoke test
    // ------------------------------------------------------------------

    /// Verifies that `InventoryPlugin` builds without panicking.
    ///
    /// Mirrors `test_inn_ui_plugin_builds` from `inn_ui.rs`.
    #[test]
    fn test_inventory_ui_plugin_builds() {
        let mut app = App::new();
        app.add_plugins(InventoryPlugin);
        // If we reach here, the plugin registered without errors.
    }

    // ------------------------------------------------------------------
    // 3.4.2  Navigation state defaults
    // ------------------------------------------------------------------

    /// `InventoryNavigationState::default()` must have the specified initial values.
    #[test]
    fn test_inventory_navigation_state_default() {
        let state = InventoryNavigationState::default();
        assert_eq!(state.selected_slot_index, None);
        assert_eq!(state.focus_on_panel, 0);
    }

    // ------------------------------------------------------------------
    // 3.4.3  Action message variants can be constructed
    // ------------------------------------------------------------------

    /// Verifies that `DropItemAction` and `TransferItemAction` can be constructed.
    #[test]
    fn test_inventory_action_button_variants() {
        let drop = DropItemAction {
            party_index: 0,
            slot_index: 3,
        };
        assert_eq!(drop.party_index, 0);
        assert_eq!(drop.slot_index, 3);

        let transfer = TransferItemAction {
            from_party_index: 0,
            from_slot_index: 2,
            to_party_index: 1,
        };
        assert_eq!(transfer.from_party_index, 0);
        assert_eq!(transfer.from_slot_index, 2);
        assert_eq!(transfer.to_party_index, 1);
    }

    // ------------------------------------------------------------------
    // 3.4.4  render_character_panel — empty inventory
    // ------------------------------------------------------------------

    /// `render_character_panel` must not panic when the character's inventory
    /// is empty.
    #[test]
    fn test_render_character_panel_does_not_panic_empty_inventory() {
        use crate::domain::character::{Alignment, Character, Sex};

        let mut global_state = GlobalState(GameState::new());

        // Add one party member with an empty inventory
        let character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        global_state
            .0
            .party
            .add_member(character)
            .expect("add_member failed");

        // Build a minimal egui context so we can drive rendering
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_character_panel(
                    ui,
                    0,    // party_index
                    true, // is_focused
                    None, // selected_slot
                    &global_state,
                    None, // no GameContent needed
                );
            });
        });
        // No panic = test passes
    }

    // ------------------------------------------------------------------
    // 3.4.5  render_character_panel — full inventory
    // ------------------------------------------------------------------

    /// `render_character_panel` must not panic when the character has
    /// `Inventory::MAX_ITEMS` slots filled.
    #[test]
    fn test_render_character_panel_does_not_panic_full_inventory() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::types::ItemId;

        let mut global_state = GlobalState(GameState::new());

        let mut character = Character::new(
            "FullBag".to_string(),
            "dwarf".to_string(),
            "robber".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );

        // Fill inventory to MAX_ITEMS
        for i in 0..Inventory::MAX_ITEMS {
            let slot = InventorySlot {
                item_id: i as ItemId,
                charges: 0,
            };
            character.inventory.items.push(slot);
        }
        assert_eq!(character.inventory.items.len(), Inventory::MAX_ITEMS);

        global_state
            .0
            .party
            .add_member(character)
            .expect("add_member failed");

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_character_panel(
                    ui,
                    0,       // party_index
                    false,   // not focused
                    Some(0), // first slot selected
                    &global_state,
                    None, // no GameContent
                );
            });
        });
        // No panic = test passes
    }

    // ------------------------------------------------------------------
    // Extra: out-of-bounds party_index is silently ignored
    // ------------------------------------------------------------------

    /// `render_character_panel` with an out-of-range `party_index` must not
    /// panic; it should simply return without rendering.
    #[test]
    fn test_render_character_panel_out_of_bounds_party_index() {
        let global_state = GlobalState(GameState::new());
        // Party is empty; party_index=0 is out of bounds

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_character_panel(
                    ui,
                    0, // out-of-bounds
                    true,
                    None,
                    &global_state,
                    None,
                );
            });
        });
        // No panic = test passes
    }

    // ------------------------------------------------------------------
    // Extra: InventoryNavigationState debug formatting
    // ------------------------------------------------------------------

    /// Verifies that `InventoryNavigationState` implements `Debug`.
    #[test]
    fn test_inventory_navigation_state_debug() {
        let state = InventoryNavigationState {
            selected_slot_index: Some(3),
            focus_on_panel: 1,
        };
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("3"));
        assert!(debug_str.contains("1"));
    }

    // ------------------------------------------------------------------
    // Extra: drop action removes the correct slot
    // ------------------------------------------------------------------

    /// An `inventory_action_system` processing a `DropItemAction` must remove
    /// the targeted slot from the party member's inventory.
    #[test]
    fn test_inventory_action_system_drop_removes_slot() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::types::ItemId;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Register messages
        app.add_message::<DropItemAction>();
        app.add_message::<TransferItemAction>();

        // Build game state with one party member that has two items
        let mut game_state = GameState::new();
        let mut character = Character::new(
            "Dropper".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let slot0 = InventorySlot {
            item_id: 10 as ItemId,
            charges: 0,
        };
        let slot1 = InventorySlot {
            item_id: 20 as ItemId,
            charges: 0,
        };
        character.inventory.items.push(slot0);
        character.inventory.items.push(slot1);
        game_state.party.add_member(character).unwrap();

        // Set mode to Inventory so the action system runs
        game_state.enter_inventory();
        app.insert_resource(GlobalState(game_state));

        // Add the action system
        app.add_systems(Update, inventory_action_system);

        // Queue a drop event for slot 0
        app.world_mut().write_message(DropItemAction {
            party_index: 0,
            slot_index: 0,
        });

        app.update();

        let gs = app.world().resource::<GlobalState>();
        // slot 0 (item_id=10) should be gone; slot 1 (item_id=20) remains
        assert_eq!(gs.0.party.members[0].inventory.items.len(), 1);
        assert_eq!(gs.0.party.members[0].inventory.items[0].item_id, 20);
    }

    // ------------------------------------------------------------------
    // Extra: transfer action moves item between party members
    // ------------------------------------------------------------------

    /// `inventory_action_system` processing a `TransferItemAction` must move
    /// the item from the source to the destination inventory.
    #[test]
    fn test_inventory_action_system_transfer_moves_item() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::types::ItemId;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.add_message::<DropItemAction>();
        app.add_message::<TransferItemAction>();

        let mut game_state = GameState::new();

        let mut giver = Character::new(
            "Giver".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        giver.inventory.items.push(InventorySlot {
            item_id: 42 as ItemId,
            charges: 0,
        });

        let receiver = Character::new(
            "Receiver".to_string(),
            "elf".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );

        game_state.party.add_member(giver).unwrap();
        game_state.party.add_member(receiver).unwrap();

        game_state.enter_inventory();
        app.insert_resource(GlobalState(game_state));

        app.add_systems(Update, inventory_action_system);

        app.world_mut().write_message(TransferItemAction {
            from_party_index: 0,
            from_slot_index: 0,
            to_party_index: 1,
        });

        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(gs.0.party.members[0].inventory.items.len(), 0);
        assert_eq!(gs.0.party.members[1].inventory.items.len(), 1);
        assert_eq!(gs.0.party.members[1].inventory.items[0].item_id, 42);
    }
}
