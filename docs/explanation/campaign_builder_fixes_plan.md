# Campaign Builder Fixes and Improvements Plan

## Overview
This plan addresses the reported issues in the Campaign Builder, including UI truncation, ID clashes, missing spell attributes, quest editor bugs, and asset management issues.

## Phase 1: UI Layout Fixes
**Objective**: Fix the truncation of lists and description panels for items, spells, and monsters.

- [x] **Task 1.1**: Update `show_items_list`, `show_spells_list`, and `show_monsters_list` in `sdk/campaign_builder/src/main.rs`.
    - Replace the fixed-width/unconstrained-height layout with a split-pane layout that fills the available space.
    - Use `ui.allocate_ui_with_layout` or `egui::SidePanel` within the central panel to ensure `ScrollArea` works correctly.
- [x] **Task 1.2**: Update preview panels (`show_item_preview`, etc.) to ensure they also fill the available height and scroll correctly.

## Phase 2: ID Management and Validation
**Objective**: Prevent and resolve ID clashes in data lists.

- [x] **Task 2.1**: Implement a validation check for duplicate IDs in `load_items`, `load_spells`, and `load_monsters`.
- [x] **Task 2.2**: Add a "Validate IDs" function in the Validation tab to report clashes.
- [x] **Task 2.3**: Ensure `next_available_*_id` logic is robust and considers all loaded data.

## Phase 3: Spell Data Enhancement
**Objective**: Add missing attributes to spells (damage, duration, etc.) and update the editor.

- [x] **Task 3.1**: Modify `Spell` struct in `src/domain/magic/types.rs` to add:
    - `damage: Option<DiceRoll>`
    - `duration: u16` (rounds)
    - `saving_throw: bool`
- [x] **Task 3.2**: Update `CampaignBuilderApp::show_spells_form` in `main.rs` to include editors for these new fields.
- [x] **Task 3.3**: Update `CampaignBuilderApp::show_spell_preview` to display these fields.

## Phase 4: Quest Editor Fixes
**Objective**: Fix "Add Stage" functionality and Quest editing.

- [x] **Task 4.1**: Modify `QuestEditorState::add_stage` in `sdk/campaign_builder/src/quest_editor.rs` to support adding stages to the `quest_buffer` when creating a new quest.
- [x] **Task 4.2**: Fix the "Edit" button in `show_quest_list` in `main.rs` to use the correct quest index (handling filtered lists correctly).

## Phase 5: Asset Manager Improvements
**Objective**: Fix "Unused Assets" reporting.

- [x] **Task 5.1**: Improve `scan_items_references` in `sdk/campaign_builder/src/asset_manager.rs` to use better heuristics or fuzzy matching for asset detection.
- [x] **Task 5.2**: (Optional) Add `icon_path` to `Item` struct if heuristics are insufficient, but prioritize heuristic improvement first to avoid massive schema changes if not strictly necessary.

## Execution Strategy
We will execute these phases sequentially, verifying each fix.
