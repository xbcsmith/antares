// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! HUD (Heads-Up Display) system for party status visualization
//!
//! This module provides a native Bevy UI-based HUD that displays:
//! - Character names with party position numbers
//! - HP bars with color-coded health states
//! - Exact HP values (current/max format)
//! - Active condition indicators with emoji and color coding
//!
//! The HUD uses a horizontal strip layout at the bottom of the screen,
//! supporting up to 6 party members in individual character cards.

use crate::domain::character::{Condition, PARTY_MAX_SIZE};
use crate::domain::conditions::ActiveCondition;
use crate::domain::types::Direction;
use crate::game::resources::GlobalState;
use bevy::prelude::*;

// ===== Constants =====

// HP bar colors
pub const HP_HEALTHY_COLOR: Color = Color::srgb(0.39, 0.78, 0.39);
pub const HP_INJURED_COLOR: Color = Color::srgb(0.90, 0.71, 0.20);
pub const HP_CRITICAL_COLOR: Color = Color::srgb(0.86, 0.20, 0.20);
pub const HP_DEAD_COLOR: Color = Color::srgb(0.31, 0.31, 0.31);

// Condition colors
pub const CONDITION_POISONED_COLOR: Color = Color::srgb(0.20, 0.71, 0.20);
pub const CONDITION_PARALYZED_COLOR: Color = Color::srgb(0.39, 0.39, 0.78);
pub const CONDITION_BUFFED_COLOR: Color = Color::srgb(0.78, 0.71, 0.39);

// HP thresholds
pub const HP_HEALTHY_THRESHOLD: f32 = 0.75;
pub const HP_CRITICAL_THRESHOLD: f32 = 0.25;

// Layout constants
pub const HUD_PANEL_HEIGHT: Val = Val::Px(80.0);
pub const CHARACTER_CARD_WIDTH: Val = Val::Px(120.0);
pub const HP_BAR_HEIGHT: Val = Val::Px(16.0);
pub const CARD_PADDING: Val = Val::Px(8.0);

// Condition priority values (higher = more severe)
pub const PRIORITY_DEAD: u8 = 100;
pub const PRIORITY_UNCONSCIOUS: u8 = 90;
pub const PRIORITY_PARALYZED: u8 = 80;
pub const PRIORITY_POISONED: u8 = 70;
pub const PRIORITY_DISEASED: u8 = 60;
pub const PRIORITY_BLINDED: u8 = 50;
pub const PRIORITY_SILENCED: u8 = 40;
pub const PRIORITY_ASLEEP: u8 = 30;
pub const PRIORITY_BUFFED: u8 = 10;
pub const PRIORITY_FINE: u8 = 0;

// Compass display constants
pub const COMPASS_SIZE: f32 = 48.0;
pub const COMPASS_BORDER_WIDTH: f32 = 2.0;
pub const COMPASS_BACKGROUND_COLOR: Color = Color::srgba(0.1, 0.1, 0.1, 0.9);
pub const COMPASS_BORDER_COLOR: Color = Color::srgba(0.4, 0.4, 0.4, 1.0);
pub const COMPASS_TEXT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);
pub const COMPASS_FONT_SIZE: f32 = 24.0;

// Portrait display constants
pub const PORTRAIT_SIZE: f32 = 40.0;
pub const PORTRAIT_MARGIN: Val = Val::Px(4.0);
pub const PORTRAIT_PLACEHOLDER_COLOR: Color = Color::srgba(0.3, 0.3, 0.4, 1.0);

// ===== Marker Components =====

/// Marker component for the HUD root container
#[derive(Component)]
pub struct HudRoot;

/// Marker component for a character card in the HUD
#[derive(Component)]
pub struct CharacterCard {
    pub party_index: usize,
}

/// Marker component for HP bar background
#[derive(Component)]
pub struct HpBarBackground;

/// Marker component for HP bar fill (the colored portion)
#[derive(Component)]
pub struct HpBarFill {
    pub party_index: usize,
}

