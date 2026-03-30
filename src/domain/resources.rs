// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Resource management system - Food, light, and rest mechanics
//!
//! This module implements party-wide resource management including food
//! consumption, light tracking, and rest/recovery mechanics.
//!
//! # Food System
//!
//! Food is tracked as inventory items (e.g. "Food Ration") carrying
//! `ConsumableEffect::IsFood(n)`, not as the legacy `Character.food` numeric counter.
//!
//! - [`count_food_in_party`] — counts total ration-units across all inventories.
//! - [`consume_food`] — removes `IsFood` items from inventories, sharing across members.
//! - [`check_starvation`] — returns `true` when no `IsFood` items remain.
//! - [`food_needed_to_rest`] — 1 ration per living member (unchanged semantics).
//!
//! The `ItemDatabase` is passed into food functions so that item IDs can be resolved
//! to their `ConsumableEffect` without coupling the resource layer to specific IDs.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 5.4 for complete specifications.

use crate::domain::character::Party;
use crate::domain::items::database::ItemDatabase;
use crate::domain::items::types::{ConsumableEffect, ItemType};
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur during resource management
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ResourceError {
    #[error("Party has no food remaining")]
    NoFoodRemaining,

    #[error("Party has no light remaining")]
    NoLightRemaining,

    #[error("Cannot rest - party is in combat")]
    CannotRestInCombat,

    #[error("Party is too hungry to rest")]
    TooHungryToRest,

    /// Rest was interrupted by an incoming random encounter.
    ///
    /// Returned when a random-encounter check fires during a per-hour rest
    /// loop before the requested rest duration is complete.
    ///
    /// # Combat Event Type Requirement
    ///
    /// Any encounter that fires while the party is resting **MUST** be started
    /// with `CombatEventType::Ambush`.  The resting party is asleep and cannot
    /// react — the ambush mechanic (monsters act first in round 1, party turns
    /// suppressed) correctly models this.  The rest system implementation is
    /// responsible for passing `CombatEventType::Ambush` to `start_encounter()`
    /// whenever it returns this error variant and triggers combat.
    ///
    /// See: `docs/explanation/combat_events_implementation_plan.md` Section 2.7.
    #[error("Cannot rest - active encounter in progress")]
    CannotRestWithActiveEncounter,

    /// Rest ended before the requested number of hours elapsed.
    ///
    /// `hours_completed` is the number of full rest hours that were
    /// successfully processed before the interruption occurred.
    #[error("Rest interrupted after {hours_completed} hour(s)")]
    RestInterrupted { hours_completed: u32 },
}

// ===== Constants =====

/// Minutes of game time consumed per tile stepped in exploration mode.
pub const TIME_COST_STEP_MINUTES: u32 = 5;

/// Minutes of game time consumed per combat round.
pub const TIME_COST_COMBAT_ROUND_MINUTES: u32 = 5;

/// Minutes of game time consumed when transitioning between maps (same-world).
pub const TIME_COST_MAP_TRANSITION_MINUTES: u32 = 30;

/// Food ration-units consumed per party member for a full rest period.
///
/// Before a rest begins the system calls [`count_food_in_party`] to verify that
/// the total `IsFood` ration-units across all character inventories is at least
/// `party.members.len() * FOOD_PER_REST`.  If the check passes, exactly
/// `party.members.len() * FOOD_PER_REST` ration-units are consumed from
/// inventories upfront and the party sleeps undisturbed.  If it fails, rest is
/// refused entirely — no items are consumed and no HP/SP is restored.
pub const FOOD_PER_REST: u32 = 1;

/// Food consumption per day of travel
pub const FOOD_PER_DAY: u32 = 3;

/// Light consumption per hour in dark areas
pub const LIGHT_PER_HOUR: u32 = 1;

/// Hours required for a complete (full-heal) rest.
///
/// A full rest takes 12 hours per the game specification.
pub const REST_DURATION_HOURS: u32 = 12;

/// HP/SP restored per hour during a full 12-hour rest (100% total).
///
/// Used only by `rest_party` (the non-interactive batch helper).
/// The interactive rest system uses `RestDuration::restore_fraction_per_hour`.
pub const HP_RESTORE_RATE: f32 = 1.0 / REST_DURATION_HOURS as f32;

/// SP restored per hour during a full 12-hour rest (100% total).
///
/// Used only by `rest_party` (the non-interactive batch helper).
pub const SP_RESTORE_RATE: f32 = 1.0 / REST_DURATION_HOURS as f32;

// ===== Rest Duration =====

/// The three player-selectable rest durations.
///
/// | Variant  | Hours | HP/SP restored |
/// |----------|-------|----------------|
/// | `Short`  |   4   |      50 %       |
/// | `Long`   |   8   |      75 %       |
/// | `Full`   |  12   |     100 %       |
///
/// Food cost is always 1 ration per party member regardless of duration.
///
/// # Examples
///
/// ```
/// use antares::domain::resources::RestDuration;
///
/// assert_eq!(RestDuration::Short.hours(), 4);
/// assert_eq!(RestDuration::Long.hours(), 8);
/// assert_eq!(RestDuration::Full.hours(), 12);
///
/// // Per-hour fraction × hours == total fraction
/// let d = RestDuration::Short;
/// let total = d.restore_fraction_per_hour() * d.hours() as f32;
/// assert!((total - 0.50).abs() < 0.001);
///
/// let d = RestDuration::Long;
/// let total = d.restore_fraction_per_hour() * d.hours() as f32;
/// assert!((total - 0.75).abs() < 0.001);
///
/// let d = RestDuration::Full;
/// let total = d.restore_fraction_per_hour() * d.hours() as f32;
/// assert!((total - 1.00).abs() < 0.001);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestDuration {
    /// 4 hours — restores 50% HP/SP.
    Short,
    /// 8 hours — restores 75% HP/SP.
    Long,
    /// 12 hours — restores 100% HP/SP.
    Full,
}

impl RestDuration {
    /// In-game hours for this rest duration.
    pub fn hours(self) -> u32 {
        match self {
            Self::Short => 4,
            Self::Long => 8,
            Self::Full => 12,
        }
    }

    /// Total HP/SP fraction restored by this duration (0.0–1.0).
    pub fn total_restore_fraction(self) -> f32 {
        match self {
            Self::Short => 0.50,
            Self::Long => 0.75,
            Self::Full => 1.00,
        }
    }

    /// HP/SP fraction restored **per hour** for this duration.
    ///
    /// Multiply by `base_hp` to get the HP healed each hour tick.
    pub fn restore_fraction_per_hour(self) -> f32 {
        self.total_restore_fraction() / self.hours() as f32
    }

    /// Construct from a raw hour count.  Returns `None` for unrecognised values.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::resources::RestDuration;
    ///
    /// assert_eq!(RestDuration::from_hours(4),  Some(RestDuration::Short));
    /// assert_eq!(RestDuration::from_hours(8),  Some(RestDuration::Long));
    /// assert_eq!(RestDuration::from_hours(12), Some(RestDuration::Full));
    /// assert_eq!(RestDuration::from_hours(6),  None);
    /// ```
    pub fn from_hours(hours: u32) -> Option<Self> {
        match hours {
            4 => Some(Self::Short),
            8 => Some(Self::Long),
            12 => Some(Self::Full),
            _ => None,
        }
    }

    /// All valid rest durations in ascending order.
    pub const ALL: [RestDuration; 3] = [Self::Short, Self::Long, Self::Full];
}

// ===== Food Management =====

/// Returns the total number of ration-units available across all party member
/// inventories by summing the `IsFood(n)` values of every food item found.
///
/// Each `InventorySlot` is resolved against `item_db` to retrieve its
/// `ConsumableEffect`.  Items that are not found in the database (e.g. during
/// tests using a sparse DB) are silently skipped.
///
/// # Arguments
///
/// * `party`   - The party whose inventories are inspected.
/// * `item_db` - The item database used to resolve item IDs to their effects.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Party, Character, Sex, Alignment, Inventory, InventorySlot};
/// use antares::domain::items::ItemDatabase;
/// use antares::domain::resources::count_food_in_party;
///
/// let item_db = ItemDatabase::new();
/// let party = Party::new();
/// assert_eq!(count_food_in_party(&party, &item_db), 0);
/// ```
pub fn count_food_in_party(party: &Party, item_db: &ItemDatabase) -> u32 {
    party
        .members
        .iter()
        .flat_map(|c| c.inventory.items.iter())
        .fold(0u32, |acc, slot| {
            if let Some(item) = item_db.get_item(slot.item_id) {
                if let ItemType::Consumable(ref data) = item.item_type {
                    if let ConsumableEffect::IsFood(rations) = data.effect {
                        return acc + rations as u32;
                    }
                }
            }
            acc
        })
}

