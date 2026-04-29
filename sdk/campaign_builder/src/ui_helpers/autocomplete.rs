// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Autocomplete widgets, entity selectors, candidate extraction, and the
//! `AutocompleteCandidateCache`.
//!
//! Also contains [`dispatch_list_action`] for uniform CRUD dispatch across editors.

use super::layout::{
    load_autocomplete_buffer, make_autocomplete_id, remove_autocomplete_buffer,
    store_autocomplete_buffer, ItemAction,
};
use antares::domain::items::Item;
use antares::domain::proficiency::{
    ProficiencyCategory, ProficiencyDatabase, ProficiencyDefinition,
};
use eframe::egui;
use egui_autocomplete::AutoCompleteTextEdit;
use std::path::PathBuf;

// =============================================================================
// Autocomplete Input Widget
// =============================================================================

/// Autocomplete text input with dropdown suggestions.
///
/// This widget wraps `egui_autocomplete::AutoCompleteTextEdit` to provide a
/// consistent interface with other UI helpers. It displays a text field with
/// a dropdown list of suggestions that filters as the user types (case-insensitive).
///
/// # Examples
///
/// ```no_run
/// use eframe::egui;
/// use campaign_builder::ui_helpers::AutocompleteInput;
///
/// fn example(ui: &mut egui::Ui) {
///     let candidates = vec!["Goblin".to_string(), "Orc".to_string(), "Dragon".to_string()];
///     let mut input = String::new();
///
///     AutocompleteInput::new("monster_select", &candidates)
///         .with_placeholder("Type monster name...")
///         .show(ui, &mut input);
/// }
/// ```
pub struct AutocompleteInput<'a> {
    /// Unique widget identifier salt
    pub(crate) _id_salt: &'a str,
    /// List of candidate suggestions
    pub(crate) candidates: &'a [String],
    /// Optional placeholder hint text
    pub(crate) placeholder: Option<&'a str>,
}

impl<'a> AutocompleteInput<'a> {
    /// Creates a new autocomplete input widget.
    ///
    /// # Arguments
    ///
    /// * `id_salt` - Unique identifier for this widget instance (used to distinguish multiple instances)
    /// * `candidates` - Slice of suggestion strings to display in dropdown
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use campaign_builder::ui_helpers::AutocompleteInput;
    ///
    /// fn example(ui: &mut egui::Ui) {
    ///     let candidates = vec!["Goblin".to_string(), "Orc".to_string()];
    ///     let mut text = String::new();
    ///     AutocompleteInput::new("my_autocomplete", &candidates).show(ui, &mut text);
    /// }
    /// ```
    pub fn new(id_salt: &'a str, candidates: &'a [String]) -> Self {
        Self {
            _id_salt: id_salt,
            candidates,
            placeholder: None,
        }
    }

    /// Sets the placeholder hint text (builder pattern).
    ///
    /// # Arguments
    ///
    /// * `placeholder` - Text to display when the input field is empty
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use campaign_builder::ui_helpers::AutocompleteInput;
    ///
    /// fn example(ui: &mut egui::Ui) {
    ///     let candidates = vec!["Goblin".to_string()];
    ///     let mut text = String::new();
    ///
    ///     AutocompleteInput::new("autocomplete", &candidates)
    ///         .with_placeholder("Start typing...")
    ///         .show(ui, &mut text);
    /// }
    /// ```
    pub fn with_placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    /// Renders the autocomplete widget.
    ///
    /// Displays a text input field with a dropdown list of filtered suggestions.
    /// The dropdown filters candidates case-insensitively as the user types.
    /// Clicking a suggestion or pressing Enter on a highlighted suggestion
    /// updates the text buffer.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `text` - Mutable reference to the text buffer to edit
    ///
    /// # Returns
    ///
    /// Returns the `egui::Response` from the text input widget, allowing
    /// for response chaining and inspection.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use campaign_builder::ui_helpers::AutocompleteInput;
    ///
    /// fn example(ui: &mut egui::Ui) {
    ///     let candidates = vec![
    ///         "Goblin".to_string(),
    ///         "Orc".to_string(),
    ///         "Dragon".to_string(),
    ///         "Skeleton".to_string(),
    ///     ];
    ///     let mut monster_name = String::new();
    ///
    ///     let response = AutocompleteInput::new("monster_input", &candidates)
    ///         .with_placeholder("Select a monster...")
    ///         .show(ui, &mut monster_name);
    ///
    ///     if response.changed() {
    ///         println!("Monster name changed to: {}", monster_name);
    ///     }
    /// }
    /// ```
    pub fn show(self, ui: &mut egui::Ui, text: &mut String) -> egui::Response {
        // Create the autocomplete text edit widget with new API
        let mut autocomplete = AutoCompleteTextEdit::new(text, self.candidates)
            .highlight_matches(true)
            .max_suggestions(10);

        // Add placeholder if provided
        if let Some(placeholder_text) = self.placeholder {
            let placeholder_owned = placeholder_text.to_string();
            autocomplete = autocomplete
                .set_text_edit_properties(move |text_edit| text_edit.hint_text(placeholder_owned));
        }

        // Show the widget and return the response
        ui.add(autocomplete)
    }
}

// =============================================================================
// Config Structs for Bundled Function Parameters
// =============================================================================

/// State bundle for [`dispatch_list_action`].
///
/// Groups the four output-state parameters so the function stays under the
/// Clippy `too_many_arguments` limit.
///
/// # Examples
///
/// ```
/// use campaign_builder::ui_helpers::DispatchActionState;
///
/// let mut buf = String::new();
/// let mut show = false;
/// let mut msg = String::new();
///
/// let mut state = DispatchActionState {
///     entity_label: "item",
///     import_export_buffer: &mut buf,
///     show_import_dialog: &mut show,
///     status_message: &mut msg,
/// };
/// assert_eq!(state.entity_label, "item");
/// ```
pub struct DispatchActionState<'a> {
    /// Human-readable entity label used in status messages (e.g. `"item"`, `"spell"`).
    pub entity_label: &'a str,
    /// Buffer holding RON text for import/export dialogs.
    pub import_export_buffer: &'a mut String,
    /// When set to `true`, the import/export dialog window will be shown.
    pub show_import_dialog: &'a mut bool,
    /// One-line status bar text updated by the action.
    pub status_message: &'a mut String,
}

/// Configuration bundle for [`autocomplete_entity_selector_generic`].
///
/// Groups the four non-data display parameters so the function stays under
/// the Clippy `too_many_arguments` limit.
///
/// # Examples
///
/// ```no_run
/// use campaign_builder::ui_helpers::AutocompleteSelectorConfig;
///
/// let cfg = AutocompleteSelectorConfig {
///     id_salt: "item_selector",
///     buffer_tag: "item",
///     label: "Item:",
///     placeholder: "Start typing...",
/// };
/// assert_eq!(cfg.buffer_tag, "item");
/// ```
pub struct AutocompleteSelectorConfig<'a> {
    /// Unique egui ID salt for the autocomplete widget.
    pub id_salt: &'a str,
    /// Short tag used as part of the egui memory key (e.g. `"item"`, `"quest"`).
    pub buffer_tag: &'a str,
    /// Text label shown to the left of the input (empty string = no label).
    pub label: &'a str,
    /// Placeholder text shown when the field is empty.
    pub placeholder: &'a str,
}

/// Configuration bundle for [`autocomplete_list_selector_generic`].
///
/// Groups the five string parameters so the function stays under the Clippy
/// `too_many_arguments` limit.
///
/// # Examples
///
/// ```no_run
/// use campaign_builder::ui_helpers::AutocompleteListSelectorConfig;
///
/// let cfg = AutocompleteListSelectorConfig {
///     id_salt: "items_list",
///     buffer_tag: "item_add",
///     label: "Items:",
///     add_label: "Add item:",
///     placeholder: "Start typing...",
/// };
/// assert_eq!(cfg.add_label, "Add item:");
/// ```
pub struct AutocompleteListSelectorConfig<'a> {
    /// Unique egui ID salt for the autocomplete widget.
    pub id_salt: &'a str,
    /// Short tag used as part of the egui memory key.
    pub buffer_tag: &'a str,
    /// Group label shown above the selector.
    pub label: &'a str,
    /// Label shown next to the "add" input field.
    pub add_label: &'a str,
    /// Placeholder text for the add input field.
    pub placeholder: &'a str,
}

/// Generic list-action dispatcher for standard editor CRUD operations.
///
/// Handles `Delete`, `Duplicate`, and `Export` (to the import-export buffer)
/// actions in a uniform way across data editors. The `Edit` action is
/// intentionally **not** handled here because it requires setting
/// editor-specific mode types and edit buffers; callers should handle it
/// themselves.
///
/// This function is used by `ItemsEditorState`, `SpellsEditorState`,
/// `MonstersEditorState`, `ConditionsEditorState`, `ProficienciesEditorState`,
/// and `DialogueEditorState` to consolidate their otherwise-identical action
/// dispatch code.
///
/// # Type Parameters
///
/// * `T` — Entity type. Must be `Clone + serde::Serialize`.
/// * `C` — Closure that prepares a duplicate entry. Called with a mutable
///   reference to the cloned entry and an immutable slice of the current
///   collection (before the new entry is pushed), so it can generate a
///   collision-free ID or updated name.
///
/// # Arguments
///
/// * `action` — The `ItemAction` to dispatch
/// * `data` — Mutable reference to the entity collection
/// * `selected_idx` — Current selection index; set to `None` after a
///   successful `Delete`
/// * `prepare_duplicate` — Called as `prepare_duplicate(new_entry, data)`
///   just before pushing the duplicate; should set the new entry's ID and
///   update its name (e.g. append `" (Copy)"`)
/// * `state` — Bundle of output-state parameters; see [`DispatchActionState`]
///
/// # Returns
///
/// `true` if the collection was mutated (i.e. `Delete` or `Duplicate`
/// happened), so the caller knows a save is needed.
///
/// # Examples
///
/// ```
/// use campaign_builder::ui_helpers::{dispatch_list_action, DispatchActionState, ItemAction};
///
/// #[derive(Clone, serde::Serialize, serde::Deserialize)]
/// struct Thing { id: u32, name: String }
///
/// let mut data = vec![
///     Thing { id: 1, name: "Alpha".to_string() },
/// ];
/// let mut selected: Option<usize> = Some(0);
/// let mut buf = String::new();
/// let mut show = false;
/// let mut msg = String::new();
///
/// let mut state = DispatchActionState {
///     entity_label: "thing",
///     import_export_buffer: &mut buf,
///     show_import_dialog: &mut show,
///     status_message: &mut msg,
/// };
/// let changed = dispatch_list_action(
///     ItemAction::Duplicate,
///     &mut data,
///     &mut selected,
///     |entry, all| {
///         entry.id = all.iter().map(|t| t.id).max().unwrap_or(0) + 1;
///         entry.name = format!("{} (Copy)", entry.name);
///     },
///     &mut state,
/// );
///
/// assert!(changed);
/// assert_eq!(data.len(), 2);
/// assert_eq!(data[1].name, "Alpha (Copy)");
/// ```
pub fn dispatch_list_action<T, C>(
    action: ItemAction,
    data: &mut Vec<T>,
    selected_idx: &mut Option<usize>,
    prepare_duplicate: C,
    state: &mut DispatchActionState<'_>,
) -> bool
where
    T: Clone + serde::Serialize,
    C: Fn(&mut T, &[T]),
{
    match action {
        ItemAction::Duplicate => {
            if let Some(idx) = *selected_idx {
                if idx < data.len() {
                    let mut new_entry = data[idx].clone();
                    prepare_duplicate(&mut new_entry, data);
                    data.push(new_entry);
                    return true;
                }
            }
        }
        ItemAction::Delete => {
            if let Some(idx) = *selected_idx {
                if idx < data.len() {
                    data.remove(idx);
                    *selected_idx = None;
                    return true;
                }
            }
        }
        ItemAction::Export => {
            if let Some(idx) = *selected_idx {
                if idx < data.len() {
                    match ron::ser::to_string_pretty(&data[idx], ron::ser::PrettyConfig::default())
                    {
                        Ok(ron_str) => {
                            *state.import_export_buffer = ron_str;
                            *state.show_import_dialog = true;
                            *state.status_message =
                                format!("{} exported to clipboard dialog", state.entity_label);
                        }
                        Err(e) => {
                            *state.status_message =
                                format!("Failed to export {}: {}", state.entity_label, e);
                        }
                    }
                }
            }
        }
        ItemAction::Edit | ItemAction::None => {}
    }
    false
}

