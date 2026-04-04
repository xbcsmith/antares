// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Focused state structs extracted from [`super::CampaignBuilderApp`].
//!
//! Grouping the ~78 fields of `CampaignBuilderApp` into four cohesive structs
//! reduces cognitive load and makes it easier to reason about which parts of
//! the application are affected by any given piece of code.
//!
//! | Struct            | Responsibility                                           |
//! |-------------------|----------------------------------------------------------|
//! | [`CampaignData`]  | All loaded game-content data vectors                     |
//! | [`EditorRegistry`]| All sub-editor instances and their transient buffers     |
//! | [`EditorUiState`] | Visibility flags, tab selection, and UI configuration    |
//! | [`ValidationState`]| Validation results, filters, and the advanced validator |

use super::*;

// ─── CampaignData ────────────────────────────────────────────────────────────

/// All game-content data vectors loaded from the open campaign's data files.
///
/// These are the authoritative in-memory copies of every serialisable
/// collection. They are loaded from RON files by the methods in
/// [`super::campaign_io`] and are written back to disk on every save.
#[derive(Default)]
pub struct CampaignData {
    /// Items loaded from `data/items.ron`.
    pub items: Vec<Item>,

    /// Spells loaded from `data/spells.ron`.
    pub spells: Vec<Spell>,

    /// Monster definitions loaded from `data/monsters.ron`.
    pub monsters: Vec<MonsterDefinition>,

    /// Condition definitions loaded from `data/conditions.ron`.
    pub conditions: Vec<ConditionDefinition>,

    /// Furniture definitions loaded from `data/furniture.ron`.
    pub furniture_definitions: Vec<antares::domain::FurnitureDefinition>,

    /// Maps loaded from `data/maps/`.
    pub maps: Vec<Map>,

    /// Quests loaded from `data/quests.ron`.
    pub quests: Vec<Quest>,

    /// Dialogue trees loaded from `data/dialogues.ron`.
    pub dialogues: Vec<DialogueTree>,

    /// Stock templates loaded from `data/npc_stock_templates.ron`.
    pub stock_templates: Vec<MerchantStockTemplate>,

    /// Proficiency definitions loaded from the proficiencies file.
    pub proficiencies: Vec<ProficiencyDefinition>,

    /// Creature definitions loaded from `data/creatures.ron`.
    pub creatures: Vec<antares::domain::visual::CreatureDefinition>,
}

// ─── EditorRegistry ──────────────────────────────────────────────────────────

/// All sub-editor instances and their associated transient state.
///
/// Each editor owns its search buffers, selection indices, mode flags, and
/// in-progress edit buffers. The data the editors display lives in
/// [`CampaignData`]; the editors hold only the *view* state.
pub struct EditorRegistry {
    /// Campaign metadata editor (edits [`super::CampaignMetadata`]).
    pub campaign_editor_state: campaign_editor::CampaignMetadataEditorState,

    /// Tool configuration / key-bindings editor.
    pub config_editor_state: config_editor::ConfigEditorState,

    /// Items editor state.
    pub items_editor_state: ItemsEditorState,

    /// Spells editor state.
    pub spells_editor_state: SpellsEditorState,

    /// Proficiencies editor state.
    pub proficiencies_editor_state: proficiencies_editor::ProficienciesEditorState,

    /// Monsters editor state.
    pub monsters_editor_state: MonstersEditorState,

    /// Creatures editor state.
    pub creatures_editor_state: creatures_editor::CreaturesEditorState,

    /// Conditions editor state.
    pub conditions_editor_state: ConditionsEditorState,

    /// Furniture editor state.
    pub furniture_editor_state: furniture_editor::FurnitureEditorState,

    /// Maps editor state.
    pub maps_editor_state: MapsEditorState,

    /// Quest editor state.
    pub quest_editor_state: QuestEditorState,

    /// Dialogue editor state.
    pub dialogue_editor_state: DialogueEditorState,

    /// NPC editor state.
    pub npc_editor_state: npc_editor::NpcEditorState,

    /// Stock-templates editor state.
    pub stock_templates_editor_state: StockTemplatesEditorState,

    /// Classes editor state.
    pub classes_editor_state: classes_editor::ClassesEditorState,

    /// Races editor state.
    pub races_editor_state: races_editor::RacesEditorState,

    /// Characters editor state.
    pub characters_editor_state: characters_editor::CharactersEditorState,

    // ── Quest editor transient buffers ──────────────────────────────────────
    /// Search / filter text for the quest list (not yet wired to a dedicated
    /// `QuestEditorState` field; kept here for future use).
    pub _quests_search_filter: String,
    /// Whether the quest preview panel is expanded.
    pub _quests_show_preview: bool,
    /// Clipboard buffer for quest import/export.
    pub _quests_import_buffer: String,
    /// Whether the quest import dialog is visible.
    pub _quests_show_import_dialog: bool,

    // ── Stock templates transient state ─────────────────────────────────────
    /// Filename for `npc_stock_templates.ron` relative to the campaign data dir.
    pub _stock_templates_file: String,
}

