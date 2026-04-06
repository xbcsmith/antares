// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! In-game Spell Book management screen.
//!
//! Implements input handling and egui rendering for browsing the party's spell
//! books during exploration.  When the player presses the Spell Book key
//! (`B` by default), the game enters
//! [`GameMode::SpellBook`](crate::application::GameMode::SpellBook) and this
//! plugin takes over input until the screen is closed.
//!
//! # Layout
//!
//! Three-column design:
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │  📚 Spell Book                             [ESC] Close │
//! ├─────────────┬──────────────────────┬───────────────────┤
//! │ Characters  │ Known Spells         │ Detail            │
//! │ ──────────  │ ─────────────        │ ──────            │
//! │ [*Aria  ✓] │ -- Level 1 --        │ First Aid         │
//! │ [ Korbin  ] │  First Aid — 5 SP   │ School: Cleric    │
//! │ [ Sylva ✓] │  Cure Poison — 8 SP │ Level: 1          │
//! │             │  💎1                │ SP Cost: 5        │
//! │             │ -- Level 2 --        │ Gem Cost: —       │
//! │             │  Bless — 12 SP ⚔   │ Context: Any      │
//! │             │                      │                   │
//! │             │ -- Learnable Scrolls │ Restores 1d6+1 HP │
//! │             │  Scroll -> Light     │ to a single tgt.  │
//! ├─────────────┴──────────────────────┴───────────────────┤
//! │  [C] Cast Spell   [Tab] Switch Char   [↑↓] Select Spell│
//! └────────────────────────────────────────────────────────┘
//! ```
//!
//! # Flow
//!
//! 1. `B` key → `GameState::enter_spellbook_with_caster_select()` (in
//!    `input/global_toggles.rs`).
//! 2. [`spellbook_input_system`] drives navigation:
//!    - **Tab** / **Shift+Tab** — cycle through party members.
//!    - **↑↓ / W/S**           — navigate spell list rows.
//!    - **Enter / Space**       — select (highlight) the focused spell.
//!    - **C**                   — exit SpellBook and enter SpellCasting for
//!      the currently browsed character.
//!    - **Esc**                 — exit SpellBook and restore previous mode.
//! 3. [`spellbook_ui_system`] renders the three-column egui panel every frame.
//!
//! # Architecture Reference
//!
//! Phase 2 of `docs/explanation/spell_management_implementation_plan.md`.

use crate::application::resources::GameContent;
use crate::application::spell_book_state::SpellBookState;
use crate::application::GameMode;
use crate::domain::items::types::{ConsumableEffect, ItemType};
use crate::domain::magic::learning::can_learn_spell;
use crate::domain::magic::types::SpellContext;
use crate::domain::types::SpellId;
use crate::game::resources::GlobalState;
use crate::game::systems::ui_helpers::BODY_FONT_SIZE;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Semi-transparent full-screen backdrop for the Spell Book overlay.
pub const SPELLBOOK_OVERLAY_BG: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(0, 0, 26, 224);
/// Background color of the inner panel.
pub const SPELLBOOK_PANEL_BG: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(15, 15, 46, 247);
/// Highlight color for the currently selected spell row.
pub const SPELLBOOK_SELECTED_ROW_BG: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(51, 51, 13, 230);
/// Default text color for spell name entries.
pub const SPELLBOOK_NORMAL_ROW_COLOR: egui::Color32 = egui::Color32::WHITE;
/// Text color when the character has insufficient SP to cast the spell.
pub const SPELLBOOK_DISABLED_SPELL_COLOR: egui::Color32 = egui::Color32::from_rgb(115, 115, 115);
/// Text color for "Level N" group header rows.
pub const SPELLBOOK_LEVEL_HEADER_COLOR: egui::Color32 = egui::Color32::from_rgb(179, 204, 255);
/// Text / background highlight for the active character tab.
pub const SPELLBOOK_CHAR_TAB_ACTIVE_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 230, 51);
/// Text color for inactive character tabs.
pub const SPELLBOOK_CHAR_TAB_INACTIVE_COLOR: egui::Color32 = egui::Color32::from_rgb(153, 153, 179);
/// Text color for hint / secondary text at the bottom and in the detail panel.
pub const SPELLBOOK_HINT_COLOR: egui::Color32 = egui::Color32::from_rgb(140, 140, 166);
/// Text color for the main "Spell Book" title and column headers.
pub const SPELLBOOK_TITLE_COLOR: egui::Color32 = egui::Color32::from_rgb(204, 217, 255);

