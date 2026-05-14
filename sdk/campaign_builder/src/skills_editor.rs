// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign Builder editor for `skills.ron`.
//!
//! The Skills Editor lets campaign authors create and maintain numeric,
//! level-scaled skill definitions while tracking where those skills are granted
//! by classes and races.

use crate::editor_context::EditorContext;
use crate::ui_helpers::{
    dispatch_list_action, handle_file_load, handle_file_save, handle_reload,
    show_standard_list_item, DispatchActionState, EditorToolbar, ItemAction, MetadataBadge,
    StandardListItemConfig, ToolbarAction, TwoColumnLayout,
};
use antares::domain::classes::ClassDefinition;
use antares::domain::races::RaceDefinition;
use antares::domain::skills::{
    validate_skill_id, SkillCategory, SkillDefinition, SkillId, SkillRank, SkillScalingMode,
};
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;

/// Editor mode for the Campaign Builder Skills Editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillsEditorMode {
    /// Viewing the list of skill definitions.
    List,
    /// Adding a new skill definition.
    Add,
    /// Editing an existing skill definition.
    Edit,
}

/// Category filter for the skills list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillCategoryFilter {
    /// Show all skills.
    All,
    /// Show combat skills.
    Combat,
    /// Show exploration skills.
    Exploration,
    /// Show knowledge skills.
    Knowledge,
    /// Show social skills.
    Social,
    /// Show utility skills.
    Utility,
}

impl SkillCategoryFilter {
    /// Returns all available filter options in UI order.
    pub fn all() -> [Self; 6] {
        [
            Self::All,
            Self::Combat,
            Self::Exploration,
            Self::Knowledge,
            Self::Social,
            Self::Utility,
        ]
    }

    /// Returns `true` when `skill` matches this filter.
    pub fn matches(self, skill: &SkillDefinition) -> bool {
        match self {
            Self::All => true,
            Self::Combat => skill.category == SkillCategory::Combat,
            Self::Exploration => skill.category == SkillCategory::Exploration,
            Self::Knowledge => skill.category == SkillCategory::Knowledge,
            Self::Social => skill.category == SkillCategory::Social,
            Self::Utility => skill.category == SkillCategory::Utility,
        }
    }

    /// Returns the user-visible filter label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Combat => "⚔️ Combat",
            Self::Exploration => "🧭 Exploration",
            Self::Knowledge => "📚 Knowledge",
            Self::Social => "💬 Social",
            Self::Utility => "🧰 Utility",
        }
    }
}

/// Records where a skill is referenced by class/race grants.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillUsage {
    /// Class IDs that grant this skill.
    pub granted_by_classes: Vec<String>,
    /// Race IDs that grant this skill.
    pub granted_by_races: Vec<String>,
}

impl SkillUsage {
    /// Returns `true` when the skill is referenced anywhere.
    pub fn is_used(&self) -> bool {
        !self.granted_by_classes.is_empty() || !self.granted_by_races.is_empty()
    }

    /// Returns the total number of class/race references.
    pub fn total_count(&self) -> usize {
        self.granted_by_classes.len() + self.granted_by_races.len()
    }
}

/// Persistent UI state for the Campaign Builder Skills Editor.
#[derive(Debug, Clone)]
pub struct SkillsEditorState {
    /// Current editor mode.
    pub mode: SkillsEditorMode,
    /// Search query for filtering skills.
    pub search_query: String,
    /// Selected skill index in the authoritative skills vector.
    pub selected_skill: Option<usize>,
    /// Editable skill definition buffer.
    pub edit_buffer: SkillDefinition,
    /// Category filter for the list.
    pub filter_category: SkillCategoryFilter,
    /// Whether the import/export window is open.
    pub show_import_dialog: bool,
    /// RON text buffer for import/export operations.
    pub import_export_buffer: String,
    /// Cached class/race skill usage.
    pub usage_cache: HashMap<SkillId, SkillUsage>,
    /// Pending delete confirmation by skill ID.
    pub confirm_delete_id: Option<SkillId>,
    /// Comma-separated edit buffer for table scaling ranks.
    pub table_ranks_buffer: String,
}

impl Default for SkillsEditorState {
    fn default() -> Self {
        Self {
            mode: SkillsEditorMode::List,
            search_query: String::new(),
            selected_skill: None,
            edit_buffer: Self::default_skill(),
            filter_category: SkillCategoryFilter::All,
            show_import_dialog: false,
            import_export_buffer: String::new(),
            usage_cache: HashMap::new(),
            confirm_delete_id: None,
            table_ranks_buffer: String::new(),
        }
    }
}

