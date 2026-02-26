// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Inventory ECS components
//!
//! Provides the ECS identity layer that links Bevy entities to party members
//! and tracks all party-member entities in a single resource.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.3 for character and party
//! specifications, and `docs/explanation/ecs_inventory_view_implementation_plan.md`
//! Phase 1 for the design rationale behind these types.

use crate::domain::character::PARTY_MAX_SIZE;
use bevy::prelude::*;

/// Links a Bevy entity to the party member at a given party index.
///
/// Attach this component to a pure-identity entity (no mesh, no transform)
/// so that inventory systems can resolve the correct [`Character`] from
/// `GlobalState` by index without coupling ECS queries to the domain model.
///
/// # Examples
///
/// ```
/// use antares::game::components::inventory::CharacterEntity;
///
/// let marker = CharacterEntity { party_index: 2 };
/// assert_eq!(marker.party_index, 2);
/// ```
///
/// [`Character`]: crate::domain::character::Character
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterEntity {
    /// Zero-based index into [`Party::members`].
    ///
    /// Valid range: `0..PARTY_MAX_SIZE` (currently `0..6`).
    ///
    /// [`Party::members`]: crate::domain::character::Party::members
    pub party_index: usize,
}

/// Resource mapping party indices to Bevy [`Entity`] handles.
///
/// One entry per possible party slot (up to [`PARTY_MAX_SIZE`]).  Each slot
/// holds `None` until the corresponding entity is spawned by
/// `setup_party_entities` during `Startup`.  Systems that need to attach or
/// query per-character components should look up the entity here.
///
/// # Examples
///
/// ```
/// use antares::game::components::inventory::PartyEntities;
///
/// let pe = PartyEntities::default();
/// assert!(pe.entities.iter().all(|e| e.is_none()));
/// ```
#[derive(Resource, Debug)]
pub struct PartyEntities {
    /// Indexed by party slot.  `entities[i]` is the Bevy entity for party
    /// member `i`, or `None` if the slot is unoccupied.
    pub entities: [Option<Entity>; PARTY_MAX_SIZE],
}

impl Default for PartyEntities {
    /// Returns a `PartyEntities` with every slot set to `None`.
    fn default() -> Self {
        Self {
            entities: [None; PARTY_MAX_SIZE],
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::{App, Entity, World};

    // ------------------------------------------------------------------
    // CharacterEntity component derive tests
    // ------------------------------------------------------------------

    /// Verify that `CharacterEntity` can be inserted into a Bevy `World`
    /// and queried back with the correct `party_index`.
    #[test]
    fn test_character_entity_component() {
        let mut world = World::new();
        let entity = world.spawn(CharacterEntity { party_index: 2 }).id();

        let stored = world
            .get::<CharacterEntity>(entity)
            .expect("CharacterEntity should be present on entity");

        assert_eq!(stored.party_index, 2);
    }

    /// Verify round-trip: spawn multiple party-index values and confirm
    /// each entity reports its own distinct index.
    #[test]
    fn test_character_entity_component_multiple_indices() {
        let mut world = World::new();

        let entities: Vec<_> = (0..PARTY_MAX_SIZE)
            .map(|i| world.spawn(CharacterEntity { party_index: i }).id())
            .collect();

        for (expected_index, entity) in entities.iter().enumerate() {
            let stored = world
                .get::<CharacterEntity>(*entity)
                .expect("CharacterEntity should be present");
            assert_eq!(stored.party_index, expected_index);
        }
    }

    /// Verify that `CharacterEntity` correctly implements `Copy` and `PartialEq`.
    #[test]
    fn test_character_entity_copy_and_eq() {
        let a = CharacterEntity { party_index: 1 };
        let b = a; // Copy
        assert_eq!(a, b);

        let c = CharacterEntity { party_index: 3 };
        assert_ne!(a, c);
    }

    // ------------------------------------------------------------------
    // PartyEntities resource tests
    // ------------------------------------------------------------------

    /// `PartyEntities::default()` must initialise all slots to `None` and
    /// have exactly `PARTY_MAX_SIZE` slots.
    #[test]
    fn test_party_entities_resource_default() {
        let pe = PartyEntities::default();

        assert_eq!(
            pe.entities.len(),
            PARTY_MAX_SIZE,
            "entities array must have exactly PARTY_MAX_SIZE slots"
        );
        assert!(
            pe.entities.iter().all(|e| e.is_none()),
            "all slots must start as None"
        );
    }

    /// Build a minimal `App`, call `init_resource::<PartyEntities>()`, and
    /// verify the resource is accessible with all `None` slots.
    #[test]
    fn test_party_entities_resource_init() {
        let mut app = App::new();
        app.init_resource::<PartyEntities>();

        let pe = app
            .world()
            .get_resource::<PartyEntities>()
            .expect("PartyEntities resource should exist after init_resource");

        assert_eq!(pe.entities.len(), PARTY_MAX_SIZE);
        assert!(pe.entities.iter().all(|e| e.is_none()));
    }

    /// Verify that individual slots in `PartyEntities` can be set and read
    /// back correctly once entities exist.
    #[test]
    fn test_party_entities_slot_assignment() {
        let mut world = World::new();

        // Spawn one entity per party slot
        let spawned: Vec<Entity> = (0..PARTY_MAX_SIZE)
            .map(|i| world.spawn(CharacterEntity { party_index: i }).id())
            .collect();

        // Build a PartyEntities resource from the spawned entities
        let mut entities_arr = [None; PARTY_MAX_SIZE];
        for (i, &entity) in spawned.iter().enumerate() {
            entities_arr[i] = Some(entity);
        }
        let pe = PartyEntities {
            entities: entities_arr,
        };

        // Every slot should now point to the corresponding entity
        for (i, &entity) in spawned.iter().enumerate() {
            assert_eq!(
                pe.entities[i],
                Some(entity),
                "slot {i} should hold the spawned entity"
            );
        }
    }

    /// Confirm that `PartyEntities` can be inserted as a resource into a
    /// `World` and retrieved without issue.
    #[test]
    fn test_party_entities_insert_resource() {
        let mut world = World::new();
        world.insert_resource(PartyEntities::default());

        let pe = world
            .get_resource::<PartyEntities>()
            .expect("resource should be retrievable after insert");

        assert_eq!(pe.entities.len(), PARTY_MAX_SIZE);
    }
}
