// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Classes Editor for Campaign Builder
//!
//! This module provides a visual editor for character classes with full UI
//! rendering via the `show()` method, following the standard editor pattern.
//! Uses shared UI components for consistent layout.

use crate::ui_helpers::{ActionButtons, EditorToolbar, ItemAction, ToolbarAction, TwoColumnLayout};
use antares::domain::classes::{ClassDefinition, SpellSchool, SpellStat};
use antares::domain::items::types::Item;
use antares::domain::types::DiceRoll;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Editor state for classes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassesEditorState {
    /// All classes being edited
    pub classes: Vec<ClassDefinition>,

    /// Currently selected class index
    pub selected_class: Option<usize>,

    /// Editor mode
    pub mode: ClassesEditorMode,

    /// Edit buffer
    pub buffer: ClassEditBuffer,

    /// Search filter
    pub search_filter: String,

    /// Unsaved changes
    pub has_unsaved_changes: bool,
}

/// Editor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassesEditorMode {
    List,
    Creating,
    Editing,
}

/// Buffer for class form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassEditBuffer {
    pub id: String,
    pub name: String,
    pub hp_die_count: String,
    pub hp_die_sides: String,
    pub hp_die_modifier: String,
    pub spell_school: Option<SpellSchool>,
    pub is_pure_caster: bool,
    pub spell_stat: Option<SpellStat>,
    pub disablement_bit_index: String,
    pub special_abilities: String, // Comma-separated
    pub description: String,
    pub starting_weapon_id: String,
    pub starting_armor_id: String,
    pub starting_items: Vec<String>,
}

impl Default for ClassEditBuffer {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            hp_die_count: "1".to_string(),
            hp_die_sides: "10".to_string(),
            hp_die_modifier: "0".to_string(),
            spell_school: None,
            is_pure_caster: false,
            spell_stat: None,
            disablement_bit_index: "0".to_string(),
            special_abilities: String::new(),
            description: String::new(),
            starting_weapon_id: String::new(),
            starting_armor_id: String::new(),
            starting_items: Vec::new(),
        }
    }
}

impl Default for ClassesEditorState {
    fn default() -> Self {
        Self {
            classes: Vec::new(),
            selected_class: None,
            mode: ClassesEditorMode::List,
            buffer: ClassEditBuffer::default(),
            search_filter: String::new(),
            has_unsaved_changes: false,
        }
    }
}

impl ClassesEditorState {
    /// Creates a new classes editor state
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts creating a new class
    pub fn start_new_class(&mut self) {
        self.mode = ClassesEditorMode::Creating;
        self.selected_class = None;
        self.buffer = ClassEditBuffer::default();
    }

    /// Starts editing an existing class
    pub fn start_edit_class(&mut self, idx: usize) {
        if idx < self.classes.len() {
            let class = &self.classes[idx];
            self.selected_class = Some(idx);
            self.mode = ClassesEditorMode::Editing;
            self.buffer = ClassEditBuffer {
                id: class.id.clone(),
                name: class.name.clone(),
                hp_die_count: class.hp_die.count.to_string(),
                hp_die_sides: class.hp_die.sides.to_string(),
                hp_die_modifier: class.hp_die.bonus.to_string(),
                spell_school: class.spell_school,
                is_pure_caster: class.is_pure_caster,
                spell_stat: class.spell_stat,
                disablement_bit_index: class.disablement_bit_index.to_string(),
                special_abilities: class.special_abilities.join(", "),
                description: class.description.clone(),
                starting_weapon_id: class
                    .starting_weapon_id
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
                starting_armor_id: class
                    .starting_armor_id
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
                starting_items: class
                    .starting_items
                    .iter()
                    .map(|id| id.to_string())
                    .collect(),
            };
        }
    }

