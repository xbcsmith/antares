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

/// Emitted when the player confirms dropping a selected item.
///
/// Removing an item places it nowhere — the item is discarded from the party
/// entirely. World-drop rendering is deferred to a later phase.
///
/// # Examples
///
/// ```
/// use antares::game::systems::inventory_ui::DropItemAction;
///
/// let action = DropItemAction { party_index: 0, slot_index: 2 };
/// assert_eq!(action.party_index, 0);
/// assert_eq!(action.slot_index, 2);
/// ```
#[derive(Message)]
pub struct DropItemAction {
    /// Index of the party member (0-based) whose inventory contains the item.
    /// Valid range: `0..party.members.len()`.
    pub party_index: usize,
    /// Index of the slot within that character's inventory to drop.
    /// Valid range: `0..inventory.items.len()`.
    pub slot_index: usize,
}

/// Emitted when the player transfers an item from one character to another.
///
/// The source slot is removed first (returning an owned `InventorySlot`), then
/// the slot is added to the destination. If `add_item` fails, the slot is
/// returned to the source to prevent item loss.
///
/// # Examples
///
/// ```
/// use antares::game::systems::inventory_ui::TransferItemAction;
///
/// let action = TransferItemAction {
///     from_party_index: 0,
///     from_slot_index: 1,
///     to_party_index: 2,
/// };
/// assert_eq!(action.from_party_index, 0);
/// assert_eq!(action.from_slot_index, 1);
/// assert_eq!(action.to_party_index, 2);
/// ```
#[derive(Message)]
pub struct TransferItemAction {
    /// Party index of the character giving the item.
    /// Valid range: `0..party.members.len()`.
    pub from_party_index: usize,
    /// Slot index in the source character's inventory.
    /// Valid range: `0..inventory.items.len()`.
    pub from_slot_index: usize,
    /// Party index of the character receiving the item.
    /// Must differ from `from_party_index`.
    pub to_party_index: usize,
}

// ===== Panel Action =====

/// Represents an action that the player has requested via the inventory UI.
///
/// `render_character_panel` returns `Option<PanelAction>` instead of writing
/// messages directly so that the render helper stays free of `MessageWriter`
/// generics.  The calling system (`inventory_ui_system`) matches on the
/// returned value and writes the appropriate message.
///
/// # Examples
///
/// ```
/// use antares::game::systems::inventory_ui::PanelAction;
///
/// let drop = PanelAction::Drop { party_index: 0, slot_index: 1 };
/// let transfer = PanelAction::Transfer {
///     from_party_index: 0,
///     from_slot_index: 0,
///     to_party_index: 1,
/// };
/// match drop {
///     PanelAction::Drop { party_index, slot_index } => {
///         assert_eq!(party_index, 0);
///         assert_eq!(slot_index, 1);
///     }
///     PanelAction::Transfer { .. } => panic!("unexpected"),
/// }
/// match transfer {
///     PanelAction::Transfer { from_party_index, from_slot_index, to_party_index } => {
///         assert_eq!(from_party_index, 0);
///         assert_eq!(from_slot_index, 0);
///         assert_eq!(to_party_index, 1);
///     }
///     PanelAction::Drop { .. } => panic!("unexpected"),
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub enum PanelAction {
    /// Drop the item at `slot_index` from party member `party_index`.
    Drop {
        /// Party member index of the owner.
        party_index: usize,
        /// Inventory slot index to drop.
        slot_index: usize,
    },
    /// Transfer the item from `from_slot_index` on `from_party_index` to
    /// `to_party_index`.
    Transfer {
        /// Party index of the character giving the item.
        from_party_index: usize,
        /// Slot index in the source character's inventory.
        from_slot_index: usize,
        /// Party index of the character receiving the item.
        to_party_index: usize,
    },
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
///
/// When a slot is selected and the focused character has an item at that slot,
/// an action row with "Drop" and "Give to {name}" buttons is rendered beneath
/// the slot listing.  Button clicks return a `PanelAction` from
/// `render_character_panel` which is then dispatched as the appropriate message.
#[allow(clippy::too_many_lines)]
fn inventory_ui_system(
    mut contexts: EguiContexts,
    global_state: Res<GlobalState>,
    nav_state: Res<InventoryNavigationState>,
    game_content: Option<Res<GameContent>>,
    mut drop_writer: MessageWriter<DropItemAction>,
    mut transfer_writer: MessageWriter<TransferItemAction>,
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

    // Collect the names of all open-panel characters for "Give to" labels.
    // We snapshot these upfront to avoid re-borrowing inside the closure.
    let panel_names: Vec<(usize, String)> = open_panels
        .iter()
        .filter_map(|&pi| {
            global_state
                .0
                .party
                .members
                .get(pi)
                .map(|m| (pi, m.name.clone()))
        })
        .collect();

    // Accumulate any action requested inside the egui closure so we can write
    // messages after the closure returns (closures cannot capture &mut writers).
    let mut pending_action: Option<PanelAction> = None;

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
                    let action = render_character_panel(
                        ui,
                        party_index,
                        is_focused,
                        selected_slot,
                        &global_state,
                        game_content.as_deref(),
                        &panel_names,
                    );
                    if action.is_some() {
                        pending_action = action;
                    }
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

        // Keyboard hint for action buttons
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Select a slot with Arrow keys, then use the action buttons above")
                .small()
                .weak(),
        );
    });

    // Dispatch the action that was requested inside the egui closure.
    if let Some(action) = pending_action {
        match action {
            PanelAction::Drop {
                party_index,
                slot_index,
            } => {
                drop_writer.write(DropItemAction {
                    party_index,
                    slot_index,
                });
            }
            PanelAction::Transfer {
                from_party_index,
                from_slot_index,
                to_party_index,
            } => {
                transfer_writer.write(TransferItemAction {
                    from_party_index,
                    from_slot_index,
                    to_party_index,
                });
            }
        }
    }
}

