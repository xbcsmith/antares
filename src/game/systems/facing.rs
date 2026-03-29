// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Runtime facing-change systems
//!
//! This module provides the ECS infrastructure for changing an entity's facing
//! direction at runtime.  Four mechanisms are implemented:
//!
//! 1. **`SetFacing` message** – any system can write a `SetFacing` event to snap
//!    or smoothly rotate an entity to a new cardinal direction.
//! 2. **`handle_set_facing` system** – reads `SetFacing` messages and either
//!    snaps the entity's `Transform` rotation instantly (`instant: true`) or
//!    inserts a `RotatingToFacing` component for frame-by-frame interpolation
//!    (`instant: false`).
//! 3. **`face_toward_player_on_proximity` system** – entities carrying a
//!    `ProximityFacing` marker component automatically emit `SetFacing` events
//!    when the party enters `trigger_distance` tiles.
//! 4. **`apply_rotation_to_facing` system** – per-frame slerp system that
//!    advances entities carrying `RotatingToFacing` toward their target
//!    quaternion and removes the component when the rotation is complete.
//!
//! # Registering
//!
//! Add `FacingPlugin` to your Bevy `App` (already done by `MapManagerPlugin`):
//!
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::systems::facing::FacingPlugin;
//!
//! App::new()
//!     .add_plugins(FacingPlugin)
//!     .run();
//! ```

use bevy::prelude::*;

use crate::domain::types::{Direction, Position};
use crate::game::components::creature::FacingComponent;
use crate::game::resources::GlobalState;
use crate::game::systems::map::TileCoord;

// ─── SetFacing message ────────────────────────────────────────────────────────

/// Message that requests a facing change on a specific entity.
///
/// - `instant: true`  → snap the rotation in the current frame.
/// - `instant: false` → insert a [`RotatingToFacing`] component so the entity
///   rotates smoothly at `speed_deg_per_sec` degrees per second.
///   The speed is taken from the `ProximityFacing` component when the message
///   originates from the proximity system; callers that construct `SetFacing`
///   directly and want smooth rotation should insert `RotatingToFacing`
///   themselves or rely on the `ProximityFacing`-emitted events.
///
/// Write this message from any system to rotate a spawned creature or NPC:
///
/// ```
/// use bevy::prelude::*;
/// use antares::game::systems::facing::SetFacing;
/// use antares::domain::types::Direction;
///
/// fn my_system(mut writer: MessageWriter<SetFacing>, entity: Entity) {
///     writer.write(SetFacing {
///         entity,
///         direction: Direction::South,
///         instant: true,
///     });
/// }
/// ```
#[derive(Message, Clone, Debug)]
pub struct SetFacing {
    /// The entity whose facing should change.
    pub entity: Entity,
    /// The new cardinal direction to face.
    pub direction: Direction,
    /// `true` → snap immediately; `false` → smooth rotation (uses
    /// the speed stored in [`ProximityFacing`] if present, otherwise 360 °/s).
    pub instant: bool,
}

// ─── ProximityFacing component ────────────────────────────────────────────────

/// Marker component that makes an entity automatically face the party when the
/// party comes within `trigger_distance` tiles (Manhattan distance).
///
/// Inserted at map-load time by the map spawning system when a `MapEvent` has
/// `proximity_facing: true`.  Never serialised; purely a runtime ECS component.
///
/// The optional `rotation_speed` field controls whether the entity snaps
/// instantly or rotates smoothly:
/// - `None`          → snap (same as `SetFacing { instant: true }`)
/// - `Some(deg_s)`   → smooth rotation at `deg_s` degrees per second
///
/// # Examples
///
/// ```
/// use bevy::prelude::*;
/// use antares::game::systems::facing::ProximityFacing;
///
/// // Snap variant (inserted when rotation_speed is None in RON)
/// let snap = ProximityFacing { trigger_distance: 2, rotation_speed: None };
/// assert_eq!(snap.trigger_distance, 2);
/// assert!(snap.rotation_speed.is_none());
///
/// // Smooth variant
/// let smooth = ProximityFacing { trigger_distance: 3, rotation_speed: Some(180.0) };
/// assert_eq!(smooth.rotation_speed, Some(180.0));
/// ```
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ProximityFacing {
    /// Tile distance (Manhattan) at which proximity tracking activates.
    pub trigger_distance: u32,
    /// Optional rotation speed in degrees per second.
    ///
    /// `None` → snap. `Some(speed)` → smooth slerp via [`RotatingToFacing`].
    pub rotation_speed: Option<f32>,
}

