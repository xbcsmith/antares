# How to Edit Maps in the Campaign Builder

This guide explains how to create and edit game maps using the integrated map editor in the Antares Campaign Builder.

---

## Prerequisites

- Campaign Builder is installed and running
- A campaign is loaded or created
- Basic familiarity with the Campaign Builder interface

---

## Opening the Map Editor

### From Campaign Builder

1. Launch the Campaign Builder application
2. Open an existing campaign or create a new one
3. Click the **Maps** tab in the main navigation
4. You'll see the map list view

---

## Creating a New Map

### Step 1: Create the Map

1. In the map list view, click **➕ New Map**
2. A new empty 20x20 map is created with an auto-incremented ID
3. The map editor opens automatically in edit mode

### Step 2: Set Map Properties

The map is created with default properties:
- **ID**: Auto-incremented from existing maps
- **Size**: 20x20 tiles (can be changed by creating a map with `Map::new(id, width, height)`)
- **Terrain**: All tiles default to Ground with no walls

---

## Understanding the Map Editor Interface

### Tool Palette (Top Bar)

The tool palette contains all editing tools:

- **🔍 Select** - Click tiles to inspect them
- **🎨 Terrain** - Paint terrain types (dropdown menu)
- **🧱 Wall** - Paint walls (dropdown menu)
- **⚡ Event** - Place events on the map
- **👤 NPC** - Place NPCs on the map
- **🧹 Erase** - Reset tiles to default (ground, no wall)
- **↩ Undo** - Undo the last action
- **↪ Redo** - Redo the last undone action

### View Options

Toggle visual elements:
- **Grid** - Show/hide grid lines
- **Events** - Show/hide event markers (red)
- **NPCs** - Show/hide NPC markers (yellow)

### Map Grid (Center)

- Visual representation of your map
- Each square is one tile
- Click to interact based on active tool
- Scroll to navigate large maps
- Selected tile highlighted in yellow

### Inspector Panel (Right Side)

Shows information about the selected tile:
- Position (X, Y coordinates)
- Terrain type
- Wall type
- Blocked status (affects movement)
- Visited status
- Event details (if present)
- NPC details (if present)

---

## Painting Terrain

### Step 1: Select Terrain Tool

1. Click the **🎨 Terrain** dropdown in the tool palette
2. Choose a terrain type from the list

### Available Terrain Types

- **Ground** - Default walkable terrain (beige)
- **Grass** - Outdoor terrain (green)
- **Water** - Blocks movement unless player has special ability (blue)
- **Stone** - Indoor floor (gray)
- **Dirt** - Path or road (brown)
- **Forest** - Trees and vegetation (dark green)
- **Mountain** - Impassable terrain (brown/gray)
- **Swamp** - Slows movement (dark green)
- **Lava** - Damages party (red/orange)

### Step 2: Paint Tiles

1. Click on any tile in the map grid
2. The tile changes to the selected terrain type
3. Continue clicking to paint multiple tiles
4. Each action is added to the undo stack

### Tips

- Use **Ground** for most indoor areas
- Use **Grass** for outdoor areas
- Use **Mountain** or **Water** to create boundaries
- Use **Stone** for dungeons and castles

---

## Painting Walls

### Step 1: Select Wall Tool

1. Click the **🧱 Wall** dropdown in the tool palette
2. Choose a wall type from the list

### Available Wall Types

- **None** - No wall (default)
- **Normal** - Solid wall, blocks movement (gray)
- **Door** - Can be opened by player (brown)
- **Torch** - Light source, doesn't block movement (orange)

### Step 2: Paint Walls

1. Click on tiles to add walls
2. Walls appear as colored overlays on the terrain
3. Normal walls automatically set the tile to "blocked"

### Tips

- Use **Normal** walls to create room boundaries
- Place **Door** walls where players need passage
- Add **Torch** walls for lighting effects in dark areas
- Walls are independent of terrain (you can have a wall on any terrain type)

---

## Placing Events

Events trigger special actions when the player enters a tile.

### Step 1: Select Event Tool

1. Click the **⚡ Event** button in the tool palette
2. The event editor appears in the inspector panel

### Step 2: Click Map Position

1. Click on the map grid where you want the event
2. The position is set in the event editor
3. The selected tile is highlighted in yellow

### Step 3: Choose Event Type

In the inspector panel, select an event type:

#### 1. Encounter (⚔️)

Random monster battle when player enters tile.

**Properties**:
- **Monster IDs**: Comma-separated list of monster IDs (e.g., "1, 2, 3")

**Example**: `1, 1, 2` spawns two of monster #1 and one of monster #2

#### 2. Treasure (💰)

Treasure chest containing items.

