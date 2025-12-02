// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use antares::domain::conditions::{
    ConditionDefinition, ConditionDuration, ConditionEffect, ConditionId,
};
use antares::domain::magic::types::Spell;
use antares::domain::types::DiceRoll;
use eframe::egui;
use std::fs;
use std::path::PathBuf;

/// Transient state used by the Conditions editor UI.
///
/// This state keeps the user's current search filter, the currently selected
/// condition (if any), a temporary edit buffer for creating or editing a
/// condition, and whether the preview pane is visible. In Phase 1 we also add
/// import/export and basic delete/duplicate dialog state.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::campaign_builder::conditions_editor::ConditionsEditorState;
///
/// // In an egui context you would store and reuse the state:
/// // let mut state = ConditionsEditorState::new();
/// ```
/// Filter for condition effect types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectTypeFilter {
    All,
    AttributeModifier,
    StatusEffect,
    DamageOverTime,
    HealOverTime,
}

impl EffectTypeFilter {
    /// Returns all filter variants for iteration
    pub fn all() -> [EffectTypeFilter; 5] {
        [
            EffectTypeFilter::All,
            EffectTypeFilter::AttributeModifier,
            EffectTypeFilter::StatusEffect,
            EffectTypeFilter::DamageOverTime,
            EffectTypeFilter::HealOverTime,
        ]
    }

    /// Returns the display name for this filter
    pub fn as_str(&self) -> &'static str {
        match self {
            EffectTypeFilter::All => "All",
            EffectTypeFilter::AttributeModifier => "Attribute",
            EffectTypeFilter::StatusEffect => "Status",
            EffectTypeFilter::DamageOverTime => "DOT",
            EffectTypeFilter::HealOverTime => "HOT",
        }
    }

    /// Check if a condition matches this filter
    pub fn matches(&self, condition: &ConditionDefinition) -> bool {
        match self {
            EffectTypeFilter::All => true,
            EffectTypeFilter::AttributeModifier => condition
                .effects
                .iter()
                .any(|e| matches!(e, ConditionEffect::AttributeModifier { .. })),
            EffectTypeFilter::StatusEffect => condition
                .effects
                .iter()
                .any(|e| matches!(e, ConditionEffect::StatusEffect(_))),
            EffectTypeFilter::DamageOverTime => condition
                .effects
                .iter()
                .any(|e| matches!(e, ConditionEffect::DamageOverTime { .. })),
            EffectTypeFilter::HealOverTime => condition
                .effects
                .iter()
                .any(|e| matches!(e, ConditionEffect::HealOverTime { .. })),
        }
    }
}

/// Sort order for conditions list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionSortOrder {
    NameAsc,
    NameDesc,
    IdAsc,
    IdDesc,
    EffectCount,
}

impl ConditionSortOrder {
    /// Returns the display name for this sort order
    pub fn as_str(&self) -> &'static str {
        match self {
            ConditionSortOrder::NameAsc => "Name (A-Z)",
            ConditionSortOrder::NameDesc => "Name (Z-A)",
            ConditionSortOrder::IdAsc => "ID (A-Z)",
            ConditionSortOrder::IdDesc => "ID (Z-A)",
            ConditionSortOrder::EffectCount => "Effect Count",
        }
    }
}

pub struct ConditionsEditorState {
    pub search_filter: String,
    pub selected_condition_id: Option<ConditionId>,
    pub edit_buffer: Option<ConditionDefinition>,
    pub show_preview: bool,
    /// Preview magnitude slider value (non-persistent UI-only preview for ActiveCondition.magnitude)
    pub preview_magnitude: f32,
    /// When deleting a condition, optionally remove references from spells (UI toggle)
    pub remove_refs_on_delete: bool,

    // Phase 1 additions
    pub show_import_dialog: bool,
    pub import_export_buffer: String,
    pub duplicate_dialog_open: bool,
    pub delete_confirmation_open: bool,
    pub selected_for_delete: Option<ConditionId>,
    pub editing_original_id: Option<ConditionId>,
    pub effect_edit_buffer: Option<EffectEditBuffer>,

    // QoL Phase additions
    /// Filter conditions by effect type
    pub filter_effect_type: EffectTypeFilter,
    /// Sort order for conditions list
    pub sort_order: ConditionSortOrder,
    /// Show/hide statistics panel
    pub show_statistics: bool,
    /// Navigation request - when set, the parent app should navigate to the specified spell
    pub navigate_to_spell: Option<String>,
}

/// Temporary typed buffer used to edit a single `ConditionEffect` variant in the UI.
///
/// This buffer stores per-variant typed fields while editing an effect inside a
/// `ConditionDefinition`'s `effects` vector (for example: attribute and value
/// for `AttributeModifier`; dice and element for `DamageOverTime`; dice for
/// `HealOverTime`; and a text tag for `StatusEffect`). The `editing_index`
/// field indicates whether the buffer refers to an existing vector entry
/// (`Some(index)`) or is creating a new effect (`None`).
///
/// Notes:
///  - The buffer lives in UI/editor state only — changes made to it are applied
///    to the editor's `edit_buffer` (the `ConditionDefinition` copy) and are
///    persisted to RON only when the parent `ConditionDefinition` is saved.
///  - Dice editing re-uses the `DiceRoll` UI pattern from the Spells editor.
#[derive(Clone)]
pub struct EffectEditBuffer {
    /// The selected effect variant name (eg: "AttributeModifier", "StatusEffect")
    pub effect_type: Option<String>,

    /// The index into the parent `ConditionDefinition::effects` vector if
    /// we're editing an existing effect. `None` while creating a new one.
    pub editing_index: Option<usize>,

    /// AttributeModifier fields
    pub attribute: String,
    pub attribute_value: i16,

    /// StatusEffect text tag
    pub status_tag: String,

    /// Dice value used by DOT / HOT
    pub dice: DiceRoll,

    /// Element used by DOT effects
    pub element: String,
}

impl Default for EffectEditBuffer {
    fn default() -> Self {
        Self {
            effect_type: None,
            editing_index: None,
            attribute: "might".to_string(),
            attribute_value: 0,
            status_tag: String::new(),
            dice: DiceRoll::new(1, 4, 0),
            element: "physical".to_string(),
        }
    }
}

impl Default for ConditionsEditorState {
    fn default() -> Self {
        Self {
            search_filter: String::new(),
            selected_condition_id: None,
            edit_buffer: None,
            show_preview: true,
            preview_magnitude: 1.0,
            remove_refs_on_delete: false,

            // new fields
            show_import_dialog: false,
            import_export_buffer: String::new(),
            duplicate_dialog_open: false,
            delete_confirmation_open: false,
            selected_for_delete: None,
            editing_original_id: None,
            effect_edit_buffer: None,

            // QoL Phase additions
            filter_effect_type: EffectTypeFilter::All,
            sort_order: ConditionSortOrder::NameAsc,
            show_statistics: false,
            navigate_to_spell: None,
        }
    }
}

impl ConditionsEditorState {
    /// Create a new editor state instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a default `ConditionDefinition` used when creating a new condition.
    pub fn default_condition() -> ConditionDefinition {
        ConditionDefinition {
            id: "new_condition".to_string(),
            name: "New Condition".to_string(),
            description: "".to_string(),
            effects: Vec::new(),
            default_duration: ConditionDuration::Rounds(3),
            icon_id: None,
        }
    }
}

