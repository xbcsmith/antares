// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Temple / Priest Resurrection Service UI
//!
//! Provides an egui-based interface for the player to spend gold and gems at a
//! priest NPC to resurrect dead party members.  This system is active when the
//! game is in [`GameMode::TempleService`] mode, which is entered when the
//! player interacts with an NPC that has `is_priest: true` and a
//! `service_catalog` containing the `"resurrect"` service.
//!
//! ## Layout
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Temple of Healing — Priest: <NPC Name>          [Esc: Exit]│
//! ├─────────────────────────────────────────────────────────────┤
//! │  "The gods have mercy on the fallen. For 500 gold and       │
//! │   1 gem I can restore the breath of life."                  │
//! │                                                             │
//! │  Dead party members:                                        │
//! │  ┌──────────────────────────────────────────────────────┐  │
//! │  │  [►] Aldric the Knight — Cost: 500 gold, 1 gem       │  │
//! │  │  [ ] Selwyn the Archer — Cost: 500 gold, 1 gem       │  │
//! │  └──────────────────────────────────────────────────────┘  │
//! │                                                             │
//! │  Party gold: 1,200   Party gems: 3                          │
//! │                                                             │
//! │  [Resurrect Selected]          [Exit Temple]               │
//! │                                                             │
//! │  ── Status ─────────────────────────────────────────────── │
//! │  Aldric has been restored to life!                          │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Mode Transitions
//!
//! The dialogue system (or any code that knows an NPC `is_priest` with
//! `"resurrect"`) transitions to [`GameMode::TempleService`] by writing:
//!
//! ```rust,ignore
//! game_state.mode = GameMode::TempleService(TempleServiceState::new(npc_id));
//! ```
//!
//! Pressing **Escape**, clicking **Exit Temple**, or successfully resurrecting
//! the last dead member returns to [`GameMode::Exploration`].

use crate::application::resources::{perform_resurrection_service, GameContent};
use crate::application::GameMode;
use crate::domain::character::{Character, Party};
use crate::game::resources::GlobalState;
use crate::game::systems::ui::GameLog;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

// ── Marker component ─────────────────────────────────────────────────────────

/// Marker component attached to the root entity of the temple service UI.
///
/// Spawned by external setup code when entering [`GameMode::TempleService`].
/// `cleanup_temple_ui` despawns all entities that carry this component when
/// the session ends.
#[derive(Component)]
pub struct TempleUiRoot;

// ── Plugin ────────────────────────────────────────────────────────────────────

/// Bevy plugin for the priest temple resurrection service UI.
///
/// Registers events and systems needed to render the temple service panel,
/// handle player input, and dispatch resurrection actions.
pub struct TemplePlugin;

impl Plugin for TemplePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TempleResurrectCharacter>()
            .add_message::<ExitTemple>()
            .add_message::<SelectTempleMember>()
            .init_resource::<TempleNavState>()
            .add_systems(
                Update,
                (
                    temple_input_system,
                    temple_selection_system,
                    temple_ui_system,
                    temple_selection_system, // second pass: handle UI-generated selections
                    temple_action_system,
                )
                    .chain(),
            );
    }
}

// ── Events ───────────────────────────────────────────────────────────────────

/// Request to resurrect the party member at `character_index` via the active
/// priest NPC.
///
/// The action system calls
/// [`perform_resurrection_service`] on receipt.
#[derive(Message)]
pub struct TempleResurrectCharacter {
    /// Index into `game_state.party.members` for the character to revive.
    pub character_index: usize,
}

/// Request to leave the temple service UI and return to Exploration mode.
#[derive(Message)]
pub struct ExitTemple;

/// Request to change the highlighted dead party member in the UI list.
///
/// `member_index` is an index into the *visible* (dead-only) list, not into
/// `party.members` directly.  Use `usize::MAX` to clear the selection.
#[derive(Message)]
pub struct SelectTempleMember {
    /// Index into the visible dead-member list, or `usize::MAX` to clear.
    pub member_index: usize,
}

// ── Navigation resource ───────────────────────────────────────────────────────

/// Keyboard navigation state for the temple service UI.
///
/// Tracks which dead member (by index in the *visible* list) currently has
/// keyboard focus, and whether the Exit button is focused.
#[derive(Resource, Default)]
pub struct TempleNavState {
    /// Index in the visible dead-member list that has keyboard focus.
    pub focused_index: Option<usize>,
    /// Whether the Exit button has keyboard focus.
    pub focus_on_exit: bool,
}

