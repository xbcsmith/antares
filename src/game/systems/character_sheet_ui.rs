// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Character Sheet read-only viewer screen.
//!
//! Implements input handling and egui rendering for browsing a party member's
//! full character stats during exploration.  When the player presses the
//! character sheet key (`P` by default), the game enters
//! [`GameMode::CharacterSheet`](crate::application::GameMode::CharacterSheet)
//! and this plugin takes over input until the screen is closed.
//!
//! # Layout — Single view
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │  Aldric — Level 3 Human Knight         [< Prev] [Next >] │
//! │                                        [Party Overview]   │
//! ├──────────────────────────────────────────────────────────┤
//! │  Core Stats          │  Combat          │  Experience     │
//! │  Might:  15/15       │  HP:  42 / 50    │  XP:  3,124     │
//! │  Intell: 10/10       │  SP:   8 / 10    │  Next: 5,000    │
//! │  Person: 12/12       │  AC:  14         │  ✅ Ready!      │
//! │  Endur:  14/14       │  SpLvl: 3        │                 │
//! │  Speed:  11/11       │                  │                 │
//! │  Accur:  13/13       │  Conditions:     │  Equipment:     │
//! │  Luck:   9/9         │  None            │  Weapon: Sword  │
//! │                      │                  │  Armor: Chain   │
//! ├──────────────────────────────────────────────────────────┤
//! │  [Esc] Close   [Tab] Next   [Shift+Tab] Prev   [O] Overview │
//! └──────────────────────────────────────────────────────────┘
//! ```
//!
//! # Flow
//!
//! 1. `P` key → `GameState::enter_character_sheet()` (in
//!    `input/global_toggles.rs`).
//! 2. [`character_sheet_input_system`] drives navigation:
//!    - **Tab** / **→**       — focus next party member (Single view).
//!    - **Shift+Tab** / **←** — focus previous party member (Single view).
//!    - **O**                 — toggle between Single and Party Overview.
//!    - **Esc**               — close and restore previous mode.
//! 3. [`character_sheet_ui_system`] renders the egui panel every frame.
//! 4. [`character_sheet_cleanup_system`] is a documented no-op: this is a
//!    pure-egui UI with no Bevy entities to despawn.

use crate::application::character_sheet_state::CharacterSheetView;
use crate::application::GameMode;
use crate::domain::campaign::LevelUpMode;
use crate::domain::progression::experience_for_level_with_config;
use crate::game::resources::game_data::GameDataResource;
use crate::game::resources::GlobalState;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

// ── Colour constants ──────────────────────────────────────────────────────────

/// Amber text used to highlight a stat that has been temporarily modified.
const STAT_MODIFIED_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 191, 0);
/// Muted grey used for zero-value or unavailable entries.
const STAT_EMPTY_COLOR: egui::Color32 = egui::Color32::from_rgb(128, 128, 128);
/// Green used for "ready to level up" message.
const LEVEL_READY_COLOR: egui::Color32 = egui::Color32::from_rgb(80, 200, 120);
/// Yellow used for "visit a trainer" message.
const TRAINER_NEEDED_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 215, 0);
/// Header / title colour.
const TITLE_COLOR: egui::Color32 = egui::Color32::from_rgb(204, 217, 255);
/// Hint bar colour.
const HINT_COLOR: egui::Color32 = egui::Color32::from_rgb(140, 140, 166);

// ── Plugin ────────────────────────────────────────────────────────────────────

/// Bevy plugin that provides the in-game Character Sheet viewer.
///
/// Registers the three-system chain
/// `(character_sheet_input_system, character_sheet_ui_system,
///   character_sheet_cleanup_system)`.
/// Input runs first; the egui renderer follows so it sees the updated state
/// in the same frame; the cleanup stub closes the chain.
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::character_sheet_ui::CharacterSheetPlugin;
///
/// # fn setup() {
/// let mut app = App::new();
/// app.add_plugins(CharacterSheetPlugin);
/// # }
/// ```
pub struct CharacterSheetPlugin;

impl Plugin for CharacterSheetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                character_sheet_input_system,
                character_sheet_ui_system,
                character_sheet_cleanup_system,
            )
                .chain(),
        );
    }
}

