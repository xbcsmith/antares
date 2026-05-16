// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! NPC Skill Trainer UI System
//!
//! Provides an egui-based three-column interface for NPC skill training.
//! The player selects a party member, picks a skill offered by the trainer
//! NPC, and pays gold to raise that skill's persistent rank by one.
//!
//! This system is active when the game is in [`GameMode::SkillTraining`] mode,
//! which is entered via the dialogue system when the player speaks to an NPC
//! with `is_skill_trainer: true`.
//!
//! ## Layout
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────┐
//! │  🎓 Skill Training — Master Willow    [↑↓] Navigate │[Tab] Switch │[ESC] │
//! │  Party gold: 1,200                                                        │
//! ├─────────────────┬───────────────────────────┬──────────────────────────┤
//! │  Party Members  │  Available Skills          │  Training Detail         │
//! │                 │                            │                          │
//! │  ► Alice        │    ► Perception  Rank 2    │  Member: Alice           │
//! │    Bob          │      Disarm Traps  Rank 0  │  Skill:  Perception      │
//! │    Carol        │      Item Lore     Rank 1  │                          │
//! │                 │                            │  Rank:   2 → 3           │
//! │                 │                            │  Cost:   200 gold        │
//! │                 │                            │                          │
//! │                 │                            │  [Train — 200 gold]      │
//! │                 │                            │  [Leave Training Grounds]│
//! │                 │                            │                          │
//! │                 │                            │  ✓ Perception trained!   │
//! └─────────────────┴───────────────────────────┴──────────────────────────┘
//! ```
//!
//! ## Mode Transitions
//!
//! The dialogue system transitions to [`GameMode::SkillTraining`] by calling:
//!
//! ```rust,ignore
//! game_state.enter_skill_training(npc_id, eligible_indices, available_skills);
//! ```
//!
//! Pressing **Escape**, clicking **Leave**, or confirming a training session
//! returns to [`GameMode::Exploration`].

use crate::application::resources::GameContent;
use crate::application::skill_training::{
    perform_skill_training_service, SKILL_TRAINING_FEE_BASE_DEFAULT,
    SKILL_TRAINING_FEE_MULTIPLIER_DEFAULT,
};
use crate::application::skill_training_state::SkillTrainingState;
use crate::application::GameMode;
use crate::domain::character::{Character, Party};
use crate::domain::skill_resolver::SkillResolver;
use crate::domain::skills::{SkillId, SkillRank};
use crate::game::resources::GlobalState;
use crate::game::systems::ui::GameLog;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Left column width: party member list.
pub const SKILL_TRAINING_LEFT_COL_W: f32 = 180.0;

/// Right column width: training detail panel.
pub const SKILL_TRAINING_RIGHT_COL_W: f32 = 250.0;

/// Colour for hint labels in the title bar.
pub const SKILL_TRAINING_HINT_COLOR: egui::Color32 = egui::Color32::from_rgb(160, 160, 120);

/// Colour for gold amounts.
pub const SKILL_TRAINING_GOLD_COLOR: egui::Color32 = egui::Color32::YELLOW;

/// Colour for success status messages.
pub const SKILL_TRAINING_SUCCESS_COLOR: egui::Color32 = egui::Color32::from_rgb(80, 220, 120);

/// Colour for error status messages.
pub const SKILL_TRAINING_ERROR_COLOR: egui::Color32 = egui::Color32::RED;

/// Background fill for keyboard-focused rows.
pub const SKILL_TRAINING_FOCUSED_ROW_FILL: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(0, 80, 40, 120);

/// Foreground text colour for keyboard-focused rows.
pub const SKILL_TRAINING_FOCUSED_ROW_TEXT: egui::Color32 = egui::Color32::from_rgb(80, 220, 120);

/// Background fill for mouse-selected (but not keyboard-focused) rows.
pub const SKILL_TRAINING_SELECTED_ROW_FILL: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(60, 60, 0, 120);

/// Foreground text colour for mouse-selected rows.
pub const SKILL_TRAINING_SELECTED_ROW_TEXT: egui::Color32 = egui::Color32::YELLOW;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SkillTrainingPreview {
    current_rank: SkillRank,
    next_rank: SkillRank,
    at_cap: bool,
    fee: u32,
}

fn skill_training_rank_preview(
    member: &Character,
    skill_id: &str,
    npc_id: &str,
    content: Option<&crate::sdk::database::ContentDatabase>,
) -> SkillTrainingPreview {
    let skill_key = skill_id.to_string();
    let current_persistent_rank = member.skill_ranks.get(&skill_key).unwrap_or(0);
    let default_fee = (SKILL_TRAINING_FEE_BASE_DEFAULT as f32
        * SKILL_TRAINING_FEE_MULTIPLIER_DEFAULT
        * (current_persistent_rank as f32 + 1.0)) as u32;

    let Some(db) = content else {
        return SkillTrainingPreview {
            current_rank: current_persistent_rank,
            next_rank: current_persistent_rank.saturating_add(1),
            at_cap: false,
            fee: default_fee,
        };
    };

    let current_rank = SkillResolver::effective_skill_rank_for_character(
        member,
        &skill_key,
        &db.skills,
        &db.classes,
        &db.races,
    )
    .unwrap_or(current_persistent_rank);

    let rank_cap = db
        .npcs
        .get_npc(npc_id)
        .and_then(|npc| npc.skill_training_max_rank)
        .or_else(|| db.skills.get(skill_id).map(|skill| skill.max_rank))
        .unwrap_or(SkillRank::MAX);

    let fee = db
        .npcs
        .get_npc(npc_id)
        .map(|npc| {
            npc.skill_training_fee(
                current_persistent_rank,
                SKILL_TRAINING_FEE_BASE_DEFAULT,
                SKILL_TRAINING_FEE_MULTIPLIER_DEFAULT,
            )
        })
        .unwrap_or(default_fee);

    if current_rank >= rank_cap {
        return SkillTrainingPreview {
            current_rank,
            next_rank: current_rank,
            at_cap: true,
            fee,
        };
    }

    let mut trained_member = member.clone();
    trained_member.skill_ranks.increment(&skill_key);
    let next_rank = SkillResolver::effective_skill_rank_for_character(
        &trained_member,
        &skill_key,
        &db.skills,
        &db.classes,
        &db.races,
    )
    .unwrap_or_else(|_| current_rank.saturating_add(1))
    .min(rank_cap);

    SkillTrainingPreview {
        current_rank,
        next_rank,
        at_cap: false,
        fee,
    }
}

