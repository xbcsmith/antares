# Combat System Implementation Plan

## Overview

Implement a turn-based combat system for Antares using Bevy's ECS architecture. The system will integrate with the existing domain layer (`src/domain/combat/`) and provide a complete gameplay loop from encounter trigger through resolution and rewards.

Core Bevy patterns:
- **State Management**: Combat sub-states via `GameMode::Combat(CombatState)`
- **Components**: ECS markers for combatants, UI elements, turn indicators
- **Systems**: Query-based logic for turn execution, damage, AI
- **Events**: `bevy_mod_messages` for decoupled action handling
- **Plugin Architecture**: Modular `CombatPlugin` with sub-plugins

## Current State Analysis

### Existing Infrastructure

| Layer | Location | Status |
|-------|----------|--------|
| Domain types | `src/domain/combat/` | ✓ Complete (`CombatState`, `Monster`, `Attack`, turn order) |
| Combat logic | `src/domain/combat/engine.rs` | ✓ Complete (resolve_attack, apply_damage, conditions) |
| Monster database | `src/domain/combat/database.rs` | ✓ Complete (load from `monsters.ron`) |
| Application layer | `src/application/mod.rs` | ✓ Partial (`enter_combat()`, `exit_combat()`, `start_encounter()`) |
| Bevy integration | `src/game/systems/` | ✗ Missing (no `combat.rs` or combat UI) |

### Identified Issues

1. **No Combat UI** — No Bevy systems render combat state or handle player input
2. **No Player Action System** — No mechanism to select targets or actions
3. **No Monster AI** — `choose_monster_attack()` exists but no automated execution
4. **No Party Sync** — Party members not copied to/from `CombatState`
5. **Encounter Trigger Stub** — `events.rs:164` has `// TODO: Start combat`
6. **No Rewards Distribution** — `LootTable` defined but unused

---

## Implementation Phases

### Phase 1: Core Combat Infrastructure

Build the foundational plugin, state management, and party synchronization.

#### 1.1 Foundation Work

Create `src/game/systems/combat.rs`:

- Define `CombatPlugin` struct implementing `Plugin`
- Register combat-related messages (`AttackAction`, `DefendAction`, `FleeAction`, etc.)
- Add `CombatTurnState` Bevy sub-states: `PlayerTurn`, `EnemyTurn`, `Animating`, `RoundEnd`
- Create `CombatResource` Bevy resource wrapping `domain::combat::CombatState`

Create `src/game/components/combat.rs`:

- `CombatantMarker { combatant_id: CombatantId }` — links ECS entities to domain combatants
- `TurnIndicator` — marker for current actor in turn order
- `TargetSelector` — marker for target selection UI
- `CombatHudRoot` — root container for combat UI

#### 1.2 Party ↔ Combat State Synchronization

Add `sync_party_to_combat` system:
- Copy `GlobalState.party.members` into `CombatState.participants` as `Combatant::Player`
- Track original indices for post-combat sync

Add `sync_combat_to_party` system (runs on combat exit):
- Copy HP, conditions, resources from `CombatState` back to party members

#### 1.3 Integrate Foundation Work

Wire up `CombatPlugin` in [antares.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/bin/antares.rs):
- Add `.add_plugins(CombatPlugin)` after existing game plugins

Connect encounter trigger in [events.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/events.rs):
- Replace `// TODO: Start combat` with `start_encounter()` call
- Emit `CombatStarted` message to notify other systems

#### 1.4 Testing Requirements

- **Unit tests** in `src/game/systems/combat.rs`:
  - `test_combat_plugin_registers_messages`
  - `test_party_sync_to_combat`
  - `test_combat_sync_to_party`

- **Integration test** in `tests/combat_integration.rs`:
  - Verify `MapEvent::Encounter` triggers combat mode transition

Run tests with:
```bash
cargo test combat
```

#### 1.5 Deliverables

- [ ] `src/game/systems/combat.rs` with `CombatPlugin`
- [ ] `src/game/components/combat.rs` with marker components
- [ ] Party sync systems
- [ ] Encounter trigger wired in `events.rs`
- [ ] Unit tests passing

#### 1.6 Success Criteria

- `cargo test combat` passes
- Stepping on `MapEvent::Encounter` tile switches `GameMode` to `Combat`
- Party members visible in `CombatState.participants`

---

### Phase 2: Combat UI System

Display combat state to the player.

#### 2.1 Feature Work

Create combat HUD layout (following [hud.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/game/systems/hud.rs) patterns):

- **Enemy Panel** (top) — Monster names, HP bars, conditions
- **Party Panel** (bottom) — Already exists in HUD, enhance with combat indicators
- **Turn Order Display** — Show initiative order with current actor highlighted
- **Action Menu** — Attack/Defend/Cast/Item/Flee buttons (visible on player turn)

