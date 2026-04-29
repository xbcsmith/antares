// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Portrait and sprite picker helpers for the NPC editor.
//!
//! Contains:
//! - [`NpcEditorState::load_portrait_texture`] – cached texture loader
//! - [`NpcEditorState::show_portrait_grid_picker`] – grid picker popup
//! - [`NpcEditorState::show_sprite_sheet_picker`] – sprite sheet picker popup
//! - [`show_npc_preview`] – standalone NPC preview panel renderer

use super::NpcEditorState;
use crate::creature_assets::CreatureAssetManager;
use crate::ui_helpers::resolve_portrait_path;
use antares::domain::dialogue::{DialogueAction, DialogueTree};
use antares::domain::world::NpcDefinition;
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;

impl NpcEditorState {
    /// Loads a portrait texture from the campaign assets directory
    ///
    /// This method caches loaded textures to avoid reloading. If the texture is already
    /// cached, it returns immediately. Failed loads are also cached to prevent repeated
    /// attempts.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context for texture registration
    /// * `campaign_dir` - The campaign directory containing assets/portraits
    /// * `portrait_id` - The portrait ID to load (e.g., "0", "1", "warrior")
    ///
    /// # Returns
    ///
    /// Returns `true` if the texture was successfully loaded (or was already cached),
    /// `false` if the load failed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::npc_editor::NpcEditorState;
    /// use std::path::PathBuf;
    ///
    /// let mut state = NpcEditorState::new();
    /// let campaign_dir = PathBuf::from("/path/to/campaign");
    /// // In egui context:
    /// // let texture = state.load_portrait_texture(ctx, Some(&campaign_dir), "0");
    /// ```
    pub fn load_portrait_texture(
        &mut self,
        ctx: &egui::Context,
        campaign_dir: Option<&PathBuf>,
        portrait_id: &str,
    ) -> bool {
        // Check if already cached
        if self.portrait_textures.contains_key(portrait_id) {
            return self
                .portrait_textures
                .get(portrait_id)
                .is_some_and(|t| t.is_some());
        }

        // Attempt to load and decode image
        let texture_handle = (|| {
            let path = resolve_portrait_path(campaign_dir, portrait_id)?;

            // Read image file; return None on failure (caller sees a "?" placeholder)
            let image_bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(_) => {
                    return None;
                }
            };

            // Decode image; return None on failure
            let dynamic_image = match image::load_from_memory(&image_bytes) {
                Ok(img) => img,
                Err(_) => {
                    return None;
                }
            };

            // Convert to RGBA8
            let rgba_image = dynamic_image.to_rgba8();
            let size = [rgba_image.width() as usize, rgba_image.height() as usize];
            let pixels = rgba_image.as_flat_samples();

            // Create egui ColorImage
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

            // Register texture with egui
            let texture_handle = ctx.load_texture(
                format!("npc_portrait_{}", portrait_id),
                color_image,
                egui::TextureOptions::LINEAR,
            );

            Some(texture_handle)
        })();

        // Cache result (even None for failed loads to avoid repeated attempts)
        let loaded = texture_handle.is_some();

        self.portrait_textures
            .insert(portrait_id.to_string(), texture_handle);