// ── Marker component ──────────────────────────────────────────────────────────

/// Marker component attached to Bevy entities that belong to the skill training UI.
///
/// `skill_training_cleanup_system` despawns entities with this component when
/// the game mode is no longer [`GameMode::SkillTraining`].
#[derive(Component)]
pub struct SkillTrainingUiRoot;

// ── Plugin ────────────────────────────────────────────────────────────────────

/// Bevy plugin for the NPC skill trainer UI.
///
/// Registers the events and systems needed to render the skill training panel,
/// handle player input, and dispatch training actions.
///
/// # Registration
///
/// ```no_run
/// use bevy::prelude::App;
/// use antares::game::systems::skill_training_ui::SkillTrainingPlugin;
///
/// let mut app = App::new();
/// app.add_plugins(SkillTrainingPlugin);
/// ```
pub struct SkillTrainingPlugin;

impl Plugin for SkillTrainingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TrainSkill>()
            .add_message::<SelectSkillTrainingMember>()
            .add_message::<SelectSkillTrainingSkill>()
            .add_message::<ExitSkillTraining>()
            .init_resource::<SkillTrainingNavState>()
            .add_systems(
                Update,
                (
                    skill_training_input_system,
                    skill_training_selection_system,
                    skill_training_ui_system,
                    skill_training_selection_system, // second pass: handle UI-generated selections
                    skill_training_action_system,
                    skill_training_cleanup_system,
                )
                    .chain(),
            );
    }
}

// ── Events ────────────────────────────────────────────────────────────────────

/// Request to train `skill_id` for the party member at `party_index`.
///
/// The action system calls [`perform_skill_training_service`] on receipt.
///
/// # Examples
///
/// ```
/// use antares::game::systems::skill_training_ui::TrainSkill;
///
/// let ev = TrainSkill {
///     party_index: 0,
///     skill_id: "perception".to_string(),
/// };
/// assert_eq!(ev.party_index, 0);
/// assert_eq!(ev.skill_id, "perception");
/// ```
#[derive(Message)]
pub struct TrainSkill {
    /// Index into `game_state.party.members` for the character to train.
    pub party_index: usize,
    /// ID of the skill to train (must exist in `db.skills` and be in the NPC's offer list).
    pub skill_id: SkillId,
}

/// Request to change the highlighted party member in the skill training list.
///
/// `member_index` is an index into [`SkillTrainingState::eligible_member_indices`],
/// **not** into `party.members` directly.  Use [`usize::MAX`] to clear the
/// current selection.
///
/// # Examples
///
/// ```
/// use antares::game::systems::skill_training_ui::SelectSkillTrainingMember;
///
/// let ev = SelectSkillTrainingMember { member_index: 1 };
/// assert_eq!(ev.member_index, 1);
/// ```
#[derive(Message)]
pub struct SelectSkillTrainingMember {
    /// Index into the eligible-member list, or `usize::MAX` to clear.
    pub member_index: usize,
}

/// Request to change the highlighted skill in the skill training list.
///
/// `skill_index` is an index into [`SkillTrainingState::available_skill_ids`].
/// Use [`usize::MAX`] to clear the current selection.
///
/// # Examples
///
/// ```
/// use antares::game::systems::skill_training_ui::SelectSkillTrainingSkill;
///
/// let ev = SelectSkillTrainingSkill { skill_index: 0 };
/// assert_eq!(ev.skill_index, 0);
/// ```
#[derive(Message)]
pub struct SelectSkillTrainingSkill {
    /// Index into the available-skill list, or `usize::MAX` to clear.
    pub skill_index: usize,
}

/// Request to leave the skill training UI and return to [`GameMode::Exploration`].
///
/// # Examples
///
/// ```
/// use antares::game::systems::skill_training_ui::ExitSkillTraining;
///
/// let _ev = ExitSkillTraining;
/// ```
#[derive(Message)]
pub struct ExitSkillTraining;

// ── Navigation resource ───────────────────────────────────────────────────────

/// Which list currently holds keyboard focus in the skill training UI.
///
/// Tab switches between the two lists.  Arrow keys navigate within the
/// currently-focused list.
#[derive(Default, PartialEq, Eq, Clone, Copy, Debug)]
pub enum FocusedSkillList {
    /// Keyboard focus is on the party-member list (left column).
    #[default]
    Members,
    /// Keyboard focus is on the available-skill list (centre column).
    Skills,
}

/// Keyboard navigation state for the skill training UI.
///
/// Tracks which list has keyboard focus, and which row in each list currently
/// has a keyboard highlight.
///
/// # Examples
///
/// ```
/// use antares::game::systems::skill_training_ui::{FocusedSkillList, SkillTrainingNavState};
///
/// let nav = SkillTrainingNavState::default();
/// assert_eq!(nav.focused_list, FocusedSkillList::Members);
/// assert!(nav.focused_member_index.is_none());
/// assert!(nav.focused_skill_index.is_none());
/// ```
#[derive(Resource, Default, Debug)]
pub struct SkillTrainingNavState {
    /// Which list (Members or Skills) currently has keyboard focus.
    pub focused_list: FocusedSkillList,
    /// Index into `eligible_member_indices` with keyboard focus (`None` = unfocused).
    pub focused_member_index: Option<usize>,
    /// Index into `available_skill_ids` with keyboard focus (`None` = unfocused).
    pub focused_skill_index: Option<usize>,
}

// ── Pure helper ───────────────────────────────────────────────────────────────

