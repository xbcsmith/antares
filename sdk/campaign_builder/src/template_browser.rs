// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Template browser UI for the campaign builder
//!
//! This module provides UI components for browsing, searching, and previewing
//! creature templates. Templates are pre-configured creatures that users can
//! instantiate and customize.
//!
//! # Architecture
//!
//! The template browser integrates with the creatures editor to:
//! - Display a gallery view of available templates
//! - Search and filter templates by category/tags
//! - Preview templates before instantiation
//! - Load templates into the editor
//!
//! # Examples
//!
//! ```no_run
//! use campaign_builder::template_browser::TemplateBrowserState;
//! use antares::domain::visual::CreatureDefinition;
//!
//! let mut state = TemplateBrowserState::new();
//! let templates = vec![];
//!
//! // In your egui UI context:
//! // state.show(ui, &templates);
//! ```

use antares::domain::visual::CreatureDefinition;
use eframe::egui;
use std::path::PathBuf;

/// State for the template browser UI
#[derive(Debug, Clone)]
pub struct TemplateBrowserState {
    /// Currently selected template index
    pub selected_template: Option<usize>,

    /// Search query for filtering templates
    pub search_query: String,

    /// Selected category filter
    pub category_filter: Option<TemplateCategory>,

    /// Selected tags filter
    pub tags_filter: Vec<String>,

    /// View mode (grid or list)
    pub view_mode: ViewMode,

    /// Show preview panel
    pub show_preview: bool,

    /// Grid item size (for grid view)
    pub grid_item_size: f32,

    /// Sort order
    pub sort_order: SortOrder,
}

/// View mode for template display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Grid view with thumbnails
    Grid,
    /// List view with details
    List,
}

/// Template categories for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateCategory {
    Humanoid,
    Quadruped,
    Dragon,
    Robot,
    Undead,
    Beast,
    Custom,
    All,
}

impl TemplateCategory {
    /// Returns all categories
    pub fn all() -> Vec<Self> {
        vec![
            Self::All,
            Self::Humanoid,
            Self::Quadruped,
            Self::Dragon,
            Self::Robot,
            Self::Undead,
            Self::Beast,
            Self::Custom,
        ]
    }

    /// Returns the display name of the category
    pub fn name(&self) -> &str {
        match self {
            Self::All => "All",
            Self::Humanoid => "Humanoid",
            Self::Quadruped => "Quadruped",
            Self::Dragon => "Dragon",
            Self::Robot => "Robot",
            Self::Undead => "Undead",
            Self::Beast => "Beast",
            Self::Custom => "Custom",
        }
    }
}

/// Sort order for templates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    NameAscending,
    NameDescending,
    DateAdded,
    Category,
}

impl SortOrder {
    /// Returns all sort orders
    pub fn all() -> Vec<Self> {
        vec![
            Self::NameAscending,
            Self::NameDescending,
            Self::DateAdded,
            Self::Category,
        ]
    }

    /// Returns the display name of the sort order
    pub fn name(&self) -> &str {
        match self {
            Self::NameAscending => "Name (A-Z)",
            Self::NameDescending => "Name (Z-A)",
            Self::DateAdded => "Date Added",
            Self::Category => "Category",
        }
    }
}

/// Template metadata for display
#[derive(Debug, Clone)]
pub struct TemplateMetadata {
    /// Template name
    pub name: String,

    /// Template description
    pub description: String,

    /// Template category
    pub category: TemplateCategory,

    /// Tags for filtering
    pub tags: Vec<String>,

    /// Author information
    pub author: Option<String>,

    /// Path to thumbnail image
    pub thumbnail_path: Option<PathBuf>,

    /// Associated creature definition
    pub creature: CreatureDefinition,
}

