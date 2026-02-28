// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Inventory UI System - Character inventory management interface
//!
//! Provides an egui-based overlay for viewing and managing character inventory
//! when the game is in `GameMode::Inventory` mode. This system is active when
//! the player presses the configured inventory key (default: `I`).
//!
//! ## Keyboard Navigation (two-phase model)
//!
//! ### Phase 1 — Slot Navigation
//!
//! | Key              | Effect                                                      |
//! |------------------|-------------------------------------------------------------|
//! | `Tab`            | Advance focus to the next character panel (yellow border)   |
//! | `Shift+Tab`      | Move focus to the previous character panel                  |
//! | `←` `→` `↑` `↓` | Navigate the slot grid inside the focused panel             |
//! | `Enter`          | Enter **Action Navigation** for the highlighted slot        |
//! | `Esc` / `I`      | Close the inventory and resume the previous game mode       |
//!
//! ### Phase 2 — Action Navigation
//!
//! | Key         | Effect                                                             |
//! |-------------|--------------------------------------------------------------------|
//! | `←` `→`     | Cycle between action buttons (Drop / Give→ …)                      |
//! | `Enter`      | Execute the focused action; return focus to slot 0 of the grid     |
//! | `Esc`        | Cancel; return to Slot Navigation at the previously selected slot   |
//!
//! Follows the `InnUiPlugin` pattern from `src/game/systems/inn_ui.rs` exactly.

use crate::application::resources::GameContent;
use crate::application::GameMode;
use crate::domain::character::{Inventory, PARTY_MAX_SIZE};
use crate::domain::items::types::ItemType;
use crate::game::resources::GlobalState;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

// ===== Layout constants =====

/// Height of the character name header bar inside each panel.
const PANEL_HEADER_H: f32 = 36.0;
/// Height of the action button strip below the grid when a slot is selected.
const PANEL_ACTION_H: f32 = 48.0;
/// Number of slot columns in the grid inside each character panel.
/// With MAX_ITEMS=64 and SLOT_COLS=8 the grid is 8×8.
const SLOT_COLS: usize = 8;
/// Grid line colour — faint white.
const GRID_LINE_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(60, 60, 60, 255);
/// Panel body background colour.
const PANEL_BG_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(18, 18, 18, 255);
/// Header background colour.
const HEADER_BG_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(35, 35, 35, 255);
/// Colour for item silhouettes.
const ITEM_SILHOUETTE_COLOR: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(230, 230, 230, 255);
/// Colour for the slot/action selection highlight ring.
const SELECT_HIGHLIGHT_COLOR: egui::Color32 = egui::Color32::YELLOW;
/// Focused panel border colour.
const FOCUSED_BORDER_COLOR: egui::Color32 = egui::Color32::YELLOW;
/// Unfocused panel border colour.
const UNFOCUSED_BORDER_COLOR: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(80, 80, 80, 255);
/// Action button highlight colour when keyboard focus is on it.
const ACTION_FOCUSED_COLOR: egui::Color32 = egui::Color32::YELLOW;

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

// ===== Navigation Phase =====

/// The two phases of keyboard inventory navigation.
///
/// The player starts in `SlotNavigation`. Pressing Enter while a slot with an
/// item is highlighted advances to `ActionNavigation`. Pressing Enter executes
/// the focused action and returns to `SlotNavigation` at slot 0. Pressing Esc
/// cancels and returns to `SlotNavigation` at the previously highlighted slot.
///
/// # Examples
///
/// ```
/// use antares::game::systems::inventory_ui::NavigationPhase;
///
/// let phase = NavigationPhase::default();
/// assert!(matches!(phase, NavigationPhase::SlotNavigation));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NavigationPhase {
    /// Arrows navigate the slot grid; Enter enters action mode.
    #[default]
    SlotNavigation,
    /// Left/Right arrows cycle action buttons; Enter executes; Esc cancels.
    ActionNavigation,
}

// ===== Navigation State =====

/// Tracks keyboard navigation state for the inventory overlay.
///
/// Mirrors the pattern of `InnNavigationState` from `inn_ui.rs`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::inventory_ui::{InventoryNavigationState, NavigationPhase};
///
/// let state = InventoryNavigationState::default();
/// assert_eq!(state.selected_slot_index, None);
/// assert_eq!(state.focus_on_panel, 0);
/// assert_eq!(state.focused_action_index, 0);
/// assert!(matches!(state.phase, NavigationPhase::SlotNavigation));
/// ```
#[derive(Resource, Default, Debug)]
pub struct InventoryNavigationState {
    /// Index of the selected slot within the focused panel (`None` = no slot highlighted).
    pub selected_slot_index: Option<usize>,
    /// Which panel column has keyboard focus (maps to `open_panels` index, not `party_index`).
    pub focus_on_panel: usize,
    /// Which action button has keyboard focus when `phase == ActionNavigation`.
    ///
    /// `0` = Drop, `1..N` = Give→ buttons in panel order.
    pub focused_action_index: usize,
    /// Current navigation phase — slot grid or action button row.
    pub phase: NavigationPhase,
}

impl InventoryNavigationState {
    /// Reset to a clean default state.
    fn reset(&mut self) {
        *self = InventoryNavigationState::default();
    }
}

// ===== Helpers =====

