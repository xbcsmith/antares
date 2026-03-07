# Procedural Meshes Direction Control Implementation Plan

## Overview

Creatures (NPCs, recruitable characters, monsters), signs, and furniture
spawned as procedural meshes currently all face the same default direction
because `spawn_creature()` ignores rotation and `MapEvent` variants carry no
`facing` field. This plan extends the domain data model with per-entity
facing, wires that facing through spawning, and adds a runtime event system
so direction can change during gameplay (e.g., a monster turning toward the
player).

The work spans four phases: a direction-to-rotation utility layer, static
map-time facing for all entity types, a runtime `SetFacing` event system,
and optional smooth rotation animation.

---

## Current State Analysis

### Existing Infrastructure

| Component                        | Location                                                                             | State                                                                                           |
| -------------------------------- | ------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| `Direction` enum                 | [src/domain/types.rs](../../src/domain/types.rs)                                     | `North/East/South/West` with `turn_left()`, `turn_right()`, `forward()` — **no yaw conversion** |
| `NpcPlacement.facing`            | [src/domain/world/npc.rs](../../src/domain/world/npc.rs)                             | `Option<Direction>` — present in domain, stored in map RON                                      |
| `ResolvedNpc.facing`             | [src/domain/world/types.rs](../../src/domain/world/types.rs)                         | Propagated from `NpcPlacement`                                                                  |
| `spawn_creature()`               | [src/game/systems/creature_spawning.rs](../../src/game/systems/creature_spawning.rs) | Takes `Vec3 position` — **no rotation parameter**                                               |
| NPC creature spawn in `map.rs`   | [src/game/systems/map.rs](../../src/game/systems/map.rs) ~L1035                      | Calls `spawn_creature()` without `resolved_npc.facing`                                          |
| Encounter creature spawn         | [src/game/systems/map.rs](../../src/game/systems/map.rs) ~L1164                      | Spawns at identity rotation                                                                     |
| `RecruitableCharacter` spawn     | [src/game/systems/map.rs](../../src/game/systems/map.rs) ~L1211                      | Spawns at identity rotation                                                                     |
| `MapEvent::Furniture`            | [src/domain/world/types.rs](../../src/domain/world/types.rs)                         | Has `rotation_y: Option<f32>` — **reference pattern**                                           |
| `MapEvent::Sign`                 | [src/domain/world/types.rs](../../src/domain/world/types.rs)                         | No `facing` field                                                                               |
| `MapEvent::NpcDialogue`          | [src/domain/world/types.rs](../../src/domain/world/types.rs)                         | No `facing` field                                                                               |
| `MapEvent::Encounter`            | [src/domain/world/types.rs](../../src/domain/world/types.rs)                         | No `facing` field                                                                               |
| `MapEvent::RecruitableCharacter` | [src/domain/world/types.rs](../../src/domain/world/types.rs)                         | No `facing` field                                                                               |

### Identified Issues

1. `spawn_creature()` has no rotation parameter; all creatures spawn facing the same direction
2. `NpcPlacement.facing` is already persisted in RON files but never read at spawn time
3. `MapEvent` variants that own a creature visual (`Sign`, `Encounter`, `RecruitableCharacter`) have no `facing` field — author cannot set direction in the map RON
4. No runtime system exists to change a spawned creature's facing after map load
5. No `FacingComponent` on spawned entities prevents runtime queries from knowing or changing an entity's cardinal direction
6. No `direction_to_yaw_radians()` conversion helper exists in the domain layer — each future call site would invent its own angle mapping

---

## Implementation Phases

### Phase 1: Direction-to-Rotation Foundation

Add the `direction_to_yaw_radians` conversion helper and a `FacingComponent`
ECS component. These are pure additions with no behaviour change.

#### 1.1 Add `direction_to_yaw_radians` to `Direction`

Add an impl method on `Direction` in [src/domain/types.rs](../../src/domain/types.rs):

- `Direction::North` → `0.0` rad
- `Direction::East` → `std::f32::consts::FRAC_PI_2` (90°)
- `Direction::South` → `std::f32::consts::PI` (180°)
- `Direction::West` → `3.0 * std::f32::consts::FRAC_PI_2` (270°)