Add `setup_combat_ui` system (runs on combat enter):
- Spawn combat-specific UI entities
- Hide exploration HUD elements

Add `cleanup_combat_ui` system (runs on combat exit):
- Despawn combat UI entities
- Restore exploration HUD

Add `update_combat_ui` system (runs every frame during combat):
- Sync HP bars with `CombatState`
- Update turn indicator position
- Show/hide action menu based on turn state

#### 2.2 Integrate Feature

Add run conditions to existing HUD systems:
- Skip exploration HUD updates when in Combat mode

#### 2.3 Configuration Updates

Add constants to `combat.rs`:
- `COMBAT_PANEL_HEIGHT`, `ENEMY_HP_BAR_WIDTH`, etc.

#### 2.4 Testing Requirements

- **Unit tests**:
  - `test_combat_ui_spawns_on_enter`
  - `test_combat_ui_despawns_on_exit`
  - `test_enemy_hp_bars_update`

- **Manual verification**:
  - Enter combat via encounter tile
  - Verify enemy panel shows monster names and HP
  - Verify turn order indicator highlights current actor

#### 2.5 Deliverables

- [ ] Combat UI layout with enemy panel
- [ ] Turn order display
- [ ] Action menu (buttons)
- [ ] HP bar sync system
- [ ] Unit tests passing

#### 2.6 Success Criteria

- Combat UI visible when entering combat
- UI correctly reflects `CombatState` data
- Clean transition back to exploration UI on exit

---

### Phase 3: Player Action System

Allow players to take actions during their turn.

#### 3.1 Feature Work

Define action messages in `combat.rs`:
```rust
#[derive(Message)]
pub struct AttackAction { pub attacker: CombatantId, pub target: CombatantId }

#[derive(Message)]
pub struct DefendAction { pub combatant: CombatantId }

#[derive(Message)]
pub struct FleeAction;
```

Add `combat_input_system`:
- Read keyboard/mouse input during `PlayerTurn` state
- Emit appropriate action messages
- Handle target selection flow

Add `handle_attack_action` system:
- Read `AttackAction` messages
- Call `domain::combat::resolve_attack()`
- Apply damage via `domain::combat::apply_damage()`
- Check combat end conditions
- Advance turn

Add `handle_flee_action` system:
- Calculate flee success (based on Handicap, speed differential)
- On success: transition to `CombatStatus::Fled`, exit combat
- On failure: consume player turn

#### 3.2 Target Selection UI

Add `enter_target_selection` system:
- Highlight valid targets
- Show selection cursor

Add `select_target` system:
- Handle click/keyboard to select target
- Emit `AttackAction` with selected target

#### 3.3 Testing Requirements

- **Unit tests**:
  - `test_attack_action_applies_damage`
  - `test_defend_action_improves_ac`
  - `test_flee_success_exits_combat`
  - `test_flee_failure_consumes_turn`

- **Integration test**:
  - Full attack sequence player → monster

Run tests with:
```bash
cargo test combat_action
```

#### 3.4 Deliverables

- [ ] Action message types
- [ ] `combat_input_system`
- [ ] Attack/Defend/Flee handlers
- [ ] Target selection UI
- [ ] Unit tests passing

#### 3.5 Success Criteria

- Player can select and attack a monster
- Damage correctly applied to target
- Turn advances after action
- Flee attempts work correctly

---

### Phase 4: Monster AI System

Automate monster turns.

#### 4.1 Feature Work

Add `execute_monster_turn` system:
- Runs when `CombatTurnState::EnemyTurn` and current combatant is a monster
- Select attack via `choose_monster_attack()`
- Select target (random living player, or lowest HP)
- Execute attack using domain functions
- Advance turn

Add AI behaviors:
- **Aggressive**: Always attack lowest HP target
- **Defensive**: Focus attacks on highest threat
- **Random**: Random target selection (default)

#### 4.2 Turn Flow Integration

Add turn advancement loop:
- After each action, check if next combatant is player or monster
- Transition to appropriate `CombatTurnState`
- Monster turns execute automatically (with optional delay for animation)

#### 4.3 Testing Requirements

- **Unit tests**:
  - `test_monster_ai_selects_target`
  - `test_monster_turn_advances_after_attack`
  - `test_monster_attacks_lowest_hp_target`

- **Integration test**:
  - Full combat round (all combatants act)

#### 4.4 Deliverables

- [ ] `execute_monster_turn` system
- [ ] Target selection AI
- [ ] Turn flow state machine
- [ ] Unit tests passing

#### 4.5 Success Criteria

- Monster turns execute automatically
- Damage applied to party members
- Combat continues through multiple rounds

---

### Phase 5: Combat Resolution & Rewards

Handle victory, defeat, and rewards.

#### 5.1 Feature Work

Add `check_combat_resolution` system:
- Monitor `CombatState.status`
- On `Victory`: emit `CombatVictory` message
- On `Defeat`: emit `CombatDefeat` message

