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
actions. Phase 4 (this addendum) extends the tray icon to Linux using GTK via a
dedicated thread, so the Campaign Builder gains a system tray on GNOME, KDE,
and XFCE without affecting the macOS or Windows builds.

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

---

### Phase 4: Linux GTK System Tray (Addendum)

#### 4.1 Foundation Work — System Package Prerequisites

`tray-icon` on Linux requires GTK and `libappindicator` (or the modern
`libayatana-appindicator`) as OS-level packages. Add the following to the
project `README.md` under a **Linux Prerequisites** heading:

| Distro family   | Command                                                         |
| --------------- | --------------------------------------------------------------- |
| Arch / Manjaro  | `pacman -S gtk3 xdotool libappindicator-gtk3`                   |
| Debian / Ubuntu | `sudo apt install libgtk-3-dev libxdo-dev libappindicator3-dev` |

The `libayatana-appindicator` variant (`libayatana-appindicator-gtk3` /
`libayatana-appindicator3-dev`) is an acceptable substitute on distros where
`libappindicator` is no longer packaged.

The existing tray assets in `sdk/campaign_builder/assets/icons/` are
platform-agnostic PNGs and are reused directly — no new art or generation
step is required.

#### 4.2 Expand `tray-icon` Dependency in `Cargo.toml`

The existing macOS dep in `sdk/campaign_builder/Cargo.toml` is kept unchanged.
Add a separate Linux entry beneath it:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
tray-icon = { version = "0.19", features = [] }

[target.'cfg(target_os = "linux")'.dependencies]
tray-icon = { version = "0.19", features = ["gtk"] }
gtk = "0.18"
```

The `"gtk"` feature flag on the Linux dep is **required** — without it
`tray-icon` does not compile the GTK backend. The macOS dep keeps
`features = []` because the GTK feature is Linux-only.

`gtk 0.18` is already present in `Cargo.lock` as a transitive dependency of
`tray-icon`; the explicit entry pins the version and allows calling
`gtk::init()` and `gtk::main()` without `unsafe` casting.

`crossbeam-channel` does **not** need to be added explicitly — it is already
a transitive dependency of `tray-icon` and backs `MenuEvent::receiver()`
internally.

#### 4.3 Broaden the Tray Module Platform Gate

`sdk/campaign_builder/src/tray.rs` currently opens with:

```rust
#![cfg(target_os = "macos")]
```

Change this to:

```rust
#![cfg(any(target_os = "macos", target_os = "linux"))]
```

All existing code in the module (`TRAY_ICON_1X`, `TRAY_ICON_2X`,
`build_tray_icon`, `handle_tray_events`, `TrayCommand`, `TRAY_CMD_TX`) uses
the cross-platform `tray-icon` API and compiles unchanged on Linux. No
modifications to the body of any existing function are required.

Update `sdk/campaign_builder/src/lib.rs` in the same way:

```rust
// before:
#[cfg(target_os = "macos")]
pub mod tray;

// after:
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod tray;
```

#### 4.4 Add `spawn_gtk_tray_thread()` to `tray.rs`

On Linux, `tray-icon` requires both the `TrayIcon` creation and GTK event
processing to happen on the thread that runs the GTK main loop. eframe uses
winit for window management (Wayland or X11), which is incompatible with
GTK's main loop on the same thread. The solution is a dedicated GTK thread,
matching the pattern from the official `tray-icon` examples.

Add the following function to `sdk/campaign_builder/src/tray.rs`, compiled
only on Linux:

```rust
/// Spawns a dedicated GTK thread that initialises GTK, creates the
/// menu-bar status item, and runs `gtk::main()`.
///
/// **Must be called before `eframe::run_native`** so that the GTK event
/// loop is running before the first eframe frame is painted.
///
/// GTK signal handlers registered by `tray-icon` push [`MenuEvent`] values
/// into the crossbeam channel backing [`tray_icon::menu::MenuEvent::receiver()`].
/// The eframe `update()` loop drains that channel directly each frame — no
/// intermediate `TrayCommand` channel is needed on Linux.
///
/// # Panics
///
/// Panics if GTK cannot be initialised (no display server available).  A
/// headless server should not run the Campaign Builder.
#[cfg(target_os = "linux")]
pub fn spawn_gtk_tray_thread() {
    std::thread::Builder::new()
        .name("gtk-tray".to_string())
        .spawn(|| {
            gtk::init().expect("failed to initialise GTK for tray icon");
            // build_tray_icon() returns (TrayIcon, Receiver<TrayCommand>).
            // The Receiver is intentionally dropped here — on Linux, update()
            // polls MenuEvent::receiver() directly rather than going through
            // the TrayCommand channel used on macOS.
            let (_tray, _rx) = build_tray_icon();
            // Blocks this thread; GTK callbacks fire and populate
            // MenuEvent::receiver() as menu items are clicked.
            gtk::main();
        })
        .expect("failed to spawn gtk-tray thread");
}
```

#### 4.5 Wire into `run()` and `update()`

**`sdk/campaign_builder/src/lib.rs` — `run()`**

Add the Linux GTK thread spawn immediately after the existing macOS tray
binding, before `eframe::run_native`:

```rust
// Existing macOS binding (unchanged):
#[cfg(target_os = "macos")]
let (_tray, tray_cmd_rx) = tray::build_tray_icon();

