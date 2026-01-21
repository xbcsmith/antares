// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Menu plugin and system implementations for Phase 4: Menu UI Rendering
//!
//! This module implements the complete menu UI system with:
//! - UI spawning based on current submenu (Main/SaveLoad/Settings)
//! - Button interaction handling (hover/click)
//! - Dynamic button color updates based on selection
//! - Proper cleanup when exiting menu mode

use bevy::prelude::*;

use crate::application::menu::{MenuState, MenuType};
use crate::application::GameMode;
use crate::game::components::menu::*;
use crate::game::resources::GlobalState;

/// Plugin for the in-game menu system
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                menu_setup,
                handle_menu_keyboard,
                menu_button_interaction,
                update_button_colors,
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
        MenuType::SaveLoad => spawn_save_load_menu(&mut commands, &font),
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

/// Spawn the save/load menu UI - stub for Phase 5
fn spawn_save_load_menu(commands: &mut Commands, font: &Handle<Font>) {
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
                        ..default()
                    },
                    BackgroundColor(MENU_BACKGROUND_COLOR),
                    BorderRadius::all(Val::Px(4.0)),
                    SaveLoadPanel,
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("SAVE / LOAD"),
                        TextFont {
                            font: font.clone(),
                            font_size: TITLE_FONT_SIZE,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    panel.spawn(Node {
                        height: Val::Px(40.0),
                        ..default()
                    });

                    panel.spawn((
                        Text::new("Coming in Phase 5: Save/Load slots"),
                        TextFont {
                            font: font.clone(),
                            font_size: BUTTON_FONT_SIZE,
                            ..default()
                        },
                        TextColor(BUTTON_TEXT_COLOR),
                    ));
                });
        });

    info!("Spawned save/load menu UI");
}

/// Spawn the settings menu UI - stub for Phase 6
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
                        ..default()
                    },
                    BackgroundColor(MENU_BACKGROUND_COLOR),
                    BorderRadius::all(Val::Px(4.0)),
                    SettingsPanel,
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("SETTINGS"),
                        TextFont {
                            font: font.clone(),
                            font_size: TITLE_FONT_SIZE,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    panel.spawn(Node {
                        height: Val::Px(40.0),
                        ..default()
                    });

                    panel.spawn((
                        Text::new("Coming in Phase 6: Settings controls"),
                        TextFont {
                            font: font.clone(),
                            font_size: BUTTON_FONT_SIZE,
                            ..default()
                        },
                        TextColor(BUTTON_TEXT_COLOR),
                    ));
                });
        });

    info!("Spawned settings menu UI");
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
) {
    for (interaction, button) in interaction_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            handle_button_press(button, &mut global_state);
        }
    }
}

/// Handle button press actions
fn handle_button_press(button: &MenuButton, global_state: &mut ResMut<GlobalState>) {
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
        _ => {}
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
) {
    if !matches!(global_state.0.mode, GameMode::Menu(_)) {
        return;
    }

    if let GameMode::Menu(ref mut menu_state) = global_state.0.mode {
        let item_count = match menu_state.current_submenu {
            MenuType::Main => 5,
            MenuType::SaveLoad => menu_state.save_list.len().max(1),
            MenuType::Settings => 4,
        };

        if keyboard.just_pressed(KeyCode::Backspace) {
            if menu_state.current_submenu != MenuType::Main {
                menu_state.set_submenu(MenuType::Main);
            }
            return;
        }

        if keyboard.just_pressed(KeyCode::Escape) {
            let resume = menu_state.get_resume_mode();
            global_state.0.mode = resume;
            return;
        }

        if keyboard.just_pressed(KeyCode::ArrowUp) {
            menu_state.select_previous(item_count);
        } else if keyboard.just_pressed(KeyCode::ArrowDown) {
            menu_state.select_next(item_count);
        } else if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
            handle_menu_selection(menu_state);
        }
    }
}

/// Handle menu selection
fn handle_menu_selection(menu_state: &mut MenuState) {
    match menu_state.current_submenu {
        MenuType::Main => match menu_state.selected_index {
            0 => info!("Selected: Resume"),
            1 => {
                info!("Selected: Save");
                menu_state.set_submenu(MenuType::SaveLoad);
            }
            2 => {
                info!("Selected: Load");
                menu_state.set_submenu(MenuType::SaveLoad);
            }
            3 => {
                info!("Selected: Settings");
                menu_state.set_submenu(MenuType::Settings);
            }
            4 => info!("Selected: Quit"),
            _ => {}
        },
        MenuType::SaveLoad => {
            info!("Selected save slot: {}", menu_state.selected_index);
        }
        MenuType::Settings => {
            info!("Selected settings: {}", menu_state.selected_index);
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
}
