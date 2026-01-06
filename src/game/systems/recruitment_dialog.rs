// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::application::resources::GameContent;
use crate::application::{GameMode, RecruitResult};
use crate::game::resources::GlobalState;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

/// Plugin that handles recruitment dialog UI when encountering NPCs on maps
pub struct RecruitmentDialogPlugin;

impl Plugin for RecruitmentDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RecruitmentDialogMessage>()
            .add_message::<RecruitmentResponseMessage>()
            .add_systems(
                Update,
                (show_recruitment_dialog, process_recruitment_responses),
            );
    }
}

/// Message to trigger recruitment dialog display
#[derive(Message, Clone)]
pub struct RecruitmentDialogMessage {
    pub character_id: String,
    pub character_name: String,
    pub character_description: String,
}

/// Message sent when player responds to recruitment dialog
#[derive(Message, Clone)]
pub enum RecruitmentResponseMessage {
    Accept(String),  // character_id
    Decline(String), // character_id
}

/// Tracks active recruitment dialog state
#[derive(Resource, Default)]
pub struct RecruitmentDialogState {
    pub active_dialog: Option<RecruitmentDialogMessage>,
}

/// System that displays the recruitment dialog UI
fn show_recruitment_dialog(
    mut contexts: EguiContexts,
    mut dialog_messages: MessageReader<RecruitmentDialogMessage>,
    mut dialog_state: Local<Option<RecruitmentDialogMessage>>,
    mut response_writer: MessageWriter<RecruitmentResponseMessage>,
    global_state: Res<GlobalState>,
) {
    // Check for new recruitment dialog triggers
    for msg in dialog_messages.read() {
        *dialog_state = Some(msg.clone());
    }

    // Only show dialog if we have an active one
    let Some(dialog_data) = dialog_state.as_ref() else {
        return;
    };

    // Clone dialog data to avoid borrow issues in the closure
    let dialog = dialog_data.clone();

    // Only show in exploration mode
    if !matches!(global_state.0.mode, GameMode::Exploration) {
        return;
    }

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    egui::Window::new("Character Encounter")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                // Character portrait placeholder
                ui.label(
                    egui::RichText::new("🧑")
                        .size(64.0)
                        .color(egui::Color32::LIGHT_BLUE),
                );

                ui.add_space(10.0);

                // Character name
                ui.label(
                    egui::RichText::new(&dialog.character_name)
                        .size(20.0)
                        .strong(),
                );

                ui.add_space(5.0);

                // Character description
                ui.label(&dialog.character_description);

                ui.add_space(10.0);

                // Check if party is full
                let party_full =
                    global_state.0.party.size() >= crate::domain::character::Party::MAX_MEMBERS;

                if party_full {
                    ui.label(
                        egui::RichText::new("Your party is full!").color(egui::Color32::YELLOW),
                    );

                    if let Some(inn_id) = global_state.0.find_nearest_inn() {
                        ui.label(format!(
                            "{} can meet you at the inn in town {}.",
                            dialog.character_name, inn_id
                        ));
                    }

                    ui.add_space(10.0);
                }

                // Recruitment prompt
                ui.label(
                    egui::RichText::new("Will you join our party?")
                        .italics()
                        .size(16.0),
                );

                ui.add_space(15.0);

                // Action buttons
                ui.horizontal(|ui| {
                    let accept_label = if party_full {
                        "Send to inn"
                    } else {
                        "Yes, join us!"
                    };

                    if ui.button(accept_label).clicked() {
                        response_writer.write(RecruitmentResponseMessage::Accept(
                            dialog.character_id.clone(),
                        ));
                        *dialog_state = None;
                    }

                    if ui.button("Not now").clicked() {
                        response_writer.write(RecruitmentResponseMessage::Decline(
                            dialog.character_id.clone(),
                        ));
                        *dialog_state = None;
                    }
                });
            });
        });
}