impl SkillsEditorState {
    /// Creates a new Skills Editor state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets transient editor state for a new or freshly opened campaign.
    pub fn reset_for_new_campaign(&mut self) {
        self.mode = SkillsEditorMode::List;
        self.search_query.clear();
        self.selected_skill = None;
        self.edit_buffer = Self::default_skill();
        self.filter_category = SkillCategoryFilter::All;
        self.show_import_dialog = false;
        self.import_export_buffer.clear();
        self.usage_cache.clear();
        self.confirm_delete_id = None;
        self.table_ranks_buffer.clear();
    }

    /// Returns a valid default skill definition for Add mode.
    pub fn default_skill() -> SkillDefinition {
        SkillDefinition {
            id: "new_skill".to_string(),
            name: "New Skill".to_string(),
            category: SkillCategory::Utility,
            description: String::new(),
            scaling: SkillScalingMode::Flat,
            max_rank: 10,
            is_trainable: true,
        }
    }

    /// Suggests a lowercase snake_case skill ID from a display name.
    pub fn suggest_skill_id(name: &str, skills: &[SkillDefinition]) -> String {
        let slug = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>()
            .split('_')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("_");

        let base = if slug.is_empty() {
            "new_skill".to_string()
        } else {
            slug
        };
        if !skills.iter().any(|skill| skill.id == base) {
            return base;
        }

        let mut counter = 2;
        loop {
            let candidate = format!("{base}_{counter}");
            if !skills.iter().any(|skill| skill.id == candidate) {
                return candidate;
            }
            counter += 1;
        }
    }

    /// Calculates class/race usage for every provided skill ID.
    pub fn calculate_usage(
        skill_ids: Vec<&str>,
        classes: &[ClassDefinition],
        races: &[RaceDefinition],
    ) -> HashMap<SkillId, SkillUsage> {
        let mut usage: HashMap<SkillId, SkillUsage> = skill_ids
            .into_iter()
            .map(|id| (id.to_string(), SkillUsage::default()))
            .collect();

        for class in classes {
            for grant in &class.skill_grants {
                if let Some(entry) = usage.get_mut(&grant.skill_id) {
                    entry.granted_by_classes.push(class.id.clone());
                }
            }
        }
        for race in races {
            for grant in &race.skill_grants {
                if let Some(entry) = usage.get_mut(&grant.skill_id) {
                    entry.granted_by_races.push(race.id.clone());
                }
            }
        }

        usage
    }

    /// Applies a new scaling mode while preserving useful values when possible.
    pub fn set_scaling_mode(&mut self, mode: SkillScalingMode) {
        self.edit_buffer.scaling = mode;
        self.sync_table_buffer_from_scaling();
    }

    fn sync_table_buffer_from_scaling(&mut self) {
        self.table_ranks_buffer = match &self.edit_buffer.scaling {
            SkillScalingMode::Table { ranks_by_level } => ranks_by_level
                .iter()
                .map(SkillRank::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            _ => String::new(),
        };
    }

    fn parse_table_ranks(text: &str) -> Result<Vec<SkillRank>, String> {
        let mut ranks = Vec::new();
        for token in text.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let rank = token
                .parse::<SkillRank>()
                .map_err(|e| format!("Invalid table rank '{token}': {e}"))?;
            ranks.push(rank);
        }
        if ranks.is_empty() {
            return Err("Table scaling requires at least one rank".to_string());
        }
        Ok(ranks)
    }

    fn validate_buffer(&self, skills: &[SkillDefinition]) -> Result<(), String> {
        validate_skill_id(&self.edit_buffer.id).map_err(|e| e.to_string())?;
        if self.edit_buffer.name.trim().is_empty() {
            return Err("Skill name cannot be empty".to_string());
        }
        if self.edit_buffer.max_rank == 0 {
            return Err("Skill max_rank must be greater than 0".to_string());
        }
        if matches!(self.mode, SkillsEditorMode::Add)
            && skills.iter().any(|skill| skill.id == self.edit_buffer.id)
        {
            return Err(format!("Skill ID '{}' already exists", self.edit_buffer.id));
        }
        self.edit_buffer.validate().map_err(|e| e.to_string())
    }