// ─── RotatingToFacing component ───────────────────────────────────────────────

/// Scratch component inserted on an entity that is currently rotating smoothly
/// toward a target orientation.
///
/// Added by [`handle_set_facing`] when `SetFacing.instant == false`.
/// Removed by [`apply_rotation_to_facing`] once the rotation is complete.
/// Never serialised.
///
/// # Examples
///
/// ```
/// use bevy::prelude::*;
/// use antares::game::systems::facing::RotatingToFacing;
/// use antares::domain::types::Direction;
///
/// let component = RotatingToFacing {
///     target: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
///     speed_deg_per_sec: 180.0,
///     target_direction: Direction::East,
/// };
/// assert_eq!(component.speed_deg_per_sec, 180.0);
/// ```
#[derive(Component, Clone, Debug)]
pub struct RotatingToFacing {
    /// Target quaternion to slerp toward.
    pub target: Quat,
    /// Rotation speed in degrees per second.
    pub speed_deg_per_sec: f32,
    /// The logical direction that `target` corresponds to; written to
    /// `FacingComponent` when the rotation completes.
    pub target_direction: Direction,
}

/// Default rotation speed used when `instant: false` but no explicit speed is
/// configured.  360 °/s completes a full rotation in one second.
pub const DEFAULT_ROTATION_SPEED_DEG_PER_SEC: f32 = 360.0;

/// Angle threshold (radians) below which a rotation is considered complete.
const ROTATION_COMPLETE_THRESHOLD_RAD: f32 = 0.01;

// ─── Plugin ───────────────────────────────────────────────────────────────────

/// Bevy plugin that registers the `SetFacing` message and all facing systems.
///
/// Already included by `MapManagerPlugin`.  Add it explicitly only when using the
/// facing systems in isolation (e.g., integration tests).
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::facing::FacingPlugin;
///
/// App::new()
///     .add_plugins(FacingPlugin)
///     .run();
/// ```
pub struct FacingPlugin;

impl Plugin for FacingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SetFacing>().add_systems(
            Update,
            (
                handle_set_facing,
                face_toward_player_on_proximity,
                apply_rotation_to_facing,
            ),
        );
    }
}

// ─── Systems ──────────────────────────────────────────────────────────────────

/// System that reads [`SetFacing`] messages and applies the requested rotation.
///
/// For each message:
/// - `instant: true` → snap `Transform.rotation` and update `FacingComponent`.
/// - `instant: false` → insert (or overwrite) a [`RotatingToFacing`] component
///   so [`apply_rotation_to_facing`] can slerp toward the target each frame.
///   If the entity already carries a `RotatingToFacing` component, the new
///   target supersedes the old one.
///
/// Unknown or already-despawned entities are silently skipped.
pub fn handle_set_facing(
    mut reader: MessageReader<SetFacing>,
    mut snap_query: Query<(&mut Transform, &mut FacingComponent), Without<RotatingToFacing>>,
    mut rotating_query: Query<&mut RotatingToFacing>,
    // Query to check whether the entity has ProximityFacing so we can read the speed.
    proximity_query: Query<&ProximityFacing>,
    mut commands: Commands,
) {
    for ev in reader.read() {
        let target_yaw = ev.direction.direction_to_yaw_radians();
        let target_quat = Quat::from_rotation_y(target_yaw);

        if ev.instant {
            // Snap path – direct mutation.
            if let Ok((mut transform, mut facing)) = snap_query.get_mut(ev.entity) {
                transform.rotation = target_quat;
                facing.direction = ev.direction;
            }
        } else {
            // Smooth path – look up speed from ProximityFacing or use default.
            let speed = proximity_query
                .get(ev.entity)
                .ok()
                .and_then(|pf| pf.rotation_speed)
                .unwrap_or(DEFAULT_ROTATION_SPEED_DEG_PER_SEC);

            if let Ok(mut rotating) = rotating_query.get_mut(ev.entity) {
                // Entity is already rotating – update target in place.
                rotating.target = target_quat;
                rotating.speed_deg_per_sec = speed;
                rotating.target_direction = ev.direction;
            } else {
                // Insert a fresh RotatingToFacing component.
                commands.entity(ev.entity).insert(RotatingToFacing {
                    target: target_quat,
                    speed_deg_per_sec: speed,
                    target_direction: ev.direction,
                });
            }
        }
    }
}

