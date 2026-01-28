# Sprite Assets Directory

This directory contains sprite sheet textures used for billboard rendering.

## Required Sprite Sheets

### Tile Sprites

- `walls.png` - 512x1024, 4x4 grid, 128x256 tiles
- `doors.png` - 512x512, 4x2 grid, 128x256 tiles
- `terrain.png` - 1024x1024, 8x8 grid, 128x128 tiles
- `trees.png` - 512x1024, 4x4 grid, 128x256 tiles
- `decorations.png` - 512x512, 8x8 grid, 64x64 tiles

### Actor Sprites

- `npcs_town.png` - 128x192, 4x4 grid, 32x48 sprites
- `monsters_basic.png` - 128x192, 4x4 grid, 32x48 sprites
- `monsters_advanced.png` - 128x192, 4x4 grid, 32x48 sprites
- `recruitables.png` - 128x96, 4x2 grid, 32x48 sprites

### Event Marker Sprites

- `signs.png` - 128x128, 4x2 grid, 32x64 sprites
- `portals.png` - 512x256, 4x2 grid, 128x128 sprites

## Format Specifications

- **Format**: PNG-24 with alpha channel
- **Color Space**: sRGB
- **Transparency**: Full alpha support required
- **Grid Layout**: Row-major order (left-to-right, top-to-bottom)

## Creation

Sprite images will be created in Phase 4 of sprite_support_implementation_plan.md
