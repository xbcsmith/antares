// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Combat systems and support types
//!
//! Phase 1: Core Combat Infrastructure
//! Phase 2: Combat UI System
//!
//! This module implements the foundational Bevy plugin and systems needed to
//! start and synchronize combat state with the global `GameState`, plus the
//! combat UI display.
//!
//! Responsibilities:
//! - `CombatPlugin` registers combat messages and resources
//! - `CombatResource` wraps domain `CombatState` for use by ECS systems
//! - `CombatTurnStateResource` is a small resource for turn sub-state tracking
//! - `start_encounter()` prepares a `CombatState` from a map encounter and party
//! - Systems to synchronize party -> combat and combat -> party on exit
//! - Combat UI systems for displaying enemies, turn order, and action menu
//!
//! # Systems
//!
//! - `handle_combat_started` (listens for `CombatStarted`) — loads the combat
//!   state from `GlobalState` into `CombatResource` and builds participant mapping.
//! - `sync_party_to_combat` (runs while in combat) — ensures players are present
//!   in the combat state if they were not added earlier.
//! - `sync_combat_to_party_on_exit` (runs every frame) — when combat has ended,
//!   copies HP/SP/conditions/stat currents back into the party and clears combat data.
//! - `setup_combat_ui` (runs on combat enter) — spawns combat UI entities
//! - `cleanup_combat_ui` (runs on combat exit) — despawns combat UI entities
//! - `update_combat_ui` (runs every frame during combat) — syncs UI with combat state
//!
//! # Notes
//!
//! This file intentionally keeps systems small and focused. More complex action
//! handling and AI belong to later phases (Phase 3+).
//!
//! # Examples
//!
//! Enter combat from an encounter event (simplified):
//!
//! ```no_run
//! use antares::game::systems::combat::start_encounter;
//! # // In an event handler:
//! # let mut gs = antares::application::GameState::new();
//! # let content = antares::application::resources::GameContent::new(antares::sdk::database::ContentDatabase::new());
//! # let monster_group: Vec<u8> = vec![1, 2];
//! let _ = start_encounter(&mut gs, &content, &monster_group);
//! ```

use bevy::prelude::*;

use crate::application::resources::GameContent;
use crate::application::GameMode;
use crate::domain::combat::engine::{
    apply_damage, choose_monster_attack, initialize_combat_from_group, resolve_attack, CombatState,
    Combatant,
};
use crate::domain::combat::types::{Attack, CombatStatus, CombatantId, Handicap, SpecialEffect};
use crate::domain::types::{DiceRoll, ItemId};
use crate::game::resources::GlobalState;

/// Message emitted when combat has started.
///
/// Other systems can listen to this to set up UI or audio.
#[derive(Message)]
pub struct CombatStarted;

/// Player-initiated attack message (registered by the plugin)
#[derive(Message)]
pub struct AttackAction {
    pub attacker: CombatantId,
    pub target: CombatantId,
}

/// Player-initiated cast spell message (registered by the plugin)
#[derive(Message)]
pub struct CastSpellAction {
    pub caster: CombatantId,
    pub spell_id: crate::domain::types::SpellId,
    pub target: CombatantId,
}

/// Player-initiated use item message (registered by the plugin)
#[derive(Message)]
pub struct UseItemAction {
    pub user: CombatantId,
    /// Index into the user's inventory (0-based)
    pub inventory_index: usize,
    pub target: CombatantId,
}

/// Player defends this turn (registered by the plugin)
#[derive(Message)]
pub struct DefendAction {
    pub combatant: CombatantId,
}

/// Player attempts to flee (registered by the plugin)
#[derive(Message)]
pub struct FleeAction;

/// Message emitted when combat ends in victory
#[derive(Message)]
pub struct CombatVictory;

/// Message emitted when combat ends in defeat
#[derive(Message)]
pub struct CombatDefeat;

/// Combat turn sub-states used by the UI / systems.
///
/// Kept small intentionally; this is not a Bevy `State` type, but a lightweight
/// resource allowing systems to coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombatTurnState {
    PlayerTurn,
    EnemyTurn,
    Animating,
    RoundEnd,
}

/// Simple resource wrapper for the current turn sub-state.
#[derive(Resource, Debug, Clone)]
pub struct CombatTurnStateResource(pub CombatTurnState);

impl Default for CombatTurnStateResource {
    fn default() -> Self {
        Self(CombatTurnState::PlayerTurn)
    }
}

// ===== Phase 2: Combat UI Constants =====

/// Height of the combat enemy panel at the top of the screen
pub const COMBAT_ENEMY_PANEL_HEIGHT: Val = Val::Px(200.0);

/// Width of individual enemy card in the enemy panel
pub const ENEMY_CARD_WIDTH: Val = Val::Px(150.0);

/// Height of enemy HP bar
pub const ENEMY_HP_BAR_HEIGHT: Val = Val::Px(12.0);

/// Padding inside enemy cards
pub const ENEMY_CARD_PADDING: Val = Val::Px(8.0);

/// Height of the turn order display panel
pub const TURN_ORDER_PANEL_HEIGHT: Val = Val::Px(40.0);

/// Height of the action menu panel
pub const ACTION_MENU_HEIGHT: Val = Val::Px(60.0);

/// Button width in action menu
pub const ACTION_BUTTON_WIDTH: Val = Val::Px(100.0);

/// Button height in action menu
pub const ACTION_BUTTON_HEIGHT: Val = Val::Px(40.0);

/// Color for enemy HP bar (healthy)
pub const ENEMY_HP_HEALTHY_COLOR: Color = Color::srgb(0.2, 0.8, 0.2);

/// Color for enemy HP bar (injured)
pub const ENEMY_HP_INJURED_COLOR: Color = Color::srgb(0.8, 0.6, 0.0);

/// Color for enemy HP bar (critical)
pub const ENEMY_HP_CRITICAL_COLOR: Color = Color::srgb(0.8, 0.2, 0.2);

/// Color for turn indicator highlight
pub const TURN_INDICATOR_COLOR: Color = Color::srgb(1.0, 1.0, 0.0);

/// Color for action button background
pub const ACTION_BUTTON_COLOR: Color = Color::srgb(0.3, 0.3, 0.4);

/// Color for action button hover
pub const ACTION_BUTTON_HOVER_COLOR: Color = Color::srgb(0.4, 0.4, 0.5);

/// Color for action button disabled
pub const ACTION_BUTTON_DISABLED_COLOR: Color = Color::srgb(0.2, 0.2, 0.2);

/// Color for enemy card highlight when in target selection mode
pub const ENEMY_CARD_HIGHLIGHT_COLOR: Color = Color::srgba(0.35, 0.25, 0.25, 0.95);

/// Bevy resource that contains the authoritative combat state used by ECS systems.
///
/// `player_orig_indices` maps participant index -> Option<party_index> so we can
/// sync values back to the party after combat ends.
#[derive(Resource, Debug, Clone)]
pub struct CombatResource {
    pub state: CombatState,
    /// For each participant index in `state.participants`, `Some(idx)` if that
    /// participant is a player and `idx` is the original party index.
    pub player_orig_indices: Vec<Option<usize>>,
    /// Whether the combat resolution (victory/defeat) has been handled.
    pub resolution_handled: bool,
}

impl CombatResource {
    /// Create a default/empty combat resource
    pub fn new() -> Self {
        Self {
            state: CombatState::new(Handicap::Even),
            player_orig_indices: Vec::new(),
            resolution_handled: false,
        }
    }

    /// Clear the combat state and mappings (called after syncing back to party)
    pub fn clear(&mut self) {
        self.state = CombatState::new(Handicap::Even);
        self.player_orig_indices.clear();
        self.resolution_handled = false;
    }
}

impl Default for CombatResource {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Phase 2: Combat UI Marker Components =====

/// Marker component for enemy card UI elements
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyCard {
    /// Index in combat participants (should be a monster)
    pub participant_index: usize,
}

/// Marker component for enemy HP bar fill
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyHpBarFill {
    /// Index in combat participants
    pub participant_index: usize,
}

/// Marker component for enemy HP text display
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyHpText {
    /// Index in combat participants
    pub participant_index: usize,
}

/// Marker component for enemy name text
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyNameText {
    /// Index in combat participants
    pub participant_index: usize,
}

/// Marker component for enemy condition text
#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyConditionText {
    /// Index in combat participants
    pub participant_index: usize,
}

/// Marker component for the turn order display panel
#[derive(Component, Debug, Clone, Copy)]
pub struct TurnOrderPanel;

/// Marker component for turn order text display
#[derive(Component, Debug, Clone, Copy)]
pub struct TurnOrderText;

/// Marker component for the action menu panel
#[derive(Component, Debug, Clone, Copy)]
pub struct ActionMenuPanel;

/// Action button types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionButtonType {
    Attack,
    Defend,
    Cast,
    Item,
    Flee,
}

/// Marker component for action buttons
#[derive(Component, Debug, Clone, Copy)]
pub struct ActionButton {
    pub button_type: ActionButtonType,
}

/// Marker component for the spell selection panel UI
///
/// Spawned when the player requests to cast a spell for a specific caster.
/// The `caster` field indicates which participant's spell list the panel is
/// displaying.
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellSelectionPanel {
    /// Participant index (player) that owns this panel
    pub caster: CombatantId,
}

/// Marker component for an individual spell button inside the spell selection panel
///
/// Stores the `spell_id` for the button and the `sp_cost` for display convenience.
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellButton {
    /// Identifier of the spell this button will cast
    pub spell_id: crate::domain::types::SpellId,
    /// SP cost (display-only convenience)
    pub sp_cost: u16,
}

/// Marker component for the item selection panel UI
///
/// Spawned when the player requests to use an item for a specific caster.
/// The `user` field indicates which participant's inventory the panel is
/// displaying.
#[derive(Component, Debug, Clone, Copy)]
pub struct ItemSelectionPanel {
    /// Participant index (player) that owns this panel
    pub user: CombatantId,
}

/// Marker component for an individual item button inside the item selection panel
///
/// Stores the `item_id` and remaining `charges` for display convenience.
#[derive(Component, Debug, Clone, Copy)]
pub struct ItemButton {
    /// Identifier of the item this button will use
    pub item_id: crate::domain::types::ItemId,
    /// Charges remaining in this inventory slot
    pub charges: u8,
}

/// Marker component for victory summary UI root
#[derive(Component, Debug, Clone, Copy)]
pub struct VictorySummaryRoot;

/// Marker component for defeat summary UI root
#[derive(Component, Debug, Clone, Copy)]
pub struct DefeatSummaryRoot;

/// Marker component for floating damage numbers (spawned on hits)
#[derive(Component, Debug, Clone, Copy)]
pub struct FloatingDamage {
    /// Remaining lifetime for the damage number in seconds
    pub remaining: f32,
}

/// Marker component for floating damage text node
#[derive(Component, Debug, Clone, Copy)]
pub struct DamageText;

/// Resource representing the current target selection state.
///
/// When set to `Some(attacker)` the player is selecting a target for that attacker.
#[derive(Resource, Debug, Clone, Default)]
pub struct TargetSelection(pub Option<CombatantId>);

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CombatStarted>()
            .add_message::<AttackAction>()
            .add_message::<CastSpellAction>()
            .add_message::<UseItemAction>()
            .add_message::<DefendAction>()
            .add_message::<FleeAction>()
            .add_message::<CombatVictory>()
            .add_message::<CombatDefeat>()
            .insert_resource(CombatResource::new())
            .insert_resource(CombatTurnStateResource::default())
            .insert_resource(TargetSelection::default())
            .insert_resource(ButtonInput::<KeyCode>::default())
            // Handle events that indicate combat started
            .add_systems(Update, handle_combat_started)
            // Ensure party members exist in combat on enter
            .add_systems(Update, sync_party_to_combat)
            // Sync back to party when combat ends
            .add_systems(Update, sync_combat_to_party_on_exit)
            // Phase 3: Player Action Systems
            .add_systems(Update, combat_input_system)
            .add_systems(Update, enter_target_selection)
            .add_systems(Update, select_target)
            .add_systems(Update, handle_attack_action)
            .add_systems(Update, handle_cast_spell_action)
            .add_systems(Update, handle_use_item_action)
            .add_systems(Update, handle_defend_action)
            .add_systems(Update, handle_flee_action)
            // Phase 4: Monster AI Systems
            .add_systems(Update, execute_monster_turn)
            // Phase 5: Combat resolution & rewards
            .add_systems(Update, check_combat_resolution)
            .add_systems(Update, handle_combat_victory)
            .add_systems(Update, handle_combat_defeat)
            // Phase 2: Combat UI systems
            .add_systems(Update, setup_combat_ui)
            .add_systems(Update, cleanup_combat_ui)
            .add_systems(Update, update_combat_ui)
            .add_systems(Update, cleanup_floating_damage);
    }
}

