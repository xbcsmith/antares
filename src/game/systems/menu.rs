// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Menu plugin and system implementations: Menu UI Rendering, Save/Load, and Settings Integration
//!
//! This module implements the complete menu UI system with:
//! - UI spawning based on current submenu (Main/SaveLoad/Settings)
//! - Button interaction handling (hover/click)
//! - Dynamic button color updates based on selection
//! - Save/Load menu with scrollable save list
//! - Save and load game operations
//! - Settings menu with volume sliders
//! - Proper cleanup when exiting menu mode

use bevy::ecs::world::World;
use bevy::prelude::*;
use chrono::Local;

use crate::application::menu::{MenuState, MenuType, SaveGameInfo};
use crate::application::save_game::SaveGameManager;
use crate::application::GameMode;
use crate::game::components::menu::*;
use crate::game::resources::GlobalState;
use crate::game::systems::mouse_input;
use crate::game::systems::ui::GameLog;
use crate::game::systems::ui_helpers::{text_style, BODY_FONT_SIZE};

/// Path to the Antares icon, relative to the Bevy asset root (campaign directory).
/// Bevy resolves paths relative to `BEVY_ASSET_ROOT` (the campaign dir), so the
/// path must include the `assets/` subdirectory prefix.
const ANTARES_ICON_PATH: &str = "assets/icons/antares_icon.png";

/// Plugin for the in-game menu system
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(
            SaveGameManager::new("saves")
                .expect("SaveGameManager state directory 'saves' must be writable"),
        );

        app.add_systems(
            Update,
            (
                submenu_transition_cleanup,
                menu_setup,
                handle_menu_keyboard,
                menu_button_interaction,
                handle_slider_mouse,
                update_button_colors,
                populate_save_list,
                apply_settings,
                menu_cleanup,
            ),
        );
    }
}

/// Recursively despawn an entity and all its children
fn despawn_with_children(
    commands: &mut Commands,
    entity: Entity,
    children_query: &Query<&Children>,
) {
    // First despawn all children recursively with safety checks
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            despawn_with_children(commands, child, children_query);
        }
    }
    // Then despawn the entity itself with safe error handling
    commands.queue(move |world: &mut World| {
        if world.get_entity(entity).is_ok() {
            world.despawn(entity);
        }
    });
}

/// Detect submenu transitions and despawn old menu UI
fn submenu_transition_cleanup(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    menu_query: Query<Entity, With<MenuRoot>>,
    children_query: Query<&Children>,
    mut previous_submenu: bevy::ecs::system::Local<Option<MenuType>>,
) {
    // Diagnostic logging: show mode, previous submenu, and how many MenuRoot entities exist
    let menu_count = menu_query.iter().count();
    debug!(
        "submenu_transition_cleanup called: mode={:?}, previous_submenu={:?}, menu_count={}",
        global_state.0.mode, *previous_submenu, menu_count
    );

    let GameMode::Menu(menu_state) = &global_state.0.mode else {
        // Not in menu mode - reset tracking
        debug!(
            "submenu_transition_cleanup: not in Menu mode - clearing previous_submenu (was: {:?})",
            *previous_submenu
        );
        *previous_submenu = None;
        return;
    };

    let current_submenu = menu_state.current_submenu;

    debug!(
        "submenu_transition_cleanup: in Menu - current_submenu={:?}, previous_submenu={:?}, menu_count={}",
        current_submenu, *previous_submenu, menu_count
    );

    // Check if this is a submenu transition (not first entry into menu)
    if let Some(prev) = *previous_submenu {
        if prev != current_submenu {
            // Submenu changed - log and despawn old UI
            let entities: Vec<Entity> = menu_query.iter().collect();
            if entities.is_empty() {
                debug!(
                    "submenu_transition_cleanup: submenu changed {:?} -> {:?} but no MenuRoot entities found",
                    prev, current_submenu
                );
            } else {
                debug!(
                    "submenu_transition_cleanup: despawning {} MenuRoot entity(ies) for transition: {:?} -> {:?}",
                    entities.len(),
                    prev,
                    current_submenu
                );
                for entity in entities.iter() {
                    debug!(
                        "submenu_transition_cleanup: despawning entity {:?} for submenu transition {:?} -> {:?}",
                        entity, prev, current_submenu
                    );
                    despawn_with_children(&mut commands, *entity, &children_query);
                }
            }
        }
    }

    // Update tracked submenu
    *previous_submenu = Some(current_submenu);
}

fn menu_setup(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    asset_server: Res<AssetServer>,
    existing_menu: Query<Entity, With<MenuRoot>>,
    children_query: Query<&Children>,
) {
    let GameMode::Menu(menu_state) = &global_state.0.mode else {
        return;
    };

    let existing_count = existing_menu.iter().count();
    debug!(
        "menu_setup called: mode={:?}, current_submenu={:?}, existing_menu_count={}",
        global_state.0.mode, menu_state.current_submenu, existing_count
    );

    if !existing_menu.is_empty() {
        debug!(
            "menu_setup: found {} existing MenuRoot entity(ies) - logging details",
            existing_count
        );

        // Log each MenuRoot entity and how many Children it has so we can diagnose
        // stray or partially-initialized UI entities that prevent spawning.
        for entity in existing_menu.iter() {
            match children_query.get(entity) {
                Ok(children) => {
                    debug!(
                        "menu_setup: existing MenuRoot entity {:?} has {} children",
                        entity,
                        children.len()
                    );
                }
                Err(_) => {
                    debug!(
                        "menu_setup: existing MenuRoot entity {:?} has no Children component",
                        entity
                    );
                }
            }
        }

        debug!(
            "menu_setup: skipping spawn due to {} existing MenuRoot entity(ies)",
            existing_count
        );
        return;
    }

    match menu_state.current_submenu {
        MenuType::Main => {
            debug!(
                "menu_setup: spawning Main menu (selected_index={})",
                menu_state.selected_index
            );
            spawn_main_menu(&mut commands, menu_state, &asset_server)
        }
        MenuType::SaveLoad => {
            debug!(
                "menu_setup: spawning Save/Load menu (selected_index={})",
                menu_state.selected_index
            );
            spawn_save_load_menu(&mut commands, menu_state)
        }
        MenuType::Settings => {
            debug!("menu_setup: spawning Settings menu");
            spawn_settings_menu(&mut commands, &global_state.0)
        }
    }
}

