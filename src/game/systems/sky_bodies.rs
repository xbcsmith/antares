// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Celestial body and cloud layer spawning and visibility system.
//!
//! Manages sun disc entities, a star-field mesh entity, and a cloud layer mesh
//! entity. Visibility is toggled based on the current map's [`SkyConfig`] and
//! the current [`TimeOfDay`]. Cloud quads drift east-to-west each frame.
//!
//! # Design
//!
//! Core logic is split into pure helper functions ([`sun_azimuths`],
//! [`generate_star_positions`], [`sky_body_visibility_flags`],
//! [`should_spawn_cloud_layer`], [`cloud_alpha`], [`cloud_base_color`],
//! [`wrap_cloud_position`]) that are unit-testable without a Bevy world.
//! Bevy systems ([`manage_sky_bodies_on_map_change`],
//! [`update_sky_body_visibility`], [`animate_clouds`]) wrap them.
//!
//! ## Sun placement
//!
//! For `sun_count = 1` the single sun is placed center-left (`-30°` azimuth).
//! For `sun_count = 2` two suns are placed symmetrically at `±60°`.
//! For `sun_count > 2` suns are distributed evenly over a `120°` arc.
//!
//! ## Star field
//!
//! Stars are scattered over the upper hemisphere using a seeded RNG whose seed
//! is derived from the map ID, guaranteeing deterministic placement.
//!
//! ## Cloud layer
//!
//! Cloud quads are distributed randomly across a horizontal plane at altitude
//! [`MAP_CLOUD_HEIGHT`]. The number of quads scales with `cloud_coverage`.
//! Cloud quads drift along the X axis each frame via [`animate_clouds`], which
//! wraps the position using [`wrap_cloud_position`] when the translation
//! exceeds half the plane width.
//!
//! When `cloud_coverage < MIN_CLOUD_COVERAGE` no cloud entity is spawned.
//!
//! ## Visibility rules
//!
//! | `is_outdoor` | `TimeOfDay`          | Suns  | Stars |
//! |-------------|----------------------|-------|-------|
//! | `false`     | (any)                | false | false |
//! | `true`      | Morning / Afternoon  | true  | false |
//! | `true`      | Evening / Night      | false | true  |
//! | `true`      | Dawn / Dusk          | true  | true  |
//!
//! Clouds are visible whenever `is_outdoor == true` (ambient light darkens
//! them at night automatically).
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sky_system_implementation_plan.md` Phase 4 and Phase 5.

use crate::domain::types::{MapId, TimeOfDay};
use crate::domain::world::{Map, SkyConfig};
use crate::game::components::sky::{CloudLayerMarker, StarFieldMarker, SunMarker};
use crate::game::resources::GlobalState;
use bevy::color::LinearRgba;
use bevy::mesh::Indices;
use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

// ===== Constants =====

/// Distance from world origin at which sun disc entities are placed (world units).
pub const SUN_DISTANCE: f32 = 500.0;

/// Elevation angle above the horizon for sun entities (radians).
///
/// 45° keeps the suns clearly in the upper portion of the field of view.
pub const SUN_ELEVATION_ANGLE_RADIANS: f32 = std::f32::consts::FRAC_PI_4;

/// Base radius of a sun disc at `sun_size = 1.0` (world units).
pub const SUN_BASE_RADIUS: f32 = SUN_DISTANCE * 0.1;

/// Radius of the hemisphere used for the star field (world units).
pub const STAR_FIELD_RADIUS: f32 = 480.0;

/// Half-size of each star triangle on the hemisphere surface (world units).
pub const STAR_POINT_SIZE: f32 = 0.8;

/// Altitude (Y position) at which the cloud layer plane is placed (world units).
///
/// Chosen to be clearly above the tallest in-scene geometry (~20 units) while
/// remaining within the camera frustum's far plane.
pub const MAP_CLOUD_HEIGHT: f32 = 40.0;

/// Total width and depth of the cloud plane mesh (world units).
///
/// The cloud layer entity's `translation.x` is wrapped within
/// `±CLOUD_PLANE_WIDTH / 2` by [`wrap_cloud_position`].
pub const CLOUD_PLANE_WIDTH: f32 = 200.0;

/// World-unit side length of each individual cloud quad.
pub const CLOUD_QUAD_SIZE: f32 = 20.0;

/// Maximum number of cloud quads at `cloud_coverage = 1.0`.
pub const MAX_CLOUD_QUADS: u32 = 50;

/// Minimum `cloud_coverage` value that triggers cloud entity spawning.
///
/// Values below this threshold are treated as "no clouds".
pub const MIN_CLOUD_COVERAGE: f32 = 0.05;

// ===== Resource =====

/// Bevy resource tracking spawned sky body entity IDs for safe despawn.
///
/// # Examples
///
/// ```
/// use antares::game::systems::sky_bodies::SkyBodyState;
///
/// let state = SkyBodyState::default();
/// assert!(state.sun_entities.is_empty());
/// assert!(state.star_entity.is_none());
/// assert!(state.cloud_entity.is_none());
/// ```
#[derive(Resource, Debug, Default)]
pub struct SkyBodyState {
    /// Entity IDs for all spawned sun disc entities.
    pub sun_entities: Vec<Entity>,
    /// Entity ID for the star-field mesh entity, if spawned.
    pub star_entity: Option<Entity>,
    /// Entity ID for the cloud layer mesh entity, if spawned.
    pub cloud_entity: Option<Entity>,
}

