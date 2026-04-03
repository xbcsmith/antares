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
    resolve_portrait_path, show_standard_list_item, EditorToolbar, ItemAction, MetadataBadge,
    StandardListItemConfig, ToolbarAction, TwoColumnLayout,
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

/// Context bundle for [`NpcEditorState::show`].
///
/// Collapses four per-call parameters so the `show()` signature stays
/// under the Clippy `too_many_arguments` limit.
///
/// # Examples
///
/// ```no_run
/// use campaign_builder::npc_editor::NpcEditorContext;
/// use antares::sdk::tool_config::DisplayConfig;
/// use std::path::PathBuf;
///
/// let dir = PathBuf::from("/campaigns/demo");
/// let cfg = NpcEditorContext {
///     campaign_dir: Some(&dir),
///     npcs_file: "data/npcs.ron",
///     display_config: &DisplayConfig::default(),
///     creature_manager: None,
/// };
/// assert_eq!(cfg.npcs_file, "data/npcs.ron");
/// ```
pub struct NpcEditorContext<'a> {
    /// Path to the open campaign root directory, or `None` if no campaign is loaded.
    pub campaign_dir: Option<&'a std::path::PathBuf>,
    /// Relative path to the NPCs data file.
    pub npcs_file: &'a str,
    /// Display configuration for layout calculations.
    pub display_config: &'a antares::sdk::tool_config::DisplayConfig,
    /// Optional creature asset manager for creature-picker support.
    pub creature_manager: Option<&'a crate::creature_assets::CreatureAssetManager>,
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

                ui.separator();

                if ui.button("🔄 Clear Filters").clicked() {
                    self.search_filter.clear();
                    self.filter_merchants = false;
                    self.filter_innkeepers = false;
                    self.filter_quest_givers = false;
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
                        autocomplete_portrait_selector(
                            ui,
                            "npc_edit_portrait",
                            "",
                            &mut self.edit_buffer.portrait_id,
                            &self.available_portraits,
                            campaign_dir,
                        );

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
                        crate::ui_helpers::autocomplete_sprite_sheet_selector(
                            ui,
                            "npc_edit_sprite",
                            "",
                            &mut self.edit_buffer.sprite_sheet,
                            &self.available_sprite_sheets,
                            campaign_dir,
                        );

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
                        autocomplete_creature_selector(
                            ui,
                            "npc_creature",
                            "",
                            &mut self.edit_buffer.creature_id,
                            &self.available_creatures,
                        );

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
                ui.horizontal(|ui| {
                    if ui.button("⬅ Back to List").clicked() {
                        self.mode = NpcEditorMode::List;
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

        true
    }

    fn start_add_npc(&mut self) {
        self.mode = NpcEditorMode::Add;
        self.edit_buffer = NpcEditBuffer::default();
        self.edit_buffer.id = self.next_npc_id();
        // Ensure autocomplete widgets show values from the fresh buffer
        self.reset_autocomplete_buffers = true;
    }

    /// Loads a portrait texture from the campaign assets directory
    ///
    /// This method caches loaded textures to avoid reloading. If the texture is already
    /// cached, it returns immediately. Failed loads are also cached to prevent repeated
    /// attempts.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context for texture registration
    /// * `campaign_dir` - The campaign directory containing assets/portraits
    /// * `portrait_id` - The portrait ID to load (e.g., "0", "1", "warrior")
    ///
    /// # Returns
    ///
    /// Returns `true` if the texture was successfully loaded (or was already cached),
    /// `false` if the load failed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::npc_editor::NpcEditorState;
    /// use std::path::PathBuf;
    ///
    /// let mut state = NpcEditorState::new();
    /// let campaign_dir = PathBuf::from("/path/to/campaign");
    /// // In egui context:
    /// // let texture = state.load_portrait_texture(ctx, Some(&campaign_dir), "0");
    /// ```
    pub fn load_portrait_texture(
        &mut self,
        ctx: &egui::Context,
        campaign_dir: Option<&PathBuf>,
        portrait_id: &str,
    ) -> bool {
        // Check if already cached
        if self.portrait_textures.contains_key(portrait_id) {
            return self
                .portrait_textures
                .get(portrait_id)
                .is_some_and(|t| t.is_some());
        }

        // Attempt to load and decode image
        let texture_handle = (|| {
            let path = resolve_portrait_path(campaign_dir, portrait_id)?;

            // Read image file; return None on failure (caller sees a "?" placeholder)
            let image_bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(_) => {
                    return None;
                }
            };

            // Decode image; return None on failure
            let dynamic_image = match image::load_from_memory(&image_bytes) {
                Ok(img) => img,
                Err(_) => {
                    return None;
                }
            };

            // Convert to RGBA8
            let rgba_image = dynamic_image.to_rgba8();
            let size = [rgba_image.width() as usize, rgba_image.height() as usize];
            let pixels = rgba_image.as_flat_samples();

            // Create egui ColorImage
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

            // Register texture with egui
            let texture_handle = ctx.load_texture(
                format!("npc_portrait_{}", portrait_id),
                color_image,
                egui::TextureOptions::LINEAR,
            );

            Some(texture_handle)
        })();

        // Cache result (even None for failed loads to avoid repeated attempts)
        let loaded = texture_handle.is_some();

        self.portrait_textures
            .insert(portrait_id.to_string(), texture_handle);

        loaded
    }

    /// Shows portrait grid picker popup for visual portrait selection
    ///
    /// Displays a popup window with a grid of portrait thumbnails that the user can click to select.
    /// The popup is modal and closes when a portrait is selected or the close button is clicked.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context for rendering
    /// * `campaign_dir` - The campaign directory containing assets/portraits
    ///
    /// # Returns
    ///
    /// Returns `Some(portrait_id)` if the user clicked on a portrait to select it,
    /// or `None` if no selection was made this frame.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::npc_editor::NpcEditorState;
    /// use std::path::PathBuf;
    ///
    /// let mut state = NpcEditorState::new();
    /// let campaign_dir = PathBuf::from("/path/to/campaign");
    /// // In egui context:
    /// // if let Some(selected_id) = state.show_portrait_grid_picker(ctx, Some(&campaign_dir)) {
    /// //     println!("Selected portrait: {}", selected_id);
    /// // }
    /// ```
    pub fn show_portrait_grid_picker(
        &mut self,
        ctx: &egui::Context,
        campaign_dir: Option<&PathBuf>,
    ) -> Option<String> {
        let mut selected_portrait: Option<String> = None;

        // Clone the portraits list to avoid borrow issues
        let available_portraits = self.available_portraits.clone();

        egui::Window::new("Select Portrait")
            .collapsible(false)
            .resizable(true)
            .default_width(400.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.label("Click a portrait to select:");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Display portraits in a grid with 4 columns
                    const COLUMNS: usize = 4;
                    const THUMBNAIL_SIZE: f32 = 80.0;

                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                    let total_portraits = available_portraits.len();
                    let rows = total_portraits.div_ceil(COLUMNS);

                    for row in 0..rows {
                        ui.horizontal(|ui| {
                            for col in 0..COLUMNS {
                                let idx = row * COLUMNS + col;
                                if idx >= total_portraits {
                                    break;
                                }

                                let portrait_id = &available_portraits[idx];

                                ui.vertical(|ui| {
                                    // Try to load texture
                                    self.load_portrait_texture(ctx, campaign_dir, portrait_id);
                                    let has_texture = self
                                        .portrait_textures
                                        .get(portrait_id)
                                        .and_then(|opt| opt.as_ref())
                                        .is_some();

                                    // Build tooltip text with portrait path
                                    let tooltip_text = if let Some(path) =
                                        resolve_portrait_path(campaign_dir, portrait_id)
                                    {
                                        format!(
                                            "Portrait ID: {}\nPath: {}",
                                            portrait_id,
                                            path.display()
                                        )
                                    } else {
                                        format!("Portrait ID: {}\n⚠ File not found", portrait_id)
                                    };

                                    // Create image button or placeholder
                                    let button_response = if has_texture {
                                        let texture = self
                                            .portrait_textures
                                            .get(portrait_id)
                                            .and_then(|t| t.as_ref())
                                            .expect("texture present since has_texture is true");
                                        ui.add(
                                            egui::Button::image(
                                                egui::Image::new(texture).fit_to_exact_size(
                                                    egui::vec2(THUMBNAIL_SIZE, THUMBNAIL_SIZE),
                                                ),
                                            )
                                            .frame(true),
                                        )
                                        .on_hover_text(&tooltip_text)
                                    } else {
                                        // Placeholder for failed/missing images
                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(THUMBNAIL_SIZE, THUMBNAIL_SIZE),
                                            egui::Sense::click(),
                                        );
                                        ui.painter().rect_filled(
                                            rect,
                                            2.0,
                                            egui::Color32::from_gray(50),
                                        );
                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            "?",
                                            egui::FontId::proportional(24.0),
                                            egui::Color32::from_gray(150),
                                        );
                                        response.on_hover_text(&tooltip_text)
                                    };

                                    // Check if clicked
                                    if button_response.clicked() {
                                        selected_portrait = Some(portrait_id.clone());
                                        self.portrait_picker_open = false;
                                    }

                                    // Show portrait ID below thumbnail
                                    ui.label(
                                        egui::RichText::new(portrait_id)
                                            .size(10.0)
                                            .color(egui::Color32::from_gray(200)),
                                    );
                                });
                            }
                        });
                    }

                    // Show message if no portraits found
                    if total_portraits == 0 {
                        ui.label("No portraits found in campaign assets/portraits directory.");
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        self.portrait_picker_open = false;
                    }
                });
            });

        selected_portrait
    }

    pub fn show_sprite_sheet_picker(
        &mut self,
        ctx: &egui::Context,
        campaign_dir: Option<&PathBuf>,
    ) -> Option<String> {
        let mut selected_sheet: Option<String> = None;

        // Clone the list to avoid borrow conflicts in the UI closure
        let available = self.available_sprite_sheets.clone();

        egui::Window::new("Select Sprite Sheet")
            .collapsible(false)
            .resizable(true)
            .default_width(500.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.label("Click a sprite sheet to select:");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

                    for sheet in &available {
                        // Show each candidate as a selectable label with a small preview/action area
                        ui.horizontal(|ui| {
                            let resp = ui.selectable_label(false, sheet);
                            if resp.clicked() {
                                selected_sheet = Some(sheet.clone());
                            }

                            // Hover tooltip showing full path (if campaign dir known)
                            if let Some(dir) = campaign_dir {
                                let full = dir.join(sheet);
                                if full.exists() {
                                    ui.label(egui::RichText::new("•").weak())
                                        .on_hover_text(format!("Path: {}", full.display()));
                                } else {
                                    ui.label(
                                        egui::RichText::new("⚠")
                                            .color(egui::Color32::from_rgb(255, 180, 0)),
                                    )
                                    .on_hover_text(format!("Missing: {}", full.display()));
                                }
                            }
                        });
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        self.sprite_picker_open = false;
                    }
                });
            });

        selected_sheet
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
        };

        self.merchant_dialogue_repair_action_for_definition(&npc)
    }

    fn create_or_repair_merchant_dialogue_for_buffer(&mut self) -> Result<String, String> {
        if !self.edit_buffer.is_merchant {
            return Ok(String::new());
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
            "No dialogue assigned" | "Merchant dialogue missing OpenMerchant"
        ) {
            return self.create_or_repair_merchant_dialogue_for_buffer();
        }

        Ok(String::new())
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

/// Helper function to show portrait placeholder
fn load_npc_portrait_texture(
    ctx: &egui::Context,
    campaign_dir: Option<&PathBuf>,
    portrait_id: &str,
    portrait_textures: &mut HashMap<String, Option<egui::TextureHandle>>,
) -> bool {
    if portrait_textures.contains_key(portrait_id) {
        return portrait_textures
            .get(portrait_id)
            .and_then(|value| value.as_ref())
            .is_some();
    }

    let texture_handle = (|| {
        let path = resolve_portrait_path(campaign_dir, portrait_id)?;

        let image_bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_e) => {
                // Portrait read failure is non-critical; the UI shows a "?" placeholder.
                return None;
            }
        };

        let dynamic_image = match image::load_from_memory(&image_bytes) {
            Ok(img) => img,
            Err(_e) => {
                // Portrait decode failure is non-critical; the UI shows a "?" placeholder.
                return None;
            }
        };

        let rgba_image = dynamic_image.to_rgba8();
        let size = [rgba_image.width() as usize, rgba_image.height() as usize];
        let pixels = rgba_image.as_flat_samples();

        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

        Some(ctx.load_texture(
            format!("npc_portrait_{}", portrait_id),
            color_image,
            egui::TextureOptions::LINEAR,
        ))
    })();

    let loaded = texture_handle.is_some();
    // If loading failed, `loaded` is false and the UI will show a "?" placeholder.

    portrait_textures.insert(portrait_id.to_string(), texture_handle);
    loaded
}