**Properties**:
- **Item IDs**: Comma-separated list of item IDs (e.g., "10, 15, 20")

**Example**: `10, 15` gives the player items #10 and #15

#### 3. Teleport (🌀)

Instantly transports player to another location.

**Properties**:
- **Destination X**: X coordinate on target map
- **Destination Y**: Y coordinate on target map
- **Target Map ID**: ID of the destination map

**Example**: X=5, Y=10, Map=2 teleports to position (5,10) on map 2

#### 4. Trap (🪤)

Damages the party or applies a status effect.

**Properties**:
- **Damage**: Amount of HP damage dealt
- **Effect**: Optional status effect name (e.g., "poison", "paralyzed")

**Example**: Damage=10, Effect="poison"

#### 5. Sign (📜)

Displays a text message to the player.

**Properties**:
- **Sign Text**: The message to display (multiline supported)

**Example**: "Welcome to the village of Millhaven!"

#### 6. NPC Dialogue (💬)

Triggers a conversation with an NPC.

**Properties**:
- **NPC ID**: The ID of the NPC to talk to

**Example**: NPC ID=5 starts dialogue with NPC #5

### Step 4: Fill in Properties

1. Enter the required information for the event type
2. Click **➕ Add Event**
3. The event is placed on the map (shown as red overlay)

### Managing Events

- **View event**: Click tile with Select tool, details appear in inspector
- **Remove event**: Select tile, click **🗑 Remove Event** button
- **Edit event**: Remove old event, add new event at same position

---

## Placing NPCs

NPCs are characters that stand on specific tiles.

### Step 1: Select NPC Tool

1. Click the **👤 NPC** button in the tool palette
2. The NPC editor appears in the inspector panel

### Step 2: Click Map Position (Optional)

1. Click on the map grid to set NPC position
2. Coordinates are filled in automatically
3. Or enter position manually in the form

### Step 3: Fill in NPC Details

**NPC ID**: Unique identifier for this NPC (number)
**Name**: NPC's display name (e.g., "Guard", "Merchant")
**Position X**: X coordinate (filled if you clicked the map)
**Position Y**: Y coordinate (filled if you clicked the map)
**Dialogue**: What the NPC says when talked to (multiline supported)

### Step 4: Add NPC

1. Click **➕ Add NPC**
2. The NPC appears on the map (shown as yellow overlay)
3. The form is cleared for adding another NPC

### Managing NPCs

- **View NPC**: Click tile with Select tool, details appear in inspector
- **Remove NPC**: Not currently exposed in UI (future enhancement)
- **Edit NPC**: Add a new NPC with corrected information

---

## Using Undo/Redo

Every edit operation is saved to the undo stack.

### Undo Last Action

- Click **↩ Undo** button
- Or use keyboard shortcut (varies by OS)
- Reverses the last tile change, event placement, or NPC addition

### Redo Last Undone Action

- Click **↪ Redo** button
- Or use keyboard shortcut (varies by OS)
- Replays the last undone action

### Supported Operations

- Tile terrain changes
- Wall placement
- Event addition/removal
- NPC addition/removal

### Tips

- Undo stack is preserved until you close the editor
- Experiment freely - you can always undo
- Redo is only available if you haven't made new edits after undoing

---

## Validating Your Map

### Run Validation

1. Click **🔄 Validate** button in the top bar
2. Validation errors appear in the inspector panel

### Common Validation Errors

#### ❌ Event on Blocked Tile

**Problem**: An event is placed on a tile that blocks movement (wall, water, mountain).

**Fix**:
- Move the event to a walkable tile, or
- Change the terrain to a walkable type

#### ❌ NPC on Blocked Tile

**Problem**: An NPC is standing on a tile that blocks movement.

**Fix**:
- Move the NPC to a walkable tile, or
- Change the terrain/wall to allow passage

#### ⚠️ Empty Map

**Warning**: The map has no events or NPCs.

**Impact**: Map may feel empty or purposeless.

**Fix**: Add encounters, treasures, signs, or NPCs to make the map interesting

---

## Saving Your Map

Maps are automatically saved when you:
- Click **💾 Save** button
- Click **← Back to List** (prompts save if unsaved changes)

### Save Location

Maps are saved in the campaign directory:
```
my_campaign/
├── campaign.ron
└── maps/
    ├── map_1.ron
    ├── map_2.ron
    └── map_3.ron
```

### File Format

Maps are saved in RON (Rusty Object Notation) format:
- Human-readable
- Easy to edit manually if needed
- Version control friendly (text format)

---

## Map List View Features

### Searching/Filtering

1. Enter text in the **🔍 Search** field
2. Maps are filtered by ID matching the search term
3. Clear the search to show all maps

### Map Preview

