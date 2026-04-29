// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! NPC Editor for Campaign Builder
//!
//! This module provides a visual NPC editor for creating and managing
//! NPC definitions that can be placed in maps throughout the campaign.
//!
//! The `show()` method provides full UI rendering following the standard editor pattern.
//! Uses shared UI components for consistent layout.
//!
//! # Features
//!
//! - NPC definition list view with search and filtering
//! - Add/Edit/Delete functionality
//! - Fields for all `NpcDefinition` properties
//! - Autocomplete for dialogue_id (from loaded dialogues)
//! - Multi-select for quest_ids (from loaded quests)
//! - Portrait path validation
//! - Import/export RON support
//!
//! # Merchant Dialogue Policy
//!
//! The final merchant dialogue contract is split into two layers:
//!
//! Authoring contract:
//!
//! - merchant-capable dialogue must explicitly contain
//!   `DialogueAction::OpenMerchant { npc_id }` for the merchant NPC
//! - `NpcDefinition::is_merchant` and `NpcDefinition::dialogue_id` are linked
//!   authoring data and must not be treated as unrelated fields
//! - SDK-managed merchant dialogue content must remain distinguishable from
//!   author-authored dialogue content so merchant generation, augmentation,
//!   validation, repair, and removal remain non-destructive
//!
//! Runtime contract:
//!
//! - executing `DialogueAction::OpenMerchant { npc_id }` opens the merchant
//!   inventory for that NPC
//! - pressing `I` while already in dialogue with a merchant NPC remains a
//!   runtime convenience shortcut only
//! - the `I` shortcut is not the content-authoring standard and does not replace
//!   the requirement for explicit `OpenMerchant` in merchant dialogue content
//!
//! This editor is a primary policy touchpoint because it owns the merchant-role
//! toggle, the assigned dialogue reference, the merchant status/help text, and
//! the loaded dialogue collection used to create, augment, validate, repair,
//! and remove SDK-managed merchant content while preserving unrelated custom
//! dialogue where possible.
//!
//! # Architecture
//!
//! Follows standard SDK editor pattern:
//! - `NpcEditorState`: Main editor state with `show()` method
//! - `NpcEditorMode`: List/Add/Edit modes
//! - `NpcEditBuffer`: Form field buffer for editing
//! - Standard UI components: EditorToolbar, TwoColumnLayout

use crate::creature_assets::CreatureAssetManager;
use crate::dialogue_editor::{DialogueEditorState, MerchantDialogueUpdate};
use crate::ui_helpers::{
    autocomplete_creature_selector, autocomplete_portrait_selector, extract_portrait_candidates,
    show_standard_list_item, EditorToolbar, ItemAction, MetadataBadge, StandardListItemConfig,
    ToolbarAction, TwoColumnLayout,
};
use antares::domain::dialogue::{DialogueAction, DialogueId, DialogueTree};
use antares::domain::inventory::{NpcEconomySettings, ServiceCatalog};
use antares::domain::quest::{Quest, QuestId};
use antares::domain::world::npc_runtime::MerchantStockTemplate;
use antares::domain::world::{NpcDefinition, SpriteReference};
use antares::domain::CreatureId;
use antares::sdk::tool_config::DisplayConfig;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;

mod context;
mod portrait_picker;

use self::portrait_picker::show_npc_preview;
pub use context::NpcEditorContext;

/// Errors that can occur in the NPC Editor.
#[derive(Debug, thiserror::Error)]
pub enum NpcEditorError {
    /// OS-level I/O failure.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// The file could not be parsed as RON.
    #[error("Parse error: {0}")]
    Parse(String),
    /// The data could not be serialised to RON.
    #[error("Serialisation error: {0}")]
    Serialization(String),
}

/// Merchant dialogue validation state derived from an NPC plus its assigned
/// dialogue tree.
///
/// This state is used by the NPC editor to surface merchant-dialogue health,
/// drive validation messaging, and choose the appropriate repair action without
/// destructively modifying unrelated dialogue content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MerchantDialogueValidationState {
    /// The NPC is not a merchant and no merchant-specific issue applies.
    NotMerchant,
    /// The merchant has no assigned dialogue tree.
    MissingDialogueId,
    /// The NPC references a dialogue ID that does not exist in the loaded data.
    MissingDialogueTree,
    /// The assigned dialogue contains the correct explicit merchant-opening path.
    Valid,
    /// The assigned dialogue contains SDK-managed merchant content while the NPC
    /// is not a merchant.
    StaleMerchantContent,
    /// The assigned dialogue contains a merchant-opening action for a different
    /// NPC ID.
    WrongMerchantTarget,
    /// The assigned dialogue exists but is missing an explicit merchant-opening
    /// path for this NPC.
    MissingOpenMerchant,
}

/// Merchant dialogue repair action recommended by validation.
///
/// These actions map validation outcomes to the least-destructive repair path
/// available in the NPC editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MerchantDialogueRepairAction {
    /// Create a new merchant dialogue tree and assign it to the NPC.
    CreateDialogue,
    /// Add the standard SDK-managed merchant branch to the assigned dialogue.
    RepairDialogue,
    /// Remove SDK-managed merchant content from a non-merchant NPC dialogue.
    RemoveMerchantContent,
    /// The assigned dialogue is missing from the loaded collection and must be
    /// replaced with a new merchant dialogue.
    ReplaceMissingDialogue,
    /// The assigned dialogue opens the wrong merchant target and should be
    /// repaired by removing stale SDK content and re-applying the correct
    /// merchant branch.
    RebindMerchantTarget,
}

/// Trainer dialogue validation state derived from an NPC plus its assigned
/// dialogue tree.
///
/// This state is used by the NPC editor to surface trainer-dialogue health,
/// drive validation messaging, and choose the appropriate repair action without
/// destructively modifying unrelated dialogue content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainerDialogueValidationState {
    /// The NPC is not a trainer and no trainer-specific issue applies.
    NotTrainer,
    /// The trainer has a valid assigned dialogue with an `OpenTraining` path.
    Valid,
    /// The trainer has no assigned dialogue tree.
    Missing,
    /// The NPC references a dialogue ID that does not exist in the loaded data.
    AssignedDialogueMissing,
    /// The assigned dialogue contains SDK-managed trainer content while the NPC
    /// is not a trainer.
    StaleTrainerContent,
}

/// Editor state for NPC editing.
///
/// Merchant dialogue lifecycle work integrates here in later phases because the
/// editor already owns the merchant-role toggle, the dialogue assignment field,
/// and the loaded dialogue collection used to inspect or repair merchant
/// dialogue compliance.
#[derive(Clone, Serialize, Deserialize)]
pub struct NpcEditorState {
    /// All NPC definitions being edited
    pub npcs: Vec<NpcDefinition>,

    /// Currently selected NPC index
    pub selected_npc: Option<usize>,

    /// NPC editor mode
    pub mode: NpcEditorMode,

    /// Edit buffer for NPC form fields
    pub edit_buffer: NpcEditBuffer,

    /// NPC search/filter string
    pub search_filter: String,

    /// Unsaved changes flag
    pub has_unsaved_changes: bool,

    /// Validation errors for current NPC
    pub validation_errors: Vec<String>,

    /// Available dialogue trees (for dialogue_id autocomplete)
    pub available_dialogues: Vec<DialogueTree>,

    /// Available quests (for quest_ids multi-select)
    pub available_quests: Vec<Quest>,

    /// Whether to show dialogue preview in edit view
    pub show_dialogue_preview: bool,

    /// Whether to show import dialog
    pub show_import_dialog: bool,

    /// Import buffer for RON text
    pub import_buffer: String,

    /// Filter: Show only merchants
    pub filter_merchants: bool,

    /// Filter: Show only innkeepers
    pub filter_innkeepers: bool,

    /// Filter: Show only NPCs with quests
    pub filter_quest_givers: bool,

    /// Filter: Show only trainers
    pub filter_trainers: bool,

    /// Available portrait IDs (cached from directory scan)
    #[serde(skip)]
    pub available_portraits: Vec<String>,

    /// Whether the portrait grid picker popup is open
    #[serde(skip)]
    pub portrait_picker_open: bool,

    /// Available sprite sheet paths (cached from directory scan)
    #[serde(skip)]
    pub available_sprite_sheets: Vec<String>,

    /// Whether the sprite sheet picker popup is open
    #[serde(skip)]
    pub sprite_picker_open: bool,

    /// Whether the creature picker popup is open
    #[serde(skip)]
    pub creature_picker_open: bool,

    /// Available creature candidates (id, name) cached for autocomplete (rebuilt when campaign dir changes)
    #[serde(skip)]
    pub available_creatures: Vec<(u32, String)>,

    /// Cached portrait textures for grid display
    #[serde(skip)]
    pub portrait_textures: HashMap<String, Option<egui::TextureHandle>>,

    /// Last campaign directory (to detect changes)
    #[serde(skip)]
    pub last_campaign_dir: Option<PathBuf>,

    /// Last NPCs filename (cached from show() call)
    #[serde(skip)]
    pub last_npcs_file: Option<String>,

    /// Whether the autocomplete buffers should be reset on next form render
    #[serde(skip)]
    pub reset_autocomplete_buffers: bool,

    /// Stock templates available for merchant assignment (populated by caller)
    #[serde(skip)]
    pub available_stock_templates: Vec<MerchantStockTemplate>,

    /// Set by the UI when the user clicks "✏ Edit template"; consumed by
    /// `CampaignBuilderApp` to switch tab and open the named template.
    #[serde(skip)]
    pub requested_template_edit: Option<String>,

    /// Status message produced by an action inside `show()` (e.g. Reload).
    ///
    /// Because `show()` returns `bool` (unsaved-changes flag) rather than a
    /// status string, the parent reads this field with `.take()` after every
    /// `show()` call and forwards it to `CampaignBuilderApp::status_message`.
    #[serde(skip)]
    pub pending_status: Option<String>,

    /// Dialogue editor state used to create and repair merchant dialogue
    /// assignments from inside the NPC editor.
    #[serde(skip)]
    pub merchant_dialogue_editor: DialogueEditorState,

    /// Optional dialogue ID that the user requested to open from the merchant
    /// dialogue status controls.
    #[serde(skip)]
    pub requested_open_dialogue: Option<DialogueId>,

    /// Optional NPC ID requested from validation/navigation workflows so the
    /// Campaign Builder can jump directly into editing the NPC.
    #[serde(skip)]
    pub requested_open_npc: Option<String>,
}

/// NPC editor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NpcEditorMode {
    /// Viewing list of NPCs
    List,
    /// Creating new NPC
    Add,
    /// Editing existing NPC
    Edit,
}

/// Buffer for NPC form fields.
///
/// The combination of `is_merchant` and `dialogue_id` is a merchant-dialogue
/// policy touchpoint. A merchant NPC is considered valid only when its assigned
/// dialogue tree explicitly contains `DialogueAction::OpenMerchant { npc_id }`.
/// Later phases use this buffer as the source of truth when deciding whether
/// merchant dialogue must be generated, augmented, or cleaned up.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NpcEditBuffer {
    pub id: String,
    pub name: String,
    pub description: String,
    pub portrait_id: String,
    pub dialogue_id: String,
    pub quest_ids: Vec<String>,
    pub faction: String,
    pub is_merchant: bool,
    pub is_innkeeper: bool,
    /// Optional creature ID for procedural mesh rendering
    pub creature_id: String,
    /// Optional sprite sheet path (relative to campaign root, e.g. 'assets/sprites/actors/wizard.png')
    pub sprite_sheet: String,
    /// Optional sprite index (0-based)
    pub sprite_index: String,
    /// ID of the stock template this merchant uses (empty = no template)
    pub stock_template: String,
    /// Whether this NPC is a trainer offering level-up services.
    pub is_trainer: bool,
    /// Per-NPC training fee base (gold per level); empty string = use campaign default.
    pub training_fee_base: String,
    /// Per-NPC training fee multiplier; empty string = use campaign default.
    pub training_fee_multiplier: String,
}

impl Default for NpcEditorState {
    fn default() -> Self {
        Self {
            npcs: Vec::new(),
            selected_npc: None,
            mode: NpcEditorMode::List,
            edit_buffer: NpcEditBuffer::default(),
            search_filter: String::new(),
            has_unsaved_changes: false,
            validation_errors: Vec::new(),
            available_dialogues: Vec::new(),
            available_quests: Vec::new(),
            show_dialogue_preview: false,
            show_import_dialog: false,
            import_buffer: String::new(),
            filter_merchants: false,
            filter_innkeepers: false,
            filter_quest_givers: false,
            filter_trainers: false,
            available_portraits: Vec::new(),
            portrait_picker_open: false,
            available_sprite_sheets: Vec::new(),
            sprite_picker_open: false,
            creature_picker_open: false,
            available_creatures: Vec::new(),
            portrait_textures: HashMap::new(),
            last_campaign_dir: None,
            last_npcs_file: None,
            reset_autocomplete_buffers: false,
            available_stock_templates: Vec::new(),
            requested_template_edit: None,
            pending_status: None,
            merchant_dialogue_editor: DialogueEditorState::default(),
            requested_open_dialogue: None,
            requested_open_npc: None,
        }
    }
}

impl NpcEditorState {
    /// Creates a new NPC editor state
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the creature ID buffer and closes the creature picker.
    pub(crate) fn apply_selected_creature_id(&mut self, id: String) {
        self.edit_buffer.creature_id = id;
        self.creature_picker_open = false;
    }