/// Marker component for HP text label
#[derive(Component)]
pub struct HpText {
    pub party_index: usize,
}

/// Marker component for condition text label
#[derive(Component)]
pub struct ConditionText {
    pub party_index: usize,
}

/// Marker component for character name label
#[derive(Component)]
pub struct CharacterNameText {
    pub party_index: usize,
}

/// Marker component for the compass container
#[derive(Component)]
pub struct CompassRoot;

/// Marker component for the compass direction text
#[derive(Component)]
pub struct CompassText;

/// Marker component for character portrait image
#[derive(Component)]
pub struct CharacterPortrait {
    pub party_index: usize,
}

// ===== Plugin =====

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hud)
            .add_systems(Update, (update_hud, update_compass, update_portraits));
    }
}

// ===== Systems =====

/// Sets up the HUD UI hierarchy (runs once at startup)
///
/// Creates the HUD container and character card slots using Bevy's
/// native UI system with flexbox layout.
///
/// # Arguments
/// * `commands` - Bevy command buffer for spawning entities
fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                height: HUD_PANEL_HEIGHT,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
            HudRoot,
        ))
        .with_children(|parent| {
            for party_index in 0..PARTY_MAX_SIZE {
                // Spawn character card inline due to Bevy's with_children closure type complexity
                parent
                    .spawn((
                        Node {
                            width: CHARACTER_CARD_WIDTH,
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(CARD_PADDING),
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
                        BorderRadius::all(Val::Px(4.0)),
                        CharacterCard { party_index },
                    ))
                    .with_children(|card| {
                        // Portrait placeholder (colored rectangle)
                        card.spawn((
                            Node {
                                width: Val::Px(PORTRAIT_SIZE),
                                height: Val::Px(PORTRAIT_SIZE),
                                margin: UiRect::all(PORTRAIT_MARGIN),
                                ..default()
                            },
                            BackgroundColor(PORTRAIT_PLACEHOLDER_COLOR),
                            BorderRadius::all(Val::Px(4.0)),
                            CharacterPortrait { party_index },
                        ));

                        // Character name text
                        card.spawn((
                            Text::new(""),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            CharacterNameText { party_index },
                        ));

                        // HP bar container (background)
                        card.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: HP_BAR_HEIGHT,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 1.0)),
                            HpBarBackground,
                        ))
                        .with_children(|bar| {
                            // HP bar fill (the colored part that changes width)
                            bar.spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(HP_HEALTHY_COLOR),
                                HpBarFill { party_index },
                            ));
                        });

                        // HP text ("45/100 HP")
                        card.spawn((
                            Text::new(""),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            HpText { party_index },
                        ));

                        // Condition text ("☠️ Poisoned")
                        card.spawn((
                            Text::new(""),
                            TextFont {
                                font_size: 10.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            ConditionText { party_index },
                        ));
                    });
            }
        });

    // Spawn compass display
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                top: Val::Px(20.0),
                width: Val::Px(COMPASS_SIZE),
                height: Val::Px(COMPASS_SIZE),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(COMPASS_BACKGROUND_COLOR),
            CompassRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("N"),
                TextFont {
                    font_size: COMPASS_FONT_SIZE,
                    ..default()
                },
                TextColor(COMPASS_TEXT_COLOR),
                CompassText,
            ));
        });
}

