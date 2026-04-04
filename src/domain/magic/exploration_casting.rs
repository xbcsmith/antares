// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Exploration-mode spell casting domain logic.
//!
//! Provides functions for casting spells outside of combat — healing between
//! fights, casting Light in dungeons, Town Portal, Create Food, etc.
//!
//! # Target Resolution
//!
//! | [`SpellTarget`]                                    | Exploration behaviour         |
//! |----------------------------------------------------|-------------------------------|
//! | `Self_`                                            | Applies to the caster only    |
//! | `SingleCharacter`                                  | Caller supplies party index   |
//! | `AllCharacters`                                    | Applied to every party member |
//! | `SingleMonster / MonsterGroup / AllMonsters / SpecificMonsters` | Returns `SpellError::CombatOnly` |
//!
//! # Architecture Reference
//!
//! Phase 3 of `docs/explanation/spell_system_updates_implementation_plan.md`.

use crate::application::GameState;
use crate::domain::character::Party;
use crate::domain::items::ItemDatabase;
use crate::domain::magic::casting::can_cast_spell;
use crate::domain::magic::effect_dispatch::{apply_spell_effect, SpellEffectResult};
use crate::domain::magic::types::{Spell, SpellError, SpellTarget};
use crate::domain::types::GameMode;
use rand::Rng;

// ===== Target Resolution =====

/// Which party member(s) should receive the spell effect during exploration.
///
/// Resolved by the UI before calling [`cast_exploration_spell`].
///
/// # Examples
///
/// ```
/// use antares::domain::magic::exploration_casting::ExplorationTarget;
///
/// let t = ExplorationTarget::AllCharacters;
/// assert!(matches!(t, ExplorationTarget::AllCharacters));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplorationTarget {
    /// Apply to the caster (implied by [`SpellTarget::Self_`]).
    Self_,
    /// Apply to a specific party member by their index in `party.members`.
    Character(usize),
    /// Apply to every living party member.
    AllCharacters,
}

impl ExplorationTarget {
    /// Map a spell's declared [`SpellTarget`] to the appropriate
    /// `ExplorationTarget`, using `caster_index` as the default for `Self_`.
    ///
    /// Returns `None` when the target is monster-only (caller should return
    /// [`SpellError::CombatOnly`] in that case).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::magic::exploration_casting::ExplorationTarget;
    /// use antares::domain::magic::types::SpellTarget;
    ///
    /// assert_eq!(
    ///     ExplorationTarget::from_spell_target(SpellTarget::Self_, 0),
    ///     Some(ExplorationTarget::Self_),
    /// );
    /// assert_eq!(
    ///     ExplorationTarget::from_spell_target(SpellTarget::AllCharacters, 0),
    ///     Some(ExplorationTarget::AllCharacters),
    /// );
    /// assert_eq!(
    ///     ExplorationTarget::from_spell_target(SpellTarget::SingleMonster, 0),
    ///     None,
    /// );
    /// ```
    pub fn from_spell_target(target: SpellTarget, _caster_index: usize) -> Option<Self> {
        match target {
            SpellTarget::Self_ => Some(ExplorationTarget::Self_),
            SpellTarget::AllCharacters => Some(ExplorationTarget::AllCharacters),
            SpellTarget::SingleCharacter => None, // Requires UI prompt
            SpellTarget::SingleMonster
            | SpellTarget::MonsterGroup
            | SpellTarget::AllMonsters
            | SpellTarget::SpecificMonsters => None,
        }
    }
}

// ===== Validation =====

