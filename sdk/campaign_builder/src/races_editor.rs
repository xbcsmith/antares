// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Races Editor for Campaign Builder
//!
//! This module provides a visual editor for character races with full UI
//! rendering via the `show()` method, following the standard editor pattern.
//! Uses shared UI components for consistent layout.

use crate::ui_helpers::{ActionButtons, EditorToolbar, ItemAction, ToolbarAction, TwoColumnLayout};
use antares::domain::races::{RaceDefinition, Resistances, SizeCategory, StatModifiers};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Editor state for races
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacesEditorState {
    /// All races being edited
    pub races: Vec<RaceDefinition>,

    /// Currently selected race index
    pub selected_race: Option<usize>,

    /// Editor mode
    pub mode: RacesEditorMode,

    /// Edit buffer
    pub buffer: RaceEditBuffer,

    /// Search filter
    pub search_filter: String,

    /// Unsaved changes
    pub has_unsaved_changes: bool,
}

/// Editor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RacesEditorMode {
    List,
    Creating,
    Editing,
}

/// Buffer for race form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceEditBuffer {
    pub id: String,
    pub name: String,
    pub description: String,
    // Stat modifiers as strings for text input
    pub might: String,
    pub intellect: String,
    pub personality: String,
    pub endurance: String,
    pub speed: String,
    pub accuracy: String,
    pub luck: String,
    // Resistances as strings for text input
    pub magic_resist: String,
    pub fire_resist: String,
    pub cold_resist: String,
    pub electricity_resist: String,
    pub acid_resist: String,
    pub fear_resist: String,
    pub poison_resist: String,
    pub psychic_resist: String,
    // Other fields
    pub size: SizeCategory,
    pub disablement_bit_index: String,
    pub special_abilities: String,      // Comma-separated
    pub proficiencies: String,          // Comma-separated
    pub incompatible_item_tags: String, // Comma-separated
}

impl Default for RaceEditBuffer {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            might: "0".to_string(),
            intellect: "0".to_string(),
            personality: "0".to_string(),
            endurance: "0".to_string(),
            speed: "0".to_string(),
            accuracy: "0".to_string(),
            luck: "0".to_string(),
            magic_resist: "0".to_string(),
            fire_resist: "0".to_string(),
            cold_resist: "0".to_string(),
            electricity_resist: "0".to_string(),
            acid_resist: "0".to_string(),
            fear_resist: "0".to_string(),
            poison_resist: "0".to_string(),
            psychic_resist: "0".to_string(),
            size: SizeCategory::Medium,
            disablement_bit_index: "0".to_string(),
            special_abilities: String::new(),
            proficiencies: String::new(),
            incompatible_item_tags: String::new(),
        }
    }
}

impl Default for RacesEditorState {
    fn default() -> Self {
        Self {
            races: Vec::new(),
            selected_race: None,
            mode: RacesEditorMode::List,
            buffer: RaceEditBuffer::default(),
            search_filter: String::new(),
            has_unsaved_changes: false,
        }
    }
}

impl RacesEditorState {
    /// Creates a new races editor state
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts creating a new race
    pub fn start_new_race(&mut self) {
        self.mode = RacesEditorMode::Creating;
        self.selected_race = None;
        self.buffer = RaceEditBuffer::default();
    }