// =============================================================================
// Entity Candidate Extraction for Autocomplete
// =============================================================================

/// Generic single-entity autocomplete selector.
///
/// Core implementation shared by all `autocomplete_*_selector` functions.
/// Entity-specific wrappers supply the candidate list, current display name,
/// and two closures that handle selection commit and clearing.
///
/// # Arguments
///
/// * `ui` — egui UI context
/// * `cfg` — display configuration bundle; see [`AutocompleteSelectorConfig`]
/// * `candidates` — list of display strings shown in the autocomplete dropdown
/// * `current_name` — display string for the currently selected entity (empty if none)
/// * `is_selected` — whether there is a current selection (controls ✖ clear button)
/// * `on_select` — called with the user's typed/selected text; should update the
///   backing field and return `true` if the value was valid and changed
/// * `on_clear` — called when the user clicks the ✖ clear button; should reset the
///   backing selection field
///
/// # Returns
///
/// `true` if the selection changed this frame.
pub fn autocomplete_entity_selector_generic(
    ui: &mut egui::Ui,
    cfg: &AutocompleteSelectorConfig<'_>,
    candidates: Vec<String>,
    current_name: String,
    is_selected: bool,
    mut on_select: impl FnMut(&str) -> bool,
    mut on_clear: impl FnMut(),
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        if !cfg.label.is_empty() {
            ui.label(cfg.label);
        }
        let buffer_id = make_autocomplete_id(ui, cfg.buffer_tag, cfg.id_salt);
        let mut text_buffer =
            load_autocomplete_buffer(ui.ctx(), buffer_id, || current_name.clone());

        let button_reserve = if is_selected {
            ui.spacing().interact_size.x * 2.0 + ui.spacing().item_spacing.x
        } else {
            0.0
        };
        let width = (ui.available_width() - button_reserve).max(120.0);

        let mut autocomplete = AutoCompleteTextEdit::new(&mut text_buffer, &candidates)
            .highlight_matches(true)
            .max_suggestions(10);
        if !cfg.placeholder.is_empty() {
            let placeholder_owned = cfg.placeholder.to_string();
            autocomplete = autocomplete
                .set_text_edit_properties(move |text_edit| text_edit.hint_text(placeholder_owned));
        }

        let response = ui.add_sized([width, ui.spacing().interact_size.y], autocomplete);

        if response.changed()
            && !text_buffer.is_empty()
            && text_buffer != current_name
            && on_select(&text_buffer)
        {
            changed = true;
        }

        let mut cleared = false;
        if is_selected
            && ui
                .small_button("✖")
                .on_hover_text("Clear selection")
                .clicked()
        {
            on_clear();
            remove_autocomplete_buffer(ui.ctx(), buffer_id);
            changed = true;
            cleared = true;
        }

        if !cleared {
            store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
        }
    });
    changed
}

/// Shows an autocomplete input for selecting an item by name.
///
/// Returns `true` if the selection changed (user selected an item from suggestions).
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display before the input
/// * `selected_item_id` - Mutable reference to the currently selected ItemId (0 means none)
/// * `items` - Slice of available items
///
/// # Returns
///
/// `true` if the user selected an item, `false` otherwise
///
/// # Examples
///
/// ```no_run
/// use eframe::egui;
/// use antares::domain::items::types::Item;
/// use antares::domain::types::ItemId;
/// use campaign_builder::ui_helpers::autocomplete_item_selector;
///
/// fn show_item_picker(ui: &mut egui::Ui, selected: &mut ItemId, items: &[Item]) {
///     if autocomplete_item_selector(ui, "weapon_picker", "Weapon:", selected, items) {
///         // User selected an item
///     }
/// }
/// ```
pub fn autocomplete_item_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_item_id: &mut antares::domain::types::ItemId,
    items: &[antares::domain::items::types::Item],
) -> bool {
    let candidates: Vec<String> = items.iter().map(|i| i.name.clone()).collect();
    let current_name = if *selected_item_id == 0 {
        String::new()
    } else {
        items
            .iter()
            .find(|i| i.id == *selected_item_id)
            .map(|i| i.name.clone())
            .unwrap_or_default()
    };
    // Use Cell so both on_select and on_clear can share mutation without conflicting borrows.
    let cell = std::cell::Cell::new(*selected_item_id);
    let is_selected = *selected_item_id != 0;
    let cfg = AutocompleteSelectorConfig {
        id_salt,
        buffer_tag: "item",
        label,
        placeholder: "Start typing item name...",
    };
    let changed = autocomplete_entity_selector_generic(
        ui,
        &cfg,
        candidates,
        current_name,
        is_selected,
        |text| {
            if let Some(item) = items.iter().find(|i| i.name == text) {
                cell.set(item.id);
                true
            } else {
                false
            }
        },
        || cell.set(0),
    );
    *selected_item_id = cell.get();
    changed
}

/// Shows an autocomplete input for selecting a quest by name.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display before the input
/// * `selected_quest_id_str` - Mutable reference to the selected quest ID as String
/// * `quests` - Slice of available quests
///
/// # Returns
///
/// `true` if the user selected a quest, `false` otherwise
///
/// # Examples
///
/// ```
/// use eframe::egui;
///
/// let mut quest_id_str = String::new();
/// let quests: Vec<antares::domain::quest::Quest> = Vec::new();
///
/// // In UI code:
/// // let changed = campaign_builder::ui_helpers::autocomplete_quest_selector(ui, "quest_sel", "Quest:", &mut quest_id_str, &quests);
/// ```
pub fn autocomplete_quest_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_quest_id_str: &mut String,
    quests: &[antares::domain::quest::Quest],
) -> bool {
    let candidates: Vec<String> = quests.iter().map(|q| q.name.clone()).collect();
    let current_name = if selected_quest_id_str.is_empty() {
        String::new()
    } else {
        selected_quest_id_str
            .parse::<antares::domain::quest::QuestId>()
            .ok()
            .and_then(|id| quests.iter().find(|q| q.id == id))
            .map(|q| q.name.clone())
            .unwrap_or_default()
    };
    // Use RefCell so both on_select and on_clear can share mutation without conflicting borrows.
    let is_selected = !selected_quest_id_str.is_empty();
    let cell = std::cell::RefCell::new(std::mem::take(selected_quest_id_str));
    let cfg = AutocompleteSelectorConfig {
        id_salt,
        buffer_tag: "quest",
        label,
        placeholder: "Start typing quest name...",
    };
    let changed = autocomplete_entity_selector_generic(
        ui,
        &cfg,
        candidates,
        current_name,
        is_selected,
        |text| {
            if let Some(quest) = quests.iter().find(|q| q.name == text) {
                *cell.borrow_mut() = quest.id.to_string();
                true
            } else {
                false
            }
        },
        || cell.borrow_mut().clear(),
    );
    *selected_quest_id_str = cell.into_inner();
    changed
}

/// Shows an autocomplete input for selecting a monster by name.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display before the input
/// * `selected_monster_name` - Mutable reference to the selected monster name
/// * `monsters` - Slice of available monsters
///
/// # Returns
///
/// `true` if the user selected a monster, `false` otherwise
pub fn autocomplete_monster_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_monster_name: &mut String,
    monsters: &[antares::domain::combat::database::MonsterDefinition],
) -> bool {
    let candidates: Vec<String> = extract_monster_candidates(monsters);
    let current_name = selected_monster_name.clone();
    // Use RefCell so both on_select and on_clear can share mutation without conflicting borrows.
    let is_selected = !selected_monster_name.is_empty();
    let cell = std::cell::RefCell::new(std::mem::take(selected_monster_name));
    let cfg = AutocompleteSelectorConfig {
        id_salt,
        buffer_tag: "monster",
        label,
        placeholder: "Start typing monster name...",
    };
    let changed = autocomplete_entity_selector_generic(
        ui,
        &cfg,
        candidates,
        current_name,
        is_selected,
        |text| {
            if monsters.iter().any(|m| m.name == text) {
                *cell.borrow_mut() = text.to_string();
                true
            } else {
                false
            }
        },
        || cell.borrow_mut().clear(),
    );
    *selected_monster_name = cell.into_inner();
    changed
}