/// Build the ordered list of action button descriptors for a focused panel.
///
/// Returns a `Vec<PanelAction>` in the same order the UI renders them:
/// `Drop` first, then one `Transfer` per other open panel member.
///
/// `panel_names` contains `(party_index, name)` for every visible panel.
/// `focused_party_index` is the panel whose actions are being computed.
/// `party_members_len` is used for bounds checking only.
fn build_action_list(
    focused_party_index: usize,
    panel_names: &[(usize, String)],
) -> Vec<PanelAction> {
    let mut actions = Vec::new();
    // Drop is always action 0
    actions.push(PanelAction::Drop {
        party_index: focused_party_index,
        slot_index: 0, // placeholder — filled in at execution time
    });
    // One Transfer per other visible panel, in panel order
    for &(other_index, _) in panel_names {
        if other_index != focused_party_index {
            actions.push(PanelAction::Transfer {
                from_party_index: focused_party_index,
                from_slot_index: 0, // placeholder — filled in at execution time
                to_party_index: other_index,
            });
        }
    }
    actions
}

// ===== Input System =====

/// Handles keyboard input for inventory navigation.
///
/// Runs every frame; only processes input when
/// `GlobalState.0.mode` is `GameMode::Inventory(_)`.
///
/// ## Key routing summary
///
/// | Phase             | Key              | Effect                                              |
/// |-------------------|------------------|-----------------------------------------------------|
/// | Either            | `Esc` (slot)     | Close inventory                                     |
/// | Either            | `Esc` (action)   | Cancel action mode, return to selected slot         |
/// | Either            | `Tab`            | Advance character panel focus (clears slot)         |
/// | Either            | `Shift+Tab`      | Retreat character panel focus (clears slot)         |
/// | SlotNavigation    | `←→↑↓`          | Navigate the slot grid                              |
/// | SlotNavigation    | `Enter`          | Enter ActionNavigation for the highlighted slot     |
/// | ActionNavigation  | `←→`            | Cycle action buttons                                |
/// | ActionNavigation  | `Enter`          | Execute focused action; return to slot 0            |
#[allow(clippy::too_many_lines)]
fn inventory_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut global_state: ResMut<GlobalState>,
    mut nav_state: ResMut<InventoryNavigationState>,
    mut drop_writer: MessageWriter<DropItemAction>,
    mut transfer_writer: MessageWriter<TransferItemAction>,
) {
    // Bail if not in inventory mode; reset nav state for next entry.
    let party_size = match &global_state.0.mode {
        GameMode::Inventory(_) => global_state.0.party.members.len().min(PARTY_MAX_SIZE),
        _ => {
            nav_state.reset();
            return;
        }
    };

    // ── Collect open panels and focused party index ────────────────────────
    let (focused_party_index, open_panels_snapshot) = match &global_state.0.mode {
        GameMode::Inventory(s) => (s.focused_index, s.open_panels.clone()),
        _ => return,
    };

    // Build panel_names from open panels (same logic as inventory_ui_system)
    let panel_names: Vec<(usize, String)> = open_panels_snapshot
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

    // ── Phase: ActionNavigation ────────────────────────────────────────────
    if nav_state.phase == NavigationPhase::ActionNavigation {
        let slot_idx = match nav_state.selected_slot_index {
            Some(s) => s,
            None => {
                // Guard: no slot selected; drop back to slot navigation
                nav_state.phase = NavigationPhase::SlotNavigation;
                return;
            }
        };

        // Esc in action mode — cancel, return to slot navigation
        if keyboard.just_pressed(KeyCode::Escape) {
            nav_state.phase = NavigationPhase::SlotNavigation;
            return;
        }

        // Build the action list for the focused panel
        let actions = build_action_list(focused_party_index, &panel_names);
        let action_count = actions.len();

        if action_count == 0 {
            nav_state.phase = NavigationPhase::SlotNavigation;
            return;
        }

        // Left/Right — cycle action button focus
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

        // Enter — execute focused action then return to slot 0
        if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter) {
            let action_idx = nav_state.focused_action_index.min(action_count - 1);
            match &actions[action_idx] {
                PanelAction::Drop { party_index, .. } => {
                    drop_writer.write(DropItemAction {
                        party_index: *party_index,
                        slot_index: slot_idx,
                    });
                }
                PanelAction::Transfer {
                    from_party_index,
                    to_party_index,
                    ..
                } => {
                    transfer_writer.write(TransferItemAction {
                        from_party_index: *from_party_index,
                        from_slot_index: slot_idx,
                        to_party_index: *to_party_index,
                    });
                }
            }
            // Return to slot 0 in slot-navigation phase
            if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
                inv_state.selected_slot = Some(0);
            }
            nav_state.selected_slot_index = Some(0);
            nav_state.focused_action_index = 0;
            nav_state.phase = NavigationPhase::SlotNavigation;
            return;
        }

        // Any other key in action mode is ignored
        return;
    }

    // ── Phase: SlotNavigation ──────────────────────────────────────────────

    // NOTE: The configured inventory toggle key ("I" by default) is intentionally
    // NOT handled here.  `handle_input` (InputPlugin) owns the open/close toggle
    // for that key.  Duplicating it here would cause the inventory to open and
    // close in the same frame because both systems run in Update with no ordering
    // guarantee between them.

    // Esc in slot mode — close inventory
    if keyboard.just_pressed(KeyCode::Escape) {
        let resume_mode = match &global_state.0.mode {
            GameMode::Inventory(s) => s.get_resume_mode(),
            _ => return,
        };
        global_state.0.mode = resume_mode;
        nav_state.reset();
        return;
    }

    // ── Tab / Shift-Tab — cycle the yellow-border panel focus ─────────────
    //
    // TAB only changes which character panel has focus. It does NOT affect
    // the slot grid cursor. Slot selection is cleared whenever focus changes
    // because the cursor position belongs to a specific character's panel.
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    let focus_next = keyboard.just_pressed(KeyCode::Tab) && !shift_held;
    let focus_prev = keyboard.just_pressed(KeyCode::Tab) && shift_held;

    if focus_next || focus_prev {
        if let GameMode::Inventory(inv_state) = &mut global_state.0.mode {
            if focus_prev {
                inv_state.focused_index = if inv_state.focused_index == 0 {
                    party_size.saturating_sub(1)
                } else {
                    inv_state.focused_index - 1
                };
                // Ensure newly focused panel is in open_panels
                if !inv_state.open_panels.contains(&inv_state.focused_index)
                    && inv_state.open_panels.len() < PARTY_MAX_SIZE
                {
                    inv_state.open_panels.push(inv_state.focused_index);
                }
            } else {
                inv_state.focused_index = if party_size == 0 {
                    0
                } else {
                    (inv_state.focused_index + 1) % party_size
                };
                // Ensure newly focused panel is in open_panels
                if !inv_state.open_panels.contains(&inv_state.focused_index)
                    && inv_state.open_panels.len() < PARTY_MAX_SIZE
                {
                    inv_state.open_panels.push(inv_state.focused_index);
                }
            }
            // Clear slot selection — cursor stays at the new panel's grid
            inv_state.selected_slot = None;
        }
        nav_state.selected_slot_index = None;
        nav_state.focused_action_index = 0;
        return;
    }

    // ── Enter — confirm slot selection → enter ActionNavigation ───────────
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter) {
        if let Some(slot_idx) = nav_state.selected_slot_index {
            // Only enter action mode if the slot actually contains an item
            let has_item = global_state
                .0
                .party
                .members
                .get(focused_party_index)
                .map(|ch| slot_idx < ch.inventory.items.len())
                .unwrap_or(false);
            if has_item {
                nav_state.phase = NavigationPhase::ActionNavigation;
                nav_state.focused_action_index = 0;
            }
        } else {
            // No slot highlighted yet — Enter starts navigation at slot 0
            if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
                inv_state.selected_slot = Some(0);
            }
            nav_state.selected_slot_index = Some(0);
        }
        return;
    }

    // ── Arrow keys — navigate the slot grid in the focused panel ──────────
    //
    // The grid is SLOT_COLS (8) columns × (MAX_ITEMS/SLOT_COLS = 8) rows.
    // Left/Right move one column; Up/Down move one full row (SLOT_COLS slots).
    // All movement wraps within 0..MAX_ITEMS.
    // The first press with no selection starts at slot 0.
    let max_slots = Inventory::MAX_ITEMS;

    let any_arrow = keyboard.just_pressed(KeyCode::ArrowRight)
        || keyboard.just_pressed(KeyCode::ArrowLeft)
        || keyboard.just_pressed(KeyCode::ArrowDown)
        || keyboard.just_pressed(KeyCode::ArrowUp);

    if any_arrow {
        if let GameMode::Inventory(inv_state) = &mut global_state.0.mode {
            let current = inv_state.selected_slot.unwrap_or(0);
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
                // ArrowUp — move one row up, wrapping from top to bottom same column
                if current < SLOT_COLS {
                    let last_row_start = (max_slots / SLOT_COLS).saturating_sub(1) * SLOT_COLS;
                    let col = current % SLOT_COLS;
                    (last_row_start + col).min(max_slots - 1)
                } else {
                    current - SLOT_COLS
                }
            };
            inv_state.selected_slot = Some(next);
            nav_state.selected_slot_index = Some(next);
        }
    }
}