// ── Pure helpers (no Bevy dependency; fully testable) ─────────────────────────

/// Returns the party members that are eligible for resurrection service.
///
/// A member is eligible when [`Condition::is_dead`] returns `true`.
/// Characters with `STONE` or `ERADICATED` conditions are automatically
/// excluded because `is_dead()` returns `false` for those values.
///
/// # Arguments
///
/// * `party` — The active party whose members are filtered.
///
/// # Returns
///
/// A `Vec` of `(party_index, &Character)` tuples, where `party_index` is the
/// index into `party.members`.  The order matches the original party order.
///
/// # Examples
///
/// ```
/// use antares::game::systems::temple_ui::visible_dead_members;
/// use antares::domain::character::{Character, Condition, Sex, Alignment, Party};
///
/// let mut party = Party::new();
///
/// // Alive member — not shown
/// let mut alive = Character::new(
///     "Gwyneth".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Female, Alignment::Good,
/// );
/// alive.hp.current = 20;
/// party.members.push(alive);
///
/// // Dead member — shown
/// let mut dead = Character::new(
///     "Aldric".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Male, Alignment::Good,
/// );
/// dead.hp.current = 0;
/// dead.conditions.add(Condition::DEAD);
/// party.members.push(dead);
///
/// let visible = visible_dead_members(&party);
/// assert_eq!(visible.len(), 1);
/// assert_eq!(visible[0].0, 1);          // party index of the dead member
/// assert_eq!(visible[0].1.name, "Aldric");
/// ```
pub fn visible_dead_members(party: &Party) -> Vec<(usize, &Character)> {
    party
        .members
        .iter()
        .enumerate()
        .filter(|(_, m)| m.conditions.is_dead())
        .collect()
}

// ── UI system ─────────────────────────────────────────────────────────────────