// ===== Plugin =====

/// Plugin that spawns sun, star-field, and cloud-layer entities and updates
/// their visibility based on the current map's sky configuration and time of
/// day.
///
/// Register this plugin **after**
/// [`SkyPlugin`](crate::game::systems::sky::SkyPlugin) so that the sky
/// background colour and celestial bodies are driven by the same time state
/// in the same frame.
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::sky_bodies::SkyBodyPlugin;
///
/// App::new().add_plugins(SkyBodyPlugin).run();
/// ```
pub struct SkyBodyPlugin;

impl Plugin for SkyBodyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SkyBodyState>();
        app.add_systems(
            Update,
            (
                manage_sky_bodies_on_map_change,
                update_sky_body_visibility.after(manage_sky_bodies_on_map_change),
                animate_clouds,
            ),
        );
    }
}

// ===== Pure helper functions =====

/// Returns azimuth angles (in radians) for `sun_count` suns.
///
/// Distribution rules:
/// - `0` → empty (no suns)
/// - `1` → `[-30°]` center-left
/// - `2` → `[-60°, +60°]` symmetric
/// - `n > 2` → evenly distributed over a `120°` arc centred at `0°`
///
/// # Examples
///
/// ```
/// use antares::game::systems::sky_bodies::sun_azimuths;
///
/// let zero = sun_azimuths(0);
/// assert!(zero.is_empty());
///
/// let one = sun_azimuths(1);
/// assert_eq!(one.len(), 1);
/// let expected = -30.0_f32.to_radians();
/// assert!((one[0] - expected).abs() < 1e-5);
///
/// let two = sun_azimuths(2);
/// assert_eq!(two.len(), 2);
/// assert!((two[0] + two[1]).abs() < 1e-5);
/// ```
pub fn sun_azimuths(sun_count: u8) -> Vec<f32> {
    if sun_count == 0 {
        return Vec::new();
    }
    match sun_count {
        1 => vec![(-30_f32).to_radians()],
        2 => vec![(-60_f32).to_radians(), (60_f32).to_radians()],
        n => {
            let arc_start = (-60_f32).to_radians();
            let arc_end = (60_f32).to_radians();
            let step = (arc_end - arc_start) / (n as f32 - 1.0);
            (0..n).map(|i| arc_start + step * i as f32).collect()
        }
    }
}

