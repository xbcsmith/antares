# macOS Tray Icon Implementation Plan

<!-- SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

## Overview

Adds the Antares icon to the macOS Campaign Builder application in two layers.
Phase 1 sets the window and Dock icon by wiring `assets/icons/antares_tray.png`
into eframe's `ViewportBuilder::with_icon()` — a pure-Rust change that requires
no new dependencies. Phase 2 adds a real macOS menu-bar status item
(NSStatusItem) using the `tray-icon` crate so the Campaign Builder can live in
the top-right menu bar, survive window closure, and expose Show/Hide and Quit
actions.

## Current State Analysis

### Existing Infrastructure

| Asset / File                                   | Notes                                                                                                                                   |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `assets/icons/antares_tray.png`                | Source icon (1024×1024 full-colour, used as canonical input)                                                                            |
| `scripts/generate_icons.sh`                    | Swift + Python script; emits `assets/icons/generated/macos/tray_icon_1x.png` (22×22) and `tray_icon_2x.png` (44×44) from the source PNG |
| `sdk/campaign_builder/assets/antares_logo.png` | Existing SDK asset; not wired into the window icon                                                                                      |
| `sdk/campaign_builder/src/lib.rs` `run()`      | Builds `eframe::NativeOptions` / `ViewportBuilder` — **no icon set**                                                                    |
| `sdk/campaign_builder/Cargo.toml`              | `eframe = "0.33"` with `glow`, `default_fonts`, `wayland`, `x11`; `image = "0.25"` with `png` already present                           |

### Identified Issues

1. The Campaign Builder window and Dock entry show a generic system icon —
   `ViewportBuilder` is constructed without `.with_icon()`.
2. No macOS menu-bar status item exists; closing the window fully terminates the
   process.
3. The generated 22×22 / 44×44 assets from `generate_icons.sh` are never
   committed to the SDK assets folder or embedded in the binary.
4. `eframe` feature flags lack `macos-private-api` (not required for the icon,
   but documented here for completeness).

---

## Implementation Phases

### Phase 1: Window and Dock Icon

#### 1.1 Foundation Work — Copy Source Icon into SDK Assets

Copy `assets/icons/antares_tray.png` to
`sdk/campaign_builder/assets/antares_tray.png` so the SDK crate can embed it
with `include_bytes!` independent of the workspace root layout.

#### 1.2 Add Icon Module

Create `sdk/campaign_builder/src/icon.rs`.

Responsibilities:

- Embed the PNG at compile time:
  `const ICON_PNG: &[u8] = include_bytes!("../assets/antares_tray.png");`
- Decode with the already-available `image` crate:
  load RGBA8 pixels, return `width`, `height`, `rgba: Vec<u8>`.
- Expose one public function:
  `pub fn app_icon_data() -> Option<Arc<egui::IconData>>`
  - Returns `None` and logs a warning if the embedded bytes fail to decode
    (defensive; `include_bytes!` guarantees presence at compile time).
- Add the module declaration (`mod icon;`) to `lib.rs`.

`egui::IconData` fields: `rgba: Vec<u8>`, `width: u32`, `height: u32`.

#### 1.3 Wire Icon into `ViewportBuilder`

In `sdk/campaign_builder/src/lib.rs`, inside `run()`, call `app_icon_data()`
before constructing `NativeOptions` and pass the result to
`ViewportBuilder::with_icon()`:

```
let icon = icon::app_icon_data();

let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 720.0])
        .with_min_inner_size([800.0, 600.0])
        .with_title("Antares Campaign Builder")
        .with_icon(icon.unwrap_or_default()),
    ..Default::default()
};
```

On macOS, eframe's `AppTitleIconSetter` calls `NSApp.setApplicationIconImage_`
each frame until the Dock icon is updated; no additional work is needed.

#### 1.4 Testing Requirements

Add to `sdk/campaign_builder/src/icon.rs` under `#[cfg(test)]`:

| Test name                                           | What it verifies                                                                         |
| --------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `test_app_icon_data_returns_some`                   | `app_icon_data()` is `Some`                                                              |
| `test_app_icon_data_dimensions_non_zero`            | `width > 0` and `height > 0`                                                             |
| `test_app_icon_data_rgba_length_matches_dimensions` | `rgba.len() == (width * height * 4) as usize`                                            |
| `test_app_icon_data_is_valid_png`                   | Raw `ICON_PNG` bytes start with the PNG magic number `[137, 80, 78, 71, 13, 10, 26, 10]` |