/// Renders a single character's inventory panel and returns any action the
/// player requested via button click.
///
/// # Arguments
///
/// * `ui` – The egui `Ui` to render into.
/// * `party_index` – Zero-based index into `global_state.0.party.members`.
/// * `is_focused` – Whether this panel currently has keyboard focus.
/// * `selected_slot` – Highlighted slot index within this panel (if any).
/// * `global_state` – Read-only reference to global game state.
/// * `game_content` – Optional content database for item name lookups.
/// * `panel_names` – Snapshot of `(party_index, name)` for every open panel,
///   used to label "Give to {name}" transfer buttons.
///
/// # Returns
///
/// `Some(PanelAction)` when the player clicked Drop or a Give-to button;
/// `None` otherwise.
#[allow(clippy::too_many_arguments)]
fn render_character_panel(
    ui: &mut egui::Ui,
    party_index: usize,
    is_focused: bool,
    selected_slot: Option<usize>,
    global_state: &GlobalState,
    game_content: Option<&GameContent>,
    panel_names: &[(usize, String)],
) -> Option<PanelAction> {
    // Bounds-check: silently skip if party_index is out of range
    if party_index >= global_state.0.party.members.len() {
        return None;
    }

    let character = &global_state.0.party.members[party_index];
    // Collect the result of any button click so we can return it after the
    // closure chain completes.
    let mut panel_action: Option<PanelAction> = None;

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
            ui.set_min_width(160.0);

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

            // ── Action row ──────────────────────────────────────────────────
            // Only rendered when a slot is selected AND it contains an item.
            if let Some(slot_idx) = selected_slot {
                if slot_idx < character.inventory.items.len() {
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Unique egui ID scope for the action row per AGENTS.md rules
                    ui.push_id("actions", |ui| {
                        ui.label(egui::RichText::new("Actions:").strong().small());

                        // Drop button
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Drop")
                                        .color(egui::Color32::from_rgb(220, 80, 80)),
                                )
                                .small(),
                            )
                            .on_hover_text("Discard this item permanently")
                            .clicked()
                        {
                            panel_action = Some(PanelAction::Drop {
                                party_index,
                                slot_index: slot_idx,
                            });
                        }

                        // "Give to {name}" buttons for every other open panel
                        for &(other_index, ref other_name) in panel_names {
                            if other_index == party_index {
                                continue;
                            }
                            let give_label = format!("Give to {}", other_name);
                            let target_full = global_state.0.party.members[other_index]
                                .inventory
                                .is_full();
                            if ui
                                .add_enabled(
                                    !target_full,
                                    egui::Button::new(
                                        egui::RichText::new(&give_label)
                                            .color(egui::Color32::from_rgb(100, 200, 100)),
                                    )
                                    .small(),
                                )
                                .on_hover_text(if target_full {
                                    format!("{}'s inventory is full", other_name)
                                } else {
                                    format!("Transfer item to {}", other_name)
                                })
                                .clicked()
                            {
                                panel_action = Some(PanelAction::Transfer {
                                    from_party_index: party_index,
                                    from_slot_index: slot_idx,
                                    to_party_index: other_index,
                                });
                            }
                        }
                    });
                }
            }
        });
    });

    panel_action
}

