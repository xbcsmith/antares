# Engine SDK Campaign Data Integration - Implementation Plan

## Overview

This plan defines the complete integration of all SDK campaign data types (`characters.ron`, `classes.ron`, `conditions.ron`, `dialogues.ron`, `items.ron`, `maps/`, `monsters.ron`, `quests.ron`, `races.ron`, `spells.ron`) into the Antares game engine to enable a full gameplay loop including party initialization, world exploration, character management, NPC interaction, quest progression, and turn-based combat.

## Current State Analysis

### Existing Infrastructure

| Component               | Location                                     | Status         | Notes                                                                                                                     |
| ----------------------- | -------------------------------------------- | -------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `ContentDatabase`       | `antares/src/sdk/database.rs#L774-804`       | ✅ Implemented | Contains all 10 database types (classes, races, items, monsters, spells, maps, quests, dialogues, conditions, characters) |
| `Campaign`              | `antares/src/sdk/campaign_loader.rs#L94-130` | ✅ Implemented | Loads metadata and config but NOT content data                                                                            |
| `GameState`             | `antares/src/application/mod.rs#L248-266`    | ✅ Implemented | Has `campaign: Option<Campaign>` but no ContentDatabase access                                                            |
| `GameState::new_game()` | `antares/src/application/mod.rs#L313-332`    | ⚠️ Partial     | Sets gold/food but does NOT load character/roster data from campaign                                                      |
| Party/Roster            | `antares/src/application/mod.rs#L254-256`    | ✅ Implemented | Empty initialization only                                                                                                 |

### Identified Issues

1. **Missing ContentDatabase Resource**: Systems cannot access item/spell/monster definitions during gameplay
2. **No Character Loading**: `characters.ron` data not loaded into roster/party during `GameState::new_game()`
3. **No Class/Race Modifiers Applied**: Base stats from `classes.ron` and `races.ron` not applied to characters
4. **Static Map System**: Maps hardcoded, not loaded dynamically from `ContentDatabase.maps`
5. **No Dialogue Integration**: `dialogues.ron` data not accessible to NPC interaction systems
6. **No Quest System**: `quests.ron` data not integrated with progression tracking
7. **Combat Uses Hardcoded Data**: Monster stats not loaded from `ContentDatabase.monsters`
8. **Spell System Incomplete**: `spells.ron` data not used for spell casting validation
9. **Condition System Missing**: `conditions.ron` effects not applied in combat
10. **No Item Validation**: Equipment restrictions from classes/races not enforced

## Implementation Phases

### Phase 1: Core Content Database Integration

**Goal**: Make all campaign content accessible to engine systems as a Bevy resource.

#### 1.1 Create ContentDatabase Resource

**File**: `antares/src/application/resources.rs` (create new file)

**Actions**:

- Create new module file with SPDX header
- Import `ContentDatabase` from `crate::sdk::database::ContentDatabase`
- Define Bevy resource wrapper: `#[derive(Resource)] pub struct GameContent(pub ContentDatabase);`
- Add constructor: `impl GameContent { pub fn new(db: ContentDatabase) -> Self { Self(db) } }`
- Export in `antares/src/application/mod.rs`: `pub mod resources;`

**Type Aliases**: None required (ContentDatabase already defined)

**Constants**: None

#### 1.2 Integrate Campaign Content Loading

**File**: `antares/src/application/mod.rs`

**Function**: `GameState::new_game()` at line 313

**Actions**:

- Add new method `pub fn load_campaign_content(&self) -> Result<ContentDatabase, SdkError>`
- Call `Campaign::load_content(&self.campaign.as_ref().unwrap().path)`
- Return loaded `ContentDatabase` or propagate error
- Update `new_game()` signature to return `Result<(Self, ContentDatabase), SdkError>`
- Document error cases in function doc comment

**Validation Criteria**:

```rust
// In new_game() after calling load_campaign_content()
assert!(content_db.classes.count() > 0, "Classes database must not be empty");
assert!(content_db.races.count() > 0, "Races database must not be empty");
assert!(content_db.characters.count() > 0, "Characters database must not be empty");
```

#### 1.3 Testing Requirements

**File**: `antares/src/application/mod.rs` (add to existing tests module)

**Test Functions**:

```rust
#[test]
fn test_game_content_resource_creation()
#[test]
fn test_load_campaign_content_success()
#[test]
fn test_load_campaign_content_missing_files_error()
#[test]
fn test_new_game_returns_content_database()
```

**Success Criteria**:

- `cargo check --all-targets --all-features` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo nextest run --all-features` passes with 4 new tests passing
- Code coverage for new resource module >80%

#### 1.4 Deliverables

- [ ] `antares/src/application/resources.rs` created with `GameContent` resource
- [ ] `GameState::load_campaign_content()` implemented
- [ ] `GameState::new_game()` returns `(GameState, ContentDatabase)`
- [ ] 4 unit tests added and passing
- [ ] Documentation updated in `docs/explanation/implementations.md`

#### 1.5 Success Criteria

| Criterion             | Validation Command                                         | Expected Output                |
| --------------------- | ---------------------------------------------------------- | ------------------------------ |
| Compilation           | `cargo check --all-targets --all-features`                 | "Finished" with 0 errors       |
| Linting               | `cargo clippy --all-targets --all-features -- -D warnings` | "Finished" with 0 warnings     |
| Tests                 | `cargo nextest run --all-features`                         | All 4 new tests pass           |
| ContentDatabase loads | Manual test with tutorial campaign                         | `content_db.items.count() > 0` |

---

### Phase 2: Character System Integration

**Goal**: Load roster characters from `characters.ron` and apply class/race modifiers.

#### 2.1 Implement Character Loading from ContentDatabase

**File**: `antares/src/application/mod.rs`

**Function**: Add new method after `new_game()`

**Actions**:

- Add method: `pub fn initialize_roster(&mut self, content_db: &ContentDatabase) -> Result<(), GameError>`
- Iterate `content_db.characters.all()`
- For each character definition:
  - Fetch `ClassDefinition` from `content_db.classes.get(character.class_id)`
  - Fetch `RaceDefinition` from `content_db.races.get(character.race_id)`
  - Apply base stats: `character.stats.might.base = race_def.base_might + class_def.might_bonus`
  - Repeat for all 7 stats (Might, Intellect, Personality, Endurance, Speed, Accuracy, Luck)
  - Apply race resistances to character
  - Set initial HP/SP: `character.hp.current = character.stats.endurance.current * HP_PER_ENDURANCE`
  - Add character to `self.roster.add(character)`
- Return `Ok(())` or `Err(GameError::InvalidCharacterData)` if class/race not found

**Type Aliases**:

- `ClassId` (from `antares/src/character/class.rs`)
- `RaceId` (from `antares/src/character/race.rs`)
- `CharacterId` (from `antares/src/character/mod.rs`)

**Constants** (verify existence or add):

- `HP_PER_ENDURANCE = 10` (in `antares/src/character/stats.rs`)
- `SP_PER_INTELLECT = 5` (in `antares/src/character/stats.rs`)

#### 2.2 Integrate into Game Initialization

**File**: `antares/src/application/mod.rs`

**Function**: `GameState::new_game()` at line 313

**Actions**:

- After loading `content_db`, call `game_state.initialize_roster(&content_db)?`
- Propagate errors to caller
- Update documentation to reflect roster initialization

**Validation Criteria**:

```rust
assert_eq!(game_state.roster.count(), content_db.characters.count());
assert!(game_state.roster.get(CharacterId(0)).unwrap().stats.might.base > 0);
```

#### 2.3 Testing Requirements

**File**: `antares/src/application/mod.rs`

**Test Functions**:

```rust
#[test]
fn test_initialize_roster_loads_all_characters()
#[test]
fn test_initialize_roster_applies_class_modifiers()
#[test]
fn test_initialize_roster_applies_race_modifiers()
#[test]
fn test_initialize_roster_sets_initial_hp_sp()
#[test]
fn test_initialize_roster_invalid_class_id_error()
#[test]
fn test_initialize_roster_invalid_race_id_error()
```

**Test Data Setup**:

- Create mock `ContentDatabase` with 1 class, 1 race, 2 characters
- Knight class: Might +5, Endurance +3
- Human race: Base Might 10, Base Endurance 10
- Expected: Character might.base = 15, hp.current = 130

#### 2.4 Deliverables

- [ ] `GameState::initialize_roster()` implemented
- [ ] Class/Race modifier application logic implemented
- [ ] Initial HP/SP calculation implemented
- [ ] 6 unit tests added and passing
- [ ] Documentation updated

#### 2.5 Success Criteria

| Criterion      | Validation Command                                                 | Expected Output |
| -------------- | ------------------------------------------------------------------ | --------------- |
| Roster loads   | `cargo nextest run test_initialize_roster_loads_all_characters`    | Test passes     |
| Stats correct  | `cargo nextest run test_initialize_roster_applies_class_modifiers` | Test passes     |
| Error handling | `cargo nextest run test_initialize_roster_invalid_class_id_error`  | Test passes     |
| All checks     | `cargo clippy --all-targets --all-features -- -D warnings`         | 0 warnings      |

---

### Phase 3: Dynamic Map System

**Goal**: Replace static map spawning with dynamic loading from `ContentDatabase.maps`.

#### 3.1 Refactor Map Loading System

**File**: `antares/src/world/map.rs` (verify exact location with grep)

**Current Function**: Locate `spawn_map()` or equivalent system

**Actions**:

- Change from `Startup` schedule to `Update` schedule
- Add event listener: `EventReader<MapChangeEvent>`
- On `MapChangeEvent(map_id)`:
  - Despawn all entities with `MapTile` component
  - Fetch `MapDefinition` from `Res<GameContent>.maps.get(map_id)`
  - Iterate `map_def.tiles`: spawn entities with `Sprite`, `Position`, `MapTile(tile_type)` components
  - Iterate `map_def.events`: spawn entities with `EventTrigger(event_type, position)` component
- Return early if `map_id` not found in database

**Type Aliases**:

- `MapId = u16` (from `antares/src/world/map.rs` - verify or define)

**Constants**:

- `TILE_SIZE = 32` (pixels, verify in map.rs)

#### 3.2 Implement Map Event System

**File**: `antares/src/world/events.rs` (create new file)

**Actions**:

- Define event types:

```rust
#[derive(Event)]
pub struct MapChangeEvent(pub MapId);

