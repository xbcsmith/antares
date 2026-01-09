# Character Recruitment System Implementation Plan

## Overview

This plan outlines the implementation of the character recruitment system, which enables players to recruit characters from maps into their active party or send them to inns for later recruitment. The system integrates recruitable character map events with the dialogue system, party management, and inn roster mechanics.

**Current State:** Phase 4 of the Game Engine Fixes implementation added visual markers and dialogue integration for recruitable characters, but the actual recruitment logic is stubbed with TODO comments. This plan addresses completing the recruitment mechanics.

**Goal:** Enable full character recruitment functionality including:
- Loading character definitions from the character database
- Adding characters to active party (with space validation)
- Storing characters at inns for later party management
- Removing recruited characters from maps to prevent re-recruitment
- Dynamic dialogue variable substitution ({CHARACTER_NAME})

## Current State Analysis

### Existing Infrastructure

**Character System (`src/domain/character.rs`):**
- `Character` struct with full runtime state (HP, SP, inventory, equipment)
- `Party` struct managing 0-6 active characters with shared gold/gems/food
- `Roster` struct managing up to 18 total characters with location tracking
- `CharacterLocation` enum tracking whether character is in party, at inn, or on map

**Character Definition System (`src/domain/character_definition.rs`):**
- `CharacterDefinition` struct defining character templates in RON files
- `CharacterDatabase` for loading and managing character definitions
- `instantiate()` method to convert definition into runtime `Character` instance
- `StartingEquipment` for defining initial equipped items

**Dialogue System (`src/game/systems/dialogue.rs`):**
- `DialogueAction::RecruitToParty { character_id }` variant defined
- `DialogueAction::RecruitToInn { character_id, innkeeper_id }` variant defined
- `handle_recruitment_actions()` system registered but contains TODO placeholders
- `execute_action()` function handles dialogue actions but recruitment actions are stubbed

**Map Event System (`src/domain/world/types.rs`):**
- `MapEvent::RecruitableCharacter { name, description, character_id }` variant
- Map events stored in `HashMap<Position, MapEvent>` in `Map` struct
- No current mechanism to remove events after interaction

**Input System (`src/game/systems/input.rs`):**
- E-key interaction detects recruitable characters and triggers dialogue ID 100
- Logs character name and ID for debugging

### Identified Issues

1. **TODO Placeholders:** Recruitment action handlers contain TODO comments instead of logic
2. **No Character Loading:** No mechanism to load `CharacterDefinition` from database and instantiate
3. **No Party Space Validation:** No check for party size limit (max 6 members)
4. **No Roster Management:** No integration with `Roster` for tracking character locations
5. **No MapEvent Removal:** Characters remain on map after recruitment, allowing duplicate recruitment
6. **No Inn Roster System:** No data structure to track which characters are stored at which inn
7. **No Dialogue Variables:** {CHARACTER_NAME} placeholder in dialogue text not substituted
8. **No Error Handling:** No user feedback when recruitment fails (party full, character not found, etc.)
9. **No HUD Refresh:** Party changes don't trigger HUD update to show new party member

## Implementation Phases

### Phase 1: Character Definition Loading and Instantiation

#### 1.1 Add Character Database to GameContent

**File:** `src/application/resources.rs`

Add `CharacterDatabase` to `GameContent` resource:

```rust
pub struct GameContent {
    pub items: ItemDatabase,
    pub quests: QuestDatabase,
    pub dialogues: DialogueDatabase,
    pub classes: ClassDatabase,
    pub races: RaceDatabase,
    pub npcs: NpcDatabase,
    pub characters: CharacterDatabase,  // NEW
}
```

**File:** `src/application/resources.rs`

Update `GameContent::load()` to load character database from `data/characters.ron`:

```rust
let characters = CharacterDatabase::load_from_file(
    &campaign_dir.join("data/characters.ron"),
    &classes,
    &races,
    &items,
)?;
```

#### 1.2 Add Character Instantiation Helper

**File:** `src/game/systems/dialogue.rs`

Add helper function to load and instantiate character:

```rust
/// Load character definition and instantiate as runtime Character
fn instantiate_character(
    character_id: &str,
    content: &GameContent,
) -> Result<Character, String> {
    let db = content.db();

    // Load character definition
    let definition = db.characters.get_character(character_id)
        .ok_or_else(|| format!("Character definition '{}' not found", character_id))?;

    // Instantiate character from definition
    definition.instantiate(&db.classes, &db.races, &db.items)
        .map_err(|e| format!("Failed to instantiate character '{}': {}", character_id, e))
}
```