/// Spawn the main menu UI
fn spawn_main_menu(commands: &mut Commands, menu_state: &MenuState, asset_server: &AssetServer) {
    let icon_handle: Handle<Image> = asset_server.load(ANTARES_ICON_PATH);
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
                    // Title row: icon + "Antares RPG" text side by side
                    panel
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Auto,
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            column_gap: Val::Px(12.0),
                            ..default()
                        })
                        .with_children(|title_row| {
                            // Icon
                            title_row.spawn((
                                Node {
                                    width: Val::Px(TITLE_FONT_SIZE * 1.4),
                                    height: Val::Px(TITLE_FONT_SIZE * 1.4),
                                    ..default()
                                },
                                ImageNode::new(icon_handle),
                            ));
                            // "Antares RPG" text
                            title_row.spawn((
                                Text::new("Antares RPG"),
                                TextFont {
                                    font_size: TITLE_FONT_SIZE,
                                    ..default()
                                },
                                TextColor(TITLE_TEXT_COLOR),
                            ));
                        });

                    // Accent separator line beneath the title
                    panel.spawn((
                        Node {
                            width: Val::Percent(90.0),
                            height: Val::Px(2.0),
                            margin: UiRect::vertical(Val::Px(10.0)),
                            ..default()
                        },
                        BackgroundColor(MENU_ACCENT_COLOR),
                    ));

                    // Spacing
                    panel.spawn(Node {
                        height: Val::Px(10.0),
                        ..default()
                    });

                    // Buttons: Resume, New Game, Save, Load, Settings, Quit
                    let buttons = [
                        (MenuButton::Resume, "Resume Game", 0),
                        (MenuButton::NewGame, "New Game", 1),
                        (MenuButton::SaveGame, "Save Game", 2),
                        (MenuButton::LoadGame, "Load Game", 3),
                        (MenuButton::Settings, "Settings", 4),
                        (MenuButton::Quit, "Quit Game", 5),
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
                                button_root
                                    .spawn(Node {
                                        width: Val::Auto,
                                        height: Val::Auto,
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    })
                                    .with_children(|text_wrapper| {
                                        text_wrapper.spawn((
                                            Text::new(btn_text.to_string()),
                                            TextFont {
                                                font_size: BUTTON_FONT_SIZE,
                                                ..default()
                                            },
                                            TextColor(BUTTON_TEXT_COLOR),
                                        ));
                                    });
                            });
                    }
                });
        });

    debug!(
        "Spawned main menu UI (selected_index: {}, current_submenu: {:?})",
        menu_state.selected_index, menu_state.current_submenu
    );
}

/// Spawn the save/load menu UI with scrollable save list
fn spawn_save_load_menu(commands: &mut Commands, menu_state: &MenuState) {
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
                    panel
                        .spawn(Node {
                            width: Val::Auto,
                            height: Val::Auto,
                            ..default()
                        })
                        .with_children(|title| {
                            title.spawn((
                                Text::new("SAVE / LOAD GAME"),
                                TextFont {
                                    font_size: TITLE_FONT_SIZE,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });

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
                                list.spawn(Node {
                                    width: Val::Auto,
                                    height: Val::Auto,
                                    ..default()
                                })
                                .with_children(|text_wrapper| {
                                    text_wrapper.spawn((
                                        Text::new("No save files found"),
                                        TextFont {
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                                    ));
                                });
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
                                        slot.spawn(Node {
                                            width: Val::Auto,
                                            height: Val::Auto,
                                            ..default()
                                        })
                                        .with_children(
                                            |text_wrapper| {
                                                text_wrapper.spawn((
                                                    Text::new(format!(
                                                        "Save: {}",
                                                        save_info.filename
                                                    )),
                                                    text_style(BODY_FONT_SIZE, Color::WHITE),
                                                ));
                                            },
                                        );

                                        // Timestamp
                                        slot.spawn(Node {
                                            width: Val::Auto,
                                            height: Val::Auto,
                                            ..default()
                                        })
                                        .with_children(
                                            |text_wrapper| {
                                                text_wrapper.spawn((
                                                    Text::new(format!(
                                                        "Date: {}",
                                                        save_info.timestamp
                                                    )),
                                                    TextFont {
                                                        font_size: 12.0,
                                                        ..default()
                                                    },
                                                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                                ));
                                            },
                                        );

                                        // Party members
                                        if !save_info.character_names.is_empty() {
                                            slot.spawn(Node {
                                                width: Val::Auto,
                                                height: Val::Auto,
                                                ..default()
                                            })
                                            .with_children(|text_wrapper| {
                                                text_wrapper.spawn((
                                                    Text::new(format!(
                                                        "Party: {}",
                                                        save_info.character_names.join(", ")
                                                    )),
                                                    TextFont {
                                                        font_size: 12.0,
                                                        ..default()
                                                    },
                                                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                                ));
                                            });
                                        }

                                        // Location
                                        slot.spawn(Node {
                                            width: Val::Auto,
                                            height: Val::Auto,
                                            ..default()
                                        })
                                        .with_children(
                                            |text_wrapper| {
                                                text_wrapper.spawn((
                                                    Text::new(format!(
                                                        "Location: {}",
                                                        save_info.location
                                                    )),
                                                    TextFont {
                                                        font_size: 12.0,
                                                        ..default()
                                                    },
                                                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                                ));
                                            },
                                        );
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
                                        width: Val::Px(BUTTON_WIDTH / 4.0 - 10.0),
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
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(BUTTON_TEXT_COLOR),
                                    ));
                                });

                            buttons
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(BUTTON_WIDTH / 4.0 - 10.0),
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
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(BUTTON_TEXT_COLOR),
                                    ));
                                });

                            buttons
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(BUTTON_WIDTH / 4.0 - 10.0),
                                        height: Val::Px(BUTTON_HEIGHT),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(BUTTON_NORMAL_COLOR),
                                    BorderRadius::all(Val::Px(4.0)),
                                    MenuButton::DeleteGame,
                                ))
                                .with_children(|button_root| {
                                    button_root.spawn((
                                        Text::new("Delete"),
                                        TextFont {
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(BUTTON_TEXT_COLOR),
                                    ));
                                });

                            buttons
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(BUTTON_WIDTH / 4.0 - 10.0),
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
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(BUTTON_TEXT_COLOR),
                                    ));
                                });
                        });
                });
        });

    debug!(
        "Spawned save/load menu UI (selected_index: {}, saves: {})",
        menu_state.selected_index,
        menu_state.save_list.len()
    );
}

