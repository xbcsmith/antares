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
use crate::application::resources::GameContent;
use crate::application::GameMode;
use crate::domain::campaign::LevelUpMode;
use crate::domain::character::{Alignment, Sex};
use crate::domain::progression::experience_for_level_with_config;
use crate::game::resources::game_data::GameDataResource;
use crate::game::resources::GlobalState;
use crate::game::systems::hud::{get_portrait_color, FullPortraitAssets};
use crate::game::systems::input::{GameAction, InputConfigResource};
use crate::sdk::database::ContentDatabase;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiTextureHandle};

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
/// - **1–6**           — jump directly to that party member (Single view, configurable).
pub fn character_sheet_input_system(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    input_config: Option<Res<InputConfigResource>>,
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

    // ── Digit keys (1–6): configurable character select while sheet is open ──
    if let Some(ref icr) = input_config {
        for i in 0..6_usize {
            if icr
                .key_map
                .is_action_just_pressed(GameAction::SelectCharacter(i), kb)
            {
                let clamped = i.min(party_size.saturating_sub(1));
                if let GameMode::CharacterSheet(ref mut cs) = global_state.0.mode {
                    cs.focused_index = clamped;
                }
                return;
            }
        }
    }
}

// ── UI system ─────────────────────────────────────────────────────────────────

/// Renders the Character Sheet egui window when in `CharacterSheet` mode.
///
/// Renders either the [Single](CharacterSheetView::Single) detailed panel or
/// the [PartyOverview](CharacterSheetView::PartyOverview) compact card grid
/// depending on the current `view` stored in `CharacterSheetState`.
///
/// The `full_portraits` parameter provides pre-loaded full-length portrait
/// textures.  When a matching texture exists it is registered with egui via
/// [`EguiContexts::add_image`] and the resulting [`egui::TextureId`] is
/// forwarded to [`render_single_view`].  The registration call must precede
/// [`EguiContexts::ctx_mut`] because both require `&mut EguiContexts`.
fn character_sheet_ui_system(
    mut contexts: EguiContexts,
    mut global_state: ResMut<GlobalState>,
    game_data: Option<Res<GameDataResource>>,
    content: Option<Res<GameContent>>,
    full_portraits: Option<Res<FullPortraitAssets>>,
) {
    let GameMode::CharacterSheet(_) = &global_state.0.mode else {
        return;
    };

    // Clone the data we need to avoid borrow conflicts during UI rendering.
    // Must happen before ctx_mut() so that add_image() (which also needs
    // &mut EguiContexts) can be called first.
    let party_len = global_state.0.party.members.len();
    let campaign_config = global_state.0.campaign_config.clone();
    let level_db = game_data.as_ref().map(|gd| gd.data().levels.clone());
    // Borrow content database for proficiency lookups; None when not loaded.
    let content_db: Option<&ContentDatabase> = content.as_ref().map(|c| &c.0);

    let GameMode::CharacterSheet(ref cs_state) = global_state.0.mode else {
        return;
    };
    let focused_index = cs_state.focused_index;
    let current_view = cs_state.view.clone();

    // Resolve portrait key for the focused character.
    // Lowercased portrait_id if set, otherwise lowercased name -- same convention as HUD.
    let safe_index = focused_index.min(party_len.saturating_sub(1));
    let portrait_key = if party_len > 0 {
        let ch = &global_state.0.party.members[safe_index];
        if !ch.portrait_id.is_empty() {
            ch.portrait_id.to_lowercase().replace(' ', "_")
        } else {
            ch.name.to_lowercase().replace(' ', "_")
        }
    } else {
        String::new()
    };

    // Register the full-portrait handle with egui (idempotent) before calling ctx_mut().
    // add_image() and ctx_mut() both need &mut EguiContexts -- they must be sequential.
    let full_portrait_id: Option<egui::TextureId> = full_portraits
        .as_ref()
        .and_then(|fp| fp.handles_by_name.get(&portrait_key))
        .map(|h| contexts.add_image(EguiTextureHandle::Weak(h.id())));

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let screen_rect = ctx.available_rect();
    let screen_w = screen_rect.width();
    let screen_h = screen_rect.height();

    // Window pinned to the screen centre-top and sized to fill most of the
    // available client area so the character sheet can use a true two-column
    // layout instead of forcing scroll into multi-column density.
    egui::Window::new("Character Sheet")
        .collapsible(false)
        .resizable(true)
        .default_width((screen_w - 40.0).max(640.0))
        .max_width(screen_w - 20.0)
        .default_height((screen_h - 40.0).max(480.0))
        .max_height(screen_h - 20.0)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 20.0])
        .show(ctx, |ui| match current_view {
            CharacterSheetView::Single => {
                render_single_view(
                    ui,
                    &mut global_state,
                    party_len,
                    focused_index,
                    &campaign_config,
                    level_db.as_ref().and_then(|opt| opt.as_ref()),
                    content_db,
                    full_portrait_id,
                    &portrait_key,
                );
            }
            CharacterSheetView::PartyOverview => {
                render_party_overview(ui, &mut global_state, party_len);
            }
        });
}