Also add the inverse: `Direction::from_yaw_radians(yaw: f32) -> Direction` —
rounds to the nearest 90° cardinal.

#### 1.2 Add `FacingComponent` ECS Component

Add to [src/game/components/creature.rs](../../src/game/components/creature.rs):

```
pub struct FacingComponent {
    pub direction: Direction,
}
```

This component is attached to every spawned creature/NPC/sign entity. It is
the authoritative runtime facing state for that entity.

#### 1.3 Add Rotation Parameter to `spawn_creature()`

Change the signature of `spawn_creature` in
[src/game/systems/creature_spawning.rs](../../src/game/systems/creature_spawning.rs)
to accept `facing: Option<Direction>`:

- Compute `Quat::from_rotation_y(facing.map_or(0.0, |d| d.direction_to_yaw_radians()))` and apply it to the parent `Transform` via `.with_rotation(rotation)`
- Insert `FacingComponent { direction: facing.unwrap_or(Direction::North) }` on the parent entity

All existing call sites pass `None` for the new parameter, preserving
current behaviour.

#### 1.4 Testing Requirements

- `test_direction_to_yaw_north` — `Direction::North.direction_to_yaw_radians() == 0.0`
- `test_direction_to_yaw_east` — `Direction::East.direction_to_yaw_radians() ≈ FRAC_PI_2`
- `test_direction_roundtrip` — for each `Direction`, assert `Direction::from_yaw_radians(d.direction_to_yaw_radians()) == d`
- `test_spawn_creature_facing_south` — call `spawn_creature` with `Some(Direction::South)`, assert parent `Transform.rotation` ≈ `Quat::from_rotation_y(PI)`
- `test_facing_component_inserted` — same spawn, assert `FacingComponent.direction == Direction::South`

#### 1.5 Deliverables

- [x] `direction_to_yaw_radians()` method on `Direction`
- [x] `Direction::from_yaw_radians()` method
- [x] `FacingComponent` component in `creature.rs`
- [x] `spawn_creature()` accepts `facing: Option<Direction>`; existing callers pass `None`
- [x] Unit tests passing

#### 1.6 Success Criteria

`cargo check --all-targets --all-features` produces zero errors. All
existing call sites compile unchanged with `None`.

---

### Phase 2: Static Map-Time Facing

Wire `facing` from all existing domain sources through to spawn time, and
add `facing` fields to the `MapEvent` variants that currently lack them.

#### 2.1 Wire `NpcPlacement.facing` at Spawn Time

In [src/game/systems/map.rs](../../src/game/systems/map.rs) at the NPC
spawn block (~L1025–1100), pass `resolved_npc.facing` to `spawn_creature`:

```
spawn_creature(..., resolved_npc.facing, ...)
```

For the sprite fallback path, apply
`Quat::from_rotation_y(facing.map_or(0.0, |d| d.direction_to_yaw_radians()))`
to the sprite entity's `Transform`.

The tutorial map RON files in `data/maps/` and `campaigns/tutorial/data/maps/`
already support `facing:` on NPC placement entries — no RON schema change needed.

#### 2.2 Add `facing` to `MapEvent` Variants

In [src/domain/world/types.rs](../../src/domain/world/types.rs), add:

```ron
#[serde(default)]
facing: Option<Direction>,
```

to the following variants: `Sign`, `NpcDialogue`, `Encounter`,
`RecruitableCharacter`. Use `#[serde(default)]` so existing RON files
with no `facing` field remain valid (defaults to `None` → identity rotation).

Map authors can then write:

```ron
RecruitableCharacter(
    name: "Old Gareth",
    character_id: "npc_old_gareth",
    dialogue_id: 100,
    facing: Some(South),
),
```

#### 2.3 Wire Event `facing` at Spawn Time

In [src/game/systems/map.rs](../../src/game/systems/map.rs) at the event
iteration block (~L1105):

- `MapEvent::Encounter { facing, .. }` → pass `facing` to `spawn_creature`
- `MapEvent::RecruitableCharacter { facing, .. }` → pass `facing` to `spawn_creature`
- `MapEvent::Sign { facing, .. }` → `spawn_sign()` in
  [src/game/systems/procedural_meshes.rs](../../src/game/systems/procedural_meshes.rs)
  must accept `facing: Option<Direction>` and convert to rotation. Also
  attach `FacingComponent` to sign entities.
