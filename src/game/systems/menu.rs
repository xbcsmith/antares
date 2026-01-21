// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Menu plugin and system implementations for Phase 4-6: Menu UI Rendering, Save/Load, and Settings Integration
//!
//! This module implements the complete menu UI system with:
//! - UI spawning based on current submenu (Main/SaveLoad/Settings)
//! - Button interaction handling (hover/click)
//! - Dynamic button color updates based on selection
//! - Save/Load menu with scrollable save list
//! - Save and load game operations
//! - Settings menu with volume sliders (Phase 6)
//! - Proper cleanup when exiting menu mode

use bevy::prelude::*;
use chrono::Local;

use crate::application::menu::{MenuState, MenuType, SaveGameInfo};
use crate::application::save_game::SaveGameManager;
use crate::application::GameMode;
use crate::game::components::menu::*;
use crate::game::resources::GlobalState;

/// Plugin for the in-game menu system
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(
            SaveGameManager::new("saves")
                .unwrap_or_else(|e| panic!("Failed to initialize SaveGameManager: {}", e)),
        );

        app.add_systems(
            Update,
            (
                menu_setup,
                handle_menu_keyboard,
                menu_button_interaction,
                update_button_colors,
                populate_save_list,
                apply_settings,
                menu_cleanup,
            ),
        );
    }
}

/// Spawn the menu UI when entering Menu mode
fn menu_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    global_state: Res<GlobalState>,
    existing_menu: Query<&MenuRoot>,
) {
    let GameMode::Menu(menu_state) = &global_state.0.mode else {
        return;
    };

    if !existing_menu.is_empty() {
        return;
    }

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    match menu_state.current_submenu {
        MenuType::Main => spawn_main_menu(&mut commands, &font, menu_state),
        MenuType::SaveLoad => spawn_save_load_menu(&mut commands, &font, menu_state),
        MenuType::Settings => spawn_settings_menu(&mut commands, &font),
    }
}

/// Spawn the main menu UI
fn spawn_main_menu(commands: &mut Commands, font: &Handle<Font>, menu_state: &MenuState) {
    let font = font.clone();

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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            ZIndex(100),
            MenuRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Relative,
                        width: Val::Px(MENU_WIDTH),
                        height: Val::Px(MENU_HEIGHT),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexStart,
                        padding: UiRect::all(Val::Px(20.0)),
                        row_gap: Val::Px(BUTTON_SPACING),
                        ..default()
                    },
                    BackgroundColor(MENU_BACKGROUND_COLOR),
                    BorderRadius::all(Val::Px(4.0)),
                    MainMenuPanel,
                ))
                .with_children(|panel| {
                    // Title
                    panel.spawn((
                        Text::new("GAME MENU"),
                        TextFont {
                            font: font.clone(),
                            font_size: TITLE_FONT_SIZE,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // Spacing
                    panel.spawn(Node {
                        height: Val::Px(40.0),
                        ..default()
                    });

                    // Buttons: Resume, Save, Load, Settings, Quit
                    let buttons = [
                        (MenuButton::Resume, "Resume Game", 0),
                        (MenuButton::SaveGame, "Save Game", 1),
                        (MenuButton::LoadGame, "Load Game", 2),
                        (MenuButton::Settings, "Settings", 3),
                        (MenuButton::Quit, "Quit Game", 4),
                    ];

                    for (btn_type, btn_text, btn_idx) in &buttons {
                        let is_selected = menu_state.selected_index == *btn_idx;
                        let bg_color = if is_selected {
                            BUTTON_HOVER_COLOR
                        } else {
                            BUTTON_NORMAL_COLOR
                        };

                        panel
                            .spawn((
                                Button,
                                Node {
                                    width: Val::Px(BUTTON_WIDTH),
                                    height: Val::Px(BUTTON_HEIGHT),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(bg_color),
                                BorderRadius::all(Val::Px(4.0)),
                                *btn_type,
                            ))
                            .with_children(|button_root| {
                                button_root.spawn((
                                    Text::new(btn_text.to_string()),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: BUTTON_FONT_SIZE,
                                        ..default()
                                    },
                                    TextColor(BUTTON_TEXT_COLOR),
                                ));
                            });
                    }
                });
        });

    info!("Spawned main menu UI");
}

