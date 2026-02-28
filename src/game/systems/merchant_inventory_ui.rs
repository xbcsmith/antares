// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Merchant Inventory UI System - Split-screen buy/sell interface
//!
//! Provides an egui-based split-screen overlay for trading with merchant NPCs.
//! This system is active when the game is in `GameMode::MerchantInventory` mode,
//! which is entered by pressing `I` while in `GameMode::Dialogue` with a
//! merchant NPC whose `is_merchant` flag is `true`.
//!
//! ## Layout
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Merchant Trade: [Character] ←→ [Merchant]      [Esc: Exit] │
//! ├────────────────────────┬────────────────────────────────────┤
//! │  [Character Name]      │       [Merchant Name]              │
//! │  (LEFT PANEL)          │       (RIGHT PANEL)                │
//! │                        │                                    │
//! │  [inventory slot grid] │  [stock entry list]                │
//! │                        │                                    │
//! │  [ Sell ]              │  [ Buy ]                           │
//! └────────────────────────┴────────────────────────────────────┘
//! ```
//!
//! ## Keyboard Navigation (two-phase model)
//!
//! ### Phase 1 — Slot Navigation
//!
//! | Key              | Effect                                                       |
//! |------------------|--------------------------------------------------------------|
//! | `Tab`            | Toggle focus between Character panel (left) and NPC panel (right) |
//! | `1`–`6`          | Switch active character (number key maps to party index 0–5) |
//! | `←` `→` `↑` `↓` | Navigate the slot grid inside the focused panel              |
//! | `Enter`          | Enter **Action Navigation** for the highlighted slot         |
//! | `Esc`            | Close merchant inventory; return to previous mode            |
//!
//! ### Phase 2 — Action Navigation
//!
//! | Key     | Effect                                                              |
//! |---------|---------------------------------------------------------------------|
//! | `←` `→` | Cycle between action buttons                                        |
//! | `Enter`  | Execute the focused action; return to Slot Navigation at slot 0    |
//! | `Esc`    | Cancel; return to Slot Navigation at the previously selected slot  |

use crate::application::merchant_inventory_state::{MerchantFocus, MerchantInventoryState};
use crate::application::resources::GameContent;
use crate::application::GameMode;
use crate::domain::character::Inventory;
use crate::domain::types::ItemId;
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
/// Height of each stock entry row in the merchant panel.
const STOCK_ROW_H: f32 = 28.0;

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
/// Colour for item names in the merchant stock list.
const STOCK_ITEM_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(200, 220, 255, 255);
/// Colour for "out of stock" entries.
const STOCK_EMPTY_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(120, 120, 120, 255);
/// Buy button accent colour.
const BUY_COLOR: egui::Color32 = egui::Color32::from_rgb(80, 200, 120);
/// Sell button accent colour.
const SELL_COLOR: egui::Color32 = egui::Color32::from_rgb(220, 160, 60);

// ===== Plugin =====

/// Plugin for the merchant buy/sell split-screen inventory UI.
pub struct MerchantInventoryPlugin;

impl Plugin for MerchantInventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BuyItemAction>()
            .add_message::<SellItemAction>()
            .init_resource::<MerchantNavState>()
            .add_systems(
                Update,
                (
                    merchant_inventory_input_system,
                    merchant_inventory_ui_system,
                    merchant_inventory_action_system,
                )
                    .chain(),
            );
    }
}

// ===== Messages =====

/// Emitted when the player confirms buying a stock entry from the merchant.
///
/// Gold is deducted from the party and the item is added to the active
/// character's inventory.
///
/// # Examples
///
/// ```
/// use antares::game::systems::merchant_inventory_ui::BuyItemAction;
///
/// let action = BuyItemAction {
///     npc_id: "merchant_tom".to_string(),
///     stock_index: 2,
///     character_index: 0,
/// };
/// assert_eq!(action.stock_index, 2);
/// assert_eq!(action.character_index, 0);
/// ```
#[derive(Message)]
pub struct BuyItemAction {
    /// The NPC ID of the merchant (for stock lookup).
    pub npc_id: String,
    /// Index into `MerchantStock::entries` for the item being bought.
    pub stock_index: usize,
    /// Party index of the character who receives the purchased item.
    pub character_index: usize,
}

/// Emitted when the player confirms selling an item from their inventory.
///
/// The item is removed from the character's inventory and gold is added
/// to the party.
///
/// # Examples
///
/// ```
/// use antares::game::systems::merchant_inventory_ui::SellItemAction;
///
/// let action = SellItemAction {
///     npc_id: "merchant_tom".to_string(),
///     character_index: 0,
///     slot_index: 5,
/// };
/// assert_eq!(action.slot_index, 5);
/// assert_eq!(action.character_index, 0);
/// ```
#[derive(Message)]
pub struct SellItemAction {
    /// The NPC ID of the merchant (for economy lookup).
    pub npc_id: String,
    /// Party index of the character selling the item.
    pub character_index: usize,
    /// Slot index in that character's inventory.
    pub slot_index: usize,
}

