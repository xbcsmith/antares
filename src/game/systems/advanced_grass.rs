// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Advanced grass rendering systems
//!
//! This module contains the core grass rendering pipeline, including:
//! - Procedural blade mesh generation
//! - Grass cluster spawning with configurable blade appearance
//! - Culling and LOD systems for performance
//! - Optional chunk-based mesh merging for advanced optimization
//!
//! Grass rendering consumes **domain** content density (`GrassDensity`) and
//! applies **performance scaling** (`GrassPerformanceLevel`) to determine
//! the final blade count per tile.

use bevy::log::debug;
use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;

use crate::domain::types;
use crate::domain::world::{self, TileVisualMetadata};
use crate::game::components::Billboard;
use crate::game::resources::GrassQualitySettings;
use crate::game::systems::map::{MapEntity, TileCoord};

// ==================== Constants ====================

const GRASS_BLADE_WIDTH: f32 = 0.15;
const GRASS_BLADE_HEIGHT_BASE: f32 = 0.4;
const GRASS_BLADE_Y_OFFSET: f32 = 0.0;
const TILE_CENTER_OFFSET: f32 = 0.5;

// ==================== Phase 2: Grass Rendering Components ====================

/// Component marking a grass cluster (parent entity containing multiple blades)
///
/// Used for distance-based culling to optimize rendering performance.
/// Clusters are culled when the camera is beyond `cull_distance`.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassCluster;
///
/// let cluster = GrassCluster { cull_distance: 50.0 };
/// ```
#[derive(Component, Clone, Copy, Debug)]
pub struct GrassCluster {
    /// Distance beyond which this cluster should be culled (hidden)
    pub cull_distance: f32,
}

impl Default for GrassCluster {
    fn default() -> Self {
        Self {
            cull_distance: 50.0,
        }
    }
}

/// Component marking an individual grass blade within a cluster
///
/// Used for LOD (Level of Detail) system to reduce blade count at distance.
/// The `lod_index` determines which blades are hidden in far LOD mode.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassBlade;
///
/// let blade = GrassBlade { lod_index: 0 };
/// ```
#[derive(Component, Clone, Copy, Debug)]
pub struct GrassBlade {
    /// Index of this blade within its cluster (0-indexed)
    pub lod_index: u32,
}

/// Component attached to spawned grass blades containing the original
/// mesh and material handles used for that blade.
///
/// This component is consumed by chunking/instancing systems so the renderer
/// can collect geometry and material data without relying on spawn internals.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassBladeInstance;
/// use bevy::prelude::{Handle, Mesh, StandardMaterial};
///
/// fn build_instance(mesh: Handle<Mesh>, material: Handle<StandardMaterial>) -> GrassBladeInstance {
///     GrassBladeInstance { mesh, material }
/// }
/// ```
#[derive(Component, Clone, Debug)]
pub struct GrassBladeInstance {
    /// Mesh handle for this blade (copied from spawn-time mesh)
    pub mesh: Handle<Mesh>,
    /// Material handle for this blade (copied from spawn-time material)
    pub material: Handle<StandardMaterial>,
}

/// Resource configuring grass rendering performance settings
///
/// Controls culling and LOD distances for grass rendering optimization.
/// Essential for maintaining 60fps with grass-heavy maps.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassRenderConfig;
///
/// let config = GrassRenderConfig {
///     cull_distance: 50.0,
///     lod_distance: 25.0,
/// };
/// ```
#[derive(Resource, Clone, Copy, Debug)]
pub struct GrassRenderConfig {
    /// Distance beyond which grass clusters are culled (default: 50.0)
    pub cull_distance: f32,
    /// Distance beyond which LOD reduction begins (default: 25.0)
    pub lod_distance: f32,
}

impl Default for GrassRenderConfig {
    fn default() -> Self {
        Self {
            cull_distance: 50.0,
            lod_distance: 25.0,
        }
    }
}

// ==================== Phase 4: Instance Batching ====================

/// Resource controlling grass instance batching behavior
///
/// When enabled, the system will aggregate per-blade instance data into
/// batches for future GPU instancing pipelines while keeping existing
/// blade meshes rendered.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassInstanceConfig;
///
/// let config = GrassInstanceConfig::default();
/// assert!(!config.enabled);
/// ```
#[derive(Resource, Clone, Copy, Debug)]
pub struct GrassInstanceConfig {
    /// Whether instance batching is enabled
    pub enabled: bool,
    /// Maximum instances per batch entity
    pub max_instances_per_batch: usize,
}

impl Default for GrassInstanceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_instances_per_batch: 1024,
        }
    }
}

/// Component storing instance data for a batched grass mesh
///
/// Instance batches are used for performance diagnostics and future GPU
/// instancing pipelines. The existing blade meshes remain rendered until
/// a GPU instancing pipeline is introduced.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassInstanceBatch;
/// use antares::domain::world::InstanceData;
/// use bevy::prelude::{Handle, Mesh, StandardMaterial};
///
/// let batch = GrassInstanceBatch {
///     mesh: Handle::<Mesh>::default(),
///     material: Handle::<StandardMaterial>::default(),
///     instances: vec![InstanceData::new([0.0, 0.0, 0.0])],
/// };
/// ```
#[derive(Component, Clone, Debug)]
pub struct GrassInstanceBatch {
    /// Mesh handle shared by all instances
    pub mesh: Handle<Mesh>,
    /// Material handle shared by all instances
    pub material: Handle<StandardMaterial>,
    /// Per-instance transform data
    pub instances: Vec<world::InstanceData>,
}

// ==================== Phase 3: Blade Configuration ====================

