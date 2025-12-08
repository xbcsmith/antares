// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::game::resources::GlobalState;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
// use bevy_egui::EguiSet;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameLog>()
            .add_systems(Update, ui_system);
    }
}

#[derive(Resource, Default)]
pub struct GameLog {
    pub messages: Vec<String>,
}

impl GameLog {
    pub fn add(&mut self, msg: String) {
        self.messages.push(msg);
        if self.messages.len() > 50 {
            self.messages.remove(0);
        }
    }
}

fn ui_system(mut contexts: EguiContexts, global_state: Res<GlobalState>, game_log: Res<GameLog>) {
    let game_state = &global_state.0;

    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    // Top Panel: Menu Bar (Placeholder)
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });
        });
    });

    // Bottom Panel: Party List
    egui::TopBottomPanel::bottom("bottom_panel")
        .resizable(true)
        .min_height(150.0)
        .show(ctx, |ui| {
            ui.heading("Party");
            ui.horizontal(|ui| {
                for (i, member) in game_state.party.members.iter().enumerate() {
                    ui.group(|ui| {
                        ui.label(format!("{}. {}", i + 1, member.name));
                        ui.label(format!("HP: {}/{}", member.hp.current, member.hp.base));
                        ui.label(format!("SP: {}/{}", member.sp.current, member.sp.base));
                        ui.label(format!("Lvl: {}", member.level));
                        ui.label(&member.class_id);
                        ui.label(&member.race_id);
                    });
                }
                if game_state.party.members.is_empty() {
                    ui.label("No party members.");
                }
            });
        });

    // Right Panel: Game Log
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .min_width(200.0)
        .show(ctx, |ui| {
            ui.heading("Game Log");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for msg in &game_log.messages {
                    ui.label(msg);
                }
            });
        });

    // Central Panel: 3D Viewport (Implicit)
}
