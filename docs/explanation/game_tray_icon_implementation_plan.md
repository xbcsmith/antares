# Game Tray Icon Implementation Plan

## Overview

Adds the Antares icon to the game binary (`src/bin/antares.rs`) in three
layers, mirroring the Campaign Builder implementation but adapted for Bevy's
plugin and windowing architecture. Phase 1 sets the window and Dock icon by
wiring `assets/icons/antares_icon.png` into a Bevy startup system via
`WinitWindows` — a pure-Rust change with no new dependencies. Phase 2 adds a
macOS menu-bar status item (`NSStatusItem`) via the `tray-icon` crate, surfaced
as a `TrayPlugin` registered in `AntaresPlugin`. Phase 3 extends the tray menu
with Show/Hide window toggle wired to Bevy's `Window::visible` field through an
`mpsc` command channel.

---

## Current State Analysis

### Existing Infrastructure

| Asset / File                               | Notes                                                                                                                                                         |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `assets/icons/antares_icon.png`            | Main game icon (full-colour); not wired into the Bevy window                                                                                                  |
| `assets/icons/antares_tray.png`            | 1024×1024 source tray icon (canonical input for icon generation)                                                                                              |
| `assets/icons/game/macos/tray_icon_1x.png` | Pre-generated 22×22 PNG — ready to embed for macOS menu-bar use                                                                                               |
| `assets/icons/game/macos/tray_icon_2x.png` | Pre-generated 44×44 PNG — ready to embed for Retina displays                                                                                                  |
| `src/bin/antares.rs` `main()`              | Constructs `WindowPlugin` / `Window` — **no icon set**                                                                                                        |
| `Cargo.toml` root                          | `bevy = "0.17"`, `image = { version = "0.25", features = ["png"] }` present; no `tray-icon` dep; `wayland-client` and `wayland-sys` already present for Linux |
| `sdk/campaign_builder/Cargo.toml`          | Reference: `tray-icon = { version = "0.19" }` under `[target.'cfg(target_os = "macos")'.dependencies]`                                                        |
| `assets/icons/game/macos/tray_icon_1x.png` | 22×22 authoritative PNG; reused for Linux (same format, same size)                                                                                            |
| `assets/icons/game/macos/tray_icon_2x.png` | 44×44 authoritative PNG; reused for Linux                                                                                                                     |

### Identified Issues

1. The game window and macOS Dock entry show a generic system icon — `Window`
   is constructed in `main()` without setting a window icon via `WinitWindows`.
2. No macOS menu-bar status item exists for the game; closing the window fully
   terminates the process.
3. The authoritative 22×22 / 44×44 game tray assets in
   `assets/icons/game/macos/` are never embedded or used.
4. The `tray-icon` crate (already used by `sdk/campaign_builder`) is absent
   from the root `Cargo.toml`.
5. No Linux system tray is implemented; `tray-icon` on Linux requires a
   dedicated GTK thread separate from Bevy's winit event loop.

---

## Implementation Phases

### Phase 1: Window and Dock Icon

#### 1.1 Foundation Work — Icon Module

Create `src/game/systems/window_icon.rs`.

Responsibilities:

- Embed the game icon at compile time:
  `const ICON_PNG: &[u8] = include_bytes!("../../../assets/icons/antares_icon.png");`
- Decode with the already-available `image` crate: load RGBA8 pixels, return
  `width`, `height`, `rgba: Vec<u8>`.
- Expose one public function:
  `pub fn load_icon() -> Option<winit::window::Icon>`
  - Decodes `ICON_PNG` to RGBA8 via `image::load_from_memory`.
  - Constructs `winit::window::Icon::from_rgba(rgba, width, height)`.
  - Returns `None` and logs a warning on decode failure (defensive; bytes are
    compile-time verified via `include_bytes!`).

Add `pub mod window_icon;` to `src/game/systems/mod.rs`.

`winit::window::Icon` is already transitively available through
`bevy::winit`; no new dependency is required.

#### 1.2 Add Window Icon Startup System

Create `pub struct WindowIconPlugin` in `src/game/systems/window_icon.rs`.

The plugin registers one `Startup` system:

```text
fn set_window_icon(
    windows: NonSendMut<bevy::winit::WinitWindows>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
)
```