/// Autocomplete selector for creature IDs.
///
/// Provides a text autocomplete for creature candidates (by name) discovered from the
/// `CreatureAssetManager`. The field stores the numeric creature ID as a `String` in
/// the buffer (consistent with other string-based ID fields like `portrait_id`). The
/// candidates list is `"id — name"` pairs so the user can type either the numeric ID
/// or the creature name to filter.
///
/// # Arguments
///
/// * `ui` - The egui UI
/// * `id_salt` - Salt used for the autocomplete widget ID (must be unique in the frame)
/// * `label` - Label shown to the left of the input field (pass `""` to omit)
/// * `selected_creature_id` - Mutable reference to the currently selected creature ID string
/// * `candidates` - Pre-built `(id, name)` pair list (built once per frame by the caller)
///
/// # Returns
///
/// Returns `true` if the selection changed.
///
/// # Examples
///
/// ```no_run
/// use eframe::egui;
/// use campaign_builder::ui_helpers::autocomplete_creature_selector;
///
/// fn example(ui: &mut egui::Ui, creatures: &[(u32, String)]) {
///     let mut creature_id_str = String::new();
///     autocomplete_creature_selector(ui, "npc_creature", "Creature ID:", &mut creature_id_str, creatures);
/// }
/// ```
pub fn autocomplete_creature_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_creature_id: &mut String,
    candidates: &[(u32, String)],
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        if !label.is_empty() {
            ui.label(label);
        }

        // Build display candidates: "id — name" so the user can type by either.
        let display_candidates: Vec<String> = candidates
            .iter()
            .map(|(id, name)| format!("{} — {}", id, name))
            .collect();

        let buffer_id = make_autocomplete_id(ui, "creature", id_salt);

        let current_value = selected_creature_id.clone();
        let mut text_buffer = load_autocomplete_buffer(ui.ctx(), buffer_id, || {
            // Initialise the buffer from the current ID: look up name for display.
            if current_value.is_empty() {
                String::new()
            } else if let Ok(id_num) = current_value.trim().parse::<u32>() {
                candidates
                    .iter()
                    .find(|(id, _)| *id == id_num)
                    .map(|(id, name)| format!("{} — {}", id, name))
                    .unwrap_or_else(|| current_value.clone())
            } else {
                current_value.clone()
            }
        });

        let response = AutocompleteInput::new(id_salt, &display_candidates)
            .with_placeholder("Type creature name or ID…")
            .show(ui, &mut text_buffer);

        // When the user picks an entry or finishes typing, extract just the numeric ID.
        if response.changed() && !text_buffer.is_empty() {
            // Accept entries from the candidate list ("id — name" format).
            if let Some(pos) = text_buffer.find(" — ") {
                let id_part = text_buffer[..pos].trim();
                if id_part.parse::<u32>().is_ok() && *selected_creature_id != id_part {
                    *selected_creature_id = id_part.to_string();
                    changed = true;
                }
            } else if text_buffer.trim().parse::<u32>().is_ok() {
                // User typed a raw numeric ID.
                let trimmed = text_buffer.trim().to_string();
                if *selected_creature_id != trimmed {
                    *selected_creature_id = trimmed;
                    changed = true;
                }
            }
        }

        // Add hover tooltip showing the resolved creature name.
        if !selected_creature_id.is_empty() {
            if let Ok(id_num) = selected_creature_id.trim().parse::<u32>() {
                if let Some((_, name)) = candidates.iter().find(|(id, _)| *id == id_num) {
                    // hover is shown on the response widget
                    let _ = response.on_hover_text(format!("Creature: {}", name));
                } else {
                    let _ = response.on_hover_text(format!(
                        "⚠ Creature ID '{}' not found in registry",
                        selected_creature_id
                    ));
                }
            }
        }

        // Clear button
        if ui.button("Clear").clicked()
            && (!selected_creature_id.is_empty() || !text_buffer.is_empty())
        {
            selected_creature_id.clear();
            text_buffer.clear();
            changed = true;
        }

        // Persist buffer back into egui memory so it survives frames. Store an
        // explicit empty string after Clear so the field remains visually empty
        // on the next frame instead of being reinitialized from stale state.
        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
    });

    changed
}

/// Shows an autocomplete input for selecting a condition by name.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display before the input
/// * `selected_condition_id` - Mutable reference to the selected condition ID
/// * `conditions` - Slice of available conditions
///
/// # Returns
///
/// `true` if the user selected a condition, `false` otherwise
pub fn autocomplete_condition_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_condition_id: &mut String,
    conditions: &[antares::domain::conditions::ConditionDefinition],
) -> bool {
    let candidates: Vec<String> = conditions.iter().map(|c| c.name.clone()).collect();
    let current_name = conditions
        .iter()
        .find(|c| c.id == *selected_condition_id)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    // Use RefCell so both on_select and on_clear can share mutation without conflicting borrows.
    let is_selected = !selected_condition_id.is_empty();
    let cell = std::cell::RefCell::new(std::mem::take(selected_condition_id));
    let cfg = AutocompleteSelectorConfig {
        id_salt,
        buffer_tag: "condition",
        label,
        placeholder: "Start typing condition name...",
    };
    let changed = autocomplete_entity_selector_generic(
        ui,
        &cfg,
        candidates,
        current_name,
        is_selected,
        |text| {
            if let Some(condition) = conditions.iter().find(|c| c.name == text) {
                *cell.borrow_mut() = condition.id.clone();
                true
            } else {
                false
            }
        },
        || cell.borrow_mut().clear(),
    );
    *selected_condition_id = cell.into_inner();
    changed
}

/// Shows an autocomplete input for selecting a map by ID.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display before the input
/// * `selected_map_id` - Mutable reference to the selected map ID string
/// * `maps` - Slice of available maps
///
/// # Returns
///
/// `true` if the user selected a map, `false` otherwise
pub fn autocomplete_map_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_map_id: &mut String,
    maps: &[antares::domain::world::Map],
) -> bool {
    let candidates: Vec<String> = maps
        .iter()
        .map(|m| format!("{} (ID: {})", m.name, m.id))
        .collect();
    let current_map_name =
        if let Ok(map_id) = selected_map_id.parse::<antares::domain::types::MapId>() {
            maps.iter()
                .find(|m| m.id == map_id)
                .map(|m| format!("{} (ID: {})", m.name, m.id))
                .unwrap_or_else(|| selected_map_id.clone())
        } else {
            selected_map_id.clone()
        };
    // Use RefCell so both on_select and on_clear can share mutation without conflicting borrows.
    let is_selected = !selected_map_id.is_empty();
    let cell = std::cell::RefCell::new(std::mem::take(selected_map_id));
    let cfg = AutocompleteSelectorConfig {
        id_salt,
        buffer_tag: "map",
        label,
        placeholder: "Start typing map name...",
    };
    let changed = autocomplete_entity_selector_generic(
        ui,
        &cfg,
        candidates,
        current_map_name,
        is_selected,
        |text| {
            if let Some(id_start) = text.rfind("(ID: ") {
                if let Some(id_end) = text[id_start..].find(')') {
                    let id_str = &text[id_start + 5..id_start + id_end];
                    *cell.borrow_mut() = id_str.to_string();
                    return true;
                }
            }
            false
        },
        || cell.borrow_mut().clear(),
    );
    *selected_map_id = cell.into_inner();
    changed
}

/// Shows an autocomplete input for selecting an NPC by ID.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display before the input
/// * `selected_npc_id` - Mutable reference to the selected NPC ID string (format: "map_id:npc_id")
/// * `maps` - Slice of available maps containing NPCs
///
/// # Returns
///
/// `true` if the user selected an NPC, `false` otherwise
pub fn autocomplete_npc_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_npc_id: &mut String,
    maps: &[antares::domain::world::Map],
) -> bool {
    let npc_candidates_list = extract_npc_candidates(maps);
    let candidates: Vec<String> = npc_candidates_list
        .iter()
        .map(|(display, _)| display.clone())
        .collect();
    let current_display = if !selected_npc_id.is_empty() {
        if let Some((map_id_str, _)) = selected_npc_id.split_once(':') {
            if let Ok(map_id) = map_id_str.parse::<antares::domain::types::MapId>() {
                let npc_id_part = &selected_npc_id[map_id_str.len() + 1..];
                maps.iter()
                    .find(|m| m.id == map_id)
                    .and_then(|m| m.npc_placements.iter().find(|n| n.npc_id == npc_id_part))
                    .map(|placement| format!("NPC ID: {} (Map: {})", placement.npc_id, map_id))
                    .unwrap_or_else(|| selected_npc_id.clone())
            } else {
                selected_npc_id.clone()
            }
        } else {
            selected_npc_id.clone()
        }
    } else {
        String::new()
    };
    // Use RefCell so both on_select and on_clear can share mutation without conflicting borrows.
    let is_selected = !selected_npc_id.is_empty();
    let cell = std::cell::RefCell::new(std::mem::take(selected_npc_id));
    let cfg = AutocompleteSelectorConfig {
        id_salt,
        buffer_tag: "npc",
        label,
        placeholder: "Start typing NPC name...",
    };
    let changed = autocomplete_entity_selector_generic(
        ui,
        &cfg,
        candidates,
        current_display,
        is_selected,
        |text| {
            for (display, npc_id) in extract_npc_candidates(maps) {
                if display == text {
                    *cell.borrow_mut() = npc_id;
                    return true;
                }
            }
            false
        },
        || cell.borrow_mut().clear(),
    );
    *selected_npc_id = cell.into_inner();
    changed
}

/// Shows an autocomplete input for selecting a recruitable character by name.
///
/// Returns `true` if the selection changed.
///
/// The `selected_character_id` is the bare character definition ID string
/// (e.g. `"old_gareth"`).  The widget displays `"{name} (ID: {id})"` in the
/// dropdown and writes back only the raw ID on selection.
pub fn autocomplete_character_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_character_id: &mut String,
    characters: &[antares::domain::character_definition::CharacterDefinition],
) -> bool {
    let candidates_list = extract_character_candidates(characters);
    let candidates: Vec<String> = candidates_list
        .iter()
        .map(|(display, _)| display.clone())
        .collect();
    let current_display = if selected_character_id.is_empty() {
        String::new()
    } else {
        characters
            .iter()
            .find(|c| c.id == *selected_character_id)
            .map(|c| format!("{} (ID: {})", c.name, c.id))
            .unwrap_or_else(|| selected_character_id.clone())
    };
    let is_selected = !selected_character_id.is_empty();
    let cell = std::cell::RefCell::new(std::mem::take(selected_character_id));
    let cfg = AutocompleteSelectorConfig {
        id_salt,
        buffer_tag: "character",
        label,
        placeholder: "Start typing character name or ID...",
    };
    let changed = autocomplete_entity_selector_generic(
        ui,
        &cfg,
        candidates,
        current_display,
        is_selected,
        |text| {
            for (display, char_id) in extract_character_candidates(characters) {
                if display == text {
                    *cell.borrow_mut() = char_id;
                    return true;
                }
            }
            false
        },
        || cell.borrow_mut().clear(),
    );
    *selected_character_id = cell.into_inner();
    changed
}