/// Renders the temple service panel when the game is in
/// [`GameMode::TempleService`] mode.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
fn temple_ui_system(
    mut contexts: EguiContexts,
    global_state: Res<GlobalState>,
    nav_state: Res<TempleNavState>,
    content: Res<GameContent>,
    mut resurrect_events: MessageWriter<TempleResurrectCharacter>,
    mut exit_events: MessageWriter<ExitTemple>,
    mut select_events: MessageWriter<SelectTempleMember>,
) {
    // Only render when in TempleService mode.
    let temple_state = match &global_state.0.mode {
        GameMode::TempleService(s) => s.clone(),
        _ => return,
    };

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    // Look up the NPC and service cost for display.
    let npc = content.db().npcs.get_npc(&temple_state.npc_id);
    let npc_name = npc.map(|n| n.name.as_str()).unwrap_or("Temple Priest");

    let service_cost = npc
        .and_then(|n| n.service_catalog.as_ref())
        .and_then(|c| c.get_service("resurrect"))
        .map(|s| (s.cost, s.gem_cost))
        .unwrap_or((500, 1));

    let dead_members = visible_dead_members(&global_state.0.party);
    let party_gold = global_state.0.party.gold;
    let party_gems = global_state.0.party.gems;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading(format!("Temple of Healing — Priest: {}", npc_name));
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                "\"The gods have mercy on the fallen. I can restore the breath of life.\"",
            )
            .italics()
            .color(egui::Color32::from_rgb(180, 200, 255)),
        );
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);

        // Party resources line
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("Party gold: {}", party_gold))
                    .color(egui::Color32::YELLOW),
            );
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new(format!("Party gems: {}", party_gems))
                    .color(egui::Color32::from_rgb(100, 200, 255)),
            );
        });
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(format!(
                "Resurrection cost: {} gold, {} gem(s)",
                service_cost.0, service_cost.1
            ))
            .color(egui::Color32::from_rgb(220, 180, 80)),
        );
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Dead party members list
        ui.label(
            egui::RichText::new("Dead party members:")
                .size(15.0)
                .strong(),
        );
        ui.add_space(4.0);

        if dead_members.is_empty() {
            ui.label(
                egui::RichText::new("No dead party members — may the gods keep them safe.")
                    .italics()
                    .weak(),
            );
        } else {
            let can_afford = party_gold >= service_cost.0 && party_gems >= service_cost.1;

            egui::ScrollArea::vertical()
                .id_salt("temple_dead_list")
                .max_height(240.0)
                .show(ui, |ui| {
                    for (list_idx, (party_idx, member)) in dead_members.iter().enumerate() {
                        ui.push_id(format!("temple_member_{}", list_idx), |ui| {
                            let is_keyboard = nav_state.focused_index == Some(list_idx);
                            let is_selected = temple_state.selected_member_index == Some(list_idx);
                            let is_active = is_keyboard || is_selected;

                            let mut frame = egui::Frame::group(ui.style());
                            if is_active {
                                frame = frame
                                    .fill(egui::Color32::from_rgba_premultiplied(80, 80, 0, 120))
                                    .stroke(egui::Stroke::new(2.0, egui::Color32::YELLOW));
                            } else {
                                frame = frame.fill(egui::Color32::from_gray(30));
                            }

                            frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // Selection indicator
                                    if is_active {
                                        ui.label(
                                            egui::RichText::new("►").color(egui::Color32::YELLOW),
                                        );
                                    } else {
                                        ui.label("  ");
                                    }

                                    ui.vertical(|ui| {
                                        let name_text = egui::RichText::new(&member.name)
                                            .strong()
                                            .color(if is_keyboard {
                                                egui::Color32::from_rgb(150, 220, 120)
                                            } else if is_selected {
                                                egui::Color32::YELLOW
                                            } else {
                                                egui::Color32::WHITE
                                            });
                                        ui.label(name_text);
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} — Level {}",
                                                member.class_id, member.level
                                            ))
                                            .small()
                                            .weak(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Cost: {} gold, {} gem(s)",
                                                service_cost.0, service_cost.1
                                            ))
                                            .small()
                                            .color(egui::Color32::from_rgb(220, 180, 80)),
                                        );
                                    });

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            // Select button
                                            if ui.button("Select").clicked() {
                                                select_events.write(SelectTempleMember {
                                                    member_index: list_idx,
                                                });
                                            }

                                            // Resurrect button (enabled only when affordable
                                            // and this member is selected)
                                            let resurrect_enabled = can_afford && is_active;
                                            if ui
                                                .add_enabled(
                                                    resurrect_enabled,
                                                    egui::Button::new(
                                                        egui::RichText::new("Resurrect").color(
                                                            if resurrect_enabled {
                                                                egui::Color32::from_rgb(
                                                                    80, 220, 120,
                                                                )
                                                            } else {
                                                                egui::Color32::GRAY
                                                            },
                                                        ),
                                                    ),
                                                )
                                                .clicked()
                                            {
                                                resurrect_events.write(TempleResurrectCharacter {
                                                    character_index: *party_idx,
                                                });
                                            }
                                        },
                                    );
                                });
                            });
                            ui.add_space(4.0);
                        });
                    }
                });
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);

        // Status / error message
        if let Some(ref msg) = temple_state.status_message {
            let is_error = msg.contains("Insufficient")
                || msg.contains("permadeath")
                || msg.contains("not dead")
                || msg.contains("not found");

            let text = if is_error {
                egui::RichText::new(msg.as_str()).color(egui::Color32::RED)
            } else {
                egui::RichText::new(msg.as_str()).color(egui::Color32::from_rgb(80, 220, 120))
            };
            ui.label(text);
            ui.add_space(6.0);
        }

        // Exit button
        ui.horizontal(|ui| {
            let exit_text = if nav_state.focus_on_exit {
                egui::RichText::new("Exit Temple")
                    .strong()
                    .size(15.0)
                    .color(egui::Color32::from_rgb(144, 238, 144))
            } else {
                egui::RichText::new("Exit Temple").size(15.0)
            };

            if ui
                .add_sized([130.0, 28.0], egui::Button::new(exit_text))
                .clicked()
            {
                exit_events.write(ExitTemple);
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
                "• Click Select or use Arrow Keys to highlight a dead party member",
            )
            .weak()
            .small(),
        );
        ui.label(
            egui::RichText::new(
                "• Click Resurrect or press Enter/Space to revive (costs gold & gems)",
            )
            .weak()
            .small(),
        );
        ui.label(
            egui::RichText::new("• Press ESC or click Exit Temple to leave")
                .weak()
                .small()
                .color(egui::Color32::from_rgb(144, 238, 144)),
        );
    });
}