// New Linux GTK thread (fire-and-forget; no binding needed):
#[cfg(target_os = "linux")]
tray::spawn_gtk_tray_thread();
```

No receiver is stored in `CampaignBuilderApp` for Linux — the event source is
`MenuEvent::receiver()`, polled directly in `update()`. The `tray_cmd_rx`
field on `CampaignBuilderApp` remains macOS-only and requires no changes.

**`sdk/campaign_builder/src/lib.rs` — `update()`**

Add a Linux block immediately after the existing macOS tray drain block:

```rust
// Existing macOS block (unchanged):
#[cfg(target_os = "macos")]
tray::handle_tray_events();

#[cfg(target_os = "macos")]
if let Some(ref rx) = self.tray_cmd_rx {
    while let Ok(cmd) = rx.try_recv() { /* ... */ }
}

// New Linux block:
// On Linux, GTK callbacks on the gtk-tray thread populate
// MenuEvent::receiver() directly; poll it here without an intermediate
// TrayCommand channel.
#[cfg(target_os = "linux")]
while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
    if event.id == tray::MENU_ID_SHOW {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    } else if event.id == tray::MENU_ID_HIDE {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    } else if event.id == tray::MENU_ID_QUIT {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}
```

`MENU_ID_SHOW`, `MENU_ID_HIDE`, and `MENU_ID_QUIT` are the existing `&str`
constants in `tray.rs`; change their visibility from `const` to `pub const`
so the Linux block in `lib.rs` can reference them.

#### 4.6 macOS vs. Linux Event Dispatch Comparison

| Concern             | macOS (Phases 2–3)                                                 | Linux (Phase 4)                                                |
| ------------------- | ------------------------------------------------------------------ | -------------------------------------------------------------- |
| Event loop          | macOS run loop; `TrayIcon` on main thread                          | GTK main loop on dedicated `"gtk-tray"` OS thread              |
| `TrayIcon` lifetime | `NonSend`-equivalent: `_tray` binding in `run()`                   | Local `_tray` binding on the GTK thread's stack                |
| Event polling       | `handle_tray_events()` called in `update()` each frame             | `MenuEvent::receiver().try_recv()` polled in `update()`        |
| Command dispatch    | `OnceLock<SyncSender>` → `TrayCommand` channel → `update()` drains | GTK callbacks → crossbeam channel → `update()` drains directly |
| `ViewportCommand`   | Issued from `TrayCommand` match in `update()`                      | Issued inline from `MenuEvent` match in `update()`             |
| System dependencies | None (macOS frameworks linked automatically)                       | `libgtk-3-dev`, `libxdo-dev`, `libappindicator3-dev`           |

#### 4.7 Testing Requirements

No new test functions are required. Broadening the module gate to
`any(target_os = "macos", target_os = "linux")` means all seven existing
tray tests now compile and run on Linux:

| Existing test                                  | Linux coverage                                      |
| ---------------------------------------------- | --------------------------------------------------- |
| `test_tray_icon_1x_png_magic`                  | Verifies `TRAY_ICON_1X` embed is valid PNG on Linux |
| `test_tray_icon_2x_png_magic`                  | Verifies `TRAY_ICON_2X` embed is valid PNG on Linux |
| `test_tray_icon_1x_dimensions`                 | Decoded 22×22 on Linux                              |
| `test_tray_icon_2x_dimensions`                 | Decoded 44×44 on Linux                              |
| `test_tray_icon_1x_rgba_length`                | RGBA buffer length on Linux                         |
| `test_tray_command_show_is_distinct_from_hide` | `TrayCommand` enum distinctness on Linux            |
| `test_tray_command_channel_send_recv`          | mpsc channel round-trip on Linux                    |

All seven tests are pure PNG-decode or channel tests; no GTK initialisation
or display server is required, so they run in any Linux CI environment.

#### 4.8 Deliverables

- [ ] `sdk/campaign_builder/Cargo.toml` — Linux `tray-icon` dep with
      `features = ["gtk"]` and `gtk = "0.18"` added under
      `[target.'cfg(target_os = "linux")'.dependencies]`
- [ ] `README.md` — Linux system package prerequisites documented
- [ ] `sdk/campaign_builder/src/tray.rs` — module gate broadened to
      `any(target_os = "macos", target_os = "linux")`; `spawn_gtk_tray_thread()`
      added under `#[cfg(target_os = "linux")]`; `MENU_ID_SHOW`,
      `MENU_ID_HIDE`, `MENU_ID_QUIT` changed to `pub const`
- [ ] `sdk/campaign_builder/src/lib.rs` — `pub mod tray` gate broadened;
      `tray::spawn_gtk_tray_thread()` call added in `run()` under
      `#[cfg(target_os = "linux")]`; Linux `MenuEvent` drain block added in
      `update()` under `#[cfg(target_os = "linux")]`
- [ ] All quality gates pass on Linux (`cargo fmt`, `cargo check`,
      `cargo clippy -D warnings`, `cargo nextest run`)

#### 4.9 Success Criteria

- Antares icon appears in the Linux system tray (GNOME, KDE, XFCE) when the
  Campaign Builder is running with `libappindicator` or
  `libayatana-appindicator` installed.
- Clicking **Show Antares Campaign Builder** raises and focuses the window.
- Clicking **Hide** hides the window; the tray icon remains visible.
- Clicking **Quit** closes the Campaign Builder cleanly via
  `ViewportCommand::Close`.
- `cargo nextest run -p campaign_builder` reports all 7 existing tray tests
  passing on Linux with no new failures.
- macOS build is unaffected; all Phase 1–3 behaviour is unchanged.
- Windows and other non-Linux/non-macOS builds compile without warnings.