    /// Saves the current class from the edit buffer
    pub fn save_class(&mut self) -> Result<(), String> {
        let id = self.buffer.id.trim().to_string();
        if id.is_empty() {
            return Err("ID cannot be empty".to_string());
        }

        let name = self.buffer.name.trim().to_string();
        if name.is_empty() {
            return Err("Name cannot be empty".to_string());
        }

        let hp_count = self
            .buffer
            .hp_die_count
            .parse::<u8>()
            .map_err(|_| "Invalid HP Die Count")?;
        let hp_sides = self
            .buffer
            .hp_die_sides
            .parse::<u8>()
            .map_err(|_| "Invalid HP Die Sides")?;
        let hp_mod = self
            .buffer
            .hp_die_modifier
            .parse::<i8>()
            .map_err(|_| "Invalid HP Die Modifier")?;

        let disablement = self
            .buffer
            .disablement_bit_index
            .parse::<u8>()
            .map_err(|_| "Invalid Disablement Bit")?;

        let abilities: Vec<String> = self
            .buffer
            .special_abilities
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let starting_weapon_id = if self.buffer.starting_weapon_id.is_empty() {
            None
        } else {
            self.buffer.starting_weapon_id.parse::<u8>().ok()
        };

        let starting_armor_id = if self.buffer.starting_armor_id.is_empty() {
            None
        } else {
            self.buffer.starting_armor_id.parse::<u8>().ok()
        };

        let starting_items: Vec<u8> = self
            .buffer
            .starting_items
            .iter()
            .filter_map(|id| id.parse::<u8>().ok())
            .collect();

        let class_def = ClassDefinition {
            id: id.clone(),
            name,
            description: self.buffer.description.clone(),
            hp_die: DiceRoll::new(hp_count, hp_sides, hp_mod),
            spell_school: self.buffer.spell_school,
            is_pure_caster: self.buffer.is_pure_caster,
            spell_stat: self.buffer.spell_stat,
            disablement_bit_index: disablement,
            special_abilities: abilities,
            starting_weapon_id,
            starting_armor_id,
            starting_items,
        };

        if let Some(idx) = self.selected_class {
            self.classes[idx] = class_def;
        } else {
            // Check for duplicate ID if creating new
            if self.classes.iter().any(|c| c.id == id) {
                return Err("Class ID already exists".to_string());
            }
            self.classes.push(class_def);
        }

        self.has_unsaved_changes = true;
        self.mode = ClassesEditorMode::List;
        self.selected_class = None;
        Ok(())
    }

    /// Deletes a class at the given index
    pub fn delete_class(&mut self, idx: usize) {
        if idx < self.classes.len() {
            self.classes.remove(idx);
            self.has_unsaved_changes = true;
            if self.selected_class == Some(idx) {
                self.selected_class = None;
                self.mode = ClassesEditorMode::List;
            }
        }
    }

    /// Cancels the current edit operation
    pub fn cancel_edit(&mut self) {
        self.mode = ClassesEditorMode::List;
        self.selected_class = None;
    }

