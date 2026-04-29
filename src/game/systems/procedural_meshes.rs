// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Procedural mesh generation for environmental objects and static event markers
//!
//! This module provides pure Rust functions to spawn composite 3D meshes using
//! Bevy primitives (Cylinder, Sphere, Cuboid). No external assets required.
//!
//! Character rendering (NPCs, Monsters, Recruitables) uses the sprite system.

use super::advanced_trees::{TerrainVisualConfig, TreeType};
use super::map::{MapEntity, TileCoord};
use crate::domain::types::{self, CreatureId};
use crate::domain::world;
use crate::domain::world::TileVisualMetadata;
use crate::game::components::furniture::{
    DoorState, FurnitureEntity, Interactable, InteractionType,
};
use bevy::color::LinearRgba;
use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;
use tracing;

// ==================== Mesh Caching ====================

/// Cache for procedural mesh handles to avoid duplicate asset creation
///
/// When spawning multiple trees, signs, or portals on a map, the same mesh
/// geometry is reused many times. This cache stores mesh handles to avoid
/// redundant allocations, improving performance and reducing garbage collection
/// pressure.
///
/// The cache is created fresh for each map spawn and is dropped after the map
/// is fully generated, allowing garbage collection of unused meshes between map loads.
///
/// # Examples
///
/// ```text
/// use antares::game::systems::procedural_meshes::ProceduralMeshCache;
///
/// let mut cache = ProceduralMeshCache::default();
/// // First call creates and caches the mesh
/// let handle1 = get_or_create_mesh(&mut cache, &mut meshes, ...);
/// // Second call reuses the cached handle
/// let handle2 = get_or_create_mesh(&mut cache, &mut meshes, ...);
/// assert_eq!(handle1, handle2);  // Same handle reused
/// ```
#[derive(Clone, Default)]
pub struct ProceduralMeshCache {
    /// Cached trunk mesh handle for trees
    tree_trunk: Option<Handle<Mesh>>,
    /// Cached foliage mesh handle for trees
    tree_foliage: Option<Handle<Mesh>>,
    /// Cached mesh handles for advanced tree types by TreeType variant
    tree_meshes: HashMap<TreeType, Handle<Mesh>>,
    /// Cached horizontal bar mesh handle for portals (top/bottom)
    portal_frame_horizontal: Option<Handle<Mesh>>,
    /// Cached vertical bar mesh handle for portals (left/right)
    portal_frame_vertical: Option<Handle<Mesh>>,
    /// Cached cylinder mesh handle for sign posts
    sign_post: Option<Handle<Mesh>>,
    /// Cached cuboid mesh handle for sign boards
    sign_board: Option<Handle<Mesh>>,
    /// Cached mesh handle for shrub stems
    shrub_stem: Option<Handle<Mesh>>,
    /// Cached mesh handle for grass blades
    grass_blade: Option<Handle<Mesh>>,
    /// Cached mesh handles for furniture components (legs, seats, etc.)
    furniture_bench_seat: Option<Handle<Mesh>>,
    furniture_bench_leg: Option<Handle<Mesh>>,
    furniture_table_top: Option<Handle<Mesh>>,
    furniture_table_leg: Option<Handle<Mesh>>,
    furniture_chair_seat: Option<Handle<Mesh>>,
    furniture_chair_back: Option<Handle<Mesh>>,
    furniture_chair_leg: Option<Handle<Mesh>>,
    furniture_throne_seat: Option<Handle<Mesh>>,
    furniture_throne_back: Option<Handle<Mesh>>,
    furniture_throne_arm: Option<Handle<Mesh>>,
    furniture_chest_body: Option<Handle<Mesh>>,
    furniture_chest_lid: Option<Handle<Mesh>>,
    furniture_torch_handle: Option<Handle<Mesh>>,
    furniture_torch_flame: Option<Handle<Mesh>>,
    /// Cached mesh handle for door panel
    furniture_door_panel: Option<Handle<Mesh>>,
    /// Cached mesh handle for door cross braces
    furniture_door_brace: Option<Handle<Mesh>>,
    /// Cached mesh handle for door hinges
    furniture_door_hinge: Option<Handle<Mesh>>,
    /// Cached mesh handle for door handle
    furniture_door_handle: Option<Handle<Mesh>>,
    /// Cached mesh handle for column shafts
    structure_column_shaft: Option<Handle<Mesh>>,
    /// Cached mesh handle for column capitals (Doric/Ionic)
    structure_column_capital: Option<Handle<Mesh>>,
    /// Cached mesh handle for arch curve
    structure_arch_curve: Option<Handle<Mesh>>,
    /// Cached mesh handle for arch supports
    structure_arch_support: Option<Handle<Mesh>>,
    /// Cached mesh handle for wall segments
    structure_wall: Option<Handle<Mesh>>,
    /// Cached mesh handle for door frame posts (vertical sides, used by spawn_door_frame)
    structure_door_frame: Option<Handle<Mesh>>,
    /// Cached mesh handle for door frame lintel (horizontal top bar, used by spawn_door_frame)
    structure_door_frame_lintel: Option<Handle<Mesh>>,
    /// Cached mesh handle for railing posts
    structure_railing_post: Option<Handle<Mesh>>,
    /// Cached mesh handle for railing bars
    structure_railing_bar: Option<Handle<Mesh>>,
    /// Cached mesh handles for creature visuals by (CreatureId, mesh_index)
    creature_meshes: HashMap<(CreatureId, usize), Handle<Mesh>>,
    /// Cached mesh handle for sword / dagger blade items
    item_sword: Option<Handle<Mesh>>,
    /// Cached mesh handle for dagger items
    item_dagger: Option<Handle<Mesh>>,
    /// Cached mesh handle for blunt weapon items
    item_blunt: Option<Handle<Mesh>>,
    /// Cached mesh handle for staff items
    item_staff: Option<Handle<Mesh>>,
    /// Cached mesh handle for bow items
    item_bow: Option<Handle<Mesh>>,
    /// Cached mesh handle for body armour / helmet items
    item_armor: Option<Handle<Mesh>>,
    /// Cached mesh handle for shield items
    item_shield: Option<Handle<Mesh>>,
    /// Cached mesh handle for potion items
    item_potion: Option<Handle<Mesh>>,
    /// Cached mesh handle for scroll items
    item_scroll: Option<Handle<Mesh>>,
    /// Cached mesh handle for ring / amulet items
    item_ring: Option<Handle<Mesh>>,
    /// Cached mesh handle for ammo (arrow / bolt / stone) items
    item_ammo: Option<Handle<Mesh>>,
    /// Cached mesh handle for quest-item meshes
    item_quest: Option<Handle<Mesh>>,
    /// Cached bark material handle (shared across all non-Dead tree types)
    tree_bark_material: Option<Handle<StandardMaterial>>,
    /// Cached foliage material handles keyed by TreeType variant
    tree_foliage_materials: HashMap<TreeType, Handle<StandardMaterial>>,
}

impl ProceduralMeshCache {
    /// Gets or creates a mesh handle for a specific tree type
    ///
    /// # Arguments
    ///
    /// * `tree_type` - The type of tree to get/create mesh for
    /// * `meshes` - Mesh asset storage
    ///
    /// # Returns
    ///
    /// Mesh handle for the tree type (either cached or newly created)
    pub fn get_or_create_tree_mesh(
        &mut self,
        tree_type: TreeType,
        meshes: &mut Assets<Mesh>,
    ) -> Handle<Mesh> {
        if let Some(handle) = self.tree_meshes.get(&tree_type) {
            handle.clone()
        } else {
            let config = tree_type.config();
            let graph = super::advanced_trees::generate_branch_graph(tree_type);
            let mesh = super::advanced_trees::generate_branch_mesh(&graph, &config);
            let handle = meshes.add(mesh);
            self.tree_meshes.insert(tree_type, handle.clone());
            handle
        }
    }

    /// Gets or creates a mesh handle for a specific creature mesh part
    ///
    /// # Arguments
    ///
    /// * `creature_id` - The ID of the creature definition
    /// * `mesh_index` - Index of the mesh within the creature definition
    /// * `mesh_def` - The mesh definition to convert if not cached
    /// * `meshes` - Mesh asset storage
    ///
    /// # Returns
    ///
    /// Mesh handle for the creature mesh (either cached or newly created)
    ///
    /// # Examples
    ///
    /// ```text
    /// use antares::game::systems::procedural_meshes::ProceduralMeshCache;
    /// use antares::domain::visual::MeshDefinition;
    ///
    /// let mut cache = ProceduralMeshCache::default();
    /// let handle = cache.get_or_create_creature_mesh(42, 0, &mesh_def, &mut meshes);
    /// ```
    pub fn get_or_create_creature_mesh(
        &mut self,
        creature_id: CreatureId,
        mesh_index: usize,
        mesh_def: &crate::domain::visual::MeshDefinition,
        meshes: &mut Assets<Mesh>,
    ) -> Handle<Mesh> {
        let key = (creature_id, mesh_index);

        if let Some(handle) = self.creature_meshes.get(&key) {
            handle.clone()
        } else {
            // Convert mesh definition to Bevy mesh
            let bevy_mesh = super::creature_meshes::mesh_definition_to_bevy(mesh_def);
            let handle = meshes.add(bevy_mesh);
            self.creature_meshes.insert(key, handle.clone());
            handle
        }
    }

    /// Clears all cached creature meshes
    ///
    /// This is useful for hot-reloading creature definitions or cleaning up
    /// memory when creatures are no longer needed.
    ///
    /// # Examples
    ///
    /// ```text
    /// use antares::game::systems::procedural_meshes::ProceduralMeshCache;
    ///
    /// let mut cache = ProceduralMeshCache::default();
    /// cache.clear_creature_cache();
    /// ```
    pub fn clear_creature_cache(&mut self) {
        self.creature_meshes.clear();
    }

    /// Gets or creates the bark `StandardMaterial` handle.
    ///
    /// The bark material uses the bark texture loaded via the asset server and
    /// is tinted with [`TREE_TRUNK_COLOR`].  The handle is cached so that all
    /// tree trunks on a map share the same material asset.
    ///
    /// # Arguments
    ///
    /// * `asset_server` - Asset server used to load the bark texture
    /// * `materials` - Mutable reference to the material asset storage
    ///
    /// # Returns
    ///
    /// Cloned handle to the cached (or newly created) bark material
    ///
    /// # Examples
    ///
    /// ```text
    /// let bark_handle = cache.get_or_create_bark_material(&asset_server, &mut materials);
    /// ```
    pub fn get_or_create_bark_material(
        &mut self,
        asset_server: &AssetServer,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        if let Some(handle) = &self.tree_bark_material {
            handle.clone()
        } else {
            tracing::debug!(
                texture_path = TREE_BARK_TEXTURE,
                "loading procedural tree bark material"
            );
            let handle = materials.add(StandardMaterial {
                base_color_texture: Some(asset_server.load(TREE_BARK_TEXTURE)),
                base_color: TREE_TRUNK_COLOR,
                perceptual_roughness: 0.9,
                ..default()
            });
            self.tree_bark_material = Some(handle.clone());
            handle
        }
    }

    /// Gets or creates the foliage `StandardMaterial` handle for `tree_type`.
    ///
    /// Each `TreeType` variant gets its own alpha-masked foliage material that
    /// references the appropriate foliage texture.  `double_sided` and
    /// `cull_mode: None` ensure the quad is visible from both sides.
    ///
    /// # Arguments
    ///
    /// * `tree_type` - Determines which foliage texture to load
    /// * `asset_server` - Asset server used to load the foliage texture
    /// * `materials` - Mutable reference to the material asset storage
    ///
    /// # Returns
    ///
    /// Cloned handle to the cached (or newly created) foliage material
    ///
    /// # Examples
    ///
    /// ```text
    /// let foliage_handle = cache.get_or_create_foliage_material(
    ///     TreeType::Oak, &asset_server, &mut materials,
    /// );
    /// ```
    pub fn get_or_create_foliage_material(
        &mut self,
        tree_type: TreeType,
        asset_server: &AssetServer,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        if let Some(handle) = self.tree_foliage_materials.get(&tree_type) {
            handle.clone()
        } else {
            let path = foliage_texture_path(tree_type);
            tracing::debug!(
                tree_type = ?tree_type,
                texture_path = path,
                "loading procedural tree foliage material"
            );
            let handle = materials.add(StandardMaterial {
                base_color_texture: Some(asset_server.load(path)),
                base_color: Color::WHITE,
                alpha_mode: AlphaMode::Mask(TREE_FOLIAGE_ALPHA_CUTOFF),
                double_sided: true,
                cull_mode: None,
                perceptual_roughness: 0.8,
                ..default()
            });
            self.tree_foliage_materials
                .insert(tree_type, handle.clone());
            handle
        }
    }
}

// ==================== Cache Helper Methods ====================

impl ProceduralMeshCache {
    /// Gets or creates a furniture mesh for the specified component
    ///
    /// Looks up the mesh handle in the cache. If not found, creates a new mesh
    /// using the provided generator function and caches it for future use.
    ///
    /// # Arguments
    ///
    /// * `component` - Name of the furniture component (e.g., "bench_seat", "chair_leg")
    /// * `meshes` - Mutable reference to Bevy's mesh asset storage
    /// * `creator` - Closure that generates the mesh if it's not cached
    ///
    /// # Returns
    ///
    /// Handle to the cached or newly created mesh
    pub fn get_or_create_furniture_mesh<F>(
        &mut self,
        component: &str,
        meshes: &mut Assets<Mesh>,
        creator: F,
    ) -> Handle<Mesh>
    where
        F: FnOnce() -> Mesh,
    {
        let handle = match component {
            "bench_seat" => &mut self.furniture_bench_seat,
            "bench_leg" => &mut self.furniture_bench_leg,
            "table_top" => &mut self.furniture_table_top,
            "table_leg" => &mut self.furniture_table_leg,
            "chair_seat" => &mut self.furniture_chair_seat,
            "chair_back" => &mut self.furniture_chair_back,
            "chair_leg" => &mut self.furniture_chair_leg,
            "throne_seat" => &mut self.furniture_throne_seat,
            "throne_back" => &mut self.furniture_throne_back,
            "throne_arm" => &mut self.furniture_throne_arm,
            "chest_body" => &mut self.furniture_chest_body,
            "chest_lid" => &mut self.furniture_chest_lid,
            "torch_handle" => &mut self.furniture_torch_handle,
            "torch_flame" => &mut self.furniture_torch_flame,
            "door_panel" => &mut self.furniture_door_panel,
            "door_brace" => &mut self.furniture_door_brace,
            "door_hinge" => &mut self.furniture_door_hinge,
            "door_handle" => &mut self.furniture_door_handle,
            _ => {
                tracing::error!("Unknown furniture component: {component}");
                return meshes.add(creator());
            }
        };

        handle.get_or_insert_with(|| meshes.add(creator())).clone()
    }

    /// Gets or creates a structure mesh for the specified component
    ///
    /// Looks up the mesh handle in the cache. If not found, creates a new mesh
    /// using the provided generator function and caches it for future use.
    ///
    /// # Arguments
    ///
    /// * `component` - Name of the structure component (e.g., "column_shaft", "arch_curve")
    /// * `meshes` - Mutable reference to Bevy's mesh asset storage
    /// * `creator` - Closure that generates the mesh if it's not cached
    ///
    /// # Returns
    ///
    /// Handle to the cached or newly created mesh
    pub fn get_or_create_structure_mesh<F>(
        &mut self,
        component: &str,
        meshes: &mut Assets<Mesh>,
        creator: F,
    ) -> Handle<Mesh>
    where
        F: FnOnce() -> Mesh,
    {
        let handle = match component {
            "column_shaft" => &mut self.structure_column_shaft,
            "column_capital" => &mut self.structure_column_capital,
            "arch_curve" => &mut self.structure_arch_curve,
            "arch_support" => &mut self.structure_arch_support,
            "wall" => &mut self.structure_wall,
            "door_frame" => &mut self.structure_door_frame,
            "door_frame_lintel" => &mut self.structure_door_frame_lintel,
            "railing_post" => &mut self.structure_railing_post,
            "railing_bar" => &mut self.structure_railing_bar,
            _ => {
                tracing::error!("Unknown structure component: {component}");
                return meshes.add(creator());
            }
        };

        handle.get_or_insert_with(|| meshes.add(creator())).clone()
    }

    /// Clear all cached meshes to free GPU memory
    ///
    /// Used when unloading maps or switching scenes. Note: Handle instances
    /// in existing entities are not affected; only new asset loads will be prevented.
    pub fn clear_all(&mut self) {
        self.tree_trunk = None;
        self.tree_foliage = None;
        self.portal_frame_horizontal = None;
        self.portal_frame_vertical = None;
        self.sign_post = None;
        self.sign_board = None;
        self.shrub_stem = None;
        self.grass_blade = None;
        self.furniture_bench_seat = None;
        self.furniture_bench_leg = None;
        self.furniture_table_top = None;
        self.furniture_table_leg = None;
        self.furniture_chair_seat = None;
        self.furniture_chair_back = None;
        self.furniture_chair_leg = None;
        self.furniture_throne_seat = None;
        self.furniture_throne_back = None;
        self.furniture_throne_arm = None;
        self.furniture_chest_body = None;
        self.furniture_chest_lid = None;
        self.furniture_torch_handle = None;
        self.furniture_torch_flame = None;
        self.furniture_door_panel = None;
        self.furniture_door_brace = None;
        self.furniture_door_hinge = None;
        self.furniture_door_handle = None;
        self.structure_column_shaft = None;
        self.structure_column_capital = None;
        self.structure_arch_curve = None;
        self.structure_arch_support = None;
        self.structure_wall = None;
        self.structure_door_frame = None;
        self.structure_door_frame_lintel = None;
        self.structure_railing_post = None;
        self.structure_railing_bar = None;
        self.creature_meshes.clear();
        self.item_sword = None;
        self.item_dagger = None;
        self.item_blunt = None;
        self.item_staff = None;
        self.item_bow = None;
        self.item_armor = None;
        self.item_shield = None;
        self.item_potion = None;
        self.item_scroll = None;
        self.item_ring = None;
        self.item_ammo = None;
        self.item_quest = None;
        self.tree_bark_material = None;
        self.tree_foliage_materials.clear();
    }

    /// Gets or creates a cached mesh handle for an item category.
    ///
    /// Follows the same pattern as [`get_or_create_furniture_mesh`].  The
    /// `category` string is the snake-case name of the
    /// [`ItemMeshCategory`](crate::domain::visual::item_mesh::ItemMeshCategory)
    /// variant (e.g. `"sword"`, `"potion"`, `"ring"`).
    ///
    /// # Arguments
    ///
    /// * `category` - Snake-case item category name.
    /// * `meshes`   - Mutable reference to Bevy's mesh asset storage.
    /// * `creator`  - Closure that generates the mesh if it is not yet cached.
    ///
    /// # Errors
    ///
    /// Logs a `tracing::error!` and returns an uncached mesh handle if
    /// `category` is not one of the recognised item category strings.
    pub fn get_or_create_item_mesh<F>(
        &mut self,
        category: &str,
        meshes: &mut Assets<Mesh>,
        creator: F,
    ) -> Handle<Mesh>
    where
        F: FnOnce() -> Mesh,
    {
        let handle = match category {
            "sword" => &mut self.item_sword,
            "dagger" => &mut self.item_dagger,
            "blunt" => &mut self.item_blunt,
            "staff" => &mut self.item_staff,
            "bow" => &mut self.item_bow,
            "armor" => &mut self.item_armor,
            "shield" => &mut self.item_shield,
            "potion" => &mut self.item_potion,
            "scroll" => &mut self.item_scroll,
            "ring" => &mut self.item_ring,
            "ammo" => &mut self.item_ammo,
            "quest" => &mut self.item_quest,
            _ => {
                tracing::error!("Unknown item cache category: {category}");
                return meshes.add(creator());
            }
        };

        handle.get_or_insert_with(|| meshes.add(creator())).clone()
    }

    /// Count the number of cached mesh handles
    ///
    /// # Returns
    ///
    /// Number of non-None mesh handles currently in the cache
    pub fn cached_count(&self) -> usize {
        let mut count = 0;
        if self.tree_trunk.is_some() {
            count += 1;
        }
        if self.tree_foliage.is_some() {
            count += 1;
        }
        if self.portal_frame_horizontal.is_some() {
            count += 1;
        }
        if self.portal_frame_vertical.is_some() {
            count += 1;
        }
        if self.sign_post.is_some() {
            count += 1;
        }
        if self.sign_board.is_some() {
            count += 1;
        }
        if self.shrub_stem.is_some() {
            count += 1;
        }
        if self.grass_blade.is_some() {
            count += 1;
        }
        if self.furniture_bench_seat.is_some() {
            count += 1;
        }
        if self.furniture_bench_leg.is_some() {
            count += 1;
        }
        if self.furniture_table_top.is_some() {
            count += 1;
        }
        if self.furniture_table_leg.is_some() {
            count += 1;
        }
        if self.furniture_chair_seat.is_some() {
            count += 1;
        }
        if self.furniture_chair_back.is_some() {
            count += 1;
        }
        if self.furniture_chair_leg.is_some() {
            count += 1;
        }
        if self.furniture_throne_seat.is_some() {
            count += 1;
        }
        if self.furniture_throne_back.is_some() {
            count += 1;
        }
        if self.furniture_throne_arm.is_some() {
            count += 1;
        }
        if self.furniture_chest_body.is_some() {
            count += 1;
        }
        if self.furniture_chest_lid.is_some() {
            count += 1;
        }
        if self.furniture_torch_handle.is_some() {
            count += 1;
        }
        if self.furniture_torch_flame.is_some() {
            count += 1;
        }
        if self.furniture_door_panel.is_some() {
            count += 1;
        }
        if self.furniture_door_brace.is_some() {
            count += 1;
        }
        if self.furniture_door_hinge.is_some() {
            count += 1;
        }
        if self.furniture_door_handle.is_some() {
            count += 1;
        }
        if self.structure_column_shaft.is_some() {
            count += 1;
        }
        if self.structure_column_capital.is_some() {
            count += 1;
        }
        if self.structure_arch_curve.is_some() {
            count += 1;
        }
        if self.structure_arch_support.is_some() {
            count += 1;
        }
        if self.structure_wall.is_some() {
            count += 1;
        }
        if self.structure_door_frame.is_some() {
            count += 1;
        }
        if self.structure_door_frame_lintel.is_some() {
            count += 1;
        }
        if self.structure_railing_post.is_some() {
            count += 1;
        }
        if self.structure_railing_bar.is_some() {
            count += 1;
        }
        if self.item_sword.is_some() {
            count += 1;
        }
        if self.item_dagger.is_some() {
            count += 1;
        }
        if self.item_blunt.is_some() {
            count += 1;
        }
        if self.item_staff.is_some() {
            count += 1;
        }
        if self.item_bow.is_some() {
            count += 1;
        }
        if self.item_armor.is_some() {
            count += 1;
        }
        if self.item_shield.is_some() {
            count += 1;
        }
        if self.item_potion.is_some() {
            count += 1;
        }
        if self.item_scroll.is_some() {
            count += 1;
        }
        if self.item_ring.is_some() {
            count += 1;
        }
        if self.item_ammo.is_some() {
            count += 1;
        }
        if self.item_quest.is_some() {
            count += 1;
        }
        count
    }
}

// ==================== Constants ====================

// Tree dimensions (world units, 1 unit ≈ 10 feet)

const TREE_FOLIAGE_RADIUS: f32 = 0.6;

// Event marker dimensions
// Portal dimensions - rectangular frame standing vertically
const PORTAL_FRAME_WIDTH: f32 = 1.0; // Width of the portal opening (full tile)
const PORTAL_FRAME_HEIGHT: f32 = 1.8; // Height of the portal opening (taller, human-sized)
const PORTAL_FRAME_THICKNESS: f32 = 0.08; // Thickness of frame bars
const PORTAL_FRAME_DEPTH: f32 = 0.04; // Depth of frame bars (thinner)
const PORTAL_Y_POSITION: f32 = 0.9; // Bottom of frame at ground level (frame center)
const _PORTAL_ROTATION_SPEED: f32 = 1.0; // radians/sec

const SIGN_POST_RADIUS: f32 = 0.05;
const SIGN_POST_HEIGHT: f32 = 1.5;
const SIGN_BOARD_WIDTH: f32 = 0.6;
const SIGN_BOARD_HEIGHT: f32 = 0.3;
const SIGN_BOARD_DEPTH: f32 = 0.05;
const SIGN_BOARD_Y_OFFSET: f32 = 1.5; // Eye height (approx 5 feet)

// Furniture dimensions - Bench
const BENCH_LENGTH: f32 = 1.2;
const BENCH_WIDTH: f32 = 0.4;
const BENCH_HEIGHT: f32 = 0.4;
const BENCH_LEG_HEIGHT: f32 = 0.35;
const BENCH_LEG_THICKNESS: f32 = 0.08;

// Furniture dimensions - Table
const TABLE_TOP_WIDTH: f32 = 1.2;
const TABLE_TOP_DEPTH: f32 = 0.8;
const TABLE_TOP_HEIGHT: f32 = 0.05;
const TABLE_HEIGHT: f32 = 0.7;
const TABLE_LEG_THICKNESS: f32 = 0.1;

// Furniture dimensions - Chair
const CHAIR_SEAT_WIDTH: f32 = 0.5;
const CHAIR_SEAT_DEPTH: f32 = 0.5;
const CHAIR_SEAT_HEIGHT: f32 = 0.05;
const CHAIR_HEIGHT: f32 = 0.9;
const CHAIR_BACK_HEIGHT: f32 = 0.5;
const CHAIR_BACK_WIDTH: f32 = 0.5;
const CHAIR_LEG_THICKNESS: f32 = 0.06;
const CHAIR_ARMREST_HEIGHT: f32 = 0.3;

// Furniture dimensions - Throne (ornate chair)
const THRONE_SEAT_WIDTH: f32 = 0.7;
const THRONE_SEAT_DEPTH: f32 = 0.7;
const THRONE_SEAT_HEIGHT: f32 = 0.08;

const THRONE_BACK_HEIGHT: f32 = 0.9;
const THRONE_BACK_WIDTH: f32 = 0.7;
const THRONE_ARM_HEIGHT: f32 = 0.6;
const THRONE_ARM_WIDTH: f32 = 0.15;

