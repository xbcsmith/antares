# How to Use the Creatures Editor

This guide explains how to use the Campaign Builder's Creatures Editor to manage procedural mesh creature definitions for your campaign.

## Table of Contents

- [Overview](#overview)
- [Getting Started](#getting-started)
- [Creating a New Creature](#creating-a-new-creature)
- [Editing Existing Creatures](#editing-existing-creatures)
- [Deleting Creatures](#deleting-creatures)
- [Understanding Creature ID Ranges](#understanding-creature-id-ranges)
- [Validation and Error Handling](#validation-and-error-handling)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

The Creatures Editor is a visual tool in the Campaign Builder that helps you manage creature visual definitions for your campaign. Creatures are procedurally-generated 3D mesh representations used for monsters, NPCs, and other entities in the game world.

### What You Can Do

- View all creatures registered in your campaign
- Add new creature references
- Edit existing creature properties
- Delete creature references
- Validate creature definitions
- Organize creatures by category (Monsters, NPCs, Templates, Variants)

## Getting Started

### Opening the Creatures Editor

1. Launch the Campaign Builder application
2. Open your campaign (or create a new one)
3. Navigate to the **Creatures** tab in the main editor window

### Understanding the Interface

The Creatures Editor has three main views:

1. **List View** - Browse all registered creatures
2. **Add View** - Create new creature references
3. **Edit View** - Modify existing creature properties

## Creating a New Creature

### Step 1: Access Add Mode

1. In the Creatures Editor, click the **Add Creature** button
2. The editor switches to Add View

### Step 2: Choose a Creature ID

The editor will suggest the next available ID based on the category you select:

- **Monsters (1-50)**: Combat enemies and hostile creatures
- **NPCs (51-100)**: Townspeople, quest givers, friendly characters
- **Templates (101-150)**: Character creation examples and starter creatures
- **Variants (151-200)**: Elite monsters, boss variants, special encounters
- **Custom (201+)**: Your own custom creatures

**Example**:
```
Category: Monsters
Suggested ID: 14 (next available in range 1-50)
```

You can accept the suggestion or enter a different ID within the valid range.

### Step 3: Enter Creature Details

Fill in the required fields:

- **ID**: Unique identifier (must be unique across all creatures)
- **Name**: Display name (e.g., "Goblin", "VillageElder", "RedDragon")
- **Filepath**: Relative path to the creature definition file

**Filepath Format**:
```
assets/creatures/[creature_name].ron
```

**Example**:
```
ID: 14
Name: Troll
Filepath: assets/creatures/troll.ron
```

### Step 4: Save the Creature

1. Click **Save** to add the creature to the registry
2. The editor validates your input:
   - ID must be unique (no duplicates)
   - ID must be within valid range for category
   - Name must be non-empty
   - Filepath must be valid format
3. If validation passes, the creature is added to `data/creatures.ron`

### Step 5: Create the Creature Definition File

The editor adds the *reference* to the registry but doesn't create the actual creature file. You need to create it separately:

1. Navigate to `campaigns/[your_campaign]/assets/creatures/`
2. Create a new file: `troll.ron` (matching the filepath you specified)
3. Define the creature's meshes, transforms, and properties

See `campaigns/tutorial/assets/creatures/goblin.ron` for an example format.

## Editing Existing Creatures

### Step 1: Select a Creature

1. In List View, browse or search for the creature you want to edit
2. Click on the creature to select it
3. Click the **Edit** button

### Step 2: Modify Properties

You can change:

- **Name**: Update the display name
- **Filepath**: Change the reference to a different creature file

**Note**: You cannot change the ID of an existing creature. If you need a different ID, delete the creature and create a new one.

### Step 3: Save Changes

1. Click **Save** to apply your changes
2. The editor validates your changes
3. If validation passes, `data/creatures.ron` is updated

**Warning**: If you change the filepath, make sure the new file exists or the creature won't load properly.

## Deleting Creatures

### When to Delete

Delete a creature reference when:

- You no longer need it in your campaign
- You're removing unused content
- You're reorganizing your creature library

**Important**: Deleting a creature reference does NOT delete the actual `.ron` file. It only removes the entry from the registry.

### How to Delete

1. In List View, select the creature you want to delete
2. Click the **Delete** button
3. Confirm the deletion when prompted
4. The creature is removed from `data/creatures.ron`

### Check Before Deleting

Before deleting a creature, verify that it's not being used by:

- Monsters (check `data/monsters.ron` for `visual_id` references)
- NPCs (check `data/npcs.ron` for `creature_id` references)

If a creature is in use, you'll need to:
1. Remove or update those references first
2. Then delete the creature

The editor will warn you if a creature is potentially in use.

## Understanding Creature ID Ranges

Creature IDs are organized into categories to make them easier to manage:

| Range     | Category  | Purpose                           | Examples              |
|-----------|-----------|-----------------------------------|-----------------------|
| 1-50      | Monsters  | Combat enemies, hostile creatures | Goblin, Orc, Dragon   |
| 51-100    | NPCs      | Townspeople, quest givers         | Innkeeper, Elder      |
| 101-150   | Templates | Character creation examples       | Knight, Mage, Rogue   |
| 151-200   | Variants  | Elite monsters, boss variants     | SkeletonWarrior       |
| 201+      | Custom    | Your own custom creatures         | Any custom content    |

### Why Use Ranges?

- **Organization**: Quickly identify creature types by ID
- **Collaboration**: Team members know where to add new creatures
- **Maintenance**: Easier to find and update creatures by category
- **Convention**: Follows standard RPG content organization

### Suggested ID Assignment

When adding creatures, the editor suggests IDs based on:

1. **Next available**: The lowest unused ID in the category
2. **Gap filling**: Fills gaps in the sequence before suggesting higher IDs

**Example**:
```
Existing Monster IDs: 1, 2, 3, 5, 7, 10
Suggested ID: 4 (fills gap)

After adding ID 4:
Suggested ID: 6 (next gap)
```

## Validation and Error Handling

### Automatic Validation

The editor validates creatures automatically when you:

- Add a new creature
- Edit an existing creature
- Save changes
- Load the campaign

### Validation Checks

1. **Duplicate IDs**: No two creatures can have the same ID
2. **ID Range**: IDs must follow category conventions
3. **File Existence**: Referenced creature files should exist
4. **Format**: Filepaths must be valid relative paths
5. **Required Fields**: Name and filepath must be non-empty

### Validation Results

The editor shows validation results with color-coded indicators:

- ✅ **Green (Valid)**: Creature is valid and ready to use
- ⚠️ **Yellow (Warning)**: Creature has warnings but will work
- ❌ **Red (Error)**: Creature has errors and must be fixed

### Common Errors and Fixes

#### Error: Duplicate ID

```
Error: Duplicate creature ID: 42
```

**Fix**: Choose a different, unused ID for your creature.

#### Error: ID Out of Range

```
Error: Creature ID 75 is outside valid range for category Monsters (1-50)
```

**Fix**: Use an ID within the correct range or change the category.

#### Warning: File Not Found

```
Warning: Creature file not found: assets/creatures/missing.ron
```

**Fix**: Create the creature definition file at the specified path.

#### Error: Invalid Filepath

```
Error: Invalid filepath format: "creature.ron" (must be relative path)
```

**Fix**: Use format `assets/creatures/creature_name.ron`

## Best Practices

### Naming Conventions

- Use **PascalCase** for creature names: `GiantRat`, `VillageElder`, `RedDragon`
- Keep names descriptive but concise
- Avoid spaces and special characters
- Match the name to the filename (lowercase with underscores)

**Example**:
```
Name: RedDragon
Filepath: assets/creatures/red_dragon.ron
```

### Organize by Category

- Keep Monsters in 1-50 range
- Keep NPCs in 51-100 range
- Use Templates for reusable base creatures
- Use Variants for specialized versions

### File Organization

Create a logical directory structure:

```
campaigns/my_campaign/
├── data/
│   └── creatures.ron          ← Registry (editor manages this)
└── assets/
    └── creatures/
        ├── monsters/
        │   ├── goblin.ron
        │   ├── orc.ron
        │   └── dragon.ron
        ├── npcs/
        │   ├── innkeeper.ron
        │   └── village_elder.ron
        └── templates/
            ├── knight.ron
            └── mage.ron
```

Update filepaths accordingly:
```
assets/creatures/monsters/goblin.ron
assets/creatures/npcs/innkeeper.ron
```

### Reuse Creatures

Multiple monsters or NPCs can share the same creature visual:

**Example**:
```
Monster ID 1 (Goblin Scout) → visual_id: 1 → Creature ID 1 (Goblin)
Monster ID 2 (Goblin Warrior) → visual_id: 1 → Creature ID 1 (Goblin)
Monster ID 3 (Goblin Shaman) → visual_id: 1 → Creature ID 1 (Goblin)
```

All three monsters use the same Goblin creature visual but have different stats.

### Version Control

The `data/creatures.ron` file is text-based (RON format) and works well with version control:

- Commit changes with descriptive messages
- Review diffs before merging
- Keep backups before major changes

## Troubleshooting

### Problem: Creatures Not Showing in Game

**Possible Causes**:
1. Creature file doesn't exist at the specified path
2. Creature file has syntax errors
3. Monster/NPC doesn't reference the creature ID

**Solution**:
1. Run validation in the editor
2. Check that creature files exist
3. Verify `visual_id` in monsters.ron and `creature_id` in npcs.ron

### Problem: Validation Errors Won't Clear

**Possible Causes**:
1. File permissions prevent saving
2. Creature file is corrupted
3. Multiple validation errors need fixing

**Solution**:
1. Check file permissions on `data/creatures.ron`
2. Fix all reported errors, not just the first one
3. Restart the editor and try again

### Problem: Lost Changes After Editing

**Possible Causes**:
1. Didn't click Save before switching views
2. Validation failed silently
3. File locked by another process

**Solution**:
1. Always click Save and wait for confirmation
2. Check for error messages in the editor
3. Close other programs that might lock files

### Problem: Can't Find Creature in List

**Possible Causes**:
1. Search filter is active
2. Wrong category selected
3. Creature was deleted

**Solution**:
1. Clear search filters
2. View "All Categories"
3. Check the creatures.ron file directly

## Additional Resources

### Related Documentation

- [Campaign Builder Overview](../tutorials/campaign_builder_overview.md)
- [Creating Procedural Meshes](../tutorials/creating_procedural_meshes.md)
- [Monster and NPC Integration](../explanation/monster_npc_creature_integration.md)

### Example Files

See the tutorial campaign for examples:

- `campaigns/tutorial/data/creatures.ron` - Creature registry
- `campaigns/tutorial/assets/creatures/` - Creature definition files
- `campaigns/tutorial/data/monsters.ron` - Monster visual_id references
- `campaigns/tutorial/data/npcs.ron` - NPC creature_id references

### Getting Help

If you encounter issues not covered in this guide:

1. Check the [FAQ](../reference/faq.md)
2. Review error messages in the editor console
3. Consult the [Architecture Reference](../reference/architecture.md)
4. Ask in the community forums

---

**Last Updated**: 2025-02-16
**Version**: 1.0
