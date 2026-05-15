// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use egui::Widget;
use v_types::Vector;

pub struct VectorDisplay {
    value: Vector,
}

impl VectorDisplay {
    pub fn new(value: Vector) -> Self {
        Self { value }
    }
}

impl Widget for VectorDisplay {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            ui.label(format!("X: {:.3}", self.value.x))
                .on_hover_text(self.value.x.to_string());
            ui.label(format!("Y: {:.3}", self.value.y))
                .on_hover_text(self.value.y.to_string());
            ui.label(format!("Z: {:.3}", self.value.z))
                .on_hover_text(self.value.z.to_string());
        })
        .response
    }
}