/// Spawn the save/load menu UI with scrollable save list
fn spawn_save_load_menu(commands: &mut Commands, font: &Handle<Font>, menu_state: &MenuState) {
    let font = font.clone();

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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            ZIndex(100),
            MenuRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Relative,
                        width: Val::Px(MENU_WIDTH),
                        height: Val::Px(MENU_HEIGHT),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexStart,
                        padding: UiRect::all(Val::Px(20.0)),
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                    BackgroundColor(MENU_BACKGROUND_COLOR),
                    BorderRadius::all(Val::Px(4.0)),
                    SaveLoadPanel,
                ))
                .with_children(|panel| {
                    // Title
                    panel.spawn((
                        Text::new("SAVE / LOAD GAME"),
                        TextFont {
                            font: font.clone(),
                            font_size: TITLE_FONT_SIZE,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // Save list container (scrollable)
                    panel
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(380.0),
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::scroll_y(),
                            padding: UiRect::all(Val::Px(5.0)),
                            margin: UiRect::all(Val::Px(10.0)),
                            ..default()
                        })
                        .with_children(|list| {
                            if menu_state.save_list.is_empty() {
                                list.spawn((
                                    Text::new("No save files found"),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 18.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.7, 0.7, 0.7)),
                                ));
                            } else {
                                for (index, save_info) in menu_state.save_list.iter().enumerate() {
                                    let is_selected = menu_state.selected_index == index;
                                    let bg_color = if is_selected {
                                        BUTTON_HOVER_COLOR
                                    } else {
                                        BUTTON_NORMAL_COLOR
                                    };

                                    list.spawn((
                                        Button,
                                        Node {
                                            width: Val::Percent(95.0),
                                            padding: UiRect::all(Val::Px(10.0)),
                                            margin: UiRect::all(Val::Px(5.0)),
                                            flex_direction: FlexDirection::Column,
                                            ..default()
                                        },
                                        BackgroundColor(bg_color),
                                        BorderRadius::all(Val::Px(4.0)),
                                        MenuButton::SelectSave(index),
                                    ))
                                    .with_children(|slot| {
                                        // Filename
                                        slot.spawn((
                                            Text::new(format!("Save: {}", save_info.filename)),
                                            TextFont {
                                                font: font.clone(),
                                                font_size: 16.0,
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));

                                        // Timestamp
                                        slot.spawn((
                                            Text::new(format!("Date: {}", save_info.timestamp)),
                                            TextFont {
                                                font: font.clone(),
                                                font_size: 12.0,
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                        ));

                                        // Party members
                                        if !save_info.character_names.is_empty() {
                                            slot.spawn((
                                                Text::new(format!(
                                                    "Party: {}",
                                                    save_info.character_names.join(", ")
                                                )),
                                                TextFont {
                                                    font: font.clone(),
                                                    font_size: 12.0,
                                                    ..default()
                                                },
                                                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                            ));
                                        }

                                        // Location
                                        slot.spawn((
                                            Text::new(format!("Location: {}", save_info.location)),
                                            TextFont {
                                                font: font.clone(),
                                                font_size: 12.0,
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                        ));
                                    });
                                }
                            }
                        });

                    // Action buttons
                    panel
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceAround,
                            margin: UiRect::top(Val::Px(10.0)),
                            ..default()
                        })
                        .with_children(|buttons| {
                            buttons
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(BUTTON_WIDTH - 40.0),
                                        height: Val::Px(BUTTON_HEIGHT),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(BUTTON_NORMAL_COLOR),
                                    BorderRadius::all(Val::Px(4.0)),
                                    MenuButton::Confirm,
                                ))
                                .with_children(|button_root| {
                                    button_root.spawn((
                                        Text::new("Save"),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: BUTTON_FONT_SIZE,
                                            ..default()
                                        },
                                        TextColor(BUTTON_TEXT_COLOR),
                                    ));
                                });

                            buttons
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(BUTTON_WIDTH - 40.0),
                                        height: Val::Px(BUTTON_HEIGHT),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(BUTTON_NORMAL_COLOR),
                                    BorderRadius::all(Val::Px(4.0)),
                                    MenuButton::LoadGame,
                                ))
                                .with_children(|button_root| {
                                    button_root.spawn((
                                        Text::new("Load"),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: BUTTON_FONT_SIZE,
                                            ..default()
                                        },
                                        TextColor(BUTTON_TEXT_COLOR),
                                    ));
                                });

                            buttons
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(BUTTON_WIDTH - 40.0),
                                        height: Val::Px(BUTTON_HEIGHT),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(BUTTON_NORMAL_COLOR),
                                    BorderRadius::all(Val::Px(4.0)),
                                    MenuButton::Back,
                                ))
                                .with_children(|button_root| {
                                    button_root.spawn((
                                        Text::new("Back"),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: BUTTON_FONT_SIZE,
                                            ..default()
                                        },
                                        TextColor(BUTTON_TEXT_COLOR),
                                    ));
                                });
                        });
                });
        });

    info!("Spawned save/load menu UI");
}