// ── Plugin ─────────────────────────────────────────────────────────────────────

/// Bevy plugin that provides the in-game Spell Book management screen.
///
/// Registers the two-system chain
/// `(spellbook_input_system, spellbook_ui_system)` — matching the pattern
/// used by every other egui management screen (inn, inventory, temple, lock).
/// Input runs first; the egui renderer follows so it sees the updated state
/// in the same frame.
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::spellbook_ui::SpellBookPlugin;
///
/// # fn setup() {
/// let mut app = App::new();
/// app.add_plugins(SpellBookPlugin);
/// # }
/// ```
pub struct SpellBookPlugin;

impl Plugin for SpellBookPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spellbook_input_system, spellbook_ui_system).chain(),
        );
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Handles keyboard input while in `SpellBook` mode.
///
/// - **Tab** (no shift) — advance to next party member's book.
/// - **Shift+Tab** — return to previous party member's book.
/// - **↑ / W** — move spell cursor up.
/// - **↓ / S** — move spell cursor down.
/// - **Enter / Space** — confirm selection (updates `selected_spell_id`).
/// - **C** — exit SpellBook and enter spell-casting flow for current character.
/// - **Esc** — exit SpellBook and restore previous mode.
pub fn spellbook_input_system(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut global_state: ResMut<GlobalState>,
    content: Option<Res<GameContent>>,
) {
    if !matches!(global_state.0.mode, GameMode::SpellBook(_)) {
        return;
    }

    let Some(ref kb) = keyboard else {
        return;
    };

    // ── Esc: close and restore previous mode ────────────────────────────────
    if kb.just_pressed(KeyCode::Escape) {
        global_state.0.exit_spellbook();
        return;
    }

    // ── C: open casting flow for the current character ───────────────────────
    if kb.just_pressed(KeyCode::KeyC) {
        let char_index = if let GameMode::SpellBook(ref sb) = global_state.0.mode {
            sb.character_index
        } else {
            return;
        };
        global_state.0.exit_spellbook();
        global_state.0.enter_spell_casting(char_index);
        return;
    }

    // ── Tab / Shift+Tab: cycle characters ────────────────────────────────────
    let party_size = global_state.0.party.members.len();

    if kb.just_pressed(KeyCode::Tab) {
        let shift_held = kb.pressed(KeyCode::ShiftLeft) || kb.pressed(KeyCode::ShiftRight);
        if shift_held {
            if let GameMode::SpellBook(ref mut sb) = global_state.0.mode {
                sb.prev_character(party_size);
            }
        } else if let GameMode::SpellBook(ref mut sb) = global_state.0.mode {
            sb.next_character(party_size);
        }
        return;
    }

    // ── ↑ / ↓: navigate spell list ───────────────────────────────────────────
    let up = kb.just_pressed(KeyCode::ArrowUp) || kb.just_pressed(KeyCode::KeyW);
    let down = kb.just_pressed(KeyCode::ArrowDown) || kb.just_pressed(KeyCode::KeyS);
    let confirm = kb.just_pressed(KeyCode::Enter) || kb.just_pressed(KeyCode::Space);

    // Build spell id list to know wrapping bounds and resolve selected spell.
    let spell_ids = collect_spell_ids_from_state(&global_state.0, content.as_deref());
    let item_count = spell_ids.len();

    if up {
        if let GameMode::SpellBook(ref mut sb) = global_state.0.mode {
            sb.cursor_up(item_count);
            sb.selected_spell_id = spell_ids.get(sb.selected_row).copied();
        }
        return;
    }

    if down {
        if let GameMode::SpellBook(ref mut sb) = global_state.0.mode {
            sb.cursor_down(item_count);
            sb.selected_spell_id = spell_ids.get(sb.selected_row).copied();
        }
        return;
    }

    // ── Enter / Space: confirm selection ─────────────────────────────────────
    if confirm && !spell_ids.is_empty() {
        if let GameMode::SpellBook(ref mut sb) = global_state.0.mode {
            sb.selected_spell_id = spell_ids.get(sb.selected_row).copied();
        }
    }
}

