// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::VecDeque;

use egui::CentralPanel;
use egui::ScrollArea;
use egui::Vec2b;
use egui::Widget;

pub struct LogView<'a> {
    log: &'a VecDeque<String>,
}

impl<'a> LogView<'a> {
    pub fn new(log: &'a VecDeque<String>) -> Self {
        Self { log }
    }
}

impl Widget for LogView<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        CentralPanel::no_frame()
            .show_inside(ui, |ui| {
                ScrollArea::new(Vec2b::new(false, true)).show(ui, |ui| {
                    ui.vertical(|ui| {
                        for line in self.log {
                            ui.monospace(line);
                        }
                    });
                });
            })
            .response
    }
}