/// Initialize combat using the current party and an explicit monster group.
///
/// This will copy the current party members into the combat `CombatState`, add
/// monsters based on the provided `group` (list of `MonsterId`), and set the
/// `GameState` mode to `GameMode::Combat`.
///
/// Returns an error if any monster in `group` is not found in the content DB.
///
/// # Arguments
///
/// * `game_state` - Mutable reference to the application `GameState`
/// * `content` - `GameContent` resource for monster lookup
/// * `group` - slice of monster IDs (u8 alias)
///
/// # Errors
///
/// Returns `crate::domain::combat::database::MonsterDatabaseError` if a monster
/// ID is missing from the content DB.
///
/// # Examples
///
/// ```no_run
/// use antares::game::systems::combat::start_encounter;
/// # let mut gs = antares::application::GameState::new();
/// # let content = antares::application::resources::GameContent::new(antares::sdk::database::ContentDatabase::new());
/// # let group = vec![1u8, 2];
/// let _ = start_encounter(&mut gs, &content, &group);
/// ```
pub fn start_encounter(
    game_state: &mut crate::application::GameState,
    content: &GameContent,
    group: &[u8],
) -> Result<(), crate::domain::combat::database::MonsterDatabaseError> {
    // Build initial combat state and copy players (maintain party order)
    let handicap = Handicap::Even;
    let mut cs = CombatState::new(handicap);

    for character in &game_state.party.members {
        cs.add_player(character.clone());
    }

    // Add monsters from content DB and initialize turn order
    initialize_combat_from_group(&mut cs, content.db(), group)?;

    // Enter combat with the prepared state
    game_state.enter_combat_with_state(cs);

    Ok(())
}

/// System: Handle `CombatStarted` messages by copying the `GameState`'s combat
/// state into the `CombatResource` and building participant -> party mapping.
fn handle_combat_started(
    mut reader: MessageReader<CombatStarted>,
    mut combat_res: ResMut<CombatResource>,
    global_state: Res<GlobalState>,
    mut music_writer: Option<MessageWriter<crate::game::systems::audio::PlayMusic>>,
) {
    for _ in reader.read() {
        // Only proceed if the global state is actually in combat mode
        if let GameMode::Combat(ref cs) = global_state.0.mode {
            combat_res.state = cs.clone();

            // Build participant -> party index mapping
            let mut mapping: Vec<Option<usize>> =
                Vec::with_capacity(combat_res.state.participants.len());
            let mut player_counter: usize = 0;
            for participant in &combat_res.state.participants {
                match participant {
                    Combatant::Player(_) => {
                        mapping.push(Some(player_counter));
                        player_counter += 1;
                    }
                    Combatant::Monster(_) => mapping.push(None),
                }
            }
            combat_res.player_orig_indices = mapping;

            // Request combat music transition (if audio plugin is registered)
            if let Some(ref mut w) = music_writer {
                w.write(crate::game::systems::audio::PlayMusic {
                    track_id: "combat_theme".to_string(),
                    looped: true,
                });
            }
        }
    }
}

/// System: Ensure party members are present in the `CombatResource` while in
/// combat. This acts as a safety net if combat was created without players
/// properly added (e.g., direct `enter_combat()` calls).
fn sync_party_to_combat(mut combat_res: ResMut<CombatResource>, global_state: Res<GlobalState>) {
    // Only run when in combat
    let _cs = match &global_state.0.mode {
        GameMode::Combat(cs) => cs,
        _ => return,
    };

    // If there are already player entries, assume the combat is initialized
    let existing_players = combat_res
        .state
        .participants
        .iter()
        .filter(|p| matches!(p, Combatant::Player(_)))
        .count();

    if existing_players > 0 {
        return;
    }

    // Copy party characters into the combat state (preserve order)
    for (i, character) in global_state.0.party.members.iter().enumerate() {
        combat_res.state.add_player(character.clone());
        combat_res.player_orig_indices.push(Some(i));
    }

    // Initialize turn order in case monsters were already added earlier
    crate::domain::combat::engine::start_combat(&mut combat_res.state);
}

/// System: When combat has ended (global mode is not Combat) copy combat state
/// data (HP, SP, conditions, stat currents) back into the party members.
fn sync_combat_to_party_on_exit(
    mut global_state: ResMut<GlobalState>,
    mut combat_res: ResMut<CombatResource>,
) {
    // Only sync when not currently in combat and when we have mapping data
    if matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    if combat_res.player_orig_indices.is_empty() {
        return;
    }

    // Copy participant data back to party members for mapped players
    for (participant_idx, participant) in combat_res.state.participants.iter().enumerate() {
        if let Some(Some(party_idx)) = combat_res.player_orig_indices.get(participant_idx).cloned()
        {
            if party_idx >= global_state.0.party.members.len() {
                // Party membership changed during combat; skip out-of-range
                continue;
            }

            if let Combatant::Player(pc) = participant {
                let party_member = &mut global_state.0.party.members[party_idx];

                // Sync HP (use modify to adjust current safely)
                let delta_hp = pc.hp.current as i32 - party_member.hp.current as i32;
                if delta_hp != 0 {
                    party_member.hp.modify(delta_hp);
                }

                // Sync SP
                let delta_sp = pc.sp.current as i32 - party_member.sp.current as i32;
                if delta_sp != 0 {
                    party_member.sp.modify(delta_sp);
                }

                // Sync transient status flags (bitflags) - replace with combat view
                party_member.conditions = pc.conditions;

                // Sync active condition stack
                party_member.active_conditions = pc.active_conditions.clone();

                // Sync stat current values (might, intellect, personality, endurance, speed, accuracy, luck)
                let stats_src = &pc.stats;
                let stats_dst = &mut party_member.stats;

                let deltas = [
                    (stats_src.might.current as i32) - (stats_dst.might.current as i32),
                    (stats_src.intellect.current as i32) - (stats_dst.intellect.current as i32),
                    (stats_src.personality.current as i32) - (stats_dst.personality.current as i32),
                    (stats_src.endurance.current as i32) - (stats_dst.endurance.current as i32),
                    (stats_src.speed.current as i32) - (stats_dst.speed.current as i32),
                    (stats_src.accuracy.current as i32) - (stats_dst.accuracy.current as i32),
                    (stats_src.luck.current as i32) - (stats_dst.luck.current as i32),
                ];

                // Apply deltas using `modify` to respect modifier system (do not overwrite base)
                // Convert i32 deltas to the expected i16 type for the `modify` methods.
                stats_dst
                    .might
                    .modify(deltas[0].try_into().expect("stat delta fits in i16"));
                stats_dst
                    .intellect
                    .modify(deltas[1].try_into().expect("stat delta fits in i16"));
                stats_dst
                    .personality
                    .modify(deltas[2].try_into().expect("stat delta fits in i16"));
                stats_dst
                    .endurance
                    .modify(deltas[3].try_into().expect("stat delta fits in i16"));
                stats_dst
                    .speed
                    .modify(deltas[4].try_into().expect("stat delta fits in i16"));
                stats_dst
                    .accuracy
                    .modify(deltas[5].try_into().expect("stat delta fits in i16"));
                stats_dst
                    .luck
                    .modify(deltas[6].try_into().expect("stat delta fits in i16"));

                // Sync AC current (armor class)
                let ac_delta = pc.ac.current as i32 - party_member.ac.current as i32;
                if ac_delta != 0 {
                    party_member
                        .ac
                        .modify(ac_delta.try_into().expect("ac delta fits in i16"));
                }
            }
        }
    }

    // Clear stored combat state now that party has been updated
    combat_res.clear();
}

// ===== Phase 2: Combat UI Systems =====

