# Antares Turn-Based RPG Architecture Document

## Inspired by Might and Magic 1: The Inner Sanctum

### 1. Executive Summary

This document outlines the architecture for Antares, a classic turn-based RPG
built in Rust, inspired by Might and Magic 1. The game will feature party-based
exploration, turn-based combat, character progression, and dungeon crawling in a
tile-based world.

---

### 2. Core Design Principles

- **Separation of Concerns**: Clear boundaries between game logic, rendering,
  and I/O
- **Data-Driven Design**: Game content defined in external data files
- **Entity-Component Pattern**: Flexible character and monster representation
- **Deterministic Gameplay**: Pure functions for game logic, making save/load
  trivial
- **Rust Best Practices**: Ownership, borrowing, and type safety for robustness

---

### 3. System Architecture

#### 3.1 High-Level Architecture

```text
┌─────────────────────────────────────────────────────────┐
│                    Game Application                     │
├─────────────────────────────────────────────────────────┤
│  Input Handler  │  Game Loop  │  Renderer  │  Audio     │
└────────┬─────────────┬─────────────┬─────────────┬──────┘
         │             │             │             │
    ┌────▼─────────────▼─────────────▼─────────────▼────┐
    │              Game State Manager                    │
    └────┬─────────────┬─────────────┬─────────────┬────┘
         │             │             │             │
    ┌────▼────┐   ┌────▼────┐   ┌────▼────┐   ┌────▼────┐
    │  World  │   │  Party  │   │ Combat  │   │   UI    │
    │ Manager │   │ Manager │   │ System  │   │ System  │
    └────┬────┘   └────┬────┘   └────┬────┘   └────┬────┘
         │             │             │             │
    ┌────▼─────────────▼─────────────▼─────────────▼────┐
    │              Core Game Systems                     │
    │  Character │ Inventory │ Spells │ Skills │ Items   │
    └────────────────────────────────────────────────────┘
```

#### 3.2 Module Structure

```text
src/
├── main.rs                 # Entry point, game loop
├── lib.rs                  # Library root
├── game/
│   ├── mod.rs             # Game state and core loop
│   ├── state.rs           # GameState struct and management
│   └── config.rs          # Game configuration
├── world/
│   ├── mod.rs             # World module exports
│   ├── map.rs             # Map data structures and logic
│   ├── tile.rs            # Tile types and properties
│   ├── location.rs        # Towns, dungeons, outdoor areas
│   └── generator.rs       # Procedural content generation
├── character/
│   ├── mod.rs             # Character module exports
│   ├── party.rs           # Party management
│   ├── roster.rs          # Character roster (pool of available characters)
│   ├── player.rs          # Player character
│   ├── stats.rs           # Attributes, stats, derived values
│   ├── class.rs           # Character classes
│   ├── race.rs            # Character races
│   ├── level.rs           # Experience and leveling
│   ├── status.rs          # Status effects (poison, sleep, etc.)
│   └── flags.rs           # Quest and progression flags per character
</parameter>
├── combat/
│   ├── mod.rs             # Combat module exports
│   ├── engine.rs          # Turn-based combat engine
│   ├── actions.rs         # Combat actions (attack, cast, flee)
│   ├── monster.rs         # Monster definitions
│   ├── encounter.rs       # Encounter generation and management
│   └── ai.rs              # Monster AI behavior
├── magic/
│   ├── mod.rs             # Magic module exports
│   ├── spell.rs           # Spell definitions
│   ├── spellbook.rs       # Character spell management
│   └── effects.rs         # Spell effect implementations
├── inventory/
│   ├── mod.rs             # Inventory module exports
│   ├── item.rs            # Item definitions
│   ├── equipment.rs       # Equipment slots and equipping
│   ├── container.rs       # Inventory container logic
│   └── shop.rs            # Shop and trading
├── ui/
│   ├── mod.rs             # UI module exports
│   ├── render.rs          # Rendering pipeline
│   ├── views.rs           # Different UI views (map, combat, menu)
│   ├── widgets.rs         # Reusable UI components
│   └── text.rs            # Text rendering and formatting
├── io/
│   ├── mod.rs             # I/O module exports
│   ├── input.rs           # Input handling
│   ├── save.rs            # Save/load game
│   └── data_loader.rs     # Load game data from files
└── utils/
    ├── mod.rs             # Utility module exports
    ├── dice.rs            # Dice rolling and RNG
    ├── math.rs            # Math utilities
    └── direction.rs       # Cardinal directions
```

---

### 4. Core Data Structures

#### 4.1 Game State

```rust
pub struct GameState {
    pub world: World,
    pub roster: Roster,
    pub party: Party,
    pub active_spells: ActiveSpells,
    pub mode: GameMode,
    pub time: GameTime,
    pub quests: QuestLog,
}

pub enum GameMode {
    Exploration,
    Combat(CombatState),
    Menu(MenuState),
    Dialogue(DialogueState),
}
```

#### 4.2 World

```rust
pub struct World {
    pub maps: HashMap<MapId, Map>,
    pub current_map: MapId,
    pub party_position: Position,
    pub party_facing: Direction,
}

pub struct Map {
    pub id: MapId,
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Vec<Tile>>,
    pub events: HashMap<Position, Event>,
    pub npcs: Vec<Npc>,
}

pub struct Tile {
    pub terrain: TerrainType,
    pub wall_type: WallType,  // None, Normal, Door, Torch
    pub blocked: bool,
    pub is_special: bool,
    pub is_dark: bool,
    pub visited: bool,
    pub event_trigger: Option<EventId>,
}

pub enum WallType {
    None,
    Normal,
    Door,
    Torch,
}
```

#### 4.3 Character

> **Note:** For comprehensive stat range documentation, see [stat_ranges.md](stat_ranges.md).

