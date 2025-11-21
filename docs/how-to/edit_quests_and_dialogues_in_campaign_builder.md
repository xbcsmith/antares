# How to Edit Quests and Dialogues in Campaign Builder

This guide walks you through creating and managing quests and dialogue trees in the Antares Campaign Builder.

**Updated for Phase 7.1**: This guide now reflects the interactive quest stage and objective editing features with modal dialogs and inline edit/delete controls.

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

### Editing Quest Stages (Phase 7.1 Feature)

Quest stages can now be edited directly with modal dialogs:

1. **Expand the Quest** - Click on the quest in the list to show its stages
2. **Expand a Stage** - Click the collapsible header to reveal stage details and objectives
3. **Edit Stage** - Click the **✏️** button next to the stage name
   - A modal dialog appears with the stage editing form
   - Fields available:
     - **Stage Number**: The numerical order of this stage
     - **Name**: Stage title
     - **Description**: Multiline text describing what happens in this stage
     - **Require all objectives**: Checkbox - if checked, ALL objectives must be completed; if unchecked, completing ANY objective completes the stage
   - Make your changes
   - Click **✅ Save** to commit changes
   - Click **❌ Cancel** to discard changes
4. **Delete Stage** - Click the **🗑️** button next to the stage name
   - Stage is removed immediately
   - No confirmation dialog (changes can be reverted by not saving the campaign)

### Adding Objectives to a Stage

Each stage needs one or more objectives that guide players toward completion.

1. **Expand the stage** you want to add an objective to
2. In the objectives section, click the small **➕** button
3. Choose objective type from dropdown and fill in fields (see "Objective Type Reference" below)
4. Click **✅ Save**

### Editing Quest Objectives (Phase 7.1 Feature)

Objectives can now be edited with a dynamic modal form:

1. **Expand a Stage** - Click the collapsible stage header
2. **View Objectives** - See the numbered list of objectives
3. **Edit Objective** - Click the **✏️** button next to any objective
   - Modal dialog opens with **Objective Type** dropdown at the top
   - Current type is pre-selected
   - Form fields below show data for the current type
   - **Change Type** - Select a different objective type from the dropdown
     - Form fields update dynamically to match the new type
     - Previous data is cleared when type changes
   - Fill in type-specific fields (see reference below)
   - Click **✅ Save** to commit changes
   - Click **❌ Cancel** to discard changes
4. **Delete Objective** - Click the **🗑️** button next to the objective
   - Objective removed immediately
   - Remaining objectives maintain their order

### Objective Type Reference

When editing an objective, the form shows different fields based on the selected type:

#### Kill Monsters

- **Monster ID**: Numeric ID of the monster type (text input)
- **Quantity**: How many to kill (text input)
- Example: Monster ID `100`, Quantity `5` = "Kill 5 of Monster 100"

#### Collect Items

- **Item ID**: Numeric ID of the item (text input)
- **Quantity**: How many to collect (text input)
- Example: Item ID `42`, Quantity `3` = "Collect 3 of Item 42"

#### Reach Location

- **Map ID**: Numeric ID of the map (text input)
- **X**: X coordinate on the map (text input)
- **Y**: Y coordinate on the map (text input)
- **Radius**: Search radius in tiles (text input)
- Example: Map `10`, X `25`, Y `30`, Radius `2` = "Reach location (25, 30) on Map 10 within 2 tiles"

#### Talk To NPC

- **NPC ID**: Numeric ID or string identifier for the NPC (text input)
- **Map ID**: Which map the NPC is on (text input)
- Example: NPC `innkeeper`, Map `5` = "Talk to NPC innkeeper on Map 5"

#### Deliver Item

- **Item ID**: ID of item to deliver (text input)
- **NPC ID**: Who receives the item (text input)
- **Quantity**: How many to deliver (text input)
- Example: Item `100`, NPC `sage`, Quantity `1` = "Deliver 1 of Item 100 to NPC sage"

