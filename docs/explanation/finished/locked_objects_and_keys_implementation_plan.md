# Locked Objects and Keys Implementation Plan

Phased implementation plan for locked doors, containers, key items, the
lockpicking skill, and the bash mechanic in the Antares game engine.

---

## Overview

This plan introduces three related mechanics that work together:

1. **Locked Objects** — Doors and containers that begin in a locked state and
   block movement or interaction until unlocked. A locked door `WallType::Door`
   with `blocked: true` stays impassable until the lock is resolved. A locked
   container `MapEvent` variant holds items that cannot be accessed until
   unlocked.

2. **Key Items** — Physical items (already representable as
   `ItemType::Quest(QuestData { is_key_item: true, .. })`) that, when the party
   carries one matching a lock's `key_item_id`, automatically unlock the object
   on interaction. Keys are consumed on use unless `consumable: false`.

3. **Lockpicking and Bash** — When the party lacks the key, a character with
   the `pick_lock` special ability (Robber class already has it in
   `data/classes.ron`) can attempt to pick the lock. Any character can attempt
   a bash regardless of class. Success and failure both have consequences
   (escalating trap chance, structural damage).

The architecture spec (Section 12.10) defines the full mechanical rules for
unlock and bash. This plan implements them faithfully without deviation.

---

### Current State (Before This Plan)

The following infrastructure already exists and **must not be re-implemented**:

| Component | Location | Status |
|-----------|----------|--------|
| `WallType::Door` tile variant | `src/domain/world/types.rs` | ✅ Exists |
| `MovementError::DoorLocked(x, y)` | `src/domain/world/movement.rs` | ✅ Defined (unused) |
| `MapEvent` enum (all existing variants) | `src/domain/world/types.rs` | ✅ Exists |
| `EventResult` enum | `src/domain/world/events.rs` | ✅ Exists |
| `ItemType::Quest(QuestData { is_key_item, .. })` | `src/domain/items/types.rs` | ✅ Exists |
| `ClassDefinition::has_ability("pick_lock")` | `src/domain/classes.rs` | ✅ Implemented |
| Robber class `special_abilities: ["pick_lock"]` | `data/classes.ron` | ✅ Data present |
| `DiceRoll` for randomised outcomes | `src/domain/types.rs` | ✅ Exists |
| Door open interaction (`E` key sets `WallType::None`) | `src/game/systems/input.rs` | ✅ Implemented (unlocked only) |
| `TransactionError` for domain-layer errors | `src/domain/transactions.rs` | ✅ Exists |
| `GameLog` for player feedback | `src/game/systems/ui.rs` | ✅ Exists |
| `Inventory::items` lookup by `item_id` | `src/domain/character.rs` | ✅ Exists |
| Save/load via RON serialisation | `src/application/save_game.rs` | ✅ Exists |

### What Is Missing (Gaps This Plan Closes)

| Gap | Phase |
|-----|-------|
| `LockState` domain type (locked, unlocked, trap chance) | Phase 1 |
| `MapEvent::LockedDoor` and `MapEvent::LockedContainer` variants | Phase 1 |
| `EventResult::Locked`, `EventResult::Unlocked`, `EventResult::LockpickFailed`, `EventResult::BashFailed`, `EventResult::TrapTriggered` | Phase 1 |
| `Map::lock_states: HashMap<String, LockState>` runtime mutable store | Phase 1 |
| `try_unlock` domain function (key check + lockpick + bash) | Phase 1 |
| Unlock transactions wired into `handle_input` door interaction | Phase 2 |
| `EventResult::Locked` shown as game log / simple dialogue message | Phase 2 |
| Lockpick UI prompt (choose Pick Lock or Bash, select character) | Phase 3 |
| Locked container `E`-key path into lockpick / bash choice | Phase 3 |
| Trap effect application on lockpick failure / bash | Phase 3 |
| Locked door visual differentiation (visual marker entity) | Phase 4 |
| Key items in `data/items.ron` and test campaign | Phase 4 |
| Locked door and container examples in tutorial campaign | Phase 4 |
| `LockState` serialised in `Map` and round-trips through save/load | Phase 5 |
| `docs/explanation/implementations.md` updated | Phase 5 |

---

## Architecture Constraints

Before writing any code, re-read the following architecture sections:

- Section 4.2 (`Tile`, `WallType`, `Map`, `World`) — the locking state must
  attach to the `Map` at runtime, not to `Tile` itself (the tile format is
  already stable; mutating it would require migrating all map RON files)
- Section 4.3 (`Inventory`, `InventorySlot`) — key lookup walks the active
  party's character inventories
- Section 4.6 (`ItemId`, `EventId` type aliases) — use these, never raw
  `u32`/`usize` for domain IDs
- Section 4.9 (`CampaignLoader`) — data files use RON format exclusively
- Section 12.10 (Unlock and Bash Mechanics) — the exact success/failure/trap
  rules defined here are **mandatory**, not suggestions

**Rules to verify before submitting each phase:**

- [ ] `ItemId`, `EventId` type aliases used — no raw integer types for
  domain identifiers
- [ ] `Inventory::MAX_ITEMS` and `Equipment::MAX_EQUIPPED` constants referenced
  where applicable — never hardcoded literals
- [ ] RON format for all new data files — no JSON or YAML for game content
- [ ] `///` doc comments on every new public function, struct, enum, and variant
- [ ] All test data in `data/test_campaign` — no test references
  `campaigns/tutorial`
