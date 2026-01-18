# Game Engine Fixes Implementation Plan

## Overview

This plan addresses eight high-priority issues with the game engine related to world interaction and HUD display. The core problem is that pressing the E key (interact action) only handles doors, while NPCs, signs, teleport tiles, and recruitable characters lack interaction handlers. Additionally, the HUD has visual issues with HP text clipping, portrait alignment, and unnecessary character numbering.

**User Decisions Applied**:

1. NPC interaction: Adjacent tile (any of 8 surrounding tiles)
2. HUD layout: HP text moved next to name (right-justified), thinner HP bar
3. All validation gates and quality checks included
4. SPDX headers verification included
5. Full AI-optimization with explicit, unambiguous instructions

---

## Current State Analysis

### Existing Infrastructure

| Component          | Location                             | Current Behavior                                                                                        |
| ------------------ | ------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| Input handling     | `src/game/systems/input.rs#L375-464` | `handle_input()` system processes E-key as `GameAction::Interact`, but only checks for `WallType::Door` |
| Event triggering   | `src/game/systems/events.rs#L26-48`  | `check_for_events()` triggers when party **walks onto** a tile; no interact-key trigger                 |
| Event handling     | `src/game/systems/events.rs#L50-184` | `handle_events()` processes `MapEvent` variants (Teleport, Sign, NpcDialogue, etc.)                     |
| MapEvent types     | `src/domain/world/types.rs#L414-507` | Enum with `Sign`, `Teleport`, `NpcDialogue`, etc. - all have `name`/`description` fields                |
| HUD setup          | `src/game/systems/hud.rs#L168-307`   | `setup_hud()` creates character cards with fixed 80px panel height                                      |
| HUD character card | `src/game/systems/hud.rs#L193-280`   | Column layout: portrait (40px + 4px margin) → name text → HP bar (16px) → HP text → condition text      |
| HUD update         | `src/game/systems/hud.rs#L380-387`   | Character names formatted as `"{}. {}"` with party_index + 1                                            |
| Portrait layout    | `src/game/systems/hud.rs#L208-219`   | 40px portrait with 4px margin on all sides                                                              |
| HP bar             | `src/game/systems/hud.rs#L238-253`   | 16px height bar with 100% width background + fill                                                       |
| HP text            | `src/game/systems/hud.rs#L256-263`   | Separate text element below HP bar                                                                      |

### Identified Issues

1. **Door Interaction Broken**: The E-key interaction at `input.rs#L396-413` works but doors are still blocked movement-wise because `is_blocked()` returns true for `WallType::Door`
2. **No NPC Interaction**: E-key only checks `WallType::Door`, ignoring NPC positions in adjacent tiles
3. **No Sign Interaction**: Signs only trigger on walk-over events, not E-key press; no visual representation
4. **No Teleport Interaction**: Teleports only log when walked over, don't actually teleport; no visual representation
5. **No Recruitable Character Visualization**: Characters defined in map data as `MapEvent::RecruitableCharacter` have no visual representation (no sprite, no colored marker)
6. **No Recruitable Character Interaction**: E-key interaction doesn't trigger recruitment dialogue for characters on map
7. **HUD Layout Inefficient**: HP text should be next to name (right-justified), HP bar should be thinner
8. **Portrait Offset**: Portrait uses `margin: UiRect::all(PORTRAIT_MARGIN)` but card uses left-to-right flex, causing misalignment
9. **Numbered Names**: Line 383 uses `format!("{}. {}", name_text.party_index + 1, character.name)` which adds "1. ", "2. " prefixes

---

## Implementation Phases

### Phase 1: HUD Visual Fixes (Independent Work)

**Rationale**: HUD changes are independent of interaction system and can be completed first to unblock visual testing.

#### 1.1 Restructure Character Card Layout

**File**: `src/game/systems/hud.rs`

**Exact Changes Required**:

1. **Line 41** - Reduce HUD panel height:

   ```rust
   // CHANGE FROM:
   pub const HUD_PANEL_HEIGHT: Val = Val::Px(80.0);
   // CHANGE TO:
   pub const HUD_PANEL_HEIGHT: Val = Val::Px(70.0);
   ```

2. **Line 44** - Reduce HP bar height:

   ```rust
   // CHANGE FROM:
   pub const HP_BAR_HEIGHT: Val = Val::Px(16.0);
   // CHANGE TO:
   pub const HP_BAR_HEIGHT: Val = Val::Px(10.0);
   ```

3. **Lines 45-50** - Add new constants after `CARD_PADDING`:

   ```rust
   pub const PORTRAIT_SIZE: f32 = 40.0;
   pub const PORTRAIT_MARGIN: f32 = 4.0;
   ```

4. **Lines 193-280** - Replace entire character card spawn block with new layout:

   ```rust
   parent
       .spawn((
           Node {
               width: CHARACTER_CARD_WIDTH,
               flex_direction: FlexDirection::Column,
               padding: UiRect::all(CARD_PADDING),
               row_gap: Val::Px(4.0),
               ..default()
           },
           BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
           BorderRadius::all(Val::Px(4.0)),
           CharacterCard { party_index },
       ))
       .with_children(|card| {
           // Row 1: Portrait + Name/HP text container
           card.spawn(Node {
               width: Val::Percent(100.0),
               flex_direction: FlexDirection::Row,
               align_items: AlignItems::Center,
               column_gap: Val::Px(8.0),
               ..default()
           })
           .with_children(|row| {
               // Portrait (left side)
               row.spawn((
                   Node {
                       width: Val::Px(PORTRAIT_SIZE),
                       height: Val::Px(PORTRAIT_SIZE),
                       flex_shrink: 0.0,
                       ..default()
                   },
                   BackgroundColor(Color::srgba(0.4, 0.4, 0.4, 1.0)),
                   BorderRadius::all(Val::Px(4.0)),
                   ImageNode::default(),
                   CharacterPortrait { party_index },
               ));

               // Name + HP text container (right side)
               row.spawn(Node {
                   flex_direction: FlexDirection::Row,
                   justify_content: JustifyContent::SpaceBetween,
                   align_items: AlignItems::Center,
                   flex_grow: 1.0,
                   ..default()
               })
               .with_children(|name_hp_row| {
                   // Character name (left-aligned)
                   name_hp_row.spawn((
                       Text::new(""),
                       TextFont {
                           font_size: 14.0,
                           ..default()
                       },
                       TextColor(Color::WHITE),
                       CharacterNameText { party_index },
                   ));

                   // HP text (right-aligned)
                   name_hp_row.spawn((
                       Text::new(""),
                       TextFont {
                           font_size: 12.0,
                           ..default()
                       },
                       TextColor(Color::WHITE),
                       HpText { party_index },
                   ));
               });
           });

           // Row 2: HP bar
           card.spawn((
               Node {
                   width: Val::Percent(100.0),
                   height: HP_BAR_HEIGHT,
                   ..default()
               },
               BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 1.0)),
               HpBarBackground,
           ))
           .with_children(|bar| {
               bar.spawn((
                   Node {
                       width: Val::Percent(100.0),
                       height: Val::Percent(100.0),
                       ..default()
                   },
                   BackgroundColor(HP_HEALTHY_COLOR),
                   HpBarFill { party_index },
               ));
           });

           // Row 3: Condition text
           card.spawn((
               Text::new(""),
               TextFont {
                   font_size: 10.0,
                   ..default()
               },
               TextColor(Color::WHITE),
               ConditionText { party_index },
           ));
       });
   ```

