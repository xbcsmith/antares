# How-To: Using Quest and Dialogue Editor Tools

This guide shows how to use the Antares SDK quest and dialogue editor modules to create and validate quests and dialogues for your campaign.

---

## Prerequisites

```rust
use antares::sdk::database::ContentDatabase;
use antares::sdk::quest_editor::*;
use antares::sdk::dialogue_editor::*;
use antares::domain::quest::{Quest, QuestStage, QuestObjective, QuestReward};
use antares::domain::dialogue::{DialogueTree, DialogueNode, DialogueChoice, DialogueAction, DialogueCondition};
use antares::domain::types::Position;
```

---

## Part 1: Creating and Validating Quests

### Step 1: Load the Content Database

Before creating quests, load your campaign's content database:

```rust
// Load campaign content (classes, races, items, monsters, maps, etc.)
let db = ContentDatabase::load_campaign("campaigns/my_campaign")?;

// Or load core game content
let db = ContentDatabase::load_core("data")?;
```

### Step 2: Create a Basic Quest

```rust
// Create a new quest
let mut quest = Quest::new(1, "Goblin Trouble", "Clear the goblin camp");

// Set optional properties
quest.min_level = Some(3);
quest.max_level = Some(6);
quest.is_main_quest = false;
quest.repeatable = false;
quest.quest_giver_npc = Some(5);
```

### Step 3: Add Quest Stages

Quests are organized into sequential stages:

```rust
// Stage 1: Travel to the camp
let mut stage1 = QuestStage::new(1, "Find the Goblin Camp");
stage1.description = "Travel to the forest and locate the goblin camp.".to_string();
stage1.add_objective(QuestObjective::ReachLocation {
    map_id: 10,
    position: Position::new(25, 30),
    radius: 5, // Triggers when within 5 tiles
});

// Stage 2: Defeat the goblins
let mut stage2 = QuestStage::new(2, "Clear the Camp");
stage2.add_objective(QuestObjective::KillMonsters {
    monster_id: 3, // Goblin ID
    quantity: 10,
});

// Stage 3: Return to quest giver
let mut stage3 = QuestStage::new(3, "Report Back");
stage3.add_objective(QuestObjective::TalkToNpc {
    npc_id: 5,
    map_id: 1, // Town map
});

quest.add_stage(stage1);
quest.add_stage(stage2);
quest.add_stage(stage3);
```

### Step 4: Add Rewards

```rust
quest.add_reward(QuestReward::Experience(500));
quest.add_reward(QuestReward::Gold(100));
quest.add_reward(QuestReward::Items(vec![(42, 1)])); // Item ID 42, quantity 1
```

### Step 5: Validate the Quest

```rust
let errors = validate_quest(&quest, &db);

if errors.is_empty() {
    println!("Quest is valid!");
} else {
    eprintln!("Quest validation errors:");
    for error in errors {
        eprintln!("  - {}", error);
    }
}
```

### Step 6: Browse Available Content

Use browser functions to populate UI dropdowns or get content information:

```rust
// List all monsters (for KillMonsters objectives)
let monsters = browse_monsters(&db);
for (id, name) in monsters {
    println!("Monster {}: {}", id, name);
}

// List all items (for CollectItems or DeliverItem objectives)
let items = browse_items(&db);

// List all maps (for ReachLocation objectives)
let maps = browse_maps(&db);

// List existing quests (for prerequisites)
let quests = browse_quests(&db);
```

### Step 7: Use Smart Suggestions

Search for content by partial name match:

```rust
// Find swords
let swords = suggest_item_ids(&db, "sword");
for (id, name) in swords {
    println!("Found: {} (ID: {})", name, id);
}

// Find dragons
let dragons = suggest_monster_ids(&db, "dragon");

// Find dungeon maps
let dungeons = suggest_map_ids(&db, "dungeon");
```

### Step 8: Validate IDs Before Use

Check if IDs are valid before creating objectives:

```rust
let monster_id = 99;
if is_valid_monster_id(&db, &monster_id) {
    // Safe to use in objective
    stage.add_objective(QuestObjective::KillMonsters {
        monster_id,
        quantity: 1,
    });
} else {
    eprintln!("Invalid monster ID: {}", monster_id);
}
```

### Step 9: Analyze Quest Dependencies

Get the chain of prerequisite quests:

```rust
let quest_id = 5;
match get_quest_dependencies(quest_id, &db) {
    Ok(deps) => {
        println!("Quest {} requires completing: {:?}", quest_id, deps);
    }
    Err(e) => {
        eprintln!("Circular dependency detected: {}", e);
    }
}
```

