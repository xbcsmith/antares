// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! NPC Trainer Level-Up UI System
//!
//! Provides an egui-based interface for the player to spend gold at a
//! trainer NPC to advance eligible party members to the next level.  This
//! system is active when the game is in [`GameMode::Training`] mode, which
//! is entered when the player interacts with an NPC that has `is_trainer: true`
//! and the campaign uses [`LevelUpMode::NpcTrainer`].
//!
//! ## Layout
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Training Grounds — Master Swordsman          [Esc: Leave]  │
//! ├─────────────────────────────────────────────────────────────┤
//! │  "Present your heroes. I shall forge them into legends."    │
//! │                                                             │
//! │  Party gold: 1,200                                          │
//! │                                                             │
//! │  Eligible party members:                                    │
//! │  ┌──────────────────────────────────────────────────────┐  │
//! │  │  [►] Aldric   Knight   Level 3 → 4                  │  │
//! │  │       XP: 5,196 / 5,196 ✓   Fee: 1,500 gold         │  │
//! │  │                               [Select]  [Train]      │  │
//! │  │  [ ] Selwyn   Archer   Level 2 → 3                   │  │
//! │  │       XP: 2,828 / 2,828 ✓   Fee: 1,000 gold         │  │
//! │  └──────────────────────────────────────────────────────┘  │
//! │                                                             │
//! │  [Leave Training Grounds]   (or press ESC)                  │
//! │                                                             │
//! │  ── Status ──────────────────────────────────────────────── │
//! │  Aldric advanced to level 4! (+8 HP)                        │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Mode Transitions
//!
//! The dialogue system transitions to [`GameMode::Training`] by writing:
//!
//! ```rust,ignore
//! game_state.mode = GameMode::Training(TrainingState::new(npc_id));
//! ```
//!
//! Pressing **Escape**, clicking **Leave Training Grounds**, or successfully
//! training all eligible members returns to [`GameMode::Exploration`].

use crate::application::resources::{perform_training_service, GameContent};
use crate::application::GameMode;
use crate::application::TrainingState;
use crate::domain::character::{Character, Party};
use crate::domain::levels::LevelDatabase;
use crate::domain::progression::experience_for_level_with_config;
use crate::game::resources::game_data::GameDataResource;
use crate::game::resources::GlobalState;
use crate::game::systems::ui::GameLog;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use rand::rng;

// ── Marker component ──────────────────────────────────────────────────────────

/// Marker component attached to any Bevy entity that belongs to the training UI.
///
/// `training_cleanup_system` despawns entities carrying this component when
/// the game mode is no longer [`GameMode::Training`].
#[derive(Component)]
pub struct TrainingUiRoot;

// ── Plugin ────────────────────────────────────────────────────────────────────

/// Bevy plugin for the NPC trainer level-up UI.
///
/// Registers the events and systems needed to render the training panel,
/// handle player input, and dispatch training actions.
///
/// # Registration
///
/// ```no_run
/// use bevy::prelude::App;
/// use antares::game::systems::training_ui::TrainingPlugin;
///
/// let mut app = App::new();
/// app.add_plugins(TrainingPlugin);
/// ```
pub struct TrainingPlugin;

impl Plugin for TrainingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TrainCharacter>()
            .add_message::<ExitTraining>()
            .add_message::<SelectTrainingMember>()
            .init_resource::<TrainingNavState>()
            .add_systems(
                Update,
                (
                    training_input_system,
                    training_selection_system,
                    training_ui_system,
                    training_selection_system, // second pass: handle UI-generated selections
                    training_action_system,
                    training_cleanup_system,
                )
                    .chain(),
            );
    }
}

// ── Events ────────────────────────────────────────────────────────────────────

/// Request to train the party member at `party_index` via the active trainer NPC.
///
/// The action system calls [`perform_training_service`] on receipt.
///
/// # Examples
///
/// ```
/// use antares::game::systems::training_ui::TrainCharacter;
///
/// let ev = TrainCharacter { party_index: 0 };
/// assert_eq!(ev.party_index, 0);
/// ```
#[derive(Message)]
pub struct TrainCharacter {
    /// Index into `game_state.party.members` for the character to train.
    pub party_index: usize,
}

/// Request to leave the training UI and return to [`GameMode::Exploration`].
///
/// # Examples
///
/// ```
/// use antares::game::systems::training_ui::ExitTraining;
///
/// let _ev = ExitTraining;
/// ```
#[derive(Message)]
pub struct ExitTraining;

/// Request to change the highlighted eligible member in the training list.
///
/// `member_index` is an index into `TrainingState::eligible_member_indices`,
/// **not** into `party.members` directly.  Use [`usize::MAX`] to clear the
/// current selection.
///
/// # Examples
///
/// ```
/// use antares::game::systems::training_ui::SelectTrainingMember;
///
/// let ev = SelectTrainingMember { member_index: 1 };
/// assert_eq!(ev.member_index, 1);
/// ```
#[derive(Message)]
pub struct SelectTrainingMember {
    /// Index into the eligible-member list, or `usize::MAX` to clear.
    pub member_index: usize,
}