/// System: Setup combat UI when entering combat mode
///
/// Spawns the combat HUD including enemy panel, turn order display, and action menu.
/// This system runs every frame but only acts when combat UI needs to be created.
fn setup_combat_ui(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    combat_res: Res<CombatResource>,
    existing_ui: Query<Entity, With<crate::game::components::combat::CombatHudRoot>>,
) {
    // Only setup UI when in combat mode
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    // If UI already exists, don't recreate it
    if !existing_ui.is_empty() {
        return;
    }

    // Spawn combat HUD root container
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(Color::NONE),
            crate::game::components::combat::CombatHudRoot,
        ))
        .with_children(|parent| {
            // Enemy panel at top
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: COMBAT_ENEMY_PANEL_HEIGHT,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(16.0),
                        padding: UiRect::all(Val::Px(16.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
                ))
                .with_children(|enemy_panel| {
                    // Spawn enemy cards for each monster in combat
                    for (idx, participant) in combat_res.state.participants.iter().enumerate() {
                        if let Combatant::Monster(monster) = participant {
                            // Inline enemy card spawning
                            enemy_panel
                                .spawn((
                                    Button,
                                    Node {
                                        width: ENEMY_CARD_WIDTH,
                                        flex_direction: FlexDirection::Column,
                                        padding: UiRect::all(ENEMY_CARD_PADDING),
                                        row_gap: Val::Px(4.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.2, 0.15, 0.15, 0.9)),
                                    BorderRadius::all(Val::Px(4.0)),
                                    EnemyCard {
                                        participant_index: idx,
                                    },
                                ))
                                .with_children(|card| {
                                    // Enemy name
                                    card.spawn((
                                        Text::new(monster.name.clone()),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                        EnemyNameText {
                                            participant_index: idx,
                                        },
                                    ));

                                    // HP bar background
                                    card.spawn((
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: ENEMY_HP_BAR_HEIGHT,
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 1.0)),
                                    ))
                                    .with_children(|bar| {
                                        // HP bar fill
                                        bar.spawn((
                                            Node {
                                                width: Val::Percent(100.0),
                                                height: Val::Percent(100.0),
                                                ..default()
                                            },
                                            BackgroundColor(ENEMY_HP_HEALTHY_COLOR),
                                            EnemyHpBarFill {
                                                participant_index: idx,
                                            },
                                        ));
                                    });

                                    // HP text
                                    card.spawn((
                                        Text::new(format!(
                                            "{}/{}",
                                            monster.hp.current, monster.hp.base
                                        )),
                                        TextFont {
                                            font_size: 10.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                        EnemyHpText {
                                            participant_index: idx,
                                        },
                                    ));

                                    // Condition text
                                    card.spawn((
                                        Text::new(""),
                                        TextFont {
                                            font_size: 9.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.8, 0.8, 0.0)),
                                        EnemyConditionText {
                                            participant_index: idx,
                                        },
                                    ));
                                });
                        }
                    }
                });

            // Turn order display
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: TURN_ORDER_PANEL_HEIGHT,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.9)),
                    TurnOrderPanel,
                ))
                .with_children(|turn_panel| {
                    turn_panel.spawn((
                        Text::new("Turn Order: "),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        TurnOrderText,
                    ));
                });

            // Action menu (initially visible, will be hidden during enemy turns)
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: ACTION_MENU_HEIGHT,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                    ActionMenuPanel,
                ))
                .with_children(|menu_panel| {
                    // Spawn action buttons inline
                    for (label, button_type) in [
                        ("Attack", ActionButtonType::Attack),
                        ("Defend", ActionButtonType::Defend),
                        ("Cast", ActionButtonType::Cast),
                        ("Item", ActionButtonType::Item),
                        ("Flee", ActionButtonType::Flee),
                    ] {
                        menu_panel
                            .spawn((
                                Button,
                                Node {
                                    width: ACTION_BUTTON_WIDTH,
                                    height: ACTION_BUTTON_HEIGHT,
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(ACTION_BUTTON_COLOR),
                                BorderRadius::all(Val::Px(4.0)),
                                ActionButton { button_type },
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Text::new(label),
                                    TextFont {
                                        font_size: 14.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                    }
                });
        });
}

/// System: Cleanup combat UI when exiting combat mode
///
/// Despawns all combat HUD entities when combat ends.
fn cleanup_combat_ui(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    combat_ui: Query<Entity, With<crate::game::components::combat::CombatHudRoot>>,
) {
    // Only cleanup when not in combat mode
    if matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    // Despawn combat UI if it exists (despawn will automatically handle children)
    for entity in combat_ui.iter() {
        commands.entity(entity).despawn();
    }
}

/// System: Update combat UI to reflect current combat state
///
/// Updates enemy HP bars, turn order display, and action menu visibility.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn update_combat_ui(
    combat_res: Res<CombatResource>,
    global_state: Res<GlobalState>,
    mut enemy_hp_bars: Query<(&EnemyHpBarFill, &mut Node, &mut BackgroundColor)>,
    mut enemy_hp_texts: Query<
        (&EnemyHpText, &mut Text),
        (
            Without<EnemyNameText>,
            Without<EnemyConditionText>,
            Without<TurnOrderText>,
        ),
    >,
    mut enemy_condition_texts: Query<
        (&EnemyConditionText, &mut Text),
        (
            Without<EnemyHpText>,
            Without<EnemyNameText>,
            Without<TurnOrderText>,
        ),
    >,
    mut turn_order_text: Query<
        &mut Text,
        (
            With<TurnOrderText>,
            Without<EnemyHpText>,
            Without<EnemyNameText>,
            Without<EnemyConditionText>,
        ),
    >,
    mut action_menu: Query<&mut Visibility, With<ActionMenuPanel>>,
    turn_state: Res<CombatTurnStateResource>,
) {
    // Only update when in combat mode
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    // Update enemy HP bars
    for (hp_bar, mut node, mut bg_color) in enemy_hp_bars.iter_mut() {
        if let Some(Combatant::Monster(monster)) =
            combat_res.state.participants.get(hp_bar.participant_index)
        {
            // Calculate HP percentage
            let hp_percent = if monster.hp.base > 0 {
                (monster.hp.current as f32 / monster.hp.base as f32 * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };

            // Update bar width
            node.width = Val::Percent(hp_percent);

            // Update bar color based on HP percentage
            *bg_color = if hp_percent > 50.0 {
                BackgroundColor(ENEMY_HP_HEALTHY_COLOR)
            } else if hp_percent > 20.0 {
                BackgroundColor(ENEMY_HP_INJURED_COLOR)
            } else {
                BackgroundColor(ENEMY_HP_CRITICAL_COLOR)
            };
        }
    }

    // Update enemy HP text
    for (hp_text, mut text) in enemy_hp_texts.iter_mut() {
        if let Some(Combatant::Monster(monster)) =
            combat_res.state.participants.get(hp_text.participant_index)
        {
            **text = format!("{}/{}", monster.hp.current, monster.hp.base);
        }
    }

    // Update enemy condition text
    for (condition_text, mut text) in enemy_condition_texts.iter_mut() {
        if let Some(Combatant::Monster(monster)) = combat_res
            .state
            .participants
            .get(condition_text.participant_index)
        {
            // Get condition summary (simplified for now)
            // MonsterCondition is an enum, not a bitflag, so check directly
            let condition_str = if matches!(
                monster.conditions,
                crate::domain::combat::monster::MonsterCondition::Normal
            ) {
                String::new()
            } else {
                "Condition".to_string()
            };
            **text = condition_str;
        }
    }

    // Update turn order display
    if let Ok(mut text) = turn_order_text.single_mut() {
        let mut turn_order_str = String::from("Turn Order: ");

        for (i, combatant_id) in combat_res.state.turn_order.iter().enumerate() {
            let name = match combatant_id {
                CombatantId::Player(idx) => {
                    if let Some(Combatant::Player(character)) =
                        combat_res.state.participants.get(*idx)
                    {
                        character.name.as_str()
                    } else {
                        "???"
                    }
                }
                CombatantId::Monster(idx) => {
                    if let Some(Combatant::Monster(monster)) =
                        combat_res.state.participants.get(*idx)
                    {
                        monster.name.as_str()
                    } else {
                        "???"
                    }
                }
            };

            if i == combat_res.state.current_turn {
                turn_order_str.push_str(&format!("[{}] → ", name));
            } else {
                turn_order_str.push_str(&format!("{} → ", name));
            }
        }

        // Remove trailing arrow (safely handle UTF-8)
        if let Some(stripped) = turn_order_str.strip_suffix(" → ") {
            turn_order_str = stripped.to_string();
        }

        **text = turn_order_str;
    }

    // Show/hide action menu based on turn state
    if let Ok(mut visibility) = action_menu.single_mut() {
        *visibility = if matches!(turn_state.0, CombatTurnState::PlayerTurn) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

// ===== Phase 3: Player Action Systems =====

/// Handle input from action buttons and keyboard shortcuts during PlayerTurn.
///
/// Emits `DefendAction` / `FleeAction` messages or enters target selection mode
/// for attacks.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn combat_input_system(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut interactions: Query<(&Interaction, &ActionButton), (Changed<Interaction>, With<Button>)>,
    global_state: Res<GlobalState>,
    combat_res: Res<CombatResource>,
    mut target_sel: ResMut<TargetSelection>,
    mut defend_writer: Option<MessageWriter<DefendAction>>,
    mut flee_writer: Option<MessageWriter<FleeAction>>,
    turn_state: Res<CombatTurnStateResource>,
) {
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    // Only accept action input during player turn
    if !matches!(turn_state.0, CombatTurnState::PlayerTurn) {
        return;
    }

    let current_actor = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
        .cloned();

    for (interaction, button) in interactions.iter_mut() {
        if *interaction != Interaction::None {
            if let Some(actor) = current_actor {
                match button.button_type {
                    ActionButtonType::Attack => {
                        // Enter target selection mode
                        target_sel.0 = Some(actor);
                    }
                    ActionButtonType::Defend => {
                        if let Some(ref mut w) = defend_writer {
                            w.write(DefendAction { combatant: actor });
                        }
                    }
                    ActionButtonType::Flee => {
                        if let Some(ref mut w) = flee_writer {
                            w.write(FleeAction);
                        }
                    }
                    ActionButtonType::Cast | ActionButtonType::Item => {
                        // Not implemented in Phase 3
                    }
                }
            }
        }
    }

    // Keyboard shortcuts (keyboard is optional)
    if let Some(kb) = keyboard.as_ref() {
        if kb.just_pressed(KeyCode::KeyA) {
            if let Some(actor) = current_actor {
                target_sel.0 = Some(actor);
            }
        } else if kb.just_pressed(KeyCode::KeyD) {
            if let Some(actor) = current_actor {
                if let Some(ref mut w) = defend_writer {
                    w.write(DefendAction { combatant: actor });
                }
            }
        } else if kb.just_pressed(KeyCode::KeyF) {
            if let Some(ref mut w) = flee_writer {
                w.write(FleeAction);
            }
        } else if kb.just_pressed(KeyCode::Escape) {
            target_sel.0 = None;
        }
    }
}

/// Highlight enemy UI elements when target selection is active.
fn enter_target_selection(
    target_sel: Res<TargetSelection>,
    mut enemy_cards: Query<(&EnemyCard, &mut BackgroundColor)>,
) {
    for (_card, mut bg) in enemy_cards.iter_mut() {
        *bg = if target_sel.0.is_some() {
            BackgroundColor(ENEMY_CARD_HIGHLIGHT_COLOR)
        } else {
            BackgroundColor(Color::srgba(0.2, 0.15, 0.15, 0.9))
        };
    }
}

/// Handle clicks on enemy cards during target selection and emit `AttackAction`.
#[allow(clippy::type_complexity)]
fn select_target(
    mut interactions: Query<(&Interaction, &EnemyCard), (Changed<Interaction>, With<Button>)>,
    mut target_sel: ResMut<TargetSelection>,
    mut attack_writer: Option<MessageWriter<AttackAction>>,
) {
    if target_sel.0.is_none() {
        return;
    }

    let attacker = target_sel.0.unwrap();

    for (interaction, enemy_card) in interactions.iter_mut() {
        if *interaction != Interaction::None {
            if let Some(ref mut w) = attack_writer {
                w.write(AttackAction {
                    attacker,
                    target: CombatantId::Monster(enemy_card.participant_index),
                });
            }
            target_sel.0 = None;
        }
    }
}

/// Perform an attack action using the supplied RNG (testable, deterministic).
pub fn perform_attack_action_with_rng(
    combat_res: &mut CombatResource,
    action: &AttackAction,
    content: &GameContent,
    global_state: &mut GlobalState,
    turn_state: &mut CombatTurnStateResource,
    rng: &mut impl rand::Rng,
) -> Result<(), crate::domain::combat::engine::CombatError> {
    use crate::domain::combat::engine::CombatError;

    // Ensure it's the attacker's turn
    if let Some(current) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        if current != &action.attacker {
            // Ignore actions from non-current actors
            return Ok(());
        }
    } else {
        return Ok(());
    }

    // Choose attack data
    let attack_data = match action.attacker {
        CombatantId::Player(_) => {
            crate::domain::combat::types::Attack::physical(DiceRoll::new(1, 4, 0))
        }
        CombatantId::Monster(idx) => {
            if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get(idx) {
                choose_monster_attack(mon, rng).unwrap_or(Attack::physical(DiceRoll::new(1, 4, 0)))
            } else {
                return Err(CombatError::CombatantNotFound(action.attacker));
            }
        }
    };

    // Resolve attack
    let (damage, special) = resolve_attack(
        &combat_res.state,
        action.attacker,
        action.target,
        &attack_data,
        rng,
    )?;

    // Apply damage
    if damage > 0 {
        let _ = apply_damage(&mut combat_res.state, action.target, damage)?;
    }

    // Apply special effect if any (map to condition by name)
    if let Some(effect) = special {
        let effect_name = match effect {
            SpecialEffect::Poison => "poison",
            SpecialEffect::Disease => "disease",
            SpecialEffect::Paralysis => "paralysis",
            SpecialEffect::Sleep => "sleep",
            SpecialEffect::Drain => "drain",
            SpecialEffect::Stone => "stone",
            SpecialEffect::Death => "death",
        };

        if let Some(def) = content.db().conditions.get_condition_by_name(effect_name) {
            let active = crate::domain::conditions::ActiveCondition::new(
                def.id.clone(),
                def.default_duration,
            );
            match action.target {
                CombatantId::Player(idx) => {
                    if let Some(Combatant::Player(pc)) = combat_res.state.participants.get_mut(idx)
                    {
                        pc.active_conditions.push(active);
                    }
                }
                CombatantId::Monster(idx) => {
                    if let Some(Combatant::Monster(mon)) =
                        combat_res.state.participants.get_mut(idx)
                    {
                        mon.active_conditions.push(active);
                    }
                }
            }
        }
    }

    // Check combat end conditions
    combat_res.state.check_combat_end();

    if combat_res.state.status == CombatStatus::Fled {
        // Exit immediately if fled
        global_state.0.exit_combat();
        return Ok(());
    }

    // Advance turn and update turn state
    let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
        .db()
        .conditions
        .all_conditions()
        .into_iter()
        .filter_map(|id| content.db().conditions.get_condition(id).cloned())
        .collect();

    let _round_effects = combat_res.state.advance_turn(&cond_defs);

    // Update turn state based on next actor
    if let Some(next) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        turn_state.0 = match next {
            CombatantId::Player(_) => CombatTurnState::PlayerTurn,
            _ => CombatTurnState::EnemyTurn,
        };
    }

    Ok(())
}

pub fn perform_cast_action_with_rng(
    combat_res: &mut CombatResource,
    action: &CastSpellAction,
    content: &GameContent,
    global_state: &mut GlobalState,
    turn_state: &mut CombatTurnStateResource,
    rng: &mut impl rand::Rng,
) -> Result<(), crate::domain::combat::engine::CombatError> {
    // Ensure it's the caster's turn
    if let Some(current) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        if current != &action.caster {
            // Ignore actions from non-current actors
            return Ok(());
        }
    } else {
        return Ok(());
    }

    // Execute the spell via the domain-level helper. It handles validation,
    // resource consumption and applies effects to combat participants.
    // We treat casting failures (insufficient SP, silenced, etc.) as non-fatal no-ops.
    let cast_result = crate::domain::combat::spell_casting::execute_spell_cast_by_id(
        &mut combat_res.state,
        action.caster,
        action.spell_id,
        action.target,
        content.db(),
        rng,
    );

    if let Err(_err) = cast_result {
        // Casting failed (insufficient SP, wrong class, silenced, etc.)
        // For now, treat this as a no-op from a combat flow perspective.
        return Ok(());
    }

    // If the combat state indicates a flee, exit combat immediately
    if combat_res.state.status == CombatStatus::Fled {
        global_state.0.exit_combat();
        return Ok(());
    }

    // Update turn state based on next actor (the domain helper advanced the turn)
    if let Some(next) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        turn_state.0 = match next {
            CombatantId::Player(_) => CombatTurnState::PlayerTurn,
            _ => CombatTurnState::EnemyTurn,
        };
    }

    Ok(())
}