impl ConditionsEditorState {
    /// Render the Conditions Editor UI into the provided `egui::Ui`.
    ///
    /// This follows the toolbar, import/load/save patterns used across other
    /// editors (items, monsters, spells).
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        conditions: &mut Vec<ConditionDefinition>,
        spells: &mut Vec<Spell>,
        campaign_dir: Option<&PathBuf>,
        conditions_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        file_load_merge_mode: &mut bool,
    ) {
        let panel_height = crate::ui_helpers::compute_panel_height(
            ui,
            crate::ui_helpers::DEFAULT_PANEL_MIN_HEIGHT,
        );

        ui.heading("⚕️ Conditions Editor");
        ui.add_space(5.0);

        // Top toolbar
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            if ui.text_edit_singleline(&mut self.search_filter).changed() {
                self.selected_condition_id = None;
            }

            // Effect type filter
            ui.separator();
            ui.label("Filter:")
                .on_hover_text("Filter conditions by effect type");
            egui::ComboBox::from_id_salt("condition_effect_filter")
                .selected_text(self.filter_effect_type.as_str())
                .show_ui(ui, |ui| {
                    for filter in EffectTypeFilter::all() {
                        if ui
                            .selectable_label(self.filter_effect_type == filter, filter.as_str())
                            .clicked()
                        {
                            self.filter_effect_type = filter;
                            self.selected_condition_id = None;
                        }
                    }
                });

            // Sort order
            ui.label("Sort:")
                .on_hover_text("Sort order for conditions list");
            egui::ComboBox::from_id_salt("condition_sort_order")
                .selected_text(self.sort_order.as_str())
                .show_ui(ui, |ui| {
                    for order in [
                        ConditionSortOrder::NameAsc,
                        ConditionSortOrder::NameDesc,
                        ConditionSortOrder::IdAsc,
                        ConditionSortOrder::IdDesc,
                        ConditionSortOrder::EffectCount,
                    ] {
                        if ui
                            .selectable_label(self.sort_order == order, order.as_str())
                            .clicked()
                        {
                            self.sort_order = order;
                        }
                    }
                });

            ui.separator();

            if ui.button("➕ New Condition").clicked() {
                self.edit_buffer = Some(Self::default_condition());
                self.selected_condition_id = None;
                self.editing_original_id = None;
            }

            if ui.button("🔄 Reload").clicked() {
                if let Some(dir) = campaign_dir {
                    let path = dir.join(conditions_file);
                    if path.exists() {
                        match fs::read_to_string(&path) {
                            Ok(contents) => {
                                match ron::from_str::<Vec<ConditionDefinition>>(&contents) {
                                    Ok(loaded) => {
                                        *conditions = loaded;
                                        *status_message =
                                            format!("Loaded conditions from: {}", path.display());
                                    }
                                    Err(e) => {
                                        *status_message =
                                            format!("Failed to parse conditions: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                *status_message = format!("Failed to read conditions: {}", e);
                            }
                        }
                    } else {
                        *status_message =
                            format!("Conditions file does not exist: {}", path.display());
                    }
                } else {
                    *status_message = "No campaign directory set".to_string();
                }
            }

            if ui.button("📥 Import").clicked() {
                self.show_import_dialog = true;
                self.import_export_buffer.clear();
            }

            ui.separator();

            if ui.button("📂 Load from File").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("RON", &["ron"])
                    .pick_file()
                {
                    let load_result = fs::read_to_string(&path).and_then(|contents| {
                        ron::from_str::<Vec<ConditionDefinition>>(&contents)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    });

                    match load_result {
                        Ok(loaded_conditions) => {
                            if *file_load_merge_mode {
                                for cond in loaded_conditions {
                                    if let Some(existing) =
                                        conditions.iter_mut().find(|c| c.id == cond.id)
                                    {
                                        *existing = cond;
                                    } else {
                                        conditions.push(cond);
                                    }
                                }
                            } else {
                                *conditions = loaded_conditions;
                            }
                            *unsaved_changes = true;
                            *status_message = format!("Loaded conditions from: {}", path.display());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to load conditions: {}", e);
                        }
                    }
                }
            }

            ui.checkbox(file_load_merge_mode, "Merge");
            ui.label(if *file_load_merge_mode {
                "(adds to existing)"
            } else {
                "(replaces all)"
            });

            if ui.button("💾 Save to File").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("conditions.ron")
                    .add_filter("RON", &["ron"])
                    .save_file()
                {
                    match ron::ser::to_string_pretty(conditions, Default::default()) {
                        Ok(contents) => match fs::write(&path, contents) {
                            Ok(_) => {
                                *status_message =
                                    format!("Saved conditions to: {}", path.display());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to save conditions: {}", e);
                            }
                        },
                        Err(e) => {
                            *status_message = format!("Failed to serialize conditions: {}", e);
                        }
                    }
                }
            }

            if ui.button("📋 Export Selected").clicked() {
                if let Some(selected_id) = &self.selected_condition_id {
                    if let Some(cond) = conditions.iter().find(|c| &c.id == selected_id) {
                        match ron::ser::to_string_pretty(cond, Default::default()) {
                            Ok(contents) => {
                                ui.ctx().copy_text(contents);
                                *status_message =
                                    "Copied selected condition to clipboard".to_string();
                            }
                            Err(e) => {
                                *status_message = format!("Failed to serialize condition: {}", e);
                            }
                        }
                    } else {
                        *status_message = "Selected condition not found".to_string();
                    }
                } else {
                    // Copy all
                    match ron::ser::to_string_pretty(conditions, Default::default()) {
                        Ok(contents) => {
                            ui.ctx().copy_text(contents);
                            *status_message = "Copied all conditions to clipboard".to_string();
                        }
                        Err(e) => {
                            *status_message = format!("Failed to serialize conditions: {}", e);
                        }
                    }
                }
            }

            ui.separator();

            // Statistics summary
            let stats = compute_condition_statistics(conditions);
            ui.label(format!("Total: {}", stats.total))
                .on_hover_text(format!(
                    "Attribute: {}, Status: {}, DOT: {}, HOT: {}, Empty: {}",
                    stats.attribute_count,
                    stats.status_count,
                    stats.dot_count,
                    stats.hot_count,
                    stats.empty_count
                ));

            ui.checkbox(&mut self.show_preview, "Preview")
                .on_hover_text("Show effect preview with magnitude scaling");
            ui.checkbox(&mut self.show_statistics, "Stats")
                .on_hover_text("Show detailed statistics panel");
        });

        // Statistics panel (collapsible)
        if self.show_statistics {
            let stats = compute_condition_statistics(conditions);
            ui.horizontal(|ui| {
                ui.label("📊 Statistics:");
                ui.label(format!("Attribute: {}", stats.attribute_count));
                ui.label(format!("Status: {}", stats.status_count));
                ui.label(format!("DOT: {}", stats.dot_count));
                ui.label(format!("HOT: {}", stats.hot_count));
                ui.label(format!("Empty: {}", stats.empty_count));
                ui.label(format!("Multi-effect: {}", stats.multi_effect_count));
            });
            ui.separator();
        }

        // Filter / list + editor layout (copied from prior render function, extended with duplicate/delete)
        ui.horizontal(|ui| {
            // Left panel: List
            ui.vertical(|ui| {
                ui.set_width(crate::ui_helpers::DEFAULT_LEFT_COLUMN_WIDTH);
                ui.set_min_height(panel_height);
                ui.heading("Conditions");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.search_filter);
                    if ui.button("❌").clicked() {
                        self.search_filter.clear();
                    }
                });

                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("conditions_list_scroll")
                    .auto_shrink([false, false])
                    .max_height(panel_height)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        let mut filtered: Vec<(usize, &ConditionDefinition)> = conditions
                            .iter()
                            .enumerate()
                            .filter(|(_, c)| {
                                // Text filter
                                let text_match = self.search_filter.is_empty()
                                    || c.name
                                        .to_lowercase()
                                        .contains(&self.search_filter.to_lowercase())
                                    || c.id
                                        .to_lowercase()
                                        .contains(&self.search_filter.to_lowercase());
                                // Effect type filter
                                let type_match = self.filter_effect_type.matches(c);
                                text_match && type_match
                            })
                            .collect();

                        // Apply sorting
                        match self.sort_order {
                            ConditionSortOrder::NameAsc => {
                                filtered.sort_by(|a, b| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()));
                            }
                            ConditionSortOrder::NameDesc => {
                                filtered.sort_by(|a, b| b.1.name.to_lowercase().cmp(&a.1.name.to_lowercase()));
                            }
                            ConditionSortOrder::IdAsc => {
                                filtered.sort_by(|a, b| a.1.id.to_lowercase().cmp(&b.1.id.to_lowercase()));
                            }
                            ConditionSortOrder::IdDesc => {
                                filtered.sort_by(|a, b| b.1.id.to_lowercase().cmp(&a.1.id.to_lowercase()));
                            }
                            ConditionSortOrder::EffectCount => {
                                filtered.sort_by(|a, b| b.1.effects.len().cmp(&a.1.effects.len()));
                            }
                        }

                        for (_idx, condition) in filtered {
                            let is_selected =
                                self.selected_condition_id.as_ref() == Some(&condition.id);
                            // Show effect type indicator in list
                            let effect_indicator = get_effect_type_indicator(condition);
                            let label = format!("{} {}", effect_indicator, condition.name);
                            if ui.selectable_label(is_selected, label).clicked() {
                                self.selected_condition_id = Some(condition.id.clone());
                            }
                        }
                    });

                ui.separator();
                if ui.button("➕ New Condition").clicked() {
                    let new_condition = ConditionDefinition {
                        id: "new_condition".to_string(),
                        name: "New Condition".to_string(),
                        description: "".to_string(),
                        effects: Vec::new(),
                        default_duration: ConditionDuration::Rounds(3),
                        icon_id: None,
                    };
                    self.edit_buffer = Some(new_condition);
                    self.selected_condition_id = None;
                }

                ui.separator();

                // Edit request capture - if Edit is clicked we will set edit buffer below to avoid borrow issues
                let mut edit_requested: Option<ConditionId> = None;

                ui.horizontal(|ui| {
                    // Edit selected - open edit buffer for the selected condition
                    if ui.button("✏️ Edit").clicked() {
                        edit_requested = self.selected_condition_id.clone();
                    }

                    // Duplicate selected
                    if ui.button("📄 Duplicate").clicked() {
                        if let Some(selected_id) = &self.selected_condition_id {
                            if let Some(original) = conditions.iter().find(|c| &c.id == selected_id)
                            {
                                // Create a copy with a unique id
                                let mut dup = original.clone();
                                let base = dup.id.clone();
                                let mut suffix = 1;
                                while conditions.iter().any(|c| c.id == dup.id) {
                                    dup.id = format!("{}_copy{}", base, suffix);
                                    suffix += 1;
                                }
                                conditions.push(dup);
                                *unsaved_changes = true;
                                *status_message = "Condition duplicated".to_string();
                            }
                        } else {
                            *status_message = "No condition selected to duplicate".to_string();
                        }
                    }

                    // Delete selected
                    if ui.button("🗑️ Delete").clicked() {
                        self.selected_for_delete = self.selected_condition_id.clone();
                        self.delete_confirmation_open = true;
                    }
                });

                // If Edit was requested, set the edit buffer after the UI closure to avoid borrow conflicts
                if let Some(edit_id) = edit_requested {
                    if let Some(cond) = conditions.iter().find(|c| c.id == edit_id) {
                        self.edit_buffer = Some(cond.clone());
                        self.editing_original_id = Some(edit_id);
                        // Clear selection so the full editor panel shows instead of read-only view
                        self.selected_condition_id = None;
                    }
                }
            });

            ui.separator();

            // Right panel: Editor
            ui.vertical(|ui| {
                ui.set_min_height(panel_height);
                ui.set_min_width(ui.available_width());

                if let Some(condition_id) = &self.selected_condition_id.clone() {
                    if let Some(condition) = conditions.iter_mut().find(|c| &c.id == condition_id) {
                        egui::ScrollArea::vertical()
                            .id_salt("condition_editor_scroll")
                            .auto_shrink([false, false])
                            .max_height(panel_height)
                            .show(ui, |ui| {
                                ui.heading("Edit Condition");
                                ui.separator();

                                // Used-by hint (shows spells referencing this condition)
                                let used_spells = spells_referencing_condition(spells, &condition.id);
                                if !used_spells.is_empty() {
                                    ui.colored_label(
                                        egui::Color32::YELLOW,
                                        format!("Used by {} spell(s):", used_spells.len()),
                                    );
                                    for s in used_spells {
                                        ui.horizontal(|ui| {
                                            ui.label(format!("- {}", s));
                                            if ui.small_button("📋").on_hover_text("Copy spell name").clicked() {
                                                ui.ctx().copy_text(s.clone());
                                                *status_message = format!("Copied spell name to clipboard: {}", s);
                                            }
                                            if ui.small_button("→").on_hover_text("Jump to spell in Spells Editor").clicked() {
                                                self.navigate_to_spell = Some(s.clone());
                                            }
                                        });
                                    }
                                    ui.separator();
                                }

                                egui::Grid::new("condition_editor_grid")
                                    .num_columns(2)
                                    .spacing([10.0, 10.0])
                                    .show(ui, |ui| {
                                        ui.label("ID:");
                                        ui.label(&condition.id);
                                        ui.end_row();

                                        ui.label("Name:");
                                        ui.text_edit_singleline(&mut condition.name);
                                        ui.end_row();

                                        ui.label("Description:");
                                        ui.text_edit_multiline(&mut condition.description);
                                        ui.end_row();
                                    });

                                ui.separator();
                                ui.label("Effects:");
                                for (idx, effect) in condition.effects.iter().enumerate() {
                                    ui.label(format!(
                                        "Effect #{}: {}",
                                        idx + 1,
                                        render_condition_effect_summary(effect)
                                    ));
                                }

                                // Preview for selected condition (non-edit mode)
                                if self.show_preview {
                                    ui.separator();
                                    ui.label("Preview:");
                                    ui.horizontal(|ui| {
                                        ui.label("Magnitude:");
                                        ui.add(egui::Slider::new(&mut self.preview_magnitude, 0.1..=3.0).text("x"));
                                    });
                                    for (idx, effect) in condition.effects.iter().enumerate() {
                                        ui.label(format!(
                                            "Effect #{}: {}",
                                            idx + 1,
                                            render_condition_effect_preview(effect, self.preview_magnitude)
                                        ));
                                    }
                                    ui.separator();
                                }
                            });
                    }
                } else if self.edit_buffer.is_some() {
                    // New condition block (Add mode)
                    if let Some(new_cond) = &mut self.edit_buffer {
                        let mut should_save = false;
                        let mut should_cancel = false;

                        egui::ScrollArea::vertical()
                            .id_salt("condition_new_scroll")
                            .auto_shrink([false, false])
                            .max_height(panel_height)
                            .show(ui, |ui| {
                                ui.heading("New Condition");
                                ui.separator();

                                // If we're editing an existing condition being renamed, warn about spells
                                // that reference the original ID.
                                if let Some(orig_id) = &self.editing_original_id {
                                    let used_spells = spells_referencing_condition(spells, orig_id);
                                    if !used_spells.is_empty() {
                                        ui.colored_label(
                                            egui::Color32::YELLOW,
                                            format!(
                                                "Note: This condition is used by {} spell(s). Renaming may break references.",
                                                used_spells.len()
                                            ),
                                        );
                                        for s in used_spells {
                                            ui.horizontal(|ui| {
                                                ui.label(format!("- {}", s));
                                                if ui.small_button("📋").on_hover_text("Copy spell name").clicked() {
                                                    ui.ctx().copy_text(s.clone());
                                                    *status_message = format!("Copied spell name to clipboard: {}", s);
                                                }
                                                if ui.small_button("→").on_hover_text("Jump to spell in Spells Editor").clicked() {
                                                    self.navigate_to_spell = Some(s.clone());
                                                }
                                            });
                                        }
                                        ui.separator();
                                    }
                                }

                                egui::Grid::new("condition_editor_grid")
                                    .num_columns(2)
                                    .spacing([10.0, 10.0])
                                    .show(ui, |ui| {
                                        ui.label("ID:");
                                        ui.text_edit_singleline(&mut new_cond.id);
                                        ui.end_row();

                                        ui.label("Name:");
                                        ui.text_edit_singleline(&mut new_cond.name);
                                        ui.end_row();

                                        ui.label("Description:");
                                        ui.text_edit_multiline(&mut new_cond.description);
                                        ui.end_row();

                                        // Default duration selector (Instant / Rounds(n) / Minutes(n) / Permanent)
                                        ui.label("Default Duration:")
                                            .on_hover_text("How long the condition lasts by default:\n• Instant: Applied once immediately\n• Rounds: Lasts N combat rounds\n• Minutes: Lasts N exploration minutes\n• Permanent: Never expires");
                                        ui.horizontal(|ui| {
                                            egui::ComboBox::from_id_salt(
                                                "condition_default_duration",
                                            )
                                            .selected_text(match &new_cond.default_duration {
                                                ConditionDuration::Instant => "Instant".to_owned(),
                                                ConditionDuration::Rounds(n) => {
                                                    format!("Rounds({})", n)
                                                }
                                                ConditionDuration::Minutes(n) => {
                                                    format!("Minutes({})", n)
                                                }
                                                ConditionDuration::Permanent => {
                                                    "Permanent".to_owned()
                                                }
                                            })
                                            .show_ui(
                                                ui,
                                                |ui| {
                                                    if ui
                                                        .selectable_label(
                                                            matches!(
                                                                new_cond.default_duration,
                                                                ConditionDuration::Instant
                                                            ),
                                                            "Instant",
                                                        )
                                                        .clicked()
                                                    {
                                                        new_cond.default_duration =
                                                            ConditionDuration::Instant;
                                                    }
                                                    if ui
                                                        .selectable_label(
                                                            matches!(
                                                                new_cond.default_duration,
                                                                ConditionDuration::Permanent
                                                            ),
                                                            "Permanent",
                                                        )
                                                        .clicked()
                                                    {
                                                        new_cond.default_duration =
                                                            ConditionDuration::Permanent;
                                                    }
                                                    if ui
                                                        .selectable_label(
                                                            matches!(
                                                                new_cond.default_duration,
                                                                ConditionDuration::Rounds(_)
                                                            ),
                                                            "Rounds",
                                                        )
                                                        .clicked()
                                                    {
                                                        if let ConditionDuration::Rounds(_) =
                                                            new_cond.default_duration
                                                        {
                                                            // keep value
                                                        } else {
                                                            new_cond.default_duration =
                                                                ConditionDuration::Rounds(1);
                                                        }
                                                    }
                                                    if ui
                                                        .selectable_label(
                                                            matches!(
                                                                new_cond.default_duration,
                                                                ConditionDuration::Minutes(_)
                                                            ),
                                                            "Minutes",
                                                        )
                                                        .clicked()
                                                    {
                                                        if let ConditionDuration::Minutes(_) =
                                                            new_cond.default_duration
                                                        {
                                                            // keep value
                                                        } else {
                                                            new_cond.default_duration =
                                                                ConditionDuration::Minutes(1);
                                                        }
                                                    }
                                                },
                                            );
                                            // If Rounds or Minutes selected, show a numeric editor
                                            match &mut new_cond.default_duration {
                                                ConditionDuration::Rounds(n) => {
                                                    ui.add(egui::DragValue::new(n));
                                                }
                                                ConditionDuration::Minutes(n) => {
                                                    ui.add(egui::DragValue::new(n));
                                                }
                                                _ => {
                                                    // nothing to show for Instant or Permanent
                                                }
                                            }
                                        });
                                        ui.end_row();

                                        // Icon ID field (optional)
                                        ui.label("Icon ID:")
                                            .on_hover_text("Optional icon identifier for UI display");
                                        {
                                            let mut icon_buf =
                                                new_cond.icon_id.clone().unwrap_or_default();
                                            let text = ui.text_edit_singleline(&mut icon_buf);
                                            if text.changed() {
                                                new_cond.icon_id = if icon_buf.trim().is_empty() {
                                                    None
                                                } else {
                                                    Some(icon_buf.clone())
                                                };
                                            }
                                            if ui.button("Clear").clicked() {
                                                new_cond.icon_id = None;
                                            }
                                        }
                                        ui.end_row();
                                    });

                                // Basic ID uniqueness check (account for editing an existing condition)
                                let id_in_use = conditions.iter().any(|c| {
                                    // When editing an existing condition, the original id shouldn't be
                                    // considered a collision with itself.
                                    if let Some(orig_id) = &self.editing_original_id {
                                        if &c.id == orig_id {
                                            return false;
                                        }
                                    }
                                    c.id == new_cond.id
                                });
                                if id_in_use {
                                    ui.colored_label(egui::Color32::RED, "ID already in use");
                                }

                                ui.separator();

                                // Effects list & basic per-effect editing
                                ui.horizontal(|ui| {
                                    ui.label("Effects:");
                                    if !new_cond.effects.is_empty() {
                                        if ui.small_button("🗑️ Clear All")
                                            .on_hover_text("Remove all effects from this condition")
                                            .clicked()
                                        {
                                            new_cond.effects.clear();
                                        }
                                    }
                                });
                                let mut effect_action: Option<(String, usize)> = None;
                                for (idx, effect) in new_cond.effects.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label(format!(
                                            "Effect #{}: {}",
                                            idx + 1,
                                            render_condition_effect_summary(effect)
                                        ));
                                        if ui.button("⬆️").clicked() {
                                            effect_action = Some(("up".to_string(), idx));
                                        }
                                        if ui.button("⬇️").clicked() {
                                            effect_action = Some(("down".to_string(), idx));
                                        }
                                        if ui.button("✏️").clicked() {
                                            effect_action = Some(("edit".to_string(), idx));
                                        }
                                        if ui.button("📄 Duplicate").clicked() {
                                            effect_action = Some(("duplicate".to_string(), idx));
                                        }
                                        if ui.button("🗑️ Delete").clicked() {
                                            effect_action = Some(("delete".to_string(), idx));
                                        }
                                    });
                                }

                                // Small preview to help designers understand the condition
                                if self.show_preview {
                                    ui.separator();
                                    ui.label("Preview:")
                                        .on_hover_text("Preview shows approximate effect values scaled by magnitude.\nThis is for design visualization only and is not saved.");
                                    ui.horizontal(|ui| {
                                        ui.label("Magnitude:")
                                            .on_hover_text("Scale factor for preview (simulates spell power bonuses)");
                                        ui.add(
                                            egui::Slider::new(&mut self.preview_magnitude, 0.1..=3.0)
                                                .text("x"),
                                        );
                                    });
                                    for (idx, effect) in new_cond.effects.iter().enumerate() {
                                        ui.label(format!(
                                            "Effect #{}: {}",
                                            idx + 1,
                                            render_condition_effect_preview(effect, self.preview_magnitude)
                                        ));
                                    }
                                    ui.separator();
                                }

                                // Add new effect (defaults to StatusEffect)
                                if ui.button("➕ Add Effect").clicked() {
                                    self.effect_edit_buffer = Some(EffectEditBuffer::default());
                                    if let Some(buf) = &mut self.effect_edit_buffer {
                                        buf.effect_type = Some("StatusEffect".to_string());
                                        buf.editing_index = None;
                                    }
                                }

                                // Apply captured action after the list render
                                if let Some((action, idx)) = effect_action {
                                    match action.as_str() {
                                        "up" => {
                                            if idx > 0 {
                                                new_cond.effects.swap(idx, idx - 1);
                                            }
                                        }
                                        "down" => {
                                            if idx + 1 < new_cond.effects.len() {
                                                new_cond.effects.swap(idx, idx + 1);
                                            }
                                        }
                                        "duplicate" => {
                                            let dup = new_cond.effects[idx].clone();
                                            new_cond.effects.insert(idx + 1, dup);
                                        }
                                        "delete" => {
                                            new_cond.effects.remove(idx);
                                        }
                                        "edit" => {
                                            if idx < new_cond.effects.len() {
                                                let eff = new_cond.effects[idx].clone();
                                                let mut buf = EffectEditBuffer::default();
                                                buf.editing_index = Some(idx);
                                                buf.effect_type = Some(match &eff {
                                                    ConditionEffect::AttributeModifier {
                                                        ..
                                                    } => "AttributeModifier".to_string(),
                                                    ConditionEffect::StatusEffect(_) => {
                                                        "StatusEffect".to_string()
                                                    }
                                                    ConditionEffect::DamageOverTime { .. } => {
                                                        "DamageOverTime".to_string()
                                                    }
                                                    ConditionEffect::HealOverTime { .. } => {
                                                        "HealOverTime".to_string()
                                                    }
                                                });

                                                match eff {
                                                    ConditionEffect::AttributeModifier {
                                                        attribute,
                                                        value,
                                                    } => {
                                                        buf.attribute = attribute.clone();
                                                        buf.attribute_value = value;
                                                    }
                                                    ConditionEffect::StatusEffect(tag) => {
                                                        buf.status_tag = tag.clone();
                                                    }
                                                    ConditionEffect::DamageOverTime {
                                                        damage,
                                                        element,
                                                    } => {
                                                        buf.dice = damage.clone();
                                                        buf.element = element.clone();
                                                    }
                                                    ConditionEffect::HealOverTime { amount } => {
                                                        buf.dice = amount.clone();
                                                    }
                                                }

                                                self.effect_edit_buffer = Some(buf);
                                            }
                                        }
                                        _ => {}
                                    }
                                }

                                // Effect editor UI (for the edit/add panel)
                                if self.effect_edit_buffer.is_some() {
                                    // Take the edit buffer for this frame to avoid nested mutable borrows
                                    let mut buf = self.effect_edit_buffer.take().unwrap();
                                    let mut keep_buf = true;

                                    ui.separator();
                                    ui.label("Edit Effect");
                                    // Effect type selector
                                    let mut effect_type = buf
                                        .effect_type
                                        .clone()
                                        .unwrap_or_else(|| "StatusEffect".to_string());
                                    egui::ComboBox::from_id_salt("effect_type_select")
                                        .selected_text(effect_type.clone())
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_label(
                                                    effect_type == "AttributeModifier",
                                                    "AttributeModifier",
                                                )
                                                .clicked()
                                            {
                                                effect_type = "AttributeModifier".to_string();
                                            }
                                            if ui
                                                .selectable_label(
                                                    effect_type == "StatusEffect",
                                                    "StatusEffect",
                                                )
                                                .clicked()
                                            {
                                                effect_type = "StatusEffect".to_string();
                                            }
                                            if ui
                                                .selectable_label(
                                                    effect_type == "DamageOverTime",
                                                    "DamageOverTime",
                                                )
                                                .clicked()
                                            {
                                                effect_type = "DamageOverTime".to_string();
                                            }
                                            if ui
                                                .selectable_label(
                                                    effect_type == "HealOverTime",
                                                    "HealOverTime",
                                                )
                                                .clicked()
                                            {
                                                effect_type = "HealOverTime".to_string();
                                            }
                                        });
                                    buf.effect_type = Some(effect_type.clone());

                                    match effect_type.as_str() {
                                        "AttributeModifier" => {
                                            ui.horizontal(|ui| {
                                                ui.label("Attribute:").on_hover_text("Primary attributes: might, intellect, personality, endurance, speed, accuracy, luck. Select 'Custom' to enter a custom attribute name.");
                                                let attributes = [
                                                    "might",
                                                    "intellect",
                                                    "personality",
                                                    "endurance",
                                                    "speed",
                                                    "accuracy",
                                                    "luck",
                                                ];
                                                egui::ComboBox::from_id_salt("attribute_select")
                                                    .selected_text(buf.attribute.clone())
                                                    .show_ui(ui, |ui| {
                                                        for a in attributes {
                                                            if ui
                                                                .selectable_value(
                                                                    &mut buf.attribute,
                                                                    a.to_string(),
                                                                    a,
                                                                )
                                                                .clicked()
                                                            {
                                                            }
                                                        }
                                                        if ui
                                                            .selectable_label(
                                                                buf.attribute == "custom",
                                                                "Custom",
                                                            )
                                                            .clicked()
                                                        {
                                                            buf.attribute = "custom".to_string();
                                                        }
                                                    });
                                                if buf.attribute == "custom" {
                                                    ui.text_edit_singleline(&mut buf.attribute);
                                                }
                                                ui.label("Value:");
                                                ui.add(
                                                    egui::DragValue::new(&mut buf.attribute_value)
                                                        .speed(1.0),
                                                );
                                            });
                                        }
                                        "StatusEffect" => {
                                            ui.horizontal(|ui| {
                                                ui.label("Status Tag:");
                                                ui.text_edit_singleline(&mut buf.status_tag);
                                            });
                                        }
                                        "DamageOverTime" => {
                                            ui.horizontal(|ui| {
                                                ui.label("Damage:");
                                                ui.add(
                                                    egui::DragValue::new(&mut buf.dice.count)
                                                        .speed(1.0)
                                                        .range(1..=100),
                                                );
                                                ui.label("d");
                                                ui.add(
                                                    egui::DragValue::new(&mut buf.dice.sides)
                                                        .speed(1.0)
                                                        .range(2..=100),
                                                );
                                                ui.label("+");
                                                ui.add(
                                                    egui::DragValue::new(&mut buf.dice.bonus)
                                                        .speed(1.0),
                                                );
                                                ui.label("Element:").on_hover_text("Choose an element for DOT effects (fire, cold, electricity, poison, acid, psychic, energy, physical). Select 'Custom' to enter a custom element name.");
                                                let elements = [
                                                    "fire",
                                                    "cold",
                                                    "electricity",
                                                    "poison",
                                                    "acid",
                                                    "psychic",
                                                    "energy",
                                                    "physical",
                                                ];
                                                egui::ComboBox::from_id_salt("dot_element_select")
                                                    .selected_text(buf.element.clone())
                                                    .show_ui(ui, |ui| {
                                                        for e in elements {
                                                            if ui
                                                                .selectable_value(
                                                                    &mut buf.element,
                                                                    e.to_string(),
                                                                    e,
                                                                )
                                                                .clicked()
                                                            {
                                                            }
                                                        }
                                                        if ui
                                                            .selectable_label(
                                                                buf.element == "custom",
                                                                "Custom",
                                                            )
                                                            .clicked()
                                                        {
                                                            buf.element = "custom".to_string();
                                                        }
                                                    });
                                                if buf.element == "custom" {
                                                    ui.text_edit_singleline(&mut buf.element);
                                                }
                                            });
                                        }
                                        "HealOverTime" => {
                                            ui.horizontal(|ui| {
                                                ui.label("Amount:");
                                                ui.add(
                                                    egui::DragValue::new(&mut buf.dice.count)
                                                        .speed(1.0)
                                                        .range(1..=100),
                                                );
                                                ui.label("d");
                                                ui.add(
                                                    egui::DragValue::new(&mut buf.dice.sides)
                                                        .speed(1.0)
                                                        .range(2..=100),
                                                );
                                                ui.label("+");
                                                ui.add(
                                                    egui::DragValue::new(&mut buf.dice.bonus)
                                                        .speed(1.0),
                                                );
                                            });
                                        }
                                        _ => {}
                                    }

                                    ui.horizontal(|ui| {
                                        let effect_validation = validate_effect_edit_buffer(&buf);
                                        if effect_validation.is_ok() {
                                            if ui.button("💾 Save Effect").clicked() {
                                                // Convert buffer into a ConditionEffect and store on the
                                                // in-flight edit buffer (this is saved to the campaign
                                                // when the parent condition is saved).
                                                let new_effect = match buf.effect_type.as_deref() {
                                                    Some("AttributeModifier") => {
                                                        ConditionEffect::AttributeModifier {
                                                            attribute: buf.attribute.clone(),
                                                            value: buf.attribute_value,
                                                        }
                                                    }
                                                    Some("StatusEffect") => {
                                                        ConditionEffect::StatusEffect(
                                                            buf.status_tag.clone(),
                                                        )
                                                    }
                                                    Some("DamageOverTime") => {
                                                        ConditionEffect::DamageOverTime {
                                                            damage: buf.dice.clone(),
                                                            element: buf.element.clone(),
                                                        }
                                                    }
                                                    Some("HealOverTime") => {
                                                        ConditionEffect::HealOverTime {
                                                            amount: buf.dice.clone(),
                                                        }
                                                    }
                                                    _ => ConditionEffect::StatusEffect(
                                                        buf.status_tag.clone(),
                                                    ),
                                                };

                                                if let Some(idx) = buf.editing_index {
                                                    if idx < new_cond.effects.len() {
                                                        new_cond.effects[idx] = new_effect;
                                                    } else {
                                                        new_cond.effects.push(new_effect);
                                                    }
                                                } else {
                                                    new_cond.effects.push(new_effect);
                                                }

                                                // We've saved the edit into the edit_buffer->effects,
                                                // we can drop the UI edit buffer now.
                                                keep_buf = false;
                                            }
                                        } else {
                                            // Show validation error and render disabled save button
                                            ui.colored_label(egui::Color32::RED, effect_validation.unwrap_err());
                                            ui.add_enabled(false, egui::Button::new("💾 Save Effect"));
                                        }
                                        if ui.button("❌ Cancel").clicked() {
                                            // Cancel editing; drop the buffer
                                            keep_buf = false;
                                        }
                                    });

                                    if keep_buf {
                                        // Return the buffer to the editor state so the user can continue editing
                                        self.effect_edit_buffer = Some(buf);
                                    } else {
                                        // Bail out without re-presenting the buffer
                                        self.effect_edit_buffer = None;
                                    }
                                }

                                ui.separator();

                                ui.horizontal(|ui| {
                                    if ui.button("💾 Save").clicked() {
                                        should_save = true;
                                    }
                                    if ui.button("❌ Cancel").clicked() {
                                        should_cancel = true;
                                    }
                                });
                            });

                        if should_save {
                            // Attempt to apply the edits using the shared helper. This
                            // handles both creating a new condition and updating an
                            // existing one (via `editing_original_id`).
                            match apply_condition_edits(
                                conditions,
                                self.editing_original_id.as_deref(),
                                new_cond,
                            ) {
                                Ok(()) => {
                                    *unsaved_changes = true;
                                    *status_message = if self.editing_original_id.is_some() {
                                        "Condition updated".to_string()
                                    } else {
                                        "Condition added".to_string()
                                    };
                                    self.selected_condition_id = Some(new_cond.id.clone());
                                    self.edit_buffer = None;
                                    self.editing_original_id = None;
                                }
                                Err(e) => {
                                    *status_message = format!("Failed to save condition: {}", e);
                                }
                            }
                        }

                        if should_cancel {
                            self.edit_buffer = None;
                            self.editing_original_id = None;
                        }
                    }
                } else {
                    ui.label(
                        "Select a condition to edit or click '➕ New Condition' to create one.",
                    );
                }
            });
        });

        // Import dialog window handling (separate)
        if self.show_import_dialog {
            self.show_import_dialog(
                ui.ctx(),
                conditions,
                unsaved_changes,
                status_message,
                campaign_dir,
                conditions_file,
            );
        }

        // Delete confirmation dialog
        if self.delete_confirmation_open {
            let mut open = self.delete_confirmation_open;
            egui::Window::new("Delete Condition")
                .open(&mut open)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("Are you sure you want to delete this condition?");
                    if let Some(del_id) = &self.selected_for_delete {
                        let used_spells = spells_referencing_condition(spells, del_id);
                        if !used_spells.is_empty() {
                            ui.colored_label(
                                egui::Color32::YELLOW,
                                format!(
                                    "Warning: {} spell(s) reference this condition:",
                                    used_spells.len()
                                ),
                            );
                            for s in used_spells {
                                ui.horizontal(|ui| {
                                    ui.label(format!("- {}", s));
                                    if ui.small_button("📋").on_hover_text("Copy spell name").clicked() {
                                        ui.ctx().copy_text(s.clone());
                                        *status_message =
                                            format!("Copied spell name to clipboard: {}", s);
                                    }
                                    if ui.small_button("→").on_hover_text("Jump to spell in Spells Editor").clicked() {
                                        self.navigate_to_spell = Some(s.clone());
                                    }
                                });
                            }
                            ui.checkbox(
                                &mut self.remove_refs_on_delete,
                                "Remove references from spells when deleting",
                            ).on_hover_text("If checked, spells that use this condition will have the reference automatically removed");
                        }
                    }

                    if ui.button("Yes, delete").clicked() {
                        if let Some(del_id) = &self.selected_for_delete {
                            if let Some(pos) = conditions.iter().position(|c| &c.id == del_id) {
                                conditions.remove(pos);
                                if self.remove_refs_on_delete {
                                    let cnt =
                                        remove_condition_references_from_spells(spells, del_id);
                                    *status_message =
                                        format!("Removed references from {} spell(s).", cnt);
                                }
                                *unsaved_changes = true;
                                *status_message = "Condition deleted".to_string();
                                self.selected_condition_id = None;
                            }
                        }
                        self.delete_confirmation_open = false;
                        self.selected_for_delete = None;
                        self.remove_refs_on_delete = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.delete_confirmation_open = false;
                        self.selected_for_delete = None;
                        self.remove_refs_on_delete = false;
                    }
                });
            self.delete_confirmation_open = open;
        }
    }

    /// Shows the import dialog allowing RON paste/copy of a single ConditionDefinition.
    fn show_import_dialog(
        &mut self,
        ctx: &egui::Context,
        conditions: &mut Vec<ConditionDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        conditions_file: &str,
    ) {
        let mut open = self.show_import_dialog;

        egui::Window::new("Import/Export Condition")
            .open(&mut open)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.heading("Condition RON Data");
                ui.separator();

                ui.label("Paste RON data to import, or copy exported data:");
                let text_edit = egui::TextEdit::multiline(&mut self.import_export_buffer)
                    .desired_rows(15)
                    .code_editor();
                ui.add(text_edit);

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("📥 Import").clicked() {
                        // Try to parse a single ConditionDefinition
                        match ron::from_str::<ConditionDefinition>(&self.import_export_buffer) {
                            Ok(mut cond) => {
                                // Ensure unique ID
                                let base_id = cond.id.clone();
                                let mut new_id = base_id.clone();
                                let mut suffix = 1;
                                while conditions.iter().any(|c| c.id == new_id) {
                                    new_id = format!("{}_copy{}", base_id, suffix);
                                    suffix += 1;
                                }
                                cond.id = new_id;
                                conditions.push(cond);
                                // Optionally save to default campaign location
                                self.save_conditions(
                                    conditions,
                                    campaign_dir,
                                    conditions_file,
                                    unsaved_changes,
                                    status_message,
                                );
                                *status_message = "Condition imported successfully".to_string();
                                self.show_import_dialog = false;
                            }
                            Err(e) => {
                                *status_message = format!("Import failed: {}", e);
                            }
                        }
                    }

                    if ui.button("📋 Copy to Clipboard").clicked() {
                        ui.ctx().copy_text(self.import_export_buffer.clone());
                        *status_message = "Copied to clipboard".to_string();
                    }

                    if ui.button("❌ Close").clicked() {
                        self.show_import_dialog = false;
                    }
                });
            });

        self.show_import_dialog = open;
    }

    /// Saves conditions to the default campaign path (if provided).
    fn save_conditions(
        &self,
        conditions: &Vec<ConditionDefinition>,
        campaign_dir: Option<&PathBuf>,
        conditions_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        if let Some(dir) = campaign_dir {
            let conditions_path = dir.join(conditions_file);

            if let Some(parent) = conditions_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(false);

            match ron::ser::to_string_pretty(conditions, ron_config) {
                Ok(contents) => match std::fs::write(&conditions_path, contents) {
                    Ok(_) => {
                        *unsaved_changes = true;
                        // Do not always set status_message here (could be used by autosave)
                    }
                    Err(e) => {
                        *status_message = format!("Failed to write conditions file: {}", e);
                    }
                },
                Err(e) => {
                    *status_message = format!("Failed to serialize conditions: {}", e);
                }
            }
        }
    }
}

