// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Inventory UI System - Character inventory management interface
//!
//! Provides an egui-based overlay for viewing and managing character inventory
//! when the game is in `GameMode::Inventory` mode. This system is active when
//! the player presses the configured inventory key (default: `I`).
//!
//! ## Keyboard Navigation (two-phase model)
//!
//! ### Slot Navigation
//!
//! | Key              | Effect                                                                        |
//! |------------------|-------------------------------------------------------------------------------|
//! | `Tab`            | Advance focus to the next character panel (yellow border)                     |
//! | `Shift+Tab`      | Move focus to the previous character panel                                    |
//! | `←` `→` `↑` `↓` | Navigate the slot grid inside the focused panel                               |
//! | `Enter`          | Enter **Action Navigation** for the highlighted slot                          |
//! | `U`              | Use the highlighted consumable directly (bypasses Action Navigation)          |
//! | `Esc` / `I`      | Close the inventory and resume the previous game mode                         |
//!
//! ### Action Navigation
//!
//! | Key         | Effect                                                             |
//! |-------------|--------------------------------------------------------------------|
//! | `←` `→`     | Cycle between action buttons (Use / Drop / Give→ …)                |
//! | `Enter`      | Execute the focused action; return focus to slot 0 of the grid     |
//! | `Esc`        | Cancel; return to Slot Navigation at the previously selected slot   |
//!
//! Follows the `InnUiPlugin` pattern from `src/game/systems/inn_ui.rs` exactly.

use crate::application::resources::GameContent;
use crate::application::GameMode;
use crate::domain::character::{EquipmentSlot, Inventory, PARTY_MAX_SIZE};
use crate::domain::combat::item_usage::{validate_item_use_slot, ItemUseError};
use crate::domain::items::consumable_usage::{
    apply_consumable_effect_exploration, ConsumableApplyResult,
};
use crate::domain::items::equipment_validation::EquipError;
use crate::domain::items::types::{normalize_duration, ConsumableData, ConsumableEffect, ItemType};
use crate::domain::magic::exploration_casting::{cast_exploration_spell, ExplorationTarget};
use crate::domain::magic::learning::{learn_spell, SpellLearnError};
use crate::domain::transactions::{drop_item, equip_item, unequip_item, TransactionError};
use crate::game::resources::GlobalState;
use crate::game::systems::item_world_events::ItemDroppedEvent;
use crate::game::systems::ui::{GameLogEvent, LogCategory};

use bevy::prelude::MessageWriter;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use super::inventory_ui_common::{
    ACTION_FOCUSED_COLOR, FOCUSED_BORDER_COLOR, GRID_LINE_COLOR, HEADER_BG_COLOR, PANEL_ACTION_H,
    PANEL_BG_COLOR, PANEL_HEADER_H, SELECT_HIGHLIGHT_COLOR, SLOT_COLS, UNFOCUSED_BORDER_COLOR,
};
// Re-export `NavigationPhase` so that existing `use
// antares::game::systems::inventory_ui::NavigationPhase` paths keep working.
pub use super::inventory_ui_common::NavigationPhase;

// ===== Layout constants (file-local) =====

/// Height of the equipment display strip shown between the header and slot grid.
/// Two rows of cells (weapon/armor/shield, then helmet/boots/ring/ring).
const EQUIP_STRIP_H: f32 = 76.0;
/// Colour for item silhouettes.
const ITEM_SILHOUETTE_COLOR: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(230, 230, 230, 255);

/// Plugin for inventory management UI
pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DropItemAction>()
            .add_message::<TransferItemAction>()
            .add_message::<UseItemExplorationAction>()
            .add_message::<EquipItemAction>()
            .add_message::<UnequipItemAction>()
            .init_resource::<InventoryNavigationState>()
            .add_systems(
                Update,
                (
                    inventory_input_system,
                    inventory_ui_system,
                    inventory_action_system,
                    handle_use_item_action_exploration,
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

/// Emitted when the player uses a consumable item outside of combat
/// (i.e. while in [`GameMode::Inventory`]).
///
/// ## Self-target contract
///
/// The effect is **always applied to the owning character** — the party member
/// identified by `party_index`.  Cross-party targeting (e.g. healing a
/// different party member from another character's inventory panel) is
/// explicitly out of scope and belongs to a future targeting phase.
///
/// ## Valid ranges
///
/// * `party_index` — `0..party.members.len()`.  Values outside this range
///   cause [`handle_use_item_action_exploration`] to write a `GameLog` error
///   entry and skip the message without panicking.
/// * `slot_index` — `0..character.inventory.items.len()`.  Values outside
///   this range are caught by [`validate_item_use_slot`] and produce a
///   `GameLog` entry containing "no item in that slot".
///
/// ## Charge semantics
///
/// One charge is consumed per use:
/// * `charges > 1` → decremented in place.
/// * `charges == 1` → the slot is removed from the inventory entirely.
/// * `charges == 0` → rejected before any mutation; "no charges" is logged.
///
/// # Examples
///
/// ```
/// use antares::game::systems::inventory_ui::UseItemExplorationAction;
///
/// let action = UseItemExplorationAction { party_index: 1, slot_index: 2 };
/// assert_eq!(action.party_index, 1);
/// assert_eq!(action.slot_index, 2);
/// ```
#[derive(Message)]
pub struct UseItemExplorationAction {
    /// Index of the party member (0-based) whose inventory contains the item.
    /// Valid range: `0..party.members.len()`.
    pub party_index: usize,
    /// Index of the slot within that character's inventory to use.
    /// Valid range: `0..character.inventory.items.len()`.
    pub slot_index: usize,
}

// ===== Equip / Unequip Message Types =====

/// Emitted when the player requests equipping an inventory item into an equipment slot.
///
/// Dispatched by pressing **E** while an equipable slot is highlighted in
/// `SlotNavigation` phase, or by clicking the **Equip** button in the action
/// strip.
///
/// # Examples
///
/// ```
/// use antares::game::systems::inventory_ui::EquipItemAction;
///
/// let action = EquipItemAction { party_index: 0, slot_index: 2 };
/// assert_eq!(action.party_index, 0);
/// assert_eq!(action.slot_index, 2);
/// ```
#[derive(Message)]
pub struct EquipItemAction {
    /// Party member index (0-based) whose inventory contains the item.
    pub party_index: usize,
    /// Slot index in that character's inventory to equip.
    pub slot_index: usize,
}

/// Emitted when the player requests unequipping an item from an equipment slot
/// back into inventory.
///
/// Dispatched by pressing **Enter** on a focused equipment strip cell, or by
/// clicking the **Unequip** button shown below the strip.
///
/// # Examples
///
/// ```
/// use antares::game::systems::inventory_ui::UnequipItemAction;
/// use antares::domain::character::EquipmentSlot;
///
/// let action = UnequipItemAction { party_index: 1, slot: EquipmentSlot::Weapon };
/// assert_eq!(action.party_index, 1);
/// assert!(matches!(action.slot, EquipmentSlot::Weapon));
/// ```
#[derive(Message)]
pub struct UnequipItemAction {
    /// Party member index (0-based) wearing the item.
    pub party_index: usize,
    /// Which equipment slot to clear.
    pub slot: EquipmentSlot,
}

// ===== Panel Action =====

/// Represents an action that the player has requested via the inventory UI.
///
/// `render_character_panel` returns a [`CharacterPanelResult`] instead of
/// writing messages directly so that the render helper stays free of
/// `MessageWriter` generics and can also report mouse-click slot selection
/// back to `inventory_ui_system`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::inventory_ui::PanelAction;
///
/// let use_action = PanelAction::Use { party_index: 0, slot_index: 3 };
/// let drop = PanelAction::Drop { party_index: 0, slot_index: 1 };
/// let transfer = PanelAction::Transfer {
///     from_party_index: 0,
///     from_slot_index: 0,
///     to_party_index: 1,
/// };
/// match use_action {
///     PanelAction::Use { party_index, slot_index } => {
///         assert_eq!(party_index, 0);
///         assert_eq!(slot_index, 3);
///     }
///     _ => panic!("unexpected"),
/// }
/// match drop {
///     PanelAction::Drop { party_index, slot_index } => {
///         assert_eq!(party_index, 0);
///         assert_eq!(slot_index, 1);
///     }
///     _ => panic!("unexpected"),
/// }
/// match transfer {
///     PanelAction::Transfer { from_party_index, from_slot_index, to_party_index } => {
///         assert_eq!(from_party_index, 0);
///         assert_eq!(from_slot_index, 0);
///         assert_eq!(to_party_index, 1);
///     }
///     _ => panic!("unexpected"),
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub enum PanelAction {
    /// Use the consumable at `slot_index` owned by `party_index`.
    Use {
        /// Party member index of the owner.
        party_index: usize,
        /// Inventory slot index to use.
        slot_index: usize,
    },
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
    /// Equip the item at `slot_index` from `party_index`'s inventory.
    Equip {
        /// Party member index of the owner.
        party_index: usize,
        /// Inventory slot index to equip.
        slot_index: usize,
    },
    /// Unequip the item in `slot` from `party_index`'s equipment back to inventory.
    Unequip {
        /// Party member index of the owner.
        party_index: usize,
        /// The equipment slot to clear.
        slot: EquipmentSlot,
    },
}

// `NavigationPhase` is re-exported from `inventory_ui_common` at the top of
// this file so that `use antares::game::systems::inventory_ui::NavigationPhase`
// continues to resolve.

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
    /// Which equipment strip cell is focused (`None` = strip not focused).
    ///
    /// Set when the player navigates Up from slot row 0, or clicks an equipment
    /// cell.  Cleared when navigating Down back to the inventory grid or when
    /// an Unequip action is dispatched.
    pub selected_equip_slot: Option<EquipmentSlot>,
}

impl InventoryNavigationState {
    /// Reset to a clean default state.
    fn reset(&mut self) {
        *self = InventoryNavigationState::default();
    }
}

/// Combined result returned from `render_character_panel`.
///
/// This lets the egui renderer report both a clicked slot selection and an
/// optional action-button click back to `inventory_ui_system`.
#[derive(Debug, PartialEq, Eq)]
pub struct CharacterPanelResult {
    /// Action button click, if any.
    pub action: Option<PanelAction>,
    /// Inventory slot index clicked in the painter-driven grid, if any.
    pub clicked_slot: Option<usize>,
}

// ===== Helpers =====

/// Build the ordered list of action button descriptors for a focused panel.
///
/// Returns a `Vec<PanelAction>` in the same order the UI renders them:
/// `Equip` first (only for equipable items), then `Use` (only for consumables),
/// then `Drop`, then one `Transfer` per other open panel member.
///
/// Button order:
/// | Index | Label  | Condition               |
/// |-------|--------|-------------------------|
/// | 0     | Equip  | Item is equipable        |
/// | 0/1   | Use    | Item is `Consumable`     |
/// | 1/2   | Drop   | Always present           |
/// | 2+    | → Name | One per other open panel |
///
/// `panel_names` contains `(party_index, name)` for every visible panel.
/// `focused_party_index` is the panel whose actions are being computed.
/// `selected_slot_index` is the inventory slot currently highlighted.
/// `character` is the focused party member, used to inspect the item type.
/// `game_content` is used to look up the item definition; if `None`, no `Equip`
/// or `Use` action is generated.
fn build_action_list(
    focused_party_index: usize,
    selected_slot_index: usize,
    panel_names: &[(usize, String)],
    character: &crate::domain::character::Character,
    game_content: Option<&crate::application::resources::GameContent>,
) -> Vec<PanelAction> {
    let mut actions = Vec::new();

    // Look up the item definition once for both equipable and consumable checks.
    let item_opt = character
        .inventory
        .items
        .get(selected_slot_index)
        .and_then(|slot| game_content.and_then(|gc| gc.db().items.get_item(slot.item_id)));

    // Prepend Equip action if the item can be placed in an equipment slot.
    // `EquipmentSlot::for_item` returns `Some(_)` for weapons, armour, and accessories.
    let is_equipable = item_opt
        .and_then(|item| EquipmentSlot::for_item(item, &character.equipment))
        .is_some();

    if is_equipable {
        actions.push(PanelAction::Equip {
            party_index: focused_party_index,
            slot_index: 0, // placeholder — filled at execution time
        });
    }

    // Prepend Use action if the focused slot contains a consumable item
    let is_consumable = item_opt
        .map(|item| matches!(item.item_type, ItemType::Consumable(_)))
        .unwrap_or(false);

    if is_consumable {
        actions.push(PanelAction::Use {
            party_index: focused_party_index,
            slot_index: 0, // placeholder — filled at execution time
        });
    }

    // Drop is always present
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

// ===== Input System Helpers =====

/// Handles keyboard input during the `ActionNavigation` phase.
///
/// Processes Escape (cancel), Left/Right (cycle actions), and Enter (execute
/// action). All paths within this function handle one key event and return.
#[allow(clippy::too_many_arguments)]
fn handle_action_selection(
    keyboard: &ButtonInput<KeyCode>,
    nav_state: &mut InventoryNavigationState,
    global_state: &mut GlobalState,
    focused_party_index: usize,
    panel_names: &[(usize, String)],
    game_content: Option<&GameContent>,
    drop_writer: &mut MessageWriter<DropItemAction>,
    transfer_writer: &mut MessageWriter<TransferItemAction>,
    use_writer: &mut MessageWriter<UseItemExplorationAction>,
    equip_writer: &mut MessageWriter<EquipItemAction>,
    unequip_writer: &mut MessageWriter<UnequipItemAction>,
) {
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
    let focused_char_opt = global_state.0.party.members.get(focused_party_index);
    let actions = match focused_char_opt {
        Some(ch) => build_action_list(focused_party_index, slot_idx, panel_names, ch, game_content),
        None => {
            nav_state.phase = NavigationPhase::SlotNavigation;
            return;
        }
    };
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
            PanelAction::Use { party_index, .. } => {
                use_writer.write(UseItemExplorationAction {
                    party_index: *party_index,
                    slot_index: slot_idx,
                });
            }
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
            PanelAction::Equip { party_index, .. } => {
                equip_writer.write(EquipItemAction {
                    party_index: *party_index,
                    slot_index: slot_idx,
                });
            }
            PanelAction::Unequip { party_index, slot } => {
                unequip_writer.write(UnequipItemAction {
                    party_index: *party_index,
                    slot: *slot,
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
    }

    // Any other key in action mode is ignored
}

/// Handles equipment-strip keyboard navigation and the `E`-key equip shortcut.
///
/// Returns `true` if input was consumed (the caller should `return` early).
fn handle_equip_flow(
    keyboard: &ButtonInput<KeyCode>,
    nav_state: &mut InventoryNavigationState,
    global_state: &mut GlobalState,
    focused_party_index: usize,
    game_content: Option<&GameContent>,
    equip_writer: &mut MessageWriter<EquipItemAction>,
    unequip_writer: &mut MessageWriter<UnequipItemAction>,
) -> bool {
    const EQUIP_SLOTS: [EquipmentSlot; 7] = [
        EquipmentSlot::Weapon,
        EquipmentSlot::Armor,
        EquipmentSlot::Shield,
        EquipmentSlot::Helmet,
        EquipmentSlot::Boots,
        EquipmentSlot::Accessory1,
        EquipmentSlot::Accessory2,
    ];

    if let Some(equip_slot) = nav_state.selected_equip_slot {
        // Esc — exit equipment strip, back to slot navigation
        if keyboard.just_pressed(KeyCode::Escape) {
            nav_state.selected_equip_slot = None;
            return true;
        }

        // ArrowDown — return to inventory grid row 0
        if keyboard.just_pressed(KeyCode::ArrowDown) {
            nav_state.selected_equip_slot = None;
            if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
                inv_state.selected_slot = Some(0);
            }
            nav_state.selected_slot_index = Some(0);
            return true;
        }

        // ArrowLeft / ArrowRight — cycle through equipment strip cells
        if keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::ArrowRight) {
            if let Some(idx) = EQUIP_SLOTS.iter().position(|&s| s == equip_slot) {
                let next = if keyboard.just_pressed(KeyCode::ArrowLeft) {
                    if idx == 0 {
                        EQUIP_SLOTS.len() - 1
                    } else {
                        idx - 1
                    }
                } else {
                    (idx + 1) % EQUIP_SLOTS.len()
                };
                nav_state.selected_equip_slot = Some(EQUIP_SLOTS[next]);
            }
            return true;
        }

        // Enter — dispatch UnequipItemAction for the focused equipment slot
        if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter) {
            unequip_writer.write(UnequipItemAction {
                party_index: focused_party_index,
                slot: equip_slot,
            });
            nav_state.selected_equip_slot = None;
            return true;
        }

        // Any other key while strip is focused is ignored
        return true;
    }

    // E key — equip the highlighted item directly (bypasses ActionNavigation)
    if keyboard.just_pressed(KeyCode::KeyE) {
        if let Some(slot_idx) = nav_state.selected_slot_index {
            let is_equipable = global_state
                .0
                .party
                .members
                .get(focused_party_index)
                .and_then(|ch| {
                    ch.inventory.items.get(slot_idx).and_then(|slot| {
                        game_content
                            .and_then(|gc| gc.db().items.get_item(slot.item_id))
                            .and_then(|item| EquipmentSlot::for_item(item, &ch.equipment))
                    })
                })
                .is_some();

            if is_equipable {
                equip_writer.write(EquipItemAction {
                    party_index: focused_party_index,
                    slot_index: slot_idx,
                });
                if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
                    inv_state.selected_slot = None;
                }
                nav_state.selected_slot_index = None;
                nav_state.focused_action_index = 0;
                nav_state.phase = NavigationPhase::SlotNavigation;
            }
        }
        return true;
    }

    false
}