Add `handle_combat_victory` system:
- Calculate total XP from monster loot tables
- Distribute XP to living party members
- Calculate gold/gem drops
- Add to party inventory
- Show victory summary UI
- Transition to exploration mode

Add `handle_combat_defeat` system:
- Show defeat message
- Handle game over / reload last save

#### 5.2 Victory Summary UI

Display post-combat results:
- XP gained per character
- Gold/gems collected
- Any items dropped

#### 5.3 Testing Requirements

- **Unit tests**:
  - `test_victory_distributes_xp`
  - `test_victory_awards_gold`
  - `test_defeat_triggers_game_over`

- **Integration test**:
  - Full combat to victory, verify party XP increased

#### 5.4 Deliverables

- [ ] Combat resolution detection
- [ ] XP distribution system
- [ ] Loot collection system
- [ ] Victory/defeat UI
- [ ] Unit tests passing

#### 5.5 Success Criteria

- Killing all monsters triggers victory
- Party receives XP and gold
- Clean transition back to exploration

---

### Phase 6: Encounter Triggers & Polish

Integrate random encounters and polish the system.

#### 6.1 Feature Work

Add random encounter system:
- Track steps in exploration
- Check encounter table for current map/terrain
- Roll for encounter based on encounter rate
- Spawn appropriate monster group

Add encounter configuration:
- Per-map encounter tables
- Terrain-based modifiers
- Safe zones (towns, inns)

#### 6.2 Polish

- Add combat animations (damage numbers, hit flash)
- Add sound effects for attacks, hits, misses
- Add combat music transition
- Improve UI visual feedback

#### 6.3 Testing Requirements

- **Unit tests**:
  - `test_random_encounter_triggers`
  - `test_safe_zones_prevent_encounters`

- **Manual verification**:
  - Walk around map, verify random encounters occur
  - Verify combat feels responsive and polished

#### 6.4 Deliverables

- [ ] Random encounter system
- [ ] Encounter configuration
- [ ] Combat animations
- [ ] Audio integration
- [ ] Tests passing

#### 6.5 Success Criteria

- Random encounters occur while exploring
- Combat feels responsive and visually polished
- Audio feedback enhances experience

---

## File Summary

| Phase | New Files | Modified Files |
|-------|-----------|----------------|
| 1 | `src/game/systems/combat.rs`, `src/game/components/combat.rs` | `src/game/systems/mod.rs`, `src/game/components.rs`, `src/bin/antares.rs`, `src/game/systems/events.rs` |
| 2 | — | `src/game/systems/combat.rs`, `src/game/systems/hud.rs` |
| 3 | — | `src/game/systems/combat.rs` |
| 4 | — | `src/game/systems/combat.rs` |
| 5 | — | `src/game/systems/combat.rs`, `src/application/mod.rs` |
| 6 | — | `src/game/systems/combat.rs`, `src/game/systems/map.rs` |

## Design Decisions

Decisions made based on project requirements:

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Animation System** | Sprite-based (see [sprite_support_implementation_plan.md](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/sprite_support_implementation_plan.md)) | Combat will use billboard sprites with `SpriteAnimation` for attack/hit effects. Sprite plan provides: Billboard system with Y-axis lock, `SpriteReference`/`SpriteAnimation` types, frame-based animation at configurable FPS. Combat animations will reuse this infrastructure. |
| **Combat Positioning** | Classic first-person view | Monsters displayed in a row facing the party (Might & Magic 1 style). No spatial grid. Simpler implementation, authentic retro feel. |
| **Spell Casting** | Separate `CastSpellAction` message | Decoupled from attack flow. Allows independent validation (MP check, target validation, spell cooldowns). Message: `CastSpellAction { caster: CombatantId, spell_id: SpellId, target: CombatantId }` |

### Dependency on Sprite Plan

Combat system implementation should begin **after** sprite support Phases 1-3 are complete, as combat relies on:

- `SpriteReference` / `SpriteAnimation` types (Phase 1)
- Billboard component and rendering system (Phase 3)
- Texture atlas infrastructure (Phase 2)

### First-Person View Layout

```text
┌─────────────────────────────────────────────┐
│                                             │
│     [Monster 1]  [Monster 2]  [Monster 3]   │  ← Billboard sprites facing camera
│                                             │
│  ─────────────────────────────────────────  │
│                                             │
│  [Turn Order: Knight → Goblin → Cleric]     │  ← Initiative display
│                                             │
│  ┌─────────────────────────────────────┐    │
│  │ Attack │ Defend │ Cast │ Item │ Flee│    │  ← Action menu (player turn only)
│  └─────────────────────────────────────┘    │
│                                             │
│  [Party HUD - existing HUD at bottom]       │
└─────────────────────────────────────────────┘
```

