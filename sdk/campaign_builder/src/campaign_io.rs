// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign I/O — load, save, validate, and campaign lifecycle methods for
//! [`CampaignBuilderApp`].
//!
//! This module contains all methods responsible for reading from and writing to
//! the filesystem: loading/saving campaign data files, validating campaign
//! content, managing the campaign lifecycle (new, open, save, save-as), and
//! building the campaign file tree.

use super::*;

/// Errors produced by campaign I/O operations (save/write to the filesystem).
#[derive(Debug, thiserror::Error)]
pub enum CampaignIoError {
    /// No campaign directory has been set.
    #[error("No campaign directory set")]
    NoCampaignDir,
    /// A parent directory could not be created.
    #[error("Failed to create directory: {0}")]
    CreateDirectoryFailed(String),
    /// RON serialisation of campaign data failed.
    #[error("Failed to serialize data: {0}")]
    SerializationFailed(String),
    /// Writing the data file to disk failed.
    #[error("Failed to write file: {0}")]
    WriteFileFailed(String),
}

/// Reads a RON file and parses it as `Vec<T>`.
///
/// Returns `Some(Vec<T>)` on success and updates `status_message`.
/// Returns `None` on any I/O or parse error, updating `status_message` with
/// a description of the failure.
///
/// This is a private helper shared by the various `load_X` methods on
/// [`CampaignBuilderApp`] to avoid duplicating the file-read / parse /
/// error-message pattern.
///
/// # Arguments
///
/// * `campaign_dir` - Optional campaign directory; `None` → returns `None`
/// * `filename` - Relative filename within the campaign directory
/// * `type_label` - Human-readable label used in status messages, e.g. `"items"`
/// * `status_message` - Updated with success/failure info
fn read_ron_collection<T: serde::de::DeserializeOwned>(
    campaign_dir: &Option<PathBuf>,
    filename: &str,
    type_label: &str,
    status_message: &mut String,
) -> Option<Vec<T>> {
    let dir = campaign_dir.as_ref()?;
    let path = dir.join(filename);
    if !path.exists() {
        return None;
    }
    match fs::read_to_string(&path) {
        Ok(contents) => match ron::from_str::<Vec<T>>(&contents) {
            Ok(data) => Some(data),
            Err(e) => {
                *status_message = format!("Failed to parse {}: {}", type_label, e);
                None
            }
        },
        Err(e) => {
            *status_message = format!("Failed to read {} file: {}", type_label, e);
            None
        }
    }
}

/// Serialises `data` as pretty-printed RON and writes it to
/// `<campaign_dir>/<filename>`.
///
/// Returns `Ok(())` on success or `Err(String)` with an error description.
///
/// This is a private helper shared by the various `save_X` methods on
/// [`CampaignBuilderApp`] to avoid duplicating the dir-create / serialize /
/// write pattern.
///
/// # Arguments
///
/// * `campaign_dir` - Optional campaign directory; `None` → `Err("No campaign directory set")`
/// * `filename` - Relative filename within the campaign directory
/// * `data` - The slice to serialise
/// * `type_label` - Human-readable label used in error messages, e.g. `"items"`
fn write_ron_collection<T: serde::Serialize>(
    campaign_dir: &Option<PathBuf>,
    filename: &str,
    data: &[T],
    type_label: &str,
) -> Result<(), CampaignIoError> {
    let dir = campaign_dir
        .as_ref()
        .ok_or(CampaignIoError::NoCampaignDir)?;
    let path = dir.join(filename);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            CampaignIoError::CreateDirectoryFailed(format!("{}: {}", type_label, e))
        })?;
    }
    let ron_config = ron::ser::PrettyConfig::new()
        .struct_names(false)
        .enumerate_arrays(false);
    let contents = ron::ser::to_string_pretty(data, ron_config)
        .map_err(|e| CampaignIoError::SerializationFailed(format!("{}: {}", type_label, e)))?;
    fs::write(&path, contents)
        .map_err(|e| CampaignIoError::WriteFileFailed(format!("{}: {}", type_label, e)))
}

impl CampaignBuilderApp {
    /// Handle a request emitted by the Maps editor to open the NPC editor for a
    /// specific NPC ID.
    ///
    /// If the Maps editor set `requested_open_npc`, attempt to find the NPC in
    /// the loaded NPC definitions and, if found, switch to the NPCs tab and
    /// start editing that NPC. If the NPC isn't found, set an informative
    /// `status_message` for the user.
    pub(crate) fn handle_maps_open_npc_request(&mut self) {
        if let Some(requested_id) = self
            .editor_registry
            .maps_editor_state
            .requested_open_npc
            .take()
        {
            if let Some(idx) = self
                .editor_registry
                .npc_editor_state
                .npcs
                .iter()
                .position(|n| n.id == requested_id)
            {
                self.ui_state.active_tab = EditorTab::NPCs;
                self.editor_registry.npc_editor_state.start_edit_npc(idx);
                self.ui_state.status_message = format!("Opening NPC editor for '{}'", requested_id);
            } else {
                self.ui_state.status_message = format!("NPC '{}' not found", requested_id);
            }
        }
    }

    /// Synchronize importer state that depends on the active campaign.
    pub(crate) fn sync_obj_importer_campaign_state(&mut self) {
        if let Some(campaign_dir) = self.campaign_dir.clone() {
            match self.obj_importer_state.load_custom_palette(&campaign_dir) {
                Ok(()) => {
                    if !self.obj_importer_state.custom_palette.colors.is_empty() {
                        self.logger.debug(
                            category::FILE_IO,
                            &format!(
                                "Loaded {} importer palette colors",
                                self.obj_importer_state.custom_palette.colors.len()
                            ),
                        );
                    }
                }
                Err(error) => {
                    self.logger.warn(
                        category::FILE_IO,
                        &format!("Failed to load importer palette: {}", error),
                    );
                    self.obj_importer_state.custom_palette =
                        color_palette::CustomPalette::default();
                }
            }
        } else {
            self.obj_importer_state.custom_palette = color_palette::CustomPalette::default();
        }

        match self
            .next_available_creature_id_for_category(creature_id_manager::CreatureCategory::Custom)
        {
            Ok(next_id) => self.obj_importer_state.set_next_creature_id(next_id),
            Err(error) => self.logger.warn(
                category::APP,
                &format!("Failed to determine next importer creature ID: {}", error),
            ),
        }
    }