impl Default for TemplateBrowserState {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateBrowserState {
    /// Creates a new template browser state
    pub fn new() -> Self {
        Self {
            selected_template: None,
            search_query: String::new(),
            category_filter: Some(TemplateCategory::All),
            tags_filter: vec![],
            view_mode: ViewMode::Grid,
            show_preview: true,
            grid_item_size: 120.0,
            sort_order: SortOrder::NameAscending,
        }
    }

    /// Shows the template browser UI
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `templates` - List of available templates
    ///
    /// # Returns
    ///
    /// Returns `Some(TemplateBrowserAction)` if an action should be performed
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        templates: &[TemplateMetadata],
    ) -> Option<TemplateBrowserAction> {
        let mut action = None;

        ui.heading("Template Browser");
        ui.separator();

        // Toolbar
        ui.horizontal(|ui| {
            // View mode selector
            if ui
                .selectable_label(self.view_mode == ViewMode::Grid, "⊞ Grid")
                .clicked()
            {
                self.view_mode = ViewMode::Grid;
            }

            if ui
                .selectable_label(self.view_mode == ViewMode::List, "☰ List")
                .clicked()
            {
                self.view_mode = ViewMode::List;
            }

            ui.separator();

            // Search bar
            ui.label("🔍");
            ui.text_edit_singleline(&mut self.search_query);

            if ui.button("🗙").on_hover_text("Clear search").clicked() {
                self.search_query.clear();
            }

            ui.separator();

            // Category filter
            ui.label("Category:");
            egui::ComboBox::from_id_source("category_filter")
                .selected_text(self.category_filter.unwrap_or(TemplateCategory::All).name())
                .show_ui(ui, |ui| {
                    for category in TemplateCategory::all() {
                        ui.selectable_value(
                            &mut self.category_filter,
                            Some(category),
                            category.name(),
                        );
                    }
                });

            ui.separator();

            // Sort order
            ui.label("Sort:");
            egui::ComboBox::from_id_source("sort_order")
                .selected_text(self.sort_order.name())
                .show_ui(ui, |ui| {
                    for order in SortOrder::all() {
                        ui.selectable_value(&mut self.sort_order, order, order.name());
                    }
                });

            ui.separator();

            ui.checkbox(&mut self.show_preview, "Show Preview");
        });

        ui.separator();

        // Filter and sort templates
        let filtered_templates = self.filter_and_sort_templates(templates);

        // Main content area
        ui.columns(if self.show_preview { 2 } else { 1 }, |columns| {
            // Left column: Template gallery/list
            columns[0].vertical(|ui| {
                ui.heading(format!("Templates ({})", filtered_templates.len()));
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(600.0)
                    .show(ui, |ui| {
                        if filtered_templates.is_empty() {
                            ui.label("No templates found");
                        } else {
                            match self.view_mode {
                                ViewMode::Grid => {
                                    action = self.show_grid_view(ui, &filtered_templates);
                                }
                                ViewMode::List => {
                                    action = self.show_list_view(ui, &filtered_templates);
                                }
                            }
                        }
                    });
            });

            // Right column: Preview (if enabled)
            if self.show_preview && columns.len() > 1 {
                columns[1].vertical(|ui| {
                    ui.heading("Preview");
                    ui.separator();

                    if let Some(idx) = self.selected_template {
                        if let Some(template) = filtered_templates.get(idx) {
                            self.show_template_preview(ui, template);

                            ui.separator();

                            // Action buttons
                            ui.horizontal(|ui| {
                                if ui.button("✓ Use Template").clicked() {
                                    action = Some(TemplateBrowserAction::UseTemplate(
                                        template.creature.clone(),
                                    ));
                                }

                                if ui.button("📋 Duplicate").clicked() {
                                    action = Some(TemplateBrowserAction::DuplicateTemplate(idx));
                                }
                            });
                        }
                    } else {
                        ui.label("Select a template to preview");
                    }
                });
            }
        });

        action
    }