/// Checks whether a character can cast a spell in exploration mode.
///
/// Validates all requirements:
/// - Character is not unconscious or silenced
/// - Character has the right class/level for the spell
/// - Character has sufficient SP and gems
/// - Spell is not `CombatOnly` (or monster-targeting)
/// - Spell is not `IndoorOnly` when party is outdoors (and vice-versa)
///
/// # Arguments
///
/// * `character`  — the character attempting to cast
/// * `spell`      — spell definition to validate against
/// * `is_outdoor` — whether the party is currently outdoors
///
/// # Errors
///
/// Returns [`SpellError::CombatOnly`] if the spell requires an active combat
/// or targets monsters.  Returns other `SpellError` variants for
/// condition/resource/level failures.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::exploration_casting::can_cast_exploration_spell;
/// use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
/// use antares::domain::character::{Character, Sex, Alignment};
///
/// let mut cleric = Character::new(
///     "Healer".to_string(), "human".to_string(), "cleric".to_string(),
///     Sex::Female, Alignment::Good,
/// );
/// cleric.level = 3;
/// cleric.sp.current = 10;
///
/// let cure = Spell::new(
///     0x0101, "Cure Wounds", SpellSchool::Cleric, 1, 2, 0,
///     SpellContext::Anytime, SpellTarget::SingleCharacter,
///     "Heals 8 HP", None, 0, false,
/// );
///
/// assert!(can_cast_exploration_spell(&cleric, &cure, false).is_ok());
/// ```
pub fn can_cast_exploration_spell(
    character: &crate::domain::character::Character,
    spell: &Spell,
    is_outdoor: bool,
) -> Result<(), SpellError> {
    // Monster-targeting spells are combat-only regardless of context flag.
    match spell.target {
        SpellTarget::SingleMonster
        | SpellTarget::MonsterGroup
        | SpellTarget::AllMonsters
        | SpellTarget::SpecificMonsters => return Err(SpellError::CombatOnly),
        _ => {}
    }

    // Delegate remaining validation to the shared can_cast_spell function.
    can_cast_spell(character, spell, &GameMode::Exploration, false, is_outdoor)
}

// ===== Casting =====

/// Casts a spell in exploration mode, applying effects directly to party state.
///
/// Assumes `can_cast_exploration_spell` has already been called and passed.
/// Consumes SP and gems from the caster, then delegates to
/// [`apply_spell_effect`] for each affected party member.
///
/// Utility side effects (food creation) are applied directly to party
/// inventories using the item database passed by the caller.
///
/// # Arguments
///
/// * `caster_index`  — index into `game_state.party.members` for the caster
/// * `spell`         — spell definition
/// * `target`        — which party member(s) receive the effect
/// * `game_state`    — mutable game state (party, active spells modified in place)
/// * `item_db`       — item database used to look up food item IDs for `CreateFood`
/// * `rng`           — RNG for any dice rolls
///
/// # Returns
///
/// A [`SpellEffectResult`] aggregating all mutations that occurred.
///
/// # Errors
///
/// Returns [`SpellError::InvalidTarget`] if `caster_index` is out of bounds.
/// Returns [`SpellError`] if validation fails.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::exploration_casting::{cast_exploration_spell, ExplorationTarget};
/// use antares::domain::magic::types::{
///     Spell, SpellSchool, SpellContext, SpellTarget, SpellEffectType,
/// };
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::items::ItemDatabase;
/// use antares::domain::types::DiceRoll;
/// use antares::application::GameState;
///
/// let mut state = GameState::new();
/// let mut cleric = Character::new(
///     "Healer".to_string(), "human".to_string(), "cleric".to_string(),
///     Sex::Female, Alignment::Good,
/// );
/// cleric.level = 3;
/// cleric.hp.current = 5;
/// cleric.hp.base = 20;
/// cleric.sp.current = 10;
/// cleric.sp.base = 20;
/// state.party.members.push(cleric);
///
/// let mut cure = Spell::new(
///     0x0101, "Cure Wounds", SpellSchool::Cleric, 1, 2, 0,
///     SpellContext::Anytime, SpellTarget::Self_,
///     "Heals 8 HP", None, 0, false,
/// );
/// cure.effect_type = Some(SpellEffectType::Healing { amount: DiceRoll::new(1, 8, 0) });
///
/// let item_db = ItemDatabase::new();
/// let result = cast_exploration_spell(
///     0, &cure, ExplorationTarget::Self_, &mut state, &item_db, &mut rand::rng(),
/// );
/// assert!(result.is_ok());
/// assert_eq!(state.party.members[0].sp.current, 8); // 10 - 2
/// ```
pub fn cast_exploration_spell<R: Rng>(
    caster_index: usize,
    spell: &Spell,
    target: ExplorationTarget,
    game_state: &mut GameState,
    item_db: &ItemDatabase,
    rng: &mut R,
) -> Result<SpellEffectResult, SpellError> {
    // 1. Validate (immutable borrow, dropped before destructure).
    {
        let caster = game_state
            .party
            .members
            .get(caster_index)
            .ok_or(SpellError::InvalidTarget)?;
        can_cast_exploration_spell(caster, spell, false)?;
    }

    // 2. Consume SP and gems.
    {
        let caster = &mut game_state.party.members[caster_index];
        caster.sp.current = caster.sp.current.saturating_sub(spell.sp_cost);
        if spell.gem_cost > 0 {
            caster.gems = caster.gems.saturating_sub(spell.gem_cost as u32);
        }
    }

    // 3. Split borrows: party and active_spells are distinct fields of GameState.
    let GameState {
        ref mut active_spells,
        ref mut party,
        ..
    } = *game_state;

    // 4. Apply effects.
    let result = match target {
        ExplorationTarget::Self_ => {
            let caster = &mut party.members[caster_index];
            apply_spell_effect(spell, Some(caster), active_spells, rng)
        }
        ExplorationTarget::Character(target_index) => {
            if target_index >= party.members.len() {
                return Err(SpellError::InvalidTarget);
            }
            let target_char = &mut party.members[target_index];
            apply_spell_effect(spell, Some(target_char), active_spells, rng)
        }
        ExplorationTarget::AllCharacters => {
            let mut total_healed = 0i32;
            let mut last_buff = None;
            let mut last_cure = None;
            let mut total_food = 0u32;
            let mut affected = Vec::new();

            for (i, member) in party.members.iter_mut().enumerate() {
                // Skip incapacitated members for healing/buff spells.
                if member.conditions.is_fatal() {
                    continue;
                }
                let r = apply_spell_effect(spell, Some(member), active_spells, rng);
                total_healed = total_healed.saturating_add(r.total_hp_healed);
                if r.buff_applied.is_some() {
                    last_buff = r.buff_applied;
                }
                if r.condition_cured.is_some() {
                    last_cure = r.condition_cured;
                }
                total_food = total_food.saturating_add(r.food_created);
                affected.push(i);
            }

            SpellEffectResult {
                success: true,
                message: format!("{} affects all party members.", spell.name),
                total_hp_healed: total_healed,
                buff_applied: last_buff,
                condition_cured: last_cure,
                food_created: total_food,
                affected_targets: affected,
            }
        }
    };

    // 5. Apply food side effects to party inventories.
    if result.food_created > 0 {
        add_food_to_party(party, item_db, result.food_created);
    }

    Ok(result)
}

