# Map Editor Debug Notes (temporary, for the next session)

These notes were prepared after investigating the issue where the Maps Editor "Edit" view could appear blank
(showing toolbars only — no map grid, no inspector). This file includes reproduction steps, how to enable debug logging,
what debug output to copy into tickets, immediate steps I took, and recommended follow-up actions and permanent fixes.

---

## TL;DR

- Repro: Editor → Maps → select map → click Edit → toolbar shows but the grid and inspector area are blank.
- Quick toggle: Use CLI `--debug` or env var `ANTARES_DEBUG=1` to enable debug logging.
- Temporary debug UI added (red border + debug label) to help determine whether the map grid is allocated but not painting.
- Next step: run the app with debug enabled and copy the console output; paste logs here and I'll fix the root cause and remove debug scaffolding.

---

## How to reproduce (baseline)

1. From terminal in project directory:
   - `cargo run --package campaign_builder -- --debug`
     - Or: `ANTARES_DEBUG=1 cargo run --package campaign_builder`
2. Open the Campaign Builder UI (the app will open).
3. Click "Maps" in the left editor list
4. Select a map (e.g., "Town Square")
5. Click "Edit" (or double click map to edit)
6. Observe the UI:
   - Toolbar and control rows should be visible.
   - If the grid or inspector is blank (completely dark area), record the issue and console logs.

---

## Debugging control (CLI + ENV)

- CLI flag options (processed by `Logger::from_args()`):
  - `--debug` or `-d` → sets `LogLevel::Debug`
  - `--verbose` or `-v` → sets `LogLevel::Verbose`
  - `--quiet` or `-q` → sets `LogLevel::Warn`
- Environment variable:
  - `ANTARES_DEBUG`:
    - `1`, `true`, `on`, or `yes` → enable debug
    - `0`, `false`, `off`, `no` → disable debug
- The code includes a `logging::debug_enabled()` function, which checks both the CLI arguments and the `ANTARES_DEBUG` environment variable. When enabled, the app prints debug logs to stderr.

---

## Temporary debug output added

(Only printed/visible when debug is enabled)

- Map editor layout / zoom diagnostics:
  - `[MapsEditor] total_width: <...> left_width: <...> inspector_min_width: <...> panel_height: <...> zoom_level: <...>`
  - `[MapsEditor][FIT] avail: <...>, map_px: <W>x<H>, fit_zoom: <...>, result: <...>, ...`
- Map grid diagnostics:
  - `MapGridWidget: map <W>x<H> map_px <w_px>x<h_px> avail Vec2(<Ax>,<Ay>) tile_size <tile> computed width <W> height <H>`
- Visual aid:
  - A temporary red border is drawn around the entire grid painter allocation so you can visually confirm it is being allocated.
  - A small UI debug label in the left column shows: `Map: <W>x<H> Zoom: <xx>% left_width: <left_width>`. (Shown only when debug is on)

---

## What to look for (console & UI)

- After the steps above, check the debug output lines:
  - If you see `[MapsEditor] total_width: ...` and `MapGridWidget: ...`: the left column is being allocated and your painting logic is being called.
  - If you see `MapGridWidget: ...` with `map_px` sizes, but no tiles, inspect `tile_size` (should be >= 8 px). Tiles may be painted with colors similar to background.
  - If you do not see any of these lines: the left UI region or the map drawing closure did not execute — probably a layout call was skipped, or `active_editor` was None at that time.
  - If left width is 0 or tiny: something prevents the left column from sizing correctly — the calculated `left_width` or `inspector_min_width` clobbered it.

---

## Immediate fixes implemented (temporary) — Completed

- `MapGridWidget`:
  - Ensures the painter area is at least the available UI size (avoids clipping and ensures the entire canvas is painted).
  - Centers the map within the painter and adjusts click coordinates accordingly so tool interactions map to the correct tile coordinates.
  - Removed the temporary red border used for visual debugging (grid border and debug UI toggles were left conditional on `ANTARES_DEBUG` and are now removed/guarded).
- `MapsEditor`:
  - Replaced `TwoColumnLayout::show(...)` usage with `TwoColumnLayout::show_split(...)` to ensure both left and right closures are invoked and continue to render properly.
  - Left column width is computed using a shared helper `compute_left_column_width(...)` to ensure the inspector min width and left-column max ratio are respected consistently across editors.
  - Fit calculation now uses the actual left panel available area (not absolute guesses/fudges). It respects min/max tile sizes and preserves user zoom when `Auto Fit` isn't active.
  - `Auto Fit on Resize` (`auto_fit_on_resize`) remains enabled by default and is toggleable. When enabled, it recomputes the map fit to the left panel on window resize events.
- Items Editor:
  - Now uses the shared left width calculation so the list won't exceed the available area and the inspector won't be clipped. We preserved the items editor's "feel" while improving layout consistency across editors.

---

## Quick fallback / safety improvements (done)

- Added the `compute_left_column_width()` helper that centralizes the left-column clamp logic:
  - Soft `MIN_SAFE_LEFT_COLUMN_WIDTH` (250 px) is enforced only when the available width permits it (this prevents a contradictory clamp when the whole app width is too narrow).
  - `left_column_max_ratio` is used to limit how large the left column can be relative to the window and is configurable via `DisplayConfig`. Default remains 0.72 but it may be tuned after UX review.