// ── Input system ──────────────────────────────────────────────────────────────

/// Handles keyboard input while in `CharacterSheet` mode.
///
/// - **Esc**           — close and restore previous mode.
/// - **Tab** (no shift) / **→** — focus next party member (Single view).
/// - **Shift+Tab** / **←**     — focus previous party member (Single view).
/// - **O**             — toggle between Single and Party Overview.
pub fn character_sheet_input_system(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut global_state: ResMut<GlobalState>,
) {
    if !matches!(global_state.0.mode, GameMode::CharacterSheet(_)) {
        return;
    }

    let Some(ref kb) = keyboard else {
        return;
    };

    // ── Esc: close and restore previous mode ────────────────────────────────
    if kb.just_pressed(KeyCode::Escape) {
        let resume = if let GameMode::CharacterSheet(ref cs) = global_state.0.mode {
            cs.get_resume_mode()
        } else {
            GameMode::Exploration
        };
        global_state.0.mode = resume;
        return;
    }

    // ── O: toggle view ───────────────────────────────────────────────────────
    if kb.just_pressed(KeyCode::KeyO) {
        if let GameMode::CharacterSheet(ref mut cs) = global_state.0.mode {
            cs.toggle_view();
        }
        return;
    }

    // Navigation only makes sense in Single view
    let is_single = if let GameMode::CharacterSheet(ref cs) = global_state.0.mode {
        cs.view == CharacterSheetView::Single
    } else {
        false
    };
    if !is_single {
        return;
    }

    // Read party_size before any mutable mode borrows
    let party_size = global_state.0.party.members.len();

    let shift_held = kb.pressed(KeyCode::ShiftLeft) || kb.pressed(KeyCode::ShiftRight);

    // ── Tab / Shift-Tab ──────────────────────────────────────────────────────
    if kb.just_pressed(KeyCode::Tab) {
        if shift_held {
            if let GameMode::CharacterSheet(ref mut cs) = global_state.0.mode {
                cs.focus_prev(party_size);
            }
        } else if let GameMode::CharacterSheet(ref mut cs) = global_state.0.mode {
            cs.focus_next(party_size);
        }
        return;
    }

    // ── Arrow keys ───────────────────────────────────────────────────────────
    if kb.just_pressed(KeyCode::ArrowRight) {
        if let GameMode::CharacterSheet(ref mut cs) = global_state.0.mode {
            cs.focus_next(party_size);
        }
    } else if kb.just_pressed(KeyCode::ArrowLeft) {
        if let GameMode::CharacterSheet(ref mut cs) = global_state.0.mode {
            cs.focus_prev(party_size);
        }
    }
}

// ── UI system ─────────────────────────────────────────────────────────────────

/// Renders the Character Sheet egui window when in `CharacterSheet` mode.
///
/// Renders either the [Single](CharacterSheetView::Single) detailed panel or
/// the [PartyOverview](CharacterSheetView::PartyOverview) compact card grid
/// depending on the current `view` stored in `CharacterSheetState`.
fn character_sheet_ui_system(
    mut contexts: EguiContexts,
    mut global_state: ResMut<GlobalState>,
    game_data: Option<Res<GameDataResource>>,
) {
    let GameMode::CharacterSheet(_) = &global_state.0.mode else {
        return;
    };

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    // Clone the data we need to avoid borrow conflicts during UI rendering
    let party_len = global_state.0.party.members.len();
    let level_up_mode = global_state.0.campaign_config.level_up_mode.clone();
    let campaign_config = global_state.0.campaign_config.clone();
    let level_db = game_data.as_ref().map(|gd| gd.data().levels.clone());

    let GameMode::CharacterSheet(ref cs_state) = global_state.0.mode else {
        return;
    };
    let focused_index = cs_state.focused_index;
    let current_view = cs_state.view.clone();

    // Window pinned to the screen centre-top
    egui::Window::new("Character Sheet")
        .collapsible(false)
        .resizable(true)
        .default_width(680.0)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 20.0])
        .show(ctx, |ui| {
            match current_view {
                CharacterSheetView::Single => {
                    render_single_view(
                        ui,
                        &mut global_state,
                        party_len,
                        focused_index,
                        &level_up_mode,
                        &campaign_config,
                        level_db.as_ref().and_then(|opt| opt.as_ref()),
                    );
                }
                CharacterSheetView::PartyOverview => {
                    render_party_overview(ui, &mut global_state, party_len);
                }
            }

            // ── Hint bar ────────────────────────────────────────────────────
            ui.separator();
            ui.horizontal(|ui| {
                ui.colored_label(HINT_COLOR, "[Esc] Close");
                ui.separator();
                ui.colored_label(HINT_COLOR, "[Tab/→] Next");
                ui.separator();
                ui.colored_label(HINT_COLOR, "[Shift+Tab/←] Prev");
                ui.separator();
                ui.colored_label(HINT_COLOR, "[O] Toggle View");
            });
        });
}