```rust
/// Type alias for race identifiers (data-driven)
pub type RaceId = String;

/// Type alias for class identifiers (data-driven)
pub type ClassId = String;

/// Represents a single character (party member or roster character)
///
/// Characters use data-driven race_id and class_id for lookups in
/// RaceDatabase and ClassDatabase respectively. This allows custom
/// races and classes to be defined in RON data files without code changes.
pub struct Character {
    pub name: String,
    pub race_id: RaceId,                // Data-driven race identifier (e.g., "human", "elf")
    pub class_id: ClassId,              // Data-driven class identifier (e.g., "knight", "sorcerer")
    pub sex: Sex,
    pub alignment: Alignment,           // Current alignment
    pub alignment_initial: Alignment,   // Starting alignment (for tracking changes)
    pub level: u32,
    pub experience: u64,
    pub age: u16,                       // Age in years (starts at 18)
    pub age_days: u32,                  // Day counter for aging
    pub stats: Stats,
    pub hp: AttributePair16,            // Hit points (current/base)
    pub sp: AttributePair16,            // Spell points (current/base)
    pub ac: AttributePair,              // Armor class (higher is better, 0-30 range)
    pub spell_level: AttributePair,     // Max spell level castable
    pub inventory: Inventory,           // Backpack items (max 6)
    pub equipment: Equipment,           // Equipped items (max 6)
    pub spells: SpellBook,              // Known spells
    pub conditions: Condition,          // Active status conditions (bitflags)
    pub active_conditions: Vec<ActiveCondition>, // Data-driven conditions
    pub resistances: Resistances,       // Damage resistances
    pub quest_flags: QuestFlags,        // Per-character quest/event tracking
    pub portrait_id: u8,                // Portrait/avatar ID
    pub worthiness: u8,                 // Special quest attribute
    pub gold: u32,                      // Individual gold (0-max)
    pub gems: u32,                      // Individual gems (0-max)
    pub food: u8,                       // Individual food units (max 40, starts at 10)
}

/// Core pattern: base value + current temporary value for buffs/debuffs
/// When saving, save base. When loading, restore current = base.
///
/// Valid ranges for u8 AttributePair:
/// - Primary attributes: 3-255 (ATTRIBUTE_MIN to ATTRIBUTE_MAX)
/// - AC: 0-30 (AC_MIN to AC_MAX)
/// - Resistances: 0-100 (percentage)
pub struct AttributePair {
    pub base: u8,      // Permanent value
    pub current: u8,   // Temporary value (includes buffs/debuffs)
}

impl AttributePair {
    /// Reset temporary value to base value
    pub fn reset(&mut self) {
        self.current = self.base;
    }

    /// Apply a temporary modifier (positive or negative)
    pub fn modify(&mut self, amount: i16) {
        self.current = self.current.saturating_add_signed(amount as i8);
    }
}

/// AttributePair for 16-bit values (HP, SP)
///
/// Valid ranges:
/// - HP: 0-9999 (HP_SP_MIN to HP_SP_MAX)
/// - SP: 0-9999 (HP_SP_MIN to HP_SP_MAX)
pub struct AttributePair16 {
    pub base: u16,     // Permanent value
    pub current: u16,  // Temporary value
}

impl AttributePair16 {
    pub fn reset(&mut self) {
        self.current = self.base;
    }
}

// ===== Stat Range Constants =====
//
// These constants define valid ranges for character statistics.
// See docs/reference/stat_ranges.md for detailed documentation.

pub const ATTRIBUTE_MIN: u8 = 3;      // Minimum primary attribute value
pub const ATTRIBUTE_MAX: u8 = 255;    // Maximum primary attribute value
pub const ATTRIBUTE_DEFAULT: u8 = 10; // Default starting attribute value

pub const HP_SP_MIN: u16 = 0;         // Minimum HP/SP value
pub const HP_SP_MAX: u16 = 9999;      // Maximum HP/SP value

pub const AC_MIN: u8 = 0;             // Minimum Armor Class
pub const AC_MAX: u8 = 30;            // Maximum Armor Class
pub const AC_DEFAULT: u8 = 10;        // Default unarmored AC

pub const LEVEL_MIN: u32 = 1;         // Minimum character level
pub const LEVEL_MAX: u32 = 200;       // Maximum character level

pub const SPELL_LEVEL_MIN: u8 = 1;    // Minimum spell level
pub const SPELL_LEVEL_MAX: u8 = 7;    // Maximum spell level

pub const AGE_MIN: u16 = 18;          // Minimum character age
pub const AGE_MAX: u16 = 200;         // Maximum character age

pub const FOOD_MIN: u8 = 0;           // Minimum food units
pub const FOOD_MAX: u8 = 40;          // Maximum food units
pub const FOOD_DEFAULT: u8 = 10;      // Default starting food

pub const RESISTANCE_MIN: u8 = 0;     // Minimum resistance (0%)
pub const RESISTANCE_MAX: u8 = 100;   // Maximum resistance (100%)

pub const PARTY_MAX_SIZE: usize = 6;      // Maximum party members
pub const ROSTER_MAX_SIZE: usize = 18;    // Maximum roster characters
pub const INVENTORY_MAX_SLOTS: usize = 6; // Inventory slots per character
pub const EQUIPMENT_MAX_SLOTS: usize = 6; // Equipment slots per character

pub const ATTRIBUTE_MODIFIER_MIN: i16 = -255; // Min modifier for effects
pub const ATTRIBUTE_MODIFIER_MAX: i16 = 255;  // Max modifier for effects

/// Primary character attributes
pub struct Stats {
    pub might: AttributePair,        // Physical strength, melee damage
    pub intellect: AttributePair,    // Magical power, spell effectiveness
    pub personality: AttributePair,  // Charisma, social interactions
    pub endurance: AttributePair,    // Constitution, HP calculation
    pub speed: AttributePair,        // Initiative, dodging, turn order
    pub accuracy: AttributePair,     // Hit chance, ranged attacks
    pub luck: AttributePair,         // Critical hits, random events, loot
}

/// Resistances to various damage types and effects
pub struct Resistances {
    pub magic: AttributePair,        // Generic magic resistance
    pub fire: AttributePair,         // Fire damage reduction
    pub cold: AttributePair,         // Cold damage reduction
    pub electricity: AttributePair,  // Lightning damage reduction
    pub acid: AttributePair,         // Acid damage reduction
    pub fear: AttributePair,         // Fear effect resistance
    pub poison: AttributePair,       // Poison resistance
    pub psychic: AttributePair,      // Mental attack resistance
}

/// Active party (max 6 characters)
pub struct Party {
    pub members: Vec<Character>,           // 0-6 active characters
    pub gold: u32,                         // Party gold (can be pooled)
    pub gems: u32,                         // Party gems (can be pooled)
    pub food: u32,                         // Party food supply (deprecated - use character food)
    pub position_index: [bool; 6],         // Combat: which positions can attack
    pub light_units: u8,                   // Available light units for dark areas
}

/// Character roster (character pool)
pub struct Roster {
    pub characters: Vec<Character>,              // Up to 18 characters total
    pub character_locations: Vec<CharacterLocation>, // Where each character's location is stored (InParty, AtInn(InnkeeperId), OnMap(MapId))
}

/// Party-wide active spell effects (separate from character conditions)
/// Each field represents duration remaining (0 = not active)
pub struct ActiveSpells {
    pub fear_protection: u8,      // Resistance to fear effects
    pub cold_protection: u8,      // Cold damage reduction
    pub fire_protection: u8,      // Fire damage reduction
    pub poison_protection: u8,    // Poison resistance
    pub acid_protection: u8,      // Acid damage reduction
    pub electricity_protection: u8, // Lightning resistance
    pub magic_protection: u8,     // Magic damage reduction
    pub light: u8,                // Illumination radius
    pub leather_skin: u8,         // AC bonus
    pub levitate: u8,             // Avoid ground traps
    pub walk_on_water: u8,        // Water traversal
    pub guard_dog: u8,            // Alert for ambushes
    pub psychic_protection: u8,   // Mental attack resistance
    pub bless: u8,                // Combat bonus
    pub invisibility: u8,         // Avoid encounters
    pub shield: u8,               // AC bonus
    pub power_shield: u8,         // Greater AC bonus
    pub cursed: u8,               // Negative effects
}

/// Per-character quest and event tracking
pub struct QuestFlags {
    pub flags: Vec<bool>,                      // Indexed flags for game events
    pub named_flags: HashMap<String, bool>,    // Named flags for clarity
}

// ===== Enums =====

// Note: Race and Class are now data-driven using RaceId and ClassId strings.
// See RaceDatabase and ClassDatabase for loading race/class definitions from RON files.
// Standard races: "human", "elf", "dwarf", "gnome", "half_orc", "half_elf"
// Standard classes: "knight", "paladin", "archer", "cleric", "sorcerer", "robber"

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sex {
    Male,
    Female,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Good,
    Neutral,
    Evil,
}

/// Character conditions (can have multiple via bitflags)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Condition(u8);

impl Condition {
    pub const FINE: u8 = 0;
    pub const ASLEEP: u8 = 1;
    pub const BLINDED: u8 = 2;
    pub const SILENCED: u8 = 4;
    pub const DISEASED: u8 = 8;
    pub const POISONED: u8 = 16;
    pub const PARALYZED: u8 = 32;
    pub const UNCONSCIOUS: u8 = 64;
    pub const DEAD: u8 = 128;
    pub const STONE: u8 = 160;      // Dead + special
    pub const ERADICATED: u8 = 255; // Permanent death

    pub fn is_bad(&self) -> bool {
        self.0 >= Self::PARALYZED
    }

    pub fn is_fatal(&self) -> bool {
        self.0 >= Self::DEAD
    }
}

/// Container for items (backpack or equipped)
pub struct Inventory {
    pub items: Vec<InventorySlot>,  // Max 6 items in backpack
}

impl Inventory {
    pub const MAX_ITEMS: usize = 6;

    pub fn is_full(&self) -> bool {
        self.items.len() >= Self::MAX_ITEMS
    }

    pub fn has_space(&self) -> bool {
        self.items.len() < Self::MAX_ITEMS
    }
}

pub struct InventorySlot {
    pub item_id: ItemId,
    pub charges: u8,  // For charged items (0 = useless, must have 1+ to recharge)
}

/// Character's known spells organized by school and level
pub struct SpellBook {
    pub cleric_spells: [Vec<SpellId>; 7],   // Cleric spells by level (1-7)
    pub sorcerer_spells: [Vec<SpellId>; 7], // Sorcerer spells by level (1-7)
}

impl SpellBook {
    /// Returns the appropriate spell list for the character's class
    ///
    /// Uses class_id to look up spell school in ClassDatabase.
    pub fn get_spell_list_for_class(&self, class_id: &str, class_db: &ClassDatabase) -> &[Vec<SpellId>; 7] {
        if let Some(class_def) = class_db.get_class(class_id) {
            match class_def.spell_school {
                Some(SpellSchool::Cleric) => &self.cleric_spells,
                Some(SpellSchool::Sorcerer) => &self.sorcerer_spells,
                None => &self.sorcerer_spells, // Non-casters default
            }
        } else {
            &self.sorcerer_spells // Unknown class defaults
        }
    }

    /// Check if class can cast spells of this school
    pub fn can_cast_school(class_id: &str, school: SpellSchool, class_db: &ClassDatabase) -> bool {
        if let Some(class_def) = class_db.get_class(class_id) {
            class_def.spell_school == Some(school)
        } else {
            false
        }
    }
}

/// Equipped items in specific slots
pub struct Equipment {
    pub weapon: Option<ItemId>,
    pub armor: Option<ItemId>,
    pub shield: Option<ItemId>,
    pub helmet: Option<ItemId>,
    pub boots: Option<ItemId>,
    pub accessory1: Option<ItemId>,
    pub accessory2: Option<ItemId>,
}

impl Equipment {
    pub const MAX_EQUIPPED: usize = 6;

    /// Count currently equipped items
    pub fn equipped_count(&self) -> usize {
        [&self.weapon, &self.armor, &self.shield, &self.helmet,
         &self.boots, &self.accessory1]
            .iter()
            .filter(|slot| slot.is_some())
            .count()
    }
}
```

#### 4.4 Combat

````rust
pub struct CombatState {
    pub participants: Vec<Combatant>,
    pub turn_order: Vec<CombatantId>,
    pub current_turn: usize,
    pub round: u32,
    pub status: CombatStatus,
    pub handicap: Handicap,  // Party advantage, monster advantage, or even
    pub can_flee: bool,
    pub can_surrender: bool,
    pub can_bribe: bool,
    pub monsters_advance: bool,
    pub monsters_regenerate: bool,
}

pub enum Handicap {
    PartyAdvantage,
    MonsterAdvantage,
    Even,
}

pub enum Combatant {
    Player(Character),
    Monster(Monster),
}

pub struct Monster {
    pub name: String,
    pub stats: Stats,
    pub hp: Health,
    pub ac: ArmorClass,
    pub attacks: Vec<Attack>,
    /// Experience awarded for defeating a monster is stored in the monster's loot table:
    /// `loot.experience`.
    pub loot: LootTable,
    pub flee_threshold: u8,
    pub special_attack_threshold: u8,  // Percentage chance
    pub resistances: MonsterResistances,
    pub can_regenerate: bool,
    pub can_advance: bool,
    pub is_undead: bool,
    pub magic_resistance: u8,
    // Runtime combat state
    pub conditions: MonsterCondition,
    pub has_acted: bool,
}

