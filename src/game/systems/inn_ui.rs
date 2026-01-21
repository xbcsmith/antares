// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Inn UI System - Party management interface at inns
//!
//! Provides an egui-based interface for recruiting, dismissing, and swapping
//! party members when visiting an inn. This system is active when the game
//! is in `GameMode::InnManagement` mode.

use crate::application::GameMode;
use crate::domain::character::{CharacterLocation, PARTY_MAX_SIZE};
use crate::game::resources::GlobalState;
use crate::game::systems::ui::GameLog;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

/// Plugin for inn party management UI
pub struct InnUiPlugin;

impl Plugin for InnUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<InnRecruitCharacter>()
            .add_message::<InnDismissCharacter>()
            .add_message::<InnSwapCharacters>()
            .add_message::<ExitInn>()
            .add_message::<SelectPartyMember>()
            .add_message::<SelectRosterMember>()
            .init_resource::<InnNavigationState>()
            // Process keyboard input first, then selection, then UI, then actions
            .add_systems(
                Update,
                (
                    inn_input_system,
                    inn_selection_system,
                    inn_ui_system,
                    inn_action_system,
                )
                    .chain(),
            );
    }
}

// ===== Events =====

/// Event to recruit a character from the roster to the party
#[derive(Message)]
pub struct InnRecruitCharacter {
    /// Index in the full roster
    pub roster_index: usize,
}

/// Event to dismiss a character from the party to the current inn
#[derive(Message)]
pub struct InnDismissCharacter {
    /// Index in the party (0-5)
    pub party_index: usize,
}

/// Event to swap a party member with a roster member
#[derive(Message)]
pub struct InnSwapCharacters {
    /// Index in the party (0-5)
    pub party_index: usize,
    /// Index in the full roster
    pub roster_index: usize,
}

/// Event to exit the inn and return to exploration mode
#[derive(Message)]
pub struct ExitInn;

/// Event to select a party member (for mouse or keyboard selection)
#[derive(Message)]
pub struct SelectPartyMember {
    /// Index in the party (0-5), or usize::MAX to clear selection
    pub party_index: usize,
}

/// Event to select a roster member (for mouse or keyboard selection)
#[derive(Message)]
pub struct SelectRosterMember {
    /// Index in the full roster, or usize::MAX to clear selection
    pub roster_index: usize,
}

/// Tracks keyboard navigation state for inn party management
#[derive(Resource, Default)]
pub struct InnNavigationState {
    /// Selected party slot (0-5) for keyboard navigation
    pub selected_party_index: Option<usize>,
    /// Selected roster index for keyboard navigation (global roster index)
    pub selected_roster_index: Option<usize>,
    /// Which section has focus: Party(true) or Roster(false)
    pub focus_on_party: bool,
}

// ===== UI System =====