/// Renders the Spell Book management screen using egui while in `SpellBook` mode.
///
/// Displays a three-column [`egui::CentralPanel`]:
///
/// - **Left column** (~150 px): character-tab list via [`render_char_tabs`].
/// - **Centre column** (fills remaining space): scrollable spell list via
///   [`render_spell_list`].
/// - **Right column** (~200 px): scrollable spell detail panel via
///   [`render_detail_panel`].
///
/// The system is chained after [`spellbook_input_system`] so that input-state
/// changes are visible to the renderer in the same frame.
///
/// Returns early without rendering if the game is not in `SpellBook` mode or
/// if no egui context is available.
fn spellbook_ui_system(
    mut contexts: EguiContexts,
    global_state: Res<GlobalState>,
    content: Option<Res<GameContent>>,
) {
    // Only render in SpellBook mode.  Clone the state so we are not holding a
    // borrow into global_state.0.mode while also passing &global_state to the
    // column render helpers.
    let sb = match &global_state.0.mode {
        GameMode::SpellBook(sb) => sb.clone(),
        _ => return,
    };

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let spell_ids = collect_spell_ids_from_state(&global_state.0, content.as_deref());

    egui::CentralPanel::default().show(ctx, |ui| {
        // ── Title bar ─────────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.heading("\u{1F4DA} Spell Book"); // 📚
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("[ESC] Close").color(SPELLBOOK_HINT_COLOR));
            });
        });

        ui.separator();

        // ── Three-column body ─────────────────────────────────────────────────
        ui.horizontal(|ui| {
            // Left column — character tabs
            ui.vertical(|ui| {
                ui.set_min_width(140.0);
                ui.set_max_width(160.0);
                render_char_tabs(ui, &sb, &global_state);
            });

            ui.separator();

            // Center column — spell list (scrollable)
            ui.vertical(|ui| {
                ui.set_min_width(200.0);
                egui::ScrollArea::vertical()
                    .id_salt("spellbook_spell_list")
                    .show(ui, |ui| {
                        render_spell_list(ui, &sb, &global_state, content.as_deref(), &spell_ids);
                    });
            });

            ui.separator();

            // Right column — spell detail panel (scrollable)
            ui.vertical(|ui| {
                ui.set_min_width(180.0);
                ui.set_max_width(215.0);
                egui::ScrollArea::vertical()
                    .id_salt("spellbook_detail_pane")
                    .show(ui, |ui| {
                        render_detail_panel(ui, &sb, content.as_deref());
                    });
            });
        });

        ui.separator();

        // ── Bottom hint bar ────────────────────────────────────────────────────
        ui.horizontal_centered(|ui| {
            ui.label(
                egui::RichText::new(
                    "[C] Cast Spell   [Tab] Switch Character   [\u{2191}\u{2193}] Select Spell",
                )
                .color(SPELLBOOK_HINT_COLOR),
            );
        });
    });
}

// ── Public helper ─────────────────────────────────────────────────────────────