#[derive(Component)]
pub struct EventTrigger {
    pub event_type: MapEventType,
    pub position: Position,
}

#[derive(Clone, Copy, Debug)]
pub enum MapEventType {
    Teleport { target_map: MapId, target_pos: Position },
    NpcDialogue { npc_id: u32 },
    CombatEncounter { monster_group_id: u32 },
    TreasureChest { loot_table_id: u32 },
}
```

- Add system to check player position against event triggers
- Fire appropriate events when player enters trigger tile

#### 3.3 Testing Requirements

**File**: `antares/src/world/map.rs`

**Test Functions**:

```rust
#[test]
fn test_map_change_event_despawns_old_tiles()
#[test]
fn test_map_change_event_spawns_new_tiles()
#[test]
fn test_map_change_event_invalid_map_id_no_crash()
#[test]
fn test_event_trigger_spawned_at_correct_position()
```

#### 3.4 Deliverables

- [ ] `spawn_map()` refactored to event-driven system
- [ ] `MapChangeEvent` implemented
- [ ] `EventTrigger` component and system implemented
- [ ] `antares/src/world/events.rs` created
- [ ] 4 unit tests added and passing

#### 3.5 Success Criteria

| Criterion         | Validation                         | Expected            |
| ----------------- | ---------------------------------- | ------------------- |
| Map loads from DB | Start game, trigger map change     | New map tiles spawn |
| Old map despawned | Count entities before/after        | Old tiles removed   |
| Tests pass        | `cargo nextest run --all-features` | 4 new tests pass    |

---

### Phase 4: Dialogue & Quest Systems

**Goal**: Integrate `dialogues.ron` and `quests.ron` for NPC interaction and progression.

#### 4.1 Implement Dialogue System

**File**: `antares/src/ui/dialogue.rs` (create new file)

**Actions**:

- Create dialogue state resource:

```rust
#[derive(Resource)]
pub struct DialogueState {
    pub active_tree_id: Option<u32>,
    pub current_node_id: u32,
    pub dialogue_history: Vec<u32>,
}
```

- Add system `handle_dialogue_trigger(trigger: Res<EventTrigger>, content: Res<GameContent>, state: ResMut<DialogueState>)`
- On `MapEventType::NpcDialogue(npc_id)`:
  - Fetch dialogue tree from `content.dialogues.get(npc_id)`
  - Set `state.active_tree_id = Some(npc_id)`
  - Set `state.current_node_id = dialogue_tree.root_node`
- Create UI rendering system to display current node text and choices
- Add input handling system for player choice selection
- Implement dialogue script actions (give item, start quest, set flag)

**Type Aliases**:

- `DialogueTreeId = u32`
- `DialogueNodeId = u32`

#### 4.2 Implement Quest System

**File**: `antares/src/application/quests.rs` (create new file or verify location)

**Actions**:

- Create quest tracking system:

```rust
pub struct QuestSystem;