/// Returns alive party members eligible for skill training.
///
/// Resolves [`SkillTrainingState::eligible_member_indices`] to actual party
/// members, skipping indices that are out of bounds or belong to a dead member.
///
/// A dead member is one where [`Character::is_alive`] returns `false`
/// (i.e. `hp.current == 0` or a fatal condition such as stone/eradicated).
///
/// # Arguments
///
/// * `state` — The active skill training session state.
/// * `party` — The active party whose members are resolved.
///
/// # Returns
///
/// A `Vec` of `(party_index, &Character)` tuples in the same order as
/// `eligible_member_indices`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::skill_training_ui::eligible_skill_training_members;
/// use antares::application::skill_training_state::SkillTrainingState;
/// use antares::domain::character::{Character, Sex, Alignment, Party};
///
/// let state = SkillTrainingState::new(
///     "trainer",
///     vec![0, 1],
///     vec!["perception".to_string()],
/// );
/// let mut party = Party::new();
/// for name in &["Alice", "Bob"] {
///     let mut c = Character::new(
///         name.to_string(),
///         "human".to_string(),
///         "knight".to_string(),
///         Sex::Female,
///         Alignment::Good,
///     );
///     c.hp.base = 30;
///     c.hp.current = 30;
///     party.members.push(c);
/// }
/// let result = eligible_skill_training_members(&state, &party);
/// assert_eq!(result.len(), 2);
/// assert_eq!(result[0].0, 0);
/// assert_eq!(result[1].0, 1);
/// ```
pub fn eligible_skill_training_members<'p>(
    state: &SkillTrainingState,
    party: &'p Party,
) -> Vec<(usize, &'p Character)> {
    state
        .eligible_member_indices
        .iter()
        .filter_map(|&party_idx| {
            let member = party.members.get(party_idx)?;
            if member.is_alive() {
                Some((party_idx, member))
            } else {
                None
            }
        })
        .collect()
}

// ── Private render helpers ────────────────────────────────────────────────────

/// Renders the party-member column (left).
///
/// Returns `Some(list_idx)` when the user clicks a member row, otherwise `None`.
fn render_member_column(
    ui: &mut egui::Ui,
    state: &SkillTrainingState,
    party: &Party,
    nav_state: &SkillTrainingNavState,
) -> Option<usize> {
    let mut clicked: Option<usize> = None;

    ui.label(
        egui::RichText::new("Party Members")
            .strong()
            .color(egui::Color32::LIGHT_GRAY),
    );
    ui.separator();
    ui.add_space(4.0);

    let members = eligible_skill_training_members(state, party);
    if members.is_empty() {
        ui.label(egui::RichText::new("No eligible members.").italics().weak());
        return None;
    }

    for (list_idx, (party_idx, member)) in members.iter().enumerate() {
        let is_keyboard = nav_state.focused_list == FocusedSkillList::Members
            && nav_state.focused_member_index == Some(list_idx);
        let is_selected = state.selected_member_index == Some(list_idx);

        let (row_fill, text_color) = if is_keyboard {
            (
                SKILL_TRAINING_FOCUSED_ROW_FILL,
                SKILL_TRAINING_FOCUSED_ROW_TEXT,
            )
        } else if is_selected {
            (
                SKILL_TRAINING_SELECTED_ROW_FILL,
                SKILL_TRAINING_SELECTED_ROW_TEXT,
            )
        } else {
            (egui::Color32::from_gray(28), egui::Color32::WHITE)
        };

        ui.push_id(format!("skill_train_member_{}", party_idx), |ui| {
            let stroke_color = if is_keyboard || is_selected {
                text_color
            } else {
                egui::Color32::DARK_GRAY
            };
            let frame = egui::Frame::group(ui.style())
                .fill(row_fill)
                .stroke(egui::Stroke::new(1.0, stroke_color));

            let response = frame
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if is_keyboard || is_selected {
                            ui.label(egui::RichText::new("►").color(text_color));
                        } else {
                            ui.label("  ");
                        }
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&member.name).strong().color(text_color));
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} Lv {}",
                                    member.class_id, member.level
                                ))
                                .small()
                                .color(egui::Color32::GRAY),
                            );
                        });
                    });
                })
                .response;

            if response.clicked() {
                clicked = Some(list_idx);
            }
        });
        ui.add_space(2.0);
    }

    clicked
}

/// Renders the available-skill column (centre).
///
/// Returns `Some(list_idx)` when the user clicks a skill row, otherwise `None`.
fn render_skill_column(
    ui: &mut egui::Ui,
    state: &SkillTrainingState,
    content: Option<&GameContent>,
    nav_state: &SkillTrainingNavState,
) -> Option<usize> {
    let mut clicked: Option<usize> = None;

    ui.label(
        egui::RichText::new("Available Skills")
            .strong()
            .color(egui::Color32::LIGHT_GRAY),
    );
    ui.separator();
    ui.add_space(4.0);

    if state.available_skill_ids.is_empty() {
        ui.label(
            egui::RichText::new("This trainer offers no skills.")
                .italics()
                .weak(),
        );
        return None;
    }

    for (list_idx, skill_id) in state.available_skill_ids.iter().enumerate() {
        let display_name = content
            .and_then(|c| c.db().skills.get(skill_id.as_str()))
            .map(|s| s.name.as_str())
            .unwrap_or(skill_id.as_str());

        let is_keyboard = nav_state.focused_list == FocusedSkillList::Skills
            && nav_state.focused_skill_index == Some(list_idx);
        let is_selected = state.selected_skill_index == Some(list_idx);

        let (row_fill, text_color) = if is_keyboard {
            (
                SKILL_TRAINING_FOCUSED_ROW_FILL,
                SKILL_TRAINING_FOCUSED_ROW_TEXT,
            )
        } else if is_selected {
            (
                SKILL_TRAINING_SELECTED_ROW_FILL,
                SKILL_TRAINING_SELECTED_ROW_TEXT,
            )
        } else {
            (egui::Color32::from_gray(28), egui::Color32::WHITE)
        };

        ui.push_id(format!("skill_train_skill_{}", list_idx), |ui| {
            let stroke_color = if is_keyboard || is_selected {
                text_color
            } else {
                egui::Color32::DARK_GRAY
            };
            let frame = egui::Frame::group(ui.style())
                .fill(row_fill)
                .stroke(egui::Stroke::new(1.0, stroke_color));

            let response = frame
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if is_keyboard || is_selected {
                            ui.label(egui::RichText::new("►").color(text_color));
                        } else {
                            ui.label("  ");
                        }
                        ui.label(egui::RichText::new(display_name).color(text_color));
                    });
                })
                .response;

            if response.clicked() {
                clicked = Some(list_idx);
            }
        });
        ui.add_space(2.0);
    }

    clicked
}

