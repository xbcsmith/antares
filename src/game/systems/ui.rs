// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::application::GameMode;
use crate::game::resources::GlobalState;
use crate::game::systems::ui_helpers::{text_style, LABEL_FONT_SIZE};
use crate::sdk::game_config::GameLogConfig;
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::ui::widget::Text;
use bevy::ui::{
    AlignItems, FlexDirection, Interaction, Overflow, PositionType, ScrollPosition, Val,
};
use bevy_egui::{egui, EguiContexts};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const GAME_LOG_PANEL_WIDTH: f32 = 300.0;
pub const GAME_LOG_PANEL_HEIGHT: f32 = 200.0;
pub const GAME_LOG_VISIBLE_LINES: usize = 12;
pub const GAME_LOG_PANEL_BACKGROUND: Color = Color::srgba(0.06, 0.09, 0.13, 0.88);
pub const GAME_LOG_VIEWPORT_BACKGROUND: Color = Color::srgba(0.03, 0.05, 0.08, 0.35);
pub const GAME_LOG_TOGGLE_KEY: KeyCode = KeyCode::KeyL;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<GameLogEvent>()
            .init_resource::<GameLog>()
            .init_resource::<GameLogUiState>()
            .init_resource::<FullscreenLogFilterState>()
            .add_systems(Startup, setup_game_log_panel)
            .add_systems(
                Update,
                (
                    consume_game_log_events,
                    toggle_game_log_panel,
                    handle_log_filter_buttons,
                    sync_game_log_panel_visibility,
                    sync_game_log_ui,
                    auto_scroll_game_log_viewport,
                    fullscreen_game_log_ui_system,
                ),
            );
    }
}

/// Category for a game log entry.
///
/// Categories are used both for visual styling and for log filtering in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogCategory {
    Combat,
    Dialogue,
    Item,
    Exploration,
    System,
}

/// Message used by gameplay systems to append to the game log without
/// directly mutating the [`GameLog`] resource.
#[derive(Message, Debug, Clone)]
pub struct GameLogEvent {
    pub text: String,
    pub category: LogCategory,
}

/// Runtime UI state for the game log panel.
#[derive(Resource, Debug, Clone)]
pub struct GameLogUiState {
    pub visible: bool,
    pub needs_scroll_to_bottom: bool,
}

impl Default for GameLogUiState {
    fn default() -> Self {
        Self {
            visible: true,
            needs_scroll_to_bottom: false,
        }
    }
}

/// Root marker for the game log panel.
#[derive(Component, Debug, Clone, Copy)]
pub struct GameLogPanelRoot;

/// Marker for the scrollable viewport inside the game log panel.
#[derive(Component, Debug, Clone, Copy)]
pub struct GameLogScrollViewport;

/// Marker for the line-list container inside the game log panel.
#[derive(Component, Debug, Clone, Copy)]
pub struct GameLogLineList;

/// Marker for an individual rendered game log line.
#[derive(Component, Debug, Clone, Copy)]
pub struct GameLogLineItem {
    pub index: usize,
}

/// Marker for a game log filter button.
#[derive(Component, Debug, Clone, Copy)]
pub struct LogFilterButton {
    pub category: LogCategory,
    pub active: bool,
}

/// Marker for the header text that displays the filtered entry count.
#[derive(Component, Debug, Clone, Copy)]
pub struct GameLogEntryCountText;

impl LogCategory {
    /// Returns all log categories in display order.
    pub fn all() -> [Self; 5] {
        [
            Self::Combat,
            Self::Dialogue,
            Self::Item,
            Self::Exploration,
            Self::System,
        ]
    }

    /// Returns the default display color for this category.
    pub fn default_color(self) -> Color {
        match self {
            Self::Combat => Color::srgb(0.86, 0.45, 0.45),
            Self::Dialogue => Color::srgb(0.85, 0.80, 0.50),
            Self::Item => Color::srgb(0.40, 0.78, 0.40),
            Self::Exploration => Color::srgb(0.55, 0.75, 0.95),
            Self::System => Color::srgb(0.70, 0.70, 0.70),
        }
    }

