// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Central boundary error type for the Antares runtime.
//!
//! Antares defines one error enum per module (combat, magic, world, items,
//! campaign loading, configuration, …). Individual domain and application
//! functions keep returning their own precise error type, but Bevy systems sit
//! at the top of the call graph and routinely call into several of those
//! modules at once. Without a shared aggregate, every system boundary needs a
//! bespoke `match`/`map_err` ladder to funnel the module errors into a single
//! reportable value.
//!
//! [`GameError`] is that shared aggregate. It gathers the crate's module error
//! types through `#[from]` conversions, so a system can write
//! `foo().map_err(GameError::from)?` (or rely on `?` directly) and obtain a
//! uniform value to log. Because it lives at the crate root it can aggregate
//! errors from every layer — domain, application, game, and the SDK content
//! toolchain whose loaders/config parsers are consumed by the Bevy startup
//! systems — without any layer having to depend on a higher one.
//!
//! Bevy systems cannot return `Result`, so the [`report_err!`] macro pairs with
//! [`GameError`]: it routes an error into both the on-screen game log (via a
//! [`GameLogEvent`](crate::game::systems::ui::GameLogEvent)) and `tracing`,
//! giving every system one consistent way to surface a failure.

use thiserror::Error;

/// Aggregate error type consumed at the Bevy system boundary.
///
/// Each variant wraps one module error via `#[from]` and is
/// `#[error(transparent)]`, so a `GameError`'s `Display`/`source` delegate to
/// the underlying module error — no information is lost and the original
/// message is preserved verbatim.
///
/// # Examples
///
/// ```
/// use antares::error::GameError;
/// use antares::domain::types::DiceRollError;
///
/// // A module error converts into `GameError` for free via `?`/`From`.
/// fn parse() -> Result<(), DiceRollError> {
///     Err(DiceRollError::ZeroSides)
/// }
///
/// fn boundary() -> Result<(), GameError> {
///     parse()?; // DiceRollError -> GameError
///     Ok(())
/// }
///
/// let err = boundary().unwrap_err();
/// // Display is transparent: it matches the underlying error's message.
/// assert_eq!(err.to_string(), DiceRollError::ZeroSides.to_string());
/// ```
#[derive(Error, Debug)]
pub enum GameError {
    // ---- application layer ----
    /// Roster initialization failed while starting a new game.
    #[error(transparent)]
    RosterInitialization(#[from] crate::application::RosterInitializationError),
    /// A party-move request could not be handled.
    #[error(transparent)]
    MoveHandle(#[from] crate::application::MoveHandleError),
    /// Character recruitment failed.
    #[error(transparent)]
    Recruitment(#[from] crate::application::RecruitmentError),
    /// An NPC training service failed.
    #[error(transparent)]
    Training(#[from] crate::application::resources::TrainingError),
    /// Saving or loading a game failed.
    #[error(transparent)]
    SaveGame(#[from] crate::application::save_game::SaveGameError),
    /// Skill training failed.
    #[error(transparent)]
    SkillTraining(#[from] crate::application::skill_training::SkillTrainingError),

    // ---- domain layer ----
    /// Loading or resolving a campaign (domain view) failed.
    #[error(transparent)]
    Campaign(#[from] crate::domain::campaign_loader::CampaignError),
    /// A character definition was invalid.
    #[error(transparent)]
    CharacterDefinition(#[from] crate::domain::character_definition::CharacterDefinitionError),
    /// A character operation failed.
    #[error(transparent)]
    Character(#[from] crate::domain::character::CharacterError),
    /// A class lookup or operation failed.
    #[error(transparent)]
    Class(#[from] crate::domain::classes::ClassError),
    /// A monster database operation failed.
    #[error(transparent)]
    MonsterDatabase(#[from] crate::domain::combat::database::MonsterDatabaseError),
    /// Applying a combat condition failed.
    #[error(transparent)]
    ConditionApply(#[from] crate::domain::combat::engine::ConditionApplyError),
    /// A combat action failed.
    #[error(transparent)]
    Combat(#[from] crate::domain::combat::engine::CombatError),
    /// Using an item in combat failed.
    #[error(transparent)]
    ItemUse(#[from] crate::domain::combat::item_usage::ItemUseError),
    /// Casting a spell failed.
    #[error(transparent)]
    SpellCast(#[from] crate::domain::combat::spell_casting::SpellCastError),
    /// An item database operation failed.
    #[error(transparent)]
    ItemDatabase(#[from] crate::domain::items::database::ItemDatabaseError),
    /// Equipping an item failed a restriction check.
    #[error(transparent)]
    Equip(#[from] crate::domain::items::equipment_validation::EquipError),
    /// A level table lookup or operation failed.
    #[error(transparent)]
    Level(#[from] crate::domain::levels::LevelError),
    /// A spell database operation failed.
    #[error(transparent)]
    SpellDatabase(#[from] crate::domain::magic::database::SpellDatabaseError),
    /// Learning a spell failed.
    #[error(transparent)]
    SpellLearn(#[from] crate::domain::magic::learning::SpellLearnError),
    /// A spell definition or lookup failed.
    #[error(transparent)]
    Spell(#[from] crate::domain::magic::types::SpellError),
    /// A party-management operation failed.
    #[error(transparent)]
    PartyManagement(#[from] crate::domain::party_manager::PartyManagementError),
    /// A path failed a security/containment check.
    #[error(transparent)]
    PathSecurity(#[from] crate::domain::path_security::PathSecurityError),
    /// A proficiency operation failed.
    #[error(transparent)]
    Proficiency(#[from] crate::domain::proficiency::ProficiencyError),
    /// A progression/level-up operation failed.
    #[error(transparent)]
    Progression(#[from] crate::domain::progression::ProgressionError),
    /// A race lookup or operation failed.
    #[error(transparent)]
    Race(#[from] crate::domain::races::RaceError),
    /// A party-resource operation failed (gold, gems, food, …).
    #[error(transparent)]
    Resource(#[from] crate::domain::resources::ResourceError),
    /// A skill check failed structurally (not a normal miss).
    #[error(transparent)]
    SkillCheck(#[from] crate::domain::skill_checks::SkillCheckError),
    /// A skill lookup or operation failed.
    #[error(transparent)]
    Skill(#[from] crate::domain::skills::SkillError),
    /// A party transaction (buy/sell/pool) failed.
    #[error(transparent)]
    Transaction(#[from] crate::domain::transactions::TransactionError),
    /// A dice-roll specification was invalid.
    #[error(transparent)]
    DiceRoll(#[from] crate::domain::types::DiceRollError),
    /// Domain-level validation of data or preconditions failed.
    #[error(transparent)]
    Validation(#[from] crate::domain::validation::ValidationError),
    /// A creature-visual database operation failed.
    #[error(transparent)]
    CreatureDatabase(#[from] crate::domain::visual::creature_database::CreatureDatabaseError),
    /// A world-event operation failed.
    #[error(transparent)]
    Event(#[from] crate::domain::world::EventError),
    /// A furniture database operation failed.
    #[error(transparent)]
    FurnitureDatabase(#[from] crate::domain::world::furniture::FurnitureDatabaseError),
    /// A landscape database operation failed.
    #[error(transparent)]
    LandscapeDatabase(#[from] crate::domain::world::landscape::LandscapeDatabaseError),
    /// A party-movement operation failed.
    #[error(transparent)]
    Movement(#[from] crate::domain::world::MovementError),
    /// A merchant stock-template database operation failed.
    #[error(transparent)]
    MerchantStockTemplateDatabase(
        #[from] crate::domain::world::npc_runtime::MerchantStockTemplateDatabaseError,
    ),
    /// An object-mesh operation failed.
    #[error(transparent)]
    ObjectMesh(#[from] crate::domain::world::object_mesh::ObjectMeshError),

    // ---- game (Bevy) layer ----
    /// A dialogue-visual operation failed.
    #[error(transparent)]
    DialogueVisual(#[from] crate::game::systems::dialogue_visuals::DialogueVisualError),

    // ---- sdk / content toolchain (consumed by startup + loader systems) ----
    /// A content-cache operation failed.
    #[error(transparent)]
    Cache(#[from] crate::sdk::cache::CacheError),
    /// Loading a campaign from disk (SDK loader) failed.
    #[error(transparent)]
    CampaignLoad(#[from] crate::sdk::campaign_loader::CampaignLoadError),
    /// Packaging/exporting a campaign failed.
    #[error(transparent)]
    Package(#[from] crate::sdk::campaign_packager::PackageError),
    /// A creature-mesh topology check failed.
    #[error(transparent)]
    Topology(#[from] crate::sdk::creature_validation::TopologyError),
    /// A content-database operation failed.
    #[error(transparent)]
    Database(#[from] crate::sdk::database::DatabaseError),
    /// A dialogue tree failed validation.
    #[error(transparent)]
    DialogueValidation(#[from] crate::sdk::dialogue_editor::DialogueValidationError),
    /// Reading or parsing the game configuration failed.
    #[error(transparent)]
    GameConfig(#[from] crate::sdk::game_config::GameConfigError),
    /// A map-editor operation failed.
    #[error(transparent)]
    MapEditor(#[from] crate::sdk::map_editor::MapEditorError),
    /// A quest failed validation.
    #[error(transparent)]
    QuestValidation(#[from] crate::sdk::quest_editor::QuestValidationError),
    /// Serializing or deserializing content failed.
    #[error(transparent)]
    Serialization(#[from] crate::sdk::serialization::SerializationError),
    /// Reading or parsing the tool configuration failed.
    #[error(transparent)]
    ToolConfig(#[from] crate::sdk::tool_config::ToolConfigError),
    /// Campaign-level (SDK) validation failed.
    #[error(transparent)]
    CampaignValidation(#[from] crate::sdk::validation::CampaignValidationError),
}

/// Route an error into `tracing` and, when a game-log writer is available, into
/// the on-screen [`GameLog`](crate::game::systems::ui::GameLog).
///
/// Bevy systems cannot return `Result`, so they cannot use `?` to surface a
/// failure. `report_err!` gives them a single uniform way to report one: it
/// always emits a `tracing::error!` record, and — in the two-argument and
/// three-argument forms — appends a
/// [`GameLogEvent`](crate::game::systems::ui::GameLogEvent) so the player sees
/// the failure in-game.
///
/// # Forms
///
/// - `report_err!(err)` — log to `tracing` only.
/// - `report_err!(writer, err)` — log to `tracing` and, if `writer`
///   (an `Option<MessageWriter<GameLogEvent>>`) is `Some`, append a
///   `System`-category log entry.
/// - `report_err!(writer, category, err)` — as above but with an explicit
///   [`LogCategory`](crate::game::systems::ui::LogCategory).
///
/// `err` may be any `T: std::fmt::Display` (e.g. a [`GameError`], a module
/// error, or `&dyn std::error::Error`). `writer` is taken by mutable reference
/// through `.as_mut()`, so the caller's `Option<MessageWriter<..>>` is left
/// usable afterwards.
#[macro_export]
macro_rules! report_err {
    ($writer:expr, $category:expr, $err:expr $(,)?) => {{
        let __antares_err = $err;
        ::tracing::error!(error = %__antares_err);
        if let ::core::option::Option::Some(__antares_writer) = ($writer).as_mut() {
            __antares_writer.write($crate::game::systems::ui::GameLogEvent {
                text: ::std::string::ToString::to_string(&__antares_err),
                category: $category,
            });
        }
    }};
    ($writer:expr, $err:expr $(,)?) => {{
        $crate::report_err!(
            $writer,
            $crate::game::systems::ui::LogCategory::System,
            $err
        );
    }};
    ($err:expr $(,)?) => {{
        let __antares_err = $err;
        ::tracing::error!(error = %__antares_err);
    }};
}

#[cfg(test)]
mod tests {
    use super::GameError;

    #[test]
    fn test_report_err_tracing_only_form() {
        // The single-argument form logs to `tracing` only and accepts any
        // `Display` value (a `GameError`, a module error, or a `String`).
        let err = GameError::from(crate::domain::types::DiceRollError::ZeroSides);
        crate::report_err!(&err);
        crate::report_err!(err.to_string());
        crate::report_err!(crate::domain::types::DiceRollError::ZeroSides);
    }

    #[test]
    fn test_from_dice_roll_error_is_transparent() {
        let src = crate::domain::types::DiceRollError::ZeroSides;
        let expected = src.to_string();
        let wrapped: GameError = src.into();
        // `#[error(transparent)]` preserves the underlying message.
        assert_eq!(wrapped.to_string(), expected);
    }

    #[test]
    fn test_from_domain_validation_error() {
        let src = crate::domain::validation::ValidationError::EmptyField("name".to_string());
        let expected = src.to_string();
        let wrapped = GameError::from(src);
        assert_eq!(wrapped.to_string(), expected);
        assert!(matches!(wrapped, GameError::Validation(_)));
    }

    #[test]
    fn test_from_sdk_campaign_validation_error() {
        let src = crate::sdk::validation::CampaignValidationError::MissingClass {
            context: "party".to_string(),
            class_id: "wizard".to_string(),
        };
        let expected = src.to_string();
        let wrapped = GameError::from(src);
        assert_eq!(wrapped.to_string(), expected);
        assert!(matches!(wrapped, GameError::CampaignValidation(_)));
    }

    #[test]
    fn test_question_mark_propagation_across_layers() {
        // A boundary fn that can fail with several different module errors and
        // funnels them all into `GameError` via `?`.
        fn boundary(kind: u8) -> Result<(), GameError> {
            match kind {
                0 => Err(crate::domain::types::DiceRollError::ZeroSides)?,
                _ => Err(crate::domain::resources::ResourceError::NoFoodRemaining)?,
            }
            #[allow(unreachable_code)]
            Ok(())
        }

        assert!(matches!(boundary(0), Err(GameError::DiceRoll(_))));
        assert!(matches!(boundary(1), Err(GameError::Resource(_))));
    }

    #[test]
    fn test_transparent_forwards_source() {
        use std::error::Error as _;
        // `#[error(transparent)]` forwards both `Display` and `source()` to the
        // wrapped error. A leaf module error has no source of its own, so the
        // transparent `GameError` reports the same (`None`) — proving the
        // wrapper adds no spurious layer.
        let src = crate::domain::types::DiceRollError::ZeroSides;
        let inner_has_source = src.source().is_some();
        let wrapped = GameError::from(crate::domain::types::DiceRollError::ZeroSides);
        assert_eq!(wrapped.source().is_some(), inner_has_source);
    }
}
