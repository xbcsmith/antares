// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Exploration-mode spell casting Bevy plugin.
//!
//! This module implements the full UI overlay and input handling for casting
//! spells outside of combat.  When the player presses the cast key (`C` by
//! default) during exploration, the game enters
//! [`GameMode::SpellCasting`](crate::application::GameMode::SpellCasting) and
//! this plugin takes over input until the cast completes or is cancelled.
//!
//! # Flow
//!
//! 1. `C` key → `GameState::enter_spell_casting_with_caster_select()` (global
//!    toggle handler in `input/global_toggles.rs`).
//! 2. [`setup_spell_casting_ui`] detects the new mode and spawns the overlay.
//! 3. [`handle_spell_casting_input`] drives the multi-step selection:
//!    - **SelectCaster** — arrow keys + Enter picks the casting character.
//!    - **SelectSpell**  — arrow keys + Enter picks a spell.
//!    - **SelectTarget** — arrow keys + Enter picks a target party member
//!      (only for `SingleCharacter` spells; skipped otherwise).
//!    - **ShowResult**   — Enter or Escape dismisses.
//! 4. [`update_spell_casting_ui`] rebuilds the list panel every frame to
//!    reflect cursor position and step changes.
//! 5. [`cleanup_spell_casting_ui`] despawns the overlay when the mode leaves
//!    `SpellCasting`.
//!
//! # Architecture Reference
//!
//! Phase 3 of `docs/explanation/spell_system_updates_implementation_plan.md`.

use crate::application::resources::GameContent;
use crate::application::spell_casting_state::{SpellCastingState, SpellCastingStep};
use crate::application::GameMode;
use crate::domain::items::ItemDatabase;
use crate::domain::magic::exploration_casting::{
    cast_exploration_spell, get_castable_exploration_spells, ExplorationTarget,
};
use crate::domain::magic::types::SpellTarget;
use crate::game::resources::GlobalState;
use crate::game::systems::ui::{GameLog, LogCategory};
use crate::game::systems::ui_helpers::{BODY_FONT_SIZE, LABEL_FONT_SIZE};
use bevy::prelude::*;

// ── Constants ────────────────────────────────────────────────────────────────

/// Background color of the spell-casting overlay.
pub const OVERLAY_BG: Color = Color::srgba(0.0, 0.0, 0.1, 0.88);
/// Background color of the centre panel.
pub const PANEL_BG: Color = Color::srgba(0.06, 0.06, 0.18, 0.97);
/// Text color for the currently selected row.
pub const SELECTED_ROW_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);
/// Text color for unselected rows.
pub const NORMAL_ROW_COLOR: Color = Color::WHITE;
/// Text color for disabled/uncastable rows.
pub const DISABLED_ROW_COLOR: Color = Color::srgb(0.45, 0.45, 0.45);
/// Text color for the hint line at the bottom.
pub const HINT_COLOR: Color = Color::srgb(0.55, 0.55, 0.65);
/// Text color for the step title.
pub const TITLE_COLOR: Color = Color::srgb(0.8, 0.85, 1.0);
/// Background highlight color for the selected row.
pub const SELECTED_ROW_BG: Color = Color::srgba(0.2, 0.2, 0.05, 0.9);

// ── Components ────────────────────────────────────────────────────────────────

/// Marker component for the root full-screen spell-casting overlay.
///
/// Spawned by [`setup_spell_casting_ui`] and despawned by
/// [`cleanup_spell_casting_ui`].
///
/// # Examples
///
/// ```
/// use antares::game::systems::exploration_spells::SpellCastingOverlay;
///
/// let _: SpellCastingOverlay = SpellCastingOverlay;
/// ```
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellCastingOverlay;

/// Marker component for the scrollable list content panel inside the overlay.
///
/// Despawned and respawned each frame by [`update_spell_casting_ui`] when the
/// current step or cursor position has changed.
///
/// # Examples
///
/// ```
/// use antares::game::systems::exploration_spells::SpellCastingContent;
///
/// let _: SpellCastingContent = SpellCastingContent;
/// ```
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellCastingContent;

