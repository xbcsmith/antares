// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Map Editor GUI Component
//!
//! This module provides an embeddable egui-based map editor for the Campaign Builder.
//!
//! # Features
//!
//! - Visual tile editing with grid display
//! - Event placement and management
//! - NPC placement with dialogue editor
//! - Map metadata editing (name, size, connections)
//! - Visual preview with color-coded tiles
//! - Undo/redo support
//! - Real-time validation feedback
//! - Standard editor pattern with EditorToolbar, TwoColumnLayout, ActionButtons
//!
//! # Architecture
//!
//! The map editor follows the standard SDK editor pattern:
//! - `MapsEditorState`: Main editor state with `show()` method
//! - `MapEditorState`: Per-map editing state (pure logic, no UI)
//! - `MapGridWidget`: egui rendering component for the tile grid
//! - Tool palette for tile/event/NPC placement
//!
//! # Usage
//!
//! ```rust,no_run
//! use antares::domain::world::Map;
//! use campaign_builder::map_editor::MapsEditorState;
//!
//! let mut state = MapsEditorState::new();
//!
//! // In egui update loop (delegated from main.rs):
//! // state.show(ui, &mut maps, campaign_dir, ...);
//! ```

use crate::ui_helpers::{
    searchable_selector_multi, ActionButtons, EditorToolbar, ItemAction, ToolbarAction,
    TwoColumnLayout,
};
use antares::domain::combat::database::MonsterDefinition;
use antares::domain::items::types::Item;
use antares::domain::types::{EventId, ItemId, MapId, MonsterId, Position};
use antares::domain::world::{Map, MapEvent, Npc, TerrainType, Tile, WallType};
use antares::sdk::tool_config::DisplayConfig;
use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use std::fs;
use std::path::PathBuf;

// ===== Editor Mode =====

/// Editor mode for the maps editor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapsEditorMode {
    /// List view - showing map list and detail preview
    List,
    /// Add mode - creating a new map
    Add,
    /// Edit mode - editing an existing map with the full editor
    Edit,
}

// ===== Tool Types =====

/// Editing tool mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTool {
    /// Select and inspect tiles
    Select,
    /// Paint tiles with selected terrain and wall
    PaintTile,
    /// Place events
    PlaceEvent,
    /// Place NPCs
    PlaceNpc,
    /// Fill region (bucket fill)
    Fill,
    /// Erase (reset to default)
    Erase,
}

impl EditorTool {
    /// Returns the tool name
    pub fn name(&self) -> &str {
        match self {
            EditorTool::Select => "Select",
            EditorTool::PaintTile => "Paint Tile",
            EditorTool::PlaceEvent => "Place Event",
            EditorTool::PlaceNpc => "Place NPC",
            EditorTool::Fill => "Fill",
            EditorTool::Erase => "Erase",
        }
    }

    /// Returns the tool icon
    pub fn icon(&self) -> &str {
        match self {
            EditorTool::Select => "👆",
            EditorTool::PaintTile => "🖌️",
            EditorTool::PlaceEvent => "🎯",
            EditorTool::PlaceNpc => "🧙",
            EditorTool::Fill => "🪣",
            EditorTool::Erase => "🧹",
        }
    }

    /// Returns all editor tools for iteration
    pub fn all() -> &'static [EditorTool] {
        &[
            EditorTool::Select,
            EditorTool::PaintTile,
            EditorTool::PlaceEvent,
            EditorTool::PlaceNpc,
            EditorTool::Fill,
            EditorTool::Erase,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomAction {
    In,
    Out,
    Fit,
    Reset,
}

// ===== Undo/Redo System =====

/// Action that can be undone/redone
#[derive(Debug, Clone)]
enum EditorAction {
    /// Tile was modified
    TileChanged {
        position: Position,
        old_tile: Tile,
        new_tile: Tile,
    },
    /// Event was added
    EventAdded {
        position: Position,
        event: MapEvent,
        event_id: Option<EventId>,
    },
    /// Event was removed
    EventRemoved {
        position: Position,
        event: MapEvent,
        event_id: Option<EventId>,
    },
    /// NPC was added
    NpcAdded { npc: Npc },
    /// NPC was removed
    NpcRemoved { index: usize, npc: Npc },
}

/// Undo/redo stack
#[derive(Debug, Clone)]
struct UndoStack {
    actions: Vec<EditorAction>,
    current: usize,
}

impl UndoStack {
    fn new() -> Self {
        Self {
            actions: Vec::new(),
            current: 0,
        }
    }

    fn push(&mut self, action: EditorAction) {
        // Remove any actions after current position
        self.actions.truncate(self.current);
        self.actions.push(action);
        self.current = self.actions.len();
    }

    fn can_undo(&self) -> bool {
        self.current > 0
    }

    fn can_redo(&self) -> bool {
        self.current < self.actions.len()
    }

    fn undo(&mut self) -> Option<EditorAction> {
        if self.can_undo() {
            self.current -= 1;
            Some(self.actions[self.current].clone())
        } else {
            None
        }
    }

    fn redo(&mut self) -> Option<EditorAction> {
        if self.can_redo() {
            let action = self.actions[self.current].clone();
            self.current += 1;
            Some(action)
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.actions.clear();
        self.current = 0;
    }
}

// ===== Map Metadata =====

/// Extended map metadata for editing
#[derive(Debug, Clone)]
pub struct MapMetadata {
    /// Map name (user-friendly)
    pub name: String,
    /// Map description
    pub description: String,
    /// Difficulty level (1-10)
    pub difficulty: u8,
    /// Is this an outdoor map?
    pub is_outdoor: bool,
    /// Connected map IDs and transition positions
    pub connections: Vec<MapConnection>,
    /// Light level (0-100, where 0 is pitch black)
    pub light_level: u8,
    /// Music track identifier
    pub music_track: String,
    /// Random encounter rate (0-100)
    pub encounter_rate: u8,
}

impl Default for MapMetadata {
    fn default() -> Self {
        Self {
            name: "New Map".to_string(),
            description: String::new(),
            difficulty: 1,
            is_outdoor: false,
            connections: Vec::new(),
            light_level: 100,
            music_track: String::new(),
            encounter_rate: 10,
        }
    }
}

/// Connection to another map
#[derive(Debug, Clone)]
pub struct MapConnection {
    /// Position of the transition on this map
    pub from_position: Position,
    /// Target map ID
    pub to_map: MapId,
    /// Position on target map
    pub to_position: Position,
    /// Description (e.g., "Stairs down", "Town entrance")
    pub description: String,
}

// ===== Map Editor State (per-map editing state) =====

/// Map editor state (pure logic, no UI)
pub struct MapEditorState {
    /// The map being edited
    pub map: Map,
    /// Extended metadata
    pub metadata: MapMetadata,
    /// Current editing tool
    pub current_tool: EditorTool,
    /// Selected position (for inspector)
    pub selected_position: Option<Position>,
    /// Currently selected terrain type for painting
    pub selected_terrain: TerrainType,
    /// Currently selected wall type for painting
    pub selected_wall: WallType,
    /// Undo/redo stack
    undo_stack: UndoStack,
    /// Has unsaved changes
    pub has_changes: bool,
    /// File path (if loaded from disk)
    pub file_path: Option<PathBuf>,
    /// Validation errors
    pub validation_errors: Vec<String>,
    /// Grid visibility
    pub show_grid: bool,
    /// Event markers visibility
    pub show_events: bool,
    /// NPC markers visibility
    pub show_npcs: bool,
    /// Auto fit the map to the available area on window resize
    pub auto_fit_on_resize: bool,
    /// Event editor state
    pub event_editor: Option<EventEditorState>,
    /// NPC editor state
    pub npc_editor: Option<NpcEditorState>,
    /// Show metadata editor panel
    pub show_metadata_editor: bool,
}

impl MapEditorState {
    /// Creates a new map editor state
    pub fn new(map: Map) -> Self {
        let metadata = MapMetadata {
            name: map.name.clone(),
            description: map.description.clone(),
            ..Default::default()
        };

        Self {
            map,
            metadata,
            current_tool: EditorTool::Select,
            selected_position: None,
            selected_terrain: TerrainType::Ground,
            selected_wall: WallType::None,
            undo_stack: UndoStack::new(),
            has_changes: false,
            file_path: None,
            validation_errors: Vec::new(),
            show_grid: true,
            show_events: true,
            show_npcs: true,
            auto_fit_on_resize: true,
            event_editor: None,
            npc_editor: None,
            show_metadata_editor: false,
        }
    }

    /// Sets a tile at the specified position
    pub fn set_tile(&mut self, pos: Position, tile: Tile) {
        if !self.map.is_valid_position(pos) {
            return;
        }

        if let Some(old_tile) = self.map.get_tile(pos).cloned() {
            if let Some(tile_mut) = self.map.get_tile_mut(pos) {
                let new_tile = tile.clone();
                *tile_mut = tile;

                self.undo_stack.push(EditorAction::TileChanged {
                    position: pos,
                    old_tile,
                    new_tile,
                });

                self.has_changes = true;
            }
        }
    }

    /// Paints a tile with the currently selected terrain and wall
    pub fn paint_tile(&mut self, pos: Position) {
        if let Some(tile) = self.map.get_tile(pos).cloned() {
            let mut new_tile = tile;
            new_tile.terrain = self.selected_terrain;
            new_tile.wall_type = self.selected_wall;
            new_tile.blocked = matches!(
                self.selected_terrain,
                TerrainType::Mountain | TerrainType::Water
            ) || matches!(self.selected_wall, WallType::Normal);
            self.set_tile(pos, new_tile);
        }
    }

    /// Paints terrain at position (kept for undo compatibility)
    fn paint_terrain(&mut self, pos: Position, terrain: TerrainType) {
        if let Some(tile) = self.map.get_tile(pos).cloned() {
            let mut new_tile = tile;
            new_tile.terrain = terrain;
            new_tile.blocked = matches!(terrain, TerrainType::Mountain | TerrainType::Water)
                || matches!(new_tile.wall_type, WallType::Normal);
            self.set_tile(pos, new_tile);
        }
    }

    /// Paints wall at position (kept for undo compatibility)
    fn paint_wall(&mut self, pos: Position, wall: WallType) {
        if let Some(tile) = self.map.get_tile(pos).cloned() {
            let mut new_tile = tile;
            new_tile.wall_type = wall;
            new_tile.blocked =
                matches!(new_tile.terrain, TerrainType::Mountain | TerrainType::Water)
                    || matches!(wall, WallType::Normal);
            self.set_tile(pos, new_tile);
        }
    }

    /// Fills a rectangular region
    pub fn fill_region(
        &mut self,
        from: Position,
        to: Position,
        terrain: TerrainType,
        wall: WallType,
    ) {
        let min_x = from.x.min(to.x);
        let max_x = from.x.max(to.x);
        let min_y = from.y.min(to.y);
        let max_y = from.y.max(to.y);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let pos = Position::new(x, y);
                if self.map.is_valid_position(pos) {
                    let new_tile = Tile::new(pos.x, pos.y, terrain, wall);
                    self.set_tile(pos, new_tile);
                }
            }
        }
    }

    /// Apply the metadata fields back into the map struct
    ///
    /// Ensures metadata set via the editor (e.g., `metadata.name`) are copied into the
    /// underlying map before saving/export.
    pub fn apply_metadata(&mut self) {
        self.map.name = self.metadata.name.clone();
        self.map.description = self.metadata.description.clone();
    }

    /// Erases a tile (resets to default)
    pub fn erase_tile(&mut self, pos: Position) {
        let default_tile = Tile::new(pos.x, pos.y, TerrainType::Ground, WallType::None);
        self.set_tile(pos, default_tile);
    }

    /// Helper: Get next available EventId for this map (scans existing tile triggers)
    ///
    /// This is an editor-only helper used to auto-assign an EventId onto the tile's
    /// `event_trigger` field without changing the core `Map` domain structure.
    fn next_available_event_id(&self) -> EventId {
        self.map
            .tiles
            .iter()
            .filter_map(|t| t.event_trigger)
            .max()
            .unwrap_or(0)
            + 1
    }

    /// Adds an event at position
    pub fn add_event(&mut self, pos: Position, event: MapEvent) {
        if !self.map.is_valid_position(pos) {
            return;
        }

        // Check for an existing event ID using an immutable borrow before we
        // compute a new one or mutably borrow the tile to set a new ID.
        let existing_id = if let Some(tile) = self.map.get_tile(pos) {
            tile.event_trigger
        } else {
            // invalid pos
            return;
        };

        // If no existing id, compute the next ID (immutably) and then set it via
        // a separate mutable borrow to avoid borrow checker conflicts.
        let event_id = if let Some(id) = existing_id {
            id
        } else {
            let id = self.next_available_event_id();
            if let Some(tile) = self.map.get_tile_mut(pos) {
                tile.event_trigger = Some(id);
            }
            id
        };

        self.map.add_event(pos, event.clone());
        self.undo_stack.push(EditorAction::EventAdded {
            position: pos,
            event,
            event_id: Some(event_id),
        });
        self.has_changes = true;
    }

    /// Removes event at position
    pub fn remove_event(&mut self, pos: Position) {
        // Grab and clear the tile's event ID (if present)
        let event_id = if let Some(tile) = self.map.get_tile_mut(pos) {
            tile.event_trigger.take()
        } else {
            None
        };

        if let Some(event) = self.map.remove_event(pos) {
            self.undo_stack.push(EditorAction::EventRemoved {
                position: pos,
                event,
                event_id,
            });
            self.has_changes = true;
        }
    }

    /// Adds an NPC
    pub fn add_npc(&mut self, npc: Npc) {
        self.map.add_npc(npc.clone());
        self.undo_stack.push(EditorAction::NpcAdded { npc });
        self.has_changes = true;
    }

    /// Removes an NPC by index
    pub fn remove_npc(&mut self, index: usize) {
        if index < self.map.npcs.len() {
            let npc = self.map.npcs.remove(index);
            self.undo_stack
                .push(EditorAction::NpcRemoved { index, npc });
            self.has_changes = true;
        }
    }

    /// Undoes the last action
    pub fn undo(&mut self) {
        if let Some(action) = self.undo_stack.undo() {
            self.apply_undo(action);
            self.has_changes = true;
        }
    }

    /// Redoes the last undone action
    pub fn redo(&mut self) {
        if let Some(action) = self.undo_stack.redo() {
            self.apply_redo(action);
            self.has_changes = true;
        }
    }

    fn apply_undo(&mut self, action: EditorAction) {
        match action {
            EditorAction::TileChanged {
                position, old_tile, ..
            } => {
                if let Some(tile) = self.map.get_tile_mut(position) {
                    *tile = old_tile;
                }
            }
            EditorAction::EventAdded { position, .. } => {
                self.map.remove_event(position);
                if let Some(tile) = self.map.get_tile_mut(position) {
                    tile.event_trigger = None;
                }
            }
            EditorAction::EventRemoved {
                position,
                event,
                event_id,
            } => {
                self.map.add_event(position, event);
                if let Some(id) = event_id {
                    if let Some(tile) = self.map.get_tile_mut(position) {
                        tile.event_trigger = Some(id);
                    }
                }
            }
            EditorAction::NpcAdded { .. } => {
                self.map.npcs.pop();
            }
            EditorAction::NpcRemoved { index, npc } => {
                self.map.npcs.insert(index, npc);
            }
        }
    }

    fn apply_redo(&mut self, action: EditorAction) {
        match action {
            EditorAction::TileChanged {
                position, new_tile, ..
            } => {
                if let Some(tile) = self.map.get_tile_mut(position) {
                    *tile = new_tile;
                }
            }
            EditorAction::EventAdded {
                position,
                event,
                event_id,
            } => {
                self.map.add_event(position, event);
                if let Some(id) = event_id {
                    if let Some(tile) = self.map.get_tile_mut(position) {
                        tile.event_trigger = Some(id);
                    }
                }
            }
            EditorAction::EventRemoved { position, .. } => {
                self.map.remove_event(position);
                if let Some(tile) = self.map.get_tile_mut(position) {
                    tile.event_trigger = None;
                }
            }
            EditorAction::NpcAdded { npc } => {
                self.map.add_npc(npc);
            }
            EditorAction::NpcRemoved { index, .. } => {
                self.map.npcs.remove(index);
            }
        }
    }

    /// Validates the map
    pub fn validate(&mut self) {
        self.validation_errors.clear();

        // Check for disconnected areas (basic check)
        if self.map.events.is_empty() && self.map.npcs.is_empty() {
            self.validation_errors
                .push("Warning: Map has no events or NPCs".to_string());
        }

        // Check for unreachable events
        for (pos, _) in &self.map.events {
            if self.map.is_blocked(*pos) {
                self.validation_errors.push(format!(
                    "Error: Event at ({}, {}) is on a blocked tile",
                    pos.x, pos.y
                ));
            }
        }

        // Check for NPCs on blocked tiles
        for npc in &self.map.npcs {
            if self.map.is_blocked(npc.position) {
                self.validation_errors.push(format!(
                    "Error: NPC '{}' at ({}, {}) is on a blocked tile",
                    npc.name, npc.position.x, npc.position.y
                ));
            }
        }

        // Check connections
        for conn in &self.metadata.connections {
            if !self.map.is_valid_position(conn.from_position) {
                self.validation_errors.push(format!(
                    "Error: Connection '{}' has invalid position",
                    conn.description
                ));
            }
        }
    }

    /// Saves the map to RON format
    pub fn save_to_ron(&self) -> Result<String, String> {
        ron::ser::to_string_pretty(&self.map, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("Failed to serialize map: {}", e))
    }

    /// Can undo?
    pub fn can_undo(&self) -> bool {
        self.undo_stack.can_undo()
    }

    /// Can redo?
    pub fn can_redo(&self) -> bool {
        self.undo_stack.can_redo()
    }

    /// Add an event at the specified position
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate on the map
    /// * `y` - Y coordinate on the map
    /// * `event` - The map event to add
    pub fn add_event_at_position(&mut self, x: u32, y: u32, event: MapEvent) {
        let pos = Position {
            x: x as i32,
            y: y as i32,
        };
        self.add_event(pos, event);
    }

    /// Show event editor UI (for integration with egui)
    pub fn show_event_editor_ui(&mut self) -> bool {
        // Returns true if event editor should be shown
        self.event_editor.is_some()
    }
}

// ===== Event Editor State =====

/// Event editor state
#[derive(Debug, Clone)]
pub struct EventEditorState {
    pub event_type: EventType,
    pub position: Position,
    pub name: String,
    pub description: String,
    // Encounter fields (typed)
    pub encounter_monsters: Vec<MonsterId>,
    // Search query for encounter monsters (persisted across frames)
    pub encounter_monsters_query: String,
    // Treasure fields (typed)
    pub treasure_items: Vec<ItemId>,
    // Search query for treasure items (persisted across frames)
    pub treasure_items_query: String,
    // Teleport fields
    pub teleport_x: String,
    pub teleport_y: String,
    pub teleport_map_id: String,
    // Teleport helper fields
    pub teleport_selected_map: Option<MapId>,
    pub teleport_selected_pos: Option<Position>,
    pub teleport_preview_enabled: bool,
    // Trap fields
    pub trap_damage: String,
    pub trap_effect: String,
    // Sign fields
    pub sign_text: String,
    // NPC Dialogue fields
    pub npc_id: String,
}

impl Default for EventEditorState {
    fn default() -> Self {
        Self {
            event_type: EventType::default(),
            position: Position::new(0, 0),
            name: String::new(),
            description: String::new(),
            encounter_monsters: Vec::new(),
            encounter_monsters_query: String::new(),
            treasure_items: Vec::new(),
            treasure_items_query: String::new(),
            teleport_x: String::new(),
            teleport_y: String::new(),
            teleport_map_id: String::new(),
            teleport_selected_map: None,
            teleport_selected_pos: None,
            teleport_preview_enabled: false,
            trap_damage: String::new(),
            trap_effect: String::new(),
            sign_text: String::new(),
            npc_id: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    Encounter,
    Treasure,
    Teleport,
    Trap,
    Sign,
    NpcDialogue,
}

impl Default for EventType {
    fn default() -> Self {
        EventType::Encounter
    }
}

impl EventType {
    pub fn name(&self) -> &str {
        match self {
            EventType::Encounter => "Encounter",
            EventType::Treasure => "Treasure",
            EventType::Teleport => "Teleport",
            EventType::Trap => "Trap",
            EventType::Sign => "Sign",
            EventType::NpcDialogue => "NPC Dialogue",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            EventType::Encounter => "⚔️",
            EventType::Treasure => "💰",
            EventType::Teleport => "🌀",
            EventType::Trap => "🪤",
            EventType::Sign => "📜",
            EventType::NpcDialogue => "💬",
        }
    }

    pub fn all() -> &'static [EventType] {
        &[
            EventType::Encounter,
            EventType::Treasure,
            EventType::Teleport,
            EventType::Trap,
            EventType::Sign,
            EventType::NpcDialogue,
        ]
    }
}

impl EventEditorState {
    pub fn to_map_event(&self) -> Result<MapEvent, String> {
        match self.event_type {
            EventType::Encounter => {
                let monsters: Vec<MonsterId> = self.encounter_monsters.clone();
                if monsters.is_empty() {
                    return Err("Encounter must have at least one monster ID".to_string());
                }
                Ok(MapEvent::Encounter {
                    name: self.name.clone(),
                    description: self.description.clone(),
                    monster_group: monsters,
                })
            }
            EventType::Treasure => {
                let loot: Vec<ItemId> = self.treasure_items.clone();
                Ok(MapEvent::Treasure {
                    name: self.name.clone(),
                    description: self.description.clone(),
                    loot,
                })
            }
            EventType::Teleport => {
                // Try to parse X/Y from the textual fields first. If parsing fails, fall back
                // to the selected preview position (if any) so users can pick positions from the
                // target map preview instead of typing numeric values.
                let (x, y) = match (
                    self.teleport_x.parse::<i32>(),
                    self.teleport_y.parse::<i32>(),
                ) {
                    (Ok(x), Ok(y)) => (x, y),
                    _ => {
                        // If we can't parse numeric coordinates, use the selected preview coordinate if set.
                        if let Some(pos) = self.teleport_selected_pos.as_ref() {
                            (pos.x, pos.y)
                        } else {
                            return Err("Invalid X or Y coordinate".to_string());
                        }
                    }
                };

                // Try parsing the target map ID from the text field. If that fails, fall back to the
                // selected map from the UI preview component, if available.
                let map_id = match self.teleport_map_id.parse::<MapId>() {
                    Ok(id) => id,
                    Err(_) => {
                        if let Some(id) = self.teleport_selected_map {
                            id
                        } else {
                            return Err("Invalid map ID".to_string());
                        }
                    }
                };

                Ok(MapEvent::Teleport {
                    name: self.name.clone(),
                    description: self.description.clone(),
                    destination: Position::new(x, y),
                    map_id,
                })
            }
            EventType::Trap => {
                let damage = self
                    .trap_damage
                    .parse()
                    .map_err(|_| "Invalid damage value")?;
                let effect = if self.trap_effect.is_empty() {
                    None
                } else {
                    Some(self.trap_effect.clone())
                };
                Ok(MapEvent::Trap {
                    name: self.name.clone(),
                    description: self.description.clone(),
                    damage,
                    effect,
                })
            }
            EventType::Sign => {
                if self.sign_text.is_empty() {
                    return Err("Sign text cannot be empty".to_string());
                }
                Ok(MapEvent::Sign {
                    name: self.name.clone(),
                    description: self.description.clone(),
                    text: self.sign_text.clone(),
                })
            }
            EventType::NpcDialogue => {
                let npc_id = self.npc_id.parse().map_err(|_| "Invalid NPC ID")?;
                Ok(MapEvent::NpcDialogue {
                    name: self.name.clone(),
                    description: self.description.clone(),
                    npc_id,
                })
            }
        }
    }