/// Renders the inn management UI when in InnManagement mode
#[allow(clippy::too_many_lines)]
#[allow(clippy::too_many_arguments)]
fn inn_ui_system(
    mut contexts: EguiContexts,
    global_state: Res<GlobalState>,
    nav_state: Res<InnNavigationState>,
    mut recruit_events: MessageWriter<InnRecruitCharacter>,
    mut dismiss_events: MessageWriter<InnDismissCharacter>,
    mut swap_events: MessageWriter<InnSwapCharacters>,
    mut exit_events: MessageWriter<ExitInn>,
    mut select_party_events: MessageWriter<SelectPartyMember>,
    mut select_roster_events: MessageWriter<SelectRosterMember>,
) {
    // Only render if we're in InnManagement mode
    let inn_state = match &global_state.0.mode {
        GameMode::InnManagement(state) => state,
        _ => return,
    };

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let current_inn_id = inn_state.current_inn_id.clone();
    let selected_party = inn_state.selected_party_slot;
    let selected_roster = inn_state.selected_roster_slot;

    // Main inn panel
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading(format!("Inn: Town {} - Party Management", current_inn_id));
        ui.add_space(10.0);

        // Active Party Section
        ui.label(egui::RichText::new("ACTIVE PARTY").size(16.0).strong());
        ui.label("Click a member to select for dismissal or swap");
        ui.add_space(5.0);

        ui.horizontal_wrapped(|ui| {
            let party_count = global_state.0.party.members.len();

            // Display party members
            for party_idx in 0..6 {
                let _is_selected = selected_party == Some(party_idx);

                if party_idx < party_count {
                    let member = &global_state.0.party.members[party_idx];

                    ui.group(|ui| {
                        ui.set_min_width(100.0);
                        ui.vertical(|ui| {
                            let is_mouse_selected = selected_party == Some(party_idx);
                            let is_keyboard_focused = nav_state.focus_on_party
                                && nav_state.selected_party_index == Some(party_idx);

                            let name_text = if is_keyboard_focused {
                                egui::RichText::new(&member.name)
                                    .strong()
                                    .color(egui::Color32::from_rgb(150, 220, 120))
                            // keyboard focus = GREEN
                            } else if is_mouse_selected {
                                egui::RichText::new(&member.name)
                                    .strong()
                                    .color(egui::Color32::YELLOW) // mouse selection = YELLOW
                            } else {
                                egui::RichText::new(&member.name)
                            };

                            if ui
                                .selectable_label(
                                    is_mouse_selected || is_keyboard_focused,
                                    name_text,
                                )
                                .clicked()
                            {
                                // Toggle selection
                                if is_mouse_selected {
                                    // Deselect if already selected
                                    select_party_events.write(SelectPartyMember {
                                        party_index: usize::MAX, // Special value to clear
                                    });
                                } else {
                                    select_party_events.write(SelectPartyMember {
                                        party_index: party_idx,
                                    });
                                }
                            }

                            ui.label(format!("Lvl {}", member.level));
                            ui.label(format!("HP: {}/{}", member.hp.current, member.hp.base));
                            ui.label(format!("SP: {}/{}", member.sp.current, member.sp.base));
                            ui.label(&member.class_id);
                            ui.label(&member.race_id);

                            ui.add_space(5.0);

                            // Dismiss button
                            if ui.button("Dismiss").clicked() {
                                dismiss_events.write(InnDismissCharacter {
                                    party_index: party_idx,
                                });
                            }
                        });
                    });
                } else {
                    // Empty slot
                    ui.group(|ui| {
                        ui.set_min_width(100.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("[Empty]").italics().weak());
                            ui.add_space(60.0);
                        });
                    });
                }
            }
        });

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

        // Available Characters Section
        ui.label(
            egui::RichText::new("AVAILABLE AT THIS INN")
                .size(16.0)
                .strong(),
        );
        ui.label("Click a character to recruit or swap");
        ui.add_space(5.0);

        // Find characters at this inn (exclude those currently in party)
        let mut inn_characters = Vec::new();
        for (roster_idx, character) in global_state.0.roster.characters.iter().enumerate() {
            if let Some(CharacterLocation::AtInn(inn_id)) =
                global_state.0.roster.character_locations.get(roster_idx)
            {
                if inn_id == &current_inn_id {
                    inn_characters.push((roster_idx, character));
                }
            }
        }

        if inn_characters.is_empty() {
            ui.label(egui::RichText::new("No characters available at this inn").italics());
        } else {
            ui.horizontal_wrapped(|ui| {
                for (roster_idx, character) in inn_characters {
                    let is_mouse_selected = selected_roster == Some(roster_idx);
                    let is_keyboard_focused = !nav_state.focus_on_party
                        && nav_state.selected_roster_index == Some(roster_idx);

                    ui.group(|ui| {
                        ui.set_min_width(120.0);
                        ui.vertical(|ui| {
                            let name_text = if is_keyboard_focused {
                                egui::RichText::new(&character.name)
                                    .strong()
                                    .color(egui::Color32::from_rgb(150, 220, 120))
                            // keyboard focus = GREEN
                            } else if is_mouse_selected {
                                egui::RichText::new(&character.name)
                                    .strong()
                                    .color(egui::Color32::YELLOW) // mouse selection = YELLOW
                            } else {
                                egui::RichText::new(&character.name)
                            };

                            // Mark selectable if either mouse or keyboard selected
                            if ui
                                .selectable_label(
                                    is_mouse_selected || is_keyboard_focused,
                                    name_text,
                                )
                                .clicked()
                            {
                                // Toggle selection
                                if is_mouse_selected {
                                    // Deselect if already selected
                                    select_roster_events.write(SelectRosterMember {
                                        roster_index: usize::MAX, // Special value to clear
                                    });
                                } else {
                                    select_roster_events.write(SelectRosterMember {
                                        roster_index: roster_idx,
                                    });
                                }
                            }

                            ui.label(&character.race_id);
                            ui.label(&character.class_id);
                            ui.label(format!("Lvl {}", character.level));
                            ui.label(format!(
                                "HP: {}/{}",
                                character.hp.current, character.hp.base
                            ));

                            ui.add_space(5.0);

                            // Recruit button (disabled if party full)
                            let party_full = global_state.0.party.members.len() >= PARTY_MAX_SIZE;
                            let button = egui::Button::new("Recruit");

                            if ui.add_enabled(!party_full, button).clicked() {
                                recruit_events.write(InnRecruitCharacter {
                                    roster_index: roster_idx,
                                });
                            }

                            if party_full {
                                ui.label(egui::RichText::new("Party full").small().weak());
                            }

                            // Swap button (enabled if party slot selected either by mouse or keyboard)
                            if let Some(party_idx) =
                                selected_party.or(nav_state.selected_party_index)
                            {
                                if ui.button("Swap").clicked() {
                                    swap_events.write(InnSwapCharacters {
                                        party_index: party_idx,
                                        roster_index: roster_idx,
                                    });
                                }
                            }
                        });
                    });
                }
            });
        }

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

        // Exit button - make it more prominent
        ui.horizontal(|ui| {
            if ui
                .add_sized(
                    [120.0, 30.0],
                    egui::Button::new(egui::RichText::new("Exit Inn").size(16.0)),
                )
                .clicked()
            {
                exit_events.write(ExitInn);
            }

            // Show ESC hint
            ui.label(
                egui::RichText::new("(or press ESC)")
                    .weak()
                    .color(egui::Color32::LIGHT_GREEN),
            );
        });

        // Instructions
        ui.add_space(10.0);
        ui.label(egui::RichText::new("Instructions:").weak());
        ui.label(
            egui::RichText::new("• Click Dismiss to send party member to this inn")
                .weak()
                .small(),
        );
        ui.label(
            egui::RichText::new("• Click Recruit to add character to party (if room)")
                .weak()
                .small(),
        );
        ui.label(
            egui::RichText::new(
                "• Select party member, then click Swap on inn character to exchange",
            )
            .weak()
            .small(),
        );
        ui.label(
            egui::RichText::new("• Press ESC or click Exit Inn to return to exploration")
                .weak()
                .small()
                .color(egui::Color32::from_rgb(144, 238, 144)),
        );
        ui.label(
            egui::RichText::new(
                "• Keyboard: TAB to switch focus, Arrow Keys to navigate, Enter/Space to select",
            )
            .weak()
            .small()
            .color(egui::Color32::LIGHT_BLUE),
        );
        ui.label(
            egui::RichText::new("• Keyboard: D to dismiss, R to recruit, S to swap")
                .weak()
                .small()
                .color(egui::Color32::LIGHT_BLUE),
        );
        ui.label(
            egui::RichText::new("• Mouse: Click to select, use buttons to perform actions")
                .weak()
                .small()
                .color(egui::Color32::LIGHT_YELLOW),
        );
    });
}

