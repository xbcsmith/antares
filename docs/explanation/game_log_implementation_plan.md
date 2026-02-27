# Game Log Implementation Plan

## Overview

The game currently has a `GameLog` resource in `src/game/systems/ui.rs` that
collects plain `String` messages from events, dialogue, and combat — but
nothing ever renders those messages to the player. Combat already has a
dedicated in-combat log bubble (`CombatLogState` / `CombatLogBubbleRoot` in
`src/game/systems/combat.rs`). This plan builds a **persistent, scrollable,
filterable full-game event log** that layers on top of the existing
infrastructure in four phases:

1. Upgrade the `GameLog` resource with typed, categorised entries
2. Render the log as a scrollable in-game UI panel with a toggle key
3. Wire every significant game event into the log
4. Add a per-category filter bar and `GameConfig` settings

---

## Current State Analysis

### Existing Infrastructure

| Component | Location | State |
|---|---|---|
| `GameLog` resource | [src/game/systems/ui.rs](../../src/game/systems/ui.rs) | Plain `Vec<String>`, 50-entry ring, **no UI** |
| `UiPlugin` | [src/game/systems/ui.rs](../../src/game/systems/ui.rs) | Only calls `init_resource::<GameLog>()` |
| `CombatLogState` + bubble | [src/game/systems/combat.rs](../../src/game/systems/combat.rs) | Full featured, combat-only |
| `CombatFeedbackEvent` | [src/game/systems/combat.rs](../../src/game/systems/combat.rs) | Drives combat bubble, not the general log |
| `GameConfig` | [src/sdk/game_config.rs](../../src/sdk/game_config.rs) | Graphics/audio/controls/camera — no log section |
| `SaveGame` | [src/application/save_game.rs](../../src/application/save_game.rs) | No log persistence |
| Existing call sites | events.rs, dialogue.rs, dialogue_visuals.rs | Already call `game_log.add(msg)` with plain strings |

### Identified Issues

1. `GameLog` stores raw strings — no category, no color, no timestamp; cannot
   be filtered
2. No Bevy UI panel exists to display log messages during exploration or
   dialogue modes
3. Many game events (map transitions, item pickups/drops, inn management,
   rest system) are not logged at all
4. No configurable toggle key or per-category filter
5. `MAX_LOG_ENTRIES = 50` is too low; long sessions lose entries
6. No `GameLogConfig` in `GameConfig`; no way to persist player preferences

---

## Implementation Phases

### Phase 1: Core Data Layer

Upgrade the `GameLog` resource from a plain string bag into a
category-typed entry store with filter state.

#### 1.1 Add `LogCategory` Enum

Add to [src/game/systems/ui.rs](../../src/game/systems/ui.rs):

```rust
// In src/game/systems/ui.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogCategory {
    Combat,
    Dialogue,
    Item,
    Exploration,
    System,
}
```

All five variants must be represented in the filter UI. `System` is a
catch-all for messages that do not fit the other four categories (errors,
config notices, etc.).

#### 1.2 Add `LogEntry` Struct

Replace the current `Vec<String>` with a `Vec<LogEntry>` where each entry
carries the category, display text, an ANSI-free color triple, and a
monotonic counter (game turn or frame counter) for ordering.

```rust
pub struct LogEntry {
    pub category: LogCategory,
    pub text: String,
    pub color: Color,
    pub sequence: u64,   // monotonically increasing counter
}
```

Color defaults by category:
- `Combat` → `Color::srgb(0.86, 0.45, 0.45)` (muted red)
- `Dialogue` → `Color::srgb(0.85, 0.80, 0.50)` (warm gold, matching
  existing `DIALOGUE_CHOICE_COLOR`)
- `Item` → `Color::srgb(0.40, 0.78, 0.40)` (green)
- `Exploration` → `Color::srgb(0.55, 0.75, 0.95)` (sky blue)
- `System` → `Color::srgb(0.70, 0.70, 0.70)` (grey)

#### 1.3 Upgrade `GameLog`

Replace the body of [src/game/systems/ui.rs](../../src/game/systems/ui.rs):

