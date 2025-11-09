# getting started tutorial

a hands-on introduction to antares rpg development and gameplay.

## overview

this tutorial will guide you through:

- building and running the project
- understanding the core game concepts
- running your first combat simulation
- exploring the data-driven content system

## prerequisites

- rust 1.70 or later
- basic familiarity with rust syntax
- a text editor or ide

## step 1: clone and build

```bash
# clone the repository (adjust path as needed)
cd antares

# build the project
cargo build

# run all tests to verify everything works
cargo test --all-features
```

**expected output**: all tests should pass with no errors.

## step 2: understanding the architecture

antares uses a layered architecture:

```text
domain layer     → core game logic (characters, combat, magic)
application layer → game state management
data layer       → ron files for content (items, spells, monsters)
```

**key insight**: game logic is pure rust code, while game content lives in
`.ron` files.

## step 3: exploring game data

open `data/items.ron` to see how items are defined:

```ron
(
    id: 1,
    name: "Dagger",
    item_type: Weapon((
        damage: (count: 1, sides: 4, bonus: 0),
        // ... more properties
    )),
)
```

**try this**: find the goblin definition in `data/monsters.ron`. note its:

- hp (health points)
- ac (armor class)
- attacks (damage dice)
- loot table (gold, gems, experience)

## step 4: creating a character

characters are the foundation of the party. here's how to create one:

```rust
use antares::domain::character::{Character, Class, Race, Sex, Alignment};

let hero = Character::new(
    "conan".to_string(),
    race::human,
    class::knight,
    sex::male,
    alignment::good,
);

println!("created: {} (hp: {})", hero.name, hero.hp.current);
```

**classes available**:

- knight (melee fighter)
- paladin (holy warrior)
- archer (ranged attacker)
- cleric (healer, divine magic)
- sorcerer (damage dealer, arcane magic)
- robber (stealth, trap finding)

## step 5: forming a party

```rust
use antares::application::gamestate;

let mut game = gamestate::new();

// create party members
let knight = character::new("sir lancelot".to_string(), race::human, class::knight, sex::male, alignment::good);
let cleric = character::new("friar tuck".to_string(), race::human, class::cleric, sex::male, alignment::good);

// add to party
game.party.add_member(knight).unwrap();
game.party.add_member(cleric).unwrap();

println!("party size: {}", game.party.size());
```

**party limits**: maximum 6 members, shared resources (gold, gems, food).

## step 6: loading game content

the game uses ron files for all content. here's how to load monsters:

```rust
use antares::domain::combat::database::monsterdatabase;
use antares::domain::types::monsterid;

// load monster database
let monsters = monsterdatabase::load_from_file("data/monsters.ron")
    .expect("failed to load monsters");

// get a specific monster
if let some(goblin_def) = monsters.get_monster(monsterid::from(1)) {
    println!("found: {}", goblin_def.name);
    println!("hp: {}, ac: {}", goblin_def.hp, goblin_def.ac);
}
```

**similarly**:

- `spelldatabase::load_from_file("data/spells.ron")` for spells
- `itemdatabase::load_from_file("data/items.ron")` for items

## step 7: setting up combat

```rust
use antares::domain::combat::engine::{combatstate, start_combat};
use antares::domain::combat::types::handicap;

// create combat encounter
let mut combat = combatstate::new(handicap::even);

// add party members
combat.add_player(knight);
combat.add_player(cleric);

// add monsters
let goblin = goblin_def.to_monster();
combat.add_monster(goblin);

// initialize combat (calculates turn order)
start_combat(&mut combat);

println!("combat started!");
println!("turn order has {} combatants", combat.turn_order.len());
```

## step 8: understanding turn order

combat is turn-based. turn order is determined by:

1. **speed stat**: higher speed acts first
2. **handicap**: party/monster advantage affects initiative
3. **random factor**: adds unpredictability

```rust
// check who goes first
for (i, combatant_id) in combat.turn_order.iter().enumerate() {
    println!("turn {}: combatant {:?}", i + 1, combatant_id);
}
```

## step 9: exploring the spell system

spells have schools, levels, and contexts:

```rust
use antares::domain::magic::database::spelldatabase;
use antares::domain::magic::types::spellschool;

let spells = spelldatabase::load_from_file("data/spells.ron").unwrap();

// get all cleric spells
let cleric_spells = spells.get_spells_by_school(spellschool::cleric);
println!("cleric has {} spells", cleric_spells.len());

// find level 1 spells
for spell in cleric_spells.iter().filter(|s| s.level == 1) {
    println!("- {} (costs {} sp)", spell.name, spell.sp_cost);
}
```

**spell contexts**:

- `anytime`: can cast anywhere
- `combatonly`: only in combat
- `outdooronly`: only in outdoor areas
- `indooronly`: only in dungeons/towns

## step 10: running integration tests

see complete game flows in action:

```bash
# run combat integration tests
cargo test --test combat_integration

# run magic system tests
cargo test --test magic_integration

# run game flow tests
cargo test --test game_flow_integration
```

**what to look for**:

- test names describe complete scenarios
- tests show how systems interact
- tests verify architecture compliance

## step 11: understanding the attributepair pattern

stats in antares use the `attributepair` pattern:

```rust
use antares::domain::character::attributepair;

let mut might = attributepair::new(15);
println!("base: {}, current: {}", might.base, might.current);

// apply buff (from spell/item)
might.modify(5);
println!("buffed: {}", might.current); // 20

// reset removes temporary modifications
might.reset();
println!("reset: {}", might.current); // 15 (back to base)
```

**why this matters**: buffs/debuffs modify `current`, but `base` stays constant.

## step 12: understanding conditions

characters can have conditions that affect their abilities:

```rust
use antares::domain::character::condition;

let mut hero = character::new(/* ... */);

// apply paralysis
hero.conditions.add(condition::paralyzed);

// check if can act
if !hero.can_act() {
    println!("{} is paralyzed and cannot act!", hero.name);
}

// clear all conditions
hero.conditions.clear();
```

**condition types**:

- `fine` (0) - no conditions
- `asleep` (1) - minor, doesn't prevent acting
- `blinded` (2) - reduces accuracy
- `silenced` (4) - cannot cast spells
- `paralyzed` (32) - cannot act
- `unconscious` (64) - cannot act
- `dead` (128) - fatal

## next steps

### extend the game

1. **add new monsters**: edit `data/monsters.ron`
2. **create new spells**: edit `data/spells.ron`
3. **design items**: edit `data/items.ron`

### explore the codebase

- `src/domain/character.rs` - character system
- `src/domain/combat/engine.rs` - combat mechanics
- `src/domain/magic/casting.rs` - spell casting rules
- `tests/` - integration tests showing complete flows

### run the quality pipeline

before committing changes:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -d warnings
cargo test --all-features
```

## common questions

### q: how do i add a new character class?

a: edit `src/domain/character.rs`, add to the `class` enum, and update
spell/equipment restrictions.

### q: what's the difference between a monster and a character?

a: monsters have ai hints (flee threshold, regeneration), characters have
inventory/equipment.

### q: how does the loot system work?

a: each monster has a `loottable` with gold/gem ranges and item drop
probabilities.

### q: can i change the party size limit?

a: yes, change `party::max_members` constant in `src/domain/character.rs`.

### q: how do i create custom maps?

a: create `.ron` files in `data/maps/` following the map format in
architecture.md.

## troubleshooting

### tests fail with "file not found"

- ensure you're running commands from the project root
- check that data files exist: `ls data/*.ron`

### clippy warnings

- run `cargo clippy` to see suggestions
- most warnings have automatic fixes: `cargo clippy --fix`

### compilation errors after editing ron files

- ron syntax is strict: use correct commas, parentheses
- validate with `cargo test` to catch parse errors

## resources

- **architecture document**: `docs/reference/architecture.md` - complete system
  design
- **implementation guide**: `docs/explanation/implementations.md` - what's been
  built
- **agents guide**: `AGENTS.md` - development rules and patterns

## feedback

found a bug? have a suggestion? this is a learning project - contributions
welcome!

---

**congratulations!** you now understand the basics of antares rpg development.

explore the integration tests for more complex examples, and refer to the
architecture document for detailed specifications.

happy adventuring! ⚔️