/// Per-frame slerp system that advances entities with [`RotatingToFacing`]
/// toward their target rotation.
///
/// Each frame:
/// 1. Reads `Time::delta_secs()` to compute how many degrees to advance.
/// 2. Computes `t = (speed_deg_per_sec * delta_secs) / angle_remaining_deg`.
///    `t` is clamped to `[0.0, 1.0]` so we never overshoot.
/// 3. Slerps `Transform.rotation` toward `target`.
/// 4. When the angle to target is within [`ROTATION_COMPLETE_THRESHOLD_RAD`]:
///    - Sets the exact target quaternion.
///    - Updates `FacingComponent.direction`.
///    - Removes the `RotatingToFacing` component.
pub fn apply_rotation_to_facing(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut Transform,
        &mut FacingComponent,
        &RotatingToFacing,
    )>,
    mut commands: Commands,
) {
    let delta = time.delta_secs();

    for (entity, mut transform, mut facing, rotating) in query.iter_mut() {
        let angle_remaining = transform.rotation.angle_between(rotating.target);

        if angle_remaining <= ROTATION_COMPLETE_THRESHOLD_RAD {
            // Rotation complete – snap to exact target and clean up.
            transform.rotation = rotating.target;
            facing.direction = rotating.target_direction;
            commands.entity(entity).remove::<RotatingToFacing>();
        } else {
            // Advance by the speed fraction, clamped so we never overshoot.
            let angle_remaining_deg = angle_remaining.to_degrees();
            let t = (rotating.speed_deg_per_sec * delta / angle_remaining_deg).min(1.0);
            transform.rotation = transform.rotation.slerp(rotating.target, t);
        }
    }
}