- `MapEvent::NpcDialogue { facing, .. }` → apply facing when the NPC is
  spawned via the `resolve_npcs` path (override NPC placement facing with
  event-level facing if `Some`); this handles cases where the same NPC
  definition is placed differently per event.

#### 2.4 Update Tutorial Map Data

Update `campaigns/tutorial/data/maps/` map RON files to add `facing:` on
at least one NPC placement and one `RecruitableCharacter` event as a
functional smoke-test for the new feature.

#### 2.5 Testing Requirements

- `test_npc_facing_applied_at_spawn` — build a test map with one NPC
  placement where `facing: Some(East)`, run the spawn system, query the NPC
  entity `Transform.rotation`, assert it ≈ `Quat::from_rotation_y(FRAC_PI_2)`
- `test_map_event_encounter_facing` — map with `Encounter { facing: Some(West), .. }`,
  assert monster entity has the correct rotation
- `test_map_event_sign_facing` — sign with `facing: Some(South)`, assert
  sign entity `Transform` has `PI` yaw rotation
- `test_map_event_ron_round_trip` — serialize a `MapEvent::RecruitableCharacter`
  with `facing: Some(North)` to RON and parse back, assert equality
- `test_facing_component_on_npc` — after spawn, query `FacingComponent`
  on NPC entity, assert `direction == East`

#### 2.6 Deliverables

- [x] `facing: Option<Direction>` added (with `#[serde(default)]`) to
      `MapEvent::Sign`, `NpcDialogue`, `Encounter`, `RecruitableCharacter`
- [x] NPC creature spawn path passes `resolved_npc.facing`
- [x] NPC sprite fallback path applies rotation from `facing`
- [x] `spawn_sign()` accepts and applies `facing`
- [x] Encounter and `RecruitableCharacter` spawn paths pass event `facing`
- [x] At least one tutorial map updated with an explicit `facing` value
- [x] All tests passing

#### 2.7 Success Criteria

Map authors can set `facing:` in the map RON on any NPC placement or
creature event. The visual immediately reflects the direction after map load
with no runtime errors.

---

### Phase 3: Runtime Facing Change System

Enable game events (e.g., a monster turning toward the player, an NPC
turning to face the party after starting dialogue) to change a spawned
entity's facing at runtime.

#### 3.1 Add `SetFacing` Bevy Message

Add to [src/game/systems/map.rs](../../src/game/systems/map.rs) (or a
dedicated `src/game/systems/facing.rs` module):

```rust
#[derive(Message)]
pub struct SetFacing {
    pub entity: Entity,
    pub direction: Direction,
    pub instant: bool,   // true = snap, false = smooth (Phase 4)
}
```

Register the message in the owning plugin.

#### 3.2 Add `handle_set_facing` System

Add to the same module a system `handle_set_facing` that reads
`MessageReader<SetFacing>` and for each event:

1. Queries `(&mut Transform, &mut FacingComponent)` on the target entity
2. Computes `Quat::from_rotation_y(direction.direction_to_yaw_radians())`
3. Sets `transform.rotation = target_quat` (snap) when `instant == true`
4. Updates `FacingComponent.direction`

Phase 4 will handle the smooth path; for now `instant == false` is treated
identically to `instant == true`.

#### 3.3 Add `FaceToward` System for Proximity Triggers

Add a `face_toward_player_on_proximity` system:

- Queries all entities with `FacingComponent` that also carry a
  `ProximityFacing` marker component (defined below)
- For each such entity, computes the 4-direction vector from the entity's
  `TileCoord` to `GlobalState::party_position`
- Emits a `SetFacing` event if the closest cardinal differs from current
  `FacingComponent.direction`

Add marker component:

```rust
#[derive(Component)]
pub struct ProximityFacing {
    pub trigger_distance: u32,   // tile distance to activate (e.g., 2)
}
```

`ProximityFacing` is never serialised; it is inserted at spawn time by the
map loading system only when a future RON flag is set. This flag is added
in Phase 3.4 below.