/// Spawn the settings menu UI with audio sliders and graphics settings
fn spawn_settings_menu(commands: &mut Commands, game_state: &crate::application::GameState) {
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
                    panel
                        .spawn(Node {
                            width: Val::Auto,
                            height: Val::Auto,
                            ..default()
                        })
                        .with_children(|title| {
                            title.spawn((
                                Text::new("SETTINGS"),
                                TextFont {
                                    font_size: TITLE_FONT_SIZE,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Audio Settings Section
                    panel.spawn(Node {
                        height: Val::Px(15.0),
                        ..default()
                    });

                    panel
                        .spawn(Node {
                            width: Val::Auto,
                            height: Val::Auto,
                            ..default()
                        })
                        .with_children(|text_wrapper| {
                            text_wrapper.spawn((
                                Text::new("Audio Settings"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.5)),
                            ));
                        });

                    // Master Volume
                    let master_vol = game_state.config.audio.master_volume;
                    panel
                        .spawn(Node {
                            width: Val::Auto,
                            height: Val::Auto,
                            ..default()
                        })
                        .with_children(|text_wrapper| {
                            text_wrapper.spawn((
                                Text::new(format!("Master Volume: {:.0}%", master_vol * 100.0)),
                                text_style(BODY_FONT_SIZE, Color::WHITE),
                            ));
                        });
                    panel
                        .spawn((
                            Button,
                            Node {
                                width: Val::Percent(95.0),
                                height: Val::Px(24.0),
                                margin: UiRect::vertical(Val::Px(4.0)),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(SLIDER_TRACK_COLOR),
                            BorderRadius::all(Val::Px(4.0)),
                            SliderTrack {
                                slider_type: VolumeSlider::Master,
                            },
                        ))
                        .with_children(|track| {
                            track.spawn((
                                Node {
                                    width: Val::Percent(master_vol * 100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(SLIDER_FILL_COLOR),
                                BorderRadius::all(Val::Px(4.0)),
                            ));
                        });
                    panel.spawn(SettingSlider::new(VolumeSlider::Master, master_vol));

                    // Music Volume
                    let music_vol = game_state.config.audio.music_volume;
                    panel
                        .spawn(Node {
                            width: Val::Auto,
                            height: Val::Auto,
                            ..default()
                        })
                        .with_children(|text_wrapper| {
                            text_wrapper.spawn((
                                Text::new(format!("Music Volume: {:.0}%", music_vol * 100.0)),
                                text_style(BODY_FONT_SIZE, Color::WHITE),
                            ));
                        });
                    panel
                        .spawn((
                            Button,
                            Node {
                                width: Val::Percent(95.0),
                                height: Val::Px(24.0),
                                margin: UiRect::vertical(Val::Px(4.0)),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(SLIDER_TRACK_COLOR),
                            BorderRadius::all(Val::Px(4.0)),
                            SliderTrack {
                                slider_type: VolumeSlider::Music,
                            },
                        ))
                        .with_children(|track| {
                            track.spawn((
                                Node {
                                    width: Val::Percent(music_vol * 100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(SLIDER_FILL_COLOR),
                                BorderRadius::all(Val::Px(4.0)),
                            ));
                        });
                    panel.spawn(SettingSlider::new(VolumeSlider::Music, music_vol));

                    // SFX Volume
                    let sfx_vol = game_state.config.audio.sfx_volume;
                    panel
                        .spawn(Node {
                            width: Val::Auto,
                            height: Val::Auto,
                            ..default()
                        })
                        .with_children(|text_wrapper| {
                            text_wrapper.spawn((
                                Text::new(format!("SFX Volume: {:.0}%", sfx_vol * 100.0)),
                                text_style(BODY_FONT_SIZE, Color::WHITE),
                            ));
                        });
                    panel
                        .spawn((
                            Button,
                            Node {
                                width: Val::Percent(95.0),
                                height: Val::Px(24.0),
                                margin: UiRect::vertical(Val::Px(4.0)),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(SLIDER_TRACK_COLOR),
                            BorderRadius::all(Val::Px(4.0)),
                            SliderTrack {
                                slider_type: VolumeSlider::Sfx,
                            },
                        ))
                        .with_children(|track| {
                            track.spawn((
                                Node {
                                    width: Val::Percent(sfx_vol * 100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(SLIDER_FILL_COLOR),
                                BorderRadius::all(Val::Px(4.0)),
                            ));
                        });
                    panel.spawn(SettingSlider::new(VolumeSlider::Sfx, sfx_vol));

                    // Ambient Volume
                    let ambient_vol = game_state.config.audio.ambient_volume;
                    panel
                        .spawn(Node {
                            width: Val::Auto,
                            height: Val::Auto,
                            ..default()
                        })
                        .with_children(|text_wrapper| {
                            text_wrapper.spawn((
                                Text::new(format!("Ambient Volume: {:.0}%", ambient_vol * 100.0)),
                                text_style(BODY_FONT_SIZE, Color::WHITE),
                            ));
                        });
                    panel
                        .spawn((
                            Button,
                            Node {
                                width: Val::Percent(95.0),
                                height: Val::Px(24.0),
                                margin: UiRect::vertical(Val::Px(4.0)),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(SLIDER_TRACK_COLOR),
                            BorderRadius::all(Val::Px(4.0)),
                            SliderTrack {
                                slider_type: VolumeSlider::Ambient,
                            },
                        ))
                        .with_children(|track| {
                            track.spawn((
                                Node {
                                    width: Val::Percent(ambient_vol * 100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(SLIDER_FILL_COLOR),
                                BorderRadius::all(Val::Px(4.0)),
                            ));
                        });
                    panel.spawn(SettingSlider::new(VolumeSlider::Ambient, ambient_vol));

                    // Graphics Settings Section
                    panel.spawn(Node {
                        height: Val::Px(20.0),
                        ..default()
                    });

                    panel
                        .spawn(Node {
                            width: Val::Auto,
                            height: Val::Auto,
                            ..default()
                        })
                        .with_children(|text_wrapper| {
                            text_wrapper.spawn((
                                Text::new("Graphics Settings"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.5)),
                            ));
                        });

                    let graphics = &game_state.config.graphics;

                    panel
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(BUTTON_WIDTH - 60.0),
                                height: Val::Px(30.0),
                                margin: UiRect::vertical(Val::Px(5.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(BUTTON_NORMAL_COLOR),
                            BorderRadius::all(Val::Px(4.0)),
                            MenuButton::ToggleFullscreen,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new(format!(
                                    "Fullscreen: {}",
                                    if graphics.fullscreen { "ON" } else { "OFF" }
                                )),
                                text_style(BODY_FONT_SIZE, Color::WHITE),
                            ));
                        });

                    panel
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(BUTTON_WIDTH - 60.0),
                                height: Val::Px(30.0),
                                margin: UiRect::vertical(Val::Px(5.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(BUTTON_NORMAL_COLOR),
                            BorderRadius::all(Val::Px(4.0)),
                            MenuButton::ToggleCombatMonsterHpBars,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new(format!(
                                    "Combat Monster HP Bars: {}",
                                    if graphics.show_combat_monster_hp_bars {
                                        "ON"
                                    } else {
                                        "OFF"
                                    }
                                )),
                                text_style(BODY_FONT_SIZE, Color::WHITE),
                            ));
                        });

                    panel
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(BUTTON_WIDTH - 60.0),
                                height: Val::Px(30.0),
                                margin: UiRect::vertical(Val::Px(5.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(BUTTON_NORMAL_COLOR),
                            BorderRadius::all(Val::Px(4.0)),
                            MenuButton::ToggleVSync,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new(format!(
                                    "VSync: {}",
                                    if graphics.vsync { "ON" } else { "OFF" }
                                )),
                                text_style(BODY_FONT_SIZE, Color::WHITE),
                            ));
                        });

                    panel
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(BUTTON_WIDTH - 60.0),
                                height: Val::Px(30.0),
                                margin: UiRect::vertical(Val::Px(5.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(BUTTON_NORMAL_COLOR),
                            BorderRadius::all(Val::Px(4.0)),
                            MenuButton::CycleShadowQuality,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new(format!("Shadow Quality: {:?}", graphics.shadow_quality)),
                                text_style(BODY_FONT_SIZE, Color::WHITE),
                            ));
                        });

                    // Controls Section
                    panel.spawn(Node {
                        height: Val::Px(20.0),
                        ..default()
                    });

                    panel
                        .spawn(Node {
                            width: Val::Auto,
                            height: Val::Auto,
                            ..default()
                        })
                        .with_children(|text_wrapper| {
                            text_wrapper.spawn((
                                Text::new("Controls"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.5)),
                            ));
                        });

                    let controls = &game_state.config.controls;

                    panel
                        .spawn(Node {
                            width: Val::Auto,
                            height: Val::Auto,
                            ..default()
                        })
                        .with_children(|text_wrapper| {
                            text_wrapper.spawn((
                                Text::new(format!("Move Forward: {:?}", controls.move_forward)),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                            ));
                        });

                    panel
                        .spawn(Node {
                            width: Val::Auto,
                            height: Val::Auto,
                            ..default()
                        })
                        .with_children(|text_wrapper| {
                            text_wrapper.spawn((
                                Text::new(format!("Move Back: {:?}", controls.move_back)),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                            ));
                        });

                    panel
                        .spawn(Node {
                            width: Val::Auto,
                            height: Val::Auto,
                            ..default()
                        })
                        .with_children(|text_wrapper| {
                            text_wrapper.spawn((
                                Text::new(format!(
                                    "Turn Left/Right: {:?}/{:?}",
                                    controls.turn_left, controls.turn_right
                                )),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                            ));
                        });

                    panel
                        .spawn(Node {
                            width: Val::Auto,
                            height: Val::Auto,
                            ..default()
                        })
                        .with_children(|text_wrapper| {
                            text_wrapper.spawn((
                                Text::new(format!(
                                    "Interact/Menu: {:?}/{:?}",
                                    controls.interact, controls.menu
                                )),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                            ));
                        });

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
                                            font_size: BUTTON_FONT_SIZE,
                                            ..default()
                                        },
                                        TextColor(BUTTON_TEXT_COLOR),
                                    ));
                                });
                        });
                });
        });

    debug!("Spawned settings menu UI");
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
            debug!(
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
    children_query: Query<&Children>,
    global_state: Res<GlobalState>,
) {
    let menu_count = menu_query.iter().count();
    debug!(
        "menu_cleanup called: mode={:?}, menu_root_count={}",
        global_state.0.mode, menu_count
    );

    if matches!(global_state.0.mode, GameMode::Menu(_)) {
        debug!("menu_cleanup: in Menu mode - skipping cleanup");
        return;
    }

    if menu_count == 0 {
        debug!("menu_cleanup: no menu entities to cleanup");
        return;
    }

    for entity in menu_query.iter() {
        debug!(
            "menu_cleanup: despawning entity {:?} due to mode {:?}",
            entity, global_state.0.mode
        );
        despawn_with_children(&mut commands, entity, &children_query);
        debug!("menu_cleanup: despawned entity {:?}", entity);
    }
}