// ===== UI System =====

/// Renders the egui inventory overlay when in `GameMode::Inventory`.
///
/// The full `CentralPanel` is divided into equal-sized character panels laid
/// out in a 3-column grid (2 rows for 4-6 characters, 1 row for 1-3).  Each
/// panel has a dark header bar with the character name and a body filled with
/// a painter-drawn slot grid showing item-type silhouettes — matching the
/// target mockup style.
#[allow(clippy::too_many_lines)]
fn inventory_ui_system(
    mut contexts: EguiContexts,
    global_state: Res<GlobalState>,
    game_content: Option<Res<GameContent>>,
    nav_state: Res<InventoryNavigationState>,
    mut drop_writer: MessageWriter<DropItemAction>,
    mut transfer_writer: MessageWriter<TransferItemAction>,
) {
    let inv_state = match &global_state.0.mode {
        GameMode::Inventory(s) => s.clone(),
        _ => return,
    };

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let open_panels = inv_state.open_panels.clone();
    let focused_index = inv_state.focused_index;
    let selected_slot = inv_state.selected_slot;

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

    let mut pending_action: Option<PanelAction> = None;

    egui::CentralPanel::default().show(ctx, |ui| {
        // ── Top bar: title + close hint ──────────────────────────────────
        ui.horizontal(|ui| {
            ui.heading("Inventory");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new("[I] or [Esc] to close")
                        .italics()
                        .weak(),
                );
            });
        });

        // ── Status line: focused character + selected item ───────────────
        {
            let party = &global_state.0.party;
            if focused_index < party.members.len() {
                let character = &party.members[focused_index];
                let status = match selected_slot {
                    Some(slot_idx) if slot_idx < character.inventory.items.len() => {
                        let slot = &character.inventory.items[slot_idx];
                        let item_name = game_content
                            .as_deref()
                            .and_then(|gc| gc.db().items.get_item(slot.item_id))
                            .map(|item| item.name.clone())
                            .unwrap_or_else(|| format!("Item #{}", slot.item_id));
                        format!(
                            "Focus: {}  |  Selected: {} (slot {})",
                            character.name, item_name, slot_idx
                        )
                    }
                    _ => format!("Focus: {}", character.name),
                };
                ui.label(egui::RichText::new(status).strong());
            }
        }

        // ── Hint line changes based on navigation phase ──────────────────
        let hint = match nav_state.phase {
            NavigationPhase::SlotNavigation => {
                "Tab: cycle character   ←→↑↓: navigate slots   Enter: select item   Esc/I: close"
            }
            NavigationPhase::ActionNavigation => "←→: cycle actions   Enter: execute   Esc: cancel",
        };
        ui.label(egui::RichText::new(hint).small().weak());
        ui.separator();

        // ── Panel layout ─────────────────────────────────────────────────
        // 1–3 panels → 1 row of 3 columns.  4–6 panels → 2 rows of 3 columns.
        let num_panels = open_panels.len().max(1);
        let cols = num_panels.min(3);
        let rows = num_panels.div_ceil(cols);

        let available = ui.available_size();
        // Divide remaining height evenly between rows; subtract inter-row gap.
        let panel_h = ((available.y - (rows as f32 - 1.0) * 4.0) / rows as f32).max(80.0);
        // Each column takes an equal share of the width minus inter-col gaps.
        let panel_w = ((available.x - (cols as f32 - 1.0) * 4.0) / cols as f32).max(80.0);

        // Lay panels out row by row using plain horizontal strips.
        for row in 0..rows {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for col in 0..cols {
                    let panel_pos = row * cols + col;
                    if panel_pos >= open_panels.len() {
                        // Placeholder to keep columns aligned
                        ui.allocate_exact_size(egui::vec2(panel_w, panel_h), egui::Sense::hover());
                        continue;
                    }
                    let party_index = open_panels[panel_pos];
                    let is_focused = party_index == focused_index;
                    // Only pass selected_slot and action focus to the focused panel —
                    // every other panel gets None so highlights only appear on the
                    // active character.
                    let panel_selected = if is_focused { selected_slot } else { None };
                    let panel_action_focus =
                        if is_focused && nav_state.phase == NavigationPhase::ActionNavigation {
                            Some(nav_state.focused_action_index)
                        } else {
                            None
                        };

                    // push_id mandatory per sdk/AGENTS.md
                    ui.push_id(format!("inv_panel_{}", party_index), |ui| {
                        let action = render_character_panel(
                            ui,
                            party_index,
                            is_focused,
                            panel_selected,
                            panel_action_focus,
                            egui::vec2(panel_w, panel_h),
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
            if row + 1 < rows {
                ui.add_space(4.0);
            }
        }
    });

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

/// Renders a single character panel at a fixed pixel size.
///
/// Layout (top to bottom):
/// - Dark header bar (`PANEL_HEADER_H` px) — character name only.
/// - Body — painter-drawn slot grid filling the remaining height.
/// - Action strip (`PANEL_ACTION_H` px) — Drop / Give buttons, only when a
///   filled slot is selected.
///
/// The slot grid is drawn entirely via `egui::Painter` so it looks like the
/// mockup: dark background, faint grid lines, white item-type silhouettes.
///
/// ## Action strip keyboard highlight
///
/// When `focused_action_index` is `Some(n)`, the nth action button in the strip
/// is rendered with a yellow border ring to indicate keyboard focus.  Mouse
/// clicks are still processed normally regardless of keyboard focus.
///
/// # Returns
///
/// `Some(PanelAction)` when the player clicked a Drop or Give button;
/// `None` otherwise.
#[allow(clippy::too_many_arguments)]
fn render_character_panel(
    ui: &mut egui::Ui,
    party_index: usize,
    is_focused: bool,
    selected_slot: Option<usize>,
    focused_action_index: Option<usize>,
    size: egui::Vec2,
    global_state: &GlobalState,
    game_content: Option<&GameContent>,
    panel_names: &[(usize, String)],
) -> Option<PanelAction> {
    if party_index >= global_state.0.party.members.len() {
        return None;
    }

    let character = &global_state.0.party.members[party_index];
    let items = &character.inventory.items;
    let mut panel_action: Option<PanelAction> = None;

    // How much vertical space does the action strip need?
    let has_action = selected_slot.map(|s| s < items.len()).unwrap_or(false);
    let action_reserve = if has_action { PANEL_ACTION_H } else { 0.0 };
    let body_h = (size.y - PANEL_HEADER_H - action_reserve).max(20.0);

    // ── Outer border ─────────────────────────────────────────────────────
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

    // ── Header ───────────────────────────────────────────────────────────
    let header_rect = egui::Rect::from_min_size(panel_rect.min, egui::vec2(size.x, PANEL_HEADER_H));
    painter.rect_filled(header_rect, 0.0, HEADER_BG_COLOR);
    painter.text(
        header_rect.left_center() + egui::vec2(8.0, 0.0),
        egui::Align2::LEFT_CENTER,
        &character.name,
        egui::FontId::proportional(16.0),
        egui::Color32::WHITE,
    );

    // ── Body: slot grid ───────────────────────────────────────────────────
    let body_rect = egui::Rect::from_min_size(
        panel_rect.min + egui::vec2(0.0, PANEL_HEADER_H),
        egui::vec2(size.x, body_h),
    );
    painter.rect_filled(body_rect, 0.0, PANEL_BG_COLOR);

    // Compute cell size to fill the body exactly: SLOT_COLS wide, rows tall.
    let slot_rows = Inventory::MAX_ITEMS.div_ceil(SLOT_COLS);
    let cell_w = (body_rect.width() / SLOT_COLS as f32).floor();
    let cell_h = (body_rect.height() / slot_rows as f32).floor();
    let cell_size = cell_w.min(cell_h).max(8.0);

    // Draw grid lines
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

    // Draw items and selection highlight in each cell.
    for slot_idx in 0..Inventory::MAX_ITEMS {
        let col = slot_idx % SLOT_COLS;
        let row = slot_idx / SLOT_COLS;
        let cell_min = body_rect.min + egui::vec2(col as f32 * cell_w, row as f32 * cell_h);
        let cell_rect = egui::Rect::from_min_size(cell_min, egui::vec2(cell_w, cell_h));

        // Selection highlight — yellow ring on the selected slot
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
            paint_item_silhouette(
                painter,
                cell_rect,
                cell_size,
                item_type,
                ITEM_SILHOUETTE_COLOR,
            );
        }
    }

    // ── Action strip (egui widgets, below the painted body) ───────────────
    if has_action {
        if let Some(slot_idx) = selected_slot {
            let action_rect = egui::Rect::from_min_size(
                panel_rect.min + egui::vec2(0.0, PANEL_HEADER_H + body_h),
                egui::vec2(size.x, action_reserve),
            );
            painter.rect_filled(action_rect, 0.0, HEADER_BG_COLOR);

            // Place an egui child UI inside the action strip for buttons.
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(action_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            child.add_space(6.0);

            // push_id for the action row — mandatory per sdk/AGENTS.md
            child.push_id("actions", |ui| {
                ui.horizontal_wrapped(|ui| {
                    // ── Action 0: Drop ────────────────────────────────────
                    let drop_focused = focused_action_index == Some(0);
                    let drop_label = egui::RichText::new("Drop")
                        .color(if drop_focused {
                            ACTION_FOCUSED_COLOR
                        } else {
                            egui::Color32::from_rgb(220, 80, 80)
                        })
                        .small();
                    let mut drop_btn = egui::Button::new(drop_label);
                    if drop_focused {
                        drop_btn = drop_btn.stroke(egui::Stroke::new(2.0, ACTION_FOCUSED_COLOR));
                    }
                    if ui
                        .add(drop_btn)
                        .on_hover_text("Discard this item permanently")
                        .clicked()
                    {
                        panel_action = Some(PanelAction::Drop {
                            party_index,
                            slot_index: slot_idx,
                        });
                    }

                    // ── Actions 1..N: Transfer to other party members ─────
                    let mut action_btn_idx: usize = 1;
                    for &(other_index, ref other_name) in panel_names {
                        if other_index == party_index {
                            continue;
                        }
                        let target_full = global_state.0.party.members[other_index]
                            .inventory
                            .is_full();
                        let transfer_focused = focused_action_index == Some(action_btn_idx);
                        let label_text = format!("→ {}", other_name);
                        let transfer_label = egui::RichText::new(&label_text)
                            .color(if transfer_focused {
                                ACTION_FOCUSED_COLOR
                            } else {
                                egui::Color32::from_rgb(100, 200, 100)
                            })
                            .small();
                        let mut transfer_btn = egui::Button::new(transfer_label);
                        if transfer_focused {
                            transfer_btn =
                                transfer_btn.stroke(egui::Stroke::new(2.0, ACTION_FOCUSED_COLOR));
                        }
                        if ui
                            .add_enabled(!target_full, transfer_btn)
                            .on_hover_text(if target_full {
                                format!("{}'s inventory is full", other_name)
                            } else {
                                format!("Give to {}", other_name)
                            })
                            .clicked()
                        {
                            panel_action = Some(PanelAction::Transfer {
                                from_party_index: party_index,
                                from_slot_index: slot_idx,
                                to_party_index: other_index,
                            });
                        }
                        action_btn_idx += 1;
                    }
                });
            });
        }
    }

    panel_action
}

/// Public wrapper around [`paint_item_silhouette`] for use by sibling UI
/// modules (`merchant_inventory_ui`, `container_inventory_ui`).
///
/// # Arguments
///
/// * `painter`   – The egui painter for the current frame.
/// * `cell_rect` – Bounding rectangle of the slot cell.
/// * `cell_size` – The size of the cell in pixels (used for scaling).
/// * `item_type` – Optional item type to paint a silhouette for.
/// * `color`     – Tint colour for the silhouette.
pub fn paint_item_silhouette_pub(
    painter: &egui::Painter,
    cell_rect: egui::Rect,
    cell_size: f32,
    item_type: Option<&ItemType>,
    color: egui::Color32,
) {
    paint_item_silhouette(painter, cell_rect, cell_size, item_type, color);
}

/// Paints an item-type silhouette inside a slot cell using the egui `Painter`.
///
/// Each `ItemType` variant maps to a distinct geometric shape so the player
/// can tell items apart at a glance without reading text labels.
///
/// | Type        | Shape                                          |
/// |-------------|------------------------------------------------|
/// | Weapon      | Thin cross (blade + crossguard)               |
/// | Armor       | Rounded rectangle (breastplate outline)       |
/// | Accessory   | Small circle (ring / amulet)                  |
/// | Consumable  | Rounded tall rect (potion flask)              |
/// | Ammo        | Small diamond                                  |
/// | Quest       | Star-like octagon                             |
/// | Unknown     | Simple question-mark placeholder rect         |
fn paint_item_silhouette(
    painter: &egui::Painter,
    cell_rect: egui::Rect,
    cell_size: f32,
    item_type: Option<&ItemType>,
    color: egui::Color32,
) {
    let c = cell_rect.center();
    let s = cell_size * 0.5; // half-cell as scale reference

    match item_type {
        Some(ItemType::Weapon(_)) => {
            // Blade: tall thin rect
            let blade_w = (s * 0.15).max(2.0);
            let blade_h = s * 0.80;
            painter.rect_filled(
                egui::Rect::from_center_size(
                    c + egui::vec2(0.0, -s * 0.10),
                    egui::vec2(blade_w, blade_h),
                ),
                1.0,
                color,
            );
            // Crossguard: wide thin rect
            let guard_w = s * 0.55;
            let guard_h = (s * 0.12).max(2.0);
            painter.rect_filled(
                egui::Rect::from_center_size(
                    c + egui::vec2(0.0, s * 0.25),
                    egui::vec2(guard_w, guard_h),
                ),
                1.0,
                color,
            );
            // Pommel: small square at bottom
            let pommel = (s * 0.18).max(2.0);
            painter.rect_filled(
                egui::Rect::from_center_size(
                    c + egui::vec2(0.0, s * 0.58),
                    egui::vec2(pommel, pommel),
                ),
                1.0,
                color,
            );
        }
        Some(ItemType::Armor(_)) => {
            // Breastplate outline: tall rounded rect
            let w = s * 0.65;
            let h = s * 0.80;
            painter.rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(w, h)),
                4.0,
                egui::Stroke::new((s * 0.12).max(2.0), color),
                egui::StrokeKind::Outside,
            );
            // Shoulder nubs
            let nub_size = s * 0.20;
            painter.rect_filled(
                egui::Rect::from_center_size(
                    c + egui::vec2(-w * 0.5 - nub_size * 0.3, -h * 0.35),
                    egui::vec2(nub_size, nub_size * 0.7),
                ),
                2.0,
                color,
            );
            painter.rect_filled(
                egui::Rect::from_center_size(
                    c + egui::vec2(w * 0.5 + nub_size * 0.3, -h * 0.35),
                    egui::vec2(nub_size, nub_size * 0.7),
                ),
                2.0,
                color,
            );
        }
        Some(ItemType::Accessory(_)) => {
            // Ring: circle outline
            let r = s * 0.35;
            let stroke_w = (s * 0.13).max(2.0);
            painter.circle_stroke(c, r, egui::Stroke::new(stroke_w, color));
            // Small gem on top
            let gem = s * 0.14;
            painter.circle_filled(c + egui::vec2(0.0, -r), gem, color);
        }
        Some(ItemType::Consumable(_)) => {
            // Potion flask: rounded tall rectangle (body)
            let flask_w = s * 0.38;
            let flask_h = s * 0.55;
            painter.rect_filled(
                egui::Rect::from_center_size(
                    c + egui::vec2(0.0, s * 0.10),
                    egui::vec2(flask_w, flask_h),
                ),
                3.0,
                color,
            );
            // Neck
            let neck_w = flask_w * 0.45;
            let neck_h = s * 0.22;
            painter.rect_filled(
                egui::Rect::from_center_size(
                    c + egui::vec2(0.0, -s * 0.25),
                    egui::vec2(neck_w, neck_h),
                ),
                1.0,
                color,
            );
            // Cork
            let cork_w = neck_w * 1.3;
            let cork_h = s * 0.10;
            painter.rect_filled(
                egui::Rect::from_center_size(
                    c + egui::vec2(0.0, -s * 0.40),
                    egui::vec2(cork_w, cork_h),
                ),
                1.0,
                color,
            );
        }
        Some(ItemType::Ammo(_)) => {
            // Arrow shaft
            let shaft_w = (s * 0.10).max(2.0);
            let shaft_h = s * 0.72;
            painter.rect_filled(
                egui::Rect::from_center_size(c, egui::vec2(shaft_w, shaft_h)),
                0.0,
                color,
            );
            // Arrowhead: small triangle approximated by a rotated rect
            let head = s * 0.20;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    c + egui::vec2(0.0, -shaft_h * 0.5 - head),
                    c + egui::vec2(-head * 0.55, -shaft_h * 0.5 + head * 0.2),
                    c + egui::vec2(head * 0.55, -shaft_h * 0.5 + head * 0.2),
                ],
                color,
                egui::Stroke::NONE,
            ));
        }
        Some(ItemType::Quest(_)) => {
            // Quest item: bordered square with a small circle inside
            let sq = s * 0.55;
            painter.rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(sq, sq)),
                2.0,
                egui::Stroke::new((s * 0.10).max(2.0), color),
                egui::StrokeKind::Outside,
            );
            painter.circle_filled(c, s * 0.18, color);
        }
        _ => {
            // Unknown / fallback: small filled square
            let sq = s * 0.35;
            painter.rect_filled(
                egui::Rect::from_center_size(c, egui::vec2(sq, sq)),
                2.0,
                egui::Color32::from_rgba_premultiplied(120, 120, 120, 180),
            );
        }
    }
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
    mut nav_state: ResMut<InventoryNavigationState>,
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

        // Clear selected_slot in InventoryState and reset nav phase
        if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
            inv_state.selected_slot = None;
        }
        nav_state.selected_slot_index = None;
        nav_state.focused_action_index = 0;
        nav_state.phase = NavigationPhase::SlotNavigation;
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
                // Clear selected_slot on success and reset nav phase
                if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
                    inv_state.selected_slot = None;
                }
                nav_state.selected_slot_index = None;
                nav_state.focused_action_index = 0;
                nav_state.phase = NavigationPhase::SlotNavigation;
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
        assert_eq!(state.focused_action_index, 0);
        assert!(matches!(state.phase, NavigationPhase::SlotNavigation));
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
                    0,                        // party_index
                    true,                     // is_focused
                    None,                     // selected_slot
                    None,                     // focused_action_index
                    egui::vec2(300.0, 400.0), // size
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
                    0,                        // party_index
                    false,                    // not focused
                    Some(0),                  // first slot selected
                    None,                     // no keyboard action focus
                    egui::vec2(300.0, 400.0), // size
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
                    None,
                    egui::vec2(300.0, 400.0), // size
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
            focused_action_index: 2,
            phase: NavigationPhase::ActionNavigation,
        };
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("3"));
        assert!(debug_str.contains("1"));
        assert!(debug_str.contains("ActionNavigation"));
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
        app.init_resource::<InventoryNavigationState>();

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
        // nav state must be reset to slot navigation
        let nav = app.world().resource::<InventoryNavigationState>();
        assert!(matches!(nav.phase, NavigationPhase::SlotNavigation));
        assert_eq!(nav.selected_slot_index, None);
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
        app.init_resource::<InventoryNavigationState>();

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
        // nav state reset
        let nav = app.world().resource::<InventoryNavigationState>();
        assert!(matches!(nav.phase, NavigationPhase::SlotNavigation));
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
        app.init_resource::<InventoryNavigationState>();
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
        app.init_resource::<InventoryNavigationState>();
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
        app.init_resource::<InventoryNavigationState>();
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
        app.init_resource::<InventoryNavigationState>();
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
        app.init_resource::<InventoryNavigationState>();
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
        app.init_resource::<InventoryNavigationState>();
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
                    0,                        // party_index
                    true,                     // is_focused
                    Some(0),                  // slot 0 selected (has item)
                    None,                     // no keyboard action focus
                    egui::vec2(300.0, 400.0), // size
                    &global_state,
                    None,
                    &panel_names,
                );
                assert!(action.is_none(), "no button clicked, should return None");
            });
        });
    }

    // ------------------------------------------------------------------
    // Extra: render_character_panel with keyboard action focus renders without panic
    // ------------------------------------------------------------------

    /// When `focused_action_index` is `Some(0)` the Drop button should be
    /// highlighted. No click is simulated so the return value is still `None`.
    #[test]
    fn test_render_character_panel_action_focus_drop_no_panic() {
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
            item_id: 5 as ItemId,
            charges: 1,
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
                let action = render_character_panel(
                    ui,
                    0,
                    true,
                    Some(0), // slot 0 is selected
                    Some(0), // keyboard focus on Drop button (index 0)
                    egui::vec2(300.0, 400.0),
                    &global_state,
                    None,
                    &panel_names,
                );
                assert!(action.is_none());
            });
        });
    }

    // ------------------------------------------------------------------
    // Extra: render_character_panel with keyboard action focus on Transfer
    // ------------------------------------------------------------------

    /// When `focused_action_index` is `Some(1)` the first Transfer button
    /// should be highlighted. No click simulated.
    #[test]
    fn test_render_character_panel_action_focus_transfer_no_panic() {
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
            item_id: 7 as ItemId,
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
                let action = render_character_panel(
                    ui,
                    0,
                    true,
                    Some(0), // slot 0 selected
                    Some(1), // keyboard focus on Transfer→Ally (index 1)
                    egui::vec2(300.0, 400.0),
                    &global_state,
                    None,
                    &panel_names,
                );
                assert!(action.is_none());
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

    // ------------------------------------------------------------------
    // Two-phase navigation: NavigationPhase enum
    // ------------------------------------------------------------------

    /// `NavigationPhase` must default to `SlotNavigation`.
    #[test]
    fn test_navigation_phase_default_is_slot_navigation() {
        let phase = NavigationPhase::default();
        assert!(matches!(phase, NavigationPhase::SlotNavigation));
    }

    /// `NavigationPhase` variants must be equal to themselves.
    #[test]
    fn test_navigation_phase_equality() {
        assert_eq!(
            NavigationPhase::SlotNavigation,
            NavigationPhase::SlotNavigation
        );
        assert_eq!(
            NavigationPhase::ActionNavigation,
            NavigationPhase::ActionNavigation
        );
        assert_ne!(
            NavigationPhase::SlotNavigation,
            NavigationPhase::ActionNavigation
        );
    }

    // ------------------------------------------------------------------
    // build_action_list helper
    // ------------------------------------------------------------------

    /// `build_action_list` with no other panels returns exactly one action: Drop.
    #[test]
    fn test_build_action_list_drop_only() {
        let panel_names: Vec<(usize, String)> = vec![(0, "Hero".to_string())];
        let actions = build_action_list(0, &panel_names);
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            actions[0],
            PanelAction::Drop { party_index: 0, .. }
        ));
    }

    /// `build_action_list` with two other panels returns Drop + two Transfer actions.
    #[test]
    fn test_build_action_list_drop_and_transfers() {
        let panel_names: Vec<(usize, String)> = vec![
            (0, "Hero".to_string()),
            (1, "Ally".to_string()),
            (2, "Mage".to_string()),
        ];
        let actions = build_action_list(0, &panel_names);
        // Drop + Transfer→1 + Transfer→2
        assert_eq!(actions.len(), 3);
        assert!(matches!(
            actions[0],
            PanelAction::Drop { party_index: 0, .. }
        ));
        assert!(matches!(
            actions[1],
            PanelAction::Transfer {
                from_party_index: 0,
                to_party_index: 1,
                ..
            }
        ));
        assert!(matches!(
            actions[2],
            PanelAction::Transfer {
                from_party_index: 0,
                to_party_index: 2,
                ..
            }
        ));
    }

    /// `build_action_list` excludes the focused panel itself from Transfer targets.
    #[test]
    fn test_build_action_list_excludes_self() {
        let panel_names: Vec<(usize, String)> = vec![(0, "A".to_string()), (1, "B".to_string())];
        let actions = build_action_list(1, &panel_names);
        // Drop(1) + Transfer(1→0)
        assert_eq!(actions.len(), 2);
        assert!(matches!(
            actions[0],
            PanelAction::Drop { party_index: 1, .. }
        ));
        assert!(matches!(
            actions[1],
            PanelAction::Transfer {
                from_party_index: 1,
                to_party_index: 0,
                ..
            }
        ));
    }

    // ------------------------------------------------------------------
    // nav state reset helper
    // ------------------------------------------------------------------

    /// `InventoryNavigationState::reset` returns the struct to its default values.
    #[test]
    fn test_inventory_navigation_state_reset() {
        let mut state = InventoryNavigationState {
            selected_slot_index: Some(5),
            focus_on_panel: 2,
            focused_action_index: 1,
            phase: NavigationPhase::ActionNavigation,
        };
        state.reset();
        assert_eq!(state.selected_slot_index, None);
        assert_eq!(state.focus_on_panel, 0);
        assert_eq!(state.focused_action_index, 0);
        assert!(matches!(state.phase, NavigationPhase::SlotNavigation));
    }

    // ------------------------------------------------------------------
    // action system resets nav phase after drop
    // ------------------------------------------------------------------

    /// After a drop action the nav state phase must return to `SlotNavigation`
    /// even if it was previously in `ActionNavigation`.
    #[test]
    fn test_action_system_drop_resets_nav_phase_to_slot() {
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
            item_id: 11 as ItemId,
            charges: 0,
        });
        game_state.party.add_member(character).unwrap();
        game_state.enter_inventory();

        // Pre-load nav state into ActionNavigation
        let nav = InventoryNavigationState {
            phase: NavigationPhase::ActionNavigation,
            selected_slot_index: Some(0),
            focused_action_index: 0,
            ..Default::default()
        };

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(nav);
        app.add_systems(Update, inventory_action_system);

        app.world_mut().write_message(DropItemAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();

        let nav_after = app.world().resource::<InventoryNavigationState>();
        assert!(
            matches!(nav_after.phase, NavigationPhase::SlotNavigation),
            "phase must be SlotNavigation after drop"
        );
        assert_eq!(nav_after.focused_action_index, 0);
        assert_eq!(nav_after.selected_slot_index, None);
    }

    // ------------------------------------------------------------------
    // action system resets nav phase after transfer
    // ------------------------------------------------------------------

    /// After a successful transfer the nav state phase must return to
    /// `SlotNavigation`.
    #[test]
    fn test_action_system_transfer_resets_nav_phase_to_slot() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::types::ItemId;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<DropItemAction>();
        app.add_message::<TransferItemAction>();

        let mut game_state = GameState::new();

        let mut src = Character::new(
            "Src".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        src.inventory.items.push(InventorySlot {
            item_id: 99 as ItemId,
            charges: 0,
        });
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

        let nav = InventoryNavigationState {
            phase: NavigationPhase::ActionNavigation,
            selected_slot_index: Some(0),
            focused_action_index: 1,
            ..Default::default()
        };

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(nav);
        app.add_systems(Update, inventory_action_system);

        app.world_mut().write_message(TransferItemAction {
            from_party_index: 0,
            from_slot_index: 0,
            to_party_index: 1,
        });
        app.update();

        let nav_after = app.world().resource::<InventoryNavigationState>();
        assert!(
            matches!(nav_after.phase, NavigationPhase::SlotNavigation),
            "phase must be SlotNavigation after successful transfer"
        );
        assert_eq!(nav_after.focused_action_index, 0);
        assert_eq!(nav_after.selected_slot_index, None);
    }
}
