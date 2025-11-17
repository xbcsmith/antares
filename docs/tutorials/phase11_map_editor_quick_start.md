# Phase 11: Map Editor Quick Start

**Goal**: Create and edit maps visually in the Campaign Builder

**Time**: 5-10 minutes

---

## What You'll Learn

- How to open the map editor
- How to paint terrain and walls
- How to place events and NPCs
- How to save your map

---

## Step 1: Open Campaign Builder

```bash
cd antares/sdk/campaign_builder
cargo run
```

---

## Step 2: Create or Load a Campaign

**Option A - New Campaign:**
1. File → New Campaign
2. Fill in campaign name and ID
3. Click the **Maps** tab

**Option B - Existing Campaign:**
1. File → Open Campaign
2. Select `campaign.ron` file
3. Click the **Maps** tab

---

## Step 3: Create Your First Map

1. Click **➕ New Map** button
2. A 20x20 empty map appears
3. Map editor opens automatically

---

## Step 4: Paint Some Terrain

**Try This:**

1. Click **🎨 Terrain** dropdown
2. Select **Grass**
3. Click several tiles on the grid → They turn green!
4. Select **Water** from dropdown
5. Paint a small pond → Blue tiles appear

**Result**: You now have a grassy area with a water feature!

---

## Step 5: Add Walls

**Create a Room:**

1. Click **🧱 Wall** dropdown
2. Select **Normal**
3. Click tiles to create a square border (10 clicks)
4. Select **Door** from dropdown
5. Click one wall tile to make an entrance

**Result**: You have a room with a door!

---

## Step 6: Place an Event

**Add a Treasure Chest:**

1. Click **⚡ Event** button in toolbar
2. Click inside your room on the map
3. In the right panel, select event type: **Treasure**
4. Enter item IDs: `1, 2, 3`
5. Click **➕ Add Event**

**Result**: A red marker appears! That's your treasure chest.

---

## Step 7: Add an NPC

**Place a Guard:**

1. Click **👤 NPC** button in toolbar
2. Click near the door
3. In the right panel, fill in:
   - **NPC ID**: 1
   - **Name**: Guard
   - **Dialogue**: "Halt! State your business."
4. Click **➕ Add NPC**

**Result**: A yellow marker appears! That's your guard.

---

## Step 8: Use Undo/Redo

**Made a Mistake?**

- Click **↩ Undo** to reverse last action
- Click **↪ Redo** to replay it
- Try it: Paint a tile, undo it, redo it!

---

## Step 9: Validate Your Map

1. Click **🔄 Validate** button
2. Check the inspector panel for errors
3. Fix any issues (events on blocked tiles, etc.)

**Green = Good!** (Or no errors shown)

---

## Step 10: Save Your Map

1. Click **💾 Save** button
2. Status message confirms save
3. Map is saved as `maps/map_1.ron`

---

## View Your Map in the List

1. Click **← Back to List** button
2. You'll see your map in the list with:
   - Map ID
   - Size (20x20)
   - Event count (1)
   - NPC count (1)
3. Click the map entry to see a preview

---

## What You Created

In 10 steps, you made:
- ✅ A grassy outdoor area
- ✅ A small pond (water)
- ✅ A walled room with a door
- ✅ A treasure chest inside
- ✅ A guard NPC at the entrance

**Congratulations! You're a map designer!** 🎉

---

## Next Steps

### Learn More Tools

- **🪣 Fill Tool**: Paint large areas quickly
- **🧹 Erase**: Reset tiles to default
- **🔍 Select**: Inspect tile details

### Try Different Event Types

- **⚔️ Encounter**: Random monster battles
- **🌀 Teleport**: Travel between maps
- **🪤 Trap**: Damage and status effects
- **📜 Sign**: Display messages

### Advanced Techniques

- Create a town with multiple buildings
- Design a multi-room dungeon
- Link maps together with teleports
- Add quest NPCs and objectives

---

## Common Questions

**Q: How do I make bigger maps?**
A: Edit the code to call `Map::new(id, width, height)` with custom dimensions.

**Q: Can I edit existing maps?**
A: Yes! Click the **✏️** button next to any map in the list.

**Q: How do I delete a map?**
A: Click the **🗑** button next to the map (note: file isn't deleted yet).

**Q: Where are my maps saved?**
A: In `my_campaign/maps/map_X.ron` (RON format, human-readable).

**Q: Can I undo multiple actions?**
A: Yes! Keep clicking undo to go back through your edit history.

---

## Keyboard Shortcuts (Future)

These will be added in future phases:

- `Ctrl+Z` - Undo
- `Ctrl+Shift+Z` - Redo
- `Ctrl+S` - Save
- `Spacebar` - Select tool
- `1-7` - Switch tools

---

## Tips for Success

1. **Save often**: Click save after major changes
2. **Validate frequently**: Catch errors early
3. **Use descriptive names**: Name NPCs clearly ("Guard", not "NPC1")
4. **Plan before painting**: Sketch your layout on paper first
5. **Test your maps**: Play through them in the game engine

---

## Troubleshooting

**Problem**: "Event on blocked tile" error
**Solution**: Move the event to a walkable tile (grass, ground, dirt)

**Problem**: Can't see event markers
**Solution**: Check that "Events" toggle is enabled in View Options

**Problem**: Changes don't save
**Solution**: Ensure campaign directory exists and you have write permissions

---

## Full Documentation

- **Detailed Guide**: `docs/how-to/edit_maps_in_campaign_builder.md`
- **Implementation Details**: `docs/explanation/phase11_map_editor_integration_implementation.md`
- **Architecture**: `docs/reference/architecture.md` (Section 4.2 - World System)

---

## What's Next?

**Phase 12** (Coming Soon):
- Quest Designer - Create quest objectives
- Dialogue Tree Editor - Build branching conversations
- Quest-Dialogue Integration - Link quests to NPCs

**Phase 13** (After Phase 12):
- Campaign Packager - Bundle for distribution
- Test Play Mode - Play your campaign
- Asset Manager - Organize campaign files

---

**Happy Map Making!** 🗺️

*Created: 2025-01-26*
*Phase: 11 of 15*
*SDK Version: Campaign Builder v0.3*