#### 3.4 Add `proximity_facing` RON Flag

Add `#[serde(default)] proximity_facing: bool` to `MapEvent::Encounter`
and `MapEvent::NpcDialogue`. When `true`, the map loading system inserts
`ProximityFacing { trigger_distance: 2 }` on the spawned entity.

#### 3.5 Emit `SetFacing` from Dialogue System

In [src/game/systems/dialogue.rs](../../src/game/systems/dialogue.rs),
when a dialogue starts (`handle_start_dialogue`):

- If the speaker entity exists and has `FacingComponent`
- Determine the 4-direction from the speaker to the party position
- Write a `SetFacing { entity: speaker_entity, direction, instant: true }`

This produces the natural behaviour of an NPC turning to face the player
when spoken to.

#### 3.6 Testing Requirements

- `test_set_facing_snaps_transform` — spawn an entity with `FacingComponent`,
  write a `SetFacing { instant: true, direction: West }`, run the system,
  assert `Transform.rotation ≈ Quat::from_rotation_y(3*FRAC_PI_2)`
- `test_set_facing_updates_facing_component` — same as above, assert
  `FacingComponent.direction == Direction::West`
- `test_proximity_facing_emits_event` — place entity with `ProximityFacing { trigger_distance: 2 }`
  at tile `(5,5)`, set party at `(5,7)`, run system, assert `SetFacing`
  event emitted with `Direction::South`
- `test_dialogue_start_triggers_face_toward_party` — start a dialogue
  with a speaker at `(3,3)`, party at `(5,3)`, assert npc faces `East`

#### 3.7 Deliverables

- [x] `SetFacing` message and `handle_set_facing` system
- [x] `ProximityFacing` component
- [x] `face_toward_player_on_proximity` system
- [x] `proximity_facing` RON flag on `MapEvent::Encounter` and `NpcDialogue`
- [x] `SetFacing` emitted from `handle_start_dialogue` for NPC speaker
- [x] All tests passing

#### 3.8 Success Criteria

Sending a `SetFacing` event immediately rotates the target creature. NPCs
with `proximity_facing: true` turn toward the party when within range.
Dialogue speakers always face the player during conversation.

---

### Phase 4: Smooth Rotation Animation

Replace the snap rotation with a frame-by-frame slerp when `instant == false`.

#### 4.1 Add `RotatingToFacing` Component

```rust
#[derive(Component)]
pub struct RotatingToFacing {
    pub target: Quat,
    pub speed_deg_per_sec: f32,   // default 360.0 (one full rotation per second)
}
```

Insert this component instead of directly setting `Transform.rotation` when
`SetFacing.instant == false`.

#### 4.2 Add `apply_rotation_to_facing` System

A per-frame system queries `(&mut Transform, &mut RotatingToFacing,
&mut FacingComponent)`:

- Uses `Quat::slerp(current, target, t)` where `t = speed * delta_secs / angle_between`
- When the rotation is within 0.01 rad of target: sets exact target rotation,
  updates `FacingComponent.direction`, removes `RotatingToFacing` component

#### 4.3 Add `rotation_speed` RON Field

Add `#[serde(default)] rotation_speed: Option<f32>` to `MapEvent::Encounter`
and `MapEvent::NpcDialogue`. When set in the RON, the value is passed to
`ProximityFacing` and used as the `speed_deg_per_sec` when emitting `SetFacing`.

Default: `None` → treated as `instant: true` (snap).

#### 4.4 Testing Requirements

- `test_rotating_to_facing_approaches_target` — insert `RotatingToFacing`
  with a 90° delta and `speed = 360`, run 0.1s of delta time, assert
  rotation moved ~36° toward target
- `test_rotating_to_facing_completes_and_removes_component` — run enough
  delta time to overshoot target, assert component is removed and final
  rotation equals exact target
- `test_set_facing_instant_false_inserts_rotating_component` — write
  `SetFacing { instant: false }`, assert `RotatingToFacing` is inserted
  instead of directly mutating `Transform`

#### 4.5 Deliverables