pub struct MonsterResistances {
    pub physical: bool,
    pub fire: bool,
    pub cold: bool,
    pub electricity: bool,
    pub energy: bool,
    pub paralysis: bool,
    pub fear: bool,
    pub sleep: bool,
}

pub enum MonsterCondition {
    Normal,
    Paralyzed,
    Webbed,
    Held,
    Asleep,
    Mindless,
    Silenced,
    Blinded,
    Afraid,
    Dead,
}
#### 4.5 Items and Equipment

```rust
/// Item definition
pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub item_type: ItemType,
    pub base_cost: u32,              // Purchase price (in gold pieces)
    pub sell_cost: u32,              // Sell price (typically base_cost / 2)
    pub disablements: Disablement,   // DEPRECATED: Legacy class/alignment restrictions (bitflags)
                                     // Use classification and tags instead
    pub tags: Vec<String>,           // Fine-grained item tags for race restrictions
                                     // Examples: "large_weapon", "heavy_armor", "elven_crafted"
    pub alignment_restriction: Option<AlignmentRestriction>, // Good/Evil/Any alignment requirement
    pub constant_bonus: Option<Bonus>, // Permanent bonus when equipped
    pub temporary_bonus: Option<Bonus>, // Charged/temporary bonus
    pub spell_effect: Option<SpellId>,  // Usable spell
    pub max_charges: u8,             // Max charges for magical items (0 = not rechargeable)
    pub is_cursed: bool,             // Cannot be unequipped
    pub icon_path: Option<String>,   // Optional icon file path
}

impl Item {
    /// Returns true if the item is magical (for Detect Magic spell)
    pub fn is_magical(&self) -> bool {
        self.constant_bonus.is_some()
            || self.temporary_bonus.is_some()
            || self.spell_effect.is_some()
            || self.max_charges > 0
            || self.is_cursed
    }

    /// Returns true if the item has usable charges
    pub fn has_charges(&self) -> bool {
        self.max_charges > 0
    }

    /// Returns true if the item can be used (has effect and charges > 0)
    pub fn can_use(&self, current_charges: u8) -> bool {
        (self.spell_effect.is_some() || self.temporary_bonus.is_some())
            && current_charges > 0
    }

    /// Returns true if the item can be recharged (has 1+ charges remaining)
    pub fn can_recharge(&self, current_charges: u8) -> bool {
        self.max_charges > 0 && current_charges > 0
    }

    /// Returns true if item is useless (has max charges but current is 0)
    pub fn is_useless(&self, current_charges: u8) -> bool {
        self.max_charges > 0 && current_charges == 0
    }

    /// Get the sell price (typically half of base cost)
    pub fn get_sell_price(&self) -> u32 {
        self.sell_cost
    }

    /// Returns the required proficiency for this item based on its classification
    /// Returns None if item has no proficiency requirement
    pub fn required_proficiency(&self) -> Option<String> {
        match &self.item_type {
            ItemType::Weapon(data) => data.classification.map(|c| c.to_proficiency_id()),
            ItemType::Armor(data) => data.classification.map(|c| c.to_proficiency_id()),
            ItemType::Accessory(data) => data.classification.map(|c| c.to_proficiency_id()),
            _ => None,
        }
    }

    /// Returns true if the item can be used by the specified alignment
    pub fn can_use_alignment(&self, alignment: Alignment) -> bool {
        match self.alignment_restriction {
            None => true, // No restriction
            Some(AlignmentRestriction::GoodOnly) => alignment == Alignment::Good,
            Some(AlignmentRestriction::EvilOnly) => alignment == Alignment::Evil,
        }
    }
}

/// Alignment restrictions for items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentRestriction {
    GoodOnly,  // Can only be used by Good characters
    EvilOnly,  // Can only be used by Evil characters
}

/// Item categories
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemType {
    Weapon(WeaponData),
    Armor(ArmorData),
    Accessory(AccessoryData),
    Consumable(ConsumableData),
    Ammo(AmmoData),
    Quest(QuestData),          // Quest items
}

pub struct WeaponData {
    pub damage: DiceRoll,      // Base damage (e.g., 1d8)
    pub bonus: i8,             // Bonus to-hit AND damage
    pub hands_required: u8,    // 1 or 2
    pub classification: Option<WeaponClassification>, // Weapon type for proficiency
}

pub struct ArmorData {
    pub ac_bonus: u8,          // AC improvement
    pub weight: u8,            // Weight in pounds
    pub classification: Option<ArmorClassification>, // Armor type for proficiency
}

pub struct AccessoryData {
    pub slot: AccessorySlot,   // Which slot it goes in (Ring, Amulet, etc.)
    pub classification: Option<MagicItemClassification>, // Magic item type
}

pub struct ConsumableData {
    pub effect: ConsumableEffect, // What the consumable does
    pub is_combat_usable: bool,   // Can be used during combat
}

pub struct AmmoData {
    pub ammo_type: AmmoType,   // Arrow, Bolt, etc.
    pub quantity: u16,         // Number of projectiles
}

pub struct QuestData {
    pub quest_id: String,
    pub is_key_item: bool,     // Required for quest progression
}

/// Dice roll for damage, healing, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiceRoll {
    pub dice: u8,              // Number of dice
    pub sides: u8,             // Sides per die
    pub bonus: i16,            // Flat bonus added to roll (not modifier)
}

/// Item bonuses (stat modifications)
#[derive(Debug, Clone, Copy)]
pub struct Bonus {
    pub attribute: BonusAttribute,
    pub value: i8,             // Can be negative for cursed items
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BonusAttribute {
    Might,
    Intellect,
    Personality,
    Endurance,
    Speed,
    Accuracy,
    Luck,
    AC,
    HP,
    SP,
    Level,
    ResistMagic,
    ResistFire,
    ResistCold,
    ResistElectricity,
    ResistAcid,
    ResistFear,
    ResistPoison,
    ResistPsychic,
}

/// DEPRECATED: Class and alignment restrictions (bitflags)
/// Use classification and tags system instead
#[derive(Debug, Clone, Copy)]
pub struct Disablement(u8);

impl Disablement {
    pub const KNIGHT: u8 = 0b00100000;
    pub const PALADIN: u8 = 0b00010000;
    pub const ARCHER: u8 = 0b00001000;
    pub const CLERIC: u8 = 0b00000100;
    pub const SORCERER: u8 = 0b00000010;
    pub const ROBBER: u8 = 0b00000001;
    pub const GOOD: u8 = 0b10000000;
    pub const EVIL: u8 = 0b01000000;
    pub const NEUTRAL: u8 = Self::GOOD | Self::EVIL;
}

/// Consumable effect variants (parameterized)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsumableEffect {
    HealHp(u16),                         // Restore N hit points
    RestoreSp(u16),                      // Restore N spell points
    CureCondition(u8),                   // Remove condition by bit flag
    BoostAttribute(AttributeType, i8),   // Temporary stat modifier
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeType {
    Might,
    Intellect,
    Personality,
    Endurance,
    Speed,
    Accuracy,
    Luck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmmoType {
    Arrow,
    Bolt,
    Stone,
}
```

#### 4.5.1 Item Classifications

Item classifications determine proficiency requirements and provide type-safe categorization.

```rust
/// Weapon classifications map to proficiency requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponClassification {
    Simple,         // Simple weapons (daggers, clubs) → "simple_weapon"
    MartialMelee,   // Martial melee weapons (swords, axes) → "martial_melee"
    MartialRanged,  // Martial ranged weapons (longbows, crossbows) → "martial_ranged"
    Blunt,          // Blunt weapons (maces, flails) → "blunt_weapon"
    Unarmed,        // Unarmed combat → "unarmed"
}

impl WeaponClassification {
    pub fn to_proficiency_id(&self) -> String {
        match self {
            WeaponClassification::Simple => "simple_weapon".to_string(),
            WeaponClassification::MartialMelee => "martial_melee".to_string(),
            WeaponClassification::MartialRanged => "martial_ranged".to_string(),
            WeaponClassification::Blunt => "blunt_weapon".to_string(),
            WeaponClassification::Unarmed => "unarmed".to_string(),
        }
    }
}

/// Armor classifications map to proficiency requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorClassification {
    Light,   // Light armor (leather, padded) → "light_armor"
    Medium,  // Medium armor (chainmail, scale) → "medium_armor"
    Heavy,   // Heavy armor (plate, full plate) → "heavy_armor"
    Shield,  // Shields → "shield"
}

impl ArmorClassification {
    pub fn to_proficiency_id(&self) -> String {
        match self {
            ArmorClassification::Light => "light_armor".to_string(),
            ArmorClassification::Medium => "medium_armor".to_string(),
            ArmorClassification::Heavy => "heavy_armor".to_string(),
            ArmorClassification::Shield => "shield".to_string(),
        }
    }
}

/// Magic item classifications for accessories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagicItemClassification {
    Arcane,     // Arcane magic items (wands, staves) → "arcane_item"
    Divine,     // Divine magic items (holy symbols, relics) → "divine_item"
    Elemental,  // Elemental magic items → "elemental_item"
}

impl MagicItemClassification {
    pub fn to_proficiency_id(&self) -> String {
        match self {
            MagicItemClassification::Arcane => "arcane_item".to_string(),
            MagicItemClassification::Divine => "divine_item".to_string(),
            MagicItemClassification::Elemental => "elemental_item".to_string(),
        }
    }
}

/// Accessory equipment slots
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessorySlot {
    Ring,    // Ring slot
    Amulet,  // Amulet/necklace slot
    Belt,    // Belt slot
    Cloak,   // Cloak/cape slot
}
```

**Standard Item Tags:**

Tags provide fine-grained item restrictions that races can declare incompatible:

- `large_weapon` - Large/oversized weapons (restricted for small races)
- `two_handed` - Two-handed weapons
- `heavy_armor` - Heavy armor pieces
- `elven_crafted` - Elven-crafted items
- `dwarven_crafted` - Dwarven-crafted items
- `requires_strength` - Items requiring high strength

**Standard Proficiency IDs:**

- Weapons: `simple_weapon`, `martial_melee`, `martial_ranged`, `blunt_weapon`, `unarmed`
- Armor: `light_armor`, `medium_armor`, `heavy_armor`, `shield`
- Magic Items: `arcane_item`, `divine_item`
````

#### 4.6 Supporting Types

```rust
/// Type aliases for clarity
pub type ItemId = u8;        // Valid range: 0-255
pub type SpellId = u16;      // High byte = school, low byte = spell number
pub type MonsterId = u8;
pub type MapId = u16;
pub type CharacterId = usize;
pub type InnkeeperId = String;
pub type EventId = u16;
pub type CharacterDefinitionId = String;
pub type RaceId = String;
pub type ClassId = String;

/// 2D position on a map
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}
```

#### 4.6.1 Class and Race Definitions - Proficiency System

Classes and races now use the proficiency system for item restrictions:

```rust
/// Class definition with proficiency support
pub struct ClassDefinition {
    pub id: ClassId,
    pub display_name: String,
    pub hp_die: u8,
    pub spell_access: SpellAccess,
    pub proficiencies: Vec<String>,           // NEW: List of proficiency IDs
    pub disablement_bit_index: Option<u8>,    // DEPRECATED: Legacy field
    // ... other fields
}

impl ClassDefinition {
    /// Check if class has specific proficiency
    pub fn has_proficiency(&self, proficiency_id: &str) -> bool {
        self.proficiencies.iter().any(|p| p == proficiency_id)
    }
}

/// Race definition with proficiency and tag restrictions
pub struct RaceDefinition {
    pub id: RaceId,
    pub display_name: String,
    pub size_category: SizeCategory,
    pub proficiencies: Vec<String>,           // NEW: List of proficiency IDs
    pub incompatible_item_tags: Vec<String>,  // NEW: Tags this race cannot use
    pub disablement_bit_index: Option<u8>,    // DEPRECATED: Legacy field
    // ... other fields
}

impl RaceDefinition {
    /// Check if race can use an item (no incompatible tags)
    pub fn can_use_item(&self, item: &Item) -> bool {
        !item.tags.iter().any(|tag| self.incompatible_item_tags.contains(tag))
    }
}
```

**Migration Notes:**

- Old data files without `proficiencies` field will default to empty vector via `#[serde(default)]`
- Legacy `disablement_bit_index` preserved for backward compatibility
- New system uses proficiency IDs and tags instead of bit flags

#### 4.7 Character Definition (Data-Driven Templates)

Character definitions are data-driven templates stored in RON files that can be
instantiated into runtime `Character` objects. This separates character templates
(campaign data) from character instances (runtime state).

```rust
/// Starting equipment specification for character definitions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StartingEquipment {
    pub weapon: Option<ItemId>,
    pub armor: Option<ItemId>,
    pub shield: Option<ItemId>,
    pub helmet: Option<ItemId>,
    pub boots: Option<ItemId>,
    pub accessory1: Option<ItemId>,
    pub accessory2: Option<ItemId>,
}

impl StartingEquipment {
    pub fn is_empty(&self) -> bool {
        self.weapon.is_none() && self.armor.is_none() && self.shield.is_none()
            && self.helmet.is_none() && self.boots.is_none()
            && self.accessory1.is_none() && self.accessory2.is_none()
    }

    pub fn equipped_count(&self) -> usize {
        [&self.weapon, &self.armor, &self.shield, &self.helmet,
         &self.boots, &self.accessory1, &self.accessory2]
            .iter()
            .filter(|slot| slot.is_some())
            .count()
    }

    pub fn all_item_ids(&self) -> Vec<ItemId> {
        [self.weapon, self.armor, self.shield, self.helmet,
         self.boots, self.accessory1, self.accessory2]
            .iter()
            .filter_map(|&id| id)
            .collect()
    }
}

/// Base stats for character definitions (before race/class modifiers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseStats {
    pub might: u8,
    pub intellect: u8,
    pub personality: u8,
    pub endurance: u8,
    pub speed: u8,
    pub accuracy: u8,
    pub luck: u8,
}

impl Default for BaseStats {
    fn default() -> Self {
        Self {
            might: 10,
            intellect: 10,
            personality: 10,
            endurance: 10,
            speed: 10,
            accuracy: 10,
            luck: 10,
        }
    }
}

/// Data-driven character template for premade characters and NPCs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterDefinition {
    pub id: CharacterDefinitionId,
    pub name: String,
    pub race_id: RaceId,
    pub class_id: ClassId,
    pub sex: Sex,
    pub alignment: Alignment,
    pub base_stats: BaseStats,
    #[serde(default)]
    pub portrait_id: u8,
    #[serde(default)]
    pub starting_gold: u32,
    #[serde(default)]
    pub starting_gems: u32,
    #[serde(default = "default_starting_food")]
    pub starting_food: u32,
    #[serde(default)]
    pub starting_items: Vec<ItemId>,
    #[serde(default)]
    pub starting_equipment: StartingEquipment,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub is_premade: bool,
}

fn default_starting_food() -> u32 { 10 }

impl CharacterDefinition {
    /// Creates a runtime Character from this definition
    pub fn instantiate(
        &self,
        races: &RaceDatabase,
        classes: &ClassDatabase,
        items: &ItemDatabase,
    ) -> Result<Character, CharacterDefinitionError> {
        // 1. Validate references exist
        // 2. Convert race_id/class_id to enums
        // 3. Apply race stat modifiers to base_stats
        // 4. Calculate starting HP (max roll of class HP die + endurance mod)
        // 5. Calculate starting SP (based on class spell_stat)
        // 6. Apply race resistances
        // 7. Populate inventory with starting_items
        // 8. Create equipment from starting_equipment
        // 9. Return fully initialized Character
    }
}

/// Database of character definitions loaded from RON files
pub struct CharacterDatabase {
    characters: HashMap<CharacterDefinitionId, CharacterDefinition>,
}

impl CharacterDatabase {
    pub fn load_from_file(path: &str) -> Result<Self, CharacterDefinitionError>;
    pub fn load_from_string(ron: &str) -> Result<Self, CharacterDefinitionError>;
    pub fn get_character(&self, id: &str) -> Option<&CharacterDefinition>;
    pub fn all_characters(&self) -> impl Iterator<Item = &CharacterDefinition>;
    pub fn premade_characters(&self) -> impl Iterator<Item = &CharacterDefinition>;
    pub fn template_characters(&self) -> impl Iterator<Item = &CharacterDefinition>;
    pub fn validate(&self) -> Result<(), CharacterDefinitionError>;
}

/// Errors for character definition operations
#[derive(Debug, Error)]
pub enum CharacterDefinitionError {
    #[error("Character not found: {0}")]
    CharacterNotFound(String),
    #[error("Failed to load character definitions: {0}")]
    LoadError(String),
    #[error("Failed to parse RON: {0}")]
    ParseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Duplicate character ID: {0}")]
    DuplicateId(String),
    #[error("Invalid race ID '{race_id}' for character '{character_id}'")]
    InvalidRaceId { character_id: String, race_id: String },
    #[error("Invalid class ID '{class_id}' for character '{character_id}'")]
    InvalidClassId { character_id: String, class_id: String },
    #[error("Invalid item ID '{item_id}' for character '{character_id}'")]
    InvalidItemId { character_id: String, item_id: ItemId },
    #[error("Instantiation error for '{character_id}': {message}")]
    InstantiationError { character_id: String, message: String },
    #[error("Inventory full for '{character_id}' when adding item {item_id}")]
    InventoryFull { character_id: String, item_id: ItemId },
}
```

**Instantiation Flow:**

The `CharacterDefinition::instantiate()` method bridges data-driven templates
with runtime Character instances:

1. **Validation**: Verify race_id, class_id, and all item IDs exist in databases
2. **Enum Conversion**: Convert string IDs to Race/Class enums
3. **Stat Application**: Apply race modifiers to base stats (clamped to 3-25)
4. **HP Calculation**: Max roll of class HP die + (endurance - 10) / 2, minimum 1
5. **SP Calculation**: Based on class spell_stat (Intellect or Personality)
6. **Resistance Setup**: Copy race resistances to character
7. **Inventory Population**: Add starting_items to inventory
8. **Equipment Setup**: Map starting_equipment slots to equipment slots
9. **Return**: Fully initialized Character ready for gameplay

