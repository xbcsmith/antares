# Party Management System Implementation Plan

## Overview

This plan implements a comprehensive party management system for Antares, enabling players to:

- Swap party members at Inns (add/remove/replace characters)
- Define starting party members via `starts_in_party` flag in character definitions
- Track character locations throughout the game (in party, at specific inns, on maps)
- Encounter recruitable characters on maps with overflow handling (send to inn if party full)
- Persist character locations across game sessions

## Current State Analysis

### Existing Infrastructure

**Character System (antares/src/domain/character.rs)**

- `Party`: Active adventuring group (0-6 members, `MAX_MEMBERS = 6`)
  - `add_member(&mut self, character: Character)`: Adds character, returns `CharacterError::PartyFull` if at capacity
  - `remove_member(&mut self, index: usize)`: Removes by index, returns `Option<Character>`
  - `is_full()`, `is_empty()`, `size()` helper methods
- `Roster`: All created characters (up to 18, `MAX_CHARACTERS = 18`)
  - `add_character(&mut self, character: Character, location: Option<InnkeeperId>)`: Adds to roster with location (InnkeeperId string)
  - `character_locations: Vec<Option<InnkeeperId>>`: Parallel array tracking where each roster character is stored (legacy; replaced by `Vec<CharacterLocation>`)

**Game State (antares/src/application/mod.rs)**

- `GameState`: Contains `roster: Roster`, `party: Party`, `mode: GameMode`, `world: World`
- `initialize_roster(&mut self, content_db: &ContentDatabase)`: Populates roster from premade characters (`is_premade == true`)
  - Currently does NOT populate starting party
  - Calls `CharacterDefinition::instantiate()` for each premade
  - Adds to roster with `location: None`

**Character Definitions (antares/src/domain/character_definition.rs)**

- `CharacterDefinition`: RON-serializable template for creating runtime `Character` instances
  - `is_premade: bool`: Distinguishes premades from templates
  - No `starts_in_party` field yet
  - `instantiate()` method creates runtime `Character` from definition + databases

**Campaign Data (campaigns/tutorial/data/characters.ron)**

- 3 tutorial premades: `tutorial_human_knight` (Kira), `tutorial_elf_sorcerer` (Sage), `tutorial_human_cleric` (Mira)
- 3 recruitable NPCs: `npc_old_gareth`, `npc_whisper`, `npc_apprentice_zara` (all `is_premade: false`)
- 3 templates for character generation

**Map Events (antares/src/game/systems/map.rs)**

- `MapEventType` enum: `Teleport`, `NpcDialogue`, `CombatEncounter`, `TreasureChest`
  - No character recruitment event type yet

**UI Systems (antares/src/game/systems/ui.rs)**

- Basic egui UI showing party members in bottom panel
- No Inn UI or character management UI yet

### Identified Issues

1. **No starting party population**: `initialize_roster` adds premades to roster but NOT to party
2. **No character location tracking**: `Roster::character_locations` exists but is unused after initialization (always `None`)
3. **No Inn interaction system**: No UI or game logic for managing party at Inns
4. **No map recruitment events**: No way to encounter and recruit NPCs on maps
5. **No persistent location updates**: Character locations are not updated when characters join/leave party or are deposited at Inns
6. **No character state enum**: `Option<TownId>` in roster cannot distinguish "in party" vs "at no inn" vs "at specific inn"

## Implementation Phases

### Phase 1: Core Data Model & Starting Party

#### 1.1 Foundation Work

**Add `CharacterLocation` enum (antares/src/domain/character.rs)**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharacterLocation {
    /// Character is in the active party
    InParty,
    /// Character is stored at a specific inn/town
    AtInn(InnkeeperId),
    /// Character is available on a specific map (for encounters)
    OnMap(MapId),
}
```

**Update `Roster` struct**

- Change `character_locations: Vec<Option<TownId>>` → `character_locations: Vec<CharacterLocation>`
- Add helper methods:
  - `find_character_by_id(&self, id: CharacterId) -> Option<usize>`: Find roster index by character ID
  - `get_character(&self, index: usize) -> Option<&Character>`: Safe indexed access
  - `get_character_mut(&mut self, index: usize) -> Option<&mut Character>`: Mutable access
  - `update_location(&mut self, index: usize, location: CharacterLocation)`: Update location tracking
  - `characters_at_inn(&self, innkeeper_id: InnkeeperId) -> Vec<(usize, &Character)>`: Get all characters at a specific inn
  - `characters_in_party(&self) -> Vec<(usize, &Character)>`: Get all characters marked `InParty`

**Add `starts_in_party` to `CharacterDefinition` (antares/src/domain/character_definition.rs)**

```rust
pub struct CharacterDefinition {
    // ... existing fields ...