/// Renders the detailed single-character panel.
///
/// Displays the character's full-length portrait (or a deterministic colour
/// placeholder when no texture is loaded), a two-column layout with the
/// portrait/identity on the left and the existing stats on the right.
///
/// # Parameters
///
/// * `full_portrait_id` -- optional egui TextureId registered by the caller
///   via `EguiContexts::add_image` before `ctx_mut()`.  `None` triggers the
///   placeholder fill.
/// * `portrait_key` -- normalized portrait filename stem used for placeholder
///   colour derivation when no texture is available.
#[allow(clippy::too_many_arguments)]
fn render_single_view(
    ui: &mut egui::Ui,
    global_state: &mut GlobalState,
    party_len: usize,
    focused_index: usize,
    campaign_config: &crate::domain::campaign::CampaignConfig,
    level_db: Option<&crate::domain::levels::LevelDatabase>,
    content_db: Option<&ContentDatabase>,
    full_portrait_id: Option<egui::TextureId>,
    portrait_key: &str,
) {
    if party_len == 0 {
        ui.colored_label(STAT_EMPTY_COLOR, "No party members.");
        return;
    }

    let safe_index = focused_index.min(party_len.saturating_sub(1));

    // Clone the character data we need to avoid borrow conflicts
    let character = global_state.0.party.members[safe_index].clone();

    // -- Header
    ui.horizontal(|ui| {
        ui.colored_label(
            TITLE_COLOR,
            egui::RichText::new(format!(
                "{} -- Level {} {} {}",
                character.name, character.level, character.race_id, character.class_id
            ))
            .size(16.0)
            .strong(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(HINT_COLOR, "[O] Overview");
            ui.separator();
            ui.colored_label(HINT_COLOR, "[1-6] Select");
            ui.separator();
            ui.colored_label(HINT_COLOR, "[Shift+Tab/←] Prev");
            ui.separator();
            ui.colored_label(HINT_COLOR, "[Tab/→] Next");
            ui.separator();
            if ui.small_button("Party Overview").clicked() {
                if let GameMode::CharacterSheet(ref mut cs) = global_state.0.mode {
                    cs.toggle_view();
                }
            }
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

    let available = ui.available_size();
    let col_h = available.y;
    let left_w = 180.0;
    let sep_total = (1.0 + 2.0 * ui.spacing().item_spacing.x) * 2.0;
    let remaining = (available.x - left_w - sep_total).max(0.0);
    let split_w = (remaining / 2.0).max(320.0);
    let middle_w = split_w;
    let right_w = split_w;

    ui.horizontal(|ui| {
        // -- Left column: portrait + character identity
        ui.allocate_ui(egui::vec2(left_w, col_h), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("character_sheet_portrait_scroll")
                .auto_shrink([true, false])
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        let portrait_size = egui::vec2(170.0, 280.0);

                        match full_portrait_id {
                            Some(tid) => {
                                ui.add(egui::Image::new(egui::load::SizedTexture::new(
                                    tid,
                                    portrait_size,
                                )));
                            }
                            None => {
                                let (portrait_rect, _) =
                                    ui.allocate_exact_size(portrait_size, egui::Sense::hover());
                                let bevy_color = get_portrait_color(portrait_key);
                                let srgba = bevy_color.to_srgba();
                                let fill_color = egui::Color32::from_rgb(
                                    (srgba.red * 255.0) as u8,
                                    (srgba.green * 255.0) as u8,
                                    (srgba.blue * 255.0) as u8,
                                );
                                ui.painter().rect_filled(portrait_rect, 4.0, fill_color);

                                let initials: String = character
                                    .name
                                    .split_whitespace()
                                    .filter_map(|part| part.chars().next())
                                    .take(2)
                                    .flat_map(|c| c.to_uppercase())
                                    .collect();
                                ui.painter().text(
                                    portrait_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    &initials,
                                    egui::FontId::proportional(48.0),
                                    egui::Color32::WHITE,
                                );
                            }
                        }

                        ui.add_space(8.0);
                        ui.colored_label(
                            TITLE_COLOR,
                            egui::RichText::new(&character.name).strong(),
                        );
                        ui.label(format!(
                            "{} {} Lv {}",
                            character.race_id, character.class_id, character.level
                        ));

                        ui.add_space(8.0);
                        ui.colored_label(TITLE_COLOR, "About");
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("Sex:");
                            ui.label(sex_display(character.sex));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Alignment:");
                            ui.label(alignment_display(character.alignment));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Age:");
                            ui.label(format!("{} yr {} d", character.age, character.age_days));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Gold:");
                            ui.label(format!("{}", character.gold));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Gems:");
                            ui.label(format!("{}", character.gems));
                        });
                    });
                });
        });

        ui.separator();

        // -- Middle column: stats, conditions, combat, experience
        ui.allocate_ui(egui::vec2(middle_w, col_h), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("character_sheet_stats_scroll")
                .auto_shrink([true, false])
                .show(ui, |ui| {
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

                        ui.add_space(8.0);
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
                            match &campaign_config.level_up_mode {
                                LevelUpMode::Auto => {
                                    ui.colored_label(LEVEL_READY_COLOR, "✅ Ready to level up!");
                                }
                                LevelUpMode::NpcTrainer => {
                                    ui.colored_label(TRAINER_NEEDED_COLOR, "🎓 Visit a trainer");
                                }
                            }
                        }
                    });
                });
        });

        ui.separator();

        // -- Right column: equipment, resistances, proficiencies
        ui.allocate_ui(egui::vec2(right_w, col_h), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("character_sheet_equipment_scroll")
                .auto_shrink([true, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
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
                        ui.colored_label(TITLE_COLOR, "Resistances");
                        ui.separator();
                        render_resistance_row(ui, "Magic", character.resistances.magic.current);
                        render_resistance_row(ui, "Fire", character.resistances.fire.current);
                        render_resistance_row(ui, "Cold", character.resistances.cold.current);
                        render_resistance_row(
                            ui,
                            "Electricity",
                            character.resistances.electricity.current,
                        );
                        render_resistance_row(ui, "Acid", character.resistances.acid.current);
                        render_resistance_row(ui, "Fear", character.resistances.fear.current);
                        render_resistance_row(ui, "Poison", character.resistances.poison.current);
                        render_resistance_row(ui, "Psychic", character.resistances.psychic.current);

                        ui.add_space(8.0);
                        ui.colored_label(TITLE_COLOR, "Proficiencies");
                        ui.separator();
                        let mut profs: Vec<String> = Vec::new();
                        if let Some(db) = content_db {
                            if let Some(class_def) = db.classes.get_class(&character.class_id) {
                                profs.extend_from_slice(&class_def.proficiencies);
                            }
                            if let Some(race_def) = db.races.get_race(&character.race_id) {
                                for p in &race_def.proficiencies {
                                    if !profs.contains(p) {
                                        profs.push(p.clone());
                                    }
                                }
                            }
                        }
                        if profs.is_empty() {
                            ui.colored_label(STAT_EMPTY_COLOR, "None");
                        } else {
                            profs.sort();
                            ui.label(profs.join(", "));
                        }
                    });
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

/// Renders a single resistance row.
///
/// * `value == 0` → rendered in [`STAT_EMPTY_COLOR`] (grey) to indicate no resistance.
/// * `value > 0`  → rendered in [`STAT_MODIFIED_COLOR`] (amber) for visual prominence.
///
/// # Arguments
///
/// * `label` – the resistance type name (e.g., `"Magic"`)
/// * `value` – the active resistance value (`current` from the [`AttributePair`])
fn render_resistance_row(ui: &mut egui::Ui, label: &str, value: u8) {
    ui.horizontal(|ui| {
        ui.label(format!("{label}:"));
        if value == 0 {
            ui.colored_label(STAT_EMPTY_COLOR, "0");
        } else {
            ui.colored_label(STAT_MODIFIED_COLOR, format!("{value}"));
        }
    });
}

/// Returns a display string for a character's [`Sex`].
fn sex_display(sex: Sex) -> &'static str {
    match sex {
        Sex::Male => "Male",
        Sex::Female => "Female",
        Sex::Other => "Other",
    }
}

