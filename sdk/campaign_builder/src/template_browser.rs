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
//! - Search and filter templates by category/tags/complexity
//! - Preview templates before instantiation
//! - Load templates into the editor or create new creatures
//!
//! # Examples
//!
//! ```no_run
//! use campaign_builder::template_browser::TemplateBrowserState;
//! use campaign_builder::template_metadata::{TemplateRegistry, TemplateEntry};
//! use campaign_builder::creature_templates::initialize_template_registry;
//!
//! let mut state = TemplateBrowserState::new();
//! let registry = initialize_template_registry();
//! let templates: Vec<&TemplateEntry> = registry.all_templates();
//!
//! // In your egui UI context:
//! // let action = state.show(ui, &templates);
//! ```

use crate::template_metadata::{Complexity, TemplateCategory, TemplateEntry};
use eframe::egui;

/// State for the template browser UI
#[derive(Debug, Clone)]
pub struct TemplateBrowserState {
    /// Currently selected template ID
    pub selected_template: Option<String>,

    /// Search query for filtering templates
    pub search_query: String,

    /// Selected category filter (None = all categories)
    pub category_filter: Option<TemplateCategory>,

    /// Selected complexity filter (None = all complexities)
    pub complexity_filter: Option<Complexity>,

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
            category_filter: None,
            complexity_filter: None,
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
    /// * `templates` - List of available template entries from registry
    ///
    /// # Returns
    ///
    /// Returns `Some(TemplateBrowserAction)` if an action should be performed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::template_browser::TemplateBrowserState;
    /// use campaign_builder::creature_templates::initialize_template_registry;
    ///
    /// let mut state = TemplateBrowserState::new();
    /// let registry = initialize_template_registry();
    /// let templates: Vec<_> = registry.all_templates();
    ///
    /// // In egui context:
    /// // let action = state.show(ui, &templates);
    /// ```
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        templates: &[&TemplateEntry],
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
            let category_text = self
                .category_filter
                .map(|c| c.name().to_string())
                .unwrap_or_else(|| "All Categories".to_string());
            egui::ComboBox::from_id_salt("template_category_filter")
                .selected_text(category_text)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.category_filter, None, "All Categories");
                    for category in TemplateCategory::all() {
                        ui.selectable_value(
                            &mut self.category_filter,
                            Some(category),
                            category.name(),
                        );
                    }
                });

            ui.separator();

            // Complexity filter
            ui.label("Complexity:");
            let complexity_text = self
                .complexity_filter
                .map(|c| c.name().to_string())
                .unwrap_or_else(|| "All Levels".to_string());
            egui::ComboBox::from_id_salt("template_complexity_filter")
                .selected_text(complexity_text)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.complexity_filter, None, "All Levels");
                    for complexity in Complexity::all() {
                        ui.selectable_value(
                            &mut self.complexity_filter,
                            Some(complexity),
                            complexity.name(),
                        );
                    }
                });

            ui.separator();

            // Sort order
            ui.label("Sort:");
            egui::ComboBox::from_id_salt("template_sort_order")
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

                    if let Some(template_id) = &self.selected_template {
                        if let Some(entry) = filtered_templates
                            .iter()
                            .find(|(id, _)| id == template_id)
                            .map(|(_, e)| *e)
                        {
                            self.show_template_preview(ui, entry);

                            ui.separator();

                            // Action buttons
                            ui.horizontal(|ui| {
                                if ui
                                    .button("✓ Apply to Current")
                                    .on_hover_text("Replace current creature with this template")
                                    .clicked()
                                {
                                    action = Some(TemplateBrowserAction::ApplyToCurrent(
                                        template_id.clone(),
                                    ));
                                }

                                if ui
                                    .button("➕ Create New")
                                    .on_hover_text("Create new creature from this template")
                                    .clicked()
                                {
                                    action =
                                        Some(TemplateBrowserAction::CreateNew(template_id.clone()));
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
        templates: &'a [&TemplateEntry],
    ) -> Vec<(String, &'a TemplateEntry)> {
        let mut filtered: Vec<_> = templates
            .iter()
            .filter(|entry| {
                let metadata = &entry.metadata;

                // Category filter
                if let Some(category) = self.category_filter {
                    if metadata.category != category {
                        return false;
                    }
                }

                // Complexity filter
                if let Some(complexity) = self.complexity_filter {
                    if metadata.complexity != complexity {
                        return false;
                    }
                }

                // Search query filter
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    let name_match = metadata.name.to_lowercase().contains(&query);
                    let desc_match = metadata.description.to_lowercase().contains(&query);
                    let tags_match = metadata
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
                        .all(|tag| metadata.tags.contains(tag));
                    if !has_all_tags {
                        return false;
                    }
                }

                true
            })
            .map(|entry| (entry.metadata.id.clone(), *entry))
            .collect();

        // Sort
        match self.sort_order {
            SortOrder::NameAscending => {
                filtered.sort_by(|a, b| a.1.metadata.name.cmp(&b.1.metadata.name));
            }
            SortOrder::NameDescending => {
                filtered.sort_by(|a, b| b.1.metadata.name.cmp(&a.1.metadata.name));
            }
            SortOrder::Category => {
                filtered.sort_by_key(|t| format!("{:?}", t.1.metadata.category));
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
        templates: &[(String, &TemplateEntry)],
    ) -> Option<TemplateBrowserAction> {
        let mut action = None;
        let item_size = self.grid_item_size;
        let available_width = ui.available_width();
        let items_per_row = (available_width / (item_size + 10.0)).max(1.0) as usize;

        for row in templates.chunks(items_per_row) {
            ui.horizontal(|ui| {
                for (template_id, entry) in row {
                    let metadata = &entry.metadata;
                    let is_selected = self.selected_template.as_ref() == Some(template_id);

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
                            let icon = match metadata.category {
                                TemplateCategory::Humanoid => "🧍",
                                TemplateCategory::Creature => "🐺",
                                TemplateCategory::Undead => "💀",
                                TemplateCategory::Robot => "🤖",
                                TemplateCategory::Primitive => "📦",
                            };

                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                icon,
                                egui::FontId::proportional(32.0),
                                egui::Color32::WHITE,
                            );

                            // Template name
                            ui.label(&metadata.name);

                            // Complexity indicator
                            let complexity_color = match metadata.complexity {
                                Complexity::Beginner => egui::Color32::GREEN,
                                Complexity::Intermediate => egui::Color32::YELLOW,
                                Complexity::Advanced => egui::Color32::LIGHT_RED,
                                Complexity::Expert => egui::Color32::RED,
                            };
                            ui.colored_label(complexity_color, metadata.complexity.name());

                            response
                        })
                        .inner;

                    if response.clicked() {
                        self.selected_template = Some(template_id.clone());
                    }

                    if response.double_clicked() {
                        action = Some(TemplateBrowserAction::ApplyToCurrent(template_id.clone()));
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
        templates: &[(String, &TemplateEntry)],
    ) -> Option<TemplateBrowserAction> {
        let mut action = None;

        for (template_id, entry) in templates {
            let metadata = &entry.metadata;
            let is_selected = self.selected_template.as_ref() == Some(template_id);

            ui.horizontal(|ui| {
                let response = ui.selectable_label(is_selected, &metadata.name);

                ui.label(format!("[{}]", metadata.category.name()));

                let complexity_color = match metadata.complexity {
                    Complexity::Beginner => egui::Color32::GREEN,
                    Complexity::Intermediate => egui::Color32::YELLOW,
                    Complexity::Advanced => egui::Color32::LIGHT_RED,
                    Complexity::Expert => egui::Color32::RED,
                };
                ui.colored_label(complexity_color, metadata.complexity.name());

                ui.label(format!("{} meshes", metadata.mesh_count));

                if !metadata.tags.is_empty() {
                    ui.label(format!("Tags: {}", metadata.tags.join(", ")));
                }

                if response.clicked() {
                    self.selected_template = Some(template_id.clone());
                }

                if response.double_clicked() {
                    action = Some(TemplateBrowserAction::ApplyToCurrent(template_id.clone()));
                }
            });

            ui.separator();
        }

        action
    }

    /// Shows template preview details
    fn show_template_preview(&self, ui: &mut egui::Ui, entry: &TemplateEntry) {
        let metadata = &entry.metadata;
        let creature = &entry.example_creature;

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(&metadata.name);
            ui.separator();

            ui.label(format!("Category: {}", metadata.category.name()));

            let complexity_color = match metadata.complexity {
                Complexity::Beginner => egui::Color32::GREEN,
                Complexity::Intermediate => egui::Color32::YELLOW,
                Complexity::Advanced => egui::Color32::LIGHT_RED,
                Complexity::Expert => egui::Color32::RED,
            };
            ui.horizontal(|ui| {
                ui.label("Complexity:");
                ui.colored_label(complexity_color, metadata.complexity.name());
            });

            ui.separator();

            ui.label("Description:");
            ui.label(&metadata.description);

            ui.separator();

            if !metadata.tags.is_empty() {
                ui.label("Tags:");
                ui.horizontal_wrapped(|ui| {
                    for tag in &metadata.tags {
                        ui.label(format!("#{}", tag));
                    }
                });
                ui.separator();
            }

            // Creature statistics
            ui.label("Creature Details:");
            ui.label(format!("  Meshes: {}", metadata.mesh_count));
            ui.label(format!("  Scale: {:.2}", creature.scale));

            if let Some(color) = creature.color_tint {
                ui.label(format!(
                    "  Color Tint: RGB({:.2}, {:.2}, {:.2})",
                    color[0], color[1], color[2]
                ));
            }

            if !creature.mesh_transforms.is_empty() {
                ui.label(format!(
                    "  Mesh Transforms: {}",
                    creature.mesh_transforms.len()
                ));
            }
        });
    }
}

/// Actions that can be triggered from the template browser
#[derive(Debug, Clone)]
pub enum TemplateBrowserAction {
    /// Apply template to current creature being edited
    ApplyToCurrent(String),

    /// Create a new creature from the selected template
    CreateNew(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature_templates::initialize_template_registry;
    use crate::template_metadata::{TemplateMetadata, TemplateRegistry};
    use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};

    #[test]
    fn test_template_browser_state_new() {
        let state = TemplateBrowserState::new();
        assert_eq!(state.selected_template, None);
        assert_eq!(state.search_query, "");
        assert_eq!(state.category_filter, None);
        assert_eq!(state.complexity_filter, None);
        assert_eq!(state.view_mode, ViewMode::Grid);
        assert!(state.show_preview);
        assert_eq!(state.grid_item_size, 120.0);
        assert_eq!(state.sort_order, SortOrder::NameAscending);
    }

    #[test]
    fn test_template_category_all() {
        let categories = TemplateCategory::all();
        assert_eq!(categories.len(), 5);
        assert!(categories.contains(&TemplateCategory::Humanoid));
        assert!(categories.contains(&TemplateCategory::Creature));
        assert!(categories.contains(&TemplateCategory::Undead));
        assert!(categories.contains(&TemplateCategory::Robot));
        assert!(categories.contains(&TemplateCategory::Primitive));
    }

    #[test]
    fn test_template_category_names() {
        assert_eq!(TemplateCategory::Humanoid.name(), "Humanoid");
        assert_eq!(TemplateCategory::Creature.name(), "Creature");
        assert_eq!(TemplateCategory::Undead.name(), "Undead");
        assert_eq!(TemplateCategory::Robot.name(), "Robot");
        assert_eq!(TemplateCategory::Primitive.name(), "Primitive");
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

    /// Helper function to create a minimal creature for tests
    fn create_test_creature(name: &str, id: u32) -> CreatureDefinition {
        CreatureDefinition {
            id,
            name: name.to_string(),
            meshes: vec![MeshDefinition {
                name: None,
                vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
                indices: vec![0, 1, 2],
                normals: None,
                uvs: None,
                color: [1.0, 1.0, 1.0, 1.0],
                lod_levels: None,
                lod_distances: None,
                material: None,
                texture_path: None,
            }],
            mesh_transforms: vec![MeshTransform::default()],
            scale: 1.0,
            color_tint: None,
        }
    }

    #[test]
    fn test_view_mode_variants() {
        assert_eq!(ViewMode::Grid, ViewMode::Grid);
        assert_eq!(ViewMode::List, ViewMode::List);
        assert_ne!(ViewMode::Grid, ViewMode::List);
    }

    #[test]
    fn test_filter_by_category() {
        let registry = initialize_template_registry();
        let templates: Vec<_> = registry.all_templates();

        let mut browser = TemplateBrowserState::new();
        browser.category_filter = Some(TemplateCategory::Humanoid);

        let filtered = browser.filter_and_sort_templates(&templates);
        assert_eq!(filtered.len(), 6);
        for (_, entry) in &filtered {
            assert_eq!(entry.metadata.category, TemplateCategory::Humanoid);
        }
    }

    #[test]
    fn test_filter_by_complexity() {
        let registry = initialize_template_registry();
        let templates: Vec<_> = registry.all_templates();

        let mut browser = TemplateBrowserState::new();
        browser.complexity_filter = Some(Complexity::Beginner);

        let filtered = browser.filter_and_sort_templates(&templates);
        assert!(!filtered.is_empty());
        for (_, entry) in &filtered {
            assert_eq!(entry.metadata.complexity, Complexity::Beginner);
        }
    }

    #[test]
    fn test_filter_by_search() {
        let registry = initialize_template_registry();
        let templates: Vec<_> = registry.all_templates();

        let mut browser = TemplateBrowserState::new();
        browser.search_query = "dragon".to_string();

        let filtered = browser.filter_and_sort_templates(&templates);
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0]
            .1
            .metadata
            .name
            .to_lowercase()
            .contains("dragon"));
    }

    #[test]
    fn test_sort_by_name_ascending() {
        let registry = initialize_template_registry();
        let templates: Vec<_> = registry.all_templates();

        let mut browser = TemplateBrowserState::new();
        browser.sort_order = SortOrder::NameAscending;

        let filtered = browser.filter_and_sort_templates(&templates);
        assert!(filtered.len() >= 2);

        // Check that names are sorted
        for i in 0..filtered.len() - 1 {
            assert!(
                filtered[i].1.metadata.name <= filtered[i + 1].1.metadata.name,
                "{} should be <= {}",
                filtered[i].1.metadata.name,
                filtered[i + 1].1.metadata.name
            );
        }
    }

    #[test]
    fn test_sort_by_name_descending() {
        let registry = initialize_template_registry();
        let templates: Vec<_> = registry.all_templates();

        let mut browser = TemplateBrowserState::new();
        browser.sort_order = SortOrder::NameDescending;

        let filtered = browser.filter_and_sort_templates(&templates);
        assert!(filtered.len() >= 2);

        // Check that names are sorted descending
        for i in 0..filtered.len() - 1 {
            assert!(
                filtered[i].1.metadata.name >= filtered[i + 1].1.metadata.name,
                "{} should be >= {}",
                filtered[i].1.metadata.name,
                filtered[i + 1].1.metadata.name
            );
        }
    }

    #[test]
    fn test_template_browser_action_variants() {
        let action1 = TemplateBrowserAction::ApplyToCurrent("template_id_1".to_string());
        let action2 = TemplateBrowserAction::CreateNew("template_id_2".to_string());

        match action1 {
            TemplateBrowserAction::ApplyToCurrent(id) => assert_eq!(id, "template_id_1"),
            _ => panic!("Expected ApplyToCurrent variant"),
        }

        match action2 {
            TemplateBrowserAction::CreateNew(id) => assert_eq!(id, "template_id_2"),
            _ => panic!("Expected CreateNew variant"),
        }
    }

    #[test]
    fn test_filter_with_registry() {
        let registry = initialize_template_registry();
        let templates: Vec<_> = registry.all_templates();

        let browser = TemplateBrowserState::new();
        let filtered = browser.filter_and_sort_templates(&templates);

        // Without filters, all templates should be returned
        assert_eq!(filtered.len(), registry.len());
    }

    #[test]
    fn test_combined_filters() {
        let registry = initialize_template_registry();
        let templates: Vec<_> = registry.all_templates();

        let mut browser = TemplateBrowserState::new();
        browser.category_filter = Some(TemplateCategory::Creature);
        browser.complexity_filter = Some(Complexity::Beginner);

        let filtered = browser.filter_and_sort_templates(&templates);

        // All filtered results should match both criteria
        for (_, entry) in &filtered {
            assert_eq!(entry.metadata.category, TemplateCategory::Creature);
            assert_eq!(entry.metadata.complexity, Complexity::Beginner);
        }
    }

    #[test]
    fn test_search_in_tags() {
        let registry = initialize_template_registry();
        let templates: Vec<_> = registry.all_templates();

        let mut browser = TemplateBrowserState::new();
        browser.search_query = "winged".to_string();

        let filtered = browser.filter_and_sort_templates(&templates);

        // Should find templates with "winged" in their tags
        assert!(!filtered.is_empty());
        for (_, entry) in &filtered {
            let has_tag = entry
                .metadata
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains("winged"));
            let has_in_name = entry.metadata.name.to_lowercase().contains("winged");
            let has_in_desc = entry.metadata.description.to_lowercase().contains("winged");
            assert!(
                has_tag || has_in_name || has_in_desc,
                "Template should have 'winged' in tags, name, or description"
            );
        }
    }

    #[test]
    fn test_complexity_levels() {
        assert_eq!(Complexity::Beginner.name(), "Beginner");
        assert_eq!(Complexity::Intermediate.name(), "Intermediate");
        assert_eq!(Complexity::Advanced.name(), "Advanced");
        assert_eq!(Complexity::Expert.name(), "Expert");
    }
}