/// Returns the list of spells the character can currently cast in exploration.
///
/// Filters the spell database to spells that:
/// - Belong to a school the character can cast (`SpellBook` membership)
/// - Are not blocked by `CombatOnly` context
/// - Are not monster-targeting
/// - Pass all other checks in [`can_cast_exploration_spell`]
///
/// # Arguments
///
/// * `character`  — the character whose castable spells are queried
/// * `spell_db`   — spell database to search
/// * `is_outdoor` — whether the party is currently outdoors
///
/// # Returns
///
/// A sorted `Vec<&Spell>` (by level then ID) of all castable exploration spells.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::exploration_casting::get_castable_exploration_spells;
/// use antares::sdk::database::SpellDatabase;
/// use antares::domain::magic::{Spell, SpellSchool, SpellContext, SpellTarget};
/// use antares::domain::character::{Character, Sex, Alignment};
///
/// let mut spell_db = SpellDatabase::new();
/// let heal = Spell::new(
///     0x0101, "First Aid", SpellSchool::Cleric, 1, 2, 0,
///     SpellContext::Anytime, SpellTarget::SingleCharacter,
///     "Heals 8 HP", None, 0, false,
/// );
/// spell_db.add_spell(heal).unwrap();
///
/// let mut cleric = Character::new(
///     "Healer".to_string(), "human".to_string(), "cleric".to_string(),
///     Sex::Female, Alignment::Good,
/// );
/// cleric.level = 3;
/// cleric.sp.current = 10;
///
/// let castable = get_castable_exploration_spells(&cleric, &spell_db, false);
/// assert_eq!(castable.len(), 1);
/// ```
pub fn get_castable_exploration_spells<'a>(
    character: &crate::domain::character::Character,
    spell_db: &'a crate::sdk::database::SpellDatabase,
    is_outdoor: bool,
) -> Vec<&'a Spell> {
    let ids = spell_db.all_spells();
    let mut spells: Vec<&Spell> = ids
        .into_iter()
        .filter_map(|id| spell_db.get_spell(id))
        .filter(|s| can_cast_exploration_spell(character, s, is_outdoor).is_ok())
        .collect();

    // Stable sort by (level, id) so the list is deterministic.
    spells.sort_by_key(|s| (s.level, s.id));
    spells
}

// ===== Utility Helpers =====

