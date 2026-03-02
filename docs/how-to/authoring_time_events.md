<!-- SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Authoring Time-Gated Map Events

This guide explains how to write map events that only activate during specific
in-game time windows. Time-gated events let you create night ambushes, day-only
merchants, story beats that unlock after a certain number of in-game days, and
much more.

---

## Prerequisites

- Familiarity with the RON map file format (see `docs/how-to/creating_maps.md`)
- A working campaign with at least one map file under `data/<your_campaign>/maps/`

---

## How Time Conditions Work

Every applicable `MapEvent` variant accepts an optional `time_condition` field.
When the field is absent or set to `None` the event fires unconditionally — just
as it did before the time system was introduced, so all existing map files
continue to work without modification.

When `time_condition` is `Some(...)` the engine evaluates the condition against
the current in-game clock **before** processing the event. If the condition is
not satisfied the event returns no result and is **not consumed** — it remains
in place and is re-evaluated the next time the party steps onto that tile.

### Supported event variants

| Variant                | Supports `time_condition` |
|------------------------|--------------------------|
| `Encounter`            | ✅ yes                   |
| `Sign`                 | ✅ yes                   |
| `NpcDialogue`          | ✅ yes                   |
| `RecruitableCharacter` | ✅ yes                   |
| `Treasure`             | ❌ no (always fires)     |
| `Teleport`             | ❌ no (always fires)     |
| `Trap`                 | ❌ no (always fires)     |
| `EnterInn`             | ❌ no (always fires)     |
| `Furniture`            | ❌ no (always fires)     |
| `Container`            | ❌ no (always fires)     |

---

## Time Condition Variants

### `DuringPeriods`

Fires only when the current time-of-day period is in the supplied list.

Available periods (24-hour boundaries):

| Period      | Hours         |
|-------------|---------------|
| `Dawn`      | 05:00 – 07:59 |
| `Morning`   | 08:00 – 11:59 |
| `Afternoon` | 12:00 – 15:59 |
| `Dusk`      | 16:00 – 18:59 |
| `Evening`   | 19:00 – 21:59 |
| `Night`     | 22:00 – 04:59 |

```ron
// A night ambush — only appears after dark
MapEvent::Encounter(
    name: "Night Ambush",
    description: "Shadows stir in the darkness.",
    monster_group: [3, 3, 4],
    time_condition: Some(DuringPeriods([Night, Evening])),
)
```

```ron
// A market sign only visible during business hours
MapEvent::Sign(
    name: "Market Notice",
    description: "A board covered in posted parchments.",
    text: "Grand Market open until dusk — finest goods in the realm!",
    time_condition: Some(DuringPeriods([Morning, Afternoon])),
)
```

---

### `AfterDay`

Fires only after a specified number of in-game days have elapsed
(`game_time.day > threshold`). Useful for story events that unlock after the
party has been adventuring for a while.

```ron
// A prophesied encounter that appears only after day 5
MapEvent::Encounter(
    name: "The Prophesied Stranger",
    description: "A hooded figure waits by the crossroads.",
    monster_group: [7],
    time_condition: Some(AfterDay(5)),
)
```

```ron
// A sign whose warning only becomes relevant mid-campaign
MapEvent::Sign(
    name: "Ancient Omen",
    description: "A crumbling stone tablet.",
    text: "The seal breaks on the sixth day. Beware.",
    time_condition: Some(AfterDay(5)),
)
```

> **Note**: `AfterDay(5)` fires when `day > 5`, meaning it first fires on day 6.
> It does **not** fire on day 5 itself.

---

### `BeforeDay`

Fires only before a specified number of in-game days have elapsed
(`game_time.day < threshold`). Use this for early-game hints or introductory
encounters that should no longer appear once the party has progressed.

```ron
// A tutorial encounter removed after the first few days
MapEvent::Encounter(
    name: "Wandering Bandit",
    description: "A lone bandit blocks the road.",
    monster_group: [1],
    time_condition: Some(BeforeDay(3)),
)
```

> **Note**: `BeforeDay(3)` fires when `day < 3`, meaning day 1 and day 2 only.
> It does **not** fire on day 3.

---

### `BetweenHours`

Fires only when the current hour falls within a specified inclusive range
(`from <= hour <= to`, 24-hour clock).

