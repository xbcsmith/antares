# How-To: Add Custom Fonts to a Campaign

This guide explains how to configure custom `.ttf` fonts for dialogue and
game menus in an Antares campaign. Both font fields are optional — you can
adopt a custom font for one UI element, both, or neither.

---

## Overview

Two fields in `config.ron` control which font the engine uses for in-game text:

- **`dialogue_font`** — the typeface rendered inside dialogue boxes when NPCs
  and events display text to the player.
- **`game_menu_font`** — the typeface used throughout the game menu screens
  (inventory, spells, character sheet, etc.).

Both fields are optional. Omitting a field (or setting it to `None`) preserves
the default Bevy engine font. There is no crash or visual degradation from
leaving either field unset — the engine falls back cleanly.

---

## Directory Layout

Font files must be placed inside a `fonts/` directory directly under your
campaign root. The engine resolves font paths relative to that root, so the
`fonts/` prefix in every path is mandatory.

```
campaigns/
└── my_campaign/
    ├── config.ron
    └── fonts/
        ├── my_dialogue_font.ttf
        └── my_menu_font.ttf
```

Place the `.ttf` files there before running the game or saving from the
Campaign Builder. The engine does not copy files automatically.

---

## config.ron Entry Format

Add a `fonts` key inside the top-level `GameConfig` struct using the
`FontConfig` helper type:

```ron
GameConfig(
    // ... other settings ...
    fonts: FontConfig(
        dialogue_font: Some("fonts/my_dialogue_font.ttf"),
        game_menu_font: Some("fonts/my_menu_font.ttf"),
    ),
    // ... other settings ...
)
```

To use the engine default for one element while customising the other, omit
the field entirely or set it to `None`:

```ron
fonts: FontConfig(
    dialogue_font: Some("fonts/my_dialogue_font.ttf"),
    // game_menu_font omitted — equivalent to None, uses engine default
),
```

Omitting the entire `fonts` key is also valid and uses both engine defaults.

---

## Validation Rules

The engine enforces the following rules when loading or saving font paths.
A path that violates any rule is rejected with a validation error:

- **Relative paths only** — the path must not begin with `/` or a Windows
  drive letter such as `C:\`. All paths are resolved relative to the campaign
  root.
- **No directory traversal** — `..` components are forbidden anywhere in the
  path.
- **Must start with `fonts/`** — the path must begin with the `fonts/` prefix.
  This ensures font files remain inside the designated campaign subdirectory.
- **Must end with `.ttf`** — only TrueType font files are accepted. Other
  extensions (`.otf`, `.woff`, etc.) are not supported.

---

## Fallback Behavior

If a configured font file is missing at runtime (the path passes validation
but the file is not present on disk):

- The engine emits a `warn!` log message identifying the missing path, for
  example: `warn!("Font asset not found: fonts/my_dialogue_font.ttf")`.
- Dialogue boxes and menus render using the Bevy default font.
- The game continues normally — there is no crash, freeze, or error screen.

The fallback is intentional so that a mistyped filename or a forgotten copy
does not break a play session.

---

## Authoring with the Campaign Builder

The **Config** tab in the Campaign Builder provides a graphical editor for
font settings. Follow these steps:

1. Open the Campaign Builder and select your campaign.
2. Navigate to the **Config** tab.
3. Scroll to the **Custom Fonts** section and expand it.
4. Type the relative path (e.g. `fonts/my_font.ttf`) in the **Dialogue Font**
   or **Game Menu Font** field.
5. Inline validation displays red error text immediately if the path violates
   any of the rules listed above (absolute path, traversal, wrong prefix, or
   wrong extension).
6. Click **Save** in the toolbar to write the updated `config.ron`.
7. Reload the campaign to confirm the value persists correctly.

Saving is blocked while any validation error is shown in red. Correct the
path before attempting to save.

---

## Testing In-Game

After placing the font file and saving `config.ron`:

1. Load the campaign in the game.
2. Open the game menu or trigger a dialogue event.
3. Confirm the custom font appears for the configured UI element.

**Troubleshooting checklist:**

- Check that the filename casing in `config.ron` matches the file on disk
  exactly. Font filenames on macOS and Linux are case-sensitive
  (`MyFont.ttf` and `myfont.ttf` are different files).
- Confirm the path in `config.ron` starts with `fonts/` and ends with `.ttf`.
- Verify the `.ttf` file exists at
  `<campaign_root>/fonts/<filename>.ttf` on disk.
- Check the game log for `warn!` messages that name the missing font asset;
  these appear at startup if a configured font file cannot be found.

---

## Acceptance Checklist

Use this checklist before releasing or sharing a campaign that uses custom
fonts:

- [ ] `.ttf` font file placed under `campaigns/<name>/fonts/`
- [ ] `config.ron` updated with valid `fonts: FontConfig(...)` entry
- [ ] Path starts with `fonts/` and ends with `.ttf`
- [ ] No absolute path or `..` traversal in the path
- [ ] Campaign Builder Config Editor shows no red validation errors
- [ ] Config saved successfully via the toolbar Save button
- [ ] Game loads campaign and custom font appears in dialogue or menu
- [ ] Missing font falls back gracefully (warn log, no crash)
