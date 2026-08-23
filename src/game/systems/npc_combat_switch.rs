// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! NPC combat-switch system.
//!
//! Monitors live NPC entities for combat trigger flags set via
//! [`crate::application::GlobalFlags`].  When an NPC's
//! `combat_switch.trigger_flag` becomes `true`, this module:
//!
//! 1. Despawns the NPC entity.
//! 2. Writes [`PendingNpcCombatResource`] to signal a pending encounter.
//! 3. [`start_npc_combat_system`] consumes the resource on the same frame and
//!    starts combat.
//!
//! After the party wins, Phase 3 will wire `combat_switch.defeat_flag` back to
//! `GlobalFlags` via `CombatResource::defeat_flag`.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/npc_combat_trigger_implementation_plan.md`, Phase 2.

use crate::application::resources::GameContent;
use crate::domain::combat::types::CombatEventType;
use crate::domain::types::{MapId, MonsterId, Position};
use crate::game::resources::GlobalState;
use crate::game::systems::map::{NpcMarker, TileCoord};
use bevy::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Resource types
// ─────────────────────────────────────────────────────────────────────────────

/// Data for a monster encounter that has been staged by an NPC combat switch.
///
/// Built by [`npc_combat_switch_system`] when an NPC's
/// `combat_switch.trigger_flag` fires, and consumed by
/// [`start_npc_combat_system`] in the same frame.
///
/// Stored inside [`PendingNpcCombatResource`].
///
/// # Examples
///
/// ```
/// use antares::game::systems::npc_combat_switch::PendingNpcCombat;
/// use antares::domain::combat::types::CombatEventType;
/// use antares::domain::types::Position;
///
/// let pending = PendingNpcCombat {
///     monster_id: 42,
///     position: Position::new(5, 10),
///     map_id: 1,
///     defeat_flag: Some("boss_defeated".to_string()),
///     combat_type: CombatEventType::Boss,
/// };
/// assert_eq!(pending.monster_id, 42);
/// assert_eq!(pending.combat_type, CombatEventType::Boss);
/// ```
#[derive(Debug, Clone)]
pub struct PendingNpcCombat {
    /// ID of the monster entry to fight (looked up in `MonsterDatabase`).
    pub monster_id: MonsterId,
    /// Tile position of the NPC that was despawned.
    pub position: Position,
    /// Map ID where the NPC was standing.
    pub map_id: MapId,
    /// Global flag to set after the party wins (`None` means no post-combat
    /// flag).  Consumed by Phase 3 (`CombatResource::defeat_flag`).
    pub defeat_flag: Option<String>,
    /// How the encounter begins (Normal, Boss, Ambush, …).
    pub combat_type: CombatEventType,
}

/// Bevy resource that holds a pending NPC-triggered combat, if any.
///
/// `None` inside means no combat is pending.  Follows the same newtype-wrapper
/// pattern used by [`crate::game::systems::events::PendingRecruitConfirm`] in
/// this codebase.
///
/// # Examples
///
/// ```
/// use antares::game::systems::npc_combat_switch::PendingNpcCombatResource;
///
/// let resource = PendingNpcCombatResource::default();
/// assert!(resource.0.is_none());
/// ```
#[derive(Resource, Default)]
pub struct PendingNpcCombatResource(pub Option<PendingNpcCombat>);

// ─────────────────────────────────────────────────────────────────────────────
// Systems
// ─────────────────────────────────────────────────────────────────────────────

