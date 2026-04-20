// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Trap notification pop-up UI.
//!
//! Renders a centred egui [`Window`] whenever the game is in
//! [`GameMode::TrapNotification`] mode.  The window shows:
//!
//! * the trap name and optional description,
//! * a per-member damage table (name, damage taken, dead/condition status), and
//! * an **OK — Continue** button (also dismissible with Escape / Enter).
//!
//! ## Mode transitions
//!
//! The pop-up transitions to [`GameMode::TrapNotification`] from
//! [`crate::game::systems::events::handle_events`] whenever a
//! [`crate::domain::world::MapEvent::Trap`] fires and at least one party
//! member was alive.  Pressing **OK** (or Escape / Enter / Space) returns
//! to [`GameMode::Exploration`].

use crate::application::GameMode;
use crate::game::resources::GlobalState;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

// ── Plugin ────────────────────────────────────────────────────────────────────

/// Bevy plugin for the trap notification pop-up.
///
/// Registers all systems needed to render the window and handle keyboard
/// dismissal.
pub struct TrapNotificationPlugin;

impl Plugin for TrapNotificationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (trap_notification_input_system, trap_notification_ui_system).chain(),
        );
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Keyboard-dismissal system for the trap notification pop-up.
///
/// When the game is in [`GameMode::TrapNotification`] and the player presses
/// Escape, Enter, or Space the mode is set to [`GameMode::Exploration`].
///
/// # Arguments
///
/// * `keyboard` - Optional keyboard input resource (absent in headless tests)
/// * `global_state` - Mutable game state
pub fn trap_notification_input_system(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut global_state: ResMut<GlobalState>,
) {
    if !matches!(global_state.0.mode, GameMode::TrapNotification(_)) {
        return;
    }
    let Some(keyboard) = keyboard else {
        return;
    };
    if keyboard.just_pressed(KeyCode::Escape)
        || keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space)
    {
        global_state.0.mode = GameMode::Exploration;
    }
}

