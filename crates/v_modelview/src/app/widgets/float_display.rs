// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use egui::Widget;

pub struct FloatDisplay {
    value: f32,
}

impl FloatDisplay {
    pub fn new(value: f32) -> Self {
        Self { value }
    }
}

impl Widget for FloatDisplay {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.label(format!("{:.3}", self.value))
            .on_hover_text(self.value.to_string())
    }
}