#### Escort NPC

- **NPC ID**: Who to escort (text input)
- **Map ID**: Destination map (text input)
- **Destination X**: Target X coordinate (text input)
- **Destination Y**: Target Y coordinate (text input)
- Example: NPC `princess`, Map `8`, X `50`, Y `40` = "Escort NPC princess to (50, 40) on Map 8"

#### Custom Flag

- **Flag Name**: Name of the custom game flag (text input)
- **Required Value**: True or false (checkbox)
- Example: Flag `dragon_defeated`, Value `true` = "Custom flag dragon_defeated must be true"

**Note**: All fields are text inputs (strings). The quest system will parse them when loading quests. Enter numeric values as text (e.g., "42" not 42).

### Setting Quest Giver

Every quest should have someone to give it out:

1. **Select the quest**
2. In quest details, fill in:
   - **NPC Name**: Who gives the quest (e.g., "Village Elder")
   - **Map ID**: Which map the NPC is on
   - **Position X, Y**: Where on that map to find them
3. Save the quest

### Deleting a Quest

1. **Select the quest** from the list
2. Right-click and select **Delete** from context menu, or
3. Click **🗑️** button next to the quest name
4. Quest is removed immediately

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

### Phase 7.1 UI Workflow Summary

Here's the typical editing workflow with the new interactive controls:

1. **Navigate to Quests tab** → See list of all quests
2. **Select a quest** → Quest details appear
3. **Expand quest stages** → Click collapsible headers
4. **Edit a stage**:
   - Click **✏️** → Modal opens
   - Modify fields → Click **✅ Save**
5. **Edit an objective**:
   - Click **✏️** on objective → Modal opens
   - Change type if needed → Form updates
   - Fill fields → Click **✅ Save**
6. **Delete items** → Click **🗑️** (immediate)
7. **Save campaign** → File → Save (or Ctrl+S)

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

### Editing Dialogue (Phase 7.7 Coming Soon)

**Note**: Dialogue node and choice editing with modal dialogs is planned for Phase 7.7. The backend methods exist; UI integration is in progress.

Current workflow:

1. **Select the dialogue** from the list
2. Click **✏️ Edit** to modify tree metadata
3. To edit nodes, select the node and use the form editor
4. Click **✅ Save**

### Deleting Dialogue Elements

1. **Select the dialogue** to delete the entire tree
2. Click **🗑️** button
3. To delete nodes: Select node and click **🗑️** (root node protected)
4. To delete choices: Select node, then click **🗑️** on the choice

### Dialogue Validation

The editor checks:

- ✅ Dialogue has a name
- ✅ Dialogue has at least one node
- ✅ Root node (Node 1) exists
- ✅ All choices point to valid nodes
- ✅ Associated quest exists (if specified)
- ✅ No unreachable nodes (orphaned content detection)

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
├── items.ron              # Items
├── spells.ron             # Spells
├── monsters.ron           # Monsters
└── maps/                   # Map files
```

## Tips & Best Practices

### Quest Design

1. **Clear Objectives**: Each stage should have obvious goals
2. **Progression**: Earlier stages easier than later ones
3. **Rewards**: Match rewards to difficulty and effort
4. **NPCs**: Always assign a quest giver location
5. **Chains**: Use prerequisite quests to create quest chains
6. **Variety**: Mix objective types (not all "kill X")
7. **Test Early**: Use the edit features to quickly iterate on objectives

### Dialogue Design

1. **Natural Speech**: Write how NPCs actually talk
2. **Branching**: Give players meaningful choices
3. **Length**: Keep individual lines reasonably short
4. **Navigation**: Always provide way back to main menu
5. **Clarity**: Make target of each choice obvious
6. **Personality**: Give each NPC distinct speech patterns

### Using Phase 7.1 Features Effectively

1. **Iterate Quickly**: Use the edit modal to rapidly test different objective types
2. **Convert Types**: Don't delete and recreate - just edit and change the type
3. **Descriptive Names**: Stage names show in collapsed view, make them clear
4. **Save Often**: Changes persist only when you save the campaign file
5. **Check Validation**: Expand the validation panel to catch issues early

### Validation

1. **Test Chains**: Verify quest prerequisites exist
2. **Check Targets**: Ensure dialogue choices lead to valid nodes
3. **Verify NPCs**: Confirm quest givers are on correct maps
4. **Consistency**: Use matching speaker names throughout
5. **Empty Stages**: Use orphaned content detection to find stages with no objectives

## Common Issues & Solutions

### Phase 7.1 Specific Issues

**Modal Dialog Won't Close**

- **Cause**: Clicked outside the modal or UI glitch
- **Fix**: Click **❌ Cancel** button explicitly to dismiss

**Changes Not Showing in List**

- **Cause**: UI hasn't refreshed after save
- **Fix**: Click away and back to the quest to refresh

**Objective Form Fields Not Updating**

- **Cause**: Type selector may need re-selection
- **Fix**: Select a different type, then select back to desired type

**Lost Changes After Cancel**

- **Cause**: Clicked Cancel instead of Save
- **Fix**: This is expected behavior - re-edit the item

### General Issues

**"Invalid Quest ID" Error**

- **Cause**: Prerequisite quest doesn't exist
- **Fix**: Create the prerequisite quest first, then reference it

**"Target Node Does Not Exist" Error**

- **Cause**: Choice points to non-existent node
- **Fix**: Check node ID spelling, or create the target node

**Orphaned Objectives**

- **Cause**: Stages with no objectives (empty)
- **Fix**: Use the orphaned content detection feature (Phase 7.5) to find them, then add objectives or delete the stage

**Quest Not Progressing**

- **Cause**: Objective parameters don't match game data
- **Fix**: Verify monster/item/map IDs exist in respective databases

## Testing Your Quests

### Manual Testing Workflow

1. **Create Quest** → Add stages → Add objectives
2. **Edit Multiple Times** → Test the edit/save cycle works
3. **Save Campaign** → File → Save
4. **Reload Campaign** → File → Open → Select same campaign.ron
5. **Verify Persistence** → Check all edits persisted correctly

### What to Test (Phase 7.1)

- [ ] Edit a stage → changes persist
- [ ] Delete a stage → removed correctly
- [ ] Edit an objective → changes persist
- [ ] Change objective type → form updates and saves
- [ ] Delete an objective → removed correctly
- [ ] Add objective → appears in list
- [ ] Modal Cancel → discards changes
- [ ] Save campaign → changes written to disk
- [ ] Reload campaign → changes still there

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
4. **Test Play**: (Coming soon) Test quests directly from Campaign Builder

## Next Steps

1. Create quest chains by linking prerequisites
2. Design branching dialogues with multiple endings
3. Set up quest-dialogue integration for dynamic storytelling
4. Use Phase 7.1 editing features to rapidly iterate on quest designs
5. (Phase 7.7) Use dialogue node editing with similar modal workflow
6. (Phase 13) Test quests in-game with test-play feature
7. (Phase 15) Use node-graph visualization for complex dialogues

## Changelog

### Phase 7.1 (2025-01-25)

- ✅ Added interactive stage editing with modal dialogs
- ✅ Added interactive objective editing with dynamic forms
- ✅ Added inline edit (✏️) and delete (🗑️) buttons
- ✅ Objective type selector with form field adaptation
- ✅ Immediate delete operations for stages and objectives

### Phase 7 Backend (2025-01-25)

- ✅ Quest stage CRUD operations (edit, save, delete)
- ✅ Quest objective CRUD operations (edit, save, delete)
- ✅ Orphaned content detection (stages with no objectives)

For more details, see `docs/explanation/implementations.md` Phase 7 and 7.1 sections.
