# Character Recruitment Events Implementation Plan

## Overview

This plan implements character recruitment through the dialogue system, enabling players to recruit pre-defined characters to their party or send them to inns via interactive dialogues. The implementation integrates with existing systems: MapEvent::RecruitableCharacter events, DialogueAction execution, and the party management system.

**Key Features**:

- Dialogue-driven recruitment using existing dialogue system infrastructure
- Automatic party capacity handling (recruit to party if space, otherwise send to inn)
- Event cleanup after successful recruitment (one-time encounters)
- Duplicate recruitment prevention via encountered_characters tracking
- Full integration with Campaign Builder SDK for content authoring

**Architecture Compliance**:

- Uses existing `GameState::recruit_from_map()` (src/application/mod.rs:L838-884)
- Leverages `MapEvent::RecruitableCharacter` (src/domain/world/types.rs:L485-492)
- Integrates with `DialogueAction::RecruitToParty` and `DialogueAction::RecruitToInn` (src/domain/dialogue.rs:L419-428)
- Respects `PARTY_MAX_SIZE = 6` constant (src/domain/character.rs:L114)
- Maintains `Roster::MAX_CHARACTERS = 18` limit (src/domain/character.rs:L1244)

**Total Estimated Duration**: 20-26 hours across 4 phases

---

## Current State Analysis

### Existing Infrastructure

#### Core Recruitment Logic (src/application/mod.rs)

**Function**: `GameState::recruit_from_map()` (L838-884)

- **Status**: ✅ IMPLEMENTED
- **Capabilities**:
  - Validates character_id exists in database
  - Checks duplicate recruitment via `encountered_characters` HashSet
  - Instantiates character from CharacterDefinition
  - Adds to party if space available (< 6 members)
  - Sends to nearest inn if party full
  - Updates roster with CharacterLocation tracking
- **Returns**: `Result<RecruitResult, RecruitmentError>`

**Error Types**: `RecruitmentError` enum (L364-379)

- `AlreadyEncountered(String)` - Character already recruited
- `CharacterNotFound(String)` - character_id not in database
- `CharacterDefinition(CharacterDefinitionError)` - Definition parsing error
- `CharacterError(CharacterError)` - Character operation error
- `PartyManager(PartyManagementError)` - Party management error

**Return Types**: `RecruitResult` enum (L383-392)

- `AddedToParty` - Character joined active party
- `SentToInn(InnkeeperId)` - Party full, sent to inn
- `Declined` - Player declined recruitment (not yet used)

#### Map Events (src/domain/world/types.rs)

**Variant**: `MapEvent::RecruitableCharacter` (L485-492)

```rust
RecruitableCharacter {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    character_id: String,
    // NOTE: No dialogue_id field currently exists
}
```

**Event Handling**: `src/game/systems/events.rs::handle_events()` (L154-166)

- **Status**: ✅ PARTIAL - Logs encounter, does NOT trigger recruitment
- **Current Behavior**: Prints message, logs to game log, TODO comment for recruitment dialog

#### Dialogue System (src/domain/dialogue.rs)

**Actions**: `DialogueAction` enum (L386-428)

- **Status**: ✅ DEFINED
- `RecruitToParty { character_id: String }` (L419)
- `RecruitToInn { character_id: String, innkeeper_id: String }` (L423-426)

**Execution**: `src/game/systems/dialogue.rs::execute_action()` (L501-600)

- **Status**: ⚠️ TODO STUBS - Contains placeholder comments at L588-602
- **Current Implementation**: Logs info message, does nothing

#### Character Database (src/domain/character_definition.rs)

**Database**: `CharacterDatabase`

- **Status**: ✅ IMPLEMENTED
- `get_character(id: &str) -> Option<&CharacterDefinition>`
- `load_from_file(path: &Path) -> Result<Self, CharacterDefinitionError>`

**Instantiation**: `CharacterDefinition::instantiate()` (architecture.md:L1060-1075)

- **Status**: ✅ IMPLEMENTED
- Converts template to live Character instance
- Applies race/class modifiers
- Initializes HP/SP, inventory, equipment

### Identified Issues

#### Issue 1: Dialogue Actions Not Wired to recruit_from_map()

**Location**: `src/game/systems/dialogue.rs::execute_action()` (L588-602)
**Problem**: RecruitToParty and RecruitToInn actions contain TODO comments, no actual implementation
**Impact**: Players cannot recruit characters through dialogues
**Required Fix**: Call `game_state.recruit_from_map()` from action execution

#### Issue 2: MapEvent Cleanup Missing

**Location**: Event removal after recruitment
**Problem**: No mechanism to remove MapEvent::RecruitableCharacter after successful recruitment
**Impact**: Players can recruit same character multiple times (though `encountered_characters` prevents it)
**Required Fix**: Store event position in recruitment context, remove event after recruitment

#### Issue 3: RecruitableCharacter Events Don't Trigger Dialogue

**Location**: `src/game/systems/events.rs::handle_events()` (L154-166)
**Problem**: RecruitableCharacter case logs message but doesn't start dialogue
**Impact**: No player interaction, automatic recruitment impossible
**Required Fix**: Add dialogue_id field to RecruitableCharacter, trigger dialogue system

#### Issue 4: No Recruitment Context Tracking

**Location**: Dialogue state management
**Problem**: When dialogue starts, no way to know it's a recruitment dialogue or which event triggered it
**Impact**: Cannot remove event after recruitment, cannot pass character_id to action
**Required Fix**: Add RecruitmentContext to DialogueState

#### Issue 5: SDK Validation Missing

**Location**: `sdk/campaign_builder/src/validation.rs`
**Problem**: No validation that character_id references in RecruitableCharacter events exist in database
**Impact**: Runtime errors when stepping on invalid recruitment events
**Required Fix**: Add validation function for character_id and dialogue_id references

---

## Implementation Phases

### Phase 0: Architecture Decision and Preparation

**Objective**: Resolve design questions and prepare implementation environment

**Duration**: 1-2 hours

**Status**: ✅ COMPLETE - Option B chosen

#### Task 0.1: Verify Tool Installation

**Owner**: AI Agent

**Status**: Ready to execute

**Commands**:

```bash
cd antares
rustup component add clippy rustfmt
cargo install cargo-nextest --locked
```

**Validation**:

- [ ] `rustup component list | grep -E "(clippy|rustfmt).*installed"` shows both installed
- [ ] `cargo nextest --version` shows version >= 0.9.0

#### Task 0.2: Architecture Decision - Recruitment Dialogue Pattern

**Owner**: Human User (AI presents options, human decides)

**Status**: ✅ COMPLETE - **Option B chosen by user**

**Decision Required**: Choose ONE recruitment interaction pattern

**Option A: Use Existing NpcDialogue Events (RECOMMENDED)**

**Rationale**:

- Reuses existing NpcDialogue infrastructure
- No new MapEvent fields needed
- NPCs can have multiple purposes (dialogue, quests, recruitment)
- Consistent with architecture pattern

**Implementation**:

1. Add recruitable characters to NPC database with `is_recruitable: bool` flag
2. Create recruitment dialogue trees with RecruitToParty/RecruitToInn actions
3. Use `MapEvent::NpcDialogue` with recruitment dialogues
4. Remove `MapEvent::RecruitableCharacter` entirely (breaking change)

**Migration Impact**: HIGH - Existing RecruitableCharacter events must convert to NpcDialogue

**Option B: Add dialogue_id to RecruitableCharacter (CURRENT PLAN)**

**Rationale**:

- Keeps recruitment events distinct from generic NPCs
- Explicit separation of concerns
- Easier for content authors to identify recruitable characters

**Implementation**:

1. Add `dialogue_id: Option<DialogueId>` field to MapEvent::RecruitableCharacter
2. Default to None (simple yes/no confirmation UI)
3. When specified, trigger dialogue with recruitment actions

**Migration Impact**: LOW - Backwards compatible (dialogue_id is optional)

**Option C: Simple Confirmation UI (NO DIALOGUE)**

**Rationale**:

- Simplest implementation
- No dialogue system dependency
- Direct recruitment on confirmation

**Implementation**:

1. Show simple yes/no prompt when stepping on RecruitableCharacter
2. Call recruit_from_map() directly on "yes"
3. No dialogue trees needed

**Migration Impact**: NONE - No existing content to migrate