- Rename `add(msg: String)` → `add_entry(text: String, category: LogCategory)`
- Add `add_combat(text: String)`, `add_dialogue(text: String)`,
  `add_item(text: String)`, `add_exploration(text: String)`,
  `add_system(text: String)` convenience helpers that call `add_entry` with
  the right category
- Add `const MAX_LOG_ENTRIES: usize = 200`
- Add `filter: HashSet<LogCategory>` with all categories enabled by default
- Add `filtered_entries(&self) -> Vec<&LogEntry>` that respects `filter`
- Add `sequence_counter: u64` for monotonic ordering

#### 1.4 Update All Existing Call Sites

Replace every `game_log.add(msg)` call in the codebase with the
appropriate typed helper:

| File | Current call | Upgrade |
|---|---|---|
| [src/game/systems/events.rs](../../src/game/systems/events.rs) | `log.add(msg)` for teleport, encounter, trap, treasure, sign | `add_exploration` / `add_combat` |
| [src/game/systems/events.rs](../../src/game/systems/events.rs) | `log.add(msg)` for NPC dialogue, recruitable character | `add_dialogue` |
| [src/game/systems/dialogue.rs](../../src/game/systems/dialogue.rs) | `log.add(...)` for dialogue nodes | `add_dialogue` |
| [src/game/systems/dialogue_visuals.rs](../../src/game/systems/dialogue_visuals.rs) | `log.add(...)` | `add_dialogue` |

#### 1.5 Testing Requirements

- `test_log_entry_category_defaults` — add one entry of each category,
  assert colors match the defaults in section 1.2
- `test_log_filter_excludes_category` — disable `LogCategory::Combat`,
  add both combat and dialogue entries, assert `filtered_entries` returns
  only the dialogue entry
- `test_log_max_entries_ring` — add `MAX_LOG_ENTRIES + 10` entries, assert
  `entries.len() == MAX_LOG_ENTRIES` and the oldest entries are dropped
- `test_log_sequence_monotonic` — add three entries, assert
  `entries[2].sequence > entries[1].sequence > entries[0].sequence`

#### 1.6 Deliverables

- [ ] `LogCategory` enum with five variants in `ui.rs`
- [ ] `LogEntry` struct with category, text, color, sequence in `ui.rs`
- [ ] `GameLog` upgraded; `MAX_LOG_ENTRIES = 200`
- [ ] `add_entry` + five typed helper methods on `GameLog`
- [ ] All existing `game_log.add(msg)` call sites migrated
- [ ] Unit tests passing

#### 1.6 Success Criteria

`cargo check --all-targets --all-features` produces zero errors. All
call sites compile. Existing event handling behaviour is unchanged.

---

### Phase 2: Scrollable UI Panel

Render the game log as a Bevy UI panel visible during exploration, dialogue,
and menu modes; hidden during combat (combat has its own bubble).

#### 2.1 Add UI State Resources

Add to [src/game/systems/ui.rs](../../src/game/systems/ui.rs):

```rust
#[derive(Resource, Default)]
pub struct GameLogUiState {
    pub visible: bool,
    pub needs_scroll_to_bottom: bool,
}
```

`visible` defaults to `true` (configured in Phase 4). `needs_scroll_to_bottom`
is set to `true` whenever a new entry is appended; the scroll system clears
it after applying.

#### 2.2 Add Component Markers

```rust
#[derive(Component)] pub struct GameLogPanelRoot;
#[derive(Component)] pub struct GameLogScrollViewport;
#[derive(Component)] pub struct GameLogLineList;
#[derive(Component)] pub struct GameLogLineItem { pub index: usize }
```

Mirror the pattern used by `CombatLogBubbleRoot` / `CombatLogBubbleViewport`
/ `CombatLogLineList` in [src/game/systems/combat.rs](../../src/game/systems/combat.rs).

#### 2.3 Spawn the Panel

Add `setup_game_log_panel` system to `UiPlugin::build`. The panel spawns
as a fixed-width column anchored to the left edge of the screen, below the
HUD (above `HUD_BOTTOM_GAP`). Parameters:

- Width: `Val::Px(300.0)` — `GAME_LOG_PANEL_WIDTH`
- Height: `Val::Px(200.0)` — `GAME_LOG_PANEL_HEIGHT`
- Background: `Color::srgba(0.06, 0.09, 0.13, 0.88)`
- `overflow: Overflow::scroll_y()` on the inner viewport
- Up to `GAME_LOG_VISIBLE_LINES = 12` lines tall

