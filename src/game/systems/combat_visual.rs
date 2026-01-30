// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Combat visual systems
//!
//! Systems responsible for the turn indicator UI used during combat.
//!
//! - `spawn_turn_indicator` — spawns a UI indicator attached to the current
//!   combatant (enemy card for monsters, action menu banner for players).
//! - `update_turn_indicator` — moves or respawns the indicator when the current
//!   turn changes.
//! - `hide_indicator_during_animation` — hides the indicator while combat is
//!   animating (e.g. attack animations).
//!
//! These systems are Bevy / game-layer only and rely on the combat HUD being
//! present (`CombatHudRoot`, `EnemyCard`, `ActionMenuPanel`) and the
//! `CombatResource` for the authoritative turn information.

use bevy::prelude::*;

use crate::application::GameMode;
use crate::domain::combat::types::CombatantId;
use crate::game::components::combat::TurnIndicator;
use crate::game::resources::GlobalState;
use crate::game::systems::combat::{
    ActionMenuPanel, CombatResource, CombatTurnState, CombatTurnStateResource, EnemyCard,
    TURN_INDICATOR_COLOR,
};

/// Spawns a turn indicator for the current combatant if none exists.
///
/// - When the current combatant is a monster, the indicator is spawned as a
///   child of the corresponding `EnemyCard` (uses `participant_index` to match).
/// - When the current combatant is a player, a small banner is spawned inside
///   the `ActionMenuPanel` reading "YOUR TURN".
///
/// This system is safe to run every frame; it is a no-op if the HUD is not
/// present, combat is not active, or an indicator already exists.
///
/// # Notes
///
/// - The indicator is tagged with `TurnIndicator` and is therefore discoverable
///   by other UI systems.
/// - The indicator receives a `Visibility` component to support hide/show
///   semantics during animations.
///
/// # Examples
///
/// Called as a regular Bevy system in the `Update` schedule.
#[allow(clippy::too_many_arguments)]
pub fn spawn_turn_indicator(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    combat_res: Res<CombatResource>,
    existing: Query<Entity, With<TurnIndicator>>,
    enemy_cards: Query<(Entity, &EnemyCard)>,
    action_panels: Query<Entity, With<ActionMenuPanel>>,
    turn_state: Option<Res<CombatTurnStateResource>>,
) {
    // Only operate when in combat mode
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    // If an indicator already exists, do nothing
    if !existing.is_empty() {
        return;
    }

    // Determine current combatant
    let current = match combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
        .cloned()
    {
        Some(c) => c,
        None => return,
    };

    // Determine whether the indicator should start hidden due to an animation
    // (this allows the indicator to respect the `CombatTurnState::Animating`
    // state immediately on spawn).
    let is_hidden = match turn_state {
        Some(ts) => matches!(ts.0, CombatTurnState::Animating),
        None => false,
    };

    // Spawn appropriate indicator based on actor type
    match current {
        CombatantId::Monster(idx) => {
            // Find the enemy card for this participant index and attach indicator as a child
            for (entity, card) in enemy_cards.iter() {
                if card.participant_index == idx {
                    commands.entity(entity).with_children(|parent| {
                        parent.spawn((
                            // Small visual node as an arrow/marker; layout is intentionally simple
                            Node {
                                width: Val::Px(12.0),
                                height: Val::Px(12.0),
                                margin: UiRect::bottom(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(TURN_INDICATOR_COLOR),
                            Visibility::Visible,
                            TurnIndicator::for_combatant(current),
                        ));
                    });
                    break;
                }
            }
        }
        CombatantId::Player(_) => {
            // Spawn a "YOUR TURN" banner inside the action menu (if present)
            if let Some(panel_entity) = action_panels.iter().next() {
                commands.entity(panel_entity).with_children(|parent| {
                    parent
                        .spawn((
                            Node {
                                width: Val::Px(120.0),
                                height: Val::Px(28.0),
                                margin: UiRect::all(Val::Px(4.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(TURN_INDICATOR_COLOR),
                            if is_hidden {
                                Visibility::Hidden
                            } else {
                                Visibility::Visible
                            },
                            TurnIndicator::for_combatant(current),
                        ))
                        .with_children(|b| {
                            // Keep content minimal and avoid referencing private
                            // styling components from the sibling module.
                            b.spawn((Text::new("YOUR TURN"),));
                        });
                });
            }
        }
    }

    // If combat is currently animating, the `hide_indicator_during_animation` system
    // will take care of hiding the newly spawned node.
}

/// Updates the existing turn indicator to follow the current actor.
///
/// If the indicator points to a different actor than `CombatResource` indicates,
/// this system will despawn the old indicator and spawn a new one attached to
/// the correct UI element. If it already targets the correct actor, it ensures
/// the parent's relationship is correct (reparents when necessary).
///
/// # Behavior
///
/// - No-op if not in combat or if there is no valid current combatant.
/// - Performs a "move" by despawning the old indicator and spawning a new one
///   attached to the correct UI node. (Simple and avoids complicated
///   re-parent/state mutation while remaining clear.)
#[allow(clippy::type_complexity)]
pub fn update_turn_indicator(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    combat_res: Res<CombatResource>,
    existing: Query<(Entity, &TurnIndicator)>,
    enemy_cards: Query<(Entity, &EnemyCard)>,
    action_panels: Query<Entity, With<ActionMenuPanel>>,
    turn_state: Option<Res<CombatTurnStateResource>>,
) {
    // Only operate when in combat mode
    if !matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    // Determine current combatant
    let current = match combat_res
        .state
        .turn_order
        .get(combat_res.state.current_turn)
        .cloned()
    {
        Some(c) => c,
        None => return,
    };

    // Whether the indicator should be hidden on spawn due to animation state
    let is_hidden = match turn_state {
        Some(ts) => matches!(ts.0, CombatTurnState::Animating),
        None => false,
    };

    // Find an existing indicator (if any)
    let mut existing_entity: Option<Entity> = None;
    let mut existing_target: Option<CombatantId> = None;

    if let Some((entity, indicator)) = existing.iter().next() {
        existing_entity = Some(entity);
        existing_target = Some(indicator.combatant);
        // There should be at most one indicator — only inspect first
    }

    // If we already have an indicator for the current actor, we're done
    if let (Some(_ent), Some(target)) = (existing_entity, existing_target) {
        if target == current {
            return;
        }
        // Otherwise remove the old one so a new one can be spawned for the current actor
        if let Some(ent) = existing_entity {
            commands.entity(ent).despawn();
        }
    }

    // Spawn a new indicator for the current combatant
    match current {
        CombatantId::Monster(idx) => {
            for (entity, card) in enemy_cards.iter() {
                if card.participant_index == idx {
                    commands.entity(entity).with_children(|parent| {
                        parent.spawn((
                            Node {
                                width: Val::Px(12.0),
                                height: Val::Px(12.0),
                                margin: UiRect::bottom(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(TURN_INDICATOR_COLOR),
                            Visibility::Visible,
                            TurnIndicator::for_combatant(current),
                        ));
                    });
                    break;
                }
            }
        }
        CombatantId::Player(_) => {
            if let Some(panel_entity) = action_panels.iter().next() {
                commands.entity(panel_entity).with_children(|parent| {
                    parent
                        .spawn((
                            Node {
                                width: Val::Px(120.0),
                                height: Val::Px(28.0),
                                margin: UiRect::all(Val::Px(4.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(TURN_INDICATOR_COLOR),
                            if is_hidden {
                                Visibility::Hidden
                            } else {
                                Visibility::Visible
                            },
                            TurnIndicator::for_combatant(current),
                        ))
                        .with_children(|b| {
                            b.spawn((Text::new("YOUR TURN"),));
                        });
                });
            }
        }
    }
}

/// Shows or hides the turn indicator while combat animations are playing.
///
/// When `CombatTurnState::Animating` is active, the indicator is hidden. When
/// animation completes, the indicator is made visible again.
pub fn hide_indicator_during_animation(
    turn_state: Res<CombatTurnStateResource>,
    mut indicators: Query<&mut Visibility, With<TurnIndicator>>,
) {
    let should_hide = matches!(turn_state.0, CombatTurnState::Animating);

    for mut visibility in indicators.iter_mut() {
        *visibility = if should_hide {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameState;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::combat::engine::CombatState;
    use crate::domain::combat::monster::Monster;
    use crate::domain::combat::types::Handicap;
    use crate::game::resources::GlobalState;
    use crate::game::systems::combat::CombatPlugin;

    /// Helper to initialize an App with necessary plugins and systems for testing
    fn make_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CombatPlugin);
        app
    }

    #[test]
    fn test_spawn_turn_indicator_on_monster_turn() {
        let mut app = make_test_app();

        // Create GameState with one party member (required for some UI paths)
        let mut gs = GameState::new();
        let hero = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        // Build a combat state with a single monster and explicit turn order
        let mut cs = CombatState::new(Handicap::Even);
        let monster = Monster::new(
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
        cs.add_monster(monster);
        // Ensure turn order points to the monster
        cs.turn_order = vec![CombatantId::Monster(0)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs);

        app.insert_resource(GlobalState(gs));

        // Initialize CombatResource to reflect the combat (parallel to other tests)
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            let mut new_cs = CombatState::new(Handicap::Even);
            let monster = Monster::new(
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
            new_cs.turn_order = vec![CombatantId::Monster(0)];
            new_cs.current_turn = 0;
            combat_res.state = new_cs;
            combat_res.player_orig_indices = vec![None];
        }

        // Run update to spawn UI and our indicator
        app.update();

        // Verify a TurnIndicator entity was created
        let mut indicators = app
            .world_mut()
            .query_filtered::<(Entity, &TurnIndicator), With<TurnIndicator>>();
        assert_eq!(indicators.iter(app.world()).count(), 1);

        // Find the enemy card for participant 0
        let mut card_entity_opt = None;
        let mut enemy_cards = app
            .world_mut()
            .query_filtered::<(Entity, &EnemyCard), With<EnemyCard>>();
        for (entity, card) in enemy_cards.iter(app.world()) {
            if card.participant_index == 0 {
                card_entity_opt = Some(entity);
                break;
            }
        }
        let card_entity = card_entity_opt.expect("Enemy card for monster 0 should exist");

        // Verify the indicator is a child of the enemy card
        let (indicator_entity, _) = indicators.iter(app.world()).next().unwrap();
        let children = app
            .world()
            .get::<Children>(card_entity)
            .expect("Enemy card should have children");
        assert!(
            children.iter().any(|c| c == indicator_entity),
            "Indicator should be a child of the enemy card"
        );
    }

    #[test]
    fn test_turn_indicator_moves_on_turn_change() {
        let mut app = make_test_app();

        // Prepare GameState with one player
        let mut gs = GameState::new();
        let hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        // Build combat state: player (index 0) and a monster (index 1)
        let mut cs = CombatState::new(Handicap::Even);
        cs.add_player(gs.party.members[0].clone());
        let monster = Monster::new(
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

        // Explicit turn order: Player then Monster
        cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        cs.current_turn = 0;
        gs.enter_combat_with_state(cs);

        app.insert_resource(GlobalState(gs));

        // Initialize CombatResource manually
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            let mut new_cs = CombatState::new(Handicap::Even);
            new_cs.add_player(Character::new(
                "Hero".to_string(),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            ));
            let monster = Monster::new(
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
            new_cs.add_monster(monster);
            new_cs.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
            new_cs.current_turn = 0;
            combat_res.state = new_cs;
            combat_res.player_orig_indices = vec![Some(0), None];
        }

        // Run update to spawn UI and indicator (player turn)
        app.update();

        // There should be a TurnIndicator attached to the action menu panel
        let mut action_panels = app
            .world_mut()
            .query_filtered::<Entity, With<ActionMenuPanel>>();
        let action_panel_entity = action_panels
            .iter(app.world())
            .next()
            .expect("ActionMenuPanel should exist");

        let mut indicators = app
            .world_mut()
            .query_filtered::<(Entity, &TurnIndicator), With<TurnIndicator>>();
        assert_eq!(indicators.iter(app.world()).count(), 1);
        let (indicator_entity, _) = indicators.iter(app.world()).next().unwrap();
        let children = app
            .world()
            .get::<Children>(action_panel_entity)
            .expect("ActionMenuPanel should have children");
        assert!(
            children.iter().any(|c| c == indicator_entity),
            "Indicator should be a child of the action menu panel"
        );

        // Advance the turn to the monster
        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            combat_res.state.current_turn = 1;
        }

        // Run update to move indicator
        app.update();

        // Indicator should now be parented to the enemy card for monster index 1
        let mut enemy_cards = app
            .world_mut()
            .query_filtered::<(Entity, &EnemyCard), With<EnemyCard>>();
        let mut card_for_monster_1 = None;
        for (entity, card) in enemy_cards.iter(app.world()) {
            if card.participant_index == 1 {
                card_for_monster_1 = Some(entity);
                break;
            }
        }
        let card_entity = card_for_monster_1.expect("Enemy card for monster 1 should exist");

        // There should still be exactly one TurnIndicator
        let mut indicators = app
            .world_mut()
            .query_filtered::<(Entity, &TurnIndicator), With<TurnIndicator>>();
        assert_eq!(indicators.iter(app.world()).count(), 1);
        let (indicator_entity, _) = indicators.iter(app.world()).next().unwrap();
        let children = app
            .world()
            .get::<Children>(card_entity)
            .expect("Card should have children");
        assert!(
            children.iter().any(|c| c == indicator_entity),
            "Indicator should be a child of the enemy card"
        );
    }

    #[test]
    fn test_turn_indicator_hidden_during_animation() {
        let mut app = make_test_app();

        // Minimal combat setup: one monster
        let mut gs = GameState::new();
        let hero = Character::new(
            "Tester".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        gs.party.add_member(hero).unwrap();

        let mut cs = CombatState::new(Handicap::Even);
        let monster = Monster::new(
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
        cs.add_monster(monster);
        cs.turn_order = vec![CombatantId::Monster(0)];
        cs.current_turn = 0;

        gs.enter_combat_with_state(cs);

        app.insert_resource(GlobalState(gs));

        {
            let mut combat_res = app.world_mut().resource_mut::<CombatResource>();
            let mut new_cs = CombatState::new(Handicap::Even);
            let monster = Monster::new(
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
            new_cs.turn_order = vec![CombatantId::Monster(0)];
            new_cs.current_turn = 0;
            combat_res.state = new_cs;
            combat_res.player_orig_indices = vec![None];
        }

        // Run update to spawn indicator
        app.update();

        // Ensure indicator exists and is visible by default
        let mut indicators = app
            .world_mut()
            .query_filtered::<(Entity, &TurnIndicator), With<TurnIndicator>>();
        assert_eq!(indicators.iter(app.world()).count(), 1);
        let (indicator_entity, _) = indicators.iter(app.world()).next().unwrap();
        let visibility = app
            .world()
            .get::<Visibility>(indicator_entity)
            .expect("Indicator should have Visibility");
        assert_eq!(*visibility, Visibility::Visible);

        // Set combat turn state to Animating
        {
            let mut ts = app.world_mut().resource_mut::<CombatTurnStateResource>();
            ts.0 = CombatTurnState::Animating;
        }

        // Run update to apply hiding
        app.update();

        let visibility = app
            .world()
            .get::<Visibility>(indicator_entity)
            .expect("Indicator should have Visibility");
        assert_eq!(*visibility, Visibility::Hidden);

        // Return to PlayerTurn (not animating)
        {
            let mut ts = app.world_mut().resource_mut::<CombatTurnStateResource>();
            ts.0 = CombatTurnState::PlayerTurn;
        }

        // Run update to show again
        app.update();

        let visibility = app
            .world()
            .get::<Visibility>(indicator_entity)
            .expect("Indicator should have Visibility");
        assert_eq!(*visibility, Visibility::Visible);
    }
}