/// Updates HUD elements based on current party state
///
/// This system runs every frame to sync UI with game state.
/// Updates HP bars, HP text, condition text, and character names.
///
/// # Arguments
/// * `global_state` - Game state containing party data
/// * `hp_bar_query` - Query for HP bar fill entities
/// * `hp_text_query` - Query for HP text entities
/// * `condition_text_query` - Query for condition text entities
/// * `name_text_query` - Query for character name text entities
#[allow(clippy::type_complexity)]
fn update_hud(
    global_state: Res<GlobalState>,
    mut hp_bar_query: Query<(&HpBarFill, &mut Node, &mut BackgroundColor)>,
    mut hp_text_query: Query<(&HpText, &mut Text), Without<ConditionText>>,
    mut condition_text_query: Query<(&ConditionText, &mut Text, &mut TextColor), Without<HpText>>,
    mut name_text_query: Query<
        (&CharacterNameText, &mut Text),
        (Without<HpText>, Without<ConditionText>),
    >,
) {
    let party = &global_state.0.party;

    // Update HP bars
    for (hp_bar, mut node, mut bg_color) in hp_bar_query.iter_mut() {
        if let Some(character) = party.members.get(hp_bar.party_index) {
            let hp_percent = character.hp.current as f32 / character.hp.base as f32;
            node.width = Val::Percent(hp_percent * 100.0);
            *bg_color = BackgroundColor(hp_bar_color(hp_percent));
        } else {
            // No character in this slot - hide bar
            node.width = Val::Px(0.0);
        }
    }

    // Update HP text
    for (hp_text, mut text) in hp_text_query.iter_mut() {
        if let Some(character) = party.members.get(hp_text.party_index) {
            **text = format_hp_display(character.hp.current, character.hp.base);
        } else {
            **text = String::new();
        }
    }

    // Update condition text
    for (condition_text, mut text, mut text_color) in condition_text_query.iter_mut() {
        if let Some(character) = party.members.get(condition_text.party_index) {
            let (cond_str, color) =
                get_priority_condition(&character.conditions, &character.active_conditions);
            let count = count_conditions(&character.conditions);

            // If multiple conditions, append count
            let display_text = if count > 1 {
                format!("{} +{}", cond_str, count - 1)
            } else {
                cond_str
            };

            **text = display_text;
            *text_color = TextColor(color);
        } else {
            **text = String::new();
        }
    }

    // Update character names
    for (name_text, mut text) in name_text_query.iter_mut() {
        if let Some(character) = party.members.get(name_text.party_index) {
            **text = format!("{}. {}", name_text.party_index + 1, character.name);
        } else {
            **text = String::new();
        }
    }
}

/// Updates compass direction display
///
/// Queries the World resource to get current party_facing and updates
/// the compass text to show N/E/S/W
///
/// # Arguments
/// * `global_state` - Game state containing world data
/// * `compass_query` - Query for compass text entity
fn update_compass(
    global_state: Res<GlobalState>,
    mut compass_query: Query<&mut Text, With<CompassText>>,
) {
    if let Ok(mut text) = compass_query.single_mut() {
        **text = direction_to_string(&global_state.0.world.party_facing);
    }
}

/// Updates portrait colors based on character portrait_id
///
/// Sets portrait placeholder background color using deterministic
/// color generation from portrait_id field.
///
/// # Arguments
/// * `global_state` - Game state containing party data
/// * `portrait_query` - Query for portrait background entities
fn update_portraits(
    global_state: Res<GlobalState>,
    mut portrait_query: Query<(&CharacterPortrait, &mut BackgroundColor)>,
) {
    let party = &global_state.0.party;

    for (portrait, mut bg_color) in portrait_query.iter_mut() {
        if let Some(character) = party.members.get(portrait.party_index) {
            *bg_color = BackgroundColor(get_portrait_color(character.portrait_id));
        } else {
            // No character in this slot - use default placeholder color
            *bg_color = BackgroundColor(PORTRAIT_PLACEHOLDER_COLOR);
        }
    }
}

// ===== Helper Functions =====

/// Returns HP bar color based on health percentage
///
/// Uses threshold constants to determine color.
///
/// # Arguments
/// * `hp_percent` - Current HP as percentage (0.0 to 1.0)
///
/// # Returns
/// Bevy Color for the HP bar
///
/// # Examples
///
/// ```
/// use antares::game::systems::hud::hp_bar_color;
/// use bevy::prelude::Color;
///
/// let color = hp_bar_color(0.80);
/// // Returns HP_HEALTHY_COLOR (green)
/// ```
pub fn hp_bar_color(hp_percent: f32) -> Color {
    if hp_percent >= HP_HEALTHY_THRESHOLD {
        HP_HEALTHY_COLOR
    } else if hp_percent >= HP_CRITICAL_THRESHOLD {
        HP_INJURED_COLOR
    } else {
        HP_CRITICAL_COLOR
    }
}

