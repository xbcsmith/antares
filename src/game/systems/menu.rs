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

use bevy::ecs::world::World;
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
                submenu_transition_cleanup,
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
            spawn_main_menu(&mut commands, menu_state)
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
fn spawn_main_menu(commands: &mut Commands, menu_state: &MenuState) {
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
                    panel
                        .spawn(Node {
                            width: Val::Auto,
                            height: Val::Auto,
                            ..default()
                        })
                        .with_children(|title| {
                            title.spawn((
                                Text::new("GAME MENU"),
                                TextFont {
                                    font_size: TITLE_FONT_SIZE,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Spacing
                    panel.spawn(Node {
                        height: Val::Px(40.0),
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
                                                    TextFont {
                                                        font_size: 16.0,
                                                        ..default()
                                                    },
                                                    TextColor(Color::WHITE),
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

/// Spawn the settings menu UI with audio sliders and graphics settings (Phase 6)
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
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
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
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
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
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
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
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
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
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
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
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
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
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
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
    mut interaction_query: Query<(&Interaction, &MenuButton), Changed<Interaction>>,
    mut global_state: ResMut<GlobalState>,
    save_manager: Res<SaveGameManager>,
) {
    for (interaction, button) in interaction_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            // Call the unified handler using dereferenced resources so the same logic
            // is used for both mouse and keyboard-driven actions.
            handle_button_press(button, &mut global_state, &save_manager);
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
                        load_game_operation(global_state, save_manager, &filename);
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
                    save_game_operation(global_state, save_manager);
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
fn save_game_operation(global_state: &mut GlobalState, save_manager: &SaveGameManager) {
    let GameMode::Menu(_menu_state) = &global_state.0.mode else {
        return;
    };

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
) {
    match save_manager.load(selected_filename) {
        Ok(loaded_state) => {
            debug!("Game loaded successfully: {}", selected_filename);

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
                handle_button_press(&sel_button, &mut global_state, &save_manager);
            }
            MenuType::SaveLoad => {
                // Pressing Enter on a save slot should load it (if there is a save at that index)
                if selected_index < save_list.len() {
                    handle_button_press(&MenuButton::LoadGame, &mut global_state, &save_manager);
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
        handle_button_press(&MenuButton::SaveGame, &mut gs, &manager);

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
        handle_button_press(&MenuButton::Confirm, &mut gs, &manager);

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
        handle_button_press(&MenuButton::LoadGame, &mut gs, &manager);

        // After a successful load the game mode should be Exploration
        assert!(matches!(gs.0.mode, GameMode::Exploration));
    }
}
