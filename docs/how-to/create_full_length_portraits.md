# How to Create Full-Length Character Portraits

Full-length portraits are displayed in the **Character Sheet** screen when a
player opens a party member's stats (default key: `P`, or by clicking a HUD
portrait).  They appear in the left column of the sheet at a rendered size of
**170 × 280 pixels**.  When no full portrait exists for a character the engine
automatically shows a colored placeholder with the character's initials, so
full-length portraits are purely additive — the game works fine without them.

---

## 1. Where Files Go

```text
<campaign_root>/
  assets/
    portraits/          ← HUD (head) portraits — 64×64 px — unchanged
    portraits/full/     ← Full-length portraits — this guide
```

Place each portrait PNG inside `assets/portraits/full/` in your campaign
directory.  For the tutorial campaign this is:

```text
campaigns/tutorial/assets/portraits/full/
```

The directory is created automatically the first time you add a file to it.
An empty directory causes no errors.

---

## 2. Naming Convention

The engine matches a portrait file to a character by comparing the **file stem
(lowercased, spaces → underscores)** against the character's `portrait_id`
field (also lowercased and spaces replaced).  If `portrait_id` is empty the
character's `name` field is used instead.

| Character `portrait_id` | File name |
|---|---|
| `aldric` | `aldric.png` |
| `Kira` | `kira.png` |
| `Old Gareth` | `old_gareth.png` |
| *(empty — name "Mira Windwhisper")* | `mira_windwhisper.png` |

Rules:
- Lowercase only — `Aldric.PNG` will **not** match.
- Spaces become underscores — `old gareth.png` will **not** match.
- Accepted extensions: `.png`, `.jpg`, `.jpeg` (PNG strongly preferred).
- No `@2x` suffix is used for full portraits (the sheet renders them at a fixed
  logical size; HiDPI scaling is handled by egui/Bevy automatically).

### Setting `portrait_id` in a character definition

In the character's `.ron` definition, set the `portrait_id` field to the file
stem you want to use:

```text
antares/campaigns/tutorial/data/characters.ron
portrait_id: "aldric",
```

If you leave `portrait_id` empty the engine falls back to the character's
`name` field, lowercased and with spaces replaced by underscores.

---

## 3. Technical Specifications

| Property | Value |
|---|---|
| **Rendered display size** | 170 × 280 px (logical pixels) |
| **Recommended source size** | 340 × 560 px (2× for HiDPI sharpness) |
| **Aspect ratio** | ~0.607 : 1 (portrait — approximately 3:5) |
| **Format** | PNG-24 (RGBA) — lossless, alpha supported |
| **Color space** | sRGB (IEC 61966-2.1), 8 bits per channel |
| **Background** | Transparent alpha OR opaque dark background |

The engine renders the image at exactly 170 × 280 logical pixels regardless of
source resolution, so a 340 × 560 source will appear crisp on HiDPI displays
while a 170 × 280 source will appear sharp on standard displays.  Either works;
340 × 560 is recommended for new assets.

---

## 4. Composition Guidelines

Full-length portraits are a **head-to-feet full-body view** — very different
from the HUD head-and-shoulders portraits.  The character sheet gives the
engine room to show the whole character.

### Framing

- Show the **full figure** from crown to feet, centered horizontally.
- Leave approximately **5–8 % clear margin** on all four sides so feet are not
  clipped and the head is not tight against the top edge.
- The character should occupy roughly **70–80 % of the canvas height**.
- A slight **3/4 view** (turned ~15–30° off center) reads better than a flat
  front-on stance and gives the art more depth.

### Pose

- **Relaxed ready stance** — weight on one foot, weapon at rest or held loosely.
  Avoid action poses (mid-swing, mid-cast); the character sheet is a stat
  screen, not a combat screen.
- For **casters**: staff held at side or arcane energy gently visible around
  hands; robes settled, not billowing.
- For **warriors**: weapon hand lowered, shield on arm or resting against leg.
- Weapon and armor class should be **readable at a glance** — this is how the
  player confirms their loadout matches the stats on the right column.

### Style

Match the style of the existing HUD portraits (defined in
`docs/explanation/portrait_prompt.md`):

> Realistic high-fantasy art style with painterly details, intricate textures,
> and dramatic lighting, matching the aesthetic of Baldur's Gate 3 or Divinity:
> Original Sin 2. Serious and gritty tone, not cartoonish.

Additional notes for full-length portraits specifically:

- **Rim lighting** on the figure helps separate it from a dark background and
  reads clearly at small display sizes.
