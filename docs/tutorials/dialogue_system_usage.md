# Dialogue System Usage Tutorial

This tutorial walks through creating interactive NPC dialogues in Antares, from dialogue data creation through gameplay testing.

## Prerequisites

- Basic understanding of Bevy ECS and Rust
- Familiarity with RON (Rusty Object Notation) data format
- Antares project setup complete and buildable
- Knowledge of NPC spawning and entity components

## Step 1: Create Dialogue Data File

Create a new dialogue file at `campaigns/tutorial/data/dialogues/merchant.ron`:

```ron
[
    DialogueTree(
        id: 100,
        name: "Merchant Greeting",
        root_node: 1,
        nodes: {
            1: DialogueNode(
                id: 1,
                text: "Welcome, traveler! Care to see my wares?",
                speaker_override: None,
                choices: [
                    DialogueChoice(
                        text: "Show me what you have",
                        target_node: Some(2),
                        conditions: [],
                        actions: [],
                        ends_dialogue: false,
                    ),
                    DialogueChoice(
                        text: "Not interested",
                        target_node: None,
                        conditions: [],
                        actions: [],
                        ends_dialogue: true,
                    ),
                ],
                conditions: [],
                actions: [],
                is_terminal: false,
            ),
            2: DialogueNode(
                id: 2,
                text: "Here are my finest goods! Armor, weapons, and potions.",
                speaker_override: None,
                choices: [
                    DialogueChoice(
                        text: "I'll take a look",
                        target_node: None,
                        conditions: [],
                        actions: [
                            OpenShop,
                        ],
                        ends_dialogue: true,
                    ),
                    DialogueChoice(
                        text: "Maybe later",
                        target_node: None,
                        conditions: [],
                        actions: [],
                        ends_dialogue: true,
                    ),
                ],
                conditions: [],
                actions: [],
                is_terminal: false,
            ),
        },
        speaker_name: Some("Merchant"),
        repeatable: true,
        associated_quest: None,
    ),
]
```

**Key Points:**

- `id: 100` - Unique dialogue tree ID (use 100+ for custom dialogues)
- `root_node: 1` - Dialogue always starts at this node ID
- `nodes: {1: ..., 2: ...}` - Map of node ID to DialogueNode
- `target_node: Some(2)` - Continue dialogue at node 2
- `target_node: None` with `ends_dialogue: true` - Dialogue ends
- `speaker_override: None` - Uses NPC's name; set to `Some("Custom Name")` to override
- `repeatable: true` - Dialogue can repeat on re-interaction

## Step 2: Spawn NPC Entity

Create a system to spawn the merchant NPC in your game initialization code:

```rust
use antares::game::components::dialogue::NpcDialogue;
use bevy::prelude::*;

fn spawn_merchant(mut commands: Commands) {
    // Spawn the merchant NPC with dialogue component
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.8, 0.6, 0.2),
                custom_size: Some(Vec2::new(32.0, 32.0)),
                ..default()
            },
            transform: Transform::from_xyz(100.0, 50.0, 1.0),
            ..default()
        },
        NpcDialogue::new(100, "Merchant"), // dialogue_id: 100 from RON file
    ));
}
```

**Key Points:**

- `NpcDialogue::new(100, "Merchant")` - Links entity to dialogue ID 100
- `Transform::from_xyz(100.0, 50.0, 1.0)` - Position in world space
- Position is where dialogue bubble will appear above the NPC
- Z-coordinate controls rendering order (higher = rendered on top)

Add this system to your plugin setup:

```rust
fn main() {
    App::new()
        // ... other plugins ...
        .add_systems(Startup, spawn_merchant)
        // ... other systems ...
        .run();
}
```

## Step 3: Create Interaction System

Create a system that handles E-key presses to initiate dialogue with nearby NPCs:

```rust
use antares::game::systems::dialogue::StartDialogue;
use bevy::prelude::*;

// Marker component for the player character
#[derive(Component)]
pub struct Player;

fn npc_interaction_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    query_player: Query<&Transform, With<Player>>,
    query_npcs: Query<(Entity, &Transform, &NpcDialogue)>,
    mut ev_start: EventWriter<StartDialogue>,
) {
    // Only process on E key press
    if !keyboard.just_pressed(KeyCode::KeyE) {
        return;
    }

    let Ok(player_transform) = query_player.get_single() else {
        return;
    };

    let player_pos = player_transform.translation;

    // Check if player is near any NPC
    for (npc_entity, npc_transform, npc_dialogue) in query_npcs.iter() {
        let distance = player_pos.distance(npc_transform.translation);

        // Interaction range: 50.0 units
        if distance < 50.0 {
            // Send StartDialogue event with speaker entity
            ev_start.send(StartDialogue {
                dialogue_id: npc_dialogue.dialogue_id,
                speaker_entity: npc_entity,
            });
            break; // Only interact with closest NPC
        }
    }
}
```