#### 1.3 Testing Requirements

**File:** `src/game/systems/dialogue.rs`

Add unit tests:

```rust
#[test]
fn test_instantiate_character_success() {
    // Arrange: Create mock GameContent with test character definition
    // Act: Call instantiate_character()
    // Assert: Returns Ok(Character) with expected name, class, race
}

#[test]
fn test_instantiate_character_not_found() {
    // Arrange: Create GameContent without target character
    // Act: Call instantiate_character("nonexistent")
    // Assert: Returns Err with descriptive message
}
```

#### 1.4 Post-Implementation Validation

Run all quality checks:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**Expected:** All checks pass with 0 errors/warnings

#### 1.5 Deliverables

- [ ] `src/application/resources.rs`: CharacterDatabase added to GameContent
- [ ] `src/application/resources.rs`: Character database loading added to GameContent::load()
- [ ] `src/game/systems/dialogue.rs`: instantiate_character() helper function added
- [ ] `src/game/systems/dialogue.rs`: 2 new unit tests for character instantiation
- [ ] All quality gates pass (fmt, check, clippy, nextest)

#### 1.6 Success Criteria

- ✅ GameContent contains CharacterDatabase loaded from `data/characters.ron`
- ✅ instantiate_character() successfully loads and instantiates character definitions
- ✅ Error handling provides descriptive messages for missing characters
- ✅ All tests pass with >80% coverage

---

### Phase 2: Party Recruitment Implementation

#### 2.1 Implement RecruitToParty Action Handler

**File:** `src/game/systems/dialogue.rs`

Replace TODO in `execute_action()` with full implementation:

```rust
DialogueAction::RecruitToParty { character_id } => {
    info!("Recruiting character '{}' to active party", character_id);

    // Check party size limit
    if game_state.party.members.len() >= Party::MAX_MEMBERS {
        warn!("Party is full (max {} members)", Party::MAX_MEMBERS);
        if let Some(ref mut log) = game_log {
            log.add(format!("Party is full! You can only have {} members.", Party::MAX_MEMBERS));
        }
        return;
    }

    // Load and instantiate character
    match instantiate_character(character_id, db) {
        Ok(character) => {
            let name = character.name.clone();
            game_state.party.members.push(character);

            info!("Successfully recruited '{}' to party", name);
            if let Some(ref mut log) = game_log {
                log.add(format!("{} has joined your party!", name));
            }

            // TODO Phase 3: Remove MapEvent from current map
            // TODO Phase 4: Trigger HUD refresh
        }
        Err(e) => {
            warn!("Failed to recruit character: {}", e);
            if let Some(ref mut log) = game_log {
                log.add(format!("Recruitment failed: {}", e));
            }
        }
    }
}
```

#### 2.2 Implement Party Size Validation

**File:** `src/domain/character.rs`

Add constant if not already defined:

```rust
impl Party {
    /// Maximum party size
    pub const MAX_MEMBERS: usize = 6;
}
```

#### 2.3 Testing Requirements

**File:** `src/game/systems/dialogue.rs`

Add integration tests:

```rust
#[test]
fn test_recruit_to_party_success() {
    // Arrange: Create GameState with empty party and mock character
    // Act: Execute RecruitToParty action
    // Assert: Character added to party, game log contains success message
}

#[test]
fn test_recruit_to_party_full() {
    // Arrange: Create GameState with 6 party members
    // Act: Execute RecruitToParty action
    // Assert: Party unchanged, game log contains "Party is full" message
}

#[test]
fn test_recruit_to_party_character_not_found() {
    // Arrange: Create GameState with empty character database
    // Act: Execute RecruitToParty action with invalid character_id
    // Assert: Party unchanged, game log contains error message
}
```

#### 2.4 Post-Implementation Validation

Run all quality checks:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**Expected:** All checks pass, 3 new tests passing

#### 2.5 Deliverables

- [ ] `src/game/systems/dialogue.rs`: RecruitToParty action fully implemented in execute_action()
- [ ] `src/domain/character.rs`: Party::MAX_MEMBERS constant verified/added
- [ ] `src/game/systems/dialogue.rs`: 3 new integration tests for party recruitment
- [ ] Error messages provide clear user feedback via game log
- [ ] All quality gates pass