All four tests must be purely unit tests (no Bevy `App`, no file I/O).

#### 1.5 Deliverables

- [ ] `sdk/campaign_builder/assets/antares_tray.png` — source icon copied into SDK
- [ ] `sdk/campaign_builder/src/icon.rs` — embed + decode module with 4 unit tests
- [ ] `sdk/campaign_builder/src/lib.rs` — `ViewportBuilder::with_icon()` wired in `run()`
- [ ] All quality gates pass (`cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo nextest run`)

#### 1.6 Success Criteria

- Campaign Builder Dock icon shows the Antares logo on macOS.
- Window title-bar icon shows the Antares logo.
- `cargo nextest run -p campaign_builder` reports 4 new passing icon tests.
- No new compiler warnings.

---

### Phase 2: macOS Menu-Bar Status Item

#### 2.1 Foundation Work — Generate and Commit Tray-Size Assets

Run `scripts/generate_icons.sh` (requires macOS with `swift` and `python3`)
to produce:

```
assets/icons/generated/macos/tray_icon_1x.png   # 22×22
assets/icons/generated/macos/tray_icon_2x.png   # 44×44
```

Copy both files to `sdk/campaign_builder/assets/icons/`:

```
sdk/campaign_builder/assets/icons/tray_icon_1x.png
sdk/campaign_builder/assets/icons/tray_icon_2x.png
```

These are the sizes macOS uses for menu-bar status items (1× for standard
displays, 2× for Retina).

#### 2.2 Add `tray-icon` Dependency

In `sdk/campaign_builder/Cargo.toml` add a platform-conditional dependency:

```
[target.'cfg(target_os = "macos")'.dependencies]
tray-icon = { version = "0.19", features = [] }
```

The `tray-icon` crate (Tauri ecosystem) wraps `NSStatusItem` for macOS,
`Shell_NotifyIcon` for Windows, and `libayatana-appindicator` for Linux. Only
the macOS path is required for this phase.

#### 2.3 Add Tray Module

Create `sdk/campaign_builder/src/tray.rs` gated with
`#![cfg(target_os = "macos")]`.

Responsibilities:

- Embed the 22×22 and 44×44 PNGs with `include_bytes!`.
- `pub fn build_tray_icon() -> tray_icon::TrayIcon`
  - Decode the 22×22 PNG to RGBA with the `image` crate.
  - Construct a `tray_icon::Icon` from the RGBA bytes.
  - Build a `tray_icon::menu::Menu` with one item:
    - `"Quit"` — ID `"quit"`
  - Return the constructed `TrayIcon` (caller must keep it alive for the
    duration of the process).
- `pub fn handle_tray_events()` — polls
  `tray_icon::menu::MenuEvent::try_recv()`;
  dispatches `"quit"` → `std::process::exit(0)`.
  Show/Hide window handling is deferred to Phase 3.

Add `#[cfg(target_os = "macos")] mod tray;` to `lib.rs`.

#### 2.4 Wire into `run()`

In `lib.rs::run()`, after `NativeOptions` is constructed and before
`eframe::run_native`:

```rust
#[cfg(target_os = "macos")]
let _tray = tray::build_tray_icon();
```

Inside `CampaignBuilderApp::update()` (the eframe frame callback), call
`tray::handle_tray_events()` once per frame, wrapped in
`#[cfg(target_os = "macos")]`. This keeps the menu-bar icon alive and
responsive without a separate thread.

The `_tray` binding must be held for the lifetime of `run_native`; dropping it
removes the status item from the menu bar.

#### 2.5 Testing Requirements

Add to `sdk/campaign_builder/src/tray.rs` under `#[cfg(test)]`:

| Test name                       | What it verifies                          |
| ------------------------------- | ----------------------------------------- |
| `test_tray_icon_1x_png_magic`   | `TRAY_ICON_1X` bytes start with PNG magic |
| `test_tray_icon_2x_png_magic`   | `TRAY_ICON_2X` bytes start with PNG magic |
| `test_tray_icon_1x_dimensions`  | Decoded width == 22, height == 22         |
| `test_tray_icon_2x_dimensions`  | Decoded width == 44, height == 44         |
| `test_tray_icon_1x_rgba_length` | `rgba.len() == 22 * 22 * 4`               |