/// Returns 3D world-space positions for `sun_count` suns placed in the upper
/// hemisphere, together with their disc radius.
///
/// Each position is at [`SUN_DISTANCE`] from the origin elevated by
/// [`SUN_ELEVATION_ANGLE_RADIANS`].  Returns an empty `Vec` when
/// `sun_count == 0`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::sky_bodies::sun_world_positions;
///
/// let empty = sun_world_positions(0, 1.0);
/// assert!(empty.is_empty());
///
/// let one = sun_world_positions(1, 1.0);
/// assert_eq!(one.len(), 1);
/// ```
pub fn sun_world_positions(sun_count: u8, sun_size: f32) -> Vec<(Vec3, f32)> {
    sun_azimuths(sun_count)
        .iter()
        .map(|&az| {
            let elev = SUN_ELEVATION_ANGLE_RADIANS;
            let pos = Vec3::new(
                SUN_DISTANCE * az.sin() * elev.cos(),
                SUN_DISTANCE * elev.sin(),
                -SUN_DISTANCE * az.cos() * elev.cos(),
            );
            let radius = SUN_BASE_RADIUS * sun_size;
            (pos, radius)
        })
        .collect()
}

/// Generates star positions as points on the upper hemisphere scaled to
/// [`STAR_FIELD_RADIUS`].
///
/// Uses a seeded RNG derived from `seed` for determinism.  Returns an empty
/// `Vec` when `star_count == 0`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::sky_bodies::generate_star_positions;
///
/// let empty = generate_star_positions(0, 0);
/// assert!(empty.is_empty());
///
/// let stars = generate_star_positions(100, 42);
/// assert_eq!(stars.len(), 100);
/// for pos in &stars {
///     assert!(pos.y >= 0.0, "star must be in upper hemisphere");
/// }
/// ```
pub fn generate_star_positions(star_count: u32, seed: u64) -> Vec<Vec3> {
    if star_count == 0 {
        return Vec::new();
    }
    let mut rng = StdRng::seed_from_u64(seed);
    (0..star_count)
        .map(|_| {
            let azimuth = rng.random_range(0.0_f32..std::f32::consts::TAU);
            let elevation = rng.random_range(0.0_f32..std::f32::consts::FRAC_PI_2);
            Vec3::new(
                elevation.cos() * azimuth.sin(),
                elevation.sin(),
                -elevation.cos() * azimuth.cos(),
            ) * STAR_FIELD_RADIUS
        })
        .collect()
}

/// Returns `(suns_visible, stars_visible)` for the given outdoor flag and
/// time of day.
///
/// Pure function — testable without a Bevy world.
///
/// # Examples
///
/// ```
/// use antares::domain::types::TimeOfDay;
/// use antares::game::systems::sky_bodies::sky_body_visibility_flags;
///
/// assert_eq!(sky_body_visibility_flags(false, TimeOfDay::Afternoon), (false, false));
/// assert_eq!(sky_body_visibility_flags(true, TimeOfDay::Night),      (false, true));
/// assert_eq!(sky_body_visibility_flags(true, TimeOfDay::Morning),    (true, false));
/// assert_eq!(sky_body_visibility_flags(true, TimeOfDay::Dawn),       (true, true));
/// ```
pub fn sky_body_visibility_flags(is_outdoor: bool, tod: TimeOfDay) -> (bool, bool) {
    if !is_outdoor {
        return (false, false);
    }
    match tod {
        TimeOfDay::Night | TimeOfDay::Evening => (false, true),
        TimeOfDay::Morning | TimeOfDay::Afternoon => (true, false),
        TimeOfDay::Dawn | TimeOfDay::Dusk => (true, true),
    }
}

/// Returns `true` when a cloud layer should be spawned for the given coverage.
///
/// Coverage values below [`MIN_CLOUD_COVERAGE`] are treated as "no clouds".
///
/// # Examples
///
/// ```
/// use antares::game::systems::sky_bodies::{should_spawn_cloud_layer, MIN_CLOUD_COVERAGE};
///
/// assert!(!should_spawn_cloud_layer(0.0));
/// assert!(!should_spawn_cloud_layer(MIN_CLOUD_COVERAGE - 0.001));
/// assert!(should_spawn_cloud_layer(MIN_CLOUD_COVERAGE));
/// assert!(should_spawn_cloud_layer(1.0));
/// ```
pub fn should_spawn_cloud_layer(coverage: f32) -> bool {
    coverage >= MIN_CLOUD_COVERAGE
}

