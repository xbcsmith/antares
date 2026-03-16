// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Lock state domain types and unlock functions.
//!
//! This module owns every lock-related domain type and the pure functions
//! (`try_unlock`, `try_lockpick`, `try_bash`) used to resolve them.
//! There is no Bevy, ECS, or UI code here — only `src/domain/`.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 12.10 for the unlock and
//! bash mechanical rules that govern success formulae and trap behaviour.

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::domain::character::{Character, Party};
use crate::domain::classes::ClassDatabase;
use crate::domain::items::database::ItemDatabase;
use crate::domain::types::ItemId;

// ===== Constants =====

/// Percentage added to `trap_chance` after each failed lockpick attempt.
pub const LOCKPICK_FAIL_TRAP_INCREMENT: u8 = 10;

/// Percentage added to `trap_chance` after any bash attempt (success or failure).
pub const BASH_TRAP_INCREMENT: u8 = 20;

/// Maximum value `trap_chance` can reach.
pub const TRAP_CHANCE_MAX: u8 = 90;

// ===== LockState =====

/// Runtime mutable state for a single lock instance.
///
/// `LockState` is keyed by a `lock_id: String` that matches the `lock_id`
/// field on `MapEvent::LockedDoor` or `MapEvent::LockedContainer`. It is
/// stored in `Map::lock_states` at runtime and serialised with save data so
/// that unlocked doors remain open across save/load cycles.
///
/// # Examples
///
/// ```
/// use antares::domain::world::lock::LockState;
///
/// let state = LockState::new("dungeon_gate");
/// assert!(state.is_locked);
/// assert_eq!(state.trap_chance, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockState {
    /// Unique identifier matching the `lock_id` in the map event.
    pub lock_id: String,
    /// Whether the lock is currently locked.
    pub is_locked: bool,
    /// Cumulative trap chance percentage (0–100).
    ///
    /// Starts at 0. Each failed lockpick attempt raises this by
    /// `LOCKPICK_FAIL_TRAP_INCREMENT`. Any bash attempt (success or failure)
    /// raises this by `BASH_TRAP_INCREMENT`. Capped at `TRAP_CHANCE_MAX`.
    pub trap_chance: u8,
}

impl LockState {
    /// Creates a new locked `LockState` with zero trap chance.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::lock::LockState;
    ///
    /// let state = LockState::new("gate_01");
    /// assert!(state.is_locked);
    /// assert_eq!(state.trap_chance, 0);
    /// assert_eq!(state.lock_id, "gate_01");
    /// ```
    pub fn new(lock_id: impl Into<String>) -> Self {
        Self {
            lock_id: lock_id.into(),
            is_locked: true,
            trap_chance: 0,
        }
    }

    /// Returns `true` if the lock has a non-zero trap chance.
    ///
    /// When `true`, callers should invoke `roll_trap` before any unlock
    /// attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::lock::LockState;
    ///
    /// let mut state = LockState::new("gate");
    /// assert!(!state.has_trap_risk());
    /// state.trap_chance = 10;
    /// assert!(state.has_trap_risk());
    /// ```
    pub fn has_trap_risk(&self) -> bool {
        self.trap_chance > 0
    }

    /// Increments trap chance by `delta`, capped at [`TRAP_CHANCE_MAX`].
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::lock::{LockState, TRAP_CHANCE_MAX};
    ///
    /// let mut state = LockState::new("gate");
    /// state.increment_trap_chance(50);
    /// assert_eq!(state.trap_chance, 50);
    /// state.increment_trap_chance(100);
    /// assert_eq!(state.trap_chance, TRAP_CHANCE_MAX);
    /// ```
    pub fn increment_trap_chance(&mut self, delta: u8) {
        self.trap_chance = self.trap_chance.saturating_add(delta).min(TRAP_CHANCE_MAX);
    }

    /// Marks the lock as unlocked.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::lock::LockState;
    ///
    /// let mut state = LockState::new("gate");
    /// assert!(state.is_locked);
    /// state.unlock();
    /// assert!(!state.is_locked);
    /// ```
    pub fn unlock(&mut self) {
        self.is_locked = false;
    }
}

// ===== UnlockOutcome =====