/// Configuration for individual grass blade appearance
///
/// Controls visual properties of spawned grass blades.
/// Converted from domain-layer `GrassBladeConfig` with clamped values.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::BladeConfig;
///
/// let config = BladeConfig {
///     length: 1.5,
///     width: 0.8,
///     tilt: 0.4,
///     curve: 0.5,
///     color_variation: 0.3,
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct BladeConfig {
    /// Blade length multiplier (clamped 0.5-2.0)
    pub length: f32,
    /// Blade width multiplier (clamped 0.5-2.0)
    pub width: f32,
    /// Blade tilt angle in radians (clamped 0.0-0.5)
    pub tilt: f32,
    /// Blade curvature amount (clamped 0.0-1.0)
    pub curve: f32,
    /// Color variation (clamped 0.0-1.0)
    pub color_variation: f32,
}

impl Default for BladeConfig {
    fn default() -> Self {
        Self {
            length: 1.0,
            width: 1.0,
            tilt: 0.3,
            curve: 0.3,
            color_variation: 0.2,
        }
    }
}

impl From<&world::GrassBladeConfig> for BladeConfig {
    fn from(config: &world::GrassBladeConfig) -> Self {
        Self {
            length: config.length.clamp(0.5, 2.0),
            width: config.width.clamp(0.5, 2.0),
            tilt: config.tilt.clamp(0.0, 0.5),
            curve: config.curve.clamp(0.0, 1.0),
            color_variation: config.color_variation.clamp(0.0, 1.0),
        }
    }
}

/// Color scheme for grass blades with variation support
///
/// Provides base and tip colors with random variation to create
/// natural-looking grass.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassColorScheme;
/// use bevy::prelude::Color;
///
/// let scheme = GrassColorScheme {
///     base_color: Color::srgb(0.2, 0.5, 0.1),
///     tip_color: Color::srgb(0.3, 0.7, 0.2),
///     variation: 0.3,
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct GrassColorScheme {
    /// Base color at blade bottom
    pub base_color: Color,
    /// Tip color at blade top
    pub tip_color: Color,
    /// Variation amount (0.0-1.0)
    pub variation: f32,
}

impl GrassColorScheme {
    /// Sample a random blade color with variation
    ///
    /// # Arguments
    ///
    /// * `rng` - Random number generator
    ///
    /// # Returns
    ///
    /// A color with applied variation
    pub fn sample_blade_color(&self, rng: &mut impl rand::Rng) -> Color {
        let base_blend = 0.7;
        let blended = Color::srgb(
            self.base_color.to_srgba().red * base_blend
                + self.tip_color.to_srgba().red * (1.0 - base_blend),
            self.base_color.to_srgba().green * base_blend
                + self.tip_color.to_srgba().green * (1.0 - base_blend),
            self.base_color.to_srgba().blue * base_blend
                + self.tip_color.to_srgba().blue * (1.0 - base_blend),
        );

        if self.variation > 0.0 {
            let variation_amount = rng.random_range(-self.variation..self.variation);
            Color::srgb(
                (blended.to_srgba().red + variation_amount).clamp(0.0, 1.0),
                (blended.to_srgba().green + variation_amount).clamp(0.0, 1.0),
                (blended.to_srgba().blue + variation_amount).clamp(0.0, 1.0),
            )
        } else {
            blended
        }
    }
}

impl Default for GrassColorScheme {
    fn default() -> Self {
        Self {
            base_color: Color::srgb(0.2, 0.5, 0.1),
            tip_color: Color::srgb(0.3, 0.7, 0.2),
            variation: 0.2,
        }
    }
}

// ==================== Grass Mesh Generation ====================

fn create_grass_blade_mesh(height: f32, width: f32, curve_amount: f32) -> Mesh {
    let segment_count = 4;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let mut curve_points = Vec::with_capacity(segment_count + 1);

    for i in 0..=segment_count {
        let t = i as f32 / segment_count as f32;

        let p0_y = 0.0;
        let p1_y = height * 0.5;
        let p2_y = height;

        let one_minus_t = 1.0 - t;
        let coeff0 = one_minus_t * one_minus_t;
        let coeff1 = 2.0 * one_minus_t * t;
        let coeff2 = t * t;

        let curve_x = coeff0 * 0.0 + coeff1 * 0.0 + coeff2 * curve_amount;
        let curve_y = coeff0 * p0_y + coeff1 * p1_y + coeff2 * p2_y;

        curve_points.push(Vec3::new(0.0, curve_y, curve_x));
    }

    for i in 0..=segment_count {
        let t = i as f32 / segment_count as f32;
        let point = curve_points[i];
        let tangent = if i == 0 {
            curve_points[1] - curve_points[0]
        } else if i == segment_count {
            curve_points[segment_count] - curve_points[segment_count - 1]
        } else {
            curve_points[i + 1] - curve_points[i - 1]
        };

        let normal = Vec3::X.cross(tangent).normalize_or_zero();
        let taper_width = width * (1.0 - t);

        positions.push([-taper_width / 2.0, point.y, point.z]);
        normals.push([normal.x, normal.y, normal.z]);
        uvs.push([0.0, t]);

        positions.push([taper_width / 2.0, point.y, point.z]);
        normals.push([normal.x, normal.y, normal.z]);
        uvs.push([1.0, t]);
    }

    for i in 0..segment_count {
        let base = (i * 2) as u32;
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);

        indices.push(base + 1);
        indices.push(base + 3);
        indices.push(base + 2);
    }

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::all(),
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));

    mesh
}

// ==================== Grass Spawning ====================

