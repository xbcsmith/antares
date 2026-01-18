# InnKeeper Party Management Fixes Implementation Plan

## Overview

This plan fixes critical InnKeeper party management issues that currently make the game unplayable. The InnKeeper UI triggers automatically before player interaction, has no input support (keyboard or mouse), and displays already-recruited characters in the recruitable list. This plan implements dialogue-triggered party management, proper input handling, and correct party/roster filtering.

**Critical Issues to Fix:**

1. Inn party management triggers automatically on `EnterInn` event (should trigger via dialogue)
2. No keyboard/mouse support for navigation and actions
3. ESC key doesn't close party management window
4. Characters already in party appear in recruitable list
5. No way to distinguish party members from available characters
6. Game gets stuck in `InnManagement` mode with no exit

**User Decisions:**

1. **All innkeepers MUST have dialogue** - No auto-trigger fallback; require dialogue_id
2. **Full keyboard navigation required** - Arrow keys, Enter, Tab, ESC for all actions
3. **Create default innkeeper dialogue template** - Use for new campaigns under construction

## Current State Analysis

### Existing Infrastructure

#### Inn UI System (`src/game/systems/inn_ui.rs`)

- `InnUiPlugin` - Bevy plugin for egui-based party management UI
- `inn_ui_system` (L63-274) - Renders party management interface when in `GameMode::InnManagement`
- `inn_action_system` (L279-356) - Processes recruit/dismiss/swap/exit events
- Events: `InnRecruitCharacter`, `InnDismissCharacter`, `InnSwapCharacters`, `ExitInn`
- **CURRENT BEHAVIOR**: UI relies on egui mouse clicks only, no keyboard support
- **ISSUE**: Exit relies on clicking "Exit Inn" button, ESC key not handled

#### Event Handler (`src/game/systems/events.rs`)

- `handle_events` (L68-330) - Processes `MapEventTriggered` messages
- `MapEvent::EnterInn` case (L307-330) - Immediately transitions to `InnManagement` mode
- **ISSUE**: Auto-triggers party management, bypassing dialogue interaction

#### Game Mode State (`src/application/mod.rs`)

- `GameMode::InnManagement(InnManagementState)` (L51)
- `InnManagementState` struct (L67-73) - Tracks `current_inn_id`, party/roster selections
- Mode transition happens in event handler, not dialogue system

#### Dialogue System

- `DialogueAction` enum (`src/domain/dialogue.rs` L390-428) - Supports various actions
- **MISSING**: No `OpenInnManagement` or `OpenPartyManagement` action
- Innkeeper NPCs have `dialogue_id` and `is_innkeeper: true` flag
- Example: `tutorial_innkeeper_town` (dialogue_id: 4) in `campaigns/tutorial/data/npcs.ron`

#### Roster Filtering (`src/domain/character.rs`)

- `Roster::characters_at_inn(&str)` - Returns characters at specific inn
- `CharacterLocation::InParty` vs `CharacterLocation::AtInn(InnkeeperId)` - Location tracking
- **ISSUE**: `inn_ui_system` filters roster but doesn't exclude party members from display

### Identified Issues

1. **Auto-Trigger Violation**: `MapEvent::EnterInn` handler directly transitions to `InnManagement` mode without player consent
2. **Missing Dialogue Integration**: No `DialogueAction::OpenInnManagement` to trigger from innkeeper conversations
3. **Input Handling Gap**: UI system has no keyboard input handling, only egui mouse interactions
4. **ESC Key Not Mapped**: No system listens for ESC to exit `InnManagement` mode
5. **Roster Filter Bug**: Characters with `CharacterLocation::InParty` appear in "AVAILABLE AT THIS INN" list
6. **Poor UX**: No visual distinction between party members and available recruits beyond sections

## Implementation Phases

### Phase 1: Add DialogueAction for Inn Management

#### 1.1 Add OpenInnManagement Action to DialogueAction Enum

**File**: `src/domain/dialogue.rs`

**Location**: After `RecruitToInn` variant (around L423-428)

**Add**:

```rust
/// Open the inn party management interface
OpenInnManagement { innkeeper_id: String },
```

**Update `DialogueAction::description()` method** (around L432-477):

```rust
DialogueAction::OpenInnManagement { innkeeper_id } => {
    format!("Open party management at inn (keeper: {})", innkeeper_id)
}
```

#### 1.2 Implement OpenInnManagement in Dialogue Execution

**File**: `src/game/systems/dialogue.rs`

**Find**: `execute_action` function (search for "fn execute_action")

**Add case** after `RecruitToInn` handling:

```rust
DialogueAction::OpenInnManagement { innkeeper_id } => {
    // Close dialogue and transition to InnManagement mode
    use crate::application::{GameMode, InnManagementState};

    global_state.0.mode = GameMode::InnManagement(InnManagementState {
        current_inn_id: innkeeper_id.clone(),
        selected_party_slot: None,
        selected_roster_slot: None,
    });

    if let Some(ref mut log) = game_log {
        log.add("Opening party management...".to_string());
    }

    info!("Opened inn party management for innkeeper: {}", innkeeper_id);
}
```