- The left column is now clamped using a robust routine that:
  - Respects inspector min width (configurable).
  - Respects max left ratio.
  - Avoids forcing a minimum left width that would entirely hide the inspector in very narrow windows.
- Grid stroke color is white (improved visibility in dark theme); tile borders and grid stroke thickness were adjusted to be visually clear and consistent.

---

## Known temporary debug changes (do not forget)

- Debug prints remain guarded by `Logger::debug_enabled()` (CLI flag `--debug` or `ANTARES_DEBUG`) and will not run by default.
- The temporary red border that was drawn around the map painter for visual confirmation was removed. Any remaining visual aids are now minimized and fully gated by debug flags.
- Final cleanup (Option D): All debug-only statements and scaffolding will be removed once we receive final confirmation that layout behavior is stable across window sizes (small/medium/large).

---

## Horizontal Spacing / Inspector Column Cutoff (Final Notes)

- Observation:
  - The only remaining visible layout issue is that the left map column can be so wide (based on computed left_width) that the inspector (right) column is partially or fully cut off in certain default or limited window widths.
- Root cause:
  - The editor computed `left_width` as `total_width - inspector_min_width - margin` or `(total_width * 0.6)` fallback, depending on available width.
  - For certain default window sizes the resulting `left_width` may still make the right column appear narrow or cut off because of theme spacing and UI content widths.
- Short-term measures applied (we implemented):
  - Left column `left_width` now has a safe minimum (250 px) and a conservative maximum ratio clamp to 72% of `total_width`.
  - The `TwoColumnLayout` usage for the maps editor now uses `show_split` to ensure both left and right columns are invoked and the map & inspector are rendered.
  - `Auto-Fit On Resize` toggle was added to allow map contents to scale with window size (enabled by default).
- Recommended next steps:
  1. Tune the maximum left column ratio value (72%) as necessary after user review: choose a value that provides enough detail to the map while leaving adequate space for the inspector (e.g., 70% or 65%).
  2. Optionally provide an inspector min width property in the `TwoColumnLayout` or at the `MapsEditor` level so the layout enforces user-defined inspector width preferences instead of using a computed default min.
  3. Add UI tests / layout assertions so the editor layout does not allow the inspector to be clipped for common screen sizes:
     - Tests for small, medium, and large window sizes that assert inspector min width is respected and content is scrollable.
  4. Add a toolbar wrap or overflow menu for the toolbar to avoid the toolbar being clipped on very small windows (UX improvement).
  5. If you prefer a stricter visual fallback: implement a faint checkerboard background or tile borders when the tile size is tiny, and add an explicit `left_width` config setting if desired.
- How to verify quickly:
  - Run app with `ANTARES_DEBUG=1`, open maps edit, and resize the main window to default width and smaller widths, verify:
    - Inspector column is still visible or gets a scroll bar rather than being cut off.
    - Map left column resizes with auto-fit toggled ON/OFF as expected.
  - Use `maps_editor_debug_repro2.txt` or equivalent debug captures to confirm `left_width` and `map_response.rect` changes on resize.

---

## Debugging steps for you (recommended)

1. Start the app with debug enabled:
   - `ANTARES_DEBUG=1 cargo run --package campaign_builder -- --debug`
2. Reproduce the issue:
   - Maps Editor -> select map -> click "Edit"
   - Resize the main window to default width and smaller widths.
3. Capture logs and screenshots:
   - `[MapsEditor] total_width: ... left_width: ... inspector_min_width: ... panel_height: ...`
   - `[MapsEditor] left_ui.available_size: ...`
   - `[MapsEditor] Adding MapGridWidget: map ... tile_size ...`
   - `[MapsEditor][FIT] ... auto_fit: true|false`
4. Paste the logs and an example screenshot where the inspector is trimmed — I will fine-tune the clamp accordingly.

---

## Next tasks I will perform after you confirm (Option D)

Completed:

- Implemented `EditorToolbar` wrapping (so toolbar items gracefully flow to multiple lines), `MapGridWidget` painting tolerances, and `compute_left_column_width()` logic.
- Updated `MapsEditor` and `ItemsEditor` to use the shared left width logic.
- Added unit tests to verify `compute_left_column_width` behavior.

Remaining/Follow-ups:

1. Tune `left_column_max_ratio` default if you'd like a different balance (e.g., 0.70 or 0.65 vs 0.72). This can be adjusted in the `DisplayConfig`.
2. Expose a configurable `inspector_min_width` option (via `DisplayConfig`) and/or add a per-editor inspector min width preference GUI so users can persist their preferred layout.
3. Optionally implement a toolbar overflow "More" menu that hides less-used actions when the width is very constrained (alternative to wrapping).
4. Add UI/Integration tests to assert:
   - Inspector min width is preserved (scrolling vs clipping) on small screens.
   - `show_split` calls both left and right closures.
   - `Auto Fit` toggles recompute scaling on window resize and preserves user zoom when appropriate.
5. After confirmation, remove all debug prints and visual debugging scaffolding (Option D); this includes removing logging or debug marks that were only intended for diagnosis.

If you want me to proceed with any of the optional follow-ups (72% default clamp, inspector min width setting, toolbar overflow fix), say which one(s) and I’ll implement them (and follow with tests & docs). If you confirm the current left-width clamp and auto-fit toggle behavior are what you desire, I’ll remove debug scaffolding (Option D) next.