    /// Renders the Skills Editor UI.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        skills: &mut Vec<SkillDefinition>,
        classes: &[ClassDefinition],
        races: &[RaceDefinition],
        ctx: &mut EditorContext<'_>,
    ) {
        ui.heading("🎯 Skills Editor");
        ui.add_space(5.0);

        let ids = skills.iter().map(|skill| skill.id.as_str()).collect();
        self.usage_cache = Self::calculate_usage(ids, classes, races);

        let toolbar_action = EditorToolbar::new("Skills")
            .with_search(&mut self.search_query)
            .with_merge_mode(ctx.file_load_merge_mode)
            .with_total_count(skills.len())
            .with_id_salt("skills_toolbar")
            .show(ui);

        match toolbar_action {
            ToolbarAction::New => {
                self.mode = SkillsEditorMode::Add;
                self.edit_buffer = Self::default_skill();
                self.edit_buffer.id = Self::suggest_skill_id(&self.edit_buffer.name, skills);
                self.sync_table_buffer_from_scaling();
                *ctx.unsaved_changes = true;
                ui.ctx().request_repaint();
            }
            ToolbarAction::Save => {
                self.save_skills(
                    skills,
                    ctx.campaign_dir,
                    ctx.data_file,
                    ctx.unsaved_changes,
                    ctx.status_message,
                );
            }
            ToolbarAction::Load => {
                handle_file_load(
                    skills,
                    *ctx.file_load_merge_mode,
                    |skill: &SkillDefinition| skill.id.clone(),
                    ctx.status_message,
                    ctx.unsaved_changes,
                );
            }
            ToolbarAction::Import => {
                self.show_import_dialog = true;
                self.import_export_buffer.clear();
                ui.ctx().request_repaint();
            }
            ToolbarAction::Export => {
                handle_file_save(skills, "skills.ron", ctx.status_message);
            }
            ToolbarAction::Reload => {
                handle_reload(skills, ctx.campaign_dir, ctx.data_file, ctx.status_message);
            }
            ToolbarAction::None => {}
        }

        ui.horizontal_wrapped(|ui| {
            ui.label("Category:");
            egui::ComboBox::from_id_salt("skills_category_filter")
                .selected_text(self.filter_category.as_str())
                .show_ui(ui, |ui| {
                    for filter in SkillCategoryFilter::all() {
                        if ui
                            .selectable_value(&mut self.filter_category, filter, filter.as_str())
                            .clicked()
                        {
                            ui.ctx().request_repaint();
                        }
                    }
                });
        });

        ui.separator();

        if self.show_import_dialog {
            self.show_import_dialog_window(
                ui.ctx(),
                skills,
                ctx.unsaved_changes,
                ctx.status_message,
            );
        }

        match self.mode {
            SkillsEditorMode::List => {
                self.show_list(ui, skills, ctx.unsaved_changes, ctx.status_message)
            }
            SkillsEditorMode::Add | SkillsEditorMode::Edit => {
                self.show_form(ui, skills, ctx.unsaved_changes, ctx.status_message)
            }
        }
    }

    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        skills: &mut Vec<SkillDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        let search = self.search_query.to_lowercase();
        let filter = self.filter_category;
        let selected = self.selected_skill;
        let filtered: Vec<(usize, SkillDefinition)> = skills
            .iter()
            .enumerate()
            .filter(|(_, skill)| {
                filter.matches(skill)
                    && (search.is_empty()
                        || skill.id.to_lowercase().contains(&search)
                        || skill.name.to_lowercase().contains(&search))
            })
            .map(|(idx, skill)| (idx, skill.clone()))
            .collect();
        let usage_snapshot = self.usage_cache.clone();

        let mut pending_selection = selected;
        let pending_action = std::cell::RefCell::new(None::<(usize, ItemAction)>);

        TwoColumnLayout::new("skills_editor_layout")
            .with_inspector_min_width(340.0)
            .show_split(
                ui,
                |left_ui| {
                    left_ui.heading("Skills");
                    left_ui.separator();
                    for (idx, skill) in &filtered {
                        left_ui.push_id(skill.id.clone(), |row_ui| {
                            let usage_count = usage_snapshot
                                .get(&skill.id)
                                .map(SkillUsage::total_count)
                                .unwrap_or(0);
                            let mut badges = vec![
                                MetadataBadge::new(category_label(skill.category))
                                    .with_color(category_color(skill.category))
                                    .with_tooltip("Skill category"),
                                MetadataBadge::new(scaling_label(&skill.scaling))
                                    .with_color(egui::Color32::LIGHT_BLUE)
                                    .with_tooltip("Scaling mode"),
                                MetadataBadge::new(format!("Max:{}", skill.max_rank))
                                    .with_tooltip("Maximum rank"),
                            ];
                            if skill.is_trainable {
                                badges.push(
                                    MetadataBadge::new("Trainable")
                                        .with_color(egui::Color32::from_rgb(90, 200, 255)),
                                );
                            }
                            if usage_count > 0 {
                                badges.push(
                                    MetadataBadge::new(format!("Used:{}", usage_count))
                                        .with_color(egui::Color32::YELLOW),
                                );
                            }
                            let config = StandardListItemConfig::new(&skill.name)
                                .with_badges(badges)
                                .with_id(&skill.id)
                                .selected(selected == Some(*idx));
                            let (clicked, action) = show_standard_list_item(row_ui, config);
                            if clicked {
                                pending_selection = Some(*idx);
                                row_ui.ctx().request_repaint();
                            }
                            if action != ItemAction::None {
                                *pending_action.borrow_mut() = Some((*idx, action));
                            }
                        });
                    }
                    if filtered.is_empty() {
                        left_ui.label("No skills found");
                    }
                },
                |right_ui| {
                    if let Some(idx) = selected {
                        if let Some((_, skill)) = filtered.iter().find(|(i, _)| *i == idx) {
                            Self::show_preview_static(
                                right_ui,
                                skill,
                                usage_snapshot.get(&skill.id),
                            );
                            right_ui.separator();
                            right_ui.horizontal_wrapped(|ui| {
                                if ui.button("✏️ Edit").clicked() {
                                    *pending_action.borrow_mut() = Some((idx, ItemAction::Edit));
                                    ui.ctx().request_repaint();
                                }
                                if ui.button("📋 Duplicate").clicked() {
                                    *pending_action.borrow_mut() =
                                        Some((idx, ItemAction::Duplicate));
                                    ui.ctx().request_repaint();
                                }
                                if ui.button("🗑️ Delete").clicked() {
                                    *pending_action.borrow_mut() = Some((idx, ItemAction::Delete));
                                    ui.ctx().request_repaint();
                                }
                            });
                        } else {
                            right_ui.label("Selected skill is hidden by the current filter.");
                        }
                    } else {
                        right_ui.centered_and_justified(|ui| {
                            ui.label("Select a skill to view details.");
                        });
                    }
                },
            );

        self.selected_skill = pending_selection;

        self.show_delete_confirmation(ui, skills, unsaved_changes, status_message);

        if let Some((idx, action)) = pending_action.into_inner() {
            self.selected_skill = Some(idx);
            match action {
                ItemAction::Edit => {
                    if let Some(skill) = skills.get(idx) {
                        self.mode = SkillsEditorMode::Edit;
                        self.edit_buffer = skill.clone();
                        self.sync_table_buffer_from_scaling();
                    }
                }
                ItemAction::Delete => {
                    if let Some(skill) = skills.get(idx) {
                        self.confirm_delete_id = Some(skill.id.clone());
                    }
                }
                ItemAction::Duplicate => {
                    let mut dummy_show = false;
                    let mut dummy_buf = String::new();
                    let mut state = DispatchActionState {
                        entity_label: "skill",
                        import_export_buffer: &mut dummy_buf,
                        show_import_dialog: &mut dummy_show,
                        status_message,
                    };
                    if dispatch_list_action(
                        ItemAction::Duplicate,
                        skills,
                        &mut self.selected_skill,
                        |entry, all| {
                            entry.id = Self::suggest_skill_id(&entry.name, all);
                            entry.name = format!("{} (Copy)", entry.name);
                        },
                        &mut state,
                    ) {
                        *unsaved_changes = true;
                    }
                }
                ItemAction::Export => {
                    if let Some(skill) = skills.get(idx) {
                        match ron::ser::to_string_pretty(skill, Default::default()) {
                            Ok(contents) => {
                                ui.ctx().copy_text(contents);
                                *status_message = "Copied skill to clipboard".to_string();
                            }
                            Err(e) => *status_message = format!("Failed to serialize skill: {e}"),
                        }
                    }
                }
                ItemAction::None => {}
            }
        }
    }

    fn show_delete_confirmation(
        &mut self,
        ui: &mut egui::Ui,
        skills: &mut Vec<SkillDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        let Some(skill_id) = self.confirm_delete_id.clone() else {
            return;
        };
        let mut open = true;
        egui::Window::new("Confirm Skill Deletion")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ui.ctx(), |dialog_ui| {
                if let Some(usage) = self.usage_cache.get(&skill_id) {
                    if usage.is_used() {
                        dialog_ui.colored_label(
                            egui::Color32::YELLOW,
                            format!(
                                "Skill '{skill_id}' is referenced {} time(s).",
                                usage.total_count()
                            ),
                        );
                    }
                }
                dialog_ui.label(format!("Delete skill '{skill_id}'?"));
                dialog_ui.separator();
                dialog_ui.horizontal_wrapped(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.confirm_delete_id = None;
                    }
                    if ui.button("Delete").clicked() {
                        if let Some(idx) = skills.iter().position(|skill| skill.id == skill_id) {
                            skills.remove(idx);
                            self.selected_skill = None;
                            *unsaved_changes = true;
                            *status_message = format!("Deleted skill '{skill_id}'");
                        }
                        self.confirm_delete_id = None;
                    }
                });
            });
        if !open {
            self.confirm_delete_id = None;
        }
    }

    fn show_form(
        &mut self,
        ui: &mut egui::Ui,
        skills: &mut Vec<SkillDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        ui.heading(match self.mode {
            SkillsEditorMode::Add => "Add Skill",
            SkillsEditorMode::Edit => "Edit Skill",
            SkillsEditorMode::List => "Skill",
        });
        ui.separator();

        ui.label("ID:");
        let id_enabled = self.mode == SkillsEditorMode::Add;
        ui.add_enabled(
            id_enabled,
            egui::TextEdit::singleline(&mut self.edit_buffer.id).desired_width(260.0),
        );
        if self.mode == SkillsEditorMode::Add && ui.button("💡 Suggest ID from Name").clicked() {
            self.edit_buffer.id = Self::suggest_skill_id(&self.edit_buffer.name, skills);
        }

        ui.label("Name:");
        ui.text_edit_singleline(&mut self.edit_buffer.name);

        ui.label("Category:");
        egui::ComboBox::from_id_salt("skills_form_category_selector")
            .selected_text(category_label(self.edit_buffer.category))
            .show_ui(ui, |ui| {
                for category in all_categories() {
                    ui.selectable_value(
                        &mut self.edit_buffer.category,
                        category,
                        category_label(category),
                    );
                }
            });

        ui.label("Description:");
        ui.text_edit_multiline(&mut self.edit_buffer.description);

        ui.horizontal(|ui| {
            ui.label("Max Rank:");
            ui.add(egui::DragValue::new(&mut self.edit_buffer.max_rank).range(1..=u16::MAX));
            ui.checkbox(&mut self.edit_buffer.is_trainable, "Trainable");
        });

        self.show_scaling_editor(ui);

        ui.separator();
        if let Err(err) = self.validate_buffer(skills) {
            ui.colored_label(egui::Color32::RED, format!("⚠️ {err}"));
        }

        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if ui.button("⬅ Back to List").clicked() {
                self.mode = SkillsEditorMode::List;
                self.edit_buffer = Self::default_skill();
                ui.ctx().request_repaint();
            }
            let can_save = self.validate_buffer(skills).is_ok();
            if ui
                .add_enabled(can_save, egui::Button::new("💾 Save"))
                .clicked()
                && can_save
            {
                if let Err(err) = self.apply_edit(skills) {
                    *status_message = err;
                } else {
                    *unsaved_changes = true;
                    *status_message = format!("Saved skill '{}'", self.edit_buffer.id);
                    self.mode = SkillsEditorMode::List;
                    self.edit_buffer = Self::default_skill();
                    ui.ctx().request_repaint();
                }
            }
            if ui.button("✕ Cancel").clicked() {
                self.mode = SkillsEditorMode::List;
                self.edit_buffer = Self::default_skill();
                ui.ctx().request_repaint();
            }
        });
    }

    fn show_scaling_editor(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label("Scaling");
            let current = scaling_label(&self.edit_buffer.scaling);
            egui::ComboBox::from_id_salt("skills_form_scaling_mode_selector")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            matches!(self.edit_buffer.scaling, SkillScalingMode::Flat),
                            "Flat",
                        )
                        .clicked()
                    {
                        self.set_scaling_mode(SkillScalingMode::Flat);
                    }
                    if ui
                        .selectable_label(
                            matches!(self.edit_buffer.scaling, SkillScalingMode::Linear { .. }),
                            "Linear",
                        )
                        .clicked()
                    {
                        self.set_scaling_mode(SkillScalingMode::Linear {
                            base: 0,
                            per_level: 1,
                        });
                    }
                    if ui
                        .selectable_label(
                            matches!(self.edit_buffer.scaling, SkillScalingMode::Step { .. }),
                            "Step",
                        )
                        .clicked()
                    {
                        self.set_scaling_mode(SkillScalingMode::Step {
                            base: 0,
                            per_levels: 2,
                            amount: 1,
                        });
                    }
                    if ui
                        .selectable_label(
                            matches!(self.edit_buffer.scaling, SkillScalingMode::Table { .. }),
                            "Table",
                        )
                        .clicked()
                    {
                        self.set_scaling_mode(SkillScalingMode::Table {
                            ranks_by_level: vec![0],
                        });
                    }
                });

            match &mut self.edit_buffer.scaling {
                SkillScalingMode::Flat => {
                    ui.label("Flat skills do not auto-scale.");
                }
                SkillScalingMode::Linear { base, per_level } => {
                    ui.horizontal(|ui| {
                        ui.label("Base:");
                        ui.add(egui::DragValue::new(base).range(0..=u16::MAX));
                        ui.label("Per Level:");
                        ui.add(egui::DragValue::new(per_level).range(0..=u16::MAX));
                    });
                }
                SkillScalingMode::Step {
                    base,
                    per_levels,
                    amount,
                } => {
                    ui.horizontal(|ui| {
                        ui.label("Base:");
                        ui.add(egui::DragValue::new(base).range(0..=u16::MAX));
                        ui.label("Every N Levels:");
                        ui.add(egui::DragValue::new(per_levels).range(1..=u16::MAX));
                        ui.label("Amount:");
                        ui.add(egui::DragValue::new(amount).range(0..=u16::MAX));
                    });
                }
                SkillScalingMode::Table { ranks_by_level } => {
                    ui.label("Ranks by level (comma-separated):");
                    if ui
                        .text_edit_singleline(&mut self.table_ranks_buffer)
                        .changed()
                    {
                        if let Ok(parsed) = Self::parse_table_ranks(&self.table_ranks_buffer) {
                            *ranks_by_level = parsed;
                        }
                    }
                    if let Err(err) = Self::parse_table_ranks(&self.table_ranks_buffer) {
                        ui.colored_label(egui::Color32::YELLOW, err);
                    }
                }
            }
        });
    }

    fn apply_edit(&mut self, skills: &mut Vec<SkillDefinition>) -> Result<(), String> {
        self.validate_buffer(skills)?;
        match self.mode {
            SkillsEditorMode::Add => skills.push(self.edit_buffer.clone()),
            SkillsEditorMode::Edit => {
                let idx = self
                    .selected_skill
                    .ok_or_else(|| "No skill selected".to_string())?;
                if idx >= skills.len() {
                    return Err("Selected skill index is out of range".to_string());
                }
                skills[idx] = self.edit_buffer.clone();
            }
            SkillsEditorMode::List => {}
        }
        Ok(())
    }

    fn show_preview_static(ui: &mut egui::Ui, skill: &SkillDefinition, usage: Option<&SkillUsage>) {
        ui.heading(&skill.name);
        ui.separator();
        egui::Grid::new("skills_preview_grid")
            .num_columns(2)
            .spacing([10.0, 5.0])
            .show(ui, |ui| {
                ui.label("ID:");
                ui.monospace(&skill.id);
                ui.end_row();
                ui.label("Category:");
                ui.colored_label(
                    category_color(skill.category),
                    category_label(skill.category),
                );
                ui.end_row();
                ui.label("Scaling:");
                ui.label(scaling_label(&skill.scaling));
                ui.end_row();
                ui.label("Max Rank:");
                ui.label(skill.max_rank.to_string());
                ui.end_row();
                ui.label("Trainable:");
                ui.label(if skill.is_trainable { "Yes" } else { "No" });
                ui.end_row();
            });
        if !skill.description.is_empty() {
            ui.separator();
            ui.label(&skill.description);
        }
        ui.separator();
        ui.heading("Usage");
        if let Some(usage) = usage {
            if usage.is_used() {
                if !usage.granted_by_classes.is_empty() {
                    ui.label(format!("Classes: {}", usage.granted_by_classes.join(", ")));
                }
                if !usage.granted_by_races.is_empty() {
                    ui.label(format!("Races: {}", usage.granted_by_races.join(", ")));
                }
            } else {
                ui.label("Not granted by any class or race.");
            }
        } else {
            ui.label("No usage data.");
        }
    }

    fn show_import_dialog_window(
        &mut self,
        ctx: &egui::Context,
        skills: &mut Vec<SkillDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        let mut open = self.show_import_dialog;
        egui::Window::new("Import/Export Skills")
            .open(&mut open)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("📋 Copy from Data").clicked() {
                        match ron::ser::to_string_pretty(skills, Default::default()) {
                            Ok(contents) => {
                                self.import_export_buffer = contents;
                                *status_message =
                                    "Copied skills to import/export buffer".to_string();
                            }
                            Err(e) => *status_message = format!("Failed to serialize skills: {e}"),
                        }
                    }
                    if ui.button("📥 Import").clicked() {
                        match ron::from_str::<Vec<SkillDefinition>>(&self.import_export_buffer) {
                            Ok(imported) => {
                                *skills = imported;
                                *unsaved_changes = true;
                                *status_message = "Imported skills".to_string();
                                self.show_import_dialog = false;
                            }
                            Err(e) => *status_message = format!("Failed to parse skills RON: {e}"),
                        }
                    }
                    if ui.button("Close").clicked() {
                        self.show_import_dialog = false;
                    }
                });
                ui.separator();
                ui.text_edit_multiline(&mut self.import_export_buffer);
            });
        self.show_import_dialog = open;
    }

    fn save_skills(
        &self,
        skills: &[SkillDefinition],
        campaign_dir: Option<&PathBuf>,
        skills_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        if let Some(dir) = campaign_dir {
            let path = dir.join(skills_file);
            if let Some(parent) = path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    *status_message = format!("Failed to create skills directory: {e}");
                    return;
                }
            }
            let mut sorted = skills.to_vec();
            sorted.sort_by(|a, b| a.id.cmp(&b.id));
            match ron::ser::to_string_pretty(&sorted, Default::default()) {
                Ok(contents) => match std::fs::write(&path, contents) {
                    Ok(()) => {
                        *status_message = format!("Saved skills to: {}", path.display());
                        *unsaved_changes = false;
                    }
                    Err(e) => *status_message = format!("Failed to write skills: {e}"),
                },
                Err(e) => *status_message = format!("Failed to serialize skills: {e}"),
            }
        }
    }
}

