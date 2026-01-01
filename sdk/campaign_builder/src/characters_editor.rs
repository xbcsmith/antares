// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Characters Editor for Campaign Builder
//!
//! This module provides a visual editor for character definitions with full UI
//! rendering via the `show()` method, following the standard editor pattern.
//! Uses shared UI components for consistent layout.

use crate::ui_helpers::{
    autocomplete_class_selector, autocomplete_item_list_selector, autocomplete_item_selector,
    autocomplete_portrait_selector, autocomplete_race_selector, extract_portrait_candidates,
    resolve_portrait_path, ActionButtons, EditorToolbar, ItemAction, ToolbarAction,
    TwoColumnLayout,
};
use antares::domain::character::{Alignment, Sex};
use antares::domain::character_definition::{
    BaseStats, CharacterDefinition, CharacterDefinitionId, StartingEquipment,
};
use antares::domain::classes::ClassDefinition;
use antares::domain::items::types::Item;
use antares::domain::races::RaceDefinition;
use antares::domain::types::{ItemId, RaceId};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Editor state for characters
#[derive(Serialize, Deserialize)]
pub struct CharactersEditorState {
    /// All characters being edited
    pub characters: Vec<CharacterDefinition>,

    /// Currently selected character index
    pub selected_character: Option<usize>,

    /// Editor mode
    pub mode: CharactersEditorMode,

    /// Edit buffer
    pub buffer: CharacterEditBuffer,

    /// Search filter
    pub search_filter: String,

    /// Filter: by race ID
    pub filter_race: Option<String>,

    /// Filter: by class ID
    pub filter_class: Option<String>,

    /// Filter: by alignment
    pub filter_alignment: Option<Alignment>,

    /// Filter: premade only
    pub filter_premade_only: bool,

    /// Unsaved changes
    pub has_unsaved_changes: bool,

    /// Whether the portrait grid picker popup is open
    #[serde(skip)]
    pub portrait_picker_open: bool,

    /// Cached portrait textures for grid display
    #[serde(skip)]
    pub portrait_textures: HashMap<String, Option<egui::TextureHandle>>,

    /// Available portrait IDs (cached from directory scan)
    #[serde(skip)]
    pub available_portraits: Vec<String>,

    /// Last campaign directory (to detect changes)
    #[serde(skip)]
    pub last_campaign_dir: Option<PathBuf>,
}

/// Editor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharactersEditorMode {
    /// Viewing list of characters
    List,
    /// Creating a new character
    Add,
    /// Editing an existing character
    Edit,
}

/// Buffer for character form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterEditBuffer {
    pub id: String,
    pub name: String,
    pub race_id: String,
    pub class_id: String,
    pub sex: Sex,
    pub alignment: Alignment,
    // Base stats as strings for text input
    pub might: String,
    pub intellect: String,
    pub personality: String,
    pub endurance: String,
    pub speed: String,
    pub accuracy: String,
    pub luck: String,
    // Other fields
    pub portrait_id: String,
    pub starting_gold: String,
    pub starting_gems: String,
    pub starting_food: String,
    pub description: String,
    pub is_premade: bool,
    // Starting items as typed vector of IDs
    pub starting_items: Vec<ItemId>,
    // Starting equipment (using ItemId, 0 = empty slot)
    pub weapon_id: ItemId,
    pub armor_id: ItemId,
    pub shield_id: ItemId,
    pub helmet_id: ItemId,
    pub boots_id: ItemId,
    pub accessory1_id: ItemId,
    pub accessory2_id: ItemId,
}

impl Default for CharacterEditBuffer {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            race_id: String::new(),
            class_id: String::new(),
            sex: Sex::Male,
            alignment: Alignment::Neutral,
            might: "10".to_string(),
            intellect: "10".to_string(),
            personality: "10".to_string(),
            endurance: "10".to_string(),
            speed: "10".to_string(),
            accuracy: "10".to_string(),
            luck: "10".to_string(),
            portrait_id: String::new(),
            starting_gold: "100".to_string(),
            starting_gems: "0".to_string(),
            starting_food: "10".to_string(),
            description: String::new(),
            is_premade: true,
            starting_items: Vec::new(),
            weapon_id: 0,
            armor_id: 0,
            shield_id: 0,
            helmet_id: 0,
            boots_id: 0,
            accessory1_id: 0,
            accessory2_id: 0,
        }
    }
}

impl Default for CharactersEditorState {
    fn default() -> Self {
        Self {
            characters: Vec::new(),
            selected_character: None,
            mode: CharactersEditorMode::List,
            buffer: CharacterEditBuffer::default(),
            search_filter: String::new(),
            filter_race: None,
            filter_class: None,
            filter_alignment: None,
            filter_premade_only: false,
            has_unsaved_changes: false,
            portrait_picker_open: false,
            portrait_textures: HashMap::new(),
            available_portraits: Vec::new(),
            last_campaign_dir: None,
        }
    }
}

impl CharactersEditorState {
    /// Creates a new characters editor state
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts creating a new character
    pub fn start_new_character(&mut self) {
        self.mode = CharactersEditorMode::Add;
        self.selected_character = None;
        self.buffer = CharacterEditBuffer::default();
    }

    /// Starts editing an existing character
    pub fn start_edit_character(&mut self, idx: usize) {
        if idx < self.characters.len() {
            let character = &self.characters[idx];
            self.selected_character = Some(idx);
            self.mode = CharactersEditorMode::Edit;
            self.buffer = CharacterEditBuffer {
                id: character.id.clone(),
                name: character.name.clone(),
                race_id: character.race_id.clone(),
                class_id: character.class_id.clone(),
                sex: character.sex,
                alignment: character.alignment,
                might: character.base_stats.might.to_string(),
                intellect: character.base_stats.intellect.to_string(),
                personality: character.base_stats.personality.to_string(),
                endurance: character.base_stats.endurance.to_string(),
                speed: character.base_stats.speed.to_string(),
                accuracy: character.base_stats.accuracy.to_string(),
                luck: character.base_stats.luck.to_string(),
                portrait_id: character.portrait_id.to_string(),
                starting_gold: character.starting_gold.to_string(),
                starting_gems: character.starting_gems.to_string(),
                starting_food: character.starting_food.to_string(),
                description: character.description.clone(),
                is_premade: character.is_premade,
                starting_items: character.starting_items.clone(),
                weapon_id: character.starting_equipment.weapon.unwrap_or(0),
                armor_id: character.starting_equipment.armor.unwrap_or(0),
                shield_id: character.starting_equipment.shield.unwrap_or(0),
                helmet_id: character.starting_equipment.helmet.unwrap_or(0),
                boots_id: character.starting_equipment.boots.unwrap_or(0),
                accessory1_id: character.starting_equipment.accessory1.unwrap_or(0),
                accessory2_id: character.starting_equipment.accessory2.unwrap_or(0),
            };
        }
    }

