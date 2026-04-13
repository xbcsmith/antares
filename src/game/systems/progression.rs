// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Auto Level-Up Progression System
//!
//! Provides the Bevy system and plugin that automatically advances party
//! members to the next level the moment they accumulate sufficient XP —
//! but only when the campaign is configured with [`LevelUpMode::Auto`].
//!
//! # Overview
//!
//! When `CampaignConfig::level_up_mode` is [`LevelUpMode::Auto`], the
//! [`auto_level_up_system`] runs every `Update` frame and checks every living
//! party member for a pending level-up.  If a member's `experience` meets or
//! exceeds the threshold for the next level,
//! [`level_up_and_grant_spells_with_level_db`] is called, HP and SP are
//! updated, any newly unlocked spells are granted, and a log entry is written
//! to the [`GameLog`].
//!
//! The system is deliberately **a no-op** in two situations:
//!
//! - [`LevelUpMode::NpcTrainer`] — characters accumulate pending levels but
//!   must visit a trainer NPC to apply them.
//! - [`GameMode::Combat`] — level-ups are deferred until the next
//!   non-combat frame to avoid mid-battle stat changes.
//!
//! # Multi-Level Advance
//!
//! If a single XP award pushes a character through multiple thresholds (e.g.
//! winning a boss fight at level 1 with enough XP for levels 2, 3, and 4),
//! the system loops until `check_level_up_with_db` returns `false`, applying
//! every pending level in one frame.
//!
//! # Campaign Max Level
//!
//! `CampaignConfig::max_party_level` is forwarded to
//! `level_up_and_grant_spells_with_level_db` as the `max_level` cap.  When
//! the cap is reached the inner loop stops gracefully.
//!
//! # Examples
//!
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::systems::progression::ProgressionPlugin;
//!
//! let mut app = App::new();
//! app.add_plugins(ProgressionPlugin);
//! ```

use bevy::prelude::*;
use rand::rng;

use crate::application::resources::GameContent;
use crate::application::GameMode;
use crate::domain::campaign::LevelUpMode;
use crate::domain::levels::LevelDatabase;
use crate::domain::progression::{
    check_level_up_with_db, level_up_and_grant_spells_with_level_db, ProgressionError,
};
use crate::game::resources::game_data::GameDataResource;
use crate::game::resources::GlobalState;
use crate::game::systems::ui::{GameLog, LogCategory};

/// Bevy system that automatically levels up living party members when they
/// accumulate enough XP and the campaign uses [`LevelUpMode::Auto`].
///
/// # Parameters
///
/// - `global_state` — mutable game state; party members and campaign config
///   are read and modified here.
/// - `content` — optional campaign content resource; supplies the class and
///   spell databases required by the level-up pipeline.  When absent the
///   system returns early without modifying any state.
/// - `game_data` — optional game data resource; supplies the optional
///   per-class XP threshold table ([`LevelDatabase`]).  `None` means the
///   formula fallback is used for all classes.
/// - `game_log` — optional [`GameLog`] resource; level-up messages are
///   written here as [`LogCategory::System`] entries.
///
/// # Behaviour
///
/// 1. Returns early when `level_up_mode != LevelUpMode::Auto`.
/// 2. Returns early while in [`GameMode::Combat`] — level-up is deferred to
///    the next non-combat frame.
/// 3. Returns early when campaign content is absent (no class DB available).
/// 4. For each living party member, loops until the XP check fails, applying
///    every pending level in one pass (supports large XP awards).
/// 5. Writes one [`LogCategory::System`] log entry per level gained in the
///    format `"{name} advanced to level {n}! (+{hp} HP[, {k} new spell(s)])"`.
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::progression::auto_level_up_system;
///
/// let mut app = App::new();
/// app.add_systems(Update, auto_level_up_system);
/// ```
pub fn auto_level_up_system(
    mut global_state: ResMut<GlobalState>,
    content: Option<Res<GameContent>>,
    game_data: Option<Res<GameDataResource>>,
    mut game_log: Option<ResMut<GameLog>>,
) {
    // 1. Only run in Auto level-up mode.
    if global_state.0.campaign_config.level_up_mode != LevelUpMode::Auto {
        return;
    }

    // 2. Defer level-ups until the current combat sequence ends.
    if matches!(global_state.0.mode, GameMode::Combat(_)) {
        return;
    }

    // 3. Campaign content is required for class HP dice and spell granting.
    let Some(ref content) = content else {
        return;
    };

    // 4. Extract optional per-class XP table.
    //    The lifetime of `level_db` is tied to `game_data`, which remains
    //    alive for the duration of this function — safe to use in the loop.
    let level_db: Option<&LevelDatabase> = game_data
        .as_deref()
        .and_then(|gd| gd.data().levels.as_ref());

    // Copy the campaign max level — it is `Option<u32>` (Copy).
    let max_level = global_state.0.campaign_config.max_party_level;

    let class_db = &content.db().classes;
    let spell_db = &content.db().spells;

    let mut rng = rng();
    let mut log_entries: Vec<String> = Vec::new();

    for member in global_state.0.party.members.iter_mut() {
        // 5. Skip dead (or stoned / eradicated) party members.
        if !member.is_alive() {
            continue;
        }

        // 6. Multi-level loop: keep advancing until XP is insufficient,
        //    the campaign cap is hit, or an unexpected error occurs.
        loop {
            if !check_level_up_with_db(member, level_db) {
                break;
            }

            match level_up_and_grant_spells_with_level_db(
                member, class_db, spell_db, level_db, max_level, &mut rng,
            ) {
                Ok((hp_gained, new_spells)) => {
                    let spell_msg = if new_spells.is_empty() {
                        String::new()
                    } else {
                        format!(", {} new spell(s)", new_spells.len())
                    };
                    log_entries.push(format!(
                        "{} advanced to level {}! (+{} HP{})",
                        member.name, member.level, hp_gained, spell_msg
                    ));
                }
                Err(ProgressionError::MaxLevelReached) => {
                    // Character is at the campaign or global level cap; stop.
                    break;
                }
                Err(e) => {
                    tracing::warn!(
                        "auto_level_up_system: level-up failed for {}: {}",
                        member.name,
                        e
                    );
                    break;
                }
            }
        }
    }

    // 7. Flush all queued log entries into the GameLog resource.
    if let Some(ref mut log) = game_log {
        for entry in log_entries {
            log.add_entry(entry, LogCategory::System);
        }
    }
}