```rust
// Example: Instantiate a premade character
let races = RaceDatabase::load_from_file("data/races.ron")?;
let classes = ClassDatabase::load_from_file("data/classes.ron")?;
let items = ItemDatabase::load_from_file("data/items.ron")?;
let characters = CharacterDatabase::load_from_file("data/characters.ron")?;

let knight_def = characters.get_character("pregen_human_knight").unwrap();
let knight = knight_def.instantiate(&races, &classes, &items)?;

assert_eq!(knight.name, "Sir Galahad");
assert_eq!(knight.race, Race::Human);
assert_eq!(knight.class, Class::Knight);
}

/// Cardinal directions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub fn turn_left(&self) -> Direction {
        match self {
            Direction::North => Direction::West,
            Direction::West => Direction::South,
            Direction::South => Direction::East,
            Direction::East => Direction::North,
        }
    }

    pub fn turn_right(&self) -> Direction {
        match self {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
        }
    }

    pub fn forward(&self, pos: Position) -> Position {
        match self {
            Direction::North => Position { x: pos.x, y: pos.y - 1 },
            Direction::East => Position { x: pos.x + 1, y: pos.y },
            Direction::South => Position { x: pos.x, y: pos.y + 1 },
            Direction::West => Position { x: pos.x - 1, y: pos.y },
        }
    }
}

/// Dice roll specification (e.g., 2d6+3)
#[derive(Debug, Clone, Copy)]
pub struct DiceRoll {
    pub count: u8,   // Number of dice
    pub sides: u8,   // Die size (d4, d6, d8, d10, d12, d20)
    pub bonus: i8,   // Fixed bonus/penalty
}

impl DiceRoll {
    pub fn new(count: u8, sides: u8, bonus: i8) -> Self {
        Self { count, sides, bonus }
    }

    pub fn roll(&self, rng: &mut impl rand::Rng) -> i32 {
        let mut total = self.bonus as i32;
        for _ in 0..self.count {
            total += rng.gen_range(1..=self.sides as i32);
        }
        total.max(0)
    }
}

/// Game time tracking
pub struct GameTime {
    pub day: u32,
    pub hour: u8,     // 0-23
    pub minute: u8,   // 0-59
}

impl GameTime {
    pub fn advance_minutes(&mut self, minutes: u32) {
        self.minute += (minutes % 60) as u8;
        let hours = minutes / 60 + (self.minute / 60) as u32;
        self.minute %= 60;

        self.hour += (hours % 24) as u8;
        let days = hours / 24 + (self.hour / 24) as u32;
        self.hour %= 24;

        self.day += days;
    }
}

/// Quest log tracking
pub struct QuestLog {
    pub active_quests: Vec<Quest>,
    pub completed_quests: Vec<QuestId>,
}

pub struct Quest {
    pub id: QuestId,
    pub name: String,
    pub description: String,
    pub objectives: Vec<QuestObjective>,
}

pub struct QuestObjective {
    pub description: String,
    pub completed: bool,
}

pub type QuestId = String;

/// Loot table for monsters
pub struct LootTable {
    pub gold: (u32, u32),                    // Min/max gold
    pub gems: Option<(u8, u8)>,              // Min/max gems (if any)
    pub items: Vec<(f32, ItemId)>,           // (probability, item_id)
    pub experience: u32,                     // Base XP value
}

/// Attack definition for monsters
pub struct Attack {
    pub damage: DiceRoll,
    pub attack_type: AttackType,
    pub special_effect: Option<SpecialEffect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackType {
    Physical,
    Fire,
    Cold,
    Electricity,
    Acid,
    Poison,
    Energy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialEffect {
    Poison,
    Disease,
    Paralysis,
    Sleep,
    Drain,      // Level/stat drain
    Stone,
    Death,
}

/// Spell school identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellSchool {
    Cleric,    // Divine magic, healing, protection
    Sorcerer,  // Arcane magic, offense, utility
}

/// Complete spell definition
pub struct Spell {
    pub id: SpellId,
    pub name: String,
    pub school: SpellSchool,
    pub level: u8,                    // 1-7
    pub sp_cost: u16,                 // Base SP cost
    pub gem_cost: u16,                // Gem cost (0 if none)
    pub context: SpellContext,        // When/where castable
    pub target: SpellTarget,          // Who/what it affects
    pub description: String,
}

/// Spell casting context restrictions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellContext {
    Anytime,           // Can cast in or out of combat
    CombatOnly,        // Only during combat
    NonCombatOnly,     // Only outside combat
    OutdoorOnly,       // Only in outdoor areas
    IndoorOnly,        // Only in indoor areas
    OutdoorCombat,     // Combat in outdoor areas only
}

/// Spell target type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellTarget {
    Self_,             // Caster only
    SingleCharacter,   // One party member
    AllCharacters,     // Entire party
    SingleMonster,     // One enemy
    MonsterGroup,      // Multiple enemies (up to N)
    AllMonsters,       // All enemies
    SpecificMonsters,  // Subset based on type (e.g., undead)
}

/// Spell casting result
pub struct SpellResult {
    pub success: bool,
    pub effect_message: String,
    pub damage: Option<i32>,          // For damage spells
    pub healing: Option<i32>,         // For healing spells
    pub affected_targets: Vec<usize>, // Indices of affected targets
}
```

---

### 5. Key Systems

#### 5.1 Turn-Based Combat System

**Flow:**

1. **Encounter Initiation**: Random encounters or scripted battles
2. **Initiative Roll**: Determine turn order based on Speed stat
3. **Turn Execution**: Each combatant takes action in order
4. **Action Resolution**: Calculate hit/miss, damage, effects
5. **Round End**: Check for combat end conditions
6. **Victory/Defeat**: Distribute XP and loot or handle party death

**Combat Actions:**

- Attack (melee/ranged)
- Cast Spell
- Use Item
- Defend (increase AC)
- Flee (chance-based)

#### 5.2 Character Progression

**Experience System:**

- Kill monsters → gain XP
- XP thresholds for leveling (exponential curve)
- Level up → improve stats, gain HP/SP, learn spells

**Class System:**

- Fighter, Paladin, Archer, Cleric, Sorcerer, Theif
- Class determines: weapon/armor restrictions, spell access, stat growth

#### 5.3 Magic System

**Dual Spell Schools:**

MM1 features two completely separate spell systems:

1. **Cleric Spells** (Divine Magic)

   - 47 spells across 7 levels
   - Cast by: Clerics, Paladins (at higher levels)
   - SP based on: Personality attribute
   - Focus: Healing, protection, support, turn undead

2. **Sorcerer Spells** (Arcane Magic)
   - 47 spells across 7 levels
   - Cast by: Sorcerers, Archers (at higher levels)
   - SP based on: Intellect attribute
   - Focus: Damage, debuffs, utility, transformation

**Spell Point Calculation:**

```rust
pub fn calculate_spell_points(character: &Character) -> u16 {
    match character.class {
        Class::Cleric | Class::Paladin => {
            // SP based on Personality
            calculate_sp_from_stat(character.stats.personality.base, character.level)
        }
        Class::Sorcerer | Class::Archer => {
            // SP based on Intellect
            calculate_sp_from_stat(character.stats.intellect.base, character.level)
        }
        _ => 0, // Non-spellcasting classes
    }
}

fn calculate_sp_from_stat(stat: u8, level: u32) -> u16 {
    // Base SP increases with both stat and level
    // Formula: (stat - 10) * level + base_per_level
    let stat_bonus = (stat as i16 - 10).max(0) as u16;
    let base_sp = level as u16 * 2; // 2 SP per level minimum
    base_sp + (stat_bonus * level as u16 / 2)
}
```

**Spell Access by Level:**

- Level 1: Can cast level 1 spells
- Level 3: Can cast level 2 spells
- Level 5: Can cast level 3 spells
- Level 7: Can cast level 4 spells
- Level 9: Can cast level 5 spells
- Level 11: Can cast level 6 spells
- Level 13+: Can cast level 7 spells

**Delayed Spell Access:**

- **Paladins**: Gain Cleric spell access at higher experience levels
- **Archers**: Gain Sorcerer spell access at higher experience levels
- Pure casters (Cleric/Sorcerer) start with immediate spell access

**Class-Specific Spell Restrictions:**

```rust
pub fn can_cast_spell(character: &Character, spell: &Spell) -> bool {
    // Check spell school matches character class
    match (character.class, spell.school) {
        (Class::Cleric, SpellSchool::Cleric) => true,
        (Class::Paladin, SpellSchool::Cleric) => {
            // Paladins need higher level for spell access
            character.level >= 3
        }
        (Class::Sorcerer, SpellSchool::Sorcerer) => true,
        (Class::Archer, SpellSchool::Sorcerer) => {
            // Archers need higher level for spell access
            character.level >= 3
        }
        _ => false,
    }
}
```

**Spell Costs:**

- Base cost usually equals spell level (Level 1 = 1 SP, Level 2 = 2 SP, etc.)
- Power spells cost: caster level + gems
- High-level spells often require gems in addition to SP
- Example costs:
  - Level 1 spells: 1 SP
  - Level 4 spells: 4 SP + 2 gems (typical)
  - Level 7 spells: 7 SP + 5-10 gems (typical)

**Spell Casting:**

- Can fizzle based on caster's primary stat (Intellect/Personality)
- Context restrictions enforced (combat-only, outdoor-only, etc.)
- Some monsters resist or reflect spells
- Dispel Magic cancels all active spells

**Spell Validation:**

```rust
pub enum SpellState {
    Ok,
    NotEnoughSP,
    NotEnoughGems,
    WrongClass,          // Class cannot cast this spell school
    LevelTooLow,         // Character level insufficient
    CombatOnly,
    NonCombatOnly,
    DoesntWork,
    OutdoorsOnly,
    IndoorOnly,
    MagicForbidden,
}

pub struct SpellCast {
    pub spell_id: SpellId,
    pub spell_school: SpellSchool,
    pub caster: CharacterId,
    pub target: Option<TargetId>,
    pub sp_cost: u16,
    pub gem_cost: u16,
    pub state: SpellState,
}
```

**Spell Categories:**

_Cleric Spells:_

- Healing (First Aid, Cure Wounds, Raise Dead)
- Protection (Protection From X, Bless)
- Status Cure (Cure Poison, Cure Disease, Cure Paralysis)
- Combat Support (Turn Undead, Holy Word)
- Utility (Create Food, Light, Surface, Town Portal)

_Sorcerer Spells:_

- Direct Damage (Flame Arrow, Fireball, Lightning Bolt)
- Area Effects (Meteor Shower, Acid Rain)
- Debuffs (Sleep, Slow, Weaken, Web)
- Buffs (Power, Quickness, Shield)
- Utility (Location, Fly, Teleport, Detect Magic)