#[allow(clippy::too_many_arguments)]
fn spawn_grass_cluster(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    cluster_center: Vec2,
    blade_height: f32,
    blade_config: &BladeConfig,
    color_scheme: &GrassColorScheme,
    parent_entity: Entity,
) {
    let mut rng = rand::rng();
    let blade_count_in_cluster = rng.random_range(5..=10);
    let cluster_radius = 0.1;

    for blade_index in 0..blade_count_in_cluster {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let distance = rng.random_range(0.0..cluster_radius);
        let offset_x = angle.cos() * distance;
        let offset_z = angle.sin() * distance;

        let blade_x = cluster_center.x + offset_x;
        let blade_z = cluster_center.y + offset_z;

        let height_variation = rng.random_range(0.7..=1.3);
        let varied_height = blade_height * blade_config.length * height_variation;

        let width_variation = rng.random_range(0.8..=1.2);
        let varied_width = GRASS_BLADE_WIDTH * blade_config.width * width_variation;

        let curve_variation = rng.random_range(0.0..=1.0);
        let curve_amount = blade_config.curve * curve_variation * 0.3;

        let rotation_y = rng.random_range(0.0..std::f32::consts::TAU);
        let lean_angle = rng.random_range(0.0..std::f32::consts::TAU);
        let lean_dir = Vec3::new(lean_angle.cos(), 0.0, lean_angle.sin());
        let tilt_axis = Vec3::new(-lean_dir.z, 0.0, lean_dir.x).normalize_or_zero();
        let tilt_amount = rng.random_range(0.0..=blade_config.tilt);
        let tilt_rotation = if tilt_axis == Vec3::ZERO {
            Quat::IDENTITY
        } else {
            Quat::from_axis_angle(tilt_axis, tilt_amount)
        };
        let final_rotation = Quat::from_rotation_y(rotation_y) * tilt_rotation;

        let blade_mesh = meshes.add(create_grass_blade_mesh(
            varied_height,
            varied_width,
            curve_amount,
        ));

        let blade_color = color_scheme.sample_blade_color(&mut rng);

        let blade_material = materials.add(StandardMaterial {
            base_color: blade_color,
            perceptual_roughness: 0.7,
            double_sided: true,
            cull_mode: None,
            alpha_mode: AlphaMode::Opaque,
            ..default()
        });

        let blade = commands
            .spawn((
                Mesh3d(blade_mesh.clone()),
                MeshMaterial3d(blade_material.clone()),
                Transform::from_xyz(blade_x, GRASS_BLADE_Y_OFFSET, blade_z)
                    .with_rotation(final_rotation),
                GlobalTransform::default(),
                Visibility::default(),
                Billboard::default(),
                GrassBlade {
                    lod_index: blade_index as u32,
                },
                GrassBladeInstance {
                    mesh: blade_mesh.clone(),
                    material: blade_material.clone(),
                },
            ))
            .id();

        commands.entity(parent_entity).add_child(blade);
    }
}

/// Spawns grass clusters for a terrain tile
///
/// This is the main entry point for grass rendering. It computes the
/// blade count using content density and performance settings, then
/// spawns one or more clusters within the tile.
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `visual_metadata` - Optional per-tile visual customization
/// * `quality_settings` - Performance settings for grass density scaling
///
/// # Returns
///
/// Entity ID of the parent grass cluster entity
///
/// # Examples
///
/// ```rust
/// use antares::domain::types::{MapId, Position};
/// use antares::game::resources::GrassQualitySettings;
/// use antares::game::systems::advanced_grass::spawn_grass;
/// use bevy::prelude::{Assets, Commands, Mesh, StandardMaterial};
///
/// fn spawn_example(
///     mut commands: Commands,
///     mut materials: bevy::prelude::ResMut<Assets<StandardMaterial>>,
///     mut meshes: bevy::prelude::ResMut<Assets<Mesh>>,
///     settings: bevy::prelude::Res<GrassQualitySettings>,
/// ) {
///     let _entity = spawn_grass(
///         &mut commands,
///         &mut materials,
///         &mut meshes,
///         Position::new(1, 2),
///         1u16,
///         None,
///         &settings,
///     );
/// }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn spawn_grass(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    map_id: types::MapId,
    visual_metadata: Option<&TileVisualMetadata>,
    quality_settings: &GrassQualitySettings,
) -> Entity {
    debug!(
        "spawn_grass called at tile ({}, {}) map {:?}",
        position.x, position.y, map_id
    );

    let blade_config = visual_metadata
        .and_then(|m| m.grass_blade_config.as_ref())
        .map(BladeConfig::from)
        .unwrap_or_default();

    let blade_height = visual_metadata
        .and_then(|m| m.height)
        .unwrap_or(GRASS_BLADE_HEIGHT_BASE)
        .clamp(0.2, 0.6);

    let color_tint = visual_metadata
        .and_then(|m| m.color_tint)
        .unwrap_or((0.3, 0.65, 0.2));

    let base_color = Color::srgb(0.2 * color_tint.0, 0.5 * color_tint.1, 0.1 * color_tint.2);
    let tip_color = Color::srgb(0.3 * color_tint.0, 0.7 * color_tint.1, 0.2 * color_tint.2);

    let color_scheme = GrassColorScheme {
        base_color,
        tip_color,
        variation: blade_config.color_variation,
    };

    let content_density = visual_metadata
        .and_then(|m| m.grass_density)
        .unwrap_or_default();

    let (min_blades, max_blades) = quality_settings.blade_count_range_for_content(content_density);

    let mut rng = rand::rng();
    let blade_count = if max_blades > 0 {
        rng.random_range(min_blades..=max_blades)
    } else {
        0
    };

    debug!(
        "grass blades for tile ({}, {}): content={:?} min={} max={} chosen={}",
        position.x, position.y, content_density, min_blades, max_blades, blade_count
    );

    if blade_count == 0 {
        return commands
            .spawn((
                Transform::from_xyz(
                    position.x as f32 + TILE_CENTER_OFFSET,
                    0.0,
                    position.y as f32 + TILE_CENTER_OFFSET,
                ),
                GlobalTransform::default(),
                Visibility::default(),
                MapEntity(map_id),
                TileCoord(position),
            ))
            .id();
    }

    let parent = commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                0.0,
                position.y as f32 + TILE_CENTER_OFFSET,
            ),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            GrassCluster::default(),
        ))
        .id();

    let cluster_count = (blade_count / 7).max(1);

    for _ in 0..cluster_count {
        let cluster_x = rng.random_range(-0.4..0.4);
        let cluster_z = rng.random_range(-0.4..0.4);
        let cluster_center = Vec2::new(cluster_x, cluster_z);

        spawn_grass_cluster(
            commands,
            materials,
            meshes,
            cluster_center,
            blade_height,
            &blade_config,
            &color_scheme,
            parent,
        );
    }

    debug!(
        "spawn_grass completed at tile ({}, {}) with {} clusters",
        position.x, position.y, cluster_count
    );

    parent
}

