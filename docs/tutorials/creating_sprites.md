# Creating Sprites for Antares

Learn how to create sprite sheets for Antares game assets, from character NPCs to environmental decorations. This tutorial covers design, technical specifications, and integration with the sprite system.

**Time Required**: 2-3 hours (including tool setup)

**Difficulty**: Intermediate

---

## What You'll Learn

- Understanding sprite sheet grid layouts
- Required technical specifications (dimensions, format)
- Tools and workflow for sprite creation
- Exporting to PNG-24 with alpha transparency
- Configuring sprites in `sprite_sheets.ron`
- Testing sprites in your campaign

---

## Prerequisites

Before starting, ensure you have:

- A graphics editor with alpha channel support (see recommendations below)
- Understanding of bitmap graphics and transparency
- Familiarity with grid-based sprite layouts
- Text editor for editing RON configuration files
- Antares Phase 3+ with sprite rendering enabled

---

## Part 1: Choosing Your Tools

### Recommended Tools

**Professional (Most Powerful)**:
- **Aseprite** - Purpose-built for sprite animation
  - Excellent for pixel art
  - Built-in grid and sprite sheet export
  - ~$20 one-time license
  - [aseprite.org](https://www.aseprite.org/)

**Free & Open Source (Best Value)**:
- **LibreSprite** - Fork of Aseprite, completely free
  - All core sprite features
  - Grid support for sprite sheets
  - Community-maintained
  - [libresprite.github.io](https://libresprite.github.io/)

- **Krita** - Powerful digital painting with layers
  - Excellent layer management
  - Professional-grade tools
  - Free and open-source
  - [krita.org](https://krita.org/)

**General Purpose (Good Enough)**:
- **GIMP** - Free general-purpose image editor
  - Grid overlay for alignment
  - Layer support
  - Requires manual sprite sheet assembly
  - [gimp.org](https://www.gimp.org/)

**Online (No Installation)**:
- **Piskel** - Browser-based pixel art
  - Simple grid-based interface
  - Quick sprite creation
  - No installation required
  - [piskelapp.com](https://www.piskelapp.com/)

### Recommended Setup

For this tutorial, we'll use **LibreSprite** (free alternative to Aseprite) as the primary recommendation. The workflow is similar in other tools.

---

## Part 2: Understanding Sprite Sheet Layout

### Grid-Based Indexing

Antares uses **row-major indexing** for sprite sheets. This means sprites are numbered left-to-right, top-to-bottom.

Example: 4×4 grid with 16 sprites

```
 0  1  2  3
 4  5  6  7
 8  9 10 11
12 13 14 15
```

**Key Point**: Sprite 0 is always top-left. Sprite 15 is always bottom-right in a 4×4 grid.

### Example: NPC Sprite Sheet Layout

For a 4×4 NPC sprite sheet (16 NPCs total):

```
Index  NPC Type
──────────────────
 0     Guard
 1     Merchant
 2     Innkeeper
 3     Blacksmith
 4     Priest
 5     Noble
 6     Peasant
 7     Child
 8     Elder
 9     Mage
10     Warrior
11     Rogue
12     Captain
13     Mayor
14     Servant
15     Beggar
```

### Sprite Sheet Dimensions Formula

```
Sheet Width  = Tile Width × Columns
Sheet Height = Tile Height × Rows

Example (NPC Sheet):
  32 pixels/sprite × 4 columns = 128 pixels wide
  48 pixels/sprite × 4 rows = 192 pixels tall
```

---

## Part 3: Creating Your First Sprite Sheet

### Step 1: Plan Your Content

**Decision**: What will you create?

Options:
- **Tile Sprites**: Walls, doors, terrain, decorations (128×256 or 128×128)
- **Actor Sprites**: NPCs, monsters, recruitables (32×48 standard)
- **Event Markers**: Signs, portals (32×64 or 128×128)

**Recommendation for First Project**: Create a small NPC sheet (4×2 grid, 8 NPCs)

### Step 2: Create New Project in LibreSprite

1. **Open LibreSprite**
2. **File → New**
3. **Set Canvas Size**:
   - For NPC sheet: 128 × 96 pixels (4 columns × 2 rows of 32×48)
   - For Tile sheet: 512 × 512 pixels (4 columns × 4 rows of 128×128)
4. **Enable Grid**: View → Grid → Show Grid
5. **Configure Grid Size**:
   - Match your sprite tile size (e.g., 32×48 for NPCs)

### Step 3: Set Up Grid for Alignment

In LibreSprite:

1. **Edit → Preferences → Grid**
2. **Set Grid Width**: Your sprite width (e.g., 32)
3. **Set Grid Height**: Your sprite height (e.g., 48)
4. **Enable "Snap to Grid"**: View → Snap to Grid

The grid will help you align sprites perfectly to the sprite sheet layout.

### Step 4: Create Individual Sprites

You have two approaches:

#### Approach A: Draw Directly on Sheet (Recommended for Learning)

1. Use the brush/pencil tool
2. Draw within grid cells one at a time
3. Reference the grid lines to stay within bounds
4. Build left-to-right, top-to-bottom

#### Approach B: Create Individual Sprites, Then Combine

1. Create each sprite in a separate file (e.g., `guard.png`, `merchant.png`)
2. Create a blank sprite sheet image
3. Copy/paste each sprite into the correct grid position
4. Use guides or grid overlays to align

**Note**: Approach B is more professional and easier to modify individual sprites later.

### Step 5: Add Transparency (Alpha Channel)

**CRITICAL**: Your sprites must have transparent backgrounds!

In LibreSprite:

1. **Layer → Transparency → Color to Alpha**
2. **Select the background color** (usually white or magenta)
3. **Click "OK"**

All selected color pixels become transparent.

**Alternative**: Use a magic wand tool to select the background, then delete.

### Step 6: Export to PNG-24

**Export Settings** (in LibreSprite):

1. **File → Export As**
2. **Set Filename**: e.g., `npcs_town.png`
3. **Set Location**: `assets/sprites/`
4. **File Type**: PNG Image (.png)
5. **Export Options**:
   - **Bit Depth**: 24-bit (RGB) or 32-bit (RGBA with alpha)
   - **Interlace**: Unchecked (not needed for game assets)
   - **Compression**: Level 6-9 (default is fine)
6. **Click "Export"**

Verify the exported file is in `assets/sprites/` directory.

---

## Part 4: Technical Specifications

### Required Format

```
Format:        PNG-24 or PNG-32 with alpha channel
Color Space:   sRGB
Bit Depth:     24-bit color + 8-bit alpha (32-bit total)
Compression:   Deflate (standard PNG compression)
Transparency:  Full alpha support required (0-255 per pixel)
```

### Sprite Size Guidelines

**Small Actors** (NPCs, Monsters, Recruitables):
- Typical size: 32×48 pixels
- Allows 4 sprites per row (128 pixels total width)
- Good for dense NPC areas (towns, taverns)

**Large Tiles** (Walls, Doors, Terrain):
- Typical size: 128×128 or 128×256 pixels
- Allows 4 sprites per row (512 pixels total width)
- Creates immersive, detailed environments

**Event Markers** (Signs, Portals):
- Signs: 32×64 or 64×64 pixels
- Portals: 128×128 pixels
- Flexible grid sizes (2-4 columns)

### Minimum & Maximum Dimensions

| Category | Min Size | Recommended | Max Size |
|----------|----------|-------------|----------|
| Actor | 16×16 | 32×48 | 64×96 |
| Tile | 64×64 | 128×128 | 256×256 |
| Marker | 32×32 | 64×64 | 128×128 |

**Performance Note**: Larger sprites (256×256+) may impact performance. Use 128×128 as a practical limit.

### Transparency Best Practices

**DO**:
- Use full alpha channel for feathered edges
- Pre-multiply alpha for cleaner anti-aliasing
- Test sprites on different background colors

**DON'T**:
- Use thin halos or glows (they'll be visible against any background)
- Leave anti-aliasing artifacts around edges
- Mix opaque and transparent pixels randomly

---

## Part 5: Configuring Your Sprite Sheet

Once your PNG is created, register it in the sprite sheet registry.

### Location

Edit: `data/sprite_sheets.ron`

### Configuration Format

```ron
"my_new_sheet": (
    texture_path: "sprites/my_new_sheet.png",
    tile_size: (32.0, 48.0),          // Width, height of each sprite
    columns: 4,                        // Sprites per row
    rows: 2,                           // Total rows
    sprites: [
        (0, "sprite_name_1"),
        (1, "sprite_name_2"),
        // ... more sprites
    ],
),
```

### Complete Example: NPC Town Sheet

```ron
"npcs_town": (
    texture_path: "sprites/npcs_town.png",
    tile_size: (32.0, 48.0),
    columns: 4,
    rows: 4,
    sprites: [
        (0, "guard"),
        (1, "merchant"),
        (2, "innkeeper"),
        (3, "blacksmith"),
        (4, "priest"),
        (5, "noble"),
        (6, "peasant"),
        (7, "child"),
        (8, "elder"),
        (9, "mage_npc"),
        (10, "warrior_npc"),
        (11, "rogue_npc"),
        (12, "captain"),
        (13, "mayor"),
        (14, "servant"),
        (15, "beggar"),
    ],
),
```

### Field Descriptions

| Field | Type | Description |
|-------|------|-------------|
| `texture_path` | String | Path relative to `assets/` directory |
| `tile_size` | (f32, f32) | (width, height) in pixels of one sprite |
| `columns` | u32 | Number of sprites per row |
| `rows` | u32 | Number of rows total |
| `sprites` | Vec<(u32, String)> | Sprite indices and names |

### Validation

After editing `sprite_sheets.ron`:

```bash
# Check syntax is valid RON
cargo check

# Run tests to verify configuration
cargo nextest run --all-features

# Look for sprite-related tests passing
```

---

## Part 6: Using Your Sprites in Campaign Content

### Attaching Sprites to Tile Visuals

In your campaign's tile definitions (e.g., `data/maps/map_1.ron`):

```ron
Tile(
    id: 1,
    tile_type: Wall,
    visual: TileVisualMetadata(
        base_color: (100, 100, 100),
        sprite: Some(SpriteReference(
            sheet_path: "sprites/walls.png",
            sprite_index: 0,           // stone_wall
            animation: None,
        )),
    ),
),
```

### Attaching Sprites to NPCs

In your NPC spawn definitions:

```ron
Npc(
    id: 1,
    name: "Town Guard",
    sprite: Some(ActorSprite(
        sheet_path: "sprites/npcs_town.png",
        sprite_index: 0,               // guard sprite (index 0)
        actor_type: Npc,
    )),
    // ... other NPC fields
),
```

### Using Named Sprites (Recommended)

Instead of hardcoding indices, use the sprite names from your RON:

```rust
// In Rust code
let sprite_ref = SpriteReference {
    sheet_path: "sprites/npcs_town.png".to_string(),
    sprite_index: 0,  // corresponds to "guard" in config
    animation: None,
};
```

Or in RON, you can use the name directly (if your system supports it):

```ron
sprite_name: "guard"  // Resolves to index 0 automatically
```

---

## Part 7: Testing Your Sprites

### Quick Test: Enable Sprite Rendering

Before full integration, test that your sprites load correctly.

### Step 1: Verify File Exists

```bash
ls -la assets/sprites/your_sprite_sheet.png
# Should show the file with reasonable size (10KB - 500KB)
```

### Step 2: Load in Campaign

Edit a test map to use your sprite:

```ron
Tile(
    id: 1,
    tile_type: Wall,
    visual: TileVisualMetadata(
        base_color: (128, 128, 128),
        sprite: Some(SpriteReference(
            sheet_path: "sprites/your_sprite_sheet.png",
            sprite_index: 0,
            animation: None,
        )),
    ),
),
```

### Step 3: Run Campaign

```bash
cargo run -- --campaign my_campaign
```

### Visual Checklist

- [ ] Sprite appears on tile (not missing texture)
- [ ] Sprite colors are correct (not inverted/desaturated)
- [ ] Transparency is preserved (background shows through)
- [ ] Sprite is correctly positioned (not stretched/offset)
- [ ] Grid alignment is correct (sprite doesn't overlap adjacent tiles)

### Common Issues & Fixes

| Problem | Cause | Solution |
|---------|-------|----------|
| Sprite missing/pink | File not found | Verify `texture_path` in RON |
| Sprite is stretched | Wrong `tile_size` | Match dimensions in RON to actual sprite size |
| Sprite looks washed out | Wrong color space | Re-export as sRGB PNG |
| Sprite has halos/glows | Alpha not pre-multiplied | Use "Color to Alpha" more carefully |
| Black background instead of transparent | No alpha channel | Re-export as PNG-32 (RGBA) |

---

## Part 8: Best Practices for Sprite Creation

### Style Consistency

**Within a Sheet**:
- Use consistent lighting direction
- Match line weights and detail level
- Maintain consistent color palette
- Use same perspective and scale

**Across Sheets**:
- NPCs should be same scale relative to tiles
- Color tones should be cohesive
- Detail level should match overall aesthetic

### Naming Conventions

Use descriptive, lowercase sprite names in your registry:

```ron
sprites: [
    (0, "stone_wall_base"),        // ✓ Clear and descriptive
    (1, "stone_wall_damaged"),     // ✓ Semantic
    (2, "s1"),                     // ✗ Too cryptic
    (3, "Wall_Type_1"),            // ✗ Inconsistent casing
]
```

### File Organization

Create a working directory to manage source files:

```
my_campaign/
├── assets/
│   ├── sprites/
│   │   ├── npcs_town.png         (Final exported)
│   │   ├── walls.png             (Final exported)
│   │   └── working/              (Source files)
│   │       ├── npcs_source.aseprite
│   │       ├── walls_source.aseprite
│   │       └── individual_sprites/
│   │           ├── guard.png
│   │           ├── merchant.png
│   │           └── ...
```

### Version Control

**For Git repositories**:

```gitignore
# Ignore source files, commit only final PNGs
assets/sprites/working/
*.aseprite
*.xcf
*.kra
```

But DO commit:
```
assets/sprites/*.png
data/sprite_sheets.ron
```

---

## Part 9: Advanced: Sprite Animations

### Frame-Based Animation

Antares supports sprite frame animations. Use multiple sprites in a sheet for animation frames.

Example: Walking animation (4 frames)

```ron
"npcs_town": (
    texture_path: "sprites/npcs_town.png",
    tile_size: (32.0, 48.0),
    columns: 4,
    rows: 4,
    sprites: [
        (0, "guard_idle"),
        (1, "guard_walk_1"),
        (2, "guard_walk_2"),
        (3, "guard_walk_3"),
        // ... rest of NPCs use indices 4-15
    ],
),
```

### Animation Configuration in Tile Metadata

```ron
Tile(
    id: 1,
    tile_type: Decorative,
    visual: TileVisualMetadata(
        base_color: (255, 255, 255),
        sprite: Some(SpriteReference(
            sheet_path: "sprites/decorations.png",
            sprite_index: 0,  // Start frame
            animation: Some(SpriteAnimation(
                frames: [0, 1, 2, 1],        // Frame sequence (can loop back)
                fps: 10.0,                   // 10 frames per second
                looping: true,               // Loop the animation
            )),
        )),
    ),
),
```

### Creating Animation Frames

**Approach**: Dedicate rows to animations

Example: 4 NPCs with 2-frame walk animation each

```
4×2 grid (128 × 96):

NPC 1 idle    | NPC 1 walk   | NPC 2 idle    | NPC 2 walk
NPC 3 idle    | NPC 3 walk   | NPC 4 idle    | NPC 4 walk
```

Then register:

```ron
sprites: [
    (0, "npc1_idle"),
    (1, "npc1_walk"),
    (2, "npc2_idle"),
    (3, "npc2_walk"),
    (4, "npc3_idle"),
    (5, "npc3_walk"),
    (6, "npc4_idle"),
    (7, "npc4_walk"),
]
```

---

## Part 10: Distribution & Sharing

### Packaging Your Sprites

**For Campaign Distribution**:

Always include sprite assets with your campaign:

```
my_campaign/
├── campaign.ron
├── data/
│   └── ... (game content)
└── assets/
    └── sprites/
        ├── npcs_town.png
        ├── walls.png
        └── decorations.png
```

### Attribution & Licensing

If using existing sprites or art:

1. **Verify License**: Ensure you have permission to use and distribute
2. **Document Attribution**: Include in `README.md` or `CREDITS.txt`
3. **Original Art**: Document as "Original artwork by [Artist]"

Example `CREDITS.txt`:

```
SPRITE ASSETS
─────────────

NPCs Town Sheet
  Created by: Jane Artist
  License: CC-BY 4.0
  Modifications: Antares format adaptation

Monsters Basic Sheet
  Source: OpenGameArt.org
  Original Author: John Sprite
  License: CC0 (Public Domain)
```

---

## Part 11: Troubleshooting

### Sprites Not Loading

**Error**: Pink/magenta texture or missing sprite

**Check**:
1. File exists at `assets/sprites/your_file.png`
2. Filename matches `texture_path` in `sprite_sheets.ron` (case-sensitive!)
3. RON syntax is valid: `cargo check`

**Fix**:
```bash
# Verify file exists
find assets/sprites -name "*.png" | grep your_file

# Verify RON can be parsed
cargo check --all-targets
```

### Sprites Appear Stretched

**Error**: Sprite looks distorted or stretched

**Check**:
1. `tile_size` in RON matches actual sprite pixel dimensions
2. Canvas dimensions match: `width = tile_width × columns`

**Fix**:
```
If your sprite sheet is 128×96 pixels with 32×48 sprites:
  128 ÷ 4 columns = 32 ✓
   96 ÷ 2 rows = 48 ✓
```

### Sprites Have Dark Halos

**Error**: Dark or colored edges around sprites

**Check**:
1. Transparency was properly set (no dark pixels under alpha)
2. Pre-multiplied alpha setting

**Fix**:
- Re-export from source file
- Use "Color to Alpha" with exact background color
- Ensure selection border is clean (no partial pixels)

### RON Syntax Errors

**Error**: `cargo check` fails with RON parsing error

**Common Issues**:
- Missing commas between entries
- Unmatched parentheses or brackets
- Trailing comma in last entry (should be `[...,]` or `[...])

**Fix**:
```ron
// Wrong
sprites: [
    (0, "guard")        // ✗ Missing comma
    (1, "merchant"),
]

// Right
sprites: [
    (0, "guard"),       // ✓ Comma after each entry
    (1, "merchant"),
]
```

---

## Next Steps

### Immediate

1. Create your first simple sprite sheet (4×2 grid, 8 sprites)
2. Export as PNG-24 with transparency
3. Add configuration to `sprite_sheets.ron`
4. Test loading in a campaign

### Soon

1. Create additional sprite sheets (tiles, decorations)
2. Add animation frames to your sprites
3. Populate your campaign with sprite-based entities
4. Test performance with multiple sprites

### Later

1. Polish sprite artwork (details, lighting, effects)
2. Create variant sprites for different conditions
3. Implement sprite swapping for equipment/status changes
4. Develop sprite authoring tools or batch conversion scripts

---

## Resources

### Documentation

- `docs/reference/architecture.md` - Sprite system architecture
- `docs/explanation/phase3_sprite_rendering_integration.md` - Sprite rendering details
- `data/sprite_sheets.ron` - Current sprite registry

### External Tools & Resources

- **LibreSprite**: [libresprite.github.io](https://libresprite.github.io/) (free pixel art)
- **Aseprite**: [aseprite.org](https://www.aseprite.org/) (paid, professional)
- **OpenGameArt**: [opengameart.org](https://opengameart.org/) (free sprites & assets)
- **Piskel**: [piskelapp.com](https://www.piskelapp.com/) (online pixel art)
- **PNG Compression**: [tinypng.com](https://tinypng.com/) (optimize sprite file sizes)

### Community

- Post sprite questions in the Antares community forum
- Share your sprite sheets with other creators
- Request feedback on assets before final integration

---

## FAQ

**Q: Can I use sprites from other games?**
A: Only if the license permits. Always check and credit original artists.

**Q: What's the maximum sprite size?**
A: 256×256 pixels is practical; larger sprites increase memory usage. For UI and large effects, consider 128×128.

**Q: Can I mix sprite sizes in one sheet?**
A: Each sheet must use uniform `tile_size`. Create separate sheets for different sizes.

**Q: How do I create animated sprites?**
A: Use consecutive sprite indices and configure `animation` in your sprite metadata (see Part 9).

**Q: What's the file size limit for sprite sheets?**
A: No hard limit, but ~500KB per sheet is reasonable. PNG compression helps keep sizes down.

**Q: Can I update sprites after release?**
A: Yes! Simply replace the PNG file. Campaigns that reference it will automatically use the new version.

---

## Quick Reference

### Sprite Sheet Dimensions Cheat Sheet

```
Actor Sprites (NPCs, Monsters):
  Small (32×48):    128×192 pixels (4×4 = 16 sprites)
  Tiny (16×24):     64×96 pixels (4×4 = 16 sprites)

Tile Sprites:
  Standard (128×128):  512×512 pixels (4×4 = 16 sprites)
  Tall (128×256):      512×1024 pixels (4×4 = 16 sprites)

Decorations:
  Small (64×64):    512×512 pixels (8×8 = 64 sprites)
  Medium (32×64):   128×128 pixels (4×2 = 8 sprites)
```

### RON Template

```ron
"my_sheet": (
    texture_path: "sprites/my_sheet.png",
    tile_size: (32.0, 48.0),
    columns: 4,
    rows: 4,
    sprites: [
        (0, "sprite_0"),
        (1, "sprite_1"),
        // ... add more
    ],
),
```

### Export Settings Checklist

- [ ] Format: PNG (not JPG, BMP, or WebP)
- [ ] Bit Depth: 24-bit (RGB) or 32-bit (RGBA)
- [ ] Transparency: Alpha channel enabled
- [ ] Color Space: sRGB
- [ ] Compression: Default/Level 6-9
- [ ] Location: `assets/sprites/`

---

End of tutorial. Happy sprite creation!