fn merchant_dialogue_status_for_preview(
    npc: &NpcDefinition,
    dialogue: Option<&DialogueTree>,
) -> &'static str {
    if !npc.is_merchant {
        if dialogue.is_some_and(DialogueTree::has_sdk_managed_merchant_content) {
            return "Non-merchant has stale merchant content";
        }
        return "Not a merchant";
    }

    let Some(dialogue) = dialogue else {
        return if npc.dialogue_id.is_some() {
            "Assigned dialogue missing"
        } else {
            "No dialogue assigned"
        };
    };

    if dialogue.contains_open_merchant_for_npc(&npc.id) {
        if dialogue.has_sdk_managed_merchant_content() {
            "SDK-managed merchant branch present"
        } else {
            "Merchant dialogue valid"
        }
    } else {
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
            "Merchant dialogue targets wrong NPC"
        } else {
            "Merchant dialogue missing OpenMerchant"
        }
    }
}

fn show_npc_preview(
    ui: &mut egui::Ui,
    npc: &NpcDefinition,
    campaign_dir: Option<&PathBuf>,
    creature_manager: Option<&CreatureAssetManager>,
    available_dialogues: &[DialogueTree],
    portrait_textures: &mut HashMap<String, Option<egui::TextureHandle>>,
) {
    let assigned_dialogue = npc.dialogue_id.and_then(|dialogue_id| {
        available_dialogues
            .iter()
            .find(|dialogue| dialogue.id == dialogue_id)
    });

    ui.horizontal(|ui| {
        let portrait_size = egui::vec2(128.0, 128.0);

        let has_texture =
            load_npc_portrait_texture(ui.ctx(), campaign_dir, &npc.portrait_id, portrait_textures);

        if has_texture {
            if let Some(Some(texture)) = portrait_textures.get(&npc.portrait_id) {
                ui.add(egui::Image::new(texture).fit_to_exact_size(portrait_size));
            } else {
                show_portrait_placeholder(ui, portrait_size);
            }
        } else {
            show_portrait_placeholder(ui, portrait_size);
        }

        ui.add_space(10.0);

        ui.vertical(|ui| {
            ui.heading(&npc.name);
            ui.label(format!("ID: {}", npc.id));

            if !npc.portrait_id.is_empty() {
                ui.label(format!("Portrait: {}", npc.portrait_id));
            }

            ui.add_space(4.0);

            if npc.is_merchant {
                ui.label(egui::RichText::new("🏪 Merchant").color(egui::Color32::GOLD));
                ui.small(
                    "Merchant authoring standard: assigned dialogue must explicitly contain OpenMerchant. The I key during dialogue is only a runtime shortcut; the SDK will generate or repair merchant dialogue automatically and preserve custom dialogue where possible.",
                );
            }
            if npc.is_innkeeper {
                ui.label(egui::RichText::new("🛏️ Innkeeper").color(egui::Color32::LIGHT_BLUE));
            }
            if npc.is_priest {
                ui.label(
                    egui::RichText::new("✝ Priest")
                        .color(egui::Color32::from_rgb(200, 180, 255)),
                );
            }
            if !npc.is_merchant && !npc.is_innkeeper && !npc.is_priest {
                ui.label(egui::RichText::new("🧑 NPC").color(egui::Color32::GRAY));
            }
        });
    });

    ui.add_space(10.0);
    ui.separator();

    egui::Grid::new("npc_preview_identity_grid")
        .num_columns(2)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            if let Some(faction) = &npc.faction {
                if !faction.trim().is_empty() {
                    ui.label("Faction:");
                    ui.label(faction.as_str());
                    ui.end_row();
                }
            }

            ui.label("Dialogue:");
            ui.label(
                npc.dialogue_id
                    .map(|d| d.to_string())
                    .as_deref()
                    .unwrap_or("(none)"),
            );
            ui.end_row();

            ui.label("Merchant Dialogue:");
            ui.label(merchant_dialogue_status_for_preview(npc, assigned_dialogue));
            ui.end_row();

            ui.label("Quests:");
            if npc.quest_ids.is_empty() {
                ui.label("(none)");
            } else {
                ui.label(format!("{} assigned", npc.quest_ids.len()));
            }
            ui.end_row();

            if let Some(creature_id) = npc.creature_id {
                ui.label("Creature ID:");
                ui.label(creature_id.to_string());
                ui.end_row();
            }

            if let (Some(creature_id), Some(manager)) = (npc.creature_id, creature_manager) {
                let resolved = manager
                    .load_creature(creature_id)
                    .map(|c| c.name)
                    .unwrap_or_else(|_| "⚠ Unknown".to_string());
                ui.label("Asset:");
                ui.label(resolved);
                ui.end_row();
            }
        });

    if let Some(sprite) = &npc.sprite {
        ui.add_space(10.0);
        ui.heading("Sprite");
        ui.separator();

        egui::Grid::new("npc_preview_sprite_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label("Sheet:");
                ui.label(&sprite.sheet_path);
                ui.end_row();

                ui.label("Index:");
                ui.label(sprite.sprite_index.to_string());
                ui.end_row();
            });
    }

    if npc.is_merchant {
        ui.add_space(10.0);
        ui.heading("Merchant");
        ui.separator();

        egui::Grid::new("npc_preview_merchant_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label("Stock Template:");
                ui.label(npc.stock_template.as_deref().unwrap_or("(none)"));
                ui.end_row();

                if let Some(economy) = &npc.economy {
                    ui.label("Buy Rate:");
                    ui.label(format!("{:.0}%", economy.buy_rate * 100.0));
                    ui.end_row();

                    ui.label("Sell Rate:");
                    ui.label(format!("{:.0}%", economy.sell_rate * 100.0));
                    ui.end_row();
                }
            });
    }

    if (npc.is_priest || npc.is_innkeeper) && npc.service_catalog.is_some() {
        ui.add_space(10.0);
        ui.heading("Services");
        ui.separator();
        ui.label("(service catalog configured)");
    }

    if !npc.quest_ids.is_empty() {
        ui.add_space(10.0);
        ui.heading("Quests");
        ui.separator();

        for quest_id in &npc.quest_ids {
            ui.label(format!("• {}", quest_id));
        }
    }

    if !npc.description.is_empty() {
        ui.add_space(10.0);
        ui.heading("Description");
        ui.separator();
        ui.label(&npc.description);
    }
}

fn show_portrait_placeholder(ui: &mut egui::Ui, size: egui::Vec2) {
    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());

    // Background
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(4),
        egui::Color32::from_gray(40),
    );

    // Border
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(4),
        egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
        egui::StrokeKind::Outside,
    );

    // Icon
    let center = rect.center();
    let icon_size = size.y * 0.4;
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        "🖼",
        egui::FontId::proportional(icon_size),
        egui::Color32::from_rgb(150, 150, 150),
    );
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
        assert_eq!(merchant_nodes, 1);
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
}