/// Marker component for the title text node inside the panel.
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellCastingTitle;

/// Marker component for the hint text at the bottom of the panel.
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellCastingHint;

// ── Plugin ────────────────────────────────────────────────────────────────────

/// Bevy plugin that provides exploration-mode spell casting UI and logic.
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::exploration_spells::ExplorationSpellPlugin;
///
/// # fn setup() {
/// let mut app = App::new();
/// app.add_plugins(ExplorationSpellPlugin);
/// # }
/// ```
pub struct ExplorationSpellPlugin;

impl Plugin for ExplorationSpellPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                setup_spell_casting_ui,
                update_spell_casting_ui,
                handle_spell_casting_input,
                cleanup_spell_casting_ui,
            )
                .chain(),
        );
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Spawns the spell-casting overlay when the game enters `SpellCasting` mode.
///
/// Idempotent: if the overlay already exists the system returns immediately.
/// If the mode is not `SpellCasting` the system also returns immediately.
pub fn setup_spell_casting_ui(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    existing: Query<Entity, With<SpellCastingOverlay>>,
) {
    // Only active in SpellCasting mode.
    if !matches!(global_state.0.mode, GameMode::SpellCasting(_)) {
        return;
    }

    // Only spawn once.
    if !existing.is_empty() {
        return;
    }

    // ── Root full-screen dim overlay ─────────────────────────────────────────
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(OVERLAY_BG),
            SpellCastingOverlay,
        ))
        .with_children(|root| {
            // ── Centre panel ─────────────────────────────────────────────────
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    min_width: Val::Px(380.0),
                    max_width: Val::Px(480.0),
                    min_height: Val::Px(200.0),
                    max_height: Val::Percent(75.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    row_gap: Val::Px(8.0),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|panel| {
                // Title placeholder — updated each frame by update_spell_casting_ui.
                panel.spawn((
                    Text::new("Spell Casting"),
                    TextFont {
                        font_size: BODY_FONT_SIZE,
                        ..default()
                    },
                    TextColor(TITLE_COLOR),
                    SpellCastingTitle,
                ));

                // Content area — children rebuilt each frame.
                panel.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        width: Val::Percent(100.0),
                        ..default()
                    },
                    SpellCastingContent,
                ));

                // Hint line — static.
                panel.spawn((
                    Text::new("↑↓ Navigate   Enter Confirm   Esc Cancel"),
                    TextFont {
                        font_size: LABEL_FONT_SIZE,
                        ..default()
                    },
                    TextColor(HINT_COLOR),
                    SpellCastingHint,
                ));
            });
        });
}

/// Despawns the spell-casting overlay when the game leaves `SpellCasting` mode.
pub fn cleanup_spell_casting_ui(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    existing: Query<Entity, With<SpellCastingOverlay>>,
) {
    if matches!(global_state.0.mode, GameMode::SpellCasting(_)) {
        return;
    }
    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }
}