// ===== Action System =====
// ===== Action Handler System =====

fn inn_selection_system(
    mut select_party_events: MessageReader<SelectPartyMember>,
    mut select_roster_events: MessageReader<SelectRosterMember>,
    mut global_state: ResMut<GlobalState>,
) {
    // Handle party selection events
    for event in select_party_events.read() {
        if let GameMode::InnManagement(state) = &mut global_state.0.mode {
            if event.party_index == usize::MAX {
                // Clear selection
                state.selected_party_slot = None;
            } else {
                // Toggle selection
                if state.selected_party_slot == Some(event.party_index) {
                    state.selected_party_slot = None;
                } else {
                    state.selected_party_slot = Some(event.party_index);
                }
            }
        }
    }

    // Handle roster selection events
    for event in select_roster_events.read() {
        if let GameMode::InnManagement(state) = &mut global_state.0.mode {
            if event.roster_index == usize::MAX {
                // Clear selection
                state.selected_roster_slot = None;
            } else {
                // Toggle selection
                if state.selected_roster_slot == Some(event.roster_index) {
                    state.selected_roster_slot = None;
                } else {
                    state.selected_roster_slot = Some(event.roster_index);
                }
            }
        }
    }
}

fn inn_action_system(
    mut recruit_events: MessageReader<InnRecruitCharacter>,
    mut dismiss_events: MessageReader<InnDismissCharacter>,
    mut swap_events: MessageReader<InnSwapCharacters>,
    mut exit_events: MessageReader<ExitInn>,
    mut global_state: ResMut<GlobalState>,
    mut game_log: Option<ResMut<GameLog>>,
) {
    // Get current inn ID before processing events (clone to avoid moving out of state)
    let current_inn_id = match &global_state.0.mode {
        GameMode::InnManagement(state) => state.current_inn_id.clone(),
        _ => return, // Not in inn mode
    };

    // Process recruit events
    for event in recruit_events.read() {
        match global_state.0.recruit_character(event.roster_index) {
            Ok(_) => {
                if let Some(character) = global_state.0.roster.characters.get(event.roster_index) {
                    if let Some(ref mut log) = game_log {
                        log.add(format!("{} recruited to party!", character.name));
                    }
                }
            }
            Err(e) => {
                if let Some(ref mut log) = game_log {
                    log.add(format!("Cannot recruit: {}", e));
                }
            }
        }
    }

    // Process dismiss events
    for event in dismiss_events.read() {
        match global_state
            .0
            .dismiss_character(event.party_index, current_inn_id.clone())
        {
            Ok(_) => {
                if let Some(ref mut log) = game_log {
                    log.add("Party member dismissed to inn.".to_string());
                }
            }
            Err(e) => {
                if let Some(ref mut log) = game_log {
                    log.add(format!("Cannot dismiss: {}", e));
                }
            }
        }
    }

    // Process swap events
    for event in swap_events.read() {
        match global_state
            .0
            .swap_party_member(event.party_index, event.roster_index)
        {
            Ok(_) => {
                if let Some(ref mut log) = game_log {
                    log.add("Party members swapped!".to_string());
                }
            }
            Err(e) => {
                if let Some(ref mut log) = game_log {
                    log.add(format!("Cannot swap: {}", e));
                }
            }
        }
    }

    // Process exit events
    for _event in exit_events.read() {
        global_state.0.mode = GameMode::Exploration;
        if let Some(ref mut log) = game_log {
            log.add("Left the inn.".to_string());
        }
    }
}