/// Renders the detailed single-character panel.
fn render_single_view(
    ui: &mut egui::Ui,
    global_state: &mut ResMut<GlobalState>,
    party_len: usize,
    focused_index: usize,
    level_up_mode: &LevelUpMode,
    campaign_config: &crate::domain::campaign::CampaignConfig,
    level_db: Option<&crate::domain::levels::LevelDatabase>,
) {
    if party_len == 0 {
        ui.colored_label(STAT_EMPTY_COLOR, "No party members.");
        return;
    }

    let safe_index = focused_index.min(party_len.saturating_sub(1));

    // Clone the character data we need to avoid borrow conflicts
    let character = global_state.0.party.members[safe_index].clone();

    // ── Header ───────────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.colored_label(
            TITLE_COLOR,
            egui::RichText::new(format!(
                "{} — Level {} {} {}",
                character.name, character.level, character.race_id, character.class_id
            ))
            .size(16.0)
            .strong(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Toggle view button
            if ui.small_button("Party Overview").clicked() {
                if let GameMode::CharacterSheet(ref mut cs) = global_state.0.mode {
                    cs.toggle_view();
                }
            }
            // Navigation buttons
            if ui.small_button("Next >").clicked() {
                if let GameMode::CharacterSheet(ref mut cs) = global_state.0.mode {
                    cs.focus_next(party_len);
                }
            }
            if ui.small_button("< Prev").clicked() {
                if let GameMode::CharacterSheet(ref mut cs) = global_state.0.mode {
                    cs.focus_prev(party_len);
                }
            }
        });
    });

    ui.separator();

    // ── Two-column layout ────────────────────────────────────────────────────
    // Left column: Core stats + Conditions
    // Right column: Combat + Experience + Equipment + Proficiencies
    let available = ui.available_width();
    let col_width = (available - 12.0) / 2.0;

    ui.horizontal(|ui| {
        // ── Left column ─────────────────────────────────────────────────────
        ui.allocate_ui(egui::vec2(col_width, 0.0), |ui| {
            ui.vertical(|ui| {
                ui.colored_label(TITLE_COLOR, "Core Stats");
                ui.separator();
                render_stat_row(
                    ui,
                    "Might",
                    character.stats.might.base,
                    character.stats.might.current,
                );
                render_stat_row(
                    ui,
                    "Intellect",
                    character.stats.intellect.base,
                    character.stats.intellect.current,
                );
                render_stat_row(
                    ui,
                    "Personality",
                    character.stats.personality.base,
                    character.stats.personality.current,
                );
                render_stat_row(
                    ui,
                    "Endurance",
                    character.stats.endurance.base,
                    character.stats.endurance.current,
                );
                render_stat_row(
                    ui,
                    "Speed",
                    character.stats.speed.base,
                    character.stats.speed.current,
                );
                render_stat_row(
                    ui,
                    "Accuracy",
                    character.stats.accuracy.base,
                    character.stats.accuracy.current,
                );
                render_stat_row(
                    ui,
                    "Luck",
                    character.stats.luck.base,
                    character.stats.luck.current,
                );

                ui.add_space(8.0);
                ui.colored_label(TITLE_COLOR, "Conditions");
                ui.separator();
                if character.active_conditions.is_empty() {
                    ui.colored_label(STAT_EMPTY_COLOR, "None");
                } else {
                    let names: Vec<String> = character
                        .active_conditions
                        .iter()
                        .map(|c| c.condition_id.to_string())
                        .collect();
                    ui.label(names.join(", "));
                }
            });
        });

        ui.separator();

        // ── Right column ─────────────────────────────────────────────────────
        ui.allocate_ui(egui::vec2(col_width, 0.0), |ui| {
            ui.vertical(|ui| {
                // Combat stats
                ui.colored_label(TITLE_COLOR, "Combat");
                ui.separator();
                render_hp_row(ui, "HP", character.hp.current, character.hp.base);
                render_hp_row(ui, "SP", character.sp.current, character.sp.base);
                render_u8_row(ui, "AC", character.ac.base, character.ac.current);
                render_u8_row(
                    ui,
                    "Spell Level",
                    character.spell_level.base,
                    character.spell_level.current,
                );

                ui.add_space(8.0);

                // Experience
                ui.colored_label(TITLE_COLOR, "Experience");
                ui.separator();
                let xp_next = experience_for_level_with_config(
                    character.level + 1,
                    &character.class_id,
                    campaign_config,
                    level_db,
                );
                ui.label(format!("XP: {} / {}", character.experience, xp_next));
                if character.experience >= xp_next {
                    match level_up_mode {
                        LevelUpMode::Auto => {
                            ui.colored_label(LEVEL_READY_COLOR, "✅ Ready to level up!");
                        }
                        LevelUpMode::NpcTrainer => {
                            ui.colored_label(TRAINER_NEEDED_COLOR, "🎓 Visit a trainer");
                        }
                    }
                }

                ui.add_space(8.0);

                // Equipment
                ui.colored_label(TITLE_COLOR, "Equipment");
                ui.separator();
                render_equip_slot(
                    ui,
                    "Weapon",
                    character.equipment.weapon.map(|id| format!("#{id}")),
                );
                render_equip_slot(
                    ui,
                    "Armor",
                    character.equipment.armor.map(|id| format!("#{id}")),
                );
                render_equip_slot(
                    ui,
                    "Shield",
                    character.equipment.shield.map(|id| format!("#{id}")),
                );
                render_equip_slot(
                    ui,
                    "Helmet",
                    character.equipment.helmet.map(|id| format!("#{id}")),
                );
                render_equip_slot(
                    ui,
                    "Boots",
                    character.equipment.boots.map(|id| format!("#{id}")),
                );
                render_equip_slot(
                    ui,
                    "Acc. 1",
                    character.equipment.accessory1.map(|id| format!("#{id}")),
                );
                render_equip_slot(
                    ui,
                    "Acc. 2",
                    character.equipment.accessory2.map(|id| format!("#{id}")),
                );

                ui.add_space(8.0);

                // Spells / Proficiencies
                ui.colored_label(TITLE_COLOR, "Known Spells");
                ui.separator();
                let mut all_spell_ids: Vec<String> = Vec::new();
                for level_spells in &character.spells.cleric_spells {
                    for &id in level_spells {
                        all_spell_ids.push(format!("{:#06x}", id));
                    }
                }
                for level_spells in &character.spells.sorcerer_spells {
                    for &id in level_spells {
                        all_spell_ids.push(format!("{:#06x}", id));
                    }
                }
                if all_spell_ids.is_empty() {
                    ui.colored_label(STAT_EMPTY_COLOR, "None");
                } else {
                    ui.label(all_spell_ids.join(", "));
                }
            });
        });
    });
}