    /// Whether this character should start in the active party (new games only)
    #[serde(default)]
    pub starts_in_party: bool,
}
```

#### 1.2 Add Starting Party Population

**Update `GameState::initialize_roster` (antares/src/application/mod.rs)**

- After adding character to roster, check `def.starts_in_party`
- If true, attempt `self.party.add_member(character.clone())` (clone before adding to roster)
- Set roster location to `CharacterLocation::InParty` for party members
- Set roster location to `CharacterLocation::AtInn(starting_innkeeper.clone())` for non-party premades
- Add `starting_innkeeper: String` to `CampaignConfig` (default: `"tutorial_innkeeper_town"`)
- Handle party overflow: if party full, log warning and place at starting inn instead
- Return `RosterInitializationError::PartyOverflow` if more than `Party::MAX_MEMBERS` have `starts_in_party: true`

**Add campaign config field (antares/src/sdk/campaign_loader.rs)**

```rust
pub struct CampaignConfig {
    // ... existing fields ...

    /// Default innkeeper NPC ID where non-party premade characters start
    #[serde(default = "default_starting_innkeeper")]
    pub starting_innkeeper: String,
}

fn default_starting_innkeeper() -> String {
    "tutorial_innkeeper_town".to_string()
}
```

#### 1.3 Integrate Starting Party

**Update tutorial campaign data (campaigns/tutorial/data/characters.ron)**

- Set `starts_in_party: true` for Kira, Sage, Mira (the 3 tutorial premades)
- Set `starts_in_party: false` (or omit, since default) for NPCs and templates

**Update tutorial campaign config (campaigns/tutorial/campaign.ron)**

- Add `starting_inn: 1` to `CampaignMetadata`

#### 1.4 Testing Requirements

**Unit tests (antares/src/application/mod.rs)**

- `test_initialize_roster_populates_starting_party`: Verify characters with `starts_in_party: true` are added to party
- `test_initialize_roster_sets_party_locations`: Verify party members have `CharacterLocation::InParty`
- `test_initialize_roster_sets_inn_locations`: Verify non-party premades have `CharacterLocation::AtInn(starting_inn)`
- `test_initialize_roster_party_overflow_error`: Verify error if >6 characters have `starts_in_party: true`
- `test_initialize_roster_respects_max_party_size`: Verify party size never exceeds `Party::MAX_MEMBERS`

**Integration test**

- `test_new_game_with_starting_party`: Full new game flow, verify party and roster are correctly populated

#### 1.5 Deliverables

- [ ] `CharacterLocation` enum added to `antares/src/domain/character.rs`
- [ ] `Roster` methods implemented (`find_character_by_id`, `update_location`, `characters_at_inn`, etc.)
- [ ] `starts_in_party` field added to `CharacterDefinition`
- [ ] `starting_inn` field added to `CampaignConfig` with default
- [ ] `initialize_roster` updated to populate party from `starts_in_party` characters
- [ ] Tutorial campaign data updated (3 starting party members)
- [ ] Tutorial campaign config updated with `starting_inn: 1`
- [ ] All Phase 1 unit tests passing
- [ ] Integration test passing

#### 1.6 Success Criteria

- Running `cargo run --bin antares -- --campaign campaigns/tutorial` shows 3 party members in HUD (Kira, Sage, Mira)
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo nextest run --all-features` passes with all Phase 1 tests green
- No breaking changes to existing save game format (migration needed for production)

---

### Phase 2: Party Management Domain Logic

#### 2.1 Party Transfer Operations

**Add `PartyManager` (new file: antares/src/domain/party_manager.rs)**

Core operations:

```rust
pub struct PartyManager;

impl PartyManager {
    /// Moves character from roster to party
    pub fn recruit_to_party(
        party: &mut Party,
        roster: &mut Roster,
        roster_index: usize,
    ) -> Result<(), PartyManagementError>;

    /// Moves character from party to inn
    pub fn dismiss_to_inn(
        party: &mut Party,
        roster: &mut Roster,
        party_index: usize,
        innkeeper_id: InnkeeperId,
    ) -> Result<Character, PartyManagementError>;

    /// Swaps party member with roster character (atomic operation)
    pub fn swap_party_member(
        party: &mut Party,
        roster: &mut Roster,
        party_index: usize,
        roster_index: usize,
    ) -> Result<(), PartyManagementError>;

    /// Validates party transfer is legal
    fn can_recruit(party: &Party, character: &Character) -> Result<(), PartyManagementError>;
}

#[derive(thiserror::Error, Debug)]
pub enum PartyManagementError {
    #[error("Party is full (max {0} members)")]
    PartyFull(usize),

    #[error("Party must have at least 1 member")]
    PartyEmpty,

    #[error("Character {0} not found in roster")]
    CharacterNotFound(usize),

    #[error("Character is already in party")]
    AlreadyInParty,

    #[error("Character is not at current inn")]
    NotAtInn,

    #[error("Invalid party index {0}")]
    InvalidPartyIndex(usize),
}
```