**Decision Criteria**:

- Content authoring complexity preference
- Player experience goals (rich dialogue vs. quick recruitment)
- Code maintenance burden

**Deliverable**: Decision documented in this plan file (update "CHOSEN OPTION:" below)

**CHOSEN OPTION:** **Option B** - Add dialogue_id to RecruitableCharacter

**Decision Rationale**:

- Backwards compatible (dialogue_id is optional, defaults to None)
- Keeps recruitment events distinct from generic NPCs
- Lower migration cost for existing content (no breaking changes)
- Explicit separation of concerns (recruitment vs. general NPC interaction)
- Allows simple confirmation UI when dialogue_id is None (future enhancement)

**Implementation Path**: Proceed with Phases 1-4 as specified in this plan.

---

### Phase 1: Core Recruitment Action Implementation

**Objective**: Implement working recruitment logic in dialogue action execution system

**Duration**: 6-8 hours

**Prerequisites**: Phase 0 complete, architecture decision made

**Dependencies**: Assumes Option B chosen (add dialogue_id field)

---

#### Task 1.1: Add dialogue_id Field to MapEvent::RecruitableCharacter

**File**: `src/domain/world/types.rs`

**Location**: MapEvent enum, RecruitableCharacter variant (L485-492)

**Changes**:

1. Modify RecruitableCharacter variant:

```rust
// BEFORE (L485-492):
RecruitableCharacter {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    character_id: String,
}

// AFTER:
RecruitableCharacter {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    character_id: String,
    /// Optional dialogue tree for recruitment interaction
    #[serde(default)]
    dialogue_id: Option<DialogueId>,
}
```

2. Add constant after MapEvent enum definition (around L507):

```rust
/// Default dialogue ID for recruitment events when none specified
pub const DEFAULT_RECRUITMENT_DIALOGUE_ID: DialogueId = 1000;
```

**Validation**:

- [ ] `cargo check --all-targets --all-features` - Compiles with no errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` - Zero warnings
- [ ] `cargo nextest run test_map_event` - Map event tests pass
- [ ] Manual: Create test RON file with RecruitableCharacter, verify dialogue_id field accepted

**Deliverables**:

- [ ] dialogue_id field added to RecruitableCharacter variant
- [ ] DEFAULT_RECRUITMENT_DIALOGUE_ID constant defined
- [ ] All quality checks pass

---

#### Task 1.2: Add RecruitmentContext to DialogueState

**File**: `src/application/dialogue.rs`

**Location**: Before DialogueState struct definition (around L40)

**Changes**:

1. Add RecruitmentContext struct definition:

```rust
// Add at line ~40, before DialogueState struct

/// Context information for character recruitment dialogues
///
/// Stores metadata needed to complete recruitment after dialogue concludes.
/// This includes the character being recruited and the map event position
/// for cleanup after successful recruitment.
#[derive(Debug, Clone)]
pub struct RecruitmentContext {
    /// ID of the character definition being recruited
    pub character_id: String,

    /// Position of the recruitment event on the map (for removal after recruitment)
    pub event_position: crate::domain::types::Position,
}
```

2. Modify DialogueState struct (L46-67), add field after speaker_entity:

```rust
pub struct DialogueState {
    pub active_tree_id: Option<DialogueId>,
    pub current_node_id: NodeId,
    pub dialogue_history: Vec<NodeId>,
    pub current_node_text: String,
    pub speaker_name: String,
    pub choices: Vec<ChoiceText>,
    pub speaker_entity: Entity,

    // NEW FIELD:
    /// Context for recruitment dialogues (None if not a recruitment interaction)
    pub recruitment_context: Option<RecruitmentContext>,
}
```

3. Update DialogueState::default() implementation to initialize new field:

```rust
impl Default for DialogueState {
    fn default() -> Self {
        Self {
            active_tree_id: None,
            current_node_id: 0,
            dialogue_history: Vec::new(),
            current_node_text: String::new(),
            speaker_name: String::new(),
            choices: Vec::new(),
            speaker_entity: Entity::PLACEHOLDER,
            recruitment_context: None,  // NEW
        }
    }
}
```

**Validation**:

- [ ] `cargo check --all-targets --all-features` - Compiles
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` - Zero warnings
- [ ] `cargo nextest run test_dialogue_state` - DialogueState tests pass
- [ ] Test verifies default() initializes recruitment_context to None

**Deliverables**:

- [ ] RecruitmentContext struct defined
- [ ] recruitment_context field added to DialogueState
- [ ] Default implementation updated
- [ ] All quality checks pass

---

#### Task 1.3: Create Bevy Resource for Pending Recruitment Context

**File**: `src/game/systems/dialogue.rs`

**Location**: After imports, before StartDialogue struct (around L30)

**Changes**:

1. Add resource definition:

```rust
// Add after imports, around line 30

/// Temporary storage for recruitment context during dialogue initialization
///
/// This resource holds recruitment context between the moment a RecruitableCharacter
/// event is triggered and when the dialogue system initializes DialogueState.
/// The context is consumed by handle_start_dialogue and cleared after use.
#[derive(Resource, Default)]
pub struct PendingRecruitmentContext(pub Option<crate::application::dialogue::RecruitmentContext>);
```

2. Register resource in DialoguePlugin::build() (L56-89):

```rust
impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<StartDialogue>()
            .add_message::<SelectDialogueChoice>()
            .add_message::<AdvanceDialogue>()
            .insert_resource(PendingRecruitmentContext::default())  // NEW LINE
            .add_systems(
                Update,
                (
                    dialogue_input_system,
                    handle_start_dialogue,
                    handle_select_choice,
                ),
            );
    }
}
```

**Validation**:

- [ ] `cargo check --all-targets --all-features` - Compiles
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` - Zero warnings
- [ ] `cargo nextest run test_dialogue_plugin` - Dialogue plugin tests pass
- [ ] Test verifies PendingRecruitmentContext resource exists after plugin initialization

**Deliverables**:

- [ ] PendingRecruitmentContext resource defined
- [ ] Resource registered in DialoguePlugin
- [ ] All quality checks pass

---

#### Task 1.4: Implement RecruitToParty Dialogue Action

**File**: `src/game/systems/dialogue.rs`

**Location**: execute_action() function (L501-600), RecruitToParty case (L588-594)

**Changes**:

Replace TODO comment block with full implementation:

```rust
DialogueAction::RecruitToParty { character_id } => {
    info!("Executing RecruitToParty action for character '{}'", character_id);

    // Call core recruitment logic
    match game_state.recruit_from_map(character_id, db) {
        Ok(RecruitResult::AddedToParty) => {
            info!("Successfully recruited '{}' to active party", character_id);
            if let Some(ref mut log) = game_log {
                if let Some(char_def) = db.characters.get_character(character_id) {
                    log.add(format!("{} joins the party!", char_def.name));
                } else {
                    log.add(format!("{} joins the party!", character_id));
                }
            }
        }
        Ok(RecruitResult::SentToInn(inn_id)) => {
            info!("Party full - sent '{}' to inn '{}'", character_id, inn_id);
            if let Some(ref mut log) = game_log {
                if let Some(char_def) = db.characters.get_character(character_id) {
                    log.add(format!(
                        "Party is full! {} will wait at the inn.",
                        char_def.name
                    ));
                } else {
                    log.add(format!("Party is full! {} sent to inn.", character_id));
                }
            }
        }
        Ok(RecruitResult::Declined) => {
            // Not currently used by recruit_from_map
            info!("Recruitment declined for '{}'", character_id);
        }
        Err(RecruitmentError::AlreadyEncountered(id)) => {
            warn!("Cannot recruit '{}': already encountered", id);
            if let Some(ref mut log) = game_log {
                log.add(format!("{} has already joined your adventure.", id));
            }
        }
        Err(RecruitmentError::CharacterNotFound(id)) => {
            error!("Character definition '{}' not found in database", id);
            if let Some(ref mut log) = game_log {
                log.add(format!("Error: Character '{}' not found.", id));
            }
        }
        Err(RecruitmentError::CharacterDefinition(err)) => {
            error!("Character definition error for '{}': {}", character_id, err);
            if let Some(ref mut log) = game_log {
                log.add(format!("Error loading character: {}", err));
            }
        }
        Err(RecruitmentError::CharacterError(err)) => {
            error!("Character operation error for '{}': {}", character_id, err);
            if let Some(ref mut log) = game_log {
                log.add(format!("Error: {}", err));
            }
        }
        Err(RecruitmentError::PartyManager(err)) => {
            error!("Party management error for '{}': {}", character_id, err);
            if let Some(ref mut log) = game_log {
                log.add(format!("Error: {}", err));
            }
        }
    }
}
```

**Validation**:

- [ ] `cargo check --all-targets --all-features` - Compiles
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` - Zero warnings
- [ ] `cargo nextest run test_recruit_to_party_action` - Test passes
- [ ] Manual test: Trigger RecruitToParty action, verify character added to party
- [ ] Manual test: Full party, verify character sent to inn