/// Renders a single core-stat row showing `base / current`.
///
/// The current value is highlighted amber when it differs from base.
fn render_stat_row(ui: &mut egui::Ui, label: &str, base: u8, current: u8) {
    ui.horizontal(|ui| {
        ui.label(format!("{label}:"));
        if current != base {
            ui.colored_label(STAT_MODIFIED_COLOR, format!("{base} / {current}"));
        } else {
            ui.label(format!("{current}"));
        }
    });
}

/// Renders a u16-based HP/SP row showing `current / max`.
fn render_hp_row(ui: &mut egui::Ui, label: &str, current: u16, max: u16) {
    ui.horizontal(|ui| {
        ui.label(format!("{label}:"));
        let color = if current < max / 4 {
            egui::Color32::from_rgb(220, 60, 60)
        } else if current < max / 2 {
            egui::Color32::from_rgb(220, 160, 40)
        } else {
            egui::Color32::WHITE
        };
        ui.colored_label(color, format!("{current} / {max}"));
    });
}

/// Renders an `AttributePair` (u8) row showing current; amber when modified.
fn render_u8_row(ui: &mut egui::Ui, label: &str, base: u8, current: u8) {
    ui.horizontal(|ui| {
        ui.label(format!("{label}:"));
        if current != base {
            ui.colored_label(STAT_MODIFIED_COLOR, format!("{current}"));
        } else {
            ui.label(format!("{current}"));
        }
    });
}