    /// Validate item IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    pub(crate) fn validate_item_ids(&self) -> Vec<validation::ValidationResult> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for item in &self.campaign_data.items {
            if !seen_ids.insert(item.id) {
                errors.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Items,
                    format!("Duplicate item ID: {}", item.id),
                ));
            }
        }
        errors
    }

    /// Validate spell IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    pub(crate) fn validate_spell_ids(&self) -> Vec<validation::ValidationResult> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for spell in &self.campaign_data.spells {
            if !seen_ids.insert(spell.id) {
                errors.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Spells,
                    format!("Duplicate spell ID: {}", spell.id),
                ));
            }
        }
        errors
    }

    /// Validate monster IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    pub(crate) fn validate_monster_ids(&self) -> Vec<validation::ValidationResult> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for monster in &self.campaign_data.monsters {
            if !seen_ids.insert(monster.id) {
                errors.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Monsters,
                    format!("Duplicate monster ID: {}", monster.id),
                ));
            }
        }
        errors
    }

    /// Validate map IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    pub(crate) fn validate_map_ids(&self) -> Vec<validation::ValidationResult> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for map in &self.campaign_data.maps {
            if !seen_ids.insert(map.id) {
                errors.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Maps,
                    format!("Duplicate map ID: {}", map.id),
                ));
            }
        }
        errors
    }

    /// Validate condition IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    pub(crate) fn validate_condition_ids(&self) -> Vec<validation::ValidationResult> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for cond in &self.campaign_data.conditions {
            if !seen_ids.insert(cond.id.clone()) {
                errors.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Conditions,
                    format!("Duplicate condition ID: {}", cond.id),
                ));
            }
        }
        errors
    }

    /// Validate NPC IDs for uniqueness
    ///
    /// Returns validation errors for any duplicate IDs found.
    pub(crate) fn validate_npc_ids(&self) -> Vec<validation::ValidationResult> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for npc in &self.editor_registry.npc_editor_state.npcs {
            if !seen_ids.insert(npc.id.clone()) {
                errors.push(validation::ValidationResult::error(
                    validation::ValidationCategory::NPCs,
                    format!("Duplicate NPC ID: {}", npc.id),
                ));
            }

            // Cross-check stock_template reference against loaded templates
            if let Some(ref tmpl_id) = npc.stock_template {
                if !self
                    .campaign_data
                    .stock_templates
                    .iter()
                    .any(|t| &t.id == tmpl_id)
                {
                    errors.push(validation::ValidationResult::error(
                        validation::ValidationCategory::NPCs,
                        format!(
                            "NPC '{}' references unknown stock template '{}'",
                            npc.id, tmpl_id
                        ),
                    ));
                }
            }
        }

        errors.extend(self.validate_merchant_dialogue_rules());
        errors
    }

    /// Validate merchant dialogue contract consistency across NPCs and dialogues.
    ///
    /// Rules checked:
    /// - merchant NPC with no dialogue_id
    /// - merchant NPC with missing dialogue tree
    /// - merchant NPC whose dialogue is missing explicit OpenMerchant
    /// - merchant NPC whose assigned OpenMerchant targets the wrong npc_id
    /// - non-merchant NPC whose assigned dialogue still contains SDK-managed merchant content
    pub(crate) fn validate_merchant_dialogue_rules(&self) -> Vec<validation::ValidationResult> {
        let mut results = Vec::new();

        for npc in &self.editor_registry.npc_editor_state.npcs {
            let assigned_dialogue = npc.dialogue_id.and_then(|dialogue_id| {
                self.campaign_data
                    .dialogues
                    .iter()
                    .find(|dialogue| dialogue.id == dialogue_id)
            });

            if npc.is_merchant {
                let Some(dialogue_id) = npc.dialogue_id else {
                    results.push(
                        validation::ValidationResult::error(
                            validation::ValidationCategory::NPCs,
                            format!("Merchant NPC '{}' has no dialogue assigned", npc.id),
                        )
                        .with_file_path(&self.campaign.npcs_file),
                    );
                    continue;
                };

                let Some(dialogue) = assigned_dialogue else {
                    results.push(
                        validation::ValidationResult::error(
                            validation::ValidationCategory::NPCs,
                            format!(
                                "Merchant NPC '{}' references missing dialogue {}",
                                npc.id, dialogue_id
                            ),
                        )
                        .with_file_path(&self.campaign.npcs_file),
                    );
                    continue;
                };

                if dialogue.contains_open_merchant_for_npc(&npc.id) {
                    continue;
                }

                let opens_other_merchant = dialogue.nodes.values().any(|node| {
                    node.actions.iter().any(|action| {
                        matches!(
                            action,
                            DialogueAction::OpenMerchant { npc_id } if npc_id != &npc.id
                        )
                    }) || node.choices.iter().any(|choice| {
                        choice.actions.iter().any(|action| {
                            matches!(
                                action,
                                DialogueAction::OpenMerchant { npc_id } if npc_id != &npc.id
                            )
                        })
                    })
                });

                if opens_other_merchant {
                    results.push(
                        validation::ValidationResult::error(
                            validation::ValidationCategory::NPCs,
                            format!(
                                "Merchant NPC '{}' uses dialogue {} that opens the wrong merchant target",
                                npc.id, dialogue.id
                            ),
                        )
                        .with_file_path(&self.campaign.dialogue_file),
                    );
                } else {
                    results.push(
                        validation::ValidationResult::error(
                            validation::ValidationCategory::NPCs,
                            format!(
                                "Merchant NPC '{}' uses dialogue {} that is missing explicit OpenMerchant",
                                npc.id, dialogue.id
                            ),
                        )
                        .with_file_path(&self.campaign.dialogue_file),
                    );
                }
            } else if let Some(dialogue) = assigned_dialogue {
                if dialogue.has_sdk_managed_merchant_content() {
                    results.push(
                        validation::ValidationResult::warning(
                            validation::ValidationCategory::NPCs,
                            format!(
                                "Non-merchant NPC '{}' still references dialogue {} with SDK-managed merchant content",
                                npc.id, dialogue.id
                            ),
                        )
                        .with_file_path(&self.campaign.dialogue_file),
                    );
                }
            }
        }

        results
    }

    /// Applies merchant dialogue repairs across loaded NPC and dialogue content.
    ///
    /// Returns a summary validation result describing the number of repairs applied.
    pub(crate) fn repair_merchant_dialogue_validation_issues(
        &mut self,
    ) -> validation::ValidationResult {
        self.editor_registry.npc_editor_state.available_dialogues =
            self.campaign_data.dialogues.clone();
        self.editor_registry
            .npc_editor_state
            .merchant_dialogue_editor
            .load_dialogues(self.campaign_data.dialogues.clone());

        let mut repaired_count = 0usize;

        for npc_index in 0..self.editor_registry.npc_editor_state.npcs.len() {
            let npc = self.editor_registry.npc_editor_state.npcs[npc_index].clone();

            self.editor_registry.npc_editor_state.edit_buffer.id = npc.id.clone();
            self.editor_registry.npc_editor_state.edit_buffer.name = npc.name.clone();
            self.editor_registry
                .npc_editor_state
                .edit_buffer
                .description = npc.description.clone();
            self.editor_registry
                .npc_editor_state
                .edit_buffer
                .portrait_id = npc.portrait_id.clone();
            self.editor_registry
                .npc_editor_state
                .edit_buffer
                .dialogue_id = npc.dialogue_id.map(|id| id.to_string()).unwrap_or_default();
            self.editor_registry.npc_editor_state.edit_buffer.quest_ids =
                npc.quest_ids.iter().map(|id| id.to_string()).collect();
            self.editor_registry.npc_editor_state.edit_buffer.faction =
                npc.faction.clone().unwrap_or_default();
            self.editor_registry
                .npc_editor_state
                .edit_buffer
                .is_merchant = npc.is_merchant;
            self.editor_registry
                .npc_editor_state
                .edit_buffer
                .is_innkeeper = npc.is_innkeeper;
            self.editor_registry
                .npc_editor_state
                .edit_buffer
                .creature_id = npc.creature_id.map(|id| id.to_string()).unwrap_or_default();
            self.editor_registry
                .npc_editor_state
                .edit_buffer
                .sprite_sheet = npc
                .sprite
                .as_ref()
                .map(|sprite| sprite.sheet_path.clone())
                .unwrap_or_default();
            self.editor_registry
                .npc_editor_state
                .edit_buffer
                .sprite_index = npc
                .sprite
                .as_ref()
                .map(|sprite| sprite.sprite_index.to_string())
                .unwrap_or_default();
            self.editor_registry
                .npc_editor_state
                .edit_buffer
                .stock_template = npc.stock_template.clone().unwrap_or_default();

            let needs_repair = self
                .editor_registry
                .npc_editor_state
                .merchant_dialogue_repair_action_for_buffer()
                .is_some();

            if !needs_repair {
                continue;
            }

            if let Ok(message) = self
                .editor_registry
                .npc_editor_state
                .repair_merchant_dialogue_for_buffer()
            {
                if !message.is_empty() {
                    repaired_count += 1;
                }
            }

            if let Ok(repaired_npc) = self
                .editor_registry
                .npc_editor_state
                .build_npc_from_edit_buffer(
                    self.editor_registry
                        .npc_editor_state
                        .edit_buffer
                        .is_merchant,
                )
            {
                self.editor_registry.npc_editor_state.npcs[npc_index] = repaired_npc;
            }
        }

        self.campaign_data.dialogues = self
            .editor_registry
            .npc_editor_state
            .available_dialogues
            .clone();
        self.editor_registry
            .dialogue_editor_state
            .load_dialogues(self.campaign_data.dialogues.clone());

        if repaired_count > 0 {
            self.unsaved_changes = true;
            validation::ValidationResult::info(
                validation::ValidationCategory::NPCs,
                format!(
                    "Applied merchant dialogue repairs to {} NPC(s); save NPCs and dialogues to persist changes",
                    repaired_count
                ),
            )
        } else {
            validation::ValidationResult::passed(
                validation::ValidationCategory::NPCs,
                "No merchant dialogue repairs were needed",
            )
        }
    }

    /// Validate stock template item ID references against the loaded item list.
    ///
    /// Returns warnings for template entries and magic pool entries whose item
    /// IDs do not exist in the loaded `items` list.
    pub(crate) fn validate_stock_template_refs(&self) -> Vec<validation::ValidationResult> {
        let mut results = Vec::new();

        for tmpl in &self.campaign_data.stock_templates {
            for entry in &tmpl.entries {
                if !self
                    .campaign_data
                    .items
                    .iter()
                    .any(|it| it.id == entry.item_id)
                {
                    results.push(validation::ValidationResult::warning(
                        validation::ValidationCategory::NPCs,
                        format!(
                            "Template '{}' references unknown item_id {}",
                            tmpl.id, entry.item_id
                        ),
                    ));
                }
            }

            for &pool_id in &tmpl.magic_item_pool {
                if !self.campaign_data.items.iter().any(|it| it.id == pool_id) {
                    results.push(validation::ValidationResult::warning(
                        validation::ValidationCategory::NPCs,
                        format!(
                            "Template '{}' magic pool references unknown item_id {}",
                            tmpl.id, pool_id
                        ),
                    ));
                }
            }
        }

        results
    }

    /// Validate character IDs for uniqueness and references
    ///
    /// Returns validation errors for:
    /// - Duplicate character IDs
    /// - Empty character IDs
    /// - Empty character names (warning)
    /// - Non-existent class references
    /// - Non-existent race references
    pub(crate) fn validate_character_ids(&self) -> Vec<validation::ValidationResult> {
        let mut results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for character in &self.editor_registry.characters_editor_state.characters {
            // Check for duplicate IDs
            if !seen_ids.insert(character.id.clone()) {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Characters,
                    format!("Duplicate character ID: '{}'", character.id),
                ));
            }

            // Check for empty IDs
            if character.id.is_empty() {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Characters,
                    "Character has empty ID",
                ));
            }

            // Check for empty names
            if character.name.is_empty() {
                results.push(validation::ValidationResult::warning(
                    validation::ValidationCategory::Characters,
                    format!("Character '{}' has empty name", character.id),
                ));
            }

            // Validate class exists
            let class_exists = self
                .editor_registry
                .classes_editor_state
                .classes
                .iter()
                .any(|c| c.id == character.class_id);
            if !class_exists && !character.class_id.is_empty() {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Characters,
                    format!(
                        "Character '{}' references non-existent class '{}'",
                        character.id, character.class_id
                    ),
                ));
            }

            // Validate race exists
            let race_exists = self
                .editor_registry
                .races_editor_state
                .races
                .iter()
                .any(|r| r.id == character.race_id);
            if !race_exists && !character.race_id.is_empty() {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Characters,
                    format!(
                        "Character '{}' references non-existent race '{}'",
                        character.id, character.race_id
                    ),
                ));
            }
        }

        // Add passed message if no characters or all valid
        if self
            .editor_registry
            .characters_editor_state
            .characters
            .is_empty()
        {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Characters,
                "No characters defined",
            ));
        } else if results
            .iter()
            .all(|r| r.severity != validation::ValidationSeverity::Error)
        {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Characters,
                format!(
                    "{} character(s) validated",
                    self.editor_registry
                        .characters_editor_state
                        .characters
                        .len()
                ),
            ));
        }

        results
    }

    /// Validate proficiency IDs for uniqueness and cross-references
    ///
    /// Returns validation errors for:
    /// - Duplicate proficiency IDs
    /// - Empty proficiency IDs
    /// - Empty proficiency names (warning)
    /// - Proficiencies referenced by classes that don't exist
    /// - Proficiencies referenced by races that don't exist
    /// - Proficiencies required by items that don't exist
    /// - Info messages for unreferenced proficiencies
    pub(crate) fn validate_proficiency_ids(&self) -> Vec<validation::ValidationResult> {
        let mut results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for proficiency in &self.campaign_data.proficiencies {
            // Check for duplicate IDs
            if !seen_ids.insert(proficiency.id.clone()) {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Proficiencies,
                    format!("Duplicate proficiency ID: '{}'", proficiency.id),
                ));
            }

            // Check for empty IDs
            if proficiency.id.is_empty() {
                results.push(validation::ValidationResult::error(
                    validation::ValidationCategory::Proficiencies,
                    "Proficiency has empty ID",
                ));
            }

            // Check for empty names
            if proficiency.name.is_empty() {
                results.push(validation::ValidationResult::warning(
                    validation::ValidationCategory::Proficiencies,
                    format!("Proficiency '{}' has empty name", proficiency.id),
                ));
            }
        }

        // Cross-reference validation: Check for proficiencies referenced by classes
        let mut referenced_proficiencies = std::collections::HashSet::new();
        for class in &self.editor_registry.classes_editor_state.classes {
            for prof_id in &class.proficiencies {
                referenced_proficiencies.insert(prof_id.clone());

                let prof_exists = self
                    .campaign_data
                    .proficiencies
                    .iter()
                    .any(|p| &p.id == prof_id);
                if !prof_exists {
                    results.push(validation::ValidationResult::error(
                        validation::ValidationCategory::Proficiencies,
                        format!(
                            "Class '{}' references non-existent proficiency '{}'",
                            class.id, prof_id
                        ),
                    ));
                }
            }
        }

        // Cross-reference validation: Check for proficiencies referenced by races
        for race in &self.editor_registry.races_editor_state.races {
            for prof_id in &race.proficiencies {
                referenced_proficiencies.insert(prof_id.clone());

                let prof_exists = self
                    .campaign_data
                    .proficiencies
                    .iter()
                    .any(|p| &p.id == prof_id);
                if !prof_exists {
                    results.push(validation::ValidationResult::error(
                        validation::ValidationCategory::Proficiencies,
                        format!(
                            "Race '{}' references non-existent proficiency '{}'",
                            race.id, prof_id
                        ),
                    ));
                }
            }
        }

        // Cross-reference validation: Check for proficiencies required by items
        for item in &self.campaign_data.items {
            if let Some(required_prof) = item.required_proficiency() {
                referenced_proficiencies.insert(required_prof.clone());

                let prof_exists = self
                    .campaign_data
                    .proficiencies
                    .iter()
                    .any(|p| p.id == required_prof);
                if !prof_exists {
                    results.push(validation::ValidationResult::error(
                        validation::ValidationCategory::Proficiencies,
                        format!(
                            "Item '{}' requires non-existent proficiency '{}'",
                            item.id, required_prof
                        ),
                    ));
                }
            }
        }

        // Warning for unreferenced proficiencies
        for proficiency in &self.campaign_data.proficiencies {
            if !referenced_proficiencies.contains(&proficiency.id) {
                results.push(validation::ValidationResult::info(
                    validation::ValidationCategory::Proficiencies,
                    format!(
                        "Proficiency '{}' is not used by any class, race, or item",
                        proficiency.id
                    ),
                ));
            }
        }

        // Add passed message if no proficiencies or all valid
        if self.campaign_data.proficiencies.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Proficiencies,
                "No proficiencies defined",
            ));
        } else if results
            .iter()
            .all(|r| r.severity != validation::ValidationSeverity::Error)
        {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Proficiencies,
                format!(
                    "{} proficiency(ies) validated",
                    self.campaign_data.proficiencies.len()
                ),
            ));
        }

        results
    }

    /// Generate category status checks (passed or no data info messages)
    ///
    /// This function checks each data category and adds:
    /// - ✅ Passed check if data exists and has no errors
    /// - ℹ️ Info message if no data is loaded for the category
    pub(crate) fn generate_category_status_checks(&self) -> Vec<validation::ValidationResult> {
        let mut results = Vec::new();

        // Items category
        if self.campaign_data.items.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Items,
                "No items loaded - add items or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Items,
                format!("{} items validated", self.campaign_data.items.len()),
            ));
        }

        // Spells category
        if self.campaign_data.spells.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Spells,
                "No spells loaded - add spells or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Spells,
                format!("{} spells validated", self.campaign_data.spells.len()),
            ));
        }

        // Monsters category
        if self.campaign_data.monsters.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Monsters,
                "No monsters loaded - add monsters or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Monsters,
                format!("{} monsters validated", self.campaign_data.monsters.len()),
            ));
        }

        // Maps category
        if self.campaign_data.maps.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Maps,
                "No maps loaded - create maps in the Maps editor",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Maps,
                format!("{} maps validated", self.campaign_data.maps.len()),
            ));
        }

        // Conditions category
        if self.campaign_data.conditions.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Conditions,
                "No conditions loaded - add conditions or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Conditions,
                format!(
                    "{} conditions validated",
                    self.campaign_data.conditions.len()
                ),
            ));
        }

        // Quests category
        if self.campaign_data.quests.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Quests,
                "No quests loaded - add quests or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Quests,
                format!("{} quests validated", self.campaign_data.quests.len()),
            ));
        }

        // Dialogues category
        if self.campaign_data.dialogues.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Dialogues,
                "No dialogues loaded - add dialogues or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Dialogues,
                format!("{} dialogues validated", self.campaign_data.dialogues.len()),
            ));
        }

        // NPCs category
        if self.editor_registry.npc_editor_state.npcs.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::NPCs,
                "No NPCs loaded - add NPCs or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::NPCs,
                format!(
                    "{} NPCs validated",
                    self.editor_registry.npc_editor_state.npcs.len()
                ),
            ));
        }

        // Classes category
        if self.editor_registry.classes_editor_state.classes.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Classes,
                "No classes loaded - add classes or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Classes,
                format!(
                    "{} classes validated",
                    self.editor_registry.classes_editor_state.classes.len()
                ),
            ));
        }

        // Races category
        if self.editor_registry.races_editor_state.races.is_empty() {
            results.push(validation::ValidationResult::info(
                validation::ValidationCategory::Races,
                "No races loaded - add races or load from file",
            ));
        } else {
            results.push(validation::ValidationResult::passed(
                validation::ValidationCategory::Races,
                format!(
                    "{} races validated",
                    self.editor_registry.races_editor_state.races.len()
                ),
            ));
        }

        results
    }

    /// Groups and filters validation results according to the UI filter settings and
    /// returns only categories that have results after applying the filters.
    ///
    /// This variation returns owned `ValidationResult` objects (cloned) to avoid
    /// lifetime/borrow conflicts with UI closures that can overlap `&mut self`
    /// borrows. Cloning is acceptable for this UI-only structure and keeps the
    /// UI code simpler and safer.
    pub(crate) fn grouped_filtered_validation_results(
        &self,
    ) -> Vec<(
        validation::ValidationCategory,
        Vec<validation::ValidationResult>,
    )> {
        use std::collections::HashMap;

        // Bucket results by category after applying UI filters.
        let mut buckets: HashMap<
            validation::ValidationCategory,
            Vec<validation::ValidationResult>,
        > = HashMap::new();

        for res in &self.validation_state.validation_errors {
            // Apply active severity filter
            let should_show = match self.validation_state.validation_filter {
                ValidationFilter::All => true,
                ValidationFilter::ErrorsOnly => {
                    res.severity == validation::ValidationSeverity::Error
                }
                ValidationFilter::WarningsOnly => {
                    res.severity == validation::ValidationSeverity::Warning
                }
            };

            if !should_show {
                continue;
            }

            // Clone the result into the bucket, providing an owned copy for UI use.
            buckets.entry(res.category).or_default().push(res.clone());
        }

        // Convert into a Vec ordered by category display order.
        let mut result: Vec<(
            validation::ValidationCategory,
            Vec<validation::ValidationResult>,
        )> = Vec::new();

        for category in validation::ValidationCategory::all() {
            if let Some(group) = buckets.remove(&category) {
                if !group.is_empty() {
                    result.push((category, group));
                }
            }
        }

        result
    }

    /// Discovers map files in the maps directory by scanning for .ron files
    ///
    /// This function scans the actual maps directory and returns paths to all .ron files found,
    /// rather than inferring filenames from map IDs. This allows maps to have any filename.
    ///
    /// # Returns
    ///
    /// A vector of map file paths relative to the campaign directory.
    pub(crate) fn discover_map_files(&self) -> Vec<String> {
        let mut map_files = Vec::new();

        if let Some(ref campaign_dir) = self.campaign_dir {
            let maps_path = campaign_dir.join(&self.campaign.maps_dir);

            if let Ok(entries) = std::fs::read_dir(&maps_path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                        // Store relative path from campaign dir
                        if let Ok(rel_path) = path.strip_prefix(campaign_dir) {
                            map_files.push(rel_path.display().to_string());
                        }
                    }
                }
            }

            // Sort for consistent ordering
            map_files.sort();
        }

        map_files
    }

    /// Load items from RON file
    pub(crate) fn load_items(&mut self) {
        self.logger.debug(category::FILE_IO, "load_items() called");
        let items_file = self.campaign.items_file.clone();
        self.logger
            .verbose(category::FILE_IO, "Loading items from campaign dir");
        if let Some(items) = read_ron_collection::<Item>(
            &self.campaign_dir,
            &items_file,
            "items",
            &mut self.ui_state.status_message,
        ) {
            let count = items.len();
            self.campaign_data.items = items;
            if let Some(ref mut manager) = self.asset_manager {
                manager.mark_data_file_loaded(&items_file, count);
            }
            let id_errors = self.validate_item_ids();
            if !id_errors.is_empty() {
                self.validation_state
                    .validation_errors
                    .extend(id_errors.clone());
                self.logger.warn(
                    category::DATA,
                    &format!(
                        "Loaded {} items with {} ID conflicts",
                        self.campaign_data.items.len(),
                        id_errors.len()
                    ),
                );
                self.ui_state.status_message = format!(
                    "⚠️ Loaded {} items with {} ID conflicts",
                    self.campaign_data.items.len(),
                    id_errors.len()
                );
            } else {
                self.logger.info(
                    category::FILE_IO,
                    &format!("Loaded {} items", self.campaign_data.items.len()),
                );
                self.ui_state.status_message =
                    format!("Loaded {} items", self.campaign_data.items.len());
            }
        } else {
            // read_ron_collection already set self.ui_state.status_message on error
            if let Some(ref mut manager) = self.asset_manager {
                manager.mark_data_file_error(&items_file, &self.ui_state.status_message.clone());
            }
            self.logger.error(
                category::FILE_IO,
                &format!("Failed to load items: {}", self.ui_state.status_message),
            );
        }
    }

    /// Save items to RON file
    pub(crate) fn save_items(&mut self) -> Result<(), CampaignIoError> {
        self.logger.debug(category::FILE_IO, "save_items() called");
        let mut sorted = self.campaign_data.items.clone();
        sorted.sort_by_key(|i| i.id);
        write_ron_collection(
            &self.campaign_dir,
            &self.campaign.items_file,
            &sorted,
            "items",
        )?;
        self.logger.info(
            category::FILE_IO,
            &format!("Saved {} items", self.campaign_data.items.len()),
        );
        self.unsaved_changes = true;
        Ok(())
    }

    /// Load spells from RON file
    pub(crate) fn load_spells(&mut self) {
        let spells_file = self.campaign.spells_file.clone();
        if let Some(spells) = read_ron_collection::<Spell>(
            &self.campaign_dir,
            &spells_file,
            "spells",
            &mut self.ui_state.status_message,
        ) {
            let count = spells.len();
            self.campaign_data.spells = spells;
            if let Some(ref mut manager) = self.asset_manager {
                manager.mark_data_file_loaded(&spells_file, count);
            }
            let id_errors = self.validate_spell_ids();
            if !id_errors.is_empty() {
                self.validation_state
                    .validation_errors
                    .extend(id_errors.clone());
                self.ui_state.status_message = format!(
                    "⚠️ Loaded {} spells with {} ID conflicts",
                    self.campaign_data.spells.len(),
                    id_errors.len()
                );
            } else {
                self.ui_state.status_message =
                    format!("Loaded {} spells", self.campaign_data.spells.len());
            }
        }
    }

    /// Save spells to RON file
    pub(crate) fn save_spells(&mut self) -> Result<(), CampaignIoError> {
        let mut sorted = self.campaign_data.spells.clone();
        sorted.sort_by_key(|s| s.id);
        write_ron_collection(
            &self.campaign_dir,
            &self.campaign.spells_file,
            &sorted,
            "spells",
        )?;
        self.unsaved_changes = true;
        Ok(())
    }

    /// Load conditions from RON file
    pub(crate) fn load_conditions(&mut self) {
        let conditions_file = self.campaign.conditions_file.clone();
        if let Some(conditions) = read_ron_collection::<ConditionDefinition>(
            &self.campaign_dir,
            &conditions_file,
            "conditions",
            &mut self.ui_state.status_message,
        ) {
            self.campaign_data.conditions = conditions;
            let id_errors = self.validate_condition_ids();
            if !id_errors.is_empty() {
                self.validation_state
                    .validation_errors
                    .extend(id_errors.clone());
                self.ui_state.status_message = format!(
                    "⚠️ Loaded {} conditions with {} ID conflicts",
                    self.campaign_data.conditions.len(),
                    id_errors.len()
                );
            } else {
                self.ui_state.status_message =
                    format!("Loaded {} conditions", self.campaign_data.conditions.len());
            }
        }
    }

    /// Save conditions to RON file
    pub(crate) fn save_conditions(&mut self) -> Result<(), CampaignIoError> {
        let mut sorted = self.campaign_data.conditions.clone();
        sorted.sort_by(|a, b| a.id.cmp(&b.id));
        write_ron_collection(
            &self.campaign_dir,
            &self.campaign.conditions_file,
            &sorted,
            "conditions",
        )?;
        self.unsaved_changes = true;
        Ok(())
    }

    /// Load proficiencies from RON file
    pub(crate) fn load_proficiencies(&mut self) {
        self.logger
            .debug(category::FILE_IO, "load_proficiencies() called");
        let proficiencies_file = self.campaign.proficiencies_file.clone();
        if let Some(ref dir) = self.campaign_dir {
            let proficiencies_path = dir.join(&proficiencies_file);
            self.logger.verbose(
                category::FILE_IO,
                &format!(
                    "Loading proficiencies from: {}",
                    proficiencies_path.display()
                ),
            );
            if proficiencies_path.exists() {
                match fs::read_to_string(&proficiencies_path) {
                    Ok(contents) => {
                        self.logger.verbose(
                            category::FILE_IO,
                            &format!("Read {} bytes from proficiencies file", contents.len()),
                        );
                        match ron::from_str::<Vec<ProficiencyDefinition>>(&contents) {
                            Ok(proficiencies) => {
                                let count = proficiencies.len();
                                self.campaign_data.proficiencies = proficiencies;

                                // Mark data file as loaded in asset manager
                                if let Some(ref mut manager) = self.asset_manager {
                                    manager.mark_data_file_loaded(&proficiencies_file, count);
                                }

                                self.logger.info(
                                    category::FILE_IO,
                                    &format!(
                                        "Loaded {} proficiencies",
                                        self.campaign_data.proficiencies.len()
                                    ),
                                );
                                self.ui_state.status_message = format!(
                                    "Loaded {} proficiencies",
                                    self.campaign_data.proficiencies.len()
                                );
                            }
                            Err(e) => {
                                // Mark data file as error in asset manager
                                if let Some(ref mut manager) = self.asset_manager {
                                    manager
                                        .mark_data_file_error(&proficiencies_file, &e.to_string());
                                }
                                self.logger.error(
                                    category::FILE_IO,
                                    &format!("Failed to parse proficiencies: {}", e),
                                );
                                self.ui_state.status_message =
                                    format!("Failed to parse proficiencies: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        // Mark data file as error in asset manager
                        if let Some(ref mut manager) = self.asset_manager {
                            manager.mark_data_file_error(&proficiencies_file, &e.to_string());
                        }
                        self.logger.error(
                            category::FILE_IO,
                            &format!("Failed to read proficiencies file: {}", e),
                        );
                        self.ui_state.status_message =
                            format!("Failed to read proficiencies file: {}", e);
                    }
                }
            } else {
                self.logger.warn(
                    category::FILE_IO,
                    &format!(
                        "Proficiencies file does not exist: {}",
                        proficiencies_path.display()
                    ),
                );
            }
        } else {
            self.logger.warn(
                category::FILE_IO,
                "No campaign directory set when trying to load proficiencies",
            );
        }
    }

    /// Save proficiencies to RON file
    pub(crate) fn save_proficiencies(&mut self) -> Result<(), CampaignIoError> {
        self.logger
            .debug(category::FILE_IO, "save_proficiencies() called");
        if let Some(ref dir) = self.campaign_dir {
            let proficiencies_path = dir.join(&self.campaign.proficiencies_file);
            self.logger.verbose(
                category::FILE_IO,
                &format!(
                    "Saving {} proficiencies to: {}",
                    self.campaign_data.proficiencies.len(),
                    proficiencies_path.display()
                ),
            );

            // Create proficiencies directory if it doesn't exist
            if let Some(parent) = proficiencies_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    CampaignIoError::CreateDirectoryFailed(format!("proficiencies: {}", e))
                })?;
            }

            let ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(false);

            // Sort by ID (String) before serializing for stable file order.
            let mut sorted_proficiencies = self.campaign_data.proficiencies.clone();
            sorted_proficiencies.sort_by(|a, b| a.id.cmp(&b.id));

            let contents =
                ron::ser::to_string_pretty(&sorted_proficiencies, ron_config).map_err(|e| {
                    CampaignIoError::SerializationFailed(format!("proficiencies: {}", e))
                })?;

            fs::write(&proficiencies_path, &contents)
                .map_err(|e| CampaignIoError::WriteFileFailed(format!("proficiencies: {}", e)))?;

            self.logger.info(
                category::FILE_IO,
                &format!(
                    "Saved {} proficiencies ({} bytes)",
                    self.campaign_data.proficiencies.len(),
                    contents.len()
                ),
            );
            self.unsaved_changes = true;
            Ok(())
        } else {
            self.logger.error(
                category::FILE_IO,
                "No campaign directory set when trying to save proficiencies",
            );
            Err(CampaignIoError::NoCampaignDir)
        }
    }

    /// Save dialogues to a file path
    ///
    /// # Arguments
    ///
    /// * `path` - The path to save dialogues to
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if save was successful
    pub(crate) fn save_dialogues_to_file(
        &self,
        path: &std::path::Path,
    ) -> Result<(), CampaignIoError> {
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| CampaignIoError::CreateDirectoryFailed(format!("dialogues: {}", e)))?;
        }

        let ron_config = ron::ser::PrettyConfig::new()
            .struct_names(false)
            .enumerate_arrays(false);

        // Sort by dialogue ID before serializing for stable file order.
        let mut sorted_dialogues = self.campaign_data.dialogues.clone();
        sorted_dialogues.sort_by_key(|d| d.id);

        let contents = ron::ser::to_string_pretty(&sorted_dialogues, ron_config)
            .map_err(|e| CampaignIoError::SerializationFailed(format!("dialogues: {}", e)))?;

        std::fs::write(path, contents)
            .map_err(|e| CampaignIoError::WriteFileFailed(format!("dialogues: {}", e)))?;

        Ok(())
    }

    /// Save NPCs to a file path
    ///
    /// # Arguments
    ///
    /// * `path` - The path to save NPCs to
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if save was successful
    pub(crate) fn save_npcs_to_file(&self, path: &std::path::Path) -> Result<(), CampaignIoError> {
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| CampaignIoError::CreateDirectoryFailed(format!("NPCs: {}", e)))?;
        }

        let ron_config = ron::ser::PrettyConfig::new()
            .struct_names(false)
            .enumerate_arrays(false);

        // Sort by NPC ID (String) before serializing for stable file order.
        let mut sorted_npcs = self.editor_registry.npc_editor_state.npcs.clone();
        sorted_npcs.sort_by(|a, b| a.id.cmp(&b.id));

        let contents = ron::ser::to_string_pretty(&sorted_npcs, ron_config)
            .map_err(|e| CampaignIoError::SerializationFailed(format!("NPCs: {}", e)))?;

        std::fs::write(path, contents)
            .map_err(|e| CampaignIoError::WriteFileFailed(format!("NPCs: {}", e)))?;

        Ok(())
    }

    /// Load NPCs from campaign file
    pub(crate) fn load_npcs(&mut self) -> Result<(), CampaignError> {
        if let Some(dir) = &self.campaign_dir {
            let npcs_path = dir.join(&self.campaign.npcs_file);

            if npcs_path.exists() {
                let contents = std::fs::read_to_string(&npcs_path).map_err(CampaignError::Io)?;

                let npcs: Vec<antares::domain::world::npc::NpcDefinition> =
                    ron::from_str(&contents).map_err(CampaignError::Deserialization)?;

                let count = npcs.len();
                self.editor_registry.npc_editor_state.npcs = npcs;
                self.logger.log(
                    LogLevel::Info,
                    category::CAMPAIGN,
                    &format!("Loaded {} NPCs", count),
                );
                // Mark data file as loaded in asset manager
                if let Some(ref mut manager) = self.asset_manager {
                    manager.mark_data_file_loaded(&self.campaign.npcs_file, count);
                }
            } else {
                self.logger.log(
                    LogLevel::Warn,
                    category::CAMPAIGN,
                    &format!("NPCs file not found: {:?}", npcs_path),
                );
            }
        }
        Ok(())
    }

    /// Load stock templates from the campaign's `npc_stock_templates.ron` file.
    ///
    /// On success, populates both `stock_templates_editor_state.templates` and
    /// the `stock_templates` mirror list.  Logs a warning (but does not fail)
    /// when the file is absent — an empty template list is a valid state for a
    /// new campaign.
    pub(crate) fn load_stock_templates(&mut self) {
        if let Some(dir) = &self.campaign_dir {
            let path = dir.join(&self.campaign.stock_templates_file);
            if path.exists() {
                match self
                    .editor_registry
                    .stock_templates_editor_state
                    .load_from_file(&path)
                {
                    Ok(()) => {
                        self.campaign_data.stock_templates = self
                            .editor_registry
                            .stock_templates_editor_state
                            .templates
                            .clone();
                        self.logger.info(
                            category::FILE_IO,
                            &format!(
                                "Loaded {} stock templates",
                                self.campaign_data.stock_templates.len()
                            ),
                        );
                        // Mark the initial-load flag as satisfied so show() does
                        // not redundantly re-read the file on first tab render.
                        self.editor_registry
                            .stock_templates_editor_state
                            .needs_initial_load = false;
                    }
                    Err(e) => {
                        self.logger.warn(
                            category::FILE_IO,
                            &format!("Failed to parse stock templates: {}", e),
                        );
                    }
                }
            } else {
                self.logger.debug(
                    category::FILE_IO,
                    &format!(
                        "Stock templates file not found (will auto-load if created later): {}",
                        path.display()
                    ),
                );
            }
        }
    }

    /// Load monsters from RON file
    pub(crate) fn load_monsters(&mut self) {
        let monsters_file = self.campaign.monsters_file.clone();
        if let Some(monsters) = read_ron_collection::<MonsterDefinition>(
            &self.campaign_dir,
            &monsters_file,
            "monsters",
            &mut self.ui_state.status_message,
        ) {
            let count = monsters.len();
            self.campaign_data.monsters = monsters;
            if let Some(ref mut manager) = self.asset_manager {
                manager.mark_data_file_loaded(&monsters_file, count);
            }
            let id_errors = self.validate_monster_ids();
            if !id_errors.is_empty() {
                self.validation_state
                    .validation_errors
                    .extend(id_errors.clone());
                self.ui_state.status_message = format!(
                    "⚠️ Loaded {} monsters with {} ID conflicts",
                    self.campaign_data.monsters.len(),
                    id_errors.len()
                );
            } else {
                self.ui_state.status_message =
                    format!("Loaded {} monsters", self.campaign_data.monsters.len());
            }
        }
    }

    /// Save monsters to RON file
    pub(crate) fn save_monsters(&mut self) -> Result<(), CampaignIoError> {
        let mut sorted = self.campaign_data.monsters.clone();
        sorted.sort_by_key(|m| m.id);
        write_ron_collection(
            &self.campaign_dir,
            &self.campaign.monsters_file,
            &sorted,
            "monsters",
        )?;
        self.unsaved_changes = true;
        Ok(())
    }

    /// Load furniture definitions from the campaign furniture RON file.
    ///
    /// Missing file is not an error — furniture support is opt-in per campaign.
    /// Syncs the loaded definitions into `furniture_editor_state` too.
    pub(crate) fn load_furniture(&mut self) {
        if let Some(defs) = read_ron_collection::<antares::domain::FurnitureDefinition>(
            &self.campaign_dir,
            &self.campaign.furniture_file,
            "furniture",
            &mut self.ui_state.status_message,
        ) {
            let count = defs.len();
            self.campaign_data.furniture_definitions = defs;
            self.editor_registry.furniture_editor_state =
                furniture_editor::FurnitureEditorState::new();
            self.logger.info(
                category::FILE_IO,
                &format!("Loaded {} furniture definitions", count),
            );
            self.ui_state.status_message = format!("Loaded {} furniture definitions", count);
        } else {
            self.campaign_data.furniture_definitions.clear();
            self.editor_registry.furniture_editor_state =
                furniture_editor::FurnitureEditorState::new();
            self.logger
                .debug(category::FILE_IO, "No furniture.ron found (opt-in)");
        }
    }

    /// Save furniture definitions to the campaign furniture RON file.
    ///
    /// Returns an `Err` on failure so the caller can aggregate warnings.
    pub(crate) fn save_furniture(&mut self) -> Result<(), CampaignIoError> {
        write_ron_collection(
            &self.campaign_dir,
            &self.campaign.furniture_file,
            &self.campaign_data.furniture_definitions,
            "furniture",
        )?;
        self.logger.info(
            category::FILE_IO,
            &format!(
                "Saved {} furniture definitions",
                self.campaign_data.furniture_definitions.len()
            ),
        );
        Ok(())
    }

    /// Load creatures from RON file
    pub(crate) fn load_creatures(&mut self) {
        let creatures_file = self.campaign.creatures_file.clone();
        if let Some(ref dir) = self.campaign_dir {
            let creatures_path = dir.join(&creatures_file);
            if creatures_path.exists() {
                match fs::read_to_string(&creatures_path) {
                    Ok(contents) => {
                        // Step 1: Parse registry file as Vec<CreatureReference>
                        match ron::from_str::<Vec<CreatureReference>>(&contents) {
                            Ok(references) => {
                                // Step 2: Load full definitions for each reference
                                let mut creatures = Vec::new();
                                let mut load_errors = Vec::new();

                                for reference in references {
                                    let creature_path = dir.join(&reference.filepath);

                                    match fs::read_to_string(&creature_path) {
                                        Ok(creature_contents) => {
                                            match ron::from_str::<
                                                antares::domain::visual::CreatureDefinition,
                                            >(
                                                &creature_contents
                                            ) {
                                                Ok(mut creature) => {
                                                    // Registry metadata is authoritative in
                                                    // registry-driven loads so one asset file can
                                                    // back multiple creature IDs.
                                                    creature.id = reference.id;
                                                    creature.name = reference.name.clone();
                                                    creatures.push(creature);
                                                }
                                                Err(e) => {
                                                    load_errors.push(format!(
                                                        "Failed to parse {}: {}",
                                                        reference.filepath, e
                                                    ));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            load_errors.push(format!(
                                                "Failed to read {}: {}",
                                                reference.filepath, e
                                            ));
                                        }
                                    }
                                }

                                if load_errors.is_empty() {
                                    let count = creatures.len();
                                    self.campaign_data.creatures = creatures;

                                    if let Some(ref mut manager) = self.asset_manager {
                                        manager.mark_data_file_loaded(&creatures_file, count);
                                    }

                                    self.ui_state.status_message =
                                        format!("Loaded {} creatures", count);
                                } else {
                                    if let Some(ref mut manager) = self.asset_manager {
                                        manager.mark_data_file_error(
                                            &creatures_file,
                                            &format!(
                                                "{} errors loading creatures",
                                                load_errors.len()
                                            ),
                                        );
                                    }

                                    self.ui_state.status_message = format!(
                                        "Loaded {} creatures with {} errors:\n{}",
                                        creatures.len(),
                                        load_errors.len(),
                                        load_errors.join("\n")
                                    );
                                    self.logger.error(
                                        category::FILE_IO,
                                        &format!(
                                            "Creature loading errors: {}",
                                            load_errors.join("\n")
                                        ),
                                    );
                                }
                            }
                            Err(e) => {
                                if let Some(ref mut manager) = self.asset_manager {
                                    manager.mark_data_file_error(&creatures_file, &e.to_string());
                                }
                                self.ui_state.status_message =
                                    format!("Failed to parse creatures registry: {}", e);
                                self.logger.error(
                                    category::FILE_IO,
                                    &format!(
                                        "Failed to parse creatures registry {:?}: {}",
                                        creatures_path, e
                                    ),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        if let Some(ref mut manager) = self.asset_manager {
                            manager.mark_data_file_error(&creatures_file, &e.to_string());
                        }
                        self.ui_state.status_message =
                            format!("Failed to read creatures registry: {}", e);
                        self.logger.error(
                            category::FILE_IO,
                            &format!(
                                "Failed to read creatures registry {:?}: {}",
                                creatures_path, e
                            ),
                        );
                    }
                }
            } else {
                self.logger.debug(
                    category::FILE_IO,
                    &format!("Creatures file does not exist: {:?}", creatures_path),
                );
            }
        } else {
            self.logger.warn(
                category::FILE_IO,
                "No campaign directory set when trying to load creatures",
            );
        }
    }

    /// Save creatures to RON file
    pub(crate) fn save_creatures(&mut self) -> Result<(), CampaignIoError> {
        if let Some(ref dir) = self.campaign_dir {
            // Step 1: Create registry entries from creatures
            let references: Vec<CreatureReference> = self
                .campaign_data
                .creatures
                .iter()
                .map(|creature| {
                    let filename = creature
                        .name
                        .to_lowercase()
                        .replace(" ", "_")
                        .replace("'", "")
                        .replace("-", "_");

                    CreatureReference {
                        id: creature.id,
                        name: creature.name.clone(),
                        filepath: format!("assets/creatures/{}.ron", filename),
                    }
                })
                .collect();

            // Step 2: Save registry file (creatures.ron)
            let creatures_path = dir.join(&self.campaign.creatures_file);
            if let Some(parent) = creatures_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    CampaignIoError::CreateDirectoryFailed(format!("creatures: {}", e))
                })?;
            }

            let registry_ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(true)
                .separate_tuple_members(true)
                .depth_limit(2);

            let registry_contents = ron::ser::to_string_pretty(&references, registry_ron_config)
                .map_err(|e| {
                    CampaignIoError::SerializationFailed(format!("creatures registry: {}", e))
                })?;

            fs::write(&creatures_path, registry_contents).map_err(|e| {
                CampaignIoError::WriteFileFailed(format!("creatures registry: {}", e))
            })?;

            // Step 3: Save individual creature files (assets/creatures/*.ron)
            let creatures_dir = dir.join("assets/creatures");
            fs::create_dir_all(&creatures_dir).map_err(|e| {
                CampaignIoError::CreateDirectoryFailed(format!("creatures assets: {}", e))
            })?;

            let creature_ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(false)
                .depth_limit(3);

            for (reference, creature) in references.iter().zip(self.campaign_data.creatures.iter())
            {
                let creature_path = dir.join(&reference.filepath);

                let creature_contents =
                    ron::ser::to_string_pretty(creature, creature_ron_config.clone()).map_err(
                        |e| {
                            CampaignIoError::SerializationFailed(format!(
                                "creature {}: {}",
                                reference.name, e
                            ))
                        },
                    )?;

                fs::write(&creature_path, creature_contents).map_err(|e| {
                    CampaignIoError::WriteFileFailed(format!(
                        "creature file {}: {}",
                        reference.filepath, e
                    ))
                })?;
            }

            self.ui_state.status_message = format!(
                "Saved {} creatures (registry + {} individual files)",
                self.campaign_data.creatures.len(),
                self.campaign_data.creatures.len()
            );
            self.unsaved_changes = true;
            Ok(())
        } else {
            Err(CampaignIoError::NoCampaignDir)
        }
    }

    /// Load maps from the maps directory
    pub(crate) fn load_maps(&mut self) {
        self.campaign_data.maps.clear();

        if let Some(ref dir) = self.campaign_dir {
            let maps_dir = dir.join(&self.campaign.maps_dir);

            if maps_dir.exists() && maps_dir.is_dir() {
                match fs::read_dir(&maps_dir) {
                    Ok(entries) => {
                        let mut loaded_count = 0;
                        for entry in entries.filter_map(|e| e.ok()) {
                            let path = entry.path();
                            if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                                match fs::read_to_string(&path) {
                                    Ok(contents) => match ron::from_str::<Map>(&contents) {
                                        Ok(map) => {
                                            self.campaign_data.maps.push(map);
                                            loaded_count += 1;

                                            // Mark individual map file as loaded in asset manager
                                            if let Some(ref mut manager) = self.asset_manager {
                                                if let Ok(relative_path) = path.strip_prefix(dir) {
                                                    if let Some(path_str) = relative_path.to_str() {
                                                        manager.mark_data_file_loaded(path_str, 1);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            self.ui_state.status_message = format!(
                                                "Failed to parse map {:?}: {}",
                                                path.file_name().unwrap_or_default(),
                                                e
                                            );

                                            // Mark individual map file as error in asset manager
                                            if let Some(ref mut manager) = self.asset_manager {
                                                if let Ok(relative_path) = path.strip_prefix(dir) {
                                                    if let Some(path_str) = relative_path.to_str() {
                                                        manager.mark_data_file_error(
                                                            path_str,
                                                            &e.to_string(),
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        self.ui_state.status_message = format!(
                                            "Failed to read map {:?}: {}",
                                            path.file_name().unwrap_or_default(),
                                            e
                                        );

                                        // Mark individual map file as error in asset manager
                                        if let Some(ref mut manager) = self.asset_manager {
                                            if let Ok(relative_path) = path.strip_prefix(dir) {
                                                if let Some(path_str) = relative_path.to_str() {
                                                    manager.mark_data_file_error(
                                                        path_str,
                                                        &e.to_string(),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if loaded_count > 0 {
                            self.ui_state.status_message = format!("Loaded {} maps", loaded_count);
                        }
                    }
                    Err(e) => {
                        self.ui_state.status_message =
                            format!("Failed to read maps directory: {}", e);
                    }
                }
            }
        }
    }

    /// Save a map to RON file
    pub(crate) fn save_map(&mut self, map: &Map) -> Result<(), CampaignIoError> {
        if let Some(ref dir) = self.campaign_dir {
            let maps_dir = dir.join(&self.campaign.maps_dir);

            // Create maps directory if it doesn't exist
            fs::create_dir_all(&maps_dir)
                .map_err(|e| CampaignIoError::CreateDirectoryFailed(format!("maps: {}", e)))?;

            let map_filename = format!("map_{}.ron", map.id);
            let map_path = maps_dir.join(map_filename);

            let ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(false);

            let contents = ron::ser::to_string_pretty(map, ron_config)
                .map_err(|e| CampaignIoError::SerializationFailed(format!("map: {}", e)))?;

            fs::write(&map_path, contents)
                .map_err(|e| CampaignIoError::WriteFileFailed(format!("map: {}", e)))?;

            self.unsaved_changes = true;
            Ok(())
        } else {
            Err(CampaignIoError::NoCampaignDir)
        }
    }

    /// Validate the campaign metadata
    pub(crate) fn validate_campaign(&mut self) {
        self.logger
            .debug(category::VALIDATION, "validate_campaign() called");

        // Always sync the stock_templates mirror from the editor state before
        // validating.  validate_npc_ids() checks self.campaign_data.stock_templates, but that
        // mirror is only refreshed during tab renders.  When the user clicks
        // "Validate Campaign" directly (toolbar, Re-validate button, metadata
        // editor) neither tab render runs first, so the mirror can be stale and
        // cause false "unknown stock template" errors for templates that are
        // perfectly loaded in the editor state.
        self.campaign_data.stock_templates = self
            .editor_registry
            .stock_templates_editor_state
            .templates
            .clone();

        self.validation_state.validation_errors.clear();

        // Validate data IDs for uniqueness (in EditorTab order)
        self.validation_state
            .validation_errors
            .extend(self.validate_item_ids());
        self.validation_state
            .validation_errors
            .extend(self.validate_spell_ids());
        self.validation_state
            .validation_errors
            .extend(self.validate_condition_ids());
        self.validation_state
            .validation_errors
            .extend(self.validate_monster_ids());
        self.validation_state
            .validation_errors
            .extend(self.validate_map_ids());
        // Quests validated elsewhere
        // Classes validated elsewhere
        // Races validated elsewhere
        self.validation_state
            .validation_errors
            .extend(self.validate_character_ids());
        // Dialogues validated elsewhere
        self.validation_state
            .validation_errors
            .extend(self.validate_npc_ids());
        self.validation_state
            .validation_errors
            .extend(self.validate_stock_template_refs());
        self.validation_state
            .validation_errors
            .extend(self.validate_proficiency_ids());

        // Add category status checks (passed or no data info)
        self.validation_state
            .validation_errors
            .extend(self.generate_category_status_checks());

        // Validate required runtime terrain tree and grass textures through existing SDK surfaces.
        if let Some(asset_manager) = self.asset_manager.as_ref() {
            self.validation_state
                .validation_errors
                .extend(self.validate_tree_texture_assets(asset_manager));
            self.validation_state
                .validation_errors
                .extend(self.validate_grass_texture_assets(asset_manager));
        } else {
            self.validation_state.validation_errors
                .push(validation::ValidationResult::warning(
                validation::ValidationCategory::Assets,
                "Terrain texture validation skipped because asset scanning has not been initialized",
            ));
        }

        // Required fields - Metadata category
        if self.campaign.id.is_empty() {
            self.validation_state
                .validation_errors
                .push(validation::ValidationResult::error(
                    validation::ValidationCategory::Metadata,
                    "Campaign ID is required",
                ));
        } else if !self
            .campaign
            .id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            self.validation_state
                .validation_errors
                .push(validation::ValidationResult::error(
                    validation::ValidationCategory::Metadata,
                    "Campaign ID must contain only alphanumeric characters and underscores",
                ));
        }

        if self.campaign.name.is_empty() {
            self.validation_state
                .validation_errors
                .push(validation::ValidationResult::error(
                    validation::ValidationCategory::Metadata,
                    "Campaign name is required",
                ));
        }

        if self.campaign.author.is_empty() {
            self.validation_state
                .validation_errors
                .push(validation::ValidationResult::warning(
                    validation::ValidationCategory::Metadata,
                    "Author name is recommended",
                ));
        }

        // Version validation - Metadata category
        if !self.campaign.version.contains('.') {
            self.validation_state
                .validation_errors
                .push(validation::ValidationResult::error(
                    validation::ValidationCategory::Metadata,
                    "Version should follow semantic versioning (e.g., 1.0.0)",
                ));
        }

        // Engine version validation - Metadata category
        if !self.campaign.engine_version.contains('.') {
            self.validation_state
                .validation_errors
                .push(validation::ValidationResult::warning(
                    validation::ValidationCategory::Metadata,
                    "Engine version should follow semantic versioning",
                ));
        }

        // Configuration validation
        if self.campaign.starting_map.is_empty() {
            self.validation_state
                .validation_errors
                .push(validation::ValidationResult::error(
                    validation::ValidationCategory::Configuration,
                    "Starting map is required",
                ));
        } else if !self.campaign_data.maps.is_empty() {
            // Only validate existence if the maps are loaded - avoids false positives during
            // tests or when the campaign hasn't loaded assets yet.
            let start_map_key = self.campaign.starting_map.trim();
            let mut found = false;

            // 1) Numeric map ID (e.g., "1")
            if let Ok(parsed_id) = start_map_key.parse::<u16>() {
                found = self.campaign_data.maps.iter().any(|m| m.id == parsed_id);
            }

            // 2) map_N pattern (e.g., "map_1")
            if !found {
                if let Some(stripped) = start_map_key.strip_prefix("map_") {
                    if let Ok(num) = stripped.parse::<u16>() {
                        found = self.campaign_data.maps.iter().any(|m| m.id == num);
                    }
                }
            }

            // 3) filename pattern with .ron (e.g., "map_1.ron" or "starter_town.ron")
            if !found && start_map_key.ends_with(".ron") {
                let base = start_map_key.trim_end_matches(".ron");
                if let Some(stripped) = base.strip_prefix("map_") {
                    if let Ok(num) = stripped.parse::<u16>() {
                        found = self.campaign_data.maps.iter().any(|m| m.id == num);
                    }
                }
                if !found {
                    if let Ok(num) = base.parse::<u16>() {
                        found = self.campaign_data.maps.iter().any(|m| m.id == num);
                    }
                }
                if !found {
                    let normalized = base.replace('_', " ").to_lowercase();
                    found = self
                        .campaign_data
                        .maps
                        .iter()
                        .any(|m| m.name.to_lowercase() == normalized);
                }
            }

            // 4) Normalized name match: starter_town -> "starter town" matches "Starter Town"
            if !found {
                let normalized = start_map_key.replace('_', " ").to_lowercase();
                found = self
                    .campaign_data
                    .maps
                    .iter()
                    .any(|m| m.name.to_lowercase() == normalized);
            }

            // If no match, this is a configuration error for the campaign
            if !found {
                self.validation_state
                    .validation_errors
                    .push(validation::ValidationResult::error(
                        validation::ValidationCategory::Configuration,
                        format!(
                            "Starting map '{}' does not match any loaded map",
                            start_map_key
                        ),
                    ));
            }
        }

        if self.campaign.max_party_size == 0 || self.campaign.max_party_size > PARTY_MAX_SIZE {
            self.validation_state
                .validation_errors
                .push(validation::ValidationResult::warning(
                    validation::ValidationCategory::Configuration,
                    format!("Max party size should be between 1 and {}", PARTY_MAX_SIZE),
                ));
        }

        if self.campaign.max_roster_size < self.campaign.max_party_size {
            self.validation_state
                .validation_errors
                .push(validation::ValidationResult::error(
                    validation::ValidationCategory::Configuration,
                    "Max roster size must be >= max party size",
                ));
        }

        if self.campaign.starting_level == 0
            || self.campaign.starting_level > self.campaign.max_level
        {
            self.validation_state
                .validation_errors
                .push(validation::ValidationResult::error(
                    validation::ValidationCategory::Configuration,
                    "Starting level must be between 1 and max level",
                ));
        }

        // Starting resource validity checks
        if self.campaign.starting_food < FOOD_MIN as u32
            || self.campaign.starting_food > FOOD_MAX as u32
        {
            self.validation_state
                .validation_errors
                .push(validation::ValidationResult::warning(
                    validation::ValidationCategory::Configuration,
                    format!(
                        "Starting food must be between {} and {}",
                        FOOD_MIN, FOOD_MAX
                    ),
                ));
        }

        if self.campaign.starting_gold > STARTING_GOLD_MAX {
            self.validation_state
                .validation_errors
                .push(validation::ValidationResult::warning(
                    validation::ValidationCategory::Configuration,
                    format!(
                        "Starting gold ({}) exceeds recommended maximum {}",
                        self.campaign.starting_gold, STARTING_GOLD_MAX
                    ),
                ));
        }

        // File path validation
        for (field, path) in [
            ("Items file", &self.campaign.items_file),
            ("Spells file", &self.campaign.spells_file),
            ("Monsters file", &self.campaign.monsters_file),
            ("Classes file", &self.campaign.classes_file),
            ("Races file", &self.campaign.races_file),
            ("Quests file", &self.campaign.quests_file),
            ("Dialogue file", &self.campaign.dialogue_file),
            ("NPCs file", &self.campaign.npcs_file),
        ] {
            if path.is_empty() {
                self.validation_state
                    .validation_errors
                    .push(validation::ValidationResult::error(
                        validation::ValidationCategory::FilePaths,
                        format!("{} path is required", field),
                    ));
            } else if !path.ends_with(".ron") {
                self.validation_state.validation_errors.push(
                    validation::ValidationResult::warning(
                        validation::ValidationCategory::FilePaths,
                        format!("{} should use .ron extension", field),
                    ),
                );
            }
        }

        // Run SDK validator for deeper content checks (e.g., starting innkeeper)
        // Only run if NPCs have been loaded into the editor OR the configured starting
        // innkeeper differs from the default tutorial innkeeper (i.e., user-specified).
        if !self.editor_registry.npc_editor_state.npcs.is_empty()
            || self.campaign.starting_innkeeper != default_starting_innkeeper()
        {
            // Build a lightweight ContentDatabase containing relevant content so the SDK Validator can
            // validate content-dependent configuration such as `starting_innkeeper`.
            let mut db = antares::sdk::database::ContentDatabase::new();

            // Populate NPC database from the editor state
            for npc in &self.editor_registry.npc_editor_state.npcs {
                if let Err(e) = db.npcs.add_npc(npc.clone()) {
                    self.logger.warn(
                        category::VALIDATION,
                        &format!(
                            "NPC '{}' could not be added to validation DB: {}",
                            npc.id, e
                        ),
                    );
                }
            }

            // Invoke SDK validator for campaign config checks
            let validator = antares::sdk::validation::Validator::new(&db);

            // Build a minimal CampaignConfig for validation - other fields are defaulted to reasonable values
            let config = antares::sdk::campaign_loader::CampaignConfig {
                starting_map: 0,
                starting_position: antares::domain::types::Position { x: 0, y: 0 },
                starting_direction: antares::domain::types::Direction::North,
                starting_gold: self.campaign.starting_gold,
                starting_food: self.campaign.starting_food,
                starting_innkeeper: self.campaign.starting_innkeeper.clone(),
                max_party_size: self.campaign.max_party_size,
                max_roster_size: self.campaign.max_roster_size,
                difficulty: antares::sdk::campaign_loader::Difficulty::Normal,
                permadeath: self.campaign.permadeath,
                allow_multiclassing: self.campaign.allow_multiclassing,
                starting_level: self.campaign.starting_level,
                max_level: self.campaign.max_level,
                starting_time: self.campaign.starting_time,
            };

            let config_errors = validator.validate_campaign_config(&config);
            for ve in config_errors {
                match ve {
                    antares::sdk::validation::ValidationError::InvalidStartingInnkeeper {
                        innkeeper_id,
                        reason,
                    } => {
                        // Use a message format that matches existing tests' expectations
                        let msg =
                            format!("Starting innkeeper '{}' invalid: {}", innkeeper_id, reason);
                        self.validation_state.validation_errors.push(
                            validation::ValidationResult::error(
                                validation::ValidationCategory::Configuration,
                                msg,
                            ),
                        );
                    }
                    other => match other.severity() {
                        antares::sdk::validation::Severity::Error => {
                            self.validation_state.validation_errors.push(
                                validation::ValidationResult::error(
                                    validation::ValidationCategory::Configuration,
                                    other.to_string(),
                                ),
                            );
                        }
                        antares::sdk::validation::Severity::Warning => {
                            self.validation_state.validation_errors.push(
                                validation::ValidationResult::warning(
                                    validation::ValidationCategory::Configuration,
                                    other.to_string(),
                                ),
                            );
                        }
                        antares::sdk::validation::Severity::Info => {
                            self.validation_state.validation_errors.push(
                                validation::ValidationResult::info(
                                    validation::ValidationCategory::Configuration,
                                    other.to_string(),
                                ),
                            );
                        }
                    },
                }
            }
        }

        // Update status using ValidationSummary
        let summary =
            validation::ValidationSummary::from_results(&self.validation_state.validation_errors);

        if self.validation_state.validation_errors.is_empty() {
            self.logger
                .info(category::VALIDATION, "Validation passed with no issues");
            self.ui_state.status_message = "✅ Validation passed!".to_string();
        } else {
            self.logger.info(
                category::VALIDATION,
                &format!(
                    "Validation complete: {} error(s), {} warning(s)",
                    summary.error_count, summary.warning_count
                ),
            );
            // Log individual errors at debug level
            for result in &self.validation_state.validation_errors {
                let level_str = match result.severity {
                    validation::ValidationSeverity::Critical => "CRITICAL",
                    validation::ValidationSeverity::Error => "ERROR",
                    validation::ValidationSeverity::Warning => "WARN",
                    validation::ValidationSeverity::Info => "INFO",
                    validation::ValidationSeverity::Passed => "PASS",
                };
                self.logger.debug(
                    category::VALIDATION,
                    &format!("[{}] {}: {}", level_str, result.category, result.message),
                );
            }
            self.ui_state.status_message = format!(
                "Validation: {} error(s), {} warning(s)",
                summary.error_count, summary.warning_count
            );
        }
    }

    /// Create a new campaign
    pub(crate) fn new_campaign(&mut self) {
        if self.unsaved_changes {
            self.ui_state.show_unsaved_warning = true;
            self.pending_action = Some(PendingAction::New);
        } else {
            self.do_new_campaign();
        }
    }

    pub(crate) fn do_new_campaign(&mut self) {
        // Reset stock templates editor so it does not retain data from a
        // previously opened campaign.  The flag inside reset_for_new_campaign
        // tells show() to auto-load if the user adds a templates file later.
        self.editor_registry
            .stock_templates_editor_state
            .reset_for_new_campaign();
        self.campaign_data.stock_templates.clear();
        self.campaign = CampaignMetadata::default();

        // Sync the campaign editor's authoritative metadata and edit buffer with
        // the newly created campaign. This ensures the editor shows the fresh
        // campaign values and a fresh buffer, preventing stale UI states.
        self.editor_registry.campaign_editor_state.metadata = self.campaign.clone();
        self.editor_registry.campaign_editor_state.buffer =
            campaign_editor::CampaignMetadataEditBuffer::from_metadata(
                &self.editor_registry.campaign_editor_state.metadata,
            );
        self.editor_registry
            .campaign_editor_state
            .has_unsaved_changes = false;
        self.editor_registry.campaign_editor_state.mode = campaign_editor::CampaignEditorMode::List;

        // Clear all loaded campaign content and reset editor states so the new
        // campaign starts with an empty workspace rather than retaining the
        // previously opened campaign's data.
        self.campaign_data.items.clear();
        self.editor_registry.items_editor_state = ItemsEditorState::new();

        self.campaign_data.spells.clear();
        self.editor_registry.spells_editor_state = SpellsEditorState::new();

        self.campaign_data.proficiencies.clear();
        self.editor_registry.proficiencies_editor_state =
            proficiencies_editor::ProficienciesEditorState::new();

        self.campaign_data.monsters.clear();
        self.editor_registry.monsters_editor_state = MonstersEditorState::new();

        self.campaign_data.creatures.clear();
        self.editor_registry.creatures_editor_state = creatures_editor::CreaturesEditorState::new();

        self.campaign_data.conditions.clear();
        self.editor_registry.conditions_editor_state = ConditionsEditorState::new();

        self.campaign_data.furniture_definitions.clear();
        self.editor_registry.furniture_editor_state = furniture_editor::FurnitureEditorState::new();

        self.campaign_data.maps.clear();
        self.editor_registry.maps_editor_state = MapsEditorState::new();

        self.campaign_data.quests.clear();
        self.editor_registry.quest_editor_state = QuestEditorState::default();

        self.campaign_data.dialogues.clear();
        self.editor_registry.dialogue_editor_state = DialogueEditorState::default();

        // Reset NPCs, characters, classes and races editors
        self.editor_registry.npc_editor_state = npc_editor::NpcEditorState::default();
        self.editor_registry.characters_editor_state =
            characters_editor::CharactersEditorState::new();
        self.editor_registry.classes_editor_state = classes_editor::ClassesEditorState::default();
        self.editor_registry.races_editor_state = races_editor::RacesEditorState::default();

        // Reset campaign file/path-related state
        self.campaign_path = None;
        self.campaign_dir = None;
        self.unsaved_changes = false;
        self.validation_state.validation_errors.clear();
        self.ui_state.file_tree.clear();

        // Clear undo/redo history and any remembered content in the manager's state
        self.undo_redo_manager.clear();
        {
            // Clear the command history when creating a new campaign.
            // The campaign data itself will be reset by the field assignments below.
            self.undo_redo_manager.clear();
        }

        // Drop asset manager to avoid referencing previous campaign assets
        self.asset_manager = None;

        self.ui_state.status_message = "New campaign created.".to_string();
    }

    /// Save campaign to file
    pub(crate) fn save_campaign(&mut self) -> Result<(), CampaignError> {
        if self.campaign_path.is_none() {
            return Err(CampaignError::NoPath);
        }

        self.do_save_campaign()
    }

    pub(crate) fn do_save_campaign(&mut self) -> Result<(), CampaignError> {
        // Clone path early to avoid borrow checker issues with mutable save methods
        let path = self.campaign_path.clone().ok_or(CampaignError::NoPath)?;

        // CRITICAL FIX: Save all data files BEFORE saving campaign metadata
        // This ensures all content is persisted when user clicks "Save Campaign"

        // Track any save failures but continue (partial save is better than no save)
        let mut save_warnings = Vec::new();

        if let Err(e) = self.save_items() {
            save_warnings.push(format!("Items: {}", e));
        }

        if let Err(e) = self.save_spells() {
            save_warnings.push(format!("Spells: {}", e));
        }

        if let Err(e) = self.save_monsters() {
            save_warnings.push(format!("Monsters: {}", e));
        }

        if let Err(e) = self.save_creatures() {
            save_warnings.push(format!("Creatures: {}", e));
        }

        if let Err(e) = self.save_conditions() {
            save_warnings.push(format!("Conditions: {}", e));
        }

        if let Err(e) = self.save_proficiencies() {
            save_warnings.push(format!("Proficiencies: {}", e));
        }

        // Save maps individually (they're saved per-map, not as a collection)
        // Clone maps to avoid borrow checker issues
        let maps_to_save = self.campaign_data.maps.clone();
        for (idx, map) in maps_to_save.iter().enumerate() {
            if let Err(e) = self.save_map(map) {
                save_warnings.push(format!("Map {}: {}", idx, e));
            }
        }

        if let Err(e) = self.save_furniture() {
            save_warnings.push(format!("Furniture: {}", e));
        }

        if let Err(e) = self.save_quests() {
            save_warnings.push(format!("Quests: {}", e));
        }

        if let Some(dir) = &self.campaign_dir {
            let dialogues_path = dir.join(&self.campaign.dialogue_file);
            if let Err(e) = self.save_dialogues_to_file(&dialogues_path) {
                save_warnings.push(format!("Dialogues: {}", e));
            }

            let npcs_path = dir.join(&self.campaign.npcs_file);
            if let Err(e) = self.save_npcs_to_file(&npcs_path) {
                save_warnings.push(format!("NPCs: {}", e));
            }

            let stock_templates_path = dir.join(&self.campaign.stock_templates_file);
            // Guard: only write stock templates if they were successfully loaded
            // from disk during this session OR the user made explicit in-editor
            // changes.  An empty Vec that was never backed by a real file must
            // NOT overwrite an existing file with `[]` — that is the root cause
            // of the repeated stock-template wipe bug.
            let should_save_templates = self
                .editor_registry
                .stock_templates_editor_state
                .loaded_from_file
                || self
                    .editor_registry
                    .stock_templates_editor_state
                    .has_unsaved_changes;
            if should_save_templates {
                if let Err(e) = self
                    .editor_registry
                    .stock_templates_editor_state
                    .save_to_file(&stock_templates_path)
                {
                    save_warnings.push(format!("StockTemplates: {}", e));
                }
            }
        }

        // Now save campaign metadata to RON format
        let ron_config = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .enumerate_arrays(false)
            .depth_limit(4);

        let ron_string = ron::ser::to_string_pretty(&self.campaign, ron_config)?;

        // Write campaign metadata file
        fs::write(&path, ron_string)?;

        self.unsaved_changes = false;

        // Keep the campaign metadata editor in sync with the saved campaign file.
        // This ensures that the UI and editor buffer reflect the authoritative
        // campaign data after a successful save.
        self.editor_registry.campaign_editor_state.metadata = self.campaign.clone();
        self.editor_registry.campaign_editor_state.buffer =
            campaign_editor::CampaignMetadataEditBuffer::from_metadata(
                &self.editor_registry.campaign_editor_state.metadata,
            );
        self.editor_registry
            .campaign_editor_state
            .has_unsaved_changes = false;
        self.editor_registry.campaign_editor_state.mode = campaign_editor::CampaignEditorMode::List;

        // Update status message based on results
        if save_warnings.is_empty() {
            self.ui_state.status_message =
                format!("✅ Campaign and all data saved to: {}", path.display());
        } else {
            self.ui_state.status_message = format!(
                "⚠️ Campaign saved with warnings:\n{}",
                save_warnings.join("\n")
            );
        }

        // Update file tree if we have a campaign directory
        if let Some(dir) = self.campaign_dir.clone() {
            self.update_file_tree(&dir);
        }

        Ok(())
    }

    /// Save campaign as (with file dialog)
    pub(crate) fn save_campaign_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("campaign.ron")
            .add_filter("RON Files", &["ron"])
            .save_file()
        {
            self.campaign_path = Some(path.clone());

            // Set campaign directory (parent of campaign.ron)
            if let Some(parent) = path.parent() {
                self.campaign_dir = Some(parent.to_path_buf());
            }

            match self.do_save_campaign() {
                Ok(()) => {}
                Err(e) => {
                    self.ui_state.status_message = format!("Failed to save: {}", e);
                }
            }
        }
    }

    /// Open campaign from file
    pub(crate) fn open_campaign(&mut self) {
        if self.unsaved_changes {
            self.ui_state.show_unsaved_warning = true;
            self.pending_action = Some(PendingAction::Open);
        } else {
            self.do_open_campaign();
        }
    }

    pub(crate) fn do_open_campaign(&mut self) {
        self.logger
            .debug(category::FILE_IO, "do_open_campaign() called");
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("RON Files", &["ron"])
            .pick_file()
        {
            self.logger.info(
                category::FILE_IO,
                &format!("Opening campaign: {}", path.display()),
            );
            match self.load_campaign_file(&path) {
                Ok(()) => {
                    self.campaign_path = Some(path.clone());

                    // Set campaign directory
                    if let Some(parent) = path.parent() {
                        let parent_buf = parent.to_path_buf();
                        self.campaign_dir = Some(parent_buf.clone());
                        self.logger.verbose(
                            category::FILE_IO,
                            &format!("Campaign directory: {}", parent_buf.display()),
                        );
                        self.update_file_tree(&parent_buf);
                    }

                    // Load data files
                    self.logger
                        .debug(category::FILE_IO, "Loading data files...");
                    self.load_items();
                    self.load_spells();
                    self.load_proficiencies();
                    self.load_monsters();
                    self.load_creatures();
                    self.load_classes_from_campaign();
                    self.load_races_from_campaign();
                    self.load_characters_from_campaign();
                    self.load_maps();
                    self.load_conditions();
                    self.load_furniture();

                    // Load quests and dialogues
                    if let Err(e) = self.load_quests() {
                        self.logger
                            .warn(category::FILE_IO, &format!("Failed to load quests: {}", e));
                    }

                    if let Err(e) = self.load_dialogues() {
                        self.logger.warn(
                            category::FILE_IO,
                            &format!("Failed to load dialogues: {}", e),
                        );
                    }

                    if let Err(e) = self.load_npcs() {
                        self.logger
                            .warn(category::FILE_IO, &format!("Failed to load NPCs: {}", e));
                    }

                    self.sync_obj_importer_campaign_state();

                    // Load item mesh assets into the Item Mesh Editor registry.
                    // Must happen after campaign_dir is set (above) and after
                    // load_items() so mesh IDs on items are already known.
                    if let Some(ref dir) = self.campaign_dir.clone() {
                        self.logger
                            .debug(category::FILE_IO, "Loading item mesh assets...");
                        self.item_mesh_editor_state.load_from_campaign(dir);
                        self.logger.info(
                            category::FILE_IO,
                            &format!(
                                "Loaded {} item mesh entries",
                                self.item_mesh_editor_state.registry.len()
                            ),
                        );
                    }

                    // Reset editor state before loading so stale data from any
                    // previously opened campaign is cleared first.  The explicit
                    // load below will clear needs_initial_load on success; on
                    // failure the auto-load in show() acts as a reliable fallback.
                    self.editor_registry
                        .stock_templates_editor_state
                        .reset_for_new_campaign();
                    self.campaign_data.stock_templates.clear();
                    self.load_stock_templates();

                    // Scan asset references and mark loaded data files
                    if let Some(ref mut manager) = self.asset_manager {
                        let campaign_refs = asset_manager::CampaignRefs {
                            items: &self.campaign_data.items,
                            quests: &self.campaign_data.quests,
                            dialogues: &self.campaign_data.dialogues,
                            maps: &self.campaign_data.maps,
                            classes: &self.editor_registry.classes_editor_state.classes,
                            characters: &self.editor_registry.characters_editor_state.characters,
                            npcs: &self.editor_registry.npc_editor_state.npcs,
                        };
                        manager.scan_references(&campaign_refs);
                        manager.mark_data_files_as_referenced();
                    }

                    self.unsaved_changes = false;
                    self.logger.info(
                        category::FILE_IO,
                        &format!("Campaign opened successfully: {}", self.campaign.name),
                    );
                    self.ui_state.status_message =
                        format!("Opened campaign from: {}", path.display());

                    // Synchronize campaign editor state with the newly opened campaign.
                    // This ensures the metadata editor shows the loaded campaign and its
                    // edit buffer reflects the current authoritative values.
                    self.editor_registry.campaign_editor_state.metadata = self.campaign.clone();
                    self.editor_registry.campaign_editor_state.buffer =
                        campaign_editor::CampaignMetadataEditBuffer::from_metadata(
                            &self.editor_registry.campaign_editor_state.metadata,
                        );
                    self.editor_registry
                        .campaign_editor_state
                        .has_unsaved_changes = false;
                    self.editor_registry.campaign_editor_state.mode =
                        campaign_editor::CampaignEditorMode::List;
                }
                Err(e) => {
                    self.logger.error(
                        category::FILE_IO,
                        &format!("Failed to load campaign: {}", e),
                    );
                    self.ui_state.status_message = format!("Failed to load campaign: {}", e);
                }
            }
        }
    }

    pub(crate) fn load_campaign_file(&mut self, path: &PathBuf) -> Result<(), CampaignError> {
        let contents = fs::read_to_string(path)?;
        self.campaign = ron::from_str(&contents)?;
        Ok(())
    }

    /// Update the file tree view
    pub(crate) fn update_file_tree(&mut self, dir: &PathBuf) {
        self.ui_state.file_tree.clear();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    let node = FileNode {
                        name,
                        is_directory: metadata.is_dir(),
                        _children: if metadata.is_dir() {
                            self.read_directory(&path)
                        } else {
                            Vec::new()
                        },
                    };

                    self.ui_state.file_tree.push(node);
                }
            }
        }

        // Sort: directories first, then alphabetically
        self.ui_state
            .file_tree
            .sort_by(|a, b| match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            });
    }

    pub(crate) fn read_directory(&self, dir: &PathBuf) -> Vec<FileNode> {
        let mut children = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let name = entry.file_name().to_string_lossy().to_string();

                    children.push(FileNode {
                        name,
                        is_directory: metadata.is_dir(),
                        _children: Vec::new(), // Don't recurse deeper for now
                    });
                }
            }
        }

        children.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        children
    }

    /// Check for unsaved changes before action
    pub(crate) fn check_unsaved_and_exit(&mut self) {
        if self.unsaved_changes {
            self.ui_state.show_unsaved_warning = true;
            self.pending_action = Some(PendingAction::Exit);
        } else {
            std::process::exit(0);
        }
    }

    /// Converts asset-manager tree texture diagnostics into validation-panel results.
    pub(crate) fn validate_tree_texture_assets(
        &self,
        asset_manager: &asset_manager::AssetManager,
    ) -> Vec<validation::ValidationResult> {
        asset_manager
            .validate_tree_texture_assets()
            .into_iter()
            .map(|issue| {
                let mut result = validation::ValidationResult::error(
                    validation::ValidationCategory::Assets,
                    issue.message,
                )
                .with_file_path(issue.expected_path.clone());

                if let Some(actual_path) = issue.actual_path {
                    result = result.with_file_path(actual_path);
                }

                result
            })
            .collect()
    }

    /// Converts asset-manager grass texture diagnostics into validation-panel results.
    pub(crate) fn validate_grass_texture_assets(
        &self,
        asset_manager: &asset_manager::AssetManager,
    ) -> Vec<validation::ValidationResult> {
        asset_manager
            .validate_grass_texture_assets()
            .into_iter()
            .map(|issue| {
                let mut result = validation::ValidationResult::error(
                    validation::ValidationCategory::Assets,
                    issue.message,
                )
                .with_file_path(issue.expected_path.clone());

                if let Some(actual_path) = issue.actual_path {
                    result = result.with_file_path(actual_path);
                }

                result
            })
            .collect()
    }

    /// Run advanced validation and generate report
    pub(crate) fn run_advanced_validation(&mut self) {
        let validator = advanced_validation::AdvancedValidator::new(
            self.campaign_data.items.clone(),
            self.campaign_data.monsters.clone(),
            self.campaign_data.quests.clone(),
            self.campaign_data.maps.clone(),
        );
        self.validation_state.validation_report = validator.generate_report();
        self.validation_state.advanced_validator = Some(validator);
    }
}

impl CampaignBuilderApp {
    /// Load quests from file
    pub(crate) fn load_quests(&mut self) -> Result<(), CampaignError> {
        if let Some(dir) = &self.campaign_dir {
            let quests_path = dir.join(&self.campaign.quests_file);
            if quests_path.exists() {
                match fs::read_to_string(&quests_path) {
                    Ok(contents) => {
                        match ron::from_str::<Vec<antares::domain::quest::Quest>>(&contents) {
                            Ok(quests) => {
                                self.campaign_data.quests = quests;
                                self.ui_state.status_message =
                                    format!("Loaded {} quests", self.campaign_data.quests.len());
                            }
                            Err(e) => {
                                self.logger.error(
                                    category::FILE_IO,
                                    &format!(
                                        "Failed to parse quests from {:?}: {}",
                                        quests_path, e
                                    ),
                                );
                                return Err(CampaignError::Deserialization(e));
                            }
                        }
                    }
                    Err(e) => {
                        self.logger.error(
                            category::FILE_IO,
                            &format!("Failed to read quests file {:?}: {}", quests_path, e),
                        );
                        return Err(CampaignError::Io(e));
                    }
                }
            } else {
                self.logger.debug(
                    category::FILE_IO,
                    &format!("Quests file does not exist: {:?}", quests_path),
                );
            }
        } else {
            self.logger.warn(
                category::FILE_IO,
                "No campaign directory set when trying to load quests",
            );
        }
        Ok(())
    }

    /// Save quests to file
    pub(crate) fn save_quests(&self) -> Result<(), CampaignError> {
        if let Some(dir) = &self.campaign_dir {
            let quests_path = dir.join(&self.campaign.quests_file);
            // Create quests directory if it doesn't exist
            if let Some(parent) = quests_path.parent() {
                fs::create_dir_all(parent).map_err(CampaignError::Io)?;
            }

            // Sort by ID before serializing for stable file order.
            let mut sorted_quests = self.campaign_data.quests.clone();
            sorted_quests.sort_by_key(|q| q.id);

            let contents = ron::ser::to_string_pretty(&sorted_quests, Default::default())?;
            fs::write(&quests_path, contents)?;
        }
        Ok(())
    }

    /// Load classes from campaign directory
    pub(crate) fn load_classes_from_campaign(&mut self) {
        if let Some(dir) = &self.campaign_dir {
            let path = dir.join(&self.campaign.classes_file);
            if path.exists() {
                match self
                    .editor_registry
                    .classes_editor_state
                    .load_from_file(&path)
                {
                    Ok(_) => {
                        self.ui_state.status_message = format!(
                            "Loaded {} classes",
                            self.editor_registry.classes_editor_state.classes.len()
                        );
                    }
                    Err(e) => {
                        self.ui_state.status_message = format!("Failed to load classes: {}", e);
                        self.logger.error(
                            category::FILE_IO,
                            &format!("Failed to load classes from {:?}: {}", path, e),
                        );
                    }
                }
            } else {
                self.logger.debug(
                    category::FILE_IO,
                    &format!("Classes file does not exist: {:?}", path),
                );
            }
        } else {
            self.logger.warn(
                category::FILE_IO,
                "No campaign directory set when trying to load classes",
            );
        }
    }

    /// Load characters from campaign directory
    pub(crate) fn load_characters_from_campaign(&mut self) {
        if let Some(dir) = &self.campaign_dir {
            let path = dir.join(&self.campaign.characters_file);
            if path.exists() {
                match self
                    .editor_registry
                    .characters_editor_state
                    .load_from_file(&path)
                {
                    Ok(_) => {
                        let count = self
                            .editor_registry
                            .characters_editor_state
                            .characters
                            .len();
                        self.ui_state.status_message = format!("Loaded {} characters", count);
                        // Mark data file as loaded in asset manager
                        if let Some(ref mut manager) = self.asset_manager {
                            manager.mark_data_file_loaded(&self.campaign.characters_file, count);
                        }
                    }
                    Err(e) => {
                        self.ui_state.status_message = format!("Failed to load characters: {}", e);
                        self.logger.error(
                            category::FILE_IO,
                            &format!("Failed to load characters from {:?}: {}", path, e),
                        );
                        // Mark data file as error in asset manager
                        if let Some(ref mut manager) = self.asset_manager {
                            manager.mark_data_file_error(
                                &self.campaign.characters_file,
                                &e.to_string(),
                            );
                        }
                    }
                }
            } else {
                // Characters file is optional, don't log error if it doesn't exist
                self.logger.debug(
                    category::FILE_IO,
                    &format!("Characters file does not exist: {:?}", path),
                );
            }
        } else {
            self.logger.warn(
                category::FILE_IO,
                "No campaign directory set when trying to load characters",
            );
        }
    }

    /// Load races from campaign directory
    pub(crate) fn load_races_from_campaign(&mut self) {
        if let Some(dir) = &self.campaign_dir {
            let path = dir.join(&self.campaign.races_file);
            if path.exists() {
                match self
                    .editor_registry
                    .races_editor_state
                    .load_from_file(&path)
                {
                    Ok(_) => {
                        self.ui_state.status_message = format!(
                            "Loaded {} races",
                            self.editor_registry.races_editor_state.races.len()
                        );
                    }
                    Err(e) => {
                        self.ui_state.status_message = format!("Failed to load races: {}", e);
                        self.logger.error(
                            category::FILE_IO,
                            &format!("Failed to load races from {:?}: {}", path, e),
                        );
                    }
                }
            } else {
                self.logger.debug(
                    category::FILE_IO,
                    &format!("Races file does not exist: {:?}", path),
                );
            }
        } else {
            self.logger.warn(
                category::FILE_IO,
                "No campaign directory set when trying to load races",
            );
        }
    }

    /// Load dialogues from campaign file
    pub(crate) fn load_dialogues(&mut self) -> Result<(), CampaignError> {
        if let Some(dir) = &self.campaign_dir {
            let dialogue_path = dir.join(&self.campaign.dialogue_file);
            if dialogue_path.exists() {
                match std::fs::read_to_string(&dialogue_path) {
                    Ok(contents) => match ron::from_str::<Vec<DialogueTree>>(&contents) {
                        Ok(dialogues) => {
                            self.campaign_data.dialogues = dialogues;
                            self.editor_registry
                                .dialogue_editor_state
                                .load_dialogues(self.campaign_data.dialogues.clone());
                            self.ui_state.status_message =
                                format!("Loaded {} dialogues", self.campaign_data.dialogues.len());
                        }
                        Err(e) => {
                            self.logger.error(
                                category::FILE_IO,
                                &format!(
                                    "Failed to parse dialogues from {:?}: {}",
                                    dialogue_path, e
                                ),
                            );
                            return Err(CampaignError::Deserialization(e));
                        }
                    },
                    Err(e) => {
                        self.logger.error(
                            category::FILE_IO,
                            &format!("Failed to read dialogues file {:?}: {}", dialogue_path, e),
                        );
                        return Err(CampaignError::Io(e));
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle a request emitted by merchant validation workflows to open the NPC
    /// editor for a specific NPC ID.
    pub(crate) fn handle_validation_open_npc_request(&mut self) {
        if let Some(requested_id) = self
            .editor_registry
            .npc_editor_state
            .requested_open_npc
            .take()
        {
            if let Some(idx) = self
                .editor_registry
                .npc_editor_state
                .npcs
                .iter()
                .position(|npc| npc.id == requested_id)
            {
                self.ui_state.active_tab = EditorTab::NPCs;
                self.editor_registry.npc_editor_state.start_edit_npc(idx);
                self.ui_state.status_message = format!("Opening NPC editor for '{}'", requested_id);
            } else {
                self.ui_state.status_message = format!(
                    "Validation requested NPC '{}', but it was not found",
                    requested_id
                );
            }
        }
    }
}