/// Updates the panel title and list content every frame while in SpellCasting mode.
///
/// Rebuilds the [`SpellCastingContent`] children on every frame to reflect the
/// current cursor position and step.  This is intentionally simple: the list
/// is short (≤6 party members or ≤40 spells) so per-frame rebuilds are cheap.
#[allow(clippy::too_many_arguments)]
pub fn update_spell_casting_ui(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    content: Option<Res<GameContent>>,
    mut title_query: Query<&mut Text, With<SpellCastingTitle>>,
    content_query: Query<Entity, With<SpellCastingContent>>,
    children_query: Query<&Children>,
) {
    let sc = match &global_state.0.mode {
        GameMode::SpellCasting(sc) => sc,
        _ => return,
    };

    // ── Update title ─────────────────────────────────────────────────────────
    let title_str = match sc.step {
        SpellCastingStep::SelectCaster => "Select Caster",
        SpellCastingStep::SelectSpell => "Select Spell",
        SpellCastingStep::SelectTarget => "Select Target",
        SpellCastingStep::ShowResult => "Spell Cast",
    };
    for mut text in title_query.iter_mut() {
        if text.0 != title_str {
            text.0 = title_str.to_string();
        }
    }

    // ── Rebuild content children ──────────────────────────────────────────────
    let Ok(content_entity) = content_query.single() else {
        return;
    };

    // Despawn existing children.
    if let Ok(children) = children_query.get(content_entity) {
        let child_entities: Vec<Entity> = children.iter().collect();
        for child in child_entities {
            commands.entity(child).despawn();
        }
    }

    // Spawn updated children based on current step.
    commands
        .entity(content_entity)
        .with_children(|list| match sc.step {
            SpellCastingStep::SelectCaster => {
                build_caster_rows(list, sc, &global_state);
            }
            SpellCastingStep::SelectSpell => {
                build_spell_rows(list, sc, &global_state, content.as_deref());
            }
            SpellCastingStep::SelectTarget => {
                build_target_rows(list, sc, &global_state);
            }
            SpellCastingStep::ShowResult => {
                build_result_rows(list, sc);
            }
        });
}

