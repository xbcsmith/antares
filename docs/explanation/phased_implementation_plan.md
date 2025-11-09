# Phased Implementation Plan for Antares RPG

**CRITICAL**: This is the master implementation plan. Each phase MUST be
completed in order. Before starting ANY phase, read this entire document and the
relevant sections of `docs/reference/architecture.md`.

---

## Table of Contents

1. [Pre-Implementation Requirements](#pre-implementation-requirements)
2. [Phase 1: Core Engine](#phase-1-core-engine-weeks-1-3)
3. [Phase 2: Combat System](#phase-2-combat-system-weeks-4-5)
4. [Phase 3: World System](#phase-3-world-system-weeks-6-8)
5. [Phase 4: Game Systems](#phase-4-game-systems-weeks-9-11)
6. [Phase 5: Content & Data](#phase-5-content--data-weeks-12-14)
7. [Phase 6: Polish & Testing](#phase-6-polish--testing-weeks-15-16)
8. [Validation Requirements](#validation-requirements)

---

## Pre-Implementation Requirements

### Before Starting ANY Phase

**MANDATORY CHECKLIST** (verify all items):

- [ ] Read `AGENTS.md` completely
- [ ] Read `docs/reference/architecture.md` Sections 1-4 (all data structures)
- [ ] Verify Rust toolchain installed: `rustup component add clippy rustfmt`
- [ ] Understand the game context (turn-based RPG, party vs roster, game modes)
- [ ] Understand type system (ItemId, SpellId, AttributePair, etc.)
- [ ] Understand validation workflow (architecture → code → tests → quality
      gates)

### Universal Rules for ALL Phases

**These apply to EVERY phase without exception:**

1. **Architecture First**: Read relevant architecture.md sections BEFORE writing
   code
2. **Exact Compliance**: Use data structures EXACTLY as defined in
   architecture.md Section 4
3. **Type Aliases**: Always use `ItemId`, `SpellId`, `MonsterId`, `MapId`,
   `CharacterId`, `TownId`, `EventId` (never raw `u32`)
4. **Constants**: Reference or extract constants (e.g., `Inventory::MAX_ITEMS`,
   never hardcode `20`)
5. **File Extensions**:
   - `.rs` for Rust source code in `src/`
   - `.ron` for game data files (items, spells, monsters, maps)
   - `.md` for documentation in `docs/`
6. **Quality Gates** (ALL must pass):

   ```bash
   cargo fmt --all
   cargo check --all-targets --all-features
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-features
   ```

7. **Documentation**: Update `docs/explanation/implementations.md` with summary
   of changes
8. **Never Modify**: Core data structures (architecture.md Section 4) without
   explicit approval

---

## Phase 1: Core Engine (Weeks 1-3)

### Goals

Establish foundational data structures and basic game state management. No UI,
no combat logic, just pure data structures and state.

### Architecture References

**READ BEFORE IMPLEMENTING:**

- Section 3: System Architecture (module structure)
- Section 4.1: Game State (`GameState`, `GameMode`)
- Section 4.2: World (`World`, `Map`, `Tile`, `WallType`)
- Section 4.3: Character (`Character`, `Stats`, `AttributePair`, `Party`,
  `Roster`, `Race`, `Class`, `Alignment`, `Condition`, `Inventory`, `Equipment`)
- Section 4.6: Supporting Types (`Position`, `Direction`, `DiceRoll`,
  `GameTime`)

### Task 1.1: Project Setup

**Objective**: Initialize Cargo project with correct structure.

**Steps**:

1. Verify `Cargo.toml` exists with:

   ```toml
   [package]
   name = "antares"
   version = "0.1.0"
   edition = "2021"

   [dependencies]
   serde = { version = "1.0", features = ["derive"] }
   ron = "0.8"
   thiserror = "1.0"
   rand = "0.8"
   ```

2. Create module structure per architecture.md Section 3.2:

   ```text
   src/
   ├── lib.rs
   ├── domain/
   │   ├── mod.rs
   │   ├── character.rs
   │   ├── world.rs
   │   ├── combat.rs
   │   ├── items.rs
   │   ├── spells.rs
   │   └── types.rs
   ├── application/
   │   ├── mod.rs
   │   └── game_state.rs
   └── main.rs
   ```

3. In `src/lib.rs`:

   ```rust
   pub mod domain;
   pub mod application;
   ```

4. Run quality gates:

   ```bash
   cargo fmt --all
   cargo check --all-targets --all-features
   ```

**Expected Result**: Project compiles with empty module structure.

### Task 1.2: Core Type Aliases and Supporting Types

**Objective**: Implement architecture.md Section 4.6 EXACTLY.

**File**: `src/domain/types.rs`

**Implementation Rules**:

1. Copy type aliases from architecture.md Section 4.6 EXACTLY:

   ```rust
   pub type ItemId = u32;
   pub type SpellId = u32;
   pub type MonsterId = u32;
   pub type MapId = u32;
   pub type CharacterId = u32;
   pub type TownId = u32;
   pub type EventId = u32;
   ```

2. Implement `Position`, `Direction`, `DiceRoll`, `GameTime` EXACTLY as in
   architecture.md
3. Include ALL methods shown in architecture.md:
   - `Direction::turn_left()`, `turn_right()`, `forward()`
   - `DiceRoll::new()`, `roll()`
   - `GameTime::advance_minutes()`

**Documentation Requirements**:

- Add `///` doc comments to ALL public items
- Include runnable examples in doc comments
- Example:

  ```rust
  /// Represents a position on the game map
  ///
  /// # Examples
  ///
  /// use antares::domain::types::Position;
  ///
  /// let pos = Position { x: 10, y: 20 };
  /// assert_eq!(pos.x, 10);
  ///
  pub struct Position {
      pub x: i32,
      pub y: i32,
  }
  ```

**Testing Requirements**:

Create `src/domain/types.rs` with tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direction_turn_left() {
        assert_eq!(Direction::North.turn_left(), Direction::West);
        assert_eq!(Direction::West.turn_left(), Direction::South);
    }

    #[test]
    fn test_direction_turn_right() {
        assert_eq!(Direction::North.turn_right(), Direction::East);
        assert_eq!(Direction::East.turn_right(), Direction::South);
    }

    #[test]
    fn test_direction_forward() {
        let pos = Position { x: 5, y: 5 };
        let new_pos = Direction::North.forward(&pos);
        assert_eq!(new_pos.y, 4); // North decreases y
    }

    #[test]
    fn test_dice_roll() {
        let roll = DiceRoll::new(2, 6, 3); // 2d6+3
        let result = roll.roll();
        assert!(result >= 5 && result <= 15); // Min: 2, Max: 12, +3
    }

    #[test]
    fn test_game_time_advance() {
        let mut time = GameTime { day: 1, hour: 23, minute: 50 };
        time.advance_minutes(20);
        assert_eq!(time.day, 2);
        assert_eq!(time.hour, 0);
        assert_eq!(time.minute, 10);
    }
}
```

**Validation**:

- [ ] All type aliases match architecture.md Section 4.6 EXACTLY
- [ ] All supporting types implemented with correct fields
- [ ] All methods from architecture.md are present
- [ ] Doc comments on all public items
- [ ] Tests cover all methods
- [ ] `cargo test` passes

### Task 1.3: Character Data Structures

**Objective**: Implement architecture.md Section 4.3 character system EXACTLY.

**File**: `src/domain/character.rs`

**Implementation Rules**:

1. Implement in this order (dependencies matter):

   - `AttributePair` and `AttributePair16` with methods
   - `Stats` struct
   - `Resistances` struct
   - `Race`, `Class`, `Sex`, `Alignment` enums
   - `Condition` struct with constants
   - `Inventory` and `InventorySlot`
   - `Equipment` struct with constants
   - `SpellBook` struct
   - `Character` struct (main)
   - `Party` struct
   - `Roster` struct
   - `QuestFlags` struct

2. **CRITICAL**: Use `AttributePair` pattern for ALL modifiable stats:

   ```rust
   pub struct AttributePair {
       pub base: u8,
       pub current: u8,
   }

   impl AttributePair {
       pub fn reset(&mut self) {
           self.current = self.base;
       }

       pub fn modify(&mut self, delta: i16) {
           let new_val = (self.current as i16 + delta).clamp(0, 255);
           self.current = new_val as u8;
       }
   }
   ```

3. **CRITICAL**: Extract constants:

   ```rust
   impl Inventory {
       pub const MAX_ITEMS: usize = 20;

       pub fn is_full(&self) -> bool {
           self.items.len() >= Self::MAX_ITEMS
       }
   }

   impl Equipment {
       pub const MAX_EQUIPPED: usize = 7;
   }
   ```

4. **CRITICAL**: Use Condition constants (not magic numbers):

   ```rust
   impl Condition {
       pub const FINE: u16 = 0x0000;
       pub const ASLEEP: u16 = 0x0001;
       pub const BLINDED: u16 = 0x0002;
       // ... all from architecture.md
   }
   ```

**Testing Requirements**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute_pair_modify() {
        let mut attr = AttributePair { base: 10, current: 10 };
        attr.modify(5);
        assert_eq!(attr.current, 15);
        assert_eq!(attr.base, 10); // Base unchanged
    }

    #[test]
    fn test_attribute_pair_reset() {
        let mut attr = AttributePair { base: 10, current: 5 };
        attr.reset();
        assert_eq!(attr.current, 10);
    }

    #[test]
    fn test_inventory_max_items() {
        let mut inv = Inventory { items: Vec::new() };
        for i in 0..Inventory::MAX_ITEMS {
            inv.items.push(InventorySlot {
                item_id: i as ItemId,
                charges: 1,
            });
        }
        assert!(inv.is_full());
    }

    #[test]
    fn test_equipment_count() {
        let equip = Equipment {
            weapon: Some(1),
            armor: Some(2),
            shield: None,
            helmet: None,
            boots: None,
            accessory1: None,
            accessory2: None,
        };
        assert_eq!(equip.equipped_count(), 2);
    }

    #[test]
    fn test_condition_flags() {
        let mut condition = Condition::FINE;
        condition |= Condition::POISONED;
        assert!(condition & Condition::POISONED != 0);
        assert!(!Condition::is_fatal(condition));
    }

    #[test]
    fn test_party_max_members() {
        let party = Party {
            members: vec![],
            gold: 100,
            gems: 10,
            food: 50,
            position_index: 0,
            light_units: 100,
        };
        // Party can have up to 6 members
        assert!(party.members.len() <= 6);
    }
}
```

**Validation**:

- [ ] All structs match architecture.md Section 4.3 field-for-field
- [ ] `AttributePair` pattern used for stats (NOT direct modification)
- [ ] Constants extracted (`MAX_ITEMS`, `MAX_EQUIPPED`, condition flags)
- [ ] Type aliases used (`ItemId`, `SpellId`, `CharacterId`)
- [ ] All enums match architecture.md
- [ ] Tests cover stat modification, inventory limits, conditions
- [ ] `cargo clippy` passes with zero warnings

### Task 1.4: World Data Structures

**Objective**: Implement architecture.md Section 4.2 world system EXACTLY.

**File**: `src/domain/world.rs`

**Implementation Rules**:

1. Implement in order:

   - `WallType` enum
   - `Tile` struct
   - `Map` struct
   - `World` struct

2. Use type aliases:

   ```rust
   use crate::domain::types::{MapId, Position, Direction, EventId};
   ```

3. Map structure:

   ```rust
   pub struct Map {
       pub id: MapId,
       pub width: u32,
       pub height: u32,
       pub tiles: Vec<Vec<Tile>>, // tiles[y][x]
       pub events: HashMap<EventId, MapEvent>,
       pub npcs: Vec<NpcData>,
   }
   ```

**Testing Requirements**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_creation() {
        let tile = Tile {
            terrain: 0,
            wall_type: WallType::None,
            blocked: false,
            is_special: false,
            is_dark: false,
            visited: false,
            event_trigger: None,
        };
        assert!(!tile.blocked);
        assert_eq!(tile.wall_type, WallType::None);
    }

    #[test]
    fn test_map_bounds() {
        let map = Map {
            id: 1,
            width: 20,
            height: 20,
            tiles: vec![vec![Tile::default(); 20]; 20],
            events: HashMap::new(),
            npcs: Vec::new(),
        };
        assert_eq!(map.tiles.len(), 20);
        assert_eq!(map.tiles[0].len(), 20);
    }

    #[test]
    fn test_world_map_access() {
        let mut world = World {
            maps: HashMap::new(),
            current_map: 1,
            party_position: Position { x: 0, y: 0 },
            party_facing: Direction::North,
        };
        world.maps.insert(1, Map::default());
        assert!(world.maps.contains_key(&1));
    }
}
```

**Validation**:

- [ ] All structs match architecture.md Section 4.2
- [ ] Type aliases used (`MapId`, `EventId`)
- [ ] Map uses 2D Vec for tiles
- [ ] Tests cover tile access and map bounds

### Task 1.5: Game State Management

**Objective**: Implement architecture.md Section 4.1 game state EXACTLY.

**File**: `src/application/game_state.rs`

**Implementation Rules**:

1. Implement `GameMode` enum EXACTLY from architecture.md
2. Implement `GameState` struct with all fields from Section 4.1
3. Add state transition methods (document state changes)

```rust
use crate::domain::{character::*, world::*, types::*};

pub enum GameMode {
    Exploration,
    Combat,
    Menu,
    Dialogue,
}

pub struct GameState {
    pub world: World,
    pub roster: Roster,
    pub party: Party,
    pub active_spells: ActiveSpells,
    pub mode: GameMode,
    pub time: GameTime,
    pub quests: QuestLog,
}

impl GameState {
    /// Create a new game with initial state
    pub fn new() -> Self {
        Self {
            world: World::new(),
            roster: Roster::new(),
            party: Party::new(),
            active_spells: ActiveSpells::default(),
            mode: GameMode::Exploration,
            time: GameTime { day: 1, hour: 6, minute: 0 },
            quests: QuestLog::default(),
        }
    }

    /// Transition from exploration to combat
    pub fn enter_combat(&mut self) {
        self.mode = GameMode::Combat;
        // Preserve party state
    }

    /// Transition from combat to exploration
    pub fn exit_combat(&mut self) {
        self.mode = GameMode::Exploration;
        // Clean up combat effects
    }
}
```

**Testing Requirements**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_state_creation() {
        let state = GameState::new();
        assert!(matches!(state.mode, GameMode::Exploration));
        assert_eq!(state.time.day, 1);
    }

    #[test]
    fn test_state_transition_preserves_party() {
        let mut state = GameState::new();
        state.party.gold = 1000;

        state.enter_combat();
        assert!(matches!(state.mode, GameMode::Combat));
        assert_eq!(state.party.gold, 1000); // Preserved

        state.exit_combat();
        assert!(matches!(state.mode, GameMode::Exploration));
        assert_eq!(state.party.gold, 1000); // Still preserved
    }

    #[test]
    fn test_game_modes() {
        let mut state = GameState::new();
        state.mode = GameMode::Menu;
        assert!(matches!(state.mode, GameMode::Menu));
    }
}
```

**Validation**:

- [ ] `GameState` has all fields from architecture.md Section 4.1
- [ ] `GameMode` enum matches architecture.md
- [ ] State transitions preserve party data
- [ ] Tests verify state preservation across mode changes

### Phase 1 Completion Checklist

**BEFORE claiming Phase 1 is complete, verify ALL:**

- [ ] All modules compile: `cargo check --all-targets --all-features`
- [ ] Zero clippy warnings:
      `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] All tests pass: `cargo test --all-features`
- [ ] Code formatted: `cargo fmt --all`
- [ ] All data structures match architecture.md Sections 4.1-4.3, 4.6 EXACTLY
- [ ] Type aliases used everywhere (no raw `u32` for IDs)
- [ ] Constants extracted (MAX_ITEMS, MAX_EQUIPPED, condition flags)
- [ ] `AttributePair` pattern used for all stats
- [ ] Doc comments on ALL public items
- [ ] Tests achieve >80% coverage
- [ ] Updated `docs/explanation/implementations.md` with Phase 1 summary

**Update Documentation**:

Add to `docs/explanation/implementations.md`:

```markdown
## Phase 1: Core Engine Implementation

**Completed**: [Date]

### Summary

Implemented core data structures and game state management per architecture.md
Sections 4.1-4.3, 4.6.

### Components Implemented

- Type aliases and supporting types (Position, Direction, DiceRoll, GameTime)
- Character system (Character, Stats, AttributePair, Party, Roster)
- World system (World, Map, Tile)
- Game state management (GameState, GameMode transitions)

### Architecture Compliance

- All structures match architecture.md field-for-field
- Type aliases used consistently
- Constants extracted (Inventory::MAX_ITEMS, Equipment::MAX_EQUIPPED)
- AttributePair pattern enforced for stat modifications

### Testing

- Unit tests for all core types
- State transition tests
- Boundary tests for inventory and equipment limits
- Coverage: [X]%

### Files Created

- `src/domain/types.rs`
- `src/domain/character.rs`
- `src/domain/world.rs`
- `src/application/game_state.rs`
```

---

## Phase 2: Combat System (Weeks 4-5)

### Goals

Implement turn-based combat system with monster AI, combat actions, and damage
calculation.

### Architecture References

**READ BEFORE IMPLEMENTING:**

- Section 4.4: Combat (`CombatState`, `Combatant`, `Monster`, `Attack`,
  `Handicap`)
- Section 5.1: Turn-Based Combat System
- Section 11.1: Combat Positioning System

### Task 2.1: Combat Data Structures

**Objective**: Implement architecture.md Section 4.4 combat structures EXACTLY.

**File**: `src/domain/combat.rs`

**Implementation Rules**:

1. Implement in order:

   - `Handicap` enum
   - `Combatant` enum
   - `MonsterResistances` struct
   - `MonsterCondition` enum
   - `Attack` struct
   - `AttackType` enum
   - `SpecialEffect` enum
   - `Monster` struct
   - `CombatState` struct

2. **CRITICAL**: Use type aliases:

   ```rust
   use crate::domain::types::{MonsterId, DiceRoll};
   ```

3. Monster structure must match architecture.md:

   ```rust
   pub struct Monster {
       pub name: String,
       pub stats: Stats,
       pub hp: AttributePair16,
       pub ac: i8,
       pub attacks: Vec<Attack>,
       pub loot: LootTable,
       pub experience_value: u32,
       pub flee_threshold: u8,
       pub special_attack_threshold: u8,
       pub resistances: MonsterResistances,
       pub can_regenerate: bool,
       pub can_advance: bool,
       pub is_undead: bool,
       pub magic_resistance: u8,
       pub conditions: u16,
       pub has_acted: bool,
   }
   ```

**Testing Requirements**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combat_state_creation() {
        let combat = CombatState {
            participants: Vec::new(),
            turn_order: Vec::new(),
            current_turn: 0,
            round: 1,
            status: CombatStatus::Active,
            handicap: Handicap::Even,
            can_flee: true,
            can_surrender: false,
            can_bribe: false,
            monsters_advance: false,
            monsters_regenerate: false,
        };
        assert_eq!(combat.round, 1);
        assert!(combat.can_flee);
    }

    #[test]
    fn test_monster_conditions() {
        let mut monster = Monster::default();
        monster.conditions = MonsterCondition::Paralyzed as u16;
        assert!(monster.conditions & (MonsterCondition::Paralyzed as u16) != 0);
    }

    #[test]
    fn test_attack_types() {
        let physical = Attack {
            damage: DiceRoll::new(2, 6, 0),
            attack_type: AttackType::Physical,
            special_effect: None,
        };
        assert!(matches!(physical.attack_type, AttackType::Physical));
    }

    #[test]
    fn test_handicap_system() {
        let combat_adv = CombatState {
            handicap: Handicap::PartyAdvantage,
            // ... other fields
        };
        assert!(matches!(combat_adv.handicap, Handicap::PartyAdvantage));
    }
}
```

**Validation**:

- [ ] All structs match architecture.md Section 4.4 EXACTLY
- [ ] Type aliases used (`MonsterId`, `DiceRoll`)
- [ ] Monster conditions use bitflags correctly
- [ ] Tests cover combat state, monster conditions, attacks

### Task 2.2: Combat Logic

**Objective**: Implement turn-based combat flow per architecture.md Section 5.1.

**File**: `src/application/combat_manager.rs`

**Implementation Rules**:

1. Combat flow must follow architecture.md Section 5.1:

   - Initiative determination
   - Turn order calculation
   - Action resolution
   - Damage calculation
   - Condition effects

2. Key functions:

   ```rust
   pub fn start_combat(
       party: &Party,
       monsters: Vec<Monster>,
       handicap: Handicap,
   ) -> CombatState;

   pub fn calculate_turn_order(
       party: &Party,
       monsters: &[Monster],
   ) -> Vec<Combatant>;

   pub fn resolve_attack(
       attacker: &Combatant,
       target: &Combatant,
       attack: &Attack,
   ) -> AttackResult;

   pub fn apply_damage(
       target: &mut Character,
       damage: i32,
       attack_type: AttackType,
   );
   ```

**Testing Requirements**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_order_by_speed() {
        let fast_char = Character { stats: Stats { speed: AttributePair { base: 20, current: 20 }, ..Default::default() }, ..Default::default() };
        let slow_char = Character { stats: Stats { speed: AttributePair { base: 5, current: 5 }, ..Default::default() }, ..Default::default() };

        let party = Party { members: vec![slow_char, fast_char], ..Default::default() };
        let turn_order = calculate_turn_order(&party, &[]);

        // Fast character should go first
        assert_eq!(turn_order[0], Combatant::Player(1));
    }

    #[test]
    fn test_damage_calculation() {
        let mut target = Character::default();
        target.hp = AttributePair16 { base: 50, current: 50 };

        apply_damage(&mut target, 10, AttackType::Physical);
        assert_eq!(target.hp.current, 40);
    }

    #[test]
    fn test_handicap_affects_initiative() {
        let party = Party::default();
        let monsters = vec![Monster::default()];

        let combat_adv = start_combat(&party, monsters.clone(), Handicap::PartyAdvantage);
        assert!(matches!(combat_adv.handicap, Handicap::PartyAdvantage));
    }

    #[test]
    fn test_monster_regeneration() {
        let mut monster = Monster::default();
        monster.can_regenerate = true;
        monster.hp = AttributePair16 { base: 100, current: 50 };

        apply_regeneration(&mut monster);
        assert!(monster.hp.current > 50);
    }
}
```

**Validation**:

- [ ] Combat flow matches architecture.md Section 5.1
- [ ] Turn order respects speed stats and handicap
- [ ] Damage calculation accounts for AC and resistances
- [ ] Monster regeneration and advancement work correctly
- [ ] Tests cover all combat phases

### Phase 2 Completion Checklist

**BEFORE claiming Phase 2 is complete, verify ALL:**

- [ ] Combat system compiles with Phase 1 code
- [ ] Zero clippy warnings
- [ ] All combat tests pass
- [ ] Combat structures match architecture.md Section 4.4
- [ ] Turn-based flow matches architecture.md Section 5.1
- [ ] Handicap system implemented correctly
- [ ] Monster AI (regeneration, advancement) works
- [ ] Doc comments on all combat functions
- [ ] Updated `docs/explanation/implementations.md`

---

## Phase 3: World System (Weeks 6-8)

### Goals

Implement map navigation, tile interactions, events, and party movement.

### Architecture References

**READ BEFORE IMPLEMENTING:**

- Section 4.2: World (already implemented in Phase 1, now add logic)
- Section 5.4: Map and Movement
- Section 11.4: Map Special Events

### Task 3.1: Movement and Navigation

**Objective**: Implement party movement and map navigation.

**File**: `src/application/world_manager.rs`

**Implementation Rules**:

1. Key functions:

   ```rust
   pub fn move_party(
       world: &mut World,
       direction: Direction,
   ) -> Result<(), MovementError>;

   pub fn check_tile_blocked(
       map: &Map,
       position: Position,
   ) -> bool;

   pub fn trigger_tile_event(
       world: &mut World,
       position: Position,
   ) -> Option<EventId>;
   ```

2. Movement must respect:
   - Tile blocked status
   - Wall types (can't walk through Normal walls)
   - Map boundaries
   - Door interactions

**Testing Requirements**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_party_basic() {
        let mut world = World::default();
        world.party_position = Position { x: 5, y: 5 };
        world.party_facing = Direction::North;

        move_party(&mut world, Direction::North).unwrap();
        assert_eq!(world.party_position.y, 4);
    }

    #[test]
    fn test_move_blocked_by_wall() {
        let mut world = World::default();
        let mut map = Map::default();
        map.tiles[4][5].wall_type = WallType::Normal;
        world.maps.insert(1, map);
        world.party_position = Position { x: 5, y: 5 };

        let result = move_party(&mut world, Direction::North);
        assert!(result.is_err());
    }

    #[test]
    fn test_map_boundaries() {
        let mut world = World::default();
        world.party_position = Position { x: 0, y: 0 };

        let result = move_party(&mut world, Direction::North);
        assert!(result.is_err()); // Can't move off map
    }

    #[test]
    fn test_door_interaction() {
        let mut map = Map::default();
        map.tiles[5][5].wall_type = WallType::Door;

        // Door should allow passage (or require interaction)
        assert_eq!(map.tiles[5][5].wall_type, WallType::Door);
    }
}
```

**Validation**:

- [ ] Movement respects tile blocking
- [ ] Map boundaries enforced
- [ ] Direction changes work correctly
- [ ] Tests cover all movement scenarios

### Task 3.2: Map Events System

**Objective**: Implement map event triggers per architecture.md Section 11.4.

**File**: `src/domain/events.rs`

**Implementation Rules**:

1. Define event types:

   ```rust
   pub enum MapEvent {
       Encounter { monster_group: Vec<MonsterId> },
       Treasure { loot: LootTable },
       Teleport { destination: Position, map_id: MapId },
       Trap { damage: DiceRoll, effect: Option<SpecialEffect> },
       Sign { text: String },
       NpcDialogue { npc_id: u32 },
   }
   ```

2. Event triggering:

   ```rust
   pub fn trigger_event(
       game_state: &mut GameState,
       event: MapEvent,
   ) -> EventResult;
   ```

**Testing Requirements**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encounter_event() {
        let event = MapEvent::Encounter {
            monster_group: vec![1, 2, 3],
        };
        let mut state = GameState::new();

        trigger_event(&mut state, event);
        assert!(matches!(state.mode, GameMode::Combat));
    }

    #[test]
    fn test_teleport_event() {
        let event = MapEvent::Teleport {
            destination: Position { x: 10, y: 10 },
            map_id: 2,
        };
        let mut state = GameState::new();

        trigger_event(&mut state, event);
        assert_eq!(state.world.current_map, 2);
        assert_eq!(state.world.party_position.x, 10);
    }

    #[test]
    fn test_trap_event_damages_party() {
        let event = MapEvent::Trap {
            damage: DiceRoll::new(2, 6, 0),
            effect: Some(SpecialEffect::Poison),
        };
        let mut state = GameState::new();
        state.party.members.push(Character::default());

        trigger_event(&mut state, event);
        // Verify damage and poison applied
    }
}
```

**Validation**:

- [ ] All event types implemented
- [ ] Events modify game state correctly
- [ ] Tests cover all event types

### Phase 3 Completion Checklist

**BEFORE claiming Phase 3 is complete:**

- [ ] Movement system works with Phase 1 World structures
- [ ] Map events trigger correctly
- [ ] Zero clippy warnings
- [ ] All tests pass
- [ ] Doc comments complete
- [ ] Updated `docs/explanation/implementations.md`

---

## Phase 4: Game Systems (Weeks 9-11)

### Goals

Implement magic system, character progression, resource management, and
conditions.

### Architecture References

**READ BEFORE IMPLEMENTING:**

- Section 4.3: Character (conditions, spell points)
- Section 5.2: Character Progression
- Section 5.3: Magic System
- Section 12.2: Food System
- Section 12.3: Rest System
- Section 12.4: Light System
- Section 12.5: Training and Leveling

### Task 4.1: Magic System

**Objective**: Implement spell casting per architecture.md Section 5.3.

**File**: `src/domain/spells.rs`

**Implementation Rules**:

1. Implement spell structures from architecture.md:

   ```rust
   pub struct Spell {
       pub id: SpellId,
       pub name: String,
       pub school: SpellSchool,
       pub level: u8,
       pub sp_cost: u16,
       pub gem_cost: u8,
       pub context: SpellContext,
       pub target: SpellTarget,
       pub description: String,
   }
   ```

2. Spell casting function from architecture.md Section 5.3:

   ```rust
   pub fn can_cast_spell(
       character: &Character,
       spell: &Spell,
       game_mode: GameMode,
   ) -> SpellState;

   pub fn cast_spell(
       caster: &mut Character,
       spell: &Spell,
       target: SpellTarget,
   ) -> SpellResult;
   ```

3. **CRITICAL**: Use `calculate_spell_points` EXACTLY as in architecture.md
   Section 5.3

**Testing Requirements**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spell_point_calculation() {
        let mut character = Character::default();
        character.class = Class::Sorcerer;
        character.level = 5;
        character.stats.intellect = AttributePair { base: 18, current: 18 };

        let sp = calculate_spell_points(&character);
        assert!(sp > 0);
    }

    #[test]
    fn test_cannot_cast_without_sp() {
        let mut character = Character::default();
        character.sp = AttributePair16 { base: 10, current: 0 };

        let spell = Spell {
            sp_cost: 5,
            ..Default::default()
        };

        let state = can_cast_spell(&character, &spell, GameMode::Exploration);
        assert!(matches!(state, SpellState::NotEnoughSP));
    }

    #[test]
    fn test_combat_only_spell_in_exploration() {
        let character = Character::default();
        let spell = Spell {
            context: SpellContext::CombatOnly,
            ..Default::default()
        };

        let state = can_cast_spell(&character, &spell, GameMode::Exploration);
        assert!(matches!(state, SpellState::CombatOnly));
    }

    #[test]
    fn test_cleric_cannot_cast_sorcerer_spell() {
        let mut character = Character::default();
        character.class = Class::Cleric;

        let spell = Spell {
            school: SpellSchool::Sorcerer,
            ..Default::default()
        };

        let state = can_cast_spell(&character, &spell, GameMode::Exploration);
        assert!(matches!(state, SpellState::WrongClass));
    }
}
```

**Validation**:

- [ ] Spell structures match architecture.md Section 4.6
- [ ] `can_cast_spell` checks all conditions from Section 5.3
- [ ] Spell point calculation uses formula from architecture.md
- [ ] Context restrictions enforced (CombatOnly, OutdoorOnly, etc.)

### Task 4.2: Character Progression

**Objective**: Implement leveling system per architecture.md Section 5.2 and
12.5.

**File**: `src/application/progression.rs`

**Implementation Rules**:

1. Implement experience and leveling:

   ```rust
   pub fn award_experience(
       character: &mut Character,
       xp: u32,
   );

   pub fn check_level_up(
       character: &Character,
   ) -> bool;

   pub fn level_up(
       character: &mut Character,
   );
   ```

2. Use `HpGainDie` from architecture.md Section 12.5:

   ```rust
   pub fn roll_hp_gain(class: Class) -> u8 {
       let die = match class {
           Class::Knight => 10,
           Class::Paladin => 10,
           // ... from architecture.md
       };
       rand::thread_rng().gen_range(1..=die)
   }
   ```

**Testing Requirements**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experience_gain() {
        let mut character = Character::default();
        character.experience = 0;

        award_experience(&mut character, 1000);
        assert_eq!(character.experience, 1000);
    }

    #[test]
    fn test_level_up_increases_level() {
        let mut character = Character::default();
        character.level = 1;

        level_up(&mut character);
        assert_eq!(character.level, 2);
    }

    #[test]
    fn test_hp_gain_by_class() {
        let knight_gain = roll_hp_gain(Class::Knight);
        assert!(knight_gain >= 1 && knight_gain <= 10);

        let sorcerer_gain = roll_hp_gain(Class::Sorcerer);
        assert!(sorcerer_gain >= 1 && sorcerer_gain <= 4);
    }
}
```

**Validation**:

- [ ] Level up system matches architecture.md Section 5.2
- [ ] HP gain uses correct dice per class (Section 12.5)
- [ ] Experience tracking works correctly

### Task 4.3: Resource Management

**Objective**: Implement food, light, rest systems per architecture.md
Section 12.

**File**: `src/application/resources.rs`

**Implementation Rules**:

1. Food consumption (Section 12.2):

   ```rust
   pub fn consume_food(party: &mut Party, amount: u32) -> Result<(), ResourceError>;
   pub fn check_starvation(party: &Party) -> bool;
   ```

2. Light system (Section 12.4):

   ```rust
   pub fn consume_light(party: &mut Party, tiles_moved: u32);
   pub fn is_dark(map: &Map, position: Position) -> bool;
   ```

3. Rest system (Section 12.3):

   ```rust
   pub fn rest_party(party: &mut Party, hours: u8) -> RestResult;
   ```

**Testing Requirements**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_food_consumption() {
        let mut party = Party::default();
        party.food = 100;

        consume_food(&mut party, 10).unwrap();
        assert_eq!(party.food, 90);
    }

    #[test]
    fn test_starvation_when_no_food() {
        let mut party = Party::default();
        party.food = 0;

        let result = consume_food(&mut party, 1);
        assert!(result.is_err());
        assert!(check_starvation(&party));
    }

    #[test]
    fn test_light_consumption() {
        let mut party = Party::default();
        party.light_units = 100;

        consume_light(&mut party, 10);
        assert!(party.light_units < 100);
    }

    #[test]
    fn test_rest_restores_hp() {
        let mut party = Party::default();
        let mut character = Character::default();
        character.hp = AttributePair16 { base: 50, current: 10 };
        party.members.push(character);

        rest_party(&mut party, 8);
        assert!(party.members[0].hp.current > 10);
    }
}
```

**Validation**:

- [ ] Food system matches architecture.md Section 12.2
- [ ] Light system matches Section 12.4
- [ ] Rest system matches Section 12.3
- [ ] Resource sharing works at party level

### Phase 4 Completion Checklist

**BEFORE claiming Phase 4 is complete:**

- [ ] Magic system fully functional
- [ ] Character progression works
- [ ] Resource management implemented
- [ ] All systems integrate with existing code
- [ ] Zero clippy warnings
- [ ] Tests pass
- [ ] Updated `docs/explanation/implementations.md`

---

## Phase 5: Content & Data (Weeks 12-14)

### Goals

Create data files for items, spells, monsters, and maps using RON format.

### Architecture References

**READ BEFORE IMPLEMENTING:**

- Section 4.5: Items
- Section 5.3: Magic System (spell data)
- Section 4.4: Combat (monster data)
- Section 7: Data-Driven Content
- Section 7.2: Example Data Format (RON)

### Task 5.1: Item Data

**Objective**: Create item data files in RON format per architecture.md
Section 7.

**File**: `data/items.ron`

**Implementation Rules**:

1. **CRITICAL**: Use `.ron` extension (NOT `.json` or `.yaml`)
2. Follow architecture.md Section 7.2 example format EXACTLY
3. Structure:

   ```ron
   [
       Item(
           id: 1,
           name: "Longsword",
           item_type: Weapon(WeaponData(
               damage: DiceRoll(count: 1, sides: 8, bonus: 0),
               bonus: 0,
               hands_required: 1,
           )),
           base_cost: 150,
           sell_cost: 75,
           disablements: 0x0000,
           constant_bonus: None,
           temporary_bonus: None,
           spell_effect: None,
           max_charges: 0,
           is_cursed: false,
       ),
       // More items...
   ]
   ```

4. Create at least:
   - 20 weapons (swords, axes, bows, staves)
   - 15 armor pieces (leather, chain, plate)
   - 10 shields
   - 20 consumables (potions, scrolls)
   - 5 quest items

**Validation**:

- [ ] File extension is `.ron` (NOT `.json` or `.yaml`)
- [ ] Format matches architecture.md Section 7.2
- [ ] All items have valid ItemId
- [ ] Disablement flags use constants from architecture.md
- [ ] File parses with `ron::from_str`

**Testing**:

```rust
#[test]
fn test_load_items_from_ron() {
    let ron_data = std::fs::read_to_string("data/items.ron").unwrap();
    let items: Vec<Item> = ron::from_str(&ron_data).unwrap();
    assert!(!items.is_empty());
    assert!(items.iter().all(|item| item.id > 0));
}
```

### Task 5.2: Spell Data

**Objective**: Create spell data files in RON format.

**File**: `data/spells.ron`

**Implementation Rules**:

1. Use `.ron` format
2. Structure per architecture.md Section 5.3:

   ```ron
   [
       Spell(
           id: 1,
           name: "Cure Light Wounds",
           school: Cleric,
           level: 1,
           sp_cost: 4,
           gem_cost: 0,
           context: Anytime,
           target: SingleCharacter,
           description: "Heals 1d8 hit points",
       ),
       // More spells...
   ]
   ```

3. Create spells covering:
   - Cleric levels 1-7
   - Sorcerer levels 1-7
   - All spell contexts (Anytime, CombatOnly, OutdoorOnly, etc.)

**Validation**:

- [ ] File is `data/spells.ron` (RON format)
- [ ] All spells have valid SpellId
- [ ] School, context, and target match enums from architecture.md
- [ ] File parses correctly

### Task 5.3: Monster Data

**Objective**: Create monster data files in RON format.

**File**: `data/monsters.ron`

**Implementation Rules**:

1. Use `.ron` format
2. Structure per architecture.md Section 4.4:

   ```ron
   [
       Monster(
           name: "Goblin",
           stats: Stats(
               might: AttributePair(base: 8, current: 8),
               intellect: AttributePair(base: 6, current: 6),
               // ...
           ),
           hp: AttributePair16(base: 10, current: 10),
           ac: 5,
           attacks: [
               Attack(
                   damage: DiceRoll(count: 1, sides: 6, bonus: 0),
                   attack_type: Physical,
                   special_effect: None,
               ),
           ],
           loot: LootTable(
               gold: DiceRoll(count: 1, sides: 10, bonus: 0),
               gems: DiceRoll(count: 0, sides: 0, bonus: 0),
               items: [],
               experience: 15,
           ),
           // ... rest of fields
       ),
   ]
   ```

**Validation**:

- [ ] File is `data/monsters.ron`
- [ ] All monsters have valid stats and attacks
- [ ] Resistances configured correctly
- [ ] File parses correctly

### Task 5.4: Map Data

**Objective**: Create initial map data in RON format.

**File**: `data/maps/town1.ron`

**Implementation Rules**:

1. Use `.ron` format in `data/maps/` directory
2. Structure:

   ```ron
   Map(
       id: 1,
       width: 20,
       height: 20,
       tiles: [
           // 2D array of tiles
       ],
       events: {
           1: Encounter(monster_group: [1, 2, 3]),
           2: Sign(text: "Welcome to town!"),
       },
       npcs: [],
   )
   ```

**Validation**:

- [ ] Maps in `data/maps/` directory
- [ ] All map files use `.ron` extension
- [ ] Tiles array is correct dimensions
- [ ] Events reference valid monster/item IDs

### Phase 5 Completion Checklist

**BEFORE claiming Phase 5 is complete:**

- [ ] All data files use `.ron` format (NOT `.json` or `.yaml`)
- [ ] Items data complete and parses
- [ ] Spells data complete and parses
- [ ] Monsters data complete and parses
- [ ] At least 3 maps created
- [ ] All IDs are consistent across files
- [ ] Data loading tests pass
- [ ] Updated `docs/explanation/implementations.md`

---

## Phase 6: Polish & Testing (Weeks 15-16)

### Goals

Integration testing, balance, bug fixes, and final documentation.

### Task 6.1: Integration Testing

**Objective**: Test complete game flows.

**File**: `tests/integration_tests.rs`

**Implementation Rules**:

1. Test complete scenarios:

   ```rust
   #[test]
   fn test_complete_combat_flow() {
       let mut game = GameState::new();
       // Create party
       // Trigger combat
       // Execute turns
       // Verify victory/defeat
   }

   #[test]
   fn test_exploration_to_combat_to_exploration() {
       let mut game = GameState::new();
       // Start in exploration
       // Enter combat
       // Win combat
       // Return to exploration
       // Verify state preserved
   }

   #[test]
   fn test_character_creation_to_first_combat() {
       // Full flow from character creation to first battle
   }
   ```

**Validation**:

- [ ] Integration tests cover all major systems
- [ ] State preservation verified across mode changes
- [ ] Resource management tested end-to-end

### Task 6.2: Balance and Polish

**Objective**: Tune game balance and fix issues.

**Activities**:

1. Balance combat (monster stats, damage)
2. Balance progression (XP curves, level requirements)
3. Balance economy (item costs, loot drops)
4. Fix any clippy warnings
5. Improve error messages
6. Add more comprehensive error handling

**Validation**:

- [ ] Combat feels balanced (not too easy/hard)
- [ ] Progression curve reasonable
- [ ] No panics or crashes in normal gameplay
- [ ] Error messages are helpful

### Task 6.3: Final Documentation

**Objective**: Complete all documentation.

**Files to Update**:

1. `docs/explanation/implementations.md` - Complete implementation summary
2. `README.md` - Usage instructions, examples
3. `docs/tutorials/getting_started.md` - New player tutorial
4. `docs/how_to/` - Guides for common tasks

**Validation**:

- [ ] All markdown files use lowercase_with_underscores.md
- [ ] README.md has usage examples
- [ ] API documentation complete
- [ ] Tutorial walkthrough works

### Phase 6 Completion Checklist

**BEFORE claiming Phase 6 is complete:**

- [ ] All integration tests pass
- [ ] Game is playable end-to-end
- [ ] Balance feels reasonable
- [ ] Zero clippy warnings
- [ ] All documentation complete
- [ ] Code coverage >80%
- [ ] `cargo doc --open` shows complete docs
- [ ] Updated `docs/explanation/implementations.md`

---

## Validation Requirements

### Per-Task Validation

After EVERY task, run:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

**ALL MUST PASS before proceeding to next task.**

### Per-Phase Validation

Before marking a phase complete, verify:

- [ ] All tasks in phase completed
- [ ] All quality gates pass
- [ ] Architecture compliance verified
- [ ] Tests achieve >80% coverage
- [ ] Documentation updated
- [ ] No architectural deviations
- [ ] Type aliases used consistently
- [ ] Constants extracted (no magic numbers)
- [ ] `AttributePair` pattern used for stats
- [ ] `.ron` format used for data files

### Final Project Validation

Before claiming project is complete:

- [ ] All 6 phases completed
- [ ] All validation checklists passed
- [ ] Integration tests pass
- [ ] Game is playable
- [ ] Documentation complete
- [ ] Code quality gates pass
- [ ] No architectural violations
- [ ] Data files all in RON format
- [ ] Type system adherence verified

---

## Critical Reminders

### The Golden Rules (Memorize These)

1. **Architecture First**: Read relevant architecture.md sections BEFORE coding
2. **Exact Compliance**: Use data structures EXACTLY as defined
3. **Type Aliases**: Always use `ItemId`, `SpellId`, etc. (never raw `u32`)
4. **Quality Gates**: All four cargo commands must pass before proceeding
5. **RON Format**: Game data uses `.ron` files, NOT `.json` or `.yaml`

### Common Mistakes to Avoid

- ❌ Modifying core data structures without approval
- ❌ Using raw `u32` instead of type aliases
- ❌ Hardcoding constants (magic numbers)
- ❌ Direct stat modification (must use `AttributePair`)
- ❌ Wrong file extensions (`.json`/`.yaml` for game data)
- ❌ Skipping quality gates
- ❌ Not reading architecture.md first

### When in Doubt

1. Check `AGENTS.md` for implementation rules
2. Check `docs/reference/architecture.md` for specifications
3. Check existing code for patterns
4. Ask for clarification rather than guessing

---

## Summary

This plan provides a clear, step-by-step path to implement Antares RPG following
the architecture document exactly. Each phase builds on the previous one, with
explicit validation requirements and testing expectations.

**Success Criteria**:

- All data structures match architecture.md field-for-field
- Type system used correctly throughout
- All quality gates pass
- Test coverage >80%
- Game is playable end-to-end
- Documentation complete

**Key to Success**:

1. Read architecture.md BEFORE implementing
2. Implement EXACTLY as specified
3. Validate AFTER each task
4. Test thoroughly
5. Document clearly

Follow this plan carefully, and the implementation will align perfectly with the
architecture vision.
