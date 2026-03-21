// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! HUD (Heads-Up Display) system for party status visualization
//!
//! This module provides a native Bevy UI-based HUD that displays:
//! - Character portraits (scaled to fit card with border)
//! - HP bars with color-coded health states and overlay text
//! - HP values (current/max format) as text overlay on HP bar
//! - Active condition indicators with emoji and color coding
//! - Mini map, compass, and clock widgets in a consolidated top-right panel
//! - Full-screen automap overlay rendering for explored map tiles
//!
//! The HUD uses a horizontal strip layout at the bottom of the screen.
//! Card count is dynamic and matches party size (1-6 members).
//! Each card displays portrait, HP bar with overlay text, and conditions.

use crate::application::GameMode;
use crate::domain::character::{Condition, PARTY_MAX_SIZE};
use crate::domain::conditions::ActiveCondition;
use crate::domain::types::{Direction, Position};
use crate::game::components::inventory::{CharacterEntity, PartyEntities};
use crate::game::resources::GlobalState;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::HashMap;
use tracing::{debug, warn};

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
pub const HUD_PANEL_HEIGHT: Val = Val::Px(70.0);
pub const HUD_BOTTOM_GAP: Val = Val::Px(24.0);
pub const CHARACTER_CARD_WIDTH: Val = Val::Px(120.0);
pub const HP_BAR_HEIGHT: Val = Val::Px(10.0);
pub const CARD_PADDING: Val = Val::Px(8.0);
pub const HP_TEXT_OVERLAY_PADDING_LEFT: Val = Val::Px(4.0);
pub const PORTRAIT_PERCENT_OF_CARD: f32 = 80.0;

// HP text overlay colors (contrast-aware, based on bar color)
pub const HP_TEXT_HEALTHY_COLOR: Color = Color::srgba(0.70, 0.92, 0.70, 1.0); // Light green tint (matches healthy bar)
pub const HP_TEXT_INJURED_COLOR: Color = Color::srgba(0.97, 0.90, 0.60, 1.0); // Light yellow tint (matches injured bar)
pub const HP_TEXT_CRITICAL_COLOR: Color = Color::srgba(0.96, 0.65, 0.65, 1.0); // Light red tint (matches critical bar)
pub const HP_TEXT_DEAD_COLOR: Color = Color::srgba(0.75, 0.75, 0.75, 1.0); // Light grey (matches dead bar)

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

// Mini map display constants
pub const MINI_MAP_SIZE_PX: f32 = 80.0;
pub const MINI_MAP_VIEWPORT_RADIUS: u32 = 6;
pub const MINI_MAP_TILE_PX: f32 = 6.0;
pub const MINI_MAP_BG_COLOR: Color = Color::srgba(0.05, 0.05, 0.05, 0.9);
pub const MINI_MAP_VISITED_FLOOR: [u8; 4] = [100, 100, 100, 255];
pub const MINI_MAP_WALL: [u8; 4] = [60, 60, 60, 255];
pub const MINI_MAP_PLAYER: [u8; 4] = [255, 255, 255, 255];
pub const MINI_MAP_UNVISITED: [u8; 4] = [0, 0, 0, 0];
pub const MINI_MAP_NPC_COLOR: [u8; 4] = [0, 200, 100, 255];

// POI colors
pub const POI_QUEST_COLOR: [u8; 4] = [255, 220, 0, 255];
pub const POI_MERCHANT_COLOR: [u8; 4] = [0, 200, 100, 255];
pub const POI_SIGN_COLOR: [u8; 4] = [180, 180, 255, 255];
pub const POI_TELEPORT_COLOR: [u8; 4] = [200, 100, 255, 255];
pub const POI_ENCOUNTER_COLOR: [u8; 4] = [220, 50, 50, 255];
pub const POI_TREASURE_COLOR: [u8; 4] = [255, 180, 0, 255];

// Automap display constants
pub const AUTOMAP_MAX_IMAGE_SIZE_PX: u32 = 768;
pub const AUTOMAP_MIN_TILE_PX: u32 = 4;
pub const AUTOMAP_MAX_TILE_PX: u32 = 16;
pub const AUTOMAP_BG_COLOR: Color = Color::srgba(0.02, 0.02, 0.02, 0.96);
pub const AUTOMAP_UNVISITED: [u8; 4] = [0, 0, 0, 255];
pub const AUTOMAP_VISITED_FLOOR: [u8; 4] = [120, 120, 120, 255];
pub const AUTOMAP_VISITED_WALL: [u8; 4] = [70, 50, 50, 255];
pub const AUTOMAP_VISITED_DOOR: [u8; 4] = [180, 140, 80, 255];
pub const AUTOMAP_VISITED_WATER: [u8; 4] = [60, 80, 160, 255];
pub const AUTOMAP_VISITED_FOREST: [u8; 4] = [50, 120, 50, 255];
pub const AUTOMAP_PLAYER: [u8; 4] = [255, 255, 255, 255];

// Clock display constants
/// Font size used for both time and day lines in the clock widget
pub const CLOCK_FONT_SIZE: f32 = 14.0;
/// Background color of the clock panel (matches compass style)
pub const CLOCK_BACKGROUND_COLOR: Color = Color::srgba(0.1, 0.1, 0.1, 0.9);
/// Border color of the clock panel (matches compass style)
pub const CLOCK_BORDER_COLOR: Color = Color::srgba(0.4, 0.4, 0.4, 1.0);
/// Default text color for the clock (used when no specific period tint is desired)
pub const CLOCK_TEXT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);
/// Text color used during night / evening — cool blue-white tint
pub const CLOCK_NIGHT_TEXT_COLOR: Color = Color::srgba(0.6, 0.6, 1.0, 1.0);
/// Text color used during day periods — warm golden tint
pub const CLOCK_DAY_TEXT_COLOR: Color = Color::srgba(1.0, 0.9, 0.5, 1.0);
/// Vertical gap between the bottom of the compass and the top of the clock panel
pub const CLOCK_TOP_OFFSET: f32 = COMPASS_SIZE + 28.0; // 48 (compass) + 8 (gap) + 20 (top margin)
/// Width of the clock panel (matches compass width)
pub const CLOCK_WIDTH: f32 = COMPASS_SIZE;
/// Padding inside the clock panel
pub const CLOCK_PADDING: f32 = 4.0;

// Portrait display constants
pub const PORTRAIT_SIZE: f32 = 40.0;
pub const PORTRAIT_MARGIN: Val = Val::Px(4.0);
pub const PORTRAIT_PLACEHOLDER_COLOR: Color = Color::srgba(0.3, 0.3, 0.4, 1.0);

// ===== Marker Components =====

/// Marker component for the HUD root container
#[derive(Component)]
pub struct HudRoot;

/// Marker component for the consolidated top-right panel container
#[derive(Component)]
pub struct TopRightPanel;

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

/// Marker component for HP text overlay on the bar
#[derive(Component)]
pub struct HpTextOverlay {
    pub party_index: usize,
}

/// Marker component for the mini map container
#[derive(Component)]
pub struct MiniMapRoot;

/// Marker component for the mini map image node
#[derive(Component)]
pub struct MiniMapCanvas;

/// Marker component for the full-screen automap root overlay
#[derive(Component)]
pub struct AutomapRoot;

/// Marker component for the automap image canvas
#[derive(Component)]
pub struct AutomapCanvas;

/// Marker component for the automap legend / side panel
#[derive(Component)]
pub struct AutomapLegend;

/// Marker component for the compass container
#[derive(Component)]
pub struct CompassRoot;

/// Marker component for the compass direction text
#[derive(Component)]
pub struct CompassText;

/// Marker component for the clock widget container (sits below the compass)
#[derive(Component)]
pub struct ClockRoot;

/// Marker component for the time-of-day text node (displays "HH:MM")
#[derive(Component)]
pub struct ClockTimeText;

/// Marker component for the calendar date text node (displays "Y{year} M{month} D{day}")
#[derive(Component)]
pub struct ClockDateText;

/// Marker component for character portrait image
#[derive(Component)]
pub struct CharacterPortrait {
    pub party_index: usize,
}

// Resource holding loaded portrait image handles for the active campaign.
//
// Indexed by normalized filename stem (lowercased, spaces -> underscores).
#[derive(Resource, Default)]
pub struct PortraitAssets {
    /// Maps filename stem (normalized: lowercase, underscores) -> Image handle.
    /// Keys are normalized filename stems (e.g., "kira", "painter", "10").
    pub handles_by_name: HashMap<String, Handle<Image>>,
    /// Optional fallback image handle
    pub fallback: Option<Handle<Image>>,
    /// Campaign ID this resource is currently populated for (to avoid re-loading)
    pub loaded_for_campaign: Option<String>,
}

/// Dynamic image handle used by the HUD mini map widget.
///
/// The image is created at startup and rewritten in-place each frame while the
/// party is exploring. The UI `ImageNode` uses this handle directly.
///
/// # Examples
///
/// ```
/// use antares::game::systems::hud::MINI_MAP_SIZE_PX;
///
/// assert_eq!(MINI_MAP_SIZE_PX as u32, 80);
/// ```
#[derive(Resource, Clone)]
pub struct MiniMapImage {
    /// Handle to the dynamic RGBA8 image used by the mini map canvas.
    pub handle: Handle<Image>,
}

/// Dynamic image handle used by the full-screen automap overlay.
///
/// The image is resized on demand to fit the current map and rewritten in-place
/// whenever the automap is open.
///
/// # Examples
///
/// ```
/// use antares::game::systems::hud::AUTOMAP_MAX_IMAGE_SIZE_PX;
///
/// assert!(AUTOMAP_MAX_IMAGE_SIZE_PX >= 256);
/// ```
#[derive(Resource, Clone)]
pub struct AutomapImage {
    /// Handle to the dynamic RGBA8 image used by the automap canvas.
    pub handle: Handle<Image>,
}

// ===== Plugin =====

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PortraitAssets::default())
            .init_resource::<Assets<Image>>()
            .add_systems(
                Startup,
                (
                    initialize_mini_map_image,
                    initialize_automap_image,
                    setup_hud,
                    setup_automap,
                    setup_party_entities,
                )
                    .chain(),
            )
            // update_hud must run during combat too so party HP bars reflect live
            // damage.  The exploration-only overlays (compass, clock, portraits) stay
            // gated so they don't render on top of the combat HUD.
            .add_systems(Update, update_hud)
            .add_systems(Update, update_automap_visibility)
            .add_systems(Update, update_automap_image.run_if(in_automap_mode))
            .add_systems(
                Update,
                (
                    ensure_portraits_loaded,
                    update_mini_map,
                    update_compass,
                    update_clock,
                    update_portraits,
                )
                    .run_if(not_in_combat),
            );
    }
}

/// Run condition: returns true when not in combat mode
///
/// Used to hide exploration HUD during combat.
fn not_in_combat(global_state: Res<GlobalState>) -> bool {
    !matches!(global_state.0.mode, GameMode::Combat(_))
}

/// Spawns one pure-identity Bevy entity per party slot and stores them in
/// the [`PartyEntities`] resource.
///
/// Each entity carries a single [`CharacterEntity`] component with its
/// zero-based `party_index`.  These entities have no mesh, transform, or
/// visibility — they exist solely so that later inventory systems can attach
/// per-character components and look up entities by party index.
///
/// The [`PartyEntities`] resource is initialised (all `None`) first, then
/// populated with the freshly-spawned entity handles.
///
/// # Arguments
///
/// * `commands` - Bevy command buffer used to spawn entities and insert the
///   resource.
fn setup_party_entities(mut commands: Commands) {
    // Pre-populate all slots as None, then fill them in.
    let mut entity_array: [Option<Entity>; PARTY_MAX_SIZE] = [None; PARTY_MAX_SIZE];

    for (party_index, slot) in entity_array.iter_mut().enumerate() {
        let entity = commands.spawn(CharacterEntity { party_index }).id();
        *slot = Some(entity);
    }

    commands.insert_resource(PartyEntities {
        entities: entity_array,
    });
}

// ===== Systems =====