/// Renders the training detail panel (right column).
///
/// Shows the selected member and skill, the current and next effective ranks,
/// the gold fee, the Train and Leave buttons, and the status message.
///
/// Returns `(train_clicked, leave_clicked)`.
fn render_detail_column(
    ui: &mut egui::Ui,
    state: &SkillTrainingState,
    party: &Party,
    content: Option<&GameContent>,
) -> (bool, bool) {
    let mut train_clicked = false;
    let mut leave_clicked = false;

    ui.label(
        egui::RichText::new("Training Detail")
            .strong()
            .color(egui::Color32::LIGHT_GRAY),
    );
    ui.separator();
    ui.add_space(4.0);

    // Resolve the selected member.
    let maybe_member: Option<(usize, &Character)> =
        state.selected_member_index.and_then(|list_idx| {
            let &party_idx = state.eligible_member_indices.get(list_idx)?;
            party.members.get(party_idx).map(|m| (party_idx, m))
        });

    // Resolve the selected skill.
    let maybe_skill_id: Option<&SkillId> = state
        .selected_skill_index
        .and_then(|idx| state.available_skill_ids.get(idx));

    match (maybe_member, maybe_skill_id) {
        (Some((_party_idx, member)), Some(skill_id)) => {
            // Member name
            ui.label(
                egui::RichText::new(format!("Member:  {}", member.name))
                    .color(egui::Color32::WHITE),
            );

            // Skill display name
            let display_name = content
                .and_then(|c| c.db().skills.get(skill_id.as_str()))
                .map(|s| s.name.as_str())
                .unwrap_or(skill_id.as_str());
            ui.label(
                egui::RichText::new(format!("Skill:   {}", display_name))
                    .color(egui::Color32::WHITE),
            );

            ui.add_space(8.0);

            let preview = skill_training_rank_preview(
                member,
                skill_id,
                &state.npc_id,
                content.map(GameContent::db),
            );

            ui.label(
                egui::RichText::new(format!(
                    "Rank:    {} \u{2192} {}",
                    preview.current_rank, preview.next_rank
                ))
                .color(egui::Color32::WHITE),
            );

            let can_afford = party.gold >= preview.fee;
            let can_train = can_afford && !preview.at_cap;

            ui.label(
                egui::RichText::new(format!("Cost:    {} gold", preview.fee)).color(
                    if can_afford {
                        SKILL_TRAINING_GOLD_COLOR
                    } else {
                        SKILL_TRAINING_ERROR_COLOR
                    },
                ),
            );

            if preview.at_cap {
                ui.label(
                    egui::RichText::new("This skill is already at the trainer's cap.")
                        .small()
                        .color(SKILL_TRAINING_ERROR_COLOR),
                );
            } else if !can_afford {
                ui.label(
                    egui::RichText::new(format!(
                        "Need {} more gold.",
                        preview.fee.saturating_sub(party.gold)
                    ))
                    .small()
                    .color(SKILL_TRAINING_ERROR_COLOR),
                );
            }

            ui.add_space(10.0);

            // Train button
            let train_label = format!("Train \u{2014} {} gold", preview.fee);
            if ui
                .add_enabled(
                    can_train,
                    egui::Button::new(egui::RichText::new(&train_label).color(if can_train {
                        SKILL_TRAINING_SUCCESS_COLOR
                    } else {
                        egui::Color32::GRAY
                    })),
                )
                .clicked()
            {
                train_clicked = true;
            }
        }
        (None, _) => {
            ui.label(
                egui::RichText::new("\u{2190} Select a party member")
                    .italics()
                    .color(SKILL_TRAINING_HINT_COLOR),
            );
        }
        (Some(_), None) => {
            ui.label(
                egui::RichText::new("\u{2191} Select a skill to train")
                    .italics()
                    .color(SKILL_TRAINING_HINT_COLOR),
            );
        }
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);

    // Status message (success or error)
    if let Some(msg) = &state.status_message {
        let is_error = msg.contains("Insufficient")
            || msg.contains("not offered")
            || msg.contains("not a skill trainer")
            || msg.contains("at maximum")
            || msg.contains("not trainable")
            || msg.contains("failed")
            || msg.contains("Failed");
        let color = if is_error {
            SKILL_TRAINING_ERROR_COLOR
        } else {
            SKILL_TRAINING_SUCCESS_COLOR
        };
        ui.label(egui::RichText::new(msg.as_str()).color(color));
        ui.add_space(6.0);
    }

    // Leave button
    if ui
        .button(egui::RichText::new("Leave Training Grounds").size(14.0))
        .clicked()
    {
        leave_clicked = true;
    }
    ui.label(
        egui::RichText::new("(or press ESC)")
            .small()
            .weak()
            .color(SKILL_TRAINING_HINT_COLOR),
    );

    (train_clicked, leave_clicked)
}

// ── UI system ─────────────────────────────────────────────────────────────────