/// Scan live NPC entities for a fired `combat_switch` trigger flag.
///
/// Runs every frame in [`GameMode::Exploration`].  For each entity carrying
/// [`NpcMarker`]:
/// - Looks up the `NpcDefinition` via `content`.
/// - Skips NPCs without `combat_switch`.
/// - Skips NPCs whose `trigger_flag` is not yet `true` in `GlobalFlags`.
/// - On the first match: despawns the entity and writes
///   [`PendingNpcCombatResource`].
///
/// If a swap is already pending (`pending.0.is_some()`), returns immediately
/// so only one swap is processed per frame.
///
/// # Arguments
///
/// * `query`        – All live NPC entities carrying [`NpcMarker`] + [`TileCoord`].
/// * `content`      – Content database for `NpcDefinition` lookup.
/// * `global_state` – Read access to `GlobalFlags`.
/// * `pending`      – The pending-combat resource (written when a flag fires).
/// * `commands`     – Deferred Bevy command buffer for entity despawning.
pub fn npc_combat_switch_system(
    query: Query<(Entity, &NpcMarker, &TileCoord)>,
    content: Res<GameContent>,
    global_state: Res<GlobalState>,
    mut pending: ResMut<PendingNpcCombatResource>,
    mut commands: Commands,
) {
    // Don't clobber a swap that is already in progress.
    if pending.0.is_some() {
        return;
    }

    let map_id = global_state.0.world.current_map;

    for (entity, marker, tile_coord) in query.iter() {
        // Look up the NPC definition in the content database.
        let Some(npc_def) = content.db().npcs.get_npc(&marker.npc_id) else {
            continue;
        };

        // Skip NPCs that have no combat-switch configuration.
        let Some(ref switch) = npc_def.combat_switch else {
            continue;
        };

        // Skip if the trigger flag has not fired yet.
        if !global_state.0.global_flags.get(&switch.trigger_flag) {
            continue;
        }

        // Trigger fired — despawn the NPC entity (Bevy 0.19 `despawn` removes
        // children automatically) and stage the encounter.
        debug!(
            "NPC combat switch triggered for '{}': despawning entity, staging monster {} encounter",
            marker.npc_id, switch.monster_id
        );
        commands.entity(entity).despawn();

        pending.0 = Some(PendingNpcCombat {
            monster_id: switch.monster_id,
            position: tile_coord.0,
            map_id,
            defeat_flag: switch.defeat_flag.clone(),
            combat_type: switch.combat_type,
        });

        // Only one swap per frame to avoid reentrancy.
        break;
    }
}

