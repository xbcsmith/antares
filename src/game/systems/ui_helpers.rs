// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared UI helper functions and constants for Bevy UI text styling and image
//! creation.
//!
//! These helpers reduce boilerplate across combat, HUD, menu, rest, and game
//! log systems where identical [`TextFont`] / [`TextColor`] patterns are
//! repeated many times.
//!
//! # Text Style Helper
//!
//! The [`text_style`] function returns a `(TextFont, TextColor)` tuple that
//! can be placed directly inside a Bevy `spawn((...))` call alongside other
//! components.  Bevy accepts nested tuples as bundles, so the returned pair
//! merges seamlessly:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use antares::game::systems::ui_helpers::{text_style, BODY_FONT_SIZE};
//! # fn example(mut commands: Commands) {
//! commands.spawn((
//!     Text::new("Hello"),
//!     text_style(BODY_FONT_SIZE, Color::WHITE),
//! ));
//! # }
//! ```
//!
//! # Image Helper
//!
//! [`create_blank_rgba_image`] produces a square, fully-transparent RGBA8
//! texture suitable for the mini-map and automap backing images.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Standard body-text font size (16 px).
///
/// Used across settings labels, combat victory/defeat summaries, turn-order
/// text, and other general-purpose UI text.
pub const BODY_FONT_SIZE: f32 = 16.0;

/// Standard label / small-text font size (14 px).
///
/// Used in automap legend entries, combat enemy-name cards, action-button
/// labels, and the game-log header.
pub const LABEL_FONT_SIZE: f32 = 14.0;

/// Creates a ([`TextFont`], [`TextColor`]) bundle pair with the given size and
/// color.
///
/// Because Bevy bundles accept nested tuples, the returned pair can be placed
/// directly inside a `spawn((...))` call alongside other components:
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use antares::game::systems::ui_helpers::text_style;
/// # fn example(mut commands: Commands) {
/// commands.spawn((
///     Text::new("hello"),
///     text_style(16.0, Color::WHITE),
/// ));
/// # }
/// ```
///
/// # Arguments
///
/// * `font_size` — Font size in logical pixels.
/// * `color`     — Text color applied via [`TextColor`].
///
/// # Returns
///
/// A `(TextFont, TextColor)` tuple ready for insertion into an entity.
pub fn text_style(font_size: f32, color: Color) -> (TextFont, TextColor) {
    (
        TextFont {
            font_size,
            ..default()
        },
        TextColor(color),
    )
}

/// Creates a ([`TextFont`], [`TextColor`]) bundle pair, optionally applying a
/// custom font handle.
///
/// When `font` is `Some`, the returned [`TextFont`] uses that specific font
/// asset.  When `None`, the Bevy engine default font is used, matching the
/// behavior of [`text_style`].
///
/// # Arguments
///
/// * `font`      — Custom font handle. `None` uses the Bevy engine default.
/// * `font_size` — Font size in logical pixels.
/// * `color`     — Text color.
///
/// # Examples
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use antares::game::systems::ui_helpers::text_style_with_font;
/// # fn example(mut commands: Commands) {
/// commands.spawn((
///     Text::new("Hello"),
///     text_style_with_font(None, 16.0, Color::WHITE),
/// ));
/// # }
/// ```
pub fn text_style_with_font(
    font: Option<Handle<Font>>,
    font_size: f32,
    color: Color,
) -> (TextFont, TextColor) {
    let text_font = match font {
        Some(handle) => TextFont {
            font: handle,
            font_size,
            ..default()
        },
        None => TextFont {
            font_size,
            ..default()
        },
    };
    (text_font, TextColor(color))
}

