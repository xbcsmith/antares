// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::ui_helpers::{
    ActionButtons, EditorToolbar, ItemAction, ToolbarAction, TwoColumnLayout,
    DEFAULT_PANEL_MIN_HEIGHT,
};
use antares::domain::character::{ATTRIBUTE_MODIFIER_MAX, ATTRIBUTE_MODIFIER_MIN};
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
/// use campaign_builder::conditions_editor::ConditionsEditorState;
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

/// Editor mode for the conditions editor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionsEditorMode {
    /// List view - showing condition list and detail preview
    List,
    /// Add mode - creating a new condition
    Add,
    /// Edit mode - editing an existing condition
    Edit,
}

pub struct ConditionsEditorState {
    pub search_filter: String,
    pub selected_condition_idx: Option<usize>,
    pub edit_buffer: Option<ConditionDefinition>,
    pub show_preview: bool,
    /// Preview magnitude slider value (non-persistent UI-only preview for ActiveCondition.magnitude)
    pub preview_magnitude: f32,
    /// When deleting a condition, optionally remove references from spells (UI toggle)
    pub remove_refs_on_delete: bool,

    // Phase 1 additions
    pub show_import_dialog: bool,
    pub import_export_buffer: String,
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
    /// File load merge mode
    pub file_load_merge_mode: bool,
    /// Current editor mode
    pub mode: ConditionsEditorMode,
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
            selected_condition_idx: None,
            edit_buffer: None,
            show_preview: true,
            preview_magnitude: 1.0,
            remove_refs_on_delete: false,

            // new fields
            show_import_dialog: false,
            import_export_buffer: String::new(),
            delete_confirmation_open: false,
            selected_for_delete: None,
            editing_original_id: None,
            effect_edit_buffer: None,

