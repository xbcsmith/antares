// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Input widgets for `AttributePair` (u8) and `AttributePair16` (u16).
//!
//! Contains [`AttributePairInputState`], [`AttributePairInput`], and
//! [`AttributePair16Input`].

use antares::domain::character::{AttributePair, AttributePair16};
use eframe::egui;

// =============================================================================
// AttributePair Input Widget
// =============================================================================

/// State for tracking AttributePair input sync behavior.
#[derive(Debug, Clone, Copy, Default)]
pub struct AttributePairInputState {
    /// Whether auto-sync is enabled (current follows base)
    pub auto_sync: bool,
}

impl AttributePairInputState {
    /// Creates a new state with auto-sync enabled.
    pub fn new() -> Self {
        Self { auto_sync: true }
    }

    /// Creates a new state with specified auto-sync setting.
    pub fn with_auto_sync(auto_sync: bool) -> Self {
        Self { auto_sync }
    }
}

/// Widget for editing an `AttributePair` (u8 base/current).
///
/// This widget provides dual input fields for base and current values,
/// with optional auto-sync behavior and a reset button.
pub struct AttributePairInput<'a> {
    /// Label for the attribute
    label: &'a str,
    /// The AttributePair to edit
    value: &'a mut AttributePair,
    /// Widget state for auto-sync
    state: Option<&'a mut AttributePairInputState>,
    /// Unique id salt for widget disambiguation
    id_salt: Option<&'a str>,
    /// Whether to show the reset button
    show_reset: bool,
    /// Whether to show the auto-sync checkbox
    show_auto_sync: bool,
}

impl<'a> AttributePairInput<'a> {
    /// Creates a new AttributePair input widget.
    ///
    /// # Arguments
    ///
    /// * `label` - Display label for the attribute (e.g., "AC", "Might")
    /// * `value` - Mutable reference to the AttributePair
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use antares::domain::character::AttributePair;
    /// use campaign_builder::ui_helpers::AttributePairInput;
    ///
    /// fn example(ui: &mut egui::Ui, ac: &mut AttributePair) {
    ///     AttributePairInput::new("AC", ac).show(ui);
    /// }
    /// ```
    pub fn new(label: &'a str, value: &'a mut AttributePair) -> Self {
        Self {
            label,
            value,
            state: None,
            id_salt: None,
            show_reset: true,
            show_auto_sync: true,
        }
    }

    /// Adds state tracking for auto-sync behavior.
    pub fn with_state(mut self, state: &'a mut AttributePairInputState) -> Self {
        self.state = Some(state);
        self
    }

    /// Sets a custom id salt for widget disambiguation.
    pub fn with_id_salt(mut self, id_salt: &'a str) -> Self {
        self.id_salt = Some(id_salt);
        self
    }

    /// Controls visibility of the reset button.
    pub fn with_reset_button(mut self, show: bool) -> Self {
        self.show_reset = show;
        self
    }

    /// Controls visibility of the auto-sync checkbox.
    pub fn with_auto_sync_checkbox(mut self, show: bool) -> Self {
        self.show_auto_sync = show;
        self
    }

    /// Renders the widget.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    ///
    /// # Returns
    ///
    /// `true` if the value was changed, `false` otherwise.
    pub fn show(self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let id_salt = self
            .id_salt
            .map(String::from)
            .unwrap_or_else(|| self.label.to_lowercase().replace(' ', "_"));

        ui.horizontal(|ui| {
            ui.label(format!("{}:", self.label));

            // Base value input
            ui.label("Base:");
            let base_response = ui.add(
                egui::DragValue::new(&mut self.value.base)
                    .range(0..=255)
                    .speed(1),
            );

            // Track if base changed for auto-sync
            let base_changed = base_response.changed();
            if base_changed {
                changed = true;
                // Auto-sync: update current to match base if enabled
                if let Some(ref state) = self.state {
                    if state.auto_sync {
                        self.value.current = self.value.base;
                    }
                }
            }

            // Current value input
            ui.label("Current:");
            if ui
                .add(
                    egui::DragValue::new(&mut self.value.current)
                        .range(0..=255)
                        .speed(1),
                )
                .changed()
            {
                changed = true;
            }

            // Auto-sync checkbox
            if self.show_auto_sync {
                if let Some(state) = self.state {
                    ui.checkbox(&mut state.auto_sync, "Sync");
                }
            }

            // Reset button
            if self.show_reset {
                ui.push_id(format!("{}_reset", id_salt), |ui| {
                    if ui
                        .button("🔄")
                        .on_hover_text("Reset current to base")
                        .clicked()
                    {
                        self.value.reset();
                        changed = true;
                    }
                });
            }
        });

        changed
    }
}