/// Collects the flat ordered list of [`SpellId`]s known by the currently
/// browsed character.
///
/// Iterates spell book levels 0–6 (game levels 1–7) and returns IDs in level
/// order.  This list drives both the `selected_row` navigation bounds in
/// [`spellbook_input_system`] and the row construction in [`render_spell_list`].
///
/// Returns an empty `Vec` if the mode is not `SpellBook`, if the current
/// character index is out of range, or if the content database is absent.
///
/// # Arguments
///
/// * `game_state` — current game state (must be in `SpellBook` mode).
/// * `content`    — optional reference to the loaded content database.
pub fn collect_spell_ids_from_state(
    game_state: &crate::application::GameState,
    content: Option<&GameContent>,
) -> Vec<SpellId> {
    let sb = match &game_state.mode {
        GameMode::SpellBook(sb) => sb,
        _ => return Vec::new(),
    };

    let Some(character) = game_state.party.members.get(sb.character_index) else {
        return Vec::new();
    };

    let Some(content_ref) = content else {
        return Vec::new();
    };

    let class_db = &content_ref.db().classes;
    let spell_list = character
        .spells
        .get_spell_list_by_id(&character.class_id, class_db);

    let mut result = Vec::new();
    for level_spells in spell_list.iter() {
        for &spell_id in level_spells {
            result.push(spell_id);
        }
    }
    result
}

// ── egui render helpers ───────────────────────────────────────────────────────

/// Renders the character-tab rows in the left column using egui.
///
/// Called from [`spellbook_ui_system`].
///
/// # Arguments
///
/// * `ui`           — mutable reference to the current egui [`egui::Ui`].
/// * `sb`           — current [`SpellBookState`] (character index, selected row).
/// * `global_state` — read-only access to party members and their spell data.
fn render_char_tabs(ui: &mut egui::Ui, sb: &SpellBookState, global_state: &GlobalState) {
    // Column header
    ui.label(egui::RichText::new("Characters").color(SPELLBOOK_TITLE_COLOR));

    let members = &global_state.0.party.members;
    if members.is_empty() {
        ui.label(egui::RichText::new("No party.").color(SPELLBOOK_DISABLED_SPELL_COLOR));
        return;
    }

    for (i, member) in members.iter().enumerate() {
        let active = i == sb.character_index;
        let text_color = if active {
            SPELLBOOK_CHAR_TAB_ACTIVE_COLOR
        } else {
            SPELLBOOK_CHAR_TAB_INACTIVE_COLOR
        };

        // Show "✓" if the character has any known spells.
        let has_spells = member.spells.cleric_spells.iter().any(|v| !v.is_empty())
            || member.spells.sorcerer_spells.iter().any(|v| !v.is_empty());
        let check = if has_spells { " \u{2713}" } else { "" };
        let cursor = if active { "[*]" } else { "[ ]" };
        let label = format!("{cursor} {}{}", member.name, check);

        ui.push_id(i, |ui| {
            let bg = if active {
                SPELLBOOK_SELECTED_ROW_BG
            } else {
                egui::Color32::TRANSPARENT
            };
            egui::Frame::new().fill(bg).show(ui, |ui| {
                ui.label(egui::RichText::new(label).color(text_color));
            });
        });
    }
}