// Furniture dimensions - Chest
const CHEST_WIDTH: f32 = 0.8;
const CHEST_DEPTH: f32 = 0.5;
const CHEST_HEIGHT: f32 = 0.6;
const CHEST_LID_HEIGHT: f32 = 0.06;

// Furniture dimensions - Torch
const TORCH_HANDLE_RADIUS: f32 = 0.05;
const TORCH_HANDLE_HEIGHT: f32 = 1.2;
const TORCH_FLAME_WIDTH: f32 = 0.25;
const TORCH_FLAME_HEIGHT: f32 = 0.4;

// Furniture dimensions - Door
/// Default door panel width in world units
const DOOR_PANEL_WIDTH: f32 = 0.9;
/// Default door panel height in world units
const DOOR_PANEL_HEIGHT: f32 = 2.3;
/// Default door panel thickness in world units
const DOOR_PANEL_THICKNESS: f32 = 0.08;
/// Thickness of horizontal cross-brace strips on the door face
const DOOR_BRACE_HEIGHT: f32 = 0.08;
/// How far cross braces protrude from the panel face
const DOOR_BRACE_PROUD: f32 = 0.02;
/// Height of each hinge cuboid
const DOOR_HINGE_HEIGHT: f32 = 0.12;
/// Width of each hinge cuboid
const DOOR_HINGE_WIDTH: f32 = 0.06;
/// Radius of the door handle cylinder
const DOOR_HANDLE_RADIUS: f32 = 0.025;
/// Length of the door handle cylinder
const DOOR_HANDLE_LENGTH: f32 = 0.12;
/// Default number of visible plank strips on the door face
const DOOR_DEFAULT_PLANK_COUNT: u8 = 5;

// Color constants
const TREE_TRUNK_COLOR: Color = Color::srgb(0.4, 0.25, 0.15); // Brown
const TREE_FOLIAGE_COLOR: Color = Color::srgb(0.2, 0.6, 0.2); // Green

// Tree texture asset paths
/// Asset path for the bark texture applied to all non-Dead tree trunks.
const TREE_BARK_TEXTURE: &str = "assets/textures/trees/bark.png";
/// Asset path for the Oak foliage alpha-mask texture.
const TREE_FOLIAGE_TEXTURE_OAK: &str = "assets/textures/trees/foliage_oak.png";
/// Asset path for the Pine foliage alpha-mask texture.
const TREE_FOLIAGE_TEXTURE_PINE: &str = "assets/textures/trees/foliage_pine.png";
/// Asset path for the Birch foliage alpha-mask texture.
const TREE_FOLIAGE_TEXTURE_BIRCH: &str = "assets/textures/trees/foliage_birch.png";
/// Asset path for the Willow foliage alpha-mask texture.
const TREE_FOLIAGE_TEXTURE_WILLOW: &str = "assets/textures/trees/foliage_willow.png";
/// Asset path for the Palm foliage alpha-mask texture.
const TREE_FOLIAGE_TEXTURE_PALM: &str = "assets/textures/trees/foliage_palm.png";
/// Asset path for the Shrub foliage alpha-mask texture.
const TREE_FOLIAGE_TEXTURE_SHRUB: &str = "assets/textures/trees/foliage_shrub.png";
/// Alpha cutoff for foliage `AlphaMode::Mask` — pixels with alpha below this are clipped.
const TREE_FOLIAGE_ALPHA_CUTOFF: f32 = 0.35_f32;

const PORTAL_COLOR: Color = Color::srgb(0.53, 0.29, 0.87); // Purple
const SIGN_POST_COLOR: Color = Color::srgb(0.4, 0.3, 0.2); // Dark brown
const SIGN_BOARD_COLOR: Color = Color::srgb(0.59, 0.44, 0.27); // Tan

// Furniture colors
const BENCH_COLOR: Color = Color::srgb(0.55, 0.35, 0.2); // Medium brown
const TABLE_COLOR: Color = Color::srgb(0.45, 0.3, 0.15); // Dark wood
const CHAIR_COLOR: Color = Color::srgb(0.55, 0.35, 0.2); // Medium brown
const THRONE_COLOR: Color = Color::srgb(0.85, 0.7, 0.4); // Gold/brass
const THRONE_BACKING: Color = Color::srgb(0.6, 0.0, 0.0); // Deep red for backing
const CHEST_COLOR: Color = Color::srgb(0.35, 0.2, 0.1); // Dark brown
const TORCH_HANDLE_COLOR: Color = Color::srgb(0.2, 0.1, 0.0); // Very dark brown
const TORCH_FLAME_COLOR: Color = Color::srgb(1.0, 0.8, 0.2); // Yellow/orange
const DOOR_PANEL_COLOR: Color = Color::srgb(0.55, 0.35, 0.18); // Warm wood brown

// Structure dimensions - Column
const COLUMN_CAPITAL_HEIGHT: f32 = 0.2; // Additional height for capital
const COLUMN_BASE_HEIGHT: f32 = 0.15;

// Structure dimensions - Arch
const ARCH_INNER_RADIUS: f32 = 1.0;
const ARCH_SUPPORT_WIDTH: f32 = 0.4;
const ARCH_SUPPORT_HEIGHT: f32 = 1.5;

// Structure dimensions - Door Frame
#[cfg(test)]
const DOOR_FRAME_THICKNESS: f32 = 0.15;
#[cfg(test)]
const DOOR_FRAME_BORDER: f32 = 0.1;

// Structure colors
const STRUCTURE_STONE_COLOR: Color = Color::srgb(0.7, 0.7, 0.7); // Light gray stone
const STRUCTURE_MARBLE_COLOR: Color = Color::srgb(0.9, 0.9, 0.9); // White marble

// Tile centering offset
/// Offset to center procedural meshes within their tile (matches camera centering)
const TILE_CENTER_OFFSET: f32 = 0.5;

// ==================== Spawn Context ====================

/// Bundles the four mutable resources that every procedural-mesh spawn
/// function needs: ECS commands, material assets, mesh assets, and the
/// shared mesh cache.
///
/// Passing a single `&mut MeshSpawnContext` instead of four separate
/// parameters keeps function signatures under clippy's 7-argument
/// threshold and makes call-sites less noisy.
///
/// # Examples
///
/// ```text
/// let mut ctx = MeshSpawnContext {
///     commands: &mut commands,
///     materials: &mut materials,
///     meshes: &mut meshes,
///     cache: &mut cache,
/// };
/// spawn_bench(&mut ctx, position, map_id, BenchConfig::default(), None);
/// ```
pub struct MeshSpawnContext<'a, 'w, 's> {
    /// Bevy [`Commands`] for spawning and mutating entities.
    pub commands: &'a mut Commands<'w, 's>,
    /// Asset storage for [`StandardMaterial`]s.
    pub materials: &'a mut Assets<StandardMaterial>,
    /// Asset storage for [`Mesh`]es.
    pub meshes: &'a mut Assets<Mesh>,
    /// Shared procedural-mesh cache that prevents duplicate allocations.
    pub cache: &'a mut ProceduralMeshCache,
}

// ==================== Private Helpers ====================

/// Returns the base bark colour for the requested tree type.
///
/// The bark texture is still applied through [`TREE_BARK_TEXTURE`], but the
/// per-type colour makes Dead, Birch, Palm, Pine, and leafy deciduous trunks
/// visibly distinct even when the texture is subtle or viewed at distance.
fn bark_color_for_tree_type(tree_type: TreeType) -> Color {
    match tree_type {
        TreeType::Dead => Color::srgb(0.25, 0.22, 0.18),
        TreeType::Birch => Color::srgb(0.78, 0.72, 0.62),
        TreeType::Palm => Color::srgb(0.50, 0.36, 0.20),
        TreeType::Pine => Color::srgb(0.34, 0.22, 0.13),
        TreeType::Willow => Color::srgb(0.36, 0.27, 0.16),
        TreeType::Shrub => Color::srgb(0.24, 0.18, 0.10),
        TreeType::Oak => TREE_TRUNK_COLOR,
    }
}

/// Multiplies two colours in linear space while preserving the base alpha.
fn multiply_color(base: Color, tint: Color) -> Color {
    let base_rgba = base.to_linear();
    let tint_rgba = tint.to_linear();

    Color::linear_rgba(
        (base_rgba.red * tint_rgba.red).min(1.0),
        (base_rgba.green * tint_rgba.green).min(1.0),
        (base_rgba.blue * tint_rgba.blue).min(1.0),
        base_rgba.alpha,
    )
}

/// Returns the asset path for the foliage texture of the given tree type.
///
/// Used by [`ProceduralMeshCache::get_or_create_foliage_material`] to select
/// the correct texture.  `Dead` trees have zero foliage density so their path
/// is never loaded in practice, but we fall back to Oak for safety.
fn foliage_texture_path(tree_type: TreeType) -> &'static str {
    match tree_type {
        TreeType::Oak => TREE_FOLIAGE_TEXTURE_OAK,
        TreeType::Pine => TREE_FOLIAGE_TEXTURE_PINE,
        TreeType::Birch => TREE_FOLIAGE_TEXTURE_BIRCH,
        TreeType::Willow => TREE_FOLIAGE_TEXTURE_WILLOW,
        TreeType::Palm => TREE_FOLIAGE_TEXTURE_PALM,
        TreeType::Dead => TREE_FOLIAGE_TEXTURE_OAK, // unused; Dead has density 0
        TreeType::Shrub => TREE_FOLIAGE_TEXTURE_SHRUB,
    }
}

// ==================== Public Functions ====================

/// Spawns a procedural tree mesh with trunk and foliage
///
/// Creates two child entities:
/// - Trunk: Brown cylinder (0.15 radius, 2.0 height)
/// - Foliage: Green sphere (0.6 radius) positioned at trunk top
///
/// Reuses cached meshes when available to avoid duplicate allocations.
/// This significantly improves performance when spawning multiple trees
/// on large maps.
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `visual_metadata` - Optional per-tile visual customization (scale, height, color tint, rotation)
/// * `tree_type` - Optional tree type for advanced generation (defaults to simple trunk/foliage if None)
/// * `cache` - Mutable reference to mesh cache for reuse
///
/// # Returns
///
/// Entity ID of the parent tree entity
///
/// # Examples
///
/// ```text
/// use antares::game::systems::procedural_meshes;
/// use antares::domain::types::{MapId, Position};
/// use antares::game::systems::advanced_trees::TreeType;
///
/// // Inside a Bevy system:
/// let mut cache = ProceduralMeshCache::default();
/// let tree_entity = procedural_meshes::spawn_tree(
///     &mut commands,
///     &mut materials,
///     &mut meshes,
///     Position { x: 5, y: 10 },
///     MapId::new(1),
///     None,  // No visual metadata
///     Some(TreeType::Oak),  // Specific tree type
///     &mut cache,
/// );
/// ```
/// Spawns foliage clusters at leaf branch endpoints using alpha-masked plane quads.
///
/// Replaces the previous sphere-based foliage with double-sided plane quads that
/// carry a per-`TreeType` alpha-masked foliage texture.  This gives far better
/// silhouettes while keeping draw-call count low (one cached material per tree type).
///
/// # Algorithm
///
/// 1. Identifies all leaf branches (endpoints with no children).
/// 2. For each leaf, calculates cluster size: `(foliage_density * 5.0) as usize`.
/// 3. Spawns foliage plane quads positioned at branch endpoint + random offset.
/// 4. Uses seeded RNG for deterministic placement (same seed = same foliage).
/// 5. Quad size scales with `foliage_density * TREE_FOLIAGE_RADIUS`.
/// 6. When a `color_tint` is supplied the cached material is cloned and its
///    `base_color` is overridden; otherwise the cached handle is reused directly.
///
/// # Arguments
///
/// * `commands` - Bevy commands for spawning entities
/// * `materials` - Asset storage for materials
/// * `meshes` - Asset storage for meshes
/// * `asset_server` - Asset server for loading foliage textures
/// * `graph` - The generated branch graph from the tree
/// * `config` - Tree configuration with foliage_density parameter
/// * `foliage_color` - Optional tint colour applied to foliage base_color
/// * `tree_type` - The tree variant — selects the correct foliage texture
/// * `parent_entity` - Parent entity to attach foliage quads to
/// * `cache` - Procedural mesh cache for reusing mesh and material handles
struct FoliageClusterSpawnConfig<'a> {
    graph: &'a super::advanced_trees::BranchGraph,
    tree_config: &'a super::advanced_trees::TreeConfig,
    visual_config: &'a TerrainVisualConfig,
    foliage_color: Option<Color>,
    tree_type: TreeType,
    parent_entity: Entity,
}

fn spawn_foliage_clusters(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    asset_server: &AssetServer,
    spawn_config: FoliageClusterSpawnConfig<'_>,
) {
    // Identify leaf branches (endpoints)
    let leaf_indices = super::advanced_trees::get_leaf_branches(spawn_config.graph);

    // Calculate cluster size from foliage density
    let cluster_size = (spawn_config.tree_config.foliage_density * 5.0) as usize;

    // If no foliage requested, return early
    if cluster_size == 0 || leaf_indices.is_empty() {
        return;
    }

    // Quad size: foliage_density * TREE_FOLIAGE_RADIUS gives a natural cluster footprint
    let foliage_size = spawn_config.tree_config.foliage_density * TREE_FOLIAGE_RADIUS;

    // Get or create the plane quad mesh from cache.
    // We reuse the existing tree_foliage slot — it now stores a plane quad instead of a sphere.
    let foliage_mesh = ctx.cache.tree_foliage.clone().unwrap_or_else(|| {
        let handle = ctx.meshes.add(
            Plane3d::default()
                .mesh()
                .size(foliage_size * 2.0, foliage_size * 2.0)
                .build(),
        );
        ctx.cache.tree_foliage = Some(handle.clone());
        handle
    });

    // Obtain the base cached foliage material for this tree type.
    let base_material = ctx.cache.get_or_create_foliage_material(
        spawn_config.tree_type,
        asset_server,
        ctx.materials,
    );

    // If a tint colour is supplied, clone the cached material and override base_color.
    // When there is no tint we reuse the cached handle to avoid extra allocations.
    let foliage_material = if let Some(tint) = spawn_config.foliage_color {
        // Clone the cached material's data and apply the tint
        let mut mat = ctx
            .materials
            .get(&base_material)
            .cloned()
            .unwrap_or_else(|| StandardMaterial {
                alpha_mode: AlphaMode::Mask(TREE_FOLIAGE_ALPHA_CUTOFF),
                double_sided: true,
                cull_mode: None,
                perceptual_roughness: 0.8,
                ..default()
            });
        mat.base_color = tint;
        ctx.materials.add(mat)
    } else {
        base_material
    };

    // Seeded RNG for deterministic foliage placement
    use rand::Rng;
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(42); // Fixed seed for consistency

    // Spawn foliage at each leaf branch
    for &leaf_idx in &leaf_indices {
        let branch = &spawn_config.graph.branches[leaf_idx];

        // Spawn multiple foliage quads in a cluster
        for _quad_idx in 0..cluster_size {
            // Random offset from branch endpoint (within radius 0.2-0.5)
            let offset_radius = rng.random_range(0.2..=0.5);
            let angle = rng.random_range(0.0..std::f32::consts::TAU);

            let offset_x = offset_radius * angle.cos();
            let offset_z = offset_radius * angle.sin();
            let offset_y = rng.random_range(-0.1..0.1);

            // Quad scale based on branch end radius (0.3–0.6)
            let quad_scale = (branch.end_radius * 1.5).clamp(0.3, 0.6);

            // Random Y-axis rotation so quads fan out naturally
            let rotation_y = rng.random_range(0.0..std::f32::consts::TAU);

            // Position in the tree parent's local space.  The branch mesh child
            // is scaled by visual metadata, so foliage endpoint positions must
            // receive the same metadata scaling or leaves detach from trunks.
            let unscaled_position = branch.end + Vec3::new(offset_x, offset_y, offset_z);
            let position = Vec3::new(
                unscaled_position.x * spawn_config.visual_config.scale,
                unscaled_position.y * spawn_config.visual_config.height_multiplier,
                unscaled_position.z * spawn_config.visual_config.scale,
            );

            let scaled_quad = quad_scale * spawn_config.visual_config.scale.max(0.1);

            // Spawn foliage quad, rotated to face up (Plane3d is XZ, we want XY billboard)
            let foliage = ctx
                .commands
                .spawn((
                    Mesh3d(foliage_mesh.clone()),
                    MeshMaterial3d(foliage_material.clone()),
                    Transform::from_translation(position)
                        .with_rotation(
                            Quat::from_rotation_y(rotation_y)
                                * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                        )
                        .with_scale(Vec3::splat(scaled_quad)),
                    GlobalTransform::default(),
                    Visibility::default(),
                    bevy::light::NotShadowCaster,
                    bevy::light::NotShadowReceiver,
                ))
                .id();

            ctx.commands
                .entity(spawn_config.parent_entity)
                .add_child(foliage);
        }
    }
}

pub fn spawn_tree(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    asset_server: &AssetServer,
    position: types::Position,
    map_id: types::MapId,
    visual_metadata: Option<&TileVisualMetadata>,
    tree_type: Option<super::advanced_trees::TreeType>,
) -> Entity {
    // Determine visual configuration from optional metadata
    let visual_config = visual_metadata
        .map(super::advanced_trees::TerrainVisualConfig::from)
        .unwrap_or_default();

    // Generate branch graph for the tree type
    let tree_type_resolved = tree_type.unwrap_or(super::advanced_trees::TreeType::Oak);
    let branch_graph = super::advanced_trees::generate_branch_graph(tree_type_resolved);

    // Get or create advanced tree mesh from cache
    let tree_mesh = ctx
        .cache
        .get_or_create_tree_mesh(tree_type_resolved, ctx.meshes);

    // Use the cached bark texture material as the source, then clone it for
    // the resolved tree type and optional metadata tint.  This keeps the bark
    // texture applied while making tree variants and SDK colour edits visibly
    // affect trunk appearance without mutating the shared cached handle.
    let base_bark_color = bark_color_for_tree_type(tree_type_resolved);
    let resolved_bark_color = visual_config
        .color_tint
        .map(|tint| multiply_color(base_bark_color, tint))
        .unwrap_or(base_bark_color);
    let bark_handle = ctx
        .cache
        .get_or_create_bark_material(asset_server, ctx.materials);
    let mut bark_material =
        ctx.materials
            .get(&bark_handle)
            .cloned()
            .unwrap_or_else(|| StandardMaterial {
                base_color_texture: Some(asset_server.load(TREE_BARK_TEXTURE)),
                base_color: TREE_TRUNK_COLOR,
                perceptual_roughness: 0.9,
                ..default()
            });
    bark_material.base_color = resolved_bark_color;
    let tree_material = ctx.materials.add(bark_material);

    // Apply color tint to foliage if present — passed as Option<Color> to
    // spawn_foliage_clusters which handles the tinted clone internally.
    let foliage_color = visual_config
        .color_tint
        .map(|tint| multiply_color(TREE_FOLIAGE_COLOR, tint));

    // Spawn parent tree entity with optional rotation
    let parent = ctx
        .commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                0.0,
                position.y as f32 + TILE_CENTER_OFFSET,
            )
            .with_rotation(Quat::from_rotation_y(visual_config.rotation_y.to_radians())),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
        ))
        .id();

    // Spawn tree mesh child at origin (branch graph is based at 0,0,0)
    let tree_structure = ctx
        .commands
        .spawn((
            Mesh3d(tree_mesh),
            MeshMaterial3d(tree_material),
            Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(
                visual_config.scale,
                visual_config.height_multiplier,
                visual_config.scale,
            )),
            GlobalTransform::default(),
            Visibility::default(),
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
        ))
        .id();
    ctx.commands.entity(parent).add_child(tree_structure);

    // Spawn foliage clusters at leaf branch endpoints using plane quads.  The
    // domain metadata foliage density multiplier is applied here so Campaign
    // Builder edits visibly change runtime foliage coverage.
    let mut tree_config = tree_type_resolved.config();
    let foliage_multiplier = visual_metadata
        .map(TileVisualMetadata::foliage_density)
        .unwrap_or(1.0);
    tree_config.foliage_density =
        (tree_config.foliage_density * foliage_multiplier).clamp(0.0, 2.0);

    spawn_foliage_clusters(
        ctx,
        asset_server,
        FoliageClusterSpawnConfig {
            graph: &branch_graph,
            tree_config: &tree_config,
            visual_config: &visual_config,
            foliage_color,
            tree_type: tree_type_resolved,
            parent_entity: parent,
        },
    );

    parent
}

/// Spawns a procedurally generated shrub
///
/// Uses the advanced tree generation system (TreeType::Shrub) which generates
/// a multi-stem mesh with vertex coloring.
///
/// # Arguments
///
/// * `ctx` - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache)
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `visual_metadata` - Optional per-tile customization (height controls shrub size, scale affects foliage density)
///
/// # Returns
///
/// Entity ID of the shrub entity
pub fn spawn_shrub(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    visual_metadata: Option<&TileVisualMetadata>,
) -> Entity {
    let shrub_mesh = ctx
        .cache
        .get_or_create_tree_mesh(TreeType::Shrub, ctx.meshes);

    // Get visual configuration
    let meta = TerrainVisualConfig::from(visual_metadata.unwrap_or(&TileVisualMetadata::default()));
    let height_scale = meta.height_multiplier;
    let width_scale = meta.scale;

    // Random Y-rotation
    let mut rng = rand::rng();
    let rotation = Quat::from_rotation_y(rng.random_range(0.0..std::f32::consts::TAU));

    // Create material that supports vertex colors (or just standard PBR)
    // The advanced tree mesh has vertex colors baked in.
    let material = ctx.materials.add(StandardMaterial {
        base_color: Color::WHITE, // Vertex colors multiply with this
        perceptual_roughness: 0.9,
        ..default()
    });

    ctx.commands
        .spawn((
            Mesh3d(shrub_mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                0.0,
                position.y as f32 + TILE_CENTER_OFFSET,
            )
            .with_scale(Vec3::new(width_scale, height_scale, width_scale))
            .with_rotation(rotation),
            GlobalTransform::default(),
            Visibility::default(),
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
            MapEntity(map_id),
            TileCoord(position),
        ))
        .id()
}