#### 2.6 Success Criteria

- ✅ Characters successfully added to party when space available
- ✅ Recruitment fails gracefully when party is full (max 6 members)
- ✅ User receives descriptive feedback via game log
- ✅ Error handling covers all failure cases (not found, party full)
- ✅ All tests pass with 100% success rate

---

### Phase 3: Inn Roster Recruitment Implementation

#### 3.1 Add Inn Roster Data Structure

**File:** `src/domain/character.rs`

Add inn roster tracking to Roster struct or create new InnRoster component:

**Option A: Extend Roster with inn tracking**

```rust
impl Roster {
    /// Get all characters at a specific inn
    pub fn characters_at_inn(&self, innkeeper_id: &InnkeeperId) -> Vec<&Character> {
        self.characters.iter()
            .enumerate()
            .filter(|(idx, _)| {
                matches!(
                    self.character_locations.get(*idx),
                    Some(CharacterLocation::AtInn(id)) if id == innkeeper_id
                )
            })
            .map(|(_, character)| character)
            .collect()
    }

    /// Add character to inn roster
    pub fn add_to_inn(
        &mut self,
        character: Character,
        innkeeper_id: InnkeeperId,
    ) -> Result<(), CharacterError> {
        if self.characters.len() >= Self::MAX_CHARACTERS {
            return Err(CharacterError::RosterFull(Self::MAX_CHARACTERS));
        }
        self.characters.push(character);
        self.character_locations.push(CharacterLocation::AtInn(innkeeper_id));
        Ok(())
    }
}
```

**Option B: Create dedicated InnRoster struct (if more complex inn management needed)**

```rust
pub struct InnRoster {
    /// Characters stored at each inn, keyed by innkeeper ID
    pub characters_by_inn: HashMap<InnkeeperId, Vec<Character>>,
}
```

**Decision Point:** Choose Option A (simpler, uses existing Roster infrastructure)

#### 3.2 Implement RecruitToInn Action Handler

**File:** `src/game/systems/dialogue.rs`

Replace TODO in `execute_action()` with full implementation:

```rust
DialogueAction::RecruitToInn { character_id, innkeeper_id } => {
    info!("Sending character '{}' to inn (keeper: {})", character_id, innkeeper_id);

    // Validate innkeeper exists
    let npc_db = &db.npcs;
    if npc_db.get_npc(innkeeper_id).is_none() {
        warn!("Innkeeper '{}' not found", innkeeper_id);
        if let Some(ref mut log) = game_log {
            log.add(format!("Error: Innkeeper '{}' not found", innkeeper_id));
        }
        return;
    }

    // Check roster size limit
    if game_state.roster.characters.len() >= Roster::MAX_CHARACTERS {
        warn!("Roster is full (max {} characters)", Roster::MAX_CHARACTERS);
        if let Some(ref mut log) = game_log {
            log.add(format!("Roster is full! Maximum {} characters.", Roster::MAX_CHARACTERS));
        }
        return;
    }

    // Load and instantiate character
    match instantiate_character(character_id, db) {
        Ok(character) => {
            let name = character.name.clone();

            // Add to roster at inn location
            if let Err(e) = game_state.roster.add_to_inn(character, innkeeper_id.clone()) {
                warn!("Failed to add character to inn: {}", e);
                if let Some(ref mut log) = game_log {
                    log.add(format!("Failed to send to inn: {}", e));
                }
                return;
            }

            info!("Successfully sent '{}' to inn '{}'", name, innkeeper_id);
            if let Some(ref mut log) = game_log {
                log.add(format!("{} has been sent to the inn!", name));
            }

            // TODO Phase 4: Remove MapEvent from current map
        }
        Err(e) => {
            warn!("Failed to recruit character: {}", e);
            if let Some(ref mut log) = game_log {
                log.add(format!("Recruitment failed: {}", e));
            }
        }
    }
}
```

#### 3.3 Testing Requirements

**File:** `src/domain/character.rs`

Add unit tests for inn roster methods:

```rust
#[test]
fn test_roster_add_to_inn() {
    // Arrange: Create empty roster and test character
    // Act: Add character to inn with innkeeper_id
    // Assert: Character added, location is CharacterLocation::AtInn
}

#[test]
fn test_roster_characters_at_inn() {
    // Arrange: Create roster with characters at different inns
    // Act: Query characters_at_inn("innkeeper_1")
    // Assert: Returns only characters at that specific inn
}

#[test]
fn test_roster_add_to_inn_full() {
    // Arrange: Create roster with MAX_CHARACTERS
    // Act: Attempt to add another character
    // Assert: Returns Err(CharacterError::RosterFull)
}
```

**File:** `src/game/systems/dialogue.rs`

Add integration tests:

```rust
#[test]
fn test_recruit_to_inn_success() {
    // Arrange: Create GameState with valid innkeeper and character
    // Act: Execute RecruitToInn action
    // Assert: Character added to roster at inn location
}

#[test]
fn test_recruit_to_inn_invalid_innkeeper() {
    // Arrange: Create GameState without target innkeeper
    // Act: Execute RecruitToInn action
    // Assert: Roster unchanged, error logged
}

#[test]
fn test_recruit_to_inn_roster_full() {
    // Arrange: Create GameState with MAX_CHARACTERS in roster
    // Act: Execute RecruitToInn action
    // Assert: Roster unchanged, "Roster is full" logged
}
```

#### 3.4 Post-Implementation Validation

Run all quality checks:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**Expected:** All checks pass, 6 new tests passing

#### 3.5 Deliverables

- [ ] `src/domain/character.rs`: Roster::add_to_inn() method added
- [ ] `src/domain/character.rs`: Roster::characters_at_inn() method added
- [ ] `src/domain/character.rs`: 3 new unit tests for inn roster methods
- [ ] `src/game/systems/dialogue.rs`: RecruitToInn action fully implemented
- [ ] `src/game/systems/dialogue.rs`: 3 new integration tests for inn recruitment
- [ ] Innkeeper validation integrated with NpcDatabase
- [ ] All quality gates pass

#### 3.6 Success Criteria

- ✅ Characters successfully added to roster at inn location
- ✅ Innkeeper ID validation prevents invalid inn assignments
- ✅ Roster size limit enforced (max 18 characters)
- ✅ CharacterLocation::AtInn properly tracks which inn
- ✅ characters_at_inn() query works correctly
- ✅ All tests pass with 100% success rate

---

### Phase 4: MapEvent Removal After Recruitment

#### 4.1 Add Map Event Removal API

**File:** `src/domain/world/map.rs`

Add method to remove events by position:

```rust
impl Map {
    /// Remove a map event at the specified position
    ///
    /// Returns true if an event was removed, false if no event existed at that position
    pub fn remove_event(&mut self, position: Position) -> bool {
        self.events.remove(&position).is_some()
    }

    /// Remove a specific event type at position (type-safe removal)
    pub fn remove_recruitable_character(&mut self, position: Position, character_id: &str) -> bool {
        if let Some(MapEvent::RecruitableCharacter { character_id: id, .. }) = self.events.get(&position) {
            if id == character_id {
                return self.remove_event(position);
            }
        }
        false
    }
}
```

#### 4.2 Track Current Interaction Position

**File:** `src/game/systems/dialogue.rs`

Add field to DialogueState to track where recruitment was triggered:

```rust
pub struct DialogueState {
    pub active_tree_id: Option<DialogueId>,
    pub current_node_id: NodeId,
    pub interaction_position: Option<Position>,  // NEW: Track where dialogue was triggered
}
```

**File:** `src/game/systems/input.rs`

Update recruitable character interaction to store position:

```rust
MapEvent::RecruitableCharacter { name, character_id, .. } => {
    info!("Interacting with recruitable character '{}' at {:?}", name, position);

    // Store position in dialogue state for later removal
    dialogue_writer.write(StartDialogue {
        dialogue_id: 100,
        interaction_position: Some(position),  // NEW
    });
    return;
}
```

#### 4.3 Remove MapEvent After Successful Recruitment

**File:** `src/game/systems/dialogue.rs`

Update both RecruitToParty and RecruitToInn to remove map event:

```rust
// After successful recruitment (both actions)
info!("Successfully recruited '{}'", name);

// Remove recruitable character event from map
if let Some(pos) = dialogue_state.interaction_position {
    if let Some(map) = game_state.world.get_current_map_mut() {
        if map.remove_recruitable_character(pos, character_id) {
            info!("Removed recruitable character event at {:?}", pos);
        } else {
            warn!("Failed to remove recruitable character event at {:?}", pos);
        }
    }
}
```