/// Renders the skill training panel when the game is in [`GameMode::SkillTraining`].
///
/// Uses the three-column `allocate_ui` layout:
/// - Left  (180 px): party member list — scrollable.
/// - Centre (flexible): available skill list — scrollable.
/// - Right (250 px): training detail panel — scrollable.
///
/// Navigation hints are embedded right-aligned in the title bar so the columns
/// own the full remaining window height with no bottom reservation needed.
///
/// This system is a complete no-op when the game mode is not `SkillTraining`.
#[allow(clippy::too_many_arguments)]
fn skill_training_ui_system(
    mut contexts: EguiContexts,
    global_state: Res<GlobalState>,
    nav_state: Res<SkillTrainingNavState>,
    content: Option<Res<GameContent>>,
    mut train_events: MessageWriter<TrainSkill>,
    mut exit_events: MessageWriter<ExitSkillTraining>,
    mut member_select: MessageWriter<SelectSkillTrainingMember>,
    mut skill_select: MessageWriter<SelectSkillTrainingSkill>,
) {
    // Only render in SkillTraining mode.  Clone the state so we don't hold a
    // borrow into global_state.0.mode while also reading global_state elsewhere.
    let st = match &global_state.0.mode {
        GameMode::SkillTraining(s) => s.clone(),
        _ => return,
    };

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    // Look up the NPC display name for the header.
    let npc_name = content
        .as_deref()
        .and_then(|c| c.db().npcs.get_npc(&st.npc_id))
        .map(|n| n.name.clone())
        .unwrap_or_else(|| "Skill Trainer".to_string());

    let party_gold = global_state.0.party.gold;

    egui::CentralPanel::default().show(ctx, |ui| {
        // ── Title bar — hints right-aligned so columns get the full remaining
        //   height without bottom reservation needed ──────────────────────────
        ui.horizontal(|ui| {
            ui.heading(format!("\u{1F393} Skill Training \u{2014} {}", npc_name)); // 🎓
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("[ESC] Leave").color(SKILL_TRAINING_HINT_COLOR));
                ui.separator();
                ui.label(egui::RichText::new("[Enter] Train").color(SKILL_TRAINING_HINT_COLOR));
                ui.separator();
                ui.label(
                    egui::RichText::new("[Tab] Switch Column").color(SKILL_TRAINING_HINT_COLOR),
                );
                ui.separator();
                ui.label(
                    egui::RichText::new("[\u{2191}\u{2193}] Navigate")
                        .color(SKILL_TRAINING_HINT_COLOR),
                );
            });
        });

        ui.separator();

        // Party gold display below the header.
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("Party gold: {}", party_gold))
                    .color(SKILL_TRAINING_GOLD_COLOR),
            );
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // Pre-compute column geometry from available_size() BEFORE entering
        // ui.horizontal.  ui.allocate_ui gives each column an explicit rect so
        // the ScrollAreas fill the full column height.
        let available = ui.available_size();
        let col_h = available.y;
        // 2 separators: 1 px line + item_spacing.x on each side, times two.
        let sep_total = (1.0 + 2.0 * ui.spacing().item_spacing.x) * 2.0;
        let center_w =
            (available.x - SKILL_TRAINING_LEFT_COL_W - SKILL_TRAINING_RIGHT_COL_W - sep_total)
                .max(160.0);

        // ── Three-column body — each column owns an explicit rect ─────────────
        ui.horizontal(|ui| {
            // ── Left column: party member list ────────────────────────────────
            ui.allocate_ui(egui::vec2(SKILL_TRAINING_LEFT_COL_W, col_h), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("skill_training_member_list")
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        if let Some(clicked) =
                            render_member_column(ui, &st, &global_state.0.party, &nav_state)
                        {
                            member_select.write(SelectSkillTrainingMember {
                                member_index: clicked,
                            });
                        }
                    });
            });

            ui.separator();

            // ── Centre column: available skill list ───────────────────────────
            ui.allocate_ui(egui::vec2(center_w, col_h), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("skill_training_skill_list")
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        if let Some(clicked) =
                            render_skill_column(ui, &st, content.as_deref(), &nav_state)
                        {
                            skill_select.write(SelectSkillTrainingSkill {
                                skill_index: clicked,
                            });
                        }
                    });
            });

            ui.separator();

            // ── Right column: training detail, fee, actions, status ───────────
            ui.allocate_ui(egui::vec2(SKILL_TRAINING_RIGHT_COL_W, col_h), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("skill_training_detail_panel")
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        let (train_clicked, leave_clicked) = render_detail_column(
                            ui,
                            &st,
                            &global_state.0.party,
                            content.as_deref(),
                        );

                        if train_clicked {
                            // Resolve the current selections into a TrainSkill event.
                            if let (Some(member_list_idx), Some(skill_list_idx)) =
                                (st.selected_member_index, st.selected_skill_index)
                            {
                                if let Some(&party_idx) =
                                    st.eligible_member_indices.get(member_list_idx)
                                {
                                    if let Some(skill_id) =
                                        st.available_skill_ids.get(skill_list_idx)
                                    {
                                        train_events.write(TrainSkill {
                                            party_index: party_idx,
                                            skill_id: skill_id.clone(),
                                        });
                                    }
                                }
                            }
                        }

                        if leave_clicked {
                            exit_events.write(ExitSkillTraining);
                        }
                    });
            });
        });
    });
}

// ── Selection system ──────────────────────────────────────────────────────────

/// Updates [`SkillTrainingState`] selection fields from selection events.
///
/// Handles both [`SelectSkillTrainingMember`] and [`SelectSkillTrainingSkill`]
/// events.  Sending `usize::MAX` as the index clears the respective selection.
fn skill_training_selection_system(
    mut member_events: MessageReader<SelectSkillTrainingMember>,
    mut skill_events: MessageReader<SelectSkillTrainingSkill>,
    mut global_state: ResMut<GlobalState>,
) {
    for ev in member_events.read() {
        if let GameMode::SkillTraining(ref mut st) = global_state.0.mode {
            if ev.member_index == usize::MAX {
                st.selected_member_index = None;
            } else {
                st.selected_member_index = Some(ev.member_index);
            }
        }
    }

    for ev in skill_events.read() {
        if let GameMode::SkillTraining(ref mut st) = global_state.0.mode {
            if ev.skill_index == usize::MAX {
                st.selected_skill_index = None;
            } else {
                st.selected_skill_index = Some(ev.skill_index);
            }
        }
    }
}

// ── Action system ─────────────────────────────────────────────────────────────