/// Result of a single unlock attempt.
///
/// Returned by [`try_unlock`], [`try_lockpick`], and [`try_bash`] to describe
/// exactly what happened so that callers can show appropriate log messages and
/// trigger side-effects.
///
/// # Examples
///
/// ```
/// use antares::domain::world::lock::{LockState, UnlockOutcome, try_unlock};
/// use antares::domain::character::Party;
/// use antares::domain::items::database::ItemDatabase;
///
/// let mut lock = LockState::new("chest");
/// let mut party = Party::new();
/// let item_db = ItemDatabase::new();
///
/// let outcome = try_unlock(&mut lock, &mut party, None, &item_db);
/// assert_eq!(outcome, UnlockOutcome::Locked { requires_key_item_id: None });
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnlockOutcome {
    /// Lock opened with the correct key item.
    OpenedWithKey {
        /// The [`ItemId`] of the key that was consumed from the party inventory.
        key_item_id: ItemId,
    },
    /// Lock picked successfully by a character.
    LockpickSuccess {
        /// Index of the picking character in the active party.
        picker_party_index: usize,
    },
    /// Lockpick attempt failed; trap chance increased.
    LockpickFailed {
        /// Index of the picking character in the active party.
        picker_party_index: usize,
        /// Updated trap chance percentage after the failure.
        new_trap_chance: u8,
    },
    /// Bash succeeded; door or container destroyed.
    BashSuccess {
        /// Index of the bashing character in the active party.
        basher_party_index: usize,
    },
    /// Bash failed; door held. Trap chance increased.
    BashFailed {
        /// Index of the bashing character in the active party.
        basher_party_index: usize,
        /// Updated trap chance percentage after the bash.
        new_trap_chance: u8,
    },
    /// A trap fired before or during the attempt.
    TrapTriggered {
        /// Hit point damage dealt to the party.
        damage: u16,
        /// Optional status effect name (future phases will populate this).
        effect: Option<String>,
    },
    /// Party lacks the key and no unlock action was attempted.
    ///
    /// Callers should prompt the player to choose Pick Lock or Bash.
    Locked {
        /// The [`ItemId`] of the required key, or `None` if no key is needed.
        requires_key_item_id: Option<ItemId>,
    },
}

// ===== try_unlock =====

/// Attempt to unlock a lock using the best available method.
///
/// Evaluation order:
/// 1. If `key_item_id` is `Some(k)` and the party carries a matching item,
///    unlock immediately and consume the key (remove it from the carrying
///    character's inventory). Returns [`UnlockOutcome::OpenedWithKey`].
/// 2. If `key_item_id` is `Some(k)` but the party lacks the key, return
///    [`UnlockOutcome::Locked`] with the required key ID.
/// 3. If `key_item_id` is `None`, the lock requires picking or bashing —
///    return [`UnlockOutcome::Locked`] with `requires_key_item_id: None`.
///
/// This function does **not** perform lockpick or bash rolls — those are
/// handled by [`try_lockpick`] and [`try_bash`] to keep responsibilities clear.
///
/// # Arguments
///
/// * `lock_state`  – Mutable runtime lock state to update on success.
/// * `party`       – Active party whose inventories are checked for the key.
/// * `key_item_id` – The item ID required to open this lock, if any.
/// * `_item_db`    – Item database (reserved for future key validation logic).
///
/// # Returns
///
/// Returns [`UnlockOutcome`] describing what happened.
///
/// # Examples
///
/// ```
/// use antares::domain::world::lock::{LockState, UnlockOutcome, try_unlock};
/// use antares::domain::character::Party;
/// use antares::domain::items::database::ItemDatabase;
///
/// let mut lock = LockState::new("chest_01");
/// let mut party = Party::new();
/// let item_db = ItemDatabase::new();
///
/// // No key in party — returns Locked
/// let outcome = try_unlock(&mut lock, &mut party, Some(42), &item_db);
/// assert_eq!(outcome, UnlockOutcome::Locked { requires_key_item_id: Some(42) });
/// assert!(lock.is_locked);
/// ```
pub fn try_unlock(
    lock_state: &mut LockState,
    party: &mut Party,
    key_item_id: Option<ItemId>,
    _item_db: &ItemDatabase,
) -> UnlockOutcome {
    if let Some(key_id) = key_item_id {
        // Walk all party member inventories looking for the matching key.
        for character in party.members.iter_mut() {
            if let Some(idx) = character
                .inventory
                .items
                .iter()
                .position(|slot| slot.item_id == key_id)
            {
                // Found — consume the key and unlock.
                character.inventory.items.remove(idx);
                lock_state.unlock();
                return UnlockOutcome::OpenedWithKey {
                    key_item_id: key_id,
                };
            }
        }
        // Key required but not present in any inventory.
        return UnlockOutcome::Locked {
            requires_key_item_id: Some(key_id),
        };
    }

    // No key required — lock must be opened by picking or bashing.
    UnlockOutcome::Locked {
        requires_key_item_id: None,
    }
}

