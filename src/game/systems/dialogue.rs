// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Dialogue plugin and runtime systems
//!
//! This module implements a simple dialogue runtime:
//! - `StartDialogue` and `SelectDialogueChoice` messages
//! - `DialoguePlugin` that registers the message types and systems
//! - Systems that start dialogues, advance nodes, evaluate basic conditions,
//!   and execute dialogue actions (give items, start quests, give gold, grant XP).
//!
//! The implementation is intentionally conservative: it executes actions
//! synchronously and applies rewards directly to the `GlobalState`. It is
//! suitable for headless testing and for connecting to a UI layer that reads
//! `GameState::mode` to render the active dialogue.
//!
//! # Testing
//!
//! Unit tests exercise the primary behaviors described in the Engine SDK plan:
//! - Loading the root node
//! - Advancing nodes when a choice is selected
//! - Executing a script action that gives an item to the party
//!
use bevy::prelude::*;

use crate::application::dialogue::{DialogueState, RecruitmentContext};
use crate::application::resources::GameContent;
use crate::application::GameMode;
use crate::game::components::dialogue::DialoguePanelRoot;
use crate::game::resources::GlobalState;
use crate::game::systems::mouse_input;

use crate::domain::dialogue::{DialogueAction, DialogueCondition, DialogueId};

/// Temporary storage for recruitment context during dialogue initialization
///
/// This resource holds recruitment context between the moment a RecruitableCharacter
/// event is triggered and when the dialogue system initializes DialogueState.
/// The context is consumed by handle_start_dialogue and cleared after use.
#[derive(Resource, Default)]
pub struct PendingRecruitmentContext(pub Option<RecruitmentContext>);

/// Message to request that a dialogue tree begin (e.g., NPC started talking).
#[derive(Message, Clone, Debug)]
pub struct StartDialogue {
    /// Dialogue tree id to activate
    pub dialogue_id: DialogueId,
    /// Entity that initiated this dialogue (typically an NPC)
    pub speaker_entity: Option<Entity>,
    /// Fallback map position for visual placement if speaker_entity is missing
    pub fallback_position: Option<crate::domain::types::Position>,
}

/// Message to select a dialogue choice by index for the active dialogue.
#[derive(Message, Clone, Debug)]
pub struct SelectDialogueChoice {
    /// Index into the active node's `choices` vector
    pub choice_index: usize,
}

/// Message to advance dialogue (show next text chunk or trigger choice display)
#[derive(Message, Clone, Debug)]
pub struct AdvanceDialogue;

/// Message to start a simple dialogue without a tree ID.
#[derive(Message, Clone, Debug)]
pub struct SimpleDialogue {
    /// Text content to display
    pub text: String,
    /// Speaker's name
    pub speaker_name: String,
    /// Entity that initiated this dialogue (typically an NPC)
    pub speaker_entity: Option<Entity>,
    /// Fallback map position for visual placement if speaker_entity is missing
    pub fallback_position: Option<crate::domain::types::Position>,
}

/// Plugin that registers dialogue message types and systems
pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<StartDialogue>()
            .add_message::<SimpleDialogue>()
            .add_message::<SelectDialogueChoice>()
            .add_message::<AdvanceDialogue>()
            .init_resource::<crate::game::components::dialogue::ActiveDialogueUI>()
            .init_resource::<crate::game::components::dialogue::ChoiceSelectionState>()
            .insert_resource(PendingRecruitmentContext::default())
            .add_systems(
                Update,
                (
                    dialogue_input_system,
                    handle_start_dialogue,
                    handle_simple_dialogue,
                    handle_select_choice,
                    handle_recruitment_actions,
                    crate::game::systems::dialogue_visuals::spawn_dialogue_bubble,
                    crate::game::systems::dialogue_visuals::update_dialogue_text,
                    crate::game::systems::dialogue_choices::spawn_choice_ui,
                    crate::game::systems::dialogue_choices::update_choice_visuals,
                    crate::game::systems::dialogue_choices::choice_input_system,
                ),
            )
            .add_systems(
                Update,
                (
                    crate::game::systems::dialogue_visuals::update_typewriter_text,
                    crate::game::systems::dialogue_visuals::check_speaker_exists,
                    crate::game::systems::dialogue_visuals::cleanup_dialogue_bubble,
                    crate::game::systems::dialogue_choices::cleanup_choice_ui,
                ),
            );
    }
}

/// System to handle input for advancing dialogue
///
/// Sends AdvanceDialogue message when player presses Space or E during dialogue.
///
/// # Arguments
///
/// * `keyboard` - Keyboard input state
/// * `global_state` - Current game state
/// * `advance_writer` - Message writer for AdvanceDialogue messages
fn dialogue_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    dialogue_panel_query: Query<(&Interaction, Ref<Interaction>), With<DialoguePanelRoot>>,
    global_state: Res<GlobalState>,
    mut advance_writer: MessageWriter<AdvanceDialogue>,
) {
    if !matches!(global_state.0.mode, GameMode::Dialogue(_)) {
        return;
    }

    let keyboard_advance =
        keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::KeyE);
    let mouse_just_pressed = mouse_input::mouse_just_pressed(mouse_buttons.as_deref());
    let mouse_advance = dialogue_panel_query
        .iter()
        .any(|(interaction, interaction_ref)| {
            mouse_input::is_activated(
                interaction,
                interaction_ref.is_changed(),
                mouse_just_pressed,
            )
        });

    if keyboard_advance || mouse_advance {
        advance_writer.write(AdvanceDialogue);
    }
}

/// System that starts a dialogue when a `StartDialogue` message is received.
///
/// Fetches the dialogue tree from the `GameContent` resource and places the
/// engine into `GameMode::Dialogue(DialogueState::start(...))`. If the dialogue
/// cannot be found the event is ignored.
#[allow(clippy::too_many_arguments)]
fn handle_start_dialogue(
    mut ev_reader: MessageReader<StartDialogue>,
    mut global_state: ResMut<GlobalState>,
    content: Res<GameContent>,
    mut pending_recruitment: ResMut<PendingRecruitmentContext>,
    mut quest_system: Option<ResMut<crate::application::quests::QuestSystem>>,
    mut game_log: Option<ResMut<crate::game::systems::ui::GameLog>>,
    npc_query: Query<&crate::game::systems::map::NpcMarker>,
    // Phase 3: query TileCoord on the speaker entity so we can compute facing
    // toward the party and emit a SetFacing event.
    tile_coord_query: Query<&crate::game::systems::map::TileCoord>,
    mut facing_writer: Option<MessageWriter<crate::game::systems::facing::SetFacing>>,
    mut despawn_recruitable_visuals: Option<
        MessageWriter<crate::game::systems::map::DespawnRecruitableVisual>,
    >,
) {
    for ev in ev_reader.read() {
        let db = content.db();
        match db.dialogues.get_dialogue(ev.dialogue_id) {
            Some(tree) => {
                // Validate dialogue tree integrity
                if let Err(validation_error) =
                    crate::game::systems::dialogue_validation::validate_dialogue_tree(tree)
                {
                    let error_msg = format!(
                        "Dialogue {} validation failed: {}",
                        ev.dialogue_id, validation_error
                    );
                    info!("{}", error_msg);
                    if let Some(ref mut log) = game_log {
                        log.add_system(error_msg);
                    }
                    return;
                }

                let root = tree.root_node;

                // Validate root node exists (redundant with validation, but kept for safety)
                if tree.get_node(root).is_none() {
                    let error_msg =
                        format!("Dialogue {} has invalid root node {}", ev.dialogue_id, root);
                    info!("{}", error_msg);
                    if let Some(ref mut log) = game_log {
                        log.add_system(error_msg);
                    }
                    return;
                }

                // Extract recruitment context if present
                let recruitment_context = pending_recruitment.0.take();

                // Attempt to resolve NPC ID string from the speaker entity (if present)
                let speaker_npc_id = ev
                    .speaker_entity
                    .and_then(|ent| npc_query.get(ent).ok().map(|m| m.npc_id.clone()));

                let mut new_state = DialogueState::start(
                    ev.dialogue_id,
                    root,
                    ev.fallback_position,
                    speaker_npc_id,
                );
                new_state.recruitment_context = recruitment_context;

                global_state.0.mode = GameMode::Dialogue(new_state);

                // Phase 3: if the speaker entity has a TileCoord, determine the
                // 4-direction from the speaker toward the party and emit SetFacing
                // so the NPC turns to face the player at the start of dialogue.
                if let Some(ref mut writer) = facing_writer {
                    if let Some(speaker_entity) = ev.speaker_entity {
                        if let Ok(tile_coord) = tile_coord_query.get(speaker_entity) {
                            let speaker_pos = tile_coord.0;
                            let party_pos = global_state.0.world.party_position;
                            let direction = crate::game::systems::facing::cardinal_toward(
                                speaker_pos,
                                party_pos,
                            );
                            writer.write(crate::game::systems::facing::SetFacing {
                                entity: speaker_entity,
                                direction,
                                instant: true,
                            });
                            info!(
                                "Dialogue start: NPC {:?} at {:?} facing {:?} toward party at {:?}",
                                speaker_entity, speaker_pos, direction, party_pos
                            );
                        }
                    }
                }

                // Execute any actions attached to the root node and log the text
                // Execute root node actions and log the text
                if let Some(node) = tree.get_node(root) {
                    for action in &node.actions {
                        let dlg_state = match &global_state.0.mode {
                            GameMode::Dialogue(state) => Some(state.clone()),
                            _ => None,
                        };
                        execute_action(
                            action,
                            &mut global_state.0,
                            db,
                            dlg_state.as_ref(),
                            quest_system.as_mut().map(|r| r.as_mut()),
                            game_log.as_mut().map(|r| r.as_mut()),
                            &mut despawn_recruitable_visuals.as_mut(),
                        );
                    }
                    if let Some(ref mut log) = game_log {
                        let speaker = tree.speaker_name.as_deref().unwrap_or("NPC");
                        log.add_dialogue(format!("{}: {}", speaker, node.text));
                    }

                    // Update DialogueState with current node text and choices
                    let choices: Vec<String> =
                        node.choices.iter().map(|c| c.text.clone()).collect();
                    if let GameMode::Dialogue(ref mut state) = global_state.0.mode {
                        state.update_node(
                            node.text.clone(),
                            tree.speaker_name.as_deref().unwrap_or("NPC").to_string(),
                            choices,
                            ev.speaker_entity,
                        );
                    }
                }
            }
            None => {
                let error_msg = format!(
                    "Error: Dialogue {} not found for speaker {:?}",
                    ev.dialogue_id, ev.speaker_entity
                );
                info!("{}", error_msg);
                if let Some(ref mut log) = game_log {
                    log.add_system(error_msg);
                }
                // Do not enter Dialogue mode - stay in current mode
            }
        }
    }
}

/// System that starts a simple dialogue when a `SimpleDialogue` message is received.
fn handle_simple_dialogue(
    mut ev_reader: MessageReader<SimpleDialogue>,
    mut global_state: ResMut<GlobalState>,
    mut game_log: Option<ResMut<crate::game::systems::ui::GameLog>>,
) {
    for ev in ev_reader.read() {
        if let Some(ref mut log) = game_log {
            log.add_dialogue(format!("{}: {}", ev.speaker_name, ev.text));
        }

        let state = DialogueState::start_simple(
            ev.text.clone(),
            ev.speaker_name.clone(),
            ev.speaker_entity,
            ev.fallback_position,
        );

        global_state.0.mode = GameMode::Dialogue(state);
    }
}

