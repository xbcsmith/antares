# Terrain

The 64×64 size is just the placeholder generator's output — not a Bevy limit

The terrain textures (ground, grass, stone, etc.) come from the `antares_sdk textures generate` CLI command. That command is explicitly documented as a **placeholder generator**:

```antares/src/sdk/cli/texture_generator.rs#L84-91
#[command(
    about = "Generate placeholder terrain textures",
    long_about = "Generates deterministic placeholder textures for terrain tiles, grass, and trees.\n\n\
                  Output sub-directories (`terrain/`, `grass/`, `trees/`) are created inside\n\
                  the specified --output-dir. Existing files are overwritten."
)]
```

The 64×64 size is hardcoded into the generator's terrain constants:

```antares/src/sdk/cli/texture_generator.rs#L252-256
/// Output image width in pixels.
const IMAGE_WIDTH: u32 = 64;
/// Output image height in pixels.
const IMAGE_HEIGHT: u32 = 64;
/// Maximum per-channel noise magnitude (±NOISE_RANGE).
const NOISE_RANGE: i32 = 10;
```

These constants only apply to the terrain tile textures. The generator was designed for minimum viable noise-colored placeholders, not final art.

## You can use any size you want

The rendering pipeline loads these as Bevy `StandardMaterial`s (PBR materials), one PNG per terrain type:

```antares/src/game/systems/terrain_materials.rs#L140-150
    for terrain in terrain_types {
        let texture_path = texture_path_for(terrain);
        let texture_handle: Handle<Image> = asset_server.load(texture_path);

        let material_handle = materials.add(StandardMaterial {
            base_color_texture: Some(texture_handle),
            perceptual_roughness: roughness_for(terrain),
            ..default()
        });

        cache.set(terrain, material_handle);
```

There's no dimension check anywhere in that path. Bevy just loads whatever PNG lives at `assets/textures/terrain/ground.png`, `grass.png`, etc. The SDK's dimension validation only covers `assets/textures/trees/` and `assets/textures/grass/grass_blade.png` — **not** terrain tiles.

## What to do

Replace the placeholder files with your own higher-resolution art at the same paths:

```/dev/null/paths.txt#L1-10
assets/textures/terrain/ground.png
assets/textures/terrain/grass.png
assets/textures/terrain/stone.png
assets/textures/terrain/mountain.png
assets/textures/terrain/dirt.png
assets/textures/terrain/water.png
assets/textures/terrain/lava.png
assets/textures/terrain/swamp.png
assets/textures/terrain/forest_floor.png
```

Common sensible sizes for a seamlessly-tiling PBR terrain texture are **256×256**, **512×512**, or **1024×1024** — all totally fine for Bevy. Modern GPUs typically support up to 8192×8192 or 16384×16384. The only practical limit is VRAM. For a tiled terrain material (which is what these are), 256×256 looks considerably better than 64×64 without being expensive.

If you'd like, I can update `IMAGE_WIDTH` / `IMAGE_HEIGHT` in the generator to produce larger placeholders by default — though the better long-term move is to drop in real art assets.