    /// Returns the abbreviated label shown in the filter bar.
    pub fn short_label(self) -> &'static str {
        match self {
            Self::Combat => "CMB",
            Self::Dialogue => "DLG",
            Self::Item => "ITM",
            Self::Exploration => "EXP",
            Self::System => "SYS",
        }
    }

    /// Parses a category name from configuration.
    pub fn from_config_name(name: &str) -> Option<Self> {
        match name {
            "Combat" => Some(Self::Combat),
            "Dialogue" => Some(Self::Dialogue),
            "Item" => Some(Self::Item),
            "Exploration" => Some(Self::Exploration),
            "System" => Some(Self::System),
            _ => None,
        }
    }
}

/// A typed game log entry.
///
/// Entries carry enough metadata for category-aware rendering, filtering, and
/// stable ordering.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub category: LogCategory,
    pub text: String,
    pub color: Color,
    pub sequence: u64,
}

impl std::fmt::Display for LogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl LogEntry {
    /// Compatibility helper that forwards to the underlying entry text.
    ///
    /// This preserves existing tests and transitional code that still treat
    /// log entries like string values.
    pub fn contains(&self, pattern: &str) -> bool {
        self.text.contains(pattern)
    }

    /// Compatibility helper that forwards to the underlying entry text.
    ///
    /// This preserves existing tests and transitional code that still check
    /// for string prefixes directly on log entries.
    pub fn starts_with(&self, pattern: &str) -> bool {
        self.text.starts_with(pattern)
    }

    /// Compatibility helper that forwards to the underlying entry text.
    ///
    /// This preserves existing tests and transitional code that still compare
    /// entries to exact string values during the migration.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl LogEntry {
    /// Creates a new log entry using the default color for `category`.
    pub fn new(category: LogCategory, text: impl Into<String>, sequence: u64) -> Self {
        Self {
            category,
            text: text.into(),
            color: category.default_color(),
            sequence,
        }
    }
}

#[derive(Resource, Debug)]
pub struct GameLog {
    pub entries: Vec<LogEntry>,
    pub messages: Vec<String>,
    pub filter: HashSet<LogCategory>,
    pub sequence_counter: u64,
    pub max_entries: usize,
}

impl Default for GameLog {
    fn default() -> Self {
        Self::new()
    }
}

impl GameLog {
    pub const MAX_LOG_ENTRIES: usize = 200;

    /// Create a new empty game log with all categories enabled.
    pub fn new() -> Self {
        let config = GameLogConfig::default();
        Self {
            entries: Vec::new(),
            messages: Vec::new(),
            filter: config
                .default_enabled_categories
                .iter()
                .filter_map(|name| LogCategory::from_config_name(name))
                .collect(),
            sequence_counter: 0,
            max_entries: config.max_entries,
        }
    }

    /// Apply game log configuration to the resource.
    pub fn apply_config(&mut self, config: &GameLogConfig) {
        self.max_entries = config.max_entries;
        self.filter = config
            .default_enabled_categories
            .iter()
            .filter_map(|name| LogCategory::from_config_name(name))
            .collect();

        if self.filter.is_empty() {
            self.filter = LogCategory::all().into_iter().collect();
        }

        if self.entries.len() > self.max_entries {
            let overflow = self.entries.len() - self.max_entries;
            self.entries.drain(0..overflow);
            self.messages.drain(0..overflow);
        }
    }

    /// Add a typed entry to the game log.
    pub fn add_entry(&mut self, text: String, category: LogCategory) {
        let entry = LogEntry::new(category, text.clone(), self.sequence_counter);
        self.sequence_counter = self.sequence_counter.saturating_add(1);
        self.entries.push(entry);
        self.messages.push(text);

        if self.entries.len() > self.max_entries {
            let overflow = self.entries.len() - self.max_entries;
            self.entries.drain(0..overflow);
            self.messages.drain(0..overflow);
        }
    }

    /// Add a combat entry to the game log.
    pub fn add_combat(&mut self, text: String) {
        self.add_entry(text, LogCategory::Combat);
    }

    /// Add a dialogue entry to the game log.
    pub fn add_dialogue(&mut self, text: String) {
        self.add_entry(text, LogCategory::Dialogue);
    }

    /// Add an item entry to the game log.
    pub fn add_item(&mut self, text: String) {
        self.add_entry(text, LogCategory::Item);
    }