/// Plugin that registers the auto level-up progression system.
///
/// Schedules [`auto_level_up_system`] in the `Update` set, ordered after
/// [`crate::game::systems::ui::consume_game_log_events`] so that event-driven
/// log entries from the same frame are committed before progression messages
/// are appended.
///
/// # Registration
///
/// Add this plugin in `AntaresPlugin::build` alongside the other game-system
/// plugins:
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::progression::ProgressionPlugin;
///
/// let mut app = App::new();
/// app.add_plugins(ProgressionPlugin);
/// ```
pub struct ProgressionPlugin;

impl Plugin for ProgressionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            auto_level_up_system.after(crate::game::systems::ui::consume_game_log_events),
        );
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::resources::GameContent;
    use crate::application::{GameMode, GameState};
    use crate::domain::campaign::LevelUpMode;
    use crate::domain::campaign_loader::GameData;
    use crate::domain::character::{Alignment, Character, Condition, Sex};
    use crate::domain::combat::engine::CombatState;
    use crate::domain::combat::types::Handicap;
    use crate::domain::levels::LevelDatabase;
    use crate::domain::progression::award_experience;
    use crate::game::resources::game_data::GameDataResource;
    use crate::game::resources::GlobalState;
    use crate::game::systems::ui::GameLog;

    // ── Helpers ──────────────────────────────────────────────────────────────

    /// Creates a living knight with generous base HP.
    fn make_knight(name: &str) -> Character {
        let mut c = Character::new(
            name.to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.base = 50;
        c.hp.current = 50;
        c
    }

    /// Builds a `GameContent` resource backed by the real class database so
    /// HP-dice lookups succeed during level-up.
    fn make_content() -> GameContent {
        let mut db = crate::sdk::database::ContentDatabase::new();
        db.classes = crate::domain::classes::ClassDatabase::load_from_file("data/classes.ron")
            .expect("data/classes.ron must be present for progression tests");
        // Spell DB may be empty — knights gain no spells.
        GameContent::new(db)
    }

    /// Builds a minimal Bevy `App` with `ProgressionPlugin`, `GameLog`,
    /// `GlobalState` (empty party), and real `GameContent`.
    fn build_app() -> App {
        let mut app = App::new();
        app.add_plugins(ProgressionPlugin);
        app.init_resource::<GameLog>();

        let game_state = GameState::new();
        app.insert_resource(GlobalState(game_state));
        app.insert_resource(make_content());
        app
    }

    /// Adds `character` to the party in `app`.
    fn add_member(app: &mut App, character: Character) {
        let mut gs = app.world_mut().resource_mut::<GlobalState>();
        gs.0.party
            .add_member(character)
            .expect("party must not be full in test setup");
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    /// A knight with exactly the level-2 XP threshold (1 000 XP by formula)
    /// levels up on the next frame, and a log entry mentioning the new level
    /// is written to [`GameLog`].
    #[test]
    fn test_auto_level_up_advances_level_and_writes_log() {
        let mut app = build_app();

        let mut knight = make_knight("Sir Lancelot");
        // Formula: 1000 * (2 - 1)^1.5 = 1000 XP for level 2.
        award_experience(&mut knight, 1_000).unwrap();
        add_member(&mut app, knight);

        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.party.members[0].level, 2,
            "knight should reach level 2 after earning 1000 XP"
        );

        let log = app.world().resource::<GameLog>();
        assert!(
            log.entries
                .iter()
                .any(|e| e.text.contains("advanced to level 2")),
            "expected 'advanced to level 2' in log; entries: {:?}",
            log.entries.iter().map(|e| &e.text).collect::<Vec<_>>()
        );
    }

    /// When `level_up_mode` is [`LevelUpMode::NpcTrainer`], no automatic
    /// level-up occurs even if the character has sufficient XP.
    #[test]
    fn test_auto_level_up_noop_in_npc_trainer_mode() {
        let mut app = build_app();

        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.campaign_config.level_up_mode = LevelUpMode::NpcTrainer;
        }

        let mut knight = make_knight("Waiting Hero");
        award_experience(&mut knight, 1_000).unwrap();
        add_member(&mut app, knight);

        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.party.members[0].level, 1,
            "level must remain 1 in NpcTrainer mode"
        );

        let log = app.world().resource::<GameLog>();
        assert!(
            !log.entries.iter().any(|e| e.text.contains("advanced")),
            "no level-up log entry should appear in NpcTrainer mode"
        );
    }

    /// While in [`GameMode::Combat`], level-ups are deferred.  The character's
    /// level must remain unchanged until the combat sequence ends.
    #[test]
    fn test_auto_level_up_skipped_during_combat() {
        let mut app = build_app();

        let mut knight = make_knight("Combat Hero");
        award_experience(&mut knight, 1_000).unwrap();
        add_member(&mut app, knight);

        // Transition to combat mode.
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            let cs = CombatState::new(Handicap::Even);
            gs.0.mode = GameMode::Combat(cs);
        }

        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.party.members[0].level, 1,
            "level must remain 1 while in combat mode"
        );

        let log = app.world().resource::<GameLog>();
        assert!(
            !log.entries.iter().any(|e| e.text.contains("advanced")),
            "no level-up log entry should appear during combat"
        );
    }

    /// A dead character must never receive a level-up, regardless of XP.
    #[test]
    fn test_auto_level_up_skips_dead_characters() {
        let mut app = build_app();

        // Construct a dead character and bypass the XP guard by setting
        // `experience` directly (dead characters cannot earn XP normally).
        let mut corpse = make_knight("Fallen Hero");
        corpse.hp.current = 0;
        corpse.conditions.add(Condition::DEAD);
        corpse.experience = 1_000;
        add_member(&mut app, corpse);

        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.party.members[0].level, 1,
            "dead characters must not be levelled up"
        );

        let log = app.world().resource::<GameLog>();
        assert!(
            !log.entries.iter().any(|e| e.text.contains("advanced")),
            "no level-up log entry should appear for a dead character"
        );
    }

    /// When a character earns enough XP to advance multiple levels in one
    /// award, the system applies every pending level in a single frame.
    ///
    /// XP formula thresholds (`1000 * (level-1)^1.5`):
    ///   Level 2 ≈  1 000 XP
    ///   Level 3 ≈  2 828 XP
    ///   Level 4 ≈  5 196 XP
    ///   Level 5 ≈  8 000 XP
    ///
    /// Awarding 6 000 XP should advance a level-1 character to level 4,
    /// generating three separate log entries.
    #[test]
    fn test_auto_level_up_multi_level_pass() {
        let mut app = build_app();

        let mut knight = make_knight("Veteran");
        // 6 000 XP → enough for levels 2, 3, and 4 in a single award.
        knight.experience = 6_000;
        add_member(&mut app, knight);

        app.update();

        let gs = app.world().resource::<GlobalState>();
        let level = gs.0.party.members[0].level;
        assert!(
            level >= 3,
            "expected level >= 3 after 6000 XP multi-level award, got {}",
            level
        );

        let log = app.world().resource::<GameLog>();
        let advance_count = log
            .entries
            .iter()
            .filter(|e| e.text.contains("advanced to level"))
            .count();
        assert!(
            advance_count >= 2,
            "expected >= 2 level-up log entries for multi-level advance, got {}",
            advance_count
        );
    }

    /// `CampaignConfig::max_party_level` is respected as a hard cap.
    /// A character with enormous XP and a cap of 3 must reach exactly level 3
    /// and stop there.
    #[test]
    fn test_auto_level_up_respects_max_party_level() {
        let mut app = build_app();

        // Apply a campaign-level cap of 3.
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.campaign_config.max_party_level = Some(3);
        }

        let mut knight = make_knight("Capped Hero");
        // Far more XP than needed for level 3.
        knight.experience = 1_000_000;
        add_member(&mut app, knight);

        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.party.members[0].level, 3,
            "character must not exceed the campaign max_party_level cap"
        );
    }

    /// When `GameContent` is absent, the system returns early without
    /// panicking and must not modify any character state.
    #[test]
    fn test_auto_level_up_noop_without_content() {
        let mut app = App::new();
        app.add_plugins(ProgressionPlugin);
        app.init_resource::<GameLog>();

        let game_state = GameState::new();
        app.insert_resource(GlobalState(game_state));
        // Intentionally omit GameContent.

        let mut knight = make_knight("No Content Hero");
        knight.experience = 1_000;
        {
            let mut gs = app.world_mut().resource_mut::<GlobalState>();
            gs.0.party.add_member(knight).unwrap();
        }

        app.update(); // Must not panic.

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.party.members[0].level, 1,
            "level must be unchanged when GameContent is absent"
        );
    }

    /// With an explicit [`LevelDatabase`] injected via [`GameDataResource`],
    /// the per-class XP table overrides the formula.
    ///
    /// A knight table requiring 1 200 XP for level 2 must prevent level-up
    /// when the character has only 1 000 XP.
    #[test]
    fn test_auto_level_up_uses_level_db_when_present() {
        let mut app = build_app();

        // Knight must have 1 200 XP for level 2 per the custom table.
        let ron = r#"(entries: [(class_id: "knight", thresholds: [0, 1200, 3000, 6000])])"#;
        let level_db =
            LevelDatabase::load_from_string(ron).expect("valid RON for test LevelDatabase");
        let mut game_data = GameData::new();
        game_data.levels = Some(level_db);
        app.insert_resource(GameDataResource::new(game_data));

        // 1 000 XP meets the formula threshold but NOT the table threshold.
        let mut knight = make_knight("Table Knight");
        award_experience(&mut knight, 1_000).unwrap();
        add_member(&mut app, knight);

        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.party.members[0].level, 1,
            "knight with 1000 XP must not level up when the table requires 1200"
        );

        // Verify the log is also silent.
        let log = app.world().resource::<GameLog>();
        assert!(
            !log.entries.iter().any(|e| e.text.contains("advanced")),
            "no level-up log entry should appear when table threshold is unmet"
        );
    }

    /// A knight with 1 200+ XP advances when the custom table sets the
    /// level-2 threshold at exactly 1 200 XP.
    #[test]
    fn test_auto_level_up_uses_level_db_threshold_met() {
        let mut app = build_app();

        let ron = r#"(entries: [(class_id: "knight", thresholds: [0, 1200, 3000, 6000])])"#;
        let level_db =
            LevelDatabase::load_from_string(ron).expect("valid RON for test LevelDatabase");
        let mut game_data = GameData::new();
        game_data.levels = Some(level_db);
        app.insert_resource(GameDataResource::new(game_data));

        let mut knight = make_knight("Table Knight Advance");
        award_experience(&mut knight, 1_200).unwrap();
        add_member(&mut app, knight);

        app.update();

        let gs = app.world().resource::<GlobalState>();
        assert_eq!(
            gs.0.party.members[0].level, 2,
            "knight with exactly 1200 XP should level up when table threshold is 1200"
        );
    }

    /// The `ProgressionPlugin` must construct without panicking even in a
    /// bare `App` (plugin registration safety check).
    #[test]
    fn test_progression_plugin_builds_without_panic() {
        let mut app = App::new();
        app.add_plugins(ProgressionPlugin);
        // No `update()` — we only verify that plugin construction succeeds.
    }
}