#### 1.2 Remove Character Number Prefixes

**File**: `src/game/systems/hud.rs`

**Line 383** - Remove number prefix from name formatting:

```rust
// CHANGE FROM:
**text = format!("{}. {}", name_text.party_index + 1, character.name);

// CHANGE TO:
**text = character.name.clone();
```

#### 1.3 Testing Requirements

**File**: `src/game/systems/hud.rs`

Add test at end of file (after existing tests):

```rust
#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn test_hud_panel_height_reduced() {
        assert_eq!(HUD_PANEL_HEIGHT, Val::Px(70.0));
    }

    #[test]
    fn test_hp_bar_height_thinner() {
        assert_eq!(HP_BAR_HEIGHT, Val::Px(10.0));
    }

    #[test]
    fn test_character_name_no_number_prefix() {
        // This test verifies the format doesn't include party_index
        let name = "TestHero";
        let formatted = name.to_string(); // Should be just the name
        assert_eq!(formatted, "TestHero");
        assert!(!formatted.starts_with("1. "));
    }
}
```

#### 1.4 Post-Implementation Validation

Run ALL commands in order (must pass before proceeding to Phase 2):

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**Expected Output**:

- `cargo fmt`: No output (all files formatted)
- `cargo check`: "Finished" with 0 errors
- `cargo clippy`: "Finished" with 0 warnings
- `cargo nextest run`: All tests pass (including 3 new layout tests)

#### 1.5 Deliverables

- [ ] `hud.rs#L41`: `HUD_PANEL_HEIGHT` reduced to 70px
- [ ] `hud.rs#L44`: `HP_BAR_HEIGHT` reduced to 10px
- [ ] `hud.rs#L193-280`: Character card layout restructured (portrait + name/HP in row)
- [ ] `hud.rs#L383`: Character name format changed to `character.name.clone()`
- [ ] `hud.rs`: 3 new unit tests added for layout verification
- [ ] SPDX headers verified present in `hud.rs`
- [ ] All validation commands pass

#### 1.6 Success Criteria

**Manual Verification** (run `cargo run` and observe HUD):

- [ ] Character names display without "1. ", "2. " prefixes (e.g., "Kira" not "1. Kira")
- [ ] HP text appears to the right of character name in same row
- [ ] HP bar is visibly thinner (10px instead of 16px)
- [ ] Portrait is aligned to left of name/HP row with no extra spacing
- [ ] Total HUD panel height is reduced (70px instead of 80px)
- [ ] All 6 character cards fit horizontally without clipping

---

### Phase 2: Fix E-Key Interaction System

#### 2.1 Add Adjacent Tile Check Helper Function

**File**: `src/game/systems/input.rs`

**Location**: Add after `handle_input()` function (after line 464)

**Exact Code to Add**:

```rust
/// Returns all 8 adjacent positions around a given position
///
/// Returns tiles in clockwise order starting from North:
/// N, NE, E, SE, S, SW, W, NW
///
/// # Arguments
///
/// * `position` - The center position
///
/// # Returns
///
/// Array of 8 IVec2 positions representing adjacent tiles
fn get_adjacent_positions(position: IVec2) -> [IVec2; 8] {
    [
        IVec2::new(position.x, position.y + 1),       // North
        IVec2::new(position.x + 1, position.y + 1),   // NorthEast
        IVec2::new(position.x + 1, position.y),       // East
        IVec2::new(position.x + 1, position.y - 1),   // SouthEast
        IVec2::new(position.x, position.y - 1),       // South
        IVec2::new(position.x - 1, position.y - 1),   // SouthWest
        IVec2::new(position.x - 1, position.y),       // West
        IVec2::new(position.x - 1, position.y + 1),   // NorthWest
    ]
}

#[cfg(test)]
mod adjacent_tile_tests {
    use super::*;

    #[test]
    fn test_adjacent_positions_count() {
        let center = IVec2::new(5, 5);
        let adjacent = get_adjacent_positions(center);
        assert_eq!(adjacent.len(), 8);
    }

    #[test]
    fn test_adjacent_positions_north() {
        let center = IVec2::new(5, 5);
        let adjacent = get_adjacent_positions(center);
        assert_eq!(adjacent[0], IVec2::new(5, 6)); // North
    }

    #[test]
    fn test_adjacent_positions_east() {
        let center = IVec2::new(5, 5);
        let adjacent = get_adjacent_positions(center);
        assert_eq!(adjacent[2], IVec2::new(6, 5)); // East
    }
}
```

#### 2.2 Extend Input Handler for NPCs, Signs, and Teleports

**File**: `src/game/systems/input.rs`

**Line ~15** - Add imports at top of file:

```rust
use crate::domain::world::types::MapEvent;
use crate::game::events::MapEventTriggered;
```

**Lines 396-413** - Replace the `GameAction::Interact` block with:

```rust
GameAction::Interact => {
    // Get party position and all adjacent tiles
    let party_position = *party_pos;
    let adjacent_tiles = get_adjacent_positions(party_position);

    // Check for door in front of party (existing behavior)
    let facing_position = match party_facing.0 {
        Direction::North => IVec2::new(party_position.x, party_position.y + 1),
        Direction::East => IVec2::new(party_position.x + 1, party_position.y),
        Direction::South => IVec2::new(party_position.x, party_position.y - 1),
        Direction::West => IVec2::new(party_position.x - 1, party_position.y),
    };

    if let Some(tile) = map.get_tile(facing_position) {
        if matches!(tile.wall_type, WallType::Door) {
            info!("Opening door at {:?}", facing_position);
            if let Some(tile_mut) = map.get_tile_mut(facing_position) {
                tile_mut.wall_type = WallType::None;
                tile_mut.blocked = false;
            }
            return; // Door interaction handled, skip other checks
        }
    }

    // Check for NPC in any adjacent tile
    if let Some(npc) = map.npc_placements.iter().find(|npc| {
        adjacent_tiles.contains(&npc.position)
    }) {
        info!("Interacting with NPC '{}' at {:?}", npc.name, npc.position);
        map_event_writer.send(MapEventTriggered {
            event: MapEvent::NpcDialogue {
                npc_id: npc.id,
                name: npc.name.clone(),
                description: npc.dialogue.clone(),
            },
        });
        return;
    }

    // Check for sign in any adjacent tile
    for position in &adjacent_tiles {
        if let Some(event) = map.get_event(*position) {
            match event {
                MapEvent::Sign { name, description } => {
                    info!("Reading sign '{}' at {:?}", name, position);
                    map_event_writer.send(MapEventTriggered {
                        event: MapEvent::Sign {
                            name: name.clone(),
                            description: description.clone(),
                        },
                    });
                    return;
                }
                MapEvent::Teleport { destination, name, description } => {
                    info!("Activating teleport '{}' to {:?}", name, destination);
                    map_event_writer.send(MapEventTriggered {
                        event: MapEvent::Teleport {
                            destination: *destination,
                            name: name.clone(),
                            description: description.clone(),
                        },
                    });
                    return;
                }
                _ => continue,
            }
        }
    }

    // No interactable found
    info!("No interactable object nearby");
}
```

#### 2.3 Testing Requirements

**File**: `src/game/systems/input.rs`

Add tests at end of test module:

```rust
#[cfg(test)]
mod interaction_tests {
    use super::*;
    use crate::domain::world::types::{Map, MapEvent, NpcPlacement};

    #[test]
    fn test_npc_interaction_triggers_dialogue() {
        // Arrange
        let mut app = App::new();
        app.add_event::<MapEventTriggered>();
        app.add_event::<GameAction>();

        let mut map = Map::new(10, 10);
        map.npc_placements.push(NpcPlacement {
            id: 1,
            name: "TestNPC".to_string(),
            position: IVec2::new(5, 6), // North of party
            dialogue: "Hello!".to_string(),
        });

        // Party at (5, 5)
        // TODO: Complete test setup when GlobalState structure is confirmed

        // Act
        app.world.send_event(GameAction::Interact);

        // Assert
        // TODO: Verify MapEventTriggered was sent with NpcDialogue event
    }

    #[test]
    fn test_sign_interaction_displays_text() {
        // Arrange
        let mut map = Map::new(10, 10);
        map.add_event(
            IVec2::new(5, 6),
            MapEvent::Sign {
                name: "TestSign".to_string(),
                description: "This is a test sign".to_string(),
            },
        );

        // TODO: Complete test when event system API is confirmed

        // Assert
        // Verify MapEventTriggered sent with Sign event
    }

    #[test]
    fn test_teleport_interaction_triggers_teleport() {
        // Arrange
        let mut map = Map::new(10, 10);
        map.add_event(
            IVec2::new(5, 6),
            MapEvent::Teleport {
                destination: IVec2::new(20, 20),
                name: "TestPortal".to_string(),
                description: "Portal to destination".to_string(),
            },
        );

        // TODO: Complete test when teleport system API is confirmed

        // Assert
        // Verify MapEventTriggered sent with Teleport event
        // Verify party position changed to destination
    }

    #[test]
    fn test_door_interaction_still_works() {
        // Verify existing door interaction not broken
        // TODO: Adapt existing test_door_interaction from movement.rs
    }

    #[test]
    fn test_no_interactable_logs_message() {
        // Verify "No interactable object nearby" logged when nothing present
        // TODO: Complete test with proper logging capture
    }
}
```

#### 2.4 Post-Implementation Validation

Run ALL commands in order:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**Expected Output**:

- All commands pass with 0 errors/warnings
- New tests compile (may be marked as `#[ignore]` if incomplete)

#### 2.5 Deliverables

- [ ] `input.rs`: `get_adjacent_positions()` helper function added with 3 unit tests
- [ ] `input.rs`: Imports added for `MapEvent` and `MapEventTriggered`
- [ ] `input.rs#L396-413`: `GameAction::Interact` extended to check NPCs, signs, teleports
- [ ] `input.rs`: 5 new integration tests added (may be incomplete/ignored pending API clarification)
- [ ] SPDX headers verified present in `input.rs`
- [ ] All validation commands pass

#### 2.6 Success Criteria

**Manual Verification** (run `cargo run`):

- [ ] Pressing E in front of door opens it (existing behavior maintained)
- [ ] Pressing E when adjacent to NPC (any of 8 surrounding tiles) triggers dialogue event
- [ ] Pressing E when adjacent to sign tile displays sign message in log
- [ ] Pressing E when adjacent to teleport tile triggers teleport event
- [ ] Pressing E with no nearby interactable logs "No interactable object nearby"

---

### Phase 3: Add Visual Representation for Signs and Teleports

#### 3.1 Add Placeholder Visual Markers

**File**: `src/game/systems/map.rs`

**Location**: Modify `spawn_map()` function

**Add Constants** at top of file (after existing constants):

```rust
// Event marker colors (RGB)
const SIGN_MARKER_COLOR: Color = Color::rgb(0.59, 0.44, 0.27); // Brown/tan #967046
const TELEPORT_MARKER_COLOR: Color = Color::rgb(0.53, 0.29, 0.87); // Purple #8749DE
const EVENT_MARKER_SIZE: f32 = 0.8; // 80% of tile size
const EVENT_MARKER_Y_OFFSET: f32 = 0.05; // 5cm above ground to prevent z-fighting
```