    /// Add an exploration entry to the game log.
    pub fn add_exploration(&mut self, text: String) {
        self.add_entry(text, LogCategory::Exploration);
    }

    /// Add a system entry to the game log.
    pub fn add_system(&mut self, text: String) {
        self.add_entry(text, LogCategory::System);
    }

    /// Get all log entries.
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    /// Get only the entries enabled by the current filter.
    pub fn filtered_entries(&self) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|entry| self.filter.contains(&entry.category))
            .collect()
    }
}

/// Persistent filter state for the fullscreen game log overlay.
///
/// Stored as a Bevy resource so filter selections survive open/close cycles
/// within the same play session.
#[derive(Resource, Debug, Clone)]
pub struct FullscreenLogFilterState {
    /// Categories currently enabled in the fullscreen log filter bar.
    pub active_categories: HashSet<LogCategory>,
}

impl Default for FullscreenLogFilterState {
    fn default() -> Self {
        Self {
            active_categories: LogCategory::all().into_iter().collect(),
        }
    }
}

fn setup_game_log_panel(
    mut commands: Commands,
    mut game_log: ResMut<GameLog>,
    global_state: Option<Res<GlobalState>>,
) {
    let owned_config = global_state
        .as_ref()
        .map(|state| state.0.config.game_log.clone())
        .unwrap_or_default();
    game_log.apply_config(&owned_config);

    let filter: HashSet<LogCategory> = owned_config
        .default_enabled_categories
        .iter()
        .filter_map(|name| LogCategory::from_config_name(name))
        .collect();

    let panel_background = Color::srgba(0.06, 0.09, 0.13, owned_config.panel_opacity);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                top: Val::Px(8.0),
                width: Val::Px(owned_config.panel_width_px),
                height: Val::Px(owned_config.panel_height_px),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(panel_background),
            if owned_config.visible_by_default {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
            GameLogPanelRoot,
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    Name::new("GameLogHeader"),
                ))
                .with_children(|header| {
                    header.spawn((
                        Text::new("Game Log"),
                        text_style(LABEL_FONT_SIZE, Color::WHITE),
                    ));

                    header
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(4.0),
                                ..default()
                            },
                            Name::new("GameLogFilterBar"),
                        ))
                        .with_children(|filter_bar| {
                            for category in LogCategory::all() {
                                let active = filter.contains(&category);
                                let button_color = category.default_color().to_srgba();
                                filter_bar
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Px(32.0),
                                            height: Val::Px(20.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        BackgroundColor(if active {
                                            Color::srgba(
                                                button_color.red,
                                                button_color.green,
                                                button_color.blue,
                                                1.0,
                                            )
                                        } else {
                                            Color::srgba(
                                                button_color.red,
                                                button_color.green,
                                                button_color.blue,
                                                0.3,
                                            )
                                        }),
                                        LogFilterButton { category, active },
                                    ))
                                    .with_children(|button| {
                                        button.spawn((
                                            Text::new(category.short_label()),
                                            TextFont {
                                                font_size: 10.0,
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    });
                            }
                        });

                    header.spawn((
                        Text::new("0 entries"),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.70, 0.70, 0.70)),
                        GameLogEntryCountText,
                    ));
                });

            panel
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        overflow: Overflow::scroll_y(),
                        padding: UiRect::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(GAME_LOG_VIEWPORT_BACKGROUND),
                    ScrollPosition::default(),
                    GameLogScrollViewport,
                ))
                .with_children(|viewport| {
                    viewport.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                        GameLogLineList,
                    ));
                });
        });
}

fn toggle_game_log_panel(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    global_state: Option<Res<GlobalState>>,
    mut ui_state: ResMut<GameLogUiState>,
) {
    if global_state
        .as_ref()
        .is_some_and(|state| matches!(state.0.mode, GameMode::Combat(_)))
    {
        return;
    }

    let Some(keyboard) = keyboard else {
        return;
    };

    let toggle_key = global_state
        .as_ref()
        .and_then(|state| parse_toggle_key(&state.0.config.game_log.toggle_key))
        .unwrap_or(GAME_LOG_TOGGLE_KEY);

    if keyboard.just_pressed(toggle_key) {
        ui_state.visible = !ui_state.visible;
    }
}

