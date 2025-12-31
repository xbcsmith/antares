# NPC Blocking Example

This example demonstrates the NPC blocking functionality implemented in Phase 1 of the NPC Gameplay Fix Implementation Plan.

## Overview

The example shows how NPCs now properly block player movement, preventing the party from walking through them. This is a critical gameplay mechanic that makes NPCs feel solid and present in the game world.

## What This Example Demonstrates

1. **New NPC Placement System**: Using `NpcPlacement` to reference NPCs by ID
2. **NPC Blocking**: NPCs at specific positions block movement
3. **Backward Compatibility**: Legacy inline NPCs also block movement
4. **Integration with Tile Blocking**: NPCs work alongside walls and terrain
5. **Boundary Checking**: Out-of-bounds positions are properly blocked

## Running the Example

```bash
cargo run --example npc_blocking_example
```

## Example Output

The example will:

1. Create a 10x10 test map
2. Add walls at positions (5,0) and (5,1)
3. Place NPCs using the new placement system:
   - Guard at (3,3) facing North
   - Merchant at (7,7) facing South
4. Add a legacy NPC (Village Elder) at (1,1)
5. Test blocking at various positions
6. Simulate party movement with blocking checks
7. Display a visual map with NPCs marked

### Sample Output

```
=== NPC Blocking System Example ===

📍 Map created: Test Town (10x10)

🧱 Added walls at (5,0) and (5,1)

👤 Adding NPCs using new placement system:
  - Guard at Position { x: 3, y: 3 } (facing North)
  - Merchant at Position { x: 7, y: 7 } (facing South)

👤 Adding legacy NPC (backward compatibility):
  - Village Elder at Position { x: 1, y: 1 }

🚶 Testing Movement Blocking:

✅ Position { x: 0, y: 0 } - Empty ground - WALKABLE
🚫 Position { x: 5, y: 0 } - Wall tile - BLOCKED
🚫 Position { x: 3, y: 3 } - Guard NPC (new placement) - BLOCKED
🚫 Position { x: 7, y: 7 } - Merchant NPC (new placement) - BLOCKED
🚫 Position { x: 1, y: 1 } - Village Elder (legacy NPC) - BLOCKED
✅ Position { x: 2, y: 2 } - Empty ground near NPCs - WALKABLE
```

### Map Visualization

The example displays an ASCII map showing:

- `.` = Walkable ground
- `#` = Wall (blocked)
- `N` = NPC placement (blocked)
- `E` = Elder / legacy NPC (blocked)
- `P` = Party position

```
. . . . . # . . . .
. E . . . # . . . .
. . . . . . . . . .
. . . N . . . . . .
. . . . . . . . . .
. . . . . . . . . .
. . . . . . . P . .
. . . . . . . N . .
. . . . . . . . . .
. . . . . . . . . .
```

## Key Concepts

### NPC Placement System

NPCs are now referenced by ID rather than embedded inline:

```rust
map.npc_placements.push(NpcPlacement {
    npc_id: "guard_1".to_string(),
    position: Position::new(3, 3),
    facing: Some(Direction::North),
    dialogue_override: None,
});
```

### Blocking Check

The `Map::is_blocked()` method now checks three sources:

1. **Tile Blocking**: Walls, mountains, water
2. **NPC Placements**: New placement system
3. **Legacy NPCs**: Backward compatibility

```rust
if map.is_blocked(target_position) {
    // Cannot move - position is blocked
} else {
    // Can move - position is walkable
}
```

### Backward Compatibility

The example demonstrates that legacy NPCs (inline in map data) continue to work:

```rust
map.npcs.push(Npc::new(
    1,
    "Village Elder".to_string(),
    "The wise leader".to_string(),
    Position::new(1, 1),
    "Welcome, traveler!".to_string(),
));
```

## Code Structure

- **`main()`**: Creates map, adds NPCs, tests blocking, displays results
- **`print_map_with_npcs()`**: Visualizes the map with ASCII art
- **Tests**: Unit tests verify the example's core functionality

## Testing

Run the example's tests:

```bash
cargo test --example npc_blocking_example
```

Tests verify:
- NPC placements block movement
- Legacy NPCs block movement
- Adjacent positions remain walkable

## Related Documentation

- [Phase 1 Implementation Summary](../docs/explanation/phase1_npc_blocking_implementation_summary.md)
- [NPC Gameplay Fix Plan](../docs/explanation/npc_gameplay_fix_implementation_plan.md)
- [Architecture Reference](../docs/reference/architecture.md)

## Next Steps

This example focuses on Phase 1 (blocking). Future phases will add:

- **Phase 2**: Visual representation of NPCs with sprites
- **Phase 3**: Dialogue interaction when talking to NPCs

## Troubleshooting

### NPCs Not Blocking

If NPCs don't block movement, verify:

1. NPCs are added to `map.npc_placements` or `map.npcs`
2. Position coordinates are within map bounds
3. `Map::is_blocked()` is called before movement

### Map Not Displaying

Ensure the map dimensions are set correctly and all positions are valid.

## Contributing

When modifying this example:

1. Keep it simple and focused on blocking mechanics
2. Add comments explaining non-obvious behavior
3. Update this README if adding new features
4. Run `cargo fmt` and `cargo clippy` before committing

---

**Last Updated**: 2025-01-26
**Phase**: 1 - NPC Externalization & Blocking
**Status**: ✅ Complete