/// Applies an edited condition buffer to a list of conditions, covering both
/// new condition creation and updating existing ones.
///
/// * `conditions` - Mutable vector of conditions to apply changes to.
/// * `original_id` - If `Some(id)` this will update the existing entry identified
///   by `id`; if `None` this will attempt to insert the `edited` condition as a
///   new entry.
/// * `edited` - The edited `ConditionDefinition` to apply.
///
/// Returns:
/// * `Ok(())` on success
/// * `Err(String)` with a descriptive message on validation error
///
/// Validation rules:
/// * ID cannot be empty
/// * When updating, if the ID changed it must not collide with another existing condition
/// * When inserting, the ID must not collide with an existing one
pub(crate) fn apply_condition_edits(
    conditions: &mut Vec<ConditionDefinition>,
    original_id: Option<&str>,
    edited: &ConditionDefinition,
) -> Result<(), String> {
    // Ensure ID present
    if edited.id.trim().is_empty() {
        return Err("ID cannot be empty".to_string());
    }

    // Validate effect fields in the definition before applying (dice, ranges, etc).
    if let Err(e) = validate_condition_definition(edited) {
        return Err(e);
    }

    // If original_id provided -> update path
    if let Some(orig_id) = original_id {
        if let Some(idx) = conditions.iter().position(|c| c.id == orig_id) {
            // If the new ID differs and is in use, it's a duplicate
            if edited.id != orig_id && conditions.iter().any(|c| c.id == edited.id) {
                return Err(format!("Duplicate condition ID: {}", edited.id));
            }
            // Replace in-place
            conditions[idx] = edited.clone();
            Ok(())
        } else {
            Err(format!("Original condition '{}' not found", orig_id))
        }
    } else {
        // New condition path: ensure ID is unique
        if conditions.iter().any(|c| c.id == edited.id) {
            return Err(format!("Duplicate condition ID: {}", edited.id));
        }
        conditions.push(edited.clone());
        Ok(())
    }
}