/// Adds `amount` food ration items to party member inventories.
///
/// Searches `item_db` for a single-ration `IsFood(1)` item (or any food item
/// if none carry exactly one ration) and distributes that many slots across
/// party member inventories in order, skipping full inventories.
///
/// Returns the number of food ration units actually added (may be less than
/// `amount` if all inventories are full).
///
/// # Examples
///
/// ```
/// use antares::domain::magic::exploration_casting::add_food_to_party;
/// use antares::domain::character::{Character, Sex, Alignment, Party};
/// use antares::domain::items::ItemDatabase;
/// use antares::domain::items::types::{Item, ItemType, ConsumableData, ConsumableEffect};
///
/// let mut item_db = ItemDatabase::new();
/// let food = Item {
///     id: 1,
///     name: "Ration".to_string(),
///     item_type: ItemType::Consumable(ConsumableData {
///         effect: ConsumableEffect::IsFood(1),
///         is_combat_usable: false,
///         duration_minutes: None,
///     }),
///     base_cost: 1,
///     sell_cost: 0,
///     alignment_restriction: None,
///     constant_bonus: None,
///     temporary_bonus: None,
///     spell_effect: None,
///     max_charges: 0,
///     is_cursed: false,
///     icon_path: None,
///     tags: vec![],
///     mesh_descriptor_override: None,
///     mesh_id: None,
/// };
/// item_db.add_item(food).unwrap();
///
/// let mut party = Party::new();
/// let member = Character::new(
///     "Hero".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Male, Alignment::Good,
/// );
/// party.members.push(member);
///
/// let added = add_food_to_party(&mut party, &item_db, 3);
/// assert_eq!(added, 3);
/// assert_eq!(party.members[0].inventory.items.len(), 3);
/// ```
pub fn add_food_to_party(party: &mut Party, item_db: &ItemDatabase, amount: u32) -> u32 {
    let food_id = find_food_item_id(item_db);
    let food_id = match food_id {
        Some(id) => id,
        None => return 0, // No food items in this database (sparse test DBs are OK).
    };

    let mut added = 0u32;
    for member in &mut party.members {
        while added < amount {
            if member.inventory.add_item(food_id, 0).is_err() {
                break; // This member's inventory is full; try the next member.
            }
            added += 1;
        }
        if added >= amount {
            break;
        }
    }
    added
}