impl QuestSystem {
    pub fn update(
        content: Res<GameContent>,
        mut game_state: ResMut<GameState>,
        events: EventReader<QuestProgressEvent>,
    ) {
        // Process events: KillMonster, CollectItem, ReachLocation
        // Update quest progress in game_state.quests
        // Check for quest completion
        // Grant rewards (XP, gold, items)
    }
}
```

- Define quest events:

```rust
#[derive(Event)]
pub enum QuestProgressEvent {
    MonsterKilled { monster_id: u32, count: u32 },
    ItemCollected { item_id: ItemId },
    LocationReached { map_id: MapId, position: Position },
}
```

- Implement quest completion rewards

**Type Aliases**:

- `QuestId = u32`
- `ItemId = u32`

#### 4.3 Testing Requirements

**Test Functions**:

```rust
// In antares/src/ui/dialogue.rs
#[test]
fn test_dialogue_tree_loads_root_node()
#[test]
fn test_dialogue_choice_advances_node()
#[test]
fn test_dialogue_script_action_gives_item()

// In antares/src/application/quests.rs
#[test]
fn test_quest_progress_updates_on_event()
#[test]
fn test_quest_completion_grants_rewards()
#[test]
fn test_quest_multiple_objectives_tracking()
```

#### 4.4 Deliverables

- [ ] `antares/src/ui/dialogue.rs` created with dialogue system
- [ ] `antares/src/application/quests.rs` created with quest system
- [ ] UI rendering for dialogue display implemented
- [ ] Quest progress events integrated
- [ ] 6 unit tests added and passing

#### 4.5 Success Criteria

| Criterion      | Validation                         | Expected               |
| -------------- | ---------------------------------- | ---------------------- |
| Dialogue loads | Trigger NPC event in test campaign | Dialogue text displays |
| Quest updates  | Kill monster, check quest log      | Progress increments    |
| Tests pass     | `cargo nextest run --all-features` | 6 new tests pass       |

---

### Phase 5: Combat Integration

**Goal**: Load monster data, spell effects, and conditions from ContentDatabase for combat.

#### 5.1 Integrate Monster Database

**File**: `antares/src/combat/engine.rs` (verify location)

**Function**: Combat initialization (locate `start_combat()` or equivalent)

**Actions**:

- Add parameter `content: &ContentDatabase` to combat initialization
- On encounter trigger with `monster_group_id`:
  - Fetch monster definitions from `content.monsters.get_group(monster_group_id)`
  - For each monster in group: create `Monster` entity with stats from definition
  - Apply monster special abilities from definition
  - Initialize combat turn order based on `Speed` stat
- Remove hardcoded monster stats

**Type Aliases**:

- `MonsterId = u32`
- `MonsterGroupId = u32`

#### 5.2 Integrate Spell System

**File**: `antares/src/magic/spell.rs` (verify location)

**Function**: Spell casting validation

**Actions**:

- Add spell validation: `pub fn can_cast_spell(character: &Character, spell_id: SpellId, content: &ContentDatabase) -> Result<bool, SpellError>`
- Check spell exists: `content.spells.get(spell_id)?`
- Validate SP cost: `character.sp.current >= spell_def.sp_cost`
- Validate class restrictions: `spell_def.allowed_classes.contains(&character.class)`
- Check conditions: silenced characters cannot cast
- On spell cast: apply effects from `spell_def.effects`

**Type Aliases**:

- `SpellId = u32`

**Constants**:

- Verify `CONDITION_SILENCED = 0b0001` exists in conditions module

#### 5.3 Integrate Condition System

**File**: `antares/src/combat/conditions.rs` (create or verify location)

**Actions**:

- Create condition application system:

```rust
pub fn apply_condition(
    target: &mut Character,
    condition_id: u32,
    content: &ContentDatabase,
) -> Result<(), ConditionError> {
    let condition_def = content.conditions.get(condition_id)?;
    target.conditions |= condition_def.flag_value;
    target.condition_durations.insert(condition_id, condition_def.base_duration);
    Ok(())
}
```

- Add condition tick system (reduce durations each turn)
- Implement condition effect enforcement (paralyzed cannot act, poisoned takes damage)

**Type Aliases**:

- `ConditionId = u32`

#### 5.4 Testing Requirements

**Test Functions**:

```rust
// In antares/src/combat/engine.rs
#[test]
fn test_combat_loads_monster_stats_from_db()
#[test]
fn test_combat_initiative_order_by_speed()
#[test]
fn test_combat_monster_special_ability_applied()