// Helper rendering & effect utilities

/// Formats a concise, human readable summary for display in the editor UI.
///
/// This is intentionally UI-side only (used for previews and lists).
pub(crate) fn render_condition_effect_summary(effect: &ConditionEffect) -> String {
    match effect {
        ConditionEffect::AttributeModifier { attribute, value } => {
            format!("Attribute: {} ({:+})", attribute, value)
        }
        ConditionEffect::StatusEffect(tag) => format!("Status: {}", tag),
        ConditionEffect::DamageOverTime { damage, element } => {
            format!(
                "DOT: {}d{}+{} ({})",
                damage.count, damage.sides, damage.bonus, element
            )
        }
        ConditionEffect::HealOverTime { amount } => {
            format!("HOT: {}d{}+{}", amount.count, amount.sides, amount.bonus)
        }
    }
}

/// Computes a preview string for the given effect adjusted by `magnitude`.
///
/// This function produces an approximate, human-readable preview rather than a
/// strict simulation — it's intended to help designers understand the relative
/// impact of a `ConditionDefinition` while editing.
pub(crate) fn render_condition_effect_preview(effect: &ConditionEffect, magnitude: f32) -> String {
    match effect {
        ConditionEffect::AttributeModifier { attribute, value } => {
            let scaled = ((*value as f32) * magnitude).round() as i16;
            format!("Attribute: {} ({:+} -> {:+})", attribute, value, scaled)
        }
        ConditionEffect::StatusEffect(tag) => format!("Status: {}", tag),
        ConditionEffect::DamageOverTime { damage, element } => {
            let avg =
                (damage.count as f32) * ((damage.sides as f32 + 1.0) / 2.0) + damage.bonus as f32;
            let scaled = (avg * magnitude).round();
            format!("DOT: avg {:.1} -> {:.1} ({})", avg, scaled, element)
        }
        ConditionEffect::HealOverTime { amount } => {
            let avg =
                (amount.count as f32) * ((amount.sides as f32 + 1.0) / 2.0) + amount.bonus as f32;
            let scaled = (avg * magnitude).round();
            format!("HOT: avg {:.1} -> {:.1}", avg, scaled)
        }
    }
}