        loaded
    }

    /// Shows portrait grid picker popup for visual portrait selection
    ///
    /// Displays a popup window with a grid of portrait thumbnails that the user can click to select.
    /// The popup is modal and closes when a portrait is selected or the close button is clicked.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context for rendering
    /// * `campaign_dir` - The campaign directory containing assets/portraits
    ///
    /// # Returns
    ///
    /// Returns `Some(portrait_id)` if the user clicked on a portrait to select it,
    /// or `None` if no selection was made this frame.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::npc_editor::NpcEditorState;
    /// use std::path::PathBuf;
    ///
    /// let mut state = NpcEditorState::new();
    /// let campaign_dir = PathBuf::from("/path/to/campaign");
    /// // In egui context:
    /// // if let Some(selected_id) = state.show_portrait_grid_picker(ctx, Some(&campaign_dir)) {
    /// //     println!("Selected portrait: {}", selected_id);
    /// // }
    /// ```
    pub fn show_portrait_grid_picker(
        &mut self,
        ctx: &egui::Context,
        campaign_dir: Option<&PathBuf>,
    ) -> Option<String> {
        let mut selected_portrait: Option<String> = None;

        // Clone the portraits list to avoid borrow issues
        let available_portraits = self.available_portraits.clone();

        const THUMBNAIL_SIZE: f32 = 80.0;
        const MIN_COLUMNS: usize = 3;
        const MAX_COLUMNS: usize = 8;
        const CELL_WIDTH: f32 = THUMBNAIL_SIZE + 18.0;
        const WINDOW_MARGIN: f32 = 48.0;
        const FOOTER_HEIGHT: f32 = 40.0;
        const DEFAULT_WIDTH: f32 = 640.0;
        const DEFAULT_HEIGHT: f32 = 560.0;
        const MIN_WIDTH: f32 = 360.0;
        const MIN_HEIGHT: f32 = 320.0;

        let content_rect = ctx.content_rect();
        let max_window_width = (content_rect.width() - WINDOW_MARGIN).max(MIN_WIDTH);
        let max_window_height = (content_rect.height() - WINDOW_MARGIN).max(MIN_HEIGHT);
        let window_width = DEFAULT_WIDTH.min(max_window_width);
        let window_height = DEFAULT_HEIGHT.min(max_window_height);

        egui::Window::new("Select Portrait")
            .collapsible(false)
            .resizable(true)
            .default_size([window_width, window_height])
            .max_size([max_window_width, max_window_height])
            .constrain_to(content_rect)
            .show(ctx, |ui| {
                ui.label("Click a portrait to select:");
                ui.separator();

                let scroll_height = (ui.available_height() - FOOTER_HEIGHT).max(160.0);
                let columns = ((ui.available_width() / CELL_WIDTH).floor() as usize)
                    .clamp(MIN_COLUMNS, MAX_COLUMNS);

                egui::ScrollArea::both()
                    .id_salt("npc_portrait_grid_picker_scroll")
                    .max_height(scroll_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Display portraits in as many columns as fit the current window width.
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                        let total_portraits = available_portraits.len();
                        let rows = total_portraits.div_ceil(columns);

                        for row in 0..rows {
                            ui.horizontal(|ui| {
                                for col in 0..columns {
                                    let idx = row * columns + col;
                                    if idx >= total_portraits {
                                        break;
                                    }

                                    let portrait_id = &available_portraits[idx];

                                    ui.vertical(|ui| {
                                        // Try to load texture
                                        self.load_portrait_texture(ctx, campaign_dir, portrait_id);
                                        let has_texture = self
                                            .portrait_textures
                                            .get(portrait_id)
                                            .and_then(|opt| opt.as_ref())
                                            .is_some();

                                        // Build tooltip text with portrait path
                                        let tooltip_text = if let Some(path) =
                                            resolve_portrait_path(campaign_dir, portrait_id)
                                        {
                                            format!(
                                                "Portrait ID: {}\nPath: {}",
                                                portrait_id,
                                                path.display()
                                            )
                                        } else {
                                            format!(
                                                "Portrait ID: {}\n⚠ File not found",
                                                portrait_id
                                            )
                                        };

                                        // Create image button or placeholder
                                        let button_response = if has_texture {
                                            let texture = self
                                                .portrait_textures
                                                .get(portrait_id)
                                                .and_then(|t| t.as_ref())
                                                .expect(
                                                    "texture present since has_texture is true",
                                                );
                                            ui.add(
                                                egui::Button::image(
                                                    egui::Image::new(texture).fit_to_exact_size(
                                                        egui::vec2(THUMBNAIL_SIZE, THUMBNAIL_SIZE),
                                                    ),
                                                )
                                                .frame(true),
                                            )
                                            .on_hover_text(&tooltip_text)
                                        } else {
                                            // Placeholder for failed/missing images
                                            let (rect, response) = ui.allocate_exact_size(
                                                egui::vec2(THUMBNAIL_SIZE, THUMBNAIL_SIZE),
                                                egui::Sense::click(),
                                            );
                                            ui.painter().rect_filled(
                                                rect,
                                                2.0,
                                                egui::Color32::from_gray(50),
                                            );
                                            ui.painter().text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                "?",
                                                egui::FontId::proportional(24.0),
                                                egui::Color32::from_gray(150),
                                            );
                                            response.on_hover_text(&tooltip_text)
                                        };

                                        // Check if clicked
                                        if button_response.clicked() {
                                            selected_portrait = Some(portrait_id.clone());
                                            self.portrait_picker_open = false;
                                        }

                                        // Show portrait ID below thumbnail
                                        ui.label(
                                            egui::RichText::new(portrait_id)
                                                .size(10.0)
                                                .color(egui::Color32::from_gray(200)),
                                        );
                                    });
                                }
                            });
                        }

                        // Show message if no portraits found
                        if total_portraits == 0 {
                            ui.label("No portraits found in campaign assets/portraits directory.");
                        }
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        self.portrait_picker_open = false;
                    }
                });
            });

        selected_portrait
    }

    /// Shows a sprite sheet picker popup for visual sprite sheet selection.
    ///
    /// Displays a popup window listing available sprite sheets that the user can click
    /// to select. The popup closes when a sheet is selected or the close button is clicked.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The egui context for rendering
    /// * `campaign_dir` - The campaign directory used to validate sprite sheet paths
    ///
    /// # Returns
    ///
    /// Returns `Some(sheet_path)` if the user selected a sprite sheet this frame,
    /// or `None` if no selection was made.
    pub fn show_sprite_sheet_picker(
        &mut self,
        ctx: &egui::Context,
        campaign_dir: Option<&PathBuf>,
    ) -> Option<String> {
        let mut selected_sheet: Option<String> = None;

        // Clone the list to avoid borrow conflicts in the UI closure
        let available = self.available_sprite_sheets.clone();

        egui::Window::new("Select Sprite Sheet")
            .collapsible(false)
            .resizable(true)
            .default_width(500.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.label("Click a sprite sheet to select:");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

                    for sheet in &available {
                        // Show each candidate as a selectable label with a small preview/action area
                        ui.horizontal(|ui| {
                            let resp = ui.selectable_label(false, sheet);
                            if resp.clicked() {
                                selected_sheet = Some(sheet.clone());
                            }

                            // Hover tooltip showing full path (if campaign dir known)
                            if let Some(dir) = campaign_dir {
                                let full = dir.join(sheet);
                                if full.exists() {
                                    ui.label(egui::RichText::new("•").weak())
                                        .on_hover_text(format!("Path: {}", full.display()));
                                } else {
                                    ui.label(
                                        egui::RichText::new("⚠")
                                            .color(egui::Color32::from_rgb(255, 180, 0)),
                                    )
                                    .on_hover_text(format!("Missing: {}", full.display()));
                                }
                            }
                        });
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        self.sprite_picker_open = false;
                    }
                });
            });

        selected_sheet
    }
}