/// Generic multi-entity autocomplete list selector.
///
/// Core implementation shared by `autocomplete_item_list_selector`,
/// `autocomplete_proficiency_list_selector`, `autocomplete_tag_list_selector`,
/// `autocomplete_ability_list_selector`, and `autocomplete_monster_list_selector`.
///
/// # Type Parameters
///
/// * `T` — The element type stored in `selected`. Must be `Clone + PartialEq`.
/// * `D` — Display function: given a `&T`, returns a `String` label.
/// * `A` — Commit-on-change function: given typed text, returns `Some(T)` if
///   a valid item was identified, `None` otherwise. The generic checks
///   `!selected.contains(&t)` before adding.
/// * `E` — Commit-on-enter function: same contract as `A` but called when the
///   user presses Enter. For types that allow free-text entry (e.g. tags),
///   this may return `Some(T)` even for text not in `candidates`.
///
/// # Arguments
///
/// * `ui` — egui UI context
/// * `cfg` — display configuration bundle; see [`AutocompleteListSelectorConfig`]
/// * `selected` — mutable reference to the currently-selected items
/// * `display_fn` — how to render each selected item as a label string
/// * `candidates` — autocomplete suggestions
/// * `on_changed` — called when autocomplete fires `changed()`
/// * `on_enter` — called when Enter is pressed; may differ from `on_changed`
///   for types that allow free-text entry
///
/// # Returns
///
/// `true` if the selection changed this frame.
pub fn autocomplete_list_selector_generic<T, D, A, E>(
    ui: &mut egui::Ui,
    cfg: &AutocompleteListSelectorConfig<'_>,
    selected: &mut Vec<T>,
    display_fn: D,
    candidates: Vec<String>,
    mut on_changed: A,
    mut on_enter: E,
) -> bool
where
    T: Clone + PartialEq,
    D: Fn(&T) -> String,
    A: FnMut(&str) -> Option<T>,
    E: FnMut(&str) -> Option<T>,
{
    let mut changed = false;

    ui.group(|ui| {
        ui.label(cfg.label);

        let mut remove_idx: Option<usize> = None;
        for (idx, item) in selected.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(display_fn(item));
                if ui.small_button("✖").clicked() {
                    remove_idx = Some(idx);
                }
            });
        }

        if let Some(idx) = remove_idx {
            selected.remove(idx);
            changed = true;
        }

        ui.separator();

        let buffer_id = make_autocomplete_id(ui, cfg.buffer_tag, cfg.id_salt);
        let mut text_buffer = load_autocomplete_buffer(ui.ctx(), buffer_id, String::new);

        ui.horizontal(|ui| {
            ui.label(cfg.add_label);
            let response = AutocompleteInput::new(&format!("{}_add", cfg.id_salt), &candidates)
                .with_placeholder(cfg.placeholder)
                .show(ui, &mut text_buffer);

            let tb = text_buffer.trim().to_string();

            if response.changed() && !tb.is_empty() {
                if let Some(new_item) = on_changed(&tb) {
                    if !selected.contains(&new_item) {
                        selected.push(new_item);
                        changed = true;
                    }
                    text_buffer.clear();
                }
            }

            if response.has_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && !tb.is_empty()
            {
                if let Some(new_item) = on_enter(&tb) {
                    if !selected.contains(&new_item) {
                        selected.push(new_item);
                        changed = true;
                    }
                }
                text_buffer.clear();
            }
        });

        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
    });

    changed
}

/// Shows an autocomplete input for adding items to a list.
///
/// Returns `true` if an item was added to the list.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display before the input
/// * `selected_items` - Mutable reference to the list of selected ItemIds
/// * `items` - Slice of available items
///
/// # Returns
///
/// `true` if an item was added, `false` otherwise
pub fn autocomplete_item_list_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_items: &mut Vec<antares::domain::types::ItemId>,
    items: &[antares::domain::items::types::Item],
) -> bool {
    let candidates: Vec<String> = items.iter().map(|i| i.name.clone()).collect();
    let cfg = AutocompleteListSelectorConfig {
        id_salt,
        buffer_tag: "item_add",
        label,
        add_label: "Add item:",
        placeholder: "Start typing item name...",
    };
    autocomplete_list_selector_generic(
        ui,
        &cfg,
        selected_items,
        |id| {
            items
                .iter()
                .find(|i| i.id == *id)
                .map(|i| i.name.clone())
                .unwrap_or_else(|| format!("Unknown item (ID: {})", id))
        },
        candidates,
        |text| items.iter().find(|i| i.name == text).map(|i| i.id),
        |text| items.iter().find(|i| i.name == text).map(|i| i.id),
    )
}

/// Shows an autocomplete list selector for proficiencies.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display
/// * `selected_proficiencies` - Mutable reference to selected proficiency IDs
/// * `proficiencies` - Slice of available proficiency definitions
///
/// # Returns
///
/// `true` if the user changed the selection, `false` otherwise
pub fn autocomplete_proficiency_list_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_proficiencies: &mut Vec<String>,
    proficiencies: &[antares::domain::proficiency::ProficiencyDefinition],
) -> bool {
    let candidates: Vec<String> = proficiencies.iter().map(|p| p.name.clone()).collect();
    let cfg = AutocompleteListSelectorConfig {
        id_salt,
        buffer_tag: "prof_add",
        label,
        add_label: "Add proficiency:",
        placeholder: "Start typing proficiency...",
    };
    autocomplete_list_selector_generic(
        ui,
        &cfg,
        selected_proficiencies,
        |id| {
            proficiencies
                .iter()
                .find(|p| p.id == *id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("Unknown proficiency (ID: {})", id))
        },
        candidates,
        |text| {
            proficiencies
                .iter()
                .find(|p| p.name == text)
                .map(|p| p.id.clone())
        },
        |text| {
            proficiencies
                .iter()
                .find(|p| p.name == text)
                .map(|p| p.id.clone())
        },
    )
}

/// Shows an autocomplete list selector for item tags.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display
/// * `selected_tags` - Mutable reference to selected tag strings
/// * `available_tags` - Slice of available tag strings
///
/// # Returns
///
/// `true` if the user changed the selection, `false` otherwise
pub fn autocomplete_tag_list_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_tags: &mut Vec<String>,
    available_tags: &[String],
) -> bool {
    let candidates: Vec<String> = available_tags.to_vec();
    let candidates_clone = candidates.clone();
    let cfg = AutocompleteListSelectorConfig {
        id_salt,
        buffer_tag: "tag_add",
        label,
        add_label: "Add tag:",
        placeholder: "Start typing tag...",
    };
    autocomplete_list_selector_generic(
        ui,
        &cfg,
        selected_tags,
        |tag| tag.clone(),
        candidates,
        move |text: &str| {
            if candidates_clone.iter().any(|c| c.as_str() == text) {
                Some(text.to_string())
            } else {
                None
            }
        },
        |text: &str| {
            if !text.is_empty() {
                Some(text.to_string())
            } else {
                None
            }
        },
    )
}

/// Shows an autocomplete list selector for special abilities.
///
/// Returns `true` if the selection changed.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget
/// * `label` - Label to display
/// * `selected_abilities` - Mutable reference to selected ability strings
/// * `available_abilities` - Slice of available ability strings
///
/// # Returns
///
/// `true` if the user changed the selection, `false` otherwise
pub fn autocomplete_ability_list_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_abilities: &mut Vec<String>,
    available_abilities: &[String],
) -> bool {
    let candidates: Vec<String> = available_abilities.to_vec();
    let candidates_clone = candidates.clone();
    let cfg = AutocompleteListSelectorConfig {
        id_salt,
        buffer_tag: "ability_add",
        label,
        add_label: "Add ability:",
        placeholder: "Start typing ability...",
    };
    autocomplete_list_selector_generic(
        ui,
        &cfg,
        selected_abilities,
        |ability| ability.clone(),
        candidates,
        move |text: &str| {
            if candidates_clone.iter().any(|c| c.as_str() == text) {
                Some(text.to_string())
            } else {
                None
            }
        },
        |text: &str| {
            if !text.is_empty() {
                Some(text.to_string())
            } else {
                None
            }
        },
    )
}

/// Shows an autocomplete input for selecting a race by name.
///
/// Returns `true` if the selection changed.
pub fn autocomplete_race_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_race_id: &mut String,
    races: &[antares::domain::races::RaceDefinition],
) -> bool {
    let candidates: Vec<String> = races.iter().map(|r| r.name.clone()).collect();
    let current_name = races
        .iter()
        .find(|r| r.id == *selected_race_id)
        .map(|r| r.name.clone())
        .unwrap_or_default();
    // Use RefCell so both on_select and on_clear can share mutation without conflicting borrows.
    let is_selected = !selected_race_id.is_empty();
    let cell = std::cell::RefCell::new(std::mem::take(selected_race_id));
    let cfg = AutocompleteSelectorConfig {
        id_salt,
        buffer_tag: "race",
        label,
        placeholder: "Start typing race name...",
    };
    let changed = autocomplete_entity_selector_generic(
        ui,
        &cfg,
        candidates,
        current_name,
        is_selected,
        |text| {
            if let Some(race) = races.iter().find(|r| r.name == text) {
                *cell.borrow_mut() = race.id.clone();
                true
            } else {
                false
            }
        },
        || cell.borrow_mut().clear(),
    );
    *selected_race_id = cell.into_inner();
    changed
}

/// Shows an autocomplete input for selecting a class by name.
///
/// Returns `true` if the selection changed.
pub fn autocomplete_class_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_class_id: &mut String,
    classes: &[antares::domain::classes::ClassDefinition],
) -> bool {
    let candidates: Vec<String> = classes.iter().map(|c| c.name.clone()).collect();
    let current_name = classes
        .iter()
        .find(|c| c.id == *selected_class_id)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    // Use RefCell so both on_select and on_clear can share mutation without conflicting borrows.
    let is_selected = !selected_class_id.is_empty();
    let cell = std::cell::RefCell::new(std::mem::take(selected_class_id));
    let cfg = AutocompleteSelectorConfig {
        id_salt,
        buffer_tag: "class",
        label,
        placeholder: "Start typing class name...",
    };
    let changed = autocomplete_entity_selector_generic(
        ui,
        &cfg,
        candidates,
        current_name,
        is_selected,
        |text| {
            if let Some(class) = classes.iter().find(|c| c.name == text) {
                *cell.borrow_mut() = class.id.clone();
                true
            } else {
                false
            }
        },
        || cell.borrow_mut().clear(),
    );
    *selected_class_id = cell.into_inner();
    changed
}