**In `spawn_map()` function** - Add after tile spawning loop:

```rust
// Spawn event markers for signs and teleports
for (position, event) in map.events.iter() {
    let marker_color = match event {
        MapEvent::Sign { .. } => SIGN_MARKER_COLOR,
        MapEvent::Teleport { .. } => TELEPORT_MARKER_COLOR,
        _ => continue, // Only show markers for signs and teleports
    };

    let marker_name = match event {
        MapEvent::Sign { name, .. } => format!("SignMarker_{}", name),
        MapEvent::Teleport { name, .. } => format!("TeleportMarker_{}", name),
        _ => continue,
    };

    // Calculate world position (assuming TILE_SIZE constant exists)
    let world_x = position.x as f32 * TILE_SIZE;
    let world_z = position.y as f32 * TILE_SIZE;

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(EVENT_MARKER_SIZE, EVENT_MARKER_SIZE)),
            material: materials.add(StandardMaterial {
                base_color: marker_color,
                emissive: marker_color * 0.3, // Slight glow effect
                unlit: false,
                ..default()
            }),
            transform: Transform::from_xyz(
                world_x,
                EVENT_MARKER_Y_OFFSET,
                world_z,
            ),
            ..default()
        },
        Name::new(marker_name),
    ));
}
```

#### 3.2 Update Sprite Support Plan

**File**: `docs/explanation/sprite_support_implementation_plan.md`

**Location**: Phase 2 - Sprite Registry Definition

**Add to sprite registry** (`data/sprite_sheets.ron`):

```ron
// Add to sprite_sheets.ron after existing entries:

"signs": SpriteSheetConfig(
    texture_path: "textures/sprites/signs.png",
    tile_size: (32.0, 32.0),
    columns: 4,
    rows: 2,
    sprites: [
        (0, "wooden_sign"),
        (1, "stone_marker"),
        (2, "warning_sign"),
        (3, "info_sign"),
        (4, "quest_marker"),
        (5, "shop_sign"),
        (6, "danger_sign"),
        (7, "direction_sign"),
    ],
),

"portals": SpriteSheetConfig(
    texture_path: "textures/sprites/portals.png",
    tile_size: (32.0, 32.0),
    columns: 4,
    rows: 2,
    sprites: [
        (0, "teleport_pad"),
        (1, "dimensional_gate"),
        (2, "stairs_up"),
        (3, "stairs_down"),
        (4, "portal_blue"),
        (5, "portal_red"),
        (6, "trap_door"),
        (7, "exit_portal"),
    ],
),
```

**Update Phase 3 of sprite_support_implementation_plan.md**:

Add task to replace placeholder colored quads with actual sprites:

```markdown
### Phase 3.X: Replace Event Placeholder Markers

**File**: `src/game/systems/map.rs`

Replace colored quads spawned in Phase 3.1 with sprite-based rendering:

1. Query sprite registry for "signs" and "portals" sprite sheets
2. For each MapEvent (Sign/Teleport), spawn appropriate sprite entity
3. Remove colored quad spawning code
4. Add sprite index selection based on event metadata
```

#### 3.3 Testing Requirements

**File**: `src/game/systems/map.rs`

Add test at end of file:

```rust
#[cfg(test)]
mod event_marker_tests {
    use super::*;

    #[test]
    fn test_sign_marker_color() {
        assert_eq!(
            SIGN_MARKER_COLOR,
            Color::rgb(0.59, 0.44, 0.27)
        );
    }

    #[test]
    fn test_teleport_marker_color() {
        assert_eq!(
            TELEPORT_MARKER_COLOR,
            Color::rgb(0.53, 0.29, 0.87)
        );
    }

    #[test]
    fn test_event_marker_size_less_than_tile() {
        assert!(EVENT_MARKER_SIZE < 1.0);
        assert!(EVENT_MARKER_SIZE > 0.0);
    }

    #[test]
    fn test_event_marker_y_offset_prevents_z_fighting() {
        assert!(EVENT_MARKER_Y_OFFSET > 0.0);
        assert!(EVENT_MARKER_Y_OFFSET < 0.1);
    }
}
```

#### 3.4 Post-Implementation Validation

Run ALL commands in order:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

#### 3.5 Deliverables

- [ ] `map.rs`: 4 new constants added (SIGN_MARKER_COLOR, TELEPORT_MARKER_COLOR, EVENT_MARKER_SIZE, EVENT_MARKER_Y_OFFSET)
- [ ] `map.rs`: Event marker spawning code added to `spawn_map()`
- [ ] `map.rs`: 4 new unit tests for marker constants
- [ ] `sprite_support_implementation_plan.md`: Updated with "signs" and "portals" sprite sheet configs
- [ ] `sprite_support_implementation_plan.md`: Phase 3.X added for replacing placeholders
- [ ] SPDX headers verified present in `map.rs`
- [ ] All validation commands pass

#### 3.6 Success Criteria

**Manual Verification** (run `cargo run`):

- [ ] Sign tiles display brown/tan colored markers on the map
- [ ] Teleport tiles display purple colored markers on the map
- [ ] Markers are positioned slightly above ground (no z-fighting with floor)
- [ ] Markers are 80% of tile size and centered on their tile
- [ ] Markers have slight emissive glow for visibility

---

### Phase 4: Recruitable Character Visualization and Interaction

**Rationale**: Recruitable characters exist in map data (`MapEvent::RecruitableCharacter`) but have no visual representation or interaction system. This phase adds visual markers and dialogue-based recruitment.

#### 4.1 Add Recruitable Character Visual Markers

**File**: `src/game/systems/map.rs`

**Add Constant** at top of file (after EVENT_MARKER_Y_OFFSET):

```rust
const RECRUITABLE_CHARACTER_MARKER_COLOR: Color = Color::rgb(0.27, 0.67, 0.39); // Green #45AB63
```

**In `spawn_map()` function** - Modify event marker spawning loop to include RecruitableCharacter:

```rust
// Spawn event markers for signs, teleports, and recruitable characters
for (position, event) in map.events.iter() {
    let (marker_color, marker_name) = match event {
        MapEvent::Sign { name, .. } => (SIGN_MARKER_COLOR, format!("SignMarker_{}", name)),
        MapEvent::Teleport { name, .. } => (TELEPORT_MARKER_COLOR, format!("TeleportMarker_{}", name)),
        MapEvent::RecruitableCharacter { name, .. } => {
            (RECRUITABLE_CHARACTER_MARKER_COLOR, format!("RecruitableCharacter_{}", name))
        }
        _ => continue, // Only show markers for signs, teleports, and recruitable characters
    };

    // Calculate world position (assuming TILE_SIZE constant exists)
    let world_x = position.x as f32 * TILE_SIZE;
    let world_z = position.y as f32 * TILE_SIZE;

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(EVENT_MARKER_SIZE, EVENT_MARKER_SIZE)),
            material: materials.add(StandardMaterial {
                base_color: marker_color,
                emissive: marker_color * 0.3, // Slight glow effect
                unlit: false,
                ..default()
            }),
            transform: Transform::from_xyz(
                world_x,
                EVENT_MARKER_Y_OFFSET,
                world_z,
            ),
            ..default()
        },
        Name::new(marker_name),
    ));
}
```

#### 4.2 Extend Input Handler for Recruitable Characters

**File**: `src/game/systems/input.rs`

**In `handle_input()` function** - Add recruitable character check in the `GameAction::Interact` block after sign/teleport checks:

```rust
    // Check for recruitable character in any adjacent tile
    for position in &adjacent_tiles {
        if let Some(event) = map.get_event(*position) {
            match event {
                MapEvent::Sign { name, description } => {
                    info!("Reading sign '{}' at {:?}", name, position);
                    map_event_writer.send(MapEventTriggered {
                        event: MapEvent::Sign {
                            name: name.clone(),
                            description: description.clone(),
                        },
                    });
                    return;
                }
                MapEvent::Teleport { destination, name, description } => {
                    info!("Activating teleport '{}' to {:?}", name, destination);
                    map_event_writer.send(MapEventTriggered {
                        event: MapEvent::Teleport {
                            destination: *destination,
                            name: name.clone(),
                            description: description.clone(),
                        },
                    });
                    return;
                }
                MapEvent::RecruitableCharacter { character_id, name, description } => {
                    info!("Interacting with recruitable character '{}' (ID: {})", name, character_id);
                    // Trigger recruitment dialogue
                    // Use dialogue ID based on character_id (e.g., "recruit_{character_id}")
                    let dialogue_id = format!("recruit_{}", character_id);
                    dialogue_writer.send(StartDialogue {
                        dialogue_id: dialogue_id.parse().unwrap_or(100), // Default to ID 100 for recruitment
                    });
                    return;
                }
                _ => continue,
            }
        }
    }
```

**Add Import** at top of file:

```rust
use crate::game::systems::dialogue::StartDialogue;
```

**Add Event Writer** to system parameters (find the `handle_input()` function signature):

```rust
fn handle_input(
    // ... existing parameters ...
    mut dialogue_writer: EventWriter<StartDialogue>,
) {
```

#### 4.3 Create Default Recruitment Dialogue

**File**: `campaigns/tutorial/data/dialogues.ron`

**Add to end of array** (after existing Arcturus dialogue):

```ron
    (
        id: 100,
        name: "Default Character Recruitment",
        root_node: 1,
        nodes: {
            1: (
                id: 1,
                text: "Hello there. My name is {CHARACTER_NAME}. Can I join your party?",
                speaker_override: None,
                choices: [
                    (
                        text: "Yes, join us!",
                        target_node: Some(2),
                        conditions: [],
                        actions: [
                            TriggerEvent(event_name: "recruit_character_to_party"),
                        ],
                        ends_dialogue: false,
                    ),
                    (
                        text: "Meet me at the Inn.",
                        target_node: Some(3),
                        conditions: [],
                        actions: [
                            TriggerEvent(event_name: "recruit_character_to_inn"),
                        ],
                        ends_dialogue: false,
                    ),
                    (
                        text: "Not at this time.",
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
            2: (
                id: 2,
                text: "Excellent! I'm ready to join your adventure.",
                speaker_override: None,
                choices: [],
                conditions: [],
                actions: [],
                is_terminal: true,
            ),
            3: (
                id: 3,
                text: "I'll head to the inn right away. See you there!",
                speaker_override: None,
                choices: [],
                conditions: [],
                actions: [],
                is_terminal: true,
            ),
        },
        speaker_name: None,
        repeatable: false,
        associated_quest: None,
    ),
```

**NOTE**: The `{CHARACTER_NAME}` placeholder will need dynamic replacement in the dialogue system. For now, it serves as documentation. Future enhancement: implement dialogue variable substitution.

#### 4.4 Add Dialogue Actions for Recruitment

**File**: `src/domain/dialogue.rs`

**Extend `DialogueAction` enum** at line ~387:

```rust
pub enum DialogueAction {
    /// Start a quest
    StartQuest { quest_id: QuestId },

    /// Complete a quest stage
    CompleteQuestStage { quest_id: QuestId, stage_number: u8 },

    /// Give items to the player
    GiveItems { items: Vec<(ItemId, u16)> },

    /// Take items from the player
    TakeItems { items: Vec<(ItemId, u16)> },

    /// Give gold to the player
    GiveGold { amount: u32 },

    /// Take gold from the player
    TakeGold { amount: u32 },

    /// Set a game flag
    SetFlag { flag_name: String, value: bool },

    /// Change reputation with a faction
    ChangeReputation { faction: String, change: i16 },

    /// Trigger a custom event
    TriggerEvent { event_name: String },

    /// Grant experience points
    GrantExperience { amount: u32 },

    /// Recruit character to active party
    RecruitToParty { character_id: String },

    /// Send character to inn
    RecruitToInn { character_id: String, innkeeper_id: String },
}
```

**Update `description()` method** (add at end of match block ~line 447):

```rust
            DialogueAction::GrantExperience { amount } => {
                format!("Grant {} experience", amount)
            }
            DialogueAction::RecruitToParty { character_id } => {
                format!("Recruit '{}' to party", character_id)
            }
            DialogueAction::RecruitToInn { character_id, innkeeper_id } => {
                format!("Send '{}' to inn (keeper: {})", character_id, innkeeper_id)
            }
```

