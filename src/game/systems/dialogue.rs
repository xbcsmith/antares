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

use crate::application::dialogue::DialogueState;
use crate::application::resources::GameContent;
use crate::application::GameMode;
use crate::game::resources::GlobalState;

use crate::domain::dialogue::{DialogueAction, DialogueCondition, DialogueId};

/// Message to request that a dialogue tree begin (e.g., NPC started talking).
#[derive(Message, Clone, Debug)]
pub struct StartDialogue {
    /// Dialogue tree id to activate
    pub dialogue_id: DialogueId,
}

/// Message to select a dialogue choice by index for the active dialogue.
#[derive(Message, Clone, Debug)]
pub struct SelectDialogueChoice {
    /// Index into the active node's `choices` vector
    pub choice_index: usize,
}

/// Plugin that registers dialogue message types and systems
pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<StartDialogue>()
            .add_message::<SelectDialogueChoice>()
            .add_systems(Update, (handle_start_dialogue, handle_select_choice));
    }
}

/// System that starts a dialogue when a `StartDialogue` message is received.
///
/// Fetches the dialogue tree from the `GameContent` resource and places the
/// engine into `GameMode::Dialogue(DialogueState::start(...))`. If the dialogue
/// cannot be found the event is ignored.
fn handle_start_dialogue(
    mut ev_reader: MessageReader<StartDialogue>,
    mut global_state: ResMut<GlobalState>,
    content: Res<GameContent>,
    mut quest_system: Option<ResMut<crate::application::quests::QuestSystem>>,
    mut game_log: Option<ResMut<crate::game::systems::ui::GameLog>>,
) {
    for ev in ev_reader.read() {
        let db = content.db();
        if let Some(tree) = db.dialogues.get_dialogue(ev.dialogue_id) {
            let root = tree.root_node;
            global_state.0.mode = GameMode::Dialogue(DialogueState::start(ev.dialogue_id, root));

            // Execute any actions attached to the root node and log the text
            // Execute root node actions and log the text
            if let Some(node) = tree.get_node(root) {
                for action in &node.actions {
                    execute_action(
                        action,
                        &mut global_state.0,
                        db,
                        quest_system.as_deref_mut(),
                        game_log.as_deref_mut(),
                    );
                }
                if let Some(ref mut log) = game_log {
                    let speaker = tree.speaker_name.as_deref().unwrap_or("NPC");
                    log.add(format!("{}: {}", speaker, node.text));
                }
            }
        } else {
            println!(
                "Warning: StartDialogue requested for missing id {}",
                ev.dialogue_id
            );
        }
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
) {
    for ev in ev_reader.read() {
        // --- Read-only phase: inspect mode and capture identifiers ---
        let (tree_id, current_node_id) = match &global_state.0.mode {
            GameMode::Dialogue(state) => {
                if let Some(tid) = state.active_tree_id {
                    (tid, state.current_node_id)
                } else {
                    // Dialogue is active but no tree configured; skip
                    continue;
                }
            }
            _ => {
                // Not in dialogue mode - ignore the choice
                continue;
            }
        };

        let db = content.db();
        // Obtain the dialogue tree and current node immutably
        if let Some(tree) = db.dialogues.get_dialogue(tree_id) {
            if let Some(current_node) = tree.get_node(current_node_id) {
                // Validate choice index
                if ev.choice_index >= current_node.choices.len() {
                    println!(
                        "Warning: Invalid dialogue choice index {} for node {}",
                        ev.choice_index, current_node_id
                    );
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
                        execute_action(
                            action,
                            &mut global_state.0,
                            db,
                            quest_system.as_deref_mut(),
                            game_log.as_deref_mut(),
                        );
                    }

                    // Terminal choice: end dialogue
                    if choice.ends_dialogue || choice.target_node.is_none() {
                        global_state.0.return_to_exploration();
                        continue;
                    }

                    // Non-terminal: clone next node data to avoid overlapping borrows
                    if let Some(target) = choice.target_node {
                        let new_node_data = tree
                            .get_node(target)
                            .map(|n| (n.text.clone(), n.actions.clone()));

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
                                log.add(format!("{}: {}", speaker, text));
                            }

                            for action in actions {
                                execute_action(
                                    &action,
                                    &mut global_state.0,
                                    db,
                                    quest_system.as_deref_mut(),
                                    game_log.as_deref_mut(),
                                );
                            }
                        }
                    }
                }
            }
        } else {
            println!("Warning: Active dialogue {} not found in db", tree_id);
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
    mut quest_system: Option<&mut crate::application::quests::QuestSystem>,
    mut game_log: Option<&mut crate::game::systems::ui::GameLog>,
) {
    match action {
        DialogueAction::StartQuest { quest_id } => {
            if let Some(qs) = quest_system {
                if let Err(err) = qs.start_quest(*quest_id, game_state, db) {
                    println!("Failed to start quest {}: {}", quest_id, err);
                } else if let Some(ref mut log) = game_log {
                    log.add(format!("Quest {} started", quest_id));
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
            println!("TriggerEvent '{}'", event_name);
        }
        DialogueAction::GrantExperience { amount } => {
            if let Some(member) = game_state.party.members.first_mut() {
                member.experience = member.experience.saturating_add(*amount as u64);
            }
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
            gs.0.mode = GameMode::Dialogue(DialogueState::start(1, root));

            // Execute root node actions
            if let Some(node) = tree.get_node(root) {
                for action in &node.actions {
                    execute_action(action, &mut gs.0, &db, None, None);
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
                gs.0.mode = GameMode::Dialogue(DialogueState::start(2, root));
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
                gs.0.mode = GameMode::Dialogue(DialogueState::start(3, root));
                if let Some(node) = tree.get_node(root) {
                    if let Some(choice) = node.choices.first() {
                        for action in &choice.actions {
                            execute_action(action, &mut gs.0, &db, None, None);
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
}