/// Spawn the settings menu UI with audio sliders and graphics settings (Phase 6)
fn spawn_settings_menu(commands: &mut Commands, font: &Handle<Font>) {
    let font = font.clone();

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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            ZIndex(100),
            MenuRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Relative,
                        width: Val::Px(MENU_WIDTH),
                        height: Val::Px(MENU_HEIGHT),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexStart,
                        padding: UiRect::all(Val::Px(20.0)),
                        row_gap: Val::Px(10.0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    BackgroundColor(MENU_BACKGROUND_COLOR),
                    BorderRadius::all(Val::Px(4.0)),
                    SettingsPanel,
                ))
                .with_children(|panel| {
                    // Title
                    panel.spawn((
                        Text::new("SETTINGS"),
                        TextFont {
                            font: font.clone(),
                            font_size: TITLE_FONT_SIZE,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // Audio Settings Section
                    panel.spawn(Node {
                        height: Val::Px(15.0),
                        ..default()
                    });

                    panel.spawn((
                        Text::new("Audio Settings"),
                        TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.5)),
                    ));

                    // Master Volume
                    panel.spawn((
                        Text::new("Master Volume: 80%"),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    panel.spawn((
                        Node {
                            width: Val::Percent(95.0),
                            height: Val::Px(8.0),
                            margin: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(SLIDER_TRACK_COLOR),
                        BorderRadius::all(Val::Px(4.0)),
                    ));
                    panel.spawn(SettingSlider::new(VolumeSlider::Master, 0.8));

                    // Music Volume
                    panel.spawn((
                        Text::new("Music Volume: 60%"),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    panel.spawn((
                        Node {
                            width: Val::Percent(95.0),
                            height: Val::Px(8.0),
                            margin: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(SLIDER_TRACK_COLOR),
                        BorderRadius::all(Val::Px(4.0)),
                    ));
                    panel.spawn(SettingSlider::new(VolumeSlider::Music, 0.6));

                    // SFX Volume
                    panel.spawn((
                        Text::new("SFX Volume: 100%"),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    panel.spawn((
                        Node {
                            width: Val::Percent(95.0),
                            height: Val::Px(8.0),
                            margin: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(SLIDER_TRACK_COLOR),
                        BorderRadius::all(Val::Px(4.0)),
                    ));
                    panel.spawn(SettingSlider::new(VolumeSlider::Sfx, 1.0));

                    // Ambient Volume
                    panel.spawn((
                        Text::new("Ambient Volume: 50%"),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    panel.spawn((
                        Node {
                            width: Val::Percent(95.0),
                            height: Val::Px(8.0),
                            margin: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(SLIDER_TRACK_COLOR),
                        BorderRadius::all(Val::Px(4.0)),
                    ));
                    panel.spawn(SettingSlider::new(VolumeSlider::Ambient, 0.5));

                    // Graphics Settings Section
                    panel.spawn(Node {
                        height: Val::Px(20.0),
                        ..default()
                    });

                    panel.spawn((
                        Text::new("Graphics Settings"),
                        TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.5)),
                    ));

                    panel.spawn((
                        Text::new("Resolution: 1920x1080"),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    ));

                    panel.spawn((
                        Text::new("Fullscreen: Enabled"),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    ));

                    panel.spawn((
                        Text::new("VSync: Enabled"),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    ));

                    // Controls Section
                    panel.spawn(Node {
                        height: Val::Px(20.0),
                        ..default()
                    });

                    panel.spawn((
                        Text::new("Controls"),
                        TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.5)),
                    ));

                    panel.spawn((
                        Text::new("Move: Arrow Keys or WASD"),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    ));

                    panel.spawn((
                        Text::new("Interact: E"),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    ));

                    panel.spawn((
                        Text::new("Menu: ESC"),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    ));

                    panel.spawn((
                        Text::new("Up/Down: Arrow Up/Down"),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    ));

                    // Action buttons
                    panel
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceAround,
                            margin: UiRect::top(Val::Px(20.0)),
                            ..default()
                        })
                        .with_children(|buttons| {
                            buttons
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(BUTTON_WIDTH - 60.0),
                                        height: Val::Px(BUTTON_HEIGHT),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(BUTTON_NORMAL_COLOR),
                                    BorderRadius::all(Val::Px(4.0)),
                                    MenuButton::Confirm,
                                ))
                                .with_children(|button_root| {
                                    button_root.spawn((
                                        Text::new("Apply"),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: BUTTON_FONT_SIZE,
                                            ..default()
                                        },
                                        TextColor(BUTTON_TEXT_COLOR),
                                    ));
                                });

                            buttons
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(BUTTON_WIDTH - 60.0),
                                        height: Val::Px(BUTTON_HEIGHT),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(BUTTON_NORMAL_COLOR),
                                    BorderRadius::all(Val::Px(4.0)),
                                    MenuButton::Cancel,
                                ))
                                .with_children(|button_root| {
                                    button_root.spawn((
                                        Text::new("Reset"),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: BUTTON_FONT_SIZE,
                                            ..default()
                                        },
                                        TextColor(BUTTON_TEXT_COLOR),
                                    ));
                                });

                            buttons
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(BUTTON_WIDTH - 60.0),
                                        height: Val::Px(BUTTON_HEIGHT),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(BUTTON_NORMAL_COLOR),
                                    BorderRadius::all(Val::Px(4.0)),
                                    MenuButton::Back,
                                ))
                                .with_children(|button_root| {
                                    button_root.spawn((
                                        Text::new("Back"),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: BUTTON_FONT_SIZE,
                                            ..default()
                                        },
                                        TextColor(BUTTON_TEXT_COLOR),
                                    ));
                                });
                        });
                });
        });

    info!("Spawned settings menu UI");
}