/// Handles [`TrainSkill`] and [`ExitSkillTraining`] events.
///
/// On a [`TrainSkill`] event the system calls [`perform_skill_training_service`],
/// updates `SkillTrainingState::status_message` with the result, and writes a
/// log entry.  On an [`ExitSkillTraining`] event the mode is set to
/// [`GameMode::Exploration`].
fn skill_training_action_system(
    mut train_events: MessageReader<TrainSkill>,
    mut exit_events: MessageReader<ExitSkillTraining>,
    mut global_state: ResMut<GlobalState>,
    content: Res<GameContent>,
    mut game_log: Option<ResMut<GameLog>>,
) {
    // Bail early when not in SkillTraining mode.
    let npc_id = match &global_state.0.mode {
        GameMode::SkillTraining(s) => s.npc_id.clone(),
        _ => return,
    };

    // ── Process training requests ────────────────────────────────────────────
    for ev in train_events.read() {
        let character_name = global_state
            .0
            .party
            .members
            .get(ev.party_index)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| format!("member {}", ev.party_index));

        let skill_id = ev.skill_id.clone();

        match perform_skill_training_service(
            &mut global_state.0,
            &npc_id,
            ev.party_index,
            &skill_id,
            content.db(),
        ) {
            Ok((new_rank, fee_paid)) => {
                let msg = format!(
                    "{} trained {}! Rank is now {} (paid {} gold).",
                    character_name, skill_id, new_rank, fee_paid
                );
                if let Some(ref mut log) = game_log {
                    log.add_dialogue(msg.clone());
                }
                if let GameMode::SkillTraining(ref mut st) = global_state.0.mode {
                    st.status_message = Some(msg);
                }
            }
            Err(e) => {
                let msg = e.to_string();
                if let Some(ref mut log) = game_log {
                    log.add_system(format!("Skill training failed: {}", msg));
                }
                if let GameMode::SkillTraining(ref mut st) = global_state.0.mode {
                    st.status_message = Some(msg);
                }
            }
        }
    }

    // ── Process exit requests ────────────────────────────────────────────────
    for _ev in exit_events.read() {
        global_state.0.mode = GameMode::Exploration;
        if let Some(ref mut log) = game_log {
            log.add_exploration("Left the skill training grounds.".to_string());
        }
    }
}

// ── Input system ──────────────────────────────────────────────────────────────

/// Handles keyboard input for the skill training UI.
///
/// - **Arrow Down / Arrow Up**: navigate within the focused list.
/// - **Tab**: switch keyboard focus between the member list and the skill list.
/// - **Enter**: confirm training when both a member and a skill are selected.
/// - **Escape**: emit [`ExitSkillTraining`] immediately.
fn skill_training_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    global_state: Res<GlobalState>,
    mut nav_state: ResMut<SkillTrainingNavState>,
    mut train_events: MessageWriter<TrainSkill>,
    mut exit_events: MessageWriter<ExitSkillTraining>,
    mut member_select: MessageWriter<SelectSkillTrainingMember>,
    mut skill_select: MessageWriter<SelectSkillTrainingSkill>,
) {
    // Only process input when in SkillTraining mode; reset nav otherwise.
    let st = match &global_state.0.mode {
        GameMode::SkillTraining(s) => s,
        _ => {
            *nav_state = SkillTrainingNavState::default();
            return;
        }
    };

    let member_list_len = st.eligible_member_indices.len();
    let skill_list_len = st.available_skill_ids.len();

    // ESC → leave immediately.
    if keyboard.just_pressed(KeyCode::Escape) {
        exit_events.write(ExitSkillTraining);
        return;
    }

    // Tab → switch focused list.
    if keyboard.just_pressed(KeyCode::Tab) {
        match nav_state.focused_list {
            FocusedSkillList::Members => {
                nav_state.focused_list = FocusedSkillList::Skills;
                if skill_list_len > 0 && nav_state.focused_skill_index.is_none() {
                    nav_state.focused_skill_index = Some(0);
                    skill_select.write(SelectSkillTrainingSkill { skill_index: 0 });
                }
            }
            FocusedSkillList::Skills => {
                nav_state.focused_list = FocusedSkillList::Members;
                if member_list_len > 0 && nav_state.focused_member_index.is_none() {
                    nav_state.focused_member_index = Some(0);
                    member_select.write(SelectSkillTrainingMember { member_index: 0 });
                }
            }
        }
        return;
    }

    // Arrow Down → advance focus within the current list.
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        match nav_state.focused_list {
            FocusedSkillList::Members if member_list_len > 0 => {
                let next = match nav_state.focused_member_index {
                    None => 0,
                    Some(i) => (i + 1).min(member_list_len - 1),
                };
                nav_state.focused_member_index = Some(next);
                member_select.write(SelectSkillTrainingMember { member_index: next });
            }
            FocusedSkillList::Skills if skill_list_len > 0 => {
                let next = match nav_state.focused_skill_index {
                    None => 0,
                    Some(i) => (i + 1).min(skill_list_len - 1),
                };
                nav_state.focused_skill_index = Some(next);
                skill_select.write(SelectSkillTrainingSkill { skill_index: next });
            }
            _ => {}
        }
    }

    // Arrow Up → move focus up within the current list.
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        match nav_state.focused_list {
            FocusedSkillList::Members if member_list_len > 0 => {
                let prev = match nav_state.focused_member_index {
                    None | Some(0) => 0,
                    Some(i) => i - 1,
                };
                nav_state.focused_member_index = Some(prev);
                member_select.write(SelectSkillTrainingMember { member_index: prev });
            }
            FocusedSkillList::Skills if skill_list_len > 0 => {
                let prev = match nav_state.focused_skill_index {
                    None | Some(0) => 0,
                    Some(i) => i - 1,
                };
                nav_state.focused_skill_index = Some(prev);
                skill_select.write(SelectSkillTrainingSkill { skill_index: prev });
            }
            _ => {}
        }
    }

    // Enter → confirm training when both member and skill are selected.
    if keyboard.just_pressed(KeyCode::Enter) {
        if let (Some(member_list_idx), Some(skill_list_idx)) =
            (st.selected_member_index, st.selected_skill_index)
        {
            if let Some(&party_idx) = st.eligible_member_indices.get(member_list_idx) {
                if let Some(skill_id) = st.available_skill_ids.get(skill_list_idx) {
                    train_events.write(TrainSkill {
                        party_index: party_idx,
                        skill_id: skill_id.clone(),
                    });
                }
            }
        }
    }
}

// ── Cleanup system ────────────────────────────────────────────────────────────