/// Add a new effect to a condition.
pub(crate) fn add_effect_to_condition(
    condition: &mut ConditionDefinition,
    effect: ConditionEffect,
) {
    condition.effects.push(effect);
}

/// Update an existing effect in a condition by index.
/// Updates an existing effect at a specific `index` for the given condition.
///
/// # Arguments
///
/// * `condition` - The `ConditionDefinition` that contains the effects vector.
/// * `index` - The index of the effect to update.
/// * `effect` - The new `ConditionEffect` value to replace the existing one.
///
/// # Errors
///
/// Returns an `Err(String)` if `index` is out of range.
pub(crate) fn update_effect_in_condition(
    condition: &mut ConditionDefinition,
    index: usize,
    effect: ConditionEffect,
) -> Result<(), String> {
    if index >= condition.effects.len() {
        return Err(format!("Effect index {} out of range", index));
    }
    condition.effects[index] = effect;
    Ok(())
}

/// Delete an effect from a condition by index.
/// Deletes the effect at `index` from the condition's effect list.
///
/// # Arguments
///
/// * `condition` - The `ConditionDefinition` whose effect will be removed.
/// * `index` - The index of the effect to remove.
///
/// # Errors
///
/// Returns `Err(String)` if `index` is out of range.
pub(crate) fn delete_effect_from_condition(
    condition: &mut ConditionDefinition,
    index: usize,
) -> Result<(), String> {
    if index >= condition.effects.len() {
        return Err(format!("Effect index {} out of range", index));
    }
    condition.effects.remove(index);
    Ok(())
}