Add to your plugin:

```rust
app.add_systems(
    Update,
    npc_interaction_system.run_if(in_state(GameMode::Exploration)),
);
```

**Key Points:**

- `ButtonInput<KeyCode>::just_pressed(KeyCode::KeyE)` - Detects E key
- `distance < 50.0` - Interaction range in world units (adjust as needed)
- `StartDialogue` event includes both dialogue_id and speaker_entity
- Runs only during Exploration game mode
- Breaks after first interaction (prevents multiple simultaneous dialogues)

## Step 4: Test in Game

1. **Build and run your campaign**:

   ```bash
   cargo run --release --bin antares -- campaigns/tutorial
   ```

2. **Navigate to the merchant**:
   - Use WASD for movement
   - Move within 50.0 units of the merchant NPC

3. **Initiate dialogue**:
   - Press E to interact
   - Dialogue bubble appears above merchant
   - Typewriter animation reveals text character-by-character

4. **Interact with choices**:
   - Use ↑/↓ arrow keys to navigate between choices
   - Or press 1-9 number keys to select directly
   - Press Enter or Space to confirm selection

5. **Expected behavior**:
   - First interaction: "Welcome, traveler! Care to see my wares?"
   - Choice 1: "Show me what you have" → Goes to node 2
   - Choice 2: "Not interested" → Ends dialogue
   - If you chose option 1, next screen: "Here are my finest goods!..."
   - Can choose "I'll take a look" to open shop or "Maybe later" to end

## Advanced: Adding Quest Integration

Modify the dialogue to integrate with the quest system:

```ron
[
    DialogueTree(
        id: 101,
        name: "Quest Giver",
        root_node: 1,
        nodes: {
            1: DialogueNode(
                id: 1,
                text: "Adventurer! I need your help with a dangerous task.",
                speaker_override: None,
                choices: [
                    DialogueChoice(
                        text: "What do you need?",
                        target_node: Some(2),
                        conditions: [],
                        actions: [],
                        ends_dialogue: false,
                    ),
                    DialogueChoice(
                        text: "I'm too busy",
                        target_node: None,
                        conditions: [],
                        actions: [],
                        ends_dialogue: true,
                    ),
                ],
                conditions: [],
                actions: [],
                is_terminal: false,
            ),
            2: DialogueNode(
                id: 2,
                text: "Retrieve the ancient artifact from the temple!",
                speaker_override: None,
                choices: [
                    DialogueChoice(
                        text: "I'll do it",
                        target_node: None,
                        conditions: [],
                        actions: [
                            StartQuest(quest_id: 10),
                        ],
                        ends_dialogue: true,
                    ),
                    DialogueChoice(
                        text: "That sounds dangerous",
                        target_node: Some(3),
                        conditions: [],
                        actions: [],
                        ends_dialogue: false,
                    ),
                ],
                conditions: [],
                actions: [],
                is_terminal: false,
            ),
            3: DialogueNode(
                id: 3,
                text: "Indeed it is! But the reward is substantial...",
                speaker_override: None,
                choices: [
                    DialogueChoice(
                        text: "I'll accept the quest",
                        target_node: None,
                        conditions: [],
                        actions: [
                            StartQuest(quest_id: 10),
                        ],
                        ends_dialogue: true,
                    ),
                    DialogueChoice(
                        text: "Too risky for me",
                        target_node: None,
                        conditions: [],
                        actions: [],
                        ends_dialogue: true,
                    ),
                ],
                conditions: [],
                actions: [],
                is_terminal: false,
            ),
        },
        speaker_name: Some("Quest Master"),
        repeatable: false,
        associated_quest: Some(10),
    ),
]
```

**Key Points:**

- `DialogueAction::StartQuest(quest_id: 10)` - Starts quest when choice selected
- `associated_quest: Some(10)` - Links dialogue to specific quest
- `repeatable: false` - Quest-giving dialogue only happens once
- Multiple paths can lead to same quest (see nodes 2 and 3)
- Use conditions to show different dialogue based on quest state (future enhancement)

## Tips and Best Practices

### Dialogue IDs

- Reserve 1-99 for tutorial campaign
- Use 100+ for custom campaign dialogues
- Keep track of all dialogue IDs in spreadsheet or documentation
- Use semantically meaningful IDs (e.g., 100=merchant, 101=questmaster, 102=innkeeper)

### Node IDs

- Start at 1 for root node (always)
- Use sequential numbering (2, 3, 4...) within each tree
- Keep IDs contiguous without gaps (easier to debug)
- Document node purposes in comments if complex

### Choice Text