/// Handles keyboard input while in `SpellCasting` mode.
///
/// Reads raw keyboard state (arrow keys, Enter, Escape) and drives the
/// multi-step flow by mutating [`GlobalState`].
#[allow(clippy::too_many_arguments)]
pub fn handle_spell_casting_input(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut global_state: ResMut<GlobalState>,
    content: Option<Res<GameContent>>,
    mut game_log: Option<ResMut<GameLog>>,
) {
    if !matches!(global_state.0.mode, GameMode::SpellCasting(_)) {
        return;
    }

    let Some(ref kb) = keyboard else {
        return;
    };

    // ── Escape: cancel ────────────────────────────────────────────────────────
    if kb.just_pressed(KeyCode::Escape) {
        global_state.0.exit_spell_casting();
        return;
    }

    // Collect cursor navigation and confirmation intents.
    let up = kb.just_pressed(KeyCode::ArrowUp) || kb.just_pressed(KeyCode::KeyW);
    let down = kb.just_pressed(KeyCode::ArrowDown) || kb.just_pressed(KeyCode::KeyS);
    let confirm = kb.just_pressed(KeyCode::Enter) || kb.just_pressed(KeyCode::Space);

    // Count items for the current step so cursor wrapping is correct.
    let item_count = count_items_for_step(&global_state.0, content.as_deref());

    let sc = match &mut global_state.0.mode {
        GameMode::SpellCasting(sc) => sc,
        _ => return,
    };

    if up {
        sc.cursor_up(item_count);
        return;
    }
    if down {
        sc.cursor_down(item_count);
        return;
    }
    if !confirm {
        return;
    }

    // ── Confirm action per step ───────────────────────────────────────────────
    let step = sc.step;
    let selected_row = sc.selected_row;

    match step {
        SpellCastingStep::SelectCaster => {
            // Find the `selected_row`-th party member.
            let party_len = {
                if let GameMode::SpellCasting(_) = &global_state.0.mode {
                    global_state.0.party.members.len()
                } else {
                    return;
                }
            };
            if selected_row < party_len {
                if let GameMode::SpellCasting(sc) = &mut global_state.0.mode {
                    sc.caster_index = selected_row;
                    sc.step = SpellCastingStep::SelectSpell;
                    sc.selected_row = 0;
                }
            }
        }

        SpellCastingStep::SelectSpell => {
            // Collect castable spell IDs for this caster.
            let spell_id = {
                let (caster_index, spell_ids) =
                    collect_castable_spell_ids(&global_state.0, content.as_deref());
                if let Some(&id) = spell_ids.get(selected_row) {
                    Some((caster_index, id))
                } else {
                    None
                }
            };

            if let Some((caster_index, spell_id)) = spell_id {
                // Look up the target type.
                let spell_target = content
                    .as_deref()
                    .and_then(|c| c.db().spells.get_spell(spell_id))
                    .map(|s| s.target);

                if let GameMode::SpellCasting(sc) = &mut global_state.0.mode {
                    sc.select_spell(spell_id);
                    sc.caster_index = caster_index;
                }

                match spell_target {
                    Some(SpellTarget::SingleCharacter) => {
                        // Need target selection step.
                        if let GameMode::SpellCasting(sc) = &mut global_state.0.mode {
                            sc.step = SpellCastingStep::SelectTarget;
                            sc.selected_row = 0;
                        }
                    }
                    _ => {
                        // Execute immediately.
                        execute_exploration_cast(
                            &mut global_state,
                            content.as_deref(),
                            &mut game_log,
                        );
                    }
                }
            }
        }

        SpellCastingStep::SelectTarget => {
            // Select a living party member at `selected_row`.
            let target_idx = {
                let members = &global_state.0.party.members;
                members
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| !m.conditions.is_fatal())
                    .nth(selected_row)
                    .map(|(i, _)| i)
            };

            if let Some(target_idx) = target_idx {
                if let GameMode::SpellCasting(sc) = &mut global_state.0.mode {
                    sc.select_target(target_idx);
                }
                execute_exploration_cast(&mut global_state, content.as_deref(), &mut game_log);
            }
        }

        SpellCastingStep::ShowResult => {
            // Dismiss — restore previous mode.
            global_state.0.exit_spell_casting();
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns the number of selectable items for the current step.
///
/// Used by the input system to set correct wrapping bounds for cursor movement.
///
/// # Examples
///
/// ```
/// use antares::application::{GameState, GameMode};
/// use antares::game::systems::exploration_spells::count_items_for_step;
///
/// let mut state = GameState::new();
/// state.enter_spell_casting_with_caster_select();
/// // No party members → 0 items in SelectCaster step.
/// assert_eq!(count_items_for_step(&state, None), 0);
/// ```
pub fn count_items_for_step(
    game_state: &crate::application::GameState,
    content: Option<&GameContent>,
) -> usize {
    let sc = match &game_state.mode {
        GameMode::SpellCasting(sc) => sc,
        _ => return 0,
    };
    match sc.step {
        SpellCastingStep::SelectCaster => game_state.party.members.len(),
        SpellCastingStep::SelectSpell => collect_castable_spell_ids(game_state, content).1.len(),
        SpellCastingStep::SelectTarget => game_state
            .party
            .members
            .iter()
            .filter(|m| !m.conditions.is_fatal())
            .count(),
        SpellCastingStep::ShowResult => 1,
    }
}

/// Collects the list of spell IDs the current caster can cast in exploration.
///
/// Returns `(caster_index, Vec<SpellId>)`.
fn collect_castable_spell_ids(
    game_state: &crate::application::GameState,
    content: Option<&GameContent>,
) -> (usize, Vec<crate::domain::types::SpellId>) {
    let sc = match &game_state.mode {
        GameMode::SpellCasting(sc) => sc,
        _ => return (0, Vec::new()),
    };
    let caster_index = sc.caster_index;
    let Some(caster) = game_state.party.members.get(caster_index) else {
        return (caster_index, Vec::new());
    };
    let Some(content_ref) = content else {
        return (caster_index, Vec::new());
    };
    let spells = get_castable_exploration_spells(caster, &content_ref.db().spells, false);
    let ids: Vec<_> = spells.iter().map(|s| s.id).collect();
    (caster_index, ids)
}

/// Executes the pending exploration spell cast, applies the result to game
/// state, and advances the UI to the `ShowResult` step.
fn execute_exploration_cast(
    global_state: &mut GlobalState,
    content: Option<&GameContent>,
    game_log: &mut Option<ResMut<GameLog>>,
) {
    let (caster_index, spell_id, target) = {
        let sc = match &global_state.0.mode {
            GameMode::SpellCasting(sc) => sc,
            _ => return,
        };

        let spell_id = match sc.selected_spell_id {
            Some(id) => id,
            None => return,
        };

        let spell_target_type = content
            .and_then(|c| c.db().spells.get_spell(spell_id))
            .map(|s| s.target);

        let target = match spell_target_type {
            Some(SpellTarget::Self_) => ExplorationTarget::Self_,
            Some(SpellTarget::AllCharacters) => ExplorationTarget::AllCharacters,
            Some(SpellTarget::SingleCharacter) => {
                match sc.target_index {
                    Some(idx) => ExplorationTarget::Character(idx),
                    None => ExplorationTarget::Self_, // fallback
                }
            }
            _ => ExplorationTarget::Self_,
        };

        (sc.caster_index, spell_id, target)
    };

    // Look up the spell definition.
    let spell = match content.and_then(|c| c.db().spells.get_spell(spell_id)) {
        Some(s) => s.clone(),
        None => {
            if let GameMode::SpellCasting(sc) = &mut global_state.0.mode {
                sc.show_result("Spell not found.".to_string());
            }
            return;
        }
    };

    // Get the item database (for Create Food side effects).
    let owned_item_db: ItemDatabase;
    let item_db: &ItemDatabase = if let Some(c) = content {
        &c.db().items
    } else {
        owned_item_db = ItemDatabase::new();
        &owned_item_db
    };

    let mut rng = rand::rng();
    let result = cast_exploration_spell(
        caster_index,
        &spell,
        target,
        &mut global_state.0,
        item_db,
        &mut rng,
    );

    let message = match result {
        Ok(ref r) => {
            if r.total_hp_healed > 0 {
                format!("{} restores {} HP.", spell.name, r.total_hp_healed)
            } else if r.food_created > 0 {
                format!("{} creates {} food rations.", spell.name, r.food_created)
            } else if r.buff_applied.is_some() {
                format!("{} takes effect.", spell.name)
            } else if r.condition_cured.is_some() {
                format!("{} cures the condition.", spell.name)
            } else {
                r.message.clone()
            }
        }
        Err(ref e) => format!("{} failed: {e}.", spell.name),
    };

    // Log the cast result.
    if let Some(ref mut log) = game_log {
        log.add_entry(message.clone(), LogCategory::Exploration);
    }

    // Advance to ShowResult.
    if let GameMode::SpellCasting(sc) = &mut global_state.0.mode {
        sc.show_result(message);
    }
}

// ── UI row builders ───────────────────────────────────────────────────────────

/// Builds the party member rows for the `SelectCaster` step.
fn build_caster_rows(
    list: &mut ChildSpawnerCommands<'_>,
    sc: &SpellCastingState,
    global_state: &GlobalState,
) {
    let members = &global_state.0.party.members;
    if members.is_empty() {
        list.spawn((
            Text::new("No party members."),
            TextFont {
                font_size: BODY_FONT_SIZE,
                ..default()
            },
            TextColor(DISABLED_ROW_COLOR),
        ));
        return;
    }
    for (i, member) in members.iter().enumerate() {
        let selected = i == sc.selected_row;
        let sp_text = format!(
            "{} [SP {}/{}]",
            member.name, member.sp.current, member.sp.base
        );
        spawn_row(list, &sp_text, selected, false);
    }
}

/// Builds the spell list rows for the `SelectSpell` step.
fn build_spell_rows(
    list: &mut ChildSpawnerCommands<'_>,
    sc: &SpellCastingState,
    global_state: &GlobalState,
    content: Option<&GameContent>,
) {
    let Some(caster) = global_state.0.party.members.get(sc.caster_index) else {
        list.spawn((
            Text::new("No caster selected."),
            TextFont {
                font_size: BODY_FONT_SIZE,
                ..default()
            },
            TextColor(DISABLED_ROW_COLOR),
        ));
        return;
    };

    let Some(content_ref) = content else {
        list.spawn((
            Text::new("Content not available."),
            TextFont {
                font_size: BODY_FONT_SIZE,
                ..default()
            },
            TextColor(DISABLED_ROW_COLOR),
        ));
        return;
    };

    let spells = get_castable_exploration_spells(caster, &content_ref.db().spells, false);

    if spells.is_empty() {
        list.spawn((
            Text::new("No castable spells."),
            TextFont {
                font_size: BODY_FONT_SIZE,
                ..default()
            },
            TextColor(DISABLED_ROW_COLOR),
        ));
        return;
    }

    for (i, spell) in spells.iter().enumerate() {
        let selected = i == sc.selected_row;
        let label = if spell.gem_cost > 0 {
            format!(
                "L{} {} — {} SP {} Gems",
                spell.level, spell.name, spell.sp_cost, spell.gem_cost
            )
        } else {
            format!("L{} {} — {} SP", spell.level, spell.name, spell.sp_cost)
        };
        spawn_row(list, &label, selected, false);
    }
}

/// Builds the party member rows for the `SelectTarget` step.
fn build_target_rows(
    list: &mut ChildSpawnerCommands<'_>,
    sc: &SpellCastingState,
    global_state: &GlobalState,
) {
    let living: Vec<_> = global_state
        .0
        .party
        .members
        .iter()
        .filter(|m| !m.conditions.is_fatal())
        .collect();

    if living.is_empty() {
        list.spawn((
            Text::new("No valid targets."),
            TextFont {
                font_size: BODY_FONT_SIZE,
                ..default()
            },
            TextColor(DISABLED_ROW_COLOR),
        ));
        return;
    }

    for (i, member) in living.iter().enumerate() {
        let selected = i == sc.selected_row;
        let hp_text = format!(
            "{} [HP {}/{}]",
            member.name, member.hp.current, member.hp.base
        );
        spawn_row(list, &hp_text, selected, false);
    }
}

/// Builds the result display rows for the `ShowResult` step.
fn build_result_rows(list: &mut ChildSpawnerCommands<'_>, sc: &SpellCastingState) {
    let msg = sc
        .feedback_message
        .as_deref()
        .unwrap_or("The spell was cast.");
    list.spawn((
        Text::new(msg.to_string()),
        TextFont {
            font_size: BODY_FONT_SIZE,
            ..default()
        },
        TextColor(Color::srgb(0.8, 1.0, 0.8)),
    ));
    list.spawn((
        Text::new("Press Enter or Esc to continue."),
        TextFont {
            font_size: LABEL_FONT_SIZE,
            ..default()
        },
        TextColor(HINT_COLOR),
    ));
}

/// Spawns a single selectable row in the list.
fn spawn_row(list: &mut ChildSpawnerCommands<'_>, label: &str, selected: bool, disabled: bool) {
    let text_color = if disabled {
        DISABLED_ROW_COLOR
    } else if selected {
        SELECTED_ROW_COLOR
    } else {
        NORMAL_ROW_COLOR
    };

    let bg = if selected {
        SELECTED_ROW_BG
    } else {
        Color::NONE
    };

    list.spawn((
        Node {
            width: Val::Percent(100.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(bg),
        BorderRadius::all(Val::Px(4.0)),
    ))
    .with_children(|row| {
        row.spawn((
            Text::new(label.to_string()),
            TextFont {
                font_size: BODY_FONT_SIZE,
                ..default()
            },
            TextColor(text_color),
        ));
    });
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::spell_casting_state::SpellCastingStep;
    use crate::application::GameState;

    #[test]
    fn test_spell_casting_overlay_is_marker_component() {
        // SpellCastingOverlay is a zero-sized marker — just verify it compiles
        // and is Clone + Copy.
        let _a: SpellCastingOverlay = SpellCastingOverlay;
        let _b = _a; // Copy
    }

    #[test]
    fn test_spell_casting_content_is_marker_component() {
        let _a: SpellCastingContent = SpellCastingContent;
        let _b = _a;
    }

    #[test]
    fn test_count_items_for_step_exploration_mode_returns_zero() {
        let state = GameState::new();
        assert_eq!(count_items_for_step(&state, None), 0);
    }

    #[test]
    fn test_count_items_for_step_select_caster_counts_party_members() {
        use crate::domain::character::{Alignment, Character, Sex};

        let mut state = GameState::new();
        for i in 0..3u32 {
            let c = Character::new(
                format!("Hero{i}"),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            state.party.members.push(c);
        }
        state.enter_spell_casting_with_caster_select();
        assert_eq!(count_items_for_step(&state, None), 3);
    }

    #[test]
    fn test_count_items_for_step_select_target_counts_living_members() {
        use crate::domain::character::{Alignment, Character, Condition, Sex};

        let mut state = GameState::new();
        let mut dead = Character::new(
            "Dead".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        dead.conditions.add(Condition::DEAD);

        let alive = Character::new(
            "Alive".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        state.party.members.push(dead);
        state.party.members.push(alive);
        state.enter_spell_casting_with_caster_select();

        // Manually set to SelectTarget step.
        if let GameMode::SpellCasting(sc) = &mut state.mode {
            sc.step = SpellCastingStep::SelectTarget;
        }

        // Only 1 living member.
        assert_eq!(count_items_for_step(&state, None), 1);
    }

    #[test]
    fn test_count_items_for_step_show_result_returns_one() {
        let mut state = GameState::new();
        state.enter_spell_casting(0);
        if let GameMode::SpellCasting(sc) = &mut state.mode {
            sc.show_result("Done".to_string());
        }
        assert_eq!(count_items_for_step(&state, None), 1);
    }

    #[test]
    fn test_collect_castable_spell_ids_no_content_returns_empty() {
        let mut state = GameState::new();
        use crate::domain::character::{Alignment, Character, Sex};
        let caster = Character::new(
            "Mage".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        state.party.members.push(caster);
        state.enter_spell_casting(0);

        let (_, ids) = collect_castable_spell_ids(&state, None);
        assert!(ids.is_empty(), "No content → empty spell list");
    }

    #[test]
    fn test_enter_and_exit_spell_casting_roundtrip() {
        let mut state = GameState::new();
        assert!(matches!(state.mode, GameMode::Exploration));

        state.enter_spell_casting(0);
        assert!(matches!(state.mode, GameMode::SpellCasting(_)));

        state.exit_spell_casting();
        assert!(matches!(state.mode, GameMode::Exploration));
    }

    #[test]
    fn test_enter_spell_casting_with_caster_select_sets_correct_step() {
        let mut state = GameState::new();
        state.enter_spell_casting_with_caster_select();
        if let GameMode::SpellCasting(sc) = &state.mode {
            assert_eq!(sc.step, SpellCastingStep::SelectCaster);
        } else {
            panic!("expected SpellCasting mode");
        }
    }

    #[test]
    fn test_exploration_target_from_self_spell() {
        use crate::domain::magic::types::SpellTarget;
        let t = ExplorationTarget::from_spell_target(SpellTarget::Self_, 0);
        assert_eq!(t, Some(ExplorationTarget::Self_));
    }

    #[test]
    fn test_exploration_target_from_all_characters_spell() {
        use crate::domain::magic::types::SpellTarget;
        let t = ExplorationTarget::from_spell_target(SpellTarget::AllCharacters, 0);
        assert_eq!(t, Some(ExplorationTarget::AllCharacters));
    }

    #[test]
    fn test_exploration_target_monster_returns_none() {
        use crate::domain::magic::types::SpellTarget;
        for tgt in [
            SpellTarget::SingleMonster,
            SpellTarget::MonsterGroup,
            SpellTarget::AllMonsters,
            SpellTarget::SpecificMonsters,
        ] {
            assert_eq!(
                ExplorationTarget::from_spell_target(tgt, 0),
                None,
                "{tgt:?} must not map to an ExplorationTarget"
            );
        }
    }
}
