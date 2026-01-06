# Innkeeper ID Migration Implementation Plan

## Overview

Migrate the inn location system from arbitrary numeric IDs (`TownId = u8`) to meaningful string-based innkeeper NPC IDs (`InnkeeperId = String`). This change enables validation of inn locations, better UI/UX (show innkeeper names/portraits), and clearer campaign authoring by tying inn locations to actual NPCs in the game world.

**Current State**: Inn locations use numeric IDs (1-255) with no validation or meaning. `TownId` type alias exists but serves no purpose beyond inn tracking.

**Target State**: Inn locations reference innkeeper NPC IDs (e.g., "tutorial_innkeeper_town"), validated against the NPC database. `TownId` type removed entirely.

**Breaking Changes**: This is a breaking change for save games and campaign data. Since only the tutorial campaign exists and backwards compatibility is not required, we will simply update all data files and documentation.

## Current State Analysis

### Existing Infrastructure

**Type System** (`src/domain/types.rs` lines 39-40):

- `TownId = u8` - numeric identifier currently only used for inn tracking
- `NpcId = String` - string identifier for NPCs (defined in `src/domain/world/npc.rs` line 48)
- `MapId = u16` - map identifier (sufficient for location tracking)

**Character Location Tracking** (`src/domain/character.rs` lines 436-445):

```rust
pub enum CharacterLocation {
    InParty,
    AtInn(InnkeeperId),      // Uses string-based InnkeeperId (NpcId)
    OnMap(MapId),        // Uses MapId for location
}
```

**Map Events** (`src/domain/world/types.rs` lines 495-506):

```rust
EnterInn {
    name: String,
    description: String,
    innkeeper_id: crate::domain::world::NpcId,         // Innkeeper NPC identifier (NpcId string)
}
```

**Campaign Configuration**:

- `src/sdk/campaign_loader.rs` line ~156: `starting_inn: u8`
- `sdk/campaign_builder/src/lib.rs` line 144: `starting_inn: u8`

**NPC System** (`src/domain/world/npc.rs` lines 79-113):

- `NpcDefinition` has `is_innkeeper: bool` flag
- NPCs use string IDs (e.g., "tutorial_innkeeper_town")
- Tutorial campaign has 2 innkeeper NPCs defined (`campaigns/tutorial/data/npcs.ron`)

**Campaign Data**:

- Tutorial campaign: `campaigns/tutorial/` (contains campaign.ron and data/)
- Test data: `data/` directory (no inn-related content found)

### Identified Issues

1. **No Validation**: Numeric inn IDs have no backing data structure - cannot validate if ID is valid
2. **Poor UX**: Cannot show innkeeper name or portrait in UI without manual mapping
3. **Confusing for Authors**: Campaign authors must remember that ID "1" maps to which inn
4. **Type Mismatch**: `TownId` (u8) vs `NpcId` (String) - incompatible types
5. **Unused Type**: `TownId` serves no purpose other than inn tracking; `MapId` handles location needs
6. **Multiple Sources of Truth**: Map events define `innkeeper_id` (NpcId string) separately from NPC definitions

## Implementation Phases

### Phase 1: Type System Foundation

#### 1.1 Remove TownId and Add InnkeeperId

**File**: `src/domain/types.rs` (lines 36-40)

**Current**:

```rust
/// Town identifier
pub type TownId = u8;
```

**Action**: Remove `TownId` entirely and add `InnkeeperId`:

```rust
/// Innkeeper NPC identifier (references NpcId with is_innkeeper=true)
pub type InnkeeperId = String;
```

**Rationale**: `TownId` is only used for inn tracking. `MapId` already handles location tracking for maps. No need for a separate town concept.

#### 1.2 Update CharacterLocation Enum

**File**: `src/domain/character.rs` (lines 436-445)

**Current**:

```rust
pub enum CharacterLocation {
    InParty,
    AtInn(TownId),
    OnMap(MapId),
}
```

**Changes**:

```rust
pub enum CharacterLocation {
    /// Character is in the active party
    InParty,

    /// Character is stored at a specific inn (references an innkeeper NPC)
    AtInn(InnkeeperId),  // Changed from TownId(u8) to InnkeeperId(String)

    /// Character is available on a specific map (for recruitment encounters)
    OnMap(MapId),
}
```

**Impact**: All code using `CharacterLocation::AtInn` must be updated to use String type.

#### 1.3 Update MapEvent Enum

**File**: `src/domain/world/types.rs` (lines 495-506)

**Current**:

```rust
EnterInn {
    name: String,
    description: String,
    innkeeper_id: String,  // Uses NPC ID strings (changed from inn_id: u8)
},
```

**Changes**:

```rust
/// Enter an inn for party management
EnterInn {
    /// Event name
    #[serde(default)]
    name: String,
    /// Event description
    #[serde(default)]
    description: String,
    /// Innkeeper NPC identifier (must exist in NPC database with is_innkeeper=true)
    innkeeper_id: String,  // Changed from inn_id: u8
},
```

#### 1.4 Update Public Exports

**File**: `src/domain/mod.rs` (line 42)

**Current**:

```rust
pub use types::{CharacterId, EventId, ItemId, MapId, MonsterId, RaceId, SpellId, TownId};
```

**Changes**:

```rust
pub use types::{CharacterId, EventId, InnkeeperId, ItemId, MapId, MonsterId, RaceId, SpellId};
```

**File**: `src/lib.rs` (line 32)

**Current**:

```rust
pub use domain::types::{CharacterId, EventId, ItemId, MapId, MonsterId, SpellId, TownId};
```

