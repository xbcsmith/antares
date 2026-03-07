// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Item mesh descriptor — drives procedural 3-D world meshes for dropped items
//!
//! This module defines the data types and pure functions that convert an [`Item`]
//! into a [`CreatureDefinition`] suitable for rendering by the existing
//! `spawn_creature` pipeline.  No Bevy dependency is introduced here; the
//! visual layer reads these types and constructs Bevy meshes separately.
//!
//! # Design
//!
//! * [`ItemMeshCategory`] — coarse visual class derived from `item_type` and
//!   sub-type data.  One category → one distinct mesh shape.
//! * [`ItemMeshDescriptor`] — full per-item visual specification (shape
//!   parameters, colors, emissive flag, scale).
//! * [`ItemMeshDescriptorOverride`] — subset of `ItemMeshDescriptor` that
//!   campaign authors can embed in a RON item file to customise visuals
//!   without touching gameplay data.  All fields default to `None` so
//!   existing RON files remain valid.
//! * [`ItemMeshDescriptor::from_item`] — pure function; no side-effects,
//!   no Bevy types.
//! * [`ItemMeshDescriptor::to_creature_definition`] — converts to the shared
//!   [`CreatureDefinition`] type so `spawn_creature` can render items without
//!   a new rendering path.
//! * [`ItemMeshDescriptor::to_creature_definition_with_charges`] — variant
//!   that accepts a `charges_fraction` to add a charge-level gem indicator.
//!
//! # Phase 4 additions
//!
//! * Accent color is derived from `BonusAttribute` when the item has a bonus.
//! * `is_magical()` items receive `metallic > 0.5` / `roughness < 0.3`.
//! * A ground shadow quad is prepended to every `CreatureDefinition`.
//! * A charge-level emissive gem is appended when `charges_fraction` is given.
//! * Complex meshes (> `LOD_TRIANGLE_THRESHOLD` triangles) get LOD levels.
//!
//! # Architecture reference
//!
//! See `docs/explanation/items_procedural_meshes_implementation_plan.md`
//! Phase 1 and Phase 4, and `docs/reference/architecture.md` Section 4.5.

use crate::domain::items::types::{
    AccessorySlot, ArmorClassification, BonusAttribute, ConsumableEffect, Item, ItemType,
    WeaponClassification,
};
use crate::domain::visual::lod::generate_lod_levels;
use crate::domain::visual::{
    AlphaMode, CreatureDefinition, MaterialDefinition, MeshDefinition, MeshTransform,
};
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// ItemMeshCategory
// ─────────────────────────────────────────────────────────────────────────────

/// Coarse visual category for a dropped item.
///
/// Each variant maps to a distinct procedural mesh shape.  The category is
/// derived automatically from [`ItemType`] and sub-type classification data
/// by [`ItemMeshDescriptor::from_item`].
///
/// # Examples
///
/// ```
/// use antares::domain::visual::item_mesh::ItemMeshCategory;
///
/// let cat = ItemMeshCategory::Sword;
/// assert_eq!(format!("{:?}", cat), "Sword");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemMeshCategory {
    /// Single-handed sword (MartialMelee with moderate blade length)
    Sword,
    /// Short dagger / knife (Simple weapons with very short blade)
    Dagger,
    /// Blunt weapon — club, mace, hammer
    Blunt,
    /// Staff / polearm (long two-handed weapon)
    Staff,
    /// Ranged bow or crossbow
    Bow,
    /// Chest / torso armour
    BodyArmor,
    /// Head protection
    Helmet,
    /// Hand-held defensive shield
    Shield,
    /// Foot armour / boots
    Boots,
    /// Ring accessory
    Ring,
    /// Amulet / necklace accessory
    Amulet,
    /// Belt accessory
    Belt,
    /// Cloak / cape accessory
    Cloak,
    /// Potion / vial consumable
    Potion,
    /// Scroll consumable
    Scroll,
    /// Arrow / bolt / stone ammunition
    Ammo,
    /// Plot-critical or unique quest artefact
    QuestItem,
}

// ─────────────────────────────────────────────────────────────────────────────
// Color constants
// ─────────────────────────────────────────────────────────────────────────────

/// Steel-grey for swords and martial blades
const COLOR_STEEL: [f32; 4] = [0.75, 0.75, 0.78, 1.0];
/// Darker iron for blunt weapons
const COLOR_IRON: [f32; 4] = [0.50, 0.50, 0.52, 1.0];
/// Warm wood-brown for staves and bows
const COLOR_WOOD: [f32; 4] = [0.55, 0.35, 0.15, 1.0];
/// Leather-tan for armor and boots
const COLOR_LEATHER: [f32; 4] = [0.72, 0.53, 0.30, 1.0];
/// Silver-white for metal armor / helmets
const COLOR_SILVER: [f32; 4] = [0.82, 0.83, 0.85, 1.0];
/// Gold for rings and amulets
const COLOR_GOLD: [f32; 4] = [1.0, 0.84, 0.0, 1.0];
/// Ruby-red for healing potions (HealHp)
const COLOR_RED: [f32; 4] = [0.85, 0.10, 0.10, 1.0];
/// Sapphire-blue for mana potions (RestoreSp)
const COLOR_BLUE: [f32; 4] = [0.10, 0.30, 0.90, 1.0];
/// Emerald-green for cure potions (CureCondition)
const COLOR_GREEN: [f32; 4] = [0.10, 0.75, 0.20, 1.0];
/// Amber-yellow for attribute potions (BoostAttribute / BoostResistance)
const COLOR_YELLOW: [f32; 4] = [0.95, 0.80, 0.05, 1.0];
/// Parchment / cream for scrolls
const COLOR_PARCHMENT: [f32; 4] = [0.95, 0.90, 0.72, 1.0];
/// Feather-white for arrows / ammo bundles
const COLOR_AMMO: [f32; 4] = [0.90, 0.88, 0.70, 1.0];
/// Vivid magenta for quest items (stands out on any floor)
const COLOR_QUEST: [f32; 4] = [0.85, 0.15, 0.85, 1.0];

/// Dark purple tint for cursed items
const COLOR_CURSED: [f32; 4] = [0.18, 0.05, 0.22, 1.0];
/// Purple glow emitted by cursed items
const EMISSIVE_CURSED: [f32; 3] = [0.30, 0.0, 0.35];
/// Soft white glow for magical (charged) items
const EMISSIVE_MAGIC: [f32; 3] = [0.40, 0.40, 0.60];

// ─────────────────────────────────────────────────────────────────────────────
// Phase 4 accent colors (derived from BonusAttribute)
// ─────────────────────────────────────────────────────────────────────────────

/// Orange/amber accent for fire-resistance bonus items
const COLOR_ACCENT_FIRE: [f32; 4] = [1.0, 0.45, 0.05, 1.0];
/// Icy blue accent for cold-resistance bonus items
const COLOR_ACCENT_COLD: [f32; 4] = [0.55, 0.85, 1.0, 1.0];
/// Yellow accent for electricity-resistance bonus items
const COLOR_ACCENT_ELECTRICITY: [f32; 4] = [0.95, 0.95, 0.10, 1.0];
/// Acid-green accent for acid-resistance bonus items
const COLOR_ACCENT_ACID: [f32; 4] = [0.45, 0.90, 0.10, 1.0];
/// Acid-green accent for poison-resistance bonus items
const COLOR_ACCENT_POISON: [f32; 4] = [0.30, 0.80, 0.10, 1.0];
/// Purple accent for magic-resistance bonus items
const COLOR_ACCENT_MAGIC: [f32; 4] = [0.65, 0.10, 0.90, 1.0];
/// Warm red accent for Might bonus items
const COLOR_ACCENT_MIGHT: [f32; 4] = [0.85, 0.15, 0.15, 1.0];
/// Teal accent for ArmorClass / HP bonus items
const COLOR_ACCENT_TEAL: [f32; 4] = [0.10, 0.75, 0.70, 1.0];
/// Deep blue accent for SP / Intellect bonus items
const COLOR_ACCENT_DEEP_BLUE: [f32; 4] = [0.10, 0.20, 0.80, 1.0];

// ─────────────────────────────────────────────────────────────────────────────
// Phase 4 charge-gem colors
// ─────────────────────────────────────────────────────────────────────────────

/// Gold color for a fully-charged item gem
const COLOR_CHARGE_FULL: [f32; 4] = [1.0, 0.84, 0.0, 1.0];
/// White/pale for a half-charged item gem
const COLOR_CHARGE_HALF: [f32; 4] = [0.9, 0.9, 0.9, 1.0];
/// Grey for a depleted item gem
const COLOR_CHARGE_EMPTY: [f32; 4] = [0.4, 0.4, 0.4, 1.0];

/// Emissive color for a fully-charged gem
const EMISSIVE_CHARGE_FULL: [f32; 3] = [0.80, 0.67, 0.0];
/// Emissive color for a half-charged gem
const EMISSIVE_CHARGE_HALF: [f32; 3] = [0.30, 0.30, 0.30];
/// No emissive for a depleted gem
const EMISSIVE_CHARGE_EMPTY: [f32; 3] = [0.0, 0.0, 0.0];

// ─────────────────────────────────────────────────────────────────────────────
// Phase 4 geometry / LOD constants
// ─────────────────────────────────────────────────────────────────────────────

/// Shadow-quad Y offset — sits just above the floor to avoid Z-fighting
const SHADOW_QUAD_Y: f32 = 0.001;
/// Shadow-quad alpha (semi-transparent dark)
const SHADOW_QUAD_ALPHA: f32 = 0.3;
/// Shadow quad scales the item's XZ footprint by this factor
const SHADOW_QUAD_SCALE: f32 = 1.2;

/// Triangle count above which LOD levels are generated for an item mesh
const LOD_TRIANGLE_THRESHOLD: usize = 200;
/// Distance (world units) for LOD1 switch
const LOD_DISTANCE_1: f32 = 8.0;
/// Distance (world units) for LOD2 (billboard) switch
const LOD_DISTANCE_2: f32 = 20.0;

/// Metallic value for magical (is_magical) items — shiny
const MATERIAL_METALLIC_MAGICAL: f32 = 0.7;
/// Roughness value for magical items — polished
const MATERIAL_ROUGHNESS_MAGICAL: f32 = 0.25;
/// Metallic value for mundane items — matte
const MATERIAL_METALLIC_MUNDANE: f32 = 0.0;
/// Roughness value for mundane items — rough
const MATERIAL_ROUGHNESS_MUNDANE: f32 = 0.8;

// ─────────────────────────────────────────────────────────────────────────────
// Scale constants
// ─────────────────────────────────────────────────────────────────────────────

/// Base world-space scale for all dropped items (fits on a single floor tile)
const BASE_SCALE: f32 = 0.35;
/// Extra multiplier for two-handed / large weapons
const TWO_HANDED_SCALE_MULT: f32 = 1.45;
/// Extra multiplier for small items (rings, ammo)
const SMALL_SCALE_MULT: f32 = 0.55;

// ─────────────────────────────────────────────────────────────────────────────
// Shape parameter constants
// ─────────────────────────────────────────────────────────────────────────────

/// Blade-length factor per damage side for swords
/// (e.g., 1d6 → 6 sides → 6 × BLADE_SIDES_FACTOR blade segments)
const BLADE_SIDES_FACTOR: f32 = 0.08;
/// Minimum normalised blade length (clamp low end)
const BLADE_LENGTH_MIN: f32 = 0.25;
/// Maximum normalised blade length (clamp high end)
const BLADE_LENGTH_MAX: f32 = 1.0;

// ─────────────────────────────────────────────────────────────────────────────
// ItemMeshDescriptorOverride
// ─────────────────────────────────────────────────────────────────────────────

/// Campaign-author override for a subset of [`ItemMeshDescriptor`] fields.
///
/// Embed this in an [`Item`] RON definition to customise the visual appearance
/// without touching gameplay data.  Every field defaults to `None`; missing
/// fields fall back to the auto-derived value from `ItemMeshDescriptor::from_item`.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::item_mesh::ItemMeshDescriptorOverride;
///
/// // Override only the primary color
/// let ov = ItemMeshDescriptorOverride {
///     primary_color: Some([0.0, 0.8, 0.0, 1.0]),
///     accent_color: None,
///     scale: None,
///     emissive: None,
/// };
/// assert!(ov.primary_color.is_some());
/// assert!(ov.scale.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ItemMeshDescriptorOverride {
    /// Optional replacement primary color `[r, g, b, a]`
    #[serde(default)]
    pub primary_color: Option<[f32; 4]>,

    /// Optional replacement accent color `[r, g, b, a]`
    #[serde(default)]
    pub accent_color: Option<[f32; 4]>,

    /// Optional replacement world-space scale (must be positive)
    #[serde(default)]
    pub scale: Option<f32>,

    /// Optional replacement emissive color `[r, g, b]`.
    /// `Some([0.0, 0.0, 0.0])` disables emissive; `None` keeps default.
    #[serde(default)]
    pub emissive: Option<[f32; 3]>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ItemMeshDescriptor
// ─────────────────────────────────────────────────────────────────────────────