// In antares/src/magic/spell.rs
#[test]
fn test_can_cast_spell_sufficient_sp_success()
#[test]
fn test_can_cast_spell_insufficient_sp_error()
#[test]
fn test_can_cast_spell_silenced_condition_error()
#[test]
fn test_spell_effect_applies_damage()

// In antares/src/combat/conditions.rs
#[test]
fn test_apply_condition_sets_flag()
#[test]
fn test_condition_duration_decrements_per_turn()
#[test]
fn test_paralyzed_condition_prevents_action()
```

#### 5.5 Deliverables

- [ ] Monster loading from ContentDatabase implemented
- [ ] Spell validation using ContentDatabase implemented
- [ ] Condition system integrated
- [ ] Condition tick/enforcement implemented
- [ ] 10 unit tests added and passing

#### 5.6 Success Criteria

| Criterion             | Validation Command                                         | Expected                     |
| --------------------- | ---------------------------------------------------------- | ---------------------------- |
| Combat loads monsters | Start combat encounter                                     | Monster stats match database |
| Spell validation      | Cast spell with low SP                                     | Error returned               |
| Conditions work       | Apply poison condition                                     | Damage per turn applied      |
| Tests pass            | `cargo nextest run --all-features`                         | 10 new tests pass            |
| Clippy clean          | `cargo clippy --all-targets --all-features -- -D warnings` | 0 warnings                   |

---

### Phase 6: Inventory & Equipment Validation

**Goal**: Enforce class/race equipment restrictions using ContentDatabase.

#### 6.1 Implement Equipment Restriction Validation

**File**: `antares/src/inventory/equipment.rs` (verify location)

**Function**: Add new function

**Actions**:

- Implement validation:

```rust
pub fn can_equip_item(
    character: &Character,
    item_id: ItemId,
    content: &ContentDatabase,
) -> Result<bool, EquipError> {
    let item_def = content.items.get(item_id)
        .ok_or(EquipError::ItemNotFound)?;

    // Check class restrictions
    if !item_def.allowed_classes.is_empty()
        && !item_def.allowed_classes.contains(&character.class) {
        return Err(EquipError::ClassRestriction);
    }

    // Check race incompatibilities (e.g., elves can't wear heavy armor)
    let race_def = content.races.get(character.race)
        .ok_or(EquipError::InvalidRace)?;
    if race_def.incompatible_tags.iter()
        .any(|tag| item_def.tags.contains(tag)) {
        return Err(EquipError::RaceRestriction);
    }

    // Check equipment slot availability
    if !character.equipment.has_slot_for(&item_def.equipment_slot) {
        return Err(EquipError::NoSlotAvailable);
    }

    Ok(true)
}
```

**Type Aliases**:

- `ItemId = u32`

**Error Types**:

```rust
#[derive(Error, Debug)]
pub enum EquipError {
    #[error("Item not found in database")]
    ItemNotFound,
    #[error("Character class cannot use this item")]
    ClassRestriction,
    #[error("Character race cannot use this item")]
    RaceRestriction,
    #[error("No equipment slot available")]
    NoSlotAvailable,
    #[error("Invalid race definition")]
    InvalidRace,
}
```

#### 6.2 Testing Requirements

**Test Functions**:

```rust
#[test]
fn test_knight_can_equip_sword()
#[test]
fn test_sorcerer_cannot_equip_plate_armor()
#[test]
fn test_elf_cannot_equip_heavy_armor()
#[test]
fn test_equip_with_full_slots_error()
#[test]
fn test_equip_invalid_item_id_error()
```

#### 6.3 Deliverables

- [ ] `can_equip_item()` function implemented
- [ ] `EquipError` type defined
- [ ] Class/Race restriction validation implemented
- [ ] 5 unit tests added and passing

#### 6.4 Success Criteria

| Criterion             | Validation                         | Expected         |
| --------------------- | ---------------------------------- | ---------------- |
| Restrictions enforced | Test with elf + heavy armor        | Error returned   |
| Tests pass            | `cargo nextest run --all-features` | 5 new tests pass |
| Clippy clean          | `cargo clippy -- -D warnings`      | 0 warnings       |

---

## Complete Verification Plan

### Automated Verification Checklist

Run these commands after completing all phases:

```bash
# 1. Format check
cargo fmt --all -- --check