/// Formats HP display as "current/max HP"
///
/// # Arguments
/// * `current` - Current HP value
/// * `max` - Maximum HP value
///
/// # Returns
/// Formatted string like "45/100 HP"
///
/// # Examples
///
/// ```
/// use antares::game::systems::hud::format_hp_display;
///
/// let display = format_hp_display(45, 100);
/// assert_eq!(display, "45/100 HP");
/// ```
pub fn format_hp_display(current: u16, max: u16) -> String {
    format!("{}/{} HP", current, max)
}

/// Returns the highest priority condition for display
///
/// Priority order (highest to lowest):
/// - DEAD/STONE/ERADICATED (100)
/// - UNCONSCIOUS (90)
/// - PARALYZED (80)
/// - POISONED (70)
/// - DISEASED (60)
/// - BLINDED (50)
/// - SILENCED (40)
/// - ASLEEP (30)
/// - Active buffs (10)
/// - FINE (0)
///
/// # Arguments
/// * `conditions` - Character's condition bitflags
/// * `active_conditions` - List of active condition effects
///
/// # Returns
/// Tuple of (condition_text, condition_color)
///
/// # Examples
///
/// ```
/// use antares::domain::character::Condition;
/// use antares::game::systems::hud::get_priority_condition;
/// use bevy::prelude::Color;
///
/// let mut conditions = Condition::new();
/// conditions.add(Condition::DEAD);
/// let (text, color) = get_priority_condition(&conditions, &[]);
/// assert!(text.contains("Dead"));
/// ```
pub fn get_priority_condition(
    conditions: &Condition,
    active_conditions: &[ActiveCondition],
) -> (String, Color) {
    // Check for fatal conditions (DEAD, STONE, ERADICATED all have DEAD bit set)
    if conditions.is_fatal() {
        return ("💀 Dead".to_string(), HP_DEAD_COLOR);
    }
    if conditions.has(Condition::UNCONSCIOUS) {
        return ("💤 Unconscious".to_string(), CONDITION_PARALYZED_COLOR);
    }
    if conditions.has(Condition::PARALYZED) {
        return ("⚡ Paralyzed".to_string(), CONDITION_PARALYZED_COLOR);
    }
    if conditions.has(Condition::POISONED) {
        return ("☠️ Poisoned".to_string(), CONDITION_POISONED_COLOR);
    }
    if conditions.has(Condition::DISEASED) {
        return ("🤢 Diseased".to_string(), CONDITION_POISONED_COLOR);
    }
    if conditions.has(Condition::BLINDED) {
        return ("👁️ Blind".to_string(), HP_INJURED_COLOR);
    }
    if conditions.has(Condition::SILENCED) {
        return ("🔇 Silenced".to_string(), HP_INJURED_COLOR);
    }
    if conditions.has(Condition::ASLEEP) {
        return ("😴 Asleep".to_string(), CONDITION_PARALYZED_COLOR);
    }
    if !active_conditions.is_empty() {
        return ("✨ Buffed".to_string(), CONDITION_BUFFED_COLOR);
    }
    ("✓ OK".to_string(), HP_HEALTHY_COLOR)
}

