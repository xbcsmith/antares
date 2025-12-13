# SDK Updates

## Icons for Editor Toolbars

I want to add a logo to the SDK Editors bottom section where it says Antares Campaign Builder. Files are in `data/assets/icons`.

## Metadata Preview

I want to add a graphic to the Campaign Metadata preview like a cover of a module.

## Map Editor Events

I am manually testing the Events in Map Editor and I think we have a lot of gaps for a good UX experience. I will try to list them.

### Icons For Maps

NPCs Events[Encounter, Treasure, Teleport, Trap, Sign, NPC Dialog] All need separate icons. They currently are all the same color and impossible to distinguish. The problem is I would expect that more than one event can occupy the same tile on the map.

### General

I want some type of hinting system for entering values. For example, if I am entering a number, I want to see what the valid range is. If I am entering a string, I want to see if there are any restrictions on the string (like length or allowed characters). Example when entering a group of monsters, I want to see what monsters are available to choose from. I don't want a bunch of labels I want a dropdown or a searchable list.

### Trap Events

- Trap Effects should be conditions. So we should look up the condition list and link the condition to the effect.