**Changes**:

```rust
pub use domain::types::{CharacterId, EventId, InnkeeperId, ItemId, MapId, MonsterId, SpellId};
```

#### 1.5 Remove All TownId Imports

**Files to update**:

- `src/application/mod.rs` (line 21): Remove `TownId` from import
- `src/application/save_game.rs` (line 391): Remove `TownId` from import
- `src/domain/character.rs` (line 15): Remove `TownId` from import
- `src/domain/party_manager.rs` (line 35): Remove `TownId` from import

**Action**: Search and replace all `use.*TownId` imports with `InnkeeperId` where needed.

#### 1.6 Testing Requirements

**Unit Tests**:

- `test_character_location_atinn_string_id` - Verify AtInn serializes/deserializes as String
- `test_innkeeper_id_type_alias` - Verify InnkeeperId is String type
- `test_map_event_enter_inn_string_id` - Verify EnterInn serializes with innkeeper_id
- `test_no_townid_in_codebase` - Verify TownId is completely removed

#### 1.7 Deliverables

- [ ] `TownId` type alias removed from `src/domain/types.rs`
- [ ] `InnkeeperId` type alias added to `src/domain/types.rs`
- [ ] `CharacterLocation::AtInn` updated to use `InnkeeperId`
- [ ] `MapEvent::EnterInn` updated to use `innkeeper_id: String`
- [ ] Public exports updated in `mod.rs` and `lib.rs`
- [ ] All `TownId` imports removed from codebase
- [ ] All Phase 1 unit tests passing

#### 1.8 Success Criteria

- `grep -r "TownId" src/` returns zero results
- `cargo check --all-features` compiles (may have errors in application logic - fixed in Phase 2)
- Type system is internally consistent
- `CharacterLocation` and `MapEvent` use String types

---

### Phase 2: Application Logic Updates

#### 2.1 Update InnManagementState

**File**: `src/application/mod.rs` (lines 69-76)

**Current**:

```rust
pub struct InnManagementState {
    pub current_inn_id: TownId,
    pub selected_party_slot: Option<usize>,
    pub selected_roster_slot: Option<usize>,
}
```

**Changes**:

```rust
pub struct InnManagementState {
    /// ID of the innkeeper NPC currently being visited
    pub current_inn_id: InnkeeperId,  // Changed from TownId to InnkeeperId
    /// Currently selected party member slot (0-5) for swap operations
    pub selected_party_slot: Option<usize>,
    /// Currently selected roster index for swap operations
    pub selected_roster_slot: Option<usize>,
}
```

**Update Constructor** (lines 88-99):

```rust
pub fn new(innkeeper_id: InnkeeperId) -> Self {  // Changed parameter type
    Self {
        current_inn_id: innkeeper_id,
        selected_party_slot: None,
        selected_roster_slot: None,
    }
}
```

**Update all usages in tests**:

- Line ~64: `InnManagementState::new(TownId::from(1))` → `InnManagementState::new("tutorial_innkeeper_town".to_string())`
- Line ~90: Similar updates in doc examples

#### 2.2 Update GameState Methods

**File**: `src/application/mod.rs`

**Method: `dismiss_character`** (lines 661-665):

**Current**:

```rust
pub fn dismiss_character(
    &mut self,
    party_index: usize,
    innkeeper_id: InnkeeperId,
) -> Result<Character, PartyManagementError>
```

**Changes**:

```rust
pub fn dismiss_character(
    &mut self,
    party_index: usize,
    innkeeper_id: InnkeeperId,  // Changed from TownId to InnkeeperId
) -> Result<Character, PartyManagementError>
```

**Method: `current_inn_id`** (lines 730-737):

**Current**:

```rust
pub fn current_inn_id(&self) -> Option<TownId> {
    // TODO: Implementation
    None
}
```

**Changes**:

```rust
pub fn current_inn_id(&self) -> Option<InnkeeperId> {  // Changed return type
    // TODO: Extract from current map's EnterInn event or game state
    // For now, return None or default innkeeper
    None
}
```

**Method: `find_nearest_inn`** (lines 764-769):

**Current**:

```rust
pub fn find_nearest_inn(&self) -> Option<TownId> {
    self.campaign.as_ref().map(|c| c.config.starting_inn)
}
```

**Changes**:

```rust
pub fn find_nearest_inn(&self) -> Option<InnkeeperId> {  // Changed return type
    // Return campaign's starting_innkeeper
    self.campaign.as_ref().map(|c| c.config.starting_innkeeper.clone())
}
```

**Method: `recruit_from_map`** (lines 825-875):

**Update line ~867**:

```rust
// Former (legacy numeric form):
self.roster.add_character(character, CharacterLocation::AtInn("tutorial_innkeeper_town"))?;

// New:
self.roster.add_character(character, CharacterLocation::AtInn(innkeeper_id))?;
```

**Method: `initialize_roster`** (lines ~567-570):

**Update to use String innkeeper ID**:

```rust
// Old:
CharacterLocation::AtInn(starting_inn)

// New:
CharacterLocation::AtInn(starting_innkeeper.clone())
```

#### 2.3 Update PartyManager

**File**: `src/domain/party_manager.rs`

**Update import** (line 35):

```rust
// Old:
use crate::domain::types::TownId;

// New:
use crate::domain::types::InnkeeperId;
```

**Method: `dismiss_to_inn`** (lines 194-199):

**Current**:

```rust
pub fn dismiss_to_inn(
    party: &mut Party,
    roster: &mut Roster,
    party_index: usize,
    innkeeper_id: InnkeeperId,
) -> Result<Character, PartyManagementError>
```

