# Combat System Status

Summary of the current combat system implementation and what remains to be built for turn-based combat encounters.

## What's Implemented ✓

### Domain Layer (`src/domain/combat/`)

| Component | File | Description |
|-----------|------|-------------|
| `CombatState` | `engine.rs` | Tracks participants, turn order, rounds, status, flags (flee/surrender/bribe) |
| `Combatant` enum | `engine.rs` | Player/Monster variants with unified interface (`can_act`, `is_alive`, `get_speed`) |
| `Monster` struct | `monster.rs` | Full definition with stats, attacks, resistances, conditions, loot table |
| `MonsterResistances` | `monster.rs` | Immunities to damage types and effects |
| `MonsterCondition` | `monster.rs` | Status enum (Normal, Paralyzed, Asleep, Dead, etc.) |
| `LootTable` | `monster.rs` | Gold, gems, items, experience rewards |
| `Attack` struct | `types.rs` | Damage roll, attack type, special effects |
| `AttackType` enum | `types.rs` | Physical, Fire, Cold, Electricity, etc. |
| `SpecialEffect` enum | `types.rs` | Poison, Paralysis, Sleep, Death, etc. |
| `Handicap` enum | `types.rs` | PartyAdvantage, MonsterAdvantage, Even |
| `CombatStatus` enum | `types.rs` | InProgress, Victory, Defeat, Fled, Surrendered |
| `CombatantId` enum | `types.rs` | Reference to Player(index) or Monster(index) |
| `MonsterDatabase` | `database.rs` | Load monsters from `monsters.ron` |

### Combat Logic Functions (`engine.rs`)

| Function | Description |
|----------|-------------|
| `start_combat()` | Initialize combat, calculate turn order |
| `calculate_turn_order()` | Speed-based ordering with handicap support |
| `resolve_attack()` | Hit calculation, damage roll, special effects |
| `apply_damage()` | Apply damage to a combatant |
| `choose_monster_attack()` | Select attack (honors special attack threshold) |
| `roll_spell_damage()` | Roll spell damage dice |
| `apply_condition_to_character()` | Apply condition effects to players |
| `apply_condition_to_monster()` | Apply condition effects to monsters |
| `initialize_combat_from_group()` | Create combat from monster IDs |
| `reconcile_character_conditions()` | Sync status flags with active conditions |
| `reconcile_monster_conditions()` | Sync monster status with active conditions |

### CombatState Methods

| Method | Description |
|--------|-------------|
| `add_player()` / `add_monster()` | Add participants |
| `alive_party_count()` / `alive_monster_count()` | Count living combatants |
| `check_combat_end()` | Detect victory/defeat |
| `advance_turn()` / `advance_round()` | Progress combat |
| `apply_dot_effects()` | Process damage/healing over time |
| `get_combatant()` / `get_combatant_mut()` | Access participants by ID |

### Application Layer (`src/application/mod.rs`)

| Function | Description |
|----------|-------------|
| `enter_combat()` | Transition GameMode to Combat |
| `enter_combat_with_state()` | Enter combat with prepared CombatState |
| `start_encounter()` | Initialize combat from monster group |
| `exit_combat()` | Return to Exploration mode |

---

## What's Missing ✗

### 1. Combat UI / Bevy Integration (Critical)

**No Bevy system or UI for combat.** The `src/game/systems/` directory has no `combat.rs` or combat-related visual systems.

The only combat reference in game systems is a TODO:
```rust
// TODO: Start combat  (events.rs:164)
```

**Needed:**
- `combat_ui.rs` - Combatant display, HP bars, turn indicators
- Combat HUD showing current turn, round, available actions
- Monster sprites/visuals during encounter

### 2. Player Action System

No mechanism for players to interact during their turn:
- Target selection UI
- Action menu (Attack, Defend, Cast Spell, Use Item, Flee)
- `execute_player_action()` function
- Input handling during combat mode

### 3. Monster AI / Turn Execution

While `choose_monster_attack()` selects an attack, there's no:
- `execute_monster_turn()` function
- Target selection AI (prioritize weak targets, heal allies, etc.)
- Automated combat loop that processes monster turns

### 4. Flee/Surrender/Bribe Mechanics

`CombatState` has flags but no implementation:
- `can_flee: bool` - No flee attempt logic or success calculation
- `can_surrender: bool` - No surrender handling
- `can_bribe: bool` - No bribe cost calculation or monster acceptance

### 5. Party ↔ Combat State Synchronization

No system to:
- Copy `GameState.party` members into `CombatState.participants` at start
- Sync damage, conditions, and resources back to party after combat
- Handle character death during combat

### 6. Combat Rewards Distribution

`LootTable` is defined but not used:
- No gold/gem distribution logic
- No XP distribution to party
- No item drop handling or inventory integration

### 7. Combat Encounter Triggers

No integration with map/event systems:
- Random encounter system (based on terrain/steps)
- Scripted encounter triggers from map tiles
- Boss encounter handling

---

## Recommended Implementation Order

1. **Combat UI System** - Display combatants, HP bars, turn order
2. **Player Action Handler** - Input handling for action selection
3. **Monster AI System** - Automated monster turn execution
4. **Combat State Manager** - Bevy resource bridging domain ↔ game
5. **Party Sync System** - Initialize from party, sync results back
6. **Reward Distribution** - Post-combat XP/gold/loot
7. **Encounter Triggers** - Map tile events triggering combat

---

## Related Files

- Domain: `src/domain/combat/*.rs`
- Application: `src/application/mod.rs` (GameMode::Combat, enter_combat, exit_combat)
- Tests: `tests/combat_integration.rs`
- Data: `data/monsters.ron`