The system:

1. Resolves the primary `winit::window::Window` via
   `WinitWindows::get_window(entity)`.
2. Calls `window_icon::load_icon()`.
3. Calls `primary.set_window_icon(icon)` on the `winit` window handle.

`WinitWindows` is a `NonSend` resource — the system **must** use
`NonSendMut<WinitWindows>` and run on the main thread. Bevy schedules
`NonSend` startup systems on the main thread automatically.

#### 1.3 Wire Plugin into `AntaresPlugin`

In `src/bin/antares.rs` `AntaresPlugin::build`, add:

```text
app.add_plugins(WindowIconPlugin);
```

No changes to the `Window` struct initialisation in `main()` are needed —
the startup system handles it after Bevy has created the winit window.

#### 1.4 Testing Requirements

Add to `src/game/systems/window_icon.rs` under `#[cfg(test)]`:

| Test name                                      | What it verifies                                                          |
| ---------------------------------------------- | ------------------------------------------------------------------------- |
| `test_icon_png_magic_bytes`                    | `ICON_PNG` bytes start with PNG magic `[137, 80, 78, 71, 13, 10, 26, 10]` |
| `test_icon_png_dimensions_non_zero`            | Decoded image `width > 0` and `height > 0`                                |
| `test_icon_png_rgba_length_matches_dimensions` | `rgba.len() == (width * height * 4) as usize`                             |

All three tests use `image::load_from_memory` only; no Bevy `App`,
`WinitWindows`, or file I/O is touched, so they run in any environment.

#### 1.5 Deliverables

- [ ] `src/game/systems/window_icon.rs` — `ICON_PNG` embed, `load_icon()`,
      `WindowIconPlugin` startup system, 3 unit tests
- [ ] `src/game/systems/mod.rs` — `pub mod window_icon;` added
- [ ] `src/bin/antares.rs` — `app.add_plugins(WindowIconPlugin)` in
      `AntaresPlugin::build`
- [ ] All quality gates pass (`cargo fmt`, `cargo check`, `cargo clippy
-D warnings`, `cargo nextest run`)

#### 1.6 Success Criteria

- Game Dock icon shows the Antares logo on macOS.
- Game window title-bar icon shows the Antares logo.
- `cargo nextest run -p antares` reports 3 new passing window icon tests.
- No new compiler warnings.

---

### Phase 2: macOS Menu-Bar Status Item

#### 2.1 Foundation Work — Tray Assets

The authoritative 22×22 and 44×44 PNGs are committed and ready to embed:

```text
assets/icons/game/macos/tray_icon_1x.png   # 22×22 — authoritative game tray icon
assets/icons/game/macos/tray_icon_2x.png   # 44×44 — authoritative Retina variant
```

No generation or conversion step is required. These files are the correct art
and are embedded directly with `include_bytes!` in Phase 2.3.

#### 2.2 Add `tray-icon` Dependency

In the root `Cargo.toml` add a platform-conditional dependency for macOS only
at this phase. Linux support is added in Phase 4, which also expands this gate:

```text
[target.'cfg(target_os = "macos")'.dependencies]
tray-icon = { version = "0.19", features = [] }
```

This keeps non-macOS/non-Linux builds unaffected. The `cfg` gate is broadened
to `any(target_os = "macos", target_os = "linux")` in Phase 4.

#### 2.3 Add Tray Module and Plugin

Create `src/game/systems/tray.rs` gated with
`#![cfg(target_os = "macos")]`.

Responsibilities:

- Embed the 22×22 and 44×44 PNGs at compile time via `include_bytes!`.
- `pub fn build_tray_icon() -> tray_icon::TrayIcon`
  - Decodes the 22×22 PNG to RGBA8 with `image::load_from_memory`.
  - Constructs `tray_icon::Icon::from_rgba(rgba, width, height)`.
  - Builds a `tray_icon::menu::Menu` with one item: **Quit** (ID `"quit"`).
  - Returns the `TrayIcon` (caller keeps it alive for the process lifetime).
- `pub fn handle_tray_events_system(mut exit: EventWriter<AppExit>)` — a
  Bevy system that polls `tray_icon::menu::MenuEvent::receiver().try_recv()`
  and dispatches `"quit"` → `exit.send(AppExit::Success)` for a clean Bevy
  shutdown. Show/Hide handling is deferred to Phase 3.