/// Handles `U`-key (use consumable), `Enter` (confirm slot), and arrow-key
/// grid navigation during the `SlotNavigation` phase.
fn handle_grid_navigation(
    keyboard: &ButtonInput<KeyCode>,
    nav_state: &mut InventoryNavigationState,
    global_state: &mut GlobalState,
    focused_party_index: usize,
    game_content: Option<&GameContent>,
    use_writer: &mut MessageWriter<UseItemExplorationAction>,
) {
    // U key — use consumable in the highlighted slot directly (bypasses ActionNavigation)
    if keyboard.just_pressed(KeyCode::KeyU) {
        if let Some(slot_idx) = nav_state.selected_slot_index {
            let is_consumable = global_state
                .0
                .party
                .members
                .get(focused_party_index)
                .and_then(|ch| ch.inventory.items.get(slot_idx))
                .and_then(|slot| game_content.and_then(|gc| gc.db().items.get_item(slot.item_id)))
                .map(|item| matches!(item.item_type, ItemType::Consumable(_)))
                .unwrap_or(false);

            if is_consumable {
                use_writer.write(UseItemExplorationAction {
                    party_index: focused_party_index,
                    slot_index: slot_idx,
                });
                if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
                    inv_state.selected_slot = None;
                }
                nav_state.selected_slot_index = None;
                nav_state.focused_action_index = 0;
                nav_state.phase = NavigationPhase::SlotNavigation;
            }
        }
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
        // ArrowUp from slot row 0 → move focus to the equipment strip
        if keyboard.just_pressed(KeyCode::ArrowUp) {
            let current = match &global_state.0.mode {
                GameMode::Inventory(s) => s.selected_slot.unwrap_or(0),
                _ => 0,
            };
            if current < SLOT_COLS {
                // Entering the equipment strip — clear grid selection
                if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
                    inv_state.selected_slot = None;
                }
                nav_state.selected_slot_index = None;
                nav_state.selected_equip_slot = Some(EquipmentSlot::Weapon);
                return;
            }
        }

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
                // ArrowUp — move one row up (row > 0 case handled above)
                current.saturating_sub(SLOT_COLS)
            };
            inv_state.selected_slot = Some(next);
            nav_state.selected_slot_index = Some(next);
        }
    }
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
/// | SlotNavigation    | `↑` (row 0)      | Move focus to the equipment strip                   |
/// | SlotNavigation    | `Enter`          | Enter ActionNavigation for the highlighted slot     |
/// | SlotNavigation    | `E`              | Equip the highlighted equipable item directly       |
/// | SlotNavigation    | `U`              | Use the highlighted consumable directly             |
/// | EquipStrip        | `←→`            | Cycle equipment strip cells                         |
/// | EquipStrip        | `↓`             | Return focus to inventory grid row 0                |
/// | EquipStrip        | `Enter`          | Dispatch UnequipItemAction for the focused cell     |
/// | ActionNavigation  | `←→`            | Cycle action buttons                                |
/// | ActionNavigation  | `Enter`          | Execute focused action; return to slot 0            |
#[allow(clippy::too_many_arguments)]
fn inventory_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut global_state: ResMut<GlobalState>,
    mut nav_state: ResMut<InventoryNavigationState>,
    mut drop_writer: MessageWriter<DropItemAction>,
    mut transfer_writer: MessageWriter<TransferItemAction>,
    game_content: Option<Res<GameContent>>,
    mut use_writer: MessageWriter<UseItemExplorationAction>,
    mut equip_writer: MessageWriter<EquipItemAction>,
    mut unequip_writer: MessageWriter<UnequipItemAction>,
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
        handle_action_selection(
            &keyboard,
            &mut nav_state,
            &mut global_state,
            focused_party_index,
            &panel_names,
            game_content.as_deref(),
            &mut drop_writer,
            &mut transfer_writer,
            &mut use_writer,
            &mut equip_writer,
            &mut unequip_writer,
        );
        return;
    }

    // ── Phase: SlotNavigation ──────────────────────────────────────────────

    // NOTE: The configured inventory toggle key ("I" by default) is intentionally
    // NOT handled here. The split input systems owned by `InputPlugin` handle the
    // open/close toggle for that key before inventory UI input runs. Duplicating
    // it here would cause the inventory to open and close in the same frame.

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
        nav_state.selected_equip_slot = None;
        return;
    }

    // ── Equipment strip navigation & E-key equip ──────────────────────────
    if handle_equip_flow(
        &keyboard,
        &mut nav_state,
        &mut global_state,
        focused_party_index,
        game_content.as_deref(),
        &mut equip_writer,
        &mut unequip_writer,
    ) {
        return;
    }

    // ── U-key, Enter, and arrow-key grid navigation ───────────────────────
    handle_grid_navigation(
        &keyboard,
        &mut nav_state,
        &mut global_state,
        focused_party_index,
        game_content.as_deref(),
        &mut use_writer,
    );
}

// ===== UI System Helpers =====

/// Renders the status line (focused character and selected item) and keyboard
/// navigation hint below it.
fn render_equipment_panel(
    ui: &mut egui::Ui,
    global_state: &GlobalState,
    focused_index: usize,
    selected_slot: Option<usize>,
    game_content: Option<&GameContent>,
    nav_state: &InventoryNavigationState,
) {
    // ── Status line: focused character + selected item ───────────────
    {
        let party = &global_state.0.party;
        if focused_index < party.members.len() {
            let character = &party.members[focused_index];
            let status = match selected_slot {
                Some(slot_idx) if slot_idx < character.inventory.items.len() => {
                    let slot = &character.inventory.items[slot_idx];
                    let item_opt = game_content.and_then(|gc| gc.db().items.get_item(slot.item_id));
                    let item_name = item_opt
                        .map(|item| item.name.clone())
                        .unwrap_or_else(|| format!("Item #{}", slot.item_id));
                    let is_consumable = item_opt
                        .map(|item| matches!(item.item_type, ItemType::Consumable(_)))
                        .unwrap_or(false);
                    let use_hint = if is_consumable { "  [U: use]" } else { "" };
                    format!(
                        "Focus: {}  |  Selected: {} (slot {}){}",
                        character.name, item_name, slot_idx, use_hint
                    )
                }
                _ => format!("Focus: {}", character.name),
            };
            ui.label(egui::RichText::new(status).strong());
        }
    }

    // ── Hint line changes based on navigation phase ──────────────────
    let hint = if nav_state.selected_equip_slot.is_some() {
        "←→: cycle equipment slots   ↓: back to inventory   Enter: unequip   Esc: cancel"
    } else {
        match nav_state.phase {
            NavigationPhase::SlotNavigation => {
                "Tab: cycle character   ←→↑↓: navigate slots   Enter: select item   E: equip   U: use   Esc/I: close"
            }
            NavigationPhase::ActionNavigation => {
                "←→: cycle actions   Enter: execute   Esc: cancel"
            }
        }
    };
    ui.label(egui::RichText::new(hint).small().weak());
}

/// Renders the grid of character panels arranged in rows and columns.
///
/// Returns a pending [`PanelAction`] and an optional clicked-slot update
/// `(party_index, slot_idx, has_item)`.
#[allow(clippy::too_many_arguments)]
fn render_item_grid(
    ui: &mut egui::Ui,
    open_panels: &[usize],
    focused_index: usize,
    selected_slot: Option<usize>,
    nav_state: &InventoryNavigationState,
    global_state: &GlobalState,
    game_content: Option<&GameContent>,
    panel_names: &[(usize, String)],
) -> (Option<PanelAction>, Option<(usize, usize, bool)>) {
    let mut pending_action: Option<PanelAction> = None;
    let mut clicked_slot_update: Option<(usize, usize, bool)> = None;

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

                // Only pass selected_equip_slot to the focused panel
                let panel_equip_slot = if is_focused {
                    nav_state.selected_equip_slot
                } else {
                    None
                };

                // push_id mandatory per sdk/AGENTS.md
                ui.push_id(format!("inv_panel_{}", party_index), |ui| {
                    let panel_result = render_character_panel(
                        ui,
                        party_index,
                        is_focused,
                        panel_selected,
                        panel_action_focus,
                        egui::vec2(panel_w, panel_h),
                        global_state,
                        game_content,
                        panel_names,
                        panel_equip_slot,
                    );
                    if let Some(action) = panel_result.action {
                        pending_action = Some(action);
                    }
                    if let Some(slot_idx) = panel_result.clicked_slot {
                        let has_item = global_state
                            .0
                            .party
                            .members
                            .get(party_index)
                            .map(|ch| slot_idx < ch.inventory.items.len())
                            .unwrap_or(false);
                        clicked_slot_update = Some((party_index, slot_idx, has_item));
                    }
                });
            }
        });
        if row + 1 < rows {
            ui.add_space(4.0);
        }
    }

    (pending_action, clicked_slot_update)
}