    /// Shows the NPC editor UI
    ///
    /// Merchant dialogue policy is intentionally surfaced here because this
    /// method receives the loaded dialogue trees that later phases must inspect
    /// for explicit `OpenMerchant` support before persisting merchant NPC
    /// changes.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `dialogues` - Available dialogue trees for autocomplete and merchant dialogue policy checks
    /// * `quests` - Available quests for multi-select
    /// * `npc_ctx` - Context bundle providing campaign directory, display configuration,
    ///   NPCs file path, and optional creature manager
    ///
    /// # Returns
    ///
    /// Returns `true` if changes were made requiring save
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        dialogues: &[DialogueTree],
        quests: &[Quest],
        npc_ctx: &NpcEditorContext<'_>,
    ) -> bool {
        // Update portrait and sprite sheet candidates if campaign directory changed
        if self.last_campaign_dir != npc_ctx.campaign_dir.cloned() {
            self.available_portraits = extract_portrait_candidates(npc_ctx.campaign_dir);
            self.available_sprite_sheets =
                crate::ui_helpers::extract_sprite_sheet_candidates(npc_ctx.campaign_dir);
            // Rebuild creature candidates for autocomplete whenever the campaign dir changes.
            self.available_creatures = npc_ctx
                .creature_manager
                .and_then(|m| m.load_all_creatures().ok())
                .map(|creatures| {
                    creatures
                        .into_iter()
                        .map(|c| (c.id, c.name))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            self.last_campaign_dir = npc_ctx.campaign_dir.cloned();
        }

        // Cache the npcs filename so Save from the editor can persist immediately
        self.last_npcs_file = Some(npc_ctx.npcs_file.to_string());

        // Update available references
        self.available_dialogues = dialogues.to_vec();
        self.available_quests = quests.to_vec();
        self.merchant_dialogue_editor
            .load_dialogues(dialogues.to_vec());

        let mut needs_save = false;

        // Show portrait grid picker popup if open
        if self.portrait_picker_open {
            if let Some(selected_id) =
                self.show_portrait_grid_picker(ui.ctx(), npc_ctx.campaign_dir)
            {
                self.edit_buffer.portrait_id = selected_id;
                needs_save = true;
                // Persist the selected portrait into the autocomplete buffer so the input
                // shows the new selection immediately (avoids stale typed text).
                crate::ui_helpers::store_autocomplete_buffer(
                    ui.ctx(),
                    egui::Id::new("autocomplete:portrait:npc_edit_portrait".to_string()),
                    &self.edit_buffer.portrait_id,
                );
            }
        }

        // Show sprite sheet picker popup if open
        if self.sprite_picker_open {
            if let Some(selected) = self.show_sprite_sheet_picker(ui.ctx(), npc_ctx.campaign_dir) {
                self.edit_buffer.sprite_sheet = selected;
                needs_save = true;
                crate::ui_helpers::store_autocomplete_buffer(
                    ui.ctx(),
                    egui::Id::new("autocomplete:sprite:npc_edit_sprite".to_string()),
                    &self.edit_buffer.sprite_sheet,
                );
            }
        }

        // Toolbar
        let toolbar_action = EditorToolbar::new("NPCs").show(ui);

        // Handle toolbar actions
        match toolbar_action {
            ToolbarAction::New => self.start_add_npc(),
            ToolbarAction::Import => self.show_import_dialog = true,
            ToolbarAction::Export => {
                needs_save |= self.export_all_npcs();
            }
            ToolbarAction::Reload => {
                if let Some(dir) = npc_ctx.campaign_dir {
                    let path = dir.join(npc_ctx.npcs_file);
                    if path.exists() {
                        match self.load_from_file(&path) {
                            Ok(()) => {
                                self.pending_status =
                                    Some(format!("Reloaded {} NPCs from disk", self.npcs.len()));
                            }
                            Err(e) => {
                                self.pending_status = Some(format!("Failed to reload NPCs: {}", e));
                            }
                        }
                    } else {
                        self.pending_status =
                            Some(format!("NPCs file not found: {}", path.display()));
                    }
                } else {
                    self.pending_status = Some("No campaign directory set".to_string());
                }
            }
            _ => {}
        }

        ui.separator();

        // Filters (only shown in list mode)
        if self.mode == NpcEditorMode::List {
            ui.horizontal(|ui| {
                ui.label("🔍 Search:");
                ui.text_edit_singleline(&mut self.search_filter);

                ui.separator();

                if ui
                    .selectable_label(self.filter_merchants, "🏪 Merchants")
                    .clicked()
                {
                    self.filter_merchants = !self.filter_merchants;
                }

                if ui
                    .selectable_label(self.filter_innkeepers, "🛏️ Innkeepers")
                    .clicked()
                {
                    self.filter_innkeepers = !self.filter_innkeepers;
                }

                if ui
                    .selectable_label(self.filter_quest_givers, "📜 Quest Givers")
                    .clicked()
                {
                    self.filter_quest_givers = !self.filter_quest_givers;
                }

                if ui
                    .selectable_label(self.filter_trainers, "🎓 Trainers")
                    .clicked()
                {
                    self.filter_trainers = !self.filter_trainers;
                }

                ui.separator();

                if ui.button("🔄 Clear Filters").clicked() {
                    self.search_filter.clear();
                    self.filter_merchants = false;
                    self.filter_innkeepers = false;
                    self.filter_quest_givers = false;
                    self.filter_trainers = false;
                }
            });

            ui.separator();
        }

        // Mode-specific UI
        match self.mode {
            NpcEditorMode::List => {
                needs_save |= self.show_list_view(
                    ui,
                    npc_ctx.display_config,
                    npc_ctx.campaign_dir,
                    npc_ctx.creature_manager,
                );
            }
            NpcEditorMode::Add | NpcEditorMode::Edit => {
                needs_save |= self.show_edit_view(
                    ui,
                    npc_ctx.campaign_dir,
                    npc_ctx.npcs_file,
                    npc_ctx.creature_manager,
                );
            }
        }

        // Import dialog
        if self.show_import_dialog {
            self.show_import_dialog_window(ui.ctx());
        }

        needs_save
    }

    fn show_list_view(
        &mut self,
        ui: &mut egui::Ui,
        _display_config: &DisplayConfig,
        campaign_dir: Option<&PathBuf>,
        creature_manager: Option<&CreatureAssetManager>,
    ) -> bool {
        let mut needs_save = false;

        let _search_lower = self.search_filter.to_lowercase();

        // Build filtered list snapshot to avoid borrow conflicts in closures
        let filtered_npcs: Vec<(usize, NpcDefinition)> = self
            .npcs
            .iter()
            .enumerate()
            .filter(|(_, npc)| self.matches_filters(npc))
            .map(|(idx, npc)| (idx, npc.clone()))
            .collect();

        // Sort by ID
        let mut sorted_npcs = filtered_npcs;
        sorted_npcs.sort_by(|(_, a), (_, b)| a.id.cmp(&b.id));

        let selected = self.selected_npc;
        let mut new_selection = selected;
        let mut action_requested: Option<(usize, ItemAction)> = None;

        ui.separator();

        let _total_width = ui.available_width();
        let _inspector_min_width = 300.0;
        // Reserve a small margin for the separator (12.0)
        let _sep_margin = 12.0;

        // Pre-compute merchant dialogue status and validation for each NPC
        // before entering closures, to avoid borrowing `self` in the left closure
        // (which would conflict with the mutable borrow of `self.portrait_textures`
        // in the right closure).
        let merchant_info: std::collections::HashMap<
            usize,
            (&'static str, bool, MerchantDialogueValidationState),
        > = sorted_npcs
            .iter()
            .map(|(idx, npc)| {
                let (status, sdk_managed) = self.merchant_dialogue_status_for_definition(npc);
                let validation = self.merchant_dialogue_validation_for_definition(npc);
                (*idx, (status, sdk_managed, validation))
            })
            .collect();

        // Pre-compute trainer validation state for each NPC for the same reason.
        let trainer_info: std::collections::HashMap<usize, TrainerDialogueValidationState> =
            sorted_npcs
                .iter()
                .map(|(idx, npc)| {
                    let validation = self.trainer_dialogue_validation_for_definition(npc);
                    (*idx, validation)
                })
                .collect();

        TwoColumnLayout::new("npcs").show_split(
            ui,
            |left_ui| {
                // Left panel: NPC list (styled to match Characters editor)
                left_ui.heading("NPCs");
                left_ui.separator();

                if sorted_npcs.is_empty() {
                    left_ui.label("No NPCs found");
                } else {
                    for (idx, npc) in &sorted_npcs {
                        left_ui.push_id(*idx, |ui| {
                            let is_selected = selected == Some(*idx);
                            let mut badges = Vec::new();

                            let (merchant_status, merchant_sdk_managed, merchant_validation_state) =
                                merchant_info
                                    .get(idx)
                                    .copied()
                                    .unwrap_or(("Unknown", false, MerchantDialogueValidationState::NotMerchant));

                            if npc.is_merchant {
                                let (merchant_badge_text, merchant_badge_color, merchant_tooltip) =
                                    match merchant_validation_state {
                                        MerchantDialogueValidationState::Valid => (
                                            "Merchant",
                                            egui::Color32::GOLD,
                                            format!(
                                                "This NPC is a merchant. Merchant dialogue status: {}",
                                                merchant_status
                                            ),
                                        ),
                                        MerchantDialogueValidationState::MissingDialogueId => (
                                            "Merchant!",
                                            egui::Color32::from_rgb(255, 120, 120),
                                            "This merchant has no dialogue assigned".to_string(),
                                        ),
                                        MerchantDialogueValidationState::MissingDialogueTree => (
                                            "Merchant!",
                                            egui::Color32::from_rgb(255, 120, 120),
                                            "This merchant references a missing dialogue tree"
                                                .to_string(),
                                        ),
                                        MerchantDialogueValidationState::WrongMerchantTarget => (
                                            "Merchant!",
                                            egui::Color32::from_rgb(255, 180, 0),
                                            "This merchant's dialogue opens the wrong merchant target"
                                                .to_string(),
                                        ),
                                        MerchantDialogueValidationState::MissingOpenMerchant => (
                                            "Merchant!",
                                            egui::Color32::from_rgb(255, 180, 0),
                                            "This merchant's dialogue is missing explicit OpenMerchant"
                                                .to_string(),
                                        ),
                                        MerchantDialogueValidationState::StaleMerchantContent => (
                                            "Merchant!",
                                            egui::Color32::from_rgb(255, 180, 0),
                                            "This merchant has stale merchant dialogue content"
                                                .to_string(),
                                        ),
                                        MerchantDialogueValidationState::NotMerchant => (
                                            "Merchant",
                                            egui::Color32::GOLD,
                                            format!(
                                                "This NPC is a merchant. Merchant dialogue status: {}",
                                                merchant_status
                                            ),
                                        ),
                                    };

                                badges.push(
                                    MetadataBadge::new(merchant_badge_text)
                                        .with_color(merchant_badge_color)
                                        .with_tooltip(merchant_tooltip),
                                );

                                if merchant_sdk_managed {
                                    badges.push(
                                        MetadataBadge::new("Merchant SDK")
                                            .with_color(egui::Color32::from_rgb(100, 200, 180))
                                            .with_tooltip(
                                                "Assigned dialogue contains SDK-managed merchant content",
                                            ),
                                    );
                                }
                            } else if merchant_validation_state
                                == MerchantDialogueValidationState::StaleMerchantContent
                            {
                                badges.push(
                                    MetadataBadge::new("Stale Merchant")
                                        .with_color(egui::Color32::from_rgb(255, 180, 0))
                                        .with_tooltip(
                                            "This non-merchant NPC still references dialogue with SDK-managed merchant content",
                                        ),
                                );
                            }
                            let trainer_validation_state = trainer_info
                                .get(idx)
                                .copied()
                                .unwrap_or(TrainerDialogueValidationState::NotTrainer);
                            if npc.is_trainer {
                                let (trainer_badge_text, trainer_badge_color, trainer_tooltip) =
                                    match trainer_validation_state {
                                        TrainerDialogueValidationState::Valid => (
                                            "🎓 Trainer",
                                            egui::Color32::from_rgb(180, 100, 220),
                                            "This NPC offers level-up training with a valid training dialogue.",
                                        ),
                                        TrainerDialogueValidationState::Missing => (
                                            "🎓 Trainer!",
                                            egui::Color32::from_rgb(255, 120, 120),
                                            "This trainer has no dialogue assigned.",
                                        ),
                                        TrainerDialogueValidationState::AssignedDialogueMissing => (
                                            "🎓 Trainer!",
                                            egui::Color32::from_rgb(255, 120, 120),
                                            "This trainer references a missing dialogue tree.",
                                        ),
                                        TrainerDialogueValidationState::StaleTrainerContent
                                        | TrainerDialogueValidationState::NotTrainer => (
                                            "🎓 Trainer",
                                            egui::Color32::from_rgb(180, 100, 220),
                                            "This NPC is a trainer.",
                                        ),
                                    };
                                badges.push(
                                    MetadataBadge::new(trainer_badge_text)
                                        .with_color(trainer_badge_color)
                                        .with_tooltip(trainer_tooltip),
                                );
                            } else if trainer_validation_state
                                == TrainerDialogueValidationState::StaleTrainerContent
                            {
                                badges.push(
                                    MetadataBadge::new("Stale Trainer")
                                        .with_color(egui::Color32::from_rgb(255, 180, 0))
                                        .with_tooltip(
                                            "This non-trainer NPC still references dialogue with SDK-managed trainer content.",
                                        ),
                                );
                            }
                            if npc.is_innkeeper {
                                badges.push(
                                    MetadataBadge::new("Innkeeper")
                                        .with_color(egui::Color32::LIGHT_BLUE)
                                        .with_tooltip("This NPC is an innkeeper"),
                                );
                            }
                            if !npc.quest_ids.is_empty() {
                                badges.push(
                                    MetadataBadge::new(format!("Quests:{}", npc.quest_ids.len()))
                                        .with_color(egui::Color32::from_rgb(200, 180, 100))
                                        .with_tooltip("Number of associated quests"),
                                );
                            }
                            if npc.dialogue_id.is_some() {
                                badges.push(
                                    MetadataBadge::new("Dialogue")
                                        .with_color(egui::Color32::from_rgb(100, 200, 180))
                                        .with_tooltip("Has dialogue tree"),
                                );
                            }
                            if let Some(faction) = npc.faction.as_deref() {
                                if !faction.trim().is_empty() {
                                    badges.push(
                                        MetadataBadge::new(format!("Faction:{}", faction))
                                            .with_color(egui::Color32::from_rgb(170, 170, 200))
                                            .with_tooltip("NPC faction"),
                                    );
                                }
                            }

                            let config = StandardListItemConfig::new(&npc.name)
                                .with_badges(badges)
                                .selected(is_selected);

                            let (clicked, ctx_action) = show_standard_list_item(ui, config);
                            if clicked {
                                new_selection = Some(*idx);
                            }
                            if ctx_action != ItemAction::None {
                                action_requested = Some((*idx, ctx_action));
                            }

                            ui.add_space(4.0);
                        });
                    }
                }
            },
            |right_ui| {
                // Right panel: Detail view
                if let Some(idx) = selected {
                    if let Some((_, npc)) = sorted_npcs.iter().find(|(i, _)| *i == idx) {
                        show_npc_preview(
                            right_ui,
                            npc,
                            campaign_dir,
                            creature_manager,
                            &self.available_dialogues,
                            &mut self.portrait_textures,
                        );
                    } else {
                        right_ui.vertical_centered(|ui| {
                            ui.add_space(100.0);
                            ui.label("Select an NPC to view details");
                        });
                    }
                } else {
                    right_ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("Select an NPC to view details");
                    });
                }
            },
        );

        // Apply selection change after closures
        self.selected_npc = new_selection;

        // Handle action button clicks after closures
        if let Some((idx, action)) = action_requested {
            match action {
                ItemAction::Edit => {
                    if idx < self.npcs.len() {
                        self.selected_npc = Some(idx);
                        self.start_edit_npc(idx);
                    }
                }
                ItemAction::Delete => {
                    if idx < self.npcs.len() {
                        self.npcs.remove(idx);
                        self.selected_npc = None;
                        needs_save = true;
                    }
                }
                ItemAction::Duplicate => {
                    if idx < self.npcs.len() {
                        let mut new_npc = self.npcs[idx].clone();
                        let next_id = self.next_npc_id();
                        new_npc.id = next_id;
                        new_npc.name = format!("{} (Copy)", new_npc.name);
                        self.npcs.push(new_npc);
                        needs_save = true;
                    }
                }
                ItemAction::Export => {
                    if idx < self.npcs.len() {
                        if let Ok(ron_str) = ron::ser::to_string_pretty(
                            &self.npcs[idx],
                            ron::ser::PrettyConfig::default(),
                        ) {
                            self.import_buffer = ron_str;
                            self.show_import_dialog = true;
                        }
                    }
                }
                ItemAction::None => {}
            }
        }

        needs_save
    }

    fn show_edit_view(
        &mut self,
        ui: &mut egui::Ui,
        campaign_dir: Option<&PathBuf>,
        npcs_file: &str,
        creature_manager: Option<&CreatureAssetManager>,
    ) -> bool {
        let mut needs_save = false;
        let is_add = self.mode == NpcEditorMode::Add;

        ui.heading(if is_add { "Add New NPC" } else { "Edit NPC" });
        ui.separator();

        // If requested, reset persistent autocomplete buffers so the form displays
        // values from the newly loaded buffer rather than stale typed text.
        if self.reset_autocomplete_buffers {
            let ctx = ui.ctx();
            crate::ui_helpers::remove_autocomplete_buffer(
                ctx,
                egui::Id::new("autocomplete:portrait:npc_edit_portrait".to_string()),
            );
            crate::ui_helpers::remove_autocomplete_buffer(
                ctx,
                egui::Id::new("autocomplete:creature:npc_creature".to_string()),
            );
            self.reset_autocomplete_buffers = false;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.heading("Basic Information");

                    ui.horizontal(|ui| {
                        ui.label("ID:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.edit_buffer.id)
                                .id_salt("npc_edit_id"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.edit_buffer.name)
                                .id_salt("npc_edit_name"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Description:");
                    });
                    ui.add(
                        egui::TextEdit::multiline(&mut self.edit_buffer.description)
                            .desired_rows(3)
                            .id_salt("npc_edit_description"),
                    );
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Appearance");

                    ui.horizontal(|ui| {
                        ui.label("Portrait ID:");
                        if autocomplete_portrait_selector(
                            ui,
                            "npc_edit_portrait",
                            "",
                            &mut self.edit_buffer.portrait_id,
                            &self.available_portraits,
                            campaign_dir,
                        ) {
                            needs_save = true;
                        }

                        // Grid picker button
                        if ui.button("🖼").on_hover_text("Browse portraits").clicked() {
                            self.portrait_picker_open = true;
                        }
                    });
                    ui.label(
                        "📁 Relative to campaign assets directory (e.g., '0', '1', 'warrior')",
                    );

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Sprite Sheet:");
                        if crate::ui_helpers::autocomplete_sprite_sheet_selector(
                            ui,
                            "npc_edit_sprite",
                            "",
                            &mut self.edit_buffer.sprite_sheet,
                            &self.available_sprite_sheets,
                            campaign_dir,
                        ) {
                            needs_save = true;
                        }

                        if ui.button("📁").on_hover_text("Browse sprite sheets").clicked() {
                            self.sprite_picker_open = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Sprite Index:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.edit_buffer.sprite_index)
                                .id_salt("npc_edit_sprite_index"),
                        );
                        ui.label("0-based index (row-major)");
                    });

                    ui.label(
                        "📁 Sprite sheets are relative to campaign assets (e.g., 'assets/sprites/actors/wizard.png')",
                    );

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Creature ID:");
                        // Autocomplete input — type by name or numeric ID
                        if autocomplete_creature_selector(
                            ui,
                            "npc_creature",
                            "",
                            &mut self.edit_buffer.creature_id,
                            &self.available_creatures,
                        ) {
                            needs_save = true;
                        }

                        // Grid picker button
                        if ui
                            .button("Browse…")
                            .on_hover_text("Select a creature asset")
                            .clicked()
                        {
                            self.creature_picker_open = true;
                        }
                        ui.label("ℹ").on_hover_text(
                            "Links this NPC to a procedural mesh creature definition. When set, \
                             the NPC spawns as a 3-D creature mesh on the map instead of a \
                             sprite placeholder.",
                        );
                    });
                });

                // Creature picker modal
                if self.creature_picker_open {
                    if let Some(manager) = creature_manager {
                        let creatures = manager.load_all_creatures().unwrap_or_default();
                        let mut picked_id: Option<String> = None;
                        let mut should_close = false;
                        egui::Window::new("Select Creature")
                            .id(egui::Id::new("npc_creature_picker"))
                            .resizable(true)
                            .show(ui.ctx(), |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("npc_creature_picker_scroll")
                                    .max_height(300.0)
                                    .show(ui, |ui| {
                                        for creature in &creatures {
                                            ui.push_id(creature.id, |ui| {
                                                let selected = self.edit_buffer.creature_id
                                                    == creature.id.to_string();
                                                if ui
                                                    .selectable_label(
                                                        selected,
                                                        format!(
                                                            "{} — {}",
                                                            creature.id, creature.name
                                                        ),
                                                    )
                                                    .clicked()
                                                {
                                                    picked_id =
                                                        Some(creature.id.to_string());
                                                }
                                            });
                                        }
                                    });
                                if ui.button("Close").clicked() {
                                    should_close = true;
                                }
                            });
                        if let Some(id) = picked_id {
                            self.apply_selected_creature_id(id.clone());
                            // Sync the autocomplete buffer so the text field shows the
                            // resolved "id — name" display string immediately.
                            let display = creatures
                                .iter()
                                .find(|c| c.id.to_string() == id)
                                .map(|c| format!("{} — {}", c.id, c.name))
                                .unwrap_or_else(|| id.clone());
                            crate::ui_helpers::store_autocomplete_buffer(
                                ui.ctx(),
                                egui::Id::new("autocomplete:creature:npc_creature".to_string()),
                                &display,
                            );
                        } else if should_close {
                            self.creature_picker_open = false;
                        }
                    } else {
                        self.creature_picker_open = false;
                    }
                }

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Dialogue & Quests");

                    ui.horizontal(|ui| {
                        ui.label("Dialogue ID:");
                        egui::ComboBox::from_id_salt("npc_edit_dialogue_select")
                            .selected_text(if self.edit_buffer.dialogue_id.is_empty() {
                                "None".to_string()
                            } else {
                                self.edit_buffer.dialogue_id.clone()
                            })
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(
                                        self.edit_buffer.dialogue_id.is_empty(),
                                        "None",
                                    )
                                    .clicked()
                                {
                                    self.edit_buffer.dialogue_id.clear();
                                }

                                for dialogue in &self.available_dialogues {
                                    let label = format!("{} - {}", dialogue.id, dialogue.name);
                                    if ui
                                        .selectable_label(
                                            self.edit_buffer.dialogue_id == dialogue.id.to_string(),
                                            &label,
                                        )
                                        .clicked()
                                    {
                                        self.edit_buffer.dialogue_id = dialogue.id.to_string();
                                    }
                                }
                            });
                    });

                    ui.separator();

                    ui.label("Associated Quests:");
                    egui::ScrollArea::vertical()
                        .id_salt("npc_edit_quests_scroll")
                        .max_height(150.0)
                        .show(ui, |ui| {
                            for (quest_idx, quest) in self.available_quests.iter().enumerate() {
                                ui.push_id(format!("npc_edit_quest_{}", quest_idx), |ui| {
                                    let quest_id_str = quest.id.to_string();
                                    let mut is_selected =
                                        self.edit_buffer.quest_ids.contains(&quest_id_str);

                                    if ui
                                        .checkbox(
                                            &mut is_selected,
                                            format!("{} - {}", quest.id, quest.name),
                                        )
                                        .on_hover_text(format!("Quest ID: {}", quest.id))
                                        .clicked()
                                    {
                                        if is_selected {
                                            if !self.edit_buffer.quest_ids.contains(&quest_id_str) {
                                                self.edit_buffer.quest_ids.push(quest_id_str);
                                            }
                                        } else {
                                            self.edit_buffer
                                                .quest_ids
                                                .retain(|id| id != &quest_id_str);
                                        }
                                    }
                                });
                            }
                        });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Faction & Roles");

                    ui.horizontal(|ui| {
                        ui.label("Faction:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.edit_buffer.faction)
                                .id_salt("npc_edit_faction"),
                        );
                    });

                    let was_merchant = self.edit_buffer.is_merchant;
                    ui.checkbox(&mut self.edit_buffer.is_merchant, "🏪 Is Merchant");

                    if self.edit_buffer.is_merchant && !was_merchant {
                        match self.auto_apply_merchant_dialogue_to_edit_buffer() {
                            Ok(message) => {
                                self.pending_status = Some(message);
                                needs_save = true;
                            }
                            Err(error) => {
                                self.validation_errors.push(error.clone());
                                self.pending_status = Some(error);
                            }
                        }
                    } else if !self.edit_buffer.is_merchant && was_merchant {
                        match self.remove_merchant_dialogue_from_edit_buffer() {
                            Ok(message) => {
                                self.pending_status = Some(message);
                                needs_save = true;
                            }
                            Err(error) => {
                                self.validation_errors.push(error.clone());
                                self.pending_status = Some(error);
                            }
                        }
                    }

                    let (status_label, sdk_managed) =
                        self.merchant_dialogue_status_for_buffer();
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let badge_color = if status_label == "Merchant dialogue valid" {
                            egui::Color32::from_rgb(80, 200, 120)
                        } else if status_label == "SDK-managed merchant branch present" {
                            egui::Color32::from_rgb(100, 200, 180)
                        } else if status_label == "Merchant dialogue missing OpenMerchant" {
                            egui::Color32::from_rgb(255, 180, 0)
                        } else if status_label == "No dialogue assigned" {
                            egui::Color32::from_rgb(220, 120, 120)
                        } else {
                            egui::Color32::from_rgb(160, 200, 255)
                        };

                        ui.label(
                            egui::RichText::new(status_label)
                                .color(badge_color)
                                .strong(),
                        );

                        if sdk_managed {
                            ui.small("(SDK managed)");
                        }
                    });

                    if !self.edit_buffer.is_merchant {
                        ui.small(
                            "Disabling merchant removes only SDK-managed merchant branch/action content. The assigned dialogue asset remains in place and unrelated non-merchant dialogue content is preserved.",
                        );
                    }

                    ui.horizontal(|ui| {
                        // "Create" and "Repair" only make sense for merchant NPCs.
                        // Previously these were always shown; clicking them on a
                        // non-merchant NPC silently returned an empty status message.
                        if self.edit_buffer.is_merchant {
                            if ui.button("Create merchant dialogue").clicked() {
                                match self.create_or_repair_merchant_dialogue_for_buffer() {
                                    Ok(message) => {
                                        self.pending_status = Some(message);
                                        needs_save = true;
                                    }
                                    Err(error) => {
                                        self.validation_errors.push(error.clone());
                                        self.pending_status = Some(error);
                                    }
                                }
                            }

                            if ui.button("Repair merchant dialogue").clicked() {
                                match self.repair_merchant_dialogue_for_buffer() {
                                    Ok(message) => {
                                        self.pending_status = Some(message);
                                        needs_save = true;
                                    }
                                    Err(error) => {
                                        self.validation_errors.push(error.clone());
                                        self.pending_status = Some(error);
                                    }
                                }
                            }
                        }

                        if ui.button("Remove merchant branch").clicked() {
                            match self.remove_merchant_dialogue_from_edit_buffer() {
                                Ok(message) => {
                                    self.pending_status = Some(message);
                                    needs_save = true;
                                }
                                Err(error) => {
                                    self.validation_errors.push(error.clone());
                                    self.pending_status = Some(error);
                                }
                            }
                        }

                        let open_enabled = !self.edit_buffer.dialogue_id.trim().is_empty();
                        if ui
                            .add_enabled(
                                open_enabled,
                                egui::Button::new("Open assigned dialogue"),
                            )
                            .clicked()
                        {
                            if let Ok(dialogue_id) =
                                self.edit_buffer.dialogue_id.parse::<DialogueId>()
                            {
                                self.requested_open_dialogue = Some(dialogue_id);
                                self.pending_status = Some(format!(
                                    "Open assigned dialogue {} from the Dialogues tab",
                                    dialogue_id
                                ));
                            }
                        }
                    });

                    ui.small(
                        "SDK workflow: enabling merchant creates or repairs dialogue automatically, existing custom dialogue is augmented instead of replaced when possible, validation can repair broken merchant states later, and disabling merchant removes only SDK-managed merchant content.",
                    );

                    if self.edit_buffer.is_merchant {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Stock Template:");
                            egui::ComboBox::from_id_salt("npc_edit_stock_template_select")
                                .selected_text(if self.edit_buffer.stock_template.is_empty() {
                                    "None (no stock)".to_string()
                                } else {
                                    self.edit_buffer.stock_template.clone()
                                })
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(
                                            self.edit_buffer.stock_template.is_empty(),
                                            "None (no stock)",
                                        )
                                        .clicked()
                                    {
                                        self.edit_buffer.stock_template.clear();
                                    }
                                    for tmpl in &self.available_stock_templates {
                                        ui.push_id(&tmpl.id, |ui| {
                                            if ui
                                                .selectable_label(
                                                    self.edit_buffer.stock_template == tmpl.id,
                                                    &tmpl.id,
                                                )
                                                .on_hover_text(format!(
                                                    "{} entries, {} magic slots",
                                                    tmpl.entries.len(),
                                                    tmpl.magic_slot_count
                                                ))
                                                .clicked()
                                            {
                                                self.edit_buffer.stock_template =
                                                    tmpl.id.clone();
                                            }
                                        });
                                    }
                                });

                            if !self.edit_buffer.stock_template.is_empty()
                                && ui.small_button("✏ Edit template").clicked() {
                                    // Signal the caller to navigate to the Stock Templates tab
                                    // and open this template for editing.
                                    self.requested_template_edit =
                                        Some(self.edit_buffer.stock_template.clone());
                                }
                        });
                    }

                    ui.add_space(4.0);
                    ui.separator();

                    let was_trainer = self.edit_buffer.is_trainer;
                    ui.checkbox(&mut self.edit_buffer.is_trainer, "🎓 Is Trainer");

                    if self.edit_buffer.is_trainer && !was_trainer {
                        match self.auto_apply_trainer_dialogue_to_edit_buffer() {
                            Ok(message) => {
                                self.pending_status = Some(message);
                                needs_save = true;
                            }
                            Err(error) => {
                                self.validation_errors.push(error.clone());
                                self.pending_status = Some(error);
                            }
                        }
                    } else if !self.edit_buffer.is_trainer && was_trainer {
                        match self.remove_trainer_dialogue_from_edit_buffer() {
                            Ok(message) => {
                                self.pending_status = Some(message);
                                needs_save = true;
                            }
                            Err(error) => {
                                self.validation_errors.push(error.clone());
                                self.pending_status = Some(error);
                            }
                        }
                    }

                    if self.edit_buffer.is_trainer {
                        let trainer_status = self.trainer_dialogue_status_for_buffer();
                        let trainer_color = if trainer_status == "Trainer dialogue valid" {
                            egui::Color32::from_rgb(80, 200, 120)
                        } else if trainer_status == "No dialogue assigned" {
                            egui::Color32::from_rgb(220, 120, 120)
                        } else {
                            egui::Color32::from_rgb(160, 200, 255)
                        };
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(trainer_status)
                                .color(trainer_color)
                                .strong(),
                        );

                        ui.horizontal(|ui| {
                            ui.label("Training Fee Base (gold per level):");
                            ui.add(
                                egui::TextEdit::singleline(
                                    &mut self.edit_buffer.training_fee_base,
                                )
                                .id_salt("npc_edit_training_fee_base"),
                            );
                            ui.small("empty = campaign default");
                        });

                        ui.horizontal(|ui| {
                            ui.label("Training Fee Multiplier:");
                            ui.add(
                                egui::TextEdit::singleline(
                                    &mut self.edit_buffer.training_fee_multiplier,
                                )
                                .id_salt("npc_edit_training_fee_multiplier"),
                            );
                            ui.small("empty = campaign default");
                        });

                        ui.horizontal(|ui| {
                            if ui.button("Create trainer dialogue").clicked() {
                                match self.create_or_repair_trainer_dialogue_for_buffer() {
                                    Ok(message) => {
                                        self.pending_status = Some(message);
                                        needs_save = true;
                                    }
                                    Err(error) => {
                                        self.validation_errors.push(error.clone());
                                        self.pending_status = Some(error);
                                    }
                                }
                            }

                            if ui.button("Repair trainer dialogue").clicked() {
                                match self.create_or_repair_trainer_dialogue_for_buffer() {
                                    Ok(message) => {
                                        self.pending_status = Some(message);
                                        needs_save = true;
                                    }
                                    Err(error) => {
                                        self.validation_errors.push(error.clone());
                                        self.pending_status = Some(error);
                                    }
                                }
                            }

                            if ui.button("Remove trainer branch").clicked() {
                                match self.remove_trainer_dialogue_from_edit_buffer() {
                                    Ok(message) => {
                                        self.pending_status = Some(message);
                                        needs_save = true;
                                    }
                                    Err(error) => {
                                        self.validation_errors.push(error.clone());
                                        self.pending_status = Some(error);
                                    }
                                }
                            }
                        });

                        ui.small(
                            "SDK workflow: enabling trainer creates or auto-applies a training dialogue, \
                             existing custom dialogue is augmented where possible, and disabling trainer \
                             removes only SDK-managed trainer content.",
                        );
                    }

                    ui.add_space(4.0);
                    ui.checkbox(&mut self.edit_buffer.is_innkeeper, "🛏️ Is Innkeeper");
                });

                ui.add_space(10.0);

                // Validation errors
                if !self.validation_errors.is_empty() {
                    ui.group(|ui| {
                        ui.heading("⚠️ Validation Errors");
                        for error in &self.validation_errors {
                            ui.colored_label(egui::Color32::RED, error);
                        }
                    });
                    ui.add_space(10.0);
                }

                // Action buttons
                ui.horizontal_wrapped(|ui| {
                    if ui.button("⬅ Back to List").clicked() {
                        self.mode = NpcEditorMode::List;
                        ui.ctx().request_repaint();
                    }

                    if ui.button("💾 Save").clicked() {
                        self.validate_edit_buffer();
                        if self.validation_errors.is_empty() {
                            let merchant_result = if self.edit_buffer.is_merchant {
                                self.auto_apply_merchant_dialogue_to_edit_buffer()
                            } else {
                                self.remove_merchant_dialogue_from_edit_buffer()
                            };

                            match merchant_result {
                                Ok(message) => {
                                    if !message.is_empty() {
                                        self.pending_status = Some(message);
                                    }
                                }
                                Err(error) => {
                                    self.validation_errors.push(error.clone());
                                    self.pending_status = Some(error);
                                }
                            }

                            if self.validation_errors.is_empty() {
                                let trainer_result = if self.edit_buffer.is_trainer {
                                    self.auto_apply_trainer_dialogue_to_edit_buffer()
                                } else {
                                    self.remove_trainer_dialogue_from_edit_buffer()
                                };

                                match trainer_result {
                                    Ok(message) => {
                                        if !message.is_empty() {
                                            self.pending_status = Some(message);
                                        }
                                    }
                                    Err(error) => {
                                        self.validation_errors.push(error.clone());
                                        self.pending_status = Some(error);
                                    }
                                }
                            }
                        }

                        if self.validation_errors.is_empty()
                            && self.save_npc() {
                                needs_save = true;
                                if let Some(dir) = campaign_dir {
                                    let path = dir.join(npcs_file);
                                    match self.save_to_file(&path) {
                                        Ok(_) => {
                                            self.has_unsaved_changes = false;
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "Saved {}",
                                                    path.display()
                                                ))
                                                .color(egui::Color32::from_rgb(80, 200, 120)),
                                            );
                                        }
                                        Err(e) => {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "Failed to save NPCs: {}",
                                                    e
                                                ))
                                                .color(egui::Color32::RED),
                                            );
                                        }
                                    }
                                }
                                self.mode = NpcEditorMode::List;
                            }
                    }

                    if ui.button("❌ Cancel").clicked() {
                        self.mode = NpcEditorMode::List;
                        self.edit_buffer = NpcEditBuffer::default();
                        ui.ctx().request_repaint();
                    }
                });
            });

        needs_save
    }

    fn matches_filters(&self, npc: &NpcDefinition) -> bool {
        // Search filter
        let search_lower = self.search_filter.to_lowercase();
        if !search_lower.is_empty()
            && !npc.name.to_lowercase().contains(&search_lower)
            && !npc.id.to_lowercase().contains(&search_lower)
        {
            return false;
        }

        // Merchant filter
        if self.filter_merchants && !npc.is_merchant {
            return false;
        }

        // Innkeeper filter
        if self.filter_innkeepers && !npc.is_innkeeper {
            return false;
        }

        // Quest giver filter
        if self.filter_quest_givers && npc.quest_ids.is_empty() {
            return false;
        }

        // Trainer filter
        if self.filter_trainers && !npc.is_trainer {
            return false;
        }

        true
    }

    fn start_add_npc(&mut self) {
        self.mode = NpcEditorMode::Add;
        self.edit_buffer = NpcEditBuffer::default();
        self.edit_buffer.id = self.next_npc_id();
        // Ensure autocomplete widgets show values from the fresh buffer
        self.reset_autocomplete_buffers = true;
    }

    pub(crate) fn start_edit_npc(&mut self, idx: usize) {
        if let Some(npc) = self.npcs.get(idx) {
            self.edit_buffer = NpcEditBuffer {
                id: npc.id.clone(),
                name: npc.name.clone(),
                description: npc.description.clone(),
                portrait_id: npc.portrait_id.clone(),
                dialogue_id: npc
                    .dialogue_id
                    .as_ref()
                    .map_or(String::new(), |id| id.to_string()),
                creature_id: npc
                    .creature_id
                    .as_ref()
                    .map_or(String::new(), |id| id.to_string()),
                quest_ids: npc.quest_ids.iter().map(|id| id.to_string()).collect(),
                faction: npc.faction.as_ref().unwrap_or(&String::new()).clone(),
                is_merchant: npc.is_merchant,
                is_innkeeper: npc.is_innkeeper,
                sprite_sheet: npc
                    .sprite
                    .as_ref()
                    .map_or(String::new(), |s| s.sheet_path.clone()),
                sprite_index: npc
                    .sprite
                    .as_ref()
                    .map_or(String::new(), |s| s.sprite_index.to_string()),
                stock_template: npc.stock_template.clone().unwrap_or_default(),
                is_trainer: npc.is_trainer,
                training_fee_base: npc
                    .training_fee_base
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                training_fee_multiplier: npc
                    .training_fee_multiplier
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            };
            self.selected_npc = Some(idx);
            self.mode = NpcEditorMode::Edit;
            // Reset persistent autocomplete buffers so the form displays fresh values
            self.reset_autocomplete_buffers = true;
        }
    }

    fn validate_edit_buffer(&mut self) {
        self.validation_errors.clear();

        // Validate ID
        if self.edit_buffer.id.is_empty() {
            self.validation_errors
                .push("ID cannot be empty".to_string());
        } else if !self.is_valid_id(&self.edit_buffer.id) {
            self.validation_errors.push(
                "ID must start with a letter and contain only alphanumeric characters and underscores"
                    .to_string(),
            );
        } else {
            // Check for duplicate IDs
            match self.mode {
                NpcEditorMode::Add => {
                    if self.npcs.iter().any(|n| n.id == self.edit_buffer.id) {
                        self.validation_errors
                            .push(format!("ID '{}' already exists", self.edit_buffer.id));
                    }
                }
                NpcEditorMode::Edit => {
                    if let Some(selected_idx) = self.selected_npc {
                        if self
                            .npcs
                            .iter()
                            .enumerate()
                            .any(|(idx, n)| idx != selected_idx && n.id == self.edit_buffer.id)
                        {
                            self.validation_errors
                                .push(format!("ID '{}' already exists", self.edit_buffer.id));
                        }
                    }
                }
                _ => {}
            }
        }

        // Validate name
        if self.edit_buffer.name.is_empty() {
            self.validation_errors
                .push("Name cannot be empty".to_string());
        }

        // Validate dialogue ID if provided
        if !self.edit_buffer.dialogue_id.is_empty() {
            let dialogue_exists = self
                .available_dialogues
                .iter()
                .any(|d| d.id.to_string() == self.edit_buffer.dialogue_id);
            if !dialogue_exists {
                self.validation_errors.push(format!(
                    "Dialogue ID '{}' does not exist",
                    self.edit_buffer.dialogue_id
                ));
            }
        }

        // Validate quest IDs
        for quest_id_str in &self.edit_buffer.quest_ids {
            let quest_exists = self
                .available_quests
                .iter()
                .any(|q| q.id.to_string() == *quest_id_str);
            if !quest_exists {
                self.validation_errors
                    .push(format!("Quest ID '{}' does not exist", quest_id_str));
            }
        }

        // Validate sprite fields: if a sprite sheet is specified, a valid numeric index must be provided
        if !self.edit_buffer.sprite_sheet.trim().is_empty() {
            if self.edit_buffer.sprite_index.trim().is_empty() {
                self.validation_errors.push(
                    "Sprite index must be provided when a sprite sheet is specified".to_string(),
                );
            } else if self.edit_buffer.sprite_index.trim().parse::<u32>().is_err() {
                self.validation_errors
                    .push("Sprite index must be a valid number".to_string());
            }
        }
    }

    fn is_valid_id(&self, id: &str) -> bool {
        if id.is_empty() {
            return false;
        }
        let first_char = id.chars().next().unwrap();
        if !first_char.is_ascii_alphabetic() && first_char != '_' {
            return false;
        }
        id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    fn merchant_dialogue_validation_for_definition(
        &self,
        npc: &NpcDefinition,
    ) -> MerchantDialogueValidationState {
        let assigned_dialogue = npc.dialogue_id.and_then(|dialogue_id| {
            self.available_dialogues
                .iter()
                .find(|dialogue| dialogue.id == dialogue_id)
        });

        if !npc.is_merchant {
            if assigned_dialogue.is_some_and(DialogueTree::has_sdk_managed_merchant_content) {
                return MerchantDialogueValidationState::StaleMerchantContent;
            }

            return MerchantDialogueValidationState::NotMerchant;
        }

        let Some(dialogue_id) = npc.dialogue_id else {
            return MerchantDialogueValidationState::MissingDialogueId;
        };

        let Some(dialogue) = self
            .available_dialogues
            .iter()
            .find(|dialogue| dialogue.id == dialogue_id)
        else {
            return MerchantDialogueValidationState::MissingDialogueTree;
        };

        if dialogue.contains_open_merchant_for_npc(&npc.id) {
            return MerchantDialogueValidationState::Valid;
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
            MerchantDialogueValidationState::WrongMerchantTarget
        } else {
            MerchantDialogueValidationState::MissingOpenMerchant
        }
    }

    fn merchant_dialogue_status_for_definition(&self, npc: &NpcDefinition) -> (&'static str, bool) {
        match self.merchant_dialogue_validation_for_definition(npc) {
            MerchantDialogueValidationState::NotMerchant => ("Not a merchant", false),
            MerchantDialogueValidationState::MissingDialogueId => ("No dialogue assigned", false),
            MerchantDialogueValidationState::MissingDialogueTree => {
                ("Assigned dialogue missing", false)
            }
            MerchantDialogueValidationState::Valid => {
                let sdk_managed = npc
                    .dialogue_id
                    .and_then(|dialogue_id| {
                        self.available_dialogues
                            .iter()
                            .find(|dialogue| dialogue.id == dialogue_id)
                    })
                    .is_some_and(DialogueTree::has_sdk_managed_merchant_content);

                if sdk_managed {
                    ("SDK-managed merchant branch present", true)
                } else {
                    ("Merchant dialogue valid", false)
                }
            }
            MerchantDialogueValidationState::StaleMerchantContent => {
                ("Non-merchant has stale merchant content", true)
            }
            MerchantDialogueValidationState::WrongMerchantTarget => {
                ("Merchant dialogue targets wrong NPC", false)
            }
            MerchantDialogueValidationState::MissingOpenMerchant => {
                ("Merchant dialogue missing OpenMerchant", false)
            }
        }
    }

    fn merchant_dialogue_repair_action_for_definition(
        &self,
        npc: &NpcDefinition,
    ) -> Option<MerchantDialogueRepairAction> {
        match self.merchant_dialogue_validation_for_definition(npc) {
            MerchantDialogueValidationState::NotMerchant => None,
            MerchantDialogueValidationState::MissingDialogueId => {
                Some(MerchantDialogueRepairAction::CreateDialogue)
            }
            MerchantDialogueValidationState::MissingDialogueTree => {
                Some(MerchantDialogueRepairAction::ReplaceMissingDialogue)
            }
            MerchantDialogueValidationState::Valid => None,
            MerchantDialogueValidationState::StaleMerchantContent => {
                Some(MerchantDialogueRepairAction::RemoveMerchantContent)
            }
            MerchantDialogueValidationState::WrongMerchantTarget => {
                Some(MerchantDialogueRepairAction::RebindMerchantTarget)
            }
            MerchantDialogueValidationState::MissingOpenMerchant => {
                Some(MerchantDialogueRepairAction::RepairDialogue)
            }
        }
    }

    fn merchant_dialogue_status_for_buffer(&self) -> (&'static str, bool) {
        let npc = NpcDefinition {
            id: self.edit_buffer.id.clone(),
            name: self.edit_buffer.name.clone(),
            description: self.edit_buffer.description.clone(),
            portrait_id: self.edit_buffer.portrait_id.clone(),
            dialogue_id: if self.edit_buffer.dialogue_id.trim().is_empty() {
                None
            } else {
                self.edit_buffer.dialogue_id.parse::<DialogueId>().ok()
            },
            creature_id: if self.edit_buffer.creature_id.is_empty() {
                None
            } else {
                self.edit_buffer.creature_id.parse::<CreatureId>().ok()
            },
            sprite: None,
            quest_ids: self
                .edit_buffer
                .quest_ids
                .iter()
                .filter_map(|s| s.parse::<QuestId>().ok())
                .collect(),
            faction: if self.edit_buffer.faction.is_empty() {
                None
            } else {
                Some(self.edit_buffer.faction.clone())
            },
            is_merchant: self.edit_buffer.is_merchant,
            is_innkeeper: self.edit_buffer.is_innkeeper,
            is_priest: false,
            stock_template: if self.edit_buffer.stock_template.is_empty() {
                None
            } else {
                Some(self.edit_buffer.stock_template.clone())
            },
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        };

        self.merchant_dialogue_status_for_definition(&npc)
    }

    pub fn merchant_dialogue_repair_action_for_buffer(
        &self,
    ) -> Option<MerchantDialogueRepairAction> {
        let npc = NpcDefinition {
            id: self.edit_buffer.id.clone(),
            name: self.edit_buffer.name.clone(),
            description: self.edit_buffer.description.clone(),
            portrait_id: self.edit_buffer.portrait_id.clone(),
            dialogue_id: if self.edit_buffer.dialogue_id.trim().is_empty() {
                None
            } else {
                self.edit_buffer.dialogue_id.parse::<DialogueId>().ok()
            },
            creature_id: if self.edit_buffer.creature_id.is_empty() {
                None
            } else {
                self.edit_buffer.creature_id.parse::<CreatureId>().ok()
            },
            sprite: None,
            quest_ids: self
                .edit_buffer
                .quest_ids
                .iter()
                .filter_map(|s| s.parse::<QuestId>().ok())
                .collect(),
            faction: if self.edit_buffer.faction.is_empty() {
                None
            } else {
                Some(self.edit_buffer.faction.clone())
            },
            is_merchant: self.edit_buffer.is_merchant,
            is_innkeeper: self.edit_buffer.is_innkeeper,
            is_priest: false,
            stock_template: if self.edit_buffer.stock_template.is_empty() {
                None
            } else {
                Some(self.edit_buffer.stock_template.clone())
            },
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        };

        self.merchant_dialogue_repair_action_for_definition(&npc)
    }

    fn create_or_repair_merchant_dialogue_for_buffer(&mut self) -> Result<String, String> {
        if !self.edit_buffer.is_merchant {
            // Return an actionable message instead of a silent empty-string no-op.
            // Previously this returned Ok("") which caused the status bar to be
            // cleared with no visible feedback, making the button appear to do nothing.
            return Ok(
                "Enable '🏪 Is Merchant' to create a merchant dialogue for this NPC.".to_string(),
            );
        }

        // If a dialogue_id is set but the referenced tree no longer exists in the
        // in-memory dialogue list (e.g. it was deleted, or the campaign was edited
        // externally in a previous session), clear the stale id so that
        // ensure_merchant_dialogue_for_npc creates a fresh tree instead of
        // returning an error such as "Assigned dialogue X was not found".
        if !self.edit_buffer.dialogue_id.trim().is_empty() {
            if let Ok(stale_id) = self.edit_buffer.dialogue_id.parse::<DialogueId>() {
                if !self
                    .merchant_dialogue_editor
                    .dialogues
                    .iter()
                    .any(|d| d.id == stale_id)
                {
                    self.edit_buffer.dialogue_id.clear();
                }
            }
        }

        let mut npc = NpcDefinition {
            id: self.edit_buffer.id.clone(),
            name: self.edit_buffer.name.clone(),
            description: self.edit_buffer.description.clone(),
            portrait_id: self.edit_buffer.portrait_id.clone(),
            dialogue_id: if self.edit_buffer.dialogue_id.trim().is_empty() {
                None
            } else {
                Some(
                    self.edit_buffer
                        .dialogue_id
                        .parse::<DialogueId>()
                        .map_err(|_| {
                            "Dialogue ID must be numeric before merchant repair".to_string()
                        })?,
                )
            },
            creature_id: if self.edit_buffer.creature_id.is_empty() {
                None
            } else {
                self.edit_buffer.creature_id.parse::<CreatureId>().ok()
            },
            sprite: None,
            quest_ids: self
                .edit_buffer
                .quest_ids
                .iter()
                .filter_map(|s| s.parse::<QuestId>().ok())
                .collect(),
            faction: if self.edit_buffer.faction.is_empty() {
                None
            } else {
                Some(self.edit_buffer.faction.clone())
            },
            is_merchant: self.edit_buffer.is_merchant,
            is_innkeeper: self.edit_buffer.is_innkeeper,
            is_priest: false,
            stock_template: if self.edit_buffer.stock_template.is_empty() {
                None
            } else {
                Some(self.edit_buffer.stock_template.clone())
            },
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        };

        let update = self
            .merchant_dialogue_editor
            .ensure_merchant_dialogue_for_npc(&mut npc)?;

        self.available_dialogues = self.merchant_dialogue_editor.dialogues.clone();
        self.edit_buffer.dialogue_id = npc.dialogue_id.map(|id| id.to_string()).unwrap_or_default();
        self.has_unsaved_changes = true;

        let message = match update {
            MerchantDialogueUpdate::Unchanged => String::new(),
            MerchantDialogueUpdate::AlreadyValid => {
                format!("Merchant dialogue already valid for '{}'", npc.id)
            }
            MerchantDialogueUpdate::CreatedNew { dialogue_id } => {
                format!("Created merchant dialogue {} for '{}'", dialogue_id, npc.id)
            }
            MerchantDialogueUpdate::AugmentedExisting { dialogue_id } => format!(
                "Repaired merchant dialogue {} for '{}'",
                dialogue_id, npc.id
            ),
            MerchantDialogueUpdate::RemovedMerchantContent { dialogue_id } => format!(
                "Removed merchant dialogue content from {} for '{}'",
                dialogue_id, npc.id
            ),
            MerchantDialogueUpdate::NoMerchantContentToRemove { dialogue_id } => format!(
                "No SDK-managed merchant dialogue content to remove from {} for '{}'",
                dialogue_id, npc.id
            ),
        };

        Ok(message)
    }

    pub fn repair_merchant_dialogue_for_buffer(&mut self) -> Result<String, String> {
        match self.merchant_dialogue_repair_action_for_buffer() {
            Some(MerchantDialogueRepairAction::CreateDialogue)
            | Some(MerchantDialogueRepairAction::RepairDialogue)
            | Some(MerchantDialogueRepairAction::ReplaceMissingDialogue) => {
                self.create_or_repair_merchant_dialogue_for_buffer()
            }
            Some(MerchantDialogueRepairAction::RemoveMerchantContent) => {
                self.remove_merchant_dialogue_from_edit_buffer()
            }
            Some(MerchantDialogueRepairAction::RebindMerchantTarget) => {
                let dialogue_id = self.edit_buffer.dialogue_id.clone();
                let correct_npc_id = self.edit_buffer.id.clone();

                // Rebind all OpenMerchant actions in the dialogue to the correct NPC.
                // This handles authored (non-SDK-managed) choices as well as SDK-managed
                // content, so the entire dialogue is consistently retargeted.
                if let Ok(d_id) = dialogue_id.parse::<DialogueId>() {
                    if let Some(dialogue) = self
                        .merchant_dialogue_editor
                        .dialogues
                        .iter_mut()
                        .find(|d| d.id == d_id)
                    {
                        for node in dialogue.nodes.values_mut() {
                            for action in &mut node.actions {
                                if let DialogueAction::OpenMerchant {
                                    npc_id: ref mut target,
                                } = action
                                {
                                    *target = correct_npc_id.clone();
                                }
                            }
                            for choice in &mut node.choices {
                                for action in &mut choice.actions {
                                    if let DialogueAction::OpenMerchant {
                                        npc_id: ref mut target,
                                    } = action
                                    {
                                        *target = correct_npc_id.clone();
                                    }
                                }
                            }
                        }
                    }
                    // Sync rebound dialogues back to available_dialogues so that
                    // subsequent SDK repair steps see the updated content.
                    self.available_dialogues = self.merchant_dialogue_editor.dialogues.clone();
                }

                let removal_message = self.remove_merchant_dialogue_from_edit_buffer()?;
                self.edit_buffer.is_merchant = true;
                self.edit_buffer.dialogue_id = dialogue_id;
                let repair_message = self.create_or_repair_merchant_dialogue_for_buffer()?;
                Ok(format!("{} {}", removal_message, repair_message)
                    .trim()
                    .to_string())
            }
            None => Ok("Merchant dialogue already compliant".to_string()),
        }
    }

    pub fn build_npc_from_edit_buffer(&self, is_merchant: bool) -> Result<NpcDefinition, String> {
        let dialogue_id = if self.edit_buffer.dialogue_id.trim().is_empty() {
            None
        } else {
            Some(
                self.edit_buffer
                    .dialogue_id
                    .parse::<DialogueId>()
                    .map_err(|_| "Dialogue ID must be numeric".to_string())?,
            )
        };

        let creature_id = if self.edit_buffer.creature_id.is_empty() {
            None
        } else {
            self.edit_buffer.creature_id.parse::<CreatureId>().ok()
        };

        let sprite = if self.edit_buffer.sprite_sheet.trim().is_empty() {
            None
        } else {
            match self.edit_buffer.sprite_index.trim().parse::<u32>() {
                Ok(idx) => Some(SpriteReference {
                    sheet_path: self.edit_buffer.sprite_sheet.clone(),
                    sprite_index: idx,
                    animation: None,
                    material_properties: None,
                }),
                Err(_) => None,
            }
        };

        let quest_ids = self
            .edit_buffer
            .quest_ids
            .iter()
            .filter_map(|s| s.parse::<QuestId>().ok())
            .collect();

        let faction = if self.edit_buffer.faction.is_empty() {
            None
        } else {
            Some(self.edit_buffer.faction.clone())
        };

        let stock_template = if self.edit_buffer.stock_template.is_empty() {
            None
        } else {
            Some(self.edit_buffer.stock_template.clone())
        };

        Ok(NpcDefinition {
            id: self.edit_buffer.id.clone(),
            name: self.edit_buffer.name.clone(),
            description: self.edit_buffer.description.clone(),
            portrait_id: self.edit_buffer.portrait_id.clone(),
            dialogue_id,
            creature_id,
            sprite,
            quest_ids,
            faction,
            is_merchant,
            is_innkeeper: self.edit_buffer.is_innkeeper,
            is_priest: false,
            stock_template,
            service_catalog: None::<ServiceCatalog>,
            economy: None::<NpcEconomySettings>,
            is_trainer: self.edit_buffer.is_trainer,
            training_fee_base: self
                .edit_buffer
                .training_fee_base
                .trim()
                .parse::<u32>()
                .ok(),
            training_fee_multiplier: self
                .edit_buffer
                .training_fee_multiplier
                .trim()
                .parse::<f32>()
                .ok(),
        })
    }

    pub fn merchant_dialogue_validation_results(
        &self,
    ) -> Vec<(String, MerchantDialogueValidationState)> {
        self.npcs
            .iter()
            .map(|npc| {
                (
                    npc.id.clone(),
                    self.merchant_dialogue_validation_for_definition(npc),
                )
            })
            .collect()
    }

    pub fn merchant_dialogue_validation_messages(&self) -> Vec<String> {
        self.npcs
            .iter()
            .filter_map(|npc| {
                let state = self.merchant_dialogue_validation_for_definition(npc);
                match state {
                    MerchantDialogueValidationState::NotMerchant
                    | MerchantDialogueValidationState::Valid => None,
                    MerchantDialogueValidationState::MissingDialogueId => Some(format!(
                        "Merchant NPC '{}' has no dialogue assigned",
                        npc.id
                    )),
                    MerchantDialogueValidationState::MissingDialogueTree => Some(format!(
                        "NPC '{}' references missing dialogue {}",
                        npc.id,
                        npc.dialogue_id
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "(none)".to_string())
                    )),
                    MerchantDialogueValidationState::StaleMerchantContent => Some(format!(
                        "Non-merchant NPC '{}' still references dialogue with SDK-managed merchant content",
                        npc.id
                    )),
                    MerchantDialogueValidationState::WrongMerchantTarget => Some(format!(
                        "Merchant NPC '{}' uses dialogue that opens the wrong merchant target",
                        npc.id
                    )),
                    MerchantDialogueValidationState::MissingOpenMerchant => Some(format!(
                        "Merchant NPC '{}' uses dialogue that is missing explicit OpenMerchant",
                        npc.id
                    )),
                }
            })
            .collect()
    }

    fn remove_merchant_dialogue_from_edit_buffer(&mut self) -> Result<String, String> {
        let npc = self.build_npc_from_edit_buffer(false)?;

        let update = self
            .merchant_dialogue_editor
            .remove_merchant_dialogue_for_npc(&npc)?;

        self.available_dialogues = self.merchant_dialogue_editor.dialogues.clone();
        self.has_unsaved_changes = true;

        let message = match update {
            MerchantDialogueUpdate::Unchanged => {
                "Merchant cleanup not required for current NPC state".to_string()
            }
            MerchantDialogueUpdate::AlreadyValid => {
                format!("Merchant dialogue already valid for '{}'", npc.id)
            }
            MerchantDialogueUpdate::CreatedNew { dialogue_id } => {
                format!("Created merchant dialogue {} for '{}'", dialogue_id, npc.id)
            }
            MerchantDialogueUpdate::AugmentedExisting { dialogue_id } => format!(
                "Repaired merchant dialogue {} for '{}'",
                dialogue_id, npc.id
            ),
            MerchantDialogueUpdate::RemovedMerchantContent { dialogue_id } => format!(
                "Removed SDK-managed merchant content from dialogue {} for '{}'; non-merchant dialogue content was preserved",
                dialogue_id, npc.id
            ),
            MerchantDialogueUpdate::NoMerchantContentToRemove { dialogue_id } => format!(
                "Dialogue {} for '{}' had no SDK-managed merchant content to remove",
                dialogue_id, npc.id
            ),
        };

        Ok(message)
    }

    fn auto_apply_merchant_dialogue_to_edit_buffer(&mut self) -> Result<String, String> {
        let (status_label, _) = self.merchant_dialogue_status_for_buffer();
        if !self.edit_buffer.is_merchant {
            return Ok(String::new());
        }

        if matches!(
            status_label,
            "No dialogue assigned"
                | "Merchant dialogue missing OpenMerchant"
                // "Assigned dialogue missing" means dialogue_id is set but the tree
                // was deleted.  create_or_repair now clears the stale id first, so
                // calling it here produces a fresh tree rather than an error.
                | "Assigned dialogue missing"
        ) {
            return self.create_or_repair_merchant_dialogue_for_buffer();
        }

        Ok(format!(
            "Merchant dialogue already valid for '{}'",
            self.edit_buffer.id
        ))
    }

    /// Returns the trainer dialogue validation state for an NPC definition.
    ///
    /// Used by the list view and edit view to surface trainer-dialogue health
    /// without destructively modifying any data.
    fn trainer_dialogue_validation_for_definition(
        &self,
        npc: &NpcDefinition,
    ) -> TrainerDialogueValidationState {
        let assigned_dialogue = npc.dialogue_id.and_then(|dialogue_id| {
            self.available_dialogues
                .iter()
                .find(|dialogue| dialogue.id == dialogue_id)
        });

        if !npc.is_trainer {
            if assigned_dialogue.is_some_and(DialogueTree::has_sdk_managed_trainer_content) {
                return TrainerDialogueValidationState::StaleTrainerContent;
            }
            return TrainerDialogueValidationState::NotTrainer;
        }

        let Some(dialogue_id) = npc.dialogue_id else {
            return TrainerDialogueValidationState::Missing;
        };

        let Some(dialogue) = self
            .available_dialogues
            .iter()
            .find(|dialogue| dialogue.id == dialogue_id)
        else {
            return TrainerDialogueValidationState::AssignedDialogueMissing;
        };

        if dialogue.contains_open_training_for_npc(&npc.id) {
            TrainerDialogueValidationState::Valid
        } else {
            // Has an assigned dialogue but it lacks an OpenTraining action.
            TrainerDialogueValidationState::Missing
        }
    }

    /// Returns a human-readable trainer dialogue status string for the current
    /// edit buffer, built into a temporary `NpcDefinition` for validation.
    fn trainer_dialogue_status_for_buffer(&self) -> &'static str {
        let npc = NpcDefinition {
            id: self.edit_buffer.id.clone(),
            name: self.edit_buffer.name.clone(),
            description: self.edit_buffer.description.clone(),
            portrait_id: self.edit_buffer.portrait_id.clone(),
            dialogue_id: if self.edit_buffer.dialogue_id.trim().is_empty() {
                None
            } else {
                self.edit_buffer.dialogue_id.parse::<DialogueId>().ok()
            },
            creature_id: if self.edit_buffer.creature_id.is_empty() {
                None
            } else {
                self.edit_buffer.creature_id.parse::<CreatureId>().ok()
            },
            sprite: None,
            quest_ids: self
                .edit_buffer
                .quest_ids
                .iter()
                .filter_map(|s| s.parse::<QuestId>().ok())
                .collect(),
            faction: if self.edit_buffer.faction.is_empty() {
                None
            } else {
                Some(self.edit_buffer.faction.clone())
            },
            is_merchant: self.edit_buffer.is_merchant,
            is_innkeeper: self.edit_buffer.is_innkeeper,
            is_priest: false,
            stock_template: if self.edit_buffer.stock_template.is_empty() {
                None
            } else {
                Some(self.edit_buffer.stock_template.clone())
            },
            service_catalog: None,
            economy: None,
            is_trainer: self.edit_buffer.is_trainer,
            training_fee_base: None,
            training_fee_multiplier: None,
        };

        match self.trainer_dialogue_validation_for_definition(&npc) {
            TrainerDialogueValidationState::NotTrainer => "Not a trainer",
            TrainerDialogueValidationState::Valid => "Trainer dialogue valid",
            TrainerDialogueValidationState::Missing => "No dialogue assigned",
            TrainerDialogueValidationState::AssignedDialogueMissing => {
                "Assigned trainer dialogue missing"
            }
            TrainerDialogueValidationState::StaleTrainerContent => {
                "Non-trainer has stale trainer content"
            }
        }
    }

    /// Creates or repairs the trainer dialogue for the NPC in the edit buffer.
    ///
    /// When `is_trainer` is `false`, returns an actionable guidance message
    /// instead of a silent no-op.
    ///
    /// When a stale `dialogue_id` is present (tree deleted externally), clears
    /// it first so a fresh tree is created rather than returning an error.
    fn create_or_repair_trainer_dialogue_for_buffer(&mut self) -> Result<String, String> {
        if !self.edit_buffer.is_trainer {
            return Ok(
                "Enable '🎓 Is Trainer' to create a trainer dialogue for this NPC.".to_string(),
            );
        }

        // Clear a stale dialogue_id (tree no longer exists) so that
        // ensure_trainer_dialogue_for_npc creates a fresh tree instead.
        if !self.edit_buffer.dialogue_id.trim().is_empty() {
            if let Ok(stale_id) = self.edit_buffer.dialogue_id.parse::<DialogueId>() {
                if !self
                    .merchant_dialogue_editor
                    .dialogues
                    .iter()
                    .any(|d| d.id == stale_id)
                {
                    self.edit_buffer.dialogue_id.clear();
                }
            }
        }

        let mut npc = NpcDefinition {
            id: self.edit_buffer.id.clone(),
            name: self.edit_buffer.name.clone(),
            description: self.edit_buffer.description.clone(),
            portrait_id: self.edit_buffer.portrait_id.clone(),
            dialogue_id: if self.edit_buffer.dialogue_id.trim().is_empty() {
                None
            } else {
                Some(
                    self.edit_buffer
                        .dialogue_id
                        .parse::<DialogueId>()
                        .map_err(|_| {
                            "Dialogue ID must be numeric before trainer repair".to_string()
                        })?,
                )
            },
            creature_id: if self.edit_buffer.creature_id.is_empty() {
                None
            } else {
                self.edit_buffer.creature_id.parse::<CreatureId>().ok()
            },
            sprite: None,
            quest_ids: self
                .edit_buffer
                .quest_ids
                .iter()
                .filter_map(|s| s.parse::<QuestId>().ok())
                .collect(),
            faction: if self.edit_buffer.faction.is_empty() {
                None
            } else {
                Some(self.edit_buffer.faction.clone())
            },
            is_merchant: self.edit_buffer.is_merchant,
            is_innkeeper: self.edit_buffer.is_innkeeper,
            is_priest: false,
            stock_template: if self.edit_buffer.stock_template.is_empty() {
                None
            } else {
                Some(self.edit_buffer.stock_template.clone())
            },
            service_catalog: None,
            economy: None,
            is_trainer: true,
            training_fee_base: None,
            training_fee_multiplier: None,
        };

        use crate::dialogue_editor::MerchantDialogueUpdate;
        let update = self
            .merchant_dialogue_editor
            .ensure_trainer_dialogue_for_npc(&mut npc)?;

        self.available_dialogues = self.merchant_dialogue_editor.dialogues.clone();
        self.edit_buffer.dialogue_id = npc.dialogue_id.map(|id| id.to_string()).unwrap_or_default();
        self.has_unsaved_changes = true;

        let message = match update {
            MerchantDialogueUpdate::Unchanged => String::new(),
            MerchantDialogueUpdate::AlreadyValid => {
                format!("Trainer dialogue already valid for '{}'", npc.id)
            }
            MerchantDialogueUpdate::CreatedNew { dialogue_id } => {
                format!("Created trainer dialogue {} for '{}'", dialogue_id, npc.id)
            }
            MerchantDialogueUpdate::AugmentedExisting { dialogue_id } => {
                format!("Repaired trainer dialogue {} for '{}'", dialogue_id, npc.id)
            }
            MerchantDialogueUpdate::RemovedMerchantContent { dialogue_id } => format!(
                "Removed trainer dialogue content from {} for '{}'",
                dialogue_id, npc.id
            ),
            MerchantDialogueUpdate::NoMerchantContentToRemove { dialogue_id } => format!(
                "No SDK-managed trainer dialogue content to remove from {} for '{}'",
                dialogue_id, npc.id
            ),
        };

        Ok(message)
    }

    /// Removes SDK-managed trainer content from the edit buffer's assigned dialogue.
    ///
    /// Non-destructive: only SDK-managed trainer nodes/choices are removed;
    /// unrelated authored content is preserved.
    fn remove_trainer_dialogue_from_edit_buffer(&mut self) -> Result<String, String> {
        let npc = NpcDefinition {
            id: self.edit_buffer.id.clone(),
            name: self.edit_buffer.name.clone(),
            description: self.edit_buffer.description.clone(),
            portrait_id: self.edit_buffer.portrait_id.clone(),
            dialogue_id: if self.edit_buffer.dialogue_id.trim().is_empty() {
                None
            } else {
                self.edit_buffer.dialogue_id.parse::<DialogueId>().ok()
            },
            creature_id: if self.edit_buffer.creature_id.is_empty() {
                None
            } else {
                self.edit_buffer.creature_id.parse::<CreatureId>().ok()
            },
            sprite: None,
            quest_ids: self
                .edit_buffer
                .quest_ids
                .iter()
                .filter_map(|s| s.parse::<QuestId>().ok())
                .collect(),
            faction: if self.edit_buffer.faction.is_empty() {
                None
            } else {
                Some(self.edit_buffer.faction.clone())
            },
            is_merchant: self.edit_buffer.is_merchant,
            is_innkeeper: self.edit_buffer.is_innkeeper,
            is_priest: false,
            stock_template: if self.edit_buffer.stock_template.is_empty() {
                None
            } else {
                Some(self.edit_buffer.stock_template.clone())
            },
            service_catalog: None,
            economy: None,
            // Intentionally false so remove_trainer_dialogue_for_npc proceeds.
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        };

        use crate::dialogue_editor::MerchantDialogueUpdate;
        let update = self
            .merchant_dialogue_editor
            .remove_trainer_dialogue_for_npc(&npc)?;

        self.available_dialogues = self.merchant_dialogue_editor.dialogues.clone();
        self.has_unsaved_changes = true;

        let message = match update {
            MerchantDialogueUpdate::Unchanged => {
                "Trainer cleanup not required for current NPC state".to_string()
            }
            MerchantDialogueUpdate::AlreadyValid => {
                format!("Trainer dialogue already valid for '{}'", npc.id)
            }
            MerchantDialogueUpdate::CreatedNew { dialogue_id } => {
                format!("Created trainer dialogue {} for '{}'", dialogue_id, npc.id)
            }
            MerchantDialogueUpdate::AugmentedExisting { dialogue_id } => format!(
                "Repaired trainer dialogue {} for '{}'",
                dialogue_id, npc.id
            ),
            MerchantDialogueUpdate::RemovedMerchantContent { dialogue_id } => format!(
                "Removed SDK-managed trainer content from dialogue {} for '{}'; non-trainer dialogue content was preserved",
                dialogue_id, npc.id
            ),
            MerchantDialogueUpdate::NoMerchantContentToRemove { dialogue_id } => format!(
                "Dialogue {} for '{}' had no SDK-managed trainer content to remove",
                dialogue_id, npc.id
            ),
        };

        Ok(message)
    }

    /// Auto-applies a trainer dialogue to the edit buffer when the buffer's
    /// current dialogue state is missing or incomplete.
    ///
    /// Called automatically when `is_trainer` is toggled on.
    fn auto_apply_trainer_dialogue_to_edit_buffer(&mut self) -> Result<String, String> {
        let status = self.trainer_dialogue_status_for_buffer();
        if !self.edit_buffer.is_trainer {
            return Ok(String::new());
        }

        if matches!(
            status,
            "No dialogue assigned" | "Assigned trainer dialogue missing"
        ) {
            return self.create_or_repair_trainer_dialogue_for_buffer();
        }

        Ok(format!(
            "Trainer dialogue already valid for '{}'",
            self.edit_buffer.id
        ))
    }

    fn save_npc(&mut self) -> bool {
        self.validate_edit_buffer();
        if !self.validation_errors.is_empty() {
            return false;
        }

        let npc_def = NpcDefinition {
            id: self.edit_buffer.id.clone(),
            name: self.edit_buffer.name.clone(),
            description: self.edit_buffer.description.clone(),
            portrait_id: self.edit_buffer.portrait_id.clone(),
            dialogue_id: if self.edit_buffer.dialogue_id.is_empty() {
                None
            } else {
                self.edit_buffer.dialogue_id.parse::<DialogueId>().ok()
            },
            creature_id: if self.edit_buffer.creature_id.is_empty() {
                None
            } else {
                self.edit_buffer.creature_id.parse::<CreatureId>().ok()
            },
            sprite: if self.edit_buffer.sprite_sheet.trim().is_empty() {
                None
            } else {
                match self.edit_buffer.sprite_index.trim().parse::<u32>() {
                    Ok(idx) => Some(SpriteReference {
                        sheet_path: self.edit_buffer.sprite_sheet.clone(),
                        sprite_index: idx,
                        animation: None,
                        material_properties: None,
                    }),
                    Err(_) => None,
                }
            },
            quest_ids: self
                .edit_buffer
                .quest_ids
                .iter()
                .filter_map(|s| s.parse::<QuestId>().ok())
                .collect(),
            faction: if self.edit_buffer.faction.is_empty() {
                None
            } else {
                Some(self.edit_buffer.faction.clone())
            },
            is_merchant: self.edit_buffer.is_merchant,
            is_innkeeper: self.edit_buffer.is_innkeeper,
            stock_template: if self.edit_buffer.stock_template.is_empty() {
                None
            } else {
                Some(self.edit_buffer.stock_template.clone())
            },
            is_priest: false,
            service_catalog: None,
            economy: None,
            is_trainer: self.edit_buffer.is_trainer,
            training_fee_base: self
                .edit_buffer
                .training_fee_base
                .trim()
                .parse::<u32>()
                .ok(),
            training_fee_multiplier: self
                .edit_buffer
                .training_fee_multiplier
                .trim()
                .parse::<f32>()
                .ok(),
        };

        // Perform the in-memory save and remember the result
        let saved = match self.mode {
            NpcEditorMode::Add => {
                self.npcs.push(npc_def);
                true
            }
            NpcEditorMode::Edit => {
                if let Some(index) = self.selected_npc {
                    if index < self.npcs.len() {
                        self.npcs[index] = npc_def;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        };

        if saved {
            self.merchant_dialogue_editor
                .load_dialogues(self.available_dialogues.clone());
        }

        // If saved, attempt immediate persistence when we have campaign dir & filename
        if saved {
            // Mark unsaved until we can persist successfully
            self.has_unsaved_changes = true;

            if let (Some(dir), Some(filename)) = (&self.last_campaign_dir, &self.last_npcs_file) {
                let path = dir.join(filename);
                match self.save_to_file(&path) {
                    Ok(_) => {
                        // Persisted successfully; clear unsaved flag
                        self.has_unsaved_changes = false;
                    }
                    Err(_persist_err) => {
                        // Persistence failure: NPCs are saved in memory; has_unsaved_changes
                        // stays true to indicate a pending disk write.
                    }
                }
            }
        }

        saved
    }

    /// Loads NPC definitions from a RON file on disk, replacing the current list.
    ///
    /// On success `self.npcs` is replaced with the loaded data, the current
    /// selection is cleared and the editor returns to List mode.
    ///
    /// # Arguments
    ///
    /// * `path` - Full path to the `.ron` file containing `Vec<NpcDefinition>`.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err(NpcEditorError)` on failure.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::npc_editor::NpcEditorState;
    /// use std::path::Path;
    ///
    /// let mut state = NpcEditorState::new();
    /// // state.load_from_file(Path::new("data/npcs.ron")).unwrap();
    /// ```
    pub fn load_from_file(&mut self, path: &std::path::Path) -> Result<(), NpcEditorError> {
        let contents = std::fs::read_to_string(path)?;
        let loaded: Vec<NpcDefinition> =
            ron::from_str(&contents).map_err(|e| NpcEditorError::Parse(e.to_string()))?;
        self.npcs = loaded;
        self.selected_npc = None;
        self.mode = NpcEditorMode::List;
        self.has_unsaved_changes = false;
        Ok(())
    }

    fn save_to_file(&self, path: &std::path::Path) -> Result<(), NpcEditorError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = ron::ser::to_string_pretty(&self.npcs, ron::ser::PrettyConfig::default())
            .map_err(|e| NpcEditorError::Serialization(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    fn export_all_npcs(&mut self) -> bool {
        if let Ok(ron_str) =
            ron::ser::to_string_pretty(&self.npcs, ron::ser::PrettyConfig::default())
        {
            self.import_buffer = ron_str;
            self.show_import_dialog = true;
            false // Export doesn't require save
        } else {
            false
        }
    }

    fn show_import_dialog_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_import_dialog;

        egui::Window::new("Import/Export NPC")
            .open(&mut open)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.heading("NPC RON Data");
                ui.separator();

                ui.label("Paste RON data to import, or copy exported data:");
                ui.add(
                    egui::TextEdit::multiline(&mut self.import_buffer)
                        .desired_rows(15)
                        .code_editor()
                        .id_salt("npc_import_buffer"),
                );

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("📥 Import").clicked() {
                        match ron::from_str::<NpcDefinition>(&self.import_buffer) {
                            Ok(mut npc) => {
                                let next_id = self.next_npc_id();
                                npc.id = next_id;
                                self.npcs.push(npc);
                                self.show_import_dialog = false;
                            }
                            Err(e) => {
                                // Show error in validation
                                self.validation_errors.push(format!("Import failed: {}", e));
                            }
                        }
                    }

                    if ui.button("📋 Copy to Clipboard").clicked() {
                        ui.ctx().copy_text(self.import_buffer.clone());
                    }

                    if ui.button("❌ Close").clicked() {
                        self.show_import_dialog = false;
                    }
                });
            });

        self.show_import_dialog = open;
    }

    /// Generate next available NPC ID
    pub fn next_npc_id(&self) -> String {
        let existing_ids: HashSet<String> = self.npcs.iter().map(|n| n.id.clone()).collect();

        let mut counter = 1;
        loop {
            let candidate = format!("npc_{}", counter);
            if !existing_ids.contains(&candidate) {
                return candidate;
            }
            counter += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use antares::domain::dialogue::{DialogueChoice, DialogueNode};

    #[test]
    fn test_npc_editor_state_new() {
        let state = NpcEditorState::new();
        assert_eq!(state.npcs.len(), 0);
        assert_eq!(state.mode, NpcEditorMode::List);
    }

    #[test]
    fn test_start_add_npc() {
        let mut state = NpcEditorState::new();
        state.start_add_npc();
        assert_eq!(state.mode, NpcEditorMode::Add);
        assert!(!state.edit_buffer.id.is_empty());
    }

    #[test]
    fn test_validate_edit_buffer_empty_id() {
        let mut state = NpcEditorState::new();
        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = String::new();
        state.edit_buffer.name = "Test".to_string();
        state.validate_edit_buffer();
        assert!(!state.validation_errors.is_empty());
        assert!(state.validation_errors[0].contains("ID cannot be empty"));
    }

    #[test]
    fn test_validate_edit_buffer_invalid_id() {
        let mut state = NpcEditorState::new();
        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "123invalid".to_string();
        state.validate_edit_buffer();
        assert!(state
            .validation_errors
            .iter()
            .any(|e| e.contains("must start")));
    }

    #[test]
    fn test_validate_edit_buffer_valid() {
        let mut state = NpcEditorState::new();
        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "valid_id".to_string();
        state.edit_buffer.name = "Test NPC".to_string();
        state.validate_edit_buffer();
        assert!(state.validation_errors.is_empty());
    }

    #[test]
    fn test_save_npc_add_mode() {
        let mut state = NpcEditorState::new();
        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "test_npc".to_string();
        state.edit_buffer.name = "Test NPC".to_string();
        state.edit_buffer.description = "Test description".to_string();

        let result = state.save_npc();
        assert!(result);
        assert_eq!(state.npcs.len(), 1);
        assert_eq!(state.npcs[0].id, "test_npc");
        assert_eq!(state.npcs[0].name, "Test NPC");
    }

    #[test]
    fn test_save_npc_with_sprite_in_add_mode() {
        let mut state = NpcEditorState::new();
        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "test_npc_sprite".to_string();
        state.edit_buffer.name = "Test NPC Sprite".to_string();
        state.edit_buffer.description = "Test sprite desc".to_string();
        state.edit_buffer.sprite_sheet = "assets/sprites/actors/wizard.png".to_string();
        state.edit_buffer.sprite_index = "12".to_string();

        let result = state.save_npc();
        assert!(result);
        assert_eq!(state.npcs.len(), 1);
        let saved = &state.npcs[0];
        assert!(saved.sprite.is_some());
        let sprite = saved.sprite.as_ref().unwrap();
        assert_eq!(sprite.sheet_path, "assets/sprites/actors/wizard.png");
        assert_eq!(sprite.sprite_index, 12);
    }

    #[test]
    fn test_start_edit_npc_populates_sprite_fields() {
        let mut state = NpcEditorState::new();

        let sprite = antares::domain::world::SpriteReference {
            sheet_path: "assets/sprites/actors/test.png".to_string(),
            sprite_index: 5,
            animation: None,
            material_properties: None,
        };

        state.npcs.push(NpcDefinition {
            id: "npc1".to_string(),
            name: "WithSprite".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            sprite: Some(sprite.clone()),
            dialogue_id: None,
            creature_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        });

        state.start_edit_npc(0);
        assert_eq!(state.mode, NpcEditorMode::Edit);
        assert_eq!(
            state.edit_buffer.sprite_sheet,
            "assets/sprites/actors/test.png"
        );
        assert_eq!(state.edit_buffer.sprite_index, "5");
    }

    #[test]
    fn test_save_npc_generates_merchant_dialogue_when_missing() {
        let mut state = NpcEditorState::new();
        state.available_dialogues = Vec::new();
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "merchant_alma".to_string();
        state.edit_buffer.name = "Alma".to_string();
        state.edit_buffer.description = "Merchant".to_string();
        state.edit_buffer.portrait_id = "alma".to_string();
        state.edit_buffer.is_merchant = true;

        let message = state
            .auto_apply_merchant_dialogue_to_edit_buffer()
            .expect("merchant dialogue generation should succeed");

        assert!(message.contains("Created merchant dialogue"));
        assert_eq!(state.edit_buffer.dialogue_id, "1");
        assert_eq!(state.available_dialogues.len(), 1);
        assert_eq!(state.merchant_dialogue_editor.dialogues.len(), 1);

        let dialogue = &state.available_dialogues[0];
        assert_eq!(dialogue.id, 1);
        assert!(dialogue.contains_open_merchant_for_npc("merchant_alma"));
        assert!(dialogue.has_sdk_managed_merchant_content());

        let saved = state.save_npc();
        assert!(saved);
        assert_eq!(state.npcs.len(), 1);
        assert_eq!(state.npcs[0].dialogue_id, Some(1));
        assert_eq!(state.npcs[0].id, "merchant_alma");
    }

    #[test]
    fn test_save_npc_augments_existing_dialogue_for_merchant() {
        let mut state = NpcEditorState::new();
        let mut dialogue = DialogueTree::new(7, "Existing Dialogue", 1);
        dialogue.add_node(DialogueNode::new(1, "Welcome traveler."));
        state.available_dialogues = vec![dialogue.clone()];
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "merchant_borin".to_string();
        state.edit_buffer.name = "Borin".to_string();
        state.edit_buffer.description = "Merchant".to_string();
        state.edit_buffer.portrait_id = "borin".to_string();
        state.edit_buffer.is_merchant = true;
        state.edit_buffer.dialogue_id = "7".to_string();

        let message = state
            .auto_apply_merchant_dialogue_to_edit_buffer()
            .expect("merchant dialogue repair should succeed");

        assert!(message.contains("Repaired merchant dialogue 7"));
        assert_eq!(state.available_dialogues.len(), 1);

        let updated = state
            .available_dialogues
            .iter()
            .find(|dialogue| dialogue.id == 7)
            .expect("updated dialogue should exist");
        assert!(updated.contains_open_merchant_for_npc("merchant_borin"));

        let root = updated
            .get_node(updated.root_node)
            .expect("root node should exist");
        let merchant_choices = root
            .choices
            .iter()
            .filter(|choice| choice.is_sdk_managed_merchant_choice())
            .count();
        assert_eq!(merchant_choices, 1);

        let saved = state.save_npc();
        assert!(saved);
        assert_eq!(state.npcs[0].dialogue_id, Some(7));
    }

    #[test]
    fn test_save_npc_merchant_dialogue_generation_is_idempotent() {
        let mut state = NpcEditorState::new();
        let mut dialogue = DialogueTree::new(9, "Custom Merchant Dialogue", 1);
        let mut root = DialogueNode::new(1, "Need something?");
        root.add_choice(DialogueChoice::new("Maybe later.", None));
        dialogue.add_node(root);

        state.available_dialogues = vec![dialogue];
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "merchant_cora".to_string();
        state.edit_buffer.name = "Cora".to_string();
        state.edit_buffer.description = "Merchant".to_string();
        state.edit_buffer.portrait_id = "cora".to_string();
        state.edit_buffer.is_merchant = true;
        state.edit_buffer.dialogue_id = "9".to_string();

        let first_message = state
            .auto_apply_merchant_dialogue_to_edit_buffer()
            .expect("first merchant repair should succeed");
        let second_message = state
            .auto_apply_merchant_dialogue_to_edit_buffer()
            .expect("second merchant repair should be idempotent");

        assert!(first_message.contains("Repaired merchant dialogue 9"));
        assert_eq!(
            second_message,
            "Merchant dialogue already valid for 'merchant_cora'"
        );

        let updated = state
            .available_dialogues
            .iter()
            .find(|dialogue| dialogue.id == 9)
            .expect("dialogue should exist");
        let root = updated
            .get_node(updated.root_node)
            .expect("root node should exist");
        let merchant_choices = root
            .choices
            .iter()
            .filter(|choice| choice.is_sdk_managed_merchant_choice())
            .count();
        assert_eq!(merchant_choices, 1);

        let merchant_nodes = updated
            .nodes
            .values()
            .filter(|node| node.has_sdk_managed_merchant_content())
            .count();
        // After augmentation there are 2 nodes with SDK merchant content:
        // (1) the root node, which received a new SDK-managed merchant choice, and
        // (2) the dedicated merchant node that was inserted.
        assert_eq!(merchant_nodes, 2);
    }

    #[test]
    fn test_merchant_dialogue_status_for_buffer_reports_missing_open_merchant() {
        let mut state = NpcEditorState::new();
        let mut dialogue = DialogueTree::new(12, "Non Merchant", 1);
        dialogue.add_node(DialogueNode::new(1, "Hello."));
        state.available_dialogues = vec![dialogue];
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.edit_buffer.id = "merchant_dain".to_string();
        state.edit_buffer.name = "Dain".to_string();
        state.edit_buffer.portrait_id = "dain".to_string();
        state.edit_buffer.is_merchant = true;
        state.edit_buffer.dialogue_id = "12".to_string();

        let (status, sdk_managed) = state.merchant_dialogue_status_for_buffer();
        assert_eq!(status, "Merchant dialogue missing OpenMerchant");
        assert!(!sdk_managed);
    }

    #[test]
    fn test_remove_merchant_dialogue_from_augmented_custom_dialogue_preserves_authored_content() {
        let mut state = NpcEditorState::new();
        let mut dialogue = DialogueTree::new(20, "Custom Dialogue", 1);
        let mut root = DialogueNode::new(1, "Welcome traveler.");
        root.add_choice(DialogueChoice::new("Tell me more.", Some(3)));
        dialogue.add_node(root);
        dialogue.add_node(DialogueNode::new(3, "Here is more information."));
        assert!(dialogue.ensure_standard_merchant_branch("merchant_tom", "Tom"));

        state.available_dialogues = vec![dialogue];
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.edit_buffer.id = "merchant_tom".to_string();
        state.edit_buffer.name = "Tom".to_string();
        state.edit_buffer.portrait_id = "tom".to_string();
        state.edit_buffer.dialogue_id = "20".to_string();
        state.edit_buffer.is_merchant = false;

        let message = state
            .remove_merchant_dialogue_from_edit_buffer()
            .expect("merchant dialogue removal should succeed");

        assert!(message.contains("Removed SDK-managed merchant content"));
        let updated = state
            .available_dialogues
            .iter()
            .find(|dialogue| dialogue.id == 20)
            .expect("updated dialogue should exist");
        assert!(!updated.contains_open_merchant_for_npc("merchant_tom"));

        let root = updated
            .get_node(updated.root_node)
            .expect("root node should exist");
        assert_eq!(root.choices.len(), 1);
        assert_eq!(root.choices[0].text, "Tell me more.");
        assert!(updated.get_node(3).is_some());
    }

    #[test]
    fn test_remove_merchant_dialogue_from_generated_template_leaves_dialogue_asset_intact() {
        let mut state = NpcEditorState::new();
        let dialogue = DialogueTree::standard_merchant_template(21, "merchant_ivy", "Ivy");
        state.available_dialogues = vec![dialogue];
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.edit_buffer.id = "merchant_ivy".to_string();
        state.edit_buffer.name = "Ivy".to_string();
        state.edit_buffer.portrait_id = "ivy".to_string();
        state.edit_buffer.dialogue_id = "21".to_string();
        state.edit_buffer.is_merchant = false;

        let message = state
            .remove_merchant_dialogue_from_edit_buffer()
            .expect("merchant template removal should succeed");

        assert!(message.contains("Removed SDK-managed merchant content"));
        let updated = state
            .available_dialogues
            .iter()
            .find(|dialogue| dialogue.id == 21)
            .expect("updated dialogue should exist");
        assert_eq!(updated.id, 21);
        assert_eq!(updated.root_node, 1);
        assert!(updated.get_node(1).is_some());
        assert!(updated.get_node(2).is_none());
        assert!(!updated.contains_open_merchant_for_npc("merchant_ivy"));
    }

    #[test]
    fn test_remove_merchant_dialogue_from_edit_buffer_is_idempotent() {
        let mut state = NpcEditorState::new();
        let dialogue = DialogueTree::standard_merchant_template(22, "merchant_sela", "Sela");
        state.available_dialogues = vec![dialogue];
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.edit_buffer.id = "merchant_sela".to_string();
        state.edit_buffer.name = "Sela".to_string();
        state.edit_buffer.portrait_id = "sela".to_string();
        state.edit_buffer.dialogue_id = "22".to_string();
        state.edit_buffer.is_merchant = false;

        let first_message = state
            .remove_merchant_dialogue_from_edit_buffer()
            .expect("first removal should succeed");
        let second_message = state
            .remove_merchant_dialogue_from_edit_buffer()
            .expect("second removal should be idempotent");

        assert!(first_message.contains("Removed SDK-managed merchant content"));
        assert!(second_message.contains("had no SDK-managed merchant content to remove"));

        let updated = state
            .available_dialogues
            .iter()
            .find(|dialogue| dialogue.id == 22)
            .expect("updated dialogue should exist");
        assert!(!updated.contains_open_merchant_for_npc("merchant_sela"));
    }

    #[test]
    fn test_remove_merchant_dialogue_from_edit_buffer_is_noop_without_merchant_content() {
        let mut state = NpcEditorState::new();
        let mut dialogue = DialogueTree::new(23, "Ordinary Dialogue", 1);
        let mut root = DialogueNode::new(1, "Good day.");
        root.add_choice(DialogueChoice::new("Farewell.", None));
        dialogue.add_node(root);

        state.available_dialogues = vec![dialogue];
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.edit_buffer.id = "npc_ordinary".to_string();
        state.edit_buffer.name = "Ordinary NPC".to_string();
        state.edit_buffer.portrait_id = "ordinary".to_string();
        state.edit_buffer.dialogue_id = "23".to_string();
        state.edit_buffer.is_merchant = false;

        let message = state
            .remove_merchant_dialogue_from_edit_buffer()
            .expect("no-op removal should succeed");

        assert!(message.contains("had no SDK-managed merchant content to remove"));
        let updated = state
            .available_dialogues
            .iter()
            .find(|dialogue| dialogue.id == 23)
            .expect("updated dialogue should exist");
        assert_eq!(updated.get_node(1).expect("root node").choices.len(), 1);
        assert_eq!(
            updated.get_node(1).expect("root node").choices[0].text,
            "Farewell."
        );
    }

    #[test]
    fn test_generated_merchant_dialogue_roundtrip_remains_runtime_valid() {
        let mut state = NpcEditorState::new();
        state.available_dialogues = Vec::new();
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "merchant_roundtrip".to_string();
        state.edit_buffer.name = "Roundtrip Merchant".to_string();
        state.edit_buffer.description = "Generated merchant dialogue".to_string();
        state.edit_buffer.portrait_id = "merchant_roundtrip".to_string();
        state.edit_buffer.is_merchant = true;

        let message = state
            .auto_apply_merchant_dialogue_to_edit_buffer()
            .expect("merchant dialogue generation should succeed");
        assert!(message.contains("Created merchant dialogue"));

        let npc = state
            .build_npc_from_edit_buffer(true)
            .expect("edit buffer should produce a merchant npc");
        let dialogue = state
            .available_dialogues
            .iter()
            .find(|dialogue| dialogue.id == npc.dialogue_id.expect("generated dialogue id"))
            .expect("generated dialogue should exist");

        assert!(
            dialogue.contains_open_merchant_for_npc(&npc.id),
            "generated merchant dialogue must contain explicit OpenMerchant for runtime compatibility"
        );
        assert!(
            dialogue.validate().is_ok(),
            "generated merchant dialogue should remain structurally valid"
        );

        // Commit the built NPC into the editor's list so it is included in save_to_file.
        state.npcs.push(npc.clone());

        let temp_dir = tempfile::tempdir().expect("tempdir");
        let dialogues_path = temp_dir.path().join("dialogues.ron");
        let npcs_path = temp_dir.path().join("npcs.ron");

        state
            .merchant_dialogue_editor
            .save_to_file(&dialogues_path)
            .expect("save generated dialogues");
        state.save_to_file(&npcs_path).expect("save npcs");

        let mut reloaded_dialogues = DialogueEditorState::new();
        reloaded_dialogues
            .load_from_file(&dialogues_path)
            .expect("reload generated dialogues");

        let reloaded_npcs_contents =
            std::fs::read_to_string(&npcs_path).expect("read saved npcs file");
        let reloaded_npcs: Vec<NpcDefinition> =
            ron::from_str(&reloaded_npcs_contents).expect("parse saved npcs");

        let reloaded_npc = reloaded_npcs
            .iter()
            .find(|npc| npc.id == "merchant_roundtrip")
            .expect("reloaded merchant npc should exist");
        let reloaded_dialogue = reloaded_dialogues
            .dialogues
            .iter()
            .find(|dialogue| dialogue.id == reloaded_npc.dialogue_id.expect("dialogue id"))
            .expect("reloaded merchant dialogue should exist");

        assert!(
            reloaded_dialogue.contains_open_merchant_for_npc(&reloaded_npc.id),
            "save/load roundtrip must preserve explicit OpenMerchant for generated merchant dialogue"
        );
        assert!(
            reloaded_dialogue.validate().is_ok(),
            "reloaded generated merchant dialogue should remain valid"
        );
    }

    #[test]
    fn test_repaired_merchant_dialogue_roundtrip_remains_runtime_valid() {
        let mut state = NpcEditorState::new();
        let mut dialogue = DialogueTree::new(24, "Repairable Merchant Dialogue", 1);
        let mut root = DialogueNode::new(1, "Welcome back.");
        root.add_choice(DialogueChoice::new("Tell me about the town.", Some(3)));
        dialogue.add_node(root);
        dialogue.add_node(DialogueNode::new(3, "The market is busy today."));
        state.available_dialogues = vec![dialogue];
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "merchant_repaired_roundtrip".to_string();
        state.edit_buffer.name = "Repaired Merchant".to_string();
        state.edit_buffer.description = "Merchant with repaired dialogue".to_string();
        state.edit_buffer.portrait_id = "merchant_repaired_roundtrip".to_string();
        state.edit_buffer.dialogue_id = "24".to_string();
        state.edit_buffer.is_merchant = true;

        let repair_message = state
            .repair_merchant_dialogue_for_buffer()
            .expect("merchant dialogue repair should succeed");
        assert!(
            repair_message.contains("Repaired merchant dialogue 24"),
            "repair should augment the assigned dialogue"
        );

        let npc = state
            .build_npc_from_edit_buffer(true)
            .expect("edit buffer should produce repaired merchant npc");

        // Commit the built NPC into the editor's list so it is included in save_to_file.
        state.npcs.push(npc.clone());

        let repaired_dialogue = state
            .available_dialogues
            .iter()
            .find(|dialogue| dialogue.id == 24)
            .expect("repaired dialogue should exist");

        assert!(
            repaired_dialogue.contains_open_merchant_for_npc(&npc.id),
            "repaired merchant dialogue must contain explicit OpenMerchant for runtime compatibility"
        );
        assert!(
            repaired_dialogue.validate().is_ok(),
            "repaired merchant dialogue should remain structurally valid"
        );
        assert!(
            repaired_dialogue.get_node(3).is_some(),
            "repair must preserve unrelated authored dialogue nodes"
        );

        let temp_dir = tempfile::tempdir().expect("tempdir");
        let dialogues_path = temp_dir.path().join("dialogues.ron");
        let npcs_path = temp_dir.path().join("npcs.ron");

        state
            .merchant_dialogue_editor
            .save_to_file(&dialogues_path)
            .expect("save repaired dialogues");
        state.save_to_file(&npcs_path).expect("save npcs");

        let mut reloaded_dialogues = DialogueEditorState::new();
        reloaded_dialogues
            .load_from_file(&dialogues_path)
            .expect("reload repaired dialogues");

        let reloaded_npcs_contents =
            std::fs::read_to_string(&npcs_path).expect("read saved npcs file");
        let reloaded_npcs: Vec<NpcDefinition> =
            ron::from_str(&reloaded_npcs_contents).expect("parse saved npcs");

        let reloaded_npc = reloaded_npcs
            .iter()
            .find(|npc| npc.id == "merchant_repaired_roundtrip")
            .expect("reloaded repaired merchant npc should exist");
        let reloaded_dialogue = reloaded_dialogues
            .dialogues
            .iter()
            .find(|dialogue| dialogue.id == 24)
            .expect("reloaded repaired dialogue should exist");

        assert!(
            reloaded_dialogue.contains_open_merchant_for_npc(&reloaded_npc.id),
            "save/load roundtrip must preserve explicit OpenMerchant for repaired merchant dialogue"
        );
        assert!(
            reloaded_dialogue.validate().is_ok(),
            "reloaded repaired merchant dialogue should remain valid"
        );
        assert!(
            reloaded_dialogue.get_node(3).is_some(),
            "reloaded repaired dialogue must preserve authored non-merchant content"
        );
    }

    #[test]
    fn test_npc_editor_roundtrip_preserves_sprite_metadata() {
        let temp_path = std::env::temp_dir().join("antares_test_npcs_sprite.ron");
        // Clean up any previous file
        let _ = std::fs::remove_file(&temp_path);

        let sprite = antares::domain::world::SpriteReference {
            sheet_path: "assets/sprites/actors/wizard.png".to_string(),
            sprite_index: 12,
            animation: None,
            material_properties: None,
        };

        let npc = NpcDefinition {
            id: "wizard".to_string(),
            name: "Wizard".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            sprite: Some(sprite.clone()),
            dialogue_id: None,
            creature_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        };

        let mut state = NpcEditorState::new();
        state.npcs.push(npc);

        // Save to file
        state
            .save_to_file(&temp_path)
            .expect("Failed to save npcs.ron for test");

        // Load back
        let contents = std::fs::read_to_string(&temp_path).expect("Failed to read saved npcs file");
        let loaded: Vec<NpcDefinition> =
            ron::from_str(&contents).expect("Failed to parse saved npcs.ron");
        assert_eq!(loaded.len(), 1);
        let loaded_npc = &loaded[0];
        assert!(loaded_npc.sprite.is_some());
        let s = loaded_npc.sprite.as_ref().unwrap();
        assert_eq!(s.sheet_path, "assets/sprites/actors/wizard.png");
        assert_eq!(s.sprite_index, 12);

        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_save_npc_edit_mode() {
        let mut state = NpcEditorState::new();

        // Add an NPC first
        state.npcs.push(NpcDefinition {
            id: "npc1".to_string(),
            name: "Original".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            sprite: None,
            dialogue_id: None,
            creature_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        });

        // Edit it
        state.mode = NpcEditorMode::Edit;
        state.selected_npc = Some(0);
        state.edit_buffer.id = "npc1".to_string();
        state.edit_buffer.name = "Modified".to_string();

        let result = state.save_npc();
        assert!(result);
        assert_eq!(state.npcs.len(), 1);
        assert_eq!(state.npcs[0].name, "Modified");
    }

    #[test]
    fn test_matches_filters_no_filters() {
        let state = NpcEditorState::new();
        let npc = NpcDefinition {
            id: "test".to_string(),
            name: "Test NPC".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            sprite: None,
            dialogue_id: None,
            creature_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        };
        assert!(state.matches_filters(&npc));
    }

    #[test]
    fn test_matches_filters_search() {
        let mut state = NpcEditorState::new();
        state.search_filter = "merchant".to_string();

        let npc = NpcDefinition {
            id: "test".to_string(),
            name: "Merchant Bob".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            sprite: None,
            dialogue_id: None,
            creature_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        };
        assert!(state.matches_filters(&npc));

        let npc2 = NpcDefinition {
            id: "test2".to_string(),
            name: "Innkeeper Jane".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            sprite: None,
            dialogue_id: None,
            creature_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: true,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        };
        assert!(!state.matches_filters(&npc2));
    }

    #[test]
    fn test_matches_filters_merchant_filter() {
        let mut state = NpcEditorState::new();
        state.filter_merchants = true;

        let merchant = NpcDefinition {
            id: "merchant".to_string(),
            name: "Bob".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            sprite: None,
            dialogue_id: None,
            creature_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        };
        assert!(state.matches_filters(&merchant));

        let non_merchant = NpcDefinition {
            id: "guard".to_string(),
            name: "Guard".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            sprite: None,
            dialogue_id: None,
            creature_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        };
        assert!(!state.matches_filters(&non_merchant));
    }

    #[test]
    fn test_next_npc_id() {
        let mut state = NpcEditorState::new();

        let id1 = state.next_npc_id();
        assert_eq!(id1, "npc_1");

        state.npcs.push(NpcDefinition {
            id: "npc_1".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            sprite: None,
            dialogue_id: None,
            creature_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        });

        let id2 = state.next_npc_id();
        assert_eq!(id2, "npc_2");
    }

    #[test]
    fn test_is_valid_id() {
        let state = NpcEditorState::new();
        assert!(state.is_valid_id("valid_id"));
        assert!(state.is_valid_id("_valid"));
        assert!(state.is_valid_id("Valid123"));
        assert!(!state.is_valid_id("123invalid"));
        assert!(!state.is_valid_id("invalid-id"));
        assert!(!state.is_valid_id(""));
    }

    #[test]
    fn test_start_edit_npc_resets_autocomplete_buffers() {
        // Ensure stored autocomplete buffers are cleared when starting an edit so the
        // UI shows values from the loaded buffer rather than stale typed text.
        let mut state = NpcEditorState::new();
        let ctx = egui::Context::default();

        // Populate stale buffer
        crate::ui_helpers::store_autocomplete_buffer(
            &ctx,
            egui::Id::new("autocomplete:portrait:npc_edit_portrait"),
            "character_040",
        );

        // Create NPC and start editing it
        state.npcs.push(NpcDefinition {
            id: "whisper".to_string(),
            name: "Whisper".to_string(),
            description: String::new(),
            portrait_id: "character_055".to_string(),
            sprite: None,
            dialogue_id: None,
            creature_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        });

        state.start_edit_npc(0);
        assert!(state.reset_autocomplete_buffers);

        // Render the form (this will clear previous buffer and store current value)
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                state.show_edit_view(ui, None, "data/npcs.ron", None);
            });
        });

        let portrait_buf = crate::ui_helpers::load_autocomplete_buffer(
            &ctx,
            egui::Id::new("autocomplete:portrait:npc_edit_portrait"),
            String::new,
        );
        assert_eq!(portrait_buf, "character_055");
    }

    #[test]
    fn test_save_npc_persists_when_campaign_dir_known() {
        // Verify save_npc() will persist to disk when a campaign dir & filename are known.
        let tmp = tempfile::tempdir().expect("Failed to create tempdir");
        let dir = tmp.path().to_path_buf();

        let mut state = NpcEditorState::new();

        // Tell the editor where NPCs should be written
        state.last_campaign_dir = Some(dir.clone());
        state.last_npcs_file = Some("data/npcs.ron".to_string());

        // Create and save a new NPC
        state.start_add_npc();
        state.edit_buffer.id = "test_npc".to_string();
        state.edit_buffer.name = "Test NPC".to_string();
        state.edit_buffer.description = "Test description".to_string();

        assert!(state.save_npc());

        let path = dir.join("data/npcs.ron");
        assert!(path.exists());
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("test_npc"));
    }

    #[test]
    fn test_validate_duplicate_id_add_mode() {
        let mut state = NpcEditorState::new();
        state.npcs.push(NpcDefinition {
            id: "existing".to_string(),
            name: "Existing NPC".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            sprite: None,
            dialogue_id: None,
            creature_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        });

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "existing".to_string();
        state.edit_buffer.name = "New NPC".to_string();
        state.validate_edit_buffer();

        assert!(!state.validation_errors.is_empty());
        assert!(state
            .validation_errors
            .iter()
            .any(|e| e.contains("already exists")));
    }

    #[test]
    fn test_npc_editor_mode_equality() {
        assert_eq!(NpcEditorMode::List, NpcEditorMode::List);
        assert_ne!(NpcEditorMode::List, NpcEditorMode::Add);
    }

    #[test]
    fn test_portrait_picker_initial_state() {
        let state = NpcEditorState::new();
        assert!(!state.portrait_picker_open);
        assert!(state.portrait_textures.is_empty());
        assert!(state.available_portraits.is_empty());
        assert!(state.last_campaign_dir.is_none());
    }

    #[test]
    fn test_portrait_picker_open_flag() {
        let mut state = NpcEditorState::new();
        assert!(!state.portrait_picker_open);

        state.portrait_picker_open = true;
        assert!(state.portrait_picker_open);

        state.portrait_picker_open = false;
        assert!(!state.portrait_picker_open);
    }

    #[test]
    fn test_portrait_texture_cache_insertion() {
        let mut state = NpcEditorState::new();
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
    fn test_portrait_texture_error_handling_missing_file() {
        // Test that missing portrait files are handled gracefully
        let mut state = NpcEditorState::new();
        let ctx = egui::Context::default();
        let campaign_dir = PathBuf::from("/nonexistent/path");

        // Attempt to load a portrait that doesn't exist
        let loaded = state.load_portrait_texture(&ctx, Some(&campaign_dir), "999");

        // Should return false and cache the failure
        assert!(!loaded);
        assert!(state.portrait_textures.contains_key("999"));
        assert!(state.portrait_textures.get("999").unwrap().is_none());
    }

    #[test]
    fn test_portrait_texture_error_handling_no_campaign_dir() {
        // Test behavior when no campaign directory is provided
        let mut state = NpcEditorState::new();
        let ctx = egui::Context::default();

        // Attempt to load portrait without campaign directory
        let loaded = state.load_portrait_texture(&ctx, None, "0");

        // Should fail gracefully and cache the failure
        assert!(!loaded);
        assert!(state.portrait_textures.contains_key("0"));
        assert!(state.portrait_textures.get("0").unwrap().is_none());
    }

    #[test]
    fn test_portrait_texture_cache_efficiency() {
        // Test that cached textures aren't reloaded
        let mut state = NpcEditorState::new();
        let ctx = egui::Context::default();

        // Pre-cache a failed load
        state.portrait_textures.insert("cached".to_string(), None);

        // Try to load again - should return cached result without attempting load
        let loaded = state.load_portrait_texture(&ctx, None, "cached");
        assert!(!loaded);

        // Cache should still have only one entry
        assert_eq!(state.portrait_textures.len(), 1);
    }

    #[test]
    fn test_new_npc_creation_workflow_with_portrait() {
        let mut state = NpcEditorState::new();

        // Start adding new NPC
        state.start_add_npc();
        assert_eq!(state.mode, NpcEditorMode::Add);

        // Fill in form including portrait
        state.edit_buffer.id = "merchant_bob".to_string();
        state.edit_buffer.name = "Bob the Merchant".to_string();
        state.edit_buffer.portrait_id = "42".to_string();
        state.edit_buffer.is_merchant = true;

        // Validate and save
        state.validate_edit_buffer();
        assert!(state.validation_errors.is_empty());

        let saved = state.save_npc();
        assert!(saved);

        // Verify NPC was created with portrait
        assert_eq!(state.npcs.len(), 1);
        assert_eq!(state.npcs[0].portrait_id, "42");
        assert_eq!(state.npcs[0].name, "Bob the Merchant");
    }

    #[test]
    fn test_edit_npc_workflow_updates_portrait() {
        let mut state = NpcEditorState::new();

        // Add initial NPC
        state.npcs.push(NpcDefinition {
            id: "guard".to_string(),
            name: "Guard".to_string(),
            description: String::new(),
            portrait_id: "10".to_string(),
            sprite: None,
            dialogue_id: None,
            creature_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        });

        // Start editing
        state.start_edit_npc(0);
        assert_eq!(state.mode, NpcEditorMode::Edit);
        assert_eq!(state.edit_buffer.portrait_id, "10");

        // Change portrait
        state.edit_buffer.portrait_id = "20".to_string();
        state.save_npc();

        // Verify portrait was updated
        assert_eq!(state.npcs[0].portrait_id, "20");
    }

    #[test]
    fn test_campaign_dir_change_triggers_portrait_rescan() {
        let mut state = NpcEditorState::new();

        // Initially no portraits cached
        assert!(state.available_portraits.is_empty());
        assert!(state.last_campaign_dir.is_none());

        // Simulate campaign directory change detection
        let dir1 = PathBuf::from("/campaign1");
        state.last_campaign_dir = Some(dir1.clone());

        let dir2 = PathBuf::from("/campaign2");
        assert_ne!(state.last_campaign_dir, Some(dir2.clone()));

        // This would trigger portrait rescan in the show() method
        state.last_campaign_dir = Some(dir2);
        assert!(state.last_campaign_dir.is_some());
    }

    #[test]
    fn test_npc_save_preserves_portrait_data() {
        let mut state = NpcEditorState::new();

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "wizard".to_string();
        state.edit_buffer.name = "Wizard".to_string();
        state.edit_buffer.portrait_id = "magic_user".to_string();
        state.edit_buffer.description = "A wise wizard".to_string();

        state.save_npc();

        assert_eq!(state.npcs[0].portrait_id, "magic_user");

        // Verify portrait ID is preserved in serialization
        let npc = &state.npcs[0];
        assert_eq!(npc.portrait_id, "magic_user");
    }

    #[test]
    fn test_multiple_npcs_different_portraits() {
        let mut state = NpcEditorState::new();

        // Add first NPC
        state.start_add_npc();
        state.edit_buffer.id = "npc1".to_string();
        state.edit_buffer.name = "NPC 1".to_string();
        state.edit_buffer.portrait_id = "1".to_string();
        state.save_npc();

        // Add second NPC
        state.start_add_npc();
        state.edit_buffer.id = "npc2".to_string();
        state.edit_buffer.name = "NPC 2".to_string();
        state.edit_buffer.portrait_id = "2".to_string();
        state.save_npc();

        // Verify both NPCs have different portraits
        assert_eq!(state.npcs.len(), 2);
        assert_eq!(state.npcs[0].portrait_id, "1");
        assert_eq!(state.npcs[1].portrait_id, "2");
    }

    #[test]
    fn test_portrait_id_empty_string_allowed() {
        let mut state = NpcEditorState::new();

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "no_portrait_npc".to_string();
        state.edit_buffer.name = "No Portrait".to_string();
        state.edit_buffer.portrait_id = String::new();

        state.validate_edit_buffer();
        assert!(state.validation_errors.is_empty());

        state.save_npc();
        assert_eq!(state.npcs[0].portrait_id, "");
    }

    // ── Stock Template Field Tests ───────────────────────────────────

    #[test]
    fn test_npc_edit_buffer_stock_template_default_empty() {
        let buf = NpcEditBuffer::default();
        assert_eq!(buf.stock_template, "");
    }

    #[test]
    fn test_save_npc_merchant_with_stock_template() {
        let mut state = NpcEditorState::new();
        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "merchant_bob".to_string();
        state.edit_buffer.name = "Merchant Bob".to_string();
        state.edit_buffer.portrait_id = "bob.png".to_string();
        state.edit_buffer.is_merchant = true;
        state.edit_buffer.stock_template = "blacksmith".to_string();

        let saved = state.save_npc();
        assert!(saved, "save_npc should succeed");
        assert_eq!(state.npcs[0].stock_template, Some("blacksmith".to_string()));
    }

    #[test]
    fn test_save_npc_merchant_no_template() {
        let mut state = NpcEditorState::new();
        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "merchant_empty".to_string();
        state.edit_buffer.name = "Empty Merchant".to_string();
        state.edit_buffer.portrait_id = "em.png".to_string();
        state.edit_buffer.is_merchant = true;
        state.edit_buffer.stock_template = String::new();

        let saved = state.save_npc();
        assert!(saved, "save_npc should succeed");
        assert_eq!(
            state.npcs[0].stock_template, None,
            "empty stock_template string should serialise as None"
        );
    }

    #[test]
    fn test_start_edit_npc_populates_stock_template() {
        use antares::domain::world::NpcDefinition;

        let mut state = NpcEditorState::new();

        let npc = NpcDefinition {
            id: "wizard_shop_keeper".to_string(),
            name: "Mysto".to_string(),
            description: "A mysterious wizard".to_string(),
            portrait_id: "wizard.png".to_string(),
            dialogue_id: None,
            creature_id: None,
            sprite: None,
            quest_ids: vec![],
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
            is_priest: false,
            stock_template: Some("wizard_shop".to_string()),
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        };
        state.npcs.push(npc);

        state.start_edit_npc(0);
        assert_eq!(state.edit_buffer.stock_template, "wizard_shop");
    }

    #[test]
    fn test_npc_creature_picker_initial_state() {
        let state = NpcEditorState::default();
        assert!(!state.creature_picker_open);
    }

    #[test]
    fn test_npc_apply_selected_creature_id_updates_buffer() {
        let mut state = NpcEditorState::default();
        state.apply_selected_creature_id("1000".to_string());
        assert_eq!(state.edit_buffer.creature_id, "1000");
        assert!(!state.creature_picker_open);
    }

    #[test]
    fn test_requested_template_edit_set_on_click() {
        let mut state = NpcEditorState::new();
        assert!(state.requested_template_edit.is_none());

        // Simulate what the UI does when "✏ Edit template" is clicked
        state.requested_template_edit = Some("foo".to_string());
        assert_eq!(state.requested_template_edit, Some("foo".to_string()));

        // Consuming it (as CampaignBuilderApp does via .take()) clears it
        let taken = state.requested_template_edit.take();
        assert_eq!(taken, Some("foo".to_string()));
        assert!(state.requested_template_edit.is_none());
    }

    // ── Reload / load_from_file tests ─────────────────────────────────────────

    /// `pending_status` starts as `None` so the parent's status bar is not
    /// polluted before any action has been taken.
    #[test]
    fn test_pending_status_initial_state() {
        let state = NpcEditorState::new();
        assert!(state.pending_status.is_none());
    }

    /// `load_from_file` replaces `self.npcs` with the file contents, clears the
    /// selection, resets to List mode, and clears the unsaved-changes flag.
    #[test]
    fn test_load_from_file_replaces_npcs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("npcs.ron");

        // Write two NPCs to disk.
        let original = vec![
            NpcDefinition {
                id: "npc_a".to_string(),
                name: "Alpha".to_string(),
                description: String::new(),
                portrait_id: String::new(),
                sprite: None,
                dialogue_id: None,
                creature_id: None,
                quest_ids: Vec::new(),
                faction: None,
                is_merchant: false,
                is_innkeeper: false,
                is_priest: false,
                stock_template: None,
                service_catalog: None,
                economy: None,
                is_trainer: false,
                training_fee_base: None,
                training_fee_multiplier: None,
            },
            NpcDefinition {
                id: "npc_b".to_string(),
                name: "Beta".to_string(),
                description: String::new(),
                portrait_id: String::new(),
                sprite: None,
                dialogue_id: None,
                creature_id: None,
                quest_ids: Vec::new(),
                faction: None,
                is_merchant: false,
                is_innkeeper: false,
                is_priest: false,
                stock_template: None,
                service_catalog: None,
                economy: None,
                is_trainer: false,
                training_fee_base: None,
                training_fee_multiplier: None,
            },
        ];
        let ron_str =
            ron::ser::to_string_pretty(&original, ron::ser::PrettyConfig::default()).unwrap();
        std::fs::write(&path, ron_str).unwrap();

        // Start the editor with a different, stale NPC list.
        let mut state = NpcEditorState::new();
        state.npcs.push(NpcDefinition {
            id: "stale".to_string(),
            name: "Stale NPC".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            sprite: None,
            dialogue_id: None,
            creature_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        });
        state.selected_npc = Some(0);
        state.mode = NpcEditorMode::Edit;
        state.has_unsaved_changes = true;

        // Reload from file.
        state.load_from_file(&path).expect("load_from_file failed");

        assert_eq!(state.npcs.len(), 2);
        assert_eq!(state.npcs[0].id, "npc_a");
        assert_eq!(state.npcs[1].id, "npc_b");
        assert!(state.selected_npc.is_none(), "selection must be cleared");
        assert_eq!(state.mode, NpcEditorMode::List, "must return to List mode");
        assert!(
            !state.has_unsaved_changes,
            "no unsaved changes after reload"
        );
    }

    /// `load_from_file` returns an `Err` when the file does not exist.
    #[test]
    fn test_load_from_file_missing_file_returns_err() {
        let mut state = NpcEditorState::new();
        let result = state.load_from_file(std::path::Path::new("/nonexistent/path/npcs.ron"));
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.to_string().contains("IO error"),
            "error should mention IO error, got: {msg}"
        );
    }

    /// `load_from_file` returns an `Err` when the file contains invalid RON.
    #[test]
    fn test_load_from_file_bad_ron_returns_err() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("npcs.ron");
        std::fs::write(&path, "this is not valid RON at all!!!").unwrap();

        let mut state = NpcEditorState::new();
        let result = state.load_from_file(&path);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.to_string().contains("Parse error"),
            "error should mention parse failure, got: {msg}"
        );
    }

    /// After a successful reload the `pending_status` field must contain a
    /// message that the parent can forward to the global status bar.
    #[test]
    fn test_reload_sets_pending_status_on_success() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().to_path_buf();
        let npcs_file = "data/npcs.ron";
        let path = dir.join(npcs_file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        // Write an empty NPC list.
        std::fs::write(&path, "[]").unwrap();

        let mut state = NpcEditorState::new();
        assert!(state.pending_status.is_none());

        // Simulate the Reload toolbar action as show() would execute it.
        if dir.join(npcs_file).exists() {
            match state.load_from_file(&dir.join(npcs_file)) {
                Ok(()) => {
                    state.pending_status =
                        Some(format!("Reloaded {} NPCs from disk", state.npcs.len()));
                }
                Err(e) => {
                    state.pending_status = Some(format!("Failed to reload NPCs: {}", e));
                }
            }
        }

        assert!(state.pending_status.is_some());
        let status = state.pending_status.take().unwrap();
        assert!(
            status.contains("Reloaded"),
            "success message should mention Reloaded, got: {status}"
        );
        // take() should clear the field.
        assert!(state.pending_status.is_none());
    }

    /// When the NPCs file does not exist, `pending_status` should contain an
    /// explanatory message and `self.npcs` must remain unchanged.
    #[test]
    fn test_reload_sets_pending_status_when_file_missing() {
        let mut state = NpcEditorState::new();
        state.npcs.push(NpcDefinition {
            id: "guard".to_string(),
            name: "Guard".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            sprite: None,
            dialogue_id: None,
            creature_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        });

        let missing_path = std::path::Path::new("/no/such/dir/npcs.ron");

        // Simulate the path.exists() guard in the Reload handler.
        if !missing_path.exists() {
            state.pending_status = Some(format!("NPCs file not found: {}", missing_path.display()));
        }

        assert!(state.pending_status.is_some());
        let status = state.pending_status.take().unwrap();
        assert!(
            status.contains("not found"),
            "message should mention not found, got: {status}"
        );
        // NPCs list must be unmodified.
        assert_eq!(
            state.npcs.len(),
            1,
            "npcs must be unchanged when file is missing"
        );
    }

    // ── Phase 4: Create Merchant Dialog ──────────────────────────────────────

    /// Simulates clicking "Create merchant dialogue" for a merchant NPC that
    /// has no pre-assigned dialogue and asserts that:
    ///
    /// - a `DialogueTree` is created and stored in `available_dialogues`
    /// - the generated tree contains an explicit `OpenMerchant` action for the NPC
    /// - the root node has at least two response branches (browse + goodbye)
    /// - `edit_buffer.dialogue_id` is set to the new dialogue's id
    #[test]
    fn test_create_merchant_dialog_generates_dialog() {
        let mut state = NpcEditorState::new();
        state.available_dialogues = Vec::new();
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "merchant_vendor".to_string();
        state.edit_buffer.name = "Vendor".to_string();
        state.edit_buffer.description = "A friendly vendor".to_string();
        state.edit_buffer.portrait_id = "vendor".to_string();
        state.edit_buffer.is_merchant = true;
        // No dialogue_id assigned — simulates clicking "Create merchant dialogue"
        // when the NPC has no existing dialogue.

        let result = state.create_or_repair_merchant_dialogue_for_buffer();
        assert!(result.is_ok(), "create should succeed: {:?}", result);

        let message = result.unwrap();
        assert!(
            message.contains("Created merchant dialogue"),
            "expected 'Created' status, got: {message}"
        );

        // A dialogue must now be present in the campaign data.
        assert_eq!(
            state.available_dialogues.len(),
            1,
            "exactly one dialogue should be present after creation"
        );

        // Retrieve the generated dialogue.
        let generated = &state.available_dialogues[0];

        // The dialogue must contain an explicit OpenMerchant action for this NPC.
        assert!(
            generated.contains_open_merchant_for_npc("merchant_vendor"),
            "generated dialogue must contain OpenMerchant for the NPC"
        );

        // The dialogue must carry SDK-managed content metadata.
        assert!(
            generated.has_sdk_managed_merchant_content(),
            "generated dialogue must be marked as SDK-managed"
        );

        // The root node must have at least two response branches —
        // one "Show me your wares." / browse choice and one "Farewell." / goodbye choice.
        let root = generated
            .get_node(generated.root_node)
            .expect("root node must exist");
        assert!(
            root.choices.len() >= 2,
            "root node must have at least two choices (browse + goodbye), got: {}",
            root.choices.len()
        );

        // The edit buffer dialogue_id must now reference the new dialogue.
        let generated_id_str = generated.id.to_string();
        assert_eq!(
            state.edit_buffer.dialogue_id, generated_id_str,
            "buffer.dialogue_id must be set to the newly created dialogue id"
        );
    }

    /// Calls the create-merchant-dialogue action for two different NPCs and
    /// asserts that each receives a distinct dialogue id so no collision occurs
    /// in the campaign's dialogue collection.
    #[test]
    fn test_create_merchant_dialog_id_is_unique() {
        let mut state = NpcEditorState::new();
        state.available_dialogues = Vec::new();
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        // ── First NPC ──────────────────────────────────────────────────────
        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "merchant_alpha".to_string();
        state.edit_buffer.name = "Alpha Merchant".to_string();
        state.edit_buffer.description = "First merchant".to_string();
        state.edit_buffer.portrait_id = "alpha".to_string();
        state.edit_buffer.is_merchant = true;
        state.edit_buffer.dialogue_id = String::new();

        let first_result = state.create_or_repair_merchant_dialogue_for_buffer();
        assert!(
            first_result.is_ok(),
            "first create should succeed: {:?}",
            first_result
        );

        let first_dialogue_id = state.edit_buffer.dialogue_id.clone();
        assert!(
            !first_dialogue_id.is_empty(),
            "first dialogue id must be set after creation"
        );

        // ── Second NPC ─────────────────────────────────────────────────────
        state.edit_buffer.id = "merchant_beta".to_string();
        state.edit_buffer.name = "Beta Merchant".to_string();
        state.edit_buffer.description = "Second merchant".to_string();
        state.edit_buffer.portrait_id = "beta".to_string();
        state.edit_buffer.is_merchant = true;
        state.edit_buffer.dialogue_id = String::new(); // no pre-assigned dialogue

        let second_result = state.create_or_repair_merchant_dialogue_for_buffer();
        assert!(
            second_result.is_ok(),
            "second create should succeed: {:?}",
            second_result
        );

        let second_dialogue_id = state.edit_buffer.dialogue_id.clone();
        assert!(
            !second_dialogue_id.is_empty(),
            "second dialogue id must be set after creation"
        );

        // The two generated dialogue ids must be different.
        assert_ne!(
            first_dialogue_id, second_dialogue_id,
            "two different NPCs must receive distinct dialogue ids"
        );

        // Both dialogues must be present in the campaign dialogue collection.
        assert_eq!(
            state.available_dialogues.len(),
            2,
            "two separate dialogues should exist after two creations"
        );

        // Each dialogue must target the correct NPC via OpenMerchant.
        let first_id: u16 = first_dialogue_id
            .parse()
            .expect("first dialogue id must be numeric");
        let second_id: u16 = second_dialogue_id
            .parse()
            .expect("second dialogue id must be numeric");

        assert!(
            state
                .available_dialogues
                .iter()
                .find(|d| d.id == first_id)
                .expect("first dialogue must exist in campaign data")
                .contains_open_merchant_for_npc("merchant_alpha"),
            "first dialogue must open the correct (alpha) merchant"
        );

        assert!(
            state
                .available_dialogues
                .iter()
                .find(|d| d.id == second_id)
                .expect("second dialogue must exist in campaign data")
                .contains_open_merchant_for_npc("merchant_beta"),
            "second dialogue must open the correct (beta) merchant"
        );
    }

    // ── create_or_repair non-merchant guard tests ─────────────────────────────

    /// Verifies that clicking "Create Merchant Dialog" on a non-merchant NPC now
    /// returns a non-empty guidance message instead of the previous silent
    /// empty-string no-op that made the button appear broken.
    ///
    /// Root cause: `create_or_repair_merchant_dialogue_for_buffer` previously
    /// returned `Ok(String::new())` when `is_merchant = false`, which caused
    /// `pending_status = Some("")` → the status bar was cleared with no visible
    /// feedback.
    #[test]
    fn test_create_merchant_dialog_returns_guidance_when_not_merchant() {
        let mut state = NpcEditorState::new();
        state.edit_buffer.id = "innkeeper_01".to_string();
        state.edit_buffer.name = "Village Innkeeper".to_string();
        state.edit_buffer.is_merchant = false; // NOT a merchant
        state.edit_buffer.is_innkeeper = true;

        let result = state.create_or_repair_merchant_dialogue_for_buffer();

        // Must be Ok (not an Err — the user just needs guidance, not an alarm)
        assert!(result.is_ok(), "must return Ok even for non-merchants");
        let message = result.unwrap();
        // Must be non-empty so the status bar shows something actionable
        assert!(
            !message.is_empty(),
            "must return a non-empty guidance message when is_merchant = false; \
             got empty string (the old silent no-op)"
        );
        assert!(
            message.to_lowercase().contains("merchant"),
            "guidance message must mention 'merchant', got: {:?}",
            message
        );
    }

    /// Verifies that a stale `dialogue_id` (pointing to a dialogue tree that no
    /// longer exists in the in-memory list) is cleared automatically so that a
    /// fresh merchant dialogue is created instead of returning the opaque error
    /// "Assigned dialogue X was not found".
    ///
    /// This covers the scenario where a user creates a new stock template in the
    /// same session, edits an NPC that already has `is_merchant = true` and a
    /// `dialogue_id` that somehow became stale (e.g. the campaign was partially
    /// reloaded), and then clicks "Create merchant dialogue".
    #[test]
    fn test_create_merchant_dialog_clears_stale_dialogue_id_and_creates_fresh() {
        let mut state = NpcEditorState::new();
        state.edit_buffer.id = "stale_merchant".to_string();
        state.edit_buffer.name = "Stale Merchant".to_string();
        state.edit_buffer.is_merchant = true;

        // Set a dialogue_id that does NOT exist in merchant_dialogue_editor.dialogues.
        // (The editor starts empty, so id 999 will never be found.)
        state.edit_buffer.dialogue_id = "999".to_string();

        let result = state.create_or_repair_merchant_dialogue_for_buffer();

        assert!(
            result.is_ok(),
            "must not error out with 'Assigned dialogue 999 was not found', got: {:?}",
            result
        );

        // A fresh dialogue must have been created and the buffer's dialogue_id updated.
        assert!(
            !state.edit_buffer.dialogue_id.is_empty(),
            "dialogue_id must be set to the newly created dialogue"
        );
        assert_ne!(
            state.edit_buffer.dialogue_id, "999",
            "stale dialogue_id 999 must have been replaced with the new dialogue's id"
        );
        assert!(
            !state.available_dialogues.is_empty(),
            "a new dialogue tree must have been added to available_dialogues"
        );
    }

    // ── Phase 7 Trainer Tests ─────────────────────────────────────────────────

    /// Verifies that enabling `is_trainer` on an NPC that has no pre-assigned
    /// dialogue automatically creates a standard trainer dialogue template and
    /// assigns its ID to the edit buffer.
    #[test]
    fn test_is_trainer_toggle_auto_applies_training_dialogue() {
        let mut state = NpcEditorState::new();
        state.available_dialogues = Vec::new();
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "master_swordsman".to_string();
        state.edit_buffer.name = "Master Swordsman".to_string();
        state.edit_buffer.portrait_id = "swordsman".to_string();
        state.edit_buffer.is_trainer = true;
        // No dialogue_id assigned — simulates toggling "Is Trainer" on.

        let result = state.auto_apply_trainer_dialogue_to_edit_buffer();
        assert!(result.is_ok(), "auto-apply should succeed: {:?}", result);

        // A dialogue must now be present.
        assert_eq!(
            state.available_dialogues.len(),
            1,
            "exactly one trainer dialogue should be created"
        );

        let generated = &state.available_dialogues[0];

        // The generated dialogue must contain an OpenTraining action for this NPC.
        assert!(
            generated.contains_open_training_for_npc("master_swordsman"),
            "generated dialogue must contain OpenTraining for the NPC"
        );

        // The generated dialogue must carry SDK-managed trainer metadata.
        assert!(
            generated.has_sdk_managed_trainer_content(),
            "generated dialogue must be marked as SDK-managed trainer content"
        );

        // The edit buffer dialogue_id must reference the new dialogue.
        let generated_id_str = generated.id.to_string();
        assert_eq!(
            state.edit_buffer.dialogue_id, generated_id_str,
            "buffer.dialogue_id must be set to the newly created trainer dialogue id"
        );
    }

    /// Verifies that `create_or_repair_trainer_dialogue_for_buffer` returns a
    /// non-empty guidance message when `is_trainer == false`, instead of a
    /// silent empty-string no-op.
    #[test]
    fn test_create_trainer_dialogue_returns_guidance_when_not_trainer() {
        let mut state = NpcEditorState::new();
        state.edit_buffer.id = "innkeeper_01".to_string();
        state.edit_buffer.name = "Village Innkeeper".to_string();
        state.edit_buffer.is_trainer = false;
        state.edit_buffer.is_innkeeper = true;

        let result = state.create_or_repair_trainer_dialogue_for_buffer();

        assert!(result.is_ok(), "must return Ok even for non-trainers");
        let message = result.unwrap();
        assert!(
            !message.is_empty(),
            "must return a non-empty guidance message when is_trainer = false; \
             got empty string (silent no-op)"
        );
        assert!(
            message.to_lowercase().contains("trainer"),
            "guidance message must mention 'trainer', got: {:?}",
            message
        );
    }

    /// Verifies that `create_or_repair_trainer_dialogue_for_buffer` generates a
    /// dialogue tree that contains an `OpenTraining` action when `is_trainer == true`.
    #[test]
    fn test_create_trainer_dialogue_generates_open_training_action() {
        let mut state = NpcEditorState::new();
        state.available_dialogues = Vec::new();
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "trainer_arya".to_string();
        state.edit_buffer.name = "Arya".to_string();
        state.edit_buffer.portrait_id = "arya".to_string();
        state.edit_buffer.is_trainer = true;

        let result = state.create_or_repair_trainer_dialogue_for_buffer();
        assert!(result.is_ok(), "create should succeed: {:?}", result);

        let message = result.unwrap();
        assert!(
            message.contains("Created trainer dialogue"),
            "expected 'Created trainer dialogue' status, got: {message}"
        );

        assert_eq!(
            state.available_dialogues.len(),
            1,
            "exactly one dialogue should exist after creation"
        );

        let generated = &state.available_dialogues[0];

        // Must contain OpenTraining for the correct NPC.
        assert!(
            generated.contains_open_training_for_npc("trainer_arya"),
            "generated dialogue must contain OpenTraining for 'trainer_arya'"
        );

        // Root node must have at least two choices (seek training + goodbye).
        let root = generated
            .get_node(generated.root_node)
            .expect("root node must exist");
        assert!(
            root.choices.len() >= 2,
            "root node must have at least two choices (seek training + goodbye), got: {}",
            root.choices.len()
        );

        // The buffer dialogue_id must now be set.
        let generated_id_str = generated.id.to_string();
        assert_eq!(
            state.edit_buffer.dialogue_id, generated_id_str,
            "buffer.dialogue_id must be set to the newly created trainer dialogue id"
        );
    }

    /// Verifies that `build_npc_from_edit_buffer` round-trips `is_trainer` and
    /// the training fee fields correctly.
    #[test]
    fn test_build_npc_from_edit_buffer_roundtrips_trainer_fields() {
        let mut state = NpcEditorState::new();
        state.edit_buffer.id = "fee_trainer".to_string();
        state.edit_buffer.name = "Fee Trainer".to_string();
        state.edit_buffer.portrait_id = "fee_trainer".to_string();
        state.edit_buffer.is_trainer = true;
        state.edit_buffer.training_fee_base = "300".to_string();
        state.edit_buffer.training_fee_multiplier = "1.5".to_string();

        let npc = state
            .build_npc_from_edit_buffer(false)
            .expect("build should succeed");

        assert!(npc.is_trainer, "is_trainer must be true");
        assert_eq!(
            npc.training_fee_base,
            Some(300),
            "training_fee_base must be 300"
        );
        assert!(
            (npc.training_fee_multiplier.unwrap() - 1.5_f32).abs() < f32::EPSILON,
            "training_fee_multiplier must be 1.5"
        );
    }

    /// Verifies that empty `training_fee_base` and `training_fee_multiplier`
    /// strings produce `None` in the built NpcDefinition (meaning "use campaign
    /// defaults").
    #[test]
    fn test_build_npc_from_edit_buffer_empty_fee_fields_yield_none() {
        let mut state = NpcEditorState::new();
        state.edit_buffer.id = "default_trainer".to_string();
        state.edit_buffer.name = "Default Trainer".to_string();
        state.edit_buffer.portrait_id = "default_trainer".to_string();
        state.edit_buffer.is_trainer = true;
        state.edit_buffer.training_fee_base = String::new();
        state.edit_buffer.training_fee_multiplier = String::new();

        let npc = state
            .build_npc_from_edit_buffer(false)
            .expect("build should succeed");

        assert!(npc.is_trainer, "is_trainer must be true");
        assert_eq!(
            npc.training_fee_base, None,
            "empty training_fee_base must yield None"
        );
        assert_eq!(
            npc.training_fee_multiplier, None,
            "empty training_fee_multiplier must yield None"
        );
    }

    /// Verifies that `filter_trainers` hides NPCs whose `is_trainer` flag is
    /// `false` and shows those whose flag is `true`.
    #[test]
    fn test_filter_trainers_hides_non_trainer_npcs() {
        let trainer = NpcDefinition {
            id: "trainer_npc".to_string(),
            name: "Trainer NPC".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            dialogue_id: None,
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            is_trainer: true,
            stock_template: None,
            service_catalog: None,
            economy: None,
            training_fee_base: None,
            training_fee_multiplier: None,
        };
        let regular = NpcDefinition {
            id: "regular_npc".to_string(),
            name: "Regular NPC".to_string(),
            description: String::new(),
            portrait_id: String::new(),
            dialogue_id: None,
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            is_trainer: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            training_fee_base: None,
            training_fee_multiplier: None,
        };

        let mut state = NpcEditorState::new();
        state.npcs = vec![trainer.clone(), regular.clone()];

        // Without filter: both should pass.
        assert!(
            state.matches_filters(&trainer),
            "trainer must pass when filter_trainers is false"
        );
        assert!(
            state.matches_filters(&regular),
            "regular NPC must pass when filter_trainers is false"
        );

        // With filter: only the trainer should pass.
        state.filter_trainers = true;
        assert!(
            state.matches_filters(&trainer),
            "trainer must pass when filter_trainers is true"
        );
        assert!(
            !state.matches_filters(&regular),
            "non-trainer NPC must be hidden when filter_trainers is true"
        );
    }

    /// Verifies that `save_npc` in Add mode correctly persists `is_trainer` and
    /// the training fee fields to the stored NpcDefinition.
    #[test]
    fn test_save_npc_persists_trainer_fields() {
        let mut state = NpcEditorState::new();
        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "saved_trainer".to_string();
        state.edit_buffer.name = "Saved Trainer".to_string();
        state.edit_buffer.portrait_id = "saved_trainer".to_string();
        state.edit_buffer.is_trainer = true;
        state.edit_buffer.training_fee_base = "500".to_string();
        state.edit_buffer.training_fee_multiplier = "2.0".to_string();

        let saved = state.save_npc();
        assert!(saved, "save_npc must succeed");
        assert_eq!(state.npcs.len(), 1);

        let npc = &state.npcs[0];
        assert!(npc.is_trainer, "saved NPC must have is_trainer = true");
        assert_eq!(npc.training_fee_base, Some(500));
        assert!(
            (npc.training_fee_multiplier.unwrap() - 2.0_f32).abs() < f32::EPSILON,
            "training_fee_multiplier must be 2.0"
        );
    }

    /// Verifies that `start_edit_npc` correctly populates trainer fields in the
    /// edit buffer when loading an existing trainer NPC.
    #[test]
    fn test_start_edit_npc_populates_trainer_fields() {
        let npc = NpcDefinition {
            id: "trainer_populator".to_string(),
            name: "Trainer Populator".to_string(),
            description: String::new(),
            portrait_id: "tp".to_string(),
            dialogue_id: None,
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            is_trainer: true,
            stock_template: None,
            service_catalog: None,
            economy: None,
            training_fee_base: Some(250),
            training_fee_multiplier: Some(1.25),
        };

        let mut state = NpcEditorState::new();
        state.npcs.push(npc);
        state.start_edit_npc(0);

        assert!(
            state.edit_buffer.is_trainer,
            "edit buffer is_trainer must be true"
        );
        assert_eq!(
            state.edit_buffer.training_fee_base, "250",
            "training_fee_base must be '250'"
        );
        assert_eq!(
            state.edit_buffer.training_fee_multiplier, "1.25",
            "training_fee_multiplier must be '1.25'"
        );
    }

    /// Verifies that removing trainer content from a trainer template dialogue
    /// leaves the dialogue asset intact in the available_dialogues collection
    /// (mirrors the merchant equivalent test).
    #[test]
    fn test_remove_trainer_dialogue_from_generated_template_leaves_dialogue_intact() {
        let mut state = NpcEditorState::new();
        let dialogue = DialogueTree::standard_trainer_template(50, "trainer_zeus", "Zeus");
        state.available_dialogues = vec![dialogue];
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.edit_buffer.id = "trainer_zeus".to_string();
        state.edit_buffer.name = "Zeus".to_string();
        state.edit_buffer.portrait_id = "zeus".to_string();
        state.edit_buffer.dialogue_id = "50".to_string();
        state.edit_buffer.is_trainer = false; // no longer a trainer

        let message = state
            .remove_trainer_dialogue_from_edit_buffer()
            .expect("remove must succeed");
        assert!(
            !message.is_empty(),
            "remove must return a non-empty status message"
        );

        // The dialogue asset itself must still be present.
        assert_eq!(
            state.available_dialogues.len(),
            1,
            "dialogue asset must remain after trainer content removal"
        );

        // The dialogue must no longer contain OpenTraining for this NPC.
        assert!(
            !state.available_dialogues[0].contains_open_training_for_npc("trainer_zeus"),
            "OpenTraining action must have been removed"
        );
    }

    /// Verifies that two different trainer NPCs receive distinct dialogue IDs.
    #[test]
    fn test_create_trainer_dialogue_id_is_unique() {
        let mut state = NpcEditorState::new();
        state.available_dialogues = Vec::new();
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        // First trainer NPC
        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "trainer_alpha".to_string();
        state.edit_buffer.name = "Alpha Trainer".to_string();
        state.edit_buffer.portrait_id = "alpha".to_string();
        state.edit_buffer.is_trainer = true;
        state.edit_buffer.dialogue_id = String::new();

        let first_result = state.create_or_repair_trainer_dialogue_for_buffer();
        assert!(first_result.is_ok(), "first create must succeed");
        let first_id = state.edit_buffer.dialogue_id.clone();
        assert!(!first_id.is_empty(), "first dialogue_id must be set");

        // Second trainer NPC
        state.edit_buffer.id = "trainer_beta".to_string();
        state.edit_buffer.name = "Beta Trainer".to_string();
        state.edit_buffer.portrait_id = "beta".to_string();
        state.edit_buffer.is_trainer = true;
        state.edit_buffer.dialogue_id = String::new();

        let second_result = state.create_or_repair_trainer_dialogue_for_buffer();
        assert!(second_result.is_ok(), "second create must succeed");
        let second_id = state.edit_buffer.dialogue_id.clone();
        assert!(!second_id.is_empty(), "second dialogue_id must be set");

        assert_ne!(
            first_id, second_id,
            "two trainers must receive distinct dialogue ids"
        );
        assert_eq!(
            state.available_dialogues.len(),
            2,
            "two dialogues must exist"
        );

        let first_parsed: u16 = first_id.parse().expect("first id must be numeric");
        let second_parsed: u16 = second_id.parse().expect("second id must be numeric");

        assert!(
            state
                .available_dialogues
                .iter()
                .find(|d| d.id == first_parsed)
                .expect("first dialogue must exist")
                .contains_open_training_for_npc("trainer_alpha"),
            "first dialogue must open correct trainer"
        );
        assert!(
            state
                .available_dialogues
                .iter()
                .find(|d| d.id == second_parsed)
                .expect("second dialogue must exist")
                .contains_open_training_for_npc("trainer_beta"),
            "second dialogue must open correct trainer"
        );
    }

    /// Verifies that `is_merchant` and `is_trainer` are fully independent — an
    /// NPC may be both, and merchant/trainer dialogue operations do not interfere.
    #[test]
    fn test_merchant_and_trainer_are_independent() {
        let mut state = NpcEditorState::new();
        state.available_dialogues = Vec::new();
        state
            .merchant_dialogue_editor
            .load_dialogues(state.available_dialogues.clone());

        state.mode = NpcEditorMode::Add;
        state.edit_buffer.id = "dual_role_npc".to_string();
        state.edit_buffer.name = "Dual Role NPC".to_string();
        state.edit_buffer.portrait_id = "dual".to_string();
        state.edit_buffer.is_merchant = true;
        state.edit_buffer.is_trainer = true;

        // Create merchant dialogue first.
        let merchant_result = state.create_or_repair_merchant_dialogue_for_buffer();
        assert!(merchant_result.is_ok(), "merchant create must succeed");
        let merchant_dialogue_id = state.edit_buffer.dialogue_id.clone();

        // Now create trainer dialogue (should create a separate tree).
        state.edit_buffer.dialogue_id = String::new();
        let trainer_result = state.create_or_repair_trainer_dialogue_for_buffer();
        assert!(trainer_result.is_ok(), "trainer create must succeed");
        let trainer_dialogue_id = state.edit_buffer.dialogue_id.clone();

        // Each operation must produce a distinct dialogue.
        assert_ne!(
            merchant_dialogue_id, trainer_dialogue_id,
            "merchant and trainer dialogues must be distinct"
        );
        assert_eq!(
            state.available_dialogues.len(),
            2,
            "two separate dialogues must exist (one merchant, one trainer)"
        );
    }
}