/// Creates the dynamic image resource backing the HUD mini map.
///
/// The image is an `RGBA8` texture sized to `MINI_MAP_SIZE_PX × MINI_MAP_SIZE_PX`
/// and initialized fully transparent so fog-of-war is hidden until map data is
/// rendered into it.
///
/// # Arguments
///
/// * `commands` - Bevy command buffer used to insert the resource
/// * `images` - Asset storage for image creation
fn initialize_mini_map_image(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = mini_map_image_size();
    let image = Image::new_fill(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &vec![0; (size * size * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    let handle = images.add(image);
    commands.insert_resource(MiniMapImage { handle });
}

/// Creates the dynamic image resource backing the full-screen automap.
///
/// The initial image is a square transparent RGBA8 texture. The rendering
/// system resizes and repaints it to match the active map when the automap is
/// displayed.
///
/// # Arguments
///
/// * `commands` - Bevy command buffer used to insert the resource
/// * `images` - Asset storage for image creation
fn initialize_automap_image(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = AUTOMAP_MAX_IMAGE_SIZE_PX;
    let image = Image::new_fill(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &vec![0; (size * size * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    let handle = images.add(image);
    commands.insert_resource(AutomapImage { handle });
}

/// Sets up the HUD UI hierarchy (runs once at startup)
///
/// Creates the HUD container and character card slots using Bevy's
/// native UI system with flexbox layout.
///
/// # Arguments
/// * `commands` - Bevy command buffer for spawning entities
/// * `mini_map_image` - Dynamic image handle used by the mini map canvas
fn setup_hud(mut commands: Commands, mini_map_image: Res<MiniMapImage>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: HUD_BOTTOM_GAP,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                height: HUD_PANEL_HEIGHT,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)), // Transparent background
            HudRoot,
        ))
        .with_children(|parent| {
            // Only spawn cards for party members that exist
            for party_index in 0..PARTY_MAX_SIZE {
                // Will be hidden if no character at this index
                // Spawn character card inline due to Bevy's with_children closure type complexity
                parent
                    .spawn((
                        Node {
                            width: CHARACTER_CARD_WIDTH,
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(CARD_PADDING),
                            row_gap: Val::Px(0.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
                        BorderRadius::all(Val::Px(4.0)),
                        CharacterCard { party_index },
                    ))
                    .with_children(|card| {
                        // Portrait (90% of card width, maintains border)
                        card.spawn((
                            Node {
                                width: Val::Percent(PORTRAIT_PERCENT_OF_CARD),
                                height: Val::Percent(PORTRAIT_PERCENT_OF_CARD),
                                margin: UiRect::all(Val::Auto),
                                ..default()
                            },
                            BackgroundColor(PORTRAIT_PLACEHOLDER_COLOR),
                            BorderRadius::all(Val::Px(4.0)),
                            ImageNode::default(),
                            CharacterPortrait { party_index },
                        ));

                        // HP bar container (with relative positioning for text overlay)
                        card.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: HP_BAR_HEIGHT,
                                position_type: PositionType::Relative,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 1.0)),
                            HpBarBackground,
                        ))
                        .with_children(|bar| {
                            // HP bar fill (the colored part)
                            bar.spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(HP_HEALTHY_COLOR),
                                HpBarFill { party_index },
                            ));

                            // HP text overlay on the bar
                            //
                            // NOTE: This needs an explicit width to reliably participate in layout,
                            // and a z-index so it renders above the bar fill.
                            bar.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: HP_TEXT_OVERLAY_PADDING_LEFT,
                                    top: Val::Px(0.0),
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                ZIndex(1),
                                Text::new(""),
                                TextFont {
                                    font_size: 10.0,
                                    ..default()
                                },
                                TextColor(HP_TEXT_HEALTHY_COLOR),
                                HpTextOverlay { party_index },
                            ));
                        });

                        // Condition text
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

    // Spawn consolidated top-right panel containing mini map, compass, and clock.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(20.0),
                right: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            TopRightPanel,
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Node {
                        width: Val::Px(MINI_MAP_SIZE_PX),
                        height: Val::Px(MINI_MAP_SIZE_PX),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        padding: UiRect::all(Val::Px(0.0)),
                        ..default()
                    },
                    BackgroundColor(MINI_MAP_BG_COLOR),
                    MiniMapRoot,
                ))
                .with_children(|mini_map| {
                    mini_map.spawn((
                        Node {
                            width: Val::Px(MINI_MAP_SIZE_PX),
                            height: Val::Px(MINI_MAP_SIZE_PX),
                            ..default()
                        },
                        ImageNode::new(mini_map_image.handle.clone()),
                        MiniMapCanvas,
                    ));
                });

            panel
                .spawn((
                    Node {
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

            panel
                .spawn((
                    Node {
                        display: Display::None,
                        width: Val::Px(CLOCK_WIDTH),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        padding: UiRect::all(Val::Px(CLOCK_PADDING)),
                        row_gap: Val::Px(2.0),
                        ..default()
                    },
                    BackgroundColor(CLOCK_BACKGROUND_COLOR),
                    ClockRoot,
                ))
                .with_children(|parent| {
                    // Time line: "HH:MM"
                    parent.spawn((
                        Text::new("00:00"),
                        TextFont {
                            font_size: CLOCK_FONT_SIZE,
                            ..default()
                        },
                        TextColor(CLOCK_DAY_TEXT_COLOR),
                        ClockTimeText,
                    ));

                    // Date line: "Y1 M1 D1"
                    parent.spawn((
                        Text::new("Y1 M1 D1"),
                        TextFont {
                            font_size: CLOCK_FONT_SIZE,
                            ..default()
                        },
                        TextColor(CLOCK_TEXT_COLOR),
                        ClockDateText,
                    ));
                });
        });
}