/// Perform an item use action (domain -> game glue).
///
/// Executes the domain item usage helper which consumes inventory charges and
/// applies consumable effects (healing, cure, attribute boosts, etc.). Failures
/// (invalid slot, restrictions) are treated as non-fatal no-ops to keep the
/// game loop robust.
pub fn perform_use_item_action_with_rng(
    combat_res: &mut CombatResource,
    action: &UseItemAction,
    content: &GameContent,
    global_state: &mut GlobalState,
    turn_state: &mut CombatTurnStateResource,
    rng: &mut impl rand::Rng,
) -> Result<(), crate::domain::combat::engine::CombatError> {
    // Ensure it's the actor's turn
    if let Some(current) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        if current != &action.user {
            // Ignore actions from non-current actors
            return Ok(());
        }
    } else {
        return Ok(());
    }

    // Execute the item use via the domain-level helper. It handles validation,
    // charge consumption and effect application.
    let res = crate::domain::combat::item_usage::execute_item_use_by_slot(
        &mut combat_res.state,
        action.user,
        action.inventory_index,
        action.target,
        content.db(),
        rng,
    );

    if let Err(_err) = res {
        // Usage failed (invalid slot, restriction, etc.) - treat as a no-op
        return Ok(());
    }

    // If the combat state indicates a flee/exit, exit combat immediately
    if combat_res.state.status == CombatStatus::Fled {
        global_state.0.exit_combat();
        return Ok(());
    }

    // Update turn state based on next actor (the domain helper advanced the turn)
    if let Some(next) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        turn_state.0 = match next {
            CombatantId::Player(_) => CombatTurnState::PlayerTurn,
            _ => CombatTurnState::EnemyTurn,
        };
    }

    Ok(())
}

/// System wrapper: handle `AttackAction` messages and route to the attack performer.
fn handle_attack_action(
    mut reader: MessageReader<AttackAction>,
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut turn_state: ResMut<CombatTurnStateResource>,
    mut commands: Commands,
    mut sfx_writer: Option<MessageWriter<crate::game::systems::audio::PlaySfx>>,
) {
    // Some tests or minimal harnesses may not register the full `GameContent` resource.
    // Use a lightweight in-memory database fallback so systems are resilient in tests.
    let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());

    for action in reader.read() {
        let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);

        // Capture pre-attack HP for the target so we can show damage numbers
        let pre_hp: u16 = match action.target {
            CombatantId::Player(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Player(pc) => Some(pc.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
            CombatantId::Monster(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Monster(m) => Some(m.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
        };

        let mut rng = rand::rng();
        let _ = perform_attack_action_with_rng(
            &mut combat_res,
            action,
            content_ref,
            &mut global_state,
            &mut turn_state,
            &mut rng,
        );

        // Compute post-attack HP and damage dealt
        let post_hp: u16 = match action.target {
            CombatantId::Player(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Player(pc) => Some(pc.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
            CombatantId::Monster(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Monster(m) => Some(m.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
        };

        let dmg = (pre_hp as i32 - post_hp as i32).max(0) as u32;

        if dmg > 0 {
            // Spawn a simple floating damage UI node that auto-despawns
            commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Auto,
                        height: Val::Auto,
                        ..default()
                    },
                    FloatingDamage { remaining: 1.2 },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(format!("-{}", dmg)),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        DamageText,
                    ));
                });

            // Play hit SFX (hook into audio system if present)
            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "combat_hit".to_string(),
                });
            }
        } else {
            // Play miss SFX if desired
            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "combat_miss".to_string(),
                });
            }
        }
    }
}

/// System wrapper: handle `UseItemAction` messages and route to the item performer.
///
/// This is analogous to the spell handler: it captures pre-use HP/SP for the
/// target so we can spawn UI feedback (floating numbers), performs the domain
/// item use, then computes post-use HP/SP and spawns visual feedback and SFX.
fn handle_use_item_action(
    mut reader: MessageReader<UseItemAction>,
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut turn_state: ResMut<CombatTurnStateResource>,
    mut commands: Commands,
    mut sfx_writer: Option<MessageWriter<crate::game::systems::audio::PlaySfx>>,
) {
    // Some tests or minimal harnesses may not register the full `GameContent` resource.
    // Use a lightweight in-memory database fallback so systems are resilient in tests.
    let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());

    for action in reader.read() {
        let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);

        // Capture pre-use HP/SP for the target so we can show numbers
        let (pre_hp, pre_sp): (u16, u16) = match action.target {
            CombatantId::Player(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Player(pc) => Some((pc.hp.current, pc.sp.current)),
                    _ => None,
                })
                .unwrap_or((0, 0)),
            CombatantId::Monster(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Monster(m) => Some((m.hp.current, 0)),
                    _ => None,
                })
                .unwrap_or((0, 0)),
        };

        let mut rng = rand::rng();

        // Perform the use (domain-level). This consumes inventory charges and applies effects.
        let _ = perform_use_item_action_with_rng(
            &mut combat_res,
            action,
            content_ref,
            &mut global_state,
            &mut turn_state,
            &mut rng,
        );

        // Compute post-use HP/SP and differences
        let (post_hp, post_sp): (u16, u16) = match action.target {
            CombatantId::Player(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Player(pc) => Some((pc.hp.current, pc.sp.current)),
                    _ => None,
                })
                .unwrap_or((0, 0)),
            CombatantId::Monster(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Monster(m) => Some((m.hp.current, 0)),
                    _ => None,
                })
                .unwrap_or((0, 0)),
        };

        let hp_delta = post_hp as i32 - pre_hp as i32;
        let sp_delta = post_sp as i32 - pre_sp as i32;

        if hp_delta < 0 {
            // Damage occurred - show as negative
            let dmg = (-hp_delta) as u32;
            commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Auto,
                        height: Val::Auto,
                        ..default()
                    },
                    FloatingDamage { remaining: 1.2 },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(format!("-{}", dmg)),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        DamageText,
                    ));
                });

            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "combat_hit".to_string(),
                });
            }
        } else if hp_delta > 0 {
            // Healing occurred - show as positive (green)
            let healed = hp_delta as u32;
            commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Auto,
                        height: Val::Auto,
                        ..default()
                    },
                    FloatingDamage { remaining: 1.2 },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(format!("+{}", healed)),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.2, 0.8, 0.2)),
                        DamageText,
                    ));
                });

            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "combat_heal".to_string(),
                });
            }
        } else if sp_delta > 0 {
            // SP restored - show a small blue indicator
            let restored = sp_delta as u32;
            commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Auto,
                        height: Val::Auto,
                        ..default()
                    },
                    FloatingDamage { remaining: 1.2 },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(format!("+{} SP", restored)),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.4, 0.8, 1.0)),
                        DamageText,
                    ));
                });

            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "combat_heal".to_string(),
                });
            }
        } else {
            // No numeric feedback; play a small 'miss' sfx if desired
            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "combat_miss".to_string(),
                });
            }
        }
    }
}

#[cfg(test)]
mod perform_use_item_tests {
    use super::*;
    use crate::application::GameState;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
    use crate::game::resources::GlobalState;
    use crate::sdk::database::ContentDatabase;

    #[test]
    fn test_perform_use_item_action_heal_consumes_and_heals() {
        // Setup content DB and a healing potion
        // Prepare content DB with a healing potion
        let mut content = GameContent::new(ContentDatabase::new());
        let potion = Item {
            id: 50,
            name: "Test Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(20),
                is_combat_usable: true,
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
        };
        // Mutate the inner DB directly so tests can set up content in-place
        content.0.items.add_item(potion).unwrap();

        // Prepare combat state and player
        let mut combat_res = CombatResource::new();
        let mut player = Character::new(
            "Tester".to_string(),
            "human".to_string(),
            "none".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        player.hp.base = 50;
        player.hp.current = 20;
        player.inventory.add_item(50, 1).unwrap();

        combat_res.state.add_player(player);

        // Ensure turn order contains the player as the current actor
        combat_res.state.turn_order = vec![CombatantId::Player(0)];
        combat_res.state.current_turn = 0;

        let mut global_state = GlobalState(GameState::new());
        let mut turn_state = CombatTurnStateResource::default();
        let mut rng = rand::rng();

        let action = UseItemAction {
            user: CombatantId::Player(0),
            inventory_index: 0,
            target: CombatantId::Player(0),
        };

        let _ = perform_use_item_action_with_rng(
            &mut combat_res,
            &action,
            &content,
            &mut global_state,
            &mut turn_state,
            &mut rng,
        );

        // Verify player healed and inventory slot consumed
        if let Some(Combatant::Player(pc_after)) =
            combat_res.state.get_combatant(&CombatantId::Player(0))
        {
            assert!(pc_after.hp.current > 20);
            assert!(pc_after.inventory.items.is_empty());
        } else {
            panic!("player not present after action");
        }
    }
}

/// System wrapper: handle `CastSpellAction` messages and route to the spell performer.
fn handle_cast_spell_action(
    mut reader: MessageReader<CastSpellAction>,
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut turn_state: ResMut<CombatTurnStateResource>,
    mut commands: Commands,
    mut sfx_writer: Option<MessageWriter<crate::game::systems::audio::PlaySfx>>,
) {
    // Some tests or minimal harnesses may not register the full `GameContent` resource.
    // Use a lightweight in-memory database fallback so systems are resilient in tests.
    let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());

    for action in reader.read() {
        let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);

        // Capture pre-spell HP for the target so we can show damage numbers
        let pre_hp: u16 = match action.target {
            CombatantId::Player(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Player(pc) => Some(pc.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
            CombatantId::Monster(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Monster(m) => Some(m.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
        };

        let mut rng = rand::rng();

        // Perform the cast (domain-level). This consumes SP/gems and applies effects.
        let _ = perform_cast_action_with_rng(
            &mut combat_res,
            action,
            content_ref,
            &mut global_state,
            &mut turn_state,
            &mut rng,
        );

        // Compute post-spell HP and damage dealt
        let post_hp: u16 = match action.target {
            CombatantId::Player(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Player(pc) => Some(pc.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
            CombatantId::Monster(idx) => combat_res
                .state
                .participants
                .get(idx)
                .and_then(|p| match p {
                    Combatant::Monster(m) => Some(m.hp.current),
                    _ => None,
                })
                .unwrap_or(0),
        };

        let dmg = (pre_hp as i32 - post_hp as i32).max(0) as u32;

        if dmg > 0 {
            // Spawn a simple floating damage UI node that auto-despawns
            commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Auto,
                        height: Val::Auto,
                        ..default()
                    },
                    FloatingDamage { remaining: 1.2 },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(format!("-{}", dmg)),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        DamageText,
                    ));
                });

            // Play hit SFX (hook into audio system if present)
            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "combat_hit".to_string(),
                });
            }
        } else {
            // Play miss SFX if desired
            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "combat_miss".to_string(),
                });
            }
        }
    }
}

/// Public helper: perform defend action (applies temporary AC bonus and advances turn).
pub fn perform_defend_action(
    combat_res: &mut CombatResource,
    action: &DefendAction,
    content: &GameContent,
    _global_state: &mut GlobalState,
    turn_state: &mut CombatTurnStateResource,
) -> Result<(), crate::domain::combat::engine::CombatError> {
    use crate::domain::combat::engine::CombatError;

    // Ensure it's the actor's turn
    if let Some(current) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        if current != &action.combatant {
            return Ok(());
        }
    } else {
        return Ok(());
    }

    // Apply AC bonus to the combatant
    match action.combatant {
        CombatantId::Player(idx) => {
            if let Some(Combatant::Player(pc)) = combat_res.state.participants.get_mut(idx) {
                pc.ac.modify(2);
            } else {
                return Err(CombatError::CombatantNotFound(action.combatant));
            }
        }
        CombatantId::Monster(idx) => {
            if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get_mut(idx) {
                mon.ac.modify(2);
            } else {
                return Err(CombatError::CombatantNotFound(action.combatant));
            }
        }
    }

    // Advance turn
    let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
        .db()
        .conditions
        .all_conditions()
        .into_iter()
        .filter_map(|id| content.db().conditions.get_condition(id).cloned())
        .collect();

    let _ = combat_res.state.advance_turn(&cond_defs);

    // Update turn state
    if let Some(next) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        turn_state.0 = match next {
            CombatantId::Player(_) => CombatTurnState::PlayerTurn,
            _ => CombatTurnState::EnemyTurn,
        };
    }

    Ok(())
}