**Changes**:

```rust
pub fn dismiss_to_inn(
    party: &mut Party,
    roster: &mut Roster,
    party_index: usize,
    innkeeper_id: InnkeeperId,  // Changed from TownId to InnkeeperId
) -> Result<Character, PartyManagementError>
```

**Update implementation**:

```rust
// Update line ~204:
*roster_location = CharacterLocation::AtInn(innkeeper_id);
```

**Method: `swap_party_member`** (lines ~350-354):

**Update to preserve String innkeeper ID**:

```rust
let dismissed_location = match roster_location {
    CharacterLocation::AtInn(innkeeper_id) => CharacterLocation::AtInn(innkeeper_id.clone()),
    CharacterLocation::OnMap(map_id) => CharacterLocation::OnMap(map_id),
    CharacterLocation::InParty => CharacterLocation::AtInn("default_innkeeper".to_string()), // Fallback
};
```

#### 2.4 Update Roster Methods

**File**: `src/domain/character.rs`

**Method: `characters_at_inn`** (lines 1335-1350):

**Current**:

```rust
pub fn characters_at_inn(&self, town_id: TownId) -> Vec<(usize, &Character)> {
    // ...
    if *tid == town_id {
    // ...
}
```

**Changes**:

```rust
pub fn characters_at_inn(&self, innkeeper_id: &str) -> Vec<(usize, &Character)> {
    self.character_locations
        .iter()
        .enumerate()
        .filter_map(|(idx, loc)| {
            if let CharacterLocation::AtInn(id) = loc {
                if id == innkeeper_id {  // String comparison
                    return self.characters.get(idx).map(|c| (idx, c));
                }
            }
            None
        })
        .collect()
}
```

**Update doc example**:

```rust
// Update example to use string ID:
let at_inn = roster.characters_at_inn("tutorial_innkeeper_town");
```

#### 2.5 Update Event Handling

**File**: `src/domain/world/events.rs`

**Update import**: Add `InnkeeperId` if needed

**Function: `trigger_event`** (lines ~214-219):

**Current**:

```rust
MapEvent::EnterInn { innkeeper_id, .. } => {
    EventResult::EnterInn { innkeeper_id }
}
```

**Changes**:

```rust
MapEvent::EnterInn { innkeeper_id, .. } => {
    // Inn entrances are repeatable - don't remove
    EventResult::EnterInn { innkeeper_id: innkeeper_id.clone() }
}
```

**Update EventResult enum**:

**File**: `src/domain/world/events.rs` (around line ~60-64)

**Current**:

```rust
EnterInn {
    innkeeper_id: crate::domain::world::NpcId,
},
```

**Changes**:

```rust
EnterInn {
    innkeeper_id: String,
},
```

#### 2.6 Update Game Systems

**File**: `src/game/systems/events.rs`

**Update handler** (lines ~139-143):

**Current**:

```rust
MapEvent::RecruitableCharacter {
    character_id,
    name,
```

**Find EnterInn handler and update to use `innkeeper_id` (String)**

#### 2.7 Update All Tests

**Search for test usages**:

```bash
grep -r "TownId::" src/
grep -r "AtInn(" src/
grep -r "innkeeper_id:" src/
```

**Update all test cases to use String IDs**:

- `AtInn(1)` → `AtInn("tutorial_innkeeper_town".to_string())`
- Use `EnterInn { innkeeper_id: "tutorial_innkeeper_town" }` (new format). Legacy numeric `inn_id: 1` should be migrated to the innkeeper string form.

**Key test files**:

- `src/application/mod.rs` (tests at end of file)
- `src/application/save_game.rs` (tests)
- `src/domain/character.rs` (tests)
- `src/domain/party_manager.rs` (tests)
- `src/domain/world/events.rs` (tests)

#### 2.8 Testing Requirements

**Unit Tests**:

- `test_dismiss_character_with_innkeeper_id` - Verify dismiss uses string ID
- `test_characters_at_inn_string_id` - Verify filtering by string ID works
- `test_recruit_from_map_sends_to_innkeeper` - Verify recruit uses string ID
- `test_enter_inn_event_with_innkeeper_id` - Verify event uses string ID
- `test_inn_management_state_string_id` - Verify state stores string ID

**Integration Tests**:

- `test_inn_management_workflow` - Full workflow with string IDs

#### 2.9 Deliverables

- [ ] `InnManagementState` updated to use `InnkeeperId`
- [ ] `GameState` methods updated (`dismiss_character`, `current_inn_id`, `find_nearest_inn`, `recruit_from_map`, `initialize_roster`)
- [ ] `PartyManager::dismiss_to_inn` and `swap_party_member` updated
- [ ] `Roster::characters_at_inn` updated to use `&str` parameter
- [ ] Event handling updated in `events.rs` and game systems
- [ ] All test cases updated to use string IDs
- [ ] All Phase 2 unit tests passing
- [ ] All Phase 2 integration tests passing

#### 2.10 Success Criteria

- All application logic compiles without errors
- `cargo check --all-targets --all-features` passes
- All tests pass with string innkeeper IDs
- No numeric inn IDs remain in domain/application layers
- `grep -r "TownId" src/` returns zero results
- `grep -r 'AtInn([0-9])' src/` returns zero results (no numeric IDs)

---

### Phase 3: Save/Load System Updates

#### 3.1 Update SaveGame Format

**File**: `src/application/save_game.rs`

**Serialization Impact**: `CharacterLocation::AtInn` now serializes as `AtInn(String)` instead of `AtInn(u8)`.

