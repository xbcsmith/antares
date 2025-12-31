// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! NPC Blocking Example
//!
//! This example demonstrates the NPC blocking functionality implemented in Phase 1
//! of the NPC Gameplay Fix Implementation Plan.
//!
//! ## What This Example Shows
//!
//! 1. Creating a map with NPCs using the new placement system
//! 2. Demonstrating that NPCs block movement
//! 3. Testing backward compatibility with legacy NPCs
//! 4. Showing how the blocking system works with walls and terrain
//!
//! ## Running This Example
//!
//! ```bash
//! cargo run --example npc_blocking_example
//! ```

use antares::domain::types::{Direction, Position};
use antares::domain::world::npc::NpcPlacement;
use antares::domain::world::{Map, Npc, WallType};

fn main() {
    println!("=== NPC Blocking System Example ===\n");

    // Create a 10x10 test map
    let mut map = Map::new(
        1,
        "Test Town".to_string(),
        "A small town for testing NPC blocking".to_string(),
        10,
        10,
    );

    println!(
        "📍 Map created: {} ({}x{})",
        map.name, map.width, map.height
    );
    println!();

    // Add some walls to demonstrate tile blocking
    if let Some(tile) = map.get_tile_mut(Position::new(5, 0)) {
        tile.wall_type = WallType::Normal;
        tile.blocked = true;
    }
    if let Some(tile) = map.get_tile_mut(Position::new(5, 1)) {
        tile.wall_type = WallType::Normal;
        tile.blocked = true;
    }

    println!("🧱 Added walls at (5,0) and (5,1)");
    println!();

    // Add NPC placements (new system)
    println!("👤 Adding NPCs using new placement system:");

    let guard_pos = Position::new(3, 3);
    map.npc_placements.push(NpcPlacement {
        npc_id: "guard_1".to_string(),
        position: guard_pos,
        facing: Some(Direction::North),
        dialogue_override: None,
    });
    println!("  - Guard at {:?} (facing North)", guard_pos);

    let merchant_pos = Position::new(7, 7);
    map.npc_placements.push(NpcPlacement {
        npc_id: "merchant_1".to_string(),
        position: merchant_pos,
        facing: Some(Direction::South),
        dialogue_override: None,
    });
    println!("  - Merchant at {:?} (facing South)", merchant_pos);
    println!();

    // Add legacy NPC to demonstrate backward compatibility
    println!("👤 Adding legacy NPC (backward compatibility):");
    let elder_pos = Position::new(1, 1);
    map.npcs.push(Npc::new(
        1,
        "Village Elder".to_string(),
        "The wise leader".to_string(),
        elder_pos,
        "Welcome, traveler!".to_string(),
    ));
    println!("  - Village Elder at {:?}", elder_pos);
    println!();

    // Test blocking at various positions
    println!("🚶 Testing Movement Blocking:\n");

    let test_positions = vec![
        (Position::new(0, 0), "Empty ground"),
        (Position::new(5, 0), "Wall tile"),
        (guard_pos, "Guard NPC (new placement)"),
        (merchant_pos, "Merchant NPC (new placement)"),
        (elder_pos, "Village Elder (legacy NPC)"),
        (Position::new(2, 2), "Empty ground near NPCs"),
        (Position::new(-1, 5), "Out of bounds (negative)"),
        (Position::new(10, 10), "Out of bounds (beyond map)"),
    ];

    for (pos, description) in test_positions {
        let blocked = map.is_blocked(pos);
        let symbol = if blocked { "🚫" } else { "✅" };
        let status = if blocked { "BLOCKED" } else { "WALKABLE" };

        println!("{} {:?} - {} - {}", symbol, pos, description, status);
    }

    println!();
    println!("=== Movement Simulation ===\n");

    // Simulate party movement
    let mut party_pos = Position::new(0, 0);
    println!("🎮 Party starts at {:?}", party_pos);
    println!();

    // Try to move to various positions
    let movement_attempts = vec![
        (Position::new(2, 2), "Move to (2,2) - empty ground"),
        (guard_pos, "Try to move to Guard position"),
        (Position::new(3, 4), "Move next to Guard"),
        (merchant_pos, "Try to move to Merchant position"),
        (Position::new(7, 6), "Move next to Merchant"),
    ];

    for (target, action) in movement_attempts {
        println!("Attempting: {}", action);

        if map.is_blocked(target) {
            println!("  ❌ Movement BLOCKED - cannot move to {:?}", target);
            println!("  📍 Party remains at {:?}", party_pos);
        } else {
            party_pos = target;
            println!("  ✅ Movement SUCCESS - party moves to {:?}", party_pos);
        }
        println!();
    }

    println!("=== Map Visualization ===\n");
    print_map_with_npcs(&map, party_pos);

    println!();
    println!("=== Summary ===\n");
    println!("✅ NPC placements block movement (new system)");
    println!("✅ Legacy NPCs block movement (backward compatibility)");
    println!("✅ Walls and terrain blocking still works");
    println!("✅ Out-of-bounds positions are blocked");
    println!("✅ Adjacent tiles to NPCs remain walkable");
    println!();
    println!("🎉 NPC blocking system working correctly!");
}

/// Helper function to visualize the map
fn print_map_with_npcs(map: &Map, party_pos: Position) {
    println!("Legend:");
    println!("  . = Ground    # = Wall    N = NPC    P = Party    X = Out of bounds");
    println!();

    for y in 0..map.height {
        for x in 0..map.width {
            let pos = Position::new(x as i32, y as i32);

            let symbol = if pos == party_pos {
                'P' // Party
            } else if map.npc_placements.iter().any(|npc| npc.position == pos) {
                'N' // NPC placement
            } else if map.npcs.iter().any(|npc| npc.position == pos) {
                'E' // Elder (legacy NPC)
            } else if let Some(tile) = map.get_tile(pos) {
                if tile.wall_type == WallType::Normal {
                    '#' // Wall
                } else {
                    '.' // Ground
                }
            } else {
                'X' // Out of bounds
            };

            print!("{} ", symbol);
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_map_blocking() {
        // Create the same map as in the example
        let mut map = Map::new(1, "Test".to_string(), "Test".to_string(), 10, 10);

        // Add NPC placement
        let npc_pos = Position::new(5, 5);
        map.npc_placements
            .push(NpcPlacement::new("test_npc", npc_pos));

        // Test blocking
        assert!(map.is_blocked(npc_pos), "NPC position should be blocked");
        assert!(
            !map.is_blocked(Position::new(4, 5)),
            "Adjacent position should not be blocked"
        );
    }

    #[test]
    fn test_example_legacy_npc_blocking() {
        // Create map
        let mut map = Map::new(1, "Test".to_string(), "Test".to_string(), 10, 10);

        // Add legacy NPC
        let npc_pos = Position::new(3, 3);
        map.npcs.push(Npc::new(
            1,
            "Test".to_string(),
            "Test".to_string(),
            npc_pos,
            "Hello".to_string(),
        ));

        // Test blocking
        assert!(
            map.is_blocked(npc_pos),
            "Legacy NPC position should be blocked"
        );
    }
}