// ==================== Phase 2: Grass Performance Systems ====================

/// System that culls grass clusters beyond the configured distance
///
/// # Examples
///
/// ```rust
/// use bevy::prelude::*;
/// use antares::game::systems::advanced_grass::{grass_distance_culling_system, GrassRenderConfig};
///
/// fn setup_app(app: &mut App) {
///     app.insert_resource(GrassRenderConfig::default())
///        .add_systems(Update, grass_distance_culling_system);
/// }
/// ```
pub fn grass_distance_culling_system(
    mut grass_query: Query<(&GlobalTransform, &mut Visibility, &GrassCluster)>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    config: Res<GrassRenderConfig>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let camera_pos = camera_transform.translation();

    for (transform, mut visibility, cluster) in grass_query.iter_mut() {
        let distance = camera_pos.distance(transform.translation());

        if distance > cluster.cull_distance.max(config.cull_distance) {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Inherited;
        }
    }
}

/// System that reduces grass blade count at distance (LOD)
///
/// # Examples
///
/// ```rust
/// use bevy::prelude::*;
/// use antares::game::systems::advanced_grass::{grass_lod_system, GrassRenderConfig};
///
/// fn setup_app(app: &mut App) {
///     app.insert_resource(GrassRenderConfig::default())
///        .add_systems(Update, grass_lod_system);
/// }
/// ```
pub fn grass_lod_system(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    cluster_query: Query<(&GlobalTransform, &Children), With<GrassCluster>>,
    mut blade_query: Query<(&mut Visibility, &GrassBlade)>,
    config: Res<GrassRenderConfig>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let camera_pos = camera_transform.translation();

    for (cluster_transform, children) in cluster_query.iter() {
        let distance = camera_pos.distance(cluster_transform.translation());

        if distance > config.cull_distance {
            continue;
        }

        let lod_ratio = if distance > config.lod_distance {
            0.5
        } else {
            1.0
        };

        for child in children.iter() {
            if let Ok((mut visibility, blade)) = blade_query.get_mut(child) {
                if lod_ratio < 1.0 && blade.lod_index % 2 == 1 {
                    *visibility = Visibility::Hidden;
                } else {
                    *visibility = Visibility::Inherited;
                }
            }
        }
    }
}

#[derive(Hash, PartialEq, Eq)]
struct InstanceBatchKey {
    mesh: bevy::asset::AssetId<Mesh>,
    material: bevy::asset::AssetId<StandardMaterial>,
    map_id: types::MapId,
}

struct InstanceBatchEntry {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    instances: Vec<world::InstanceData>,
}

/// Build instance batches from existing grass blades
///
/// This system aggregates per-blade instance data into batch components to
/// support performance diagnostics and future GPU instancing pipelines.
///
/// # Examples
///
/// ```rust
/// use bevy::prelude::*;
/// use antares::game::systems::advanced_grass::{
///     build_grass_instance_batches_system, GrassInstanceConfig,
/// };
///
/// fn setup_app(app: &mut App) {
///     app.insert_resource(GrassInstanceConfig::default())
///        .add_systems(Update, build_grass_instance_batches_system);
/// }
/// ```
pub fn build_grass_instance_batches_system(
    mut commands: Commands,
    cluster_query: Query<(&Children, &GlobalTransform, &MapEntity), With<GrassCluster>>,
    blade_query: Query<(&Transform, Option<&GlobalTransform>, &GrassBladeInstance)>,
    mut existing_batches: Query<Entity, With<GrassInstanceBatch>>,
    config: Res<GrassInstanceConfig>,
) {
    if !config.enabled {
        return;
    }

    for ent in existing_batches.iter_mut() {
        commands.entity(ent).despawn();
    }

    let mut buckets: HashMap<InstanceBatchKey, InstanceBatchEntry> = HashMap::new();

    for (children, cluster_global, map_entity) in cluster_query.iter() {
        for child in children.iter() {
            if let Ok((child_local, child_global_opt, blade_inst)) = blade_query.get(child) {
                let world_transform = if let Some(child_global) = child_global_opt {
                    if child_global.translation() == Vec3::ZERO
                        && child_local.translation != Vec3::ZERO
                    {
                        cluster_global.mul_transform(*child_local)
                    } else {
                        *child_global
                    }
                } else {
                    cluster_global.mul_transform(*child_local)
                };

                let (scale, rotation, translation) =
                    world_transform.to_scale_rotation_translation();
                let (yaw, _, _) = rotation.to_euler(EulerRot::YXZ);
                let uniform_scale = (scale.x + scale.y + scale.z) / 3.0;

                let instance = world::InstanceData {
                    position: [translation.x, translation.y, translation.z],
                    scale: uniform_scale,
                    rotation_y: yaw,
                };

                let key = InstanceBatchKey {
                    mesh: blade_inst.mesh.id(),
                    material: blade_inst.material.id(),
                    map_id: map_entity.0,
                };

                let entry = buckets.entry(key).or_insert_with(|| InstanceBatchEntry {
                    mesh: blade_inst.mesh.clone(),
                    material: blade_inst.material.clone(),
                    instances: Vec::new(),
                });
                entry.instances.push(instance);
            }
        }
    }

    let max_instances = config.max_instances_per_batch.max(1);
    for (key, entry) in buckets.into_iter() {
        for (batch_index, chunk) in entry.instances.chunks(max_instances).enumerate() {
            commands.spawn((
                GrassInstanceBatch {
                    mesh: entry.mesh.clone(),
                    material: entry.material.clone(),
                    instances: chunk.to_vec(),
                },
                MapEntity(key.map_id),
                Name::new(format!("GrassInstanceBatch_{}_{}", key.map_id, batch_index)),
            ));
        }
    }
}