/// Helper function to load a portrait texture into a texture cache map.
fn load_npc_portrait_texture(
    ctx: &egui::Context,
    campaign_dir: Option<&PathBuf>,
    portrait_id: &str,
    portrait_textures: &mut HashMap<String, Option<egui::TextureHandle>>,
) -> bool {
    if portrait_textures.contains_key(portrait_id) {
        return portrait_textures
            .get(portrait_id)
            .and_then(|value| value.as_ref())
            .is_some();
    }

    let texture_handle = (|| {
        let path = resolve_portrait_path(campaign_dir, portrait_id)?;

        let image_bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_e) => {
                // Portrait read failure is non-critical; the UI shows a "?" placeholder.
                return None;
            }
        };

        let dynamic_image = match image::load_from_memory(&image_bytes) {
            Ok(img) => img,
            Err(_e) => {
                // Portrait decode failure is non-critical; the UI shows a "?" placeholder.
                return None;
            }
        };

        let rgba_image = dynamic_image.to_rgba8();
        let size = [rgba_image.width() as usize, rgba_image.height() as usize];
        let pixels = rgba_image.as_flat_samples();

        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

        Some(ctx.load_texture(
            format!("npc_portrait_{}", portrait_id),
            color_image,
            egui::TextureOptions::LINEAR,
        ))
    })();

    let loaded = texture_handle.is_some();
    // If loading failed, `loaded` is false and the UI will show a "?" placeholder.

    portrait_textures.insert(portrait_id.to_string(), texture_handle);
    loaded
}