// ===== try_lockpick =====

/// Attempt to pick a lock.
///
/// Uses the character at `picker_party_index`. The character must have the
/// `"pick_lock"` special ability (via their class definition) or the attempt
/// automatically fails with [`UnlockOutcome::LockpickFailed`].
///
/// **Success formula** (architecture Section 12.10):
///
/// - Base chance: 30 %
/// - +5 % per character level above 1
/// - +10 % bonus if class is `"robber"`
/// - Result clamped to \[5 %, 95 %\]
///
/// **Trap behaviour:**  A trap roll is performed **before** the pick attempt
/// when `trap_chance > 0`.  If the trap fires,
/// [`UnlockOutcome::TrapTriggered`] is returned immediately and no pick roll
/// is made.
///
/// **On failure:** `trap_chance` is incremented by
/// [`LOCKPICK_FAIL_TRAP_INCREMENT`].
/// **On success:** [`LockState::unlock`] is called.
///
/// # Arguments
///
/// * `lock_state`         – Mutable runtime lock state.
/// * `character`          – The character attempting to pick the lock.
/// * `picker_party_index` – Party index embedded in the returned outcome.
/// * `class_db`           – Class database for ability lookup.
/// * `rng`                – Random source.
///
/// # Returns
///
/// Returns [`UnlockOutcome`] describing what happened.
pub fn try_lockpick<R: Rng>(
    lock_state: &mut LockState,
    character: &Character,
    picker_party_index: usize,
    class_db: &ClassDatabase,
    rng: &mut R,
) -> UnlockOutcome {
    // Trap check — fires before the pick attempt.
    if lock_state.has_trap_risk() {
        if let Some(trap) = roll_trap(lock_state.trap_chance, rng) {
            return trap;
        }
    }

    // Ability check — character must have the "pick_lock" ability.
    let has_ability = class_db
        .get_class(&character.class_id)
        .map(|c| c.has_ability("pick_lock"))
        .unwrap_or(false);

    if !has_ability {
        // Auto-fail: no pick_lock ability.
        lock_state.increment_trap_chance(LOCKPICK_FAIL_TRAP_INCREMENT);
        return UnlockOutcome::LockpickFailed {
            picker_party_index,
            new_trap_chance: lock_state.trap_chance,
        };
    }

    // Success formula.
    let level_bonus = character.level.saturating_sub(1) as i32 * 5;
    let class_bonus: i32 = if character.class_id == "robber" {
        10
    } else {
        0
    };
    let success_chance = (30 + level_bonus + class_bonus).clamp(5, 95) as u32;

    let roll: u32 = rng.random_range(0..100);
    if roll < success_chance {
        lock_state.unlock();
        UnlockOutcome::LockpickSuccess { picker_party_index }
    } else {
        lock_state.increment_trap_chance(LOCKPICK_FAIL_TRAP_INCREMENT);
        UnlockOutcome::LockpickFailed {
            picker_party_index,
            new_trap_chance: lock_state.trap_chance,
        }
    }
}

// ===== try_bash =====