/// Consumes food for the party by removing `IsFood` items from character
/// inventories.
///
/// The function needs `total_needed = amount_per_member × members.len()` ration
/// units.  It iterates over every party member's inventory (in member order,
/// slot order) and removes items one slot at a time until the requirement is
/// satisfied.  If one member has no food, another member's surplus is used
/// (pooling / sharing).
///
/// # Arguments
///
/// * `party`            - The party whose inventories are modified.
/// * `item_db`          - Item database for resolving `ConsumableEffect`.
/// * `amount_per_member` - Number of ration units required per member.
///
/// # Returns
///
/// Returns `Ok(total_consumed)` — the total ration units removed from
/// inventories — on success.
///
/// # Errors
///
/// * [`ResourceError::NoFoodRemaining`] — not enough `IsFood` units across all
///   inventories.  No items are removed when this error is returned.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Party, Character, Sex, Alignment, InventorySlot};
/// use antares::domain::items::{ItemDatabase, Item, ItemType};
/// use antares::domain::items::types::{ConsumableData, ConsumableEffect};
/// use antares::domain::resources::consume_food;
///
/// // Build a minimal item database with one food item (id=1, IsFood(1)).
/// let mut item_db = ItemDatabase::new();
/// let food_item = Item {
///     id: 1,
///     name: "Food Ration".to_string(),
///     item_type: ItemType::Consumable(ConsumableData {
///         effect: ConsumableEffect::IsFood(1),
///         is_combat_usable: false,
///         duration_minutes: None,
///     }),
///     base_cost: 5,
///     sell_cost: 2,
///     alignment_restriction: None,
///     constant_bonus: None,
///     temporary_bonus: None,
///     spell_effect: None,
///     max_charges: 0,
///     is_cursed: false,
///     icon_path: None,
///     tags: vec![],
///     mesh_descriptor_override: None,
/// };
/// item_db.add_item(food_item).unwrap();
///
/// let mut party = Party::new();
/// let mut hero = Character::new(
///     "Hero".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// // Give hero 3 food rations (item id=1).
/// hero.inventory.add_item(1, 0).unwrap();
/// hero.inventory.add_item(1, 0).unwrap();
/// hero.inventory.add_item(1, 0).unwrap();
/// party.add_member(hero).unwrap();
///
/// let consumed = consume_food(&mut party, &item_db, 1).unwrap();
/// assert_eq!(consumed, 1);
/// assert_eq!(party.members[0].inventory.items.len(), 2);
/// ```
pub fn consume_food(
    party: &mut Party,
    item_db: &ItemDatabase,
    amount_per_member: u32,
) -> Result<u32, ResourceError> {
    let total_needed = amount_per_member * party.members.len() as u32;

    // Pre-check: count total available rations without modifying anything.
    let available = count_food_in_party(party, item_db);
    if available < total_needed {
        return Err(ResourceError::NoFoodRemaining);
    }

    // ── Pass 1: member-first consumption ──────────────────────────────────
    // Each member pays `amount_per_member` ration-units from their own
    // inventory.  We track `total_pass1_consumed` — the sum of ration values
    // of all items actually removed.  A multi-ration item (e.g. Trail Ration,
    // IsFood(3)) may provide more units than the member individually needed;
    // that overpayment counts toward other members' needs, which is accounted
    // for when computing the shortfall after this pass.
    let mut total_pass1_consumed: u32 = 0;

    for character in &mut party.members {
        let mut needed = amount_per_member;
        let mut slot_index = 0;

        while slot_index < character.inventory.items.len() && needed > 0 {
            let item_id = character.inventory.items[slot_index].item_id;
            let ration_value = ration_value_of(item_id, item_db);

            if ration_value > 0 {
                character.inventory.remove_item(slot_index);
                total_pass1_consumed += ration_value;
                needed = needed.saturating_sub(ration_value);
                // Do NOT advance slot_index — removal shifts items down.
            } else {
                slot_index += 1;
            }
        }
    }

    // The net shortfall after Pass 1 accounts for overpayments: if a
    // multi-ration item removed by one member covered more than that member's
    // individual need, those extra units offset other members' gaps.
    let shortfall = total_needed.saturating_sub(total_pass1_consumed);

    // ── Pass 2: pool surplus to cover shortfalls ───────────────────────────
    // If some members had no food of their own, consume from whoever still has
    // food items (the party pools its surplus).
    let mut total_pass2_consumed: u32 = 0;
    if shortfall > 0 {
        let mut remaining_shortfall = shortfall;
        'pool: for character in &mut party.members {
            let mut slot_index = 0;
            while slot_index < character.inventory.items.len() && remaining_shortfall > 0 {
                let item_id = character.inventory.items[slot_index].item_id;
                let ration_value = ration_value_of(item_id, item_db);

                if ration_value > 0 {
                    character.inventory.remove_item(slot_index);
                    total_pass2_consumed += ration_value;
                    remaining_shortfall = remaining_shortfall.saturating_sub(ration_value);
                    if remaining_shortfall == 0 {
                        break 'pool;
                    }
                    // Do NOT advance slot_index — removal shifts items down.
                } else {
                    slot_index += 1;
                }
            }
        }
    }

    Ok(total_pass1_consumed + total_pass2_consumed)
}

/// Returns the ration value of the item with `item_id` in `item_db`, or 0
/// if the item is not found or is not an `IsFood` consumable.
fn ration_value_of(item_id: crate::domain::types::ItemId, item_db: &ItemDatabase) -> u32 {
    if let Some(item) = item_db.get_item(item_id) {
        if let ItemType::Consumable(ref data) = item.item_type {
            if let ConsumableEffect::IsFood(rations) = data.effect {
                return rations as u32;
            }
        }
    }
    0
}

/// Checks if the party is starving (no `IsFood` items in any inventory).
///
/// Returns `true` when the total ration count across all party member inventories
/// is zero.
///
/// # Arguments
///
/// * `party`   - The party to check.
/// * `item_db` - Item database for resolving `ConsumableEffect`.
///
/// # Examples
///
/// ```
/// use antares::domain::character::Party;
/// use antares::domain::items::ItemDatabase;
/// use antares::domain::resources::check_starvation;
///
/// let item_db = ItemDatabase::new();
/// let party = Party::new();
/// // Empty party with no food items is considered starving.
/// assert!(check_starvation(&party, &item_db));
/// ```
pub fn check_starvation(party: &Party, item_db: &ItemDatabase) -> bool {
    count_food_in_party(party, item_db) == 0
}

// ===== Light Management =====

/// Consumes light for the party
///
/// Decrements the party's light counter. Used when traveling through
/// dark areas like dungeons.
///
/// # Arguments
///
/// * `party` - The party consuming light
/// * `amount` - Amount of light to consume
///
/// # Returns
///
/// Returns `Ok(amount)` if successful, or an error if there's no light.
///
/// # Examples
///
/// ```
/// use antares::domain::character::Party;
/// use antares::domain::resources::consume_light;
///
/// let mut party = Party::new();
/// party.light_units = 100;
///
/// consume_light(&mut party, 10).unwrap();
/// assert_eq!(party.light_units, 90);
/// ```
pub fn consume_light(party: &mut Party, amount: u32) -> Result<u32, ResourceError> {
    if party.light_units == 0 {
        return Err(ResourceError::NoLightRemaining);
    }

    let consumed = amount.min(party.light_units as u32);
    party.light_units = party.light_units.saturating_sub(consumed as u8);
    Ok(consumed)
}

/// Checks if the party is in darkness
///
/// Returns true if the party's light has run out.
///
/// # Examples
///
/// ```
/// use antares::domain::character::Party;
/// use antares::domain::resources::is_dark;
///
/// let mut party = Party::new();
/// party.light_units = 0;
///
/// assert!(is_dark(&party));
///
/// party.light_units = 10;
/// assert!(!is_dark(&party));
/// ```
pub fn is_dark(party: &Party) -> bool {
    party.light_units == 0
}

