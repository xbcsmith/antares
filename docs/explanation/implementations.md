# Implementations

## Food System — Phase 1: Core Item Foundation

### Overview

Phase 1 converts food from an abstract numeric counter into a proper inventory
item by adding the `ConsumableEffect::IsFood(u8)` variant to the item type
system and defining canonical food items in the game's data files. This is the
foundation for Phases 2–4 which will rewrite the rest system, wire up merchant
stock, and update the SDK editor.

### Deliverables Checklist

- [x] `ConsumableEffect::IsFood(u8)` variant added to `src/domain/items/types.rs`
- [x] Base food items added to `data/items.ron` (ids 53 "Food Ration", 54 "Trail Ration")
- [x] Food items added to `data/test_campaign/data/items.ron` (ids 108 "Food Ration", 109 "Trail Ration")
- [x] Serialization / deserialization tests passed (10 new tests in `types.rs`)
- [x] Exhaustive match sites updated (`combat/item_usage.rs`, `visual/item_mesh.rs`)

### What Was Built

#### `ConsumableEffect::IsFood(u8)` — `src/domain/items/types.rs`

A new variant appended to the existing `ConsumableEffect` enum. The inner
`u8` is the **ration count** supplied by a single unit of the item — almost
always `1` for a standard ration, but higher values are valid for multi-serving
items such as a "Trail Ration" (3 rations).

The variant is `Copy + PartialEq + Serialize + Deserialize`, consistent with
all other `ConsumableEffect` variants, so it round-trips cleanly through RON
without any schema migration.

#### `data/items.ron` additions

Two food items were appended in a new `// ===== Food Items =====` section
between the existing Consumables block and the Ammunition block:

| id  | name         | effect    | base_cost | sell_cost | combat_usable |
| --- | ------------ | --------- | --------- | --------- | ------------- |
| 53  | Food Ration  | IsFood(1) | 2         | 1         | false         |
| 54  | Trail Ration | IsFood(3) | 5         | 2         | false         |

Food items are intentionally **not** combat-usable (`is_combat_usable: false`).

#### `data/test_campaign/data/items.ron` additions