    /// Filters and sorts templates based on current settings
    fn filter_and_sort_templates<'a>(
        &self,
        templates: &'a [TemplateMetadata],
    ) -> Vec<(usize, &'a TemplateMetadata)> {
        let mut filtered: Vec<_> = templates
            .iter()
            .enumerate()
            .filter(|(_, template)| {
                // Category filter
                if let Some(category) = self.category_filter {
                    if category != TemplateCategory::All && template.category != category {
                        return false;
                    }
                }

                // Search query filter
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    let name_match = template.name.to_lowercase().contains(&query);
                    let desc_match = template.description.to_lowercase().contains(&query);
                    let tags_match = template
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query));

                    if !name_match && !desc_match && !tags_match {
                        return false;
                    }
                }

                // Tags filter
                if !self.tags_filter.is_empty() {
                    let has_all_tags = self
                        .tags_filter
                        .iter()
                        .all(|tag| template.tags.contains(tag));
                    if !has_all_tags {
                        return false;
                    }
                }

                true
            })
            .collect();

        // Sort
        match self.sort_order {
            SortOrder::NameAscending => {
                filtered.sort_by(|a, b| a.1.name.cmp(&b.1.name));
            }
            SortOrder::NameDescending => {
                filtered.sort_by(|a, b| b.1.name.cmp(&a.1.name));
            }
            SortOrder::Category => {
                filtered.sort_by_key(|t| t.1.category as u8);
            }
            SortOrder::DateAdded => {
                // Keep original order (assumed to be date added)
            }
        }

        filtered
    }

    /// Shows templates in grid view
    fn show_grid_view(
        &mut self,
        ui: &mut egui::Ui,
        templates: &[(usize, &TemplateMetadata)],
    ) -> Option<TemplateBrowserAction> {
        let mut action = None;
        let item_size = self.grid_item_size;
        let available_width = ui.available_width();
        let items_per_row = (available_width / (item_size + 10.0)).max(1.0) as usize;

        for row in templates.chunks(items_per_row) {
            ui.horizontal(|ui| {
                for (original_idx, template) in row {
                    let is_selected = self.selected_template == Some(*original_idx);

                    let response = ui
                        .vertical(|ui| {
                            // Thumbnail placeholder
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(item_size, item_size),
                                egui::Sense::click(),
                            );

                            // Draw thumbnail background
                            let bg_color = if is_selected {
                                egui::Color32::from_rgb(70, 100, 150)
                            } else {
                                egui::Color32::from_gray(60)
                            };
                            ui.painter().rect_filled(rect, 4.0, bg_color);

                            // Draw placeholder icon or thumbnail
                            let icon = match template.category {
                                TemplateCategory::Humanoid => "🧍",
                                TemplateCategory::Quadruped => "🐎",
                                TemplateCategory::Dragon => "🐉",
                                TemplateCategory::Robot => "🤖",
                                TemplateCategory::Undead => "💀",
                                TemplateCategory::Beast => "🦁",
                                TemplateCategory::Custom => "✨",
                                TemplateCategory::All => "📦",
                            };

                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                icon,
                                egui::FontId::proportional(32.0),
                                egui::Color32::WHITE,
                            );

                            // Template name
                            ui.label(&template.name);

                            response
                        })
                        .inner;

                    if response.clicked() {
                        self.selected_template = Some(*original_idx);
                    }

                    if response.double_clicked() {
                        action = Some(TemplateBrowserAction::UseTemplate(
                            template.creature.clone(),
                        ));
                    }
                }
            });
        }

        action
    }

    /// Shows templates in list view
    fn show_list_view(
        &mut self,
        ui: &mut egui::Ui,
        templates: &[(usize, &TemplateMetadata)],
    ) -> Option<TemplateBrowserAction> {
        let mut action = None;

        for (original_idx, template) in templates {
            let is_selected = self.selected_template == Some(*original_idx);

            ui.horizontal(|ui| {
                let response = ui.selectable_label(is_selected, &template.name);

                ui.label(format!("[{}]", template.category.name()));

                if !template.tags.is_empty() {
                    ui.label(format!("Tags: {}", template.tags.join(", ")));
                }

                if let Some(author) = &template.author {
                    ui.label(format!("by {}", author));
                }

                if response.clicked() {
                    self.selected_template = Some(*original_idx);
                }

                if response.double_clicked() {
                    action = Some(TemplateBrowserAction::UseTemplate(
                        template.creature.clone(),
                    ));
                }
            });

            ui.separator();
        }

        action
    }

    /// Shows template preview details
    fn show_template_preview(&self, ui: &mut egui::Ui, template: &TemplateMetadata) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(format!("Name: {}", template.name));
            ui.label(format!("Category: {}", template.category.name()));

            ui.separator();

            ui.label("Description:");
            ui.label(&template.description);

            ui.separator();

            if !template.tags.is_empty() {
                ui.label("Tags:");
                ui.horizontal_wrapped(|ui| {
                    for tag in &template.tags {
                        ui.label(format!("#{}", tag));
                    }
                });
                ui.separator();
            }

            if let Some(author) = &template.author {
                ui.label(format!("Author: {}", author));
                ui.separator();
            }

            // Creature statistics
            ui.label("Creature Details:");
            ui.label(format!("  Meshes: {}", template.creature.meshes.len()));
            ui.label(format!("  Scale: {:.2}", template.creature.scale));

            if let Some(color) = template.creature.color_tint {
                ui.label(format!(
                    "  Color Tint: RGB({:.2}, {:.2}, {:.2})",
                    color[0], color[1], color[2]
                ));
            }

            if let Some(lod_levels) = &template.creature.lod_levels {
                ui.label(format!("  LOD Levels: {}", lod_levels.len()));
            }

            ui.label(format!(
                "  Animations: {}",
                template.creature.animations.len()
            ));
        });
    }
}