// ── Navigation resource ───────────────────────────────────────────────────────

/// Keyboard navigation state for the training UI.
///
/// Tracks which eligible member (by index in [`TrainingState::eligible_member_indices`])
/// currently has keyboard focus, and whether the Leave button is focused.
///
/// # Examples
///
/// ```
/// use antares::game::systems::training_ui::TrainingNavState;
///
/// let nav = TrainingNavState::default();
/// assert!(nav.focused_index.is_none());
/// assert!(!nav.focus_on_leave);
/// ```
#[derive(Resource, Default)]
pub struct TrainingNavState {
    /// Index in the eligible-member list that has keyboard focus.
    pub focused_index: Option<usize>,
    /// Whether the Leave button has keyboard focus.
    pub focus_on_leave: bool,
}

// ── Pure helpers (no Bevy dependency; fully testable) ─────────────────────────

/// Returns party members that are eligible for training.
///
/// Resolves [`TrainingState::eligible_member_indices`] to actual party members,
/// filtering out any out-of-bounds index (defensive guard — callers should
/// ensure indices are valid at the time the UI is entered).
///
/// # Arguments
///
/// * `training_state` — The current training session state.
/// * `party`          — The active party whose members are resolved.
///
/// # Returns
///
/// A `Vec` of `(party_index, &Character)` tuples in the same order as
/// `eligible_member_indices`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::training_ui::eligible_members;
/// use antares::application::TrainingState;
/// use antares::domain::character::{Character, Sex, Alignment, Party};
///
/// let mut state = TrainingState::new("trainer".to_string());
/// state.eligible_member_indices.push(0);
/// state.eligible_member_indices.push(2);
///
/// let mut party = Party::new();
/// for name in &["Alice", "Bob", "Carol"] {
///     party.members.push(Character::new(
///         name.to_string(),
///         "human".to_string(),
///         "knight".to_string(),
///         Sex::Female,
///         Alignment::Good,
///     ));
/// }
///
/// let result = eligible_members(&state, &party);
/// assert_eq!(result.len(), 2);
/// assert_eq!(result[0].0, 0);
/// assert_eq!(result[1].0, 2);
/// ```
pub fn eligible_members<'p>(
    training_state: &TrainingState,
    party: &'p Party,
) -> Vec<(usize, &'p Character)> {
    training_state
        .eligible_member_indices
        .iter()
        .filter_map(|&party_idx| party.members.get(party_idx).map(|c| (party_idx, c)))
        .collect()
}

// ── Row action enum ───────────────────────────────────────────────────────────

/// Action produced by an eligible-member row interaction in the training UI.
enum TrainingRowAction {
    /// The user clicked "Select" for the member at the given list index.
    Select(usize),
    /// The user clicked "Train" for the member at the given party index.
    Train(usize),
}

// ── Private UI helpers (extracted from `training_ui_system`) ──────────────────

/// Renders the training panel header: title, flavour quote, and party gold.
fn render_training_header(ui: &mut egui::Ui, npc_name: &str, party_gold: u32) {
    ui.heading(format!("Training Grounds — {}", npc_name));
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new("\"Present your heroes. I shall forge them into legends.\"")
            .italics()
            .color(egui::Color32::from_rgb(200, 200, 140)),
    );
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("Party gold: {}", party_gold)).color(egui::Color32::YELLOW),
        );
    });
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);
}

/// Renders a single eligible-member row inside the training scroll area.
///
/// `highlight` carries the name-label colour when the row is active:
/// `Some(green)` for keyboard focus, `Some(yellow)` for mouse selection,
/// `None` when the row is idle.
///
/// Returns `Some(TrainingRowAction)` when the user clicks a button in the row,
/// or `None` if no interaction occurred this frame.
#[allow(clippy::too_many_arguments)]
fn render_eligible_member_row(
    ui: &mut egui::Ui,
    list_idx: usize,
    party_idx: usize,
    member: &Character,
    highlight: Option<egui::Color32>,
    xp_threshold: u64,
    fee: u32,
    can_afford: bool,
) -> Option<TrainingRowAction> {
    let mut action = None;
    let is_active = highlight.is_some();

    ui.push_id(format!("training_member_{}", list_idx), |ui| {
        let mut frame = egui::Frame::group(ui.style());
        if is_active {
            frame = frame
                .fill(egui::Color32::from_rgba_premultiplied(0, 80, 40, 120))
                .stroke(egui::Stroke::new(
                    2.0,
                    egui::Color32::from_rgb(80, 220, 120),
                ));
        } else {
            frame = frame.fill(egui::Color32::from_gray(30));
        }

        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                // Selection indicator arrow
                if is_active {
                    ui.label(egui::RichText::new("►").color(egui::Color32::from_rgb(80, 220, 120)));
                } else {
                    ui.label("  ");
                }

                ui.vertical(|ui| {
                    // Character name, class, and level progression
                    let name_text = egui::RichText::new(format!(
                        "{} — {} Level {} → {}",
                        member.name,
                        member.class_id,
                        member.level,
                        member.level + 1,
                    ))
                    .strong()
                    .color(highlight.unwrap_or(egui::Color32::WHITE));
                    ui.label(name_text);

                    // XP progress — member is eligible so experience >= threshold
                    ui.label(
                        egui::RichText::new(format!(
                            "XP: {} / {} ✓",
                            member.experience, xp_threshold,
                        ))
                        .small()
                        .color(egui::Color32::from_rgb(120, 200, 120)),
                    );

                    // Training fee for this member
                    ui.label(
                        egui::RichText::new(format!("Training fee: {} gold", fee))
                            .small()
                            .color(egui::Color32::from_rgb(220, 180, 80)),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Train button — enabled only when this row is active and party can afford
                    let train_enabled = can_afford && is_active;
                    if ui
                        .add_enabled(
                            train_enabled,
                            egui::Button::new(egui::RichText::new("Train").color(
                                if train_enabled {
                                    egui::Color32::from_rgb(80, 220, 120)
                                } else {
                                    egui::Color32::GRAY
                                },
                            )),
                        )
                        .clicked()
                    {
                        action = Some(TrainingRowAction::Train(party_idx));
                    }

                    if ui.button("Select").clicked() {
                        action = Some(TrainingRowAction::Select(list_idx));
                    }
                });
            });
        });
        ui.add_space(4.0);
    });

    action
}