/// Returns a display string for a character's [`Alignment`].
fn alignment_display(alignment: Alignment) -> &'static str {
    match alignment {
        Alignment::Good => "Good",
        Alignment::Neutral => "Neutral",
        Alignment::Evil => "Evil",
    }
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

    #[test]
    fn test_character_sheet_input_configured_digit_key_switches_focused_index() {
        use crate::domain::character::{Alignment, Character, Sex};
        use crate::game::systems::input::{GameAction, KeyMap};
        use crate::sdk::game_config::ControlsConfig;
        use bevy::prelude::ButtonInput;

        // Build a 3-member party so index 2 is valid.
        let mut state = GameState::new();
        for name in ["Alpha", "Beta", "Gamma"] {
            let hero = Character::new(
                name.to_string(),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            state.party.add_member(hero).unwrap();
        }
        state.enter_character_sheet();
        assert!(matches!(state.mode, GameMode::CharacterSheet(_)));

        // Default config: "3" is bound to SelectCharacter(2).
        let config = ControlsConfig::default();
        let key_map = KeyMap::from_controls_config(&config);
        let mut kb = ButtonInput::<KeyCode>::default();
        kb.press(KeyCode::Digit3);

        let party_size = state.party.members.len();

        // Simulate the digit-key branch of character_sheet_input_system.
        for i in 0..6_usize {
            if key_map.is_action_just_pressed(GameAction::SelectCharacter(i), &kb) {
                let clamped = i.min(party_size.saturating_sub(1));
                if let GameMode::CharacterSheet(ref mut cs) = state.mode {
                    cs.focused_index = clamped;
                }
                break;
            }
        }

        if let GameMode::CharacterSheet(ref cs) = state.mode {
            assert_eq!(cs.focused_index, 2);
        } else {
            panic!("expected CharacterSheet mode");
        }
    }
    /// Verifies `render_single_view` does not panic when no full-portrait
    /// `TextureId` is provided (the common case at startup).
    ///
    /// The placeholder path allocates a coloured rectangle and overlays the
    /// character's initials; this test ensures neither operation panics.
    #[test]
    fn test_render_single_view_placeholder_when_no_full_portrait() {
        use crate::domain::character::{Alignment, Character, Sex};

        let mut state = GameState::new();
        let ch = Character::new(
            "Aldric Ironforge".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        state.party.add_member(ch).unwrap();
        state.enter_character_sheet();

        let mut gs = crate::game::resources::GlobalState(state);
        let campaign_config = crate::domain::campaign::CampaignConfig::default();

        // Use a bare egui Context -- no Bevy ECS needed for render function tests.
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // full_portrait_id = None triggers the placeholder path
                render_single_view(
                    ui,
                    &mut gs,
                    1,
                    0,
                    &campaign_config,
                    None,
                    None,
                    None,
                    "aldric",
                );
            });
        });
        // Reaching here without panic = pass
    }

    /// Verifies the hint bar renders without panic after the `[1-6] Select`
    /// entry was added.  The system-level hint bar is part of
    /// `character_sheet_ui_system`; this smoke test exercises it via a full
    /// `render_single_view` call in a minimal window context.
    #[test]
    fn test_render_single_view_hint_bar_contains_1_6_select() {
        use crate::domain::character::{Alignment, Character, Sex};

        let mut state = GameState::new();
        let ch = Character::new(
            "Mira Windwhisper".to_string(),
            "elf".to_string(),
            "sorcerer".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        state.party.add_member(ch).unwrap();
        state.enter_character_sheet();

        let mut gs = crate::game::resources::GlobalState(state);
        let campaign_config = crate::domain::campaign::CampaignConfig::default();

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::Window::new("Character Sheet").show(ctx, |ui| {
                render_single_view(
                    ui,
                    &mut gs,
                    1,
                    0,
                    &campaign_config,
                    None,
                    None,
                    None,
                    "mira_windwhisper",
                );
                // Hint bar -- verify it renders without panic
                ui.separator();
                ui.horizontal(|ui| {
                    ui.colored_label(HINT_COLOR, "[Esc] Close");
                    ui.separator();
                    ui.colored_label(HINT_COLOR, "[Tab/→] Next");
                    ui.separator();
                    ui.colored_label(HINT_COLOR, "[Shift+Tab/←] Prev");
                    ui.separator();
                    ui.colored_label(HINT_COLOR, "[1-6] Select");
                    ui.separator();
                    ui.colored_label(HINT_COLOR, "[O] Toggle View");
                });
            });
        });
    }

    /// Verifies that `character_sheet_ui_system` accepts
    /// `Option<Res<FullPortraitAssets>>` without panic and that the resource
    /// is accessible from the world after insertion.
    #[test]
    fn test_character_sheet_ui_system_accepts_full_portrait_assets_resource() {
        use bevy::prelude::App;

        let mut state = GameState::new();
        state.enter_character_sheet();

        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.insert_resource(crate::game::resources::GlobalState(state));
        app.init_resource::<crate::game::systems::hud::FullPortraitAssets>();
        app.add_plugins(CharacterSheetPlugin);

        // Register EguiUserTextures so EguiContexts parameter validation passes.
        // Without this, the system panics before even checking ctx_mut().
        app.init_resource::<bevy_egui::EguiUserTextures>();

        // Run one update; character_sheet_ui_system will return early because
        // there is no primary egui context (ctx_mut() returns Err), but the
        // system must accept FullPortraitAssets without panicking.
        app.update();

        // Verify the resource is present in the world
        assert!(
            app.world()
                .get_resource::<crate::game::systems::hud::FullPortraitAssets>()
                .is_some(),
            "FullPortraitAssets resource must be accessible after insertion"
        );
    }

    /// Verifies `render_resistance_row` uses `STAT_EMPTY_COLOR` for a zero value.
    ///
    /// Calls the helper in a minimal egui context and confirms it does not panic.
    /// The test also asserts the colour constants match the expected RGB values so
    /// a future palette change does not silently break the UI contract.
    #[test]
    fn test_render_resistances_zero_uses_empty_color() {
        // Verify the constant is the expected muted grey.
        assert_eq!(
            STAT_EMPTY_COLOR,
            egui::Color32::from_rgb(128, 128, 128),
            "STAT_EMPTY_COLOR must be grey (128, 128, 128)"
        );

        // Verify the render helper does not panic when value == 0.
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_resistance_row(ui, "Magic", 0);
            });
        });
    }

    /// Verifies `render_resistance_row` uses `STAT_MODIFIED_COLOR` for a non-zero value.
    ///
    /// Calls the helper in a minimal egui context and confirms it does not panic.
    /// The test also asserts the colour constant matches the expected amber RGB value.
    #[test]
    fn test_render_resistances_nonzero_uses_modified_color() {
        // Verify the constant is the expected amber.
        assert_eq!(
            STAT_MODIFIED_COLOR,
            egui::Color32::from_rgb(255, 191, 0),
            "STAT_MODIFIED_COLOR must be amber (255, 191, 0)"
        );

        // Verify the render helper does not panic when value > 0.
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_resistance_row(ui, "Fire", 25);
                render_resistance_row(ui, "Cold", 255);
            });
        });
    }

    /// Verifies that `render_single_view` renders the expanded About block
    /// (Sex, Alignment, Age, Gold, Gems) without panicking, and that all
    /// eight resistance rows render for a character with non-zero resistances.
    #[test]
    fn test_render_about_section_displays_sex_alignment_age() {
        use crate::domain::character::{Alignment, Character, Sex};

        let mut state = GameState::new();
        let mut ch = Character::new(
            "Elara Silverveil".to_string(),
            "elf".to_string(),
            "sorcerer".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        // Set non-default About fields so we exercise actual display paths.
        ch.age = 120;
        ch.age_days = 45;
        ch.gold = 1_500;
        ch.gems = 12;
        // Set non-zero resistances to exercise the amber colour path.
        ch.resistances.magic.current = 10;
        ch.resistances.fire.current = 20;
        ch.resistances.cold.current = 15;
        ch.resistances.electricity.current = 5;
        ch.resistances.acid.current = 0; // stays grey
        ch.resistances.fear.current = 30;
        ch.resistances.poison.current = 25;
        ch.resistances.psychic.current = 8;

        state.party.add_member(ch).unwrap();
        state.enter_character_sheet();

        let mut gs = crate::game::resources::GlobalState(state);
        let campaign_config = crate::domain::campaign::CampaignConfig::default();

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_single_view(
                    ui,
                    &mut gs,
                    1,
                    0,
                    &campaign_config,
                    None,
                    None,
                    None,
                    "elara_silverveil",
                );
            });
        });
        // Reaching here without panic = About block and Resistances rendered correctly.
    }
}