/// Computes cloud layer opacity from `density` and `coverage`.
///
/// Result is clamped to `[0.0, 1.0]`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::sky_bodies::cloud_alpha;
///
/// assert_eq!(cloud_alpha(0.0, 0.8), 0.0);
/// assert!((cloud_alpha(0.5, 0.8) - 0.4).abs() < 1e-5);
/// assert_eq!(cloud_alpha(1.0, 1.0), 1.0);
/// ```
pub fn cloud_alpha(density: f32, coverage: f32) -> f32 {
    (density * coverage).clamp(0.0, 1.0)
}

/// Returns the RGBA base colour for the cloud material.
///
/// RGB channels come from `cloud_color`; alpha is `density * coverage`
/// (clamped to `[0.0, 1.0]`).
///
/// # Examples
///
/// ```
/// use antares::game::systems::sky_bodies::{cloud_alpha, cloud_base_color};
///
/// let color = cloud_base_color([0.9, 0.9, 0.9, 0.8], 0.5, 0.8);
/// assert!((color[0] - 0.9).abs() < 1e-5);
/// assert!((color[3] - cloud_alpha(0.5, 0.8)).abs() < 1e-5);
/// ```
pub fn cloud_base_color(cloud_color: [f32; 4], density: f32, coverage: f32) -> [f32; 4] {
    let alpha = cloud_alpha(density, coverage);
    [cloud_color[0], cloud_color[1], cloud_color[2], alpha]
}

/// Wraps the cloud layer X translation when it exceeds `±half_width`.
///
/// When `x > half_width`, subtracts `2 * half_width` (wraps to negative side).
/// When `x < -half_width`, adds `2 * half_width` (wraps to positive side).
/// Otherwise returns `x` unchanged.
///
/// # Examples
///
/// ```
/// use antares::game::systems::sky_bodies::{wrap_cloud_position, CLOUD_PLANE_WIDTH};
///
/// let half = CLOUD_PLANE_WIDTH / 2.0;
/// assert!((wrap_cloud_position(0.0, half) - 0.0).abs() < 1e-5);
/// let wrapped = wrap_cloud_position(half + 5.0, half);
/// assert!((wrapped - (-half + 5.0)).abs() < 1e-5);
/// ```
pub fn wrap_cloud_position(x: f32, half_width: f32) -> f32 {
    if x > half_width {
        x - 2.0 * half_width
    } else if x < -half_width {
        x + 2.0 * half_width
    } else {
        x
    }
}

// ===== Mesh building =====