All five tests decode pixel data only; no `NSApp` / `NSStatusItem` is touched
so they run on any platform under `#[cfg(test)]` without the macOS guard.

#### 2.6 Deliverables

- [ ] `scripts/generate_icons.sh` run; generated PNGs reviewed
- [ ] `sdk/campaign_builder/assets/icons/tray_icon_1x.png` (22×22)
- [ ] `sdk/campaign_builder/assets/icons/tray_icon_2x.png` (44×44)
- [ ] `sdk/campaign_builder/Cargo.toml` — `tray-icon` platform dep added
- [ ] `sdk/campaign_builder/src/tray.rs` — tray module with `build_tray_icon`, `handle_tray_events`, 5 unit tests
- [ ] `sdk/campaign_builder/src/lib.rs` — `_tray` binding in `run()`, `handle_tray_events()` call in `update()`
- [ ] All quality gates pass on macOS (`cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo nextest run`)

#### 2.7 Success Criteria

- Antares icon appears in the macOS menu bar when the Campaign Builder is
  running.
- Clicking **Quit** in the status menu terminates the process cleanly.
- `cargo nextest run -p campaign_builder` reports 5 new passing tray tests.
- Non-macOS builds compile without warnings (all tray code is gated on
  `cfg(target_os = "macos")`).
- Full-colour icon is used; no monochrome template variant is required.

---

### Phase 3: Show/Hide Window from Menu Bar

#### 3.1 Feature Work

Extend the tray menu with a `"Show Antares Campaign Builder"` / `"Hide"`
toggle item and wire it to the eframe window via a `std::sync::mpsc` channel
shared between the tray event handler and the egui update loop.

Key design:

- Add a `TrayCommand` enum in `tray.rs`: `enum TrayCommand { ShowWindow, HideWindow, Quit }`.
- Change `build_tray_icon()` to also return a `std::sync::mpsc::Receiver<TrayCommand>`.
- Extend the tray menu with `"Show Antares Campaign Builder"` — ID `"show"` and
  `"Hide"` — ID `"hide"`.
- `handle_tray_events()` now sends `TrayCommand` values over the channel instead
  of calling `std::process::exit` directly (Quit remains an exception for
  immediate termination).

#### 3.2 Integrate with eframe Window

`CampaignBuilderApp` stores the `Receiver<TrayCommand>` (behind
`#[cfg(target_os = "macos")]`).

In `update()`, drain the receiver each frame:

- `TrayCommand::ShowWindow` → call `ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus)` and
  `ui.ctx().send_viewport_cmd(egui::ViewportCommand::Visible(true))`.
- `TrayCommand::HideWindow` → `ui.ctx().send_viewport_cmd(egui::ViewportCommand::Visible(false))`.
- `TrayCommand::Quit` → `ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close)`.

`egui::ViewportCommand` is available in egui 0.27+ and is present in the
project's egui 0.33 dependency.

#### 3.3 Testing Requirements

Add to `sdk/campaign_builder/src/tray.rs` under `#[cfg(test)]`:

| Test name                                      | What it verifies                                                                     |
| ---------------------------------------------- | ------------------------------------------------------------------------------------ |
| `test_tray_command_show_is_distinct_from_hide` | `TrayCommand::ShowWindow != TrayCommand::HideWindow` (requires `PartialEq` derive)   |
| `test_tray_command_channel_send_recv`          | A `TrayCommand::ShowWindow` sent over an `mpsc` channel is received without blocking |

#### 3.4 Deliverables

- [ ] `sdk/campaign_builder/src/tray.rs` — `TrayCommand` enum, updated `build_tray_icon()` signature, Show/Hide menu items, channel-based dispatch
- [ ] `sdk/campaign_builder/src/lib.rs` — `CampaignBuilderApp` stores `Receiver<TrayCommand>`; `update()` drains it and sends `ViewportCommand`
- [ ] All quality gates pass (`cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo nextest run`)

#### 3.5 Success Criteria

- Clicking **Show Antares Campaign Builder** in the menu bar raises and focuses
  the window when it is hidden or minimised.
- Clicking **Hide** removes the window from view without terminating the process.
- The Antares icon remains in the menu bar while the window is hidden.
- `cargo nextest run -p campaign_builder` reports 2 new passing tray tests.
- Non-macOS builds compile without warnings.