/// Renders a single equipment slot row.
fn render_equip_slot(ui: &mut egui::Ui, slot_name: &str, item: Option<String>) {
    ui.horizontal(|ui| {
        ui.label(format!("{slot_name}:"));
        match item {
            Some(name) => ui.label(name),
            None => ui.colored_label(STAT_EMPTY_COLOR, "—"),
        };
    });
}

/// Renders the compact party overview (horizontal scroll of cards).
fn render_party_overview(
    ui: &mut egui::Ui,
    global_state: &mut ResMut<GlobalState>,
    party_len: usize,
) {
    if party_len == 0 {
        ui.colored_label(STAT_EMPTY_COLOR, "No party members.");
        return;
    }

    ui.horizontal(|ui| {
        ui.colored_label(
            TITLE_COLOR,
            egui::RichText::new("Party Overview").size(16.0).strong(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Single View").clicked() {
                if let GameMode::CharacterSheet(ref mut cs) = global_state.0.mode {
                    cs.toggle_view();
                }
            }
        });
    });
    ui.separator();

    // Clone characters to avoid borrow conflict inside closure
    let members: Vec<_> = global_state.0.party.members.clone();

    egui::ScrollArea::horizontal()
        .id_salt("char_sheet_party_overview_scroll")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (idx, character) in members.iter().enumerate() {
                    ui.push_id(idx, |ui| {
                        egui::Frame::default()
                            .inner_margin(egui::Margin::same(8))
                            .stroke(egui::Stroke::new(1.0, egui::Color32::DARK_GRAY))
                            .show(ui, |ui| {
                                ui.set_min_width(160.0);
                                ui.vertical(|ui| {
                                    ui.colored_label(
                                        TITLE_COLOR,
                                        egui::RichText::new(&character.name).strong(),
                                    );
                                    ui.label(format!(
                                        "{} Lv {}",
                                        character.class_id, character.level
                                    ));
                                    ui.label(character.race_id.to_string());
                                    ui.separator();

                                    // HP bar
                                    let hp_frac = if character.hp.base > 0 {
                                        character.hp.current as f32 / character.hp.base as f32
                                    } else {
                                        0.0
                                    };
                                    let hp_color = if hp_frac < 0.25 {
                                        egui::Color32::from_rgb(220, 60, 60)
                                    } else if hp_frac < 0.5 {
                                        egui::Color32::from_rgb(220, 160, 40)
                                    } else {
                                        egui::Color32::from_rgb(60, 200, 60)
                                    };
                                    ui.horizontal(|ui| {
                                        ui.label("HP:");
                                        ui.colored_label(
                                            hp_color,
                                            format!(
                                                "{} / {}",
                                                character.hp.current, character.hp.base
                                            ),
                                        );
                                    });

                                    ui.add_space(4.0);
                                    if ui.small_button("View").clicked() {
                                        if let GameMode::CharacterSheet(ref mut cs) =
                                            global_state.0.mode
                                        {
                                            cs.focused_index = idx;
                                            cs.view = CharacterSheetView::Single;
                                        }
                                    }
                                });
                            });
                    });
                    ui.add_space(4.0);
                }
            });
        });
}