/// Builds a [`Mesh`] representing the star field.
///
/// Each star is a tiny equilateral triangle on the surface of the hemisphere.
/// Vertex colours are white with alpha driven by `density` (clamped to
/// `[0.2, 1.0]`).
///
/// Returns an empty [`Mesh`] with zero vertices when `positions` is empty.
pub fn build_star_mesh(positions: &[Vec3], density: f32) -> Mesh {
    if positions.is_empty() {
        return Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::all(),
        );
    }
    let alpha = density.clamp(0.2, 1.0);
    let half = STAR_POINT_SIZE;

    let mut vertices: Vec<[f32; 3]> = Vec::with_capacity(positions.len() * 3);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(positions.len() * 3);
    let mut indices: Vec<u32> = Vec::with_capacity(positions.len() * 3);

    for (i, &pos) in positions.iter().enumerate() {
        let outward = pos.normalize_or_zero();
        let right = if outward.x.abs() < 0.9 {
            outward.cross(Vec3::X).normalize_or_zero()
        } else {
            outward.cross(Vec3::Y).normalize_or_zero()
        };
        let up = outward.cross(right);

        let v0 = pos + up * half;
        let v1 = pos - up * half * 0.5 + right * (half * 0.866);
        let v2 = pos - up * half * 0.5 - right * (half * 0.866);

        vertices.push(v0.into());
        vertices.push(v1.into());
        vertices.push(v2.into());
        for _ in 0..3 {
            colors.push([1.0, 1.0, 1.0, alpha]);
        }
        let base = (i * 3) as u32;
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::all(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Builds a cloud layer [`Mesh`] as a set of flat quads distributed randomly
/// across a horizontal plane.
///
/// Quads are distributed within a `CLOUD_PLANE_WIDTH × CLOUD_PLANE_WIDTH` area
/// using a seeded RNG. The number of quads scales with
/// `coverage * MAX_CLOUD_QUADS`.
///
/// Returns an empty mesh when [`should_spawn_cloud_layer`] returns `false`.
pub fn build_cloud_mesh(coverage: f32, seed: u64) -> Mesh {
    if !should_spawn_cloud_layer(coverage) {
        return Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::all(),
        );
    }
    let num_quads = ((coverage * MAX_CLOUD_QUADS as f32) as u32).max(1);
    let mut rng = StdRng::seed_from_u64(seed);
    let half = CLOUD_PLANE_WIDTH / 2.0;
    let q_half = CLOUD_QUAD_SIZE / 2.0;
    let cap = num_quads as usize;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(cap * 4);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(cap * 4);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(cap * 4);
    let mut indices: Vec<u32> = Vec::with_capacity(cap * 6);

    for i in 0..num_quads {
        let cx = rng.random_range(-half..half);
        let cz = rng.random_range(-half..half);
        let base = i * 4;

        positions.extend_from_slice(&[
            [cx - q_half, 0.0, cz - q_half],
            [cx + q_half, 0.0, cz - q_half],
            [cx + q_half, 0.0, cz + q_half],
            [cx - q_half, 0.0, cz + q_half],
        ]);
        uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        normals.extend_from_slice(&[
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::all(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

// ===== Spawn / despawn helpers =====

/// Spawns sun disc, star-field, and cloud-layer entities for `map` and
/// records their entity IDs in `state`.
///
/// Called by [`manage_sky_bodies_on_map_change`] whenever the current map
/// changes.  Free function (not a system) to simplify testing.
pub fn spawn_sky_bodies(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    state: &mut SkyBodyState,
    map: &Map,
) {
    let default_sky = SkyConfig::default();
    let sky = map.sky.as_ref().unwrap_or(&default_sky);

    // ── Sun discs ──────────────────────────────────────────────────────────
    for (pos, radius) in sun_world_positions(sky.sun_count, sky.sun_size) {
        let mesh_handle = meshes.add(Sphere::new(radius));
        let [r, g, b, a] = sky.sun_color;
        let mat = materials.add(StandardMaterial {
            base_color: Color::srgba(r, g, b, a),
            emissive: LinearRgba::new(r * 2.0, g * 2.0, b * 2.0, 1.0),
            unlit: true,
            ..Default::default()
        });
        let entity = commands
            .spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(mat),
                Transform::from_translation(pos),
                GlobalTransform::default(),
                Visibility::default(),
                SunMarker,
            ))
            .id();
        state.sun_entities.push(entity);
    }

    // ── Star field ─────────────────────────────────────────────────────────
    let seed = u64::from(map.id);
    let star_positions = generate_star_positions(sky.star_count, seed);
    if !star_positions.is_empty() {
        let star_mesh = build_star_mesh(&star_positions, sky.star_density);
        let mesh_handle = meshes.add(star_mesh);
        let mat = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..Default::default()
        });
        let entity = commands
            .spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(mat),
                Transform::IDENTITY,
                GlobalTransform::default(),
                Visibility::default(),
                StarFieldMarker,
            ))
            .id();
        state.star_entity = Some(entity);
    }

    // ── Cloud layer ────────────────────────────────────────────────────────
    spawn_cloud_layer(commands, meshes, materials, state, map);
}

/// Despawns all sky body entities tracked in `state` and clears the lists.
///
/// Called by [`manage_sky_bodies_on_map_change`] before re-spawning for the
/// new map.
pub fn despawn_sky_bodies(commands: &mut Commands, state: &mut SkyBodyState) {
    for entity in state.sun_entities.drain(..) {
        commands.entity(entity).despawn();
    }
    if let Some(entity) = state.star_entity.take() {
        commands.entity(entity).despawn();
    }
    despawn_cloud_layer(commands, state);
}