fn sync_game_log_panel_visibility(
    global_state: Option<Res<GlobalState>>,
    ui_state: Res<GameLogUiState>,
    mut panel_query: Query<&mut Visibility, With<GameLogPanelRoot>>,
) {
    let in_combat = global_state
        .as_ref()
        .is_some_and(|state| matches!(state.0.mode, GameMode::Combat(_)));
    let in_fullscreen_log = global_state
        .as_ref()
        .is_some_and(|state| matches!(state.0.mode, GameMode::GameLog));
    let should_show = ui_state.visible && !in_combat && !in_fullscreen_log;

    for mut visibility in &mut panel_query {
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn consume_game_log_events(mut reader: MessageReader<GameLogEvent>, mut game_log: ResMut<GameLog>) {
    for event in reader.read() {
        game_log.add_entry(event.text.clone(), event.category);
    }
}

fn sync_game_log_ui(
    mut commands: Commands,
    game_log: Res<GameLog>,
    mut ui_state: ResMut<GameLogUiState>,
    global_state: Option<Res<GlobalState>>,
    line_list_query: Query<(Entity, Option<&Children>), With<GameLogLineList>>,
    mut entry_count_query: Query<&mut Text, With<GameLogEntryCountText>>,
) {
    if let Some(global_state) = global_state.as_ref() {
        if matches!(global_state.0.mode, GameMode::Combat(_)) {
            return;
        }

        if !matches!(
            global_state.0.mode,
            GameMode::Exploration | GameMode::Dialogue(_) | GameMode::Menu(_) | GameMode::GameLog
        ) {
            return;
        }
    }

    if !game_log.is_changed() && !ui_state.is_changed() {
        return;
    }

    let Ok((line_list_entity, children)) = line_list_query.single() else {
        return;
    };

    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let filtered = game_log.filtered_entries();
    for mut entry_count in &mut entry_count_query {
        **entry_count = format!("{} entries", filtered.len());
    }

    if filtered.is_empty() {
        return;
    }

    let start = filtered.len().saturating_sub(GAME_LOG_VISIBLE_LINES);
    let visible_entries = &filtered[start..];

    commands.entity(line_list_entity).with_children(|list| {
        for (render_idx, entry) in visible_entries.iter().enumerate() {
            list.spawn((
                Text::new(entry.text.clone()),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(entry.color),
                GameLogLineItem {
                    index: start + render_idx,
                },
            ));
        }
    });

    if game_log.is_changed() {
        ui_state.needs_scroll_to_bottom = true;
    }
}

fn auto_scroll_game_log_viewport(
    mut ui_state: ResMut<GameLogUiState>,
    mut viewport_query: Query<&mut ScrollPosition, With<GameLogScrollViewport>>,
) {
    if !ui_state.needs_scroll_to_bottom {
        return;
    }

    for mut scroll in &mut viewport_query {
        scroll.0.y = f32::MAX;
    }

    ui_state.needs_scroll_to_bottom = false;
}

fn handle_log_filter_buttons(
    mut interaction_query: Query<
        (&Interaction, &mut LogFilterButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut game_log: ResMut<GameLog>,
    mut ui_state: ResMut<GameLogUiState>,
) {
    for (interaction, mut button, mut background) in &mut interaction_query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if game_log.filter.contains(&button.category) {
            game_log.filter.remove(&button.category);
            button.active = false;
        } else {
            game_log.filter.insert(button.category);
            button.active = true;
        }

        let color = button.category.default_color().to_srgba();
        *background = BackgroundColor(if button.active {
            Color::srgba(color.red, color.green, color.blue, 1.0)
        } else {
            Color::srgba(color.red, color.green, color.blue, 0.3)
        });

        ui_state.needs_scroll_to_bottom = true;
    }
}

fn parse_toggle_key(key: &str) -> Option<KeyCode> {
    match key {
        "G" => Some(KeyCode::KeyG),
        "L" => Some(KeyCode::KeyL),
        "I" => Some(KeyCode::KeyI),
        "M" => Some(KeyCode::KeyM),
        "R" => Some(KeyCode::KeyR),
        "Escape" => Some(KeyCode::Escape),
        "Tab" => Some(KeyCode::Tab),
        "F1" => Some(KeyCode::F1),
        "F2" => Some(KeyCode::F2),
        "F3" => Some(KeyCode::F3),
        "F4" => Some(KeyCode::F4),
        "F5" => Some(KeyCode::F5),
        "F6" => Some(KeyCode::F6),
        "F7" => Some(KeyCode::F7),
        "F8" => Some(KeyCode::F8),
        "F9" => Some(KeyCode::F9),
        "F10" => Some(KeyCode::F10),
        "F11" => Some(KeyCode::F11),
        "F12" => Some(KeyCode::F12),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Full-screen game log overlay
// ─────────────────────────────────────────────────────────────────────────────

/// Renders the full-screen game log overlay using egui when the game mode is
/// [`GameMode::GameLog`].
///
/// Displays all [`GameLog`] entries in a scrollable list with category filter
/// buttons.  The overlay occupies the entire screen and is dismissed by ESC
/// (handled in `handle_global_mode_toggles`) or by pressing the fullscreen
/// toggle key again (handled in `handle_global_mode_toggles`).
fn fullscreen_game_log_ui_system(
    contexts: Option<EguiContexts>,
    global_state: Option<Res<GlobalState>>,
    game_log: Res<GameLog>,
    mut filter_state: ResMut<FullscreenLogFilterState>,
) {
    let Some(ref global_state) = global_state else {
        return;
    };

    if !matches!(global_state.0.mode, GameMode::GameLog) {
        return;
    }

    let Some(mut contexts) = contexts else {
        return;
    };

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical(|ui| {
            // ── Header row ───────────────────────────────────────────────
            ui.horizontal(|ui| {
                ui.heading("Game Log");
                ui.separator();

                // Category filter toggle buttons.
                for category in LogCategory::all() {
                    let active = filter_state.active_categories.contains(&category);
                    let label = category.short_label();
                    let response = ui.selectable_label(active, label);
                    if response.clicked() {
                        if active {
                            filter_state.active_categories.remove(&category);
                        } else {
                            filter_state.active_categories.insert(category);
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let filtered_count = game_log
                        .entries
                        .iter()
                        .filter(|e| filter_state.active_categories.contains(&e.category))
                        .count();
                    ui.label(format!("{} entries", filtered_count));
                });
            });

            ui.separator();

            // ── Scrollable entry list ────────────────────────────────────
            egui::ScrollArea::vertical()
                .id_salt("fullscreen_game_log_scroll")
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for entry in &game_log.entries {
                        if !filter_state.active_categories.contains(&entry.category) {
                            continue;
                        }

                        let color = bevy_color_to_egui(entry.color);
                        ui.label(egui::RichText::new(&entry.text).color(color));
                    }
                });
        });

        // ── Footer hint ──────────────────────────────────────────────────
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new("Press ESC or G to close")
                    .small()
                    .color(egui::Color32::GRAY),
            );
        });
    });
}