**Implementation details:**

- `recruit_to_party`:
  - Verify party not full
  - Verify character location is `AtInn(_)` or `OnMap(_)` (not already `InParty`)
  - Clone character from roster, add to party
  - Update roster location to `InParty`
- `dismiss_to_inn`:
  - Verify party size > 1 (cannot remove last member)
  - Remove from party by index
  5. Update roster location to `AtInn(InnkeeperId)` (uses string-based InnkeeperId)
  - Return removed character
- `swap_party_member`:
  - Atomic operation: remove from party, add to roster at inn, recruit from roster
  - Prevents edge case where party becomes empty mid-operation

#### 2.2 Game State Integration

**Update `GameState` (antares/src/application/mod.rs)**

Add party management methods:

```rust
impl GameState {
    /// Recruits character from current location to party
    pub fn recruit_character(&mut self, roster_index: usize) -> Result<(), PartyManagementError> {
        PartyManager::recruit_to_party(&mut self.party, &mut self.roster, roster_index)
    }

    /// Dismisses party member to current inn
    pub fn dismiss_character(&mut self, party_index: usize, innkeeper_id: InnkeeperId) -> Result<Character, PartyManagementError> {
        PartyManager::dismiss_to_inn(&mut self.party, &mut self.roster, party_index, innkeeper_id)
    }
}

    /// Swaps party member with roster character
    pub fn swap_party_member(&mut self, party_index: usize, roster_index: usize) -> Result<(), PartyManagementError> {
        PartyManager::swap_party_member(&mut self.party, &mut self.roster, party_index, roster_index)
    }

    /// Gets current inn ID from world state (helper)
    pub fn current_inn_id(&self) -> Option<TownId> {
        // Implementation depends on world/location system
        // For MVP, could track in GameState or infer from current_map
        None // TODO: Implement once Inn/Town system exists
    }
}
```

#### 2.3 Location Tracking Updates

**Update party modification methods**

Modify `Party::add_member` and `Party::remove_member` to accept `roster: &mut Roster` and update locations automatically (or require caller to update).

**Alternative: Keep Party/Roster independent, require GameState to coordinate**

- Cleaner separation of concerns
- GameState methods (`recruit_character`, etc.) handle location updates
- Party and Roster remain simple data structures

#### 2.4 Testing Requirements

**Unit tests (antares/src/domain/party_manager.rs)**

- `test_recruit_to_party_success`: Add character from inn to party
- `test_recruit_to_party_when_full`: Verify `PartyFull` error
- `test_recruit_already_in_party`: Verify `AlreadyInParty` error
- `test_dismiss_to_inn_success`: Remove from party, verify location update
- `test_dismiss_last_member_fails`: Verify `PartyEmpty` error
- `test_swap_party_member_atomic`: Verify swap operation maintains consistency
- `test_location_tracking_consistency`: Verify roster locations always match party state

**Integration tests (antares/src/application/mod.rs)**

- `test_game_state_recruit_character`: Full flow through GameState
- `test_game_state_dismiss_character`: Full flow with location update
- `test_party_management_maintains_invariants`: Verify roster and party stay synchronized

#### 2.5 Deliverables

- [ ] `PartyManager` module created with all core operations
- [ ] `PartyManagementError` enum with all error cases
- [ ] `GameState` integration methods implemented
- [ ] All Phase 2 unit tests passing
- [ ] Integration tests passing
- [ ] Documentation comments on all public methods

#### 2.6 Success Criteria

- `PartyManager::recruit_to_party` successfully moves character from inn to party
- `PartyManager::dismiss_to_inn` successfully moves character from party to inn
- Location tracking in `Roster` always reflects actual party state
- All error cases properly handled and tested
- No data corruption or inconsistent state possible

---

### Phase 3: Inn UI System (Bevy/egui)

#### 3.1 Inn Interaction Mode

**Add new `GameMode` variant (antares/src/application/mod.rs)**