/// Spawns the cloud layer entity for `map` and records its entity ID in `state`.
///
/// No entity is spawned when:
/// - `map.is_outdoor == false`, or
/// - [`should_spawn_cloud_layer`] returns `false` for `cloud_coverage`.
pub fn spawn_cloud_layer(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    state: &mut SkyBodyState,
    map: &Map,
) {
    if !map.is_outdoor {
        return;
    }
    let default_sky = SkyConfig::default();
    let sky = map.sky.as_ref().unwrap_or(&default_sky);

    if !should_spawn_cloud_layer(sky.cloud_coverage) {
        return;
    }

    // Use a different seed offset from stars so cloud positions are independent.
    let seed = u64::from(map.id).wrapping_add(0x9E37_79B9);
    let cloud_mesh = build_cloud_mesh(sky.cloud_coverage, seed);
    let mesh_handle = meshes.add(cloud_mesh);

    let [r, g, b, _] = sky.cloud_color;
    let alpha = cloud_alpha(sky.cloud_density, sky.cloud_coverage);
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgba(r, g, b, alpha),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..Default::default()
    });

    let entity = commands
        .spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(mat),
            Transform::from_xyz(0.0, MAP_CLOUD_HEIGHT, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
            CloudLayerMarker {
                cloud_speed: sky.cloud_speed,
                plane_half_width: CLOUD_PLANE_WIDTH / 2.0,
            },
        ))
        .id();
    state.cloud_entity = Some(entity);
}

/// Despawns the cloud layer entity tracked in `state`.
pub fn despawn_cloud_layer(commands: &mut Commands, state: &mut SkyBodyState) {
    if let Some(entity) = state.cloud_entity.take() {
        commands.entity(entity).despawn();
    }
}

// ===== Bevy systems =====

/// System: detects map changes and re-spawns sky bodies when the current map
/// changes.
///
/// Uses a [`Local<Option<MapId>>`] to track the previous map ID.  On the
/// first frame (when `None`) sky bodies are spawned for the starting map.
pub fn manage_sky_bodies_on_map_change(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<SkyBodyState>,
    global_state: Res<GlobalState>,
    mut last_map_id: Local<Option<MapId>>,
) {
    let current_map_id = global_state.0.world.current_map;
    if *last_map_id == Some(current_map_id) {
        return;
    }
    despawn_sky_bodies(&mut commands, &mut state);
    if let Some(map) = global_state.0.world.get_current_map() {
        spawn_sky_bodies(
            &mut commands,
            meshes.as_mut(),
            materials.as_mut(),
            state.as_mut(),
            map,
        );
    }
    *last_map_id = Some(current_map_id);
}

/// System: updates [`Visibility`] on all [`SunMarker`] and [`StarFieldMarker`]
/// entities based on the current map's outdoor flag and time of day.
pub fn update_sky_body_visibility(
    global_state: Res<GlobalState>,
    mut sun_query: Query<&mut Visibility, With<SunMarker>>,
    mut star_query: Query<&mut Visibility, (With<StarFieldMarker>, Without<SunMarker>)>,
) {
    let game_state = &global_state.0;
    let tod = game_state.time_of_day();
    let is_outdoor = game_state
        .world
        .get_current_map()
        .map(|m| m.is_outdoor)
        .unwrap_or(false);

    let (suns_visible, stars_visible) = sky_body_visibility_flags(is_outdoor, tod);

    let sun_vis = if suns_visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    let star_vis = if stars_visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for mut vis in sun_query.iter_mut() {
        *vis = sun_vis;
    }
    for mut vis in star_query.iter_mut() {
        *vis = star_vis;
    }
}