/// Handle button interactions
fn menu_button_interaction(
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    mut interaction_query: Query<(&Interaction, Ref<Interaction>, &MenuButton), With<Button>>,
    mut global_state: ResMut<GlobalState>,
    save_manager: Res<SaveGameManager>,
    mut game_log: Option<ResMut<GameLog>>,
) {
    let mouse_just_pressed = mouse_input::mouse_just_pressed(mouse_buttons.as_deref());

    for (interaction, interaction_ref, button) in interaction_query.iter_mut() {
        if mouse_input::is_activated(
            interaction,
            interaction_ref.is_changed(),
            mouse_just_pressed,
        ) {
            // Call the unified handler using dereferenced resources so the same logic
            // is used for both mouse and keyboard-driven actions.
            handle_button_press(
                button,
                &mut global_state,
                &save_manager,
                game_log.as_deref_mut(),
            );
        }
    }
}

/// Handle button press actions
///
/// This accepts plain `&mut GlobalState` and `&SaveGameManager` so it can be
/// called from both systems and unit tests without wrapping/unwrapping Res/ResMut.
fn handle_button_press(
    button: &MenuButton,
    global_state: &mut GlobalState,
    save_manager: &SaveGameManager,
    game_log: Option<&mut GameLog>,
) {
    match button {
        MenuButton::Resume => {
            let GameMode::Menu(menu_state) = &global_state.0.mode else {
                return;
            };
            let resume_mode = menu_state.get_resume_mode();
            debug!("Resume pressed, returning to: {:?}", resume_mode);
            global_state.0.mode = resume_mode;
        }
        MenuButton::SaveGame => {
            if let GameMode::Menu(ref mut menu_state) = global_state.0.mode {
                debug!("Save Game pressed");
                menu_state.set_submenu(MenuType::SaveLoad);
            }
        }
        MenuButton::LoadGame => {
            if let GameMode::Menu(ref mut menu_state) = global_state.0.mode {
                // If we're already in SaveLoad, perform the load operation for the selected slot.
                if menu_state.current_submenu == MenuType::SaveLoad {
                    debug!("Load Game pressed (in SaveLoad)");
                    if let Some(save_info) = menu_state.save_list.get(menu_state.selected_index) {
                        let filename = save_info.filename.clone();
                        load_game_operation(global_state, save_manager, &filename, game_log);
                    } else {
                        warn!("Load pressed but no save selected");
                    }
                } else {
                    debug!("Load Game pressed (from Main)");
                    menu_state.set_submenu(MenuType::SaveLoad);
                }
            }
        }
        MenuButton::Settings => {
            if let GameMode::Menu(ref mut menu_state) = global_state.0.mode {
                debug!("Settings pressed");
                menu_state.set_submenu(MenuType::Settings);
            }
        }
        MenuButton::Quit => {
            debug!("Quit pressed - exiting");
            std::process::exit(0);
        }
        MenuButton::Back => {
            if let GameMode::Menu(ref mut menu_state) = global_state.0.mode {
                debug!("Back pressed");
                menu_state.set_submenu(MenuType::Main);
            }
        }
        MenuButton::Confirm => {
            debug!("Confirm pressed");
            let submenu = if let GameMode::Menu(menu_state) = &global_state.0.mode {
                Some(menu_state.current_submenu)
            } else {
                None
            };

            match submenu {
                Some(MenuType::SaveLoad) => {
                    // In the Save/Load dialog the Confirm button is labeled "Save".
                    save_game_operation(global_state, save_manager, game_log);
                }
                Some(MenuType::Settings) => {
                    if let GameMode::Menu(ref mut menu_state) = global_state.0.mode {
                        menu_state.set_submenu(MenuType::Main);
                    }
                }
                _ => {}
            }
        }
        MenuButton::SelectSave(index) => {
            if let GameMode::Menu(ref mut menu_state) = global_state.0.mode {
                debug!("Selected save slot: {}", index);
                menu_state.selected_index = *index;
            }
        }
        MenuButton::NewGame => {
            debug!("New Game pressed");
            if let Some(campaign) = global_state.0.campaign.clone() {
                match crate::application::GameState::new_game(campaign) {
                    Ok((new_state, _)) => {
                        global_state.0 = new_state;
                        debug!("Started new game from campaign");
                    }
                    Err(e) => {
                        error!("Failed to start new game: {}", e);
                    }
                }
            } else {
                warn!("New Game pressed but no active campaign found");
            }
        }
        MenuButton::DeleteGame => {
            debug!("Delete Game pressed");
            delete_game_operation(global_state, save_manager);
        }
        MenuButton::ToggleFullscreen => {
            global_state.0.config.graphics.fullscreen = !global_state.0.config.graphics.fullscreen;
            debug!(
                "Fullscreen toggled: {}",
                global_state.0.config.graphics.fullscreen
            );
        }
        MenuButton::ToggleVSync => {
            global_state.0.config.graphics.vsync = !global_state.0.config.graphics.vsync;
            debug!("VSync toggled: {}", global_state.0.config.graphics.vsync);
        }
        MenuButton::CycleShadowQuality => {
            use crate::sdk::game_config::ShadowQuality;
            global_state.0.config.graphics.shadow_quality =
                match global_state.0.config.graphics.shadow_quality {
                    ShadowQuality::Low => ShadowQuality::Medium,
                    ShadowQuality::Medium => ShadowQuality::High,
                    ShadowQuality::High => ShadowQuality::Ultra,
                    ShadowQuality::Ultra => ShadowQuality::Low,
                };
            debug!(
                "Shadow quality cycled: {:?}",
                global_state.0.config.graphics.shadow_quality
            );
        }
        MenuButton::ToggleCombatMonsterHpBars => {
            global_state.0.config.graphics.show_combat_monster_hp_bars =
                !global_state.0.config.graphics.show_combat_monster_hp_bars;
            debug!(
                "Combat monster HP bars toggled: {}",
                global_state.0.config.graphics.show_combat_monster_hp_bars
            );
        }
        MenuButton::Cancel => {
            debug!("Cancel pressed");
            if let GameMode::Menu(ref mut menu_state) = global_state.0.mode {
                match menu_state.current_submenu {
                    MenuType::Settings => {
                        debug!("Settings reset - returning to main menu");
                        menu_state.set_submenu(MenuType::Main);
                    }
                    _ => {
                        menu_state.set_submenu(MenuType::Main);
                    }
                }
            }
        }
    }
}