#### 1.3 Remove Auto-Trigger from EnterInn Event Handler

**File**: `src/game/systems/events.rs`

**Location**: `handle_events` function, `MapEvent::EnterInn` case (L307-330)

**Change from**:

```rust
MapEvent::EnterInn {
    name,
    description,
    innkeeper_id,
} => {
    let msg = format!("{} - {}", name, description);
    // ... logging ...

    // Transition GameMode to InnManagement
    use crate::application::{GameMode, InnManagementState};
    global_state.0.mode = GameMode::InnManagement(InnManagementState {
        current_inn_id: innkeeper_id.clone(),
        // ...
    });
}
```

**Change to**:

```rust
MapEvent::EnterInn {
    name,
    description,
    innkeeper_id,
} => {
    let msg = format!("{} - {}", name, description);
    println!("{}", msg);
    if let Some(ref mut log) = game_log {
        log.add(msg);
    }

    // Find innkeeper NPC and trigger dialogue if available
    if let Some(npc_def) = content.db().npcs.get_npc(innkeeper_id) {
        if let Some(dialogue_id) = npc_def.dialogue_id {
            // Find NPC entity
            let speaker_entity = npc_query
                .iter()
                .find(|(_, marker, _)| marker.npc_id == *innkeeper_id)
                .map(|(entity, _, _)| entity);

            // Trigger innkeeper dialogue
            dialogue_writer.write(StartDialogue {
                dialogue_id,
                speaker_entity,
                fallback_position: Some(trigger.position),
            });

            if let Some(ref mut log) = game_log {
                log.add(format!("Speaking with {}...", npc_def.name));
            }
        } else {
            // ERROR: All innkeepers must have dialogue
            error!("Innkeeper '{}' has no dialogue_id. All innkeepers must have dialogue configured.", innkeeper_id);
            if let Some(ref mut log) = game_log {
                log.add(format!("Error: Innkeeper '{}' is not properly configured", npc_def.name));
            }
        }
    } else {
        if let Some(ref mut log) = game_log {
            log.add(format!("Error: Innkeeper '{}' not found", innkeeper_id));
        }
    }
}
```

#### 1.4 Update Tutorial Innkeeper Dialogues

**File**: `campaigns/tutorial/data/dialogues.ron`

**Find**: Dialogue ID 4 ("Innkeeper Town Square Greeting") - around L414-511

**Add choice node** to trigger party management:

```ron
2: (
    id: 2,
    text: "Of course! Let me help you manage your party.",
    speaker_override: None,
    choices: [],
    conditions: [],
    actions: [
        OpenInnManagement(
            innkeeper_id: "tutorial_innkeeper_town",
        ),
    ],
    is_terminal: true,
),
```

**Update root node (node 1)** to add choice:

```ron
1: (
    id: 1,
    text: "Welcome to the Cozy Inn! How can I help you today?",
    speaker_override: None,
    choices: [
        (
            text: "I'd like to manage my party.",
            target_node: Some(2),
            conditions: [],
            required_items: [],
            required_gold: 0,
        ),
        (
            text: "Just passing through. Goodbye.",
            target_node: None,
            conditions: [],
            required_items: [],
            required_gold: 0,
        ),
    ],
    conditions: [],
    actions: [],
    is_terminal: false,
),
```

**Repeat for Dialogue ID 9** ("Innkeeper Mountain Pass Greeting") with `innkeeper_id: "tutorial_innkeeper_town2"`

#### 1.5 Testing Requirements

**Unit Tests** (`src/game/systems/dialogue.rs`):

- `test_open_inn_management_action_transitions_mode` - Verify mode transition
- `test_open_inn_management_sets_innkeeper_id` - Verify state initialization

**Integration Tests**:

- Start game, walk to inn entrance, trigger `EnterInn` event
- Verify dialogue opens (not immediate party management)
- Select "manage party" dialogue choice
- Verify transition to `InnManagement` mode

#### 1.6 Deliverables

- [ ] `DialogueAction::OpenInnManagement` variant added
- [ ] `execute_action` handles `OpenInnManagement` action
- [ ] `EnterInn` event triggers innkeeper dialogue (not direct management)
- [ ] Tutorial innkeeper dialogues updated with party management option
- [ ] Tests pass for dialogue-triggered party management

#### 1.7 Success Criteria

- Walking over `EnterInn` tile opens innkeeper dialogue
- Selecting "manage party" choice transitions to `InnManagement` mode
- `InnManagementState.current_inn_id` set correctly
- No auto-trigger of party management UI

---

### Phase 2: Implement Keyboard and Mouse Input Support

#### 2.1 Add Keyboard Navigation State

**File**: `src/game/systems/inn_ui.rs`

**Add state resource** (after event structs, around L59):