- `pub struct TrayPlugin` — a Bevy `Plugin` that:
  - In a `Startup` system: calls `build_tray_icon()` and inserts the returned
    `TrayIcon` as a `NonSend` resource (so it stays alive for the process).
  - In an `Update` system: runs `handle_tray_events_system` once per frame.

Add `#[cfg(target_os = "macos")] pub mod tray;` to
`src/game/systems/mod.rs`.

#### 2.4 Wire Plugin into `AntaresPlugin`

In `src/bin/antares.rs` `AntaresPlugin::build`, add — wrapped in a cfg gate:

```text
#[cfg(target_os = "macos")]
app.add_plugins(tray::TrayPlugin);
```

`TrayPlugin` must be registered **after** `DefaultPlugins` so that the
winit event loop is already running when the startup system fires.

The `TrayIcon` is inserted as a `NonSend` resource so it is held by the
Bevy world for the lifetime of the app — equivalent to the `let _tray`
binding used in the Campaign Builder.

#### 2.5 Testing Requirements

Add to `src/game/systems/tray.rs` under `#[cfg(test)]`:

| Test name                       | What it verifies                          |
| ------------------------------- | ----------------------------------------- |
| `test_tray_icon_1x_png_magic`   | `TRAY_ICON_1X` bytes start with PNG magic |
| `test_tray_icon_2x_png_magic`   | `TRAY_ICON_2X` bytes start with PNG magic |
| `test_tray_icon_1x_dimensions`  | Decoded width == 22, height == 22         |
| `test_tray_icon_2x_dimensions`  | Decoded width == 44, height == 44         |
| `test_tray_icon_1x_rgba_length` | `rgba.len() == 22 * 22 * 4`               |

No `NSApp` / `NSStatusItem` is touched; tests are pure data decoding and
run in any environment where the crate compiles (macOS only, due to the
file-level `#![cfg(target_os = "macos")]` gate).

#### 2.6 Deliverables

- [ ] Root `Cargo.toml` — `tray-icon = "0.19"` under
      `[target.'cfg(target_os = "macos")'.dependencies]`
- [ ] `src/game/systems/tray.rs` — `build_tray_icon()`, `handle_tray_events()`,
      `TrayPlugin` (startup + update systems), 5 unit tests
- [ ] `src/game/systems/mod.rs` — `#[cfg(target_os = "macos")] pub mod tray;`
      added
- [ ] `src/bin/antares.rs` — `app.add_plugins(tray::TrayPlugin)` in
      `AntaresPlugin::build`, cfg-gated
- [ ] All quality gates pass on macOS (`cargo fmt`, `cargo check`,
      `cargo clippy -D warnings`, `cargo nextest run`)

#### 2.7 Success Criteria

- Antares icon appears in the macOS menu bar while the game is running.
- Clicking **Quit** in the status menu terminates the process cleanly.
- `cargo nextest run -p antares` reports 5 new passing tray tests.
- Non-macOS builds compile without warnings (all tray code is
  `cfg(target_os = "macos")`-gated).
- Full-colour icon is used; no monochrome template variant is required.
- Clicking **Quit** triggers a clean Bevy shutdown via `AppExit::Success`
  (no `process::exit`).

---

### Phase 3: Show/Hide Window from Menu Bar

#### 3.1 Feature Work — TrayCommand Enum and Channel

Extend `src/game/systems/tray.rs` with a channel-based dispatch mechanism,
mirroring the Campaign Builder Phase 3 design but adapted for Bevy resources.

Key design:

- Add `TrayCommand` enum: `enum TrayCommand { ShowWindow, HideWindow, Quit }`.
  Derives `Debug`, `Clone`, `PartialEq`.
- Change `build_tray_icon()` to return
  `(tray_icon::TrayIcon, std::sync::mpsc::Receiver<TrayCommand>)`.
- Store the `SyncSender<TrayCommand>` in a module-level
  `OnceLock<SyncSender<TrayCommand>>` static — `SyncSender` is both `Send`
  and `Sync`, satisfying the `static` bound without a `Mutex`.