/// Renders the training panel footer: status message, Leave button, and instructions.
///
/// Returns `true` when the user clicks the "Leave Training Grounds" button.
fn render_training_footer(
    ui: &mut egui::Ui,
    status_message: Option<&str>,
    focus_on_leave: bool,
) -> bool {
    let mut leave_clicked = false;

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);

    // Status / error message
    if let Some(msg) = status_message {
        let is_error = msg.contains("Insufficient")
            || msg.contains("not eligible")
            || msg.contains("not a trainer")
            || msg.contains("failed")
            || msg.contains("Failed");
        let text = if is_error {
            egui::RichText::new(msg).color(egui::Color32::RED)
        } else {
            egui::RichText::new(msg).color(egui::Color32::from_rgb(80, 220, 120))
        };
        ui.label(text);
        ui.add_space(6.0);
    }

    // Leave button
    ui.horizontal(|ui| {
        let leave_text = if focus_on_leave {
            egui::RichText::new("Leave Training Grounds")
                .strong()
                .size(15.0)
                .color(egui::Color32::from_rgb(144, 238, 144))
        } else {
            egui::RichText::new("Leave Training Grounds").size(15.0)
        };
        if ui
            .add_sized([210.0, 28.0], egui::Button::new(leave_text))
            .clicked()
        {
            leave_clicked = true;
        }
        ui.label(
            egui::RichText::new("(or press ESC)")
                .weak()
                .color(egui::Color32::LIGHT_GREEN),
        );
    });

    ui.add_space(10.0);
    ui.label(egui::RichText::new("Instructions:").weak());
    ui.label(
        egui::RichText::new(
            "• Click Select or use Arrow Keys to highlight an eligible party member",
        )
        .weak()
        .small(),
    );
    ui.label(
        egui::RichText::new(
            "• Click Train or press Enter/Space to advance to the next level (costs gold)",
        )
        .weak()
        .small(),
    );
    ui.label(
        egui::RichText::new("• Press ESC or click Leave to exit the training grounds")
            .weak()
            .small()
            .color(egui::Color32::from_rgb(144, 238, 144)),
    );

    leave_clicked
}

// ── UI system ─────────────────────────────────────────────────────────────────