- Keep **fine costume detail on the torso and upper body** where it is most
  visible at 170 × 280 display size; legs and feet can be simpler.
- **Avoid pure black backgrounds** — use the campaign's standard dark neutral
  (`#282c33` or similar) so portraits look cohesive when placed side by side
  in the sheet.
- If using a transparent background, egui will composite the portrait over the
  character sheet's window background color.

---

## 5. AI Generation Prompts

The prompts below are designed for Midjourney v6, DALL-E 3, or similar
high-fidelity image generators.  They extend the character descriptions in
`docs/explanation/portrait_prompt.md` with full-body framing instructions.

### Base prompt template

Replace `[CHARACTER DESCRIPTION]` with any of the per-character descriptions
from `docs/explanation/portrait_prompt.md` (the "Characters Sorted by Gender,
Race, and Class" section).

```text
Full-body fantasy character portrait, head-to-feet, relaxed ready stance,
slight 3/4 view. [CHARACTER DESCRIPTION].
Realistic high-fantasy art style, painterly details, intricate textures,
dramatic rim lighting, Baldur's Gate 3 / Divinity Original Sin 2 aesthetic.
Serious tone, not cartoonish. Full figure centered on a dark #282c33
background, 5% margin on all sides, weapon at rest. No text, no labels,
no cropping of feet or head. Vertical orientation, 3:5 aspect ratio.
--ar 3:5 --stylize 300 --v 6
```

### Tutorial campaign — per-character prompts

These correspond to the characters defined in
`docs/explanation/character_generation_prompt.md`.

#### Kira (Human Knight) — `kira.png`

```text
Full-body fantasy character portrait, head-to-feet, relaxed ready stance,
slight 3/4 view. A young female human knight with a determined expression,
fair skin, and brown hair tied back in a ponytail. Polished steel plate armor
with a blue tabard, broadsword resting at her hip, right hand on pommel.
Realistic high-fantasy painterly style, dramatic rim lighting, Baldur's Gate 3
aesthetic. Dark #282c33 background, full figure with 5% margin, no cropping.
No text or labels. Vertical 3:5 aspect ratio. --ar 3:5 --stylize 300 --v 6
```

#### Sirius (Elf Sorcerer) — `sirius.png`

```text
Full-body fantasy character portrait, head-to-feet, relaxed ready stance,
slight 3/4 view. A severe male elf sorcerer with sharp angular features, long
silver hair, and glowing blue eyes. Dark blue robes with silver embroidery,
arcane staff held loosely at his side, faint magical energy around his free
hand. Realistic high-fantasy painterly style, dramatic rim lighting, Baldur's
Gate 3 aesthetic. Dark #282c33 background, full figure with 5% margin, no
cropping. No text or labels. Vertical 3:5 aspect ratio.
--ar 3:5 --stylize 300 --v 6
```

#### Mira (Human Cleric) — `mira.png`

```text
Full-body fantasy character portrait, head-to-feet, relaxed ready stance,
slight 3/4 view. A serene female human cleric with a kind face, fair skin, and
warm eyes. White and gold priestly robes, holy symbol glowing softly at her
chest, mace hanging at her belt. Soft holy rim lighting, realistic
high-fantasy painterly style, Baldur's Gate 3 aesthetic. Dark #282c33
background, full figure with 5% margin, no cropping. No text or labels.
Vertical 3:5 aspect ratio. --ar 3:5 --stylize 300 --v 6
```

#### Old Gareth (Dwarf Knight) — `old_gareth.png`

```text
Full-body fantasy character portrait, head-to-feet, relaxed ready stance,
slight 3/4 view. A grizzled elderly male dwarf with a thick gray beard,
weathered skin, and a stern but noble expression. Heavy chainmail armor with a
fur-lined cloak, large warhammer resting head-down on the ground beside him,
hand on the haft. Realistic high-fantasy painterly style, dramatic rim
lighting, Baldur's Gate 3 aesthetic. Dark #282c33 background, full figure with
5% margin, no cropping. No text or labels. Vertical 3:5 aspect ratio.
--ar 3:5 --stylize 300 --v 6
```

#### Whisper (Elf Rogue) — `whisper.png`

```text
Full-body fantasy character portrait, head-to-feet, relaxed ready stance,
slight 3/4 view. A nimble female elf rogue with a mischievous smirk, sharp
features, and short messy dark hair. Tight-fitting leather armor with a dark
green hooded cloak pushed back, dual daggers sheathed at her waist, one hand
resting on a hilt. Shadowy side-rim lighting, realistic high-fantasy painterly
style, Baldur's Gate 3 aesthetic. Dark #282c33 background, full figure with 5%
margin, no cropping. No text or labels. Vertical 3:5 aspect ratio.
--ar 3:5 --stylize 300 --v 6
```

#### Zara (Gnome Sorcerer) — `zara.png`

```text
Full-body fantasy character portrait, head-to-feet, relaxed ready stance,
slight 3/4 view. A cute and enthusiastic female humanoid gnome sorcerer (short
stature, precise humanoid anatomy, standing upright on two legs, two arms),
large expressive eyes, wild pink hair, oversized round glasses. Colorful
patchwork robes, large spellbook tucked under one arm, other hand gesturing
with a tiny spark of arcane energy. Warm whimsical rim lighting, realistic
high-fantasy painterly style, Baldur's Gate 3 aesthetic. Dark #282c33
background, full figure with 5% margin, no cropping. No text or labels.
Vertical 3:5 aspect ratio. --ar 3:5 --stylize 300 --v 6
```

---

## 6. Export and Optimization

### Recommended export steps

1. **Crop and resize** the generated image to your target resolution
   (340 × 560 recommended).

   ```text
   # Resize to 340x560 preserving aspect ratio with Lanczos filter
   convert input.png -filter Lanczos -resize 340x560! \
     campaigns/tutorial/assets/portraits/full/kira.png
   ```

2. **Convert to sRGB** if your source is in a wide-gamut color space:

   ```text
   convert input.png -colorspace sRGB -type TrueColorAlpha \
     campaigns/tutorial/assets/portraits/full/kira.png
   ```

3. **Lossless optimization** — run after export to reduce file size:

   ```text
   oxipng -o6 campaigns/tutorial/assets/portraits/full/kira.png
   # or:
   pngcrush -brute campaigns/tutorial/assets/portraits/full/kira.png
   ```

4. **Verify naming** — confirm the file stem (lowercased, underscores) matches
   the character's `portrait_id` (or `name` if `portrait_id` is empty).

---

## 7. The Placeholder Fallback

When a full portrait file is absent the engine renders a colored rectangle
(170 × 280 px) filled with a deterministic color derived from the
`portrait_key`, then overlays the character's **initials** (first letter of
each name token, up to two letters) in white at 48 pt.