/// System: animates the cloud layer by translating it along the X axis.
///
/// Each frame, `cloud_speed * delta_seconds` is added to the cloud entity's
/// `transform.translation.x`. When the position exceeds the plane's half-width,
/// it is wrapped to the opposite side via [`wrap_cloud_position`].
pub fn animate_clouds(time: Res<Time>, mut query: Query<(&mut Transform, &CloudLayerMarker)>) {
    for (mut transform, marker) in query.iter_mut() {
        transform.translation.x += marker.cloud_speed * time.delta_secs();
        transform.translation.x =
            wrap_cloud_position(transform.translation.x, marker.plane_half_width);
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    // ── Sun position tests ────────────────────────────────────────────────

    #[test]
    fn test_sun_positions_one_sun() {
        let azimuths = sun_azimuths(1);
        assert_eq!(
            azimuths.len(),
            1,
            "single sun must produce exactly 1 azimuth"
        );
        let expected = (-30_f32).to_radians();
        assert!(
            (azimuths[0] - expected).abs() < 1e-5,
            "single sun azimuth must be -30° (center-left), got {:.2}°",
            azimuths[0].to_degrees()
        );
    }

    #[test]
    fn test_sun_positions_two_suns() {
        let azimuths = sun_azimuths(2);
        assert_eq!(
            azimuths.len(),
            2,
            "two suns must produce exactly 2 azimuths"
        );
        assert!(
            (azimuths[0] + azimuths[1]).abs() < 1e-5,
            "two-sun azimuths must be symmetric about 0°; got {:.2}° and {:.2}°",
            azimuths[0].to_degrees(),
            azimuths[1].to_degrees()
        );
        assert!(
            azimuths[0].abs() > 1e-3,
            "first sun must not be at azimuth 0"
        );
        assert!(
            azimuths[1].abs() > 1e-3,
            "second sun must not be at azimuth 0"
        );
    }

    #[test]
    fn test_sun_count_zero_spawns_nothing() {
        let positions = sun_world_positions(0, 1.0);
        assert!(
            positions.is_empty(),
            "sun_count = 0 must produce no world positions"
        );
        let azimuths = sun_azimuths(0);
        assert!(
            azimuths.is_empty(),
            "sun_count = 0 must produce no azimuths"
        );
    }

    // ── Star field tests ──────────────────────────────────────────────────

    #[test]
    fn test_star_count_zero_spawns_empty_field() {
        let positions = generate_star_positions(0, 0);
        assert!(
            positions.is_empty(),
            "star_count = 0 must produce an empty positions list"
        );
        let mesh = build_star_mesh(&positions, 0.5);
        let vertex_count = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .map(|a| a.len())
            .unwrap_or(0);
        assert_eq!(
            vertex_count, 0,
            "star field built from 0 positions must have 0 vertices"
        );
    }

    // ── Visibility logic tests ────────────────────────────────────────────

    #[test]
    fn test_sky_body_visibility_night_shows_stars() {
        // Night: stars visible, suns hidden
        let (suns, stars) = sky_body_visibility_flags(true, TimeOfDay::Night);
        assert!(!suns, "Night outdoor: suns must be hidden");
        assert!(stars, "Night outdoor: stars must be visible");

        // Evening: stars visible, suns hidden
        let (suns, stars) = sky_body_visibility_flags(true, TimeOfDay::Evening);
        assert!(!suns, "Evening outdoor: suns must be hidden");
        assert!(stars, "Evening outdoor: stars must be visible");

        // Indoor night: neither visible
        let (suns, stars) = sky_body_visibility_flags(false, TimeOfDay::Night);
        assert!(!suns, "Indoor night: suns must be hidden");
        assert!(!stars, "Indoor night: stars must be hidden");
    }

    #[test]
    fn test_sky_body_visibility_afternoon_shows_suns() {
        // Afternoon: suns visible, stars hidden
        let (suns, stars) = sky_body_visibility_flags(true, TimeOfDay::Afternoon);
        assert!(suns, "Afternoon outdoor: suns must be visible");
        assert!(!stars, "Afternoon outdoor: stars must be hidden");

        // Morning: suns visible, stars hidden
        let (suns, stars) = sky_body_visibility_flags(true, TimeOfDay::Morning);
        assert!(suns, "Morning outdoor: suns must be visible");
        assert!(!stars, "Morning outdoor: stars must be hidden");

        // Dawn: both visible (transition)
        let (suns, stars) = sky_body_visibility_flags(true, TimeOfDay::Dawn);
        assert!(suns, "Dawn outdoor: suns must be visible");
        assert!(stars, "Dawn outdoor: stars must be visible");

        // Dusk: both visible (transition)
        let (suns, stars) = sky_body_visibility_flags(true, TimeOfDay::Dusk);
        assert!(suns, "Dusk outdoor: suns must be visible");
        assert!(stars, "Dusk outdoor: stars must be visible");
    }

    // ── Cloud layer tests ─────────────────────────────────────────────────

    #[test]
    fn test_cloud_coverage_zero_skips_spawn() {
        assert!(
            !should_spawn_cloud_layer(0.0),
            "cloud_coverage = 0.0 must not spawn"
        );
        assert!(
            !should_spawn_cloud_layer(MIN_CLOUD_COVERAGE - 0.001),
            "cloud_coverage just below MIN_CLOUD_COVERAGE must not spawn"
        );
        assert!(
            should_spawn_cloud_layer(MIN_CLOUD_COVERAGE),
            "cloud_coverage = MIN_CLOUD_COVERAGE must spawn"
        );
        assert!(
            should_spawn_cloud_layer(0.5),
            "cloud_coverage = 0.5 must spawn"
        );
        assert!(
            should_spawn_cloud_layer(1.0),
            "cloud_coverage = 1.0 must spawn"
        );
    }

    #[test]
    fn test_cloud_density_affects_opacity() {
        assert_eq!(cloud_alpha(0.0, 0.8), 0.0, "density = 0.0 → alpha = 0.0");
        assert_eq!(cloud_alpha(0.5, 0.0), 0.0, "coverage = 0.0 → alpha = 0.0");
        let alpha = cloud_alpha(0.5, 0.8);
        assert!(
            (alpha - 0.4).abs() < 1e-5,
            "0.5 * 0.8 must equal 0.4, got {alpha}"
        );
        assert_eq!(
            cloud_alpha(1.0, 1.0),
            1.0,
            "density = 1.0, coverage = 1.0 → alpha = 1.0"
        );
        assert_eq!(
            cloud_alpha(2.0, 2.0),
            1.0,
            "product > 1.0 must be clamped to 1.0"
        );
    }

    #[test]
    fn test_animate_clouds_wraps_position() {
        let half = CLOUD_PLANE_WIDTH / 2.0;

        // Within range — no wrap.
        assert!(
            (wrap_cloud_position(0.0, half) - 0.0).abs() < 1e-5,
            "x = 0 must remain 0"
        );
        assert!(
            (wrap_cloud_position(half - 0.01, half) - (half - 0.01)).abs() < 1e-5,
            "x just below half_width must not wrap"
        );

        // Exceeds positive half_width → wraps to negative side.
        let result = wrap_cloud_position(half + 5.0, half);
        let expected = -half + 5.0;
        assert!(
            (result - expected).abs() < 1e-5,
            "x > half_width: got {result}, expected {expected}"
        );

        // Below negative half_width → wraps to positive side.
        let result2 = wrap_cloud_position(-half - 5.0, half);
        let expected2 = half - 5.0;
        assert!(
            (result2 - expected2).abs() < 1e-5,
            "x < -half_width: got {result2}, expected {expected2}"
        );
    }

    #[test]
    fn test_cloud_color_applied_to_material() {
        let sky_color = [0.9_f32, 0.9, 0.9, 0.8];

        // RGB channels are preserved; alpha = density * coverage.
        let color = cloud_base_color(sky_color, 0.5, 0.8);
        assert!((color[0] - 0.9).abs() < 1e-5, "R must match cloud_color[0]");
        assert!((color[1] - 0.9).abs() < 1e-5, "G must match cloud_color[1]");
        assert!((color[2] - 0.9).abs() < 1e-5, "B must match cloud_color[2]");
        let expected_alpha = cloud_alpha(0.5, 0.8);
        assert!(
            (color[3] - expected_alpha).abs() < 1e-5,
            "alpha must equal density * coverage = {expected_alpha}"
        );

        // Default SkyConfig: cloud_coverage=0.3, cloud_density=0.5 → alpha=0.15.
        let default_sky = SkyConfig::default();
        let default_color = cloud_base_color(
            default_sky.cloud_color,
            default_sky.cloud_density,
            default_sky.cloud_coverage,
        );
        let expected_default_alpha =
            cloud_alpha(default_sky.cloud_density, default_sky.cloud_coverage);
        assert!(
            (default_color[3] - expected_default_alpha).abs() < 1e-5,
            "default sky: alpha = 0.5 * 0.3 = 0.15, got {}",
            default_color[3]
        );
    }
}