fn merchant_dialogue_status_for_preview(
    npc: &NpcDefinition,
    dialogue: Option<&DialogueTree>,
) -> &'static str {
    if !npc.is_merchant {
        if dialogue.is_some_and(DialogueTree::has_sdk_managed_merchant_content) {
            return "Non-merchant has stale merchant content";
        }
        return "Not a merchant";
    }

    let Some(dialogue) = dialogue else {
        return if npc.dialogue_id.is_some() {
            "Assigned dialogue missing"
        } else {
            "No dialogue assigned"
        };
    };

    if dialogue.contains_open_merchant_for_npc(&npc.id) {
        if dialogue.has_sdk_managed_merchant_content() {
            "SDK-managed merchant branch present"
        } else {
            "Merchant dialogue valid"
        }
    } else {
        let opens_other_merchant = dialogue.nodes.values().any(|node| {
            node.actions.iter().any(|action| {
                matches!(
                    action,
                    DialogueAction::OpenMerchant { npc_id } if npc_id != &npc.id
                )
            }) || node.choices.iter().any(|choice| {
                choice.actions.iter().any(|action| {
                    matches!(
                        action,
                        DialogueAction::OpenMerchant { npc_id } if npc_id != &npc.id
                    )
                })
            })
        });

        if opens_other_merchant {
            "Merchant dialogue targets wrong NPC"
        } else {
            "Merchant dialogue missing OpenMerchant"
        }
    }
}