# 2. Compilation check
cargo check --all-targets --all-features

# 3. Linting
cargo clippy --all-targets --all-features -- -D warnings

# 4. Test execution
cargo nextest run --all-features

# 5. Test coverage (if cargo-tarpaulin installed)
cargo tarpaulin --all-features --workspace --timeout 300
```

**Expected Results**:

- Format: No changes needed
- Check: 0 errors
- Clippy: 0 warnings
- Tests: All tests pass (35+ new tests from all phases)
- Coverage: >80% for new code

### Manual Gameplay Verification

**Scenario**: Complete gameplay loop test with tutorial campaign

| Step | Action                | Expected Result                  | Verification                            |
| ---- | --------------------- | -------------------------------- | --------------------------------------- |
| 1    | Load campaign         | `GameContent` resource populated | `content.items.count() > 0`             |
| 2    | Start new game        | Roster loaded with characters    | `game_state.roster.count() == 3`        |
| 3    | Check character stats | Class/Race modifiers applied     | Knight has Might > base value           |
| 4    | Walk to new map       | Map change event triggers        | Old tiles despawn, new tiles spawn      |
| 5    | Talk to NPC           | Dialogue system activates        | Dialogue UI displays text               |
| 6    | Accept quest          | Quest added to log               | `game_state.quests.active_count() == 1` |
| 7    | Trigger combat        | Monster group loads              | Monster stats match `monsters.ron`      |
| 8    | Cast spell            | Spell validation succeeds        | SP cost deducted, effect applied        |
| 9    | Apply condition       | Condition flag set               | Target poisoned, takes damage per turn  |
| 10   | Kill monster          | Quest progress updates           | Quest completion check runs             |
| 11   | Loot item             | Item added to inventory          | Inventory count increases               |
| 12   | Equip item            | Restriction validation runs      | Elf cannot equip heavy armor            |

### Database Population Verification

**Command**: After Phase 1 completion, run in test:

```rust
#[test]
fn test_content_database_fully_populated() {
    let campaign = load_test_campaign();
    let (game_state, content_db) = GameState::new_game(campaign).unwrap();

    assert!(content_db.classes.count() > 0, "Classes not loaded");
    assert!(content_db.races.count() > 0, "Races not loaded");
    assert!(content_db.items.count() > 0, "Items not loaded");
    assert!(content_db.monsters.count() > 0, "Monsters not loaded");
    assert!(content_db.spells.count() > 0, "Spells not loaded");
    assert!(content_db.maps.count() > 0, "Maps not loaded");
    assert!(content_db.quests.count() > 0, "Quests not loaded");
    assert!(content_db.dialogues.count() > 0, "Dialogues not loaded");
    assert!(content_db.conditions.count() > 0, "Conditions not loaded");
    assert!(content_db.characters.count() > 0, "Characters not loaded");
}
```

## Architecture Compliance Checklist

- [ ] All type aliases used: `ItemId`, `SpellId`, `MonsterId`, `MapId`, `CharacterId`, `QuestId`, `DialogueTreeId`, `ConditionId`
- [ ] No raw `u32` or `usize` used for entity IDs
- [ ] `AttributePair` pattern used for character stats (base + current)
- [ ] Constants extracted (HP_PER_ENDURANCE, SP_PER_INTELLECT, TILE_SIZE)
- [ ] Error types use `thiserror` derive
- [ ] All public functions have `///` doc comments with examples
- [ ] No `unwrap()` without justification (use `?` operator)
- [ ] RON format used for all data files (not JSON/YAML)
- [ ] Module placement follows `src/{domain}/` structure
- [ ] Tests use descriptive names: `test_{function}_{condition}_{expected}`