    /// Saves the current character from the edit buffer
    pub fn save_character(&mut self) -> Result<(), String> {
        let id = self.buffer.id.trim().to_string();
        if id.is_empty() {
            return Err("ID cannot be empty".to_string());
        }

        let name = self.buffer.name.trim().to_string();
        if name.is_empty() {
            return Err("Name cannot be empty".to_string());
        }

        let race_id = self.buffer.race_id.trim().to_string();
        if race_id.is_empty() {
            return Err("Race ID cannot be empty".to_string());
        }

        let class_id = self.buffer.class_id.trim().to_string();
        if class_id.is_empty() {
            return Err("Class ID cannot be empty".to_string());
        }

        // Parse base stats
        let might = self
            .buffer
            .might
            .parse::<u8>()
            .map_err(|_| "Invalid Might value")?;
        let intellect = self
            .buffer
            .intellect
            .parse::<u8>()
            .map_err(|_| "Invalid Intellect value")?;
        let personality = self
            .buffer
            .personality
            .parse::<u8>()
            .map_err(|_| "Invalid Personality value")?;
        let endurance = self
            .buffer
            .endurance
            .parse::<u8>()
            .map_err(|_| "Invalid Endurance value")?;
        let speed = self
            .buffer
            .speed
            .parse::<u8>()
            .map_err(|_| "Invalid Speed value")?;
        let accuracy = self
            .buffer
            .accuracy
            .parse::<u8>()
            .map_err(|_| "Invalid Accuracy value")?;
        let luck = self
            .buffer
            .luck
            .parse::<u8>()
            .map_err(|_| "Invalid Luck value")?;

        // Parse other fields
        // Portrait IDs are now strings (filename stems). Accept whatever the user typed
        // and store it as a trimmed string. An empty string indicates no portrait.
        let portrait_id = self.buffer.portrait_id.trim().to_string();
        let starting_gold = self
            .buffer
            .starting_gold
            .parse::<u32>()
            .map_err(|_| "Invalid Starting Gold")?;
        let starting_gems = self
            .buffer
            .starting_gems
            .parse::<u32>()
            .map_err(|_| "Invalid Starting Gems")?;
        let starting_food = self
            .buffer
            .starting_food
            .parse::<u8>()
            .map_err(|_| "Invalid Starting Food")?;

        // Starting items as typed Vec<ItemId> from the edit buffer
        let starting_items: Vec<ItemId> = self.buffer.starting_items.clone();

        // Starting equipment - convert 0 to None for optional fields
        let weapon = if self.buffer.weapon_id == 0 {
            None
        } else {
            Some(self.buffer.weapon_id)
        };
        let armor = if self.buffer.armor_id == 0 {
            None
        } else {
            Some(self.buffer.armor_id)
        };
        let shield = if self.buffer.shield_id == 0 {
            None
        } else {
            Some(self.buffer.shield_id)
        };
        let helmet = if self.buffer.helmet_id == 0 {
            None
        } else {
            Some(self.buffer.helmet_id)
        };
        let boots = if self.buffer.boots_id == 0 {
            None
        } else {
            Some(self.buffer.boots_id)
        };
        let accessory1 = if self.buffer.accessory1_id == 0 {
            None
        } else {
            Some(self.buffer.accessory1_id)
        };
        let accessory2 = if self.buffer.accessory2_id == 0 {
            None
        } else {
            Some(self.buffer.accessory2_id)
        };

        let character = CharacterDefinition {
            id: id.clone(),
            name,
            race_id,
            class_id,
            sex: self.buffer.sex,
            alignment: self.buffer.alignment,
            base_stats: BaseStats::new(
                might,
                intellect,
                personality,
                endurance,
                speed,
                accuracy,
                luck,
            ),
            portrait_id,
            starting_gold,
            starting_gems,
            starting_food,
            starting_items,
            starting_equipment: StartingEquipment {
                weapon,
                armor,
                shield,
                helmet,
                boots,
                accessory1,
                accessory2,
            },
            description: self.buffer.description.clone(),
            is_premade: self.buffer.is_premade,
        };

        if let Some(idx) = self.selected_character {
            self.characters[idx] = character;
        } else {
            // Check for duplicate ID if creating new
            if self.characters.iter().any(|c| c.id == id) {
                return Err("Character ID already exists".to_string());
            }
            self.characters.push(character);
        }

        self.has_unsaved_changes = true;
        self.mode = CharactersEditorMode::List;
        self.selected_character = None;
        Ok(())
    }

    /// Deletes a character at the given index
    pub fn delete_character(&mut self, idx: usize) {
        if idx < self.characters.len() {
            self.characters.remove(idx);
            self.has_unsaved_changes = true;
            if self.selected_character == Some(idx) {
                self.selected_character = None;
                self.mode = CharactersEditorMode::List;
            }
        }
    }

    /// Cancels the current edit operation
    pub fn cancel_edit(&mut self) {
        self.mode = CharactersEditorMode::List;
        self.selected_character = None;
    }