    /// Initializes an `EventEditorState` from an existing `MapEvent` for editing.
    ///
    /// # Arguments
    ///
    /// * `position` - Position of the event on the map
    /// * `event` - Reference to the existing `MapEvent`
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::MapEvent;
    /// use antares::domain::types::Position;
    /// use campaign_builder::map_editor::{EventEditorState, EventType};
    ///
    /// let event = MapEvent::Sign {
    ///     name: "Sign".to_string(),
    ///     description: "Desc".to_string(),
    ///     text: "Hello".to_string(),
    /// };
    ///
    /// let editor = EventEditorState::from_map_event(Position::new(1, 1), &event);
    /// assert_eq!(editor.event_type, EventType::Sign);
    /// ```
    pub fn from_map_event(position: Position, event: &MapEvent) -> Self {
        let mut s = EventEditorState::default();
        s.position = position;
        match event {
            MapEvent::Encounter {
                name,
                description,
                monster_group,
            } => {
                s.event_type = EventType::Encounter;
                s.name = name.clone();
                s.description = description.clone();
                s.encounter_monsters = monster_group.clone();
            }
            MapEvent::Treasure {
                name,
                description,
                loot,
            } => {
                s.event_type = EventType::Treasure;
                s.name = name.clone();
                s.description = description.clone();
                s.treasure_items = loot.clone();
            }
            MapEvent::Teleport {
                name,
                description,
                destination,
                map_id,
            } => {
                s.event_type = EventType::Teleport;
                s.name = name.clone();
                s.description = description.clone();
                s.teleport_x = destination.x.to_string();
                s.teleport_y = destination.y.to_string();
                s.teleport_map_id = map_id.to_string();
                s.teleport_selected_map = Some(*map_id);
                s.teleport_selected_pos = Some(*destination);
                s.teleport_preview_enabled = true;
            }
            MapEvent::Trap {
                name,
                description,
                damage,
                effect,
            } => {
                s.event_type = EventType::Trap;
                s.name = name.clone();
                s.description = description.clone();
                s.trap_damage = damage.to_string();
                s.trap_effect = effect.clone().unwrap_or_default();
            }
            MapEvent::Sign {
                name,
                description,
                text,
            } => {
                s.event_type = EventType::Sign;
                s.name = name.clone();
                s.description = description.clone();
                s.sign_text = text.clone();
            }
            MapEvent::NpcDialogue {
                name,
                description,
                npc_id,
            } => {
                s.event_type = EventType::NpcDialogue;
                s.name = name.clone();
                s.description = description.clone();
                s.npc_id = npc_id.to_string();
            }
        }
        s
    }
}

// ===== NPC Editor State =====

/// NPC editor state
#[derive(Debug, Clone, Default)]
pub struct NpcEditorState {
    pub npc_id: String,
    pub name: String,
    pub description: String,
    pub position_x: String,
    pub position_y: String,
    pub dialogue: String,
}

impl NpcEditorState {
    pub fn to_npc(&self) -> Result<Npc, String> {
        let id = self.npc_id.parse().map_err(|_| "Invalid NPC ID")?;
        let x = self
            .position_x
            .parse()
            .map_err(|_| "Invalid X coordinate")?;
        let y = self
            .position_y
            .parse()
            .map_err(|_| "Invalid Y coordinate")?;

        if self.name.is_empty() {
            return Err("NPC name cannot be empty".to_string());
        }

        Ok(Npc::new(
            id,
            self.name.clone(),
            self.description.clone(),
            Position::new(x, y),
            self.dialogue.clone(),
        ))
    }

    pub fn clear(&mut self) {
        self.npc_id.clear();
        self.name.clear();
        self.description.clear();
        self.position_x.clear();
        self.position_y.clear();
        self.dialogue.clear();
    }
}

// ===== Map Grid Widget =====

/// Map grid visualization widget
pub struct MapGridWidget<'a> {
    state: &'a mut MapEditorState,
    tile_size: f32,
}

impl<'a> MapGridWidget<'a> {
    pub fn new(state: &'a mut MapEditorState) -> Self {
        Self {
            state,
            tile_size: 24.0,
        }
    }

    pub fn tile_size(mut self, size: f32) -> Self {
        self.tile_size = size;
        self
    }