**RON Format Change**:

- **Old**: `AtInn(1)`
- **New**: `AtInn("tutorial_innkeeper_town")`

**No Migration Needed**: Since backwards compatibility is not required, simply update save/load to work with new format.

#### 3.2 Update Save/Load Tests

**Update all save/load tests** to use string IDs:

**File**: `src/application/save_game.rs` (tests section)

**Example changes**:

```rust
// Old:
let inn1: TownId = 1;
roster.add_character(char1, CharacterLocation::AtInn(inn1));

// New:
roster.add_character(char1, CharacterLocation::AtInn("tutorial_innkeeper_town".to_string()));
```

**Key test functions to update**:

- `test_save_inn_locations` (line ~649)
- `test_save_full_roster_state` (line ~846)
- `test_save_load_preserves_character_invariants` (line ~920)
- `test_save_load_character_sent_to_inn` (line ~2253)

#### 3.3 Verify RON Serialization

**Test**: Create a simple test to verify RON round-trip:

```rust
#[test]
fn test_character_location_ron_serialization() {
    let location = CharacterLocation::AtInn("test_innkeeper".to_string());

    // Serialize to RON
    let ron_string = ron::to_string(&location).unwrap();
    assert!(ron_string.contains("test_innkeeper"));

    // Deserialize from RON
    let deserialized: CharacterLocation = ron::from_str(&ron_string).unwrap();
    assert_eq!(location, deserialized);
}
```

#### 3.4 Testing Requirements

**Tests**:

- `test_save_load_with_innkeeper_id` - Verify round-trip with string IDs
- `test_character_location_ron_serialization` - Verify RON format
- `test_save_game_format` - Document expected RON structure

#### 3.5 Deliverables

- [ ] Save/load system handles `AtInn(String)` correctly
- [ ] All save/load tests updated to use string IDs
- [ ] RON serialization tests passing
- [ ] All Phase 3 tests passing

#### 3.6 Success Criteria

- Save games serialize/deserialize correctly with string IDs
- `cargo test --lib save_game` passes all tests
- RON format is human-readable and correct

---

### Phase 4: Campaign Configuration Updates

#### 4.1 Update CampaignConfig

**File**: `src/sdk/campaign_loader.rs` (lines ~156-157)

**Current**:

```rust
#[serde(default = "default_starting_inn")]
pub starting_inn: u8,
```

**Changes**:

```rust
/// Default innkeeper where non-party premade characters start
#[serde(default = "default_starting_innkeeper")]
pub starting_innkeeper: String,
```

**Update default function**:

**Current**:

```rust
fn default_starting_inn() -> u8 {
    1
}
```

**Changes**:

```rust
fn default_starting_innkeeper() -> String {
    "tutorial_innkeeper_town".to_string()
}
```

#### 4.2 Update CampaignMetadata

**File**: `src/sdk/campaign_loader.rs` (lines ~427-428)

**Current**:

```rust
#[serde(default = "default_starting_inn")]
pub starting_inn: u8,
```

**Changes**:

```rust
#[serde(default = "default_starting_innkeeper")]
pub starting_innkeeper: String,
```

**Update conversion** (`TryFrom<CampaignMetadata> for Campaign` impl):

**Find and update**:

```rust
// Old:
starting_inn: metadata.starting_inn,

// New:
starting_innkeeper: metadata.starting_innkeeper.clone(),
```

#### 4.3 Update SDK CampaignMetadata

**File**: `sdk/campaign_builder/src/lib.rs` (line 144)

**Current**:

```rust
#[serde(default = "default_starting_inn")]
starting_inn: u8,
```

**Changes**:

```rust
#[serde(default = "default_starting_innkeeper")]
starting_innkeeper: String,
```

**Update default function** (add after line 195):

```rust
fn default_starting_innkeeper() -> String {
    "tutorial_innkeeper_town".to_string()
}
```

**Update default implementation** (line 211):

**Current**:

```rust
starting_inn: 1,
```

**Changes**:

```rust
starting_innkeeper: "tutorial_innkeeper_town".to_string(),
```

**Update test** (line 4919):

**Current**:

```rust
starting_inn: 1,
```

**Changes**:

```rust
starting_innkeeper: "tutorial_innkeeper_town".to_string(),
```

#### 4.4 Add Validation

**File**: `src/sdk/validation.rs` (lines ~388-407)

**Add to `validate_characters` method**:

```rust
// Validate starting_innkeeper exists and is an innkeeper
if let Some(npc) = self.db.npcs.get_npc(&self.db.campaign.config.starting_innkeeper) {
    if !npc.is_innkeeper {
        errors.push(ValidationError::InvalidStartingInnkeeper {
            innkeeper_id: self.db.campaign.config.starting_innkeeper.clone(),
            reason: format!(
                "NPC '{}' exists but is not marked as is_innkeeper=true",
                npc.name
            ),
        });
    }
} else {
    errors.push(ValidationError::InvalidStartingInnkeeper {
        innkeeper_id: self.db.campaign.config.starting_innkeeper.clone(),
        reason: "NPC not found in database".to_string(),
    });
}
```

**Add new error variant** to `ValidationError` enum:

```rust
#[error("Invalid starting innkeeper '{innkeeper_id}': {reason}")]
InvalidStartingInnkeeper {
    innkeeper_id: String,
    reason: String,
},
```

#### 4.5 Testing Requirements

**Tests**:

- `test_campaign_config_starting_innkeeper` - Verify field exists and is String
- `test_validate_starting_innkeeper_exists` - Verify validation rejects missing NPC
- `test_validate_starting_innkeeper_is_innkeeper` - Verify validation rejects non-innkeeper
- `test_default_starting_innkeeper` - Verify default value

#### 4.6 Deliverables

- [x] `CampaignConfig::starting_innkeeper` field added (String type)
- [x] `CampaignMetadata::starting_innkeeper` field added (both in antares and SDK)
- [x] Default value set to "tutorial_innkeeper_town"
- [x] Validation logic added to check innkeeper exists and has `is_innkeeper=true`
- [x] `ValidationError::InvalidStartingInnkeeper` variant added
- [x] All Phase 4 tests passing

#### 4.7 Success Criteria

- Campaign configuration uses string innkeeper IDs
- Validation prevents invalid innkeeper IDs in campaigns
- Default value works with tutorial campaign
- `cargo test --lib sdk::validation` passes

---

### Phase 5: SDK UI Updates

#### 5.1 Update CampaignMetadataEditBuffer

**File**: `sdk/campaign_builder/src/campaign_editor.rs` (line 92)

**Current**:

```rust
pub starting_inn: u8,
```

**Changes**:

```rust
pub starting_innkeeper: String,
```

**Update `from_metadata`** (line 129):

**Current**:

```rust
starting_inn: m.starting_inn,
```

**Changes**:

```rust
starting_innkeeper: m.starting_innkeeper.clone(),
```

**Update `apply_to`** (line 163):

**Current**:

```rust
dest.starting_inn = self.starting_inn;
```

**Changes**:

```rust
dest.starting_innkeeper = self.starting_innkeeper.clone();
```

#### 5.2 Update UI Control

**File**: `sdk/campaign_builder/src/campaign_editor.rs` (lines ~879-889)

**Current** (DragValue for numeric input):

```rust
ui.label("Starting Inn:")
    .on_hover_text("Default inn where non-party premade characters start (default: 1)");
let mut inn = self.buffer.starting_inn as i32;
if ui
    .add(egui::DragValue::new(&mut inn).range(1..=255))
    .changed()
{
    self.buffer.starting_inn = (inn.max(1)) as u8;
    self.has_unsaved_changes = true;
    *unsaved_changes = true;
}
ui.end_row();
```

**Changes** (ComboBox for NPC selection):

```rust
ui.label("Starting Innkeeper:")
    .on_hover_text("Default innkeeper NPC where non-party premade characters start");

// Get available innkeepers from NPC list
let innkeepers: Vec<_> = self
    .app
    .npc_editor_state
    .npcs
    .iter()
    .filter(|npc| npc.is_innkeeper)
    .collect();

egui::ComboBox::from_id_salt("starting_innkeeper")
    .selected_text(
        innkeepers
            .iter()
            .find(|npc| npc.id == self.buffer.starting_innkeeper)
            .map(|npc| format!("{} ({})", npc.name, npc.id))
            .unwrap_or_else(|| self.buffer.starting_innkeeper.clone())
    )
    .show_ui(ui, |ui| {
        for npc in &innkeepers {
            if ui
                .selectable_label(
                    self.buffer.starting_innkeeper == npc.id,
                    format!("{} ({})", npc.name, npc.id)
                )
                .clicked()
            {
                self.buffer.starting_innkeeper = npc.id.clone();
                self.has_unsaved_changes = true;
                *unsaved_changes = true;
            }
        }
    });
ui.end_row();
```

**Note**: The exact implementation depends on how NPCs are accessed in the campaign editor. May need to pass NPCs as a parameter to the `show` method.

#### 5.3 Update Method Signature

**File**: `sdk/campaign_builder/src/campaign_editor.rs`

**If needed**, add NPCs parameter to `show` method to enable NPC dropdown in UI.

**Alternative**: Access NPCs from the app state if already available.

#### 5.4 Testing Requirements

**Tests**:

- `test_starting_innkeeper_ui_updates_buffer` - Verify UI updates field
- `test_starting_innkeeper_persists` - Verify value saves to campaign.ron
- `test_starting_innkeeper_dropdown_filters_innkeepers` - Verify only innkeepers shown

#### 5.5 Deliverables

- [x] `CampaignMetadataEditBuffer::starting_innkeeper` field updated to String
- [x] UI control changed from DragValue to ComboBox/dropdown (shows Name (ID) for clarity)
- [x] UI filters NPCs to only show `is_innkeeper=true` (falls back to manual text input if no innkeepers are loaded)
- [x] UI shows innkeeper name and ID for clarity
- [x] All Phase 5 tests passing

**Note**: When no innkeeper NPCs are available in the editor, the UI gracefully falls back to a single-line text input so authors can manually enter an innkeeper ID.

#### 5.6 Success Criteria

- Campaign Builder UI allows selecting innkeeper from NPC list
- Only NPCs with `is_innkeeper=true` are shown in dropdown
- Selected innkeeper ID is saved to campaign.ron
- UI is intuitive and shows helpful information (name + ID), and falls back to a manual text input when no innkeeper NPCs are available
- Phase 5 unit/UI tests pass (editor buffer behavior and round-trip persistence verified)

---

### Phase 6: Campaign Data Migration

#### 6.1 Update Tutorial Campaign Configuration

**File**: `campaigns/tutorial/campaign.ron` (line 13)

**Current**:

```ron
starting_inn: 1,
```

**Changes**:

```ron
starting_innkeeper: "tutorial_innkeeper_town",
```

**Or if field is missing** (relies on default), add it explicitly:

```ron
starting_innkeeper: "tutorial_innkeeper_town",
```

