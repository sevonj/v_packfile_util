// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use egui::CentralPanel;
use egui::Widget;

pub struct StatusPage<'a> {
    title: &'a str,
    subtitle: &'a str,
}

impl<'a> StatusPage<'a> {
    pub fn new(title: &'a str, subtitle: &'a str) -> Self {
        Self { title, subtitle }
    }

    pub fn status_no_file() -> Self {
        Self::new("No File", "Drop a file here or pick one from the menu.")
    }
}

impl Widget for StatusPage<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        CentralPanel::no_frame()
            .show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(self.title);
                    ui.label(self.subtitle);
                })
                .response
            })
            .response
    }
}
