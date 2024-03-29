/*
 * Copyright (C) 2023 Alexis Purslane <alexispurslane@pm.me>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::render_thread::RenderState;

pub fn performance_stats_window(ui: &mut imgui::Ui, render_state: &RenderState, avg_dt: f32) {
    ui.window("Performance Stats")
        .size([300.0, 200.0], imgui::Condition::FirstUseEver)
        .position([1600.0, 20.0], imgui::Condition::FirstUseEver)
        .build(|| {
            ui.text(format!(
                "FPS (3 frame running average): {}",
                1000.0 / avg_dt
            ));
            ui.text(format!(
                "Frame Time (3 frame running average): {}ms",
                avg_dt
            ));
            ui.separator();

            if let Some(cam) = render_state.camera.as_ref() {
                let rot = cam
                    .view
                    .to_scale_rotation_translation()
                    .1
                    .to_euler(glam::EulerRot::XYZ);
                ui.text(format!(
                    "Camera direction: [{} {} {}]",
                    (rot.0 * 180.0 / std::f32::consts::PI).round(),
                    (rot.1 * 180.0 / std::f32::consts::PI).round(),
                    (rot.2 * 180.0 / std::f32::consts::PI).round()
                ));
            }
            ui.text(format!(
                "Allocated entities: {:?}",
                render_state.entity_transforms.len()
            ));
            ui.text(format!("Models loaded: {:?}", render_state.models.len()));
        });
}