#### 6.2 Update Tutorial Map Events

**File**: `campaigns/tutorial/data/maps/map_1.ron` (lines ~7680-7686)

**Current**:

```ron
(
    x: 5,
    y: 4,
): EnterInn(
    name: "Cozy Inn Entrance",
    description: "A welcoming inn where you can rest and manage your party.",
    innkeeper_id: "tutorial_innkeeper_town",
),
```

**Changes**:

```ron
(
    x: 5,
    y: 4,
): EnterInn(
    name: "Cozy Inn Entrance",
    description: "A welcoming inn where you can rest and manage your party.",
    innkeeper_id: "tutorial_innkeeper_town",
),
```

**Search for all EnterInn events**:

```bash
grep -r "EnterInn" campaigns/tutorial/data/maps/
```

**Update each occurrence** to use `innkeeper_id: "tutorial_innkeeper_town"`.

#### 6.3 Verify Tutorial Innkeeper NPC

**File**: `campaigns/tutorial/data/npcs.ron` (lines 16-26)

**Verify NPC exists**:

```ron
(
    id: "tutorial_innkeeper_town",
    name: "InnKeeper",
    description: "The proprietor of the Cozy Inn who offers rooms and warm meals to travelers.",
    portrait_id: "inkeeper_1",
    dialogue_id: None,
    quest_ids: [],
    faction: Some("Innkeepers Guild"),
    is_merchant: false,
    is_innkeeper: true,  // Must be true
),
```

**Confirm**:

- `id: "tutorial_innkeeper_town"` matches campaign config
- `is_innkeeper: true` is set

**Check for other innkeepers** (line ~104-108):

```ron
(
    id: "tutorial_innkeeper_hideout",
    ...
    is_innkeeper: true,
),
```

**If multiple innkeepers exist**, verify all map events reference valid innkeeper IDs.

#### 6.4 Testing Requirements

**Tests**:

- `test_tutorial_campaign_loads` - Verify campaign loads without errors
- `test_tutorial_inn_event_valid` - Verify EnterInn event references valid innkeeper
- `test_tutorial_starting_innkeeper_exists` - Verify starting_innkeeper NPC exists
- `test_tutorial_new_game_initializes` - Verify new game with updated campaign works
- `test_tutorial_party_initialization` - Verify 3 characters in party, others at inn

**Integration Test**:

- `test_tutorial_full_workflow` - Start game, enter inn, recruit/dismiss characters

#### 6.5 Deliverables

- [ ] `campaigns/tutorial/campaign.ron` updated with `starting_innkeeper: "tutorial_innkeeper_town"`
- [ ] All `EnterInn` events in tutorial maps updated to use `innkeeper_id`
- [ ] Tutorial innkeeper NPCs verified to have `is_innkeeper: true`
- [ ] All Phase 6 tests passing

#### 6.6 Success Criteria

- Tutorial campaign loads successfully
- `cargo run --bin antares -- --campaign campaigns/tutorial` starts without errors
- Party management at inn works with string innkeeper IDs
- Validation passes for tutorial campaign
- Can enter inn via map event
- Can recruit/dismiss characters at inn

---

### Phase 7: Documentation and Final Validation

#### 7.1 Update Campaign Content Format Documentation

**File**: `docs/reference/campaign_content_format.md`

**Add section**:

````markdown
### Inn and Innkeeper System

Inn locations are referenced by innkeeper NPC IDs (String type), not numeric IDs.

**Campaign Configuration**:

- `starting_innkeeper: String` - NPC ID of the default innkeeper where non-party characters start
  - Must reference an NPC with `is_innkeeper: true`
  - Example: `"tutorial_innkeeper_town"`
  - Default: `"tutorial_innkeeper_town"`

**Map Events**:

- `EnterInn { innkeeper_id: String, ... }` - Triggers inn management interface
  - Must reference an NPC with `is_innkeeper: true`
  - Example: `innkeeper_id: "tutorial_innkeeper_town"`

**NPC Definition**:

- `is_innkeeper: bool` - Marks NPC as an innkeeper who can manage party roster
  - Required for NPCs referenced by `starting_innkeeper` or `EnterInn` events

**Character Location Tracking**:

- `CharacterLocation::AtInn(InnkeeperId)` - Character is stored at specified innkeeper's inn
  - Uses string innkeeper NPC ID
  - Example: `AtInn("tutorial_innkeeper_town")`

**Example**:

```ron
// In npcs.ron:
(
    id: "cozy_inn_mary",
    name: "Mary the Innkeeper",
    description: "A cheerful innkeeper who runs the Cozy Inn.",
    portrait_id: "innkeeper_mary",
    is_innkeeper: true,
    is_merchant: false,
    ...
)

// In campaign.ron:
CampaignMetadata(
    ...
    starting_innkeeper: "cozy_inn_mary",
    ...
)

// In map.ron events:
(x: 5, y: 4): EnterInn(
    name: "Cozy Inn Entrance",
    description: "A welcoming inn...",
    innkeeper_id: "cozy_inn_mary",
),
```
````

**Validation**:

- Campaign validator checks that `starting_innkeeper` references an existing NPC
- Campaign validator verifies NPC has `is_innkeeper: true`
- Map validator checks that all `EnterInn` events reference valid innkeeper NPCs

````

#### 7.2 Update Architecture Documentation

**File**: `docs/reference/architecture.md`

**Update Section 4.6 (Type Aliases)**:

**Remove**:
```markdown
- `TownId = u8`: Town identifier
````

**Add**:

```markdown
- `InnkeeperId = String`: Innkeeper NPC identifier (references NpcId with is_innkeeper=true)
```

**Update Section 7.3 (Map Events)**:

- Document `EnterInn` uses `innkeeper_id: String` instead of `inn_id: u8`

**Update Section 4.3 (Character Location)**:

- Document `CharacterLocation::AtInn(InnkeeperId)` uses String type

#### 7.3 Update Implementation Documentation

**File**: `docs/explanation/implementations.md`

**Update previous entry** for "Campaign Builder SDK - Starting Inn UI Control":

**Change title to**: "Campaign Builder SDK - Starting Innkeeper UI Control"

**Update content** to reflect String type and NPC dropdown:

```markdown
## Campaign Builder SDK - Starting Innkeeper UI Control - COMPLETED

### Summary

Added UI control for the `starting_innkeeper` field in the Campaign Builder's Campaign Metadata Editor. This field is a string identifier (NPC ID) that references an innkeeper NPC where non-party premade characters are placed at when a new game begins.

### Changes Made

**Type Change**: Changed from `starting_inn: u8` (numeric) to `starting_innkeeper: String` (NPC ID)

**UI Change**: Changed from DragValue (numeric input) to ComboBox (NPC dropdown)

**Validation**: Added validation to ensure innkeeper NPC exists and has `is_innkeeper: true`

...
```

#### 7.4 Update CHANGELOG

**File**: `CHANGELOG.md`

**Add entry**:

```markdown
## [Unreleased]

### BREAKING CHANGES

- **Inn System Migration**: Changed from numeric inn IDs to innkeeper NPC IDs

  - **Type Removal**: `TownId = u8` type removed entirely
  - **Type Addition**: `InnkeeperId = String` type added
  - **CharacterLocation**: `AtInn(u8)` → `AtInn(String)`
  - **MapEvent**: `EnterInn { inn_id: u8 }` → `EnterInn { innkeeper_id: String }`
  - **CampaignConfig**: `starting_inn: u8` → `starting_innkeeper: String`
  - **Save Game Format**: Breaking change - old saves will not load
  - **Campaign Format**: Breaking change - old campaigns must update to use string innkeeper IDs
  - **Example**: `AtInn(1)` → `AtInn("tutorial_innkeeper_town")`

- **Benefits**:

  - Validation: Innkeeper IDs are validated against NPC database
  - Better UX: UI shows innkeeper names and portraits
  - Clearer authoring: Campaign authors select from NPC list, not arbitrary numbers

- **Migration**: No migration tool provided - update campaign data manually:
  - Change `inn_id: 1` to `innkeeper_id: "tutorial_innkeeper_town"` in map events
  - Change `starting_inn: 1` to `starting_innkeeper: "tutorial_innkeeper_town"` in campaign.ron
  - Verify all referenced NPCs have `is_innkeeper: true`
```

#### 7.5 Testing Requirements

**Full Test Suite**:

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# All features
cargo nextest run --all-features

# Clippy
cargo clippy --all-targets --all-features -- -D warnings

# Format check
cargo fmt --all -- --check
```

**Campaign Validation**:

```bash
# Validate tutorial campaign
cargo run --bin campaign_validator -- --campaign campaigns/tutorial
```

**Manual Testing Checklist**:

- [ ] Load tutorial campaign in game
- [ ] Start new game
- [ ] Verify starting party has 3 members (Kira, Sage, Mira)
- [ ] Verify non-party characters (Old Gareth, Whisper, Zara) are at inn
- [ ] Navigate to inn entrance on map (5, 4)
- [ ] Trigger EnterInn event
- [ ] Verify inn UI opens and shows correct characters
- [ ] Recruit character from inn to party
- [ ] Dismiss character from party to inn
- [ ] Save game
- [ ] Load game
- [ ] Verify all character locations preserved correctly
- [ ] Test Campaign Builder SDK:
  - [ ] Open tutorial campaign
  - [ ] Edit campaign metadata
  - [ ] Change starting_innkeeper using dropdown
  - [ ] Verify only innkeepers shown in dropdown
  - [ ] Save campaign
  - [ ] Reload and verify change persisted

#### 7.6 Deliverables

- [ ] Campaign content format documentation updated
- [ ] Architecture documentation updated
- [ ] Implementation documentation updated
- [ ] CHANGELOG updated with breaking changes
- [ ] All tests passing (unit + integration)
- [ ] Campaign validator passes for tutorial
- [ ] Manual testing checklist completed
- [ ] No `TownId` references in codebase
- [ ] No numeric inn IDs in campaign data

#### 7.7 Success Criteria

