# Portrait export checklist (HUD portraits)

This document is a concise, actionable checklist for artists and exporters producing HUD (hub) portraits for Antares campaigns.

TL;DR
- Primary asset: 1× PNG, 64×64 px, sRGB (8-bit/channel RGBA)
- HUD render size: 40 px (in-game constant `PORTRAIT_SIZE = 40.0`)
- HiDPI: include 2× at 128×128 (name with `@2x` suffix, e.g. `10@2x.png`)
- Sprite sheet (optional): 5×4 grid, cells = 64×64 px, 4 px spacing → sheet = 336×268 px

Why 64×64?
```antares/docs/explanation/character_portraits.md#L1-4
Create a single image sprite sheet with 20 small fantasy character portraits arranged in a 5x4 grid, each exactly 64x64 pixels with 4 px between them...
```

Implementation note (HUD render size)
```antares/src/game/systems/hud.rs#L64-68
// Portrait display constants
pub const PORTRAIT_SIZE: f32 = 40.0;
```

Where to put files
- Place portrait images in the campaign's assets folder:
  `campaigns/<campaign>/assets/portraits/`
- Numeric filenames map to `portrait_id` values (`10.png` → id 10).
- Normalized names map by filename stem (`kira.png` → "kira").

Naming conventions
- Numeric: `10.png` (1×), `10@2x.png` (2×) — numeric stems map to `portrait_id`.
- Named: `kira.png`, `kira@2x.png` — lowercased, spaces → underscores.
- Always use lowercase, no spaces.

File format & color/profile
- Format: PNG (RGBA) — lossless, alpha channel supported.
- Color profile: sRGB (IEC61966-2.1). Convert/flatten to sRGB on export.
- Bit depth: 8 bits per channel (typical for screen).
- Optimize: run a lossless optimizer (`oxipng -o6` or `pngcrush -brute`) for smaller build size.
- Do not use lossy palette converters unless you have a pixel-art-specific workflow that preserves look.

Sprite sheet spec (if providing a sheet)
- Grid: 5 columns × 4 rows = 20 portraits
- Cell size: 64×64 px
- Spacing (gutter) between cells: 4 px
- Total sheet size: 5*64 + 4*4 = 336 px width; 4*64 + 3*4 = 268 px height (336×268 px)
- Background: dark background is fine for the sheet; individual PNGs may be exported with transparency if preferred.
- Keep at least 4 px clear space between the subject and cell edges so crops/variants don't clip the artwork.

HiDPI / scale variants
- Provide 2× assets for Retina/HiDPI: 128×128 px named `name@2x.png` or `10@2x.png`.
- Providing 3× (192×192) is optional for extreme DPI targets.
- Note: loader currently maps numeric stems and names; `@2x` is a recommended convention — engine selection of @2x may require runtime device-pixel-ratio detection (can be implemented if desired).

Artwork guidelines (style & composition)
- View: head-and-shoulders only; faces fully visible. No full helmets or full head coverings.
- Consistency: match style and scale across the set (eye line across portraits should be consistent).
- No text, labels, or logos on the portrait art.
- Keep lighting consistent (rim light / strong highlights recommended per project style).
- Pixel art: snap to pixel grid and keep palettes consistent; if upscaling pixel art use nearest-neighbor to preserve crispness.

Export settings (Photoshop / Affinity / GIMP / Figma)
- Color mode: RGB, 8-bit/channel
- Convert to profile: sRGB (IEC61966-2.1)
- Export as PNG-24 (or PNG with alpha)
- For pixel art: use nearest-neighbor / integer-scaled exports; avoid interpolation artifacts

Example commands (ImageMagick; adjust per tool)
```/dev/null/commands.sh#L1-12
# Resize single image to 64×64 (1×)
convert input.png -filter Lanczos -resize 64x64 campaigns/tutorial/assets/portraits/10.png

# Resize to 128×128 (2×)
convert input.png -filter Lanczos -resize 128x128 campaigns/tutorial/assets/portraits/10@2x.png

# Generate a 336×268 template (5x4 grid, 64px cells, 4px spacing)
# (This is an illustrative example; use your preferred export workflow.)
convert -size 336x268 xc:'#121217' -stroke '#444' -strokewidth 1 \
  -fill 'rgba(255,255,255,0.02)' -draw 'rectangle 0,0 64,64 rectangle 68,0 132,64 ...' \
  campaigns/tutorial/assets/portraits/template_sheet_64x_4px.png

# Composite a 64px sample into top-left cell
convert campaigns/tutorial/assets/portraits/template_sheet_64x_4px.png \
  campaigns/tutorial/assets/portraits/10.png -geometry +0+0 -composite \
  campaigns/tutorial/assets/portraits/template_with_sample_64.png
```

Optimization and checking
- Run a lossless optimizer:
```/dev/null/commands.sh#L13-15
# Lossless optimization (example)
oxipng -o6 campaigns/tutorial/assets/portraits/10.png
```
- Visual checks:
  - Inspect at 100% and 200% in an image viewer.
  - Verify subject is centered and consistent with other portraits.
  - Verify alpha (transparency) where applicable and no stray pixels.
- In-game checks:
  - Confirm HUD displays the portrait in the character card.
  - Verify portrait looks good at 40 px, and at 2× screens if you provided @2x assets.

Acceptance checklist (before submitting assets)
- [ ] Asset(s) exported at exact size(s): 64×64 (1×) and 128×128 (2×) if provided
- [ ] Pixel grid, detail, and facial placement consistent across portraits
- [ ] Filenames use lowercase, no spaces; numeric stems use correct `portrait_id`
- [ ] Color space: sRGB (8-bit); PNG with alpha (where appropriate)
- [ ] Sprite sheet (if used): 5×4 grid, cells 64×64, 4 px gutters, total 336×268
- [ ] Performed lossless optimization and confirmed visual parity
- [ ] Source files (PSB / PSD / Figma / SVG) kept in a `sources/` folder alongside the PNGs
- [ ] Add an entry to the campaign docs if you introduce new named portraits (document filename ↔ in-game name & id)

References
- HUD implementation & constant: `PORTRAIT_SIZE = 40.0`
```antares/src/game/systems/hud.rs#L64-68
pub const PORTRAIT_SIZE: f32 = 40.0;
```
- Portrait sprite/thumbnail guidance
```antares/docs/explanation/character_portraits.md#L1-4
Create a single image sprite sheet with 20 small fantasy character portraits arranged in a 5x4 grid, each exactly 64x64 pixels with 4 px between them...
```

Notes / FAQ
- Q: What DPI should I put in the PNG metadata?  
  A: DPI is meaningless for raster game assets — use pixels (64×64). If you must set DPI, 72 DPI is an acceptable default; the engine ignores DPI metadata.
- Q: Should portraits have transparent backgrounds?  
  A: Either is fine. Individual PNGs with alpha are convenient. If you deliver a sprite sheet with a dark sheet background, ensure each cell area is consistent and crop-friendly.

If you'd like, I can:
- generate a ready-made 64×64 template sprite sheet + 2× version, and
- create example 1×/2× exports for a sample character and place them in `campaigns/tutorial/assets/portraits/`.

Want me to create those sample files and add them to the tutorial assets? If yes, tell me which sample(s) to use.