/// Actions that can be performed in the template browser
#[derive(Debug, Clone)]
pub enum TemplateBrowserAction {
    /// Use a template (instantiate it)
    UseTemplate(CreatureDefinition),

    /// Duplicate a template for editing
    DuplicateTemplate(usize),

    /// Delete a custom template
    DeleteTemplate(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_browser_state_new() {
        let state = TemplateBrowserState::new();
        assert_eq!(state.selected_template, None);
        assert_eq!(state.search_query, "");
        assert_eq!(state.category_filter, Some(TemplateCategory::All));
        assert_eq!(state.view_mode, ViewMode::Grid);
        assert!(state.show_preview);
        assert_eq!(state.grid_item_size, 120.0);
        assert_eq!(state.sort_order, SortOrder::NameAscending);
    }

    #[test]
    fn test_template_category_all() {
        let categories = TemplateCategory::all();
        assert_eq!(categories.len(), 8);
        assert!(categories.contains(&TemplateCategory::All));
        assert!(categories.contains(&TemplateCategory::Humanoid));
        assert!(categories.contains(&TemplateCategory::Dragon));
    }

    #[test]
    fn test_template_category_names() {
        assert_eq!(TemplateCategory::Humanoid.name(), "Humanoid");
        assert_eq!(TemplateCategory::Dragon.name(), "Dragon");
        assert_eq!(TemplateCategory::All.name(), "All");
    }

    #[test]
    fn test_sort_order_all() {
        let orders = SortOrder::all();
        assert_eq!(orders.len(), 4);
        assert!(orders.contains(&SortOrder::NameAscending));
        assert!(orders.contains(&SortOrder::Category));
    }

    #[test]
    fn test_sort_order_names() {
        assert_eq!(SortOrder::NameAscending.name(), "Name (A-Z)");
        assert_eq!(SortOrder::NameDescending.name(), "Name (Z-A)");
        assert_eq!(SortOrder::DateAdded.name(), "Date Added");
    }

    #[test]
    fn test_view_mode_variants() {
        assert_eq!(ViewMode::Grid, ViewMode::Grid);
        assert_eq!(ViewMode::List, ViewMode::List);
        assert_ne!(ViewMode::Grid, ViewMode::List);
    }

    #[test]
    fn test_template_metadata_creation() {
        let metadata = TemplateMetadata {
            name: "Test Template".to_string(),
            description: "A test template".to_string(),
            category: TemplateCategory::Humanoid,
            tags: vec!["test".to_string(), "example".to_string()],
            author: Some("Test Author".to_string()),
            thumbnail_path: None,
            creature: CreatureDefinition::default(),
        };

        assert_eq!(metadata.name, "Test Template");
        assert_eq!(metadata.category, TemplateCategory::Humanoid);
        assert_eq!(metadata.tags.len(), 2);
    }

    #[test]
    fn test_filter_by_category() {
        let mut state = TemplateBrowserState::new();
        state.category_filter = Some(TemplateCategory::Dragon);

        let templates = vec![
            TemplateMetadata {
                name: "Dragon1".to_string(),
                description: "".to_string(),
                category: TemplateCategory::Dragon,
                tags: vec![],
                author: None,
                thumbnail_path: None,
                creature: CreatureDefinition::default(),
            },
            TemplateMetadata {
                name: "Humanoid1".to_string(),
                description: "".to_string(),
                category: TemplateCategory::Humanoid,
                tags: vec![],
                author: None,
                thumbnail_path: None,
                creature: CreatureDefinition::default(),
            },
        ];

        let filtered = state.filter_and_sort_templates(&templates);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.name, "Dragon1");
    }

