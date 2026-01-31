// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Combat ECS components
//!
//! Lightweight marker and helper components used by the combat systems and UI.
//!
//! These components are intentionally small and focused on linking ECS entities
//! (UI elements, turn indicators, selection cursors) with the domain `CombatState`
//! model via `CombatantId` values.

use crate::domain::combat::types::CombatantId;
use bevy::prelude::*;

/// Links an ECS entity to a combat participant identified by `CombatantId`.
///
/// This component is used to map UI elements and sprites to the underlying
/// domain combatant so systems can resolve actions, targets, and status.
///
/// # Examples
///
/// ```
/// use antares::game::components::combat::CombatantMarker;
/// use antares::domain::combat::types::CombatantId;
///
/// let marker = CombatantMarker::new(CombatantId::Player(0));
/// assert_eq!(marker.combatant_id, CombatantId::Player(0));
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CombatantMarker {
    /// The combatant this entity represents (player or monster)
    pub combatant_id: CombatantId,
}

impl CombatantMarker {
    /// Create a new marker for the given combatant id
    pub fn new(id: CombatantId) -> Self {
        Self { combatant_id: id }
    }
}

/// Marker component denoting which combatant is currently indicated by a UI turn
/// indicator.
///
/// Typically attached to an entity (e.g., a highlight sprite) so the UI can
/// position it relative to the target combatant's visual.
///
/// # Examples
///
/// ```
/// use antares::game::components::combat::TurnIndicator;
/// use antares::domain::combat::types::CombatantId;
///
/// let t = TurnIndicator::for_combatant(CombatantId::Monster(2));
/// assert_eq!(t.combatant, CombatantId::Monster(2));
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TurnIndicator {
    /// Combatant currently highlighted by this indicator
    pub combatant: CombatantId,
}

impl TurnIndicator {
    /// Create a `TurnIndicator` targeting the specified combatant.
    pub fn for_combatant(combatant: CombatantId) -> Self {
        Self { combatant }
    }
}

/// Marker component for target selection UI.
///
/// This component is attached to UI entities that perform or display a target
/// selection flow. It contains the actor that is currently selecting and the
/// optionally selected target.
///
/// # Examples
///
/// ```
/// use antares::game::components::combat::TargetSelector;
/// use antares::domain::combat::types::CombatantId;
///
/// let mut ts = TargetSelector::new(CombatantId::Player(0));
/// assert_eq!(ts.selecting_for, CombatantId::Player(0));
/// assert!(ts.selected_target.is_none());
///
/// ts.selected_target = Some(CombatantId::Monster(1));
/// assert_eq!(ts.selected_target, Some(CombatantId::Monster(1)));
/// ```
#[derive(Component, Debug, Clone)]
pub struct TargetSelector {
    /// The combatant that is performing target selection (attacker/caster)
    pub selecting_for: CombatantId,
    /// Currently selected target (if any)
    pub selected_target: Option<CombatantId>,
}

impl TargetSelector {
    /// Create a new `TargetSelector` for a combatant.
    pub fn new(selecting_for: CombatantId) -> Self {
        Self {
            selecting_for,
            selected_target: None,
        }
    }

    /// Clear the current selection.
    pub fn clear(&mut self) {
        self.selected_target = None;
    }

    /// Set the selected target.
    pub fn select(&mut self, target: CombatantId) {
        self.selected_target = Some(target);
    }
}

/// Root marker for combat HUD elements.
///
/// Attach this to the top-level UI node for the combat HUD so it can be
/// readily spawned/despawned as a unit when entering/exiting combat.
///
/// # Examples
///
/// ```
/// use antares::game::components::combat::CombatHudRoot;
///
/// let root = CombatHudRoot;
/// let _ = root; // Marker is zero-sized and used only as an entity tag
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CombatHudRoot;