#### 4.4 Testing Requirements

**File:** `src/domain/world/map.rs`

Add unit tests:

```rust
#[test]
fn test_remove_event_success() {
    // Arrange: Create map with event at position
    // Act: remove_event(position)
    // Assert: Returns true, event no longer in map
}

#[test]
fn test_remove_event_not_found() {
    // Arrange: Create empty map
    // Act: remove_event(position)
    // Assert: Returns false, map unchanged
}

#[test]
fn test_remove_recruitable_character_type_safe() {
    // Arrange: Create map with recruitable character event
    // Act: remove_recruitable_character(position, character_id)
    // Assert: Event removed only if character_id matches
}
```

**File:** `src/game/systems/dialogue.rs`

Add integration test:

```rust
#[test]
fn test_recruitment_removes_map_event() {
    // Arrange: Create GameState with map containing recruitable character
    // Act: Execute RecruitToParty action with interaction_position set
    // Assert: MapEvent no longer exists at that position
}
```

#### 4.5 Post-Implementation Validation

Run all quality checks and manual verification:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
cargo run  # Manual test: recruit character, verify green marker disappears
```

**Expected:** All checks pass, map marker disappears after recruitment

#### 4.6 Deliverables

- [ ] `src/domain/world/map.rs`: remove_event() method added
- [ ] `src/domain/world/map.rs`: remove_recruitable_character() method added
- [ ] `src/domain/world/map.rs`: 3 new unit tests for event removal
- [ ] `src/game/systems/dialogue.rs`: DialogueState.interaction_position field added
- [ ] `src/game/systems/input.rs`: StartDialogue includes interaction position
- [ ] `src/game/systems/dialogue.rs`: MapEvent removal integrated into recruitment actions
- [ ] `src/game/systems/dialogue.rs`: 1 new integration test for event removal
- [ ] All quality gates pass
- [ ] Manual verification: green markers disappear after recruitment

#### 4.7 Success Criteria

- ✅ MapEvent removed from map after successful recruitment
- ✅ Green visual marker disappears from map
- ✅ Re-visiting same tile shows no recruitable character
- ✅ Type-safe removal prevents accidental deletion of wrong events
- ✅ All tests pass with 100% success rate

---

### Phase 5: Dynamic Dialogue Variable Substitution

#### 5.1 Add Variable Substitution Context

**File:** `src/domain/dialogue.rs`

Add context struct for dialogue variables:

```rust
/// Context for dialogue variable substitution
pub struct DialogueContext {
    pub variables: HashMap<String, String>,
}

