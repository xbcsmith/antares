# How to Update Your Campaign with Procedural Mesh Creatures

## Overview

This guide shows you how to add procedural mesh creature visual definitions to an existing Antares campaign. Creature visuals are stored in `data/creatures.ron` and can be linked to monsters in your campaign.

## Prerequisites

- Existing campaign directory with `campaign.ron`
- Basic understanding of RON (Rusty Object Notation) file format
- Campaign Builder SDK installed (optional, for visual editing)

## Step 1: Create the Creatures Data File

### 1.1 Create the Data Directory (if it doesn't exist)

```bash
cd your-campaign-directory
mkdir -p data
```

### 1.2 Create `data/creatures.ron`

Create a new file at `data/creatures.ron` with this basic structure:

```ron
// SPDX-FileCopyrightText: 2025 Your Name <your@email.com>
// SPDX-License-Identifier: Apache-2.0

// Creature visual definitions for My Campaign

[
    // Creatures will be added here
]
```

## Step 2: Add Creature Definitions

### 2.1 Using a Template (Recommended)

Copy a template from the library and customize it:

```bash
# From the antares root directory
cp data/creature_templates/humanoid.ron your-campaign/data/my_creature.ron
```

Edit `my_creature.ron` to customize:
- Change `id` to a unique number (1000+ recommended for custom creatures)
- Change `name` to your creature's name
- Adjust `color` values for meshes
- Modify `scale` to resize the creature

Example customization:

```ron
CreatureDefinition(
    id: 2001,
    name: "Blue Guard",
    meshes: [
        MeshDefinition(
            vertices: [/* ... */],
            indices: [/* ... */],
            normals: None,
            uvs: None,
            color: [0.2, 0.3, 0.8, 1.0], // Changed to blue
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        ),
        // ... more meshes
    ],
    mesh_transforms: [/* ... */],
    scale: 1.2, // 20% larger
    color_tint: None,
)
```

### 2.2 Copy the Definition to creatures.ron

Copy the entire `CreatureDefinition(...)` block into your `data/creatures.ron` array:

```ron
[
    CreatureDefinition(
        id: 2001,
        name: "Blue Guard",
        // ... full definition
    ),
    // Add more creatures here
]
```

### 2.3 Create Additional Creatures

Add more creatures to the array, ensuring each has a **unique ID**:

```ron
[
    CreatureDefinition(
        id: 2001,
        name: "Blue Guard",
        // ...
    ),
    CreatureDefinition(
        id: 2002,
        name: "Red Dragon",
        // ...
    ),
    CreatureDefinition(
        id: 2003,
        name: "Green Goblin",
        // ...
    ),
]
```

## Step 3: Update Campaign Metadata

### 3.1 Edit `campaign.ron`

Open your campaign's `campaign.ron` file and ensure it includes the creatures file reference:

```ron
(
    name: "My Campaign",
    version: "1.0.0",
    author: "Your Name",
    description: "Campaign description",

    // ... other fields

    // Add this line if not present:
    creatures_file: "data/creatures.ron",
)
```

**Note**: If `creatures_file` is not specified, the default path `"data/creatures.ron"` is used automatically.

## Step 4: Link Creatures to Monsters

### 4.1 Update Monster Definitions

In your `data/monsters.ron` file, link monsters to creature visuals using the `visual_id` field:

```ron
[
    MonsterDefinition(
        id: 1,
        name: "Guard",
        // ... stats, attacks, etc.

        // Link to creature visual ID 2001
        visual_id: Some(2001),
    ),
    MonsterDefinition(
        id: 2,
        name: "Dragon",
        // ... stats, attacks, etc.

        // Link to creature visual ID 2002
        visual_id: Some(2002),
    ),
]
```

### 4.2 Visual ID Matching

Ensure the `visual_id` in your monster definition matches a creature `id` in `creatures.ron`:

- Monster `visual_id: Some(2001)` → Creature `id: 2001`
- Monster `visual_id: Some(2002)` → Creature `id: 2002`

## Step 5: Validate Your Campaign

### 5.1 Using Campaign Builder (Recommended)

```bash
# From antares root directory
cargo run --bin campaign_builder
```

1. Open your campaign: File → Open Campaign
2. Navigate to Creatures tab
3. Check for validation errors (red messages)
4. Fix any issues reported

### 5.2 Using SDK Validation Tool

```bash
cargo run --bin validate_campaign -- --path /path/to/your-campaign
```

### 5.3 Common Validation Errors

**Error**: "Creature not found: 2001"
- **Fix**: Ensure creature with ID 2001 exists in `creatures.ron`

**Error**: "Duplicate creature ID: 2001"
- **Fix**: Each creature must have a unique ID

**Error**: "Creature has no meshes"
- **Fix**: Add at least one mesh to the `meshes` array