    fn tile_color(tile: &Tile, has_event: bool, has_npc: bool) -> Color32 {
        if has_npc {
            return Color32::from_rgb(255, 200, 0); // Yellow for NPCs
        }

        // Determine terrain color first so we can blend it with wall color if needed
        let terrain_color = match tile.terrain {
            TerrainType::Ground => Color32::from_rgb(210, 180, 140), // Tan
            TerrainType::Grass => Color32::from_rgb(50, 205, 50),    // Lime Green
            TerrainType::Water => Color32::from_rgb(30, 144, 255),   // Dodger Blue
            TerrainType::Lava => Color32::from_rgb(255, 69, 0),      // Red-Orange
            TerrainType::Swamp => Color32::from_rgb(85, 107, 47),    // Dark Olive Green
            TerrainType::Stone => Color32::from_rgb(169, 169, 169),  // Dark Gray
            TerrainType::Dirt => Color32::from_rgb(139, 69, 19),     // Saddle Brown
            TerrainType::Forest => Color32::from_rgb(34, 139, 34),   // Forest Green
            TerrainType::Mountain => Color32::from_rgb(105, 105, 105), // Dim Gray
        };

        if has_event {
            return Color32::from_rgb(255, 100, 100); // Red for events
        }

        if tile.wall_type != WallType::None {
            return match tile.wall_type {
                // For normal walls, render a darker version of the terrain color
                // so that Forest Normal looks like a green wall while Stone Normal
                // remains greyish, keeping the wall/terrain visual distinction.
                WallType::Normal => {
                    // darken factor (0.0..1.0) — lower = darker
                    let factor = 0.60_f32;
                    let r = (terrain_color.r() as f32 * factor).round() as u8;
                    let g = (terrain_color.g() as f32 * factor).round() as u8;
                    let b = (terrain_color.b() as f32 * factor).round() as u8;
                    Color32::from_rgb(r, g, b)
                }
                WallType::Door => Color32::from_rgb(139, 69, 19),
                WallType::Torch => Color32::from_rgb(255, 165, 0),
                WallType::None => Color32::WHITE,
            };
        }

        terrain_color
    }
}

impl<'a> Widget for MapGridWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        // Compute the nominal map pixel size
        let map_width_px = self.state.map.width as f32 * self.tile_size;
        let map_height_px = self.state.map.height as f32 * self.tile_size;

        // Obtain the UI available size so we can ensure the painter covers the full panel area
        // and avoid the grid being cut off if the UI provides more space than the grid requires.
        let avail = ui.available_size();

        // The allocated painter size should be at least the map size but also at least
        // the available UI size so the canvas won't be narrower/taller than the available
        // area and nothing gets clipped in the layout.
        let width = map_width_px.max(avail.x).max(1.0);
        let height = map_height_px.max(avail.y).max(1.0);

        let (response, painter) =
            ui.allocate_painter(Vec2::new(width, height), Sense::click_and_drag());

        // Draw a visible debug border around the allocated grid area so we can see whether the widget's area
        // is being allocated (if the plot is blank, this border will still render).
        // Removed temporary debug red border to avoid distracting visual noise.

        let grid_offset = Vec2::new(
            ((width - map_width_px) / 2.0).max(0.0),
            ((height - map_height_px) / 2.0).max(0.0),
        );

        let to_screen = |x: i32, y: i32| -> Pos2 {
            response.rect.min
                + grid_offset
                + Vec2::new(x as f32 * self.tile_size, y as f32 * self.tile_size)
        };

        // Draw tiles
        for y in 0..self.state.map.height as i32 {
            for x in 0..self.state.map.width as i32 {
                let pos = Position::new(x, y);
                if let Some(tile) = self.state.map.get_tile(pos) {
                    let has_event =
                        self.state.show_events && self.state.map.events.contains_key(&pos);
                    let has_npc = self.state.show_npcs
                        && self.state.map.npcs.iter().any(|npc| npc.position == pos);

                    let color = Self::tile_color(tile, has_event, has_npc);

                    let rect = Rect::from_min_size(
                        to_screen(x, y),
                        Vec2::new(self.tile_size, self.tile_size),
                    );

                    painter.rect_filled(rect, 0.0, color);

                    // Draw grid lines
                    if self.state.show_grid {
                        painter.rect_stroke(
                            rect,
                            0.0,
                            // Make the grid color white for better visibility on dark themes
                            Stroke::new(1.0, Color32::WHITE),
                            egui::StrokeKind::Outside,
                        );
                    }

                    // Highlight selected tile
                    if self.state.selected_position == Some(pos) {
                        painter.rect_stroke(
                            rect,
                            0.0,
                            Stroke::new(2.0, Color32::YELLOW),
                            egui::StrokeKind::Outside,
                        );
                    }
                }
            }
        }

        // Handle clicks
        if response.clicked() {
            if let Some(click_pos) = response.interact_pointer_pos() {
                // Transform the click point into the map-local coordinates by subtracting the painter min and the
                // grid offset to account for centering. This ensures we calculate the correct tile indices.
                let local_pos = click_pos - response.rect.min - grid_offset;
                let x = (local_pos.x / self.tile_size) as i32;
                let y = (local_pos.y / self.tile_size) as i32;
                let pos = Position::new(x, y);

                if self.state.map.is_valid_position(pos) {
                    self.state.selected_position = Some(pos);

                    // Apply current tool
                    match self.state.current_tool {
                        EditorTool::Select => {}
                        EditorTool::PaintTile => {
                            self.state.paint_tile(pos);
                        }
                        EditorTool::Erase => {
                            self.state.erase_tile(pos);
                        }
                        EditorTool::PlaceEvent => {
                            if self.state.event_editor.is_none() {
                                self.state.event_editor = Some(EventEditorState {
                                    position: pos,
                                    ..Default::default()
                                });
                            } else if let Some(ref mut editor) = self.state.event_editor {
                                editor.position = pos;
                            }
                        }
                        EditorTool::PlaceNpc => {
                            if self.state.npc_editor.is_none() {
                                self.state.npc_editor = Some(NpcEditorState {
                                    position_x: pos.x.to_string(),
                                    position_y: pos.y.to_string(),
                                    ..Default::default()
                                });
                            } else if let Some(ref mut editor) = self.state.npc_editor {
                                editor.position_x = pos.x.to_string();
                                editor.position_y = pos.y.to_string();
                            }
                        }
                        EditorTool::Fill => {
                            // Fill tool requires two clicks (start and end)
                            // For simplicity, we'll just paint single tiles for now
                            self.state.paint_tile(pos);
                        }
                    }
                }
            }
        }

        response
    }
}

// Map Preview Widget: interactive preview of a map for selecting teleport destination.
//
// This widget is similar to the main grid, but it renders an arbitrary Map reference
// and signals clicks back via a mutable Option<Position> passed in by the caller.
pub struct MapPreviewWidget<'a> {
    map: &'a Map,
    selected_pos: &'a mut Option<Position>,
    tile_size: f32,
}

impl<'a> MapPreviewWidget<'a> {
    pub fn new(map: &'a Map, selected_pos: &'a mut Option<Position>) -> Self {
        Self {
            map,
            selected_pos,
            tile_size: 12.0,
        }
    }

    pub fn tile_size(mut self, tile_size: f32) -> Self {
        self.tile_size = tile_size;
        self
    }
}

impl<'a> Widget for MapPreviewWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tile_size = self.tile_size;

        let map_width_px = self.map.width as f32 * tile_size;
        let map_height_px = self.map.height as f32 * tile_size;

        let avail = ui.available_size();
        let width = map_width_px.max(avail.x).max(1.0);
        let height = map_height_px.max(avail.y).max(1.0);

        let (response, painter) = ui.allocate_painter(Vec2::new(width, height), Sense::click());

        let grid_offset = Vec2::new(
            ((width - map_width_px) / 2.0).max(0.0),
            ((height - map_height_px) / 2.0).max(0.0),
        );

        let to_screen = |x: i32, y: i32| -> Pos2 {
            response.rect.min + grid_offset + Vec2::new(x as f32 * tile_size, y as f32 * tile_size)
        };

        for y in 0..self.map.height as i32 {
            for x in 0..self.map.width as i32 {
                let pos = Position::new(x, y);
                if let Some(tile) = self.map.get_tile(pos) {
                    let has_event = self.map.events.contains_key(&pos);
                    let has_npc = self.map.npcs.iter().any(|n| n.position == pos);
                    let color = MapGridWidget::tile_color(tile, has_event, has_npc);

                    let rect =
                        Rect::from_min_size(to_screen(x, y), Vec2::new(tile_size, tile_size));
                    painter.rect_filled(rect, 0.0, color);

                    painter.rect_stroke(
                        rect,
                        0.0,
                        Stroke::new(1.0, Color32::WHITE),
                        egui::StrokeKind::Outside,
                    );

                    if let Some(sel_pos) = self.selected_pos {
                        if *sel_pos == pos {
                            painter.rect_stroke(
                                rect,
                                0.0,
                                Stroke::new(2.0, Color32::YELLOW),
                                egui::StrokeKind::Outside,
                            );
                        }
                    }
                }
            }
        }

        if response.clicked() {
            if let Some(click_pos) = response.interact_pointer_pos() {
                let local_pos = click_pos - response.rect.min - grid_offset;
                let x = (local_pos.x / tile_size) as i32;
                let y = (local_pos.y / tile_size) as i32;
                let pos = Position::new(x, y);
                if self.map.is_valid_position(pos) {
                    *self.selected_pos = Some(pos);
                }
            }
        }

        response
    }
}

// ===== Main Maps Editor State =====

/// Suggest maps by partial input (id or name)
fn suggest_maps_for_partial(maps: &[Map], partial: &str) -> Vec<(MapId, String)> {
    let partial_lower = partial.to_lowercase();

    let mut suggestions: Vec<(MapId, String)> = maps
        .iter()
        .filter(|m| {
            // Match against id text or name substring, case-insensitive
            m.id.to_string().contains(&partial_lower)
                || m.name.to_lowercase().contains(&partial_lower)
        })
        .map(|m| (m.id, m.name.clone()))
        .take(10)
        .collect();

    // Sort by ID for deterministic ordering
    suggestions.sort_unstable_by_key(|(id, _name)| *id);
    suggestions
}

// ===== Main Maps Editor State =====

// ===== Main Maps Editor State =====

/// Main maps editor state following the standard SDK editor pattern.
///
/// This struct holds all state for the maps editor, including the list of maps,
/// search/filter settings, and the currently active map editor state (if any).
///
/// # Usage
///
/// ```rust,no_run
/// use campaign_builder::map_editor::MapsEditorState;
///
/// let mut state = MapsEditorState::new();
///
/// // In the main app update loop:
/// // state.show(ui, &mut maps, campaign_dir, maps_dir, &mut unsaved, &mut status);
/// ```
pub struct MapsEditorState {
    /// Current editor mode
    pub mode: MapsEditorMode,
    /// Search filter text
    pub search_filter: String,
    /// Currently selected map index
    pub selected_map_idx: Option<usize>,
    /// Active map editor state (when editing a specific map)
    pub active_editor: Option<MapEditorState>,
    /// File load merge mode
    pub file_load_merge_mode: bool,
    /// Show import dialog
    pub show_import_dialog: bool,
    /// Import/export buffer
    pub import_export_buffer: String,
    /// New map width (for create dialog)
    pub new_map_width: u32,
    /// New map height (for create dialog)
    pub new_map_height: u32,
    /// New map name (for create dialog)
    pub new_map_name: String,
    /// Global zoom level for map grid (1.0 = 100%)
    pub zoom_level: f32,
}

/// Minimum zoom level (25%)
const MIN_ZOOM: f32 = 0.25;
/// Maximum zoom level (400%)
const MAX_ZOOM: f32 = 4.0;
/// Default zoom level (100%)
const DEFAULT_ZOOM: f32 = 1.0;
/// Zoom step for in/out buttons
const ZOOM_STEP: f32 = 0.25;
/// Base tile size in pixels (before zoom)
const BASE_TILE_SIZE: f32 = 24.0;
/// Minimum tile size in pixels (for usability)
const MIN_TILE_SIZE: f32 = 8.0;
/// Horizontal padding (pixels) to keep around the map when computing left column width.
const MAP_HORIZONTAL_PADDING: f32 = 8.0;

/// Compute the fallback requested left width for maps:
/// - `total_width` - total available width for interface.
/// - `inspector_min_width` - minimum width for the inspector (right column).
/// - `sep_margin` - separation/margin between columns.
/// - `map_pixel_width` - width of the map in pixels at the current zoom.
/// - `map_padding` - horizontal padding around the map content.
///
/// This function centralizes the "requested left width" calculation for the MapsEditor
/// to avoid duplicating the logic inline, and enables simple unit testing.
fn compute_map_requested_left(
    total_width: f32,
    inspector_min_width: f32,
    sep_margin: f32,
    map_pixel_width: f32,
    map_padding: f32,
) -> f32 {
    let fallback_requested_left = total_width - inspector_min_width - sep_margin;
    let desired_left_for_map = map_pixel_width + 2.0 * map_padding;
    fallback_requested_left.min(desired_left_for_map)
}

impl Default for MapsEditorState {
    fn default() -> Self {
        Self {
            mode: MapsEditorMode::List,
            search_filter: String::new(),
            selected_map_idx: None,
            active_editor: None,
            file_load_merge_mode: false,
            show_import_dialog: false,
            import_export_buffer: String::new(),
            new_map_width: 20,
            new_map_height: 20,
            new_map_name: "New Map".to_string(),
            zoom_level: DEFAULT_ZOOM,
        }
    }
}

impl MapsEditorState {
    /// Create a new maps editor state
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the next available map ID
    fn next_available_map_id(maps: &[Map]) -> MapId {
        maps.iter().map(|m| m.id).max().unwrap_or(0) + 1
    }