/// egui rendering system for the trap notification pop-up.
///
/// Shows a centred floating [`egui::Window`] with the trap name, description,
/// and per-member damage report.  An **OK — Continue** button dismisses the
/// pop-up and returns to [`GameMode::Exploration`].
///
/// The system is a no-op when the game is not in
/// [`GameMode::TrapNotification`] mode or when the egui context is
/// unavailable (headless tests).
///
/// # Arguments
///
/// * `contexts` - Bevy-egui context provider
/// * `global_state` - Mutable game state
pub fn trap_notification_ui_system(
    mut contexts: Option<EguiContexts>,
    mut global_state: ResMut<GlobalState>,
) {
    let trap_state = match &global_state.0.mode {
        GameMode::TrapNotification(s) => s.clone(),
        _ => return,
    };

    let Some(ref mut contexts) = contexts else {
        return;
    };

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let mut dismiss = false;

    egui::Window::new("trap_notification_window")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .min_width(320.0)
        .show(ctx, |ui| {
            // ── Title ─────────────────────────────────────────────────────
            if trap_state.avoided {
                ui.heading(format!(
                    "\u{1f6e1} Trap Avoided \u{2014} {}",
                    trap_state.trap_name
                ));
            } else {
                ui.heading(format!("\u{26a0} Trap! \u{2014} {}", trap_state.trap_name));
            }

            if !trap_state.trap_description.is_empty() {
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(&trap_state.trap_description)
                        .italics()
                        .color(egui::Color32::LIGHT_GRAY),
                );
            }

            ui.separator();

            // ── Body ──────────────────────────────────────────────────────
            if trap_state.avoided {
                ui.label("The party floats safely over the trap (Levitate active).");
            } else {
                ui.label(egui::RichText::new("Party Damage Report:").strong());
                ui.add_space(4.0);

                for result in &trap_state.member_results {
                    ui.push_id(&result.name, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("{:<16}", result.name)).strong());
                            ui.label(format!("took {} damage", result.damage_taken));
                            if result.died {
                                ui.label(
                                    egui::RichText::new("  [DEAD]")
                                        .color(egui::Color32::RED)
                                        .strong(),
                                );
                            } else if let Some(ref effect) = trap_state.effect {
                                let effect_lower = effect.to_lowercase();
                                let display = match effect_lower.as_str() {
                                    "poison" => "Poisoned",
                                    "paralysis" | "paralyzed" => "Paralyzed",
                                    "teleport" => "Teleported",
                                    other => other,
                                };
                                ui.label(
                                    egui::RichText::new(format!("  [{}]", display))
                                        .color(egui::Color32::YELLOW),
                                );
                            }
                        });
                    });
                }
            }

            ui.separator();

            // ── Footer button ─────────────────────────────────────────────
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("OK \u{2014} Continue").clicked() {
                    dismiss = true;
                }
            });
        });

    if dismiss {
        global_state.0.mode = GameMode::Exploration;
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{TrapMemberResult, TrapNotificationState};

    /// `TrapNotificationPlugin` builds without panicking (smoke test).
    #[test]
    fn test_trap_notification_plugin_builds() {
        let mut app = App::new();
        app.add_plugins(TrapNotificationPlugin);
        // Plugin registered successfully if we reach here.
    }

    /// `trap_notification_input_system` transitions to Exploration when Escape
    /// is pressed while in TrapNotification mode.
    #[test]
    fn test_trap_notification_input_system_escape_dismisses() {
        use crate::application::GameState;
        use bevy::input::ButtonInput;

        let mut app = App::new();
        // Do NOT add InputPlugin — it would register a First-schedule clear
        // that wipes just_pressed before our Update system sees it.
        // Manually insert the resource so the system can read it.
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.add_plugins(TrapNotificationPlugin);

        let mut gs = GameState::new();
        gs.mode = GameMode::TrapNotification(TrapNotificationState::new_avoided(
            "Test Trap".to_string(),
            String::new(),
        ));
        app.insert_resource(GlobalState(gs));

        // Simulate an Escape key press (just_pressed stays set because there
        // is no InputPlugin clear system registered).
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();

        let state = app.world().resource::<GlobalState>();
        assert!(
            matches!(state.0.mode, GameMode::Exploration),
            "Escape must dismiss the trap notification, got {:?}",
            state.0.mode
        );
    }

    /// When in a different mode, the input system must be a no-op.
    #[test]
    fn test_trap_notification_input_system_noop_in_exploration() {
        use crate::application::GameState;
        use bevy::input::ButtonInput;

        let mut app = App::new();
        // Manually insert ButtonInput — no InputPlugin so just_pressed is not cleared.
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.add_plugins(TrapNotificationPlugin);

        let gs = GameState::new(); // starts in Exploration
        app.insert_resource(GlobalState(gs));

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();

        let state = app.world().resource::<GlobalState>();
        assert!(
            matches!(state.0.mode, GameMode::Exploration),
            "Exploration mode must not be disturbed by Escape, got {:?}",
            state.0.mode
        );
    }

    /// `TrapNotificationState::new_avoided` sets the avoided flag and leaves
    /// member_results empty.
    #[test]
    fn test_trap_notification_state_new_avoided_fields() {
        let s =
            TrapNotificationState::new_avoided("Pit Trap".to_string(), "A deep pit.".to_string());
        assert!(s.avoided);
        assert!(s.member_results.is_empty());
        assert!(s.effect.is_none());
        assert_eq!(s.trap_name, "Pit Trap");
        assert_eq!(s.trap_description, "A deep pit.");
    }

    /// `TrapNotificationState::new_triggered` stores member results and effect.
    #[test]
    fn test_trap_notification_state_new_triggered_fields() {
        let results = vec![TrapMemberResult {
            name: "Aldric".to_string(),
            damage_taken: 15,
            died: false,
        }];
        let s = TrapNotificationState::new_triggered(
            "Spike Trap".to_string(),
            "Sharp spikes!".to_string(),
            results.clone(),
            Some("poison".to_string()),
        );
        assert!(!s.avoided);
        assert_eq!(s.member_results, results);
        assert_eq!(s.effect, Some("poison".to_string()));
    }

    /// A TrapMemberResult with died=true should be treated as dead.
    #[test]
    fn test_trap_member_result_died_flag() {
        let dead_result = TrapMemberResult {
            name: "Selwyn".to_string(),
            damage_taken: 30,
            died: true,
        };
        assert!(dead_result.died);
        assert_eq!(dead_result.damage_taken, 30);
    }
}