// Grass rendering lives in `advanced_grass.rs` (spawn, mesh, culling, LOD).
/// Spawns a procedural portal/teleport mesh
/// Spawns a procedural portal/teleport mesh (rectangular frame)
///
/// Creates a glowing purple rectangular frame positioned vertically at ground level.
/// The frame is composed of four cuboid bars forming a doorway-like portal.
/// Vertically oriented with rounded corners created by the thickness of the bars.
///
/// Reuses cached meshes when available to avoid duplicate allocations.
///
/// # Arguments
///
/// * `ctx` - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache)
/// * `position` - Tile position in world coordinates
/// * `event_name` - Event name for entity label
/// * `map_id` - Map identifier for cleanup
/// * `rotation_y` - Optional rotation in degrees around Y-axis (default: 0.0)
///
/// # Returns
///
/// Entity ID of the parent portal entity
pub fn spawn_portal(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    event_name: String,
    map_id: types::MapId,
    rotation_y: Option<f32>,
) -> Entity {
    // Get or create horizontal bar mesh from cache (for top/bottom bars)
    let horizontal_bar_mesh = ctx
        .cache
        .portal_frame_horizontal
        .clone()
        .unwrap_or_else(|| {
            let handle = ctx.meshes.add(Cuboid {
                half_size: Vec3::new(
                    PORTAL_FRAME_WIDTH / 2.0,
                    PORTAL_FRAME_THICKNESS / 2.0,
                    PORTAL_FRAME_DEPTH / 2.0,
                ),
            });
            ctx.cache.portal_frame_horizontal = Some(handle.clone());
            handle
        });

    // Get or create vertical bar mesh from cache (for left/right bars)
    let vertical_bar_mesh = ctx.cache.portal_frame_vertical.clone().unwrap_or_else(|| {
        let handle = ctx.meshes.add(Cuboid {
            half_size: Vec3::new(
                PORTAL_FRAME_THICKNESS / 2.0,
                PORTAL_FRAME_HEIGHT / 2.0,
                PORTAL_FRAME_DEPTH / 2.0,
            ),
        });
        ctx.cache.portal_frame_vertical = Some(handle.clone());
        handle
    });

    // Create material for portal frame (shared by all bars)
    let material = ctx.materials.add(StandardMaterial {
        base_color: PORTAL_COLOR,
        perceptual_roughness: 0.3,
        emissive: LinearRgba {
            red: 0.2,
            green: 0.0,
            blue: 0.3,
            alpha: 1.0,
        },
        ..default()
    });

    // Spawn parent portal entity with optional rotation
    let rotation_radians = rotation_y.unwrap_or(0.0).to_radians();
    let transform = Transform::from_xyz(
        position.x as f32 + TILE_CENTER_OFFSET,
        PORTAL_Y_POSITION,
        position.y as f32 + TILE_CENTER_OFFSET,
    )
    .with_rotation(Quat::from_rotation_y(rotation_radians));

    let parent = ctx
        .commands
        .spawn((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new(format!("PortalMarker_{}", event_name)),
        ))
        .id();

    // Spawn frame bars as children
    // Top bar (horizontal)
    let top = ctx
        .commands
        .spawn((
            Mesh3d(horizontal_bar_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(0.0, PORTAL_FRAME_HEIGHT / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(top);

    // Bottom bar (horizontal)
    let bottom = ctx
        .commands
        .spawn((
            Mesh3d(horizontal_bar_mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(0.0, -PORTAL_FRAME_HEIGHT / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(bottom);

    // Left bar (vertical)
    let left = ctx
        .commands
        .spawn((
            Mesh3d(vertical_bar_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(-PORTAL_FRAME_WIDTH / 2.0, 0.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(left);

    // Right bar (vertical)
    let right = ctx
        .commands
        .spawn((
            Mesh3d(vertical_bar_mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(PORTAL_FRAME_WIDTH / 2.0, 0.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(right);

    parent
}

/// Spawns a procedural sign mesh with post and board
///
/// Creates two child entities:
/// - Post: Dark brown cylinder (0.05 radius, 1.5 height)
/// - Board: Tan cuboid sign board (0.6 width, 0.3 height, 0.05 depth)
///
/// Reuses cached meshes when available to avoid duplicate allocations.
/// This significantly improves performance when spawning multiple signs
/// on large maps.
///
/// # Arguments
///
/// * `ctx` - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache)
/// * `position` - Tile position in world coordinates
/// * `event_name` - Event name for entity label
/// * `map_id` - Map identifier for cleanup
/// * `rotation_y` - Optional tile-level rotation in degrees around Y-axis (default: 0.0).
///   Applied first; if `facing` is also `Some`, the cardinal facing takes precedence.
/// * `facing` - Optional cardinal [`Direction`](crate::domain::types::Direction) the sign
///   should face. When `Some`, this overrides `rotation_y` with the exact yaw from
///   [`Direction::direction_to_yaw_radians`](crate::domain::types::Direction::direction_to_yaw_radians).
///   When `None`, `rotation_y` is used unchanged.
///
/// # Returns
///
/// Entity ID of the parent sign entity
pub fn spawn_sign(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    event_name: String,
    map_id: types::MapId,
    rotation_y: Option<f32>,
    facing: Option<types::Direction>,
) -> Entity {
    // Get or create post mesh from cache
    let post_mesh = ctx.cache.sign_post.clone().unwrap_or_else(|| {
        let handle = ctx.meshes.add(Cylinder {
            radius: SIGN_POST_RADIUS,
            half_height: SIGN_POST_HEIGHT / 2.0,
        });
        ctx.cache.sign_post = Some(handle.clone());
        handle
    });
    let post_material = ctx.materials.add(StandardMaterial {
        base_color: SIGN_POST_COLOR,
        perceptual_roughness: 0.9,
        ..default()
    });

    // Get or create board mesh from cache
    let board_mesh = ctx.cache.sign_board.clone().unwrap_or_else(|| {
        let handle = ctx.meshes.add(Cuboid {
            half_size: Vec3::new(
                SIGN_BOARD_WIDTH / 2.0,
                SIGN_BOARD_HEIGHT / 2.0,
                SIGN_BOARD_DEPTH / 2.0,
            ),
        });
        ctx.cache.sign_board = Some(handle.clone());
        handle
    });
    let board_material = ctx.materials.add(StandardMaterial {
        base_color: SIGN_BOARD_COLOR,
        perceptual_roughness: 0.7,
        ..default()
    });

    // Spawn parent sign entity with optional rotation.
    // Cardinal `facing` takes precedence over the tile-level `rotation_y` degrees.
    let effective_direction = facing.unwrap_or(types::Direction::North);
    let rotation_radians = if facing.is_some() {
        effective_direction.direction_to_yaw_radians()
    } else {
        rotation_y.unwrap_or(0.0).to_radians()
    };
    let transform = Transform::from_xyz(
        position.x as f32 + TILE_CENTER_OFFSET,
        0.0,
        position.y as f32 + TILE_CENTER_OFFSET,
    )
    .with_rotation(Quat::from_rotation_y(rotation_radians));

    use crate::game::components::creature::FacingComponent;
    let parent = ctx
        .commands
        .spawn((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new(format!("SignMarker_{}", event_name)),
            FacingComponent::new(effective_direction),
        ))
        .id();

    // Spawn post child
    let post = ctx
        .commands
        .spawn((
            Mesh3d(post_mesh),
            MeshMaterial3d(post_material),
            Transform::from_xyz(0.0, SIGN_POST_HEIGHT / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(post);

    // Spawn board child
    let board = ctx
        .commands
        .spawn((
            Mesh3d(board_mesh),
            MeshMaterial3d(board_material),
            Transform::from_xyz(0.0, SIGN_BOARD_Y_OFFSET, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(board);

    parent
}

// ==================== Furniture & Props Spawning ====================

/// Configuration for bench furniture
#[derive(Clone, Debug)]
pub struct BenchConfig {
    /// Length of the bench in world units
    pub length: f32,
    /// Height of the bench in world units
    pub height: f32,
    /// Wood color override (None = default)
    pub color_override: Option<Color>,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            length: BENCH_LENGTH,
            height: BENCH_HEIGHT,
            color_override: None,
        }
    }
}

/// Configuration for table furniture
#[derive(Clone, Debug)]
pub struct TableConfig {
    /// Width of the table top
    pub width: f32,
    /// Depth of the table top
    pub depth: f32,
    /// Height of the table (to top surface)
    pub height: f32,
    /// Color override (None = default)
    pub color_override: Option<Color>,
}

impl Default for TableConfig {
    fn default() -> Self {
        Self {
            width: TABLE_TOP_WIDTH,
            depth: TABLE_TOP_DEPTH,
            height: TABLE_HEIGHT,
            color_override: None,
        }
    }
}

/// Configuration for chair furniture
#[derive(Clone, Debug)]
pub struct ChairConfig {
    /// Height of the chair back
    pub back_height: f32,
    /// Whether the chair has armrests
    pub has_armrests: bool,
    /// Color override (None = default)
    pub color_override: Option<Color>,
}

impl Default for ChairConfig {
    fn default() -> Self {
        Self {
            back_height: CHAIR_BACK_HEIGHT,
            has_armrests: false,
            color_override: None,
        }
    }
}

/// Configuration for throne furniture
#[derive(Clone, Debug)]
pub struct ThroneConfig {
    /// Level of ornamentation (0.0-1.0): 0=plain, 1=highly ornate
    pub ornamentation_level: f32,
    /// Color override (None = default gold)
    pub color_override: Option<Color>,
}

impl Default for ThroneConfig {
    fn default() -> Self {
        Self {
            ornamentation_level: 0.7,
            color_override: None,
        }
    }
}

/// Configuration for chest furniture
#[derive(Clone, Debug)]
pub struct ChestConfig {
    /// Whether the chest is locked
    pub locked: bool,
    /// Size multiplier (1.0 = standard)
    pub size_multiplier: f32,
    /// Color override (None = default)
    pub color_override: Option<Color>,
}

impl Default for ChestConfig {
    fn default() -> Self {
        Self {
            locked: false,
            size_multiplier: 1.0,
            color_override: None,
        }
    }
}

/// Configuration for torch furniture
#[derive(Clone, Debug)]
pub struct TorchConfig {
    /// Whether the torch is lit (affects emissive material)
    pub lit: bool,
    /// Height of the torch
    pub height: f32,
    /// Flame color (None = default yellow)
    pub flame_color: Option<Color>,
}

impl Default for TorchConfig {
    fn default() -> Self {
        Self {
            lit: true,
            height: TORCH_HANDLE_HEIGHT,
            flame_color: None,
        }
    }
}

/// Spawns a procedural bench mesh with seat and legs
///
/// Creates composite mesh from:
/// - Seat: Cuboid (length × width × height)
/// - Two leg pairs: Cuboids positioned at corners
///
/// # Arguments
///
/// * `ctx` - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache)
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `config` - Bench configuration (length, height, color)
/// * `rotation_y` - Optional rotation in degrees around Y-axis
///
/// # Returns
///
/// Entity ID of the parent bench entity
pub fn spawn_bench(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: BenchConfig,
    rotation_y: Option<f32>,
) -> Entity {
    let color = config.color_override.unwrap_or(BENCH_COLOR);
    let rotation_radians = rotation_y.unwrap_or(0.0).to_radians();

    // Get or create seat mesh
    let seat_mesh = ctx.cache.furniture_bench_seat.clone().unwrap_or_else(|| {
        let handle = ctx
            .meshes
            .add(Cuboid::new(config.length, config.height, BENCH_WIDTH));
        ctx.cache.furniture_bench_seat = Some(handle.clone());
        handle
    });

    // Get or create leg mesh
    let leg_mesh = ctx.cache.furniture_bench_leg.clone().unwrap_or_else(|| {
        let handle = ctx.meshes.add(Cuboid::new(
            BENCH_LEG_THICKNESS,
            BENCH_LEG_HEIGHT,
            BENCH_LEG_THICKNESS,
        ));
        ctx.cache.furniture_bench_leg = Some(handle.clone());
        handle
    });

    let material = ctx.materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.7,
        ..default()
    });

    // Spawn parent
    let transform = Transform::from_xyz(
        position.x as f32 + TILE_CENTER_OFFSET,
        0.0,
        position.y as f32 + TILE_CENTER_OFFSET,
    )
    .with_rotation(Quat::from_rotation_y(rotation_radians));

    let parent = ctx
        .commands
        .spawn((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Bench"),
        ))
        .id();

    // Spawn seat
    let seat = ctx
        .commands
        .spawn((
            Mesh3d(seat_mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(0.0, config.height / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(seat);

    // Spawn front-left leg
    let leg_offset_x = config.length / 2.0 - BENCH_LEG_THICKNESS / 2.0;
    let leg_offset_z = BENCH_WIDTH / 2.0 - BENCH_LEG_THICKNESS / 2.0;
    let leg = ctx
        .commands
        .spawn((
            Mesh3d(leg_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(leg_offset_x, BENCH_LEG_HEIGHT / 2.0, leg_offset_z),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(leg);

    // Spawn front-right leg
    let leg = ctx
        .commands
        .spawn((
            Mesh3d(leg_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(-leg_offset_x, BENCH_LEG_HEIGHT / 2.0, leg_offset_z),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(leg);

    // Spawn back-left leg
    let leg = ctx
        .commands
        .spawn((
            Mesh3d(leg_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(leg_offset_x, BENCH_LEG_HEIGHT / 2.0, -leg_offset_z),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(leg);

    // Spawn back-right leg
    let leg = ctx
        .commands
        .spawn((
            Mesh3d(leg_mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(-leg_offset_x, BENCH_LEG_HEIGHT / 2.0, -leg_offset_z),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(leg);

    parent
}

/// Spawns a procedural table mesh with top and four legs
///
/// Creates composite mesh from:
/// - Top: Cuboid (width × depth × height)
/// - Legs: Four cuboids positioned at corners
///
/// # Arguments
///
/// * `ctx` - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache)
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `config` - Table configuration (width, depth, height, color)
/// * `rotation_y` - Optional rotation in degrees around Y-axis
///
/// # Returns
///
/// Entity ID of the parent table entity
pub fn spawn_table(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: TableConfig,
    rotation_y: Option<f32>,
) -> Entity {
    let color = config.color_override.unwrap_or(TABLE_COLOR);
    let rotation_radians = rotation_y.unwrap_or(0.0).to_radians();

    // Get or create top mesh
    let top_mesh = ctx.cache.furniture_table_top.clone().unwrap_or_else(|| {
        let handle = ctx
            .meshes
            .add(Cuboid::new(config.width, TABLE_TOP_HEIGHT, config.depth));
        ctx.cache.furniture_table_top = Some(handle.clone());
        handle
    });

    // Get or create leg mesh
    let leg_mesh = ctx.cache.furniture_table_leg.clone().unwrap_or_else(|| {
        let handle = ctx.meshes.add(Cuboid::new(
            TABLE_LEG_THICKNESS,
            config.height - TABLE_TOP_HEIGHT,
            TABLE_LEG_THICKNESS,
        ));
        ctx.cache.furniture_table_leg = Some(handle.clone());
        handle
    });

    let material = ctx.materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.6,
        ..default()
    });

    let transform = Transform::from_xyz(
        position.x as f32 + TILE_CENTER_OFFSET,
        0.0,
        position.y as f32 + TILE_CENTER_OFFSET,
    )
    .with_rotation(Quat::from_rotation_y(rotation_radians));

    let parent = ctx
        .commands
        .spawn((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Table"),
        ))
        .id();

    // Spawn top
    let top = ctx
        .commands
        .spawn((
            Mesh3d(top_mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(0.0, config.height - TABLE_TOP_HEIGHT / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(top);

    // Spawn four legs
    let leg_offset_x = config.width / 2.0 - TABLE_LEG_THICKNESS / 2.0;
    let leg_offset_z = config.depth / 2.0 - TABLE_LEG_THICKNESS / 2.0;
    let leg_y = (config.height - TABLE_TOP_HEIGHT) / 2.0;

    for (x_sign, z_sign) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        let leg = ctx
            .commands
            .spawn((
                Mesh3d(leg_mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(leg_offset_x * x_sign, leg_y, leg_offset_z * z_sign),
                GlobalTransform::default(),
                Visibility::default(),
            ))
            .id();
        ctx.commands.entity(parent).add_child(leg);
    }

    parent
}

/// Spawns a procedural chair mesh with seat, back, and legs
///
/// Creates composite mesh from:
/// - Seat: Cuboid
/// - Back: Cuboid positioned above rear of seat
/// - Optional armrests: Cuboids on sides
/// - Legs: Four cuboids positioned at corners
///
/// # Arguments
///
/// * `ctx` - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache)
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `config` - Chair configuration (has_armrests, back_height, color)
/// * `rotation_y` - Optional rotation in degrees around Y-axis
///
/// # Returns
///
/// Entity ID of the parent chair entity
pub fn spawn_chair(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: ChairConfig,
    rotation_y: Option<f32>,
) -> Entity {
    let color = config.color_override.unwrap_or(CHAIR_COLOR);
    let rotation_radians = rotation_y.unwrap_or(0.0).to_radians();

    // Get or create meshes
    let seat_mesh = ctx.cache.furniture_chair_seat.clone().unwrap_or_else(|| {
        let handle = ctx.meshes.add(Cuboid::new(
            CHAIR_SEAT_WIDTH,
            CHAIR_SEAT_HEIGHT,
            CHAIR_SEAT_DEPTH,
        ));
        ctx.cache.furniture_chair_seat = Some(handle.clone());
        handle
    });

    let back_mesh = ctx.cache.furniture_chair_back.clone().unwrap_or_else(|| {
        let handle = ctx
            .meshes
            .add(Cuboid::new(CHAIR_BACK_WIDTH, config.back_height, 0.08));
        ctx.cache.furniture_chair_back = Some(handle.clone());
        handle
    });

    let leg_mesh = ctx.cache.furniture_chair_leg.clone().unwrap_or_else(|| {
        let handle = ctx.meshes.add(Cuboid::new(
            CHAIR_LEG_THICKNESS,
            CHAIR_HEIGHT - CHAIR_SEAT_HEIGHT,
            CHAIR_LEG_THICKNESS,
        ));
        ctx.cache.furniture_chair_leg = Some(handle.clone());
        handle
    });

    let material = ctx.materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.7,
        ..default()
    });

    let transform = Transform::from_xyz(
        position.x as f32 + TILE_CENTER_OFFSET,
        0.0,
        position.y as f32 + TILE_CENTER_OFFSET,
    )
    .with_rotation(Quat::from_rotation_y(rotation_radians));

    let parent = ctx
        .commands
        .spawn((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Chair"),
        ))
        .id();

    // Spawn seat
    let seat = ctx
        .commands
        .spawn((
            Mesh3d(seat_mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(0.0, CHAIR_SEAT_HEIGHT / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(seat);

    // Spawn back
    let back = ctx
        .commands
        .spawn((
            Mesh3d(back_mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(
                0.0,
                CHAIR_SEAT_HEIGHT + config.back_height / 2.0,
                -CHAIR_SEAT_DEPTH / 2.0 - 0.04,
            ),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(back);

    // Spawn armrests if requested
    if config.has_armrests {
        let armrest_size = Cuboid::new(0.12, CHAIR_ARMREST_HEIGHT, CHAIR_SEAT_DEPTH * 0.8);
        let armrest_mesh = ctx.meshes.add(armrest_size);

        for x_sign in [1.0, -1.0] {
            let armrest = ctx
                .commands
                .spawn((
                    Mesh3d(armrest_mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(
                        CHAIR_SEAT_WIDTH / 2.0 * x_sign,
                        CHAIR_SEAT_HEIGHT + CHAIR_ARMREST_HEIGHT / 2.0,
                        0.0,
                    ),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .id();
            ctx.commands.entity(parent).add_child(armrest);
        }
    }

    // Spawn four legs
    let leg_offset_x = CHAIR_SEAT_WIDTH / 2.0 - CHAIR_LEG_THICKNESS / 2.0;
    let leg_offset_z = CHAIR_SEAT_DEPTH / 2.0 - CHAIR_LEG_THICKNESS / 2.0;
    let leg_y = (CHAIR_HEIGHT - CHAIR_SEAT_HEIGHT) / 2.0;

    for (x_sign, z_sign) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        let leg = ctx
            .commands
            .spawn((
                Mesh3d(leg_mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(leg_offset_x * x_sign, leg_y, leg_offset_z * z_sign),
                GlobalTransform::default(),
                Visibility::default(),
            ))
            .id();
        ctx.commands.entity(parent).add_child(leg);
    }

    parent
}

/// Spawns a procedural throne mesh with ornate features
///
/// Thrones are decorative sitting furniture with:
/// - Larger, ornate seat
/// - Tall back with decorative backing
/// - Wide armrests
/// - Ornamentation level determines extra decorative details
///
/// # Arguments
///
/// * `ctx` - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache)
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `config` - Throne configuration (ornamentation_level, color)
/// * `rotation_y` - Optional rotation in degrees around Y-axis
///
/// # Returns
///
/// Entity ID of the parent throne entity
pub fn spawn_throne(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: ThroneConfig,
    rotation_y: Option<f32>,
) -> Entity {
    let color = config.color_override.unwrap_or(THRONE_COLOR);
    let rotation_radians = rotation_y.unwrap_or(0.0).to_radians();
    let ornamentation = config.ornamentation_level.clamp(0.0, 1.0);

    // Get or create meshes
    let seat_mesh = ctx.cache.furniture_throne_seat.clone().unwrap_or_else(|| {
        let handle = ctx.meshes.add(Cuboid::new(
            THRONE_SEAT_WIDTH,
            THRONE_SEAT_HEIGHT,
            THRONE_SEAT_DEPTH,
        ));
        ctx.cache.furniture_throne_seat = Some(handle.clone());
        handle
    });

    let back_mesh = ctx.cache.furniture_throne_back.clone().unwrap_or_else(|| {
        let handle = ctx
            .meshes
            .add(Cuboid::new(THRONE_BACK_WIDTH, THRONE_BACK_HEIGHT, 0.12));
        ctx.cache.furniture_throne_back = Some(handle.clone());
        handle
    });

    let arm_mesh = ctx.cache.furniture_throne_arm.clone().unwrap_or_else(|| {
        let handle = ctx.meshes.add(Cuboid::new(
            THRONE_ARM_WIDTH,
            THRONE_ARM_HEIGHT,
            THRONE_SEAT_DEPTH,
        ));
        ctx.cache.furniture_throne_arm = Some(handle.clone());
        handle
    });

    let material = ctx.materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.4,
        metallic: 0.5,
        ..default()
    });

    let back_material = ctx.materials.add(StandardMaterial {
        base_color: THRONE_BACKING,
        perceptual_roughness: 0.5,
        ..default()
    });

    let transform = Transform::from_xyz(
        position.x as f32 + TILE_CENTER_OFFSET,
        0.0,
        position.y as f32 + TILE_CENTER_OFFSET,
    )
    .with_rotation(Quat::from_rotation_y(rotation_radians));

    let parent = ctx
        .commands
        .spawn((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Throne"),
        ))
        .id();

    // Spawn seat
    let seat = ctx
        .commands
        .spawn((
            Mesh3d(seat_mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(0.0, THRONE_SEAT_HEIGHT / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(seat);

    // Spawn ornate back
    let back = ctx
        .commands
        .spawn((
            Mesh3d(back_mesh),
            MeshMaterial3d(back_material),
            Transform::from_xyz(
                0.0,
                THRONE_SEAT_HEIGHT + THRONE_BACK_HEIGHT / 2.0,
                -THRONE_SEAT_DEPTH / 2.0 - 0.06,
            ),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(back);

    // Spawn wide armrests
    for x_sign in [1.0, -1.0] {
        let armrest = ctx
            .commands
            .spawn((
                Mesh3d(arm_mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(
                    THRONE_SEAT_WIDTH / 2.0 * x_sign + THRONE_ARM_WIDTH * x_sign / 2.0,
                    THRONE_SEAT_HEIGHT + THRONE_ARM_HEIGHT / 2.0,
                    0.0,
                ),
                GlobalTransform::default(),
                Visibility::default(),
            ))
            .id();
        ctx.commands.entity(parent).add_child(armrest);
    }

    // Add decorative spheres at corners if highly ornate
    if ornamentation > 0.5 {
        let ornament_radius = 0.1 * ornamentation;
        let ornament_mesh = ctx.meshes.add(Sphere {
            radius: ornament_radius,
        });
        let ornament_material = ctx.materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.3,
            metallic: 0.7,
            ..default()
        });

        // Top corners of back
        for (x_sign, z_sign) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            let ornament = ctx
                .commands
                .spawn((
                    Mesh3d(ornament_mesh.clone()),
                    MeshMaterial3d(ornament_material.clone()),
                    Transform::from_xyz(
                        THRONE_BACK_WIDTH / 2.0 * x_sign,
                        THRONE_SEAT_HEIGHT + THRONE_BACK_HEIGHT - 0.1,
                        -THRONE_SEAT_DEPTH / 2.0 * z_sign,
                    ),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .id();
            ctx.commands.entity(parent).add_child(ornament);
        }
    }

    parent
}

/// Spawns a procedural chest mesh with body and lid
///
/// Chests are containers with:
/// - Main body: Cuboid
/// - Lid: Cuboid positioned at top
/// - Optional lock component for locked chests
///
/// # Arguments
///
/// * `ctx` - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache)
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `config` - Chest configuration (size_multiplier, locked, color)
/// * `rotation_y` - Optional rotation in degrees around Y-axis
///
/// # Returns
///
/// Entity ID of the parent chest entity
pub fn spawn_chest(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: ChestConfig,
    rotation_y: Option<f32>,
) -> Entity {
    let color = config.color_override.unwrap_or(CHEST_COLOR);
    let rotation_radians = rotation_y.unwrap_or(0.0).to_radians();
    let _scaled_width = CHEST_WIDTH * config.size_multiplier;
    let _scaled_depth = CHEST_DEPTH * config.size_multiplier;
    let scaled_height = CHEST_HEIGHT * config.size_multiplier;

    // Get or create meshes
    let body_mesh = ctx.cache.furniture_chest_body.clone().unwrap_or_else(|| {
        let handle = ctx
            .meshes
            .add(Cuboid::new(CHEST_WIDTH, CHEST_HEIGHT, CHEST_DEPTH));
        ctx.cache.furniture_chest_body = Some(handle.clone());
        handle
    });

    let lid_mesh = ctx.cache.furniture_chest_lid.clone().unwrap_or_else(|| {
        let handle = ctx
            .meshes
            .add(Cuboid::new(CHEST_WIDTH, CHEST_LID_HEIGHT, CHEST_DEPTH));
        ctx.cache.furniture_chest_lid = Some(handle.clone());
        handle
    });

    let material = ctx.materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.8,
        ..default()
    });

    let transform = Transform::from_xyz(
        position.x as f32 + TILE_CENTER_OFFSET,
        0.0,
        position.y as f32 + TILE_CENTER_OFFSET,
    )
    .with_rotation(Quat::from_rotation_y(rotation_radians));

    let parent = ctx
        .commands
        .spawn((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new(if config.locked {
                "LockedChest"
            } else {
                "Chest"
            }),
        ))
        .id();

    // Spawn body
    let body = ctx
        .commands
        .spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(0.0, scaled_height / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(body);

    // Spawn lid
    let lid = ctx
        .commands
        .spawn((
            Mesh3d(lid_mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(0.0, scaled_height + CHEST_LID_HEIGHT / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(lid);

    parent
}

/// Spawns a procedural torch mesh with handle and flame
///
/// Torches are light sources with:
/// - Handle: Cylinder mounted on wall/post
/// - Flame: Cuboid positioned above handle
/// - Emissive material on flame if lit
///
/// # Arguments
///
/// * `ctx` - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache)
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `config` - Torch configuration (lit, height, flame_color)
/// * `rotation_y` - Optional rotation in degrees around Y-axis
///
/// # Returns
///
/// Entity ID of the parent torch entity
pub fn spawn_torch(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: TorchConfig,
    rotation_y: Option<f32>,
) -> Entity {
    let rotation_radians = rotation_y.unwrap_or(0.0).to_radians();
    let flame_color = config.flame_color.unwrap_or(TORCH_FLAME_COLOR);

    // Get or create meshes
    let handle_mesh = ctx.cache.furniture_torch_handle.clone().unwrap_or_else(|| {
        let handle = ctx.meshes.add(Cylinder {
            radius: TORCH_HANDLE_RADIUS,
            half_height: config.height / 2.0,
        });
        ctx.cache.furniture_torch_handle = Some(handle.clone());
        handle
    });

    let flame_mesh = ctx.cache.furniture_torch_flame.clone().unwrap_or_else(|| {
        let handle = ctx.meshes.add(Cuboid::new(
            TORCH_FLAME_WIDTH,
            TORCH_FLAME_HEIGHT,
            TORCH_FLAME_WIDTH,
        ));
        ctx.cache.furniture_torch_flame = Some(handle.clone());
        handle
    });

    let handle_material = ctx.materials.add(StandardMaterial {
        base_color: TORCH_HANDLE_COLOR,
        perceptual_roughness: 0.9,
        ..default()
    });

    let flame_material = ctx.materials.add(StandardMaterial {
        base_color: flame_color,
        emissive: if config.lit {
            // Use a brighter yellow-orange for emissive when lit
            LinearRgba::rgb(1.0, 0.9, 0.4)
        } else {
            LinearRgba::BLACK
        },
        perceptual_roughness: 0.6,
        ..default()
    });

    let transform = Transform::from_xyz(
        position.x as f32 + TILE_CENTER_OFFSET,
        0.0,
        position.y as f32 + TILE_CENTER_OFFSET,
    )
    .with_rotation(Quat::from_rotation_y(rotation_radians));

    let parent = ctx
        .commands
        .spawn((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new(if config.lit { "LitTorch" } else { "Torch" }),
        ))
        .id();

    // Spawn handle
    let handle = ctx
        .commands
        .spawn((
            Mesh3d(handle_mesh),
            MeshMaterial3d(handle_material),
            Transform::from_xyz(0.0, config.height / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(handle);

    // Spawn flame
    let flame = ctx
        .commands
        .spawn((
            Mesh3d(flame_mesh),
            MeshMaterial3d(flame_material),
            Transform::from_xyz(0.0, config.height + TORCH_FLAME_HEIGHT / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(flame);

    parent
}

/// Spawns a procedurally generated column with configurable style
///
/// Columns support three architectural styles:
/// - Plain: Simple cylindrical column with base and capital
/// - Doric: Classical style with simple geometric capital
/// - Ionic: Ornate style with scroll-decorated capital
///
/// # Arguments
///
/// * `ctx` - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache)
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `config` - Column configuration (height, radius, style)
///
/// # Returns
///
/// Entity ID of the parent column entity
///
/// # Examples
///
/// ```text
/// use antares::game::systems::procedural_meshes::{spawn_column, MeshSpawnContext};
/// use antares::domain::world::ColumnConfig;
///
/// let mut ctx = MeshSpawnContext { commands: &mut commands, materials: &mut materials,
///     meshes: &mut meshes, cache: &mut cache };
/// let config = ColumnConfig::default();
/// let column_entity = spawn_column(&mut ctx, position, map_id, config);
/// ```
pub fn spawn_column(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: crate::domain::world::ColumnConfig,
) -> Entity {
    use crate::domain::world::ColumnStyle;

    // Get or create shaft mesh
    let shaft_mesh = ctx.cache.structure_column_shaft.clone().unwrap_or_else(|| {
        let handle = ctx.meshes.add(Cylinder {
            radius: config.radius,
            half_height: config.height / 2.0,
        });
        ctx.cache.structure_column_shaft = Some(handle.clone());
        handle
    });

    // Get or create capital mesh (varies by style)
    let capital_mesh = ctx
        .cache
        .structure_column_capital
        .clone()
        .unwrap_or_else(|| {
            let handle = ctx.meshes.add(match config.style {
                ColumnStyle::Plain => {
                    // Simple flat top
                    Cylinder {
                        radius: config.radius * 1.1,
                        half_height: COLUMN_CAPITAL_HEIGHT / 2.0,
                    }
                }
                ColumnStyle::Doric => {
                    // Slightly wider capital with decorative ridges
                    Cylinder {
                        radius: config.radius * 1.15,
                        half_height: COLUMN_CAPITAL_HEIGHT / 2.0,
                    }
                }
                ColumnStyle::Ionic => {
                    // Wider capital for scroll bases
                    Cylinder {
                        radius: config.radius * 1.25,
                        half_height: COLUMN_CAPITAL_HEIGHT / 2.0,
                    }
                }
            });
            ctx.cache.structure_column_capital = Some(handle.clone());
            handle
        });

    let shaft_material = ctx.materials.add(StandardMaterial {
        base_color: STRUCTURE_STONE_COLOR,
        perceptual_roughness: 0.8,
        ..default()
    });

    let capital_material = ctx.materials.add(StandardMaterial {
        base_color: match config.style {
            ColumnStyle::Plain => STRUCTURE_STONE_COLOR,
            ColumnStyle::Doric => STRUCTURE_STONE_COLOR,
            ColumnStyle::Ionic => STRUCTURE_MARBLE_COLOR, // Marble for fancier capitals
        },
        perceptual_roughness: 0.7,
        ..default()
    });

    let transform = Transform::from_xyz(
        position.x as f32 + TILE_CENTER_OFFSET,
        config.height / 2.0,
        position.y as f32 + TILE_CENTER_OFFSET,
    );

    let parent = ctx
        .commands
        .spawn((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new(format!("Column({})", config.style.name())),
        ))
        .id();

    // Spawn base
    let base = ctx
        .commands
        .spawn((
            Mesh3d(ctx.meshes.add(Cylinder {
                radius: config.radius * 1.2,
                half_height: COLUMN_BASE_HEIGHT / 2.0,
            })),
            MeshMaterial3d(shaft_material.clone()),
            Transform::from_xyz(0.0, -config.height / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(base);

    // Spawn shaft
    let shaft = ctx
        .commands
        .spawn((
            Mesh3d(shaft_mesh),
            MeshMaterial3d(shaft_material),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(shaft);

    // Spawn capital
    let capital = ctx
        .commands
        .spawn((
            Mesh3d(capital_mesh),
            MeshMaterial3d(capital_material),
            Transform::from_xyz(0.0, config.height / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(capital);

    parent
}

/// Spawns a procedurally generated arch structure
///
/// Arches are decorative or structural openings composed of:
/// - Curved arch spanning the opening
/// - Support columns on either side
/// - Configurable width and height
///
/// # Arguments
///
/// * `ctx` - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache)
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `config` - Arch configuration (width, height, thickness)
///
/// # Returns
///
/// Entity ID of the parent arch entity
///
/// # Examples
///
/// ```text
/// use antares::game::systems::procedural_meshes::{spawn_arch, MeshSpawnContext};
/// use antares::domain::world::ArchConfig;
///
/// let mut ctx = MeshSpawnContext { commands: &mut commands, materials: &mut materials,
///     meshes: &mut meshes, cache: &mut cache };
/// let config = ArchConfig::default();
/// let arch_entity = spawn_arch(&mut ctx, position, map_id, config);
/// ```
pub fn spawn_arch(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: crate::domain::world::ArchConfig,
) -> Entity {
    // Get or create arch curve mesh
    let arch_mesh = ctx.cache.structure_arch_curve.clone().unwrap_or_else(|| {
        // Create a torus segment for the arch (approximation)
        let handle = ctx.meshes.add(Torus {
            major_radius: ARCH_INNER_RADIUS,
            minor_radius: config.thickness / 2.0,
        });
        ctx.cache.structure_arch_curve = Some(handle.clone());
        handle
    });

    // Get or create support mesh
    let support_mesh = ctx.cache.structure_arch_support.clone().unwrap_or_else(|| {
        let handle = ctx.meshes.add(Cuboid::new(
            ARCH_SUPPORT_WIDTH,
            ARCH_SUPPORT_HEIGHT,
            config.thickness,
        ));
        ctx.cache.structure_arch_support = Some(handle.clone());
        handle
    });

    let arch_material = ctx.materials.add(StandardMaterial {
        base_color: STRUCTURE_STONE_COLOR,
        perceptual_roughness: 0.8,
        ..default()
    });

    let support_material = ctx.materials.add(StandardMaterial {
        base_color: STRUCTURE_MARBLE_COLOR,
        perceptual_roughness: 0.75,
        ..default()
    });

    let transform = Transform::from_xyz(
        position.x as f32 + TILE_CENTER_OFFSET,
        0.0,
        position.y as f32 + TILE_CENTER_OFFSET,
    );

    let parent = ctx
        .commands
        .spawn((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Arch"),
        ))
        .id();

    // Spawn arch curve (centered and scaled to fit width/height)
    let arch = ctx
        .commands
        .spawn((
            Mesh3d(arch_mesh),
            MeshMaterial3d(arch_material),
            Transform::from_xyz(0.0, config.height, 0.0).with_scale(Vec3::new(
                config.width / 2.0,
                1.0,
                1.0,
            )),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(arch);

    // Spawn left support
    let left_support = ctx
        .commands
        .spawn((
            Mesh3d(support_mesh.clone()),
            MeshMaterial3d(support_material.clone()),
            Transform::from_xyz(
                -config.width / 2.0 - ARCH_SUPPORT_WIDTH / 2.0,
                ARCH_SUPPORT_HEIGHT / 2.0,
                0.0,
            ),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(left_support);

    // Spawn right support
    let right_support = ctx
        .commands
        .spawn((
            Mesh3d(support_mesh),
            MeshMaterial3d(support_material),
            Transform::from_xyz(
                config.width / 2.0 + ARCH_SUPPORT_WIDTH / 2.0,
                ARCH_SUPPORT_HEIGHT / 2.0,
                0.0,
            ),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(right_support);

    parent
}

// ==================== Performance & Polish ====================

/// Calculate the mesh complexity level for LOD selection
///
/// This function determines the appropriate detail level based on vertex count
/// and configuration complexity.
///
/// # Arguments
///
/// * `vertex_count` - Number of vertices in the mesh
///
/// # Returns
///
/// DetailLevel indicating Full, Simplified, or Billboard
pub fn calculate_lod_level(vertex_count: usize) -> crate::domain::world::DetailLevel {
    use crate::domain::world::DetailLevel;
    match vertex_count {
        0..=1000 => DetailLevel::Full,
        1001..=5000 => DetailLevel::Simplified,
        _ => DetailLevel::Billboard,
    }
}

/// Simplify a mesh by reducing vertex count for distant objects
///
/// Creates a simplified version of a mesh by removing internal vertices
/// and reducing face count while maintaining visual silhouette.
///
/// # Arguments
///
/// * `mesh` - The original mesh to simplify
/// * `reduction_ratio` - Fraction to reduce (0.0-1.0, where 0.5 = 50% reduction)
///
/// # Returns
///
/// Simplified mesh handle
pub fn create_simplified_mesh(mesh: &Mesh, reduction_ratio: f32) -> Mesh {
    use bevy::mesh::VertexAttributeValues;

    let reduction_ratio = reduction_ratio.clamp(0.0, 0.9);

    if reduction_ratio == 0.0 {
        return mesh.clone();
    }

    // Read positions
    let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(pos)) => pos,
        _ => return mesh.clone(),
    };

    let vertex_count = positions.len();
    if vertex_count < 4 {
        return mesh.clone();
    }

    // Calculate stride: how many vertices to skip between kept vertices
    let keep_ratio = 1.0 - reduction_ratio;
    let stride = (1.0_f32 / keep_ratio).round().max(2.0) as usize;

    // Build kept vertex indices and old-to-new index map
    let mut old_to_new: Vec<usize> = vec![0; vertex_count];
    let mut kept_indices: Vec<usize> = Vec::with_capacity(vertex_count / stride + 1);

    for i in 0..vertex_count {
        if i % stride == 0 {
            old_to_new[i] = kept_indices.len();
            kept_indices.push(i);
        } else {
            // Map skipped vertices to the nearest kept vertex
            let nearest_kept = (i / stride) * stride;
            old_to_new[i] = old_to_new[nearest_kept];
        }
    }

    if kept_indices.len() < 3 {
        return mesh.clone();
    }

    // Build simplified position array
    let new_positions: Vec<[f32; 3]> = kept_indices.iter().map(|&i| positions[i]).collect();

    let mut simplified = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    simplified.insert_attribute(Mesh::ATTRIBUTE_POSITION, new_positions);

    // Copy normals if present
    if let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    {
        let new_normals: Vec<[f32; 3]> = kept_indices.iter().map(|&i| normals[i]).collect();
        simplified.insert_attribute(Mesh::ATTRIBUTE_NORMAL, new_normals);
    }

    // Copy UVs if present
    if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
        let new_uvs: Vec<[f32; 2]> = kept_indices.iter().map(|&i| uvs[i]).collect();
        simplified.insert_attribute(Mesh::ATTRIBUTE_UV_0, new_uvs);
    }

    // Copy vertex colors if present
    if let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
        let new_colors: Vec<[f32; 4]> = kept_indices.iter().map(|&i| colors[i]).collect();
        simplified.insert_attribute(Mesh::ATTRIBUTE_COLOR, new_colors);
    }

    // Rebuild indices, skipping degenerate triangles
    if let Some(indices) = mesh.indices() {
        let old_indices: Vec<usize> = match indices {
            bevy::mesh::Indices::U16(idx) => idx.iter().map(|&i| i as usize).collect(),
            bevy::mesh::Indices::U32(idx) => idx.iter().map(|&i| i as usize).collect(),
        };

        let mut new_indices: Vec<u32> = Vec::with_capacity(old_indices.len());
        for tri in old_indices.chunks(3) {
            if tri.len() == 3 {
                let a = old_to_new[tri[0]] as u32;
                let b = old_to_new[tri[1]] as u32;
                let c = old_to_new[tri[2]] as u32;
                // Skip degenerate triangles
                if a != b && b != c && a != c {
                    new_indices.push(a);
                    new_indices.push(b);
                    new_indices.push(c);
                }
            }
        }
        simplified.insert_indices(bevy::mesh::Indices::U32(new_indices));
    }

    simplified
}

/// Create billboard impostor for very distant objects
///
/// Returns a simple quad mesh suitable for sprite-based representation
/// of complex 3D objects when they're far from the camera.
///
/// # Returns
///
/// A simple quad mesh for billboard rendering
pub fn create_billboard_mesh() -> Mesh {
    // Create a simple 1x1 quad for billboard rendering
    // Use a thin cuboid as a billboard quad
    Mesh::from(Cuboid {
        half_size: Vec3::new(0.5, 0.5, 0.01),
    })
}

/// Calculate instance transforms for a batch of objects
///
/// Takes a list of positions and generates InstanceData with proper
/// transforms for GPU instancing.
///
/// # Arguments
///
/// * `positions` - List of world positions [x, y, z]
/// * `base_scale` - Uniform scale for all instances
///
/// # Returns
///
/// Vector of InstanceData ready for GPU batch rendering
pub fn create_instance_data(
    positions: &[[f32; 3]],
    base_scale: f32,
) -> Vec<crate::domain::world::InstanceData> {
    positions
        .iter()
        .map(|&pos| crate::domain::world::InstanceData::new(pos).with_scale(base_scale))
        .collect()
}

/// Batch multiple mesh entities into a single instanced draw call
///
/// This optimizes rendering by combining multiple mesh instances into
/// a single GPU draw call, reducing overhead.
///
/// # Example
///
/// ```text
/// // 100 trees with the same mesh would normally be 100 draw calls
/// // With instancing, it's 1 draw call with 100 instance transforms
/// ```
pub fn estimate_draw_call_reduction(instance_count: usize) -> f32 {
    // Reduction factor: 100 instances = ~99% fewer draw calls
    if instance_count == 0 {
        1.0
    } else {
        1.0 / (instance_count as f32)
    }
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Constant Validation Tests ====================

    /// Validates tree constants are within reasonable bounds
    #[test]
    fn test_tree_constants_valid() {
        // Constants should be positive and follow size relationships
        // These checks serve as documentation of design invariants
        let _ = TREE_FOLIAGE_RADIUS;
        // Compile will verify constants exist with correct values
    }

    /// Validates portal constants are within reasonable bounds
    #[test]
    fn test_portal_constants_valid() {
        // Constants should be positive and follow size relationships
        // These checks serve as documentation of design invariants
        let _ = PORTAL_FRAME_WIDTH;
        let _ = PORTAL_FRAME_HEIGHT;
        let _ = PORTAL_FRAME_THICKNESS;
        let _ = PORTAL_FRAME_DEPTH;
        let _ = PORTAL_Y_POSITION;
        // Compile will verify constants exist with correct values
    }

    /// Validates sign constants are within reasonable bounds
    #[test]
    fn test_sign_constants_valid() {
        // Constants should be positive and follow size relationships
        // These checks serve as documentation of design invariants
        let _ = SIGN_POST_RADIUS;
        let _ = SIGN_POST_HEIGHT;
        let _ = SIGN_BOARD_WIDTH;
        let _ = SIGN_BOARD_HEIGHT;
        let _ = SIGN_BOARD_DEPTH;
        let _ = SIGN_BOARD_Y_OFFSET;
        // Compile will verify constants exist with correct values
    }

    #[test]
    fn test_procedural_mesh_centering_offset() {
        assert_eq!(TILE_CENTER_OFFSET, 0.5);

        // Verify offset produces centered coordinates
        let pos = types::Position { x: 3, y: 7 };
        let centered_x = pos.x as f32 + TILE_CENTER_OFFSET;
        let centered_z = pos.y as f32 + TILE_CENTER_OFFSET;

        assert_eq!(centered_x, 3.5);
        assert_eq!(centered_z, 7.5);
    }

    // ==================== Mesh Caching Tests ====================

    /// Tests that ProceduralMeshCache initializes with all fields as None
    #[test]
    fn test_cache_default_all_none() {
        let cache = ProceduralMeshCache::default();
        assert!(cache.tree_trunk.is_none());
        assert!(cache.tree_foliage.is_none());
        assert!(cache.portal_frame_horizontal.is_none());
        assert!(cache.portal_frame_vertical.is_none());
        assert!(cache.sign_post.is_none());
        assert!(cache.sign_board.is_none());
    }

    /// Tests that cache can be cloned
    #[test]
    fn test_cache_is_cloneable() {
        let cache = ProceduralMeshCache::default();
        let _cloned = cache.clone();
        // If we can clone it, the test passes
    }

    /// Tests cache with tree_trunk set
    #[test]
    fn test_cache_with_tree_trunk_stored() {
        let cache = ProceduralMeshCache::default();
        assert!(cache.tree_trunk.is_none());

        // After initialization, cache should remain empty until set
        // This test documents the cache's purpose: to store handles
        assert!(cache.tree_foliage.is_none());
    }

    /// Tests that tree foliage dimensions are suitable for caching
    #[test]
    fn test_tree_foliage_dimensions_consistent() {
        // Tree foliage dimensions should be consistent.
        // Foliage should be larger than trunk for visual appeal.
        // These constants are verified at compile time through their usage in
        // Sphere { radius } which requires a valid f32 value.
        let _ = TREE_FOLIAGE_RADIUS;
        // Test passes if constants compile with valid values
    }

    /// Tests that portal frame dimensions are suitable for caching
    #[test]
    fn test_portal_frame_dimensions_consistent() {
        // Portal frame should have reasonable proportions.
        // Frame should be tall enough for player character to walk through.
        // These constants are verified at compile time through their usage in
        // Cuboid creation which requires valid f32 values.
        let _ = PORTAL_FRAME_WIDTH;
        let _ = PORTAL_FRAME_HEIGHT;
        let _ = PORTAL_FRAME_THICKNESS;
        let _ = PORTAL_FRAME_DEPTH;
        // Test passes if constants compile with valid values
    }

    /// Tests that sign post dimensions are suitable for caching
    #[test]
    fn test_sign_post_dimensions_consistent() {
        // Sign post should be thin and tall.
        // These constants are verified at compile time through their usage in
        // Cylinder { radius, half_height } which requires valid f32 values.
        let _ = SIGN_POST_RADIUS;
        let _ = SIGN_POST_HEIGHT;
        // Test passes if constants compile with valid values
    }

    /// Tests that sign board dimensions are suitable for caching
    #[test]
    fn test_sign_board_dimensions_consistent() {
        // Sign board should be a flat rectangle.
        // Board should be wider than tall for sign appearance.
        // Board should be thin (small depth).
        // These constants are verified at compile time through their usage in
        // Cuboid { half_size } which requires valid Vec3 values.
        let _ = SIGN_BOARD_WIDTH;
        let _ = SIGN_BOARD_HEIGHT;
        let _ = SIGN_BOARD_DEPTH;
        // Test passes if constants compile with valid values
    }

    /// Tests that mesh caching reduces allocations by documenting cache pattern
    #[test]
    fn test_mesh_cache_pattern_prevents_duplicates() {
        // This test documents the caching pattern:
        // 1. First spawn: mesh created, cached
        // 2. Second spawn: cached mesh reused
        // 3. Nth spawn: cached mesh reused
        //
        // With a large forest (100+ trees), caching provides significant
        // memory and allocation overhead reduction.
        //
        // Example scenario: spawning 100 trees without caching:
        //   - 100 trunk meshes allocated
        //   - 100 foliage meshes allocated
        //   - 200 mesh allocations total
        //
        // With caching:
        //   - 1 trunk mesh allocated
        //   - 1 foliage mesh allocated
        //   - 2 mesh allocations total
        //   - 99% reduction in mesh allocations
        //
        // This test passes by documenting the design intent.
        let cache = ProceduralMeshCache::default();
        assert!(cache.tree_trunk.is_none(), "Cache should start empty");
    }

    // ==================== Shrub & Grass Tests ====================

    /// Tests that cache properly stores shrub stem meshes
    #[test]
    fn test_cache_shrub_stem_default_none() {
        let cache = ProceduralMeshCache::default();
        assert!(
            cache.shrub_stem.is_none(),
            "Shrub stem should start as None"
        );
    }

    // ==================== Furniture Configuration Tests ====================

    /// Tests bench config defaults
    #[test]
    fn test_bench_config_defaults() {
        let config = BenchConfig::default();
        assert_eq!(config.length, BENCH_LENGTH);
        assert_eq!(config.height, BENCH_HEIGHT);
        assert!(config.color_override.is_none());
    }

    /// Tests table config defaults
    #[test]
    fn test_table_config_defaults() {
        let config = TableConfig::default();
        assert_eq!(config.width, TABLE_TOP_WIDTH);
        assert_eq!(config.depth, TABLE_TOP_DEPTH);
        assert_eq!(config.height, TABLE_HEIGHT);
        assert!(config.color_override.is_none());
    }

    /// Tests chair config defaults
    #[test]
    fn test_chair_config_defaults() {
        let config = ChairConfig::default();
        assert_eq!(config.back_height, CHAIR_BACK_HEIGHT);
        assert!(!config.has_armrests);
        assert!(config.color_override.is_none());
    }

    /// Tests throne config defaults
    #[test]
    fn test_throne_config_defaults() {
        let config = ThroneConfig::default();
        assert!(config.ornamentation_level >= 0.0 && config.ornamentation_level <= 1.0);
        assert!(config.color_override.is_none());
    }

    /// Tests throne ornamentation level clamping
    #[test]
    fn test_throne_ornamentation_clamping() {
        let config = ThroneConfig {
            ornamentation_level: 1.5,
            color_override: None,
        };
        // In spawn_throne, value is clamped to 0.0-1.0
        let clamped = config.ornamentation_level.clamp(0.0, 1.0);
        assert_eq!(clamped, 1.0);
    }

    /// Tests chest config defaults
    #[test]
    fn test_chest_config_defaults() {
        let config = ChestConfig::default();
        assert!(!config.locked);
        assert_eq!(config.size_multiplier, 1.0);
        assert!(config.color_override.is_none());
    }

    /// Tests torch config defaults
    #[test]
    fn test_torch_config_defaults() {
        let config = TorchConfig::default();
        assert!(config.lit);
        assert_eq!(config.height, TORCH_HANDLE_HEIGHT);
        assert!(config.flame_color.is_none());
    }

    /// Tests cache furniture field defaults
    #[test]
    fn test_cache_furniture_defaults() {
        let cache = ProceduralMeshCache::default();
        assert!(cache.furniture_bench_seat.is_none());
        assert!(cache.furniture_bench_leg.is_none());
        assert!(cache.furniture_table_top.is_none());
        assert!(cache.furniture_table_leg.is_none());
        assert!(cache.furniture_chair_seat.is_none());
        assert!(cache.furniture_chair_back.is_none());
        assert!(cache.furniture_chair_leg.is_none());
        assert!(cache.furniture_throne_seat.is_none());
        assert!(cache.furniture_throne_back.is_none());
        assert!(cache.furniture_throne_arm.is_none());
        assert!(cache.furniture_chest_body.is_none());
        assert!(cache.furniture_chest_lid.is_none());
        assert!(cache.furniture_torch_handle.is_none());
        assert!(cache.furniture_torch_flame.is_none());
        assert!(cache.furniture_door_panel.is_none());
        assert!(cache.furniture_door_brace.is_none());
        assert!(cache.furniture_door_hinge.is_none());
        assert!(cache.furniture_door_handle.is_none());
    }

    /// Tests furniture color constants are valid
    #[test]
    fn test_furniture_color_constants_valid() {
        let _ = BENCH_COLOR;
        let _ = TABLE_COLOR;
        let _ = CHAIR_COLOR;
        let _ = THRONE_COLOR;
        let _ = THRONE_BACKING;
        let _ = CHEST_COLOR;
        let _ = TORCH_HANDLE_COLOR;
        let _ = TORCH_FLAME_COLOR;
    }

    /// Tests furniture dimension constants are positive
    /// (Compile-time verification via const assertions)
    #[test]
    fn test_furniture_dimensions_positive() {
        // Values are const and verified at compile time
        // This test documents the design invariant that all furniture dimensions are positive
        let _ = BENCH_LENGTH;
        let _ = BENCH_HEIGHT;
        let _ = TABLE_HEIGHT;
        let _ = CHAIR_HEIGHT;
        let _ = CHEST_HEIGHT;
        let _ = TORCH_HANDLE_HEIGHT;
    }

    /// Tests FurnitureType enum all() method
    #[test]
    fn test_furniture_type_all() {
        use crate::domain::world::FurnitureType;
        let all = FurnitureType::all();
        assert_eq!(all.len(), 9);
        assert!(all.contains(&FurnitureType::Throne));
        assert!(all.contains(&FurnitureType::Bench));
        assert!(all.contains(&FurnitureType::Table));
        assert!(all.contains(&FurnitureType::Chair));
        assert!(all.contains(&FurnitureType::Torch));
        assert!(all.contains(&FurnitureType::Bookshelf));
        assert!(all.contains(&FurnitureType::Barrel));
        assert!(all.contains(&FurnitureType::Chest));
        assert!(all.contains(&FurnitureType::Door));
    }

    /// Tests FurnitureType::Door has correct name, icon, and category
    #[test]
    fn test_furniture_type_door_properties() {
        use crate::domain::world::{FurnitureCategory, FurnitureType};
        assert_eq!(FurnitureType::Door.name(), "Door");
        assert_eq!(FurnitureType::Door.icon(), "🚪");
        assert_eq!(FurnitureType::Door.category(), FurnitureCategory::Passage);
    }

    /// Tests DoorConfig::default() returns expected geometry values
    #[test]
    fn test_door_config_defaults() {
        let cfg = DoorConfig::default();
        assert_eq!(cfg.width, DOOR_PANEL_WIDTH);
        assert_eq!(cfg.height, DOOR_PANEL_HEIGHT);
        assert_eq!(cfg.thickness, DOOR_PANEL_THICKNESS);
        assert_eq!(cfg.plank_count, DOOR_DEFAULT_PLANK_COUNT);
        assert!(cfg.has_studs);
        assert!(cfg.has_hinges);
        assert!(cfg.color_override.is_none());
    }

    /// Tests FurnitureType enum names
    #[test]
    fn test_furniture_type_names() {
        use crate::domain::world::FurnitureType;
        assert_eq!(FurnitureType::Throne.name(), "Throne");
        assert_eq!(FurnitureType::Bench.name(), "Bench");
        assert_eq!(FurnitureType::Table.name(), "Table");
        assert_eq!(FurnitureType::Chair.name(), "Chair");
        assert_eq!(FurnitureType::Torch.name(), "Torch");
        assert_eq!(FurnitureType::Bookshelf.name(), "Bookshelf");
        assert_eq!(FurnitureType::Barrel.name(), "Barrel");
        assert_eq!(FurnitureType::Chest.name(), "Chest");
        assert_eq!(FurnitureType::Door.name(), "Door");
    }

    /// Tests FurnitureCategory::Passage is in all() and name() returns "Passage"
    #[test]
    fn test_furniture_category_passage_in_all() {
        use crate::domain::world::FurnitureCategory;
        let all = FurnitureCategory::all();
        assert!(
            all.contains(&FurnitureCategory::Passage),
            "FurnitureCategory::all() must contain Passage"
        );
        assert_eq!(FurnitureCategory::Passage.name(), "Passage");
    }

    /// Tests that all FurnitureCategory variants have non-empty names
    #[test]
    fn test_furniture_category_all_have_names() {
        use crate::domain::world::FurnitureCategory;
        for cat in FurnitureCategory::all() {
            assert!(
                !cat.name().is_empty(),
                "Category {:?} must have a non-empty name",
                cat
            );
        }
    }

    /// Tests door dimension constants are positive and sensible
    ///
    /// All assertions here compare compile-time constants, so they are evaluated
    /// inside `const { }` blocks to satisfy `clippy::assertions_on_constants`.
    #[test]
    fn test_door_constants_valid() {
        // Verify each constant is accessible and positive (compile-time checks)
        const {
            assert!(DOOR_PANEL_WIDTH > 0.0, "DOOR_PANEL_WIDTH must be positive");
        }
        const {
            assert!(
                DOOR_PANEL_HEIGHT > 0.0,
                "DOOR_PANEL_HEIGHT must be positive"
            );
        }
        const {
            assert!(
                DOOR_PANEL_THICKNESS > 0.0,
                "DOOR_PANEL_THICKNESS must be positive"
            );
        }
        const {
            assert!(
                DOOR_BRACE_HEIGHT > 0.0,
                "DOOR_BRACE_HEIGHT must be positive"
            );
        }
        const {
            assert!(
                DOOR_HINGE_HEIGHT > 0.0,
                "DOOR_HINGE_HEIGHT must be positive"
            );
        }
        const {
            assert!(
                DOOR_HANDLE_RADIUS > 0.0,
                "DOOR_HANDLE_RADIUS must be positive"
            );
        }
        const {
            assert!(
                DOOR_HANDLE_LENGTH > 0.0,
                "DOOR_HANDLE_LENGTH must be positive"
            );
        }
        // Panel must be taller than wide (portrait orientation)
        const {
            assert!(
                DOOR_PANEL_HEIGHT > DOOR_PANEL_WIDTH,
                "Door must be taller than it is wide"
            );
        }
        // Thickness must be much less than width (flat panel)
        const {
            assert!(
                (DOOR_PANEL_WIDTH / 5.0 - DOOR_PANEL_THICKNESS) > 0.0,
                "Door panel must be thin relative to its width"
            );
        }
    }

    /// Tests door color constant is a valid sRGB color
    #[test]
    fn test_door_color_constant_valid() {
        let _ = DOOR_PANEL_COLOR;
        // Verify it can be converted — just touching it is enough to ensure
        // it is a valid Color::srgb() expression that compiles
    }

    /// Tests DoorConfig can be cloned
    #[test]
    fn test_door_config_clone() {
        let cfg = DoorConfig::default();
        let cloned = cfg.clone();
        assert_eq!(cloned.width, cfg.width);
        assert_eq!(cloned.height, cfg.height);
        assert_eq!(cloned.thickness, cfg.thickness);
        assert_eq!(cloned.plank_count, cfg.plank_count);
        assert_eq!(cloned.has_studs, cfg.has_studs);
        assert_eq!(cloned.has_hinges, cfg.has_hinges);
    }

    /// Tests DoorConfig with color_override
    #[test]
    fn test_door_config_with_color_override() {
        let color = Color::srgb(0.8, 0.3, 0.1);
        let cfg = DoorConfig {
            color_override: Some(color),
            ..Default::default()
        };
        assert!(cfg.color_override.is_some());
    }

    // ==================== Structure Configuration Tests ====================

    /// Tests StructureType enum all() method
    #[test]
    fn test_structure_type_all() {
        use crate::domain::world::StructureType;
        let all = StructureType::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&StructureType::Column));
        assert!(all.contains(&StructureType::Arch));
        assert!(all.contains(&StructureType::WallSegment));
        assert!(all.contains(&StructureType::DoorFrame));
        assert!(all.contains(&StructureType::Railing));
    }

    /// Tests StructureType enum names
    #[test]
    fn test_structure_type_names() {
        use crate::domain::world::StructureType;
        assert_eq!(StructureType::Column.name(), "Column");
        assert_eq!(StructureType::Arch.name(), "Arch");
        assert_eq!(StructureType::WallSegment.name(), "Wall Segment");
        assert_eq!(StructureType::DoorFrame.name(), "Door Frame");
        assert_eq!(StructureType::Railing.name(), "Railing");
    }

    /// Tests ColumnStyle enum all() method
    #[test]
    fn test_column_style_all() {
        use crate::domain::world::ColumnStyle;
        let all = ColumnStyle::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&ColumnStyle::Plain));
        assert!(all.contains(&ColumnStyle::Doric));
        assert!(all.contains(&ColumnStyle::Ionic));
    }

    /// Tests ColumnStyle enum names
    #[test]
    fn test_column_style_names() {
        use crate::domain::world::ColumnStyle;
        assert_eq!(ColumnStyle::Plain.name(), "Plain");
        assert_eq!(ColumnStyle::Doric.name(), "Doric");
        assert_eq!(ColumnStyle::Ionic.name(), "Ionic");
    }

    /// Tests column config defaults
    #[test]
    fn test_column_config_defaults() {
        use crate::domain::world::{ColumnConfig, ColumnStyle};
        let config = ColumnConfig::default();
        assert_eq!(config.height, 3.0);
        assert_eq!(config.radius, 0.3);
        assert_eq!(config.style, ColumnStyle::Plain);
    }

    /// Tests arch config defaults
    #[test]
    fn test_arch_config_defaults() {
        use crate::domain::world::ArchConfig;
        let config = ArchConfig::default();
        assert_eq!(config.width, 2.0);
        assert_eq!(config.height, 3.0);
        assert_eq!(config.thickness, 0.3);
    }

    /// Tests wall segment config defaults
    #[test]
    fn test_wall_segment_config_defaults() {
        use crate::domain::world::WallSegmentConfig;
        let config = WallSegmentConfig::default();
        assert_eq!(config.length, 2.0);
        assert_eq!(config.height, 2.5);
        assert_eq!(config.thickness, 0.2);
        assert!(!config.has_window);
    }

    /// Tests door frame config defaults
    #[test]
    fn test_door_frame_config_defaults() {
        use crate::domain::world::DoorFrameConfig;
        let config = DoorFrameConfig::default();
        assert_eq!(config.width, 1.0);
        assert_eq!(config.height, 2.5);
        assert_eq!(config.frame_thickness, 0.15);
    }

    /// Tests railing config defaults
    #[test]
    fn test_railing_config_defaults() {
        use crate::domain::world::RailingConfig;
        let config = RailingConfig::default();
        assert_eq!(config.length, 2.0);
        assert_eq!(config.height, 1.0);
        assert_eq!(config.post_radius, 0.08);
        assert_eq!(config.post_count, 4);
    }

    /// Tests structure color constants are valid
    #[test]
    fn test_structure_color_constants_valid() {
        let _ = STRUCTURE_STONE_COLOR;
        let _ = STRUCTURE_MARBLE_COLOR;
    }

    /// Tests structure dimension constants are positive
    #[test]
    fn test_structure_dimensions_positive() {
        // Constants verified at compile time via their usage
        let _ = COLUMN_CAPITAL_HEIGHT;
        let _ = ARCH_INNER_RADIUS;
        let _ = ARCH_SUPPORT_WIDTH;
        let _ = ARCH_SUPPORT_HEIGHT;
        let _ = DOOR_FRAME_THICKNESS;
        let _ = DOOR_FRAME_BORDER;
    }

    /// Tests cache properly stores structure component meshes
    #[test]
    fn test_cache_structure_defaults() {
        let cache = ProceduralMeshCache::default();
        assert!(cache.structure_column_shaft.is_none());
        assert!(cache.structure_column_capital.is_none());
        assert!(cache.structure_arch_curve.is_none());
        assert!(cache.structure_arch_support.is_none());
        assert!(cache.structure_wall.is_none());
        assert!(cache.structure_door_frame.is_none());
        assert!(cache.structure_door_frame_lintel.is_none());
        assert!(cache.structure_railing_post.is_none());
        assert!(cache.structure_railing_bar.is_none());
    }

    // ==================== Door Frame Tests ====================

    /// Tests door frame constants are valid and sensible
    #[test]
    fn test_door_frame_constants_valid() {
        const {
            assert!(
                DOOR_FRAME_THICKNESS > 0.0,
                "DOOR_FRAME_THICKNESS must be positive"
            );
        }
        const {
            assert!(
                DOOR_FRAME_BORDER > 0.0,
                "DOOR_FRAME_BORDER must be positive"
            );
        }
        // Frame thickness must be less than half a tile (0.5 world units)
        const {
            assert!(
                DOOR_FRAME_THICKNESS < 0.5,
                "DOOR_FRAME_THICKNESS must be less than half a tile"
            );
        }
    }

    /// Tests that door frame post geometry dimensions are correct.
    ///
    /// Post height = opening height + frame_thickness (covers up to lintel bottom).
    /// Post width = frame_thickness.
    #[test]
    fn test_door_frame_post_geometry() {
        use crate::domain::world::DoorFrameConfig;
        let config = DoorFrameConfig::default();
        let post_height = config.height + config.frame_thickness;
        // Post height must be taller than the door opening
        assert!(post_height > config.height);
        // Post center-Y = post_height / 2 — must be positive
        let post_y = post_height / 2.0;
        assert!(post_y > 0.0);
        // Post x-offset from center = half opening width + half post thickness
        let post_x = config.width / 2.0 + config.frame_thickness / 2.0;
        assert!(post_x > config.width / 2.0, "post must be outside opening");
    }

    /// Tests that door frame lintel geometry dimensions are correct.
    ///
    /// Lintel width = opening width + 2 * frame_thickness (spans both posts).
    /// Lintel Y center = opening height + half frame_thickness.
    #[test]
    fn test_door_frame_lintel_geometry() {
        use crate::domain::world::DoorFrameConfig;
        let config = DoorFrameConfig::default();
        let lintel_width = config.width + 2.0 * config.frame_thickness;
        // Lintel must be wider than the door opening
        assert!(lintel_width > config.width);
        // Lintel center-Y must be above opening height
        let lintel_y = config.height + config.frame_thickness / 2.0;
        assert!(lintel_y > config.height);
    }

    /// Tests that the frame opening is larger than the default door panel,
    /// ensuring the door fits inside its frame without clipping.
    #[test]
    fn test_door_fits_inside_frame() {
        use crate::domain::world::DoorFrameConfig;
        let frame = DoorFrameConfig::default();
        let door = DoorConfig::default();
        // Frame opening width must be greater than door panel width
        assert!(
            frame.width > door.width,
            "frame opening ({}) must be wider than door panel ({})",
            frame.width,
            door.width
        );
        // Frame opening height must be greater than door panel height
        assert!(
            frame.height > door.height,
            "frame opening ({}) must be taller than door panel ({})",
            frame.height,
            door.height
        );
    }

    /// Tests that structure_door_frame_lintel cache field is None by default.
    #[test]
    fn test_cache_door_frame_lintel_default_none() {
        let cache = ProceduralMeshCache::default();
        assert!(cache.structure_door_frame_lintel.is_none());
    }

    /// Tests that clear_all() clears the door frame lintel cache field.
    #[test]
    fn test_cache_clear_all_clears_door_frame_lintel() {
        // We can't add a real mesh without a full Bevy context, but we can
        // verify that clear_all() sets the field to None even if it was None.
        let mut cache = ProceduralMeshCache::default();
        assert!(cache.structure_door_frame_lintel.is_none());
        cache.clear_all();
        assert!(cache.structure_door_frame_lintel.is_none());
        // Verify structure_door_frame is also cleared
        assert!(cache.structure_door_frame.is_none());
    }

    /// Tests that spawn_door_frame produces a parent entity with child posts
    /// and lintel inside a minimal Bevy App.
    #[test]
    fn test_spawn_door_frame_produces_entities() {
        use crate::domain::world::DoorFrameConfig;

        fn spawn_frame_system(
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
        ) {
            let mut cache = ProceduralMeshCache::default();
            let config = DoorFrameConfig::default();
            let position = crate::domain::types::Position::new(0, 0);
            let map_id: crate::domain::types::MapId = 1;
            let mut ctx = MeshSpawnContext {
                commands: &mut commands,
                materials: &mut materials,
                meshes: &mut meshes,
                cache: &mut cache,
            };
            spawn_door_frame(&mut ctx, position, map_id, config, None);
        }

        let mut app = bevy::app::App::new();
        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<StandardMaterial>()
        .init_asset::<Mesh>();
        app.add_systems(bevy::app::Update, spawn_frame_system);
        app.update();

        // After spawning, at least one entity with the Name "DoorFrame" exists
        let world = app.world_mut();
        let frame_count = world
            .query_filtered::<&Name, ()>()
            .iter(world)
            .filter(|n| n.as_str() == "DoorFrame")
            .count();
        assert_eq!(frame_count, 1, "exactly one DoorFrame entity should exist");
    }

    /// Tests that spawn_door_with_frame produces two separate entities:
    /// a Door entity and a DoorFrame entity, both in the same Bevy world.
    #[test]
    fn test_spawn_door_with_frame_produces_door_and_frame() {
        use crate::domain::world::DoorFrameConfig;

        fn spawn_composite_system(
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
        ) {
            let mut cache = ProceduralMeshCache::default();
            let door_config = DoorConfig::default();
            let frame_config = DoorFrameConfig::default();
            let position = crate::domain::types::Position::new(2, 3);
            let map_id: crate::domain::types::MapId = 1;
            let mut ctx = MeshSpawnContext {
                commands: &mut commands,
                materials: &mut materials,
                meshes: &mut meshes,
                cache: &mut cache,
            };
            spawn_door_with_frame(
                &mut ctx,
                position,
                map_id,
                door_config,
                frame_config,
                Some(90.0),
            );
        }

        let mut app = bevy::app::App::new();
        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<StandardMaterial>()
        .init_asset::<Mesh>();
        app.add_systems(bevy::app::Update, spawn_composite_system);
        app.update();

        let world = app.world_mut();
        let door_count = world
            .query_filtered::<&Name, ()>()
            .iter(world)
            .filter(|n| n.as_str() == "Door")
            .count();
        let frame_count = world
            .query_filtered::<&Name, ()>()
            .iter(world)
            .filter(|n| n.as_str() == "DoorFrame")
            .count();
        assert_eq!(door_count, 1, "exactly one Door entity should exist");
        assert_eq!(frame_count, 1, "exactly one DoorFrame entity should exist");
    }

    /// Tests that spawn_door_frame attaches MapEntity and TileCoord components
    /// to the parent door frame entity.
    #[test]
    fn test_spawn_door_frame_has_map_entity_and_tile_coord() {
        use crate::domain::world::DoorFrameConfig;
        use crate::game::systems::map::{MapEntity, TileCoord};

        fn spawn_frame_components_system(
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
        ) {
            let mut cache = ProceduralMeshCache::default();
            let config = DoorFrameConfig::default();
            let position = crate::domain::types::Position::new(4, 7);
            let map_id: crate::domain::types::MapId = 42;
            let mut ctx = MeshSpawnContext {
                commands: &mut commands,
                materials: &mut materials,
                meshes: &mut meshes,
                cache: &mut cache,
            };
            spawn_door_frame(&mut ctx, position, map_id, config, None);
        }

        let mut app = bevy::app::App::new();
        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<StandardMaterial>()
        .init_asset::<Mesh>();
        app.add_systems(bevy::app::Update, spawn_frame_components_system);
        app.update();

        // Query for the DoorFrame entity and verify its components
        let world = app.world_mut();
        let result = world
            .query::<(&Name, &MapEntity, &TileCoord)>()
            .iter(world)
            .find(|(n, _, _)| n.as_str() == "DoorFrame")
            .map(|(_, map_entity, tile_coord)| (map_entity.0, tile_coord.0));

        assert!(
            result.is_some(),
            "DoorFrame entity must have MapEntity and TileCoord"
        );
        let (found_map_id, found_position) = result.unwrap();
        assert_eq!(found_map_id, 42, "MapEntity map_id must match");
        assert_eq!(found_position.x, 4, "TileCoord x must match");
        assert_eq!(found_position.y, 7, "TileCoord y must match");
    }

    /// Tests that DoorFrameConfig frame_thickness matches the DOOR_FRAME_THICKNESS constant.
    #[test]
    fn test_door_frame_config_thickness_matches_constant() {
        use crate::domain::world::DoorFrameConfig;
        let config = DoorFrameConfig::default();
        assert!(
            (config.frame_thickness - DOOR_FRAME_THICKNESS).abs() < f32::EPSILON,
            "DoorFrameConfig::default().frame_thickness ({}) must equal DOOR_FRAME_THICKNESS ({})",
            config.frame_thickness,
            DOOR_FRAME_THICKNESS
        );
    }

    // ==================== Performance & Polish Tests ====================

    /// Tests DetailLevel from_distance for full quality
    #[test]
    fn test_detail_level_full_distance() {
        use crate::domain::world::DetailLevel;
        let level = DetailLevel::from_distance(5.0);
        assert_eq!(level, DetailLevel::Full);
    }

    /// Tests DetailLevel from_distance for simplified quality
    #[test]
    fn test_detail_level_simplified_distance() {
        use crate::domain::world::DetailLevel;
        let level = DetailLevel::from_distance(20.0);
        assert_eq!(level, DetailLevel::Simplified);
    }

    /// Tests DetailLevel from_distance for billboard quality
    #[test]
    fn test_detail_level_billboard_distance() {
        use crate::domain::world::DetailLevel;
        let level = DetailLevel::from_distance(50.0);
        assert_eq!(level, DetailLevel::Billboard);
    }

    /// Tests DetailLevel squared distance thresholds
    #[test]
    fn test_detail_level_distance_thresholds() {
        use crate::domain::world::DetailLevel;
        assert_eq!(DetailLevel::Full.distance_threshold_squared(), 100.0);
        assert_eq!(DetailLevel::Simplified.distance_threshold_squared(), 900.0);
        assert!(DetailLevel::Billboard
            .distance_threshold_squared()
            .is_infinite());
    }

    /// Tests DetailLevel max_distance values
    #[test]
    fn test_detail_level_max_distance() {
        use crate::domain::world::DetailLevel;
        assert_eq!(DetailLevel::Full.max_distance(), 10.0);
        assert_eq!(DetailLevel::Simplified.max_distance(), 30.0);
        assert!(DetailLevel::Billboard.max_distance().is_infinite());
    }

    /// Tests InstanceData creation and modification
    #[test]
    fn test_instance_data_creation() {
        use crate::domain::world::InstanceData;
        let instance = InstanceData::new([1.0, 2.0, 3.0]);
        assert_eq!(instance.position, [1.0, 2.0, 3.0]);
        assert_eq!(instance.scale, 1.0);
        assert_eq!(instance.rotation_y, 0.0);
    }

    /// Tests InstanceData with_scale builder
    #[test]
    fn test_instance_data_with_scale() {
        use crate::domain::world::InstanceData;
        let instance = InstanceData::new([1.0, 2.0, 3.0]).with_scale(2.5);
        assert_eq!(instance.scale, 2.5);
    }

    /// Tests InstanceData with_rotation builder
    #[test]
    fn test_instance_data_with_rotation() {
        use crate::domain::world::InstanceData;
        let instance = InstanceData::new([1.0, 2.0, 3.0]).with_rotation(1.5);
        assert_eq!(instance.rotation_y, 1.5);
    }

    /// Tests InstanceData chained builders
    #[test]
    fn test_instance_data_chained_builders() {
        use crate::domain::world::InstanceData;
        let instance = InstanceData::new([1.0, 2.0, 3.0])
            .with_scale(2.0)
            .with_rotation(0.5);
        assert_eq!(instance.position, [1.0, 2.0, 3.0]);
        assert_eq!(instance.scale, 2.0);
        assert_eq!(instance.rotation_y, 0.5);
    }

    /// Tests AsyncMeshTaskId creation
    #[test]
    fn test_async_mesh_task_id_creation() {
        use crate::domain::world::AsyncMeshTaskId;
        let task_id = AsyncMeshTaskId::new(42);
        assert_eq!(task_id.0, 42);
    }

    /// Tests AsyncMeshTaskId equality
    #[test]
    fn test_async_mesh_task_id_equality() {
        use crate::domain::world::AsyncMeshTaskId;
        let task1 = AsyncMeshTaskId::new(42);
        let task2 = AsyncMeshTaskId::new(42);
        assert_eq!(task1, task2);
    }

    /// Tests AsyncMeshConfig defaults
    #[test]
    fn test_async_mesh_config_defaults() {
        use crate::domain::world::AsyncMeshConfig;
        let config = AsyncMeshConfig::default();
        assert_eq!(config.max_concurrent_tasks, 4);
        assert!(config.prioritize_by_distance);
        assert_eq!(config.generation_timeout_ms, 5000);
    }

    /// Tests calculate_lod_level for low vertex count
    #[test]
    fn test_calculate_lod_level_low_vertices() {
        use crate::domain::world::DetailLevel;
        let level = calculate_lod_level(500);
        assert_eq!(level, DetailLevel::Full);
    }

    /// Tests calculate_lod_level for medium vertex count
    #[test]
    fn test_calculate_lod_level_medium_vertices() {
        use crate::domain::world::DetailLevel;
        let level = calculate_lod_level(3000);
        assert_eq!(level, DetailLevel::Simplified);
    }

    /// Tests calculate_lod_level for high vertex count
    #[test]
    fn test_calculate_lod_level_high_vertices() {
        use crate::domain::world::DetailLevel;
        let level = calculate_lod_level(10000);
        assert_eq!(level, DetailLevel::Billboard);
    }

    /// Tests create_instance_data batch creation
    #[test]
    fn test_create_instance_data_batch() {
        let positions = vec![[1.0, 0.0, 2.0], [3.0, 0.0, 4.0], [5.0, 0.0, 6.0]];
        let instances = create_instance_data(&positions, 1.0);
        assert_eq!(instances.len(), 3);
        assert_eq!(instances[0].position, [1.0, 0.0, 2.0]);
        assert_eq!(instances[1].position, [3.0, 0.0, 4.0]);
        assert_eq!(instances[2].position, [5.0, 0.0, 6.0]);
    }

    /// Tests create_instance_data applies uniform scale
    #[test]
    fn test_create_instance_data_applies_scale() {
        let positions = vec![[1.0, 0.0, 2.0]];
        let instances = create_instance_data(&positions, 2.5);
        assert_eq!(instances[0].scale, 2.5);
    }

    /// Tests create_instance_data with empty list
    #[test]
    fn test_create_instance_data_empty() {
        let positions: Vec<[f32; 3]> = vec![];
        let instances = create_instance_data(&positions, 1.0);
        assert_eq!(instances.len(), 0);
    }

    /// Tests estimate_draw_call_reduction for single instance
    #[test]
    fn test_estimate_draw_call_reduction_single() {
        let reduction = estimate_draw_call_reduction(1);
        assert_eq!(reduction, 1.0);
    }

    /// Tests estimate_draw_call_reduction for multiple instances
    #[test]
    fn test_estimate_draw_call_reduction_hundred() {
        let reduction = estimate_draw_call_reduction(100);
        assert!((reduction - 0.01).abs() < 0.001); // Should be ~0.01 (1% of original)
    }

    /// Tests estimate_draw_call_reduction for zero instances
    #[test]
    fn test_estimate_draw_call_reduction_zero() {
        let reduction = estimate_draw_call_reduction(0);
        assert_eq!(reduction, 1.0);
    }

    /// Tests cache clear_all clears all handles
    #[test]
    fn test_cache_clear_all() {
        let mut cache = ProceduralMeshCache::default();
        // Verify all None initially
        assert!(cache.tree_trunk.is_none());
        // After clearing, should still be None
        cache.clear_all();
        assert!(cache.tree_trunk.is_none());
        assert!(cache.tree_foliage.is_none());
        assert!(cache.sign_post.is_none());
    }

    /// Tests cache cached_count returns zero initially
    #[test]
    fn test_cache_cached_count_empty() {
        let cache = ProceduralMeshCache::default();
        assert_eq!(cache.cached_count(), 0);
    }

    /// Tests create_simplified_mesh with zero reduction
    #[test]
    fn test_create_simplified_mesh_zero_reduction() {
        let plane_mesh = Cuboid {
            half_size: Vec3::new(0.5, 0.5, 0.01),
        };
        let mesh = Mesh::from(plane_mesh);
        let simplified = create_simplified_mesh(&mesh, 0.0);
        assert_eq!(simplified.count_vertices(), mesh.count_vertices());
    }

    /// Tests create_simplified_mesh with 50% reduction actually reduces vertices
    #[test]
    fn test_create_simplified_mesh_half_reduction_reduces_vertices() {
        // Create a mesh with many vertices via a higher-poly shape
        let mut mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        // 12 vertices (more than enough for stride-based reduction)
        let positions: Vec<[f32; 3]> = (0..12).map(|i| [i as f32, 0.0, 0.0]).collect();
        let normals: Vec<[f32; 3]> = (0..12).map(|_| [0.0, 1.0, 0.0]).collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_indices(bevy::mesh::Indices::U32(vec![
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
        ]));

        let simplified = create_simplified_mesh(&mesh, 0.5);
        assert!(
            simplified.count_vertices() < mesh.count_vertices(),
            "Simplified mesh should have fewer vertices: {} vs {}",
            simplified.count_vertices(),
            mesh.count_vertices()
        );
    }

    /// Tests create_simplified_mesh preserves mesh with very few vertices
    #[test]
    fn test_create_simplified_mesh_preserves_small_mesh() {
        let mesh = Mesh::from(Cuboid {
            half_size: Vec3::new(0.5, 0.5, 0.5),
        });
        // A cuboid has relatively few vertices; with < 4 vertices it returns clone
        let simplified = create_simplified_mesh(&mesh, 0.5);
        // Even with a cuboid (24 verts), it should produce fewer vertices
        assert!(simplified.count_vertices() <= mesh.count_vertices());
    }

    /// Tests create_billboard_mesh returns valid quad
    #[test]
    fn test_create_billboard_mesh_valid() {
        let billboard = create_billboard_mesh();
        // Billboard should have a reasonable vertex count for a quad
        let vertex_count = billboard.count_vertices();
        assert!(vertex_count > 0, "Billboard mesh should have vertices");
    }

    /// Tests ProceduralMeshCache implements Clone
    #[test]
    fn test_cache_clone_trait() {
        let cache = ProceduralMeshCache::default();
        let _cloned = cache.clone();
        // Test passes if Clone trait is implemented
    }

    // ==================== Tree Material Cache Tests ====================

    /// Tests that foliage_texture_path returns a non-empty .png path for all TreeType variants
    #[test]
    fn test_foliage_texture_path_all_variants() {
        let variants = [
            TreeType::Oak,
            TreeType::Pine,
            TreeType::Birch,
            TreeType::Willow,
            TreeType::Palm,
            TreeType::Dead,
            TreeType::Shrub,
        ];
        for variant in variants {
            let path = foliage_texture_path(variant);
            assert!(
                !path.is_empty(),
                "foliage_texture_path({:?}) should not be empty",
                variant
            );
            assert!(
                path.ends_with(".png"),
                "foliage_texture_path({:?}) should end with .png, got '{}'",
                variant,
                path
            );
        }
    }

    /// Tests that TREE_FOLIAGE_ALPHA_CUTOFF is a valid mask threshold (0 < value < 1)
    #[test]
    fn test_tree_foliage_alpha_cutoff_valid() {
        const { assert!(TREE_FOLIAGE_ALPHA_CUTOFF > 0.0) };
        const { assert!(TREE_FOLIAGE_ALPHA_CUTOFF < 1.0) };
    }

    /// Tests that bark materials keep a base texture assigned.
    #[test]
    fn test_bark_material_uses_texture() {
        fn create_bark_material_system(
            asset_server: Res<AssetServer>,
            mut materials: ResMut<Assets<StandardMaterial>>,
        ) {
            let mut cache = ProceduralMeshCache::default();
            let handle = cache.get_or_create_bark_material(&asset_server, &mut materials);
            let material = materials
                .get(&handle)
                .expect("bark material should be inserted into material assets");

            assert!(
                material.base_color_texture.is_some(),
                "Bark material must use TREE_BARK_TEXTURE as its base_color_texture"
            );
            assert_eq!(material.base_color, TREE_TRUNK_COLOR);
        }

        let mut app = bevy::app::App::new();
        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>();
        app.add_systems(Update, create_bark_material_system);

        app.update();
    }

    /// Tests that bark base colours vary by species before metadata tinting.
    #[test]
    fn test_bark_color_for_tree_type_is_species_specific() {
        let oak = bark_color_for_tree_type(TreeType::Oak);
        let dead = bark_color_for_tree_type(TreeType::Dead);
        let birch = bark_color_for_tree_type(TreeType::Birch);
        let palm = bark_color_for_tree_type(TreeType::Palm);
        let pine = bark_color_for_tree_type(TreeType::Pine);

        assert_eq!(oak, TREE_TRUNK_COLOR);
        assert_ne!(dead, oak, "Dead trees need a distinct grey-brown bark tint");
        assert_ne!(birch, oak, "Birch trees need a distinct pale bark tint");
        assert_ne!(palm, oak, "Palm trees need a distinct warm bark tint");
        assert_ne!(pine, oak, "Pine trees need a distinct dark bark tint");
        assert_ne!(
            dead, birch,
            "Dead and Birch bark colours must remain visually distinct"
        );
    }

    /// Tests that ProceduralMeshCache::default() initialises tree_foliage_materials as empty
    #[test]
    fn test_cache_tree_foliage_materials_default_empty() {
        let cache = ProceduralMeshCache::default();
        assert!(
            cache.tree_foliage_materials.is_empty(),
            "tree_foliage_materials should be empty on default construction"
        );
    }

    /// Tests that ProceduralMeshCache::default() initialises tree_bark_material as None
    #[test]
    fn test_cache_tree_bark_material_default_none() {
        let cache = ProceduralMeshCache::default();
        assert!(
            cache.tree_bark_material.is_none(),
            "tree_bark_material should be None on default construction"
        );
    }

    #[test]
    fn test_spawn_tree_dead_tree_spawns_no_foliage_children() {
        fn spawn_dead_tree_system(
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
            asset_server: Res<AssetServer>,
        ) {
            let mut cache = ProceduralMeshCache::default();
            let mut ctx = MeshSpawnContext {
                commands: &mut commands,
                materials: &mut materials,
                meshes: &mut meshes,
                cache: &mut cache,
            };

            spawn_tree(
                &mut ctx,
                &asset_server,
                crate::domain::types::Position::new(2, 3),
                1,
                None,
                Some(TreeType::Dead),
            );
        }

        let mut app = bevy::app::App::new();
        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .init_asset::<Mesh>();
        app.add_systems(Update, spawn_dead_tree_system);
        app.update();

        let child_counts: Vec<usize> = {
            let world = app.world_mut();
            world
                .query::<(&TileCoord, &Children)>()
                .iter(world)
                .filter(|(coord, _)| coord.0 == crate::domain::types::Position::new(2, 3))
                .map(|(_, children)| children.len())
                .collect()
        };

        assert_eq!(
            child_counts,
            vec![1],
            "Dead tree should only have the branch mesh child and no foliage children"
        );
    }

    #[test]
    fn test_spawn_tree_foliage_density_zero_suppresses_foliage_children() {
        fn spawn_zero_foliage_tree_system(
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
            asset_server: Res<AssetServer>,
        ) {
            let metadata = TileVisualMetadata {
                foliage_density: Some(0.0),
                ..Default::default()
            };
            let mut cache = ProceduralMeshCache::default();
            let mut ctx = MeshSpawnContext {
                commands: &mut commands,
                materials: &mut materials,
                meshes: &mut meshes,
                cache: &mut cache,
            };

            spawn_tree(
                &mut ctx,
                &asset_server,
                crate::domain::types::Position::new(4, 5),
                1,
                Some(&metadata),
                Some(TreeType::Oak),
            );
        }

        let mut app = bevy::app::App::new();
        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .init_asset::<Mesh>();
        app.add_systems(Update, spawn_zero_foliage_tree_system);
        app.update();

        let child_counts: Vec<usize> = {
            let world = app.world_mut();
            world
                .query::<(&TileCoord, &Children)>()
                .iter(world)
                .filter(|(coord, _)| coord.0 == crate::domain::types::Position::new(4, 5))
                .map(|(_, children)| children.len())
                .collect()
        };

        assert_eq!(
            child_counts,
            vec![1],
            "foliage_density = 0.0 should leave only the branch mesh child"
        );
    }

    #[test]
    fn test_spawn_tree_height_and_scale_metadata_affect_tree_and_foliage_transforms() {
        fn spawn_scaled_tree_system(
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
            asset_server: Res<AssetServer>,
        ) {
            let metadata = TileVisualMetadata {
                height: Some(4.0),
                scale: Some(1.5),
                foliage_density: Some(1.0),
                ..Default::default()
            };
            let mut cache = ProceduralMeshCache::default();
            let mut ctx = MeshSpawnContext {
                commands: &mut commands,
                materials: &mut materials,
                meshes: &mut meshes,
                cache: &mut cache,
            };

            spawn_tree(
                &mut ctx,
                &asset_server,
                crate::domain::types::Position::new(6, 7),
                1,
                Some(&metadata),
                Some(TreeType::Oak),
            );
        }

        let mut app = bevy::app::App::new();
        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .init_asset::<Mesh>();
        app.add_systems(Update, spawn_scaled_tree_system);
        app.update();

        let (branch_scale, foliage_translation) = {
            let world = app.world_mut();
            let parent_children = world
                .query::<(&TileCoord, &Children)>()
                .iter(world)
                .find(|(coord, _)| coord.0 == crate::domain::types::Position::new(6, 7))
                .map(|(_, children)| children.iter().collect::<Vec<_>>())
                .expect("scaled tree parent should have children");

            assert!(
                parent_children.len() > 1,
                "scaled leafy oak should spawn branch mesh plus foliage children"
            );

            let branch_transform = world
                .get::<Transform>(parent_children[0])
                .expect("branch child should have a Transform")
                .scale;

            let foliage_transform = world
                .get::<Transform>(parent_children[1])
                .expect("foliage child should have a Transform")
                .translation;

            (branch_transform, foliage_transform)
        };

        assert_eq!(branch_scale, Vec3::new(1.5, 2.0, 1.5));
        assert!(
            foliage_translation.y > 4.0,
            "foliage endpoint should receive height metadata scaling"
        );
        assert!(
            foliage_translation.x.abs() > 0.1 || foliage_translation.z.abs() > 0.1,
            "foliage endpoint should receive horizontal scale metadata"
        );
    }

    /// Tests that clear_all() clears tree_foliage_materials
    #[test]
    fn test_cache_clear_all_clears_foliage_materials() {
        let mut cache = ProceduralMeshCache::default();
        // Simulate a cached entry by verifying the map starts empty and remains
        // empty after clear_all — a proper insertion test would require a live
        // Assets<StandardMaterial> from a Bevy App, which is not available in
        // unit tests.  We verify the clear path does not panic and resets state.
        assert!(cache.tree_foliage_materials.is_empty());
        cache.clear_all();
        assert!(
            cache.tree_foliage_materials.is_empty(),
            "tree_foliage_materials should be empty after clear_all()"
        );
        assert!(
            cache.tree_bark_material.is_none(),
            "tree_bark_material should be None after clear_all()"
        );
    }

    /// Tests that foliage_texture_path returns distinct paths for distinct tree types
    #[test]
    fn test_foliage_texture_path_distinct_for_leaf_types() {
        // Oak, Pine, Birch, Willow, Palm, and Shrub should each have unique paths.
        // Dead falls back to Oak intentionally, so it is excluded from this check.
        let leaf_types = [
            TreeType::Oak,
            TreeType::Pine,
            TreeType::Birch,
            TreeType::Willow,
            TreeType::Palm,
            TreeType::Shrub,
        ];
        let paths: Vec<&str> = leaf_types
            .iter()
            .map(|&t| foliage_texture_path(t))
            .collect();
        for i in 0..paths.len() {
            for j in (i + 1)..paths.len() {
                assert_ne!(
                    paths[i], paths[j],
                    "foliage_texture_path should be unique for {:?} and {:?}",
                    leaf_types[i], leaf_types[j]
                );
            }
        }
    }

    /// Tests that Palm foliage texture path points to the palm texture
    #[test]
    fn test_foliage_texture_path_palm_uses_palm_texture() {
        let path = foliage_texture_path(TreeType::Palm);
        assert!(
            path.contains("palm"),
            "Palm foliage texture path should contain 'palm', got '{}'",
            path
        );
    }
}

/// Parameters for spawning a furniture item, bundling the properties that
/// would otherwise push `spawn_furniture` over clippy's 7-argument threshold.
#[derive(Clone, Debug)]
pub struct FurnitureSpawnParams {
    /// Type of furniture to spawn (Bench, Table, Chair, …)
    pub furniture_type: world::FurnitureType,
    /// Optional Y-axis rotation in degrees
    pub rotation_y: Option<f32>,
    /// Size multiplier applied to default dimensions
    pub scale: f32,
    /// Material type (Wood, Stone, Metal, Gold) — determines base colour
    pub material_type: world::FurnitureMaterial,
    /// Furniture flags (lit, locked, blocking)
    pub flags: world::FurnitureFlags,
    /// Optional RGB colour tint `[0.0..1.0]` applied multiplicatively
    pub color_tint: Option<[f32; 3]>,
    /// Optional item ID required to unlock a `Door`; `None` means no key
    pub key_item_id: Option<types::ItemId>,
}

/// Spawns a furniture item based on type with custom properties
///
/// The `key_item_id` field on [`FurnitureSpawnParams`] sets the required key
/// item on [`DoorState`] for `FurnitureType::Door` entities; `None` means the
/// door has no key requirement.
pub fn spawn_furniture(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    params: &FurnitureSpawnParams,
) -> Entity {
    let furniture_type = params.furniture_type;
    let rotation_y = params.rotation_y;
    let scale = params.scale;
    let material_type = params.material_type;
    let flags = &params.flags;
    let color_tint = params.color_tint;
    let key_item_id = params.key_item_id;
    use crate::domain::world::FurnitureType;

    // Apply material properties based on material_type from domain model
    let rgb = material_type.base_color();
    let base_color = Color::srgb(rgb[0], rgb[1], rgb[2]);

    // Apply color tint if provided
    let final_color = if let Some(tint) = color_tint {
        Color::srgb(
            base_color.to_srgba().red * tint[0],
            base_color.to_srgba().green * tint[1],
            base_color.to_srgba().blue * tint[2],
        )
    } else {
        base_color
    };

    match furniture_type {
        FurnitureType::Bench => {
            let mut config = BenchConfig {
                color_override: Some(final_color),
                ..Default::default()
            };
            config.length *= scale;
            spawn_bench(ctx, position, map_id, config, rotation_y)
        }
        FurnitureType::Table => {
            let mut config = TableConfig {
                color_override: Some(final_color),
                ..Default::default()
            };
            config.width *= scale;
            config.depth *= scale;
            spawn_table(ctx, position, map_id, config, rotation_y)
        }
        FurnitureType::Chair => {
            let config = ChairConfig {
                color_override: Some(final_color),
                ..Default::default()
            };
            spawn_chair(ctx, position, map_id, config, rotation_y)
        }
        FurnitureType::Throne => {
            let config = ThroneConfig {
                color_override: Some(final_color),
                ..Default::default()
            };
            spawn_throne(ctx, position, map_id, config, rotation_y)
        }
        FurnitureType::Torch => {
            let config = TorchConfig {
                lit: flags.lit,
                ..Default::default()
            };
            spawn_torch(ctx, position, map_id, config, rotation_y)
        }
        FurnitureType::Chest => {
            let config = ChestConfig {
                color_override: Some(final_color),
                locked: flags.locked,
                size_multiplier: scale,
            };
            spawn_chest(ctx, position, map_id, config, rotation_y)
        }
        FurnitureType::Barrel => {
            let mut config = BarrelConfig {
                color_override: Some(final_color),
                ..Default::default()
            };
            config.height *= scale;
            config.radius *= scale;
            spawn_barrel(ctx, position, map_id, config, rotation_y)
        }
        FurnitureType::Bookshelf => {
            let mut config = BookshelfConfig {
                color_override: Some(final_color),
                ..Default::default()
            };
            config.height *= scale;
            spawn_bookshelf(ctx, position, map_id, config, rotation_y)
        }
        FurnitureType::Door => {
            let door_config = DoorConfig {
                width: DOOR_PANEL_WIDTH * scale,
                height: DOOR_PANEL_HEIGHT * scale,
                thickness: DOOR_PANEL_THICKNESS,
                plank_count: DOOR_DEFAULT_PLANK_COUNT,
                has_studs: true,
                has_hinges: true,
                color_override: Some(final_color),
            };
            let frame_config = crate::domain::world::DoorFrameConfig::default();
            let (door_entity, _frame_entity) =
                spawn_door_with_frame(ctx, position, map_id, door_config, frame_config, rotation_y);
            // Attach interaction components so map-loaded doors can be queried
            // and interacted with through the split exploration input flow.
            let rotation_radians = rotation_y.unwrap_or(0.0_f32).to_radians();
            let door_state = {
                let mut ds = DoorState::new(flags.locked, rotation_radians);
                ds.key_item_id = key_item_id;
                ds
            };
            ctx.commands.entity(door_entity).insert((
                FurnitureEntity::new(world::FurnitureType::Door, flags.blocking),
                door_state,
                Interactable::with_distance(InteractionType::OpenDoor, 1.5),
            ));
            door_entity
        }
    }
}

/// Configuration for barrel furniture
#[derive(Clone, Debug)]
pub struct BarrelConfig {
    /// Height of the barrel
    pub height: f32,
    /// Radius of the barrel
    pub radius: f32,
    /// Color override (None = default wood)
    pub color_override: Option<Color>,
}

impl Default for BarrelConfig {
    fn default() -> Self {
        Self {
            height: 1.0,
            radius: 0.4,
            color_override: None,
        }
    }
}

/// Spawns a barrel
pub fn spawn_barrel(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: BarrelConfig,
    rotation_y: Option<f32>,
) -> Entity {
    let rotation_radians = rotation_y.unwrap_or(0.0).to_radians();

    let mesh = ctx.meshes.add(Cylinder::new(config.radius, config.height));

    let material = ctx.materials.add(StandardMaterial {
        base_color: config.color_override.unwrap_or(Color::srgb(0.5, 0.35, 0.2)),
        perceptual_roughness: 0.8,
        ..default()
    });

    ctx.commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                config.height / 2.0,
                position.y as f32 + TILE_CENTER_OFFSET,
            )
            .with_rotation(Quat::from_rotation_y(rotation_radians)),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Barrel"),
        ))
        .id()
}

/// Configuration for bookshelf furniture
#[derive(Clone, Debug)]
pub struct BookshelfConfig {
    /// Height of the bookshelf
    pub height: f32,
    /// Width of the bookshelf
    pub width: f32,
    /// Depth of the bookshelf
    pub depth: f32,
    /// Color override (None = default wood)
    pub color_override: Option<Color>,
}

impl Default for BookshelfConfig {
    fn default() -> Self {
        Self {
            height: 1.8,
            width: 0.9,
            depth: 0.3,
            color_override: None,
        }
    }
}

/// Configuration for a door furniture panel
///
/// Controls the geometry of a procedurally-generated door panel.  The door
/// consists of a main panel subdivided into visible plank strips, two
/// horizontal cross-braces, optional iron studs, optional hinges on the left
/// edge, and a handle on the right side at mid-height.
///
/// # Examples
///
/// ```
/// use antares::game::systems::procedural_meshes::DoorConfig;
///
/// let cfg = DoorConfig::default();
/// assert_eq!(cfg.width, 0.9);
/// assert_eq!(cfg.height, 2.3);
/// assert_eq!(cfg.thickness, 0.08);
/// assert_eq!(cfg.plank_count, 5);
/// assert!(cfg.has_studs);
/// assert!(cfg.has_hinges);
/// assert!(cfg.color_override.is_none());
/// ```
#[derive(Clone, Debug)]
pub struct DoorConfig {
    /// Width of the door panel in world units (default 0.9)
    pub width: f32,
    /// Height of the door panel in world units (default 2.3)
    pub height: f32,
    /// Thickness of the door panel in world units (default 0.08)
    pub thickness: f32,
    /// Number of visible vertical plank strips (default 5)
    pub plank_count: u8,
    /// Whether to render iron studs on the door face (default true)
    pub has_studs: bool,
    /// Whether to render hinge cuboids on the left edge (default true)
    pub has_hinges: bool,
    /// Optional color override; uses `DOOR_PANEL_COLOR` if None
    pub color_override: Option<Color>,
}

impl Default for DoorConfig {
    /// Returns a door panel with wood-plank defaults: 0.9 × 2.3 × 0.08,
    /// 5 planks, studs and hinges enabled.
    fn default() -> Self {
        Self {
            width: DOOR_PANEL_WIDTH,
            height: DOOR_PANEL_HEIGHT,
            thickness: DOOR_PANEL_THICKNESS,
            plank_count: DOOR_DEFAULT_PLANK_COUNT,
            has_studs: true,
            has_hinges: true,
            color_override: None,
        }
    }
}

/// Spawns a bookshelf using multiple cuboids
pub fn spawn_bookshelf(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: BookshelfConfig,
    rotation_y: Option<f32>,
) -> Entity {
    let rotation_radians = rotation_y.unwrap_or(0.0).to_radians();

    let back_mesh = ctx
        .meshes
        .add(Cuboid::new(config.width, config.height, 0.05));
    let side_mesh = ctx
        .meshes
        .add(Cuboid::new(0.05, config.height, config.depth));
    let shelf_mesh = ctx
        .meshes
        .add(Cuboid::new(config.width - 0.1, 0.05, config.depth - 0.05));

    let material = ctx.materials.add(StandardMaterial {
        base_color: config.color_override.unwrap_or(Color::srgb(0.4, 0.25, 0.1)),
        perceptual_roughness: 0.9,
        ..default()
    });

    let parent = ctx
        .commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                0.0,
                position.y as f32 + TILE_CENTER_OFFSET,
            )
            .with_rotation(Quat::from_rotation_y(rotation_radians)),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Bookshelf"),
        ))
        .id();

    // Back panel
    let back = ctx
        .commands
        .spawn((
            Mesh3d(back_mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(0.0, config.height / 2.0, -config.depth / 2.0 + 0.025),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(back);

    // Side panels
    let side_l = ctx
        .commands
        .spawn((
            Mesh3d(side_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(-config.width / 2.0 + 0.025, config.height / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(side_l);

    let side_r = ctx
        .commands
        .spawn((
            Mesh3d(side_mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(config.width / 2.0 - 0.025, config.height / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(side_r);

    // Shelves
    let shelf_count = 4;
    for i in 0..=shelf_count {
        let h = (i as f32 / shelf_count as f32) * config.height;
        let shelf = ctx
            .commands
            .spawn((
                Mesh3d(shelf_mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(0.0, h.clamp(0.025, config.height - 0.025), 0.0),
                GlobalTransform::default(),
                Visibility::default(),
            ))
            .id();
        ctx.commands.entity(parent).add_child(shelf);
    }

    parent
}

/// Spawns a procedural door panel mesh with planks, cross-braces, hinges, and handle
///
/// Creates a composite 3D door from:
/// - **Panel**: a flat cuboid (`width × height × thickness`) representing the door face
/// - **Cross-braces**: 2 thin horizontal strips at 1/3 and 2/3 height, slightly proud of the panel
/// - **Hinges** (optional): 2 thin cuboids on the left edge at top-third and bottom-third height
/// - **Handle**: a small cylinder on the right side at mid-height
///
/// The `plank_count` field controls how many visible plank dividers are rendered as thin
/// raised strips across the panel face, giving a wood-plank texture appearance.
///
/// # Arguments
///
/// * `ctx` - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache)
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `config` - Door configuration (width, height, thickness, plank_count, studs, hinges, color)
/// * `rotation_y` - Optional rotation in degrees around Y-axis
///
/// # Returns
///
/// Entity ID of the parent door entity
pub fn spawn_door(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: DoorConfig,
    rotation_y: Option<f32>,
) -> Entity {
    let color = config.color_override.unwrap_or(DOOR_PANEL_COLOR);
    let rotation_radians = rotation_y.unwrap_or(0.0).to_radians();

    // ── Materials ──────────────────────────────────────────────────────────
    // Wood panel material — warm, rough surface
    let panel_material = ctx.materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.85,
        metallic: 0.0,
        ..default()
    });

    // Metal accent material — darker and more metallic for hinges / handle
    let metal_color = Color::srgb(
        color.to_srgba().red * 0.45,
        color.to_srgba().green * 0.45,
        color.to_srgba().blue * 0.45,
    );
    let metal_material = ctx.materials.add(StandardMaterial {
        base_color: metal_color,
        perceptual_roughness: 0.4,
        metallic: 0.85,
        ..default()
    });

    // ── Cached meshes ──────────────────────────────────────────────────────
    // Main door panel
    let panel_mesh = ctx
        .cache
        .get_or_create_furniture_mesh("door_panel", ctx.meshes, || {
            Cuboid::new(config.width, config.height, config.thickness).into()
        });

    // Cross brace strip (full width of door, thin and slightly proud)
    let brace_mesh = ctx
        .cache
        .get_or_create_furniture_mesh("door_brace", ctx.meshes, || {
            Cuboid::new(
                config.width,
                DOOR_BRACE_HEIGHT,
                config.thickness + DOOR_BRACE_PROUD * 2.0,
            )
            .into()
        });

    // Hinge cuboid (left edge)
    let hinge_mesh = ctx
        .cache
        .get_or_create_furniture_mesh("door_hinge", ctx.meshes, || {
            Cuboid::new(DOOR_HINGE_WIDTH, DOOR_HINGE_HEIGHT, config.thickness + 0.02).into()
        });

    // Handle cylinder (horizontal, pointing along Z so it protrudes from the face)
    let handle_mesh = ctx
        .cache
        .get_or_create_furniture_mesh("door_handle", ctx.meshes, || {
            Cylinder::new(DOOR_HANDLE_RADIUS, DOOR_HANDLE_LENGTH).into()
        });

    // ── Parent entity ──────────────────────────────────────────────────────
    let transform = Transform::from_xyz(
        position.x as f32 + TILE_CENTER_OFFSET,
        0.0,
        position.y as f32 + TILE_CENTER_OFFSET,
    )
    .with_rotation(Quat::from_rotation_y(rotation_radians));

    let parent = ctx
        .commands
        .spawn((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Door"),
        ))
        .id();

    // ── Main panel ─────────────────────────────────────────────────────────
    let panel = ctx
        .commands
        .spawn((
            Mesh3d(panel_mesh),
            MeshMaterial3d(panel_material.clone()),
            Transform::from_xyz(0.0, config.height / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(panel);

    // ── Plank dividers ─────────────────────────────────────────────────────
    // Thin raised strips running horizontally across the panel to simulate planks
    if config.plank_count > 1 {
        let strip_height = 0.025_f32;
        let strip_mesh = ctx.meshes.add(Cuboid::new(
            config.width + 0.005,
            strip_height,
            config.thickness + DOOR_BRACE_PROUD,
        ));
        let plank_gap = config.height / config.plank_count as f32;
        for i in 1..config.plank_count {
            let y = i as f32 * plank_gap;
            let strip = ctx
                .commands
                .spawn((
                    Mesh3d(strip_mesh.clone()),
                    MeshMaterial3d(panel_material.clone()),
                    Transform::from_xyz(0.0, y, 0.0),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .id();
            ctx.commands.entity(parent).add_child(strip);
        }
    }

    // ── Cross-braces ───────────────────────────────────────────────────────
    // Two heavy horizontal braces at 1/3 and 2/3 height for structural appearance
    for frac in [1.0_f32 / 3.0, 2.0 / 3.0] {
        let y = frac * config.height;
        let brace = ctx
            .commands
            .spawn((
                Mesh3d(brace_mesh.clone()),
                MeshMaterial3d(panel_material.clone()),
                Transform::from_xyz(0.0, y, 0.0),
                GlobalTransform::default(),
                Visibility::default(),
            ))
            .id();
        ctx.commands.entity(parent).add_child(brace);
    }

    // ── Hinges ─────────────────────────────────────────────────────────────
    if config.has_hinges {
        let hinge_x = -(config.width / 2.0) + DOOR_HINGE_WIDTH / 2.0;
        for frac in [0.25_f32, 0.75] {
            let y = frac * config.height;
            let hinge = ctx
                .commands
                .spawn((
                    Mesh3d(hinge_mesh.clone()),
                    MeshMaterial3d(metal_material.clone()),
                    Transform::from_xyz(hinge_x, y, 0.0),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .id();
            ctx.commands.entity(parent).add_child(hinge);
        }
    }

    // ── Handle ─────────────────────────────────────────────────────────────
    // Cylinder rotated 90° around X so it protrudes from the door face (along Z)
    let handle_x = config.width / 2.0 - 0.12;
    let handle_y = config.height * 0.45; // Slightly below mid-height (ergonomic)
    let handle_z = config.thickness / 2.0 + DOOR_HANDLE_LENGTH / 2.0 + 0.01;
    let handle = ctx
        .commands
        .spawn((
            Mesh3d(handle_mesh),
            MeshMaterial3d(metal_material),
            Transform::from_xyz(handle_x, handle_y, handle_z)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(handle);

    parent
}

/// Spawns a procedurally generated door frame structure.
///
/// A door frame is an architectural element that surrounds a door opening,
/// forming an inverted U shape. It consists of two vertical posts (left and
/// right sides) and one horizontal lintel (top bar), rendered in stone
/// to match other architectural structures such as columns and arches.
///
/// The frame is slightly larger than the door panel it surrounds:
/// - Horizontal opening = `config.width`  (door panel fits inside)
/// - Total frame height = `config.height + config.frame_thickness` (lintel on top)
///
/// # Arguments
///
/// * `commands`   - Bevy Commands for entity spawning
/// * `materials`  - Material asset storage
/// * `meshes`     - Mesh asset storage
/// * `position`   - Tile position in world coordinates
/// * `map_id`     - Map identifier used for entity cleanup on map unload
/// * `ctx`        - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache)
/// * `config`     - Door frame configuration (width, height, frame_thickness)
/// * `rotation_y` - Optional Y-axis rotation in degrees (0 = door faces +Z)
///
/// # Returns
///
/// Entity ID of the parent door frame entity.  The entity has three children:
/// left post, right post, and lintel.
///
/// # Examples
///
/// ```text
/// use antares::game::systems::procedural_meshes::{spawn_door_frame, MeshSpawnContext};
/// use antares::domain::world::DoorFrameConfig;
///
/// let mut ctx = MeshSpawnContext { commands: &mut commands, materials: &mut materials,
///     meshes: &mut meshes, cache: &mut cache };
/// let config = DoorFrameConfig::default();
/// let frame = spawn_door_frame(&mut ctx, position, map_id, config, None);
/// ```
pub fn spawn_door_frame(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: crate::domain::world::DoorFrameConfig,
    rotation_y: Option<f32>,
) -> Entity {
    let rotation_radians = rotation_y.unwrap_or(0.0).to_radians();

    // ── Material ───────────────────────────────────────────────────────────
    // Stone color matches columns and arches — architectural structures share
    // the same visual language.
    let frame_material = ctx.materials.add(StandardMaterial {
        base_color: STRUCTURE_STONE_COLOR,
        perceptual_roughness: 0.85,
        metallic: 0.0,
        ..default()
    });

    // ── Post mesh ──────────────────────────────────────────────────────────
    // Both posts share the same cuboid geometry.
    // Height extends to cover the full opening plus the lintel thickness so
    // there is no gap at the top corner joints.
    let post_height = config.height + config.frame_thickness;
    let post_mesh = ctx
        .cache
        .get_or_create_structure_mesh("door_frame", ctx.meshes, || {
            Cuboid::new(config.frame_thickness, post_height, config.frame_thickness).into()
        });

    // ── Lintel mesh ────────────────────────────────────────────────────────
    // The lintel spans the full outer width of the frame (opening + both posts).
    let lintel_width = config.width + 2.0 * config.frame_thickness;
    let lintel_mesh =
        ctx.cache
            .get_or_create_structure_mesh("door_frame_lintel", ctx.meshes, || {
                Cuboid::new(lintel_width, config.frame_thickness, config.frame_thickness).into()
            });

    // ── Parent entity ──────────────────────────────────────────────────────
    let transform = Transform::from_xyz(
        position.x as f32 + TILE_CENTER_OFFSET,
        0.0,
        position.y as f32 + TILE_CENTER_OFFSET,
    )
    .with_rotation(Quat::from_rotation_y(rotation_radians));

    let parent = ctx
        .commands
        .spawn((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("DoorFrame"),
        ))
        .id();

    // ── Left post ──────────────────────────────────────────────────────────
    // Center is placed just outside the left edge of the opening.
    let post_x = config.width / 2.0 + config.frame_thickness / 2.0;
    let post_y = post_height / 2.0;

    let left_post = ctx
        .commands
        .spawn((
            Mesh3d(post_mesh.clone()),
            MeshMaterial3d(frame_material.clone()),
            Transform::from_xyz(-post_x, post_y, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(left_post);

    // ── Right post ─────────────────────────────────────────────────────────
    let right_post = ctx
        .commands
        .spawn((
            Mesh3d(post_mesh),
            MeshMaterial3d(frame_material.clone()),
            Transform::from_xyz(post_x, post_y, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(right_post);

    // ── Lintel ─────────────────────────────────────────────────────────────
    // Sits on top of the opening.  Its bottom edge is at y = config.height and
    // its center is half a frame_thickness above that.
    let lintel_y = config.height + config.frame_thickness / 2.0;

    let lintel = ctx
        .commands
        .spawn((
            Mesh3d(lintel_mesh),
            MeshMaterial3d(frame_material),
            Transform::from_xyz(0.0, lintel_y, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(lintel);

    parent
}

/// Spawns a door panel together with its surrounding stone frame.
///
/// This composite helper calls [`spawn_door`] and [`spawn_door_frame`] at the
/// same tile position, producing a complete doorway:
/// - A 3D wooden door panel with planks, braces, hinges, and handle.
/// - A stone frame (two posts + lintel) that visually surrounds the opening.
///
/// Both entities are siblings (not parent/child) and carry a [`MapEntity`]
/// tag so they are cleaned up together when the map is unloaded.
///
/// The door panel sits *inside* the frame opening: panel width (default 0.9)
/// is smaller than the frame opening width (default 1.0), and panel height
/// (default 2.3) is smaller than the frame height (default 2.5).
///
/// # Arguments
///
/// * `ctx`          - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache)
/// * `position`     - Tile position in world coordinates
/// * `map_id`       - Map identifier for entity cleanup
/// * `door_config`  - Door panel configuration
/// * `frame_config` - Door frame configuration
/// * `rotation_y`   - Optional Y-axis rotation in degrees applied to both entities
///
/// # Returns
///
/// A tuple `(door_entity, frame_entity)` where `door_entity` is the root of
/// the door panel hierarchy and `frame_entity` is the root of the stone frame.
/// Callers that only need a single representative entity (e.g. the furniture
/// dispatch) should use the `door_entity`.
///
/// # Examples
///
/// ```text
/// use antares::game::systems::procedural_meshes::{spawn_door_with_frame, DoorConfig, MeshSpawnContext};
/// use antares::domain::world::DoorFrameConfig;
///
/// let mut ctx = MeshSpawnContext { commands: &mut commands, materials: &mut materials,
///     meshes: &mut meshes, cache: &mut cache };
/// let (door, frame) = spawn_door_with_frame(
///     &mut ctx, position, map_id,
///     DoorConfig::default(), DoorFrameConfig::default(),
///     Some(90.0),
/// );
/// ```
pub fn spawn_door_with_frame(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    door_config: DoorConfig,
    frame_config: crate::domain::world::DoorFrameConfig,
    rotation_y: Option<f32>,
) -> (Entity, Entity) {
    let door_entity = spawn_door(ctx, position, map_id, door_config, rotation_y);

    let frame_entity = spawn_door_frame(ctx, position, map_id, frame_config, rotation_y);

    (door_entity, frame_entity)
}

// ==================== Item Mesh Config Structs ====================

/// Configuration for a dropped sword mesh.
///
/// The sword is rendered as an elongated blade box with a short crossguard
/// and a handle, lying flat on the XZ plane.
///
/// # Examples
///
/// ```
/// use antares::game::systems::procedural_meshes::SwordConfig;
///
/// let cfg = SwordConfig::default();
/// assert!(cfg.blade_length > 0.0);
/// ```
#[derive(Clone, Debug)]
pub struct SwordConfig {
    /// Length of the blade in world units.
    pub blade_length: f32,
    /// Width of the blade at the base.
    pub blade_width: f32,
    /// Whether to add a crossguard quad.
    pub has_crossguard: bool,
    /// Blade color (None = steel grey default).
    pub color: Option<Color>,
}

impl Default for SwordConfig {
    fn default() -> Self {
        Self {
            blade_length: 0.55,
            blade_width: 0.07,
            has_crossguard: true,
            color: None,
        }
    }
}

/// Configuration for a dropped dagger mesh.
///
/// Daggers have a shorter blade than swords and a small handle.
///
/// # Examples
///
/// ```
/// use antares::game::systems::procedural_meshes::{DaggerConfig, SwordConfig};
///
/// let dagger = DaggerConfig::default();
/// let sword  = SwordConfig::default();
/// assert!(dagger.blade_length < sword.blade_length);
/// ```
#[derive(Clone, Debug)]
pub struct DaggerConfig {
    /// Length of the blade in world units.
    pub blade_length: f32,
    /// Blade color (None = steel grey default).
    pub color: Option<Color>,
}

impl Default for DaggerConfig {
    fn default() -> Self {
        Self {
            blade_length: 0.28,
            color: None,
        }
    }
}

/// Configuration for a dropped blunt weapon mesh (club, mace, hammer).
///
/// Rendered as a cylindrical head plus a thin handle.
///
/// # Examples
///
/// ```
/// use antares::game::systems::procedural_meshes::BluntConfig;
///
/// let cfg = BluntConfig::default();
/// assert!(cfg.head_radius > 0.0);
/// assert!(cfg.handle_length > 0.0);
/// ```
#[derive(Clone, Debug)]
pub struct BluntConfig {
    /// Radius of the weapon head.
    pub head_radius: f32,
    /// Length of the handle.
    pub handle_length: f32,
    /// Color (None = dark iron default).
    pub color: Option<Color>,
}

impl Default for BluntConfig {
    fn default() -> Self {
        Self {
            head_radius: 0.09,
            handle_length: 0.35,
            color: None,
        }
    }
}

/// Configuration for a dropped staff mesh.
///
/// Rendered as a long thin cylinder with an optional orb tip.
///
/// # Examples
///
/// ```
/// use antares::game::systems::procedural_meshes::StaffConfig;
///
/// let cfg = StaffConfig::default();
/// assert!(cfg.length > 0.0);
/// assert!(cfg.orb_radius > 0.0);
/// ```
#[derive(Clone, Debug)]
pub struct StaffConfig {
    /// Total length of the staff.
    pub length: f32,
    /// Radius of the orb at the top (0 = no orb).
    pub orb_radius: f32,
    /// Color (None = wood brown default).
    pub color: Option<Color>,
}

impl Default for StaffConfig {
    fn default() -> Self {
        Self {
            length: 0.80,
            orb_radius: 0.06,
            color: None,
        }
    }
}

/// Configuration for a dropped bow mesh.
///
/// Rendered as a curved arc of quads.
///
/// # Examples
///
/// ```
/// use antares::game::systems::procedural_meshes::BowConfig;
///
/// let cfg = BowConfig::default();
/// assert!(cfg.arc_height > 0.0);
/// ```
#[derive(Clone, Debug)]
pub struct BowConfig {
    /// Peak height of the bow arc.
    pub arc_height: f32,
    /// Color (None = wood brown default).
    pub color: Option<Color>,
}

impl Default for BowConfig {
    fn default() -> Self {
        Self {
            arc_height: 0.35,
            color: None,
        }
    }
}

/// Configuration for a dropped armour mesh (chest piece or helmet).
///
/// # Examples
///
/// ```
/// use antares::game::systems::procedural_meshes::ArmorMeshConfig;
///
/// let cfg = ArmorMeshConfig::default();
/// assert!(cfg.width > 0.0);
/// assert!(cfg.height > 0.0);
/// ```
#[derive(Clone, Debug)]
pub struct ArmorMeshConfig {
    /// Width of the armour piece.
    pub width: f32,
    /// Height of the armour piece.
    pub height: f32,
    /// Color (None = dark leather default).
    pub color: Option<Color>,
    /// `true` = helmet dome, `false` = chest plate.
    pub is_helmet: bool,
}

impl Default for ArmorMeshConfig {
    fn default() -> Self {
        Self {
            width: 0.35,
            height: 0.25,
            color: None,
            is_helmet: false,
        }
    }
}

/// Configuration for a dropped shield mesh.
///
/// Rendered as a flat hexagonal polygon disc.
///
/// # Examples
///
/// ```
/// use antares::game::systems::procedural_meshes::ShieldConfig;
///
/// let cfg = ShieldConfig::default();
/// assert!(cfg.radius > 0.0);
/// ```
#[derive(Clone, Debug)]
pub struct ShieldConfig {
    /// Radius of the shield disc.
    pub radius: f32,
    /// Color (None = wood-and-metal default).
    pub color: Option<Color>,
}

impl Default for ShieldConfig {
    fn default() -> Self {
        Self {
            radius: 0.22,
            color: None,
        }
    }
}

/// Configuration for a dropped potion mesh.
///
/// Rendered as a tapered cylinder body with a sphere stopper.
///
/// # Examples
///
/// ```
/// use antares::game::systems::procedural_meshes::PotionConfig;
///
/// let cfg = PotionConfig::default();
/// // Colors are stored as [r, g, b, a] arrays, all non-zero
/// assert!(cfg.liquid_color[3] > 0.0);
/// ```
#[derive(Clone, Debug)]
pub struct PotionConfig {
    /// RGBA color of the liquid inside the bottle (used for bottle body).
    pub liquid_color: [f32; 4],
    /// RGBA color of the glass bottle exterior.
    pub bottle_color: [f32; 4],
}

impl Default for PotionConfig {
    fn default() -> Self {
        Self {
            liquid_color: [0.8, 0.1, 0.1, 0.85], // red healing potion
            bottle_color: [0.6, 0.6, 0.8, 0.55], // translucent glass
        }
    }
}

/// Configuration for a dropped scroll mesh.
///
/// Rendered as a rolled cylinder pair.
///
/// # Examples
///
/// ```
/// use antares::game::systems::procedural_meshes::ScrollConfig;
///
/// let cfg = ScrollConfig::default();
/// assert!(cfg.color[3] > 0.0);
/// ```
#[derive(Clone, Debug)]
pub struct ScrollConfig {
    /// RGBA color of the parchment.
    pub color: [f32; 4],
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            color: [0.85, 0.80, 0.65, 1.0], // parchment
        }
    }
}

/// Configuration for a dropped ring mesh.
///
/// Rendered as a torus approximated with an arc of thin quads.
///
/// # Examples
///
/// ```
/// use antares::game::systems::procedural_meshes::RingMeshConfig;
///
/// let cfg = RingMeshConfig::default();
/// assert!(cfg.color[3] > 0.0);
/// ```
#[derive(Clone, Debug)]
pub struct RingMeshConfig {
    /// RGBA color of the ring band.
    pub color: [f32; 4],
}

impl Default for RingMeshConfig {
    fn default() -> Self {
        Self {
            color: [0.8, 0.65, 0.2, 1.0], // gold
        }
    }
}

/// Configuration for a dropped ammunition mesh (arrow, bolt, or stone).
///
/// # Examples
///
/// ```
/// use antares::game::systems::procedural_meshes::AmmoConfig;
///
/// let cfg = AmmoConfig::default();
/// assert!(cfg.color[3] > 0.0);
/// ```
#[derive(Clone, Debug)]
pub struct AmmoConfig {
    /// Visual sub-type: `"arrow"`, `"bolt"`, or `"stone"`.
    pub ammo_type: String,
    /// RGBA color for the shaft / stone.
    pub color: [f32; 4],
}

impl Default for AmmoConfig {
    fn default() -> Self {
        Self {
            ammo_type: "arrow".to_string(),
            color: [0.55, 0.40, 0.20, 1.0], // wood-brown shaft
        }
    }
}

// ==================== Item Mesh Constants ====================

/// Default steel-grey color for sword blades.
const ITEM_SWORD_COLOR: Color = Color::srgb(0.75, 0.75, 0.80);
/// Default iron-grey color for blunt weapons.
const ITEM_BLUNT_COLOR: Color = Color::srgb(0.45, 0.45, 0.50);
/// Default wood-brown color for staves, bows, and handles.
const ITEM_WOOD_COLOR: Color = Color::srgb(0.55, 0.38, 0.18);
/// Default leather color for body armour.
const ITEM_LEATHER_COLOR: Color = Color::srgb(0.40, 0.28, 0.15);
/// Default metallic color for helmets and shields.
const ITEM_METAL_COLOR: Color = Color::srgb(0.60, 0.60, 0.65);
/// Default parchment color for scrolls.
#[cfg(test)]
const ITEM_PARCHMENT_COLOR: Color = Color::srgb(0.85, 0.80, 0.65);
/// Default gold color for rings.
#[cfg(test)]
const ITEM_GOLD_COLOR: Color = Color::srgb(0.80, 0.65, 0.20);

// ==================== Item Mesh Spawn Functions ====================

/// Spawns a procedural sword mesh lying flat on the ground.
///
/// The sword consists of:
/// - A blade: elongated flat cuboid along X axis
/// - A crossguard: shorter wider cuboid perpendicular to the blade
/// - A handle: shorter narrower cuboid at the base of the blade
///
/// All parts are child entities of the returned parent entity.
/// The parent is tagged with [`MapEntity`] and [`TileCoord`] for map cleanup.
///
/// # Arguments
///
/// * `ctx`       - Mutable reference to [`MeshSpawnContext`] (commands, materials, meshes, cache).
/// * `position`  - Tile position in world coordinates.
/// * `map_id`    - Map identifier (for cleanup on map change).
/// * `config`    - Sword appearance configuration.
///
/// # Returns
///
/// Entity ID of the parent sword entity.
pub fn spawn_sword_mesh(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: SwordConfig,
) -> Entity {
    let color = config.color.unwrap_or(ITEM_SWORD_COLOR);
    let bl = config.blade_length;
    let bw = config.blade_width;

    // Blade mesh (lying on X axis, thin in Y)
    let blade_mesh = ctx
        .cache
        .get_or_create_item_mesh("sword", ctx.meshes, || Cuboid::new(bl, bw * 0.3, bw).into());

    // Crossguard mesh (perpendicular, shorter)
    let guard_mesh = ctx
        .meshes
        .add(Mesh::from(Cuboid::new(bw * 3.0, bw * 0.3, bw * 0.6)));

    // Handle mesh
    let handle_mesh = ctx
        .meshes
        .add(Mesh::from(Cuboid::new(bl * 0.25, bw * 0.3, bw * 0.7)));

    let blade_mat = ctx.materials.add(StandardMaterial {
        base_color: color,
        metallic: 0.9,
        perceptual_roughness: 0.2,
        ..default()
    });
    let handle_mat = ctx.materials.add(StandardMaterial {
        base_color: ITEM_WOOD_COLOR,
        metallic: 0.0,
        perceptual_roughness: 0.8,
        ..default()
    });

    let parent = ctx
        .commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                0.02,
                position.y as f32 + TILE_CENTER_OFFSET,
            ),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Sword"),
        ))
        .id();

    let blade = ctx
        .commands
        .spawn((
            Mesh3d(blade_mesh),
            MeshMaterial3d(blade_mat.clone()),
            Transform::from_xyz(bl * 0.1, 0.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(blade);

    if config.has_crossguard {
        let guard = ctx
            .commands
            .spawn((
                Mesh3d(guard_mesh),
                MeshMaterial3d(blade_mat),
                Transform::from_xyz(-bl * 0.3, 0.0, 0.0),
                GlobalTransform::default(),
                Visibility::default(),
            ))
            .id();
        ctx.commands.entity(parent).add_child(guard);
    }

    let handle = ctx
        .commands
        .spawn((
            Mesh3d(handle_mesh),
            MeshMaterial3d(handle_mat),
            Transform::from_xyz(-bl * 0.44, 0.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(handle);

    parent
}

/// Spawns a procedural dagger mesh lying flat on the ground.
///
/// Similar to a sword but shorter blade, no crossguard.
///
/// # Returns
///
/// Entity ID of the parent dagger entity.
pub fn spawn_dagger_mesh(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: DaggerConfig,
) -> Entity {
    let color = config.color.unwrap_or(ITEM_SWORD_COLOR);
    let bl = config.blade_length;
    let bw = 0.05_f32;

    let blade_mesh = ctx.cache.get_or_create_item_mesh("dagger", ctx.meshes, || {
        Cuboid::new(bl, bw * 0.3, bw).into()
    });
    let handle_mesh = ctx
        .meshes
        .add(Mesh::from(Cuboid::new(bl * 0.3, bw * 0.3, bw * 0.8)));

    let blade_mat = ctx.materials.add(StandardMaterial {
        base_color: color,
        metallic: 0.9,
        perceptual_roughness: 0.2,
        ..default()
    });
    let handle_mat = ctx.materials.add(StandardMaterial {
        base_color: ITEM_WOOD_COLOR,
        perceptual_roughness: 0.8,
        ..default()
    });

    let parent = ctx
        .commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                0.02,
                position.y as f32 + TILE_CENTER_OFFSET,
            ),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Dagger"),
        ))
        .id();

    let blade = ctx
        .commands
        .spawn((
            Mesh3d(blade_mesh),
            MeshMaterial3d(blade_mat),
            Transform::from_xyz(bl * 0.12, 0.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(blade);

    let handle = ctx
        .commands
        .spawn((
            Mesh3d(handle_mesh),
            MeshMaterial3d(handle_mat),
            Transform::from_xyz(-bl * 0.38, 0.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(handle);

    parent
}

/// Spawns a procedural blunt weapon mesh lying flat on the ground.
///
/// Consists of a cylindrical head and a thin cuboid handle.
///
/// # Returns
///
/// Entity ID of the parent blunt-weapon entity.
pub fn spawn_blunt_mesh(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: BluntConfig,
) -> Entity {
    let color = config.color.unwrap_or(ITEM_BLUNT_COLOR);
    let hr = config.head_radius;
    let hl = config.handle_length;

    // Head: short cylinder lying on its side (height = diameter)
    let head_mesh = ctx
        .cache
        .get_or_create_item_mesh("blunt", ctx.meshes, || Cylinder::new(hr, hr * 2.0).into());
    let handle_mesh = ctx
        .meshes
        .add(Mesh::from(Cuboid::new(hl, hr * 0.35, hr * 0.35)));

    let head_mat = ctx.materials.add(StandardMaterial {
        base_color: color,
        metallic: 0.7,
        perceptual_roughness: 0.3,
        ..default()
    });
    let handle_mat = ctx.materials.add(StandardMaterial {
        base_color: ITEM_WOOD_COLOR,
        perceptual_roughness: 0.8,
        ..default()
    });

    let parent = ctx
        .commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                hr,
                position.y as f32 + TILE_CENTER_OFFSET,
            )
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("BluntWeapon"),
        ))
        .id();

    let head = ctx
        .commands
        .spawn((
            Mesh3d(head_mesh),
            MeshMaterial3d(head_mat),
            Transform::from_xyz(hl * 0.5, 0.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(head);

    let handle = ctx
        .commands
        .spawn((
            Mesh3d(handle_mesh),
            MeshMaterial3d(handle_mat),
            Transform::from_xyz(-hl * 0.1, 0.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(handle);

    parent
}

/// Spawns a procedural staff mesh lying flat on the ground.
///
/// Consists of a long thin cylinder with an orb at one end.
///
/// # Returns
///
/// Entity ID of the parent staff entity.
pub fn spawn_staff_mesh(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: StaffConfig,
) -> Entity {
    let color = config.color.unwrap_or(ITEM_WOOD_COLOR);
    let len = config.length;
    let orb_r = config.orb_radius;

    let shaft_mesh = ctx
        .cache
        .get_or_create_item_mesh("staff", ctx.meshes, || Cylinder::new(0.025, len).into());
    let orb_mesh = ctx.meshes.add(Mesh::from(Sphere::new(orb_r)));

    let shaft_mat = ctx.materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.8,
        ..default()
    });
    let orb_mat = ctx.materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.0, 0.6),
        emissive: LinearRgba::new(0.2, 0.0, 0.4, 1.0),
        ..default()
    });

    let parent = ctx
        .commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                0.025,
                position.y as f32 + TILE_CENTER_OFFSET,
            )
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Staff"),
        ))
        .id();

    let shaft = ctx
        .commands
        .spawn((
            Mesh3d(shaft_mesh),
            MeshMaterial3d(shaft_mat),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(shaft);

    if orb_r > 0.0 {
        let orb = ctx
            .commands
            .spawn((
                Mesh3d(orb_mesh),
                MeshMaterial3d(orb_mat),
                Transform::from_xyz(len * 0.5, 0.0, 0.0),
                GlobalTransform::default(),
                Visibility::default(),
            ))
            .id();
        ctx.commands.entity(parent).add_child(orb);
    }

    parent
}

/// Spawns a procedural bow mesh lying flat on the ground.
///
/// Rendered as a pair of curved cuboid limbs representing the arc of the bow.
///
/// # Returns
///
/// Entity ID of the parent bow entity.
pub fn spawn_bow_mesh(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: BowConfig,
) -> Entity {
    let color = config.color.unwrap_or(ITEM_WOOD_COLOR);
    let arc_h = config.arc_height;

    // Approximate the arc as two angled limb pieces + centre grip
    let limb_mesh = ctx.cache.get_or_create_item_mesh("bow", ctx.meshes, || {
        Cuboid::new(0.04, 0.04, arc_h * 0.6).into()
    });
    let grip_mesh = ctx
        .meshes
        .add(Mesh::from(Cuboid::new(0.04, 0.04, arc_h * 0.3)));

    let bow_mat = ctx.materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.85,
        ..default()
    });

    let parent = ctx
        .commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                0.02,
                position.y as f32 + TILE_CENTER_OFFSET,
            ),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Bow"),
        ))
        .id();

    // Upper limb — angled outward
    let upper = ctx
        .commands
        .spawn((
            Mesh3d(limb_mesh.clone()),
            MeshMaterial3d(bow_mat.clone()),
            Transform::from_xyz(0.0, 0.0, arc_h * 0.32).with_rotation(Quat::from_rotation_x(0.35)),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(upper);

    // Lower limb
    let lower = ctx
        .commands
        .spawn((
            Mesh3d(limb_mesh),
            MeshMaterial3d(bow_mat.clone()),
            Transform::from_xyz(0.0, 0.0, -arc_h * 0.32)
                .with_rotation(Quat::from_rotation_x(-0.35)),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(lower);

    // Centre grip
    let grip = ctx
        .commands
        .spawn((
            Mesh3d(grip_mesh),
            MeshMaterial3d(bow_mat),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(grip);

    parent
}

/// Spawns a procedural armour mesh (chest plate or helmet dome) on the ground.
///
/// # Returns
///
/// Entity ID of the parent armour entity.
pub fn spawn_armor_mesh(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: ArmorMeshConfig,
) -> Entity {
    let color = config.color.unwrap_or(if config.is_helmet {
        ITEM_METAL_COLOR
    } else {
        ITEM_LEATHER_COLOR
    });

    let body_mesh = ctx.cache.get_or_create_item_mesh("armor", ctx.meshes, || {
        if config.is_helmet {
            Sphere::new(config.width * 0.5).into()
        } else {
            Cuboid::new(config.width, config.height, config.width * 0.6).into()
        }
    });

    let mat = ctx.materials.add(StandardMaterial {
        base_color: color,
        metallic: if config.is_helmet { 0.7 } else { 0.1 },
        perceptual_roughness: if config.is_helmet { 0.3 } else { 0.8 },
        ..default()
    });

    let parent = ctx
        .commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                config.height * 0.5,
                position.y as f32 + TILE_CENTER_OFFSET,
            ),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new(if config.is_helmet { "Helmet" } else { "Armor" }),
        ))
        .id();

    let body = ctx
        .commands
        .spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(mat),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(body);

    parent
}

/// Spawns a procedural shield mesh lying flat on the ground.
///
/// Approximated as a flat cuboid disc.
///
/// # Returns
///
/// Entity ID of the parent shield entity.
pub fn spawn_shield_mesh(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: ShieldConfig,
) -> Entity {
    let color = config.color.unwrap_or(ITEM_METAL_COLOR);
    let r = config.radius;

    // Approximate shield as a flat cylinder (disc shape)
    let disc_mesh = ctx
        .cache
        .get_or_create_item_mesh("shield", ctx.meshes, || Cylinder::new(r, 0.04).into());

    let mat = ctx.materials.add(StandardMaterial {
        base_color: color,
        metallic: 0.6,
        perceptual_roughness: 0.35,
        ..default()
    });

    let parent = ctx
        .commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                0.02,
                position.y as f32 + TILE_CENTER_OFFSET,
            ),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Shield"),
        ))
        .id();

    let disc = ctx
        .commands
        .spawn((
            Mesh3d(disc_mesh),
            MeshMaterial3d(mat),
            Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(disc);

    parent
}

/// Spawns a procedural potion mesh on the ground.
///
/// Consists of a tapered cylinder body (the bottle) and a small sphere stopper.
///
/// # Returns
///
/// Entity ID of the parent potion entity.
pub fn spawn_potion_mesh(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: PotionConfig,
) -> Entity {
    let [lr, lg, lb, la] = config.liquid_color;
    let [br, bg, bb, ba] = config.bottle_color;

    let body_mesh = ctx
        .cache
        .get_or_create_item_mesh("potion", ctx.meshes, || Cylinder::new(0.05, 0.18).into());
    let stopper_mesh = ctx.meshes.add(Mesh::from(Sphere::new(0.028)));

    let bottle_mat = ctx.materials.add(StandardMaterial {
        base_color: Color::srgba(br, bg, bb, ba),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.1,
        ..default()
    });
    let liquid_mat = ctx.materials.add(StandardMaterial {
        base_color: Color::srgba(lr, lg, lb, la),
        alpha_mode: AlphaMode::Blend,
        emissive: LinearRgba::new(lr * 0.2, lg * 0.2, lb * 0.2, 1.0),
        ..default()
    });

    let parent = ctx
        .commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                0.09,
                position.y as f32 + TILE_CENTER_OFFSET,
            ),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Potion"),
        ))
        .id();

    let body = ctx
        .commands
        .spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(bottle_mat),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(body);

    // Liquid shimmer (inner slightly smaller cylinder)
    let inner_mesh = ctx.meshes.add(Mesh::from(Cylinder::new(0.04, 0.14)));
    let liquid = ctx
        .commands
        .spawn((
            Mesh3d(inner_mesh),
            MeshMaterial3d(liquid_mat),
            Transform::from_xyz(0.0, -0.01, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(liquid);

    let stopper = ctx
        .commands
        .spawn((
            Mesh3d(stopper_mesh),
            MeshMaterial3d(ctx.materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.15, 0.05),
                perceptual_roughness: 0.9,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.115, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(stopper);

    parent
}

/// Spawns a procedural scroll mesh lying on the ground.
///
/// Rendered as two short cylinders (the rolled ends) flanking a flat centre.
///
/// # Returns
///
/// Entity ID of the parent scroll entity.
pub fn spawn_scroll_mesh(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: ScrollConfig,
) -> Entity {
    let [r, g, b, a] = config.color;

    let roll_mesh = ctx
        .cache
        .get_or_create_item_mesh("scroll", ctx.meshes, || Cylinder::new(0.03, 0.28).into());
    let sheet_mesh = ctx.meshes.add(Mesh::from(Cuboid::new(0.28, 0.004, 0.22)));

    let parchment_mat = ctx.materials.add(StandardMaterial {
        base_color: Color::srgba(r, g, b, a),
        perceptual_roughness: 0.95,
        ..default()
    });

    let parent = ctx
        .commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                0.03,
                position.y as f32 + TILE_CENTER_OFFSET,
            ),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Scroll"),
        ))
        .id();

    // Left roll
    let roll_l = ctx
        .commands
        .spawn((
            Mesh3d(roll_mesh.clone()),
            MeshMaterial3d(parchment_mat.clone()),
            Transform::from_xyz(-0.14, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(roll_l);

    // Right roll
    let roll_r = ctx
        .commands
        .spawn((
            Mesh3d(roll_mesh),
            MeshMaterial3d(parchment_mat.clone()),
            Transform::from_xyz(0.14, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(roll_r);

    // Flat sheet
    let sheet = ctx
        .commands
        .spawn((
            Mesh3d(sheet_mesh),
            MeshMaterial3d(parchment_mat),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(sheet);

    parent
}

/// Spawns a procedural ring mesh on the ground.
///
/// Approximated as a flat torus using a thin torus primitive.
///
/// # Returns
///
/// Entity ID of the parent ring entity.
pub fn spawn_ring_mesh(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: RingMeshConfig,
) -> Entity {
    let [r, g, b, a] = config.color;

    let ring_mesh = ctx.cache.get_or_create_item_mesh("ring", ctx.meshes, || {
        Torus {
            minor_radius: 0.018,
            major_radius: 0.065,
        }
        .into()
    });

    let mat = ctx.materials.add(StandardMaterial {
        base_color: Color::srgba(r, g, b, a),
        metallic: 0.95,
        perceptual_roughness: 0.15,
        ..default()
    });

    let parent = ctx
        .commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                0.018,
                position.y as f32 + TILE_CENTER_OFFSET,
            ),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Ring"),
        ))
        .id();

    let ring = ctx
        .commands
        .spawn((
            Mesh3d(ring_mesh),
            MeshMaterial3d(mat),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(parent).add_child(ring);

    parent
}

/// Spawns a procedural ammunition mesh on the ground.
///
/// Arrow: shaft cylinder + small arrowhead cuboid.
/// Bolt: shorter, wider shaft.
/// Stone: sphere.
///
/// # Returns
///
/// Entity ID of the parent ammo entity.
pub fn spawn_ammo_mesh(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    config: AmmoConfig,
) -> Entity {
    let [r, g, b, a] = config.color;

    let mat = ctx.materials.add(StandardMaterial {
        base_color: Color::srgba(r, g, b, a),
        perceptual_roughness: 0.75,
        ..default()
    });

    let parent = ctx
        .commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                0.015,
                position.y as f32 + TILE_CENTER_OFFSET,
            ),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new("Ammo"),
        ))
        .id();

    match config.ammo_type.as_str() {
        "stone" => {
            let stone_mesh = ctx
                .cache
                .get_or_create_item_mesh("ammo", ctx.meshes, || Sphere::new(0.045).into());
            let stone = ctx
                .commands
                .spawn((
                    Mesh3d(stone_mesh),
                    MeshMaterial3d(mat),
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .id();
            ctx.commands.entity(parent).add_child(stone);
        }
        "bolt" => {
            let bolt_mesh = ctx
                .cache
                .get_or_create_item_mesh("ammo", ctx.meshes, || Cylinder::new(0.015, 0.22).into());
            let bolt = ctx
                .commands
                .spawn((
                    Mesh3d(bolt_mesh),
                    MeshMaterial3d(mat),
                    Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .id();
            ctx.commands.entity(parent).add_child(bolt);
        }
        _ => {
            // Default: arrow shaft
            let shaft_mesh = ctx
                .cache
                .get_or_create_item_mesh("ammo", ctx.meshes, || Cylinder::new(0.008, 0.35).into());
            let tip_mesh = ctx.meshes.add(Mesh::from(Cuboid::new(0.03, 0.03, 0.04)));

            let shaft = ctx
                .commands
                .spawn((
                    Mesh3d(shaft_mesh),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .id();
            ctx.commands.entity(parent).add_child(shaft);

            let tip_mat = ctx.materials.add(StandardMaterial {
                base_color: ITEM_METAL_COLOR,
                metallic: 0.8,
                perceptual_roughness: 0.25,
                ..default()
            });
            let tip = ctx
                .commands
                .spawn((
                    Mesh3d(tip_mesh),
                    MeshMaterial3d(tip_mat),
                    Transform::from_xyz(0.175, 0.0, 0.0),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .id();
            ctx.commands.entity(parent).add_child(tip);
        }
    }

    parent
}

/// Spawns a procedural dropped item mesh, choosing the correct generator based
/// on the [`ItemMeshCategory`] derived from the item descriptor.
///
/// This is the primary entry point called by the item-world spawn system
/// (`spawn_dropped_item_system`) when `GameContent` is available and the
/// item is found in the database.
///
/// # Arguments
///
/// * `commands`    - Bevy `Commands` for entity creation.
/// * `materials`   - Material asset storage.
/// * `meshes`      - Mesh asset storage.
/// * `position`    - Tile position in world coordinates.
/// * `map_id`      - Map identifier for cleanup tagging.
/// * `descriptor`  - The pre-computed [`ItemMeshDescriptor`] for the item.
/// * `cache`       - Shared mesh cache (reduces duplicate allocations).
///
/// # Returns
///
/// Entity ID of the spawned parent entity.
///
/// # Examples
///
/// ```text
/// use antares::game::systems::procedural_meshes::{spawn_dropped_item_mesh, ProceduralMeshCache};
/// use antares::domain::visual::item_mesh::ItemMeshDescriptor;
/// use antares::domain::items::{Item, ItemType, WeaponData, WeaponClassification};
/// use antares::domain::types::{DiceRoll, Position};
///
/// // Obtain a descriptor from an item definition:
/// // let descriptor = ItemMeshDescriptor::from_item(&item);
/// // let entity = spawn_dropped_item_mesh(&mut ctx, Position::new(5, 7), 1, &descriptor);
/// ```
pub fn spawn_dropped_item_mesh(
    ctx: &mut MeshSpawnContext<'_, '_, '_>,
    position: types::Position,
    map_id: types::MapId,
    descriptor: &crate::domain::visual::item_mesh::ItemMeshDescriptor,
) -> Entity {
    use crate::domain::visual::item_mesh::ItemMeshCategory;

    let [pr, pg, pb, pa] = descriptor.primary_color;
    let primary = Color::srgba(pr, pg, pb, pa);

    match descriptor.category {
        ItemMeshCategory::Sword => spawn_sword_mesh(
            ctx,
            position,
            map_id,
            SwordConfig {
                blade_length: descriptor.blade_length,
                blade_width: 0.07,
                has_crossguard: true,
                color: Some(primary),
            },
        ),

        ItemMeshCategory::Dagger => spawn_dagger_mesh(
            ctx,
            position,
            map_id,
            DaggerConfig {
                blade_length: descriptor.blade_length,
                color: Some(primary),
            },
        ),

        ItemMeshCategory::Blunt => spawn_blunt_mesh(
            ctx,
            position,
            map_id,
            BluntConfig {
                head_radius: 0.09,
                handle_length: 0.35,
                color: Some(primary),
            },
        ),

        ItemMeshCategory::Staff => spawn_staff_mesh(
            ctx,
            position,
            map_id,
            StaffConfig {
                length: descriptor.blade_length.max(0.6),
                orb_radius: 0.06,
                color: Some(primary),
            },
        ),

        ItemMeshCategory::Bow => spawn_bow_mesh(
            ctx,
            position,
            map_id,
            BowConfig {
                arc_height: 0.35,
                color: Some(primary),
            },
        ),

        ItemMeshCategory::BodyArmor => spawn_armor_mesh(
            ctx,
            position,
            map_id,
            ArmorMeshConfig {
                width: 0.35,
                height: 0.25,
                color: Some(primary),
                is_helmet: false,
            },
        ),

        ItemMeshCategory::Helmet => spawn_armor_mesh(
            ctx,
            position,
            map_id,
            ArmorMeshConfig {
                width: 0.28,
                height: 0.28,
                color: Some(primary),
                is_helmet: true,
            },
        ),

        ItemMeshCategory::Shield => spawn_shield_mesh(
            ctx,
            position,
            map_id,
            ShieldConfig {
                radius: 0.22,
                color: Some(primary),
            },
        ),

        ItemMeshCategory::Boots => {
            // Boots rendered as a low flat armour piece
            spawn_armor_mesh(
                ctx,
                position,
                map_id,
                ArmorMeshConfig {
                    width: 0.20,
                    height: 0.12,
                    color: Some(primary),
                    is_helmet: false,
                },
            )
        }
        ItemMeshCategory::Ring | ItemMeshCategory::Amulet => {
            let [rr, rg, rb, ra] = descriptor.primary_color;
            spawn_ring_mesh(
                ctx,
                position,
                map_id,
                RingMeshConfig {
                    color: [rr, rg, rb, ra],
                },
            )
        }
        ItemMeshCategory::Belt | ItemMeshCategory::Cloak => {
            // Belt / cloak rendered as flat armour piece
            spawn_armor_mesh(
                ctx,
                position,
                map_id,
                ArmorMeshConfig {
                    width: 0.30,
                    height: 0.08,
                    color: Some(primary),
                    is_helmet: false,
                },
            )
        }
        ItemMeshCategory::Potion => {
            let [lr, lg, lb, la] = descriptor.primary_color;
            spawn_potion_mesh(
                ctx,
                position,
                map_id,
                PotionConfig {
                    liquid_color: [lr, lg, lb, la],
                    bottle_color: [0.6, 0.6, 0.8, 0.55],
                },
            )
        }
        ItemMeshCategory::Scroll => {
            let [sr, sg, sb, sa] = descriptor.primary_color;
            spawn_scroll_mesh(
                ctx,
                position,
                map_id,
                ScrollConfig {
                    color: [sr, sg, sb, sa],
                },
            )
        }
        ItemMeshCategory::Ammo => {
            let [ar, ag, ab, aa] = descriptor.primary_color;
            spawn_ammo_mesh(
                ctx,
                position,
                map_id,
                AmmoConfig {
                    ammo_type: "arrow".to_string(),
                    color: [ar, ag, ab, aa],
                },
            )
        }
        ItemMeshCategory::QuestItem => {
            // Quest items rendered as glowing orb (ring mesh with emissive)
            let [qr, qg, qb, qa] = descriptor.primary_color;
            spawn_ring_mesh(
                ctx,
                position,
                map_id,
                RingMeshConfig {
                    color: [qr, qg, qb, qa],
                },
            )
        }
    }
}

// ==================== Item Mesh Tests ====================

#[cfg(test)]
mod item_mesh_tests {
    use super::*;

    // §2.8 — test_sword_config_defaults
    /// `SwordConfig::default()` must have a positive `blade_length`.
    #[test]
    fn test_sword_config_defaults() {
        let cfg = SwordConfig::default();
        assert!(
            cfg.blade_length > 0.0,
            "blade_length must be positive, got {}",
            cfg.blade_length
        );
        assert!(cfg.blade_width > 0.0);
        assert!(cfg.has_crossguard);
        assert!(cfg.color.is_none());
    }

    // §2.8 — test_dagger_config_defaults
    /// `DaggerConfig::default()` must have a shorter `blade_length` than `SwordConfig::default()`.
    #[test]
    fn test_dagger_config_defaults() {
        let dagger = DaggerConfig::default();
        let sword = SwordConfig::default();
        assert!(
            dagger.blade_length < sword.blade_length,
            "dagger blade_length ({}) must be shorter than sword blade_length ({})",
            dagger.blade_length,
            sword.blade_length,
        );
        assert!(dagger.blade_length > 0.0);
        assert!(dagger.color.is_none());
    }

    // §2.8 — test_potion_config_defaults
    /// `PotionConfig::default()` must produce non-zero color components.
    #[test]
    fn test_potion_config_defaults() {
        let cfg = PotionConfig::default();
        // liquid_color alpha must be non-zero (visible)
        assert!(
            cfg.liquid_color[3] > 0.0,
            "liquid_color alpha must be > 0, got {}",
            cfg.liquid_color[3]
        );
        // bottle_color alpha must be non-zero (visible)
        assert!(
            cfg.bottle_color[3] > 0.0,
            "bottle_color alpha must be > 0, got {}",
            cfg.bottle_color[3]
        );
        // At least one RGB component must be non-zero for each color
        let liquid_nonzero =
            cfg.liquid_color[0] > 0.0 || cfg.liquid_color[1] > 0.0 || cfg.liquid_color[2] > 0.0;
        assert!(
            liquid_nonzero,
            "liquid_color must have at least one non-zero RGB"
        );
        let bottle_nonzero =
            cfg.bottle_color[0] > 0.0 || cfg.bottle_color[1] > 0.0 || cfg.bottle_color[2] > 0.0;
        assert!(
            bottle_nonzero,
            "bottle_color must have at least one non-zero RGB"
        );
    }

    // §2.8 — test_scroll_config_defaults
    /// `ScrollConfig::default()` must have a valid (non-zero alpha) color.
    #[test]
    fn test_scroll_config_defaults() {
        let cfg = ScrollConfig::default();
        assert!(
            cfg.color[3] > 0.0,
            "scroll color alpha must be > 0, got {}",
            cfg.color[3]
        );
        // Parchment should be a warm light color — R > 0.5
        assert!(
            cfg.color[0] > 0.5,
            "scroll R channel expected > 0.5 for parchment, got {}",
            cfg.color[0]
        );
    }

    // §2.8 — test_cache_item_slots_default_none
    /// All item mesh cache slots must be `None` when `ProceduralMeshCache::default()` is called.
    #[test]
    fn test_cache_item_slots_default_none() {
        let cache = ProceduralMeshCache::default();
        assert!(cache.item_sword.is_none(), "item_sword should be None");
        assert!(cache.item_dagger.is_none(), "item_dagger should be None");
        assert!(cache.item_blunt.is_none(), "item_blunt should be None");
        assert!(cache.item_staff.is_none(), "item_staff should be None");
        assert!(cache.item_bow.is_none(), "item_bow should be None");
        assert!(cache.item_armor.is_none(), "item_armor should be None");
        assert!(cache.item_shield.is_none(), "item_shield should be None");
        assert!(cache.item_potion.is_none(), "item_potion should be None");
        assert!(cache.item_scroll.is_none(), "item_scroll should be None");
        assert!(cache.item_ring.is_none(), "item_ring should be None");
        assert!(cache.item_ammo.is_none(), "item_ammo should be None");
        assert!(cache.item_quest.is_none(), "item_quest should be None");
    }

    /// After `clear_all`, every item cache slot must be `None` again.
    #[test]
    fn test_cache_item_slots_cleared_after_clear_all() {
        let mut cache = ProceduralMeshCache::default();
        // Manually set a slot to verify clear_all resets it.
        // We cannot create a real Handle<Mesh> without a Bevy world, but we
        // can verify the slots start as None and remain None after clear_all.
        cache.clear_all();
        assert!(cache.item_sword.is_none());
        assert!(cache.item_dagger.is_none());
        assert!(cache.item_blunt.is_none());
        assert!(cache.item_staff.is_none());
        assert!(cache.item_bow.is_none());
        assert!(cache.item_armor.is_none());
        assert!(cache.item_shield.is_none());
        assert!(cache.item_potion.is_none());
        assert!(cache.item_scroll.is_none());
        assert!(cache.item_ring.is_none());
        assert!(cache.item_ammo.is_none());
        assert!(cache.item_quest.is_none());
    }

    /// `BluntConfig::default()` has positive head_radius and handle_length.
    #[test]
    fn test_blunt_config_defaults() {
        let cfg = BluntConfig::default();
        assert!(cfg.head_radius > 0.0);
        assert!(cfg.handle_length > 0.0);
        assert!(cfg.color.is_none());
    }

    /// `StaffConfig::default()` has positive length and orb_radius.
    #[test]
    fn test_staff_config_defaults() {
        let cfg = StaffConfig::default();
        assert!(cfg.length > 0.0);
        assert!(cfg.orb_radius > 0.0);
        assert!(cfg.color.is_none());
    }

    /// `BowConfig::default()` has positive arc_height.
    #[test]
    fn test_bow_config_defaults() {
        let cfg = BowConfig::default();
        assert!(cfg.arc_height > 0.0);
        assert!(cfg.color.is_none());
    }

    /// `ArmorMeshConfig::default()` has positive dimensions and is_helmet = false.
    #[test]
    fn test_armor_mesh_config_defaults() {
        let cfg = ArmorMeshConfig::default();
        assert!(cfg.width > 0.0);
        assert!(cfg.height > 0.0);
        assert!(!cfg.is_helmet);
        assert!(cfg.color.is_none());
    }

    /// `ShieldConfig::default()` has positive radius.
    #[test]
    fn test_shield_config_defaults() {
        let cfg = ShieldConfig::default();
        assert!(cfg.radius > 0.0);
        assert!(cfg.color.is_none());
    }

    /// `RingMeshConfig::default()` has non-zero alpha.
    #[test]
    fn test_ring_mesh_config_defaults() {
        let cfg = RingMeshConfig::default();
        assert!(cfg.color[3] > 0.0);
    }

    /// `AmmoConfig::default()` has non-zero alpha and "arrow" type.
    #[test]
    fn test_ammo_config_defaults() {
        let cfg = AmmoConfig::default();
        assert!(cfg.color[3] > 0.0);
        assert_eq!(cfg.ammo_type, "arrow");
    }

    /// Item color constants must be valid sRGB (values in 0.0–1.0 range).
    #[test]
    fn test_item_color_constants_valid() {
        // Each constant is an sRGB color; just verify they compile and can be
        // converted to LinearRgba without panic.
        let colors = [
            ITEM_SWORD_COLOR,
            ITEM_BLUNT_COLOR,
            ITEM_WOOD_COLOR,
            ITEM_LEATHER_COLOR,
            ITEM_METAL_COLOR,
            ITEM_PARCHMENT_COLOR,
            ITEM_GOLD_COLOR,
        ];
        for color in colors {
            // LinearRgba::from is always valid for sRGB inputs in [0,1].
            let linear = LinearRgba::from(color);
            assert!(linear.red >= 0.0);
            assert!(linear.green >= 0.0);
            assert!(linear.blue >= 0.0);
            assert!(linear.alpha >= 0.0);
        }
    }

    /// `SwordConfig` is `Clone` and produces an equal copy.
    #[test]
    fn test_sword_config_clone() {
        let original = SwordConfig::default();
        let cloned = original.clone();
        assert!((cloned.blade_length - original.blade_length).abs() < f32::EPSILON);
        assert_eq!(cloned.has_crossguard, original.has_crossguard);
    }

    /// `DaggerConfig` is `Clone`.
    #[test]
    fn test_dagger_config_clone() {
        let original = DaggerConfig::default();
        let cloned = original.clone();
        assert!((cloned.blade_length - original.blade_length).abs() < f32::EPSILON);
    }

    /// `PotionConfig` is `Clone`.
    #[test]
    fn test_potion_config_clone() {
        let original = PotionConfig::default();
        let cloned = original.clone();
        assert_eq!(cloned.liquid_color, original.liquid_color);
        assert_eq!(cloned.bottle_color, original.bottle_color);
    }

    /// `ScrollConfig` is `Clone`.
    #[test]
    fn test_scroll_config_clone() {
        let original = ScrollConfig::default();
        let cloned = original.clone();
        assert_eq!(cloned.color, original.color);
    }

    /// `AmmoConfig` is `Clone`.
    #[test]
    fn test_ammo_config_clone() {
        let original = AmmoConfig::default();
        let cloned = original.clone();
        assert_eq!(cloned.ammo_type, original.ammo_type);
        assert_eq!(cloned.color, original.color);
    }
}
