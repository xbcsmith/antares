# How to Create Sprites for Antares RPG

This guide walks you through creating and registering custom sprite sheets for Antares RPG.

## Prerequisites

Before creating sprites, you should:

1. Read [`docs/tutorials/creating_sprites.md`](../tutorials/creating_sprites.md) - Comprehensive learning guide
2. Have a pixel art editor (LibreSprite, Aseprite, or GIMP recommended)
3. Understand the PNG-24 format with transparency
4. Know your target sprite size (typically 64x64 for tiles/actors, 32x32 for UI)

## Quick Start: Your First Sprite Sheet

### Step 1: Create Your Sprite Sheet PNG

Using your pixel art editor:

1. Create a new image with dimensions that are multiples of your tile size
   - Example: 256x256 for 4x4 grid of 64x64 sprites
2. Draw or import your sprites in a grid layout
3. Ensure transparent areas use the alpha channel (not white/magic pink)
4. Export as **PNG-24 with transparency**
5. Place in `assets/sprites/{category}/`

Categories:
- `assets/sprites/tiles/` - Walls, doors, terrain, decorations
- `assets/sprites/actors/` - NPCs, monsters, recruitable characters
- `assets/sprites/events/` - Signs, portals, treasure markers
- `assets/sprites/ui/` - UI elements, icons

### Step 2: Register in Sprite Sheet Registry

Edit `data/sprite_sheets.ron` and add your sprite sheet:

```ron
SpriteSheetConfig(
    texture_path: "assets/sprites/tiles/my_walls.png",
    tile_size: 64,
    columns: 4,
    rows: 4,
    sprites: {
        0: "stone_wall_north",
        1: "stone_wall_east",
        2: "stone_wall_south",
        3: "stone_wall_west",
        4: "brick_wall_north",
        // ... more sprites
    },
)
```

**Key fields:**
- `texture_path`: Full path to your PNG file (relative to project root)
- `tile_size`: Pixel width/height of each sprite (e.g., 64)
- `columns`: How many sprites wide your sheet is
- `rows`: How many sprites tall your sheet is
- `sprites`: Map of index → name for easy reference

### Step 3: Reference Sprites in Your Campaign

In your map or tile data, use sprite references:

```ron
// Tile with sprite
(
    id: 10,
    visual: (
        sprite: (
            sheet_path: "assets/sprites/tiles/my_walls.png",
            sprite_index: 0,  // First sprite in sheet
        ),
    ),
)

// Or use named reference (if you prefer)
// See: docs/tutorials/creating_sprites.md Part 6
```

### Step 4: Verify in Game

Launch the game with your campaign:

```bash
cargo run --bin antares
```

Navigate to your map and verify sprites render correctly.

## Common Tasks

### Adding Sprites to an Existing Sheet

If you already have sprites registered and want to add more:

1. Add new sprites to your PNG file
2. Update sprite count in registry (increase `columns` or `rows` if needed)
3. Add new index→name mappings in the `sprites` map
4. No code changes needed - registry change is sufficient

### Creating a Tile Sprite Sheet

Tiles are static environmental objects:

```ron
SpriteSheetConfig(
    texture_path: "assets/sprites/tiles/terrain.png",
    tile_size: 64,
    columns: 4,
    rows: 2,
    sprites: {
        0: "grass_1",
        1: "grass_2",
        2: "dirt_1",
        3: "dirt_2",
        4: "sand_1",
        5: "sand_2",
        6: "water_1",
        7: "water_2",
    },
)
```

PNG requirements:
- 256x128 pixels (4 columns × 2 rows of 64x64 sprites)
- Each sprite has 64×64 pixels
- Transparent background

### Creating an Actor Sprite Sheet

Actors are characters (NPCs, monsters):

```ron
SpriteSheetConfig(
    texture_path: "assets/sprites/actors/npcs_town.png",
    tile_size: 64,
    columns: 3,
    rows: 2,
    sprites: {
        0: "guard_idle",
        1: "merchant_idle",
        2: "child_idle",
        3: "guard_talking",
        4: "merchant_talking",
        5: "child_talking",
    },
)
```

Use in map NPCs:

```ron
(
    id: 50,
    npc_id: 10,
    name: "Guard Captain",
    position: (x: 5, y: 5),
    facing: North,
    // Sprite reference handled by system
)
```

### Creating Event Marker Sprites

Event markers show interactive points on the map:

```ron
SpriteSheetConfig(
    texture_path: "assets/sprites/events/landmarks.png",
    tile_size: 32,
    columns: 4,
    rows: 1,
    sprites: {
        0: "sign_board",
        1: "teleport_circle",
        2: "treasure_chest",
        3: "portal_blue",
    },
)
```

These typically use smaller tiles (32x32) since they overlay on map tiles.

### Adding Animation to Sprites

For animated sprites, reference frames in sequence:

```ron
(
    id: 11,
    visual: (
        sprite: (
            sheet_path: "assets/sprites/tiles/water.png",
            sprite_index: 0,
            animation: (
                frames: [0, 1, 2, 3],  // Frame indices
                fps: 8.0,              // Frames per second
                looping: true,         // Loop when finished
            ),
        ),
    ),
)
```

Your sprite sheet should have animation frames in sequence.

## File Organization Best Practices

### Directory Structure

```
assets/sprites/
├── tiles/
│   ├── walls.png
│   ├── doors.png
│   ├── terrain.png
│   ├── decorations.png
│   └── trees.png
├── actors/
│   ├── npcs_town.png
│   ├── npcs_dungeon.png
│   ├── monsters_basic.png
│   └── monsters_advanced.png
├── events/
│   ├── signs.png
│   ├── portals.png
│   └── treasures.png
├── ui/
│   └── icons.png
└── README.md
```

### Naming Conventions

- Use lowercase with underscores: `stone_wall.png` not `StoneWall.png`
- Group related sprites: `npcs_town.png`, `npcs_dungeon.png`
- Suffix by category: `_tiles`, `_actors`, `_events`
- Be descriptive: `doors_wooden` better than `d1.png`

### Version Control

Track PNG files but optimize size:

```bash
# Optimize PNG files (remove metadata)
optipng -o2 assets/sprites/**/*.png
# or
pngcrush -brute assets/sprites/**/*.png
```

Add to `.gitignore` (if applicable):
```
# Don't track working files
assets/sprites/*.xcf    # GIMP projects
assets/sprites/*.aseprite  # Aseprite projects
```

## Troubleshooting

### Problem: Sprite not appearing in game

**Solution:**
1. Verify PNG file exists at path in registry
2. Check `tile_size` matches your actual sprite pixel dimensions
3. Verify `columns` × `rows` equals total sprites in PNG
4. Check sprite index doesn't exceed grid size (index 0-15 for 4x4 grid)

### Problem: Sprite appears stretched or squashed

**Solution:**
1. Verify `tile_size` value matches sprite dimensions
2. Confirm PNG is actually square sprites (not rectangular)
3. Check your image editor isn't scaling the PNG

### Problem: Transparent parts show as black/white

**Solution:**
1. Export as PNG-24 with **transparency** enabled
2. Avoid magic color approach (pink or white backgrounds)
3. Use proper alpha channel in your editor
4. Test PNG in another viewer to confirm transparency works

### Problem: Registry file not loading

**Solution:**
1. Run validator: `cargo check --all-targets --all-features`
2. Check `data/sprite_sheets.ron` syntax using RON validator
3. Verify texture paths use forward slashes: `assets/sprites/...` not `assets\sprites\...`
4. Ensure commas after each SpriteSheetConfig block

## Integration with Campaign Builder

When using Campaign Builder SDK to select sprites:

1. Sprites auto-load from registry
2. Tile inspector shows available sheets
3. Click sprite to select from grid
4. Selected sprite saved to map data automatically

No additional work needed once sprites are registered.

## Performance Tips

- Keep sprite sheets under 2048×2048 pixels
- Use consistent tile sizes within category (all 64x64 or all 32x32)
- Group related sprites in one sheet (not one sprite per file)
- Optimize PNG files to reduce disk space

## Next Steps

### Learn More

- **In-Depth Guide**: See [`docs/tutorials/creating_sprites.md`](../tutorials/creating_sprites.md) for:
  - Step-by-step sprite creation in LibreSprite
  - Animation frame creation
  - Art style best practices
  - Distribution and licensing

### Related Tasks

- **Create Maps**: Use sprites in maps via `docs/how-to/creating_maps.md`
- **Browse Sprites**: See available sprites via `docs/how-to/use_sprite_browser_in_campaign_builder.md`
- **Campaign Building**: Full workflow in `docs/tutorials/creating_campaigns.md`

## Validation Checklist

Before committing sprites:

- [ ] PNG files placed in correct `assets/sprites/{category}/` directory
- [ ] PNG files are PNG-24 format with transparency (not indexed color)
- [ ] Registry entries added to `data/sprite_sheets.ron`
- [ ] `texture_path` uses forward slashes
- [ ] `tile_size` matches actual sprite dimensions
- [ ] `columns` × `rows` ≥ number of sprites in PNG
- [ ] All sprite indices in registry are valid
- [ ] `cargo check --all-targets --all-features` passes
- [ ] Sprites render correctly in game

## See Also

- **Tutorial**: [`docs/tutorials/creating_sprites.md`](../tutorials/creating_sprites.md) - Learn sprite creation
- **Campaign Builder Guide**: [`docs/how-to/use_sprite_browser_in_campaign_builder.md`](./use_sprite_browser_in_campaign_builder.md) - How to select sprites in editor
- **Architecture**: [`docs/reference/architecture.md`](../reference/architecture.md) Section 4.1 - Sprite data structures
- **Map Creation**: [`docs/how-to/creating_maps.md`](./creating_maps.md) - Use sprites in maps
