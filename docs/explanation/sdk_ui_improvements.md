# SDK UI Improvements

## Implementation Plan - Quest Editor Enhancements

This plan outlines the updates to the Antares SDK Quest Editor that must be made to improve quality of life.

## General

- All tabs should have a load from file button that lets the user select a file outside of hte campaign directory. Items, Monsters, Maps, Quests, and Classes should all have a load from file button.
- All tabs should have a save to file button that saves the file outside of the campaign directory. Items, Monsters, Maps, Quests, and Classes should all have a save to file button.
- All tabs should have a save button that saves the file to the campaign directory. Items, Monsters, Maps, Quests, and Classes should all have a save button.
- All ScrollArea boxes should expand to fill the available space in the window.

## Classes Tab

Should prepopulate with the classes in the campaign directory.
Should have a description of the class.
Should have a way to create the starting equipment for the class populated from the campaign items data.

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


### Verification Plan

Write Automated Tests did not happen as there are numerous issues with the UI.  


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