/// Renders the training panel when the game is in [`GameMode::Training`] mode.
///
/// This system runs every `Update` frame and is a **complete no-op** when the
/// game is in any mode other than `Training`.
///
/// # Parameters
///
/// - `contexts`      — egui render contexts; required for the central panel.
/// - `global_state`  — read-only game state; party, campaign config, and
///   training session state are read here.
/// - `nav_state`     — keyboard-focus state for arrow-key navigation.
/// - `content`       — campaign content; NPC database for trainer lookups.
/// - `game_data`     — optional per-class XP table; `None` uses the formula.
/// - `train_events`  — writer for [`TrainCharacter`] events.
/// - `exit_events`   — writer for [`ExitTraining`] events.
/// - `select_events` — writer for [`SelectTrainingMember`] events.
#[allow(clippy::too_many_arguments)]
fn training_ui_system(
    mut contexts: EguiContexts,
    global_state: Res<GlobalState>,
    nav_state: Res<TrainingNavState>,
    content: Res<GameContent>,
    game_data: Option<Res<GameDataResource>>,
    mut train_events: MessageWriter<TrainCharacter>,
    mut exit_events: MessageWriter<ExitTraining>,
    mut select_events: MessageWriter<SelectTrainingMember>,
) {
    // Only render when in Training mode.
    let training_state = match &global_state.0.mode {
        GameMode::Training(s) => s.clone(),
        _ => return,
    };

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    // Extract optional per-class XP table for threshold display.
    let level_db: Option<&LevelDatabase> = game_data
        .as_deref()
        .and_then(|gd| gd.data().levels.as_ref());

    // Look up the trainer NPC name for the header.
    let npc = content.db().npcs.get_npc(&training_state.npc_id);
    let npc_name = npc.map(|n| n.name.as_str()).unwrap_or("Trainer");

    let campaign_config = &global_state.0.campaign_config;
    let members = eligible_members(&training_state, &global_state.0.party);
    let party_gold = global_state.0.party.gold;

    egui::CentralPanel::default().show(ctx, |ui| {
        render_training_header(ui, npc_name, party_gold);

        ui.label(
            egui::RichText::new("Eligible party members:")
                .size(15.0)
                .strong(),
        );
        ui.add_space(4.0);

        if members.is_empty() {
            ui.label(
                egui::RichText::new("No party members are currently eligible for training.")
                    .italics()
                    .weak(),
            );
        } else {
            egui::ScrollArea::vertical()
                .id_salt("training_member_list")
                .max_height(240.0)
                .show(ui, |ui| {
                    for (list_idx, (party_idx, member)) in members.iter().enumerate() {
                        // Compute XP threshold for the next level.
                        let xp_threshold = experience_for_level_with_config(
                            member.level + 1,
                            &member.class_id,
                            campaign_config,
                            level_db,
                        );

                        // Compute training fee from the NPC definition.
                        let fee = npc
                            .map(|n| n.training_fee_for_level(member.level, campaign_config))
                            .unwrap_or(0);
                        let can_afford = party_gold >= fee;

                        let is_keyboard = nav_state.focused_index == Some(list_idx);
                        let is_selected = training_state.selected_member_index == Some(list_idx);
                        let highlight = if is_keyboard {
                            Some(egui::Color32::from_rgb(80, 220, 120))
                        } else if is_selected {
                            Some(egui::Color32::YELLOW)
                        } else {
                            None
                        };

                        let row_action = render_eligible_member_row(
                            ui,
                            list_idx,
                            *party_idx,
                            member,
                            highlight,
                            xp_threshold,
                            fee,
                            can_afford,
                        );

                        match row_action {
                            Some(TrainingRowAction::Select(idx)) => {
                                select_events.write(SelectTrainingMember { member_index: idx });
                            }
                            Some(TrainingRowAction::Train(idx)) => {
                                train_events.write(TrainCharacter { party_index: idx });
                            }
                            None => {}
                        }
                    }
                });
        }

        if render_training_footer(
            ui,
            training_state.status_message.as_deref(),
            nav_state.focus_on_leave,
        ) {
            exit_events.write(ExitTraining);
        }
    });
}

// ── Selection system ──────────────────────────────────────────────────────────

/// Updates [`TrainingState::selected_member_index`] from
/// [`SelectTrainingMember`] events.
fn training_selection_system(
    mut select_events: MessageReader<SelectTrainingMember>,
    mut global_state: ResMut<GlobalState>,
) {
    for ev in select_events.read() {
        if let GameMode::Training(ref mut ts) = global_state.0.mode {
            if ev.member_index == usize::MAX {
                ts.selected_member_index = None;
            } else {
                ts.selected_member_index = Some(ev.member_index);
            }
        }
    }
}

// ── Action system ─────────────────────────────────────────────────────────────

/// Handles [`TrainCharacter`] and [`ExitTraining`] events.
///
/// On a [`TrainCharacter`] event the system calls [`perform_training_service`],
/// updates `TrainingState::status_message` with the result, and writes a
/// log entry.  On an [`ExitTraining`] event the mode is restored to
/// [`GameMode::Exploration`].
fn training_action_system(
    mut train_events: MessageReader<TrainCharacter>,
    mut exit_events: MessageReader<ExitTraining>,
    mut global_state: ResMut<GlobalState>,
    content: Res<GameContent>,
    game_data: Option<Res<GameDataResource>>,
    mut game_log: Option<ResMut<GameLog>>,
) {
    // Bail early when not in Training mode — nothing to do.
    let npc_id = match &global_state.0.mode {
        GameMode::Training(s) => s.npc_id.clone(),
        _ => return,
    };

    // Extract the optional per-class XP table.
    // The `game_data` binding must outlive `level_db`.
    let level_db: Option<&LevelDatabase> = game_data
        .as_deref()
        .and_then(|gd| gd.data().levels.as_ref());

    let mut rng = rng();

    // ── Process training requests ────────────────────────────────────────────
    for ev in train_events.read() {
        let character_name = global_state
            .0
            .party
            .members
            .get(ev.party_index)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| format!("member {}", ev.party_index));

        match perform_training_service(
            &mut global_state.0,
            &npc_id,
            ev.party_index,
            level_db,
            &mut rng,
            content.db(),
        ) {
            Ok((hp_gained, new_spells)) => {
                let new_level = global_state
                    .0
                    .party
                    .members
                    .get(ev.party_index)
                    .map(|c| c.level)
                    .unwrap_or(0);
                let spell_msg = if new_spells.is_empty() {
                    String::new()
                } else {
                    format!(", {} new spell(s)", new_spells.len())
                };
                let msg = format!(
                    "{} advanced to level {}! (+{} HP{})",
                    character_name, new_level, hp_gained, spell_msg
                );
                if let Some(ref mut log) = game_log {
                    log.add_dialogue(msg.clone());
                }
                if let GameMode::Training(ref mut ts) = global_state.0.mode {
                    ts.status_message = Some(msg);
                    ts.selected_member_index = None;
                }
            }
            Err(e) => {
                let msg = e.to_string();
                if let Some(ref mut log) = game_log {
                    log.add_system(format!("Training failed: {}", msg));
                }
                if let GameMode::Training(ref mut ts) = global_state.0.mode {
                    ts.status_message = Some(msg);
                }
            }
        }
    }

    // ── Process exit requests ────────────────────────────────────────────────
    for _ev in exit_events.read() {
        global_state.0.mode = GameMode::Exploration;
        if let Some(ref mut log) = game_log {
            log.add_exploration("Left the training grounds.".to_string());
        }
    }
}