/// Widget for editing an `AttributePair16` (u16 base/current).
///
/// This widget provides dual input fields for base and current values,
/// with optional auto-sync behavior and a reset button. Used for larger
/// values like HP and SP.
pub struct AttributePair16Input<'a> {
    /// Label for the attribute
    label: &'a str,
    /// The AttributePair16 to edit
    value: &'a mut AttributePair16,
    /// Widget state for auto-sync
    state: Option<&'a mut AttributePairInputState>,
    /// Unique id salt for widget disambiguation
    id_salt: Option<&'a str>,
    /// Whether to show the reset button
    show_reset: bool,
    /// Whether to show the auto-sync checkbox
    show_auto_sync: bool,
    /// Maximum value allowed
    max_value: u16,
}

impl<'a> AttributePair16Input<'a> {
    /// Creates a new AttributePair16 input widget.
    ///
    /// # Arguments
    ///
    /// * `label` - Display label for the attribute (e.g., "HP", "SP")
    /// * `value` - Mutable reference to the AttributePair16
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eframe::egui;
    /// use antares::domain::character::AttributePair16;
    /// use campaign_builder::ui_helpers::AttributePair16Input;
    ///
    /// fn example(ui: &mut egui::Ui, hp: &mut AttributePair16) {
    ///     AttributePair16Input::new("HP", hp).show(ui);
    /// }
    /// ```
    pub fn new(label: &'a str, value: &'a mut AttributePair16) -> Self {
        Self {
            label,
            value,
            state: None,
            id_salt: None,
            show_reset: true,
            show_auto_sync: true,
            max_value: u16::MAX,
        }
    }

    /// Adds state tracking for auto-sync behavior.
    pub fn with_state(mut self, state: &'a mut AttributePairInputState) -> Self {
        self.state = Some(state);
        self
    }

    /// Sets a custom id salt for widget disambiguation.
    pub fn with_id_salt(mut self, id_salt: &'a str) -> Self {
        self.id_salt = Some(id_salt);
        self
    }

    /// Controls visibility of the reset button.
    pub fn with_reset_button(mut self, show: bool) -> Self {
        self.show_reset = show;
        self
    }

    /// Controls visibility of the auto-sync checkbox.
    pub fn with_auto_sync_checkbox(mut self, show: bool) -> Self {
        self.show_auto_sync = show;
        self
    }

    /// Sets the maximum allowed value.
    pub fn with_max_value(mut self, max: u16) -> Self {
        self.max_value = max;
        self
    }

    /// Renders the widget.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    ///
    /// # Returns
    ///
    /// `true` if the value was changed, `false` otherwise.
    pub fn show(self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let id_salt = self
            .id_salt
            .map(String::from)
            .unwrap_or_else(|| self.label.to_lowercase().replace(' ', "_"));

        ui.horizontal(|ui| {
            ui.label(format!("{}:", self.label));

            // Base value input
            ui.label("Base:");
            let base_response = ui.add(
                egui::DragValue::new(&mut self.value.base)
                    .range(0..=self.max_value)
                    .speed(1),
            );

            // Track if base changed for auto-sync
            let base_changed = base_response.changed();
            if base_changed {
                changed = true;
                // Auto-sync: update current to match base if enabled
                if let Some(ref state) = self.state {
                    if state.auto_sync {
                        self.value.current = self.value.base;
                    }
                }
            }

            // Current value input
            ui.label("Current:");
            if ui
                .add(
                    egui::DragValue::new(&mut self.value.current)
                        .range(0..=self.max_value)
                        .speed(1),
                )
                .changed()
            {
                changed = true;
            }

            // Auto-sync checkbox
            if self.show_auto_sync {
                if let Some(state) = self.state {
                    ui.checkbox(&mut state.auto_sync, "Sync");
                }
            }

            // Reset button
            if self.show_reset {
                ui.push_id(format!("{}_reset", id_salt), |ui| {
                    if ui
                        .button("🔄")
                        .on_hover_text("Reset current to base")
                        .clicked()
                    {
                        self.value.reset();
                        changed = true;
                    }
                });
            }
        });

        changed
    }
}