For example, a character named **"Aldric Ironforge"** with no full portrait
gets a placeholder with the initials **AI**.

This means:
- You do not have to provide full portraits for every character before
  releasing a campaign — the placeholder is always shown as a clean fallback.
- You can add portraits incrementally without restarting the game; the engine
  rescans the directory each time a campaign is loaded.

---

## 8. Testing Your Portrait In-Game

1. Place the PNG in `<campaign_root>/assets/portraits/full/<key>.png`.
2. Launch the game and load your campaign.
3. Press `P` (or `1`–`6`) to open the Character Sheet.
4. The portrait should appear in the left column at 170 × 280 px.
5. If the colored placeholder still shows, check:
   - The filename stem matches the character's `portrait_id` (or `name`) after
     lowercasing and replacing spaces with underscores.
   - The file extension is `.png`, `.jpg`, or `.jpeg`.
   - The file is in `assets/portraits/full/` (not `assets/portraits/`).

---

## 9. Acceptance Checklist

- [ ] File placed in `<campaign_root>/assets/portraits/full/`
- [ ] Filename is lowercase with underscores, matching `portrait_id` (or `name`)
- [ ] Format: PNG-24 (RGBA), sRGB color space, 8 bits per channel
- [ ] Source resolution: 340 × 560 px (or 170 × 280 px minimum)
- [ ] Full figure visible — no feet or head clipped
- [ ] Relaxed ready stance, slight 3/4 view
- [ ] Rim lighting separates the figure from the dark background
- [ ] Lossless optimization applied (`oxipng` or `pngcrush`)
- [ ] Verified in-game: portrait appears in Character Sheet left column

---

## References

- Character Sheet rendering: `src/game/systems/character_sheet_ui.rs`
  — `render_single_view`, portrait area 170 × 280 px, `get_portrait_color`
  placeholder
- Full portrait loader: `src/game/systems/hud.rs`
  — `FullPortraitAssets`, `ensure_full_portraits_loaded`
- Asset path: `<campaign_root>/assets/portraits/full/<key>.png`
- HUD portrait guide (64 × 64): `docs/how-to/portrait_export_checklist.md`
- Per-character AI prompts (HUD): `docs/explanation/portrait_prompt.md`
- Tutorial character AI prompts: `docs/explanation/character_generation_prompt.md`