</text>

<old_text line=287> **Events:**

- Encounters (monsters)
- NPCs (quests, dialogue)
- Treasures (chests, loot)
- Traps (damage, status effects)
- Teleporters
- Doors (locked/unlocked)

#### 5.4 Map and Movement

**First-Person Perspective:**

- Party occupies single tile
- Faces one of four cardinal directions
- Turn left/right, move forward/backward
- Strafe left/right

**Map Types:**

- Outdoor (wilderness, roads)
- Towns (shops, inns, temples)
- Dungeons (multi-level, dangerous)

**Events:**

- Encounters (monsters)
- NPCs (quests, dialogue)
- Treasures (chests, loot)
- Traps (damage, status effects)
- Teleporters
- Doors (locked/unlocked)

#### 5.5 Saving and Loading

**Serialization Strategy:**

- Use `serde` for JSON or binary serialization
- Save entire GameState (including Roster + Party + ActiveSpells)
- Include version number for compatibility
- Save AttributePair base values, restore to current on load
- Auto-save on location change
- Manual save at inns

**Note:** When loading, reset all temporary attribute values to base values.

---

### 6. Technology Stack

#### 6.1 Core Libraries

- **Game Loop**: `winit` + custom loop or `ggez`/`bracket-lib`
- **Rendering**:
  - Text mode: `crossterm` or `termion` for terminal UI
  - Graphics mode: `pixels` + custom rendering or `ggez`
- **Serialization**: `serde` + `serde_json` or `bincode`
- **Random Numbers**: `rand` crate
- **Audio** (optional): `rodio` or `kira`
- **Configuration**: `toml` or `ron` for data files

#### 6.2 Rendering Approaches

**Option 1: Terminal/ASCII** (Most MM1-authentic)

- Pros: Fast development, nostalgic, portable
- Cons: Limited visual appeal

**Option 2: Tile-Based 2D**

- Pros: Clear visuals, flexible
- Cons: Requires art assets

**Option 3: Hybrid**

- 2D tiles for map
- Character/portrait art for menus
- ASCII for text

---

### 7. Data-Driven Content

#### 7.1 External Data Files

```text
data/
├── monsters.ron          # Monster definitions
├── items.ron            # Item database
├── spells.ron           # Spell definitions
├── characters.ron       # Character definition templates (premade, NPCs)
├── maps/
│   ├── town_sorpigal.ron
│   ├── dungeon_1.ron
│   └── overworld.ron
├── classes.ron          # Class definitions
├── races.ron            # Race definitions
└── dialogue.ron         # NPC dialogue trees

campaigns/
└── <campaign_name>/
    └── data/
        └── characters.ron  # Campaign-specific character definitions
```

#### 7.2 Example Data Format (RON)

**Monster Definition:**

```ron
// monsters.ron
[
    Monster(
        id: "goblin",
        name: "Goblin",
        stats: Stats(
            might: 6,
            intellect: 3,
            endurance: 5,
            speed: 8,
            accuracy: 6,
            luck: 4,
        ),
        hp: 8,
        ac: 10,
        attacks: ["club"],
        xp_value: 10,
        loot: LootTable(
            gold: (1, 10),
            items: [(0.1, "club"), (0.05, "leather_armor")],
        ),
    ),
]
```

**Item Definitions (Examples from MM1):**

```ron
// items.ron - Non-magical items
[
    Item(
        id: 1,
        name: "Club",
        item_type: Weapon(WeaponData(
            damage: DiceRoll(1, 3, 0),  // 1d3 damage
            bonus: 0,                    // No bonus
            hands_required: 1,
        )),
        base_cost: 1,
        sell_cost: 0,
        disablements: Disablement(0xFF),  // All classes can use
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
    ),

    // Simple enhanced item
    Item(
        id: 2,
        name: "Club +1",
        item_type: Weapon(WeaponData(
            damage: DiceRoll(1, 3, 0),
            bonus: 1,                    // +1 to-hit and damage
            hands_required: 1,
        )),
        base_cost: 30,
        sell_cost: 15,
        disablements: Disablement(0xFF),
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
    ),

    // Magical item with equip bonus
    Item(
        id: 3,
        name: "Battle Axe +1",
        item_type: Weapon(WeaponData(
            damage: DiceRoll(1, 8, 0),
            bonus: 1,
            hands_required: 1,
        )),
        base_cost: 300,
        sell_cost: 150,
        disablements: Disablement(0b00111001), // KPAR
        constant_bonus: Some(Bonus(
            attribute: ResistFire,
            value: 20,                   // Fire +20%
        )),
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
    ),

    // Magical item with use power
    Item(
        id: 4,
        name: "Flaming Club",
        item_type: Weapon(WeaponData(
            damage: DiceRoll(1, 3, 0),
            bonus: 3,
            hands_required: 1,
        )),
        base_cost: 500,
        sell_cost: 250,
        disablements: Disablement(0xFF),
        constant_bonus: Some(Bonus(
            attribute: ResistFire,
            value: 20,
        )),
        temporary_bonus: None,
        spell_effect: Some(0x0104),      // S1/4 (Sorcerer L1 #4: Flame Arrow)
        max_charges: 30,
        is_cursed: false,
    ),

    // Complex magical item with both equip and use powers
    Item(
        id: 5,
        name: "Accurate Sword",
        item_type: Weapon(WeaponData(
            damage: DiceRoll(1, 8, 0),
            bonus: 6,                    // 8/6 in MM1 notation
            hands_required: 1,
        )),
        base_cost: 6500,
        sell_cost: 3250,
        disablements: Disablement(0b00101000), // KPA, Good only
        constant_bonus: Some(Bonus(
            attribute: Accuracy,
            value: 6,                    // Accuracy +6 when equipped
        )),
        temporary_bonus: Some(Bonus(
            attribute: Accuracy,
            value: 5,                    // Accuracy (Temp) +5 when used
        )),
        spell_effect: None,
        max_charges: 10,
        is_cursed: false,
    ),

    // Non-magical armor
    Item(
        id: 20,
        name: "Chain Mail",
        item_type: Armor(ArmorData(
            ac_bonus: 5,                 // 0/5 in MM1 notation
            weight: 40,
        )),
        base_cost: 200,
        sell_cost: 100,
        disablements: Disablement(0b00011100), // KPAC
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
    ),

    // Magical armor with resistance
    Item(
        id: 21,
        name: "Chain Mail +1",
        item_type: Armor(ArmorData(
            ac_bonus: 6,
            weight: 40,
        )),
        base_cost: 500,
        sell_cost: 250,
        disablements: Disablement(0b00011100),
        constant_bonus: Some(Bonus(
            attribute: ResistFire,
            value: 5,
        )),
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
    ),

    // Quest item (non-equippable)
    Item(
        id: 100,
        name: "Ruby Whistle",
        item_type: Quest(QuestData(
            quest_id: "brothers_quest",
            is_key_item: true,
        )),
        base_cost: 500,
        sell_cost: 250,
        disablements: Disablement(0x00),  // No one can equip
        constant_bonus: Some(Bonus(
            attribute: Luck,
            value: 2,                    // Carried bonus (not equipped)
        )),
        temporary_bonus: None,
        spell_effect: Some(0x0101),      // C1/1 (Cleric L1 #1: Awaken)
        max_charges: 200,
        is_cursed: false,
    ),

    // Cursed item
    Item(
        id: 101,
        name: "Mace of Undead",
        item_type: Weapon(WeaponData(
            damage: DiceRoll(1, 6, 0),
            bonus: 0,
            hands_required: 1,
        )),
        base_cost: 500,
        sell_cost: 250,
        disablements: Disablement(0b00011100), // KPAC, Good only
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: Some(0x0105),      // C1/5 (Light)
        max_charges: 10,
        is_cursed: true,                 // Cannot unequip!
    ),
]
```

**Item Notation Explained:**

- **Damage/Bonus** (e.g., "8/6"): Base damage 1-8, +6 to-hit and +6 damage
- **AC Bonus** (e.g., "0/9"): +9 to Armor Class
- **Spell Effect** (e.g., "C1/5" or "S3/4"): Cleric/Sorcerer spell, Level/Number
- **Equip Bonus**: Permanent bonus while equipped (constant_bonus)
- **Use Bonus**: Temporary bonus when used, consumes charges (temporary_bonus)
- **Charges**: Number of times magical effect can be used (0 = non-magical)

**Character Definition:**