- [ ] `cargo fmt --all` → no output
- [ ] `cargo check --all-targets --all-features` → 0 errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` → 0 warnings
- [ ] `cargo nextest run --all-features` → 0 failures

---

## Phase 1: Domain Layer — Lock State, Map Events, and Unlock Logic

**Goal:** Define all pure-domain types and functions needed to represent and
resolve locks. No Bevy, no ECS, no UI in this phase — only `src/domain/`.

### 1.1 Add `LockState` to `src/domain/world/lock.rs` (new file)

Create `src/domain/world/lock.rs`. This module owns every lock-related domain
type.

```rust
// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0
```

#### 1.1.1 `LockState`

```rust
/// Runtime mutable state for a single lock instance.
///
/// `LockState` is keyed by a `lock_id: String` that matches the `lock_id`
/// field on `MapEvent::LockedDoor` or `MapEvent::LockedContainer`. It is
/// stored in `Map::lock_states` at runtime and serialised with save data so
/// that unlocked doors remain open across save/load cycles.
///
/// # Examples
///
/// ```
/// use antares::domain::world::lock::LockState;
///
/// let state = LockState::new("dungeon_gate");
/// assert!(state.is_locked);
/// assert_eq!(state.trap_chance, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LockState {
    /// Unique identifier matching the `lock_id` in the map event.
    pub lock_id: String,
    /// Whether the lock is currently locked.
    pub is_locked: bool,
    /// Cumulative trap chance percentage (0–100).
    ///
    /// Starts at 0. Each failed lockpick attempt raises this by
    /// `LOCKPICK_FAIL_TRAP_INCREMENT`. Any bash attempt (success or failure)
    /// raises this by `BASH_TRAP_INCREMENT`. Capped at
    /// `TRAP_CHANCE_MAX`.
    pub trap_chance: u8,
}

/// Percentage added to `trap_chance` after each failed lockpick attempt.
pub const LOCKPICK_FAIL_TRAP_INCREMENT: u8 = 10;

/// Percentage added to `trap_chance` after any bash attempt.
pub const BASH_TRAP_INCREMENT: u8 = 20;

/// Maximum value `trap_chance` can reach.
pub const TRAP_CHANCE_MAX: u8 = 90;
```

Implement:

```rust
impl LockState {
    /// Creates a new locked `LockState` with zero trap chance.
    pub fn new(lock_id: impl Into<String>) -> Self { ... }

    /// Returns `true` if the lock has a non-zero trap chance and a trap
    /// should be checked before any unlock attempt.
    pub fn has_trap_risk(&self) -> bool { self.trap_chance > 0 }

    /// Increments trap chance by `delta`, capped at `TRAP_CHANCE_MAX`.
    pub fn increment_trap_chance(&mut self, delta: u8) { ... }

    /// Marks the lock as unlocked.
    pub fn unlock(&mut self) { self.is_locked = false; }
}
```

#### 1.1.2 `UnlockOutcome`

```rust
/// Result of a single unlock attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnlockOutcome {
    /// Lock opened with the correct key.
    OpenedWithKey { key_item_id: ItemId },
    /// Lock picked successfully by a character.
    LockpickSuccess { picker_party_index: usize },
    /// Lockpick attempt failed; trap chance increased.
    LockpickFailed { picker_party_index: usize, new_trap_chance: u8 },
    /// Bash succeeded; door destroyed.
    BashSuccess { basher_party_index: usize },
    /// Bash failed; door held. Trap chance increased.
    BashFailed { basher_party_index: usize, new_trap_chance: u8 },
    /// A trap fired before or during the attempt.
    TrapTriggered { damage: u16, effect: Option<String> },
    /// Party lacks the key and no unlock action was attempted.
    Locked { requires_key_item_id: Option<ItemId> },
}
```

#### 1.1.3 `try_unlock` pure function

```rust
/// Attempt to unlock a lock using the best available method.
///
/// Evaluation order:
/// 1. If the party carries `lock.key_item_id`, unlock immediately and
///    consume the key (remove it from the carrying character's inventory).
/// 2. Otherwise return `UnlockOutcome::Locked` with the required key ID.
///    The caller (Bevy system) then prompts the player to choose Pick Lock
///    or Bash.
///
/// This function does **not** perform lockpick or bash rolls — those are
/// handled by `try_lockpick` and `try_bash` to keep responsibility clear.
///
/// # Arguments
///
/// * `lock_state`   – Mutable runtime lock state to update on success.
/// * `party`        – Active party (checked for key item possession).
/// * `key_item_id`  – The item ID required to open this lock, if any.
/// * `item_db`      – Item database for key lookup.
///
/// # Returns
///
/// Returns `UnlockOutcome` describing what happened.
pub fn try_unlock(
    lock_state: &mut LockState,
    party: &mut Party,
    key_item_id: Option<ItemId>,
    item_db: &ItemDatabase,
) -> UnlockOutcome { ... }
```

Key lookup walks `party.members` inventories. When found, the `InventorySlot`
is removed from that character's `Inventory`. The `lock_state.unlock()` method
is called and `UnlockOutcome::OpenedWithKey` is returned.

If no key is present, return `UnlockOutcome::Locked`.

#### 1.1.4 `try_lockpick` pure function

```rust
/// Attempt to pick a lock.
///
/// Uses the character at `picker_party_index`. The character must have
/// `class_def.has_ability("pick_lock")` or the attempt automatically fails
/// with `LockpickFailed`.
///
/// Success formula (per architecture Section 12.10):
/// - Base chance: 30%
/// - +5% per character level above 1
/// - +10% bonus if class is "robber"
/// - Result clamped to [5%, 95%]
///
/// On failure: `trap_chance` is incremented by `LOCKPICK_FAIL_TRAP_INCREMENT`.
/// On success: `lock_state.unlock()` is called.
/// A trap roll is performed **before** the pick attempt if `trap_chance > 0`.
/// If the trap roll fires, `TrapTriggered` is returned immediately and no
/// pick roll is made.
///
/// # Arguments
///
/// * `lock_state`         – Mutable runtime lock state.
/// * `character`          – The character attempting to pick the lock.
/// * `picker_party_index` – Index used in the returned outcome.
/// * `class_db`           – Class database for ability lookup.
/// * `rng`                – Seeded random source (accepts `&mut impl Rng`).
pub fn try_lockpick<R: rand::Rng>(
    lock_state: &mut LockState,
    character: &Character,
    picker_party_index: usize,
    class_db: &ClassDatabase,
    rng: &mut R,
) -> UnlockOutcome { ... }
```

#### 1.1.5 `try_bash` pure function

```rust
/// Attempt to bash open a locked door or container.
///
/// No class restriction (architecture Section 12.10 explicitly states "No
/// class restrictions").
///
/// Success formula:
/// - Base chance: 25%
/// - +3% per character level
/// - +5% if character Might >= 15
/// - Result clamped to [5%, 80%]
///
/// On any bash attempt (success or failure): `trap_chance` is incremented
/// by `BASH_TRAP_INCREMENT`.
/// A trap roll is performed **before** the bash attempt if `trap_chance > 0`.
/// If the trap roll fires, `TrapTriggered` is returned immediately.
///
/// On success: `lock_state.unlock()` is called.
pub fn try_bash<R: rand::Rng>(
    lock_state: &mut LockState,
    character: &Character,
    basher_party_index: usize,
    rng: &mut R,
) -> UnlockOutcome { ... }
```

#### 1.1.6 `roll_trap` private helper

```rust
/// Roll to see if a trap fires, given the current `trap_chance`.
///
/// Returns `Some(TrapTriggered { damage, effect })` if the trap fires,
/// `None` otherwise.
///
/// Damage is 1d6 × (trap_chance / 10), minimum 1.
/// Effect is currently `None` (future phases may add status conditions).
fn roll_trap<R: rand::Rng>(trap_chance: u8, rng: &mut R) -> Option<UnlockOutcome> { ... }
```

### 1.2 Add `MapEvent::LockedDoor` and `MapEvent::LockedContainer`

**File:** `src/domain/world/types.rs`

Add two new variants to `MapEvent`:

```rust
/// A locked door that blocks passage until unlocked.
///
/// The door tile itself uses `WallType::Door` with `blocked: true`. When the
/// lock is resolved, the engine clears `blocked` and sends a
/// `DoorOpenedEvent`.  The `lock_id` is the key into `Map::lock_states`.
LockedDoor {
    /// Display name used in game log messages.
    #[serde(default)]
    name: String,
    /// Unique identifier for this lock instance.
    ///
    /// Must match a `LockState::lock_id` in `Map::lock_states`.
    lock_id: String,
    /// The `ItemId` of the key that opens this lock, if any.
    ///
    /// `None` means the door can only be opened by lockpicking or bashing.
    #[serde(default)]
    key_item_id: Option<crate::domain::types::ItemId>,
    /// Initial trap chance when the map loads (0–90).
    ///
    /// Copied into `LockState::trap_chance` during map initialisation.
    #[serde(default)]
    initial_trap_chance: u8,
},