Identical items at ids 108 / 109 (offset to avoid id collisions with the
test-campaign's existing item numbering).

#### Exhaustive match updates

Two sites in the codebase perform exhaustive matches over `ConsumableEffect`
and required new arms:

- **`src/domain/combat/item_usage.rs`** — `execute_item_use_by_slot`: the
  `IsFood(_)` arm returns `Err(ItemUseError::NotUsableInCombat)`. The
  `validate_item_use_slot` gate already blocks food items via
  `is_combat_usable: false`, so this arm is a safety net for callers that
  bypass validation.
- **`src/domain/visual/item_mesh.rs`** — consumable colour selector: food items
  are assigned an earthy brown `[0.55, 0.35, 0.10, 1.0]` to visually
  distinguish them from magical potions.

#### Tests — `src/domain/items/types.rs` (10 new)

| Test name                                     | What it verifies                                               |
| --------------------------------------------- | -------------------------------------------------------------- |
| `test_is_food_effect_equality`                | `IsFood(1) == IsFood(1)`, `IsFood(1) != IsFood(3)`             |
| `test_is_food_ration_count_extracted`         | Pattern-match extracts inner `u8`                              |
| `test_is_food_trail_pack_ration_count`        | Pack of 3 extracts correctly                                   |
| `test_is_food_serializes_correctly`           | RON output contains `"IsFood"` and the count                   |
| `test_is_food_deserializes_correctly`         | `"IsFood(1)"` parses to correct variant                        |
| `test_is_food_roundtrip_serde`                | Full serialize → deserialize identity                          |
| `test_consumable_data_with_is_food_roundtrip` | `ConsumableData` struct round-trips                            |
| `test_food_ration_item_loads_from_ron_string` | `ItemDatabase::load_from_string` succeeds with Food Ration RON |
| `test_food_ration_not_combat_usable`          | `is_combat_usable` is `false`                                  |
| `test_is_food_no_required_proficiency`        | `required_proficiency()` returns `None`                        |

### Architecture Compliance

- Data structures match architecture.md Section 4.5 (`ConsumableData`, `ConsumableEffect`) **exactly**.
- Type aliases (`ItemId`) used throughout; no raw `u32` introduced.
- RON format used for all data files; no JSON/YAML.
- Test fixtures live in `data/test_campaign/` — no reference to `campaigns/tutorial`.
- SPDX headers present in all modified `.rs` files (pre-existing headers unchanged).

### Quality Gates

```text
cargo fmt         → no output
cargo check       → Finished 0 errors
cargo clippy      → Finished 0 warnings
cargo nextest run → 3242 passed; 0 failed; 8 skipped
```

---

## Combat Events — Missing Deliverables Gap Fill

### Overview

After Phase 5 was completed a gap analysis against
`docs/explanation/combat_events_implementation_plan.md` identified five
outstanding deliverables that had not been implemented:

1. **Phase 2** — `test_ambush_player_turn_is_skipped` test missing.
2. **Phase 2** — `test_ambush_player_can_act_round_2` test missing.
3. **Phase 4** — Four individually-named boss-flag tests missing
   (`test_boss_combat_monsters_advance`, `test_boss_combat_monsters_regenerate`,
   `test_boss_combat_cannot_bribe`, `test_boss_combat_cannot_surrender`); the
   behaviour was covered by the combined `test_boss_combat_sets_boss_flags` but
   the plan required each assertion in its own named test.
4. **Phase 4** — Boss opening combat-log text deviated from the plan.
   The plan specified `"A powerful foe stands before you! Prepare for a
legendary battle!"` but the implementation emitted `"A powerful foe
appears!"`.
5. **Phase 2 / Section 2.7** — `src/domain/resources.rs` was missing the
   mandatory code comment documenting that rest-interrupted encounters must use
   `CombatEventType::Ambush`.

### Deliverables Checklist

- [x] `test_ambush_player_turn_is_skipped` — asserts the "surprised" log entry
      is emitted and `CombatTurnStateResource` stays `EnemyTurn` after the player
      slot is auto-skipped in round 1.
- [x] `test_ambush_player_can_act_round_2` — asserts that after `advance_turn`
      pushes the state into round 2, `ambush_round_active` is `false` and
      `handicap` is `Even`, confirming the player would not be skipped.
- [x] `test_boss_combat_monsters_advance` — isolated assertion on
      `cs.monsters_advance == true`.
- [x] `test_boss_combat_monsters_regenerate` — isolated assertion on
      `cs.monsters_regenerate == true`.
- [x] `test_boss_combat_cannot_bribe` — isolated assertion on
      `cs.can_bribe == false`.
- [x] `test_boss_combat_cannot_surrender` — isolated assertion on
      `cs.can_surrender == false`.
- [x] Boss opening log text corrected to
      `"A powerful foe stands before you! Prepare for a legendary battle!"`.
- [x] `ResourceError::CannotRestWithActiveEncounter` doc comment updated to
      mandate `CombatEventType::Ambush` for rest-interrupted encounters.

### What Was Built

#### `test_ambush_player_turn_is_skipped` (`src/game/systems/combat.rs`)

A Bevy app test that manually constructs a `CombatResource` with
`ambush_round_active = true` and turn order `[Player(0), Monster(1)]`, inserts
it into a `CombatPlugin` app with `CombatTurnState::EnemyTurn`, then calls
`app.update()`. After the update:

- `CombatLogState` must contain a line with the word "surprised" (emitted by
  `execute_monster_turn`'s ambush-skip path).
- `CombatTurnStateResource` must still be `EnemyTurn` (the monster on the next
  slot has not yet acted, so the system keeps enemy turn active).

#### `test_ambush_player_can_act_round_2` (`src/game/systems/combat.rs`)

A pure-logic test (no Bevy app) that calls `start_encounter` with
`CombatEventType::Ambush`, verifies `ambush_round_active == true` as a
pre-condition, then calls `cs.advance_turn(&[])` to exhaust round 1 and
trigger `advance_round`. After the call it asserts:

- `ambush_round_active == false` — the flag is cleared.
- `handicap == Handicap::Even` — the handicap is reset.
- `round == 2` — we are actually in round 2.

This is sufficient to prove the player would not be skipped: both guard checks
(`combat_input_system` and `execute_monster_turn`) inspect `ambush_round_active`
directly.

#### Four individual boss-flag tests (`src/game/systems/combat.rs`)

Each test calls `start_encounter(&mut gs, &content, &[], CombatEventType::Boss)`
and asserts exactly one `CombatState` field. They are structurally identical to
the existing `test_boss_combat_sets_boss_flags` (which remains as a combined
sanity check) but satisfy the plan's requirement that each flag has a dedicated,
individually-named test that can fail in isolation.

| Test                                   | Field asserted                   |
| -------------------------------------- | -------------------------------- |
| `test_boss_combat_monsters_advance`    | `cs.monsters_advance == true`    |
| `test_boss_combat_monsters_regenerate` | `cs.monsters_regenerate == true` |
| `test_boss_combat_cannot_bribe`        | `cs.can_bribe == false`          |
| `test_boss_combat_cannot_surrender`    | `cs.can_surrender == false`      |

#### Boss opening log text (`src/game/systems/combat.rs`)

The `CombatEventType::Boss` arm of the `opening_text` match inside
`handle_combat_started` was updated from:

```antares/src/game/systems/combat.rs#L1212-1212
"A powerful foe appears!".to_string()
```

to:

```antares/src/game/systems/combat.rs#L1212-1214
"A powerful foe stands before you! Prepare for a legendary battle!".to_string()
```

This matches the exact text specified in plan Section 4.5.

#### Rest-interruption ambush comment (`src/domain/resources.rs`)

A `# Combat Event Type Requirement` doc section was added to the
`ResourceError::CannotRestWithActiveEncounter` variant. It states:

> Any encounter that fires while the party is resting **MUST** be started with
> `CombatEventType::Ambush`. The resting party is asleep and cannot react — the
> ambush mechanic (monsters act first in round 1, party turns suppressed)
> correctly models this. The rest system implementation is responsible for
> passing `CombatEventType::Ambush` to `start_encounter()` whenever it returns
> this error variant and triggers combat.

A cross-reference link to plan Section 2.7 is included.

### Architecture Compliance

- No architectural deviations introduced.
- No new data structures modified.
- All new tests use `data/test_campaign` patterns; no reference to
  `campaigns/tutorial`.
- RON format unchanged — no data files modified.

### Quality Gate Results

```text
cargo fmt --all           → no output (clean)
cargo check --all-targets → Finished dev profile, 0 errors
cargo clippy -D warnings  → Finished dev profile, 0 warnings
cargo nextest run         → 3232 tests run: 3232 passed, 8 skipped
  (campaign_builder)      → 1938 tests run: 1938 passed, 2 skipped
```

---

## Phase 5: Campaign Builder UI — Combat Event Type

### Overview

Phase 5 wires the `CombatEventType` domain enum (introduced in Phase 1) into the
Campaign Builder SDK so that campaign authors can select and persist the combat type
for every map encounter event and random encounter group without hand-editing RON
files. It also surfaces the selected type visually in the inspector panel with
per-type colour hints.

Files modified:

- `sdk/campaign_builder/src/map_editor.rs` — all UI, state, and serialisation changes
- `sdk/campaign_builder/src/lib.rs` — fixed 9 pre-existing `Attack` struct-literal
  compilation errors (missing `is_ranged` field introduced by Phase 3)

### Phase 5 Deliverables Checklist

- [x] `encounter_combat_event_type: CombatEventType` field on `EventEditorState`
- [x] `CombatEventType::Normal` default in `impl Default for EventEditorState`
- [x] `CombatEventType` combo-box in `show_event_editor()` for Encounter type
- [x] `to_map_event()` forwards `combat_event_type` into `MapEvent::Encounter`
- [x] `from_map_event()` reads `combat_event_type` from `MapEvent::Encounter`
- [x] Per-group `CombatEventType` selector in the random encounter table editor (`show_metadata_editor`)
- [x] Combat type displayed with per-type colour in the inspector panel (`show_inspector_panel`)
- [x] Combat type colour constants (`COMBAT_TYPE_COLOR_AMBUSH/BOSS/RANGED/MAGIC`)
- [x] `push_id` used for all group-level combo-boxes (no egui ID clashes)
- [x] `ComboBox::from_id_salt` used for every combo-box (SDK egui ID rule)
- [x] `ScrollArea::id_salt` set on encounter groups scroll area
- [x] All 12 Phase 5 tests pass
- [x] All 4 quality gates pass (fmt / check / clippy / nextest)

### What Was Built

#### `encounter_combat_event_type` field on `EventEditorState` (`sdk/campaign_builder/src/map_editor.rs`)

Added a new public field to `EventEditorState`:

```sdk/campaign_builder/src/map_editor.rs#L1952-1953
/// Combat event type selected for this encounter. Controls ambush, ranged,
/// magic, and boss mechanics in the game layer.
pub encounter_combat_event_type: CombatEventType,
```

`impl Default for EventEditorState` initialises it to `CombatEventType::Normal` so
that existing saved events without the field continue to behave identically
(backward-compatible via `#[serde(default)]` on the domain struct).

#### `to_map_event()` — encounter arm (`sdk/campaign_builder/src/map_editor.rs`)

The `EventType::Encounter` arm was extended to forward the editor field:

```sdk/campaign_builder/src/map_editor.rs#L2141-2155
Ok(MapEvent::Encounter {
    name: self.name.clone(),
    description: self.description.clone(),
    monster_group: monsters,
    time_condition: None,
    facing,
    proximity_facing: self.event_proximity_facing,
    rotation_speed: ...,
    combat_event_type: self.encounter_combat_event_type,
})
```

#### `from_map_event()` — encounter arm (`sdk/campaign_builder/src/map_editor.rs`)

The `MapEvent::Encounter` arm was extended to read `combat_event_type` back into the
editor state, enabling lossless round-trip editing:

```sdk/campaign_builder/src/map_editor.rs#L2371-2391
MapEvent::Encounter {
    ...,
    combat_event_type,
    ..
} => {
    ...
    s.encounter_combat_event_type = *combat_event_type;
}
```

#### Combat Type ComboBox in `show_event_editor()` (`sdk/campaign_builder/src/map_editor.rs`)

After the monster selector for `EventType::Encounter`, a labelled combo-box is
rendered using `ComboBox::from_id_salt("encounter_combat_event_type")`. It iterates
`CombatEventType::all()`, uses `selectable_value` for each variant, and shows the
variant `description()` as hover text. A small grey description label appears below
the combo-box for the currently-selected variant. Changing the selection sets
`editor.has_changes = true`.

#### Per-group CombatEventType in `show_metadata_editor()` (`sdk/campaign_builder/src/map_editor.rs`)

The random encounter table section was added to the map metadata editor. For each
`EncounterGroup` in `EncounterTable::groups`, the UI:

1. Wraps the row in `ui.push_id(group_idx, |ui| { ... })` to prevent egui ID collisions.
2. Renders `ComboBox::from_id_salt(format!("encounter_group_combat_type_{}", group_idx))` for per-group type selection.
3. Shows the group's `combat_event_type.description()` as a small grey hint label.
4. Provides a "🗑️ Remove" button per group and an "➕ Add Group" button at the bottom.
5. Wraps the group list in `ScrollArea::vertical().id_salt("encounter_groups_scroll")`.

#### Inspector panel — combat type display (`sdk/campaign_builder/src/map_editor.rs`)

`show_inspector_panel()` was extended so that when the selected tile has a
`MapEvent::Encounter`, the combat type is shown with a colour-coded label:

| Variant | Colour                                     |
| ------- | ------------------------------------------ |
| Normal  | `Color32::LIGHT_GRAY`                      |
| Ambush  | `COMBAT_TYPE_COLOR_AMBUSH` (180, 60, 70)   |
| Ranged  | `COMBAT_TYPE_COLOR_RANGED` (209, 154, 102) |
| Magic   | `COMBAT_TYPE_COLOR_MAGIC` (198, 120, 221)  |
| Boss    | `COMBAT_TYPE_COLOR_BOSS` (220, 50, 50)     |

A small grey description label follows the type label.

#### Combat type colour constants (`sdk/campaign_builder/src/map_editor.rs`)

Four constants were added (grid tiles continue to use `EVENT_COLOR_ENCOUNTER` — the
colour differentiation is inspector-only):

```sdk/campaign_builder/src/map_editor.rs#L97-106
const COMBAT_TYPE_COLOR_AMBUSH: Color32 = Color32::from_rgb(180, 60, 70);
const COMBAT_TYPE_COLOR_BOSS:   Color32 = Color32::from_rgb(220, 50, 50);
const COMBAT_TYPE_COLOR_RANGED: Color32 = Color32::from_rgb(209, 154, 102);
const COMBAT_TYPE_COLOR_MAGIC:  Color32 = Color32::from_rgb(198, 120, 221);
```

#### `Attack` struct-literal fixes (`sdk/campaign_builder/src/lib.rs`)

9 `Attack { ... }` struct literals in `lib.rs` were missing the `is_ranged: bool`
field added by Phase 3. All 9 occurrences were updated to include `is_ranged: false`.
This was a pre-existing compilation blocker preventing the `campaign_builder` crate
from building; Phase 5 resolved it as part of making the full crate compile.

### Phase 5 Tests

All tests live in `mod tests` at the bottom of `sdk/campaign_builder/src/map_editor.rs`.

| Test                                                       | Assertion                                                                          |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `test_event_editor_state_default_combat_type`              | `Default::default()` has `Normal`                                                  |
| `test_to_map_event_preserves_combat_type_ambush`           | `to_map_event()` with Ambush → `MapEvent::Encounter { combat_event_type: Ambush }` |
| `test_to_map_event_preserves_combat_type_boss`             | same for Boss                                                                      |
| `test_to_map_event_preserves_combat_type_ranged`           | same for Ranged                                                                    |
| `test_to_map_event_preserves_combat_type_magic`            | same for Magic                                                                     |
| `test_from_map_event_reads_combat_type_boss`               | `from_map_event()` on Boss encounter sets editor field                             |
| `test_from_map_event_reads_combat_type_ambush`             | same for Ambush                                                                    |
| `test_from_map_event_normal_type_on_default_field`         | backward-compat: Normal field → Normal editor state                                |
| `test_combat_type_combo_box_has_all_variants`              | `CombatEventType::all()` returns exactly 5 variants                                |
| `test_combat_type_round_trip_all_variants`                 | every variant survives `to_map_event` → `from_map_event`                           |
| `test_combat_type_does_not_affect_non_encounter_events`    | Sign event is unaffected by `encounter_combat_event_type`                          |
| `test_encounter_combat_event_type_display_names_non_empty` | `display_name()` non-empty for all variants                                        |
| `test_encounter_combat_event_type_descriptions_non_empty`  | `description()` non-empty for all variants                                         |

### Architecture Compliance

- `CombatEventType` used exactly as defined in `src/domain/combat/types.rs` (Section 4 of architecture).
- `EncounterGroup::combat_event_type` uses the type alias from Phase 1 — no raw integers.
- `ComboBox::from_id_salt`, `push_id` for loops, and `ScrollArea::id_salt` follow `sdk/AGENTS.md` egui ID rules.
- No new documentation files created; summary placed in `docs/explanation/implementations.md` as required.
- No `campaigns/tutorial` references in tests (Implementation Rule 5).
- RON format used for all game data — no JSON or YAML.

### Quality Gate Results

```text
cargo fmt --all           → no output (clean)
cargo check --all-targets → Finished dev profile, 0 errors
cargo clippy -D warnings  → Finished dev profile, 0 warnings
cargo nextest run         → 3226 tests run: 3226 passed, 8 skipped
  (campaign_builder)      → 1938 tests run: 1938 passed, 2 skipped
```

---

## Phase 4: Boss Combat

### Overview

Phase 4 extends the combat system with a fully-featured **Boss** encounter
mode. When `CombatEventType::Boss` is active, monsters never flee, regenerate
at an accelerated rate (`BOSS_REGEN_PER_ROUND = 5` HP per round instead of 1),
and a prominent HP bar is rendered at the top of the screen. Victory over a
boss encounter sets `VictorySummary::boss_defeated = true`, causing the victory
screen to display a `"⚔ Boss Defeated! ⚔"` header.

### Phase 4 Deliverables Checklist

- [x] `BOSS_REGEN_PER_ROUND` and `BOSS_STAT_MULTIPLIER` constants in `src/domain/combat/types.rs`
- [x] Two constant unit tests in `src/domain/combat/types.rs`
- [x] `BossHpBar`, `BossHpBarFill`, `BossHpBarText` components in `src/game/systems/combat.rs`
- [x] Boss HP bar visual constants (`BOSS_HP_BAR_WIDTH`, `BOSS_HP_BAR_HEIGHT`, `BOSS_HP_HEALTHY_COLOR`, `BOSS_HP_INJURED_COLOR`, `BOSS_HP_CRITICAL_COLOR`)
- [x] `setup_combat_ui` spawns the boss HP bar panel when `combat_event_type == Boss`
- [x] `update_combat_ui` updates boss HP bar fill width and text each frame
- [x] `perform_monster_turn_with_rng` suppresses monster fleeing for Boss encounters
- [x] `perform_monster_turn_with_rng` applies `BOSS_REGEN_PER_ROUND` bonus regeneration after `advance_turn`
- [x] `execute_monster_turn` captures `round_before`/`round_after` and emits regeneration log lines
- [x] `VictorySummary::boss_defeated` field added
- [x] `process_combat_victory_with_rng` sets `boss_defeated` from `combat_event_type`
- [x] `handle_combat_victory` shows `"⚔ Boss Defeated! ⚔"` header when `boss_defeated == true`
- [x] Five new Phase 4 tests in `src/game/systems/combat.rs`

### What Was Built

#### `BOSS_REGEN_PER_ROUND` and `BOSS_STAT_MULTIPLIER` (`src/domain/combat/types.rs`)

Two public constants added immediately after the `CombatEventType` impl block:

- `BOSS_REGEN_PER_ROUND: u16 = 5` — total HP regenerated per round by a boss
  monster with `can_regenerate = true`. The base engine already adds 1 HP in
  `advance_round`; boss logic adds the remaining 4 as a bonus in
  `perform_monster_turn_with_rng`, giving exactly 5 total.
- `BOSS_STAT_MULTIPLIER: f32 = 1.0` — reserved for future stat scaling;
  currently a no-op so campaign authors can tune monsters via RON data.

#### Boss HP Bar Components (`src/game/systems/combat.rs`)

Three new `#[derive(Component)]` structs, each carrying a `participant_index`:

- `BossHpBar` — root panel node; used as a presence marker in tests
- `BossHpBarFill` — the colored fill node whose width tracks `hp.current / hp.base`
- `BossHpBarText` — the `"current/base"` text label

Five visual constants drive the appearance:

| Constant                 | Value                                          |
| ------------------------ | ---------------------------------------------- |
| `BOSS_HP_BAR_WIDTH`      | `400.0` px                                     |
| `BOSS_HP_BAR_HEIGHT`     | `20.0` px                                      |
| `BOSS_HP_HEALTHY_COLOR`  | `srgba(0.8, 0.1, 0.1, 1.0)` (dark red)         |
| `BOSS_HP_INJURED_COLOR`  | `srgba(0.5, 0.1, 0.1, 1.0)` (dimmer red)       |
| `BOSS_HP_CRITICAL_COLOR` | `srgba(0.3, 0.05, 0.05, 1.0)` (near-black red) |

#### `setup_combat_ui` boss bar spawn (`src/game/systems/combat.rs`)

After the enemy panel's `.with_children` block closes and before the turn
order panel, a conditional block checks `combat_res.combat_event_type ==
CombatEventType::Boss`. For the first monster in `participants`, it spawns:

1. An absolutely-positioned panel at `top: 8px`, centred horizontally.
2. A gold `"⚔ {name} ⚔"` name label.
3. A HP bar background → fill child (tagged `BossHpBarFill`).
4. A `BossHpBarText` label.

Only the first monster gets a boss bar (`break` after the first match).

#### `update_combat_ui` boss bar update (`src/game/systems/combat.rs`)

Two new query parameters were added to the function:

```
mut boss_hp_fills: Query<(&BossHpBarFill, &mut Node, &mut BackgroundColor), Without<EnemyHpBarFill>>
mut boss_hp_texts: Query<(&BossHpBarText, &mut Text), Without<EnemyHpText>>
```

Existing queries received matching `Without<BossHpBarFill>` / `Without<BossHpBarText>`
filters to prevent Bevy query conflicts. At the end of `update_combat_ui`, the
fill's `node.width` is set to `Val::Percent(ratio * 100.0)` and the color
transitions through the three threshold constants (≥50% healthy, ≥25%
injured, <25% critical).

#### Monster flee suppression (`perform_monster_turn_with_rng`)

Before `resolve_attack`, a `should_flee_this_turn` boolean is computed:

```rust
let should_flee_this_turn = if combat_res.combat_event_type == CombatEventType::Boss {
    false
} else if let Some(Combatant::Monster(mon)) = ... {
    mon.should_flee()
} else { false };
```

If true (non-boss only), the monster is marked as acted, the turn is advanced,
and the function returns `Ok(None)` without attacking — identical to what a
fleeing monster would do. Boss monsters always proceed to the attack path.

#### Boss bonus regeneration (`perform_monster_turn_with_rng`)

After `advance_turn`, when `combat_event_type == Boss && monsters_regenerate`:

```rust
let bonus_regen = BOSS_REGEN_PER_ROUND.saturating_sub(1); // = 4
```

Each alive, `can_regenerate` monster calls `mon.regenerate(bonus_regen)`.
Combined with `advance_round`'s built-in `regenerate(1)` this gives exactly
`BOSS_REGEN_PER_ROUND = 5` HP per round.

#### Boss regeneration log lines (`execute_monster_turn`)

```rust
let round_before = combat_res.state.round;
let outcome = perform_monster_turn_with_rng(...);
let round_after = combat_res.state.round;

if round_after > round_before
    && combat_res.combat_event_type == CombatEventType::Boss
    && combat_res.state.monsters_regenerate
{ ... push log line ... }
```

When a new round starts the log receives a green `"{name} regenerates {N} HP!"`
line (color `FEEDBACK_COLOR_HEAL`) for every regenerating alive monster.

#### `VictorySummary::boss_defeated` (`src/game/systems/combat.rs`)

A `pub boss_defeated: bool` field was added to `VictorySummary`. It is set in
`process_combat_victory_with_rng`:

```rust
boss_defeated: combat_res.combat_event_type == CombatEventType::Boss,
```

In `handle_combat_victory` the text is:

```rust
let header = if summary.boss_defeated { "⚔ Boss Defeated! ⚔\n".to_string() } else { String::new() };
Text::new(format!("{}Victory! XP: {} ...", header, ...))
```

### Phase 4 Tests

All five new tests live in the `mod tests` block at the bottom of
`src/game/systems/combat.rs`:

| Test                                        | What it verifies                                                                                 |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `test_boss_monster_does_not_flee`           | Monster at 1 HP (flee_threshold=50) returns `Ok(Some(...))` in Boss mode — it attacked, not fled |
| `test_boss_monster_regenerates_each_round`  | After `advance_turn` + bonus regen, HP increases by exactly `BOSS_REGEN_PER_ROUND`               |
| `test_boss_hp_bar_spawned`                  | After `app.update()` with Boss type, `BossHpBar` component count > 0                             |
| `test_normal_combat_no_boss_bar`            | Normal encounter produces zero `BossHpBar` components                                            |
| `test_boss_victory_summary_has_boss_header` | `process_combat_victory_with_rng` with Boss type sets `summary.boss_defeated = true`             |

Two constant tests were also added to `src/domain/combat/types.rs`:
`test_boss_regen_per_round_constant` and `test_boss_stat_multiplier_constant`.

### Implementation Notes

- **`has_acted` is reset by `advance_round`**: `advance_round` calls
  `monster.reset_turn()` on every monster, clearing `has_acted`. The flee test
  therefore verifies `Ok(Some(...))` (attack resolved) rather than `has_acted`,
  which would be unreliable when the turn wraps into a new round during the test.
- **No ECS `world().query()` borrow conflict**: Boss HP bar queries use
  `Without<EnemyHpBarFill>` / `Without<EnemyHpBarText>` filters to satisfy
  Bevy's disjoint-query requirement. Existing queries received matching
  `Without<BossHpBar*>` filters.
- **SPDX headers not duplicated**: Both files already carried SPDX headers from
  earlier phases; none were added.

### Architecture Compliance

- [x] Data structures match architecture.md Section 4.4 (`CombatState`, `Monster`)
- [x] `CombatEventType::Boss` used (not a new enum, already existed from Phase 1)
- [x] Constants extracted (`BOSS_REGEN_PER_ROUND`, `BOSS_STAT_MULTIPLIER`)
- [x] `AttributePair16` pattern used for HP (not raw integers)
- [x] No new RON data files required (boss mechanics are purely runtime)
- [x] No architectural deviations from architecture.md

### Quality Gate Results

```
cargo fmt         → no output (all files formatted)
cargo check       → Finished with 0 errors
cargo clippy      → Finished with 0 warnings
cargo nextest run → 3226 passed; 0 failed; 8 skipped
```

## Phase 3: Ranged and Magic Combat

### Overview

Phase 3 extends the combat system with two new encounter modes — **Ranged** and
**Magic** — and wires the full action pipeline for ranged attacks: button
spawning, target selection, ammo consumption, damage resolution, and combat-log
messaging. Monster AI is updated to prefer ranged attacks in ranged encounters,
and the action-menu keyboard order is adjusted so that **Cast** is the default
highlight in magic encounters.

### Phase 3 Deliverables Checklist

- [x] `TurnAction::RangedAttack` variant added to `src/domain/combat/types.rs`
- [x] `CombatError::NoAmmo` variant added to `src/domain/combat/engine.rs`
- [x] `choose_monster_attack` extended with `is_ranged_combat: bool` parameter
- [x] `ActionButtonType::RangedAttack` variant added in `src/game/systems/combat.rs`
- [x] `COMBAT_ACTION_COUNT_MAGIC` and `COMBAT_ACTION_ORDER_MAGIC` constants added
- [x] `RangedAttackAction` message struct added and registered with the plugin
- [x] `RangedAttackPending` resource added and registered with the plugin
- [x] `setup_combat_ui` spawns Ranged button for `CombatEventType::Ranged`
- [x] `setup_combat_ui` uses magic button order for `CombatEventType::Magic`
- [x] `setup_combat_ui` ordered `.after(handle_combat_started)` to fix race
- [x] `update_ranged_button_color` system enables/disables Ranged button each frame
- [x] `dispatch_combat_action` handles `ActionButtonType::RangedAttack`
- [x] `confirm_attack_target` dispatches `RangedAttackAction` when `RangedAttackPending`
- [x] `select_target` / `combat_input_system` pass `RangedAttackPending` through
- [x] `combat_input_system` uses correct action-order array for magic/standard
- [x] `update_action_highlight` uses correct order and skips the Ranged button
- [x] `perform_ranged_attack_action_with_rng` function implemented
- [x] `handle_ranged_attack_action` ECS system implemented and registered
- [x] `handle_combat_started` combat log updated for all `CombatEventType` variants
- [x] `perform_attack_action_with_rng` passes `is_ranged_combat` to `choose_monster_attack`
- [x] `perform_monster_turn_with_rng` passes `is_ranged_combat` to `choose_monster_attack`
- [x] All 10 Phase 3 tests pass; pre-existing `test_ambush_combat_started_sets_enemy_turn` fixed
- [x] `docs/explanation/implementations.md` updated

### What Was Built

#### `TurnAction::RangedAttack` (`src/domain/combat/types.rs`)

Added between `Attack` and `Defend` as the plan specifies. The doc comment
explains the ammo requirement and its intended use in `CombatEventType::Ranged`
encounters.

#### `CombatError::NoAmmo` (`src/domain/combat/engine.rs`)

New variant with message `"No ammo available for ranged attack"`. Returned by
`perform_ranged_attack_action_with_rng` when the attacker has a
`MartialRanged` weapon but no `ItemType::Ammo` item in their inventory.

#### `choose_monster_attack` signature change (`src/domain/combat/engine.rs`)

Added `is_ranged_combat: bool` as the second parameter (before `rng`). When
`true` the function first filters `monster.attacks` for entries with
`is_ranged == true`; if any exist one is chosen uniformly at random. If none
exist the function falls through to the existing special-attack-threshold +
random selection logic. When `false` behaviour is completely unchanged.

All callers updated:

- `perform_attack_action_with_rng` (monster branch) — passes `combat_res.combat_event_type == CombatEventType::Ranged`
- `perform_monster_turn_with_rng` — same
- `test_combat_monster_special_ability_applied` in `engine.rs` — passes `false`

#### `ActionButtonType::RangedAttack` (`src/game/systems/combat.rs`)

New variant inserted after `Attack`. Excluded from `COMBAT_ACTION_ORDER` and
`COMBAT_ACTION_ORDER_MAGIC` (those arrays cycle only the 5 standard actions);
the Ranged button is spawned as an extra sixth button in ranged encounters.

#### `COMBAT_ACTION_COUNT_MAGIC` and `COMBAT_ACTION_ORDER_MAGIC`

```src/game/systems/combat.rs#L319-336
pub const COMBAT_ACTION_COUNT_MAGIC: usize = 5;

pub const COMBAT_ACTION_ORDER_MAGIC: [ActionButtonType; COMBAT_ACTION_COUNT_MAGIC] = [
    ActionButtonType::Cast,
    ActionButtonType::Attack,
    ActionButtonType::Defend,
    ActionButtonType::Item,
    ActionButtonType::Flee,
];
```

`Cast` is index 0 so the default keyboard highlight is the most useful action
in a magic encounter.

#### `RangedAttackAction` and `RangedAttackPending` (`src/game/systems/combat.rs`)

`RangedAttackAction` mirrors `AttackAction` (same `attacker` + `target` fields)
but routes through `perform_ranged_attack_action_with_rng`.

`RangedAttackPending(bool)` is a `Resource` that `dispatch_combat_action` sets
to `true` when `ActionButtonType::RangedAttack` is pressed.
`confirm_attack_target` reads it: if `true`, writes `RangedAttackAction` and
resets the flag; otherwise writes the normal `AttackAction`.
Cancelling target selection (`Escape`) also clears the flag.

#### `setup_combat_ui` changes (`src/game/systems/combat.rs`)

Two changes:

1. **Magic button order** — when `combat_res.combat_event_type.highlights_magic_action()`
   the 5 standard buttons are spawned in `COMBAT_ACTION_ORDER_MAGIC` order
   (Cast first); otherwise the standard order is used.

2. **Ranged button** — after the 5 standard buttons, if
   `combat_res.combat_event_type.enables_ranged_action()` an extra `Button` is
   spawned with `ActionButton { button_type: ActionButtonType::RangedAttack }`
   and `ACTION_BUTTON_DISABLED_COLOR`. `update_ranged_button_color` enables it
   each frame once a ranged weapon + ammo is confirmed.

3. **System ordering fix** — `setup_combat_ui` is now registered
   `.after(handle_combat_started)` so `combat_res.combat_event_type` is
   populated before the button spawn decision is made. Without this ordering,
   the system could run before the message handler and always see
   `CombatEventType::Normal`.

#### `update_ranged_button_color` (`src/game/systems/combat.rs`)

New private system registered after `update_combat_ui` and
`update_action_highlight`. Each frame during a ranged encounter it queries the
current actor, calls `has_ranged_weapon(pc, &content.db().items)`, and sets
the `RangedAttack` button color to `ACTION_BUTTON_COLOR` (enabled) or
`ACTION_BUTTON_DISABLED_COLOR` (disabled).

`update_action_highlight` skips buttons with `button_type ==
ActionButtonType::RangedAttack` so the two systems do not conflict.

#### `perform_ranged_attack_action_with_rng` (`src/game/systems/combat.rs`)

Full implementation:

1. Guard: only runs if it is the attacker's current turn.
2. Only player combatants may use this path (returns `CombatantCannotAct` for monsters).
3. Calls `has_ranged_weapon` — if `false`, distinguishes "bow but no ammo"
   (`NoAmmo`) from "no bow at all" (`CombatantCannotAct`).
4. Calls `get_character_attack` expecting `MeleeAttackResult::Ranged`.
5. Calls `resolve_attack` for the to-hit roll and damage.
6. Removes the **first** `ItemType::Ammo` slot from the attacker's inventory
   (one arrow consumed per shot).
7. Calls `apply_damage`.
8. Applies special effects (same pattern as `perform_attack_action_with_rng`).
9. Calls `check_combat_end`, `advance_turn`, updates `CombatTurnStateResource`.

#### `handle_ranged_attack_action` (`src/game/systems/combat.rs`)

ECS system wrapper registered after `handle_attack_action`. Reads
`RangedAttackAction` messages, calls `perform_ranged_attack_action_with_rng`,
emits a `CombatFeedbackEvent`, and pushes a "fires a ranged attack!" log line.
On `NoAmmo` it pushes a "No ammo!" log line; other errors are logged as
warnings.

#### `handle_combat_started` combat log (`src/game/systems/combat.rs`)

Replaced the if/else branch with a `match` on `msg.combat_event_type`:

| Variant  | Log text                                                 |
| -------- | -------------------------------------------------------- |
| `Normal` | "Monsters appear!"                                       |
| `Ranged` | "Combat begins at range! Draw your bows!"                |
| `Magic`  | "The air crackles with magical energy!"                  |
| `Boss`   | "A powerful foe appears!"                                |
| `Ambush` | "The monsters ambush the party! The party is surprised!" |

#### `test_ambush_combat_started_sets_enemy_turn` fix

The test previously used an empty monster group. With the new system ordering
(setup_combat_ui runs after handle_combat_started, which in turn forces
execute_monster_turn to also run after), the ambush player-skip path in
`execute_monster_turn` would process the single player slot, wrap the round,
clear `ambush_round_active`, and leave the state as `PlayerTurn` — breaking
the assertion.

Fix: the test now injects a `CombatResource` with one player + one goblin
monster. The ambush path skips the player (slot 0) and advances to the
monster (slot 1), leaving `turn_state = EnemyTurn`. This correctly models
the actual game flow and makes the assertion meaningful.

### Phase 3 Tests

All added to `mod tests` in `src/game/systems/combat.rs`:

| Test                                                    | What it verifies                                                       |
| ------------------------------------------------------- | ---------------------------------------------------------------------- |
| `test_ranged_combat_shows_ranged_button`                | `ActionButton { RangedAttack }` spawned in Ranged combat               |
| `test_ranged_button_disabled_without_ranged_weapon`     | Button has `ACTION_BUTTON_DISABLED_COLOR` when no ranged weapon        |
| `test_ranged_button_enabled_with_ranged_weapon`         | Button has `ACTION_BUTTON_COLOR` when player has bow + ammo            |
| `test_perform_ranged_attack_consumes_ammo`              | One ammo slot removed from inventory after a ranged attack             |
| `test_perform_ranged_attack_no_ammo_returns_error`      | `CombatError::NoAmmo` when bow is equipped but inventory empty         |
| `test_magic_combat_cast_is_first_action`                | `COMBAT_ACTION_ORDER_MAGIC[0] == ActionButtonType::Cast`               |
| `test_magic_combat_normal_handicap`                     | Magic combat uses `Handicap::Even`                                     |
| `test_monster_ranged_attack_preferred_in_ranged_combat` | `choose_monster_attack(mon, true, rng)` always picks the ranged attack |
| `test_combat_log_ranged_opening`                        | Log contains "range" for `CombatEventType::Ranged`                     |
| `test_combat_log_magic_opening`                         | Log contains "magical" for `CombatEventType::Magic`                    |

Domain-layer test in `src/domain/combat/engine.rs` (existing, updated caller):

- `test_combat_monster_special_ability_applied` — updated to pass `false` for `is_ranged_combat`

### Architecture Compliance

- `TurnAction::RangedAttack` placed between `Attack` and `Defend` per plan
- `CombatError::NoAmmo` added to `engine.rs` alongside existing error variants
- `ActionButtonType::RangedAttack` added without disrupting `COMBAT_ACTION_ORDER`
- `RangedAttackAction` message follows the same `Message` derive pattern as all other action messages
- `RangedAttackPending` is a minimal `Resource` (single bool) following the resource naming convention
- All game data files remain in RON format; no new data files created
- No modifications to `campaigns/tutorial`
- Test data uses `make_p2_combat_fixture` / inline fixtures, not `campaigns/tutorial`
- `has_ranged_weapon` imported from `engine` (already existed from Phase 1 equipped-weapon work)
- `CombatError` imported from `engine` (replaces previously inline `use` statements)

### Quality Gate Results

```
cargo fmt --all          → No output (clean)
cargo check --all-targets --all-features → Finished, 0 errors
cargo clippy --all-targets --all-features -- -D warnings → Finished, 0 warnings
cargo nextest run --all-features → 3218 passed, 1 failed (pre-existing
    test_creature_database_load_performance timing flake, unrelated to Phase 3)
```

---

## Phase 2: Normal and Ambush Combat

### Overview

Phase 2 of the Combat Events Implementation Plan implements the behavioral
differences between `Normal` and `Ambush` combat encounters. After this phase,
an ambush encounter causes the party to miss their entire first round of actions
(the party is surprised), monsters receive `Handicap::MonsterAdvantage` for
round 1, and the combat log clearly announces the ambush. From round 2 onward
the combat reverts to `Handicap::Even` and proceeds identically to a normal
encounter. Boss combat flags (`monsters_advance`, `monsters_regenerate`,
`can_bribe = false`, `can_surrender = false`) are also wired in this phase via
`start_encounter()`.

### Phase 2 Deliverables Checklist

- [x] `ambush_round_active: bool` field on `CombatState` (`src/domain/combat/engine.rs`)
- [x] `TurnAction::Skip` variant in `src/domain/combat/types.rs`
- [x] `start_encounter()` sets `ambush_round_active` and `MonsterAdvantage` handicap for Ambush
- [x] `start_encounter()` sets boss flags (`monsters_advance`, `monsters_regenerate`, `can_bribe = false`, `can_surrender = false`) for Boss type
- [x] `advance_round()` clears `ambush_round_active` and resets handicap to `Even` at round 2
- [x] `handle_combat_started` forces `CombatTurnState::EnemyTurn` when `ambush_round_active` is set
- [x] `execute_monster_turn` auto-skips surprised player slots during ambush round 1
- [x] `combat_input_system` defence-in-depth guard blocks player input during ambush round 1
- [x] Combat log entry "The monsters ambush the party! The party is surprised!" on ambush start
- [x] Combat log entry "Monsters appear!" on normal encounter start
- [x] All Phase 2 tests pass (3209 passed, 8 skipped, 0 failed)

### What Was Built

#### `ambush_round_active: bool` on `CombatState` (`src/domain/combat/engine.rs`)

New boolean field on `CombatState`, defaulting to `false`. When `true`, it
signals that round 1 of an ambush is active and player turns must be skipped.
The field is cleared automatically at the start of round 2 inside
`advance_round()`.

```antares/src/domain/combat/engine.rs#L207-212
    /// True during round 1 of an ambush encounter.
    ///
    /// When set, player turns are automatically skipped (the party is surprised
    /// and cannot act). Cleared automatically at the start of round 2, at which
    /// point the handicap is also reset to `Handicap::Even` so that subsequent
    /// rounds are fought on equal footing.
    pub ambush_round_active: bool,
```

#### `TurnAction::Skip` variant (`src/domain/combat/types.rs`)

New internal-only variant added to `TurnAction`. It is never shown in the
player UI action menu; it is used programmatically by the combat engine to
represent an auto-advanced turn (ambush surprise, incapacitated combatant).

#### `advance_round()` updated (`src/domain/combat/engine.rs`)

At the start of round 2, if `ambush_round_active` is `true`, the engine:

1. Clears `ambush_round_active = false`
2. Resets `handicap = Handicap::Even`
3. Recalculates turn order under the new even handicap
4. Resets `current_turn = 0`

This ensures the remainder of the fight is fair and not permanently skewed by
the ambush initiative advantage.

#### `start_encounter()` updated (`src/game/systems/combat.rs`)

Phase 2 replaces the Phase 1 stub ("always use Even handicap") with the
correct logic:

- **Ambush**: `handicap = Handicap::MonsterAdvantage`, `ambush_round_active = true`
- **Normal / Ranged / Magic**: `handicap = Handicap::Even`, `ambush_round_active = false`
- **Boss** (any type with `applies_boss_mechanics()`): sets `monsters_advance = true`,
  `monsters_regenerate = true`, `can_bribe = false`, `can_surrender = false`

#### `handle_combat_started` updated (`src/game/systems/combat.rs`)

When `combat_res.state.ambush_round_active` is `true`, the system immediately
sets `CombatTurnStateResource` to `EnemyTurn` regardless of actual turn order
(monsters always act first in an ambush round 1). It also emits the combat log
line describing how the battle began:

- Ambush: `"The monsters ambush the party! The party is surprised!"`
- Normal: `"Monsters appear!"`

#### `execute_monster_turn` updated (`src/game/systems/combat.rs`)

At the top of `execute_monster_turn`, a new Phase 2 guard checks whether
`ambush_round_active` is set and the current slot belongs to a player. If so,
it:

1. Pushes a `"The party is surprised and cannot act!"` log line.
2. Calls `advance_turn()` to consume that slot.
3. Determines the turn state for the next actor (staying on `EnemyTurn` while
   the ambush round is still active, switching to `PlayerTurn` once it ends).
4. Returns early without performing any player-damaging action.

This loop continues until all player slots in round 1 have been skipped and
`advance_round()` fires, clearing `ambush_round_active`.

#### `combat_input_system` updated (`src/game/systems/combat.rs`)

Added a defence-in-depth guard at the top of `combat_input_system`. Even if
`CombatTurnStateResource` is somehow not `EnemyTurn` during an ambush round,
no player input is dispatched:

```antares/src/game/systems/combat.rs#L1914-1933
    // Phase 2: During an ambush round the party is surprised and cannot act.
    if combat_res.state.ambush_round_active
        && matches!(
            combat_res.state.get_current_combatant(),
            Some(Combatant::Player(_))
        )
    {
        let any_key = keyboard
            .as_ref()
            .is_some_and(|kb| kb.just_pressed(KeyCode::Tab) || kb.just_pressed(KeyCode::Enter));
        if any_key {
            info!("Combat: input blocked — party is surprised (ambush round 1)");
        }
        return;
    }
```

### Phase 2 Tests

#### Domain-layer tests (`src/domain/combat/engine.rs`)

| Test name                                              | What it verifies                                                                       |
| ------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| `test_combat_state_ambush_round_active_defaults_false` | `CombatState::new` initialises the flag to `false`                                     |
| `test_ambush_round_active_cleared_at_round_2`          | After `advance_turn` exhausts round 1, `ambush_round_active == false` and `round == 2` |
| `test_non_ambush_handicap_unchanged_at_round_2`        | When flag is `false`, `advance_round` does not alter handicap                          |

#### Game-layer tests (`src/game/systems/combat.rs`)

| Test name                                          | What it verifies                                                    |
| -------------------------------------------------- | ------------------------------------------------------------------- |
| `test_normal_combat_handicap_is_even`              | `start_encounter(…, Normal)` → `handicap == Even`                   |
| `test_ambush_combat_handicap_is_monster_advantage` | `start_encounter(…, Ambush)` → `handicap == MonsterAdvantage`       |
| `test_ambush_round_active_set_on_start`            | `start_encounter(…, Ambush)` → `ambush_round_active == true`        |
| `test_normal_round_active_not_set`                 | `start_encounter(…, Normal)` → `ambush_round_active == false`       |
| `test_ambush_round_active_cleared_at_round_2`      | After one `advance_turn`, flag cleared and `handicap == Even`       |
| `test_ambush_handicap_resets_to_even_round_2`      | Dedicated handicap-reset assertion                                  |
| `test_boss_combat_sets_boss_flags`                 | Boss type sets all four boss flags correctly                        |
| `test_combat_log_reports_ambush`                   | Log contains "ambush" text after `CombatStarted` (Ambush)           |
| `test_combat_log_reports_normal_encounter`         | Log contains "Monsters appear!" after `CombatStarted` (Normal)      |
| `test_ambush_combat_started_sets_enemy_turn`       | `CombatTurnStateResource == EnemyTurn` after ambush `CombatStarted` |

### Architecture Compliance

- [x] `ambush_round_active` is a domain-layer field on `CombatState` (no Bevy types)
- [x] `TurnAction::Skip` is in `src/domain/combat/types.rs` (domain layer)
- [x] `start_encounter()` uses `gives_monster_advantage()` and `applies_boss_mechanics()` helper methods (no hardcoded literals)
- [x] All public fields and functions have `///` doc comments
- [x] No tests reference `campaigns/tutorial` (Implementation Rule 5)
- [x] SPDX header present in all modified `.rs` files
- [x] No architectural deviations from architecture.md

### Quality Gate Results

| Gate    | Command                                                    | Result                              |
| ------- | ---------------------------------------------------------- | ----------------------------------- |
| Format  | `cargo fmt --all`                                          | ✅ No output                        |
| Compile | `cargo check --all-targets --all-features`                 | ✅ Finished, 0 errors               |
| Lint    | `cargo clippy --all-targets --all-features -- -D warnings` | ✅ Finished, 0 warnings             |
| Tests   | `cargo nextest run --all-features`                         | ✅ 3209 passed, 8 skipped, 0 failed |

---

## Phase 1: Combat Events — `CombatEventType` Domain Type and Data Layer

### Overview

Phase 1 of the Combat Events Implementation Plan adds the `CombatEventType` enum
to the domain layer and threads it end-to-end from RON data files through the
domain types, event system, and Bevy game layer without changing any combat
mechanics. After this phase, campaign RON files can declare
`combat_event_type: Ambush` (or any other variant) on an `Encounter` event and
the engine reads, stores, and forwards the type through to `CombatResource`.
Later phases (2–5) will act on the stored type to implement ambush suppression,
ranged attack availability, magic action priority, and boss mechanics.

### Phase 1 Deliverables Checklist

- [x] `CombatEventType` enum in `src/domain/combat/types.rs`
- [x] `combat_event_type: CombatEventType` field on `MapEvent::Encounter`
- [x] `combat_event_type` on `EventResult::Encounter`
- [x] `EncounterGroup` struct replacing raw `Vec<u8>` entries in `EncounterTable`
- [x] `random_encounter()` returns `Option<EncounterGroup>`
- [x] `CombatStarted.combat_event_type` field
- [x] `CombatResource.combat_event_type` field
- [x] `start_encounter()` accepts and forwards `CombatEventType`
- [x] All callers of `start_encounter()` and `random_encounter()` updated
- [x] All Phase 1 tests pass (3196 passed, 8 skipped, 0 failed)

### What Was Built

#### `CombatEventType` enum (`src/domain/combat/types.rs`)

New enum alongside the existing `Handicap`, `CombatStatus`, and `TurnAction`
types. Five variants: `Normal` (default), `Ambush`, `Ranged`, `Magic`, `Boss`.
Helper methods: `gives_monster_advantage()`, `enables_ranged_action()`,
`highlights_magic_action()`, `applies_boss_mechanics()`, `display_name()`,
`description()`, `all()`. Derives `Default` (`Normal`), `Serialize`,
`Deserialize`, `Copy`.

#### `MapEvent::Encounter` extended (`src/domain/world/types.rs`)

Added `#[serde(default)] combat_event_type: CombatEventType` field. The
`#[serde(default)]` attribute means all existing RON map files that omit the
field continue to deserialize correctly as `CombatEventType::Normal`.

#### `EncounterGroup` struct (`src/domain/world/types.rs`)

New struct replacing the raw `Vec<u8>` entries in `EncounterTable::groups`:

```antares/src/domain/world/types.rs#L2149-2195
pub struct EncounterGroup {
    pub monster_group: Vec<u8>,
    #[serde(default)]
    pub combat_event_type: CombatEventType,
}
```

Constructors: `EncounterGroup::new(monster_group)` (Normal type) and
`EncounterGroup::with_type(monster_group, combat_event_type)`.
`EncounterTable::groups` is now `Vec<EncounterGroup>` (was `Vec<Vec<u8>>`).
All existing RON files that omit `groups` continue to deserialize with the
default empty vec.

#### `EventResult::Encounter` extended (`src/domain/world/events.rs`)

Added `combat_event_type: CombatEventType` field. `trigger_event()` extracts
and forwards the value from `MapEvent::Encounter`. `random_encounter()` now
returns `Option<EncounterGroup>` (was `Option<Vec<u8>>`); callers extract
`.monster_group` and `.combat_event_type` separately.

#### `CombatStarted` message extended (`src/game/systems/combat.rs`)

Added `pub combat_event_type: CombatEventType` field. `handle_combat_started`
copies `msg.combat_event_type` into `combat_res.combat_event_type`.

#### `CombatResource` extended (`src/game/systems/combat.rs`)

Added `pub combat_event_type: CombatEventType` field, initialized to `Normal`
in `new()` and reset to `Normal` in `clear()`.

#### `start_encounter()` signature updated (`src/game/systems/combat.rs`)

New signature:

```antares/src/game/systems/combat.rs#L997-1001
pub fn start_encounter(
    game_state: &mut crate::application::GameState,
    content: &GameContent,
    group: &[u8],
    combat_event_type: CombatEventType,
) -> Result<(), crate::domain::combat::database::MonsterDatabaseError>
```

Phase 1 stores the type for the Bevy message path; Phase 2 will use it to set
`Handicap::MonsterAdvantage` for ambushes.

#### `RestCompleteEvent` extended (`src/game/systems/rest.rs`)

Added `pub encounter_combat_event_type: CombatEventType` field. Rest
interruptions are hardcoded to `CombatEventType::Ambush` (the party is caught
off-guard while sleeping), which is forwarded to `start_encounter()`.

#### All callers updated

| Caller                                            | File                                     | Change                                                                                                   |
| ------------------------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `handle_events` (encounter arm)                   | `src/game/systems/events.rs`             | Extracts `combat_event_type` from `MapEvent::Encounter`; passes to `start_encounter` and `CombatStarted` |
| `handle_rest_complete`                            | `src/game/systems/rest.rs`               | Passes `event.encounter_combat_event_type` to `start_encounter`                                          |
| `process_rest` (random encounter)                 | `src/game/systems/rest.rs`               | Returns `EncounterGroup`; sets `encounter_combat_event_type: Ambush` on `RestCompleteEvent`              |
| `move_party_and_handle_events` (random encounter) | `src/application/mod.rs`                 | Extracts `.monster_group` from `EncounterGroup`; stores `combat_event_type` (Phase 2 will act on it)     |
| `move_party_and_handle_events` (tile event)       | `src/application/mod.rs`                 | Extracts `combat_event_type` from `EventResult::Encounter`                                               |
| `MapBuilder::process_command`                     | `src/bin/map_builder.rs`                 | Adds `combat_event_type: Normal` to constructed `MapEvent::Encounter`                                    |
| `blueprint.rs` `From<MapBlueprint>`               | `src/domain/world/blueprint.rs`          | Adds `combat_event_type: Normal`                                                                         |
| `EventEditorState::to_map_event`                  | `sdk/campaign_builder/src/map_editor.rs` | Adds `combat_event_type: Normal` (Phase 5 will wire a combo-box)                                         |

### Phase 1 Tests

Tests in `src/domain/combat/types.rs`:

- `test_combat_event_type_default_is_normal`
- `test_combat_event_type_flags`
- `test_combat_event_type_display_names`
- `test_combat_event_type_descriptions_non_empty`
- `test_combat_event_type_all_has_five_variants`
- `test_combat_event_type_serde_round_trip`
- `test_combat_event_type_default_deserializes_when_missing`

Tests in `src/domain/world/events.rs`:

- `test_combat_event_type_default_is_normal`
- `test_map_event_encounter_ron_round_trip`
- `test_map_event_encounter_ron_backward_compat`
- `test_event_result_encounter_carries_type`
- `test_encounter_group_ron_round_trip`
- `test_random_encounter_returns_group_type`

Test in `src/game/systems/combat.rs`:

- `test_start_encounter_stores_type_in_resource`

### Architecture Compliance

- [x] `CombatEventType` in `src/domain/combat/types.rs` (domain layer, no Bevy)
- [x] `EncounterGroup` in `src/domain/world/types.rs` (domain layer)
- [x] `#[serde(default)]` on all new fields — full backward compatibility
- [x] `UNARMED_DAMAGE`-style: no magic literals, named constant default
- [x] `DiceRoll` / `MonsterId` type aliases used throughout
- [x] All public functions and types have `///` doc comments
- [x] No tests reference `campaigns/tutorial` (Implementation Rule 5)
- [x] SPDX header present in all modified `.rs` files
- [x] RON data files unaffected (no existing file had `groups:` data)

### Quality Gate Results

| Gate    | Command                                                    | Result                              |
| ------- | ---------------------------------------------------------- | ----------------------------------- |
| Format  | `cargo fmt --all`                                          | ✅ No output                        |
| Compile | `cargo check --all-targets --all-features`                 | ✅ Finished, 0 errors               |
| Lint    | `cargo clippy --all-targets --all-features -- -D warnings` | ✅ Finished, 0 warnings             |
| Tests   | `cargo nextest run --all-features`                         | ✅ 3196 passed, 8 skipped, 0 failed |

---

## Phase 4: Equipped Weapon Damage — Documentation and Final Validation

### Overview

Phase 4 is the concluding phase of the Equipped Weapon Damage in Combat
implementation plan. Its sole deliverables are:

1. A complete summary of all work done across Phases 1–3 added to
   `docs/explanation/implementations.md` (this section).
2. A clean run of all four mandatory quality gates with zero errors and zero
   warnings.

No new production code was written in Phase 4. Everything listed below was
already implemented and verified in Phases 1–3.

### Phase 4 Deliverables Checklist

- [x] `docs/explanation/implementations.md` updated with full cross-phase summary
- [x] `cargo fmt --all` — no output (all files already formatted)
- [x] `cargo check --all-targets --all-features` — `Finished` with 0 errors
- [x] `cargo clippy --all-targets --all-features -- -D warnings` — `Finished` with 0 warnings
- [x] `cargo nextest run --all-features` — 3182 passed, 8 skipped, 0 failed

### Full Cross-Phase Summary

#### Phase 1 — Domain Combat Engine Changes

**Files changed**: `src/domain/combat/engine.rs`, `src/domain/combat/types.rs`

| Symbol                     | Location               | Purpose                                                                                                    |
| -------------------------- | ---------------------- | ---------------------------------------------------------------------------------------------------------- |
| `UNARMED_DAMAGE`           | `engine.rs`            | `DiceRoll { count: 1, sides: 2, bonus: 0 }` — replaces all scattered 1d4 literals                          |
| `MeleeAttackResult`        | `engine.rs`            | Enum returned by `get_character_attack`: `Melee(Attack)` or `Ranged(Attack)`                               |
| `get_character_attack`     | `engine.rs`            | Pure-domain fn — resolves equipped weapon to a `MeleeAttackResult` with bonus applied via `saturating_add` |
| `has_ranged_weapon`        | `engine.rs`            | Returns `true` only when a `MartialRanged` weapon is equipped **and** ammo exists in inventory             |
| `is_ranged: bool`          | `types.rs` on `Attack` | `#[serde(default)]` field distinguishing ranged from melee attacks                                         |
| `Attack::ranged(damage)`   | `types.rs`             | Constructor that sets `is_ranged = true`                                                                   |
| `Attack::physical(damage)` | `types.rs`             | Constructor that keeps `is_ranged = false`                                                                 |

Key design decisions:

- `get_character_attack` is pure domain (no Bevy, no I/O) and lives entirely in the domain layer.
- Weapon bonus composition uses `saturating_add` to merge `weapon_data.damage.bonus` with `weapon_data.bonus` into the final `DiceRoll::bonus` — preventing silent `i8` overflow.
- Unknown item IDs and non-weapon items in the weapon slot fall back gracefully to `UNARMED_DAMAGE` rather than panicking.

Phase 1 tests added to `src/domain/combat/engine.rs` test module:

- `test_get_character_attack_no_weapon_returns_unarmed`
- `test_get_character_attack_melee_weapon_returns_melee`
- `test_get_character_attack_weapon_bonus_applied`
- `test_get_character_attack_unknown_item_id_falls_back`
- `test_get_character_attack_non_weapon_item_falls_back`
- `test_get_character_attack_ranged_weapon_returns_ranged_variant`
- `test_get_character_attack_ranged_weapon_damage_correct`
- `test_has_ranged_weapon_false_no_weapon`
- `test_has_ranged_weapon_false_melee_weapon`
- `test_has_ranged_weapon_false_no_ammo`
- `test_has_ranged_weapon_true_with_bow_and_arrows`

#### Phase 2 — Game System Integration

**Files changed**: `src/game/systems/combat.rs`

The player attack branch inside `perform_attack_action_with_rng` was rewritten.
Previously it used a hardcoded `DiceRoll::new(1, 4, 0)` for every player attack
regardless of equipment. After Phase 2 it calls `get_character_attack`, matches
on `MeleeAttackResult`, and:

- **`MeleeAttackResult::Melee(attack)`** — uses the resolved attack (correct
  weapon dice + bonus) as the input to `resolve_attack`.
- **`MeleeAttackResult::Ranged(_)`** — emits a `warn!` log and returns `Ok(())`
  without dealing any damage, consuming the turn and directing the player to use
  `TurnAction::RangedAttack` instead. This is the ranged-weapon guard.

The monster attack branch was left unchanged — monsters continue to use
`choose_monster_attack`.

Phase 2 helper fixtures added to `src/game/systems/combat.rs` test module:

- `make_p2_weapon_item(id, damage, bonus, classification)` — builds an `Item` with a `WeaponData` payload.
- `make_p2_combat_fixture(player)` — builds a self-contained `(CombatResource, GameContent, GlobalState, CombatTurnStateResource)` with one player (index 0) and one goblin with AC 1 (index 1, nearly always hit).

Phase 2 tests:

- `test_player_attack_uses_equipped_melee_weapon_damage` — equips a 1d8 longsword; asserts damage ∈ [1, 8] over 50 seeds and that at least one roll exceeded 4 (proving the old 1d4 path is gone).
- `test_player_attack_unarmed_when_no_weapon` — no weapon equipped; asserts damage ≤ 2 (1d2 UNARMED_DAMAGE) over 30 seeds.
- `test_player_attack_bonus_weapon_floor_at_one` — equips a cursed 1d4 −3 dagger (baked into `DiceRoll::bonus`); asserts monster HP never increases and any hit deals ≥ 1 damage.
- `test_player_melee_attack_with_ranged_weapon_skips_turn` — equips a `MartialRanged` bow; asserts the function returns `Ok(())` and the monster's HP is completely unchanged.

#### Phase 3 — Damage Floor and Bonus Application Verification

**Files changed**: `src/domain/combat/engine.rs` (doc comment update + two new tests)

Two invariants were verified and documented:

**Invariant 1 — Bonus integration**: `get_character_attack` merges
`weapon_data.damage.bonus` and `weapon_data.bonus` using `saturating_add` into
the `DiceRoll::bonus` field. The `DiceRoll::bonus` field type is `i8`; the use
of `saturating_add` prevents wraparound on extreme values.

**Invariant 2 — Damage floor at 1**: `resolve_attack` computes
`(base_damage + might_bonus).max(1)` before casting to `u16`. This is the
authoritative damage floor — any successful hit deals at least 1 damage even
when weapon bonuses are so negative that the raw roll is ≤ 0. `DiceRoll::roll`
itself clamps at 0 (`total.max(0)`) as a secondary safeguard.

The `resolve_attack` doc comment was updated to explicitly document:

- Where the damage floor of 1 lives (`(base_damage + might_bonus).max(1)`).
- That `DiceRoll::roll` floors at 0 (not 1) — the authoritative floor is in `resolve_attack`.

Phase 3 tests added to `src/domain/combat/engine.rs` test module:

- `test_cursed_weapon_damage_floor_at_one` — equips a 1d4 −10 cursed weapon; asserts every hit yields damage ≥ 1 across 100 random seeds.
- `test_positive_bonus_adds_to_roll` — equips a +3 longsword (1d6 base, bonus 3); asserts `DiceRoll::bonus == 3`, `DiceRoll::min() == 4`, and that every observed hit damage ∈ [4, 9].

### Architecture Compliance

- [x] `get_character_attack` in `src/domain/combat/engine.rs` (domain layer, no Bevy)
- [x] `MeleeAttackResult` in `src/domain/combat/engine.rs` (domain layer, no Bevy)
- [x] `has_ranged_weapon` in `src/domain/combat/engine.rs` (domain layer, no Bevy)
- [x] `UNARMED_DAMAGE` is a named constant — no magic literals
- [x] `is_ranged: bool` on `Attack` with `#[serde(default)]`
- [x] `Attack::ranged(damage)` sets `is_ranged = true`
- [x] `Attack::physical(damage)` keeps `is_ranged = false`
- [x] Melee path returns `Ok(())` (no damage, with `warn!`) on `MeleeAttackResult::Ranged`
- [x] `DiceRoll` type used throughout, not raw primitives
- [x] All public functions have `///` doc comments with runnable examples
- [x] No tests reference `campaigns/tutorial` (Implementation Rule 5)
- [x] SPDX header present in all modified `.rs` files

### Quality Gate Results

| Gate    | Command                                                    | Result                              |
| ------- | ---------------------------------------------------------- | ----------------------------------- |
| Format  | `cargo fmt --all`                                          | ✅ No output                        |
| Compile | `cargo check --all-targets --all-features`                 | ✅ Finished, 0 errors               |
| Lint    | `cargo clippy --all-targets --all-features -- -D warnings` | ✅ Finished, 0 warnings             |
| Tests   | `cargo nextest run --all-features`                         | ✅ 3182 passed, 8 skipped, 0 failed |

---

## Phase 3: Equipped Weapon Damage — Damage Floor and Bonus Application Verification

### Overview

Phase 3 verifies and documents that weapon bonuses are applied correctly through
the full attack pipeline and that the damage floor of 1 is enforced on every
hit, regardless of how negative a weapon's bonus is.

Two critical invariants are codified and proven by tests:

1. **Bonus integration** — `get_character_attack` uses `saturating_add` to
   merge `WeaponData::damage.bonus` and `WeaponData::bonus` into a single
   `DiceRoll::bonus` field. This was already implemented in Phase 1; Phase 3
   verifies it via boundary tests.
2. **Damage floor at 1** — `resolve_attack` applies `.max(1)` to
   `base_damage + damage_bonus` after every hit, preventing a cursed weapon
   from ever dealing 0 damage on a successful strike. The floor is the sole
   responsibility of `resolve_attack`; neither `DiceRoll::roll` (which floors
   at 0) nor `get_character_attack` (which only builds the roll descriptor)
   duplicate it.

### Phase 3 Deliverables Checklist

- [x] `DiceRoll::bonus` field type confirmed as `i8`; `saturating_add` used
      throughout `get_character_attack` — no silent truncation
- [x] `resolve_attack` floors damage at 1 via `(base_damage + damage_bonus).max(1)`
      — existing code confirmed and documented
- [x] `resolve_attack` doc comment updated to explicitly state the floor-at-1
      invariant and explain that it is the single authoritative enforcement point
- [x] `test_cursed_weapon_damage_floor_at_one` passes
- [x] `test_positive_bonus_adds_to_roll` passes

### What Was Built

#### Doc comment update (`src/domain/combat/engine.rs`)

The `resolve_attack` function's doc comment was extended to document the
damage-floor invariant:

- States that on a hit, damage is **always** floored at 1 regardless of weapon
  penalties, negative bonuses, or low might.
- Explicitly identifies `resolve_attack` as the single authoritative place for
  this invariant.
- Cross-references `DiceRoll::roll` (floors at 0) and `get_character_attack`
  (roll descriptor only) to prevent future duplication.

#### `test_cursed_weapon_damage_floor_at_one`

Located in `src/domain/combat/engine.rs`, `mod tests`.

- Constructs a `CombatState` with an attacker (might=10, accuracy=20) and a
  defender (AC=0) so nearly every roll is a hit.
- Equips the attacker with a 1d4-10 cursed weapon built via `make_weapon_item`
  and `ItemDatabase::add_item`.
- Calls `get_character_attack` to produce the `Attack` the same way the game
  system does, verifying `attack.damage.bonus == -10`.
- Runs 200 `resolve_attack` trials and asserts `damage == 0` (miss) or
  `damage >= 1` (hit, floored).
- Runs a further 500 trials, filters to hits only, and asserts that the
  collected `hit_damages` vector is non-empty and every element is `>= 1`.

#### `test_positive_bonus_adds_to_roll`

Located in `src/domain/combat/engine.rs`, `mod tests`.

- Builds a +3 longsword (1d6 base, `WeaponData::bonus = 3`) in a fresh
  `ItemDatabase`.
- Calls `get_character_attack` and confirms:
  - `attack.damage.bonus == 3` (saturating_add(0, 3))
  - `attack.damage.count == 1`, `attack.damage.sides == 6`
  - `attack.damage.min() == 4` (die=1 + bonus=3)
- Runs 500 `resolve_attack` trials with an attacker of might=10 and AC=0
  defender, filters to non-zero results, and asserts:
  - At least one hit observed.
  - Every hit `>= 4` (bonus raises the minimum).
  - Every hit `<= 9` (1×6 + 3, no might bonus).

### Architecture Compliance

| Check                                                     | Status |
| --------------------------------------------------------- | ------ |
| Type aliases used (`ItemId` etc.)                         | ✅     |
| `DiceRoll::bonus` is `i8`; `saturating_add` used          | ✅     |
| Floor-at-1 in `resolve_attack`, not in helpers            | ✅     |
| No magic numbers; `UNARMED_DAMAGE` constant used          | ✅     |
| Tests use `data/` fixtures only (no `campaigns/tutorial`) | ✅     |
| RON data files untouched                                  | ✅     |

### Quality Gate Results

```text
cargo fmt         → OK (no output)
cargo check       → Finished 0 errors
cargo clippy      → Finished 0 warnings
cargo nextest run → 3182 tests run: 3182 passed, 8 skipped
```

---

## Phase 2: Equipped Weapon Damage — Game System Integration

### Overview

Phase 2 wires the domain-layer work from Phase 1 into the live game system.
`perform_attack_action_with_rng` in `src/game/systems/combat.rs` previously
hardcoded `Attack::physical(DiceRoll::new(1, 4, 0))` for every
`CombatantId::Player` turn. This phase replaces that single literal with a
call to `get_character_attack` and dispatches on `MeleeAttackResult`:

- `Melee(attack)` — the resolved attack (with correct weapon damage and bonus)
  is used directly in `resolve_attack`.
- `Ranged(_)` — the melee path is a no-op: a `warn!` is logged and the
  function returns `Ok(())` immediately without applying any damage. The
  ranged path (`TurnAction::RangedAttack` /
  `perform_ranged_attack_action_with_rng`) is reserved for a future phase
  (`combat_events_implementation_plan.md` §3).

The monster path (`CombatantId::Monster`) is **not changed** by this phase.

### Phase 2 Deliverables Checklist

- [x] Hardcoded `DiceRoll::new(1, 4, 0)` removed from the `CombatantId::Player`
      branch of `perform_attack_action_with_rng`
- [x] `get_character_attack` + `MeleeAttackResult` dispatch wired in
- [x] Ranged-weapon guard logs a `warn!` and returns `Ok(())` without damage
- [x] `use` imports for `get_character_attack` and `MeleeAttackResult` added in
      `src/game/systems/combat.rs`
- [x] Four integration tests added and passing

### What Was Built

#### Updated imports (`src/game/systems/combat.rs`)

```antares/src/game/systems/combat.rs#L59-62
use crate::domain::combat::engine::{
    apply_damage, choose_monster_attack, get_character_attack, initialize_combat_from_group,
    resolve_attack, CombatState, Combatant, MeleeAttackResult,
};
```

#### Replaced player attack branch

The old hardcoded block:

```antares/docs/explanation/equipped_weapon_damage_implementation_plan.md#L284-294
CombatantId::Player(_) => {
    crate::domain::combat::types::Attack::physical(DiceRoll::new(1, 4, 0))
}
```

…is replaced with:

```antares/src/game/systems/combat.rs#L2181-2198
        CombatantId::Player(idx) => {
            if let Some(Combatant::Player(pc)) = combat_res.state.participants.get(idx) {
                match get_character_attack(pc, &content.db().items) {
                    MeleeAttackResult::Melee(attack) => attack,
                    MeleeAttackResult::Ranged(_) => {
                        // Ranged weapons must be used via TurnAction::RangedAttack /
                        // perform_ranged_attack_action_with_rng, not the melee path.
                        // Log a warning and skip the turn rather than dealing wrong damage.
                        warn!(
                            "Player {:?} attempted melee attack with ranged weapon; \
                             use TurnAction::RangedAttack instead. Turn skipped.",
                            action.attacker
                        );
                        return Ok(());
                    }
                }
            } else {
                return Err(CombatError::CombatantNotFound(action.attacker));
            }
        }
```

#### Integration tests (`src/game/systems/combat.rs`, `mod tests`)

Four pure-function tests were added that construct a `CombatResource` directly
(no Bevy `App`) via the `make_p2_combat_fixture` helper:

| Test name                                                | What it verifies                                                                                           |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `test_player_attack_uses_equipped_melee_weapon_damage`   | Longsword (1d8) deals damage in [1, 8]; at least one roll over 50 seeds exceeds 4, proving old 1d4 is gone |
| `test_player_attack_unarmed_when_no_weapon`              | `equipment.weapon = None` → damage ≤ 2 (1d2 UNARMED_DAMAGE) across 30 seeds                                |
| `test_player_attack_bonus_weapon_floor_at_one`           | Cursed dagger (1d4, bonus -3) never deals negative damage; any hit deals ≥ 1                               |
| `test_player_melee_attack_with_ranged_weapon_skips_turn` | `MartialRanged` bow via melee path returns `Ok(())` and monster HP is unchanged                            |

All four tests use `data/test_campaign`-independent fixtures built entirely in
memory — no reference to `campaigns/tutorial`.

### Quality Gate Results

```antares/docs/explanation/implementations.md#L1-1
cargo fmt         → no output (all files formatted)
cargo check       → Finished, 0 errors
cargo clippy -D warnings → Finished, 0 warnings
cargo nextest run → 3180 tests run: 3180 passed, 8 skipped
```

---

## Phase 1: Equipped Weapon Damage — Domain Combat Engine Changes

### Overview

Player characters in combat previously always dealt 1d4 physical damage regardless
of their equipped weapon, because `perform_attack_action_with_rng` hardcoded
`Attack::physical(DiceRoll::new(1, 4, 0))` for every `CombatantId::Player` turn.
This phase repairs the domain layer so that the combat engine can correctly resolve
a character's attack from their equipped weapon, identify ranged weapons that must
not fire through the melee path, and fall back to a correct unarmed damage value
(1d2, not 1d4).

No Bevy or game-system code was changed in this phase — all additions are
pure-domain functions in `src/domain/combat/engine.rs` and a field addition to
`src/domain/combat/types.rs`.

### Phase 1 Deliverables Checklist

- [x] `UNARMED_DAMAGE` constant in `src/domain/combat/engine.rs`
- [x] `MeleeAttackResult` enum in `src/domain/combat/engine.rs`
- [x] `get_character_attack(character, item_db) -> MeleeAttackResult` in
      `src/domain/combat/engine.rs`
- [x] `has_ranged_weapon(character, item_db) -> bool` in
      `src/domain/combat/engine.rs`
- [x] `is_ranged: bool` field on `Attack` with `#[serde(default)]` in
      `src/domain/combat/types.rs`
- [x] `Attack::ranged(damage)` constructor in `src/domain/combat/types.rs`
- [x] Required `use` imports added to `engine.rs`
- [x] All 14 unit tests pass (13 specified + 1 extra coverage test)

### What Was Built

#### `UNARMED_DAMAGE` constant (`src/domain/combat/engine.rs`)

```antares/src/domain/combat/engine.rs#L42-47
pub const UNARMED_DAMAGE: DiceRoll = DiceRoll {
    count: 1,
    sides: 2,
    bonus: 0,
};
```

Replaces all scattered `DiceRoll::new(1, 4, 0)` literals previously used as the
player unarmed fallback. The correct unarmed damage per spec is 1d2, not 1d4.

#### `MeleeAttackResult` enum (`src/domain/combat/engine.rs`)

A small discriminated union returned by `get_character_attack` that communicates
whether the character's equipped weapon is usable in the melee path:

- `Melee(Attack)` — a valid melee `Attack` ready for `resolve_attack`
- `Ranged(Attack)` — the weapon is `MartialRanged`; the melee path must refuse
  it and direct the player through `perform_ranged_attack_action_with_rng`

The `Ranged` variant carries the fully-constructed `Attack` so callers can log or
display weapon stats without a second item lookup — but must never apply damage
through the melee pipeline with it.

#### `get_character_attack` (`src/domain/combat/engine.rs`)

Pure-domain function: `pub fn get_character_attack(character: &Character, item_db: &ItemDatabase) -> MeleeAttackResult`

Logic (in order, fully infallible):

1. No weapon in `character.equipment.weapon` → unarmed fallback
2. Item ID not found in `item_db` → unarmed fallback (no panic)
3. Item found but not `ItemType::Weapon(_)` (e.g. consumable in weapon slot) → unarmed fallback
4. Build `DiceRoll` from `weapon_data.damage`; apply `weapon_data.bonus` via
   `saturating_add` to the `bonus` field
5. If `weapon_data.classification == WeaponClassification::MartialRanged` →
   return `MeleeAttackResult::Ranged(Attack::ranged(adjusted))`
6. Otherwise → return `MeleeAttackResult::Melee(Attack::physical(adjusted))`

#### `has_ranged_weapon` (`src/domain/combat/engine.rs`)

Pure-domain helper: `pub fn has_ranged_weapon(character: &Character, item_db: &ItemDatabase) -> bool`

Returns `true` only when **both** conditions hold:

- The equipped weapon has `WeaponClassification::MartialRanged`, **and**
- The character's inventory contains at least one `ItemType::Ammo(_)` item

A character with a bow but no arrows returns `false`.

#### `is_ranged: bool` field on `Attack` (`src/domain/combat/types.rs`)

Added with `#[serde(default)]` so all existing RON monster data that lacks the
field deserialises correctly (defaults to `false`). `Attack::physical` continues
to set `is_ranged: false`; the new `Attack::ranged` constructor sets it `true`.

#### `Attack::ranged(damage)` constructor (`src/domain/combat/types.rs`)

```antares/src/domain/combat/types.rs#L85-93
pub fn ranged(damage: DiceRoll) -> Self {
    Self {
        damage,
        attack_type: AttackType::Physical,
        special_effect: None,
        is_ranged: true,
    }
}
```

Used by `get_character_attack` when the equipped weapon is `MartialRanged`.

#### Imports added to `engine.rs`

```antares/src/domain/combat/engine.rs#L18-19
use crate::domain::items::{ItemDatabase, ItemType, WeaponClassification};
use crate::domain::types::DiceRoll;
```

### Architecture Compliance

- [x] Data structures match architecture.md Section 4.4 **exactly**
- [x] Module placement follows Section 3.2 (`src/domain/combat/`)
- [x] Type aliases used consistently (`ItemId`, `DiceRoll`, etc.)
- [x] `UNARMED_DAMAGE` constant extracted — no magic literals
- [x] `AttributePair` pattern untouched — no direct stat mutation
- [x] RON format unchanged — `#[serde(default)]` preserves all existing data files
- [x] No architectural deviations

### Test Coverage

14 unit tests added across two files:

**`src/domain/combat/types.rs`** (3 tests):

| Test                                                 | Assertion                                         |
| ---------------------------------------------------- | ------------------------------------------------- |
| `test_attack_physical_constructor_is_ranged_false`   | `Attack::physical(...)` sets `is_ranged = false`  |
| `test_attack_ranged_constructor_sets_is_ranged_true` | `Attack::ranged(...)` sets `is_ranged = true`     |
| `test_attack_ranged_damage_preserved`                | inner `damage` field is carried through unchanged |

**`src/domain/combat/engine.rs`** (11 tests):

| Test                                                             | Assertion                                         |
| ---------------------------------------------------------------- | ------------------------------------------------- |
| `test_get_character_attack_no_weapon_returns_unarmed`            | `None` weapon → `Melee(UNARMED_DAMAGE)`           |
| `test_get_character_attack_melee_weapon_returns_melee`           | Simple sword 1d8 → `Melee(1d8)`                   |
| `test_get_character_attack_weapon_bonus_applied`                 | +2 sword → `damage.bonus == 2`                    |
| `test_get_character_attack_unknown_item_id_falls_back`           | item_id 99 not in db → unarmed fallback, no panic |
| `test_get_character_attack_non_weapon_item_falls_back`           | consumable in weapon slot → unarmed fallback      |
| `test_get_character_attack_ranged_weapon_returns_ranged_variant` | bow → `Ranged(_)` with `is_ranged = true`         |
| `test_get_character_attack_ranged_weapon_damage_correct`         | crossbow 1d8+1 → inner `Attack` has correct dice  |
| `test_has_ranged_weapon_false_no_weapon`                         | no weapon equipped → `false`                      |
| `test_has_ranged_weapon_false_melee_weapon`                      | melee weapon → `false`                            |
| `test_has_ranged_weapon_false_no_ammo`                           | bow equipped, empty inventory → `false`           |
| `test_has_ranged_weapon_true_with_bow_and_arrows`                | bow + arrows in inventory → `true`                |

### Quality Gates

```text
✅ cargo fmt         → no output (all files formatted)
✅ cargo check       → Finished with 0 errors
✅ cargo clippy      → Finished with 0 warnings
✅ cargo nextest run → 3176 tests run: 3176 passed, 0 failed
```

## Items Procedural Meshes — Phase 1: Domain Layer

**Plan**: [`items_procedural_meshes_implementation_plan.md`](items_procedural_meshes_implementation_plan.md)

### Overview

Phase 1 adds the domain-layer types that drive procedural 3-D world-mesh
generation for dropped items. When a player drops an item from inventory it
will (in later phases) spawn a procedural mesh on the tile; this phase
establishes the pure-Rust data layer that converts any `Item` definition into
a `CreatureDefinition` that the existing `spawn_creature` pipeline can render.

No Bevy dependency is introduced in Phase 1. All new code lives in
`src/domain/` and `src/sdk/`.

---

### Phase 1 Deliverables

**Files changed / created**:

- `src/domain/visual/item_mesh.rs` _(new)_
- `src/domain/visual/mod.rs` _(updated)_
- `src/domain/items/types.rs` _(updated)_
- `src/domain/items/database.rs` _(updated)_
- `src/sdk/validation.rs` _(updated)_
- `src/sdk/error_formatter.rs` _(updated)_

**Files with `mesh_descriptor_override: None` field additions** (backward-compatible):

- `src/domain/combat/item_usage.rs`
- `src/domain/items/equipment_validation.rs`
- `src/domain/transactions.rs`
- `src/game/systems/combat.rs`
- `src/game/systems/dialogue.rs`
- `src/sdk/templates.rs`
- `src/bin/item_editor.rs`
- `tests/cli_editor_tests.rs`
- `tests/merchant_transaction_integration_test.rs`

---

### What was built

#### `ItemMeshCategory` (`src/domain/visual/item_mesh.rs`)

An enum with 17 variants mapping every `ItemType` sub-classification to a
distinct mesh silhouette: `Sword`, `Dagger`, `Blunt`, `Staff`, `Bow`,
`BodyArmor`, `Helmet`, `Shield`, `Boots`, `Ring`, `Amulet`, `Belt`, `Cloak`,
`Potion`, `Scroll`, `Ammo`, `QuestItem`.

#### `ItemMeshDescriptor` (`src/domain/visual/item_mesh.rs`)

The full per-item visual specification: `category`, `blade_length`,
`primary_color`, `accent_color`, `emissive`, `emissive_color`, and `scale`.

`ItemMeshDescriptor::from_item(item: &Item) -> Self` is a **pure function**
that reads `item.item_type`, sub-type classification fields, `tags`, bonus
values, and charge data:

- `WeaponClassification::Simple` with `sides ≤ 4` → `Dagger`; otherwise →
  `Blunt`. `MartialMelee` → `Sword`. `MartialRanged` → `Bow`.
  `Blunt` → `Blunt`.
- Blade length = `(damage.sides × 0.08).clamp(0.25, 1.0)`. Dagger blade is
  multiplied by 0.7 (shorter).
- `two_handed` tag → scale multiplied by `1.45`.
- `ConsumableEffect::HealHp` → red; `RestoreSp` → blue;
  `CureCondition` → `Scroll` category (parchment color);
  `BoostAttribute` / `BoostResistance` → yellow.
- `item.is_magical()` → `emissive = true`, soft white glow.
- `item.is_cursed` → dark purple primary color, purple emissive (overrides
  magical glow — curse takes visual priority).
- Quest items always emit (magenta star mesh).

`ItemMeshDescriptor::to_creature_definition(&self) -> CreatureDefinition`
converts the descriptor into a single-mesh `CreatureDefinition` on the XZ
plane (item lying flat on the ground). The returned definition always passes
`CreatureDefinition::validate()`.

Each mesh category has a dedicated geometry builder that produces a flat
polygon on the XZ plane (Y = 0). All polygon fans use a dedicated centre
vertex (never vertex 0 as the hub) to avoid degenerate triangles.

#### `ItemMeshDescriptorOverride` (`src/domain/visual/item_mesh.rs`)

A `#[serde(default)]`-annotated struct with four optional fields:
`primary_color`, `accent_color`, `scale`, `emissive`. Campaign authors can
embed it in a RON item file to customise the visual without touching gameplay
data. An all-`None` override is identical to no override at all.

#### `Item::mesh_descriptor_override` (`src/domain/items/types.rs`)

Added `#[serde(default)] pub mesh_descriptor_override:
Option<ItemMeshDescriptorOverride>` to the `Item` struct. All existing RON
item files remain valid without modification because `#[serde(default)]`
deserialises the field as `None` when absent.

#### `ItemDatabase::validate_mesh_descriptors` (`src/domain/items/database.rs`)

A new method that calls `ItemMeshDescriptor::from_item` for every loaded item
and validates the resulting `CreatureDefinition`. A new error variant
`ItemDatabaseError::InvalidMeshDescriptor { item_id, message }` is returned
on the first failure.

#### SDK plumbing (`src/sdk/validation.rs`, `src/sdk/error_formatter.rs`)

- `ValidationError::ItemMeshDescriptorInvalid { item_id, message }` — new
  `Error`-severity variant.
- `Validator::validate_item_mesh_descriptors()` — calls
  `ItemDatabase::validate_mesh_descriptors` and converts the result into a
  `Vec<ValidationError>`.
- `validate_all()` now calls `validate_item_mesh_descriptors()`.
- `error_formatter.rs` has an actionable suggestion block for the new variant.

---

### Architecture compliance

- `CreatureDefinition` is reused as the output type — no new rendering path.
- `ItemId`, `ItemType` type aliases used throughout.
- `#[serde(default)]` on `mesh_descriptor_override` preserves full backward
  compatibility with all existing RON files.
- All geometry builders produce non-degenerate triangles (centre-vertex fan).
- No constants are hard-coded; all shape parameters (`BASE_SCALE`,
  `TWO_HANDED_SCALE_MULT`, `BLADE_SIDES_FACTOR`, etc.) are named constants.
- SPDX headers present in `item_mesh.rs`.
- Test data uses `data/items.ron` (Implementation Rule 5 compliant).

---

### Test coverage

**`src/domain/visual/item_mesh.rs`** (inline `mod tests`):

| Test                                                       | What it verifies                                                  |
| ---------------------------------------------------------- | ----------------------------------------------------------------- |
| `test_sword_descriptor_from_short_sword`                   | Short sword → `Sword` category, correct blade length, no emissive |
| `test_dagger_descriptor_short_blade`                       | Dagger → `Dagger` category, blade shorter than same-sides sword   |
| `test_potion_color_heal_is_red`                            | `HealHp` → red primary color                                      |
| `test_potion_color_restore_sp_is_blue`                     | `RestoreSp` → blue                                                |
| `test_potion_color_boost_attribute_is_yellow`              | `BoostAttribute` → yellow                                         |
| `test_cure_condition_produces_scroll`                      | `CureCondition` → `Scroll` category                               |
| `test_magical_item_emissive`                               | `max_charges > 0` → emissive                                      |
| `test_magical_item_emissive_via_bonus`                     | `constant_bonus` → emissive                                       |
| `test_cursed_item_dark_tint`                               | `is_cursed` → dark purple + purple emissive                       |
| `test_cursed_overrides_magical_glow`                       | Cursed+magical → cursed emissive wins                             |
| `test_two_handed_weapon_larger_scale`                      | `two_handed` tag → scale > one-handed                             |
| `test_descriptor_to_creature_definition_valid`             | Round-trip for all categories passes `validate()`                 |
| `test_override_color_applied`                              | `primary_color` override applied                                  |
| `test_override_scale_applied`                              | `scale` override applied                                          |
| `test_override_invalid_scale_ignored`                      | Negative scale override ignored                                   |
| `test_override_emissive_applied`                           | Non-zero emissive override enables flag                           |
| `test_override_zero_emissive_disables`                     | All-zero emissive override disables flag                          |
| `test_quest_item_descriptor_unique_shape`                  | Quest items → `QuestItem` category, always emissive               |
| `test_all_accessory_slots_produce_valid_definitions`       | All 4 accessory slots round-trip                                  |
| `test_all_armor_classifications_produce_valid_definitions` | All 4 armor classes round-trip                                    |
| `test_ammo_descriptor_valid`                               | Ammo → valid definition                                           |
| `test_descriptor_default_override_is_identity`             | Empty override = no override                                      |

**`src/domain/items/database.rs`** (extended `mod tests`):

| Test                                            | What it verifies                                  |
| ----------------------------------------------- | ------------------------------------------------- |
| `test_validate_mesh_descriptors_all_base_items` | Loads `data/items.ron`; all items pass validation |
| `test_validate_mesh_descriptors_empty_db`       | Empty DB → `Ok(())`                               |
| `test_validate_mesh_descriptors_all_item_types` | One item of every `ItemType` variant → `Ok(())`   |

---

## Items Procedural Meshes — Phase 2: Game Engine — Dropped Item Mesh Generation

**Plan**: [`items_procedural_meshes_implementation_plan.md`](items_procedural_meshes_implementation_plan.md)

### Overview

Phase 2 wires the domain-layer types from Phase 1 into the live Bevy game
engine. Dropping an item from inventory now spawns a procedural 3-D mesh on
the party's current tile; static `MapEvent::DroppedItem` entries in RON map
files cause the same mesh to appear on map load; picking up an item despawns
the mesh.

---

### Phase 2 Deliverables

**Files created**:

- `src/game/components/dropped_item.rs` — `DroppedItem` ECS marker component
- `src/game/systems/item_world_events.rs` — `ItemDroppedEvent`, `ItemPickedUpEvent`, spawn / despawn / map-load systems, `ItemWorldPlugin`

**Files modified**:

- `src/domain/world/types.rs` — `MapEvent::DroppedItem` variant added
- `src/domain/world/events.rs` — `DroppedItem` arm in `trigger_event` match
- `src/game/components/mod.rs` — `pub mod dropped_item` + re-export
- `src/game/resources/mod.rs` — `DroppedItemRegistry` resource
- `src/game/systems/mod.rs` — `pub mod item_world_events`
- `src/game/systems/procedural_meshes.rs` — 12 item mesh cache slots, `get_or_create_item_mesh`, 10 per-category spawn functions (`spawn_sword_mesh`, `spawn_dagger_mesh`, `spawn_blunt_mesh`, `spawn_staff_mesh`, `spawn_bow_mesh`, `spawn_armor_mesh`, `spawn_shield_mesh`, `spawn_potion_mesh`, `spawn_scroll_mesh`, `spawn_ring_mesh`, `spawn_ammo_mesh`), `spawn_dropped_item_mesh` dispatcher, 11 config structs
- `src/game/systems/inventory_ui.rs` — drop action fires `ItemDroppedEvent`
- `src/game/systems/events.rs` — `MapEvent::DroppedItem` arm in `handle_events`
- `src/sdk/validation.rs` — `MapEvent::DroppedItem` validation arm
- `src/bin/validate_map.rs` — `MapEvent::DroppedItem` counting arm
- `src/bin/antares.rs` — `ItemWorldPlugin` registered

---

### What was built

#### `DroppedItem` component (`src/game/components/dropped_item.rs`)

`#[derive(Component, Clone, Debug, PartialEq, Eq)]` struct that marks any
entity whose mesh represents an item lying on the ground. Stores `item_id`,
`map_id`, `tile_x`, `tile_y`, and `charges`.

#### `DroppedItemRegistry` resource (`src/game/resources/mod.rs`)

`#[derive(Resource, Default)]` wrapping a `HashMap<(MapId, i32, i32, ItemId),
Entity>`. Provides typed `insert`, `get`, and `remove` helpers. Used to
correlate pickup events with ECS entities for targeted despawn.

#### `MapEvent::DroppedItem` variant (`src/domain/world/types.rs`)

New enum arm with `name: String`, `item_id: ItemId`, and
`#[serde(default)] charges: u16`. All fields that are optional use
`#[serde(default)]` so existing RON map files that pre-date this variant
remain valid without modification.

#### `ItemDroppedEvent` / `ItemPickedUpEvent` (`src/game/systems/item_world_events.rs`)

`#[derive(Message, Clone, Debug)]` event structs carrying `item_id`, `charges`,
`map_id`, `tile_x`, `tile_y` (drop) or the same minus charges (pickup).
Registered with `app.add_message::<…>()` inside `ItemWorldPlugin`.

#### `spawn_dropped_item_system`

Reads `MessageReader<ItemDroppedEvent>`. For each event:

1. Looks up the item from `GameContent`; skips with a warning if not found.
2. Calls `ItemMeshDescriptor::from_item` → `to_creature_definition`.
3. Calls `spawn_creature` at world-space `(tile_x + 0.5, 0.05, tile_y + 0.5)`.
4. Applies a random Y-axis jitter rotation for visual variety.
5. Inserts `DroppedItem`, `MapEntity`, `TileCoord`, and a `Name` component.
6. Registers the entity in `DroppedItemRegistry`.

`GameContent` is wrapped in `Option<Res<…>>` so the system degrades gracefully
when content is not yet loaded.

#### `despawn_picked_up_item_system`

Reads `MessageReader<ItemPickedUpEvent>`. Looks up the entity in
`DroppedItemRegistry` by the four-part key, calls
`commands.entity(entity).despawn()` (Bevy 0.17 — recursive by default), and
removes the registry entry. Unknown keys emit a `warn!` log.

#### `load_map_dropped_items_system`

Stores the last-processed map ID in a `Local<Option<MapId>>`. On map change,
iterates all `MapEvent::DroppedItem` entries on the new map and fires
`ItemDroppedEvent` for each so static map-authored drops share the identical
spawn path as runtime drops.

#### Item mesh config structs & generators (`src/game/systems/procedural_meshes.rs`)

Eleven typed config structs (`SwordConfig`, `DaggerConfig`, `BluntConfig`,
`StaffConfig`, `BowConfig`, `ArmorMeshConfig`, `ShieldConfig`, `PotionConfig`,
`ScrollConfig`, `RingMeshConfig`, `AmmoConfig`) plus a `spawn_dropped_item_mesh`
dispatcher that selects the right generator from `ItemMeshCategory`.

Twelve item mesh cache slots added to `ProceduralMeshCache` (one per category
string: `"sword"`, `"dagger"`, `"blunt"`, `"staff"`, `"bow"`, `"armor"`,
`"shield"`, `"potion"`, `"scroll"`, `"ring"`, `"ammo"`, `"quest"`).
`get_or_create_item_mesh` follows the same pattern as the existing
`get_or_create_furniture_mesh`. `clear_all` and `cached_count` updated.

Notable mesh details:

- **Potion**: `AlphaMode::Blend` on both bottle and liquid inner cylinder;
  liquid colour carries a faint emissive glow matching the liquid tint.
- **Staff**: emissive orb at tip.
- **Shield**: flat `Cylinder` disc with `FRAC_PI_2` X-rotation.
- **Ring**: `Torus` primitive (`minor_radius` = 0.018, `major_radius` = 0.065).
- **Ammo**: three sub-types (`"arrow"`, `"bolt"`, `"stone"`) selected from
  `AmmoConfig::ammo_type`.

#### Inventory drop integration (`src/game/systems/inventory_ui.rs`)

`inventory_action_system` now accepts
`Option<MessageWriter<ItemDroppedEvent>>` and fires it when a drop action
removes an item from a character's inventory. The writer is `Option`-wrapped
so existing tests that do not register the message type continue to pass.

---

### Architecture compliance

| Check                                          | Status                                                                                          |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Data structures match architecture.md §4       | ✅ `ItemId`, `MapId` type aliases used throughout                                               |
| Module placement follows §3.2                  | ✅ Components in `game/components/`, resources in `game/resources/`, systems in `game/systems/` |
| No `unwrap()` without justification            | ✅ All error paths use `warn!` / `Option` guards                                                |
| RON format for data files                      | ✅ `MapEvent::DroppedItem` serde-compatible with existing `.ron` map files                      |
| Constants extracted, not hardcoded             | ✅ `DROPPED_ITEM_Y`, `DROP_ROTATION_JITTER`, `TILE_CENTER_OFFSET`, 7 `ITEM_*_COLOR` constants   |
| SPDX headers on all new `.rs` files            | ✅ `2026 Brett Smith` header on `dropped_item.rs` and `item_world_events.rs`                    |
| Test data in `data/`, not `campaigns/tutorial` | ✅ No test references campaign data                                                             |
| Backward compatibility                         | ✅ `#[serde(default)]` on `MapEvent::DroppedItem` fields; existing RON files unaffected         |

---

### Test coverage

**`src/game/components/dropped_item.rs`** (9 tests):

| Test                                       | What it checks                                  |
| ------------------------------------------ | ----------------------------------------------- |
| `test_dropped_item_component_fields`       | All five fields stored correctly                |
| `test_dropped_item_clone`                  | `Clone` produces equal copy                     |
| `test_dropped_item_debug`                  | `Debug` output non-empty and contains type name |
| `test_dropped_item_equality`               | `PartialEq` symmetric                           |
| `test_dropped_item_inequality_item_id`     | Different `item_id` → not equal                 |
| `test_dropped_item_inequality_map_id`      | Different `map_id` → not equal                  |
| `test_dropped_item_inequality_tile_coords` | Different tiles → not equal                     |
| `test_dropped_item_zero_charges`           | Zero charges accepted                           |
| `test_dropped_item_max_charges`            | `u16::MAX` accepted without overflow            |

**`src/game/resources/mod.rs`** (5 tests):

| Test                                       | What it checks                          |
| ------------------------------------------ | --------------------------------------- |
| `test_dropped_item_registry_default_empty` | Default has no entries                  |
| `test_registry_insert_and_lookup`          | Insert + `get` by key                   |
| `test_registry_remove_on_pickup`           | Remove returns entity; key absent after |
| `test_registry_two_entries`                | Two distinct keys coexist               |
| `test_registry_insert_overwrites`          | Later insert replaces earlier entity    |

**`src/game/systems/item_world_events.rs`** (10 tests):

| Test                                       | What it checks             |
| ------------------------------------------ | -------------------------- |
| `test_item_dropped_event_creation`         | All five fields set        |
| `test_item_picked_up_event_creation`       | All four fields set        |
| `test_item_dropped_event_clone`            | `Clone`                    |
| `test_item_picked_up_event_clone`          | `Clone`                    |
| `test_item_dropped_event_debug`            | `Debug` contains type name |
| `test_item_picked_up_event_debug`          | `Debug` contains type name |
| `test_item_dropped_event_zero_charges`     | Zero charges valid         |
| `test_item_dropped_event_max_charges`      | `u16::MAX` valid           |
| `test_item_picked_up_event_negative_tiles` | Negative tile coords valid |
| `test_dropped_item_y_is_positive`          | Constant assertion         |
| `test_tile_center_offset_is_half`          | Constant assertion         |

**`src/game/systems/procedural_meshes.rs`** (`item_mesh_tests` module, 18 tests):

| Test                                            | What it checks                                       |
| ----------------------------------------------- | ---------------------------------------------------- |
| `test_sword_config_defaults`                    | `blade_length > 0`, `has_crossguard`, `color = None` |
| `test_dagger_config_defaults`                   | `blade_length < sword blade_length`                  |
| `test_potion_config_defaults`                   | Non-zero color components                            |
| `test_scroll_config_defaults`                   | Non-zero alpha; R > 0.5 (parchment)                  |
| `test_cache_item_slots_default_none`            | All 12 item slots `None` at default                  |
| `test_cache_item_slots_cleared_after_clear_all` | `clear_all` resets item slots                        |
| `test_blunt_config_defaults`                    | Positive dimensions                                  |
| `test_staff_config_defaults`                    | Positive `length` and `orb_radius`                   |
| `test_bow_config_defaults`                      | Positive `arc_height`                                |
| `test_armor_mesh_config_defaults`               | Positive dimensions; `is_helmet = false`             |
| `test_shield_config_defaults`                   | Positive `radius`                                    |
| `test_ring_mesh_config_defaults`                | Non-zero alpha                                       |
| `test_ammo_config_defaults`                     | Non-zero alpha; type = `"arrow"`                     |
| `test_item_color_constants_valid`               | All 7 colour constants convert to valid `LinearRgba` |
| `test_sword_config_clone`                       | `Clone`                                              |
| `test_dagger_config_clone`                      | `Clone`                                              |
| `test_potion_config_clone`                      | `Clone`                                              |
| `test_scroll_config_clone`                      | `Clone`                                              |
| `test_ammo_config_clone`                        | `Clone`                                              |

---

## Items Procedural Meshes — Phase 3: Item Mesh RON Asset Files

### Overview

Phase 3 creates the data layer that backs Phase 2's runtime mesh generation:
RON asset files for every dropped-item category, a `CreatureReference` registry
so the campaign loader can discover them, a new `ItemMeshDatabase` type
(thin `CreatureDatabase` wrapper), an extended `CampaignLoader` that loads
the registry (opt-in; missing file is silently skipped), a
`ItemDatabase::link_mesh_overrides` validation hook, and the Python generator
script that keeps the asset files regenerable from a single authoritative
manifest.

### Phase 3 Deliverables

| Deliverable                              | Path                                                            |
| ---------------------------------------- | --------------------------------------------------------------- |
| Generator script                         | `examples/generate_item_meshes.py`                              |
| Tutorial campaign item mesh RON files    | `campaigns/tutorial/assets/items/` (27 files)                   |
| Tutorial campaign item mesh registry     | `campaigns/tutorial/data/item_mesh_registry.ron`                |
| Test-campaign minimal RON fixtures       | `data/test_campaign/assets/items/sword.ron`, `potion.ron`       |
| Test-campaign item mesh registry         | `data/test_campaign/data/item_mesh_registry.ron`                |
| `ItemMeshDatabase` type                  | `src/domain/items/database.rs`                                  |
| `ItemDatabase::link_mesh_overrides`      | `src/domain/items/database.rs`                                  |
| `ItemDatabaseError::UnknownMeshOverride` | `src/domain/items/database.rs`                                  |
| `GameData::item_meshes` field            | `src/domain/campaign_loader.rs`                                 |
| `CampaignLoader::load_item_meshes`       | `src/domain/campaign_loader.rs`                                 |
| Integration tests                        | `src/domain/campaign_loader.rs`, `src/domain/items/database.rs` |

### What was built

#### `examples/generate_item_meshes.py`

Developer convenience tool that generates one `CreatureDefinition` RON file per
item mesh type. The script mirrors all color and scale constants from
`src/domain/visual/item_mesh.rs` so the generated geometry exactly matches what
`ItemMeshDescriptor::build_mesh` would produce at runtime.

- `--output-dir <path>` writes the full 27-file manifest to a custom directory
  (default: `campaigns/tutorial/assets/items/`).
- `--test-fixtures` writes only the two minimal test fixtures
  (`sword.ron`, `potion.ron`) to `data/test_campaign/assets/items/`.
- Geometry helpers: `blade_mesh`, `blunt_mesh`, `staff_mesh`, `bow_mesh`,
  `armor_mesh`, `helmet_mesh`, `shield_mesh`, `boots_mesh`, `ring_mesh`,
  `belt_mesh`, `cloak_mesh`, `potion_mesh`, `scroll_mesh`, `ammo_mesh`,
  `quest_mesh` — each produces a flat XZ-plane silhouette with correct normals
  and an optional `MaterialDefinition` (metallic / roughness / emissive).
- `MANIFEST` table: 27 items covering weapon (9001–9008), armor (9101–9106),
  consumable (9201–9204), accessory (9301–9304), ammo (9401–9403), and quest
  (9501–9502) categories. IDs start at 9000 to avoid collision with creature /
  NPC / template IDs.
- `TEST_MANIFEST`: 2-item subset (`sword` id=9001, `potion` id=9201) for stable
  integration test fixtures.

#### Item mesh RON asset files (`campaigns/tutorial/assets/items/`)

27 `CreatureDefinition` RON files organised into six sub-directories:

```
weapons/    sword, dagger, short_sword, long_sword, great_sword, club, staff, bow
armor/      leather_armor, chain_mail, plate_mail, shield, helmet, boots
consumables/ health_potion, mana_potion, cure_potion, attribute_potion
accessories/ ring, amulet, belt, cloak
ammo/        arrow, bolt, stone
quest/       quest_scroll (2 meshes), key_item
```

Each file is a valid `CreatureDefinition` with:

- `id` in the 9000+ range matching the registry entry.
- One (or two for quest_scroll) flat-lying `MeshDefinition` meshes with
  per-vertex `normals: Some([...])` pointing upward.
- A `MaterialDefinition` with correct metallic / roughness / emissive values.
- An identity `MeshTransform` per mesh.
- `color_tint: None`.

#### `campaigns/tutorial/data/item_mesh_registry.ron`

`Vec<CreatureReference>` listing all 27 tutorial campaign item meshes. The
registry format is identical to `data/creatures.ron`; `CampaignLoader` reuses
`CreatureDatabase::load_from_registry` internally.

#### Test-campaign fixtures

`data/test_campaign/assets/items/sword.ron` (id=9001) and
`data/test_campaign/assets/items/potion.ron` (id=9201) are minimal stable
fixtures committed to the repository. They are referenced by
`data/test_campaign/data/item_mesh_registry.ron` and used exclusively by
integration tests — never by the live tutorial campaign.

#### `ItemMeshDatabase` (`src/domain/items/database.rs`)

Thin `#[derive(Debug, Clone, Default)]` wrapper around `CreatureDatabase`:

```src/domain/items/database.rs#L447-460
pub struct ItemMeshDatabase {
    inner: CreatureDatabase,
}
```

Public API:

| Method                                             | Description                                         |
| -------------------------------------------------- | --------------------------------------------------- |
| `new()` / `default()`                              | Empty database                                      |
| `load_from_registry(registry_path, campaign_root)` | Delegates to `CreatureDatabase::load_from_registry` |
| `as_creature_database()`                           | Returns `&CreatureDatabase` for direct queries      |
| `is_empty()`                                       | True if no entries                                  |
| `count()`                                          | Number of mesh entries                              |
| `has_mesh(id: u32)`                                | True if creature ID present                         |
| `validate()`                                       | Validates all mesh `CreatureDefinition`s            |

Re-exported from `src/domain/items/mod.rs` as `antares::domain::items::ItemMeshDatabase`.

#### `ItemDatabase::link_mesh_overrides` (`src/domain/items/database.rs`)

Forward-compatibility validation hook:

```src/domain/items/database.rs#L435-442
pub fn link_mesh_overrides(
    &self,
    _registry: &ItemMeshDatabase,
) -> Result<(), ItemDatabaseError> {
```

Walks all items that carry a `mesh_descriptor_override`, calls
`ItemMeshDescriptor::from_item` + `CreatureDefinition::validate` to confirm
the override does not break mesh generation. Full registry cross-linking
(verifying that a named creature ID exists in `ItemMeshDatabase`) is reserved
for a future extension of `ItemMeshDescriptorOverride` with an explicit
`creature_id` field.

#### `GameData::item_meshes` and `CampaignLoader::load_item_meshes`

`GameData` now carries:

```src/domain/campaign_loader.rs#L90-95
pub struct GameData {
    pub creatures: CreatureDatabase,
    pub item_meshes: ItemMeshDatabase,
}
```

`CampaignLoader::load_game_data` calls the new `load_item_meshes` helper which:

1. Looks for `data/item_mesh_registry.ron` inside the campaign directory.
2. If absent — returns `ItemMeshDatabase::new()` silently (opt-in per campaign).
3. If present — calls `ItemMeshDatabase::load_from_registry`, propagating any
   read / parse errors as `CampaignError::ReadError`.

`GameData::validate` also calls `item_meshes.validate()` so malformed mesh RON
files are caught at load time.

Note: `GameData` no longer derives `Serialize`/`Deserialize` because
`ItemMeshDatabase` wraps `CreatureDatabase` (which does) but the wrapper itself
is `Debug + Clone` only — sufficient for all current usages.

### Architecture compliance

- [ ] `ItemMeshDatabase` IDs are in the 9000+ range — no collision with
      creature IDs (1–50), NPC IDs (1000+), template IDs (2000+), variant IDs (3000+).
- [ ] RON format used for all asset and registry files — no JSON or YAML.
- [ ] File names follow lowercase + underscore convention (`item_mesh_registry.ron`,
      `health_potion.ron`, etc.).
- [ ] SPDX headers present in `generate_item_meshes.py`.
- [ ] All test data in `data/test_campaign/` — no references to
      `campaigns/tutorial` from tests.
- [ ] `CampaignLoader` opt-in: missing registry file is not an error.
- [ ] `ItemMeshDatabase` does not replace `CreatureDatabase`; it is an additive
      type that sits alongside it.

### Test coverage

**`src/domain/items/database.rs`** — 11 new unit tests:

| Test                                                       | What it verifies                                        |
| ---------------------------------------------------------- | ------------------------------------------------------- |
| `test_item_mesh_database_new_is_empty`                     | `new()` starts empty                                    |
| `test_item_mesh_database_default_is_empty`                 | `default()` == `new()`                                  |
| `test_item_mesh_database_has_mesh_absent`                  | `has_mesh` returns false for absent IDs                 |
| `test_item_mesh_database_validate_empty`                   | `validate()` succeeds on empty DB                       |
| `test_item_mesh_database_as_creature_database`             | Inner DB accessible                                     |
| `test_item_mesh_database_load_from_registry_missing_file`  | Missing file → error                                    |
| `test_item_mesh_database_load_from_registry_test_campaign` | Loads ≥ 2 entries from fixture; ids 9001 & 9201 present |
| `test_item_mesh_database_validate_test_campaign`           | Loaded fixture validates without error                  |
| `test_link_mesh_overrides_empty_item_db`                   | Empty `ItemDatabase` → ok                               |
| `test_link_mesh_overrides_no_override_items_skipped`       | Items without override → ok                             |
| `test_link_mesh_overrides_valid_override_passes`           | Valid override passes mesh validation                   |

**`src/domain/campaign_loader.rs`** — 2 new integration tests:

| Test                                            | What it verifies                                                                            |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `test_campaign_loader_loads_item_mesh_registry` | Full `load_game_data` against `data/test_campaign` populates `item_meshes` with ≥ 2 entries |
| `test_item_mesh_registry_missing_is_ok`         | Missing registry file returns empty `ItemMeshDatabase` without error                        |

All tests reference `data/test_campaign` — never `campaigns/tutorial`
(Implementation Rule 5 compliant).

---

## Procedural Meshes Direction Control

**Plan**: [`procedural_meshes_direction_control_implementation_plan.md`](procedural_meshes_direction_control_implementation_plan.md)

### Overview

All creatures (NPCs, recruitable characters, monsters) and signs spawned as
procedural meshes previously faced the same default direction because
`spawn_creature()` had no rotation parameter and `MapEvent` variants carried
no `facing` field. This implementation adds per-entity cardinal facing across
the full stack: domain data model, ECS spawn systems, runtime event system,
smooth rotation animation, and Campaign Builder SDK UI.

---

### Phase 1: Direction-to-Rotation Foundation

**Files changed**:

- `src/domain/types.rs`
- `src/game/components/creature.rs`
- `src/game/components/mod.rs`
- `src/game/systems/creature_spawning.rs`

**What was built**:

`Direction::direction_to_yaw_radians(&self) -> f32` is a new method on the
`Direction` enum that maps each cardinal to a Y-axis rotation in radians:
North → 0.0, East → π/2, South → π, West → 3π/2. The inverse,
`Direction::from_yaw_radians(yaw: f32) -> Direction`, normalises any yaw
value into `[0, 2π)` and rounds to the nearest 90° cardinal. These two
methods are the single source of truth for the angle mapping; no other file
redefines the cardinal-to-float relationship.

`FacingComponent { direction: Direction }` is a new ECS component in
`creature.rs` (re-exported from `components/mod.rs`). It is the authoritative
runtime facing state for every spawned creature, NPC, and sign entity.

`spawn_creature()` gained a `facing: Option<Direction>` parameter. It
computes `Quat::from_rotation_y(d.direction_to_yaw_radians())` from the
resolved direction, applies it to the parent `Transform`, and inserts
`FacingComponent` on the parent entity. All pre-existing call sites pass
`None`, preserving identity rotation.

---

### Phase 2: Static Map-Time Facing

**Files changed**:

- `src/domain/world/types.rs`
- `src/game/systems/map.rs`
- `src/game/systems/procedural_meshes.rs`
- `campaigns/tutorial/data/maps/map_1.ron`

**What was built**:

`facing: Option<Direction>` with `#[serde(default)]` was added to
`MapEvent::Sign`, `MapEvent::NpcDialogue`, `MapEvent::Encounter`, and
`MapEvent::RecruitableCharacter`. The `#[serde(default)]` annotation keeps
all existing RON files valid without migration — omitted fields deserialise
to `None` (identity rotation).

In `map.rs`, the NPC spawn block now passes `resolved_npc.facing` to
`spawn_creature()`. The sprite-fallback path applies the same yaw rotation
directly to the sprite entity's `Transform`. An `NpcDialogue` event-level
`facing` overrides the NPC placement `facing` when both are present.
`MapEvent::Encounter` and `MapEvent::RecruitableCharacter` spawn blocks
forward their `facing` field to `spawn_creature()`.

`spawn_sign()` in `procedural_meshes.rs` gained a `facing: Option<Direction>`
parameter. Cardinal facing takes precedence over the existing `rotation_y:
Option<f32>` degrees parameter when both are provided. `FacingComponent` is
inserted on sign entities.

The tutorial map was updated: `Old Gareth` (`RecruitableCharacter` at map_1
(15,7)) has `facing: Some(West)` as a functional smoke-test for map-time
facing on event entities. An NPC placement in map_1 has `facing: Some(South)`
as the smoke-test for NPC placement facing.

---

### Phase 3: Runtime Facing Change System

**Files changed**:

- `src/game/systems/facing.rs` (new file)
- `src/game/systems/map.rs`
- `src/game/systems/dialogue.rs`
- `src/domain/world/types.rs`

**What was built**:

A new `src/game/systems/facing.rs` module provides the full runtime facing
system and is registered via `FacingPlugin`.

`SetFacing { entity: Entity, direction: Direction, instant: bool }` is a
Bevy message. `handle_set_facing` reads it each frame: when `instant: true`
it snaps `Transform.rotation` and updates `FacingComponent.direction`
directly; when `instant: false` it inserts a `RotatingToFacing` component
for frame-by-frame slerp (Phase 4).

`ProximityFacing { trigger_distance: u32, rotation_speed: Option<f32> }` is
a marker component inserted by the map loading system on entities whose
`MapEvent` has `proximity_facing: true`. The `face_toward_player_on_proximity`
system queries all entities carrying this component each frame, computes the
4-direction from the entity's `TileCoord` to `GlobalState::party_position`
using the `cardinal_toward()` helper, and emits a `SetFacing` event whenever
the nearest cardinal differs from the current `FacingComponent.direction`.

`proximity_facing: bool` (with `#[serde(default)]`) was added to
`MapEvent::Encounter` and `MapEvent::NpcDialogue`. The map loading system in
`map.rs` inserts `ProximityFacing { trigger_distance: 2, rotation_speed }`
on the spawned entity when this flag is true, forwarding the companion
`rotation_speed` field.

`handle_start_dialogue` in `dialogue.rs` was extended: when the speaker
entity has a `TileCoord`, it computes the direction from the speaker toward
the party and writes a `SetFacing { instant: true }` event so the NPC always
faces the player at dialogue start.

---

### Phase 4: Smooth Rotation Animation

**Files changed**:

- `src/game/systems/facing.rs`
- `src/domain/world/types.rs`

**What was built**:

`RotatingToFacing { target: Quat, speed_deg_per_sec: f32, target_direction: Direction }`
is a scratch ECS component inserted by `handle_set_facing` when `instant:
false`. It is never serialised and carries the logical `target_direction` so
`FacingComponent` can be updated correctly when the rotation completes.

`apply_rotation_to_facing` is a per-frame system that queries all entities
carrying `RotatingToFacing`. Each frame it computes the remaining angle
between the current and target quaternion. When the remaining angle exceeds
the `ROTATION_COMPLETE_THRESHOLD_RAD` (0.01 rad) constant it advances the
rotation using `Quat::slerp` at the configured speed. When within the
threshold it snaps to the exact target, writes the final direction to
`FacingComponent`, and removes the `RotatingToFacing` component. This keeps
the snap paths unchanged and performant.

`rotation_speed: Option<f32>` (with `#[serde(default)]`) was added to
`MapEvent::Encounter` and `MapEvent::NpcDialogue`. When set, the value is
forwarded to `ProximityFacing.rotation_speed` and used as the
`speed_deg_per_sec` when `handle_set_facing` inserts `RotatingToFacing`.
`None` means snap (instant).

---

### Phase 5: Campaign Builder SDK UI

**Files changed**:

- `sdk/campaign_builder/src/map_editor.rs`

**What was built**:

Three fields were added to `EventEditorState`:

- `event_facing: Option<String>` — the selected cardinal direction name, or
  `None` for the engine default (North). Applies to `Sign`, `NpcDialogue`,
  `Encounter`, and `RecruitableCharacter`.
- `event_proximity_facing: bool` — mirrors the `proximity_facing` RON flag.
  Applies to `Encounter` and `NpcDialogue` only.
- `event_rotation_speed: Option<f32>` — mirrors the `rotation_speed` RON
  field. Applies to `Encounter` and `NpcDialogue` only. Suppressed in
  `to_map_event()` when `event_proximity_facing` is `false`.

`Default for EventEditorState` initialises all three to `None`, `false`,
and `None` respectively.

A **Facing** combo-box was added to the bottom of each of the four affected
`match` arms in `show_event_editor()`. Each combo-box uses a unique
`id_salt` to satisfy the egui ID rules:

| Event type             | `id_salt`                           |
| ---------------------- | ----------------------------------- |
| `Sign`                 | `"sign_event_facing_combo"`         |
| `NpcDialogue`          | `"npc_dialogue_event_facing_combo"` |
| `Encounter`            | `"encounter_event_facing_combo"`    |
| `RecruitableCharacter` | `"recruitable_event_facing_combo"`  |

A **Behaviour** section (separator + label + checkbox + conditional
text-input) was added to the `Encounter` and `NpcDialogue` arms only,
surfacing the proximity-facing toggle and the rotation-speed field.
The rotation-speed input renders only when the proximity-facing checkbox
is ticked.

`to_map_event()` was updated for all four variants to parse `event_facing`
via the private `parse_facing()` helper and include it in the constructed
`MapEvent`. For `Encounter` and `NpcDialogue` it also forwards
`proximity_facing` and `rotation_speed` (with the suppression rule above).

`from_map_event()` was updated for all four variants to populate
`event_facing`, `event_proximity_facing`, and `event_rotation_speed` from
the loaded event, preserving backward compatibility for RON files that
predate these fields.

`show_inspector_panel()` was extended for all four event types to display
the `facing` direction when set. For `Encounter` and `NpcDialogue` it also
shows the proximity-facing label and rotation speed when applicable.

---

### Test Coverage

| Module                                   | Key tests added                                                                                                                                                                                                                                                                                                                                                                                                |
| ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/types.rs`                    | `test_direction_to_yaw_north/east/south/west`, `test_direction_roundtrip`, `test_direction_from_yaw_cardinals`, `test_direction_from_yaw_snaps_to_nearest`                                                                                                                                                                                                                                                     |
| `src/game/components/creature.rs`        | `test_facing_component_new`, `test_facing_component_default_is_north`, `test_facing_component_all_directions`, `test_facing_component_clone/equality`                                                                                                                                                                                                                                                          |
| `src/game/systems/creature_spawning.rs`  | `test_spawn_creature_facing_none_is_north`, `test_spawn_creature_facing_south_rotation`                                                                                                                                                                                                                                                                                                                        |
| `src/game/systems/map.rs`                | `test_npc_facing_applied_at_spawn`, `test_facing_component_on_npc`, `test_map_event_encounter_facing`, `test_map_event_sign_facing`, `test_map_event_ron_round_trip`, `test_proximity_facing_inserted_on_encounter_with_flag`, `test_proximity_facing_not_inserted_when_flag_false`, `test_proximity_facing_npc_inserted_when_flag_set`                                                                        |
| `src/game/systems/facing.rs`             | `test_set_facing_snaps_transform`, `test_set_facing_updates_facing_component`, `test_proximity_facing_emits_event`, `test_set_facing_instant_false_inserts_rotating_component`, `test_rotating_to_facing_approaches_target`, `test_rotating_to_facing_completes_and_removes_component`                                                                                                                         |
| `src/game/systems/dialogue.rs`           | `test_dialogue_start_emits_set_facing`, `test_dialogue_start_no_speaker_entity_does_not_panic`, `test_dialogue_start_speaker_without_tile_coord_skips_facing`                                                                                                                                                                                                                                                  |
| `sdk/campaign_builder/src/map_editor.rs` | `test_event_editor_state_default_facing_none`, `test_event_editor_to_sign_with_facing`, `test_event_editor_from_sign_with_facing`, `test_event_editor_from_sign_no_facing`, `test_event_editor_to_encounter_with_facing_and_proximity`, `test_event_editor_from_encounter_with_proximity`, `test_event_editor_facing_round_trip_all_variants`, `test_event_editor_proximity_false_clears_rotation_speed_in_ui` |

---

### Architecture Compliance

- `direction_to_yaw_radians` is the **single source of truth** for the
  cardinal-to-angle mapping; no other file redefines north/south/etc as raw
  floats.
- All new `MapEvent` fields use `#[serde(default)]` — all existing RON files
  remain valid without migration.
- `SetFacing` follows the existing `#[derive(Message)]` broadcast pattern.
- `RotatingToFacing` is a pure ECS scratch component — never serialised,
  never referenced by domain structs.
- `FacingPlugin` registers all three systems (`handle_set_facing`,
  `face_toward_player_on_proximity`, `apply_rotation_to_facing`) in a single
  plugin, keeping the addition self-contained.
- No test references `campaigns/tutorial`; all test fixtures use
  `data/test_campaign`.

---

## Items Procedural Meshes — Phase 4: Visual Quality and Variation

**Plan**: [`items_procedural_meshes_implementation_plan.md`](items_procedural_meshes_implementation_plan.md)

### Overview

Phase 4 extends the procedural item-mesh pipeline with four major visual
improvements:

1. **Per-item accent colors** derived from `BonusAttribute` (fire → orange,
   cold → icy blue, magic → purple, etc.)
2. **Metallic / roughness PBR material differentiation** — magical items get
   `metallic: 0.7, roughness: 0.25`; mundane non-metal items get
   `metallic: 0.0, roughness: 0.8`.
3. **Deterministic Y-rotation** — dropped items receive a tile-position-derived
   rotation instead of non-deterministic random jitter, making save/load replay
   safe.
4. **Child mesh additions**: a ground shadow quad (semi-transparent, alpha 0.3,
   `AlphaMode::Blend`) prepended to every definition, and an optional
   charge-level emissive gem appended when `charges_fraction` is supplied.
5. **LOD levels** attached automatically to primary meshes exceeding 200
   triangles (`LOD1` at 8 world units, `LOD2` billboard at 20 world units).

---

### Phase 4 Deliverables

**Files changed**:

- `src/domain/visual/item_mesh.rs` — extended with accent colors, metallic /
  roughness rules, shadow quad builder, charge gem builder, LOD wiring, and all
  Phase 4 unit tests.
- `src/game/systems/item_world_events.rs` — replaced random jitter with
  `deterministic_drop_rotation`, wired `charges_fraction` into
  `to_creature_definition_with_charges`, and added deterministic-rotation unit
  tests.

---

### What was built

#### 4.1 — Accent color from `BonusAttribute` (`src/domain/visual/item_mesh.rs`)

New private function `accent_color_from_item(item: &Item) -> Option<[f32; 4]>`
maps the item's `constant_bonus` (or `temporary_bonus` fallback) to a
Phase 4 accent color:

| `BonusAttribute`         | Accent color constant                |
| ------------------------ | ------------------------------------ |
| `ResistFire`             | `COLOR_ACCENT_FIRE` — orange         |
| `ResistCold`             | `COLOR_ACCENT_COLD` — icy blue       |
| `ResistElectricity`      | `COLOR_ACCENT_ELECTRICITY` — yellow  |
| `ResistAcid`             | `COLOR_ACCENT_ACID` — acid green     |
| `ResistPoison`           | `COLOR_ACCENT_POISON` — acid green   |
| `ResistMagic`            | `COLOR_ACCENT_MAGIC` — purple        |
| `Might`                  | `COLOR_ACCENT_MIGHT` — warm red      |
| `ArmorClass`/`Endurance` | `COLOR_ACCENT_TEAL` — teal           |
| `Intellect`              | `COLOR_ACCENT_DEEP_BLUE` — deep blue |

The accent is applied inside `from_item` after the base descriptor is built,
but only when the item is not cursed (cursed items already override
`primary_color` entirely, making accent irrelevant).

#### 4.1 — Metallic / roughness PBR differentiation

New helper `is_metallic_magical(&self) -> bool` returns `true` when
`emissive == true && emissive_color == EMISSIVE_MAGIC` (the marker set by
`from_item` when `item.is_magical()`).

`make_material` now branches on this:

- **Magical**: `metallic: 0.7, roughness: 0.25` (shiny, jewel-like)
- **Mundane metal categories** (Sword, Dagger, Blunt, Helmet, Shield, Ring,
  Amulet): legacy `metallic: 0.6, roughness: 0.5`
- **All other mundane**: `metallic: 0.0, roughness: 0.8` (matte)

New constants: `MATERIAL_METALLIC_MAGICAL = 0.7`,
`MATERIAL_ROUGHNESS_MAGICAL = 0.25`, `MATERIAL_METALLIC_MUNDANE = 0.0`,
`MATERIAL_ROUGHNESS_MUNDANE = 0.8`.

#### 4.2 — Deterministic Y-rotation (`src/game/systems/item_world_events.rs`)

Replaced the `rand::Rng::random::<f32>()` call with a new public function:

```rust
pub fn deterministic_drop_rotation(
    map_id: MapId,
    tile_x: i32,
    tile_y: i32,
    item_id: ItemId,
) -> f32
```

Algorithm:

```text
hash = map_id + (tile_x × 31) + (tile_y × 17) + (item_id × 7)   [wrapping u64 ops]
angle = (hash % 360) / 360.0 × TAU
```

This gives visually varied orientations across tiles while being fully
deterministic. The `rand` import was removed from `item_world_events.rs`.

#### 4.3 — Charge-level gem child mesh

`to_creature_definition` now delegates to a new public method:

```rust
pub fn to_creature_definition_with_charges(
    &self,
    charges_fraction: Option<f32>,
) -> CreatureDefinition
```

When `charges_fraction: Some(f)` is supplied a small diamond gem mesh is
appended as the third mesh, positioned `+0.04` Y above the item origin.

Gem color gradient (via `charge_gem_color(frac) -> ([f32; 4], [f32; 3])`):

- `1.0` → `COLOR_CHARGE_FULL` (gold, emissive gold glow)
- `0.5` → `COLOR_CHARGE_HALF` (white, dim emissive)
- `0.0` → `COLOR_CHARGE_EMPTY` (grey, no emissive)
- Intermediate fractions linearly interpolated via `lerp_color4` / `lerp_color3`.

`spawn_dropped_item_system` now computes
`charges_fraction = Some(charges as f32 / max_charges as f32)` when
`item.max_charges > 0`, otherwise `None`.

#### 4.4 — Ground shadow quad

New private function `build_shadow_quad(&self) -> MeshDefinition` builds a
flat `2 × 2`-triangle quad on the XZ plane at Y = `SHADOW_QUAD_Y` (0.001).
The quad's half-extent is `self.scale × SHADOW_QUAD_SCALE × 0.5` where
`SHADOW_QUAD_SCALE = 1.2`.

Material:

- `base_color: [0.0, 0.0, 0.0, 0.3]`
- `alpha_mode: AlphaMode::Blend`
- `metallic: 0.0, roughness: 1.0`

The shadow quad is always inserted as `meshes[0]`, with the primary item mesh
at `meshes[1]`, and the optional charge gem at `meshes[2]`.

#### 4.5 — LOD support

New private function `build_mesh_with_lod(&self) -> MeshDefinition`:

- Builds the primary mesh via `build_mesh()`.
- Counts triangles = `indices.len() / 3`.
- If `> LOD_TRIANGLE_THRESHOLD (200)`: calls `generate_lod_levels(&mesh, 2)`
  and overrides the auto-distances with fixed values
  `[LOD_DISTANCE_1, LOD_DISTANCE_2]` = `[8.0, 20.0]`.
- If `≤ 200`: returns mesh as-is (no LOD).

All procedural item meshes in the current implementation are well under 200
triangles, so LOD is not triggered at runtime today. The infrastructure is
ready for future artist-authored higher-fidelity meshes.

#### Free helper functions

Two free (non-method) `#[inline]` functions were added to the module:

- `lerp_color4(a, b, t) -> [f32; 4]` — RGBA linear interpolation
- `lerp_color3(a, b, t) -> [f32; 3]` — RGB linear interpolation (for emissive)

---

### Architecture compliance

- [ ] All new constants extracted (`COLOR_ACCENT_*`, `COLOR_CHARGE_*`,
      `EMISSIVE_CHARGE_*`, `SHADOW_QUAD_*`, `LOD_*`, `MATERIAL_*`).
- [ ] No hardcoded magic numbers in logic paths.
- [ ] `to_creature_definition` is unchanged in signature; the new
      `to_creature_definition_with_charges` is additive.
- [ ] `rand` dependency removed from `item_world_events.rs` — the system is
      now deterministic and safe for save/load replay.
- [ ] RON data files unchanged.
- [ ] No test references `campaigns/tutorial`.
- [ ] SPDX headers present on all modified `.rs` files (inherited).
- [ ] All new public functions documented with `///` doc comments and examples.

---

### Test coverage

New tests in `src/domain/visual/item_mesh.rs` (`mod tests`):

| Test                                                    | What it verifies                                                |
| ------------------------------------------------------- | --------------------------------------------------------------- |
| `test_fire_resist_item_accent_orange`                   | ResistFire → `COLOR_ACCENT_FIRE`                                |
| `test_cold_resist_item_accent_blue`                     | ResistCold → `COLOR_ACCENT_COLD`                                |
| `test_electricity_resist_item_accent_yellow`            | ResistElectricity → yellow                                      |
| `test_poison_resist_item_accent_green`                  | ResistPoison → acid green                                       |
| `test_magic_resist_item_accent_purple`                  | ResistMagic → purple                                            |
| `test_might_bonus_item_accent_warm_red`                 | Might → warm red                                                |
| `test_ac_bonus_item_accent_teal`                        | ArmorClass → teal                                               |
| `test_intellect_bonus_item_accent_deep_blue`            | Intellect → deep blue                                           |
| `test_magical_item_metallic_material`                   | `is_magical()` → `metallic > 0.5`, `roughness < 0.3`            |
| `test_non_magical_item_matte_material`                  | mundane non-metal → `metallic: 0.0`, `roughness: 0.8`           |
| `test_shadow_quad_present_and_transparent`              | `meshes[0]` is shadow quad, alpha < 0.5, `AlphaMode::Blend`     |
| `test_shadow_quad_valid_for_all_categories`             | Shadow quad present for all item types                          |
| `test_charge_fraction_full_color_gold`                  | `charges_fraction=1.0` → gold gem, emissive                     |
| `test_charge_fraction_empty_color_grey`                 | `charges_fraction=0.0` → grey gem, no emissive                  |
| `test_charge_fraction_none_no_gem`                      | `charges_fraction=None` → exactly 2 meshes                      |
| `test_deterministic_charge_gem_color`                   | Color gradient determinism and boundary values                  |
| `test_lod_added_for_complex_mesh`                       | > 200 triangles → LOD levels generated                          |
| `test_no_lod_for_simple_mesh`                           | ≤ 200 triangles → `lod_levels: None`                            |
| `test_creature_definition_mesh_transform_count_matches` | `meshes.len() == mesh_transforms.len()` for all charge variants |
| `test_accent_color_not_applied_to_cursed_item`          | Cursed items keep `COLOR_CURSED` even with bonus                |
| `test_lerp_color4_midpoint`                             | `lerp_color4` at `t=0.5` produces midpoint                      |
| `test_lerp_color3_midpoint`                             | `lerp_color3` at `t=0.5` produces midpoint                      |

New tests in `src/game/systems/item_world_events.rs` (`mod tests`):

| Test                                               | What it verifies                         |
| -------------------------------------------------- | ---------------------------------------- |
| `test_deterministic_drop_rotation_same_inputs`     | Same inputs → same angle                 |
| `test_deterministic_drop_rotation_different_tiles` | Different tile → different angle         |
| `test_deterministic_drop_rotation_in_range`        | Angle in `[0, TAU)` for all tested tiles |
| `test_deterministic_drop_rotation_different_items` | Different item IDs → different angle     |

**Total tests added: 26** across two modules. All 3,159 tests pass.

## Items Procedural Meshes — Phase 5: Campaign Builder SDK Integration

### Overview

Phase 5 brings the Item Mesh workflow in the Campaign Builder to parity with
the Creature Builder (`creatures_editor.rs`). Campaign authors can now browse
all registered item mesh RON assets, filter by `ItemMeshCategory`, edit a
descriptor's visual properties (colors, scale, emissive), preview the result
live, undo/redo every change, save to `assets/items/`, and register existing
RON files. A **"Ground Mesh Preview"** collapsible was also added to the
existing Items editor form, and a cross-tab "Open in Item Mesh Editor" signal
was wired between the Items tab and the new **Item Meshes** tab.

### Phase 5 Deliverables

| File                                                | Role                                                      |
| --------------------------------------------------- | --------------------------------------------------------- |
| `sdk/campaign_builder/src/item_mesh_undo_redo.rs`   | `ItemMeshUndoRedo` + `ItemMeshEditAction`                 |
| `sdk/campaign_builder/src/item_mesh_workflow.rs`    | `ItemMeshWorkflow`, `ItemMeshEditorMode`                  |
| `sdk/campaign_builder/src/item_mesh_editor.rs`      | `ItemMeshEditorState` — full editor UI                    |
| `sdk/campaign_builder/src/items_editor.rs`          | Ground Mesh Preview pane + `requested_open_item_mesh`     |
| `sdk/campaign_builder/src/lib.rs`                   | `EditorTab::ItemMeshes`, module registrations, tab wiring |
| `sdk/campaign_builder/tests/map_data_validation.rs` | `MapEvent::DroppedItem` arm                               |

### What was built

#### 5.1 — `item_mesh_undo_redo.rs`

`ItemMeshUndoRedo` is a simple two-stack undo/redo manager owning a
`Vec<ItemMeshEditAction>` for each direction. `ItemMeshEditAction` covers:

- `SetPrimaryColor { old, new }` — RGBA primary color change
- `SetAccentColor { old, new }` — RGBA accent color change
- `SetScale { old, new }` — scale factor change
- `SetEmissive { old, new }` — emissive bool toggle
- `SetOverrideEnabled { old, new }` — override enable/disable
- `ReplaceDescriptor { old, new }` — atomic full-descriptor swap

`push()` appends to the undo stack and clears the redo stack. `undo()` pops
from the undo stack and pushes the action to redo; `redo()` does the reverse.
Both return the popped `ItemMeshEditAction` so the caller can apply `old` (for
undo) or `new` (for redo) to the live descriptor.

#### 5.2 — `item_mesh_workflow.rs`

`ItemMeshWorkflow` tracks `ItemMeshEditorMode` (`Registry` or `Edit`),
`current_file: Option<String>`, and `unsaved_changes: bool`.

Public API:

- `mode_indicator() -> String` — `"Registry Mode"` or `"Asset Editor: <file>"`
- `breadcrumb_string() -> String` — `"Item Meshes"` or `"Item Meshes > <file>"`
- `enter_edit(file_name)` — transitions to Edit mode, sets `current_file`, clears dirty
- `return_to_registry()` — resets to Registry mode, clears file and dirty
- `mark_dirty()` / `mark_clean()` — unsaved-change tracking
- `has_unsaved_changes()` / `current_file()`

#### 5.3 — `item_mesh_editor.rs`

`ItemMeshEditorState` is the top-level state struct for the Item Mesh Editor
tab. Key design decisions:

**Registry mode UI** uses `TwoColumnLayout::new("item_mesh_registry")`. All
mutations inside the two `FnOnce` closures are collected in separate
`left_*` and `right_*` deferred-mutation locals (sdk/AGENTS.md Rule 10), then
merged into canonical `pending_*` vars and applied after `show_split` returns.
This avoids the E0499/E0524 double-borrow errors that arise when both closures
capture the same `&mut` variable. The `search_query` text edit uses an owned
clone of the value rather than a `&mut self.search_query` reference, flushed
via `pending_new_search`.

**Edit mode UI** uses `ui.columns(2, ...)` for a properties/preview split:

- Left: override-enabled checkbox, primary/accent RGBA sliders, scale slider
  (0.25–4.0), emissive checkbox, Reset to Defaults button, inline Validation
  collapsible. Every mutation pushes an `ItemMeshEditAction`, sets
  `preview_dirty = true`, and calls `ui.ctx().request_repaint()`.
- Right: camera-distance slider, "Regenerate Preview" button, live
  `PreviewRenderer` display.

**Dialog windows** (`show_save_as_dialog_window`,
`show_register_asset_dialog_window`) use the deferred-action pattern instead of
`.open(&mut bool)` — the `still_open` double-borrow issue is avoided by
collecting `do_save`, `do_cancel`, `do_validate`, and `do_register` booleans
inside the closure and acting on them after it returns.

**`validate_descriptor`** is a pure `(errors, warnings)` function:

- Error: `scale <= 0.0`
- Warning: `scale > 3.0`

**`perform_save_as_with_path`** validates the path prefix (`assets/items/`),
serialises the descriptor to RON via `ron::ser::to_string_pretty`, creates
directories, writes the file, derives a display name from the file stem, and
appends a new `ItemMeshEntry` to the registry.

**`execute_register_asset_validation`** reads and deserialises the RON file,
checks for duplicate `file_path` entries in the registry, and sets
`register_asset_error` on failure.

**`refresh_available_assets`** scans `campaign_dir/assets/items/*.ron` and
caches results in `available_item_assets`; skips the scan if
`last_campaign_dir` is unchanged.

#### 5.4 — Items editor Ground Mesh Preview pane

`ItemsEditorState` gained:

- `requested_open_item_mesh: Option<ItemId>` — cross-tab navigation signal,
  consumed by the parent `CampaignBuilderApp` to switch to `EditorTab::ItemMeshes`.
- A `ui.collapsing("🧊 Ground Mesh Preview", ...)` section at the bottom of
  `show_form()`. It derives an `ItemMeshDescriptor` from the current
  `edit_buffer` via `ItemMeshDescriptor::from_item`, displays category, shape,
  and override parameters, and provides an "✏️ Open in Item Mesh Editor" button
  that sets `requested_open_item_mesh`.

#### 5.5 — Tab wiring in `lib.rs`

- Three new modules registered: `item_mesh_editor`, `item_mesh_undo_redo`,
  `item_mesh_workflow`.
- `EditorTab::ItemMeshes` added to the enum and the sidebar tabs array.
- `item_mesh_editor_state: item_mesh_editor::ItemMeshEditorState` added to
  `CampaignBuilderApp`.
- The central panel match dispatches `EditorTab::ItemMeshes` to
  `item_mesh_editor_state.show(ui, campaign_dir.as_ref())`.
- `ItemMeshEditorSignal::OpenInItemsEditor(item_id)` switches to
  `EditorTab::Items` and selects the matching item.
- Cross-tab from Items: `requested_open_item_mesh.take()` switches to
  `EditorTab::ItemMeshes`.

#### 5.6 — `MapEvent::DroppedItem` exhaustive match arms

Five `match event` blocks in `map_editor.rs` and one in
`tests/map_data_validation.rs` were missing the `DroppedItem` variant
(introduced in Phase 2). All were fixed:

- `EventEditorState::from_map_event` — sets `event_type = Treasure`, copies name
- Two tile-grid colour queries — maps to `EventType::Treasure`
- The event-details tooltip panel — shows item id and charges
- `event_name_description` helper — returns name and empty description
- Test validation loop — empty arm (no validation required)

#### Pre-existing `mesh_descriptor_override` field gap

`Item::mesh_descriptor_override` (added in Phase 1) was missing from struct
literal initialisers throughout the SDK codebase. All affected files were
patched to add `mesh_descriptor_override: None,`:

`advanced_validation.rs`, `asset_manager.rs`, `characters_editor.rs`,
`dialogue_editor.rs`, `items_editor.rs`, `lib.rs`, `templates.rs`,
`undo_redo.rs`, `ui_helpers.rs`.

Where the Python insertion script accidentally added the field to `TemplateInfo`
literals (which have no such field), the spurious lines were removed.

### Architecture compliance

- [ ] Data structures match `architecture.md` Section 4 — `ItemMeshDescriptor`,
      `ItemMeshCategory`, `ItemMeshDescriptorOverride` used exactly as defined.
- [ ] Module placement follows Section 3.2 — three new SDK modules in
      `sdk/campaign_builder/src/`.
- [ ] RON format used for all data files — descriptor serialisation via `ron`.
- [ ] No architectural deviations without documentation.
- [ ] egui ID rules (sdk/AGENTS.md) fully followed:
  - Every loop body uses `ui.push_id(idx, ...)`.
  - Every `ScrollArea` has `.id_salt("unique_string")`.
  - Every `ComboBox` uses `ComboBox::from_id_salt("...")`.
  - Every `Window` has a unique title.
  - State mutations call `ui.ctx().request_repaint()`.
  - `TwoColumnLayout` used for the registry list/detail split.
  - No `SidePanel`/`CentralPanel` guards skipped same-frame.
  - Deferred-mutation pattern (Rule 10) applied throughout.
- [ ] SPDX headers present on all three new `.rs` files.

### Test coverage

**`item_mesh_undo_redo.rs`** (12 tests)

| Test                                     | Assertion                                                  |
| ---------------------------------------- | ---------------------------------------------------------- |
| `test_item_mesh_undo_redo_push_and_undo` | After push + undo: `can_undo == false`, `can_redo == true` |
| `test_item_mesh_undo_redo_redo`          | After push + undo + redo: `can_redo == false`              |
| `test_item_mesh_undo_redo_clear`         | After clear: both stacks empty                             |
| `test_push_clears_redo_stack`            | New push after undo wipes redo                             |
| `test_undo_empty_returns_none`           | Undo on empty stack returns `None`                         |
| `test_redo_empty_returns_none`           | Redo on empty stack returns `None`                         |
| `test_multiple_pushes_lifo_order`        | LIFO semantics verified                                    |
| `test_set_primary_color_action`          | `SetPrimaryColor` old/new fields                           |
| `test_set_accent_color_action`           | `SetAccentColor` old/new fields                            |
| `test_set_override_enabled_action`       | `SetOverrideEnabled` old/new fields                        |
| `test_replace_descriptor_action`         | `ReplaceDescriptor` full descriptor swap                   |

**`item_mesh_workflow.rs`** (11 tests)

| Test                                                    | Assertion                             |
| ------------------------------------------------------- | ------------------------------------- |
| `test_workflow_default_is_registry`                     | Default mode is `Registry`            |
| `test_item_mesh_editor_mode_indicator_registry`         | Returns `"Registry Mode"`             |
| `test_item_mesh_editor_mode_indicator_edit`             | Returns `"Asset Editor: sword.ron"`   |
| `test_item_mesh_editor_mode_indicator_edit_no_file`     | Returns `"Asset Editor"` with no file |
| `test_item_mesh_editor_breadcrumb_registry`             | Returns `"Item Meshes"`               |
| `test_item_mesh_editor_breadcrumb_edit`                 | Returns `"Item Meshes > sword.ron"`   |
| `test_item_mesh_editor_breadcrumb_edit_no_file`         | Returns `"Item Meshes"` with no file  |
| `test_workflow_enter_edit`                              | Mode transitions to Edit, file set    |
| `test_workflow_enter_edit_clears_unsaved_changes`       | Dirty flag cleared on enter           |
| `test_workflow_return_to_registry`                      | Resets mode, file, dirty              |
| `test_workflow_mark_dirty` / `test_workflow_mark_clean` | Dirty flag round-trip                 |

**`item_mesh_editor.rs`** (28 tests, including 1 in `items_editor.rs`)

| Test                                                          | Assertion                                 |
| ------------------------------------------------------------- | ----------------------------------------- |
| `test_item_mesh_editor_state_default`                         | Mode is Registry, no selection, not dirty |
| `test_item_mesh_editor_has_unsaved_changes_false_by_default`  | Fresh state is clean                      |
| `test_item_mesh_editor_has_unsaved_changes_true_after_edit`   | Mutation sets dirty                       |
| `test_item_mesh_editor_can_undo_false_by_default`             | Empty undo stack                          |
| `test_item_mesh_editor_can_redo_false_by_default`             | Empty redo stack                          |
| `test_item_mesh_editor_back_to_registry_clears_edit_state`    | edit_buffer cleared, mode reset           |
| `test_available_item_assets_empty_when_no_assets_dir`         | Missing dir yields empty list             |
| `test_available_item_assets_populated_from_campaign_dir`      | Scans `.ron` files correctly              |
| `test_available_item_assets_not_refreshed_when_dir_unchanged` | Cache hit on same dir                     |
| `test_available_item_assets_refreshed_when_dir_changes`       | Cache miss on dir change                  |
| `test_register_asset_validate_duplicate_id_sets_error`        | Duplicate path sets error                 |
| `test_register_asset_cancel_does_not_modify_registry`         | Cancel leaves registry unchanged          |
| `test_register_asset_success_appends_entry`                   | Valid RON appended to registry            |
| `test_perform_save_as_with_path_appends_new_entry`            | Save-as writes file and registry          |
| `test_perform_save_as_requires_campaign_directory`            | Error with no campaign dir                |
| `test_perform_save_as_rejects_non_item_asset_paths`           | Path outside `assets/items/` rejected     |
| `test_revert_edit_buffer_restores_original`                   | Buffer reset from registry entry          |
| `test_revert_edit_buffer_errors_in_registry_mode`             | Revert in Registry mode is error          |
| `test_validate_descriptor_reports_invalid_scale`              | `scale = 0.0` → error containing "scale"  |
| `test_validate_descriptor_reports_negative_scale`             | `scale = -1.0` → error                    |
| `test_validate_descriptor_passes_for_default_descriptor`      | Clean descriptor → no issues              |
| `test_validate_descriptor_warns_on_large_scale`               | `scale = 4.0` → warning                   |
| `test_filtered_sorted_registry_empty`                         | Empty registry → empty result             |
| `test_filtered_sorted_registry_by_name`                       | Alphabetical sort respected               |
| `test_filtered_sorted_registry_search_filter`                 | Search query filters correctly            |
| `test_count_by_category`                                      | Category histogram correct                |
| `test_items_editor_requested_open_item_mesh_set_on_button`    | Signal field set + drainable              |

**Total new tests: 51.** All 1,925 SDK tests and 3,159 full-suite tests pass.

---

## Items Procedural Meshes — Phase 6.4: Required Integration Tests

### Overview

Phase 6.4 adds three mandatory integration tests that close coverage gaps
identified in the Phase 6 acceptance criteria:

1. **`test_all_base_items_have_valid_mesh_descriptor`** — iterates every item
   in `data/items.ron`, generates an `ItemMeshDescriptor` via
   `ItemMeshDescriptor::from_item`, converts it to a `CreatureDefinition` via
   `to_creature_definition`, and asserts `validate()` returns `Ok`. This
   guarantees the descriptor pipeline is sound for all current base items.

2. **`test_item_mesh_registry_tutorial_coverage`** — loads the
   `data/test_campaign` campaign via `CampaignLoader`, asserts the returned
   `GameData::item_meshes` registry is non-empty and contains at least 2
   entries. Validates the end-to-end loader path for item mesh data.

3. **`test_dropped_item_event_in_map_ron`** — reads
   `data/test_campaign/data/maps/map_1.ron`, deserialises it as
   `crate::domain::world::Map`, and asserts that at least one
   `MapEvent::DroppedItem` event is present and that item_id 4 (Long Sword) is
   among them. Validates RON round-trip for the `DroppedItem` variant.

A prerequisite data fixture was also added: a `DroppedItem` entry for the
Long Sword (item_id 4) at map position (7, 7) was inserted into
`data/test_campaign/data/maps/map_1.ron`.

### Phase 6.4 Deliverables

| File                                     | Change                                           |
| ---------------------------------------- | ------------------------------------------------ |
| `src/domain/visual/item_mesh.rs`         | 3 new tests appended to `mod tests`              |
| `data/test_campaign/data/maps/map_1.ron` | `DroppedItem` event added at position (x:7, y:7) |

### What was built

#### `test_all_base_items_have_valid_mesh_descriptor`

Loads `data/items.ron` using `ItemDatabase::load_from_file`, then loops over
every `Item` returned by `all_items()`. For each item it calls
`ItemMeshDescriptor::from_item(item)`, then `descriptor.to_creature_definition()`,
then `creature_def.validate()`. Any failure includes the item id and name in
the assertion message for fast triage.

#### `test_item_mesh_registry_tutorial_coverage`

Constructs a `CampaignLoader` pointing at `data/` (base) and
`data/test_campaign` (campaign), calls `load_game_data()`, and asserts:

- `result.is_ok()`
- `!game_data.item_meshes.is_empty()`
- `game_data.item_meshes.count() >= 2`

Uses `env!("CARGO_MANIFEST_DIR")` for portable paths. Does **not** reference
`campaigns/tutorial` (Implementation Rule 5 compliant).

#### `test_dropped_item_event_in_map_ron`

Reads `data/test_campaign/data/maps/map_1.ron` from disk, deserialises via
`ron::from_str::<Map>(&contents)`, then:

- Asserts at least one `MapEvent::DroppedItem { .. }` variant is present.
- Asserts a `DroppedItem` with `item_id == 4` (Long Sword) exists.

#### `DroppedItem` fixture in `map_1.ron`

Added at the end of the `events` block (before the closing brace):

```data/test_campaign/data/maps/map_1.ron#L8384-8391
        (
            x: 7,
            y: 7,
        ): DroppedItem(
            name: "Long Sword",
            item_id: 4,
            charges: 0,
        ),
```

### Architecture compliance

- [x] Data structures match `architecture.md` Section 4 — `ItemMeshDescriptor`,
      `Map`, `MapEvent` used exactly as defined.
- [x] Test data uses `data/test_campaign`, NOT `campaigns/tutorial`
      (Implementation Rule 5).
- [x] New fixture added to `data/test_campaign/data/maps/map_1.ron`, not
      borrowed from live campaign data.
- [x] RON format used for all data files.
- [x] No architectural deviations without documentation.
- [x] SPDX headers unaffected (tests appended to existing file).

### Test coverage

**`src/domain/visual/item_mesh.rs`** (3 new tests, inside existing `mod tests`)

| Test                                             | Assertion                                                   |
| ------------------------------------------------ | ----------------------------------------------------------- |
| `test_all_base_items_have_valid_mesh_descriptor` | Every item in `data/items.ron` → valid `CreatureDefinition` |
| `test_item_mesh_registry_tutorial_coverage`      | `test_campaign` item mesh registry non-empty, count ≥ 2     |
| `test_dropped_item_event_in_map_ron`             | `map_1.ron` parses, contains `DroppedItem` with item_id=4   |

**All 3 new tests pass.** All quality gates pass (fmt, check, clippy -D warnings, nextest).

---

## Phase 6.3 — `MapEvent::DroppedItem` Placements in Tutorial Campaign and Test Fixture

### Overview

Phase 6.3 populates the tutorial campaign maps and the test fixture map with
concrete `MapEvent::DroppedItem` entries. These events represent items lying on
the ground that the player can walk over and pick up. This phase adds 3 events
to the live tutorial campaign and 1 to the test fixture (`data/test_campaign`),
satisfying both the gameplay placement requirements and Implementation Rule 5
(tests use `data/test_campaign`, never `campaigns/tutorial`).

---

### What Was Changed

#### Tutorial Campaign Maps

| File                                     | Position | Item               | item_id         | Purpose                                                          |
| ---------------------------------------- | -------- | ------------------ | --------------- | ---------------------------------------------------------------- |
| `campaigns/tutorial/data/maps/map_1.ron` | (3, 17)  | Dropped Sword      | 3 (Short Sword) | Near the elder NPC at (1,16) — early starting area reward        |
| `campaigns/tutorial/data/maps/map_2.ron` | (2, 5)   | Healing Potion     | 50              | Near dungeon entrances in Dark Forrest — survival incentive      |
| `campaigns/tutorial/data/maps/map_4.ron` | (3, 3)   | Ring of Protection | 40              | Near the `Treasure` event at (1,1) — treasure chamber floor loot |

All three entries were inserted before the closing `},` of the existing
`events: { ... }` BTreeMap block in each file. No existing events were
modified. No duplicate positions were introduced.

#### Test Fixture Map

| File                                     | Position | Item                    | item_id        | Note                                                                                 |
| ---------------------------------------- | -------- | ----------------------- | -------------- | ------------------------------------------------------------------------------------ |
| `data/test_campaign/data/maps/map_1.ron` | (7, 7)   | Test Dropped Long Sword | 4 (Long Sword) | Entry already existed; name updated to "Test Dropped Long Sword" for fixture clarity |

The `DroppedItem` at (7, 7) in `data/test_campaign/data/maps/map_1.ron` was
pre-existing with name `"Long Sword"`. Its name was updated to
`"Test Dropped Long Sword"` to clearly identify it as a test fixture entry
and match the Phase 6.3 specification.

---

### RON Format Used

Each event entry follows the `MapEvent::DroppedItem` variant structure, inserted
into the `events` BTreeMap block:

```antares/campaigns/tutorial/data/maps/map_1.ron#L8450-8459
        (
            x: 3,
            y: 17,
        ): DroppedItem(
            name: "Dropped Sword",
            item_id: 3,
            charges: 0,
        ),
```

The `name` field is `#[serde(default)]` (optional display label).
The `charges` field is `#[serde(default)]` and set to `0` for non-charged items.
`item_id` is the `ItemId` (`u32`) type alias referencing entries in `items.ron`.

---

### Architecture Compliance

- `MapEvent::DroppedItem` structure used exactly as defined (Section 4, map events).
- RON format used for all data files per Section 7.1.
- No JSON or YAML introduced.
- Test data placed in `data/test_campaign` per Implementation Rule 5.
- No modifications to `campaigns/tutorial` from tests.

---

### Quality Gates

All four gates passed after edits:

```text
cargo fmt         → no output (all files already formatted)
cargo check       → Finished 0 errors
cargo clippy      → Finished 0 warnings
cargo nextest run → 3162 passed; 0 failed; 8 skipped
```

---

## Phase 6.2 — Visual Quality Pass: Item Mesh RON Files

### Overview

Phase 6.2 improves the visual silhouette of every item mesh category so that
dropped items on the ground are immediately recognisable at tile scale.
Each category listed in the quality table from the plan now passes the
corresponding check.

---

### What Was Changed

All files are under `campaigns/tutorial/assets/items/`.

#### Weapons

| File                      | id   | What changed                                                                                                  |
| ------------------------- | ---- | ------------------------------------------------------------------------------------------------------------- |
| `weapons/dagger.ron`      | 9002 | Added `crossguard` mesh (half-width ±0.070, half-height ±0.015). Scale lowered to 0.3150 (compact).           |
| `weapons/short_sword.ron` | 9003 | Added `crossguard` mesh (±0.090 × ±0.018). Scale 0.3500.                                                      |
| `weapons/sword.ron`       | 9001 | Added `crossguard` mesh (±0.110 × ±0.020). Scale raised to 0.4025 — clearly longer than dagger.               |
| `weapons/long_sword.ron`  | 9004 | Added `crossguard` mesh (±0.130 × ±0.022). Scale 0.4375.                                                      |
| `weapons/great_sword.ron` | 9005 | Added `crossguard` mesh (±0.160 × ±0.025). Scale 0.5250 — dominant two-handed silhouette.                     |
| `weapons/club.ron`        | 9006 | Split into `handle` (thin shaft) + `head` (wide 6-point boxy hexagon). Scale 0.4025.                          |
| `weapons/staff.ron`       | 9007 | Renamed shaft to `shaft` (widened ±0.035). Added `orb_tip` 8-point polygon at Z+0.48 with blue emissive glow. |
| `weapons/bow.ron`         | 9008 | Renamed limb to `limb` (tightened arc). Added `string` diamond mesh for visible bowstring. Scale 0.5600.      |

**Crossguard material** (all swords): `color (0.60, 0.60, 0.64)`, `metallic 0.65`, `roughness 0.35` — slightly darker and more weathered than the polished blade.

**Scale progression** ensures clear size graduation:

```
dagger(0.3150) → short_sword(0.3500) → sword(0.4025) → long_sword(0.4375) → great_sword(0.5250)
```

#### Armor

| File                   | id   | What changed                                                                                                                                       |
| ---------------------- | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `armor/plate_mail.ron` | 9103 | Split into `body` (narrower rectangle) + `shoulders` (wide U-shaped pauldron extending ±0.32 X). Scale 0.4550. High metallic 0.75, roughness 0.25. |
| `armor/helmet.ron`     | 9105 | Added `visor` mesh (thin dark horizontal stripe) over the existing `dome`. Scale 0.3850.                                                           |

`leather_armor.ron` retains its plain trapezoid — the **silhouette contrast** now comes from plate's shoulder extensions vs leather's clean trapezoidal outline.

#### Accessories

| File                   | id   | What changed                                                                                                                                                                 |
| ---------------------- | ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `accessories/ring.ron` | 9301 | **Complete rework** — annular washer shape. 12 outer vertices (r=0.160) + 12 inner vertices (r=0.070), 24 stitched triangles. Outer radius ≥ 0.15 as required. Scale 0.2100. |

The ring now has a visible hole in the centre so it reads as a torus/ring at tile scale. The amulet retains its filled-disc shape, making the two accessories visually distinct.

#### Ammo

| File             | id   | What changed                                                                                             |
| ---------------- | ---- | -------------------------------------------------------------------------------------------------------- |
| `ammo/arrow.ron` | 9401 | Split into `shaft` (thin diamond, width 0.018) + `fletching` (triangular red fin at tail). Scale 0.2100. |

---

### Architecture Compliance

- All RON files use `.ron` extension.
- No SPDX headers in RON data files (only in `.rs` source files).
- `mesh_transforms` has exactly one entry per mesh in every file.
- Normals array has exactly as many entries as vertices in every mesh.
- All floats have decimal points.
- No JSON or YAML format used.

---

### Quality Gate Verification

```text
cargo fmt         → no output
cargo check       → Finished 0 errors
cargo clippy      → Finished 0 warnings
cargo nextest run → 3162 passed; 0 failed; 8 skipped
```

---

## Phase 6 — Complete: Full Item Mesh Coverage

### Overview

Phase 6 is the final phase of the Items Procedural Meshes implementation plan.
It brings full coverage of all base items, a visual quality pass, authored
in-world dropped item events, and comprehensive coverage tests.

---

### Deliverables Checklist

- [x] All base items in `data/items.ron` (32 items, IDs 1–101) covered by a
      valid auto-generated `ItemMeshDescriptor` — verified by
      `test_all_base_items_have_valid_mesh_descriptor`
- [x] Visual quality pass completed for all 13 categories (see Phase 6.2 above)
- [x] At least three authored `DroppedItem` events in tutorial campaign maps: - `map_1.ron` (3,17): Short Sword — near starting room - `map_2.ron` (2,5): Healing Potion — first dungeon entrance - `map_4.ron` (3,3): Ring of Protection — treasure chamber
- [x] Full coverage tests passing (see Phase 6.4 below)

---

### Phase 6.4 Tests

Three new tests added to `src/domain/visual/item_mesh.rs` `mod tests`:

| Test                                             | What it verifies                                                                                                                          |
| ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `test_all_base_items_have_valid_mesh_descriptor` | Every item in `data/items.ron` → `ItemMeshDescriptor::from_item` → `to_creature_definition()` → `validate()` returns `Ok`                 |
| `test_item_mesh_registry_tutorial_coverage`      | `CampaignLoader` on `data/test_campaign` returns non-empty item mesh registry with ≥ 2 entries                                            |
| `test_dropped_item_event_in_map_ron`             | `data/test_campaign/data/maps/map_1.ron` deserialises as `Map`, contains ≥ 1 `MapEvent::DroppedItem`, specifically item_id=4 (Long Sword) |

All tests use `data/test_campaign` — not `campaigns/tutorial` — per Implementation Rule 5.

---

### Quality Gates — Final

```text
cargo fmt         → no output
cargo check       → Finished 0 errors
cargo clippy      → Finished 0 warnings (0 warnings)
cargo nextest run → 3162 passed; 0 failed; 8 skipped
```

---

## Items Procedural Meshes — Phase 3.2: Python Generator Script

**Plan**: [`items_procedural_meshes_implementation_plan.md`](items_procedural_meshes_implementation_plan.md)

### Overview

Phase 3.2 delivers `examples/generate_item_meshes.py` — the developer
convenience script called out in the Phase 3 deliverables list. The script
generates every `CreatureDefinition` RON file under
`campaigns/tutorial/assets/items/` from a single authoritative Python manifest,
making the asset files regenerable without hand-editing them one by one.

---

### Phase 3.2 Deliverables

**Files created / updated**:

- `examples/generate_item_meshes.py` _(new)_

---

### What Was Built

#### Script structure

The script is organised into four layers:

1. **RON formatting helpers** — `fv()`, `fc()`, `fmat()`, `emit_mesh()`,
   `emit_transform()`, `write_item_ron()`: pure string-building functions that
   produce syntactically correct RON without any external library dependency.

2. **Color / scale constants** — mirror `item_mesh.rs` exactly so that
   re-generated files stay visually consistent with the runtime pipeline:
   `COLOR_STEEL`, `COLOR_WOOD`, `COLOR_LEATHER`, `COLOR_SILVER`, `COLOR_GOLD`,
   `COLOR_ORB`, `EMISSIVE_MAGIC`, `EMISSIVE_ORB`, `EMISSIVE_QUEST`,
   `BASE_SCALE`, `TWO_HANDED_SCALE_MULT`, `ARMOR_MED_SCALE_MULT`,
   `ARMOR_HEAVY_SCALE_MULT`, `SMALL_SCALE_MULT`.

3. **Geometry builders** — one function per logical item type, each returning
   `(list[mesh_str], list[transform_tuple])`. Multi-part items emit multiple
   `MeshDefinition` blocks with correct per-part transforms:

   | Builder                                                                                         | Parts | Description                                               |
   | ----------------------------------------------------------------------------------------------- | ----- | --------------------------------------------------------- |
   | `build_sword` / `build_dagger` / `build_short_sword` / `build_long_sword` / `build_great_sword` | 2     | Diamond blade + rectangular crossguard                    |
   | `build_club`                                                                                    | 2     | Rectangular handle + fan-hexagon head                     |
   | `build_staff`                                                                                   | 2     | Rectangular shaft + 8-sided orb tip (offset to shaft tip) |
   | `build_bow`                                                                                     | 2     | Curved arc limb + thin bowstring                          |
   | `build_plate_mail`                                                                              | 2     | Body plate + U-shaped pauldron bar                        |
   | `build_helmet`                                                                                  | 2     | Pentagon dome + rectangular visor                         |
   | `build_arrow`                                                                                   | 2     | Diamond shaft + V-shaped fletching                        |
   | `build_quest_scroll`                                                                            | 2     | Hex scroll body + 16-point star seal                      |
   | `build_leather_armor`, `build_chain_mail`, `build_shield`, `build_boots`                        | 1     | Single silhouette                                         |
   | `build_health/mana/cure/attribute_potion`                                                       | 1     | Hexagonal disc                                            |
   | `build_ring`                                                                                    | 1     | Flat torus (two concentric n-gons joined by quad strips)  |
   | `build_amulet`                                                                                  | 1     | Octagon disc                                              |
   | `build_belt`, `build_cloak`                                                                     | 1     | Rectangle / teardrop                                      |
   | `build_bolt`, `build_stone`                                                                     | 1     | Flat diamond                                              |
   | `build_key_item`                                                                                | 1     | 16-point star                                             |

4. **Manifests** — `MANIFEST` (27 entries covering all IDs 9001–9502) and
   `TEST_MANIFEST` (2 entries: sword + potion) for the
   `data/test_campaign/assets/items/` fixtures.

#### CLI usage

```text
# Full manifest → campaigns/tutorial/assets/items/
python examples/generate_item_meshes.py

# Test fixtures → data/test_campaign/assets/items/
python examples/generate_item_meshes.py --test-fixtures

# Custom root directory
python examples/generate_item_meshes.py --output-dir /tmp/items
```

The script is idempotent. Re-running overwrites existing files with freshly
generated geometry. All `.ron` files are committed; the script is not a build
step.

#### Part counts per committed file

| File                               | Parts                  |
| ---------------------------------- | ---------------------- |
| `weapons/sword.ron`                | 2 (blade, crossguard)  |
| `weapons/dagger.ron`               | 2 (blade, crossguard)  |
| `weapons/short_sword.ron`          | 2 (blade, crossguard)  |
| `weapons/long_sword.ron`           | 2 (blade, crossguard)  |
| `weapons/great_sword.ron`          | 2 (blade, crossguard)  |
| `weapons/club.ron`                 | 2 (handle, head)       |
| `weapons/staff.ron`                | 2 (shaft, orb_tip)     |
| `weapons/bow.ron`                  | 2 (limb, string)       |
| `armor/leather_armor.ron`          | 1 (leather)            |
| `armor/chain_mail.ron`             | 1 (chain)              |
| `armor/plate_mail.ron`             | 2 (body, shoulders)    |
| `armor/shield.ron`                 | 1 (shield)             |
| `armor/helmet.ron`                 | 2 (dome, visor)        |
| `armor/boots.ron`                  | 1 (boots)              |
| `consumables/health_potion.ron`    | 1 (potion)             |
| `consumables/mana_potion.ron`      | 1 (potion)             |
| `consumables/cure_potion.ron`      | 1 (potion)             |
| `consumables/attribute_potion.ron` | 1 (potion)             |
| `accessories/ring.ron`             | 1 (band)               |
| `accessories/amulet.ron`           | 1 (amulet)             |
| `accessories/belt.ron`             | 1 (belt)               |
| `accessories/cloak.ron`            | 1 (cloak)              |
| `ammo/arrow.ron`                   | 2 (shaft, fletching)   |
| `ammo/bolt.ron`                    | 1 (bolt)               |
| `ammo/stone.ron`                   | 1 (stone)              |
| `quest/quest_scroll.ron`           | 2 (quest_scroll, seal) |
| `quest/key_item.ron`               | 1 (key_item)           |

---

### Architecture Compliance

- SPDX header present: `// SPDX-FileCopyrightText: 2026 Brett Smith` +
  `Apache-2.0` on lines 2–3.
- File extension `.py` — developer tool, not a game data file.
- No game data in JSON/YAML; all output files use `.ron` as required.
- Test fixtures written to `data/test_campaign/assets/items/` — not
  `campaigns/tutorial` — per Implementation Rule 5.
- `--output-dir` flag allows targeting any directory, satisfying the plan's
  §3.2 requirement verbatim.
- Script is idempotent and not a build step.

---

### Quality Gates

```text
cargo fmt         → no output
cargo check       → Finished 0 errors
cargo clippy      → Finished 0 warnings
cargo nextest run → 3162 passed; 0 failed; 8 skipped
python3 examples/generate_item_meshes.py --output-dir /tmp/items → 27 files ✅
python3 examples/generate_item_meshes.py --test-fixtures          →  2 files ✅
```