/// Finds the item ID of the best food item in the database.
///
/// Prefers single-ration `IsFood(1)` items so ration-units map 1-to-1 with
/// inventory slots, falling back to any food item.  Among equals, the
/// lowest ID wins for determinism.
fn find_food_item_id(item_db: &ItemDatabase) -> Option<crate::domain::types::ItemId> {
    use crate::domain::items::types::ConsumableEffect;
    use crate::domain::items::types::ItemType;

    let mut all_items = item_db.all_items();
    all_items.sort_by_key(|i| i.id);

    let mut single_ration: Option<crate::domain::types::ItemId> = None;
    let mut any_food: Option<crate::domain::types::ItemId> = None;

    for item in &all_items {
        if let ItemType::Consumable(ref data) = item.item_type {
            if let ConsumableEffect::IsFood(rations) = data.effect {
                if rations == 1 && single_ration.is_none() {
                    single_ration = Some(item.id);
                }
                if any_food.is_none() {
                    any_food = Some(item.id);
                }
            }
        }
    }

    single_ration.or(any_food)
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameState;
    use crate::domain::character::{Alignment, Character, Condition, Party, Sex};
    use crate::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
    use crate::domain::items::ItemDatabase;
    use crate::domain::magic::types::{
        BuffField, Spell, SpellContext, SpellEffectType, SpellSchool, SpellTarget, UtilityType,
    };
    use crate::domain::types::DiceRoll;
    use rand::rng;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn make_cleric(level: u32, sp: u16) -> Character {
        let mut c = Character::new(
            "Healer".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        c.level = level;
        c.sp.current = sp;
        c.sp.base = sp;
        c.hp.current = 10;
        c.hp.base = 20;
        c
    }

    fn make_sorcerer(level: u32, sp: u16) -> Character {
        let mut c = Character::new(
            "Mage".to_string(),
            "human".to_string(),
            "sorcerer".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.level = level;
        c.sp.current = sp;
        c.sp.base = sp;
        c.hp.current = 10;
        c.hp.base = 20;
        c
    }

    fn make_knight(level: u32) -> Character {
        let mut c = Character::new(
            "Knight".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.level = level;
        c.sp.current = 0;
        c.sp.base = 0;
        c
    }

    fn make_anytime_heal_spell() -> Spell {
        let mut s = Spell::new(
            0x0101,
            "First Aid",
            SpellSchool::Cleric,
            1,
            2,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Heals 8 HP",
            None,
            0,
            false,
        );
        s.effect_type = Some(SpellEffectType::Healing {
            amount: DiceRoll::new(1, 8, 0),
        });
        s
    }

    fn make_combat_only_spell() -> Spell {
        Spell::new(
            0x0201,
            "Fireball",
            SpellSchool::Sorcerer,
            1,
            5,
            0,
            SpellContext::CombatOnly,
            SpellTarget::AllMonsters,
            "Burns everything",
            Some(DiceRoll::new(3, 6, 0)),
            0,
            false,
        )
    }

    fn make_noncombat_light_spell() -> Spell {
        let mut s = Spell::new(
            0x0102,
            "Light",
            SpellSchool::Cleric,
            1,
            3,
            0,
            SpellContext::NonCombatOnly,
            SpellTarget::Self_,
            "Creates light",
            None,
            60,
            false,
        );
        s.effect_type = Some(SpellEffectType::Buff {
            buff_field: BuffField::Light,
            duration: 60,
        });
        s
    }

    fn make_food_item_db() -> ItemDatabase {
        let mut db = ItemDatabase::new();
        let food = Item {
            id: 108,
            name: "Food Ration".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::IsFood(1),
                is_combat_usable: false,
                duration_minutes: None,
            }),
            base_cost: 1,
            sell_cost: 0,
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
        };
        db.add_item(food).unwrap();
        db
    }

    fn make_create_food_spell() -> Spell {
        let mut s = Spell::new(
            0x0301,
            "Create Food",
            SpellSchool::Cleric,
            3,
            15,
            0,
            SpellContext::NonCombatOnly,
            SpellTarget::Self_,
            "Creates food for the party",
            None,
            0,
            false,
        );
        s.effect_type = Some(SpellEffectType::Utility {
            utility_type: UtilityType::CreateFood { amount: 6 },
        });
        s
    }

    // ── can_cast_exploration_spell tests ─────────────────────────────────────

    #[test]
    fn test_can_cast_exploration_anytime_spell_succeeds() {
        let cleric = make_cleric(3, 10);
        let spell = make_anytime_heal_spell();
        assert!(can_cast_exploration_spell(&cleric, &spell, false).is_ok());
    }

    #[test]
    fn test_can_cast_exploration_noncombat_spell_succeeds() {
        let cleric = make_cleric(3, 10);
        let spell = make_noncombat_light_spell();
        assert!(can_cast_exploration_spell(&cleric, &spell, false).is_ok());
    }

    #[test]
    fn test_can_cast_exploration_rejects_combat_only() {
        let sorcerer = make_sorcerer(3, 20);
        let spell = make_combat_only_spell();
        assert!(matches!(
            can_cast_exploration_spell(&sorcerer, &spell, false),
            Err(SpellError::CombatOnly)
        ));
    }

    #[test]
    fn test_can_cast_exploration_rejects_monster_targets() {
        let sorcerer = make_sorcerer(3, 20);
        let monster_spell = Spell::new(
            0x0202,
            "Magic Missile",
            SpellSchool::Sorcerer,
            1,
            3,
            0,
            SpellContext::Anytime, // Even Anytime spells that target monsters are combat-only
            SpellTarget::SingleMonster,
            "Hits a monster",
            None,
            0,
            false,
        );
        assert!(matches!(
            can_cast_exploration_spell(&sorcerer, &monster_spell, false),
            Err(SpellError::CombatOnly)
        ));
    }

    #[test]
    fn test_can_cast_exploration_rejects_insufficient_sp() {
        let cleric = make_cleric(3, 0); // No SP
        let spell = make_anytime_heal_spell(); // Costs 2 SP
        assert!(matches!(
            can_cast_exploration_spell(&cleric, &spell, false),
            Err(SpellError::NotEnoughSP { .. })
        ));
    }

    #[test]
    fn test_can_cast_exploration_rejects_wrong_class() {
        let knight = make_knight(5); // Knights can't cast Cleric spells
        let spell = make_anytime_heal_spell();
        assert!(matches!(
            can_cast_exploration_spell(&knight, &spell, false),
            Err(SpellError::WrongClass(..))
        ));
    }

    #[test]
    fn test_can_cast_exploration_rejects_silenced_character() {
        let mut cleric = make_cleric(3, 10);
        cleric.conditions.add(Condition::SILENCED);
        let spell = make_anytime_heal_spell();
        assert!(matches!(
            can_cast_exploration_spell(&cleric, &spell, false),
            Err(SpellError::Silenced)
        ));
    }

    #[test]
    fn test_can_cast_exploration_rejects_unconscious_character() {
        let mut cleric = make_cleric(3, 10);
        cleric.conditions.add(Condition::UNCONSCIOUS);
        let spell = make_anytime_heal_spell();
        assert!(matches!(
            can_cast_exploration_spell(&cleric, &spell, false),
            Err(SpellError::Unconscious)
        ));
    }

    // ── cast_exploration_spell tests ─────────────────────────────────────────

    #[test]
    fn test_cast_exploration_spell_self_target_consumes_sp() {
        let mut state = GameState::new();
        let mut cleric = make_cleric(3, 10);
        cleric.hp.current = 5;
        state.party.members.push(cleric);

        let spell = make_anytime_heal_spell();
        let item_db = ItemDatabase::new();
        let result = cast_exploration_spell(
            0,
            &spell,
            ExplorationTarget::Self_,
            &mut state,
            &item_db,
            &mut rng(),
        );
        assert!(result.is_ok());
        // SP consumed: 10 - 2 = 8
        assert_eq!(state.party.members[0].sp.current, 8);
    }

    #[test]
    fn test_cast_exploration_spell_heals_target() {
        let mut state = GameState::new();
        let mut cleric = make_cleric(3, 10);
        cleric.hp.current = 5;
        cleric.hp.base = 20;
        state.party.members.push(cleric);

        let spell = make_anytime_heal_spell();
        let item_db = ItemDatabase::new();
        let result = cast_exploration_spell(
            0,
            &spell,
            ExplorationTarget::Self_,
            &mut state,
            &item_db,
            &mut rng(),
        );
        assert!(result.is_ok());
        let r = result.unwrap();
        // Healing should have occurred (1d8, minimum 1).
        assert!(r.total_hp_healed > 0);
        // HP after heal is at most hp.base.
        assert!(state.party.members[0].hp.current <= state.party.members[0].hp.base);
    }

    #[test]
    fn test_cast_exploration_spell_heals_other_character() {
        let mut state = GameState::new();
        let cleric = make_cleric(3, 10);
        let mut wounded = make_knight(3);
        wounded.hp.current = 3;
        wounded.hp.base = 15;
        state.party.members.push(cleric);
        state.party.members.push(wounded);

        let spell = make_anytime_heal_spell();
        let item_db = ItemDatabase::new();
        let result = cast_exploration_spell(
            0,
            &spell,
            ExplorationTarget::Character(1),
            &mut state,
            &item_db,
            &mut rng(),
        );
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.total_hp_healed > 0);
    }

    #[test]
    fn test_cast_exploration_spell_all_characters() {
        let mut state = GameState::new();
        let cleric = make_cleric(3, 20);
        let mut wounded1 = make_knight(3);
        wounded1.hp.current = 5;
        wounded1.hp.base = 15;
        let mut wounded2 = make_knight(3);
        wounded2.hp.current = 3;
        wounded2.hp.base = 12;
        state.party.members.push(cleric);
        state.party.members.push(wounded1);
        state.party.members.push(wounded2);

        let spell = make_anytime_heal_spell();
        let item_db = ItemDatabase::new();
        let result = cast_exploration_spell(
            0,
            &spell,
            ExplorationTarget::AllCharacters,
            &mut state,
            &item_db,
            &mut rng(),
        );
        assert!(result.is_ok());
        // Should have touched multiple targets.
        let r = result.unwrap();
        assert!(!r.affected_targets.is_empty());
    }

    #[test]
    fn test_cast_exploration_spell_rejects_combat_only() {
        let mut state = GameState::new();
        let sorcerer = make_sorcerer(3, 20);
        state.party.members.push(sorcerer);

        let spell = make_combat_only_spell();
        let item_db = ItemDatabase::new();
        let result = cast_exploration_spell(
            0,
            &spell,
            ExplorationTarget::Self_,
            &mut state,
            &item_db,
            &mut rng(),
        );
        assert!(matches!(result, Err(SpellError::CombatOnly)));
    }

    #[test]
    fn test_cast_exploration_spell_rejects_out_of_bounds_caster() {
        let mut state = GameState::new();
        let spell = make_anytime_heal_spell();
        let item_db = ItemDatabase::new();
        let result = cast_exploration_spell(
            5,
            &spell,
            ExplorationTarget::Self_,
            &mut state,
            &item_db,
            &mut rng(),
        );
        assert!(matches!(result, Err(SpellError::InvalidTarget)));
    }

    #[test]
    fn test_cast_exploration_spell_rejects_out_of_bounds_target() {
        let mut state = GameState::new();
        let cleric = make_cleric(3, 10);
        state.party.members.push(cleric);

        let spell = make_anytime_heal_spell();
        let item_db = ItemDatabase::new();
        let result = cast_exploration_spell(
            0,
            &spell,
            ExplorationTarget::Character(99),
            &mut state,
            &item_db,
            &mut rng(),
        );
        assert!(matches!(result, Err(SpellError::InvalidTarget)));
    }

    #[test]
    fn test_cast_exploration_spell_buff_light_updates_active_spells() {
        let mut state = GameState::new();
        let cleric = make_cleric(3, 10);
        state.party.members.push(cleric);
        assert_eq!(state.active_spells.light, 0);

        let spell = make_noncombat_light_spell();
        let item_db = ItemDatabase::new();
        let result = cast_exploration_spell(
            0,
            &spell,
            ExplorationTarget::Self_,
            &mut state,
            &item_db,
            &mut rng(),
        );
        assert!(result.is_ok());
        // Light duration should now be set.
        assert!(state.active_spells.light > 0);
    }

    #[test]
    fn test_cast_exploration_spell_create_food_adds_items() {
        let mut state = GameState::new();
        let cleric = make_cleric(5, 30);
        state.party.members.push(cleric);

        let spell = make_create_food_spell();
        let item_db = make_food_item_db();

        let initial_inventory_len = state.party.members[0].inventory.items.len();
        let result = cast_exploration_spell(
            0,
            &spell,
            ExplorationTarget::Self_,
            &mut state,
            &item_db,
            &mut rng(),
        );
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.food_created, 6);
        // 6 ration slots should have been added.
        assert_eq!(
            state.party.members[0].inventory.items.len(),
            initial_inventory_len + 6
        );
    }

    #[test]
    fn test_cast_exploration_spell_consumes_gems() {
        let mut state = GameState::new();
        let mut sorcerer = make_sorcerer(5, 30);
        sorcerer.gems = 5;
        state.party.members.push(sorcerer);

        let mut gem_spell = Spell::new(
            0x0501,
            "Ice Ray",
            SpellSchool::Sorcerer,
            1,
            5,
            2, // Costs 2 gems
            SpellContext::NonCombatOnly,
            SpellTarget::Self_,
            "Freezes water",
            None,
            0,
            false,
        );
        gem_spell.effect_type = Some(SpellEffectType::Utility {
            utility_type: UtilityType::Information,
        });

        let item_db = ItemDatabase::new();
        let result = cast_exploration_spell(
            0,
            &gem_spell,
            ExplorationTarget::Self_,
            &mut state,
            &item_db,
            &mut rng(),
        );
        assert!(result.is_ok());
        assert_eq!(state.party.members[0].gems, 3); // 5 - 2
    }

    #[test]
    fn test_cast_exploration_all_chars_skips_dead() {
        let mut state = GameState::new();
        let cleric = make_cleric(3, 20);
        let mut dead_member = make_knight(3);
        dead_member.conditions.add(Condition::DEAD);
        dead_member.hp.current = 1;
        dead_member.hp.base = 15;
        state.party.members.push(cleric);
        state.party.members.push(dead_member);

        let spell = make_anytime_heal_spell();
        let item_db = ItemDatabase::new();
        let result = cast_exploration_spell(
            0,
            &spell,
            ExplorationTarget::AllCharacters,
            &mut state,
            &item_db,
            &mut rng(),
        );
        assert!(result.is_ok());
        let r = result.unwrap();
        // Dead member should not be in affected_targets.
        assert!(!r.affected_targets.contains(&1));
    }

    // ── get_castable_exploration_spells tests ─────────────────────────────────

    #[test]
    fn test_get_castable_exploration_spells_excludes_combat_only() {
        let mut spell_db = crate::sdk::database::SpellDatabase::new();
        let heal = make_anytime_heal_spell();
        let combat = make_combat_only_spell();
        spell_db.add_spell(heal).unwrap();
        spell_db.add_spell(combat).unwrap();

        let cleric = make_cleric(3, 10);
        let sorcerer = make_sorcerer(3, 20);

        let cleric_castable = get_castable_exploration_spells(&cleric, &spell_db, false);
        // Heal is Anytime+Cleric → included; Fireball is CombatOnly → excluded
        assert!(cleric_castable.iter().any(|s| s.name == "First Aid"));
        assert!(!cleric_castable.iter().any(|s| s.name == "Fireball"));

        let sorc_castable = get_castable_exploration_spells(&sorcerer, &spell_db, false);
        // Sorcerer can't cast Cleric spells.
        assert!(sorc_castable.is_empty());
    }

    #[test]
    fn test_get_castable_exploration_spells_excludes_insufficient_sp() {
        let mut spell_db = crate::sdk::database::SpellDatabase::new();
        spell_db.add_spell(make_anytime_heal_spell()).unwrap(); // Costs 2 SP
        let cleric = make_cleric(3, 0); // No SP
        let castable = get_castable_exploration_spells(&cleric, &spell_db, false);
        assert!(castable.is_empty());
    }

    #[test]
    fn test_get_castable_exploration_spells_sorted_by_level_id() {
        let mut spell_db = crate::sdk::database::SpellDatabase::new();
        let l2 = Spell::new(
            0x0201,
            "Level2",
            SpellSchool::Cleric,
            2,
            2,
            0,
            SpellContext::Anytime,
            SpellTarget::Self_,
            "",
            None,
            0,
            false,
        );
        let l1 = Spell::new(
            0x0101,
            "Level1",
            SpellSchool::Cleric,
            1,
            2,
            0,
            SpellContext::Anytime,
            SpellTarget::Self_,
            "",
            None,
            0,
            false,
        );
        spell_db.add_spell(l2).unwrap();
        spell_db.add_spell(l1).unwrap();

        let cleric = make_cleric(5, 20);
        let castable = get_castable_exploration_spells(&cleric, &spell_db, false);
        assert_eq!(castable.len(), 2);
        assert!(castable[0].level <= castable[1].level);
    }

    // ── add_food_to_party tests ───────────────────────────────────────────────

    #[test]
    fn test_add_food_to_party_with_empty_db_returns_zero() {
        let mut party = Party::new();
        let member = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        party.members.push(member);
        let db = ItemDatabase::new(); // No food items
        let added = add_food_to_party(&mut party, &db, 5);
        assert_eq!(added, 0);
    }

    #[test]
    fn test_add_food_to_party_distributes_across_members() {
        let item_db = make_food_item_db();
        let mut party = Party::new();
        let member1 = Character::new(
            "Hero1".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let member2 = Character::new(
            "Hero2".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        party.members.push(member1);
        party.members.push(member2);
        // Fill member1's inventory (MAX_ITEMS slots)
        for i in 0..crate::domain::character::Inventory::MAX_ITEMS {
            party.members[0]
                .inventory
                .add_item(i as u8 + 100, 0)
                .unwrap();
        }
        // Now add 3 rations — must go to member2.
        let added = add_food_to_party(&mut party, &item_db, 3);
        assert_eq!(added, 3);
        assert_eq!(
            party.members[1].inventory.items.len(),
            3,
            "Food must overflow to member2 when member1 is full"
        );
    }

    // ── ExplorationTarget::from_spell_target tests ───────────────────────────

    #[test]
    fn test_exploration_target_from_self() {
        assert_eq!(
            ExplorationTarget::from_spell_target(SpellTarget::Self_, 2),
            Some(ExplorationTarget::Self_),
        );
    }

    #[test]
    fn test_exploration_target_from_all_characters() {
        assert_eq!(
            ExplorationTarget::from_spell_target(SpellTarget::AllCharacters, 0),
            Some(ExplorationTarget::AllCharacters),
        );
    }

    #[test]
    fn test_exploration_target_from_single_character_returns_none() {
        // SingleCharacter requires a UI prompt, so from_spell_target cannot resolve it.
        assert_eq!(
            ExplorationTarget::from_spell_target(SpellTarget::SingleCharacter, 0),
            None,
        );
    }

    #[test]
    fn test_exploration_target_from_monster_targets_returns_none() {
        for t in [
            SpellTarget::SingleMonster,
            SpellTarget::MonsterGroup,
            SpellTarget::AllMonsters,
            SpellTarget::SpecificMonsters,
        ] {
            assert_eq!(
                ExplorationTarget::from_spell_target(t, 0),
                None,
                "{t:?} should not resolve to an ExplorationTarget"
            );
        }
    }
}