    /// Build a filtered list snapshot of maps (preserve original indices) sorted by ID.
    ///
    /// This keeps the underlying `maps` vector untouched while producing a snapshot of the UI
    /// list sorted by `MapId` and containing the original indices so that selection by index
    /// into the `maps` vector remains valid.
    fn build_filtered_maps_snapshot(
        maps: &[Map],
        search_filter: &str,
    ) -> Vec<(usize, MapId, String, u32, u32, usize, usize)> {
        let search_lower = search_filter.to_lowercase();

        let mut filtered: Vec<(usize, MapId, String, u32, u32, usize, usize)> = maps
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                search_lower.is_empty()
                    || m.name.to_lowercase().contains(&search_lower)
                    || m.id.to_string().contains(&search_lower)
            })
            .map(|(idx, m)| {
                (
                    idx,
                    m.id,
                    m.name.clone(),
                    m.width,
                    m.height,
                    m.events.len(),
                    m.npcs.len(),
                )
            })
            .collect();

        // Sort by ID for deterministic ordering in the UI list
        filtered.sort_unstable_by_key(|(_, id, ..)| *id);

        filtered
    }

    /// Render the Maps Editor UI
    ///
    /// This follows the standard editor pattern with EditorToolbar, TwoColumnLayout,
    /// and ActionButtons.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        maps: &mut Vec<Map>,
        monsters: &[MonsterDefinition],
        items: &[Item],
        campaign_dir: Option<&PathBuf>,
        maps_dir: &str,
        display_config: &DisplayConfig,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        ui.heading("🗺️ Maps Editor");
        ui.add_space(5.0);

        // Use shared EditorToolbar component
        let toolbar_action = EditorToolbar::new("Maps")
            .with_search(&mut self.search_filter)
            .with_merge_mode(&mut self.file_load_merge_mode)
            .with_total_count(maps.len())
            .with_id_salt("maps_toolbar")
            .show(ui);

        // Handle toolbar actions
        match toolbar_action {
            ToolbarAction::New => {
                let new_id = Self::next_available_map_id(maps);
                let new_map = Map::new(
                    new_id,
                    self.new_map_name.clone(),
                    String::new(),
                    self.new_map_width,
                    self.new_map_height,
                );
                maps.push(new_map.clone());
                self.selected_map_idx = Some(maps.len() - 1);
                self.active_editor = Some(MapEditorState::new(new_map));
                self.mode = MapsEditorMode::Add;
                *unsaved_changes = true;
            }
            ToolbarAction::Save => {
                self.save_all_maps(
                    maps,
                    campaign_dir,
                    maps_dir,
                    unsaved_changes,
                    status_message,
                );
            }
            ToolbarAction::Load => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("RON", &["ron"])
                    .pick_file()
                {
                    match fs::read_to_string(&path) {
                        Ok(contents) => match ron::from_str::<Map>(&contents) {
                            Ok(loaded_map) => {
                                if self.file_load_merge_mode {
                                    if let Some(existing) =
                                        maps.iter_mut().find(|m| m.id == loaded_map.id)
                                    {
                                        *existing = loaded_map;
                                    } else {
                                        maps.push(loaded_map);
                                    }
                                } else {
                                    maps.push(loaded_map);
                                }
                                *unsaved_changes = true;
                                *status_message = format!("Loaded map from: {}", path.display());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to parse map: {}", e);
                            }
                        },
                        Err(e) => {
                            *status_message = format!("Failed to read map file: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Import => {
                self.show_import_dialog = true;
                self.import_export_buffer.clear();
            }
            ToolbarAction::Export => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("maps.ron")
                    .add_filter("RON", &["ron"])
                    .save_file()
                {
                    match ron::ser::to_string_pretty(maps, Default::default()) {
                        Ok(contents) => match fs::write(&path, contents) {
                            Ok(_) => {
                                *status_message = format!("Exported maps to: {}", path.display());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to export maps: {}", e);
                            }
                        },
                        Err(e) => {
                            *status_message = format!("Failed to serialize maps: {}", e);
                        }
                    }
                }
            }
            ToolbarAction::Reload => {
                self.load_maps(maps, campaign_dir, maps_dir, status_message);
            }
            ToolbarAction::None => {}
        }

        ui.separator();

        // Show appropriate view based on mode
        match self.mode {
            MapsEditorMode::List => {
                self.show_list(
                    ui,
                    maps,
                    campaign_dir,
                    maps_dir,
                    unsaved_changes,
                    status_message,
                );
            }
            MapsEditorMode::Add | MapsEditorMode::Edit => {
                self.show_editor(
                    ui,
                    maps,
                    monsters,
                    items,
                    campaign_dir,
                    maps_dir,
                    display_config,
                    unsaved_changes,
                    status_message,
                );
            }
        }

        // Import dialog
        if self.show_import_dialog {
            self.show_import_dialog_window(ui.ctx(), maps, unsaved_changes, status_message);
        }
    }

    /// Show the list view with map previews
    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        maps: &mut Vec<Map>,
        campaign_dir: Option<&PathBuf>,
        maps_dir: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        // Build and sort filtered list snapshot for UI display
        let filtered_maps = Self::build_filtered_maps_snapshot(maps, &self.search_filter);

        let selected = self.selected_map_idx;
        let mut new_selection = selected;
        let mut action_requested: Option<ItemAction> = None;

        TwoColumnLayout::new("maps").show_split(
            ui,
            |left_ui| {
                // Left panel: Map list
                left_ui.heading("Maps");
                left_ui.separator();

                for (idx, id, name, width, height, events, npcs) in &filtered_maps {
                    let is_selected = selected == Some(*idx);
                    let label = format!(
                        "[{}] {} ({}x{}) E:{} N:{}",
                        id, name, width, height, events, npcs
                    );
                    if left_ui.selectable_label(is_selected, &label).clicked() {
                        new_selection = Some(*idx);
                    }
                }

                if filtered_maps.is_empty() {
                    left_ui.label("No maps found");
                    left_ui.add_space(10.0);
                    left_ui.label("Create a new map using the toolbar.");
                }
            },
            |right_ui| {
                // Right panel: Detail view or preview
                if let Some(idx) = selected {
                    if idx < maps.len() {
                        let map = &maps[idx];

                        right_ui.heading(&map.name);
                        right_ui.separator();

                        // Action buttons
                        action_requested = Some(
                            ActionButtons::new()
                                .enabled(true)
                                .with_duplicate(true)
                                .show(right_ui),
                        );

                        right_ui.separator();

                        // Map info
                        right_ui.group(|ui| {
                            ui.label(format!("Map ID: {}", map.id));
                            ui.label(format!("Size: {}x{}", map.width, map.height));
                            ui.label(format!("Events: {}", map.events.len()));
                            ui.label(format!("NPCs: {}", map.npcs.len()));
                            if !map.description.is_empty() {
                                ui.separator();
                                ui.label("Description:");
                                ui.label(&map.description);
                            }
                        });

                        right_ui.add_space(10.0);

                        // Preview
                        right_ui.heading("Preview");
                        right_ui.separator();
                        Self::show_map_preview(right_ui, map);
                    }
                } else {
                    right_ui.heading("No Map Selected");
                    right_ui.separator();
                    right_ui.label("Select a map from the list to view details.");
                    right_ui.add_space(20.0);

                    // New map creation form
                    right_ui.group(|ui| {
                        ui.heading("Create New Map");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.new_map_name);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Width:");
                            ui.add(egui::DragValue::new(&mut self.new_map_width).range(5..=100));
                            ui.label("Height:");
                            ui.add(egui::DragValue::new(&mut self.new_map_height).range(5..=100));
                        });
                    });
                }
            },
        );

        // Apply selection change
        if new_selection != selected {
            self.selected_map_idx = new_selection;
        }

        // Handle actions
        if let Some(action) = action_requested {
            match action {
                ItemAction::Edit => {
                    if let Some(idx) = self.selected_map_idx {
                        if idx < maps.len() {
                            self.active_editor = Some(MapEditorState::new(maps[idx].clone()));
                            self.mode = MapsEditorMode::Edit;
                        }
                    }
                }
                ItemAction::Delete => {
                    if let Some(idx) = self.selected_map_idx {
                        if idx < maps.len() {
                            // Save map before deletion for undo (not implemented yet)
                            let map = &maps[idx];
                            if let Some(dir) = campaign_dir {
                                let map_path =
                                    dir.join(maps_dir).join(format!("map_{}.ron", map.id));
                                if map_path.exists() {
                                    let _ = fs::remove_file(&map_path);
                                }
                            }
                            maps.remove(idx);
                            self.selected_map_idx = None;
                            *unsaved_changes = true;
                            *status_message = "Map deleted".to_string();
                        }
                    }
                }
                ItemAction::Duplicate => {
                    if let Some(idx) = self.selected_map_idx {
                        if idx < maps.len() {
                            let mut new_map = maps[idx].clone();
                            new_map.id = Self::next_available_map_id(maps);
                            new_map.name = format!("{} (Copy)", new_map.name);
                            maps.push(new_map);
                            self.selected_map_idx = Some(maps.len() - 1);
                            *unsaved_changes = true;
                            *status_message = "Map duplicated".to_string();
                        }
                    }
                }
                ItemAction::Export => {
                    if let Some(idx) = self.selected_map_idx {
                        if idx < maps.len() {
                            let map = &maps[idx];
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name(format!("map_{}.ron", map.id))
                                .add_filter("RON", &["ron"])
                                .save_file()
                            {
                                match ron::ser::to_string_pretty(map, Default::default()) {
                                    Ok(contents) => match fs::write(&path, contents) {
                                        Ok(_) => {
                                            *status_message =
                                                format!("Exported map to: {}", path.display());
                                        }
                                        Err(e) => {
                                            *status_message =
                                                format!("Failed to export map: {}", e);
                                        }
                                    },
                                    Err(e) => {
                                        *status_message = format!("Failed to serialize map: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
                ItemAction::None => {}
            }
        }
    }

    /// Show the full map editor
    #[allow(clippy::too_many_arguments, clippy::ptr_arg)]
    fn show_editor(
        &mut self,
        ui: &mut egui::Ui,
        maps: &mut Vec<Map>,
        monsters: &[MonsterDefinition],
        items: &[Item],
        campaign_dir: Option<&PathBuf>,
        maps_dir: &str,
        display_config: &DisplayConfig,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        // Top bar with back button and save
        let mut back_clicked = false;
        let mut save_clicked = false;

        ui.horizontal(|ui| {
            if ui.button("← Back to List").clicked() {
                back_clicked = true;
            }

            ui.separator();

            if let Some(ref editor) = self.active_editor {
                ui.label(format!(
                    "Editing: {} (ID: {})",
                    editor.metadata.name, editor.map.id
                ));

                if editor.has_changes {
                    ui.label("●").on_hover_text("Unsaved changes");
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("💾 Save Map").clicked() {
                    save_clicked = true;
                }

                // Get undo/redo state before buttons
                let (can_undo, can_redo) = if let Some(ref editor) = self.active_editor {
                    (editor.can_undo(), editor.can_redo())
                } else {
                    (false, false)
                };

                if ui
                    .add_enabled(can_redo, egui::Button::new("↪ Redo"))
                    .clicked()
                {
                    if let Some(ref mut ed) = self.active_editor {
                        ed.redo();
                    }
                }

                if ui
                    .add_enabled(can_undo, egui::Button::new("↩ Undo"))
                    .clicked()
                {
                    if let Some(ref mut ed) = self.active_editor {
                        ed.undo();
                    }
                }
            });
        });

        ui.separator();

        // Show the map editor content using two-column layout
        if let Some(ref mut editor) = self.active_editor {
            // Use a raw pointer to the editor to avoid simultaneous mutable borrows in the left/right closures
            let editor_ptr: *mut MapEditorState = editor as *mut _;
            // Map view (Grid/Events/NPCs & Zoom) and Tool palette rows.
            // Show a dedicated row for map view toggles and zoom controls first so the Tools row isn't cut off on narrow windows.
            let view_zoom_action = {
                let mut a: Option<ZoomAction> = None;

                ui.horizontal(|ui| {
                    // View options
                    ui.label("View: ");
                    ui.checkbox(&mut editor.show_grid, "Grid");
                    ui.checkbox(&mut editor.show_events, "Events");
                    ui.checkbox(&mut editor.show_npcs, "NPCs");

                    ui.checkbox(&mut editor.auto_fit_on_resize, "Auto Fit").on_hover_text(
                        "When enabled, the map will automatically scale to fit the left column when the window is resized. Manual zoom persists until Fit is clicked.",
                    );

                    ui.separator();

                    // Zoom controls
                    ui.label("Zoom:");
                    if ui.button("➖").on_hover_text("Zoom Out").clicked() {
                        a = Some(ZoomAction::Out);
                    }

                    ui.label(format!("{}%", (self.zoom_level * 100.0) as i32));

                    if ui.button("➕").on_hover_text("Zoom In").clicked() {
                        a = Some(ZoomAction::In);
                    }

                    if ui.button("⊡ Fit").on_hover_text("Fit map to available space").clicked() {
                        a = Some(ZoomAction::Fit);
                    }

                    if ui.button("100%").on_hover_text("Reset to 100%").clicked() {
                        a = Some(ZoomAction::Reset);
                    }
                });

                a
            };

            // Tool palette row (Tools, Terrain, and Wall)
            let tool_zoom_action = Self::show_tool_palette(ui, editor, self.zoom_level);

            // Prefer zoom action from the view toolbar over the tool palette (fallback to tool palette).
            let zoom_action = view_zoom_action.or(tool_zoom_action);

            // Apply immediate zoom in/out/reset changes
            if let Some(action) = zoom_action {
                match action {
                    ZoomAction::In => {
                        self.zoom_level = (self.zoom_level + ZOOM_STEP).min(MAX_ZOOM);
                    }
                    ZoomAction::Out => {
                        self.zoom_level = (self.zoom_level - ZOOM_STEP).max(MIN_ZOOM);
                    }
                    ZoomAction::Reset => {
                        self.zoom_level = DEFAULT_ZOOM;
                    }
                    ZoomAction::Fit => {
                        // Fit will be handled while drawing the grid where available size/context is known
                    }
                }
            }

            let fit_requested = matches!(zoom_action, Some(ZoomAction::Fit));
            let mut new_zoom: Option<f32> = None;

            ui.separator();

            // Main content: grid on left, inspector on right
            {
                // Compute overall panel height and left column width for TwoColumnLayout
                let panel_height = crate::ui_helpers::compute_panel_height(
                    ui,
                    crate::ui_helpers::DEFAULT_PANEL_MIN_HEIGHT,
                );

                let total_width = ui.available_width();
                let sep_margin = 12.0;
                // Use configured inspector minimum width (fallback to default helper constant if needed).
                let inspector_min_width = display_config
                    .inspector_min_width
                    .max(crate::ui_helpers::DEFAULT_INSPECTOR_MIN_WIDTH);

                // Map-specific logic: limit the requested left width to the map pixel width + a small padding,
                // so the list column will not be larger than needed for the map itself (prevents excessive horizontal padding).
                let default_requested_left = compute_map_requested_left(
                    total_width,
                    inspector_min_width,
                    sep_margin,
                    editor.map.width as f32 * BASE_TILE_SIZE * self.zoom_level,
                    MAP_HORIZONTAL_PADDING,
                );

                let left_width = crate::ui_helpers::compute_left_column_width(
                    total_width,
                    default_requested_left,
                    inspector_min_width,
                    sep_margin,
                    crate::ui_helpers::MIN_SAFE_LEFT_COLUMN_WIDTH,
                    display_config.left_column_max_ratio,
                );

                // Debug prints removed: layout diagnostics no longer logged to stderr.

                // Use the shared TwoColumnLayout to split the editor area cleanly
                TwoColumnLayout::new("maps")
                    .with_left_width(left_width)
                    .with_min_height(panel_height)
                    .with_inspector_min_width(display_config.inspector_min_width)
                    .with_max_left_ratio(display_config.left_column_max_ratio)
                    .show_split(
                        ui,
                        |left_ui| {
                            // Re-acquire the editor reference via raw pointer to avoid simultaneous borrows
                            let editor_ref: &mut MapEditorState = unsafe { &mut *editor_ptr };

                            // Compute draw_zoom using left_ui available size to ensure Fit uses the actual left column
                            // Honor "Auto Fit" when it's enabled (editor_auto_fit_on_resize).
                            let draw_zoom = if fit_requested
                                || (self.zoom_level == DEFAULT_ZOOM)
                                || editor_ref.auto_fit_on_resize
                            {
                                let avail = left_ui.available_size();
                                let map_width = editor_ref.map.width as f32 * BASE_TILE_SIZE;
                                let map_height = editor_ref.map.height as f32 * BASE_TILE_SIZE;

                                // Avoid division by zero
                                let zoom_x = if map_width > 0.0 {
                                    avail.x / map_width
                                } else {
                                    self.zoom_level
                                };
                                let zoom_y = if map_height > 0.0 {
                                    avail.y / map_height
                                } else {
                                    self.zoom_level
                                };

                                let fit_zoom = zoom_x.min(zoom_y);

                                let min_zoom_for_tiles = MIN_TILE_SIZE / BASE_TILE_SIZE;

                                let result =
                                    fit_zoom.clamp(min_zoom_for_tiles.max(MIN_ZOOM), MAX_ZOOM);

                                // Persist the computed zoom so it becomes the new global zoom when opening the editor
                                new_zoom = Some(result);

                                // Debug prints removed: fit calculation logging suppressed.

                                result
                            } else {
                                self.zoom_level
                            };

                            let effective_tile_size = BASE_TILE_SIZE * draw_zoom;

                            // Debug info: show map dims, zoom and left column width
                            // Removed temporary UI debug label.

                            // Removed temporary debug print.

                            egui::ScrollArea::both()
                                .id_salt("map_editor_grid_scroll")
                                .max_height(panel_height)
                                .auto_shrink([false, false])
                                .show(left_ui, |ui| {
                                    // Debug logging removed: no additional stderr logs.

                                    let map_response = ui.add(
                                        MapGridWidget::new(editor_ref)
                                            .tile_size(effective_tile_size),
                                    );

                                    // Debug logging removed for map widget response rect.

                                    // Small debug label removed.
                                });
                        },
                        |right_ui| {
                            // Re-acquire the editor reference via raw pointer to avoid simultaneous borrows
                            let editor_ref: &mut MapEditorState = unsafe { &mut *editor_ptr };

                            // Right panel: Inspector and tool-specific editors
                            right_ui.set_min_width(display_config.inspector_min_width);

                            egui::ScrollArea::vertical()
                                .id_salt("map_editor_inspector_scroll")
                                .show(right_ui, |ui| {
                                    Self::show_inspector_panel(
                                        ui, editor_ref, maps, monsters, items,
                                    );
                                });
                        },
                    );
            }

            // If a fit was requested, update the global zoom to persist the change
            if let Some(z) = new_zoom {
                self.zoom_level = z;
            }
        }

        // Handle back action
        if back_clicked {
            // Save changes if any
            if self.active_editor.is_some() {
                // Scoped mutable borrow to collect a map to save if any changes exist.
                let map_opt: Option<Map> = self.active_editor.as_mut().and_then(|editor| {
                    if editor.has_changes {
                        // Ensure metadata is reflected in the map before saving
                        editor.apply_metadata();
                        Some(editor.map.clone())
                    } else {
                        None
                    }
                });

                if let Some(map) = map_opt {
                    if let Some(idx) = self.selected_map_idx {
                        if idx < maps.len() {
                            maps[idx] = map.clone();
                        }
                    }

                    // Save to file (mutable borrow released)
                    if let Err(e) = self.save_map(&map, campaign_dir, maps_dir) {
                        *status_message = format!("Failed to save map: {}", e);
                    } else {
                        *status_message = "Map saved".to_string();
                        *unsaved_changes = true;
                        // Re-borrow to update the editor with the saved map and clear dirty flag
                        if let Some(editor) = self.active_editor.as_mut() {
                            editor.map = map;
                            editor.has_changes = false;
                        }
                    }
                }
            }
            self.mode = MapsEditorMode::List;
            self.active_editor = None;
        }

        // Handle save action
        if save_clicked {
            // Acquire a clone of the map to save while avoiding overlapping borrows
            let map_opt: Option<Map> = self.active_editor.as_mut().map(|editor| {
                // Sync metadata to the underlying map before saving
                editor.apply_metadata();
                editor.map.clone()
            });

            if let Some(map) = map_opt {
                if let Some(idx) = self.selected_map_idx {
                    if idx < maps.len() {
                        maps[idx] = map.clone();
                    }
                }
                if let Err(e) = self.save_map(&map, campaign_dir, maps_dir) {
                    *status_message = format!("Failed to save map: {}", e);
                } else {
                    *status_message = format!("Map {} saved", map.id);
                    *unsaved_changes = true;
                    // Re-borrow to clear flags and update editor's map
                    if let Some(editor) = self.active_editor.as_mut() {
                        editor.has_changes = false;
                        editor.map = map;
                    }
                }
            }
        }
    }

    /// Show tool palette with zoom controls
    fn show_tool_palette(
        ui: &mut egui::Ui,
        editor: &mut MapEditorState,
        current_zoom: f32,
    ) -> Option<ZoomAction> {
        let action: Option<ZoomAction> = None;

        ui.horizontal(|ui| {
            ui.label("Tools:");

            for tool in EditorTool::all() {
                if ui
                    .selectable_label(
                        editor.current_tool == *tool,
                        format!("{} {}", tool.icon(), tool.name()),
                    )
                    .clicked()
                {
                    editor.current_tool = *tool;
                }
            }

            ui.separator();

            // Terrain selection
            ui.label("Terrain:");
            egui::ComboBox::from_id_salt("map_terrain_palette")
                .selected_text(format!("{:?}", editor.selected_terrain))
                .show_ui(ui, |ui| {
                    for terrain in &[
                        TerrainType::Ground,
                        TerrainType::Grass,
                        TerrainType::Water,
                        TerrainType::Stone,
                        TerrainType::Dirt,
                        TerrainType::Forest,
                        TerrainType::Mountain,
                        TerrainType::Lava,
                        TerrainType::Swamp,
                    ] {
                        ui.selectable_value(
                            &mut editor.selected_terrain,
                            *terrain,
                            format!("{:?}", terrain),
                        );
                    }
                });

            ui.label("Wall:");
            egui::ComboBox::from_id_salt("map_wall_palette")
                .selected_text(format!("{:?}", editor.selected_wall))
                .show_ui(ui, |ui| {
                    for wall in &[
                        WallType::None,
                        WallType::Normal,
                        WallType::Door,
                        WallType::Torch,
                    ] {
                        ui.selectable_value(
                            &mut editor.selected_wall,
                            *wall,
                            format!("{:?}", wall),
                        );
                    }
                });

            // Keep a lightweight separator; the view controls and zoom are now above the tools.
            ui.separator();
        });
        action
    }

    /// Show map view toggle controls (Grid, Events, NPCs, Auto Fit) and Zoom controls.
    fn show_map_view_controls(
        ui: &mut egui::Ui,
        editor: &mut MapEditorState,
        current_zoom: f32,
    ) -> Option<ZoomAction> {
        let mut action: Option<ZoomAction> = None;

        ui.horizontal(|ui| {
            // View options
            ui.checkbox(&mut editor.show_grid, "Grid");
            ui.checkbox(&mut editor.show_events, "Events");
            ui.checkbox(&mut editor.show_npcs, "NPCs");

            ui.checkbox(&mut editor.auto_fit_on_resize, "Auto Fit").on_hover_text(
                "When enabled, the map will automatically scale to fit the left column when the window is resized. Manual zoom persists until Fit is clicked.",
            );

            ui.separator();

            // Zoom controls
            ui.label("Zoom:");
            if ui.button("➖").on_hover_text("Zoom Out").clicked() {
                action = Some(ZoomAction::Out);
            }

            ui.label(format!("{}%", (current_zoom * 100.0) as i32));

            if ui.button("➕").on_hover_text("Zoom In").clicked() {
                action = Some(ZoomAction::In);
            }

            if ui.button("⊡ Fit").on_hover_text("Fit map to available space").clicked() {
                action = Some(ZoomAction::Fit);
            }

            if ui.button("100%").on_hover_text("Reset to 100%").clicked() {
                action = Some(ZoomAction::Reset);
            }
        });

        action
    }

    /// Show inspector panel
    fn show_inspector_panel(
        ui: &mut egui::Ui,
        editor: &mut MapEditorState,
        maps: &[Map],
        monsters: &[MonsterDefinition],
        items: &[Item],
    ) {
        ui.heading("Inspector");
        ui.separator();

        // Metadata editor toggle
        if ui.button("🗺️ Edit Map Metadata").clicked() {
            editor.show_metadata_editor = !editor.show_metadata_editor;
        }

        if editor.show_metadata_editor {
            ui.separator();
            Self::show_metadata_editor(ui, editor);
            ui.separator();
        }

        // Map info
        ui.group(|ui| {
            ui.label(format!("Map ID: {}", editor.map.id));
            ui.label(format!("Size: {}×{}", editor.map.width, editor.map.height));
            ui.label(format!("Name: {}", editor.metadata.name));
        });

        ui.separator();

        // Selected tile info
        if let Some(pos) = editor.selected_position {
            ui.group(|ui| {
                ui.label(format!("Position: ({}, {})", pos.x, pos.y));

                if let Some(tile) = editor.map.get_tile(pos) {
                    ui.label(format!("Terrain: {:?}", tile.terrain));
                    ui.label(format!("Wall: {:?}", tile.wall_type));
                    ui.label(format!("Blocked: {}", tile.blocked));
                }

                if let Some(npc) = editor.map.npcs.iter().find(|n| n.position == pos) {
                    ui.separator();
                    ui.label("NPC:");
                    ui.label(format!("Name: {}", npc.name));
                    ui.label(format!("ID: {}", npc.id));
                }

                if let Some(event) = editor.map.get_event(pos) {
                    ui.separator();
                    ui.label("Event:");
                    match event {
                        MapEvent::Encounter { monster_group, .. } => {
                            ui.label(format!("Encounter: {:?}", monster_group));
                        }
                        MapEvent::Treasure { loot, .. } => {
                            ui.label(format!("Treasure: {:?}", loot));
                        }
                        MapEvent::Teleport {
                            destination,
                            map_id,
                            ..
                        } => {
                            ui.label(format!(
                                "Teleport to map {} at ({}, {})",
                                map_id, destination.x, destination.y
                            ));
                        }
                        MapEvent::Trap { damage, effect, .. } => {
                            ui.label(format!("Trap: {} damage", damage));
                            if let Some(eff) = effect {
                                ui.label(format!("Effect: {}", eff));
                            }
                        }
                        MapEvent::Sign { text, .. } => {
                            ui.label(format!("Sign: {}", text));
                        }
                        MapEvent::NpcDialogue { npc_id, .. } => {
                            ui.label(format!("NPC Dialogue: {}", npc_id));
                        }
                    }

                    if ui.button("🗑 Remove Event").clicked() {
                        editor.remove_event(pos);
                    }
                }
            });
        } else {
            ui.label("No tile selected");
        }

        ui.add_space(10.0);

        // Event editor (when PlaceEvent tool is active)
        if matches!(editor.current_tool, EditorTool::PlaceEvent) {
            ui.group(|ui| {
                ui.heading("Event Editor");
                Self::show_event_editor(ui, editor, maps, monsters, items);
            });
        }

        // NPC editor (when PlaceNpc tool is active)
        if matches!(editor.current_tool, EditorTool::PlaceNpc) {
            ui.group(|ui| {
                ui.heading("NPC Editor");
                Self::show_npc_editor(ui, editor);
            });
        }

        ui.add_space(10.0);

        // Statistics
        ui.group(|ui| {
            ui.heading("Statistics");
            ui.label(format!("Events: {}", editor.map.events.len()));
            ui.label(format!("NPCs: {}", editor.map.npcs.len()));
        });

        // Validation errors
        if !editor.validation_errors.is_empty() {
            ui.add_space(10.0);
            ui.group(|ui| {
                ui.heading("Validation");
                for error in &editor.validation_errors {
                    ui.label(error);
                }
            });
        }
    }

    /// Show metadata editor panel
    fn show_metadata_editor(ui: &mut egui::Ui, editor: &mut MapEditorState) {
        ui.group(|ui| {
            ui.heading("Map Metadata");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Name:");
                if ui.text_edit_singleline(&mut editor.metadata.name).changed() {
                    editor.has_changes = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Description:");
            });
            if ui
                .text_edit_multiline(&mut editor.metadata.description)
                .changed()
            {
                editor.has_changes = true;
            }

            ui.horizontal(|ui| {
                ui.label("Difficulty:");
                if ui
                    .add(egui::Slider::new(&mut editor.metadata.difficulty, 1..=10))
                    .changed()
                {
                    editor.has_changes = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Light Level:");
                if ui
                    .add(egui::Slider::new(&mut editor.metadata.light_level, 0..=100))
                    .changed()
                {
                    editor.has_changes = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Encounter Rate:");
                if ui
                    .add(egui::Slider::new(
                        &mut editor.metadata.encounter_rate,
                        0..=100,
                    ))
                    .changed()
                {
                    editor.has_changes = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Music Track:");
                if ui
                    .text_edit_singleline(&mut editor.metadata.music_track)
                    .changed()
                {
                    editor.has_changes = true;
                }
            });

            if ui
                .checkbox(&mut editor.metadata.is_outdoor, "Outdoor Map")
                .changed()
            {
                editor.has_changes = true;
            }

            ui.separator();

            if ui.button("Close").clicked() {
                editor.show_metadata_editor = false;
            }
        });
    }

    /// Show event editor
    fn show_event_editor(
        ui: &mut egui::Ui,
        editor: &mut MapEditorState,
        maps: &[Map],
        monsters: &[MonsterDefinition],
        items: &[Item],
    ) {
        if let Some(ref mut event_editor) = editor.event_editor {
            egui::ComboBox::from_id_salt("map_event_type_combo")
                .selected_text(event_editor.event_type.name())
                .show_ui(ui, |ui| {
                    for event_type in EventType::all() {
                        if ui
                            .selectable_label(
                                event_editor.event_type == *event_type,
                                format!("{} {}", event_type.icon(), event_type.name()),
                            )
                            .clicked()
                        {
                            event_editor.event_type = *event_type;
                        }
                    }
                });

            ui.separator();

            // Common event fields: name and description
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut event_editor.name);
            });
            ui.label("Description:");
            ui.text_edit_multiline(&mut event_editor.description);

            match event_editor.event_type {
                EventType::Encounter => {
                    // Multi-select searchable list for monsters (id+name)
                    let changed = searchable_selector_multi(
                        ui,
                        "event_encounter_monsters",
                        "Encounter Monsters",
                        &mut event_editor.encounter_monsters,
                        monsters,
                        |m| m.id,
                        |m| m.name.clone(),
                        &mut event_editor.encounter_monsters_query,
                    );
                    if changed {
                        editor.has_changes = true;
                    }
                }
                EventType::Treasure => {
                    // Multi-select searchable list for treasure items (id+name)
                    let changed = searchable_selector_multi(
                        ui,
                        "event_treasure_items",
                        "Treasure Items",
                        &mut event_editor.treasure_items,
                        items,
                        |i| i.id,
                        |i| i.name.clone(),
                        &mut event_editor.treasure_items_query,
                    );
                    if changed {
                        editor.has_changes = true;
                    }
                }
                EventType::Teleport => {
                    ui.label("Target Map:");
                    ui.horizontal(|ui| {
                        // Editable text input for map id or name
                        let changed = ui
                            .text_edit_singleline(&mut event_editor.teleport_map_id)
                            .changed();
                        if changed {
                            // If user types, clear the previously selected map
                            event_editor.teleport_selected_map = None;
                            event_editor.teleport_preview_enabled = false;
                        }
                    });

                    // Suggestions based on typed input (id or name)
                    let suggestions = suggest_maps_for_partial(maps, &event_editor.teleport_map_id);
                    if !suggestions.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Suggestions:");
                            for (sid, sname) in suggestions.iter() {
                                let label = format!("[{}] {}", sid, sname);
                                // Use small button so UI remains compact
                                if ui.small_button(&label).clicked() {
                                    event_editor.teleport_map_id = sid.to_string();
                                    event_editor.teleport_selected_map = Some(*sid);
                                    event_editor.teleport_preview_enabled = true;
                                }
                            }
                        });
                    } else {
                        // If empty input, provide quick selection of few maps (first 8)
                        if event_editor.teleport_map_id.is_empty() && !maps.is_empty() {
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Common:");
                                for m in maps.iter().take(8) {
                                    let label = format!("[{}] {}", m.id, m.name);
                                    if ui.small_button(&label).clicked() {
                                        event_editor.teleport_map_id = m.id.to_string();
                                        event_editor.teleport_selected_map = Some(m.id);
                                        event_editor.teleport_preview_enabled = true;
                                    }
                                }
                            });
                        }
                    }

                    ui.horizontal(|ui| {
                        ui.label("Destination X:");
                        if ui
                            .text_edit_singleline(&mut event_editor.teleport_x)
                            .changed()
                        {
                            if let (Ok(x), Ok(y)) = (
                                event_editor.teleport_x.parse::<i32>(),
                                event_editor.teleport_y.parse::<i32>(),
                            ) {
                                event_editor.teleport_selected_pos = Some(Position::new(x, y));
                            }
                        }

                        ui.label("Destination Y:");
                        if ui
                            .text_edit_singleline(&mut event_editor.teleport_y)
                            .changed()
                        {
                            if let (Ok(x), Ok(y)) = (
                                event_editor.teleport_x.parse::<i32>(),
                                event_editor.teleport_y.parse::<i32>(),
                            ) {
                                event_editor.teleport_selected_pos = Some(Position::new(x, y));
                            }
                        }
                    });

                    ui.checkbox(
                        &mut event_editor.teleport_preview_enabled,
                        "Show Target Map Preview",
                    );

                    if event_editor.teleport_preview_enabled {
                        let map_id_opt = event_editor
                            .teleport_selected_map
                            .or_else(|| event_editor.teleport_map_id.parse::<MapId>().ok());

                        if let Some(map_id) = map_id_opt {
                            if let Some(target_map) = maps.iter().find(|m| m.id == map_id) {
                                ui.label(format!(
                                    "Preview: [{}] {}",
                                    target_map.id, target_map.name
                                ));

                                // Draw interactive preview so the user can click to pick destination tile
                                let selected_pos_ref = &mut event_editor.teleport_selected_pos;
                                let preview_widget =
                                    MapPreviewWidget::new(target_map, selected_pos_ref)
                                        .tile_size(18.0);
                                let resp = ui.add(preview_widget);
                                if resp.clicked() {
                                    if let Some(pos) = event_editor.teleport_selected_pos {
                                        event_editor.teleport_x = pos.x.to_string();
                                        event_editor.teleport_y = pos.y.to_string();
                                    }
                                }
                            } else {
                                ui.label("Selected map not found in campaign.");
                            }
                        } else {
                            ui.label("Select a target map to enable preview");
                        }
                    }
                }
                EventType::Trap => {
                    ui.label("Damage:");
                    ui.text_edit_singleline(&mut event_editor.trap_damage);
                    ui.label("Effect (optional):");
                    ui.text_edit_singleline(&mut event_editor.trap_effect);
                }
                EventType::Sign => {
                    ui.label("Sign Text:");
                    ui.text_edit_multiline(&mut event_editor.sign_text);
                }
                EventType::NpcDialogue => {
                    ui.label("NPC ID:");
                    ui.text_edit_singleline(&mut event_editor.npc_id);
                }
            }

            ui.separator();

            let mut add_event = false;
            let mut event_to_add: Option<MapEvent> = None;
            let mut event_pos = Position::new(0, 0);

            if ui.button("➕ Add Event").clicked() {
                match event_editor.to_map_event() {
                    Ok(event) => {
                        event_pos = event_editor.position;
                        event_to_add = Some(event);
                        add_event = true;
                    }
                    Err(err) => {
                        ui.label(format!("Error: {}", err));
                    }
                }
            }

            // Apply after borrow ends
            if add_event {
                if let Some(event) = event_to_add {
                    editor.add_event(event_pos, event);
                    editor.event_editor = None;
                }
            }
        } else {
            ui.label("Click on the map to place an event");
        }
    }

    /// Show NPC editor
    fn show_npc_editor(ui: &mut egui::Ui, editor: &mut MapEditorState) {
        if let Some(ref mut npc_editor) = editor.npc_editor {
            ui.label("NPC ID:");
            ui.text_edit_singleline(&mut npc_editor.npc_id);

            ui.label("Name:");
            ui.text_edit_singleline(&mut npc_editor.name);

            ui.label("Position X:");
            ui.text_edit_singleline(&mut npc_editor.position_x);

            ui.label("Position Y:");
            ui.text_edit_singleline(&mut npc_editor.position_y);

            ui.label("Dialogue:");
            ui.text_edit_multiline(&mut npc_editor.dialogue);

            ui.separator();

            let mut add_npc = false;
            let mut npc_to_add: Option<Npc> = None;

            if ui.button("➕ Add NPC").clicked() {
                match npc_editor.to_npc() {
                    Ok(npc) => {
                        npc_to_add = Some(npc);
                        add_npc = true;
                    }
                    Err(err) => {
                        ui.label(format!("Error: {}", err));
                    }
                }
            }

            // Apply after borrow ends
            if add_npc {
                if let Some(npc) = npc_to_add {
                    editor.add_npc(npc);
                    if let Some(ref mut ed) = editor.npc_editor {
                        ed.clear();
                    }
                }
            }
        } else {
            ui.label("Click on the map to place an NPC");
        }
    }

    /// Show a small preview of a map
    fn show_map_preview(ui: &mut egui::Ui, map: &Map) {
        let tile_size = 8.0;
        let preview_width = (map.width.min(30) as f32 * tile_size).min(240.0);
        let preview_height = (map.height.min(20) as f32 * tile_size).min(160.0);

        let (response, painter) = ui.allocate_painter(
            egui::Vec2::new(preview_width, preview_height),
            egui::Sense::hover(),
        );

        let rect = response.rect;

        let scale_x = preview_width / (map.width as f32 * tile_size);
        let scale_y = preview_height / (map.height as f32 * tile_size);
        let scale = scale_x.min(scale_y);

        let actual_tile_size = tile_size * scale;

        // Draw a detailed view of the map with terrain colors
        for y in 0..map.height.min(20) {
            for x in 0..map.width.min(30) {
                let pos = Position::new(x as i32, y as i32);
                if let Some(tile) = map.get_tile(pos) {
                    // Base color from terrain type
                    let base_color = match tile.terrain {
                        TerrainType::Ground => Color32::from_rgb(160, 140, 120),
                        TerrainType::Grass => Color32::from_rgb(100, 180, 100),
                        TerrainType::Water => Color32::from_rgb(80, 120, 200),
                        TerrainType::Lava => Color32::from_rgb(220, 60, 30),
                        TerrainType::Swamp => Color32::from_rgb(90, 100, 70),
                        TerrainType::Stone => Color32::from_rgb(120, 120, 130),
                        TerrainType::Dirt => Color32::from_rgb(140, 110, 80),
                        TerrainType::Forest => Color32::from_rgb(60, 120, 60),
                        TerrainType::Mountain => Color32::from_rgb(100, 100, 110),
                    };

                    // Darken if blocked by wall
                    let color = if tile.blocked {
                        Color32::from_rgb(
                            base_color.r() / 2,
                            base_color.g() / 2,
                            base_color.b() / 2,
                        )
                    } else {
                        base_color
                    };

                    let tile_rect = Rect::from_min_size(
                        rect.min
                            + Vec2::new(x as f32 * actual_tile_size, y as f32 * actual_tile_size),
                        Vec2::new(actual_tile_size, actual_tile_size),
                    );

                    painter.rect_filled(tile_rect, 0.0, color);
                }
            }
        }

        // Draw event markers
        for (pos, _) in &map.events {
            if pos.x >= 0 && pos.x < map.width as i32 && pos.y >= 0 && pos.y < map.height as i32 {
                let marker_pos = rect.min
                    + Vec2::new(
                        pos.x as f32 * actual_tile_size + actual_tile_size / 2.0,
                        pos.y as f32 * actual_tile_size + actual_tile_size / 2.0,
                    );
                painter.circle_filled(marker_pos, actual_tile_size / 3.0, Color32::RED);
            }
        }

        // Draw NPC markers
        for npc in &map.npcs {
            let pos = &npc.position;
            if pos.x >= 0 && pos.x < map.width as i32 && pos.y >= 0 && pos.y < map.height as i32 {
                let marker_pos = rect.min
                    + Vec2::new(
                        pos.x as f32 * actual_tile_size + actual_tile_size / 2.0,
                        pos.y as f32 * actual_tile_size + actual_tile_size / 2.0,
                    );
                painter.circle_filled(marker_pos, actual_tile_size / 3.0, Color32::YELLOW);
            }
        }
    }

    /// Show import dialog window
    fn show_import_dialog_window(
        &mut self,
        ctx: &egui::Context,
        maps: &mut Vec<Map>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        let mut close_dialog = false;

        egui::Window::new("Import Map (RON)")
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.label("Paste RON data below:");
                ui.add(
                    egui::TextEdit::multiline(&mut self.import_export_buffer)
                        .desired_rows(10)
                        .desired_width(400.0),
                );

                ui.horizontal(|ui| {
                    if ui.button("Import").clicked() {
                        match ron::from_str::<Map>(&self.import_export_buffer) {
                            Ok(map) => {
                                if self.file_load_merge_mode {
                                    if let Some(existing) = maps.iter_mut().find(|m| m.id == map.id)
                                    {
                                        *existing = map;
                                    } else {
                                        maps.push(map);
                                    }
                                } else {
                                    maps.push(map);
                                }
                                *unsaved_changes = true;
                                *status_message = "Map imported successfully".to_string();
                                close_dialog = true;
                            }
                            Err(e) => {
                                *status_message = format!("Failed to parse RON: {}", e);
                            }
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        close_dialog = true;
                    }
                });
            });

        if close_dialog {
            self.show_import_dialog = false;
            self.import_export_buffer.clear();
        }
    }

    /// Save a single map to file
    fn save_map(
        &self,
        map: &Map,
        campaign_dir: Option<&PathBuf>,
        maps_dir: &str,
    ) -> Result<(), String> {
        if let Some(dir) = campaign_dir {
            let maps_path = dir.join(maps_dir);

            // Create maps directory if it doesn't exist
            fs::create_dir_all(&maps_path)
                .map_err(|e| format!("Failed to create maps directory: {}", e))?;

            let map_filename = format!("map_{}.ron", map.id);
            let map_path = maps_path.join(map_filename);

            let ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(false);

            let contents = ron::ser::to_string_pretty(map, ron_config)
                .map_err(|e| format!("Failed to serialize map: {}", e))?;

            fs::write(&map_path, contents)
                .map_err(|e| format!("Failed to write map file: {}", e))?;

            Ok(())
        } else {
            Err("No campaign directory set".to_string())
        }
    }

    /// Save all maps to files
    fn save_all_maps(
        &self,
        maps: &[Map],
        campaign_dir: Option<&PathBuf>,
        maps_dir: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        if let Some(dir) = campaign_dir {
            let maps_path = dir.join(maps_dir);

            // Create maps directory if it doesn't exist
            if let Err(e) = fs::create_dir_all(&maps_path) {
                *status_message = format!("Failed to create maps directory: {}", e);
                return;
            }

            let mut saved_count = 0;
            for map in maps {
                match self.save_map(map, campaign_dir, maps_dir) {
                    Ok(_) => saved_count += 1,
                    Err(e) => {
                        *status_message = format!("Failed to save map {}: {}", map.id, e);
                        return;
                    }
                }
            }

            *unsaved_changes = true;
            *status_message = format!("Saved {} maps", saved_count);
        } else {
            *status_message = "No campaign directory set".to_string();
        }
    }

    /// Load maps from campaign directory
    fn load_maps(
        &self,
        maps: &mut Vec<Map>,
        campaign_dir: Option<&PathBuf>,
        maps_dir: &str,
        status_message: &mut String,
    ) {
        maps.clear();

        if let Some(dir) = campaign_dir {
            let maps_path = dir.join(maps_dir);

            if maps_path.exists() && maps_path.is_dir() {
                match fs::read_dir(&maps_path) {
                    Ok(entries) => {
                        let mut loaded_count = 0;
                        for entry in entries.filter_map(|e| e.ok()) {
                            let path = entry.path();
                            if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                                match fs::read_to_string(&path) {
                                    Ok(contents) => match ron::from_str::<Map>(&contents) {
                                        Ok(mut map) => {
                                            // Backfill missing event ids for older map formats
                                            let mut next_id = map
                                                .tiles
                                                .iter()
                                                .filter_map(|t| t.event_trigger)
                                                .max()
                                                .unwrap_or(0)
                                                + 1;

                                            // Collect positions to avoid holding an immutable borrow
                                            // of `map.events` while mutably borrowing tiles later.
                                            let event_positions: Vec<Position> =
                                                map.events.keys().cloned().collect();

                                            for pos in event_positions {
                                                if let Some(tile) = map.get_tile_mut(pos) {
                                                    if tile.event_trigger.is_none() {
                                                        tile.event_trigger = Some(next_id);
                                                        next_id += 1;
                                                    }
                                                }
                                            }

                                            maps.push(map);
                                            loaded_count += 1;
                                        }
                                        Err(e) => {
                                            *status_message = format!(
                                                "Failed to parse map {:?}: {}",
                                                path.file_name().unwrap_or_default(),
                                                e
                                            );
                                        }
                                    },
                                    Err(e) => {
                                        *status_message = format!(
                                            "Failed to read map {:?}: {}",
                                            path.file_name().unwrap_or_default(),
                                            e
                                        );
                                    }
                                }
                            }
                        }

                        if loaded_count > 0 {
                            // Sort maps by ID so load order is deterministic and stable
                            // across platforms and runs. This prevents filesystem iteration
                            // order from confusing users when they expect map IDs to match.
                            maps.sort_unstable_by_key(|m| m.id);
                            *status_message = format!("Loaded {} maps", loaded_count);
                        }
                    }
                    Err(e) => {
                        *status_message = format!("Failed to read maps directory: {}", e);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_editor_state_creation() {
        let map = Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10);
        let state = MapEditorState::new(map);

        assert_eq!(state.map.id, 1);
        assert_eq!(state.map.width, 10);
        assert_eq!(state.map.height, 10);
        assert!(!state.has_changes);
        assert_eq!(state.current_tool, EditorTool::Select);
        assert!(!state.show_metadata_editor);
        assert_eq!(state.metadata.name, "Map 1");
    }

    #[test]
    fn compute_map_requested_left_small_map() {
        let total_width = 800.0;
        let inspector_min_width = 300.0;
        let sep_margin = 12.0;
        let map_pixel_width = 200.0; // map 200px wide
        let padding = 8.0;
        let computed = compute_map_requested_left(
            total_width,
            inspector_min_width,
            sep_margin,
            map_pixel_width,
            padding,
        );
        // expected: desired_left_for_map = 200 + 16 = 216; fallback = 800 - 300 - 12 = 488; result = min(216, 488) = 216
        assert_eq!(computed, 216.0);
    }

    #[test]
    fn compute_map_requested_left_fallback_when_map_too_large() {
        let total_width = 800.0;
        let inspector_min_width = 300.0;
        let sep_margin = 12.0;
        let map_pixel_width = 600.0; // map 600 px wide
        let padding = 8.0;
        let computed = compute_map_requested_left(
            total_width,
            inspector_min_width,
            sep_margin,
            map_pixel_width,
            padding,
        );
        // fallback = 488; desired = 600 + 16 = 616; result = min(616, 488) = 488
        assert_eq!(computed, 488.0);
    }

    #[test]
    fn compute_map_requested_left_no_space() {
        // total width smaller than inspector min + margin -> fallback negative
        let total_width = 200.0;
        let inspector_min_width = 220.0;
        let sep_margin = 12.0;
        let map_pixel_width = 100.0;
        let padding = 8.0;
        let computed = compute_map_requested_left(
            total_width,
            inspector_min_width,
            sep_margin,
            map_pixel_width,
            padding,
        );
        // fallback = 200 - 220 - 12 = -32
        assert_eq!(computed, -32.0);
    }

    #[test]
    fn test_set_tile_creates_undo_action() {
        let map = Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10);
        let mut state = MapEditorState::new(map);

        let pos = Position::new(5, 5);
        let tile = Tile::new(5, 5, TerrainType::Water, WallType::None);
        state.set_tile(pos, tile);

        assert!(state.has_changes);
        assert!(state.can_undo());
        assert!(!state.can_redo());
    }

    #[test]
    fn test_undo_redo_tile_change() {
        let map = Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10);
        let mut state = MapEditorState::new(map);

        let pos = Position::new(5, 5);
        let original_tile = state.map.get_tile(pos).unwrap().clone();
        let new_tile = Tile::new(5, 5, TerrainType::Water, WallType::None);

        state.set_tile(pos, new_tile.clone());
        assert_eq!(state.map.get_tile(pos).unwrap().terrain, TerrainType::Water);

        state.undo();
        assert_eq!(
            state.map.get_tile(pos).unwrap().terrain,
            original_tile.terrain
        );

        state.redo();
        assert_eq!(state.map.get_tile(pos).unwrap().terrain, TerrainType::Water);
    }

    #[test]
    fn test_paint_terrain() {
        let map = Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10);
        let mut state = MapEditorState::new(map);

        let pos = Position::new(3, 3);
        state.paint_terrain(pos, TerrainType::Forest);

        assert_eq!(
            state.map.get_tile(pos).unwrap().terrain,
            TerrainType::Forest
        );
        assert!(state.has_changes);
    }

    #[test]
    fn test_paint_wall() {
        let map = Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10);
        let mut state = MapEditorState::new(map);

        let pos = Position::new(3, 3);
        state.paint_wall(pos, WallType::Door);

        assert_eq!(state.map.get_tile(pos).unwrap().wall_type, WallType::Door);
        assert!(state.has_changes);
    }

    #[test]
    fn test_add_remove_event() {
        let map = Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10);
        let mut state = MapEditorState::new(map);

        let pos = Position::new(5, 5);
        let event = MapEvent::Sign {
            name: "Sign".to_string(),
            description: "Desc".to_string(),
            text: "Test".to_string(),
        };

        state.add_event(pos, event);
        assert!(state.map.get_event(pos).is_some());
        assert!(state.has_changes);

        state.remove_event(pos);
        assert!(state.map.get_event(pos).is_none());
    }

    #[test]
    fn test_add_remove_npc() {
        let map = Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10);
        let mut state = MapEditorState::new(map);

        let npc = Npc::new(
            1,
            "Merchant".to_string(),
            "Desc".to_string(),
            Position::new(5, 5),
            "Hello!".to_string(),
        );

        state.add_npc(npc);
        assert_eq!(state.map.npcs.len(), 1);
        assert!(state.has_changes);

        state.remove_npc(0);
        assert_eq!(state.map.npcs.len(), 0);
    }

    #[test]
    fn test_fill_region() {
        let map = Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10);
        let mut state = MapEditorState::new(map);

        let from = Position::new(2, 2);
        let to = Position::new(4, 4);

        state.fill_region(from, to, TerrainType::Stone, WallType::None);

        for y in 2..=4 {
            for x in 2..=4 {
                let pos = Position::new(x, y);
                assert_eq!(state.map.get_tile(pos).unwrap().terrain, TerrainType::Stone);
            }
        }
    }

    #[test]
    fn test_validation_events_on_blocked_tiles() {
        let map = Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10);
        let mut state = MapEditorState::new(map);

        // Place a wall
        let pos = Position::new(5, 5);
        state.paint_wall(pos, WallType::Normal);

        // Add event on blocked tile
        let event = MapEvent::Sign {
            name: "Sign".to_string(),
            description: "Desc".to_string(),
            text: "Test".to_string(),
        };
        state.add_event(pos, event);

        // Validate
        state.validate();

        assert!(!state.validation_errors.is_empty());
        assert!(state.validation_errors[0].contains("blocked tile"));
    }

    #[test]
    fn test_event_editor_state_to_encounter() {
        let editor = EventEditorState {
            event_type: EventType::Encounter,
            name: "Encounter".to_string(),
            description: "Desc".to_string(),
            encounter_monsters: vec![1, 2, 3],
            ..Default::default()
        };

        let event = editor.to_map_event().unwrap();
        match event {
            MapEvent::Encounter {
                name,
                description,
                monster_group,
            } => {
                assert_eq!(monster_group, vec![1u8, 2u8, 3u8]);
            }
            _ => panic!("expected encounter event"),
        }
    }

    #[test]
    fn test_event_editor_state_to_sign() {
        let editor = EventEditorState {
            event_type: EventType::Sign,
            name: "Sign".to_string(),
            description: "Desc".to_string(),
            sign_text: "Hello World".to_string(),
            ..Default::default()
        };

        let event = editor.to_map_event().unwrap();
        match event {
            MapEvent::Sign { text, .. } => {
                assert_eq!(text, "Hello World".to_string());
            }
            _ => panic!("Expected Sign event"),
        }
    }

    #[test]
    fn test_event_editor_state_to_teleport_with_selected_fallback() {
        let editor = EventEditorState {
            event_type: EventType::Teleport,
            name: "Teleport".to_string(),
            description: "Desc".to_string(),
            // user didn't type numeric coordinates, selection fallback will be used
            teleport_x: String::new(),
            teleport_y: String::new(),
            teleport_map_id: String::new(),
            teleport_selected_map: Some(7),
            teleport_selected_pos: Some(Position::new(4, 5)),
            ..Default::default()
        };

        let event = editor.to_map_event().unwrap();
        match event {
            MapEvent::Teleport {
                destination,
                map_id,
                ..
            } => {
                assert_eq!(destination, Position::new(4, 5));
                assert_eq!(map_id, 7);
            }
            _ => panic!("Expected Teleport event"),
        }
    }

    #[test]
    fn test_event_editor_state_to_teleport_invalid_no_fallback() {
        let editor = EventEditorState {
            event_type: EventType::Teleport,
            name: "Teleport".to_string(),
            description: "Desc".to_string(),
            // no numeric coordinates and no preview selection -> should fail
            teleport_x: String::new(),
            teleport_y: String::new(),
            teleport_map_id: String::new(),
            teleport_selected_map: None,
            teleport_selected_pos: None,
            ..Default::default()
        };

        assert!(editor.to_map_event().is_err());
    }

    #[test]
    fn test_npc_editor_state_to_npc() {
        let editor = NpcEditorState {
            npc_id: "42".to_string(),
            name: "Guard".to_string(),
            description: "Desc".to_string(),
            position_x: "10".to_string(),
            position_y: "15".to_string(),
            dialogue: "Halt!".to_string(),
        };

        let npc = editor.to_npc().unwrap();
        assert_eq!(npc.id, 42);
        assert_eq!(npc.name, "Guard");
        assert_eq!(npc.position, Position::new(10, 15));
        assert_eq!(npc.dialogue, "Halt!");
    }

    #[test]
    fn test_editor_tool_names() {
        assert_eq!(EditorTool::Select.name(), "Select");
        assert_eq!(EditorTool::PaintTile.name(), "Paint Tile");
        assert_eq!(EditorTool::PlaceEvent.name(), "Place Event");
    }

    #[test]
    fn test_event_type_all() {
        let types = EventType::all();
        assert_eq!(types.len(), 6);
        assert!(types.contains(&EventType::Encounter));
        assert!(types.contains(&EventType::Treasure));
        assert!(types.contains(&EventType::Sign));
    }

    #[test]
    fn test_save_to_ron() {
        let mut state =
            MapEditorState::new(Map::new(1, "Map 1".to_string(), "Desc".to_string(), 5, 5));
        state.selected_terrain = TerrainType::Grass;
        state.paint_tile(Position { x: 0, y: 0 });

        let ron = state.save_to_ron().unwrap();
        assert!(ron.contains("id:"));
        assert!(ron.contains("width:"));
        assert!(ron.contains("height:"));
    }

    #[test]
    fn test_metadata_editor() {
        let mut state =
            MapEditorState::new(Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10));

        state.metadata.name = "Test Map".to_string();
        state.metadata.description = "A test map".to_string();
        state.metadata.difficulty = 5;
        state.metadata.light_level = 80;
        state.metadata.encounter_rate = 25;
        state.metadata.music_track = "dungeon.ogg".to_string();
        state.metadata.is_outdoor = true;

        assert_eq!(state.metadata.name, "Test Map");
        assert_eq!(state.metadata.difficulty, 5);
        assert_eq!(state.metadata.light_level, 80);
    }

    #[test]
    fn test_apply_metadata_sync() {
        let mut state =
            MapEditorState::new(Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10));

        state.metadata.name = "Renamed Map".to_string();
        state.metadata.description = "A new description".to_string();

        // apply metadata to underlying map
        state.apply_metadata();

        assert_eq!(state.map.name, "Renamed Map");
        assert_eq!(state.map.description, "A new description");
    }

    #[test]
    fn test_save_action_updates_maps_vector() {
        let mut maps: Vec<Map> = vec![Map::new(
            1,
            "You Forgot Map".to_string(),
            "Desc".to_string(),
            10,
            10,
        )];
        let mut maps_editor = MapsEditorState::new();

        // Activate editor for the first map
        maps_editor.selected_map_idx = Some(0);
        maps_editor.active_editor = Some(MapEditorState::new(maps[0].clone()));

        // Modify metadata
        if let Some(ref mut editor) = maps_editor.active_editor {
            editor.metadata.name = "Synchronized Map".to_string();
            editor.metadata.description = "Synchronized description".to_string();
            editor.apply_metadata();

            let map = editor.map.clone();
            if let Some(idx) = maps_editor.selected_map_idx {
                if idx < maps.len() {
                    maps[idx] = map.clone();
                }
            }
        }

        assert_eq!(maps[0].name, "Synchronized Map");
        assert_eq!(maps[0].description, "Synchronized description");
    }

    #[test]
    fn test_load_maps_sorts_by_id() {
        use std::fs;
        use tempfile::tempdir;

        // Create a temporary campaign directory with a `maps` subdirectory
        let tmpdir = tempdir().expect("Failed to create tempdir");
        let campaign_dir_buf = tmpdir.path().to_path_buf();
        let maps_dir = "maps";
        let maps_path = tmpdir.path().join(maps_dir);
        fs::create_dir_all(&maps_path).expect("Failed to create maps dir");

        // Create three maps with different IDs and write in non-sorted order (3,1,2)
        let map3 = Map::new(3, "Map 3".to_string(), "Desc".to_string(), 10, 10);
        let map1 = Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10);
        let map2 = Map::new(2, "Map 2".to_string(), "Desc".to_string(), 10, 10);

        // Use ron pretty default config to serialize maps
        let ron_cfg = ron::ser::PrettyConfig::default();

        fs::write(
            maps_path.join("map_3.ron"),
            ron::ser::to_string_pretty(&map3, ron_cfg.clone()).expect("Serialize map3"),
        )
        .expect("Write map_3");

        fs::write(
            maps_path.join("map_1.ron"),
            ron::ser::to_string_pretty(&map1, ron_cfg.clone()).expect("Serialize map1"),
        )
        .expect("Write map_1");

        fs::write(
            maps_path.join("map_2.ron"),
            ron::ser::to_string_pretty(&map2, ron_cfg.clone()).expect("Serialize map2"),
        )
        .expect("Write map_2");

        // Now load maps using the editor function and ensure they are sorted by id
        let mut loaded_maps: Vec<Map> = Vec::new();
        let mut status_message = String::new();
        let state = MapsEditorState::new();
        state.load_maps(
            &mut loaded_maps,
            Some(&campaign_dir_buf),
            maps_dir,
            &mut status_message,
        );

        assert_eq!(loaded_maps.len(), 3);
        assert_eq!(loaded_maps[0].id, 1);
        assert_eq!(loaded_maps[1].id, 2);
        assert_eq!(loaded_maps[2].id, 3);
    }

    #[test]
    fn test_undo_redo_event_id_preserved() {
        // Should preserve tile.event_trigger across add -> undo -> redo
        let mut state = MapEditorState::new(Map::new(
            1,
            "UndoRedo Map".to_string(),
            "Desc".to_string(),
            10,
            10,
        ));
        let pos = Position::new(3, 3);
        let event = MapEvent::Sign {
            name: "Sign".to_string(),
            description: "Desc".to_string(),
            text: "Hello UndoRedo".to_string(),
        };

        // Add event and check tile has an event id
        state.add_event(pos, event);
        let id_opt = state.map.get_tile(pos).unwrap().event_trigger;
        assert!(id_opt.is_some());
        let assigned_id = id_opt.unwrap();

        // Undo -> event removed and id cleared
        state.undo();
        assert!(state.map.get_event(pos).is_none());
        assert!(state.map.get_tile(pos).unwrap().event_trigger.is_none());

        // Redo -> event restored and id restored
        state.redo();
        assert!(state.map.get_event(pos).is_some());
        assert_eq!(
            state.map.get_tile(pos).unwrap().event_trigger,
            Some(assigned_id)
        );
    }

    #[test]
    fn test_load_maps_backfills_event_ids() {
        use std::fs;
        use tempfile::tempdir;

        // Create a temporary campaign directory with a `maps` subdirectory
        let tmpdir = tempdir().expect("Failed to create tempdir");
        let campaign_dir_buf = tmpdir.path().to_path_buf();
        let maps_dir = "maps";
        let maps_path = tmpdir.path().join(maps_dir);
        fs::create_dir_all(&maps_path).expect("Failed to create maps dir");

        // Build a map in which we add an event but DO NOT set tile.event_trigger
        let mut map = Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);
        map.add_event(
            pos,
            MapEvent::Sign {
                name: "Sign".to_string(),
                description: "Desc".to_string(),
                text: "Test sign".to_string(),
            },
        );
        // Intentionally keep event_trigger None (simulate older map files)
        assert!(map.get_tile(pos).unwrap().event_trigger.is_none());

        // Write it out to a file
        let ron_cfg = ron::ser::PrettyConfig::default();
        fs::write(
            maps_path.join("map_noids.ron"),
            ron::ser::to_string_pretty(&map, ron_cfg).expect("Serialize map"),
        )
        .expect("Write map");

        // Load via MapsEditorState::load_maps - it should backfill event IDs
        let mut loaded_maps: Vec<Map> = Vec::new();
        let mut status_message = String::new();
        let state = MapsEditorState::new();
        state.load_maps(
            &mut loaded_maps,
            Some(&campaign_dir_buf),
            maps_dir,
            &mut status_message,
        );

        assert_eq!(loaded_maps.len(), 1);
        let loaded = &loaded_maps[0];
        // the tile for the event must now have an assigned event_trigger
        let tid = loaded.get_tile(pos).unwrap().event_trigger;
        assert!(tid.is_some());
    }

    #[test]
    fn test_build_filtered_maps_snapshot_sorts_by_id() {
        // Create maps in non-sorted order (3, 1, 2)
        let maps = vec![
            Map::new(3, "Map 3".to_string(), "Desc".to_string(), 10, 10),
            Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10),
            Map::new(2, "Map 2".to_string(), "Desc".to_string(), 10, 10),
        ];

        let snapshot = MapsEditorState::build_filtered_maps_snapshot(&maps, "");
        assert_eq!(snapshot.len(), 3);

        // Snapshot sorted by ID (1,2,3)
        assert_eq!(snapshot[0].1, 1);
        assert_eq!(snapshot[1].1, 2);
        assert_eq!(snapshot[2].1, 3);

        // Original indices preserved in snapshot mapping
        assert_eq!(snapshot[0].0, 1); // map ID 1 was originally at index 1
        assert_eq!(snapshot[1].0, 2); // map ID 2 was originally at index 2
        assert_eq!(snapshot[2].0, 0); // map ID 3 was originally at index 0
    }

    #[test]
    fn test_suggest_maps_for_partial() {
        // Create a small set of maps to test suggestion behavior
        let maps = vec![
            Map::new(1, "Starter Town".to_string(), "Desc".to_string(), 10, 10),
            Map::new(2, "Dark Forest".to_string(), "Desc".to_string(), 10, 10),
            Map::new(3, "Ancient Ruins".to_string(), "Desc".to_string(), 10, 10),
        ];

        // Partial name match
        let results = suggest_maps_for_partial(&maps, "Dark");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 2);
        assert!(results[0].1.to_lowercase().contains("dark"));

        // Partial id match
        let results = suggest_maps_for_partial(&maps, "1");
        assert!(!results.is_empty());
        assert!(results.iter().any(|(id, _)| *id == 1));

        // Partial lowercase name fragment
        let results = suggest_maps_for_partial(&maps, "anc");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 3);
    }

    #[test]
    fn test_add_event_at_position() {
        let mut state =
            MapEditorState::new(Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10));
        let event = MapEvent::Sign {
            name: "Sign".to_string(),
            description: "Desc".to_string(),
            text: "Test sign".to_string(),
        };

        state.add_event_at_position(5, 5, event);

        let pos = Position { x: 5, y: 5 };
        assert!(state.map.events.contains_key(&pos));
    }

    #[test]
    fn test_show_event_editor_ui() {
        let mut state =
            MapEditorState::new(Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10));
        assert!(!state.show_event_editor_ui());

        state.event_editor = Some(EventEditorState::default());
        assert!(state.show_event_editor_ui());
    }

    #[test]
    fn test_map_preview_with_terrain_types() {
        let map = Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10);
        let mut state = MapEditorState::new(map.clone());

        // Paint different terrain types
        state.selected_terrain = TerrainType::Grass;
        state.paint_tile(Position { x: 0, y: 0 });
        state.selected_terrain = TerrainType::Water;
        state.paint_tile(Position { x: 1, y: 0 });
        state.selected_terrain = TerrainType::Stone;
        state.paint_tile(Position { x: 2, y: 0 });

        // Verify terrain was set
        assert_eq!(
            state.map.get_tile(Position { x: 0, y: 0 }).unwrap().terrain,
            TerrainType::Grass
        );
        assert_eq!(
            state.map.get_tile(Position { x: 1, y: 0 }).unwrap().terrain,
            TerrainType::Water
        );
        assert_eq!(
            state.map.get_tile(Position { x: 2, y: 0 }).unwrap().terrain,
            TerrainType::Stone
        );
    }

    #[test]
    fn test_map_preview_with_events() {
        let mut state =
            MapEditorState::new(Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10));

        // Add events at different positions
        let event1 = MapEvent::Sign {
            name: "Sign".to_string(),
            description: "Desc".to_string(),
            text: "Welcome".to_string(),
        };
        let event2 = MapEvent::Treasure {
            name: "Treasure".to_string(),
            description: "Desc".to_string(),
            loot: vec![1, 2, 3],
        };

        state.add_event_at_position(5, 5, event1);
        state.add_event_at_position(7, 7, event2);

        // Verify events are in the map
        assert!(state.map.events.contains_key(&Position { x: 5, y: 5 }));
        assert!(state.map.events.contains_key(&Position { x: 7, y: 7 }));
        assert_eq!(state.map.events.len(), 2);
    }

    #[test]
    fn test_tile_painting_with_undo() {
        let mut state =
            MapEditorState::new(Map::new(1, "Map 1".to_string(), "Desc".to_string(), 5, 5));

        // Paint a tile
        state.selected_terrain = TerrainType::Lava;
        state.paint_tile(Position { x: 2, y: 2 });
        assert_eq!(
            state.map.get_tile(Position { x: 2, y: 2 }).unwrap().terrain,
            TerrainType::Lava
        );

        // Undo
        state.undo();
        assert_eq!(
            state.map.get_tile(Position { x: 2, y: 2 }).unwrap().terrain,
            TerrainType::Ground
        );

        // Redo
        state.redo();
        assert_eq!(
            state.map.get_tile(Position { x: 2, y: 2 }).unwrap().terrain,
            TerrainType::Lava
        );
    }

    #[test]
    fn test_event_placement_tool() {
        let mut state =
            MapEditorState::new(Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10));

        // Initially no event editor
        assert!(state.event_editor.is_none());

        // Select place event tool
        state.current_tool = EditorTool::PlaceEvent;

        // Simulate placing event at position (manually initialize editor)
        state.event_editor = Some(EventEditorState {
            position: Position { x: 3, y: 3 },
            event_type: EventType::Sign,
            sign_text: "Test".to_string(),
            ..Default::default()
        });

        // Create and add the event
        if let Some(ref editor) = state.event_editor {
            if let Ok(event) = editor.to_map_event() {
                state.add_event(editor.position, event);
            }
        }

        // Verify event was added
        assert!(state.map.events.contains_key(&Position { x: 3, y: 3 }));
    }

    #[test]
    fn test_maps_editor_state_creation() {
        let state = MapsEditorState::new();
        assert_eq!(state.mode, MapsEditorMode::List);
        assert!(state.search_filter.is_empty());
        assert!(state.selected_map_idx.is_none());
        assert!(state.active_editor.is_none());
    }

    #[test]
    fn test_next_available_map_id() {
        let maps = vec![
            Map::new(1, "Map 1".to_string(), "Desc".to_string(), 10, 10),
            Map::new(5, "Map 5".to_string(), "Desc".to_string(), 10, 10),
            Map::new(3, "Map 3".to_string(), "Desc".to_string(), 10, 10),
        ];

        let next_id = MapsEditorState::next_available_map_id(&maps);
        assert_eq!(next_id, 6);
    }

    #[test]
    fn test_next_available_map_id_empty() {
        let maps: Vec<Map> = vec![];
        let next_id = MapsEditorState::next_available_map_id(&maps);
        assert_eq!(next_id, 1);
    }

    #[test]
    fn test_editor_tool_all() {
        let tools = EditorTool::all();
        assert_eq!(tools.len(), 6);
        assert!(tools.contains(&EditorTool::Select));
        assert!(tools.contains(&EditorTool::PaintTile));
        assert!(tools.contains(&EditorTool::PlaceEvent));
        assert!(tools.contains(&EditorTool::PlaceNpc));
        assert!(tools.contains(&EditorTool::Fill));
        assert!(tools.contains(&EditorTool::Erase));
    }

    #[test]
    fn test_maps_editor_mode() {
        let mut state = MapsEditorState::new();

        state.mode = MapsEditorMode::Add;
        assert_eq!(state.mode, MapsEditorMode::Add);

        state.mode = MapsEditorMode::Edit;
        assert_eq!(state.mode, MapsEditorMode::Edit);

        state.mode = MapsEditorMode::List;
        assert_eq!(state.mode, MapsEditorMode::List);
    }
}