Hierarchy:
```
GameLogPanelRoot (Node, BackgroundColor, GameLogPanelRoot)
  └─ header row: "Game Log" label + filter buttons placeholder
  └─ GameLogScrollViewport (Node, overflow: scroll_y, GameLogScrollViewport)
       └─ GameLogLineList (Node, flex_column, GameLogLineList)
            └─ Text rows (spawned/updated by sync system)
```

#### 2.4 Add Toggle System

Add `toggle_game_log_panel` system (runs during `Update`):

- Reads `GameConfig.game_log.toggle_key` (Phase 4 adds this; use `KeyCode::KeyL` as default)
- Flips `GameLogUiState.visible` on press
- Sets `Visibility::Hidden` / `Visibility::Visible` on `GameLogPanelRoot`

#### 2.5 Add Sync System

Add `sync_game_log_ui` system:

- Runs when `GameLog` resource `is_changed()` OR `GameLogUiState` is changed
- Only runs when mode is `Exploration` or `Dialogue` or `Menu`
- Despawns existing `GameLogLineList` children
- Spawns one `Text` entity per `filtered_entries()` result (latest N entries
  limited by `GAME_LOG_VISIBLE_LINES`)
- Colors each text node from `entry.color`
- Auto-scrolls to bottom when `needs_scroll_to_bottom == true`

#### 2.6 Hide During Combat

`sync_game_log_ui` must not run during `GameMode::Combat(_)`. The toggle
system must also be suppressed during combat to avoid confusing the panel
visibility flag.

#### 2.7 Testing Requirements

- `test_game_log_panel_spawns_on_startup` — after `App` startup with
  `UiPlugin`, assert `GameLogPanelRoot` entity exists
- `test_game_log_panel_visibility_toggle` — set `GameLogUiState.visible =
  false`, run systems, assert panel node has `Visibility::Hidden`
- `test_game_log_sync_shows_filtered_entries` — disable `Combat` filter,
  add one combat and one dialogue entry, run sync, assert only one `Text`
  child is spawned under `GameLogLineList`

#### 2.8 Deliverables

- [ ] `GameLogUiState` resource
- [ ] `GameLogPanelRoot`, `GameLogScrollViewport`, `GameLogLineList`,
  `GameLogLineItem` markers
- [ ] `setup_game_log_panel` system registered in `UiPlugin`
- [ ] `toggle_game_log_panel` system (uses `KeyCode::KeyL` default)
- [ ] `sync_game_log_ui` system with change detection
- [ ] Panel hidden during `GameMode::Combat`
- [ ] Integration tests passing

#### 2.9 Success Criteria

Pressing `L` during exploration toggles a scrollable log panel on the left
side of the screen. New messages appear at the bottom. The panel is
invisible during combat.

---

### Phase 3: Event Coverage

Wire every meaningful in-game event into the appropriate typed log category.

#### 3.1 Exploration Events (already partially covered)

Verify and complete coverage in [src/game/systems/events.rs](../../src/game/systems/events.rs):

| Event | Category | Message Format |
|---|---|---|
| `MapEvent::Teleport` | `Exploration` | `"Entering {map_name}..."` |
| `MapEvent::Sign` | `Exploration` | `"{sign_name}: {text}"` |
| `MapEvent::Trap` | `Combat` | `"Trapped! Took {damage} damage."` |
| `MapEvent::Treasure` | `Item` | `"Found treasure! {loot_count} item(s)."` |
| `MapEvent::Encounter` | `Combat` | `"Monsters! ({count} foes)"` |
| `MapEvent::NpcDialogue` | `Dialogue` | `"{npc_name} speaks."` |
| `MapEvent::RecruitableCharacter` | `Dialogue` | `"Met {name}."` |
| `MapEvent::EnterInn` | `Exploration` | `"Entering {inn_name}."` |
| `MapEvent::EnterMerchant` | `Exploration` | `"Visiting {merchant_name}."` |

#### 3.2 Map Transition Events

Add a `GameLogEvent` Bevy message type (defined in `ui.rs`) so domain-
adjacent systems can fire log entries without coupling to `GameLog` directly:

```rust
#[derive(Message)]
pub struct GameLogEvent {
    pub text: String,
    pub category: LogCategory,
}
```

In [src/game/systems/map.rs](../../src/game/systems/map.rs), handle
`MapChangeEvent` completion: after a map is loaded, write a `GameLogEvent`
with `Exploration` category: `"Entered {map_name} ({map_id})."`. Add a
`consume_game_log_events` system in `UiPlugin::build` that reads
`MessageReader<GameLogEvent>` and appends to `GameLog`.

#### 3.3 Combat Damage Events

After a combat round resolves (inside [src/game/systems/combat.rs](../../src/game/systems/combat.rs)),
listen for `CombatFeedbackEvent` and mirror the plain-text representation
of each event to `GameLog` with category `Combat`. This ensures that after
combat ends and the combat bubble is cleaned up, the player can still review
what happened via the general log.

Use the existing `format_combat_log_line` helper — call `.plain_text()` on
the result and pass it to `game_log.add_combat(...)`.

#### 3.4 Dialogue Events

In [src/game/systems/dialogue.rs](../../src/game/systems/dialogue.rs), log
NPC name + node text whenever a dialogue node advances
(`handle_start_dialogue`, `handle_advance_dialogue`). Use `add_dialogue`.
Keep messages concise: `"{speaker}: {first 80 chars of text}"`.

#### 3.5 Item Events

Item pickups, drops, and purchases are currently handled in domain
transactions (`src/domain/transactions.rs`) and Bevy inventory UI systems.
Add `GameLogEvent` writes at every call site in the Bevy layer that performs
an inventory transaction:

| Action | Category | Format |
|---|---|---|
| Pick up item | `Item` | `"Picked up {item_name}."` |
| Drop item | `Item` | `"Dropped {item_name}."` |
| Buy item | `Item` | `"Bought {item_name} for {cost} gold."` |
| Sell item | `Item` | `"Sold {item_name} for {value} gold."` |
| Equip item | `Item` | `"{character} equipped {item_name}."` |

#### 3.6 Party / InnKeeper Events

In [src/game/systems/inn_ui.rs](../../src/game/systems/inn_ui.rs) (or
wherever party recruitment/dismissal occurs), write `GameLogEvent` entries:

| Action | Category | Format |
|---|---|---|
| Recruit to party | `Dialogue` | `"{name} joins the party."` |
| Dismiss to inn | `Dialogue` | `"{name} waits at the inn."` |
| Rest | `Exploration` | `"The party rests. HP restored."` |

#### 3.7 Testing Requirements

- `test_map_change_logs_exploration_entry` — fire `MapChangeEvent`, assert
  a `GameLog` entry with `LogCategory::Exploration` is created
- `test_combat_feedback_mirrors_to_game_log` — fire a `CombatFeedbackEvent`
  for a hit, assert a `LogCategory::Combat` entry appears in `GameLog`
- `test_item_pickup_logs_item_entry` — simulate a pickup transaction, assert
  `LogCategory::Item` entry in `GameLog`

#### 3.8 Deliverables

- [ ] `GameLogEvent` Bevy message and `consume_game_log_events` system
- [ ] Map transition log entries in `map.rs`
- [ ] Combat feedback mirrored to `GameLog` in `combat.rs`
- [ ] Dialogue node advance log entries in `dialogue.rs`
- [ ] Item transaction log entries at Bevy call sites
- [ ] Inn / party management log entries
- [ ] All tests passing

#### 3.9 Success Criteria

Every significant game event (map enter, encounter, dialogue, item transaction,
rest, recruitment) produces a visible log entry. No important event is silent.

---

### Phase 4: Filter Bar and Configuration

Add a per-category filter bar to the log panel and expose log settings via
`GameConfig`.

#### 4.1 Add `GameLogConfig` to `GameConfig`