- All documentation is accurate and complete
- Breaking changes are clearly documented in CHANGELOG
- Tutorial campaign demonstrates new innkeeper ID system
- All tests pass without errors or warnings
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo nextest run --all-features` passes with 100% success
- Campaign loads and runs without errors
- Inn management works correctly with string IDs
- SDK UI shows innkeeper dropdown correctly

---

## Testing Strategy

### Unit Test Coverage

**Minimum coverage targets**:

- Type system: 100% (simple type aliases)
- CharacterLocation: 100% (enum serialization)
- Application logic: >90% (critical paths)
- Validation: 100% (all error cases)
- Save/load: 100% (round-trip tests)

**Key test files**:

- `src/domain/types.rs` - Type alias tests
- `src/domain/character.rs` - CharacterLocation tests
- `src/domain/world/types.rs` - MapEvent tests
- `src/application/mod.rs` - GameState tests
- `src/application/save_game.rs` - Save/load tests
- `src/domain/party_manager.rs` - Party management tests
- `src/sdk/validation.rs` - Validation tests

### Integration Test Scenarios

1. **New Game Initialization**

   - Start new game with innkeeper IDs
   - Verify starting party and inn characters
   - Verify character locations use string IDs

2. **Party Management Workflow**

   - Recruit character from inn to party
   - Dismiss character from party to inn
   - Swap party members
   - Verify locations update correctly

3. **Save/Load Round-Trip**

   - Save game with characters at various locations
   - Load game
   - Verify all character locations preserved
   - Verify string IDs match

4. **Inn Event Trigger**

   - Move to inn entrance on map
   - Trigger EnterInn event
   - Verify InnManagement mode activated
   - Verify correct innkeeper ID used

5. **Validation**
   - Valid innkeeper: passes
   - Missing innkeeper: fails with error
   - Non-innkeeper NPC: fails with error
   - Invalid ID format: fails with error

### Manual Testing Checklist

See Phase 7.5 above for complete manual testing checklist.

---

## Risk Assessment

### Eliminated Risks (due to no backwards compatibility requirement)

- ~~Migration tool complexity~~ - Not needed
- ~~Save game compatibility~~ - Not needed
- ~~Gradual rollout~~ - Can change everything at once

### Remaining Risks

**Medium Risk**:

1. **Comprehensive Code Changes**: Many files need updates

   - **Mitigation**: Phased approach with testing at each phase
   - **Impact**: High number of files to update
   - **Timeline**: 3-5 days of implementation

2. **Test Data Updates**: All tests need string IDs
   - **Mitigation**: Systematic search and replace
   - **Impact**: Many test files to update
   - **Timeline**: 1-2 days

**Low Risk**:

1. **SDK UI Implementation**: Dropdown needs NPC list access

   - **Mitigation**: Reuse existing ComboBox patterns
   - **Impact**: Minimal - straightforward UI change
   - **Timeline**: 0.5 days

2. **Documentation Updates**: Multiple docs need updates
   - **Mitigation**: Clear template and examples
   - **Impact**: Minimal - documentation is straightforward
   - **Timeline**: 0.5 days

---

## Dependencies

### External Crates

- **serde**: Serialization for String type (no change needed - already supports String)
- **ron**: RON format handles String natively (no change needed)
- **egui**: UI components for NPC dropdown (already available - ComboBox widget)

### Internal Systems

- **NPC System**: Must be fully implemented ✅ (already exists)
- **NPC Database**: Must support `get_npc()` and iteration ✅ (already exists)
- **Validation System**: Must support custom validators ✅ (already exists)
- **Campaign Loader**: Must handle new field names (Phase 4)
- **SDK**: Must provide NPC list to UI (Phase 5)

### No External Dependencies

All required systems already exist. No external APIs or new crates needed.

---

## Timeline Estimate

**Total Estimated Time**: 4-6 days

**Phase Breakdown**:

- Phase 1: Type System Foundation - 1 day

  - Update type aliases, enums, exports
  - Remove all TownId references
  - Update basic unit tests

- Phase 2: Application Logic Updates - 2 days

  - Update all application and domain logic
  - Update all test cases to use string IDs
  - Verify compilation and tests

- Phase 3: Save/Load System Updates - 0.5 days

  - Update save/load tests
  - Verify RON serialization

- Phase 4: Campaign Configuration Updates - 0.5 days

  - Update config structs
  - Add validation
  - Update tests

- Phase 5: SDK UI Updates - 1 day

  - Update edit buffer
  - Implement NPC dropdown
  - Update tests

- Phase 6: Campaign Data Migration - 0.5 days

  - Update tutorial campaign.ron
  - Update map events
  - Verify NPC data

- Phase 7: Documentation and Validation - 1 day
  - Update all documentation
  - Run full test suite
  - Manual testing
  - Final verification

**Parallelization Opportunities**:

- Documentation can be written alongside implementation
- Tests can be written before implementation (TDD approach)
- Phase 4 and 5 can partially overlap (config + UI)

**Critical Path**:
Phase 1 → Phase 2 → Phase 3 → Phase 6 → Phase 7
(Phases 4 and 5 can run in parallel with later parts of Phase 2)

---

## Conclusion

This migration replaces arbitrary numeric inn IDs with meaningful innkeeper NPC IDs, providing validation, better UX, and clearer campaign authoring. By removing the unused `TownId` type entirely and leveraging the existing NPC system, we create a cleaner, more maintainable codebase.

**Key Advantages of This Approach**:

1. **No Legacy Baggage**: Removing `TownId` entirely eliminates confusion
2. **Strong Validation**: Innkeeper IDs must reference valid NPCs with `is_innkeeper=true`
3. **Better Developer Experience**: Type system is clearer - `InnkeeperId` vs `MapId` vs `NpcId`
4. **Better User Experience**: UI shows innkeeper names and portraits, not numbers
5. **Simpler Implementation**: No migration tool complexity since backwards compatibility not required

**Breaking Changes Justified**:

- Project is early in development
- Only one campaign exists (tutorial)
- Benefits far outweigh migration cost
- Clean break is better than maintaining compatibility layers

**Recommendation**:
Execute all phases as a single feature branch to ensure consistency and avoid partial migration states. Test thoroughly after each phase but merge everything together.

**Next Steps After Plan Approval**:

1. Create feature branch: `feature/innkeeper-id-migration`
2. Execute Phase 1 (Type System)
3. Run tests, verify compilation
4. Execute Phase 2 (Application Logic)
5. Continue through phases sequentially
6. Final validation and documentation
7. Merge to main
