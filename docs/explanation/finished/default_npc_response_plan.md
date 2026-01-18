# Default NPC Response Implementation Plan

This plan documents the implementation of a visual fallback for NPCs that do not have an associated dialogue tree. Instead of a simple log message, these NPCs will display a dialogue bubble with a standard greeting.

## Objective

Enhance the player experience by providing visual feedback during interaction with all NPCs, even those without complex dialogue trees.

- **Current Behavior**: Interaction with a dialogue-less NPC prints a message to the game log: `"{NPC NAME}: Hello, traveler!"`.
- **Target Behavior**: A dialogue bubble appears above the NPC with the text `"Hello! I am {NPC NAME}."` and a single `"Goodbye"` choice to close it.

## Proposed Changes

### 1. Application Layer: `DialogueState`

Update `DialogueState` to support "Simple" dialogues that are not backed by a `DialogueTree`.

- **Location**: `src/application/dialogue.rs`
- **Change**: Ensure `DialogueState` can be initialized with `active_tree_id: None` while still having `current_text` and `current_choices` populated.

### 2. Game Layer: Dialogue Messaging

Introduce a way to trigger these simple dialogues without requiring a `DialogueId`.

- **Location**: `src/game/systems/dialogue.rs`
- **New Message**:
  ```rust
  pub struct SimpleDialogue {
      pub text: String,
      pub speaker_name: String,
      pub speaker_entity: Option<Entity>,
  }
  ```
- **New System**: `handle_simple_dialogue`
  - Listens for `SimpleDialogue` events.
  - Switches `GameMode` to `Dialogue`.
  - Initializes `DialogueState` with the provided text and a default "Goodbye" choice.

### 3. Game Layer: Event Fallback

Update the interaction event handling to use the new visual fallback.

- **Location**: `src/game/systems/events.rs`
- **Change**: In `MapEvent::NpcDialogue` handling, if `npc_def.dialogue_id` is `None`, send a `SimpleDialogue` message instead of just loging.

### 4. Logic Adjustments

- **Choice Selection**: Ensure that if a dialogue is "simple" (no tree ID), selecting any choice (like "Goodbye") simply calls `state.end()` and returns to `Exploration` mode.

## Verification Plan

### Automated Tests
- Test `DialogueState` initialization with `active_tree_id: None`.
- Test `handle_simple_dialogue` correctly transitions game mode.

### Manual Verification
1. Remove `dialogue_id` from an NPC in `campaigns/tutorial/data/npcs.ron` (e.g., the Blacksmith).
2. Interact with that NPC in-game.
3. Verify the dialogue bubble appears with the correct greeting.
4. Verify "Goodbye" terminates the interface.