/// Encounter monster list selector with per-type count controls.
///
/// Unlike the generic list selector, this widget allows **multiple instances**
/// of the same monster type in a single encounter (e.g. four Skeletons).
/// Monsters are displayed grouped by type; each row shows the name, the
/// current count, and ➕ / ➖ buttons to increment or decrement that count.
/// An autocomplete field below the list lets the user add a new monster type
/// (or an extra copy of an existing one) by name.
///
/// The underlying `Vec<MonsterId>` stores one entry per monster instance, so
/// three Skeletons are represented as three copies of the Skeleton ID.
/// Display grouping is derived from that flat list on every frame.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique identifier for this widget instance
/// * `label` - Label shown above the monster list group
/// * `selected_monsters` - Mutable reference to the encounter's monster list;
///   duplicates represent multiple monsters of the same type
/// * `monsters` - Slice of all available [`MonsterDefinition`]s
///
/// # Returns
///
/// Returns `true` if the list changed during this frame.
///
/// # Examples
///
/// ```no_run
/// use campaign_builder::ui_helpers::autocomplete_monster_list_selector;
/// use antares::domain::types::MonsterId;
///
/// fn show_encounter_editor(
///     ui: &mut egui::Ui,
///     selected: &mut Vec<MonsterId>,
///     monsters: &[antares::domain::combat::database::MonsterDefinition],
/// ) {
///     if autocomplete_monster_list_selector(ui, "my_encounter", "Monsters", selected, monsters) {
///         println!("Monster list changed");
///     }
/// }
/// ```
pub fn autocomplete_monster_list_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_monsters: &mut Vec<antares::domain::types::MonsterId>,
    monsters: &[antares::domain::combat::database::MonsterDefinition],
) -> bool {
    let mut changed = false;

    ui.group(|ui| {
        ui.label(label);

        // Build an ordered list of unique monster IDs with per-type counts,
        // preserving first-seen order so the display is stable across frames.
        let mut order: Vec<antares::domain::types::MonsterId> = Vec::new();
        let mut counts: std::collections::HashMap<antares::domain::types::MonsterId, usize> =
            std::collections::HashMap::new();
        for &mid in selected_monsters.iter() {
            *counts.entry(mid).or_insert(0) += 1;
            if !order.contains(&mid) {
                order.push(mid);
            }
        }

        let mut add_one: Option<antares::domain::types::MonsterId> = None;
        let mut remove_one: Option<antares::domain::types::MonsterId> = None;

        for &mid in &order {
            let count = counts[&mid];
            let name = monsters
                .iter()
                .find(|m| m.id == mid)
                .map(|m| m.name.as_str())
                .unwrap_or("Unknown");

            ui.push_id(mid, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("{} \u{00d7}{}", name, count));
                    if ui
                        .small_button("\u{2795}")
                        .on_hover_text("Add another of this monster")
                        .clicked()
                    {
                        add_one = Some(mid);
                    }
                    if ui
                        .small_button("\u{2796}")
                        .on_hover_text("Remove one of this monster")
                        .clicked()
                    {
                        remove_one = Some(mid);
                    }
                });
            });
        }

        if let Some(mid) = add_one {
            selected_monsters.push(mid);
            changed = true;
        }

        if let Some(mid) = remove_one {
            // Remove the last occurrence so the count decrements consistently.
            if let Some(pos) = selected_monsters.iter().rposition(|&m| m == mid) {
                selected_monsters.remove(pos);
                changed = true;
            }
        }

        ui.separator();

        // Autocomplete input to add a new monster type, or an extra copy of an
        // existing one. Unlike other list selectors this intentionally allows
        // duplicate IDs — every add pushes unconditionally onto the vec.
        let candidates: Vec<String> = monsters.iter().map(|m| m.name.clone()).collect();
        let buffer_id = make_autocomplete_id(ui, "monster_add", id_salt);
        let mut text_buffer = load_autocomplete_buffer(ui.ctx(), buffer_id, String::new);

        ui.horizontal(|ui| {
            ui.label("Add monster:");
            let response = AutocompleteInput::new(&format!("{}_add", id_salt), &candidates)
                .with_placeholder("Start typing monster name...")
                .show(ui, &mut text_buffer);

            let tb = text_buffer.trim().to_string();

            if response.changed() && !tb.is_empty() {
                if let Some(mid) = monsters.iter().find(|m| m.name == tb).map(|m| m.id) {
                    selected_monsters.push(mid);
                    changed = true;
                    text_buffer.clear();
                }
            }

            if response.has_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && !tb.is_empty()
            {
                if let Some(mid) = monsters.iter().find(|m| m.name == tb).map(|m| m.id) {
                    selected_monsters.push(mid);
                    changed = true;
                }
                text_buffer.clear();
            }
        });

        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
    });

    changed
}

/// Portrait selector widget with autocomplete functionality
///
/// Provides an autocomplete text input for selecting a portrait by ID.
/// The widget displays available portrait IDs as suggestions and allows
/// the user to type and select from the list.
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique identifier for this widget instance
/// * `label` - Text label to display before the input
/// * `selected_portrait_id` - Mutable reference to the currently selected portrait ID
/// * `available_portraits` - Slice of available portrait IDs to choose from
///
/// # Returns
///
/// Returns `true` if the selection changed during this frame, `false` otherwise
///
/// # Examples
///
/// ```no_run
/// use campaign_builder::ui_helpers::autocomplete_portrait_selector;
/// use std::path::PathBuf;
///
/// fn show_character_editor(ui: &mut egui::Ui, portrait_id: &mut String, portraits: &[String]) {
///     // Provide the campaign directory as a PathBuf and pass it as `Option<&PathBuf>`
///     let campaign_dir = PathBuf::from(".");
///     if autocomplete_portrait_selector(ui, "char_portrait", "Portrait:", portrait_id, portraits, Some(&campaign_dir)) {
///         println!("Portrait selection changed to: {}", portrait_id);
///     }
/// }
/// ```
pub fn autocomplete_portrait_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_portrait_id: &mut String,
    available_portraits: &[String],
    campaign_dir: Option<&PathBuf>,
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(label);

        // Get current portrait ID display value
        let current_value = selected_portrait_id.clone();

        let buffer_id = make_autocomplete_id(ui, "portrait", id_salt);

        // Build candidates from available portraits
        let candidates: Vec<String> = available_portraits.to_vec();

        // Persistent buffer logic
        let mut text_buffer =
            load_autocomplete_buffer(ui.ctx(), buffer_id, || current_value.clone());

        let mut response = AutocompleteInput::new(id_salt, &candidates)
            .with_placeholder("Start typing portrait ID...")
            .show(ui, &mut text_buffer);

        // Add tooltip showing full portrait path for current selection
        if !selected_portrait_id.is_empty() {
            if let Some(path) = resolve_portrait_path(campaign_dir, selected_portrait_id) {
                response = response.on_hover_text(format!("Portrait path: {}", path.display()));
            } else {
                response = response.on_hover_text(format!(
                    "⚠ Portrait '{}' not found in campaign assets/portraits",
                    selected_portrait_id
                ));
            }
        }

        // Commit valid selections
        if response.changed()
            && !text_buffer.is_empty()
            && text_buffer != current_value
            && available_portraits.contains(&text_buffer)
        {
            *selected_portrait_id = text_buffer.clone();
            changed = true;
        }

        // Show clear button
        if ui.button("Clear").clicked()
            && (!selected_portrait_id.is_empty() || !text_buffer.is_empty())
        {
            selected_portrait_id.clear();
            text_buffer.clear();
            changed = true;
        }

        // Persist buffer back into egui memory so it survives frames. Store an
        // explicit empty string after Clear so the field remains visually empty
        // on the next frame instead of being reinitialized from stale state.
        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
    });

    changed
}

/// Autocomplete selector for sprite sheet paths.
///
/// Provides a text autocomplete for sprite sheet candidates discovered in the campaign's
/// `assets/sprites/` directory. The displayed candidate strings are the relative paths
/// (relative to the campaign directory), e.g. `assets/sprites/actors/wizard.png`.
///
/// # Arguments
///
/// * `ui` - The egui UI
/// * `id_salt` - Salt used for the autocomplete id
/// * `label` - Label shown to the user
/// * `selected_sheet` - Mutable reference to currently selected sheet path
/// * `available_sheets` - Candidate sheet paths to suggest
/// * `campaign_dir` - Optional campaign dir (used to validate/show full path tooltip)
///
/// # Returns
///
/// Returns `true` if the selection changed (cleared or set)
pub fn autocomplete_sprite_sheet_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_sheet: &mut String,
    available_sheets: &[String],
    campaign_dir: Option<&PathBuf>,
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(label);

        // Current display value
        let current_value = selected_sheet.clone();

        let buffer_id = make_autocomplete_id(ui, "sprite", id_salt);

        // Build candidates from available sprite sheets
        let candidates: Vec<String> = available_sheets.to_vec();

        // Persistent buffer logic
        let mut text_buffer =
            load_autocomplete_buffer(ui.ctx(), buffer_id, || current_value.clone());

        let mut response = AutocompleteInput::new(id_salt, &candidates)
            .with_placeholder("Start typing sprite sheet path...")
            .show(ui, &mut text_buffer);

        // Add tooltip showing full sprite path for current selection
        if !selected_sheet.is_empty() {
            if let Some(dir) = campaign_dir {
                let path = dir.join(selected_sheet.as_str());
                if path.exists() {
                    response = response.on_hover_text(format!("Sprite path: {}", path.display()));
                } else {
                    response = response.on_hover_text(format!(
                        "⚠ Sprite '{}' not found in campaign assets/sprites",
                        selected_sheet.as_str()
                    ));
                }
            } else {
                response = response.on_hover_text(format!("Sprite: {}", selected_sheet.as_str()));
            }
        }

        // Commit valid selections (only accept selections that are in candidates)
        if response.changed()
            && !text_buffer.is_empty()
            && text_buffer != current_value
            && candidates.contains(&text_buffer)
        {
            *selected_sheet = text_buffer.clone();
            changed = true;
        }

        // Clear button
        if ui.button("Clear").clicked() && (!selected_sheet.is_empty() || !text_buffer.is_empty()) {
            selected_sheet.clear();
            text_buffer.clear();
            changed = true;
        }

        // Persist buffer back into egui memory so it survives frames. Store an
        // explicit empty string after Clear so the field remains visually empty
        // on the next frame instead of being reinitialized from stale state.
        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
    });

    changed
}

/// Extracts monster name candidates from a list of monster definitions.
///
/// Returns a vector of monster names suitable for autocomplete widgets.
///
/// # Arguments
///
/// * `monsters` - Slice of monster definitions to extract names from
///
/// # Returns
///
/// A vector of monster names as strings
///
/// # Examples
///
/// ```no_run
/// use antares::domain::combat::database::MonsterDefinition;
/// use campaign_builder::ui_helpers::extract_monster_candidates;
///
/// let monsters: Vec<MonsterDefinition> = Vec::new();
/// let candidates = extract_monster_candidates(&monsters);
/// ```
pub fn extract_monster_candidates(
    monsters: &[antares::domain::combat::database::MonsterDefinition],
) -> Vec<String> {
    monsters.iter().map(|m| m.name.clone()).collect()
}

/// Extracts race name candidates from a list of race definitions.
pub fn extract_race_candidates(races: &[antares::domain::races::RaceDefinition]) -> Vec<String> {
    races.iter().map(|r| r.name.clone()).collect()
}

/// Extracts class name candidates from a list of class definitions.
pub fn extract_class_candidates(
    classes: &[antares::domain::classes::ClassDefinition],
) -> Vec<String> {
    classes.iter().map(|c| c.name.clone()).collect()
}

/// Extracts item candidates from a list of items.
///
/// Returns a vector of tuples mapping item display name to ItemId.
/// Display format is "{name} (ID: {id})" for clarity in the autocomplete UI.
///
/// # Arguments
///
/// * `items` - Slice of items to extract candidates from
///
/// # Returns
///
/// A vector of tuples (display_name, ItemId)
///
/// # Examples
///
/// ```no_run
/// use antares::domain::items::types::Item;
/// use campaign_builder::ui_helpers::extract_item_candidates;
///
/// let items: Vec<Item> = Vec::new();
/// let candidates = extract_item_candidates(&items);
/// ```
pub fn extract_item_candidates(
    items: &[antares::domain::items::types::Item],
) -> Vec<(String, antares::domain::types::ItemId)> {
    items
        .iter()
        .map(|item| (format!("{} (ID: {})", item.name, item.id), item.id))
        .collect()
}