    /// Starts editing an existing race
    pub fn start_edit_race(&mut self, idx: usize) {
        if idx < self.races.len() {
            let race = &self.races[idx];
            self.selected_race = Some(idx);
            self.mode = RacesEditorMode::Editing;
            self.buffer = RaceEditBuffer {
                id: race.id.clone(),
                name: race.name.clone(),
                description: race.description.clone(),
                might: race.stat_modifiers.might.to_string(),
                intellect: race.stat_modifiers.intellect.to_string(),
                personality: race.stat_modifiers.personality.to_string(),
                endurance: race.stat_modifiers.endurance.to_string(),
                speed: race.stat_modifiers.speed.to_string(),
                accuracy: race.stat_modifiers.accuracy.to_string(),
                luck: race.stat_modifiers.luck.to_string(),
                magic_resist: race.resistances.magic.to_string(),
                fire_resist: race.resistances.fire.to_string(),
                cold_resist: race.resistances.cold.to_string(),
                electricity_resist: race.resistances.electricity.to_string(),
                acid_resist: race.resistances.acid.to_string(),
                fear_resist: race.resistances.fear.to_string(),
                poison_resist: race.resistances.poison.to_string(),
                psychic_resist: race.resistances.psychic.to_string(),
                size: race.size,
                disablement_bit_index: race.disablement_bit_index.to_string(),
                special_abilities: race.special_abilities.join(", "),
                proficiencies: race.proficiencies.join(", "),
                incompatible_item_tags: race.incompatible_item_tags.join(", "),
            };
        }
    }

    /// Saves the current race from the edit buffer
    pub fn save_race(&mut self) -> Result<(), String> {
        let id = self.buffer.id.trim().to_string();
        if id.is_empty() {
            return Err("ID cannot be empty".to_string());
        }

        let name = self.buffer.name.trim().to_string();
        if name.is_empty() {
            return Err("Name cannot be empty".to_string());
        }

        // Parse stat modifiers
        let might = self
            .buffer
            .might
            .parse::<i8>()
            .map_err(|_| "Invalid Might value")?;
        let intellect = self
            .buffer
            .intellect
            .parse::<i8>()
            .map_err(|_| "Invalid Intellect value")?;
        let personality = self
            .buffer
            .personality
            .parse::<i8>()
            .map_err(|_| "Invalid Personality value")?;
        let endurance = self
            .buffer
            .endurance
            .parse::<i8>()
            .map_err(|_| "Invalid Endurance value")?;
        let speed = self
            .buffer
            .speed
            .parse::<i8>()
            .map_err(|_| "Invalid Speed value")?;
        let accuracy = self
            .buffer
            .accuracy
            .parse::<i8>()
            .map_err(|_| "Invalid Accuracy value")?;
        let luck = self
            .buffer
            .luck
            .parse::<i8>()
            .map_err(|_| "Invalid Luck value")?;

        // Parse resistances
        let magic_resist = self
            .buffer
            .magic_resist
            .parse::<u8>()
            .map_err(|_| "Invalid Magic resistance")?;
        let fire_resist = self
            .buffer
            .fire_resist
            .parse::<u8>()
            .map_err(|_| "Invalid Fire resistance")?;
        let cold_resist = self
            .buffer
            .cold_resist
            .parse::<u8>()
            .map_err(|_| "Invalid Cold resistance")?;
        let electricity_resist = self
            .buffer
            .electricity_resist
            .parse::<u8>()
            .map_err(|_| "Invalid Electricity resistance")?;
        let acid_resist = self
            .buffer
            .acid_resist
            .parse::<u8>()
            .map_err(|_| "Invalid Acid resistance")?;
        let fear_resist = self
            .buffer
            .fear_resist
            .parse::<u8>()
            .map_err(|_| "Invalid Fear resistance")?;
        let poison_resist = self
            .buffer
            .poison_resist
            .parse::<u8>()
            .map_err(|_| "Invalid Poison resistance")?;
        let psychic_resist = self
            .buffer
            .psychic_resist
            .parse::<u8>()
            .map_err(|_| "Invalid Psychic resistance")?;

        let disablement = self
            .buffer
            .disablement_bit_index
            .parse::<u8>()
            .map_err(|_| "Invalid Disablement Bit")?;

        let special_abilities: Vec<String> = self
            .buffer
            .special_abilities
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let proficiencies: Vec<String> = self
            .buffer
            .proficiencies
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let incompatible_item_tags: Vec<String> = self
            .buffer
            .incompatible_item_tags
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let race_def = RaceDefinition {
            id: id.clone(),
            name,
            description: self.buffer.description.clone(),
            stat_modifiers: StatModifiers {
                might,
                intellect,
                personality,
                endurance,
                speed,
                accuracy,
                luck,
            },
            resistances: Resistances {
                magic: magic_resist,
                fire: fire_resist,
                cold: cold_resist,
                electricity: electricity_resist,
                acid: acid_resist,
                fear: fear_resist,
                poison: poison_resist,
                psychic: psychic_resist,
            },
            special_abilities,
            size: self.buffer.size,
            disablement_bit_index: disablement,
            proficiencies,
            incompatible_item_tags,
        };

        if let Some(idx) = self.selected_race {
            self.races[idx] = race_def;
        } else {
            // Check for duplicate ID if creating new
            if self.races.iter().any(|r| r.id == id) {
                return Err("Race ID already exists".to_string());
            }
            self.races.push(race_def);
        }

        self.has_unsaved_changes = true;
        self.mode = RacesEditorMode::List;
        self.selected_race = None;
        Ok(())
    }