### Step 10: Generate Quest Summary

Create a human-readable summary:

```rust
let summary = generate_quest_summary(&quest);
println!("{}", summary);
```

Output:
```
Quest 1: Goblin Trouble
Description: Clear the goblin camp
Type: Side Quest
Min Level: 3
Max Level: 6
Stages: 3
Rewards: 3
```

---

## Part 2: Creating and Validating Dialogues

### Step 1: Create a Dialogue Tree

```rust
let mut dialogue = DialogueTree::new(1, "Merchant Conversation", 1);
dialogue.speaker_name = Some("Merchant".to_string());
dialogue.repeatable = true;
```

### Step 2: Add Dialogue Nodes

Create the conversation structure:

```rust
// Node 1: Greeting (root node)
let mut node1 = DialogueNode::new(1, "Welcome to my shop! What can I do for you?");
node1.add_choice(DialogueChoice::new("What do you sell?", Some(2)));
node1.add_choice(DialogueChoice::new("Do you have any work?", Some(3)));
node1.add_choice(DialogueChoice::new("Goodbye", None)); // None = end dialogue

// Node 2: Shop info
let mut node2 = DialogueNode::new(2, "I sell weapons, armor, and potions.");
node2.add_choice(DialogueChoice::new("Thanks", Some(1))); // Loop back to greeting

// Node 3: Quest offer
let mut node3 = DialogueNode::new(3, "Actually, I need someone to retrieve stolen goods.");
let mut accept = DialogueChoice::new("I'll help you", Some(4));
accept.add_action(DialogueAction::StartQuest { quest_id: 1 });
node3.add_choice(accept);
node3.add_choice(DialogueChoice::new("Maybe later", Some(1)));

// Node 4: Quest accepted
let mut node4 = DialogueNode::new(4, "Thank you! The thieves fled to the old cave.");
node4.is_terminal = true; // Mark as ending node

dialogue.add_node(node1);
dialogue.add_node(node2);
dialogue.add_node(node3);
dialogue.add_node(node4);
```

### Step 3: Add Conditional Dialogue

Show different options based on game state:

```rust
let mut node = DialogueNode::new(5, "Have you completed my task?");

// Only show if player has the quest
node.add_condition(DialogueCondition::HasQuest { quest_id: 1 });

// Choice only available if quest is complete
let mut complete_choice = DialogueChoice::new("Yes, here are your goods", Some(6));
complete_choice.add_condition(DialogueCondition::QuestStage {
    quest_id: 1,
    stage_number: 3, // Final stage
});
complete_choice.add_action(DialogueAction::CompleteQuestStage {
    quest_id: 1,
    stage_number: 3,
});
complete_choice.add_action(DialogueAction::GiveGold { amount: 100 });

node.add_choice(complete_choice);
node.add_choice(DialogueChoice::new("Not yet", Some(1)));
```

### Step 4: Use Complex Conditions

Combine conditions with logical operators:

```rust
// Node only shown if level >= 5 AND has quest 1 OR completed quest 2
node.add_condition(DialogueCondition::And(vec![
    DialogueCondition::MinLevel { level: 5 },
    DialogueCondition::Or(vec![
        DialogueCondition::HasQuest { quest_id: 1 },
        DialogueCondition::CompletedQuest { quest_id: 2 },
    ]),
]));

// Choice only available if NOT banned
let mut choice = DialogueChoice::new("Enter the guild", Some(10));
choice.add_condition(DialogueCondition::Not(Box::new(
    DialogueCondition::FlagSet {
        flag_name: "guild_banned".to_string(),
        value: true,
    }
)));
```

### Step 5: Add Actions to Nodes and Choices

Trigger game events when nodes are shown or choices are selected:

```rust
// Give items when node is displayed
node.add_action(DialogueAction::GiveItems {
    items: vec![(15, 1)], // Item 15, quantity 1
});

// Take gold when choice is selected
choice.add_action(DialogueAction::TakeGold { amount: 50 });

// Set a flag
choice.add_action(DialogueAction::SetFlag {
    flag_name: "merchant_met".to_string(),
    value: true,
});

// Change reputation
choice.add_action(DialogueAction::ChangeReputation {
    faction: "Merchants Guild".to_string(),
    change: 10,
});
```

### Step 6: Validate the Dialogue Tree

```rust
let errors = validate_dialogue(&dialogue, &db);

if errors.is_empty() {
    println!("Dialogue is valid!");
} else {
    eprintln!("Dialogue validation errors:");
    for error in errors {
        eprintln!("  - {}", error);
    }
}
```

### Step 7: Analyze Dialogue Structure

