# Phase 12 Quest & Dialogue Editor - Quick Start Tutorial

**Estimated Time:** 15-20 minutes

**What You'll Learn:**
- Create your first quest
- Create your first dialogue tree
- Link them together
- Save and validate your content

## Part 1: Create a Simple Quest (5 minutes)

### Step 1: Open the Quest Editor

1. Launch Campaign Builder
2. Open your campaign
3. Click the **Quests** tab

### Step 2: Create a New Quest

Click **➕ New Quest** and fill in:

```
ID:              1
Name:            "Find the Lost Coin"
Description:     "The mayor lost an ancient coin. Find it in the forest."
Min Level:       1
Max Level:       10
Repeatable:      ☐ (unchecked)
Main Quest:      ☐ (unchecked)
```

Click **✅ Create**

### Step 3: Add a Stage

Your new quest appears in the list. Click on it to see details, then click **➕ Add Stage**:

```
Stage Name:      "Search the Forest"
Description:     "Look for the coin in the forest east of town"
Require All:     ☑ (checked)
```

Click **✅ Create**

### Step 4: Add an Objective

Click **➕ Add Objective** for the stage:

```
Type:            "Reach Location"
Map ID:          1
Position X:      50
Position Y:      60
Radius:          2
```

Click **✅ Create**

**Congratulations!** You've created your first quest with one stage and one objective.

## Part 2: Create a Simple Dialogue Tree (5 minutes)

### Step 1: Open the Dialogue Editor

Click the **Dialogues** tab

### Step 2: Create a New Dialogue

Click **➕ New Dialogue** and fill in:

```
ID:                  1
Name:                "Mayor's Request"
Speaker Name:        "Mayor"
Repeatable:          ☑ (checked)
Associated Quest:    1
```

Click **✅ Create**

The system creates Node 1 (root node) automatically.

### Step 3: Edit the Root Node

Your new dialogue has Node 1. Click **✏️** to edit it:

```
Node ID:             1
Text:                "Adventurer! My ancient coin is missing. Will you find it?"
Speaker Override:    (leave blank)
Terminal:            ☐ (unchecked)
```

Click **✅ Save**

### Step 4: Add Player Choices

Select Node 1 and click **➕ Add Choice**:

**Choice 1:**
```
Text:                "I'll find your coin!"
Target Node:         2
Ends Dialogue:       ☐ (unchecked)
```

**Choice 2:**
```
Text:                "I'm too busy"
Target Node:         3
Ends Dialogue:       ☑ (checked)
```

### Step 5: Add Response Nodes

Click **➕ Add Node**:

**Node 2 (Quest Accepted):**
```
Node ID:             2
Text:                "Excellent! Look in the forest east of town. Thank you!"
Speaker Override:    (blank)
Terminal:            ☑ (checked - dialogue ends here)
```

**Node 3 (Quest Declined):**
```
Node ID:             3
Text:                "Perhaps another time..."
Speaker Override:    (blank)
Terminal:            ☑ (checked - dialogue ends here)
```

Click **✅ Create** for each node.

**Congratulations!** You've created a branching dialogue tree with 3 nodes and 2 choices.

## Part 3: Link Quest to Dialogue (5 minutes)

### Step 1: Set Quest Giver

Go back to the **Quests** tab and select Quest 1:

Find the quest giver section and fill in:

```
NPC Name:            "Mayor"
Map ID:              1
Position X:          20
Position Y:          25
```

Click **✅ Save**

### Step 2: Verify the Link

Go to the **Dialogues** tab and select your "Mayor's Request" dialogue:

Notice it shows:
```
Associated Quest:    1
```

This connects the dialogue to the quest!

### Step 3: Validate Everything

1. **Check Quest:**
   - Name: ✓
   - Has stage: ✓
   - Has objective: ✓
   - Has quest giver: ✓

2. **Check Dialogue:**
   - Name: ✓
   - Has nodes: ✓
   - Root node exists: ✓
   - All choices target valid nodes: ✓

## Summary: What You Created

### Quest Flow
```
Quest 1: "Find the Lost Coin"
└─ Stage 1: "Search the Forest"
   └─ Objective: Reach location (50, 60) on Map 1
```

### Dialogue Flow
```
Dialogue 1: "Mayor's Request"
├─ Node 1: "Will you find my coin?"
│  ├─ Choice A: "I'll find it" → Node 2
│  └─ Choice B: "I'm too busy" → Node 3
├─ Node 2: "Thank you! Look in the forest"
└─ Node 3: "Perhaps another time"
```

### How It Works in Game
1. Player talks to Mayor on Map 1 at (20, 25)
2. Mayor says: "Will you find my coin?"
3. Player chooses "I'll find it" or "I'm too busy"
4. If accepted: Quest "Find the Lost Coin" starts
5. Player travels to (50, 60) on Map 1
6. Objective completes automatically
7. Player returns to Mayor and completes quest

## Next Steps

### Expand Your Quest
- Add more stages (find clues, defeat guard, retrieve coin)
- Use different objective types
- Add rewards (gold, items)

### Expand Your Dialogue
- Add more branches (ask for hints)
- Add more nodes for longer conversations
- Create dialogue for quest completion

### Advanced Features (Phase 13+)
- Conditions: "Do this only if player has X item"
- Actions: "Give 100 gold when this choice is made"
- Multiple dialogues: Different NPCs with related quests

## Quick Reference

### Objective Types
| Type | Use When | Parameters |
|------|----------|------------|
| Kill Monsters | Quest needs monster defeats | Monster ID, Quantity |
| Collect Items | Quest needs item gathering | Item ID, Quantity |
| Reach Location | Quest needs visiting place | Map ID, X, Y, Radius |
| Talk to NPC | Quest needs NPC interaction | NPC ID, Map ID |
| Deliver Item | Quest needs bringing something | Item ID, NPC, Quantity |
| Escort NPC | Quest needs following NPC | NPC ID, Map, Destination |
| Custom Flag | Quest needs specific condition | Flag name, Value |

### Node Tips
- Node 1 = Root (starting point)
- Each choice needs target node
- Use Terminal flag to end dialogue
- Give each node 1-3 choices

### Quest Tips
- Always add quest giver (NPC name + location)
- Min/Max level helps players find appropriate quests
- Repeatable for optional quests
- Main Quest for critical story quests

## Troubleshooting

**"Node not found" error:**
→ The target node doesn't exist. Create it first.

**Quest won't validate:**
→ Check: Name not empty? Has stage? Stage has objective?

**Dialogue won't start:**
→ Check: Root node (ID 1) exists? Dialogue saved?

**Can't find quest in dialogue:**
→ The quest ID in Associated Quest must match existing quest ID.

## Learn More

- **Quest System**: `docs/how-to/edit_quests_and_dialogues_in_campaign_builder.md`
- **Architecture**: `docs/reference/architecture.md` Section 4.3-4.4
- **Implementation**: `docs/explanation/phase12_quest_dialogue_tools_implementation.md`

---

**You're now ready to design quests and dialogues!**

Next, try:
1. Creating a quest with 2-3 stages
2. Creating dialogue with 5+ nodes
3. Linking multiple quests with prerequisites
4. Testing in-game (Phase 14)