/// Updates HUD elements based on current party state
///
/// This system runs every frame to sync UI with game state.
/// Updates HP bars, HP overlay text color, condition text, and character card visibility.
///
/// # Arguments
/// * `global_state` - Game state containing party data
/// * `card_query` - Query for character card visibility
/// * `hp_bar_query` - Query for HP bar fill entities
/// * `hp_overlay_query` - Query for HP text overlay entities
/// * `condition_text_query` - Query for condition text entities
#[allow(clippy::type_complexity)]
fn update_hud(
    global_state: Res<GlobalState>,
    mut card_query: Query<(&CharacterCard, &mut Node), Without<HpBarFill>>,
    mut hp_bar_query: Query<(&HpBarFill, &mut Node, &mut BackgroundColor)>,
    mut hp_overlay_query: Query<
        (&HpTextOverlay, &mut Text, &mut TextColor),
        Without<ConditionText>,
    >,
    mut condition_text_query: Query<
        (&ConditionText, &mut Text, &mut TextColor),
        Without<HpTextOverlay>,
    >,
) {
    let party = &global_state.0.party;

    // Update card visibility - hide cards that don't have characters
    for (card, mut node) in card_query.iter_mut() {
        if party.members.get(card.party_index).is_some() {
            node.display = Display::Flex;
        } else {
            node.display = Display::None;
        }
    }

    // Update HP bars and colors
    for (hp_bar, mut node, mut bg_color) in hp_bar_query.iter_mut() {
        if let Some(character) = party.members.get(hp_bar.party_index) {
            // Guard against division by zero and clamp percent to [0.0, 1.0]
            let hp_percent = if character.hp.base == 0 {
                0.0
            } else {
                (character.hp.current as f32 / character.hp.base as f32).clamp(0.0, 1.0)
            };
            node.width = Val::Percent(hp_percent * 100.0);
            *bg_color = BackgroundColor(hp_bar_color(hp_percent));
        } else {
            // No character in this slot - hide bar
            node.width = Val::Px(0.0);
        }
    }

    // Update HP text overlay with contrast-aware colors
    for (hp_overlay, mut text, mut text_color) in hp_overlay_query.iter_mut() {
        if let Some(character) = party.members.get(hp_overlay.party_index) {
            **text = format_hp_display(character.hp.current, character.hp.base);

            // Color text based on HP bar color for contrast
            let hp_percent = if character.hp.base == 0 {
                0.0
            } else {
                (character.hp.current as f32 / character.hp.base as f32).clamp(0.0, 1.0)
            };

            *text_color = TextColor(hp_text_overlay_color(hp_percent));
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
}

/// Updates the mini map image with the currently-visible world viewport.
///
/// The mini map is centered on the party and renders visited floor tiles,
/// walls, the party marker, and discovered NPC dots. Unvisited or
/// out-of-bounds tiles remain transparent.
///
/// # Arguments
///
/// * `global_state` - Game state containing world and party position
/// * `mini_map_image` - Dynamic image handle resource
/// * `images` - Asset storage containing the writable mini map image
fn update_mini_map(
    global_state: Res<GlobalState>,
    mini_map_image: Res<MiniMapImage>,
    mut images: ResMut<Assets<Image>>,
    mut mini_map_root_query: Query<&mut Node, With<MiniMapRoot>>,
) {
    let show_minimap = global_state.0.config.graphics.show_minimap;

    for mut node in &mut mini_map_root_query {
        node.display = if show_minimap {
            Display::Flex
        } else {
            Display::None
        };
    }

    if !show_minimap {
        return;
    }

    let Some(map) = global_state.0.world.get_current_map() else {
        return;
    };

    let Some(image) = images.get_mut(&mini_map_image.handle) else {
        return;
    };

    let size = mini_map_image_size();
    let player_pos = global_state.0.world.party_position;
    let viewport_diameter = mini_map_viewport_diameter() as i32;
    let pixel_scale = mini_map_pixels_per_tile() as i32;
    let center_tile_index = (viewport_diameter / 2).min(viewport_diameter.saturating_sub(1));

    image.data = Some(vec![0; (size * size * 4) as usize]);

    let Some(data) = image.data.as_mut() else {
        return;
    };

    for tile_y in 0..viewport_diameter {
        for tile_x in 0..viewport_diameter {
            let world_x = player_pos.x + tile_x - center_tile_index;
            let world_y = player_pos.y + tile_y - center_tile_index;
            let world_pos = Position::new(world_x, world_y);

            let color = if !map.is_valid_position(world_pos) {
                MINI_MAP_UNVISITED
            } else if let Some(tile) = map.get_tile(world_pos) {
                if !tile.visited {
                    MINI_MAP_UNVISITED
                } else if world_pos == player_pos {
                    MINI_MAP_PLAYER
                } else if tile.is_blocked() {
                    MINI_MAP_WALL
                } else {
                    MINI_MAP_VISITED_FLOOR
                }
            } else {
                MINI_MAP_UNVISITED
            };

            fill_mini_map_tile(data, tile_x, tile_y, color, size, pixel_scale);
        }
    }

    for npc in &map.npc_placements {
        let dx = npc.position.x - player_pos.x;
        let dy = npc.position.y - player_pos.y;
        let radius = MINI_MAP_VIEWPORT_RADIUS as i32;

        if dx.abs() > radius || dy.abs() > radius {
            continue;
        }

        let Some(tile) = map.get_tile(npc.position) else {
            continue;
        };

        if !tile.visited {
            continue;
        }

        let tile_x = dx + center_tile_index;
        let tile_y = dy + center_tile_index;
        fill_mini_map_npc_dot(data, tile_x, tile_y, size, pixel_scale);
    }

    for (poi_position, poi) in map.collect_map_pois(&global_state.0.quests) {
        let dx = poi_position.x - player_pos.x;
        let dy = poi_position.y - player_pos.y;
        let radius = MINI_MAP_VIEWPORT_RADIUS as i32;

        if dx.abs() > radius || dy.abs() > radius {
            continue;
        }

        let tile_x = dx + center_tile_index;
        let tile_y = dy + center_tile_index;
        fill_mini_map_poi_dot(data, tile_x, tile_y, poi_color(&poi), size, pixel_scale);
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

/// Updates the clock widget with the current in-game time and calendar date.
///
/// Runs every frame (guarded by `not_in_combat`) so that any time
/// advancement — a step, rest, map transition, or combat round — is
/// reflected in the HUD without delay.
///
/// The time text color shifts between [`CLOCK_DAY_TEXT_COLOR`] during bright
/// periods (Dawn through Dusk) and [`CLOCK_NIGHT_TEXT_COLOR`] during dark
/// periods (Evening and Night), giving the player an ambient visual cue.
///
/// # Arguments
/// * `global_state` - Game state containing the `time` field
/// * `time_query`   - Query for the [`ClockTimeText`] entity
/// * `date_query`   - Query for the [`ClockDateText`] entity
#[allow(clippy::type_complexity)]
fn update_clock(
    global_state: Res<GlobalState>,
    mut time_query: Query<
        (&mut Text, &mut TextColor),
        (With<ClockTimeText>, Without<ClockDateText>),
    >,
    mut date_query: Query<
        (&mut Text, &mut TextColor),
        (With<ClockDateText>, Without<ClockTimeText>),
    >,
) {
    let game_time = &global_state.0.time;
    let time_of_day = global_state.0.time_of_day();
    let time_color = clock_text_color(time_of_day);

    for (mut text, mut color) in &mut time_query {
        **text = format_clock_time(game_time.hour, game_time.minute);
        *color = TextColor(time_color);
    }
    for (mut text, _color) in &mut date_query {
        **text = format_clock_date(game_time.year, game_time.month, game_time.day);
    }
}

/// Spawns the full-screen automap overlay hierarchy.
///
/// The overlay is hidden by default and becomes visible only while the game is
/// in `GameMode::Automap`.
///
/// # Arguments
///
/// * `commands` - Bevy command buffer used to spawn UI entities
/// * `automap_image` - Dynamic image resource used by the automap canvas
fn setup_automap(mut commands: Commands, automap_image: Res<AutomapImage>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::None,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Stretch,
                padding: UiRect::all(Val::Px(24.0)),
                ..default()
            },
            BackgroundColor(AUTOMAP_BG_COLOR),
            ZIndex(10),
            AutomapRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_grow: 1.0,
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|canvas_parent| {
                canvas_parent.spawn((
                    Node {
                        width: Val::Px(AUTOMAP_MAX_IMAGE_SIZE_PX as f32),
                        height: Val::Px(AUTOMAP_MAX_IMAGE_SIZE_PX as f32),
                        ..default()
                    },
                    ImageNode::new(automap_image.handle.clone()),
                    AutomapCanvas,
                ));
            });

            root.spawn((
                Node {
                    width: Val::Px(240.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    align_items: AlignItems::FlexStart,
                    justify_content: JustifyContent::FlexStart,
                    padding: UiRect::all(Val::Px(12.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.08, 0.08, 0.08, 0.85)),
                AutomapLegend,
            ))
            .with_children(|legend| {
                legend.spawn((
                    Text::new("Automap"),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

                for (color, label) in [
                    (AUTOMAP_PLAYER, "You are here"),
                    (POI_QUEST_COLOR, "Quest objective"),
                    (POI_MERCHANT_COLOR, "Merchant"),
                    (POI_SIGN_COLOR, "Sign / notice"),
                    (POI_TELEPORT_COLOR, "Teleport"),
                    (POI_ENCOUNTER_COLOR, "Monster encounter"),
                    (POI_TREASURE_COLOR, "Treasure"),
                ] {
                    legend
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(8.0),
                                ..default()
                            },
                            BackgroundColor(Color::NONE),
                        ))
                        .with_children(|row| {
                            row.spawn((
                                Node {
                                    width: Val::Px(20.0),
                                    height: Val::Px(20.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba_u8(
                                    color[0], color[1], color[2], color[3],
                                )),
                            ));
                            row.spawn((
                                Text::new(label),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                }

                legend.spawn((
                    Text::new("Gray: Explored floor"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                legend.spawn((
                    Text::new("Dark red: Wall"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                legend.spawn((
                    Text::new("Tan: Door"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                legend.spawn((
                    Text::new("Blue: Water"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                legend.spawn((
                    Text::new("Green: Forest / grass"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });

            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(24.0),
                    bottom: Val::Px(24.0),
                    ..default()
                },
                Text::new("M / Esc — close map"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Updates automap root visibility based on the current game mode.
///
/// # Arguments
///
/// * `global_state` - Game state containing the current mode
/// * `automap_query` - Query for the overlay root node
fn update_automap_visibility(
    global_state: Res<GlobalState>,
    mut automap_query: Query<&mut Node, With<AutomapRoot>>,
) {
    let display = if matches!(global_state.0.mode, GameMode::Automap) {
        Display::Flex
    } else {
        Display::None
    };

    for mut node in &mut automap_query {
        node.display = display;
    }
}

/// Updates the full-screen automap image while the automap overlay is open.
///
/// The rendered image covers the whole current map using fog-of-war coloring.
///
/// # Arguments
///
/// * `global_state` - Game state containing the world and player position
/// * `automap_image` - Dynamic image resource used by the automap canvas
/// * `images` - Asset storage containing the writable automap image
/// * `canvas_query` - Query for the automap canvas node
fn update_automap_image(
    global_state: Res<GlobalState>,
    automap_image: Res<AutomapImage>,
    mut images: ResMut<Assets<Image>>,
    mut canvas_query: Query<&mut Node, With<AutomapCanvas>>,
) {
    let Some(map) = global_state.0.world.get_current_map() else {
        return;
    };

    let max_dimension = map.width.max(map.height).max(1);
    let tile_px =
        (AUTOMAP_MAX_IMAGE_SIZE_PX / max_dimension).clamp(AUTOMAP_MIN_TILE_PX, AUTOMAP_MAX_TILE_PX);
    let image_width = map.width * tile_px;
    let image_height = map.height * tile_px;

    let Some(image) = images.get_mut(&automap_image.handle) else {
        return;
    };

    image.texture_descriptor.size = Extent3d {
        width: image_width,
        height: image_height,
        depth_or_array_layers: 1,
    };
    image.data = Some(vec![0; (image_width * image_height * 4) as usize]);

    let Some(data) = image.data.as_mut() else {
        return;
    };

    for y in 0..map.height {
        for x in 0..map.width {
            let pos = Position::new(x as i32, y as i32);
            let Some(tile) = map.get_tile(pos) else {
                continue;
            };

            let mut color = automap_tile_color(tile);
            if pos == global_state.0.world.party_position {
                color = AUTOMAP_PLAYER;
            }

            fill_automap_tile(data, x, y, color, image_width, tile_px);
        }
    }

    for (poi_position, poi) in map.collect_map_pois(&global_state.0.quests) {
        if poi_position.x < 0 || poi_position.y < 0 {
            continue;
        }

        fill_automap_poi_dot(
            data,
            poi_position.x as u32,
            poi_position.y as u32,
            poi_color(&poi),
            image_width,
            tile_px,
        );
    }

    for mut node in &mut canvas_query {
        node.width = Val::Px(image_width as f32);
        node.height = Val::Px(image_height as f32);
    }
}

/// Updates portrait images and background colors based on campaign assets
///
/// If an image matching the character's portrait exists it will be displayed
/// (numeric filename `10.png` or name-based `kira.png`). Otherwise the
/// deterministic color placeholder continues to be used.
///
/// # Arguments
/// * `global_state` - Game state containing party data
/// * `portraits` - Loaded portrait assets (resource)
/// * `portrait_query` - Query for portrait UI nodes (background + image)
fn update_portraits(
    global_state: Res<GlobalState>,
    portraits: Res<PortraitAssets>,
    asset_server: Option<Res<AssetServer>>,
    images: Option<Res<Assets<Image>>>,
    mut portrait_query: Query<(&CharacterPortrait, &mut BackgroundColor, &mut ImageNode)>,
) {
    let party = &global_state.0.party;

    for (portrait, mut bg_color, mut image_node) in portrait_query.iter_mut() {
        if let Some(character) = party.members.get(portrait.party_index) {
            debug!(
                "update_portraits: slot {} checking for character '{}' (portrait_id={})",
                portrait.party_index, character.name, character.portrait_id
            );

            // Normalize keys
            let portrait_key = character
                .portrait_id
                .trim()
                .to_lowercase()
                .replace(' ', "_");
            let name_key = character.name.to_lowercase().replace(' ', "_");

            debug!(
                "update_portraits: slot {} normalized keys => portrait_key='{}', name_key='{}'",
                portrait.party_index, portrait_key, name_key
            );

            // Helper to check if a handle is loaded. We consider a handle 'loaded' if:
            // - It's present in the `Assets<Image>` storage (useful for tests where assets
            //   are inserted directly), or
            // - The AssetServer reports the handle as `Loaded` (normal runtime case).
            let is_handle_loaded = |handle: &Handle<Image>| -> bool {
                if handle == &Handle::<Image>::default() {
                    return false;
                }
                // If the asset exists in the Assets<Image> storage, treat it as loaded.
                if let Some(imgs) = &images {
                    if imgs.get(handle.id()).is_some() {
                        return true;
                    }
                }
                // Otherwise, if an AssetServer is available, consult its load state.
                if let Some(server) = &asset_server {
                    server
                        .get_load_state(handle.id())
                        .map(|s| s.is_loaded())
                        .unwrap_or(false)
                } else {
                    // No AssetServer available (tests / minimal env) -> assume available
                    true
                }
            };

            // Lookup by explicit portrait_id key first (if provided)
            if !portrait_key.is_empty() {
                if let Some(handle) = portraits.handles_by_name.get(&portrait_key) {
                    debug!(
                        "update_portraits: slot {} found handle for key '{}' (handle id={:?})",
                        portrait.party_index,
                        portrait_key,
                        handle.id()
                    );
                    if is_handle_loaded(handle) {
                        debug!(
                            "update_portraits: slot {} applying loaded portrait '{}' (handle id={:?})",
                            portrait.party_index,
                            portrait_key,
                            handle.id()
                        );
                        image_node.image = handle.clone();
                        image_node.color = Color::WHITE;
                        *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0));
                        continue;
                    } else {
                        debug!(
                            "update_portraits: slot {} found portrait '{}' but not yet loaded; showing placeholder color",
                            portrait.party_index,
                            portrait_key
                        );
                        // Keep placeholder until asset is fully loaded
                        image_node.image = Handle::<Image>::default();
                        image_node.color = Color::WHITE;
                        *bg_color = BackgroundColor(get_portrait_color(portrait_key.as_str()));
                        continue;
                    }
                }
            }

            // Then try lookup by normalized name
            if let Some(handle) = portraits.handles_by_name.get(&name_key) {
                debug!(
                    "update_portraits: slot {} fallback found handle for name '{}' (handle id={:?})",
                    portrait.party_index, name_key, handle.id()
                );
                if is_handle_loaded(handle) {
                    debug!(
                        "update_portraits: slot {} applying loaded portrait for name '{}'",
                        portrait.party_index, name_key
                    );
                    image_node.image = handle.clone();
                    image_node.color = Color::WHITE;
                    *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0));
                } else {
                    debug!(
                        "update_portraits: slot {} handle for name '{}' not yet loaded; using placeholder color",
                        portrait.party_index, name_key
                    );
                    // No image available / not yet loaded -> fallback to deterministic color
                    image_node.image = Handle::<Image>::default();
                    image_node.color = Color::WHITE;
                    let color_key = if !portrait_key.is_empty() {
                        portrait_key.as_str()
                    } else {
                        name_key.as_str()
                    };
                    *bg_color = BackgroundColor(get_portrait_color(color_key));
                }
            } else {
                // No image available -> fallback to deterministic color based on portrait_key or name_key
                let color_key = if !portrait_key.is_empty() {
                    portrait_key.as_str()
                } else {
                    name_key.as_str()
                };
                debug!(
                    "update_portraits: slot {} no image found for '{}'/'{}'; using placeholder color key='{}'",
                    portrait.party_index, portrait_key, name_key, color_key
                );
                image_node.image = Handle::<Image>::default();
                image_node.color = Color::WHITE;
                *bg_color = BackgroundColor(get_portrait_color(color_key));
            }
        } else {
            debug!(
                "update_portraits: clearing empty slot {}",
                portrait.party_index
            );
            // Empty slot -> clear image and use default placeholder color
            image_node.image = ImageNode::default().image;
            image_node.color = Color::WHITE;
            *bg_color = BackgroundColor(PORTRAIT_PLACEHOLDER_COLOR);
        }
    }
}

/// Ensures portrait image assets for the active campaign are discovered and loaded.
///
/// Scans `<campaign_root>/assets/portraits/` for image files (png/jpg/jpeg)
/// and loads them via the `AssetServer`. Files are indexed by normalized filename stem
/// (lowercased, spaces replaced by underscores).
fn ensure_portraits_loaded(
    global_state: Res<GlobalState>,
    asset_server: Option<Res<AssetServer>>,
    mut portraits: ResMut<PortraitAssets>,
) {
    // Only proceed when campaign is loaded
    let campaign = match &global_state.0.campaign {
        Some(c) => c,
        None => return,
    };
    debug!("ensure_portraits_loaded: campaign id = {}", campaign.id);

    // If there's no AssetServer resource available yet (e.g., test environments that don't
    // register the full set of Bevy plugins), skip loading for now. We'll try again on the
    // next frame when the resource may exist.
    let asset_server = match asset_server {
        Some(a) => a,
        None => {
            debug!("ensure_portraits_loaded: no AssetServer available; skipping portrait load for campaign {}", campaign.id);
            return;
        }
    };

    // If we've already loaded portraits for this campaign, nothing to do
    if let Some(loaded_id) = &portraits.loaded_for_campaign {
        if loaded_id == &campaign.id {
            return;
        }
    }

    let portraits_dir = campaign.root_path.join("assets/portraits");
    if !portraits_dir.exists() {
        debug!("ensure_portraits_loaded: portraits_dir '{}' does not exist for campaign '{}', marking as loaded", portraits_dir.display(), campaign.id);
        portraits.loaded_for_campaign = Some(campaign.id.clone());
        return;
    }

    if let Ok(entries) = std::fs::read_dir(&portraits_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext = ext.to_lowercase();
                if ext == "png" || ext == "jpg" || ext == "jpeg" {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        debug!(
                            "ensure_portraits_loaded: found portrait file: {}",
                            path.display()
                        );

                        // Compute the path relative to the campaign root and load it via the campaign's
                        // named AssetSource (`<campaign_id>://...`). This keeps asset references relative
                        // to the campaign directory while ensuring they are approved by the AssetServer.
                        let relative_path = match path.strip_prefix(&campaign.root_path) {
                            Ok(p) => p.to_path_buf(),
                            Err(_) => {
                                // Fallback: try canonicalizing both path and campaign root to compute a relative path.
                                match (
                                    std::fs::canonicalize(&path),
                                    std::fs::canonicalize(&campaign.root_path),
                                ) {
                                    (Ok(abs_path), Ok(abs_root)) => abs_path
                                        .strip_prefix(&abs_root)
                                        .map(|p| p.to_path_buf())
                                        .unwrap_or_else(|_| {
                                            std::path::PathBuf::from(
                                                path.file_name()
                                                    .and_then(|n| n.to_str())
                                                    .unwrap_or_default(),
                                            )
                                        }),
                                    _ => std::path::PathBuf::from(
                                        path.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or_default(),
                                    ),
                                }
                            }
                        };

                        // Normalize separators to '/' for AssetPath parsing
                        let rel_str = relative_path
                            .components()
                            .map(|c| c.as_os_str().to_string_lossy())
                            .collect::<Vec<_>>()
                            .join("/");

                        // Load relative to the campaign (BEVY_ASSET_ROOT points to the campaign directory)
                        debug!(
                            "ensure_portraits_loaded: loading portrait via relative asset path '{}' (abs={})",
                            rel_str,
                            path.display()
                        );

                        // Attempt a normal load first.
                        let mut handle: Handle<Image> = asset_server.load(rel_str.clone());

                        // Log immediate handle id and load state for debugging
                        debug!(
                            "ensure_portraits_loaded: after load(): handle id={:?}, load_state={:?}",
                            handle.id(),
                            asset_server.get_load_state(handle.id())
                        );

                        if handle == Handle::default() {
                            warn!(
                                "ensure_portraits_loaded: AssetServer returned default handle when loading '{}' (likely unapproved path or missing loader); attempting load_override (campaign='{}')",
                                rel_str, campaign.id
                            );
                            let override_handle: Handle<Image> =
                                asset_server.load_override(rel_str.clone());

                            debug!(
                                "ensure_portraits_loaded: after load_override(): handle id={:?}, load_state={:?}",
                                override_handle.id(),
                                asset_server.get_load_state(override_handle.id())
                            );

                            if override_handle != Handle::default() {
                                handle = override_handle;
                            } else {
                                warn!(
                                    "ensure_portraits_loaded: failed to load portrait '{}' for campaign '{}' (unapproved path or missing loader); skipping",
                                    rel_str, campaign.id
                                );
                                continue;
                            }
                        }

                        // Only index non-default handles so we don't store transparent-placeholder handles
                        // which would prevent later attempts to resolve the asset.
                        if handle != Handle::default() {
                            let key = stem.to_lowercase().replace(' ', "_");
                            portraits
                                .handles_by_name
                                .insert(key.clone(), handle.clone());
                            debug!(
                                "ensure_portraits_loaded: indexed '{}' as key '{}' (handle id={:?})",
                                path.display(),
                                key,
                                handle.id()
                            );
                        } else {
                            // Defensive: shouldn't normally happen because of checks above.
                            warn!(
                                "ensure_portraits_loaded: skipping indexing of portrait '{}' (default handle)",
                                rel_str
                            );
                        }
                    }
                }
            }
        }
    }

    portraits.loaded_for_campaign = Some(campaign.id.clone());
    debug!(
        "ensure_portraits_loaded: loaded {} portraits for campaign '{}'",
        portraits.handles_by_name.len(),
        campaign.id
    );
}

// (removed) Path canonicalization helper
// Portraits are now loaded via named campaign asset sources (e.g. `campaign_id://assets/portraits/foo.png`)
// so the dedicated absolute-path helper is no longer needed.

// ===== Helper Functions =====

/// Run condition: returns true when the game is currently in automap mode.
///
/// # Examples
///
/// ```
/// use antares::application::GameState;
/// use antares::game::resources::GlobalState;
/// use antares::game::systems::hud::in_automap_mode;
/// use bevy::prelude::World;
///
/// let mut world = World::new();
/// world.insert_resource(GlobalState(GameState::new()));
/// assert!(!world.run_system_cached(in_automap_mode).unwrap());
/// ```
fn in_automap_mode(global_state: Res<GlobalState>) -> bool {
    matches!(global_state.0.mode, GameMode::Automap)
}

/// Returns the dynamic mini map image size in pixels.
///
/// # Examples
///
/// ```
/// use antares::game::systems::hud::mini_map_image_size;
///
/// assert_eq!(mini_map_image_size(), 80);
/// ```
pub fn mini_map_image_size() -> u32 {
    MINI_MAP_SIZE_PX as u32
}

/// Returns the mini map viewport diameter in tiles.
///
/// This is `radius * 2 + 1`, which keeps the player centered in the viewport.
///
/// # Examples
///
/// ```
/// use antares::game::systems::hud::mini_map_viewport_diameter;
///
/// assert_eq!(mini_map_viewport_diameter(), 13);
/// ```
pub fn mini_map_viewport_diameter() -> u32 {
    MINI_MAP_VIEWPORT_RADIUS * 2 + 1
}

/// Returns the number of pixels used to render one mini map tile.
///
/// # Examples
///
/// ```
/// use antares::game::systems::hud::mini_map_pixels_per_tile;
///
/// assert_eq!(mini_map_pixels_per_tile(), 6);
/// ```
pub fn mini_map_pixels_per_tile() -> u32 {
    MINI_MAP_TILE_PX as u32
}

/// Returns the starting byte offset of a pixel inside an RGBA8 mini map buffer.
///
/// # Arguments
///
/// * `x` - Pixel x coordinate
/// * `y` - Pixel y coordinate
/// * `image_size` - Width/height of the square image in pixels
pub fn mini_map_pixel_offset(x: u32, y: u32, image_size: u32) -> usize {
    ((y * image_size + x) * 4) as usize
}

/// Writes a solid-colored tile-sized block into the mini map pixel buffer.
pub fn fill_mini_map_tile(
    data: &mut [u8],
    tile_x: i32,
    tile_y: i32,
    color: [u8; 4],
    image_size: u32,
    pixel_scale: i32,
) {
    if tile_x < 0 || tile_y < 0 || pixel_scale <= 0 {
        return;
    }

    let start_x = tile_x * pixel_scale;
    let start_y = tile_y * pixel_scale;

    for py in 0..pixel_scale {
        for px in 0..pixel_scale {
            let x = start_x + px;
            let y = start_y + py;

            if x < 0 || y < 0 {
                continue;
            }

            let x = x as u32;
            let y = y as u32;

            if x >= image_size || y >= image_size {
                continue;
            }

            let offset = mini_map_pixel_offset(x, y, image_size);
            data[offset..offset + 4].copy_from_slice(&color);
        }
    }
}

/// Writes a 2×2 NPC marker inside the tile cell on the mini map.
pub fn fill_mini_map_npc_dot(
    data: &mut [u8],
    tile_x: i32,
    tile_y: i32,
    image_size: u32,
    pixel_scale: i32,
) {
    if tile_x < 0 || tile_y < 0 || pixel_scale <= 0 {
        return;
    }

    let start_x = tile_x * pixel_scale + (pixel_scale / 2) - 1;
    let start_y = tile_y * pixel_scale + (pixel_scale / 2) - 1;

    for py in 0..2 {
        for px in 0..2 {
            let x = start_x + px;
            let y = start_y + py;

            if x < 0 || y < 0 {
                continue;
            }

            let x = x as u32;
            let y = y as u32;

            if x >= image_size || y >= image_size {
                continue;
            }

            let offset = mini_map_pixel_offset(x, y, image_size);
            data[offset..offset + 4].copy_from_slice(&MINI_MAP_NPC_COLOR);
        }
    }
}

/// Writes a 2×2 POI marker inside the tile cell on the mini map.
pub fn fill_mini_map_poi_dot(
    data: &mut [u8],
    tile_x: i32,
    tile_y: i32,
    color: [u8; 4],
    image_size: u32,
    pixel_scale: i32,
) {
    if tile_x < 0 || tile_y < 0 || pixel_scale <= 0 {
        return;
    }

    let start_x = tile_x * pixel_scale + (pixel_scale / 2) - 1;
    let start_y = tile_y * pixel_scale + (pixel_scale / 2) - 1;

    for py in 0..2 {
        for px in 0..2 {
            let x = start_x + px;
            let y = start_y + py;

            if x < 0 || y < 0 {
                continue;
            }

            let x = x as u32;
            let y = y as u32;

            if x >= image_size || y >= image_size {
                continue;
            }

            let offset = mini_map_pixel_offset(x, y, image_size);
            data[offset..offset + 4].copy_from_slice(&color);
        }
    }
}

/// Returns the color used for an automap tile based on visit state and terrain.
///
/// # Arguments
///
/// * `tile` - Tile to classify for automap rendering
pub fn automap_tile_color(tile: &crate::domain::world::Tile) -> [u8; 4] {
    use crate::domain::world::{TerrainType, WallType};

    if !tile.visited {
        return AUTOMAP_UNVISITED;
    }

    match tile.wall_type {
        WallType::Door => AUTOMAP_VISITED_DOOR,
        WallType::Normal | WallType::Torch => AUTOMAP_VISITED_WALL,
        WallType::None => match tile.terrain {
            TerrainType::Water => AUTOMAP_VISITED_WATER,
            TerrainType::Grass | TerrainType::Forest => AUTOMAP_VISITED_FOREST,
            _ => AUTOMAP_VISITED_FLOOR,
        },
    }
}

/// Returns the display color for a point of interest.
pub fn poi_color(poi: &crate::domain::world::PointOfInterest) -> [u8; 4] {
    match poi {
        crate::domain::world::PointOfInterest::QuestObjective { .. } => POI_QUEST_COLOR,
        crate::domain::world::PointOfInterest::Merchant => POI_MERCHANT_COLOR,
        crate::domain::world::PointOfInterest::Sign => POI_SIGN_COLOR,
        crate::domain::world::PointOfInterest::Teleport => POI_TELEPORT_COLOR,
        crate::domain::world::PointOfInterest::Encounter => POI_ENCOUNTER_COLOR,
        crate::domain::world::PointOfInterest::Treasure => POI_TREASURE_COLOR,
    }
}

/// Returns the starting byte offset of a pixel inside an RGBA8 automap buffer.
///
/// # Arguments
///
/// * `x` - Pixel x coordinate
/// * `y` - Pixel y coordinate
/// * `image_width` - Width of the image in pixels
pub fn automap_pixel_offset(x: u32, y: u32, image_width: u32) -> usize {
    ((y * image_width + x) * 4) as usize
}

/// Writes a solid-colored tile-sized block into the automap pixel buffer.
pub fn fill_automap_tile(
    data: &mut [u8],
    tile_x: u32,
    tile_y: u32,
    color: [u8; 4],
    image_width: u32,
    pixel_scale: u32,
) {
    let start_x = tile_x * pixel_scale;
    let start_y = tile_y * pixel_scale;

    for py in 0..pixel_scale {
        for px in 0..pixel_scale {
            let x = start_x + px;
            let y = start_y + py;
            let offset = automap_pixel_offset(x, y, image_width);
            data[offset..offset + 4].copy_from_slice(&color);
        }
    }
}

/// Writes a 3×3 POI marker inside the tile cell on the automap.
pub fn fill_automap_poi_dot(
    data: &mut [u8],
    tile_x: u32,
    tile_y: u32,
    color: [u8; 4],
    image_width: u32,
    pixel_scale: u32,
) {
    let start_x = tile_x * pixel_scale + (pixel_scale / 2).saturating_sub(1);
    let start_y = tile_y * pixel_scale + (pixel_scale / 2).saturating_sub(1);

    for py in 0..3 {
        for px in 0..3 {
            let x = start_x + px;
            let y = start_y + py;
            let offset = automap_pixel_offset(x, y, image_width);
            data[offset..offset + 4].copy_from_slice(&color);
        }
    }
}

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

/// Formats HP display as "HP: current/max"
///
/// # Arguments
/// * `current` - Current HP value
/// * `max` - Maximum HP value
///
/// # Returns
/// Formatted string like "HP: 45/100"
///
/// # Examples
///
/// ```
/// use antares::game::systems::hud::format_hp_display;
///
/// let display = format_hp_display(45, 100);
/// assert_eq!(display, "HP: 45/100");
/// ```
pub fn format_hp_display(current: u16, max: u16) -> String {
    format!("HP: {}/{}", current, max)
}

/// Returns the appropriate text color for HP overlay based on health percentage
///
/// Uses contrast-aware colors that are darker/lighter than the bar background:
/// - Healthy (75%+): Off-white text for green bar
/// - Injured (25-75%): Dark text for yellow bar
/// - Critical (0-25%): Off-white text for red bar
/// - Dead (0%): Light grey text for grey bar
///
/// # Arguments
/// * `hp_percent` - Health percentage (0.0 to 1.0)
///
/// # Returns
/// Color appropriate for the current HP state
///
/// # Examples
///
/// ```
/// use antares::game::systems::hud::hp_text_overlay_color;
///
/// let healthy_color = hp_text_overlay_color(0.90);
/// let critical_color = hp_text_overlay_color(0.10);
/// ```
pub fn hp_text_overlay_color(hp_percent: f32) -> Color {
    if hp_percent > HP_HEALTHY_THRESHOLD {
        HP_TEXT_HEALTHY_COLOR
    } else if hp_percent > HP_CRITICAL_THRESHOLD {
        HP_TEXT_INJURED_COLOR
    } else if hp_percent > 0.0 {
        HP_TEXT_CRITICAL_COLOR
    } else {
        HP_TEXT_DEAD_COLOR
    }
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
/// Formats the in-game hour and minute as a zero-padded "HH:MM" string.
///
/// # Arguments
/// * `hour`   - Current hour (0–23)
/// * `minute` - Current minute (0–59)
///
/// # Returns
/// A `String` in `"HH:MM"` format.
///
/// # Examples
///
/// ```
/// use antares::game::systems::hud::format_clock_time;
///
/// assert_eq!(format_clock_time(0, 0),   "00:00");
/// assert_eq!(format_clock_time(9, 5),   "09:05");
/// assert_eq!(format_clock_time(12, 5),  "12:05");
/// assert_eq!(format_clock_time(23, 59), "23:59");
/// ```
pub fn format_clock_time(hour: u8, minute: u8) -> String {
    format!("{:02}:{:02}", hour, minute)
}

/// Formats the in-game calendar date as `"Y{year} M{month} D{day}"`.
///
/// The compact format is designed to fit within the fixed-width clock panel
/// that sits below the compass widget.  It shows all three calendar fields
/// in a predictable left-to-right order.
///
/// # Arguments
/// * `year`  - Current year (1-based)
/// * `month` - Current month within the year (1-based, 1–12)
/// * `day`   - Current day within the month (1-based, 1–30)
///
/// # Returns
/// A `String` in `"Y{year} M{month} D{day}"` format.
///
/// # Examples
///
/// ```
/// use antares::game::systems::hud::format_clock_date;
///
/// assert_eq!(format_clock_date(1, 1, 1),   "Y1 M1 D1");
/// assert_eq!(format_clock_date(4, 12, 30), "Y4 M12 D30");
/// assert_eq!(format_clock_date(1, 6, 15),  "Y1 M6 D15");
/// assert_eq!(format_clock_date(2, 1, 1),   "Y2 M1 D1");
/// ```
pub fn format_clock_date(year: u32, month: u32, day: u32) -> String {
    format!("Y{} M{} D{}", year, month, day)
}

/// Returns the appropriate clock text color for a given [`TimeOfDay`] period.
///
/// - Dark periods (Evening, Night) → [`CLOCK_NIGHT_TEXT_COLOR`] (cool blue-white)
/// - All other periods             → [`CLOCK_DAY_TEXT_COLOR`]   (warm golden)
///
/// # Examples
///
/// ```
/// use antares::domain::types::TimeOfDay;
/// use antares::game::systems::hud::{
///     clock_text_color, CLOCK_DAY_TEXT_COLOR, CLOCK_NIGHT_TEXT_COLOR,
/// };
/// use bevy::prelude::Color;
///
/// let night_color = clock_text_color(TimeOfDay::Night);
/// let day_color   = clock_text_color(TimeOfDay::Morning);
/// // Night → cool tint
/// assert_eq!(night_color, CLOCK_NIGHT_TEXT_COLOR);
/// // Morning → warm tint
/// assert_eq!(day_color,   CLOCK_DAY_TEXT_COLOR);
/// ```
pub fn clock_text_color(time_of_day: crate::domain::types::TimeOfDay) -> Color {
    if time_of_day.is_dark() {
        CLOCK_NIGHT_TEXT_COLOR
    } else {
        CLOCK_DAY_TEXT_COLOR
    }
}

pub fn direction_to_string(direction: &Direction) -> String {
    match direction {
        Direction::North => "N".to_string(),
        Direction::East => "E".to_string(),
        Direction::South => "S".to_string(),
        Direction::West => "W".to_string(),
    }
}

/// Returns portrait color based on portrait key
///
/// Generates a deterministic color from the portrait key (filename stem) for placeholder display.
/// Each portrait key maps to a unique color for visual distinction.
///
/// # Arguments
/// * `portrait_key` - Character's portrait identifier (filename stem / normalized key)
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
/// let color = get_portrait_color("0");
/// // Returns a deterministic color based on key
/// ```
pub fn get_portrait_color(portrait_key: &str) -> Color {
    // Deterministic color from string key via hashing
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    portrait_key.hash(&mut hasher);
    let hash = hasher.finish();

    // Use golden angle distribution on hash to obtain hue
    let hue = (hash as f32 * 137.5) % 360.0;
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
mod layout_tests {
    use super::*;

    #[test]
    fn test_hud_panel_height_reduced() {
        assert_eq!(HUD_PANEL_HEIGHT, Val::Px(70.0));
    }

    #[test]
    fn test_hp_bar_height_thinner() {
        assert_eq!(HP_BAR_HEIGHT, Val::Px(10.0));
    }

    #[test]
    fn test_portrait_percent_of_card() {
        assert_eq!(PORTRAIT_PERCENT_OF_CARD, 80.0);
    }

    #[test]
    fn test_hp_text_overlay_padding() {
        assert_eq!(HP_TEXT_OVERLAY_PADDING_LEFT, Val::Px(4.0));
    }

    #[test]
    fn test_hp_text_overlay_color_healthy() {
        let color = hp_text_overlay_color(0.90);
        assert_eq!(color, HP_TEXT_HEALTHY_COLOR);
    }

    #[test]
    fn test_hp_text_overlay_color_injured() {
        let color = hp_text_overlay_color(0.50);
        assert_eq!(color, HP_TEXT_INJURED_COLOR);
    }

    #[test]
    fn test_hp_text_overlay_color_critical() {
        let color = hp_text_overlay_color(0.10);
        assert_eq!(color, HP_TEXT_CRITICAL_COLOR);
    }

    #[test]
    fn test_hp_text_overlay_color_dead() {
        let color = hp_text_overlay_color(0.0);
        assert_eq!(color, HP_TEXT_DEAD_COLOR);
    }

    #[test]
    fn test_hp_text_overlay_color_boundary_healthy_threshold() {
        // At threshold, should use injured color
        let color = hp_text_overlay_color(HP_HEALTHY_THRESHOLD);
        assert_eq!(color, HP_TEXT_INJURED_COLOR);
    }

    #[test]
    fn test_hp_text_overlay_color_boundary_critical_threshold() {
        // Just above critical threshold, should use injured color
        let color = hp_text_overlay_color(HP_CRITICAL_THRESHOLD + 0.01);
        assert_eq!(color, HP_TEXT_INJURED_COLOR);
    }

    #[test]
    fn test_format_hp_display() {
        let display = format_hp_display(45, 100);
        assert_eq!(display, "HP: 45/100");
    }

    #[test]
    fn test_format_hp_display_full() {
        let display = format_hp_display(100, 100);
        assert_eq!(display, "HP: 100/100");
    }

    #[test]
    fn test_format_hp_display_zero() {
        let display = format_hp_display(0, 100);
        assert_eq!(display, "HP: 0/100");
    }
}

#[cfg(test)]
mod party_entity_tests {
    use super::*;
    use bevy::prelude::{App, Startup};

    /// `setup_party_entities` must insert a `PartyEntities` resource and spawn
    /// exactly `PARTY_MAX_SIZE` entities, each carrying the correct
    /// `CharacterEntity { party_index }`.
    #[test]
    fn test_setup_party_entities_spawns_correct_count() {
        let mut app = App::new();
        app.add_systems(Startup, setup_party_entities);
        app.update();

        let world = app.world();
        let pe = world
            .get_resource::<PartyEntities>()
            .expect("PartyEntities resource must exist after setup_party_entities");

        assert_eq!(
            pe.entities.len(),
            PARTY_MAX_SIZE,
            "resource must have exactly PARTY_MAX_SIZE slots"
        );
        assert!(
            pe.entities.iter().all(|e| e.is_some()),
            "all slots must be populated after startup"
        );
    }

    /// Each entity stored in `PartyEntities` must carry the correct
    /// `CharacterEntity` component with the matching `party_index`.
    #[test]
    fn test_setup_party_entities_correct_indices() {
        let mut app = App::new();
        app.add_systems(Startup, setup_party_entities);
        app.update();

        let world = app.world();
        let pe = world
            .get_resource::<PartyEntities>()
            .expect("PartyEntities resource must exist");

        for (expected_index, maybe_entity) in pe.entities.iter().enumerate() {
            let entity = maybe_entity.expect("slot should be populated");
            let marker = world
                .get::<CharacterEntity>(entity)
                .expect("entity must have CharacterEntity component");
            assert_eq!(
                marker.party_index, expected_index,
                "party_index must match slot position"
            );
        }
    }

    /// Running `setup_party_entities` twice (e.g. in back-to-back updates)
    /// must not panic; the resource simply gets overwritten.
    #[test]
    fn test_setup_party_entities_idempotent_resource_insert() {
        let mut app = App::new();
        // Register as a regular Update system so we can call update() twice.
        app.add_systems(Startup, setup_party_entities);
        app.update();
        // A second update should not panic (resource overwrite is safe).
        app.update();

        let world = app.world();
        let pe = world
            .get_resource::<PartyEntities>()
            .expect("PartyEntities must still exist after second update");
        assert_eq!(pe.entities.len(), PARTY_MAX_SIZE);
    }
}

mod tests {
    use super::*;

    // Helper to compare colors (Bevy Color may have floating point precision differences)
    #[allow(dead_code)]
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
        assert_eq!(display, "HP: 45/100");
    }

    #[test]
    fn test_format_hp_display_full() {
        let display = format_hp_display(100, 100);
        assert_eq!(display, "HP: 100/100");
    }

    #[test]
    fn test_format_hp_display_zero() {
        let display = format_hp_display(0, 100);
        assert_eq!(display, "HP: 0/100");
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
    fn test_clock_constants_valid() {
        // Font size must be positive
        const { assert!(CLOCK_FONT_SIZE > 0.0) }
        // Top offset must place the clock below the compass
        const { assert!(CLOCK_TOP_OFFSET > COMPASS_SIZE) }
        // Clock width must be positive
        const { assert!(CLOCK_WIDTH > 0.0) }
        // Padding must be non-negative
        const { assert!(CLOCK_PADDING >= 0.0) }
    }

    #[test]
    fn test_compass_constants_valid() {
        // Verify compass constants are defined with reasonable values
        assert_eq!(COMPASS_SIZE, 48.0);
        assert_eq!(COMPASS_BORDER_WIDTH, 2.0);
        assert_eq!(COMPASS_FONT_SIZE, 24.0);
    }

    /// Verifies that the HUD systems populate character name and HP text fields.
    ///
    /// This test:
    /// - Creates a minimal `GameState` with a single party member
    /// - Runs the HUD setup and update systems via a `bevy::prelude::App`
    /// - Checks that the HP overlay text entities for slot 0 are populated
    #[test]
    fn test_update_hud_populates_texts() {
        use super::{GlobalState, HpTextOverlay, HudPlugin};
        use crate::application::GameState;
        use crate::domain::character::{Alignment, AttributePair16, Character, Sex};
        use bevy::prelude::*;

        // Prepare GameState with a single character (slot 0)
        let mut state = GameState::new();
        let mut ch = Character::new(
            "Test Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.hp = AttributePair16 {
            base: 100,
            current: 45,
        };
        ch.portrait_id = "10".to_string();
        state.party.add_member(ch).unwrap();

        // Build an App and add minimal plugins + HUD plugin (keeps test lightweight)
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(HudPlugin);
        app.insert_resource(GlobalState(state));

        // Run startup (setup_hud) and one update (update_hud)
        app.update(); // Startup
        app.update(); // Update

        // Verify HP overlay text was populated for party slot 0
        let mut hp_overlay_found = false;
        {
            let world = app.world_mut();
            let mut q = world.query::<(&HpTextOverlay, &bevy::ui::widget::Text)>();
            for (hp_comp, text) in q.iter(world) {
                if hp_comp.party_index == 0 {
                    // Should display "HP: 45/100" (format from format_hp_display)
                    assert!(
                        text.contains("HP: 45/100"),
                        "Unexpected HP overlay text for slot 0: '{:?}'",
                        text
                    );
                    hp_overlay_found = true;
                }
            }
        }

        assert!(hp_overlay_found, "HP overlay text not populated for slot 0");
    }

    #[test]
    fn test_update_hud_handles_zero_base() {
        use super::{GlobalState, HpBarFill, HudPlugin, HP_CRITICAL_COLOR};
        use crate::application::GameState;
        use crate::domain::character::{Alignment, AttributePair16, Character, Sex};
        use bevy::prelude::*;

        // Prepare GameState with a single character (slot 0) that has a zero base HP
        let mut state = GameState::new();
        let mut ch = Character::new(
            "Zero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.hp = AttributePair16 {
            base: 0,
            current: 10,
        };
        state.party.add_member(ch).unwrap();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(HudPlugin);
        app.insert_resource(GlobalState(state));

        // Run startup and one update so HUD is populated
        app.update();
        app.update();

        // Verify HP bar width is 0% and color is critical
        {
            let world = app.world_mut();
            let mut q = world.query::<(&HpBarFill, &Node, &BackgroundColor)>();
            let mut found = false;
            for (hp_bar, node, bg_color) in q.iter(world) {
                if hp_bar.party_index == 0 {
                    assert_eq!(node.width, Val::Percent(0.0));
                    assert_eq!(*bg_color, BackgroundColor(HP_CRITICAL_COLOR));
                    found = true;
                }
            }
            assert!(found, "HP bar not found for slot 0");
        }
    }

    #[test]
    fn test_update_portraits_skip_default_handle_shows_placeholder() {
        use super::{get_portrait_color, GlobalState, HudPlugin, PortraitAssets};
        use crate::application::GameState;
        use crate::domain::character::{Alignment, Character, Sex};
        use bevy::prelude::*;

        // Prepare GameState with a character that has portrait_id = 10
        let mut state = GameState::new();
        let mut ch = Character::new(
            "Painter".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        ch.portrait_id = "10".to_string();
        state.party.add_member(ch).unwrap();

        // Build App and add HUD plugin
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.add_plugins(HudPlugin);
        app.insert_resource(GlobalState(state));

        // Insert a default handle for name '10' to simulate an AssetServer refusal
        {
            let world = app.world_mut();
            let mut portraits = world.resource_mut::<PortraitAssets>();
            portraits
                .handles_by_name
                .insert("10".to_string(), Handle::<Image>::default());
        }

        // Run startup and an update so placeholders are applied
        app.update();
        app.update();

        // Verify placeholder color is used (not transparent) and image is default
        {
            let world = app.world_mut();
            let mut portrait_query =
                world.query::<(&CharacterPortrait, &BackgroundColor, &ImageNode)>();
            let mut found_placeholder = false;
            for (portrait, bg_color, image_node) in portrait_query.iter(world) {
                if portrait.party_index == 0 {
                    assert_eq!(*bg_color, BackgroundColor(get_portrait_color("10")));
                    assert_eq!(image_node.image, Handle::<Image>::default());
                    found_placeholder = true;
                }
            }
            assert!(found_placeholder, "Portrait color not set for slot 0");
        }
    }

    /// Verifies portrait placeholder behavior and that inserting a portrait handle
    /// causes the HUD to set the portrait background to transparent and apply the image.
    #[test]
    fn test_update_portraits_placeholder_and_image() {
        use crate::application::GameState;
        use crate::domain::character::{Alignment, Character, Sex};
        use bevy::prelude::*;

        // Prepare GameState with a character that has portrait_id = 10
        let mut state = GameState::new();
        let mut ch = Character::new(
            "Painter".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        ch.portrait_id = "10".to_string();
        state.party.add_member(ch).unwrap();

        // Build App and add HUD plugin
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.add_plugins(HudPlugin);
        app.insert_resource(GlobalState(state));

        // Run startup and an update so placeholders are applied
        app.update();
        app.update();

        // Query portrait slot 0 and verify placeholder color is present (deterministic color based on portrait_id)
        // Query portrait slot 0 and verify deterministic color is applied based on the name key
        {
            let world = app.world_mut();
            let mut portrait_query =
                world.query::<(&CharacterPortrait, &BackgroundColor, &ImageNode)>();
            let mut found_placeholder = false;
            for (portrait, bg_color, _image_node) in portrait_query.iter(world) {
                if portrait.party_index == 0 {
                    // For an occupied slot with no image asset, when an explicit `portrait_id` is provided
                    // but no image asset exists, the HUD should use the deterministic placeholder color
                    // computed from the explicit portrait key.
                    assert_eq!(*bg_color, BackgroundColor(get_portrait_color("10")));
                    found_placeholder = true;
                }
            }
            assert!(found_placeholder, "Portrait color not set for slot 0");
        }

        // Insert a (test) image asset for name 'painter' and run update to apply it.
        // In some test environments the `Assets<Image>` resource may not have been
        // initialized by asset-related plugins, so ensure it exists before adding.
        let handle = {
            let world = app.world_mut();
            // Ensure the Assets<Image> resource exists in this test world.
            if world.get_resource::<Assets<Image>>().is_none() {
                world.init_resource::<Assets<Image>>();
            }
            let mut images = world.resource_mut::<Assets<Image>>();

            // Create a minimal 1x1 white image so the asset can be treated as \"loaded\"
            // by the Assets storage in the test environment.
            let img = Image::new_fill(
                bevy::render::render_resource::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                bevy::render::render_resource::TextureDimension::D2,
                &[255u8, 255u8, 255u8, 255u8],
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                bevy::asset::RenderAssetUsages::all(),
            );
            images.add(img)
        };
        {
            let world = app.world_mut();
            let mut portraits = world.resource_mut::<PortraitAssets>();
            portraits
                .handles_by_name
                .insert("painter".to_string(), handle.clone());
        }

        // Sanity checks: ensure the portrait mapping and image resource are present
        {
            let world = app.world();
            let portraits = world.resource::<PortraitAssets>();
            assert!(
                portraits.handles_by_name.contains_key("painter"),
                "Portrait mapping missing for 'painter'"
            );
            let images = world.resource::<Assets<Image>>();
            assert!(
                images.get(handle.id()).is_some(),
                "Inserted image asset not present in Assets<Image>"
            );
        }

        app.update();

        // After providing a handle, the background should be transparent and the image handle applied
        {
            let world = app.world_mut();
            let mut portrait_query =
                world.query::<(&CharacterPortrait, &BackgroundColor, &ImageNode)>();
            let mut found_image = false;
            for (portrait, bg_color, image_node) in portrait_query.iter(world) {
                if portrait.party_index == 0 {
                    assert_eq!(*bg_color, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)));
                    assert_eq!(image_node.image, handle);
                    found_image = true;
                }
            }
            assert!(
                found_image,
                "Portrait image not applied for slot 0 after inserting asset handle"
            );
        }
    }

    #[test]
    fn test_get_portrait_color_deterministic() {
        // Same portrait key should always produce same color
        let color1 = get_portrait_color("42");
        let color2 = get_portrait_color("42");
        assert!(colors_approx_equal(color1, color2));
    }

    #[test]
    fn test_get_portrait_color_different_ids() {
        // Different keys should produce different colors
        let color1 = get_portrait_color("0");
        let color2 = get_portrait_color("1");
        assert!(!colors_approx_equal(color1, color2));
    }

    #[test]
    fn test_get_portrait_color_full_range() {
        // Test boundary values on string keys
        let _color_min = get_portrait_color("0");
        let _color_max = get_portrait_color("255");
        // Should not panic and should produce valid colors
    }

    #[test]
    fn test_portrait_constants_valid() {
        // Verify portrait constants are defined with reasonable values
        assert_eq!(PORTRAIT_SIZE, 40.0);
    }

    #[test]
    fn test_update_portraits_portrait_id_priority() {
        use crate::application::GameState;
        use crate::domain::character::{Alignment, Character, Sex};
        use bevy::prelude::*;

        // Prepare GameState with a character that has portrait_id = "10"
        let mut state = GameState::new();
        let mut ch = Character::new(
            "Painter".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        ch.portrait_id = "10".to_string();
        state.party.add_member(ch).unwrap();

        // Build App and add HUD plugin
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.add_plugins(HudPlugin);
        app.insert_resource(GlobalState(state));

        // Run startup and an update so placeholders are applied
        app.update();
        app.update();

        // Insert two handles: one for "10" and one for "painter"
        let handle_10 = Handle::<Image>::default();
        let handle_painter = Handle::<Image>::default();
        {
            let world = app.world_mut();
            let mut portraits = world.resource_mut::<PortraitAssets>();
            portraits
                .handles_by_name
                .insert("10".to_string(), handle_10.clone());
            portraits
                .handles_by_name
                .insert("painter".to_string(), handle_painter.clone());
        }
        app.update();

        // Verify the portrait image applied is the one indexed by explicit portrait_id ("10")
        {
            let world = app.world_mut();
            let mut portrait_query =
                world.query::<(&CharacterPortrait, &BackgroundColor, &ImageNode)>();
            let mut found_image = false;
            for (portrait, _bg, image_node) in portrait_query.iter(world) {
                if portrait.party_index == 0 {
                    assert_eq!(image_node.image, handle_10);
                    found_image = true;
                }
            }
            assert!(
                found_image,
                "Portrait image not applied for slot 0 after inserting both keys"
            );
        }
    }

    #[test]
    fn test_scan_portraits_dir_filters_images() {
        // Inline helper inside test to avoid dead-code at top-level.
        fn scan_portraits_dir(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
            let mut results = Vec::new();
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            let ext = ext.to_lowercase();
                            if ext == "png" || ext == "jpg" || ext == "jpeg" {
                                results.push(path);
                            }
                        }
                    }
                }
            }
            results
        }

        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("portraits");
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("10.png"), b"foo").unwrap();
        std::fs::write(dir.join("sage.jpg"), b"bar").unwrap();
        std::fs::write(dir.join("README.md"), b"baz").unwrap();

        let files = scan_portraits_dir(&dir);
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(names.contains(&"10.png".to_string()));
        assert!(names.contains(&"sage.jpg".to_string()));
        assert!(!names.contains(&"README.md".to_string()));
    }
}

// ===== Clock Tests =====

#[cfg(test)]
mod minimap_tests {
    use super::*;
    use crate::application::GameState;
    use crate::domain::types::Position;
    use crate::domain::world::{Map, WallType};

    fn setup_minimap_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<Assets<Image>>();
        {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            let size = mini_map_image_size();
            let image = Image::new_fill(
                Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &vec![0; (size * size * 4) as usize],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::all(),
            );
            let handle = images.add(image);
            app.world_mut().insert_resource(MiniMapImage { handle });
        }
        app
    }

    #[test]
    fn test_mini_map_image_dimensions() {
        let app = setup_minimap_test_app();
        let mini_map = app.world().resource::<MiniMapImage>().clone();
        let images = app.world().resource::<Assets<Image>>();
        let image = images.get(&mini_map.handle).unwrap();

        assert_eq!(image.texture_descriptor.size.width, mini_map_image_size());
        assert_eq!(image.texture_descriptor.size.height, mini_map_image_size());
    }

    #[test]
    fn test_mini_map_player_pixel_is_white() {
        let mut app = setup_minimap_test_app();

        let mut state = GameState::new();
        let map = Map::new(1, "Mini Map".to_string(), "Test".to_string(), 13, 13);
        state.world.add_map(map);
        state.world.set_current_map(1);
        state.world.set_party_position(Position::new(5, 5));
        crate::domain::world::mark_visible_area(
            &mut state.world,
            Position::new(5, 5),
            MINI_MAP_VIEWPORT_RADIUS,
        );
        app.insert_resource(GlobalState(state));

        app.world_mut()
            .run_system_cached(update_mini_map)
            .expect("update_mini_map system should run in test");

        let mini_map = app.world().resource::<MiniMapImage>().clone();
        let images = app.world().resource::<Assets<Image>>();
        let image = images.get(&mini_map.handle).unwrap();
        let data = image.data.as_ref().unwrap();

        let center_pixel = (mini_map_viewport_diameter() / 2) * mini_map_pixels_per_tile()
            + (mini_map_pixels_per_tile() / 2);
        let offset = mini_map_pixel_offset(center_pixel, center_pixel, mini_map_image_size());

        assert_eq!(&data[offset..offset + 4], &MINI_MAP_PLAYER);
    }

    #[test]
    fn test_mini_map_unvisited_is_transparent() {
        let mut app = setup_minimap_test_app();

        let mut state = GameState::new();
        let map = Map::new(1, "Mini Map".to_string(), "Test".to_string(), 13, 13);
        state.world.add_map(map);
        state.world.set_current_map(1);
        state.world.set_party_position(Position::new(5, 5));
        app.insert_resource(GlobalState(state));

        app.world_mut()
            .run_system_cached(update_mini_map)
            .expect("update_mini_map system should run in test");

        let mini_map = app.world().resource::<MiniMapImage>().clone();
        let images = app.world().resource::<Assets<Image>>();
        let image = images.get(&mini_map.handle).unwrap();
        let data = image.data.as_ref().unwrap();

        let pixel_x = (mini_map_viewport_diameter() / 2 + 1) * mini_map_pixels_per_tile();
        let pixel_y = (mini_map_viewport_diameter() / 2) * mini_map_pixels_per_tile();
        let offset = mini_map_pixel_offset(pixel_x, pixel_y, mini_map_image_size());

        assert_eq!(data[offset + 3], 0);
    }

    #[test]
    fn test_mini_map_visited_wall_color() {
        let mut app = setup_minimap_test_app();

        let mut state = GameState::new();
        let mut map = Map::new(1, "Mini Map".to_string(), "Test".to_string(), 13, 13);
        let wall_pos = Position::new(6, 5);
        if let Some(tile) = map.get_tile_mut(wall_pos) {
            tile.wall_type = WallType::Normal;
            tile.blocked = true;
            tile.mark_visited();
        }
        state.world.add_map(map);
        state.world.set_current_map(1);
        state.world.set_party_position(Position::new(5, 5));
        if let Some(tile) = state
            .world
            .get_current_map_mut()
            .and_then(|m| m.get_tile_mut(Position::new(5, 5)))
        {
            tile.mark_visited();
        }
        app.insert_resource(GlobalState(state));

        app.world_mut()
            .run_system_cached(update_mini_map)
            .expect("update_mini_map system should run in test");

        let mini_map = app.world().resource::<MiniMapImage>().clone();
        let images = app.world().resource::<Assets<Image>>();
        let image = images.get(&mini_map.handle).unwrap();
        let data = image.data.as_ref().unwrap();

        let tile_x = mini_map_viewport_diameter() / 2 + 1;
        let tile_y = mini_map_viewport_diameter() / 2;
        let pixel_x = tile_x * mini_map_pixels_per_tile();
        let pixel_y = tile_y * mini_map_pixels_per_tile();
        let offset = mini_map_pixel_offset(pixel_x, pixel_y, mini_map_image_size());

        assert_eq!(&data[offset..offset + 4], &MINI_MAP_WALL);
    }

    #[test]
    fn test_mini_map_poi_dot_rendered() {
        let mut app = setup_minimap_test_app();

        let mut state = GameState::new();
        let mut map = Map::new(1, "Mini Map".to_string(), "Test".to_string(), 13, 13);
        let merchant_pos = Position::new(6, 5);

        if let Some(tile) = map.get_tile_mut(merchant_pos) {
            tile.mark_visited();
        }

        if let Some(tile) = map.get_tile_mut(Position::new(5, 5)) {
            tile.mark_visited();
        }

        map.npc_placements
            .push(crate::domain::world::NpcPlacement::new(
                "merchant_alder",
                merchant_pos,
            ));

        state.world.add_map(map);
        state.world.set_current_map(1);
        state.world.set_party_position(Position::new(5, 5));
        app.insert_resource(GlobalState(state));

        app.world_mut()
            .run_system_cached(update_mini_map)
            .expect("update_mini_map system should run in test");

        let mini_map = app.world().resource::<MiniMapImage>().clone();
        let images = app.world().resource::<Assets<Image>>();
        let image = images.get(&mini_map.handle).unwrap();
        let data = image.data.as_ref().unwrap();

        let tile_x = mini_map_viewport_diameter() / 2 + 1;
        let tile_y = mini_map_viewport_diameter() / 2;
        let pixel_x = tile_x * mini_map_pixels_per_tile() + (mini_map_pixels_per_tile() / 2) - 1;
        let pixel_y = tile_y * mini_map_pixels_per_tile() + (mini_map_pixels_per_tile() / 2) - 1;
        let offset = mini_map_pixel_offset(pixel_x, pixel_y, mini_map_image_size());

        assert_eq!(&data[offset..offset + 4], &POI_MERCHANT_COLOR);
    }

    #[test]
    fn test_mini_map_hidden_when_show_minimap_false() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<Assets<Image>>();

        let size = mini_map_image_size();
        let image = Image::new_fill(
            Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &vec![0; (size * size * 4) as usize],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::all(),
        );
        let handle = {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            images.add(image)
        };
        app.world_mut().insert_resource(MiniMapImage { handle });

        app.world_mut().spawn((Node::default(), MiniMapRoot));

        let mut state = GameState::new();
        state.config.graphics.show_minimap = false;
        let map = Map::new(1, "Mini Map".to_string(), "Test".to_string(), 13, 13);
        state.world.add_map(map);
        state.world.set_current_map(1);
        state.world.set_party_position(Position::new(5, 5));
        app.insert_resource(GlobalState(state));

        app.world_mut()
            .run_system_cached(update_mini_map)
            .expect("update_mini_map system should run in test");

        let world = app.world_mut();
        let mut query = world.query::<(&Node, &MiniMapRoot)>();
        let (node, _) = query
            .single(world)
            .expect("MiniMapRoot entity should exist in test world");

        assert_eq!(node.display, Display::None);
    }

    #[test]
    fn test_mini_map_quest_objective_poi_dot_rendered() {
        let mut app = setup_minimap_test_app();

        let mut state = GameState::new();
        let mut map = Map::new(1, "Mini Map".to_string(), "Test".to_string(), 13, 13);
        let objective_pos = Position::new(6, 5);

        if let Some(tile) = map.get_tile_mut(objective_pos) {
            tile.mark_visited();
        }

        if let Some(tile) = map.get_tile_mut(Position::new(5, 5)) {
            tile.mark_visited();
        }

        state.world.add_map(map);
        state.world.set_current_map(1);
        state.world.set_party_position(Position::new(5, 5));

        let mut quest = crate::application::Quest::new(
            "42".to_string(),
            "Find the marker".to_string(),
            "Reach the marked tile".to_string(),
        );
        quest.add_objective_with_location(
            "Reach the objective".to_string(),
            Some(1),
            Some(objective_pos),
        );
        state.quests.add_quest(quest);

        app.insert_resource(GlobalState(state));

        app.world_mut()
            .run_system_cached(update_mini_map)
            .expect("update_mini_map system should run in test");

        let mini_map = app.world().resource::<MiniMapImage>().clone();
        let images = app.world().resource::<Assets<Image>>();
        let image = images.get(&mini_map.handle).unwrap();
        let data = image.data.as_ref().unwrap();

        let tile_x = mini_map_viewport_diameter() / 2 + 1;
        let tile_y = mini_map_viewport_diameter() / 2;
        let pixel_x = tile_x * mini_map_pixels_per_tile() + (mini_map_pixels_per_tile() / 2) - 1;
        let pixel_y = tile_y * mini_map_pixels_per_tile() + (mini_map_pixels_per_tile() / 2) - 1;
        let offset = mini_map_pixel_offset(pixel_x, pixel_y, mini_map_image_size());

        assert_eq!(&data[offset..offset + 4], &POI_QUEST_COLOR);
    }
}

#[cfg(test)]
mod automap_tests {
    use super::*;
    use crate::application::GameState;
    use crate::domain::types::Position;
    use crate::domain::world::{Map, TerrainType};

    fn setup_automap_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<Assets<Image>>();
        {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            let size = AUTOMAP_MAX_IMAGE_SIZE_PX;
            let image = Image::new_fill(
                Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &vec![0; (size * size * 4) as usize],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::all(),
            );
            let handle = images.add(image);
            app.world_mut().insert_resource(AutomapImage { handle });
        }

        app.world_mut()
            .spawn((Node::default(), ImageNode::default(), AutomapCanvas));

        app
    }

    #[test]
    fn test_automap_image_unvisited_is_black() {
        let mut app = setup_automap_test_app();

        let mut state = GameState::new();
        let map = Map::new(1, "Automap".to_string(), "Test".to_string(), 8, 8);
        state.world.add_map(map);
        state.world.set_current_map(1);
        state.world.set_party_position(Position::new(2, 2));
        state.mode = GameMode::Automap;
        app.insert_resource(GlobalState(state));

        app.world_mut()
            .run_system_cached(update_automap_image)
            .expect("update_automap_image system should run in test");

        let automap = app.world().resource::<AutomapImage>().clone();
        let images = app.world().resource::<Assets<Image>>();
        let image = images.get(&automap.handle).unwrap();
        let data = image.data.as_ref().unwrap();
        let offset = automap_pixel_offset(0, 0, image.texture_descriptor.size.width);

        assert_eq!(&data[offset..offset + 4], &AUTOMAP_UNVISITED);
    }

    #[test]
    fn test_automap_image_visited_floor_is_gray() {
        let mut app = setup_automap_test_app();

        let mut state = GameState::new();
        let mut map = Map::new(1, "Automap".to_string(), "Test".to_string(), 8, 8);
        let floor_pos = Position::new(1, 1);
        if let Some(tile) = map.get_tile_mut(floor_pos) {
            tile.terrain = TerrainType::Ground;
            tile.mark_visited();
        }
        state.world.add_map(map);
        state.world.set_current_map(1);
        state.world.set_party_position(Position::new(7, 7));
        state.mode = GameMode::Automap;
        app.insert_resource(GlobalState(state));

        app.world_mut()
            .run_system_cached(update_automap_image)
            .expect("update_automap_image system should run in test");

        let automap = app.world().resource::<AutomapImage>().clone();
        let images = app.world().resource::<Assets<Image>>();
        let image = images.get(&automap.handle).unwrap();
        let tile_px =
            (AUTOMAP_MAX_IMAGE_SIZE_PX / 8).clamp(AUTOMAP_MIN_TILE_PX, AUTOMAP_MAX_TILE_PX);
        let pixel_x = tile_px;
        let pixel_y = tile_px;
        let data = image.data.as_ref().unwrap();
        let offset = automap_pixel_offset(pixel_x, pixel_y, image.texture_descriptor.size.width);

        assert_eq!(&data[offset..offset + 4], &AUTOMAP_VISITED_FLOOR);
    }
}

#[cfg(test)]
mod clock_tests {
    use super::*;
    use crate::domain::types::TimeOfDay;

    // ── format_clock_time ────────────────────────────────────────────────────

    /// hour=0, minute=0 → "00:00"  (midnight — both fields must be zero-padded)
    #[test]
    fn test_clock_format_midnight() {
        assert_eq!(format_clock_time(0, 0), "00:00");
    }

    /// hour=12, minute=5 → "12:05"  (single-digit minute is zero-padded)
    #[test]
    fn test_clock_format_noon() {
        assert_eq!(format_clock_time(12, 5), "12:05");
    }

    /// hour=9, minute=0 → "09:00"  (single-digit hour is zero-padded)
    #[test]
    fn test_clock_format_single_digit_hour() {
        assert_eq!(format_clock_time(9, 0), "09:00");
    }

    /// hour=23, minute=59 → "23:59"  (end-of-day boundary)
    #[test]
    fn test_clock_format_end_of_day() {
        assert_eq!(format_clock_time(23, 59), "23:59");
    }

    /// hour=0, minute=1 → "00:01"  (both zero-padded)
    #[test]
    fn test_clock_format_zero_hour_one_minute() {
        assert_eq!(format_clock_time(0, 1), "00:01");
    }

    /// hour=6, minute=30 → "06:30"  (dawn start time — used in GameState default)
    #[test]
    fn test_clock_format_dawn_default() {
        assert_eq!(format_clock_time(6, 30), "06:30");
    }

    /// Verify all valid hours (0-23) produce correctly formatted strings
    #[test]
    fn test_clock_format_all_hours_produce_valid_strings() {
        for hour in 0u8..24 {
            let formatted = format_clock_time(hour, 0);
            // Must be exactly 5 characters: "HH:MM"
            assert_eq!(
                formatted.len(),
                5,
                "format_clock_time({}, 0) produced '{}' — expected 5 chars",
                hour,
                formatted
            );
            assert!(
                formatted.contains(':'),
                "format_clock_time({}, 0) missing colon",
                hour
            );
        }
    }

    /// Verify all valid minutes (0-59) produce correctly formatted strings
    #[test]
    fn test_clock_format_all_minutes_produce_valid_strings() {
        for minute in 0u8..60 {
            let formatted = format_clock_time(0, minute);
            assert_eq!(
                formatted.len(),
                5,
                "format_clock_time(0, {}) produced '{}' — expected 5 chars",
                minute,
                formatted
            );
        }
    }

    // ── format_clock_date ────────────────────────────────────────────────────

    /// (1, 1, 1) → "Y1 M1 D1"  (first day of the first month of year 1)
    #[test]
    fn test_format_clock_date_defaults() {
        assert_eq!(format_clock_date(1, 1, 1), "Y1 M1 D1");
    }

    /// (4, 12, 30) → "Y4 M12 D30"  (large multi-digit values)
    #[test]
    fn test_format_clock_date_large_values() {
        assert_eq!(format_clock_date(4, 12, 30), "Y4 M12 D30");
    }

    /// (1, 6, 15) → "Y1 M6 D15"  (mid-year, mid-month)
    #[test]
    fn test_clock_date_display_mid_year() {
        assert_eq!(format_clock_date(1, 6, 15), "Y1 M6 D15");
    }

    /// (2, 1, 1) → "Y2 M1 D1"  (start of year 2)
    #[test]
    fn test_clock_date_display_year_two() {
        assert_eq!(format_clock_date(2, 1, 1), "Y2 M1 D1");
    }

    /// (1, 12, 30) → "Y1 M12 D30"  (last day of year 1)
    #[test]
    fn test_clock_date_display_last_day_of_year() {
        assert_eq!(format_clock_date(1, 12, 30), "Y1 M12 D30");
    }

    /// (u32::MAX, u32::MAX, u32::MAX) must not panic
    #[test]
    fn test_clock_date_display_max() {
        let result = format_clock_date(u32::MAX, u32::MAX, u32::MAX);
        assert!(
            result.starts_with("Y"),
            "format_clock_date(MAX,MAX,MAX) should start with 'Y', got '{result}'"
        );
    }

    // ── clock_text_color ─────────────────────────────────────────────────────

    /// Night period must return CLOCK_NIGHT_TEXT_COLOR
    #[test]
    fn test_clock_text_color_night_returns_night_color() {
        let color = clock_text_color(TimeOfDay::Night);
        assert_eq!(color, CLOCK_NIGHT_TEXT_COLOR);
    }

    /// Evening period must return CLOCK_NIGHT_TEXT_COLOR (it is_dark())
    #[test]
    fn test_clock_text_color_evening_returns_night_color() {
        let color = clock_text_color(TimeOfDay::Evening);
        assert_eq!(color, CLOCK_NIGHT_TEXT_COLOR);
    }

    /// Dawn period must return CLOCK_DAY_TEXT_COLOR
    #[test]
    fn test_clock_text_color_dawn_returns_day_color() {
        let color = clock_text_color(TimeOfDay::Dawn);
        assert_eq!(color, CLOCK_DAY_TEXT_COLOR);
    }

    /// Morning period must return CLOCK_DAY_TEXT_COLOR
    #[test]
    fn test_clock_text_color_morning_returns_day_color() {
        let color = clock_text_color(TimeOfDay::Morning);
        assert_eq!(color, CLOCK_DAY_TEXT_COLOR);
    }

    /// Afternoon period must return CLOCK_DAY_TEXT_COLOR
    #[test]
    fn test_clock_text_color_afternoon_returns_day_color() {
        let color = clock_text_color(TimeOfDay::Afternoon);
        assert_eq!(color, CLOCK_DAY_TEXT_COLOR);
    }

    /// Dusk period must return CLOCK_DAY_TEXT_COLOR (it is NOT dark)
    #[test]
    fn test_clock_text_color_dusk_returns_day_color() {
        let color = clock_text_color(TimeOfDay::Dusk);
        assert_eq!(color, CLOCK_DAY_TEXT_COLOR);
    }

    /// clock_text_color must agree with TimeOfDay::is_dark() for every period
    #[test]
    fn test_clock_text_color_agrees_with_is_dark_for_all_periods() {
        let all_periods = [
            TimeOfDay::Dawn,
            TimeOfDay::Morning,
            TimeOfDay::Afternoon,
            TimeOfDay::Dusk,
            TimeOfDay::Evening,
            TimeOfDay::Night,
        ];
        for period in all_periods {
            let color = clock_text_color(period);
            if period.is_dark() {
                assert_eq!(
                    color, CLOCK_NIGHT_TEXT_COLOR,
                    "{:?} is_dark() but did not return CLOCK_NIGHT_TEXT_COLOR",
                    period
                );
            } else {
                assert_eq!(
                    color, CLOCK_DAY_TEXT_COLOR,
                    "{:?} is not dark but did not return CLOCK_DAY_TEXT_COLOR",
                    period
                );
            }
        }
    }

    // ── constant sanity checks ───────────────────────────────────────────────

    /// CLOCK_FONT_SIZE must be a positive, legible value
    #[test]
    fn test_clock_font_size_is_positive() {
        const { assert!(CLOCK_FONT_SIZE > 0.0) }
    }

    /// CLOCK_TOP_OFFSET must be strictly larger than COMPASS_SIZE so the
    /// clock panel does not overlap the compass widget
    #[test]
    fn test_clock_top_offset_places_clock_below_compass() {
        const { assert!(CLOCK_TOP_OFFSET > COMPASS_SIZE) }
    }

    /// CLOCK_WIDTH must be a positive value
    #[test]
    fn test_clock_width_is_positive() {
        const { assert!(CLOCK_WIDTH > 0.0) }
    }

    /// CLOCK_PADDING must be non-negative
    #[test]
    fn test_clock_padding_is_non_negative() {
        const { assert!(CLOCK_PADDING >= 0.0) }
    }

    /// Night and day text colors must be distinct (different visual tints)
    #[test]
    fn test_clock_night_and_day_colors_are_distinct() {
        let night = CLOCK_NIGHT_TEXT_COLOR.to_srgba();
        let day = CLOCK_DAY_TEXT_COLOR.to_srgba();
        // At least one channel must differ noticeably
        let differs = (night.red - day.red).abs() > 0.05
            || (night.green - day.green).abs() > 0.05
            || (night.blue - day.blue).abs() > 0.05;
        assert!(
            differs,
            "CLOCK_NIGHT_TEXT_COLOR and CLOCK_DAY_TEXT_COLOR should be visually distinct"
        );
    }

    /// All clock color constants must be fully opaque (alpha == 1.0)
    #[test]
    fn test_clock_colors_are_opaque() {
        for (name, color) in [
            ("CLOCK_TEXT_COLOR", CLOCK_TEXT_COLOR),
            ("CLOCK_NIGHT_TEXT_COLOR", CLOCK_NIGHT_TEXT_COLOR),
            ("CLOCK_DAY_TEXT_COLOR", CLOCK_DAY_TEXT_COLOR),
        ] {
            let alpha = color.to_srgba().alpha;
            assert!(
                (alpha - 1.0).abs() < f32::EPSILON,
                "{} must have alpha 1.0, got {}",
                name,
                alpha
            );
        }
    }

    /// Background and border constants must have positive alpha (visible)
    #[test]
    fn test_clock_background_and_border_colors_are_visible() {
        assert!(
            CLOCK_BACKGROUND_COLOR.to_srgba().alpha > 0.0,
            "CLOCK_BACKGROUND_COLOR must not be fully transparent"
        );
        assert!(
            CLOCK_BORDER_COLOR.to_srgba().alpha > 0.0,
            "CLOCK_BORDER_COLOR must not be fully transparent"
        );
    }

    // ── Bevy integration: clock widget spawned and updated ───────────────────

    /// After startup the clock widget must exist in the world with initial
    /// placeholder text ("00:00" for time, "Y1 M1 D1" for date).
    #[test]
    fn test_clock_widget_spawned_on_startup() {
        use crate::application::GameState;
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(HudPlugin);
        app.insert_resource(GlobalState(GameState::new()));

        // Startup systems run on the first update
        app.update();

        let world = app.world_mut();

        // ClockTimeText entity must exist
        let mut time_q = world.query_filtered::<&Text, With<ClockTimeText>>();
        let time_count = time_q.iter(world).count();
        assert_eq!(time_count, 1, "Expected exactly one ClockTimeText entity");

        // ClockDateText entity must exist
        let mut date_q = world.query_filtered::<&Text, With<ClockDateText>>();
        let date_count = date_q.iter(world).count();
        assert_eq!(date_count, 1, "Expected exactly one ClockDateText entity");

        // ClockRoot entity must exist
        let mut root_q = world.query_filtered::<Entity, With<ClockRoot>>();
        let root_count = root_q.iter(world).count();
        assert_eq!(root_count, 1, "Expected exactly one ClockRoot entity");
    }

    /// After startup + one update, the clock text must reflect the default
    /// GameState time (Year 1, Month 1, Day 1, 06:00 — the canonical starting time).
    #[test]
    fn test_clock_widget_shows_default_game_time() {
        use crate::application::GameState;
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(HudPlugin);
        app.insert_resource(GlobalState(GameState::new()));

        app.update(); // Startup
        app.update(); // Update (runs update_clock)

        let world = app.world_mut();

        // Time text should reflect the default start time
        let mut time_q = world.query_filtered::<&Text, With<ClockTimeText>>();
        let time_text_ref = time_q.single(world).unwrap();
        let time_text: &str = time_text_ref;
        // GameState::new() starts at 06:00
        assert!(
            time_text.contains("06:00"),
            "ClockTimeText should contain '06:00' at default start, got '{}'",
            time_text
        );

        // Date text should say "Y1 M1 D1"
        let mut date_q = world.query_filtered::<&Text, With<ClockDateText>>();
        let date_text_ref = date_q.single(world).unwrap();
        let date_text: &str = date_text_ref;
        assert!(
            date_text.contains("Y1 M1 D1"),
            "ClockDateText should contain 'Y1 M1 D1' at default start, got '{}'",
            date_text
        );
    }

    /// Advancing time in the GameState must be reflected in the clock widget
    /// on the next update cycle.
    #[test]
    fn test_clock_widget_updates_after_time_advance() {
        use crate::application::GameState;
        use bevy::prelude::*;

        let mut state = GameState::new();
        // Advance 18 hours from 06:00 → 00:00 on Day 2
        state.time.advance_hours(18);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(HudPlugin);
        app.insert_resource(GlobalState(state));

        app.update(); // Startup
        app.update(); // Update

        let world = app.world_mut();

        let mut time_q = world.query_filtered::<&Text, With<ClockTimeText>>();
        let time_text_ref = time_q.single(world).unwrap();
        let time_text: &str = time_text_ref;
        assert!(
            time_text.contains("00:00"),
            "ClockTimeText should be '00:00' after 18h advance from 06:00, got '{}'",
            time_text
        );

        // After 18h from Day 1, 06:00 → Day 2, 00:00 — still Year 1, Month 1
        let mut date_q = world.query_filtered::<&Text, With<ClockDateText>>();
        let date_text_ref = date_q.single(world).unwrap();
        let date_text: &str = date_text_ref;
        assert!(
            date_text.contains("Y1 M1 D2"),
            "ClockDateText should contain 'Y1 M1 D2' after rolling over midnight, got '{}'",
            date_text
        );
    }
}