Get statistics about the dialogue tree:

```rust
let stats = analyze_dialogue(&dialogue);

println!("Dialogue Statistics:");
println!("  Nodes: {}", stats.node_count);
println!("  Choices: {}", stats.choice_count);
println!("  Terminal Nodes: {}", stats.terminal_node_count);
println!("  Max Depth: {}", stats.max_depth);
println!("  Orphaned Nodes: {}", stats.orphaned_node_count);
println!("  Conditional Nodes: {}", stats.conditional_node_count);
println!("  Action Nodes: {}", stats.action_node_count);
```

### Step 8: Generate Dialogue Summary

```rust
let summary = generate_dialogue_summary(&dialogue);
println!("{}", summary);
```

Output:
```
Dialogue 1: Merchant Conversation
Speaker: Merchant
Root Node: 1
Repeatable: Yes
Total Nodes: 4
Total Choices: 6
Terminal Nodes: 1
Max Depth: 2
```

---

## Part 3: Common Patterns

### Pattern 1: Multi-Stage Quest with Items

```rust
let mut quest = Quest::new(10, "The Missing Heirloom", "Find the family heirloom");

// Stage 1: Get quest item
let mut stage1 = QuestStage::new(1, "Search the Ruins");
stage1.add_objective(QuestObjective::CollectItems {
    item_id: 77, // Heirloom item
    quantity: 1,
});

// Stage 2: Return item
let mut stage2 = QuestStage::new(2, "Return to Owner");
stage2.add_objective(QuestObjective::DeliverItem {
    item_id: 77,
    npc_id: 10,
    quantity: 1,
});

quest.add_stage(stage1);
quest.add_stage(stage2);
quest.add_reward(QuestReward::Gold(200));
```

### Pattern 2: Quest with Prerequisites

```rust
let mut quest = Quest::new(20, "Advanced Training", "Complete advanced training");
quest.min_level = Some(10);
quest.required_quests = vec![5, 6, 7]; // Must complete quests 5, 6, 7 first

// Validate no circular dependencies
let errors = validate_quest(&quest, &db);
```

### Pattern 3: Quest Chain

```rust
let mut quest1 = Quest::new(1, "Prove Yourself", "...");
// ... setup quest1 ...

let mut quest2 = Quest::new(2, "Earn Trust", "...");
quest2.required_quests = vec![1]; // Requires quest 1

let mut quest3 = Quest::new(3, "Final Test", "...");
quest3.required_quests = vec![2]; // Requires quest 2
quest3.add_reward(QuestReward::UnlockQuest(4)); // Unlocks quest 4 on completion
```

### Pattern 4: Merchant Dialogue with Buy/Sell

```rust
let mut dialogue = DialogueTree::new(100, "Weapon Merchant", 1);

let mut greeting = DialogueNode::new(1, "Looking to buy or sell?");
greeting.add_choice(DialogueChoice::new("Buy", Some(2)));
greeting.add_choice(DialogueChoice::new("Sell", Some(3)));
greeting.add_choice(DialogueChoice::new("Leave", None));

// Buy node
let mut buy_node = DialogueNode::new(2, "I have fine weapons for sale.");
// In a real implementation, this would trigger shop UI
buy_node.add_choice(DialogueChoice::new("Back", Some(1)));

// Sell node - only if player has items
let mut sell_node = DialogueNode::new(3, "What do you want to sell?");
sell_node.add_condition(DialogueCondition::HasItem {
    item_id: 1, // Any item
    quantity: 1,
});
sell_node.add_choice(DialogueChoice::new("Back", Some(1)));

dialogue.add_node(greeting);
dialogue.add_node(buy_node);
dialogue.add_node(sell_node);
```

### Pattern 5: Quest Turn-In Dialogue

```rust
let mut dialogue = DialogueTree::new(200, "Quest Turn-In", 1);

// Greeting - different based on quest status
let mut greeting = DialogueNode::new(1, "Have you completed my task?");
greeting.add_condition(DialogueCondition::HasQuest { quest_id: 10 });

// Not complete yet
let mut not_done = DialogueChoice::new("Not yet", Some(2));
not_done.add_condition(DialogueCondition::Not(Box::new(
    DialogueCondition::QuestStage { quest_id: 10, stage_number: 3 }
)));
greeting.add_choice(not_done);

// Complete - give rewards
let mut complete = DialogueChoice::new("Yes, here it is", Some(3));
complete.add_condition(DialogueCondition::QuestStage {
    quest_id: 10,
    stage_number: 3,
});
complete.add_action(DialogueAction::CompleteQuestStage {
    quest_id: 10,
    stage_number: 3,
});
complete.add_action(DialogueAction::GiveGold { amount: 500 });
complete.add_action(DialogueAction::GrantExperience { amount: 1000 });
greeting.add_choice(complete);

dialogue.add_node(greeting);
```