- Extend the tray menu with:
  - **Show Antares** — ID `"show"`
  - **Hide** — ID `"hide"`
  - **Quit** — ID `"quit"` (unchanged)
- Update `handle_tray_events_system` to no longer take `EventWriter<AppExit>`
  directly; instead dispatch all three IDs over the channel:
  - `"quit"` → `tx.send(TrayCommand::Quit)` (non-blocking)
  - `"show"` → `tx.send(TrayCommand::ShowWindow)` (non-blocking)
  - `"hide"` → `tx.send(TrayCommand::HideWindow)` (non-blocking)

#### 3.2 Integrate with Bevy Window Visibility

Introduce a `TrayCommandReceiver` newtype resource:

```text
#[cfg(target_os = "macos")]
#[derive(Resource)]
pub struct TrayCommandReceiver(pub std::sync::mpsc::Receiver<TrayCommand>);
```

Update `TrayPlugin::build`:

- **Startup system**: insert `TrayCommandReceiver(rx)` as a `Resource`
  alongside inserting the `TrayIcon` as a `NonSend` resource.
- **Update system** (`drain_tray_commands`): query
  `ResMut<TrayCommandReceiver>` and
  `Query<&mut Window, With<PrimaryWindow>>`, then drain the channel:
  - `TrayCommand::ShowWindow` → set `window.visible = true` and request focus
    via `window.focused = true` (Bevy 0.17 `Window` fields).
  - `TrayCommand::HideWindow` → set `window.visible = false`.
  - `TrayCommand::Quit` → `exit.send(AppExit::Success)` via
    `EventWriter<AppExit>` for a clean Bevy shutdown. The system signature
    becomes `drain_tray_commands(res: ResMut<TrayCommandReceiver>,
window: Query<&mut Window, With<PrimaryWindow>>,
exit: EventWriter<AppExit>)`.

The `drain_tray_commands` system runs in `Update`, ordered **after**
`handle_tray_events_system` so the channel is always populated before being
drained in the same frame.

#### 3.3 Testing Requirements

Add to `src/game/systems/tray.rs` under `#[cfg(test)]`:

| Test name                                      | What it verifies                                                                                            |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `test_tray_command_show_is_distinct_from_hide` | `TrayCommand::ShowWindow != TrayCommand::HideWindow`; confirms `PartialEq` derive                           |
| `test_tray_command_channel_send_recv`          | A `TrayCommand::ShowWindow` sent over an `mpsc::sync_channel` is received via `try_recv()` without blocking |

Both tests are purely data-structure / channel tests; no Bevy `App`,
`NSApp`, or `NSStatusItem` is touched.

#### 3.4 Deliverables

- [ ] `src/game/systems/tray.rs` — `TrayCommand` enum; `OnceLock<SyncSender>`
      static; updated `build_tray_icon()` signature; Show + Hide menu items;
      updated `handle_tray_events()`; `TrayCommandReceiver` resource;
      `drain_tray_commands` system; 2 new unit tests
- [ ] `src/bin/antares.rs` — no changes required (plugin registration is
      already in place from Phase 2)
- [ ] All quality gates pass (`cargo fmt`, `cargo check`,
      `cargo clippy -D warnings`, `cargo nextest run`)

#### 3.5 Success Criteria

- Clicking **Show Antares** in the menu bar raises and focuses the game window
  when it is hidden or minimised.
- Clicking **Hide** removes the game window from view without terminating the
  process.
- The Antares icon remains in the macOS menu bar while the window is hidden.
- `cargo nextest run -p antares` reports 2 new passing tray tests.
- Non-macOS/non-Linux builds compile without warnings.

---

### Phase 4: Linux GTK System Tray

#### 4.1 Foundation Work — Linux Tray Assets

No new art is required. The authoritative 22×22 and 44×44 PNGs committed for
macOS are identical in format to what Linux desktop environments expect. Copy
them into a platform-specific directory so the `include_bytes!` paths in
`tray.rs` remain explicit and self-documenting:

```text
assets/icons/game/linux/tray_icon_1x.png   # copied from game/macos/tray_icon_1x.png
assets/icons/game/linux/tray_icon_2x.png   # copied from game/macos/tray_icon_2x.png
```