/// Renders the NPC detail preview panel (portrait, identity grid, merchant/service info).
///
/// Called from the list view's right panel whenever an NPC is selected.
pub(super) fn show_npc_preview(
    ui: &mut egui::Ui,
    npc: &NpcDefinition,
    campaign_dir: Option<&PathBuf>,
    creature_manager: Option<&CreatureAssetManager>,
    available_dialogues: &[DialogueTree],
    portrait_textures: &mut HashMap<String, Option<egui::TextureHandle>>,
) {
    let assigned_dialogue = npc.dialogue_id.and_then(|dialogue_id| {
        available_dialogues
            .iter()
            .find(|dialogue| dialogue.id == dialogue_id)
    });

    ui.horizontal(|ui| {
        let portrait_size = egui::vec2(128.0, 128.0);

        let has_texture =
            load_npc_portrait_texture(ui.ctx(), campaign_dir, &npc.portrait_id, portrait_textures);

        if has_texture {
            if let Some(Some(texture)) = portrait_textures.get(&npc.portrait_id) {
                ui.add(egui::Image::new(texture).fit_to_exact_size(portrait_size));
            } else {
                show_portrait_placeholder(ui, portrait_size);
            }
        } else {
            show_portrait_placeholder(ui, portrait_size);
        }

        ui.add_space(10.0);

        ui.vertical(|ui| {
            ui.heading(&npc.name);
            ui.label(format!("ID: {}", npc.id));

            if !npc.portrait_id.is_empty() {
                ui.label(format!("Portrait: {}", npc.portrait_id));
            }

            ui.add_space(4.0);

            if npc.is_merchant {
                ui.label(egui::RichText::new("🏪 Merchant").color(egui::Color32::GOLD));
                ui.small(
                    "Merchant authoring standard: assigned dialogue must explicitly contain OpenMerchant. The I key during dialogue is only a runtime shortcut; the SDK will generate or repair merchant dialogue automatically and preserve custom dialogue where possible.",
                );
            }
            if npc.is_innkeeper {
                ui.label(egui::RichText::new("🛏️ Innkeeper").color(egui::Color32::LIGHT_BLUE));
            }
            if npc.is_priest {
                ui.label(
                    egui::RichText::new("✝ Priest")
                        .color(egui::Color32::from_rgb(200, 180, 255)),
                );
            }
            if !npc.is_merchant && !npc.is_innkeeper && !npc.is_priest {
                ui.label(egui::RichText::new("🧑 NPC").color(egui::Color32::GRAY));
            }
        });
    });

    ui.add_space(10.0);
    ui.separator();

    egui::Grid::new("npc_preview_identity_grid")
        .num_columns(2)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            if let Some(faction) = &npc.faction {
                if !faction.trim().is_empty() {
                    ui.label("Faction:");
                    ui.label(faction.as_str());
                    ui.end_row();
                }
            }

            ui.label("Dialogue:");
            ui.label(
                npc.dialogue_id
                    .map(|d| d.to_string())
                    .as_deref()
                    .unwrap_or("(none)"),
            );
            ui.end_row();

            ui.label("Merchant Dialogue:");
            ui.label(merchant_dialogue_status_for_preview(npc, assigned_dialogue));
            ui.end_row();

            ui.label("Quests:");
            if npc.quest_ids.is_empty() {
                ui.label("(none)");
            } else {
                ui.label(format!("{} assigned", npc.quest_ids.len()));
            }
            ui.end_row();

            if let Some(creature_id) = npc.creature_id {
                ui.label("Creature ID:");
                ui.label(creature_id.to_string());
                ui.end_row();
            }

            if let (Some(creature_id), Some(manager)) = (npc.creature_id, creature_manager) {
                let resolved = manager
                    .load_creature(creature_id)
                    .map(|c| c.name)
                    .unwrap_or_else(|_| "⚠ Unknown".to_string());
                ui.label("Asset:");
                ui.label(resolved);
                ui.end_row();
            }
        });

    if let Some(sprite) = &npc.sprite {
        ui.add_space(10.0);
        ui.heading("Sprite");
        ui.separator();

        egui::Grid::new("npc_preview_sprite_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label("Sheet:");
                ui.label(&sprite.sheet_path);
                ui.end_row();

                ui.label("Index:");
                ui.label(sprite.sprite_index.to_string());
                ui.end_row();
            });
    }

    if npc.is_merchant {
        ui.add_space(10.0);
        ui.heading("Merchant");
        ui.separator();

        egui::Grid::new("npc_preview_merchant_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label("Stock Template:");
                ui.label(npc.stock_template.as_deref().unwrap_or("(none)"));
                ui.end_row();

                if let Some(economy) = &npc.economy {
                    ui.label("Buy Rate:");
                    ui.label(format!("{:.0}%", economy.buy_rate * 100.0));
                    ui.end_row();

                    ui.label("Sell Rate:");
                    ui.label(format!("{:.0}%", economy.sell_rate * 100.0));
                    ui.end_row();
                }
            });
    }

    if (npc.is_priest || npc.is_innkeeper) && npc.service_catalog.is_some() {
        ui.add_space(10.0);
        ui.heading("Services");
        ui.separator();
        ui.label("(service catalog configured)");
    }

    if !npc.quest_ids.is_empty() {
        ui.add_space(10.0);
        ui.heading("Quests");
        ui.separator();

        for quest_id in &npc.quest_ids {
            ui.label(format!("• {}", quest_id));
        }
    }

    if !npc.description.is_empty() {
        ui.add_space(10.0);
        ui.heading("Description");
        ui.separator();
        ui.label(&npc.description);
    }
}

fn show_portrait_placeholder(ui: &mut egui::Ui, size: egui::Vec2) {
    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());

    // Background
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(4),
        egui::Color32::from_gray(40),
    );

    // Border
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(4),
        egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
        egui::StrokeKind::Outside,
    );

    // Icon
    let center = rect.center();
    let icon_size = size.y * 0.4;
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        "🖼",
        egui::FontId::proportional(icon_size),
        egui::Color32::from_rgb(150, 150, 150),
    );
}