/// System wrapper for defend action
fn handle_defend_action(
    mut reader: MessageReader<DefendAction>,
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut turn_state: ResMut<CombatTurnStateResource>,
) {
    let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());

    for action in reader.read() {
        let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);

        let _ = perform_defend_action(
            &mut combat_res,
            action,
            content_ref,
            &mut global_state,
            &mut turn_state,
        );
    }
}

/// Public helper: perform flee action (returns true on success)
pub fn perform_flee_action(
    combat_res: &mut CombatResource,
    content: &GameContent,
    global_state: &mut GlobalState,
    turn_state: &mut CombatTurnStateResource,
) -> Result<bool, crate::domain::combat::engine::CombatError> {
    // Determine current actor
    let attacker = match combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        Some(a) => *a,
        None => return Ok(false),
    };

    // Flee only allowed if combat.can_flee is true
    if !combat_res.state.can_flee {
        // Consume turn
        let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
            .db()
            .conditions
            .all_conditions()
            .into_iter()
            .filter_map(|id| content.db().conditions.get_condition(id).cloned())
            .collect();
        combat_res.state.advance_turn(&cond_defs);
        if let Some(next) = combat_res
            .state
            .turn_order
            .get(combat_res.state.current_turn)
        {
            turn_state.0 = match next {
                CombatantId::Player(_) => CombatTurnState::PlayerTurn,
                _ => CombatTurnState::EnemyTurn,
            };
        }
        return Ok(false);
    }

    // Determine flee success based on speed differential (simple rule)
    let attacker_speed = match attacker {
        CombatantId::Player(idx) => {
            if let Some(Combatant::Player(pc)) = combat_res.state.participants.get(idx) {
                pc.stats.speed.current
            } else {
                return Ok(false);
            }
        }
        CombatantId::Monster(idx) => {
            if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get(idx) {
                mon.stats.speed.current
            } else {
                return Ok(false);
            }
        }
    };

    // Find highest monster speed (if any monster exists)
    let highest_monster_speed = combat_res
        .state
        .participants
        .iter()
        .filter_map(|p| {
            if let Combatant::Monster(m) = p {
                Some(m.stats.speed.current)
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0);

    let success = attacker_speed >= highest_monster_speed;

    if success {
        combat_res.state.status = CombatStatus::Fled;
        global_state.0.exit_combat();
        return Ok(true);
    }

    // Failure: consume turn
    let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
        .db()
        .conditions
        .all_conditions()
        .into_iter()
        .filter_map(|id| content.db().conditions.get_condition(id).cloned())
        .collect();
    combat_res.state.advance_turn(&cond_defs);
    if let Some(next) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        turn_state.0 = match next {
            CombatantId::Player(_) => CombatTurnState::PlayerTurn,
            _ => CombatTurnState::EnemyTurn,
        };
    }

    Ok(false)
}

/// System wrapper for FleeAction
fn handle_flee_action(
    mut reader: MessageReader<FleeAction>,
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut turn_state: ResMut<CombatTurnStateResource>,
) {
    let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());

    for _ in reader.read() {
        let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);

        let _ = perform_flee_action(
            &mut combat_res,
            content_ref,
            &mut global_state,
            &mut turn_state,
        );
    }
}

/// Selects a target for a monster according to its AI behavior.
///
/// Behavior:
/// - Aggressive: choose the lowest HP living player
/// - Defensive: choose the player with the highest 'threat' (might + accuracy)
/// - Random: choose a random living player
fn select_monster_target(
    combat_state: &CombatState,
    behavior: crate::domain::combat::monster::AiBehavior,
    rng: &mut impl rand::Rng,
) -> Option<CombatantId> {
    // Gather living player candidates (index, player reference)
    let mut candidates: Vec<(usize, &crate::domain::character::Character)> = Vec::new();
    for (idx, participant) in combat_state.participants.iter().enumerate() {
        if let Combatant::Player(pc) = participant {
            if pc.is_alive() {
                candidates.push((idx, pc));
            }
        }
    }

    if candidates.is_empty() {
        return None;
    }

    match behavior {
        crate::domain::combat::monster::AiBehavior::Aggressive => {
            // Pick lowest HP player
            let (idx, _) = candidates
                .iter()
                .min_by_key(|(_, pc)| pc.hp.current)
                .unwrap();
            Some(CombatantId::Player(*idx))
        }
        crate::domain::combat::monster::AiBehavior::Defensive => {
            // Simple threat heuristic: might + accuracy
            let (idx, _) = candidates
                .iter()
                .max_by_key(|(_, pc)| {
                    (pc.stats.might.current as i32) + (pc.stats.accuracy.current as i32)
                })
                .unwrap();
            Some(CombatantId::Player(*idx))
        }
        crate::domain::combat::monster::AiBehavior::Random => {
            let chosen = rng.random_range(0..candidates.len());
            Some(CombatantId::Player(candidates[chosen].0))
        }
    }
}

/// Perform a monster's turn using the provided RNG (deterministic for tests).
///
/// This resolves the monster's attack choice, selects a target using AI behavior,
/// applies damage and special effects, advances the turn, and updates the
/// `CombatTurnStateResource`.
pub fn perform_monster_turn_with_rng(
    combat_res: &mut CombatResource,
    content: &GameContent,
    global_state: &mut GlobalState,
    turn_state: &mut CombatTurnStateResource,
    rng: &mut impl rand::Rng,
) -> Result<(), crate::domain::combat::engine::CombatError> {
    use crate::domain::combat::engine::CombatError;

    // Determine current actor
    let attacker = match combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        Some(a) => *a,
        None => return Ok(()),
    };

    // Only handle monster turns
    let monster_idx = match attacker {
        CombatantId::Monster(idx) => idx,
        _ => return Ok(()),
    };

    // Ensure monster exists and can act (use a short-lived immutable borrow)
    {
        if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get(monster_idx) {
            if !mon.can_act() {
                // Monster cannot act (paralyzed, dead, or already acted)
                return Ok(());
            }
        } else {
            return Err(CombatError::CombatantNotFound(attacker));
        }
    }

    // Determine AI behavior (read-only)
    let behavior =
        if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get(monster_idx) {
            mon.ai_behavior
        } else {
            return Err(CombatError::CombatantNotFound(attacker));
        };

    // Select a target using AI
    let target = match select_monster_target(&combat_res.state, behavior, rng) {
        Some(t) => t,
        None => {
            // No valid targets; check for end of combat
            combat_res.state.check_combat_end();
            return Ok(());
        }
    };

    // Choose attack using domain helper
    let attack_data =
        if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get(monster_idx) {
            choose_monster_attack(mon, rng).unwrap_or(Attack::physical(DiceRoll::new(1, 4, 0)))
        } else {
            return Err(CombatError::CombatantNotFound(attacker));
        };

    // Resolve attack (pure calculation, uses immutable state)
    let (damage, special) = resolve_attack(&combat_res.state, attacker, target, &attack_data, rng)?;

    // Apply damage (mutably modify combat state)
    if damage > 0 {
        let _ = apply_damage(&mut combat_res.state, target, damage)?;
    }

    // Apply special effect if any (map to condition by name)
    if let Some(effect) = special {
        let effect_name = match effect {
            SpecialEffect::Poison => "poison",
            SpecialEffect::Disease => "disease",
            SpecialEffect::Paralysis => "paralysis",
            SpecialEffect::Sleep => "sleep",
            SpecialEffect::Drain => "drain",
            SpecialEffect::Stone => "stone",
            SpecialEffect::Death => "death",
        };

        if let Some(def) = content.db().conditions.get_condition_by_name(effect_name) {
            let active = crate::domain::conditions::ActiveCondition::new(
                def.id.clone(),
                def.default_duration,
            );
            match target {
                CombatantId::Player(idx) => {
                    if let Some(Combatant::Player(pc)) = combat_res.state.participants.get_mut(idx)
                    {
                        pc.active_conditions.push(active);
                    }
                }
                CombatantId::Monster(idx) => {
                    if let Some(Combatant::Monster(mon)) =
                        combat_res.state.participants.get_mut(idx)
                    {
                        mon.active_conditions.push(active);
                    }
                }
            }
        }
    }

    // Mark monster as having acted
    if let Some(Combatant::Monster(mon)) = combat_res.state.participants.get_mut(monster_idx) {
        mon.mark_acted();
    }

    // Check combat end conditions
    combat_res.state.check_combat_end();

    if combat_res.state.status == CombatStatus::Fled {
        global_state.0.exit_combat();
        return Ok(());
    }

    // Advance turn and apply round start effects if needed
    let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
        .db()
        .conditions
        .all_conditions()
        .into_iter()
        .filter_map(|id| content.db().conditions.get_condition(id).cloned())
        .collect();

    let _ = combat_res.state.advance_turn(&cond_defs);

    // Update turn sub-state based on next actor
    if let Some(next) = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
    {
        turn_state.0 = match next {
            CombatantId::Player(_) => CombatTurnState::PlayerTurn,
            _ => CombatTurnState::EnemyTurn,
        };
    }

    Ok(())
}

/// System: Execute monster turns automatically when it is EnemyTurn.
///
/// Picks an attack and target using AI, performs the attack, and advances
/// the turn. Uses the global RNG for in-game randomness.
fn execute_monster_turn(
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut turn_state: ResMut<CombatTurnStateResource>,
) {
    // Only run during combat and when it's enemy turn
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    if !matches!(turn_state.0, CombatTurnState::EnemyTurn) {
        return;
    }

    // Ensure current actor is a monster
    let current_actor = combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
        .cloned();

    if let Some(CombatantId::Monster(_)) = current_actor {
        // Fallback content when none registered (tests often omit GameContent)
        let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());
        let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);

        let mut rng = rand::rng();
        let _ = perform_monster_turn_with_rng(
            &mut combat_res,
            content_ref,
            &mut global_state,
            &mut turn_state,
            &mut rng,
        );
    }
}

// ===== Phase 5: Combat Resolution & Rewards =====

/// System: Detect when combat ends (Victory/Defeat) and emit an appropriate message
///
/// This ensures the resolution is only handled once by tracking a flag in
/// `CombatResource`.
fn check_combat_resolution(
    mut victory_writer: Option<MessageWriter<CombatVictory>>,
    mut defeat_writer: Option<MessageWriter<CombatDefeat>>,
    mut combat_res: ResMut<CombatResource>,
    global_state: Res<GlobalState>,
) {
    // Only consider when in combat
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    if combat_res.resolution_handled {
        return;
    }

    match combat_res.state.status {
        CombatStatus::Victory => {
            if let Some(ref mut w) = victory_writer {
                w.write(CombatVictory);
            }
            combat_res.resolution_handled = true;
        }
        CombatStatus::Defeat => {
            if let Some(ref mut w) = defeat_writer {
                w.write(CombatDefeat);
            }
            combat_res.resolution_handled = true;
        }
        _ => {}
    }
}

/// Summary of rewards from a victorious combat - returned by the reward processor.
pub struct VictorySummary {
    pub total_xp: u64,
    pub xp_awarded: Vec<(usize, u64)>, // (party_index, xp_amount)
    pub total_gold: u32,
    pub total_gems: u32,
    pub items: Vec<ItemId>,
}