/// Attempt to bash open a locked door or container.
///
/// No class restriction — architecture Section 12.10 explicitly states "No
/// class restrictions".
///
/// **Success formula:**
///
/// - Base chance: 25 %
/// - +3 % per character level
/// - +5 % if character Might (`current`) ≥ 15
/// - Result clamped to \[5 %, 80 %\]
///
/// **Trap behaviour:** A trap roll is performed **before** the bash attempt
/// when `trap_chance > 0`. If the trap fires,
/// [`UnlockOutcome::TrapTriggered`] is returned immediately and the trap
/// increment does **not** occur.
///
/// **On any actual bash attempt** (success or failure): `trap_chance` is
/// incremented by [`BASH_TRAP_INCREMENT`].
/// **On success:** [`LockState::unlock`] is called.
///
/// # Arguments
///
/// * `lock_state`         – Mutable runtime lock state.
/// * `character`          – The character attempting to bash.
/// * `basher_party_index` – Party index embedded in the returned outcome.
/// * `rng`                – Random source.
///
/// # Returns
///
/// Returns [`UnlockOutcome`] describing what happened.
pub fn try_bash<R: Rng>(
    lock_state: &mut LockState,
    character: &Character,
    basher_party_index: usize,
    rng: &mut R,
) -> UnlockOutcome {
    // Trap check — fires before the bash attempt; no increment if trap fires.
    if lock_state.has_trap_risk() {
        if let Some(trap) = roll_trap(lock_state.trap_chance, rng) {
            return trap;
        }
    }

    // Success formula.
    let level_bonus = character.level as i32 * 3;
    let might_bonus: i32 = if character.stats.might.current >= 15 {
        5
    } else {
        0
    };
    let success_chance = (25 + level_bonus + might_bonus).clamp(5, 80) as u32;

    // Always increment trap chance on an actual bash attempt.
    lock_state.increment_trap_chance(BASH_TRAP_INCREMENT);

    let roll: u32 = rng.random_range(0..100);
    if roll < success_chance {
        lock_state.unlock();
        UnlockOutcome::BashSuccess { basher_party_index }
    } else {
        UnlockOutcome::BashFailed {
            basher_party_index,
            new_trap_chance: lock_state.trap_chance,
        }
    }
}

// ===== roll_trap (private) =====