// ===== Rest and Recovery =====

/// Returns the number of food ration-units the party needs to begin a rest.
///
/// The rule is **1 ration per living party member**.  This must be satisfied
/// in full before rest begins; partial food is never consumed.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Party, Character, Sex, Alignment};
/// use antares::domain::resources::food_needed_to_rest;
///
/// let mut party = Party::new();
/// assert_eq!(food_needed_to_rest(&party), 0);
///
/// let hero = Character::new(
///     "Hero".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// party.add_member(hero).unwrap();
/// assert_eq!(food_needed_to_rest(&party), 1);
/// ```
pub fn food_needed_to_rest(party: &Party) -> u32 {
    party.members.len() as u32 * FOOD_PER_REST
}

/// Revives a character from unconscious if their HP is above 0.
///
/// Clears the `UNCONSCIOUS` bitflag AND removes any `ActiveCondition`
/// with `condition_id == "unconscious"` from `active_conditions`.
/// Both must be cleared together to prevent `reconcile_character_conditions`
/// from re-setting the bitflag on the next turn tick.
///
/// This function is a no-op if the character is not unconscious or if
/// `hp.current == 0`.
///
/// # Arguments
///
/// * `character` - The character to potentially revive
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment, Condition};
/// use antares::domain::conditions::{ActiveCondition, ConditionDuration};
/// use antares::domain::resources::revive_from_unconscious;
///
/// let mut c = Character::new(
///     "Hero".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// c.hp.current = 5;
/// c.conditions.add(Condition::UNCONSCIOUS);
/// c.add_condition(ActiveCondition::new(
///     "unconscious".to_string(),
///     ConditionDuration::Permanent,
/// ));
///
/// revive_from_unconscious(&mut c);
///
/// assert!(!c.conditions.has(Condition::UNCONSCIOUS));
/// ```
pub fn revive_from_unconscious(character: &mut crate::domain::character::Character) {
    use crate::domain::character::Condition;
    if character.conditions.has(Condition::UNCONSCIOUS) && character.hp.current > 0 {
        character.conditions.remove(Condition::UNCONSCIOUS);
        character.remove_condition("unconscious");
    }
}

/// Revives a dead character, restoring them to `hp` hit points.
///
/// Clears the `DEAD` bitflag, removes the `"dead"` `ActiveCondition` entry,
/// and sets `hp.current` to `hp.min(character.hp.base)`.
///
/// This function is a **no-op** if `character.conditions.is_dead()` returns
/// `false` (i.e. the character is `STONE` or `ERADICATED` — those cannot be
/// revived here and require different treatment).
///
/// The **caller** is responsible for checking campaign permadeath before
/// calling this function.
///
/// # Arguments
///
/// * `character` - The dead character to revive.
/// * `hp`        - Hit points to restore (clamped to `character.hp.base`).
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment, Condition};
/// use antares::domain::conditions::{ActiveCondition, ConditionDuration};
/// use antares::domain::resources::revive_from_dead;
///
/// let mut c = Character::new(
///     "Hero".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// c.hp.base = 20;
/// c.hp.current = 0;
/// c.conditions.add(Condition::DEAD);
/// c.add_condition(ActiveCondition::new(
///     "dead".to_string(),
///     ConditionDuration::Permanent,
/// ));
///
/// revive_from_dead(&mut c, 5);
///
/// assert!(!c.conditions.has(Condition::DEAD));
/// assert_eq!(c.hp.current, 5);
/// ```
pub fn revive_from_dead(character: &mut crate::domain::character::Character, hp: u16) {
    use crate::domain::character::Condition;
    if !character.conditions.is_dead() {
        return; // Not plain-dead (could be Stone/Eradicated) — no-op.
    }
    character.conditions.remove(Condition::DEAD);
    character.remove_condition("dead");
    let restored = hp.min(character.hp.base);
    character.hp.current = restored;
}

/// Restores one hour of HP and SP for every living party member.
///
/// This is the **pure healing tick** used by the rest-orchestration loop.
/// Food is **not** checked or consumed here — the caller must verify and
/// consume food via [`food_needed_to_rest`] and [`consume_food`] **before**
/// the first call to this function.
///
/// The caller is also responsible for:
///
/// 1. Checking for random encounters **between** calls.
/// 2. Advancing time via `GameState::advance_time(60, ...)` after each call
///    so that active-spell durations are ticked correctly.
///
/// No food consumption and no time advancement occur inside this function.
///
/// ## Healing model — cumulative target
///
/// Rather than computing `floor(base × fraction)` per tick (which rounds to
/// **zero** for characters with low base HP, e.g. `base=10` with a Full-rest
/// fraction of `1/12 ≈ 0.083`), this function computes the **cumulative HP
/// target** for the completed hour count and clamps to it:
///
/// ```text
/// target_hp = round(base × fraction_per_hour × hours_completed_after_tick)
/// ```
///
/// `hp.current` is raised to `max(current, target)` so partial healing
/// already applied by earlier ticks is never reversed.  This guarantees that
/// a Full 12-hour rest always restores 100% HP regardless of base HP magnitude.
///
/// # Arguments
///
/// * `party`                     — the party resting (modified in place)
/// * `restore_fraction_per_hour` — fraction of each character's base HP/SP
///   to restore this tick.  Use [`RestDuration::restore_fraction_per_hour`]
///   to obtain the correct value for the chosen duration:
///   - `RestDuration::Short.restore_fraction_per_hour()` → 0.125 /hr (50% over 4 h)
///   - `RestDuration::Long.restore_fraction_per_hour()`  → 0.09375/hr (75% over 8 h)
///   - `RestDuration::Full.restore_fraction_per_hour()`  → 0.08333/hr (100% over 12 h)
/// * `hours_completed_after_tick` — the number of in-game hours that will
///   have been completed **once this tick finishes** (i.e. `hours_completed + 1`
///   from `RestState`).  Used to compute the cumulative healing target.
///
/// # Returns
///
/// Always returns `Ok(())`.  The `Result` return type is kept so that
/// callers can use `?` and future error variants (e.g. a condition that
/// prevents rest) can be added without breaking call sites.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Party, Character, Sex, Alignment, InventorySlot};
/// use antares::domain::items::{ItemDatabase, Item, ItemType};
/// use antares::domain::items::types::{ConsumableData, ConsumableEffect};
/// use antares::domain::resources::{
///     food_needed_to_rest, consume_food, rest_party_hour, RestDuration,
/// };
///
/// // Build item DB with food ration (id=1).
/// let mut item_db = ItemDatabase::new();
/// let food_item = Item {
///     id: 1,
///     name: "Food Ration".to_string(),
///     item_type: ItemType::Consumable(ConsumableData {
///         effect: ConsumableEffect::IsFood(1),
///         is_combat_usable: false,
///         duration_minutes: None,
///     }),
///     base_cost: 5,
///     sell_cost: 2,
///     alignment_restriction: None,
///     constant_bonus: None,
///     temporary_bonus: None,
///     spell_effect: None,
///     max_charges: 0,
///     is_cursed: false,
///     icon_path: None,
///     tags: vec![],
///     mesh_descriptor_override: None,
/// };
/// item_db.add_item(food_item).unwrap();
///
/// let mut party = Party::new();
/// let mut hero = Character::new(
///     "Hero".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// // Low base HP — previously truncated to 0 per tick with as u16.
/// hero.hp.base = 10;
/// hero.hp.current = 0;
/// hero.sp.base = 10;
/// hero.sp.current = 0;
/// // Give hero enough food rations for a rest.
/// hero.inventory.add_item(1, 0).unwrap();
/// party.add_member(hero).unwrap();
///
/// // Pay for rest upfront, then tick all 12 hours.
/// let duration = RestDuration::Full;
/// let needed = food_needed_to_rest(&party);
/// consume_food(&mut party, &item_db, needed).unwrap();
/// for hour in 1..=duration.hours() {
///     rest_party_hour(&mut party, duration.restore_fraction_per_hour(), hour).unwrap();
/// }
/// // Full 12-hour rest must always restore 100% HP regardless of base HP size.
/// assert_eq!(party.members[0].hp.current, party.members[0].hp.base);
/// assert_eq!(party.members[0].sp.current, party.members[0].sp.base);
/// ```
pub fn rest_party_hour(
    party: &mut Party,
    restore_fraction_per_hour: f32,
    hours_completed_after_tick: u32,
) -> Result<(), ResourceError> {
    // Food was already consumed by the caller before rest began.
    // This function is a pure healing tick — no food check, no consumption.

    // Restore one hour of HP and SP for each living party member.
    for character in &mut party.members {
        // Skip characters with a fatal condition; unconscious characters are
        // healed and revived below once their HP rises above 0.
        if character.conditions.is_fatal() {
            continue;
        }

        // Compute cumulative heal target for this tick using floating-point
        // arithmetic, then round to the nearest integer.  Taking the target
        // relative to the *start* of rest (not the per-tick delta) avoids
        // accumulated rounding error and guarantees correct totals even for
        // characters with very low base HP (e.g. base=8 with Full fraction
        // 1/12 ≈ 0.083 would truncate to 0 per tick with naive `as u16`).
        let hp_target = ((character.hp.base as f32
            * restore_fraction_per_hour
            * hours_completed_after_tick as f32)
            .round() as u16)
            .min(character.hp.base);
        character.hp.current = character.hp.current.max(hp_target);

        let sp_target = ((character.sp.base as f32
            * restore_fraction_per_hour
            * hours_completed_after_tick as f32)
            .round() as u16)
            .min(character.sp.base);
        character.sp.current = character.sp.current.max(sp_target);

        // Revive from unconscious once HP is above 0.
        revive_from_unconscious(character);

        // Tick minute-based conditions and timed stat boosts for one hour (60 minutes).
        for _ in 0..60 {
            character.tick_conditions_minute();
            character.tick_timed_stat_boosts_minute();
        }
    }

    Ok(())
}

