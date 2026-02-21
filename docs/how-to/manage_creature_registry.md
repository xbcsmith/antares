# How to Manage the Creature Registry

This guide explains how to manage creature registry entries in the Antares Campaign Builder.

## Overview

The creature registry is a centralized database that tracks all creature definitions in your campaign. Each creature has:

- **ID**: A unique numeric identifier (1-50 for Monsters, 51-100 for NPCs, etc.)
- **Name**: Display name for the creature
- **Filepath**: Relative path to the creature's `.ron` asset file

The registry management UI helps you organize, validate, and maintain creature references.

## Understanding Creature Categories

Creatures are organized into categories based on their ID ranges:

| Category  | ID Range | Purpose                        |
|-----------|----------|--------------------------------|
| Monsters  | 1-50     | Combat enemies                 |
| NPCs      | 51-100   | Non-player characters          |
| Templates | 101-150  | Reusable creature templates    |
| Variants  | 151-200  | Variations of existing types   |
| Custom    | 201+     | Campaign-specific creatures    |

## Opening the Registry Editor

1. Launch Campaign Builder
2. Open your campaign (`File > Open Campaign`)
3. Navigate to the **Creatures** tab
4. The registry list view opens by default

## Registry Overview Panel

At the top of the screen, you'll see statistics:

```
📊 32 creatures registered (15 Monsters, 8 NPCs, 6 Templates, 3 Variants)
```

This shows:
- Total creature count
- Breakdown by category

## Viewing Creature Entries

The registry list displays:

- **ID**: Color-coded by category (red=Monsters, blue=NPCs, purple=Templates, green=Variants, orange=Custom)
- **Name**: Creature display name
- **Status**: ✓ (valid) or ⚠ (warning/error)
- **Category**: Auto-detected from ID

### Filtering by Category

Use the **Category** dropdown to filter:

- **All**: Show all creatures
- **Monsters**: Show IDs 1-50
- **NPCs**: Show IDs 51-100
- **Templates**: Show IDs 101-150
- **Variants**: Show IDs 151-200
- **Custom**: Show IDs 201+

### Searching

Type in the search box to filter by:
- Creature name (case-insensitive)
- Creature ID (exact match)

### Sorting

Use the **Sort** dropdown to order by:
- **By ID**: Numeric order (default)
- **By Name**: Alphabetical order
- **By Category**: Grouped by category, then by ID

## Adding a New Creature

### Method 1: Create New Asset

1. Click **New** button
2. The editor suggests the next available ID for your selected category
3. Edit creature properties (name, meshes, etc.)
4. Click **Save** to create the asset file and add to registry

### Method 2: Import Existing File

1. Click **Import** (if available)
2. Browse to an existing `.ron` creature file
3. The system validates the ID and adds it to the registry

## Editing a Registry Entry

1. Click on a creature in the list to select it
2. Double-click or click **Edit** to open the creature editor
3. Modify creature properties
4. Click **Save** to update

> **Note**: Editing in registry mode only changes the reference metadata. To edit the actual creature asset (meshes, properties), switch to Asset Editor mode in Phase 2.

## Removing a Creature

1. Select the creature in the list
2. Click **Delete**
3. Confirm the deletion
4. The registry entry is removed (asset file may be preserved)

## Validating the Registry

### Manual Validation

Click the **🔄 Revalidate** button to check for:
- **Duplicate IDs**: Multiple creatures with the same ID
- **Missing Files**: Registry entries pointing to non-existent files
- **Invalid References**: Malformed file paths
- **Category Mismatches**: IDs outside expected range

### Validation Results

The validation panel shows:

```
⚠ 2 ID conflict(s) detected
ID 5: 2 creatures (Goblin, OldGoblin)
```

Or if all is well:

```
✓ No ID conflicts detected
```

## Resolving ID Conflicts

### Manual Resolution

1. Select the conflicting creature
2. Edit its ID to an unused value
3. Save changes
4. Revalidate

### Auto-Fix (Future Enhancement)

The **Auto-Fix IDs** button (Phase 1.3) will:
1. Detect all conflicts
2. Suggest new IDs in the correct category range
3. Apply changes automatically

## Best Practices

### ID Assignment Strategy

- **Reserve ranges**: Leave gaps for future additions
  - Monsters 1-10: Basic enemies
  - Monsters 11-20: Advanced enemies
  - Monsters 21-30: Boss creatures

- **Use consistent numbering**: Group related creatures
  - 1: Goblin Scout
  - 2: Goblin Warrior
  - 3: Goblin Shaman

### Naming Conventions

- Use descriptive names: `"Goblin Warrior"` not `"Goblin2"`
- Include type for NPCs: `"Innkeeper - Old Town"`
- Mark templates clearly: `"Template: Humanoid Base"`

### File Organization

Keep creature files organized:
```
campaign/
  assets/
    creatures/
      monsters/
        goblin.ron
        orc.ron
      npcs/
        innkeeper.ron
        merchant.ron
      templates/
        humanoid_base.ron
```

## Troubleshooting

### "Duplicate ID" Error

**Problem**: Two creatures have the same ID.

**Solution**:
1. Open validation panel
2. Identify conflicting creatures
3. Change one creature's ID to an unused value
4. Revalidate

### "ID Out of Range" Warning

**Problem**: Creature ID doesn't match its category.

**Example**: Monster with ID 75 (should be 1-50)

**Solution**:
1. Decide correct category
2. Change ID to match range
3. Update creature type if needed

### "Missing File" Error

**Problem**: Registry references a file that doesn't exist.

**Solution**:
1. Check if file was moved/deleted
2. Update filepath in registry
3. Or remove registry entry if creature is obsolete

## Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| New Creature | `Ctrl+N` (if implemented) |
| Search | `Ctrl+F` (if implemented) |
| Delete Selected | `Delete` (if implemented) |
| Refresh/Revalidate | `F5` (if implemented) |

## Next Steps

- **Phase 2**: Learn to edit creature assets (meshes, materials)
- **Phase 3**: Apply templates to create creatures faster
- **Phase 4**: Advanced mesh editing tools
- **Phase 5**: Workflow integration and preview features

## Related Documentation

- [Creature Editor Architecture](../reference/architecture.md#creature-system)
- [Creature Asset Format](../reference/creature_ron_format.md)
- [Campaign Builder Overview](../tutorials/getting_started.md)

## Common Workflows

### Creating a New Monster Pack

1. Filter by **Monsters** category
2. Check next available ID (e.g., ID 15)
3. Click **New** for each monster
4. Assign sequential IDs: 15, 16, 17...
5. Name consistently: "Forest Wolf", "Forest Bear", "Forest Dragon"
6. Save all and validate

### Migrating Creatures from Another Campaign

1. Copy `.ron` files to your campaign's `assets/creatures/` folder
2. Click **Import** for each file
3. System assigns new IDs to avoid conflicts
4. Review and adjust IDs to match your category scheme
5. Validate to ensure no conflicts

### Cleaning Up Unused Creatures

1. Review registry list
2. Identify creatures not used in maps/encounters
3. Select and delete unused entries
4. Optionally delete asset files
5. Validate to confirm clean state

## Tips

- **Start with templates**: Create base templates first, then variants
- **Leave gaps**: Don't use every ID sequentially - leave room for expansion
- **Document custom ranges**: If using Custom category (201+), document your numbering scheme
- **Validate frequently**: Run validation after bulk changes
- **Backup before auto-fix**: Always back up before using automatic ID reassignment

---

For more detailed information about creature data structures and the underlying system, see the [Architecture Documentation](../reference/architecture.md).