/// Returns numeric priority for a condition
///
/// Higher values = more severe conditions displayed first
///
/// # Arguments
/// * `conditions` - Character's condition bitflags
///
/// # Returns
/// Priority value (0-100)
///
/// # Examples
///
/// ```
/// use antares::domain::character::Condition;
/// use antares::game::systems::hud::{get_condition_priority, PRIORITY_DEAD};
///
/// let mut conditions = Condition::new();
/// conditions.add(Condition::DEAD);
/// assert_eq!(get_condition_priority(&conditions), PRIORITY_DEAD);
/// ```
pub fn get_condition_priority(conditions: &Condition) -> u8 {
    // Use is_fatal() to detect DEAD/STONE/ERADICATED correctly
    if conditions.is_fatal() {
        return PRIORITY_DEAD;
    }
    if conditions.has(Condition::UNCONSCIOUS) {
        return PRIORITY_UNCONSCIOUS;
    }
    if conditions.has(Condition::PARALYZED) {
        return PRIORITY_PARALYZED;
    }
    if conditions.has(Condition::POISONED) {
        return PRIORITY_POISONED;
    }
    if conditions.has(Condition::DISEASED) {
        return PRIORITY_DISEASED;
    }
    if conditions.has(Condition::BLINDED) {
        return PRIORITY_BLINDED;
    }
    if conditions.has(Condition::SILENCED) {
        return PRIORITY_SILENCED;
    }
    if conditions.has(Condition::ASLEEP) {
        return PRIORITY_ASLEEP;
    }
    PRIORITY_FINE
}

/// Counts number of active negative conditions
///
/// Used to display "+N conditions" when multiple exist
///
/// # Arguments
/// * `conditions` - Character's condition bitflags
///
/// # Returns
/// Count of active conditions (0-8)
///
/// # Examples
///
/// ```
/// use antares::domain::character::Condition;
/// use antares::game::systems::hud::count_conditions;
///
/// let mut conditions = Condition::new();
/// conditions.add(Condition::POISONED);
/// conditions.add(Condition::BLINDED);
/// assert_eq!(count_conditions(&conditions), 2);
/// ```
pub fn count_conditions(conditions: &Condition) -> u8 {
    let mut count = 0;
    if conditions.has(Condition::ASLEEP) {
        count += 1;
    }
    if conditions.has(Condition::BLINDED) {
        count += 1;
    }
    if conditions.has(Condition::SILENCED) {
        count += 1;
    }
    if conditions.has(Condition::DISEASED) {
        count += 1;
    }
    if conditions.has(Condition::POISONED) {
        count += 1;
    }
    if conditions.has(Condition::PARALYZED) {
        count += 1;
    }
    if conditions.has(Condition::UNCONSCIOUS) {
        count += 1;
    }
    if conditions.has(Condition::DEAD) {
        count += 1;
    }
    count
}

/// Converts Direction enum to display string
///
/// # Arguments
/// * `direction` - The cardinal direction from World state
///
/// # Returns
/// Single character string: "N", "E", "S", or "W"
///
/// # Examples
///
/// ```
/// use antares::domain::types::Direction;
/// use antares::game::systems::hud::direction_to_string;
///
/// assert_eq!(direction_to_string(&Direction::North), "N");
/// assert_eq!(direction_to_string(&Direction::East), "E");
/// ```
pub fn direction_to_string(direction: &Direction) -> String {
    match direction {
        Direction::North => "N".to_string(),
        Direction::East => "E".to_string(),
        Direction::South => "S".to_string(),
        Direction::West => "W".to_string(),
    }
}