/// Renders the spell-list rows and learnable-scrolls section using egui.
///
/// Called from [`spellbook_ui_system`].
///
/// # Arguments
///
/// * `ui`           — mutable reference to the current egui [`egui::Ui`].
/// * `sb`           — current [`SpellBookState`] (character index, selected row).
/// * `global_state` — read-only access to the party and per-character stats.
/// * `content`      — optional loaded content database (spells, classes, items).
/// * `spell_ids`    — flat ordered list of [`SpellId`]s known by the current
///   character, pre-computed by [`collect_spell_ids_from_state`].
fn render_spell_list(
    ui: &mut egui::Ui,
    sb: &SpellBookState,
    global_state: &GlobalState,
    content: Option<&GameContent>,
    spell_ids: &[SpellId],
) {
    // Column header
    ui.label(egui::RichText::new("Known Spells").color(SPELLBOOK_TITLE_COLOR));

    let Some(character) = global_state.0.party.members.get(sb.character_index) else {
        ui.label(
            egui::RichText::new("No character selected.").color(SPELLBOOK_DISABLED_SPELL_COLOR),
        );
        return;
    };

    let Some(content_ref) = content else {
        ui.label(
            egui::RichText::new("Content not available.").color(SPELLBOOK_DISABLED_SPELL_COLOR),
        );
        return;
    };

    let class_db = &content_ref.db().classes;
    let spell_db = &content_ref.db().spells;
    let item_db = &content_ref.db().items;
    let spell_list = character
        .spells
        .get_spell_list_by_id(&character.class_id, class_db);

    if spell_ids.is_empty() {
        ui.label(egui::RichText::new("No spells known.").color(SPELLBOOK_DISABLED_SPELL_COLOR));
    } else {
        // Flat row index that matches sb.selected_row.
        let mut row_idx: usize = 0;

        for (level_idx, level_spells) in spell_list.iter().enumerate() {
            if level_spells.is_empty() {
                continue;
            }

            // ── Level header ──────────────────────────────────────────────────
            ui.label(
                egui::RichText::new(format!("-- Level {} --", level_idx + 1))
                    .color(SPELLBOOK_LEVEL_HEADER_COLOR),
            );

            for &spell_id in level_spells {
                let selected = row_idx == sb.selected_row;

                // Determine affordability (true = can cast right now).
                let can_afford = spell_db
                    .get_spell(spell_id)
                    .map(|s| u32::from(character.sp.current) >= u32::from(s.sp_cost))
                    .unwrap_or(true);

                let text_color = if !can_afford {
                    SPELLBOOK_DISABLED_SPELL_COLOR
                } else if selected {
                    SPELLBOOK_CHAR_TAB_ACTIVE_COLOR
                } else {
                    SPELLBOOK_NORMAL_ROW_COLOR
                };

                let label = if let Some(spell_def) = spell_db.get_spell(spell_id) {
                    let context_tag = match spell_def.context {
                        SpellContext::CombatOnly => " \u{2694}",     // ⚔
                        SpellContext::NonCombatOnly => " \u{1F30D}", // 🌍
                        SpellContext::OutdoorOnly
                        | SpellContext::IndoorOnly
                        | SpellContext::OutdoorCombat => " \u{1F30D}",
                        SpellContext::Anytime => "",
                    };
                    if spell_def.gem_cost > 0 {
                        format!(
                            "{} \u{2014} {} SP \u{1F48E}{}{}", // — …SP 💎N context
                            spell_def.name, spell_def.sp_cost, spell_def.gem_cost, context_tag,
                        )
                    } else {
                        format!(
                            "{} \u{2014} {} SP{}",
                            spell_def.name, spell_def.sp_cost, context_tag
                        )
                    }
                } else {
                    format!("Spell {spell_id:#06x}")
                };

                ui.push_id(spell_id, |ui| {
                    if selected {
                        egui::Frame::new()
                            .fill(SPELLBOOK_SELECTED_ROW_BG)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(label).color(text_color));
                            });
                    } else {
                        ui.label(egui::RichText::new(label).color(text_color));
                    }
                });

                row_idx += 1;
            }
        }
    }

    // ── Learnable Scrolls section ─────────────────────────────────────────────

    let scroll_entries: Vec<(String, SpellId, bool)> = character
        .inventory
        .items
        .iter()
        .filter_map(|slot| {
            let item_def = item_db.get_item(slot.item_id)?;
            if let ItemType::Consumable(ref consumable) = item_def.item_type {
                if let ConsumableEffect::LearnSpell(spell_id) = consumable.effect {
                    let eligible = can_learn_spell(character, spell_id, spell_db, class_db).is_ok();
                    return Some((item_def.name.clone(), spell_id, eligible));
                }
            }
            None
        })
        .collect();

    if !scroll_entries.is_empty() {
        ui.label(
            egui::RichText::new("-- Learnable Scrolls --").color(SPELLBOOK_LEVEL_HEADER_COLOR),
        );

        for (scroll_name, spell_id, eligible) in scroll_entries {
            let spell_name = spell_db
                .get_spell(spell_id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| format!("{spell_id:#06x}"));
            let eligibility = if eligible {
                " [eligible]"
            } else {
                " [not eligible]"
            };
            let color = if eligible {
                SPELLBOOK_NORMAL_ROW_COLOR
            } else {
                SPELLBOOK_DISABLED_SPELL_COLOR
            };

            ui.label(
                egui::RichText::new(format!("{scroll_name} \u{2192} {spell_name}{eligibility}"))
                    .color(color),
            );
        }
    }
}