```rust
/// Tracks keyboard navigation state for inn party management
#[derive(Resource, Default)]
pub struct InnNavigationState {
    /// Selected party slot (0-5) for keyboard navigation
    pub selected_party_index: Option<usize>,
    /// Selected roster index for keyboard navigation
    pub selected_roster_index: Option<usize>,
    /// Which section has focus: Party(true) or Roster(false)
    pub focus_on_party: bool,
}
```

**Register resource** in `InnUiPlugin::build`:

```rust
app.init_resource::<InnNavigationState>();
```

#### 2.2 Add Full Keyboard Input System

**File**: `src/game/systems/inn_ui.rs`

**Add new system** after `inn_action_system` (around L357):

```rust
/// Handles keyboard input for inn party management
fn inn_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    global_state: Res<GlobalState>,
    mut nav_state: ResMut<InnNavigationState>,
    mut recruit_events: MessageWriter<InnRecruitCharacter>,
    mut dismiss_events: MessageWriter<InnDismissCharacter>,
    mut swap_events: MessageWriter<InnSwapCharacters>,
    mut exit_events: MessageWriter<ExitInn>,
) {
    // Only process input when in InnManagement mode
    let inn_state = match &global_state.0.mode {
        GameMode::InnManagement(state) => state,
        _ => {
            // Reset navigation state when not in inn mode
            *nav_state = InnNavigationState::default();
            return;
        }
    };

    let party_count = global_state.0.party.members.len();

    // Count characters at this inn
    let roster_count = global_state.0.roster.characters.iter().enumerate()
        .filter(|(idx, _)| {
            if let Some(CharacterLocation::AtInn(inn_id)) =
                global_state.0.roster.character_locations.get(*idx) {
                inn_id == &inn_state.current_inn_id
            } else {
                false
            }
        })
        .count();

    // ESC key exits inn management
    if keyboard.just_pressed(KeyCode::Escape) {
        exit_events.write(ExitInn);
        *nav_state = InnNavigationState::default();
        return;
    }

    // Tab key switches focus between party and roster
    if keyboard.just_pressed(KeyCode::Tab) {
        nav_state.focus_on_party = !nav_state.focus_on_party;
        // Clear selections when switching focus
        if nav_state.focus_on_party {
            nav_state.selected_roster_index = None;
        } else {
            nav_state.selected_party_index = None;
        }
    }

    // Arrow key navigation
    if nav_state.focus_on_party {
        // Navigate party members
        if keyboard.just_pressed(KeyCode::ArrowRight) {
            nav_state.selected_party_index = Some(
                nav_state.selected_party_index.map_or(0, |i| (i + 1).min(party_count.saturating_sub(1)))
            );
        }
        if keyboard.just_pressed(KeyCode::ArrowLeft) {
            nav_state.selected_party_index = nav_state.selected_party_index
                .and_then(|i| i.checked_sub(1));
        }

        // Enter/Space to dismiss selected party member
        if (keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space)) {
            if let Some(party_idx) = nav_state.selected_party_index {
                if party_idx < party_count {
                    dismiss_events.write(InnDismissCharacter { party_index: party_idx });
                }
            }
        }
    } else {
        // Navigate roster characters at inn
        if keyboard.just_pressed(KeyCode::ArrowRight) {
            nav_state.selected_roster_index = Some(
                nav_state.selected_roster_index.map_or(0, |i| (i + 1).min(roster_count.saturating_sub(1)))
            );
        }
        if keyboard.just_pressed(KeyCode::ArrowLeft) {
            nav_state.selected_roster_index = nav_state.selected_roster_index
                .and_then(|i| i.checked_sub(1));
        }

        // Enter/Space to recruit selected roster character
        if (keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space)) {
            if let Some(roster_idx) = nav_state.selected_roster_index {
                if party_count < 6 {
                    recruit_events.write(InnRecruitCharacter { roster_index: roster_idx });
                }
            }
        }

        // S key to swap (if both party and roster characters selected)
        if keyboard.just_pressed(KeyCode::KeyS) {
            if let (Some(party_idx), Some(roster_idx)) =
                (nav_state.selected_party_index, nav_state.selected_roster_index) {
                swap_events.write(InnSwapCharacters {
                    party_index: party_idx,
                    roster_index: roster_idx,
                });
            }
        }
    }
}
```

**Register system** in `InnUiPlugin::build`:

```rust
impl Plugin for InnUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                inn_ui_system.run_if(resource_exists::<GlobalState>),
                inn_action_system.run_if(resource_exists::<GlobalState>),
                inn_input_system.run_if(resource_exists::<GlobalState>), // NEW
            ),
        );
    }
}
```

#### 2.2 Fix Roster Filtering to Exclude Party Members

**File**: `src/game/systems/inn_ui.rs`

**Location**: `inn_ui_system` function, "Available Characters" section (around L171-184)

**Change from**:

```rust
// Find characters at this inn
let mut inn_characters = Vec::new();
for (roster_idx, character) in global_state.0.roster.characters.iter().enumerate() {
    if let Some(CharacterLocation::AtInn(inn_id)) =
        global_state.0.roster.character_locations.get(roster_idx)
    {
        if inn_id == &current_inn_id {
            inn_characters.push((roster_idx, character));
        }
    }
}
```

**Change to**:

```rust
// Find characters at this inn (exclude those currently in party)
let mut inn_characters = Vec::new();
for (roster_idx, character) in global_state.0.roster.characters.iter().enumerate() {
    if let Some(location) = global_state.0.roster.character_locations.get(roster_idx) {
        // Only include characters AT THIS INN, not in party
        if let CharacterLocation::AtInn(inn_id) = location {
            if inn_id == &current_inn_id {
                inn_characters.push((roster_idx, character));
            }
        }
        // Characters with InParty location are excluded
    }
}
```

#### 2.3 Add Visual Feedback for ESC Key

**File**: `src/game/systems/inn_ui.rs`

**Location**: `inn_ui_system` function, bottom instructions section (around L257-271)

**Update instructions**:

```rust
// Instructions
ui.add_space(10.0);
ui.label(egui::RichText::new("Instructions:").weak());
ui.label(
    egui::RichText::new("• Click Dismiss to send party member to this inn")
        .weak()
        .small(),
);
ui.label(
    egui::RichText::new("• Click Recruit to add character to party (if room)")
        .weak()
        .small(),
);
ui.label(
    egui::RichText::new(
        "• Select party member, then click Swap on inn character to exchange",
    )
    .weak()
    .small(),
);
ui.label(
    egui::RichText::new("• Press ESC or click Exit Inn to return to exploration")
        .weak()
        .small()
        .color(egui::Color32::LIGHT_GREEN),
);
ui.label(
    egui::RichText::new("• Use TAB to switch focus, Arrow Keys to navigate, Enter/Space to select")
        .weak()
        .small()
        .color(egui::Color32::LIGHT_BLUE),
);
ui.label(
    egui::RichText::new("• Select party member + roster character, press S to swap")
        .weak()
        .small()
        .color(egui::Color32::LIGHT_BLUE),
);
```

#### 2.4 Update UI to Show Keyboard Focus

**File**: `src/game/systems/inn_ui.rs`

**Update `inn_ui_system`** to use `InnNavigationState`:

- Add parameter: `nav_state: Res<InnNavigationState>`
- Highlight party slot when `nav_state.selected_party_index == Some(idx)` and `nav_state.focus_on_party`
- Highlight roster character when `nav_state.selected_roster_index == Some(idx)` and `!nav_state.focus_on_party`
- Use distinct colors: keyboard focus = GREEN, mouse hover = YELLOW

#### 2.5 Testing Requirements

**Unit Tests** (`src/game/systems/inn_ui.rs`):

- `test_esc_key_exits_inn_management` - Verify ESC triggers ExitInn event
- `test_input_ignored_outside_inn_mode` - Verify ESC doesn't trigger in other modes
- `test_tab_switches_focus` - Verify Tab toggles between party and roster
- `test_arrow_keys_navigate_party` - Verify left/right arrows navigate party slots
- `test_arrow_keys_navigate_roster` - Verify left/right arrows navigate roster characters
- `test_enter_recruits_selected_character` - Verify Enter/Space recruits when roster focused
- `test_enter_dismisses_selected_character` - Verify Enter/Space dismisses when party focused
- `test_s_key_swaps_characters` - Verify S key swaps selected party and roster characters
- `test_roster_excludes_party_members` - Verify party members not in available list
- `test_navigation_state_resets_on_mode_change` - Verify state clears when exiting inn

**Manual Test Cases**:

- Open inn party management
- Press ESC → should return to Exploration mode
- Verify characters in party don't appear in "AVAILABLE AT THIS INN"
- Dismiss character → verify appears in available list
- Recruit character → verify removed from available list

#### 2.6 Deliverables

- [ ] `InnNavigationState` resource added
- [ ] `inn_input_system` added with full keyboard support
- [ ] System registered in `InnUiPlugin`
- [ ] Roster filtering excludes `CharacterLocation::InParty` characters
- [ ] UI updated to show keyboard focus highlights
- [ ] Instructions updated with keyboard shortcuts
- [ ] Tests pass for all input handling

#### 2.7 Success Criteria

- ESC key exits `InnManagement` mode and returns to `Exploration`
- Tab key switches focus between party and roster sections
- Arrow keys navigate within focused section
- Enter/Space recruits or dismisses based on focus
- S key swaps selected party and roster characters
- Keyboard focus visually distinct from mouse selection
- Characters in active party never appear in "AVAILABLE AT THIS INN" list
- Characters at other inns never appear in current inn's list
- UI instructions clearly list all keyboard shortcuts

---

### Phase 3: Add Default Innkeeper Dialogue Template