    #[test]
    fn test_filter_by_search() {
        let mut state = TemplateBrowserState::new();
        state.search_query = "dragon".to_string();
        state.category_filter = Some(TemplateCategory::All);

        let templates = vec![
            TemplateMetadata {
                name: "Red Dragon".to_string(),
                description: "".to_string(),
                category: TemplateCategory::Dragon,
                tags: vec![],
                author: None,
                thumbnail_path: None,
                creature: CreatureDefinition::default(),
            },
            TemplateMetadata {
                name: "Knight".to_string(),
                description: "".to_string(),
                category: TemplateCategory::Humanoid,
                tags: vec![],
                author: None,
                thumbnail_path: None,
                creature: CreatureDefinition::default(),
            },
        ];

        let filtered = state.filter_and_sort_templates(&templates);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.name, "Red Dragon");
    }

    #[test]
    fn test_sort_by_name_ascending() {
        let mut state = TemplateBrowserState::new();
        state.sort_order = SortOrder::NameAscending;
        state.category_filter = Some(TemplateCategory::All);

        let templates = vec![
            TemplateMetadata {
                name: "Zombie".to_string(),
                description: "".to_string(),
                category: TemplateCategory::Undead,
                tags: vec![],
                author: None,
                thumbnail_path: None,
                creature: CreatureDefinition::default(),
            },
            TemplateMetadata {
                name: "Archer".to_string(),
                description: "".to_string(),
                category: TemplateCategory::Humanoid,
                tags: vec![],
                author: None,
                thumbnail_path: None,
                creature: CreatureDefinition::default(),
            },
        ];

        let filtered = state.filter_and_sort_templates(&templates);
        assert_eq!(filtered[0].1.name, "Archer");
        assert_eq!(filtered[1].1.name, "Zombie");
    }

    #[test]
    fn test_template_browser_action_variants() {
        let creature = CreatureDefinition::default();

        let action = TemplateBrowserAction::UseTemplate(creature.clone());
        assert!(matches!(action, TemplateBrowserAction::UseTemplate(_)));

        let action = TemplateBrowserAction::DuplicateTemplate(0);
        assert!(matches!(
            action,
            TemplateBrowserAction::DuplicateTemplate(0)
        ));

        let action = TemplateBrowserAction::DeleteTemplate(1);
        assert!(matches!(action, TemplateBrowserAction::DeleteTemplate(1)));
    }
}