// ── Selection system ──────────────────────────────────────────────────────────

/// Updates `TempleServiceState::selected_member_index` from
/// [`SelectTempleMember`] events.
fn temple_selection_system(
    mut select_events: MessageReader<SelectTempleMember>,
    mut global_state: ResMut<GlobalState>,
) {
    for ev in select_events.read() {
        if let GameMode::TempleService(ref mut ts) = global_state.0.mode {
            if ev.member_index == usize::MAX {
                ts.selected_member_index = None;
            } else {
                ts.selected_member_index = Some(ev.member_index);
            }
        }
    }
}

// ── Action system ─────────────────────────────────────────────────────────────

/// Handles [`TempleResurrectCharacter`] and [`ExitTemple`] events.
fn temple_action_system(
    mut resurrect_events: MessageReader<TempleResurrectCharacter>,
    mut exit_events: MessageReader<ExitTemple>,
    mut global_state: ResMut<GlobalState>,
    content: Res<GameContent>,
    mut game_log: Option<ResMut<GameLog>>,
) {
    // Collect npc_id while in temple mode so we can call perform_resurrection_service
    let npc_id = match &global_state.0.mode {
        GameMode::TempleService(s) => s.npc_id.clone(),
        _ => return,
    };

    // Process resurrection requests
    for ev in resurrect_events.read() {
        let character_name = global_state
            .0
            .party
            .members
            .get(ev.character_index)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| format!("member {}", ev.character_index));

        match perform_resurrection_service(
            &mut global_state.0,
            &npc_id,
            ev.character_index,
            content.db(),
        ) {
            Ok(()) => {
                let msg = format!("{} has been restored to life!", character_name);
                if let Some(ref mut log) = game_log {
                    log.add_dialogue(msg.clone());
                }
                if let GameMode::TempleService(ref mut ts) = global_state.0.mode {
                    ts.status_message = Some(msg);
                    ts.selected_member_index = None;
                }
            }
            Err(e) => {
                // Surface the error back in the UI status message
                if let Some(ref mut log) = game_log {
                    log.add_system(format!("Resurrection failed: {}", e));
                }
                if let GameMode::TempleService(ref mut ts) = global_state.0.mode {
                    ts.status_message = Some(e);
                }
            }
        }
    }

    // Process exit requests
    for _ev in exit_events.read() {
        global_state.0.mode = GameMode::Exploration;
        if let Some(ref mut log) = game_log {
            log.add_exploration("Left the temple.".to_string());
        }
    }
}

// ── Input system ──────────────────────────────────────────────────────────────