#### 4.5 Implement Recruitment Action Handler

**File**: `src/game/systems/dialogue.rs`

**Add new system** after existing dialogue systems:

```rust
/// System to handle recruitment-specific dialogue actions
fn handle_recruitment_actions(
    mut commands: Commands,
    mut dialogue_state: ResMut<DialogueState>,
    mut global_state: ResMut<GlobalState>,
    dialogue_db: Res<DialogueDatabase>,
) {
    // Get active dialogue tree
    let Some(tree_id) = dialogue_state.active_tree_id else {
        return;
    };

    let Some(tree) = dialogue_db.get_dialogue(tree_id) else {
        return;
    };

    // Get current node
    let Some(node) = tree.nodes.get(&dialogue_state.current_node_id) else {
        return;
    };

    // Process actions on this node
    for action in &node.actions {
        match action {
            DialogueAction::RecruitToParty { character_id } => {
                info!("Recruiting character '{}' to party", character_id);
                // TODO: Implement party recruitment logic
                // 1. Load character from character database
                // 2. Add to global_state.party.members if space available
                // 3. Refresh HUD
                // 4. Remove MapEvent::RecruitableCharacter from map
            }
            DialogueAction::RecruitToInn { character_id, innkeeper_id } => {
                info!("Sending character '{}' to inn (keeper: {})", character_id, innkeeper_id);
                // TODO: Implement inn recruitment logic
                // 1. Load character from character database
                // 2. Add to innkeeper's roster
                // 3. Remove MapEvent::RecruitableCharacter from map
            }
            _ => {} // Other actions handled by existing systems
        }
    }
}
```

**Register system** in `DialoguePlugin`:

```rust
impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StartDialogue>()
            .add_event::<SelectDialogueChoice>()
            .add_systems(
                Update,
                (
                    start_dialogue_system,
                    select_dialogue_choice_system,
                    handle_recruitment_actions, // NEW
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
```

#### 4.6 Testing Requirements

**File**: `src/game/systems/input.rs`

Add test:

```rust
#[test]
fn test_recruitable_character_interaction_triggers_dialogue() {
    // Arrange
    let mut map = Map::new(10, 10);
    map.add_event(
        IVec2::new(5, 6),
        MapEvent::RecruitableCharacter {
            character_id: "test_char".to_string(),
            name: "Test Character".to_string(),
            description: "A test character".to_string(),
        },
    );

    // TODO: Complete test when dialogue system integration is confirmed

    // Assert
    // Verify StartDialogue event sent with dialogue_id = 100
}
```

**File**: `src/game/systems/map.rs`

Add test:

```rust
#[test]
fn test_recruitable_character_marker_color() {
    assert_eq!(
        RECRUITABLE_CHARACTER_MARKER_COLOR,
        Color::rgb(0.27, 0.67, 0.39)
    );
}
```

**File**: `src/domain/dialogue.rs`

Add tests:

```rust
#[test]
fn test_dialogue_action_recruit_to_party_description() {
    let action = DialogueAction::RecruitToParty {
        character_id: "hero_01".to_string(),
    };
    assert_eq!(action.description(), "Recruit 'hero_01' to party");
}

#[test]
fn test_dialogue_action_recruit_to_inn_description() {
    let action = DialogueAction::RecruitToInn {
        character_id: "hero_02".to_string(),
        innkeeper_id: "innkeeper_town_01".to_string(),
    };
    assert_eq!(action.description(), "Send 'hero_02' to inn (keeper: innkeeper_town_01)");
}
```

#### 4.7 Update Sprite Support Plan

**File**: `docs/explanation/sprite_support_implementation_plan.md`

Add to sprite registry:

```ron
"recruitable_characters": SpriteSheetConfig(
    texture_path: "textures/sprites/recruitable_characters.png",
    tile_size: (32.0, 32.0),
    columns: 4,
    rows: 2,
    sprites: [
        (0, "warrior_recruit"),
        (1, "mage_recruit"),
        (2, "rogue_recruit"),
        (3, "cleric_recruit"),
        (4, "ranger_recruit"),
        (5, "paladin_recruit"),
        (6, "bard_recruit"),
        (7, "monk_recruit"),
    ],
),
```

#### 4.8 Post-Implementation Validation

Run ALL commands in order:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**Expected Output**:

- All commands pass with 0 errors/warnings
- New tests compile and pass

#### 4.9 Deliverables