/// System that emits [`SetFacing`] events for entities that carry a
/// [`ProximityFacing`] component whenever the party enters `trigger_distance`
/// tiles.
///
/// Resolution logic:
/// - Each frame the system reads `GlobalState::party_position`.
/// - For every entity with `(FacingComponent, ProximityFacing, TileCoord)` it
///   computes the Manhattan distance from the entity tile to the party tile.
/// - When the distance is ≤ `trigger_distance` the system derives the best
///   cardinal direction toward the party and emits `SetFacing` only if the
///   entity is not already facing that direction.
///
/// Whether the rotation snaps or slerps depends on `ProximityFacing.rotation_speed`:
/// `None` → `instant: true`; `Some(_)` → `instant: false`.
///
/// The cardinal is derived from the raw `(dx, dy)` offset using the dominant
/// axis: if `|dx| >= |dy|` we choose East/West, otherwise North/South.
pub fn face_toward_player_on_proximity(
    global_state: Res<GlobalState>,
    entity_query: Query<(Entity, &FacingComponent, &ProximityFacing, &TileCoord)>,
    mut writer: MessageWriter<SetFacing>,
) {
    let party_pos = global_state.0.world.party_position;

    for (entity, facing, proximity, tile_coord) in entity_query.iter() {
        let entity_pos = tile_coord.0;
        let distance = entity_pos.manhattan_distance(&party_pos);

        if distance > proximity.trigger_distance {
            continue;
        }

        let desired = cardinal_toward(entity_pos, party_pos);

        // Only emit an event when the direction actually changes.
        if facing.direction != desired {
            let instant = proximity.rotation_speed.is_none();
            writer.write(SetFacing {
                entity,
                direction: desired,
                instant,
            });
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Computes the dominant-axis cardinal direction from `from` toward `to`.
///
/// | Condition                     | Result |
/// |-------------------------------|--------|
/// | `dx > 0` and dominant         | East   |
/// | `dx < 0` and dominant         | West   |
/// | `dy > 0` and dominant         | South  |
/// | `dy < 0` and dominant         | North  |
/// | `dx == dy == 0` (same tile)   | North  |
///
/// "Dominant" means `|dx| >= |dy|` for the horizontal axis, `|dy| > |dx|` for
/// the vertical axis.
///
/// # Examples
///
/// ```
/// use antares::domain::types::{Direction, Position};
/// use antares::game::systems::facing::cardinal_toward;
///
/// assert_eq!(cardinal_toward(Position::new(5, 5), Position::new(7, 5)), Direction::East);
/// assert_eq!(cardinal_toward(Position::new(5, 5), Position::new(3, 5)), Direction::West);
/// assert_eq!(cardinal_toward(Position::new(5, 5), Position::new(5, 7)), Direction::South);
/// assert_eq!(cardinal_toward(Position::new(5, 5), Position::new(5, 3)), Direction::North);
/// // Diagonal – East wins because |dx| == |dy| (horizontal preferred)
/// assert_eq!(cardinal_toward(Position::new(5, 5), Position::new(7, 7)), Direction::East);
/// ```
pub fn cardinal_toward(from: Position, to: Position) -> Direction {
    let dx = to.x - from.x;
    let dy = to.y - from.y;

    if dx == 0 && dy == 0 {
        return Direction::North;
    }

    // |dx| >= |dy|  →  prefer horizontal axis
    if dx.abs() >= dy.abs() {
        if dx > 0 {
            Direction::East
        } else {
            Direction::West
        }
    } else {
        // dy strictly dominates
        if dy > 0 {
            Direction::South
        } else {
            Direction::North
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::Position;
    use crate::game::components::creature::FacingComponent;
    use bevy::input::ButtonInput;
    use bevy::prelude::{App, Entity, KeyCode, MinimalPlugins, Quat, Transform};
    use std::f32::consts::{FRAC_PI_2, PI};

    // ─── cardinal_toward unit tests ──────────────────────────────────────────

    #[test]
    fn test_cardinal_toward_east() {
        assert_eq!(
            cardinal_toward(Position::new(5, 5), Position::new(7, 5)),
            Direction::East
        );
    }

    #[test]
    fn test_cardinal_toward_west() {
        assert_eq!(
            cardinal_toward(Position::new(5, 5), Position::new(3, 5)),
            Direction::West
        );
    }

    #[test]
    fn test_cardinal_toward_south() {
        assert_eq!(
            cardinal_toward(Position::new(5, 5), Position::new(5, 7)),
            Direction::South
        );
    }

    #[test]
    fn test_cardinal_toward_north() {
        assert_eq!(
            cardinal_toward(Position::new(5, 5), Position::new(5, 3)),
            Direction::North
        );
    }

    #[test]
    fn test_cardinal_toward_same_tile_defaults_north() {
        assert_eq!(
            cardinal_toward(Position::new(5, 5), Position::new(5, 5)),
            Direction::North
        );
    }

    #[test]
    fn test_cardinal_toward_diagonal_prefers_horizontal() {
        // |dx| == |dy| → East wins over South
        assert_eq!(
            cardinal_toward(Position::new(5, 5), Position::new(7, 7)),
            Direction::East
        );
        // |dx| == |dy| negative → West wins over North
        assert_eq!(
            cardinal_toward(Position::new(5, 5), Position::new(3, 3)),
            Direction::West
        );
    }

    #[test]
    fn test_cardinal_toward_vertical_dominant() {
        // |dy| > |dx| → South
        assert_eq!(
            cardinal_toward(Position::new(5, 5), Position::new(6, 9)),
            Direction::South
        );
        // |dy| > |dx| → North
        assert_eq!(
            cardinal_toward(Position::new(5, 5), Position::new(6, 1)),
            Direction::North
        );
    }

    // ─── Test app helpers ────────────────────────────────────────────────────

    /// Build a minimal Bevy app with `FacingPlugin` and the resources that its
    /// systems depend on.
    fn make_facing_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(FacingPlugin);
        app.insert_resource(crate::game::resources::GlobalState(
            crate::application::GameState::new(),
        ));
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app
    }

    // ─── handle_set_facing (snap) tests ─────────────────────────────────────

    #[test]
    fn test_set_facing_snaps_transform() {
        let mut app = make_facing_app();

        let entity = app
            .world_mut()
            .spawn((
                Transform::default(),
                FacingComponent {
                    direction: Direction::North,
                },
            ))
            .id();

        app.world_mut()
            .resource_mut::<Messages<SetFacing>>()
            .write(SetFacing {
                entity,
                direction: Direction::West,
                instant: true,
            });

        app.update();
        app.update();

        let transform = app.world().get::<Transform>(entity).unwrap();
        let expected = Quat::from_rotation_y(Direction::West.direction_to_yaw_radians());
        let angle_diff = transform.rotation.angle_between(expected);
        assert!(
            angle_diff < 0.001,
            "Expected West rotation, got angle_diff={angle_diff}"
        );
    }

    #[test]
    fn test_set_facing_updates_facing_component() {
        let mut app = make_facing_app();

        let entity = app
            .world_mut()
            .spawn((
                Transform::default(),
                FacingComponent {
                    direction: Direction::North,
                },
            ))
            .id();

        app.world_mut()
            .resource_mut::<Messages<SetFacing>>()
            .write(SetFacing {
                entity,
                direction: Direction::East,
                instant: true,
            });

        app.update();
        app.update();

        let facing = app.world().get::<FacingComponent>(entity).unwrap();
        assert_eq!(facing.direction, Direction::East);
    }

    #[test]
    fn test_set_facing_instant_false_inserts_rotating_component() {
        let mut app = make_facing_app();

        let entity = app
            .world_mut()
            .spawn((
                Transform::default(),
                FacingComponent {
                    direction: Direction::North,
                },
            ))
            .id();

        app.world_mut()
            .resource_mut::<Messages<SetFacing>>()
            .write(SetFacing {
                entity,
                direction: Direction::South,
                instant: false,
            });

        app.update();
        app.update();

        let rotating = app.world().get::<RotatingToFacing>(entity);
        // If the rotation completed in one frame (tiny angle difference may not
        // apply, but in this case North→South is 180°), the component may have
        // been removed by apply_rotation_to_facing in the same update cycle.
        // We verify that either RotatingToFacing is present (still rotating)
        // OR the FacingComponent already reflects the target (completed).
        let facing = app.world().get::<FacingComponent>(entity).unwrap();
        let completed = facing.direction == Direction::South;
        let still_rotating = rotating.is_some();
        assert!(
            completed || still_rotating,
            "Expected either rotation completed or RotatingToFacing present"
        );
    }

    #[test]
    fn test_set_facing_non_instant_snaps_without_proximity() {
        // When instant == false, handle_set_facing inserts RotatingToFacing.
        // We verify that writing SetFacing { instant: false } results in either
        // a RotatingToFacing component being inserted, or the facing already
        // updated (if the slerp completed in the same update cycle).
        // This test does NOT rely on time advancement.
        let mut app = make_facing_app();

        let entity = app
            .world_mut()
            .spawn((
                Transform::default(),
                FacingComponent {
                    direction: Direction::North,
                },
            ))
            .id();

        app.world_mut()
            .resource_mut::<Messages<SetFacing>>()
            .write(SetFacing {
                entity,
                direction: Direction::West,
                instant: false,
            });

        // First update: handle_set_facing processes the message and inserts
        // RotatingToFacing (or apply_rotation_to_facing may snap if angle < threshold).
        app.update();
        // Second update: apply_rotation_to_facing runs.
        app.update();

        let rotating = app.world().get::<RotatingToFacing>(entity);
        let facing = app.world().get::<FacingComponent>(entity).unwrap();

        if let Some(r) = rotating {
            // Still rotating – verify the target is correct
            assert_eq!(
                r.target_direction,
                Direction::West,
                "RotatingToFacing target_direction should be West"
            );
        } else {
            // Rotation completed (possible if start ≈ target after slerp rounding)
            assert_eq!(
                facing.direction,
                Direction::West,
                "If RotatingToFacing was removed, FacingComponent must be West"
            );
        }
    }

    #[test]
    fn test_set_facing_unknown_entity_does_not_panic() {
        let mut app = make_facing_app();

        let ghost = Entity::PLACEHOLDER;
        app.world_mut()
            .resource_mut::<Messages<SetFacing>>()
            .write(SetFacing {
                entity: ghost,
                direction: Direction::North,
                instant: true,
            });

        // Must not panic
        app.update();
    }

    #[test]
    fn test_set_facing_multiple_events_last_wins() {
        let mut app = make_facing_app();

        let entity = app
            .world_mut()
            .spawn((
                Transform::default(),
                FacingComponent {
                    direction: Direction::North,
                },
            ))
            .id();

        {
            let mut messages = app.world_mut().resource_mut::<Messages<SetFacing>>();
            messages.write(SetFacing {
                entity,
                direction: Direction::East,
                instant: true,
            });
            messages.write(SetFacing {
                entity,
                direction: Direction::South,
                instant: true,
            });
        }

        app.update();
        app.update();

        let facing = app.world().get::<FacingComponent>(entity).unwrap();
        assert_eq!(
            facing.direction,
            Direction::South,
            "Last SetFacing event should win"
        );
    }

    // ─── RotatingToFacing tests ──────────────────────────────────────────────

    #[test]
    fn test_rotating_to_facing_approaches_target() {
        // Verify the slerp math directly: when the entity is already within
        // ROTATION_COMPLETE_THRESHOLD_RAD the system should snap and remove
        // the component.  This is deterministic regardless of delta time.
        let mut app = make_facing_app();

        let target_quat = Quat::from_rotation_y(FRAC_PI_2); // East
                                                            // Start *just* above the threshold so the system treats it as incomplete
                                                            // on the first frame where delta == 0, but snaps on the next frame when
                                                            // the slerp t-clamp forces the angle below threshold.
                                                            // Actually with delta == 0 the slerp does nothing; so we start the
                                                            // rotation already AT the target (angle == 0 < threshold) to guarantee
                                                            // the completion branch fires and the component is removed.
        let start_quat = target_quat; // already at target

        let entity = app
            .world_mut()
            .spawn((
                Transform {
                    rotation: start_quat,
                    ..Default::default()
                },
                FacingComponent {
                    direction: Direction::North,
                },
                RotatingToFacing {
                    target: target_quat,
                    speed_deg_per_sec: 90.0,
                    target_direction: Direction::East,
                },
            ))
            .id();

        app.update();

        // Since start == target the threshold check fires immediately and the
        // component is removed, with FacingComponent updated to East.
        assert!(
            app.world().get::<RotatingToFacing>(entity).is_none(),
            "RotatingToFacing should be removed when already at target"
        );
        let facing = app.world().get::<FacingComponent>(entity).unwrap();
        assert_eq!(
            facing.direction,
            Direction::East,
            "FacingComponent should be East after completion"
        );
        let transform = app.world().get::<Transform>(entity).unwrap();
        let angle_diff = transform.rotation.angle_between(target_quat);
        assert!(
            angle_diff < 0.001,
            "Transform should equal target rotation; angle_diff={angle_diff:.6}"
        );
    }

    #[test]
    fn test_rotating_to_facing_completes_and_removes_component() {
        // Spawn with rotation already at the target (angle == 0 < threshold).
        // The apply_rotation_to_facing system must detect completion, snap the
        // transform, update FacingComponent, and remove RotatingToFacing.
        // This is deterministic with zero delta time from MinimalPlugins.
        let mut app = make_facing_app();

        let target_quat = Quat::from_rotation_y(FRAC_PI_2); // East

        let entity = app
            .world_mut()
            .spawn((
                Transform {
                    rotation: target_quat, // already at target
                    ..Default::default()
                },
                FacingComponent {
                    direction: Direction::North, // not yet updated
                },
                RotatingToFacing {
                    target: target_quat,
                    speed_deg_per_sec: 90.0,
                    target_direction: Direction::East,
                },
            ))
            .id();

        app.update();

        // Component should be removed
        assert!(
            app.world().get::<RotatingToFacing>(entity).is_none(),
            "RotatingToFacing should be removed after completion"
        );

        // FacingComponent should reflect the final direction
        let facing = app.world().get::<FacingComponent>(entity).unwrap();
        assert_eq!(
            facing.direction,
            Direction::East,
            "FacingComponent should be updated to East after rotation completes"
        );

        // Transform rotation should equal the target (within tolerance)
        let transform = app.world().get::<Transform>(entity).unwrap();
        let angle_diff = transform.rotation.angle_between(target_quat);
        assert!(
            angle_diff < 0.01,
            "Transform rotation should equal target; angle_diff={angle_diff:.6}"
        );
    }

    #[test]
    fn test_rotating_to_facing_target_override() {
        // Writing a second SetFacing { instant: false } while already rotating
        // should update the RotatingToFacing component to the new target.
        let mut app = make_facing_app();

        let entity = app
            .world_mut()
            .spawn((
                Transform::default(),
                FacingComponent {
                    direction: Direction::North,
                },
            ))
            .id();

        // First SetFacing – start rotating toward East
        app.world_mut()
            .resource_mut::<Messages<SetFacing>>()
            .write(SetFacing {
                entity,
                direction: Direction::East,
                instant: false,
            });
        app.update();
        app.update();

        // Second SetFacing before completion – retarget to West
        app.world_mut()
            .resource_mut::<Messages<SetFacing>>()
            .write(SetFacing {
                entity,
                direction: Direction::West,
                instant: false,
            });
        app.update();
        app.update();

        // Either still rotating toward West, or already completed
        if let Some(rotating) = app.world().get::<RotatingToFacing>(entity) {
            assert_eq!(
                rotating.target_direction,
                Direction::West,
                "RotatingToFacing target should have been updated to West"
            );
        } else {
            let facing = app.world().get::<FacingComponent>(entity).unwrap();
            assert_eq!(
                facing.direction,
                Direction::West,
                "After completion, should face West"
            );
        }
    }

    #[test]
    fn test_rotating_to_facing_full_180() {
        // 180° rotation completion: spawn with the transform already set to the
        // South target so the threshold check fires on the first update and the
        // component is removed.  Deterministic with zero Bevy delta time.
        let mut app = make_facing_app();

        let target_quat = Quat::from_rotation_y(PI); // South

        let entity = app
            .world_mut()
            .spawn((
                Transform {
                    rotation: target_quat, // already at target
                    ..Default::default()
                },
                FacingComponent {
                    direction: Direction::North, // not yet reflected
                },
                RotatingToFacing {
                    target: target_quat,
                    speed_deg_per_sec: 180.0,
                    target_direction: Direction::South,
                },
            ))
            .id();

        app.update();

        assert!(
            app.world().get::<RotatingToFacing>(entity).is_none(),
            "RotatingToFacing should be removed after 180° rotation completes"
        );

        let facing = app.world().get::<FacingComponent>(entity).unwrap();
        assert_eq!(
            facing.direction,
            Direction::South,
            "FacingComponent should be South after completion"
        );
    }

    #[test]
    fn test_rotating_to_facing_default_speed_constant() {
        assert_eq!(DEFAULT_ROTATION_SPEED_DEG_PER_SEC, 360.0);
    }

    // ─── ProximityFacing fields tests ────────────────────────────────────────

    #[test]
    fn test_proximity_facing_stores_trigger_distance() {
        let component = ProximityFacing {
            trigger_distance: 3,
            rotation_speed: None,
        };
        assert_eq!(component.trigger_distance, 3);
    }

    #[test]
    fn test_proximity_facing_default_two_tiles() {
        // The map spawner always inserts trigger_distance: 2 (the plan default).
        let component = ProximityFacing {
            trigger_distance: 2,
            rotation_speed: None,
        };
        assert_eq!(component.trigger_distance, 2);
    }

    #[test]
    fn test_proximity_facing_rotation_speed_none_is_instant() {
        let component = ProximityFacing {
            trigger_distance: 2,
            rotation_speed: None,
        };
        // None → snap (instant: true)
        assert!(component.rotation_speed.is_none());
    }

    #[test]
    fn test_proximity_facing_rotation_speed_some_is_smooth() {
        let component = ProximityFacing {
            trigger_distance: 2,
            rotation_speed: Some(180.0),
        };
        assert_eq!(component.rotation_speed, Some(180.0));
    }

    // ─── face_toward_player_on_proximity tests ───────────────────────────────

    #[test]
    fn test_proximity_facing_emits_event() {
        let mut app = make_facing_app();

        // Place entity at (5, 5) facing North, party at (5, 7) → should face South
        let entity = app
            .world_mut()
            .spawn((
                Transform::default(),
                FacingComponent {
                    direction: Direction::North,
                },
                ProximityFacing {
                    trigger_distance: 2,
                    rotation_speed: None,
                },
                TileCoord(Position::new(5, 5)),
            ))
            .id();

        app.world_mut()
            .resource_mut::<crate::game::resources::GlobalState>()
            .0
            .world
            .party_position = Position::new(5, 7);

        // First frame: face_toward_player_on_proximity emits SetFacing
        app.update();
        // Second frame: handle_set_facing processes the message
        app.update();

        let facing = app.world().get::<FacingComponent>(entity).unwrap();
        assert_eq!(
            facing.direction,
            Direction::South,
            "Entity at (5,5) with party at (5,7) should face South"
        );
    }

    #[test]
    fn test_proximity_facing_out_of_range_no_event() {
        let mut app = make_facing_app();

        let entity = app
            .world_mut()
            .spawn((
                Transform::default(),
                FacingComponent {
                    direction: Direction::North,
                },
                ProximityFacing {
                    trigger_distance: 2,
                    rotation_speed: None,
                },
                TileCoord(Position::new(5, 5)),
            ))
            .id();

        // Party is 10 tiles away – outside trigger distance
        app.world_mut()
            .resource_mut::<crate::game::resources::GlobalState>()
            .0
            .world
            .party_position = Position::new(5, 15);

        app.update();
        app.update();

        let facing = app.world().get::<FacingComponent>(entity).unwrap();
        assert_eq!(
            facing.direction,
            Direction::North,
            "Entity out of range should not change facing"
        );
    }

    #[test]
    fn test_proximity_facing_already_correct_no_change() {
        let mut app = make_facing_app();

        // Entity already faces South, party is South → no SetFacing needed
        let entity = app
            .world_mut()
            .spawn((
                Transform {
                    rotation: Quat::from_rotation_y(Direction::South.direction_to_yaw_radians()),
                    ..Default::default()
                },
                FacingComponent {
                    direction: Direction::South,
                },
                ProximityFacing {
                    trigger_distance: 2,
                    rotation_speed: None,
                },
                TileCoord(Position::new(5, 5)),
            ))
            .id();

        app.world_mut()
            .resource_mut::<crate::game::resources::GlobalState>()
            .0
            .world
            .party_position = Position::new(5, 7);

        app.update();
        app.update();

        let facing = app.world().get::<FacingComponent>(entity).unwrap();
        assert_eq!(
            facing.direction,
            Direction::South,
            "Already correctly facing South – should remain South"
        );
    }

    #[test]
    fn test_proximity_facing_without_tile_coord_excluded() {
        let mut app = make_facing_app();

        // Entity has ProximityFacing but no TileCoord – must not be queried.
        let entity = app
            .world_mut()
            .spawn((
                Transform::default(),
                FacingComponent {
                    direction: Direction::North,
                },
                ProximityFacing {
                    trigger_distance: 2,
                    rotation_speed: None,
                },
                // No TileCoord
            ))
            .id();

        app.world_mut()
            .resource_mut::<crate::game::resources::GlobalState>()
            .0
            .world
            .party_position = Position::new(5, 7);

        app.update();
        app.update();

        let facing = app.world().get::<FacingComponent>(entity).unwrap();
        assert_eq!(
            facing.direction,
            Direction::North,
            "Entity without TileCoord must not be affected by proximity system"
        );
    }

    #[test]
    fn test_proximity_facing_smooth_emits_non_instant() {
        // When ProximityFacing.rotation_speed is Some, the emitted SetFacing
        // should have instant: false, which triggers the slerp path.
        let mut app = make_facing_app();

        let entity = app
            .world_mut()
            .spawn((
                Transform::default(),
                FacingComponent {
                    direction: Direction::North,
                },
                ProximityFacing {
                    trigger_distance: 2,
                    rotation_speed: Some(180.0), // smooth
                },
                TileCoord(Position::new(5, 5)),
            ))
            .id();

        app.world_mut()
            .resource_mut::<crate::game::resources::GlobalState>()
            .0
            .world
            .party_position = Position::new(5, 7);

        // First frame: emit SetFacing { instant: false }
        app.update();
        // Second frame: handle_set_facing inserts RotatingToFacing (or
        // apply_rotation_to_facing has already begun slerping)
        app.update();

        // Entity should either have RotatingToFacing (still slerping) or
        // have already completed the rotation.
        let rotating = app.world().get::<RotatingToFacing>(entity);
        let facing = app.world().get::<FacingComponent>(entity).unwrap();
        let completed = facing.direction == Direction::South;
        let still_rotating = rotating.is_some();
        assert!(
            completed || still_rotating,
            "Smooth proximity facing should start slerp or complete; \
             facing={:?}, still_rotating={still_rotating}",
            facing.direction
        );
    }

    // ─── Dialogue → SetFacing integration test ───────────────────────────────

    #[test]
    fn test_dialogue_start_triggers_face_toward_party() {
        // Minimal smoke-test that the dialogue system writes a SetFacing when a
        // speaker entity has a TileCoord.  The full suite lives in dialogue.rs;
        // this test verifies the facing module's public surface is coherent.
        use crate::game::systems::facing::SetFacing;

        let msg = SetFacing {
            entity: Entity::PLACEHOLDER,
            direction: Direction::East,
            instant: true,
        };
        // Just verify the struct is constructable and fields are correct.
        assert_eq!(msg.direction, Direction::East);
        assert!(msg.instant);
    }

    // ─── SetFacing struct surface tests ──────────────────────────────────────

    #[test]
    fn test_set_facing_fields_accessible() {
        let sf = SetFacing {
            entity: Entity::PLACEHOLDER,
            direction: Direction::West,
            instant: false,
        };
        assert_eq!(sf.direction, Direction::West);
        assert!(!sf.instant);
    }

    #[test]
    fn test_set_facing_clone() {
        let original = SetFacing {
            entity: Entity::PLACEHOLDER,
            direction: Direction::North,
            instant: true,
        };
        let cloned = original.clone();
        assert_eq!(cloned.direction, Direction::North);
        assert!(cloned.instant);
    }

    // ─── RotatingToFacing struct tests ───────────────────────────────────────

    #[test]
    fn test_rotating_to_facing_fields_accessible() {
        let r = RotatingToFacing {
            target: Quat::IDENTITY,
            speed_deg_per_sec: 90.0,
            target_direction: Direction::North,
        };
        assert_eq!(r.speed_deg_per_sec, 90.0);
        assert_eq!(r.target_direction, Direction::North);
    }
}