1. Click on a map in the list
2. A small preview appears showing:
   - Blocked tiles (dark gray)
   - Events (red)
   - NPCs (yellow)
   - Overall map layout

### Reloading Maps

Click **🔄 Reload** to:
- Reload all maps from disk
- Pick up changes made externally
- Refresh the map list

### Deleting Maps

1. Click **🗑** button on a map
2. The map is removed from the list
3. **Warning**: This does not delete the file from disk yet (future enhancement)

---

## Tips and Best Practices

### Map Design

1. **Start with boundaries**: Use walls or blocked terrain to define map edges
2. **Add landmarks**: Use different terrain types to create visual variety
3. **Plan encounters**: Place encounters in logical locations (guards at gates, monsters in dungeons)
4. **Use signs**: Add signs to guide players and provide lore
5. **Test walkability**: Ensure players can reach all important areas

### Performance

- Keep maps under 100x100 for best performance
- Larger maps may have scrolling lag on low-end systems
- Use multiple smaller maps instead of one giant map

### Organization

- Use consistent ID numbering (e.g., 1-10 for town maps, 11-20 for dungeons)
- Name NPCs descriptively ("Guard", "Shopkeeper", not "NPC1")
- Document teleport destinations (write comments in campaign notes)

### Validation

- Run validation frequently while editing
- Fix errors immediately to avoid confusion later
- Treat warnings seriously - empty maps are boring

---

## Keyboard Shortcuts

(To be implemented in future versions)

- `Ctrl+Z` / `Cmd+Z` - Undo
- `Ctrl+Shift+Z` / `Cmd+Shift+Z` - Redo
- `Ctrl+S` / `Cmd+S` - Save
- `Spacebar` - Switch to Select tool
- `1-7` - Switch between tools
- `G` - Toggle grid
- `E` - Toggle event markers
- `N` - Toggle NPC markers

---

## Troubleshooting

### Map Won't Save

**Problem**: "Failed to save map" error appears.

**Solutions**:
- Check that campaign directory exists
- Verify you have write permissions
- Ensure maps directory exists in campaign folder
- Check disk space

### Changes Not Appearing

**Problem**: Edits don't show on the grid.

**Solutions**:
- Verify you clicked the tile after selecting a tool
- Check that undo didn't reverse your change
- Try reloading the map (Back to List, then Edit again)

### Events Not Visible

**Problem**: Can't see red event markers.

**Solutions**:
- Check that "Events" toggle is enabled in View Options
- Verify the event was actually added (check inspector)
- Try zooming in or scrolling to the event location

### Validation Errors Won't Clear

**Problem**: Validation keeps showing old errors.

**Solutions**:
- Fix the actual problem (move event/NPC, change terrain)
- Click **Validate** again to refresh
- Save and reload the map

---

## Advanced Techniques

### Creating Room Templates

1. Design a reusable room layout (e.g., 10x10 room with door)
2. Save the map
3. Copy the map RON file
4. Paste into new maps, changing IDs as needed

### Connecting Maps with Teleports

1. Create Map A and Map B
2. On Map A, place a teleport event at the exit (e.g., position 19,10)
3. Set destination to Map B's entrance (e.g., position 0,10)
4. On Map B, place a teleport event at the entrance back to Map A
5. Test both directions work correctly

### Creating Dungeon Levels

Use teleports to create multi-level dungeons:
- Map 10: Dungeon Level 1 (stairs down at 15,15 → Map 11)
- Map 11: Dungeon Level 2 (stairs up at 5,5 → Map 10)
- Map 11: (stairs down at 15,15 → Map 12)
- Map 12: Dungeon Level 3 (stairs up at 5,5 → Map 11)

### Populating Towns

1. Create outer walls with one entrance (door)
2. Add buildings using walls to create interior spaces
3. Place shop NPCs in appropriate buildings
4. Add signs for building labels ("Inn", "Blacksmith")
5. Place guards at town entrance
6. Add treasure or quest NPCs in hidden corners

---

## Next Steps

After creating your maps:

1. **Test Your Maps**: Use the game engine to play through your maps
2. **Balance Encounters**: Adjust monster groups based on difficulty
3. **Add Quests**: Link map events to quest objectives (Phase 12)
4. **Create Dialogue**: Use the dialogue editor to create NPC conversations (Phase 12)
5. **Package Campaign**: Create a distributable campaign package (Phase 13)

---

## See Also

- [Campaign Builder Overview](../tutorials/campaign_builder_getting_started.md)
- [Architecture Reference - World System](../reference/architecture.md#world-system)
- [Phase 11 Implementation](../explanation/phase11_map_editor_integration_implementation.md)
- [Map Data Format](../reference/map_ron_format.md)

---

**Last Updated**: 2025-01-26
**Version**: Phase 11