    /// Returns filtered characters based on search filter and other filters
    pub fn filtered_characters(&self) -> Vec<(usize, &CharacterDefinition)> {
        self.characters
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                // Search filter
                let matches_search = self.search_filter.is_empty()
                    || c.name
                        .to_lowercase()
                        .contains(&self.search_filter.to_lowercase())
                    || c.id
                        .to_lowercase()
                        .contains(&self.search_filter.to_lowercase());

                // Race filter
                let matches_race = self
                    .filter_race
                    .as_ref()
                    .map(|r| c.race_id == *r)
                    .unwrap_or(true);

                // Class filter
                let matches_class = self
                    .filter_class
                    .as_ref()
                    .map(|cl| c.class_id == *cl)
                    .unwrap_or(true);

                // Alignment filter
                let matches_alignment = self
                    .filter_alignment
                    .map(|a| c.alignment == a)
                    .unwrap_or(true);

                // Premade filter
                let matches_premade = !self.filter_premade_only || c.is_premade;

                matches_search
                    && matches_race
                    && matches_class
                    && matches_alignment
                    && matches_premade
            })
            .collect()
    }

    /// Generates the next available character ID
    pub fn next_available_character_id(&self) -> String {
        let mut idx = 1;
        loop {
            let id = format!("character_{}", idx);
            if !self.characters.iter().any(|c| c.id == id) {
                return id;
            }
            idx += 1;
        }
    }

    /// Clears all filters
    pub fn clear_filters(&mut self) {
        self.filter_race = None;
        self.filter_class = None;
        self.filter_alignment = None;
        self.filter_premade_only = false;
    }

    /// Loads a portrait texture from file and caches it
    ///
    /// This method attempts to load a portrait image from the campaign directory,
    /// decode it, convert it to egui's ColorImage format, and cache it as a TextureHandle.
    /// If the texture is already cached, returns the cached version.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context for texture registration
    /// * `campaign_dir` - The campaign directory containing assets/portraits
    /// * `portrait_id` - The portrait ID (filename stem) to load
    ///
    /// # Returns
    ///
    /// Returns `Some(&TextureHandle)` if the image was successfully loaded and cached,
    /// or `None` if the image could not be loaded (file not found, decode error, etc.).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::characters_editor::CharactersEditorState;
    /// use std::path::PathBuf;
    ///
    /// let mut state = CharactersEditorState::new();
    /// let campaign_dir = PathBuf::from("/path/to/campaign");
    /// // In egui context:
    /// // let texture = state.load_portrait_texture(ctx, Some(&campaign_dir), "0");
    /// ```
    pub fn load_portrait_texture(
        &mut self,
        ctx: &egui::Context,
        campaign_dir: Option<&PathBuf>,
        portrait_id: &str,
    ) -> bool {
        // Check if already cached
        if self.portrait_textures.contains_key(portrait_id) {
            return self.portrait_textures.get(portrait_id).unwrap().is_some();
        }

        // Attempt to load and decode image
        let texture_handle = (|| {
            let path = resolve_portrait_path(campaign_dir, portrait_id)?;

            // Read image file
            let image_bytes = std::fs::read(&path).ok()?;

            // Decode image using image crate
            let dynamic_image = image::load_from_memory(&image_bytes).ok()?;

            // Convert to RGBA8
            let rgba_image = dynamic_image.to_rgba8();
            let size = [rgba_image.width() as usize, rgba_image.height() as usize];
            let pixels = rgba_image.as_flat_samples();

            // Create egui ColorImage
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

            // Register texture with egui
            let texture_handle = ctx.load_texture(
                format!("portrait_{}", portrait_id),
                color_image,
                egui::TextureOptions::LINEAR,
            );

            Some(texture_handle)
        })();

        // Cache result (even None for failed loads to avoid repeated attempts)
        let loaded = texture_handle.is_some();
        self.portrait_textures
            .insert(portrait_id.to_string(), texture_handle);

        loaded
    }

    /// Shows portrait grid picker popup for visual portrait selection
    ///
    /// Displays a popup window with a grid of portrait thumbnails that the user can click to select.
    /// The popup is modal and closes when a portrait is selected or the close button is clicked.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context for rendering
    /// * `campaign_dir` - The campaign directory containing assets/portraits
    ///
    /// # Returns
    ///
    /// Returns `Some(portrait_id)` if the user clicked on a portrait to select it,
    /// or `None` if no selection was made this frame.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::characters_editor::CharactersEditorState;
    /// use std::path::PathBuf;
    ///
    /// let mut state = CharactersEditorState::new();
    /// let campaign_dir = PathBuf::from("/path/to/campaign");
    /// // In egui context:
    /// // if let Some(selected_id) = state.show_portrait_grid_picker(ctx, Some(&campaign_dir)) {
    /// //     println!("Selected portrait: {}", selected_id);
    /// // }
    /// ```
    pub fn show_portrait_grid_picker(
        &mut self,
        ctx: &egui::Context,
        campaign_dir: Option<&PathBuf>,
    ) -> Option<String> {
        let mut selected_portrait: Option<String> = None;

        // Clone the portraits list to avoid borrow issues
        let available_portraits = self.available_portraits.clone();

        egui::Window::new("Select Portrait")
            .collapsible(false)
            .resizable(true)
            .default_width(400.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.label("Click a portrait to select:");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Display portraits in a grid with 4 columns
                    const COLUMNS: usize = 4;
                    const THUMBNAIL_SIZE: f32 = 80.0;

                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                    let total_portraits = available_portraits.len();
                    let rows = total_portraits.div_ceil(COLUMNS);

                    for row in 0..rows {
                        ui.horizontal(|ui| {
                            for col in 0..COLUMNS {
                                let idx = row * COLUMNS + col;
                                if idx >= total_portraits {
                                    break;
                                }

                                let portrait_id = &available_portraits[idx];

                                ui.vertical(|ui| {
                                    // Try to load texture
                                    self.load_portrait_texture(ctx, campaign_dir, portrait_id);
                                    let has_texture = self
                                        .portrait_textures
                                        .get(portrait_id)
                                        .and_then(|opt| opt.as_ref())
                                        .is_some();

                                    // Create image button or placeholder
                                    let button_response = if has_texture {
                                        let texture = self
                                            .portrait_textures
                                            .get(portrait_id)
                                            .unwrap()
                                            .as_ref()
                                            .unwrap();
                                        ui.add(
                                            egui::Button::image(
                                                egui::Image::new(texture).fit_to_exact_size(
                                                    egui::vec2(THUMBNAIL_SIZE, THUMBNAIL_SIZE),
                                                ),
                                            )
                                            .frame(true),
                                        )
                                    } else {
                                        // Placeholder for failed/missing images
                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(THUMBNAIL_SIZE, THUMBNAIL_SIZE),
                                            egui::Sense::click(),
                                        );
                                        ui.painter().rect_filled(
                                            rect,
                                            2.0,
                                            egui::Color32::from_gray(50),
                                        );
                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            "?",
                                            egui::FontId::proportional(24.0),
                                            egui::Color32::from_gray(150),
                                        );
                                        response
                                    };

                                    // Check if clicked
                                    if button_response.clicked() {
                                        selected_portrait = Some(portrait_id.clone());
                                        self.portrait_picker_open = false;
                                    }

                                    // Show portrait ID below thumbnail
                                    ui.label(
                                        egui::RichText::new(portrait_id)
                                            .size(10.0)
                                            .color(egui::Color32::from_gray(200)),
                                    );
                                });
                            }
                        });
                    }

                    // Show message if no portraits found
                    if total_portraits == 0 {
                        ui.label("No portraits found in campaign assets/portraits directory.");
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        self.portrait_picker_open = false;
                    }
                });
            });

        selected_portrait
    }

    /// Loads characters from a file path
    pub fn load_from_file(&mut self, path: &std::path::Path) -> Result<(), String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
        let characters: Vec<CharacterDefinition> =
            ron::from_str(&content).map_err(|e| format!("Failed to parse characters: {}", e))?;
        self.characters = characters;
        self.has_unsaved_changes = false;
        Ok(())
    }

    /// Saves characters to a file path
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        let content = ron::ser::to_string_pretty(&self.characters, Default::default())
            .map_err(|e| format!("Failed to serialize characters: {}", e))?;
        std::fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(())
    }

    /// Main UI rendering method following standard editor signature
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `races` - Available races for dropdown selection
    /// * `classes` - Available classes for dropdown selection
    /// * `items` - Available items for equipment/item selection
    /// * `campaign_dir` - Optional campaign directory path
    /// * `characters_file` - Filename for characters data
    /// * `unsaved_changes` - Mutable flag for tracking unsaved changes
    /// * `status_message` - Mutable string for status messages
    /// * `file_load_merge_mode` - Whether to merge or replace when loading files
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        races: &[RaceDefinition],
        classes: &[ClassDefinition],
        items: &[Item],
        campaign_dir: Option<&PathBuf>,
        characters_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        file_load_merge_mode: &mut bool,
    ) {
        // Scan portraits if campaign directory changed
        let campaign_dir_changed = match (&self.last_campaign_dir, campaign_dir) {
            (None, Some(_)) => true,
            (Some(_), None) => true,
            (Some(last), Some(current)) => last != current,
            (None, None) => false,
        };

        if campaign_dir_changed {
            self.available_portraits = extract_portrait_candidates(campaign_dir);
            self.last_campaign_dir = campaign_dir.cloned();
        }

        ui.heading("👤 Characters Editor");
        ui.add_space(5.0);

        // Use shared EditorToolbar component
        let toolbar_action = EditorToolbar::new("Characters")
            .with_search(&mut self.search_filter)
            .with_merge_mode(file_load_merge_mode)
            .with_total_count(self.characters.len())
            .with_id_salt("characters_toolbar")
            .show(ui);

        // Handle toolbar actions
        match toolbar_action {
            ToolbarAction::New => {
                self.start_new_character();
                self.buffer.id = self.next_available_character_id();
                *unsaved_changes = true;
            }
            ToolbarAction::Save => {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(characters_file);
                    match self.save_to_file(&path) {
                        Ok(_) => {
                            *status_message = format!("Saved {} characters", self.characters.len());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to save characters: {}", e);
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
                        ron::from_str::<Vec<CharacterDefinition>>(&contents)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    });

                    match load_result {
                        Ok(loaded_characters) => {
                            if *file_load_merge_mode {
                                for character in loaded_characters {
                                    if let Some(existing) =
                                        self.characters.iter_mut().find(|c| c.id == character.id)
                                    {
                                        *existing = character;
                                    } else {
                                        self.characters.push(character);
                                    }
                                }
                            } else {
                                self.characters = loaded_characters;
                            }
                            *unsaved_changes = true;
                            *status_message = format!("Loaded characters from: {}", path.display());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to load characters: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Import => {
                *status_message = "Import not yet implemented for characters".to_string();
            }
            ToolbarAction::Export => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("characters.ron")
                    .add_filter("RON", &["ron"])
                    .save_file()
                {
                    match ron::ser::to_string_pretty(&self.characters, Default::default()) {
                        Ok(contents) => match std::fs::write(&path, contents) {
                            Ok(_) => {
                                *status_message =
                                    format!("Exported characters to: {}", path.display());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to export characters: {}", e);
                            }
                        },
                        Err(e) => {
                            *status_message = format!("Failed to serialize characters: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Reload => {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(characters_file);
                    if path.exists() {
                        match self.load_from_file(&path) {
                            Ok(_) => {
                                *status_message =
                                    format!("Loaded {} characters", self.characters.len());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to load characters: {}", e);
                            }
                        }
                    } else {
                        *status_message = "Characters file does not exist".to_string();
                    }
                }
            }
            ToolbarAction::None => {}
        }

        // Show filters
        self.show_filters(ui, races, classes);

        ui.separator();

        // Main content - use TwoColumnLayout for list mode
        match self.mode {
            CharactersEditorMode::List => {
                self.show_list(ui, items, campaign_dir, unsaved_changes);
            }
            CharactersEditorMode::Add | CharactersEditorMode::Edit => {
                self.show_character_form(ui, races, classes, items);
            }
        }

        // Show portrait grid picker popup if open
        if self.portrait_picker_open {
            if let Some(selected_id) = self.show_portrait_grid_picker(ui.ctx(), campaign_dir) {
                self.buffer.portrait_id = selected_id;
                *unsaved_changes = true;
            }
        }
    }

    /// Show filter controls
    fn show_filters(
        &mut self,
        ui: &mut egui::Ui,
        races: &[RaceDefinition],
        classes: &[ClassDefinition],
    ) {
        ui.horizontal(|ui| {
            ui.label("Filters:");

            // Race filter dropdown
            egui::ComboBox::from_id_salt("filter_race")
                .selected_text(self.filter_race.as_deref().unwrap_or("All Races"))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.filter_race.is_none(), "All Races")
                        .clicked()
                    {
                        self.filter_race = None;
                    }
                    for race in races {
                        if ui
                            .selectable_label(
                                self.filter_race.as_ref() == Some(&race.id),
                                &race.name,
                            )
                            .clicked()
                        {
                            self.filter_race = Some(race.id.clone());
                        }
                    }
                });

            // Class filter dropdown
            egui::ComboBox::from_id_salt("filter_class")
                .selected_text(self.filter_class.as_deref().unwrap_or("All Classes"))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.filter_class.is_none(), "All Classes")
                        .clicked()
                    {
                        self.filter_class = None;
                    }
                    for class in classes {
                        if ui
                            .selectable_label(
                                self.filter_class.as_ref() == Some(&class.id),
                                &class.name,
                            )
                            .clicked()
                        {
                            self.filter_class = Some(class.id.clone());
                        }
                    }
                });

            // Alignment filter dropdown
            egui::ComboBox::from_id_salt("filter_alignment")
                .selected_text(
                    self.filter_alignment
                        .map(alignment_name)
                        .unwrap_or("All Alignments"),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.filter_alignment.is_none(), "All Alignments")
                        .clicked()
                    {
                        self.filter_alignment = None;
                    }
                    for alignment in [Alignment::Good, Alignment::Neutral, Alignment::Evil] {
                        if ui
                            .selectable_label(
                                self.filter_alignment == Some(alignment),
                                alignment_name(alignment),
                            )
                            .clicked()
                        {
                            self.filter_alignment = Some(alignment);
                        }
                    }
                });

            // Premade only checkbox
            ui.checkbox(&mut self.filter_premade_only, "Premade Only");

            // Clear filters button
            if ui.button("Clear Filters").clicked() {
                self.clear_filters();
            }
        });
    }

    /// Show list view with two-column layout
    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        items: &[Item],
        campaign_dir: Option<&PathBuf>,
        unsaved_changes: &mut bool,
    ) {
        // Clone data needed for closures to avoid borrow conflicts
        let selected_character_idx = self.selected_character;
        let filtered_characters: Vec<(usize, CharacterDefinition)> = self
            .filtered_characters()
            .into_iter()
            .map(|(idx, c)| (idx, c.clone()))
            .collect();

        let mut action_idx: Option<usize> = None;
        let mut action_type = ItemAction::None;
        let mut select_idx: Option<usize> = None;

        TwoColumnLayout::new("characters_list")
            .with_left_width(350.0)
            .show_split(
                ui,
                |left_ui| {
                    // Left panel: character list
                    egui::ScrollArea::vertical()
                        .id_salt("characters_scroll")
                        .show(left_ui, |ui| {
                            if filtered_characters.is_empty() {
                                ui.label("No characters found. Click 'New' to create one.");
                            } else {
                                for (original_idx, character) in &filtered_characters {
                                    let is_selected = selected_character_idx == Some(*original_idx);

                                    // Character info
                                    let label = format!(
                                        "{} ({} {})",
                                        character.name, character.race_id, character.class_id
                                    );
                                    let response = ui.selectable_label(is_selected, label);

                                    if response.clicked() {
                                        select_idx = Some(*original_idx);
                                    }

                                    // Show character type badge
                                    ui.horizontal(|ui| {
                                        ui.add_space(20.0);
                                        if character.is_premade {
                                            ui.label(
                                                egui::RichText::new("⭐ Premade")
                                                    .small()
                                                    .color(egui::Color32::GOLD),
                                            );
                                        } else {
                                            ui.label(
                                                egui::RichText::new("📋 Template")
                                                    .small()
                                                    .color(egui::Color32::LIGHT_BLUE),
                                            );
                                        }
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "| {} | ID: {}",
                                                alignment_name(character.alignment),
                                                character.id
                                            ))
                                            .small()
                                            .weak(),
                                        );
                                    });
                                    ui.add_space(4.0);
                                }
                            }
                        });
                },
                |right_ui| {
                    // Right panel: preview of selected character
                    if let Some(idx) = selected_character_idx {
                        if let Some(character) = self.characters.get(idx).cloned() {
                            // Action buttons (correct placement - in RIGHT panel)
                            let action = ActionButtons::new()
                                .enabled(true)
                                .with_edit(true)
                                .with_delete(true)
                                .with_duplicate(true)
                                .show(right_ui);

                            if action != ItemAction::None {
                                action_idx = Some(idx);
                                action_type = action;
                            }

                            right_ui.separator();
                            self.show_character_preview(right_ui, &character, items, campaign_dir);
                        }
                    } else {
                        right_ui.label("Select a character to view details.");
                    }
                },
            );

        // Handle selection
        if let Some(idx) = select_idx {
            self.selected_character = Some(idx);
        }

        // Handle actions
        if let Some(idx) = action_idx {
            match action_type {
                ItemAction::Edit => {
                    self.start_edit_character(idx);
                    *unsaved_changes = true;
                }
                ItemAction::Delete => {
                    self.delete_character(idx);
                    *unsaved_changes = true;
                }
                ItemAction::Duplicate => {
                    if let Some(character) = self.characters.get(idx).cloned() {
                        let new_id = self.next_available_character_id();
                        let mut new_character = character;
                        new_character.id = new_id.clone();
                        new_character.name = format!("{} (Copy)", new_character.name);
                        self.characters.push(new_character);
                        *unsaved_changes = true;
                    }
                }
                _ => {}
            }
        }
    }

    /// Show character preview panel
    fn show_character_preview(
        &mut self,
        ui: &mut egui::Ui,
        character: &CharacterDefinition,
        items: &[Item],
        campaign_dir: Option<&PathBuf>,
    ) {
        // Display portrait at the top of the preview
        ui.horizontal(|ui| {
            // Portrait display (left side)
            let portrait_size = egui::vec2(128.0, 128.0);

            // Try to load the portrait texture
            let has_texture = self.load_portrait_texture(
                ui.ctx(),
                campaign_dir,
                &character.portrait_id.to_string(),
            );

            if has_texture {
                if let Some(Some(texture)) = self
                    .portrait_textures
                    .get(&character.portrait_id.to_string())
                {
                    ui.add(egui::Image::new(texture).fit_to_exact_size(portrait_size));
                } else {
                    // Show placeholder if texture failed to load
                    show_portrait_placeholder(ui, portrait_size);
                }
            } else {
                // Show placeholder if no portrait path found
                show_portrait_placeholder(ui, portrait_size);
            }

            ui.add_space(10.0);

            // Character name and basic info (right side of portrait)
            ui.vertical(|ui| {
                ui.heading(&character.name);
                ui.label(format!("ID: {}", character.id));
                ui.label(format!("Portrait: {}", character.portrait_id));
                if character.is_premade {
                    ui.label(
                        egui::RichText::new("⭐ Premade Character").color(egui::Color32::GOLD),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("📋 Character Template")
                            .color(egui::Color32::LIGHT_BLUE),
                    );
                }
            });
        });

        ui.add_space(10.0);
        ui.separator();

        egui::Grid::new("character_preview_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label("Race:");
                ui.label(&character.race_id);
                ui.end_row();

                ui.label("Class:");
                ui.label(&character.class_id);
                ui.end_row();

                ui.label("Sex:");
                ui.label(sex_name(character.sex));
                ui.end_row();

                ui.label("Alignment:");
                ui.label(alignment_name(character.alignment));
                ui.end_row();
            });

        ui.add_space(10.0);
        ui.heading("Base Stats");
        ui.separator();

        egui::Grid::new("character_stats_grid")
            .num_columns(4)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label("Might:");
                ui.label(character.base_stats.might.to_string());
                ui.label("Intellect:");
                ui.label(character.base_stats.intellect.to_string());
                ui.end_row();

                ui.label("Personality:");
                ui.label(character.base_stats.personality.to_string());
                ui.label("Endurance:");
                ui.label(character.base_stats.endurance.to_string());
                ui.end_row();

                ui.label("Speed:");
                ui.label(character.base_stats.speed.to_string());
                ui.label("Accuracy:");
                ui.label(character.base_stats.accuracy.to_string());
                ui.end_row();

                ui.label("Luck:");
                ui.label(character.base_stats.luck.to_string());
                ui.end_row();
            });

        ui.add_space(10.0);
        ui.heading("Starting Resources");
        ui.separator();

        egui::Grid::new("character_resources_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label("Gold:");
                ui.label(character.starting_gold.to_string());
                ui.end_row();

                ui.label("Gems:");
                ui.label(character.starting_gems.to_string());
                ui.end_row();

                ui.label("Food:");
                ui.label(character.starting_food.to_string());
                ui.end_row();
            });

        // Show starting equipment
        if !character.starting_equipment.is_empty() {
            ui.add_space(10.0);
            ui.heading("Starting Equipment");
            ui.separator();

            egui::Grid::new("character_equipment_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    if let Some(id) = character.starting_equipment.weapon {
                        ui.label("Weapon:");
                        ui.label(item_name_by_id(items, id));
                        ui.end_row();
                    }
                    if let Some(id) = character.starting_equipment.armor {
                        ui.label("Armor:");
                        ui.label(item_name_by_id(items, id));
                        ui.end_row();
                    }
                    if let Some(id) = character.starting_equipment.shield {
                        ui.label("Shield:");
                        ui.label(item_name_by_id(items, id));
                        ui.end_row();
                    }
                    if let Some(id) = character.starting_equipment.helmet {
                        ui.label("Helmet:");
                        ui.label(item_name_by_id(items, id));
                        ui.end_row();
                    }
                    if let Some(id) = character.starting_equipment.boots {
                        ui.label("Boots:");
                        ui.label(item_name_by_id(items, id));
                        ui.end_row();
                    }
                    if let Some(id) = character.starting_equipment.accessory1 {
                        ui.label("Accessory 1:");
                        ui.label(item_name_by_id(items, id));
                        ui.end_row();
                    }
                    if let Some(id) = character.starting_equipment.accessory2 {
                        ui.label("Accessory 2:");
                        ui.label(item_name_by_id(items, id));
                        ui.end_row();
                    }
                });
        }

        // Show starting items
        if !character.starting_items.is_empty() {
            ui.add_space(10.0);
            ui.heading("Starting Items");
            ui.separator();

            for item_id in &character.starting_items {
                ui.label(format!("• {}", item_name_by_id(items, *item_id)));
            }
        }

        // Show description
        if !character.description.is_empty() {
            ui.add_space(10.0);
            ui.heading("Description");
            ui.separator();
            ui.label(&character.description);
        }
    }

    /// Show character edit/create form
    fn show_character_form(
        &mut self,
        ui: &mut egui::Ui,
        races: &[RaceDefinition],
        classes: &[ClassDefinition],
        items: &[Item],
    ) {
        let title = if self.mode == CharactersEditorMode::Add {
            "New Character"
        } else {
            "Edit Character"
        };

        ui.heading(title);
        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("character_form_scroll")
            .show(ui, |ui| {
                // Basic Info Section
                ui.heading("Basic Information");

                egui::Grid::new("character_basic_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("ID:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.id).hint_text("unique_id"),
                        );
                        ui.end_row();

                        ui.label("Name:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.name)
                                .hint_text("Character Name"),
                        );
                        ui.end_row();

                        ui.label("Race:");
                        if autocomplete_race_selector(
                            ui,
                            "race_select",
                            "",
                            &mut self.buffer.race_id,
                            races,
                        ) {
                            // Selection changed
                        }
                        ui.end_row();

                        ui.label("Class:");
                        if autocomplete_class_selector(
                            ui,
                            "class_select",
                            "",
                            &mut self.buffer.class_id,
                            classes,
                        ) {
                            // Selection changed
                        }
                        ui.end_row();

                        ui.label("Sex:");
                        egui::ComboBox::from_id_salt("sex_select")
                            .selected_text(sex_name(self.buffer.sex))
                            .show_ui(ui, |ui| {
                                for sex in [Sex::Male, Sex::Female, Sex::Other] {
                                    if ui
                                        .selectable_label(self.buffer.sex == sex, sex_name(sex))
                                        .clicked()
                                    {
                                        self.buffer.sex = sex;
                                    }
                                }
                            });
                        ui.end_row();

                        ui.label("Alignment:");
                        egui::ComboBox::from_id_salt("alignment_select")
                            .selected_text(alignment_name(self.buffer.alignment))
                            .show_ui(ui, |ui| {
                                for alignment in
                                    [Alignment::Good, Alignment::Neutral, Alignment::Evil]
                                {
                                    if ui
                                        .selectable_label(
                                            self.buffer.alignment == alignment,
                                            alignment_name(alignment),
                                        )
                                        .clicked()
                                    {
                                        self.buffer.alignment = alignment;
                                    }
                                }
                            });
                        ui.end_row();

                        ui.label("Portrait ID:");
                        ui.horizontal(|ui| {
                            // Autocomplete input
                            autocomplete_portrait_selector(
                                ui,
                                "character_portrait",
                                "",
                                &mut self.buffer.portrait_id,
                                &self.available_portraits,
                            );

                            // Grid picker button
                            if ui.button("🖼").on_hover_text("Browse portraits").clicked() {
                                self.portrait_picker_open = true;
                            }
                        });
                        ui.end_row();

                        ui.label("Premade:");
                        ui.checkbox(&mut self.buffer.is_premade, "");
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.heading("Base Stats");

                egui::Grid::new("character_stats_form_grid")
                    .num_columns(4)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Might:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.might).desired_width(40.0),
                        );
                        ui.label("Intellect:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.intellect)
                                .desired_width(40.0),
                        );
                        ui.end_row();

                        ui.label("Personality:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.personality)
                                .desired_width(40.0),
                        );
                        ui.label("Endurance:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.endurance)
                                .desired_width(40.0),
                        );
                        ui.end_row();

                        ui.label("Speed:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.speed).desired_width(40.0),
                        );
                        ui.label("Accuracy:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.accuracy)
                                .desired_width(40.0),
                        );
                        ui.end_row();

                        ui.label("Luck:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.luck).desired_width(40.0),
                        );
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.heading("Starting Resources");

                egui::Grid::new("character_resources_form_grid")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Gold:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.starting_gold)
                                .desired_width(80.0),
                        );
                        ui.end_row();

                        ui.label("Gems:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.starting_gems)
                                .desired_width(80.0),
                        );
                        ui.end_row();

                        ui.label("Food:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.buffer.starting_food)
                                .desired_width(80.0),
                        );
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.heading("Starting Equipment");

                self.show_equipment_editor(ui, items);

                ui.add_space(10.0);
                ui.heading("Starting Items");

                if autocomplete_item_list_selector(
                    ui,
                    "character_starting_items",
                    "Starting Items",
                    &mut self.buffer.starting_items,
                    items,
                ) {
                    self.has_unsaved_changes = true;
                }

                ui.add_space(10.0);
                ui.heading("Description");

                ui.add(
                    egui::TextEdit::multiline(&mut self.buffer.description)
                        .hint_text("Character backstory/biography...")
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                );

                ui.add_space(20.0);
                ui.separator();

                // Save/Cancel/Back to List buttons
                ui.horizontal(|ui| {
                    if ui.button("Back to List").clicked() {
                        self.cancel_edit();
                    }
                    if ui.button("💾 Save").clicked() {
                        match self.save_character() {
                            Ok(_) => {}
                            Err(e) => {
                                // Show error - in real impl this would be a toast/status
                                ui.label(egui::RichText::new(e).color(egui::Color32::RED));
                            }
                        }
                    }
                    if ui.button("❌ Cancel").clicked() {
                        self.cancel_edit();
                    }
                });
            });
    }

    /// Show equipment slot editor
    fn show_equipment_editor(&mut self, ui: &mut egui::Ui, items: &[Item]) {
        egui::Grid::new("character_equipment_form_grid")
            .num_columns(2)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                // Weapon slot
                if autocomplete_item_selector(
                    ui,
                    "weapon_slot",
                    "Weapon:",
                    &mut self.buffer.weapon_id,
                    items,
                ) {
                    self.has_unsaved_changes = true;
                }
                ui.end_row();

                // Armor slot
                if autocomplete_item_selector(
                    ui,
                    "armor_slot",
                    "Armor:",
                    &mut self.buffer.armor_id,
                    items,
                ) {
                    self.has_unsaved_changes = true;
                }
                ui.end_row();

                // Shield slot
                if autocomplete_item_selector(
                    ui,
                    "shield_slot",
                    "Shield:",
                    &mut self.buffer.shield_id,
                    items,
                ) {
                    self.has_unsaved_changes = true;
                }
                ui.end_row();

                // Helmet slot
                if autocomplete_item_selector(
                    ui,
                    "helmet_slot",
                    "Helmet:",
                    &mut self.buffer.helmet_id,
                    items,
                ) {
                    self.has_unsaved_changes = true;
                }
                ui.end_row();

                // Boots slot
                if autocomplete_item_selector(
                    ui,
                    "boots_slot",
                    "Boots:",
                    &mut self.buffer.boots_id,
                    items,
                ) {
                    self.has_unsaved_changes = true;
                }
                ui.end_row();

                // Accessory 1
                if autocomplete_item_selector(
                    ui,
                    "accessory1_slot",
                    "Accessory 1:",
                    &mut self.buffer.accessory1_id,
                    items,
                ) {
                    self.has_unsaved_changes = true;
                }
                ui.end_row();

                // Accessory 2
                if autocomplete_item_selector(
                    ui,
                    "accessory2_slot",
                    "Accessory 2:",
                    &mut self.buffer.accessory2_id,
                    items,
                ) {
                    self.has_unsaved_changes = true;
                }
                ui.end_row();
            });
    }
}