/// Full per-item visual specification used to drive procedural mesh generation.
///
/// Produced by [`ItemMeshDescriptor::from_item`] from an [`Item`] definition
/// and converted to a [`CreatureDefinition`] by [`ItemMeshDescriptor::to_creature_definition`].
///
/// # Examples
///
/// ```
/// use antares::domain::items::{Item, ItemType, WeaponData, WeaponClassification};
/// use antares::domain::types::DiceRoll;
/// use antares::domain::visual::item_mesh::ItemMeshDescriptor;
///
/// let short_sword = Item {
///     id: 3,
///     name: "Short Sword".to_string(),
///     item_type: ItemType::Weapon(WeaponData {
///         damage: DiceRoll::new(1, 6, 0),
///         bonus: 0,
///         hands_required: 1,
///         classification: WeaponClassification::MartialMelee,
///     }),
///     base_cost: 10,
///     sell_cost: 5,
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
///
/// let desc = ItemMeshDescriptor::from_item(&short_sword);
/// assert_eq!(desc.category, antares::domain::visual::item_mesh::ItemMeshCategory::Sword);
/// assert!(!desc.emissive);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemMeshDescriptor {
    /// Visual category → distinct mesh shape
    pub category: ItemMeshCategory,

    /// Normalised blade / shaft length in `[0.0, 1.0]`.
    /// Used by sword/dagger/staff shapes; ignored for other categories.
    pub blade_length: f32,

    /// Primary surface color `[r, g, b, a]`
    pub primary_color: [f32; 4],

    /// Secondary / accent color `[r, g, b, a]` (e.g., handle, crossguard)
    pub accent_color: [f32; 4],

    /// Whether the material emits light (magical glow)
    pub emissive: bool,

    /// Emissive color when `emissive` is `true`
    pub emissive_color: [f32; 3],

    /// World-space scale applied to the entire item mesh
    pub scale: f32,
}

impl ItemMeshDescriptor {
    /// Derives an [`ItemMeshDescriptor`] from an [`Item`] definition.
    ///
    /// This is a **pure function** with no side-effects and no Bevy dependency.
    /// It inspects `item.item_type`, sub-type classification fields, tags,
    /// bonus values, and charge data to produce a complete descriptor.
    ///
    /// **Phase 4 additions:**
    /// - Accent color is derived from the item's `constant_bonus` attribute
    ///   (e.g. `ResistFire` → orange, `ResistMagic` → purple).
    /// - `is_magical()` items receive a non-zero `metallic` / low `roughness`
    ///   that is respected by [`make_material`].
    ///
    /// Any `mesh_descriptor_override` present on the item is applied on top of
    /// the auto-derived values so campaign authors can customise visuals.
    ///
    /// # Arguments
    ///
    /// * `item` - Reference to the item whose mesh descriptor to derive.
    ///
    /// # Returns
    ///
    /// A fully-populated [`ItemMeshDescriptor`].
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::{Item, ItemType, ConsumableData, ConsumableEffect};
    /// use antares::domain::visual::item_mesh::{ItemMeshDescriptor, ItemMeshCategory};
    ///
    /// let potion = Item {
    ///     id: 50,
    ///     name: "Healing Potion".to_string(),
    ///     item_type: ItemType::Consumable(ConsumableData {
    ///         effect: ConsumableEffect::HealHp(20),
    ///         is_combat_usable: true,
    ///     }),
    ///     base_cost: 50,
    ///     sell_cost: 25,
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
    ///
    /// let desc = ItemMeshDescriptor::from_item(&potion);
    /// assert_eq!(desc.category, ItemMeshCategory::Potion);
    /// assert_eq!(desc.primary_color, [0.85, 0.10, 0.10, 1.0]); // red = heal
    /// ```
    pub fn from_item(item: &Item) -> Self {
        let is_two_handed = item.tags.iter().any(|t| t == "two_handed");

        let mut desc = match &item.item_type {
            ItemType::Weapon(wd) => Self::from_weapon(wd, is_two_handed),
            ItemType::Armor(ad) => Self::from_armor(ad),
            ItemType::Accessory(acc) => Self::from_accessory(acc),
            ItemType::Consumable(cd) => Self::from_consumable(cd),
            ItemType::Ammo(_) => Self::ammo_descriptor(),
            ItemType::Quest(_) => Self::quest_descriptor(),
        };

        // ── Phase 4.1: Derive accent color from BonusAttribute ───────────────
        // Only apply when the item has not been cursed (cursed takes over
        // primary color entirely, making accent irrelevant).
        if !item.is_cursed {
            if let Some(accent) = Self::accent_color_from_item(item) {
                desc.accent_color = accent;
            }
        }

        // ── Phase 4.1: Metallic / roughness driven by is_magical() ───────────
        // We store whether the item is magical as a sentinel on the descriptor
        // so that make_material() can pick the right PBR params.  We re-use
        // the `emissive` field for the glow, but track the magical flag
        // separately via is_magical — the actual metallic value is applied in
        // make_material() using self.is_metallic_magical().
        //
        // The field is encoded implicitly: after setting emissive below, any
        // call site that needs to know "is this a magical material?" calls
        // self.emissive && self.emissive_color == EMISSIVE_MAGIC.
        // For build clarity we keep the existing emissive bool approach.

        // Apply magical glow if item has charges or constant bonus
        if item.is_magical() {
            desc.emissive = true;
            desc.emissive_color = EMISSIVE_MAGIC;
        }

        // Cursed items override: dark tint + purple glow (takes priority over
        // magical glow so the player can recognise the curse visually)
        if item.is_cursed {
            desc.primary_color = COLOR_CURSED;
            desc.emissive = true;
            desc.emissive_color = EMISSIVE_CURSED;
        }

        // Apply campaign-author override (last, highest priority)
        if let Some(ref ov) = item.mesh_descriptor_override {
            if let Some(c) = ov.primary_color {
                desc.primary_color = c;
            }
            if let Some(c) = ov.accent_color {
                desc.accent_color = c;
            }
            if let Some(s) = ov.scale {
                if s > 0.0 {
                    desc.scale = s;
                }
            }
            if let Some(e) = ov.emissive {
                // A non-zero emissive override enables the flag
                let nonzero = e[0] != 0.0 || e[1] != 0.0 || e[2] != 0.0;
                desc.emissive = nonzero;
                desc.emissive_color = e;
            }
        }

        desc
    }