#### 3.1 Create Default Dialogue Template in Tutorial Campaign

**File**: `campaigns/tutorial/data/dialogues.ron`

**Add default innkeeper dialogue** (ID 999) for campaigns under construction:

```ron
(
    id: 999,
    name: "Default Innkeeper Greeting",
    root_node: 1,
    nodes: {
        1: (
            id: 1,
            text: "Welcome to my establishment! What can I do for you?",
            speaker_override: None,
            choices: [
                (
                    text: "I need to manage my party.",
                    target_node: Some(2),
                    conditions: [],
                    required_items: [],
                    required_gold: 0,
                ),
                (
                    text: "Nothing right now. Farewell.",
                    target_node: None,
                    conditions: [],
                    required_items: [],
                    required_gold: 0,
                ),
            ],
            conditions: [],
            actions: [],
            is_terminal: false,
        ),
        2: (
            id: 2,
            text: "Certainly! Let me help you organize your party.",
            speaker_override: None,
            choices: [],
            conditions: [],
            actions: [
                TriggerEvent(
                    event_name: "open_inn_party_management",
                ),
            ],
            is_terminal: true,
        ),
    },
    speaker_name: Some("Innkeeper"),
    repeatable: true,
    associated_quest: None,
),
```

#### 3.2 Validate All Innkeepers Have Dialogue in SDK

**File**: `src/sdk/validation.rs` (or campaign validation module)

**Add validation rule**:

```rust
// Validate that all NPCs with is_innkeeper=true have dialogue_id
for npc in &campaign.npcs {
    if npc.is_innkeeper && npc.dialogue_id.is_none() {
        errors.push(format!(
            "Innkeeper NPC '{}' must have dialogue_id configured",
            npc.id
        ));
    }
}
```

#### 3.3 Handle TriggerEvent for Inn Management

**File**: `src/game/systems/dialogue.rs`

**Location**: `execute_action` function, `DialogueAction::TriggerEvent` case

**Add special handling**:

```rust
DialogueAction::TriggerEvent { event_name } => {
    if event_name == "open_inn_party_management" {
        // Extract innkeeper_id from current DialogueState context
        // This requires DialogueState to track the speaker NPC ID
        if let GameMode::Dialogue(ref dialogue_state) = global_state.0.mode {
            if let Some(ref speaker_npc_id) = dialogue_state.speaker_npc_id {
                // Transition to InnManagement mode
                use crate::application::{GameMode, InnManagementState};
                global_state.0.mode = GameMode::InnManagement(InnManagementState {
                    current_inn_id: speaker_npc_id.clone(),
                    selected_party_slot: None,
                    selected_roster_slot: None,
                });

                if let Some(ref mut log) = game_log {
                    log.add("Opening party management...".to_string());
                }
            } else {
                warn!("TriggerEvent 'open_inn_party_management' called but no speaker_npc_id in DialogueState");
            }
        }
    }

    // Log generic event trigger
    info!("Dialogue triggered event: {}", event_name);
    if let Some(ref mut log) = game_log {
        log.add(format!("Event triggered: {}", event_name));
    }
}
```

#### 3.4 Track Speaker NPC ID in DialogueState

**File**: `src/application/dialogue.rs`

**Find**: `DialogueState` struct definition

**Add field**:

```rust
pub struct DialogueState {
    pub dialogue_id: DialogueId,
    pub current_node_id: NodeId,
    pub current_text: String,
    pub current_speaker: String,
    pub current_choices: Vec<String>,
    pub speaker_entity: Option<Entity>,
    pub speaker_npc_id: Option<String>, // NEW - Track which NPC is speaking
    pub recruitment_context: Option<RecruitmentContext>,
}
```

**Update constructor**:

```rust
pub fn start(
    dialogue_id: DialogueId,
    root_node: NodeId,
    speaker: String,
    speaker_entity: Option<Entity>,
    speaker_npc_id: Option<String>, // NEW parameter
) -> Self {
    Self {
        dialogue_id,
        current_node_id: root_node,
        current_text: String::new(),
        current_speaker: speaker,
        current_choices: Vec::new(),
        speaker_entity,
        speaker_npc_id, // NEW
        recruitment_context: None,
    }
}
```

**Update callers** in `src/game/systems/dialogue.rs` (search for `DialogueState::start`):

- Pass NPC ID from `NpcDialogue` and `EnterInn` event contexts

#### 3.5 Update Existing Tutorial Innkeepers

**Files**:

- `campaigns/tutorial/data/dialogues.ron` - Update dialogues 4 and 9
- `campaigns/tutorial/data/npcs.ron` - Verify all innkeepers have dialogue_id

**Ensure**:

- `tutorial_innkeeper_town` (dialogue_id: 4) includes party management option
- `tutorial_innkeeper_town2` (dialogue_id: 9) includes party management option
- Both use `OpenInnManagement` action with correct innkeeper_id

#### 3.6 Document Innkeeper Requirements