- [x] `RotatingToFacing` component
- [x] `apply_rotation_to_facing` slerp system
- [x] `rotation_speed` RON field on `Encounter` and `NpcDialogue`
- [x] `SetFacing.instant == false` path inserts `RotatingToFacing`
- [x] All tests passing

#### 4.6 Success Criteria

Proximity-triggered monsters rotate smoothly toward the player at a
configurable speed instead of instantly snapping. All snap paths remain
unchanged and performant.

---

---

## Phase 5: Campaign Builder SDK UI

Exposes all new domain fields introduced in Phases 2–4 as editable controls
in the Campaign Builder map editor, so campaign authors never need to hand-edit
RON files to set facing directions, proximity behaviour, or rotation speed.

---

### 5.1 Add `facing` to `EventEditorState`

In `sdk/campaign_builder/src/map_editor.rs`, add a new field to
`EventEditorState` alongside the existing per-event-type fields:

```rust
/// Facing direction for the spawned entity (Sign, NpcDialogue, Encounter,
/// RecruitableCharacter). `None` means the engine default (North).
pub event_facing: Option<String>,
```

Initialise it to `None` in `Default for EventEditorState`.

---

### 5.2 Add Facing Combo-Box to Affected Event Editor Panels

The four `MapEvent` variants that gain `facing` in Phase 2 are `Sign`,
`NpcDialogue`, `Encounter`, and `RecruitableCharacter`. In `show_event_editor()`
in `sdk/campaign_builder/src/map_editor.rs`, add the following block at the
**bottom** of each of those four `match` arms, immediately before the arm
closes — using the same pattern as the existing `facing` combo-box in
`show_npc_placement_editor()`:

```rust
// Facing direction
ui.horizontal(|ui| {
    ui.label("Facing:")
        .on_hover_text("Initial facing direction of the spawned entity");
    egui::ComboBox::from_id_salt("event_facing_combo")
        .selected_text(
            event_editor.event_facing.as_deref().unwrap_or("None")
        )
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut event_editor.event_facing, None, "None");
            for dir in &["North", "East", "South", "West"] {
                ui.selectable_value(
                    &mut event_editor.event_facing,
                    Some((*dir).to_string()),
                    *dir,
                );
            }
        });
    if event_editor.event_facing.is_some() {
        if ui.small_button("✖ Clear").clicked() {
            event_editor.event_facing = None;
            editor.has_changes = true;
        }
    }
});
if event_editor.event_facing.is_some() {
    editor.has_changes = true;
}
```

Each arm uses a **unique `id_salt`** to satisfy the egui ID rules:

| Event type             | `id_salt`                           |
| ---------------------- | ----------------------------------- |
| `Sign`                 | `"sign_event_facing_combo"`         |
| `NpcDialogue`          | `"npc_dialogue_event_facing_combo"` |
| `Encounter`            | `"encounter_event_facing_combo"`    |
| `RecruitableCharacter` | `"recruitable_event_facing_combo"`  |

---

### 5.3 Update `to_map_event()` to Forward `event_facing`

In `impl EventEditorState`, update the four relevant arms of `to_map_event()`
to parse `event_facing` into `Option<Direction>` and include it in the
constructed `MapEvent`:

```rust
let facing = event_editor.event_facing.as_deref().and_then(|f| match f {
    "North" => Some(Direction::North),
    "South" => Some(Direction::South),
    "East"  => Some(Direction::East),
    "West"  => Some(Direction::West),
    _       => None,
});

// Then pass `facing` into the MapEvent variant, e.g.:
MapEvent::Sign {
    name: self.name.clone(),
    description: self.description.clone(),
    text: self.sign_text.clone(),
    facing,
}
```

Apply the same pattern for `NpcDialogue`, `Encounter`, and
`RecruitableCharacter`.

---

### 5.4 Update `from_map_event()` to Read `event_facing`

In `impl EventEditorState`, update the four relevant arms of `from_map_event()`
to populate `event_facing` from the loaded event:

```rust
MapEvent::Sign { facing, .. } => {
    s.event_type = EventType::Sign;
    // ... existing fields ...
    s.event_facing = facing.map(|d| format!("{:?}", d));
}
```