/// Rests the party for the given number of hours, restoring HP and SP.
///
/// Food is consumed **upfront** before any healing occurs.  The party needs
/// exactly [`food_needed_to_rest`] rations (1 per member).  If there is not
/// enough food the function returns [`ResourceError::TooHungryToRest`] and
/// the party's food, HP, and SP are left unchanged.
///
/// **Time is NOT advanced inside this function.** The caller is responsible
/// for advancing game time (e.g. via `GameState::advance_time(hours * 60, ...)`)
/// so that active-spell durations and merchant restocking are handled correctly.
///
/// # Arguments
///
/// * `party` - The party resting (modified in place)
/// * `hours` - Number of hours to rest
///
/// # Returns
///
/// Returns `Ok(())` on success, or [`ResourceError::TooHungryToRest`] when
/// the party does not have enough food to rest.
///
/// # Errors
///
/// * [`ResourceError::TooHungryToRest`] — total `IsFood` ration-units across
///   all character inventories is less than `party.members.len()`.
///   No items are consumed and no healing is applied.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Party, Character, Sex, Alignment};
/// use antares::domain::items::{ItemDatabase, Item, ItemType};
/// use antares::domain::items::types::{ConsumableData, ConsumableEffect};
/// use antares::domain::resources::rest_party;
///
/// // Build item DB with food ration (id=1).
/// let mut item_db = ItemDatabase::new();
/// let food_item = Item {
///     id: 1,
///     name: "Food Ration".to_string(),
///     item_type: ItemType::Consumable(ConsumableData {
///         effect: ConsumableEffect::IsFood(1),
///         is_combat_usable: false,
///         duration_minutes: None,
///     }),
///     base_cost: 5,
///     sell_cost: 2,
///     alignment_restriction: None,
///     constant_bonus: None,
///     temporary_bonus: None,
///     spell_effect: None,
///     max_charges: 0,
///     is_cursed: false,
///     icon_path: None,
///     tags: vec![],
///     mesh_descriptor_override: None,
/// };
/// item_db.add_item(food_item).unwrap();
///
/// let mut party = Party::new();
/// let mut character = Character::new(
///     "Hero".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// character.hp.current = 5;
/// character.hp.base = 20;
/// // Give 1 ration — enough for 1 member.
/// character.inventory.add_item(1, 0).unwrap();
/// party.add_member(character).unwrap();
///
/// rest_party(&mut party, &item_db, 12).unwrap();
///
/// assert_eq!(party.members[0].hp.current, 20); // fully healed after 12 hours
/// assert_eq!(party.members[0].inventory.items.len(), 0); // ration consumed
/// ```
pub fn rest_party(
    party: &mut Party,
    item_db: &ItemDatabase,
    hours: u32,
) -> Result<(), ResourceError> {
    // Upfront food check: every member needs FOOD_PER_REST rations.
    // If the party cannot fully pay, refuse rest — no food consumed, no healing.
    let needed = food_needed_to_rest(party);
    if count_food_in_party(party, item_db) < needed {
        return Err(ResourceError::TooHungryToRest);
    }

    // Consume all required food now (from inventories), before any healing occurs.
    consume_food(party, item_db, FOOD_PER_REST)?;

    // Calculate total minutes of rest for condition ticking.
    let total_minutes = hours * 60;

    // Restore HP and SP for each party member.
    for character in &mut party.members {
        // Skip characters with a fatal condition; unconscious characters are
        // healed and revived below once their HP rises above 0.
        if character.conditions.is_fatal() {
            continue;
        }

        // Compute total HP to restore using round() to avoid truncation to zero
        // for characters with low base HP (e.g. base=10 with HP_RESTORE_RATE=1/12
        // gives 0.833/hr which `as u16` truncates to 0 every tick).
        let hp_to_restore = ((character.hp.base as f32 * HP_RESTORE_RATE * hours as f32).round()
            as u16)
            .min(character.hp.base);
        character.hp.current = (character.hp.current + hp_to_restore).min(character.hp.base);

        // Restore SP proportional to hours rested.
        let sp_to_restore = ((character.sp.base as f32 * SP_RESTORE_RATE * hours as f32).round()
            as u16)
            .min(character.sp.base);
        character.sp.current = (character.sp.current + sp_to_restore).min(character.sp.base);

        // Revive from unconscious once HP is above 0.
        revive_from_unconscious(character);

        // Tick conditions for the full duration of rest.
        for _ in 0..total_minutes {
            character.tick_conditions_minute();
        }
    }

    // NOTE: Time is NOT advanced here. The caller (GameState::rest_party) must
    // call self.advance_time(hours * 60, templates) to tick active-spell
    // durations and trigger merchant restocking.

    Ok(())
}

