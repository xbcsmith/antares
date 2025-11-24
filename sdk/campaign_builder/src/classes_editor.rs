// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Classes Editor for Campaign Builder
//!
//! This module provides a visual editor for character classes.

use antares::domain::classes::{ClassDefinition, ClassId, SpellSchool, SpellStat};
use antares::domain::types::DiceRoll;
use serde::{Deserialize, Serialize};

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
    pub disablement_bit: String,
    pub special_abilities: String, // Comma-separated
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
            disablement_bit: "0".to_string(),
            special_abilities: String::new(),
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_new_class(&mut self) {
        self.mode = ClassesEditorMode::Creating;
        self.selected_class = None;
        self.buffer = ClassEditBuffer::default();
    }

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
                disablement_bit: class.disablement_bit.to_string(),
                special_abilities: class.special_abilities.join(", "),
            };
        }
    }

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
            .disablement_bit
            .parse::<u8>()
            .map_err(|_| "Invalid Disablement Bit")?;

        let abilities: Vec<String> = self
            .buffer
            .special_abilities
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let class_def = ClassDefinition {
            id: id.clone(),
            name,
            hp_die: DiceRoll::new(hp_count, hp_sides, hp_mod),
            spell_school: self.buffer.spell_school,
            is_pure_caster: self.buffer.is_pure_caster,
            spell_stat: self.buffer.spell_stat,
            disablement_bit: disablement,
            special_abilities: abilities,
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

    pub fn cancel_edit(&mut self) {
        self.mode = ClassesEditorMode::List;
        self.selected_class = None;
    }

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
}