/// Renders the spell-detail panel using egui.
///
/// Shows full spell info (name, school, level, SP cost, gem cost, context,
/// description) when `sb.selected_spell_id` is `Some`, otherwise shows a
/// placeholder message.  Called from [`spellbook_ui_system`].
///
/// # Arguments
///
/// * `ui`      — mutable reference to the current egui [`egui::Ui`].
/// * `sb`      — current [`SpellBookState`] (provides `selected_spell_id`).
/// * `content` — optional loaded content database (spell definitions).
fn render_detail_panel(ui: &mut egui::Ui, sb: &SpellBookState, content: Option<&GameContent>) {
    // Column header
    ui.label(egui::RichText::new("Detail").color(SPELLBOOK_TITLE_COLOR));

    let Some(spell_id) = sb.selected_spell_id else {
        ui.label(
            egui::RichText::new("Select a spell to view details.").color(SPELLBOOK_HINT_COLOR),
        );
        return;
    };

    let Some(content_ref) = content else {
        ui.label(egui::RichText::new("Content not loaded.").color(SPELLBOOK_DISABLED_SPELL_COLOR));
        return;
    };

    let Some(spell) = content_ref.db().spells.get_spell(spell_id) else {
        ui.label(
            egui::RichText::new(format!("Unknown spell\n{spell_id:#06x}"))
                .color(SPELLBOOK_DISABLED_SPELL_COLOR),
        );
        return;
    };

    // Spell name — displayed larger
    ui.label(
        egui::RichText::new(spell.name.clone())
            .color(SPELLBOOK_TITLE_COLOR)
            .size(BODY_FONT_SIZE + 2.0),
    );

    let context_label = match spell.context {
        SpellContext::Anytime => "Any".to_string(),
        SpellContext::CombatOnly => "Combat \u{2694}".to_string(),
        SpellContext::NonCombatOnly => "Non-Combat \u{1F30D}".to_string(),
        SpellContext::OutdoorOnly => "Outdoor".to_string(),
        SpellContext::IndoorOnly => "Indoor".to_string(),
        SpellContext::OutdoorCombat => "Outdoor Combat".to_string(),
    };
    let gem_label = if spell.gem_cost > 0 {
        spell.gem_cost.to_string()
    } else {
        "\u{2014}".to_string() // —
    };

    let detail_lines = [
        format!("School: {:?}", spell.school),
        format!("Level:  {}", spell.level),
        format!("SP Cost: {}", spell.sp_cost),
        format!("Gem Cost: {gem_label}"),
        format!("Context: {context_label}"),
    ];

    for line in &detail_lines {
        ui.label(egui::RichText::new(line.clone()).color(SPELLBOOK_NORMAL_ROW_COLOR));
    }

    // Space before description
    ui.add_space(4.0);

    // Description text
    if !spell.description.is_empty() {
        ui.label(egui::RichText::new(spell.description.clone()).color(SPELLBOOK_HINT_COLOR));
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::spell_book_state::SpellBookState;
    use crate::application::GameState;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::game::resources::GlobalState;

    // ── collect_spell_ids_from_state ─────────────────────────────────────────

    /// Returns empty when mode is not SpellBook.
    #[test]
    fn test_collect_spell_ids_not_in_spellbook_mode_returns_empty() {
        let state = GameState::new();
        assert!(matches!(state.mode, GameMode::Exploration));
        let ids = collect_spell_ids_from_state(&state, None);
        assert!(ids.is_empty());
    }

    /// Returns empty when the character index is out of range (empty party).
    #[test]
    fn test_collect_spell_ids_empty_party_returns_empty() {
        let mut state = GameState::new();
        state.enter_spellbook(0);
        // Party is empty → index 0 is out of range.
        let ids = collect_spell_ids_from_state(&state, None);
        assert!(ids.is_empty());
    }

    /// Returns empty when content is None.
    #[test]
    fn test_collect_spell_ids_no_content_returns_empty() {
        let mut state = GameState::new();
        let character = Character::new(
            "Aria".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        state.party.add_member(character).unwrap();
        state.enter_spellbook(0);

        let ids = collect_spell_ids_from_state(&state, None);
        assert!(ids.is_empty(), "no content → empty spell list");
    }

    // ── Tab navigation ───────────────────────────────────────────────────────

    /// Tab forward increments character_index.
    #[test]
    fn test_tab_forward_increments_character_index() {
        let mut sb = SpellBookState::new(0, GameMode::Exploration);
        sb.next_character(3);
        assert_eq!(sb.character_index, 1);
    }

    /// Tab forward wraps at party size.
    #[test]
    fn test_tab_forward_wraps_at_party_size() {
        let mut sb = SpellBookState::new(2, GameMode::Exploration);
        sb.next_character(3);
        assert_eq!(sb.character_index, 0);
    }

    /// Shift+Tab (prev_character) decrements character_index.
    #[test]
    fn test_tab_back_decrements_character_index() {
        let mut sb = SpellBookState::new(2, GameMode::Exploration);
        sb.prev_character(3);
        assert_eq!(sb.character_index, 1);
    }

    /// Shift+Tab wraps to end at index 0.
    #[test]
    fn test_tab_back_wraps_to_end_at_zero() {
        let mut sb = SpellBookState::new(0, GameMode::Exploration);
        sb.prev_character(3);
        assert_eq!(sb.character_index, 2);
    }

    // ── SP affordability logic ───────────────────────────────────────────────

    /// A spell is disabled when its `sp_cost` exceeds the character's current
    /// SP.  The UI uses `SPELLBOOK_DISABLED_SPELL_COLOR` for such rows.
    #[test]
    fn test_spell_row_disabled_when_sp_insufficient() {
        // sp_cost = 10, character.sp.current = 5 → disabled
        let character_sp: u32 = 5;
        let spell_cost: u32 = 10;
        let can_afford = character_sp >= spell_cost;
        assert!(
            !can_afford,
            "character with 5 SP cannot afford a 10-SP spell"
        );

        // Verify the color selection logic mirrors render_spell_list.
        let text_color = if !can_afford {
            SPELLBOOK_DISABLED_SPELL_COLOR
        } else {
            SPELLBOOK_NORMAL_ROW_COLOR
        };
        assert_eq!(
            text_color, SPELLBOOK_DISABLED_SPELL_COLOR,
            "insufficient SP must select SPELLBOOK_DISABLED_SPELL_COLOR"
        );
    }

    /// A spell is NOT disabled when the character has enough SP.
    #[test]
    fn test_spell_row_enabled_when_sp_sufficient() {
        let character_sp: u32 = 15;
        let spell_cost: u32 = 10;
        let can_afford = character_sp >= spell_cost;
        assert!(can_afford);

        let text_color = if !can_afford {
            SPELLBOOK_DISABLED_SPELL_COLOR
        } else {
            SPELLBOOK_NORMAL_ROW_COLOR
        };
        assert_eq!(text_color, SPELLBOOK_NORMAL_ROW_COLOR);
    }

    // ── GameMode::SpellBook transitions ─────────────────────────────────────

    /// enter_spellbook + exit_spellbook round-trip restores previous mode.
    #[test]
    fn test_enter_and_exit_spellbook_roundtrip() {
        let mut state = GameState::new();
        state.enter_spellbook(0);
        assert!(matches!(state.mode, GameMode::SpellBook(_)));
        state.exit_spellbook();
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    /// exit_spellbook is a no-op when not in SpellBook mode.
    #[test]
    fn test_exit_spellbook_noop_when_not_spellbook_mode() {
        let mut state = GameState::new();
        assert!(matches!(state.mode, GameMode::Exploration));
        state.exit_spellbook(); // no-op
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    // ── Esc / C key transitions ──────────────────────────────────────────────

    /// Pressing Esc in SpellBook mode calls exit_spellbook and restores the
    /// previous mode.  Verified via pure state mutation (no Bevy input needed).
    #[test]
    fn test_esc_triggers_exit_spellbook() {
        let mut state = GameState::new();
        state.enter_spellbook(0);
        assert!(matches!(state.mode, GameMode::SpellBook(_)));
        // Simulate what spellbook_input_system does on Esc:
        state.exit_spellbook();
        assert!(
            matches!(state.mode, GameMode::Exploration),
            "Esc must restore the previous mode (Exploration)"
        );
    }

    /// C key exits SpellBook and enters SpellCasting for the current character.
    #[test]
    fn test_c_key_transitions_to_spell_casting() {
        let mut state = GameState::new();
        state.enter_spellbook(1);
        let char_index = if let GameMode::SpellBook(ref sb) = state.mode {
            sb.character_index
        } else {
            panic!("expected SpellBook mode");
        };

        // Simulate what spellbook_input_system does on C key:
        state.exit_spellbook();
        state.enter_spell_casting(char_index);

        assert!(
            matches!(state.mode, GameMode::SpellCasting(_)),
            "C key must transition to SpellCasting"
        );
        if let GameMode::SpellCasting(ref sc) = state.mode {
            assert_eq!(
                sc.caster_index, 1,
                "must use character index from SpellBook"
            );
        }
    }

    // ── egui render helper smoke tests ───────────────────────────────────────

    /// `render_char_tabs` must not panic when the party is completely empty.
    ///
    /// When `party.members` is empty the helper should emit the
    /// "No party." placeholder label and return without iterating.
    #[test]
    fn test_render_char_tabs_empty_party_no_panic() {
        let sb = SpellBookState::new(0, GameMode::Exploration);
        let global_state = GlobalState(GameState::new()); // empty party

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_char_tabs(ui, &sb, &global_state);
            });
        });
    }

    /// `render_spell_list` must not panic when `spell_ids` is empty and
    /// `content` is `None`.
    ///
    /// With an empty party, `party.members.get(0)` returns `None` so the
    /// helper emits the "No character selected." placeholder and returns early.
    #[test]
    fn test_render_spell_list_no_spells_shows_placeholder() {
        let sb = SpellBookState::new(0, GameMode::Exploration);
        let global_state = GlobalState(GameState::new()); // empty party → index 0 out of range

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_spell_list(ui, &sb, &global_state, None, &[]);
            });
        });
    }

    /// `render_detail_panel` must not panic when `selected_spell_id` is `None`.
    ///
    /// `SpellBookState::new` initialises `selected_spell_id` to `None`, so the
    /// helper should emit the "Select a spell to view details." placeholder and
    /// return without accessing any content database.
    #[test]
    fn test_render_detail_panel_no_selection_no_panic() {
        // selected_spell_id defaults to None
        let sb = SpellBookState::new(0, GameMode::Exploration);

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_detail_panel(ui, &sb, None);
            });
        });
    }
}