    /// Returns filtered classes based on search filter
    pub fn filtered_classes(&self) -> Vec<(usize, &ClassDefinition)> {
        self.classes
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                self.search_filter.is_empty()
                    || c.name
                        .to_lowercase()
                        .contains(&self.search_filter.to_lowercase())
                    || c.id
                        .to_lowercase()
                        .contains(&self.search_filter.to_lowercase())
            })
            .collect()
    }

    /// Generates the next available class ID
    pub fn next_available_class_id(&self) -> String {
        let max_id = self
            .classes
            .iter()
            .filter_map(|c| c.id.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        (max_id + 1).to_string()
    }

    /// Loads classes from a file path
    pub fn load_from_file(&mut self, path: &std::path::Path) -> Result<(), String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
        let classes: Vec<ClassDefinition> =
            ron::from_str(&content).map_err(|e| format!("Failed to parse classes: {}", e))?;
        self.classes = classes;
        self.has_unsaved_changes = false;
        Ok(())
    }

    /// Saves classes to a file path
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        let content = ron::ser::to_string_pretty(&self.classes, Default::default())
            .map_err(|e| format!("Failed to serialize classes: {}", e))?;
        std::fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(())
    }

    /// Main UI rendering method following standard editor signature
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `items` - Available items for starting equipment selection
    /// * `campaign_dir` - Optional campaign directory path
    /// * `classes_file` - Filename for classes data
    /// * `unsaved_changes` - Mutable flag for tracking unsaved changes
    /// * `status_message` - Mutable string for status messages
    /// * `file_load_merge_mode` - Whether to merge or replace when loading files
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        items: &[Item],
        campaign_dir: Option<&PathBuf>,
        classes_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        file_load_merge_mode: &mut bool,
    ) {
        ui.heading("🛡️ Classes Editor");
        ui.add_space(5.0);

        // Use shared EditorToolbar component
        let toolbar_action = EditorToolbar::new("Classes")
            .with_search(&mut self.search_filter)
            .with_merge_mode(file_load_merge_mode)
            .with_total_count(self.classes.len())
            .with_id_salt("classes_toolbar")
            .show(ui);

        // Handle toolbar actions
        match toolbar_action {
            ToolbarAction::New => {
                self.start_new_class();
                self.buffer.id = self.next_available_class_id();
                *unsaved_changes = true;
            }
            ToolbarAction::Save => {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(classes_file);
                    match self.save_to_file(&path) {
                        Ok(_) => {
                            *status_message = format!("Saved {} classes", self.classes.len());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to save classes: {}", e);
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
                        ron::from_str::<Vec<ClassDefinition>>(&contents)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    });

                    match load_result {
                        Ok(loaded_classes) => {
                            if *file_load_merge_mode {
                                for class in loaded_classes {
                                    if let Some(existing) =
                                        self.classes.iter_mut().find(|c| c.id == class.id)
                                    {
                                        *existing = class;
                                    } else {
                                        self.classes.push(class);
                                    }
                                }
                            } else {
                                self.classes = loaded_classes;
                            }
                            *unsaved_changes = true;
                            *status_message = format!("Loaded classes from: {}", path.display());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to load classes: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Import => {
                // Import not yet implemented for classes
                *status_message = "Import not yet implemented for classes".to_string();
            }
            ToolbarAction::Export => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("classes.ron")
                    .add_filter("RON", &["ron"])
                    .save_file()
                {
                    match ron::ser::to_string_pretty(&self.classes, Default::default()) {
                        Ok(contents) => match std::fs::write(&path, contents) {
                            Ok(_) => {
                                *status_message = format!("Saved classes to: {}", path.display());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to save classes: {}", e);
                            }
                        },
                        Err(e) => {
                            *status_message = format!("Failed to serialize classes: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Reload => {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(classes_file);
                    if path.exists() {
                        match self.load_from_file(&path) {
                            Ok(_) => {
                                *status_message = format!("Loaded {} classes", self.classes.len());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to load classes: {}", e);
                            }
                        }
                    } else {
                        *status_message = "Classes file does not exist".to_string();
                    }
                }
            }
            ToolbarAction::None => {}
        }

        ui.separator();

        // Main content - use TwoColumnLayout for list mode
        match self.mode {
            ClassesEditorMode::List => {
                // Clone data needed for closures to avoid borrow conflicts
                let selected_class_idx = self.selected_class;
                let search_filter = self.search_filter.clone();
                let classes_snapshot: Vec<(usize, ClassDefinition)> = self
                    .classes
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| {
                        search_filter.is_empty()
                            || c.name
                                .to_lowercase()
                                .contains(&search_filter.to_lowercase())
                            || c.id.to_lowercase().contains(&search_filter.to_lowercase())
                    })
                    .map(|(i, c)| (i, c.clone()))
                    .collect();

                // Track actions from UI
                let mut new_selection: Option<usize> = None;
                let mut action_requested: Option<ItemAction> = None;

                TwoColumnLayout::new("classes").show_split(
                    ui,
                    |left_ui| {
                        // Left panel: Classes list
                        left_ui.heading("Classes");
                        left_ui.separator();

                        for (idx, class) in &classes_snapshot {
                            let is_selected = selected_class_idx == Some(*idx);
                            if left_ui.selectable_label(is_selected, &class.name).clicked() {
                                new_selection = Some(*idx);
                            }
                        }
                    },
                    |right_ui| {
                        // Right panel: Detail view
                        if let Some(idx) = selected_class_idx {
                            if let Some((_, class)) =
                                classes_snapshot.iter().find(|(i, _)| *i == idx)
                            {
                                right_ui.heading(&class.name);
                                right_ui.separator();

                                // Action buttons using shared component
                                let action = ActionButtons::new().enabled(true).show(right_ui);
                                if action != ItemAction::None {
                                    action_requested = Some(action);
                                }

                                right_ui.separator();

                                // Class details
                                egui::Grid::new("class_detail_grid")
                                    .num_columns(2)
                                    .spacing([10.0, 5.0])
                                    .show(right_ui, |ui| {
                                        ui.label("ID:");
                                        ui.label(&class.id);
                                        ui.end_row();

                                        ui.label("HP Die:");
                                        ui.label(format!(
                                            "{}d{}{:+}",
                                            class.hp_die.count,
                                            class.hp_die.sides,
                                            class.hp_die.bonus
                                        ));
                                        ui.end_row();

                                        ui.label("Spell School:");
                                        ui.label(
                                            class
                                                .spell_school
                                                .map(|s| format!("{:?}", s))
                                                .unwrap_or_else(|| "None".to_string()),
                                        );
                                        ui.end_row();

                                        ui.label("Pure Caster:");
                                        ui.label(if class.is_pure_caster { "Yes" } else { "No" });
                                        ui.end_row();

                                        ui.label("Description:");
                                        ui.label(&class.description);
                                        ui.end_row();
                                    });
                            } else {
                                right_ui.label("Select a class to view details");
                            }
                        } else {
                            right_ui.label("Select a class to view details");
                        }
                    },
                );

                // Apply selection change after closures
                if let Some(idx) = new_selection {
                    self.selected_class = Some(idx);
                }

                // Handle action button clicks after closures
                if let Some(action) = action_requested {
                    match action {
                        ItemAction::Edit => {
                            if let Some(idx) = self.selected_class {
                                self.start_edit_class(idx);
                            }
                        }
                        ItemAction::Delete => {
                            if let Some(idx) = self.selected_class {
                                self.delete_class(idx);
                                *unsaved_changes = true;
                            }
                        }
                        ItemAction::Duplicate => {
                            if let Some(idx) = self.selected_class {
                                if let Some(class) = self.classes.get(idx).cloned() {
                                    let mut dup = class;
                                    let base_id = dup.id.clone();
                                    let mut suffix = 1;
                                    while self.classes.iter().any(|c| c.id == dup.id) {
                                        dup.id = format!("{}_copy{}", base_id, suffix);
                                        suffix += 1;
                                    }
                                    self.classes.push(dup);
                                    *unsaved_changes = true;
                                    *status_message = "Class duplicated".to_string();
                                }
                            }
                        }
                        ItemAction::Export => {
                            if let Some(idx) = self.selected_class {
                                if let Some(class) = self.classes.get(idx) {
                                    match ron::ser::to_string_pretty(class, Default::default()) {
                                        Ok(contents) => {
                                            ui.ctx().copy_text(contents);
                                            *status_message =
                                                "Copied class to clipboard".to_string();
                                        }
                                        Err(e) => {
                                            *status_message =
                                                format!("Failed to serialize class: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        ItemAction::None => {}
                    }
                }
            }
            ClassesEditorMode::Creating | ClassesEditorMode::Editing => {
                self.show_class_form(ui, items, unsaved_changes);
            }
        }
    }

    /// Shows the class edit form
    fn show_class_form(&mut self, ui: &mut egui::Ui, items: &[Item], unsaved_changes: &mut bool) {
        let is_creating = self.mode == ClassesEditorMode::Creating;
        ui.heading(if is_creating {
            "Create New Class"
        } else {
            "Edit Class"
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.label("Basic Info");
                    ui.horizontal(|ui| {
                        ui.label("ID:");
                        ui.text_edit_singleline(&mut self.buffer.id);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.buffer.name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Description:");
                        ui.text_edit_multiline(&mut self.buffer.description);
                    });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label("Hit Points");
                    ui.horizontal(|ui| {
                        ui.label("Count:");
                        ui.text_edit_singleline(&mut self.buffer.hp_die_count);
                        ui.label("Sides:");
                        ui.text_edit_singleline(&mut self.buffer.hp_die_sides);
                        ui.label("Bonus:");
                        ui.text_edit_singleline(&mut self.buffer.hp_die_modifier);
                    });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label("Magic");
                    ui.checkbox(&mut self.buffer.is_pure_caster, "Pure Caster");

                    ui.horizontal(|ui| {
                        ui.label("Spell School:");
                        egui::ComboBox::from_id_salt("spell_school")
                            .selected_text(format!("{:?}", self.buffer.spell_school))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.buffer.spell_school, None, "None");
                                ui.selectable_value(
                                    &mut self.buffer.spell_school,
                                    Some(SpellSchool::Cleric),
                                    "Cleric",
                                );
                                ui.selectable_value(
                                    &mut self.buffer.spell_school,
                                    Some(SpellSchool::Sorcerer),
                                    "Sorcerer",
                                );
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Spell Stat:");
                        egui::ComboBox::from_id_salt("spell_stat")
                            .selected_text(format!("{:?}", self.buffer.spell_stat))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.buffer.spell_stat, None, "None");
                                ui.selectable_value(
                                    &mut self.buffer.spell_stat,
                                    Some(SpellStat::Intellect),
                                    "Intellect",
                                );
                                ui.selectable_value(
                                    &mut self.buffer.spell_stat,
                                    Some(SpellStat::Personality),
                                    "Personality",
                                );
                            });
                    });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label("Item Restrictions");
                    ui.horizontal(|ui| {
                        ui.label("Disablement Bit:");
                        ui.text_edit_singleline(&mut self.buffer.disablement_bit_index);
                        ui.label("ℹ️").on_hover_text(
                            "This bit flag (0-7) determines item restrictions.\n\
                             Items can be flagged to disable usage by specific classes.\n\
                             Bit 0 = Knight, Bit 1 = Paladin, Bit 2 = Archer, etc.\n\
                             Example: A class with bit 2 cannot use items with disablement flag bit 2 set."
                        );
                    });
                    if let Ok(bit) = self.buffer.disablement_bit_index.parse::<u8>() {
                        if bit <= 7 {
                            ui.label(format!("This class uses restriction bit position: {}", bit));
                        }
                    }
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label("Starting Equipment");

                    // Starting Weapon
                    ui.horizontal(|ui| {
                        ui.label("Starting Weapon:");
                        let current_weapon = if self.buffer.starting_weapon_id.is_empty() {
                            "None".to_string()
                        } else {
                            items
                                .iter()
                                .find(|item| item.id.to_string() == self.buffer.starting_weapon_id)
                                .map(|item| format!("{} (ID: {})", item.name, item.id))
                                .unwrap_or_else(|| {
                                    format!("ID: {}", self.buffer.starting_weapon_id)
                                })
                        };

                        egui::ComboBox::from_id_salt("starting_weapon")
                            .selected_text(current_weapon)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(self.buffer.starting_weapon_id.is_empty(), "None")
                                    .clicked()
                                {
                                    self.buffer.starting_weapon_id = String::new();
                                }
                                for item in items {
                                    if item.is_weapon() {
                                        let is_selected =
                                            item.id.to_string() == self.buffer.starting_weapon_id;
                                        if ui
                                            .selectable_label(
                                                is_selected,
                                                format!("{} (ID: {})", item.name, item.id),
                                            )
                                            .clicked()
                                        {
                                            self.buffer.starting_weapon_id = item.id.to_string();
                                        }
                                    }
                                }
                            });
                    });

                    // Starting Armor
                    ui.horizontal(|ui| {
                        ui.label("Starting Armor:");
                        let current_armor = if self.buffer.starting_armor_id.is_empty() {
                            "None".to_string()
                        } else {
                            items
                                .iter()
                                .find(|item| item.id.to_string() == self.buffer.starting_armor_id)
                                .map(|item| format!("{} (ID: {})", item.name, item.id))
                                .unwrap_or_else(|| format!("ID: {}", self.buffer.starting_armor_id))
                        };

                        egui::ComboBox::from_id_salt("starting_armor")
                            .selected_text(current_armor)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(self.buffer.starting_armor_id.is_empty(), "None")
                                    .clicked()
                                {
                                    self.buffer.starting_armor_id = String::new();
                                }
                                for item in items {
                                    if item.is_armor() {
                                        let is_selected =
                                            item.id.to_string() == self.buffer.starting_armor_id;
                                        if ui
                                            .selectable_label(
                                                is_selected,
                                                format!("{} (ID: {})", item.name, item.id),
                                            )
                                            .clicked()
                                        {
                                            self.buffer.starting_armor_id = item.id.to_string();
                                        }
                                    }
                                }
                            });
                    });

                    // Starting Items List
                    ui.label("Starting Items:");
                    let mut items_to_remove = Vec::new();
                    for (idx, item_id) in self.buffer.starting_items.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let item_name = items
                                .iter()
                                .find(|item| item.id.to_string() == *item_id)
                                .map(|item| item.name.clone())
                                .unwrap_or_else(|| format!("Unknown (ID: {})", item_id));
                            ui.label(item_name);
                            if ui.small_button("🗑️").clicked() {
                                items_to_remove.push(idx);
                            }
                        });
                    }
                    for idx in items_to_remove.into_iter().rev() {
                        self.buffer.starting_items.remove(idx);
                    }

                    if ui.button("➕ Add Starting Item").clicked() {
                        self.buffer.starting_items.push(String::new());
                    }
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label("Special Abilities (comma separated):");
                    ui.text_edit_multiline(&mut self.buffer.special_abilities);
                });

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("⬅ Back to List").clicked() {
                        self.cancel_edit();
                    }

                    if ui.button("✅ Save").clicked() {
                        if let Err(e) = self.save_class() {
                            eprintln!("Error saving class: {}", e);
                        } else {
                            *unsaved_changes = true;
                        }
                    }
                    if ui.button("❌ Cancel").clicked() {
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
    fn test_classes_editor_state_creation() {
        let state = ClassesEditorState::new();
        assert!(state.classes.is_empty());
        assert_eq!(state.mode, ClassesEditorMode::List);
        assert!(state.selected_class.is_none());
    }

    #[test]
    fn test_start_new_class() {
        let mut state = ClassesEditorState::new();
        state.start_new_class();
        assert_eq!(state.mode, ClassesEditorMode::Creating);
        assert!(state.selected_class.is_none());
    }

    #[test]
    fn test_save_class_creates_new() {
        let mut state = ClassesEditorState::new();
        state.start_new_class();
        state.buffer.id = "knight".to_string();
        state.buffer.name = "Knight".to_string();

        let result = state.save_class();
        assert!(result.is_ok());
        assert_eq!(state.classes.len(), 1);
        assert_eq!(state.classes[0].id, "knight");
    }

    #[test]
    fn test_save_class_empty_id_error() {
        let mut state = ClassesEditorState::new();
        state.start_new_class();
        state.buffer.id = "".to_string();
        state.buffer.name = "Knight".to_string();

        let result = state.save_class();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ID cannot be empty"));
    }

    #[test]
    fn test_save_class_empty_name_error() {
        let mut state = ClassesEditorState::new();
        state.start_new_class();
        state.buffer.id = "knight".to_string();
        state.buffer.name = "".to_string();

        let result = state.save_class();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Name cannot be empty"));
    }

    #[test]
    fn test_save_class_duplicate_id_error() {
        let mut state = ClassesEditorState::new();

        // Create first class
        state.start_new_class();
        state.buffer.id = "knight".to_string();
        state.buffer.name = "Knight".to_string();
        state.save_class().unwrap();

        // Try to create duplicate
        state.start_new_class();
        state.buffer.id = "knight".to_string();
        state.buffer.name = "Another Knight".to_string();

        let result = state.save_class();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_delete_class() {
        let mut state = ClassesEditorState::new();
        state.start_new_class();
        state.buffer.id = "knight".to_string();
        state.buffer.name = "Knight".to_string();
        state.save_class().unwrap();

        assert_eq!(state.classes.len(), 1);
        state.delete_class(0);
        assert!(state.classes.is_empty());
    }

    #[test]
    fn test_cancel_edit() {
        let mut state = ClassesEditorState::new();
        state.start_new_class();
        assert_eq!(state.mode, ClassesEditorMode::Creating);

        state.cancel_edit();
        assert_eq!(state.mode, ClassesEditorMode::List);
        assert!(state.selected_class.is_none());
    }

    #[test]
    fn test_filtered_classes() {
        let mut state = ClassesEditorState::new();

        // Add classes
        state.start_new_class();
        state.buffer.id = "knight".to_string();
        state.buffer.name = "Knight".to_string();
        state.save_class().unwrap();

        state.start_new_class();
        state.buffer.id = "paladin".to_string();
        state.buffer.name = "Paladin".to_string();
        state.save_class().unwrap();

        // No filter
        assert_eq!(state.filtered_classes().len(), 2);

        // Filter by name
        state.search_filter = "knight".to_string();
        assert_eq!(state.filtered_classes().len(), 1);
        assert_eq!(state.filtered_classes()[0].1.id, "knight");
    }

    #[test]
    fn test_next_available_class_id() {
        let mut state = ClassesEditorState::new();

        // Empty list
        assert_eq!(state.next_available_class_id(), "1");

        // Add numeric IDs
        state.start_new_class();
        state.buffer.id = "1".to_string();
        state.buffer.name = "Class 1".to_string();
        state.save_class().unwrap();

        state.start_new_class();
        state.buffer.id = "3".to_string();
        state.buffer.name = "Class 3".to_string();
        state.save_class().unwrap();

        assert_eq!(state.next_available_class_id(), "4");
    }

    #[test]
    fn test_start_edit_class() {
        let mut state = ClassesEditorState::new();

        state.start_new_class();
        state.buffer.id = "knight".to_string();
        state.buffer.name = "Knight".to_string();
        state.buffer.description = "A noble warrior".to_string();
        state.save_class().unwrap();

        state.start_edit_class(0);

        assert_eq!(state.mode, ClassesEditorMode::Editing);
        assert_eq!(state.selected_class, Some(0));
        assert_eq!(state.buffer.id, "knight");
        assert_eq!(state.buffer.name, "Knight");
        assert_eq!(state.buffer.description, "A noble warrior");
    }

    #[test]
    fn test_edit_class_saves_changes() {
        let mut state = ClassesEditorState::new();

        // Create class
        state.start_new_class();
        state.buffer.id = "knight".to_string();
        state.buffer.name = "Knight".to_string();
        state.save_class().unwrap();

        // Edit class
        state.start_edit_class(0);
        state.buffer.name = "Noble Knight".to_string();
        state.save_class().unwrap();

        assert_eq!(state.classes.len(), 1);
        assert_eq!(state.classes[0].name, "Noble Knight");
    }

    #[test]
    fn test_class_edit_buffer_default() {
        let buffer = ClassEditBuffer::default();
        assert!(buffer.id.is_empty());
        assert!(buffer.name.is_empty());
        assert_eq!(buffer.hp_die_count, "1");
        assert_eq!(buffer.hp_die_sides, "10");
        assert_eq!(buffer.hp_die_modifier, "0");
        assert!(buffer.spell_school.is_none());
        assert!(!buffer.is_pure_caster);
        assert!(buffer.spell_stat.is_none());
    }

    #[test]
    fn test_editor_mode_transitions() {
        let mut state = ClassesEditorState::new();
        assert_eq!(state.mode, ClassesEditorMode::List);

        state.start_new_class();
        assert_eq!(state.mode, ClassesEditorMode::Creating);

        state.cancel_edit();
        assert_eq!(state.mode, ClassesEditorMode::List);

        state.start_new_class();
        state.buffer.id = "test".to_string();
        state.buffer.name = "Test".to_string();
        state.save_class().unwrap();
        assert_eq!(state.mode, ClassesEditorMode::List);

        state.start_edit_class(0);
        assert_eq!(state.mode, ClassesEditorMode::Editing);
    }
}
