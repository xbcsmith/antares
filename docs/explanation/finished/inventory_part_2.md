# Inventory Part 2

Write a plan with a phased approach to create an inventory view in the game for characters. Follow the rules in @PLAN.md

## Overview

Bind the default open inventory key to "i" and make it configurable in the Game Configuration. Once the inventory screen should only take up a section of hte game window above the characters hud portrait. We should be able to TAB through the characters to select them and open each inventory for each character. I should be able to have 6 inventorie open at one time. I should also be able to have one inventory open and tab through each inventory. I should be able to select items in the inventory to "drop" or send to another character or NPC that has inventory capability.

An RPG inventory system is typically implemented using the engine's Entity Component System (ECS) architecture, where items and inventories are represented as entities with specific components and systems manage the logic.

We will implement a Surface ECS.

Surface ECS: wrap state in Components (Medium effort)

Add `#[derive(Component)]` to inventory-related structs and give each character a Bevy `Entity`. The domain types (`Inventory`, `InventorySlot`, `MerchantStock`, etc.) stay structurally identical; they just become components instead of fields nested inside `Character` inside `Party` inside `GlobalState`.

Application layer (`src/application/mod.rs`) — one `NpcRuntimeStore` field

`GameState` holds `npc_runtime: NpcRuntimeStore` which is a `HashMap<NpcId, NpcRuntimeState>`. Under ECS, this would become a `Resource` (it already effectively is one via `GlobalState`). `ensure_npc_runtime_initialized()` would become a startup `System`. The change here is small either way.

## Core Concepts

Items as Entities: Each individual item (e.g., a "Steel Sword" or a "Health Potion") is its own entity in the Bevy world, with components defining its properties like ItemType, ItemQuantity, ItemDurability, etc..

Inventory as a Component: An inventory is a component attached to a player, chest, or monster entity. It typically uses a data structure like a HashMap to map slot indices to item entities.

Events and Commands: Actions such as picking up, dropping, moving, or using an item are handled using Bevy's event system or entity commands.

Systems Logic: Systems contain the logic for managing the inventory, such as checking if a slot is valid for an item, managing stack sizes, and updating the UI.

## Implementation Approaches

Developers can choose between building a custom system or using an existing crate:

## Custom Implementation

Defining Data: Using Rust enums and structs with #[derive(Component)] to define item classifications (e.g., Consumable, Equipment) and data.

Managing Slots: Implementing logic for inventory slots, equipment slots, and ensuring items fit within container dimensions (e.g., a grid-based inventory).

UI Integration: Creating a UI that dynamically reflects the state of the inventory components, a common topic in Bevy tutorials.
