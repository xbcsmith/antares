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
use crate::domain::combat::engine::{initialize_combat_from_group, CombatState, Combatant};
use crate::domain::combat::types::{CombatantId, Handicap};
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

/// Player defends this turn (registered by the plugin)
#[derive(Message)]
pub struct DefendAction {
    pub combatant: CombatantId,
}

/// Player attempts to flee (registered by the plugin)
#[derive(Message)]
pub struct FleeAction;

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
}

impl CombatResource {
    /// Create a default/empty combat resource
    pub fn new() -> Self {
        Self {
            state: CombatState::new(Handicap::Even),
            player_orig_indices: Vec::new(),
        }
    }

    /// Clear the combat state and mappings (called after syncing back to party)
    pub fn clear(&mut self) {
        self.state = CombatState::new(Handicap::Even);
        self.player_orig_indices.clear();
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

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CombatStarted>()
            .add_message::<AttackAction>()
            .add_message::<DefendAction>()
            .add_message::<FleeAction>()
            .insert_resource(CombatResource::new())
            .insert_resource(CombatTurnStateResource::default())
            // Handle events that indicate combat started
            .add_systems(Update, handle_combat_started)
            // Ensure party members exist in combat on enter
            .add_systems(Update, sync_party_to_combat)
            // Sync back to party when combat ends
            .add_systems(Update, sync_combat_to_party_on_exit)
            // Phase 2: Combat UI systems
            .add_systems(Update, setup_combat_ui)
            .add_systems(Update, cleanup_combat_ui)
            .add_systems(Update, update_combat_ui);
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
}