/// Dispatches a pending [`PanelAction`] into the appropriate message writer.
fn render_action_bar(
    pending_action: Option<PanelAction>,
    use_writer: &mut MessageWriter<UseItemExplorationAction>,
    drop_writer: &mut MessageWriter<DropItemAction>,
    transfer_writer: &mut MessageWriter<TransferItemAction>,
    equip_writer: &mut MessageWriter<EquipItemAction>,
    unequip_writer: &mut MessageWriter<UnequipItemAction>,
) {
    if let Some(action) = pending_action {
        match action {
            PanelAction::Use {
                party_index,
                slot_index,
            } => {
                use_writer.write(UseItemExplorationAction {
                    party_index,
                    slot_index,
                });
            }
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
            PanelAction::Equip {
                party_index,
                slot_index,
            } => {
                equip_writer.write(EquipItemAction {
                    party_index,
                    slot_index,
                });
            }
            PanelAction::Unequip { party_index, slot } => {
                unequip_writer.write(UnequipItemAction { party_index, slot });
            }
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
#[allow(clippy::too_many_arguments)]
fn inventory_ui_system(
    mut contexts: EguiContexts,
    mut global_state: ResMut<GlobalState>,
    game_content: Option<Res<GameContent>>,
    mut nav_state: ResMut<InventoryNavigationState>,
    mut drop_writer: MessageWriter<DropItemAction>,
    mut transfer_writer: MessageWriter<TransferItemAction>,
    mut use_writer: MessageWriter<UseItemExplorationAction>,
    mut equip_writer: MessageWriter<EquipItemAction>,
    mut unequip_writer: MessageWriter<UnequipItemAction>,
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
    let mut clicked_slot_update: Option<(usize, usize, bool)> = None;

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

        render_equipment_panel(
            ui,
            &global_state,
            focused_index,
            selected_slot,
            game_content.as_deref(),
            &nav_state,
        );
        ui.separator();

        // ── Panel layout ─────────────────────────────────────────────────
        let (action, slot_update) = render_item_grid(
            ui,
            &open_panels,
            focused_index,
            selected_slot,
            &nav_state,
            &global_state,
            game_content.as_deref(),
            &panel_names,
        );
        pending_action = action;
        clicked_slot_update = slot_update;
    });

    if let Some((party_index, slot_idx, has_item)) = clicked_slot_update {
        if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
            inv_state.focused_index = party_index;
            inv_state.selected_slot = Some(slot_idx);
        }

        nav_state.selected_slot_index = Some(slot_idx);
        nav_state.focus_on_panel = open_panels
            .iter()
            .position(|&pi| pi == party_index)
            .unwrap_or(nav_state.focus_on_panel);
        nav_state.focused_action_index = 0;
        nav_state.phase = if has_item {
            NavigationPhase::ActionNavigation
        } else {
            NavigationPhase::SlotNavigation
        };
        nav_state.selected_equip_slot = None;
    }

    render_action_bar(
        pending_action,
        &mut use_writer,
        &mut drop_writer,
        &mut transfer_writer,
        &mut equip_writer,
        &mut unequip_writer,
    );
}

/// Renders a single character panel at a fixed pixel size.
///
/// Layout (top to bottom):
/// - Dark header bar (`PANEL_HEADER_H` px) — character name only.
/// - Equipment strip (`EQUIP_STRIP_H` px) — seven equipment cells in two rows.
/// - Body — painter-drawn slot grid filling the remaining height.
/// - Action strip (`PANEL_ACTION_H` px) — Drop / Give / Equip / Unequip buttons.
///
/// The slot grid is drawn entirely via `egui::Painter` so it looks like the
/// mockup: dark background, faint grid lines, white item-type silhouettes.
///
/// ## Equipment strip
///
/// The strip shows all seven equipment slots in two rows:
/// - Row 1: Weapon · Armor · Shield
/// - Row 2: Helmet · Boots · Ring · Ring
///
/// Each cell shows the slot label and equipped item name (or "—" when empty).
/// When `selected_equip_slot` is `Some(slot)`, that cell is highlighted and
/// an **Unequip** button appears below the strip.
///
/// ## Action strip keyboard highlight
///
/// When `focused_action_index` is `Some(n)`, the nth action button in the strip
/// is rendered with a yellow border ring to indicate keyboard focus.  Mouse
/// clicks are still processed normally regardless of keyboard focus.
///
/// # Returns
///
/// A [`CharacterPanelResult`] containing:
/// - `action`: an action-button click (Drop, Give, Equip, or Unequip)
/// - `clicked_slot`: a slot-grid click detected from the painter-driven grid
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
    selected_equip_slot: Option<EquipmentSlot>,
) -> CharacterPanelResult {
    if party_index >= global_state.0.party.members.len() {
        return CharacterPanelResult {
            action: None,
            clicked_slot: None,
        };
    }

    let character = &global_state.0.party.members[party_index];
    let items = &character.inventory.items;
    let mut panel_action: Option<PanelAction> = None;
    let mut clicked_slot: Option<usize> = None;

    // How much vertical space do the action strips need?
    let has_equip_action = selected_equip_slot.is_some();
    let equip_action_reserve = if has_equip_action {
        PANEL_ACTION_H
    } else {
        0.0
    };
    let has_action = selected_slot.map(|s| s < items.len()).unwrap_or(false);
    let action_reserve = if has_action { PANEL_ACTION_H } else { 0.0 };
    let body_h =
        (size.y - PANEL_HEADER_H - EQUIP_STRIP_H - equip_action_reserve - action_reserve).max(20.0);

    // ── Outer border ─────────────────────────────────────────────────────
    let border_color = if is_focused {
        FOCUSED_BORDER_COLOR
    } else {
        UNFOCUSED_BORDER_COLOR
    };
    let (panel_rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter().clone();
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

    // ── Equipment strip ───────────────────────────────────────────────────
    // Two rows of cells: Row 1 = Weapon / Armor / Shield (3 cells)
    //                    Row 2 = Helmet / Boots / Ring  / Ring  (4 cells)
    let equip_strip_rect = egui::Rect::from_min_size(
        panel_rect.min + egui::vec2(0.0, PANEL_HEADER_H),
        egui::vec2(size.x, EQUIP_STRIP_H),
    );
    painter.rect_filled(equip_strip_rect, 0.0, PANEL_BG_COLOR);

    let equip_cell_w = (size.x / 4.0).floor().max(20.0);
    let equip_cell_h = (EQUIP_STRIP_H / 2.0).floor();

    // All seven slots grouped into rows for rendering
    let equip_rows: [&[(EquipmentSlot, &str)]; 2] = [
        &[
            (EquipmentSlot::Weapon, "Weapon"),
            (EquipmentSlot::Armor, "Armor"),
            (EquipmentSlot::Shield, "Shield"),
        ],
        &[
            (EquipmentSlot::Helmet, "Helmet"),
            (EquipmentSlot::Boots, "Boots"),
            (EquipmentSlot::Accessory1, "Ring"),
            (EquipmentSlot::Accessory2, "Ring"),
        ],
    ];

    for (row_idx, row_slots) in equip_rows.iter().enumerate() {
        for (col_idx, (slot, slot_label)) in row_slots.iter().enumerate() {
            let cell_min = equip_strip_rect.min
                + egui::vec2(col_idx as f32 * equip_cell_w, row_idx as f32 * equip_cell_h);
            let cell_rect =
                egui::Rect::from_min_size(cell_min, egui::vec2(equip_cell_w, equip_cell_h));

            // Cell border
            painter.rect_stroke(
                cell_rect.shrink(1.0),
                1.0,
                egui::Stroke::new(1.0, GRID_LINE_COLOR),
                egui::StrokeKind::Outside,
            );

            // Keyboard-focus selection highlight (green tint)
            if selected_equip_slot == Some(*slot) {
                painter.rect_filled(
                    cell_rect.shrink(1.0),
                    0.0,
                    egui::Color32::from_rgba_premultiplied(0, 120, 60, 60),
                );
                painter.rect_stroke(
                    cell_rect.shrink(1.0),
                    1.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 220, 100)),
                    egui::StrokeKind::Outside,
                );
            }

            // Slot label in small dimmed text
            painter.text(
                cell_rect.min + egui::vec2(3.0, 2.0),
                egui::Align2::LEFT_TOP,
                slot_label,
                egui::FontId::proportional(9.0),
                egui::Color32::from_rgba_premultiplied(130, 130, 130, 255),
            );

            // Equipped item name or "—"
            let (item_display, item_color): (std::borrow::Cow<str>, egui::Color32) =
                match slot.get(&character.equipment) {
                    Some(item_id) => {
                        let name = game_content
                            .and_then(|gc| gc.db().items.get_item(item_id))
                            .map(|it| std::borrow::Cow::Owned(it.name.clone()))
                            .unwrap_or_else(|| std::borrow::Cow::Owned(format!("#{item_id}")));
                        (name, egui::Color32::WHITE)
                    }
                    None => (
                        std::borrow::Cow::Borrowed("—"),
                        egui::Color32::from_rgba_premultiplied(90, 90, 90, 255),
                    ),
                };

            painter.text(
                cell_rect.left_center() + egui::vec2(3.0, 5.0),
                egui::Align2::LEFT_CENTER,
                item_display.as_ref(),
                egui::FontId::proportional(10.0),
                item_color,
            );

            // Mouse-click on a filled slot → dispatch Unequip immediately
            let cell_sense = ui.allocate_rect(cell_rect, egui::Sense::click());
            if cell_sense.clicked() && slot.get(&character.equipment).is_some() {
                panel_action = Some(PanelAction::Unequip {
                    party_index,
                    slot: *slot,
                });
            }
        }
    }

    // ── Unequip button strip (below equipment strip, keyboard-nav only) ───
    if has_equip_action {
        let unequip_rect = egui::Rect::from_min_size(
            egui::pos2(equip_strip_rect.min.x, equip_strip_rect.max.y),
            egui::vec2(size.x, equip_action_reserve),
        );
        painter.rect_filled(unequip_rect, 0.0, HEADER_BG_COLOR);

        let mut unequip_child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(unequip_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        unequip_child.add_space(6.0);
        unequip_child.push_id("equip_strip_actions", |ui| {
            ui.horizontal_wrapped(|ui| {
                let unequip_label = egui::RichText::new("Unequip")
                    .color(egui::Color32::from_rgb(100, 200, 100))
                    .small();
                if ui
                    .add(egui::Button::new(unequip_label))
                    .on_hover_text("Return this item to inventory")
                    .clicked()
                {
                    if let Some(slot) = selected_equip_slot {
                        panel_action = Some(PanelAction::Unequip { party_index, slot });
                    }
                }
            });
        });
    }

    // ── Body: slot grid ───────────────────────────────────────────────────
    let body_rect = egui::Rect::from_min_size(
        panel_rect.min + egui::vec2(0.0, PANEL_HEADER_H + EQUIP_STRIP_H + equip_action_reserve),
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

        let cell_response = ui.allocate_rect(cell_rect, egui::Sense::click());

        // Item silhouette
        if slot_idx < items.len() {
            let item_type = game_content
                .and_then(|gc| gc.db().items.get_item(items[slot_idx].item_id))
                .map(|it| &it.item_type);
            paint_item_silhouette(
                &painter,
                cell_rect,
                cell_size,
                item_type,
                ITEM_SILHOUETTE_COLOR,
            );
        }

        if cell_response.clicked() {
            clicked_slot = Some(slot_idx);
        }
    }

    // ── Action strip (egui widgets, below the painted body) ───────────────
    if has_action {
        if let Some(slot_idx) = selected_slot {
            let action_rect = egui::Rect::from_min_size(
                panel_rect.min
                    + egui::vec2(
                        0.0,
                        PANEL_HEADER_H + EQUIP_STRIP_H + equip_action_reserve + body_h,
                    ),
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
                    // Look up item once for both Equip and Use checks.
                    let item_opt = items
                        .get(slot_idx)
                        .and_then(|s| game_content?.db().items.get_item(s.item_id));

                    let is_equipable = item_opt
                        .and_then(|item| EquipmentSlot::for_item(item, &character.equipment))
                        .is_some();

                    let is_consumable = item_opt
                        .map(|item| matches!(item.item_type, ItemType::Consumable(_)))
                        .unwrap_or(false);

                    // Running index used to match focused_action_index
                    let mut btn_idx: usize = 0;

                    // ── Action: Equip (equipable items, appears first) ─────
                    if is_equipable {
                        let equip_focused = focused_action_index == Some(btn_idx);
                        let equip_label = egui::RichText::new("Equip")
                            .color(if equip_focused {
                                ACTION_FOCUSED_COLOR
                            } else {
                                egui::Color32::from_rgb(100, 200, 100)
                            })
                            .small();
                        let mut equip_btn = egui::Button::new(equip_label);
                        if equip_focused {
                            equip_btn =
                                equip_btn.stroke(egui::Stroke::new(2.0, ACTION_FOCUSED_COLOR));
                        }
                        if ui.add(equip_btn).on_hover_text("Equip this item").clicked() {
                            panel_action = Some(PanelAction::Equip {
                                party_index,
                                slot_index: slot_idx,
                            });
                        }
                        btn_idx += 1;
                    }

                    // ── Action: Use (consumable only, appears after Equip) ─
                    if is_consumable {
                        let use_focused = focused_action_index == Some(btn_idx);
                        let use_label = egui::RichText::new("Use")
                            .color(if use_focused {
                                ACTION_FOCUSED_COLOR
                            } else {
                                egui::Color32::from_rgb(100, 180, 255)
                            })
                            .small();
                        let mut use_btn = egui::Button::new(use_label);
                        if use_focused {
                            use_btn = use_btn.stroke(egui::Stroke::new(2.0, ACTION_FOCUSED_COLOR));
                        }
                        if ui
                            .add(use_btn)
                            .on_hover_text("Use this consumable item")
                            .clicked()
                        {
                            panel_action = Some(PanelAction::Use {
                                party_index,
                                slot_index: slot_idx,
                            });
                        }
                        btn_idx += 1;
                    }

                    // ── Action: Drop ──────────────────────────────────────
                    let drop_focused = focused_action_index == Some(btn_idx);
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
                    btn_idx += 1;

                    // ── Actions: Transfer to other party members ──────────
                    for &(other_index, ref other_name) in panel_names {
                        if other_index == party_index {
                            continue;
                        }
                        let target_full = global_state.0.party.members[other_index]
                            .inventory
                            .is_full();
                        let transfer_focused = focused_action_index == Some(btn_idx);
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
                        btn_idx += 1;
                    }
                    // Suppress unused variable warning when no transfers
                    let _ = btn_idx;
                });
            });
        }
    }

    CharacterPanelResult {
        action: panel_action,
        clicked_slot,
    }
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
#[allow(clippy::too_many_arguments)]
fn inventory_action_system(
    mut drop_reader: MessageReader<DropItemAction>,
    mut transfer_reader: MessageReader<TransferItemAction>,
    mut equip_reader: MessageReader<EquipItemAction>,
    mut unequip_reader: MessageReader<UnequipItemAction>,
    mut global_state: ResMut<GlobalState>,
    mut nav_state: ResMut<InventoryNavigationState>,
    mut item_dropped_writer: Option<MessageWriter<ItemDroppedEvent>>,
    mut game_log_writer: Option<MessageWriter<GameLogEvent>>,
    game_content: Option<Res<GameContent>>,
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

    let equip_events: Vec<(usize, usize)> = equip_reader
        .read()
        .map(|e| (e.party_index, e.slot_index))
        .collect();

    let unequip_events: Vec<(usize, EquipmentSlot)> = unequip_reader
        .read()
        .map(|e| (e.party_index, e.slot))
        .collect();

    // ── Drop events ─────────────────────────────────────────────────────────
    for (party_index, slot_index) in drop_events {
        // ── Bounds-check party index ────────────────────────────────────────
        if party_index >= global_state.0.party.members.len() {
            warn!(
                "DropItemAction: party_index {} out of bounds (party size {})",
                party_index,
                global_state.0.party.members.len()
            );
            continue;
        }

        // ── Bounds-check slot index ─────────────────────────────────────────
        let inv_len = global_state.0.party.members[party_index]
            .inventory
            .items
            .len();
        if slot_index >= inv_len {
            warn!(
                "DropItemAction: slot_index {} out of bounds (inventory size {}) for party[{}]",
                slot_index, inv_len, party_index
            );
            continue;
        }

        // ── Read world position values as copies before splitting the borrow ─
        let map_id = global_state.0.world.current_map;
        let pos = global_state.0.world.party_position;

        // ── Call drop_item() — splits borrow across party and world fields ───
        //
        // Rust's NLL (Non-Lexical Lifetimes) allows simultaneous mutable borrows
        // of distinct struct fields.  `game_state.party.members[i]` and
        // `game_state.world` are disjoint fields of `GameState`.
        let game_state = &mut global_state.0;
        match drop_item(
            &mut game_state.party.members[party_index],
            party_index,
            slot_index,
            &mut game_state.world,
            map_id,
            pos,
        ) {
            Ok(dropped) => {
                info!(
                    "Dropped item from party[{}] slot {} (item_id={}, charges={}) \
                     onto map {} at {:?}",
                    party_index, slot_index, dropped.item_id, dropped.charges, map_id, pos
                );

                let item_name = game_content
                    .as_deref()
                    .and_then(|content| content.db().items.get_item(dropped.item_id))
                    .map(|item| item.name.clone())
                    .unwrap_or_else(|| format!("item {}", dropped.item_id));

                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: format!("Dropped {}.", item_name),
                        category: LogCategory::Item,
                    });
                }

                // Fire ItemDroppedEvent so the 3-D world mesh spawns at the
                // party's current tile position (visual system).
                if let Some(ref mut writer) = item_dropped_writer {
                    writer.write(ItemDroppedEvent {
                        item_id: dropped.item_id,
                        charges: dropped.charges as u16,
                        map_id,
                        tile_x: pos.x,
                        tile_y: pos.y,
                    });
                }
            }
            Err(e) => {
                warn!(
                    "DropItemAction: drop_item failed for party[{}] slot {}: {}",
                    party_index, slot_index, e
                );
            }
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

    // ── Equip events ─────────────────────────────────────────────────────────
    for (party_index, slot_index) in equip_events {
        // Bounds-check party index
        if party_index >= global_state.0.party.members.len() {
            warn!(
                "EquipItemAction: party_index {} out of bounds (party size {})",
                party_index,
                global_state.0.party.members.len()
            );
            continue;
        }

        // Bounds-check slot index
        let inv_len = global_state.0.party.members[party_index]
            .inventory
            .items
            .len();
        if slot_index >= inv_len {
            warn!(
                "EquipItemAction: slot_index {} out of bounds (inventory size {}) for party[{}]",
                slot_index, inv_len, party_index
            );
            continue;
        }

        // Resolve game content — required for equip validation
        let content = match game_content.as_deref() {
            Some(gc) => gc,
            None => {
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: "Cannot equip: game content not available.".to_string(),
                        category: LogCategory::System,
                    });
                }
                continue;
            }
        };
        let content_db = content.db();

        let item_name = global_state.0.party.members[party_index]
            .inventory
            .items
            .get(slot_index)
            .and_then(|slot| content_db.items.get_item(slot.item_id))
            .map(|item| item.name.clone())
            .unwrap_or_else(|| "unknown item".to_string());
        let character_name = global_state.0.party.members[party_index].name.clone();

        match equip_item(
            &mut global_state.0.party.members[party_index],
            slot_index,
            &content_db.items,
            &content_db.classes,
            &content_db.races,
        ) {
            Ok(()) => {
                info!(
                    "Equipped item from party[{}] slot {}",
                    party_index, slot_index
                );
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: format!("{} equipped {}.", character_name, item_name),
                        category: LogCategory::Item,
                    });
                }
                if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
                    inv_state.selected_slot = None;
                }
                nav_state.selected_slot_index = None;
                nav_state.focused_action_index = 0;
                nav_state.phase = NavigationPhase::SlotNavigation;
            }
            Err(e) => {
                let msg = match &e {
                    EquipError::ItemNotFound(_) => "Cannot equip: item not found.".to_string(),
                    EquipError::ClassRestriction => "Cannot equip: class restriction.".to_string(),
                    EquipError::RaceRestriction => "Cannot equip: race restriction.".to_string(),
                    EquipError::NoSlotAvailable => "Cannot equip: no slot available.".to_string(),
                    EquipError::AlignmentRestriction => {
                        "Cannot equip: alignment restriction.".to_string()
                    }
                    EquipError::InvalidRace(s) => format!("Cannot equip: invalid race ({s})."),
                    EquipError::InvalidClass(s) => format!("Cannot equip: invalid class ({s})."),
                };
                warn!(
                    "EquipItemAction failed for party[{}] slot {}: {}",
                    party_index, slot_index, msg
                );
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Item,
                    });
                }
            }
        }
    }

    // ── Unequip events ────────────────────────────────────────────────────────
    for (party_index, slot) in unequip_events {
        // Bounds-check party index
        if party_index >= global_state.0.party.members.len() {
            warn!(
                "UnequipItemAction: party_index {} out of bounds (party size {})",
                party_index,
                global_state.0.party.members.len()
            );
            continue;
        }

        // Resolve game content — required for AC recalculation
        let content = match game_content.as_deref() {
            Some(gc) => gc,
            None => {
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: "Cannot unequip: game content not available.".to_string(),
                        category: LogCategory::System,
                    });
                }
                continue;
            }
        };
        let content_db = content.db();

        match unequip_item(
            &mut global_state.0.party.members[party_index],
            slot,
            &content_db.items,
        ) {
            Ok(()) => {
                info!("Unequipped slot {:?} from party[{}]", slot, party_index);
                nav_state.selected_equip_slot = None;
            }
            Err(TransactionError::InventoryFull { .. }) => {
                let msg = "Cannot unequip: inventory is full.".to_string();
                warn!("UnequipItemAction: {}", msg);
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category: LogCategory::Item,
                    });
                }
            }
            Err(e) => {
                warn!(
                    "UnequipItemAction: unexpected error for party[{}] slot {:?}: {}",
                    party_index, slot, e
                );
            }
        }
    }
}