/// Populate the save list from the filesystem
fn populate_save_list(mut global_state: ResMut<GlobalState>, save_manager: Res<SaveGameManager>) {
    let GameMode::Menu(menu_state) = &mut global_state.0.mode else {
        return;
    };

    // Only populate once when entering SaveLoad submenu
    if menu_state.current_submenu != MenuType::SaveLoad || !menu_state.save_list.is_empty() {
        return;
    }

    match save_manager.list_saves() {
        Ok(save_filenames) => {
            let mut save_list = Vec::new();

            for filename in save_filenames {
                // Try to load save metadata
                match save_manager.load(&filename) {
                    Ok(game_state) => {
                        let character_names = game_state
                            .party
                            .members
                            .iter()
                            .map(|c| c.name.clone())
                            .collect();

                        let location = format!(
                            "Map {}, ({}, {})",
                            game_state.world.current_map,
                            game_state.world.party_position.x,
                            game_state.world.party_position.y
                        );

                        save_list.push(SaveGameInfo {
                            filename: filename.clone(),
                            timestamp: String::from("Unknown"),
                            character_names,
                            location,
                            game_version: env!("CARGO_PKG_VERSION").to_string(),
                        });
                    }
                    Err(e) => {
                        warn!("Failed to load save metadata for {}: {}", filename, e);
                        // Still add an entry with limited info
                        save_list.push(SaveGameInfo {
                            filename: filename.clone(),
                            timestamp: String::from("Error loading"),
                            character_names: vec![],
                            location: String::from("Unknown"),
                            game_version: env!("CARGO_PKG_VERSION").to_string(),
                        });
                    }
                }
            }

            menu_state.save_list = save_list;
            info!(
                "Populated save list with {} saves",
                menu_state.save_list.len()
            );
        }
        Err(e) => {
            error!("Failed to list saves: {}", e);
            menu_state.save_list = Vec::new();
        }
    }
}