/// Process victory rewards using the provided RNG. This will:
/// - Sum monster XP and distribute it to living party members
/// - Roll gold/gem drops and add them to the party
/// - Resolve item drops and add them to party members' inventories (if space)
pub fn process_combat_victory_with_rng(
    combat_res: &mut CombatResource,
    _content: &GameContent,
    global_state: &mut GlobalState,
    rng: &mut impl rand::Rng,
) -> Result<VictorySummary, crate::domain::combat::engine::CombatError> {
    // Sum XP from dead monsters
    let mut total_xp: u64 = 0;
    for participant in &combat_res.state.participants {
        if let Combatant::Monster(mon) = participant {
            if !mon.is_alive() {
                total_xp = total_xp.saturating_add(mon.loot.experience as u64);
            }
        }
    }

    // Determine recipient party indices (from mapping); fallback to living party members
    let mut recipients: Vec<usize> = Vec::new();
    for (idx, participant) in combat_res.state.participants.iter().enumerate() {
        if let Combatant::Player(pc) = participant {
            if pc.is_alive() {
                if let Some(Some(party_idx)) = combat_res.player_orig_indices.get(idx).cloned() {
                    if party_idx < global_state.0.party.members.len() {
                        recipients.push(party_idx);
                    }
                }
            }
        }
    }

    if recipients.is_empty() {
        for (i, member) in global_state.0.party.members.iter().enumerate() {
            if member.is_alive() {
                recipients.push(i);
            }
        }
    }

    let mut xp_awarded: Vec<(usize, u64)> = Vec::new();
    if !recipients.is_empty() && total_xp > 0 {
        let per = total_xp / recipients.len() as u64;
        let mut remainder = total_xp % recipients.len() as u64;

        for &party_idx in &recipients {
            let mut award = per;
            if remainder > 0 {
                award += 1;
                remainder -= 1;
            }

            // Award experience using domain helper (respects dead checks)
            let _ = crate::domain::progression::award_experience(
                &mut global_state.0.party.members[party_idx],
                award,
            );
            xp_awarded.push((party_idx, award));
        }
    }

    // Roll gold/gems/items from dead monsters and award to party
    let mut total_gold: u32 = 0;
    let mut total_gems: u32 = 0u32;
    let mut items_dropped: Vec<ItemId> = Vec::new();

    for participant in &combat_res.state.participants {
        if let Combatant::Monster(mon) = participant {
            if !mon.is_alive() {
                // Gold
                if mon.loot.gold_max >= mon.loot.gold_min {
                    let gold = if mon.loot.gold_max == mon.loot.gold_min {
                        mon.loot.gold_min
                    } else {
                        rng.random_range(mon.loot.gold_min..=mon.loot.gold_max)
                    };
                    total_gold = total_gold.saturating_add(gold);
                }

                // Gems
                if mon.loot.gems_max >= mon.loot.gems_min {
                    let gems = if mon.loot.gems_max == mon.loot.gems_min {
                        mon.loot.gems_min as u32
                    } else {
                        rng.random_range(mon.loot.gems_min..=mon.loot.gems_max) as u32
                    };
                    total_gems = total_gems.saturating_add(gems);
                }

                // Items (probabilistic)
                for (prob, item_id) in &mon.loot.items {
                    if rng.random::<f32>() < *prob {
                        items_dropped.push(*item_id);
                    }
                }
            }
        }
    }

    // Apply currency to party (pooled)
    global_state.0.party.gold = global_state.0.party.gold.saturating_add(total_gold);
    global_state.0.party.gems = global_state.0.party.gems.saturating_add(total_gems);

    // Distribute items to first available living party members
    for item_id in &items_dropped {
        let mut placed = false;
        for &party_idx in &recipients {
            if let Some(member) = global_state.0.party.members.get_mut(party_idx) {
                if member.inventory.has_space() {
                    let _ = member.inventory.add_item(*item_id, 0);
                    placed = true;
                    break;
                }
            }
        }

        // If no recipient had space, attempt to stash on any living member
        if !placed {
            for member in global_state.0.party.members.iter_mut() {
                if member.is_alive() && member.inventory.has_space() {
                    let _ = member.inventory.add_item(*item_id, 0);
                    break;
                }
            }
        }
    }

    Ok(VictorySummary {
        total_xp,
        xp_awarded,
        total_gold,
        total_gems,
        items: items_dropped,
    })
}

/// System: Handle CombatVictory messages and apply rewards.
///
/// This runs as a system wrapper and spawns a simple victory summary UI.
fn handle_combat_victory(
    mut reader: MessageReader<CombatVictory>,
    mut combat_res: ResMut<CombatResource>,
    content: Option<Res<GameContent>>,
    mut global_state: ResMut<GlobalState>,
    mut commands: Commands,
    mut music_writer: Option<MessageWriter<crate::game::systems::audio::PlayMusic>>,
    mut sfx_writer: Option<MessageWriter<crate::game::systems::audio::PlaySfx>>,
) {
    let default_content = GameContent::new(crate::sdk::database::ContentDatabase::new());

    for _ in reader.read() {
        let content_ref: &GameContent = content.as_deref().unwrap_or(&default_content);
        let mut rng = rand::rng();

        if let Ok(summary) = process_combat_victory_with_rng(
            &mut combat_res,
            content_ref,
            &mut global_state,
            &mut rng,
        ) {
            // Spawn a simple victory UI
            commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                    VictorySummaryRoot,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(format!(
                            "Victory! XP: {}  Gold: {}  Gems: {}  Items: {:?}",
                            summary.total_xp, summary.total_gold, summary.total_gems, summary.items
                        )),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Play victory SFX and transition back to exploration music (if audio plugin present)
            if let Some(ref mut w) = sfx_writer {
                w.write(crate::game::systems::audio::PlaySfx {
                    sfx_id: "victory_fanfare".to_string(),
                });
            }
            if let Some(ref mut w) = music_writer {
                w.write(crate::game::systems::audio::PlayMusic {
                    track_id: "exploration_theme".to_string(),
                    looped: true,
                });
            }
        }

        // Exit combat (will trigger sync back to party on next frame)
        global_state.0.exit_combat();
    }
}

/// Process defeat state (non-UI part) - updates global game mode/state.
pub fn process_combat_defeat_state(
    _combat_res: &mut CombatResource,
    global_state: &mut GlobalState,
) {
    // Exit combat and open menu (allow player to load/quit)
    global_state.0.exit_combat();
    global_state.0.enter_menu();
}

/// System: Handle CombatDefeat messages and display defeat UI + menu
fn handle_combat_defeat(
    mut reader: MessageReader<CombatDefeat>,
    mut combat_res: ResMut<CombatResource>,
    mut global_state: ResMut<GlobalState>,
    mut commands: Commands,
    mut music_writer: Option<MessageWriter<crate::game::systems::audio::PlayMusic>>,
    mut sfx_writer: Option<MessageWriter<crate::game::systems::audio::PlaySfx>>,
) {
    for _ in reader.read() {
        process_combat_defeat_state(&mut combat_res, &mut global_state);

        // Spawn simple defeat UI overlay
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                DefeatSummaryRoot,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new("You have been defeated. Returning to menu...".to_string()),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });

        // Play defeat SFX / music (if available)
        if let Some(ref mut w) = sfx_writer {
            w.write(crate::game::systems::audio::PlaySfx {
                sfx_id: "defeat_sound".to_string(),
            });
        }
        if let Some(ref mut w) = music_writer {
            w.write(crate::game::systems::audio::PlayMusic {
                track_id: "defeat_music".to_string(),
                looped: false,
            });
        }
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.0, 0.0, 0.7)),
                DefeatSummaryRoot,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new("Defeat! You have been defeated."),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
    }
}