```rust
pub enum GameMode {
    Exploration,
    Combat(CombatState),
    Menu,
    Dialogue(DialogueState),
    InnManagement(InnManagementState), // NEW
}

#[derive(Debug, Clone)]
pub struct InnManagementState {
    /// ID of the innkeeper NPC currently being visited
    pub current_inn_id: InnkeeperId,
    pub selected_party_slot: Option<usize>,
    pub selected_roster_slot: Option<usize>,
}
```

**Add Inn trigger to map events (antares/src/game/systems/map.rs)**

```rust
pub enum MapEventType {
    Teleport { target_map: MapId, target_pos: Position },
    NpcDialogue { npc_id: String },
    CombatEncounter { monster_group_id: u8 },
    TreasureChest { loot_table_id: u8 },
    EnterInn { innkeeper_id: InnkeeperId }, // NEW (uses InnkeeperId string)
}
```

#### 3.2 Inn UI Component

**Create `InnUiPlugin` (new file: antares/src/game/systems/inn_ui.rs)**

UI layout:

```
┌─────────────────────────────────────────────────────┐
│ Inn: [Inn Name] - Party Management                  │
├─────────────────────────────────────────────────────┤
│                                                     │
│ ACTIVE PARTY (Click to dismiss)                    │
│ ┌───────┐ ┌───────┐ ┌───────┐                     │
│ │ Kira  │ │ Sage  │ │ Mira  │ [Empty] [Empty] ... │
│ │ Lvl 1 │ │ Lvl 1 │ │ Lvl 1 │                     │
│ │ HP:10 │ │ HP:4  │ │ HP:10 │                     │
│ └───────┘ └───────┘ └───────┘                     │
│                                                     │
│ AVAILABLE AT THIS INN (Click to recruit)           │
│ ┌─────────┐ ┌─────────┐                           │
│ │ Gareth  │ │ Whisper │                           │
│ │ Dwarf   │ │ Elf     │                           │
│ │ Knight  │ │ Robber  │                           │
│ └─────────┘ └─────────┘                           │
│                                                     │
│ [Exit Inn]                                          │
└─────────────────────────────────────────────────────┘
```

**Core functionality:**

- Display party members (6 slots, empty slots shown)
- Display roster characters at current inn
- Click party member → Show "Dismiss to Inn" button
- Click roster member → Show "Add to Party" button (disabled if party full)
- Swap mode: Select party member, then roster member → Swap action
- Exit button → Return to `GameMode::Exploration`

**State management:**

- Store `InnManagementState` in Bevy resource
- Handle UI events via Bevy systems
- Send commands to `GlobalState` to modify party/roster

#### 3.3 Event Handling

**Create Bevy events (antares/src/game/systems/inn_ui.rs)**

```rust
#[derive(Event)]
pub struct InnRecruitCharacter {
    pub roster_index: usize,
}

#[derive(Event)]
pub struct InnDismissCharacter {
    pub party_index: usize,
}

#[derive(Event)]
pub struct InnSwapCharacters {
    pub party_index: usize,
    pub roster_index: usize,
}

#[derive(Event)]
pub struct ExitInn;
```

**System to process events**

```rust
fn inn_action_system(
    mut recruit_events: EventReader<InnRecruitCharacter>,
    mut dismiss_events: EventReader<InnDismissCharacter>,
    mut swap_events: EventReader<InnSwapCharacters>,
    mut exit_events: EventReader<ExitInn>,
    mut global_state: ResMut<GlobalState>,
    inn_state: Res<InnManagementState>,
    mut game_log: ResMut<GameLog>,
) {
    // Process recruit events
    for event in recruit_events.read() {
        match global_state.0.recruit_character(event.roster_index) {
            Ok(_) => game_log.add("Character recruited!".to_string()),
            Err(e) => game_log.add(format!("Cannot recruit: {}", e)),
        }
    }

    // Process dismiss events (similar)
    // Process swap events (similar)
    // Process exit events → set GameMode back to Exploration
}
```

#### 3.4 Map Integration

**Add Inn map event blueprint (campaigns/tutorial/data/maps/)**

Example RON for tutorial town map:

```ron
MapEventBlueprint(
    position: (5, 10),
    name: "Traveler's Rest Inn",
    description: "A cozy inn where adventurers gather",
    event_type: EnterInn(innkeeper_id: "tutorial_innkeeper_town"),
)
```

**Update event trigger system (antares/src/game/systems/events.rs)**

- Handle `MapEventType::EnterInn`
- Transition `GameMode` to `InnManagement(InnManagementState { current_inn_id, ... })`
- Show Inn UI overlay

#### 3.5 Testing Requirements

**UI tests (manual/screenshot tests)**