```ron
// characters.ron - Premade characters and NPC templates
[
    (
        id: "pregen_human_knight",
        name: "Sir Galahad",
        race_id: "human",
        class_id: "knight",
        sex: Male,
        alignment: Good,
        base_stats: (
            might: 14,
            intellect: 8,
            personality: 10,
            endurance: 14,
            speed: 10,
            accuracy: 12,
            luck: 8,
        ),
        portrait_id: "1",
        starting_gold: 100,
        starting_gems: 0,
        starting_food: 10,
        starting_items: [1, 20],        // Club, Chain Mail
        starting_equipment: (
            weapon: Some(1),            // Club equipped
            armor: Some(20),            // Chain Mail equipped
            shield: None,
            helmet: None,
            boots: None,
            accessory1: None,
            accessory2: None,
        ),
        description: "A noble knight seeking glory and honor.",
        is_premade: true,
    ),
    (
        id: "pregen_elf_sorcerer",
        name: "Elindra",
        race_id: "elf",
        class_id: "sorcerer",
        sex: Female,
        alignment: Neutral,
        base_stats: (
            might: 8,
            intellect: 16,
            personality: 12,
            endurance: 8,
            speed: 12,
            accuracy: 10,
            luck: 10,
        ),
        portrait_id: "5",
        starting_gold: 50,
        starting_gems: 5,
        starting_food: 10,
        starting_items: [1],            // Club only
        starting_equipment: (
            weapon: Some(1),
            armor: None,
            shield: None,
            helmet: None,
            boots: None,
            accessory1: None,
            accessory2: None,
        ),
        description: "An elven mage with a talent for arcane arts.",
        is_premade: true,
    ),
    (
        id: "npc_template_guard",
        name: "Town Guard",
        race_id: "human",
        class_id: "knight",
        sex: Male,
        alignment: Good,
        base_stats: (
            might: 12,
            intellect: 8,
            personality: 8,
            endurance: 12,
            speed: 10,
            accuracy: 10,
            luck: 8,
        ),
        portrait_id: "10",
        starting_gold: 20,
        starting_gems: 0,
        starting_food: 5,
        starting_items: [],
        starting_equipment: (
            weapon: Some(2),            // Club +1
            armor: Some(20),            // Chain Mail
            shield: None,
            helmet: None,
            boots: None,
            accessory1: None,
            accessory2: None,
        ),
        description: "A loyal town guard.",
        is_premade: false,              // Template, not premade
    ),
]
```

**Character Definition Fields Explained:**

- **id**: Unique identifier for the character definition (string)
- **race_id/class_id**: References to races.ron and classes.ron entries
- **base_stats**: Starting stats before race modifiers are applied
- **starting_items**: Items added to inventory (by ItemId)
- **starting_equipment**: Items equipped in slots (by ItemId)
- **is_premade**: `true` for player-selectable premade characters, `false` for templates

**Instantiation Process:**

When a `CharacterDefinition` is instantiated:

1. Race stat modifiers are applied to `base_stats`
2. HP is calculated: max(class HP die) + (endurance - 10) / 2
3. SP is calculated based on class `spell_stat` (Intellect or Personality)
4. Race resistances are copied to the character
5. Starting items populate the inventory
6. Starting equipment is placed in equipment slots

---

#### 7.3 Test Coverage

The project includes comprehensive automated and manual testing to ensure data integrity and system correctness.

**Automated Integration Tests:**

Location: `tests/cli_editor_tests.rs` (20 tests, 959 lines)

**Test Categories:**

1. **Class Editor Round-Trip Tests (4 tests)**

   - Proficiency preservation across serialization/deserialization
   - Spellcasting class data integrity
   - Legacy disablement handling
   - Migration path from old to new format

2. **Item Editor Round-Trip Tests (10 tests)**

   - All 6 item types (Weapon, Armor, Accessory, Consumable, Ammo, Quest)
   - All classification enums (WeaponClassification, ArmorClassification, AccessorySlot)
   - All consumable effect variants (HealHp, RestoreSp, CureCondition, BoostAttribute)
   - Field name validation (base_cost, sell_cost, max_charges, is_cursed, tags)

3. **Race Editor Round-Trip Tests (3 tests)**

   - Stat modifiers (all 7 attributes)
   - Resistances (all 8 types)
   - Special abilities preservation
   - Proficiencies and incompatible_item_tags arrays

4. **Legacy Data Compatibility Tests (4 tests)**
   - Classes without proficiencies field
   - Races without proficiencies/incompatible_item_tags
   - Items without tags/classifications
   - Hybrid data with both old and new fields

**Round-Trip Test Pattern:**

All integration tests follow this pattern to ensure data integrity:

```rust
// 1. Create test data structure
let test_item = create_test_weapon();

// 2. Serialize to RON format
let ron_string = ron::ser::to_string_pretty(&test_item, Default::default())?;

// 3. Write to temporary file
std::fs::write(&temp_file, &ron_string)?;

// 4. Read from file
let read_string = std::fs::read_to_string(&temp_file)?;

// 5. Deserialize back
let deserialized: Item = ron::from_str(&read_string)?;

// 6. Assert all fields match original
assert_eq!(test_item, deserialized);
```

**Manual Test Checklist:**

Location: `docs/explanation/phase5_manual_test_checklist.md` (24 test scenarios, 886 lines)

**Test Suites:**

1. **Class Editor - Proficiency System (4 tests)**

   - Standard proficiencies
   - Custom proficiencies with warnings
   - Editing existing proficiencies
   - Empty proficiencies

2. **Race Editor - Proficiencies and Tags (4 tests)**

   - Proficiencies and incompatible tags
   - Custom tags with warnings
   - Editing tags
   - No restrictions

3. **Item Editor - Classifications and Tags (5 tests)**

   - Weapon with classification and tags
   - Armor with classification
   - Alignment restrictions (Good/Evil/Any)
   - Accessory with magic classification
   - Custom tags with warnings

4. **Legacy Data Compatibility (3 tests)**

   - Load legacy class files
   - Load legacy race files
   - Load legacy item files

5. **Integration Testing (2 tests)**

   - Full workflow across all editors
   - Cross-editor data validation

6. **Error Handling (2 tests)**
   - Invalid input handling
   - File I/O error handling

**Test Execution:**

```bash
# Run automated tests
cargo test --all-features

# Manual testing
cargo build --release --bin class_editor
cargo build --release --bin race_editor
cargo build --release --bin item_editor
# Follow manual test checklist procedures
```

**Test Results:**

- All 307 automated tests pass (287 existing + 20 new integration tests)
- Zero clippy warnings with `-D warnings` flag
- > 80% code coverage for editor data structures
- All quality gates passing (fmt, check, clippy, test)

---

### 8. Development Phases

#### Phase 1: Core Engine (Weeks 1-3)

- Basic game loop
- Character creation
- Stats and combat math
- Simple map rendering
- Movement system

#### Phase 2: Combat (Weeks 4-5)

- Turn-based combat engine
- Basic monster AI
- Combat UI
- XP and leveling

#### Phase 3: World (Weeks 6-8)

- Map loading system
- Multiple map types
- Events and triggers
- NPCs and dialogue

#### Phase 4: Systems (Weeks 9-11)

- Inventory and equipment
- Magic system
- Status effects
- Shops and trading

#### Phase 5: Content (Weeks 12-14)

- Create maps
- Design encounters
- Write quests
- Balance gameplay

#### Phase 6: Polish (Weeks 15-16)

- Save/load
- Audio
- UI improvements
- Bug fixes

---

### 9. Testing Strategy

- **Unit Tests**: Combat math, stat calculations, dice rolling, character
  creation
- **Integration Tests**: Save/load, map transitions, combat flow, rest/food
  system
- **Playtesting**: Balance, difficulty curve, fun factor, progression curve

---

### 10. Future Enhancements

- Multiplayer party management
- Modding support
- Procedural dungeon generation
- Advanced AI behaviors
- Voice acting
- Controller support

---

### 11. Additional Systems Details

#### 11.1 Combat Positioning System

Party members are arranged in a 2x3 grid:

```text
[0] [1]
[2] [3]
[4] [5]
```

Combat positioning affects:

- Who can attack in melee (front row: 0,1,2,3)
- Wall blocking (left wall blocks position 2, right wall blocks position 3)
- Ranged attacks available to all
- Monster targeting preferences
- Spell effectiveness

#### 11.2 Character Age System

Characters age over time:

- Age tracked in years + day counter
- Aging affects stats after certain thresholds
- Death from old age possible
- Age acceleration from certain events/spells

#### 11.3 Resource Sharing

Party can share resources:

- Gold can be pooled or individual
- Food is party-wide
- Gems can be pooled or individual
- Items must be explicitly traded

Trade system returns:

- Success
- SourceHasNoItem
- TargetInventoryFull

#### 11.4 Map Special Events

Each map implements special event handlers:

```rust
pub trait MapSpecials {
    fn check_special(&mut self, position: Position, facing: Direction) -> EventResult;
    fn trigger_event(&mut self, event_id: EventId) -> EventResult;
}
```

Events can:

- Modify party state
- Change conditions
- Grant items/gold
- Teleport party
- Start combat
- Show dialogue
- Set quest flags

### 12. Additional Game Systems

#### 12.1 Character Creation System

**Stat Generation:**

- Each of 7 stats randomly rolled 3-18
- Player can reroll until satisfied
- Class eligibility determined by stat values
- Prime statistics must meet minimum thresholds

**Creation Flow:**

1. Generate random stats (3-18 for each)
2. Show eligible classes based on stats
3. Player selects class
4. Player selects race (may modify stats)
5. Player selects alignment (no stat effect)
6. Player selects sex (no stat effect)
7. Player enters name (max 15 characters)
8. Character saved to roster

**Starting Values:**

- Age: 18 years
- Level: 1
- HP: Maximum for class (modified by endurance)
- SP: Based on prime statistic
- Gold: 0
- Gems: 0
- Food: 10 units
- Experience: 0

#### 12.2 Food System

**Food Mechanics:**

- Each character carries individual food supply
- Maximum 40 food units per character
- 1 food unit = 1 day's supply
- Rest requires 1 food unit per character
- Rest fails if character has no food
- Create Food spell adds 6 units to caster's supply
- Food can be shared/traded between party members

#### 12.3 Rest System

**Rest Effects:**

- Restores all HP to maximum (unless inhibited)
- Restores all SP to maximum (unless inhibited)
- Consumes 1 food unit per character
- All party-wide protection spells expire
- Advances time by ~8 hours
- Ages character by 1 day counter

**Rest Restrictions:**

- Cannot rest if too dangerous (map-dependent)
- Cannot rest without food
- May encounter monsters during rest (some wake asleep)
- Characters age 80+ may die from old age during rest

**Rest Locations:**