impl DialogueContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
    }

    pub fn substitute(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (key, value) in &self.variables {
            let placeholder = format!("{{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}
```

#### 5.2 Integrate Variable Substitution into Dialogue System

**File:** `src/game/systems/dialogue.rs`

Add DialogueContext to StartDialogue message:

```rust
pub struct StartDialogue {
    pub dialogue_id: DialogueId,
    pub interaction_position: Option<Position>,
    pub context: Option<DialogueContext>,  // NEW
}
```

**File:** `src/game/systems/input.rs`

Create context with character name when triggering recruitment dialogue:

```rust
MapEvent::RecruitableCharacter { name, character_id, .. } => {
    info!("Interacting with recruitable character '{}'", name);

    // Create dialogue context with character name
    let mut context = DialogueContext::new();
    context.set("CHARACTER_NAME", name);
    context.set("CHARACTER_ID", character_id);

    dialogue_writer.write(StartDialogue {
        dialogue_id: 100,
        interaction_position: Some(position),
        context: Some(context),
    });
    return;
}
```

**File:** `src/game/systems/dialogue.rs`

Apply substitution when displaying dialogue text:

```rust
fn handle_start_dialogue(/* ... */) {
    for ev in ev_reader.read() {
        // ... load dialogue tree ...

        if let Some(node) = tree.get_node(root) {
            // Apply variable substitution if context provided
            let text = if let Some(ref context) = ev.context {
                context.substitute(&node.text)
            } else {
                node.text.clone()
            };

            if let Some(ref mut log) = game_log {
                let speaker = tree.speaker_name.as_deref().unwrap_or("NPC");
                log.add(format!("{}: {}", speaker, text));
            }
        }
    }
}
```

#### 5.3 Update Default Recruitment Dialogue

**File:** `campaigns/tutorial/data/dialogues.ron`

Verify dialogue uses {CHARACTER_NAME} placeholder:

```ron
(
    id: 100,
    name: "Default Character Recruitment",
    root_node: 1,
    nodes: {
        1: (
            text: "Hello there. My name is {CHARACTER_NAME}. Can I join your party?",
            // ... rest of dialogue unchanged ...
        ),
    },
)
```

#### 5.4 Testing Requirements

**File:** `src/domain/dialogue.rs`

Add unit tests:

```rust
#[test]
fn test_dialogue_context_substitute_single_variable() {
    let mut context = DialogueContext::new();
    context.set("NAME", "Kira");
    assert_eq!(context.substitute("Hello, {NAME}!"), "Hello, Kira!");
}

#[test]
fn test_dialogue_context_substitute_multiple_variables() {
    let mut context = DialogueContext::new();
    context.set("NAME", "Kira");
    context.set("CLASS", "Knight");
    let result = context.substitute("{NAME} the {CLASS}");
    assert_eq!(result, "Kira the Knight");
}

#[test]
fn test_dialogue_context_substitute_no_variables() {
    let context = DialogueContext::new();
    assert_eq!(context.substitute("No variables"), "No variables");
}
```

**File:** `src/game/systems/dialogue.rs`

Add integration test:

```rust
#[test]
fn test_recruitment_dialogue_substitutes_character_name() {
    // Arrange: Create dialogue with {CHARACTER_NAME} placeholder
    // Act: Trigger StartDialogue with context containing CHARACTER_NAME
    // Assert: Game log shows substituted text with actual character name
}
```

#### 5.5 Post-Implementation Validation

Run all quality checks and manual verification:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
cargo run  # Manual test: interact with recruitable character, verify name displayed
```

**Expected:** All checks pass, dialogue shows actual character name

#### 5.6 Deliverables

- [ ] `src/domain/dialogue.rs`: DialogueContext struct added
- [ ] `src/domain/dialogue.rs`: Variable substitution methods implemented
- [ ] `src/domain/dialogue.rs`: 3 new unit tests for context substitution
- [ ] `src/game/systems/dialogue.rs`: StartDialogue includes optional context
- [ ] `src/game/systems/dialogue.rs`: handle_start_dialogue applies substitution
- [ ] `src/game/systems/input.rs`: Context created with character name for recruitment
- [ ] `src/game/systems/dialogue.rs`: 1 new integration test for substitution
- [ ] `campaigns/tutorial/data/dialogues.ron`: Recruitment dialogue verified
- [ ] All quality gates pass
- [ ] Manual verification: character name appears in dialogue

#### 5.7 Success Criteria

- ✅ {CHARACTER_NAME} placeholder replaced with actual character name
- ✅ Dialogue displays: "Hello there. My name is Kira. Can I join your party?"
- ✅ Variable substitution works for any dialogue node text
- ✅ Missing variables leave placeholders unchanged (graceful degradation)
- ✅ All tests pass with 100% success rate

---

### Phase 6: HUD Refresh Integration

#### 6.1 Add HUD Refresh Message

**File:** `src/game/systems/hud.rs`

Define message to trigger HUD rebuild:

```rust
/// Message to request HUD refresh (e.g., after party changes)
#[derive(Message, Clone, Debug)]
pub struct RefreshHud;
```

**File:** `src/game/systems/hud.rs`

Add system to handle refresh messages:

```rust
fn handle_hud_refresh(
    mut commands: Commands,
    mut refresh_events: MessageReader<RefreshHud>,
    existing_hud: Query<Entity, With<HudRoot>>,
    global_state: Res<GlobalState>,
    // ... other resources needed for spawn_hud
) {
    if refresh_events.read().count() == 0 {
        return;
    }

    info!("Refreshing HUD due to party changes");

    // Despawn existing HUD
    for entity in existing_hud.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Respawn HUD with updated party
    spawn_hud(commands, global_state, /* ... */);
}
```

Register in HudPlugin:

```rust
impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RefreshHud>()
            .add_systems(Startup, spawn_hud)
            .add_systems(Update, (update_hud, handle_hud_refresh));
    }
}
```

#### 6.2 Trigger HUD Refresh After Recruitment

**File:** `src/game/systems/dialogue.rs`

Add MessageWriter parameter and send refresh message:

```rust
fn execute_action(
    // ... existing parameters ...
    mut hud_refresh: Option<MessageWriter<RefreshHud>>,
) {
    // ... existing code ...

    DialogueAction::RecruitToParty { character_id } => {
        // ... recruitment logic ...

        if successful {
            // Trigger HUD refresh to show new party member
            if let Some(ref mut refresh) = hud_refresh {
                refresh.write(RefreshHud);
            }
        }
    }
}
```

#### 6.3 Testing Requirements

**File:** `src/game/systems/hud.rs`

Add unit tests:

```rust
#[test]
fn test_hud_refresh_despawns_existing() {
    // Arrange: Create app with spawned HUD
    // Act: Send RefreshHud message
    // Assert: Old HUD entities despawned
}

#[test]
fn test_hud_refresh_respawns_with_new_party() {
    // Arrange: Create app with 3-member party
    // Act: Add 4th member, send RefreshHud
    // Assert: HUD shows 4 character cards
}
```

**File:** `src/game/systems/dialogue.rs`

Add integration test:

```rust
#[test]
fn test_recruitment_triggers_hud_refresh() {
    // Arrange: Create app with dialogue and HUD systems
    // Act: Execute RecruitToParty action
    // Assert: RefreshHud message sent
}
```

#### 6.4 Post-Implementation Validation

Run all quality checks and manual verification:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
cargo run  # Manual test: recruit character, verify HUD updates immediately
```

**Expected:** All checks pass, HUD updates to show new party member

#### 6.5 Deliverables

- [ ] `src/game/systems/hud.rs`: RefreshHud message type added
- [ ] `src/game/systems/hud.rs`: handle_hud_refresh system implemented
- [ ] `src/game/systems/hud.rs`: 2 new unit tests for HUD refresh
- [ ] `src/game/systems/dialogue.rs`: execute_action sends RefreshHud after recruitment
- [ ] `src/game/systems/dialogue.rs`: 1 new integration test for HUD refresh trigger
- [ ] All quality gates pass
- [ ] Manual verification: HUD updates immediately after recruitment

#### 6.6 Success Criteria

- ✅ HUD refreshes immediately when character recruited to party
- ✅ New party member appears in HUD character cards
- ✅ Old HUD entities properly despawned (no memory leak)
- ✅ HUD rebuild uses current party state from GlobalState
- ✅ All tests pass with 100% success rate

---

## Final Validation Checklist

### Code Quality

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes with 0 errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes with 0 warnings
- [ ] `cargo nextest run --all-features` passes with 100% success rate
- [ ] All TODO comments removed from recruitment action handlers
- [ ] SPDX headers present in all modified files

### Architecture Compliance

- [ ] CharacterDatabase properly integrated into GameContent
- [ ] Party size limit enforced (max 6 members)
- [ ] Roster size limit enforced (max 18 characters)
- [ ] CharacterLocation properly tracks party vs inn vs map
- [ ] MapEvent removal uses type-safe API
- [ ] Dialogue variable substitution extensible for future use
- [ ] HUD refresh uses message-based architecture (no direct coupling)

### Testing

- [ ] Unit tests added for all new public APIs
- [ ] Integration tests verify end-to-end recruitment flow
- [ ] Error cases tested (party full, roster full, character not found)
- [ ] Edge cases tested (boundary conditions, invalid data)
- [ ] Test coverage >80% for new code

### Documentation

- [ ] `docs/explanation/implementations.md` updated with recruitment system summary
- [ ] Code comments document complex logic and design decisions
- [ ] Public APIs have doc comments with examples
- [ ] Known limitations documented in implementation summary

### Manual Verification

**Test 1: Recruit to Party (Success)**
1. Start game, navigate to recruitable character (green marker)
2. Press E to interact
3. Select "Yes, join us!" dialogue option
4. Verify: Character added to party, HUD shows new member, green marker disappears

**Test 2: Recruit to Party (Full)**
1. Fill party with 6 members
2. Navigate to recruitable character, press E
3. Select "Yes, join us!"
4. Verify: Error message "Party is full!", character not added, marker remains

**Test 3: Recruit to Inn (Success)**
1. Start game, navigate to recruitable character
2. Press E to interact
3. Select "Meet me at the Inn."
4. Verify: Success message, character added to roster at inn, marker disappears

**Test 4: Recruit to Inn (Roster Full)**
1. Fill roster with 18 characters
2. Navigate to recruitable character, press E
3. Select "Meet me at the Inn."
4. Verify: Error message "Roster is full!", character not added, marker remains

**Test 5: Dialogue Variable Substitution**
1. Navigate to recruitable character named "Kira"
2. Press E to interact
3. Verify: Dialogue shows "Hello there. My name is Kira. Can I join your party?"

**Test 6: Character Not Found**
1. Create recruitable character event with invalid character_id
2. Press E to interact, select recruitment option
3. Verify: Error message displayed, no crash, marker remains

---

## Implementation Order Summary

Recommended implementation order:

1. **Phase 1**: Character Definition Loading (foundation for all recruitment)
2. **Phase 2**: Party Recruitment (most common use case, immediate value)
3. **Phase 3**: Inn Roster Recruitment (extends roster system)
4. **Phase 4**: MapEvent Removal (prevents duplicate recruitment)
5. **Phase 5**: Dialogue Variables (improves UX, non-blocking)
6. **Phase 6**: HUD Refresh (polish, immediate visual feedback)

**Total Estimated Effort:** 3-4 development sessions (assuming 2-3 hours per phase)

**Dependencies:**
- Phase 2, 3, 4, 5, 6 all depend on Phase 1
- Phase 6 depends on Phase 2 (no HUD refresh needed for inn recruitment)
- Phases 2-5 can be implemented in parallel if desired

---

## Post-Completion Documentation Update

After all phases complete, update `docs/explanation/implementations.md`:

```markdown
## Character Recruitment System - COMPLETED (2025-01-XX)

### Summary

Implemented full character recruitment system enabling players to recruit characters from maps into active party or send to inns. System integrates CharacterDatabase loading, party/roster management, dialogue variable substitution, MapEvent removal, and HUD refresh.

### Phases Completed

- ✅ Phase 1: Character Definition Loading
- ✅ Phase 2: Party Recruitment Implementation
- ✅ Phase 3: Inn Roster Recruitment Implementation
- ✅ Phase 4: MapEvent Removal After Recruitment
- ✅ Phase 5: Dynamic Dialogue Variable Substitution
- ✅ Phase 6: HUD Refresh Integration

### Files Modified

- `src/application/resources.rs` - CharacterDatabase integration
- `src/game/systems/dialogue.rs` - Recruitment action handlers
- `src/game/systems/input.rs` - Dialogue context creation
- `src/game/systems/hud.rs` - HUD refresh system
- `src/domain/character.rs` - Inn roster management
- `src/domain/world/map.rs` - Event removal API
- `src/domain/dialogue.rs` - Variable substitution
- `campaigns/tutorial/data/dialogues.ron` - Recruitment dialogue

### Validation Results

- All cargo quality gates passed (fmt, check, clippy, nextest)
- XX new tests added, 100% passing
- Manual verification completed for all success and error cases
```

---

## Design Decisions (Resolved)

**Q1: Should inn roster be part of Roster struct or separate InnRoster struct?**
**A1:** Use Roster struct with CharacterLocation::AtInn tracking (simpler, reuses existing infrastructure)

**Q2: How should dialogue variables be scoped (global vs per-dialogue)?**
**A2:** Per-dialogue context passed with StartDialogue message (more flexible, no global state)

**Q3: Should HUD refresh be automatic or message-triggered?**
**A3:** Message-triggered via RefreshHud message (decouples systems, testable)

**Q4: How to prevent duplicate recruitment?**
**A4:** Remove MapEvent after successful recruitment (simplest, prevents all edge cases)

**Q5: What happens if character database doesn't contain recruitable character?**
**A5:** Display error message in game log, recruitment fails gracefully, marker remains

---

## Known Limitations (Post-Implementation)

- Character portraits not yet assigned from recruitable character definitions
- No party management UI to move characters between party and inn roster
- No way to dismiss/delete recruited characters from roster
- Inn capacity limits not enforced (all inns can hold unlimited characters)
- Recruited characters don't gain experience or equipment from recruitment dialogue
- No save/load integration for recruited character state persistence