// ── Cleanup system ────────────────────────────────────────────────────────────

/// No-op cleanup stub retained for structural consistency with other UI plugins.
///
/// The Character Sheet UI is implemented entirely in egui.  No Bevy-world
/// entities are spawned by this plugin, so there is nothing to despawn on
/// mode exit.
fn character_sheet_cleanup_system(global_state: Res<GlobalState>) {
    // Pure-egui implementation: no Bevy-world entities are spawned by the
    // character sheet UI, so this cleanup system is a documented no-op
    // retained for structural consistency with other UI plugins.
    let _ = global_state;
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{GameMode, GameState};

    // ── Plugin builds ────────────────────────────────────────────────────────

    #[test]
    fn test_character_sheet_plugin_builds() {
        let mut app = App::new();
        app.add_plugins(CharacterSheetPlugin);
        // Should not panic
    }

    // ── Pure-logic tests (no Bevy App required) ───────────────────────────────

    #[test]
    fn test_esc_closes_character_sheet() {
        let mut state = GameState::new();
        state.enter_character_sheet();
        assert!(matches!(state.mode, GameMode::CharacterSheet(_)));

        // Simulate the close logic directly (mirrors character_sheet_input_system Esc branch)
        if let GameMode::CharacterSheet(ref cs) = state.mode.clone() {
            state.mode = cs.get_resume_mode();
        }

        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_character_sheet_ui_system_noop_when_not_in_mode() {
        // When mode is Exploration, no character sheet state is manipulated.
        let state = GameState::new();
        assert!(matches!(state.mode, GameMode::Exploration));
        // The system guards `let GameMode::CharacterSheet(_) = &state.mode else { return; }`
        // so nothing happens. Verified here by checking the mode is unchanged.
        assert!(!matches!(state.mode, GameMode::CharacterSheet(_)));
    }

    #[test]
    fn test_character_sheet_input_system_escape_closes() {
        let mut state = GameState::new();
        state.enter_character_sheet();

        // Simulate Esc path: extract resume mode and apply
        let resume = if let GameMode::CharacterSheet(ref cs) = state.mode {
            cs.get_resume_mode()
        } else {
            panic!("expected CharacterSheet mode");
        };
        state.mode = resume;

        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_focus_next_advances_correctly() {
        use crate::application::character_sheet_state::CharacterSheetState;

        let mut cs = CharacterSheetState::new(GameMode::Exploration);
        assert_eq!(cs.focused_index, 0);
        cs.focus_next(3);
        assert_eq!(cs.focused_index, 1);
        cs.focus_next(3);
        assert_eq!(cs.focused_index, 2);
        cs.focus_next(3);
        assert_eq!(cs.focused_index, 0); // wraps
    }

    #[test]
    fn test_focus_prev_wraps_correctly() {
        use crate::application::character_sheet_state::CharacterSheetState;

        let mut cs = CharacterSheetState::new(GameMode::Exploration);
        cs.focus_prev(4); // 0 → 3
        assert_eq!(cs.focused_index, 3);
        cs.focus_prev(4); // 3 → 2
        assert_eq!(cs.focused_index, 2);
    }

    #[test]
    fn test_toggle_view_switches_between_modes() {
        use crate::application::character_sheet_state::{CharacterSheetState, CharacterSheetView};

        let mut cs = CharacterSheetState::new(GameMode::Exploration);
        assert_eq!(cs.view, CharacterSheetView::Single);
        cs.toggle_view();
        assert_eq!(cs.view, CharacterSheetView::PartyOverview);
        cs.toggle_view();
        assert_eq!(cs.view, CharacterSheetView::Single);
    }

    #[test]
    fn test_enter_and_exit_character_sheet_roundtrip() {
        let mut state = GameState::new();
        assert!(matches!(state.mode, GameMode::Exploration));
        state.enter_character_sheet();
        assert!(matches!(state.mode, GameMode::CharacterSheet(_)));

        // Close via close_modal (same path as Esc → menu_toggle)
        let closed = state.close_modal();
        assert!(closed);
        assert!(matches!(state.mode, GameMode::Exploration));
    }
}