/// Save the current game state to a file
fn save_game_operation(
    global_state: &mut GlobalState,
    save_manager: &SaveGameManager,
    game_log: Option<&mut GameLog>,
) {
    let GameMode::Menu(_menu_state) = &global_state.0.mode else {
        return;
    };

    // Snapshot the live game log entries into GameState so they are included
    // in the serialised save file.
    if let Some(log) = game_log {
        global_state.0.game_log_entries = log.to_saved_entries();
    }

    // Generate filename with timestamp
    let timestamp = Local::now();
    let filename = timestamp.format("save_%Y%m%d_%H%M%S").to_string();

    // Attempt to save
    match save_manager.save(&filename, &global_state.0) {
        Ok(_) => {
            debug!("Game saved successfully: {}", filename);
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
    global_state: &mut GlobalState,
    save_manager: &SaveGameManager,
    selected_filename: &str,
    game_log: Option<&mut GameLog>,
) {
    match save_manager.load(selected_filename) {
        Ok(loaded_state) => {
            debug!("Game loaded successfully: {}", selected_filename);

            // Replace game state
            global_state.0 = loaded_state;

            // Return to exploration mode
            global_state.0.mode = GameMode::Exploration;

            // Restore the live game log from the entries embedded in the save.
            if let Some(log) = game_log {
                let entries = std::mem::take(&mut global_state.0.game_log_entries);
                log.restore_from_saved(entries);
            }
        }
        Err(e) => {
            error!("Failed to load game: {}", e);
        }
    }
}

/// Delete a save game file
fn delete_game_operation(global_state: &mut GlobalState, save_manager: &SaveGameManager) {
    let GameMode::Menu(menu_state) = &mut global_state.0.mode else {
        return;
    };

    if let Some(save_info) = menu_state.save_list.get(menu_state.selected_index) {
        let filename = save_info.filename.clone();
        match save_manager.delete(&filename) {
            Ok(_) => {
                debug!("Game deleted successfully: {}", filename);
                menu_state.save_list.clear(); // Force repopulation
            }
            Err(e) => {
                error!("Failed to delete game: {}", e);
            }
        }
    } else {
        warn!("Delete Game pressed but no save selected");
    }
}

fn update_slider_resource_value(
    global_state: &mut GlobalState,
    slider_type: VolumeSlider,
    value: f32,
) {
    let clamped = value.clamp(0.0, 1.0);
    match slider_type {
        VolumeSlider::Master => {
            global_state.0.config.audio.master_volume = clamped;
        }
        VolumeSlider::Music => {
            global_state.0.config.audio.music_volume = clamped;
        }
        VolumeSlider::Sfx => {
            global_state.0.config.audio.sfx_volume = clamped;
        }
        VolumeSlider::Ambient => {
            global_state.0.config.audio.ambient_volume = clamped;
        }
    }
}

fn slider_value_from_cursor(
    cursor_position: Vec2,
    node: &Node,
    transform: &GlobalTransform,
) -> f32 {
    let width = match node.width {
        Val::Px(width) => width,
        Val::Percent(percent) => percent,
        _ => return 0.0,
    };

    if width <= 0.0 {
        return 0.0;
    }

    let center = transform.translation().truncate();
    let left = center.x - (width * 0.5);
    ((cursor_position.x - left) / width).clamp(0.0, 1.0)
}

/// Query for slider track button entities with their interaction state and layout.
type SliderTrackQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        Ref<'static, Interaction>,
        &'static SliderTrack,
        &'static Node,
        &'static GlobalTransform,
    ),
    With<Button>,
>;

/// Handle mouse input for settings sliders.
///
/// This supports both click-to-set and hover+drag while the left mouse button is held.
fn handle_slider_mouse(
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    primary_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut slider_query: Query<&mut SettingSlider>,
    track_query: SliderTrackQuery,
    mut global_state: ResMut<GlobalState>,
) {
    if !matches!(global_state.0.mode, GameMode::Menu(ref m) if m.current_submenu == MenuType::Settings)
    {
        return;
    }

    let mouse_just_pressed = mouse_input::mouse_just_pressed(mouse_buttons.as_deref());
    let mouse_pressed = mouse_buttons
        .as_deref()
        .is_some_and(|buttons| buttons.pressed(MouseButton::Left));

    let Ok(window) = primary_window.single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    for (interaction, interaction_ref, track, node, transform) in track_query.iter() {
        let activated = mouse_input::is_activated(
            interaction,
            interaction_ref.is_changed(),
            mouse_just_pressed,
        );
        let dragged = mouse_pressed && *interaction == Interaction::Hovered;

        if !activated && !dragged {
            continue;
        }

        let value = slider_value_from_cursor(cursor_position, node, transform);
        update_slider_resource_value(&mut global_state, track.slider_type, value);

        for mut slider in slider_query.iter_mut() {
            if slider.slider_type == track.slider_type {
                slider.current_value = value;
            }
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
                debug!("Master volume updated to {:.0}%", slider.as_percentage());
            }
            VolumeSlider::Music => {
                global_state.0.config.audio.music_volume = slider.current_value;
                debug!("Music volume updated to {:.0}%", slider.as_percentage());
            }
            VolumeSlider::Sfx => {
                global_state.0.config.audio.sfx_volume = slider.current_value;
                debug!("SFX volume updated to {:.0}%", slider.as_percentage());
            }
            VolumeSlider::Ambient => {
                global_state.0.config.audio.ambient_volume = slider.current_value;
                debug!("Ambient volume updated to {:.0}%", slider.as_percentage());
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
                MenuButton::NewGame => Some(1),
                MenuButton::SaveGame => Some(2),
                MenuButton::LoadGame => Some(3),
                MenuButton::Settings => Some(4),
                MenuButton::Quit => Some(5),
                _ => None,
            },
            MenuType::SaveLoad => match button {
                MenuButton::SelectSave(idx) => Some(*idx),
                _ => None,
            },
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
    mut game_log: Option<ResMut<GameLog>>,
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
                    // There are six items on the main menu: Resume, New Game, Save, Load, Settings, Quit
                    MenuType::Main => 6,
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

    if keyboard.just_pressed(KeyCode::ArrowUp) {
        if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
            menu_state.select_previous(item_count);
        }
        return; // Consume arrow key input, don't fall through to other handlers
    }

    if keyboard.just_pressed(KeyCode::ArrowDown) {
        if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
            menu_state.select_next(item_count);
        }
        return; // Consume arrow key input, don't fall through to other handlers
    }

    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        // Handle selection based on current menu state
        match submenu {
            MenuType::Main => {
                // Map the selected index to the corresponding MenuButton and reuse the unified handler.
                let sel_button = match selected_index {
                    0 => MenuButton::Resume,
                    1 => MenuButton::NewGame,
                    2 => MenuButton::SaveGame,
                    3 => MenuButton::LoadGame,
                    4 => MenuButton::Settings,
                    5 => MenuButton::Quit,
                    _ => return,
                };
                handle_button_press(
                    &sel_button,
                    &mut global_state,
                    &save_manager,
                    game_log.as_deref_mut(),
                );
            }
            MenuType::SaveLoad => {
                // Pressing Enter on a save slot should load it (if there is a save at that index)
                if selected_index < save_list.len() {
                    handle_button_press(
                        &MenuButton::LoadGame,
                        &mut global_state,
                        &save_manager,
                        game_log.as_deref_mut(),
                    );
                }
            }
            MenuType::Settings => {
                debug!("Selected settings: {}", selected_index);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameState;
    use bevy::window::PrimaryWindow;

    fn setup_menu_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GlobalState(GameState::new()));
        app.insert_resource(
            SaveGameManager::new(tempfile::TempDir::new().unwrap().path()).unwrap(),
        );
        app.add_systems(Update, (menu_button_interaction, handle_slider_mouse));
        app
    }

    fn set_menu_mode(app: &mut App, submenu: MenuType) {
        let mut global_state = app.world_mut().resource_mut::<GlobalState>();
        global_state.0.enter_menu();
        if let GameMode::Menu(menu_state) = &mut global_state.0.mode {
            menu_state.set_submenu(submenu);
        }
    }

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
    // Settings Menu Integration Tests
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
        assert_eq!(SLIDER_TRACK_COLOR, Color::srgb(0.10, 0.13, 0.22));
        assert_eq!(SLIDER_FILL_COLOR, Color::srgb(0.91, 0.60, 0.04));
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

    // Additional tests for menu keyboard / save/load behavior

    #[test]
    fn test_menu_state_wraps_correctly_with_six_items() {
        let mut ms = MenuState::new(GameMode::Exploration);
        ms.selected_index = 5;
        ms.select_next(6); // should wrap to 0
        assert_eq!(ms.selected_index, 0);

        ms.select_previous(6); // wrap back to 5
        assert_eq!(ms.selected_index, 5);
    }

    #[test]
    fn test_save_button_opens_saveload_submenu() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut gs = GlobalState(GameState::new());
        gs.0.enter_menu();

        // Press Save (simulate button press)
        handle_button_press(&MenuButton::SaveGame, &mut gs, &manager, None);

        // Should be in Menu mode and SaveLoad submenu
        if let GameMode::Menu(ms) = &gs.0.mode {
            assert_eq!(ms.current_submenu, MenuType::SaveLoad);
        } else {
            panic!("Expected to be in Menu mode");
        }
    }

    #[test]
    fn test_confirm_in_saveload_creates_save_and_returns_to_main() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut gs = GlobalState(GameState::new());
        gs.0.enter_menu();

        // Enter SaveLoad submenu
        if let GameMode::Menu(ms) = &mut gs.0.mode {
            ms.set_submenu(MenuType::SaveLoad);
        } else {
            panic!("Expected to be in Menu mode");
        }

        // No saves initially
        assert_eq!(manager.list_saves().unwrap().len(), 0);

        // Press Confirm (Save) in SaveLoad
        handle_button_press(&MenuButton::Confirm, &mut gs, &manager, None);

        // After save, we should have at least one save file and menu returned to Main
        let saves = manager.list_saves().unwrap();
        assert!(!saves.is_empty());

        if let GameMode::Menu(ms) = &gs.0.mode {
            assert_eq!(ms.current_submenu, MenuType::Main);
        } else {
            panic!("Expected to be in Menu mode");
        }
    }

    #[test]
    fn test_load_in_saveload_loads_selected_save() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        // Create a save named "test_save"
        let saved_state = GameState::new();
        manager.save("test_save", &saved_state).unwrap();

        // Create a fresh global state and open SaveLoad with an entry for "test_save"
        let mut gs = GlobalState(GameState::new());
        gs.0.enter_menu();
        if let GameMode::Menu(ms) = &mut gs.0.mode {
            ms.set_submenu(MenuType::SaveLoad);
            ms.save_list = vec![SaveGameInfo {
                filename: "test_save".to_string(),
                timestamp: "now".to_string(),
                character_names: vec![],
                location: "unknown".to_string(),
                game_version: env!("CARGO_PKG_VERSION").to_string(),
            }];
            ms.selected_index = 0;
        } else {
            panic!("Expected to be in Menu mode");
        }

        // Press Load
        handle_button_press(&MenuButton::LoadGame, &mut gs, &manager, None);

        // After a successful load the game mode should be Exploration
        assert!(matches!(gs.0.mode, GameMode::Exploration));
    }

    #[test]
    fn test_mouse_click_resume_button() {
        let mut app = setup_menu_test_app();
        set_menu_mode(&mut app, MenuType::Main);

        app.world_mut()
            .spawn((Button, Interaction::Pressed, MenuButton::Resume));

        app.update();

        let global_state = app.world().resource::<GlobalState>();
        assert!(!matches!(global_state.0.mode, GameMode::Menu(_)));
    }

    #[test]
    fn test_mouse_hovered_click_save_button() {
        let mut app = setup_menu_test_app();
        set_menu_mode(&mut app, MenuType::Main);

        let mut mouse_buttons = ButtonInput::<MouseButton>::default();
        mouse_buttons.press(MouseButton::Left);
        app.insert_resource(mouse_buttons);

        app.world_mut()
            .spawn((Button, Interaction::Hovered, MenuButton::SaveGame));

        app.update();

        let global_state = app.world().resource::<GlobalState>();
        if let GameMode::Menu(menu_state) = &global_state.0.mode {
            assert_eq!(menu_state.current_submenu, MenuType::SaveLoad);
        } else {
            panic!("Expected menu mode");
        }
    }

    #[test]
    fn test_mouse_hover_does_not_dispatch_menu() {
        let mut app = setup_menu_test_app();
        set_menu_mode(&mut app, MenuType::Main);

        app.world_mut()
            .spawn((Button, Interaction::Hovered, MenuButton::SaveGame));

        app.update();

        let global_state = app.world().resource::<GlobalState>();
        if let GameMode::Menu(menu_state) = &global_state.0.mode {
            assert_eq!(menu_state.current_submenu, MenuType::Main);
        } else {
            panic!("Expected menu mode");
        }
    }

    #[test]
    fn test_slider_mouse_click_sets_value() {
        let mut app = setup_menu_test_app();
        set_menu_mode(&mut app, MenuType::Settings);

        let mut window = Window::default();
        window.set_cursor_position(Some(Vec2::new(400.0, 300.0)));
        app.world_mut().spawn((window, PrimaryWindow));

        app.world_mut()
            .spawn(SettingSlider::new(VolumeSlider::Master, 0.0));
        app.world_mut().spawn((
            Button,
            Interaction::Pressed,
            SliderTrack {
                slider_type: VolumeSlider::Master,
            },
            Node {
                width: Val::Px(200.0),
                height: Val::Px(24.0),
                ..default()
            },
            GlobalTransform::from_xyz(400.0, 300.0, 0.0),
        ));

        app.update();

        let global_state = app.world().resource::<GlobalState>();
        assert!((global_state.0.config.audio.master_volume - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_slider_drag_updates_value() {
        let mut app = setup_menu_test_app();
        set_menu_mode(&mut app, MenuType::Settings);

        let mut mouse_buttons = ButtonInput::<MouseButton>::default();
        mouse_buttons.press(MouseButton::Left);
        app.insert_resource(mouse_buttons);

        let mut window = Window::default();
        window.set_cursor_position(Some(Vec2::new(350.0, 300.0)));
        let window_entity = app.world_mut().spawn((window, PrimaryWindow)).id();

        app.world_mut()
            .spawn(SettingSlider::new(VolumeSlider::Music, 0.0));
        let track_entity = app
            .world_mut()
            .spawn((
                Button,
                Interaction::Hovered,
                SliderTrack {
                    slider_type: VolumeSlider::Music,
                },
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(24.0),
                    ..default()
                },
                GlobalTransform::from_xyz(400.0, 300.0, 0.0),
            ))
            .id();

        app.update();

        let first_value = app
            .world()
            .resource::<GlobalState>()
            .0
            .config
            .audio
            .music_volume;

        {
            let world = app.world_mut();
            let mut entity = world.entity_mut(window_entity);
            let mut window = entity.get_mut::<Window>().unwrap();
            window.set_cursor_position(Some(Vec2::new(450.0, 300.0)));
        }
        {
            let world = app.world_mut();
            let mut entity = world.entity_mut(track_entity);
            let mut interaction = entity.get_mut::<Interaction>().unwrap();
            *interaction = Interaction::Hovered;
        }

        app.update();

        let second_value = app
            .world()
            .resource::<GlobalState>()
            .0
            .config
            .audio
            .music_volume;
        assert!(second_value > first_value);
    }

    #[test]
    fn test_save_game_operation_snapshots_log_entries() {
        use crate::game::systems::ui::GameLog;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        // Build a game log with a couple of entries.
        let mut log = GameLog::new();
        log.add_combat("Hit goblin for 5 damage.".to_string());
        log.add_item("Found a sword.".to_string());

        let mut gs = GlobalState(GameState::new());
        gs.0.enter_menu();
        if let GameMode::Menu(ms) = &mut gs.0.mode {
            ms.set_submenu(MenuType::SaveLoad);
        }

        // GameState::game_log_entries should be empty before save.
        assert!(gs.0.game_log_entries.is_empty());

        // Perform the save operation with the live log.
        save_game_operation(&mut gs, &manager, Some(&mut log));

        // After save the snapshot should be embedded in the game state.
        assert_eq!(gs.0.game_log_entries.len(), 2);
        assert_eq!(gs.0.game_log_entries[0].category, "Combat");
        assert_eq!(gs.0.game_log_entries[0].text, "Hit goblin for 5 damage.");
        assert_eq!(gs.0.game_log_entries[1].category, "Item");
        assert_eq!(gs.0.game_log_entries[1].text, "Found a sword.");
    }

    #[test]
    fn test_load_game_operation_restores_log_entries() {
        use crate::application::save_game::SavedLogEntry;
        use crate::game::systems::ui::GameLog;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        // Create a game state with pre-populated log entries.
        let mut saved_state = GameState::new();
        saved_state.game_log_entries = vec![
            SavedLogEntry {
                category: "Combat".to_string(),
                text: "Slew the dragon.".to_string(),
                sequence: 0,
            },
            SavedLogEntry {
                category: "Exploration".to_string(),
                text: "Entered the dungeon.".to_string(),
                sequence: 1,
            },
        ];
        manager.save("log_restore_test", &saved_state).unwrap();

        // Load into a fresh global state with an empty game log.
        let mut gs = GlobalState(GameState::new());
        let mut log = GameLog::new();
        assert!(log.entries().is_empty());

        load_game_operation(&mut gs, &manager, "log_restore_test", Some(&mut log));

        // The live game log should now contain the restored entries.
        let entries = log.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "Slew the dragon.");
        assert_eq!(entries[1].text, "Entered the dungeon.");

        // The entries should have been moved out of game_log_entries.
        assert!(gs.0.game_log_entries.is_empty());
    }

    #[test]
    fn test_save_load_cycle_preserves_game_log() {
        use crate::game::systems::ui::{GameLog, LogCategory};
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        // Populate the live game log before saving.
        let mut log = GameLog::new();
        log.add_dialogue("The innkeeper greeted you.".to_string());
        log.add_combat("You defeated a skeleton.".to_string());
        log.add_exploration("You discovered a secret passage.".to_string());

        // Enter the save menu and save.
        let mut gs = GlobalState(GameState::new());
        gs.0.enter_menu();
        if let GameMode::Menu(ms) = &mut gs.0.mode {
            ms.set_submenu(MenuType::SaveLoad);
        }
        save_game_operation(&mut gs, &manager, Some(&mut log));

        // Clear the in-memory log to simulate a fresh session after restart.
        let mut fresh_log = GameLog::new();
        assert!(fresh_log.entries().is_empty());

        // Load the save into the fresh log.
        let saves = manager.list_saves().unwrap();
        assert!(!saves.is_empty());
        let filename = saves[0].clone();

        let mut gs2 = GlobalState(GameState::new());
        load_game_operation(&mut gs2, &manager, &filename, Some(&mut fresh_log));

        // Verify all three entries are restored with correct categories and text.
        let entries = fresh_log.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "The innkeeper greeted you.");
        assert_eq!(entries[1].text, "You defeated a skeleton.");
        assert_eq!(entries[2].text, "You discovered a secret passage.");

        assert_eq!(entries[0].category, LogCategory::Dialogue);
        assert_eq!(entries[1].category, LogCategory::Combat);
        assert_eq!(entries[2].category, LogCategory::Exploration);
    }

    #[test]
    fn test_save_load_without_game_log_resource_does_not_panic() {
        // Passing None for game_log must never panic — the log is optional
        // (the resource may not be registered in all test environments).
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let manager = SaveGameManager::new(temp_dir.path()).unwrap();

        let mut gs = GlobalState(GameState::new());
        gs.0.enter_menu();
        if let GameMode::Menu(ms) = &mut gs.0.mode {
            ms.set_submenu(MenuType::SaveLoad);
        }

        // Save without a log — must succeed and produce an empty entries list.
        save_game_operation(&mut gs, &manager, None);

        let saves = manager.list_saves().unwrap();
        assert!(!saves.is_empty());
        let filename = saves[0].clone();

        let mut gs2 = GlobalState(GameState::new());
        load_game_operation(&mut gs2, &manager, &filename, None);

        assert!(matches!(gs2.0.mode, GameMode::Exploration));
    }
}