// ===== Navigation state =====

/// Tracks keyboard navigation phase for the merchant inventory screen.
///
/// Mirrors `InventoryNavigationState` from `inventory_ui.rs`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::merchant_inventory_ui::MerchantNavState;
/// use antares::game::systems::inventory_ui::NavigationPhase;
///
/// let state = MerchantNavState::default();
/// assert_eq!(state.selected_slot_index, None);
/// assert_eq!(state.focused_action_index, 0);
/// assert!(matches!(state.phase, NavigationPhase::SlotNavigation));
/// ```
#[derive(Resource, Default, Debug)]
pub struct MerchantNavState {
    /// Highlighted slot/row index in the focused panel (`None` = none highlighted).
    pub selected_slot_index: Option<usize>,
    /// Focused action button index when `phase == ActionNavigation`.
    ///
    /// `0` = primary action (Buy or Sell).
    pub focused_action_index: usize,
    /// Current navigation phase.
    pub phase: NavigationPhase,
}

impl MerchantNavState {
    /// Reset to a clean default state.
    fn reset(&mut self) {
        *self = MerchantNavState::default();
    }
}

// ===== Input system =====

/// Handles keyboard input for merchant inventory navigation.
///
/// Runs every frame; only processes input when
/// `GlobalState.0.mode == GameMode::MerchantInventory(_)`.
#[allow(clippy::too_many_lines)]
fn merchant_inventory_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut global_state: ResMut<GlobalState>,
    mut nav_state: ResMut<MerchantNavState>,
    mut buy_writer: MessageWriter<BuyItemAction>,
    mut sell_writer: MessageWriter<SellItemAction>,
) {
    // Guard: only operate in MerchantInventory mode
    let merchant_state = match &global_state.0.mode {
        GameMode::MerchantInventory(s) => s.clone(),
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
        if let GameMode::MerchantInventory(ref mut ms) = global_state.0.mode {
            ms.switch_character(new_idx, party_size);
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

        // Left/Right cycle action buttons (only one action per panel, but
        // keep the model consistent for future multi-action extension)
        if keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::ArrowRight) {
            // Single action per panel — nothing to cycle
            return;
        }

        // Enter → execute focused action
        if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter) {
            let npc_id = merchant_state.npc_id.clone();
            let char_idx = merchant_state.active_character_index;

            match merchant_state.focus {
                MerchantFocus::Right => {
                    // Buy action: highlighted merchant stock row
                    buy_writer.write(BuyItemAction {
                        npc_id,
                        stock_index: slot_idx,
                        character_index: char_idx,
                    });
                }
                MerchantFocus::Left => {
                    // Sell action: highlighted character inventory slot
                    sell_writer.write(SellItemAction {
                        npc_id,
                        character_index: char_idx,
                        slot_index: slot_idx,
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

    // Esc → close merchant inventory screen
    if keyboard.just_pressed(KeyCode::Escape) {
        let resume = merchant_state.get_resume_mode();
        global_state.0.mode = resume;
        nav_state.reset();
        return;
    }

    // Tab → toggle panel focus (character ↔ merchant)
    if keyboard.just_pressed(KeyCode::Tab) {
        if let GameMode::MerchantInventory(ref mut ms) = global_state.0.mode {
            ms.toggle_focus();
        }
        nav_state.selected_slot_index = None;
        nav_state.focused_action_index = 0;
        return;
    }

    // Enter → enter action mode (if a slot/row is highlighted and has content)
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter) {
        if let Some(slot_idx) = nav_state.selected_slot_index {
            let has_content = has_content_at_slot(&merchant_state, slot_idx, &global_state);
            if has_content {
                nav_state.phase = NavigationPhase::ActionNavigation;
                nav_state.focused_action_index = 0;
            }
        } else {
            // No slot highlighted yet — Enter starts navigation at slot 0
            nav_state.selected_slot_index = Some(0);
            if let GameMode::MerchantInventory(ref mut ms) = global_state.0.mode {
                match ms.focus {
                    MerchantFocus::Left => ms.character_selected_slot = Some(0),
                    MerchantFocus::Right => ms.merchant_selected_slot = Some(0),
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

    match merchant_state.focus {
        MerchantFocus::Left => {
            // Character panel: grid navigation (same logic as inventory_ui)
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
            if let GameMode::MerchantInventory(ref mut ms) = global_state.0.mode {
                ms.character_selected_slot = Some(next);
            }
        }
        MerchantFocus::Right => {
            // Merchant panel: linear list navigation (Up/Down only; Left/Right
            // also move through the list for discoverability)
            let stock_len = merchant_stock_len(&merchant_state, &global_state);
            if stock_len == 0 {
                return;
            }
            let current = nav_state.selected_slot_index.unwrap_or(0);
            let next = if keyboard.just_pressed(KeyCode::ArrowDown)
                || keyboard.just_pressed(KeyCode::ArrowRight)
            {
                (current + 1) % stock_len
            } else {
                // ArrowUp / ArrowLeft
                if current == 0 {
                    stock_len - 1
                } else {
                    current - 1
                }
            };
            nav_state.selected_slot_index = Some(next);
            if let GameMode::MerchantInventory(ref mut ms) = global_state.0.mode {
                ms.merchant_selected_slot = Some(next);
            }
        }
    }
}

// ===== UI system =====

/// Renders the merchant inventory split-screen overlay.
#[allow(clippy::too_many_lines)]
fn merchant_inventory_ui_system(
    mut contexts: EguiContexts,
    global_state: Res<GlobalState>,
    game_content: Option<Res<GameContent>>,
    nav_state: Res<MerchantNavState>,
    mut buy_writer: MessageWriter<BuyItemAction>,
    mut sell_writer: MessageWriter<SellItemAction>,
) {
    let merchant_state = match &global_state.0.mode {
        GameMode::MerchantInventory(s) => s.clone(),
        _ => return,
    };

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let char_idx = merchant_state.active_character_index;
    let char_focused = merchant_state.character_has_focus();
    let merchant_focused = merchant_state.merchant_has_focus();

    egui::CentralPanel::default().show(ctx, |ui| {
        // ── Top bar ──────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.heading("Merchant Trade");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new("[Esc] close   [Tab] switch panel   [1-6] switch character")
                        .small()
                        .weak(),
                );
            });
        });

        // ── Status / hint line ───────────────────────────────────────────
        let hint = match nav_state.phase {
            NavigationPhase::SlotNavigation => {
                "Tab: switch panel   1-6: change character   ←→↑↓: navigate   Enter: select   Esc: close"
            }
            NavigationPhase::ActionNavigation => "Enter: execute action   Esc: cancel",
        };
        ui.label(egui::RichText::new(hint).small().weak());
        ui.separator();

        // ── Active character selector strip ──────────────────────────────
        let party_len = global_state.0.party.members.len();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Character:").strong());
            for i in 0..party_len {
                ui.push_id(format!("merch_char_btn_{}", i), |ui| {
                    let member = &global_state.0.party.members[i];
                    let is_active = i == char_idx;
                    let label = egui::RichText::new(format!("[{}] {}", i + 1, member.name))
                        .color(if is_active {
                            egui::Color32::YELLOW
                        } else {
                            egui::Color32::LIGHT_GRAY
                        })
                        .small();
                    if ui.button(label).clicked() {
                        // Character switching via mouse is handled via direct
                        // mutation within the UI — we send the action inline.
                        // (Cannot call ResMut here; handled in input system on
                        //  next frame — acceptable single-frame lag)
                    }
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
            ui.push_id("merch_char_panel", |ui| {
                if let Some(action) = render_character_sell_panel(
                    ui,
                    char_idx,
                    char_focused,
                    merchant_state.character_selected_slot,
                    if char_focused
                        && nav_state.phase == NavigationPhase::ActionNavigation
                    {
                        Some(nav_state.focused_action_index)
                    } else {
                        None
                    },
                    egui::vec2(half_w, panel_h),
                    &global_state,
                    game_content.as_deref(),
                ) {
                    let SellAction { character_index, slot_index } = action;
                    sell_writer.write(SellItemAction {
                        npc_id: merchant_state.npc_id.clone(),
                        character_index,
                        slot_index,
                    });
                }
            });

            // ── RIGHT: Merchant stock panel ───────────────────────────────
            ui.push_id("merch_stock_panel", |ui| {
                if let Some(action) = render_merchant_stock_panel(
                    ui,
                    &merchant_state,
                    merchant_focused,
                    merchant_state.merchant_selected_slot,
                    if merchant_focused
                        && nav_state.phase == NavigationPhase::ActionNavigation
                    {
                        Some(nav_state.focused_action_index)
                    } else {
                        None
                    },
                    egui::vec2(half_w, panel_h),
                    &global_state,
                    game_content.as_deref(),
                ) {
                    let BuyAction { npc_id, stock_index, character_index } = action;
                    buy_writer.write(BuyItemAction {
                        npc_id,
                        stock_index,
                        character_index,
                    });
                }
            });
        });
    });
}

// ===== Panel render helpers =====

/// Return value from `render_character_sell_panel`.
struct SellAction {
    character_index: usize,
    slot_index: usize,
}

/// Render the character inventory panel (left side) and return a `SellAction`
/// if the player clicked the Sell button.
#[allow(clippy::too_many_arguments)]
fn render_character_sell_panel(
    ui: &mut egui::Ui,
    party_index: usize,
    is_focused: bool,
    selected_slot: Option<usize>,
    focused_action_index: Option<usize>,
    size: egui::Vec2,
    global_state: &GlobalState,
    game_content: Option<&GameContent>,
) -> Option<SellAction> {
    if party_index >= global_state.0.party.members.len() {
        return None;
    }

    let character = &global_state.0.party.members[party_index];
    let items = &character.inventory.items;
    let mut result: Option<SellAction> = None;

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

    // ── Action strip: Sell button ─────────────────────────────────────────
    if has_action {
        if let Some(slot_idx) = selected_slot {
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

            child.push_id("sell_actions", |ui| {
                ui.horizontal_wrapped(|ui| {
                    let sell_focused = focused_action_index == Some(0);

                    // Calculate sell price for tooltip
                    let sell_price = game_content
                        .and_then(|gc| gc.db().items.get_item(items[slot_idx].item_id))
                        .map(|item| item.sell_cost)
                        .unwrap_or(0);

                    let sell_label = egui::RichText::new(format!("Sell ({} gold)", sell_price))
                        .color(if sell_focused {
                            ACTION_FOCUSED_COLOR
                        } else {
                            SELL_COLOR
                        })
                        .small();
                    let mut sell_btn = egui::Button::new(sell_label);
                    if sell_focused {
                        sell_btn = sell_btn.stroke(egui::Stroke::new(2.0, ACTION_FOCUSED_COLOR));
                    }
                    if ui
                        .add(sell_btn)
                        .on_hover_text(format!("Sell this item for {} gold", sell_price))
                        .clicked()
                    {
                        result = Some(SellAction {
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

/// Return value from `render_merchant_stock_panel`.
struct BuyAction {
    npc_id: String,
    stock_index: usize,
    character_index: usize,
}

/// Render the merchant stock panel (right side) and return a `BuyAction`
/// if the player clicked the Buy button.
#[allow(clippy::too_many_arguments)]
fn render_merchant_stock_panel(
    ui: &mut egui::Ui,
    merchant_state: &MerchantInventoryState,
    is_focused: bool,
    selected_slot: Option<usize>,
    focused_action_index: Option<usize>,
    size: egui::Vec2,
    global_state: &GlobalState,
    game_content: Option<&GameContent>,
) -> Option<BuyAction> {
    let mut result: Option<BuyAction> = None;

    // Retrieve stock entries
    let stock_entries: Vec<(ItemId, u8, u32)> = {
        let npc_runtime = global_state.0.npc_runtime.get(&merchant_state.npc_id);
        match npc_runtime {
            Some(rt) => rt
                .stock
                .as_ref()
                .map(|s| {
                    s.entries
                        .iter()
                        .map(|e| {
                            let base_price = game_content
                                .and_then(|gc| gc.db().items.get_item(e.item_id))
                                .map(|it| it.base_cost)
                                .unwrap_or(0);
                            let price = s.effective_price(e.item_id, base_price);
                            (e.item_id, e.quantity, price)
                        })
                        .collect()
                })
                .unwrap_or_default(),
            None => Vec::new(),
        }
    };

    let stock_len = stock_entries.len();
    let has_action = selected_slot
        .map(|s| s < stock_len && stock_entries[s].1 > 0)
        .unwrap_or(false);
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
        &merchant_state.npc_name,
        egui::FontId::proportional(16.0),
        egui::Color32::WHITE,
    );
    painter.text(
        header_rect.right_center() - egui::vec2(8.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        "MERCHANT",
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(160, 160, 160),
    );

    // ── Body: stock list ──────────────────────────────────────────────────
    let body_rect = egui::Rect::from_min_size(
        panel_rect.min + egui::vec2(0.0, PANEL_HEADER_H),
        egui::vec2(size.x, body_h),
    );
    // Paint body background then drop painter before calling ui.new_child() (mutable borrow)
    painter.rect_filled(body_rect, 0.0, PANEL_BG_COLOR);
    let _ = painter;

    // Render stock entries as a scrollable list using egui widgets
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(body_rect)
            .layout(egui::Layout::top_down(egui::Align::LEFT)),
    );

    egui::ScrollArea::vertical()
        .id_salt("merchant_stock_scroll")
        .max_height(body_h)
        .show(&mut child, |ui| {
            for (i, (item_id, qty, price)) in stock_entries.iter().enumerate() {
                ui.push_id(format!("stock_row_{}", i), |ui| {
                    let is_selected = selected_slot == Some(i);
                    let is_available = *qty > 0;

                    let item_name = game_content
                        .and_then(|gc| gc.db().items.get_item(*item_id))
                        .map(|it| it.name.clone())
                        .unwrap_or_else(|| format!("Item #{}", item_id));

                    let row_color = if !is_available {
                        STOCK_EMPTY_COLOR
                    } else {
                        STOCK_ITEM_COLOR
                    };

                    let row_bg = if is_selected {
                        egui::Color32::from_rgba_premultiplied(100, 85, 0, 80)
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    let (row_rect, response) = ui.allocate_exact_size(
                        egui::vec2(body_rect.width(), STOCK_ROW_H),
                        egui::Sense::click(),
                    );

                    // Row background
                    if is_selected {
                        ui.painter().rect_filled(row_rect, 0.0, row_bg);
                        ui.painter().rect_stroke(
                            row_rect.shrink(1.0),
                            0.0,
                            egui::Stroke::new(1.5, SELECT_HIGHLIGHT_COLOR),
                            egui::StrokeKind::Outside,
                        );
                    }

                    // Row text: name, qty, price
                    let label = if is_available {
                        format!("  {}  x{}  {} gp", item_name, qty, price)
                    } else {
                        format!("  {}  [Out of Stock]", item_name)
                    };
                    ui.painter().text(
                        row_rect.left_center() + egui::vec2(4.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(14.0),
                        row_color,
                    );

                    if response.clicked() && is_available {
                        // Mouse click on a stock row selects it
                        let _ = row_rect; // suppress unused warning
                    }
                });
            }

            if stock_entries.is_empty() {
                ui.label(
                    egui::RichText::new("  (No stock available)")
                        .color(STOCK_EMPTY_COLOR)
                        .small(),
                );
            }
        });

    // ── Action strip: Buy button ──────────────────────────────────────────
    if has_action {
        if let Some(stock_idx) = selected_slot {
            if let Some(&(item_id, _qty, price)) = stock_entries.get(stock_idx) {
                let item_name = game_content
                    .and_then(|gc| gc.db().items.get_item(item_id))
                    .map(|it| it.name.clone())
                    .unwrap_or_else(|| format!("Item #{}", item_id));

                let party_gold = global_state.0.party.gold;
                let can_afford = party_gold >= price;
                let char_inv_full = global_state
                    .0
                    .party
                    .members
                    .get(merchant_state.active_character_index)
                    .map(|ch| ch.inventory.is_full())
                    .unwrap_or(true);

                let action_rect = egui::Rect::from_min_size(
                    panel_rect.min + egui::vec2(0.0, PANEL_HEADER_H + body_h),
                    egui::vec2(size.x, action_reserve),
                );
                // Use painter_at to avoid borrow conflict with ui.new_child()
                ui.painter_at(action_rect)
                    .rect_filled(action_rect, 0.0, HEADER_BG_COLOR);

                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(action_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                child.add_space(6.0);

                child.push_id("buy_actions", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let buy_focused = focused_action_index == Some(0);
                        let buy_enabled = can_afford && !char_inv_full;

                        let buy_label = egui::RichText::new(format!("Buy ({} gold)", price))
                            .color(if buy_focused {
                                ACTION_FOCUSED_COLOR
                            } else {
                                BUY_COLOR
                            })
                            .small();
                        let mut buy_btn = egui::Button::new(buy_label);
                        if buy_focused {
                            buy_btn = buy_btn.stroke(egui::Stroke::new(2.0, ACTION_FOCUSED_COLOR));
                        }

                        let hover_text = if !can_afford {
                            format!("Not enough gold (need {}, have {})", price, party_gold)
                        } else if char_inv_full {
                            "Character's inventory is full".to_string()
                        } else {
                            format!("Buy {} for {} gold", item_name, price)
                        };

                        if ui
                            .add_enabled(buy_enabled, buy_btn)
                            .on_hover_text(hover_text)
                            .clicked()
                        {
                            result = Some(BuyAction {
                                npc_id: merchant_state.npc_id.clone(),
                                stock_index: stock_idx,
                                character_index: merchant_state.active_character_index,
                            });
                        }
                    });
                });
            }
        }
    }

    result
}

// ===== Action system =====

/// Executes buy and sell actions.
///
/// Reads `BuyItemAction` and `SellItemAction` messages, mutates `GlobalState`,
/// and resets keyboard navigation state after each action.
#[allow(clippy::too_many_lines)]
fn merchant_inventory_action_system(
    mut buy_reader: MessageReader<BuyItemAction>,
    mut sell_reader: MessageReader<SellItemAction>,
    mut global_state: ResMut<GlobalState>,
    mut nav_state: ResMut<MerchantNavState>,
    game_content: Option<Res<GameContent>>,
) {
    let buy_events: Vec<(String, usize, usize)> = buy_reader
        .read()
        .map(|e| (e.npc_id.clone(), e.stock_index, e.character_index))
        .collect();

    let sell_events: Vec<(String, usize, usize)> = sell_reader
        .read()
        .map(|e| (e.npc_id.clone(), e.character_index, e.slot_index))
        .collect();

    // ── Buy events ────────────────────────────────────────────────────────
    for (npc_id, stock_index, character_index) in buy_events {
        // Bounds-check character
        if character_index >= global_state.0.party.members.len() {
            warn!(
                "BuyItemAction: character_index {} out of bounds (party size {})",
                character_index,
                global_state.0.party.members.len()
            );
            continue;
        }

        // Check inventory space
        if global_state.0.party.members[character_index]
            .inventory
            .is_full()
        {
            warn!(
                "BuyItemAction: character[{}] inventory is full; cannot buy",
                character_index
            );
            continue;
        }

        // Retrieve item_id and price from NPC runtime stock
        let (item_id, price) = {
            let rt = global_state.0.npc_runtime.get(&npc_id);
            match rt {
                Some(rt) => {
                    let stock = match rt.stock.as_ref() {
                        Some(s) => s,
                        None => {
                            warn!("BuyItemAction: NPC {} has no stock", npc_id);
                            continue;
                        }
                    };
                    let entry = match stock.entries.get(stock_index) {
                        Some(e) => e,
                        None => {
                            warn!(
                                "BuyItemAction: stock_index {} out of bounds for NPC {}",
                                stock_index, npc_id
                            );
                            continue;
                        }
                    };
                    if !entry.is_available() {
                        warn!(
                            "BuyItemAction: NPC {} stock entry {} is out of stock",
                            npc_id, stock_index
                        );
                        continue;
                    }
                    let base_cost = game_content
                        .as_deref()
                        .and_then(|gc| gc.db().items.get_item(entry.item_id))
                        .map(|it| it.base_cost)
                        .unwrap_or(0);
                    let price = stock.effective_price(entry.item_id, base_cost);
                    (entry.item_id, price)
                }
                None => {
                    warn!("BuyItemAction: no runtime state for NPC {}", npc_id);
                    continue;
                }
            }
        };

        // Check gold
        if global_state.0.party.gold < price {
            warn!(
                "BuyItemAction: not enough gold (have {}, need {})",
                global_state.0.party.gold, price
            );
            continue;
        }

        // Deduct gold
        global_state.0.party.gold = global_state.0.party.gold.saturating_sub(price);

        // Add item to character inventory
        match global_state.0.party.members[character_index]
            .inventory
            .add_item(item_id, 0)
        {
            Ok(()) => {
                info!(
                    "Bought item_id={} from NPC {} for {} gold; added to party[{}]",
                    item_id, npc_id, price, character_index
                );
                // Decrement NPC stock
                if let Some(rt) = global_state.0.npc_runtime.get_mut(&npc_id) {
                    if let Some(stock) = rt.stock.as_mut() {
                        stock.decrement(item_id);
                    }
                }
            }
            Err(err) => {
                // Rollback gold
                global_state.0.party.gold = global_state.0.party.gold.saturating_add(price);
                warn!(
                    "BuyItemAction: add_item failed for party[{}]: {:?}; gold refunded",
                    character_index, err
                );
            }
        }

        nav_state.selected_slot_index = None;
        nav_state.focused_action_index = 0;
        nav_state.phase = NavigationPhase::SlotNavigation;
    }

    // ── Sell events ───────────────────────────────────────────────────────
    for (npc_id, character_index, slot_index) in sell_events {
        if character_index >= global_state.0.party.members.len() {
            warn!(
                "SellItemAction: character_index {} out of bounds (party size {})",
                character_index,
                global_state.0.party.members.len()
            );
            continue;
        }

        let inv_len = global_state.0.party.members[character_index]
            .inventory
            .items
            .len();
        if slot_index >= inv_len {
            warn!(
                "SellItemAction: slot_index {} out of bounds (inventory size {}) for party[{}]",
                slot_index, inv_len, character_index
            );
            continue;
        }

        let item_id = global_state.0.party.members[character_index]
            .inventory
            .items[slot_index]
            .item_id;

        // Determine sell price from NPC economy settings or item sell_cost
        let sell_price = {
            let base_sell_cost = game_content
                .as_deref()
                .and_then(|gc| gc.db().items.get_item(item_id))
                .map(|it| it.sell_cost)
                .unwrap_or(0);

            let economy = global_state.0.npc_runtime.get(&npc_id).and_then(|_rt| {
                // Economy settings live on NpcDefinition; look up from content
                game_content
                    .as_deref()
                    .and_then(|gc| gc.db().npcs.get_npc(&npc_id))
                    .and_then(|npc| npc.economy.clone())
            });

            match economy {
                Some(eco) => eco.npc_buy_price(base_sell_cost),
                None => base_sell_cost,
            }
        };

        // Remove item from character
        if let Some(removed) = global_state.0.party.members[character_index]
            .inventory
            .remove_item(slot_index)
        {
            global_state.0.party.gold = global_state.0.party.gold.saturating_add(sell_price);
            info!(
                "Sold item_id={} from party[{}] slot {} to NPC {} for {} gold",
                removed.item_id, character_index, slot_index, npc_id, sell_price
            );
        }

        nav_state.selected_slot_index = None;
        nav_state.focused_action_index = 0;
        nav_state.phase = NavigationPhase::SlotNavigation;
    }
}

// ===== Helpers =====

/// Returns the number of stock entries the given merchant has.
fn merchant_stock_len(
    merchant_state: &MerchantInventoryState,
    global_state: &GlobalState,
) -> usize {
    global_state
        .0
        .npc_runtime
        .get(&merchant_state.npc_id)
        .and_then(|rt| rt.stock.as_ref())
        .map(|s| s.entries.len())
        .unwrap_or(0)
}

/// Returns `true` if the highlighted slot in the currently focused panel
/// contains a usable item or available stock entry.
fn has_content_at_slot(
    merchant_state: &MerchantInventoryState,
    slot_idx: usize,
    global_state: &GlobalState,
) -> bool {
    match merchant_state.focus {
        MerchantFocus::Left => {
            // Character panel: check inventory item exists
            global_state
                .0
                .party
                .members
                .get(merchant_state.active_character_index)
                .map(|ch| slot_idx < ch.inventory.items.len())
                .unwrap_or(false)
        }
        MerchantFocus::Right => {
            // Merchant panel: check stock entry is available
            global_state
                .0
                .npc_runtime
                .get(&merchant_state.npc_id)
                .and_then(|rt| rt.stock.as_ref())
                .and_then(|s| s.entries.get(slot_idx))
                .map(|e| e.is_available())
                .unwrap_or(false)
        }
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::merchant_inventory_state::{MerchantFocus, MerchantInventoryState};
    use crate::application::{GameMode, GameState};
    use crate::domain::inventory::{MerchantStock, StockEntry};
    use crate::domain::world::npc_runtime::NpcRuntimeState;

    fn make_game_state_with_merchant() -> (GameState, String) {
        use crate::domain::character::{Alignment, Character, Sex};

        let mut state = GameState::new();

        // Create a party member with two items so the character panel has content
        let mut character = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        character
            .inventory
            .add_item(1, 0)
            .expect("add item should succeed");
        character
            .inventory
            .add_item(2, 0)
            .expect("add second item should succeed");
        state
            .party
            .add_member(character)
            .expect("add_member should succeed");
        state.party.gold = 500;

        // Set up merchant runtime stock
        let npc_id = "test_merchant".to_string();
        let mut npc_state = NpcRuntimeState::new(npc_id.clone());
        let mut stock = MerchantStock::new();
        stock.entries.push(StockEntry::new(10, 5));
        stock.entries.push(StockEntry::new(11, 1));
        npc_state.stock = Some(stock);
        state.npc_runtime.insert(npc_state);

        state.enter_merchant_inventory(npc_id.clone(), "Test Merchant".to_string());

        (state, npc_id)
    }

    #[test]
    fn test_merchant_inventory_plugin_builds() {
        let mut app = App::new();
        app.add_plugins(MerchantInventoryPlugin);
        // Plugin registers the resource and messages without panic
    }

    #[test]
    fn test_merchant_nav_state_default() {
        let state = MerchantNavState::default();
        assert_eq!(state.selected_slot_index, None);
        assert_eq!(state.focused_action_index, 0);
        assert!(matches!(state.phase, NavigationPhase::SlotNavigation));
    }

    #[test]
    fn test_merchant_nav_state_reset() {
        let mut state = MerchantNavState {
            selected_slot_index: Some(3),
            focused_action_index: 2,
            phase: NavigationPhase::ActionNavigation,
        };
        state.reset();
        assert_eq!(state.selected_slot_index, None);
        assert_eq!(state.focused_action_index, 0);
        assert!(matches!(state.phase, NavigationPhase::SlotNavigation));
    }

    #[test]
    fn test_buy_item_action_fields() {
        let action = BuyItemAction {
            npc_id: "merchant_tom".to_string(),
            stock_index: 2,
            character_index: 0,
        };
        assert_eq!(action.npc_id, "merchant_tom");
        assert_eq!(action.stock_index, 2);
        assert_eq!(action.character_index, 0);
    }

    #[test]
    fn test_sell_item_action_fields() {
        let action = SellItemAction {
            npc_id: "merchant_tom".to_string(),
            character_index: 1,
            slot_index: 3,
        };
        assert_eq!(action.npc_id, "merchant_tom");
        assert_eq!(action.character_index, 1);
        assert_eq!(action.slot_index, 3);
    }

    #[test]
    fn test_merchant_stock_len_returns_zero_for_unknown_npc() {
        let state = GameState::new();
        let global = crate::game::resources::GlobalState(state);

        let ms = MerchantInventoryState::new(
            "unknown_npc".to_string(),
            "Unknown".to_string(),
            0,
            GameMode::Exploration,
        );

        assert_eq!(merchant_stock_len(&ms, &global), 0);
    }

    #[test]
    fn test_merchant_stock_len_returns_correct_count() {
        let (game_state, npc_id) = make_game_state_with_merchant();
        let global = crate::game::resources::GlobalState(game_state);

        let ms = MerchantInventoryState::new(
            npc_id,
            "Test Merchant".to_string(),
            0,
            GameMode::Exploration,
        );

        assert_eq!(merchant_stock_len(&ms, &global), 2);
    }

    #[test]
    fn test_has_content_at_slot_character_panel_with_item() {
        let (game_state, npc_id) = make_game_state_with_merchant();
        let global = crate::game::resources::GlobalState(game_state);

        let ms = MerchantInventoryState::new(
            npc_id,
            "Test Merchant".to_string(),
            0,
            GameMode::Exploration,
        );

        // Character has 2 items at slots 0 and 1
        assert!(has_content_at_slot(&ms, 0, &global));
        assert!(has_content_at_slot(&ms, 1, &global));
        assert!(!has_content_at_slot(&ms, 5, &global));
    }

    #[test]
    fn test_has_content_at_slot_merchant_panel_available_stock() {
        let (game_state, npc_id) = make_game_state_with_merchant();
        let global = crate::game::resources::GlobalState(game_state);

        let mut ms = MerchantInventoryState::new(
            npc_id,
            "Test Merchant".to_string(),
            0,
            GameMode::Exploration,
        );
        ms.focus = MerchantFocus::Right;

        // Stock entries: index 0 has qty 5 (available), index 1 has qty 1 (available)
        assert!(has_content_at_slot(&ms, 0, &global));
        assert!(has_content_at_slot(&ms, 1, &global));
        assert!(!has_content_at_slot(&ms, 99, &global));
    }

    #[test]
    fn test_has_content_at_slot_merchant_panel_out_of_stock() {
        let mut state = GameState::new();
        let npc_id = "test_merchant_empty".to_string();
        let mut npc_state = NpcRuntimeState::new(npc_id.clone());
        let mut stock = MerchantStock::new();
        // qty = 0 means out of stock
        stock.entries.push(StockEntry::new(10, 0));
        npc_state.stock = Some(stock);
        state.npc_runtime.insert(npc_state);

        let global = crate::game::resources::GlobalState(state);
        let mut ms = MerchantInventoryState::new(
            npc_id,
            "Empty Merchant".to_string(),
            0,
            GameMode::Exploration,
        );
        ms.focus = MerchantFocus::Right;

        assert!(!has_content_at_slot(&ms, 0, &global));
    }

    #[test]
    fn test_game_state_enter_merchant_inventory_sets_mode() {
        let mut state = GameState::new();
        state.enter_merchant_inventory("npc_001".to_string(), "Test NPC".to_string());
        assert!(matches!(state.mode, GameMode::MerchantInventory(_)));
    }

    #[test]
    fn test_game_state_enter_merchant_inventory_stores_previous_mode() {
        let mut state = GameState::new();
        // Mode starts as Exploration
        assert!(matches!(state.mode, GameMode::Exploration));
        state.enter_merchant_inventory("npc_001".to_string(), "Test NPC".to_string());
        if let GameMode::MerchantInventory(ref ms) = state.mode {
            assert!(matches!(ms.get_resume_mode(), GameMode::Exploration));
        } else {
            panic!("Expected MerchantInventory mode");
        }
    }

    #[test]
    fn test_game_state_enter_merchant_inventory_defaults_to_character_zero() {
        let mut state = GameState::new();
        state.enter_merchant_inventory("npc_001".to_string(), "NPC".to_string());
        if let GameMode::MerchantInventory(ref ms) = state.mode {
            assert_eq!(ms.active_character_index, 0);
        } else {
            panic!("Expected MerchantInventory mode");
        }
    }

    #[test]
    fn test_game_state_enter_merchant_inventory_focus_defaults_left() {
        let mut state = GameState::new();
        state.enter_merchant_inventory("npc_001".to_string(), "NPC".to_string());
        if let GameMode::MerchantInventory(ref ms) = state.mode {
            assert!(matches!(ms.focus, MerchantFocus::Left));
        } else {
            panic!("Expected MerchantInventory mode");
        }
    }
}