- Inn UI displays correctly with party members
- Inn UI shows available characters at current inn
- Recruit button works, updates party
- Dismiss button works, removes from party
- Swap operation works atomically
- Exit returns to exploration mode

**Integration tests**

- `test_inn_recruit_updates_game_state`: Verify event → state change
- `test_inn_ui_reflects_current_inn`: Only characters at current inn shown
- `test_inn_cannot_dismiss_last_member`: Verify error handling in UI

#### 3.6 Deliverables

- [ ] `InnManagementState` added to `GameMode`
- [ ] `MapEventType::EnterInn` added
- [ ] `InnUiPlugin` implemented with full UI
- [ ] Bevy events for recruit/dismiss/swap/exit
- [ ] Event processing system implemented
- [ ] Tutorial town map updated with Inn event
- [ ] Manual testing checklist completed

#### 3.7 Success Criteria

- Player can trigger Inn UI by stepping on Inn tile
- Inn UI shows current party and available characters
- Recruiting character from inn adds to party, updates location
- Dismissing character from party removes and updates location
- Party never exceeds 6 members, never goes below 1 member
- Exiting inn returns to exploration mode

---

### Phase 4: Map Encounter & Recruitment System

#### 4.1 Recruitable Character Encounters

**Add `MapEventType::RecruitableCharacter` (antares/src/game/systems/map.rs)**

```rust
pub enum MapEventType {
    // ... existing variants ...
    RecruitableCharacter {
        character_id: String, // CharacterDefinition ID (e.g., "npc_old_gareth")
    },
}
```

**Create recruitment dialogue flow**

When player triggers `RecruitableCharacter` event:

1. Show character portrait and description
2. Present dialogue: "Will you join our party?"
   - Yes, and party has room → Add to party immediately, set location `InParty`
   - Yes, but party full → "Meet us at [nearest inn]", set location `AtInn(nearest_inn_id)`
   - No → Character remains on map (`OnMap(current_map_id)`)

**Update `ContentDatabase` character handling (antares/src/sdk/database.rs)**

- Track which characters are recruitable NPCs (not premades, not templates)
- Load NPCs into roster when encountered (lazy loading)
- Add `recruit_npc(&mut self, character_id: &str) -> Result<Character, DatabaseError>`

#### 4.2 Encounter State Management

**Add encounter tracking to `GameState`**

```rust
impl GameState {
    /// Tracks which characters have been encountered on maps
    pub encountered_characters: HashSet<String>, // CharacterDefinition IDs

    /// Attempts to recruit character from map encounter
    pub fn recruit_from_map(
        &mut self,
        character_id: &str,
        content_db: &ContentDatabase,
    ) -> Result<RecruitResult, RecruitmentError>;
}

pub enum RecruitResult {
    AddedToParty,
    SentToInn(TownId),
    Declined,
}
```

**Prevent re-recruiting**

- Once character recruited or sent to inn, mark as encountered
- Future triggers of same map event show "already recruited" message
- Save `encountered_characters` in save game

#### 4.3 Nearest Inn Calculation

**Add `World::find_nearest_inn` (antares/src/domain/world/types.rs)**

```rust
impl World {
    /// Finds nearest inn to current party position
    pub fn find_nearest_inn(&self) -> Option<TownId> {
        // Simple implementation: Return inn on current map if exists
        // Advanced: Pathfinding to find closest inn across maps
        Some(1) // TODO: Implement proper inn discovery
    }
}
```

**Alternative: Campaign-defined fallback inn**

- Use `CampaignConfig::starting_inn` as default
- Avoids complex pathfinding for MVP

#### 4.4 Map Blueprint Integration

**Update tutorial campaign maps (campaigns/tutorial/data/maps/)**

Add recruitable character events:

```ron
MapEventBlueprint(
    position: (15, 8),
    name: "Old Gareth",
    description: "A grizzled dwarf veteran stands here",
    event_type: RecruitableCharacter(character_id: "npc_old_gareth"),
),

MapEventBlueprint(
    position: (22, 14),
    name: "Whisper",
    description: "A nimble elf watches from the shadows",
    event_type: RecruitableCharacter(character_id: "npc_whisper"),
),
```

#### 4.5 Recruitment Dialog UI

**Create recruitment dialog component (antares/src/game/systems/recruitment_dialog.rs)**

UI flow:

```
┌──────────────────────────────────────────┐
│ [Character Portrait]                     │
│                                          │
│ Old Gareth                               │
│ Dwarf Knight                             │
│                                          │
│ "A grizzled veteran who runs the town's │
│  smithy. Will you join our party?"      │
│                                          │
│ [Yes, join us!]  [Not now]              │
└──────────────────────────────────────────┘
```

**If party full:**