fn all_categories() -> [SkillCategory; 5] {
    [
        SkillCategory::Combat,
        SkillCategory::Exploration,
        SkillCategory::Knowledge,
        SkillCategory::Social,
        SkillCategory::Utility,
    ]
}

fn category_label(category: SkillCategory) -> &'static str {
    match category {
        SkillCategory::Combat => "Combat",
        SkillCategory::Exploration => "Exploration",
        SkillCategory::Knowledge => "Knowledge",
        SkillCategory::Social => "Social",
        SkillCategory::Utility => "Utility",
    }
}

fn category_color(category: SkillCategory) -> egui::Color32 {
    match category {
        SkillCategory::Combat => egui::Color32::from_rgb(220, 90, 80),
        SkillCategory::Exploration => egui::Color32::from_rgb(90, 180, 120),
        SkillCategory::Knowledge => egui::Color32::from_rgb(100, 150, 240),
        SkillCategory::Social => egui::Color32::from_rgb(220, 170, 80),
        SkillCategory::Utility => egui::Color32::from_rgb(170, 170, 170),
    }
}

fn scaling_label(scaling: &SkillScalingMode) -> &'static str {
    match scaling {
        SkillScalingMode::Flat => "Flat",
        SkillScalingMode::Linear { .. } => "Linear",
        SkillScalingMode::Step { .. } => "Step",
        SkillScalingMode::Table { .. } => "Table",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use antares::domain::skills::SkillGrant;

    fn skill(id: &str, category: SkillCategory, scaling: SkillScalingMode) -> SkillDefinition {
        SkillDefinition {
            id: id.to_string(),
            name: id.replace('_', " "),
            category,
            description: String::new(),
            scaling,
            max_rank: 20,
            is_trainable: true,
        }
    }

    #[test]
    fn test_skills_editor_state_default() {
        let state = SkillsEditorState::new();
        assert_eq!(state.mode, SkillsEditorMode::List);
        assert_eq!(state.selected_skill, None);
        assert_eq!(state.filter_category, SkillCategoryFilter::All);
    }

    #[test]
    fn test_skills_editor_default_skill_validates() {
        let skill = SkillsEditorState::default_skill();
        assert!(skill.validate().is_ok());
    }

    #[test]
    fn test_skill_category_filter_matches_expected_categories() {
        let combat = skill("leadership", SkillCategory::Combat, SkillScalingMode::Flat);
        let social = skill("diplomacy", SkillCategory::Social, SkillScalingMode::Flat);
        assert!(SkillCategoryFilter::All.matches(&combat));
        assert!(SkillCategoryFilter::Combat.matches(&combat));
        assert!(!SkillCategoryFilter::Combat.matches(&social));
    }

    #[test]
    fn test_skill_scaling_editor_round_trips_linear() {
        let mut state = SkillsEditorState::new();
        state.set_scaling_mode(SkillScalingMode::Linear {
            base: 2,
            per_level: 3,
        });
        assert!(matches!(
            state.edit_buffer.scaling,
            SkillScalingMode::Linear {
                base: 2,
                per_level: 3
            }
        ));
    }

    #[test]
    fn test_skill_scaling_editor_round_trips_step() {
        let mut state = SkillsEditorState::new();
        state.set_scaling_mode(SkillScalingMode::Step {
            base: 1,
            per_levels: 4,
            amount: 2,
        });
        assert!(matches!(
            state.edit_buffer.scaling,
            SkillScalingMode::Step {
                base: 1,
                per_levels: 4,
                amount: 2
            }
        ));
    }

    #[test]
    fn test_skill_usage_cache_tracks_class_references() {
        let class = ClassDefinition {
            id: "robber".to_string(),
            name: "Robber".to_string(),
            description: String::new(),
            hp_die: antares::domain::types::DiceRoll::new(1, 6, 0),
            spell_school: None,
            is_pure_caster: false,
            spell_stat: None,
            special_abilities: vec![],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec![],
            skill_grants: vec![SkillGrant {
                skill_id: "disarm_traps".to_string(),
                flat_bonus: 2,
                per_level_bonus: 0,
                minimum_rank: None,
                maximum_rank_override: None,
            }],
        };
        let usage = SkillsEditorState::calculate_usage(vec!["disarm_traps"], &[class], &[]);
        assert_eq!(usage["disarm_traps"].granted_by_classes, vec!["robber"]);
    }

    #[test]
    fn test_skill_usage_cache_tracks_race_references() {
        let race = RaceDefinition {
            id: "elf".to_string(),
            name: "Elf".to_string(),
            description: String::new(),
            stat_modifiers: antares::domain::races::StatModifiers::default(),
            resistances: antares::domain::races::Resistances::default(),
            special_abilities: vec![],
            size: antares::domain::races::SizeCategory::Medium,
            proficiencies: vec![],
            incompatible_item_tags: vec![],
            skill_grants: vec![SkillGrant {
                skill_id: "perception".to_string(),
                flat_bonus: 1,
                per_level_bonus: 0,
                minimum_rank: None,
                maximum_rank_override: None,
            }],
        };
        let usage = SkillsEditorState::calculate_usage(vec!["perception"], &[], &[race]);
        assert_eq!(usage["perception"].granted_by_races, vec!["elf"]);
    }

    #[test]
    fn test_skill_validation_rejects_unknown_class_skill_reference() {
        let state = SkillsEditorState::new();
        let class = ClassDefinition {
            id: "knight".to_string(),
            name: "Knight".to_string(),
            description: String::new(),
            hp_die: antares::domain::types::DiceRoll::new(1, 10, 0),
            spell_school: None,
            is_pure_caster: false,
            spell_stat: None,
            special_abilities: vec![],
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: vec![],
            proficiencies: vec![],
            skill_grants: vec![SkillGrant {
                skill_id: "missing".to_string(),
                flat_bonus: 1,
                per_level_bonus: 0,
                minimum_rank: None,
                maximum_rank_override: None,
            }],
        };
        let mut app = crate::CampaignBuilderApp::default();
        app.editor_registry.classes_editor_state.classes = vec![class];
        app.campaign_data.skills = vec![skill(
            "known",
            SkillCategory::Utility,
            SkillScalingMode::Flat,
        )];
        let results = app.validate_skill_ids();
        assert!(results.iter().any(|r| r.message.contains("missing")));
        drop(state);
    }

    #[test]
    fn test_skill_validation_rejects_unknown_race_skill_reference() {
        let race = RaceDefinition {
            id: "dwarf".to_string(),
            name: "Dwarf".to_string(),
            description: String::new(),
            stat_modifiers: antares::domain::races::StatModifiers::default(),
            resistances: antares::domain::races::Resistances::default(),
            special_abilities: vec![],
            size: antares::domain::races::SizeCategory::Medium,
            proficiencies: vec![],
            incompatible_item_tags: vec![],
            skill_grants: vec![SkillGrant {
                skill_id: "missing".to_string(),
                flat_bonus: 1,
                per_level_bonus: 0,
                minimum_rank: None,
                maximum_rank_override: None,
            }],
        };
        let mut app = crate::CampaignBuilderApp::default();
        app.editor_registry.races_editor_state.races = vec![race];
        app.campaign_data.skills = vec![skill(
            "known",
            SkillCategory::Utility,
            SkillScalingMode::Flat,
        )];
        let results = app.validate_skill_ids();
        assert!(results.iter().any(|r| r.message.contains("missing")));
    }
}