// ===== Action System =====

/// Processes `DropItemAction` and `TransferItemAction` events each frame.
///
/// **Drop semantics**: the item is removed from the character's inventory and
/// discarded.  The `InventoryState.selected_slot` is reset to `None`.
///
/// **Transfer semantics**: the item is removed from the source inventory and
/// added to the destination inventory.  If `add_item` returns an error (e.g.
/// destination full) the item is put back into the source inventory to prevent
/// item loss.  `InventoryState.selected_slot` is reset on success.
///
/// All operations are bounds-checked; out-of-range indices produce a warning
/// log and no mutation.
fn inventory_action_system(
    mut drop_reader: MessageReader<DropItemAction>,
    mut transfer_reader: MessageReader<TransferItemAction>,
    mut global_state: ResMut<GlobalState>,
) {
    // Collect messages upfront so we do not hold a borrow while mutating state.
    let drop_events: Vec<(usize, usize)> = drop_reader
        .read()
        .map(|e| (e.party_index, e.slot_index))
        .collect();

    let transfer_events: Vec<(usize, usize, usize)> = transfer_reader
        .read()
        .map(|e| (e.from_party_index, e.from_slot_index, e.to_party_index))
        .collect();

    // ── Drop events ─────────────────────────────────────────────────────────
    for (party_index, slot_index) in drop_events {
        let party = &mut global_state.0.party;

        // Bounds-check party index
        if party_index >= party.members.len() {
            warn!(
                "DropItemAction: party_index {} out of bounds (party size {})",
                party_index,
                party.members.len()
            );
            continue;
        }

        // Bounds-check slot index
        let inv_len = party.members[party_index].inventory.items.len();
        if slot_index >= inv_len {
            warn!(
                "DropItemAction: slot_index {} out of bounds (inventory size {}) for party[{}]",
                slot_index, inv_len, party_index
            );
            continue;
        }

        // Remove the item — remove_item returns Option<InventorySlot>
        if let Some(dropped) = party.members[party_index].inventory.remove_item(slot_index) {
            info!(
                "Dropped item from party[{}] slot {} (item_id={})",
                party_index, slot_index, dropped.item_id
            );
        }

        // Clear selected_slot in InventoryState
        if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
            inv_state.selected_slot = None;
        }
    }

    // ── Transfer events ──────────────────────────────────────────────────────
    for (from_party_index, from_slot_index, to_party_index) in transfer_events {
        // Transferring to yourself is a no-op
        if from_party_index == to_party_index {
            warn!(
                "TransferItemAction: from_party_index == to_party_index ({}); ignoring",
                from_party_index
            );
            continue;
        }

        let party = &mut global_state.0.party;

        // Bounds-check both party indices
        if from_party_index >= party.members.len() {
            warn!(
                "TransferItemAction: from_party_index {} out of bounds (party size {})",
                from_party_index,
                party.members.len()
            );
            continue;
        }
        if to_party_index >= party.members.len() {
            warn!(
                "TransferItemAction: to_party_index {} out of bounds (party size {})",
                to_party_index,
                party.members.len()
            );
            continue;
        }

        // Bounds-check source slot index
        let src_inv_len = party.members[from_party_index].inventory.items.len();
        if from_slot_index >= src_inv_len {
            warn!(
                "TransferItemAction: from_slot_index {} out of bounds (source inventory size {}) for party[{}]",
                from_slot_index, src_inv_len, from_party_index
            );
            continue;
        }

        // Check destination capacity before removing from source
        if party.members[to_party_index].inventory.is_full() {
            warn!(
                "Transfer failed: target party[{}] inventory is full",
                to_party_index
            );
            continue;
        }

        // Remove from source — returns owned InventorySlot; borrow released here
        let slot = match party.members[from_party_index]
            .inventory
            .remove_item(from_slot_index)
        {
            Some(s) => s,
            None => {
                // Defensive: item was already removed by a concurrent message
                warn!(
                    "TransferItemAction: no item at party[{}] slot {} (already removed?)",
                    from_party_index, from_slot_index
                );
                continue;
            }
        };

        // Add to destination; rollback to source on failure to prevent item loss
        match party.members[to_party_index]
            .inventory
            .add_item(slot.item_id, slot.charges)
        {
            Ok(()) => {
                info!(
                    "Transferred item (item_id={}) from party[{}] slot {} to party[{}]",
                    slot.item_id, from_party_index, from_slot_index, to_party_index
                );
                // Clear selected_slot on success
                if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
                    inv_state.selected_slot = None;
                }
            }
            Err(err) => {
                // Rollback: return item to the source inventory
                warn!(
                    "TransferItemAction: add_item to party[{}] failed ({:?}); rolling back to party[{}]",
                    to_party_index, err, from_party_index
                );
                // Best-effort rollback — if source is somehow also full, log
                if let Err(rollback_err) = party.members[from_party_index]
                    .inventory
                    .add_item(slot.item_id, slot.charges)
                {
                    error!(
                        "TransferItemAction ROLLBACK FAILED for party[{}] (item_id={}): {:?}. Item lost!",
                        from_party_index, slot.item_id, rollback_err
                    );
                }
            }
        }
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
                    &[],  // no open panels for transfer buttons
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
                    &[],  // no open panels for transfer buttons
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
                    &[],
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
        // selected_slot must be cleared
        if let GameMode::Inventory(ref inv_state) = gs.0.mode {
            assert_eq!(inv_state.selected_slot, None);
        }
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
        // selected_slot must be cleared on success
        if let GameMode::Inventory(ref inv_state) = gs.0.mode {
            assert_eq!(inv_state.selected_slot, None);
        }
    }

    // ------------------------------------------------------------------
    // 4.4.1  Drop removes item and clears selected_slot
    // ------------------------------------------------------------------

    /// Full Phase 4 test: a `DropItemAction` removes the item from inventory
    /// and sets `selected_slot` to `None`.
    #[test]
    fn test_drop_item_action_removes_from_inventory() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::types::ItemId;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<DropItemAction>();
        app.add_message::<TransferItemAction>();

        let mut game_state = GameState::new();
        let mut character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        character.inventory.items.push(InventorySlot {
            item_id: 99 as ItemId,
            charges: 3,
        });
        game_state.party.add_member(character).unwrap();

        // Open inventory with slot 0 pre-selected
        game_state.enter_inventory();
        if let GameMode::Inventory(ref mut inv_state) = game_state.mode {
            inv_state.selected_slot = Some(0);
        }
        app.insert_resource(GlobalState(game_state));
        app.add_systems(Update, inventory_action_system);

        app.world_mut().write_message(DropItemAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            gs.0.party.members[0].inventory.items.is_empty(),
            "inventory should be empty after drop"
        );
        if let GameMode::Inventory(ref inv_state) = gs.0.mode {
            assert_eq!(
                inv_state.selected_slot, None,
                "selected_slot must be cleared after drop"
            );
        }
    }

    // ------------------------------------------------------------------
    // 4.4.2  Drop with invalid slot_index does not panic
    // ------------------------------------------------------------------

    /// A `DropItemAction` with an out-of-bounds `slot_index` must not panic
    /// and must leave inventory unchanged.
    #[test]
    fn test_drop_item_action_invalid_index_no_panic() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::types::ItemId;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<DropItemAction>();
        app.add_message::<TransferItemAction>();

        let mut game_state = GameState::new();
        let mut character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        character.inventory.items.push(InventorySlot {
            item_id: 1 as ItemId,
            charges: 0,
        });
        game_state.party.add_member(character).unwrap();
        game_state.enter_inventory();
        app.insert_resource(GlobalState(game_state));
        app.add_systems(Update, inventory_action_system);

        // slot_index=99 is out of bounds
        app.world_mut().write_message(DropItemAction {
            party_index: 0,
            slot_index: 99,
        });
        app.update();

        let gs = app.world().resource::<GlobalState>();
        // Inventory unchanged — still one item
        assert_eq!(
            gs.0.party.members[0].inventory.items.len(),
            1,
            "inventory should be unchanged after invalid drop"
        );
    }

    // ------------------------------------------------------------------
    // 4.4.3  Drop with invalid party_index does not panic
    // ------------------------------------------------------------------

    /// A `DropItemAction` with an out-of-bounds `party_index` must not panic.
    #[test]
    fn test_drop_item_invalid_party_index_no_panic() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<DropItemAction>();
        app.add_message::<TransferItemAction>();

        let mut game_state = GameState::new();
        // Empty party
        game_state.enter_inventory();
        app.insert_resource(GlobalState(game_state));
        app.add_systems(Update, inventory_action_system);

        // party_index=99 — no such member
        app.world_mut().write_message(DropItemAction {
            party_index: 99,
            slot_index: 0,
        });
        app.update();
        // No panic = test passes
    }

    // ------------------------------------------------------------------
    // 4.4.4  Transfer moves item between characters, gold unchanged
    // ------------------------------------------------------------------

    /// A successful transfer must move the item and not modify gold.
    #[test]
    fn test_transfer_item_character_to_character_success() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::types::ItemId;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<DropItemAction>();
        app.add_message::<TransferItemAction>();

        let mut game_state = GameState::new();

        let mut src = Character::new(
            "Source".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        src.gold = 100;
        src.inventory.items.push(InventorySlot {
            item_id: 7 as ItemId,
            charges: 2,
        });

        let mut dst = Character::new(
            "Destination".to_string(),
            "elf".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        dst.gold = 50;

        game_state.party.add_member(src).unwrap();
        game_state.party.add_member(dst).unwrap();
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
        // Source inventory empty
        assert!(
            gs.0.party.members[0].inventory.items.is_empty(),
            "source inventory should be empty"
        );
        // Destination has the item with correct charges
        assert_eq!(gs.0.party.members[1].inventory.items.len(), 1);
        assert_eq!(gs.0.party.members[1].inventory.items[0].item_id, 7);
        assert_eq!(gs.0.party.members[1].inventory.items[0].charges, 2);
        // Gold unchanged
        assert_eq!(
            gs.0.party.members[0].gold, 100,
            "source gold should be unchanged"
        );
        assert_eq!(
            gs.0.party.members[1].gold, 50,
            "destination gold should be unchanged"
        );
    }

    // ------------------------------------------------------------------
    // 4.4.5  Transfer fails when target inventory is full
    // ------------------------------------------------------------------

    /// When the target character's inventory is at `MAX_ITEMS`, the transfer
    /// must be rejected and both inventories must remain unchanged.
    #[test]
    fn test_transfer_item_target_inventory_full() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::types::ItemId;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<DropItemAction>();
        app.add_message::<TransferItemAction>();

        let mut game_state = GameState::new();

        let mut src = Character::new(
            "Source".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        src.inventory.items.push(InventorySlot {
            item_id: 5 as ItemId,
            charges: 0,
        });

        let mut dst = Character::new(
            "FullDst".to_string(),
            "dwarf".to_string(),
            "robber".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        // Fill destination to capacity
        for i in 0..Inventory::MAX_ITEMS {
            dst.inventory.items.push(InventorySlot {
                item_id: (100 + i) as ItemId,
                charges: 0,
            });
        }
        assert!(dst.inventory.is_full());

        game_state.party.add_member(src).unwrap();
        game_state.party.add_member(dst).unwrap();
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
        // Source inventory unchanged (still has item_id=5)
        assert_eq!(
            gs.0.party.members[0].inventory.items.len(),
            1,
            "source inventory should be unchanged"
        );
        assert_eq!(gs.0.party.members[0].inventory.items[0].item_id, 5);
        // Destination still full
        assert_eq!(
            gs.0.party.members[1].inventory.items.len(),
            Inventory::MAX_ITEMS,
            "destination inventory should still be full"
        );
    }

    // ------------------------------------------------------------------
    // 4.4.6  Transfer with out-of-bounds from_slot_index does not panic
    // ------------------------------------------------------------------

    /// A `TransferItemAction` with a `from_slot_index` beyond the source
    /// inventory length must not panic and must not mutate any inventory.
    #[test]
    fn test_transfer_item_no_item_at_source_slot() {
        use crate::domain::character::{Alignment, Character, Sex};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<DropItemAction>();
        app.add_message::<TransferItemAction>();

        let mut game_state = GameState::new();

        let src = Character::new(
            "EmptySrc".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let dst = Character::new(
            "Dst".to_string(),
            "elf".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        game_state.party.add_member(src).unwrap();
        game_state.party.add_member(dst).unwrap();
        game_state.enter_inventory();
        app.insert_resource(GlobalState(game_state));
        app.add_systems(Update, inventory_action_system);

        // from_slot_index=5 is beyond the empty source inventory
        app.world_mut().write_message(TransferItemAction {
            from_party_index: 0,
            from_slot_index: 5,
            to_party_index: 1,
        });
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(gs.0.party.members[0].inventory.items.is_empty());
        assert!(gs.0.party.members[1].inventory.items.is_empty());
    }

    // ------------------------------------------------------------------
    // 4.4.7  PanelAction::Drop variant field values
    // ------------------------------------------------------------------

    /// `PanelAction::Drop` must carry `party_index` and `slot_index` correctly.
    #[test]
    fn test_panel_action_drop_variant() {
        let action = PanelAction::Drop {
            party_index: 0,
            slot_index: 1,
        };
        match action {
            PanelAction::Drop {
                party_index,
                slot_index,
            } => {
                assert_eq!(party_index, 0, "party_index should be 0");
                assert_eq!(slot_index, 1, "slot_index should be 1");
            }
            PanelAction::Transfer { .. } => panic!("expected Drop variant"),
        }
    }

    // ------------------------------------------------------------------
    // 4.4.8  PanelAction::Transfer variant field values
    // ------------------------------------------------------------------

    /// `PanelAction::Transfer` must carry all three indices correctly.
    #[test]
    fn test_panel_action_transfer_variant() {
        let action = PanelAction::Transfer {
            from_party_index: 0,
            from_slot_index: 0,
            to_party_index: 1,
        };
        match action {
            PanelAction::Transfer {
                from_party_index,
                from_slot_index,
                to_party_index,
            } => {
                assert_eq!(from_party_index, 0, "from_party_index should be 0");
                assert_eq!(from_slot_index, 0, "from_slot_index should be 0");
                assert_eq!(to_party_index, 1, "to_party_index should be 1");
            }
            PanelAction::Drop { .. } => panic!("expected Transfer variant"),
        }
    }

    // ------------------------------------------------------------------
    // Extra: render_character_panel renders action buttons when slot selected
    // ------------------------------------------------------------------

    /// When a slot with an item is selected and a second panel is open,
    /// `render_character_panel` should not panic and should return `None` when
    /// no button is clicked (no simulated click in headless egui context).
    #[test]
    fn test_render_character_panel_action_row_no_panic_with_two_panels() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::types::ItemId;

        let mut global_state = GlobalState(GameState::new());

        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.inventory.items.push(InventorySlot {
            item_id: 1 as ItemId,
            charges: 0,
        });

        let ally = Character::new(
            "Ally".to_string(),
            "elf".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );

        global_state.0.party.add_member(hero).unwrap();
        global_state.0.party.add_member(ally).unwrap();

        let panel_names = vec![(0usize, "Hero".to_string()), (1usize, "Ally".to_string())];

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // No simulated click occurs in headless context; return value is None.
                let action = render_character_panel(
                    ui,
                    0,       // party_index
                    true,    // is_focused
                    Some(0), // slot 0 selected (has item)
                    &global_state,
                    None,
                    &panel_names,
                );
                assert!(action.is_none(), "no button clicked, should return None");
            });
        });
    }

    // ------------------------------------------------------------------
    // Extra: PanelAction implements Debug and PartialEq
    // ------------------------------------------------------------------

    /// `PanelAction` must implement `Debug` and `PartialEq` for test assertions.
    #[test]
    fn test_panel_action_debug_and_eq() {
        let a = PanelAction::Drop {
            party_index: 1,
            slot_index: 2,
        };
        let b = PanelAction::Drop {
            party_index: 1,
            slot_index: 2,
        };
        assert_eq!(a, b);

        let c = PanelAction::Transfer {
            from_party_index: 0,
            from_slot_index: 1,
            to_party_index: 2,
        };
        let debug_str = format!("{:?}", c);
        assert!(debug_str.contains("Transfer"));
        assert!(debug_str.contains("from_party_index"));
    }
}