```
┌──────────────────────────────────────────┐
│ Your party is full!                      │
│                                          │
│ Old Gareth can meet you at:             │
│ Traveler's Rest Inn (Town Square)       │
│                                          │
│ [Send to inn]  [Cancel]                 │
└──────────────────────────────────────────┘
```

#### 4.6 Testing Requirements

**Unit tests**

- `test_recruit_from_map_adds_to_party`: Party has room → character added
- `test_recruit_from_map_sends_to_inn`: Party full → character sent to inn
- `test_recruit_from_map_prevents_duplicates`: Already recruited → error
- `test_encounter_tracking_persists`: `encountered_characters` saved/loaded

**Integration tests**

- `test_map_encounter_full_flow`: Trigger event → dialog → recruit → verify state
- `test_recruited_character_appears_at_inn`: Send to inn → verify shows in Inn UI

#### 4.7 Deliverables

- [ ] `MapEventType::RecruitableCharacter` added
- [ ] `recruit_from_map` implemented in `GameState`
- [ ] `encountered_characters` tracking added
- [ ] `World::find_nearest_inn` or fallback inn logic
- [ ] Recruitment dialog UI component
- [ ] Tutorial maps updated with NPC encounter events
- [ ] All Phase 4 unit tests passing
- [ ] Integration tests passing

#### 4.8 Success Criteria

- Player can encounter NPCs on maps (Gareth, Whisper, Zara)
- Recruitment dialog appears with character info
- Accepting adds to party if room, sends to inn if full
- Declining leaves character on map (can return later)
- Once recruited, character no longer appears on map
- Recruited NPCs appear at designated inn

---

### Phase 5: Persistence & Save Game Integration

#### 5.1 Save Game Format Updates

**Update save game schema (antares/src/application/save_game.rs)**

Ensure serialization includes:

- `Roster::character_locations` (now `Vec<CharacterLocation>` instead of `Vec<Option<TownId>>`)
- `GameState::encountered_characters` (new field)
- All character state (in party vs at inn)

**Migration strategy for existing saves**

- Detect old save format (missing `encountered_characters` field)
- Default `encountered_characters` to empty set
- Migrate `character_locations` from `Option<TownId>` to `CharacterLocation`
  - `None` → `CharacterLocation::InParty` if character is in party, else `CharacterLocation::AtInn(1)`
  - `Some(town_id)` → `CharacterLocation::AtInn(town_id)`

#### 5.2 Save/Load Testing

**Unit tests (antares/src/application/save_game.rs)**

- `test_save_party_locations`: Save game → load → verify party member locations preserved
- `test_save_inn_locations`: Save game → load → verify characters at inns preserved
- `test_save_encountered_characters`: Save game → load → verify encounter tracking preserved
- `test_save_migration_from_old_format`: Old save → load → verify migration successful

**Integration tests**

- `test_full_save_load_cycle`: New game → recruit characters → save → load → verify all state
- `test_party_management_persists`: Swap party → save → load → verify swapped state

#### 5.3 Deliverables

- [ ] Save game schema supports `CharacterLocation` enum
- [ ] Save game schema includes `encountered_characters`
- [ ] Migration code for old save format
- [ ] All Phase 5 unit tests passing
- [ ] Integration tests passing

#### 5.4 Success Criteria

- Saving game preserves all character locations (party, inn, map)
- Loading game restores exact party/roster state
- Encounter tracking persists across save/load
- Old save games can be loaded with migration (no data loss)

---

### Phase 6: Campaign SDK & Content Tools

#### 6.1 Character Definition Schema Updates

**Document `starts_in_party` field (docs/reference/campaign_content_format.md)**

Add documentation for new fields:

````markdown
## characters.ron Schema

### CharacterDefinition Fields

- `starts_in_party` (bool, optional, default: false): If true, this character
  will be added to the starting party when creating a new game. Maximum 6
  characters can have this flag set to true (enforced at campaign load time).

  Use case: Pre-made tutorial characters that should immediately be available.

  Example:

  ```ron
  (
      id: "tutorial_human_knight",
      name: "Kira",
      // ... other fields ...
      starts_in_party: true, // This character starts in party
  )
  ```
````

#### 6.2 Campaign Validation

**Add validation to `CampaignLoader` (antares/src/sdk/campaign_loader.rs)**

Validate on campaign load:

```rust
impl Campaign {
    /// Validates character definitions for party management constraints
    pub fn validate_characters(&self) -> Result<(), ValidationError> {
        let starting_party_count = self.content_db.characters
            .premade_characters()
            .filter(|c| c.starts_in_party)
            .count();

        if starting_party_count > 6 {
            return Err(ValidationError::TooManyStartingPartyMembers {
                count: starting_party_count,
                max: 6,
            });
        }

        Ok(())
    }
}
```