/// Roll to see if a trap fires given the current `trap_chance`.
///
/// Returns `Some(UnlockOutcome::TrapTriggered { damage, effect })` if the trap
/// fires, `None` otherwise.
///
/// - The trap fires when `random_range(0..100) < trap_chance`.
/// - Damage is `1d6 × (trap_chance / 10)`, minimum 1.
/// - `effect` is `None` in Phase 1 (future phases may add status conditions).
fn roll_trap<R: Rng>(trap_chance: u8, rng: &mut R) -> Option<UnlockOutcome> {
    let roll: u32 = rng.random_range(0..100);
    if roll < trap_chance as u32 {
        let d6: u32 = rng.random_range(1..=6);
        let multiplier = (trap_chance / 10) as u32;
        let damage = (d6 * multiplier).max(1) as u16;
        Some(UnlockOutcome::TrapTriggered {
            damage,
            effect: None,
        })
    } else {
        None
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, InventorySlot, Party, Sex};
    use crate::domain::classes::ClassDatabase;
    use crate::domain::items::database::ItemDatabase;
    use rand::SeedableRng;

    // ─── helpers ─────────────────────────────────────────────────────────────

    /// Build a party with a single member who carries the given key item.
    fn make_party_with_key(key_id: ItemId) -> Party {
        let mut party = Party::new();
        let mut character = Character::new(
            "Tester".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        character.inventory.items.push(InventorySlot {
            item_id: key_id,
            charges: 0,
        });
        party.members.push(character);
        party
    }

    /// Build a single character with the given class and level.
    fn make_character(class_id: &str, level: u32) -> Character {
        let mut c = Character::new(
            "Test".to_string(),
            "human".to_string(),
            class_id.to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        c.level = level;
        c
    }

    /// Load the class database from the project-local fixture.
    fn load_class_db() -> ClassDatabase {
        ClassDatabase::load_from_file("data/classes.ron")
            .expect("data/classes.ron must exist for lock tests")
    }

    // ─── LockState tests ─────────────────────────────────────────────────────

    /// New `LockState` starts locked with `trap_chance == 0`.
    #[test]
    fn test_lock_state_new_is_locked() {
        let state = LockState::new("dungeon_gate");
        assert!(state.is_locked, "new LockState must start locked");
        assert_eq!(state.trap_chance, 0, "trap_chance must start at zero");
        assert_eq!(state.lock_id, "dungeon_gate");
    }

    /// `unlock()` sets `is_locked` to `false`.
    #[test]
    fn test_lock_state_unlock() {
        let mut state = LockState::new("gate");
        state.unlock();
        assert!(!state.is_locked, "unlock() must set is_locked to false");
    }

    /// `increment_trap_chance` cannot exceed `TRAP_CHANCE_MAX`.
    #[test]
    fn test_lock_state_increment_trap_chance_clamps() {
        let mut state = LockState::new("gate");
        state.increment_trap_chance(50);
        assert_eq!(state.trap_chance, 50);
        state.increment_trap_chance(100);
        assert_eq!(
            state.trap_chance, TRAP_CHANCE_MAX,
            "trap_chance must not exceed TRAP_CHANCE_MAX"
        );
    }

    // ─── try_unlock tests ────────────────────────────────────────────────────

    /// Party member has the correct key — `try_unlock` returns `OpenedWithKey`
    /// and removes the item from inventory.
    #[test]
    fn test_try_unlock_with_correct_key_removes_item_from_inventory() {
        let mut lock = LockState::new("chest_01");
        let mut party = make_party_with_key(42);
        let item_db = ItemDatabase::new();

        let outcome = try_unlock(&mut lock, &mut party, Some(42), &item_db);

        assert_eq!(outcome, UnlockOutcome::OpenedWithKey { key_item_id: 42 });
        assert!(!lock.is_locked, "lock must be open after key use");
        assert!(
            party.members[0].inventory.items.is_empty(),
            "key must be consumed from inventory"
        );
    }

    /// Party has a key but the wrong `ItemId` — returns `Locked`.
    #[test]
    fn test_try_unlock_with_wrong_key_returns_locked() {
        let mut lock = LockState::new("chest_02");
        let mut party = make_party_with_key(99); // wrong item id
        let item_db = ItemDatabase::new();

        let outcome = try_unlock(&mut lock, &mut party, Some(42), &item_db);

        assert_eq!(
            outcome,
            UnlockOutcome::Locked {
                requires_key_item_id: Some(42)
            }
        );
        assert!(lock.is_locked, "lock must remain locked");
        assert_eq!(
            party.members[0].inventory.items.len(),
            1,
            "wrong key must NOT be consumed"
        );
    }

    /// Lock has `key_item_id: None` — `try_unlock` returns `Locked` with
    /// `requires_key_item_id: None`, indicating pick/bash is needed.
    #[test]
    fn test_try_unlock_no_key_required_still_needs_lockpick_or_bash() {
        let mut lock = LockState::new("chest_03");
        let mut party = Party::new();
        let item_db = ItemDatabase::new();

        let outcome = try_unlock(&mut lock, &mut party, None, &item_db);

        assert_eq!(
            outcome,
            UnlockOutcome::Locked {
                requires_key_item_id: None
            }
        );
        assert!(lock.is_locked);
    }

    // ─── try_lockpick tests ──────────────────────────────────────────────────

    /// Seeded RNG eventually produces a roll below the robber's success
    /// threshold — outcome is `LockpickSuccess` and lock is unlocked.
    ///
    /// Uses a robber at level 20 (95 % chance) to ensure a success occurs
    /// within a small number of trials.
    #[test]
    fn test_try_lockpick_success_unlocks() {
        let class_db = load_class_db();
        let character = make_character("robber", 20); // 30+95+10 → capped at 95%
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let mut succeeded = false;

        for _ in 0..50 {
            let mut lock = LockState::new("door_01");
            match try_lockpick(&mut lock, &character, 0, &class_db, &mut rng) {
                UnlockOutcome::LockpickSuccess { .. } => {
                    assert!(
                        !lock.is_locked,
                        "LockpickSuccess must set is_locked to false"
                    );
                    succeeded = true;
                    break;
                }
                UnlockOutcome::LockpickFailed { .. } => {
                    assert!(lock.is_locked, "LockpickFailed must leave lock locked");
                }
                other => panic!("Unexpected outcome: {:?}", other),
            }
        }
        assert!(
            succeeded,
            "Expected at least one LockpickSuccess with 95% chance in 50 trials"
        );
    }

    /// A failed lockpick attempt increments `trap_chance` by
    /// `LOCKPICK_FAIL_TRAP_INCREMENT`.
    ///
    /// Uses a robber at level 1 (40 % success → 60 % failure) to ensure a
    /// failure is encountered within a small number of trials.
    #[test]
    fn test_try_lockpick_failure_increments_trap_chance() {
        let class_db = load_class_db();
        let character = make_character("robber", 1); // 30+0+10 = 40% success
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let mut saw_failure = false;

        for _ in 0..50 {
            let mut lock = LockState::new("door_02");
            match try_lockpick(&mut lock, &character, 1, &class_db, &mut rng) {
                UnlockOutcome::LockpickFailed {
                    new_trap_chance, ..
                } => {
                    assert_eq!(
                        new_trap_chance, LOCKPICK_FAIL_TRAP_INCREMENT,
                        "trap_chance must increase by LOCKPICK_FAIL_TRAP_INCREMENT on failure"
                    );
                    assert_eq!(lock.trap_chance, new_trap_chance);
                    saw_failure = true;
                    break;
                }
                UnlockOutcome::LockpickSuccess { .. } => {}
                other => panic!("Unexpected outcome: {:?}", other),
            }
        }
        assert!(
            saw_failure,
            "Expected at least one LockpickFailed with 60% failure chance in 50 trials"
        );
    }

    /// A class without the `pick_lock` ability always returns `LockpickFailed`
    /// regardless of the RNG roll.
    #[test]
    fn test_try_lockpick_class_without_pick_lock_always_fails() {
        let class_db = load_class_db();
        let mut lock = LockState::new("door_03");
        let character = make_character("knight", 10); // knight has no pick_lock
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);

        let outcome = try_lockpick(&mut lock, &character, 0, &class_db, &mut rng);

        assert!(
            matches!(
                outcome,
                UnlockOutcome::LockpickFailed {
                    picker_party_index: 0,
                    ..
                }
            ),
            "knight must always fail to pick a lock, got: {:?}",
            outcome
        );
        assert_eq!(lock.trap_chance, LOCKPICK_FAIL_TRAP_INCREMENT);
    }

    /// When `trap_chance == 100` the trap fires with certainty before the
    /// pick attempt — outcome is `TrapTriggered` and the lock stays locked.
    #[test]
    fn test_try_lockpick_trap_fires_before_attempt() {
        let class_db = load_class_db();
        let mut lock = LockState {
            lock_id: "door_04".to_string(),
            is_locked: true,
            trap_chance: 100, // always fires (100 is above TRAP_CHANCE_MAX but valid for testing)
        };
        let character = make_character("robber", 5);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);

        let outcome = try_lockpick(&mut lock, &character, 0, &class_db, &mut rng);

        assert!(
            matches!(outcome, UnlockOutcome::TrapTriggered { .. }),
            "trap must fire before pick when trap_chance is 100, got: {:?}",
            outcome
        );
        assert!(lock.is_locked, "lock must remain locked after trap fires");
    }

    // ─── try_bash tests ──────────────────────────────────────────────────────

    /// A successful bash unlocks the lock.
    ///
    /// Verifies that `trap_chance` is always incremented on an actual bash
    /// attempt (not interrupted by a trap).
    #[test]
    fn test_try_bash_success_unlocks() {
        let character = make_character("knight", 5);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let mut saw_success = false;

        for _ in 0..50 {
            let mut lock = LockState::new("door_05");
            match try_bash(&mut lock, &character, 0, &mut rng) {
                UnlockOutcome::BashSuccess { .. } => {
                    assert!(!lock.is_locked, "BashSuccess must unlock the lock");
                    assert_eq!(
                        lock.trap_chance, BASH_TRAP_INCREMENT,
                        "trap_chance must be incremented on any bash attempt"
                    );
                    saw_success = true;
                    break;
                }
                UnlockOutcome::BashFailed {
                    new_trap_chance, ..
                } => {
                    assert_eq!(
                        new_trap_chance, BASH_TRAP_INCREMENT,
                        "trap_chance must be incremented even on a failed bash"
                    );
                    assert!(lock.is_locked);
                }
                other => panic!("Unexpected outcome: {:?}", other),
            }
        }
        assert!(
            saw_success,
            "Expected at least one BashSuccess with 40% chance in 50 trials"
        );
    }

    /// A failed bash increments `trap_chance` by `BASH_TRAP_INCREMENT`.
    #[test]
    fn test_try_bash_failure_increments_trap_chance() {
        let character = make_character("knight", 1); // 25+3 = 28% chance
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let mut saw_failure = false;

        for _ in 0..50 {
            let mut lock = LockState::new("door_06");
            match try_bash(&mut lock, &character, 2, &mut rng) {
                UnlockOutcome::BashFailed {
                    basher_party_index: 2,
                    new_trap_chance,
                } => {
                    assert_eq!(new_trap_chance, BASH_TRAP_INCREMENT);
                    assert_eq!(lock.trap_chance, BASH_TRAP_INCREMENT);
                    assert!(lock.is_locked);
                    saw_failure = true;
                    break;
                }
                UnlockOutcome::BashSuccess { .. } => {}
                other => panic!("Unexpected outcome: {:?}", other),
            }
        }
        assert!(
            saw_failure,
            "Expected at least one BashFailed with 72% failure chance in 50 trials"
        );
    }

    /// Any class — including a sorcerer — can attempt a bash.
    #[test]
    fn test_try_bash_no_class_restriction() {
        let mut lock = LockState::new("door_07");
        let character = make_character("sorcerer", 3);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);

        let outcome = try_bash(&mut lock, &character, 0, &mut rng);

        assert!(
            matches!(
                outcome,
                UnlockOutcome::BashSuccess { .. }
                    | UnlockOutcome::BashFailed { .. }
                    | UnlockOutcome::TrapTriggered { .. }
            ),
            "sorcerer must be able to attempt a bash, got: {:?}",
            outcome
        );
    }

    // ─── Map lock_state tests ────────────────────────────────────────────────

    /// `init_lock_states` populates `lock_states` from `LockedDoor` events.
    #[test]
    fn test_map_init_lock_states_populates_from_events() {
        use crate::domain::types::Position;
        use crate::domain::world::types::{Map, MapEvent};

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        map.add_event(
            Position::new(2, 2),
            MapEvent::LockedDoor {
                name: "Gate A".to_string(),
                lock_id: "gate_a".to_string(),
                key_item_id: None,
                initial_trap_chance: 0,
            },
        );
        map.add_event(
            Position::new(5, 5),
            MapEvent::LockedDoor {
                name: "Gate B".to_string(),
                lock_id: "gate_b".to_string(),
                key_item_id: Some(99),
                initial_trap_chance: 20,
            },
        );

        map.init_lock_states();

        assert_eq!(map.lock_states.len(), 2);

        let state_a = map
            .lock_states
            .get("gate_a")
            .expect("gate_a must be present");
        assert!(state_a.is_locked);
        assert_eq!(state_a.trap_chance, 0);

        let state_b = map
            .lock_states
            .get("gate_b")
            .expect("gate_b must be present");
        assert!(state_b.is_locked);
        assert_eq!(state_b.trap_chance, 20);
    }

    /// `init_lock_states` does NOT overwrite a lock state already present in
    /// `lock_states` (e.g. loaded from save data).
    #[test]
    fn test_map_init_lock_states_does_not_overwrite_existing() {
        use crate::domain::types::Position;
        use crate::domain::world::types::{Map, MapEvent};

        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        map.add_event(
            Position::new(3, 3),
            MapEvent::LockedDoor {
                name: "Already Open".to_string(),
                lock_id: "open_door".to_string(),
                key_item_id: None,
                initial_trap_chance: 0,
            },
        );

        // Pre-populate an unlocked state (as if loaded from save data).
        let mut unlocked = LockState::new("open_door");
        unlocked.unlock();
        map.lock_states.insert("open_door".to_string(), unlocked);

        map.init_lock_states();

        let state = map
            .lock_states
            .get("open_door")
            .expect("open_door must still be present");
        assert!(
            !state.is_locked,
            "init_lock_states must not re-lock a previously unlocked door"
        );
    }
}