impl Default for EditorRegistry {
    fn default() -> Self {
        Self {
            campaign_editor_state: campaign_editor::CampaignMetadataEditorState::new(),
            config_editor_state: config_editor::ConfigEditorState::new(),
            items_editor_state: ItemsEditorState::new(),
            spells_editor_state: SpellsEditorState::new(),
            proficiencies_editor_state: proficiencies_editor::ProficienciesEditorState::new(),
            monsters_editor_state: MonstersEditorState::new(),
            creatures_editor_state: creatures_editor::CreaturesEditorState::new(),
            conditions_editor_state: ConditionsEditorState::new(),
            furniture_editor_state: furniture_editor::FurnitureEditorState::new(),
            maps_editor_state: MapsEditorState::new(),
            quest_editor_state: QuestEditorState::default(),
            dialogue_editor_state: DialogueEditorState::default(),
            npc_editor_state: npc_editor::NpcEditorState::default(),
            stock_templates_editor_state: StockTemplatesEditorState::default(),
            classes_editor_state: classes_editor::ClassesEditorState::default(),
            races_editor_state: races_editor::RacesEditorState::default(),
            characters_editor_state: characters_editor::CharactersEditorState::default(),
            _quests_search_filter: String::new(),
            _quests_show_preview: true,
            _quests_import_buffer: String::new(),
            _quests_show_import_dialog: false,
            _stock_templates_file: "data/npc_stock_templates.ron".to_string(),
        }
    }
}

// ─── EditorUiState ───────────────────────────────────────────────────────────

/// Transient UI state: which tab is active, what dialogs are open, and
/// other display-only flags that do not belong in the campaign data or
/// in the per-editor state structs.
pub struct EditorUiState {
    /// Currently selected top-level editor tab.
    pub active_tab: EditorTab,

    /// One-line status bar message shown in the footer.
    pub status_message: String,

    /// Cached file-tree nodes for the campaign directory panel.
    pub file_tree: Vec<FileNode>,

    /// Whether the "About" dialog is visible.
    pub show_about_dialog: bool,

    /// Whether the "unsaved changes" warning dialog is visible.
    pub show_unsaved_warning: bool,

    /// Whether the asset manager panel is open.
    pub show_asset_manager: bool,

    /// Filter for the asset manager; `None` means show all types.
    pub asset_type_filter: Option<asset_manager::AssetType>,

    /// Whether newly-loaded data is merged into existing data (`true`) or
    /// replaces it (`false`).
    pub file_load_merge_mode: bool,

    /// Whether the preferences dialog is visible.
    pub show_preferences: bool,

    /// Whether the developer debug panel is visible.
    pub show_debug_panel: bool,

    /// Minimum log level shown in the debug panel.
    pub debug_panel_filter_level: LogLevel,

    /// Whether the debug panel auto-scrolls to the latest log entry.
    pub debug_panel_auto_scroll: bool,

    /// Whether the template browser overlay is visible.
    pub show_template_browser: bool,

    /// Currently selected template category in the template browser.
    pub template_category: templates::TemplateCategory,

    /// Whether the creature template browser overlay is visible.
    pub show_creature_template_browser: bool,

    /// Whether the balance stats dialog is visible.
    pub show_balance_stats: bool,

    /// Whether the cleanup candidates panel is visible.
    pub show_cleanup_candidates: bool,

    /// Set of asset paths selected for cleanup.
    pub cleanup_candidates_selected: std::collections::HashSet<PathBuf>,
}

impl Default for EditorUiState {
    fn default() -> Self {
        Self {
            active_tab: EditorTab::Metadata,
            status_message: String::new(),
            file_tree: Vec::new(),
            show_about_dialog: false,
            show_unsaved_warning: false,
            show_asset_manager: false,
            asset_type_filter: None,
            file_load_merge_mode: true,
            show_preferences: false,
            show_debug_panel: false,
            debug_panel_filter_level: LogLevel::Info,
            debug_panel_auto_scroll: true,
            show_template_browser: false,
            template_category: templates::TemplateCategory::Item,
            show_creature_template_browser: false,
            show_balance_stats: false,
            show_cleanup_candidates: false,
            cleanup_candidates_selected: std::collections::HashSet::new(),
        }
    }
}

// ─── ValidationState ─────────────────────────────────────────────────────────

/// Validation results, filter settings, and the optional advanced validator.
///
/// Keeping these together makes it easy to reset, pass around, or inspect the
/// entire validation subsystem without touching editor or data state.
pub struct ValidationState {
    /// All validation results from the most recent `validate_campaign()` run.
    pub validation_errors: Vec<validation::ValidationResult>,

    /// Active severity filter applied to the validation panel display.
    pub validation_filter: ValidationFilter,

    /// If set, the asset manager focuses this path after the next render pass.
    pub validation_focus_asset: Option<PathBuf>,

    /// Optional advanced validator populated after `run_advanced_validation()`.
    pub advanced_validator: Option<advanced_validation::AdvancedValidator>,

    /// Whether the validation report overlay window is visible.
    pub show_validation_report: bool,

    /// Rendered Markdown text of the most recent advanced validation report.
    pub validation_report: String,
}

impl Default for ValidationState {
    fn default() -> Self {
        Self {
            validation_errors: Vec::new(),
            validation_filter: ValidationFilter::All,
            validation_focus_asset: None,
            advanced_validator: None,
            show_validation_report: false,
            validation_report: String::new(),
        }
    }
}