            // QoL Phase additions
            filter_effect_type: EffectTypeFilter::All,
            sort_order: ConditionSortOrder::NameAsc,
            show_statistics: false,
            navigate_to_spell: None,
            file_load_merge_mode: false,
            mode: ConditionsEditorMode::List,
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
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        conditions: &mut Vec<ConditionDefinition>,
        spells: &mut [Spell],
        campaign_dir: Option<&PathBuf>,
        conditions_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        _file_load_merge_mode: &mut bool,
    ) {
        ui.heading("⚕️ Conditions Editor");
        ui.add_space(5.0);

        // Use shared EditorToolbar component
        let toolbar_action = EditorToolbar::new("Conditions")
            .with_search(&mut self.search_filter)
            .with_merge_mode(&mut self.file_load_merge_mode)
            .with_total_count(conditions.len())
            .with_id_salt("conditions_toolbar")
            .show(ui);

        // Handle toolbar actions
        match toolbar_action {
            ToolbarAction::New => {
                self.edit_buffer = Some(Self::default_condition());
                self.editing_original_id = None;
                self.mode = ConditionsEditorMode::Add;
            }
            ToolbarAction::Save => {
                self.save_conditions(
                    conditions,
                    campaign_dir,
                    conditions_file,
                    unsaved_changes,
                    status_message,
                );
            }
            ToolbarAction::Load => {
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
                            if self.file_load_merge_mode {
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
            ToolbarAction::Import => {
                self.show_import_dialog = true;
                self.import_export_buffer.clear();
            }
            ToolbarAction::Export => {
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
            ToolbarAction::Reload => {
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
            ToolbarAction::None => {}
        }

        // Filter toolbar (conditions-specific)
        ui.horizontal(|ui| {
            ui.label("Filters:");

            // Effect type filter
            egui::ComboBox::from_id_salt("condition_effect_filter")
                .selected_text(self.filter_effect_type.as_str())
                .show_ui(ui, |ui| {
                    for filter in EffectTypeFilter::all() {
                        if ui
                            .selectable_label(self.filter_effect_type == filter, filter.as_str())
                            .clicked()
                        {
                            self.filter_effect_type = filter;
                            self.selected_condition_idx = None;
                        }
                    }
                });

            ui.separator();

            // Sort order
            ui.label("Sort:");
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
        }

        ui.separator();

        // Show appropriate view based on mode
        match self.mode {
            ConditionsEditorMode::List => self.show_list(
                ui,
                conditions,
                spells,
                unsaved_changes,
                status_message,
                campaign_dir,
                conditions_file,
            ),
            ConditionsEditorMode::Add | ConditionsEditorMode::Edit => self.show_form(
                ui,
                conditions,
                spells,
                unsaved_changes,
                status_message,
                campaign_dir,
                conditions_file,
            ),
        }

        // Import dialog window handling
        if self.show_import_dialog {
            self.show_import_dialog_window(
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
            self.show_delete_confirmation(
                ui.ctx(),
                conditions,
                spells,
                unsaved_changes,
                status_message,
                campaign_dir,
                conditions_file,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        conditions: &mut Vec<ConditionDefinition>,
        spells: &mut [Spell],
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        conditions_file: &str,
    ) {
        let search_lower = self.search_filter.to_lowercase();

        // Build filtered list snapshot to avoid borrow conflicts in closures
        let mut filtered_conditions: Vec<(usize, String, ConditionDefinition)> = conditions
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                // Text filter
                let text_match = search_lower.is_empty()
                    || c.name.to_lowercase().contains(&search_lower)
                    || c.id.to_lowercase().contains(&search_lower);
                // Effect type filter
                let type_match = self.filter_effect_type.matches(c);
                text_match && type_match
            })
            .map(|(idx, c)| {
                let effect_indicator = get_effect_type_indicator(c);
                let label = format!("{} {}", effect_indicator, c.name);
                (idx, label, c.clone())
            })
            .collect();

        // Apply sorting
        match self.sort_order {
            ConditionSortOrder::NameAsc => {
                filtered_conditions
                    .sort_by(|a, b| a.2.name.to_lowercase().cmp(&b.2.name.to_lowercase()));
            }
            ConditionSortOrder::NameDesc => {
                filtered_conditions
                    .sort_by(|a, b| b.2.name.to_lowercase().cmp(&a.2.name.to_lowercase()));
            }
            ConditionSortOrder::IdAsc => {
                filtered_conditions
                    .sort_by(|a, b| a.2.id.to_lowercase().cmp(&b.2.id.to_lowercase()));
            }
            ConditionSortOrder::IdDesc => {
                filtered_conditions
                    .sort_by(|a, b| b.2.id.to_lowercase().cmp(&a.2.id.to_lowercase()));
            }
            ConditionSortOrder::EffectCount => {
                filtered_conditions.sort_by(|a, b| b.2.effects.len().cmp(&a.2.effects.len()));
            }
        }

        let selected = self.selected_condition_idx;
        let mut new_selection = selected;
        let mut action_requested: Option<ItemAction> = None;
        let show_preview = self.show_preview;
        let preview_magnitude = self.preview_magnitude;

        // Capture spell references for selected condition (if any)
        let selected_spell_refs: Vec<String> = if let Some(idx) = selected {
            if idx < conditions.len() {
                spells_referencing_condition(spells, &conditions[idx].id)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let mut navigate_spell: Option<String> = None;

        // Use shared TwoColumnLayout component
        TwoColumnLayout::new("conditions").show_split(
            ui,
            |left_ui| {
                // Left panel: Conditions list
                left_ui.heading("Conditions");
                left_ui.separator();

                for (idx, label, _) in &filtered_conditions {
                    let is_selected = selected == Some(*idx);
                    if left_ui.selectable_label(is_selected, label).clicked() {
                        new_selection = Some(*idx);
                    }
                }

                if filtered_conditions.is_empty() {
                    left_ui.label("No conditions found");
                }
            },
            |right_ui| {
                // Right panel: Detail view
                if let Some(idx) = selected {
                    if let Some((_, _, condition)) =
                        filtered_conditions.iter().find(|(i, _, _)| *i == idx)
                    {
                        right_ui.heading(&condition.name);
                        right_ui.separator();

                        // Use shared ActionButtons component
                        let action = ActionButtons::new().enabled(true).show(right_ui);
                        if action != ItemAction::None {
                            action_requested = Some(action);
                        }

                        right_ui.separator();

                        // Show spell references
                        if !selected_spell_refs.is_empty() {
                            right_ui.colored_label(
                                egui::Color32::YELLOW,
                                format!("Used by {} spell(s):", selected_spell_refs.len()),
                            );
                            for s in &selected_spell_refs {
                                right_ui.horizontal(|ui| {
                                    ui.label(format!("- {}", s));
                                    if ui
                                        .small_button("📋")
                                        .on_hover_text("Copy spell name")
                                        .clicked()
                                    {
                                        ui.ctx().copy_text(s.clone());
                                    }
                                    if ui
                                        .small_button("→")
                                        .on_hover_text("Jump to spell in Spells Editor")
                                        .clicked()
                                    {
                                        navigate_spell = Some(s.clone());
                                    }
                                });
                            }
                            right_ui.separator();
                        }

                        // Show condition details
                        Self::show_preview_static(
                            right_ui,
                            condition,
                            show_preview,
                            preview_magnitude,
                        );
                    } else {
                        right_ui.vertical_centered(|ui| {
                            ui.add_space(100.0);
                            ui.label("Select a condition to view details");
                        });
                    }
                } else {
                    right_ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("Select a condition to view details");
                    });
                }
            },
        );

        // Apply selection change after closures
        self.selected_condition_idx = new_selection;

        // Handle navigation request
        if navigate_spell.is_some() {
            self.navigate_to_spell = navigate_spell;
        }

        // Handle action button clicks after closures
        if let Some(action) = action_requested {
            match action {
                ItemAction::Edit => {
                    if let Some(idx) = self.selected_condition_idx {
                        if idx < conditions.len() {
                            self.mode = ConditionsEditorMode::Edit;
                            self.edit_buffer = Some(conditions[idx].clone());
                            self.editing_original_id = Some(conditions[idx].id.clone());
                        }
                    }
                }
                ItemAction::Delete => {
                    if let Some(idx) = self.selected_condition_idx {
                        if idx < conditions.len() {
                            self.selected_for_delete = Some(conditions[idx].id.clone());
                            self.delete_confirmation_open = true;
                        }
                    }
                }
                ItemAction::Duplicate => {
                    if let Some(idx) = self.selected_condition_idx {
                        if idx < conditions.len() {
                            let mut dup = conditions[idx].clone();
                            let base = dup.id.clone();
                            let mut suffix = 1;
                            while conditions.iter().any(|c| c.id == dup.id) {
                                dup.id = format!("{}_copy{}", base, suffix);
                                suffix += 1;
                            }
                            dup.name = format!("{} (Copy)", dup.name);
                            conditions.push(dup);
                            self.save_conditions(
                                conditions,
                                campaign_dir,
                                conditions_file,
                                unsaved_changes,
                                status_message,
                            );
                        }
                    }
                }
                ItemAction::Export => {
                    if let Some(idx) = self.selected_condition_idx {
                        if idx < conditions.len() {
                            if let Ok(ron_str) = ron::ser::to_string_pretty(
                                &conditions[idx],
                                ron::ser::PrettyConfig::default(),
                            ) {
                                self.import_export_buffer = ron_str;
                                self.show_import_dialog = true;
                                *status_message =
                                    "Condition exported to clipboard dialog".to_string();
                            } else {
                                *status_message = "Failed to export condition".to_string();
                            }
                        }
                    }
                }
                ItemAction::None => {}
            }
        }
    }

    /// Static preview method that doesn't require self
    fn show_preview_static(
        ui: &mut egui::Ui,
        condition: &ConditionDefinition,
        show_preview: bool,
        _preview_magnitude: f32,
    ) {
        let panel_height = crate::ui_helpers::compute_panel_height(ui, DEFAULT_PANEL_MIN_HEIGHT);

        egui::ScrollArea::vertical()
            .max_height(panel_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.heading("Basic Info");
                    ui.label(format!("ID: {}", condition.id));
                    ui.label(format!("Description: {}", condition.description));

                    ui.separator();

                    ui.label("Duration:");
                    match &condition.default_duration {
                        ConditionDuration::Instant => ui.label("Instant"),
                        ConditionDuration::Rounds(n) => ui.label(format!("{} rounds", n)),
                        ConditionDuration::Minutes(n) => ui.label(format!("{} minutes", n)),
                        ConditionDuration::Permanent => ui.label("Permanent"),
                    };

                    if let Some(icon) = &condition.icon_id {
                        ui.label(format!("Icon: {}", icon));
                    }
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Effects");
                    if condition.effects.is_empty() {
                        ui.label("No effects defined");
                    } else {
                        for (idx, effect) in condition.effects.iter().enumerate() {
                            ui.label(format!(
                                "Effect #{}: {}",
                                idx + 1,
                                render_condition_effect_summary(effect)
                            ));
                        }
                    }

                    if show_preview && !condition.effects.is_empty() {
                        ui.separator();
                        ui.label("Preview (with magnitude):");
                        for (idx, effect) in condition.effects.iter().enumerate() {
                            ui.label(format!(
                                "Effect #{}: {}",
                                idx + 1,
                                render_condition_effect_preview(effect, _preview_magnitude)
                            ));
                        }
                    }
                });
            });
    }

    #[allow(clippy::too_many_arguments)]
    fn show_form(
        &mut self,
        ui: &mut egui::Ui,
        conditions: &mut Vec<ConditionDefinition>,
        spells: &mut [Spell],
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        conditions_file: &str,
    ) {
        let is_edit = self.mode == ConditionsEditorMode::Edit;
        let title = if is_edit {
            "Edit Condition"
        } else {
            "New Condition"
        };

        ui.heading(title);
        ui.separator();

        if let Some(new_cond) = &mut self.edit_buffer {
            let mut should_save = false;
            let mut should_cancel = false;

            // If we're editing an existing condition, warn about spells that reference it
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
                            if ui
                                .small_button("📋")
                                .on_hover_text("Copy spell name")
                                .clicked()
                            {
                                ui.ctx().copy_text(s.clone());
                                *status_message = format!("Copied spell name to clipboard: {}", s);
                            }
                            if ui
                                .small_button("→")
                                .on_hover_text("Jump to spell in Spells Editor")
                                .clicked()
                            {
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

                    // Default duration selector
                    ui.label("Default Duration:")
                        .on_hover_text("How long the condition lasts by default");
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("condition_default_duration")
                            .selected_text(match &new_cond.default_duration {
                                ConditionDuration::Instant => "Instant".to_owned(),
                                ConditionDuration::Rounds(n) => format!("Rounds({})", n),
                                ConditionDuration::Minutes(n) => format!("Minutes({})", n),
                                ConditionDuration::Permanent => "Permanent".to_owned(),
                            })
                            .show_ui(ui, |ui| {
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
                                    new_cond.default_duration = ConditionDuration::Instant;
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
                                    new_cond.default_duration = ConditionDuration::Permanent;
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
                                    if !matches!(
                                        new_cond.default_duration,
                                        ConditionDuration::Rounds(_)
                                    ) {
                                        new_cond.default_duration = ConditionDuration::Rounds(1);
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
                                    if !matches!(
                                        new_cond.default_duration,
                                        ConditionDuration::Minutes(_)
                                    ) {
                                        new_cond.default_duration = ConditionDuration::Minutes(1);
                                    }
                                }
                            });
                        // If Rounds or Minutes selected, show a numeric editor
                        match &mut new_cond.default_duration {
                            ConditionDuration::Rounds(n) => {
                                ui.add(egui::DragValue::new(n));
                            }
                            ConditionDuration::Minutes(n) => {
                                ui.add(egui::DragValue::new(n));
                            }
                            _ => {}
                        }
                    });
                    ui.end_row();

                    // Icon ID field (optional)
                    ui.label("Icon ID:")
                        .on_hover_text("Optional icon identifier for UI display");
                    ui.horizontal(|ui| {
                        let mut icon_buf = new_cond.icon_id.clone().unwrap_or_default();
                        let text = ui.text_edit_singleline(&mut icon_buf);
                        if text.changed() {
                            new_cond.icon_id = if icon_buf.trim().is_empty() {
                                None
                            } else {
                                Some(icon_buf)
                            };
                        }
                        if ui.button("Clear").clicked() {
                            new_cond.icon_id = None;
                        }
                    });
                    ui.end_row();
                });

            // Basic ID uniqueness check
            let id_in_use = conditions.iter().any(|c| {
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

            // Effects list & editing
            ui.horizontal(|ui| {
                ui.label("Effects:");
                if !new_cond.effects.is_empty() {
                    if ui
                        .small_button("🗑️ Clear All")
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

            // Preview for current edit buffer
            if self.show_preview && !new_cond.effects.is_empty() {
                ui.separator();
                ui.label("Preview:")
                    .on_hover_text("Preview shows approximate effect values scaled by magnitude.");
                ui.horizontal(|ui| {
                    ui.label("Magnitude:");
                    ui.add(egui::Slider::new(&mut self.preview_magnitude, 0.1..=3.0).text("x"));
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

            // Add new effect button
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
                            let effect_type = Some(match &eff {
                                ConditionEffect::AttributeModifier { .. } => {
                                    "AttributeModifier".to_string()
                                }
                                ConditionEffect::StatusEffect(_) => "StatusEffect".to_string(),
                                ConditionEffect::DamageOverTime { .. } => {
                                    "DamageOverTime".to_string()
                                }
                                ConditionEffect::HealOverTime { .. } => "HealOverTime".to_string(),
                            });
                            let mut buf = EffectEditBuffer {
                                editing_index: Some(idx),
                                effect_type,
                                ..Default::default()
                            };

                            match eff {
                                ConditionEffect::AttributeModifier { attribute, value } => {
                                    buf.attribute = attribute;
                                    buf.attribute_value = value;
                                }
                                ConditionEffect::StatusEffect(tag) => {
                                    buf.status_tag = tag;
                                }
                                ConditionEffect::DamageOverTime { damage, element } => {
                                    buf.dice = damage;
                                    buf.element = element;
                                }
                                ConditionEffect::HealOverTime { amount } => {
                                    buf.dice = amount;
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
                            .selectable_label(effect_type == "StatusEffect", "StatusEffect")
                            .clicked()
                        {
                            effect_type = "StatusEffect".to_string();
                        }
                        if ui
                            .selectable_label(effect_type == "DamageOverTime", "DamageOverTime")
                            .clicked()
                        {
                            effect_type = "DamageOverTime".to_string();
                        }
                        if ui
                            .selectable_label(effect_type == "HealOverTime", "HealOverTime")
                            .clicked()
                        {
                            effect_type = "HealOverTime".to_string();
                        }
                    });
                buf.effect_type = Some(effect_type.clone());

                match effect_type.as_str() {
                    "AttributeModifier" => {
                        ui.horizontal(|ui| {
                            ui.label("Attribute:");
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
                                            .selectable_value(&mut buf.attribute, a.to_string(), a)
                                            .clicked()
                                        {}
                                    }
                                    if ui
                                        .selectable_label(buf.attribute == "custom", "Custom")
                                        .clicked()
                                    {
                                        buf.attribute = "custom".to_string();
                                    }
                                });
                            if buf.attribute == "custom" {
                                ui.text_edit_singleline(&mut buf.attribute);
                            }
                            ui.label("Value:");
                            ui.add(egui::DragValue::new(&mut buf.attribute_value).speed(1.0));
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
                            ui.add(egui::DragValue::new(&mut buf.dice.bonus).speed(1.0));
                            ui.label("Element:");
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
                                            .selectable_value(&mut buf.element, e.to_string(), e)
                                            .clicked()
                                        {}
                                    }
                                    if ui
                                        .selectable_label(buf.element == "custom", "Custom")
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
                            ui.add(egui::DragValue::new(&mut buf.dice.bonus).speed(1.0));
                        });
                    }
                    _ => {}
                }

                ui.horizontal(|ui| {
                    let effect_validation = validate_effect_edit_buffer(&buf);
                    if let Err(ref validation_error) = effect_validation {
                        ui.colored_label(egui::Color32::RED, validation_error);
                        ui.add_enabled(false, egui::Button::new("💾 Save Effect"));
                    } else if ui.button("💾 Save Effect").clicked() {
                        let new_effect = match buf.effect_type.as_deref() {
                            Some("AttributeModifier") => ConditionEffect::AttributeModifier {
                                attribute: buf.attribute.clone(),
                                value: buf.attribute_value,
                            },
                            Some("StatusEffect") => {
                                ConditionEffect::StatusEffect(buf.status_tag.clone())
                            }
                            Some("DamageOverTime") => ConditionEffect::DamageOverTime {
                                damage: buf.dice,
                                element: buf.element.clone(),
                            },
                            Some("HealOverTime") => {
                                ConditionEffect::HealOverTime { amount: buf.dice }
                            }
                            _ => ConditionEffect::StatusEffect(buf.status_tag.clone()),
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

                        keep_buf = false;
                    }
                    if ui.button("❌ Cancel").clicked() {
                        keep_buf = false;
                    }
                });

                if keep_buf {
                    self.effect_edit_buffer = Some(buf);
                } else {
                    self.effect_edit_buffer = None;
                }
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("⬅ Back to List").clicked() {
                    should_cancel = true;
                }

                if ui.button("💾 Save").clicked() {
                    should_save = true;
                }
                if ui.button("❌ Cancel").clicked() {
                    should_cancel = true;
                }
            });

            if should_save {
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
                        // Find the index of the saved condition
                        self.selected_condition_idx =
                            conditions.iter().position(|c| c.id == new_cond.id);
                        self.edit_buffer = None;
                        self.editing_original_id = None;
                        self.mode = ConditionsEditorMode::List;
                        self.save_conditions(
                            conditions,
                            campaign_dir,
                            conditions_file,
                            unsaved_changes,
                            status_message,
                        );
                    }
                    Err(e) => {
                        *status_message = format!("Failed to save condition: {}", e);
                    }
                }
            }

            if should_cancel {
                self.edit_buffer = None;
                self.editing_original_id = None;
                self.mode = ConditionsEditorMode::List;
            }
        }
    }

    fn show_import_dialog_window(
        &mut self,
        ctx: &egui::Context,
        conditions: &mut Vec<ConditionDefinition>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        conditions_file: &str,
    ) {
        let mut open = self.show_import_dialog;
        egui::Window::new("Import/Export Conditions")
            .open(&mut open)
            .resizable(true)
            .default_width(600.0)
            .show(ctx, |ui| {
                ui.label("Paste RON data below to import, or copy from here to export:");
                ui.add(
                    egui::TextEdit::multiline(&mut self.import_export_buffer)
                        .desired_width(f32::INFINITY)
                        .desired_rows(15)
                        .font(egui::TextStyle::Monospace),
                );

                ui.horizontal(|ui| {
                    if ui.button("📋 Copy to Clipboard").clicked() {
                        ui.ctx().copy_text(self.import_export_buffer.clone());
                        *status_message = "Copied to clipboard".to_string();
                    }

                    if ui.button("📥 Import").clicked() {
                        match ron::from_str::<Vec<ConditionDefinition>>(&self.import_export_buffer)
                        {
                            Ok(imported) => {
                                if self.file_load_merge_mode {
                                    for cond in imported {
                                        if let Some(existing) =
                                            conditions.iter_mut().find(|c| c.id == cond.id)
                                        {
                                            *existing = cond;
                                        } else {
                                            conditions.push(cond);
                                        }
                                    }
                                } else {
                                    *conditions = imported;
                                }
                                *unsaved_changes = true;
                                *status_message = "Conditions imported successfully".to_string();
                                self.save_conditions(
                                    conditions,
                                    campaign_dir,
                                    conditions_file,
                                    unsaved_changes,
                                    status_message,
                                );
                                self.show_import_dialog = false;
                            }
                            Err(e) => {
                                // Try importing a single condition
                                match ron::from_str::<ConditionDefinition>(
                                    &self.import_export_buffer,
                                ) {
                                    Ok(cond) => {
                                        if let Some(existing) =
                                            conditions.iter_mut().find(|c| c.id == cond.id)
                                        {
                                            *existing = cond;
                                        } else {
                                            conditions.push(cond);
                                        }
                                        *unsaved_changes = true;
                                        *status_message =
                                            "Single condition imported successfully".to_string();
                                        self.save_conditions(
                                            conditions,
                                            campaign_dir,
                                            conditions_file,
                                            unsaved_changes,
                                            status_message,
                                        );
                                        self.show_import_dialog = false;
                                    }
                                    Err(_) => {
                                        *status_message = format!("Failed to parse RON: {}", e);
                                    }
                                }
                            }
                        }
                    }

                    if ui.button("❌ Close").clicked() {
                        self.show_import_dialog = false;
                    }
                });
            });
        self.show_import_dialog = open;
    }

    #[allow(clippy::too_many_arguments)]
    fn show_delete_confirmation(
        &mut self,
        ctx: &egui::Context,
        conditions: &mut Vec<ConditionDefinition>,
        spells: &mut [Spell],
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        conditions_file: &str,
    ) {
        let mut open = self.delete_confirmation_open;
        egui::Window::new("Delete Condition")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
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
                                if ui
                                    .small_button("📋")
                                    .on_hover_text("Copy spell name")
                                    .clicked()
                                {
                                    ui.ctx().copy_text(s.clone());
                                    *status_message =
                                        format!("Copied spell name to clipboard: {}", s);
                                }
                                if ui
                                    .small_button("→")
                                    .on_hover_text("Jump to spell in Spells Editor")
                                    .clicked()
                                {
                                    self.navigate_to_spell = Some(s.clone());
                                }
                            });
                        }
                        ui.checkbox(
                            &mut self.remove_refs_on_delete,
                            "Remove references from spells when deleting",
                        )
                        .on_hover_text(
                            "If checked, spells that use this condition will have the reference automatically removed",
                        );
                    }
                }

                ui.horizontal(|ui| {
                    if ui.button("Yes, delete").clicked() {
                        if let Some(del_id) = &self.selected_for_delete {
                            if let Some(pos) = conditions.iter().position(|c| &c.id == del_id) {
                                conditions.remove(pos);

                                // Combine status messages so we only assign once (avoids unused assignment warning)
                                let message = if self.remove_refs_on_delete {
                                    let cnt = remove_condition_references_from_spells(spells, del_id);
                                    format!("Removed references from {} spell(s). Condition deleted.", cnt)
                                } else {
                                    "Condition deleted".to_string()
                                };

                                *unsaved_changes = true;
                                *status_message = message;
                                self.selected_condition_idx = None;
                                self.save_conditions(
                                    conditions,
                                    campaign_dir,
                                    conditions_file,
                                    unsaved_changes,
                                    status_message,
                                );
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
            });
        self.delete_confirmation_open = open;
    }

    fn save_conditions(
        &self,
        conditions: &Vec<ConditionDefinition>,
        campaign_dir: Option<&PathBuf>,
        conditions_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        if let Some(dir) = campaign_dir {
            let path = dir.join(conditions_file);
            match ron::ser::to_string_pretty(conditions, Default::default()) {
                Ok(contents) => match fs::write(&path, contents) {
                    Ok(_) => {
                        *unsaved_changes = false;
                        *status_message = format!("Saved conditions to: {}", path.display());
                    }
                    Err(e) => {
                        *status_message = format!("Failed to save conditions: {}", e);
                    }
                },
                Err(e) => {
                    *status_message = format!("Failed to serialize conditions: {}", e);
                }
            }
        } else {
            *status_message = "No campaign directory set".to_string();
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Apply condition edits to the conditions list.
///
/// This function handles both creating a new condition and updating an existing one.
/// If `original_id` is `Some`, it updates the condition with that ID. Otherwise,
/// it adds a new condition.
///
/// # Arguments
///
/// * `conditions` - The list of conditions to modify
/// * `original_id` - The original ID of the condition being edited (if editing)
/// * `new_cond` - The new condition data
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error message on failure.
pub(crate) fn apply_condition_edits(
    conditions: &mut Vec<ConditionDefinition>,
    original_id: Option<&str>,
    new_cond: &ConditionDefinition,
) -> Result<(), String> {
    // Validate the condition
    if new_cond.id.trim().is_empty() {
        return Err("ID cannot be empty".to_string());
    }
    if new_cond.name.trim().is_empty() {
        return Err("Name cannot be empty".to_string());
    }

    // Validate each effect
    for effect in &new_cond.effects {
        match effect {
            ConditionEffect::AttributeModifier { attribute, value } => {
                if attribute.trim().is_empty() {
                    return Err("Attribute modifier must have a non-empty attribute".to_string());
                }
                // Check value is in reasonable range using defined constants
                if *value < ATTRIBUTE_MODIFIER_MIN || *value > ATTRIBUTE_MODIFIER_MAX {
                    return Err(format!(
                        "Attribute modifier value {} is out of range ({} to {})",
                        value, ATTRIBUTE_MODIFIER_MIN, ATTRIBUTE_MODIFIER_MAX
                    ));
                }
            }
            ConditionEffect::StatusEffect(tag) => {
                if tag.trim().is_empty() {
                    return Err("Status effect must have a non-empty tag".to_string());
                }
            }
            ConditionEffect::DamageOverTime { damage, element } => {
                if damage.count < 1 {
                    return Err("Damage over time dice count must be >= 1".to_string());
                }
                if damage.sides < 2 {
                    return Err("Damage over time dice sides must be >= 2".to_string());
                }
                if element.trim().is_empty() {
                    return Err("Damage over time must have a non-empty element".to_string());
                }
            }
            ConditionEffect::HealOverTime { amount } => {
                if amount.count < 1 {
                    return Err("Heal over time dice count must be >= 1".to_string());
                }
                if amount.sides < 2 {
                    return Err("Heal over time dice sides must be >= 2".to_string());
                }
            }
        }
    }

    // Check for ID collision (excluding the original)
    let id_collision = conditions.iter().any(|c| {
        if let Some(orig) = original_id {
            if c.id == orig {
                return false;
            }
        }
        c.id == new_cond.id
    });

    if id_collision {
        return Err("ID already in use".to_string());
    }

    if let Some(orig_id) = original_id {
        // Update existing condition
        if let Some(existing) = conditions.iter_mut().find(|c| c.id == orig_id) {
            *existing = new_cond.clone();
        } else {
            return Err("Original condition not found".to_string());
        }
    } else {
        // Add new condition
        conditions.push(new_cond.clone());
    }

    Ok(())
}

/// Render a summary of a condition effect.
pub(crate) fn render_condition_effect_summary(effect: &ConditionEffect) -> String {
    match effect {
        ConditionEffect::AttributeModifier { attribute, value } => {
            let sign = if *value >= 0 { "+" } else { "" };
            format!("Attribute: {} {}{}", attribute, sign, value)
        }
        ConditionEffect::StatusEffect(tag) => {
            format!("Status: {}", tag)
        }
        ConditionEffect::DamageOverTime { damage, element } => {
            format!(
                "DOT: {}d{}+{} {}",
                damage.count, damage.sides, damage.bonus, element
            )
        }
        ConditionEffect::HealOverTime { amount } => {
            format!("HOT: {}d{}+{}", amount.count, amount.sides, amount.bonus)
        }
    }
}

/// Computes a preview string for the given effect adjusted by magnitude.
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

/// Validate the UI-side effect edit buffer.
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

// ===== Effect Helper Functions (for tests and external use) =====

/// Add an effect to a condition's effects list.
pub fn add_effect_to_condition(
    condition: &mut antares::domain::conditions::ConditionDefinition,
    effect: antares::domain::conditions::ConditionEffect,
) {
    condition.effects.push(effect);
}

/// Delete an effect from a condition's effects list by index.
/// Returns an error if the index is out of bounds.
pub fn delete_effect_from_condition(
    condition: &mut antares::domain::conditions::ConditionDefinition,
    index: usize,
) -> Result<(), String> {
    if index >= condition.effects.len() {
        return Err(format!(
            "Index {} out of bounds (effects count: {})",
            index,
            condition.effects.len()
        ));
    }
    condition.effects.remove(index);
    Ok(())
}

/// Duplicate an effect in a condition's effects list.
/// The duplicated effect is inserted right after the original.
/// Returns an error if the index is out of bounds.
pub fn duplicate_effect_in_condition(
    condition: &mut antares::domain::conditions::ConditionDefinition,
    index: usize,
) -> Result<(), String> {
    if index >= condition.effects.len() {
        return Err(format!(
            "Index {} out of bounds (effects count: {})",
            index,
            condition.effects.len()
        ));
    }
    let effect = condition.effects[index].clone();
    condition.effects.insert(index + 1, effect);
    Ok(())
}

/// Move an effect in a condition's effects list by a relative offset.
/// Positive offset moves down, negative offset moves up.
/// Returns an error if the index or resulting position is out of bounds.
pub fn move_effect_in_condition(
    condition: &mut antares::domain::conditions::ConditionDefinition,
    index: usize,
    offset: i32,
) -> Result<(), String> {
    if index >= condition.effects.len() {
        return Err(format!(
            "Index {} out of bounds (effects count: {})",
            index,
            condition.effects.len()
        ));
    }
    let new_index = (index as i32 + offset) as usize;
    if new_index >= condition.effects.len() {
        return Err(format!(
            "New index {} out of bounds (effects count: {})",
            new_index,
            condition.effects.len()
        ));
    }
    let effect = condition.effects.remove(index);
    condition.effects.insert(new_index, effect);
    Ok(())
}

/// Update an effect at a given index with a new effect.
/// Returns an error if the index is out of bounds.
pub fn update_effect_in_condition(
    condition: &mut antares::domain::conditions::ConditionDefinition,
    index: usize,
    new_effect: antares::domain::conditions::ConditionEffect,
) -> Result<(), String> {
    if index >= condition.effects.len() {
        return Err(format!(
            "Index {} out of bounds (effects count: {})",
            index,
            condition.effects.len()
        ));
    }
    condition.effects[index] = new_effect;
    Ok(())
}

/// Returns a vector of spell names that reference condition_id.
pub(crate) fn spells_referencing_condition(spells: &[Spell], condition_id: &str) -> Vec<String> {
    let mut result = Vec::new();
    for spell in spells.iter() {
        if spell.applied_conditions.iter().any(|c| c == condition_id) {
            result.push(spell.name.clone());
        }
    }
    result
}

/// Remove references of condition_id from spells and return the number of spells modified.
pub(crate) fn remove_condition_references_from_spells(
    spells: &mut [Spell],
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

/// Render the conditions editor (compatibility wrapper).
pub fn render_conditions_editor(
    ui: &mut egui::Ui,
    state: &mut ConditionsEditorState,
    conditions: &mut Vec<ConditionDefinition>,
) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ConditionsEditorState Tests
    // =========================================================================

    #[test]
    fn test_conditions_editor_state_new() {
        let state = ConditionsEditorState::new();
        assert_eq!(state.mode, ConditionsEditorMode::List);
        assert!(state.search_filter.is_empty());
        assert!(state.selected_condition_idx.is_none());
        assert!(!state.show_import_dialog);
        assert!(state.import_export_buffer.is_empty());
        assert!(state.show_preview); // Default is true
    }

    #[test]
    fn test_conditions_editor_state_default() {
        let state = ConditionsEditorState::default();
        assert_eq!(state.mode, ConditionsEditorMode::List);
        assert_eq!(state.filter_effect_type, EffectTypeFilter::All);
        assert_eq!(state.sort_order, ConditionSortOrder::NameAsc);
        assert!(!state.show_statistics);
        assert!(state.navigate_to_spell.is_none());
    }

    #[test]
    fn test_default_condition_creation() {
        let condition = ConditionsEditorState::default_condition();
        assert_eq!(condition.id, "new_condition");
        assert_eq!(condition.name, "New Condition");
        assert!(condition.description.is_empty());
        assert!(condition.effects.is_empty());
        assert_eq!(condition.default_duration, ConditionDuration::Rounds(3));
        assert!(condition.icon_id.is_none());
    }

    // =========================================================================
    // ConditionsEditorMode Tests
    // =========================================================================

    #[test]
    fn test_conditions_editor_mode_variants() {
        assert_eq!(ConditionsEditorMode::List, ConditionsEditorMode::List);
        assert_eq!(ConditionsEditorMode::Add, ConditionsEditorMode::Add);
        assert_eq!(ConditionsEditorMode::Edit, ConditionsEditorMode::Edit);
        assert_ne!(ConditionsEditorMode::List, ConditionsEditorMode::Add);
    }

    // =========================================================================
    // EffectTypeFilter Tests
    // =========================================================================

    #[test]
    fn test_effect_type_filter_as_str() {
        assert_eq!(EffectTypeFilter::All.as_str(), "All");
        assert_eq!(EffectTypeFilter::AttributeModifier.as_str(), "Attribute");
        assert_eq!(EffectTypeFilter::StatusEffect.as_str(), "Status");
        assert_eq!(EffectTypeFilter::DamageOverTime.as_str(), "DOT");
        assert_eq!(EffectTypeFilter::HealOverTime.as_str(), "HOT");
    }

    #[test]
    fn test_effect_type_filter_all() {
        let filters = EffectTypeFilter::all();
        assert_eq!(filters.len(), 5);
        assert!(filters.contains(&EffectTypeFilter::All));
        assert!(filters.contains(&EffectTypeFilter::AttributeModifier));
        assert!(filters.contains(&EffectTypeFilter::StatusEffect));
        assert!(filters.contains(&EffectTypeFilter::DamageOverTime));
        assert!(filters.contains(&EffectTypeFilter::HealOverTime));
    }

    #[test]
    fn test_effect_type_filter_matches_all() {
        let condition = ConditionsEditorState::default_condition();
        assert!(EffectTypeFilter::All.matches(&condition));
    }

    // =========================================================================
    // ConditionSortOrder Tests
    // =========================================================================

    #[test]
    fn test_condition_sort_order_as_str() {
        assert_eq!(ConditionSortOrder::NameAsc.as_str(), "Name (A-Z)");
        assert_eq!(ConditionSortOrder::NameDesc.as_str(), "Name (Z-A)");
        assert_eq!(ConditionSortOrder::IdAsc.as_str(), "ID (A-Z)");
        assert_eq!(ConditionSortOrder::IdDesc.as_str(), "ID (Z-A)");
        assert_eq!(ConditionSortOrder::EffectCount.as_str(), "Effect Count");
    }

    // =========================================================================
    // EffectEditBuffer Tests
    // =========================================================================

    #[test]
    fn test_effect_edit_buffer_default() {
        let buffer = EffectEditBuffer::default();
        assert!(buffer.effect_type.is_none());
        assert!(buffer.editing_index.is_none());
        assert_eq!(buffer.attribute, "might"); // Default is "might"
        assert_eq!(buffer.attribute_value, 0);
        assert!(buffer.status_tag.is_empty());
        assert_eq!(buffer.element, "physical"); // Default is "physical"
    }

    // =========================================================================
    // Editor State Transitions Tests
    // =========================================================================

    #[test]
    fn test_editor_mode_transitions() {
        let mut state = ConditionsEditorState::new();
        assert_eq!(state.mode, ConditionsEditorMode::List);

        state.mode = ConditionsEditorMode::Add;
        assert_eq!(state.mode, ConditionsEditorMode::Add);

        state.mode = ConditionsEditorMode::Edit;
        assert_eq!(state.mode, ConditionsEditorMode::Edit);

        state.mode = ConditionsEditorMode::List;
        assert_eq!(state.mode, ConditionsEditorMode::List);
    }

    #[test]
    fn test_selected_condition_handling() {
        let mut state = ConditionsEditorState::new();
        assert!(state.selected_condition_idx.is_none());

        state.selected_condition_idx = Some(0);
        assert_eq!(state.selected_condition_idx, Some(0));

        state.selected_condition_idx = Some(5);
        assert_eq!(state.selected_condition_idx, Some(5));

        state.selected_condition_idx = None;
        assert!(state.selected_condition_idx.is_none());
    }

    #[test]
    fn test_filter_and_sort_changes() {
        let mut state = ConditionsEditorState::new();

        // Change filter
        state.filter_effect_type = EffectTypeFilter::DamageOverTime;
        assert_eq!(state.filter_effect_type, EffectTypeFilter::DamageOverTime);

        // Change sort order
        state.sort_order = ConditionSortOrder::IdDesc;
        assert_eq!(state.sort_order, ConditionSortOrder::IdDesc);
    }

    // =========================================================================
    // Condition Statistics Tests
    // =========================================================================

    #[test]
    fn test_compute_condition_statistics_empty() {
        let conditions: Vec<ConditionDefinition> = Vec::new();
        let stats = compute_condition_statistics(&conditions);
        assert_eq!(stats.total, 0);
        assert_eq!(stats.attribute_count, 0);
        assert_eq!(stats.status_count, 0);
        assert_eq!(stats.dot_count, 0);
        assert_eq!(stats.hot_count, 0);
        assert_eq!(stats.empty_count, 0);
        assert_eq!(stats.multi_effect_count, 0);
    }

    #[test]
    fn test_compute_condition_statistics_with_conditions() {
        let conditions = vec![
            ConditionDefinition {
                id: "empty".to_string(),
                name: "Empty".to_string(),
                description: String::new(),
                effects: vec![],
                default_duration: ConditionDuration::Permanent,
                icon_id: None,
            },
            ConditionDefinition {
                id: "strength_buff".to_string(),
                name: "Strength Buff".to_string(),
                description: String::new(),
                effects: vec![ConditionEffect::AttributeModifier {
                    attribute: "might".to_string(),
                    value: 5,
                }],
                default_duration: ConditionDuration::Permanent,
                icon_id: None,
            },
        ];
        let stats = compute_condition_statistics(&conditions);
        assert_eq!(stats.total, 2);
        assert_eq!(stats.attribute_count, 1);
        assert_eq!(stats.empty_count, 1);
    }

    // =========================================================================
    // Validation Tests
    // =========================================================================

    #[test]
    fn test_validate_effect_edit_buffer_attribute_modifier() {
        let buffer = EffectEditBuffer {
            effect_type: Some("AttributeModifier".to_string()),
            attribute: "might".to_string(),
            attribute_value: 5,
            ..Default::default()
        };

        let result = validate_effect_edit_buffer(&buffer);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_effect_edit_buffer_empty_attribute() {
        let buffer = EffectEditBuffer {
            effect_type: Some("AttributeModifier".to_string()),
            attribute: String::new(),
            attribute_value: 5,
            ..Default::default()
        };

        let result = validate_effect_edit_buffer(&buffer);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_effect_edit_buffer_status_effect() {
        let buffer = EffectEditBuffer {
            effect_type: Some("StatusEffect".to_string()),
            status_tag: "poisoned".to_string(),
            ..Default::default()
        };

        let result = validate_effect_edit_buffer(&buffer);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_effect_edit_buffer_empty_status() {
        let buffer = EffectEditBuffer {
            effect_type: Some("StatusEffect".to_string()),
            status_tag: String::new(),
            ..Default::default()
        };

        let result = validate_effect_edit_buffer(&buffer);
        assert!(result.is_err());
    }

    // =========================================================================
    // Effect Rendering Tests
    // =========================================================================

    #[test]
    fn test_render_condition_effect_summary_attribute() {
        let effect = ConditionEffect::AttributeModifier {
            attribute: "might".to_string(),
            value: 5,
        };
        let summary = render_condition_effect_summary(&effect);
        assert!(summary.contains("Attribute"));
        assert!(summary.contains("might"));
        assert!(summary.contains("+5"));
    }

    #[test]
    fn test_render_condition_effect_summary_negative() {
        let effect = ConditionEffect::AttributeModifier {
            attribute: "speed".to_string(),
            value: -3,
        };
        let summary = render_condition_effect_summary(&effect);
        assert!(summary.contains("speed"));
        assert!(summary.contains("-3"));
    }

    #[test]
    fn test_render_condition_effect_summary_status() {
        let effect = ConditionEffect::StatusEffect("poisoned".to_string());
        let summary = render_condition_effect_summary(&effect);
        assert!(summary.contains("Status"));
        assert!(summary.contains("poisoned"));
    }

    #[test]
    fn test_preview_toggle() {
        let mut state = ConditionsEditorState::new();
        assert!(state.show_preview); // Default is true

        state.show_preview = false;
        assert!(!state.show_preview);

        state.show_preview = true;
        assert!(state.show_preview);
    }

    #[test]
    fn test_statistics_toggle() {
        let mut state = ConditionsEditorState::new();
        assert!(!state.show_statistics);

        state.show_statistics = true;
        assert!(state.show_statistics);

        state.show_statistics = false;
        assert!(!state.show_statistics);
    }
}