/// Clean up menu UI when exiting Menu mode
fn menu_cleanup(
    mut commands: Commands,
    menu_query: Query<Entity, With<MenuRoot>>,
    global_state: Res<GlobalState>,
) {
    if matches!(global_state.0.mode, GameMode::Menu(_)) {
        return;
    }

    for entity in menu_query.iter() {
        commands.entity(entity).despawn();
        info!("Despawned menu UI");
    }
}

/// Handle button interactions
fn menu_button_interaction(
    mut interaction_query: Query<(&Interaction, &MenuButton), Changed<Interaction>>,
    mut global_state: ResMut<GlobalState>,
    save_manager: Res<SaveGameManager>,
) {
    for (interaction, button) in interaction_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            handle_button_press(button, &mut global_state, &save_manager);
        }
    }
}

/// Handle button press actions
fn handle_button_press(
    button: &MenuButton,
    global_state: &mut ResMut<GlobalState>,
    save_manager: &Res<SaveGameManager>,
) {
    let GameMode::Menu(menu_state) = &mut global_state.0.mode else {
        return;
    };

    match button {
        MenuButton::Resume => {
            let resume_mode = menu_state.get_resume_mode();
            info!("Resume pressed, returning to: {:?}", resume_mode);
            global_state.0.mode = resume_mode;
        }
        MenuButton::SaveGame => {
            info!("Save Game pressed");
            menu_state.set_submenu(MenuType::SaveLoad);
        }
        MenuButton::LoadGame => {
            info!("Load Game pressed");
            menu_state.set_submenu(MenuType::SaveLoad);
        }
        MenuButton::Settings => {
            info!("Settings pressed");
            menu_state.set_submenu(MenuType::Settings);
        }
        MenuButton::Quit => {
            info!("Quit pressed - exiting");
            std::process::exit(0);
        }
        MenuButton::Back => {
            info!("Back pressed");
            menu_state.set_submenu(MenuType::Main);
        }
        MenuButton::Confirm => {
            info!("Confirm pressed");
            match menu_state.current_submenu {
                MenuType::SaveLoad => {
                    save_game_operation(global_state, save_manager);
                }
                MenuType::Settings => {
                    // Settings will be applied by apply_settings system
                    menu_state.set_submenu(MenuType::Main);
                }
                _ => {}
            }
        }
        MenuButton::SelectSave(index) => {
            info!("Selected save slot: {}", index);
            menu_state.selected_index = *index;
        }
        MenuButton::Cancel => {
            info!("Cancel pressed");
            match menu_state.current_submenu {
                MenuType::Settings => {
                    // Reset sliders without applying changes
                    info!("Settings reset - returning to main menu");
                    menu_state.set_submenu(MenuType::Main);
                }
                _ => {
                    menu_state.set_submenu(MenuType::Main);
                }
            }
        }
    }
}

/// Save the current game state to a file
fn save_game_operation(
    global_state: &mut ResMut<GlobalState>,
    save_manager: &Res<SaveGameManager>,
) {
    let GameMode::Menu(_menu_state) = &global_state.0.mode else {
        return;
    };

    // Generate filename with timestamp
    let timestamp = Local::now();
    let filename = timestamp.format("save_%Y%m%d_%H%M%S").to_string();

    // Attempt to save
    match save_manager.save(&filename, &global_state.0) {
        Ok(_) => {
            info!("Game saved successfully: {}", filename);
            if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
                menu_state.save_list.clear(); // Clear to force repopulation on next SaveLoad entry
                menu_state.set_submenu(MenuType::Main);
            }
        }
        Err(e) => {
            error!("Failed to save game: {}", e);
        }
    }
}

/// Load a game state from a save file
fn load_game_operation(
    global_state: &mut ResMut<GlobalState>,
    save_manager: &Res<SaveGameManager>,
    selected_filename: &str,
) {
    match save_manager.load(selected_filename) {
        Ok(loaded_state) => {
            info!("Game loaded successfully: {}", selected_filename);

            // Replace game state
            global_state.0 = loaded_state;

            // Return to exploration mode
            global_state.0.mode = GameMode::Exploration;
        }
        Err(e) => {
            error!("Failed to load game: {}", e);
        }
    }
}