/// Converts a Bevy [`Color`] to an egui [`egui::Color32`].
fn bevy_color_to_egui(color: Color) -> egui::Color32 {
    let srgba = color.to_srgba();
    egui::Color32::from_rgba_unmultiplied(
        (srgba.red * 255.0) as u8,
        (srgba.green * 255.0) as u8,
        (srgba.blue * 255.0) as u8,
        (srgba.alpha * 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color_components(color: Color) -> [f32; 4] {
        color.to_srgba().to_f32_array()
    }

    #[test]
    fn test_fullscreen_log_filter_state_default_all_enabled() {
        let state = FullscreenLogFilterState::default();
        for category in LogCategory::all() {
            assert!(
                state.active_categories.contains(&category),
                "expected {:?} to be enabled by default",
                category
            );
        }
    }

    #[test]
    fn test_fullscreen_log_filter_state_toggle_category() {
        let mut state = FullscreenLogFilterState::default();
        state.active_categories.remove(&LogCategory::Combat);
        assert!(!state.active_categories.contains(&LogCategory::Combat));
        state.active_categories.insert(LogCategory::Combat);
        assert!(state.active_categories.contains(&LogCategory::Combat));
    }

    #[test]
    fn test_bevy_color_to_egui_converts_correctly() {
        let white = bevy_color_to_egui(Color::srgb(1.0, 1.0, 1.0));
        // Allow ±1 tolerance for floating-point rounding in sRGB conversion.
        assert!(
            white.r() >= 254,
            "expected white.r >= 254, got {}",
            white.r()
        );
        assert!(
            white.g() >= 254,
            "expected white.g >= 254, got {}",
            white.g()
        );
        assert!(
            white.b() >= 254,
            "expected white.b >= 254, got {}",
            white.b()
        );

        let red = bevy_color_to_egui(Color::srgb(1.0, 0.0, 0.0));
        assert!(red.r() >= 254, "expected red.r >= 254, got {}", red.r());
        assert_eq!(red.g(), 0);
        assert_eq!(red.b(), 0);
    }

    #[test]
    fn test_game_log_panel_renders_in_upper_left() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);
        app.update();

        let mut query = app
            .world_mut()
            .query_filtered::<&Node, With<GameLogPanelRoot>>();
        let node = query
            .iter(app.world())
            .next()
            .expect("GameLogPanelRoot should exist");

        assert_eq!(node.left, Val::Px(8.0), "panel must be at left: 8px");
        assert_eq!(node.top, Val::Px(8.0), "panel must be at top: 8px");
        assert_eq!(
            node.position_type,
            PositionType::Absolute,
            "panel must use absolute positioning"
        );
    }

    #[test]
    fn test_parse_toggle_key_g() {
        assert_eq!(parse_toggle_key("G"), Some(KeyCode::KeyG));
    }

    #[test]
    fn test_log_entry_category_defaults() {
        let mut log = GameLog::new();

        log.add_combat("combat".to_string());
        log.add_dialogue("dialogue".to_string());
        log.add_item("item".to_string());
        log.add_exploration("exploration".to_string());
        log.add_system("system".to_string());

        let entries = log.entries();
        assert_eq!(entries.len(), 5);
        assert_eq!(entries[0].category, LogCategory::Combat);
        assert_eq!(
            color_components(entries[0].color),
            color_components(LogCategory::Combat.default_color())
        );
        assert_eq!(entries[1].category, LogCategory::Dialogue);
        assert_eq!(
            color_components(entries[1].color),
            color_components(LogCategory::Dialogue.default_color())
        );
        assert_eq!(entries[2].category, LogCategory::Item);
        assert_eq!(
            color_components(entries[2].color),
            color_components(LogCategory::Item.default_color())
        );
        assert_eq!(entries[3].category, LogCategory::Exploration);
        assert_eq!(
            color_components(entries[3].color),
            color_components(LogCategory::Exploration.default_color())
        );
        assert_eq!(entries[4].category, LogCategory::System);
        assert_eq!(
            color_components(entries[4].color),
            color_components(LogCategory::System.default_color())
        );
    }

    #[test]
    fn test_log_filter_excludes_category() {
        let mut log = GameLog::new();
        log.filter.remove(&LogCategory::Combat);

        log.add_combat("attack".to_string());
        log.add_dialogue("hello".to_string());

        let filtered = log.filtered_entries();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].category, LogCategory::Dialogue);
        assert_eq!(filtered[0].text, "hello");
    }

    #[test]
    fn test_log_max_entries_ring() {
        let mut log = GameLog::new();

        for idx in 0..(GameLog::MAX_LOG_ENTRIES + 10) {
            log.add_system(format!("entry {}", idx));
        }

        assert_eq!(log.entries.len(), GameLog::MAX_LOG_ENTRIES);
        assert_eq!(
            log.entries.first().map(|entry| entry.text.as_str()),
            Some("entry 10")
        );
        assert_eq!(
            log.entries.last().map(|entry| entry.text.as_str()),
            Some("entry 209")
        );
    }

    #[test]
    fn test_log_sequence_monotonic() {
        let mut log = GameLog::new();

        log.add_system("first".to_string());
        log.add_system("second".to_string());
        log.add_system("third".to_string());

        assert!(log.entries[2].sequence > log.entries[1].sequence);
        assert!(log.entries[1].sequence > log.entries[0].sequence);
    }

    #[test]
    fn test_game_log_panel_spawns_on_startup() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);

        app.update();

        let mut query = app
            .world_mut()
            .query_filtered::<Entity, With<GameLogPanelRoot>>();
        assert!(
            query.iter(app.world()).next().is_some(),
            "expected GameLogPanelRoot to exist after startup"
        );
    }

    #[test]
    fn test_map_change_logs_exploration_entry() {
        let mut log = GameLog::new();
        let event = GameLogEvent {
            text: "Entered Test Map (7).".to_string(),
            category: LogCategory::Exploration,
        };

        log.add_entry(event.text, event.category);

        let entries = log.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].category, LogCategory::Exploration);
        assert_eq!(entries[0].text, "Entered Test Map (7).");
    }

    #[test]
    fn test_combat_feedback_mirrors_to_game_log() {
        let mut log = GameLog::new();
        let event = GameLogEvent {
            text: "Hero hits Goblin for 5 damage.".to_string(),
            category: LogCategory::Combat,
        };

        log.add_entry(event.text, event.category);

        let entries = log.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].category, LogCategory::Combat);
        assert_eq!(entries[0].text, "Hero hits Goblin for 5 damage.");
    }

    #[test]
    fn test_item_pickup_logs_item_entry() {
        let mut log = GameLog::new();
        let event = GameLogEvent {
            text: "Picked up Iron Sword.".to_string(),
            category: LogCategory::Item,
        };

        log.add_entry(event.text, event.category);

        let entries = log.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].category, LogCategory::Item);
        assert_eq!(entries[0].text, "Picked up Iron Sword.");
    }

    #[test]
    fn test_filter_button_toggles_category() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ButtonInput<KeyCode>>();
        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.add_plugins(UiPlugin);

        app.update();

        let button_entity = {
            let mut query = app
                .world_mut()
                .query_filtered::<(Entity, &LogFilterButton), With<Button>>();
            query
                .iter(app.world())
                .find(|(_, button)| button.category == LogCategory::Combat)
                .map(|(entity, _)| entity)
                .expect("combat filter button should exist")
        };

        app.world_mut()
            .entity_mut(button_entity)
            .insert(Interaction::Pressed);

        app.update();

        let log = app.world().resource::<GameLog>();
        assert!(
            !log.filter.contains(&LogCategory::Combat),
            "combat category should be removed from the filter after button press"
        );
    }

    #[test]
    fn test_game_log_panel_visibility_toggle() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ButtonInput<KeyCode>>();
        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.add_plugins(UiPlugin);

        app.update();

        app.world_mut().resource_mut::<GameLogUiState>().visible = false;
        app.update();

        let mut query = app
            .world_mut()
            .query_filtered::<&Visibility, With<GameLogPanelRoot>>();
        let visibility = query
            .iter(app.world())
            .next()
            .expect("panel visibility should exist");
        assert!(matches!(visibility, Visibility::Hidden));
    }

    #[test]
    fn test_game_log_sync_shows_filtered_entries() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ButtonInput<KeyCode>>();
        app.insert_resource(GlobalState(crate::application::GameState::new()));
        app.add_plugins(UiPlugin);

        app.update();

        {
            let mut log = app.world_mut().resource_mut::<GameLog>();
            log.filter.remove(&LogCategory::Combat);
            log.add_combat("attack".to_string());
            log.add_dialogue("hello".to_string());
        }

        app.update();

        let mut line_list_query = app
            .world_mut()
            .query_filtered::<(&Children, Entity), With<GameLogLineList>>();
        let (children, _line_list_entity) = line_list_query
            .iter(app.world())
            .next()
            .expect("line list should exist");

        assert_eq!(
            children.len(),
            1,
            "only one filtered line should be spawned"
        );

        let child = children[0];
        let text = app
            .world()
            .get::<Text>(child)
            .expect("spawned log line should have Text");
        assert_eq!(text.as_str(), "hello");
    }

    #[test]
    fn test_panel_opacity_from_config() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ButtonInput<KeyCode>>();

        let mut state = crate::application::GameState::new();
        state.config.game_log.panel_opacity = 0.5;
        app.insert_resource(GlobalState(state));

        app.add_plugins(UiPlugin);
        app.update();

        let mut query = app
            .world_mut()
            .query_filtered::<&BackgroundColor, With<GameLogPanelRoot>>();
        let background = query
            .iter(app.world())
            .next()
            .expect("panel background should exist");

        let alpha = background.0.to_srgba().alpha;
        assert!(
            (alpha - 0.5).abs() < 0.01,
            "expected panel opacity close to 0.5, got {}",
            alpha
        );
    }
}