/// Handles keyboard input for the temple service UI.
#[allow(clippy::too_many_lines)]
fn temple_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    global_state: Res<GlobalState>,
    mut nav_state: ResMut<TempleNavState>,
    mut resurrect_events: MessageWriter<TempleResurrectCharacter>,
    mut exit_events: MessageWriter<ExitTemple>,
    mut select_events: MessageWriter<SelectTempleMember>,
) {
    // Only process input in temple mode
    let _temple_state = match &global_state.0.mode {
        GameMode::TempleService(s) => s,
        _ => {
            *nav_state = TempleNavState::default();
            return;
        }
    };

    let dead_members = visible_dead_members(&global_state.0.party);
    let list_len = dead_members.len();

    // ESC or Tab → exit (when exit button is focused) / shift focus
    if keyboard.just_pressed(KeyCode::Escape) {
        exit_events.write(ExitTemple);
        return;
    }

    // Arrow Down / Arrow Up — navigate through the dead-member list
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        nav_state.focus_on_exit = false;
        if list_len > 0 {
            let next = match nav_state.focused_index {
                None => 0,
                Some(i) => (i + 1).min(list_len - 1),
            };
            nav_state.focused_index = Some(next);
            select_events.write(SelectTempleMember { member_index: next });
        }
    } else if keyboard.just_pressed(KeyCode::ArrowUp) {
        nav_state.focus_on_exit = false;
        if list_len > 0 {
            let prev = match nav_state.focused_index {
                None => 0,
                Some(0) => 0,
                Some(i) => i - 1,
            };
            nav_state.focused_index = Some(prev);
            select_events.write(SelectTempleMember { member_index: prev });
        }
    }

    // Tab — toggle between list and Exit button
    if keyboard.just_pressed(KeyCode::Tab) {
        nav_state.focus_on_exit = !nav_state.focus_on_exit;
        if nav_state.focus_on_exit {
            nav_state.focused_index = None;
        } else if list_len > 0 {
            nav_state.focused_index = Some(0);
            select_events.write(SelectTempleMember { member_index: 0 });
        }
    }

    // Enter / Space — confirm (Exit if exit has focus; else resurrect selected)
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        if nav_state.focus_on_exit {
            exit_events.write(ExitTemple);
        } else if let Some(list_idx) = nav_state.focused_index {
            if let Some((party_idx, _)) = dead_members.get(list_idx) {
                resurrect_events.write(TempleResurrectCharacter {
                    character_index: *party_idx,
                });
            }
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, Condition, Sex};

    // ── helpers ────────────────────────────────────────────────────────────────

    fn make_alive(name: &str) -> Character {
        let mut c = Character::new(
            name.to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.base = 20;
        c.hp.current = 15;
        c
    }

    fn make_dead(name: &str) -> Character {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};
        let mut c = Character::new(
            name.to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.base = 20;
        c.hp.current = 0;
        c.conditions.add(Condition::DEAD);
        c.add_condition(ActiveCondition::new(
            "dead".to_string(),
            ConditionDuration::Permanent,
        ));
        c
    }

    fn make_stone(name: &str) -> Character {
        let mut c = Character::new(
            name.to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.current = 0;
        c.conditions.add(Condition::STONE);
        c
    }

    fn make_eradicated(name: &str) -> Character {
        let mut c = Character::new(
            name.to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.current = 0;
        c.conditions.add(Condition::ERADICATED);
        c
    }

    fn make_unconscious(name: &str) -> Character {
        let mut c = Character::new(
            name.to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.current = 0;
        c.conditions.add(Condition::UNCONSCIOUS);
        c
    }

    // ── visible_dead_members tests ─────────────────────────────────────────────

    /// `visible_dead_members` returns only members where `is_dead()` is true.
    ///
    /// Alive, unconscious, stone, and eradicated members must not appear.
    #[test]
    fn test_temple_ui_shows_dead_members_only() {
        let mut party = Party::new();
        party.members.push(make_alive("Gwyneth")); // alive    — excluded
        party.members.push(make_dead("Aldric")); // dead     — included
        party.members.push(make_unconscious("Mira")); // unconscious — excluded
        party.members.push(make_dead("Bram")); // dead     — included

        let visible = visible_dead_members(&party);

        assert_eq!(
            visible.len(),
            2,
            "only the two dead members should be visible"
        );

        // Verify we got the right party indices
        let indices: Vec<usize> = visible.iter().map(|(i, _)| *i).collect();
        assert!(
            indices.contains(&1),
            "party index 1 (Aldric) must be visible"
        );
        assert!(indices.contains(&3), "party index 3 (Bram) must be visible");

        // Verify names
        let names: Vec<&str> = visible.iter().map(|(_, c)| c.name.as_str()).collect();
        assert!(
            names.contains(&"Aldric"),
            "Aldric must appear in visible list"
        );
        assert!(names.contains(&"Bram"), "Bram must appear in visible list");
        assert!(!names.contains(&"Gwyneth"), "alive Gwyneth must not appear");
        assert!(!names.contains(&"Mira"), "unconscious Mira must not appear");
    }

    /// `visible_dead_members` must exclude characters with the `STONE` condition.
    ///
    /// `Condition::is_dead()` returns `false` for STONE because the STONE value
    /// (160) is ≥ `Condition::STONE`, which signals a worse-than-dead state that
    /// requires different handling (e.g. de-stoning, not resurrection).
    #[test]
    fn test_temple_ui_does_not_show_eradicated() {
        let mut party = Party::new();
        party.members.push(make_dead("Aldric")); // dead         — shown
        party.members.push(make_stone("Selwyn")); // stone        — NOT shown
        party.members.push(make_eradicated("Thorn")); // eradicated   — NOT shown
        party.members.push(make_alive("Gwyneth")); // alive        — NOT shown

        let visible = visible_dead_members(&party);

        assert_eq!(
            visible.len(),
            1,
            "only the plain-dead member should be visible, got {} entries",
            visible.len()
        );
        assert_eq!(
            visible[0].1.name, "Aldric",
            "only Aldric (plain dead) should appear"
        );

        let names: Vec<&str> = visible.iter().map(|(_, c)| c.name.as_str()).collect();
        assert!(!names.contains(&"Selwyn"), "stone Selwyn must not appear");
        assert!(
            !names.contains(&"Thorn"),
            "eradicated Thorn must not appear"
        );
        assert!(!names.contains(&"Gwyneth"), "alive Gwyneth must not appear");
    }

    /// An empty party produces an empty visible list.
    #[test]
    fn test_visible_dead_members_empty_party() {
        let party = Party::new();
        let visible = visible_dead_members(&party);
        assert!(visible.is_empty(), "empty party must return empty list");
    }

    /// A party with only living members produces an empty visible list.
    #[test]
    fn test_visible_dead_members_all_alive() {
        let mut party = Party::new();
        party.members.push(make_alive("Alice"));
        party.members.push(make_alive("Bob"));
        let visible = visible_dead_members(&party);
        assert!(
            visible.is_empty(),
            "party with only living members must return empty list"
        );
    }

    /// A party with only dead members produces a list of all of them.
    #[test]
    fn test_visible_dead_members_all_dead() {
        let mut party = Party::new();
        party.members.push(make_dead("Alice"));
        party.members.push(make_dead("Bob"));
        party.members.push(make_dead("Carol"));
        let visible = visible_dead_members(&party);
        assert_eq!(
            visible.len(),
            3,
            "party with only dead members must return all three"
        );
    }

    /// Party indices in the returned tuples match the actual indices into
    /// `party.members`.
    #[test]
    fn test_visible_dead_members_correct_party_indices() {
        let mut party = Party::new();
        party.members.push(make_alive("Slot0")); // index 0 — alive, excluded
        party.members.push(make_dead("Slot1")); // index 1 — dead,  included
        party.members.push(make_stone("Slot2")); // index 2 — stone, excluded
        party.members.push(make_dead("Slot3")); // index 3 — dead,  included

        let visible = visible_dead_members(&party);
        assert_eq!(visible.len(), 2);
        assert_eq!(
            visible[0].0, 1,
            "first visible entry should map to party index 1"
        );
        assert_eq!(
            visible[1].0, 3,
            "second visible entry should map to party index 3"
        );
    }

    // ── TempleServiceState tests ───────────────────────────────────────────────

    /// `TempleServiceState::new` initialises with no selection and no message.
    #[test]
    fn test_temple_service_state_new() {
        use crate::application::TempleServiceState;

        let state = TempleServiceState::new("temple_priest".to_string());
        assert_eq!(state.npc_id, "temple_priest");
        assert!(state.selected_member_index.is_none());
        assert!(state.status_message.is_none());
    }

    /// `TempleServiceState::clear` resets selection and status message.
    #[test]
    fn test_temple_service_state_clear() {
        use crate::application::TempleServiceState;

        let mut state = TempleServiceState::new("temple_priest".to_string());
        state.selected_member_index = Some(2);
        state.status_message = Some("Resurrection complete!".to_string());

        state.clear();

        assert!(
            state.selected_member_index.is_none(),
            "clear() must reset selected_member_index"
        );
        assert!(
            state.status_message.is_none(),
            "clear() must reset status_message"
        );
        // npc_id must be preserved
        assert_eq!(state.npc_id, "temple_priest");
    }

    /// `GameMode::TempleService` can be created and pattern-matched.
    #[test]
    fn test_game_mode_temple_service_variant() {
        use crate::application::{GameMode, TempleServiceState};

        let mode = GameMode::TempleService(TempleServiceState::new("priest_01".to_string()));
        assert!(
            matches!(mode, GameMode::TempleService(_)),
            "TempleService variant must be matchable"
        );
    }

    /// `TemplePlugin` builds without panicking (smoke test).
    #[test]
    fn test_temple_plugin_builds() {
        use bevy::prelude::App;
        let mut app = App::new();
        app.add_plugins(TemplePlugin);
        // Just verify plugin builds without errors.
        // Plugin registered successfully if we reach here.
    }
}