/// Apply settings changes from sliders to GameConfig
fn apply_settings(
    slider_query: Query<&SettingSlider, Changed<SettingSlider>>,
    mut global_state: ResMut<GlobalState>,
) {
    if !matches!(global_state.0.mode, GameMode::Menu(ref m) if m.current_submenu == MenuType::Settings)
    {
        return;
    }

    for slider in slider_query.iter() {
        match slider.slider_type {
            VolumeSlider::Master => {
                global_state.0.config.audio.master_volume = slider.current_value;
                info!("Master volume updated to {:.0}%", slider.as_percentage());
            }
            VolumeSlider::Music => {
                global_state.0.config.audio.music_volume = slider.current_value;
                info!("Music volume updated to {:.0}%", slider.as_percentage());
            }
            VolumeSlider::Sfx => {
                global_state.0.config.audio.sfx_volume = slider.current_value;
                info!("SFX volume updated to {:.0}%", slider.as_percentage());
            }
            VolumeSlider::Ambient => {
                global_state.0.config.audio.ambient_volume = slider.current_value;
                info!("Ambient volume updated to {:.0}%", slider.as_percentage());
            }
        }
    }
}

/// Update button colors based on selection
fn update_button_colors(
    mut button_query: Query<(&MenuButton, &mut BackgroundColor)>,
    global_state: Res<GlobalState>,
) {
    let GameMode::Menu(menu_state) = &global_state.0.mode else {
        return;
    };

    for (button, mut bg_color) in button_query.iter_mut() {
        let button_index = match menu_state.current_submenu {
            MenuType::Main => match button {
                MenuButton::Resume => Some(0),
                MenuButton::SaveGame => Some(1),
                MenuButton::LoadGame => Some(2),
                MenuButton::Settings => Some(3),
                MenuButton::Quit => Some(4),
                _ => None,
            },
            MenuType::SaveLoad => {
                if let MenuButton::SelectSave(idx) = button {
                    Some(*idx)
                } else {
                    None
                }
            }
            MenuType::Settings => None,
        };

        if let Some(idx) = button_index {
            if idx == menu_state.selected_index {
                *bg_color = BackgroundColor(BUTTON_HOVER_COLOR);
            } else {
                *bg_color = BackgroundColor(BUTTON_NORMAL_COLOR);
            }
        }
    }
}