/// Handles keyboard input for inn party management
#[allow(clippy::too_many_arguments)]
fn inn_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    global_state: Res<GlobalState>,
    mut nav_state: ResMut<InnNavigationState>,
    mut recruit_events: MessageWriter<InnRecruitCharacter>,
    mut dismiss_events: MessageWriter<InnDismissCharacter>,
    mut swap_events: MessageWriter<InnSwapCharacters>,
    mut exit_events: MessageWriter<ExitInn>,
    mut select_party_events: MessageWriter<SelectPartyMember>,
    mut select_roster_events: MessageWriter<SelectRosterMember>,
) {
    // Only process input when in InnManagement mode
    let inn_state = match &global_state.0.mode {
        GameMode::InnManagement(state) => state,
        _ => {
            *nav_state = InnNavigationState::default();
            return;
        }
    };

    let party_count = global_state.0.party.members.len();

    // Collect roster indices for characters at this inn (exclude InParty)
    let inn_roster_indices: Vec<usize> = global_state
        .0
        .roster
        .character_locations
        .iter()
        .enumerate()
        .filter_map(|(idx, loc)| {
            if let CharacterLocation::AtInn(inn_id) = loc {
                if inn_id == &inn_state.current_inn_id {
                    Some(idx)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let roster_count = inn_roster_indices.len();

    // ESC key exits inn management
    if keyboard.just_pressed(KeyCode::Escape) {
        exit_events.write(ExitInn);
        *nav_state = InnNavigationState::default();
        return;
    }

    // Tab key switches focus between party and roster
    if keyboard.just_pressed(KeyCode::Tab) {
        nav_state.focus_on_party = !nav_state.focus_on_party;
        // Clear selections when switching focus
        if nav_state.focus_on_party {
            nav_state.selected_roster_index = None;
        } else {
            nav_state.selected_party_index = None;
        }
    }

    // Arrow key navigation
    if nav_state.focus_on_party {
        if party_count == 0 {
            return;
        }

        // Navigate right (next)
        if keyboard.just_pressed(KeyCode::ArrowRight) {
            let next = match nav_state.selected_party_index {
                Some(i) => {
                    if i + 1 < party_count {
                        i + 1
                    } else {
                        0
                    }
                }
                None => 0,
            };
            nav_state.selected_party_index = Some(next);
        }

        // Navigate left (previous)
        if keyboard.just_pressed(KeyCode::ArrowLeft) {
            let prev = match nav_state.selected_party_index {
                Some(i) => {
                    if i > 0 {
                        i - 1
                    } else {
                        party_count.saturating_sub(1)
                    }
                }
                None => party_count.saturating_sub(1),
            };
            nav_state.selected_party_index = Some(prev);
        }

        // Enter/Space to select/dismiss party member
        if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
            if let Some(party_idx) = nav_state.selected_party_index {
                if party_idx < party_count {
                    // First select, then dismiss if already selected
                    select_party_events.write(SelectPartyMember {
                        party_index: party_idx,
                    });
                }
            }
        }

        // D key to dismiss selected party member
        if keyboard.just_pressed(KeyCode::KeyD) {
            if let Some(party_idx) = nav_state.selected_party_index {
                if party_idx < party_count {
                    dismiss_events.write(InnDismissCharacter {
                        party_index: party_idx,
                    });
                }
            }
        }
    } else {
        // Roster focused navigation
        if roster_count == 0 {
            nav_state.selected_roster_index = None;
        } else {
            if keyboard.just_pressed(KeyCode::ArrowRight) {
                let pos = nav_state.selected_roster_index.and_then(|global_idx| {
                    inn_roster_indices.iter().position(|&x| x == global_idx)
                });
                let next_pos = match pos {
                    Some(p) => {
                        if p + 1 < roster_count {
                            p + 1
                        } else {
                            0
                        }
                    }
                    None => 0,
                };
                nav_state.selected_roster_index = Some(inn_roster_indices[next_pos]);
            }

            if keyboard.just_pressed(KeyCode::ArrowLeft) {
                let pos = nav_state.selected_roster_index.and_then(|global_idx| {
                    inn_roster_indices.iter().position(|&x| x == global_idx)
                });
                let prev_pos = match pos {
                    Some(p) => {
                        if p > 0 {
                            p - 1
                        } else {
                            roster_count - 1
                        }
                    }
                    None => roster_count - 1,
                };
                nav_state.selected_roster_index = Some(inn_roster_indices[prev_pos]);
            }

            // Enter/Space to select roster character
            if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
                if let Some(roster_idx) = nav_state.selected_roster_index {
                    select_roster_events.write(SelectRosterMember {
                        roster_index: roster_idx,
                    });
                }
            }

            // R key to recruit selected roster character
            if keyboard.just_pressed(KeyCode::KeyR) {
                if let Some(roster_idx) = nav_state.selected_roster_index {
                    if party_count < PARTY_MAX_SIZE {
                        recruit_events.write(InnRecruitCharacter {
                            roster_index: roster_idx,
                        });
                    }
                }
            }

            // S key to swap (if both party and roster characters selected)
            if keyboard.just_pressed(KeyCode::KeyS) {
                if let (Some(party_idx), Some(roster_idx)) = (
                    nav_state.selected_party_index,
                    nav_state.selected_roster_index,
                ) {
                    swap_events.write(InnSwapCharacters {
                        party_index: party_idx,
                        roster_index: roster_idx,
                    });
                }
            }
        }
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{GameState, InnManagementState};
    use crate::domain::character::{Alignment, Character, CharacterLocation, Sex};

    #[test]
    fn test_inn_management_state_creation() {
        let state = InnManagementState::new("tutorial_innkeeper_town".to_string());
        assert_eq!(state.current_inn_id, "tutorial_innkeeper_town".to_string());
        assert_eq!(state.selected_party_slot, None);
        assert_eq!(state.selected_roster_slot, None);
    }

    #[test]
    fn test_inn_management_state_clear_selection() {
        let mut state = InnManagementState::new("tutorial_innkeeper_town".to_string());
        state.selected_party_slot = Some(2);
        state.selected_roster_slot = Some(5);

        state.clear_selection();

        assert_eq!(state.selected_party_slot, None);
        assert_eq!(state.selected_roster_slot, None);
    }

    #[test]
    fn test_game_mode_inn_management() {
        let state = InnManagementState::new("tutorial_innkeeper_town".to_string());
        let mode = GameMode::InnManagement(state.clone());

        assert!(matches!(mode, GameMode::InnManagement(_)));

        if let GameMode::InnManagement(inner) = mode {
            assert_eq!(inner.current_inn_id, "tutorial_innkeeper_town".to_string());
        }
    }

    #[test]
    fn test_recruit_character_from_inn() {
        let mut game_state = GameState::new();
        let inn_id = "tutorial_innkeeper_town".to_string();

        // Create character at inn
        let character = Character::new(
            "TestChar".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        game_state
            .roster
            .add_character(character, CharacterLocation::AtInn(inn_id.clone()))
            .unwrap();

        // Recruit character
        let result = game_state.recruit_character(0);
        assert!(result.is_ok());
        assert_eq!(game_state.party.members.len(), 1);
        assert_eq!(game_state.party.members[0].name, "TestChar");
    }

    #[test]
    fn test_dismiss_character_to_inn() {
        let mut game_state = GameState::new();
        let inn_id = "tutorial_innkeeper_town".to_string();

        // Create two characters in party (need 2+ to dismiss one)
        let char1 = Character::new(
            "PartyChar1".to_string(),
            "elf".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Neutral,
        );
        let char2 = Character::new(
            "PartyChar2".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        game_state
            .roster
            .add_character(char1, CharacterLocation::InParty)
            .unwrap();
        game_state
            .roster
            .add_character(char2, CharacterLocation::InParty)
            .unwrap();

        let char1_clone = game_state.roster.characters[0].clone();
        let char2_clone = game_state.roster.characters[1].clone();
        game_state.party.add_member(char1_clone).unwrap();
        game_state.party.add_member(char2_clone).unwrap();

        // Dismiss first character
        let result = game_state.dismiss_character(0, inn_id.clone());
        assert!(result.is_ok(), "Dismiss failed: {:?}", result.err());
        assert_eq!(game_state.party.members.len(), 1);
        assert!(matches!(
            game_state.roster.character_locations[0],
            CharacterLocation::AtInn(_)
        ));
    }

    #[test]
    fn test_swap_party_member_with_inn_character() {
        let mut game_state = GameState::new();
        let inn_id = "tutorial_innkeeper_town".to_string();

        // Create two characters
        let char1 = Character::new(
            "InParty".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let char2 = Character::new(
            "AtInn".to_string(),
            "dwarf".to_string(),
            "robber".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        game_state
            .roster
            .add_character(char1, CharacterLocation::InParty)
            .unwrap();
        game_state
            .roster
            .add_character(char2, CharacterLocation::AtInn(inn_id.clone()))
            .unwrap();

        let char_clone = game_state.roster.characters[0].clone();
        game_state.party.add_member(char_clone).unwrap();

        // Swap characters
        let result = game_state.swap_party_member(0, 1);
        assert!(result.is_ok());
        assert_eq!(game_state.party.members.len(), 1);
        assert_eq!(game_state.party.members[0].name, "AtInn");
    }

    #[test]
    fn test_recruit_fails_when_party_full() {
        let mut game_state = GameState::new();
        let inn_id = "tutorial_innkeeper_town".to_string();

        // Fill party with 6 members
        for i in 0..6 {
            let character = Character::new(
                format!("Member{}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            game_state
                .roster
                .add_character(character, CharacterLocation::InParty)
                .unwrap();
            let char_clone = game_state.roster.characters[i].clone();
            game_state.party.add_member(char_clone).unwrap();
        }

        // Add one more at inn
        let extra = Character::new(
            "Extra".to_string(),
            "elf".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        game_state
            .roster
            .add_character(extra, CharacterLocation::AtInn(inn_id))
            .unwrap();

        // Attempt to recruit should fail
        let result = game_state.recruit_character(6);
        assert!(result.is_err());
        assert_eq!(game_state.party.members.len(), 6);
    }

    #[test]
    fn test_dismiss_fails_when_party_empty() {
        let mut game_state = GameState::new();
        let inn_id = "tutorial_innkeeper_town".to_string();

        // Attempt to dismiss from empty party should fail
        let result = game_state.dismiss_character(0, inn_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_inn_ui_plugin_builds() {
        let mut app = App::new();
        app.add_plugins(InnUiPlugin);

        // Just verify plugin builds without errors
        // Plugin registered successfully if we reach here
    }
}