// ===== Exploration Use Helpers =====

/// Formats a validation error from [`ItemUseError`] into a player-facing message.
fn build_use_error_message(e: &ItemUseError, item_name: &str) -> String {
    match e {
        ItemUseError::InventorySlotInvalid(_) => {
            "Cannot use item: no item in that slot.".to_string()
        }
        ItemUseError::ItemNotFound(_) => "Cannot use item: item data not found.".to_string(),
        ItemUseError::NotConsumable => {
            format!("Cannot use {item_name}: not a consumable.")
        }
        ItemUseError::NotUsableInCombat => {
            format!("Cannot use {item_name} outside of combat.")
        }
        ItemUseError::NoCharges => {
            format!("Cannot use {item_name}: no charges remaining.")
        }
        ItemUseError::AlignmentRestriction => {
            format!("Cannot use {item_name}: alignment restriction.")
        }
        ItemUseError::ClassRestriction => {
            format!("Cannot use {item_name}: class restriction.")
        }
        ItemUseError::RaceRestriction => {
            format!("Cannot use {item_name}: race restriction.")
        }
        ItemUseError::InvalidTarget => {
            format!("Cannot use {item_name}: invalid target.")
        }
        ItemUseError::Other(msg) => {
            format!("Cannot use item: {msg}.")
        }
    }
}

/// Resolves the item name and [`ConsumableData`] for a consumable use action.
///
/// Returns `Err((message, category))` if the slot is empty, the item is not
/// found in the database, or the item is not a consumable.
fn resolve_consumable_for_use(
    character: &crate::domain::character::Character,
    slot_index: usize,
    game_content: &GameContent,
) -> Result<(String, ConsumableData), (String, LogCategory)> {
    let content_db = game_content.db();
    let slot = character.inventory.items.get(slot_index).ok_or_else(|| {
        (
            "Cannot use item: no item in that slot.".to_string(),
            LogCategory::System,
        )
    })?;
    let item = content_db.items.get_item(slot.item_id).ok_or_else(|| {
        (
            "Cannot use item: item data not found.".to_string(),
            LogCategory::System,
        )
    })?;
    let consumable = match &item.item_type {
        ItemType::Consumable(data) => *data,
        _ => {
            return Err((
                format!("Cannot use {}: not a consumable.", item.name),
                LogCategory::Item,
            ))
        }
    };
    Ok((item.name.clone(), consumable))
}

/// Builds the player-facing log message after a consumable effect is applied.
fn build_consumable_use_log(
    consumable_data: &ConsumableData,
    result: &ConsumableApplyResult,
    item_name: &str,
    character_name: &str,
) -> String {
    let minutes_opt = normalize_duration(consumable_data.duration_minutes);

    match consumable_data.effect {
        ConsumableEffect::HealHp(_) => {
            if result.healing == 0 {
                format!("{item_name} used. {character_name} was already at full health.")
            } else {
                format!(
                    "{item_name} used. {character_name} recovered {} HP.",
                    result.healing
                )
            }
        }
        ConsumableEffect::RestoreSp(_) => {
            if result.sp_restored == 0 {
                format!("{item_name} used. {character_name} was already at full SP.")
            } else {
                format!(
                    "{item_name} used. {character_name} recovered {} SP.",
                    result.sp_restored
                )
            }
        }
        ConsumableEffect::CureCondition(_) => {
            format!("{item_name} used. Conditions cleared.")
        }
        ConsumableEffect::BoostAttribute(attr, _) => {
            if result.attribute_boost_is_timed {
                if let Some(mins) = minutes_opt {
                    format!(
                        "{item_name} used. {} increased for {} minutes.",
                        attr.display_name(),
                        mins
                    )
                } else {
                    format!(
                        "{item_name} used. {character_name}'s {} increased.",
                        attr.display_name()
                    )
                }
            } else {
                format!(
                    "{item_name} used. {character_name}'s {} increased.",
                    attr.display_name()
                )
            }
        }
        ConsumableEffect::BoostResistance(res, _) => {
            if result.resistance_boost_is_timed {
                if let Some(mins) = minutes_opt {
                    format!(
                        "{item_name} used. {} resistance active for {} minutes.",
                        res.display_name(),
                        mins
                    )
                } else {
                    format!(
                        "{item_name} used. {character_name}'s {} resistance increased.",
                        res.display_name()
                    )
                }
            } else {
                format!(
                    "{item_name} used. {character_name}'s {} resistance increased.",
                    res.display_name()
                )
            }
        }
        ConsumableEffect::IsFood(_) => {
            format!("{item_name} used.")
        }
        ConsumableEffect::Resurrect(_) => {
            if result.healing == 0 {
                format!("{item_name} used. {character_name} could not be resurrected.")
            } else {
                format!(
                    "{item_name} used. {character_name} has been resurrected with {} HP.",
                    result.healing
                )
            }
        }
        ConsumableEffect::CastSpell(spell_id) => {
            // The actual cast is dispatched in handle_use_item_action_exploration
            // and a more informative message is built there.  This fallback is
            // used when the spell ID cannot be resolved.
            format!("{item_name} used. Casting spell {}.", spell_id)
        }
        ConsumableEffect::LearnSpell(spell_id) => {
            // The actual learn call is dispatched in handle_use_item_action_exploration.
            format!(
                "{item_name} used. {} attempts to learn spell {}.",
                character_name, spell_id
            )
        }
    }
}

// ===== Exploration Use Handler =====

