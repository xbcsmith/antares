# Classes

## Introduction to Classes

There are now four main schools for classes:

- Fighter
- Magic-Users
- Clerics
- Thieves

## Class Data Structure

Currently the class data structure is as follows:

```rust
 let knight = ClassDefinition {
     id: "knight".to_string(),
     name: "Knight".to_string(),
     description: "A brave warrior".to_string(),
     hp_die: DiceRoll::new(1, 10, 0),
     spell_school: None,
     is_pure_caster: false,
     spell_stat: None,
     disablement_bit_index: 0,
     special_abilities: vec!["multiple_attacks".to_string()],
     starting_weapon_id: None,
     starting_armor_id: None,
     starting_items: vec![],
};
```


## Updates to Class Data Structure

We want to add class school to the class data structure. This will allow for the name to whatever the user wants it to be while assigning it a school that it derives its abilities from.

The plan is to add `class_school` to the data structure where `class_school` is a string that represents the basis for the class. Values would be "fighter", "magic-user", "cleric", or "thief".

This change allows users to create classes with different schools and abilities. For example the user could create a class called "Rogue Trickster" with the class_school "thief", a `spell_school` of "magic-user", and a `spell_stat` of "intelligence" with `is_pure_caster` set to `false`.
