# Standardizing Campaign Builder Left Panel Lists

This plan standardizes the UI presentation of left panel lists across all editors in the Campaign Builder SDK. It introduces a reusable `StandardListItem` component in `ui_helpers.rs` to ensure consistency in labels, rich metadata badges, and context menu actions (Edit, Delete, Duplicate).

## Proposed Changes

### [SDK] ui_helpers

#### [MODIFY] [ui_helpers.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/ui_helpers.rs)
- Introduce `StandardListItem` struct and implementation.
- This struct will use the builder pattern or a simple `show` method to render:
    - Primary `selectable_label`.
    - Indented `horizontal` layout for `RichText` badges.
    - Attached `context_menu` for common actions.
- It will handle selection logic via a passed-in mutable state or closure.

### [SDK] Editors

Each of the following editors will be refactored to use `StandardListItem` in their respective `show_list` or equivalent methods.

#### [MODIFY] [classes_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/classes_editor.rs)
- Standardize list item: Class Name + [Hit Die] [Proficiency Count] | ID.
- Move "Edit", "Delete", "Duplicate" actions to context menu.

#### [MODIFY] [items_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/items_editor.rs)
- Convert appended icons ("✨", "💀", "📜") into styled badges: [Magic] [Cursed] [Quest].
- Add ID display in metadata line.
- Implement context menu for item actions.

#### [MODIFY] [monsters_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/monsters_editor.rs)
- Standardize list item: Monster Name + [HP] [AC] [CR] | ID.
- Implement context menu for item actions.

#### [MODIFY] [conditions_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/conditions_editor.rs)
- Standardize list item: Condition Name + [Effect Type Badge] | ID.
- Implement context menu for item actions.

#### [MODIFY] [spells_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/spells_editor.rs)
- Standardize list item: Spell Name + [School] [Level] | ID.
- Implement context menu for item actions.

#### [MODIFY] [map_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/map_editor.rs)
- Standardize list item: Map Name + [Size] [Outdoor/Indoor] | ID.
- Implement context menu for item actions.

#### [MODIFY] [quest_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/quest_editor.rs)
- Standardize list item: Quest Name + [Main/Side] [Repeatable] [Lv Range] | ID.
- Implement context menu for item actions.

#### [MODIFY] [dialogue_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/dialogue_editor.rs)
- Update to use `StandardListItem` helper for long-term consistency, keeping existing rich metadata badges.

## Verification Plan

### Automated Tests
- Run `cargo check` to ensure no breaking changes in types or signatures.
- Run `cargo test` in `sdk/campaign_builder` to verify that refactored components still integrate correctly.

### Manual Verification
- Open each editor in the Campaign Builder UI.
- Verify that the left panel list items look consistent (Title bold/selected, Metadata indented and styled).
- Right-click items to ensure "Edit", "Delete", and "Duplicate" context menus work as expected.
- Search and filter to ensure the list still updates correctly with the new component.