#### 4.2 System Package Requirements

`tray-icon` on Linux uses GTK and `libappindicator` (or the modern
`libayatana-appindicator`). These are OS-level packages; they cannot be
installed via Cargo. Add a note to the project `README.md` and any CI
configuration documenting the requirement:

| Distro family   | Command                                                         |
| --------------- | --------------------------------------------------------------- |
| Arch / Manjaro  | `pacman -S gtk3 xdotool libappindicator-gtk3`                   |
| Debian / Ubuntu | `sudo apt install libgtk-3-dev libxdo-dev libappindicator3-dev` |

The `libayatana-appindicator` variant (`libayatana-appindicator-gtk3` /
`libayatana-appindicator3-dev`) is an acceptable substitute on distros where
`libappindicator` is no longer packaged.

#### 4.3 Expand `Cargo.toml` Dependencies

Replace the macOS-only gate from Phase 2 with a combined gate, and add `gtk`
explicitly so `gtk::init()` and `gtk::events_pending()` are callable:

```text
[target.'cfg(any(target_os = "macos", target_os = "linux"))'.dependencies]
tray-icon = { version = "0.19", features = [] }

[target.'cfg(target_os = "linux")'.dependencies]
tray-icon = { version = "0.19", features = ["gtk"] }
gtk = "0.18"
```

The `"gtk"` feature flag on the Linux dep is **required** — without it
`tray-icon` does not compile the GTK backend. The macOS dep keeps
`features = []` because the GTK feature is Linux-only.

`gtk 0.18.2` is already present in `Cargo.lock` as a transitive dependency of
`tray-icon`; adding it directly pins the version and allows calling GTK APIs
(`gtk::init`, `gtk::main`) without `unsafe` casting.

`crossbeam-channel` is already a transitive dependency of `tray-icon` (it
backs `MenuEvent::receiver()`) and does **not** need to be added explicitly.

#### 4.4 GTK Thread Design

On Linux, `tray-icon` requires the `TrayIcon` to be created on the same thread
that runs the GTK event loop. Bevy's main thread runs the winit event loop
(Wayland or X11), which is incompatible with GTK's main loop. The solution is a
dedicated GTK thread, matching the pattern from the official `tray-icon` examples.

Introduce `pub fn spawn_gtk_tray_thread()` in `tray.rs`, compiled only on
Linux, that:

1. Spawns a named OS thread (`"gtk-tray"`) that:
   - Calls `gtk::init().expect(...)` (panics if no display server is
     available — a headless server should not run the game).
   - Calls `build_tray_icon()` to construct the `TrayIcon` on this thread
     and binds it to a local `_tray` variable (keeps it alive for the life
     of the thread).
   - Calls `gtk::main()`, which **blocks** the thread and hands control to
     GTK's own event loop. GTK internally fires the signal handlers that
     `tray-icon` registered, and those handlers push to the crossbeam channel
     backing `MenuEvent::receiver()`.

The sketch matches the official `tray-icon` example:

```text
thread::spawn(|| {
    gtk::init().unwrap();
    let _tray = build_tray_icon();
    gtk::main(); // blocks; tray-icon GTK callbacks fire here
});
```

Because `gtk::main()` blocks indefinitely, the GTK thread **cannot** also poll
`MenuEvent::receiver()` and forward to a `TrayCommand` channel. Instead,
`drain_tray_commands` on the Bevy main thread polls `MenuEvent::receiver()`
directly. This is safe because `MenuEvent::receiver()` returns a reference to
the crossbeam `Receiver` inside `tray-icon`, which is designed for cross-thread
use — GTK callbacks push to it on the GTK thread, and the Bevy system drains it
on the main thread.

The GTK thread holds `_tray` for its entire lifetime. `gtk::main()` never
returns under normal operation, so the thread (and the `TrayIcon`) live for the
duration of the process.

#### 4.5 Expand Module and Plugin Platform Gates

Change the file-level attribute in `src/game/systems/tray.rs` from:

```text
#![cfg(target_os = "macos")]
```

to:

```text
#![cfg(any(target_os = "macos", target_os = "linux"))]
```

Update `TrayPlugin::build` to branch by platform:

```text
Startup system (setup_tray_icon):
  #[cfg(target_os = "macos")]  → existing NonSend-resource path (unchanged):
                                  build_tray_icon() → NonSend TrayIcon +
                                  TrayCommandReceiver resource
  #[cfg(target_os = "linux")]  → call spawn_gtk_tray_thread() (fire-and-forget;
                                  no receiver resource needed — see below)

Update system:
  handle_tray_events_system    → compiled only on macOS; sends TrayCommand
                                  values over the OnceLock channel
  drain_tray_commands          → compiled on both platforms, but with
                                  different event sources:
    macOS: drains TrayCommandReceiver (mpsc channel populated by
           handle_tray_events_system)
    Linux: polls tray_icon::menu::MenuEvent::receiver().try_recv() directly,
           then maps menu IDs to window/exit actions inline — no intermediate
           TrayCommand channel required
```

The Linux path in `drain_tray_commands` maps `MenuEvent` IDs directly to Bevy
actions rather than going through `TrayCommand`:

```text
while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
    match event.id.as_ref() {
        "show" → window.visible = true; window.focused = true
        "hide" → window.visible = false
        "quit" → exit.send(AppExit::Success)
        _      → (ignored)
    }
}
```

Update `src/game/systems/mod.rs` gate from
`#[cfg(target_os = "macos")]` to
`#[cfg(any(target_os = "macos", target_os = "linux"))]`.

Update `src/bin/antares.rs` plugin registration gate similarly.

#### 4.6 Testing Requirements

Add to `src/game/systems/tray.rs` under `#[cfg(test)]`:

| Test name                             | What it verifies                                |
| ------------------------------------- | ----------------------------------------------- |
| `test_linux_tray_icon_1x_png_magic`   | Linux `TRAY_ICON_1X` bytes start with PNG magic |
| `test_linux_tray_icon_2x_png_magic`   | Linux `TRAY_ICON_2X` bytes start with PNG magic |
| `test_linux_tray_icon_1x_dimensions`  | Decoded width == 22, height == 22               |
| `test_linux_tray_icon_2x_dimensions`  | Decoded width == 44, height == 44               |
| `test_linux_tray_icon_1x_rgba_length` | `rgba.len() == 22 * 22 * 4`                     |

These five tests mirror the macOS Phase 2 tests but reference the Linux asset
paths. No GTK or display server is touched; they are pure PNG decode tests and
run in any environment where the crate compiles.

The Phase 3 channel tests (`test_tray_command_show_is_distinct_from_hide` and
`test_tray_command_channel_send_recv`) are already platform-agnostic and cover
Linux without modification.

#### 4.7 Deliverables

- [ ] `assets/icons/game/linux/tray_icon_1x.png` — copied from `game/macos/`
- [ ] `assets/icons/game/linux/tray_icon_2x.png` — copied from `game/macos/`
- [ ] Root `Cargo.toml` — `tray-icon` gate expanded to
      `any(target_os = "macos", target_os = "linux")`; `gtk = "0.18"` added
      under `[target.'cfg(target_os = "linux")'.dependencies]`
- [ ] `README.md` — Linux system package prerequisites documented
- [ ] `src/game/systems/tray.rs` — `spawn_gtk_tray_thread()` added; Linux
      asset embeds added; `cfg` gate broadened; `TrayPlugin` branched by
      platform; 5 new Linux PNG unit tests
- [ ] `src/game/systems/mod.rs` — `cfg` gate broadened
- [ ] `src/bin/antares.rs` — `cfg` gate broadened
- [ ] All quality gates pass on Linux (`cargo fmt`, `cargo check`,
      `cargo clippy -D warnings`, `cargo nextest run`)

#### 4.8 Success Criteria

- Antares icon appears in the Linux system tray (GNOME, KDE, XFCE) when the
  game is running with `libappindicator` or `libayatana-appindicator` installed.
- Clicking **Show Antares** raises and focuses the game window.
- Clicking **Hide** hides the game window; the tray icon remains.
- Clicking **Quit** triggers a clean Bevy shutdown via `AppExit::Success`.
- `cargo nextest run -p antares` reports 5 new passing Linux PNG tests.
- macOS build is unaffected; Windows and other non-Linux/non-macOS builds
  compile without warnings.

---

## Architecture Notes