/// Handles [`UseItemExplorationAction`] messages emitted by the inventory UI.
///
/// For each message this system:
/// 1. Resolves `game_content` (returns early if unavailable).
/// 2. Bounds-checks `party_index`.
/// 3. Validates the item via [`validate_item_use_slot`] with `in_combat = false`.
/// 4. Captures the item name and [`ConsumableEffect`] before mutation.
/// 5. Consumes one charge (or removes the slot when the last charge is spent).
/// 6. Applies the effect to the owning character via [`apply_consumable_effect_exploration`].
/// 7. Writes a player-visible [`GameLog`] message describing the outcome.
/// 8. Resets navigation state so the UI returns to slot-navigation phase.
///
/// Every failure path — including validation errors and defensive charge
/// checks — writes a [`GameLog`] entry so the player is never silently blocked.
///
/// # Design
///
/// Self-target only: the effect is always applied to the character who owns
/// the item. Cross-party targeting is out of scope for this phase.
fn handle_use_item_action_exploration(
    mut reader: MessageReader<UseItemExplorationAction>,
    mut global_state: ResMut<GlobalState>,
    mut nav_state: ResMut<InventoryNavigationState>,
    game_content: Option<Res<GameContent>>,
    mut game_log_writer: Option<MessageWriter<GameLogEvent>>,
) {
    // Collect messages upfront to avoid borrow conflicts with mutable state.
    let messages: Vec<(usize, usize)> = reader
        .read()
        .map(|m| (m.party_index, m.slot_index))
        .collect();

    if messages.is_empty() {
        return;
    }

    // Step 1: resolve game_content — required for validation and item lookup.
    let content = match game_content.as_deref() {
        Some(gc) => gc,
        None => {
            if let Some(ref mut writer) = game_log_writer {
                writer.write(GameLogEvent {
                    text: "Cannot use item: game content not available.".to_string(),
                    category: LogCategory::System,
                });
            }
            return;
        }
    };
    let content_db = content.db();

    for (party_index, slot_index) in messages {
        // Step 2: bounds-check party_index.
        if party_index >= global_state.0.party.members.len() {
            if let Some(ref mut writer) = game_log_writer {
                writer.write(GameLogEvent {
                    text: "Cannot use item: invalid character.".to_string(),
                    category: LogCategory::System,
                });
            }
            continue;
        }

        // Step 3: validate via shared gate (in_combat = false).
        let validation_result = {
            let character = &global_state.0.party.members[party_index];
            validate_item_use_slot(character, slot_index, content_db, false)
        };

        if let Err(ref e) = validation_result {
            let item_name: String = global_state
                .0
                .party
                .members
                .get(party_index)
                .and_then(|ch| ch.inventory.items.get(slot_index))
                .and_then(|slot| content_db.items.get_item(slot.item_id))
                .map(|item| item.name.clone())
                .unwrap_or_else(|| "that item".to_string());

            if let Some(ref mut writer) = game_log_writer {
                writer.write(GameLogEvent {
                    text: build_use_error_message(e, &item_name),
                    category: LogCategory::Item,
                });
            }

            // Reset navigation state even on failure so the UI is not stuck.
            if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
                inv_state.selected_slot = None;
            }
            nav_state.selected_slot_index = None;
            nav_state.focused_action_index = 0;
            nav_state.phase = NavigationPhase::SlotNavigation;

            continue;
        }

        // Step 4: capture item name and full ConsumableData before any mutation.
        let (item_name, consumable_data) = match resolve_consumable_for_use(
            &global_state.0.party.members[party_index],
            slot_index,
            content,
        ) {
            Ok(result) => result,
            Err((msg, category)) => {
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: msg,
                        category,
                    });
                }
                continue;
            }
        };

        // Step 5: consume one charge (mutable borrow of the character).
        {
            let character = &mut global_state.0.party.members[party_index];
            let charges = match character.inventory.items.get(slot_index) {
                Some(s) => s.charges,
                None => {
                    if let Some(ref mut writer) = game_log_writer {
                        writer.write(GameLogEvent {
                            text: "Cannot use item: no item in that slot.".to_string(),
                            category: LogCategory::System,
                        });
                    }
                    continue;
                }
            };

            if charges == 0 {
                // Defensive check — validate_item_use_slot should have caught this.
                if let Some(ref mut writer) = game_log_writer {
                    writer.write(GameLogEvent {
                        text: format!("Cannot use {item_name}: no charges remaining."),
                        category: LogCategory::Item,
                    });
                }
                continue;
            } else if charges > 1 {
                character.inventory.items[slot_index].charges -= 1;
            } else {
                // charges == 1: remove the slot entirely.
                let _ = character.inventory.remove_item(slot_index);
            }
        }

        // Capture character name before any mutable borrows in steps 6a/6b.
        let character_name = global_state
            .0
            .party
            .members
            .get(party_index)
            .map(|ch| ch.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Step 6: apply the effect to the owning character via the exploration
        // helper. Split borrows: get active_spells separately from the character
        // so the borrow checker sees them as disjoint fields.
        let result: ConsumableApplyResult = {
            let gs = &mut global_state.0;
            let character = &mut gs.party.members[party_index];
            apply_consumable_effect_exploration(character, &mut gs.active_spells, &consumable_data)
        };

        // Step 6a: dispatch ConsumableEffect::CastSpell.
        //
        // `apply_consumable_effect_exploration` signals the spell ID via
        // `result.spell_cast_id` without casting anything itself.  Here we
        // perform the actual cast through the exploration pipeline so that SP
        // is consumed, buffs are applied, and healing takes effect.
        let cast_spell_log: Option<String> = if let Some(spell_id) = result.spell_cast_id {
            if let Some(spell_def) = content_db.spells.get_spell(spell_id).cloned() {
                let spell_name = spell_def.name.clone();
                let mut rng = rand::rng();
                match cast_exploration_spell(
                    party_index,
                    &spell_def,
                    ExplorationTarget::Self_,
                    &mut global_state.0,
                    &content_db.items,
                    &mut rng,
                ) {
                    Ok(_) => Some(format!(
                        "{item_name} used. {character_name} casts {spell_name}."
                    )),
                    Err(e) => Some(format!(
                        "{item_name} used. Failed to cast {spell_name}: {e}."
                    )),
                }
            } else {
                None
            }
        } else {
            None
        };

        // Step 6b: dispatch ConsumableEffect::LearnSpell.
        //
        // `apply_consumable_effect_exploration` signals the spell ID via
        // `result.spell_learn_id` without modifying the spellbook.  Here we
        // call `learn_spell` directly to perform the actual acquisition.  On
        // failure (wrong class, already known, etc.) the error is logged and
        // the item charge is NOT refunded — the scroll was consumed regardless.
        let learn_spell_log: Option<String> = if let Some(spell_id) = result.spell_learn_id {
            let spell_name = content_db
                .spells
                .get_spell(spell_id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| spell_id.to_string());
            match learn_spell(
                &mut global_state.0.party.members[party_index],
                spell_id,
                &content_db.spells,
                &content_db.classes,
            ) {
                Ok(()) => Some(format!(
                    "{item_name} used. {character_name} learned {spell_name}."
                )),
                Err(SpellLearnError::AlreadyKnown(_)) => Some(format!(
                    "{item_name} used. {character_name} already knows {spell_name}."
                )),
                Err(e) => {
                    tracing::warn!(
                        "LearnSpell scroll: could not teach {} to {} — {}",
                        spell_name,
                        character_name,
                        e
                    );
                    Some(format!(
                        "{item_name} used. {character_name} could not learn {spell_name}."
                    ))
                }
            }
        } else {
            None
        };

        // Step 7: write a success GameLog message.  Spell-dispatch outcomes
        // (CastSpell / LearnSpell) override the generic consumable message so
        // the player sees the resolved spell name rather than a raw spell ID.
        let log_msg = cast_spell_log.or(learn_spell_log).unwrap_or_else(|| {
            build_consumable_use_log(&consumable_data, &result, &item_name, &character_name)
        });

        if let Some(ref mut writer) = game_log_writer {
            writer.write(GameLogEvent {
                text: log_msg,
                category: LogCategory::Item,
            });
        }

        // Step 8: reset navigation state.
        if let GameMode::Inventory(ref mut inv_state) = global_state.0.mode {
            inv_state.selected_slot = None;
        }
        nav_state.selected_slot_index = None;
        nav_state.focused_action_index = 0;
        nav_state.phase = NavigationPhase::SlotNavigation;
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameState;
    use crate::game::systems::ui::GameLog;

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
                let result = render_character_panel(
                    ui,
                    0,                        // party_index
                    true,                     // is_focused
                    None,                     // selected_slot
                    None,                     // focused_action_index
                    egui::vec2(300.0, 400.0), // size
                    &global_state,
                    None, // no GameContent needed
                    &[],  // no open panels for transfer buttons
                    None, // no equipment strip selection
                );
                assert!(result.action.is_none());
                assert!(result.clicked_slot.is_none());
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
                let result = render_character_panel(
                    ui,
                    0,                        // party_index
                    false,                    // not focused
                    Some(0),                  // first slot selected
                    None,                     // no keyboard action focus
                    egui::vec2(300.0, 400.0), // size
                    &global_state,
                    None, // no GameContent
                    &[],  // no open panels for transfer buttons
                    None, // no equipment strip selection
                );
                assert!(result.clicked_slot.is_none());
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
                let result = render_character_panel(
                    ui,
                    0, // out-of-bounds
                    true,
                    None,
                    None,
                    egui::vec2(300.0, 400.0), // size
                    &global_state,
                    None,
                    &[],
                    None,
                );
                assert!(result.action.is_none());
                assert!(result.clicked_slot.is_none());
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
            selected_equip_slot: None,
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
        app.add_message::<EquipItemAction>();
        app.add_message::<UnequipItemAction>();

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

        // Add a default map to the world (map id=0 matches world.current_map default)
        // so that drop_item() can persist the dropped item to the map.
        {
            use crate::domain::world::Map;
            let map = Map::new(0, "Default Map".to_string(), "Test".to_string(), 20, 20);
            game_state.world.add_map(map);
        }

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
        app.add_message::<EquipItemAction>();
        app.add_message::<UnequipItemAction>();

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

    /// Full test: a `DropItemAction` removes the item from inventory
    /// and sets `selected_slot` to `None`.
    #[test]
    fn test_drop_item_action_removes_from_inventory() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::types::ItemId;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<DropItemAction>();
        app.add_message::<TransferItemAction>();
        app.add_message::<EquipItemAction>();
        app.add_message::<UnequipItemAction>();

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

        // Add a default map so drop_item() can persist the item to the world.
        {
            use crate::domain::world::Map;
            let map = Map::new(0, "Default Map".to_string(), "Test".to_string(), 20, 20);
            game_state.world.add_map(map);
        }

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
        app.add_message::<EquipItemAction>();
        app.add_message::<UnequipItemAction>();

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
        app.add_message::<EquipItemAction>();
        app.add_message::<UnequipItemAction>();

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
        app.add_message::<EquipItemAction>();
        app.add_message::<UnequipItemAction>();

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
        app.add_message::<EquipItemAction>();
        app.add_message::<UnequipItemAction>();

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
        app.add_message::<EquipItemAction>();
        app.add_message::<UnequipItemAction>();

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
            PanelAction::Use { .. } => panic!("expected Drop variant"),
            PanelAction::Equip { .. } => panic!("expected Drop variant"),
            PanelAction::Unequip { .. } => panic!("expected Drop variant"),
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
            PanelAction::Use { .. } => panic!("expected Transfer variant"),
            PanelAction::Equip { .. } => panic!("expected Transfer variant"),
            PanelAction::Unequip { .. } => panic!("expected Transfer variant"),
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
                // No simulated click occurs in headless context; return value carries no action.
                let result = render_character_panel(
                    ui,
                    0,                        // party_index
                    true,                     // is_focused
                    Some(0),                  // slot 0 selected (has item)
                    None,                     // no keyboard action focus
                    egui::vec2(300.0, 400.0), // size
                    &global_state,
                    None,
                    &panel_names,
                    None,
                );
                assert!(result.action.is_none());
                assert!(result.clicked_slot.is_none());
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
                let result = render_character_panel(
                    ui,
                    0,
                    true,
                    Some(0), // slot 0 is selected
                    Some(0), // keyboard focus on Drop button (index 0)
                    egui::vec2(300.0, 400.0),
                    &global_state,
                    None,
                    &panel_names,
                    None,
                );
                assert!(result.action.is_none());
                assert!(result.clicked_slot.is_none());
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
                let result = render_character_panel(
                    ui,
                    0,
                    true,
                    Some(0), // slot 0 selected
                    Some(1), // keyboard focus on Transfer→Ally (index 1)
                    egui::vec2(300.0, 400.0),
                    &global_state,
                    None,
                    &panel_names,
                    None,
                );
                assert!(result.action.is_none());
                assert!(result.clicked_slot.is_none());
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
        use crate::domain::character::{Alignment, Character, Sex};

        let panel_names: Vec<(usize, String)> = vec![(0, "Hero".to_string())];
        let character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let actions = build_action_list(0, 0, &panel_names, &character, None);
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            actions[0],
            PanelAction::Drop { party_index: 0, .. }
        ));
    }

    /// `build_action_list` with two other panels returns Drop + two Transfer actions.
    #[test]
    fn test_build_action_list_drop_and_transfers() {
        use crate::domain::character::{Alignment, Character, Sex};

        let panel_names: Vec<(usize, String)> = vec![
            (0, "Hero".to_string()),
            (1, "Ally".to_string()),
            (2, "Mage".to_string()),
        ];
        let character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let actions = build_action_list(0, 0, &panel_names, &character, None);
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
        use crate::domain::character::{Alignment, Character, Sex};

        let panel_names: Vec<(usize, String)> = vec![(0, "A".to_string()), (1, "B".to_string())];
        let character = Character::new(
            "A".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let actions = build_action_list(1, 0, &panel_names, &character, None);
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
            selected_equip_slot: None,
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
        app.add_message::<EquipItemAction>();
        app.add_message::<UnequipItemAction>();

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
        app.add_message::<EquipItemAction>();
        app.add_message::<UnequipItemAction>();

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

    // ------------------------------------------------------------------
    // handle_use_item_action_exploration tests
    // ------------------------------------------------------------------

    /// Helper to build a minimal ContentDatabase with a single healing potion.
    fn make_heal_potion_db() -> crate::sdk::database::ContentDatabase {
        use crate::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
        use crate::sdk::database::ContentDatabase;

        let mut db = ContentDatabase::new();
        let item = Item {
            id: 1,
            name: "Healing Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(50),
                is_combat_usable: true,
                duration_minutes: None,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        db.items.add_item(item).unwrap();
        db
    }

    /// Helper to build a ContentDatabase with an SP potion.
    fn make_sp_potion_db() -> crate::sdk::database::ContentDatabase {
        use crate::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
        use crate::sdk::database::ContentDatabase;

        let mut db = ContentDatabase::new();
        let item = Item {
            id: 2,
            name: "Mana Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::RestoreSp(30),
                is_combat_usable: true,
                duration_minutes: None,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        db.items.add_item(item).unwrap();
        db
    }

    /// Sets up an `App` with the minimal plugins, `GlobalState` in Inventory
    /// mode, `InventoryNavigationState`, `GameLog`, `GameContent`, and the
    /// `handle_use_item_action_exploration` system.
    fn make_exploration_use_app(
        game_state: crate::application::GameState,
        content_db: crate::sdk::database::ContentDatabase,
    ) -> App {
        use crate::application::resources::GameContent;
        use crate::game::systems::ui::UiPlugin;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);
        app.add_message::<UseItemExplorationAction>();
        app.insert_resource(GameContent::new(content_db));
        app.insert_resource(GlobalState(game_state));
        app.init_resource::<InventoryNavigationState>();
        app.add_systems(Update, handle_use_item_action_exploration);
        app
    }

    /// A [`UseItemExplorationAction`] for a healing potion increases
    /// `character.hp.current`, removes the slot from inventory (last charge),
    /// and writes a "recovered" message to `GameLog`.
    #[test]
    fn test_exploration_use_heals_character() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.hp.base = 100;
        ch.hp.current = 40;
        ch.inventory.items.push(InventorySlot {
            item_id: 1,
            charges: 1,
        });
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = make_exploration_use_app(game_state, make_heal_potion_db());
        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            gs.0.party.members[0].hp.current > 40,
            "HP should have increased"
        );
        assert_eq!(
            gs.0.party.members[0].inventory.items.len(),
            0,
            "last-charge slot should be removed"
        );

        let log = app.world().resource::<GameLog>();
        assert!(
            log.entries()
                .iter()
                .any(|entry| entry.text.contains("recovered")),
            "GameLog should contain 'recovered'"
        );
    }

    /// A [`UseItemExplorationAction`] for an SP potion increases
    /// `character.sp.current` and writes a message containing "SP".
    #[test]
    fn test_exploration_use_restores_sp() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Wizard".to_string(),
            "human".to_string(),
            "sorcerer".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        ch.sp.base = 50;
        ch.sp.current = 10;
        ch.inventory.items.push(InventorySlot {
            item_id: 2,
            charges: 1,
        });
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = make_exploration_use_app(game_state, make_sp_potion_db());
        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            gs.0.party.members[0].sp.current > 10,
            "SP should have increased"
        );

        let log = app.world().resource::<GameLog>();
        assert!(
            log.entries().iter().any(|entry| entry.text.contains("SP")),
            "GameLog should contain 'SP'"
        );
    }

    /// A [`UseItemExplorationAction`] for a cure potion clears the matching
    /// condition bits and writes "Conditions cleared" to `GameLog`.
    #[test]
    fn test_exploration_use_cures_condition() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, Condition, InventorySlot, Sex};
        use crate::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
        use crate::sdk::database::ContentDatabase;

        let mut db = ContentDatabase::new();
        let item = Item {
            id: 3,
            name: "Cure Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::CureCondition(Condition::POISONED),
                is_combat_usable: true,
                duration_minutes: None,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        db.items.add_item(item).unwrap();

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Sick Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.conditions.add(Condition::POISONED);
        ch.inventory.items.push(InventorySlot {
            item_id: 3,
            charges: 1,
        });
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(crate::game::systems::ui::UiPlugin);
        app.add_message::<UseItemExplorationAction>();
        app.insert_resource(GameContent::new(db));
        app.insert_resource(GlobalState(game_state));
        app.init_resource::<InventoryNavigationState>();
        app.add_systems(Update, handle_use_item_action_exploration);

        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            !gs.0.party.members[0].conditions.has(Condition::POISONED),
            "POISONED condition should be cleared"
        );

        let log = app.world().resource::<GameLog>();
        assert!(
            log.entries()
                .iter()
                .any(|entry| entry.text.contains("Conditions cleared")),
            "GameLog should contain 'Conditions cleared'"
        );
    }

    /// A [`UseItemExplorationAction`] for a `BoostAttribute` potion increases
    /// the corresponding `stats.<attr>.current` field.
    #[test]
    fn test_exploration_use_boosts_attribute() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::items::types::{
            AttributeType, ConsumableData, ConsumableEffect, Item, ItemType,
        };
        use crate::sdk::database::ContentDatabase;

        let mut db = ContentDatabase::new();
        let item = Item {
            id: 4,
            name: "Might Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::BoostAttribute(AttributeType::Might, 5),
                is_combat_usable: true,
                duration_minutes: None,
            }),
            base_cost: 20,
            sell_cost: 10,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        db.items.add_item(item).unwrap();

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Warrior".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let before_might = ch.stats.might.current;
        ch.inventory.items.push(InventorySlot {
            item_id: 4,
            charges: 1,
        });
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(crate::game::systems::ui::UiPlugin);
        app.add_message::<UseItemExplorationAction>();
        app.insert_resource(GameContent::new(db));
        app.insert_resource(GlobalState(game_state));
        app.init_resource::<InventoryNavigationState>();
        app.add_systems(Update, handle_use_item_action_exploration);

        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            gs.0.party.members[0].stats.might.current > before_might,
            "Might should have increased after using Might Potion"
        );
    }

    /// A [`UseItemExplorationAction`] for a `BoostResistance` potion increases
    /// the corresponding resistance field on the character.
    #[test]
    fn test_exploration_use_boosts_resistance() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::items::types::{
            ConsumableData, ConsumableEffect, Item, ItemType, ResistanceType,
        };
        use crate::sdk::database::ContentDatabase;

        let mut db = ContentDatabase::new();
        let item = Item {
            id: 5,
            name: "Fire Resist Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::BoostResistance(ResistanceType::Fire, 10),
                is_combat_usable: true,
                duration_minutes: None,
            }),
            base_cost: 20,
            sell_cost: 10,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        db.items.add_item(item).unwrap();

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Ranger".to_string(),
            "human".to_string(),
            "robber".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        let before_fire = ch.resistances.fire.current;
        ch.inventory.items.push(InventorySlot {
            item_id: 5,
            charges: 1,
        });
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(crate::game::systems::ui::UiPlugin);
        app.add_message::<UseItemExplorationAction>();
        app.insert_resource(GameContent::new(db));
        app.insert_resource(GlobalState(game_state));
        app.init_resource::<InventoryNavigationState>();
        app.add_systems(Update, handle_use_item_action_exploration);

        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            gs.0.party.members[0].resistances.fire.current > before_fire,
            "Fire resistance should have increased"
        );
    }

    /// An item with `charges = 3` should have `charges == 2` after one use;
    /// the inventory slot must still be present.
    #[test]
    fn test_exploration_use_decrements_multi_charge_item() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.hp.base = 100;
        ch.hp.current = 10;
        ch.inventory.items.push(InventorySlot {
            item_id: 1,
            charges: 3,
        });
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = make_exploration_use_app(game_state, make_heal_potion_db());
        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.party.members[0].inventory.items.len(),
            1,
            "slot should still be present after decrement"
        );
        assert_eq!(
            gs.0.party.members[0].inventory.items[0].charges, 2,
            "charges should be decremented from 3 to 2"
        );
    }

    /// An item with `charges = 1` should have its slot removed entirely after use.
    #[test]
    fn test_exploration_use_removes_last_charge() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.hp.base = 100;
        ch.hp.current = 10;
        ch.inventory.items.push(InventorySlot {
            item_id: 1,
            charges: 1,
        });
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = make_exploration_use_app(game_state, make_heal_potion_db());
        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.party.members[0].inventory.items.len(),
            0,
            "slot should be removed after last charge consumed"
        );
    }

    /// After a successful use, `nav_state.phase` must be `SlotNavigation`,
    /// `selected_slot_index` must be `None`, and `focused_action_index` must be 0.
    #[test]
    fn test_exploration_use_resets_nav_state() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.hp.base = 100;
        ch.hp.current = 10;
        ch.inventory.items.push(InventorySlot {
            item_id: 1,
            charges: 1,
        });
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = make_exploration_use_app(game_state, make_heal_potion_db());

        // Pre-set nav_state to ActionNavigation to confirm it is fully reset.
        {
            let mut nav = app.world_mut().resource_mut::<InventoryNavigationState>();
            nav.selected_slot_index = Some(0);
            nav.focused_action_index = 1;
            nav.phase = NavigationPhase::ActionNavigation;
        }

        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();

        let nav = app.world().resource::<InventoryNavigationState>();
        assert!(
            matches!(nav.phase, NavigationPhase::SlotNavigation),
            "phase should be reset to SlotNavigation"
        );
        assert_eq!(nav.selected_slot_index, None);
        assert_eq!(nav.focused_action_index, 0);
    }

    /// A successful use appends exactly one entry to `GameLog.messages`.
    #[test]
    fn test_exploration_use_writes_game_log() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.hp.base = 100;
        ch.hp.current = 10;
        ch.inventory.items.push(InventorySlot {
            item_id: 1,
            charges: 1,
        });
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = make_exploration_use_app(game_state, make_heal_potion_db());
        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();
        app.update();

        let log = app.world().resource::<GameLog>();
        assert_eq!(
            log.entries().len(),
            1,
            "exactly one GameLog entry should be written on success"
        );
    }

    /// A `slot_index` beyond inventory length writes "no item in that slot"
    /// to `GameLog`.
    #[test]
    fn test_exploration_use_invalid_slot_writes_log() {
        use crate::domain::character::{Alignment, Character, Sex};

        let mut game_state = GameState::new();
        let ch = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = make_exploration_use_app(game_state, make_heal_potion_db());
        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 99, // beyond inventory
        });
        app.update();
        app.update();

        let log = app.world().resource::<GameLog>();
        assert!(
            log.entries()
                .iter()
                .any(|entry| entry.text.contains("no item in that slot")),
            "GameLog should say 'no item in that slot'"
        );
    }

    /// A non-consumable item (e.g. a weapon) in the target slot writes
    /// "not a consumable" to `GameLog`.
    #[test]
    fn test_exploration_use_non_consumable_writes_log() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::items::types::{Item, ItemType, WeaponClassification, WeaponData};
        use crate::domain::types::DiceRoll;
        use crate::sdk::database::ContentDatabase;

        let mut db = ContentDatabase::new();
        let weapon = Item {
            id: 10,
            name: "Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 50,
            sell_cost: 25,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        db.items.add_item(weapon).unwrap();

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Warrior".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.inventory.items.push(InventorySlot {
            item_id: 10,
            charges: 0,
        });
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(crate::game::systems::ui::UiPlugin);
        app.add_message::<UseItemExplorationAction>();
        app.insert_resource(GameContent::new(db));
        app.insert_resource(GlobalState(game_state));
        app.init_resource::<InventoryNavigationState>();
        app.add_systems(Update, handle_use_item_action_exploration);

        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();
        app.update();

        let log = app.world().resource::<GameLog>();
        assert!(
            log.entries()
                .iter()
                .any(|entry| entry.text.contains("not a consumable")),
            "GameLog should say 'not a consumable'"
        );
    }

    /// An item with `charges = 0` writes "no charges" to `GameLog` and does
    /// not apply any effect.
    #[test]
    fn test_exploration_use_zero_charges_writes_log() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.inventory.items.push(InventorySlot {
            item_id: 1,
            charges: 0,
        });
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = make_exploration_use_app(game_state, make_heal_potion_db());
        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();
        app.update();

        let log = app.world().resource::<GameLog>();
        assert!(
            log.entries()
                .iter()
                .any(|entry| entry.text.contains("no charges")),
            "GameLog should say 'no charges'"
        );
    }

    /// An item with `is_combat_usable: false` can be used outside of combat;
    /// the effect is applied and the item name appears in `GameLog`.
    #[test]
    fn test_exploration_use_non_combat_usable_item_succeeds() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
        use crate::sdk::database::ContentDatabase;

        let mut db = ContentDatabase::new();
        let item = Item {
            id: 6,
            name: "Exploration Tonic".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(20),
                is_combat_usable: false, // only usable outside combat
                duration_minutes: None,
            }),
            base_cost: 15,
            sell_cost: 7,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        db.items.add_item(item).unwrap();

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Explorer".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.hp.base = 100;
        ch.hp.current = 50;
        ch.inventory.items.push(InventorySlot {
            item_id: 6,
            charges: 1,
        });
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(crate::game::systems::ui::UiPlugin);
        app.add_message::<UseItemExplorationAction>();
        app.insert_resource(GameContent::new(db));
        app.insert_resource(GlobalState(game_state));
        app.init_resource::<InventoryNavigationState>();
        app.add_systems(Update, handle_use_item_action_exploration);

        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            gs.0.party.members[0].hp.current > 50,
            "effect should have been applied even though is_combat_usable is false"
        );

        let log = app.world().resource::<GameLog>();
        assert!(
            log.entries()
                .iter()
                .any(|entry| entry.text.contains("Exploration Tonic")),
            "GameLog should contain the item name"
        );
    }

    /// A `party_index` beyond the party size writes "invalid character" to
    /// `GameLog` and does not panic.
    #[test]
    fn test_exploration_use_invalid_party_index_writes_log() {
        let mut game_state = GameState::new();
        // Party is empty — any party_index is out of bounds.
        game_state.enter_inventory();

        let mut app = make_exploration_use_app(game_state, make_heal_potion_db());
        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 99,
            slot_index: 0,
        });
        app.update();
        app.update();

        let log = app.world().resource::<GameLog>();
        assert!(
            log.entries()
                .iter()
                .any(|entry| entry.text.contains("invalid character")),
            "GameLog should say 'invalid character'"
        );
    }

    // ------------------------------------------------------------------
    // build_action_list with consumable/non-consumable
    // ------------------------------------------------------------------

    /// `build_action_list` for a slot containing a consumable returns `Use` as
    /// the first action, before `Drop`.
    #[test]
    fn test_build_action_list_use_first_for_consumable() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
        use crate::sdk::database::ContentDatabase;

        // Build a ContentDatabase with one consumable item
        let mut content_db = ContentDatabase::new();
        let item = Item {
            id: 1,
            name: "Healing Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(20),
                is_combat_usable: true,
                duration_minutes: None,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        content_db.items.add_item(item).unwrap();
        let game_content = GameContent::new(content_db);

        let mut character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        character.inventory.items.push(InventorySlot {
            item_id: 1,
            charges: 1,
        });

        let panel_names: Vec<(usize, String)> = vec![(0, "Hero".to_string())];
        let actions = build_action_list(0, 0, &panel_names, &character, Some(&game_content));

        // Use must be first
        assert!(
            matches!(actions[0], PanelAction::Use { party_index: 0, .. }),
            "first action should be Use for a consumable slot"
        );
        // Drop must be second
        assert!(
            matches!(actions[1], PanelAction::Drop { party_index: 0, .. }),
            "second action should be Drop"
        );
        assert_eq!(
            actions.len(),
            2,
            "should have exactly Use + Drop with no other panels"
        );
    }

    /// `build_action_list` for a non-consumable slot (e.g., a weapon) returns only
    /// `Drop` and `Transfer` — no `Use` action.
    #[test]
    fn test_build_action_list_no_use_for_non_consumable() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::items::types::{Item, ItemType, WeaponClassification, WeaponData};
        use crate::domain::types::DiceRoll;
        use crate::sdk::database::ContentDatabase;

        let mut content_db = ContentDatabase::new();
        let weapon = Item {
            id: 2,
            name: "Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 50,
            sell_cost: 25,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        content_db.items.add_item(weapon).unwrap();
        let game_content = GameContent::new(content_db);

        let mut character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        character.inventory.items.push(InventorySlot {
            item_id: 2,
            charges: 0,
        });

        let panel_names: Vec<(usize, String)> = vec![(0, "Hero".to_string())];
        let actions = build_action_list(0, 0, &panel_names, &character, Some(&game_content));

        // No Use action for a weapon
        assert!(
            actions
                .iter()
                .all(|a| !matches!(a, PanelAction::Use { .. })),
            "no Use action should appear for a non-consumable slot"
        );
        // Weapons are equipable, so Equip is the first action
        assert!(
            matches!(actions[0], PanelAction::Equip { party_index: 0, .. }),
            "first action should be Equip for an equipable non-consumable slot"
        );
        // Drop is the second action
        assert!(
            matches!(actions[1], PanelAction::Drop { party_index: 0, .. }),
            "second action should be Drop for a non-consumable slot"
        );
    }

    /// `build_action_list` with `game_content = None` must never return a `Use`
    /// action (cannot determine item type without content DB).
    #[test]
    fn test_build_action_list_no_use_when_no_content() {
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};

        let mut character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        character.inventory.items.push(InventorySlot {
            item_id: 99,
            charges: 1,
        });

        let panel_names: Vec<(usize, String)> = vec![(0, "Hero".to_string())];
        let actions = build_action_list(0, 0, &panel_names, &character, None);

        assert!(
            actions
                .iter()
                .all(|a| !matches!(a, PanelAction::Use { .. })),
            "no Use action should appear when game_content is None"
        );
        assert!(matches!(actions[0], PanelAction::Drop { .. }));
    }

    /// `PanelAction::Use` carries `party_index` and `slot_index` correctly and
    /// implements `Debug` and `PartialEq`.
    #[test]
    fn test_panel_action_use_variant() {
        let a = PanelAction::Use {
            party_index: 0,
            slot_index: 2,
        };
        let b = PanelAction::Use {
            party_index: 0,
            slot_index: 2,
        };
        assert_eq!(a, b, "identical Use variants must be equal");

        let debug_str = format!("{:?}", a);
        assert!(
            debug_str.contains("Use"),
            "Debug output should contain 'Use'"
        );
        assert!(
            debug_str.contains("party_index"),
            "Debug output should contain 'party_index'"
        );
        assert!(
            debug_str.contains("slot_index"),
            "Debug output should contain 'slot_index'"
        );

        // Round-trip through matching
        match a {
            PanelAction::Use {
                party_index,
                slot_index,
            } => {
                assert_eq!(party_index, 0);
                assert_eq!(slot_index, 2);
            }
            _ => panic!("expected Use variant"),
        }
    }

    /// Drop and Transfer actions remain present and unaffected after the
    /// `build_action_list` signature change.
    #[test]
    fn test_build_action_list_drop_transfer_unchanged() {
        use crate::domain::character::{Alignment, Character, Sex};

        let character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // No items in inventory → slot 0 is empty → no Use action
        let panel_names: Vec<(usize, String)> =
            vec![(0, "Hero".to_string()), (1, "Ally".to_string())];
        let actions = build_action_list(0, 0, &panel_names, &character, None);

        // Drop + Transfer→1
        assert_eq!(actions.len(), 2);
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
    }

    // ====================================================================
    // Equip / Unequip tests
    // ====================================================================

    // ------------------------------------------------------------------
    // 4.5.1  E key dispatches EquipItemAction
    // ------------------------------------------------------------------

    /// Pressing **E** while an equipable item is selected in `SlotNavigation`
    /// phase dispatches `EquipItemAction { party_index: 0, slot_index: 0 }`.
    #[test]
    fn test_equip_action_dispatched_on_e_key() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::items::types::{Item, ItemType, WeaponClassification, WeaponData};
        use crate::domain::types::DiceRoll;
        use crate::sdk::database::ContentDatabase;

        let mut content_db = ContentDatabase::new();
        let weapon = Item {
            id: 1,
            name: "Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 50,
            sell_cost: 25,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        content_db.items.add_item(weapon).unwrap();

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.inventory.items.push(InventorySlot {
            item_id: 1,
            charges: 0,
        });
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Register ButtonInput<KeyCode> manually (without InputPlugin) so that
        // just_pressed is NOT cleared by InputPlugin's clear system before
        // inventory_input_system runs.
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_message::<DropItemAction>();
        app.add_message::<TransferItemAction>();
        app.add_message::<UseItemExplorationAction>();
        app.add_message::<EquipItemAction>();
        app.add_message::<UnequipItemAction>();
        app.insert_resource(GameContent::new(content_db));
        app.insert_resource(GlobalState(game_state));
        let nav = InventoryNavigationState {
            selected_slot_index: Some(0),
            ..Default::default()
        };
        app.insert_resource(nav);
        app.add_systems(Update, inventory_input_system);

        // Press E before update so just_pressed(KeyE) is true during the frame.
        // Without InputPlugin there is no clear-system to wipe just_pressed
        // before inventory_input_system runs.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyE);
        app.update();

        // When E dispatches EquipItemAction, the input system also clears
        // selected_slot_index to None.  That side-effect proves the message
        // was dispatched (it only happens inside the `if is_equipable` branch).
        let nav = app.world().resource::<InventoryNavigationState>();
        assert_eq!(
            nav.selected_slot_index, None,
            "selected_slot_index must be cleared after E dispatches EquipItemAction"
        );
        assert!(
            matches!(nav.phase, NavigationPhase::SlotNavigation),
            "phase must remain SlotNavigation after E-key equip dispatch"
        );
    }

    // ------------------------------------------------------------------
    // 4.5.2  Equip button absent for consumable
    // ------------------------------------------------------------------

    /// `build_action_list` for a slot containing a **consumable** must NOT
    /// include an `Equip` action.
    #[test]
    fn test_equip_button_absent_for_consumable() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
        use crate::sdk::database::ContentDatabase;

        let mut content_db = ContentDatabase::new();
        let potion = Item {
            id: 1,
            name: "Healing Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(20),
                is_combat_usable: true,
                duration_minutes: None,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        content_db.items.add_item(potion).unwrap();
        let game_content = GameContent::new(content_db);

        let mut character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        character.inventory.items.push(InventorySlot {
            item_id: 1,
            charges: 1,
        });

        let panel_names: Vec<(usize, String)> = vec![(0, "Hero".to_string())];
        let actions = build_action_list(0, 0, &panel_names, &character, Some(&game_content));

        let has_equip = actions
            .iter()
            .any(|a| matches!(a, PanelAction::Equip { .. }));
        assert!(!has_equip, "Equip should not appear for a consumable");
    }

    // ------------------------------------------------------------------
    // 4.5.3  Equip button present for weapon and is first
    // ------------------------------------------------------------------

    /// `build_action_list` for a slot containing a **weapon** returns `Equip`
    /// as the first action.
    #[test]
    fn test_equip_button_present_for_weapon() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::items::types::{Item, ItemType, WeaponClassification, WeaponData};
        use crate::domain::types::DiceRoll;
        use crate::sdk::database::ContentDatabase;

        let mut content_db = ContentDatabase::new();
        let weapon = Item {
            id: 2,
            name: "Longsword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 50,
            sell_cost: 25,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        content_db.items.add_item(weapon).unwrap();
        let game_content = GameContent::new(content_db);

        let mut character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        character.inventory.items.push(InventorySlot {
            item_id: 2,
            charges: 0,
        });

        let panel_names: Vec<(usize, String)> = vec![(0, "Hero".to_string())];
        let actions = build_action_list(0, 0, &panel_names, &character, Some(&game_content));

        assert!(
            matches!(actions[0], PanelAction::Equip { party_index: 0, .. }),
            "Equip should be the first action for a weapon"
        );
    }

    // ------------------------------------------------------------------
    // 4.5.4  EquipItemAction moves item from inventory into equipment slot
    // ------------------------------------------------------------------

    /// Sending `EquipItemAction` moves the weapon from `inventory.items` into
    /// `equipment.weapon` and clears `selected_slot_index`.
    #[test]
    fn test_equip_action_system_moves_item_to_slot() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::classes::ClassDefinition;
        use crate::domain::items::types::{Item, ItemType, WeaponClassification, WeaponData};
        use crate::domain::races::RaceDefinition;
        use crate::domain::types::DiceRoll;
        use crate::sdk::database::ContentDatabase;

        let mut content_db = ContentDatabase::new();

        // Add a weapon to the item database
        let weapon = Item {
            id: 1,
            name: "Iron Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 50,
            sell_cost: 25,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        content_db.items.add_item(weapon).unwrap();

        // Add a knight class with martial_melee proficiency
        content_db
            .classes
            .add_class(ClassDefinition {
                id: "knight".to_string(),
                name: "Knight".to_string(),
                description: String::new(),
                hp_die: DiceRoll::new(1, 10, 0),
                spell_school: None,
                is_pure_caster: false,
                spell_stat: None,
                special_abilities: vec![],
                starting_weapon_id: None,
                starting_armor_id: None,
                starting_items: vec![],
                proficiencies: vec!["martial_melee".to_string()],
            })
            .unwrap();

        // Add a human race (no restrictions)
        content_db
            .races
            .add_race(RaceDefinition::new(
                "human".to_string(),
                "Human".to_string(),
                "A versatile race".to_string(),
            ))
            .unwrap();

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Knight".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.inventory.items.push(InventorySlot {
            item_id: 1,
            charges: 0,
        });
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<DropItemAction>();
        app.add_message::<TransferItemAction>();
        app.add_message::<EquipItemAction>();
        app.add_message::<UnequipItemAction>();
        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(content_db));
        app.init_resource::<InventoryNavigationState>();
        app.init_resource::<GameLog>();
        app.add_systems(Update, inventory_action_system);

        app.world_mut().write_message(EquipItemAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.party.members[0].equipment.weapon,
            Some(1),
            "weapon slot should hold item_id=1"
        );
        assert_eq!(
            gs.0.party.members[0].inventory.items.len(),
            0,
            "inventory should be empty after equipping"
        );

        let nav = app.world().resource::<InventoryNavigationState>();
        assert_eq!(
            nav.selected_slot_index, None,
            "selected_slot_index should be cleared"
        );
    }

    // ------------------------------------------------------------------
    // 4.5.5  UnequipItemAction returns item to inventory
    // ------------------------------------------------------------------

    /// Sending `UnequipItemAction` clears `equipment.weapon` and adds the
    /// item back to `inventory.items`.
    #[test]
    fn test_unequip_action_system_returns_item_to_inventory() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, EquipmentSlot, Sex};
        use crate::domain::items::types::{Item, ItemType, WeaponClassification, WeaponData};
        use crate::domain::types::DiceRoll;
        use crate::sdk::database::ContentDatabase;

        let mut content_db = ContentDatabase::new();
        let weapon = Item {
            id: 1,
            name: "Shortsword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 6, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 30,
            sell_cost: 15,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        content_db.items.add_item(weapon).unwrap();

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Place weapon directly in equipment slot
        ch.equipment.weapon = Some(1);
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<DropItemAction>();
        app.add_message::<TransferItemAction>();
        app.add_message::<EquipItemAction>();
        app.add_message::<UnequipItemAction>();
        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(content_db));
        app.init_resource::<InventoryNavigationState>();
        app.init_resource::<GameLog>();
        app.add_systems(Update, inventory_action_system);

        app.world_mut().write_message(UnequipItemAction {
            party_index: 0,
            slot: EquipmentSlot::Weapon,
        });
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.party.members[0].equipment.weapon, None,
            "weapon slot should be cleared"
        );
        assert_eq!(
            gs.0.party.members[0].inventory.items.len(),
            1,
            "item should return to inventory"
        );
        assert_eq!(
            gs.0.party.members[0].inventory.items[0].item_id, 1,
            "returned item should be item_id=1"
        );
    }

    // ------------------------------------------------------------------
    // 4.5.6  UnequipItemAction with full inventory logs "inventory is full"
    // ------------------------------------------------------------------

    /// When the character's inventory is full, `UnequipItemAction` must write
    /// "inventory is full" to `GameLog` and leave the equipment slot unchanged.
    #[test]
    fn test_unequip_action_system_inventory_full_logs_error() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, EquipmentSlot, InventorySlot, Sex};
        use crate::domain::items::types::{Item, ItemType, WeaponClassification, WeaponData};
        use crate::domain::types::{DiceRoll, ItemId};
        use crate::sdk::database::ContentDatabase;

        let mut content_db = ContentDatabase::new();
        let weapon = Item {
            id: 1,
            name: "Great Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(2, 6, 0),
                bonus: 0,
                hands_required: 2,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 100,
            sell_cost: 50,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        content_db.items.add_item(weapon).unwrap();

        let mut game_state = GameState::new();
        let mut ch = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Fill inventory completely
        for i in 0..Inventory::MAX_ITEMS {
            ch.inventory.items.push(InventorySlot {
                item_id: (i + 10) as ItemId,
                charges: 0,
            });
        }
        ch.equipment.weapon = Some(1);
        game_state.party.add_member(ch).unwrap();
        game_state.enter_inventory();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(crate::game::systems::ui::UiPlugin);
        app.add_message::<DropItemAction>();
        app.add_message::<TransferItemAction>();
        app.add_message::<EquipItemAction>();
        app.add_message::<UnequipItemAction>();
        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(content_db));
        app.init_resource::<InventoryNavigationState>();
        app.add_systems(Update, inventory_action_system);

        app.world_mut().write_message(UnequipItemAction {
            party_index: 0,
            slot: EquipmentSlot::Weapon,
        });
        app.update();
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.party.members[0].equipment.weapon,
            Some(1),
            "equipment slot must be unchanged when inventory is full"
        );

        let log = app.world().resource::<GameLog>();
        assert!(
            log.entries()
                .iter()
                .any(|entry| entry.text.contains("inventory is full")),
            "GameLog should contain 'inventory is full'"
        );
    }

    // ------------------------------------------------------------------
    // 4.5.7  Equipment strip renders equipped item name without panic
    // ------------------------------------------------------------------

    /// `render_character_panel` with a weapon equipped must complete without
    /// panicking, and the weapon's `item_id` must be accessible via the
    /// character's equipment struct (proving the strip would display it).
    #[test]
    fn test_equipment_strip_shows_equipped_item_name() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, EquipmentSlot, Sex};
        use crate::domain::items::types::{Item, ItemType, WeaponClassification, WeaponData};
        use crate::domain::types::DiceRoll;
        use crate::sdk::database::ContentDatabase;

        let mut content_db = ContentDatabase::new();
        let sword = Item {
            id: 42,
            name: "Flame Blade".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 6, 0),
                bonus: 1,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 100,
            sell_cost: 50,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        content_db.items.add_item(sword).unwrap();
        let game_content = GameContent::new(content_db);

        let mut global_state = GlobalState(GameState::new());
        let mut hero = Character::new(
            "Warrior".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Directly set the weapon slot to item_id=42
        hero.equipment.weapon = Some(42);
        global_state.0.party.add_member(hero).unwrap();

        // Verify the weapon is accessible via EquipmentSlot::Weapon
        assert_eq!(
            EquipmentSlot::Weapon.get(&global_state.0.party.members[0].equipment),
            Some(42),
            "weapon slot should hold item_id=42"
        );
        let equipped_name = game_content
            .db()
            .items
            .get_item(42)
            .map(|i| i.name.as_str())
            .unwrap_or("");
        assert_eq!(
            equipped_name, "Flame Blade",
            "item name should be Flame Blade"
        );

        // Render the panel with the equipment strip cell focused — must not panic
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let result = render_character_panel(
                    ui,
                    0,
                    true,
                    None,
                    None,
                    egui::vec2(400.0, 500.0),
                    &global_state,
                    Some(&game_content),
                    &[],
                    Some(EquipmentSlot::Weapon),
                );
                assert!(result.action.is_none());
                assert!(result.clicked_slot.is_none());
            });
        });
        // No panic = the equipped item name was rendered in the strip
    }

    // ── 7.1  Exploration scroll dispatch (CastSpell / LearnSpell) ───────────

    fn make_cast_spell_scroll_db() -> crate::sdk::database::ContentDatabase {
        use crate::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
        use crate::domain::magic::types::{
            Spell, SpellContext, SpellEffectType, SpellSchool, SpellTarget,
        };
        use crate::domain::types::DiceRoll;
        use crate::sdk::database::ContentDatabase;

        let mut db = ContentDatabase::new();

        // CastSpell scroll — casts spell 0x0101 (First Aid).
        let scroll = Item {
            id: 1,
            name: "Healing Scroll".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::CastSpell(0x0101),
                is_combat_usable: false,
                duration_minutes: None,
            }),
            base_cost: 50,
            sell_cost: 25,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        db.items.add_item(scroll).unwrap();

        // The spell that the scroll casts.
        let mut spell = Spell::new(
            0x0101,
            "First Aid",
            SpellSchool::Cleric,
            1,
            2,
            0,
            SpellContext::Anytime,
            SpellTarget::Self_,
            "Minor healing",
            None,
            0,
            false,
        );
        spell.effect_type = Some(SpellEffectType::Healing {
            amount: DiceRoll::new(1, 6, 0),
        });
        db.spells.add_spell(spell).unwrap();

        db
    }

    fn make_learn_spell_scroll_db() -> crate::sdk::database::ContentDatabase {
        use crate::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
        use crate::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};
        use crate::sdk::database::ContentDatabase;

        let mut db = ContentDatabase::new();

        // LearnSpell scroll — permanently teaches spell 0x0101.
        let scroll = Item {
            id: 2,
            name: "Learning Scroll".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::LearnSpell(0x0101),
                is_combat_usable: false,
                duration_minutes: None,
            }),
            base_cost: 100,
            sell_cost: 50,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        db.items.add_item(scroll).unwrap();

        let spell = Spell::new(
            0x0101,
            "First Aid",
            SpellSchool::Cleric,
            1,
            2,
            0,
            SpellContext::Anytime,
            SpellTarget::Self_,
            "Minor healing",
            None,
            0,
            false,
        );
        db.spells.add_spell(spell).unwrap();

        db
    }

    /// Using a `CastSpell` scroll whose spell ID is NOT in the database does
    /// not panic and writes a log entry containing the item name.
    #[test]
    fn test_cast_spell_scroll_unknown_spell_id_no_panic() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
        use crate::sdk::database::ContentDatabase;

        let mut db = ContentDatabase::new();
        let scroll = Item {
            id: 1,
            name: "Mystery Scroll".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::CastSpell(9999), // not in DB
                is_combat_usable: false,
                duration_minutes: None,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        db.items.add_item(scroll).unwrap();

        let mut gs = GameState::new();
        let mut hero = Character::new(
            "Mage".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        hero.inventory.items.push(InventorySlot {
            item_id: 1,
            charges: 1,
        });
        gs.party.add_member(hero).unwrap();
        gs.enter_inventory();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(crate::game::systems::ui::UiPlugin);
        app.add_message::<UseItemExplorationAction>();
        app.insert_resource(GameContent::new(db));
        app.insert_resource(GlobalState(gs));
        app.init_resource::<InventoryNavigationState>();
        app.add_systems(Update, handle_use_item_action_exploration);

        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();
        app.update();

        let log = app.world().resource::<GameLog>();
        assert!(
            log.entries()
                .iter()
                .any(|e| e.text.contains("Mystery Scroll")),
            "log must mention the scroll name even when spell ID is unknown"
        );
        // Item must be consumed (slot removed)
        let gs_after = app.world().resource::<GlobalState>();
        assert!(
            gs_after.0.party.members[0].inventory.items.is_empty(),
            "scroll slot must be consumed even when spell is unknown"
        );
    }

    /// Using a `LearnSpell` scroll whose spell ID is NOT in the database does
    /// not panic and writes a log entry containing "could not learn".
    #[test]
    fn test_learn_spell_scroll_unknown_spell_id_no_panic() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};
        use crate::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
        use crate::sdk::database::ContentDatabase;

        let mut db = ContentDatabase::new();
        let scroll = Item {
            id: 1,
            name: "Cryptic Scroll".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::LearnSpell(9999), // not in DB
                is_combat_usable: false,
                duration_minutes: None,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        db.items.add_item(scroll).unwrap();

        let mut gs = GameState::new();
        let mut hero = Character::new(
            "Scholar".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.inventory.items.push(InventorySlot {
            item_id: 1,
            charges: 1,
        });
        gs.party.add_member(hero).unwrap();
        gs.enter_inventory();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(crate::game::systems::ui::UiPlugin);
        app.add_message::<UseItemExplorationAction>();
        app.insert_resource(GameContent::new(db));
        app.insert_resource(GlobalState(gs));
        app.init_resource::<InventoryNavigationState>();
        app.add_systems(Update, handle_use_item_action_exploration);

        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();
        app.update();

        let log = app.world().resource::<GameLog>();
        assert!(
            log.entries()
                .iter()
                .any(|e| e.text.contains("could not learn")),
            "log must report failure to learn when spell ID is unknown"
        );
        // Scroll is consumed regardless of failure.
        let gs_after = app.world().resource::<GlobalState>();
        assert!(
            gs_after.0.party.members[0].inventory.items.is_empty(),
            "scroll slot must be consumed even when learning fails"
        );
    }

    /// Using a `CastSpell` scroll whose spell IS in the database writes the
    /// resolved spell NAME to the GameLog (not just the raw numeric ID).
    #[test]
    fn test_cast_spell_scroll_logs_spell_name_on_failure() {
        // Character has 0 SP so the cast fails, but the log must still name
        // the spell rather than showing "Casting spell 257".
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};

        let db = make_cast_spell_scroll_db();

        let mut gs = GameState::new();
        let mut hero = Character::new(
            "Caster".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        hero.hp.base = 30;
        hero.hp.current = 10;
        hero.sp.base = 10;
        hero.sp.current = 0; // not enough SP — cast will fail
        hero.level = 3;
        hero.inventory.items.push(InventorySlot {
            item_id: 1,
            charges: 1,
        });
        gs.party.add_member(hero).unwrap();
        gs.enter_inventory();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(crate::game::systems::ui::UiPlugin);
        app.add_message::<UseItemExplorationAction>();
        app.insert_resource(GameContent::new(db));
        app.insert_resource(GlobalState(gs));
        app.init_resource::<InventoryNavigationState>();
        app.add_systems(Update, handle_use_item_action_exploration);

        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();
        app.update();

        let log = app.world().resource::<GameLog>();
        let has_spell_name = log.entries().iter().any(|e| e.text.contains("First Aid"));
        assert!(
            has_spell_name,
            "GameLog must contain the resolved spell name 'First Aid'; entries: {:?}",
            log.entries().iter().map(|e| &e.text).collect::<Vec<_>>()
        );
    }

    /// Using a `LearnSpell` scroll with a spell in the database writes the
    /// resolved spell NAME to the GameLog (not the raw numeric ID).
    #[test]
    fn test_learn_spell_scroll_logs_spell_name() {
        use crate::application::resources::GameContent;
        use crate::domain::character::{Alignment, Character, InventorySlot, Sex};

        let db = make_learn_spell_scroll_db();

        let mut gs = GameState::new();
        let mut hero = Character::new(
            "Bookworm".to_string(),
            "human".to_string(),
            "knight".to_string(), // wrong class — will fail but logs spell name
            Sex::Male,
            Alignment::Good,
        );
        hero.inventory.items.push(InventorySlot {
            item_id: 2,
            charges: 1,
        });
        gs.party.add_member(hero).unwrap();
        gs.enter_inventory();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(crate::game::systems::ui::UiPlugin);
        app.add_message::<UseItemExplorationAction>();
        app.insert_resource(GameContent::new(db));
        app.insert_resource(GlobalState(gs));
        app.init_resource::<InventoryNavigationState>();
        app.add_systems(Update, handle_use_item_action_exploration);

        app.world_mut().write_message(UseItemExplorationAction {
            party_index: 0,
            slot_index: 0,
        });
        app.update();
        app.update();

        let log = app.world().resource::<GameLog>();
        let has_spell_name = log.entries().iter().any(|e| e.text.contains("First Aid"));
        assert!(
            has_spell_name,
            "GameLog must contain the resolved spell name 'First Aid'; entries: {:?}",
            log.entries().iter().map(|e| &e.text).collect::<Vec<_>>()
        );
        // Scroll consumed regardless.
        let gs_after = app.world().resource::<GlobalState>();
        assert!(
            gs_after.0.party.members[0].inventory.items.is_empty(),
            "scroll must be consumed even when learning fails"
        );
    }
}