**Test Cases Required**:

**Test 1**: Successful recruitment to party

```rust
#[test]
fn test_recruit_to_party_action_success() {
    // Arrange
    let mut game_state = GameState::default();
    let db = create_test_database_with_character("test_knight");

    // Act
    execute_action(
        &DialogueAction::RecruitToParty {
            character_id: "test_knight".to_string()
        },
        &mut game_state,
        &db,
        None,
        None,
    );

    // Assert
    assert_eq!(game_state.party.size(), 1);
    assert_eq!(game_state.party.members[0].name, "Test Knight");
    assert!(game_state.encountered_characters.contains("test_knight"));
}
```

**Test 2**: Party full, send to inn

```rust
#[test]
fn test_recruit_to_party_action_when_party_full() {
    // Arrange
    let mut game_state = GameState::default();
    fill_party_to_capacity(&mut game_state); // 6 members
    let db = create_test_database_with_character("test_mage");

    // Act
    execute_action(
        &DialogueAction::RecruitToParty {
            character_id: "test_mage".to_string()
        },
        &mut game_state,
        &db,
        None,
        None,
    );

    // Assert
    assert_eq!(game_state.party.size(), 6); // Still at max
    assert_eq!(game_state.roster.characters.len(), 7); // 6 in party + 1 at inn
    assert!(matches!(
        game_state.roster.character_locations.last(),
        Some(CharacterLocation::AtInn(_))
    ));
}
```

**Test 3**: Already recruited (duplicate prevention)

```rust
#[test]
fn test_recruit_to_party_action_already_recruited() {
    // Arrange
    let mut game_state = GameState::default();
    let db = create_test_database_with_character("test_knight");

    // First recruitment
    game_state.recruit_from_map("test_knight", &db).unwrap();

    // Act - attempt second recruitment
    execute_action(
        &DialogueAction::RecruitToParty {
            character_id: "test_knight".to_string()
        },
        &mut game_state,
        &db,
        None,
        None,
    );

    // Assert - party size unchanged
    assert_eq!(game_state.party.size(), 1);
    assert_eq!(game_state.roster.characters.len(), 1);
}
```

**Test 4**: Character not found in database

```rust
#[test]
fn test_recruit_to_party_action_character_not_found() {
    // Arrange
    let mut game_state = GameState::default();
    let db = ContentDatabase::new(); // Empty database

    // Act
    execute_action(
        &DialogueAction::RecruitToParty {
            character_id: "nonexistent".to_string()
        },
        &mut game_state,
        &db,
        None,
        None,
    );

    // Assert - no changes to party
    assert_eq!(game_state.party.size(), 0);
    assert_eq!(game_state.roster.characters.len(), 0);
}
```

**Deliverables**:

- [ ] RecruitToParty action implementation complete
- [ ] All 4 test cases implemented and passing
- [ ] Error handling covers all RecruitmentError variants
- [ ] Game log messages added for all outcomes
- [ ] All quality checks pass

---

#### Task 1.5: Implement RecruitToInn Dialogue Action

**File**: `src/game/systems/dialogue.rs`

**Location**: execute_action() function (L501-600), RecruitToInn case (L596-602)

**Changes**:

Replace TODO comment block with implementation:

```rust
DialogueAction::RecruitToInn { character_id, innkeeper_id } => {
    info!(
        "Executing RecruitToInn action for character '{}' at inn '{}'",
        character_id, innkeeper_id
    );

    // NOTE: recruit_from_map() handles inn assignment automatically when party is full,
    // but this action explicitly sends to a specific innkeeper regardless of party capacity.
    // We need to manually implement this logic.

    // 1. Verify character not already encountered
    if game_state.encountered_characters.contains(character_id) {
        warn!("Cannot recruit '{}': already encountered", character_id);
        if let Some(ref mut log) = game_log {
            log.add(format!("{} has already been recruited.", character_id));
        }
        return;
    }

    // 2. Verify innkeeper exists
    if db.npcs.get_npc(innkeeper_id).is_none() {
        error!("Innkeeper '{}' not found in database", innkeeper_id);
        if let Some(ref mut log) = game_log {
            log.add(format!("Error: Innkeeper '{}' not found.", innkeeper_id));
        }
        return;
    }

    // 3. Get character definition
    let char_def = match db.characters.get_character(character_id) {
        Some(def) => def,
        None => {
            error!("Character definition '{}' not found in database", character_id);
            if let Some(ref mut log) = game_log {
                log.add(format!("Error: Character '{}' not found.", character_id));
            }
            return;
        }
    };

    // 4. Instantiate character
    let character = match char_def.instantiate(&db.races, &db.classes, &db.items) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to instantiate character '{}': {}", character_id, e);
            if let Some(ref mut log) = game_log {
                log.add(format!("Error creating character: {}", e));
            }
            return;
        }
    };

    // 5. Add to roster at specified inn
    let location = crate::domain::character::CharacterLocation::AtInn(innkeeper_id.clone());
    if let Err(e) = game_state.roster.add_character(character, location) {
        error!("Failed to add character to roster: {}", e);
        if let Some(ref mut log) = game_log {
            log.add(format!("Error: {}", e));
        }
        return;
    }

    // 6. Mark as encountered
    game_state.encountered_characters.insert(character_id.to_string());

    // 7. Log success
    info!("Successfully recruited '{}' to inn '{}'", character_id, innkeeper_id);
    if let Some(ref mut log) = game_log {
        log.add(format!("{} will wait at the inn.", char_def.name));
    }
}
```

**Validation**:

- [ ] `cargo check --all-targets --all-features` - Compiles
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` - Zero warnings
- [ ] `cargo nextest run test_recruit_to_inn_action` - Test passes
- [ ] Manual test: Trigger RecruitToInn action, verify character at inn
- [ ] Manual test: Verify character location tracked correctly

**Test Cases Required**:

**Test 1**: Successful recruitment to inn

```rust
#[test]
fn test_recruit_to_inn_action_success() {
    // Arrange
    let mut game_state = GameState::default();
    let db = create_test_database_with_character_and_innkeeper("test_mage", "town_inn");

    // Act
    execute_action(
        &DialogueAction::RecruitToInn {
            character_id: "test_mage".to_string(),
            innkeeper_id: "town_inn".to_string(),
        },
        &mut game_state,
        &db,
        None,
        None,
    );

    // Assert
    assert_eq!(game_state.roster.characters.len(), 1);
    assert!(matches!(
        game_state.roster.character_locations[0],
        CharacterLocation::AtInn(ref id) if id == "town_inn"
    ));
    assert!(game_state.encountered_characters.contains("test_mage"));
}
```

**Test 2**: Duplicate recruitment prevention

```rust
#[test]
fn test_recruit_to_inn_action_already_recruited() {
    // Arrange
    let mut game_state = GameState::default();
    let db = create_test_database_with_character_and_innkeeper("test_mage", "town_inn");

    // First recruitment
    execute_action(
        &DialogueAction::RecruitToInn {
            character_id: "test_mage".to_string(),
            innkeeper_id: "town_inn".to_string(),
        },
        &mut game_state,
        &db,
        None,
        None,
    );

    // Act - second recruitment attempt
    execute_action(
        &DialogueAction::RecruitToInn {
            character_id: "test_mage".to_string(),
            innkeeper_id: "town_inn".to_string(),
        },
        &mut game_state,
        &db,
        None,
        None,
    );

    // Assert - only one character in roster
    assert_eq!(game_state.roster.characters.len(), 1);
}
```

**Test 3**: Invalid innkeeper

```rust
#[test]
fn test_recruit_to_inn_action_invalid_innkeeper() {
    // Arrange
    let mut game_state = GameState::default();
    let db = create_test_database_with_character("test_mage");
    // No innkeeper in database

    // Act
    execute_action(
        &DialogueAction::RecruitToInn {
            character_id: "test_mage".to_string(),
            innkeeper_id: "nonexistent_inn".to_string(),
        },
        &mut game_state,
        &db,
        None,
        None,
    );

    // Assert - no character added
    assert_eq!(game_state.roster.characters.len(), 0);
}
```

**Deliverables**:

- [ ] RecruitToInn action implementation complete
- [ ] All 3 test cases implemented and passing
- [ ] Explicit innkeeper validation added
- [ ] Character location tracking verified
- [ ] All quality checks pass

---

### Phase 2: Dialogue Integration and Event Triggering

**Objective**: Wire recruitment events to trigger dialogues with proper context

**Duration**: 6-8 hours

**Prerequisites**: Phase 1 complete

---

#### Task 2.1: Update Event Handler to Trigger Recruitment Dialogue

**File**: `src/game/systems/events.rs`

**Location**: handle_events() function, RecruitableCharacter case (L154-166)

**Changes**:

Replace current implementation with dialogue triggering logic:

```rust
MapEvent::RecruitableCharacter {
    character_id,
    name,
    description,
    dialogue_id,
} => {
    let msg = format!("{} - {}", name, description);
    println!("{}", msg);
    if let Some(ref mut log) = game_log {
        log.add(msg);
    }

    // Get current party position for event cleanup tracking
    let current_pos = global_state.0.world.party_position;

    // If dialogue is specified, trigger dialogue system
    if let Some(dlg_id) = dialogue_id {
        // Find NPC entity at current position for speaker (optional visual)
        let speaker_entity = npc_query
            .iter()
            .find(|(_, _, coord)| {
                coord.x == current_pos.x && coord.y == current_pos.y
            })
            .map(|(entity, _, _)| entity)
            .unwrap_or(Entity::PLACEHOLDER);

        // Create recruitment context
        let recruitment_context = crate::application::dialogue::RecruitmentContext {
            character_id: character_id.clone(),
            event_position: current_pos,
        };

        // Store context in PendingRecruitmentContext resource for handle_start_dialogue to consume
        // NOTE: This requires access to World, which handle_events doesn't have directly.
        // SOLUTION: Use Commands to defer context storage

        // TODO: Need to add Commands parameter to handle_events signature
        // For now, log warning - this will be fixed in Task 2.2

        warn!(
            "RecruitableCharacter dialogue trigger not yet implemented. \
             Requires Commands parameter in handle_events. \
             Character: {}, Dialogue: {}",
            character_id, dlg_id
        );

        // Send StartDialogue message
        dialogue_writer.write(StartDialogue {
            dialogue_id: *dlg_id,
            speaker_entity,
        });

        info!("Starting recruitment dialogue {} for character {}", dlg_id, character_id);
    } else {
        // No dialogue specified, simple confirmation
        // TODO: Implement simple yes/no confirmation UI (future enhancement)
        warn!(
            "RecruitableCharacter event for '{}' has no dialogue_id. \
             Simple confirmation UI not yet implemented.",
            character_id
        );
    }
}
```

**BLOCKER IDENTIFIED**: handle_events() needs Commands parameter to store PendingRecruitmentContext.

**Validation**:

- [ ] Code compiles with warning about unimplemented Commands
- [ ] Manual test: Step on RecruitableCharacter, verify warning logged
- [ ] Proceed to Task 2.2 to resolve blocker

**Deliverables**:

- [ ] Event handler updated with dialogue triggering logic
- [ ] Blocker documented for Task 2.2

---

#### Task 2.2: Add Commands Parameter to Event Handler

**File**: `src/game/systems/events.rs`

**Location**: handle_events() function signature (L56)

**Changes**:

1. Update handle_events() signature to include Commands:

```rust
// BEFORE (L56):
fn handle_events(
    mut event_reader: MessageReader<MapEventTriggered>,
    mut map_change_writer: MessageWriter<MapChangeEvent>,
    mut dialogue_writer: MessageWriter<StartDialogue>,
    content: Res<GameContent>,
    mut game_log: Option<ResMut<crate::game::systems::ui::GameLog>>,
    mut global_state: ResMut<GlobalState>,
    npc_query: Query<(Entity, &NpcMarker, &TileCoord)>,
) {

// AFTER:
fn handle_events(
    mut event_reader: MessageReader<MapEventTriggered>,
    mut map_change_writer: MessageWriter<MapChangeEvent>,
    mut dialogue_writer: MessageWriter<StartDialogue>,
    content: Res<GameContent>,
    mut game_log: Option<ResMut<crate::game::systems::ui::GameLog>>,
    mut global_state: ResMut<GlobalState>,
    npc_query: Query<(Entity, &NpcMarker, &TileCoord)>,
    mut pending_recruitment: ResMut<crate::game::systems::dialogue::PendingRecruitmentContext>,  // NEW
) {
```

2. Update RecruitableCharacter case to store context:

```rust
MapEvent::RecruitableCharacter {
    character_id,
    name,
    description,
    dialogue_id,
} => {
    let msg = format!("{} - {}", name, description);
    println!("{}", msg);
    if let Some(ref mut log) = game_log {
        log.add(msg);
    }

    let current_pos = global_state.0.world.party_position;

    if let Some(dlg_id) = dialogue_id {
        let speaker_entity = npc_query
            .iter()
            .find(|(_, _, coord)| {
                coord.x == current_pos.x && coord.y == current_pos.y
            })
            .map(|(entity, _, _)| entity)
            .unwrap_or(Entity::PLACEHOLDER);

        // Store recruitment context
        pending_recruitment.0 = Some(crate::application::dialogue::RecruitmentContext {
            character_id: character_id.clone(),
            event_position: current_pos,
        });

        // Trigger dialogue
        dialogue_writer.write(StartDialogue {
            dialogue_id: *dlg_id,
            speaker_entity,
        });

        info!("Starting recruitment dialogue {} for character {}", dlg_id, character_id);
    } else {
        warn!(
            "RecruitableCharacter event for '{}' has no dialogue_id. \
             Simple confirmation UI not yet implemented.",
            character_id
        );
    }
}
```

**Validation**:

- [ ] `cargo check --all-targets --all-features` - Compiles
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` - Zero warnings
- [ ] `cargo nextest run test_event_handling` - Event tests pass
- [ ] Manual test: Step on RecruitableCharacter with dialogue_id, verify PendingRecruitmentContext populated

**Deliverables**:

- [ ] Commands parameter added to handle_events
- [ ] PendingRecruitmentContext storage implemented
- [ ] All quality checks pass

---

#### Task 2.3: Consume Pending Context in handle_start_dialogue

**File**: `src/game/systems/dialogue.rs`

**Location**: handle_start_dialogue() function (L117-202)

**Changes**:

1. Add parameter to function signature:

```rust
// BEFORE (L117):
fn handle_start_dialogue(
    mut event_reader: MessageReader<StartDialogue>,
    content: Res<GameContent>,
    mut global_state: ResMut<GlobalState>,
) {

// AFTER:
fn handle_start_dialogue(
    mut event_reader: MessageReader<StartDialogue>,
    content: Res<GameContent>,
    mut global_state: ResMut<GlobalState>,
    mut pending_recruitment: ResMut<PendingRecruitmentContext>,  // NEW
) {
```

2. Consume pending context when creating DialogueState (around L175-195):

```rust
// After successful tree and root node lookup, when creating DialogueState:

// Extract recruitment context if present
let recruitment_context = pending_recruitment.0.take();

// Get speaker name
let speaker_name = if let Some(npc_id) = tree.npc_id.as_ref() {
    if let Some(npc) = db.npcs.get_npc(npc_id) {
        npc.name.clone()
    } else {
        tree.speaker_name.clone()
    }
} else {
    tree.speaker_name.clone()
};

// Create dialogue state with recruitment context
let new_state = crate::application::dialogue::DialogueState {
    active_tree_id: Some(tree_id),
    current_node_id: root_node.id,
    dialogue_history: vec![root_node.id],
    current_node_text: root_node.text.clone(),
    speaker_name,
    choices: root_node
        .choices
        .iter()
        .map(|c| crate::application::dialogue::ChoiceText {
            text: c.text.clone(),
            available: true,
        })
        .collect(),
    speaker_entity: msg.speaker_entity,
    recruitment_context,  // NEW - set from pending context
};

global_state.0.mode = crate::application::GameMode::Dialogue(new_state);
```

**Validation**:

- [ ] `cargo check --all-targets --all-features` - Compiles
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` - Zero warnings
- [ ] `cargo nextest run test_start_dialogue_with_recruitment` - Test passes
- [ ] Test verifies recruitment_context transferred to DialogueState
- [ ] Test verifies PendingRecruitmentContext cleared after consumption

**Test Case Required**:

```rust
#[test]
fn test_start_dialogue_with_recruitment_context() {
    // Arrange
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(DialoguePlugin);

    let mut pending = PendingRecruitmentContext::default();
    pending.0 = Some(RecruitmentContext {
        character_id: "test_knight".to_string(),
        event_position: Position::new(5, 5),
    });
    app.insert_resource(pending);

    let db = create_test_database_with_dialogue(1000);
    app.insert_resource(GameContent::new(db));

    let mut game_state = GameState::default();
    game_state.mode = GameMode::Exploration;
    app.insert_resource(GlobalState(game_state));

    // Act
    app.world_mut()
        .resource_mut::<Messages<StartDialogue>>()
        .write(StartDialogue {
            dialogue_id: 1000,
            speaker_entity: Entity::PLACEHOLDER,
        });

    app.update();

    // Assert
    let state = &app.world().resource::<GlobalState>().0;
    if let GameMode::Dialogue(dlg_state) = &state.mode {
        assert!(dlg_state.recruitment_context.is_some());
        assert_eq!(dlg_state.recruitment_context.as_ref().unwrap().character_id, "test_knight");
    } else {
        panic!("Expected Dialogue mode");
    }

    // Verify pending context cleared
    let pending = app.world().resource::<PendingRecruitmentContext>();
    assert!(pending.0.is_none());
}
```

**Deliverables**:

- [ ] handle_start_dialogue consumes PendingRecruitmentContext
- [ ] recruitment_context set in DialogueState
- [ ] Test case implemented and passing
- [ ] All quality checks pass

---

#### Task 2.4: Remove Map Event After Successful Recruitment

**File**: `src/game/systems/dialogue.rs`

**Location**: execute_action() function, RecruitToParty and RecruitToInn actions

**Changes**:

1. Add DialogueState parameter to execute_action signature (L501):

```rust
// BEFORE:
fn execute_action(
    action: &DialogueAction,
    game_state: &mut crate::application::GameState,
    db: &crate::sdk::database::ContentDatabase,
    mut quest_system: Option<&mut crate::application::quests::QuestSystem>,
    mut game_log: Option<&mut crate::game::systems::ui::GameLog>,
) {

// AFTER:
fn execute_action(
    action: &DialogueAction,
    game_state: &mut crate::application::GameState,
    db: &crate::sdk::database::ContentDatabase,
    dialogue_state: Option<&crate::application::dialogue::DialogueState>,  // NEW
    mut quest_system: Option<&mut crate::application::quests::QuestSystem>,
    mut game_log: Option<&mut crate::game::systems::ui::GameLog>,
) {
```

2. Update all call sites of execute_action to pass dialogue_state (search for "execute_action" calls).

3. Add event removal to RecruitToParty action (after successful recruitment):

```rust
DialogueAction::RecruitToParty { character_id } => {
    info!("Executing RecruitToParty action for character '{}'", character_id);

    match game_state.recruit_from_map(character_id, db) {
        Ok(RecruitResult::AddedToParty) => {
            info!("Successfully recruited '{}' to active party", character_id);
            if let Some(ref mut log) = game_log {
                if let Some(char_def) = db.characters.get_character(character_id) {
                    log.add(format!("{} joins the party!", char_def.name));
                } else {
                    log.add(format!("{} joins the party!", character_id));
                }
            }

            // NEW: Remove recruitment event from map
            if let Some(dlg_state) = dialogue_state {
                if let Some(ref recruitment_ctx) = dlg_state.recruitment_context {
                    if let Some(current_map) = game_state.world.get_current_map_mut() {
                        if let Some(removed_event) = current_map.remove_event_at(recruitment_ctx.event_position) {
                            info!("Removed recruitment event at {:?}", recruitment_ctx.event_position);
                        } else {
                            warn!("No event found at recruitment position {:?}", recruitment_ctx.event_position);
                        }
                    }
                }
            }
        }
        Ok(RecruitResult::SentToInn(inn_id)) => {
            // ... existing code ...

            // NEW: Remove recruitment event from map (same as above)
            if let Some(dlg_state) = dialogue_state {
                if let Some(ref recruitment_ctx) = dlg_state.recruitment_context {
                    if let Some(current_map) = game_state.world.get_current_map_mut() {
                        if let Some(_removed_event) = current_map.remove_event_at(recruitment_ctx.event_position) {
                            info!("Removed recruitment event at {:?}", recruitment_ctx.event_position);
                        }
                    }
                }
            }
        }
        // ... error cases unchanged ...
    }
}
```

4. Add same event removal logic to RecruitToInn action (after successful recruitment).

5. Implement Map::remove_event_at() method (if not exists):

**File**: `src/domain/world/types.rs`

**Location**: impl Map block (around L350)

```rust
impl Map {
    // ... existing methods ...

    /// Removes and returns the event at the specified position
    ///
    /// # Arguments
    ///
    /// * `position` - Map position to check
    ///
    /// # Returns
    ///
    /// Returns `Some(MapEvent)` if event existed and was removed, `None` otherwise
    pub fn remove_event_at(&mut self, position: Position) -> Option<MapEvent> {
        self.events.remove(&position)
    }

    /// Checks if an event exists at the specified position
    pub fn has_event_at(&self, position: Position) -> bool {
        self.events.contains_key(&position)
    }
}
```

**Validation**:

- [ ] `cargo check --all-targets --all-features` - Compiles
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` - Zero warnings
- [ ] `cargo nextest run test_recruitment_removes_event` - Test passes
- [ ] Manual test: Recruit character, return to event position, verify event gone

**Test Case Required**:

```rust
#[test]
fn test_recruitment_removes_map_event() {
    // Arrange
    let mut game_state = GameState::default();
    let mut map = Map::new(1, "Test".to_string(), "Test".to_string(), 10, 10);
    let event_pos = Position::new(5, 5);

    map.add_event(
        event_pos,
        MapEvent::RecruitableCharacter {
            name: "Knight".to_string(),
            description: "A brave knight".to_string(),
            character_id: "test_knight".to_string(),
            dialogue_id: Some(1000),
        },
    );

    game_state.world.add_map(map);
    game_state.world.set_current_map(1);
    game_state.world.set_party_position(event_pos);

    let db = create_test_database_with_character("test_knight");

    let dialogue_state = DialogueState {
        recruitment_context: Some(RecruitmentContext {
            character_id: "test_knight".to_string(),
            event_position: event_pos,
        }),
        ..Default::default()
    };

    // Verify event exists before recruitment
    assert!(game_state.world.get_current_map().unwrap().has_event_at(event_pos));

    // Act
    execute_action(
        &DialogueAction::RecruitToParty {
            character_id: "test_knight".to_string(),
        },
        &mut game_state,
        &db,
        Some(&dialogue_state),
        None,
        None,
    );

    // Assert
    assert!(!game_state.world.get_current_map().unwrap().has_event_at(event_pos));
}
```

**Deliverables**:

- [ ] execute_action signature updated with dialogue_state parameter
- [ ] All call sites updated
- [ ] Event removal implemented in both actions
- [ ] Map::remove_event_at() implemented
- [ ] Test case passing
- [ ] All quality checks pass

---

### Phase 3: SDK and Campaign Builder Updates

**Objective**: Update map editor and validation for dialogue_id field

**Duration**: 4-6 hours

**Prerequisites**: Phase 1 complete (Phase 2 can run in parallel)

---

#### Task 3.1: Update Map Editor for dialogue_id Field

**File**: `sdk/campaign_builder/src/map_editor.rs`

**Location**: EventEditorState struct (L1031-1041) and UI rendering

**Changes**:

1. Add fields to EventEditorState:

```rust
pub struct EventEditorState {
    pub event_type: EventType,
    pub position: Position,
    pub name: String,
    pub description: String,

    // ... existing fields ...

    // NEW for RecruitableCharacter:
    pub recruitable_character_id: String,
    pub recruitable_dialogue_id: String,
}
```

2. Update EventEditorState::default() to initialize new fields:

```rust
impl Default for EventEditorState {
    fn default() -> Self {
        Self {
            // ... existing fields ...
            recruitable_character_id: String::new(),
            recruitable_dialogue_id: String::new(),
        }
    }
}
```

3. Add UI fields for RecruitableCharacter event type (search for "EventType::RecruitableCharacter" in render function):

```rust
EventType::RecruitableCharacter => {
    ui.heading("Recruitable Character Event");

    ui.horizontal(|ui| {
        ui.label("Event Name:");
        ui.text_edit_singleline(&mut state.name);
    });

    ui.horizontal(|ui| {
        ui.label("Description:");
        ui.text_edit_multiline(&mut state.description);
    });

    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Character ID:");
        ui.text_edit_singleline(&mut state.recruitable_character_id);
        if ui.button("📋 Browse").clicked() {
            // TODO: Show character picker dialog
        }
    });

    ui.horizontal(|ui| {
        ui.label("Dialogue ID:");
        ui.text_edit_singleline(&mut state.recruitable_dialogue_id);
        if ui.button("↺ Default (1000)").clicked() {
            state.recruitable_dialogue_id = "1000".to_string();
        }
        if ui.button("✖ None").clicked() {
            state.recruitable_dialogue_id.clear();
        }
    });

    ui.label("💡 Tip: Leave Dialogue ID empty for simple yes/no recruitment");
}
```

4. Update event construction when saving (search for "EventType::RecruitableCharacter" in save/apply logic):

```rust
EventType::RecruitableCharacter => {
    let dialogue_id = if state.recruitable_dialogue_id.is_empty() {
        None
    } else {
        state.recruitable_dialogue_id
            .parse::<DialogueId>()
            .ok()
    };

    MapEvent::RecruitableCharacter {
        name: state.name.clone(),
        description: state.description.clone(),
        character_id: state.recruitable_character_id.clone(),
        dialogue_id,
    }
}
```

5. Update event loading when editing existing event:

```rust
MapEvent::RecruitableCharacter {
    name,
    description,
    character_id,
    dialogue_id,
} => {
    state.event_type = EventType::RecruitableCharacter;
    state.name = name.clone();
    state.description = description.clone();
    state.recruitable_character_id = character_id.clone();
    state.recruitable_dialogue_id = dialogue_id
        .map(|id| id.to_string())
        .unwrap_or_default();
}
```

**Validation**:

- [ ] `cargo check --package campaign_builder` - Compiles
- [ ] `cargo clippy --package campaign_builder -- -D warnings` - Zero warnings
- [ ] Manual test: Open map editor, create RecruitableCharacter event
- [ ] Manual test: Verify dialogue_id field appears and accepts input
- [ ] Manual test: Save event, reload, verify dialogue_id persists
- [ ] Manual test: Set dialogue_id to empty, verify saved as None in RON

**Deliverables**:

- [ ] EventEditorState updated with new fields
- [ ] UI rendering includes dialogue_id input
- [ ] Event construction uses dialogue_id
- [ ] Event loading populates dialogue_id
- [ ] All quality checks pass

---

#### Task 3.2: Add Character and Dialogue Reference Validation

**File**: `sdk/campaign_builder/src/validation.rs`

**Location**: Add new validation function

**Changes**:

1. Add validation function:

```rust
/// Validates that RecruitableCharacter events reference valid character and dialogue IDs
///
/// # Arguments
///
/// * `maps` - All maps in the campaign
/// * `char_db` - Character database
/// * `dialogue_db` - Dialogue database
///
/// # Returns
///
/// Vector of validation errors found
pub fn validate_recruitable_character_references(
    maps: &[crate::domain::world::Map],
    char_db: &crate::domain::character_definition::CharacterDatabase,
    dialogue_db: &crate::domain::dialogue::DialogueDatabase,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for map in maps {
        for (position, event) in &map.events {
            if let crate::domain::world::MapEvent::RecruitableCharacter {
                name,
                character_id,
                dialogue_id,
                ..
            } = event
            {
                // Validate character exists in database
                if char_db.get_character(character_id).is_none() {
                    errors.push(ValidationError {
                        severity: Severity::Error,
                        category: ErrorCategory::MissingReference,
                        message: format!(
                            "Map '{}' ({}): RecruitableCharacter event '{}' references unknown character_id '{}'",
                            map.name, map.id, name, character_id
                        ),
                        location: Some(format!("Position {:?}", position)),
                        suggestion: Some(format!(
                            "Add character '{}' to data/characters.ron or fix character_id reference",
                            character_id
                        )),
                    });
                }

                // Validate dialogue exists (if specified)
                if let Some(dlg_id) = dialogue_id {
                    if dialogue_db.get_dialogue(*dlg_id).is_none() {
                        errors.push(ValidationError {
                            severity: Severity::Error,
                            category: ErrorCategory::MissingReference,
                            message: format!(
                                "Map '{}' ({}): RecruitableCharacter event '{}' references unknown dialogue_id {}",
                                map.name, map.id, name, dlg_id
                            ),
                            location: Some(format!("Position {:?}", position)),
                            suggestion: Some(format!(
                                "Add dialogue {} to data/dialogues.ron or remove dialogue_id field",
                                dlg_id
                            )),
                        });
                    }
                }
            }
        }
    }

    errors
}
```

2. Add to main validation function (search for "pub fn validate_campaign" or similar):

```rust
pub fn validate_campaign(
    maps: &[Map],
    char_db: &CharacterDatabase,
    dialogue_db: &DialogueDatabase,
    // ... other databases
) -> ValidationReport {
    let mut errors = Vec::new();

    // ... existing validations ...

    // NEW: Validate recruitment references
    errors.extend(validate_recruitable_character_references(maps, char_db, dialogue_db));

    ValidationReport { errors }
}
```

**Validation**:

- [ ] `cargo check --package campaign_builder` - Compiles
- [ ] `cargo clippy --package campaign_builder -- -D warnings` - Zero warnings
- [ ] `cargo nextest run --package campaign_builder test_validation` - Tests pass

**Test Cases Required**:

```rust
#[test]
fn test_validation_invalid_character_id() {
    // Arrange
    let mut map = Map::new(1, "Test".to_string(), "Test".to_string(), 10, 10);
    map.add_event(
        Position::new(5, 5),
        MapEvent::RecruitableCharacter {
            name: "Knight".to_string(),
            description: "Test".to_string(),
            character_id: "nonexistent".to_string(),
            dialogue_id: None,
        },
    );

    let char_db = CharacterDatabase::new(); // Empty
    let dialogue_db = DialogueDatabase::new(); // Empty

    // Act
    let errors = validate_recruitable_character_references(
        &[map],
        &char_db,
        &dialogue_db,
    );

    // Assert
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("unknown character_id 'nonexistent'"));
}

#[test]
fn test_validation_invalid_dialogue_id() {
    // Arrange
    let mut map = Map::new(1, "Test".to_string(), "Test".to_string(), 10, 10);
    map.add_event(
        Position::new(5, 5),
        MapEvent::RecruitableCharacter {
            name: "Knight".to_string(),
            description: "Test".to_string(),
            character_id: "test_knight".to_string(),
            dialogue_id: Some(9999), // Invalid
        },
    );

    let mut char_db = CharacterDatabase::new();
    char_db.add_character(create_test_character_definition("test_knight"));
    let dialogue_db = DialogueDatabase::new(); // Empty

    // Act
    let errors = validate_recruitable_character_references(
        &[map],
        &char_db,
        &dialogue_db,
    );

    // Assert
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("unknown dialogue_id 9999"));
}

#[test]
fn test_validation_valid_references() {
    // Arrange
    let mut map = Map::new(1, "Test".to_string(), "Test".to_string(), 10, 10);
    map.add_event(
        Position::new(5, 5),
        MapEvent::RecruitableCharacter {
            name: "Knight".to_string(),
            description: "Test".to_string(),
            character_id: "test_knight".to_string(),
            dialogue_id: Some(1000),
        },
    );

    let mut char_db = CharacterDatabase::new();
    char_db.add_character(create_test_character_definition("test_knight"));

    let mut dialogue_db = DialogueDatabase::new();
    dialogue_db.add_dialogue(create_test_dialogue(1000));

    // Act
    let errors = validate_recruitable_character_references(
        &[map],
        &char_db,
        &dialogue_db,
    );

    // Assert
    assert_eq!(errors.len(), 0);
}
```

**Deliverables**:

- [ ] validate_recruitable_character_references() implemented
- [ ] Integrated into main validation function
- [ ] All 3 test cases passing
- [ ] All quality checks pass

---

### Phase 4: Integration Testing and Documentation

**Objective**: End-to-end testing and documentation updates

**Duration**: 4-6 hours

**Prerequisites**: Phases 1, 2, 3 complete

---

#### Task 4.1: End-to-End Recruitment Integration Test

**File**: `tests/integration/recruitment_test.rs` (NEW FILE)

**Create comprehensive integration test covering full recruitment flow**

**Validation**:

- [ ] `cargo nextest run --test recruitment_test` - All tests pass
- [ ] Test coverage >80% for recruitment code paths
- [ ] Manual testing: Full recruitment flow works in-game

**Deliverables**:

- [ ] Integration test file created
- [ ] Tests cover success, failure, and edge cases
- [ ] All tests passing
- [ ] All quality checks pass

---

#### Task 4.2: Update Implementation Documentation

**File**: `docs/explanation/implementations.md`

**Location**: Add new section at end of file

**Changes**:

Add comprehensive summary of recruitment implementation:

````markdown
## Character Recruitment System

**Status**: ✅ Implemented
**Date**: 2025-01-XX
**Implementation Plan**: `docs/explanation/character_recruitment_implementation_plan.md`

### Overview

The character recruitment system enables players to recruit pre-defined characters to their party or send them to inns through interactive dialogues. Characters are defined in the character database (`data/characters.ron`) and referenced by `character_id` in map events.

### Architecture

**Core Components**:

1. **MapEvent::RecruitableCharacter** (`src/domain/world/types.rs:L485-492`)

   - Stores character_id and optional dialogue_id
   - Placed on maps at recruitable character locations
   - One-time events (removed after recruitment)

2. **DialogueAction Execution** (`src/game/systems/dialogue.rs:L501-600`)

   - `RecruitToParty`: Calls GameState::recruit_from_map()
   - `RecruitToInn`: Explicitly sends character to specified inn
   - Handles all error cases with user feedback

3. **RecruitmentContext** (`src/application/dialogue.rs`)

   - Stores character_id and event_position
   - Passed through dialogue system for event cleanup
   - Enables event removal after successful recruitment

4. **Event Triggering** (`src/game/systems/events.rs:L154-166`)

   - Detects RecruitableCharacter events
   - Triggers dialogue system with recruitment context
   - Uses PendingRecruitmentContext resource for state transfer

5. **Party Management** (`src/application/mod.rs:L838-884`)
   - `recruit_from_map()`: Core recruitment logic
   - Validates character exists, prevents duplicates
   - Auto-assigns to party or inn based on capacity
   - Updates Roster with CharacterLocation tracking

### Key Design Decisions

**Dialogue-Driven Recruitment**: Uses existing dialogue system infrastructure for player interaction, enabling rich narrative context for recruitment decisions.

**Character Templates**: CharacterDefinition.instantiate() creates fresh Character instances from templates, preserving data-driven design principles.

**Event Cleanup**: Recruitment events are one-time encounters, automatically removed from maps after successful recruitment to prevent duplicate recruitment attempts.

**Capacity Enforcement**: Party limited to PARTY_MAX_SIZE (6 members). When party is full, characters are automatically sent to nearest inn.

**Duplicate Prevention**: encountered_characters HashSet tracks all recruited characters, preventing re-recruitment even if event removal fails.

### Implementation Files

**Core Domain**:

- `src/domain/world/types.rs` - MapEvent::RecruitableCharacter with dialogue_id field
- `src/application/dialogue.rs` - RecruitmentContext struct
- `src/application/mod.rs` - recruit_from_map() core logic
- `src/domain/dialogue.rs` - RecruitToParty and RecruitToInn actions

**Game Systems**:

- `src/game/systems/dialogue.rs` - Action execution, PendingRecruitmentContext resource
- `src/game/systems/events.rs` - Event triggering and context storage

**SDK Tools**:

- `sdk/campaign_builder/src/map_editor.rs` - UI for editing recruitment events
- `sdk/campaign_builder/src/validation.rs` - Character and dialogue reference validation

### Testing

**Unit Tests**:

- RecruitToParty action (4 test cases: success, party full, duplicate, not found)
- RecruitToInn action (3 test cases: success, duplicate, invalid innkeeper)
- Event removal (1 test case: verifies map cleanup)
- Dialogue state initialization with recruitment context

**Integration Tests**:

- End-to-end recruitment flow (event → dialogue → action → party update)
- Event cleanup verification
- Duplicate recruitment prevention
- SDK validation tests

**Test Coverage**: >80% for all recruitment code paths

### Usage Example

**Data Files**:

```ron
// data/characters.ron
CharacterDefinition(
    id: "village_knight",
    name: "Sir Roland",
    race_id: "human",
    class_id: "knight",
    // ... stats, equipment, etc.
)

// data/dialogues.ron
DialogueTree(
    id: 1001,
    name: "Sir Roland Recruitment",
    root_node: 1,
    nodes: {
        1: DialogueNode(
            text: "I seek worthy companions. Will you have me?",
            choices: [
                DialogueChoice(
                    text: "Join our party!",
                    actions: [RecruitToParty(character_id: "village_knight")],
                    ends_dialogue: true,
                ),
                DialogueChoice(
                    text: "Wait at the inn.",
                    actions: [RecruitToInn(
                        character_id: "village_knight",
                        innkeeper_id: "town_innkeeper"
                    )],
                    ends_dialogue: true,
                ),
            ],
        ),
    },
)

// data/maps/village.ron
events: {
    (5, 10): RecruitableCharacter(
        name: "Sir Roland",
        description: "A knight in shining armor",
        character_id: "village_knight",
        dialogue_id: Some(1001),
    ),
}
```
````

### Future Enhancements

- Conditional recruitment (require quest completion, reputation thresholds)
- Recruitment costs (gold payment for mercenaries)
- Recruitment refusal dialogues based on character compatibility
- Simple yes/no confirmation UI when dialogue_id is None

````

**Validation**:

- [ ] Documentation file updated
- [ ] All links valid
- [ ] Code examples use correct syntax
- [ ] Markdown properly formatted

**Deliverables**:

- [ ] Implementation summary added to implementations.md
- [ ] Examples included
- [ ] Future enhancements documented
- [ ] All quality checks pass

---

## Implementation Timeline

| Phase | Tasks | Duration | Dependencies | Owner |
|-------|-------|----------|--------------|-------|
| Phase 0: Preparation | 2 tasks | 1-2 hours | None | Human + AI |
| Phase 1: Core Actions | 5 tasks | 6-8 hours | Phase 0 | AI Agent |
| Phase 2: Dialogue Integration | 4 tasks | 6-8 hours | Phase 1 | AI Agent |
| Phase 3: SDK Updates | 2 tasks | 4-6 hours | Phase 1 | AI Agent |
| Phase 4: Testing & Docs | 2 tasks | 4-6 hours | Phases 1-3 | AI Agent |
| **TOTAL** | **15 tasks** | **21-30 hours** | Sequential | Mixed |

**Critical Path**: Phase 0 → Phase 1 → Phase 2 → Phase 4
**Parallel Work**: Phase 3 can run concurrently with Phase 2

**Milestones**:

1. **Day 1-2**: Phase 0 complete, architecture decision made
2. **Day 3-4**: Phase 1 complete, recruitment actions functional
3. **Day 5-6**: Phase 2 complete, dialogue integration working
4. **Day 7**: Phase 3 complete, SDK tools updated
5. **Day 8**: Phase 4 complete, all tests passing, documentation updated

---

## Success Criteria

### Phase-Level Success Criteria

**Phase 0: Preparation**
- [ ] All tools installed (clippy, rustfmt, nextest)
- [x] Architecture decision made and documented (Option B chosen)
- [x] Human stakeholder approval obtained

**Phase 1: Core Actions**
- [ ] dialogue_id field added to MapEvent::RecruitableCharacter
- [ ] RecruitmentContext struct defined
- [ ] PendingRecruitmentContext resource created
- [ ] RecruitToParty action fully implemented with error handling
- [ ] RecruitToInn action fully implemented with error handling
- [ ] All 7 unit tests passing (4 for RecruitToParty, 3 for RecruitToInn)
- [ ] `cargo check` passes
- [ ] `cargo clippy` passes with zero warnings
- [ ] `cargo nextest run` passes all tests

**Phase 2: Dialogue Integration**
- [ ] Event handler triggers recruitment dialogue
- [ ] PendingRecruitmentContext stored on event trigger
- [ ] handle_start_dialogue consumes pending context
- [ ] recruitment_context set in DialogueState
- [ ] Map events removed after successful recruitment
- [ ] Map::remove_event_at() implemented
- [ ] All 2 integration tests passing
- [ ] `cargo check` passes
- [ ] `cargo clippy` passes with zero warnings
- [ ] `cargo nextest run` passes all tests

**Phase 3: SDK Updates**
- [ ] Map editor UI includes dialogue_id field
- [ ] Event construction uses dialogue_id
- [ ] Event loading populates dialogue_id correctly
- [ ] validate_recruitable_character_references() implemented
- [ ] Character ID validation working
- [ ] Dialogue ID validation working
- [ ] All 3 SDK tests passing
- [ ] `cargo check --package campaign_builder` passes
- [ ] `cargo clippy --package campaign_builder` passes

**Phase 4: Testing & Documentation**
- [ ] Integration test file created
- [ ] End-to-end recruitment test passing
- [ ] Test coverage >80% for recruitment code
- [ ] implementations.md updated with recruitment summary
- [ ] All code examples in docs verified
- [ ] Manual testing confirms in-game functionality

### Final Acceptance Criteria

**Functional Requirements**:
- [ ] Player can step on RecruitableCharacter event
- [ ] Dialogue triggers with correct dialogue_id
- [ ] Player can select recruitment choice
- [ ] Character added to party if space available (< 6 members)
- [ ] Character sent to inn if party full (= 6 members)
- [ ] Recruitment event removed from map after recruitment
- [ ] Duplicate recruitment prevented (encountered_characters tracking)
- [ ] Error messages displayed for invalid character_id
- [ ] Game log shows recruitment success/failure messages

**Code Quality**:
- [ ] `cargo fmt --all` - No changes (already formatted)
- [ ] `cargo check --all-targets --all-features` - Zero errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` - Zero warnings
- [ ] `cargo nextest run --all-features` - All tests pass
- [ ] Test coverage >80% for all new code
- [ ] No compiler warnings
- [ ] No clippy warnings
- [ ] All doc comments include examples

**Architecture Compliance**:
- [ ] Uses existing GameState::recruit_from_map() function
- [ ] Respects PARTY_MAX_SIZE constant (6)
- [ ] Respects ROSTER_MAX_SIZE constant (18)
- [ ] Uses type aliases (DialogueId, CharacterId, etc.)
- [ ] Follows module structure (Section 3.2 of architecture.md)
- [ ] RON format used for data files
- [ ] No architectural deviations introduced

**Documentation**:
- [ ] implementations.md updated
- [ ] All file paths specified with line numbers
- [ ] Code examples are compilable
- [ ] No broken markdown links
- [ ] SPDX headers added to all new files

**SDK Validation**:
- [ ] Map editor can create/edit recruitment events
- [ ] dialogue_id field appears in UI
- [ ] Validation catches invalid character_id references
- [ ] Validation catches invalid dialogue_id references
- [ ] Campaign builder validation passes

---

## Deliverables Summary

### Code Deliverables

**New Files**:
- [ ] `tests/integration/recruitment_test.rs` - Integration tests

**Modified Files**:
- [ ] `src/domain/world/types.rs` - Added dialogue_id field to RecruitableCharacter
- [ ] `src/application/dialogue.rs` - Added RecruitmentContext struct and recruitment_context field
- [ ] `src/game/systems/dialogue.rs` - Implemented RecruitToParty/RecruitToInn actions, PendingRecruitmentContext resource
- [ ] `src/game/systems/events.rs` - Updated event handler to trigger recruitment dialogue
- [ ] `src/domain/world/types.rs` - Added Map::remove_event_at() method
- [ ] `sdk/campaign_builder/src/map_editor.rs` - Added dialogue_id UI fields
- [ ] `sdk/campaign_builder/src/validation.rs` - Added recruitment reference validation

**Test Files**:
- [ ] Unit tests for RecruitToParty action (4 tests)
- [ ] Unit tests for RecruitToInn action (3 tests)
- [ ] Unit test for event removal (1 test)
- [ ] Unit test for dialogue state with recruitment context (1 test)
- [ ] Integration test for end-to-end recruitment (1 test)
- [ ] SDK validation tests (3 tests)

**Total Test Count**: 13 new tests minimum

### Documentation Deliverables

- [ ] `docs/explanation/implementations.md` - Recruitment system summary section
- [ ] This plan file - Updated with CHOSEN OPTION decision
- [ ] All code doc comments with examples

### Validation Checklist

Before marking implementation complete, verify:

1. **Compile & Lint**:
   ```bash
   cargo fmt --all
   cargo check --all-targets --all-features
   cargo clippy --all-targets --all-features -- -D warnings
````

2. **Tests**:

   ```bash
   cargo nextest run --all-features
   cargo nextest run --package campaign_builder
   cargo nextest run --test recruitment_test
   ```

3. **Manual Testing**:

   - Create character definition in data/characters.ron
   - Create recruitment dialogue in data/dialogues.ron
   - Add RecruitableCharacter event to map
   - Launch game, step on event
   - Verify dialogue triggers
   - Select recruitment option
   - Verify character in party or inn
   - Return to event position
   - Verify event removed

4. **Documentation**:
   - Verify all links in implementations.md work
   - Verify code examples compile
   - Verify line numbers in file references are accurate

---

## Migration Guide for Existing Content

**For maps with existing RecruitableCharacter events**:

1. **No immediate changes required** - dialogue_id defaults to None
2. **To add custom dialogue**:
   - Create dialogue tree in `data/dialogues.ron`
   - Add `dialogue_id: Some(XXX)` to event in map file
   - Include RecruitToParty or RecruitToInn action in dialogue choices
3. **Validation**: Run `cargo run --bin campaign_builder -- validate` to check references

**RON Format Example**:

```ron
// Before (still valid):
(5, 10): RecruitableCharacter(
    name: "Sir Roland",
    description: "A knight seeks companions",
    character_id: "village_knight",
)

// After (with custom dialogue):
(5, 10): RecruitableCharacter(
    name: "Sir Roland",
    description: "A knight seeks companions",
    character_id: "village_knight",
    dialogue_id: Some(1001),
)
```

---

## Appendix: Architectural Decisions

### Decision: Use Option B (dialogue_id in RecruitableCharacter)

**Status**: ✅ APPROVED by user on 2025-01-XX

**Rationale**:

- Backwards compatible (dialogue_id is optional)
- Keeps recruitment events distinct from generic NPCs
- Lower migration cost for existing content
- Explicit separation of concerns
- Enables future simple UI when dialogue_id is None

**Trade-offs Accepted**:

- Slightly more complex than simple yes/no UI (acceptable for richer player experience)
- Requires dialogue tree creation for rich recruitment interactions (aligns with data-driven design)
- More complex than reusing NpcDialogue events (acceptable for explicit separation)

**Alternatives Considered**:

- Option A (NpcDialogue): Rejected due to high migration cost and loss of recruitment event distinction
- Option C (Simple UI): Deferred to future enhancement when dialogue_id is None (can coexist with Option B)

**Implementation Start Date**: Ready to proceed with Phase 1

---

## Contact & Support

**Implementation Questions**: Refer to architecture.md Section 4 (Data Structures) and Section 3.2 (Module Structure)

**Testing Issues**: See AGENTS.md Emergency Procedures section

**SDK Issues**: Consult sdk/campaign_builder/README.md (if exists) or source code documentation