### Bevy vs. eframe Differences

| Concern                  | Campaign Builder (eframe)                         | Game (Bevy)                                           |
| ------------------------ | ------------------------------------------------- | ----------------------------------------------------- |
| Window icon              | `ViewportBuilder::with_icon()`                    | `WinitWindows::get_window().set_window_icon()`        |
| Tray lifetime management | `let _tray` binding in `run()`                    | `NonSend` resource (macOS) / GTK thread (Linux)       |
| Event polling            | `handle_tray_events()` in `eframe::App::update()` | Bevy system (macOS) / direct `MenuEvent` poll (Linux) |
| Show/Hide window         | `ViewportCommand::Visible(bool)`                  | `Window::visible = bool` on `PrimaryWindow` entity    |
| Graceful quit            | `ViewportCommand::Close`                          | `EventWriter<AppExit>` with `AppExit::Success`        |
| Receiver storage         | Struct field on `CampaignBuilderApp`              | `TrayCommandReceiver` Bevy `Resource`                 |

### macOS vs. Linux Tray Architecture

| Concern             | macOS                                                      | Linux                                                        |
| ------------------- | ---------------------------------------------------------- | ------------------------------------------------------------ |
| Event loop          | macOS run loop; `TrayIcon` created on main thread          | GTK main loop; must run on a dedicated thread                |
| Thread model        | Main thread only; `NonSend` resource guards this           | Dedicated `"gtk-tray"` OS thread spawned at startup          |
| `TrayIcon` lifetime | Held as Bevy `NonSend` resource                            | Held as a local `_tray` binding on the GTK thread's stack    |
| Tray event polling  | `handle_tray_events_system` Bevy `Update` system           | `drain_tray_commands` polls `MenuEvent::receiver()` directly |
| Command dispatch    | `OnceLock<SyncSender>` → Bevy `drain_tray_commands` system | GTK callbacks → crossbeam channel → `drain_tray_commands`    |
| System dependencies | None (macOS frameworks linked automatically)               | `libgtk-3-dev`, `libxdo-dev`, `libappindicator3-dev`         |

### `NonSend` Resource for `TrayIcon` (macOS)

On macOS, `tray_icon::TrayIcon` is not `Send` (it holds Objective-C pointers
that must be accessed on the main thread). Bevy's `NonSend` resource system
guarantees that systems accessing it run on the main thread — the same
guarantee that `WinitWindows` relies on.

On Linux, `TrayIcon` is created on the dedicated GTK thread and held there for
the process lifetime; it never crosses thread boundaries, so the `Send` bound
is irrelevant.

### `OnceLock<SyncSender>` Pattern

Identical to the Campaign Builder Phase 3 implementation. `SyncSender<T>` is
both `Send` and `Sync`, satisfying the `static` constraint for
`OnceLock<SyncSender<TrayCommand>>`. The bounded capacity of 32 ensures that
rapid menu interactions cannot block the OS callback thread.

### System Ordering

```text
Startup:
  set_window_icon            (NonSendMut<WinitWindows>)
  setup_tray_icon:
    macOS → inserts NonSend TrayIcon + TrayCommandReceiver resource
    Linux → calls spawn_gtk_tray_thread(); inserts TrayCommandReceiver resource

Update (Phase 2, macOS only):
  handle_tray_events_system  (EventWriter<AppExit>; polls MenuEvent channel,
                               emits AppExit::Success on "quit")

Update (Phase 3+, macOS only):
  handle_tray_events_system  (no EventWriter; sends all IDs over channel)

Update (Phase 3+, both platforms):
  drain_tray_commands        (Query<&mut Window, With<PrimaryWindow>>,
                               EventWriter<AppExit>,
                               [macOS only] ResMut<TrayCommandReceiver>)
    — macOS: drains TrayCommandReceiver; runs after handle_tray_events_system
    — Linux: polls MenuEvent::receiver().try_recv() directly; runs
             independently (GTK callbacks populate it asynchronously)

GTK thread (Linux only, Phase 4):
  gtk::init()
  let _tray = build_tray_icon()   // TrayIcon held on this thread
  gtk::main()                      // blocks; GTK callbacks push to
                                   // MenuEvent::receiver() crossbeam channel
```