- Wilderness: possible but risky
- Dungeons: often too dangerous
- Towns/Inns: safest option

#### 12.4 Light System

**Light Mechanics:**

- Dark areas require light units to navigate
- 1 light unit consumed per dark square entered
- Light spell grants 1 light unit
- Lasting Light spell grants 20 light units
- Party tracks total light units available
- Without light, party must "feel" through darkness

**Light Display:**

- Protect command shows active light units
- Format: "Light (X)" where X = available units

#### 12.5 Training and Leveling

**Experience Requirements:**

- Level 1→2: ~2000 XP
- Each subsequent level approximately doubles
- Dead/stone/eradicated characters gain no XP

**Level Up Process:**

1. Accumulate required experience points
2. Visit training grounds in town
3. Pay training fee (gold cost increases with level)
4. Character advances one level
5. Gain HP (roll class-specific die, modified by endurance)
6. Possibly gain spell access (every other level)
7. Possibly gain special abilities (class-specific)

**HP Gain Per Level:**

```rust
pub enum HpGainDie {
    Knight => 1d12,      // 1-12 HP per level
    Paladin => 1d10,     // 1-10 HP per level
    Archer => 1d10,      // 1-10 HP per level
    Cleric => 1d8,       // 1-8 HP per level
    Robber => 1d8,       // 1-8 HP per level
    Sorcerer => 1d6,     // 1-6 HP per level
}
// Modified by Endurance attribute
```

**Spell Access:**

- Spell casters gain spells every other level
- Access to spell level = (character level + 1) / 2
- Level 1: Access to level 1 spells
- Level 3: Access to level 2 spells
- Level 5: Access to level 3 spells, etc.

**Class Spell Access:**

```rust
pub fn max_spell_level(character_level: u32, class: Class) -> u8 {
    let base_level = (character_level + 1) / 2;

    match class {
        Class::Cleric | Class::Sorcerer => {
            // Full casters get immediate access
            base_level.min(7) as u8
        }
        Class::Paladin | Class::Archer => {
            // Hybrid classes get delayed access
            if character_level < 3 {
                0 // No spells at levels 1-2
            } else {
                ((character_level - 2) / 2).min(7) as u8
            }
        }
        _ => 0, // Non-spellcasting classes
    }
}
```

**Special Abilities:**

- Knights: Multiple attacks per round at higher levels
- Paladins: Cleric spell access starting at level 3
- Archers: Sorcerer spell access starting at level 3
- Robbers: Improved lock picking and trap disarming
- Clerics: Full divine magic access from level 1
- Sorcerers: Full arcane magic access from level 1

#### 12.6 Item Charge System

**Charge Mechanics:**

- Many magical items have limited charges
- Detect Magic spell reveals remaining charges
- Items with 0 charges become useless
- Useless items cannot be recharged
- Recharge Item spell requires 1+ charges remaining

**Recharging:**

- Costs: 6 SP + 4 gems (Level 6 Sorcerer spell)
- Restores 1-4 charges to item
- Risk of spell failure (destroys item)
- Can only recharge items in caster's backpack

**Charge Consumption:**

- 1 charge consumed per spell cast from item
- Automatic depletion when item used
- No warning before last charge

#### 12.7 Inn and Save System

**Inn Mechanics:**

- Each town has an inn
- Party must sign in to save progress
- Sign in prompt when entering inn
- Answering "Yes" saves all character data to disk
- Characters saved with current stats/conditions

**Save Data:**

- All character statistics
- Current location (town/inn)
- Inventory and equipment
- Quest progress and flags
- Party composition

**Loading:**

- Resume from last inn where party signed in
- Can form different party from roster
- Each town tracks which characters last visited

#### 12.8 Resource Management Commands

**Gather Command:**

- Transfers all gold/gems/food from party to one character
- Limited by character's maximum capacity
- Gold: unlimited capacity
- Gems: unlimited capacity
- Food: 40 unit maximum

**Share Command:**

- Evenly distributes gold/gems/food among all party members
- Rounds down if not evenly divisible
- Remainder stays with original holder

**Trade Command:**

- Transfer specific amounts or items between characters
- Source must have item/amount
- Destination must have capacity
- Returns: Success, NoItem, InventoryFull

#### 12.9 Search Mechanics

**Search Command:**

- Searches current square for hidden items/treasure
- Always search after defeating monsters
- Can search any square anytime
- Don't need to search immediately after combat
- May find: gold, gems, items, secret passages

**Search Results:**

- Success: Item/treasure found
- Failure: "You find nothing"
- Some squares have guaranteed finds
- Most treasure is random

#### 12.10 Unlock and Bash Mechanics

**Unlock (Pick Lock):**

- Allows character to attempt lock picking
- Robbers have best success chance
- Success: Door unlocked, traps disarmed, party can advance
- Failure: Door remains locked, may trigger trap
- Can retry, but each failure increases trap chance

**Bash:**

- Attempt to break down door with force
- Success: Door destroyed, party moves through
- Failure: Door remains, party doesn't move
- Either outcome may trigger traps
- No class restrictions

**Trap System:**

- Locked doors may be trapped
- Traps trigger on: failed unlock, any bash attempt
- Trap effects: damage, status conditions, teleport
- Successful unlock disarms traps

#### 12.11 Item System Details

**Item Classification:**

MM1 features over 200 unique items ranging from simple non-magical equipment to
complex magical artifacts with multiple powers.

**1. Non-Magical Items**

Basic equipment with no special properties:

- Simple weapons (Club, Dagger, Mace)
- Basic armor (Padded Armor, Leather Armor, Chain Mail)
- Standard shields (Small Shield, Large Shield)
- No bonuses, no charges, no magical effects
- Lowest cost, available in shops
- Cannot be detected by Detect Magic spell

**2. Enhanced Items (+1, +2, +3)**

Improved versions of basic equipment:

- Better damage/AC bonus
- Still non-magical (no special powers)
- Example: Club +1 (3/1), Dagger +2 (4/2)
- Higher cost, sometimes found as loot

**3. Magical Items with Equip Bonuses**

Items that grant permanent bonuses while equipped:

- Stat bonuses (Accuracy +6, Might +5)
- Resistances (Fire +20%, Cold +40%)
- Bonuses apply continuously when equipped
- No charges required
- Example: Battle Axe +1 with Fire +20%
- Detected by Detect Magic spell

**4. Magical Items with Use Powers**

Items with activated abilities:

- Cast spells (S1/4, C3/6)
- Grant temporary stat boosts
- Require charges to use
- Charges deplete with each use
- Example: Flaming Club (30 charges, casts Flame Arrow)
- Items with 0 charges become useless

**5. Complex Magical Items**

Items with both equip bonuses AND use powers:

- Permanent bonus when equipped
- Additional temporary effect when used
- Require charges for activated effect
- Example: Accurate Sword
  - Equip: Accuracy +6 (permanent)
  - Use: Accuracy +5 temporary (10 charges)

**6. Quest Items**

Special items for quest progression:

- Cannot be equipped (No Equip flag)
- May have magical properties
- May grant bonuses just by carrying
- Example: Ruby Whistle (Luck +2, 200 charges)

**7. Cursed Items**

Dangerous magical items:

- Cannot be unequipped without Remove Curse spell
- May have negative effects
- Example: Bag of Garbage, Mace of Undead
- Detected by Detect Magic spell

**Charge Mechanics:**

```rust
// Charge management
pub struct ChargeState {
    pub max_charges: u8,      // Maximum charges item can hold
    pub current_charges: u8,  // Charges remaining
}

impl ChargeState {
    pub fn use_charge(&mut self) -> bool {
        if self.current_charges > 0 {
            self.current_charges -= 1;
            true
        } else {
            false // Item is useless
        }
    }

    pub fn is_useless(&self) -> bool {
        self.max_charges > 0 && self.current_charges == 0
    }

    pub fn can_recharge(&self) -> bool {
        self.max_charges > 0 && self.current_charges > 0
    }
}
```

**Charge Rules:**

- Items with 0 current charges cannot be used
- Items with 0 charges cannot be recharged
- Recharge Item spell requires 1+ charges remaining
- Recharge restores 1-4 charges (risk of destroying item)
- Detect Magic spell reveals remaining charges

**Item Bonuses:**

**Permanent (Equip) Bonuses:**

- Applied when item is equipped
- Removed when item is unequipped
- Stored in `constant_bonus` field
- Examples: Accuracy +6, Fire +20%, Might +5

**Temporary (Use) Bonuses:**

- Applied when item is used
- Last for limited duration (combat or time-based)
- Consume one charge per use
- Stored in `temporary_bonus` field
- Examples: Might (Temp) +5, Speed (Temp) +5

**Spell Effects:**

- Items that cast spells when used
- Stored in `spell_effect` field
- Format: School + Level + Number (e.g., C1/5, S3/4)
- Consume charges like temporary bonuses

**Item Notation (MM1 Style):**

- **Weapon Damage**: "8/6" = 1-8 base damage, +6 to-hit and +6 damage
- **Armor AC**: "0/9" = +9 to Armor Class
- **Spell Reference**: "C1/5" = Cleric Level 1, Spell #5
- **Spell Reference**: "S3/4" = Sorcerer Level 3, Spell #4
- **Charges**: Number of uses remaining (shown by Detect Magic)

**Detection and Identification:**

- Detect Magic (S1/2): Reveals if item is magical
- Shows remaining charges for charged items
- Does not reveal specific bonuses or effects
- Essential for charge management

### 13. References

- Might and Magic 1 gameplay mechanics
- Might and Magic 1 Game Manual (1987)
- Might and Magic 1 Clue Book
- ScummVM MM1 engine implementation
- Classic CRPG design patterns
- Rust game development best practices