/// Extracts quest candidates from a list of quests.
///
/// Returns a vector of tuples mapping quest display name to QuestId.
///
/// # Arguments
///
/// * `quests` - Slice of quests to extract candidates from
///
/// # Returns
///
/// A vector of tuples (display_name, QuestId)
///
/// # Examples
///
/// ```
/// use antares::domain::quest::Quest;
/// use campaign_builder::ui_helpers::extract_quest_candidates;
///
/// let quests = vec![
///     Quest {
///         id: 1,
///         name: "Save the Village".to_string(),
///         description: "Help save the village from bandits".to_string(),
///         stages: vec![],
///         rewards: vec![],
///         required_quests: vec![],
///         min_level: Some(1),
///         max_level: None,
///         repeatable: false,
///         is_main_quest: true,
///         quest_giver_npc: None,
///         quest_giver_map: None,
///         quest_giver_position: None,
///     },
/// ];
///
/// let candidates = extract_quest_candidates(&quests);
/// assert_eq!(candidates.len(), 1);
/// assert_eq!(candidates[0].0, "Save the Village (ID: 1)");
/// assert_eq!(candidates[0].1, 1);
/// ```
pub fn extract_quest_candidates(
    quests: &[antares::domain::quest::Quest],
) -> Vec<(String, antares::domain::quest::QuestId)> {
    quests
        .iter()
        .map(|quest| (format!("{} (ID: {})", quest.name, quest.id), quest.id))
        .collect()
}

/// Extracts condition candidates from a list of condition definitions.
///
/// Returns a vector of tuples mapping condition name to ConditionId.
///
/// # Arguments
///
/// * `conditions` - Slice of condition definitions to extract candidates from
///
/// # Returns
///
/// A vector of tuples (condition_name, ConditionId)
///
/// # Examples
///
/// ```no_run
/// use antares::domain::conditions::ConditionDefinition;
/// use campaign_builder::ui_helpers::extract_condition_candidates;
///
/// let conditions: Vec<ConditionDefinition> = Vec::new();
/// let candidates = extract_condition_candidates(&conditions);
/// ```
pub fn extract_condition_candidates(
    conditions: &[antares::domain::conditions::ConditionDefinition],
) -> Vec<(String, String)> {
    conditions
        .iter()
        .map(|cond| (cond.name.clone(), cond.id.clone()))
        .collect()
}

/// Extracts spell candidates from a list of spell definitions.
///
/// Returns a vector of tuples mapping spell display name to SpellId.
/// Display format is "{name} (ID: {id})" for clarity.
///
/// # Arguments
///
/// * `spells` - Slice of spells to extract candidates from
///
/// # Returns
///
/// A vector of tuples (display_name, SpellId)
///
/// # Examples
///
/// ```no_run
/// use antares::domain::types::SpellId;
/// use campaign_builder::ui_helpers::extract_spell_candidates;
///
/// // Use an empty slice in examples to avoid constructing a full `Spell` value
/// let spells: &[antares::domain::magic::types::Spell] = &[];
/// let candidates = extract_spell_candidates(spells);
/// assert!(candidates.is_empty());
/// ```
pub fn extract_spell_candidates(
    spells: &[antares::domain::magic::types::Spell],
) -> Vec<(String, antares::domain::types::SpellId)> {
    spells
        .iter()
        .map(|spell| (format!("{} (ID: {})", spell.name, spell.id), spell.id))
        .collect()
}

/// Extracts proficiency candidates from the proficiency database.
///
/// Returns a vector of proficiency ID strings suitable for autocomplete.
///
/// # Arguments
///
/// * `proficiencies` - Slice of proficiency IDs
///
/// # Returns
///
/// A vector of proficiency ID strings
///
/// # Examples
///
/// ```no_run
/// use antares::domain::proficiency::{ProficiencyDefinition, ProficiencyId, ProficiencyCategory};
/// use campaign_builder::ui_helpers::extract_proficiency_candidates;
///
/// let proficiencies = vec![
///     ProficiencyDefinition {
///         id: "sword".to_string(),
///         name: "Sword".to_string(),
///         category: ProficiencyCategory::Weapon,
///         description: "Sword proficiency".to_string(),
///     },
/// ];
/// let candidates = extract_proficiency_candidates(&proficiencies);
/// assert_eq!(candidates.len(), 1);
/// ```
pub fn extract_proficiency_candidates(
    proficiencies: &[antares::domain::proficiency::ProficiencyDefinition],
) -> Vec<(String, String)> {
    proficiencies
        .iter()
        .map(|p| (format!("{} ({})", p.name, p.id), p.id.clone()))
        .collect()
}

/// Loads proficiency definitions with a tri-stage fallback:
/// 1. Campaign directory RON file
/// 2. Global data directory RON file
/// 3. Synthetic generation based on item classifications
pub fn load_proficiencies(
    campaign_dir: Option<&PathBuf>,
    items: &[Item],
) -> Vec<ProficiencyDefinition> {
    // Stage 1: Try campaign directory
    if let Some(dir) = campaign_dir {
        let path = dir.join("data/proficiencies.ron");
        if path.exists() {
            if let Ok(db) = ProficiencyDatabase::load_from_file(&path) {
                return db.all().into_iter().cloned().collect();
            }
        }
    }

    // Stage 2: Try global data directory
    if let Ok(db) = ProficiencyDatabase::load_from_file("data/proficiencies.ron") {
        return db.all().into_iter().cloned().collect();
    }

    // Stage 3: Synthetic Fallback
    generate_synthetic_proficiencies(items)
}