- Keep under 50 characters for readability on screen
- Use action-oriented language ("Show me wares" not "I want to see the items")
- Make choices feel meaningfully different (avoid "Yes/No/Maybe")
- Provide both positive and exit options

### Speaker Names

- Maintain consistency across related dialogues
- Use titles when appropriate ("Merchant", "Guard Captain", "Sage")
- Consider NPC archetype for name choice

### Testing Strategy

1. **Test Happy Path**: Complete dialogue normally
2. **Test Rejection**: Choose "not interested" options
3. **Test All Branches**: Ensure all dialogue nodes are reachable
4. **Test Interruption**: Walk away mid-dialogue (should end gracefully)
5. **Test Repetition**: Re-interact with repeatable dialogue

### Validation

Use the dialogue validation module to catch errors:

```bash
# In Rust code:
use antares::game::systems::dialogue_validation::validate_dialogue_tree;

let tree = /* your dialogue tree */;
match validate_dialogue_tree(&tree, &database) {
    Ok(_) => println!("Dialogue valid!"),
    Err(errors) => println!("Errors: {:?}", errors),
}
```

## Troubleshooting

### Problem: Bubble doesn't appear

**Symptoms**: Press E to interact, nothing happens

**Diagnostics**:

1. Check dialogue_id matches RON file: `grep "id: 100" campaigns/tutorial/data/dialogues.ron`
2. Verify NPC has NpcDialogue component:
   ```rust
   assert!(query.contains(npc_entity)); // NpcDialogue component
   ```
3. Check console for errors: `RUST_LOG=debug cargo run --bin antares`
4. Verify NPC position is within interaction range (< 50.0 units)

**Solutions**:

- Reload dialogue file (may need to rebuild)
- Check dialogue database loading in campaign initializer
- Add debug logging to interaction system:
  ```rust
  println!("NPC at {:?}, Player at {:?}, Distance: {}",
      npc_pos, player_pos, distance);
  ```

### Problem: Text doesn't typewriter animate

**Symptoms**: Text appears instantly or not at all

**Diagnostics**:

1. Verify TypewriterText component exists:
   ```rust
   query.contains(text_entity) // TypewriterText component
   ```
2. Check DIALOGUE_TYPEWRITER_SPEED constant in dialogue_visuals.rs
3. Ensure update_typewriter_text system is running:
   ```rust
   app.add_systems(Update, update_typewriter_text);
   ```

**Solutions**:

- Increase DIALOGUE_TYPEWRITER_SPEED (higher = faster reveal)
- Check that text entity has TypewriterText component
- Verify system is registered in plugin and not gated by invalid conditions

### Problem: Choices don't show

**Symptoms**: Dialogue text appears but no choice buttons visible

**Diagnostics**:

1. Check node has choices in RON:
   ```ron
   choices: [
       DialogueChoice(...),
   ]
   ```
2. Verify DialogueNode not marked is_terminal: true
3. Check console logs: `grep "choice_count" <output>`
4. Ensure choice UI systems are registered

**Solutions**:

- Add at least one DialogueChoice to the node
- Set is_terminal: false (allows showing choices)
- Register spawn_choice_ui system:
   ```rust
   app.add_systems(Update, spawn_choice_ui);
   ```
- Check that choice input system is active:
   ```rust
   app.add_systems(Update, choice_input_system.run_if(in_state(GameMode::Dialogue)));
   ```

## Next Steps

1. **Explore Real Examples**
   - Read `campaigns/tutorial/data/dialogues.ron` for complete examples
   - Find pattern for your dialogue type (quest, merchant, story, etc.)

2. **Understand Data Structures**
   - Read `docs/reference/architecture.md` Section 4 for dialogue structures
   - Review DialogueTree, DialogueNode, DialogueChoice definitions

3. **Advanced Topics**
   - Explore `src/domain/dialogue.rs` for DialogueCondition and DialogueAction types
   - Understand how conditions gate dialogue choices based on game state
   - Learn how actions trigger game events (quests, inventory changes, etc.)

4. **Create Your Campaign**
   - Copy tutorial campaign: `cp -r campaigns/tutorial campaigns/my_campaign`
   - Create your dialogues in `campaigns/my_campaign/data/dialogues/`
   - Modify NPC spawn logic for your world
   - Test and iterate

## Additional Resources

- **Dialogue System Implementation**: `docs/explanation/implementations.md` (Phase 1-7)
- **Architecture Overview**: `docs/reference/architecture.md` (Sections 3.2, 4.x, 7.1)
- **Implementation Plan**: `docs/explanation/dialogue_system_implementation_plan.md`
- **Tutorial Campaign**: `campaigns/tutorial/` (full example)

---

**Last Updated**: January 2025
**Status**: Production Ready
**Tested With**: Antares v0.17+ (Bevy 0.17)