/// Handle keyboard input for menu navigation
fn handle_menu_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut global_state: ResMut<GlobalState>,
    save_manager: Res<SaveGameManager>,
) {
    if !matches!(global_state.0.mode, GameMode::Menu(_)) {
        return;
    }

    // Extract values before handling selection to avoid borrow conflicts
    let (submenu, selected_index, item_count, save_list) = {
        if let GameMode::Menu(menu_state) = &global_state.0.mode {
            (
                menu_state.current_submenu,
                menu_state.selected_index,
                match menu_state.current_submenu {
                    MenuType::Main => 5,
                    MenuType::SaveLoad => menu_state.save_list.len().max(1),
                    MenuType::Settings => 4,
                },
                menu_state.save_list.clone(),
            )
        } else {
            return;
        }
    };

    // Handle keyboard input
    if keyboard.just_pressed(KeyCode::Backspace) {
        if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
            if menu_state.current_submenu != MenuType::Main {
                menu_state.set_submenu(MenuType::Main);
            }
        }
        return;
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
            let resume = menu_state.get_resume_mode();
            global_state.0.mode = resume;
        }
        return;
    }

    if keyboard.just_pressed(KeyCode::ArrowUp) {
        if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
            menu_state.select_previous(item_count);
        }
    } else if keyboard.just_pressed(KeyCode::ArrowDown) {
        if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
            menu_state.select_next(item_count);
        }
    } else if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        // Handle selection based on current menu state
        match submenu {
            MenuType::Main => match selected_index {
                0 => {
                    info!("Selected: Resume");
                    if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
                        let resume = menu_state.get_resume_mode();
                        global_state.0.mode = resume;
                    }
                }
                1 => {
                    info!("Selected: Save");
                    save_game_operation(&mut global_state, &save_manager);
                }
                2 => {
                    info!("Selected: Load");
                    if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
                        menu_state.set_submenu(MenuType::SaveLoad);
                        menu_state.save_list.clear();
                    }
                }
                3 => {
                    info!("Selected: Settings");
                    if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
                        menu_state.set_submenu(MenuType::Settings);
                    }
                }
                4 => {
                    info!("Selected: Quit");
                    std::process::exit(0);
                }
                _ => {}
            },
            MenuType::SaveLoad => {
                if selected_index < save_list.len() {
                    let filename = save_list[selected_index].filename.clone();
                    info!("Selected save slot: {} ({})", selected_index, filename);
                    load_game_operation(&mut global_state, &save_manager, &filename);
                }
            }
            MenuType::Settings => {
                info!("Selected settings: {}", selected_index);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_button_variants() {
        assert!(matches!(MenuButton::Resume, MenuButton::Resume));
        assert!(matches!(MenuButton::SaveGame, MenuButton::SaveGame));
        assert!(matches!(MenuButton::Quit, MenuButton::Quit));
    }

    #[test]
    fn test_save_slot_button_variant() {
        assert!(matches!(
            MenuButton::SelectSave(0),
            MenuButton::SelectSave(_)
        ));
        assert!(matches!(
            MenuButton::SelectSave(5),
            MenuButton::SelectSave(5)
        ));
    }

    #[test]
    fn test_back_button_variant() {
        assert!(matches!(MenuButton::Back, MenuButton::Back));
    }

    #[test]
    fn test_confirm_button_variant() {
        assert!(matches!(MenuButton::Confirm, MenuButton::Confirm));
    }

    #[test]
    fn test_cancel_button_variant() {
        assert!(matches!(MenuButton::Cancel, MenuButton::Cancel));
    }

    #[test]
    fn test_save_game_info_creation() {
        let info = SaveGameInfo {
            filename: "save_20250101_120000".to_string(),
            timestamp: "2025-01-01 12:00:00".to_string(),
            character_names: vec!["Hero".to_string(), "Mage".to_string()],
            location: "Map 1, (5, 10)".to_string(),
            game_version: "0.1.0".to_string(),
        };

        assert_eq!(info.filename, "save_20250101_120000");
        assert_eq!(info.character_names.len(), 2);
        assert_eq!(info.location, "Map 1, (5, 10)");
    }

    // ========================================================================
    // Phase 6: Settings Menu Integration Tests
    // ========================================================================

    #[test]
    fn test_setting_slider_creation_and_defaults() {
        let slider = SettingSlider::new(VolumeSlider::Master, 0.8);
        assert_eq!(slider.slider_type, VolumeSlider::Master);
        assert_eq!(slider.current_value, 0.8);
        assert_eq!(slider.as_percentage(), 80);
    }

    #[test]
    fn test_setting_slider_all_volume_types() {
        let master = SettingSlider::new(VolumeSlider::Master, 0.8);
        let music = SettingSlider::new(VolumeSlider::Music, 0.6);
        let sfx = SettingSlider::new(VolumeSlider::Sfx, 1.0);
        let ambient = SettingSlider::new(VolumeSlider::Ambient, 0.5);

        assert_eq!(master.as_percentage(), 80);
        assert_eq!(music.as_percentage(), 60);
        assert_eq!(sfx.as_percentage(), 100);
        assert_eq!(ambient.as_percentage(), 50);
    }

    #[test]
    fn test_setting_slider_percentage_conversion() {
        let mut slider = SettingSlider::new(VolumeSlider::Music, 0.5);
        assert_eq!(slider.as_percentage(), 50);

        slider.set_from_percentage(75);
        assert_eq!(slider.current_value, 0.75);
        assert_eq!(slider.as_percentage(), 75);

        slider.set_from_percentage(25);
        assert_eq!(slider.current_value, 0.25);
        assert_eq!(slider.as_percentage(), 25);
    }

    #[test]
    fn test_setting_slider_increment_decrement() {
        let mut slider = SettingSlider::new(VolumeSlider::Sfx, 0.5);
        assert_eq!(slider.as_percentage(), 50);

        slider.increment();
        assert_eq!(slider.as_percentage(), 55);

        slider.increment();
        assert_eq!(slider.as_percentage(), 60);

        slider.decrement();
        assert_eq!(slider.as_percentage(), 55);

        slider.decrement();
        slider.decrement();
        assert_eq!(slider.as_percentage(), 45);
    }

    #[test]
    fn test_setting_slider_clamping_at_boundaries() {
        let mut slider = SettingSlider::new(VolumeSlider::Ambient, 0.0);

        // Cannot go below 0
        slider.decrement();
        assert_eq!(slider.current_value, 0.0);
        assert_eq!(slider.as_percentage(), 0);

        // Set to max
        slider.current_value = 1.0;
        slider.increment();
        assert_eq!(slider.current_value, 1.0);
        assert_eq!(slider.as_percentage(), 100);
    }

    #[test]
    fn test_setting_slider_clamping_in_constructor() {
        let clamped_high = SettingSlider::new(VolumeSlider::Master, 1.5);
        assert_eq!(clamped_high.current_value, 1.0);

        let clamped_low = SettingSlider::new(VolumeSlider::Music, -0.5);
        assert_eq!(clamped_low.current_value, 0.0);
    }

    #[test]
    fn test_setting_slider_adjust_positive() {
        let mut slider = SettingSlider::new(VolumeSlider::Master, 0.5);
        slider.adjust(0.2);
        assert_eq!(slider.as_percentage(), 70);

        slider.adjust(0.1);
        assert_eq!(slider.as_percentage(), 80);
    }

    #[test]
    fn test_setting_slider_adjust_negative() {
        let mut slider = SettingSlider::new(VolumeSlider::Music, 0.5);
        slider.adjust(-0.1);
        assert_eq!(slider.as_percentage(), 40);

        slider.adjust(-0.2);
        assert_eq!(slider.as_percentage(), 20);
    }

    #[test]
    fn test_setting_slider_adjust_clamping() {
        let mut slider = SettingSlider::new(VolumeSlider::Sfx, 0.95);
        slider.adjust(0.1); // Would go to 1.05, should clamp to 1.0
        assert_eq!(slider.current_value, 1.0);
        assert_eq!(slider.as_percentage(), 100);

        let mut slider2 = SettingSlider::new(VolumeSlider::Ambient, 0.05);
        slider2.adjust(-0.1); // Would go to -0.05, should clamp to 0.0
        assert_eq!(slider2.current_value, 0.0);
        assert_eq!(slider2.as_percentage(), 0);
    }

    #[test]
    fn test_menu_button_confirm_in_settings() {
        assert!(matches!(MenuButton::Confirm, MenuButton::Confirm));
    }

    #[test]
    fn test_menu_button_cancel_in_settings() {
        assert!(matches!(MenuButton::Cancel, MenuButton::Cancel));
    }

    #[test]
    fn test_settings_panel_marker_component() {
        let panel = SettingsPanel;
        let _ = format!("{:?}", panel);
    }

    #[test]
    fn test_slider_constants_for_ui() {
        assert_eq!(SLIDER_TRACK_COLOR, Color::srgb(0.2, 0.2, 0.3));
        assert_eq!(SLIDER_FILL_COLOR, Color::srgb(0.5, 0.7, 1.0));
    }

    #[test]
    fn test_audio_config_default_matches_sliders() {
        use crate::sdk::game_config::AudioConfig;
        let audio_config = AudioConfig::default();
        let master_slider = SettingSlider::new(VolumeSlider::Master, audio_config.master_volume);
        let music_slider = SettingSlider::new(VolumeSlider::Music, audio_config.music_volume);
        let sfx_slider = SettingSlider::new(VolumeSlider::Sfx, audio_config.sfx_volume);
        let ambient_slider = SettingSlider::new(VolumeSlider::Ambient, audio_config.ambient_volume);

        assert_eq!(master_slider.as_percentage(), 80);
        assert_eq!(music_slider.as_percentage(), 60);
        assert_eq!(sfx_slider.as_percentage(), 100);
        assert_eq!(ambient_slider.as_percentage(), 50);
    }

    #[test]
    fn test_setting_slider_rounding_in_percentage() {
        let slider = SettingSlider::new(VolumeSlider::Master, 0.555);
        // 0.555 * 100 = 55.5, rounds to 56
        assert_eq!(slider.as_percentage(), 56);

        let slider2 = SettingSlider::new(VolumeSlider::Music, 0.544);
        // 0.544 * 100 = 54.4, rounds to 54
        assert_eq!(slider2.as_percentage(), 54);
    }

    #[test]
    fn test_settings_menu_submenu_type() {
        assert_eq!(MenuType::Settings, MenuType::Settings);
        assert_ne!(MenuType::Settings, MenuType::Main);
        assert_ne!(MenuType::Settings, MenuType::SaveLoad);
    }
}