/// Creates a square RGBA8 image filled with transparent black pixels.
///
/// The returned [`Image`] uses [`TextureFormat::Rgba8UnormSrgb`] and is
/// flagged for all render-asset usages so it can be written to by CPU-side
/// map-painting logic and simultaneously displayed by the GPU.
///
/// This is used by the mini-map and automap initialisation systems to create
/// the backing texture that is later painted by the map rendering logic.
///
/// # Arguments
///
/// * `size` — Width **and** height of the square image in pixels.
///
/// # Returns
///
/// A new [`Image`] of dimensions `size × size` with every pixel set to
/// `[0, 0, 0, 0]`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::ui_helpers::create_blank_rgba_image;
///
/// let img = create_blank_rgba_image(64);
/// assert_eq!(img.width(), 64);
/// assert_eq!(img.height(), 64);
/// let data = img.data.as_ref().expect("image data should be present");
/// assert_eq!(data.len(), 64 * 64 * 4);
/// ```
pub fn create_blank_rgba_image(size: u32) -> Image {
    Image::new_fill(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &vec![0u8; (size * size * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    )
}

// ===== Condition & Turn-State Card Color Scheme =====
//
// Single source of truth for condition-based card tinting, shared by the
// combat enemy cards (`src/game/systems/combat.rs`) and the HUD party cards
// (`src/game/systems/hud.rs`). See `docs/reference/condition_color_scheme.md`
// for the full reference table. All tints use alpha < 1.0 — translucent
// overlays only, never opaque, so the card's icon/text content stays legible.

/// Colour for fatal condition tints ("Dead") — transparent red.
pub const CONDITION_FATAL_COLOR: Color = Color::srgba(0.85, 0.2, 0.2, 0.85);

/// Colour for generic non-fatal condition tints (Paralyzed, Asleep, …) —
/// transparent yellow. The fallback used whenever a more specific tint
/// (poisoned, unconscious) does not apply.
pub const CONDITION_STATUS_COLOR: Color = Color::srgba(0.9, 0.85, 0.3, 0.85);

/// Colour for poisoned/diseased condition tints — transparent green.
pub const CONDITION_POISON_TINT_COLOR: Color = Color::srgba(0.2, 0.7, 0.2, 0.75);

/// Colour for unconscious condition tints — transparent grey. Players only;
/// monsters have no unconscious state (they go straight to `Dead`).
pub const CONDITION_UNCONSCIOUS_TINT_COLOR: Color = Color::srgba(0.5, 0.5, 0.5, 0.75);

/// Which condition category currently applies to a card's owner, used to
/// select a background tint.
///
/// Both HUD party cards (from `Character` condition bitflags) and combat
/// enemy cards (from `MonsterCondition`) reduce their richer, type-specific
/// condition state down to one of these variants before calling
/// [`resolve_card_background`], giving both card kinds one shared mapping to
/// colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CardConditionTint {
    /// No active condition — the card keeps its default background.
    #[default]
    None,
    /// Dead, or any other fatal state.
    Fatal,
    /// Poisoned or diseased.
    Poisoned,
    /// Unconscious (players only).
    Unconscious,
    /// Any other non-fatal status condition (paralyzed, asleep, blinded,
    /// silenced, held, webbed, mindless, afraid, …).
    Status,
}

impl CardConditionTint {
    /// Returns the translucent background tint for this condition category,
    /// or `None` when the card should keep its default background.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::ui_helpers::{CardConditionTint, CONDITION_FATAL_COLOR};
    ///
    /// assert_eq!(CardConditionTint::Fatal.color(), Some(CONDITION_FATAL_COLOR));
    /// assert_eq!(CardConditionTint::None.color(), None);
    /// ```
    pub fn color(self) -> Option<Color> {
        match self {
            CardConditionTint::None => None,
            CardConditionTint::Fatal => Some(CONDITION_FATAL_COLOR),
            CardConditionTint::Poisoned => Some(CONDITION_POISON_TINT_COLOR),
            CardConditionTint::Unconscious => Some(CONDITION_UNCONSCIOUS_TINT_COLOR),
            CardConditionTint::Status => Some(CONDITION_STATUS_COLOR),
        }
    }
}

/// Resolves a card's background colour from the universal precedence rule
/// used across all combat UI: **active-turn highlight > condition tint >
/// default background**. Pure and free of any Bevy `App`/ECS types, so it is
/// directly unit-testable without spinning up a world.
///
/// `active_turn_color` is passed in by the caller (rather than hardcoded)
/// because each card kind defines its own active-turn constant alongside its
/// other card colours (e.g. `hud::ACTIVE_TURN_CARD_COLOR`).
///
/// # Examples
///
/// ```
/// use antares::game::systems::ui_helpers::{
///     resolve_card_background, CardConditionTint, CONDITION_FATAL_COLOR,
/// };
/// use bevy::prelude::Color;
///
/// let default_color = Color::srgba(0.2, 0.2, 0.2, 0.7);
/// let active_color = Color::srgba(0.15, 0.45, 0.15, 0.7);
///
/// // Active turn wins even over a fatal condition.
/// assert_eq!(
///     resolve_card_background(true, active_color, CardConditionTint::Fatal, default_color),
///     active_color
/// );
///
/// // No active turn: condition tint wins over the default background.
/// assert_eq!(
///     resolve_card_background(false, active_color, CardConditionTint::Fatal, default_color),
///     CONDITION_FATAL_COLOR
/// );
///
/// // Neither active turn nor condition: keep the default background.
/// assert_eq!(
///     resolve_card_background(false, active_color, CardConditionTint::None, default_color),
///     default_color
/// );
/// ```
pub fn resolve_card_background(
    is_active_turn: bool,
    active_turn_color: Color,
    condition: CardConditionTint,
    default_color: Color,
) -> Color {
    if is_active_turn {
        active_turn_color
    } else if let Some(tint) = condition.color() {
        tint
    } else {
        default_color
    }
}

// ===== Party-Target Eligibility =====
//
// Single source of truth for "who may this beneficial action target?",
// shared by the combat party-target panel for both item use (from
// `ConsumableEffect`) and spell casting (from `SpellEffectType`). Pure and
// free of Bevy ECS types so it is unit-testable without a world.

/// Which party members a beneficial action (item or spell) may target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TargetEligibility {
    /// Any party member is a valid target.
    #[default]
    Any,
    /// Only living (non-dead) members — heals, cures, restores, boosts.
    LivingOnly,
    /// Only dead members — resurrection effects.
    DeadOnly,
}