/// System that processes recruitment responses and updates game state
fn process_recruitment_responses(
    mut response_messages: MessageReader<RecruitmentResponseMessage>,
    mut global_state: ResMut<GlobalState>,
    content: Res<GameContent>,
) {
    for response in response_messages.read() {
        match response {
            RecruitmentResponseMessage::Accept(character_id) => {
                match global_state.0.recruit_from_map(character_id, content.db()) {
                    Ok(RecruitResult::AddedToParty) => {
                        info!("Character '{}' joined the party", character_id);
                    }
                    Ok(RecruitResult::SentToInn(inn_id)) => {
                        info!(
                            "Character '{}' sent to inn {} (party full)",
                            character_id, inn_id
                        );
                    }
                    Ok(RecruitResult::Declined) => {
                        // This shouldn't happen from Accept response
                        warn!("Unexpected Declined result from recruitment");
                    }
                    Err(e) => {
                        error!("Failed to recruit character '{}': {}", character_id, e);
                    }
                }
            }
            RecruitmentResponseMessage::Decline(character_id) => {
                info!("Player declined to recruit character '{}'", character_id);
                // Don't mark as encountered - player can return later
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameState;
    use crate::domain::character::{Alignment, Character, CharacterLocation, Sex};
    use crate::domain::character_definition::CharacterDefinition;
    use crate::sdk::database::ContentDatabase;

    fn create_test_character_def() -> CharacterDefinition {
        CharacterDefinition::new(
            "test_npc".to_string(),
            "Test NPC".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        )
    }

    fn create_test_content_db() -> ContentDatabase {
        let mut db = ContentDatabase::new();

        // Add test character definition
        let char_def = create_test_character_def();
        db.characters.add_character(char_def).unwrap();

        // Add minimal race and class data
        db.races = crate::domain::races::RaceDatabase::load_from_string(
            r#"[
            (
                id: "human",
                name: "Human",
                stat_modifiers: (might: 0, intellect: 0, personality: 0, endurance: 0, speed: 0, accuracy: 0, luck: 0),
                description: "Test human",
            ),
        ]"#,
        )
        .unwrap();

        db.classes = crate::domain::classes::ClassDatabase::load_from_string(
            r#"[
            (
                id: "knight",
                name: "Knight",
                description: "Test knight",
                hp_die: (count: 1, sides: 10, bonus: 0),
                spell_school: None,
                is_pure_caster: false,
                spell_stat: None,
                special_abilities: [],
                starting_weapon_id: None,
                starting_armor_id: None,
                hp_per_level: 10,
                sp_per_level: 0,
                base_thac0: 10,
                thac0_per_level: 1,
                stat_modifiers: (might: 2, intellect: 0, personality: 0, endurance: 2, speed: 0, accuracy: 0, luck: 0),
                usable_items: All,
                usable_armor: All,
                usable_weapons: All,
                learnable_spell_levels: None,
            ),
        ]"#,
        )
        .unwrap();

        db
    }

    #[test]
    fn test_recruit_from_map_adds_to_party_when_space_available() {
        let mut state = GameState::new();
        let content_db = create_test_content_db();

        // Party has room (0/6)
        assert_eq!(state.party.size(), 0);

        let result = state.recruit_from_map("test_npc", &content_db).unwrap();

        assert_eq!(result, RecruitResult::AddedToParty);
        assert_eq!(state.party.size(), 1);
        assert_eq!(state.roster.characters.len(), 1);
        assert!(state.encountered_characters.contains("test_npc"));
    }

    #[test]
    fn test_recruit_from_map_sends_to_inn_when_party_full() {
        let mut state = GameState::new();
        let content_db = create_test_content_db();

        // Fill the party to max capacity
        for i in 0..crate::domain::character::Party::MAX_MEMBERS {
            let char = Character::new(
                format!("Hero{}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            state.party.add_member(char.clone()).unwrap();
            state
                .roster
                .add_character(char, CharacterLocation::InParty)
                .unwrap();
        }

        assert_eq!(
            state.party.size(),
            crate::domain::character::Party::MAX_MEMBERS
        );

        let result = state.recruit_from_map("test_npc", &content_db).unwrap();

        // Should send to inn since party is full
        assert!(matches!(result, RecruitResult::SentToInn(_)));
        assert_eq!(
            state.party.size(),
            crate::domain::character::Party::MAX_MEMBERS
        ); // Party size unchanged
        assert_eq!(state.roster.characters.len(), 7); // Roster has original 6 + new character
        assert!(state.encountered_characters.contains("test_npc"));
    }

    #[test]
    fn test_recruit_from_map_prevents_duplicate_recruitment() {
        let mut state = GameState::new();
        let content_db = create_test_content_db();

        // First recruitment succeeds
        let result1 = state.recruit_from_map("test_npc", &content_db);
        assert!(result1.is_ok());

        // Second recruitment fails - already encountered
        let result2 = state.recruit_from_map("test_npc", &content_db);
        assert!(result2.is_err());
        assert!(matches!(
            result2,
            Err(crate::application::RecruitmentError::AlreadyEncountered(_))
        ));
    }

    #[test]
    fn test_recruit_from_map_character_not_found() {
        let mut state = GameState::new();
        let content_db = create_test_content_db();

        let result = state.recruit_from_map("nonexistent_npc", &content_db);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(crate::application::RecruitmentError::CharacterNotFound(_))
        ));
    }

    #[test]
    fn test_find_nearest_inn_returns_campaign_starting_inn() {
        // Without campaign - returns None
        let state = GameState::new();
        assert_eq!(state.find_nearest_inn(), None);

        // With campaign - would return starting_inn
        // (tested in integration tests with actual campaign data)
    }

    #[test]
    fn test_encountered_characters_tracking() {
        let mut state = GameState::new();
        let content_db = create_test_content_db();

        assert!(state.encountered_characters.is_empty());

        state.recruit_from_map("test_npc", &content_db).unwrap();

        assert_eq!(state.encountered_characters.len(), 1);
        assert!(state.encountered_characters.contains("test_npc"));
    }

    #[test]
    fn test_recruitment_dialog_message_creation() {
        let msg = RecruitmentDialogMessage {
            character_id: "npc_gareth".to_string(),
            character_name: "Old Gareth".to_string(),
            character_description: "A grizzled dwarf veteran".to_string(),
        };

        assert_eq!(msg.character_id, "npc_gareth");
        assert_eq!(msg.character_name, "Old Gareth");
    }

    #[test]
    fn test_recruitment_response_accept() {
        let response = RecruitmentResponseMessage::Accept("npc_test".to_string());

        match response {
            RecruitmentResponseMessage::Accept(id) => {
                assert_eq!(id, "npc_test");
            }
            _ => panic!("Expected Accept variant"),
        }
    }

    #[test]
    fn test_recruitment_response_decline() {
        let response = RecruitmentResponseMessage::Decline("npc_test".to_string());

        match response {
            RecruitmentResponseMessage::Decline(id) => {
                assert_eq!(id, "npc_test");
            }
            _ => panic!("Expected Decline variant"),
        }
    }
}