// ==================== Phase 4: Optional Chunking ====================

/// Component that marks a merged grass chunk entity
#[derive(Component, Clone, Copy, Debug)]
pub struct GrassChunk {
    /// Chunk coordinates (x,z) in chunk-space
    pub coords: (i32, i32),
    /// Distance beyond which the chunk should be considered for culling
    pub cull_distance: f32,
}

impl Default for GrassChunk {
    fn default() -> Self {
        Self {
            coords: (0, 0),
            cull_distance: 50.0,
        }
    }
}

/// Configuration resource for chunking behavior
#[derive(Resource, Clone, Copy, Debug)]
pub struct GrassChunkConfig {
    /// Width/height of a single chunk (world units)
    pub chunk_size: f32,
    /// Maximum expected blade height (used to conservatively expand AABB)
    pub max_blade_height: f32,
}

impl Default for GrassChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 30.0,
            max_blade_height: 1.0,
        }
    }
}

struct BladeGather {
    material: Handle<StandardMaterial>,
    world_transform: GlobalTransform,
    map_id: crate::domain::types::MapId,
}

/// Build merged chunk meshes for existing grass blades.
pub fn build_grass_chunks_system(
    mut commands: Commands,
    cluster_query: Query<(&Children, &GlobalTransform, &MapEntity), With<GrassCluster>>,
    blade_query: Query<(&Transform, Option<&GlobalTransform>, &GrassBladeInstance)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut existing_chunks: Query<Entity, With<GrassChunk>>,
    config: Option<Res<GrassChunkConfig>>,
) {
    for ent in existing_chunks.iter_mut() {
        commands.entity(ent).despawn();
    }

    let mut buckets: HashMap<(i32, i32), Vec<BladeGather>> = HashMap::new();
    let config = config.map(|r| *r).unwrap_or_default();

    for (children, cluster_global, map_entity) in cluster_query.iter() {
        for child in children.iter() {
            if let Ok((child_local, child_global_opt, blade_inst)) = blade_query.get(child) {
                let world_transform = if let Some(child_global) = child_global_opt {
                    if child_global.translation() == Vec3::ZERO
                        && child_local.translation != Vec3::ZERO
                    {
                        cluster_global.mul_transform(*child_local)
                    } else {
                        *child_global
                    }
                } else {
                    cluster_global.mul_transform(*child_local)
                };

                let pos = world_transform.translation();
                let cx = (pos.x / config.chunk_size).floor() as i32;
                let cz = (pos.z / config.chunk_size).floor() as i32;
                buckets.entry((cx, cz)).or_default().push(BladeGather {
                    material: blade_inst.material.clone(),
                    world_transform,
                    map_id: map_entity.0,
                });
            }
        }
    }

    for ((cx, cz), blades) in buckets.into_iter() {
        let chunk_center = Vec3::new(
            cx as f32 * config.chunk_size + config.chunk_size * 0.5,
            0.0,
            cz as f32 * config.chunk_size + config.chunk_size * 0.5,
        );

        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut normals: Vec<[f32; 3]> = Vec::new();
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        let mut vertex_offset: u32 = 0;

        for blade in blades.iter() {
            let world_pos = blade.world_transform.translation();
            let local = world_pos - chunk_center;

            let half_w = 0.1_f32;
            let height = config.max_blade_height;

            positions.push([local.x - half_w, 0.0, local.z]);
            positions.push([local.x + half_w, 0.0, local.z]);
            positions.push([local.x - half_w, height, local.z]);
            positions.push([local.x + half_w, height, local.z]);

            normals.push([0.0, 1.0, 0.0]);
            normals.push([0.0, 1.0, 0.0]);
            normals.push([0.0, 1.0, 0.0]);
            normals.push([0.0, 1.0, 0.0]);

            uvs.push([0.0, 0.0]);
            uvs.push([1.0, 0.0]);
            uvs.push([0.0, 1.0]);
            uvs.push([1.0, 1.0]);

            indices.push(vertex_offset);
            indices.push(vertex_offset + 1);
            indices.push(vertex_offset + 2);
            indices.push(vertex_offset + 2);
            indices.push(vertex_offset + 1);
            indices.push(vertex_offset + 3);

            vertex_offset += 4;
        }

        if positions.is_empty() || indices.is_empty() {
            continue;
        }

        let mut merged = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::all(),
        );
        merged.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        merged.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        merged.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        merged.insert_indices(bevy::mesh::Indices::U32(indices));

        let mesh_handle = meshes.add(merged);

        let material_color = blades
            .first()
            .and_then(|b| materials.get(&b.material))
            .map(|m| m.base_color)
            .unwrap_or(Color::srgb(0.25, 0.6, 0.25));

        let material_handle = materials.add(StandardMaterial {
            base_color: material_color,
            perceptual_roughness: 0.8,
            double_sided: true,
            ..default()
        });

        let map_id = blades[0].map_id;
        commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform::from_translation(chunk_center),
            GlobalTransform::default(),
            Visibility::default(),
            GrassChunk {
                coords: (cx, cz),
                cull_distance: GrassChunk::default().cull_distance,
            },
            MapEntity(map_id),
            Name::new(format!("GrassChunk_{}_{}", cx, cz)),
        ));
    }
}