/// Returns portrait color based on character portrait_id
///
/// Generates a deterministic color from portrait_id for placeholder display.
/// Each portrait_id maps to a unique color for visual distinction.
///
/// # Arguments
/// * `portrait_id` - Character's portrait identifier (0-255)
///
/// # Returns
/// Bevy Color for the portrait placeholder
///
/// # Examples
///
/// ```
/// use antares::game::systems::hud::get_portrait_color;
/// use bevy::prelude::Color;
///
/// let color = get_portrait_color(0);
/// // Returns a deterministic color based on ID
/// ```
pub fn get_portrait_color(portrait_id: u8) -> Color {
    // Generate deterministic colors based on portrait_id
    // Uses HSL-like distribution for visual variety
    let hue = (portrait_id as f32 * 137.5) % 360.0; // Golden angle distribution
    let saturation = 0.6;
    let lightness = 0.5;

    // Convert HSL to RGB (simplified)
    let c = (1.0_f32 - (2.0_f32 * lightness - 1.0_f32).abs()) * saturation;
    let x = c * (1.0_f32 - ((hue / 60.0_f32) % 2.0_f32 - 1.0_f32).abs());
    let m = lightness - c / 2.0_f32;

    let (r, g, b) = if hue < 60.0 {
        (c, x, 0.0)
    } else if hue < 120.0 {
        (x, c, 0.0)
    } else if hue < 180.0 {
        (0.0, c, x)
    } else if hue < 240.0 {
        (0.0, x, c)
    } else if hue < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Color::srgb(r + m, g + m, b + m)
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to compare colors (Bevy Color may have floating point precision differences)
    fn colors_approx_equal(a: Color, b: Color) -> bool {
        let a_rgba = a.to_srgba();
        let b_rgba = b.to_srgba();
        (a_rgba.red - b_rgba.red).abs() < 0.01
            && (a_rgba.green - b_rgba.green).abs() < 0.01
            && (a_rgba.blue - b_rgba.blue).abs() < 0.01
    }

    #[test]
    fn test_hp_bar_color_healthy() {
        let color = hp_bar_color(0.80);
        assert!(colors_approx_equal(color, HP_HEALTHY_COLOR));
    }

    #[test]
    fn test_hp_bar_color_injured() {
        let color = hp_bar_color(0.50);
        assert!(colors_approx_equal(color, HP_INJURED_COLOR));
    }

    #[test]
    fn test_hp_bar_color_critical() {
        let color = hp_bar_color(0.15);
        assert!(colors_approx_equal(color, HP_CRITICAL_COLOR));
    }

    #[test]
    fn test_hp_bar_color_boundary_healthy() {
        let color = hp_bar_color(HP_HEALTHY_THRESHOLD);
        assert!(colors_approx_equal(color, HP_HEALTHY_COLOR));
    }

    #[test]
    fn test_hp_bar_color_boundary_critical() {
        let color = hp_bar_color(HP_CRITICAL_THRESHOLD);
        assert!(colors_approx_equal(color, HP_INJURED_COLOR));
    }

    #[test]
    fn test_format_hp_display() {
        let display = format_hp_display(45, 100);
        assert_eq!(display, "45/100 HP");
    }

    #[test]
    fn test_format_hp_display_full() {
        let display = format_hp_display(100, 100);
        assert_eq!(display, "100/100 HP");
    }

    #[test]
    fn test_format_hp_display_zero() {
        let display = format_hp_display(0, 100);
        assert_eq!(display, "0/100 HP");
    }

    #[test]
    fn test_get_priority_condition_dead() {
        let mut conditions = Condition::new();
        conditions.add(Condition::DEAD);
        let (text, color) = get_priority_condition(&conditions, &[]);
        assert!(text.contains("Dead"));
        assert!(colors_approx_equal(color, HP_DEAD_COLOR));
    }

    #[test]
    fn test_get_priority_condition_poisoned() {
        let mut conditions = Condition::new();
        conditions.add(Condition::POISONED);
        let (text, color) = get_priority_condition(&conditions, &[]);
        assert!(text.contains("Poison"));
        assert!(colors_approx_equal(color, CONDITION_POISONED_COLOR));
    }

    #[test]
    fn test_get_priority_condition_paralyzed() {
        let mut conditions = Condition::new();
        conditions.add(Condition::PARALYZED);
        let (text, color) = get_priority_condition(&conditions, &[]);
        assert!(text.contains("Paralyzed"));
        assert!(colors_approx_equal(color, CONDITION_PARALYZED_COLOR));
    }

    #[test]
    fn test_get_priority_condition_fine() {
        let conditions = Condition::new();
        let (text, color) = get_priority_condition(&conditions, &[]);
        assert!(text.contains("OK"));
        assert!(colors_approx_equal(color, HP_HEALTHY_COLOR));
    }

    #[test]
    fn test_get_priority_condition_multiple() {
        // Dead takes priority over poisoned
        let mut conditions = Condition::new();
        conditions.add(Condition::DEAD);
        conditions.add(Condition::POISONED);
        let (text, _) = get_priority_condition(&conditions, &[]);
        assert!(text.contains("Dead"));
    }

    #[test]
    fn test_get_condition_priority_dead() {
        let mut conditions = Condition::new();
        conditions.add(Condition::DEAD);
        assert_eq!(get_condition_priority(&conditions), PRIORITY_DEAD);
    }

    #[test]
    fn test_get_condition_priority_poisoned() {
        let mut conditions = Condition::new();
        conditions.add(Condition::POISONED);
        assert_eq!(get_condition_priority(&conditions), PRIORITY_POISONED);
    }

    #[test]
    fn test_get_condition_priority_fine() {
        let conditions = Condition::new();
        assert_eq!(get_condition_priority(&conditions), PRIORITY_FINE);
    }

    #[test]
    fn test_count_conditions_none() {
        let conditions = Condition::new();
        assert_eq!(count_conditions(&conditions), 0);
    }

    #[test]
    fn test_count_conditions_single() {
        let mut conditions = Condition::new();
        conditions.add(Condition::POISONED);
        assert_eq!(count_conditions(&conditions), 1);
    }

    #[test]
    fn test_count_conditions_multiple() {
        let mut conditions = Condition::new();
        conditions.add(Condition::POISONED);
        conditions.add(Condition::BLINDED);
        conditions.add(Condition::SILENCED);
        assert_eq!(count_conditions(&conditions), 3);
    }

    #[test]
    fn test_count_conditions_all() {
        let mut conditions = Condition::new();
        conditions.add(Condition::ASLEEP);
        conditions.add(Condition::BLINDED);
        conditions.add(Condition::SILENCED);
        conditions.add(Condition::DISEASED);
        conditions.add(Condition::POISONED);
        conditions.add(Condition::PARALYZED);
        conditions.add(Condition::UNCONSCIOUS);
        conditions.add(Condition::DEAD);
        assert_eq!(count_conditions(&conditions), 8);
    }

    #[test]
    fn test_direction_to_string_north() {
        assert_eq!(direction_to_string(&Direction::North), "N");
    }

    #[test]
    fn test_direction_to_string_east() {
        assert_eq!(direction_to_string(&Direction::East), "E");
    }

    #[test]
    fn test_direction_to_string_south() {
        assert_eq!(direction_to_string(&Direction::South), "S");
    }

    #[test]
    fn test_direction_to_string_west() {
        assert_eq!(direction_to_string(&Direction::West), "W");
    }

    #[test]
    fn test_compass_constants_valid() {
        // Verify compass constants are defined with reasonable values
        assert_eq!(COMPASS_SIZE, 48.0);
        assert_eq!(COMPASS_BORDER_WIDTH, 2.0);
        assert_eq!(COMPASS_FONT_SIZE, 24.0);
    }

    #[test]
    fn test_get_portrait_color_deterministic() {
        // Same portrait_id should always produce same color
        let color1 = get_portrait_color(42);
        let color2 = get_portrait_color(42);
        assert!(colors_approx_equal(color1, color2));
    }

    #[test]
    fn test_get_portrait_color_different_ids() {
        // Different IDs should produce different colors
        let color1 = get_portrait_color(0);
        let color2 = get_portrait_color(1);
        assert!(!colors_approx_equal(color1, color2));
    }

    #[test]
    fn test_get_portrait_color_full_range() {
        // Test boundary values
        let _color_min = get_portrait_color(0);
        let _color_max = get_portrait_color(255);
        // Should not panic and should produce valid colors
    }

    #[test]
    fn test_portrait_constants_valid() {
        // Verify portrait constants are defined with reasonable values
        assert_eq!(PORTRAIT_SIZE, 40.0);
    }
}