pub(crate) fn generate_synthetic_proficiencies(items: &[Item]) -> Vec<ProficiencyDefinition> {
    let mut profs = std::collections::HashMap::new();

    // Standard proficiencies that should always be available
    let standard = vec![
        (
            "simple_weapon",
            "Simple Weapons",
            ProficiencyCategory::Weapon,
        ),
        (
            "martial_melee",
            "Martial Melee Weapons",
            ProficiencyCategory::Weapon,
        ),
        (
            "martial_ranged",
            "Martial Ranged Weapons",
            ProficiencyCategory::Weapon,
        ),
        ("blunt_weapon", "Blunt Weapons", ProficiencyCategory::Weapon),
        ("unarmed", "Unarmed Combat", ProficiencyCategory::Weapon),
        ("light_armor", "Light Armor", ProficiencyCategory::Armor),
        ("medium_armor", "Medium Armor", ProficiencyCategory::Armor),
        ("heavy_armor", "Heavy Armor", ProficiencyCategory::Armor),
        ("shield", "Shield", ProficiencyCategory::Shield),
        (
            "arcane_item",
            "Arcane Magic Items",
            ProficiencyCategory::MagicItem,
        ),
        (
            "divine_item",
            "Divine Magic Items",
            ProficiencyCategory::MagicItem,
        ),
    ];

    for (id, name, cat) in standard {
        profs.insert(
            id.to_string(),
            ProficiencyDefinition::with_description(
                id.to_string(),
                name.to_string(),
                cat,
                format!("Standard {} proficiency", name),
            ),
        );
    }

    // Scan items for any classifications not in standard
    for item in items {
        if let Some(prof_id) = item.required_proficiency() {
            if !profs.contains_key(&prof_id) {
                let name = prof_id
                    .replace('_', " ")
                    .split_whitespace()
                    .map(|w| {
                        let mut c = w.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                let category = match &item.item_type {
                    antares::domain::items::ItemType::Weapon(_) => ProficiencyCategory::Weapon,
                    antares::domain::items::ItemType::Armor(_) => ProficiencyCategory::Armor,
                    antares::domain::items::ItemType::Accessory(_) => {
                        ProficiencyCategory::MagicItem
                    }
                    _ => ProficiencyCategory::Weapon,
                };

                profs.insert(
                    prof_id.clone(),
                    ProficiencyDefinition::with_description(
                        prof_id.clone(),
                        name,
                        category,
                        "Derived from campaign items".to_string(),
                    ),
                );
            }
        }
    }

    let mut result: Vec<_> = profs.into_values().collect();
    result.sort_by(|a, b| a.id.cmp(&b.id));
    result
}

/// Extracts item tag candidates from a list of items.
///
/// Returns a vector of unique item tags suitable for autocomplete widgets.
///
/// # Arguments
///
/// * `items` - Slice of items to extract tags from
///
/// # Returns
///
/// A vector of unique tag strings sorted alphabetically
///
/// # Examples
///
/// ```no_run
/// use antares::domain::items::types::Item;
/// use campaign_builder::ui_helpers::extract_item_tag_candidates;
///
/// let items = vec![]; // Items with tags
/// let candidates = extract_item_tag_candidates(&items);
/// ```
pub fn extract_item_tag_candidates(items: &[antares::domain::items::types::Item]) -> Vec<String> {
    use std::collections::HashSet;

    let mut tags = HashSet::new();
    for item in items {
        for tag in &item.tags {
            tags.insert(tag.clone());
        }
    }

    let mut result: Vec<String> = tags.into_iter().collect();
    result.sort();
    result
}

/// Extracts special ability candidates from existing race definitions.
///
/// Returns a vector of unique special abilities suitable for autocomplete widgets.
///
/// # Arguments
///
/// * `races` - Slice of race definitions to extract abilities from
///
/// # Returns
///
/// A vector of unique special ability strings sorted alphabetically
///
/// # Examples
///
/// ```no_run
/// use antares::domain::races::RaceDefinition;
/// use campaign_builder::ui_helpers::extract_special_ability_candidates;
///
/// let races = vec![]; // Races with special abilities
/// let candidates = extract_special_ability_candidates(&races);
/// ```
pub fn extract_special_ability_candidates(
    races: &[antares::domain::races::RaceDefinition],
) -> Vec<String> {
    use std::collections::HashSet;

    let mut abilities = HashSet::new();
    for race in races {
        for ability in &race.special_abilities {
            abilities.insert(ability.clone());
        }
    }

    // Add common standard abilities
    let standard_abilities = vec![
        "infravision",
        "magic_resistance",
        "poison_immunity",
        "disease_immunity",
        "keen_senses",
        "darkvision",
        "lucky",
        "brave",
        "stonecunning",
        "trance",
    ];

    for ability in standard_abilities {
        abilities.insert(ability.to_string());
    }

    let mut result: Vec<String> = abilities.into_iter().collect();
    result.sort();
    result
}

/// Extracts map candidates for autocomplete from a slice of maps.
///
/// Returns a list of tuples containing display string and map ID.
///
/// # Examples
///
/// ```
/// use antares::domain::world::Map;
/// use campaign_builder::ui_helpers::extract_map_candidates;
///
/// let maps = vec![
///     Map::new(1, "Town Square".to_string(), "Starting area".to_string(), 20, 20),
///     Map::new(2, "Dark Forest".to_string(), "Dangerous woods".to_string(), 30, 30),
/// ];
/// let candidates = extract_map_candidates(&maps);
/// assert_eq!(candidates.len(), 2);
/// assert_eq!(candidates[0].0, "Town Square (ID: 1)");
/// ```
pub fn extract_map_candidates(
    maps: &[antares::domain::world::Map],
) -> Vec<(String, antares::domain::types::MapId)> {
    maps.iter()
        .map(|map| (format!("{} (ID: {})", map.name, map.id), map.id))
        .collect()
}

/// Extracts NPC candidates for autocomplete from all NPCs in all maps.
///
/// Returns a list of tuples containing display string and NPC ID.
/// NPC IDs are formatted as "{map_id}:{npc_id}" to ensure uniqueness across maps.
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Map, npc::NpcPlacement};
/// use antares::domain::types::Position;
/// use campaign_builder::ui_helpers::extract_npc_candidates;
///
/// let mut map = Map::new(1, "Town".to_string(), "Desc".to_string(), 10, 10);
/// map.npc_placements.push(NpcPlacement::new("merchant", Position::new(5, 5)));
///
/// let candidates = extract_npc_candidates(&[map]);
/// assert_eq!(candidates.len(), 1);
/// assert_eq!(candidates[0].1, "1:merchant".to_string());
/// assert!(candidates[0].0.contains("Town"));
/// ```
pub fn extract_npc_candidates(maps: &[antares::domain::world::Map]) -> Vec<(String, String)> {
    let mut candidates = Vec::new();
    for map in maps {
        for placement in &map.npc_placements {
            let display = format!(
                "{} (Map: {}, Position: {:?})",
                placement.npc_id, map.name, placement.position
            );
            let npc_id = format!("{}:{}", map.id, placement.npc_id);
            candidates.push((display, npc_id));
        }
    }
    candidates
}

/// Extracts character candidates from a list of character definitions.
///
/// Returns `(display, id)` pairs where display is `"{name} (ID: {id})"`.
///
/// # Examples
///
/// ```
/// use campaign_builder::ui_helpers::extract_character_candidates;
/// use antares::domain::character::Alignment;
/// use antares::domain::character::Sex;
/// use antares::domain::character_definition::CharacterDefinition;
///
/// let characters = vec![CharacterDefinition::new(
///     "old_gareth".to_string(),
///     "Old Gareth".to_string(),
///     "dwarf".to_string(),
///     "fighter".to_string(),
///     Sex::Male,
///     Alignment::Neutral,
/// )];
/// let candidates = extract_character_candidates(&characters);
/// assert_eq!(candidates.len(), 1);
/// assert_eq!(candidates[0].1, "old_gareth");
/// ```
pub fn extract_character_candidates(
    characters: &[antares::domain::character_definition::CharacterDefinition],
) -> Vec<(String, String)> {
    characters
        .iter()
        .map(|c| (format!("{} (ID: {})", c.name, c.id), c.id.clone()))
        .collect()
}

/// Extracts portrait candidates from the campaign's portrait assets directory.
///
/// Scans the `campaign_dir/assets/portraits` directory for image files (`.png`, `.jpg`, `.jpeg`)
/// and returns a sorted list of portrait ID strings extracted from filenames.
///
/// # Arguments
///
/// * `campaign_dir` - Optional path to the campaign directory
///
/// # Returns
///
/// A vector of portrait ID strings sorted numerically (e.g., ["0", "1", "2", "10", "20"])
/// Returns an empty vector if the campaign directory is None or the portraits directory doesn't exist.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use campaign_builder::ui_helpers::extract_portrait_candidates;
///
/// let campaign_dir = PathBuf::from("/path/to/campaign");
/// let portraits = extract_portrait_candidates(Some(&campaign_dir));
/// // Returns portrait IDs found in /path/to/campaign/assets/portraits/
/// // e.g., ["0", "1", "2"] if files 0.png, 1.png, 2.png exist
/// ```
pub fn extract_portrait_candidates(campaign_dir: Option<&PathBuf>) -> Vec<String> {
    let Some(dir) = campaign_dir else {
        return Vec::new();
    };

    let portraits_dir = dir.join("assets").join("portraits");

    // Return empty if directory doesn't exist
    if !portraits_dir.exists() || !portraits_dir.is_dir() {
        return Vec::new();
    }

    let Ok(entries) = std::fs::read_dir(&portraits_dir) else {
        return Vec::new();
    };

    let mut portrait_ids = Vec::new();
    let valid_extensions = ["png", "jpg", "jpeg"];

    for entry in entries.flatten() {
        let path = entry.path();

        // Only process files with valid image extensions
        if let Some(extension) = path.extension() {
            let ext_str = extension.to_string_lossy().to_lowercase();
            if valid_extensions.contains(&ext_str.as_str()) {
                // Extract filename without extension as portrait ID
                if let Some(file_stem) = path.file_stem() {
                    let portrait_id = file_stem.to_string_lossy().to_string();

                    // Prioritize PNG files - if we already have this ID from a different format, skip
                    if ext_str == "png" {
                        // Remove any existing entry for this ID and add PNG version
                        portrait_ids.retain(|id| id != &portrait_id);
                        portrait_ids.push(portrait_id);
                    } else if !portrait_ids.contains(&portrait_id) {
                        portrait_ids.push(portrait_id);
                    }
                }
            }
        }
    }

    // Sort numerically if possible, otherwise alphabetically
    portrait_ids.sort_by(|a, b| match (a.parse::<u32>(), b.parse::<u32>()) {
        (Ok(a_num), Ok(b_num)) => a_num.cmp(&b_num),
        _ => a.cmp(b),
    });

    portrait_ids
}

/// Extract sprite sheet candidate paths from a campaign's `assets/sprites` directory.
///
/// Returns a vector of relative paths (e.g., "assets/sprites/background.png",
/// "assets/sprites/actors/wizard.png"). If `campaign_dir` is `None`, the
/// campaign directory doesn't exist, or there is no `assets/sprites`
/// directory, an empty vector is returned.
///
/// The function traverses the `assets/sprites` tree recursively and returns
/// deterministic, sorted, deduplicated results suitable for UI pickers.
pub fn extract_sprite_sheet_candidates(campaign_dir: Option<&PathBuf>) -> Vec<String> {
    let Some(dir) = campaign_dir else {
        return Vec::new();
    };

    let sprites_dir = dir.join("assets").join("sprites");

    // Return empty if directory doesn't exist
    if !sprites_dir.exists() || !sprites_dir.is_dir() {
        return Vec::new();
    }

    let mut candidates: Vec<String> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![sprites_dir];

    // Iterative DFS to avoid dependency on `walkdir` crate
    while let Some(path) = stack.pop() {
        let entries = match std::fs::read_dir(&path) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.is_file() {
                // Convert to path relative to campaign dir (e.g., assets/sprites/...)
                if let Ok(rel) = p.strip_prefix(dir) {
                    if let Some(s) = rel.to_str() {
                        // Normalize separators to '/' for deterministic tests across platforms
                        candidates.push(s.replace('\\', "/"));
                    }
                }
            }
        }
    }

    // Deterministic ordering and uniqueness
    candidates.sort();
    candidates.dedup();

    candidates
}

/// Extracts creature asset file path candidates from a campaign's `assets/creatures` directory.
///
/// Returns a vector of relative paths (e.g., `"assets/creatures/goblin.ron"`,
/// `"assets/creatures/orc_warrior.ron"`) suitable for autocomplete widgets. Only
/// `.ron` files are included.  If `campaign_dir` is `None`, the campaign directory
/// does not exist, or there is no `assets/creatures` subdirectory, an empty vector
/// is returned.
///
/// Results are sorted alphabetically and deduplicated for deterministic output.
///
/// # Arguments
///
/// * `campaign_dir` - Optional path to the campaign root directory
///
/// # Returns
///
/// A sorted, deduplicated vector of relative creature asset paths.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use campaign_builder::ui_helpers::extract_creature_asset_candidates;
///
/// let campaign_dir = PathBuf::from("/path/to/campaign");
/// let candidates = extract_creature_asset_candidates(Some(&campaign_dir));
/// // Returns e.g. ["assets/creatures/goblin.ron", "assets/creatures/orc.ron"]
/// ```
pub fn extract_creature_asset_candidates(campaign_dir: Option<&PathBuf>) -> Vec<String> {
    let Some(dir) = campaign_dir else {
        return Vec::new();
    };

    let creatures_dir = dir.join("assets").join("creatures");

    if !creatures_dir.exists() || !creatures_dir.is_dir() {
        return Vec::new();
    }

    let Ok(entries) = std::fs::read_dir(&creatures_dir) else {
        return Vec::new();
    };

    let mut candidates: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "ron" {
                    if let Ok(rel) = path.strip_prefix(dir) {
                        if let Some(s) = rel.to_str() {
                            candidates.push(s.replace('\\', "/"));
                        }
                    }
                }
            }
        }
    }

    candidates.sort();
    candidates.dedup();
    candidates
}

/// Autocomplete selector for creature asset file paths.
///
/// Provides a text autocomplete for creature `.ron` asset paths discovered in
/// the campaign's `assets/creatures/` directory.  The displayed candidate strings
/// are the relative paths (relative to the campaign directory), e.g.
/// `"assets/creatures/goblin.ron"`.
///
/// The widget follows the same persistent-buffer pattern used by
/// [`autocomplete_portrait_selector`] and [`autocomplete_sprite_sheet_selector`]:
/// the typed text survives across frames and is only committed to the output
/// `selected_path` string when the user picks a value that exists in the
/// candidate list.  A "Clear" button resets the selection.
///
/// # Arguments
///
/// * `ui` - The egui UI
/// * `id_salt` - Salt for the autocomplete widget ID (must be unique in the scope)
/// * `label` - Label shown to the left of the input
/// * `selected_path` - Mutable reference to the currently selected relative path
/// * `available_paths` - Candidate paths to suggest (from [`extract_creature_asset_candidates`])
/// * `campaign_dir` - Optional campaign directory (used to show a hover tooltip
///   indicating whether the selected file actually exists on disk)
///
/// # Returns
///
/// Returns `true` if the selection changed (a new path was selected or the field
/// was cleared).
///
/// # Examples
///
/// ```no_run
/// use campaign_builder::ui_helpers::{
///     autocomplete_creature_asset_selector, extract_creature_asset_candidates,
/// };
/// use std::path::PathBuf;
///
/// fn show_register_dialog(
///     ui: &mut egui::Ui,
///     path: &mut String,
///     campaign_dir: Option<&PathBuf>,
/// ) {
///     let candidates = extract_creature_asset_candidates(campaign_dir);
///     if autocomplete_creature_asset_selector(
///         ui,
///         "register_asset_path",
///         "Path:",
///         path,
///         &candidates,
///         campaign_dir,
///     ) {
///         println!("Creature asset path changed to: {}", path);
///     }
/// }
/// ```
pub fn autocomplete_creature_asset_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_path: &mut String,
    available_paths: &[String],
    campaign_dir: Option<&PathBuf>,
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(label);

        let current_value = selected_path.clone();
        let buffer_id = make_autocomplete_id(ui, "creature_asset", id_salt);
        let candidates: Vec<String> = available_paths.to_vec();

        let mut text_buffer =
            load_autocomplete_buffer(ui.ctx(), buffer_id, || current_value.clone());

        let mut response = AutocompleteInput::new(id_salt, &candidates)
            .with_placeholder("Start typing path, e.g. assets/creatures/goblin.ron")
            .show(ui, &mut text_buffer);

        // Tooltip: show whether the file exists, or a warning if not found
        if !selected_path.is_empty() {
            if let Some(dir) = campaign_dir {
                let full_path = dir.join(selected_path.as_str());
                if full_path.exists() {
                    response =
                        response.on_hover_text(format!("File found: {}", full_path.display()));
                } else {
                    response = response
                        .on_hover_text(format!("⚠ File not found: {}", full_path.display()));
                }
            } else {
                response =
                    response.on_hover_text(format!("Asset path: {}", selected_path.as_str()));
            }
        }

        // Only commit when the typed text matches a known candidate
        if response.changed()
            && !text_buffer.is_empty()
            && text_buffer != current_value
            && candidates.contains(&text_buffer)
        {
            *selected_path = text_buffer.clone();
            changed = true;
        }

        if ui.button("Clear").clicked() && (!selected_path.is_empty() || !text_buffer.is_empty()) {
            selected_path.clear();
            text_buffer.clear();
            changed = true;
        }

        // Persist buffer back into egui memory so it survives frames. Store an
        // explicit empty string after Clear so the field remains visually empty
        // on the next frame instead of being reinitialized from stale state.
        store_autocomplete_buffer(ui.ctx(), buffer_id, &text_buffer);
    });

    changed
}