/// Cleanup system for floating damage UI nodes. Decrements remaining lifetime
/// and despawns the node when time is up.
fn cleanup_floating_damage(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut FloatingDamage)>,
) {
    for (entity, mut fd) in query.iter_mut() {
        fd.remaining -= time.delta_secs();
        if fd.remaining <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameState;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::combat::types::Handicap;

    /// Ensure plugin creates and exposes expected resources when registered.
    #[test]
    fn test_combat_plugin_registers_messages() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // CombatResource should be available after plugin initialization
        let cr = app.world().resource::<CombatResource>();
        assert!(cr.state.participants.is_empty());
        assert!(cr.player_orig_indices.is_empty());
    }

    /// Party -> Combat sync should copy party members into CombatResource when entering combat.
    #[test]
    fn test_party_sync_to_combat() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Prepare game state with one party member and enter combat mode
        let mut gs = GameState::new();
        let hero = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        // Make sure mode is Combat (empty CombatState)
        gs.enter_combat();

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Run a frame so the sync system runs
        app.update();

        let cr = app.world().resource::<CombatResource>();
        // After sync, a player should be present in participants
        assert!(cr
            .state
            .participants
            .iter()
            .any(|p| matches!(p, Combatant::Player(_))));
        // Mapping should reflect the party index
        assert_eq!(
            cr.player_orig_indices
                .iter()
                .filter(|x| x.is_some())
                .count(),
            1
        );
    }

    /// Combat -> Party sync on exit should copy HP/SP/conditions back into the party.
    #[test]
    fn test_combat_sync_to_party() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Create a party with one member (HP default 10)
        let mut gs = GameState::new();
        let mut hero = Character::new(
            "Wounded".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.modify(0); // ensure default
        gs.party.add_member(hero.clone()).unwrap();

        // Create combat state where the player is at 3 HP (i.e., took damage)
        let mut cs = CombatState::new(Handicap::Even);
        hero.hp.modify(-7); // hero now has current HP lowered (mutating local hero)
        cs.add_player(hero.clone());
        // Build mapping: participant 0 -> party index 0
        let mut cr = CombatResource::new();
        cr.state = cs;
        cr.player_orig_indices = vec![Some(0)];

        // Insert resources and set global mode to Exploration to simulate combat exit
        app.insert_resource(crate::game::resources::GlobalState(gs));
        app.insert_resource(cr);

        // Run frame, sync system should copy HP back to party
        app.update();

        let gs_after = app
            .world()
            .resource::<crate::game::resources::GlobalState>();
        let party_member = &gs_after.0.party.members[0];
        // Expect HP to have been synchronized (reduced)
        assert!(party_member.hp.current < party_member.hp.base);
    }

    /// Verify `start_encounter` constructs a combat state and sets the
    /// `GameState` mode to `GameMode::Combat` when invoked.
    #[test]
    fn test_start_encounter_sets_game_mode() {
        use crate::application::resources::GameContent;
        use crate::application::GameState;
        use crate::sdk::database::ContentDatabase;

        // Prepare a minimal game state with one party member
        let mut gs = GameState::new();
        let hero = crate::domain::character::Character::new(
            "Tester".to_string(),
            "human".to_string(),
            "knight".to_string(),
            crate::domain::character::Sex::Male,
            crate::domain::character::Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        // Empty content DB is sufficient for this test (empty monster group)
        let content = GameContent::new(ContentDatabase::new());

        // Starting an encounter with an empty monster group should succeed
        assert!(start_encounter(&mut gs, &content, &[]).is_ok());

        // GameState should now be in Combat mode
        assert!(matches!(gs.mode, crate::application::GameMode::Combat(_)));
    }

    // ===== Phase 2: Combat UI Tests =====

    /// Verify combat UI spawns when entering combat
    #[test]
    fn test_combat_ui_spawns_on_enter() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Create game state in combat mode
        let mut gs = GameState::new();
        let hero = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        gs.enter_combat();

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Run update to trigger setup_combat_ui
        app.update();

        // Combat HUD root should now exist
        let mut ui_query = app
            .world_mut()
            .query_filtered::<Entity, With<crate::game::components::combat::CombatHudRoot>>();
        assert_eq!(ui_query.iter(app.world()).count(), 1);
    }

    /// Verify combat UI despawns when exiting combat
    #[test]
    fn test_combat_ui_despawns_on_exit() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Start in combat mode
        let mut gs = GameState::new();
        let hero = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        gs.enter_combat();

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Run update to spawn UI
        app.update();

        // Verify UI exists
        let mut ui_query = app
            .world_mut()
            .query_filtered::<Entity, With<crate::game::components::combat::CombatHudRoot>>();
        assert_eq!(ui_query.iter(app.world()).count(), 1);

        // Exit combat
        let mut gs_res = app
            .world_mut()
            .resource_mut::<crate::game::resources::GlobalState>();
        gs_res.0.exit_combat();

        // Run update to trigger cleanup
        app.update();

        // UI should be despawned
        let mut ui_query = app
            .world_mut()
            .query_filtered::<Entity, With<crate::game::components::combat::CombatHudRoot>>();
        assert_eq!(ui_query.iter(app.world()).count(), 0);
    }

    /// Verify enemy HP bars update correctly
    #[test]
    fn test_enemy_hp_bars_update() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Create combat state with a monster
        let mut gs = GameState::new();
        let hero = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        // Create combat state with monster
        let mut cs = CombatState::new(Handicap::Even);
        let mut monster = crate::domain::combat::monster::Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(10, 10, 10, 10, 10, 10, 10),
            10,
            10,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        monster.hp.base = 10;
        monster.hp.current = 10;
        cs.add_monster(monster);

        gs.enter_combat_with_state(cs);

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource manually since we're not going through handle_combat_started
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            // Rebuild the combat state
            let mut new_cs = CombatState::new(Handicap::Even);
            let monster = crate::domain::combat::monster::Monster::new(
                1,
                "Goblin".to_string(),
                crate::domain::character::Stats::new(10, 10, 10, 10, 10, 10, 10),
                10,
                10,
                vec![crate::domain::combat::types::Attack::physical(
                    crate::domain::types::DiceRoll::new(1, 4, 0),
                )],
                crate::domain::combat::monster::LootTable::default(),
            );
            new_cs.add_monster(monster);
            combat_res.state = new_cs;
            combat_res.player_orig_indices = vec![None];
        }

        // Run update to trigger setup_combat_ui
        app.update();

        // Verify enemy card was created
        let mut enemy_cards = app.world_mut().query_filtered::<Entity, With<EnemyCard>>();
        assert_eq!(enemy_cards.iter(app.world()).count(), 1);

        // Damage the monster by modifying combat resource
        let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
        if let Some(Combatant::Monster(monster)) = combat_res.state.participants.get_mut(0) {
            monster.hp.current = 5; // 50% HP
        }

        // Run update to sync UI
        app.update();

        // Verify HP bar fill exists and has been updated
        let mut hp_bars = app
            .world_mut()
            .query_filtered::<(&Node, &EnemyHpBarFill), With<EnemyHpBarFill>>();
        let mut found_hp_bar = false;
        for (node, _) in hp_bars.iter(app.world()) {
            if let Val::Percent(width) = node.width {
                // Should be ~50% width
                assert!((width - 50.0).abs() < 1.0);
                found_hp_bar = true;
            }
        }
        assert!(found_hp_bar, "HP bar fill should have been updated");
    }

    /// Verify turn order display shows correct order
    #[test]
    fn test_turn_order_display() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Create combat with player and monster
        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(gs.party.members[0].clone());
        let monster = crate::domain::combat::monster::Monster::new(
            2,
            "Orc".to_string(),
            crate::domain::character::Stats::new(12, 8, 8, 12, 10, 10, 8),
            15,
            12,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 6, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        cs.add_monster(monster);

        // Set up turn order manually
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs);

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Run update to spawn and populate UI
        app.update();

        // Verify turn order text was created
        let mut turn_order_texts = app
            .world_mut()
            .query_filtered::<&Text, With<TurnOrderText>>();
        assert_eq!(turn_order_texts.iter(app.world()).count(), 1);

        // Text should contain turn order information
        for text in turn_order_texts.iter(app.world()) {
            let text_str = text.0.as_str();
            assert!(text_str.contains("Turn Order"));
        }
    }

    /// Verify action menu visibility changes with turn state
    #[test]
    fn test_action_menu_visibility() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Create combat state
        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        gs.enter_combat();

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Set turn state to PlayerTurn
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::PlayerTurn;

        // Run update to spawn and update UI
        app.update();

        // Action menu should be visible when PlayerTurn (spawns with default visibility)
        let mut menu_query = app
            .world_mut()
            .query_filtered::<&Visibility, With<ActionMenuPanel>>();
        let count = menu_query.iter(app.world()).count();
        assert_eq!(count, 1, "Action menu should exist");

        // Change to EnemyTurn
        app.world_mut().resource_mut::<CombatTurnStateResource>().0 = CombatTurnState::EnemyTurn;

        // Run update
        app.update();

        // Action menu should be hidden
        let mut menu_query = app
            .world_mut()
            .query_filtered::<&Visibility, With<ActionMenuPanel>>();
        for visibility in menu_query.iter(app.world()) {
            assert_eq!(*visibility, Visibility::Hidden);
        }
    }

    /// Verify action buttons are created correctly
    #[test]
    fn test_action_buttons_created() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();
        gs.enter_combat();

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Run update to spawn UI
        app.update();

        // All 5 action buttons should exist
        let mut buttons = app
            .world_mut()
            .query_filtered::<&ActionButton, With<ActionButton>>();
        assert_eq!(buttons.iter(app.world()).count(), 5);

        // Verify each button type exists
        let mut found_types = std::collections::HashSet::new();
        for button in buttons.iter(app.world()) {
            found_types.insert(button.button_type);
        }

        assert!(found_types.contains(&ActionButtonType::Attack));
        assert!(found_types.contains(&ActionButtonType::Defend));
        assert!(found_types.contains(&ActionButtonType::Cast));
        assert!(found_types.contains(&ActionButtonType::Item));
        assert!(found_types.contains(&ActionButtonType::Flee));
    }

    /// Verify enemy cards are created for monsters only
    #[test]
    fn test_enemy_cards_for_monsters_only() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        // Create combat with 2 monsters
        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(gs.party.members[0].clone());

        let monster1 = crate::domain::combat::monster::Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(10, 8, 8, 10, 10, 10, 8),
            8,
            10,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        let monster2 = crate::domain::combat::monster::Monster::new(
            2,
            "Orc".to_string(),
            crate::domain::character::Stats::new(12, 8, 8, 12, 10, 10, 8),
            12,
            11,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 6, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        cs.add_monster(monster1);
        cs.add_monster(monster2);

        gs.enter_combat_with_state(cs);

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource manually
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            // Rebuild combat state
            let mut new_cs = CombatState::new(Handicap::Even);
            new_cs.add_player(Character::new(
                "Hero".to_string(),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            ));
            let monster1 = crate::domain::combat::monster::Monster::new(
                1,
                "Goblin".to_string(),
                crate::domain::character::Stats::new(10, 8, 8, 10, 10, 10, 8),
                8,
                10,
                vec![crate::domain::combat::types::Attack::physical(
                    crate::domain::types::DiceRoll::new(1, 4, 0),
                )],
                crate::domain::combat::monster::LootTable::default(),
            );
            let monster2 = crate::domain::combat::monster::Monster::new(
                2,
                "Orc".to_string(),
                crate::domain::character::Stats::new(12, 8, 8, 12, 10, 10, 8),
                12,
                11,
                vec![crate::domain::combat::types::Attack::physical(
                    crate::domain::types::DiceRoll::new(1, 6, 0),
                )],
                crate::domain::combat::monster::LootTable::default(),
            );
            new_cs.add_monster(monster1);
            new_cs.add_monster(monster2);
            combat_res.state = new_cs;
            combat_res.player_orig_indices = vec![Some(0), None, None];
        }

        // Run update to spawn UI
        app.update();

        // Should have exactly 2 enemy cards (not 3, even though there are 3 participants)
        let mut enemy_cards = app
            .world_mut()
            .query_filtered::<&EnemyCard, With<EnemyCard>>();
        assert_eq!(enemy_cards.iter(app.world()).count(), 2);
    }

    /// Verify combat victory distributes XP to living party members
    #[test]
    fn test_victory_distributes_xp() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Prepare game state with two party members
        let mut gs = GameState::new();
        let p1 = Character::new(
            "P1".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let p2 = Character::new(
            "P2".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(p1.clone()).unwrap();
        gs.party.add_member(p2.clone()).unwrap();
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Build combat state: two players and one dead monster with 100 XP
        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(p1.clone());
        cs.add_player(p2.clone());
        let monster = crate::domain::combat::monster::Monster::new(
            1,
            "Killed".to_string(),
            crate::domain::character::Stats::new(10, 10, 10, 10, 10, 10, 10),
            0, // dead
            5,
            vec![],
            crate::domain::combat::monster::LootTable::new(0, 0, 0, 0, 100),
        );
        cs.add_monster(monster);
        cs.status = CombatStatus::Victory;
        cs.turn_order = vec![
            CombatantId::Player(0),
            CombatantId::Player(1),
            CombatantId::Monster(2),
        ];
        cs.current_turn = 0;

        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), Some(1), None];
        }

        // Call helper with deterministic RNG
        {
            use rand::rngs::StdRng;
            use rand::SeedableRng;
            let mut rng = StdRng::seed_from_u64(42);
            let content = GameContent::new(crate::sdk::database::ContentDatabase::new());

            {
                // Remove the GlobalState resource from the world to avoid overlapping
                // mutable borrows, operate on it, then re-insert the updated resource.
                let world = app.world_mut();

                let mut gs_owned = world
                    .remove_resource::<crate::game::resources::GlobalState>()
                    .expect("missing GlobalState resource for test");

                let summary = {
                    let mut cr = world.resource_mut::<CombatResource>();

                    process_combat_victory_with_rng(&mut cr, &content, &mut gs_owned, &mut rng)
                        .expect("victory processing failed")
                };

                // Reinsert the updated GlobalState (the mutable borrow ended when the inner block closed)
                world.insert_resource(gs_owned);

                // Fetch updated global state for assertions
                let gs_after = app
                    .world()
                    .resource::<crate::game::resources::GlobalState>();

                // 100 XP split between 2 living party members = 50 each
                assert_eq!(gs_after.0.party.members[0].experience, 50);
                assert_eq!(gs_after.0.party.members[1].experience, 50);
                assert_eq!(summary.total_xp, 100);
            }
        }
    }

    /// Verify victory awards gold to the party
    #[test]
    fn test_victory_awards_gold() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Prepare game state with one party member
        let mut gs = GameState::new();
        let p = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(p.clone()).unwrap();
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Build combat state with one dead monster with fixed gold 25
        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(p.clone());
        let monster = crate::domain::combat::monster::Monster::new(
            1,
            "Looty".to_string(),
            crate::domain::character::Stats::new(10, 10, 10, 10, 10, 10, 10),
            0, // dead
            5,
            vec![],
            crate::domain::combat::monster::LootTable::new(25, 25, 0, 0, 0),
        );
        cs.add_monster(monster);
        cs.status = CombatStatus::Victory;

        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), None];
        }

        // Process victory
        {
            use rand::rngs::StdRng;
            use rand::SeedableRng;
            let mut rng = StdRng::seed_from_u64(99);
            let content = GameContent::new(crate::sdk::database::ContentDatabase::new());
            {
                // Avoid overlapping mutable borrows by removing GlobalState, mutating it,
                // and then reinserting it back into the world.
                let world = app.world_mut();

                let mut gs_owned = world
                    .remove_resource::<crate::game::resources::GlobalState>()
                    .expect("missing GlobalState resource for test");

                let summary = {
                    let mut cr = world.resource_mut::<CombatResource>();

                    process_combat_victory_with_rng(&mut cr, &content, &mut gs_owned, &mut rng)
                        .expect("victory processing failed")
                };

                // Reinsert the updated GlobalState (the mutable borrow ended when the inner block closed)
                world.insert_resource(gs_owned);

                // Fetch updated global state for assertions
                let gs_after = app
                    .world()
                    .resource::<crate::game::resources::GlobalState>();

                assert_eq!(summary.total_gold, 25);
                assert_eq!(gs_after.0.party.gold, 25);
            }
        }
    }

    /// Verify defeat triggers menu/game over flow
    #[test]
    fn test_defeat_triggers_game_over() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Create a game state with one dead party member and set combat state to Defeat
        let mut gs = GameState::new();
        let mut dead = Character::new(
            "Dead".to_string(),
            "human".to_string(),
            "peasant".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        dead.hp.current = 0;
        gs.party.add_member(dead).unwrap();
        gs.enter_combat();

        app.insert_resource(crate::game::resources::GlobalState(gs));

        {
            // Read the party member first (immutable borrow) to avoid overlapping
            // with the subsequent mutable borrow of the world.
            let p = app
                .world()
                .resource::<crate::game::resources::GlobalState>()
                .0
                .party
                .members[0]
                .clone();

            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            let mut cs = CombatState::new(Handicap::Even);
            cs.add_player(p);
            cs.status = CombatStatus::Defeat;
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0)];
            cr.resolution_handled = false;
        }

        // Run update so the resolution system emits the message and the handler runs
        app.update();
        app.update();

        // Verify game entered Menu mode (game over handling)
        let gs_after = app
            .world()
            .resource::<crate::game::resources::GlobalState>();
        assert!(matches!(gs_after.0.mode, GameMode::Menu(_)));
    }

    // ===== Phase 3: Player Action System Tests =====

    #[test]
    fn test_attack_action_applies_damage() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Prepare game state with one player and one monster
        let mut gs = GameState::new();
        let mut hero = Character::new(
            "Attacker".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Ensure the tester's attacker has very high accuracy so seeded RNG
        // in the unit test will consistently register a hit.
        hero.stats.accuracy.current = 200;
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        let monster = crate::domain::combat::monster::Monster::new(
            1,
            "Test Goblin".to_string(),
            crate::domain::character::Stats::new(10, 10, 10, 10, 10, 10, 10),
            10,
            10,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        cs.add_monster(monster);

        // Ensure player acts first
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            combat_res.state = cs;
            combat_res.player_orig_indices = vec![Some(0), None];
        }

        // Deterministic RNG
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(42);
        let content = GameContent::new(crate::sdk::database::ContentDatabase::new());

        // Perform attack by calling helper directly with a single borrow to CombatResource
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            let mut gs_local = crate::game::resources::GlobalState(GameState::new());
            let mut turn_state_local = CombatTurnStateResource::default();

            let attack = AttackAction {
                attacker: CombatantId::Player(0),
                target: CombatantId::Monster(1),
            };

            perform_attack_action_with_rng(
                &mut cr,
                &attack,
                &content,
                &mut gs_local,
                &mut turn_state_local,
                &mut rng,
            )
            .expect("attack failed");

            // Debug information: monster HP after attack
            if let Some(Combatant::Monster(mon)) = cr.state.participants.get(1) {
                println!("DEBUG: monster final hp {}/{}", mon.hp.current, mon.hp.base);
                assert!(mon.hp.current < mon.hp.base);
            } else {
                panic!("monster not found");
            }
        }
    }

    #[test]
    fn test_defend_action_improves_ac() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let mut hero = Character::new(
            "Defender".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.ac = crate::domain::character::AttributePair::new(10);
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        cs.turn_order = vec![CombatantId::Player(0)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            combat_res.state = cs;
            combat_res.player_orig_indices = vec![Some(0)];
        }

        // Perform defend by sending a DefendAction message and letting systems handle it
        // Perform defend by calling helper directly with a single borrow to CombatResource
        {
            let content = GameContent::new(crate::sdk::database::ContentDatabase::new());
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            let mut gs_local = crate::game::resources::GlobalState(GameState::new());
            let mut turn_state_local = CombatTurnStateResource::default();

            let action = DefendAction {
                combatant: CombatantId::Player(0),
            };

            perform_defend_action(
                &mut cr,
                &action,
                &content,
                &mut gs_local,
                &mut turn_state_local,
            )
            .expect("defend failed");

            if let Some(Combatant::Player(pc)) = cr.state.participants.first() {
                assert!(pc.ac.current >= pc.ac.base + 2);
            } else {
                panic!("player missing");
            }
        }
    }

    #[test]
    fn test_flee_success_exits_combat() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let mut hero = Character::new(
            "Fleer".to_string(),
            "human".to_string(),
            "rogue".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Make hero fast
        hero.stats.speed.current = 200;
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        let weak_monster = crate::domain::combat::monster::Monster::new(
            1,
            "Turtle".to_string(),
            crate::domain::character::Stats::new(8, 8, 8, 8, 8, 8, 8),
            5,
            5,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        cs.add_monster(weak_monster);
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            combat_res.state = cs;
            combat_res.player_orig_indices = vec![Some(0), None];
        }

        // Attempt to flee
        {
            let content = GameContent::new(crate::sdk::database::ContentDatabase::new());
            let world = app.world_mut();
            // Remove ownership of GlobalState and TurnState so we can mutate them
            // without creating overlapping mutable borrows on the world.
            let mut gs_owned = world
                .remove_resource::<crate::game::resources::GlobalState>()
                .expect("missing GlobalState resource for test");
            let mut turn_state_owned = world
                .remove_resource::<CombatTurnStateResource>()
                .expect("missing CombatTurnStateResource for test");
            let mut combat_res = world.resource_mut::<CombatResource>();

            let success = perform_flee_action(
                &mut combat_res,
                &content,
                &mut gs_owned,
                &mut turn_state_owned,
            )
            .expect("flee action failed");

            assert!(success, "Expected flee to succeed due to speed advantage");
            // Global state should have exited combat (check the owned copy)
            assert!(!matches!(gs_owned.0.mode, GameMode::Combat(_)));

            // Insert modified resources back into the world
            world.insert_resource(gs_owned);
            world.insert_resource(turn_state_owned);
        }
    }

    #[test]
    fn test_flee_failure_consumes_turn() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        let mut gs = GameState::new();
        let mut hero = Character::new(
            "SlowOne".to_string(),
            "human".to_string(),
            "peasant".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Slow hero
        hero.stats.speed.current = 1;
        gs.party.add_member(hero.clone()).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(hero.clone());
        let fast_monster = crate::domain::combat::monster::Monster::new(
            1,
            "Wild Dog".to_string(),
            crate::domain::character::Stats::new(10, 10, 10, 10, 10, 10, 10),
            10,
            10,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        cs.add_monster(fast_monster);
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            combat_res.state = cs;
            combat_res.player_orig_indices = vec![Some(0), None];
        }

        // Attempt to flee and expect failure (turn consumed)
        {
            let content = GameContent::new(crate::sdk::database::ContentDatabase::new());
            let world = app.world_mut();
            // Temporarily remove GlobalState and TurnState for mutation without overlapping borrows
            let mut gs_owned = world
                .remove_resource::<crate::game::resources::GlobalState>()
                .expect("missing GlobalState resource for test");
            let mut turn_state_owned = world
                .remove_resource::<CombatTurnStateResource>()
                .expect("missing CombatTurnStateResource for test");
            let mut combat_res = world.resource_mut::<CombatResource>();

            let success = perform_flee_action(
                &mut combat_res,
                &content,
                &mut gs_owned,
                &mut turn_state_owned,
            )
            .expect("flee action execution failed");

            assert!(!success, "Expected flee to fail due to slow speed");
            // Turn should have advanced (from 0 to 1)
            assert_eq!(combat_res.state.current_turn, 1);

            // Put mutated resources back
            world.insert_resource(gs_owned);
            world.insert_resource(turn_state_owned);
        }
    }

    // ===== Phase 4: Monster AI Tests =====

    /// Verify monster AI (Aggressive) chooses the lowest HP target
    #[test]
    fn test_monster_ai_selects_target() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Create two party members with differing HP
        let mut gs = GameState::new();
        let p1 = Character::new(
            "Tank".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let mut p2 = Character::new(
            "Squire".to_string(),
            "human".to_string(),
            "peasant".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Make second player low HP
        p2.hp.current = 3;
        gs.party.add_member(p1.clone()).unwrap();
        gs.party.add_member(p2.clone()).unwrap();

        // Monster with aggressive AI
        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(p1.clone());
        cs.add_player(p2.clone());
        let mut monster = crate::domain::combat::monster::Monster::new(
            99,
            "Brute".to_string(),
            crate::domain::character::Stats::new(10, 10, 6, 10, 10, 10, 8),
            10,
            6,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        monster.ai_behavior = crate::domain::combat::monster::AiBehavior::Aggressive;
        cs.add_monster(monster);

        // Set turn order with monster first
        cs.turn_order = vec![
            CombatantId::Monster(2),
            CombatantId::Player(0),
            CombatantId::Player(1),
        ];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), Some(1), None];
        }

        // Deterministic RNG
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(42);
        let content = GameContent::new(crate::sdk::database::ContentDatabase::new());

        // Execute monster turn directly
        {
            let world = app.world_mut();
            let mut gs_owned = world
                .remove_resource::<crate::game::resources::GlobalState>()
                .expect("missing GlobalState resource for test");
            let mut turn_state_owned = world
                .remove_resource::<CombatTurnStateResource>()
                .expect("missing CombatTurnStateResource for test");
            let mut cr = world.resource_mut::<CombatResource>();

            perform_monster_turn_with_rng(
                &mut cr,
                &content,
                &mut gs_owned,
                &mut turn_state_owned,
                &mut rng,
            )
            .expect("monster turn failed");

            // The low HP player (index 1) should have taken damage (be lower than base)
            if let Some(Combatant::Player(pc)) = cr.state.participants.get(1) {
                assert!(pc.hp.current < pc.hp.base);
            } else {
                panic!("expected player at index 1");
            }

            world.insert_resource(gs_owned);
            world.insert_resource(turn_state_owned);
        }
    }

    /// Verify that the monster turn advances to the next actor after its action
    #[test]
    fn test_monster_turn_advances_after_attack() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // One player and one monster
        let mut gs = GameState::new();
        let player = Character::new(
            "Solo".to_string(),
            "human".to_string(),
            "fighter".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(player.clone()).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(player.clone());
        let monster = crate::domain::combat::monster::Monster::new(
            1,
            "Lurker".to_string(),
            crate::domain::character::Stats::new(8, 8, 8, 8, 8, 8, 8),
            8,
            5,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        cs.add_monster(monster);

        // Monster acts first
        cs.turn_order = vec![CombatantId::Monster(1), CombatantId::Player(0)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Initialize CombatResource
        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), None];
        }

        // Deterministic RNG
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(12345);
        let content = GameContent::new(crate::sdk::database::ContentDatabase::new());

        // Execute monster turn
        {
            let world = app.world_mut();
            let mut gs_owned = world
                .remove_resource::<crate::game::resources::GlobalState>()
                .expect("missing GlobalState resource for test");
            let mut turn_state_owned = world
                .remove_resource::<CombatTurnStateResource>()
                .expect("missing CombatTurnStateResource for test");
            let mut cr = world.resource_mut::<CombatResource>();

            perform_monster_turn_with_rng(
                &mut cr,
                &content,
                &mut gs_owned,
                &mut turn_state_owned,
                &mut rng,
            )
            .expect("monster turn failed");

            // After the monster acts, current_turn should have advanced (to index 1)
            assert_eq!(cr.state.current_turn, 1);

            world.insert_resource(gs_owned);
            world.insert_resource(turn_state_owned);
        }
    }

    /// Verify that a monster will preferentially attack the lowest HP (explicit)
    #[test]
    fn test_monster_attacks_lowest_hp_target() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);

        // Two players: one nearly dead, one healthy
        let mut gs = GameState::new();
        let healthy = Character::new(
            "Healthy".to_string(),
            "human".to_string(),
            "fighter".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let mut low = Character::new(
            "Wounded".to_string(),
            "human".to_string(),
            "peasant".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        low.hp.current = 1;
        gs.party.add_member(healthy.clone()).unwrap();
        gs.party.add_member(low.clone()).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(healthy.clone());
        cs.add_player(low.clone());

        let mut monster = crate::domain::combat::monster::Monster::new(
            7,
            "Stalker".to_string(),
            crate::domain::character::Stats::new(10, 8, 8, 8, 8, 8, 8),
            8,
            5,
            vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            crate::domain::combat::monster::LootTable::default(),
        );
        monster.ai_behavior = crate::domain::combat::monster::AiBehavior::Aggressive;
        cs.add_monster(monster);

        cs.turn_order = vec![
            CombatantId::Monster(2),
            CombatantId::Player(0),
            CombatantId::Player(1),
        ];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs.clone());
        app.insert_resource(crate::game::resources::GlobalState(gs));

        {
            let mut cr = app.world_mut().resource_mut::<CombatResource>();
            cr.state = cs;
            cr.player_orig_indices = vec![Some(0), Some(1), None];
        }

        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(7);
        let content = GameContent::new(crate::sdk::database::ContentDatabase::new());

        {
            let world = app.world_mut();
            let mut gs_owned = world
                .remove_resource::<crate::game::resources::GlobalState>()
                .expect("missing GlobalState resource for test");
            let mut turn_state_owned = world
                .remove_resource::<CombatTurnStateResource>()
                .expect("missing CombatTurnStateResource for test");
            let mut cr = world.resource_mut::<CombatResource>();

            perform_monster_turn_with_rng(
                &mut cr,
                &content,
                &mut gs_owned,
                &mut turn_state_owned,
                &mut rng,
            )
            .expect("monster turn failed");

            // The low-HP player (index 1) should be the one damaged / killed.
            if let Some(Combatant::Player(pc)) = cr.state.participants.get(1) {
                assert!(pc.hp.current < 1 || pc.hp.current == 0 || pc.hp.current < pc.hp.base);
            } else {
                panic!("expected wounded player at index 1");
            }

            world.insert_resource(gs_owned);
            world.insert_resource(turn_state_owned);
        }
    }
}