**Error**: "Invalid scale: 0.0"
- **Fix**: Scale must be greater than 0.0 (use 1.0 for normal size)

**Error**: "Mesh index out of bounds"
- **Fix**: Ensure `mesh_transforms` array has same length as `meshes` array

## Step 6: Test in Game

### 6.1 Load Campaign

Start the game and load your campaign:

```bash
cargo run --bin antares -- --campaign /path/to/your-campaign
```

### 6.2 Verify Creatures Render

- Enter a map with monsters
- Verify monsters display with correct creature visuals
- Check colors, sizes, and shapes match expectations

### 6.3 Debug Rendering Issues

If creatures don't appear:

1. Check console for error messages
2. Verify `visual_id` is set in monster definition
3. Ensure creature ID exists in `creatures.ron`
4. Check that meshes have valid geometry (non-empty vertices/indices)

## Step 7: Create Variations (Optional)

### 7.1 Color Variations

Create variants by copying a creature and changing colors:

```ron
[
    // Base creature
    CreatureDefinition(
        id: 2001,
        name: "Dragon (Red)",
        meshes: [
            MeshDefinition(
                // ...
                color: [1.0, 0.0, 0.0, 1.0], // Red
            ),
        ],
        // ...
    ),
    // Color variation
    CreatureDefinition(
        id: 2002,
        name: "Dragon (Blue)",
        meshes: [
            MeshDefinition(
                // ...
                color: [0.0, 0.0, 1.0, 1.0], // Blue
            ),
        ],
        // ...
    ),
]
```

### 7.2 Size Variations

Create size variants by changing the `scale` field:

```ron
CreatureDefinition(
    id: 2003,
    name: "Dragon (Ancient)",
    // ... same meshes as base dragon
    scale: 2.0, // Double size
    // ...
)
```

## Troubleshooting

### Problem: Campaign won't load

**Solution**:
1. Check `campaign.ron` syntax (matching parentheses, commas)
2. Verify `creatures_file` path is correct
3. Ensure `data/creatures.ron` exists

### Problem: Creatures file won't parse

**Solution**:
1. Validate RON syntax (all brackets/parentheses balanced)
2. Check for missing commas between array elements
3. Ensure all fields are present (vertices, indices, color, etc.)
4. Remove trailing commas after last array element

### Problem: Creature appears as white cube

**Solution**:
1. Check that `vertices` and `indices` are not empty
2. Verify indices reference valid vertex positions
3. Ensure `color` is set (default is white if missing)
4. Check console for mesh validation warnings

### Problem: Monster has no visual

**Solution**:
1. Verify `visual_id` is set: `visual_id: Some(2001)`
2. Check that creature ID 2001 exists in `creatures.ron`
3. Ensure campaign loaded creatures file (check console logs)

### Problem: Duplicate ID error

**Solution**:
1. Search `creatures.ron` for duplicate `id` values
2. Assign unique IDs to each creature (2000+, 3000+, etc.)
3. Update monster `visual_id` references if IDs changed

## Best Practices

### ID Numbering Scheme

Use ID ranges to organize creatures:

- **1000-1999**: Template-based creatures
- **2000-2999**: Humanoid creatures
- **3000-3999**: Beast/animal creatures
- **4000-4999**: Dragon/flying creatures
- **5000-5999**: Undead/monster creatures
- **6000+**: Special/boss creatures

### File Organization

For large campaigns, split creatures into multiple files:

```ron
// In campaign.ron
creatures_file: "data/creatures.ron",
```

```ron
// In data/creatures.ron - use #include (if supported) or combine manually
[
    // Or load all creatures here
    CreatureDefinition(/* ... */),
    CreatureDefinition(/* ... */),
]
```

### Version Control

Commit creatures separately for easier tracking:

```bash
git add data/creatures.ron
git commit -m "Add Blue Guard creature (ID: 2001)"
```

### Documentation

Add comments to your `creatures.ron` file:

```ron
[
    // === GUARDS (2000-2099) ===
    CreatureDefinition(
        id: 2001,
        name: "Blue Guard",
        // Blue-armored humanoid guard
        // ...
    ),

    // === DRAGONS (4000-4099) ===
    CreatureDefinition(
        id: 4001,
        name: "Red Dragon",
        // Large fire-breathing dragon
        // ...
    ),
]
```

## Next Steps

- **Learn creature variations**: See `docs/how-to/create_creature_variations.md`
- **Add LOD levels**: See `docs/how-to/optimize_creatures_with_lod.md`
- **Create animations**: See `docs/how-to/animate_creatures.md`
- **Use Campaign Builder**: See `docs/how-to/use_creature_editor.md`

## Reference

- **Creature Templates**: `data/creature_templates/`
- **Template Reference**: `docs/reference/creature_templates.md`
- **RON Format Guide**: `docs/reference/ron_format.md`
- **Validation Errors**: `docs/reference/validation_errors.md`