/// A locked container (chest, barrel, crate) that holds items.
///
/// Uses the same lock resolution mechanics as `LockedDoor`. Once unlocked
/// the engine transitions to `GameMode::ContainerInventory` with the
/// container's item list.
LockedContainer {
    /// Display name shown in the right-panel header when opened.
    #[serde(default)]
    name: String,
    /// Unique identifier for this lock instance.
    lock_id: String,
    /// The `ItemId` of the key that opens this lock, if any.
    #[serde(default)]
    key_item_id: Option<crate::domain::types::ItemId>,
    /// Items inside the container, accessible after unlocking.
    #[serde(default)]
    items: Vec<crate::domain::character::InventorySlot>,
    /// Initial trap chance (0–90).
    #[serde(default)]
    initial_trap_chance: u8,
},
```

Because `MapEvent` is `#[derive(Serialize, Deserialize)]`, the new variants
serialise automatically. All existing RON map files are unaffected — RON
ignores unknown enum variants during deserialisation (it errors on unknown
**fields** but not on unknown variants when the enum uses externally-tagged
format, which is RON's default for enums).

### 1.3 Add `lock_states` to `Map`

**File:** `src/domain/world/types.rs`

Add a field to `pub struct Map`:

```rust
/// Runtime lock states for all locked objects on this map.
///
/// Keyed by `LockState::lock_id`. Populated from `MapEvent::LockedDoor` and
/// `MapEvent::LockedContainer` during map loading/initialisation. Serialised
/// with save data so unlock state persists across save/load cycles.
#[serde(default, skip_serializing_if = "HashMap::is_empty")]
pub lock_states: HashMap<String, crate::domain::world::lock::LockState>,
```

Use `#[serde(default)]` so all existing map RON files that lack this field
deserialise without error.

Add a helper to `impl Map`:

```rust
/// Initialise lock states from all `LockedDoor` and `LockedContainer` events.
///
/// Called once when a map is loaded. Entries already present in
/// `lock_states` (loaded from save data) are **not** overwritten, so a
/// previously unlocked door remains open after loading.
pub fn init_lock_states(&mut self) { ... }
```

### 1.4 Add `EventResult` variants

**File:** `src/domain/world/events.rs`

Add to `EventResult`:

```rust
/// A locked object was interacted with; the lock was not resolved.
Locked {
    /// The `lock_id` of the object.
    lock_id: String,
    /// Required key item ID, if any.
    requires_key_item_id: Option<crate::domain::types::ItemId>,
},
/// A lock was successfully opened.
Unlocked {
    lock_id: String,
    /// How it was opened.
    method: UnlockMethod,
},
/// A lockpick attempt failed.
LockpickFailed {
    lock_id: String,
    new_trap_chance: u8,
},
/// A bash attempt failed.
BashFailed {
    lock_id: String,
    new_trap_chance: u8,
},
/// A trap was triggered.
TrapTriggered {
    lock_id: String,
    damage: u16,
    effect: Option<String>,
},
```

Add a small supporting enum:

```rust
/// How a lock was opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnlockMethod {
    Key { item_id: crate::domain::types::ItemId },
    Lockpick { picker_party_index: usize },
    Bash { basher_party_index: usize },
}
```

### 1.5 Export from `src/domain/world/mod.rs`

Add:

```rust
pub mod lock;
pub use lock::{LockState, UnlockOutcome, try_unlock, try_lockpick, try_bash,
               LOCKPICK_FAIL_TRAP_INCREMENT, BASH_TRAP_INCREMENT, TRAP_CHANCE_MAX};
```

### 1.6 Testing Requirements for Phase 1

All tests live in `src/domain/world/lock.rs` under `#[cfg(test)] mod tests`.
All test data uses `data/test_campaign`.

**Required tests:**

- `test_lock_state_new_is_locked` — new `LockState` starts locked with
  `trap_chance == 0`
- `test_lock_state_unlock` — `unlock()` sets `is_locked = false`
- `test_lock_state_increment_trap_chance_clamps` — cannot exceed
  `TRAP_CHANCE_MAX`
- `test_try_unlock_with_correct_key_removes_item_from_inventory` — party
  member has key item; `try_unlock` returns `OpenedWithKey` and item is gone
- `test_try_unlock_with_wrong_key_returns_locked` — party has a key but wrong
  `ItemId`; returns `Locked`
- `test_try_unlock_no_key_required_still_needs_lockpick_or_bash` — lock has
  `key_item_id: None`; `try_unlock` returns `Locked` (pick/bash handled
  separately)
- `test_try_lockpick_success_unlocks` — seeded RNG producing a roll below
  success threshold; outcome is `LockpickSuccess`
- `test_try_lockpick_failure_increments_trap_chance` — roll above threshold;
  `trap_chance` increased by `LOCKPICK_FAIL_TRAP_INCREMENT`
- `test_try_lockpick_class_without_pick_lock_always_fails` — knight character
  (no `pick_lock` ability) always returns `LockpickFailed`
- `test_try_lockpick_trap_fires_before_attempt` — `trap_chance == 100`
  (impossible to not fire); outcome is `TrapTriggered` before pick roll
- `test_try_bash_success_unlocks` — seeded RNG; `BashSuccess`
- `test_try_bash_failure_increments_trap_chance` — `BashFailed` with
  incremented `trap_chance`
- `test_try_bash_no_class_restriction` — sorcerer character can bash
- `test_map_init_lock_states_populates_from_events` — map with two
  `LockedDoor` events; `init_lock_states()` creates two `LockState` entries
- `test_map_init_lock_states_does_not_overwrite_existing` — pre-populate
  one unlocked state; `init_lock_states()` does not re-lock it

### 1.7 Deliverables

- [ ] `src/domain/world/lock.rs` — `LockState`, `UnlockOutcome`,
  `try_unlock`, `try_lockpick`, `try_bash`
- [ ] `src/domain/world/types.rs` — `MapEvent::LockedDoor`,
  `MapEvent::LockedContainer`, `Map::lock_states`
- [ ] `src/domain/world/events.rs` — new `EventResult` variants
- [ ] `src/domain/world/mod.rs` — exports updated
- [ ] All tests passing; all four quality gates pass

### 1.8 Success Criteria

1. `try_unlock` correctly consumes a matching key from party inventory.
2. `try_lockpick` applies the Robber class bonus and increments trap chance on
   failure.
3. `try_bash` works for any class and increments trap chance on any attempt.
4. Trap fires before the unlock attempt when `trap_chance > 0` and the RNG
   roll is below threshold.
5. `Map::lock_states` is populated by `init_lock_states()` and existing states
   are not overwritten.

---

## Phase 2: Interaction Wiring — Locked Door `E`-Key Path

**Goal:** Pressing `E` in front of a locked door checks the party for the key,
shows a game log message if locked, and fires the key-unlock path immediately
if the key is present. If neither key nor lockpick/bash has been chosen yet,
the `EventResult::Locked` message is shown and a pending action is stored
awaiting Phase 3's UI prompt.

### 2.1 Call `map.init_lock_states()` on Map Load

**File:** `src/game/systems/map.rs` (or wherever map loading is handled)

After a map finishes loading its tile/event data, call
`map.init_lock_states()` once. This seeds the `lock_states` `HashMap` from
the map's `LockedDoor` and `LockedContainer` events.

This must run before any player interaction is possible. The correct insertion
point is after `Map` deserialization completes, in the startup or map-change
system.

### 2.2 Extend Door Interaction in `handle_input`

**File:** `src/game/systems/input.rs`

The current door interaction code in `handle_input` is:

```rust
if tile.wall_type == WallType::Door {
    tile.wall_type = WallType::None;
    door_messages.write(DoorOpenedEvent { position: target });
    return;
}
```

Replace this with a check that first queries whether there is an associated
`LockedDoor` event at the target position:

1. Look up whether `map.get_event(target)` is
   `MapEvent::LockedDoor { lock_id, key_item_id, .. }`.
2. Look up `map.lock_states.get(lock_id)`.
3. If `lock_state.is_locked == true`:
   a. Call `try_unlock(lock_state, party, key_item_id, item_db)`.
   b. If `OpenedWithKey`: open the door (set `WallType::None`, clear
      `blocked`, remove the `LockedDoor` event, send `DoorOpenedEvent`,
      log success message).
   c. If `Locked`: write a `LockInteractionPending` resource/message so Phase
      3's UI can prompt Pick Lock / Bash. Log "The door is locked." to the
      game log. Do **not** open the door.
   d. If `TrapTriggered`: apply trap damage to party (split evenly, minimum 1
      per member), log trap message.
4. If `lock_state.is_locked == false`: open the door normally (the player
   re-interacts with a door they already unlocked in a prior step that left
   `WallType::Door` visible — clear it to `None`).
5. If there is **no** `LockedDoor` event at the target: existing behaviour
   (open the door immediately).

`item_db` requires adding `Option<Res<GameContent>>` to `handle_input`'s
system parameters. The database is accessed as
`game_content.as_deref().map(|gc| &gc.db().items)`.

#### 2.2.1 `LockInteractionPending` resource

Add a Bevy `Resource` to signal Phase 3's UI:

```rust
/// Signals that the player just interacted with a locked object and must
/// choose whether to pick the lock or bash it open.
#[derive(Resource, Default, Debug, Clone)]
pub struct LockInteractionPending {
    /// The lock_id of the object awaiting action.
    pub lock_id: Option<String>,
    /// The position of the locked object on the current map.
    pub position: Option<crate::domain::types::Position>,
    /// Whether lockpicking is available (true if any party member has
    /// `pick_lock` ability).
    pub can_lockpick: bool,
}
```

Register this resource with `app.init_resource::<LockInteractionPending>()`
in `InputPlugin::build`.

### 2.3 Add `LockedDoor` Handling to `handle_events`

**File:** `src/game/systems/events.rs`

Although locked door interaction is primarily triggered from `handle_input`
(player explicitly presses `E`), the `handle_events` system must also handle
`MapEvent::LockedDoor` if it arrives via the `MapEventTriggered` message (e.g.
from programmatic tests). Add a match arm:

```rust
MapEvent::LockedDoor { lock_id, key_item_id, .. } => {
    // Check lock state and attempt key-unlock; set LockInteractionPending
    // if locked and no key present. Same logic as Phase 2.2 but operating
    // on the message-triggered path.
}
```

### 2.4 Game Log Messages for Lock Interactions

All messages are written to `GameLog` (the `ResMut<GameLog>` already passed
through `handle_input` and `handle_events`):

| Outcome | Message |
|---------|---------|
| Locked, key required | `"The door is locked. You need a key."` |
| Locked, no key needed | `"The door is locked."` |
| `OpenedWithKey` | `"You unlock the door with the {key_name}."` |
| `TrapTriggered` | `"A trap fires! The party takes {damage} damage."` |

The key name is looked up from `item_db.get_item(key_item_id).name`.

### 2.5 Testing Requirements for Phase 2

**New integration tests in `src/game/systems/input.rs`:**

- `test_e_key_on_regular_door_opens_it` — unlocked `WallType::Door` with no
  `LockedDoor` event; pressing `E` sets tile to `WallType::None`
- `test_e_key_on_locked_door_with_correct_key_opens_it` — party has matching
  key; door opens, key consumed, game log contains success message
- `test_e_key_on_locked_door_without_key_sets_pending` — no key; door stays
  locked, `LockInteractionPending.lock_id` is `Some(...)`, game log contains
  locked message
- `test_e_key_on_locked_door_wrong_key_stays_locked` — party has different key;
  stays locked

**New integration tests in `src/game/systems/events.rs`:**

- `test_locked_door_event_sets_pending_resource` — `MapEvent::LockedDoor`
  arrives via `MapEventTriggered`; assert `LockInteractionPending` is set

### 2.6 Deliverables

- [ ] `src/game/systems/map.rs` (or loader) — `map.init_lock_states()` called
  on map load
- [ ] `src/game/systems/input.rs` — locked door check before plain door open;
  `LockInteractionPending` resource registered and populated
- [ ] `src/game/systems/events.rs` — `MapEvent::LockedDoor` match arm
- [ ] Game log messages for all lock outcomes
- [ ] All four quality gates pass

### 2.7 Success Criteria

1. Pressing `E` in front of a door with no `LockedDoor` event still opens it
   immediately (no regression).
2. Pressing `E` in front of a locked door with the correct key in the party
   opens the door and consumes the key.
3. Pressing `E` in front of a locked door without the key leaves it locked and
   sets `LockInteractionPending`.
4. A trap that fires reduces the party's HP correctly.

---

## Phase 3: Lockpick / Bash UI Prompt and Locked Container Interaction

**Goal:** When `LockInteractionPending` is set, a small modal UI prompts the
player to choose Pick Lock (Robber only) or Bash, and to select the acting
character. Locked containers use the same prompt before revealing their items.

### 3.1 Create `src/game/systems/lock_ui.rs` (new file)

```rust
// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Lock interaction UI — Pick Lock / Bash prompt.
```

#### 3.1.1 `LockActionChosen` message

```rust
/// Emitted when the player selects an action from the lock UI prompt.
#[derive(Message)]
pub struct LockActionChosen {
    pub lock_id: String,
    pub position: crate::domain::types::Position,
    pub action: LockAction,
    pub party_index: usize,
}

/// The player's chosen action for a lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockAction {
    Lockpick,
    Bash,
}
```

#### 3.1.2 `lock_prompt_ui_system`

The system runs only when `LockInteractionPending.lock_id.is_some()`. It
renders a small centred `egui::Window` with:

```text
┌──────────────────────────────────────┐
│  Locked Door                         │
│                                      │
│  [Pick Lock]  (Robber only)          │
│  [Bash]       (Any character)        │
│  [Cancel]                            │
│                                      │
│  Character: [1] Alice  [2] Bob  ...  │
└──────────────────────────────────────┘
```

- **Pick Lock** is greyed out (disabled egui button) if no party member has
  the `pick_lock` class ability.
- Character selection uses number keys `1`–`6` and mouse click on character
  name labels.
- `Cancel` clears `LockInteractionPending` and returns without action.
- Clicking a primary action button with a character selected emits
  `LockActionChosen` and clears `LockInteractionPending`.

Keyboard navigation:
- `Tab` cycles between buttons.
- `Enter` confirms focused button.
- `Esc` cancels (same as Cancel button).

#### 3.1.3 `lock_action_system`

Reads `LockActionChosen` messages and performs the chosen unlock attempt:

1. Look up `map.lock_states.get_mut(lock_id)`.
2. Get the acting character from `party.members[party_index]`.
3. Dispatch to `try_lockpick` or `try_bash` using a seeded RNG
   (`rand::thread_rng()` is acceptable here; the pure functions are
   independently testable with seeded RNGs).
4. Handle `UnlockOutcome`:
   - `LockpickSuccess` / `BashSuccess` → open the object (door or container),
     log success, clear `LockInteractionPending`.
   - `LockpickFailed` / `BashFailed` → log failure message with updated trap
     chance, clear `LockInteractionPending`.
   - `TrapTriggered` → apply damage to party, log trap message, clear
     `LockInteractionPending`.

For a **door** success: set the tile `wall_type = WallType::None`, `blocked =
false`, remove the `LockedDoor` map event, send `DoorOpenedEvent`.

For a **container** success: call
`game_state.enter_container_inventory(lock_id, name, items)`, remove the
`LockedContainer` map event (or replace it with an unlocked `MapEvent`
placeholder so the container is accessible again without re-triggering the
prompt).

#### 3.1.4 `LockUiPlugin`

```rust
pub struct LockUiPlugin;

impl Plugin for LockUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<LockActionChosen>()
            .add_systems(Update, (lock_prompt_ui_system, lock_action_system).chain());
    }
}
```

Register in `src/bin/antares.rs`:

```rust
app.add_plugins(antares::game::systems::lock_ui::LockUiPlugin);
```

### 3.2 Locked Container `E`-Key Path

**File:** `src/game/systems/input.rs`

When the party presses `E` while facing a `MapEvent::LockedContainer`:

1. Follow the same key-check logic as Phase 2.2 for doors.
2. If key present: open immediately (transition to
   `GameMode::ContainerInventory` with the container's item list). Remove the
   `LockedContainer` event and replace with a `MapEvent::Container` (or
   equivalent unlocked variant from the buy/sell plan).
3. If no key: set `LockInteractionPending` as for locked doors. Phase 3's UI
   prompt handles the rest; `lock_action_system` opens the container on
   success.

### 3.3 Failure Messages

| Outcome | Game Log Message |
|---------|-----------------|
| `LockpickFailed` (first try) | `"You fail to pick the lock. Trap chance: {n}%."` |
| `LockpickFailed` (no ability) | `"Only a skilled Robber can pick this lock."` |
| `BashFailed` | `"You fail to break open the door. Trap chance: {n}%."` |
| `LockpickSuccess` | `"{name} picks the lock!"` |
| `BashSuccess` | `"{name} smashes the door open!"` |
| Container unlocked | `"The {name} is unlocked."` |

### 3.4 Testing Requirements for Phase 3

**New tests in `src/game/systems/lock_ui.rs`:**

- `test_lock_action_chosen_fields` — construct `LockActionChosen` and verify
  fields
- `test_lock_action_system_bash_success_opens_door` — inject
  `LockActionChosen { action: Bash }` with a seeded RNG always succeeding;
  assert door tile is set to `WallType::None`
- `test_lock_action_system_bash_failure_logs_message` — seeded RNG always
  failing; assert game log contains failure message and `lock_state.is_locked`
  remains `true`
- `test_lock_action_system_lockpick_no_ability_logs_rejection` — knight
  character; assert rejected with appropriate message
- `test_lock_action_system_trap_triggered_damages_party` — `trap_chance == 100`
  (always fires); assert party HP reduced
- `test_lock_action_system_container_success_enters_container_mode` — bash
  success on a `LockedContainer`; assert game mode is `ContainerInventory`

### 3.5 Deliverables

- [ ] `src/game/systems/lock_ui.rs` — `LockUiPlugin`, `lock_prompt_ui_system`,
  `lock_action_system`, `LockActionChosen`, `LockAction`
- [ ] `src/game/systems/input.rs` — locked container path added alongside
  locked door path
- [ ] `src/bin/antares.rs` — `LockUiPlugin` registered
- [ ] All four quality gates pass

### 3.6 Success Criteria

1. When `LockInteractionPending` is set, the lock prompt UI appears.
2. Pick Lock button is disabled if no party member has the ability.
3. Selecting Bash with any character and confirming executes the bash attempt.
4. Successful bash on a door opens it; successful bash/pick on a container
   transitions to `ContainerInventory`.
5. Trap damage is applied to the party on `TrapTriggered`.
6. Cancel clears the pending state with no side effects.

---

## Phase 4: Visual Differentiation and Content Data

**Goal:** Locked objects are visually distinct from unlocked ones. Key items
exist in the item database. Tutorial and test campaign maps include locked
doors and containers with matching keys.

### 4.1 Locked Door Visual Marker

**File:** `src/game/systems/map.rs` (tile spawning) or
`src/game/systems/procedural_meshes.rs`

When a `MapEvent::LockedDoor` is present at a tile position and its
`LockState::is_locked == true`, spawn a small visual marker on or adjacent to
the door mesh to indicate it is locked. A suitable approach is:

- Reuse the existing `spawn_chest` or a simple padlock mesh variant from
  `procedural_meshes.rs` scaled to 0.2× and positioned at door-frame height.
- Give it a distinct colour tint: deep amber (`[0.8, 0.5, 0.1]`) to
  differentiate from unlocked (which has no marker).

When the door is unlocked, despawn the marker entity. Track the marker entity
with a `LockedDoorMarker` Bevy `Component` keyed by `lock_id`:

```rust
#[derive(Component)]
pub struct LockedDoorMarker {
    pub lock_id: String,
}
```

Despawn all `LockedDoorMarker` entities matching `lock_id` in
`lock_action_system` after a successful unlock.

### 4.2 Add Key Items to `data/items.ron`

Add at least three key items to `data/items.ron` (base data, not
campaign-specific). These serve as fixtures for integration tests and as
examples for campaign authors:

```ron
// Dungeon Gate Key — opens dungeon entrance doors
(
    id: 200,
    name: "Dungeon Gate Key",
    item_type: Quest((
        quest_id: "dungeon_key",
        is_key_item: true,
    )),
    base_cost: 0,
    sell_cost: 0,
    // ... other fields at default
),

// Chest Key — generic chest key
(
    id: 201,
    name: "Old Chest Key",
    item_type: Quest((
        quest_id: "chest_key",
        is_key_item: true,
    )),
    base_cost: 0,
    sell_cost: 0,
),

// Iron Lock Key — used for locked iron doors in tutorial
(
    id: 202,
    name: "Iron Lock Key",
    item_type: Quest((
        quest_id: "iron_lock_key",
        is_key_item: true,
    )),
    base_cost: 0,
    sell_cost: 0,
),
```

Mirror these entries in `data/test_campaign/data/items.ron` (or the test
campaign's items override file). IDs 200–202 must not conflict with existing
item IDs — verify against `data/items.ron` before assigning.

### 4.3 Add Locked Objects to Test Campaign

**File:** `data/test_campaign/data/maps/map_1.ron`

Add:

1. One `MapEvent::LockedDoor` at an accessible position with
   `key_item_id: Some(200)` and the corresponding tile set to
   `wall_type: Door, blocked: true`.
2. One `MapEvent::LockedDoor` at a second position with
   `key_item_id: None` (lockpick/bash only).
3. One `MapEvent::LockedContainer` at a third position with
   `key_item_id: Some(201)` and two items inside.

Add item 200 (Dungeon Gate Key) to the starting inventory of the first
character in `data/test_campaign/data/characters.ron` so integration tests can
exercise the key-unlock path without manual setup.

### 4.4 Add Locked Objects to Tutorial Campaign

**File:** `campaigns/tutorial/data/maps/map_1.ron`

Add a locked chest somewhere in the tutorial map that teaches the mechanic to
new players. Use `key_item_id: Some(200)` and place the matching key as
treasure or quest reward earlier in the same map.

Add a locked door leading to an optional area with `key_item_id: None`
(lockpick/bash required) so players can discover the alternative mechanics.

**Rule:** Tutorial campaign changes are for live game content only, never for
tests. All integration tests reference `data/test_campaign`.

### 4.5 SDK Validation Update

**File:** `src/sdk/validation.rs`

Add two new validation rules:

1. Every `MapEvent::LockedDoor` and `MapEvent::LockedContainer` with a
   non-`None` `key_item_id` must reference an item that exists in the
   `ItemDatabase` and has `is_key_item: true` (i.e. is
   `ItemType::Quest(QuestData { is_key_item: true })`).
2. Every `lock_id` in `LockedDoor` / `LockedContainer` events on a map must be
   unique within that map (no two events share the same `lock_id`).

### 4.6 Testing Requirements for Phase 4

**New tests in `src/sdk/validation.rs`:**

- `test_locked_door_with_valid_key_item_passes_validation`
- `test_locked_door_with_invalid_key_item_id_fails_validation`
- `test_locked_door_with_non_key_item_fails_validation`
- `test_locked_door_with_duplicate_lock_id_fails_validation`

**New integration tests in `src/game/systems/map.rs`:**

- `test_locked_door_marker_spawned_on_map_load` — verify a
  `LockedDoorMarker` entity is present after loading a map with a
  `LockedDoor` event
- `test_locked_door_marker_despawned_after_unlock` — bash/pick success removes
  the marker entity

### 4.7 Deliverables

- [ ] `src/game/systems/map.rs` — `LockedDoorMarker` component; spawn/despawn
  logic
- [ ] `data/items.ron` — items 200–202 added
- [ ] `data/test_campaign/data/items.ron` — items 200–202 mirrored
- [ ] `data/test_campaign/data/maps/map_1.ron` — locked door and container
  fixtures added; key item in starting inventory
- [ ] `campaigns/tutorial/data/maps/map_1.ron` — locked chest and locked door
  added
- [ ] `src/sdk/validation.rs` — key item validity and lock_id uniqueness rules
- [ ] All four quality gates pass

### 4.8 Success Criteria

1. The locked door marker is visible in game and despawns when the door is
   unlocked.
2. The campaign validator rejects maps where `key_item_id` references a
   non-key item or a non-existent item.
3. The campaign validator rejects maps with duplicate `lock_id` values.
4. Tutorial players can find the key, use it on the locked chest, and receive
   the contents.

---

## Phase 5: Save/Load Persistence, Trap Effects, and Documentation

**Goal:** Lock states survive save/load. Traps apply real status conditions
from the architecture condition system. Documentation is updated.

### 5.1 Verify `lock_states` Serialisation Round-Trip

`Map::lock_states` is `#[serde(default)]` so it already participates in the
standard `GameState` RON serialisation via `save_game`. Verify correctness with
a targeted test:

**File:** `src/application/save_game.rs`

Test name: `test_save_load_preserves_unlocked_door_state`

1. Load a `GameState` with a map containing a `LockedDoor`.
2. Call `map.init_lock_states()` to seed the runtime state.
3. Unlock the door via `lock_state.unlock()`.
4. Serialise with `save_game`.
5. Deserialise with `load_game`.
6. Assert the loaded map's `lock_states` entry has `is_locked == false`.
7. Assert the loaded map's tile at the door position has `wall_type ==
   WallType::None` (the tile mutation from unlock must also persist — this
   means the tile mutation is part of the save state, which it already is since
   `Tile` is serialised with the map).

Test name: `test_save_load_preserves_trap_chance`

1. Create a lock state with `trap_chance == 30`.
2. Round-trip through save/load.
3. Assert `trap_chance == 30` after load.

### 5.2 Trap Status Condition Effects

**File:** `src/game/systems/lock_ui.rs` (`lock_action_system`)

In Phase 1, `roll_trap` returns `TrapTriggered { damage, effect: None }`. In
this phase, extend `roll_trap` to produce real effects by mapping the
`trap_chance` range to condition strings:

| `trap_chance` range | Effect |
|---------------------|--------|
| 0–29 | `None` (damage only) |
| 30–59 | `Some("poison")` → apply `Condition::POISONED` to the lead character |
| 60–89 | `Some("paralysis")` → apply `Condition::PARALYZED` to all party members |
| 90+ | `Some("teleport")` → teleport party to map start position |

Apply conditions using the existing `character.conditions` field (a `Condition`
bitmask). The `Condition` constants (`POISONED`, `PARALYZED`) are already
defined in the architecture and codebase.

The teleport effect sets `world.party_position` to `map.starting_position` (or
`Position::new(1, 1)` as a safe fallback if the map has no configured start).

### 5.3 Lockpick Skill Progression

**File:** `src/domain/world/lock.rs`

The current `try_lockpick` formula uses a flat per-level bonus. Extend it to
also check the character's `stats.speed.current` (a higher Speed stat
represents nimble fingers):

```text
success_chance = base(30)
               + (level - 1) * 5
               + if class == "robber" { 10 } else { 0 }
               + (speed.current.saturating_sub(10) / 2)
```

Clamp to `[5, 95]`. This uses the `AttributePair` pattern: read
`character.stats.speed.current`, not `.base`.

Update the affected tests to reflect the new formula.

### 5.4 Update `docs/explanation/implementations.md`

Add a new section summarising the locked objects and keys implementation:

- What existed before this plan (the `MovementError::DoorLocked` stub,
  `ClassDefinition::has_ability("pick_lock")` on the Robber)
- What each phase added
- File locations for all new/modified modules
- Known limitations (no per-character lockpick XP gain, no master key item,
  no magic unlock spell integration)
- Data-authoring guide: how to add a locked door to a campaign map in RON

### 5.5 Testing Requirements for Phase 5

**In `src/application/save_game.rs`:**

- `test_save_load_preserves_unlocked_door_state`
- `test_save_load_preserves_trap_chance`

**In `src/domain/world/lock.rs`:**

- `test_roll_trap_poison_range` — `trap_chance == 45`; assert effect is
  `"poison"`
- `test_roll_trap_paralysis_range` — `trap_chance == 70`; assert effect is
  `"paralysis"`
- `test_try_lockpick_speed_bonus_applied` — character with Speed 16 has higher
  success chance than identical character with Speed 10

**In `src/game/systems/lock_ui.rs`:**

- `test_trap_poison_effect_applies_condition_to_lead_character` — trap fires
  with `"poison"`; assert lead character has `Condition::POISONED` set
- `test_trap_paralysis_effect_applies_to_all_party_members`
- `test_trap_teleport_effect_moves_party_to_start`

### 5.6 Deliverables

- [ ] `src/application/save_game.rs` — persistence tests added
- [ ] `src/domain/world/lock.rs` — `roll_trap` produces condition effects;
  `try_lockpick` incorporates Speed bonus
- [ ] `src/game/systems/lock_ui.rs` — `lock_action_system` applies trap
  conditions and teleport
- [ ] `docs/explanation/implementations.md` — updated with summary section
- [ ] All four quality gates pass

### 5.7 Success Criteria

1. Unlocking a door in one session and saving/loading leaves the door open.
2. Trap conditions (poison, paralysis) are applied to the correct characters.
3. A teleport trap moves the party to the map start position.
4. The Speed stat meaningfully increases lockpick success chance.
5. `implementations.md` accurately describes the implemented system and
   provides a RON authoring example.

---

## Candidate Files Summary

### New Files

| File | Purpose |
|------|---------|
| `src/domain/world/lock.rs` | `LockState`, `UnlockOutcome`, domain functions |
| `src/game/systems/lock_ui.rs` | `LockUiPlugin`, prompt UI, action system |

### Modified Files

| File | Phase | Nature of Change |
|------|-------|------------------|
| `src/domain/world/types.rs` | 1 | `MapEvent::LockedDoor`, `MapEvent::LockedContainer`, `Map::lock_states` |
| `src/domain/world/events.rs` | 1 | New `EventResult` variants, `UnlockMethod` enum |
| `src/domain/world/mod.rs` | 1 | Export `lock` module and its public items |
| `src/game/systems/map.rs` | 2, 4 | `init_lock_states()` on load; `LockedDoorMarker` spawn/despawn |
| `src/game/systems/input.rs` | 2, 3 | Locked door/container check before plain door open; `LockInteractionPending` |
| `src/game/systems/events.rs` | 2 | `MapEvent::LockedDoor` match arm |
| `src/bin/antares.rs` | 3 | Register `LockUiPlugin` |
| `data/items.ron` | 4 | Key items 200–202 added |
| `data/test_campaign/data/items.ron` | 4 | Key items 200–202 mirrored |
| `data/test_campaign/data/maps/map_1.ron` | 4 | Locked door and container test fixtures |
| `campaigns/tutorial/data/maps/map_1.ron` | 4 | Tutorial locked chest and locked door |
| `src/sdk/validation.rs` | 4 | Key item validity and `lock_id` uniqueness rules |
| `src/application/save_game.rs` | 5 | Persistence tests |
| `docs/explanation/implementations.md` | 5 | Implementation summary |

---

## Quality Gate Checklist (Run After Every Phase)

```text
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

Expected results:

```text
✅ cargo fmt         → No output
✅ cargo check       → "Finished" with 0 errors
✅ cargo clippy      → "Finished" with 0 warnings
✅ cargo nextest run → all tests pass; 0 failed
```

**IF ANY GATE FAILS, STOP AND FIX BEFORE PROCEEDING TO THE NEXT PHASE.**

---

## Architecture Compliance Checklist

- [ ] `ItemId`, `EventId` type aliases used — no raw integer types for domain IDs
- [ ] `Inventory::MAX_ITEMS` constant used where slot limits are checked —
  never hardcoded numeric literal
- [ ] `AttributePair` pattern respected — `stats.speed.current` read for
  lockpick bonus, never `stats.speed.base` directly
- [ ] `Condition::POISONED`, `Condition::PARALYZED` constants used — never
  raw bitmask literals
- [ ] RON format for all new data files — no JSON or YAML for game content
- [ ] `///` doc comments on every new public function, struct, enum, and variant
- [ ] All test data in `data/test_campaign` — no test references
  `campaigns/tutorial`
- [ ] `LockState` serialised via `#[serde(default)]` — no parallel
  serialisation path introduced
- [ ] `try_unlock`, `try_lockpick`, `try_bash` are pure functions with no
  Bevy dependencies — testable in isolation with seeded RNGs
- [ ] No architectural deviations from `docs/reference/architecture.md`
  Section 12.10