Add to [src/sdk/game_config.rs](../../src/sdk/game_config.rs):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameLogConfig {
    pub max_entries: usize,           // default: 200
    pub visible_by_default: bool,     // default: true
    pub toggle_key: String,           // default: "L"
    pub show_timestamps: bool,        // default: false
    pub panel_width_px: f32,          // default: 300.0
    pub panel_height_px: f32,         // default: 200.0
    pub panel_opacity: f32,           // default: 0.88 (0.0–1.0)
    pub default_enabled_categories: Vec<String>, // default: all five
}
```

Add `pub game_log: GameLogConfig` to `GameConfig` and update
`GameConfig::default()` and `GameConfig::validate()` accordingly. Update
the config template at [campaigns/config.template.ron](../../campaigns/config.template.ron).

#### 4.2 Wire Config into Systems

In `setup_game_log_panel` and `toggle_game_log_panel`, read the loaded
`GameConfig` (from `GlobalState` or a dedicated `Res<LoadedGameConfig>`) to
set initial visibility, panel dimensions, opacity, and which key triggers toggle.

Read `max_entries` during `GameLog` setup and store in a field so the ring
buffer uses it instead of the compile-time constant.

#### 4.3 Add Filter Bar UI

Insert a horizontal filter bar as the header row of the `GameLogPanelRoot`.
Spawn one `Button` per `LogCategory` variant:

- Labels: "CMB" / "DLG" / "ITM" / "EXP" / "SYS" (abbreviated to fit)
- Active color: category's default color with 100% opacity
- Inactive color: same color at 30% opacity
- Click toggles the corresponding `LogCategory` in `GameLog.filter`
- Triggers `GameLogUiState` change to re-run `sync_game_log_ui`

Add `handle_log_filter_buttons` system that reads `Interaction` on filter
buttons, updates `GameLog.filter`, and sets `needs_scroll_to_bottom = true`.

#### 4.4 Add Filter Button Marker Component

```rust
#[derive(Component)]
pub struct LogFilterButton {
    pub category: LogCategory,
    pub active: bool,
}
```

`handle_log_filter_buttons` queries `(&LogFilterButton, &Interaction,
&mut BackgroundColor)` to handle clicks and visual feedback.

#### 4.5 Show Entry Count

Add a small `Text` node to the top-right of the header row showing
`"N entries"` (count of `filtered_entries()`). Update it in `sync_game_log_ui`.

#### 4.6 Testing Requirements

- `test_filter_button_toggles_category` — simulate a button press on the
  `Combat` filter button, assert `LogCategory::Combat` is removed from
  `GameLog.filter`
- `test_game_log_config_validates` — construct a `GameLogConfig` with
  `max_entries = 0`, call `GameConfig::validate()`, assert it returns an
  error
- `test_game_log_config_round_trip` — serialize default `GameLogConfig` to
  RON and parse it back, assert equality
- `test_panel_opacity_from_config` — load a config with `panel_opacity: 0.5`,
  run startup, assert `GameLogPanelRoot` `BackgroundColor` alpha ≈ 0.5

#### 4.7 Deliverables

- [ ] `GameLogConfig` struct in `game_config.rs`
- [ ] `GameConfig` updated with `game_log: GameLogConfig` field
- [ ] `config.template.ron` updated with `game_log` section
- [ ] Filter bar UI row with five category buttons
- [ ] `LogFilterButton` marker component
- [ ] `handle_log_filter_buttons` system
- [ ] Entry count label in header
- [ ] Config read at panel setup and toggle key binding
- [ ] All tests passing

#### 4.8 Success Criteria

Clicking a filter button immediately hides or reveals entries of that
category. Config file controls panel visibility, size, opacity, and the
toggle key. `cargo nextest run --all-features` passes with ≥ 80% coverage
on new code.

---

## Architecture Compliance Notes

- All new `.rs` files belong in `src/game/systems/` or `src/sdk/`
- No new RON data files are created; only `config.template.ron` is updated
- `LogCategory` uses `Serialize`/`Deserialize` so it can be persisted in
  `GameLogConfig.default_enabled_categories`
- The `GameLogEvent` message follows the existing `#[derive(Message)]`
  pattern used throughout `src/game/systems/`
- Constants (`MAX_LOG_ENTRIES`, `GAME_LOG_PANEL_WIDTH`, etc.) are extracted;
  no magic numbers in implementation code
- `GameLog` filter state is a `HashSet<LogCategory>` — O(1) lookup, not a
  linear scan
- No changes to any domain-layer data structures; all new types live in the
  game or SDK layers