Apply the same pattern for `NpcDialogue`, `Encounter`, and
`RecruitableCharacter`. When the loaded event's `facing` field is `None`
(as it will be for all pre-existing RON files), `event_facing` is set to
`None` and the combo-box shows "None" — preserving backward compatibility.

---

### 5.5 Add `proximity_facing` and `rotation_speed` Fields to `EventEditorState`

Add two more fields, covering the Phase 3 and Phase 4 RON flags that apply
to `Encounter` and `NpcDialogue` events:

```rust
/// Insert `ProximityFacing` component on the spawned entity so it turns
/// toward the party when the party is within range.
/// Applies to Encounter and NpcDialogue events only.
pub event_proximity_facing: bool,

/// Rotation speed in degrees per second for smooth facing transitions.
/// `None` means snap (instant). Applies to Encounter and NpcDialogue only.
pub event_rotation_speed: Option<f32>,
```

Initialise both to their "off" defaults in `Default for EventEditorState`:

```rust
event_proximity_facing: false,
event_rotation_speed: None,
```

---

### 5.6 Add Proximity Facing and Rotation Speed UI to Encounter and NpcDialogue Panels

In `show_event_editor()`, within the `EventType::Encounter` and
`EventType::NpcDialogue` arms only, add the following block **after** the
facing combo-box from §5.2:

```rust
ui.separator();
ui.label("🔄 Behaviour:");

// Proximity facing toggle
if ui
    .checkbox(&mut event_editor.event_proximity_facing, "Turn to face party on proximity")
    .on_hover_text(
        "When enabled the entity automatically turns toward the party \
         when they step within 2 tiles."
    )
    .changed()
{
    editor.has_changes = true;
}

// Rotation speed (only meaningful when proximity_facing is on)
if event_editor.event_proximity_facing {
    ui.horizontal(|ui| {
        ui.label("Rotation Speed (deg/s):")
            .on_hover_text("Leave empty for instant snap. Enter a value for smooth rotation.");
        let mut speed_str = event_editor
            .event_rotation_speed
            .map(|s| s.to_string())
            .unwrap_or_default();
        if ui.text_edit_singleline(&mut speed_str).changed() {
            event_editor.event_rotation_speed = speed_str.parse::<f32>().ok();
            editor.has_changes = true;
        }
        if event_editor.event_rotation_speed.is_some()
            && ui.small_button("✖ Snap").clicked()
        {
            event_editor.event_rotation_speed = None;
            editor.has_changes = true;
        }
    });
}
```

---

### 5.7 Update `to_map_event()` and `from_map_event()` for Phase 3–4 Fields

**`to_map_event()`** — for `Encounter` and `NpcDialogue`:

```rust
MapEvent::Encounter {
    name: self.name.clone(),
    description: self.description.clone(),
    monster_group: monsters,
    facing,
    proximity_facing: self.event_proximity_facing,
    rotation_speed: self.event_rotation_speed,
    // ...
}
```

**`from_map_event()`** — for `Encounter` and `NpcDialogue`:

```rust
MapEvent::Encounter {
    facing,
    proximity_facing,
    rotation_speed,
    ..
} => {
    // ... existing fields ...
    s.event_facing = facing.map(|d| format!("{:?}", d));
    s.event_proximity_facing = *proximity_facing;
    s.event_rotation_speed = *rotation_speed;
}
```

---

### 5.8 Display Facing and Behaviour in the Inspector Panel

In `show_inspector_panel()` in `sdk/campaign_builder/src/map_editor.rs`,
extend the `MapEvent::Sign`, `MapEvent::NpcDialogue`, `MapEvent::Encounter`,
and `MapEvent::RecruitableCharacter` display branches to show the facing
and behaviour flags:

```rust
if let Some(dir) = facing {
    ui.label(format!("Facing: {:?}", dir));
}
```

For `Encounter` and `NpcDialogue`, also show:

```rust
if *proximity_facing {
    ui.label("🔄 Turns toward party on proximity");
    if let Some(speed) = rotation_speed {
        ui.label(format!("  Rotation speed: {:.0} deg/s", speed));
    } else {
        ui.label("  Rotation: instant snap");
    }
}
```

---

### 5.9 Testing Requirements

**`EventEditorState` unit tests** (in `mod tests` of `map_editor.rs`):

