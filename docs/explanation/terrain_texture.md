# Terrain Textures

Terrain textures are now **per-definition**, not fixed filenames. Each
`TerrainDefinition` in `data/terrain.ron` (or a campaign's `data/terrain.ron`)
carries a `texture_path` field that points to the PNG asset used by the
rendering pipeline.

## How Texture Loading Works

The `TerrainMaterialCache` loads materials on demand:

```antares/src/game/resources/terrain_material_cache.rs#L51-80
/// Cache for terrain PBR materials indexed by TerrainId.
pub struct TerrainMaterialCache { ... }
```

When a tile with `terrain: 13001` (Grass) is spawned, the renderer calls
`TerrainMaterialCache::get_or_load(13001)`, which looks up the
`TerrainDefinition` for ID 13001, reads its `texture_path`, and loads it as a
Bevy `StandardMaterial`.

## Built-in Terrain Texture Paths

| ID | Name | Default texture path |
|----|------|---------------------|
| 13000 | Ground | `assets/textures/terrain/ground.png` |
| 13001 | Grass | `assets/textures/terrain/grass.png` |
| 13002 | Water | `assets/textures/terrain/water.png` |
| 13003 | Lava | `assets/textures/terrain/lava.png` |
| 13004 | Swamp | `assets/textures/terrain/swamp.png` |
| 13005 | Stone | `assets/textures/terrain/stone.png` |
| 13006 | Dirt | `assets/textures/terrain/dirt.png` |
| 13007 | Forest | `assets/textures/terrain/forest_floor.png` |
| 13008 | Mountain | `assets/textures/terrain/mountain.png` |
| 13009 | Sand | `assets/textures/terrain/sand.png` |
| 13010 | Snow | `assets/textures/terrain/snow.png` |
| 13011 | Ice | `assets/textures/terrain/ice.png` |

## Placeholder vs. Final Art

The `antares_sdk textures generate` CLI command (see `src/sdk/cli/texture_generator.rs`)
generates deterministic 64×64 placeholder PNGs for the built-in terrain types. These
are placeholder stubs — replace them with your own art at the same paths for better
visual quality.

## Using Custom Terrain Textures

To override a built-in texture or define a new terrain with a custom texture,
add a `TerrainDefinition` entry to your campaign's `data/terrain.ron`:

```ron
// campaigns/my_campaign/data/terrain.ron
[
    TerrainDefinition(
        id: 13100,
        name: "Cobblestone",
        texture_path: "assets/textures/terrain/cobblestone.png",
        mesh_style: Flat,
        vegetation: None,
        blocked: false,
        roughness: 0.9,
        height: 0.0,
    ),
]
```

Any `TerrainId` >= 13100 is reserved for campaign-defined terrain (by convention).
Built-in IDs 13000–13099 are reserved for the engine.

## Texture Size

Bevy loads the PNG at whatever resolution it is — there is no engine-imposed
size constraint for terrain tiles. Common choices:

- **64×64** — placeholder quality (what the generator produces)
- **256×256** — noticeably better for seamless tiling
- **512×512** or **1024×1024** — high quality, reasonable VRAM cost

The texture is applied as a `base_color_texture` on a `StandardMaterial` (PBR).