/// Duplicate an effect in a condition at `index` (insert copy after the index).
/// Duplicates the effect at `index` and inserts the copy directly after it.
///
/// # Arguments
///
/// * `condition` - Target `ConditionDefinition` containing the effect list.
/// * `index` - Index of the effect to duplicate.
///
/// # Errors
///
/// Returns `Err(String)` if `index` is out of range.
pub(crate) fn duplicate_effect_in_condition(
    condition: &mut ConditionDefinition,
    index: usize,
) -> Result<(), String> {
    if index >= condition.effects.len() {
        return Err(format!("Effect index {} out of range", index));
    }
    let dup = condition.effects[index].clone();
    condition.effects.insert(index + 1, dup);
    Ok(())
}

/// Move an effect within the effect list by direction (dir = -1 up, dir = 1 down).
/// Moves an effect at `index` by the specified `dir`.
///
/// * `dir < 0` moves the effect up (towards the start of the list).
/// * `dir >= 0` moves the effect down (towards the end of the list).
///
/// # Arguments
///
/// * `condition` - The `ConditionDefinition` to modify.
/// * `index` - The index of the effect to move.
/// * `dir` - Direction: negative for up; non-negative for down.
///
/// # Errors
///
/// Returns `Err(String)` if:
/// * `index` is out of range, or
/// * attempting to move 'up' at the top or 'down' at the bottom of the list.
pub(crate) fn move_effect_in_condition(
    condition: &mut ConditionDefinition,
    index: usize,
    dir: i8,
) -> Result<(), String> {
    if index >= condition.effects.len() {
        return Err("Index out of range".to_string());
    }
    if dir < 0 {
        if index == 0 {
            return Err("Cannot move up".to_string());
        }
        condition.effects.swap(index - 1, index);
    } else {
        if index + 1 >= condition.effects.len() {
            return Err("Cannot move down".to_string());
        }
        condition.effects.swap(index, index + 1);
    }
    Ok(())
}