/// Culling system for grass chunks
pub fn grass_chunk_culling_system(
    mut chunk_query: Query<(
        &Transform,
        Option<&GlobalTransform>,
        &mut Visibility,
        &GrassChunk,
    )>,
    camera_query: Query<(&Transform, Option<&GlobalTransform>), With<Camera3d>>,
    config: Option<Res<GrassRenderConfig>>,
) {
    let Ok((cam_local, cam_global_opt)) = camera_query.single() else {
        return;
    };
    let camera_pos = if let Some(cam_global) = cam_global_opt {
        if cam_global.translation() == Vec3::ZERO && cam_local.translation != Vec3::ZERO {
            cam_local.translation
        } else {
            cam_global.translation()
        }
    } else {
        cam_local.translation
    };
    let config = config.map(|r| *r).unwrap_or_default();

    for (local_transform, chunk_global_opt, mut visibility, chunk) in chunk_query.iter_mut() {
        let chunk_pos = if let Some(chunk_global) = chunk_global_opt {
            if chunk_global.translation() == Vec3::ZERO && local_transform.translation != Vec3::ZERO
            {
                local_transform.translation
            } else {
                chunk_global.translation()
            }
        } else {
            local_transform.translation
        };

        let distance = camera_pos.distance(chunk_pos);
        if distance > chunk.cull_distance.max(config.cull_distance) {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Inherited;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::world::GrassDensity;
    use bevy::mesh::Mesh;

    fn compute_bounds(positions: &[[f32; 3]]) -> (Vec3, Vec3) {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        for pos in positions {
            let p = Vec3::new(pos[0], pos[1], pos[2]);
            min = min.min(p);
            max = max.max(p);
        }

        (min, max)
    }

    #[test]
    fn test_blade_config_default_values() {
        let config = BladeConfig::default();
        assert_eq!(config.length, 1.0);
        assert_eq!(config.width, 1.0);
        assert_eq!(config.tilt, 0.3);
        assert_eq!(config.curve, 0.3);
        assert_eq!(config.color_variation, 0.2);
    }

    #[test]
    fn test_grass_render_config_default_values() {
        let config = GrassRenderConfig::default();
        assert_eq!(config.cull_distance, 50.0);
        assert_eq!(config.lod_distance, 25.0);
    }

    #[test]
    fn test_create_grass_blade_mesh_vertex_count() {
        let blade = create_grass_blade_mesh(0.4, 0.15, 0.1);
        assert_eq!(blade.count_vertices(), 10);
    }

    #[test]
    fn test_create_grass_blade_mesh_indices() {
        let blade = create_grass_blade_mesh(0.4, 0.15, 0.1);
        let indices_count = blade
            .indices()
            .map(|indices| match indices {
                bevy::mesh::Indices::U32(idx) => idx.len(),
                bevy::mesh::Indices::U16(idx) => idx.len(),
            })
            .unwrap_or(0);

        assert_eq!(indices_count, 24);
    }

    #[test]
    fn test_create_grass_blade_mesh_normals() {
        let blade = create_grass_blade_mesh(0.4, 0.15, 0.1);
        let normals = blade
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .expect("Blade should have normals")
            .as_float3()
            .expect("Normals should be float3");

        for normal in normals {
            let length =
                (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            assert!(normal[0].abs() < 0.01);
            assert!((length - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_create_grass_blade_mesh_has_uvs() {
        let blade = create_grass_blade_mesh(0.4, 0.15, 0.1);
        assert!(blade.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
    }

    #[test]
    fn test_create_grass_blade_mesh_bounds_are_valid() {
        let blade = create_grass_blade_mesh(0.4, 0.15, 0.1);
        let positions = blade
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("Blade should have positions")
            .as_float3()
            .expect("Positions should be float3");

        let (min, max) = compute_bounds(positions);
        assert!(max.y > min.y);
        assert!(max.x > min.x);
    }

    #[test]
    fn test_grass_blade_child_inherits_parent_transform() {
        let expected = Vec3::new(2.25, 0.0, 3.5);
        let parent = Transform::from_translation(Vec3::new(2.0, 0.0, 3.0));
        let child = Transform::from_translation(Vec3::new(0.25, 0.0, 0.5));
        let combined = parent.mul_transform(child);
        let actual = combined.translation;

        assert!((actual.x - expected.x).abs() < 0.001);
        assert!((actual.y - expected.y).abs() < 0.001);
        assert!((actual.z - expected.z).abs() < 0.001);
    }

    #[test]
    fn test_grass_color_scheme_no_variation_is_stable() {
        let scheme = GrassColorScheme {
            base_color: Color::srgb(0.2, 0.5, 0.1),
            tip_color: Color::srgb(0.3, 0.7, 0.2),
            variation: 0.0,
        };

        let mut rng = rand::rng();
        let color_a = scheme.sample_blade_color(&mut rng);
        let color_b = scheme.sample_blade_color(&mut rng);

        assert_eq!(color_a.to_srgba(), color_b.to_srgba());
    }

    #[test]
    fn test_grass_distance_culling_system_hides_far_cluster() {
        let mut app = App::new();
        app.insert_resource(GrassRenderConfig::default());
        app.add_systems(Update, grass_distance_culling_system);

        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        ));

        let far_cluster = app
            .world_mut()
            .spawn((
                Transform::from_xyz(100.0, 0.0, 0.0),
                GlobalTransform::from(Transform::from_xyz(100.0, 0.0, 0.0)),
                Visibility::default(),
                GrassCluster::default(),
            ))
            .id();

        app.update();

        let visibility = app.world().get::<Visibility>(far_cluster).unwrap();
        assert!(matches!(visibility, Visibility::Hidden));
    }

    #[test]
    fn test_grass_lod_system_hides_odd_blades_far_distance() {
        let mut app = App::new();
        app.insert_resource(GrassRenderConfig {
            cull_distance: 50.0,
            lod_distance: 25.0,
        });
        app.add_systems(Update, grass_lod_system);

        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        ));

        let cluster = app
            .world_mut()
            .spawn((
                Transform::from_xyz(30.0, 0.0, 0.0),
                GlobalTransform::from(Transform::from_xyz(30.0, 0.0, 0.0)),
                Visibility::default(),
                GrassCluster::default(),
            ))
            .id();

        let blade_even = app
            .world_mut()
            .spawn((
                Transform::default(),
                GlobalTransform::default(),
                Visibility::default(),
                GrassBlade { lod_index: 0 },
            ))
            .id();

        let blade_odd = app
            .world_mut()
            .spawn((
                Transform::default(),
                GlobalTransform::default(),
                Visibility::default(),
                GrassBlade { lod_index: 1 },
            ))
            .id();

        app.world_mut().entity_mut(cluster).add_child(blade_even);
        app.world_mut().entity_mut(cluster).add_child(blade_odd);

        app.update();

        let even_visibility = app.world().get::<Visibility>(blade_even).unwrap();
        let odd_visibility = app.world().get::<Visibility>(blade_odd).unwrap();

        assert!(matches!(even_visibility, Visibility::Inherited));
        assert!(matches!(odd_visibility, Visibility::Hidden));
    }

    #[test]
    fn test_build_grass_chunks_groups_blades_into_chunks() {
        let mut app = App::new();

        app.insert_resource(GrassChunkConfig {
            chunk_size: 2.0,
            max_blade_height: 1.0,
        });
        app.insert_resource(GrassRenderConfig::default());

        app.add_systems(Update, build_grass_chunks_system);

        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());

        let cube = {
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
            let mut test_mesh = Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::all(),
            );
            test_mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![[0.0, 0.0, 0.0], [0.1, 0.0, 0.0], [0.0, 0.1, 0.0]],
            );
            test_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 3]);
            test_mesh.insert_attribute(
                Mesh::ATTRIBUTE_UV_0,
                vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            );
            test_mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2]));
            meshes.add(test_mesh)
        };

        let mat = {
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
            materials.add(StandardMaterial {
                base_color: Color::WHITE,
                ..default()
            })
        };

        let cluster = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::ZERO),
                GlobalTransform::default(),
                GrassCluster::default(),
                MapEntity(1),
            ))
            .id();

        let blade_a = app
            .world_mut()
            .spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(Vec3::new(0.1, 0.0, 0.1)),
                GlobalTransform::default(),
                GrassBladeInstance {
                    mesh: cube.clone(),
                    material: mat.clone(),
                },
            ))
            .id();

        let blade_b = app
            .world_mut()
            .spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(Vec3::new(3.1, 0.0, 0.1)),
                GlobalTransform::default(),
                GrassBladeInstance {
                    mesh: cube.clone(),
                    material: mat.clone(),
                },
            ))
            .id();

        app.world_mut().entity_mut(cluster).add_child(blade_a);
        app.world_mut().entity_mut(cluster).add_child(blade_b);

        app.update();

        let mut q = app.world_mut().query_filtered::<Entity, With<GrassChunk>>();
        let count = q.iter(app.world()).count();
        assert!(count >= 2, "Expected at least 2 chunks, found {}", count);
    }

    #[test]
    fn test_grass_chunk_culling_hides_far_chunks() {
        let mut app = App::new();

        app.insert_resource(GrassRenderConfig::default());
        app.add_systems(Update, grass_chunk_culling_system);

        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        ));

        let chunk = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(1000.0, 0.0, 0.0)),
                GlobalTransform::from(Transform::from_translation(Vec3::new(1000.0, 0.0, 0.0))),
                Visibility::default(),
                GrassChunk {
                    coords: (100, 0),
                    cull_distance: 50.0,
                },
            ))
            .id();

        app.update();

        let vis = app.world().get::<Visibility>(chunk).unwrap();
        assert!(matches!(vis, Visibility::Hidden));
    }

    #[test]
    fn test_build_grass_instance_batches_groups_instances() {
        let mut app = App::new();
        app.insert_resource(GrassInstanceConfig {
            enabled: true,
            max_instances_per_batch: 2,
        });
        app.add_systems(Update, build_grass_instance_batches_system);

        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());

        let mesh = {
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
            let mut test_mesh = Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::all(),
            );
            test_mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![[0.0, 0.0, 0.0], [0.1, 0.0, 0.0], [0.0, 0.1, 0.0]],
            );
            test_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 3]);
            test_mesh.insert_attribute(
                Mesh::ATTRIBUTE_UV_0,
                vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            );
            test_mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2]));
            meshes.add(test_mesh)
        };

        let material = {
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
            materials.add(StandardMaterial::default())
        };

        let cluster = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::ZERO),
                GlobalTransform::default(),
                GrassCluster::default(),
                MapEntity(1),
            ))
            .id();

        for blade_index in 0..3 {
            let blade = app
                .world_mut()
                .spawn((
                    Transform::from_translation(Vec3::new(blade_index as f32, 0.0, 0.0)),
                    GlobalTransform::default(),
                    GrassBladeInstance {
                        mesh: mesh.clone(),
                        material: material.clone(),
                    },
                ))
                .id();
            app.world_mut().entity_mut(cluster).add_child(blade);
        }

        app.update();

        let batches: Vec<_> = {
            let world = app.world_mut();
            world.query::<&GrassInstanceBatch>().iter(world).collect()
        };

        let total_instances: usize = batches.iter().map(|b| b.instances.len()).sum();
        assert_eq!(total_instances, 3);
        assert!(batches.len() >= 2);
    }

    #[test]
    fn test_blade_config_from_domain_config() {
        let domain = world::GrassBladeConfig {
            length: 1.4,
            width: 0.9,
            tilt: 0.4,
            curve: 0.5,
            color_variation: 0.3,
        };

        let config = BladeConfig::from(&domain);
        assert_eq!(config.length, 1.4);
        assert_eq!(config.width, 0.9);
        assert_eq!(config.tilt, 0.4);
        assert_eq!(config.curve, 0.5);
        assert_eq!(config.color_variation, 0.3);
    }

    #[test]
    fn test_grass_color_scheme_variation_can_change_color() {
        let scheme = GrassColorScheme {
            base_color: Color::srgb(0.2, 0.5, 0.1),
            tip_color: Color::srgb(0.3, 0.7, 0.2),
            variation: 0.5,
        };

        let mut rng = rand::rng();
        let first = scheme.sample_blade_color(&mut rng);
        let mut changed = false;

        for _ in 0..10 {
            let next = scheme.sample_blade_color(&mut rng);
            if (next.to_srgba().red - first.to_srgba().red).abs() > 0.01
                || (next.to_srgba().green - first.to_srgba().green).abs() > 0.01
                || (next.to_srgba().blue - first.to_srgba().blue).abs() > 0.01
            {
                changed = true;
                break;
            }
        }

        assert!(changed, "Expected color variation to change at least once");
    }

    #[test]
    fn test_spawn_grass_with_none_density_spawns_no_blades() {
        fn spawn_none_density_system(
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
            settings: Res<GrassQualitySettings>,
        ) {
            let metadata = TileVisualMetadata {
                grass_density: Some(GrassDensity::None),
                ..Default::default()
            };

            spawn_grass(
                &mut commands,
                &mut materials,
                &mut meshes,
                types::Position::new(0, 0),
                1u16,
                Some(&metadata),
                &settings,
            );
        }

        let mut app = App::new();
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.insert_resource(GrassQualitySettings {
            performance_level:
                crate::game::resources::grass_quality_settings::GrassPerformanceLevel::Medium,
        });
        app.add_systems(Update, spawn_none_density_system);

        app.update();

        let (blade_count, cluster_count) = {
            let world = app.world_mut();
            let blades = world
                .query_filtered::<Entity, With<GrassBlade>>()
                .iter(world)
                .count();
            let clusters = world
                .query_filtered::<Entity, With<GrassCluster>>()
                .iter(world)
                .count();
            (blades, clusters)
        };

        assert_eq!(blade_count, 0);
        assert_eq!(cluster_count, 0);
    }

    #[test]
    fn test_blade_config_clamps_out_of_range_values() {
        let domain = world::GrassBladeConfig {
            length: 0.1,
            width: 4.0,
            tilt: -1.0,
            curve: 2.0,
            color_variation: -0.5,
        };

        let config = BladeConfig::from(&domain);
        assert_eq!(config.length, 0.5);
        assert_eq!(config.width, 2.0);
        assert_eq!(config.tilt, 0.0);
        assert_eq!(config.curve, 1.0);
        assert_eq!(config.color_variation, 0.0);
    }

    #[test]
    fn test_spawn_grass_with_density_spawns_blades_and_cluster() {
        fn spawn_low_density_system(
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
            settings: Res<GrassQualitySettings>,
        ) {
            let metadata = TileVisualMetadata {
                grass_density: Some(GrassDensity::Low),
                ..Default::default()
            };

            spawn_grass(
                &mut commands,
                &mut materials,
                &mut meshes,
                types::Position::new(1, 1),
                1u16,
                Some(&metadata),
                &settings,
            );
        }

        let mut app = App::new();
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.insert_resource(GrassQualitySettings {
            performance_level:
                crate::game::resources::grass_quality_settings::GrassPerformanceLevel::Medium,
        });
        app.add_systems(Update, spawn_low_density_system);

        app.update();

        let (blade_count, cluster_count) = {
            let world = app.world_mut();
            let blades = world
                .query_filtered::<Entity, With<GrassBlade>>()
                .iter(world)
                .count();
            let clusters = world
                .query_filtered::<Entity, With<GrassCluster>>()
                .iter(world)
                .count();
            (blades, clusters)
        };

        assert!(blade_count > 0);
        assert!(cluster_count > 0);
    }
}