```ron
// A day merchant available during shop hours
MapEvent::NpcDialogue(
    name: "Travelling Merchant",
    description: "A cheerful merchant with a loaded cart.",
    npc_id: "travelling_merchant_01",
    time_condition: Some(BetweenHours(from: 8, to: 18)),
)
```

```ron
// A night watchman only on duty late at night
MapEvent::NpcDialogue(
    name: "Night Watchman",
    description: "A guard doing the overnight rounds.",
    npc_id: "watchman_gruff",
    time_condition: Some(BetweenHours(from: 22, to: 4)),
)
```

> **Warning**: `BetweenHours` uses simple inclusive comparison of the `hour`
> field. A range that crosses midnight (e.g. `from: 22, to: 4`) will **only**
> match hour 22 and hour 23; it will not match hours 0–4 because `0 >= 22` is
> false. For a true cross-midnight window use `DuringPeriods([Night])` instead,
> or split the condition across two events.

---

## Complete Map File Example

Below is a self-contained example map that uses all four condition types.
Save it as `data/<your_campaign>/maps/crossroads.ron`.

```ron
// crossroads.ron — example map demonstrating time-gated events
Map(
    id: 10,
    name: "The Crossroads",
    description: "Four roads meet beneath an ancient standing stone.",
    width: 16,
    height: 16,
    tiles: [/* ... tile data ... */],
    allow_random_encounters: true,
    encounter_table: Some(EncounterTable(
        encounter_rate: 0.1,
        groups: [
            [1, 2],
            [3],
        ],
    )),
    events: {
        // ── Night-only ambush ──────────────────────────────────────────────
        (8, 8): MapEvent::Encounter(
            name: "Night Ambush",
            description: "Bandits strike from the shadows.",
            monster_group: [5, 5, 6],
            time_condition: Some(DuringPeriods([Night, Evening])),
        ),

        // ── Day-only travelling merchant ───────────────────────────────────
        (4, 8): MapEvent::NpcDialogue(
            name: "Travelling Merchant",
            description: "A merchant rests his cart at the crossroads.",
            npc_id: "merchant_crossroads",
            time_condition: Some(BetweenHours(from: 8, to: 18)),
        ),

        // ── Recruitable wanderer (only after day 3) ────────────────────────
        (8, 4): MapEvent::RecruitableCharacter(
            name: "Wandering Swordsman",
            description: "A veteran warrior looking for purpose.",
            character_id: "swordsman_edric",
            dialogue_id: Some(200),
            time_condition: Some(AfterDay(3)),
        ),

        // ── Early-game warning sign (gone after day 10) ────────────────────
        (12, 8): MapEvent::Sign(
            name: "Faded Warning",
            description: "A rotting wooden sign.",
            text: "Turn back, travellers — the eastern road is not safe for the weak.",
            time_condition: Some(BeforeDay(10)),
        ),

        // ── Unconditional treasure (no time_condition — always fires) ──────
        (8, 12): MapEvent::Treasure(
            name: "Abandoned Saddlebag",
            description: "Someone left in a hurry.",
            loot: [15, 22],
        ),
    },
)
```

---

## Backward Compatibility

Existing map RON files that do **not** include a `time_condition` field on any
event will continue to load and behave exactly as before. The field uses
`#[serde(default)]` which resolves to `None` when absent.

---

## Testing Your Time-Gated Events

1. Set the campaign's `starting_time` in `config.ron` to a time that satisfies
   your condition (e.g. `(day: 1, hour: 23, minute: 0)` for a night event).
2. Launch the game and step onto the event tile.
3. Verify the event fires as expected.
4. Advance the clock (rest, walk around) so the condition is no longer met and
   confirm the event returns `None`.

For automated testing, use `antares::domain::world::trigger_event` directly in
unit tests — pass a `GameTime` constructed for the exact time window you want
to test. See `src/domain/world/events.rs` for examples of this pattern.

---

## See Also

- `docs/how-to/creating_maps.md` — full map file format reference
- `docs/explanation/time_system_implementation_plan.md` — design decisions
- `src/domain/world/types.rs` — `TimeCondition` enum source of truth
- `src/domain/world/events.rs` — `trigger_event` implementation