**File**: `campaigns/tutorial/README.md`

**Add section**:

```markdown
## Innkeeper Requirements

**MANDATORY**: All NPCs with `is_innkeeper: true` MUST have a `dialogue_id` configured.

- Default template: Dialogue ID 999 (use for campaigns under construction)
- Custom dialogues: Must include party management option using `OpenInnManagement { innkeeper_id }`
- Validation: SDK will reject campaigns with innkeepers lacking dialogue_id

### Example Innkeeper Dialogue

See dialogue ID 4 or 9 in `data/dialogues.ron` for reference implementation.
```

**File**: `docs/explanation/modding_guide.md` (if exists)

**Add innkeeper setup section**:

```markdown
## Creating Innkeepers

1. Define NPC in `npcs.ron` with `is_innkeeper: true`
2. Create dialogue in `dialogues.ron` with party management option
3. Add `OpenInnManagement { innkeeper_id: "your_innkeeper_id" }` action to dialogue node
4. Reference dialogue in NPC definition: `dialogue_id: Some(your_dialogue_id)`
```

#### 3.7 Testing Requirements

**Unit Tests**:

- `test_trigger_event_opens_inn_management` - Verify event triggers mode change
- `test_dialogue_state_tracks_speaker_npc_id` - Verify NPC ID storage

**Integration Tests**:

- Walk to inn with generic innkeeper (using dialogue 999)
- Select "manage party" option
- Verify party management opens with correct innkeeper ID

#### 3.8 Deliverables

- [ ] Default innkeeper dialogue template (ID 999) created
- [ ] SDK validation enforces dialogue_id for innkeepers
- [ ] `DialogueState.speaker_npc_id` field added
- [ ] `TriggerEvent("open_inn_party_management")` handler implemented
- [ ] Tutorial innkeeper dialogues 4 and 9 updated
- [ ] Documentation updated with innkeeper requirements
- [ ] Modding guide includes innkeeper setup instructions

#### 3.9 Success Criteria

- All NPCs with `is_innkeeper: true` MUST have dialogue_id (enforced by validation)
- Default dialogue template (ID 999) works for campaigns under construction
- `speaker_npc_id` correctly tracked throughout dialogue flow
- Tutorial innkeepers (ID 4, ID 9) provide party management option
- SDK rejects campaigns with innkeepers lacking dialogue

---

### Phase 4: Integration Testing and Bug Fixes

#### 4.1 End-to-End Test Scenario

**Test Case**: Complete Inn Party Management Flow

1. Start tutorial campaign
2. Walk to inn entrance at map position (5, 4) on map_1
3. Trigger `EnterInn` event
4. **Expected**: Innkeeper dialogue opens (not party management)
5. Select "I'd like to manage my party" choice
6. **Expected**: Party management UI opens
7. View active party members
8. View characters available at this inn
9. **Expected**: Party members NOT in available list
10. Press Tab → **Expected**: Focus switches to roster section (visual highlight)
11. Press Arrow Right → **Expected**: Next roster character highlighted
12. Press Enter → **Expected**: Character recruited to party (if room)
13. Press Tab → **Expected**: Focus switches back to party section
14. Press Arrow Right → **Expected**: Next party member highlighted
15. Press Enter → **Expected**: Party member dismissed to inn
16. Press ESC key → **Expected**: Return to Exploration mode
17. Walk away from inn
18. Walk back to inn entrance
19. **Expected**: Can repeat the flow (dialogue is repeatable)

#### 4.2 Edge Case Testing

**Test Cases**:

- Inn with no characters stored → "No characters available" message shown
- Party full (6 members) → Recruit button disabled
- Party empty → Dismiss functionality unavailable
- Multiple inns → Characters at other inns don't appear
- Dismiss character → Appears immediately in available list
- Recruit character → Removed immediately from available list
- Swap operation → Both characters swap locations correctly

#### 4.3 Input Validation Testing

**Test Cases**:

- ESC pressed in Exploration mode → No effect on inn state
- ESC pressed in Dialogue mode → Closes dialogue (not inn management)
- ESC pressed in InnManagement mode → Exits to Exploration
- Mouse click "Exit Inn" → Same behavior as ESC
- Multiple rapid ESC presses → Single exit, no crashes
- Arrow keys in Exploration mode → No effect on inn state
- Tab key in other modes → Doesn't affect inn navigation state
- Enter/Space keys trigger correct action based on focus
- Navigation wraps/clamps at boundaries (doesn't crash)
- Keyboard and mouse selection work independently

#### 4.4 Regression Testing

**Verify existing functionality**:

- Recruit/dismiss/swap operations still work via UI buttons
- Game log messages appear correctly
- Character location tracking updates properly
- Party size limits enforced (max 6)
- Roster locations persist across save/load (if applicable)

#### 4.5 Performance Testing

**Verify**:

- UI renders at 60fps with large rosters (20+ characters)
- No frame drops when switching between modes
- ESC key response time < 100ms
- egui UI doesn't block game logic

#### 4.6 Deliverables

- [ ] All end-to-end test scenarios pass
- [ ] Edge cases handled gracefully
- [ ] Input validation prevents invalid states
- [ ] No regressions in existing party management features
- [ ] Performance meets criteria

#### 4.7 Success Criteria

- Complete inn party management flow works from EnterInn to ESC exit
- No way to get stuck in InnManagement mode
- Character filtering is 100% accurate (no duplicates or missing characters)
- All keyboard and mouse inputs work as expected

---

## Testing Strategy

### Unit Test Coverage

**Files requiring new tests**:

- `src/domain/dialogue.rs` - DialogueAction::OpenInnManagement
- `src/game/systems/dialogue.rs` - execute_action for OpenInnManagement
- `src/game/systems/events.rs` - EnterInn dialogue trigger (not direct mode change)
- `src/game/systems/inn_ui.rs` - Full keyboard navigation, roster filtering
- `src/sdk/validation.rs` - Innkeeper dialogue_id validation

**Target coverage**: >90% for changed code paths

### Integration Test Coverage

**New integration tests**:

- `tests/inn_management_dialogue_flow_test.rs` - Full dialogue-to-management flow
- `tests/inn_keyboard_navigation_test.rs` - Full keyboard navigation (Tab, arrows, Enter, S, ESC)
- `tests/inn_roster_filtering_test.rs` - Party member exclusion
- `tests/inn_validation_test.rs` - SDK validation for innkeepers

### Manual Testing Checklist

- [ ] Walk to inn entrance → dialogue opens (not auto-trigger)
- [ ] Select party management option → UI opens
- [ ] Press Tab → focus switches between sections
- [ ] Press Arrow keys → navigates within focused section
- [ ] Press Enter → recruits/dismisses based on focus
- [ ] Press S → swaps selected characters
- [ ] Press ESC → returns to exploration
- [ ] Party members not in available list
- [ ] Dismissed characters appear in available list
- [ ] Recruited characters removed from available list
- [ ] Keyboard focus visually distinct from mouse hover
- [ ] Can exit and re-enter inn
- [ ] Multiple inns maintain separate character lists
- [ ] SDK validation rejects innkeepers without dialogue

---

## Risk Assessment

### High Risk

**Breaking dialogue system**: Adding OpenInnManagement action could conflict with existing actions

- **Mitigation**: Follow exact pattern of RecruitToParty/RecruitToInn actions
- **Rollback**: Revert dialogue.rs changes, use TriggerEvent fallback

**Roster filtering regression**: Incorrect filtering could hide characters or show duplicates

- **Mitigation**: Write comprehensive unit tests for all CharacterLocation variants
- **Rollback**: Revert to original filtering logic, add todo comment

### Medium Risk

**DialogueState API changes**: Adding speaker_npc_id could break existing dialogue flows

- **Mitigation**: Make field `Option<String>`, default to None for backward compatibility
- **Testing**: Verify all existing dialogues still work (recruitment, NPC conversations)

**Input system conflicts**: Keyboard handling could conflict with other UI systems

- **Mitigation**: Check GameMode before processing input in inn_input_system
- **Testing**: Test all keyboard inputs in all game modes (Exploration, Combat, Menu, Dialogue)
- **Risk**: Arrow keys might conflict with dialogue choice navigation

### Low Risk

**Tutorial data changes**: Modifying innkeeper dialogues is low risk

- **Mitigation**: Keep original dialogue structure, only add new nodes
- **Rollback**: RON data is version controlled, easy to revert

**Performance impact**: Adding input system unlikely to impact performance

- **Validation**: Benchmark before/after with `cargo flamegraph`

---

## Dependencies

### Internal Systems

**Must be working**:

- Dialogue system (`src/game/systems/dialogue.rs`)
- Event system (`src/game/systems/events.rs`)
- Inn UI system (`src/game/systems/inn_ui.rs`)
- Party manager (`src/domain/party_manager.rs`)
- Roster location tracking (`src/domain/character.rs`)
- Input handling (`bevy::input::ButtonInput<KeyCode>`)
- SDK validation system (`src/sdk/validation.rs`)

### External Dependencies

**No new dependencies required**

**Existing dependencies used**:

- `bevy` - Input handling, ECS systems
- `bevy_egui` - UI rendering
- `bevy_mod_message` - Event messaging

---

## Rollback Procedure

### Phase 1 Rollback

**If dialogue integration fails**:

1. Revert `src/domain/dialogue.rs` changes
2. Revert `src/game/systems/dialogue.rs` execute_action changes
3. Keep EnterInn event triggering dialogue (better UX than auto-trigger)
4. Use TriggerEvent("open_inn_party_management") as temporary solution

### Phase 2 Rollback

**If keyboard navigation breaks UI**:

1. Remove `inn_input_system` from plugin registration
2. Remove `InnNavigationState` resource
3. Keep mouse-only interaction via egui buttons
4. Keep "Exit Inn" button as only exit method
5. Revert roster filtering changes if characters disappear
6. Document full keyboard navigation as future work

### Phase 3 Rollback

**If default dialogue breaks**:

1. Remove dialogue ID 999 from campaigns
2. Revert DialogueState.speaker_npc_id changes
3. Disable SDK validation for innkeeper dialogue_id
4. Allow fallback auto-trigger for innkeepers without dialogue (Phase 1 revert needed)
5. Document manual dialogue setup requirement

---

## Post-Implementation Validation

### Acceptance Criteria Checklist

- [ ] EnterInn event opens innkeeper dialogue (NEVER auto-triggers party management)
- [ ] Innkeepers without dialogue_id rejected by SDK validation
- [ ] Dialogue choice triggers party management UI
- [ ] ESC key exits party management and returns to Exploration
- [ ] Tab key switches focus between party and roster sections
- [ ] Arrow keys navigate within focused section
- [ ] Enter/Space recruits or dismisses based on focus
- [ ] S key swaps selected characters
- [ ] Keyboard focus visually highlighted (distinct from mouse)
- [ ] Characters in active party excluded from "AVAILABLE" list
- [ ] Characters at other inns excluded from current inn's list
- [ ] Recruit/dismiss/swap operations work correctly
- [ ] Game log messages accurate and helpful
- [ ] No way to get stuck in InnManagement mode
- [ ] UI instructions list all keyboard shortcuts
- [ ] Default innkeeper dialogue (ID 999) works for new campaigns
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Manual test checklist complete

### Documentation Updates

**Files to update**:

- [ ] `docs/explanation/implementations.md` - Add inn management fixes summary
- [ ] `campaigns/tutorial/README.md` - Document innkeeper dialogue pattern
- [ ] Code comments in modified files

### Quality Gates

**Before merging**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**All must pass with zero warnings.**

---

## Timeline Estimate

**Phase 1**: Add DialogueAction - 5 hours

- Implement action variant: 1 hour
- Update execute_action: 1 hour
- Modify EnterInn handler (no fallback): 1.5 hours
- Update tutorial dialogues: 1 hour
- Add SDK validation: 0.5 hours

**Phase 2**: Full Keyboard Navigation - 6 hours

- Add InnNavigationState resource: 0.5 hours
- Implement inn_input_system with full navigation: 2 hours
- Fix roster filtering: 1 hour
- Update UI with keyboard focus highlights: 1.5 hours
- Update UI instructions: 0.5 hours
- Write comprehensive tests: 1.5 hours

**Phase 3**: Default Dialogue and Documentation - 3 hours

- Create default template (ID 999): 0.5 hours
- Add DialogueState.speaker_npc_id: 1 hour
- Handle TriggerEvent: 0.5 hours
- Update tutorial innkeepers 4 and 9: 0.5 hours
- Write documentation: 0.5 hours

**Phase 4**: Integration Testing - 4 hours

- End-to-end tests with keyboard: 1.5 hours
- Edge case testing: 1 hour
- Keyboard navigation edge cases: 1 hour
- Manual testing: 0.5 hours

**Total estimated effort**: 18 hours

---

## Success Metrics

**Functionality**:

- 100% of critical issues resolved
- 0% chance of getting stuck in InnManagement mode
- 100% accurate character filtering (no party members in available list)
- 100% of innkeepers have dialogue (enforced by validation)

**Code Quality**:

- > 90% test coverage for changed code
- 0 clippy warnings
- 0 compiler warnings
- SDK validation prevents invalid innkeeper configurations

**User Experience**:

- Full keyboard navigation (Tab, arrows, Enter, S, ESC) works flawlessly
- Mouse interaction still works (keyboard adds option, doesn't replace)
- Keyboard focus clearly visible and distinct from mouse hover
- Dialogue flow feels natural and expected (never auto-triggers)
- UI instructions clear and list all keyboard shortcuts
- No confusing or duplicate character listings
- Default innkeeper dialogue works for campaigns under construction

---

## References

**Architecture Documents**:

- `docs/reference/architecture.md` - Section 4 (Game State Management)
- `docs/explanation/finished/party_management_implementation_plan.md` - Original party management design
- `docs/explanation/finished/innkeeper_id_migration_plan.md` - Innkeeper ID system

**Related Code**:

- `src/application/mod.rs` - GameMode and InnManagementState
- `src/domain/dialogue.rs` - DialogueAction enum
- `src/domain/party_manager.rs` - Party operations
- `src/game/systems/inn_ui.rs` - UI rendering and event handling
- `src/game/systems/events.rs` - Event processing
- `campaigns/tutorial/data/dialogues.ron` - Innkeeper dialogue examples

**Campaign Data**:

- `campaigns/tutorial/data/npcs.ron` - Innkeeper definitions (is_innkeeper flag)
- `campaigns/tutorial/data/maps/map_1.ron` - EnterInn event examples