/// Helper function to show a portrait placeholder when image is missing or failed to load
fn show_portrait_placeholder(ui: &mut egui::Ui, size: egui::Vec2) {
    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());

    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(4),
        egui::Color32::from_rgb(60, 60, 60),
    );

    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(4),
        egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
        egui::StrokeKind::Outside,
    );

    // Draw a simple "no image" icon in the center
    let center = rect.center();
    let icon_size = 32.0;

    // Draw an X or image icon placeholder
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        "🖼",
        egui::FontId::proportional(icon_size),
        egui::Color32::from_rgb(150, 150, 150),
    );
}

/// Helper function to get sex name
fn sex_name(sex: Sex) -> &'static str {
    match sex {
        Sex::Male => "Male",
        Sex::Female => "Female",
        Sex::Other => "Other",
    }
}

/// Helper function to get alignment name
fn alignment_name(alignment: Alignment) -> &'static str {
    match alignment {
        Alignment::Good => "Good",
        Alignment::Neutral => "Neutral",
        Alignment::Evil => "Evil",
    }
}

/// Helper function to get item name by ID
fn item_name_by_id(items: &[Item], item_id: ItemId) -> String {
    items
        .iter()
        .find(|i| i.id == item_id)
        .map(|i| i.name.clone())
        .unwrap_or_else(|| format!("Unknown (ID: {})", item_id))
}