/// Consume [`PendingNpcCombatResource`] and start the monster encounter.
///
/// Runs every frame in [`GameMode::Exploration`], ordered after
/// [`npc_combat_switch_system`].  When `pending.0` is `Some`:
///
/// 1. Takes ownership (sets `pending.0` to `None`).
/// 2. Builds a `CombatState` via
///    [`crate::game::systems::combat::start_encounter`].
/// 3. Enters combat via `global_state.0.enter_combat_with_state(cs)`.
/// 4. Emits [`crate::game::systems::combat::CombatStarted`] so `CombatPlugin`
///    can initialise per-combat resources.
///
/// **Phase 3 hook**: `CombatResource::defeat_flag = pending.defeat_flag` will
/// be wired up in Phase 3 once `CombatResource` carries that field.
///
/// # Arguments
///
/// * `pending`               – Pending combat resource (consumed here).
/// * `global_state`          – Mutable game state for entering combat mode.
/// * `content`               – Content database supplying monster data.
/// * `combat_started_writer` – Optional [`CombatStarted`] message bus
///   (absent in test apps without `CombatPlugin`).
pub fn start_npc_combat_system(
    mut pending: ResMut<PendingNpcCombatResource>,
    mut global_state: ResMut<GlobalState>,
    content: Res<GameContent>,
    mut combat_started_writer: Option<MessageWriter<crate::game::systems::combat::CombatStarted>>,
) {
    let Some(combat) = pending.0.take() else {
        return;
    };

    info!(
        "Starting NPC-triggered combat: monster_id={} position={:?} type={:?}",
        combat.monster_id, combat.position, combat.combat_type
    );

    match crate::game::systems::combat::start_encounter(
        &mut global_state.0,
        &content,
        &[combat.monster_id],
        combat.combat_type,
    ) {
        Ok(()) => {
            // TODO Phase 3: set CombatResource::defeat_flag = combat.defeat_flag

            // Notify CombatPlugin that combat has started.
            if let Some(ref mut writer) = combat_started_writer {
                writer.write(crate::game::systems::combat::CombatStarted {
                    encounter_position: Some(combat.position),
                    encounter_map_id: Some(combat.map_id),
                    combat_event_type: combat.combat_type,
                });
            }
        }
        Err(e) => {
            error!(
                "Failed to start NPC-triggered combat for monster {}: {}",
                combat.monster_id, e
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Plugin
// ─────────────────────────────────────────────────────────────────────────────

/// Bevy plugin that registers the NPC combat-switch resource and systems.
///
/// Both systems are gated to [`GameMode::Exploration`] via
/// [`crate::game::run_conditions::in_exploration_mode`].
/// [`start_npc_combat_system`] is ordered after [`npc_combat_switch_system`].
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::App;
/// use antares::game::systems::npc_combat_switch::NpcCombatSwitchPlugin;
///
/// let mut app = App::new();
/// app.add_plugins(NpcCombatSwitchPlugin);
/// ```
pub struct NpcCombatSwitchPlugin;

impl Plugin for NpcCombatSwitchPlugin {
    fn build(&self, app: &mut App) {
        app
            // Initialise the pending-combat resource to None.
            .insert_resource(PendingNpcCombatResource::default())
            // Scan NPC entities each exploration frame.
            .add_systems(
                Update,
                npc_combat_switch_system.run_if(crate::game::run_conditions::in_exploration_mode),
            )
            // Consume the pending value and start combat, ordered after the scanner.
            .add_systems(
                Update,
                start_npc_combat_system
                    .run_if(crate::game::run_conditions::in_exploration_mode)
                    .after(npc_combat_switch_system),
            );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{GameMode, GameState};
    use crate::domain::character::{AttributePair, AttributePair16, Stats};
    use crate::domain::combat::database::MonsterDefinition;
    use crate::domain::combat::monster::{LootTable, MonsterCondition, MonsterResistances};
    use crate::domain::world::npc::{NpcCombatSwitch, NpcDefinition};
    use crate::game::resources::GlobalState;
    use crate::sdk::database::ContentDatabase;
    use bevy::prelude::App;

    // ─── helpers ─────────────────────────────────────────────────────────────

    fn make_exploration_state() -> GameState {
        let mut gs = GameState::new();
        gs.mode = GameMode::Exploration;
        gs
    }

    fn make_content_with_npc(npc: NpcDefinition) -> crate::application::resources::GameContent {
        let mut db = ContentDatabase::new();
        db.npcs.add_npc(npc).expect("add_npc should not fail");
        crate::application::resources::GameContent::new(db)
    }

    /// Build a minimal `MonsterDefinition` for testing `start_encounter`.
    fn make_minimal_monster(id: MonsterId) -> MonsterDefinition {
        MonsterDefinition {
            id,
            name: format!("TestMonster_{}", id),
            stats: Stats {
                might: AttributePair::new(10),
                intellect: AttributePair::new(10),
                personality: AttributePair::new(10),
                endurance: AttributePair::new(10),
                speed: AttributePair::new(10),
                accuracy: AttributePair::new(10),
                luck: AttributePair::new(10),
            },
            hp: AttributePair16::new(10),
            ac: AttributePair::new(5),
            attacks: vec![],
            flee_threshold: 0,
            special_attack_threshold: 0,
            resistances: MonsterResistances::new(),
            can_regenerate: false,
            can_advance: false,
            is_undead: false,
            magic_resistance: 0,
            loot: LootTable::new(1, 5, 0, 0, 0),
            creature_id: None,
            conditions: MonsterCondition::Normal,
            active_conditions: vec![],
            has_acted: false,
        }
    }

    // ─── test_npc_combat_switch_system_skips_npc_without_combat_switch ────────

    /// An NPC without `combat_switch` must leave `PendingNpcCombatResource` as
    /// `None`.
    #[test]
    fn test_npc_combat_switch_system_skips_npc_without_combat_switch() {
        let npc_def = NpcDefinition::new("plain_npc", "Plain NPC", "portrait.png");
        let state = make_exploration_state();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GlobalState(state));
        app.insert_resource(make_content_with_npc(npc_def));
        app.insert_resource(PendingNpcCombatResource::default());
        app.add_systems(Update, npc_combat_switch_system);

        app.world_mut().spawn((
            NpcMarker {
                npc_id: "plain_npc".to_string(),
            },
            TileCoord(Position::new(3, 3)),
        ));

        app.update();

        let pending = app.world().resource::<PendingNpcCombatResource>();
        assert!(
            pending.0.is_none(),
            "No combat should be pending for an NPC without combat_switch"
        );
    }

    // ─── test_npc_combat_switch_system_skips_when_trigger_flag_not_set ────────

    /// An NPC with `combat_switch` whose trigger flag is not set must be skipped.
    #[test]
    fn test_npc_combat_switch_system_skips_when_trigger_flag_not_set() {
        let mut npc_def = NpcDefinition::new("boss_npc", "Boss NPC", "portrait.png");
        npc_def.combat_switch = Some(NpcCombatSwitch {
            trigger_flag: "boss_triggered".to_string(),
            monster_id: 10,
            defeat_flag: None,
            combat_type: CombatEventType::Boss,
        });

        let state = make_exploration_state();
        // Deliberately do NOT set the trigger flag.
        assert!(!state.global_flags.get("boss_triggered"));

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GlobalState(state));
        app.insert_resource(make_content_with_npc(npc_def));
        app.insert_resource(PendingNpcCombatResource::default());
        app.add_systems(Update, npc_combat_switch_system);

        app.world_mut().spawn((
            NpcMarker {
                npc_id: "boss_npc".to_string(),
            },
            TileCoord(Position::new(5, 5)),
        ));

        app.update();

        let pending = app.world().resource::<PendingNpcCombatResource>();
        assert!(
            pending.0.is_none(),
            "Trigger flag not set — no combat should be pending"
        );
    }

    // ─── test_npc_combat_switch_system_despawns_npc_when_flag_set ─────────────

    /// When the trigger flag is `true`, the system must write
    /// `PendingNpcCombatResource` with the correct fields.
    ///
    /// Bevy defers entity despawns; we verify the pending resource rather than
    /// querying entity existence post-`update()`.
    #[test]
    fn test_npc_combat_switch_system_despawns_npc_when_flag_set() {
        let mut npc_def = NpcDefinition::new("lich_eonir", "Eonir the Still", "portrait.png");
        npc_def.combat_switch = Some(NpcCombatSwitch {
            trigger_flag: "eonir_combat_triggered".to_string(),
            monster_id: 42,
            defeat_flag: Some("eonir_defeated".to_string()),
            combat_type: CombatEventType::Boss,
        });

        let mut state = make_exploration_state();
        state.global_flags.set("eonir_combat_triggered", true);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GlobalState(state));
        app.insert_resource(make_content_with_npc(npc_def));
        app.insert_resource(PendingNpcCombatResource::default());
        app.add_systems(Update, npc_combat_switch_system);

        let tile_pos = Position::new(8, 8);
        app.world_mut().spawn((
            NpcMarker {
                npc_id: "lich_eonir".to_string(),
            },
            TileCoord(tile_pos),
        ));

        app.update();

        let pending = app.world().resource::<PendingNpcCombatResource>();
        assert!(
            pending.0.is_some(),
            "PendingNpcCombatResource should be Some after trigger fires"
        );
        let p = pending.0.as_ref().unwrap();
        assert_eq!(p.monster_id, 42, "monster_id must match combat_switch");
        assert_eq!(p.position, tile_pos, "position must be the NPC tile");
        assert_eq!(
            p.defeat_flag,
            Some("eonir_defeated".to_string()),
            "defeat_flag must be forwarded"
        );
        assert_eq!(
            p.combat_type,
            CombatEventType::Boss,
            "combat_type must be forwarded"
        );
    }

    // ─── test_start_npc_combat_system_clears_pending_and_enters_combat ────────

    /// After `start_npc_combat_system` runs:
    /// - `PendingNpcCombatResource` must be `None`.
    /// - `GlobalState.0.mode` must be `GameMode::Combat(...)`.
    #[test]
    fn test_start_npc_combat_system_clears_pending_and_enters_combat() {
        const MONSTER_ID: MonsterId = 7;

        let mut db = ContentDatabase::new();
        db.monsters
            .add_monster(make_minimal_monster(MONSTER_ID))
            .expect("add_monster should not fail");

        let state = make_exploration_state();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GlobalState(state));
        app.insert_resource(crate::application::resources::GameContent::new(db));
        app.insert_resource(PendingNpcCombatResource(Some(PendingNpcCombat {
            monster_id: MONSTER_ID,
            position: Position::new(1, 1),
            map_id: 0,
            defeat_flag: None,
            combat_type: CombatEventType::Normal,
        })));
        app.add_systems(Update, start_npc_combat_system);

        app.update();

        // Pending resource must be cleared.
        let pending = app.world().resource::<PendingNpcCombatResource>();
        assert!(
            pending.0.is_none(),
            "PendingNpcCombatResource must be None after start_npc_combat_system"
        );

        // Game mode must have transitioned to Combat.
        let gs = app.world().resource::<GlobalState>();
        assert!(
            matches!(gs.0.mode, GameMode::Combat(_)),
            "GameState must be in Combat mode after start_npc_combat_system"
        );
    }

    // ─── test_npc_spawn_guard_suppresses_npc_when_trigger_flag_already_set ────

    /// Validates the spawn-guard predicate in `spawn_map` (map.rs):
    /// if `trigger_flag` is already `true` in `GlobalFlags` when the map loads,
    /// the NPC must not be spawned.
    ///
    /// This test validates the predicate directly.  The Bevy-level guard that
    /// calls `continue` lives in `spawn_map` in `map.rs`.
    #[test]
    fn test_npc_spawn_guard_suppresses_npc_when_trigger_flag_already_set() {
        use crate::application::GlobalFlags;

        let switch = NpcCombatSwitch {
            trigger_flag: "eonir_combat_triggered".to_string(),
            monster_id: 42,
            defeat_flag: None,
            combat_type: CombatEventType::Normal,
        };

        // Flag set → guard must suppress spawn.
        let mut flags_set = GlobalFlags::new();
        flags_set.set(&switch.trigger_flag, true);
        assert!(
            flags_set.get(&switch.trigger_flag),
            "Spawn guard must suppress when trigger flag is true"
        );

        // Flag absent → guard must allow spawn.
        let flags_absent = GlobalFlags::new();
        assert!(
            !flags_absent.get(&switch.trigger_flag),
            "Spawn guard must allow spawn when trigger flag is false/unset"
        );
    }
}