#### 6.3 Content Authoring Tools

**Add CLI validation command (antares/src/bin/campaign_validator.rs)**

```bash
# Validate campaign content
cargo run --bin campaign_validator -- --campaign campaigns/tutorial

# Output:
✓ Campaign structure valid
✓ 3 starting party members (max 6)
✓ 3 recruitable NPCs found
✓ Map events valid
✓ All character IDs referenced in maps exist in characters.ron
```

**Validation checks:**

- Starting party count <= 6
- All `RecruitableCharacter` event IDs exist in `characters.ron`
- All `EnterInn` event IDs are valid `TownId` values
- No duplicate character IDs

#### 6.4 Deliverables

- [ ] Character schema documentation updated
- [ ] Campaign validation implemented
- [ ] CLI validator tool created
- [ ] Tutorial campaign validated with tool

#### 6.5 Success Criteria

- Campaign authors can set `starts_in_party` flag
- Validation prevents invalid configurations (>6 starting party)
- CLI tool provides clear error messages for content issues

---

## Testing Strategy

### Unit Test Coverage

**Minimum coverage targets:**

- `CharacterLocation` enum: 100%
- `PartyManager` operations: 100%
- `Roster` location tracking methods: 100%
- Save/load character locations: 100%

### Integration Test Scenarios

1. **New Game Flow**: Start game → verify 3 party members, 3 NPCs at inn
2. **Inn Management**: Enter inn → recruit character → dismiss character → verify state
3. **Map Recruitment**: Trigger NPC encounter → recruit when full → verify sent to inn
4. **Save/Load**: Full flow → save → load → verify all state preserved
5. **Swap Party Members**: Swap party member with inn character → verify atomic operation

### Manual Testing Checklist

- [ ] Start new tutorial campaign, 3 characters in party
- [ ] Enter inn, see available characters
- [ ] Recruit character from inn to party
- [ ] Dismiss character from party to inn
- [ ] Swap party member with inn character
- [ ] Attempt to dismiss last party member (should fail)
- [ ] Attempt to recruit when party full (should fail)
- [ ] Encounter NPC on map, recruit to party
- [ ] Encounter NPC on map when party full, send to inn
- [ ] Save game with mixed party/inn characters
- [ ] Load game, verify party and inn characters correct
- [ ] Verify recruited NPC no longer appears on map

---

## Migration & Rollout

### Backwards Compatibility

**Breaking changes:**

- `Roster::character_locations` type changed: `Vec<Option<TownId>>` → `Vec<CharacterLocation>`
- `GameState` added field: `encountered_characters: HashSet<String>`

**Migration path:**

1. Version save game format (add `version: u8` field)
2. Detect version on load
3. If old version, run migration:
   - Convert `Option<TownId>` to `CharacterLocation`
   - Initialize `encountered_characters` to empty
4. Save migrated game in new format

### Phased Rollout

**Phase 1-2 (Core)**: Can ship independently, enables starting party
**Phase 3 (Inn UI)**: Requires Phase 1-2, enables full party management
**Phase 4 (Map Encounters)**: Requires Phase 1-3, enables NPC recruitment
**Phase 5 (Persistence)**: Required for production, ensures state preservation
**Phase 6 (SDK)**: Quality-of-life for content authors

**Minimum viable product (MVP)**: Phases 1-3 + 5
**Full feature set**: All phases

---

## Open Questions

### Question 1: Inn Identification

**How should inns be identified in the game world?**

**Option A**: Use `TownId` (current approach)

- Simple, already exists in type system
- Assumes 1 inn per town
- What if multiple inns in one town?

**Option B**: Create separate `InnId` type alias

- More specific, allows multiple inns per town
- Requires additional world building (inn database)
- More complex but more flexible

**Option C**: Use map-based identification (map ID + position)

- Most flexible, any map tile can be an inn
- Complex to track, harder to serialize
- Requires spatial lookup

**Recommendation**: Start with **Option A** (TownId), migrate to **Option B** if needed

### Question 2: Character State When Recruited from Map

**What happens to character's starting inventory/gold when recruited from map vs inn?**

**Option A**: Characters recruited from maps use their `CharacterDefinition` starting values

- Consistent with how premades work
- Characters always have expected starting gear

**Option B**: Characters recruited from map have reduced gear (simulating "adventure ready" vs "fresh recruit")

- More realistic, but requires defining "reduced" gear
- More complex content authoring