- `test_event_editor_state_default_facing_none` — `EventEditorState::default()`
  has `event_facing == None`, `event_proximity_facing == false`,
  `event_rotation_speed == None`.
- `test_event_editor_to_sign_with_facing` — set `event_type = Sign`,
  `event_facing = Some("East".to_string())`; call `to_map_event()`; assert
  result is `MapEvent::Sign { facing: Some(Direction::East), .. }`.
- `test_event_editor_from_sign_with_facing` — call `from_map_event()` with
  `MapEvent::Sign { facing: Some(Direction::West), .. }`; assert
  `event_facing == Some("West".to_string())`.
- `test_event_editor_from_sign_no_facing` — call `from_map_event()` with a
  `MapEvent::Sign` whose `facing` is `None`; assert `event_facing == None`.
- `test_event_editor_to_encounter_with_facing_and_proximity` — set
  `event_facing = Some("South".to_string())`, `event_proximity_facing = true`,
  `event_rotation_speed = Some(180.0)`; call `to_map_event()`; assert the
  resulting `MapEvent::Encounter` has all three fields set correctly.
- `test_event_editor_from_encounter_with_proximity` — round-trip an
  `Encounter` with `proximity_facing: true, rotation_speed: Some(90.0)`;
  assert the buffer fields survive.
- `test_event_editor_facing_round_trip_all_variants` — for each of the four
  `Direction` values and each of the four affected event types, assert that
  `from_map_event(to_map_event())` is a lossless round-trip.
- `test_event_editor_proximity_false_clears_rotation_speed_in_ui` — a
  documentation test confirming that when `event_proximity_facing` is `false`
  the rotation-speed field is not forwarded to `to_map_event()` (i.e., it
  defaults to `None` in the output even when the buffer holds a value).

---

### 5.10 Deliverables

- [ ] `event_facing: Option<String>` field on `EventEditorState`
- [ ] `event_proximity_facing: bool` and `event_rotation_speed: Option<f32>`
      fields on `EventEditorState`
- [ ] `Default` initialises all three fields to their "off" values
- [ ] **Facing** combo-box visible in the editor for `Sign`, `NpcDialogue`,
      `Encounter`, and `RecruitableCharacter` event types
- [ ] **Proximity Facing** checkbox and **Rotation Speed** input visible for
      `Encounter` and `NpcDialogue` only
- [ ] `to_map_event()` forwards all three fields for all relevant variants
- [ ] `from_map_event()` reads all three fields for all relevant variants
- [ ] Inspector panel displays facing and behaviour flags for all four types
- [ ] Unique `id_salt` used for every combo-box (no egui ID clashes)
- [ ] All eight new tests pass; all four quality gates pass with zero warnings

### 5.11 Success Criteria

A campaign author can open the map editor, select any `Sign`, `NpcDialogue`,
`Encounter`, or `RecruitableCharacter` event, choose "South" from the Facing
combo-box, tick "Turn to face party on proximity" (for Encounter / NpcDialogue),
enter `180` as the rotation speed, save the map, and confirm the saved RON
contains `facing: Some(South)`, `proximity_facing: true`, and
`rotation_speed: Some(180.0)`. Loading the same map back into the editor
repopulates all three controls correctly. All four `cargo` quality gates pass
with zero warnings.

---

## Architecture Compliance Notes

- `direction_to_yaw_radians` is the single source of truth for the angle
  mapping; no other file redefines north/south/etc as raw floats
- `MapEvent` field additions use `#[serde(default)]` — all existing RON
  files remain valid without migration
- `SetFacing` follows the existing `#[derive(Message)]` broadcast pattern
- `RotatingToFacing` is a pure ECS scratch component — never serialised,
  never in domain structs
- No new `.rs` files are absolutely required; `facing.rs` is permitted if
  `map.rs` becomes too large but is not mandatory
- **SDK changes ARE required**: Phase 2 adds `facing` to four `MapEvent`
  variants; Phases 3–4 add `proximity_facing` and `rotation_speed` to two
  variants. All must be surfaced in the Campaign Builder map editor (Phase 5)
  so campaign authors can configure them without editing RON by hand.
