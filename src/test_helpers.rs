// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared test helpers for constructing game entities.
//!
//! All functions in this module are gated behind `#[cfg(test)]` and provide
//! convenient factories for [`Character`], [`Item`], [`Spell`], [`Party`],
//! and related types used across many test modules.
//!
//! # Usage
//!
//! Import the factory you need in your test module:
//!
//! ```rust,ignore
//! use crate::test_helpers::factories::test_character;
//!
//! let hero = test_character("Hero");
//! ```

#[cfg(test)]
pub mod factories {
    use crate::domain::character::{Alignment, Character, Party, Sex};
    use crate::domain::items::types::{
        ConsumableData, ConsumableEffect, Item, ItemType, WeaponClassification, WeaponData,
    };
    use crate::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};
    use crate::domain::types::{DiceRoll, ItemId, SpellId};

    // ===== Character Factories =====

    /// Creates a basic test character with the given name.
    ///
    /// Uses `"human"` race, `"knight"` class, [`Sex::Male`], [`Alignment::Good`].
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the character
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_character;
    ///
    /// let hero = test_character("Sir Lancelot");
    /// assert_eq!(hero.name, "Sir Lancelot");
    /// assert_eq!(hero.race_id, "human");
    /// assert_eq!(hero.class_id, "knight");
    /// ```
    pub fn test_character(name: &str) -> Character {
        Character::new(
            name.to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        )
    }

    /// Creates a test character with the specified class.
    ///
    /// Uses `"human"` race, [`Sex::Male`], [`Alignment::Good`].
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the character
    /// * `class_id` - The class identifier (e.g. `"knight"`, `"cleric"`, `"sorcerer"`)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_character_with_class;
    ///
    /// let mage = test_character_with_class("Gandalf", "sorcerer");
    /// assert_eq!(mage.class_id, "sorcerer");
    /// ```
    pub fn test_character_with_class(name: &str, class_id: &str) -> Character {
        Character::new(
            name.to_string(),
            "human".to_string(),
            class_id.to_string(),
            Sex::Male,
            Alignment::Good,
        )
    }

    /// Creates a test character with specified race and class.
    ///
    /// Uses [`Sex::Male`] and [`Alignment::Good`].
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the character
    /// * `race_id` - The race identifier (e.g. `"human"`, `"elf"`, `"gnome"`)
    /// * `class_id` - The class identifier (e.g. `"knight"`, `"cleric"`, `"mage"`)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_character_with_race_class;
    ///
    /// let elf_mage = test_character_with_race_class("Elrond", "elf", "mage");
    /// assert_eq!(elf_mage.race_id, "elf");
    /// assert_eq!(elf_mage.class_id, "mage");
    /// ```
    pub fn test_character_with_race_class(name: &str, race_id: &str, class_id: &str) -> Character {
        Character::new(
            name.to_string(),
            race_id.to_string(),
            class_id.to_string(),
            Sex::Male,
            Alignment::Good,
        )
    }

    /// Creates a dead test character (HP set to 0).
    ///
    /// Useful for testing death-related logic such as resurrection,
    /// experience denial, and condition checks.
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the character
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_dead_character;
    ///
    /// let corpse = test_dead_character("Fallen Hero");
    /// assert_eq!(corpse.hp.current, 0);
    /// ```
    pub fn test_dead_character(name: &str) -> Character {
        let mut c = test_character(name);
        c.hp.current = 0;
        c
    }

    /// Creates a test character equipped with a basic weapon in their inventory.
    ///
    /// The character is a `"knight"` with a sword (item ID `1`) added to their
    /// inventory with 0 charges (non-magical weapon).
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the character
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_character_with_weapon;
    ///
    /// let knight = test_character_with_weapon("Knight");
    /// assert_eq!(knight.name, "Knight");
    /// assert!(!knight.inventory.items.is_empty());
    /// ```
    pub fn test_character_with_weapon(name: &str) -> Character {
        let mut c = test_character(name);
        let sword = test_weapon("Test Sword");
        c.inventory
            .add_item(sword.id, 0)
            .expect("inventory should not be full for a new character");
        c
    }

    /// Creates a test character with a spell in their spell list.
    ///
    /// The character is a `"sorcerer"` with 20 base/current SP and a single
    /// sorcerer spell (ID `0x0201`) added at spell level 1.
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the character
    /// * `spell_name` - A label for the spell (used only for documentation;
    ///   the spell ID `0x0201` is always added)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_character_with_spell;
    ///
    /// let mage = test_character_with_spell("Merlin", "Fireball");
    /// assert_eq!(mage.class_id, "sorcerer");
    /// assert_eq!(mage.sp.base, 20);
    /// assert_eq!(mage.sp.current, 20);
    /// assert!(!mage.spells.sorcerer_spells[0].is_empty());
    /// ```
    pub fn test_character_with_spell(name: &str, _spell_name: &str) -> Character {
        let mut c = test_character_with_class(name, "sorcerer");
        c.sp.base = 20;
        c.sp.current = 20;
        // Add a level-1 sorcerer spell (ID 0x0201) to the spellbook
        let spell_id: SpellId = 0x0201;
        c.spells.sorcerer_spells[0].push(spell_id);
        c
    }

    /// Creates a test character with several items in their inventory.
    ///
    /// The character starts with a consumable potion (item ID `10`) and a
    /// weapon (item ID `1`) already in their backpack.
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the character
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_character_with_inventory;
    ///
    /// let adventurer = test_character_with_inventory("Adventurer");
    /// assert_eq!(adventurer.inventory.items.len(), 2);
    /// ```
    pub fn test_character_with_inventory(name: &str) -> Character {
        let mut c = test_character(name);
        let potion = test_item("Potion");
        let sword = test_weapon("Sword");
        c.inventory
            .add_item(potion.id, 1)
            .expect("inventory should not be full");
        c.inventory
            .add_item(sword.id, 0)
            .expect("inventory should not be full");
        c
    }

    // ===== Party Factories =====

    /// Creates a test party with two default members: a fighter and a healer.
    ///
    /// The party contains:
    /// - `"Fighter"` — a `"knight"` character
    /// - `"Healer"` — a `"cleric"` character
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_party;
    ///
    /// let party = test_party();
    /// assert_eq!(party.size(), 2);
    /// assert_eq!(party.members[0].name, "Fighter");
    /// assert_eq!(party.members[1].class_id, "cleric");
    /// ```
    pub fn test_party() -> Party {
        let mut party = Party::default();
        party
            .add_member(test_character("Fighter"))
            .expect("party should accept first member");
        party
            .add_member(test_character_with_class("Healer", "cleric"))
            .expect("party should accept second member");
        party
    }

    /// Creates a test party with `n` members (clamped to max 6).
    ///
    /// Members are named `"Alpha"`, `"Beta"`, `"Gamma"`, `"Delta"`,
    /// `"Epsilon"`, `"Zeta"` in order. All are default `"knight"` characters.
    ///
    /// # Arguments
    ///
    /// * `n` - Number of members to add (clamped to `Party::MAX_MEMBERS`)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_party_with_members;
    ///
    /// let party = test_party_with_members(4);
    /// assert_eq!(party.size(), 4);
    /// assert_eq!(party.members[0].name, "Alpha");
    /// assert_eq!(party.members[3].name, "Delta");
    /// ```
    pub fn test_party_with_members(n: usize) -> Party {
        let mut party = Party::default();
        let names = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta"];
        for name in names.iter().take(n.min(Party::MAX_MEMBERS)) {
            party
                .add_member(test_character(name))
                .expect("party should not be full within limit");
        }
        party
    }

    // ===== Item Factories =====

    /// Creates a basic test consumable item (healing potion).
    ///
    /// Returns an [`Item`] with:
    /// - `id`: `10`
    /// - `item_type`: `Consumable` — heals 20 HP, usable in combat
    /// - `base_cost`: `50` gold, `sell_cost`: `25` gold
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the item
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_item;
    ///
    /// let potion = test_item("Healing Potion");
    /// assert_eq!(potion.name, "Healing Potion");
    /// assert!(potion.is_consumable());
    /// ```
    pub fn test_item(name: &str) -> Item {
        Item {
            id: 10 as ItemId,
            name: name.to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(20),
                is_combat_usable: true,
                duration_minutes: None,
            }),
            base_cost: 50,
            sell_cost: 25,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: Vec::new(),
            mesh_descriptor_override: None,
            mesh_id: None,
        }
    }

    /// Creates a test weapon item (a basic sword).
    ///
    /// Returns an [`Item`] with:
    /// - `id`: `1`
    /// - `item_type`: `Weapon` — 1d8 damage, +0 bonus, one-handed, simple
    /// - `base_cost`: `100` gold, `sell_cost`: `50` gold
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the weapon
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_weapon;
    ///
    /// let sword = test_weapon("Longsword");
    /// assert_eq!(sword.name, "Longsword");
    /// assert!(sword.is_weapon());
    /// ```
    pub fn test_weapon(name: &str) -> Item {
        Item {
            id: 1 as ItemId,
            name: name.to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::Simple,
            }),
            base_cost: 100,
            sell_cost: 50,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: Vec::new(),
            mesh_descriptor_override: None,
            mesh_id: None,
        }
    }

    // ===== Spell Factories =====

    /// Creates a basic test spell (a level-1 sorcerer attack spell).
    ///
    /// Returns a [`Spell`] with:
    /// - `id`: `0x0201` (sorcerer school, spell 1)
    /// - `school`: `Sorcerer`
    /// - `level`: `1`, `sp_cost`: `2`, `gem_cost`: `0`
    /// - `context`: `CombatOnly`, `target`: `SingleMonster`
    /// - `damage`: `1d6+0`
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the spell
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_spell;
    ///
    /// let spell = test_spell("Magic Missile");
    /// assert_eq!(spell.name, "Magic Missile");
    /// assert_eq!(spell.school, antares::domain::magic::types::SpellSchool::Sorcerer);
    /// assert_eq!(spell.level, 1);
    /// assert_eq!(spell.sp_cost, 2);
    /// ```
    pub fn test_spell(name: &str) -> Spell {
        Spell::new(
            0x0201 as SpellId,
            name,
            SpellSchool::Sorcerer,
            1, // level
            2, // sp_cost
            0, // gem_cost
            SpellContext::CombatOnly,
            SpellTarget::SingleMonster,
            format!("Test spell: {name}"),
            Some(DiceRoll::new(1, 6, 0)),
            0,    // duration (instant)
            true, // saving_throw
        )
    }

    // ===== Factory Self-Tests =====

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_factory_test_character() {
            let c = test_character("Hero");
            assert_eq!(c.name, "Hero");
            assert_eq!(c.race_id, "human");
            assert_eq!(c.class_id, "knight");
            assert_eq!(c.level, 1);
            assert!(c.is_alive());
        }

        #[test]
        fn test_factory_test_character_with_class() {
            let c = test_character_with_class("Mage", "sorcerer");
            assert_eq!(c.name, "Mage");
            assert_eq!(c.class_id, "sorcerer");
            assert_eq!(c.race_id, "human");
        }

        #[test]
        fn test_factory_test_character_with_race_class() {
            let c = test_character_with_race_class("Legolas", "elf", "archer");
            assert_eq!(c.name, "Legolas");
            assert_eq!(c.race_id, "elf");
            assert_eq!(c.class_id, "archer");
        }

        #[test]
        fn test_factory_test_dead_character() {
            let c = test_dead_character("Fallen");
            assert_eq!(c.name, "Fallen");
            assert_eq!(c.hp.current, 0);
        }

        #[test]
        fn test_factory_test_character_with_weapon() {
            let c = test_character_with_weapon("Knight");
            assert_eq!(c.name, "Knight");
            assert_eq!(c.inventory.items.len(), 1);
            // The weapon item ID should match test_weapon's id (1)
            assert_eq!(c.inventory.items[0].item_id, 1);
        }

        #[test]
        fn test_factory_test_character_with_spell() {
            let c = test_character_with_spell("Wizard", "Fireball");
            assert_eq!(c.name, "Wizard");
            assert_eq!(c.class_id, "sorcerer");
            assert_eq!(c.sp.base, 20);
            assert_eq!(c.sp.current, 20);
            // Should have one spell at sorcerer level 1
            assert_eq!(c.spells.sorcerer_spells[0].len(), 1);
            assert_eq!(c.spells.sorcerer_spells[0][0], 0x0201);
        }

        #[test]
        fn test_factory_test_character_with_inventory() {
            let c = test_character_with_inventory("Adventurer");
            assert_eq!(c.name, "Adventurer");
            assert_eq!(c.inventory.items.len(), 2);
            // First item is the potion (id 10), second is the weapon (id 1)
            assert_eq!(c.inventory.items[0].item_id, 10);
            assert_eq!(c.inventory.items[1].item_id, 1);
        }

        #[test]
        fn test_factory_test_party() {
            let party = test_party();
            assert_eq!(party.size(), 2);
            assert_eq!(party.members[0].name, "Fighter");
            assert_eq!(party.members[0].class_id, "knight");
            assert_eq!(party.members[1].name, "Healer");
            assert_eq!(party.members[1].class_id, "cleric");
        }

        #[test]
        fn test_factory_test_party_with_members_zero() {
            let party = test_party_with_members(0);
            assert!(party.is_empty());
            assert_eq!(party.size(), 0);
        }

        #[test]
        fn test_factory_test_party_with_members_three() {
            let party = test_party_with_members(3);
            assert_eq!(party.size(), 3);
            assert_eq!(party.members[0].name, "Alpha");
            assert_eq!(party.members[1].name, "Beta");
            assert_eq!(party.members[2].name, "Gamma");
        }

        #[test]
        fn test_factory_test_party_with_members_clamps_to_max() {
            let party = test_party_with_members(10);
            assert_eq!(party.size(), Party::MAX_MEMBERS);
            assert_eq!(party.members[5].name, "Zeta");
        }

        #[test]
        fn test_factory_test_item() {
            let item = test_item("Healing Potion");
            assert_eq!(item.name, "Healing Potion");
            assert_eq!(item.id, 10);
            assert!(item.is_consumable());
            assert!(!item.is_weapon());
            assert!(!item.is_cursed);
            assert_eq!(item.base_cost, 50);
            assert_eq!(item.sell_cost, 25);
        }

        #[test]
        fn test_factory_test_weapon() {
            let item = test_weapon("Longsword");
            assert_eq!(item.name, "Longsword");
            assert_eq!(item.id, 1);
            assert!(item.is_weapon());
            assert!(!item.is_consumable());
            assert!(!item.is_cursed);
            assert_eq!(item.base_cost, 100);
            assert_eq!(item.sell_cost, 50);
            if let ItemType::Weapon(ref data) = item.item_type {
                assert_eq!(data.damage, DiceRoll::new(1, 8, 0));
                assert_eq!(data.bonus, 0);
                assert_eq!(data.hands_required, 1);
                assert_eq!(data.classification, WeaponClassification::Simple);
            } else {
                panic!("test_weapon should produce a Weapon item type");
            }
        }

        #[test]
        fn test_factory_test_spell() {
            let spell = test_spell("Magic Missile");
            assert_eq!(spell.name, "Magic Missile");
            assert_eq!(spell.id, 0x0201);
            assert_eq!(spell.school, SpellSchool::Sorcerer);
            assert_eq!(spell.level, 1);
            assert_eq!(spell.sp_cost, 2);
            assert_eq!(spell.gem_cost, 0);
            assert_eq!(spell.context, SpellContext::CombatOnly);
            assert_eq!(spell.target, SpellTarget::SingleMonster);
            assert!(spell.damage.is_some());
            assert_eq!(spell.duration, 0);
            assert!(spell.saving_throw);
        }
    }
}