/// Applies starvation damage to the party
///
/// When the party is out of food, each member takes damage over time.
/// This should be called periodically (e.g., every hour or day).
///
/// # Arguments
///
/// * `party` - The party suffering from starvation
/// * `damage_per_member` - HP damage to apply to each member
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Party, Character, Sex, Alignment};
/// use antares::domain::resources::{apply_starvation_damage, check_starvation};
/// use antares::domain::items::ItemDatabase;
///
/// let mut party = Party::new();
/// let mut character = Character::new(
///     "Hero".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// character.hp.current = 20;
/// party.add_member(character).unwrap();
///
/// apply_starvation_damage(&mut party, 5);
/// assert_eq!(party.members[0].hp.current, 15);
/// ```
pub fn apply_starvation_damage(party: &mut Party, damage_per_member: u16) {
    for character in &mut party.members {
        // Skip already dead characters
        if character.conditions.is_fatal() {
            continue;
        }

        character.hp.current = character.hp.current.saturating_sub(damage_per_member);

        // If HP drops to 0, character dies.
        // Both the bitflag AND an ActiveCondition are set so that the
        // data-driven condition system and the bitflag-based combat checks
        // stay in sync (same pattern used in apply_damage).
        if character.hp.current == 0 {
            use crate::domain::conditions::{ActiveCondition, ConditionDuration};
            character
                .conditions
                .add(crate::domain::character::Condition::DEAD);
            character.add_condition(ActiveCondition::new(
                "dead".to_string(),
                ConditionDuration::Permanent,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, Condition, Sex};
    use crate::domain::items::types::{ConsumableData, ConsumableEffect};
    use crate::domain::items::{Item, ItemDatabase, ItemType};

    /// Build a minimal `ItemDatabase` containing a single "Food Ration" item
    /// with id=1 that provides `IsFood(1)`.
    fn make_food_db() -> ItemDatabase {
        let mut db = ItemDatabase::new();
        db.add_item(Item {
            id: 1,
            name: "Food Ration".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::IsFood(1),
                is_combat_usable: false,
                duration_minutes: None,
            }),
            base_cost: 5,
            sell_cost: 2,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        })
        .unwrap();
        db
    }

    /// Build a minimal `ItemDatabase` containing a "Trail Ration" item
    /// with id=2 that provides `IsFood(3)` — used for multi-ration tests.
    fn make_trail_ration_db() -> ItemDatabase {
        let mut db = make_food_db();
        db.add_item(Item {
            id: 2,
            name: "Trail Ration".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::IsFood(3),
                is_combat_usable: false,
                duration_minutes: None,
            }),
            base_cost: 12,
            sell_cost: 4,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        })
        .unwrap();
        db
    }

    /// Create a 3-member test party.  No food is added to inventories — callers
    /// add food items themselves to exercise exactly the scenario they want.
    fn create_test_party() -> Party {
        let mut party = Party::new();
        for i in 0..3 {
            let mut character = Character::new(
                format!("Hero{}", i),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            character.hp.base = 20;
            character.hp.current = 20;
            character.sp.base = 10;
            character.sp.current = 10;
            party.add_member(character).unwrap();
        }
        party
    }

    /// Give each member of `party` the specified number of food ration items
    /// (item id=1, IsFood(1)).
    fn give_food(party: &mut Party, rations_per_member: usize) {
        for member in &mut party.members {
            for _ in 0..rations_per_member {
                member.inventory.add_item(1, 0).unwrap();
            }
        }
    }

    // ===== Food Tests =====

    #[test]
    fn test_consume_food() {
        let item_db = make_food_db();
        let mut party = create_test_party();
        // Give each of the 3 members 4 rations.
        give_food(&mut party, 4);

        // Consuming 1 ration per member = 3 total consumed.
        let consumed = consume_food(&mut party, &item_db, 1).unwrap();
        assert_eq!(consumed, 3, "consume_food must consume 1 ration per member");
        // Each member should now have 3 rations left.
        for member in &party.members {
            assert_eq!(
                member.inventory.items.len(),
                3,
                "each member should have 3 rations remaining"
            );
        }
    }

    #[test]
    fn test_consume_food_not_enough() {
        let item_db = make_food_db();
        let mut party = create_test_party();
        // Give only 2 rations total (split: member 0 gets 1, member 1 gets 1).
        party.members[0].inventory.add_item(1, 0).unwrap();
        party.members[1].inventory.add_item(1, 0).unwrap();
        // 3 members need 3 rations — only 2 available.

        let result = consume_food(&mut party, &item_db, 1);
        assert!(
            matches!(result, Err(ResourceError::NoFoodRemaining)),
            "consume_food must return NoFoodRemaining when not enough food"
        );
        // Inventories must be untouched on failure.
        assert_eq!(
            party.members[0].inventory.items.len(),
            1,
            "inventory must be unchanged on failure"
        );
        assert_eq!(
            party.members[1].inventory.items.len(),
            1,
            "inventory must be unchanged on failure"
        );
    }

    #[test]
    fn test_check_starvation() {
        let item_db = make_food_db();
        let mut party = create_test_party();

        // No food items — starving.
        assert!(check_starvation(&party, &item_db));

        // Add a food ration to first member.
        party.members[0].inventory.add_item(1, 0).unwrap();
        assert!(!check_starvation(&party, &item_db));
    }

    #[test]
    fn test_count_food_in_party_empty() {
        let item_db = make_food_db();
        let party = create_test_party();
        assert_eq!(count_food_in_party(&party, &item_db), 0);
    }

    #[test]
    fn test_count_food_in_party_multiple_members() {
        let item_db = make_food_db();
        let mut party = create_test_party();
        // 3 members × 2 rations each = 6 total.
        give_food(&mut party, 2);
        assert_eq!(count_food_in_party(&party, &item_db), 6);
    }

    #[test]
    fn test_count_food_multi_ration_item() {
        let item_db = make_trail_ration_db();
        let mut party = create_test_party();
        // Give member 0 one Trail Ration (IsFood(3)).
        party.members[0].inventory.add_item(2, 0).unwrap();
        assert_eq!(count_food_in_party(&party, &item_db), 3);
    }

    #[test]
    fn test_consume_food_sharing_across_members() {
        let item_db = make_food_db();
        let mut party = create_test_party();
        // Member 0 has 3 rations, members 1 and 2 have none.
        for _ in 0..3 {
            party.members[0].inventory.add_item(1, 0).unwrap();
        }

        // 3 members need 3 rations total — all from member 0's inventory.
        let consumed = consume_food(&mut party, &item_db, 1).unwrap();
        assert_eq!(consumed, 3, "all 3 rations consumed via sharing");
        assert_eq!(
            party.members[0].inventory.items.len(),
            0,
            "member 0 inventory emptied"
        );
    }

    #[test]
    fn test_consume_food_trail_ration_counts_as_three() {
        let item_db = make_trail_ration_db();
        let mut party = create_test_party();
        // Give member 0 one Trail Ration (IsFood(3)) — satisfies all 3 members.
        party.members[0].inventory.add_item(2, 0).unwrap();

        let consumed = consume_food(&mut party, &item_db, 1).unwrap();
        assert_eq!(
            consumed, 3,
            "trail ration provides 3 ration units for 3 members"
        );
        assert_eq!(
            party.members[0].inventory.items.len(),
            0,
            "trail ration slot removed"
        );
    }

    // ===== Light Tests =====

    #[test]
    fn test_consume_light() {
        let mut party = create_test_party();
        party.light_units = 100;

        consume_light(&mut party, 10).unwrap();
        assert_eq!(party.light_units, 90);

        consume_light(&mut party, 50).unwrap();
        assert_eq!(party.light_units, 40);
    }

    #[test]
    fn test_consume_light_when_dark() {
        let mut party = create_test_party();
        party.light_units = 0;

        let result = consume_light(&mut party, 10);
        assert!(matches!(result, Err(ResourceError::NoLightRemaining)));
    }

    #[test]
    fn test_is_dark() {
        let mut party = create_test_party();
        party.light_units = 0;

        assert!(is_dark(&party));

        party.light_units = 1;
        assert!(!is_dark(&party));
    }

    // ===== rest_party() Tests =====

    #[test]
    fn test_rest_restores_hp() {
        let item_db = make_food_db();
        let mut party = create_test_party();
        party.members[0].hp.current = 0; // Depleted HP
                                         // Give all 3 members food so rest succeeds.
        give_food(&mut party, 3);

        rest_party(&mut party, &item_db, 12).unwrap();

        // After 12 hours, should be fully healed (base is 20).
        assert_eq!(party.members[0].hp.current, 20);
    }

    #[test]
    fn test_rest_restores_sp() {
        let item_db = make_food_db();
        let mut party = create_test_party();
        party.members[0].sp.current = 0; // Depleted SP
        give_food(&mut party, 3);

        rest_party(&mut party, &item_db, 12).unwrap();

        // After 12 hours, should be fully restored (base is 10).
        assert_eq!(party.members[0].sp.current, 10);
    }

    /// A character at 0 HP with no explicit DEAD/UNCONSCIOUS condition
    /// (the normal state after combat before a death mechanic is added)
    /// must be fully healed by a 12-hour rest.
    #[test]
    fn test_rest_heals_zero_hp_no_condition() {
        let item_db = make_food_db();
        let mut party = Party::new();
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Small base HP — previously broken by truncation to 0 per tick.
        hero.hp.base = 10;
        hero.hp.current = 0;
        hero.sp.base = 8;
        hero.sp.current = 0;
        // No DEAD condition — character has 0 HP but is not flagged dead.
        // Give 1 food ration (sufficient for 1 member).
        hero.inventory.add_item(1, 0).unwrap();
        party.add_member(hero).unwrap();

        rest_party(&mut party, &item_db, 12).unwrap();

        assert_eq!(
            party.members[0].hp.current, 10,
            "0-HP character with no dead condition must be fully healed after 12h rest"
        );
        assert_eq!(
            party.members[0].sp.current, 8,
            "0-HP character with no dead condition must have SP fully restored after 12h rest"
        );
    }

    /// `rest_party()` must NOT advance game time; that is the caller's responsibility.
    #[test]
    fn test_rest_party_no_longer_advances_time() {
        let item_db = make_food_db();
        let mut party = create_test_party();
        give_food(&mut party, 3);

        // If this compiles with (party, &item_db, hours) it confirms the signature is correct.
        let result = rest_party(&mut party, &item_db, 12);
        assert!(
            result.is_ok(),
            "rest_party() must succeed with food available"
        );
    }

    /// `rest_party()` must consume exactly 1 ration per party member upfront.
    #[test]
    fn test_rest_consumes_food() {
        let item_db = make_food_db();
        let mut party = create_test_party(); // 3 members
                                             // Give 5 rations to member 0 (sharing satisfies all 3 members).
        for _ in 0..5 {
            party.members[0].inventory.add_item(1, 0).unwrap();
        }

        rest_party(&mut party, &item_db, 12).unwrap();

        // 3 rations consumed (1 per member), 2 remain in member 0's inventory.
        let total_remaining: usize = party.members.iter().map(|m| m.inventory.items.len()).sum();
        assert_eq!(total_remaining, 2, "3 of 5 rations consumed; 2 remain");
    }

    /// `rest_party()` must refuse when food < members.len(), leaving inventories unchanged.
    #[test]
    fn test_rest_party_fails_without_enough_food() {
        let item_db = make_food_db();
        let mut party = create_test_party(); // 3 members need 3 rations
                                             // Give only 2 rations — one short.
        party.members[0].inventory.add_item(1, 0).unwrap();
        party.members[1].inventory.add_item(1, 0).unwrap();

        let result = rest_party(&mut party, &item_db, 12);

        assert!(
            matches!(result, Err(ResourceError::TooHungryToRest)),
            "rest_party must fail with TooHungryToRest when party lacks food; got: {result:?}"
        );
        // Inventories must be untouched on failure.
        assert_eq!(
            party.members[0].inventory.items.len(),
            1,
            "food must not be consumed on failed rest"
        );
        assert_eq!(
            party.members[1].inventory.items.len(),
            1,
            "food must not be consumed on failed rest"
        );
    }

    /// `rest_party()` must refuse when food is zero.
    #[test]
    fn test_rest_party_fails_without_food() {
        let item_db = make_food_db();
        let mut party = create_test_party();
        // No food added.

        let result = rest_party(&mut party, &item_db, 12);

        assert!(
            matches!(result, Err(ResourceError::TooHungryToRest)),
            "rest_party must fail with TooHungryToRest when party has no food"
        );
    }

    #[test]
    fn test_rest_partial_hours() {
        let item_db = make_food_db();
        let mut party = create_test_party();
        party.members[0].hp.current = 0; // Depleted HP
        give_food(&mut party, 3);

        rest_party(&mut party, &item_db, 6).unwrap(); // 6 hours rest

        // 6 hours × (1/12 per hour) = 50% restoration.
        // round(20 × (1/12) × 6) = round(10.0) = 10 HP restored.
        let expected = ((20.0_f32 * HP_RESTORE_RATE * 6.0).round() as u16).min(20);
        assert_eq!(
            party.members[0].hp.current, expected,
            "6-hour rest should restore ~50% of max HP"
        );
    }

    /// Dead characters must not receive any HP restoration during rest.
    #[test]
    fn test_rest_skips_dead_characters() {
        let item_db = make_food_db();
        let mut party = create_test_party();
        party.members[0].hp.current = 0;
        party.members[0].conditions.add(Condition::DEAD);
        give_food(&mut party, 3);

        let initial_hp = party.members[0].hp.current;
        rest_party(&mut party, &item_db, 12).unwrap();

        // Dead character should not heal
        assert_eq!(
            party.members[0].hp.current, initial_hp,
            "dead characters must not gain HP during rest"
        );
    }

    // ===== food_needed_to_rest() Tests =====

    /// `food_needed_to_rest` returns 0 for an empty party.
    #[test]
    fn test_food_needed_to_rest_empty_party() {
        let party = Party::new();
        assert_eq!(food_needed_to_rest(&party), 0);
    }

    /// `food_needed_to_rest` returns exactly `members.len()` for a normal party.
    #[test]
    fn test_food_needed_to_rest_three_members() {
        let party = create_test_party(); // 3 members
        assert_eq!(food_needed_to_rest(&party), 3);
    }

    // ===== RestDuration Tests =====

    #[test]
    fn test_rest_duration_hours() {
        assert_eq!(RestDuration::Short.hours(), 4);
        assert_eq!(RestDuration::Long.hours(), 8);
        assert_eq!(RestDuration::Full.hours(), 12);
    }

    #[test]
    fn test_rest_duration_total_fraction() {
        assert!((RestDuration::Short.total_restore_fraction() - 0.50).abs() < 0.001);
        assert!((RestDuration::Long.total_restore_fraction() - 0.75).abs() < 0.001);
        assert!((RestDuration::Full.total_restore_fraction() - 1.00).abs() < 0.001);
    }

    #[test]
    fn test_rest_duration_per_hour_fraction_totals() {
        for d in RestDuration::ALL {
            let total = d.restore_fraction_per_hour() * d.hours() as f32;
            assert!(
                (total - d.total_restore_fraction()).abs() < 0.001,
                "{d:?}: per-hour × hours should equal total fraction"
            );
        }
    }

    #[test]
    fn test_rest_duration_from_hours() {
        assert_eq!(RestDuration::from_hours(4), Some(RestDuration::Short));
        assert_eq!(RestDuration::from_hours(8), Some(RestDuration::Long));
        assert_eq!(RestDuration::from_hours(12), Some(RestDuration::Full));
        assert_eq!(RestDuration::from_hours(6), None);
        assert_eq!(RestDuration::from_hours(0), None);
    }

    // ===== rest_party_hour() Tests =====

    /// A Full 12-hour rest must fully restore HP and SP.
    /// Food is paid upfront by the caller — rest_party_hour does not touch food.
    #[test]
    fn test_full_rest_heals_in_12_hours() {
        let item_db = make_food_db();
        let mut party = Party::new();
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.base = 120;
        hero.hp.current = 0;
        hero.sp.base = 60;
        hero.sp.current = 0;
        // Add enough food rations for 1 member.
        for _ in 0..5 {
            hero.inventory.add_item(1, 0).unwrap();
        }
        party.add_member(hero).unwrap();

        let duration = RestDuration::Full;
        let needed = food_needed_to_rest(&party);
        consume_food(&mut party, &item_db, needed).unwrap();
        for hour in 1..=duration.hours() {
            rest_party_hour(&mut party, duration.restore_fraction_per_hour(), hour).unwrap();
        }

        assert_eq!(
            party.members[0].hp.current, party.members[0].hp.base,
            "Full rest must fully restore HP"
        );
        assert_eq!(
            party.members[0].sp.current, party.members[0].sp.base,
            "Full rest must fully restore SP"
        );
    }

    /// A Full rest must heal a character with very low base HP (e.g. 8 or 10)
    /// to 100%.  Previously the per-tick `as u16` truncation rounded the
    /// fractional amount to 0 every hour, leaving the character unhealed.
    #[test]
    fn test_full_rest_heals_low_base_hp() {
        let item_db = make_food_db();
        for base_hp in [8u16, 10, 11] {
            let mut party = Party::new();
            let mut hero = Character::new(
                "Hero".to_string(),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            hero.hp.base = base_hp;
            hero.hp.current = 0;
            hero.sp.base = base_hp;
            hero.sp.current = 0;
            // Give 5 rations.
            for _ in 0..5 {
                hero.inventory.add_item(1, 0).unwrap();
            }
            party.add_member(hero).unwrap();

            let duration = RestDuration::Full;
            let needed = food_needed_to_rest(&party);
            consume_food(&mut party, &item_db, needed).unwrap();
            for hour in 1..=duration.hours() {
                rest_party_hour(&mut party, duration.restore_fraction_per_hour(), hour).unwrap();
            }

            assert_eq!(
                party.members[0].hp.current, base_hp,
                "Full rest must restore 100% HP for base_hp={base_hp}"
            );
            assert_eq!(
                party.members[0].sp.current, base_hp,
                "Full rest must restore 100% SP for base_hp={base_hp}"
            );
        }
    }

    /// A Short 4-hour rest must restore 50% HP and SP.
    #[test]
    fn test_short_rest_heals_50_percent() {
        let item_db = make_food_db();
        let mut party = Party::new();
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.base = 100;
        hero.hp.current = 0;
        hero.sp.base = 100;
        hero.sp.current = 0;
        for _ in 0..5 {
            hero.inventory.add_item(1, 0).unwrap();
        }
        party.add_member(hero).unwrap();

        let duration = RestDuration::Short;
        let needed = food_needed_to_rest(&party);
        consume_food(&mut party, &item_db, needed).unwrap();
        for hour in 1..=duration.hours() {
            rest_party_hour(&mut party, duration.restore_fraction_per_hour(), hour).unwrap();
        }

        let hp = party.members[0].hp.current;
        assert!(
            (48..=52).contains(&hp),
            "Short rest must restore ~50% HP, got {hp}"
        );
    }

    /// A Long 8-hour rest must restore 75% HP and SP.
    #[test]
    fn test_long_rest_heals_75_percent() {
        let item_db = make_food_db();
        let mut party = Party::new();
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.base = 100;
        hero.hp.current = 0;
        hero.sp.base = 100;
        hero.sp.current = 0;
        for _ in 0..5 {
            hero.inventory.add_item(1, 0).unwrap();
        }
        party.add_member(hero).unwrap();

        let duration = RestDuration::Long;
        let needed = food_needed_to_rest(&party);
        consume_food(&mut party, &item_db, needed).unwrap();
        for hour in 1..=duration.hours() {
            rest_party_hour(&mut party, duration.restore_fraction_per_hour(), hour).unwrap();
        }

        let hp = party.members[0].hp.current;
        // Cumulative target at hour 8: round(100 × 0.09375 × 8) = round(75.0) = 75.
        assert!(
            (74..=76).contains(&hp),
            "Long rest must restore ~75% HP, got {hp}"
        );
    }

    /// `rest_party_hour` does not consume food items — inventories are unchanged after a call.
    #[test]
    fn test_rest_party_hour_does_not_consume_food() {
        let mut party = Party::new();
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.base = 20;
        hero.hp.current = 0;
        // Give 3 rations — rest_party_hour must not touch them.
        hero.inventory.add_item(1, 0).unwrap();
        hero.inventory.add_item(1, 0).unwrap();
        hero.inventory.add_item(1, 0).unwrap();
        party.add_member(hero).unwrap();

        rest_party_hour(
            &mut party,
            RestDuration::Full.restore_fraction_per_hour(),
            1,
        )
        .unwrap();

        assert_eq!(
            party.members[0].inventory.items.len(),
            3,
            "rest_party_hour must not consume food items"
        );
    }

    /// `rest_party_hour` succeeds even when inventories have no food (food already paid upfront).
    #[test]
    fn test_rest_party_hour_succeeds_without_food() {
        let mut party = Party::new();
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.base = 20;
        hero.hp.current = 0;
        // No food items — food was already consumed by caller.
        party.add_member(hero).unwrap();

        let result = rest_party_hour(
            &mut party,
            RestDuration::Full.restore_fraction_per_hour(),
            1,
        );
        assert!(
            result.is_ok(),
            "rest_party_hour must succeed when food is zero — food was already paid upfront"
        );
    }

    /// 4 hours with Short duration must restore the same as calling
    /// `rest_party_hour` 4 times with `Short.restore_fraction_per_hour()`.
    #[test]
    fn test_partial_rest_heals_proportionally() {
        let item_db = make_food_db();
        let mut party = Party::new();
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.base = 120;
        hero.hp.current = 0;
        hero.sp.base = 120;
        hero.sp.current = 0;
        for _ in 0..5 {
            hero.inventory.add_item(1, 0).unwrap();
        }
        party.add_member(hero).unwrap();

        let duration = RestDuration::Short; // 4 hours → 50%
        let needed = food_needed_to_rest(&party);
        consume_food(&mut party, &item_db, needed).unwrap();
        for hour in 1..=duration.hours() {
            rest_party_hour(&mut party, duration.restore_fraction_per_hour(), hour).unwrap();
        }

        // Cumulative target at hour 4: round(120 × 0.125 × 4) = round(60.0) = 60 (50% of 120).
        let expected_hp = (120_f32 * duration.total_restore_fraction()).round() as u16;
        let expected_sp = (120_f32 * duration.total_restore_fraction()).round() as u16;

        assert_eq!(
            party.members[0].hp.current, expected_hp,
            "Short rest must restore 50% HP (expected {expected_hp})"
        );
        assert_eq!(
            party.members[0].sp.current, expected_sp,
            "Short rest must restore 50% SP (expected {expected_sp})"
        );
    }

    /// Characters with a fatal condition must not receive healing from
    /// `rest_party_hour()`.
    #[test]
    fn test_rest_party_hour_skips_dead_characters() {
        let mut party = Party::new();
        let mut dead_hero = Character::new(
            "Dead".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        dead_hero.hp.base = 20;
        dead_hero.hp.current = 0;
        dead_hero.conditions.add(Condition::DEAD);
        party.add_member(dead_hero).unwrap();

        rest_party_hour(
            &mut party,
            RestDuration::Full.restore_fraction_per_hour(),
            1,
        )
        .unwrap();

        assert_eq!(
            party.members[0].hp.current, 0,
            "dead character must not gain HP from rest_party_hour"
        );
    }

    /// An unconscious character at 0 HP must be healed by `rest_party_hour`
    /// and the UNCONSCIOUS condition must be cleared once HP > 0.
    #[test]
    fn test_rest_party_hour_revives_unconscious_character() {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut party = Party::new();
        let mut hero = Character::new(
            "Out".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.base = 20;
        hero.hp.current = 0;
        hero.conditions.add(Condition::UNCONSCIOUS);
        hero.add_condition(ActiveCondition::new(
            "unconscious".to_string(),
            ConditionDuration::Permanent,
        ));
        party.add_member(hero).unwrap();

        // Run full 12-hour rest
        let frac = RestDuration::Full.restore_fraction_per_hour();
        for hour in 1..=RestDuration::Full.hours() {
            rest_party_hour(&mut party, frac, hour).unwrap();
        }

        assert_eq!(
            party.members[0].hp.current, 20,
            "unconscious character must be fully healed after full rest"
        );
        assert!(
            !party.members[0].conditions.has(Condition::UNCONSCIOUS),
            "UNCONSCIOUS condition must be cleared once HP > 0"
        );
        assert!(
            party.members[0]
                .active_conditions
                .iter()
                .all(|c| c.condition_id != "unconscious"),
            "active_conditions must not contain 'unconscious' after revival"
        );
    }

    /// A character at 0 HP with no DEAD or UNCONSCIOUS condition must be
    /// healed by `rest_party_hour()` — this is the normal post-combat state
    /// before an explicit death mechanic is introduced.
    #[test]
    fn test_rest_party_hour_heals_zero_hp_no_condition() {
        let mut party = Party::new();
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.base = 10;
        hero.hp.current = 0;
        // conditions defaults to FINE (0) — no DEAD or UNCONSCIOUS bit set.
        party.add_member(hero).unwrap();

        let frac = RestDuration::Full.restore_fraction_per_hour();
        for hour in 1..=RestDuration::Full.hours() {
            rest_party_hour(&mut party, frac, hour).unwrap();
        }

        assert_eq!(
            party.members[0].hp.current, 10,
            "0-HP character with no dead condition must be fully healed after 12 hour ticks"
        );
    }

    // ===== ResourceError Variant Tests =====

    #[test]
    fn test_resource_error_cannot_rest_with_active_encounter_display() {
        let err = ResourceError::CannotRestWithActiveEncounter;
        let msg = err.to_string();
        assert!(
            msg.contains("encounter"),
            "CannotRestWithActiveEncounter message must mention encounter; got: {msg}"
        );
    }

    #[test]
    fn test_resource_error_rest_interrupted_display() {
        let err = ResourceError::RestInterrupted { hours_completed: 3 };
        let msg = err.to_string();
        assert!(
            msg.contains('3'),
            "RestInterrupted message must include hours_completed; got: {msg}"
        );
    }

    #[test]
    fn test_resource_error_rest_interrupted_hours_field() {
        match (ResourceError::RestInterrupted { hours_completed: 7 }) {
            ResourceError::RestInterrupted { hours_completed } => {
                assert_eq!(hours_completed, 7);
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    // ===== apply_starvation_damage() Tests =====

    #[test]
    fn test_apply_starvation_damage() {
        let mut party = create_test_party();
        party.members[0].hp.current = 20;

        apply_starvation_damage(&mut party, 5);
        assert_eq!(party.members[0].hp.current, 15);

        apply_starvation_damage(&mut party, 5);
        assert_eq!(party.members[0].hp.current, 10);
    }

    #[test]
    fn test_starvation_kills_character() {
        let mut party = create_test_party();
        party.members[0].hp.current = 3;

        apply_starvation_damage(&mut party, 5);

        assert_eq!(party.members[0].hp.current, 0);
        assert!(party.members[0].conditions.is_fatal());
    }

    /// `apply_starvation_damage` must set both the `DEAD` bitflag AND push an
    /// `ActiveCondition("dead")` entry when a character's HP reaches 0.
    #[test]
    fn test_apply_starvation_damage_sets_dead_condition() {
        use crate::domain::conditions::ConditionDuration;

        let mut party = create_test_party();
        party.members[0].hp.current = 3;

        apply_starvation_damage(&mut party, 5);

        assert_eq!(party.members[0].hp.current, 0, "HP must reach 0");
        assert!(
            party.members[0]
                .conditions
                .has(crate::domain::character::Condition::DEAD),
            "DEAD bitflag must be set by apply_starvation_damage"
        );
        let has_active = party.members[0]
            .active_conditions
            .iter()
            .any(|ac| ac.condition_id == "dead");
        assert!(has_active, "ActiveCondition('dead') must be pushed");
        let entry = party.members[0]
            .active_conditions
            .iter()
            .find(|ac| ac.condition_id == "dead")
            .unwrap();
        assert_eq!(
            entry.duration,
            ConditionDuration::Permanent,
            "dead ActiveCondition must have Permanent duration"
        );
    }

    #[test]
    fn test_dead_character_skipped_in_rest() {
        let item_db = make_food_db();
        let mut party = create_test_party();
        party.members[0].hp.current = 0;
        party.members[0].conditions.add(Condition::DEAD);
        give_food(&mut party, 3);

        let initial_hp = party.members[0].hp.current;
        rest_party(&mut party, &item_db, 12).unwrap();

        // Dead character should not heal
        assert_eq!(party.members[0].hp.current, initial_hp);
    }

    /// `revive_from_unconscious` must be a no-op when the character has FINE condition.
    #[test]
    fn test_revive_from_unconscious_noop_when_fine() {
        let mut c = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.current = 10;
        // No UNCONSCIOUS flag set
        revive_from_unconscious(&mut c);
        assert!(
            c.conditions.is_fine(),
            "conditions must remain FINE after revive_from_unconscious on a healthy character"
        );
    }

    /// `revive_from_unconscious` must clear both the bitflag and the ActiveCondition entry.
    #[test]
    fn test_revive_from_unconscious_clears_both() {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut c = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.current = 5;
        c.conditions.add(Condition::UNCONSCIOUS);
        c.add_condition(ActiveCondition::new(
            "unconscious".to_string(),
            ConditionDuration::Permanent,
        ));

        revive_from_unconscious(&mut c);

        assert!(
            !c.conditions.has(Condition::UNCONSCIOUS),
            "UNCONSCIOUS bitflag must be cleared"
        );
        assert!(
            c.active_conditions
                .iter()
                .all(|ac| ac.condition_id != "unconscious"),
            "active_conditions must not contain 'unconscious' after revival"
        );
    }

    /// `rest_party` must revive an unconscious character (0 HP) and clear the condition.
    #[test]
    fn test_rest_revives_unconscious_character() {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let item_db = make_food_db();
        let mut party = Party::new();
        let mut hero = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.hp.base = 20;
        hero.hp.current = 0;
        hero.conditions.add(Condition::UNCONSCIOUS);
        hero.add_condition(ActiveCondition::new(
            "unconscious".to_string(),
            ConditionDuration::Permanent,
        ));
        // Give 1 food ration (sufficient for 1 member).
        hero.inventory.add_item(1, 0).unwrap();
        party.add_member(hero).unwrap();

        rest_party(&mut party, &item_db, 12).unwrap();

        assert!(
            party.members[0].hp.current > 0,
            "unconscious character must have HP > 0 after rest"
        );
        assert!(
            !party.members[0].conditions.has(Condition::UNCONSCIOUS),
            "UNCONSCIOUS condition must be cleared after rest revives character"
        );
    }

    /// `revive_from_dead` must clear the DEAD bitflag, remove the active
    /// condition, and set `hp.current` to the requested value.
    #[test]
    fn test_revive_from_dead_clears_both() {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut c = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.base = 20;
        c.hp.current = 0;
        c.conditions.add(Condition::DEAD);
        c.add_condition(ActiveCondition::new(
            "dead".to_string(),
            ConditionDuration::Permanent,
        ));

        revive_from_dead(&mut c, 5);

        assert!(
            !c.conditions.has(Condition::DEAD),
            "DEAD bitflag must be cleared after revive_from_dead"
        );
        assert_eq!(
            c.hp.current, 5,
            "hp.current must be set to the requested HP"
        );
        assert!(
            !c.active_conditions
                .iter()
                .any(|ac| ac.condition_id == "dead"),
            "active_conditions must not contain 'dead' after revive_from_dead"
        );
    }

    /// `revive_from_dead` must clamp restored HP to `hp.base`.
    #[test]
    fn test_revive_from_dead_clamps_to_base() {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut c = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.base = 10;
        c.hp.current = 0;
        c.conditions.add(Condition::DEAD);
        c.add_condition(ActiveCondition::new(
            "dead".to_string(),
            ConditionDuration::Permanent,
        ));

        // Request more HP than base — must clamp.
        revive_from_dead(&mut c, 50);

        assert_eq!(
            c.hp.current, 10,
            "revive_from_dead must clamp restored HP to hp.base"
        );
    }

    /// `revive_from_dead` must be a no-op when the character is STONE
    /// (STONE value ≥ STONE constant, so `is_dead()` returns false).
    #[test]
    fn test_revive_from_dead_noop_on_stone() {
        let mut c = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.base = 20;
        c.hp.current = 0;
        c.conditions.add(Condition::STONE);

        revive_from_dead(&mut c, 5);

        // HP must remain 0 — no revival
        assert_eq!(
            c.hp.current, 0,
            "revive_from_dead must be a no-op for STONE"
        );
        assert!(
            c.conditions.has(Condition::STONE),
            "STONE condition must remain unchanged"
        );
    }

    /// `revive_from_dead` must be a no-op when the character is ERADICATED
    /// (value 255, `is_dead()` returns false).
    #[test]
    fn test_revive_from_dead_noop_on_eradicated() {
        let mut c = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.base = 20;
        c.hp.current = 0;
        c.conditions.add(Condition::ERADICATED);

        revive_from_dead(&mut c, 5);

        assert_eq!(
            c.hp.current, 0,
            "revive_from_dead must be a no-op for ERADICATED"
        );
        assert!(
            c.conditions.has(Condition::ERADICATED),
            "ERADICATED condition must remain unchanged"
        );
    }

    #[test]
    fn test_rest_does_not_heal_fatal_character() {
        let item_db = make_food_db();
        let mut party = create_test_party();
        party.members[0].hp.current = 0;
        party.members[0].conditions.add(Condition::DEAD);
        give_food(&mut party, 3);

        let initial_hp = party.members[0].hp.current;
        rest_party(&mut party, &item_db, 12).unwrap();

        assert_eq!(
            party.members[0].hp.current, initial_hp,
            "character with DEAD condition must receive no HP during rest"
        );
        assert!(
            party.members[0].conditions.has(Condition::DEAD),
            "DEAD condition must remain set after rest"
        );
    }
}