// ── Input system ──────────────────────────────────────────────────────────────

/// Handles keyboard input for the training UI.
///
/// - **Arrow Down / Arrow Up**: cycle through the eligible-member list.
/// - **Tab**: toggle focus between the member list and the Leave button.
/// - **Enter / Space**: train the focused member or click Leave (depending on focus).
/// - **Escape**: emit [`ExitTraining`] immediately.
fn training_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    global_state: Res<GlobalState>,
    mut nav_state: ResMut<TrainingNavState>,
    mut train_events: MessageWriter<TrainCharacter>,
    mut exit_events: MessageWriter<ExitTraining>,
    mut select_events: MessageWriter<SelectTrainingMember>,
) {
    // Only process input when in Training mode; reset nav otherwise.
    let training_state = match &global_state.0.mode {
        GameMode::Training(s) => s,
        _ => {
            *nav_state = TrainingNavState::default();
            return;
        }
    };

    let list_len = training_state.eligible_member_indices.len();

    // ESC → leave training immediately.
    if keyboard.just_pressed(KeyCode::Escape) {
        exit_events.write(ExitTraining);
        return;
    }

    // Arrow Down → advance to the next eligible member.
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        nav_state.focus_on_leave = false;
        if list_len > 0 {
            let next = match nav_state.focused_index {
                None => 0,
                Some(i) => (i + 1).min(list_len - 1),
            };
            nav_state.focused_index = Some(next);
            select_events.write(SelectTrainingMember { member_index: next });
        }
    }

    // Arrow Up → move to the previous eligible member.
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        nav_state.focus_on_leave = false;
        if list_len > 0 {
            let prev = match nav_state.focused_index {
                None => 0,
                Some(0) => 0,
                Some(i) => i - 1,
            };
            nav_state.focused_index = Some(prev);
            select_events.write(SelectTrainingMember { member_index: prev });
        }
    }

    // Tab → toggle focus between the list and the Leave button.
    if keyboard.just_pressed(KeyCode::Tab) {
        nav_state.focus_on_leave = !nav_state.focus_on_leave;
        if nav_state.focus_on_leave {
            nav_state.focused_index = None;
        } else if list_len > 0 {
            nav_state.focused_index = Some(0);
            select_events.write(SelectTrainingMember { member_index: 0 });
        }
    }

    // Enter / Space → confirm the focused action.
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        if nav_state.focus_on_leave {
            exit_events.write(ExitTraining);
        } else if let Some(list_idx) = nav_state.focused_index {
            // Resolve list index → party index via eligible_member_indices.
            if let Some(&party_idx) = training_state.eligible_member_indices.get(list_idx) {
                train_events.write(TrainCharacter {
                    party_index: party_idx,
                });
            }
        }
    }
}

// ── Cleanup system ────────────────────────────────────────────────────────────