/// Helper function for tests - creates a test item
#[cfg(test)]
fn create_test_item(id: ItemId, name: &str) -> Item {
    use antares::domain::items::types::{ConsumableData, ConsumableEffect, ItemType};

    Item {
        id,
        name: name.to_string(),
        item_type: ItemType::Consumable(ConsumableData {
            effect: ConsumableEffect::HealHp(0),
            is_combat_usable: false,
        }),
        base_cost: 10,
        sell_cost: 5,
        alignment_restriction: None,
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
        icon_path: None,
        tags: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_characters_editor_state_creation() {
        let state = CharactersEditorState::new();
        assert!(state.characters.is_empty());
        assert!(state.selected_character.is_none());
        assert_eq!(state.mode, CharactersEditorMode::List);
    }

    #[test]
    fn test_start_new_character() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        assert_eq!(state.mode, CharactersEditorMode::Add);
        assert!(state.selected_character.is_none());
    }

    #[test]
    fn test_save_character_creates_new() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        state.buffer.id = "test_char".to_string();
        state.buffer.name = "Test Character".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();

        let result = state.save_character();
        assert!(result.is_ok());
        assert_eq!(state.characters.len(), 1);
        assert_eq!(state.characters[0].id, "test_char");
    }

    #[test]
    fn test_save_character_empty_id_error() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        state.buffer.name = "Test".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();

        let result = state.save_character();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "ID cannot be empty");
    }

    #[test]
    fn test_save_character_empty_name_error() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        state.buffer.id = "test".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();

        let result = state.save_character();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Name cannot be empty");
    }

    #[test]
    fn test_save_character_empty_race_error() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        state.buffer.id = "test".to_string();
        state.buffer.name = "Test".to_string();
        state.buffer.class_id = "knight".to_string();

        let result = state.save_character();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Race ID cannot be empty");
    }

    #[test]
    fn test_save_character_empty_class_error() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        state.buffer.id = "test".to_string();
        state.buffer.name = "Test".to_string();
        state.buffer.race_id = "human".to_string();

        let result = state.save_character();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Class ID cannot be empty");
    }

    #[test]
    fn test_save_character_duplicate_id_error() {
        let mut state = CharactersEditorState::new();

        // Create first character
        state.start_new_character();
        state.buffer.id = "duplicate_id".to_string();
        state.buffer.name = "First".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.save_character().unwrap();

        // Try to create second with same ID
        state.start_new_character();
        state.buffer.id = "duplicate_id".to_string();
        state.buffer.name = "Second".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();

        let result = state.save_character();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Character ID already exists");
    }

    #[test]
    fn test_delete_character() {
        let mut state = CharactersEditorState::new();

        // Create a character
        state.start_new_character();
        state.buffer.id = "to_delete".to_string();
        state.buffer.name = "Delete Me".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.save_character().unwrap();

        assert_eq!(state.characters.len(), 1);

        state.delete_character(0);
        assert!(state.characters.is_empty());
    }

    #[test]
    fn test_cancel_edit() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        assert_eq!(state.mode, CharactersEditorMode::Add);

        state.cancel_edit();
        assert_eq!(state.mode, CharactersEditorMode::List);
    }

    #[test]
    fn test_filtered_characters() {
        let mut state = CharactersEditorState::new();

        // Create two characters
        state.start_new_character();
        state.buffer.id = "char1".to_string();
        state.buffer.name = "Human Knight".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.buffer.alignment = Alignment::Good;
        state.buffer.is_premade = true;
        state.save_character().unwrap();

        state.start_new_character();
        state.buffer.id = "char2".to_string();
        state.buffer.name = "Elf Sorcerer".to_string();
        state.buffer.race_id = "elf".to_string();
        state.buffer.class_id = "sorcerer".to_string();
        state.buffer.alignment = Alignment::Neutral;
        state.buffer.is_premade = false;
        state.save_character().unwrap();

        // No filter - should return both
        assert_eq!(state.filtered_characters().len(), 2);

        // Filter by search
        state.search_filter = "Knight".to_string();
        assert_eq!(state.filtered_characters().len(), 1);
        state.search_filter.clear();

        // Filter by race
        state.filter_race = Some("elf".to_string());
        assert_eq!(state.filtered_characters().len(), 1);
        assert_eq!(state.filtered_characters()[0].1.name, "Elf Sorcerer");
        state.filter_race = None;

        // Filter by class
        state.filter_class = Some("knight".to_string());
        assert_eq!(state.filtered_characters().len(), 1);
        state.filter_class = None;

        // Filter by alignment
        state.filter_alignment = Some(Alignment::Good);
        assert_eq!(state.filtered_characters().len(), 1);
        state.filter_alignment = None;

        // Filter by premade
        state.filter_premade_only = true;
        assert_eq!(state.filtered_characters().len(), 1);
        assert_eq!(state.filtered_characters()[0].1.name, "Human Knight");
    }

    #[test]
    fn test_next_available_character_id() {
        let mut state = CharactersEditorState::new();

        assert_eq!(state.next_available_character_id(), "character_1");

        // Add a character
        state.start_new_character();
        state.buffer.id = "character_1".to_string();
        state.buffer.name = "Test".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.save_character().unwrap();

        assert_eq!(state.next_available_character_id(), "character_2");
    }

    #[test]
    fn test_start_edit_character() {
        let mut state = CharactersEditorState::new();

        // Create a character
        state.start_new_character();
        state.buffer.id = "edit_me".to_string();
        state.buffer.name = "Edit Me".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.buffer.alignment = Alignment::Good;
        state.save_character().unwrap();

        // Start editing
        state.start_edit_character(0);
        assert_eq!(state.mode, CharactersEditorMode::Edit);
        assert_eq!(state.selected_character, Some(0));
        assert_eq!(state.buffer.id, "edit_me");
        assert_eq!(state.buffer.name, "Edit Me");
        assert_eq!(state.buffer.alignment, Alignment::Good);
    }

    #[test]
    fn test_edit_character_saves_changes() {
        let mut state = CharactersEditorState::new();

        // Create a character
        state.start_new_character();
        state.buffer.id = "update_me".to_string();
        state.buffer.name = "Original Name".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.save_character().unwrap();

        // Edit it
        state.start_edit_character(0);
        state.buffer.name = "Updated Name".to_string();
        state.save_character().unwrap();

        assert_eq!(state.characters.len(), 1);
        assert_eq!(state.characters[0].name, "Updated Name");
    }

    #[test]
    fn test_character_edit_buffer_default() {
        let buffer = CharacterEditBuffer::default();
        assert!(buffer.id.is_empty());
        assert!(buffer.name.is_empty());
        assert_eq!(buffer.sex, Sex::Male);
        assert_eq!(buffer.alignment, Alignment::Neutral);
        assert_eq!(buffer.might, "10");
        assert!(buffer.is_premade);
    }

    #[test]
    fn test_editor_mode_transitions() {
        let mut state = CharactersEditorState::new();
        assert_eq!(state.mode, CharactersEditorMode::List);

        state.start_new_character();
        assert_eq!(state.mode, CharactersEditorMode::Add);

        state.cancel_edit();
        assert_eq!(state.mode, CharactersEditorMode::List);

        // Create a character to edit
        state.start_new_character();
        state.buffer.id = "transition_test".to_string();
        state.buffer.name = "Test".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.save_character().unwrap();
        assert_eq!(state.mode, CharactersEditorMode::List);

        state.start_edit_character(0);
        assert_eq!(state.mode, CharactersEditorMode::Edit);
    }

    #[test]
    fn test_clear_filters() {
        let mut state = CharactersEditorState::new();
        state.filter_race = Some("human".to_string());
        state.filter_class = Some("knight".to_string());
        state.filter_alignment = Some(Alignment::Good);
        state.filter_premade_only = true;

        state.clear_filters();

        assert!(state.filter_race.is_none());
        assert!(state.filter_class.is_none());
        assert!(state.filter_alignment.is_none());
        assert!(!state.filter_premade_only);
    }

    #[test]
    fn test_sex_name_helper() {
        assert_eq!(sex_name(Sex::Male), "Male");
        assert_eq!(sex_name(Sex::Female), "Female");
    }

    #[test]
    fn test_alignment_name_helper() {
        assert_eq!(alignment_name(Alignment::Good), "Good");
        assert_eq!(alignment_name(Alignment::Neutral), "Neutral");
        assert_eq!(alignment_name(Alignment::Evil), "Evil");
    }

    #[test]
    fn test_save_character_with_starting_items() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        state.buffer.id = "with_items".to_string();
        state.buffer.name = "Character With Items".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.buffer.starting_items = vec![1u8, 2u8, 3u8];

        state.save_character().unwrap();

        assert_eq!(state.characters[0].starting_items, vec![1, 2, 3]);
    }

    #[test]
    fn test_save_character_with_equipment() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        state.buffer.id = "with_equipment".to_string();
        state.buffer.name = "Character With Equipment".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.buffer.weapon_id = 1;
        state.buffer.armor_id = 2;

        state.save_character().unwrap();
        assert!(state.characters.iter().any(|c| c.id == "with_equipment"));
        assert_eq!(state.characters[0].starting_equipment.weapon, Some(1));
        assert_eq!(state.characters[0].starting_equipment.armor, Some(2));
        assert!(state.characters[0].starting_equipment.shield.is_none());
    }

    #[test]
    fn test_autocomplete_equipment_buffer_initialization() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();

        // Verify default equipment IDs are 0 (empty)
        assert_eq!(state.buffer.weapon_id, 0);
        assert_eq!(state.buffer.armor_id, 0);
        assert_eq!(state.buffer.shield_id, 0);
        assert_eq!(state.buffer.helmet_id, 0);
        assert_eq!(state.buffer.boots_id, 0);
        assert_eq!(state.buffer.accessory1_id, 0);
        assert_eq!(state.buffer.accessory2_id, 0);
    }

    #[test]
    fn test_autocomplete_starting_items_initialization() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();

        // Verify starting items is an empty vector
        assert!(state.buffer.starting_items.is_empty());
    }

    #[test]
    fn test_autocomplete_equipment_edit_loads_values() {
        let mut state = CharactersEditorState::new();

        // Create a character with equipment
        let character = CharacterDefinition {
            id: "test_char".to_string(),
            name: "Test Character".to_string(),
            race_id: "human".to_string(),
            class_id: "knight".to_string(),
            sex: Sex::Male,
            alignment: Alignment::Good,
            base_stats: BaseStats::new(10, 10, 10, 10, 10, 10, 10),
            portrait_id: "0".to_string(),
            starting_gold: 100,
            starting_gems: 0,
            starting_food: 10,
            starting_items: vec![50, 51],
            starting_equipment: StartingEquipment {
                weapon: Some(10),
                armor: Some(20),
                shield: Some(30),
                helmet: None,
                boots: None,
                accessory1: None,
                accessory2: None,
            },
            description: String::new(),
            is_premade: true,
        };

        state.characters.push(character);
        state.start_edit_character(0);

        // Verify equipment IDs are loaded into buffer
        assert_eq!(state.buffer.weapon_id, 10);
        assert_eq!(state.buffer.armor_id, 20);
        assert_eq!(state.buffer.shield_id, 30);
        assert_eq!(state.buffer.helmet_id, 0);
        assert_eq!(state.buffer.boots_id, 0);
        assert_eq!(state.buffer.accessory1_id, 0);
        assert_eq!(state.buffer.accessory2_id, 0);

        // Verify starting items are loaded
        assert_eq!(state.buffer.starting_items, vec![50, 51]);
    }

    #[test]
    fn test_autocomplete_equipment_zero_converts_to_none() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        state.buffer.id = "zero_test".to_string();
        state.buffer.name = "Zero Equipment Test".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.buffer.weapon_id = 0;
        state.buffer.armor_id = 5;

        state.save_character().unwrap();

        let saved = &state.characters[0];
        assert_eq!(saved.starting_equipment.weapon, None);
        assert_eq!(saved.starting_equipment.armor, Some(5));
    }

    #[test]
    fn test_autocomplete_starting_items_persistence() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        state.buffer.id = "items_persist".to_string();
        state.buffer.name = "Items Persistence Test".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.buffer.starting_items = vec![1, 2, 3, 4];

        state.save_character().unwrap();

        let saved = &state.characters[0];
        assert_eq!(saved.starting_items, vec![1, 2, 3, 4]);

        // Edit and verify items are loaded back
        state.start_edit_character(0);
        assert_eq!(state.buffer.starting_items, vec![1, 2, 3, 4]);
    }

    /// Verify that starting_items in the Character edit buffer round-trips to
    /// the domain `CharacterDefinition` and persists through RON serialization.
    #[test]
    fn test_character_starting_items_roundtrip() {
        // Arrange: create an editor state and configure starting items
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        state.buffer.id = "items_rt".to_string();
        state.buffer.name = "Items RoundTrip".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();

        // Use item IDs that exist in sample data; values mirror tutorial campaign usage
        state.buffer.starting_items = vec![50, 50];

        // Act: persist via save_character()
        state.save_character().unwrap();

        // Assert: find saved character and verify starting_items
        let saved = state
            .characters
            .iter()
            .find(|c| c.id == "items_rt")
            .expect("Saved character not found")
            .clone();

        assert_eq!(saved.starting_items, vec![50, 50]);

        // Serialization round-trip (RON) preserves Vec<ItemId>
        let ron_str = ron::ser::to_string(&saved).expect("Failed to serialize character to RON");
        let parsed: antares::domain::character_definition::CharacterDefinition =
            ron::from_str(&ron_str).expect("Failed to deserialize character from RON");

        assert_eq!(parsed.starting_items, vec![50, 50]);
    }

    #[test]
    fn test_save_character_invalid_stat() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        state.buffer.id = "invalid_stat".to_string();
        state.buffer.name = "Test".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.buffer.might = "not_a_number".to_string();

        let result = state.save_character();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid Might value");
    }

    #[test]
    fn test_save_character_invalid_gold() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        state.buffer.id = "invalid_gold".to_string();
        state.buffer.name = "Test".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.buffer.starting_gold = "invalid".to_string();

        let result = state.save_character();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid Starting Gold");
    }

    #[test]
    fn test_has_unsaved_changes_flag() {
        let mut state = CharactersEditorState::new();
        assert!(!state.has_unsaved_changes);

        state.start_new_character();
        state.buffer.id = "changes_test".to_string();
        state.buffer.name = "Test".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.save_character().unwrap();

        assert!(state.has_unsaved_changes);
    }

    #[test]
    fn test_portrait_picker_initial_state() {
        let state = CharactersEditorState::new();
        assert!(!state.portrait_picker_open);
        assert!(state.portrait_textures.is_empty());
        assert!(state.available_portraits.is_empty());
        assert!(state.last_campaign_dir.is_none());
    }

    #[test]
    fn test_portrait_picker_open_flag() {
        let mut state = CharactersEditorState::new();
        assert!(!state.portrait_picker_open);

        state.portrait_picker_open = true;
        assert!(state.portrait_picker_open);

        state.portrait_picker_open = false;
        assert!(!state.portrait_picker_open);
    }

    #[test]
    fn test_available_portraits_cache() {
        let mut state = CharactersEditorState::new();
        assert!(state.available_portraits.is_empty());

        // Simulate scanning portraits
        state.available_portraits = vec!["0".to_string(), "1".to_string(), "2".to_string()];
        assert_eq!(state.available_portraits.len(), 3);
        assert_eq!(state.available_portraits[0], "0");
        assert_eq!(state.available_portraits[1], "1");
        assert_eq!(state.available_portraits[2], "2");
    }

    #[test]
    fn test_campaign_dir_change_detection() {
        let mut state = CharactersEditorState::new();
        let dir1 = PathBuf::from("/path/to/campaign1");
        let dir2 = PathBuf::from("/path/to/campaign2");

        // Initial state
        assert!(state.last_campaign_dir.is_none());

        // Set first directory
        state.last_campaign_dir = Some(dir1.clone());
        assert_eq!(state.last_campaign_dir, Some(dir1.clone()));

        // Change to different directory
        state.last_campaign_dir = Some(dir2.clone());
        assert_eq!(state.last_campaign_dir, Some(dir2));
        assert_ne!(state.last_campaign_dir, Some(dir1));
    }

    #[test]
    fn test_portrait_texture_cache_insertion() {
        let mut state = CharactersEditorState::new();
        assert!(state.portrait_textures.is_empty());

        // Simulate failed texture load (None)
        state.portrait_textures.insert("0".to_string(), None);
        assert_eq!(state.portrait_textures.len(), 1);
        assert!(state.portrait_textures.contains_key("0"));
        assert!(state.portrait_textures.get("0").unwrap().is_none());

        // Add another entry
        state.portrait_textures.insert("1".to_string(), None);
        assert_eq!(state.portrait_textures.len(), 2);
    }

    #[test]
    fn test_portrait_id_in_edit_buffer() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();

        // Default should be empty
        assert!(state.buffer.portrait_id.is_empty());

        // Set portrait ID
        state.buffer.portrait_id = "42".to_string();
        assert_eq!(state.buffer.portrait_id, "42");

        // Clear portrait ID
        state.buffer.portrait_id.clear();
        assert!(state.buffer.portrait_id.is_empty());
    }

    #[test]
    fn test_save_character_with_portrait() {
        let mut state = CharactersEditorState::new();
        state.start_new_character();
        state.buffer.id = "portrait_char".to_string();
        state.buffer.name = "Portrait Test".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.buffer.portrait_id = "5".to_string();

        state.save_character().unwrap();

        assert_eq!(state.characters.len(), 1);
        assert_eq!(state.characters[0].portrait_id, "5");
    }

    #[test]
    fn test_edit_character_updates_portrait() {
        let mut state = CharactersEditorState::new();

        // Create character with portrait
        state.start_new_character();
        state.buffer.id = "update_portrait".to_string();
        state.buffer.name = "Test".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.buffer.portrait_id = "1".to_string();
        state.save_character().unwrap();

        // Edit and change portrait
        state.start_edit_character(0);
        state.buffer.portrait_id = "42".to_string();
        state.save_character().unwrap();

        assert_eq!(state.characters[0].portrait_id, "42");
    }

    #[test]
    fn test_portrait_preview_texture_loading() {
        let mut state = CharactersEditorState::new();

        // Create character with portrait
        state.start_new_character();
        state.buffer.id = "preview_test".to_string();
        state.buffer.name = "Preview Test".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.buffer.portrait_id = "0".to_string();
        state.save_character().unwrap();

        // Verify character has portrait_id set
        assert_eq!(state.characters[0].portrait_id, "0");

        // Texture cache should be empty initially
        assert!(state.portrait_textures.is_empty());
    }

    #[test]
    fn test_portrait_preview_with_missing_portrait() {
        let mut state = CharactersEditorState::new();

        // Create character with non-existent portrait
        state.start_new_character();
        state.buffer.id = "missing_portrait".to_string();
        state.buffer.name = "Missing Portrait Test".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.buffer.portrait_id = "999".to_string();
        state.save_character().unwrap();

        // Verify character saved with portrait_id
        assert_eq!(state.characters[0].portrait_id, "999");

        // Cache should handle missing portraits gracefully
        assert!(state.portrait_textures.is_empty());
    }

    #[test]
    fn test_portrait_preview_cache_persistence() {
        let mut state = CharactersEditorState::new();

        // Simulate cached texture (None represents failed load)
        state.portrait_textures.insert("0".to_string(), None);

        // Create character with same portrait
        state.start_new_character();
        state.buffer.id = "cache_test".to_string();
        state.buffer.name = "Cache Test".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.buffer.portrait_id = "0".to_string();
        state.save_character().unwrap();

        // Cache should persist
        assert!(state.portrait_textures.contains_key("0"));
        assert_eq!(state.portrait_textures.len(), 1);
    }

    #[test]
    fn test_preview_shows_character_with_portrait() {
        let mut state = CharactersEditorState::new();

        // Create multiple characters with different portraits
        for i in 0..3 {
            state.start_new_character();
            state.buffer.id = format!("char_{}", i);
            state.buffer.name = format!("Character {}", i);
            state.buffer.race_id = "human".to_string();
            state.buffer.class_id = "knight".to_string();
            state.buffer.portrait_id = i.to_string();
            state.save_character().unwrap();
        }

        // Verify all characters have unique portraits
        assert_eq!(state.characters.len(), 3);
        assert_eq!(state.characters[0].portrait_id, "0");
        assert_eq!(state.characters[1].portrait_id, "1");
        assert_eq!(state.characters[2].portrait_id, "2");
    }

    #[test]
    fn test_portrait_preview_empty_portrait_id() {
        let mut state = CharactersEditorState::new();

        // Create character without portrait
        state.start_new_character();
        state.buffer.id = "no_portrait".to_string();
        state.buffer.name = "No Portrait".to_string();
        state.buffer.race_id = "human".to_string();
        state.buffer.class_id = "knight".to_string();
        state.buffer.portrait_id = String::new();
        state.save_character().unwrap();

        // Verify character has empty portrait_id
        assert_eq!(state.characters[0].portrait_id, "");

        // Should not attempt to load empty portrait
        assert!(state.portrait_textures.is_empty());
    }
}