    /// Deletes a race at the given index
    pub fn delete_race(&mut self, idx: usize) {
        if idx < self.races.len() {
            self.races.remove(idx);
            self.has_unsaved_changes = true;
            if self.selected_race == Some(idx) {
                self.selected_race = None;
                self.mode = RacesEditorMode::List;
            }
        }
    }

    /// Cancels the current edit operation
    pub fn cancel_edit(&mut self) {
        self.mode = RacesEditorMode::List;
        self.selected_race = None;
    }

    /// Returns filtered races based on search filter
    pub fn filtered_races(&self) -> Vec<(usize, &RaceDefinition)> {
        self.races
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                self.search_filter.is_empty()
                    || r.name
                        .to_lowercase()
                        .contains(&self.search_filter.to_lowercase())
                    || r.id
                        .to_lowercase()
                        .contains(&self.search_filter.to_lowercase())
            })
            .collect()
    }

    /// Generates the next available race ID
    pub fn next_available_race_id(&self) -> String {
        let max_id = self
            .races
            .iter()
            .filter_map(|r| r.id.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        (max_id + 1).to_string()
    }

    /// Loads races from a file path
    pub fn load_from_file(&mut self, path: &std::path::Path) -> Result<(), String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
        let races: Vec<RaceDefinition> =
            ron::from_str(&content).map_err(|e| format!("Failed to parse races: {}", e))?;
        self.races = races;
        self.has_unsaved_changes = false;
        Ok(())
    }

    /// Saves races to a file path
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        let content = ron::ser::to_string_pretty(&self.races, Default::default())
            .map_err(|e| format!("Failed to serialize races: {}", e))?;
        std::fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(())
    }

    /// Main UI rendering method following standard editor signature
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `campaign_dir` - Optional campaign directory path
    /// * `races_file` - Filename for races data
    /// * `unsaved_changes` - Mutable flag for tracking unsaved changes
    /// * `status_message` - Mutable string for status messages
    /// * `file_load_merge_mode` - Whether to merge or replace when loading files
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        campaign_dir: Option<&PathBuf>,
        races_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        file_load_merge_mode: &mut bool,
    ) {
        ui.heading("🧬 Races Editor");
        ui.add_space(5.0);

        // Use shared EditorToolbar component
        let toolbar_action = EditorToolbar::new("Races")
            .with_search(&mut self.search_filter)
            .with_merge_mode(file_load_merge_mode)
            .with_total_count(self.races.len())
            .with_id_salt("races_toolbar")
            .show(ui);

        // Handle toolbar actions
        match toolbar_action {
            ToolbarAction::New => {
                self.start_new_race();
                self.buffer.id = self.next_available_race_id();
                *unsaved_changes = true;
            }
            ToolbarAction::Save => {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(races_file);
                    match self.save_to_file(&path) {
                        Ok(_) => {
                            *status_message = format!("Saved {} races", self.races.len());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to save races: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Load => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("RON", &["ron"])
                    .pick_file()
                {
                    let load_result = std::fs::read_to_string(&path).and_then(|contents| {
                        ron::from_str::<Vec<RaceDefinition>>(&contents)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    });

                    match load_result {
                        Ok(loaded_races) => {
                            if *file_load_merge_mode {
                                for race in loaded_races {
                                    if let Some(existing) =
                                        self.races.iter_mut().find(|r| r.id == race.id)
                                    {
                                        *existing = race;
                                    } else {
                                        self.races.push(race);
                                    }
                                }
                            } else {
                                self.races = loaded_races;
                            }
                            *unsaved_changes = true;
                            *status_message = format!("Loaded races from: {}", path.display());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to load races: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Import => {
                // Import not yet implemented for races
                *status_message = "Import not yet implemented for races".to_string();
            }
            ToolbarAction::Export => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("races.ron")
                    .add_filter("RON", &["ron"])
                    .save_file()
                {
                    match ron::ser::to_string_pretty(&self.races, Default::default()) {
                        Ok(contents) => match std::fs::write(&path, contents) {
                            Ok(_) => {
                                *status_message = format!("Saved races to: {}", path.display());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to save races: {}", e);
                            }
                        },
                        Err(e) => {
                            *status_message = format!("Failed to serialize races: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Reload => {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(races_file);
                    if path.exists() {
                        match self.load_from_file(&path) {
                            Ok(_) => {
                                *status_message = format!("Loaded {} races", self.races.len());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to load races: {}", e);
                            }
                        }
                    } else {
                        *status_message = "Races file does not exist".to_string();
                    }
                }
            }
            ToolbarAction::None => {}
        }

        ui.separator();

        // Main content - use TwoColumnLayout for list mode
        match self.mode {
            RacesEditorMode::List => {
                // Clone data needed for closures to avoid borrow conflicts
                let selected_race_idx = self.selected_race;
                let search_filter = self.search_filter.clone();
                let races_snapshot: Vec<(usize, RaceDefinition)> = self
                    .races
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| {
                        search_filter.is_empty()
                            || r.name
                                .to_lowercase()
                                .contains(&search_filter.to_lowercase())
                            || r.id.to_lowercase().contains(&search_filter.to_lowercase())
                    })
                    .map(|(i, r)| (i, r.clone()))
                    .collect();

                // Actions to perform after layout
                let mut action_to_perform: Option<(usize, ItemAction)> = None;
                let mut new_selection: Option<usize> = None;

                TwoColumnLayout::new("races_layout")
                    .with_left_width(250.0)
                    .show(ui, |left_ui, right_ui| {
                        // Left panel - race list
                        egui::ScrollArea::vertical()
                            .id_salt("races_list_scroll")
                            .show(left_ui, |ui: &mut egui::Ui| {
                                for (idx, race) in &races_snapshot {
                                    let is_selected = selected_race_idx == Some(*idx);
                                    let size_str = match race.size {
                                        SizeCategory::Small => "S",
                                        SizeCategory::Medium => "M",
                                        SizeCategory::Large => "L",
                                    };

                                    let response = ui.selectable_label(
                                        is_selected,
                                        format!("[{}] {} ({})", size_str, race.name, race.id),
                                    );

                                    if response.clicked() {
                                        new_selection = Some(*idx);
                                    }

                                    // Context menu
                                    response.context_menu(|ui| {
                                        if ui.button("Edit").clicked() {
                                            action_to_perform = Some((*idx, ItemAction::Edit));
                                            ui.close();
                                        }
                                        if ui.button("Delete").clicked() {
                                            action_to_perform = Some((*idx, ItemAction::Delete));
                                            ui.close();
                                        }
                                        if ui.button("Duplicate").clicked() {
                                            action_to_perform = Some((*idx, ItemAction::Duplicate));
                                            ui.close();
                                        }
                                    });
                                }
                            });

                        // Right panel - race details
                        if let Some(idx) = selected_race_idx {
                            if idx < races_snapshot.len() {
                                if let Some((_, race)) =
                                    races_snapshot.iter().find(|(i, _)| *i == idx)
                                {
                                    egui::ScrollArea::vertical()
                                        .id_salt("race_details_scroll")
                                        .show(right_ui, |ui| {
                                            ui.heading(&race.name);
                                            ui.label(format!("ID: {}", race.id));
                                            ui.label(format!("Size: {:?}", race.size));

                                            if !race.description.is_empty() {
                                                ui.add_space(5.0);
                                                ui.label(&race.description);
                                            }

                                            ui.add_space(10.0);
                                            ui.heading("Stat Modifiers");
                                            ui.horizontal(|ui| {
                                                ui.label(format!(
                                                    "Might: {:+}",
                                                    race.stat_modifiers.might
                                                ));
                                                ui.label(format!(
                                                    "Intellect: {:+}",
                                                    race.stat_modifiers.intellect
                                                ));
                                                ui.label(format!(
                                                    "Personality: {:+}",
                                                    race.stat_modifiers.personality
                                                ));
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(format!(
                                                    "Endurance: {:+}",
                                                    race.stat_modifiers.endurance
                                                ));
                                                ui.label(format!(
                                                    "Speed: {:+}",
                                                    race.stat_modifiers.speed
                                                ));
                                                ui.label(format!(
                                                    "Accuracy: {:+}",
                                                    race.stat_modifiers.accuracy
                                                ));
                                                ui.label(format!(
                                                    "Luck: {:+}",
                                                    race.stat_modifiers.luck
                                                ));
                                            });

                                            ui.add_space(10.0);
                                            ui.heading("Resistances");
                                            ui.horizontal(|ui| {
                                                ui.label(format!(
                                                    "Magic: {}%",
                                                    race.resistances.magic
                                                ));
                                                ui.label(format!(
                                                    "Fire: {}%",
                                                    race.resistances.fire
                                                ));
                                                ui.label(format!(
                                                    "Cold: {}%",
                                                    race.resistances.cold
                                                ));
                                                ui.label(format!(
                                                    "Elec: {}%",
                                                    race.resistances.electricity
                                                ));
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(format!(
                                                    "Acid: {}%",
                                                    race.resistances.acid
                                                ));
                                                ui.label(format!(
                                                    "Fear: {}%",
                                                    race.resistances.fear
                                                ));
                                                ui.label(format!(
                                                    "Poison: {}%",
                                                    race.resistances.poison
                                                ));
                                                ui.label(format!(
                                                    "Psychic: {}%",
                                                    race.resistances.psychic
                                                ));
                                            });

                                            if !race.special_abilities.is_empty() {
                                                ui.add_space(10.0);
                                                ui.heading("Special Abilities");
                                                for ability in &race.special_abilities {
                                                    ui.label(format!("• {}", ability));
                                                }
                                            }

                                            if !race.proficiencies.is_empty() {
                                                ui.add_space(10.0);
                                                ui.heading("Proficiencies");
                                                for prof in &race.proficiencies {
                                                    ui.label(format!("• {}", prof));
                                                }
                                            }

                                            if !race.incompatible_item_tags.is_empty() {
                                                ui.add_space(10.0);
                                                ui.heading("Incompatible Item Tags");
                                                for tag in &race.incompatible_item_tags {
                                                    ui.label(format!("• {}", tag));
                                                }
                                            }

                                            ui.add_space(10.0);
                                            ui.label(format!(
                                                "Disablement Bit: {} (mask: 0b{:08b})",
                                                race.disablement_bit_index,
                                                race.disablement_mask()
                                            ));

                                            // Action buttons
                                            ui.add_space(10.0);
                                            let action = ActionButtons::new()
                                                .with_edit(true)
                                                .with_delete(true)
                                                .with_duplicate(true)
                                                .show(ui);

                                            if action != ItemAction::None {
                                                action_to_perform = Some((idx, action));
                                            }
                                        });
                                }
                            }
                        } else {
                            right_ui.centered_and_justified(|ui| {
                                ui.label("Select a race to view details");
                            });
                        }
                    });

                // Apply selection change
                if let Some(idx) = new_selection {
                    self.selected_race = Some(idx);
                }

                // Handle actions after the layout
                if let Some((idx, action)) = action_to_perform {
                    match action {
                        ItemAction::Edit => {
                            self.start_edit_race(idx);
                        }
                        ItemAction::Delete => {
                            self.delete_race(idx);
                            *unsaved_changes = true;
                        }
                        ItemAction::Duplicate => {
                            if idx < self.races.len() {
                                let mut new_race = self.races[idx].clone();
                                new_race.id = format!("{}_copy", new_race.id);
                                new_race.name = format!("{} (Copy)", new_race.name);
                                self.races.push(new_race);
                                *unsaved_changes = true;
                            }
                        }
                        ItemAction::Export | ItemAction::None => {}
                    }
                }
            }
            RacesEditorMode::Creating | RacesEditorMode::Editing => {
                self.show_race_form(ui, unsaved_changes, status_message);
            }
        }
    }

    /// Shows the race editing form
    fn show_race_form(
        &mut self,
        ui: &mut egui::Ui,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        let title = if self.mode == RacesEditorMode::Creating {
            "Create New Race"
        } else {
            "Edit Race"
        };

        ui.heading(title);
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .id_salt("race_form_scroll")
            .show(ui, |ui| {
                egui::Grid::new("race_form_grid")
                    .num_columns(2)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        // Basic info
                        ui.label("ID:");
                        ui.text_edit_singleline(&mut self.buffer.id);
                        ui.end_row();

                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.buffer.name);
                        ui.end_row();

                        ui.label("Description:");
                        ui.text_edit_multiline(&mut self.buffer.description);
                        ui.end_row();

                        // Size category
                        ui.label("Size:");
                        egui::ComboBox::from_id_salt("size_combo")
                            .selected_text(format!("{:?}", self.buffer.size))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.buffer.size,
                                    SizeCategory::Small,
                                    "Small",
                                );
                                ui.selectable_value(
                                    &mut self.buffer.size,
                                    SizeCategory::Medium,
                                    "Medium",
                                );
                                ui.selectable_value(
                                    &mut self.buffer.size,
                                    SizeCategory::Large,
                                    "Large",
                                );
                            });
                        ui.end_row();

                        ui.label("Disablement Bit:");
                        ui.text_edit_singleline(&mut self.buffer.disablement_bit_index);
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.heading("Stat Modifiers (-10 to +10)");
                egui::Grid::new("stat_modifiers_grid")
                    .num_columns(4)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Might:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.might).desired_width(50.0),
                        );
                        ui.label("Intellect:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.intellect)
                                .desired_width(50.0),
                        );
                        ui.end_row();

                        ui.label("Personality:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.personality)
                                .desired_width(50.0),
                        );
                        ui.label("Endurance:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.endurance)
                                .desired_width(50.0),
                        );
                        ui.end_row();

                        ui.label("Speed:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.speed).desired_width(50.0),
                        );
                        ui.label("Accuracy:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.accuracy)
                                .desired_width(50.0),
                        );
                        ui.end_row();

                        ui.label("Luck:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.luck).desired_width(50.0),
                        );
                        ui.label("");
                        ui.label("");
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.heading("Resistances (0-100%)");
                egui::Grid::new("resistances_grid")
                    .num_columns(4)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Magic:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.magic_resist)
                                .desired_width(50.0),
                        );
                        ui.label("Fire:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.fire_resist)
                                .desired_width(50.0),
                        );
                        ui.end_row();

                        ui.label("Cold:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.cold_resist)
                                .desired_width(50.0),
                        );
                        ui.label("Electricity:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.electricity_resist)
                                .desired_width(50.0),
                        );
                        ui.end_row();

                        ui.label("Acid:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.acid_resist)
                                .desired_width(50.0),
                        );
                        ui.label("Fear:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.fear_resist)
                                .desired_width(50.0),
                        );
                        ui.end_row();

                        ui.label("Poison:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.poison_resist)
                                .desired_width(50.0),
                        );
                        ui.label("Psychic:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.psychic_resist)
                                .desired_width(50.0),
                        );
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.heading("Abilities & Proficiencies");
                egui::Grid::new("abilities_grid")
                    .num_columns(2)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Special Abilities:");
                        ui.text_edit_singleline(&mut self.buffer.special_abilities);
                        ui.end_row();

                        ui.label("Proficiencies:");
                        ui.text_edit_singleline(&mut self.buffer.proficiencies);
                        ui.end_row();

                        ui.label("Incompatible Tags:");
                        ui.text_edit_singleline(&mut self.buffer.incompatible_item_tags);
                        ui.end_row();
                    });

                ui.add_space(5.0);
                ui.label("(Separate multiple values with commas)");

                ui.add_space(20.0);

                // Form buttons
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        match self.save_race() {
                            Ok(_) => {
                                *unsaved_changes = true;
                                *status_message = "Race saved".to_string();
                            }
                            Err(e) => {
                                *status_message = format!("Error: {}", e);
                            }
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.cancel_edit();
                    }
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_races_editor_state_creation() {
        let state = RacesEditorState::new();
        assert!(state.races.is_empty());
        assert!(state.selected_race.is_none());
        assert_eq!(state.mode, RacesEditorMode::List);
    }

    #[test]
    fn test_start_new_race() {
        let mut state = RacesEditorState::new();
        state.start_new_race();
        assert_eq!(state.mode, RacesEditorMode::Creating);
    }

    #[test]
    fn test_save_race_creates_new() {
        let mut state = RacesEditorState::new();
        state.start_new_race();
        state.buffer.id = "test_race".to_string();
        state.buffer.name = "Test Race".to_string();

        let result = state.save_race();
        assert!(result.is_ok());
        assert_eq!(state.races.len(), 1);
    }

    #[test]
    fn test_save_race_empty_id_error() {
        let mut state = RacesEditorState::new();
        state.start_new_race();
        state.buffer.name = "Test Race".to_string();

        let result = state.save_race();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ID cannot be empty"));
    }

    #[test]
    fn test_save_race_empty_name_error() {
        let mut state = RacesEditorState::new();
        state.start_new_race();
        state.buffer.id = "test_race".to_string();

        let result = state.save_race();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Name cannot be empty"));
    }

    #[test]
    fn test_save_race_duplicate_id_error() {
        let mut state = RacesEditorState::new();

        // Create first race
        state.start_new_race();
        state.buffer.id = "test_race".to_string();
        state.buffer.name = "Test Race".to_string();
        state.save_race().unwrap();

        // Try to create duplicate
        state.start_new_race();
        state.buffer.id = "test_race".to_string();
        state.buffer.name = "Another Race".to_string();

        let result = state.save_race();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_delete_race() {
        let mut state = RacesEditorState::new();
        state.start_new_race();
        state.buffer.id = "test_race".to_string();
        state.buffer.name = "Test Race".to_string();
        state.save_race().unwrap();

        assert_eq!(state.races.len(), 1);
        state.delete_race(0);
        assert_eq!(state.races.len(), 0);
    }

    #[test]
    fn test_cancel_edit() {
        let mut state = RacesEditorState::new();
        state.start_new_race();
        assert_eq!(state.mode, RacesEditorMode::Creating);

        state.cancel_edit();
        assert_eq!(state.mode, RacesEditorMode::List);
    }

    #[test]
    fn test_filtered_races() {
        let mut state = RacesEditorState::new();

        // Add races
        state.races.push(RaceDefinition {
            id: "human".to_string(),
            name: "Human".to_string(),
            description: String::new(),
            stat_modifiers: StatModifiers::default(),
            resistances: Resistances::default(),
            special_abilities: vec![],
            size: SizeCategory::Medium,
            disablement_bit_index: 0,
            proficiencies: vec![],
            incompatible_item_tags: vec![],
        });
        state.races.push(RaceDefinition {
            id: "elf".to_string(),
            name: "Elf".to_string(),
            description: String::new(),
            stat_modifiers: StatModifiers::default(),
            resistances: Resistances::default(),
            special_abilities: vec![],
            size: SizeCategory::Medium,
            disablement_bit_index: 1,
            proficiencies: vec![],
            incompatible_item_tags: vec![],
        });

        // No filter - should return both
        assert_eq!(state.filtered_races().len(), 2);

        // Filter by name
        state.search_filter = "elf".to_string();
        assert_eq!(state.filtered_races().len(), 1);

        // Filter with no matches
        state.search_filter = "dwarf".to_string();
        assert_eq!(state.filtered_races().len(), 0);
    }

    #[test]
    fn test_next_available_race_id() {
        let state = RacesEditorState::new();
        assert_eq!(state.next_available_race_id(), "1");
    }

    #[test]
    fn test_start_edit_race() {
        let mut state = RacesEditorState::new();
        state.races.push(RaceDefinition {
            id: "human".to_string(),
            name: "Human".to_string(),
            description: "A versatile race".to_string(),
            stat_modifiers: StatModifiers::default(),
            resistances: Resistances::default(),
            special_abilities: vec!["adaptable".to_string()],
            size: SizeCategory::Medium,
            disablement_bit_index: 0,
            proficiencies: vec![],
            incompatible_item_tags: vec![],
        });

        state.start_edit_race(0);

        assert_eq!(state.mode, RacesEditorMode::Editing);
        assert_eq!(state.selected_race, Some(0));
        assert_eq!(state.buffer.id, "human");
        assert_eq!(state.buffer.name, "Human");
        assert_eq!(state.buffer.description, "A versatile race");
    }

    #[test]
    fn test_edit_race_saves_changes() {
        let mut state = RacesEditorState::new();
        state.races.push(RaceDefinition {
            id: "human".to_string(),
            name: "Human".to_string(),
            description: String::new(),
            stat_modifiers: StatModifiers::default(),
            resistances: Resistances::default(),
            special_abilities: vec![],
            size: SizeCategory::Medium,
            disablement_bit_index: 0,
            proficiencies: vec![],
            incompatible_item_tags: vec![],
        });

        state.start_edit_race(0);
        state.buffer.name = "Updated Human".to_string();
        state.save_race().unwrap();

        assert_eq!(state.races[0].name, "Updated Human");
    }

    #[test]
    fn test_race_edit_buffer_default() {
        let buffer = RaceEditBuffer::default();
        assert!(buffer.id.is_empty());
        assert!(buffer.name.is_empty());
        assert_eq!(buffer.might, "0");
        assert_eq!(buffer.size, SizeCategory::Medium);
    }

    #[test]
    fn test_editor_mode_transitions() {
        let mut state = RacesEditorState::new();

        // List -> Creating
        assert_eq!(state.mode, RacesEditorMode::List);
        state.start_new_race();
        assert_eq!(state.mode, RacesEditorMode::Creating);

        // Creating -> List (cancel)
        state.cancel_edit();
        assert_eq!(state.mode, RacesEditorMode::List);

        // Add a race then edit
        state.start_new_race();
        state.buffer.id = "test".to_string();
        state.buffer.name = "Test".to_string();
        state.save_race().unwrap();

        // List -> Editing
        state.start_edit_race(0);
        assert_eq!(state.mode, RacesEditorMode::Editing);

        // Editing -> List (save)
        state.buffer.name = "Updated".to_string();
        state.save_race().unwrap();
        assert_eq!(state.mode, RacesEditorMode::List);
    }
}
