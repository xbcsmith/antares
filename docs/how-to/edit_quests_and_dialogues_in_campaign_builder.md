# How to Edit Quests and Dialogues in Campaign Builder

This guide walks you through creating and managing quests and dialogue trees in the Antares Campaign Builder.

## Quest Editor

### Creating a New Quest

1. **Open Campaign Builder** and navigate to the **Quests** tab
2. Click **➕ New Quest** button
3. Fill in the quest form:
   - **ID**: Unique quest identifier (e.g., 1, 5, 100)
   - **Name**: Quest title (e.g., "The Lost Amulet")
   - **Description**: Long description of the quest (e.g., "An ancient amulet has disappeared from the temple...")
   - **Min Level**: Minimum character level to accept quest (default: 1)
   - **Max Level**: Maximum recommended level (default: 30)
   - **Repeatable**: Check if players can repeat this quest
   - **Main Quest**: Check if this is a critical story quest
4. Click **✅ Create**

### Adding Stages to a Quest

Stages organize quest objectives in sequence. Players complete them in order.

1. **Select the quest** from the quest list
2. Click **➕ Add Stage** button
3. Fill in stage details:
   - **Stage Name**: Name of this objective phase (e.g., "Find the Treasure Map")
   - **Description**: What the player needs to do
   - **Require All Objectives**: Check if ALL objectives must be completed, uncheck if ANY objective completes the stage
4. Click **✅ Create**

### Adding Objectives to a Stage

Each stage needs one or more objectives that guide players toward completion.

1. **Select the stage** you want to add an objective to
2. Click **➕ Add Objective**
3. Choose objective type from dropdown:

   **Kill Monsters**
   - Select monster ID from database
   - Set quantity (how many to kill)
   - Example: Kill 5 Goblins

   **Collect Items**
   - Select item ID from database
   - Set quantity (how many to collect)
   - Example: Collect 3 Ancient Coins

   **Reach Location**
   - Enter map ID where location is
   - Set X and Y coordinates
   - Set search radius (how close player needs to be)
   - Example: Go to (25, 30) on Map 5

   **Talk to NPC**
   - Enter NPC ID or name
   - Select map where NPC is located
   - Example: Talk to "Innkeeper" on Map 2

   **Deliver Item**
   - Select item ID to deliver
   - Enter NPC ID who receives it
   - Set quantity
   - Example: Deliver 1 Ancient Scroll to "Sage"

   **Escort NPC**
   - Enter NPC ID to escort
   - Select destination map
   - Set destination X, Y coordinates
   - Example: Escort "Princess" to (50, 40) on Map 8

   **Custom Flag**
   - Enter flag name (e.g., "dragon_defeated")
   - Set required value (true/false)
   - Example: Custom flag "forge_repaired" must be true

4. Fill in objective-specific fields
5. Click **✅ Create**

### Setting Quest Giver

Every quest should have someone to give it out:

1. **Select the quest**
2. In quest details, fill in:
   - **NPC Name**: Who gives the quest (e.g., "Village Elder")
   - **Map ID**: Which map the NPC is on
   - **Position X, Y**: Where on that map to find them
3. Save the quest

### Editing a Quest

1. **Select the quest** from the list
2. Click **✏️ Edit** button
3. Modify any fields
4. Click **✅ Save**

To edit stages and objectives, use the **✏️** button next to each item in the list.

### Deleting a Quest

1. **Select the quest** from the list
2. Click **🗑️** button next to the quest name
3. Quest is removed immediately

### Searching Quests

Use the search box at the top of the quest list to filter by:
- Quest name (case-insensitive)
- Quest ID
- Any part of the name

Example: Type "dragon" to find all dragon-related quests

### Quest Validation

The quest editor automatically checks:
- ✅ Quest has a name
- ✅ Quest has at least one stage
- ✅ Each stage has at least one objective
- ✅ All prerequisite quests exist (if specified)

Fix any errors shown in the validation panel before saving.

## Dialogue Editor

### Creating a New Dialogue Tree

1. **Open Campaign Builder** and navigate to the **Dialogues** tab
2. Click **➕ New Dialogue** button
3. Fill in dialogue details:
   - **ID**: Unique dialogue identifier (e.g., 1, 10, 50)
   - **Name**: Dialogue title (e.g., "Merchant Trade")
   - **Speaker Name**: Default NPC name for this dialogue (e.g., "Merchant")
   - **Repeatable**: Check if dialogue can be triggered multiple times
   - **Associated Quest**: (Optional) Link this dialogue to a quest ID
4. Click **✅ Create**

The system automatically creates a root node (Node 1) to start the dialogue.

### Adding Nodes to Dialogue

Nodes are individual lines of dialogue. Each node can have multiple player responses that branch to other nodes.

1. **Select the dialogue tree**
2. Click **➕ Add Node**
3. Fill in node details:
   - **Node ID**: Unique identifier within this dialogue (e.g., 2, 3, 10)
   - **Text**: What the NPC says
   - **Speaker Override**: (Optional) Different speaker for this node
   - **Terminal Node**: Check if dialogue ends here
4. Click **✅ Create**

### Adding Choices to a Node

Choices are player responses that lead to other nodes.

1. **Select a node** from the node list
2. Click **➕ Add Choice**
3. Fill in:
   - **Choice Text**: What player says (e.g., "I'll help you")
   - **Target Node**: Which node to go to next
   - **Ends Dialogue**: Check if this choice ends the conversation
4. Click **✅ Create**

The player will see all choices for a node and can pick one.

### Building Dialogue Branches

Create branching conversations by having nodes with multiple choices:

```
Node 1: "How can I help you?"
├─ Choice A: "I want to buy something" → Node 2 (Merchant shop)
├─ Choice B: "Tell me about this town" → Node 3 (Town info)
└─ Choice C: "Never mind" → (Dialogue ends)

Node 2: "What do you want to buy?"
├─ Choice A: "Show me weapons" → Node 4
└─ Choice B: "Never mind" → Node 1

Node 3: "This is our fair town..."
└─ Choice A: "Go back" → Node 1
```

### Editing Dialogue

1. **Select the dialogue** from the list
2. Click **✏️ Edit** to modify tree metadata
3. To edit nodes, select the node and click **✏️**
4. To edit choices, select the node and click **✏️** on the choice
5. Click **✅ Save**

### Deleting Dialogue Elements

1. **Select the dialogue** to delete the entire tree
2. Click **🗑️** button
3. To delete nodes: Select node and click **🗑️**
4. To delete choices: Select node, then click **🗑️** on the choice

### Dialogue Validation

The editor checks:
- ✅ Dialogue has a name
- ✅ Dialogue has at least one node
- ✅ Root node (Node 1) exists
- ✅ All choices point to valid nodes
- ✅ Associated quest exists (if specified)

### Searching Dialogues

Use the search box to filter by:
- Dialogue name (case-insensitive)
- Dialogue ID
- Speaker name

Example: Type "merchant" to find all merchant dialogues

## Quest-Dialogue Integration

### Linking Quest to Dialogue

To have an NPC give out a quest through dialogue:

1. **Create a quest** with the NPC as quest giver
   - Set Quest Giver NPC name
   - Set Quest Giver Map and Position

2. **Create a dialogue** for that NPC
   - Set Speaker Name to match NPC name
   - Optionally set Associated Quest

3. **Add a choice** in the dialogue that starts the quest
   - Use "Start Quest" action (Phase 13 feature)
   - Target the quest ID you created

### Common Patterns

**Simple Quest Giver Dialogue:**
```
Node 1: "I need your help!"
└─ Choice: "I'll do it" → Node 2 (dialogue ends, quest starts)
```

**Merchant with Multiple Options:**
```
Node 1: "Welcome! What can I do for you?"
├─ Choice A: "Buy" → Node 2 (shop menu)
├─ Choice B: "Sell" → Node 3 (sell menu)
└─ Choice C: "Goodbye" → (dialogue ends)
```

**Conditional Dialogue (Phase 13):**
```
Node 1: "Have you defeated the dragon?"
├─ If quest completed: "Well done!" → Node 2
└─ If not completed: "Come back when you're done" → (ends)
```

## File Organization

Your campaign directory will contain:

```
my_campaign/
├── campaign.ron           # Campaign metadata
├── quests.ron             # All quests
├── dialogues.ron          # All dialogue trees
├── items.ron              # Items (from Phase 10)
├── spells.ron             # Spells
├── monsters.ron           # Monsters
└── maps/                   # Map files (from Phase 11)
```

## Tips & Best Practices

### Quest Design

1. **Clear Objectives**: Each stage should have obvious goals
2. **Progression**: Earlier stages easier than later ones
3. **Rewards**: Match rewards to difficulty and effort
4. **NPCs**: Always assign a quest giver location
5. **Chains**: Use prerequisite quests to create quest chains
6. **Variety**: Mix objective types (not all "kill X")

### Dialogue Design

1. **Natural Speech**: Write how NPCs actually talk
2. **Branching**: Give players meaningful choices
3. **Length**: Keep individual lines reasonably short
4. **Navigation**: Always provide way back to main menu
5. **Clarity**: Make target of each choice obvious
6. **Personality**: Give each NPC distinct speech patterns

### Validation

1. **Test Chains**: Verify quest prerequisites exist
2. **Check Targets**: Ensure dialogue choices lead to valid nodes
3. **Verify NPCs**: Confirm quest givers are on correct maps
4. **Consistency**: Use matching speaker names throughout

## Common Issues & Solutions

### "Invalid Quest ID" Error
- **Cause**: Prerequisite quest doesn't exist
- **Fix**: Create the prerequisite quest first, then reference it

### "Target Node Does Not Exist" Error
- **Cause**: Choice points to non-existent node
- **Fix**: Check node ID spelling, or create the target node

### Orphaned Nodes
- **Cause**: Nodes with no incoming choices
- **Fix**: Add a choice from another node that targets it

### Dialogue Won't Start
- **Cause**: Root node (Node 1) is missing
- **Fix**: Add a new node with ID 1, or set it as root

### Quest Not Progressing
- **Cause**: Objective parameters don't match game data
- **Fix**: Verify monster/item/map IDs exist in respective databases

## Advanced Features (Phase 13+)

### Quest Actions
- Start quests from dialogue
- Complete quest stages
- Give quest rewards

### Dialogue Conditions
- Check if player has quest
- Check player inventory
- Check player level
- Check custom flags

### Dialogue Actions
- Give/take items
- Give/take gold
- Set story flags
- Trigger events

## Exporting & Testing

1. **Save Campaign**: All changes saved to `quests.ron` and `dialogues.ron`
2. **Test in Game**: (Phase 14) Load campaign in game and test quest flow
3. **Validate**: Campaign Builder validates references before saving

## Next Steps

1. Create quest chains by linking prerequisites
2. Design branching dialogues with multiple endings
3. Set up quest-dialogue integration for dynamic storytelling
4. (Phase 13) Test quests in-game with test-play feature
5. (Phase 15) Use node-graph visualization for complex dialogues
