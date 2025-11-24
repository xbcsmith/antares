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
//!
//! # Architecture
//!
//! The map editor is split into:
//! - `MapEditorState`: Pure state management (separate from UI)
//! - `MapEditorWidget`: egui rendering component
//! - Tool palette for tile/event/NPC placement
//!
//! # Usage
//!
//! ```rust
//! use antares::domain::world::Map;
//! use campaign_builder::map_editor::{MapEditorState, MapEditorWidget};
//!
//! let mut state = MapEditorState::new(Map::new(1, 20, 20));
//! let widget = MapEditorWidget::new(&mut state);
//!
//! // In egui update loop:
//! ui.add(widget);
//! ```

use antares::domain::types::{MapId, Position};
use antares::domain::world::{Map, MapEvent, Npc, TerrainType, Tile, WallType};
use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};
use std::path::PathBuf;

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
    EventAdded { position: Position, event: MapEvent },
    /// Event was removed
    EventRemoved { position: Position, event: MapEvent },
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

// ===== Map Editor State =====

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
            name: format!("Map {}", map.id),
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
                    let new_tile = Tile::new(terrain, wall);
                    self.set_tile(pos, new_tile);
                }
            }
        }
    }

    /// Erases a tile (resets to default)
    pub fn erase_tile(&mut self, pos: Position) {
        let default_tile = Tile::new(TerrainType::Ground, WallType::None);
        self.set_tile(pos, default_tile);
    }

    /// Adds an event at position
    pub fn add_event(&mut self, pos: Position, event: MapEvent) {
        if !self.map.is_valid_position(pos) {
            return;
        }

        self.map.add_event(pos, event.clone());
        self.undo_stack.push(EditorAction::EventAdded {
            position: pos,
            event,
        });
        self.has_changes = true;
    }

    /// Removes event at position
    pub fn remove_event(&mut self, pos: Position) {
        if let Some(event) = self.map.remove_event(pos) {
            self.undo_stack.push(EditorAction::EventRemoved {
                position: pos,
                event,
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
            }
            EditorAction::EventRemoved { position, event } => {
                self.map.add_event(position, event);
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
            EditorAction::EventAdded { position, event } => {
                self.map.add_event(position, event);
            }
            EditorAction::EventRemoved { position, .. } => {
                self.map.remove_event(position);
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
                .push("⚠️ Warning: Map has no events or NPCs".to_string());
        }

        // Check for unreachable events
        for (pos, _) in &self.map.events {
            if self.map.is_blocked(*pos) {
                self.validation_errors.push(format!(
                    "❌ Error: Event at ({}, {}) is on a blocked tile",
                    pos.x, pos.y
                ));
            }
        }

        // Check for NPCs on blocked tiles
        for npc in &self.map.npcs {
            if self.map.is_blocked(npc.position) {
                self.validation_errors.push(format!(
                    "❌ Error: NPC '{}' at ({}, {}) is on a blocked tile",
                    npc.name, npc.position.x, npc.position.y
                ));
            }
        }

        // Check connections
        for conn in &self.metadata.connections {
            if !self.map.is_valid_position(conn.from_position) {
                self.validation_errors.push(format!(
                    "❌ Error: Connection '{}' has invalid position",
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
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{Map, MapEvent};
    /// use campaign_builder::map_editor::MapEditorState;
    ///
    /// let mut state = MapEditorState::new(Map::new(1, 10, 10));
    /// // Add event implementation
    /// ```
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
    // Encounter fields
    pub encounter_monsters: String,
    // Treasure fields
    pub treasure_items: String,
    // Teleport fields
    pub teleport_x: String,
    pub teleport_y: String,
    pub teleport_map_id: String,
    // Trap fields
    pub trap_damage: String,
    pub trap_effect: String,
    // Sign fields
    pub sign_text: String,
    // NPC dialogue fields
    pub npc_id: String,
}

impl Default for EventEditorState {
    fn default() -> Self {
        Self {
            event_type: EventType::default(),
            position: Position::new(0, 0),
            encounter_monsters: String::new(),
            treasure_items: String::new(),
            teleport_x: String::new(),
            teleport_y: String::new(),
            teleport_map_id: String::new(),
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
                let monsters: Vec<u8> = self
                    .encounter_monsters
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if monsters.is_empty() {
                    return Err("Encounter must have at least one monster ID".to_string());
                }
                Ok(MapEvent::Encounter {
                    monster_group: monsters,
                })
            }
            EventType::Treasure => {
                let loot: Vec<u8> = self
                    .treasure_items
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                Ok(MapEvent::Treasure { loot })
            }
            EventType::Teleport => {
                let x = self
                    .teleport_x
                    .parse()
                    .map_err(|_| "Invalid X coordinate")?;
                let y = self
                    .teleport_y
                    .parse()
                    .map_err(|_| "Invalid Y coordinate")?;
                let map_id = self.teleport_map_id.parse().map_err(|_| "Invalid map ID")?;
                Ok(MapEvent::Teleport {
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
                Ok(MapEvent::Trap { damage, effect })
            }
            EventType::Sign => {
                if self.sign_text.is_empty() {
                    return Err("Sign text cannot be empty".to_string());
                }
                Ok(MapEvent::Sign {
                    text: self.sign_text.clone(),
                })
            }
            EventType::NpcDialogue => {
                let npc_id = self.npc_id.parse().map_err(|_| "Invalid NPC ID")?;
                Ok(MapEvent::NpcDialogue { npc_id })
            }
        }
    }
}

// ===== NPC Editor State =====

/// NPC editor state
#[derive(Debug, Clone, Default)]
pub struct NpcEditorState {
    pub npc_id: String,
    pub name: String,
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
            Position::new(x, y),
            self.dialogue.clone(),
        ))
    }

    pub fn clear(&mut self) {
        self.npc_id.clear();
        self.name.clear();
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

        if has_event {
            return Color32::from_rgb(255, 100, 100); // Red for events
        }

        if tile.wall_type != WallType::None {
            return match tile.wall_type {
                WallType::Normal => Color32::from_rgb(80, 80, 80),
                WallType::Door => Color32::from_rgb(139, 69, 19),
                WallType::Torch => Color32::from_rgb(255, 165, 0),
                WallType::None => Color32::WHITE,
            };
        }

        match tile.terrain {
            TerrainType::Ground => Color32::from_rgb(210, 180, 140), // Tan
            TerrainType::Grass => Color32::from_rgb(50, 205, 50),    // Lime Green
            TerrainType::Water => Color32::from_rgb(30, 144, 255),   // Dodger Blue
            TerrainType::Lava => Color32::from_rgb(255, 69, 0),      // Red-Orange
            TerrainType::Swamp => Color32::from_rgb(85, 107, 47),    // Dark Olive Green
            TerrainType::Stone => Color32::from_rgb(169, 169, 169),  // Dark Gray
            TerrainType::Dirt => Color32::from_rgb(139, 69, 19),     // Saddle Brown
            TerrainType::Forest => Color32::from_rgb(34, 139, 34),   // Forest Green
            TerrainType::Mountain => Color32::from_rgb(105, 105, 105), // Dim Gray
        }
    }
}

impl<'a> Widget for MapGridWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let width = self.state.map.width as f32 * self.tile_size;
        let height = self.state.map.height as f32 * self.tile_size;
        let (response, painter) =
            ui.allocate_painter(Vec2::new(width, height), Sense::click_and_drag());

        let to_screen = |x: i32, y: i32| -> Pos2 {
            response.rect.min + Vec2::new(x as f32 * self.tile_size, y as f32 * self.tile_size)
        };

        // Draw tiles
        for y in 0..self.state.map.height as i32 {
            for x in 0..self.state.map.width as i32 {
                let pos = Position::new(x, y);
                if let Some(tile) = self.state.map.get_tile(pos) {
                    let has_event = self.state.map.events.contains_key(&pos);
                    let has_npc = self.state.map.npcs.iter().any(|npc| npc.position == pos);

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
                            Stroke::new(1.0, Color32::from_gray(100)),
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
                let local_pos = click_pos - response.rect.min;
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

// ===== Map Editor Widget (Main Component) =====

/// Main map editor widget
pub struct MapEditorWidget<'a> {
    state: &'a mut MapEditorState,
}

impl<'a> MapEditorWidget<'a> {
    pub fn new(state: &'a mut MapEditorState) -> Self {
        Self { state }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("🗺️ Map Editor");
            ui.separator();
            ui.label(format!("Map ID: {}", self.state.map.id));
            ui.separator();
            ui.label(format!(
                "Size: {}x{}",
                self.state.map.width, self.state.map.height
            ));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.state.has_changes {
                    ui.label("●").on_hover_text("Unsaved changes");
                }

                if ui
                    .button("💾 Save")
                    .on_hover_text("Save map to file")
                    .clicked()
                {
                    // Save will be handled by parent component
                }

                if ui
                    .button("🔄 Validate")
                    .on_hover_text("Validate map")
                    .clicked()
                {
                    self.state.validate();
                }
            });
        });

        ui.separator();

        // Tool palette
        self.show_tool_palette(ui);

        ui.separator();

        // Main content area
        egui::SidePanel::right("map_inspector")
            .default_width(300.0)
            .show_inside(ui, |ui| {
                self.show_inspector_panel(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.add(MapGridWidget::new(self.state).tile_size(24.0));
            });
        });
    }

    fn show_tool_palette(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Tools:");

            if ui
                .selectable_label(
                    matches!(self.state.current_tool, EditorTool::Select),
                    format!("{} Select", EditorTool::Select.icon()),
                )
                .clicked()
            {
                self.state.current_tool = EditorTool::Select;
            }

            if ui
                .selectable_label(
                    matches!(self.state.current_tool, EditorTool::PaintTile),
                    format!("{} Paint", EditorTool::PaintTile.icon()),
                )
                .clicked()
            {
                self.state.current_tool = EditorTool::PaintTile;
            }

            ui.separator();

            if ui
                .selectable_label(
                    matches!(self.state.current_tool, EditorTool::PlaceEvent),
                    format!("{} Event", EditorTool::PlaceEvent.icon()),
                )
                .clicked()
            {
                self.state.current_tool = EditorTool::PlaceEvent;
            }

            if ui
                .selectable_label(
                    matches!(self.state.current_tool, EditorTool::PlaceNpc),
                    format!("{} NPC", EditorTool::PlaceNpc.icon()),
                )
                .clicked()
            {
                self.state.current_tool = EditorTool::PlaceNpc;
            }

            ui.separator();

            if ui
                .selectable_label(
                    matches!(self.state.current_tool, EditorTool::Erase),
                    format!("{} Erase", EditorTool::Erase.icon()),
                )
                .clicked()
            {
                self.state.current_tool = EditorTool::Erase;
            }

            ui.separator();

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(self.state.can_redo(), egui::Button::new("↪ Redo"))
                    .clicked()
                {
                    self.state.redo();
                }

                if ui
                    .add_enabled(self.state.can_undo(), egui::Button::new("↩ Undo"))
                    .clicked()
                {
                    self.state.undo();
                }
            });
        });

        // Terrain and wall selection (separate row)
        ui.horizontal(|ui| {
            ui.label("Terrain:");

            egui::ComboBox::from_id_salt("map_terrain_palette_combo")
                .selected_text(format!("{:?}", self.state.selected_terrain))
                .show_ui(ui, |ui| {
                    for terrain in &[
                        TerrainType::Ground,
                        TerrainType::Grass,
                        TerrainType::Water,
                        TerrainType::Stone,
                        TerrainType::Dirt,
                        TerrainType::Forest,
                        TerrainType::Mountain,
                        TerrainType::Swamp,
                        TerrainType::Lava,
                    ] {
                        ui.selectable_value(
                            &mut self.state.selected_terrain,
                            *terrain,
                            format!("{:?}", terrain),
                        );
                    }
                });

            ui.separator();
            ui.label("Wall:");

            egui::ComboBox::from_id_salt("map_wall_palette_combo")
                .selected_text(format!("{:?}", self.state.selected_wall))
                .show_ui(ui, |ui| {
                    for wall in &[
                        WallType::None,
                        WallType::Normal,
                        WallType::Door,
                        WallType::Torch,
                    ] {
                        ui.selectable_value(
                            &mut self.state.selected_wall,
                            *wall,
                            format!("{:?}", wall),
                        );
                    }
                });
        });

        // View options
        ui.horizontal(|ui| {
            ui.label("View:");
            ui.checkbox(&mut self.state.show_grid, "Grid");
            ui.checkbox(&mut self.state.show_events, "Events");
            ui.checkbox(&mut self.state.show_npcs, "NPCs");
        });
    }

    fn show_inspector_panel(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Inspector");
            ui.separator();

            // Map metadata button
            if ui.button("🗺️ Edit Map Metadata").clicked() {
                self.state.show_metadata_editor = !self.state.show_metadata_editor;
            }

            if self.state.show_metadata_editor {
                ui.separator();
                self.show_metadata_editor_panel(ui);
                ui.separator();
            }

            // Map info display
            ui.group(|ui| {
                ui.label(format!("Map ID: {}", self.state.map.id));
                ui.label(format!(
                    "Size: {}×{}",
                    self.state.map.width, self.state.map.height
                ));
                ui.label(format!("Name: {}", self.state.metadata.name));
                if !self.state.metadata.description.is_empty() {
                    ui.label(format!("Description: {}", self.state.metadata.description));
                }
            });

            ui.separator();

            // Selected tile info
            if let Some(pos) = self.state.selected_position {
                ui.group(|ui| {
                    ui.label(format!("Position: ({}, {})", pos.x, pos.y));

                    if let Some(tile) = self.state.map.get_tile(pos) {
                        ui.label(format!("Terrain: {:?}", tile.terrain));
                        ui.label(format!("Wall: {:?}", tile.wall_type));
                        ui.label(format!("Blocked: {}", tile.blocked));
                        ui.label(format!("Visited: {}", tile.visited));
                    }

                    if let Some(npc) = self.state.map.npcs.iter().find(|n| n.position == pos) {
                        ui.separator();
                        ui.label("NPC:");
                        ui.label(format!("Name: {}", npc.name));
                        ui.label(format!("ID: {}", npc.id));
                        ui.label(format!("Dialogue: {}", npc.dialogue));
                    }

                    if let Some(event) = self.state.map.get_event(pos) {
                        ui.separator();
                        ui.label("Event:");
                        match event {
                            MapEvent::Encounter { monster_group } => {
                                ui.label(format!("Encounter: {:?}", monster_group));
                            }
                            MapEvent::Treasure { loot } => {
                                ui.label(format!("Treasure: {:?}", loot));
                            }
                            MapEvent::Teleport {
                                destination,
                                map_id,
                            } => {
                                ui.label(format!(
                                    "Teleport to map {} at ({}, {})",
                                    map_id, destination.x, destination.y
                                ));
                            }
                            MapEvent::Trap { damage, effect } => {
                                ui.label(format!("Trap: {} damage", damage));
                                if let Some(eff) = effect {
                                    ui.label(format!("Effect: {}", eff));
                                }
                            }
                            MapEvent::Sign { text } => {
                                ui.label(format!("Sign: {}", text));
                            }
                            MapEvent::NpcDialogue { npc_id } => {
                                ui.label(format!("NPC Dialogue: {}", npc_id));
                            }
                        }

                        if ui.button("🗑 Remove Event").clicked() {
                            self.state.remove_event(pos);
                        }
                    }
                });
            } else {
                ui.label("No tile selected");
            }

            ui.add_space(10.0);

            // Event editor
            if matches!(self.state.current_tool, EditorTool::PlaceEvent) {
                ui.group(|ui| {
                    ui.heading("Event Editor");
                    self.show_event_editor(ui);
                });
            }

            // NPC editor
            if matches!(self.state.current_tool, EditorTool::PlaceNpc) {
                ui.group(|ui| {
                    ui.heading("NPC Editor");
                    self.show_npc_editor(ui);
                });
            }

            ui.add_space(10.0);

            // Map statistics
            ui.group(|ui| {
                ui.heading("Statistics");
                ui.label(format!("Events: {}", self.state.map.events.len()));
                ui.label(format!("NPCs: {}", self.state.map.npcs.len()));
            });

            // Validation errors
            if !self.state.validation_errors.is_empty() {
                ui.add_space(10.0);
                ui.group(|ui| {
                    ui.heading("Validation");
                    for error in &self.state.validation_errors {
                        ui.label(error);
                    }
                });
            }
        });
    }

    fn show_event_editor(&mut self, ui: &mut Ui) {
        if let Some(ref mut event_editor) = self.state.event_editor {
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

            match event_editor.event_type {
                EventType::Encounter => {
                    ui.label("Monster IDs (comma-separated):");
                    ui.text_edit_singleline(&mut event_editor.encounter_monsters);
                }
                EventType::Treasure => {
                    ui.label("Item IDs (comma-separated):");
                    ui.text_edit_singleline(&mut event_editor.treasure_items);
                }
                EventType::Teleport => {
                    ui.label("Destination X:");
                    ui.text_edit_singleline(&mut event_editor.teleport_x);
                    ui.label("Destination Y:");
                    ui.text_edit_singleline(&mut event_editor.teleport_y);
                    ui.label("Target Map ID:");
                    ui.text_edit_singleline(&mut event_editor.teleport_map_id);
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

            if ui.button("➕ Add Event").clicked() {
                match event_editor.to_map_event() {
                    Ok(event) => {
                        let pos = event_editor.position;
                        self.state.add_event(pos, event);
                        self.state.event_editor = None;
                    }
                    Err(err) => {
                        // Show error (in real implementation, use proper error handling)
                        println!("Error creating event: {}", err);
                    }
                }
            }
        }
    }

    fn show_metadata_editor_panel(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.heading("Map Metadata");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Name:");
                if ui
                    .text_edit_singleline(&mut self.state.metadata.name)
                    .changed()
                {
                    self.state.has_changes = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Description:");
            });
            if ui
                .text_edit_multiline(&mut self.state.metadata.description)
                .changed()
            {
                self.state.has_changes = true;
            }

            ui.horizontal(|ui| {
                ui.label("Difficulty:");
                if ui
                    .add(egui::Slider::new(
                        &mut self.state.metadata.difficulty,
                        1..=10,
                    ))
                    .changed()
                {
                    self.state.has_changes = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Light Level:");
                if ui
                    .add(egui::Slider::new(
                        &mut self.state.metadata.light_level,
                        0..=100,
                    ))
                    .changed()
                {
                    self.state.has_changes = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Encounter Rate:");
                if ui
                    .add(egui::Slider::new(
                        &mut self.state.metadata.encounter_rate,
                        0..=100,
                    ))
                    .changed()
                {
                    self.state.has_changes = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Music Track:");
                if ui
                    .text_edit_singleline(&mut self.state.metadata.music_track)
                    .changed()
                {
                    self.state.has_changes = true;
                }
            });

            if ui
                .checkbox(&mut self.state.metadata.is_outdoor, "Outdoor Map")
                .changed()
            {
                self.state.has_changes = true;
            }

            ui.separator();

            if ui.button("Close").clicked() {
                self.state.show_metadata_editor = false;
            }
        });
    }

    fn show_npc_editor(&mut self, ui: &mut Ui) {
        let mut should_add = false;
        let mut npc_result = None;
        let mut should_clear = false;

        if let Some(ref mut npc_editor) = self.state.npc_editor {
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

            if ui.button("➕ Add NPC").clicked() {
                match npc_editor.to_npc() {
                    Ok(npc) => {
                        npc_result = Some(npc);
                        should_add = true;
                        should_clear = true;
                    }
                    Err(err) => {
                        println!("Error creating NPC: {}", err);
                    }
                }
            }
        }

        // Handle add after we've released the borrow
        if should_add {
            if let Some(npc) = npc_result {
                self.state.add_npc(npc);
            }
        }

        if should_clear {
            if let Some(ref mut npc_editor) = self.state.npc_editor {
                npc_editor.clear();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_editor_state_creation() {
        let map = Map::new(1, 10, 10);
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
    fn test_set_tile_creates_undo_action() {
        let map = Map::new(1, 10, 10);
        let mut state = MapEditorState::new(map);

        let pos = Position::new(5, 5);
        let tile = Tile::new(TerrainType::Water, WallType::None);
        state.set_tile(pos, tile);

        assert!(state.has_changes);
        assert!(state.can_undo());
        assert!(!state.can_redo());
    }

    #[test]
    fn test_undo_redo_tile_change() {
        let map = Map::new(1, 10, 10);
        let mut state = MapEditorState::new(map);

        let pos = Position::new(5, 5);
        let original_tile = state.map.get_tile(pos).unwrap().clone();
        let new_tile = Tile::new(TerrainType::Water, WallType::None);

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
        let map = Map::new(1, 10, 10);
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
        let map = Map::new(1, 10, 10);
        let mut state = MapEditorState::new(map);

        let pos = Position::new(3, 3);
        state.paint_wall(pos, WallType::Door);

        assert_eq!(state.map.get_tile(pos).unwrap().wall_type, WallType::Door);
        assert!(state.has_changes);
    }

    #[test]
    fn test_add_remove_event() {
        let map = Map::new(1, 10, 10);
        let mut state = MapEditorState::new(map);

        let pos = Position::new(5, 5);
        let event = MapEvent::Sign {
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
        let map = Map::new(1, 10, 10);
        let mut state = MapEditorState::new(map);

        let npc = Npc::new(
            1,
            "Merchant".to_string(),
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
        let map = Map::new(1, 10, 10);
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
        let map = Map::new(1, 10, 10);
        let mut state = MapEditorState::new(map);

        // Place a wall
        let pos = Position::new(5, 5);
        state.paint_wall(pos, WallType::Normal);

        // Add event on blocked tile
        let event = MapEvent::Sign {
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
            encounter_monsters: "1, 2, 3".to_string(),
            ..Default::default()
        };

        let event = editor.to_map_event().unwrap();
        match event {
            MapEvent::Encounter { monster_group } => {
                assert_eq!(monster_group, vec![1, 2, 3]);
            }
            _ => panic!("Expected Encounter event"),
        }
    }

    #[test]
    fn test_event_editor_state_to_sign() {
        let editor = EventEditorState {
            event_type: EventType::Sign,
            sign_text: "Hello World".to_string(),
            ..Default::default()
        };

        let event = editor.to_map_event().unwrap();
        match event {
            MapEvent::Sign { text } => {
                assert_eq!(text, "Hello World");
            }
            _ => panic!("Expected Sign event"),
        }
    }

    #[test]
    fn test_npc_editor_state_to_npc() {
        let editor = NpcEditorState {
            npc_id: "42".to_string(),
            name: "Guard".to_string(),
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
        let mut state = MapEditorState::new(Map::new(1, 5, 5));
        state.selected_terrain = TerrainType::Grass;
        state.paint_tile(Position { x: 0, y: 0 });

        let ron = state.save_to_ron().unwrap();
        assert!(ron.contains("id:"));
        assert!(ron.contains("width:"));
        assert!(ron.contains("height:"));
    }

    #[test]
    fn test_metadata_editor() {
        let mut state = MapEditorState::new(Map::new(1, 10, 10));

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
        assert_eq!(state.metadata.encounter_rate, 25);
        assert!(state.metadata.is_outdoor);
    }

    #[test]
    fn test_add_event_at_position() {
        let mut state = MapEditorState::new(Map::new(1, 10, 10));
        let event = MapEvent::Sign {
            text: "Test sign".to_string(),
        };

        state.add_event_at_position(5, 5, event);

        let pos = Position { x: 5, y: 5 };
        assert!(state.map.events.contains_key(&pos));
    }

    #[test]
    fn test_show_event_editor_ui() {
        let mut state = MapEditorState::new(Map::new(1, 10, 10));
        assert!(!state.show_event_editor_ui());

        state.event_editor = Some(EventEditorState::default());
        assert!(state.show_event_editor_ui());
    }

    #[test]
    fn test_map_preview_with_terrain_types() {
        let map = Map::new(1, 10, 10);
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
        let mut state = MapEditorState::new(Map::new(1, 10, 10));

        // Add events at different positions
        let event1 = MapEvent::Sign {
            text: "Welcome".to_string(),
        };
        let event2 = MapEvent::Treasure {
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
        let mut state = MapEditorState::new(Map::new(1, 5, 5));

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
        let mut state = MapEditorState::new(Map::new(1, 10, 10));

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
}