## Documentation Updates

After each phase completion, update:

**File**: `antares/docs/explanation/implementations.md`

**Section to Add**:

```markdown
### Engine SDK Integration - Phase {N}

**Date**: {YYYY-MM-DD}
**Implemented By**: {Agent/Developer}

**Summary**: {One paragraph describing what was implemented}

**Files Modified**:

- `antares/src/application/mod.rs`: Added {functionality}
- `antares/src/world/map.rs`: Refactored {system}

**Tests Added**: {Count} tests covering {coverage areas}

**Validation**: All quality checks passed (fmt, check, clippy, nextest)
```

## Risk Mitigation

| Risk                        | Impact | Mitigation                                                  |
| --------------------------- | ------ | ----------------------------------------------------------- |
| ContentDatabase not loaded  | High   | Phase 1 validation ensures DB populated before later phases |
| Type mismatches             | Medium | Use type aliases consistently, verified by clippy           |
| Missing data files          | High   | Error handling returns `Result<T, E>`, propagates to UI     |
| Performance with large maps | Medium | Benchmark map loading, optimize if >100ms                   |
| Circular dependencies       | High   | Follow layered architecture, domain layer independent       |

## Next Steps After Completion

1. **Performance Profiling**: Measure ContentDatabase lookup times
2. **Asset Preloading**: Cache frequently accessed definitions
3. **Mod Support**: Allow ContentDatabase extension/override
4. **Save Game Integration**: Serialize ContentDatabase reference in save files
5. **Hot Reloading**: Watch data files for changes during development