/// Validate the UI-side effect edit buffer. This mirrors the checks done on
/// `ConditionEffect` fields and is used to provide immediate UI feedback.
pub(crate) fn validate_effect_edit_buffer(buf: &EffectEditBuffer) -> Result<(), String> {
    match buf.effect_type.as_deref() {
        Some("AttributeModifier") => {
            if buf.attribute.trim().is_empty() {
                return Err("Attribute name cannot be empty".to_string());
            }
            if buf.attribute_value.abs() > 255 {
                return Err("Attribute value must be in range [-255..255]".to_string());
            }
            Ok(())
        }
        Some("StatusEffect") => {
            if buf.status_tag.trim().is_empty() {
                return Err("Status tag cannot be empty".to_string());
            }
            Ok(())
        }
        Some("DamageOverTime") => {
            if buf.dice.count < 1 {
                return Err("Damage dice count must be >= 1".to_string());
            }
            if buf.dice.sides < 2 {
                return Err("Damage dice sides must be >= 2".to_string());
            }
            if buf.element.trim().is_empty() {
                return Err("Damage element cannot be empty".to_string());
            }
            Ok(())
        }
        Some("HealOverTime") => {
            if buf.dice.count < 1 {
                return Err("Heal dice count must be >= 1".to_string());
            }
            if buf.dice.sides < 2 {
                return Err("Heal dice sides must be >= 2".to_string());
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Perform broader validation on an entire `ConditionDefinition`. This ensures
/// IDs and effect contents are sensible prior to saving.
pub(crate) fn validate_condition_definition(cond: &ConditionDefinition) -> Result<(), String> {
    if cond.id.trim().is_empty() {
        return Err("Condition ID cannot be empty".to_string());
    }

    for (idx, eff) in cond.effects.iter().enumerate() {
        match eff {
            ConditionEffect::AttributeModifier { attribute, value } => {
                if attribute.trim().is_empty() {
                    return Err(format!("Effect #{}: attribute must be set", idx + 1));
                }
                if value.abs() > 255 {
                    return Err(format!(
                        "Effect #{}: attribute value must be in range [-255..255]",
                        idx + 1
                    ));
                }
            }
            ConditionEffect::StatusEffect(tag) => {
                if tag.trim().is_empty() {
                    return Err(format!("Effect #{}: status tag cannot be empty", idx + 1));
                }
            }
            ConditionEffect::DamageOverTime { damage, element } => {
                if damage.count < 1 {
                    return Err(format!(
                        "Effect #{}: damage dice count must be >= 1",
                        idx + 1
                    ));
                }
                if damage.sides < 2 {
                    return Err(format!(
                        "Effect #{}: damage dice sides must be >= 2",
                        idx + 1
                    ));
                }
                if element.trim().is_empty() {
                    return Err(format!(
                        "Effect #{}: damage element cannot be empty",
                        idx + 1
                    ));
                }
            }
            ConditionEffect::HealOverTime { amount } => {
                if amount.count < 1 {
                    return Err(format!("Effect #{}: heal dice count must be >= 1", idx + 1));
                }
                if amount.sides < 2 {
                    return Err(format!("Effect #{}: heal dice sides must be >= 2", idx + 1));
                }
            }
        }
    }

    Ok(())
}

/// Returns a vector of spell names that reference `condition_id`.
pub(crate) fn spells_referencing_condition(spells: &Vec<Spell>, condition_id: &str) -> Vec<String> {
    let mut result = Vec::new();
    for spell in spells.iter() {
        if spell.applied_conditions.iter().any(|c| c == condition_id) {
            result.push(spell.name.clone());
        }
    }
    result
}

/// Remove references of `condition_id` from spells and return the number of spells modified.
pub(crate) fn remove_condition_references_from_spells(
    spells: &mut Vec<Spell>,
    condition_id: &str,
) -> usize {
    let mut modified = 0usize;
    for spell in spells.iter_mut() {
        let original_len = spell.applied_conditions.len();
        spell.applied_conditions.retain(|c| c != condition_id);
        if spell.applied_conditions.len() != original_len {
            modified += 1;
        }
    }
    modified
}

/// Statistics about conditions in the campaign
#[derive(Debug, Clone, Default)]
pub struct ConditionStatistics {
    pub total: usize,
    pub attribute_count: usize,
    pub status_count: usize,
    pub dot_count: usize,
    pub hot_count: usize,
    pub empty_count: usize,
    pub multi_effect_count: usize,
}

/// Compute statistics about the conditions collection
pub fn compute_condition_statistics(conditions: &[ConditionDefinition]) -> ConditionStatistics {
    let mut stats = ConditionStatistics {
        total: conditions.len(),
        ..Default::default()
    };

    for cond in conditions {
        if cond.effects.is_empty() {
            stats.empty_count += 1;
        }
        if cond.effects.len() > 1 {
            stats.multi_effect_count += 1;
        }
        for effect in &cond.effects {
            match effect {
                ConditionEffect::AttributeModifier { .. } => stats.attribute_count += 1,
                ConditionEffect::StatusEffect(_) => stats.status_count += 1,
                ConditionEffect::DamageOverTime { .. } => stats.dot_count += 1,
                ConditionEffect::HealOverTime { .. } => stats.hot_count += 1,
            }
        }
    }

    stats
}

/// Returns a short emoji indicator for the primary effect type of a condition
fn get_effect_type_indicator(condition: &ConditionDefinition) -> &'static str {
    if condition.effects.is_empty() {
        return "○"; // Empty
    }
    // Check for the "primary" effect type (first effect determines icon)
    match &condition.effects[0] {
        ConditionEffect::AttributeModifier { value, .. } => {
            if *value >= 0 {
                "⬆" // Buff
            } else {
                "⬇" // Debuff
            }
        }
        ConditionEffect::StatusEffect(_) => "◆", // Status
        ConditionEffect::DamageOverTime { .. } => "🔥", // DOT
        ConditionEffect::HealOverTime { .. } => "💚", // HOT
    }
}

pub fn render_conditions_editor(
    ui: &mut egui::Ui,
    state: &mut ConditionsEditorState,
    conditions: &mut Vec<ConditionDefinition>,
) {
    // For compatibility we provide local placeholders if the app does not yet use
    // the newer `show(...)` signature. Editor features will be limited without
    // actual `campaign_dir` and `status_message` passed.
    let mut unsaved_changes = false;
    let mut status_message = String::new();
    let mut file_load_merge_mode = false;
    let mut dummy_spells: Vec<Spell> = Vec::new();
    state.show(
        ui,
        conditions,
        &mut dummy_spells,
        None,
        "data/conditions.ron",
        &mut unsaved_changes,
        &mut status_message,
        &mut file_load_merge_mode,
    );
}
