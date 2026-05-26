// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Celestial body spawning and visibility system.
//!
//! Manages sun disc entities and a star-field mesh entity whose visibility is
//! toggled based on the current map's [`SkyConfig`] and the current
//! [`TimeOfDay`].
//!
//! # Design
//!
//! Core logic is split into pure helper functions ([`sun_azimuths`],
//! [`generate_star_positions`], [`sky_body_visibility_flags`]) that are
//! unit-testable without a Bevy world.  Bevy systems
//! ([`manage_sky_bodies_on_map_change`], [`update_sky_body_visibility`]) wrap
//! them.
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
//! ## Visibility rules
//!
//! | `is_outdoor` | `TimeOfDay`          | Suns  | Stars |
//! |-------------|----------------------|-------|-------|
//! | `false`     | (any)                | false | false |
//! | `true`      | Morning / Afternoon  | true  | false |
//! | `true`      | Evening / Night      | false | true  |
//! | `true`      | Dawn / Dusk          | true  | true  |
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sky_system_implementation_plan.md` Phase 4.

use crate::domain::types::{MapId, TimeOfDay};
use crate::domain::world::{Map, SkyConfig};
use crate::game::components::sky::{StarFieldMarker, SunMarker};
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
/// ```
#[derive(Resource, Debug, Default)]
pub struct SkyBodyState {
    /// Entity IDs for all spawned sun disc entities.
    pub sun_entities: Vec<Entity>,
    /// Entity ID for the star-field mesh entity, if spawned.
    pub star_entity: Option<Entity>,
}

// ===== Plugin =====

/// Plugin that spawns sun and star-field entities and updates their visibility
/// based on the current map's sky configuration and time of day.
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

// ===== Spawn / despawn helpers =====

/// Spawns sun disc and star-field entities for `map` and records their
/// entity IDs in `state`.
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
}
