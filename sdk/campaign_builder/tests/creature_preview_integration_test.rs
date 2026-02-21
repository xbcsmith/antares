// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use antares::domain::visual::{CreatureDefinition, MeshDefinition, MeshTransform};
use campaign_builder::creatures_editor::{CreaturesEditorMode, CreaturesEditorState};
use eframe::egui;

fn make_mesh(color: [f32; 4]) -> MeshDefinition {
    MeshDefinition {
        name: Some("mesh".to_string()),
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        indices: vec![0, 1, 2],
        normals: None,
        uvs: None,
        color,
        lod_levels: None,
        lod_distances: None,
        material: None,
        texture_path: None,
    }
}

fn run_editor_frame(state: &mut CreaturesEditorState, creatures: &mut Vec<CreatureDefinition>) {
    let mut unsaved_changes = false;
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let _ = state.show(
                ui,
                creatures,
                &None,
                "data/creatures.ron",
                &mut unsaved_changes,
            );
        });
    });
}

#[test]
fn test_preview_updates_after_transform_edit_in_ui_frame() {
    let mut state = CreaturesEditorState::new();
    let mut creatures = vec![CreatureDefinition {
        id: 1,
        name: "Goblin".to_string(),
        meshes: vec![make_mesh([1.0, 0.0, 0.0, 1.0])],
        mesh_transforms: vec![MeshTransform::identity()],
        scale: 1.0,
        color_tint: None,
    }];

    state.mode = CreaturesEditorMode::Edit;
    state.selected_creature = Some(0);
    state.edit_buffer = creatures[0].clone();
    state.mesh_visibility = vec![true];
    state.selected_mesh_index = Some(0);

    state.edit_buffer.mesh_transforms[0].translation = [2.0, 0.0, -1.0];
    state.preview_dirty = true;

    run_editor_frame(&mut state, &mut creatures);

    assert!(!state.preview_dirty);
    assert_eq!(state.preview_state.statistics.mesh_count, 1);
    assert_eq!(state.preview_state.statistics.vertex_count, 3);
}

#[test]
fn test_preview_updates_after_color_edit_in_ui_frame() {
    let mut state = CreaturesEditorState::new();
    let mut creatures = vec![CreatureDefinition {
        id: 2,
        name: "Orc".to_string(),
        meshes: vec![make_mesh([0.0, 0.0, 1.0, 1.0])],
        mesh_transforms: vec![MeshTransform::identity()],
        scale: 1.0,
        color_tint: None,
    }];

    state.mode = CreaturesEditorMode::Edit;
    state.selected_creature = Some(0);
    state.edit_buffer = creatures[0].clone();
    state.mesh_visibility = vec![true];

    state.edit_buffer.meshes[0].color = [0.2, 0.8, 0.1, 1.0];
    state.preview_dirty = true;

    run_editor_frame(&mut state, &mut creatures);

    assert!(!state.preview_dirty);
    assert_eq!(state.preview_state.statistics.triangle_count, 1);
    assert_eq!(state.preview_state.statistics.vertex_count, 3);
}
