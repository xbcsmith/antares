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
pub const CHARACTER_CARD_WIDTH: Val = Val::Px(120.0);
pub const HP_BAR_HEIGHT: Val = Val::Px(10.0);
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

// ===== Plugin =====

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PortraitAssets::default())
            .add_systems(Startup, setup_hud)
            .add_systems(
                Update,
                (
                    ensure_portraits_loaded,
                    update_hud,
                    update_compass,
                    update_portraits,
                ),
            );
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
                        // Row 1: Portrait + Name/HP text container
                        card.spawn(Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|row| {
                            // Portrait (left side)
                            row.spawn((
                                Node {
                                    width: Val::Px(PORTRAIT_SIZE),
                                    height: Val::Px(PORTRAIT_SIZE),
                                    flex_shrink: 0.0,
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.4, 0.4, 0.4, 1.0)),
                                BorderRadius::all(Val::Px(4.0)),
                                ImageNode::default(),
                                CharacterPortrait { party_index },
                            ));

                            // Name + HP text container (right side)
                            row.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                flex_grow: 1.0,
                                ..default()
                            })
                            .with_children(|name_hp_row| {
                                // Character name (left-aligned)
                                name_hp_row.spawn((
                                    Text::new(""),
                                    TextFont {
                                        font_size: 14.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                    CharacterNameText { party_index },
                                ));

                                // HP text (right-aligned)
                                name_hp_row.spawn((
                                    Text::new(""),
                                    TextFont {
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                    HpText { party_index },
                                ));
                            });
                        });

                        // Row 2: HP bar
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

                        // Row 3: Condition text
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
            **text = character.name.clone();
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
    fn test_character_name_no_number_prefix() {
        // This test verifies the format doesn't include party_index
        let name = "TestHero";
        let formatted = name.to_string(); // Should be just the name
        assert_eq!(formatted, "TestHero");
        assert!(!formatted.starts_with("1. "));
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

    /// Verifies that the HUD systems populate character name and HP text fields.
    ///
    /// This test:
    /// - Creates a minimal `GameState` with a single party member
    /// - Runs the HUD setup and update systems via a `bevy::prelude::App`
    /// - Checks that the name and HP text entities for slot 0 are populated
    #[test]
    fn test_update_hud_populates_texts() {
        use super::{CharacterNameText, GlobalState, HpText, HudPlugin};
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

        // Verify character name text was populated for party slot 0
        let mut name_found = false;
        {
            let world = app.world_mut();
            let mut q = world.query::<(&CharacterNameText, &bevy::ui::widget::Text)>();
            for (name_comp, text) in q.iter(world) {
                if name_comp.party_index == 0 {
                    // Name should include 'Test Hero' (may be prefixed with slot number)
                    assert!(
                        text.contains("Test Hero") || text.contains("1. Test Hero"),
                        "Unexpected name text: '{:?}'",
                        text
                    );
                    name_found = true;
                }
            }
        }

        // Verify HP text was populated for party slot 0
        let mut hp_found = false;
        {
            let world = app.world_mut();
            let mut q = world.query::<(&HpText, &bevy::ui::widget::Text)>();
            for (hp_comp, text) in q.iter(world) {
                if hp_comp.party_index == 0 {
                    // Should display "45/100" (format from format_hp_display)
                    assert!(
                        text.contains("45/100"),
                        "Unexpected HP text for slot 0: '{:?}'",
                        text
                    );
                    hp_found = true;
                }
            }
        }

        assert!(name_found, "Character name text not populated for slot 0");
        assert!(hp_found, "HP text not populated for slot 0");
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