/// Despawns [`SkillTrainingUiRoot`] entities and resets [`SkillTrainingNavState`]
/// when the game leaves [`GameMode::SkillTraining`].
///
/// This system is a no-op while the game remains in SkillTraining mode.
fn skill_training_cleanup_system(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    mut nav_state: ResMut<SkillTrainingNavState>,
    roots: Query<Entity, With<SkillTrainingUiRoot>>,
) {
    if !matches!(global_state.0.mode, GameMode::SkillTraining(_)) {
        *nav_state = SkillTrainingNavState::default();
        for entity in roots.iter() {
            commands.entity(entity).despawn();
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::skill_training_state::SkillTrainingState;
    use crate::application::{GameMode, GameState};
    use crate::domain::character::{Alignment, Character, Party, Sex};

    // ── Test constants ────────────────────────────────────────────────────────

    /// Minimal RON string for a single trainable "perception" skill.
    const PERCEPTION_SKILL_RON: &str = r#"[
        (
            id: "perception",
            name: "Perception",
            category: Exploration,
            description: "Awareness of the environment.",
            scaling: Linear(base: 0, per_level: 1),
            max_rank: 50,
            is_trainable: true,
        ),
    ]"#;

    // ── Test helpers ──────────────────────────────────────────────────────────

    fn make_alive_member(name: &str) -> Character {
        let mut c = Character::new(
            name.to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.base = 30;
        c.hp.current = 30;
        c
    }

    fn make_dead_member(name: &str) -> Character {
        let mut c = make_alive_member(name);
        c.hp.current = 0;
        c
    }

    /// Builds a `ContentDatabase` with classes, races, a trainable perception
    /// skill, and a skill trainer NPC.
    fn make_skill_trainer_db() -> crate::sdk::database::ContentDatabase {
        let mut db = crate::sdk::database::ContentDatabase::new();
        db.classes = crate::domain::classes::ClassDatabase::load_from_file("data/classes.ron")
            .expect("data/classes.ron must exist");
        db.races = crate::domain::races::RaceDatabase::load_from_file("data/races.ron")
            .expect("data/races.ron must exist");
        db.skills = crate::domain::skills::SkillDatabase::load_from_string(PERCEPTION_SKILL_RON)
            .expect("inline skill RON must parse");

        let mut npc =
            crate::domain::world::npc::NpcDefinition::new("perception_sage", "Sage", "sage.png");
        npc.is_skill_trainer = true;
        npc.trainable_skill_ids = vec!["perception".to_string()];
        npc.skill_training_fee_base = Some(50);
        db.npcs.add_npc(npc).unwrap();
        db
    }

    // ── Default state ─────────────────────────────────────────────────────────

    /// Initial `SkillTrainingNavState` and `SkillTrainingState` must have all
    /// selection fields set to `None` and focus defaulting to the member list.
    #[test]
    fn test_skill_training_state_default_selection() {
        let nav = SkillTrainingNavState::default();
        assert_eq!(nav.focused_list, FocusedSkillList::Members);
        assert!(nav.focused_member_index.is_none());
        assert!(nav.focused_skill_index.is_none());

        let state = SkillTrainingState::new("trainer", vec![], vec![]);
        assert!(state.selected_member_index.is_none());
        assert!(state.selected_skill_index.is_none());
        assert!(state.status_message.is_none());
    }

    // ── eligible_skill_training_members ───────────────────────────────────────

    /// Empty `eligible_member_indices` must yield an empty result without panic.
    #[test]
    fn test_eligible_members_empty_list() {
        let state = SkillTrainingState::new("trainer", vec![], vec!["perception".to_string()]);
        let party = Party::new();
        let result = eligible_skill_training_members(&state, &party);
        assert!(
            result.is_empty(),
            "empty eligible_member_indices must yield empty result"
        );
    }

    /// Out-of-bounds indices in `eligible_member_indices` must be silently dropped.
    #[test]
    fn test_eligible_members_out_of_bounds_filtered() {
        let state = SkillTrainingState::new("trainer", vec![99], vec!["perception".to_string()]);
        let party = Party::new();
        let result = eligible_skill_training_members(&state, &party);
        assert!(result.is_empty(), "out-of-bounds index must be filtered");
    }

    /// Dead members must be excluded from the eligible member list.
    #[test]
    fn test_skill_training_eligible_members_filters_dead_members() {
        let state =
            SkillTrainingState::new("trainer", vec![0, 1, 2], vec!["perception".to_string()]);

        let mut party = Party::new();
        party.members.push(make_alive_member("Alice")); // index 0 — alive
        party.members.push(make_dead_member("Bob")); //   index 1 — dead
        party.members.push(make_alive_member("Carol")); // index 2 — alive

        let result = eligible_skill_training_members(&state, &party);

        assert_eq!(result.len(), 2, "dead member must be excluded");
        assert_eq!(result[0].0, 0);
        assert_eq!(result[0].1.name, "Alice");
        assert_eq!(result[1].0, 2);
        assert_eq!(result[1].1.name, "Carol");
    }

    // ── available_skill_ids reflects NPC offer list ───────────────────────────

    /// `state.available_skill_ids` is the canonical source for which skills can
    /// be trained — skills NOT in this list are not presented to the player.
    #[test]
    fn test_skill_training_available_skills_filters_unoffered_skills() {
        let state = SkillTrainingState::new(
            "trainer",
            vec![0],
            vec!["perception".to_string(), "disarm_traps".to_string()],
        );

        assert_eq!(state.available_skill_ids.len(), 2);
        assert!(state
            .available_skill_ids
            .contains(&"perception".to_string()));
        assert!(state
            .available_skill_ids
            .contains(&"disarm_traps".to_string()));
        // "diplomacy" was not offered by this NPC and must be absent.
        assert!(!state.available_skill_ids.contains(&"diplomacy".to_string()));
    }

    // ── Plugin smoke test ─────────────────────────────────────────────────────

    #[test]
    fn test_skill_training_plugin_builds() {
        let mut app = App::new();
        app.add_plugins(SkillTrainingPlugin);
        // If we reach here the plugin registered without panicking.
    }

    // ── Action system: exit ───────────────────────────────────────────────────

    /// Receiving an `ExitSkillTraining` event must transition the game to
    /// `GameMode::Exploration`.
    #[test]
    fn test_skill_training_input_escape_exits_mode() {
        use crate::application::resources::GameContent;
        use crate::game::resources::GlobalState;
        use crate::game::systems::ui::GameLog;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<TrainSkill>();
        app.add_message::<SelectSkillTrainingMember>();
        app.add_message::<SelectSkillTrainingSkill>();
        app.add_message::<ExitSkillTraining>();
        app.add_systems(Update, skill_training_action_system);
        app.init_resource::<GameLog>();

        let mut game_state = GameState::new();
        game_state.enter_skill_training("trainer", vec![], vec![]);
        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(ContentDatabase::new()));

        app.world_mut()
            .resource_mut::<Messages<ExitSkillTraining>>()
            .write(ExitSkillTraining);

        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::Exploration),
            "mode must be Exploration after ExitSkillTraining"
        );
    }

    #[test]
    fn test_skill_training_escape_key_exits_mode() {
        use crate::application::resources::GameContent;
        use crate::game::resources::GlobalState;
        use crate::game::systems::ui::GameLog;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<TrainSkill>();
        app.add_message::<SelectSkillTrainingMember>();
        app.add_message::<SelectSkillTrainingSkill>();
        app.add_message::<ExitSkillTraining>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<SkillTrainingNavState>();
        app.init_resource::<GameLog>();
        app.add_systems(
            Update,
            (skill_training_input_system, skill_training_action_system).chain(),
        );

        let mut game_state = GameState::new();
        game_state.enter_skill_training("trainer", vec![], vec![]);
        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(ContentDatabase::new()));

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::Exploration),
            "actual Escape key input must leave SkillTraining mode"
        );
    }

    #[test]
    fn test_skill_training_preview_clamps_to_cap() {
        let mut db = make_skill_trainer_db();
        db.npcs = {
            let mut npcs = crate::sdk::database::NpcDatabase::new();
            let mut npc = crate::domain::world::npc::NpcDefinition::new(
                "perception_sage",
                "Sage",
                "sage.png",
            );
            npc.is_skill_trainer = true;
            npc.trainable_skill_ids = vec!["perception".to_string()];
            npc.skill_training_max_rank = Some(5);
            npcs.add_npc(npc).unwrap();
            npcs
        };

        let mut hero = make_alive_member("Capped Hero");
        hero.skill_ranks.set("perception".to_string(), 5);

        let preview =
            skill_training_rank_preview(&hero, "perception", "perception_sage", Some(&db));

        assert!(preview.at_cap);
        assert_eq!(preview.current_rank, 5);
        assert_eq!(preview.next_rank, 5);
    }

    // ── Action system: success ────────────────────────────────────────────────

    /// A successful `TrainSkill` event must set a non-empty success status
    /// message in `SkillTrainingState`.
    #[test]
    fn test_skill_training_action_success_updates_status() {
        use crate::application::resources::GameContent;
        use crate::game::resources::GlobalState;
        use crate::game::systems::ui::GameLog;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<TrainSkill>();
        app.add_message::<SelectSkillTrainingMember>();
        app.add_message::<SelectSkillTrainingSkill>();
        app.add_message::<ExitSkillTraining>();
        app.add_systems(Update, skill_training_action_system);
        app.init_resource::<GameLog>();

        let db = make_skill_trainer_db();

        let mut game_state = GameState::new();
        let hero = make_alive_member("Sir Lancelot");
        game_state.party.members.push(hero);
        game_state.party.gold = 500; // more than the 50-gold fee
        game_state.enter_skill_training("perception_sage", vec![0], vec!["perception".to_string()]);
        if let GameMode::SkillTraining(ref mut st) = game_state.mode {
            st.selected_member_index = Some(0);
            st.selected_skill_index = Some(0);
        }

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));

        app.world_mut()
            .resource_mut::<Messages<TrainSkill>>()
            .write(TrainSkill {
                party_index: 0,
                skill_id: "perception".to_string(),
            });

        app.update();

        let gs = app.world().resource::<GlobalState>();
        if let GameMode::SkillTraining(ref st) = gs.0.mode {
            let msg = st.status_message.as_deref().unwrap_or("");
            assert!(
                msg.contains("perception") || msg.contains("trained"),
                "success status message must mention 'perception' or 'trained'; got: {:?}",
                msg
            );
        } else {
            panic!("mode should still be SkillTraining after a single successful TrainSkill event");
        }
    }

    // ── Action system: failure ────────────────────────────────────────────────

    /// A `TrainSkill` event that fails due to insufficient gold must set an
    /// error status message in `SkillTrainingState`.
    #[test]
    fn test_skill_training_action_failure_updates_status() {
        use crate::application::resources::GameContent;
        use crate::game::resources::GlobalState;
        use crate::game::systems::ui::GameLog;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<TrainSkill>();
        app.add_message::<SelectSkillTrainingMember>();
        app.add_message::<SelectSkillTrainingSkill>();
        app.add_message::<ExitSkillTraining>();
        app.add_systems(Update, skill_training_action_system);
        app.init_resource::<GameLog>();

        let db = make_skill_trainer_db();

        let mut game_state = GameState::new();
        let hero = make_alive_member("Penniless Hero");
        game_state.party.members.push(hero);
        game_state.party.gold = 5; // fee is 50, so this is insufficient
        game_state.enter_skill_training("perception_sage", vec![0], vec!["perception".to_string()]);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));

        app.world_mut()
            .resource_mut::<Messages<TrainSkill>>()
            .write(TrainSkill {
                party_index: 0,
                skill_id: "perception".to_string(),
            });

        app.update();

        let gs = app.world().resource::<GlobalState>();
        if let GameMode::SkillTraining(ref st) = gs.0.mode {
            let msg = st.status_message.as_deref().unwrap_or("");
            assert!(
                msg.contains("Insufficient") || msg.contains("gold"),
                "failure status message must mention insufficient gold; got: {:?}",
                msg
            );
        } else {
            panic!("mode should remain SkillTraining after a failed training attempt");
        }
    }

    // ── Action system: no-op outside skill training ───────────────────────────

    /// When the game is NOT in `SkillTraining` mode the action system must be a
    /// complete no-op — no state changes, no panics.
    #[test]
    fn test_skill_training_system_noop_when_not_in_skill_training_mode() {
        use crate::application::resources::GameContent;
        use crate::game::resources::GlobalState;
        use crate::game::systems::ui::GameLog;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<TrainSkill>();
        app.add_message::<SelectSkillTrainingMember>();
        app.add_message::<SelectSkillTrainingSkill>();
        app.add_message::<ExitSkillTraining>();
        app.add_systems(Update, skill_training_action_system);
        app.init_resource::<GameLog>();

        let mut game_state = GameState::new();
        game_state.mode = GameMode::Exploration;
        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(ContentDatabase::new()));

        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::Exploration),
            "Exploration mode must be unchanged after an update when not in SkillTraining"
        );
    }
}
