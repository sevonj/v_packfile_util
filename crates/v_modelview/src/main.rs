// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([400.0, 300.0]),
        depth_buffer: 32,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "V Model Viewer",
        native_options,
        Box::new(|cc| {
            cc.egui_ctx
                .options_mut(|opt| opt.zoom_with_keyboard = false);
            Ok(Box::new(v_modelview::VModelViewer::new(cc)))
        }),
    )
}