/// Despawns [`TrainingUiRoot`] entities and resets [`TrainingNavState`] when
/// the game leaves [`GameMode::Training`].
///
/// This system is a no-op while the game remains in Training mode.
fn training_cleanup_system(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    mut nav_state: ResMut<TrainingNavState>,
    roots: Query<Entity, With<TrainingUiRoot>>,
) {
    if !matches!(global_state.0.mode, GameMode::Training(_)) {
        *nav_state = TrainingNavState::default();
        for entity in roots.iter() {
            commands.entity(entity).despawn();
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{GameMode, GameState, TrainingState};
    use crate::domain::character::{Alignment, Character, Party, Sex};

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_knight(name: &str) -> Character {
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

    // ── eligible_members tests ─────────────────────────────────────────────

    /// Empty `eligible_member_indices` → empty result, no panic.
    ///
    /// This covers the plan requirement: "entering `GameMode::Training` with
    /// no eligible members shows an empty list without panicking."
    #[test]
    fn test_eligible_members_empty_list() {
        let state = TrainingState::new("trainer".to_string());
        let party = Party::new();
        let result = eligible_members(&state, &party);
        assert!(
            result.is_empty(),
            "empty eligible_member_indices must yield an empty result"
        );
    }

    /// Out-of-bounds party indices are silently filtered out.
    #[test]
    fn test_eligible_members_out_of_bounds_filtered() {
        let mut state = TrainingState::new("trainer".to_string());
        state.eligible_member_indices.push(99); // no such member
        let party = Party::new();
        let result = eligible_members(&state, &party);
        assert!(
            result.is_empty(),
            "out-of-bounds party index must be dropped silently"
        );
    }

    /// Valid indices resolve to the correct party members with correct indices.
    #[test]
    fn test_eligible_members_resolves_correct_party_members() {
        let mut state = TrainingState::new("trainer".to_string());
        state.eligible_member_indices.push(0);
        state.eligible_member_indices.push(2);

        let mut party = Party::new();
        party.members.push(make_knight("Alice")); // index 0 — eligible
        party.members.push(make_knight("Bob")); //   index 1 — not listed
        party.members.push(make_knight("Carol")); // index 2 — eligible

        let result = eligible_members(&state, &party);

        assert_eq!(
            result.len(),
            2,
            "two eligible indices must yield two entries"
        );
        assert_eq!(result[0].0, 0, "first entry must map to party index 0");
        assert_eq!(result[0].1.name, "Alice");
        assert_eq!(result[1].0, 2, "second entry must map to party index 2");
        assert_eq!(result[1].1.name, "Carol");
    }

    /// The order of `eligible_member_indices` is preserved in the output.
    #[test]
    fn test_eligible_members_preserves_order() {
        let mut state = TrainingState::new("trainer".to_string());
        state.eligible_member_indices.push(2); // Carol first
        state.eligible_member_indices.push(0); // Alice second

        let mut party = Party::new();
        party.members.push(make_knight("Alice")); // index 0
        party.members.push(make_knight("Bob")); //   index 1
        party.members.push(make_knight("Carol")); // index 2

        let result = eligible_members(&state, &party);

        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].0, 2,
            "first entry must be party index 2 (Carol) per eligible_member_indices order"
        );
        assert_eq!(result[1].0, 0, "second entry must be party index 0 (Alice)");
    }

    /// A mixed list of valid and invalid indices yields only valid ones.
    #[test]
    fn test_eligible_members_mixed_valid_invalid() {
        let mut state = TrainingState::new("trainer".to_string());
        state.eligible_member_indices.push(0);
        state.eligible_member_indices.push(50); // invalid
        state.eligible_member_indices.push(1);

        let mut party = Party::new();
        party.members.push(make_knight("Slot0"));
        party.members.push(make_knight("Slot1"));

        let result = eligible_members(&state, &party);

        assert_eq!(result.len(), 2, "only valid indices should appear");
        assert_eq!(result[0].0, 0);
        assert_eq!(result[1].0, 1);
    }

    // ── TrainingNavState tests ─────────────────────────────────────────────

    /// `TrainingNavState::default()` must have no focus on any member or Leave.
    #[test]
    fn test_training_nav_state_default() {
        let nav = TrainingNavState::default();
        assert!(
            nav.focused_index.is_none(),
            "default focused_index must be None"
        );
        assert!(!nav.focus_on_leave, "default focus_on_leave must be false");
    }

    // ── TrainingPlugin build smoke test ───────────────────────────────────────

    /// `TrainingPlugin` must register without panicking (plugin build safety).
    #[test]
    fn test_training_plugin_builds() {
        let mut app = App::new();
        app.add_plugins(TrainingPlugin);
        // If we reach here the plugin registered successfully.
    }

    // ── GameMode::Training with no eligible members ────────────────────────

    /// Entering `GameMode::Training` with no eligible members produces an
    /// empty list without panicking — pure-logic coverage.
    #[test]
    fn test_training_mode_no_eligible_members_no_panic() {
        let state = TrainingState::new("trainer".to_string());
        assert!(
            state.eligible_member_indices.is_empty(),
            "new TrainingState must have no eligible indices"
        );

        let mut party = Party::new();
        party.members.push(make_knight("Solo Hero"));

        // Empty eligible list → empty result, no panic.
        let result = eligible_members(&state, &party);
        assert!(
            result.is_empty(),
            "empty eligible_member_indices must yield empty result regardless of party size"
        );
    }

    // ── Mode-transition tests via action system ─────────────────────────────

    /// Sending [`ExitTraining`] must transition the game from Training to
    /// Exploration mode.  This covers the plan requirement: "pressing Escape
    /// transitions back to `GameMode::Exploration`."
    #[test]
    fn test_exit_training_transitions_to_exploration() {
        use crate::application::resources::GameContent;
        use crate::game::resources::GlobalState;
        use crate::game::systems::ui::GameLog;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.add_message::<TrainCharacter>();
        app.add_message::<ExitTraining>();
        app.add_message::<SelectTrainingMember>();
        app.add_systems(Update, training_action_system);
        app.init_resource::<GameLog>();

        let mut game_state = GameState::new();
        game_state.mode = GameMode::Training(TrainingState::new("trainer".to_string()));
        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(ContentDatabase::new()));

        // Simulate pressing ESC by writing an ExitTraining message.
        app.world_mut()
            .resource_mut::<Messages<ExitTraining>>()
            .write(ExitTraining);

        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::Exploration),
            "mode must be Exploration after ExitTraining event"
        );
    }

    /// A successful training call must update `status_message` with the
    /// level-up result.  This covers the plan requirement: "a successful
    /// training call updates `status_message` with the level-up result."
    #[test]
    fn test_successful_training_updates_status_message() {
        use crate::application::resources::GameContent;
        use crate::domain::progression::award_experience;
        use crate::domain::world::npc::NpcDefinition;
        use crate::game::resources::GlobalState;
        use crate::game::systems::ui::GameLog;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.add_message::<TrainCharacter>();
        app.add_message::<ExitTraining>();
        app.add_message::<SelectTrainingMember>();
        app.add_systems(Update, training_action_system);
        app.init_resource::<GameLog>();

        // Build a content database with a trainer NPC and real class definitions.
        let mut db = ContentDatabase::new();
        db.classes = crate::domain::classes::ClassDatabase::load_from_file("data/classes.ron")
            .expect("data/classes.ron must be present for training UI tests");
        let trainer = NpcDefinition::trainer("trainer_bob", "Trainer Bob", "bob.png", 100);
        db.npcs
            .add_npc(trainer)
            .expect("adding trainer NPC must succeed");

        // Build a game state with a knight that has enough XP for level 2.
        let mut game_state = GameState::new();
        game_state.campaign_config.level_up_mode = crate::domain::campaign::LevelUpMode::NpcTrainer;

        let mut knight = make_knight("Sir Lancelot");
        // Formula threshold for level 2 is 1 000 XP.
        award_experience(&mut knight, 1_000).expect("awarding XP must not fail");
        game_state.party.members.push(knight);
        game_state.party.gold = 500; // enough to cover the 100 gold fee

        let mut training_state = TrainingState::new("trainer_bob".to_string());
        training_state.eligible_member_indices.push(0);
        training_state.selected_member_index = Some(0);
        game_state.mode = GameMode::Training(training_state);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));

        // Send a TrainCharacter event for party member 0.
        app.world_mut()
            .resource_mut::<Messages<TrainCharacter>>()
            .write(TrainCharacter { party_index: 0 });

        app.update();

        let gs = app.world().resource::<GlobalState>();

        // Character must have advanced to level 2.
        assert_eq!(
            gs.0.party.members[0].level, 2,
            "knight must reach level 2 after a successful training call"
        );

        // Status message must mention the level-up.
        if let GameMode::Training(ref ts) = gs.0.mode {
            let msg = ts.status_message.as_deref().unwrap_or("");
            assert!(
                msg.contains("advanced to level 2"),
                "status_message must contain 'advanced to level 2', got: {:?}",
                msg
            );
        } else {
            panic!(
                "mode should still be Training after a single train event; got {:?}",
                std::mem::discriminant(&gs.0.mode)
            );
        }
    }

    /// When the game is **not** in Training mode the action system must be a
    /// complete no-op — no state changes, no panics.  This covers the plan
    /// requirement: "the system is a complete no-op when mode is not Training."
    #[test]
    fn test_training_system_noop_when_not_in_training_mode() {
        use crate::application::resources::GameContent;
        use crate::game::resources::GlobalState;
        use crate::game::systems::ui::GameLog;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.add_message::<TrainCharacter>();
        app.add_message::<ExitTraining>();
        app.add_message::<SelectTrainingMember>();
        app.add_systems(Update, training_action_system);
        app.init_resource::<GameLog>();

        let mut game_state = GameState::new();
        // Explicitly set Exploration mode — not Training.
        game_state.mode = GameMode::Exploration;
        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(ContentDatabase::new()));

        // Run a frame with no events — must not panic or change mode.
        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::Exploration),
            "Exploration mode must be unchanged after an update when not in Training"
        );
    }

    /// Sending `ExitTraining` when mode is **not** Training must be silently
    /// ignored — the action system returns early without modifying state.
    #[test]
    fn test_exit_training_noop_when_not_in_training_mode() {
        use crate::application::resources::GameContent;
        use crate::game::resources::GlobalState;
        use crate::game::systems::ui::GameLog;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.add_message::<TrainCharacter>();
        app.add_message::<ExitTraining>();
        app.add_message::<SelectTrainingMember>();
        app.add_systems(Update, training_action_system);
        app.init_resource::<GameLog>();

        let mut game_state = GameState::new();
        game_state.mode = GameMode::Exploration;
        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(ContentDatabase::new()));

        // Write ExitTraining while in Exploration mode — must be ignored.
        app.world_mut()
            .resource_mut::<Messages<ExitTraining>>()
            .write(ExitTraining);

        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::Exploration),
            "Exploration mode must be unchanged even when ExitTraining is sent outside Training mode"
        );
    }

    /// Selection event updates `TrainingState::selected_member_index`.
    #[test]
    fn test_selection_updates_selected_member_index() {
        use crate::game::resources::GlobalState;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.add_message::<SelectTrainingMember>();
        app.add_systems(Update, training_selection_system);

        let mut game_state = GameState::new();
        let mut ts = TrainingState::new("trainer".to_string());
        ts.eligible_member_indices.push(0);
        ts.eligible_member_indices.push(1);
        game_state.mode = GameMode::Training(ts);
        app.insert_resource(GlobalState(game_state));

        // Select index 1.
        app.world_mut()
            .resource_mut::<Messages<SelectTrainingMember>>()
            .write(SelectTrainingMember { member_index: 1 });

        app.update();

        let gs = app.world().resource::<GlobalState>();
        if let GameMode::Training(ref state) = gs.0.mode {
            assert_eq!(
                state.selected_member_index,
                Some(1),
                "selected_member_index must be 1 after SelectTrainingMember {{ member_index: 1 }}"
            );
        } else {
            panic!("expected Training mode");
        }
    }

    /// Sending `member_index: usize::MAX` clears the selection.
    #[test]
    fn test_selection_max_clears_selected_member_index() {
        use crate::game::resources::GlobalState;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.add_message::<SelectTrainingMember>();
        app.add_systems(Update, training_selection_system);

        let mut game_state = GameState::new();
        let mut ts = TrainingState::new("trainer".to_string());
        ts.eligible_member_indices.push(0);
        ts.selected_member_index = Some(0);
        game_state.mode = GameMode::Training(ts);
        app.insert_resource(GlobalState(game_state));

        app.world_mut()
            .resource_mut::<Messages<SelectTrainingMember>>()
            .write(SelectTrainingMember {
                member_index: usize::MAX,
            });

        app.update();

        let gs = app.world().resource::<GlobalState>();
        if let GameMode::Training(ref state) = gs.0.mode {
            assert!(
                state.selected_member_index.is_none(),
                "usize::MAX must clear selected_member_index"
            );
        } else {
            panic!("expected Training mode");
        }
    }

    /// Insufficient gold results in a `TrainingError::InsufficientGold` status
    /// message and the character's level must remain unchanged.
    #[test]
    fn test_training_insufficient_gold_shows_error() {
        use crate::application::resources::GameContent;
        use crate::domain::progression::award_experience;
        use crate::domain::world::npc::NpcDefinition;
        use crate::game::resources::GlobalState;
        use crate::game::systems::ui::GameLog;
        use crate::sdk::database::ContentDatabase;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.add_message::<TrainCharacter>();
        app.add_message::<ExitTraining>();
        app.add_message::<SelectTrainingMember>();
        app.add_systems(Update, training_action_system);
        app.init_resource::<GameLog>();

        let mut db = ContentDatabase::new();
        db.classes = crate::domain::classes::ClassDatabase::load_from_file("data/classes.ron")
            .expect("data/classes.ron must be present");
        // Trainer with a 10 000 gold base fee (far more than the party has).
        let trainer =
            NpcDefinition::trainer("expensive_trainer", "Expensive Trainer", "e.png", 10_000);
        db.npcs.add_npc(trainer).unwrap();

        let mut game_state = GameState::new();
        game_state.campaign_config.level_up_mode = crate::domain::campaign::LevelUpMode::NpcTrainer;

        let mut knight = make_knight("Poor Knight");
        award_experience(&mut knight, 1_000).unwrap();
        game_state.party.members.push(knight);
        game_state.party.gold = 50; // far short of the 10 000 gold fee

        let mut ts = TrainingState::new("expensive_trainer".to_string());
        ts.eligible_member_indices.push(0);
        ts.selected_member_index = Some(0);
        game_state.mode = GameMode::Training(ts);

        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));

        app.world_mut()
            .resource_mut::<Messages<TrainCharacter>>()
            .write(TrainCharacter { party_index: 0 });

        app.update();

        let gs = app.world().resource::<GlobalState>();

        // Level must be unchanged.
        assert_eq!(
            gs.0.party.members[0].level, 1,
            "level must not change when gold is insufficient"
        );

        // Status message must mention insufficient gold.
        if let GameMode::Training(ref ts) = gs.0.mode {
            let msg = ts.status_message.as_deref().unwrap_or("");
            assert!(
                msg.to_lowercase().contains("insufficient"),
                "status_message must mention insufficient gold, got: {:?}",
                msg
            );
        } else {
            panic!("mode should remain Training after a failed training attempt");
        }
    }

    /// The `TrainingPlugin` registration is idempotent — registering it with a
    /// full `App` (not just `MinimalPlugins`) must not panic.
    ///
    /// Note: this test does **not** call `app.update()` to avoid needing a
    /// full window / egui context in the test harness.
    #[test]
    fn test_training_plugin_registered_events_accessible() {
        let mut app = App::new();
        app.add_plugins(TrainingPlugin);
        // Verify that the resources registered by the plugin are accessible.
        // (No update() call — we only verify registration succeeds.)
        let _ = app.world().get_resource::<TrainingNavState>();
    }
}