**Option C**: Map encounters can override starting gear via event parameters

- Maximum flexibility
- Most complex to implement and author

**Recommendation**: Start with **Option A**, add override capability later if needed

### Question 3: Re-encountering Characters

**What happens if player encounters a recruitable character, declines, then returns later?**

**Option A**: Character persists on map indefinitely until recruited

- Simple, player can always change mind
- Character never moves, feels static

**Option B**: Character disappears after N game days if not recruited

- More dynamic, encourages timely decisions
- Requires time-based event system

**Option C**: Character moves to inn automatically after being declined

- Simplest, character is always available somewhere
- Less immersive ("why did they go to the inn?")

**Recommendation**: Start with **Option A**, consider **Option C** for specific story scenarios

### Question 4: Party Member Dismissal Restrictions

**Should certain story-critical characters be un-dismissible?**

**Option A**: All characters can be dismissed (current plan)

- Maximum player freedom
- Risk of soft-locking if player dismisses everyone

**Option B**: Flag certain characters as "required" (cannot be dismissed)

- Prevents soft-locks
- Reduces player freedom

**Option C**: Warn player before dismissing last strong character (UX only)

- Informative but not restrictive
- Player can still make bad decisions

**Recommendation**: Start with **Option A** + minimum party size = 1 constraint, add "required" flag if story demands it

---

## Success Metrics

### Technical Metrics

- All quality checks pass: `cargo fmt`, `cargo check`, `cargo clippy`, `cargo nextest run`
- Test coverage >80% for all new modules
- Zero memory leaks or data races (verified by Miri if applicable)
- Save game migration success rate 100%

### Gameplay Metrics

- Player can start game with 3 party members (tutorial campaign)
- Player can recruit all 3 NPCs (Gareth, Whisper, Zara)
- Player can swap party members at inns without bugs
- Player can save/load with any party configuration
- No reports of lost characters or corrupted party state

### Content Authoring Metrics

- Campaign validator catches 100% of invalid configurations
- Content authors can set starting party via RON data
- Content authors can place recruitable NPCs on maps
- Documentation clear enough for external contributors

---

## Risk Assessment

### High Risk

- **Save game compatibility**: Breaking change to save format
  - Mitigation: Implement migration, test thoroughly, version saves
- **State synchronization bugs**: Roster and Party getting out of sync
  - Mitigation: Centralize all operations in `PartyManager`, extensive tests

### Medium Risk

- **UI complexity**: Inn management UI has many edge cases
  - Mitigation: Phased implementation, manual testing checklist
- **Performance**: Searching roster for characters at specific inn
  - Mitigation: Add indices/caches if needed (unlikely for roster size <20)

### Low Risk

- **Content authoring errors**: Invalid `starts_in_party` configurations
  - Mitigation: Validation at campaign load time, CLI validator tool
- **Map encounter edge cases**: Player exploits recruitment system
  - Mitigation: Track encounters per character, prevent re-recruitment

---

## Dependencies

### External Crates

No new dependencies required. Uses existing:

- `bevy` (game engine)
- `bevy_egui` (UI)
- `serde` (serialization)
- `thiserror` (error handling)

### Internal Systems

**Phase 1** depends on:

- Existing `Roster`, `Party`, `Character` structures
- Existing `ContentDatabase` and `CharacterDefinition`

**Phase 2** depends on:

- Phase 1 (CharacterLocation enum)

**Phase 3** depends on:

- Phase 2 (PartyManager operations)
- Existing Bevy UI systems

**Phase 4** depends on:

- Phase 3 (Inn system for overflow handling)
- Existing map event system

**Phase 5** depends on:

- All previous phases (complete feature to persist)

**Phase 6** depends on:

- All previous phases (validation requires complete schema)

---

## Timeline Estimate

**Phase 1**: 4-6 hours (data model + starting party)
**Phase 2**: 6-8 hours (domain logic + tests)
**Phase 3**: 8-12 hours (UI implementation + event handling)
**Phase 4**: 6-8 hours (map encounters + dialog)
**Phase 5**: 4-6 hours (save/load + migration)
**Phase 6**: 2-4 hours (validation + documentation)

**Total**: 30-44 hours (approximately 1 week of focused development)

**MVP (Phases 1-3+5)**: ~22-32 hours (3-4 days)

---

## Conclusion

This implementation plan provides a comprehensive roadmap for adding party management to Antares. By following a phased approach, we can deliver incremental value while maintaining code quality and test coverage. The design leverages existing systems (Roster, Party, CharacterDefinition) and extends them with minimal breaking changes. The result will be a flexible, content-author-friendly system for managing party composition throughout the game.