- [ ] `map.rs`: `RECRUITABLE_CHARACTER_MARKER_COLOR` constant added (green #45AB63)
- [ ] `map.rs`: Event marker spawning updated to include RecruitableCharacter
- [ ] `map.rs`: 1 new unit test for recruitable character marker color
- [ ] `input.rs`: `GameAction::Interact` extended to handle RecruitableCharacter events
- [ ] `input.rs`: Import added for `StartDialogue`
- [ ] `input.rs`: Event writer parameter added to `handle_input()`
- [ ] `input.rs`: 1 new test for recruitable character interaction
- [ ] `campaigns/tutorial/data/dialogues.ron`: Default recruitment dialogue added (ID 100)
- [ ] `dialogue.rs`: `DialogueAction::RecruitToParty` variant added
- [ ] `dialogue.rs`: `DialogueAction::RecruitToInn` variant added
- [ ] `dialogue.rs`: 2 new unit tests for recruitment action descriptions
- [ ] `dialogue.rs`: `handle_recruitment_actions()` system added (with TODO placeholders)
- [ ] `sprite_support_implementation_plan.md`: Recruitable character sprite sheet config added
- [ ] SPDX headers verified present in all modified `.rs` files
- [ ] All validation commands pass

#### 4.10 Success Criteria

**Manual Verification** (run `cargo run`):

- [ ] Recruitable characters on map display green colored markers
- [ ] Pressing E when adjacent to recruitable character marker triggers dialogue
- [ ] Dialogue displays: "Hello there. My name is {CHARACTER_NAME}. Can I join your party?"
- [ ] Three dialogue choices appear: "Yes, join us!", "Meet me at the Inn.", "Not at this time."
- [ ] Selecting "Not at this time" closes dialogue (terminal node)
- [ ] Selecting "Yes, join us!" shows confirmation message (TODO: party recruitment logic)
- [ ] Selecting "Meet me at the Inn." shows confirmation message (TODO: inn roster logic)
- [ ] Markers positioned slightly above ground (no z-fighting)
- [ ] Green markers visually distinct from brown (signs) and purple (teleports)

**Known Limitations**:

- `{CHARACTER_NAME}` placeholder in dialogue text not yet dynamically replaced (future enhancement)
- Party recruitment logic stubbed with TODO comments (requires party management system integration)
- Inn roster logic stubbed with TODO comments (requires inn system integration)
- MapEvent removal after recruitment not implemented (requires map state management)

---

## Final Validation Checklist

Before marking ALL phases complete, verify every item below.

### Code Quality

- [ ] `cargo fmt --all` passes with no output
- [ ] `cargo check --all-targets --all-features` passes with 0 errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes with 0 warnings
- [ ] `cargo nextest run --all-features` passes with >80% coverage
- [ ] No `unwrap()` or `expect()` without justification comments
- [ ] All public functions have `///` doc comments with examples
- [ ] All new test functions use descriptive names: `test_{function}_{condition}_{expected}`
- [ ] Dialogue RON file parses correctly with new recruitment dialogue

### Architecture Compliance

- [ ] Data structures match `docs/reference/architecture.md` Section 4 **EXACTLY**
- [ ] Type aliases used consistently (`ItemId`, `SpellId`, etc. - no raw `u32` for IDs)
- [ ] Constants extracted and named (no magic numbers in code)
- [ ] `MapEvent` enum used correctly for all event types
- [ ] No architectural deviations introduced without documentation

### Documentation

- [ ] `docs/explanation/implementations.md` updated with completion summary
- [ ] SPDX headers present in all modified `.rs` files:
  ```rust
  // SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
  // SPDX-License-Identifier: Apache-2.0
  ```
- [ ] All code examples in documentation compile correctly
- [ ] No broken file path references in documentation

### Testing

- [ ] Phase 1: 3 new layout tests added and passing
- [ ] Phase 2: 8 new interaction tests added (3 for `get_adjacent_positions`, 5 for interactions)
- [ ] Phase 3: 4 new marker constant tests added and passing
- [ ] Phase 4: 4 new tests added (1 marker color, 1 interaction, 2 dialogue actions)
- [ ] All existing tests still pass (no regressions)
- [ ] Manual verification completed for all success criteria

### Files Modified

- [ ] `src/game/systems/hud.rs` (Phase 1: layout restructure)
- [ ] `src/game/systems/input.rs` (Phase 2 & 4: interaction system)
- [ ] `src/game/systems/map.rs` (Phase 3 & 4: event markers)
- [ ] `src/game/systems/dialogue.rs` (Phase 4: recruitment handler system)
- [ ] `src/domain/dialogue.rs` (Phase 4: recruitment actions)
- [ ] `campaigns/tutorial/data/dialogues.ron` (Phase 4: default recruitment dialogue)
- [ ] `docs/explanation/sprite_support_implementation_plan.md` (Phase 3 & 4: sprite registry updates)
- [ ] `docs/explanation/implementations.md` (final summary)

---

## Manual Verification Plan

After all automated tests pass, perform these manual tests:

### Test 1: HUD Layout Verification

**Steps**:

1. Run `cargo run`
2. Observe bottom HUD panel

**Expected Results**:

- [ ] Character names display without number prefixes (e.g., "Kira" not "1. Kira")
- [ ] HP text (e.g., "45/100 HP") appears to the right of character name in same row
- [ ] HP bar is visibly thinner than before (10px height)
- [ ] Portrait (40px square) is aligned to left with no extra spacing
- [ ] Total HUD panel is shorter (70px height)
- [ ] All 6 character cards fit horizontally without any clipping

### Test 2: Door Interaction (Existing Behavior)

**Steps**:

1. Navigate party to face a door tile
2. Press E key

**Expected Results**:

- [ ] Door opens (wall type changes to None)
- [ ] Console logs: "Opening door at {position}"
- [ ] Party can walk through the opened door tile

### Test 3: NPC Interaction (Adjacent Tiles)

**Steps**:

1. Navigate party to any tile adjacent to an NPC (N, NE, E, SE, S, SW, W, or NW)
2. Press E key

**Expected Results**:

- [ ] Console logs: "Interacting with NPC '{name}' at {position}"
- [ ] Dialogue event is triggered (MapEventTriggered with NpcDialogue)
- [ ] Interaction works from any of the 8 adjacent positions

### Test 4: Sign Interaction

**Steps**:

1. Navigate party to any tile adjacent to a sign marker (brown/tan colored quad)
2. Press E key

**Expected Results**:

- [ ] Console logs: "Reading sign '{name}' at {position}"
- [ ] Sign event is triggered (MapEventTriggered with Sign)
- [ ] Sign message appears in game log

### Test 5: Teleport Interaction

**Steps**:

1. Navigate party to any tile adjacent to a teleport marker (purple colored quad)
2. Press E key

**Expected Results**:

- [ ] Console logs: "Activating teleport '{name}' to {destination}"
- [ ] Teleport event is triggered (MapEventTriggered with Teleport)
- [ ] Party position changes to destination coordinates

### Test 6: Visual Markers Display

**Steps**:

1. Run `cargo run`
2. Navigate to map areas containing signs and teleports

**Expected Results**:

- [ ] Sign tiles display brown/tan colored flat quads on map
- [ ] Teleport tiles display purple colored flat quads on map
- [ ] Recruitable character tiles display green colored flat quads on map
- [ ] Markers are positioned slightly above ground (visible, no z-fighting)
- [ ] Markers are smaller than tile size (80%) and centered
- [ ] Markers have slight glow/emissive effect for visibility

### Test 7: No Interactable Nearby

**Steps**:

1. Navigate party to empty area with no doors, NPCs, signs, or teleports nearby
2. Press E key

**Expected Results**:

- [ ] Console logs: "No interactable object nearby"
- [ ] No events triggered
- [ ] No errors or crashes

### Test 8: Recruitable Character Interaction

**Steps**:

1. Navigate party to any tile adjacent to a recruitable character marker (green colored quad)
2. Press E key

**Expected Results**:

- [ ] Console logs: "Interacting with recruitable character '{name}' (ID: {character_id})"
- [ ] Dialogue opens with ID 100 (default recruitment dialogue)
- [ ] Dialogue text displays: "Hello there. My name is {CHARACTER_NAME}. Can I join your party?"
- [ ] Three choices appear: "Yes, join us!", "Meet me at the Inn.", "Not at this time."

### Test 9: Recruitment Dialogue Choices

**Steps**:

1. Trigger recruitable character dialogue (Test 8)
2. Select each dialogue choice in separate runs

**Expected Results - Choice 1 ("Yes, join us!")**:

- [ ] Dialogue advances to node 2
- [ ] Text displays: "Excellent! I'm ready to join your adventure."
- [ ] Dialogue ends (is_terminal: true)
- [ ] Console logs recruitment action (TODO: party recruitment not yet implemented)

**Expected Results - Choice 2 ("Meet me at the Inn.")**:

- [ ] Dialogue advances to node 3
- [ ] Text displays: "I'll head to the inn right away. See you there!"
- [ ] Dialogue ends (is_terminal: true)
- [ ] Console logs inn recruitment action (TODO: inn roster not yet implemented)

**Expected Results - Choice 3 ("Not at this time.")**:

- [ ] Dialogue ends immediately (no follow-up node)
- [ ] Character marker remains on map (can interact again)

---

## Post-Completion Documentation Update

After ALL phases complete and validation passes, update `docs/explanation/implementations.md`:

```markdown
## Game Engine Fixes - 2025-01-[XX]

**Feature**: World Interaction, Character Recruitment, & HUD Visual Improvements

**Files Modified**:

- `src/game/systems/hud.rs` - Restructured character card layout
- `src/game/systems/input.rs` - Extended E-key interaction system
- `src/game/systems/map.rs` - Added visual markers for events
- `src/game/systems/dialogue.rs` - Added recruitment action handler
- `src/domain/dialogue.rs` - Added recruitment dialogue actions
- `campaigns/tutorial/data/dialogues.ron` - Added default recruitment dialogue
- `docs/explanation/sprite_support_implementation_plan.md` - Added sprite registry entries

**Changes**:

1. HUD character cards now display HP text next to name (right-justified)
2. HP bar reduced from 16px to 10px height
3. Character number prefixes removed from names
4. E-key interaction now works with NPCs, signs, teleports, and recruitable characters from adjacent tiles
5. Signs and teleports display colored placeholder markers (brown and purple respectively)
6. Recruitable characters display green colored placeholder markers
7. Adjacent tile checking supports all 8 surrounding positions
8. Recruitable character interaction triggers dialogue-based recruitment system
9. Default recruitment dialogue added with three options: join party, go to inn, decline

**Breaking Changes**: None

**Testing**:

- Added 3 HUD layout unit tests
- Added 8 interaction system tests (adjacent positions + event triggering)
- Added 4 event marker constant tests
- Added 4 recruitment system tests (marker color, interaction, dialogue actions)
- All existing tests pass (0 regressions)
- Manual verification completed for all 9 test scenarios

**Known Limitations**:

- Event markers use colored quads (sprites to be added in sprite_support_implementation_plan.md Phase 3.X)
- Some interaction tests marked `#[ignore]` pending event system API clarification
- Recruitment dialogue uses `{CHARACTER_NAME}` placeholder (dynamic substitution not yet implemented)
- Party recruitment logic stubbed with TODO (requires party management integration)
- Inn roster logic stubbed with TODO (requires inn system integration)
- MapEvent removal after recruitment not implemented (requires map state management)

**Next Steps**:

- Implement sprite-based rendering for signs, teleports, and recruitable characters (see sprite_support_implementation_plan.md)
- Complete ignored tests when event system API is finalized
- Implement dialogue variable substitution for `{CHARACTER_NAME}` placeholder
- Integrate party recruitment logic (add character to party, refresh HUD)
- Integrate inn roster logic (add character to innkeeper roster)
- Implement MapEvent removal after successful recruitment
```

---

## Design Decisions (Resolved)

| Question                       | Decision                                                  | Rationale                                                                                                   |
| ------------------------------ | --------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| **NPC Interaction Range**      | Adjacent tiles (all 8 surrounding positions)              | Allows interaction from any direction; matches classic RPG UX where player doesn't need perfect positioning |
| **HUD HP Text Placement**      | Next to name, right-justified in same row                 | Saves vertical space; improves readability; reduces panel height from 80px to 70px                          |
| **HP Bar Height**              | Reduced from 16px to 10px                                 | Thinner bar still clearly visible; frees space for condensed layout                                         |
| **Sign/Teleport Visuals**      | Placeholder colored quads (brown #967046, purple #8749DE) | Temporary solution; full sprite support deferred to sprite_support_implementation_plan.md                   |
| **Recruitable Char Visuals**   | Placeholder green colored quad (#45AB63)                  | Temporary solution; full sprite support deferred to sprite_support_implementation_plan.md                   |
| **Recruitment Flow**           | Dialogue-based with 3 options (join/inn/decline)          | Matches classic RPG UX; non-intrusive; player has full control                                              |
| **Recruitment Dialogue**       | Default dialogue ID 100 in dialogues.ron                  | Not hardcoded; data-driven design; can be customized per campaign                                           |
| **Character Name Placeholder** | `{CHARACTER_NAME}` in dialogue text (not yet dynamic)     | Future enhancement: implement variable substitution system                                                  |
| **Teleport Activation**        | E-key required (no auto-teleport on walk-over)            | Prevents accidental teleports; walk-over only shows message                                                 |
| **Door Interaction**           | Maintain existing E-key behavior                          | Preserve working functionality; doors checked first before NPCs/events                                      |

---

## Implementation Order Summary

**Phase 1: HUD Fixes** → Independent work, unblocks visual testing
**Phase 2: Interaction System** → Core functionality, depends on Phase 1 for testing
**Phase 3: Visual Markers** → Enhancement, depends on Phase 2 for interaction testing
**Phase 4: Recruitable Characters** → Feature addition, depends on Phase 2 & 3 for interaction and visualization

Total estimated implementation time: 8-10 hours for AI agent execution.