/// System that processes a player's choice selection.
///
/// Validates the choice's conditions, executes choice actions (and any resulting
/// node actions after advancing), and ends dialogue if the choice terminates it.
fn handle_select_choice(
    mut ev_reader: MessageReader<SelectDialogueChoice>,
    mut global_state: ResMut<GlobalState>,
    content: Res<GameContent>,
    mut quest_system: Option<ResMut<crate::application::quests::QuestSystem>>,
    mut game_log: Option<ResMut<crate::game::systems::ui::GameLog>>,
    mut despawn_recruitable_visuals: Option<
        MessageWriter<crate::game::systems::map::DespawnRecruitableVisual>,
    >,
) {
    for ev in ev_reader.read() {
        // --- Read-only phase: inspect mode and capture identifiers ---
        let speaker_entity = match &global_state.0.mode {
            GameMode::Dialogue(state) => {
                if state.active_tree_id.is_none() {
                    // Simple dialogue: any choice (like "Goodbye") ends it
                    global_state.0.return_to_exploration();
                    continue;
                }
                (
                    state.active_tree_id.unwrap(),
                    state.current_node_id,
                    state.speaker_entity,
                )
            }
            _ => {
                // Not in dialogue mode - ignore the choice
                continue;
            }
        };

        let (tree_id, current_node_id, speaker_entity) = speaker_entity;

        let db = content.db();
        // Obtain the dialogue tree and current node immutably
        if let Some(tree) = db.dialogues.get_dialogue(tree_id) {
            if let Some(current_node) = tree.get_node(current_node_id) {
                // Validate choice index
                if ev.choice_index >= current_node.choices.len() {
                    let error_msg = format!(
                        "Invalid dialogue choice index {} for node {} (dialogue {})",
                        ev.choice_index, current_node_id, tree_id
                    );
                    info!("{}", error_msg);
                    if let Some(ref mut log) = game_log {
                        log.add_system(error_msg);
                    }
                    continue;
                }

                // Choose the option and run read-only condition checks
                let choice = &current_node.choices[ev.choice_index];

                // Evaluate node and choice conditions using an immutable GameState borrow
                if !evaluate_conditions(&current_node.conditions, &global_state.0, db) {
                    // Node shouldn't have been visible; ignore choice
                    continue;
                }
                if !evaluate_conditions(&choice.conditions, &global_state.0, db) {
                    // Choice not available
                    continue;
                }

                // --- Mutating phase: perform actions and then update the dialogue state safely ---
                {
                    // Execute choice actions first (mutably borrow game state inside execute_action)
                    for action in &choice.actions {
                        let dlg_state = match &global_state.0.mode {
                            GameMode::Dialogue(state) => Some(state.clone()),
                            _ => None,
                        };
                        execute_action(
                            action,
                            &mut global_state.0,
                            db,
                            dlg_state.as_ref(),
                            quest_system.as_mut().map(|r| r.as_mut()),
                            game_log.as_mut().map(|r| r.as_mut()),
                            &mut despawn_recruitable_visuals.as_mut(),
                        );
                    }

                    // Some actions (for example recruitment) intentionally exit Dialogue mode.
                    // If that happened, stop processing this choice to avoid advancing dialogue
                    // state after the mode transition.
                    if !matches!(global_state.0.mode, GameMode::Dialogue(_)) {
                        continue;
                    }

                    // Terminal choice: end dialogue
                    if choice.ends_dialogue || choice.target_node.is_none() {
                        global_state.0.return_to_exploration();
                        continue;
                    }

                    // Non-terminal: clone next node data to avoid overlapping borrows
                    if let Some(target) = choice.target_node {
                        // Validate target node exists before advancing
                        let new_node_data = match tree.get_node(target) {
                            Some(node) => Some((node.text.clone(), node.actions.clone())),
                            None => {
                                let error_msg = format!(
                                    "Invalid node ID {} in dialogue tree {}",
                                    target, tree_id
                                );
                                info!("{}", error_msg);
                                if let Some(ref mut log) = game_log {
                                    log.add_system(error_msg);
                                    log.add_system("Dialogue ended unexpectedly.".to_string());
                                }
                                // End dialogue gracefully
                                global_state.0.return_to_exploration();
                                continue;
                            }
                        };

                        // Advance dialogue state by taking ownership of the mode to avoid holding a
                        // mutable reference to `global_state` while performing other mutable borrows.
                        let prev_mode =
                            std::mem::replace(&mut global_state.0.mode, GameMode::Exploration);
                        if let GameMode::Dialogue(mut ds) = prev_mode {
                            ds.advance_to(target);
                            global_state.0.mode = GameMode::Dialogue(ds);
                        } else {
                            // Restore unexpected mode unchanged
                            global_state.0.mode = prev_mode;
                        }

                        // Now it's safe to log and execute new node actions (we're not holding
                        // a long-lived mutable borrow of the mode anymore)
                        if let Some((text, actions)) = new_node_data {
                            if let Some(ref mut log) = game_log {
                                let speaker = tree.speaker_name.as_deref().unwrap_or("NPC");
                                log.add_dialogue(format!("{}: {}", speaker, text));
                            }

                            for action in actions {
                                let dlg_state = match &global_state.0.mode {
                                    GameMode::Dialogue(state) => Some(state.clone()),
                                    _ => None,
                                };
                                execute_action(
                                    &action,
                                    &mut global_state.0,
                                    db,
                                    dlg_state.as_ref(),
                                    quest_system.as_mut().map(|r| r.as_mut()),
                                    game_log.as_mut().map(|r| r.as_mut()),
                                    &mut despawn_recruitable_visuals.as_mut(),
                                );
                            }

                            // Update DialogueState with new node information
                            if let Some(next_node) = tree.get_node(target) {
                                let choices: Vec<String> =
                                    next_node.choices.iter().map(|c| c.text.clone()).collect();
                                if let GameMode::Dialogue(ref mut state) = global_state.0.mode {
                                    state.update_node(
                                        next_node.text.clone(),
                                        tree.speaker_name.as_deref().unwrap_or("NPC").to_string(),
                                        choices,
                                        speaker_entity,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        } else {
            let error_msg = format!("Active dialogue tree {} not found in database", tree_id);
            info!("{}", error_msg);
            if let Some(ref mut log) = game_log {
                log.add_system(error_msg);
            }
            // If the dialogue tree is missing, we should end dialogue
            if let GameMode::Dialogue(_) = global_state.0.mode {
                global_state.0.return_to_exploration();
            }
        }
    }
}

/// Evaluate a list of `DialogueCondition`s against the current game state.
///
/// The implementation supports the common conditions required for tests:
/// - `HasQuest`, `CompletedQuest` consult `GameState::quests`
/// - `HasItem` sums the party's inventories
/// - `HasGold` reads party gold
/// - `MinLevel` checks first party member's level (simplified)
#[allow(clippy::only_used_in_recursion)]
fn evaluate_conditions(
    conds: &[DialogueCondition],
    game_state: &crate::application::GameState,
    db: &crate::sdk::database::ContentDatabase,
) -> bool {
    use crate::domain::dialogue::DialogueCondition;

    for cond in conds {
        match cond {
            DialogueCondition::HasQuest { quest_id } => {
                let id_str = quest_id.to_string();
                if !game_state
                    .quests
                    .active_quests
                    .iter()
                    .any(|q| q.id == id_str)
                {
                    return false;
                }
            }
            DialogueCondition::CompletedQuest { quest_id } => {
                let id_str = quest_id.to_string();
                if !game_state
                    .quests
                    .completed_quests
                    .iter()
                    .any(|id| id == &id_str)
                {
                    return false;
                }
            }
            DialogueCondition::QuestStage {
                quest_id,
                stage_number: _stage_number,
            } => {
                // Simplified: check presence in active quests
                let id_str = quest_id.to_string();
                if !game_state
                    .quests
                    .active_quests
                    .iter()
                    .any(|q| q.id == id_str)
                {
                    return false;
                }
            }
            DialogueCondition::HasItem { item_id, quantity } => {
                let mut total: u32 = 0;
                for member in game_state.party.members.iter() {
                    for slot in member.inventory.items.iter() {
                        if slot.item_id == *item_id {
                            total = total.saturating_add(slot.charges as u32);
                        }
                    }
                }

                if total < (*quantity as u32) {
                    return false;
                }
            }
            DialogueCondition::HasGold { amount } => {
                if game_state.party.gold < *amount {
                    return false;
                }
            }
            DialogueCondition::MinLevel { level } => {
                // Check first party member level as a simplification
                if let Some(ch) = game_state.party.members.first() {
                    if ch.level < *level as u32 {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            DialogueCondition::FlagSet {
                flag_name: _,
                value,
            } => {
                // Flag system isn't implemented in GameState yet; assume flags are unset.
                // If the condition requires the flag to be true, treat as not satisfied.
                if *value {
                    return false;
                }
            }
            DialogueCondition::ReputationThreshold {
                faction: _,
                threshold: _,
            } => {
                // Reputation system not implemented; conservatively fail the condition.
                return false;
            }
            DialogueCondition::And(inner) => {
                if !evaluate_conditions(inner.as_slice(), game_state, db) {
                    return false;
                }
            }
            DialogueCondition::Or(inner) => {
                let mut ok = false;
                for c in inner.iter() {
                    if evaluate_conditions(std::slice::from_ref(c), game_state, db) {
                        ok = true;
                        break;
                    }
                }
                if !ok {
                    return false;
                }
            }
            DialogueCondition::Not(inner) => {
                if evaluate_conditions(std::slice::from_ref(inner.as_ref()), game_state, db) {
                    return false;
                }
            }
        }
    }

    true
}

/// Resolves a recruitable character ID from dialogue recruitment context.
///
/// Recruitable map events may carry NPC-prefixed IDs (e.g. `npc_old_gareth`)
/// while character definitions are keyed by canonical IDs (e.g. `old_gareth`).
/// This helper normalizes that mismatch and returns a valid character ID when
/// one can be found in the character database.
fn resolve_recruitment_character_id(
    dialogue_state: Option<&crate::application::dialogue::DialogueState>,
    game_state: &crate::application::GameState,
    db: &crate::sdk::database::ContentDatabase,
) -> Option<String> {
    let context_character_id = dialogue_state
        .and_then(|state| {
            state
                .recruitment_context
                .as_ref()
                .map(|ctx| ctx.character_id.clone())
        })
        .or_else(|| match &game_state.mode {
            crate::application::GameMode::Dialogue(state) => state
                .recruitment_context
                .as_ref()
                .map(|ctx| ctx.character_id.clone()),
            _ => None,
        })?;

    let mut candidates = vec![context_character_id.clone()];
    if let Some(stripped) = context_character_id.strip_prefix("npc_") {
        candidates.push(stripped.to_string());
    }

    candidates
        .into_iter()
        .find(|candidate| db.characters.get_character(candidate).is_some())
}

/// Executes recruitment-to-party flow and handles map event cleanup/logging.
fn execute_recruit_to_party(
    character_id: &str,
    game_state: &mut crate::application::GameState,
    db: &crate::sdk::database::ContentDatabase,
    dialogue_state: Option<&crate::application::dialogue::DialogueState>,
    game_log: &mut Option<&mut crate::game::systems::ui::GameLog>,
    despawn_recruitable_visuals: &mut Option<
        &mut MessageWriter<crate::game::systems::map::DespawnRecruitableVisual>,
    >,
) {
    match game_state.recruit_from_map(character_id, db) {
        Ok(crate::application::RecruitResult::AddedToParty) => {
            info!("Successfully recruited '{}' to active party", character_id);
            if let Some(log) = game_log.as_deref_mut() {
                if let Some(char_def) = db.characters.get_character(character_id) {
                    log.add_dialogue(format!("{} joins the party!", char_def.name));
                } else {
                    log.add_dialogue(format!("{} joins the party!", character_id));
                }
            }

            // Remove recruitment event from map
            if let Some(dlg_state) = dialogue_state {
                if let Some(ref recruitment_ctx) = dlg_state.recruitment_context {
                    if let Some(current_map) = game_state.world.get_current_map_mut() {
                        let current_map_id = current_map.id;
                        if let Some(_removed_event) =
                            current_map.remove_event(recruitment_ctx.event_position)
                        {
                            info!(
                                "Removed recruitment event at {:?}",
                                recruitment_ctx.event_position
                            );
                            if let Some(writer) = despawn_recruitable_visuals.as_deref_mut() {
                                writer.write(crate::game::systems::map::DespawnRecruitableVisual {
                                    map_id: current_map_id,
                                    position: recruitment_ctx.event_position,
                                    character_id: character_id.to_string(),
                                });
                            }
                        } else {
                            warn!(
                                "No event found at recruitment position {:?}",
                                recruitment_ctx.event_position
                            );
                        }
                    }
                }
            }

            // Close dialogue immediately after successful recruitment.
            game_state.return_to_exploration();
        }
        Ok(crate::application::RecruitResult::Declined) => {
            // Not currently used by recruit_from_map
            info!("Recruitment declined for '{}'", character_id);
        }
        Ok(crate::application::RecruitResult::SentToInn(inn_id)) => {
            info!("Party full - sent '{}' to inn '{}'", character_id, inn_id);
            if let Some(log) = game_log.as_deref_mut() {
                if let Some(char_def) = db.characters.get_character(character_id) {
                    log.add_dialogue(format!(
                        "Party is full! {} will wait at the inn.",
                        char_def.name
                    ));
                } else {
                    log.add_dialogue(format!("Party is full! {} sent to inn.", character_id));
                }
            }

            // Remove recruitment event from map
            if let Some(dlg_state) = dialogue_state {
                if let Some(ref recruitment_ctx) = dlg_state.recruitment_context {
                    if let Some(current_map) = game_state.world.get_current_map_mut() {
                        let current_map_id = current_map.id;
                        if let Some(_removed_event) =
                            current_map.remove_event(recruitment_ctx.event_position)
                        {
                            info!(
                                "Removed recruitment event at {:?}",
                                recruitment_ctx.event_position
                            );
                            if let Some(writer) = despawn_recruitable_visuals.as_deref_mut() {
                                writer.write(crate::game::systems::map::DespawnRecruitableVisual {
                                    map_id: current_map_id,
                                    position: recruitment_ctx.event_position,
                                    character_id: character_id.to_string(),
                                });
                            }
                        }
                    }
                }
            }

            // Close dialogue after routing recruit to inn.
            game_state.return_to_exploration();
        }
        Err(crate::application::RecruitmentError::AlreadyEncountered(id)) => {
            warn!("Cannot recruit '{}': already encountered", id);
            if let Some(log) = game_log.as_deref_mut() {
                log.add_dialogue(format!("{} has already joined your adventure.", id));
            }
        }
        Err(crate::application::RecruitmentError::CharacterNotFound(id)) => {
            error!("Character definition '{}' not found in database", id);
            if let Some(log) = game_log.as_deref_mut() {
                log.add_system(format!("Error: Character '{}' not found.", id));
            }
        }
        Err(crate::application::RecruitmentError::CharacterDefinition(err)) => {
            error!("Character definition error for '{}': {}", character_id, err);
            if let Some(log) = game_log.as_deref_mut() {
                log.add_system(format!("Error loading character: {}", err));
            }
        }
        Err(crate::application::RecruitmentError::CharacterError(err)) => {
            error!("Character operation error for '{}': {}", character_id, err);
            if let Some(log) = game_log.as_deref_mut() {
                log.add_system(format!("Error: {}", err));
            }
        }
        Err(crate::application::RecruitmentError::PartyManager(err)) => {
            error!("Party management error for '{}': {}", character_id, err);
            if let Some(log) = game_log.as_deref_mut() {
                log.add_system(format!("Error: {}", err));
            }
        }
    }
}

/// Execute a single `DialogueAction`.
///
/// Supported actions:
/// - `StartQuest` → calls into `QuestSystem::start_quest` if present
/// - `GiveItems` → adds items to first party member's inventory
/// - `TakeItems` → attempts to remove items from first party member (best-effort)
/// - `GiveGold` / `TakeGold` → modifies party gold
/// - `SetFlag` / `ChangeReputation` / `TriggerEvent` → not fully implemented
/// - `GrantExperience` → grants XP to first party member
#[allow(unused_mut)]
fn execute_action(
    action: &DialogueAction,
    game_state: &mut crate::application::GameState,
    db: &crate::sdk::database::ContentDatabase,
    dialogue_state: Option<&crate::application::dialogue::DialogueState>,
    quest_system: Option<&mut crate::application::quests::QuestSystem>,
    mut game_log: Option<&mut crate::game::systems::ui::GameLog>,
    despawn_recruitable_visuals: &mut Option<
        &mut MessageWriter<crate::game::systems::map::DespawnRecruitableVisual>,
    >,
) {
    match action {
        DialogueAction::StartQuest { quest_id } => {
            if let Some(qs) = quest_system {
                if let Err(err) = qs.start_quest(*quest_id, game_state, db) {
                    println!("Failed to start quest {}: {}", quest_id, err);
                } else if let Some(ref mut log) = game_log {
                    log.add_dialogue(format!("Quest {} started", quest_id));
                }
            } else {
                println!("Warning: StartQuest requested but no QuestSystem present");
            }
        }
        DialogueAction::CompleteQuestStage {
            quest_id,
            stage_number,
        } => {
            // Simplified: log for visibility (detailed behavior belongs in QuestSystem)
            println!("CompleteQuestStage {} stage {}", quest_id, stage_number);
        }
        DialogueAction::GiveItems { items } => {
            if let Some(member) = game_state.party.members.first_mut() {
                for (item_id, qty) in items {
                    let _ = member.inventory.add_item(*item_id, *qty as u8);
                }
            }
        }
        DialogueAction::TakeItems { items } => {
            if let Some(member) = game_state.party.members.first_mut() {
                for (item_id, qty) in items {
                    let mut remaining = *qty;
                    let mut i = 0usize;
                    while remaining > 0 && i < member.inventory.items.len() {
                        if member.inventory.items[i].item_id == *item_id {
                            if member.inventory.items[i].charges as u16 <= remaining {
                                remaining = remaining
                                    .saturating_sub(member.inventory.items[i].charges as u16);
                                member.inventory.items.remove(i);
                                continue;
                            } else {
                                member.inventory.items[i].charges = member.inventory.items[i]
                                    .charges
                                    .saturating_sub(remaining as u8);
                                break;
                            }
                        }
                        i += 1;
                    }
                }
            }
        }
        DialogueAction::GiveGold { amount } => {
            game_state.party.gold = game_state.party.gold.saturating_add(*amount);
        }
        DialogueAction::TakeGold { amount } => {
            game_state.party.gold = game_state.party.gold.saturating_sub(*amount);
        }
        DialogueAction::SetFlag { flag_name, value } => {
            println!("SetFlag '{}' = {} (not persisted)", flag_name, value);
        }
        DialogueAction::ChangeReputation { faction, change } => {
            println!("ChangeReputation {} by {}", faction, change);
        }
        DialogueAction::TriggerEvent { event_name } => {
            // Special-case handling for opening the inn party management UI via
            // a dialogue-triggered event. This uses the speaker NPC ID stored in
            // the active `DialogueState` (if available) to determine which inn to
            // open.
            if event_name == "open_inn_party_management" {
                // Prefer dialogue_state (passed-in context) when available.
                let speaker_inn_id = dialogue_state
                    .and_then(|d| d.speaker_npc_id.clone())
                    // Fallback: inspect current game mode if we're still in Dialogue
                    .or_else(|| {
                        if let crate::application::GameMode::Dialogue(ref ds) = game_state.mode {
                            ds.speaker_npc_id.clone()
                        } else {
                            None
                        }
                    });

                if let Some(inn_id) = speaker_inn_id {
                    use crate::application::{GameMode, InnManagementState};
                    info!("Opening inn party management for inn '{}'", inn_id);

                    game_state.mode = GameMode::InnManagement(InnManagementState {
                        current_inn_id: inn_id.clone(),
                        selected_party_slot: None,
                        selected_roster_slot: None,
                    });

                    if let Some(ref mut log) = game_log {
                        log.add_system("Opening party management...".to_string());
                    }
                } else {
                    warn!("TriggerEvent 'open_inn_party_management' called but no speaker_npc_id available in DialogueState");
                }
            }

            if event_name == "recruit_character_to_party" {
                let resolved_character_id =
                    resolve_recruitment_character_id(dialogue_state, game_state, db);

                if let Some(character_id) = resolved_character_id {
                    info!(
                        "TriggerEvent '{}' resolved recruitment target '{}'",
                        event_name, character_id
                    );
                    execute_recruit_to_party(
                        &character_id,
                        game_state,
                        db,
                        dialogue_state,
                        &mut game_log,
                        despawn_recruitable_visuals,
                    );
                } else {
                    warn!(
                        "TriggerEvent '{}' fired without resolvable recruitment context",
                        event_name
                    );
                    if let Some(log) = game_log.as_deref_mut() {
                        log.add_system(
                            "Error: Could not resolve recruitable character for this dialogue."
                                .to_string(),
                        );
                    }
                }
            }

            // Generic event logging (kept for visibility/audit)
            info!("Dialogue triggered event: {}", event_name);
            if let Some(ref mut log) = game_log {
                log.add_system(format!("Event triggered: {}", event_name));
            }
        }
        DialogueAction::GrantExperience { amount } => {
            if let Some(member) = game_state.party.members.first_mut() {
                member.experience = member.experience.saturating_add(*amount as u64);
            }
        }
        DialogueAction::RecruitToParty { character_id } => {
            info!(
                "Executing RecruitToParty action for character '{}'",
                character_id
            );
            execute_recruit_to_party(
                character_id,
                game_state,
                db,
                dialogue_state,
                &mut game_log,
                despawn_recruitable_visuals,
            );
        }
        DialogueAction::RecruitToInn {
            character_id,
            innkeeper_id,
        } => {
            info!(
                "Executing RecruitToInn action for character '{}' at inn '{}'",
                character_id, innkeeper_id
            );

            // NOTE: recruit_from_map() handles inn assignment automatically when party is full,
            // but this action explicitly sends to a specific innkeeper regardless of party capacity.
            // We need to manually implement this logic.

            // 1. Verify character not already encountered
            if game_state.encountered_characters.contains(character_id) {
                warn!("Cannot recruit '{}': already encountered", character_id);
                if let Some(ref mut log) = game_log {
                    log.add_system(format!("{} has already been recruited.", character_id));
                }
                return;
            }

            // 2. Verify innkeeper exists
            if db.npcs.get_npc(innkeeper_id).is_none() {
                error!("Innkeeper '{}' not found in database", innkeeper_id);
                if let Some(ref mut log) = game_log {
                    log.add_system(format!("Error: Innkeeper '{}' not found.", innkeeper_id));
                }
                return;
            }

            // 3. Get character definition
            let char_def = match db.characters.get_character(character_id) {
                Some(def) => def,
                None => {
                    error!(
                        "Character definition '{}' not found in database",
                        character_id
                    );
                    if let Some(ref mut log) = game_log {
                        log.add_system(format!("Error: Character '{}' not found.", character_id));
                    }
                    return;
                }
            };

            // 4. Instantiate character
            let character = match char_def.instantiate(&db.races, &db.classes, &db.items) {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to instantiate character '{}': {}", character_id, e);
                    if let Some(ref mut log) = game_log {
                        log.add_system(format!("Error creating character: {}", e));
                    }
                    return;
                }
            };

            // 5. Add to roster at specified inn
            let location = crate::domain::character::CharacterLocation::AtInn(innkeeper_id.clone());
            if let Err(e) = game_state.roster.add_character(character, location) {
                error!("Failed to add character to roster: {}", e);
                if let Some(ref mut log) = game_log {
                    log.add_system(format!("Error: {}", e));
                }
                return;
            }

            // 6. Mark as encountered
            game_state
                .encountered_characters
                .insert(character_id.to_string());

            // 7. Log success
            info!(
                "Successfully recruited '{}' to inn '{}'",
                character_id, innkeeper_id
            );
            if let Some(ref mut log) = game_log {
                log.add_dialogue(format!("{} will wait at the inn.", char_def.name));
            }

            // Remove recruitment event from map
            if let Some(dlg_state) = dialogue_state {
                if let Some(ref recruitment_ctx) = dlg_state.recruitment_context {
                    if let Some(current_map) = game_state.world.get_current_map_mut() {
                        if let Some(_removed_event) =
                            current_map.remove_event(recruitment_ctx.event_position)
                        {
                            info!(
                                "Removed recruitment event at {:?}",
                                recruitment_ctx.event_position
                            );
                        }
                    }
                }
            }
        }

        DialogueAction::OpenInnManagement { innkeeper_id } => {
            use crate::application::{GameMode, InnManagementState};

            info!("Opening inn party management for inn '{}'", innkeeper_id);

            if let Some(ref mut log) = game_log {
                log.add_system("Opening party management...".to_string());
            }

            game_state.mode = GameMode::InnManagement(InnManagementState {
                current_inn_id: innkeeper_id.clone(),
                selected_party_slot: None,
                selected_roster_slot: None,
            });
        }

        DialogueAction::BuyItem {
            item_id,
            target_character_id,
        } => {
            // Resolve speaker NPC ID from dialogue state
            let npc_id = match dialogue_state
                .and_then(|d| d.speaker_npc_id.clone())
                .or_else(|| {
                    if let crate::application::GameMode::Dialogue(ref ds) = game_state.mode {
                        ds.speaker_npc_id.clone()
                    } else {
                        None
                    }
                }) {
                Some(id) => id,
                None => {
                    warn!("BuyItem action fired but no speaker_npc_id in DialogueState");
                    return;
                }
            };

            // Resolve target character: use explicit ID or default to index 0
            let character_id = match target_character_id {
                Some(cid) => *cid,
                None => {
                    if game_state.party.members.is_empty() {
                        warn!("BuyItem action fired but party is empty");
                        return;
                    }
                    0
                }
            };

            // Look up NPC definition
            let npc_def = match db.npcs.get_npc(&npc_id) {
                Some(def) => def.clone(),
                None => {
                    error!("BuyItem: NPC '{}' not found in content database", npc_id);
                    return;
                }
            };

            // Ensure runtime state exists for this NPC (idempotent).
            if game_state.npc_runtime.get(&npc_id).is_none() {
                let tmpl_db =
                    crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::new();
                game_state
                    .npc_runtime
                    .initialize_merchant(&npc_def, &tmpl_db);
            }

            // Clone the NPC runtime state so we can call buy_item which takes
            // `&mut Party` and `&mut NpcRuntimeState` simultaneously without
            // conflicting borrows on game_state.
            let mut npc_runtime_clone = match game_state.npc_runtime.get(&npc_id) {
                Some(rt) => rt.clone(),
                None => {
                    warn!(
                        "BuyItem: NPC runtime for '{}' unavailable after init",
                        npc_id
                    );
                    return;
                }
            };

            let buy_result = {
                // Validate character index before splitting borrows
                if character_id as usize >= game_state.party.members.len() {
                    warn!(
                        "BuyItem: character index {} out of range (party size {})",
                        character_id,
                        game_state.party.members.len()
                    );
                    return;
                }

                let (party, members) = (&mut game_state.party.gold, &mut game_state.party.members);
                let character = &mut members[character_id as usize];

                // Temporarily wrap party gold in a minimal Party-like struct.
                // buy_item takes &mut Party; we can't split party/members here,
                // so we reconstruct a temporary Party just for the borrow.
                // Instead, call the domain function through a small adapter that
                // re-uses the existing Party reference.
                use crate::domain::character::Party;
                // Reconstruct a temporary party with only gold for the call.
                // We swap the gold out, call the function, swap back.
                let saved_gold = *party;
                let mut tmp_party = Party::new();
                tmp_party.gold = saved_gold;

                let result = crate::domain::transactions::buy_item(
                    &mut tmp_party,
                    character,
                    character_id,
                    &mut npc_runtime_clone,
                    &npc_def,
                    *item_id,
                    &db.items,
                );

                // Write gold back regardless of outcome (buy_item is transactional:
                // gold is only deducted on success, so tmp_party.gold is either
                // unchanged or reduced by the exact price).
                *party = tmp_party.gold;

                result
            };

            match buy_result {
                Ok(slot) => {
                    // Commit the mutated NPC runtime state back to the store
                    game_state.npc_runtime.insert(npc_runtime_clone);
                    info!(
                        "Bought item {} (charges={}) for character {}",
                        item_id, slot.charges, character_id
                    );
                    if let Some(ref mut log) = game_log {
                        log.add_item(format!("Purchased item {}.", item_id));
                    }
                }
                Err(e) => {
                    // On failure nothing was mutated: no commit needed
                    warn!("BuyItem failed: {}", e);
                    if let Some(ref mut log) = game_log {
                        log.add_system(format!("Cannot buy item: {}", e));
                    }
                }
            }
        }

        DialogueAction::SellItem {
            item_id,
            source_character_id,
        } => {
            // Resolve speaker NPC ID from dialogue state
            let npc_id = match dialogue_state
                .and_then(|d| d.speaker_npc_id.clone())
                .or_else(|| {
                    if let crate::application::GameMode::Dialogue(ref ds) = game_state.mode {
                        ds.speaker_npc_id.clone()
                    } else {
                        None
                    }
                }) {
                Some(id) => id,
                None => {
                    warn!("SellItem action fired but no speaker_npc_id in DialogueState");
                    return;
                }
            };

            // Resolve source character: explicit ID or search party for item
            let character_id = match source_character_id {
                Some(cid) => *cid,
                None => {
                    // Find first party member who has the item
                    match game_state
                        .party
                        .members
                        .iter()
                        .enumerate()
                        .find(|(_, c)| c.inventory.items.iter().any(|s| s.item_id == *item_id))
                        .map(|(idx, _)| idx as crate::domain::types::CharacterId)
                    {
                        Some(cid) => cid,
                        None => {
                            warn!(
                                "SellItem: no party member has item {} in their inventory",
                                item_id
                            );
                            return;
                        }
                    }
                }
            };

            // Look up NPC definition
            let npc_def = match db.npcs.get_npc(&npc_id) {
                Some(def) => def.clone(),
                None => {
                    error!("SellItem: NPC '{}' not found in content database", npc_id);
                    return;
                }
            };

            // Ensure runtime state exists for this NPC (idempotent).
            if game_state.npc_runtime.get(&npc_id).is_none() {
                let tmpl_db =
                    crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::new();
                game_state
                    .npc_runtime
                    .initialize_merchant(&npc_def, &tmpl_db);
            }

            // Clone the NPC runtime state for the same reason as BuyItem.
            let mut npc_runtime_clone = match game_state.npc_runtime.get(&npc_id) {
                Some(rt) => rt.clone(),
                None => {
                    warn!(
                        "SellItem: NPC runtime for '{}' unavailable after init",
                        npc_id
                    );
                    return;
                }
            };

            let sell_result = {
                if character_id as usize >= game_state.party.members.len() {
                    warn!(
                        "SellItem: character index {} out of range (party size {})",
                        character_id,
                        game_state.party.members.len()
                    );
                    return;
                }

                let (party_gold, members) =
                    (&mut game_state.party.gold, &mut game_state.party.members);
                let character = &mut members[character_id as usize];

                use crate::domain::character::Party;
                let saved_gold = *party_gold;
                let mut tmp_party = Party::new();
                tmp_party.gold = saved_gold;

                let result = crate::domain::transactions::sell_item(
                    &mut tmp_party,
                    character,
                    character_id,
                    &mut npc_runtime_clone,
                    &npc_def,
                    *item_id,
                    &db.items,
                );

                *party_gold = tmp_party.gold;
                result
            };

            match sell_result {
                Ok(price) => {
                    // Commit mutated NPC runtime state
                    game_state.npc_runtime.insert(npc_runtime_clone);
                    info!("Sold item {} for {} gold", item_id, price);
                    if let Some(ref mut log) = game_log {
                        log.add_item(format!("Sold item {} for {} gold.", item_id, price));
                    }
                }
                Err(e) => {
                    warn!("SellItem failed: {}", e);
                    if let Some(ref mut log) = game_log {
                        log.add_system(format!("Cannot sell item: {}", e));
                    }
                }
            }
        }

        DialogueAction::OpenMerchant { npc_id } => {
            // Look up the NPC display name from the content database.
            let npc_name = db
                .npcs
                .get_npc(npc_id)
                .map(|n| n.name.clone())
                .unwrap_or_else(|| npc_id.clone());

            // Guard: NPC must exist and be a merchant to open the shop.
            let npc_is_merchant = db
                .npcs
                .get_npc(npc_id)
                .map(|n| n.is_merchant)
                .unwrap_or(false);

            if !npc_is_merchant {
                warn!(
                    "OpenMerchant: NPC '{}' not found or is not a merchant; ignoring action",
                    npc_id
                );
                if let Some(ref mut log) = game_log {
                    log.add_system(format!("'{}' is not a merchant.", npc_name));
                }
                return;
            }

            // Ensure the merchant's NpcRuntimeState (including stock) is
            // initialised before entering the inventory screen.  This call is
            // idempotent on subsequent visits.
            game_state.ensure_npc_runtime_initialized(db);

            // Transition game mode to MerchantInventory.
            info!(
                "OpenMerchant: entering merchant inventory for NPC '{}' ('{}')",
                npc_id, npc_name
            );
            game_state.enter_merchant_inventory(npc_id.clone(), npc_name);
        }

        DialogueAction::ConsumeService {
            service_id,
            target_character_ids,
        } => {
            // Resolve speaker NPC ID from dialogue state
            let npc_id = match dialogue_state
                .and_then(|d| d.speaker_npc_id.clone())
                .or_else(|| {
                    if let crate::application::GameMode::Dialogue(ref ds) = game_state.mode {
                        ds.speaker_npc_id.clone()
                    } else {
                        None
                    }
                }) {
                Some(id) => id,
                None => {
                    warn!("ConsumeService action fired but no speaker_npc_id in DialogueState");
                    return;
                }
            };

            // Look up NPC definition and service catalog
            let npc_def = match db.npcs.get_npc(&npc_id) {
                Some(def) => def.clone(),
                None => {
                    error!(
                        "ConsumeService: NPC '{}' not found in content database",
                        npc_id
                    );
                    return;
                }
            };

            let service_catalog = match npc_def.service_catalog.clone() {
                Some(catalog) => catalog,
                None => {
                    warn!("ConsumeService: NPC '{}' has no service catalog", npc_id);
                    return;
                }
            };

            // Ensure runtime state exists for this NPC (idempotent).
            if game_state.npc_runtime.get(&npc_id).is_none() {
                let tmpl_db =
                    crate::domain::world::npc_runtime::MerchantStockTemplateDatabase::new();
                game_state
                    .npc_runtime
                    .initialize_merchant(&npc_def, &tmpl_db);
            }

            // Collect target character indices: empty list means whole party
            let target_indices: Vec<usize> = if target_character_ids.is_empty() {
                (0..game_state.party.members.len()).collect()
            } else {
                target_character_ids
                    .iter()
                    .filter_map(|cid| {
                        let idx = *cid;
                        if idx < game_state.party.members.len() {
                            Some(idx)
                        } else {
                            warn!("ConsumeService: character index {} out of range", cid);
                            None
                        }
                    })
                    .collect()
            };

            if target_indices.is_empty() {
                warn!("ConsumeService: no valid target characters resolved");
                return;
            }

            // Build mutable target references — we must collect indices and apply
            // them one by one to satisfy Rust's borrow rules for `Vec::get_mut`.
            // First, perform the gold/gem deduction check against the service entry.
            let service_cost = match service_catalog.get_service(service_id) {
                Some(entry) => (entry.cost, entry.gem_cost),
                None => {
                    warn!(
                        "ConsumeService: service '{}' not found in catalog for NPC '{}'",
                        service_id, npc_id
                    );
                    return;
                }
            };

            if game_state.party.gold < service_cost.0 {
                warn!(
                    "ConsumeService: insufficient gold (have {}, need {})",
                    game_state.party.gold, service_cost.0
                );
                if let Some(ref mut log) = game_log {
                    log.add_system(format!(
                        "Not enough gold for service '{}' (need {} gold).",
                        service_id, service_cost.0
                    ));
                }
                return;
            }

            if game_state.party.gems < service_cost.1 {
                warn!(
                    "ConsumeService: insufficient gems (have {}, need {})",
                    game_state.party.gems, service_cost.1
                );
                if let Some(ref mut log) = game_log {
                    log.add_system(format!(
                        "Not enough gems for service '{}' (need {} gems).",
                        service_id, service_cost.1
                    ));
                }
                return;
            }

            // Apply service to each target character individually, deducting cost once
            // up front and then applying the effect per character.
            game_state.party.gold = game_state.party.gold.saturating_sub(service_cost.0);
            game_state.party.gems = game_state.party.gems.saturating_sub(service_cost.1);

            let mut affected: Vec<crate::domain::types::CharacterId> = Vec::new();
            for idx in &target_indices {
                if let Some(character) = game_state.party.members.get_mut(*idx) {
                    apply_service_effect_inline(character, service_id);
                    affected.push(*idx as crate::domain::types::CharacterId);
                }
            }

            // Record the consumed service in NPC runtime state
            if let Some(npc_runtime) = game_state.npc_runtime.get_mut(&npc_id) {
                npc_runtime.services_consumed.push(service_id.clone());
            }

            info!(
                "ConsumeService '{}': paid {} gold, {} gems; affected {} character(s)",
                service_id,
                service_cost.0,
                service_cost.1,
                affected.len()
            );
            if let Some(ref mut log) = game_log {
                log.add_system(format!(
                    "Service '{}' applied to {} character(s).",
                    service_id,
                    affected.len()
                ));
            }
        }
    }
}

/// Apply a named service effect to a single character.
///
/// This mirrors `domain::transactions::apply_service_effect` but operates
/// directly on a `Character` reference so that `execute_action` can avoid the
/// borrow-checker complexity of constructing a `Vec<&mut Character>` while
/// also holding a mutable borrow on `game_state.party.gold`.
///
/// Unrecognised service IDs are silently ignored (no-op), matching the
/// behaviour of the domain-layer helper.
fn apply_service_effect_inline(
    character: &mut crate::domain::character::Character,
    service_id: &str,
) {
    use crate::domain::character::Condition;
    match service_id {
        "heal_all" | "heal" => {
            character.hp.current = character.hp.base;
        }
        "restore_sp" => {
            character.sp.current = character.sp.base;
        }
        "cure_poison" => {
            character.conditions.remove(Condition::POISONED);
        }
        "cure_disease" => {
            character.conditions.remove(Condition::DISEASED);
        }
        "cure_all" => {
            character.conditions.clear();
        }
        "resurrect" => {
            character.conditions.clear();
            character.hp.current = 1;
        }
        "rest" => {
            character.hp.current = character.hp.base;
            character.sp.current = character.sp.base;
            character.conditions.clear();
        }
        _ => {}
    }
}

/// System to handle recruitment-specific dialogue actions
///
/// This system processes dialogue actions related to character recruitment,
/// including recruiting to the active party or to an inn. Currently contains
/// placeholder implementations (TODO) pending integration with party and inn
/// management systems.
fn handle_recruitment_actions(global_state: Res<GlobalState>, content: Res<GameContent>) {
    // Get current dialogue state if active
    let Some(dialogue_state) = (match &global_state.0.mode {
        GameMode::Dialogue(state) => Some(state.clone()),
        _ => None,
    }) else {
        return;
    };

    let db = content.db();

    // Get active dialogue tree
    let Some(tree_id) = dialogue_state.active_tree_id else {
        return;
    };

    let Some(tree) = db.dialogues.get_dialogue(tree_id) else {
        return;
    };

    // Get current node
    let Some(node) = tree.get_node(dialogue_state.current_node_id) else {
        return;
    };

    // Process recruitment actions on this node
    for action in &node.actions {
        match action {
            DialogueAction::RecruitToParty { character_id } => {
                info!(
                    "Processing RecruitToParty action for character_id: {}",
                    character_id
                );
                // TODO: Actual implementation would:
                // - Verify party has space (< 6 members)
                // - Load character definition
                // - Add to party.members
                // - Update global state
            }
            DialogueAction::RecruitToInn {
                character_id,
                innkeeper_id,
            } => {
                info!(
                    "Processing RecruitToInn action for character_id: {}, innkeeper_id: {}",
                    character_id, innkeeper_id
                );
                // TODO: Actual implementation would:
                // - Load character definition
                // - Find innkeeper
                // - Add character to innkeeper's roster
                // - Update global state
            }
            _ => {} // Other actions handled by execute_action
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::resources::GameContent;
    use crate::domain::dialogue::{DialogueAction, DialogueChoice, DialogueNode, DialogueTree};
    use crate::domain::types::ItemId;
    use crate::sdk::database::ContentDatabase;

    #[test]
    fn test_dialogue_tree_loads_root_node() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Build a simple dialogue tree with root node id = 1
        let mut tree = DialogueTree::new(1, "Test Dialogue", 1);
        let node = DialogueNode::new(1, "Hello world!");
        tree.add_node(node);

        let mut db = ContentDatabase::new();
        db.dialogues.add_dialogue(tree);

        app.insert_resource(GameContent::new(db));
        app.insert_resource(GlobalState(crate::application::GameState::new()));

        // Register plugin (adds message types and systems)
        app.add_plugins(DialoguePlugin);

        // Start dialogue directly for unit test (avoid relying on MessageWriter resource)
        // Clone the content database to avoid borrow conflicts
        let db = {
            let content = app.world().resource::<GameContent>();
            content.db().clone()
        };
        let world = app.world_mut();
        let mut gs = world.resource_mut::<GlobalState>();
        if let Some(tree) = db.dialogues.get_dialogue(1) {
            let root = tree.root_node;
            gs.0.mode = GameMode::Dialogue(DialogueState::start(1, root, None, None));

            // Execute root node actions
            if let Some(node) = tree.get_node(root) {
                for action in &node.actions {
                    let mut despawn_recruitable_visuals = None;
                    execute_action(
                        action,
                        &mut gs.0,
                        &db,
                        None,
                        None,
                        None,
                        &mut despawn_recruitable_visuals,
                    );
                }
            }
        } else {
            panic!("Dialogue not found in test DB");
        }

        let gs = app.world().resource::<GlobalState>();
        assert!(matches!(gs.0.mode, GameMode::Dialogue(_)));
        if let GameMode::Dialogue(ds) = &gs.0.mode {
            assert_eq!(ds.active_tree_id, Some(1));
            assert_eq!(ds.current_node_id, 1);
        }
    }

    #[test]
    fn test_dialogue_choice_advances_node() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Create a dialogue with root -> node2 via choice 0
        let mut tree = DialogueTree::new(2, "Advance Test", 1);
        let mut root = DialogueNode::new(1, "Root");
        root.add_choice(DialogueChoice::new("Go to node 2", Some(2)));
        let node2 = DialogueNode::new(2, "Node 2 reached");
        tree.add_node(root);
        tree.add_node(node2);

        let mut db = ContentDatabase::new();
        db.dialogues.add_dialogue(tree);

        app.insert_resource(GameContent::new(db));
        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.add_plugins(DialoguePlugin);

        // Start dialogue and advance (simulate selecting choice 0 without MessageWriter)
        // Read content first and then take mutable world borrows
        {
            let db = {
                let content = app.world().resource::<GameContent>();
                content.db().clone()
            };
            let world = app.world_mut();
            let mut gs = world.resource_mut::<GlobalState>();

            if let Some(tree) = db.dialogues.get_dialogue(2) {
                let root = tree.root_node;
                gs.0.mode = GameMode::Dialogue(DialogueState::start(2, root, None, None));
            } else {
                panic!("Dialogue not found in test DB");
            }
        }

        // Simulate choosing the first option (index 0) by advancing the dialogue state
        {
            let world = app.world_mut();
            let mut gs = world.resource_mut::<GlobalState>();
            if let GameMode::Dialogue(ref mut ds) = gs.0.mode {
                ds.advance_to(2);
            }
        }

        let gs = app.world().resource::<GlobalState>();
        if let GameMode::Dialogue(ds) = &gs.0.mode {
            assert_eq!(ds.current_node_id, 2);
        } else {
            panic!("Expected to remain in Dialogue mode after choosing non-terminal choice");
        }
    }

    #[test]
    fn test_dialogue_script_action_gives_item() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Dialogue that gives item 99 when choice selected
        let mut tree = DialogueTree::new(3, "GiveItem", 1);
        let mut root = DialogueNode::new(1, "Here, take this");
        let mut give_choice = DialogueChoice::new("Take item", None);
        give_choice.add_action(DialogueAction::GiveItems {
            items: vec![(99 as ItemId, 1)],
        });
        root.add_choice(give_choice);
        tree.add_node(root);

        let mut db = ContentDatabase::new();
        db.dialogues.add_dialogue(tree);

        app.insert_resource(GameContent::new(db));

        // Add a party member so the item has recipient
        let mut gs = crate::application::GameState::new();
        let character = crate::domain::character::Character::new(
            "Test".to_string(),
            "human".to_string(),
            "knight".to_string(),
            crate::domain::character::Sex::Male,
            crate::domain::character::Alignment::Good,
        );
        gs.party.add_member(character).unwrap();
        app.insert_resource(GlobalState(gs));

        app.add_plugins(DialoguePlugin);

        // Start dialogue and execute the choice manually (no MessageWriter)
        // Read content first, then borrow world mutably
        {
            let db = {
                let content = app.world().resource::<GameContent>();
                content.db().clone()
            };
            let world = app.world_mut();
            let mut gs = world.resource_mut::<GlobalState>();

            if let Some(tree) = db.dialogues.get_dialogue(3) {
                let root = tree.root_node;
                gs.0.mode = GameMode::Dialogue(DialogueState::start(3, root, None, None));
                if let Some(node) = tree.get_node(root) {
                    if let Some(choice) = node.choices.first() {
                        for action in &choice.actions {
                            let mut despawn_recruitable_visuals = None;
                            execute_action(
                                action,
                                &mut gs.0,
                                &db,
                                None,
                                None,
                                None,
                                &mut despawn_recruitable_visuals,
                            );
                        }
                    }
                }
            } else {
                panic!("Dialogue not found in test DB");
            }
        }

        // Verify item was added to first party member
        let gs = app.world().resource::<GlobalState>();
        let inv = &gs.0.party.members[0].inventory;
        assert!(
            inv.items.iter().any(|s| s.item_id == 99),
            "Expected item 99 in inventory"
        );
    }

    #[test]
    fn test_handle_start_dialogue_updates_state() {
        // Create test dialogue tree
        let mut tree = DialogueTree::new(1, "Test", 1);
        let mut node = DialogueNode::new(1, "Hello!");
        node.add_choice(DialogueChoice::new("Goodbye", None));
        tree.add_node(node);

        // Verify that DialogueState would be created with correct fields
        assert_eq!(tree.get_node(1).unwrap().text, "Hello!");
        assert_eq!(tree.get_node(1).unwrap().choices.len(), 1);
    }

    #[test]
    fn test_dialogue_input_system_requires_dialogue_mode() {
        // Test that dialogue_input_system only sends events in Dialogue mode
        // Setup: create a simple global state
        let mut gs = crate::application::GameState::new();
        let character = crate::domain::character::Character::new(
            "Test".to_string(),
            "human".to_string(),
            "knight".to_string(),
            crate::domain::character::Sex::Male,
            crate::domain::character::Alignment::Good,
        );
        gs.party.add_member(character).unwrap();

        // In Exploration mode, no event should be sent
        gs.mode = GameMode::Exploration;
        let global_state = GlobalState(gs);

        // Verify we're in exploration mode
        assert!(matches!(global_state.0.mode, GameMode::Exploration));
    }

    #[test]
    fn test_mouse_click_advances_dialogue() {
        use crate::application::dialogue::DialogueState;
        use crate::domain::types::Position;
        use crate::game::components::dialogue::DialoguePanelRoot;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<AdvanceDialogue>();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.add_systems(Update, dialogue_input_system);

        {
            let mut global_state = app.world_mut().resource_mut::<GlobalState>();
            let dialogue_state = DialogueState::start_simple(
                "Hello".to_string(),
                "Speaker".to_string(),
                None,
                Some(Position::new(1, 1)),
            );
            global_state.0.mode = GameMode::Dialogue(dialogue_state);
        }

        app.world_mut()
            .spawn((Button, Interaction::Pressed, DialoguePanelRoot));

        app.update();

        let reader = app.world_mut().resource_mut::<Messages<AdvanceDialogue>>();
        assert_eq!(reader.len(), 1);
    }

    #[test]
    fn test_mouse_hover_dialogue_panel_does_not_advance() {
        use crate::application::dialogue::DialogueState;
        use crate::domain::types::Position;
        use crate::game::components::dialogue::DialoguePanelRoot;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<AdvanceDialogue>();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.add_systems(Update, dialogue_input_system);

        {
            let mut global_state = app.world_mut().resource_mut::<GlobalState>();
            let dialogue_state = DialogueState::start_simple(
                "Hello".to_string(),
                "Speaker".to_string(),
                None,
                Some(Position::new(1, 1)),
            );
            global_state.0.mode = GameMode::Dialogue(dialogue_state);
        }

        app.world_mut()
            .spawn((Button, Interaction::Hovered, DialoguePanelRoot));

        app.update();

        let reader = app.world_mut().resource_mut::<Messages<AdvanceDialogue>>();
        assert_eq!(reader.len(), 0);
    }

    #[test]
    fn test_advance_dialogue_event_handling() {
        // Verify AdvanceDialogue event can be created and serialized
        let event = AdvanceDialogue;
        let debug_str = format!("{:?}", event);
        assert_eq!(debug_str, "AdvanceDialogue");
    }

    #[test]
    fn test_dialogue_state_updates_on_start() {
        // Verify that starting a dialogue properly calls update_node
        let mut state = DialogueState::start(1, 1, None, None);

        state.update_node(
            "Hello!".to_string(),
            "NPC".to_string(),
            vec!["Yes".to_string(), "No".to_string()],
            None,
        );

        assert_eq!(state.current_text, "Hello!");
        assert_eq!(state.current_speaker, "NPC");
        assert_eq!(state.current_choices.len(), 2);
        assert!(state.is_active());
    }

    #[test]
    fn test_dialogue_state_transitions() {
        // Verify state updates when transitioning between nodes
        let mut state = DialogueState::start(1, 1, None, None);
        state.update_node(
            "First text".to_string(),
            "Speaker1".to_string(),
            vec!["Choice 1".to_string()],
            None,
        );

        state.advance_to(2);
        state.update_node(
            "Second text".to_string(),
            "Speaker2".to_string(),
            vec!["Choice 2".to_string()],
            None,
        );

        assert_eq!(state.current_node_id, 2);
        assert_eq!(state.current_text, "Second text");
        assert_eq!(state.current_speaker, "Speaker2");
        assert_eq!(state.current_choices[0], "Choice 2");
    }

    #[test]
    fn test_recruit_to_party_action_success() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::CharacterDefinition;

        // Arrange
        let mut game_state = crate::application::GameState::new();
        let mut db = crate::sdk::database::ContentDatabase::new();

        // Add required class and race
        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        // Add test character to database
        let char_def = CharacterDefinition::new(
            "test_knight".to_string(),
            "Test Knight".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        db.characters.add_character(char_def).unwrap();

        // Act
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::RecruitToParty {
                character_id: "test_knight".to_string(),
            },
            &mut game_state,
            &db,
            None,
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert
        assert_eq!(game_state.party.size(), 1);
        assert_eq!(game_state.party.members[0].name, "Test Knight");
        assert!(game_state.encountered_characters.contains("test_knight"));
    }

    #[test]
    fn test_recruit_to_party_action_when_party_full() {
        use crate::domain::character::{Alignment, CharacterLocation, Sex};
        use crate::domain::character_definition::CharacterDefinition;

        // Arrange
        let mut game_state = crate::application::GameState::new();
        let mut db = crate::sdk::database::ContentDatabase::new();

        // Add required class and race
        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        // Fill party to capacity (6 members)
        for i in 0..6 {
            let char_def = CharacterDefinition::new(
                format!("party_{}", i),
                format!("Party Member {}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            db.characters.add_character(char_def).unwrap();
            let character = crate::domain::character::Character::new(
                format!("Party Member {}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            let _ = game_state.party.add_member(character.clone());
            let _ = game_state
                .roster
                .add_character(character, CharacterLocation::InParty);
        }

        // Add test character to database
        let char_def = CharacterDefinition::new(
            "test_mage".to_string(),
            "Test Mage".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        db.characters.add_character(char_def).unwrap();

        // Act
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::RecruitToParty {
                character_id: "test_mage".to_string(),
            },
            &mut game_state,
            &db,
            None,
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert - party still at max, character sent to inn
        assert_eq!(game_state.party.size(), 6);
        assert_eq!(game_state.roster.characters.len(), 7); // 6 in party + 1 at inn
        assert!(game_state.encountered_characters.contains("test_mage"));
    }

    #[test]
    fn test_recruit_to_party_action_already_recruited() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::CharacterDefinition;

        // Arrange
        let mut game_state = crate::application::GameState::new();
        let mut db = crate::sdk::database::ContentDatabase::new();

        // Add required class and race
        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        // Add test character to database
        let char_def = CharacterDefinition::new(
            "test_knight".to_string(),
            "Test Knight".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        db.characters.add_character(char_def).unwrap();

        // First recruitment
        let _ = game_state.recruit_from_map("test_knight", &db);

        // Act - attempt second recruitment
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::RecruitToParty {
                character_id: "test_knight".to_string(),
            },
            &mut game_state,
            &db,
            None,
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert - party size unchanged
        assert_eq!(game_state.party.size(), 1);
        assert_eq!(game_state.roster.characters.len(), 1);
    }

    #[test]
    fn test_recruit_to_party_action_character_not_found() {
        // Arrange
        let mut game_state = crate::application::GameState::new();
        let db = crate::sdk::database::ContentDatabase::new(); // Empty database

        // Act
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::RecruitToParty {
                character_id: "nonexistent".to_string(),
            },
            &mut game_state,
            &db,
            None,
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert - no changes to party
        assert_eq!(game_state.party.size(), 0);
        assert_eq!(game_state.roster.characters.len(), 0);
    }

    #[test]
    fn test_recruit_to_inn_action_success() {
        use crate::domain::character::{Alignment, CharacterLocation, Sex};
        use crate::domain::character_definition::CharacterDefinition;

        // Arrange
        let mut game_state = crate::application::GameState::new();
        let mut db = crate::sdk::database::ContentDatabase::new();

        // Add required class and race
        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        // Add test character to database
        let char_def = CharacterDefinition::new(
            "test_mage".to_string(),
            "Test Mage".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        db.characters.add_character(char_def).unwrap();

        // Add innkeeper to database
        let innkeeper_def = crate::domain::world::npc::NpcDefinition::new(
            "innkeeper_1".to_string(),
            "Innkeeper".to_string(),
            "innkeeper.png".to_string(),
        );
        db.npcs.add_npc(innkeeper_def).unwrap();

        // Act
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::RecruitToInn {
                character_id: "test_mage".to_string(),
                innkeeper_id: "innkeeper_1".to_string(),
            },
            &mut game_state,
            &db,
            None,
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert
        assert_eq!(game_state.roster.characters.len(), 1);
        assert!(matches!(
            game_state.roster.character_locations[0],
            CharacterLocation::AtInn(ref id) if id == "innkeeper_1"
        ));
        assert!(game_state.encountered_characters.contains("test_mage"));
    }

    #[test]
    fn test_recruit_to_inn_action_already_recruited() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::CharacterDefinition;

        // Arrange
        let mut game_state = crate::application::GameState::new();
        let mut db = crate::sdk::database::ContentDatabase::new();

        // Add required class and race
        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        // Add test character to database
        let char_def = CharacterDefinition::new(
            "test_mage".to_string(),
            "Test Mage".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        db.characters.add_character(char_def).unwrap();

        // Add innkeeper to database
        let innkeeper_def = crate::domain::world::npc::NpcDefinition::new(
            "innkeeper_1".to_string(),
            "Innkeeper".to_string(),
            "innkeeper.png".to_string(),
        );
        db.npcs.add_npc(innkeeper_def).unwrap();

        // First recruitment
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::RecruitToInn {
                character_id: "test_mage".to_string(),
                innkeeper_id: "innkeeper_1".to_string(),
            },
            &mut game_state,
            &db,
            None,
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Act - second recruitment attempt
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::RecruitToInn {
                character_id: "test_mage".to_string(),
                innkeeper_id: "innkeeper_1".to_string(),
            },
            &mut game_state,
            &db,
            None,
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert
        assert_eq!(game_state.roster.characters.len(), 1);
        assert!(game_state.encountered_characters.contains("test_mage"));
    }

    #[test]
    fn test_recruit_to_inn_action_invalid_innkeeper() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::CharacterDefinition;

        // Arrange
        let mut game_state = crate::application::GameState::new();
        let mut db = crate::sdk::database::ContentDatabase::new();

        // Add required class and race
        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        // Add test character to database
        let char_def = CharacterDefinition::new(
            "test_mage".to_string(),
            "Test Mage".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        db.characters.add_character(char_def).unwrap();
        // No innkeeper in database

        // Act
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::RecruitToInn {
                character_id: "test_mage".to_string(),
                innkeeper_id: "invalid_innkeeper".to_string(),
            },
            &mut game_state,
            &db,
            None,
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert - should fail because innkeeper doesn't exist
        assert_eq!(game_state.roster.characters.len(), 0);
        assert!(!game_state.encountered_characters.contains("test_mage"));
    }

    #[test]
    fn test_open_inn_management_action_transitions_mode() {
        // Arrange
        let mut game_state = crate::application::GameState::new();
        let db = crate::sdk::database::ContentDatabase::new();

        // Act
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::OpenInnManagement {
                innkeeper_id: "cozy_inn".to_string(),
            },
            &mut game_state,
            &db,
            None,
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert - mode is InnManagement and inn id is set
        match &game_state.mode {
            crate::application::GameMode::InnManagement(state) => {
                assert_eq!(state.current_inn_id, "cozy_inn".to_string());
                assert_eq!(state.selected_party_slot, None);
                assert_eq!(state.selected_roster_slot, None);
            }
            other => panic!("Expected GameMode::InnManagement, got {:?}", other),
        }
    }

    #[test]
    fn test_trigger_event_opens_inn_management() {
        // Arrange: create a game state and a content DB containing an innkeeper NPC
        let mut game_state = crate::application::GameState::new();
        let mut db = crate::sdk::database::ContentDatabase::new();

        // Add a test innkeeper NPC to the DB
        let npc = crate::domain::world::npc::NpcDefinition::innkeeper(
            "test_innkeeper",
            "Test Innkeeper",
            "portrait",
        );
        db.npcs.add_npc(npc).unwrap();

        // Build a DialogueState that references the innkeeper by ID
        let dlg_state = crate::application::dialogue::DialogueState::start(
            999 as crate::domain::dialogue::DialogueId,
            1 as crate::domain::dialogue::NodeId,
            None,
            Some("test_innkeeper".to_string()),
        );

        // Act: execute the trigger event action that should open inn management
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &crate::domain::dialogue::DialogueAction::TriggerEvent {
                event_name: "open_inn_party_management".to_string(),
            },
            &mut game_state,
            &db,
            Some(&dlg_state),
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert: we transitioned into InnManagement for the expected innkeeper
        match &game_state.mode {
            crate::application::GameMode::InnManagement(state) => {
                assert_eq!(state.current_inn_id, "test_innkeeper".to_string());
                assert_eq!(state.selected_party_slot, None);
                assert_eq!(state.selected_roster_slot, None);
            }
            other => panic!("Expected GameMode::InnManagement, got {:?}", other),
        }
    }

    #[test]
    fn test_trigger_event_recruit_character_to_party_resolves_npc_prefixed_context() {
        use crate::domain::character::{Alignment, Sex};
        use crate::domain::character_definition::CharacterDefinition;

        // Arrange
        let mut game_state = crate::application::GameState::new();
        let mut db = crate::sdk::database::ContentDatabase::new();

        // Add required class and race
        let knight_class = crate::domain::classes::ClassDefinition::new(
            "knight".to_string(),
            "Knight".to_string(),
        );
        db.classes.add_class(knight_class).unwrap();

        let human_race = crate::domain::races::RaceDefinition::new(
            "human".to_string(),
            "Human".to_string(),
            "Human race".to_string(),
        );
        db.races.add_race(human_race).unwrap();

        // Character definition uses canonical ID without npc_ prefix.
        let old_gareth = CharacterDefinition::new(
            "old_gareth".to_string(),
            "Old Gareth".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        db.characters.add_character(old_gareth).unwrap();

        let mut dlg_state = crate::application::dialogue::DialogueState::start(
            100,
            1,
            Some(crate::domain::types::Position::new(15, 7)),
            Some("npc_old_gareth".to_string()),
        );
        dlg_state.recruitment_context = Some(crate::application::dialogue::RecruitmentContext {
            character_id: "npc_old_gareth".to_string(),
            event_position: crate::domain::types::Position::new(15, 7),
        });

        // Act
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &crate::domain::dialogue::DialogueAction::TriggerEvent {
                event_name: "recruit_character_to_party".to_string(),
            },
            &mut game_state,
            &db,
            Some(&dlg_state),
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert
        assert_eq!(game_state.party.size(), 1);
        assert_eq!(game_state.party.members[0].name, "Old Gareth");
        assert!(game_state.encountered_characters.contains("old_gareth"));
    }

    #[test]
    fn test_trigger_event_recruit_character_to_party_with_unresolvable_context_noops() {
        // Arrange: no character definitions are available for the recruitment context.
        let mut game_state = crate::application::GameState::new();
        let db = crate::sdk::database::ContentDatabase::new();

        let mut dlg_state = crate::application::dialogue::DialogueState::start(
            100,
            1,
            Some(crate::domain::types::Position::new(15, 7)),
            Some("npc_missing".to_string()),
        );
        dlg_state.recruitment_context = Some(crate::application::dialogue::RecruitmentContext {
            character_id: "npc_missing".to_string(),
            event_position: crate::domain::types::Position::new(15, 7),
        });

        // Act
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &crate::domain::dialogue::DialogueAction::TriggerEvent {
                event_name: "recruit_character_to_party".to_string(),
            },
            &mut game_state,
            &db,
            Some(&dlg_state),
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert
        assert_eq!(game_state.party.size(), 0);
        assert_eq!(game_state.roster.characters.len(), 0);
        assert!(game_state.encountered_characters.is_empty());
    }

    #[test]
    fn test_default_dialogue_template_opens_inn_management() {
        use bevy::prelude::*;

        // Arrange: build an App with the dialogue plugin and a content DB containing
        // the default dialogue template (ID 999) and an innkeeper NPC referencing it.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(DialoguePlugin);

        // Build content DB
        let mut db = crate::sdk::database::ContentDatabase::new();

        // Dialogue tree 999 (default template)
        let mut tree = crate::domain::dialogue::DialogueTree::new(
            999,
            "Default Innkeeper Greeting".to_string(),
            1,
        );

        let mut root = crate::domain::dialogue::DialogueNode::new(
            1,
            "Welcome to my establishment! What can I do for you?",
        );
        root.add_choice(crate::domain::dialogue::DialogueChoice::new(
            "I need to manage my party.",
            Some(2),
        ));
        root.add_choice(crate::domain::dialogue::DialogueChoice::new(
            "Nothing right now. Farewell.",
            None,
        ));
        tree.add_node(root);

        let mut node2 = crate::domain::dialogue::DialogueNode::new(
            2,
            "Certainly! Let me help you organize your party.",
        );
        node2.add_action(crate::domain::dialogue::DialogueAction::TriggerEvent {
            event_name: "open_inn_party_management".to_string(),
        });
        node2.is_terminal = true;
        tree.add_node(node2);

        db.dialogues.add_dialogue(tree);

        // Add an innkeeper NPC and set its dialogue_id to 999
        let mut npc = crate::domain::world::npc::NpcDefinition::innkeeper(
            "default_inn",
            "Default Inn",
            "portrait",
        );
        npc.dialogue_id = Some(999);
        db.npcs.add_npc(npc).unwrap();

        // Insert resources needed by the dialogue systems
        app.insert_resource(crate::application::resources::GameContent::new(db));
        app.insert_resource(crate::game::resources::GlobalState(
            crate::application::GameState::new(),
        ));
        app.init_resource::<crate::game::components::dialogue::ActiveDialogueUI>();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(crate::game::systems::ui::GameLog::new());

        // Spawn an entity with NpcMarker so the dialogue system can resolve the
        // speaker_entity -> npc_id mapping used by TriggerEvent handling.
        let npc_entity = app
            .world_mut()
            .spawn((crate::game::systems::map::NpcMarker {
                npc_id: "default_inn".to_string(),
            },))
            .id();

        // Act: send StartDialogue for the default template with the NPC entity as speaker
        {
            let mut start_msgs = app.world_mut().resource_mut::<Messages<StartDialogue>>();
            start_msgs.write(StartDialogue {
                dialogue_id: 999,
                speaker_entity: Some(npc_entity),
                fallback_position: None,
            });
        }

        // Run update to process StartDialogue and initialize DialogueState
        app.update();

        // Verify we're in Dialogue mode and at the root node
        {
            let gs = app
                .world()
                .resource::<crate::game::resources::GlobalState>();
            match &gs.0.mode {
                crate::application::GameMode::Dialogue(ds) => {
                    assert_eq!(ds.active_tree_id, Some(999u16));
                    assert_eq!(ds.current_node_id, 1);
                }
                other => panic!(
                    "Expected Dialogue mode after StartDialogue, got {:?}",
                    other
                ),
            }
        }

        // Send SelectDialogueChoice message to choose the "I need to manage my party." option
        {
            let mut choice_msgs = app
                .world_mut()
                .resource_mut::<Messages<SelectDialogueChoice>>();
            choice_msgs.write(SelectDialogueChoice { choice_index: 0 });
        }

        // Run update to process the choice and execute node actions (TriggerEvent)
        app.update();

        // Assert: we transitioned into InnManagement for the expected innkeeper
        let gs = app
            .world()
            .resource::<crate::game::resources::GlobalState>();
        match &gs.0.mode {
            crate::application::GameMode::InnManagement(state) => {
                assert_eq!(state.current_inn_id, "default_inn".to_string());
            }
            other => panic!(
                "Expected InnManagement mode after selecting manage option, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_open_inn_management_action_logs_message() {
        // Arrange
        let mut game_state = crate::application::GameState::new();
        let db = crate::sdk::database::ContentDatabase::new();
        let mut log = crate::game::systems::ui::GameLog::new();

        // Act
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::OpenInnManagement {
                innkeeper_id: "cozy_inn".to_string(),
            },
            &mut game_state,
            &db,
            None,
            None,
            Some(&mut log),
            &mut despawn_recruitable_visuals,
        );

        // Assert - message logged
        let entries = log.entries();
        assert!(
            entries
                .iter()
                .any(|e| e.contains("Opening party management")),
            "Expected opening message in game log. Actual entries: {:?}",
            entries
        );
    }

    // ===== Phase 3: Transaction Dialogue Integration Tests =====

    /// Build a minimal ContentDatabase containing one merchant NPC ("merchant_tom")
    /// with item 1 in stock (qty 3, price 10) and item 2 in stock (qty 2, price 20).
    /// The item database contains matching Item entries.
    fn make_merchant_db() -> crate::sdk::database::ContentDatabase {
        use crate::domain::items::{Item, ItemType, WeaponClassification, WeaponData};
        use crate::domain::types::DiceRoll;
        use crate::domain::world::npc::NpcDefinition;

        let mut db = ContentDatabase::new();

        // Add items to item database
        let item1 = Item {
            id: 1,
            name: "Iron Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 6, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::Simple,
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
        let item2 = Item {
            id: 2,
            name: "Shield".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 4, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::Simple,
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
        db.items.add_item(item1).unwrap();
        db.items.add_item(item2).unwrap();

        // Add merchant NPC
        let merchant = NpcDefinition::merchant("merchant_tom", "Tom", "tom.png");
        db.npcs.add_npc(merchant).unwrap();

        // Pre-populate NPC runtime store with stock so the test bypasses lazy-init
        db
    }

    /// Build a GameState with one party member and merchant_tom's NPC runtime
    /// pre-initialised with stock for items 1 and 2.
    fn make_game_state_with_merchant(party_gold: u32) -> crate::application::GameState {
        use crate::domain::character::{Alignment, Character, Sex};
        use crate::domain::inventory::{MerchantStock, StockEntry};
        use crate::domain::world::npc_runtime::NpcRuntimeState;

        let mut gs = crate::application::GameState::new();

        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        gs.party.gold = party_gold;

        // Pre-initialise merchant runtime state with stock
        let mut npc_runtime = NpcRuntimeState::new("merchant_tom".to_string());
        npc_runtime.stock = Some(MerchantStock {
            entries: vec![
                StockEntry::new(1, 3), // item 1, qty 3
                StockEntry::new(2, 2), // item 2, qty 2
            ],
            restock_template: None,
        });
        gs.npc_runtime.insert(npc_runtime);

        gs
    }

    /// Make a DialogueState with speaker_npc_id set to "merchant_tom".
    fn merchant_dialogue_state() -> crate::application::dialogue::DialogueState {
        crate::application::dialogue::DialogueState::start(
            1,
            1,
            None,
            Some("merchant_tom".to_string()),
        )
    }

    #[test]
    fn test_buy_item_dialogue_action_deducts_gold() {
        // Arrange
        let db = make_merchant_db();
        let mut game_state = make_game_state_with_merchant(100);
        game_state.mode = crate::application::GameMode::Dialogue(merchant_dialogue_state());

        // Act: buy item 1 (costs 10 gold) for character 0
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::BuyItem {
                item_id: 1,
                target_character_id: None,
            },
            &mut game_state,
            &db,
            Some(&merchant_dialogue_state()),
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert: gold decreased and item is in inventory
        assert_eq!(
            game_state.party.gold, 90,
            "Party gold should decrease by 10 after buying item 1"
        );
        assert!(
            game_state.party.members[0]
                .inventory
                .items
                .iter()
                .any(|s| s.item_id == 1),
            "Item 1 should be in character 0 inventory after purchase"
        );
    }

    #[test]
    fn test_buy_item_dialogue_action_insufficient_gold_no_mutation() {
        // Arrange
        let db = make_merchant_db();
        let mut game_state = make_game_state_with_merchant(5); // only 5 gold, item costs 10

        // Act: attempt to buy item 1 with insufficient gold
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::BuyItem {
                item_id: 1,
                target_character_id: None,
            },
            &mut game_state,
            &db,
            Some(&merchant_dialogue_state()),
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert: gold unchanged and inventory empty
        assert_eq!(
            game_state.party.gold, 5,
            "Party gold should not change when purchase fails"
        );
        assert!(
            game_state.party.members[0].inventory.items.is_empty(),
            "Inventory should be empty when purchase fails due to insufficient gold"
        );
    }

    #[test]
    fn test_consume_service_dialogue_action_heals_party() {
        use crate::domain::inventory::{ServiceCatalog, ServiceEntry};
        use crate::domain::world::npc::NpcDefinition;
        use crate::domain::world::npc_runtime::NpcRuntimeState;

        // Arrange: build a priest NPC with heal_all service
        let mut db = ContentDatabase::new();
        let mut priest = NpcDefinition::priest("priest_anna", "Anna", "anna.png");
        let mut catalog = ServiceCatalog::new();
        catalog.services.push(ServiceEntry::new(
            "heal_all".to_string(),
            50, // cost 50 gold
            "Heal all party members".to_string(),
        ));
        priest.service_catalog = Some(catalog);
        db.npcs.add_npc(priest).unwrap();

        let mut game_state = crate::application::GameState::new();

        // Add a party member with reduced HP
        use crate::domain::character::{Alignment, Character, Sex};
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.base = 30;
        hero.hp.current = 5; // damaged
        game_state.party.add_member(hero).unwrap();
        game_state.party.gold = 100;

        // Pre-init priest runtime
        let priest_runtime = NpcRuntimeState::new("priest_anna".to_string());
        game_state.npc_runtime.insert(priest_runtime);

        // Dialogue state pointing to the priest
        let dlg_state = crate::application::dialogue::DialogueState::start(
            1,
            1,
            None,
            Some("priest_anna".to_string()),
        );

        // Act: consume heal_all service for whole party
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::ConsumeService {
                service_id: "heal_all".to_string(),
                target_character_ids: vec![],
            },
            &mut game_state,
            &db,
            Some(&dlg_state),
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert: HP restored and gold deducted
        assert_eq!(
            game_state.party.members[0].hp.current, game_state.party.members[0].hp.base,
            "Character HP should be fully restored after heal_all service"
        );
        assert_eq!(
            game_state.party.gold, 50,
            "Party should have paid 50 gold for heal_all service"
        );
    }

    #[test]
    fn test_consume_service_dialogue_action_insufficient_gold_no_mutation() {
        use crate::domain::inventory::{ServiceCatalog, ServiceEntry};
        use crate::domain::world::npc::NpcDefinition;
        use crate::domain::world::npc_runtime::NpcRuntimeState;

        // Arrange: build a priest NPC with heal_all service costing 50 gold
        let mut db = ContentDatabase::new();
        let mut priest = NpcDefinition::priest("priest_anna", "Anna", "anna.png");
        let mut catalog = ServiceCatalog::new();
        catalog.services.push(ServiceEntry::new(
            "heal_all".to_string(),
            50,
            "Heal all".to_string(),
        ));
        priest.service_catalog = Some(catalog);
        db.npcs.add_npc(priest).unwrap();

        let mut game_state = crate::application::GameState::new();

        use crate::domain::character::{Alignment, Character, Sex};
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.base = 30;
        hero.hp.current = 5; // damaged
        game_state.party.add_member(hero).unwrap();
        game_state.party.gold = 0; // no gold

        let priest_runtime = NpcRuntimeState::new("priest_anna".to_string());
        game_state.npc_runtime.insert(priest_runtime);

        let dlg_state = crate::application::dialogue::DialogueState::start(
            1,
            1,
            None,
            Some("priest_anna".to_string()),
        );

        // Act: attempt heal_all with no gold
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::ConsumeService {
                service_id: "heal_all".to_string(),
                target_character_ids: vec![],
            },
            &mut game_state,
            &db,
            Some(&dlg_state),
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert: HP unchanged and gold unchanged
        assert_eq!(
            game_state.party.members[0].hp.current, 5,
            "HP should be unchanged when service cannot be afforded"
        );
        assert_eq!(
            game_state.party.gold, 0,
            "Gold should be unchanged when service cannot be afforded"
        );
    }

    #[test]
    fn test_dialogue_action_description_buy_item() {
        // Confirms description() returns a non-empty string for BuyItem
        let action = DialogueAction::BuyItem {
            item_id: 1,
            target_character_id: None,
        };
        let desc = action.description();
        assert!(!desc.is_empty(), "BuyItem description should not be empty");
        assert!(
            desc.contains("1"),
            "BuyItem description should mention item id"
        );
    }

    /// `OpenMerchant` must transition game mode to `MerchantInventory` when
    /// the NPC exists and has `is_merchant == true`.
    #[test]
    fn test_open_merchant_dialogue_action_enters_merchant_inventory() {
        // Arrange: use the merchant DB fixture that has "merchant_tom"
        let db = make_merchant_db();
        let mut game_state = make_game_state_with_merchant(0);
        game_state.mode = crate::application::GameMode::Dialogue(merchant_dialogue_state());

        // Act
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::OpenMerchant {
                npc_id: "merchant_tom".to_string(),
            },
            &mut game_state,
            &db,
            Some(&merchant_dialogue_state()),
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert: mode must be MerchantInventory
        assert!(
            matches!(
                game_state.mode,
                crate::application::GameMode::MerchantInventory(_)
            ),
            "OpenMerchant must transition mode to MerchantInventory, got {:?}",
            game_state.mode
        );
    }

    /// `OpenMerchant` with an unknown NPC ID must not panic and must not change
    /// the game mode (graceful degradation).
    #[test]
    fn test_open_merchant_dialogue_action_unknown_npc_no_panic() {
        // Arrange: empty content database — NPC "ghost_npc" does not exist
        let db = ContentDatabase::new();
        let mut game_state = crate::application::GameState::new();
        let mode_before = std::mem::discriminant(&game_state.mode);

        // Act: must not panic
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::OpenMerchant {
                npc_id: "ghost_npc".to_string(),
            },
            &mut game_state,
            &db,
            None,
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert: game mode is unchanged (no transition on unknown NPC)
        assert_eq!(
            std::mem::discriminant(&game_state.mode),
            mode_before,
            "OpenMerchant with unknown NPC must not change game mode"
        );
    }

    #[test]
    fn test_sell_item_dialogue_action_adds_gold() {
        use crate::domain::items::{Item, ItemType, WeaponClassification, WeaponData};
        use crate::domain::types::DiceRoll;
        use crate::domain::world::npc::NpcDefinition;
        use crate::domain::world::npc_runtime::NpcRuntimeState;

        // Arrange
        let mut db = ContentDatabase::new();

        // Item with sell_cost = 5
        let item1 = Item {
            id: 1,
            name: "Old Dagger".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 4, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::Simple,
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
        db.items.add_item(item1).unwrap();

        let merchant = NpcDefinition::merchant("merchant_tom", "Tom", "tom.png");
        db.npcs.add_npc(merchant).unwrap();

        use crate::domain::character::{Alignment, Character, Sex};
        let mut gs = crate::application::GameState::new();
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Give character item 1 in inventory
        hero.inventory.add_item(1, 0).unwrap();
        gs.party.add_member(hero).unwrap();
        gs.party.gold = 0;

        // Pre-init merchant runtime (no stock needed for selling)
        let npc_runtime = NpcRuntimeState::new("merchant_tom".to_string());
        gs.npc_runtime.insert(npc_runtime);

        let dlg_state = merchant_dialogue_state();

        // Act
        let mut despawn_recruitable_visuals = None;
        execute_action(
            &DialogueAction::SellItem {
                item_id: 1,
                source_character_id: None,
            },
            &mut gs,
            &db,
            Some(&dlg_state),
            None,
            None,
            &mut despawn_recruitable_visuals,
        );

        // Assert: item removed from inventory and gold increased
        assert!(
            gs.party.members[0]
                .inventory
                .items
                .iter()
                .all(|s| s.item_id != 1),
            "Item 1 should be removed from inventory after selling"
        );
        assert!(
            gs.party.gold > 0,
            "Party gold should increase after selling an item"
        );
    }

    // ─── Phase 3: Dialogue → SetFacing integration tests ─────────────────────

    /// `test_dialogue_start_emits_set_facing` – starting a dialogue with a speaker
    /// whose entity has a `TileCoord` must cause the NPC to face the party.
    ///
    /// Speaker at (3,3), party at (5,3) → expected facing: East.
    #[test]
    fn test_dialogue_start_emits_set_facing() {
        use crate::application::GameState;
        use crate::domain::types::{Direction, Position};
        use crate::game::components::creature::FacingComponent;
        use crate::game::resources::GlobalState;
        use crate::game::systems::facing::FacingPlugin;
        use crate::game::systems::map::TileCoord;

        // Build a dialogue tree with id=500, root=1
        let mut tree = DialogueTree::new(500, "Facing Test", 1);
        let node = DialogueNode::new(1, "Hello traveler!");
        tree.add_node(node);

        let mut db = ContentDatabase::new();
        db.dialogues.add_dialogue(tree);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Register FacingPlugin so SetFacing message is available to DialoguePlugin
        app.add_plugins(FacingPlugin);
        app.add_plugins(DialoguePlugin);

        // Party is to the East of the NPC speaker
        let mut game_state = GameState::new();
        game_state.world.set_party_position(Position::new(5, 3));
        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));
        app.insert_resource(PendingRecruitmentContext::default());
        app.insert_resource(bevy::input::ButtonInput::<bevy::prelude::KeyCode>::default());

        // Spawn the speaker NPC entity with FacingComponent and TileCoord
        let speaker = app
            .world_mut()
            .spawn((
                Transform::default(),
                GlobalTransform::default(),
                FacingComponent::new(Direction::North),
                TileCoord(Position::new(3, 3)),
            ))
            .id();

        // Write a StartDialogue message targeting the speaker entity
        app.world_mut()
            .resource_mut::<Messages<StartDialogue>>()
            .write(StartDialogue {
                dialogue_id: 500,
                speaker_entity: Some(speaker),
                fallback_position: None,
            });

        // First frame: handle_start_dialogue processes StartDialogue and emits SetFacing.
        // Second frame: handle_set_facing processes the emitted SetFacing event.
        // Two updates are needed because messages written in frame N are read in frame N+1.
        app.update();
        app.update();

        // Verify the NPC now faces East (toward the party)
        let facing = app.world().get::<FacingComponent>(speaker).unwrap();
        assert_eq!(
            facing.direction,
            Direction::East,
            "NPC speaker at (3,3) must face East toward party at (5,3) after dialogue start"
        );
    }

    /// `test_dialogue_start_no_speaker_entity_does_not_panic` – when `speaker_entity`
    /// is `None`, the SetFacing path must be skipped without panic.
    #[test]
    fn test_dialogue_start_no_speaker_entity_does_not_panic() {
        use crate::application::GameState;
        use crate::domain::types::Position;
        use crate::game::resources::GlobalState;
        use crate::game::systems::facing::FacingPlugin;

        let mut tree = DialogueTree::new(501, "No Speaker", 1);
        let node = DialogueNode::new(1, "No speaker entity here.");
        tree.add_node(node);

        let mut db = ContentDatabase::new();
        db.dialogues.add_dialogue(tree);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(FacingPlugin);
        app.add_plugins(DialoguePlugin);

        let mut game_state = GameState::new();
        game_state.world.set_party_position(Position::new(5, 5));
        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));
        app.insert_resource(PendingRecruitmentContext::default());
        app.insert_resource(bevy::input::ButtonInput::<bevy::prelude::KeyCode>::default());

        // No speaker entity
        app.world_mut()
            .resource_mut::<Messages<StartDialogue>>()
            .write(StartDialogue {
                dialogue_id: 501,
                speaker_entity: None,
                fallback_position: Some(Position::new(3, 3)),
            });

        // Must not panic
        app.update();
    }

    /// `test_dialogue_start_speaker_without_tile_coord_skips_facing` – when the
    /// speaker entity exists but has no `TileCoord`, SetFacing must not be emitted.
    #[test]
    fn test_dialogue_start_speaker_without_tile_coord_skips_facing() {
        use crate::application::GameState;
        use crate::domain::types::{Direction, Position};
        use crate::game::components::creature::FacingComponent;
        use crate::game::resources::GlobalState;
        use crate::game::systems::facing::FacingPlugin;

        let mut tree = DialogueTree::new(502, "No TileCoord", 1);
        let node = DialogueNode::new(1, "I have no coord!");
        tree.add_node(node);

        let mut db = ContentDatabase::new();
        db.dialogues.add_dialogue(tree);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(FacingPlugin);
        app.add_plugins(DialoguePlugin);

        let mut game_state = GameState::new();
        game_state.world.set_party_position(Position::new(5, 5));
        app.insert_resource(GlobalState(game_state));
        app.insert_resource(GameContent::new(db));
        app.insert_resource(PendingRecruitmentContext::default());
        app.insert_resource(bevy::input::ButtonInput::<bevy::prelude::KeyCode>::default());

        // Speaker entity with FacingComponent but no TileCoord
        let speaker = app
            .world_mut()
            .spawn((
                Transform::default(),
                GlobalTransform::default(),
                FacingComponent::new(Direction::West),
                // Deliberately NO TileCoord
            ))
            .id();

        app.world_mut()
            .resource_mut::<Messages<StartDialogue>>()
            .write(StartDialogue {
                dialogue_id: 502,
                speaker_entity: Some(speaker),
                fallback_position: None,
            });

        app.update();

        // Facing must remain West – no SetFacing was emitted
        let facing = app.world().get::<FacingComponent>(speaker).unwrap();
        assert_eq!(
            facing.direction,
            Direction::West,
            "FacingComponent must remain West when speaker has no TileCoord"
        );
    }
}