impl TargetEligibility {
    /// Returns whether `character` is a valid target under this rule.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::character::{Alignment, Character, Condition, Sex};
    /// use antares::game::systems::ui_helpers::TargetEligibility;
    ///
    /// let mut hero = Character::new(
    ///     "Kira".to_string(),
    ///     "human".to_string(),
    ///     "knight".to_string(),
    ///     Sex::Female,
    ///     Alignment::Good,
    /// );
    /// assert!(TargetEligibility::LivingOnly.is_eligible(&hero));
    /// assert!(!TargetEligibility::DeadOnly.is_eligible(&hero));
    ///
    /// hero.conditions.add(Condition::DEAD);
    /// assert!(!TargetEligibility::LivingOnly.is_eligible(&hero));
    /// assert!(TargetEligibility::DeadOnly.is_eligible(&hero));
    /// assert!(TargetEligibility::Any.is_eligible(&hero));
    /// ```
    pub fn is_eligible(self, character: &crate::domain::character::Character) -> bool {
        match self {
            TargetEligibility::Any => true,
            TargetEligibility::LivingOnly => !character.conditions.is_dead(),
            TargetEligibility::DeadOnly => character.conditions.is_dead(),
        }
    }
}

/// Derives the party-target eligibility rule for a consumable effect.
///
/// `Resurrect` may only target dead members; every other beneficial effect
/// (heal, restore, cure, boost) only living ones. Effects that never reach
/// the party-target panel (`IsFood`, `CastSpell`, `LearnSpell`) return
/// [`TargetEligibility::Any`].
///
/// # Examples
///
/// ```
/// use antares::domain::items::types::ConsumableEffect;
/// use antares::game::systems::ui_helpers::{
///     consumable_target_eligibility, TargetEligibility,
/// };
///
/// assert_eq!(
///     consumable_target_eligibility(&ConsumableEffect::HealHp(20)),
///     TargetEligibility::LivingOnly
/// );
/// assert_eq!(
///     consumable_target_eligibility(&ConsumableEffect::Resurrect(10)),
///     TargetEligibility::DeadOnly
/// );
/// ```
pub fn consumable_target_eligibility(
    effect: &crate::domain::items::types::ConsumableEffect,
) -> TargetEligibility {
    use crate::domain::items::types::ConsumableEffect;
    match effect {
        ConsumableEffect::Resurrect(_) => TargetEligibility::DeadOnly,
        ConsumableEffect::HealHp(_)
        | ConsumableEffect::RestoreSp(_)
        | ConsumableEffect::CureCondition(_)
        | ConsumableEffect::BoostAttribute(_, _)
        | ConsumableEffect::BoostResistance(_, _) => TargetEligibility::LivingOnly,
        ConsumableEffect::IsFood(_)
        | ConsumableEffect::CastSpell(_)
        | ConsumableEffect::LearnSpell(_) => TargetEligibility::Any,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_style_returns_correct_font_size() {
        let (font, _color) = text_style(20.0, Color::WHITE);
        assert!((font.font_size - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_text_style_returns_correct_color() {
        let (_font, color) = text_style(14.0, Color::WHITE);
        assert_eq!(color.0, Color::WHITE);
    }

    #[test]
    fn test_text_style_with_font_none_font_size_correct() {
        let (font, _color) = text_style_with_font(None, 20.0, Color::WHITE);
        assert!((font.font_size - 20.0).abs() < f32::EPSILON);
    }

    // ── Condition & turn-state card color scheme ────────────────────────────

    /// Active-turn highlight wins over every condition tint, per the
    /// documented precedence rule.
    #[test]
    fn test_resolve_card_background_active_turn_wins_over_condition() {
        let default_color = Color::srgba(0.2, 0.2, 0.2, 0.7);
        let active_color = Color::srgba(0.15, 0.45, 0.15, 0.7);
        for condition in [
            CardConditionTint::None,
            CardConditionTint::Fatal,
            CardConditionTint::Poisoned,
            CardConditionTint::Unconscious,
            CardConditionTint::Status,
        ] {
            assert_eq!(
                resolve_card_background(true, active_color, condition, default_color),
                active_color,
                "active turn must win over {:?}",
                condition
            );
        }
    }

    /// Without an active turn, a condition tint wins over the default
    /// background.
    #[test]
    fn test_resolve_card_background_condition_wins_over_default() {
        let default_color = Color::srgba(0.2, 0.2, 0.2, 0.7);
        let active_color = Color::srgba(0.15, 0.45, 0.15, 0.7);
        let cases = [
            (CardConditionTint::Fatal, CONDITION_FATAL_COLOR),
            (CardConditionTint::Poisoned, CONDITION_POISON_TINT_COLOR),
            (
                CardConditionTint::Unconscious,
                CONDITION_UNCONSCIOUS_TINT_COLOR,
            ),
            (CardConditionTint::Status, CONDITION_STATUS_COLOR),
        ];
        for (condition, expected) in cases {
            assert_eq!(
                resolve_card_background(false, active_color, condition, default_color),
                expected
            );
        }
    }

    /// With no active turn and no condition, the card keeps its default
    /// background.
    #[test]
    fn test_resolve_card_background_falls_back_to_default() {
        let default_color = Color::srgba(0.2, 0.2, 0.2, 0.7);
        let active_color = Color::srgba(0.15, 0.45, 0.15, 0.7);
        assert_eq!(
            resolve_card_background(false, active_color, CardConditionTint::None, default_color),
            default_color
        );
    }

    /// Every condition tint constant must be translucent (alpha < 1.0) — the
    /// plan mandates translucent overlays only, never opaque high-saturation
    /// colours.
    #[test]
    fn test_condition_tint_colors_are_translucent() {
        for color in [
            CONDITION_FATAL_COLOR,
            CONDITION_STATUS_COLOR,
            CONDITION_POISON_TINT_COLOR,
            CONDITION_UNCONSCIOUS_TINT_COLOR,
        ] {
            let srgba = color.to_srgba();
            assert!(
                srgba.alpha < 1.0,
                "condition tint {:?} must have alpha < 1.0",
                color
            );
        }
    }

    // ── Party-target eligibility ────────────────────────────────────────────

    fn test_character() -> crate::domain::character::Character {
        use crate::domain::character::{Alignment, Character, Sex};
        Character::new(
            "Test".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Neutral,
        )
    }

    /// Every beneficial non-resurrect consumable targets living members only;
    /// resurrect targets dead members only; pass-through effects allow any.
    #[test]
    fn test_consumable_target_eligibility_mapping() {
        use crate::domain::items::types::{AttributeType, ConsumableEffect, ResistanceType};
        let cases = [
            (ConsumableEffect::HealHp(20), TargetEligibility::LivingOnly),
            (
                ConsumableEffect::RestoreSp(10),
                TargetEligibility::LivingOnly,
            ),
            (
                ConsumableEffect::CureCondition(4),
                TargetEligibility::LivingOnly,
            ),
            (
                ConsumableEffect::BoostAttribute(AttributeType::Might, 2),
                TargetEligibility::LivingOnly,
            ),
            (
                ConsumableEffect::BoostResistance(ResistanceType::Fire, 10),
                TargetEligibility::LivingOnly,
            ),
            (ConsumableEffect::Resurrect(10), TargetEligibility::DeadOnly),
            (ConsumableEffect::IsFood(1), TargetEligibility::Any),
            (ConsumableEffect::CastSpell(260), TargetEligibility::Any),
            (ConsumableEffect::LearnSpell(260), TargetEligibility::Any),
        ];
        for (effect, expected) in cases {
            assert_eq!(
                consumable_target_eligibility(&effect),
                expected,
                "wrong eligibility for {:?}",
                effect
            );
        }
    }

    /// A living character is eligible for `LivingOnly`/`Any` but not
    /// `DeadOnly`; a dead character is the reverse.
    #[test]
    fn test_target_eligibility_is_eligible() {
        use crate::domain::character::Condition;
        let mut ch = test_character();
        assert!(TargetEligibility::Any.is_eligible(&ch));
        assert!(TargetEligibility::LivingOnly.is_eligible(&ch));
        assert!(!TargetEligibility::DeadOnly.is_eligible(&ch));

        ch.conditions.add(Condition::DEAD);
        assert!(TargetEligibility::Any.is_eligible(&ch));
        assert!(!TargetEligibility::LivingOnly.is_eligible(&ch));
        assert!(TargetEligibility::DeadOnly.is_eligible(&ch));
    }

    #[test]
    fn test_text_style_with_font_none_color_correct() {
        let (_font, color) = text_style_with_font(None, 16.0, Color::srgb(1.0, 0.0, 0.0));
        assert_eq!(color.0, Color::srgb(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_text_style_with_font_some_sets_handle() {
        use bevy::text::Font;
        let handle: Handle<Font> = Handle::default();
        let (text_font, _color) = text_style_with_font(Some(handle.clone()), 16.0, Color::WHITE);
        assert_eq!(text_font.font, handle);
    }

    #[test]
    fn test_body_font_size_value() {
        assert!((BODY_FONT_SIZE - 16.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_label_font_size_value() {
        assert!((LABEL_FONT_SIZE - 14.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_create_blank_rgba_image_dimensions() {
        let img = create_blank_rgba_image(32);
        assert_eq!(img.width(), 32);
        assert_eq!(img.height(), 32);
    }

    #[test]
    fn test_create_blank_rgba_image_data_length() {
        let img = create_blank_rgba_image(16);
        let data = img.data.as_ref().expect("image data should be present");
        // 16 * 16 pixels * 4 bytes (RGBA)
        assert_eq!(data.len(), 16 * 16 * 4);
    }

    #[test]
    fn test_create_blank_rgba_image_all_zeros() {
        let img = create_blank_rgba_image(8);
        let data = img.data.as_ref().expect("image data should be present");
        assert!(data.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_create_blank_rgba_image_size_one() {
        let img = create_blank_rgba_image(1);
        let data = img.data.as_ref().expect("image data should be present");
        assert_eq!(data.len(), 4);
        assert_eq!(data.as_slice(), &[0u8, 0, 0, 0]);
    }
}