---

## Part 4: Best Practices

### Quest Design

1. **Sequential Stages**: Always number stages 1, 2, 3, ... with no gaps
2. **Validate Early**: Run validation after each stage is added
3. **Clear Objectives**: Use descriptive names for stages and objectives
4. **Test Prerequisites**: Use `get_quest_dependencies()` to verify quest chains
5. **Balance Rewards**: Match rewards to quest difficulty and level range

### Dialogue Design

1. **Mark Terminal Nodes**: Always set `is_terminal = true` for ending nodes
2. **Avoid Orphans**: Use `analyze_dialogue()` to check for unreachable nodes
3. **Test Conditions**: Verify all condition combinations work as expected
4. **Provide Exits**: Always give players a way to end or exit dialogue
5. **Loops Are OK**: Repeatable dialogues can loop back to greeting

### Validation Workflow

```rust
// Recommended validation workflow
fn validate_content(db: &ContentDatabase) -> Result<(), String> {
    // 1. Validate quests
    for quest_id in db.quests.all_quests() {
        if let Some(quest) = db.quests.get_quest(quest_id) {
            let errors = validate_quest(quest, db);
            if !errors.is_empty() {
                return Err(format!("Quest {} invalid: {:?}", quest_id, errors));
            }
        }
    }

    // 2. Validate dialogues
    for dialogue_id in db.dialogues.all_dialogues() {
        if let Some(dialogue) = db.dialogues.get_dialogue(dialogue_id) {
            let errors = validate_dialogue(dialogue, db);
            if !errors.is_empty() {
                return Err(format!("Dialogue {} invalid: {:?}", dialogue_id, errors));
            }
        }
    }

    Ok(())
}
```

---

## Part 5: Troubleshooting

### Common Quest Errors

**Error**: `QuestValidationError::NoStages`
- **Fix**: Add at least one stage with `quest.add_stage()`

**Error**: `QuestValidationError::InvalidMonsterId`
- **Fix**: Check monster ID exists with `is_valid_monster_id(&db, &id)`
- Browse available monsters with `browse_monsters(&db)`

**Error**: `QuestValidationError::NonSequentialStages`
- **Fix**: Ensure stage numbers are 1, 2, 3, ... with no gaps

**Error**: `QuestValidationError::CircularDependency`
- **Fix**: Remove self-references in `required_quests`
- Check full dependency chain with `get_quest_dependencies()`

### Common Dialogue Errors

**Error**: `DialogueValidationError::RootNodeMissing`
- **Fix**: Ensure node with ID matching `dialogue.root_node` exists

**Error**: `DialogueValidationError::InvalidChoiceTarget`
- **Fix**: Verify target node exists before creating choice

**Error**: `DialogueValidationError::OrphanedNode`
- **Fix**: Connect orphaned nodes to the main tree
- Use `get_reachable_nodes()` to find disconnected nodes

**Error**: `DialogueValidationError::NonTerminalNodeWithoutChoices`
- **Fix**: Either add choices or set `node.is_terminal = true`

---

## Part 6: Integration with Game

### Loading Quests and Dialogues

Your game should load quests and dialogues from the campaign database:

```rust
// In game initialization
let db = ContentDatabase::load_campaign("campaigns/current")?;

// Access quests
if let Some(quest) = db.quests.get_quest(quest_id) {
    game_state.start_quest(quest);
}

// Access dialogues
if let Some(dialogue) = db.dialogues.get_dialogue(dialogue_id) {
    game_state.start_dialogue(dialogue);
}
```

### Tracking Quest Progress

Use `QuestProgress` to track player progress:

```rust
use antares::domain::quest::QuestProgress;

let mut progress = QuestProgress::new(quest_id);
progress.current_stage = 1;

// Update objective progress
progress.update_objective(0, 5); // Objective 0, progress = 5

// Advance to next stage
progress.advance_stage();
assert_eq!(progress.current_stage, 2);

// Complete quest
progress.complete();
progress.turn_in();
```

---

## Additional Resources

- **Architecture**: `docs/explanation/sdk_and_campaign_architecture.md` - Quest and dialogue system design
- **API Reference**: Run `cargo doc --open` and see `antares::sdk::quest_editor`, `antares::sdk::dialogue_editor`
- **Examples**: See test modules in `src/sdk/quest_editor.rs` and `src/sdk/dialogue_editor.rs`
