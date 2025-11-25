# SDK UI Improvements

## Implementation Plan - Quest Editor Enhancements

This plan outlines the updates to the Antares SDK Quest Editor that must be made to improve quality of life.

## Quest Rewards

[PASSED] - Add edit_reward, save_reward, delete_reward methods to QuestEditorState.


## SDK UI - Quest Objectives Enhancements

Update the Quest Editor UI to use dropdowns for ID selection.

[MODIFY] - main.rs Update show_quest_objectives_editor to use egui::ComboBox for:

[FAILED] - Monster IDs: Populate from self.monsters.
[FAILED] - Item IDs: Populate from self.items.
[FAILED] - Map IDs: Populate from self.maps.
[FAILED] - NPC IDs: Populate from self.maps (iterating through NPCs in each map).

[FAILED] - Ensure the  ObjectiveEditBuffer is updated correctly when a dropdown selection changes.


## Phase 3: SDK UI - Quest Rewards Implementation

Add support for managing Quest Rewards in the UI.

[MODIFY] 
main.rs
Implement show_quest_rewards_editor method (similar to objectives editor).
Add "Rewards" section to 
show_quest_form
.
Implement the Reward Editor Modal with support for:
Experience (Integer input)
Gold (Integer input)
Items (Dropdown for Item ID + Quantity input)
Unlock Quest (Dropdown for Quest ID)
Set Flag (Text input for flag name + Checkbox for value)
Reputation (Text input for faction + Integer input for change)

## Phase 4: Verification & Polish

Verify that all dropdowns populate correctly with loaded data.
Verify that saving/loading quests works with the new fields.
Verify that cargo check passes.

### Verification Plan

Write Automated Tests

Run cargo test -p campaign_builder to verify state logic changes.

#### Problems in Manual Verification

Launch the Campaign Builder: cargo run -p campaign_builder.

Objectives:
| Objective Type | Expected Result | Worked | Problem | Solution |
|-----|-----|-----|-----|-----|
| Create New Quest | New quest is created | Yes | None |
| Auto populate the ID field | ID is auto populated | No | ID field is not auto populated | Auto populate the ID field |
| Add Quest Stages | New stages are added | No | Added 2 identical stages causing UI ID Clashes | Add one stage at a time use unique ID for each stage |
| Add Objectives | New objectives are added | No | None | Add objectives to the quest |
| Kill Monsters | Monster ID dropdown shows loaded monsters. | No | Not Available | Add Kill Monsters objective to the quest |
| Collect Items | Item ID dropdown shows loaded items. | No | Not Available | Add Collect Items objective to the quest |
| Talk to NPC | NPC dropdown shows NPCs from loaded maps. | No | Not Available | Add Talk to NPC objective to the quest |

Can not add Objectives

No way to add objectives to the quest. 

- FAILED - Add a "Kill Monsters" objective. Verify the Monster ID dropdown shows loaded monsters.
- FAILED - Add a "Collect Items" objective. Verify the Item ID dropdown shows loaded items.
- FAILED -Add a "Talk to NPC" objective. Verify the NPC dropdown shows NPCs from loaded maps.

Rewards:
- PASSED - Add a "Gold" reward. Verify it saves.
- PASSED - Add an "Item" reward. Verify the dropdown works and it saves.

Persistence:

- FAILED - Save the quest with Save Quest Button
- FAILED - Restart the application.
- FAILED - Load the quest and verify all fields are preserved.
- PASSED - Save the quest with Save Campaign Button