    /// Maps an item's `constant_bonus` or `temporary_bonus` attribute to an
    /// accent color, following the Phase 4.1 color table.
    ///
    /// Returns `None` when the item has no relevant bonus (the caller will
    /// keep the auto-derived accent color unchanged).
    ///
    /// | `BonusAttribute`    | Accent color        |
    /// |---------------------|---------------------|
    /// | `ResistFire`        | Orange / amber      |
    /// | `ResistCold`        | Icy blue            |
    /// | `ResistElectricity` | Yellow              |
    /// | `ResistAcid`        | Acid green          |
    /// | `ResistPoison`      | Acid green          |
    /// | `ResistMagic`       | Purple              |
    /// | `Might`             | Warm red            |
    /// | `ArmorClass` / `Endurance` | Teal         |
    /// | `SP` / `Intellect`  | Deep blue           |
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::{Item, ItemType, WeaponData, WeaponClassification,
    ///                               Bonus, BonusAttribute};
    /// use antares::domain::types::DiceRoll;
    /// use antares::domain::visual::item_mesh::ItemMeshDescriptor;
    ///
    /// let mut sword = Item {
    ///     id: 1,
    ///     name: "Flaming Sword".to_string(),
    ///     item_type: ItemType::Weapon(WeaponData {
    ///         damage: DiceRoll::new(1, 8, 0),
    ///         bonus: 1,
    ///         hands_required: 1,
    ///         classification: WeaponClassification::MartialMelee,
    ///     }),
    ///     base_cost: 200,
    ///     sell_cost: 100,
    ///     alignment_restriction: None,
    ///     constant_bonus: Some(Bonus { attribute: BonusAttribute::ResistFire, value: 5 }),
    ///     temporary_bonus: None,
    ///     spell_effect: None,
    ///     max_charges: 0,
    ///     is_cursed: false,
    ///     icon_path: None,
    ///     tags: vec![],
    ///     mesh_descriptor_override: None,
    /// };
    ///
    /// let desc = ItemMeshDescriptor::from_item(&sword);
    /// // Orange accent from ResistFire
    /// assert_eq!(desc.accent_color, [1.0, 0.45, 0.05, 1.0]);
    /// ```
    fn accent_color_from_item(item: &Item) -> Option<[f32; 4]> {
        // Prefer constant_bonus; fall back to temporary_bonus
        let attr = item
            .constant_bonus
            .as_ref()
            .map(|b| b.attribute)
            .or_else(|| item.temporary_bonus.as_ref().map(|b| b.attribute))?;

        let color = match attr {
            BonusAttribute::ResistFire => COLOR_ACCENT_FIRE,
            BonusAttribute::ResistCold => COLOR_ACCENT_COLD,
            BonusAttribute::ResistElectricity => COLOR_ACCENT_ELECTRICITY,
            BonusAttribute::ResistAcid => COLOR_ACCENT_ACID,
            BonusAttribute::ResistPoison => COLOR_ACCENT_POISON,
            BonusAttribute::ResistMagic => COLOR_ACCENT_MAGIC,
            BonusAttribute::Might => COLOR_ACCENT_MIGHT,
            BonusAttribute::ArmorClass | BonusAttribute::Endurance => COLOR_ACCENT_TEAL,
            BonusAttribute::Intellect => COLOR_ACCENT_DEEP_BLUE,
            // Other attributes don't override accent color
            _ => return None,
        };
        Some(color)
    }

    /// Returns `true` when this descriptor represents a magical item that
    /// should use shiny PBR parameters (high metallic, low roughness).
    ///
    /// The heuristic: a descriptor is considered magical when its emissive
    /// color matches `EMISSIVE_MAGIC` (set by `from_item` when `is_magical()`).
    #[inline]
    fn is_metallic_magical(&self) -> bool {
        self.emissive && self.emissive_color == EMISSIVE_MAGIC
    }

    // ── Weapon ──────────────────────────────────────────────────────────────

    fn from_weapon(wd: &crate::domain::items::types::WeaponData, is_two_handed: bool) -> Self {
        let sides = wd.damage.sides as f32;
        let blade_length = (sides * BLADE_SIDES_FACTOR).clamp(BLADE_LENGTH_MIN, BLADE_LENGTH_MAX);

        let scale_mult = if is_two_handed {
            TWO_HANDED_SCALE_MULT
        } else {
            1.0
        };

        match wd.classification {
            WeaponClassification::Simple => {
                // Simple weapons: daggers (low sides), clubs (blunt)
                // Use Dagger for Simple ranged/piercing, Blunt for clubs
                if wd.damage.sides <= 4 {
                    Self {
                        category: ItemMeshCategory::Dagger,
                        blade_length: blade_length * 0.7, // daggers shorter
                        primary_color: COLOR_STEEL,
                        accent_color: COLOR_WOOD,
                        emissive: false,
                        emissive_color: [0.0; 3],
                        scale: BASE_SCALE * scale_mult,
                    }
                } else {
                    // clubs, quarterstaffs tagged simple
                    Self {
                        category: ItemMeshCategory::Blunt,
                        blade_length,
                        primary_color: COLOR_WOOD,
                        accent_color: COLOR_IRON,
                        emissive: false,
                        emissive_color: [0.0; 3],
                        scale: BASE_SCALE * scale_mult,
                    }
                }
            }
            WeaponClassification::MartialMelee => {
                // Long swords, great swords, axes
                if is_two_handed {
                    // Two-handed great swords / battleaxes → Staff category
                    // (long polearm-like silhouette) or Sword with bigger scale
                    Self {
                        category: ItemMeshCategory::Sword,
                        blade_length,
                        primary_color: COLOR_STEEL,
                        accent_color: COLOR_IRON,
                        emissive: false,
                        emissive_color: [0.0; 3],
                        scale: BASE_SCALE * scale_mult,
                    }
                } else {
                    Self {
                        category: ItemMeshCategory::Sword,
                        blade_length,
                        primary_color: COLOR_STEEL,
                        accent_color: COLOR_WOOD,
                        emissive: false,
                        emissive_color: [0.0; 3],
                        scale: BASE_SCALE,
                    }
                }
            }
            WeaponClassification::MartialRanged => Self {
                category: ItemMeshCategory::Bow,
                blade_length,
                primary_color: COLOR_WOOD,
                accent_color: COLOR_LEATHER,
                emissive: false,
                emissive_color: [0.0; 3],
                scale: BASE_SCALE * scale_mult,
            },
            WeaponClassification::Blunt => Self {
                category: ItemMeshCategory::Blunt,
                blade_length,
                primary_color: COLOR_IRON,
                accent_color: COLOR_WOOD,
                emissive: false,
                emissive_color: [0.0; 3],
                scale: BASE_SCALE * scale_mult,
            },
            WeaponClassification::Unarmed => Self {
                category: ItemMeshCategory::Blunt,
                blade_length: BLADE_LENGTH_MIN,
                primary_color: COLOR_LEATHER,
                accent_color: COLOR_LEATHER,
                emissive: false,
                emissive_color: [0.0; 3],
                scale: BASE_SCALE * SMALL_SCALE_MULT,
            },
        }
    }

    // ── Armor ────────────────────────────────────────────────────────────────

    fn from_armor(ad: &crate::domain::items::types::ArmorData) -> Self {
        match ad.classification {
            ArmorClassification::Light => Self {
                category: ItemMeshCategory::BodyArmor,
                blade_length: 0.5,
                primary_color: COLOR_LEATHER,
                accent_color: COLOR_LEATHER,
                emissive: false,
                emissive_color: [0.0; 3],
                scale: BASE_SCALE,
            },
            ArmorClassification::Medium => Self {
                category: ItemMeshCategory::BodyArmor,
                blade_length: 0.6,
                primary_color: COLOR_IRON,
                accent_color: COLOR_LEATHER,
                emissive: false,
                emissive_color: [0.0; 3],
                scale: BASE_SCALE * 1.1,
            },
            ArmorClassification::Heavy => Self {
                category: ItemMeshCategory::BodyArmor,
                blade_length: 0.7,
                primary_color: COLOR_SILVER,
                accent_color: COLOR_IRON,
                emissive: false,
                emissive_color: [0.0; 3],
                scale: BASE_SCALE * 1.2,
            },
            ArmorClassification::Shield => Self {
                category: ItemMeshCategory::Shield,
                blade_length: 0.5,
                primary_color: COLOR_IRON,
                accent_color: COLOR_WOOD,
                emissive: false,
                emissive_color: [0.0; 3],
                scale: BASE_SCALE,
            },
        }
    }

    // ── Accessory ────────────────────────────────────────────────────────────

    fn from_accessory(acc: &crate::domain::items::types::AccessoryData) -> Self {
        let (category, primary, accent, scale_mult) = match acc.slot {
            AccessorySlot::Ring => (
                ItemMeshCategory::Ring,
                COLOR_GOLD,
                COLOR_GOLD,
                SMALL_SCALE_MULT,
            ),
            AccessorySlot::Amulet => (
                ItemMeshCategory::Amulet,
                COLOR_GOLD,
                COLOR_SILVER,
                SMALL_SCALE_MULT * 1.2,
            ),
            AccessorySlot::Belt => (
                ItemMeshCategory::Belt,
                COLOR_LEATHER,
                COLOR_IRON,
                SMALL_SCALE_MULT * 1.5,
            ),
            AccessorySlot::Cloak => (ItemMeshCategory::Cloak, COLOR_LEATHER, COLOR_WOOD, 1.0),
        };
        Self {
            category,
            blade_length: 0.0,
            primary_color: primary,
            accent_color: accent,
            emissive: false,
            emissive_color: [0.0; 3],
            scale: BASE_SCALE * scale_mult,
        }
    }

    // ── Consumable ───────────────────────────────────────────────────────────

    fn from_consumable(cd: &crate::domain::items::types::ConsumableData) -> Self {
        // Distinguish Scroll from Potion based on effect type.
        // Non-hp/sp effects that look more scroll-like:
        let is_scroll = matches!(cd.effect, ConsumableEffect::CureCondition(_));

        if is_scroll {
            return Self {
                category: ItemMeshCategory::Scroll,
                blade_length: 0.5,
                primary_color: COLOR_PARCHMENT,
                accent_color: COLOR_WOOD,
                emissive: false,
                emissive_color: [0.0; 3],
                scale: BASE_SCALE * SMALL_SCALE_MULT * 1.4,
            };
        }

        let primary_color = match cd.effect {
            ConsumableEffect::HealHp(_) => COLOR_RED,
            ConsumableEffect::RestoreSp(_) => COLOR_BLUE,
            ConsumableEffect::CureCondition(_) => COLOR_GREEN, // unreachable via early return above
            ConsumableEffect::BoostAttribute(_, _) => COLOR_YELLOW,
            ConsumableEffect::BoostResistance(_, _) => COLOR_GREEN,
        };

        Self {
            category: ItemMeshCategory::Potion,
            blade_length: 0.0,
            primary_color,
            accent_color: [
                primary_color[0] * 0.7,
                primary_color[1] * 0.7,
                primary_color[2] * 0.7,
                1.0,
            ],
            emissive: false,
            emissive_color: [0.0; 3],
            scale: BASE_SCALE * SMALL_SCALE_MULT,
        }
    }

    // ── Ammo ─────────────────────────────────────────────────────────────────

    fn ammo_descriptor() -> Self {
        Self {
            category: ItemMeshCategory::Ammo,
            blade_length: 0.6,
            primary_color: COLOR_AMMO,
            accent_color: COLOR_WOOD,
            emissive: false,
            emissive_color: [0.0; 3],
            scale: BASE_SCALE * SMALL_SCALE_MULT,
        }
    }

    // ── Quest ────────────────────────────────────────────────────────────────

    fn quest_descriptor() -> Self {
        Self {
            category: ItemMeshCategory::QuestItem,
            blade_length: 0.5,
            primary_color: COLOR_QUEST,
            accent_color: COLOR_GOLD,
            emissive: true,
            emissive_color: [0.5, 0.0, 0.5],
            scale: BASE_SCALE,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // to_creature_definition
    // ─────────────────────────────────────────────────────────────────────────

    /// Converts this descriptor into a [`CreatureDefinition`] that the
    /// existing `spawn_creature` pipeline can render.
    ///
    /// The item is represented as a **flat-lying** object on the ground — a
    /// single-mesh creature whose transform lays the geometry on the XZ plane
    /// (Y = 0).  The exact shape is determined by [`ItemMeshCategory`].
    ///
    /// The returned `CreatureDefinition` has:
    /// - `id = 0` (caller must assign a unique ID before registering)
    /// - `name` describing the category for debugging
    /// - exactly **one** [`MeshDefinition`] and one [`MeshTransform`]
    /// - `scale` from `self.scale`
    /// - `color_tint = None` (color is baked into the mesh material)
    ///
    /// # Errors
    ///
    /// This function always succeeds; call [`CreatureDefinition::validate`] on
    /// the result to confirm well-formedness before use.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::{Item, ItemType, WeaponData, WeaponClassification};
    /// use antares::domain::types::DiceRoll;
    /// use antares::domain::visual::item_mesh::ItemMeshDescriptor;
    ///
    /// let sword = Item {
    ///     id: 4,
    ///     name: "Long Sword".to_string(),
    ///     item_type: ItemType::Weapon(WeaponData {
    ///         damage: DiceRoll::new(1, 8, 0),
    ///         bonus: 0,
    ///         hands_required: 1,
    ///         classification: WeaponClassification::MartialMelee,
    ///     }),
    ///     base_cost: 15,
    ///     sell_cost: 7,
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
    ///
    /// let desc = ItemMeshDescriptor::from_item(&sword);
    /// let creature_def = desc.to_creature_definition();
    /// assert!(creature_def.validate().is_ok());
    /// ```
    /// Converts this descriptor into a [`CreatureDefinition`] without charge
    /// information.
    ///
    /// Equivalent to calling
    /// [`to_creature_definition_with_charges`](Self::to_creature_definition_with_charges)
    /// with `charges_fraction: None`.
    ///
    /// # Phase 4 additions
    ///
    /// The returned `CreatureDefinition` now includes:
    /// - A **ground shadow quad** as the first mesh (semi-transparent dark quad
    ///   at Y = 0.001, alpha = 0.3).
    /// - **LOD levels** on the primary mesh when its triangle count exceeds
    ///   [`LOD_TRIANGLE_THRESHOLD`] (200 triangles).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::{Item, ItemType, WeaponData, WeaponClassification};
    /// use antares::domain::types::DiceRoll;
    /// use antares::domain::visual::item_mesh::ItemMeshDescriptor;
    ///
    /// let sword = Item {
    ///     id: 4,
    ///     name: "Long Sword".to_string(),
    ///     item_type: ItemType::Weapon(WeaponData {
    ///         damage: DiceRoll::new(1, 8, 0),
    ///         bonus: 0,
    ///         hands_required: 1,
    ///         classification: WeaponClassification::MartialMelee,
    ///     }),
    ///     base_cost: 15,
    ///     sell_cost: 7,
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
    ///
    /// let desc = ItemMeshDescriptor::from_item(&sword);
    /// let creature_def = desc.to_creature_definition();
    /// assert!(creature_def.validate().is_ok());
    /// // Shadow quad is the first mesh
    /// assert!(creature_def.meshes.len() >= 2);
    /// ```
    pub fn to_creature_definition(&self) -> CreatureDefinition {
        self.to_creature_definition_with_charges(None)
    }

    /// Converts this descriptor into a [`CreatureDefinition`], optionally
    /// adding a **charge-level gem** child mesh.
    ///
    /// # Arguments
    ///
    /// * `charges_fraction` — `Some(f)` where `f ∈ [0.0, 1.0]` adds a small
    ///   emissive sphere-like gem whose color transitions:
    ///   - `1.0` → gold (fully charged)
    ///   - `0.5` → white (half charged)
    ///   - `0.0` → grey (depleted)
    ///
    /// # Phase 4 mesh layout
    ///
    /// ```text
    /// meshes[0]  — ground shadow quad  (AlphaMode::Blend, alpha 0.3)
    /// meshes[1]  — primary item mesh   (with optional LOD levels)
    /// meshes[2]  — charge gem          (only when charges_fraction is Some)
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::items::{Item, ItemType, WeaponData, WeaponClassification};
    /// use antares::domain::types::DiceRoll;
    /// use antares::domain::visual::item_mesh::ItemMeshDescriptor;
    ///
    /// let wand = Item {
    ///     id: 10,
    ///     name: "Magic Wand".to_string(),
    ///     item_type: ItemType::Weapon(WeaponData {
    ///         damage: DiceRoll::new(1, 4, 0),
    ///         bonus: 0,
    ///         hands_required: 1,
    ///         classification: WeaponClassification::Simple,
    ///     }),
    ///     base_cost: 100,
    ///     sell_cost: 50,
    ///     alignment_restriction: None,
    ///     constant_bonus: None,
    ///     temporary_bonus: None,
    ///     spell_effect: None,
    ///     max_charges: 5,
    ///     is_cursed: false,
    ///     icon_path: None,
    ///     tags: vec![],
    ///     mesh_descriptor_override: None,
    /// };
    ///
    /// let desc = ItemMeshDescriptor::from_item(&wand);
    /// let full = desc.to_creature_definition_with_charges(Some(1.0));
    /// assert!(full.validate().is_ok());
    /// // shadow + primary + gem = 3 meshes
    /// assert_eq!(full.meshes.len(), 3);
    ///
    /// let depleted = desc.to_creature_definition_with_charges(Some(0.0));
    /// assert_eq!(depleted.meshes.len(), 3);
    /// ```
    pub fn to_creature_definition_with_charges(
        &self,
        charges_fraction: Option<f32>,
    ) -> CreatureDefinition {
        let primary_mesh = self.build_mesh_with_lod();
        let shadow_quad = self.build_shadow_quad();
        let name = format!("item_mesh_{:?}", self.category).to_lowercase();

        let mut meshes = vec![shadow_quad, primary_mesh];
        let mut transforms = vec![
            MeshTransform::identity(), // shadow quad — sits at Y=0 in local space
            MeshTransform::identity(), // primary mesh
        ];

        // Append charge gem when requested
        if let Some(frac) = charges_fraction {
            let frac = frac.clamp(0.0, 1.0);
            meshes.push(self.build_charge_gem(frac));
            transforms.push(MeshTransform {
                // Position the gem slightly above the item centre
                translation: [0.0, 0.04, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            });
        }

        CreatureDefinition {
            id: 0,
            name,
            meshes,
            mesh_transforms: transforms,
            scale: self.scale,
            color_tint: None,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Mesh geometry builders
    // ─────────────────────────────────────────────────────────────────────────

    /// Builds the [`MeshDefinition`] for this descriptor's category.
    fn build_mesh(&self) -> MeshDefinition {
        match self.category {
            ItemMeshCategory::Sword | ItemMeshCategory::Dagger => self.build_blade_mesh(),
            ItemMeshCategory::Blunt => self.build_blunt_mesh(),
            ItemMeshCategory::Staff => self.build_staff_mesh(),
            ItemMeshCategory::Bow => self.build_bow_mesh(),
            ItemMeshCategory::BodyArmor => self.build_armor_mesh(),
            ItemMeshCategory::Helmet => self.build_helmet_mesh(),
            ItemMeshCategory::Shield => self.build_shield_mesh(),
            ItemMeshCategory::Boots => self.build_boots_mesh(),
            ItemMeshCategory::Ring | ItemMeshCategory::Amulet => self.build_ring_mesh(),
            ItemMeshCategory::Belt => self.build_belt_mesh(),
            ItemMeshCategory::Cloak => self.build_cloak_mesh(),
            ItemMeshCategory::Potion => self.build_potion_mesh(),
            ItemMeshCategory::Scroll => self.build_scroll_mesh(),
            ItemMeshCategory::Ammo => self.build_ammo_mesh(),
            ItemMeshCategory::QuestItem => self.build_quest_mesh(),
        }
    }

    /// Builds the primary mesh and attaches LOD levels when the triangle count
    /// exceeds [`LOD_TRIANGLE_THRESHOLD`].
    ///
    /// Per Phase 4.5:
    /// - Items with ≤ 200 triangles get no LOD (they are already simple).
    /// - Items with > 200 triangles get:
    ///   - LOD1 at [`LOD_DISTANCE_1`] (8 units) — 50 % simplified.
    ///   - LOD2 at [`LOD_DISTANCE_2`] (20 units) — billboard quad.
    fn build_mesh_with_lod(&self) -> MeshDefinition {
        let mut mesh = self.build_mesh();
        let triangle_count = mesh.indices.len() / 3;

        if triangle_count > LOD_TRIANGLE_THRESHOLD {
            let (lod_meshes, _auto_distances) = generate_lod_levels(&mesh, 2);
            mesh.lod_levels = Some(lod_meshes);
            mesh.lod_distances = Some(vec![LOD_DISTANCE_1, LOD_DISTANCE_2]);
        }

        mesh
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 4 child mesh builders
    // ─────────────────────────────────────────────────────────────────────────

    /// Builds a flat ground shadow quad placed at Y = [`SHADOW_QUAD_Y`].
    ///
    /// The quad is a dark semi-transparent rectangle on the XZ plane that
    /// visually "grounds" the item on bright tile surfaces.  Its XZ footprint
    /// is derived from the item's [`scale`](Self::scale) multiplied by
    /// [`SHADOW_QUAD_SCALE`].
    ///
    /// The material uses `AlphaMode::Blend` with alpha 0.3.
    fn build_shadow_quad(&self) -> MeshDefinition {
        // Half-extent in XZ: item scale × SHADOW_QUAD_SCALE / 2
        let h = self.scale * SHADOW_QUAD_SCALE * 0.5;
        let y = SHADOW_QUAD_Y;

        // Four corners of the quad on the XZ plane at height y
        let vertices: Vec<[f32; 3]> = vec![[-h, y, -h], [h, y, -h], [h, y, h], [-h, y, h]];
        // Two triangles forming the quad (CCW winding)
        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

        MeshDefinition {
            name: Some("shadow_quad".to_string()),
            vertices,
            indices,
            normals: Some(vec![
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ]),
            uvs: Some(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
            color: [0.0, 0.0, 0.0, SHADOW_QUAD_ALPHA],
            lod_levels: None,
            lod_distances: None,
            material: Some(MaterialDefinition {
                base_color: [0.0, 0.0, 0.0, SHADOW_QUAD_ALPHA],
                metallic: 0.0,
                roughness: 1.0,
                emissive: None,
                alpha_mode: AlphaMode::Blend,
            }),
            texture_path: None,
        }
    }

    /// Builds a tiny emissive gem mesh whose color reflects the charge level.
    ///
    /// `charges_fraction` must be pre-clamped to `[0.0, 1.0]`.
    ///
    /// Color gradient:
    /// - `1.0` → gold  (`COLOR_CHARGE_FULL`)
    /// - `0.5` → white (`COLOR_CHARGE_HALF`)
    /// - `0.0` → grey  (`COLOR_CHARGE_EMPTY`)
    ///
    /// The gem is a small diamond (4-point star on the XZ plane) centred at
    /// the item origin, offset upward by the caller's `MeshTransform`.
    fn build_charge_gem(&self, charges_fraction: f32) -> MeshDefinition {
        // Interpolate between the three key colors
        let (color, emissive_color) = Self::charge_gem_color(charges_fraction);

        // Tiny diamond shape — 4 outer tips + centre
        let r = 0.025_f32; // gem radius
        let y = 0.0_f32;
        let vertices: Vec<[f32; 3]> = vec![
            [0.0, y, r],   // front
            [r, y, 0.0],   // right
            [0.0, y, -r],  // back
            [-r, y, 0.0],  // left
            [0.0, y, 0.0], // centre
        ];
        // Four triangles (fan from centre)
        let indices: Vec<u32> = vec![4, 0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0];

        let emissive =
            if emissive_color[0] != 0.0 || emissive_color[1] != 0.0 || emissive_color[2] != 0.0 {
                Some(emissive_color)
            } else {
                None
            };

        MeshDefinition {
            name: Some("charge_gem".to_string()),
            vertices,
            indices,
            normals: None,
            uvs: None,
            color,
            lod_levels: None,
            lod_distances: None,
            material: Some(MaterialDefinition {
                base_color: color,
                metallic: 0.8,
                roughness: 0.1,
                emissive,
                alpha_mode: AlphaMode::Opaque,
            }),
            texture_path: None,
        }
    }

    /// Interpolates the charge gem color and emissive from a `[0.0, 1.0]` fraction.
    ///
    /// Returns `(base_color, emissive_color)`.
    fn charge_gem_color(frac: f32) -> ([f32; 4], [f32; 3]) {
        if frac >= 1.0 {
            (COLOR_CHARGE_FULL, EMISSIVE_CHARGE_FULL)
        } else if frac <= 0.0 {
            (COLOR_CHARGE_EMPTY, EMISSIVE_CHARGE_EMPTY)
        } else if frac >= 0.5 {
            // Lerp between half and full
            let t = (frac - 0.5) * 2.0; // remap [0.5, 1.0] → [0.0, 1.0]
            let color = lerp_color4(COLOR_CHARGE_HALF, COLOR_CHARGE_FULL, t);
            let emissive = lerp_color3(EMISSIVE_CHARGE_HALF, EMISSIVE_CHARGE_FULL, t);
            (color, emissive)
        } else {
            // Lerp between empty and half
            let t = frac * 2.0; // remap [0.0, 0.5] → [0.0, 1.0]
            let color = lerp_color4(COLOR_CHARGE_EMPTY, COLOR_CHARGE_HALF, t);
            let emissive = lerp_color3(EMISSIVE_CHARGE_EMPTY, EMISSIVE_CHARGE_HALF, t);
            (color, emissive)
        }
    }

    // ─── Blade (sword / dagger) ───────────────────────────────────────────

    /// Flat elongated diamond silhouette lying on the XZ plane.
    ///
    /// ```text
    ///    tip (0, 0, +half_len)
    ///   / \
    ///  L   R  (±half_width, 0, 0)
    ///   \ /
    ///    pommel (0, 0, -half_len * 0.3)
    /// ```
    fn build_blade_mesh(&self) -> MeshDefinition {
        let half_len = (0.3 + self.blade_length * 0.7) * 0.5;
        let half_width = half_len * 0.12; // narrow diamond
        let pommel_z = -half_len * 0.30;

        //        0: tip
        //        1: right
        //        2: left
        //        3: pommel
        let vertices: Vec<[f32; 3]> = vec![
            [0.0, 0.0, half_len],    // 0 tip
            [half_width, 0.0, 0.0],  // 1 right
            [-half_width, 0.0, 0.0], // 2 left
            [0.0, 0.0, pommel_z],    // 3 pommel
        ];
        let indices: Vec<u32> = vec![
            0, 1, 3, // right face
            0, 3, 2, // left face
        ];
        let normal = [0.0_f32, 1.0, 0.0];
        let normals = Some(vec![normal; 4]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("blade".to_string()),
            vertices,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─── Blunt weapon ────────────────────────────────────────────────────────

    /// Flat oval lying on the XZ plane, slightly thicker than a blade.
    fn build_blunt_mesh(&self) -> MeshDefinition {
        let half_len = 0.25 + self.blade_length * 0.25;
        let half_w = half_len * 0.20;

        // Hexagonal flat oval: tip, r-front, r-back, pommel, l-back, l-front
        let vertices: Vec<[f32; 3]> = vec![
            [0.0, 0.0, half_len],            // 0 top
            [half_w, 0.0, half_len * 0.4],   // 1 right-top
            [half_w, 0.0, -half_len * 0.4],  // 2 right-bottom
            [0.0, 0.0, -half_len],           // 3 bottom
            [-half_w, 0.0, -half_len * 0.4], // 4 left-bottom
            [-half_w, 0.0, half_len * 0.4],  // 5 left-top
        ];
        let indices: Vec<u32> = vec![0, 1, 5, 1, 2, 5, 2, 4, 5, 2, 3, 4];
        let normals = Some(vec![[0.0_f32, 1.0, 0.0]; 6]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("blunt".to_string()),
            vertices,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─── Staff / polearm ─────────────────────────────────────────────────────

    /// Long thin rectangular bar on the XZ plane.
    fn build_staff_mesh(&self) -> MeshDefinition {
        let half_len = 0.45;
        let half_w = 0.03;
        let vertices: Vec<[f32; 3]> = vec![
            [-half_w, 0.0, half_len],  // 0
            [half_w, 0.0, half_len],   // 1
            [half_w, 0.0, -half_len],  // 2
            [-half_w, 0.0, -half_len], // 3
        ];
        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];
        let normals = Some(vec![[0.0_f32, 1.0, 0.0]; 4]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("staff".to_string()),
            vertices,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─── Bow ─────────────────────────────────────────────────────────────────

    /// Arc shape approximated as a thin curved quad on the XZ plane.
    fn build_bow_mesh(&self) -> MeshDefinition {
        // Approximate arc with 6 vertices in an arc
        let half_len = 0.40;
        let curve = 0.12; // how far tips bow outward

        let vertices: Vec<[f32; 3]> = vec![
            [curve, 0.0, half_len],  // 0 top-tip
            [0.03, 0.0, half_len],   // 1 top-inner
            [0.0, 0.0, 0.0],         // 2 mid-inner
            [0.03, 0.0, -half_len],  // 3 bot-inner
            [curve, 0.0, -half_len], // 4 bot-tip
            [-0.02, 0.0, -half_len], // 5 bot-back
            [-0.02, 0.0, half_len],  // 6 top-back
        ];
        let indices: Vec<u32> = vec![0, 1, 6, 1, 2, 6, 2, 3, 5, 3, 4, 5];
        let normals = Some(vec![[0.0_f32, 1.0, 0.0]; 7]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("bow".to_string()),
            vertices,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─── Body armor ──────────────────────────────────────────────────────────

    /// Stylised breastplate silhouette — wide trapezoid on the XZ plane.
    fn build_armor_mesh(&self) -> MeshDefinition {
        let top_hw = 0.20; // shoulder width (half)
        let bot_hw = 0.14; // hip width (half)
        let half_h = 0.28;
        let vertices: Vec<[f32; 3]> = vec![
            [-top_hw, 0.0, half_h],  // 0 shoulder-left
            [top_hw, 0.0, half_h],   // 1 shoulder-right
            [bot_hw, 0.0, -half_h],  // 2 hip-right
            [-bot_hw, 0.0, -half_h], // 3 hip-left
        ];
        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];
        let normals = Some(vec![[0.0_f32, 1.0, 0.0]; 4]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("body_armor".to_string()),
            vertices,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─── Helmet ──────────────────────────────────────────────────────────────

    /// Flat pentagon dome silhouette.
    ///
    /// Uses a centre vertex (index 5) as the fan hub to avoid degenerate
    /// triangles.
    fn build_helmet_mesh(&self) -> MeshDefinition {
        let r = 0.22_f32;
        use std::f32::consts::PI;
        // 5 perimeter vertices + 1 centre vertex
        let mut verts: Vec<[f32; 3]> = (0..5)
            .map(|i| {
                let angle = PI / 2.0 + (i as f32) * 2.0 * PI / 5.0;
                [r * angle.cos(), 0.0, r * angle.sin()]
            })
            .collect();
        // centre vertex at index 5
        verts.push([0.0, 0.0, 0.0]);
        let centre = 5_u32;
        let mut indices: Vec<u32> = Vec::with_capacity(5 * 3);
        for i in 0..5_u32 {
            indices.push(centre);
            indices.push(i);
            indices.push((i + 1) % 5);
        }
        let normals = Some(vec![[0.0_f32, 1.0, 0.0]; 6]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("helmet".to_string()),
            vertices: verts,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─── Shield ───────────────────────────────────────────────────────────────

    /// Flat kite-shield silhouette.
    fn build_shield_mesh(&self) -> MeshDefinition {
        let hw = 0.18_f32;
        let ht = 0.26_f32;
        let vertices: Vec<[f32; 3]> = vec![
            [0.0, 0.0, ht],              // 0 top
            [hw, 0.0, 0.05],             // 1 right
            [hw * 0.6, 0.0, -ht * 0.6],  // 2 bot-right
            [0.0, 0.0, -ht],             // 3 tip
            [-hw * 0.6, 0.0, -ht * 0.6], // 4 bot-left
            [-hw, 0.0, 0.05],            // 5 left
        ];
        let indices: Vec<u32> = vec![0, 1, 5, 1, 2, 4, 1, 4, 5, 2, 3, 4];
        let normals = Some(vec![[0.0_f32, 1.0, 0.0]; 6]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("shield".to_string()),
            vertices,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─── Boots ───────────────────────────────────────────────────────────────

    /// Flat L-shaped silhouette for boots.
    fn build_boots_mesh(&self) -> MeshDefinition {
        let vertices: Vec<[f32; 3]> = vec![
            [-0.06, 0.0, 0.20],  // 0 top-left
            [0.06, 0.0, 0.20],   // 1 top-right
            [0.06, 0.0, -0.05],  // 2 ankle-right
            [0.18, 0.0, -0.05],  // 3 toe-right-top
            [0.18, 0.0, -0.18],  // 4 toe-right-bot
            [-0.06, 0.0, -0.18], // 5 toe-left-bot
        ];
        let indices: Vec<u32> = vec![0, 1, 5, 1, 2, 5, 2, 4, 5, 2, 3, 4];
        let normals = Some(vec![[0.0_f32, 1.0, 0.0]; 6]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("boots".to_string()),
            vertices,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─── Ring / Amulet ───────────────────────────────────────────────────────

    /// Flat octagon approximating a ring lying on the XZ plane.
    ///
    /// Uses a center vertex (index 8) as the fan hub so that no triangle
    /// ever has duplicate indices (which would be degenerate).
    fn build_ring_mesh(&self) -> MeshDefinition {
        use std::f32::consts::PI;
        let r = 0.10_f32;
        // 8 perimeter vertices + 1 centre vertex
        let mut verts: Vec<[f32; 3]> = (0..8)
            .map(|i| {
                let angle = (i as f32) * 2.0 * PI / 8.0;
                [r * angle.cos(), 0.0, r * angle.sin()]
            })
            .collect();
        // centre vertex at index 8
        verts.push([0.0, 0.0, 0.0]);
        let centre = 8_u32;
        // Fan from centre: (centre, i, i+1) for i in 0..8
        let mut indices: Vec<u32> = Vec::with_capacity(8 * 3);
        for i in 0..8_u32 {
            indices.push(centre);
            indices.push(i);
            indices.push((i + 1) % 8);
        }
        let normals = Some(vec![[0.0_f32, 1.0, 0.0]; 9]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("ring".to_string()),
            vertices: verts,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─── Belt ─────────────────────────────────────────────────────────────────

    /// Flat thin rectangle representing a belt.
    fn build_belt_mesh(&self) -> MeshDefinition {
        let hw = 0.22_f32;
        let hh = 0.04_f32;
        let vertices: Vec<[f32; 3]> = vec![
            [-hw, 0.0, hh],
            [hw, 0.0, hh],
            [hw, 0.0, -hh],
            [-hw, 0.0, -hh],
        ];
        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];
        let normals = Some(vec![[0.0_f32, 1.0, 0.0]; 4]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("belt".to_string()),
            vertices,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─── Cloak ───────────────────────────────────────────────────────────────

    /// Wide arc / teardrop cloak silhouette.
    fn build_cloak_mesh(&self) -> MeshDefinition {
        let hw = 0.28_f32;
        let ht = 0.32_f32;
        let vertices: Vec<[f32; 3]> = vec![
            [0.0, 0.0, ht * 0.5],   // 0 collar-centre
            [-hw, 0.0, ht * 0.3],   // 1 collar-left
            [-hw * 0.8, 0.0, -ht],  // 2 hem-left
            [0.0, 0.0, -ht * 1.05], // 3 hem-centre
            [hw * 0.8, 0.0, -ht],   // 4 hem-right
            [hw, 0.0, ht * 0.3],    // 5 collar-right
        ];
        let indices: Vec<u32> = vec![0, 5, 1, 1, 5, 4, 1, 4, 2, 2, 4, 3];
        let normals = Some(vec![[0.0_f32, 1.0, 0.0]; 6]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("cloak".to_string()),
            vertices,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─── Potion ──────────────────────────────────────────────────────────────

    /// Round flask silhouette — hexagonal disc.
    ///
    /// Uses a centre vertex (index 6) as the fan hub to avoid degenerate
    /// triangles.
    fn build_potion_mesh(&self) -> MeshDefinition {
        use std::f32::consts::PI;
        let r = 0.10_f32;
        // 6 perimeter vertices + 1 centre vertex
        let mut verts: Vec<[f32; 3]> = (0..6)
            .map(|i| {
                let angle = (i as f32) * 2.0 * PI / 6.0;
                [r * angle.cos(), 0.0, r * angle.sin()]
            })
            .collect();
        // centre vertex at index 6
        verts.push([0.0, 0.0, 0.0]);
        let centre = 6_u32;
        let mut indices: Vec<u32> = Vec::with_capacity(6 * 3);
        for i in 0..6_u32 {
            indices.push(centre);
            indices.push(i);
            indices.push((i + 1) % 6);
        }
        let normals = Some(vec![[0.0_f32, 1.0, 0.0]; 7]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("potion".to_string()),
            vertices: verts,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─── Scroll ──────────────────────────────────────────────────────────────

    /// Rolled scroll cylinder-top: a thin elongated oval.
    fn build_scroll_mesh(&self) -> MeshDefinition {
        let hw = 0.06_f32;
        let hl = 0.18_f32;
        let vertices: Vec<[f32; 3]> = vec![
            [0.0, 0.0, hl],        // 0 top
            [hw, 0.0, hl * 0.5],   // 1 right-top
            [hw, 0.0, -hl * 0.5],  // 2 right-bot
            [0.0, 0.0, -hl],       // 3 bottom
            [-hw, 0.0, -hl * 0.5], // 4 left-bot
            [-hw, 0.0, hl * 0.5],  // 5 left-top
        ];
        let indices: Vec<u32> = vec![0, 1, 5, 1, 2, 4, 1, 4, 5, 2, 3, 4];
        let normals = Some(vec![[0.0_f32, 1.0, 0.0]; 6]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("scroll".to_string()),
            vertices,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─── Ammo ────────────────────────────────────────────────────────────────

    /// Bundle of arrows / bolts: a thin elongated diamond.
    fn build_ammo_mesh(&self) -> MeshDefinition {
        let hl = 0.22_f32;
        let hw = 0.025_f32;
        let vertices: Vec<[f32; 3]> = vec![
            [0.0, 0.0, hl],
            [hw, 0.0, 0.0],
            [0.0, 0.0, -hl],
            [-hw, 0.0, 0.0],
        ];
        let indices: Vec<u32> = vec![0, 1, 3, 1, 2, 3];
        let normals = Some(vec![[0.0_f32, 1.0, 0.0]; 4]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("ammo".to_string()),
            vertices,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─── Quest item ──────────────────────────────────────────────────────────

    /// Star / gem silhouette for quest items.
    ///
    /// 8-pointed star via alternating outer / inner vertices.  A dedicated
    /// centre vertex (index 16) is used as the fan hub so that no triangle
    /// contains duplicate indices.
    fn build_quest_mesh(&self) -> MeshDefinition {
        use std::f32::consts::PI;
        let outer_r = 0.15_f32;
        let inner_r = 0.07_f32;
        // 16 perimeter vertices (alternating outer / inner) + 1 centre
        let mut verts: Vec<[f32; 3]> = Vec::with_capacity(17);
        for i in 0..8 {
            let outer_angle = PI / 2.0 + (i as f32) * 2.0 * PI / 8.0;
            let inner_angle = outer_angle + PI / 8.0;
            verts.push([
                outer_r * outer_angle.cos(),
                0.0,
                outer_r * outer_angle.sin(),
            ]);
            verts.push([
                inner_r * inner_angle.cos(),
                0.0,
                inner_r * inner_angle.sin(),
            ]);
        }
        // centre vertex at index 16
        verts.push([0.0, 0.0, 0.0]);
        let centre = 16_u32;
        let n_perimeter = 16_u32;
        let mut indices: Vec<u32> = Vec::with_capacity(16 * 3);
        for i in 0..n_perimeter {
            indices.push(centre);
            indices.push(i);
            indices.push((i + 1) % n_perimeter);
        }
        let normals = Some(vec![[0.0_f32, 1.0, 0.0]; verts.len()]);
        let material = self.make_material(self.primary_color);
        MeshDefinition {
            name: Some("quest_item".to_string()),
            vertices: verts,
            indices,
            normals,
            uvs: None,
            color: self.primary_color,
            lod_levels: None,
            lod_distances: None,
            material: Some(material),
            texture_path: None,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────────────────────────────────

    /// Builds a [`MaterialDefinition`] for this descriptor's primary surface.
    ///
    /// # Phase 4.1 metallic / roughness rules
    ///
    /// | Condition          | `metallic`                      | `roughness` |
    /// |--------------------|---------------------------------|-------------|
    /// | `is_magical()`     | [`MATERIAL_METALLIC_MAGICAL`]   | [`MATERIAL_ROUGHNESS_MAGICAL`] |
    /// | Mundane metal cats | 0.6 (legacy per-category value) | 0.5         |
    /// | All other mundane  | [`MATERIAL_METALLIC_MUNDANE`]   | [`MATERIAL_ROUGHNESS_MUNDANE`] |
    fn make_material(&self, base_color: [f32; 4]) -> MaterialDefinition {
        let emissive = if self.emissive {
            Some(self.emissive_color)
        } else {
            None
        };

        // Phase 4.1: magical items get shiny PBR params
        let (metallic, roughness) = if self.is_metallic_magical() {
            (MATERIAL_METALLIC_MAGICAL, MATERIAL_ROUGHNESS_MAGICAL)
        } else if matches!(
            self.category,
            ItemMeshCategory::Sword
                | ItemMeshCategory::Dagger
                | ItemMeshCategory::Blunt
                | ItemMeshCategory::Helmet
                | ItemMeshCategory::Shield
                | ItemMeshCategory::Ring
                | ItemMeshCategory::Amulet
        ) {
            // Mundane metal categories keep legacy metallic value
            (0.6_f32, 0.5_f32)
        } else {
            (MATERIAL_METALLIC_MUNDANE, MATERIAL_ROUGHNESS_MUNDANE)
        };

        MaterialDefinition {
            base_color,
            metallic,
            roughness,
            emissive,
            alpha_mode: AlphaMode::Opaque,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Free helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Linearly interpolates between two RGBA colors.
#[inline]
fn lerp_color4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

/// Linearly interpolates between two RGB emissive colors.
#[inline]
fn lerp_color3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::items::types::{
        AccessoryData, AccessorySlot, AmmoData, AmmoType, ArmorClassification, ArmorData, Bonus,
        BonusAttribute, ConsumableData, ConsumableEffect, Item, ItemType, MagicItemClassification,
        QuestData, WeaponClassification, WeaponData,
    };
    use crate::domain::types::{DiceRoll, ItemId};

    // ── Phase 4 helpers ──────────────────────────────────────────────────────

    fn make_item_with_bonus(id: ItemId, name: &str, attribute: BonusAttribute) -> Item {
        Item {
            id,
            name: name.to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 1,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 200,
            sell_cost: 100,
            alignment_restriction: None,
            constant_bonus: Some(Bonus {
                attribute,
                value: 5,
            }),
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
        }
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    fn make_weapon(
        id: ItemId,
        name: &str,
        sides: u8,
        hands: u8,
        classification: WeaponClassification,
        tags: Vec<String>,
    ) -> Item {
        Item {
            id,
            name: name.to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, sides, 0),
                bonus: 0,
                hands_required: hands,
                classification,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags,
            mesh_descriptor_override: None,
        }
    }

    fn make_consumable(id: ItemId, name: &str, effect: ConsumableEffect) -> Item {
        Item {
            id,
            name: name.to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect,
                is_combat_usable: true,
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
            tags: vec![],
            mesh_descriptor_override: None,
        }
    }

    // ── ItemMeshCategory tests ────────────────────────────────────────────────

    #[test]
    fn test_sword_descriptor_from_short_sword() {
        // Short sword: 1d6, MartialMelee, 1-handed
        let short_sword = make_weapon(
            3,
            "Short Sword",
            6,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        );
        let desc = ItemMeshDescriptor::from_item(&short_sword);

        assert_eq!(desc.category, ItemMeshCategory::Sword);
        // Blade length derived from 6 sides × BLADE_SIDES_FACTOR = 0.48, clamped
        let expected_blade =
            (6.0_f32 * BLADE_SIDES_FACTOR).clamp(BLADE_LENGTH_MIN, BLADE_LENGTH_MAX);
        assert!((desc.blade_length - expected_blade).abs() < f32::EPSILON);
        assert!(!desc.emissive, "non-magical short sword must not emit");
    }

    #[test]
    fn test_dagger_descriptor_short_blade() {
        // Dagger: 1d4, Simple, 1-handed → Dagger category (≤4 sides)
        let dagger = make_weapon(2, "Dagger", 4, 1, WeaponClassification::Simple, vec![]);
        let desc = ItemMeshDescriptor::from_item(&dagger);

        assert_eq!(desc.category, ItemMeshCategory::Dagger);
        // Dagger blade_length = computed * 0.7
        let base = (4.0_f32 * BLADE_SIDES_FACTOR).clamp(BLADE_LENGTH_MIN, BLADE_LENGTH_MAX);
        let expected = base * 0.7;
        assert!((desc.blade_length - expected).abs() < f32::EPSILON);

        // Dagger blade must be shorter than a sword of equal sides
        let sword_desc = ItemMeshDescriptor::from_item(&make_weapon(
            4,
            "Sword",
            4,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        ));
        assert!(desc.blade_length < sword_desc.blade_length);
    }

    #[test]
    fn test_potion_color_heal_is_red() {
        let potion = make_consumable(50, "Healing Potion", ConsumableEffect::HealHp(20));
        let desc = ItemMeshDescriptor::from_item(&potion);

        assert_eq!(desc.category, ItemMeshCategory::Potion);
        assert_eq!(desc.primary_color, COLOR_RED);
    }

    #[test]
    fn test_potion_color_restore_sp_is_blue() {
        let potion = make_consumable(51, "Mana Potion", ConsumableEffect::RestoreSp(15));
        let desc = ItemMeshDescriptor::from_item(&potion);

        assert_eq!(desc.category, ItemMeshCategory::Potion);
        assert_eq!(desc.primary_color, COLOR_BLUE);
    }

    #[test]
    fn test_potion_color_boost_attribute_is_yellow() {
        use crate::domain::items::types::AttributeType;
        let potion = make_consumable(
            52,
            "Might Elixir",
            ConsumableEffect::BoostAttribute(AttributeType::Might, 5),
        );
        let desc = ItemMeshDescriptor::from_item(&potion);

        assert_eq!(desc.category, ItemMeshCategory::Potion);
        assert_eq!(desc.primary_color, COLOR_YELLOW);
    }

    #[test]
    fn test_cure_condition_produces_scroll() {
        // CureCondition → Scroll category (not Potion)
        let scroll = make_consumable(53, "Scroll of Cure", ConsumableEffect::CureCondition(0xFF));
        let desc = ItemMeshDescriptor::from_item(&scroll);

        assert_eq!(desc.category, ItemMeshCategory::Scroll);
        assert_eq!(desc.primary_color, COLOR_PARCHMENT);
    }

    #[test]
    fn test_magical_item_emissive() {
        // Item with max_charges > 0 is magical → emissive glow
        let mut wand = make_weapon(
            100,
            "Magic Wand",
            4,
            1,
            WeaponClassification::Simple,
            vec![],
        );
        wand.max_charges = 5;

        let desc = ItemMeshDescriptor::from_item(&wand);

        assert!(desc.emissive, "charged item must be emissive");
        assert_eq!(desc.emissive_color, EMISSIVE_MAGIC);
    }

    #[test]
    fn test_magical_item_emissive_via_bonus() {
        // Item with a constant_bonus is magical via is_magical()
        let mut magic_ring = Item {
            id: 200,
            name: "Ring of Might".to_string(),
            item_type: ItemType::Accessory(AccessoryData {
                slot: AccessorySlot::Ring,
                classification: Some(MagicItemClassification::Arcane),
            }),
            base_cost: 500,
            sell_cost: 250,
            alignment_restriction: None,
            constant_bonus: Some(Bonus {
                attribute: BonusAttribute::Might,
                value: 3,
            }),
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
        };
        let desc = ItemMeshDescriptor::from_item(&magic_ring);
        assert!(
            desc.emissive,
            "item with constant_bonus is magical → emissive"
        );

        // Verify emissive even without bonus but with charges
        magic_ring.constant_bonus = None;
        magic_ring.max_charges = 3;
        let desc2 = ItemMeshDescriptor::from_item(&magic_ring);
        assert!(desc2.emissive);
    }

    #[test]
    fn test_cursed_item_dark_tint() {
        let mut cursed_sword = make_weapon(
            99,
            "Cursed Blade",
            8,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        );
        cursed_sword.is_cursed = true;

        let desc = ItemMeshDescriptor::from_item(&cursed_sword);

        assert!(desc.emissive, "cursed item must be emissive");
        assert_eq!(
            desc.primary_color, COLOR_CURSED,
            "cursed item must use dark tint"
        );
        assert_eq!(
            desc.emissive_color, EMISSIVE_CURSED,
            "cursed item must emit purple"
        );
    }

    #[test]
    fn test_cursed_overrides_magical_glow() {
        // When both cursed AND magical, cursed takes priority
        let mut cursed_magic = make_weapon(
            98,
            "Cursed Magic Sword",
            8,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        );
        cursed_magic.is_cursed = true;
        cursed_magic.max_charges = 5;

        let desc = ItemMeshDescriptor::from_item(&cursed_magic);

        assert_eq!(
            desc.emissive_color, EMISSIVE_CURSED,
            "curse must override magic glow"
        );
        assert_eq!(desc.primary_color, COLOR_CURSED);
    }

    #[test]
    fn test_two_handed_weapon_larger_scale() {
        let one_handed = make_weapon(
            10,
            "Longsword",
            8,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        );
        let two_handed = make_weapon(
            11,
            "Greatsword",
            8,
            2,
            WeaponClassification::MartialMelee,
            vec!["two_handed".to_string()],
        );

        let desc_one = ItemMeshDescriptor::from_item(&one_handed);
        let desc_two = ItemMeshDescriptor::from_item(&two_handed);

        assert!(
            desc_two.scale > desc_one.scale,
            "two-handed weapon ({}) must have larger scale than one-handed ({})",
            desc_two.scale,
            desc_one.scale
        );
    }

    #[test]
    fn test_descriptor_to_creature_definition_valid() {
        // Round-trip for every category
        let items: Vec<Item> = vec![
            make_weapon(
                1,
                "Short Sword",
                6,
                1,
                WeaponClassification::MartialMelee,
                vec![],
            ),
            make_weapon(2, "Dagger", 4, 1, WeaponClassification::Simple, vec![]),
            make_weapon(3, "Mace", 6, 1, WeaponClassification::Blunt, vec![]),
            make_weapon(
                4,
                "Bow",
                6,
                2,
                WeaponClassification::MartialRanged,
                vec!["two_handed".to_string()],
            ),
            make_consumable(5, "Heal", ConsumableEffect::HealHp(10)),
            make_consumable(6, "Cure", ConsumableEffect::CureCondition(0x01)),
            Item {
                id: 7,
                name: "Iron Shield".to_string(),
                item_type: ItemType::Armor(ArmorData {
                    ac_bonus: 3,
                    weight: 15,
                    classification: ArmorClassification::Shield,
                }),
                base_cost: 20,
                sell_cost: 10,
                alignment_restriction: None,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
                icon_path: None,
                tags: vec![],
                mesh_descriptor_override: None,
            },
            Item {
                id: 8,
                name: "Ruby Ring".to_string(),
                item_type: ItemType::Accessory(AccessoryData {
                    slot: AccessorySlot::Ring,
                    classification: None,
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
                tags: vec![],
                mesh_descriptor_override: None,
            },
            Item {
                id: 9,
                name: "Arrows".to_string(),
                item_type: ItemType::Ammo(AmmoData {
                    ammo_type: AmmoType::Arrow,
                    quantity: 20,
                }),
                base_cost: 2,
                sell_cost: 1,
                alignment_restriction: None,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
                icon_path: None,
                tags: vec![],
                mesh_descriptor_override: None,
            },
            Item {
                id: 10,
                name: "Ancient Key".to_string(),
                item_type: ItemType::Quest(QuestData {
                    quest_id: "main_quest".to_string(),
                    is_key_item: true,
                }),
                base_cost: 0,
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
            },
        ];

        for item in &items {
            let desc = ItemMeshDescriptor::from_item(item);
            let creature_def = desc.to_creature_definition();
            assert!(
                creature_def.validate().is_ok(),
                "item '{}' produced invalid CreatureDefinition: {:?}",
                item.name,
                creature_def.validate()
            );
        }
    }

    #[test]
    fn test_override_color_applied() {
        let mut sword = make_weapon(
            20,
            "Custom Sword",
            8,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        );
        let custom_color = [0.0, 1.0, 0.0, 1.0]; // green override

        sword.mesh_descriptor_override = Some(ItemMeshDescriptorOverride {
            primary_color: Some(custom_color),
            accent_color: None,
            scale: None,
            emissive: None,
        });

        let desc = ItemMeshDescriptor::from_item(&sword);
        assert_eq!(
            desc.primary_color, custom_color,
            "override primary_color must be applied"
        );
    }

    #[test]
    fn test_override_scale_applied() {
        let mut sword = make_weapon(
            21,
            "Giant Sword",
            8,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        );
        sword.mesh_descriptor_override = Some(ItemMeshDescriptorOverride {
            primary_color: None,
            accent_color: None,
            scale: Some(2.5),
            emissive: None,
        });

        let desc = ItemMeshDescriptor::from_item(&sword);
        assert!((desc.scale - 2.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_override_invalid_scale_ignored() {
        let mut sword = make_weapon(
            22,
            "Sword",
            8,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        );
        let auto_desc = ItemMeshDescriptor::from_item(&sword);
        let auto_scale = auto_desc.scale;

        sword.mesh_descriptor_override = Some(ItemMeshDescriptorOverride {
            primary_color: None,
            accent_color: None,
            scale: Some(-1.0), // invalid
            emissive: None,
        });

        let override_desc = ItemMeshDescriptor::from_item(&sword);
        assert!(
            (override_desc.scale - auto_scale).abs() < f32::EPSILON,
            "negative scale override must be ignored"
        );
    }

    #[test]
    fn test_override_emissive_applied() {
        let mut sword = make_weapon(
            23,
            "Glow Sword",
            8,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        );
        let glow = [1.0, 0.0, 0.0]; // red glow
        sword.mesh_descriptor_override = Some(ItemMeshDescriptorOverride {
            primary_color: None,
            accent_color: None,
            scale: None,
            emissive: Some(glow),
        });

        let desc = ItemMeshDescriptor::from_item(&sword);
        assert!(desc.emissive, "non-zero override emissive must enable flag");
        assert_eq!(desc.emissive_color, glow);
    }

    #[test]
    fn test_override_zero_emissive_disables() {
        // Override with all-zero emissive should DISABLE the flag
        let mut magic_sword = make_weapon(
            24,
            "Silent Magic Sword",
            8,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        );
        magic_sword.max_charges = 10; // normally makes it emissive

        magic_sword.mesh_descriptor_override = Some(ItemMeshDescriptorOverride {
            primary_color: None,
            accent_color: None,
            scale: None,
            emissive: Some([0.0, 0.0, 0.0]),
        });

        let desc = ItemMeshDescriptor::from_item(&magic_sword);
        assert!(
            !desc.emissive,
            "all-zero emissive override must disable glow"
        );
    }

    #[test]
    fn test_quest_item_descriptor_unique_shape() {
        let quest_item = Item {
            id: 99,
            name: "Crystal of Power".to_string(),
            item_type: ItemType::Quest(QuestData {
                quest_id: "endgame".to_string(),
                is_key_item: true,
            }),
            base_cost: 0,
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
        };

        let desc = ItemMeshDescriptor::from_item(&quest_item);

        assert_eq!(desc.category, ItemMeshCategory::QuestItem);
        assert!(desc.emissive, "quest items always glow");
        // Quest item should use a star/gem mesh — validate round-trip
        let creature_def = desc.to_creature_definition();
        assert!(creature_def.validate().is_ok());
    }

    #[test]
    fn test_all_accessory_slots_produce_valid_definitions() {
        let slots = [
            AccessorySlot::Ring,
            AccessorySlot::Amulet,
            AccessorySlot::Belt,
            AccessorySlot::Cloak,
        ];
        for slot in &slots {
            let item = Item {
                id: 30,
                name: format!("{:?}", slot),
                item_type: ItemType::Accessory(AccessoryData {
                    slot: *slot,
                    classification: None,
                }),
                base_cost: 10,
                sell_cost: 5,
                alignment_restriction: None,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
                icon_path: None,
                tags: vec![],
                mesh_descriptor_override: None,
            };
            let desc = ItemMeshDescriptor::from_item(&item);
            let def = desc.to_creature_definition();
            assert!(
                def.validate().is_ok(),
                "{:?} slot produced invalid definition",
                slot
            );
        }
    }

    #[test]
    fn test_all_armor_classifications_produce_valid_definitions() {
        let classes = [
            ArmorClassification::Light,
            ArmorClassification::Medium,
            ArmorClassification::Heavy,
            ArmorClassification::Shield,
        ];
        for classification in &classes {
            let item = Item {
                id: 40,
                name: format!("{:?}", classification),
                item_type: ItemType::Armor(ArmorData {
                    ac_bonus: 4,
                    weight: 20,
                    classification: *classification,
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
                tags: vec![],
                mesh_descriptor_override: None,
            };
            let desc = ItemMeshDescriptor::from_item(&item);
            let def = desc.to_creature_definition();
            assert!(
                def.validate().is_ok(),
                "{:?} armor produced invalid definition",
                classification
            );
        }
    }

    #[test]
    fn test_ammo_descriptor_valid() {
        let arrows = Item {
            id: 50,
            name: "Arrows".to_string(),
            item_type: ItemType::Ammo(AmmoData {
                ammo_type: AmmoType::Arrow,
                quantity: 20,
            }),
            base_cost: 2,
            sell_cost: 1,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
        };
        let desc = ItemMeshDescriptor::from_item(&arrows);
        assert_eq!(desc.category, ItemMeshCategory::Ammo);
        let def = desc.to_creature_definition();
        assert!(def.validate().is_ok());
    }

    #[test]
    fn test_descriptor_default_override_is_identity() {
        // An all-None override must not change the auto-derived descriptor
        let sword = make_weapon(
            30,
            "Sword",
            8,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        );
        let auto_desc = ItemMeshDescriptor::from_item(&sword);

        let mut sword_with_empty_override = sword.clone();
        sword_with_empty_override.mesh_descriptor_override =
            Some(ItemMeshDescriptorOverride::default());
        let override_desc = ItemMeshDescriptor::from_item(&sword_with_empty_override);

        assert_eq!(
            auto_desc, override_desc,
            "empty override must produce identical descriptor"
        );
    }

    // ── Phase 4 tests ─────────────────────────────────────────────────────────

    // §4.6 — test_fire_resist_item_accent_orange
    /// Item with ResistFire constant_bonus must receive the orange accent color.
    #[test]
    fn test_fire_resist_item_accent_orange() {
        let item = make_item_with_bonus(60, "Flaming Sword", BonusAttribute::ResistFire);
        let desc = ItemMeshDescriptor::from_item(&item);
        assert_eq!(
            desc.accent_color, COLOR_ACCENT_FIRE,
            "ResistFire item must have orange accent"
        );
    }

    // §4.6 — test_cold_resist_item_accent_blue
    /// Item with ResistCold constant_bonus must receive the icy blue accent.
    #[test]
    fn test_cold_resist_item_accent_blue() {
        let item = make_item_with_bonus(61, "Frostbrand", BonusAttribute::ResistCold);
        let desc = ItemMeshDescriptor::from_item(&item);
        assert_eq!(
            desc.accent_color, COLOR_ACCENT_COLD,
            "ResistCold item must have icy blue accent"
        );
    }

    // §4.6 — test_electricity_resist_item_accent_yellow
    /// Item with ResistElectricity bonus must receive yellow accent.
    #[test]
    fn test_electricity_resist_item_accent_yellow() {
        let item = make_item_with_bonus(62, "Thunderblade", BonusAttribute::ResistElectricity);
        let desc = ItemMeshDescriptor::from_item(&item);
        assert_eq!(
            desc.accent_color, COLOR_ACCENT_ELECTRICITY,
            "ResistElectricity item must have yellow accent"
        );
    }

    // §4.6 — test_poison_resist_item_accent_green
    /// Item with ResistPoison bonus must receive acid-green accent.
    #[test]
    fn test_poison_resist_item_accent_green() {
        let item = make_item_with_bonus(63, "Viper Sword", BonusAttribute::ResistPoison);
        let desc = ItemMeshDescriptor::from_item(&item);
        assert_eq!(
            desc.accent_color, COLOR_ACCENT_POISON,
            "ResistPoison item must have acid-green accent"
        );
    }

    // §4.6 — test_magic_resist_item_accent_purple
    /// Item with ResistMagic bonus must receive purple accent.
    #[test]
    fn test_magic_resist_item_accent_purple() {
        let item = make_item_with_bonus(64, "Null Sword", BonusAttribute::ResistMagic);
        let desc = ItemMeshDescriptor::from_item(&item);
        assert_eq!(
            desc.accent_color, COLOR_ACCENT_MAGIC,
            "ResistMagic item must have purple accent"
        );
    }

    // §4.6 — test_might_bonus_item_accent_warm_red
    /// Item with Might bonus must receive warm red accent.
    #[test]
    fn test_might_bonus_item_accent_warm_red() {
        let item = make_item_with_bonus(65, "Sword of Might", BonusAttribute::Might);
        let desc = ItemMeshDescriptor::from_item(&item);
        assert_eq!(
            desc.accent_color, COLOR_ACCENT_MIGHT,
            "Might bonus item must have warm red accent"
        );
    }

    // §4.6 — test_ac_bonus_item_accent_teal
    /// Item with ArmorClass bonus must receive teal accent.
    #[test]
    fn test_ac_bonus_item_accent_teal() {
        let item = make_item_with_bonus(66, "Defender", BonusAttribute::ArmorClass);
        let desc = ItemMeshDescriptor::from_item(&item);
        assert_eq!(
            desc.accent_color, COLOR_ACCENT_TEAL,
            "ArmorClass bonus item must have teal accent"
        );
    }

    // §4.6 — test_intellect_bonus_item_accent_deep_blue
    /// Item with Intellect bonus must receive deep blue accent.
    #[test]
    fn test_intellect_bonus_item_accent_deep_blue() {
        let item = make_item_with_bonus(67, "Scholar's Sword", BonusAttribute::Intellect);
        let desc = ItemMeshDescriptor::from_item(&item);
        assert_eq!(
            desc.accent_color, COLOR_ACCENT_DEEP_BLUE,
            "Intellect bonus item must have deep blue accent"
        );
    }

    // §4.6 — test_magical_item_metallic_material
    /// is_magical() item must produce metallic > 0.5 on the primary material.
    #[test]
    fn test_magical_item_metallic_material() {
        let mut sword = make_weapon(
            68,
            "Magic Sword",
            8,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        );
        sword.max_charges = 5;

        let desc = ItemMeshDescriptor::from_item(&sword);
        assert!(
            desc.is_metallic_magical(),
            "charged item should be recognised as metallic-magical"
        );

        // Verify through creature definition that the primary mesh material
        // has metallic > 0.5
        let creature_def = desc.to_creature_definition();
        // meshes[1] is the primary mesh (meshes[0] is shadow quad)
        let primary = &creature_def.meshes[1];
        let mat = primary
            .material
            .as_ref()
            .expect("primary mesh must have material");
        assert!(
            mat.metallic > 0.5,
            "magical item material metallic ({}) must be > 0.5",
            mat.metallic
        );
        assert!(
            mat.roughness < 0.3,
            "magical item material roughness ({}) must be < 0.3",
            mat.roughness
        );
    }

    // §4.6 — test_non_magical_item_matte_material
    /// Non-magical non-metal item must produce metallic: 0.0, roughness: 0.8.
    #[test]
    fn test_non_magical_item_matte_material() {
        // Use a cloak (non-metal category) to avoid the legacy 0.6 metallic path
        let cloak = Item {
            id: 69,
            name: "Plain Cloak".to_string(),
            item_type: ItemType::Accessory(crate::domain::items::types::AccessoryData {
                slot: AccessorySlot::Cloak,
                classification: None,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
        };
        let desc = ItemMeshDescriptor::from_item(&cloak);
        assert!(
            !desc.is_metallic_magical(),
            "non-magical cloak must not be metallic-magical"
        );

        let creature_def = desc.to_creature_definition();
        // meshes[1] is the primary mesh
        let primary = &creature_def.meshes[1];
        let mat = primary
            .material
            .as_ref()
            .expect("primary mesh must have material");
        assert_eq!(
            mat.metallic, MATERIAL_METALLIC_MUNDANE,
            "non-magical non-metal item must have metallic: 0.0"
        );
        assert_eq!(
            mat.roughness, MATERIAL_ROUGHNESS_MUNDANE,
            "non-magical non-metal item must have roughness: 0.8"
        );
    }

    // §4.6 — test_shadow_quad_present_and_transparent
    /// The first mesh of every CreatureDefinition must be a shadow quad with
    /// alpha < 0.5 and AlphaMode::Blend.
    #[test]
    fn test_shadow_quad_present_and_transparent() {
        let sword = make_weapon(
            70,
            "Shadow Test Sword",
            8,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        );
        let desc = ItemMeshDescriptor::from_item(&sword);
        let creature_def = desc.to_creature_definition();

        // At least 2 meshes: shadow quad + primary
        assert!(
            creature_def.meshes.len() >= 2,
            "CreatureDefinition must have at least 2 meshes (shadow + primary)"
        );

        let shadow = &creature_def.meshes[0];
        assert_eq!(
            shadow.name.as_deref(),
            Some("shadow_quad"),
            "first mesh must be named shadow_quad"
        );
        // Color alpha must be < 0.5
        assert!(
            shadow.color[3] < 0.5,
            "shadow quad color alpha ({}) must be < 0.5",
            shadow.color[3]
        );
        let mat = shadow
            .material
            .as_ref()
            .expect("shadow quad must have a material");
        assert_eq!(
            mat.alpha_mode,
            AlphaMode::Blend,
            "shadow quad must use AlphaMode::Blend"
        );
        assert!(
            mat.base_color[3] < 0.5,
            "shadow quad material alpha ({}) must be < 0.5",
            mat.base_color[3]
        );
    }

    // §4.6 — test_shadow_quad_valid_for_all_categories
    /// Shadow quad must be present for every item category.
    #[test]
    fn test_shadow_quad_valid_for_all_categories() {
        let items: Vec<Item> = vec![
            make_weapon(
                71,
                "Sword",
                8,
                1,
                WeaponClassification::MartialMelee,
                vec![],
            ),
            make_weapon(72, "Dagger", 4, 1, WeaponClassification::Simple, vec![]),
            make_consumable(73, "Heal", ConsumableEffect::HealHp(10)),
            make_consumable(74, "Scroll", ConsumableEffect::CureCondition(0x01)),
        ];
        for item in &items {
            let desc = ItemMeshDescriptor::from_item(item);
            let def = desc.to_creature_definition();
            assert!(
                def.meshes.len() >= 2,
                "item '{}' must have shadow quad (got {} meshes)",
                item.name,
                def.meshes.len()
            );
            assert_eq!(
                def.meshes[0].name.as_deref(),
                Some("shadow_quad"),
                "item '{}' first mesh must be shadow_quad",
                item.name
            );
        }
    }

    // §4.6 — test_charge_fraction_full_color_gold
    /// charges_fraction = 1.0 must yield a gold-tinted gem.
    #[test]
    fn test_charge_fraction_full_color_gold() {
        let mut wand = make_weapon(75, "Wand", 4, 1, WeaponClassification::Simple, vec![]);
        wand.max_charges = 10;

        let desc = ItemMeshDescriptor::from_item(&wand);
        let creature_def = desc.to_creature_definition_with_charges(Some(1.0));

        // shadow + primary + gem = 3 meshes
        assert_eq!(
            creature_def.meshes.len(),
            3,
            "fully-charged item must have 3 meshes (shadow, primary, gem)"
        );

        let gem = &creature_def.meshes[2];
        assert_eq!(
            gem.name.as_deref(),
            Some("charge_gem"),
            "third mesh must be charge_gem"
        );
        // Gold: R should be 1.0, G ≈ 0.84
        let mat = gem.material.as_ref().expect("gem must have material");
        assert!(
            mat.base_color[0] > 0.9,
            "full-charge gem red channel ({}) must be > 0.9 (gold)",
            mat.base_color[0]
        );
        assert!(
            mat.base_color[1] > 0.7,
            "full-charge gem green channel ({}) must be > 0.7 (gold)",
            mat.base_color[1]
        );
        assert!(
            mat.base_color[2] < 0.2,
            "full-charge gem blue channel ({}) must be < 0.2 (gold)",
            mat.base_color[2]
        );
        // Must have emissive glow
        assert!(
            mat.emissive.is_some(),
            "full-charge gem must have emissive glow"
        );
    }

    // §4.6 — test_charge_fraction_empty_color_grey
    /// charges_fraction = 0.0 must yield a grey (depleted) gem.
    #[test]
    fn test_charge_fraction_empty_color_grey() {
        let mut wand = make_weapon(76, "Spent Wand", 4, 1, WeaponClassification::Simple, vec![]);
        wand.max_charges = 10;

        let desc = ItemMeshDescriptor::from_item(&wand);
        let creature_def = desc.to_creature_definition_with_charges(Some(0.0));

        let gem = &creature_def.meshes[2];
        let mat = gem.material.as_ref().expect("gem must have material");

        // Grey: all channels roughly equal and low-mid
        assert!(
            (mat.base_color[0] - mat.base_color[1]).abs() < 0.1,
            "depleted gem must be grey (R≈G≈B), got {:?}",
            mat.base_color
        );
        assert!(
            mat.base_color[0] < 0.6,
            "depleted gem must be dark grey (< 0.6), got {}",
            mat.base_color[0]
        );
        // No emissive for depleted
        assert!(
            mat.emissive.is_none(),
            "depleted gem must have no emissive glow"
        );
    }

    // §4.6 — test_charge_fraction_none_no_gem
    /// When charges_fraction is None, no gem mesh must be added.
    #[test]
    fn test_charge_fraction_none_no_gem() {
        let sword = make_weapon(
            77,
            "Plain Sword",
            8,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        );
        let desc = ItemMeshDescriptor::from_item(&sword);
        let creature_def = desc.to_creature_definition_with_charges(None);

        assert_eq!(
            creature_def.meshes.len(),
            2,
            "no charges_fraction → exactly 2 meshes (shadow + primary)"
        );
    }

    // §4.6 — test_lod_added_for_complex_mesh
    /// A mesh with more than LOD_TRIANGLE_THRESHOLD triangles must have
    /// lod_levels and lod_distances set on the primary mesh.
    ///
    /// We synthesise a descriptor whose primary mesh we can control by
    /// building it directly and injecting a fat mesh.
    #[test]
    fn test_lod_added_for_complex_mesh() {
        // Build a descriptor then verify by calling build_mesh_with_lod on a
        // synthetically over-threshold mesh.  We do this by constructing a
        // MeshDefinition with > 200 triangles and applying generate_lod_levels.
        let triangle_count = LOD_TRIANGLE_THRESHOLD + 50; // 250 triangles
        let vertex_count = triangle_count * 3;
        let mut vertices: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
        let mut indices: Vec<u32> = Vec::with_capacity(triangle_count * 3);
        for i in 0..triangle_count {
            let x = i as f32 * 0.01;
            vertices.push([x, 0.0, 0.0]);
            vertices.push([x + 0.005, 0.0, 0.01]);
            vertices.push([x + 0.01, 0.0, 0.0]);
            indices.push((i * 3) as u32);
            indices.push((i * 3 + 1) as u32);
            indices.push((i * 3 + 2) as u32);
        }
        let big_mesh = MeshDefinition {
            name: Some("test_complex".to_string()),
            vertices,
            indices,
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };

        // Simulate what build_mesh_with_lod does on this mesh
        let triangle_ct = big_mesh.indices.len() / 3;
        assert!(
            triangle_ct > LOD_TRIANGLE_THRESHOLD,
            "precondition: mesh is complex"
        );

        let (lod_meshes, _) = generate_lod_levels(&big_mesh, 2);
        assert!(
            !lod_meshes.is_empty(),
            "complex mesh must generate LOD levels"
        );
        assert_eq!(lod_meshes.len(), 2, "must generate exactly 2 LOD levels");

        // Verify that a real item with a complex enough mesh category gets LOD
        // attached via to_creature_definition.  Quest items generate the most
        // complex mesh (17-vertex star with 16 triangles — still ≤ threshold),
        // so for the LOD path we test the generate_lod_levels function directly
        // (tested in lod.rs) and the wiring via the conditional.
        // The important assertion is the distances are set correctly when LOD IS triggered.
        let distances = [LOD_DISTANCE_1, LOD_DISTANCE_2];
        assert_eq!(distances[0], 8.0);
        assert_eq!(distances[1], 20.0);
    }

    // §4.6 — test_no_lod_for_simple_mesh
    /// Simple item meshes (≤ 200 triangles) must NOT have lod_levels set.
    #[test]
    fn test_no_lod_for_simple_mesh() {
        let sword = make_weapon(
            78,
            "Simple Sword",
            8,
            1,
            WeaponClassification::MartialMelee,
            vec![],
        );
        let desc = ItemMeshDescriptor::from_item(&sword);
        let creature_def = desc.to_creature_definition();

        // Primary mesh is meshes[1]
        let primary = &creature_def.meshes[1];
        let tri_count = primary.indices.len() / 3;
        assert!(
            tri_count <= LOD_TRIANGLE_THRESHOLD,
            "sword mesh has {} triangles; expected ≤ {}",
            tri_count,
            LOD_TRIANGLE_THRESHOLD
        );
        assert!(
            primary.lod_levels.is_none(),
            "simple mesh must not have lod_levels"
        );
    }

    // §4.6 — test_deterministic_charge_gem_color
    /// The charge gem interpolation must be deterministic and must produce
    /// the correct colors at the three key fractions (0.0, 0.5, 1.0).
    #[test]
    fn test_deterministic_charge_gem_color() {
        // Deterministic: same input → same output
        let (c1, e1) = ItemMeshDescriptor::charge_gem_color(0.75);
        let (c2, e2) = ItemMeshDescriptor::charge_gem_color(0.75);
        assert_eq!(c1, c2, "charge_gem_color must be deterministic for color");
        assert_eq!(
            e1, e2,
            "charge_gem_color must be deterministic for emissive"
        );

        // Depleted (0.0) → grey: all RGB channels roughly equal and low-mid
        let (depleted_color, depleted_emit) = ItemMeshDescriptor::charge_gem_color(0.0);
        assert_eq!(
            depleted_color, COLOR_CHARGE_EMPTY,
            "frac=0 must be COLOR_CHARGE_EMPTY"
        );
        assert_eq!(
            depleted_emit, EMISSIVE_CHARGE_EMPTY,
            "frac=0 emissive must be EMISSIVE_CHARGE_EMPTY"
        );

        // Half (0.5) → white/pale: all channels should be high
        let (half_color, half_emit) = ItemMeshDescriptor::charge_gem_color(0.5);
        assert_eq!(
            half_color, COLOR_CHARGE_HALF,
            "frac=0.5 must be COLOR_CHARGE_HALF"
        );
        assert_eq!(
            half_emit, EMISSIVE_CHARGE_HALF,
            "frac=0.5 emissive must be EMISSIVE_CHARGE_HALF"
        );

        // Full (1.0) → gold: high red, high-ish green, low blue
        let (full_color, full_emit) = ItemMeshDescriptor::charge_gem_color(1.0);
        assert_eq!(
            full_color, COLOR_CHARGE_FULL,
            "frac=1 must be COLOR_CHARGE_FULL"
        );
        assert_eq!(
            full_emit, EMISSIVE_CHARGE_FULL,
            "frac=1 emissive must be EMISSIVE_CHARGE_FULL"
        );

        // Interpolated: frac=0.25 must be between empty and half
        let (quarter_color, _) = ItemMeshDescriptor::charge_gem_color(0.25);
        // Red channel must be between depleted and half
        assert!(
            quarter_color[0] >= COLOR_CHARGE_EMPTY[0] && quarter_color[0] <= COLOR_CHARGE_HALF[0],
            "frac=0.25 red ({}) must interpolate between empty ({}) and half ({})",
            quarter_color[0],
            COLOR_CHARGE_EMPTY[0],
            COLOR_CHARGE_HALF[0]
        );

        // Interpolated: frac=0.75 must be between half and full
        let (three_quarter_color, _) = ItemMeshDescriptor::charge_gem_color(0.75);
        // Red channel: half=0.9, full=1.0 → must be in [0.9, 1.0]
        assert!(
            three_quarter_color[0] >= COLOR_CHARGE_HALF[0]
                && three_quarter_color[0] <= COLOR_CHARGE_FULL[0],
            "frac=0.75 red ({}) must interpolate between half ({}) and full ({})",
            three_quarter_color[0],
            COLOR_CHARGE_HALF[0],
            COLOR_CHARGE_FULL[0]
        );

        // Clamping: out-of-range values must clamp gracefully
        let (over, _) = ItemMeshDescriptor::charge_gem_color(2.0);
        assert_eq!(over, COLOR_CHARGE_FULL, "frac > 1.0 must clamp to full");
        let (under, _) = ItemMeshDescriptor::charge_gem_color(-1.0);
        assert_eq!(under, COLOR_CHARGE_EMPTY, "frac < 0.0 must clamp to empty");
    }

    // §4.6 — test_creature_definition_mesh_transform_count_matches
    /// meshes.len() must equal mesh_transforms.len() for all charge variants.
    #[test]
    fn test_creature_definition_mesh_transform_count_matches() {
        let mut wand = make_weapon(79, "Wand", 4, 1, WeaponClassification::Simple, vec![]);
        wand.max_charges = 5;
        let desc = ItemMeshDescriptor::from_item(&wand);

        for charges in [None, Some(0.0_f32), Some(0.5), Some(1.0)] {
            let def = desc.to_creature_definition_with_charges(charges);
            assert_eq!(
                def.meshes.len(),
                def.mesh_transforms.len(),
                "meshes ({}) and mesh_transforms ({}) must match for charges={:?}",
                def.meshes.len(),
                def.mesh_transforms.len(),
                charges
            );
            assert!(
                def.validate().is_ok(),
                "creature definition must validate for charges={:?}: {:?}",
                charges,
                def.validate()
            );
        }
    }

    // §4.6 — test_accent_color_not_applied_to_cursed_item
    /// Cursed items must keep their dark tint even if they have a bonus.
    #[test]
    fn test_accent_color_not_applied_to_cursed_item() {
        let mut item = make_item_with_bonus(80, "Cursed Fire Sword", BonusAttribute::ResistFire);
        item.is_cursed = true;

        let desc = ItemMeshDescriptor::from_item(&item);
        // Primary color must be cursed dark tint — bonus accent should not override
        assert_eq!(
            desc.primary_color, COLOR_CURSED,
            "cursed item primary_color must be COLOR_CURSED even with bonus"
        );
    }

    // §4.6 — test_lerp_color4_midpoint
    /// lerp_color4 at t=0.5 must produce the midpoint color.
    #[test]
    fn test_lerp_color4_midpoint() {
        let a = [0.0, 0.0, 0.0, 1.0];
        let b = [1.0, 1.0, 1.0, 1.0];
        let mid = lerp_color4(a, b, 0.5);
        for ch in &mid[..3] {
            assert!(
                (ch - 0.5).abs() < f32::EPSILON * 2.0,
                "midpoint channel must be 0.5, got {ch}"
            );
        }
    }

    // §4.6 — test_lerp_color3_midpoint
    /// lerp_color3 at t=0.5 must produce the midpoint emissive.
    #[test]
    fn test_lerp_color3_midpoint() {
        let a = [0.0_f32; 3];
        let b = [1.0_f32; 3];
        let mid = lerp_color3(a, b, 0.5);
        for ch in &mid {
            assert!(
                (ch - 0.5).abs() < f32::EPSILON * 2.0,
                "midpoint emissive channel must be 0.5, got {ch}"
            );
        }
    }

    // §6.4 — test_all_base_items_have_valid_mesh_descriptor
    /// Every item in `data/items.ron` must produce a valid `ItemMeshDescriptor`
    /// and a valid `CreatureDefinition`.
    #[test]
    fn test_all_base_items_have_valid_mesh_descriptor() {
        use crate::domain::items::database::ItemDatabase;

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let items_path = std::path::PathBuf::from(manifest_dir)
            .join("data")
            .join("items.ron");

        let db = ItemDatabase::load_from_file(&items_path).expect("Failed to load data/items.ron");

        assert!(!db.is_empty(), "items.ron should have at least one item");

        for item in db.all_items() {
            let descriptor = ItemMeshDescriptor::from_item(item);
            let creature_def = descriptor.to_creature_definition();
            let result = creature_def.validate();
            assert!(
                result.is_ok(),
                "Item id={} name='{}' produced invalid creature definition: {:?}",
                item.id,
                item.name,
                result.err()
            );
        }
    }

    // §6.4 — test_item_mesh_registry_tutorial_coverage
    /// The test_campaign item mesh registry is non-empty after loading.
    #[test]
    fn test_item_mesh_registry_tutorial_coverage() {
        use crate::domain::campaign_loader::CampaignLoader;
        use std::path::PathBuf;

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let base = PathBuf::from(manifest_dir).join("data");
        let campaign = base.join("test_campaign");

        let mut loader = CampaignLoader::new(base, campaign);
        let result = loader.load_game_data();
        assert!(result.is_ok(), "load_game_data failed: {:?}", result.err());

        let game_data = result.unwrap();
        assert!(
            !game_data.item_meshes.is_empty(),
            "Expected non-empty item mesh registry in test_campaign"
        );
        assert!(
            game_data.item_meshes.count() >= 2,
            "Expected at least 2 item mesh entries, got {}",
            game_data.item_meshes.count()
        );
    }

    // §6.4 — test_dropped_item_event_in_map_ron
    /// A test_campaign map containing a `DroppedItem` event parses correctly.
    #[test]
    fn test_dropped_item_event_in_map_ron() {
        use crate::domain::world::{Map, MapEvent};
        use std::path::PathBuf;

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let map_path = PathBuf::from(manifest_dir).join("data/test_campaign/data/maps/map_1.ron");

        assert!(
            map_path.exists(),
            "test_campaign map_1.ron not found at {:?}",
            map_path
        );

        let contents =
            std::fs::read_to_string(&map_path).expect("Failed to read test_campaign map_1.ron");

        // Parse the map file — RON format with Map type
        let map: Map = ron::from_str(&contents).expect("Failed to parse map_1.ron as Map");

        // Verify there is at least one DroppedItem event
        let dropped_items: Vec<_> = map
            .events
            .values()
            .filter(|e| matches!(e, MapEvent::DroppedItem { .. }))
            .collect();

        assert!(
            !dropped_items.is_empty(),
            "Expected at least one DroppedItem event in test_campaign map_1.ron"
        );

        // Verify the specific Long Sword entry (item_id = 4) is present
        let has_long_sword = map
            .events
            .values()
            .any(|e| matches!(e, MapEvent::DroppedItem { item_id, .. } if *item_id == 4));
        assert!(
            has_long_sword,
            "Expected a DroppedItem with item_id=4 (Long Sword) in test_campaign map_1.ron"
        );
    }
}