/// Resolves a portrait ID to its full file path.
///
/// Attempts to find the portrait file in the campaign's assets/portraits directory.
/// Prioritizes PNG format, but will also check for JPG/JPEG if PNG is not found.
///
/// # Arguments
///
/// * `campaign_dir` - Optional path to the campaign directory
/// * `portrait_id` - The portrait ID (filename without extension)
///
/// # Returns
///
/// `Some(PathBuf)` if the portrait file exists, `None` otherwise.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use campaign_builder::ui_helpers::resolve_portrait_path;
///
/// let campaign_dir = PathBuf::from("/path/to/campaign");
/// let portrait_path = resolve_portrait_path(Some(&campaign_dir), "0");
/// // Returns Some(/path/to/campaign/assets/portraits/0.png) if it exists
/// ```
pub fn resolve_portrait_path(campaign_dir: Option<&PathBuf>, portrait_id: &str) -> Option<PathBuf> {
    let dir = campaign_dir?;
    let portraits_dir = dir.join("assets").join("portraits");

    // Prioritize PNG format
    let png_path = portraits_dir.join(format!("{}.png", portrait_id));
    if png_path.exists() {
        return Some(png_path);
    }

    // Try other supported formats
    for ext in ["jpg", "jpeg"] {
        let path = portraits_dir.join(format!("{}.{}", portrait_id, ext));
        if path.exists() {
            return Some(path);
        }
    }

    None
}

// =============================================================================
// Candidate Cache for Performance Optimization
// =============================================================================

/// Cache for autocomplete candidates to avoid regenerating on every frame.
///
/// This structure caches candidate lists and invalidates them only when
/// the underlying data changes (add/delete/import operations).
#[derive(Debug, Default)]
pub struct AutocompleteCandidateCache {
    /// Cached item candidates with generation counter
    pub(crate) items: Option<(Vec<(String, antares::domain::types::ItemId)>, u64)>,
    /// Cached monster candidates with generation counter
    pub(crate) monsters: Option<(Vec<String>, u64)>,
    /// Cached condition candidates with generation counter
    pub(crate) conditions: Option<(Vec<(String, String)>, u64)>,
    /// Cached spell candidates with generation counter
    pub(crate) spells: Option<(Vec<(String, antares::domain::types::SpellId)>, u64)>,
    /// Cached proficiency candidates with generation counter
    pub(crate) proficiencies: Option<(Vec<(String, String)>, u64)>,
    /// Generation counter for items (incremented on data changes)
    pub(crate) items_generation: u64,
    /// Generation counter for monsters
    pub(crate) monsters_generation: u64,
    /// Generation counter for conditions
    pub(crate) conditions_generation: u64,
    /// Generation counter for spells
    pub(crate) spells_generation: u64,
    /// Generation counter for proficiencies
    pub(crate) proficiencies_generation: u64,
}

impl AutocompleteCandidateCache {
    /// Creates a new empty candidate cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Invalidates the item candidates cache.
    ///
    /// Call this when items are added, deleted, or imported.
    pub fn invalidate_items(&mut self) {
        self.items_generation += 1;
        self.items = None;
    }

    /// Invalidates the monster candidates cache.
    ///
    /// Call this when monsters are added, deleted, or imported.
    pub fn invalidate_monsters(&mut self) {
        self.monsters_generation += 1;
        self.monsters = None;
    }

    /// Invalidates the condition candidates cache.
    ///
    /// Call this when conditions are added, deleted, or imported.
    pub fn invalidate_conditions(&mut self) {
        self.conditions_generation += 1;
        self.conditions = None;
    }

    /// Invalidates the spell candidates cache.
    ///
    /// Call this when spells are added, deleted, or imported.
    pub fn invalidate_spells(&mut self) {
        self.spells_generation += 1;
        self.spells = None;
    }

    /// Invalidates the proficiency candidates cache.
    ///
    /// Call this when proficiencies are added, deleted, or imported.
    pub fn invalidate_proficiencies(&mut self) {
        self.proficiencies_generation += 1;
        self.proficiencies = None;
    }

    /// Invalidates all caches.
    ///
    /// Call this when loading a new campaign or resetting data.
    pub fn invalidate_all(&mut self) {
        self.invalidate_items();
        self.invalidate_monsters();
        self.invalidate_conditions();
        self.invalidate_spells();
        self.invalidate_proficiencies();
    }

    /// Gets or generates item candidates.
    ///
    /// Returns cached candidates if available and valid, otherwise generates
    /// new candidates and caches them.
    pub fn get_or_generate_items(
        &mut self,
        items: &[antares::domain::items::types::Item],
    ) -> Vec<(String, antares::domain::types::ItemId)> {
        // Check if cache is valid
        if let Some((ref candidates, gen)) = &self.items {
            if *gen == self.items_generation {
                return candidates.clone();
            }
        }

        // Generate new candidates
        let candidates = extract_item_candidates(items);
        self.items = Some((candidates.clone(), self.items_generation));
        candidates
    }

    /// Gets or generates monster candidates.
    pub fn get_or_generate_monsters(
        &mut self,
        monsters: &[antares::domain::combat::database::MonsterDefinition],
    ) -> Vec<String> {
        if let Some((ref candidates, gen)) = &self.monsters {
            if *gen == self.monsters_generation {
                return candidates.clone();
            }
        }

        let candidates = extract_monster_candidates(monsters);
        self.monsters = Some((candidates.clone(), self.monsters_generation));
        candidates
    }

    /// Gets or generates condition candidates.
    pub fn get_or_generate_conditions(
        &mut self,
        conditions: &[antares::domain::conditions::ConditionDefinition],
    ) -> Vec<(String, String)> {
        if let Some((ref candidates, gen)) = &self.conditions {
            if *gen == self.conditions_generation {
                return candidates.clone();
            }
        }

        let candidates = extract_condition_candidates(conditions);
        self.conditions = Some((candidates.clone(), self.conditions_generation));
        candidates
    }

    /// Gets or generates spell candidates.
    pub fn get_or_generate_spells(
        &mut self,
        spells: &[antares::domain::magic::types::Spell],
    ) -> Vec<(String, antares::domain::types::SpellId)> {
        if let Some((ref candidates, gen)) = &self.spells {
            if *gen == self.spells_generation {
                return candidates.clone();
            }
        }

        let candidates = extract_spell_candidates(spells);
        self.spells = Some((candidates.clone(), self.spells_generation));
        candidates
    }

    /// Gets or generates proficiency candidates.
    pub fn get_or_generate_proficiencies(
        &mut self,
        proficiencies: &[antares::domain::proficiency::ProficiencyDefinition],
    ) -> Vec<(String, String)> {
        if let Some((ref candidates, gen)) = &self.proficiencies {
            if *gen == self.proficiencies_generation {
                return candidates.clone();
            }
        }

        let candidates = extract_proficiency_candidates(proficiencies);
        self.proficiencies = Some((candidates.clone(), self.proficiencies_generation));
        candidates
    }
}

// =============================================================================
// Spell Selector
// =============================================================================

/// Shows an autocomplete input for selecting a spell by name.
///
/// Returns `true` if the selection changed (user selected a spell from suggestions).
///
/// # Arguments
///
/// * `ui` - The egui UI context
/// * `id_salt` - Unique ID salt for this widget instance
/// * `label` - Label to display before the input field
/// * `selected_spell_id` - Mutable reference to the currently selected [`SpellId`]
///   (value `0` means no spell selected)
/// * `spells` - Slice of available spells to autocomplete against
///
/// # Returns
///
/// `true` if the user selected a different spell (or cleared the selection),
/// `false` if nothing changed this frame.
///
/// # Examples
///
/// ```no_run
/// use eframe::egui;
/// use antares::domain::magic::types::Spell;
/// use antares::domain::types::SpellId;
/// use campaign_builder::ui_helpers::autocomplete_spell_selector;
///
/// fn show_spell_picker(ui: &mut egui::Ui, selected: &mut SpellId, spells: &[Spell]) {
///     if autocomplete_spell_selector(ui, "spell_picker", "Spell:", selected, spells) {
///         // User changed the selected spell
///     }
/// }
/// ```
pub fn autocomplete_spell_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_spell_id: &mut antares::domain::types::SpellId,
    spells: &[antares::domain::magic::types::Spell],
) -> bool {
    let candidates: Vec<String> = spells.iter().map(|s| s.name.clone()).collect();
    let current_name = if *selected_spell_id == 0 {
        String::new()
    } else {
        spells
            .iter()
            .find(|s| s.id == *selected_spell_id)
            .map(|s| s.name.clone())
            .unwrap_or_default()
    };
    // Use Cell so both on_select and on_clear can share mutation without conflicting borrows.
    let cell = std::cell::Cell::new(*selected_spell_id);
    let is_selected = *selected_spell_id != 0;
    let cfg = AutocompleteSelectorConfig {
        id_salt,
        buffer_tag: "spell",
        label,
        placeholder: "Start typing spell name...",
    };
    let changed = autocomplete_entity_selector_generic(
        ui,
        &cfg,
        candidates,
        current_name,
        is_selected,
        |text| {
            if let Some(spell) = spells.iter().find(|s| s.name == text) {
                cell.set(spell.id);
                true
            } else {
                false
            }
        },
        || cell.set(0),
    );
    *selected_spell_id = cell.get();
    changed
}
